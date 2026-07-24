use super::*;

pub(crate) fn create_manuscript_tables(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS novel (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            subtitle TEXT NOT NULL,
            author_name TEXT NOT NULL,
            sort_order INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (project_id) REFERENCES project(id)
        );

        CREATE TABLE IF NOT EXISTS manuscript_area (
            id TEXT PRIMARY KEY,
            novel_id TEXT NOT NULL,
            title TEXT NOT NULL,
            summary TEXT NOT NULL,
            sort_order INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (novel_id) REFERENCES novel(id)
        );

        CREATE TABLE IF NOT EXISTS chapter (
            id TEXT PRIMARY KEY,
            novel_id TEXT NOT NULL,
            area_id TEXT NOT NULL,
            title TEXT NOT NULL,
            summary TEXT NOT NULL,
            sort_order INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (novel_id) REFERENCES novel(id),
            FOREIGN KEY (area_id) REFERENCES manuscript_area(id)
        );

        CREATE TABLE IF NOT EXISTS scene (
            id TEXT PRIMARY KEY,
            chapter_id TEXT NOT NULL,
            title TEXT NOT NULL,
            text TEXT NOT NULL,
            status TEXT NOT NULL,
            summary TEXT NOT NULL,
            sort_order INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (chapter_id) REFERENCES chapter(id)
        );
        ",
    )?;

    Ok(())
}

pub(crate) fn create_project_bible_tables(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS character (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            name TEXT NOT NULL,
            role TEXT NOT NULL,
            description TEXT NOT NULL,
            notes TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (project_id) REFERENCES project(id)
        );

        CREATE TABLE IF NOT EXISTS location (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            notes TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (project_id) REFERENCES project(id)
        );

        CREATE TABLE IF NOT EXISTS item (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            notes TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (project_id) REFERENCES project(id)
        );

        CREATE TABLE IF NOT EXISTS note (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            note_type TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (project_id) REFERENCES project(id)
        );

        CREATE TABLE IF NOT EXISTS faction (
            id TEXT PRIMARY KEY, project_id TEXT NOT NULL, name TEXT NOT NULL,
            faction_type TEXT NOT NULL, description TEXT NOT NULL, goals TEXT NOT NULL,
            notes TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT,
            FOREIGN KEY (project_id) REFERENCES project(id)
        );

        CREATE TABLE IF NOT EXISTS event (
            id TEXT PRIMARY KEY, project_id TEXT NOT NULL, title TEXT NOT NULL,
            time_label TEXT NOT NULL, summary TEXT NOT NULL, outcome TEXT NOT NULL, notes TEXT NOT NULL,
            sort_order INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT,
            FOREIGN KEY (project_id) REFERENCES project(id)
        );

        CREATE TABLE IF NOT EXISTS world_system (
            id TEXT PRIMARY KEY, project_id TEXT NOT NULL, name TEXT NOT NULL,
            category TEXT NOT NULL, template_id TEXT NOT NULL, principle TEXT NOT NULL,
            rules TEXT NOT NULL, capabilities TEXT NOT NULL, costs TEXT NOT NULL,
            limits TEXT NOT NULL, access TEXT NOT NULL, distribution TEXT NOT NULL,
            world_impact TEXT NOT NULL, open_questions TEXT NOT NULL, notes TEXT NOT NULL,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT,
            FOREIGN KEY (project_id) REFERENCES project(id)
        );

        CREATE INDEX IF NOT EXISTS idx_world_system_category ON world_system(project_id, category, name) WHERE deleted_at IS NULL;

        CREATE TABLE IF NOT EXISTS bible_relationship (
            id TEXT PRIMARY KEY, source_type TEXT NOT NULL, source_id TEXT NOT NULL,
            target_type TEXT NOT NULL, target_id TEXT NOT NULL, relationship_type TEXT NOT NULL,
            description TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_bible_relationship_source ON bible_relationship(source_type, source_id) WHERE deleted_at IS NULL;
        CREATE INDEX IF NOT EXISTS idx_bible_relationship_target ON bible_relationship(target_type, target_id) WHERE deleted_at IS NULL;

        CREATE TABLE IF NOT EXISTS scene_link (
            id TEXT PRIMARY KEY,
            scene_id TEXT NOT NULL,
            target_type TEXT NOT NULL,
            target_id TEXT NOT NULL,
            relation TEXT NOT NULL,
            sort_order INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (scene_id) REFERENCES scene(id)
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_scene_link_active_unique
        ON scene_link(scene_id, target_type, target_id, relation)
        WHERE deleted_at IS NULL;

        CREATE INDEX IF NOT EXISTS idx_scene_link_target
        ON scene_link(target_type, target_id)
        WHERE deleted_at IS NULL;
        ",
    )?;

    Ok(())
}

pub(crate) fn create_ai_tables(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS ai_suggestion (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            scene_id TEXT NOT NULL,
            action_type TEXT NOT NULL,
            input_excerpt TEXT NOT NULL,
            context_summary TEXT NOT NULL,
            result TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (project_id) REFERENCES project(id),
            FOREIGN KEY (scene_id) REFERENCES scene(id)
        );

        CREATE INDEX IF NOT EXISTS idx_ai_suggestion_scene
        ON ai_suggestion(scene_id, action_type, status)
        WHERE deleted_at IS NULL;

        CREATE TABLE IF NOT EXISTS ai_bible_suggestion (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            target_type TEXT NOT NULL,
            target_id TEXT,
            mode TEXT NOT NULL,
            keywords TEXT NOT NULL,
            result_json TEXT NOT NULL,
            source_json TEXT,
            target_updated_at TEXT,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (project_id) REFERENCES project(id)
        );

        CREATE INDEX IF NOT EXISTS idx_ai_bible_suggestion_target
        ON ai_bible_suggestion(target_type, target_id, mode, status)
        WHERE deleted_at IS NULL;
        ",
    )?;

    Ok(())
}

pub(crate) fn create_export_profile_table(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS export_profile (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            include_novel_title INTEGER NOT NULL,
            include_area_titles INTEGER NOT NULL,
            include_chapter_titles INTEGER NOT NULL,
            include_scene_titles INTEGER NOT NULL,
            include_scene_text INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT,
            FOREIGN KEY (project_id) REFERENCES project(id)
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_export_profile_active_project
        ON export_profile(project_id) WHERE deleted_at IS NULL;
        ",
    )?;
    Ok(())
}

pub(crate) fn seed_initial_manuscript(connection: &Connection) -> Result<(), AppError> {
    let project: Option<(String, String, String)> = connection
        .query_row(
            "
            SELECT id, name, default_language
            FROM project
            WHERE deleted_at IS NULL
            ORDER BY created_at ASC
            LIMIT 1
            ",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;

    let Some((project_id, project_name, language)) = project else {
        return Ok(());
    };

    let active_novel_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM novel WHERE project_id = ?1 AND deleted_at IS NULL",
        params![project_id],
        |row| row.get(0),
    )?;

    if active_novel_count > 0 {
        return Ok(());
    }

    let now = timestamp();
    let novel_id = Uuid::new_v4().to_string();
    let area_id = Uuid::new_v4().to_string();
    let chapter_id = Uuid::new_v4().to_string();
    let scene_id = Uuid::new_v4().to_string();
    let (area_title, chapter_title, scene_title) = if language == "en" {
        ("Part 1", "Chapter 1", "New scene")
    } else {
        ("Bereich 1", "Kapitel 1", "Neue Szene")
    };

    connection.execute(
        "
        INSERT INTO novel (
            id, project_id, title, subtitle, author_name, sort_order,
            created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, '', '', 1, ?4, ?4, NULL)
        ",
        params![novel_id, project_id, project_name, now],
    )?;
    connection.execute(
        "
        INSERT INTO manuscript_area (
            id, novel_id, title, summary, sort_order, created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, '', 1, ?4, ?4, NULL)
        ",
        params![area_id, novel_id, area_title, now],
    )?;
    connection.execute(
        "
        INSERT INTO chapter (
            id, novel_id, area_id, title, summary, sort_order, created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, ?4, '', 1, ?5, ?5, NULL)
        ",
        params![chapter_id, novel_id, area_id, chapter_title, now],
    )?;
    connection.execute(
        "
        INSERT INTO scene (
            id, chapter_id, title, text, status, summary, sort_order,
            created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, '', ?4, '', 1, ?5, ?5, NULL)
        ",
        params![scene_id, chapter_id, scene_title, DEFAULT_SCENE_STATUS, now],
    )?;

    Ok(())
}

pub(crate) fn initialize_project_schema(
    connection: &Connection,
    name: &str,
    description: &str,
    default_language: &str,
) -> Result<(), AppError> {
    let now = timestamp();
    let project_id = Uuid::new_v4().to_string();

    connection.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );

        CREATE TABLE project (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            default_language TEXT NOT NULL,
            editor_font_family TEXT NOT NULL DEFAULT 'serif',
            editor_font_size INTEGER NOT NULL DEFAULT 19,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT
        );
        ",
    )?;

    create_manuscript_tables(connection)?;
    create_project_bible_tables(connection)?;
    create_ai_tables(connection)?;
    create_export_profile_table(connection)?;
    crate::story_structure::create_story_structure_tables(connection)?;

    connection.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
        params![SCHEMA_VERSION, now],
    )?;
    connection.execute(
        "
        INSERT INTO project (
            id,
            name,
            description,
            default_language,
            editor_font_family,
            editor_font_size,
            created_at,
            updated_at,
            deleted_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, NULL)
        ",
        params![
            project_id,
            name,
            description,
            default_language,
            DEFAULT_EDITOR_FONT_FAMILY,
            DEFAULT_EDITOR_FONT_SIZE,
            now
        ],
    )?;
    seed_initial_manuscript(connection)?;

    Ok(())
}

pub(crate) fn read_project_snapshot(
    connection: &Connection,
    path: &Path,
) -> Result<ProjectSnapshot, AppError> {
    ensure_schema_version(connection)?;

    let mut statement = connection.prepare(
        "
        SELECT
            id,
            name,
            description,
            default_language,
            editor_font_family,
            editor_font_size,
            created_at,
            updated_at
        FROM project
        WHERE deleted_at IS NULL
        LIMIT 1
        ",
    )?;

    let snapshot = statement.query_row([], |row| {
        Ok(ProjectSnapshot {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            default_language: row.get(3)?,
            editor_font_family: normalize_editor_font_family(&row.get::<_, String>(4)?),
            editor_font_size: normalize_editor_font_size(row.get(5)?),
            path: path.to_string_lossy().into_owned(),
            storage_kind: "file".to_string(),
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    Ok(snapshot)
}

pub(crate) fn ensure_schema_version(connection: &Connection) -> Result<(), AppError> {
    let version: i64 = connection.query_row(
        "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
        [],
        |row| row.get(0),
    )?;

    if version == 1 {
        migrate_schema_v1_to_v2(connection)?;
        return Ok(());
    }

    if (2..=9).contains(&version) {
        if version <= 2 {
            migrate_schema_v2_to_v3(connection)?;
        }
        if version <= 3 {
            migrate_schema_v3_to_v4(connection)?;
        }
        if version <= 4 {
            migrate_schema_v4_to_v5(connection)?;
        }
        if version <= 5 {
            migrate_schema_v5_to_v6(connection)?;
        }
        if version <= 6 {
            migrate_schema_v6_to_v7(connection)?;
        }
        if version <= 7 {
            migrate_schema_v7_to_v8(connection)?;
        }
        if version <= 8 {
            migrate_schema_v8_to_v9(connection)?;
        }
        migrate_schema_v9_to_v10(connection)?;
        migrate_schema_v10_to_v11(connection)?;
        migrate_schema_v11_to_v12(connection)?;
        migrate_schema_v12_to_v13(connection)?;
        migrate_schema_v13_to_v14(connection)?;
        migrate_schema_v14_to_v15(connection)?;
        migrate_schema_v15_to_v16(connection)?;
        return Ok(());
    }

    if version == 10 {
        migrate_schema_v10_to_v11(connection)?;
        migrate_schema_v11_to_v12(connection)?;
        migrate_schema_v12_to_v13(connection)?;
        migrate_schema_v13_to_v14(connection)?;
        migrate_schema_v14_to_v15(connection)?;
        migrate_schema_v15_to_v16(connection)?;
        return Ok(());
    }

    if version == 11 {
        migrate_schema_v11_to_v12(connection)?;
        migrate_schema_v12_to_v13(connection)?;
        migrate_schema_v13_to_v14(connection)?;
        migrate_schema_v14_to_v15(connection)?;
        migrate_schema_v15_to_v16(connection)?;
        return Ok(());
    }

    if version == 12 {
        migrate_schema_v12_to_v13(connection)?;
        migrate_schema_v13_to_v14(connection)?;
        migrate_schema_v14_to_v15(connection)?;
        migrate_schema_v15_to_v16(connection)?;
        return Ok(());
    }

    if version == 13 {
        migrate_schema_v13_to_v14(connection)?;
        migrate_schema_v14_to_v15(connection)?;
        migrate_schema_v15_to_v16(connection)?;
        return Ok(());
    }

    if version == 14 {
        migrate_schema_v14_to_v15(connection)?;
        migrate_schema_v15_to_v16(connection)?;
        return Ok(());
    }

    if version == 15 {
        migrate_schema_v15_to_v16(connection)?;
        return Ok(());
    }

    if version != SCHEMA_VERSION {
        return Err(AppError::new(
            "unsupported_schema_version",
            format!("Die Projektdatei nutzt Schema-Version {version}."),
        ));
    }

    Ok(())
}

pub(crate) fn migrate_schema_v1_to_v2(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        create_manuscript_tables(connection)?;
        create_project_bible_tables(connection)?;
        create_ai_tables(connection)?;
        create_export_profile_table(connection)?;
        crate::story_structure::create_story_structure_tables(connection)?;
        ensure_project_editor_settings_columns(connection)?;
        seed_initial_manuscript(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![SCHEMA_VERSION, timestamp()],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v11_to_v12(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        create_project_bible_tables(connection)?;
        let now = timestamp();
        connection.execute(
            "INSERT OR IGNORE INTO faction (id, project_id, name, faction_type, description, goals, notes, created_at, updated_at, deleted_at)
             SELECT id, project_id, title, '', body, '', '', created_at, updated_at, NULL FROM note WHERE note_type = 'faction' AND deleted_at IS NULL",
            [],
        )?;
        let mut statement = connection.prepare(
            "SELECT id, project_id, title, body, created_at, updated_at FROM note WHERE note_type = 'event' AND deleted_at IS NULL ORDER BY created_at, id",
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
        for (index, (id, project_id, title, body, created_at, updated_at)) in
            rows.into_iter().enumerate()
        {
            connection.execute(
                "INSERT OR IGNORE INTO event (id, project_id, title, time_label, summary, outcome, notes, sort_order, created_at, updated_at, deleted_at) VALUES (?1, ?2, ?3, '', ?4, '', '', ?5, ?6, ?7, NULL)",
                params![id, project_id, title, body, index as i64 + 1, created_at, updated_at],
            )?;
        }
        connection.execute("UPDATE scene_link SET target_type = 'faction', updated_at = ?1 WHERE target_type = 'note' AND target_id IN (SELECT id FROM note WHERE note_type = 'faction' AND deleted_at IS NULL)", params![now])?;
        connection.execute("UPDATE scene_link SET target_type = 'event', updated_at = ?1 WHERE target_type = 'note' AND target_id IN (SELECT id FROM note WHERE note_type = 'event' AND deleted_at IS NULL)", params![now])?;
        connection.execute("UPDATE note SET deleted_at = ?1, updated_at = ?1 WHERE note_type IN ('faction', 'event') AND deleted_at IS NULL", params![now])?;
        repair_typed_note_relationships(connection, &now)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![12, now],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v12_to_v13(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        create_project_bible_tables(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![13, timestamp()],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v13_to_v14(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        let now = timestamp();
        repair_typed_note_relationships(connection, &now)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![14, now],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v14_to_v15(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        crate::story_structure::create_story_structure_tables(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![15, timestamp()],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v15_to_v16(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        let now = timestamp();
        connection.execute("UPDATE project SET default_language=CASE WHEN lower(default_language) IN ('en','en-gb','en-us') THEN 'en' ELSE 'de' END, updated_at=?1 WHERE deleted_at IS NULL", params![now])?;
        connection.execute("UPDATE scene SET status=CASE status WHEN 'Fertig' THEN 'complete' WHEN 'Ueberarbeitung' THEN 'revision' WHEN 'Überarbeitung' THEN 'revision' WHEN 'complete' THEN 'complete' WHEN 'revision' THEN 'revision' ELSE 'draft' END, updated_at=?1 WHERE deleted_at IS NULL", params![now])?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![SCHEMA_VERSION, now],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn repair_typed_note_relationships(
    connection: &Connection,
    now: &str,
) -> Result<(), AppError> {
    connection.execute(
        "UPDATE bible_relationship
         SET source_type = 'faction', updated_at = ?1
         WHERE deleted_at IS NULL AND source_type = 'note'
           AND source_id IN (
             SELECT note.id FROM note
             INNER JOIN faction ON faction.id = note.id AND faction.deleted_at IS NULL
             WHERE note.note_type = 'faction' AND note.deleted_at IS NOT NULL
           )",
        params![now],
    )?;
    connection.execute(
        "UPDATE bible_relationship
         SET target_type = 'faction', updated_at = ?1
         WHERE deleted_at IS NULL AND target_type = 'note'
           AND target_id IN (
             SELECT note.id FROM note
             INNER JOIN faction ON faction.id = note.id AND faction.deleted_at IS NULL
             WHERE note.note_type = 'faction' AND note.deleted_at IS NOT NULL
           )",
        params![now],
    )?;
    connection.execute(
        "UPDATE bible_relationship
         SET source_type = 'event', updated_at = ?1
         WHERE deleted_at IS NULL AND source_type = 'note'
           AND source_id IN (
             SELECT note.id FROM note
             INNER JOIN event ON event.id = note.id AND event.deleted_at IS NULL
             WHERE note.note_type = 'event' AND note.deleted_at IS NOT NULL
           )",
        params![now],
    )?;
    connection.execute(
        "UPDATE bible_relationship
         SET target_type = 'event', updated_at = ?1
         WHERE deleted_at IS NULL AND target_type = 'note'
           AND target_id IN (
             SELECT note.id FROM note
             INNER JOIN event ON event.id = note.id AND event.deleted_at IS NULL
             WHERE note.note_type = 'event' AND note.deleted_at IS NOT NULL
           )",
        params![now],
    )?;
    Ok(())
}

pub(crate) fn migrate_schema_v2_to_v3(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = OFF; BEGIN;")?;
    let result = (|| {
        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS manuscript_area (
                id TEXT PRIMARY KEY,
                novel_id TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                sort_order INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                deleted_at TEXT
            );
            ",
        )?;

        if !table_has_column(connection, "chapter", "area_id")? {
            connection.execute("ALTER TABLE chapter ADD COLUMN area_id TEXT", [])?;
        }

        let mut novel_statement = connection.prepare(
            "
            SELECT id
            FROM novel
            WHERE deleted_at IS NULL
            ORDER BY sort_order ASC, created_at ASC
            ",
        )?;
        let novel_ids = novel_statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let now = timestamp();
        for novel_id in novel_ids {
            let area_id: Option<String> = connection
                .query_row(
                    "
                    SELECT id
                    FROM manuscript_area
                    WHERE novel_id = ?1 AND deleted_at IS NULL
                    ORDER BY sort_order ASC, created_at ASC
                    LIMIT 1
                    ",
                    params![novel_id],
                    |row| row.get(0),
                )
                .optional()?;
            let area_id = match area_id {
                Some(area_id) => area_id,
                None => {
                    let area_id = Uuid::new_v4().to_string();
                    connection.execute(
                        "
                        INSERT INTO manuscript_area (
                            id, novel_id, title, summary, sort_order,
                            created_at, updated_at, deleted_at
                        )
                        VALUES (?1, ?2, 'Bereich 1', '', 1, ?3, ?3, NULL)
                        ",
                        params![area_id, novel_id, now],
                    )?;
                    area_id
                }
            };

            connection.execute(
                "
                UPDATE chapter
                SET area_id = ?1, updated_at = ?2
                WHERE novel_id = ?3 AND deleted_at IS NULL AND (area_id IS NULL OR area_id = '')
                ",
                params![area_id, now, novel_id],
            )?;
        }

        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![3, timestamp()],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT; PRAGMA foreign_keys = ON;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK; PRAGMA foreign_keys = ON;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v3_to_v4(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        create_project_bible_tables(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![4, timestamp()],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v4_to_v5(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        create_ai_tables(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![5, timestamp()],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v5_to_v6(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        create_ai_tables(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![6, timestamp()],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v6_to_v7(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        ensure_project_editor_settings_columns(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![7, timestamp()],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v7_to_v8(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        crate::story_structure::create_story_structure_tables(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![8, timestamp()],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v8_to_v9(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        crate::story_structure::migrate_story_structure_v8_to_v9(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![9, timestamp()],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v9_to_v10(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        create_export_profile_table(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![10, timestamp()],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn migrate_schema_v10_to_v11(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        if !table_has_column(connection, "ai_bible_suggestion", "source_json")? {
            connection.execute(
                "ALTER TABLE ai_bible_suggestion ADD COLUMN source_json TEXT",
                [],
            )?;
        }
        if !table_has_column(connection, "ai_bible_suggestion", "target_updated_at")? {
            connection.execute(
                "ALTER TABLE ai_bible_suggestion ADD COLUMN target_updated_at TEXT",
                [],
            )?;
        }
        connection.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            params![11, timestamp()],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            connection.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

pub(crate) fn ensure_project_editor_settings_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    if !table_has_column(connection, "project", "editor_font_family")? {
        connection.execute(
            "ALTER TABLE project ADD COLUMN editor_font_family TEXT NOT NULL DEFAULT 'serif'",
            [],
        )?;
    }

    if !table_has_column(connection, "project", "editor_font_size")? {
        connection.execute(
            "ALTER TABLE project ADD COLUMN editor_font_size INTEGER NOT NULL DEFAULT 19",
            [],
        )?;
    }

    Ok(())
}

pub(crate) fn table_has_column(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, AppError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let column_names = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(column_names.iter().any(|name| name == column_name))
}
