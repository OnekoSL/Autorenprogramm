use super::*;

#[tauri::command]
pub fn get_manuscript_tree(state: State<AppState>) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| read_manuscript_tree(connection))
}

#[tauri::command]
pub fn create_area(
    input: CreateAreaInput,
    state: State<AppState>,
) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        create_area_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

#[tauri::command]
pub fn create_chapter(
    input: CreateChapterInput,
    state: State<AppState>,
) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        create_chapter_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

#[tauri::command]
pub fn create_scene(
    input: CreateSceneInput,
    state: State<AppState>,
) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        create_scene_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

#[tauri::command]
pub fn update_area(
    input: UpdateAreaInput,
    state: State<AppState>,
) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        update_area_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

#[tauri::command]
pub fn update_chapter(
    input: UpdateChapterInput,
    state: State<AppState>,
) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        update_chapter_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

#[tauri::command]
pub fn update_scene(
    input: UpdateSceneInput,
    state: State<AppState>,
) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        update_scene_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

#[tauri::command]
pub fn move_area(input: MoveAreaInput, state: State<AppState>) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        move_area_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

#[tauri::command]
pub fn move_chapter(
    input: MoveChapterInput,
    state: State<AppState>,
) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        move_chapter_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

#[tauri::command]
pub fn move_scene(
    input: MoveSceneInput,
    state: State<AppState>,
) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        move_scene_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

#[tauri::command]
pub fn delete_area(
    input: DeleteAreaInput,
    state: State<AppState>,
) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        delete_area_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

#[tauri::command]
pub fn delete_chapter(
    input: DeleteChapterInput,
    state: State<AppState>,
) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        delete_chapter_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

#[tauri::command]
pub fn delete_scene(
    input: DeleteSceneInput,
    state: State<AppState>,
) -> Result<ManuscriptTree, AppError> {
    with_active_connection(&state, |connection| {
        delete_scene_in_connection(connection, input)?;
        read_manuscript_tree(connection)
    })
}

pub(crate) fn read_manuscript_tree(connection: &Connection) -> Result<ManuscriptTree, AppError> {
    let novel = read_active_novel(connection)?;
    let mut area_statement = connection.prepare(
        "
        SELECT id, title, summary, sort_order
        FROM manuscript_area
        WHERE novel_id = ?1 AND deleted_at IS NULL
        ORDER BY sort_order ASC, created_at ASC
        ",
    )?;
    let areas = area_statement
        .query_map(params![novel.id.clone()], |row| {
            Ok(AreaSnapshot {
                id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                sort_order: row.get(3)?,
                chapters: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut areas_with_chapters = Vec::with_capacity(areas.len());
    for mut area in areas {
        area.chapters = read_chapters_for_area(connection, &area.id)?;
        areas_with_chapters.push(area);
    }

    Ok(ManuscriptTree {
        novel,
        areas: areas_with_chapters,
    })
}

pub(crate) fn read_active_novel(connection: &Connection) -> Result<NovelSnapshot, AppError> {
    connection
        .query_row(
            "
            SELECT id, title, subtitle, author_name
            FROM novel
            WHERE deleted_at IS NULL
            ORDER BY sort_order ASC, created_at ASC
            LIMIT 1
            ",
            [],
            |row| {
                Ok(NovelSnapshot {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    subtitle: row.get(2)?,
                    author_name: row.get(3)?,
                })
            },
        )
        .map_err(AppError::storage)
}

pub(crate) fn read_scenes_for_chapter(
    connection: &Connection,
    chapter_id: &str,
) -> Result<Vec<SceneSnapshot>, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT id, chapter_id, title, text, status, summary, sort_order
        FROM scene
        WHERE chapter_id = ?1 AND deleted_at IS NULL
        ORDER BY sort_order ASC, created_at ASC
        ",
    )?;

    let scenes = statement
        .query_map(params![chapter_id], |row| {
            Ok(SceneSnapshot {
                id: row.get(0)?,
                chapter_id: row.get(1)?,
                title: row.get(2)?,
                text: row.get(3)?,
                status: row.get(4)?,
                summary: row.get(5)?,
                sort_order: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(scenes)
}

pub(crate) fn read_chapters_for_area(
    connection: &Connection,
    area_id: &str,
) -> Result<Vec<ChapterSnapshot>, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT id, area_id, title, summary, sort_order
        FROM chapter
        WHERE area_id = ?1 AND deleted_at IS NULL
        ORDER BY sort_order ASC, created_at ASC
        ",
    )?;
    let chapters = statement
        .query_map(params![area_id], |row| {
            Ok(ChapterSnapshot {
                id: row.get(0)?,
                area_id: row.get(1)?,
                title: row.get(2)?,
                summary: row.get(3)?,
                sort_order: row.get(4)?,
                scenes: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut chapters_with_scenes = Vec::with_capacity(chapters.len());
    for mut chapter in chapters {
        chapter.scenes = read_scenes_for_chapter(connection, &chapter.id)?;
        chapters_with_scenes.push(chapter);
    }

    Ok(chapters_with_scenes)
}

pub(crate) fn create_area_in_connection(
    connection: &mut Connection,
    input: CreateAreaInput,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    let novel = read_active_novel(&tx)?;
    let areas = read_area_ids(&tx, &novel.id)?;
    let insert_index = input
        .after_area_id
        .as_ref()
        .and_then(|id| areas.iter().position(|area_id| area_id == id))
        .map(|index| index + 1)
        .unwrap_or(areas.len());
    let now = timestamp();
    let title = normalize_title(
        input.title.as_deref(),
        if read_active_project_language(&tx)? == "en" {
            "New part"
        } else {
            "Neuer Bereich"
        },
    );
    let area_id = Uuid::new_v4().to_string();

    tx.execute(
        "
        INSERT INTO manuscript_area (
            id, novel_id, title, summary, sort_order, created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, '', ?4, ?5, ?5, NULL)
        ",
        params![area_id, novel.id, title, (insert_index as i64) + 1, now],
    )?;
    normalize_area_order(&tx, &novel.id, Some((area_id, insert_index)))?;
    tx.commit()?;

    Ok(())
}

pub(crate) fn create_chapter_in_connection(
    connection: &mut Connection,
    input: CreateChapterInput,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    let novel = read_active_novel(&tx)?;
    let area_id = input
        .area_id
        .or_else(|| first_area_id(&tx).ok().flatten())
        .ok_or_else(|| AppError::new("area_missing", "Es gibt keinen aktiven Bereich."))?;
    ensure_active_area(&tx, &area_id)?;
    let chapters = read_chapter_ids(&tx, &area_id)?;
    let insert_index = input
        .after_chapter_id
        .as_ref()
        .and_then(|id| chapters.iter().position(|chapter_id| chapter_id == id))
        .map(|index| index + 1)
        .unwrap_or(chapters.len());
    let now = timestamp();
    let title = normalize_title(
        input.title.as_deref(),
        if read_active_project_language(&tx)? == "en" {
            "New chapter"
        } else {
            "Neues Kapitel"
        },
    );
    let chapter_id = Uuid::new_v4().to_string();

    tx.execute(
        "
        INSERT INTO chapter (
            id, novel_id, area_id, title, summary, sort_order, created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, ?4, '', ?5, ?6, ?6, NULL)
        ",
        params![
            chapter_id,
            novel.id,
            area_id,
            title,
            (insert_index as i64) + 1,
            now
        ],
    )?;
    normalize_chapter_order(&tx, &area_id, Some((chapter_id, insert_index)))?;
    tx.commit()?;

    Ok(())
}

pub(crate) fn create_scene_in_connection(
    connection: &mut Connection,
    input: CreateSceneInput,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    let chapter_id = input
        .chapter_id
        .or_else(|| first_chapter_id(&tx).ok().flatten())
        .ok_or_else(|| AppError::new("chapter_missing", "Es gibt kein aktives Kapitel."))?;
    ensure_active_chapter(&tx, &chapter_id)?;
    let scenes = read_scene_ids(&tx, &chapter_id)?;
    let insert_index = input
        .after_scene_id
        .as_ref()
        .and_then(|id| scenes.iter().position(|scene_id| scene_id == id))
        .map(|index| index + 1)
        .unwrap_or(scenes.len());
    let now = timestamp();
    let title = normalize_title(
        input.title.as_deref(),
        if read_active_project_language(&tx)? == "en" {
            "New scene"
        } else {
            "Neue Szene"
        },
    );
    let scene_id = Uuid::new_v4().to_string();

    tx.execute(
        "
        INSERT INTO scene (
            id, chapter_id, title, text, status, summary, sort_order,
            created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, '', ?4, '', ?5, ?6, ?6, NULL)
        ",
        params![
            scene_id,
            chapter_id,
            title,
            DEFAULT_SCENE_STATUS,
            (insert_index as i64) + 1,
            now
        ],
    )?;
    normalize_scene_order(&tx, &chapter_id, Some((scene_id, insert_index)))?;
    tx.commit()?;

    Ok(())
}

pub(crate) fn update_chapter_in_connection(
    connection: &mut Connection,
    input: UpdateChapterInput,
) -> Result<(), AppError> {
    let title = normalize_title(
        Some(&input.title),
        if read_active_project_language(connection)? == "en" {
            "New chapter"
        } else {
            "Neues Kapitel"
        },
    );
    let changed = connection.execute(
        "
        UPDATE chapter
        SET title = ?1, summary = ?2, updated_at = ?3
        WHERE id = ?4 AND deleted_at IS NULL
        ",
        params![title, input.summary, timestamp(), input.id],
    )?;

    require_changed(
        changed,
        "chapter_missing",
        "Das Kapitel wurde nicht gefunden.",
    )
}

pub(crate) fn update_area_in_connection(
    connection: &mut Connection,
    input: UpdateAreaInput,
) -> Result<(), AppError> {
    let title = normalize_title(
        Some(&input.title),
        if read_active_project_language(connection)? == "en" {
            "New part"
        } else {
            "Neuer Bereich"
        },
    );
    let changed = connection.execute(
        "
        UPDATE manuscript_area
        SET title = ?1, summary = ?2, updated_at = ?3
        WHERE id = ?4 AND deleted_at IS NULL
        ",
        params![title, input.summary, timestamp(), input.id],
    )?;

    require_changed(changed, "area_missing", "Der Bereich wurde nicht gefunden.")
}

pub(crate) fn update_scene_in_connection(
    connection: &mut Connection,
    input: UpdateSceneInput,
) -> Result<(), AppError> {
    let title = normalize_title(
        Some(&input.title),
        if read_active_project_language(connection)? == "en" {
            "New scene"
        } else {
            "Neue Szene"
        },
    );
    let status = normalize_scene_status(&input.status);
    let changed = connection.execute(
        "
        UPDATE scene
        SET title = ?1, text = ?2, status = ?3, summary = ?4, updated_at = ?5
        WHERE id = ?6 AND deleted_at IS NULL
        ",
        params![
            title,
            input.text,
            status,
            input.summary,
            timestamp(),
            input.id
        ],
    )?;

    require_changed(changed, "scene_missing", "Die Szene wurde nicht gefunden.")
}

pub(crate) fn move_chapter_in_connection(
    connection: &mut Connection,
    input: MoveChapterInput,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    ensure_active_chapter(&tx, &input.chapter_id)?;
    ensure_active_area(&tx, &input.target_area_id)?;
    let source_area_id: String = tx.query_row(
        "SELECT area_id FROM chapter WHERE id = ?1 AND deleted_at IS NULL",
        params![input.chapter_id],
        |row| row.get(0),
    )?;

    tx.execute(
        "
        UPDATE chapter
        SET area_id = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        ",
        params![input.target_area_id, timestamp(), input.chapter_id],
    )?;
    normalize_chapter_order(&tx, &source_area_id, None)?;
    normalize_chapter_order(
        &tx,
        &input.target_area_id,
        Some((input.chapter_id, input.target_index)),
    )?;
    tx.commit()?;

    Ok(())
}

pub(crate) fn move_area_in_connection(
    connection: &mut Connection,
    input: MoveAreaInput,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    let novel = read_active_novel(&tx)?;
    ensure_active_area(&tx, &input.area_id)?;
    normalize_area_order(&tx, &novel.id, Some((input.area_id, input.target_index)))?;
    tx.commit()?;

    Ok(())
}

pub(crate) fn move_scene_in_connection(
    connection: &mut Connection,
    input: MoveSceneInput,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    ensure_active_chapter(&tx, &input.target_chapter_id)?;
    let source_chapter_id: String = tx.query_row(
        "SELECT chapter_id FROM scene WHERE id = ?1 AND deleted_at IS NULL",
        params![input.scene_id],
        |row| row.get(0),
    )?;

    tx.execute(
        "
        UPDATE scene
        SET chapter_id = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        ",
        params![input.target_chapter_id, timestamp(), input.scene_id],
    )?;
    normalize_scene_order(&tx, &source_chapter_id, None)?;
    normalize_scene_order(
        &tx,
        &input.target_chapter_id,
        Some((input.scene_id, input.target_index)),
    )?;
    tx.commit()?;

    Ok(())
}

pub(crate) fn delete_chapter_in_connection(
    connection: &mut Connection,
    input: DeleteChapterInput,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    ensure_active_chapter(&tx, &input.id)?;
    let area_id: String = tx.query_row(
        "SELECT area_id FROM chapter WHERE id = ?1 AND deleted_at IS NULL",
        params![input.id],
        |row| row.get(0),
    )?;
    let now = timestamp();

    tx.execute(
        "UPDATE scene_link SET deleted_at = ?1, updated_at = ?1
         WHERE deleted_at IS NULL AND scene_id IN (
             SELECT id FROM scene WHERE chapter_id = ?2 AND deleted_at IS NULL
         )",
        params![now, input.id],
    )?;
    tx.execute("UPDATE structure_scene_link SET deleted_at=?1, updated_at=?1 WHERE deleted_at IS NULL AND scene_id IN (SELECT id FROM scene WHERE chapter_id=?2 AND deleted_at IS NULL)", params![now, input.id])?;
    tx.execute(
        "UPDATE scene SET deleted_at = ?1, updated_at = ?1 WHERE chapter_id = ?2 AND deleted_at IS NULL",
        params![now, input.id],
    )?;
    tx.execute(
        "UPDATE chapter SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        params![now, input.id],
    )?;
    normalize_chapter_order(&tx, &area_id, None)?;
    tx.commit()?;

    Ok(())
}

pub(crate) fn delete_area_in_connection(
    connection: &mut Connection,
    input: DeleteAreaInput,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    let novel = read_active_novel(&tx)?;
    ensure_active_area(&tx, &input.id)?;
    let now = timestamp();

    tx.execute(
        "UPDATE scene_link SET deleted_at = ?1, updated_at = ?1
         WHERE deleted_at IS NULL AND scene_id IN (
             SELECT scene.id FROM scene
             INNER JOIN chapter ON chapter.id = scene.chapter_id
             WHERE chapter.area_id = ?2 AND scene.deleted_at IS NULL
         )",
        params![now, input.id],
    )?;
    tx.execute("UPDATE structure_scene_link SET deleted_at=?1, updated_at=?1 WHERE deleted_at IS NULL AND scene_id IN (SELECT scene.id FROM scene INNER JOIN chapter ON chapter.id=scene.chapter_id WHERE chapter.area_id=?2 AND scene.deleted_at IS NULL)", params![now, input.id])?;
    tx.execute(
        "
        UPDATE scene
        SET deleted_at = ?1, updated_at = ?1
        WHERE chapter_id IN (
            SELECT id FROM chapter WHERE area_id = ?2 AND deleted_at IS NULL
        )
        AND deleted_at IS NULL
        ",
        params![now, input.id],
    )?;
    tx.execute(
        "
        UPDATE chapter
        SET deleted_at = ?1, updated_at = ?1
        WHERE area_id = ?2 AND deleted_at IS NULL
        ",
        params![now, input.id],
    )?;
    tx.execute(
        "
        UPDATE manuscript_area
        SET deleted_at = ?1, updated_at = ?1
        WHERE id = ?2 AND deleted_at IS NULL
        ",
        params![now, input.id],
    )?;
    normalize_area_order(&tx, &novel.id, None)?;
    tx.commit()?;

    Ok(())
}

pub(crate) fn delete_scene_in_connection(
    connection: &mut Connection,
    input: DeleteSceneInput,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    let chapter_id: String = tx.query_row(
        "SELECT chapter_id FROM scene WHERE id = ?1 AND deleted_at IS NULL",
        params![input.id],
        |row| row.get(0),
    )?;
    let now = timestamp();

    tx.execute(
        "UPDATE scene_link SET deleted_at = ?1, updated_at = ?1 WHERE scene_id = ?2 AND deleted_at IS NULL",
        params![now, input.id],
    )?;
    tx.execute("UPDATE structure_scene_link SET deleted_at=?1, updated_at=?1 WHERE scene_id=?2 AND deleted_at IS NULL", params![now, input.id])?;
    tx.execute(
        "UPDATE scene SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        params![now, input.id],
    )?;
    normalize_scene_order(&tx, &chapter_id, None)?;
    tx.commit()?;

    Ok(())
}

pub(crate) fn read_area_ids(
    connection: &Connection,
    novel_id: &str,
) -> Result<Vec<String>, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT id
        FROM manuscript_area
        WHERE novel_id = ?1 AND deleted_at IS NULL
        ORDER BY sort_order ASC, created_at ASC
        ",
    )?;

    let ids = statement
        .query_map(params![novel_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ids)
}

pub(crate) fn read_chapter_ids(
    connection: &Connection,
    area_id: &str,
) -> Result<Vec<String>, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT id
        FROM chapter
        WHERE area_id = ?1 AND deleted_at IS NULL
        ORDER BY sort_order ASC, created_at ASC
        ",
    )?;

    let ids = statement
        .query_map(params![area_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ids)
}

pub(crate) fn read_scene_ids(
    connection: &Connection,
    chapter_id: &str,
) -> Result<Vec<String>, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT id
        FROM scene
        WHERE chapter_id = ?1 AND deleted_at IS NULL
        ORDER BY sort_order ASC, created_at ASC
        ",
    )?;

    let ids = statement
        .query_map(params![chapter_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ids)
}

pub(crate) fn first_chapter_id(connection: &Connection) -> Result<Option<String>, AppError> {
    Ok(connection
        .query_row(
            "
            SELECT chapter.id
            FROM chapter
            JOIN novel ON novel.id = chapter.novel_id
            JOIN manuscript_area ON manuscript_area.id = chapter.area_id
            WHERE chapter.deleted_at IS NULL
                AND novel.deleted_at IS NULL
                AND manuscript_area.deleted_at IS NULL
            ORDER BY novel.sort_order ASC, manuscript_area.sort_order ASC, chapter.sort_order ASC, chapter.created_at ASC
            LIMIT 1
            ",
            [],
            |row| row.get(0),
        )
        .optional()?)
}

pub(crate) fn first_area_id(connection: &Connection) -> Result<Option<String>, AppError> {
    Ok(connection
        .query_row(
            "
            SELECT manuscript_area.id
            FROM manuscript_area
            JOIN novel ON novel.id = manuscript_area.novel_id
            WHERE manuscript_area.deleted_at IS NULL AND novel.deleted_at IS NULL
            ORDER BY novel.sort_order ASC, manuscript_area.sort_order ASC, manuscript_area.created_at ASC
            LIMIT 1
            ",
            [],
            |row| row.get(0),
        )
        .optional()?)
}

pub(crate) fn ensure_active_area(connection: &Connection, area_id: &str) -> Result<(), AppError> {
    let exists: Option<String> = connection
        .query_row(
            "SELECT id FROM manuscript_area WHERE id = ?1 AND deleted_at IS NULL",
            params![area_id],
            |row| row.get(0),
        )
        .optional()?;

    exists
        .map(|_| ())
        .ok_or_else(|| AppError::new("area_missing", "Der Bereich wurde nicht gefunden."))
}

pub(crate) fn ensure_active_chapter(
    connection: &Connection,
    chapter_id: &str,
) -> Result<(), AppError> {
    let exists: Option<String> = connection
        .query_row(
            "SELECT id FROM chapter WHERE id = ?1 AND deleted_at IS NULL",
            params![chapter_id],
            |row| row.get(0),
        )
        .optional()?;

    exists
        .map(|_| ())
        .ok_or_else(|| AppError::new("chapter_missing", "Das Kapitel wurde nicht gefunden."))
}

pub(crate) fn normalize_area_order(
    connection: &Connection,
    novel_id: &str,
    moved: Option<(String, usize)>,
) -> Result<(), AppError> {
    let mut ids = read_area_ids(connection, novel_id)?;
    if let Some((moved_id, target_index)) = moved {
        ids.retain(|id| id != &moved_id);
        let index = target_index.min(ids.len());
        ids.insert(index, moved_id);
    }

    for (index, id) in ids.iter().enumerate() {
        connection.execute(
            "UPDATE manuscript_area SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![(index as i64) + 1, timestamp(), id],
        )?;
    }

    Ok(())
}

pub(crate) fn normalize_chapter_order(
    connection: &Connection,
    area_id: &str,
    moved: Option<(String, usize)>,
) -> Result<(), AppError> {
    let mut ids = read_chapter_ids(connection, area_id)?;
    if let Some((moved_id, target_index)) = moved {
        ids.retain(|id| id != &moved_id);
        let index = target_index.min(ids.len());
        ids.insert(index, moved_id);
    }

    for (index, id) in ids.iter().enumerate() {
        connection.execute(
            "UPDATE chapter SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![(index as i64) + 1, timestamp(), id],
        )?;
    }

    Ok(())
}

pub(crate) fn normalize_scene_order(
    connection: &Connection,
    chapter_id: &str,
    moved: Option<(String, usize)>,
) -> Result<(), AppError> {
    let mut ids = read_scene_ids(connection, chapter_id)?;
    if let Some((moved_id, target_index)) = moved {
        ids.retain(|id| id != &moved_id);
        let index = target_index.min(ids.len());
        ids.insert(index, moved_id);
    }

    for (index, id) in ids.iter().enumerate() {
        connection.execute(
            "UPDATE scene SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![(index as i64) + 1, timestamp(), id],
        )?;
    }

    Ok(())
}

pub(crate) fn normalize_title(value: Option<&str>, fallback: &str) -> String {
    let title = value.unwrap_or("").trim();
    if title.is_empty() {
        fallback.to_string()
    } else {
        title.to_string()
    }
}

pub(crate) fn normalize_scene_status(value: &str) -> String {
    match value.trim() {
        "revision" => "revision".to_string(),
        "complete" => "complete".to_string(),
        _ => DEFAULT_SCENE_STATUS.to_string(),
    }
}

pub(crate) fn validate_project_language(value: &str) -> Result<&'static str, AppError> {
    match value.trim() {
        "de" => Ok("de"),
        "en" => Ok("en"),
        _ => Err(AppError::new(
            "invalid_project_language",
            "Unterstützt werden Deutsch und Englisch.",
        )),
    }
}

pub(crate) fn require_changed(
    changed: usize,
    code: &'static str,
    message: &'static str,
) -> Result<(), AppError> {
    if changed == 0 {
        Err(AppError::new(code, message))
    } else {
        Ok(())
    }
}

pub(crate) fn read_active_project_id(connection: &Connection) -> Result<String, AppError> {
    connection
        .query_row(
            "
            SELECT id
            FROM project
            WHERE deleted_at IS NULL
            ORDER BY created_at ASC
            LIMIT 1
            ",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::storage)
}
