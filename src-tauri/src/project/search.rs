use super::*;

#[tauri::command]
pub fn search_scenes(
    input: SearchScenesInput,
    state: State<AppState>,
) -> Result<Vec<SearchSceneResult>, AppError> {
    with_active_connection(&state, |connection| {
        search_scenes_in_connection(connection, &input.query)
    })
}

#[tauri::command]
pub fn search_project(
    input: SearchProjectInput,
    state: State<AppState>,
) -> Result<Vec<SearchProjectResult>, AppError> {
    with_active_connection(&state, |connection| {
        search_project_in_connection(connection, &input.query, &input.scope)
    })
}

pub(crate) fn search_scenes_in_connection(
    connection: &Connection,
    query: &str,
) -> Result<Vec<SearchSceneResult>, AppError> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let query_lower = query.to_lowercase();

    let mut statement = connection.prepare(
        "
        SELECT
            scene.id,
            scene.title,
            scene.summary,
            scene.text,
            chapter.title,
            manuscript_area.title
        FROM scene
        JOIN chapter ON chapter.id = scene.chapter_id
        JOIN manuscript_area ON manuscript_area.id = chapter.area_id
        WHERE scene.deleted_at IS NULL
            AND chapter.deleted_at IS NULL
            AND manuscript_area.deleted_at IS NULL
        ORDER BY
            manuscript_area.sort_order ASC,
            manuscript_area.created_at ASC,
            chapter.sort_order ASC,
            chapter.created_at ASC,
            scene.sort_order ASC,
            scene.created_at ASC
        ",
    )?;

    let rows = statement
        .query_map([], |row| {
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

    let mut results = Vec::new();
    for (scene_id, scene_title, scene_summary, scene_text, chapter_title, area_title) in rows {
        let mut matched_fields = Vec::new();
        let mut snippets = Vec::new();

        if find_case_insensitive(&scene_title, &query_lower).is_some() {
            matched_fields.push("Titel".to_string());
            snippets.push(format!(
                "Titel: {}",
                compact_search_snippet(&scene_title, query)
            ));
        }

        if find_case_insensitive(&scene_summary, &query_lower).is_some() {
            matched_fields.push("Kurzbeschreibung".to_string());
            snippets.push(format!(
                "Kurzbeschreibung: {}",
                compact_search_snippet(&scene_summary, query)
            ));
        }

        if find_case_insensitive(&scene_text, &query_lower).is_some() {
            matched_fields.push("Text".to_string());
            snippets.push(format!(
                "Text: {}",
                compact_search_snippet(&scene_text, query)
            ));
        }

        if !matched_fields.is_empty() {
            results.push(SearchSceneResult {
                scene_id,
                scene_title,
                chapter_title,
                area_title,
                matched_fields,
                snippets,
            });
        }
    }

    Ok(results)
}

pub(crate) fn search_project_in_connection(
    connection: &Connection,
    query: &str,
    scope: &str,
) -> Result<Vec<SearchProjectResult>, AppError> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let scope = normalize_search_scope(scope)?;
    let mut results = Vec::new();

    if scope == "all" || scope == "scenes" {
        results.extend(search_project_scenes_in_connection(connection, query)?);
    }

    if scope == "all" || scope == "bible" {
        results.extend(search_project_bible_in_connection(connection, query)?);
    }

    Ok(results)
}

pub(crate) fn normalize_search_scope(scope: &str) -> Result<&'static str, AppError> {
    match scope.trim() {
        "" | "all" => Ok("all"),
        "scenes" => Ok("scenes"),
        "bible" => Ok("bible"),
        _ => Err(AppError::new(
            "invalid_search_scope",
            "Der Suchbereich ist ungültig.",
        )),
    }
}

pub(crate) fn search_project_scenes_in_connection(
    connection: &Connection,
    query: &str,
) -> Result<Vec<SearchProjectResult>, AppError> {
    Ok(search_scenes_in_connection(connection, query)?
        .into_iter()
        .map(|result| SearchProjectResult {
            kind: "scene".to_string(),
            id: format!("scene:{}", result.scene_id),
            title: result.scene_title,
            subtitle: format!("{} / {}", result.area_title, result.chapter_title),
            matched_fields: result.matched_fields,
            snippets: result.snippets,
            scene_id: Some(result.scene_id),
            target_type: None,
            target_id: None,
        })
        .collect())
}

pub(crate) fn search_project_bible_in_connection(
    connection: &Connection,
    query: &str,
) -> Result<Vec<SearchProjectResult>, AppError> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    let mut characters = connection.prepare(
        "
        SELECT id, name, role, description, notes
        FROM character
        WHERE deleted_at IS NULL
        ORDER BY name COLLATE NOCASE ASC, created_at ASC
        ",
    )?;
    let character_rows = characters
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, name, role, description, notes) in character_rows {
        push_bible_search_result(
            &mut results,
            query,
            &query_lower,
            BibleSearchCandidate {
                target_type: "character",
                target_id: &id,
                title: &name,
                subtitle: "Figur",
                fields: &[
                    ("Name", name.as_str()),
                    ("Rolle", role.as_str()),
                    ("Beschreibung", description.as_str()),
                    ("Notizen", notes.as_str()),
                ],
            },
        );
    }

    let mut locations = connection.prepare(
        "
        SELECT id, name, description, notes
        FROM location
        WHERE deleted_at IS NULL
        ORDER BY name COLLATE NOCASE ASC, created_at ASC
        ",
    )?;
    let location_rows = locations
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, name, description, notes) in location_rows {
        push_bible_search_result(
            &mut results,
            query,
            &query_lower,
            BibleSearchCandidate {
                target_type: "location",
                target_id: &id,
                title: &name,
                subtitle: "Ort",
                fields: &[
                    ("Name", name.as_str()),
                    ("Beschreibung", description.as_str()),
                    ("Notizen", notes.as_str()),
                ],
            },
        );
    }

    let mut items = connection.prepare(
        "
        SELECT id, name, description, notes
        FROM item
        WHERE deleted_at IS NULL
        ORDER BY name COLLATE NOCASE ASC, created_at ASC
        ",
    )?;
    let item_rows = items
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, name, description, notes) in item_rows {
        push_bible_search_result(
            &mut results,
            query,
            &query_lower,
            BibleSearchCandidate {
                target_type: "item",
                target_id: &id,
                title: &name,
                subtitle: "Gegenstand",
                fields: &[
                    ("Name", name.as_str()),
                    ("Beschreibung", description.as_str()),
                    ("Notizen", notes.as_str()),
                ],
            },
        );
    }

    let mut notes = connection.prepare(
        "
        SELECT id, title, body, note_type
        FROM note
        WHERE deleted_at IS NULL
        ORDER BY title COLLATE NOCASE ASC, created_at ASC
        ",
    )?;
    let note_rows = notes
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, title, body, note_type) in note_rows {
        push_bible_search_result(
            &mut results,
            query,
            &query_lower,
            BibleSearchCandidate {
                target_type: "note",
                target_id: &id,
                title: &title,
                subtitle: "Notiz",
                fields: &[
                    ("Titel", title.as_str()),
                    ("Inhalt", body.as_str()),
                    ("Notiztyp", note_type.as_str()),
                ],
            },
        );
    }

    for faction in read_factions(connection)? {
        push_bible_search_result(
            &mut results,
            query,
            &query_lower,
            BibleSearchCandidate {
                target_type: "faction",
                target_id: &faction.id,
                title: &faction.name,
                subtitle: "Fraktion",
                fields: &[
                    ("Name", &faction.name),
                    ("Art", &faction.faction_type),
                    ("Beschreibung", &faction.description),
                    ("Ziele", &faction.goals),
                    ("Notizen", &faction.notes),
                ],
            },
        );
    }
    for event in read_events(connection)? {
        push_bible_search_result(
            &mut results,
            query,
            &query_lower,
            BibleSearchCandidate {
                target_type: "event",
                target_id: &event.id,
                title: &event.title,
                subtitle: "Ereignis",
                fields: &[
                    ("Titel", &event.title),
                    ("Zeit", &event.time_label),
                    ("Zusammenfassung", &event.summary),
                    ("Verlauf/Ausgang", &event.outcome),
                    ("Notizen", &event.notes),
                ],
            },
        );
    }
    for system in read_world_systems(connection)? {
        push_bible_search_result(
            &mut results,
            query,
            &query_lower,
            BibleSearchCandidate {
                target_type: "world_system",
                target_id: &system.id,
                title: &system.name,
                subtitle: "Weltsystem",
                fields: &[
                    ("Name", &system.name),
                    ("Kategorie", &system.category),
                    ("Grundprinzip", &system.principle),
                    ("Regeln", &system.rules),
                    ("Möglichkeiten", &system.capabilities),
                    ("Kosten", &system.costs),
                    ("Grenzen", &system.limits),
                    ("Zugang", &system.access),
                    ("Verbreitung", &system.distribution),
                    ("Auswirkungen", &system.world_impact),
                    ("Offene Fragen", &system.open_questions),
                    ("Notizen", &system.notes),
                ],
            },
        );
    }

    Ok(results)
}

pub(crate) struct BibleSearchCandidate<'a> {
    target_type: &'a str,
    target_id: &'a str,
    title: &'a str,
    subtitle: &'a str,
    fields: &'a [(&'a str, &'a str)],
}

pub(crate) fn push_bible_search_result(
    results: &mut Vec<SearchProjectResult>,
    query: &str,
    query_lower: &str,
    candidate: BibleSearchCandidate<'_>,
) {
    let mut matched_fields = Vec::new();
    let mut snippets = Vec::new();

    for (label, value) in candidate.fields {
        if find_case_insensitive(value, query_lower).is_some() {
            matched_fields.push((*label).to_string());
            snippets.push(format!("{label}: {}", compact_search_snippet(value, query)));
        }
    }

    if matched_fields.is_empty() {
        return;
    }

    results.push(SearchProjectResult {
        kind: "bible".to_string(),
        id: format!("{}:{}", candidate.target_type, candidate.target_id),
        title: candidate.title.to_string(),
        subtitle: candidate.subtitle.to_string(),
        matched_fields,
        snippets,
        scene_id: None,
        target_type: Some(candidate.target_type.to_string()),
        target_id: Some(candidate.target_id.to_string()),
    });
}

pub(crate) fn find_case_insensitive(haystack: &str, query_lower: &str) -> Option<usize> {
    if query_lower.is_empty() {
        return None;
    }

    haystack.char_indices().find_map(|(index, _)| {
        haystack[index..]
            .to_lowercase()
            .starts_with(query_lower)
            .then_some(index)
    })
}

pub(crate) fn compact_search_snippet(value: &str, query: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }

    let query_lower = query.to_lowercase();
    let Some(match_byte_index) = find_case_insensitive(value, &query_lower) else {
        return excerpt(value, 180);
    };

    let chars = value.chars().collect::<Vec<_>>();
    let match_char_index = value[..match_byte_index].chars().count();
    let query_char_count = query.chars().count().max(1);
    let start = match_char_index.saturating_sub(45);
    let end = (match_char_index + query_char_count + 95).min(chars.len());
    let mut snippet = chars[start..end].iter().collect::<String>();

    if start > 0 {
        snippet = format!("...{snippet}");
    }
    if end < chars.len() {
        snippet.push_str("...");
    }

    snippet
}
