use crate::project::{read_active_project_language, with_active_connection, AppError, AppState};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

const TEMPLATE_RESOURCES: [&str; 6] = [
    include_str!("../resources/story-structures/three-act.json"),
    include_str!("../resources/story-structures/five-act.json"),
    include_str!("../resources/story-structures/hero-journey.json"),
    include_str!("../resources/story-structures/save-the-cat.json"),
    include_str!("../resources/story-structures/mystery.json"),
    include_str!("../resources/story-structures/freestyle.json"),
];
const TEMPLATE_RESOURCES_EN: [&str; 6] = [
    include_str!("../resources/story-structures/three-act.en.json"),
    include_str!("../resources/story-structures/five-act.en.json"),
    include_str!("../resources/story-structures/hero-journey.en.json"),
    include_str!("../resources/story-structures/save-the-cat.en.json"),
    include_str!("../resources/story-structures/mystery.en.json"),
    include_str!("../resources/story-structures/freestyle.en.json"),
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StructureBeat {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub notes: String,
    pub guiding_questions: Vec<String>,
    pub required: bool,
    pub suggested_percentage_start: Option<i64>,
    pub suggested_percentage_end: Option<i64>,
    pub order: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manuscript_chapter_id: Option<String>,
    #[serde(default)]
    pub scene_ids: Vec<String>,
    #[serde(default)]
    pub character_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StructureAct {
    pub id: String,
    pub title: String,
    pub description: String,
    pub order: i64,
    pub beats: Vec<StructureBeat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manuscript_area_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoryStructureTemplate {
    pub format_version: i64,
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub planning_methods: Vec<PlanningMethodInfo>,
    pub acts: Vec<StructureAct>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanningMethodInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStructure {
    pub id: String,
    pub novel_id: String,
    pub template_id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub acts: Vec<StructureAct>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyStoryStructureTemplateInput {
    pub template_id: String,
    pub replace_existing: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStructureBeatInput {
    pub id: String,
    pub title: String,
    pub description: String,
    pub summary: String,
    pub notes: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceStructureBeatLinksInput {
    pub beat_id: String,
    pub target_ids: Vec<String>,
}

fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn validate_template(template: &StoryStructureTemplate) -> Result<(), AppError> {
    const CATEGORIES: [&str; 5] = [
        "plot",
        "genre",
        "character_arc",
        "planning_method",
        "custom",
    ];
    if template.format_version != 1
        || template.id.trim().is_empty()
        || template.name.trim().is_empty()
    {
        return Err(AppError::new(
            "invalid_structure_template",
            "Eine Strukturvorlage hat ungültige Metadaten.",
        ));
    }
    if !CATEGORIES.contains(&template.category.as_str()) || template.acts.is_empty() {
        return Err(AppError::new(
            "invalid_structure_template",
            format!("Die Strukturvorlage '{}' ist ungültig.", template.name),
        ));
    }
    if template.planning_methods.iter().any(|method| {
        method.id.trim().is_empty()
            || method.name.trim().is_empty()
            || method.description.trim().is_empty()
            || method.steps.is_empty()
            || method.steps.iter().any(|step| step.trim().is_empty())
    }) {
        return Err(AppError::new(
            "invalid_structure_template",
            format!(
                "Die Arbeitsmethoden der Vorlage '{}' sind ungültig.",
                template.name
            ),
        ));
    }

    let mut act_ids = std::collections::HashSet::new();
    let mut beat_ids = std::collections::HashSet::new();
    for act in &template.acts {
        if act.id.trim().is_empty()
            || act.title.trim().is_empty()
            || !act_ids.insert(&act.id)
            || act.beats.is_empty()
        {
            return Err(AppError::new(
                "invalid_structure_template",
                format!("Die Akte der Vorlage '{}' sind ungültig.", template.name),
            ));
        }
        for beat in &act.beats {
            let valid_range = beat
                .suggested_percentage_start
                .map(|value| (0..=100).contains(&value))
                .unwrap_or(true)
                && beat
                    .suggested_percentage_end
                    .map(|value| (0..=100).contains(&value))
                    .unwrap_or(true)
                && match (
                    beat.suggested_percentage_start,
                    beat.suggested_percentage_end,
                ) {
                    (Some(start), Some(end)) => start <= end,
                    _ => true,
                };
            if beat.id.trim().is_empty()
                || beat.title.trim().is_empty()
                || !beat_ids.insert(&beat.id)
                || !valid_range
            {
                return Err(AppError::new(
                    "invalid_structure_template",
                    format!("Die Beats der Vorlage '{}' sind ungültig.", template.name),
                ));
            }
        }
    }
    Ok(())
}

fn parse_templates(resources: &[&str]) -> Result<Vec<StoryStructureTemplate>, AppError> {
    let mut templates = resources
        .iter()
        .filter_map(|resource| serde_json::from_str::<StoryStructureTemplate>(resource).ok())
        .filter(|template| validate_template(template).is_ok())
        .collect::<Vec<_>>();
    if !templates.iter().any(|template| template.id == "freestyle") {
        let english = resources
            .iter()
            .any(|resource| resource.contains("\"name\": \"Free-form structure\""));
        let fallback: StoryStructureTemplate = serde_json::from_str(if english {
            TEMPLATE_RESOURCES_EN[5]
        } else {
            TEMPLATE_RESOURCES[5]
        })
        .map_err(|error| {
            AppError::new(
                "invalid_structure_template",
                format!("Die Fallback-Strukturvorlage konnte nicht gelesen werden: {error}"),
            )
        })?;
        templates.push(fallback);
    }
    Ok(templates)
}

fn templates() -> Result<Vec<StoryStructureTemplate>, AppError> {
    parse_templates(&TEMPLATE_RESOURCES)
}

fn templates_for_language(language: &str) -> Result<Vec<StoryStructureTemplate>, AppError> {
    parse_templates(if language == "en" {
        &TEMPLATE_RESOURCES_EN
    } else {
        &TEMPLATE_RESOURCES
    })
}

pub(crate) fn template_display_name(template_id: &str) -> Result<Option<String>, AppError> {
    Ok(templates()?
        .into_iter()
        .find(|template| template.id == template_id)
        .map(|template| template.name))
}

pub(crate) fn apply_initial_template(
    connection: &mut Connection,
    template_id: &str,
) -> Result<ProjectStructure, AppError> {
    let language = read_active_project_language(connection)?;
    let template = templates_for_language(&language)?
        .into_iter()
        .find(|template| template.id == template_id)
        .ok_or_else(|| {
            AppError::new(
                "unknown_structure_template",
                "Die ausgewählte Strukturvorlage existiert nicht.",
            )
        })?;
    apply_template(connection, &template, false)
}

#[tauri::command]
pub fn list_story_structure_templates(
    state: State<AppState>,
) -> Result<Vec<StoryStructureTemplate>, AppError> {
    with_active_connection(&state, |connection| {
        templates_for_language(&read_active_project_language(connection)?)
    })
}

#[tauri::command]
pub fn get_project_structure(state: State<AppState>) -> Result<Option<ProjectStructure>, AppError> {
    with_active_connection(&state, read_project_structure)
}

#[tauri::command]
pub fn apply_story_structure_template(
    input: ApplyStoryStructureTemplateInput,
    state: State<AppState>,
) -> Result<ProjectStructure, AppError> {
    with_active_connection(&state, |connection| {
        let template = templates_for_language(&read_active_project_language(connection)?)?
            .into_iter()
            .find(|template| template.id == input.template_id)
            .ok_or_else(|| {
                AppError::new(
                    "unknown_structure_template",
                    "Die ausgewählte Strukturvorlage existiert nicht.",
                )
            })?;
        apply_template(connection, &template, input.replace_existing)
    })
}

#[tauri::command]
pub fn generate_story_structure_manuscript(
    state: State<AppState>,
) -> Result<ProjectStructure, AppError> {
    with_active_connection(&state, generate_manuscript_in_connection)
}

#[tauri::command]
pub fn update_structure_beat(
    input: UpdateStructureBeatInput,
    state: State<AppState>,
) -> Result<ProjectStructure, AppError> {
    with_active_connection(&state, |connection| {
        update_structure_beat_in_connection(connection, input)?;
        read_project_structure(connection)?.ok_or_else(|| {
            AppError::new(
                "missing_project_structure",
                "Der Roman besitzt noch keine aktive Struktur.",
            )
        })
    })
}

#[tauri::command]
pub fn replace_structure_beat_scene_links(
    input: ReplaceStructureBeatLinksInput,
    state: State<AppState>,
) -> Result<ProjectStructure, AppError> {
    with_active_connection(&state, |connection| {
        replace_structure_beat_links(connection, input, "scene")?;
        read_project_structure(connection)?.ok_or_else(|| {
            AppError::new(
                "missing_project_structure",
                "Der Roman besitzt noch keine aktive Struktur.",
            )
        })
    })
}

#[tauri::command]
pub fn replace_structure_beat_character_links(
    input: ReplaceStructureBeatLinksInput,
    state: State<AppState>,
) -> Result<ProjectStructure, AppError> {
    with_active_connection(&state, |connection| {
        replace_structure_beat_links(connection, input, "character")?;
        read_project_structure(connection)?.ok_or_else(|| {
            AppError::new(
                "missing_project_structure",
                "Der Roman besitzt noch keine aktive Struktur.",
            )
        })
    })
}

fn generate_manuscript_in_connection(
    connection: &mut Connection,
) -> Result<ProjectStructure, AppError> {
    let structure = read_project_structure(connection)?.ok_or_else(|| {
        AppError::new(
            "missing_project_structure",
            "Der Roman besitzt noch keine aktive Struktur.",
        )
    })?;
    if structure.acts.iter().all(|act| {
        act.manuscript_area_id.is_some()
            && act
                .beats
                .iter()
                .all(|beat| beat.manuscript_chapter_id.is_some())
    }) {
        return Ok(structure);
    }
    let transaction = connection.transaction()?;
    generate_existing_structure_scaffold(&transaction, &structure, &timestamp())?;
    transaction.commit()?;
    read_project_structure(connection)?.ok_or_else(|| {
        AppError::new(
            "storage_error",
            "Die Gliederung konnte nicht gespeichert werden.",
        )
    })
}

pub(crate) fn create_story_structure_tables(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS project_structure (
            id TEXT PRIMARY KEY,
            novel_id TEXT NOT NULL,
            template_id TEXT NOT NULL,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            category TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (novel_id) REFERENCES novel(id)
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_project_structure_active_novel
        ON project_structure(novel_id) WHERE deleted_at IS NULL;

        CREATE TABLE IF NOT EXISTS project_structure_act (
            id TEXT PRIMARY KEY,
            structure_id TEXT NOT NULL,
            template_act_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            manuscript_area_id TEXT,
            sort_order INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (structure_id) REFERENCES project_structure(id)
        );

        CREATE TABLE IF NOT EXISTS project_structure_beat (
            id TEXT PRIMARY KEY,
            act_id TEXT NOT NULL,
            template_beat_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            summary TEXT NOT NULL DEFAULT '',
            notes TEXT NOT NULL DEFAULT '',
            manuscript_chapter_id TEXT,
            guiding_questions_json TEXT NOT NULL,
            required INTEGER NOT NULL,
            suggested_percentage_start INTEGER,
            suggested_percentage_end INTEGER,
            status TEXT NOT NULL,
            sort_order INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (act_id) REFERENCES project_structure_act(id)
        );
        CREATE TABLE IF NOT EXISTS structure_scene_link (
            id TEXT PRIMARY KEY, beat_id TEXT NOT NULL, scene_id TEXT NOT NULL,
            sort_order INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT,
            FOREIGN KEY (beat_id) REFERENCES project_structure_beat(id), FOREIGN KEY (scene_id) REFERENCES scene(id)
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_structure_scene_link_active ON structure_scene_link(beat_id, scene_id) WHERE deleted_at IS NULL;
        CREATE TABLE IF NOT EXISTS structure_character_link (
            id TEXT PRIMARY KEY, beat_id TEXT NOT NULL, character_id TEXT NOT NULL,
            sort_order INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT,
            FOREIGN KEY (beat_id) REFERENCES project_structure_beat(id), FOREIGN KEY (character_id) REFERENCES character(id)
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_structure_character_link_active ON structure_character_link(beat_id, character_id) WHERE deleted_at IS NULL;
        ",
    )?;
    for (column, definition) in [
        ("summary", "TEXT NOT NULL DEFAULT ''"),
        ("notes", "TEXT NOT NULL DEFAULT ''"),
    ] {
        let mut statement = connection.prepare("PRAGMA table_info(project_structure_beat)")?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        if !columns.iter().any(|existing| existing == column) {
            connection.execute(
                &format!("ALTER TABLE project_structure_beat ADD COLUMN {column} {definition}"),
                [],
            )?;
        }
    }
    Ok(())
}

pub(crate) fn migrate_story_structure_v8_to_v9(connection: &Connection) -> Result<(), AppError> {
    let mut act_columns = connection.prepare("PRAGMA table_info(project_structure_act)")?;
    let act_columns = act_columns
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if !act_columns
        .iter()
        .any(|column| column == "manuscript_area_id")
    {
        connection.execute(
            "ALTER TABLE project_structure_act ADD COLUMN manuscript_area_id TEXT",
            [],
        )?;
    }

    let mut beat_columns = connection.prepare("PRAGMA table_info(project_structure_beat)")?;
    let beat_columns = beat_columns
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if !beat_columns
        .iter()
        .any(|column| column == "manuscript_chapter_id")
    {
        connection.execute(
            "ALTER TABLE project_structure_beat ADD COLUMN manuscript_chapter_id TEXT",
            [],
        )?;
    }
    Ok(())
}

fn active_novel_id(connection: &Connection) -> Result<String, AppError> {
    connection
        .query_row(
            "SELECT id FROM novel WHERE deleted_at IS NULL ORDER BY sort_order, created_at LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::new("missing_novel", "Das Projekt enthält keinen aktiven Roman."))
}

fn apply_template(
    connection: &mut Connection,
    template: &StoryStructureTemplate,
    replace_existing: bool,
) -> Result<ProjectStructure, AppError> {
    let novel_id = active_novel_id(connection)?;
    let existing = read_project_structure(connection)?;
    if existing.is_some() && !replace_existing {
        return Err(AppError::new(
            "structure_already_exists",
            "Der Roman besitzt bereits eine Struktur.",
        ));
    }

    let transaction = connection.transaction()?;
    let now = timestamp();
    if let Some(existing) = existing {
        cleanup_generated_scaffold(&transaction, &existing, &now)?;
        transaction.execute("UPDATE structure_scene_link SET deleted_at=?1, updated_at=?1 WHERE deleted_at IS NULL AND beat_id IN (SELECT project_structure_beat.id FROM project_structure_beat INNER JOIN project_structure_act ON project_structure_act.id=project_structure_beat.act_id WHERE project_structure_act.structure_id=?2)", params![now, existing.id])?;
        transaction.execute("UPDATE structure_character_link SET deleted_at=?1, updated_at=?1 WHERE deleted_at IS NULL AND beat_id IN (SELECT project_structure_beat.id FROM project_structure_beat INNER JOIN project_structure_act ON project_structure_act.id=project_structure_beat.act_id WHERE project_structure_act.structure_id=?2)", params![now, existing.id])?;
        transaction.execute("UPDATE project_structure_beat SET deleted_at = ?1, updated_at = ?1 WHERE act_id IN (SELECT id FROM project_structure_act WHERE structure_id = ?2 AND deleted_at IS NULL) AND deleted_at IS NULL", params![now, existing.id])?;
        transaction.execute("UPDATE project_structure_act SET deleted_at = ?1, updated_at = ?1 WHERE structure_id = ?2 AND deleted_at IS NULL", params![now, existing.id])?;
        transaction.execute("UPDATE project_structure SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL", params![now, existing.id])?;
    }

    let structure_id = Uuid::new_v4().to_string();
    transaction.execute("INSERT INTO project_structure (id, novel_id, template_id, name, description, category, created_at, updated_at, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, NULL)", params![structure_id, novel_id, template.id, template.name, template.description, template.category, now])?;
    let placeholder = pristine_placeholder(&transaction, &novel_id)?;
    let base_area_order: i64 = transaction.query_row("SELECT COALESCE(MAX(sort_order), 0) FROM manuscript_area WHERE novel_id = ?1 AND deleted_at IS NULL", params![novel_id], |row| row.get(0))?;
    for (act_index, act) in template.acts.iter().enumerate() {
        let act_id = Uuid::new_v4().to_string();
        let area_id = if act_index == 0 {
            if let Some((area_id, _, _)) = &placeholder {
                transaction.execute("UPDATE manuscript_area SET title = ?1, summary = ?2, sort_order = 1, updated_at = ?3 WHERE id = ?4", params![act.title, act.description, now, area_id])?;
                area_id.clone()
            } else {
                insert_area(
                    &transaction,
                    &novel_id,
                    act,
                    base_area_order + act_index as i64 + 1,
                    &now,
                )?
            }
        } else {
            insert_area(
                &transaction,
                &novel_id,
                act,
                base_area_order + act_index as i64 + 1,
                &now,
            )?
        };
        transaction.execute("INSERT INTO project_structure_act (id, structure_id, template_act_id, title, description, manuscript_area_id, sort_order, created_at, updated_at, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, NULL)", params![act_id, structure_id, act.id, act.title, act.description, area_id, act.order, now])?;
        for (beat_index, beat) in act.beats.iter().enumerate() {
            let chapter_id = if act_index == 0 && beat_index == 0 {
                if let Some((_, chapter_id, _)) = &placeholder {
                    transaction.execute("UPDATE chapter SET title = ?1, summary = ?2, sort_order = 1, updated_at = ?3 WHERE id = ?4", params![beat.title, beat.description, now, chapter_id])?;
                    chapter_id.clone()
                } else {
                    insert_chapter(
                        &transaction,
                        &novel_id,
                        &area_id,
                        beat,
                        beat_index as i64 + 1,
                        &now,
                    )?
                }
            } else {
                insert_chapter(
                    &transaction,
                    &novel_id,
                    &area_id,
                    beat,
                    beat_index as i64 + 1,
                    &now,
                )?
            };
            transaction.execute("INSERT INTO project_structure_beat (id, act_id, template_beat_id, title, description, summary, notes, manuscript_chapter_id, guiding_questions_json, required, suggested_percentage_start, suggested_percentage_end, status, sort_order, created_at, updated_at, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, '', '', ?6, ?7, ?8, ?9, ?10, 'empty', ?11, ?12, ?12, NULL)", params![Uuid::new_v4().to_string(), act_id, beat.id, beat.title, beat.description, chapter_id, serde_json::to_string(&beat.guiding_questions)?, beat.required, beat.suggested_percentage_start, beat.suggested_percentage_end, beat.order, now])?;
        }
    }
    transaction.commit()?;
    read_project_structure(connection)?.ok_or_else(|| {
        AppError::new(
            "storage_error",
            "Die Struktur konnte nicht gespeichert werden.",
        )
    })
}

fn insert_area(
    transaction: &Transaction<'_>,
    novel_id: &str,
    act: &StructureAct,
    order: i64,
    now: &str,
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    transaction.execute("INSERT INTO manuscript_area (id, novel_id, title, summary, sort_order, created_at, updated_at, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, NULL)", params![id, novel_id, act.title, act.description, order, now])?;
    Ok(id)
}

fn insert_chapter(
    transaction: &Transaction<'_>,
    novel_id: &str,
    area_id: &str,
    beat: &StructureBeat,
    order: i64,
    now: &str,
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    transaction.execute("INSERT INTO chapter (id, novel_id, area_id, title, summary, sort_order, created_at, updated_at, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, NULL)", params![id, novel_id, area_id, beat.title, beat.description, order, now])?;
    Ok(id)
}

fn pristine_placeholder(
    transaction: &Transaction<'_>,
    novel_id: &str,
) -> Result<Option<(String, String, String)>, AppError> {
    let counts: (i64, i64, i64) = transaction.query_row(
        "SELECT (SELECT COUNT(*) FROM manuscript_area WHERE novel_id = ?1 AND deleted_at IS NULL), (SELECT COUNT(*) FROM chapter WHERE novel_id = ?1 AND deleted_at IS NULL), (SELECT COUNT(*) FROM scene WHERE deleted_at IS NULL AND chapter_id IN (SELECT id FROM chapter WHERE novel_id = ?1 AND deleted_at IS NULL))",
        params![novel_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
    if counts != (1, 1, 1) {
        return Ok(None);
    }
    let candidate = transaction.query_row(
        "SELECT a.id, c.id, s.id FROM manuscript_area a JOIN chapter c ON c.area_id = a.id AND c.deleted_at IS NULL JOIN scene s ON s.chapter_id = c.id AND s.deleted_at IS NULL WHERE a.novel_id = ?1 AND a.deleted_at IS NULL AND a.title IN ('Bereich 1','Part 1') AND a.summary = '' AND c.title IN ('Kapitel 1','Chapter 1') AND c.summary = '' AND s.title IN ('Neue Szene','New scene') AND s.text = '' AND s.summary = '' AND s.status = 'draft' AND NOT EXISTS (SELECT 1 FROM scene_link sl WHERE sl.scene_id = s.id AND sl.deleted_at IS NULL) AND NOT EXISTS (SELECT 1 FROM ai_suggestion ai WHERE ai.scene_id = s.id AND ai.deleted_at IS NULL)",
        params![novel_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .optional()?;
    Ok(candidate)
}

fn generated_chapter_state(
    transaction: &Transaction<'_>,
    beat: &StructureBeat,
) -> Result<(bool, Option<String>), AppError> {
    let Some(chapter_id) = &beat.manuscript_chapter_id else {
        return Ok((false, None));
    };
    let metadata: Option<(String, String)> = transaction
        .query_row(
            "SELECT title, summary FROM chapter WHERE id = ?1 AND deleted_at IS NULL",
            params![chapter_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if metadata.as_ref() != Some(&(beat.title.clone(), beat.description.clone())) {
        return Ok((false, None));
    }
    let scenes: Vec<(String, String, String, String, String)> = {
        let mut statement = transaction.prepare("SELECT id, title, text, summary, status FROM scene WHERE chapter_id = ?1 AND deleted_at IS NULL ORDER BY sort_order")?;
        let rows = statement
            .query_map(params![chapter_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    if scenes.is_empty() {
        return Ok((true, None));
    }
    if scenes.len() != 1 {
        return Ok((false, None));
    }
    let scene = &scenes[0];
    if !matches!(scene.1.as_str(), "Neue Szene" | "New scene")
        || !scene.2.is_empty()
        || !scene.3.is_empty()
        || scene.4 != "draft"
    {
        return Ok((false, None));
    }
    let references: i64 = transaction.query_row("SELECT (SELECT COUNT(*) FROM scene_link WHERE scene_id = ?1 AND deleted_at IS NULL) + (SELECT COUNT(*) FROM ai_suggestion WHERE scene_id = ?1 AND deleted_at IS NULL)", params![scene.0], |row| row.get(0))?;
    Ok((references == 0, (references == 0).then(|| scene.0.clone())))
}

fn cleanup_generated_scaffold(
    transaction: &Transaction<'_>,
    structure: &ProjectStructure,
    now: &str,
) -> Result<(), AppError> {
    let mut all_safe = true;
    let mut default_scene: Option<(String, String, String)> = None;
    for act in &structure.acts {
        let area_safe = match &act.manuscript_area_id {
            Some(area_id) => transaction.query_row("SELECT title = ?1 AND summary = ?2 FROM manuscript_area WHERE id = ?3 AND deleted_at IS NULL", params![act.title, act.description, area_id], |row| row.get::<_, bool>(0)).optional()?.unwrap_or(false),
            None => false,
        };
        all_safe &= area_safe;
        for beat in &act.beats {
            let (safe, scene_id) = generated_chapter_state(transaction, beat)?;
            all_safe &= safe;
            if let (Some(area_id), Some(chapter_id), Some(scene_id)) = (
                &act.manuscript_area_id,
                &beat.manuscript_chapter_id,
                scene_id,
            ) {
                default_scene = Some((area_id.clone(), chapter_id.clone(), scene_id));
            }
        }
    }
    let mapped_area_count = structure
        .acts
        .iter()
        .filter(|act| act.manuscript_area_id.is_some())
        .count() as i64;
    let active_area_count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM manuscript_area WHERE novel_id = ?1 AND deleted_at IS NULL",
        params![structure.novel_id],
        |row| row.get(0),
    )?;

    if all_safe && mapped_area_count == active_area_count {
        for act in &structure.acts {
            for beat in &act.beats {
                if let Some(chapter_id) = &beat.manuscript_chapter_id {
                    if default_scene.as_ref().map(|item| &item.1) != Some(chapter_id) {
                        transaction.execute("UPDATE scene SET deleted_at = ?1, updated_at = ?1 WHERE chapter_id = ?2 AND deleted_at IS NULL", params![now, chapter_id])?;
                        transaction.execute("UPDATE chapter SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL", params![now, chapter_id])?;
                    }
                }
            }
            if let Some(area_id) = &act.manuscript_area_id {
                if default_scene.as_ref().map(|item| &item.0) != Some(area_id) {
                    transaction.execute("UPDATE manuscript_area SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL", params![now, area_id])?;
                }
            }
        }
        if let Some((area_id, chapter_id, _)) = default_scene {
            transaction.execute("UPDATE manuscript_area SET title = 'Bereich 1', summary = '', sort_order = 1, updated_at = ?1 WHERE id = ?2", params![now, area_id])?;
            transaction.execute("UPDATE chapter SET title = 'Kapitel 1', summary = '', sort_order = 1, updated_at = ?1 WHERE id = ?2", params![now, chapter_id])?;
        }
        return Ok(());
    }

    for act in &structure.acts {
        for beat in &act.beats {
            let (safe, default_scene) = generated_chapter_state(transaction, beat)?;
            if safe && default_scene.is_none() {
                if let Some(chapter_id) = &beat.manuscript_chapter_id {
                    transaction.execute("UPDATE chapter SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL", params![now, chapter_id])?;
                }
            }
        }
        if let Some(area_id) = &act.manuscript_area_id {
            let chapter_count: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM chapter WHERE area_id = ?1 AND deleted_at IS NULL",
                params![area_id],
                |row| row.get(0),
            )?;
            if chapter_count == 0 {
                transaction.execute("UPDATE manuscript_area SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL", params![now, area_id])?;
            }
        }
    }
    Ok(())
}

fn generate_existing_structure_scaffold(
    transaction: &Transaction<'_>,
    structure: &ProjectStructure,
    now: &str,
) -> Result<(), AppError> {
    let placeholder = pristine_placeholder(transaction, &structure.novel_id)?;
    let base_area_order: i64 = transaction.query_row("SELECT COALESCE(MAX(sort_order), 0) FROM manuscript_area WHERE novel_id = ?1 AND deleted_at IS NULL", params![structure.novel_id], |row| row.get(0))?;
    for (act_index, act) in structure.acts.iter().enumerate() {
        let area_id = if let Some(id) = &act.manuscript_area_id {
            id.clone()
        } else if let (0, Some((placeholder_area_id, _, _))) = (act_index, placeholder.as_ref()) {
            let id = placeholder_area_id.clone();
            transaction.execute("UPDATE manuscript_area SET title = ?1, summary = ?2, updated_at = ?3 WHERE id = ?4", params![act.title, act.description, now, id])?;
            id
        } else {
            insert_area(
                transaction,
                &structure.novel_id,
                act,
                base_area_order + act_index as i64 + 1,
                now,
            )?
        };
        transaction.execute("UPDATE project_structure_act SET manuscript_area_id = ?1, updated_at = ?2 WHERE structure_id = ?3 AND template_act_id = ?4 AND deleted_at IS NULL", params![area_id, now, structure.id, act.id])?;
        for (beat_index, beat) in act.beats.iter().enumerate() {
            if beat.manuscript_chapter_id.is_some() {
                continue;
            }
            let chapter_id = if let (0, 0, Some((_, placeholder_chapter_id, _))) =
                (act_index, beat_index, placeholder.as_ref())
            {
                let id = placeholder_chapter_id.clone();
                transaction.execute(
                    "UPDATE chapter SET title = ?1, summary = ?2, updated_at = ?3 WHERE id = ?4",
                    params![beat.title, beat.description, now, id],
                )?;
                id
            } else {
                insert_chapter(
                    transaction,
                    &structure.novel_id,
                    &area_id,
                    beat,
                    beat_index as i64 + 1,
                    now,
                )?
            };
            transaction.execute("UPDATE project_structure_beat SET manuscript_chapter_id = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL", params![chapter_id, now, beat.id])?;
        }
    }
    Ok(())
}

fn read_project_structure(
    connection: &mut Connection,
) -> Result<Option<ProjectStructure>, AppError> {
    let novel_id = active_novel_id(connection)?;
    migrate_legacy_builtin_structure_texts(connection, &novel_id)?;
    let structure = connection.query_row(
        "SELECT id, template_id, name, description, category, created_at, updated_at FROM project_structure WHERE novel_id = ?1 AND deleted_at IS NULL LIMIT 1",
        params![novel_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?, row.get::<_, String>(6)?)),
    ).optional()?;
    let Some((id, template_id, name, description, category, created_at, updated_at)) = structure
    else {
        return Ok(None);
    };

    let mut act_statement = connection.prepare("SELECT id, template_act_id, title, description, sort_order, manuscript_area_id FROM project_structure_act WHERE structure_id = ?1 AND deleted_at IS NULL ORDER BY sort_order, created_at")?;
    let act_rows = act_statement
        .query_map(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut acts = Vec::new();
    for (act_id, template_act_id, title, description, order, manuscript_area_id) in act_rows {
        let mut beat_statement = connection.prepare("SELECT id, title, description, summary, notes, manuscript_chapter_id, guiding_questions_json, required, suggested_percentage_start, suggested_percentage_end, status, sort_order FROM project_structure_beat WHERE act_id = ?1 AND deleted_at IS NULL ORDER BY sort_order, created_at")?;
        let beats = beat_statement
            .query_map(params![act_id], |row| {
                let beat_id: String = row.get(0)?;
                let questions: String = row.get(6)?;
                let scene_ids =
                    read_link_ids(connection, "structure_scene_link", "scene_id", &beat_id)
                        .unwrap_or_default();
                let character_ids = read_link_ids(
                    connection,
                    "structure_character_link",
                    "character_id",
                    &beat_id,
                )
                .unwrap_or_default();
                Ok(StructureBeat {
                    id: beat_id,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    summary: row.get(3)?,
                    notes: row.get(4)?,
                    guiding_questions: serde_json::from_str(&questions).unwrap_or_default(),
                    manuscript_chapter_id: row.get(5)?,
                    required: row.get(7)?,
                    suggested_percentage_start: row.get(8)?,
                    suggested_percentage_end: row.get(9)?,
                    status: Some(row.get(10)?),
                    order: row.get(11)?,
                    scene_ids,
                    character_ids,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        acts.push(StructureAct {
            id: template_act_id,
            title,
            description,
            order,
            beats,
            manuscript_area_id,
        });
    }
    Ok(Some(ProjectStructure {
        id,
        novel_id,
        template_id,
        name,
        description,
        category,
        acts,
        created_at,
        updated_at,
    }))
}

fn read_link_ids(
    connection: &Connection,
    table: &str,
    target_column: &str,
    beat_id: &str,
) -> Result<Vec<String>, AppError> {
    let sql = format!("SELECT {target_column} FROM {table} WHERE beat_id=?1 AND deleted_at IS NULL ORDER BY sort_order, created_at");
    let mut statement = connection.prepare(&sql)?;
    let ids = statement
        .query_map(params![beat_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn ensure_active_beat(connection: &Connection, beat_id: &str) -> Result<(), AppError> {
    let exists = connection.query_row("SELECT project_structure_beat.id FROM project_structure_beat INNER JOIN project_structure_act ON project_structure_act.id=project_structure_beat.act_id INNER JOIN project_structure ON project_structure.id=project_structure_act.structure_id INNER JOIN novel ON novel.id=project_structure.novel_id WHERE project_structure_beat.id=?1 AND project_structure_beat.deleted_at IS NULL AND project_structure_act.deleted_at IS NULL AND project_structure.deleted_at IS NULL AND novel.deleted_at IS NULL", params![beat_id], |row| row.get::<_, String>(0)).optional()?;
    exists.map(|_| ()).ok_or_else(|| {
        AppError::new(
            "structure_beat_missing",
            "Der Struktur-Beat wurde nicht gefunden.",
        )
    })
}

fn update_structure_beat_in_connection(
    connection: &mut Connection,
    input: UpdateStructureBeatInput,
) -> Result<(), AppError> {
    ensure_active_beat(connection, &input.id)?;
    let title = input.title.trim();
    if title.is_empty() {
        return Err(AppError::new(
            "structure_beat_title_empty",
            "Der Beat-Titel darf nicht leer sein.",
        ));
    }
    if !matches!(
        input.status.as_str(),
        "empty" | "planned" | "in_progress" | "complete"
    ) {
        return Err(AppError::new(
            "structure_beat_status_invalid",
            "Der Beat-Status ist ungültig.",
        ));
    }
    connection.execute("UPDATE project_structure_beat SET title=?1, description=?2, summary=?3, notes=?4, status=?5, updated_at=?6 WHERE id=?7 AND deleted_at IS NULL", params![title, input.description.trim(), input.summary.trim(), input.notes.trim(), input.status, timestamp(), input.id])?;
    Ok(())
}

fn replace_structure_beat_links(
    connection: &mut Connection,
    input: ReplaceStructureBeatLinksInput,
    kind: &str,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    ensure_active_beat(&tx, &input.beat_id)?;
    let (table, column, target_table, error_code, error_message) = if kind == "scene" {
        (
            "structure_scene_link",
            "scene_id",
            "scene",
            "scene_missing",
            "Eine ausgewählte Szene wurde nicht gefunden.",
        )
    } else {
        (
            "structure_character_link",
            "character_id",
            "character",
            "project_bible_card_missing",
            "Eine ausgewählte Figur wurde nicht gefunden.",
        )
    };
    let mut ids = Vec::new();
    for id in input.target_ids {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    for id in &ids {
        let sql = format!("SELECT id FROM {target_table} WHERE id=?1 AND deleted_at IS NULL");
        if tx
            .query_row(&sql, params![id], |row| row.get::<_, String>(0))
            .optional()?
            .is_none()
        {
            return Err(AppError::new(error_code, error_message));
        }
    }
    let now = timestamp();
    tx.execute(&format!("UPDATE {table} SET deleted_at=?1, updated_at=?1 WHERE beat_id=?2 AND deleted_at IS NULL"), params![now, input.beat_id])?;
    for (index, id) in ids.iter().enumerate() {
        tx.execute(&format!("INSERT INTO {table} (id, beat_id, {column}, sort_order, created_at, updated_at, deleted_at) VALUES (?1,?2,?3,?4,?5,?5,NULL)"), params![Uuid::new_v4().to_string(), input.beat_id, id, index as i64, now])?;
    }
    tx.commit()?;
    Ok(())
}

fn legacy_ascii_text(value: &str) -> String {
    value
        .replace('Ä', "Ae")
        .replace('Ö', "Oe")
        .replace('Ü', "Ue")
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss")
}

fn migrate_legacy_builtin_structure_texts(
    connection: &mut Connection,
    novel_id: &str,
) -> Result<(), AppError> {
    let active: Option<(String, String)> = connection
        .query_row(
            "SELECT id, template_id FROM project_structure WHERE novel_id = ?1 AND deleted_at IS NULL LIMIT 1",
            params![novel_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((structure_id, template_id)) = active else {
        return Ok(());
    };
    let Some(template) = templates()?
        .into_iter()
        .find(|template| template.id == template_id)
    else {
        return Ok(());
    };

    let transaction = connection.transaction()?;
    transaction.execute(
        "UPDATE project_structure SET name = ?1, description = ?2 WHERE id = ?3 AND name = ?4 AND description = ?5",
        params![
            template.name,
            template.description,
            structure_id,
            legacy_ascii_text(&template.name),
            legacy_ascii_text(&template.description)
        ],
    )?;

    for act in &template.acts {
        let stored_act: Option<(String, Option<String>)> = transaction
            .query_row(
                "SELECT id, manuscript_area_id FROM project_structure_act WHERE structure_id = ?1 AND template_act_id = ?2 AND deleted_at IS NULL",
                params![structure_id, act.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((act_id, area_id)) = stored_act else {
            continue;
        };
        let legacy_act_title = legacy_ascii_text(&act.title);
        let legacy_act_description = legacy_ascii_text(&act.description);
        transaction.execute(
            "UPDATE project_structure_act SET title = ?1, description = ?2 WHERE id = ?3 AND title = ?4 AND description = ?5",
            params![act.title, act.description, act_id, legacy_act_title, legacy_act_description],
        )?;
        if let Some(area_id) = area_id {
            transaction.execute(
                "UPDATE manuscript_area SET title = ?1, summary = ?2 WHERE id = ?3 AND title = ?4 AND summary = ?5",
                params![act.title, act.description, area_id, legacy_act_title, legacy_act_description],
            )?;
        }

        for beat in &act.beats {
            let legacy_title = legacy_ascii_text(&beat.title);
            let legacy_description = legacy_ascii_text(&beat.description);
            let questions = serde_json::to_string(&beat.guiding_questions)?;
            let legacy_questions = serde_json::to_string(
                &beat
                    .guiding_questions
                    .iter()
                    .map(|question| legacy_ascii_text(question))
                    .collect::<Vec<_>>(),
            )?;
            let chapter_id: Option<String> = transaction
                .query_row(
                    "SELECT manuscript_chapter_id FROM project_structure_beat WHERE act_id = ?1 AND template_beat_id = ?2 AND deleted_at IS NULL",
                    params![act_id, beat.id],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();
            transaction.execute(
                "UPDATE project_structure_beat SET title = ?1, description = ?2, guiding_questions_json = ?3 WHERE act_id = ?4 AND template_beat_id = ?5 AND title = ?6 AND description = ?7 AND guiding_questions_json = ?8 AND deleted_at IS NULL",
                params![beat.title, beat.description, questions, act_id, beat.id, legacy_title, legacy_description, legacy_questions],
            )?;
            if let Some(chapter_id) = chapter_id {
                transaction.execute(
                    "UPDATE chapter SET title = ?1, summary = ?2 WHERE id = ?3 AND title = ?4 AND summary = ?5",
                    params![beat.title, beat.description, chapter_id, legacy_title, legacy_description],
                )?;
            }
        }
    }
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_connection(pristine: bool) -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "
                PRAGMA foreign_keys = ON;
                CREATE TABLE novel (
                    id TEXT PRIMARY KEY,
                    sort_order INTEGER NOT NULL,
                    created_at TEXT NOT NULL,
                    deleted_at TEXT
                );
                CREATE TABLE manuscript_area (id TEXT PRIMARY KEY, novel_id TEXT NOT NULL, title TEXT NOT NULL, summary TEXT NOT NULL, sort_order INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT);
                CREATE TABLE chapter (id TEXT PRIMARY KEY, novel_id TEXT NOT NULL, area_id TEXT NOT NULL, title TEXT NOT NULL, summary TEXT NOT NULL, sort_order INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT);
                CREATE TABLE scene (id TEXT PRIMARY KEY, chapter_id TEXT NOT NULL, title TEXT NOT NULL, text TEXT NOT NULL, status TEXT NOT NULL, summary TEXT NOT NULL, sort_order INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT);
                CREATE TABLE character (id TEXT PRIMARY KEY, name TEXT NOT NULL, deleted_at TEXT);
                CREATE TABLE scene_link (id TEXT PRIMARY KEY, scene_id TEXT NOT NULL, deleted_at TEXT);
                CREATE TABLE ai_suggestion (id TEXT PRIMARY KEY, scene_id TEXT NOT NULL, deleted_at TEXT);
                INSERT INTO novel (id, sort_order, created_at, deleted_at)
                VALUES ('novel-1', 0, '1', NULL);
                INSERT INTO character VALUES ('character-1','Mira',NULL), ('character-2','Tao',NULL);
                ",
            )
            .unwrap();
        if pristine {
            connection.execute_batch("INSERT INTO manuscript_area VALUES ('area-1','novel-1','Bereich 1','',1,'1','1',NULL); INSERT INTO chapter VALUES ('chapter-1','novel-1','area-1','Kapitel 1','',1,'1','1',NULL); INSERT INTO scene VALUES ('scene-1','chapter-1','Neue Szene','','draft','',1,'1','1',NULL);").unwrap();
        } else {
            connection.execute_batch("INSERT INTO manuscript_area VALUES ('area-1','novel-1','Bestehend','Notiz',1,'1','1',NULL); INSERT INTO chapter VALUES ('chapter-1','novel-1','area-1','Vorhandenes Kapitel','Inhalt',1,'1','1',NULL); INSERT INTO scene VALUES ('scene-1','chapter-1','Unverändert','Text','draft','Inhalt',1,'1','1',NULL);").unwrap();
        }
        create_story_structure_tables(&connection).unwrap();
        connection
    }

    #[test]
    fn all_built_in_templates_are_valid_and_complete() {
        let templates = templates().unwrap();
        assert_eq!(templates.len(), 6);
        let freestyle = templates
            .iter()
            .find(|template| template.id == "freestyle")
            .unwrap();
        assert_eq!(freestyle.acts.len(), 1);
        assert_eq!(freestyle.acts[0].beats.len(), 1);
        assert_eq!(freestyle.planning_methods.len(), 1);
        assert_eq!(freestyle.planning_methods[0].id, "snowflake");
        assert_eq!(freestyle.planning_methods[0].steps[2], "Figurenübersicht");
        assert_eq!(
            templates
                .iter()
                .find(|template| template.id == "five-act")
                .unwrap()
                .name,
            "Fünfaktstruktur"
        );
        assert!(!templates.iter().any(|template| template.id == "snowflake"));
        assert!(templates.iter().all(|template| !template.acts.is_empty()));
        assert!(templates
            .iter()
            .flat_map(|template| &template.acts)
            .flat_map(|act| &act.beats)
            .all(|beat| beat.guiding_questions.len() >= 2));
    }

    #[test]
    fn english_templates_match_german_stable_ids() {
        let german = templates_for_language("de").unwrap();
        let english = templates_for_language("en").unwrap();
        assert_eq!(german.len(), english.len());
        for german_template in german {
            let english_template = english
                .iter()
                .find(|item| item.id == german_template.id)
                .unwrap();
            assert_ne!(german_template.name, english_template.name);
            let german_act_ids = german_template
                .acts
                .iter()
                .map(|act| &act.id)
                .collect::<Vec<_>>();
            let english_act_ids = english_template
                .acts
                .iter()
                .map(|act| &act.id)
                .collect::<Vec<_>>();
            assert_eq!(german_act_ids, english_act_ids);
            let german_beat_ids = german_template
                .acts
                .iter()
                .flat_map(|act| act.beats.iter().map(|beat| &beat.id))
                .collect::<Vec<_>>();
            let english_beat_ids = english_template
                .acts
                .iter()
                .flat_map(|act| act.beats.iter().map(|beat| &beat.id))
                .collect::<Vec<_>>();
            assert_eq!(german_beat_ids, english_beat_ids);
        }
    }

    #[test]
    fn rejects_invalid_percentage_ranges() {
        let mut template = templates().unwrap().remove(0);
        template.acts[0].beats[0].suggested_percentage_start = Some(90);
        template.acts[0].beats[0].suggested_percentage_end = Some(10);
        assert_eq!(
            validate_template(&template).unwrap_err().code,
            "invalid_structure_template"
        );
    }

    #[test]
    fn upgrades_only_unchanged_legacy_builtin_texts() {
        let mut connection = test_connection(true);
        let template = templates()
            .unwrap()
            .into_iter()
            .find(|template| template.id == "five-act")
            .unwrap();
        let structure = apply_template(&mut connection, &template, false).unwrap();

        connection
            .execute(
                "UPDATE project_structure SET name = 'Fuenfaktstruktur', description = 'Feingliedrige Dramaturgie fuer komplexe Konflikte und Konsequenzen.' WHERE id = ?1",
                params![structure.id],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE project_structure_act SET title = 'Akt 1: Einfuehrung' WHERE structure_id = ?1 AND template_act_id = 'introduction'",
                params![structure.id],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE manuscript_area SET title = 'Akt 1: Einfuehrung' WHERE id = ?1",
                params![structure.acts[0].manuscript_area_id],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE project_structure_beat SET title = 'Ausloeser', description = 'Der zentrale Konflikt wird in Bewegung gesetzt.', guiding_questions_json = '[\"Was stoert die Ordnung?\",\"Wer muss darauf reagieren?\"]' WHERE template_beat_id = 'trigger'",
                [],
            )
            .unwrap();
        let trigger_chapter_id = structure.acts[0].beats[1]
            .manuscript_chapter_id
            .as_deref()
            .unwrap();
        connection
            .execute(
                "UPDATE chapter SET title = 'Ausloeser' WHERE id = ?1",
                params![trigger_chapter_id],
            )
            .unwrap();

        let migrated = read_project_structure(&mut connection).unwrap().unwrap();
        assert_eq!(migrated.name, "Fünfaktstruktur");
        assert_eq!(migrated.acts[0].title, "Akt 1: Einführung");
        assert_eq!(migrated.acts[0].beats[1].title, "Auslöser");
        assert_eq!(
            migrated.acts[0].beats[1].guiding_questions[0],
            "Was stört die Ordnung?"
        );
        let area_title: String = connection
            .query_row(
                "SELECT title FROM manuscript_area WHERE id = ?1",
                params![migrated.acts[0].manuscript_area_id],
                |row| row.get(0),
            )
            .unwrap();
        let chapter_title: String = connection
            .query_row(
                "SELECT title FROM chapter WHERE id = ?1",
                params![trigger_chapter_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(area_title, "Akt 1: Einführung");
        assert_eq!(chapter_title, "Auslöser");
    }

    #[test]
    fn applies_and_replaces_one_structure_without_touching_scenes() {
        let mut connection = test_connection(false);
        let templates = templates().unwrap();

        let first = apply_template(&mut connection, &templates[0], false).unwrap();
        assert_eq!(first.template_id, "three-act");
        assert!(first.acts.iter().all(|act| !act.beats.is_empty()));
        assert!(first
            .acts
            .iter()
            .flat_map(|act| &act.beats)
            .all(|beat| beat.status.as_deref() == Some("empty")));

        let error = apply_template(&mut connection, &templates[1], false).unwrap_err();
        assert_eq!(error.code, "structure_already_exists");

        let replacement = apply_template(&mut connection, &templates[1], true).unwrap();
        assert_eq!(replacement.template_id, "five-act");
        let active_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM project_structure WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let scene_title: String = connection
            .query_row("SELECT title FROM scene WHERE id = 'scene-1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(active_count, 1);
        assert_eq!(scene_title, "Unverändert");
    }

    #[test]
    fn transforms_pristine_placeholder_into_template_scaffold() {
        let mut connection = test_connection(true);
        let structure = apply_template(&mut connection, &templates().unwrap()[0], false).unwrap();
        let area_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM manuscript_area WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let chapter_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM chapter WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let scene_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM scene WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((area_count, chapter_count, scene_count), (3, 10, 1));
        assert_eq!(
            structure.acts[0].manuscript_area_id.as_deref(),
            Some("area-1")
        );
        assert_eq!(
            structure.acts[0].beats[0].manuscript_chapter_id.as_deref(),
            Some("chapter-1")
        );
    }

    #[test]
    fn appends_scaffold_to_existing_manuscript() {
        let mut connection = test_connection(false);
        apply_template(&mut connection, &templates().unwrap()[0], false).unwrap();
        let area_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM manuscript_area WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let chapter_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM chapter WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((area_count, chapter_count), (4, 11));
    }

    #[test]
    fn keeps_edited_content_when_replacing_scaffold() {
        let mut connection = test_connection(true);
        let templates = templates().unwrap();
        apply_template(&mut connection, &templates[0], false).unwrap();
        connection
            .execute(
                "UPDATE scene SET text = 'Bleibt erhalten' WHERE id = 'scene-1'",
                [],
            )
            .unwrap();
        apply_template(&mut connection, &templates[1], true).unwrap();
        let text: String = connection
            .query_row(
                "SELECT text FROM scene WHERE id = 'scene-1' AND deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(text, "Bleibt erhalten");
    }

    #[test]
    fn generates_missing_legacy_scaffold_idempotently() {
        let mut connection = test_connection(false);
        apply_template(&mut connection, &templates().unwrap()[0], false).unwrap();
        connection.execute_batch("DELETE FROM chapter WHERE area_id != 'area-1'; DELETE FROM manuscript_area WHERE id != 'area-1'; UPDATE project_structure_act SET manuscript_area_id = NULL; UPDATE project_structure_beat SET manuscript_chapter_id = NULL;").unwrap();

        generate_manuscript_in_connection(&mut connection).unwrap();
        generate_manuscript_in_connection(&mut connection).unwrap();
        let counts: (i64, i64) = connection.query_row("SELECT (SELECT COUNT(*) FROM manuscript_area WHERE deleted_at IS NULL), (SELECT COUNT(*) FROM chapter WHERE deleted_at IS NULL)", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(counts, (4, 11));
    }

    #[test]
    fn updates_beats_and_replaces_scene_and_character_links() {
        let mut connection = test_connection(true);
        let structure = apply_template(&mut connection, &templates().unwrap()[0], false).unwrap();
        let beat_id = structure.acts[0].beats[0].id.clone();
        update_structure_beat_in_connection(
            &mut connection,
            UpdateStructureBeatInput {
                id: beat_id.clone(),
                title: "Auftakt".into(),
                description: "Plan".into(),
                summary: "Kurz".into(),
                notes: "Intern".into(),
                status: "planned".into(),
            },
        )
        .unwrap();
        replace_structure_beat_links(
            &mut connection,
            ReplaceStructureBeatLinksInput {
                beat_id: beat_id.clone(),
                target_ids: vec!["scene-1".into(), "scene-1".into()],
            },
            "scene",
        )
        .unwrap();
        replace_structure_beat_links(
            &mut connection,
            ReplaceStructureBeatLinksInput {
                beat_id: beat_id.clone(),
                target_ids: vec!["character-2".into(), "character-1".into()],
            },
            "character",
        )
        .unwrap();
        let updated = read_project_structure(&mut connection).unwrap().unwrap();
        let beat = &updated.acts[0].beats[0];
        assert_eq!(
            (
                &beat.title,
                &beat.summary,
                &beat.notes,
                beat.status.as_deref()
            ),
            (
                &"Auftakt".to_string(),
                &"Kurz".to_string(),
                &"Intern".to_string(),
                Some("planned")
            )
        );
        assert_eq!(beat.scene_ids, vec!["scene-1"]);
        assert_eq!(beat.character_ids, vec!["character-2", "character-1"]);
        assert_eq!(
            update_structure_beat_in_connection(
                &mut connection,
                UpdateStructureBeatInput {
                    id: beat_id.clone(),
                    title: "".into(),
                    description: "".into(),
                    summary: "".into(),
                    notes: "".into(),
                    status: "empty".into()
                }
            )
            .unwrap_err()
            .code,
            "structure_beat_title_empty"
        );
        assert_eq!(
            update_structure_beat_in_connection(
                &mut connection,
                UpdateStructureBeatInput {
                    id: beat_id,
                    title: "X".into(),
                    description: "".into(),
                    summary: "".into(),
                    notes: "".into(),
                    status: "invalid".into()
                }
            )
            .unwrap_err()
            .code,
            "structure_beat_status_invalid"
        );
    }
}
