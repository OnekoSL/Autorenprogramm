use super::*;

#[tauri::command]
pub fn generate_bible_card_from_keywords(
    input: GenerateBibleCardInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let request = with_active_connection(&state, |connection| {
        build_ai_bible_generate_request(connection, input)
    })?;
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    let draft = call_openai_compatible_bible_chat(&settings, &request)?;

    with_active_connection(&state, |connection| {
        insert_ai_bible_suggestion(connection, &request, draft)
    })
}

#[tauri::command]
pub fn complete_bible_card_from_keywords(
    input: CompleteBibleCardInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let request = with_active_connection(&state, |connection| {
        build_ai_bible_complete_request(connection, input)
    })?;
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    let draft = call_openai_compatible_bible_chat(&settings, &request)?;

    with_active_connection(&state, |connection| {
        insert_ai_bible_suggestion(connection, &request, draft)
    })
}

#[tauri::command]
pub fn edit_bible_card(
    input: EditBibleCardInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let request = with_active_connection(&state, |connection| {
        build_ai_bible_edit_request(connection, input)
    })?;
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    let draft = call_openai_compatible_bible_chat(&settings, &request)?;

    with_active_connection(&state, |connection| {
        insert_ai_bible_suggestion(connection, &request, draft)
    })
}

#[tauri::command]
pub fn list_ai_bible_suggestions(
    input: ListAiBibleSuggestionsInput,
    state: State<AppState>,
) -> Result<Vec<AiBibleSuggestionSnapshot>, AppError> {
    with_active_connection(&state, |connection| {
        read_ai_bible_suggestions(
            connection,
            input.target_type.as_deref(),
            input.target_id.as_deref(),
        )
    })
}

#[tauri::command]
pub fn accept_ai_bible_suggestion(
    input: AcceptAiBibleSuggestionInput,
    state: State<AppState>,
) -> Result<AiBibleAcceptResult, AppError> {
    with_active_connection(&state, |connection| {
        let (target_type, target_id) = accept_ai_bible_suggestion_in_connection(connection, input)?;
        Ok(AiBibleAcceptResult {
            snapshot: read_project_bible(connection)?,
            target_type,
            target_id,
        })
    })
}

#[tauri::command]
pub fn reject_ai_bible_suggestion(
    input: RejectAiBibleSuggestionInput,
    state: State<AppState>,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        update_ai_bible_suggestion_status(connection, &input.id, AI_STATUS_REJECTED)
    })
}

#[cfg(test)]
pub(crate) fn generate_bible_card_with_provider(
    connection: &Connection,
    input: GenerateBibleCardInput,
    provider: impl FnOnce(&AiBibleRequest) -> Result<AiBibleCardDraft, AppError>,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let request = build_ai_bible_generate_request(connection, input)?;
    let draft = provider(&request)?;

    insert_ai_bible_suggestion(connection, &request, draft)
}

#[cfg(test)]
pub(crate) fn complete_bible_card_with_provider(
    connection: &Connection,
    input: CompleteBibleCardInput,
    provider: impl FnOnce(&AiBibleRequest) -> Result<AiBibleCardDraft, AppError>,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let request = build_ai_bible_complete_request(connection, input)?;
    let draft = provider(&request)?;

    insert_ai_bible_suggestion(connection, &request, draft)
}

pub(crate) fn build_ai_bible_generate_request(
    connection: &Connection,
    input: GenerateBibleCardInput,
) -> Result<AiBibleRequest, AppError> {
    validate_target_type(&input.target_type)?;
    let keywords = normalize_keywords(&input.keywords)?;

    Ok(AiBibleRequest {
        project_id: read_active_project_id(connection)?,
        language: read_active_project_language(connection)?,
        target_type: input.target_type,
        target_id: None,
        mode: AI_BIBLE_MODE_GENERATE.to_string(),
        keywords,
        existing_card_summary: "Keine bestehende Karte.".to_string(),
        source_draft: None,
        target_updated_at: None,
    })
}

pub(crate) fn build_ai_bible_complete_request(
    connection: &Connection,
    input: CompleteBibleCardInput,
) -> Result<AiBibleRequest, AppError> {
    validate_target_type(&input.target_type)?;
    ensure_active_project_bible_card(connection, &input.target_type, &input.target_id)?;
    let keywords = normalize_keywords(&input.keywords)?;

    let (source_draft, target_updated_at) =
        read_ai_bible_source_card(connection, &input.target_type, &input.target_id)?;
    Ok(AiBibleRequest {
        project_id: read_active_project_id(connection)?,
        language: read_active_project_language(connection)?,
        target_type: input.target_type.clone(),
        target_id: Some(input.target_id.clone()),
        mode: AI_BIBLE_MODE_COMPLETE.to_string(),
        keywords,
        existing_card_summary: read_project_bible_context_card(
            connection,
            &input.target_type,
            &input.target_id,
        )?
        .unwrap_or_else(|| "Keine bestehende Karte.".to_string()),
        source_draft: Some(source_draft),
        target_updated_at: Some(target_updated_at),
    })
}

pub(crate) fn build_ai_bible_edit_request(
    connection: &Connection,
    input: EditBibleCardInput,
) -> Result<AiBibleRequest, AppError> {
    validate_target_type(&input.target_type)?;
    ensure_active_project_bible_card(connection, &input.target_type, &input.target_id)?;
    let instruction = normalize_keywords(&input.instruction)?;
    let (source_draft, target_updated_at) =
        read_ai_bible_source_card(connection, &input.target_type, &input.target_id)?;
    Ok(AiBibleRequest {
        project_id: read_active_project_id(connection)?,
        language: read_active_project_language(connection)?,
        target_type: input.target_type.clone(),
        target_id: Some(input.target_id.clone()),
        mode: AI_BIBLE_MODE_EDIT.to_string(),
        keywords: instruction,
        existing_card_summary: read_project_bible_context_card(
            connection,
            &input.target_type,
            &input.target_id,
        )?
        .unwrap_or_else(|| "Keine bestehende Karte.".to_string()),
        source_draft: Some(source_draft),
        target_updated_at: Some(target_updated_at),
    })
}

pub(crate) fn bible_system_prompt(language: &str, json_only: bool) -> &'static str {
    match (language, json_only) {
        ("en", true) => "You are a British-English writing assistant for fantasy and science-fiction authors. Turn brief keywords into concrete, atmospheric story-bible data. Remain plausible and controlled, do not merely copy the keywords, and return valid JSON only. An explicitly requested different language takes precedence.",
        ("en", false) => "You are a British-English writing assistant for fantasy and science-fiction authors. Turn brief keywords into concrete, atmospheric story-bible data. Remain plausible and controlled, do not merely copy the keywords, and provide no thinking or analysis. An explicitly requested different language takes precedence.",
        (_, true) => "Du bist eine deutschsprachige Schreibassistenz für Fantasy- und Science-Fiction-Autoren. Entwickle aus knappen Stichworten konkrete, atmosphärische Projektbibel-Daten. Bleibe plausibel und kontrolliert und antworte nur mit validem JSON. Eine ausdrücklich verlangte andere Sprache hat Vorrang.",
        _ => "Du bist eine deutschsprachige Schreibassistenz für Fantasy- und Science-Fiction-Autoren. Entwickle aus knappen Stichworten konkrete, atmosphärische Projektbibel-Daten. Bleibe plausibel und kontrolliert und antworte ohne Thinking oder Analyse. Eine ausdrücklich verlangte andere Sprache hat Vorrang.",
    }
}

pub(crate) fn call_openai_compatible_bible_chat(
    settings: &AiSettings,
    request: &AiBibleRequest,
) -> Result<AiBibleCardDraft, AppError> {
    if is_local_ollama_default(&settings.base_url) {
        return call_ollama_bible_chat(settings, request);
    }

    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let prompt = build_bible_card_prompt(request);
    let temperature = if request.mode == AI_BIBLE_MODE_GENERATE {
        0.75
    } else {
        0.4
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": [
            {
                "role": "system",
                "content": bible_system_prompt(&request.language, false)
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": temperature,
        "stream": false,
        "max_tokens": 900,
        "reasoning_effort": "none",
        "response_format": { "type": "json_object" }
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
    let message = &parsed["choices"][0]["message"];
    let content = message["content"].as_str().unwrap_or("").trim();

    if content.is_empty() {
        if message["reasoning"]
            .as_str()
            .unwrap_or("")
            .trim()
            .is_empty()
            && message["reasoning_content"]
                .as_str()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            return Err(AppError::new(
                "ai_empty_response",
                "Der lokale KI-Dienst hat keinen Kartenentwurf geliefert.",
            ));
        }

        return Err(AppError::new(
            "ai_empty_response",
            "Das lokale KI-Modell hat nur Denktext, aber keinen Kartenentwurf geliefert. Bitte erneut versuchen oder ein Nicht-Reasoning-Modell wählen.",
        ));
    }

    parse_ai_bible_draft(&request.target_type, content)
}

pub(crate) fn call_ollama_bible_chat(
    settings: &AiSettings,
    request: &AiBibleRequest,
) -> Result<AiBibleCardDraft, AppError> {
    let endpoint = format!("{}/api/chat", ollama_root_url(&settings.base_url));
    let prompt = build_bible_card_prompt(request);
    let temperature = if request.mode == AI_BIBLE_MODE_GENERATE {
        0.75
    } else {
        0.4
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let response = client
        .post(endpoint)
        .json(&serde_json::json!({
            "model": settings.model,
            "messages": [
                {
                    "role": "system",
                    "content": bible_system_prompt(&request.language, true)
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "think": false,
            "format": bible_json_schema(&request.target_type),
            "stream": false,
            "options": {
                "temperature": temperature,
                "num_predict": 900,
                "stop": ["\n```"]
            }
        }))
        .send()?;

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
            format!("Ollama antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    let content = parsed["message"]["content"].as_str().unwrap_or("").trim();

    if content.is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keinen Kartenentwurf geliefert.",
        ));
    }

    parse_ai_bible_draft(&request.target_type, content)
}

pub(crate) fn bible_json_schema(target_type: &str) -> serde_json::Value {
    let string = || serde_json::json!({ "type": "string" });
    let (properties, required) = match target_type {
        "character" => (
            serde_json::json!({
                "name": string(),
                "role": string(),
                "description": string(),
                "notes": string()
            }),
            vec!["name", "role", "description", "notes"],
        ),
        "location" | "item" => (
            serde_json::json!({
                "name": string(),
                "description": string(),
                "notes": string()
            }),
            vec!["name", "description", "notes"],
        ),
        "faction" => (
            serde_json::json!({
                "name": string(),
                "role": string(),
                "description": string(),
                "body": string(),
                "notes": string()
            }),
            vec!["name", "role", "description", "body", "notes"],
        ),
        "event" => (
            serde_json::json!({
                "title": string(),
                "role": string(),
                "description": string(),
                "body": string(),
                "notes": string()
            }),
            vec!["title", "role", "description", "body", "notes"],
        ),
        "note" => (
            serde_json::json!({
                "title": string(),
                "body": string(),
                "noteType": {
                    "type": "string",
                    "enum": ["general", "research", "lore", "faction", "event", "todo"]
                }
            }),
            vec!["title", "body", "noteType"],
        ),
        _ => (serde_json::json!({}), Vec::new()),
    };

    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

pub(crate) fn build_bible_card_prompt(request: &AiBibleRequest) -> String {
    let language_hint = ai_language_instruction(&request.language);
    let schema_hint = match request.target_type.as_str() {
        "character" => r#"{"name":"...","role":"...","description":"...","notes":"..."}"#,
        "location" | "item" => r#"{"name":"...","description":"...","notes":"..."}"#,
        "faction" => {
            r#"{"name":"...","role":"...","description":"...","body":"...","notes":"..."}"#
        }
        "event" => r#"{"title":"...","role":"...","description":"...","body":"...","notes":"..."}"#,
        "note" => {
            r#"{"title":"...","body":"...","noteType":"general|research|lore|faction|event|todo"}"#
        }
        _ => "{}",
    };
    let mode_hint = if request.mode == AI_BIBLE_MODE_COMPLETE {
        "Vervollständige die bestehende Karte. Schlage vor allem fehlende Informationen vor. Bewahre vorhandene Fakten und Ton."
    } else if request.mode == AI_BIBLE_MODE_EDIT {
        "Bearbeite oder erweitere die bestehende Karte entsprechend dem Auftrag. Du darfst gefüllte Felder ändern, musst aber nicht betroffene Fakten und den etablierten Ton bewahren. Gib die vollständige resultierende Karte aus."
    } else {
        "Erzeuge genau eine neue Projektbibel-Karte. Interpretiere die Stichworte kreativ und mache daraus eine brauchbare Romanfigur, einen Ort, Gegenstand, eine Notiz, Fraktion oder ein Ereignis."
    };
    let craft_hint = match request.target_type.as_str() {
        "character" => {
            "- Name: Wenn nur Herkunft, Klang oder Typ genannt ist, erfinde einen passenden Eigennamen; nutze niemals Adjektive wie Brutale als Namen.\n\
        - Rolle: kurz und funktional, z. B. Antagonist, Mentorin, Schmugglerkönigin.\n\
        - Beschreibung: 2 bis 4 konkrete Sätze mit Aussehen, Auftreten, innerem Antrieb und Konfliktpotenzial.\n\
        - Notizen: 2 bis 4 kurze, autorennützliche Details wie Geheimnis, Schwachpunkt, Beziehung, Motiv oder wiederkehrendes Merkmal."
        }
        "location" => {
            "- Name: einen prägnanten Ortsnamen erfinden, falls keiner genannt ist.\n\
        - Beschreibung: 2 bis 4 konkrete Sätze mit Atmosphäre, Funktion im Roman und sichtbaren Details.\n\
        - Notizen: 2 bis 4 kurze Details zu Geschichte, Gefahr, Bewohnern, Geheimnis oder Szenenpotenzial."
        }
        "item" => {
            "- Name: einen prägnanten Gegenstandsnamen erfinden, falls keiner genannt ist.\n\
        - Beschreibung: 2 bis 4 konkrete Sätze mit Aussehen, Funktion, Herkunft und erzählerischem Nutzen.\n\
        - Notizen: 2 bis 4 kurze Details zu Preis, Risiko, Besitzer, Geheimnis oder Begrenzung."
        }
        "note" => {
            "- Titel: kurz und eindeutig.\n\
        - Inhalt: 3 bis 6 konkrete Sätze, die aus den Stichworten nutzbares Projektwissen machen.\n\
        - noteType: passend aus general, research, lore, faction, event oder todo wählen."
        }
        "faction" => "- Name: prägnanter Fraktionsname.\n- role: Art der Fraktion.\n- Beschreibung: Wesen, Struktur und Auftreten.\n- body: Ziele und Interessen.\n- Notizen: Geheimnisse, Ressourcen oder interne Konflikte.",
        "event" => "- Titel: eindeutiger Ereignisname.\n- role: freie Zeitangabe.\n- Beschreibung: knappe Zusammenfassung.\n- body: Verlauf und Ausgang.\n- Notizen: Folgen, offene Fragen oder Details.",
        _ => "",
    };

    format!(
        "/no_think\n\
        {mode_hint}\n\
        Anforderungen:\n\
        - {language_hint}\n\
        - Genau ein JSON-Objekt ausgeben, ohne Markdown und ohne Erklärung.\n\
        - Keine Analyse, kein Denktext, kein reasoning.\n\
        - Schema exakt passend zum Kartentyp verwenden: {schema_hint}\n\
        - Leere Felder als leere Strings ausgeben.\n\
        - Keine Liste und keine zweite Karte erzeugen.\n\
        - Wiederhole die Stichworte nicht mechanisch; verdichte, konkretisiere und ergänze plausibel.\n\
        - Keine Meta-Kommentare wie \"aus den Stichworten\".\n\n\
        Ausarbeitungsregeln:\n{craft_hint}\n\n\
        Kartentyp: {}\n\n\
        Bestehende Karte:\n{}\n\n\
        {}:\n{}",
        request.target_type,
        request.existing_card_summary,
        if request.mode == AI_BIBLE_MODE_EDIT { "Bearbeitungsauftrag" } else { "Stichworte" },
        request.keywords
    )
}

pub(crate) fn ai_language_instruction(language: &str) -> &'static str {
    if language == "en" {
        "Use British English by default. If the user's instruction explicitly requests another language, follow that request"
    } else {
        "Antworte standardmäßig auf Deutsch. Verlangt der Benutzerauftrag ausdrücklich eine andere Sprache, hat dieser Auftrag Vorrang"
    }
}

pub(crate) fn read_ai_bible_source_card(
    connection: &Connection,
    target_type: &str,
    target_id: &str,
) -> Result<(AiBibleCardDraft, String), AppError> {
    let missing = || {
        AppError::new(
            "project_bible_card_missing",
            "Die Projektbibel-Karte wurde nicht gefunden.",
        )
    };
    match target_type {
        "character" => connection.query_row(
            "SELECT name, role, description, notes, updated_at FROM character WHERE id = ?1 AND deleted_at IS NULL",
            params![target_id],
            |row| Ok((AiBibleCardDraft { name: row.get(0)?, role: row.get(1)?, description: row.get(2)?, notes: row.get(3)?, ..AiBibleCardDraft::default() }, row.get(4)?)),
        ).optional()?.ok_or_else(missing),
        "location" | "item" => {
            let table = if target_type == "location" { "location" } else { "item" };
            connection.query_row(
                &format!("SELECT name, description, notes, updated_at FROM {table} WHERE id = ?1 AND deleted_at IS NULL"),
                params![target_id],
                |row| Ok((AiBibleCardDraft { name: row.get(0)?, description: row.get(1)?, notes: row.get(2)?, ..AiBibleCardDraft::default() }, row.get(3)?)),
            ).optional()?.ok_or_else(missing)
        }
        "faction" => connection.query_row("SELECT name, faction_type, description, goals, notes, updated_at FROM faction WHERE id=?1 AND deleted_at IS NULL", params![target_id], |row| Ok((AiBibleCardDraft { name: row.get(0)?, role: row.get(1)?, description: row.get(2)?, body: row.get(3)?, notes: row.get(4)?, ..AiBibleCardDraft::default() }, row.get(5)?))).optional()?.ok_or_else(missing),
        "event" => connection.query_row("SELECT title, time_label, summary, outcome, notes, updated_at FROM event WHERE id=?1 AND deleted_at IS NULL", params![target_id], |row| Ok((AiBibleCardDraft { title: row.get(0)?, role: row.get(1)?, description: row.get(2)?, body: row.get(3)?, notes: row.get(4)?, ..AiBibleCardDraft::default() }, row.get(5)?))).optional()?.ok_or_else(missing),
        "note" => connection.query_row(
            "SELECT title, body, note_type, updated_at FROM note WHERE id = ?1 AND deleted_at IS NULL",
            params![target_id],
            |row| Ok((AiBibleCardDraft { title: row.get(0)?, body: row.get(1)?, note_type: row.get(2)?, ..AiBibleCardDraft::default() }, row.get(3)?)),
        ).optional()?.ok_or_else(missing),
        _ => Err(AppError::new("invalid_project_bible_card_type", "Dieser Projektbibel-Kartentyp wird nicht unterstützt.")),
    }
}

pub(crate) fn parse_ai_bible_draft(
    target_type: &str,
    content: &str,
) -> Result<AiBibleCardDraft, AppError> {
    let value = parse_bible_json_value(content)?;
    if !value.is_object() {
        return Err(AppError::new(
            "ai_invalid_response",
            "Der lokale KI-Dienst hat keinen gültigen Kartenentwurf geliefert.",
        ));
    }

    let mut draft = AiBibleCardDraft::default();
    match target_type {
        "character" => {
            draft.name = json_string(&value, "name");
            draft.role = json_string(&value, "role");
            draft.description = json_string(&value, "description");
            draft.notes = json_string(&value, "notes");
        }
        "location" | "item" => {
            draft.name = json_string(&value, "name");
            draft.description = json_string(&value, "description");
            draft.notes = json_string(&value, "notes");
        }
        "faction" => {
            draft.name = json_string(&value, "name");
            draft.role = json_string(&value, "role");
            draft.description = json_string(&value, "description");
            draft.body = json_string(&value, "body");
            draft.notes = json_string(&value, "notes");
        }
        "event" => {
            draft.title = json_string(&value, "title");
            draft.role = json_string(&value, "role");
            draft.description = json_string(&value, "description");
            draft.body = json_string(&value, "body");
            draft.notes = json_string(&value, "notes");
        }
        "note" => {
            draft.title = json_string(&value, "title");
            draft.body = json_string(&value, "body");
            draft.note_type = normalize_note_type(
                value["noteType"]
                    .as_str()
                    .or_else(|| value["note_type"].as_str()),
            );
        }
        _ => {
            return Err(AppError::new(
                "invalid_project_bible_card_type",
                "Dieser Projektbibel-Kartentyp wird nicht unterstützt.",
            ));
        }
    }

    Ok(normalize_ai_bible_draft(target_type, draft))
}

pub(crate) fn parse_bible_json_value(content: &str) -> Result<serde_json::Value, AppError> {
    let stripped = strip_json_code_fence(content);
    let candidate = extract_first_json_object(stripped).unwrap_or(stripped);
    match serde_json::from_str(candidate) {
        Ok(value) => Ok(value),
        Err(_) => {
            let repaired = quote_known_json_keys(candidate);
            serde_json::from_str(&repaired).map_err(|_| {
                AppError::new(
                    "ai_invalid_response",
                    "Der lokale KI-Dienst hat keinen gültigen JSON-Kartenentwurf geliefert. Bitte erneut versuchen oder ein anderes Modell wählen.",
                )
            })
        }
    }
}

pub(crate) fn extract_first_json_object(content: &str) -> Option<&str> {
    let mut depth = 0;
    let mut start = None;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in content.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        if character == '"' {
            in_string = true;
        } else if character == '{' {
            if depth == 0 {
                start = Some(index);
            }
            depth += 1;
        } else if character == '}' && depth > 0 {
            depth -= 1;
            if depth == 0 {
                return start.map(|start| &content[start..=index]);
            }
        }
    }

    None
}

pub(crate) fn quote_known_json_keys(content: &str) -> String {
    let mut repaired = content.to_string();
    for key in [
        "name",
        "role",
        "description",
        "notes",
        "title",
        "body",
        "noteType",
        "note_type",
    ] {
        for prefix in ["{", ","] {
            repaired = repaired.replace(&format!("{prefix}{key}:"), &format!("{prefix}\"{key}\":"));
            repaired =
                repaired.replace(&format!("{prefix} {key}:"), &format!("{prefix} \"{key}\":"));
            repaired = repaired.replace(
                &format!("{prefix}\n  {key}:"),
                &format!("{prefix}\n  \"{key}\":"),
            );
            repaired = repaired.replace(
                &format!("{prefix}\n    {key}:"),
                &format!("{prefix}\n    \"{key}\":"),
            );
        }
    }

    repaired
}

pub(crate) fn strip_json_code_fence(content: &str) -> &str {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed;
    }

    let without_opening = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```JSON")
        .trim_start_matches("```")
        .trim();

    without_opening
        .strip_suffix("```")
        .unwrap_or(without_opening)
        .trim()
}

pub(crate) fn json_string(value: &serde_json::Value, key: &str) -> String {
    value[key].as_str().unwrap_or("").trim().to_string()
}

pub(crate) fn normalize_ai_bible_draft(
    target_type: &str,
    mut draft: AiBibleCardDraft,
) -> AiBibleCardDraft {
    draft.name = draft.name.trim().to_string();
    draft.role = draft.role.trim().to_string();
    draft.description = draft.description.trim().to_string();
    draft.notes = draft.notes.trim().to_string();
    draft.title = draft.title.trim().to_string();
    draft.body = draft.body.trim().to_string();
    draft.note_type = normalize_note_type(Some(&draft.note_type));

    if !matches!(target_type, "note" | "event") {
        draft.title.clear();
        if target_type != "faction" {
            draft.body.clear();
        }
        draft.note_type = "general".to_string();
    }

    if !matches!(target_type, "character" | "faction" | "event") {
        draft.role.clear();
    }

    if target_type == "note" {
        draft.name.clear();
        draft.role.clear();
        draft.description.clear();
        draft.notes.clear();
    }
    if target_type == "event" {
        draft.name.clear();
        draft.note_type = "general".to_string();
    }

    draft
}

pub(crate) fn normalize_keywords(keywords: &str) -> Result<String, AppError> {
    let keywords = keywords.trim();
    if keywords.is_empty() {
        return Err(AppError::new(
            "ai_bible_keywords_empty",
            "Bitte gib Stichworte für die Projektbibel-KI ein.",
        ));
    }

    Ok(keywords.to_string())
}

pub(crate) fn insert_ai_bible_suggestion(
    connection: &Connection,
    request: &AiBibleRequest,
    draft: AiBibleCardDraft,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let now = timestamp();
    let id = Uuid::new_v4().to_string();
    let draft = normalize_ai_bible_draft(&request.target_type, draft);
    let result_json = serde_json::to_string(&draft)?;
    let source_json = request
        .source_draft
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;

    connection.execute(
        "
        INSERT INTO ai_bible_suggestion (
            id, project_id, target_type, target_id, mode, keywords, result_json,
            source_json, target_updated_at, status,
            created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, NULL)
        ",
        params![
            id,
            request.project_id,
            request.target_type,
            request.target_id,
            request.mode,
            request.keywords,
            result_json,
            source_json,
            request.target_updated_at,
            AI_STATUS_DRAFT,
            now
        ],
    )?;

    read_ai_bible_suggestion(connection, &id)
}

pub(crate) fn read_ai_bible_suggestion(
    connection: &Connection,
    suggestion_id: &str,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    connection
        .query_row(
            "
            SELECT id, project_id, target_type, target_id, mode, keywords, result_json,
                source_json, target_updated_at,
                status, created_at, updated_at, deleted_at
            FROM ai_bible_suggestion
            WHERE id = ?1 AND deleted_at IS NULL
            ",
            params![suggestion_id],
            map_ai_bible_suggestion_row,
        )
        .optional()?
        .ok_or_else(|| {
            AppError::new(
                "ai_bible_suggestion_missing",
                "Der Projektbibel-KI-Vorschlag wurde nicht gefunden.",
            )
        })
}

pub(crate) fn read_ai_bible_suggestions(
    connection: &Connection,
    target_type: Option<&str>,
    target_id: Option<&str>,
) -> Result<Vec<AiBibleSuggestionSnapshot>, AppError> {
    if let Some(target_type) = target_type {
        validate_target_type(target_type)?;
    }

    let mut statement = connection.prepare(
        "
        SELECT id, project_id, target_type, target_id, mode, keywords, result_json,
            source_json, target_updated_at,
            status, created_at, updated_at, deleted_at
        FROM ai_bible_suggestion
        WHERE deleted_at IS NULL
            AND (?1 IS NULL OR target_type = ?1)
            AND (
                ?2 IS NULL
                OR target_id = ?2
                OR (target_id IS NULL AND ?2 = '')
            )
        ORDER BY created_at DESC
        ",
    )?;

    let suggestions = statement
        .query_map(params![target_type, target_id], map_ai_bible_suggestion_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(suggestions)
}

pub(crate) fn map_ai_bible_suggestion_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<AiBibleSuggestionSnapshot> {
    let target_type: String = row.get(2)?;
    let result_json: String = row.get(6)?;
    let draft = serde_json::from_str::<AiBibleCardDraft>(&result_json)
        .map(|draft| normalize_ai_bible_draft(&target_type, draft))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    let source_json: Option<String> = row.get(7)?;
    let source_draft = source_json
        .as_deref()
        .map(serde_json::from_str::<AiBibleCardDraft>)
        .transpose()
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                7,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?
        .map(|draft| normalize_ai_bible_draft(&target_type, draft));

    Ok(AiBibleSuggestionSnapshot {
        id: row.get(0)?,
        project_id: row.get(1)?,
        target_type,
        target_id: row.get(3)?,
        mode: row.get(4)?,
        keywords: row.get(5)?,
        draft,
        source_draft,
        target_updated_at: row.get(8)?,
        status: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        deleted_at: row.get(12)?,
    })
}

pub(crate) fn accept_ai_bible_suggestion_in_connection(
    connection: &mut Connection,
    input: AcceptAiBibleSuggestionInput,
) -> Result<(String, String), AppError> {
    let suggestion = read_ai_bible_suggestion(connection, &input.id)?;
    if suggestion.status != AI_STATUS_DRAFT {
        return Err(AppError::new(
            "ai_suggestion_not_draft",
            "Nur offene KI-Vorschläge können übernommen werden.",
        ));
    }

    validate_target_type(&suggestion.target_type)?;
    let draft = normalize_ai_bible_draft(&suggestion.target_type, input.draft);
    let now = timestamp();
    let transaction = connection.transaction()?;
    let target_id = if suggestion.mode == AI_BIBLE_MODE_GENERATE {
        create_project_bible_card_from_draft(&transaction, &suggestion, &draft, &now)?
    } else if suggestion.mode == AI_BIBLE_MODE_COMPLETE {
        let target_id = suggestion.target_id.clone().ok_or_else(|| {
            AppError::new(
                "ai_bible_target_missing",
                "Der Projektbibel-KI-Vorschlag hat keine Zielkarte.",
            )
        })?;
        complete_project_bible_card_from_draft(
            &transaction,
            &suggestion.target_type,
            &target_id,
            &draft,
            &now,
        )?;
        target_id
    } else if suggestion.mode == AI_BIBLE_MODE_EDIT {
        let target_id = suggestion.target_id.clone().ok_or_else(|| {
            AppError::new(
                "ai_bible_target_missing",
                "Der Projektbibel-KI-Vorschlag hat keine Zielkarte.",
            )
        })?;
        edit_project_bible_card_from_draft(
            &transaction,
            &suggestion,
            &target_id,
            &draft,
            &input.selected_fields,
            &now,
        )?;
        target_id
    } else {
        return Err(AppError::new(
            "ai_bible_mode_invalid",
            "Dieser Projektbibel-KI-Modus wird nicht unterstützt.",
        ));
    };

    transaction.execute(
        "
        UPDATE ai_bible_suggestion
        SET status = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        ",
        params![AI_STATUS_ACCEPTED, now, input.id],
    )?;
    transaction.commit()?;

    Ok((suggestion.target_type, target_id))
}

pub(crate) fn edit_project_bible_card_from_draft(
    connection: &Connection,
    suggestion: &AiBibleSuggestionSnapshot,
    target_id: &str,
    draft: &AiBibleCardDraft,
    selected_fields: &[String],
    now: &str,
) -> Result<(), AppError> {
    let allowed: &[&str] = match suggestion.target_type.as_str() {
        "character" => &["name", "role", "description", "notes"],
        "location" | "item" => &["name", "description", "notes"],
        "note" => &["title", "body", "noteType"],
        "faction" => &["name", "role", "description", "body", "notes"],
        "event" => &["title", "role", "description", "body", "notes"],
        _ => &[],
    };
    if selected_fields.is_empty() {
        return Err(AppError::new(
            "ai_bible_no_fields_selected",
            "Wähle mindestens ein geändertes Feld zur Übernahme aus.",
        ));
    }
    if selected_fields
        .iter()
        .any(|field| !allowed.contains(&field.as_str()))
    {
        return Err(AppError::new(
            "ai_bible_field_invalid",
            "Der Vorschlag enthält ein für diesen Kartentyp ungültiges Feld.",
        ));
    }
    let (current_draft, current_updated_at) =
        read_ai_bible_source_card(connection, &suggestion.target_type, target_id)?;
    if suggestion.target_updated_at.as_deref() != Some(current_updated_at.as_str())
        || suggestion.source_draft.as_ref() != Some(&current_draft)
    {
        return Err(AppError::new(
            "ai_bible_edit_conflict",
            "Die Projektbibel-Karte wurde seit Erstellung des KI-Vorschlags geändert. Erstelle bitte einen neuen Vorschlag.",
        ));
    }
    let has = |field: &str| selected_fields.iter().any(|selected| selected == field);
    let changed = match suggestion.target_type.as_str() {
        "character" => connection.execute(
            "UPDATE character SET name = CASE WHEN ?1 THEN ?2 ELSE name END, role = CASE WHEN ?3 THEN ?4 ELSE role END, description = CASE WHEN ?5 THEN ?6 ELSE description END, notes = CASE WHEN ?7 THEN ?8 ELSE notes END, updated_at = ?9 WHERE id = ?10 AND deleted_at IS NULL",
            params![has("name"), normalize_title(Some(&draft.name), "Neue Figur"), has("role"), draft.role, has("description"), draft.description, has("notes"), draft.notes, now, target_id],
        )?,
        "location" | "item" => {
            let table = if suggestion.target_type == "location" { "location" } else { "item" };
            let default_name = if suggestion.target_type == "location" { "Neuer Ort" } else { "Neuer Gegenstand" };
            connection.execute(
                &format!("UPDATE {table} SET name = CASE WHEN ?1 THEN ?2 ELSE name END, description = CASE WHEN ?3 THEN ?4 ELSE description END, notes = CASE WHEN ?5 THEN ?6 ELSE notes END, updated_at = ?7 WHERE id = ?8 AND deleted_at IS NULL"),
                params![has("name"), normalize_title(Some(&draft.name), default_name), has("description"), draft.description, has("notes"), draft.notes, now, target_id],
            )?
        }
        "note" => connection.execute(
            "UPDATE note SET title = CASE WHEN ?1 THEN ?2 ELSE title END, body = CASE WHEN ?3 THEN ?4 ELSE body END, note_type = CASE WHEN ?5 THEN ?6 ELSE note_type END, updated_at = ?7 WHERE id = ?8 AND deleted_at IS NULL",
            params![has("title"), normalize_title(Some(&draft.title), "Neue Notiz"), has("body"), draft.body, has("noteType"), normalize_note_type(Some(&draft.note_type)), now, target_id],
        )?,
        "faction" => connection.execute("UPDATE faction SET name=CASE WHEN ?1 THEN ?2 ELSE name END, faction_type=CASE WHEN ?3 THEN ?4 ELSE faction_type END, description=CASE WHEN ?5 THEN ?6 ELSE description END, goals=CASE WHEN ?7 THEN ?8 ELSE goals END, notes=CASE WHEN ?9 THEN ?10 ELSE notes END, updated_at=?11 WHERE id=?12 AND deleted_at IS NULL", params![has("name"), normalize_title(Some(&draft.name), "Neue Fraktion"), has("role"), draft.role, has("description"), draft.description, has("body"), draft.body, has("notes"), draft.notes, now, target_id])?,
        "event" => connection.execute("UPDATE event SET title=CASE WHEN ?1 THEN ?2 ELSE title END, time_label=CASE WHEN ?3 THEN ?4 ELSE time_label END, summary=CASE WHEN ?5 THEN ?6 ELSE summary END, outcome=CASE WHEN ?7 THEN ?8 ELSE outcome END, notes=CASE WHEN ?9 THEN ?10 ELSE notes END, updated_at=?11 WHERE id=?12 AND deleted_at IS NULL", params![has("title"), normalize_title(Some(&draft.title), "Neues Ereignis"), has("role"), draft.role, has("description"), draft.description, has("body"), draft.body, has("notes"), draft.notes, now, target_id])?,
        _ => 0,
    };
    require_changed(
        changed,
        "project_bible_card_missing",
        "Die Projektbibel-Karte wurde nicht gefunden.",
    )
}

pub(crate) fn create_project_bible_card_from_draft(
    connection: &Connection,
    suggestion: &AiBibleSuggestionSnapshot,
    draft: &AiBibleCardDraft,
    now: &str,
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();

    match suggestion.target_type.as_str() {
        "character" => {
            connection.execute(
                "
                INSERT INTO character (
                    id, project_id, name, role, description, notes, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, NULL)
                ",
                params![
                    id,
                    suggestion.project_id,
                    normalize_title(Some(&draft.name), "Neue Figur"),
                    draft.role,
                    draft.description,
                    draft.notes,
                    now
                ],
            )?;
        }
        "location" => {
            connection.execute(
                "
                INSERT INTO location (
                    id, project_id, name, description, notes, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, NULL)
                ",
                params![
                    id,
                    suggestion.project_id,
                    normalize_title(Some(&draft.name), "Neuer Ort"),
                    draft.description,
                    draft.notes,
                    now
                ],
            )?;
        }
        "item" => {
            connection.execute(
                "
                INSERT INTO item (
                    id, project_id, name, description, notes, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, NULL)
                ",
                params![
                    id,
                    suggestion.project_id,
                    normalize_title(Some(&draft.name), "Neuer Gegenstand"),
                    draft.description,
                    draft.notes,
                    now
                ],
            )?;
        }
        "note" => {
            connection.execute(
                "
                INSERT INTO note (
                    id, project_id, title, body, note_type, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, NULL)
                ",
                params![
                    id,
                    suggestion.project_id,
                    normalize_title(Some(&draft.title), "Neue Notiz"),
                    draft.body,
                    normalize_note_type(Some(&draft.note_type)),
                    now
                ],
            )?;
        }
        "faction" => {
            connection.execute("INSERT INTO faction (id, project_id, name, faction_type, description, goals, notes, created_at, updated_at, deleted_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,NULL)", params![id, suggestion.project_id, normalize_title(Some(&draft.name), "Neue Fraktion"), draft.role, draft.description, draft.body, draft.notes, now])?;
        }
        "event" => {
            let order: i64 = connection.query_row("SELECT COALESCE(MAX(sort_order),0)+1 FROM event WHERE project_id=?1 AND deleted_at IS NULL", params![suggestion.project_id], |row| row.get(0))?;
            connection.execute("INSERT INTO event (id, project_id, title, time_label, summary, outcome, notes, sort_order, created_at, updated_at, deleted_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,NULL)", params![id, suggestion.project_id, normalize_title(Some(&draft.title), "Neues Ereignis"), draft.role, draft.description, draft.body, draft.notes, order, now])?;
        }
        _ => unreachable!(),
    }

    Ok(id)
}

pub(crate) fn complete_project_bible_card_from_draft(
    connection: &Connection,
    target_type: &str,
    target_id: &str,
    draft: &AiBibleCardDraft,
    now: &str,
) -> Result<(), AppError> {
    let changed = match target_type {
        "character" => {
            let current = connection
                .query_row(
                    "
                    SELECT name, role, description, notes
                    FROM character
                    WHERE id = ?1 AND deleted_at IS NULL
                    ",
                    params![target_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| {
                    AppError::new(
                        "project_bible_card_missing",
                        "Die Projektbibel-Karte wurde nicht gefunden.",
                    )
                })?;
            connection.execute(
                "
                UPDATE character
                SET name = ?1, role = ?2, description = ?3, notes = ?4, updated_at = ?5
                WHERE id = ?6 AND deleted_at IS NULL
                ",
                params![
                    fill_if_blank_or_default(&current.0, &draft.name, "Neue Figur"),
                    fill_if_blank(&current.1, &draft.role),
                    fill_if_blank(&current.2, &draft.description),
                    fill_if_blank(&current.3, &draft.notes),
                    now,
                    target_id
                ],
            )?
        }
        "location" => {
            let current = connection
                .query_row(
                    "
                    SELECT name, description, notes
                    FROM location
                    WHERE id = ?1 AND deleted_at IS NULL
                    ",
                    params![target_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| {
                    AppError::new(
                        "project_bible_card_missing",
                        "Die Projektbibel-Karte wurde nicht gefunden.",
                    )
                })?;
            connection.execute(
                "
                UPDATE location
                SET name = ?1, description = ?2, notes = ?3, updated_at = ?4
                WHERE id = ?5 AND deleted_at IS NULL
                ",
                params![
                    fill_if_blank_or_default(&current.0, &draft.name, "Neuer Ort"),
                    fill_if_blank(&current.1, &draft.description),
                    fill_if_blank(&current.2, &draft.notes),
                    now,
                    target_id
                ],
            )?
        }
        "item" => {
            let current = connection
                .query_row(
                    "
                    SELECT name, description, notes
                    FROM item
                    WHERE id = ?1 AND deleted_at IS NULL
                    ",
                    params![target_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| {
                    AppError::new(
                        "project_bible_card_missing",
                        "Die Projektbibel-Karte wurde nicht gefunden.",
                    )
                })?;
            connection.execute(
                "
                UPDATE item
                SET name = ?1, description = ?2, notes = ?3, updated_at = ?4
                WHERE id = ?5 AND deleted_at IS NULL
                ",
                params![
                    fill_if_blank_or_default(&current.0, &draft.name, "Neuer Gegenstand"),
                    fill_if_blank(&current.1, &draft.description),
                    fill_if_blank(&current.2, &draft.notes),
                    now,
                    target_id
                ],
            )?
        }
        "note" => {
            let current = connection
                .query_row(
                    "
                    SELECT title, body, note_type
                    FROM note
                    WHERE id = ?1 AND deleted_at IS NULL
                    ",
                    params![target_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| {
                    AppError::new(
                        "project_bible_card_missing",
                        "Die Projektbibel-Karte wurde nicht gefunden.",
                    )
                })?;
            connection.execute(
                "
                UPDATE note
                SET title = ?1, body = ?2, note_type = ?3, updated_at = ?4
                WHERE id = ?5 AND deleted_at IS NULL
                ",
                params![
                    fill_if_blank_or_default(&current.0, &draft.title, "Neue Notiz"),
                    fill_if_blank(&current.1, &draft.body),
                    current.2,
                    now,
                    target_id
                ],
            )?
        }
        _ => unreachable!(),
    };

    require_changed(
        changed,
        "project_bible_card_missing",
        "Die Projektbibel-Karte wurde nicht gefunden.",
    )
}

pub(crate) fn fill_if_blank(current: &str, proposed: &str) -> String {
    if current.trim().is_empty() {
        proposed.trim().to_string()
    } else {
        current.to_string()
    }
}

pub(crate) fn fill_if_blank_or_default(
    current: &str,
    proposed: &str,
    default_value: &str,
) -> String {
    if current.trim().is_empty() || current.trim() == default_value {
        normalize_title(Some(proposed), default_value)
    } else {
        current.to_string()
    }
}

pub(crate) fn update_ai_bible_suggestion_status(
    connection: &Connection,
    suggestion_id: &str,
    status: &str,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let suggestion = read_ai_bible_suggestion(connection, suggestion_id)?;
    if suggestion.status != AI_STATUS_DRAFT {
        return Err(AppError::new(
            "ai_suggestion_not_draft",
            "Nur offene KI-Vorschläge können verworfen werden.",
        ));
    }

    let now = timestamp();
    connection.execute(
        "
        UPDATE ai_bible_suggestion
        SET status = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        ",
        params![status, now, suggestion_id],
    )?;

    read_ai_bible_suggestion(connection, suggestion_id)
}
