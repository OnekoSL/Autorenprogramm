use super::*;

#[tauri::command]
pub fn get_project_bible(state: State<AppState>) -> Result<ProjectBibleSnapshot, AppError> {
    with_active_connection(&state, |connection| read_project_bible(connection))
}

#[tauri::command]
pub fn create_project_bible_card(
    input: CreateProjectBibleCardInput,
    state: State<AppState>,
) -> Result<ProjectBibleSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        create_project_bible_card_in_connection(connection, input)?;
        read_project_bible(connection)
    })
}

#[tauri::command]
pub fn update_project_bible_card(
    input: UpdateProjectBibleCardInput,
    state: State<AppState>,
) -> Result<ProjectBibleSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        update_project_bible_card_in_connection(connection, input)?;
        read_project_bible(connection)
    })
}

#[tauri::command]
pub fn delete_project_bible_card(
    input: DeleteProjectBibleCardInput,
    state: State<AppState>,
) -> Result<ProjectBibleSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        delete_project_bible_card_in_connection(connection, input)?;
        read_project_bible(connection)
    })
}

#[tauri::command]
pub fn get_scene_links_for_card(
    input: ProjectBibleCardLinkInput,
    state: State<AppState>,
) -> Result<Vec<SceneLinkSnapshot>, AppError> {
    with_active_connection(&state, |connection| {
        validate_target_type(&input.target_type)?;
        ensure_active_project_bible_card(connection, &input.target_type, &input.target_id)?;
        read_scene_links_for_card(connection, &input.target_type, &input.target_id)
    })
}

#[tauri::command]
pub fn replace_scene_links_for_card(
    input: ReplaceSceneLinksForCardInput,
    state: State<AppState>,
) -> Result<ProjectBibleSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        replace_scene_links_for_card_in_connection(connection, input)?;
        read_project_bible(connection)
    })
}

#[tauri::command]
pub fn upsert_bible_relationship(
    input: UpsertBibleRelationshipInput,
    state: State<AppState>,
) -> Result<ProjectBibleSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        upsert_bible_relationship_in_connection(connection, input)?;
        read_project_bible(connection)
    })
}

#[tauri::command]
pub fn delete_bible_relationship(
    input: DeleteBibleRelationshipInput,
    state: State<AppState>,
) -> Result<ProjectBibleSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        let changed = connection.execute("UPDATE bible_relationship SET deleted_at=?1, updated_at=?1 WHERE id=?2 AND deleted_at IS NULL", params![timestamp(), input.id])?;
        require_changed(
            changed,
            "bible_relationship_missing",
            "Die Beziehung wurde nicht gefunden.",
        )?;
        read_project_bible(connection)
    })
}

#[tauri::command]
pub fn reorder_bible_events(
    input: ReorderBibleEventsInput,
    state: State<AppState>,
) -> Result<ProjectBibleSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        let tx = connection.transaction()?;
        let active = read_events(&tx)?;
        if input.event_ids.len() != active.len()
            || input
                .event_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != active.len()
            || input
                .event_ids
                .iter()
                .any(|id| !active.iter().any(|event| &event.id == id))
        {
            return Err(AppError::new(
                "invalid_event_order",
                "Die Ereignisreihenfolge ist unvollständig oder ungültig.",
            ));
        }
        let now = timestamp();
        for (index, id) in input.event_ids.iter().enumerate() {
            tx.execute(
                "UPDATE event SET sort_order=?1, updated_at=?2 WHERE id=?3 AND deleted_at IS NULL",
                params![index as i64 + 1, now, id],
            )?;
        }
        tx.commit()?;
        read_project_bible(connection)
    })
}

pub(crate) fn read_project_bible(
    connection: &Connection,
) -> Result<ProjectBibleSnapshot, AppError> {
    Ok(ProjectBibleSnapshot {
        characters: read_characters(connection)?,
        locations: read_locations(connection)?,
        items: read_items(connection)?,
        notes: read_notes(connection)?,
        factions: read_factions(connection)?,
        events: read_events(connection)?,
        scene_links: read_active_scene_links(connection)?,
        relationships: read_bible_relationships(connection)?,
    })
}

pub(crate) fn read_characters(connection: &Connection) -> Result<Vec<CharacterSnapshot>, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT id, project_id, name, role, description, notes, created_at, updated_at, deleted_at
        FROM character
        WHERE deleted_at IS NULL
        ORDER BY name COLLATE NOCASE ASC, created_at ASC
        ",
    )?;

    let characters = statement
        .query_map([], |row| {
            Ok(CharacterSnapshot {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                role: row.get(3)?,
                description: row.get(4)?,
                notes: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                deleted_at: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(characters)
}

pub(crate) fn read_locations(connection: &Connection) -> Result<Vec<LocationSnapshot>, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT id, project_id, name, description, notes, created_at, updated_at, deleted_at
        FROM location
        WHERE deleted_at IS NULL
        ORDER BY name COLLATE NOCASE ASC, created_at ASC
        ",
    )?;

    let locations = statement
        .query_map([], |row| {
            Ok(LocationSnapshot {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                notes: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                deleted_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(locations)
}

pub(crate) fn read_items(connection: &Connection) -> Result<Vec<ItemSnapshot>, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT id, project_id, name, description, notes, created_at, updated_at, deleted_at
        FROM item
        WHERE deleted_at IS NULL
        ORDER BY name COLLATE NOCASE ASC, created_at ASC
        ",
    )?;

    let items = statement
        .query_map([], |row| {
            Ok(ItemSnapshot {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                notes: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                deleted_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

pub(crate) fn read_notes(connection: &Connection) -> Result<Vec<NoteSnapshot>, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT id, project_id, title, body, note_type, created_at, updated_at, deleted_at
        FROM note
        WHERE deleted_at IS NULL
        ORDER BY title COLLATE NOCASE ASC, created_at ASC
        ",
    )?;

    let notes = statement
        .query_map([], |row| {
            Ok(NoteSnapshot {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                body: row.get(3)?,
                note_type: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                deleted_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(notes)
}

pub(crate) fn read_factions(connection: &Connection) -> Result<Vec<FactionSnapshot>, AppError> {
    let mut statement = connection.prepare("SELECT id, project_id, name, faction_type, description, goals, notes, created_at, updated_at, deleted_at FROM faction WHERE deleted_at IS NULL ORDER BY name COLLATE NOCASE, created_at")?;
    let rows = statement
        .query_map([], |row| {
            Ok(FactionSnapshot {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                faction_type: row.get(3)?,
                description: row.get(4)?,
                goals: row.get(5)?,
                notes: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                deleted_at: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn read_events(connection: &Connection) -> Result<Vec<EventSnapshot>, AppError> {
    let mut statement = connection.prepare("SELECT id, project_id, title, time_label, summary, outcome, notes, sort_order, created_at, updated_at, deleted_at FROM event WHERE deleted_at IS NULL ORDER BY sort_order, created_at")?;
    let rows = statement
        .query_map([], |row| {
            Ok(EventSnapshot {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                time_label: row.get(3)?,
                summary: row.get(4)?,
                outcome: row.get(5)?,
                notes: row.get(6)?,
                sort_order: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                deleted_at: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn read_bible_relationships(
    connection: &Connection,
) -> Result<Vec<BibleRelationshipSnapshot>, AppError> {
    let mut statement = connection.prepare("SELECT id, source_type, source_id, target_type, target_id, relationship_type, description, created_at, updated_at FROM bible_relationship WHERE deleted_at IS NULL ORDER BY created_at")?;
    let rows = statement
        .query_map([], |row| {
            Ok(BibleRelationshipSnapshot {
                id: row.get(0)?,
                source_type: row.get(1)?,
                source_id: row.get(2)?,
                target_type: row.get(3)?,
                target_id: row.get(4)?,
                relationship_type: row.get(5)?,
                description: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn read_active_scene_links(
    connection: &Connection,
) -> Result<Vec<SceneLinkSnapshot>, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT
            scene_link.id,
            scene_link.scene_id,
            scene_link.target_type,
            scene_link.target_id,
            scene_link.relation,
            scene_link.sort_order,
            scene.title,
            chapter.title,
            manuscript_area.title
        FROM scene_link
        INNER JOIN scene ON scene.id = scene_link.scene_id AND scene.deleted_at IS NULL
        INNER JOIN chapter ON chapter.id = scene.chapter_id AND chapter.deleted_at IS NULL
        INNER JOIN manuscript_area
            ON manuscript_area.id = chapter.area_id AND manuscript_area.deleted_at IS NULL
        WHERE scene_link.deleted_at IS NULL
        ORDER BY manuscript_area.sort_order ASC, chapter.sort_order ASC, scene.sort_order ASC,
            scene_link.sort_order ASC, scene_link.created_at ASC
        ",
    )?;

    let scene_links = statement
        .query_map([], |row| {
            Ok(SceneLinkSnapshot {
                id: row.get(0)?,
                scene_id: row.get(1)?,
                target_type: row.get(2)?,
                target_id: row.get(3)?,
                relation: row.get(4)?,
                sort_order: row.get(5)?,
                scene_title: row.get(6)?,
                chapter_title: row.get(7)?,
                area_title: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(scene_links)
}

pub(crate) fn read_scene_links_for_card(
    connection: &Connection,
    target_type: &str,
    target_id: &str,
) -> Result<Vec<SceneLinkSnapshot>, AppError> {
    Ok(read_active_scene_links(connection)?
        .into_iter()
        .filter(|link| link.target_type == target_type && link.target_id == target_id)
        .collect())
}

pub(crate) fn create_project_bible_card_in_connection(
    connection: &mut Connection,
    input: CreateProjectBibleCardInput,
) -> Result<(), AppError> {
    validate_target_type(&input.target_type)?;
    let project_id = read_active_project_id(connection)?;
    let now = timestamp();
    let id = Uuid::new_v4().to_string();

    match input.target_type.as_str() {
        "character" => {
            connection.execute(
                "
                INSERT INTO character (
                    id, project_id, name, role, description, notes, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, 'Neue Figur', '', '', '', ?3, ?3, NULL)
                ",
                params![id, project_id, now],
            )?;
        }
        "location" => {
            connection.execute(
                "
                INSERT INTO location (
                    id, project_id, name, description, notes, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, 'Neuer Ort', '', '', ?3, ?3, NULL)
                ",
                params![id, project_id, now],
            )?;
        }
        "item" => {
            connection.execute(
                "
                INSERT INTO item (
                    id, project_id, name, description, notes, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, 'Neuer Gegenstand', '', '', ?3, ?3, NULL)
                ",
                params![id, project_id, now],
            )?;
        }
        "note" => {
            connection.execute(
                "
                INSERT INTO note (
                    id, project_id, title, body, note_type, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, 'Neue Notiz', '', 'general', ?3, ?3, NULL)
                ",
                params![id, project_id, now],
            )?;
        }
        "faction" => {
            connection.execute("INSERT INTO faction (id, project_id, name, faction_type, description, goals, notes, created_at, updated_at, deleted_at) VALUES (?1, ?2, 'Neue Fraktion', '', '', '', '', ?3, ?3, NULL)", params![id, project_id, now])?;
        }
        "event" => {
            let sort_order: i64 = connection.query_row("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM event WHERE project_id = ?1 AND deleted_at IS NULL", params![project_id], |row| row.get(0))?;
            connection.execute("INSERT INTO event (id, project_id, title, time_label, summary, outcome, notes, sort_order, created_at, updated_at, deleted_at) VALUES (?1, ?2, 'Neues Ereignis', '', '', '', '', ?3, ?4, ?4, NULL)", params![id, project_id, sort_order, now])?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

pub(crate) fn upsert_bible_relationship_in_connection(
    connection: &mut Connection,
    input: UpsertBibleRelationshipInput,
) -> Result<(), AppError> {
    validate_target_type(&input.source_type)?;
    validate_target_type(&input.target_type)?;
    ensure_active_project_bible_card(connection, &input.source_type, &input.source_id)?;
    ensure_active_project_bible_card(connection, &input.target_type, &input.target_id)?;
    if input.source_type == input.target_type && input.source_id == input.target_id {
        return Err(AppError::new(
            "bible_relationship_self",
            "Eine Karte kann nicht mit sich selbst verknüpft werden.",
        ));
    }
    let valid = match input.relationship_type.as_str() {
        "related" => true,
        "knows" => {
            matches!(input.source_type.as_str(), "character" | "faction")
                && matches!(input.target_type.as_str(), "character" | "faction")
        }
        "family" => input.source_type == "character" && input.target_type == "character",
        "allied" | "enemy" => {
            matches!(input.source_type.as_str(), "character" | "faction")
                && matches!(input.target_type.as_str(), "character" | "faction")
        }
        "member_of" | "leads" => {
            input.target_type == "faction"
                && matches!(input.source_type.as_str(), "character" | "faction")
        }
        "owns" => input.target_type == "item",
        "located_in" => input.target_type == "location",
        "created_by" => {
            input.source_type == "item"
                && matches!(input.target_type.as_str(), "character" | "faction")
        }
        "participates_in" => input.target_type == "event",
        "caused" => input.target_type == "event",
        "follows" => input.source_type == "event" && input.target_type == "event",
        "uses_system" => input.target_type == "world_system",
        "depends_on" => input.source_type == "world_system" || input.target_type == "world_system",
        "affects" => input.source_type == "world_system" || input.target_type == "world_system",
        "created_by_system" => {
            input.source_type == "world_system" || input.target_type == "world_system"
        }
        _ => false,
    };
    if !valid {
        return Err(AppError::new(
            "bible_relationship_type_invalid",
            "Dieser Beziehungstyp passt nicht zu den ausgewählten Karten.",
        ));
    }
    let now = timestamp();
    if let Some(id) = input.id {
        let changed = connection.execute("UPDATE bible_relationship SET source_type=?1, source_id=?2, target_type=?3, target_id=?4, relationship_type=?5, description=?6, updated_at=?7 WHERE id=?8 AND deleted_at IS NULL", params![input.source_type, input.source_id, input.target_type, input.target_id, input.relationship_type, input.description.trim(), now, id])?;
        require_changed(
            changed,
            "bible_relationship_missing",
            "Die Beziehung wurde nicht gefunden.",
        )
    } else {
        connection.execute("INSERT INTO bible_relationship (id, source_type, source_id, target_type, target_id, relationship_type, description, created_at, updated_at, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, NULL)", params![Uuid::new_v4().to_string(), input.source_type, input.source_id, input.target_type, input.target_id, input.relationship_type, input.description.trim(), now])?;
        Ok(())
    }
}

pub(crate) fn update_project_bible_card_in_connection(
    connection: &mut Connection,
    input: UpdateProjectBibleCardInput,
) -> Result<(), AppError> {
    validate_target_type(&input.target_type)?;
    let now = timestamp();
    let changed = match input.target_type.as_str() {
        "character" => connection.execute(
            "
            UPDATE character
            SET name = ?1, role = ?2, description = ?3, notes = ?4, updated_at = ?5
            WHERE id = ?6 AND deleted_at IS NULL
            ",
            params![
                normalize_title(input.name.as_deref(), "Neue Figur"),
                input.role.unwrap_or_default(),
                input.description.unwrap_or_default(),
                input.notes.unwrap_or_default(),
                now,
                input.id
            ],
        )?,
        "location" => connection.execute(
            "
            UPDATE location
            SET name = ?1, description = ?2, notes = ?3, updated_at = ?4
            WHERE id = ?5 AND deleted_at IS NULL
            ",
            params![
                normalize_title(input.name.as_deref(), "Neuer Ort"),
                input.description.unwrap_or_default(),
                input.notes.unwrap_or_default(),
                now,
                input.id
            ],
        )?,
        "item" => connection.execute(
            "
            UPDATE item
            SET name = ?1, description = ?2, notes = ?3, updated_at = ?4
            WHERE id = ?5 AND deleted_at IS NULL
            ",
            params![
                normalize_title(input.name.as_deref(), "Neuer Gegenstand"),
                input.description.unwrap_or_default(),
                input.notes.unwrap_or_default(),
                now,
                input.id
            ],
        )?,
        "note" => connection.execute(
            "
            UPDATE note
            SET title = ?1, body = ?2, note_type = ?3, updated_at = ?4
            WHERE id = ?5 AND deleted_at IS NULL
            ",
            params![
                normalize_title(input.title.as_deref(), "Neue Notiz"),
                input.body.unwrap_or_default(),
                normalize_note_type(input.note_type.as_deref()),
                now,
                input.id
            ],
        )?,
        "faction" => connection.execute("UPDATE faction SET name=?1, faction_type=?2, description=?3, goals=?4, notes=?5, updated_at=?6 WHERE id=?7 AND deleted_at IS NULL", params![normalize_title(input.name.as_deref(), "Neue Fraktion"), input.role.unwrap_or_default(), input.description.unwrap_or_default(), input.body.unwrap_or_default(), input.notes.unwrap_or_default(), now, input.id])?,
        "event" => connection.execute("UPDATE event SET title=?1, time_label=?2, summary=?3, outcome=?4, notes=?5, updated_at=?6 WHERE id=?7 AND deleted_at IS NULL", params![normalize_title(input.title.as_deref(), "Neues Ereignis"), input.role.unwrap_or_default(), input.description.unwrap_or_default(), input.body.unwrap_or_default(), input.notes.unwrap_or_default(), now, input.id])?,
        _ => unreachable!(),
    };

    require_changed(
        changed,
        "project_bible_card_missing",
        "Die Projektbibel-Karte wurde nicht gefunden.",
    )
}

pub(crate) fn delete_project_bible_card_in_connection(
    connection: &mut Connection,
    input: DeleteProjectBibleCardInput,
) -> Result<(), AppError> {
    validate_target_type(&input.target_type)?;
    let tx = connection.transaction()?;
    let now = timestamp();
    let changed = match input.target_type.as_str() {
        "character" => tx.execute(
            "UPDATE character SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, input.id],
        )?,
        "location" => tx.execute(
            "UPDATE location SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, input.id],
        )?,
        "item" => tx.execute(
            "UPDATE item SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, input.id],
        )?,
        "note" => tx.execute(
            "UPDATE note SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, input.id],
        )?,
        "faction" => tx.execute("UPDATE faction SET deleted_at=?1, updated_at=?1 WHERE id=?2 AND deleted_at IS NULL", params![now, input.id])?,
        "event" => tx.execute("UPDATE event SET deleted_at=?1, updated_at=?1 WHERE id=?2 AND deleted_at IS NULL", params![now, input.id])?,
        _ => unreachable!(),
    };

    require_changed(
        changed,
        "project_bible_card_missing",
        "Die Projektbibel-Karte wurde nicht gefunden.",
    )?;
    tx.execute(
        "
        UPDATE scene_link
        SET deleted_at = ?1, updated_at = ?1
        WHERE target_type = ?2 AND target_id = ?3 AND deleted_at IS NULL
        ",
        params![now, input.target_type, input.id],
    )?;
    tx.execute("UPDATE bible_relationship SET deleted_at=?1, updated_at=?1 WHERE deleted_at IS NULL AND ((source_type=?2 AND source_id=?3) OR (target_type=?2 AND target_id=?3))", params![now, input.target_type, input.id])?;
    if input.target_type == "character" {
        tx.execute("UPDATE structure_character_link SET deleted_at=?1, updated_at=?1 WHERE character_id=?2 AND deleted_at IS NULL", params![now, input.id])?;
    }
    tx.commit()?;

    Ok(())
}

pub(crate) fn replace_scene_links_for_card_in_connection(
    connection: &mut Connection,
    input: ReplaceSceneLinksForCardInput,
) -> Result<(), AppError> {
    validate_target_type(&input.target_type)?;
    let tx = connection.transaction()?;
    ensure_active_project_bible_card(&tx, &input.target_type, &input.target_id)?;
    let mut scene_ids = Vec::new();
    for scene_id in &input.scene_ids {
        if !scene_ids.contains(scene_id) {
            scene_ids.push(scene_id.clone());
        }
    }
    for scene_id in &scene_ids {
        ensure_active_scene(&tx, scene_id)?;
    }

    let now = timestamp();
    tx.execute(
        "
        UPDATE scene_link
        SET deleted_at = ?1, updated_at = ?1
        WHERE target_type = ?2 AND target_id = ?3 AND relation = ?4 AND deleted_at IS NULL
        ",
        params![
            now,
            input.target_type,
            input.target_id,
            SCENE_LINK_RELATION_ASSIGNED
        ],
    )?;

    for (index, scene_id) in scene_ids.iter().enumerate() {
        tx.execute(
            "
            INSERT INTO scene_link (
                id, scene_id, target_type, target_id, relation, sort_order,
                created_at, updated_at, deleted_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, NULL)
            ",
            params![
                Uuid::new_v4().to_string(),
                scene_id,
                input.target_type,
                input.target_id,
                SCENE_LINK_RELATION_ASSIGNED,
                (index as i64) + 1,
                timestamp()
            ],
        )?;
    }

    tx.commit()?;

    Ok(())
}

pub(crate) fn validate_target_type(target_type: &str) -> Result<(), AppError> {
    match target_type {
        "character" | "location" | "item" | "note" | "faction" | "event" | "world_system" => Ok(()),
        _ => Err(AppError::new(
            "invalid_project_bible_card_type",
            "Dieser Projektbibel-Kartentyp wird nicht unterstützt.",
        )),
    }
}

pub(crate) fn normalize_note_type(value: Option<&str>) -> String {
    match value.unwrap_or("").trim() {
        "research" => "research".to_string(),
        "lore" => "lore".to_string(),
        "faction" => "faction".to_string(),
        "event" => "event".to_string(),
        "todo" => "todo".to_string(),
        _ => "general".to_string(),
    }
}

pub(crate) fn ensure_active_scene(connection: &Connection, scene_id: &str) -> Result<(), AppError> {
    let exists: Option<String> = connection
        .query_row(
            "SELECT id FROM scene WHERE id = ?1 AND deleted_at IS NULL",
            params![scene_id],
            |row| row.get(0),
        )
        .optional()?;

    exists
        .map(|_| ())
        .ok_or_else(|| AppError::new("scene_missing", "Die Szene wurde nicht gefunden."))
}

pub(crate) fn ensure_active_project_bible_card(
    connection: &Connection,
    target_type: &str,
    target_id: &str,
) -> Result<(), AppError> {
    validate_target_type(target_type)?;
    let query = match target_type {
        "character" => "SELECT id FROM character WHERE id = ?1 AND deleted_at IS NULL",
        "location" => "SELECT id FROM location WHERE id = ?1 AND deleted_at IS NULL",
        "item" => "SELECT id FROM item WHERE id = ?1 AND deleted_at IS NULL",
        "note" => "SELECT id FROM note WHERE id = ?1 AND deleted_at IS NULL",
        "faction" => "SELECT id FROM faction WHERE id = ?1 AND deleted_at IS NULL",
        "event" => "SELECT id FROM event WHERE id = ?1 AND deleted_at IS NULL",
        "world_system" => "SELECT id FROM world_system WHERE id = ?1 AND deleted_at IS NULL",
        _ => unreachable!(),
    };
    let exists: Option<String> = connection
        .query_row(query, params![target_id], |row| row.get(0))
        .optional()?;

    exists.map(|_| ()).ok_or_else(|| {
        AppError::new(
            "project_bible_card_missing",
            "Die Projektbibel-Karte wurde nicht gefunden.",
        )
    })
}
