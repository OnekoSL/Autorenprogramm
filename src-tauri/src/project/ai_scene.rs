use super::*;

#[tauri::command]
pub fn summarize_scene(
    input: SummarizeSceneInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<AiSuggestionSnapshot, AppError> {
    let request = with_active_connection(&state, |connection| {
        build_ai_scene_request(connection, &input)
    })?;
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    let result = call_openai_compatible_chat(&settings, &request)?;

    with_active_connection(&state, |connection| {
        insert_ai_suggestion(connection, &request, result)
    })
}

#[tauri::command]
pub fn check_scene_contradictions(
    input: SummarizeSceneInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<AiSuggestionSnapshot, AppError> {
    let request = with_active_connection(&state, |connection| {
        build_ai_contradiction_request(connection, &input)
    })?;
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    let result = call_openai_compatible_contradictions(&settings, &request)?;

    with_active_connection(&state, |connection| {
        insert_ai_contradiction_suggestion(connection, &request, result)
    })
}

#[tauri::command]
pub fn list_ai_suggestions_for_scene(
    input: SummarizeSceneInput,
    state: State<AppState>,
) -> Result<Vec<AiSuggestionSnapshot>, AppError> {
    with_active_connection(&state, |connection| {
        ensure_active_scene(connection, &input.scene_id)?;
        read_ai_suggestions_for_scene(connection, &input.scene_id)
    })
}

#[tauri::command]
pub fn accept_ai_suggestion(
    input: AiSuggestionInput,
    state: State<AppState>,
) -> Result<AiSuggestionSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        accept_ai_suggestion_in_connection(connection, input)
    })
}

#[tauri::command]
pub fn reject_ai_suggestion(
    input: AiSuggestionInput,
    state: State<AppState>,
) -> Result<AiSuggestionSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        update_ai_suggestion_status(connection, &input.id, AI_STATUS_REJECTED)
    })
}

#[cfg(test)]
pub(crate) fn summarize_scene_with_provider(
    connection: &Connection,
    input: &SummarizeSceneInput,
    provider: impl FnOnce(&AiSceneRequest) -> Result<String, AppError>,
) -> Result<AiSuggestionSnapshot, AppError> {
    let request = build_ai_scene_request(connection, input)?;
    let result = provider(&request)?;

    insert_ai_suggestion(connection, &request, result)
}

#[cfg(test)]
pub(crate) fn check_scene_contradictions_with_provider(
    connection: &Connection,
    input: &SummarizeSceneInput,
    provider: impl FnOnce(&AiContradictionRequest) -> Result<String, AppError>,
) -> Result<AiSuggestionSnapshot, AppError> {
    let request = build_ai_contradiction_request(connection, input)?;
    let result = provider(&request)?;

    insert_ai_contradiction_suggestion(connection, &request, result)
}

pub(crate) fn build_summarize_scene_prompt(request: &AiSceneRequest) -> String {
    let language_hint = ai_language_instruction(&request.language);
    format!(
        "Fasse die folgende Szene für das Feld Kurzbeschreibung zusammen.\n\
        Anforderungen:\n\
        - {language_hint}\n\
        - 2 bis 4 kurze Sätze.\n\
        - Neutral und konkret.\n\
        - Keine neuen Fakten erfinden.\n\
        - Nur die Zusammenfassung ausgeben.\n\n\
        Szenentitel: {}\n\n\
        Bisherige Kurzbeschreibung: {}\n\n\
        Projektkontext:\n{}\n\n\
        Szenentext:\n{}",
        request.scene_title,
        if request.scene_summary.trim().is_empty() {
            "Keine"
        } else {
            request.scene_summary.trim()
        },
        request.context_summary,
        request.scene_text
    )
}

pub(crate) fn insert_ai_suggestion(
    connection: &Connection,
    request: &AiSceneRequest,
    result: String,
) -> Result<AiSuggestionSnapshot, AppError> {
    insert_ai_suggestion_record(
        connection,
        AiSuggestionRecordInput {
            project_id: &request.project_id,
            scene_id: &request.scene_id,
            action_type: AI_ACTION_SUMMARIZE_SCENE,
            input_text: &request.scene_text,
            context_summary: &request.context_summary,
            result,
            empty_message: "Der lokale KI-Dienst hat keine Zusammenfassung geliefert.",
        },
    )
}

pub(crate) fn insert_ai_contradiction_suggestion(
    connection: &Connection,
    request: &AiContradictionRequest,
    result: String,
) -> Result<AiSuggestionSnapshot, AppError> {
    insert_ai_suggestion_record(
        connection,
        AiSuggestionRecordInput {
            project_id: &request.project_id,
            scene_id: &request.scene_id,
            action_type: AI_ACTION_CHECK_SCENE_CONTRADICTIONS,
            input_text: &request.scene_text,
            context_summary: &request.context_summary,
            result,
            empty_message: "Der lokale KI-Dienst hat keine Widerspruchsprüfung geliefert.",
        },
    )
}

pub(crate) struct AiSuggestionRecordInput<'a> {
    project_id: &'a str,
    scene_id: &'a str,
    action_type: &'a str,
    input_text: &'a str,
    context_summary: &'a str,
    result: String,
    empty_message: &'a str,
}

pub(crate) fn insert_ai_suggestion_record(
    connection: &Connection,
    input: AiSuggestionRecordInput<'_>,
) -> Result<AiSuggestionSnapshot, AppError> {
    ensure_active_scene(connection, input.scene_id)?;
    let now = timestamp();
    let id = Uuid::new_v4().to_string();
    let input_excerpt = excerpt(input.input_text, 800);
    let result = input.result.trim();
    if result.is_empty() {
        return Err(AppError::new("ai_empty_response", input.empty_message));
    }

    connection.execute(
        "
        INSERT INTO ai_suggestion (
            id, project_id, scene_id, action_type, input_excerpt, context_summary, result, status,
            created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, NULL)
        ",
        params![
            id,
            input.project_id,
            input.scene_id,
            input.action_type,
            input_excerpt,
            input.context_summary,
            result,
            AI_STATUS_DRAFT,
            now
        ],
    )?;

    read_ai_suggestion(connection, &id)
}

pub(crate) fn read_ai_suggestion(
    connection: &Connection,
    suggestion_id: &str,
) -> Result<AiSuggestionSnapshot, AppError> {
    connection
        .query_row(
            "
            SELECT id, project_id, scene_id, action_type, input_excerpt, context_summary, result,
                status, created_at, updated_at, deleted_at
            FROM ai_suggestion
            WHERE id = ?1 AND deleted_at IS NULL
            ",
            params![suggestion_id],
            map_ai_suggestion_row,
        )
        .optional()?
        .ok_or_else(|| {
            AppError::new(
                "ai_suggestion_missing",
                "Der KI-Vorschlag wurde nicht gefunden.",
            )
        })
}

pub(crate) fn read_ai_suggestions_for_scene(
    connection: &Connection,
    scene_id: &str,
) -> Result<Vec<AiSuggestionSnapshot>, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT id, project_id, scene_id, action_type, input_excerpt, context_summary, result,
            status, created_at, updated_at, deleted_at
        FROM ai_suggestion
        WHERE scene_id = ?1 AND deleted_at IS NULL
        ORDER BY created_at DESC
        ",
    )?;

    let suggestions = statement
        .query_map(params![scene_id], map_ai_suggestion_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(suggestions)
}

pub(crate) fn map_ai_suggestion_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<AiSuggestionSnapshot> {
    Ok(AiSuggestionSnapshot {
        id: row.get(0)?,
        project_id: row.get(1)?,
        scene_id: row.get(2)?,
        action_type: row.get(3)?,
        input_excerpt: row.get(4)?,
        context_summary: row.get(5)?,
        result: row.get(6)?,
        status: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        deleted_at: row.get(10)?,
    })
}

pub(crate) fn accept_ai_suggestion_in_connection(
    connection: &mut Connection,
    input: AiSuggestionInput,
) -> Result<AiSuggestionSnapshot, AppError> {
    let suggestion = read_ai_suggestion(connection, &input.id)?;
    if suggestion.action_type != AI_ACTION_SUMMARIZE_SCENE {
        return Err(AppError::new(
            "ai_action_not_acceptible",
            "Dieser KI-Vorschlag kann hier nicht übernommen werden.",
        ));
    }

    if suggestion.status != AI_STATUS_DRAFT {
        return Err(AppError::new(
            "ai_suggestion_not_draft",
            "Nur offene KI-Vorschläge können übernommen werden.",
        ));
    }

    let now = timestamp();
    let transaction = connection.transaction()?;
    transaction.execute(
        "
        UPDATE scene
        SET summary = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        ",
        params![suggestion.result, now, suggestion.scene_id],
    )?;
    transaction.execute(
        "
        UPDATE ai_suggestion
        SET status = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        ",
        params![AI_STATUS_ACCEPTED, now, input.id],
    )?;
    transaction.commit()?;

    read_ai_suggestion(connection, &input.id)
}

pub(crate) fn update_ai_suggestion_status(
    connection: &Connection,
    suggestion_id: &str,
    status: &str,
) -> Result<AiSuggestionSnapshot, AppError> {
    let suggestion = read_ai_suggestion(connection, suggestion_id)?;
    if suggestion.status != AI_STATUS_DRAFT {
        return Err(AppError::new(
            "ai_suggestion_not_draft",
            "Nur offene KI-Vorschläge können verworfen werden.",
        ));
    }

    let now = timestamp();
    connection.execute(
        "
        UPDATE ai_suggestion
        SET status = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        ",
        params![status, now, suggestion_id],
    )?;

    read_ai_suggestion(connection, suggestion_id)
}
