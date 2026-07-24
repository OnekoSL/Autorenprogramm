use super::*;

#[tauri::command]
pub fn send_ai_chat_message(
    input: SendAiChatMessageInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<AiChatResponse, AppError> {
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    with_active_connection(&state, |connection| {
        let context = build_ai_chat_context(connection, &input)?;
        let language = read_active_project_language(connection)?;
        let content =
            call_openai_compatible_free_chat(&settings, &input, context.as_deref(), &language)?;

        Ok(AiChatResponse {
            message: AiChatMessage {
                role: "assistant".to_string(),
                content,
            },
        })
    })
}

#[tauri::command]
pub fn generate_text_alternatives(
    input: GenerateTextAlternativesInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<TextAlternativesResponse, AppError> {
    let request = with_active_connection(&state, |connection| {
        build_text_alternatives_request(connection, input)
    })?;
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    let alternatives = call_openai_compatible_text_alternatives(&settings, &request)?;

    Ok(TextAlternativesResponse { alternatives })
}

#[tauri::command]
pub fn improve_text(
    input: ImproveTextInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<TextImprovementsResponse, AppError> {
    let request = with_active_connection(&state, |connection| {
        build_improve_text_request(connection, input)
    })?;
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    let improvements = call_openai_compatible_text_improvements(&settings, &request)?;

    Ok(TextImprovementsResponse { improvements })
}

pub(crate) fn build_ai_scene_request(
    connection: &Connection,
    input: &SummarizeSceneInput,
) -> Result<AiSceneRequest, AppError> {
    let (project_id, scene_id, scene_title, scene_summary, scene_text): (
        String,
        String,
        String,
        String,
        String,
    ) = connection
        .query_row(
            "
            SELECT project.id, scene.id, scene.title, scene.summary, scene.text
            FROM scene
            INNER JOIN chapter ON chapter.id = scene.chapter_id AND chapter.deleted_at IS NULL
            INNER JOIN novel ON novel.id = chapter.novel_id AND novel.deleted_at IS NULL
            INNER JOIN project ON project.id = novel.project_id AND project.deleted_at IS NULL
            WHERE scene.id = ?1 AND scene.deleted_at IS NULL
            LIMIT 1
            ",
            params![input.scene_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::new("scene_missing", "Die Szene wurde nicht gefunden."))?;

    if scene_text.trim().is_empty() {
        return Err(AppError::new(
            "ai_scene_empty",
            "Die aktuelle Szene enthält noch keinen Text.",
        ));
    }

    Ok(AiSceneRequest {
        project_id,
        language: read_active_project_language(connection)?,
        scene_id: scene_id.clone(),
        scene_title,
        scene_summary,
        scene_text,
        context_summary: build_scene_project_bible_context(connection, &scene_id)?,
    })
}

pub(crate) fn build_ai_contradiction_request(
    connection: &Connection,
    input: &SummarizeSceneInput,
) -> Result<AiContradictionRequest, AppError> {
    let request = build_ai_scene_request(connection, input)?;
    if !scene_has_project_bible_context(connection, &request.scene_id)? {
        return Err(AppError::new(
            "ai_scene_context_missing",
            "Verknuepfe zuerst mindestens eine Projektbibel-Karte mit dieser Szene.",
        ));
    }

    Ok(AiContradictionRequest {
        project_id: request.project_id,
        language: request.language,
        scene_id: request.scene_id,
        scene_title: request.scene_title,
        scene_summary: request.scene_summary,
        scene_text: request.scene_text,
        context_summary: request.context_summary,
    })
}

pub(crate) fn scene_has_project_bible_context(
    connection: &Connection,
    scene_id: &str,
) -> Result<bool, AppError> {
    let count: i64 = connection.query_row(
        "
        SELECT COUNT(*)
        FROM scene_link
        WHERE scene_id = ?1 AND deleted_at IS NULL
        ",
        params![scene_id],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

pub(crate) fn build_scene_project_bible_context(
    connection: &Connection,
    scene_id: &str,
) -> Result<String, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT target_type, target_id
        FROM scene_link
        WHERE scene_id = ?1 AND deleted_at IS NULL
        ORDER BY sort_order ASC, created_at ASC
        ",
    )?;
    let links = statement
        .query_map(params![scene_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut context_parts = Vec::new();
    let mut included = std::collections::HashSet::new();
    for (target_type, target_id) in links {
        included.insert((target_type.clone(), target_id.clone()));
        if let Some(context) =
            read_project_bible_context_card(connection, &target_type, &target_id)?
        {
            context_parts.push(context);
        }
        let mut relations = connection.prepare("SELECT source_type, source_id, target_type, target_id, relationship_type, description FROM bible_relationship WHERE deleted_at IS NULL AND ((source_type=?1 AND source_id=?2) OR (target_type=?1 AND target_id=?2)) ORDER BY created_at")?;
        let rows = relations
            .query_map(params![target_type, target_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        for (
            source_type,
            source_id,
            relation_target_type,
            relation_target_id,
            relation_type,
            description,
        ) in rows
        {
            let (related_type, related_id) = if source_type == target_type && source_id == target_id
            {
                (relation_target_type, relation_target_id)
            } else {
                (source_type, source_id)
            };
            if included.insert((related_type.clone(), related_id.clone())) {
                if let Some(card) =
                    read_project_bible_context_card(connection, &related_type, &related_id)?
                {
                    context_parts.push(format!(
                        "Direkte Beziehung ({relation_type}{}):\n{card}",
                        if description.trim().is_empty() {
                            String::new()
                        } else {
                            format!(", {description}")
                        }
                    ));
                }
            }
        }
    }

    if context_parts.is_empty() {
        Ok("Keine Projektbibel-Karten mit dieser Szene verknüpft.".to_string())
    } else {
        Ok(context_parts.join("\n\n"))
    }
}

pub(crate) fn build_ai_chat_context(
    connection: &Connection,
    input: &SendAiChatMessageInput,
) -> Result<Option<String>, AppError> {
    if !input.include_scene_context {
        return Ok(None);
    }

    let scene_id = input.scene_id.as_deref().unwrap_or("").trim();
    if scene_id.is_empty() {
        return Ok(None);
    }

    ensure_active_scene(connection, scene_id)?;
    let (title, status, summary, text): (String, String, String, String) = connection.query_row(
        "
        SELECT title, status, summary, text
        FROM scene
        WHERE id = ?1 AND deleted_at IS NULL
        ",
        params![scene_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let bible_context = build_scene_project_bible_context(connection, scene_id)?;
    let excerpt = if text.chars().count() > 4000 {
        format!("{}...", text.chars().take(4000).collect::<String>())
    } else {
        text
    };

    Ok(Some(format!(
        "Aktuelle Szene:\nTitel: {title}\nStatus: {status}\nKurzbeschreibung: {}\n\nSzenentext-Auszug:\n{excerpt}\n\nProjektbibel-Kontext:\n{bible_context}",
        if summary.trim().is_empty() {
            "Keine Kurzbeschreibung"
        } else {
            summary.trim()
        }
    )))
}

pub(crate) fn build_text_alternatives_request(
    connection: &Connection,
    input: GenerateTextAlternativesInput,
) -> Result<TextAlternativesRequest, AppError> {
    ensure_active_scene(connection, &input.scene_id)?;
    let selected_text = input.selected_text.trim();
    if selected_text.is_empty() {
        return Err(AppError::new(
            "empty_text_selection",
            "Bitte markiere zuerst ein Wort oder einen Satz.",
        ));
    }

    let word_count = selected_text.split_whitespace().count();
    let is_phrase = word_count > 1 || selected_text.chars().any(|ch| ".!?;,:".contains(ch));

    Ok(TextAlternativesRequest {
        language: read_active_project_language(connection)?,
        scene_id: input.scene_id,
        selected_text: selected_text.to_string(),
        context_before: excerpt(input.context_before.trim(), 700),
        context_after: excerpt(input.context_after.trim(), 700),
        is_phrase,
    })
}

pub(crate) fn build_improve_text_request(
    connection: &Connection,
    input: ImproveTextInput,
) -> Result<TextImprovementRequest, AppError> {
    ensure_active_scene(connection, &input.scene_id)?;
    let mode = match input.mode.trim() {
        "selection" => "selection",
        "scene" => "scene",
        _ => {
            return Err(AppError::new(
                "invalid_improve_text_mode",
                "Der Textverbesserungsmodus ist ungültig.",
            ));
        }
    };

    let selected_text = if mode == "selection" {
        let text = input.selected_text.trim();
        if text.is_empty() {
            return Err(AppError::new(
                "empty_text_selection",
                "Bitte markiere zuerst Text im Szeneneditor.",
            ));
        }

        text.to_string()
    } else {
        let scene_text: String = connection.query_row(
            "
            SELECT text
            FROM scene
            WHERE id = ?1 AND deleted_at IS NULL
            ",
            params![input.scene_id],
            |row| row.get(0),
        )?;
        let text = scene_text.trim();
        if text.is_empty() {
            return Err(AppError::new(
                "empty_scene_text",
                "Die Szene braucht Text, bevor sie verbessert werden kann.",
            ));
        }

        excerpt(text, 6000)
    };

    Ok(TextImprovementRequest {
        language: read_active_project_language(connection)?,
        scene_id: input.scene_id,
        selected_text,
        context_before: excerpt(input.context_before.trim(), 900),
        context_after: excerpt(input.context_after.trim(), 900),
        mode: mode.to_string(),
    })
}

pub(crate) fn read_project_bible_context_card(
    connection: &Connection,
    target_type: &str,
    target_id: &str,
) -> Result<Option<String>, AppError> {
    match target_type {
        "character" => connection
            .query_row(
                "
                SELECT name, role, description, notes
                FROM character
                WHERE id = ?1 AND deleted_at IS NULL
                ",
                params![target_id],
                |row| {
                    Ok(format!(
                        "Figur: {}\nRolle: {}\nBeschreibung: {}\nNotizen: {}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?
                    ))
                },
            )
            .optional()
            .map_err(AppError::from),
        "location" => connection
            .query_row(
                "
                SELECT name, description, notes
                FROM location
                WHERE id = ?1 AND deleted_at IS NULL
                ",
                params![target_id],
                |row| {
                    Ok(format!(
                        "Ort: {}\nBeschreibung: {}\nNotizen: {}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ))
                },
            )
            .optional()
            .map_err(AppError::from),
        "item" => connection
            .query_row(
                "
                SELECT name, description, notes
                FROM item
                WHERE id = ?1 AND deleted_at IS NULL
                ",
                params![target_id],
                |row| {
                    Ok(format!(
                        "Gegenstand: {}\nBeschreibung: {}\nNotizen: {}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ))
                },
            )
            .optional()
            .map_err(AppError::from),
        "note" => connection
            .query_row(
                "
                SELECT title, note_type, body
                FROM note
                WHERE id = ?1 AND deleted_at IS NULL
                ",
                params![target_id],
                |row| {
                    Ok(format!(
                        "Notiz: {}\nTyp: {}\nInhalt: {}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ))
                },
            )
            .optional()
            .map_err(AppError::from),
        "faction" => connection.query_row("SELECT name, faction_type, description, goals, notes FROM faction WHERE id=?1 AND deleted_at IS NULL", params![target_id], |row| Ok(format!("Fraktion: {}\nArt: {}\nBeschreibung: {}\nZiele: {}\nNotizen: {}", row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?))).optional().map_err(AppError::from),
        "event" => connection.query_row("SELECT title, time_label, summary, outcome, notes FROM event WHERE id=?1 AND deleted_at IS NULL", params![target_id], |row| Ok(format!("Ereignis: {}\nZeit: {}\nZusammenfassung: {}\nVerlauf/Ausgang: {}\nNotizen: {}", row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?))).optional().map_err(AppError::from),
        "world_system" => connection.query_row("SELECT name, category, principle, rules, capabilities, costs, limits, access, distribution, world_impact, open_questions FROM world_system WHERE id=?1 AND deleted_at IS NULL", params![target_id], |row| Ok(format!("Weltsystem: {}\nKategorie: {}\nGrundprinzip: {}\nRegeln: {}\nMöglichkeiten: {}\nKosten: {}\nGrenzen: {}\nZugang: {}\nVerbreitung: {}\nAuswirkungen: {}\nOffene Fragen: {}", row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?, row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, String>(8)?, row.get::<_, String>(9)?, row.get::<_, String>(10)?))).optional().map_err(AppError::from),
        _ => Ok(None),
    }
}

pub(crate) fn call_openai_compatible_chat(
    settings: &AiSettings,
    request: &AiSceneRequest,
) -> Result<String, AppError> {
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let prompt = build_summarize_scene_prompt(request);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": [
            {
                "role": "system",
                "content": if request.language == "en" { "You are a writing assistant for novelists. Use British English by default. Use only the provided scene text and context, and invent no facts." } else { "Du bist eine Schreibassistenz für Romanautoren. Antworte standardmäßig auf Deutsch. Nutze nur den gegebenen Szenentext und Kontext und erfinde keine Fakten." }
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.2,
        "stream": false,
        "max_tokens": 220,
        "reasoning_effort": "none"
    }));

    let response = request_builder.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Der lokale KI-Dienst antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    let content = parsed["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if content.is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keine Zusammenfassung geliefert.",
        ));
    }

    Ok(content)
}

pub(crate) fn call_openai_compatible_contradictions(
    settings: &AiSettings,
    request: &AiContradictionRequest,
) -> Result<String, AppError> {
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let prompt = build_check_contradictions_prompt(request);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": [
            {
                "role": "system",
                "content": if request.language == "en" { "You are a continuity assistant for novelists. Use British English by default. Check only the provided scene against the project context, invent no facts and state uncertainty clearly." } else { "Du bist eine Kontinuitätsassistenz für Romanautoren. Antworte standardmäßig auf Deutsch. Prüfe nur den gegebenen Szenentext gegen den Projektkontext, erfinde keine Fakten und formuliere Unsicherheiten klar." }
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.1,
        "stream": false,
        "max_tokens": 520,
        "reasoning_effort": "none"
    }));

    let response = request_builder.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Der lokale KI-Dienst antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    let content = clean_ai_chat_content(
        parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or(""),
    );

    if content.trim().is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keine Widerspruchsprüfung geliefert.",
        ));
    }

    Ok(content.trim().to_string())
}

pub(crate) fn build_check_contradictions_prompt(request: &AiContradictionRequest) -> String {
    if request.language == "en" {
        return format!(
            "Check the current scene for concrete contradictions, ambiguities or missing continuity against the linked project context. Check only this scene and the supplied context; do not perform a global novel analysis or invent facts. Return at most five concise Markdown bullet points with category, finding, scene evidence and context evidence. Mark uncertainty clearly. If none exist, state that no concrete contradictions were found.\n\nScene title: {}\n\nSummary: {}\n\nProject context:\n{}\n\nScene text:\n{}",
            request.scene_title,
            if request.scene_summary.trim().is_empty() { "No summary" } else { request.scene_summary.trim() },
            request.context_summary,
            request.scene_text
        );
    }
    format!(
        "Prüfe die aktuelle Szene auf mögliche Widersprüche, Unklarheiten oder fehlende Anschlussstellen gegen den verknüpften Projektkontext.\n\
        Begrenzung:\n\
        - Prüfe nur diese Szene und den unten gegebenen Projektkontext.\n\
        - Keine globale Romananalyse.\n\
        - Keine neuen Fakten erfinden.\n\
        - Melde nur konkrete Auffälligkeiten oder schreibe: \"Keine konkreten Widersprüche gefunden.\"\n\
        - Wenn etwas nur unsicher ist, markiere es als Unsicherheit.\n\n\
        Ausgabeformat:\n\
        - Kurze Markdown-Liste.\n\
        - Pro Punkt: Kategorie, Befund, Bezug aus Szene, Bezug aus Projektkontext.\n\
        - Maximal 5 Punkte.\n\n\
        Szenentitel: {}\n\n\
        Kurzbeschreibung: {}\n\n\
        Projektkontext:\n{}\n\n\
        Szenentext:\n{}",
        request.scene_title,
        if request.scene_summary.trim().is_empty() {
            "Keine Kurzbeschreibung"
        } else {
            request.scene_summary.trim()
        },
        request.context_summary,
        request.scene_text
    )
}

pub(crate) fn build_text_alternatives_prompt(request: &TextAlternativesRequest) -> String {
    let task = if request.language == "en" && request.is_phrase {
        "Produce 3 to 5 rephrased alternatives for the selected sentence or passage."
    } else if request.language == "en" {
        "Produce 5 concise synonyms or very short alternatives for the selected word."
    } else if request.is_phrase {
        "Erzeuge 3 bis 5 umformulierte Alternativen für den markierten Satz oder Abschnitt."
    } else {
        "Erzeuge 5 kurze Synonyme oder sehr kurze Alternativen für das markierte Wort."
    };
    let language = ai_language_instruction(&request.language);

    format!(
        "{task}\n\nContext before:\n{}\n\nSelection:\n{}\n\nContext after:\n{}\n\n{language}. Return only a JSON array of strings. No explanations, numbering, analysis, reasoning, thoughts or invented facts.",
        request.context_before, request.selected_text, request.context_after
    )
}

pub(crate) fn build_text_improvement_prompt(request: &TextImprovementRequest) -> String {
    let scope = if request.language == "en" && request.mode == "selection" {
        "Improve the selected passage."
    } else if request.language == "en" {
        "Improve the entire scene."
    } else if request.mode == "selection" {
        "Verbessere die markierte Textstelle."
    } else {
        "Verbessere die gesamte Szene."
    };

    let language = ai_language_instruction(&request.language);
    format!(
        "{scope}\n\nProduce 2 to 3 clearer and more fluent versions. Reduce repetition and awkward wording while preserving content, tone, viewpoint, facts and characters. Invent no new details.\n\nContext before:\n{}\n\nText to improve:\n{}\n\nContext after:\n{}\n\n{language}. Return only a JSON array of strings. No explanations, numbering, analysis, reasoning, thoughts or Markdown fences.",
        request.context_before, request.selected_text, request.context_after
    )
}

pub(crate) fn parse_text_alternatives_response(content: &str) -> Result<Vec<String>, AppError> {
    let cleaned = strip_ai_reasoning_blocks(&clean_ai_chat_content(content));
    let json_candidate = cleaned
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Some(alternatives) = parse_text_alternatives_json(json_candidate) {
        return Ok(alternatives);
    }

    if let Some(candidate) = extract_json_array(json_candidate) {
        if let Some(alternatives) = parse_text_alternatives_json(candidate) {
            return Ok(alternatives);
        }
    }

    let alternatives = normalize_text_alternatives(
        cleaned
            .lines()
            .filter(|line| !looks_like_reasoning_line(line))
            .map(|line| {
                line.trim()
                    .trim_start_matches(|ch: char| {
                        ch.is_ascii_digit()
                            || ch == '.'
                            || ch == ')'
                            || ch == '-'
                            || ch == '*'
                            || ch.is_whitespace()
                    })
                    .trim_matches('"')
                    .trim()
                    .to_string()
            })
            .collect(),
    );

    if alternatives.is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keine verwertbaren Alternativen geliefert.",
        ));
    }

    Ok(alternatives)
}

pub(crate) fn parse_text_improvements_response(content: &str) -> Result<Vec<String>, AppError> {
    let mut improvements = parse_text_alternatives_response(content)?;
    improvements.truncate(3);
    if improvements.is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keine verwertbaren Textverbesserungen geliefert.",
        ));
    }

    Ok(improvements)
}

pub(crate) fn parse_text_alternatives_json(candidate: &str) -> Option<Vec<String>> {
    if let Ok(values) = serde_json::from_str::<Vec<String>>(candidate) {
        let alternatives = normalize_text_alternatives(values);
        if !alternatives.is_empty() {
            return Some(alternatives);
        }
    }

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(candidate) {
        if let Some(values) = parsed
            .get("alternatives")
            .and_then(|value| value.as_array())
        {
            let alternatives = normalize_text_alternatives(
                values
                    .iter()
                    .filter_map(|value| value.as_str().map(ToString::to_string))
                    .collect(),
            );
            if !alternatives.is_empty() {
                return Some(alternatives);
            }
        }
    }

    None
}

pub(crate) fn extract_json_array(content: &str) -> Option<&str> {
    let start = content.find('[')?;
    let end = content.rfind(']')?;
    if start >= end {
        return None;
    }

    Some(&content[start..=end])
}

pub(crate) fn strip_ai_reasoning_blocks(content: &str) -> String {
    let mut cleaned = content.to_string();
    for tag in ["reasoning", "think"] {
        loop {
            let lower = cleaned.to_lowercase();
            let Some(start) = lower.find(&format!("<{tag}>")) else {
                break;
            };
            let end_tag = format!("</{tag}>");
            if let Some(relative_end) = lower[start..].find(&end_tag) {
                let end = start + relative_end + end_tag.len();
                cleaned.replace_range(start..end, "");
            } else {
                cleaned.replace_range(start.., "");
                break;
            }
        }
    }

    cleaned.trim().to_string()
}

pub(crate) fn looks_like_reasoning_line(line: &str) -> bool {
    let lower = line.trim().to_lowercase();
    lower.is_empty()
        || lower.contains("<reasoning")
        || lower.contains("</reasoning")
        || lower.contains("<think")
        || lower.contains("</think")
        || lower.starts_with("i need")
        || lower.starts_with("first,")
        || lower.starts_with("possible synonyms")
        || lower.contains("json format")
        || lower.contains("the user wants")
        || lower.contains("i'll analyze")
}

pub(crate) fn normalize_text_alternatives(values: Vec<String>) -> Vec<String> {
    let mut alternatives = Vec::new();
    for value in values {
        let alternative = value.trim();
        if alternative.is_empty()
            || looks_like_reasoning_line(alternative)
            || alternatives.iter().any(|item| item == alternative)
        {
            continue;
        }
        alternatives.push(alternative.to_string());
        if alternatives.len() >= 5 {
            break;
        }
    }

    alternatives
}

pub(crate) fn call_openai_compatible_text_alternatives(
    settings: &AiSettings,
    request: &TextAlternativesRequest,
) -> Result<Vec<String>, AppError> {
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let prompt = build_text_alternatives_prompt(request);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": [
            {
                "role": "system",
                "content": if request.language == "en" { "You are a British-English writing assistant. Return only final synonyms or rephrasings while preserving tone, meaning and facts. Return only the requested JSON without analysis or reasoning. An explicitly requested different language takes precedence." } else { "Du bist eine deutschsprachige Schreibassistenz. Liefere nur finale Synonyme oder Umformulierungen und bewahre Ton, Bedeutung und Fakten. Antworte ausschließlich im verlangten JSON-Format ohne Analyse oder Reasoning. Eine ausdrücklich verlangte andere Sprache hat Vorrang." }
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.55,
        "stream": false,
        "max_tokens": 300,
        "reasoning_effort": "none",
        "stop": ["<|im_end|>", "<|im_start|>", "<|endoftext|>"]
    }));

    let response = request_builder.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Der lokale KI-Dienst antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    parse_text_alternatives_response(
        parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or(""),
    )
}

pub(crate) fn call_openai_compatible_text_improvements(
    settings: &AiSettings,
    request: &TextImprovementRequest,
) -> Result<Vec<String>, AppError> {
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let prompt = build_text_improvement_prompt(request);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": [
            {
                "role": "system",
                "content": if request.language == "en" { "You are a British-English writing assistant for novelists. Improve text carefully while preserving content, tone, viewpoint and facts. Invent no details and return only the requested JSON without analysis or reasoning. An explicitly requested different language takes precedence." } else { "Du bist eine deutschsprachige Schreibassistenz für Romanautoren. Verbessere Text behutsam und bewahre Inhalt, Ton, Perspektive und Fakten. Erfinde keine Details und antworte ausschließlich im verlangten JSON-Format ohne Analyse oder Reasoning. Eine ausdrücklich verlangte andere Sprache hat Vorrang." }
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.45,
        "stream": false,
        "max_tokens": 1200,
        "reasoning_effort": "none",
        "stop": ["<|im_end|>", "<|im_start|>", "<|endoftext|>"]
    }));

    let response = request_builder.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Der lokale KI-Dienst antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    parse_text_improvements_response(
        parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or(""),
    )
}

pub(crate) fn normalize_chat_messages(
    input: &SendAiChatMessageInput,
) -> Result<Vec<AiChatMessage>, AppError> {
    let mut messages = Vec::new();

    for message in &input.messages {
        let role = message.role.trim();
        let content = message.content.trim();
        if content.is_empty() {
            continue;
        }

        if role != "user" && role != "assistant" {
            return Err(AppError::new(
                "invalid_ai_chat_role",
                "Der KI-Chat unterstützt nur Nutzer- und Assistenznachrichten.",
            ));
        }

        messages.push(AiChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    if messages
        .last()
        .filter(|message| message.role == "user")
        .is_none()
    {
        return Err(AppError::new(
            "empty_ai_chat_message",
            "Bitte gib eine Nachricht für den KI-Chat ein.",
        ));
    }

    Ok(messages
        .into_iter()
        .rev()
        .take(24)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect())
}

pub(crate) fn normalize_ai_chat_mode(
    input: &SendAiChatMessageInput,
) -> Result<&'static str, AppError> {
    match input.mode.trim() {
        AI_CHAT_MODE_IDEAS => Ok(AI_CHAT_MODE_IDEAS),
        AI_CHAT_MODE_WRITING => Ok(AI_CHAT_MODE_WRITING),
        _ => Err(AppError::new(
            "invalid_ai_chat_mode",
            "Der KI-Chat-Modus wird nicht unterstützt.",
        )),
    }
}

pub(crate) fn ai_chat_system_prompt(mode: &str, language: &str) -> &'static str {
    if language == "en" {
        return if mode == AI_CHAT_MODE_WRITING {
            "You are a British-English writing assistant for novelists. Perform concrete prose work directly when asked: extend, rewrite, continue or tighten scenes, write dialogue and adapt style. Preserve characters, setting, viewpoint and established facts; invent no project facts. Return final prose without meta commentary, role tokens or reasoning unless the user explicitly asks for advice. An explicitly requested different language takes precedence."
        } else {
            "You are a British-English ideas adviser for novelists. Analyse writing problems and offer useful options, structural ideas, variants, questions and concrete approaches. Write extended manuscript prose only when explicitly requested. Use available context, invent no project facts and output no role tokens or reasoning. An explicitly requested different language takes precedence."
        };
    }
    match mode {
        AI_CHAT_MODE_WRITING => {
            "Du bist eine deutsche Schreibassistenz für Romanautoren im Modus Schreibassistent. Deine Aufgabe ist konkrete Textarbeit: Szenen erweitern, umschreiben, fortsetzen, verdichten, Dialog schreiben, Stil anpassen und Prosa liefern. Wenn die letzte Nutzeranfrage eine konkrete Schreibanweisung ist, z. B. erweitere, schreibe, überarbeite, formuliere um, mach melancholischer, dramatischer oder ähnlich, dann liefere direkt den neuen oder überarbeiteten Text. Gib dann keine nummerierte Liste, keine allgemeinen Tipps, keine Methodenerklärung und keine Meta-Einleitung aus. Wiederhole keine Kontextblöcke wie 'Aktuelle Szene', 'Titel', 'Status' oder 'Kurzbeschreibung'. Bleibe textnah: Bewahre Figuren, Ort, Situation und Fakten, füge keine neuen Namen, Beziehungen oder Orte hinzu, wenn sie nicht im Kontext stehen. Setze Stimmung dosiert ein; vermeide wiederholte Motive, Pathos-Kippmomente, unpassende Neubeginns-Rhetorik und mehrfach wiederholte Sätze. Wenn der Nutzer mit 'die Szene' oder 'dahingehend' auf vorherige Nachrichten verweist, beziehe dich auf die aktuelle Szene und die letzte Stilvorgabe. Nutze vorhandenen Kontext, erfinde aber keine Projektfakten. Gib keine ChatML-, Rollen- oder Stop-Tokens aus."
        }
        _ => {
            "Du bist eine deutsche Schreibassistenz für Romanautoren im Modus Ideenberater. Deine Aufgabe ist Beratung: Analysiere Schreibprobleme, liefere kreative Optionen, Strukturideen, Varianten, Fragen und konkrete Ansatzpunkte. Du darfst Listen verwenden, wenn sie hilfreich sind. Schreibe nur dann längere Manuskriptprosa, wenn der Nutzer ausdrücklich einen Textentwurf verlangt. Nutze vorhandenen Kontext, erfinde aber keine Projektfakten. Gib keine ChatML-, Rollen- oder Stop-Tokens aus."
        }
    }
}

pub(crate) fn clean_ai_chat_content(content: &str) -> String {
    let cleaned = content
        .replace("<|im_end|>", "")
        .replace("<|im_start|>", "")
        .replace("<|endoftext|>", "")
        .trim()
        .to_string();
    let lower = cleaned.to_lowercase();
    let first_reasoning_block = ["<reasoning>", "<think>"]
        .iter()
        .filter_map(|tag| lower.find(tag))
        .min();

    if let Some(start) = first_reasoning_block {
        let answer_before_reasoning = cleaned[..start].trim();
        if !answer_before_reasoning.is_empty() {
            return answer_before_reasoning.to_string();
        }
    }

    strip_ai_reasoning_blocks(&cleaned)
        .trim_start_matches(':')
        .trim()
        .to_string()
}

pub(crate) fn call_openai_compatible_free_chat(
    settings: &AiSettings,
    input: &SendAiChatMessageInput,
    scene_context: Option<&str>,
    language: &str,
) -> Result<String, AppError> {
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let messages = build_openai_chat_messages(input, scene_context, language)?;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": messages,
        "temperature": 0.7,
        "stream": false,
        "max_tokens": 1200,
        "reasoning_effort": "none",
        "stop": ["<|im_end|>", "<|im_start|>", "<|endoftext|>"]
    }));

    let response = request_builder.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Der lokale KI-Dienst antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    let content = clean_ai_chat_content(
        parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or(""),
    );

    if content.is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keine Chat-Antwort geliefert.",
        ));
    }

    Ok(content)
}

pub(crate) fn build_openai_chat_messages(
    input: &SendAiChatMessageInput,
    scene_context: Option<&str>,
    language: &str,
) -> Result<Vec<serde_json::Value>, AppError> {
    let mode = normalize_ai_chat_mode(input)?;
    let mut system_content = ai_chat_system_prompt(mode, language).to_string();

    if let Some(context) = scene_context.filter(|context| !context.trim().is_empty()) {
        let context_instruction = if language == "en" {
            "Current scene context. If the user asks to extend or revise the scene, use this text as the basis and return a concrete new version or continuation:"
        } else {
            "Aktueller Szenenkontext. Wenn der Nutzer eine Szene erweitern oder überarbeiten will, nutze diesen Text als Arbeitsgrundlage und gib eine konkrete neue Fassung oder Erweiterung aus:"
        };
        system_content.push_str(&format!("\n\n{context_instruction}\n\n{context}"));
    }

    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": system_content
    })];

    for message in normalize_chat_messages(input)? {
        messages.push(serde_json::json!({
            "role": message.role,
            "content": message.content,
        }));
    }

    Ok(messages)
}
