use super::*;

#[tauri::command]
pub fn get_worldbuilding(state: State<AppState>) -> Result<WorldbuildingSnapshot, AppError> {
    with_active_connection(&state, |connection| read_worldbuilding(connection))
}

#[tauri::command]
pub fn create_world_system(
    input: CreateWorldSystemInput,
    state: State<AppState>,
) -> Result<WorldbuildingSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        create_world_system_in_connection(connection, input)?;
        read_worldbuilding(connection)
    })
}

#[tauri::command]
pub fn update_world_system(
    input: UpdateWorldSystemInput,
    state: State<AppState>,
) -> Result<WorldbuildingSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        update_world_system_in_connection(connection, input)?;
        read_worldbuilding(connection)
    })
}

#[tauri::command]
pub fn delete_world_system(
    input: DeleteWorldSystemInput,
    state: State<AppState>,
) -> Result<WorldbuildingSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        delete_world_system_in_connection(connection, input)?;
        read_worldbuilding(connection)
    })
}

pub(crate) fn read_world_systems(
    connection: &Connection,
) -> Result<Vec<WorldSystemSnapshot>, AppError> {
    let mut statement = connection.prepare("SELECT id, project_id, name, category, template_id, principle, rules, capabilities, costs, limits, access, distribution, world_impact, open_questions, notes, created_at, updated_at, deleted_at FROM world_system WHERE deleted_at IS NULL ORDER BY name COLLATE NOCASE, created_at")?;
    let rows = statement
        .query_map([], |row| {
            Ok(WorldSystemSnapshot {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                category: row.get(3)?,
                template_id: row.get(4)?,
                principle: row.get(5)?,
                rules: row.get(6)?,
                capabilities: row.get(7)?,
                costs: row.get(8)?,
                limits: row.get(9)?,
                access: row.get(10)?,
                distribution: row.get(11)?,
                world_impact: row.get(12)?,
                open_questions: row.get(13)?,
                notes: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
                deleted_at: row.get(17)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn world_system_templates(language: &str) -> Vec<WorldSystemTemplateSnapshot> {
    let german = [
        include_str!("../../resources/world-systems/magic.json"),
        include_str!("../../resources/world-systems/technology.json"),
        include_str!("../../resources/world-systems/psionics.json"),
        include_str!("../../resources/world-systems/hybrid.json"),
        include_str!("../../resources/world-systems/general.json"),
    ];
    let english = [
        include_str!("../../resources/world-systems/magic.en.json"),
        include_str!("../../resources/world-systems/technology.en.json"),
        include_str!("../../resources/world-systems/psionics.en.json"),
        include_str!("../../resources/world-systems/hybrid.en.json"),
        include_str!("../../resources/world-systems/general.en.json"),
    ];
    parse_world_system_templates(if language == "en" { &english } else { &german }, language)
}

pub(crate) fn parse_world_system_templates(
    json_templates: &[&str],
    language: &str,
) -> Vec<WorldSystemTemplateSnapshot> {
    let mut templates = json_templates
        .iter()
        .filter_map(|json| serde_json::from_str::<WorldSystemTemplateSnapshot>(json).ok())
        .collect::<Vec<_>>();
    if !templates.iter().any(|template| template.id == "general") {
        templates.push(WorldSystemTemplateSnapshot {
            id: "general".to_string(),
            name: if language == "en" {
                "General world system"
            } else {
                "Allgemeines Weltsystem"
            }
            .to_string(),
            category: "other".to_string(),
            guiding_questions: if language == "en" {
                vec![
                    "What is the core principle?".to_string(),
                    "What does the system make possible?".to_string(),
                    "What costs and limits apply?".to_string(),
                ]
            } else {
                vec![
                    "Was ist das Grundprinzip?".to_string(),
                    "Was ermöglicht das System?".to_string(),
                    "Welche Kosten und Grenzen gelten?".to_string(),
                    "Wer hat Zugang und wie verbreitet ist es?".to_string(),
                    "Welche Auswirkungen hat es auf die Welt?".to_string(),
                ]
            },
        });
    }
    templates
}

pub(crate) fn read_worldbuilding(
    connection: &Connection,
) -> Result<WorldbuildingSnapshot, AppError> {
    let language = read_active_project_language(connection)?;
    Ok(WorldbuildingSnapshot {
        systems: read_world_systems(connection)?,
        templates: world_system_templates(&language),
    })
}

pub(crate) fn validate_world_system_category(category: &str) -> Result<(), AppError> {
    if matches!(
        category,
        "magic" | "technology" | "psionics" | "biotech" | "supernatural" | "hybrid" | "other"
    ) {
        Ok(())
    } else {
        Err(AppError::new(
            "world_system_category_invalid",
            "Diese Weltsystem-Kategorie wird nicht unterstützt.",
        ))
    }
}

pub(crate) fn normalize_world_template_id(connection: &Connection, template_id: &str) -> String {
    let language = read_active_project_language(connection).unwrap_or_else(|_| "de".to_string());
    world_system_templates(&language)
        .into_iter()
        .find(|template| template.id == template_id)
        .map(|template| template.id)
        .unwrap_or_else(|| "general".to_string())
}

pub(crate) fn create_world_system_in_connection(
    connection: &mut Connection,
    input: CreateWorldSystemInput,
) -> Result<(), AppError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::new(
            "world_system_name_empty",
            "Der Name des Weltsystems darf nicht leer sein.",
        ));
    }
    validate_world_system_category(&input.category)?;
    let now = timestamp();
    connection.execute("INSERT INTO world_system (id, project_id, name, category, template_id, principle, rules, capabilities, costs, limits, access, distribution, world_impact, open_questions, notes, created_at, updated_at, deleted_at) VALUES (?1,?2,?3,?4,?5,'','','','','','','','','','',?6,?6,NULL)", params![Uuid::new_v4().to_string(), read_active_project_id(connection)?, name, input.category, normalize_world_template_id(connection, &input.template_id), now])?;
    Ok(())
}

pub(crate) fn update_world_system_in_connection(
    connection: &mut Connection,
    input: UpdateWorldSystemInput,
) -> Result<(), AppError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::new(
            "world_system_name_empty",
            "Der Name des Weltsystems darf nicht leer sein.",
        ));
    }
    validate_world_system_category(&input.category)?;
    let changed = connection.execute("UPDATE world_system SET name=?1, category=?2, template_id=?3, principle=?4, rules=?5, capabilities=?6, costs=?7, limits=?8, access=?9, distribution=?10, world_impact=?11, open_questions=?12, notes=?13, updated_at=?14 WHERE id=?15 AND deleted_at IS NULL", params![name, input.category, normalize_world_template_id(connection, &input.template_id), input.principle, input.rules, input.capabilities, input.costs, input.limits, input.access, input.distribution, input.world_impact, input.open_questions, input.notes, timestamp(), input.id])?;
    require_changed(
        changed,
        "world_system_missing",
        "Das Weltsystem wurde nicht gefunden.",
    )
}

pub(crate) fn delete_world_system_in_connection(
    connection: &mut Connection,
    input: DeleteWorldSystemInput,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    let now = timestamp();
    let changed = tx.execute(
        "UPDATE world_system SET deleted_at=?1, updated_at=?1 WHERE id=?2 AND deleted_at IS NULL",
        params![now, input.id],
    )?;
    require_changed(
        changed,
        "world_system_missing",
        "Das Weltsystem wurde nicht gefunden.",
    )?;
    tx.execute("UPDATE scene_link SET deleted_at=?1, updated_at=?1 WHERE target_type='world_system' AND target_id=?2 AND deleted_at IS NULL", params![now, input.id])?;
    tx.execute("UPDATE bible_relationship SET deleted_at=?1, updated_at=?1 WHERE deleted_at IS NULL AND ((source_type='world_system' AND source_id=?2) OR (target_type='world_system' AND target_id=?2))", params![now, input.id])?;
    tx.commit()?;
    Ok(())
}
