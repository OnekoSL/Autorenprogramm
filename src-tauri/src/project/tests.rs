use super::*;
use std::io::Read;
use tempfile::tempdir;

fn input(path: PathBuf, name: &str) -> CreateProjectInput {
    CreateProjectInput {
        name: name.to_string(),
        description: Some("Ein Testprojekt".to_string()),
        default_language: "de".to_string(),
        path: path.to_string_lossy().into_owned(),
    }
}

fn mock_models_server(status_code: u16, body: &'static str) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buffer = [0; 65536];
            let _ = std::io::Read::read(&mut stream, &mut buffer);
            let response = format!(
                    "HTTP/1.1 {status_code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
        }
    });

    format!("http://{address}/v1")
}

fn create_legacy_v1_project(path: PathBuf, name: &str) -> PathBuf {
    let path = normalize_project_path(&path.to_string_lossy()).unwrap();
    let connection = Connection::open(&path).unwrap();
    let now = timestamp();

    connection
        .execute_batch(
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
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );
                ",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (1, ?1)",
            params![now],
        )
        .unwrap();
    connection
        .execute(
            "
                INSERT INTO project (
                    id, name, description, default_language, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, '', 'de', ?3, ?3, NULL)
                ",
            params![Uuid::new_v4().to_string(), name, now],
        )
        .unwrap();

    path
}

fn create_legacy_v2_project(path: PathBuf, name: &str) -> PathBuf {
    let path = normalize_project_path(&path.to_string_lossy()).unwrap();
    let connection = Connection::open(&path).unwrap();
    let now = timestamp();
    let project_id = Uuid::new_v4().to_string();
    let novel_id = Uuid::new_v4().to_string();
    let chapter_1_id = Uuid::new_v4().to_string();
    let chapter_2_id = Uuid::new_v4().to_string();
    let scene_id = Uuid::new_v4().to_string();

    connection
        .execute_batch(
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
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );

                CREATE TABLE novel (
                    id TEXT PRIMARY KEY,
                    project_id TEXT NOT NULL,
                    title TEXT NOT NULL,
                    subtitle TEXT NOT NULL,
                    author_name TEXT NOT NULL,
                    sort_order INTEGER NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );

                CREATE TABLE chapter (
                    id TEXT PRIMARY KEY,
                    novel_id TEXT NOT NULL,
                    title TEXT NOT NULL,
                    summary TEXT NOT NULL,
                    sort_order INTEGER NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );

                CREATE TABLE scene (
                    id TEXT PRIMARY KEY,
                    chapter_id TEXT NOT NULL,
                    title TEXT NOT NULL,
                    text TEXT NOT NULL,
                    status TEXT NOT NULL,
                    summary TEXT NOT NULL,
                    sort_order INTEGER NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );
                ",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (2, ?1)",
            params![now],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO project VALUES (?1, ?2, '', 'de', ?3, ?3, NULL)",
            params![project_id, name, now],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO novel VALUES (?1, ?2, ?3, '', '', 1, ?4, ?4, NULL)",
            params![novel_id, project_id, name, now],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO chapter VALUES (?1, ?2, 'Kapitel A', '', 1, ?3, ?3, NULL)",
            params![chapter_1_id, novel_id, now],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO chapter VALUES (?1, ?2, 'Kapitel B', '', 2, ?3, ?3, NULL)",
            params![chapter_2_id, novel_id, now],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO scene VALUES (?1, ?2, 'Szene A', '', ?3, '', 1, ?4, ?4, NULL)",
            params![scene_id, chapter_1_id, DEFAULT_SCENE_STATUS, now],
        )
        .unwrap();

    path
}

fn create_legacy_v3_project(path: PathBuf, name: &str) -> PathBuf {
    let path = normalize_project_path(&path.to_string_lossy()).unwrap();
    let connection = Connection::open(&path).unwrap();
    let now = timestamp();
    let project_id = Uuid::new_v4().to_string();

    connection
        .execute_batch(
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
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );
                ",
        )
        .unwrap();
    create_manuscript_tables(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (3, ?1)",
            params![now],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO project VALUES (?1, ?2, '', 'de', ?3, ?3, NULL)",
            params![project_id, name, now],
        )
        .unwrap();
    seed_initial_manuscript(&connection).unwrap();

    path
}

fn create_legacy_v6_project(path: PathBuf, name: &str) -> PathBuf {
    let path = normalize_project_path(&path.to_string_lossy()).unwrap();
    let connection = Connection::open(&path).unwrap();
    let now = timestamp();
    let project_id = Uuid::new_v4().to_string();

    connection
        .execute_batch(
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
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );
                ",
        )
        .unwrap();
    create_manuscript_tables(&connection).unwrap();
    create_project_bible_tables(&connection).unwrap();
    create_ai_tables(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (6, ?1)",
            params![now],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO project VALUES (?1, ?2, '', 'de', ?3, ?3, NULL)",
            params![project_id, name, now],
        )
        .unwrap();
    seed_initial_manuscript(&connection).unwrap();

    path
}

fn table_exists(connection: &Connection, table_name: &str) -> bool {
    connection
        .query_row(
            "
                SELECT name
                FROM sqlite_master
                WHERE type = 'table' AND name = ?1
                ",
            params![table_name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .unwrap()
        .is_some()
}

#[test]
fn creates_project_file_with_schema_and_project_record() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("Eldara");

    let snapshot = create_project_file(input(path, "Eldara")).unwrap();

    assert!(snapshot.path.ends_with(".autorenprojekt"));
    assert_eq!(snapshot.name, "Eldara");
    assert_eq!(snapshot.default_language, "de");
    assert_eq!(snapshot.editor_font_family, DEFAULT_EDITOR_FONT_FAMILY);
    assert_eq!(snapshot.editor_font_size, DEFAULT_EDITOR_FONT_SIZE);

    let connection = Connection::open(&snapshot.path).unwrap();
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM project", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let tree = read_manuscript_tree(&connection).unwrap();
    assert_eq!(tree.novel.title, "Eldara");
    assert_eq!(tree.areas.len(), 1);
    assert_eq!(tree.areas[0].title, "Bereich 1");
    assert_eq!(tree.areas[0].chapters.len(), 1);
    assert_eq!(tree.areas[0].chapters[0].title, "Kapitel 1");
    assert_eq!(tree.areas[0].chapters[0].scenes.len(), 1);
    assert_eq!(tree.areas[0].chapters[0].scenes[0].title, "Neue Szene");
}

#[test]
fn creates_english_project_with_localised_manuscript_defaults() {
    let dir = tempdir().unwrap();
    let mut project_input = input(dir.path().join("English"), "English");
    project_input.default_language = "en".to_string();

    let snapshot = create_project_file(project_input).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();

    assert_eq!(snapshot.default_language, "en");
    assert_eq!(tree.areas[0].title, "Part 1");
    assert_eq!(tree.areas[0].chapters[0].title, "Chapter 1");
    assert_eq!(tree.areas[0].chapters[0].scenes[0].title, "New scene");
    assert_eq!(tree.areas[0].chapters[0].scenes[0].status, "draft");
}

#[test]
fn rejects_unknown_and_locked_project_languages() {
    let dir = tempdir().unwrap();
    let mut invalid = input(dir.path().join("Invalid"), "Invalid");
    invalid.default_language = "fr".to_string();
    assert_eq!(
        create_project_file(invalid).unwrap_err().code,
        "invalid_project_language"
    );

    let created = create_project_file(input(dir.path().join("Locked"), "Locked")).unwrap();
    let error = update_project_file(
        created.path,
        UpdateProjectInput {
            name: "Locked".to_string(),
            description: String::new(),
            default_language: "en".to_string(),
            editor_font_family: DEFAULT_EDITOR_FONT_FAMILY.to_string(),
            editor_font_size: DEFAULT_EDITOR_FONT_SIZE,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "project_language_locked");
}

#[test]
fn creates_project_bible_schema_for_new_projects() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();
    let version: i64 = connection
        .query_row(
            "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(version, SCHEMA_VERSION);
    for table_name in [
        "character",
        "location",
        "item",
        "note",
        "faction",
        "event",
        "bible_relationship",
        "world_system",
        "scene_link",
        "ai_suggestion",
        "ai_bible_suggestion",
    ] {
        assert!(table_exists(&connection, table_name));
    }
}

#[test]
fn does_not_overwrite_existing_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("Eldara.autorenprojekt");
    fs::write(&path, "already here").unwrap();

    let error = create_project_file(input(path, "Eldara")).unwrap_err();

    assert_eq!(error.code, "project_file_exists");
}

#[test]
fn opens_existing_valid_project() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("Eldara");
    let created = create_project_file(input(path, "Eldara")).unwrap();

    let opened = open_project_file(created.path.clone()).unwrap();

    assert_eq!(opened.id, created.id);
    assert_eq!(opened.name, "Eldara");
}

#[test]
fn saves_project_copy_as_openable_backup() {
    let dir = tempdir().unwrap();
    let created = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let backup_path = dir.path().join("Eldara Backup.autorenprojekt");

    let copied_path =
        save_project_copy_as_file(&created.path, backup_path.to_str().unwrap()).unwrap();

    assert!(copied_path.exists());
    assert!(fs::metadata(&copied_path).unwrap().len() > 0);
    let opened = open_project_file(copied_path.to_string_lossy().into_owned()).unwrap();
    assert_eq!(opened.id, created.id);
    assert_eq!(opened.name, created.name);
}

#[test]
fn rejects_empty_project_copy_path() {
    let dir = tempdir().unwrap();
    let created = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();

    let error = save_project_copy_as_file(&created.path, "   ").unwrap_err();

    assert_eq!(error.code, "invalid_project_path");
}

#[test]
fn rejects_project_copy_to_active_path() {
    let dir = tempdir().unwrap();
    let created = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();

    let error = save_project_copy_as_file(&created.path, &created.path).unwrap_err();

    assert_eq!(error.code, "same_project_copy_path");
}

#[test]
fn updates_project_metadata() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("Eldara");
    let created = create_project_file(input(path, "Eldara")).unwrap();

    let updated = update_project_file(
        created.path.clone(),
        UpdateProjectInput {
            name: "Die blutigen drei".to_string(),
            description: "Ein blutiger Fantasyroman".to_string(),
            default_language: "de".to_string(),
            editor_font_family: "monospace".to_string(),
            editor_font_size: 22,
        },
    )
    .unwrap();

    assert_eq!(updated.name, "Die blutigen drei");
    assert_eq!(updated.description, "Ein blutiger Fantasyroman");
    assert_eq!(updated.default_language, "de");
    assert_eq!(updated.editor_font_family, "monospace");
    assert_eq!(updated.editor_font_size, 22);

    let reopened = open_project_file(created.path).unwrap();
    assert_eq!(reopened.name, "Die blutigen drei");
    assert_eq!(reopened.description, "Ein blutiger Fantasyroman");
    assert_eq!(reopened.default_language, "de");
    assert_eq!(reopened.editor_font_family, "monospace");
    assert_eq!(reopened.editor_font_size, 22);
}

#[test]
fn rejects_empty_project_metadata_updates() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("Eldara");
    let created = create_project_file(input(path, "Eldara")).unwrap();

    let name_error = update_project_file(
        created.path.clone(),
        UpdateProjectInput {
            name: " ".to_string(),
            description: String::new(),
            default_language: "de".to_string(),
            editor_font_family: DEFAULT_EDITOR_FONT_FAMILY.to_string(),
            editor_font_size: DEFAULT_EDITOR_FONT_SIZE,
        },
    )
    .unwrap_err();
    assert_eq!(name_error.code, "invalid_project_name");

    let language_error = update_project_file(
        created.path,
        UpdateProjectInput {
            name: "Eldara".to_string(),
            description: String::new(),
            default_language: " ".to_string(),
            editor_font_family: DEFAULT_EDITOR_FONT_FAMILY.to_string(),
            editor_font_size: DEFAULT_EDITOR_FONT_SIZE,
        },
    )
    .unwrap_err();
    assert_eq!(language_error.code, "invalid_project_language");
}

#[test]
fn rejects_invalid_editor_settings() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("Eldara");
    let created = create_project_file(input(path, "Eldara")).unwrap();

    let font_error = update_project_file(
        created.path.clone(),
        UpdateProjectInput {
            name: "Eldara".to_string(),
            description: String::new(),
            default_language: "de".to_string(),
            editor_font_family: "comic".to_string(),
            editor_font_size: DEFAULT_EDITOR_FONT_SIZE,
        },
    )
    .unwrap_err();
    assert_eq!(font_error.code, "invalid_editor_font_family");

    let size_error = update_project_file(
        created.path,
        UpdateProjectInput {
            name: "Eldara".to_string(),
            description: String::new(),
            default_language: "de".to_string(),
            editor_font_family: DEFAULT_EDITOR_FONT_FAMILY.to_string(),
            editor_font_size: 42,
        },
    )
    .unwrap_err();
    assert_eq!(size_error.code, "invalid_editor_font_size");
}

#[test]
fn searches_scenes_in_title_summary_and_text_case_insensitive() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id,
            title: "Der Schwur".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: "Neblige Kueste".to_string(),
            text: "Mira sieht das graue Meer.".to_string(),
        },
    )
    .unwrap();

    let title_results = search_scenes_in_connection(&connection, "schwur").unwrap();
    let summary_results = search_scenes_in_connection(&connection, "KUESTE").unwrap();
    let text_results = search_scenes_in_connection(&connection, "meer").unwrap();

    assert_eq!(title_results.len(), 1);
    assert_eq!(title_results[0].matched_fields, vec!["Titel"]);
    assert_eq!(summary_results.len(), 1);
    assert_eq!(summary_results[0].matched_fields, vec!["Kurzbeschreibung"]);
    assert_eq!(text_results.len(), 1);
    assert_eq!(text_results[0].matched_fields, vec!["Text"]);
    assert!(text_results[0].snippets[0].contains("Meer"));
}

#[test]
fn searches_scenes_in_manuscript_order() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let initial_tree = read_manuscript_tree(&connection).unwrap();
    let chapter_id = initial_tree.areas[0].chapters[0].id.clone();
    let first_scene = initial_tree.areas[0].chapters[0].scenes[0].clone();
    create_scene_in_connection(
        &mut connection,
        CreateSceneInput {
            chapter_id: Some(chapter_id),
            title: Some("Zweite Szene".to_string()),
            after_scene_id: Some(first_scene.id.clone()),
        },
    )
    .unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    let second_scene = tree.areas[0].chapters[0].scenes[1].clone();

    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: first_scene.id.clone(),
            title: "Erster Treffer".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
            text: "Das Suchwort steht zuerst.".to_string(),
        },
    )
    .unwrap();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: second_scene.id.clone(),
            title: "Zweiter Treffer".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
            text: "Das Suchwort steht spaeter.".to_string(),
        },
    )
    .unwrap();

    let results = search_scenes_in_connection(&connection, "Suchwort").unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].scene_title, "Erster Treffer");
    assert_eq!(results[1].scene_title, "Zweiter Treffer");
}

#[test]
fn search_ignores_archived_scenes() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id.clone(),
            title: "Archivierter Treffer".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
            text: "Dieses Suchwort soll verschwinden.".to_string(),
        },
    )
    .unwrap();
    delete_scene_in_connection(&mut connection, DeleteSceneInput { id: scene.id }).unwrap();

    let results = search_scenes_in_connection(&connection, "Suchwort").unwrap();

    assert!(results.is_empty());
}

#[test]
fn search_project_finds_scenes_and_project_bible_cards() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id,
            title: "Kuestenfund".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
            text: "Das Suchwort steht im Manuskript.".to_string(),
        },
    )
    .unwrap();

    for target_type in ["character", "location", "item", "note"] {
        create_project_bible_card_in_connection(
            &mut connection,
            CreateProjectBibleCardInput {
                target_type: target_type.to_string(),
            },
        )
        .unwrap();
    }
    let bible = read_project_bible(&connection).unwrap();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "character".to_string(),
            id: bible.characters[0].id.clone(),
            name: Some("Mira".to_string()),
            role: Some("Traegt das Suchwort".to_string()),
            description: Some(String::new()),
            notes: Some(String::new()),
            title: None,
            body: None,
            note_type: None,
        },
    )
    .unwrap();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "location".to_string(),
            id: bible.locations[0].id.clone(),
            name: Some("Alter Turm".to_string()),
            role: None,
            description: Some("Das Suchwort liegt im Nebel.".to_string()),
            notes: Some(String::new()),
            title: None,
            body: None,
            note_type: None,
        },
    )
    .unwrap();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "item".to_string(),
            id: bible.items[0].id.clone(),
            name: Some("Schluessel".to_string()),
            role: None,
            description: Some(String::new()),
            notes: Some("Suchwort aus Silber.".to_string()),
            title: None,
            body: None,
            note_type: None,
        },
    )
    .unwrap();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "note".to_string(),
            id: bible.notes[0].id.clone(),
            name: None,
            role: None,
            description: None,
            notes: None,
            title: Some("Suchwort-Regel".to_string()),
            body: Some("Alte Regel.".to_string()),
            note_type: Some("lore".to_string()),
        },
    )
    .unwrap();

    let results = search_project_in_connection(&connection, "suchwort", "all").unwrap();

    assert_eq!(results.len(), 5);
    assert_eq!(results[0].kind, "scene");
    assert_eq!(results[1].target_type.as_deref(), Some("character"));
    assert_eq!(results[2].target_type.as_deref(), Some("location"));
    assert_eq!(results[3].target_type.as_deref(), Some("item"));
    assert_eq!(results[4].target_type.as_deref(), Some("note"));
}

#[test]
fn search_project_respects_scope_filters() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id,
            title: "Suchwort Szene".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
            text: String::new(),
        },
    )
    .unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "character".to_string(),
        },
    )
    .unwrap();
    let character_id = read_project_bible(&connection).unwrap().characters[0]
        .id
        .clone();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "character".to_string(),
            id: character_id,
            name: Some("Suchwort Figur".to_string()),
            role: Some(String::new()),
            description: Some(String::new()),
            notes: Some(String::new()),
            title: None,
            body: None,
            note_type: None,
        },
    )
    .unwrap();

    let scene_results = search_project_in_connection(&connection, "Suchwort", "scenes").unwrap();
    let bible_results = search_project_in_connection(&connection, "Suchwort", "bible").unwrap();

    assert_eq!(scene_results.len(), 1);
    assert!(scene_results.iter().all(|result| result.kind == "scene"));
    assert_eq!(bible_results.len(), 1);
    assert!(bible_results.iter().all(|result| result.kind == "bible"));
}

#[test]
fn search_project_ignores_archived_project_bible_cards() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "item".to_string(),
        },
    )
    .unwrap();
    let item_id = read_project_bible(&connection).unwrap().items[0].id.clone();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "item".to_string(),
            id: item_id.clone(),
            name: Some("Suchwort Artefakt".to_string()),
            role: None,
            description: Some(String::new()),
            notes: Some(String::new()),
            title: None,
            body: None,
            note_type: None,
        },
    )
    .unwrap();
    delete_project_bible_card_in_connection(
        &mut connection,
        DeleteProjectBibleCardInput {
            target_type: "item".to_string(),
            id: item_id,
        },
    )
    .unwrap();

    let results = search_project_in_connection(&connection, "Suchwort", "bible").unwrap();

    assert!(results.is_empty());
}

#[test]
fn exports_markdown_with_area_chapter_and_scene_text_only() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id.clone(),
            title: "Interner Szenentitel".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: "Nicht exportieren".to_string(),
            text: "Erster Absatz.\n\nZweiter Absatz.".to_string(),
        },
    )
    .unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "character".to_string(),
        },
    )
    .unwrap();

    let export_path = dir.path().join("manuskript.md");
    export_manuscript_markdown_in_connection(&connection, export_path.to_str().unwrap()).unwrap();
    let exported = fs::read_to_string(export_path).unwrap();

    assert!(exported.contains("# Bereich 1"));
    assert!(exported.contains("## Kapitel 1"));
    assert!(exported.contains("Erster Absatz."));
    assert!(exported.contains("Zweiter Absatz."));
    assert!(!exported.contains("Interner Szenentitel"));
    assert!(!exported.contains("Nicht exportieren"));
    assert!(!exported.contains("Neue Figur"));
}

#[test]
fn exports_markdown_in_manuscript_order_and_skips_archived_scenes() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let initial_tree = read_manuscript_tree(&connection).unwrap();
    let chapter_id = initial_tree.areas[0].chapters[0].id.clone();
    let first_scene = initial_tree.areas[0].chapters[0].scenes[0].clone();
    create_scene_in_connection(
        &mut connection,
        CreateSceneInput {
            chapter_id: Some(chapter_id),
            title: Some("Zweite Szene".to_string()),
            after_scene_id: Some(first_scene.id.clone()),
        },
    )
    .unwrap();
    let second_scene =
        read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[1].clone();

    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: first_scene.id,
            title: "Archiv".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
            text: "Soll verschwinden.".to_string(),
        },
    )
    .unwrap();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: second_scene.id.clone(),
            title: "Bleibt".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
            text: "Bleibt sichtbar.".to_string(),
        },
    )
    .unwrap();
    let archived_scene_id = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes
        [0]
    .id
    .clone();
    delete_scene_in_connection(
        &mut connection,
        DeleteSceneInput {
            id: archived_scene_id,
        },
    )
    .unwrap();

    let exported = build_markdown_export(
        &read_manuscript_tree(&connection).unwrap(),
        &ExportProfileSnapshot::default(),
    );

    assert!(!exported.contains("Soll verschwinden."));
    assert!(exported.contains("Bleibt sichtbar."));
}

#[test]
fn saves_export_profile_and_reloads_it() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Profil"), "Profil")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();
    let profile = ExportProfileSnapshot {
        include_novel_title: true,
        include_area_titles: false,
        include_chapter_titles: true,
        include_scene_titles: true,
        include_scene_text: false,
    };
    write_export_profile(&connection, &profile).unwrap();
    assert_eq!(read_export_profile(&connection).unwrap(), profile);
}

#[test]
fn exports_outline_with_dynamic_markdown_heading_levels() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Gliederung"), "Gliederung")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    let profile = ExportProfileSnapshot {
        include_novel_title: true,
        include_area_titles: false,
        include_chapter_titles: true,
        include_scene_titles: true,
        include_scene_text: false,
    };
    let exported = build_markdown_export(&tree, &profile);
    assert!(exported.contains("# Gliederung"));
    assert!(!exported.contains("Bereich 1"));
    assert!(exported.contains("## Kapitel 1"));
    assert!(exported.contains("### Neue Szene"));
}

#[test]
fn exports_scene_text_without_any_headings() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Nur Text"), "Nur Text")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id,
            title: "Intern".to_string(),
            text: "Nur der Inhalt.".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: "Nicht exportieren".to_string(),
        },
    )
    .unwrap();
    let profile = ExportProfileSnapshot {
        include_novel_title: false,
        include_area_titles: false,
        include_chapter_titles: false,
        include_scene_titles: false,
        include_scene_text: true,
    };
    let exported = build_markdown_export(&read_manuscript_tree(&connection).unwrap(), &profile);
    assert_eq!(exported, "Nur der Inhalt.\n");
}

#[test]
fn preserves_umlauts_and_sharp_s_in_markdown_and_docx() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Übung"), "Grüße aus Köln")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id,
            title: "Überraschung".to_string(),
            text: "Äpfel, Öl und süße Grüße.".to_string(),
            status: "revision".to_string(),
            summary: String::new(),
        },
    )
    .unwrap();
    let profile = ExportProfileSnapshot {
        include_novel_title: true,
        include_area_titles: false,
        include_chapter_titles: false,
        include_scene_titles: true,
        include_scene_text: true,
    };
    let tree = read_manuscript_tree(&connection).unwrap();
    let markdown = build_markdown_export(&tree, &profile);
    let docx_xml = build_docx_document_xml(&tree, &profile);
    assert!(markdown.contains("Grüße aus Köln"));
    assert!(markdown.contains("Äpfel, Öl und süße Grüße."));
    assert!(docx_xml.contains("Überraschung"));
    assert!(docx_xml.contains("Äpfel, Öl und süße Grüße."));
    assert_eq!(tree.areas[0].chapters[0].scenes[0].status, "revision");
}

#[test]
fn exports_docx_outline_with_scene_heading_and_without_scene_text() {
    let dir = tempdir().unwrap();
    let snapshot =
        create_project_file(input(dir.path().join("DOCX Gliederung"), "DOCX Gliederung")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    let profile = ExportProfileSnapshot {
        include_novel_title: false,
        include_area_titles: false,
        include_chapter_titles: false,
        include_scene_titles: true,
        include_scene_text: false,
    };
    let xml = build_docx_document_xml(&tree, &profile);
    assert!(xml.contains("Heading1"));
    assert!(xml.contains("Neue Szene"));
    assert!(!xml.contains("Bereich 1"));
    assert!(!xml.contains("Kapitel 1"));
}

#[test]
fn rejects_export_profile_without_selected_content() {
    let profile = ExportProfileSnapshot {
        include_novel_title: false,
        include_area_titles: false,
        include_chapter_titles: false,
        include_scene_titles: false,
        include_scene_text: false,
    };
    assert_eq!(
        validate_export_profile(&profile).unwrap_err().code,
        "empty_export_profile"
    );
}

#[test]
fn rejects_empty_export_path() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();

    let error = export_manuscript_markdown_in_connection(&connection, "  ").unwrap_err();

    assert_eq!(error.code, "invalid_export_path");
}

#[test]
fn exports_docx_with_required_openxml_files() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id,
            title: "Nicht im Export".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
            text: "Ein schlichter Absatz.".to_string(),
        },
    )
    .unwrap();

    let export_path = dir.path().join("manuskript.docx");
    export_manuscript_docx_in_connection(&connection, export_path.to_str().unwrap()).unwrap();
    let exported = fs::read(&export_path).unwrap();
    assert!(!exported.is_empty());

    let mut archive = zip::ZipArchive::new(Cursor::new(exported)).unwrap();
    assert!(archive.by_name("[Content_Types].xml").is_ok());
    assert!(archive.by_name("_rels/.rels").is_ok());
    let mut document = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut document)
        .unwrap();

    assert!(document.contains("Bereich 1"));
    assert!(document.contains("Kapitel 1"));
    assert!(document.contains("Ein schlichter Absatz."));
    assert!(!document.contains("Nicht im Export"));
}

#[test]
fn migrates_v1_project_to_manuscript_schema() {
    let dir = tempdir().unwrap();
    let path = create_legacy_v1_project(dir.path().join("Altprojekt"), "Altprojekt");

    let opened = open_project_file(path.to_string_lossy().into_owned()).unwrap();
    let connection = Connection::open(opened.path).unwrap();
    let version: i64 = connection
        .query_row(
            "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(tree.novel.title, "Altprojekt");
    assert_eq!(tree.areas.len(), 1);
    assert_eq!(tree.areas[0].title, "Bereich 1");
    assert_eq!(tree.areas[0].chapters.len(), 1);
    assert_eq!(tree.areas[0].chapters[0].scenes.len(), 1);
    assert!(table_exists(&connection, "scene_link"));
    assert!(table_exists(&connection, "ai_suggestion"));
    assert!(table_exists(&connection, "ai_bible_suggestion"));
}

#[test]
fn migrates_v2_project_chapters_into_default_area() {
    let dir = tempdir().unwrap();
    let path = create_legacy_v2_project(dir.path().join("V2Projekt"), "V2Projekt");

    let opened = open_project_file(path.to_string_lossy().into_owned()).unwrap();
    let connection = Connection::open(opened.path).unwrap();
    let version: i64 = connection
        .query_row(
            "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(tree.areas.len(), 1);
    assert_eq!(tree.areas[0].title, "Bereich 1");
    assert_eq!(tree.areas[0].chapters.len(), 2);
    assert_eq!(tree.areas[0].chapters[0].title, "Kapitel A");
    assert_eq!(tree.areas[0].chapters[0].scenes[0].title, "Szene A");
    assert!(table_exists(&connection, "scene_link"));
    assert!(table_exists(&connection, "ai_suggestion"));
    assert!(table_exists(&connection, "ai_bible_suggestion"));
}

#[test]
fn migrates_v3_project_to_project_bible_schema() {
    let dir = tempdir().unwrap();
    let path = create_legacy_v3_project(dir.path().join("V3Projekt"), "V3Projekt");

    let opened = open_project_file(path.to_string_lossy().into_owned()).unwrap();
    let connection = Connection::open(opened.path).unwrap();
    let version: i64 = connection
        .query_row(
            "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(version, SCHEMA_VERSION);
    for table_name in [
        "character",
        "location",
        "item",
        "note",
        "scene_link",
        "ai_suggestion",
        "ai_bible_suggestion",
    ] {
        assert!(table_exists(&connection, table_name));
    }
}

#[test]
fn migrates_v6_project_to_editor_settings_schema() {
    let dir = tempdir().unwrap();
    let path = create_legacy_v6_project(dir.path().join("V6Projekt"), "V6Projekt");

    let opened = open_project_file(path.to_string_lossy().into_owned()).unwrap();
    let connection = Connection::open(opened.path).unwrap();
    let version: i64 = connection
        .query_row(
            "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(version, SCHEMA_VERSION);
    assert!(table_has_column(&connection, "project", "editor_font_family").unwrap());
    assert!(table_has_column(&connection, "project", "editor_font_size").unwrap());
    assert_eq!(opened.editor_font_family, DEFAULT_EDITOR_FONT_FAMILY);
    assert_eq!(opened.editor_font_size, DEFAULT_EDITOR_FONT_SIZE);
}

#[test]
fn creates_updates_and_archives_project_bible_cards() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();

    for target_type in ["character", "location", "item", "note"] {
        create_project_bible_card_in_connection(
            &mut connection,
            CreateProjectBibleCardInput {
                target_type: target_type.to_string(),
            },
        )
        .unwrap();
    }

    let bible = read_project_bible(&connection).unwrap();
    assert_eq!(bible.characters.len(), 1);
    assert_eq!(bible.locations.len(), 1);
    assert_eq!(bible.items.len(), 1);
    assert_eq!(bible.notes.len(), 1);

    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "character".to_string(),
            id: bible.characters[0].id.clone(),
            name: Some("Mira".to_string()),
            role: Some("Hauptfigur".to_string()),
            description: Some("Schreiberin".to_string()),
            notes: Some("Traegt ein Geheimnis.".to_string()),
            title: None,
            body: None,
            note_type: None,
        },
    )
    .unwrap();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "location".to_string(),
            id: bible.locations[0].id.clone(),
            name: Some("Alter Turm".to_string()),
            role: None,
            description: Some("Verbotener Ort".to_string()),
            notes: Some("Nordrand".to_string()),
            title: None,
            body: None,
            note_type: None,
        },
    )
    .unwrap();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "item".to_string(),
            id: bible.items[0].id.clone(),
            name: Some("Schluessel".to_string()),
            role: None,
            description: Some("Öffnet die Kammer".to_string()),
            notes: Some("Silber".to_string()),
            title: None,
            body: None,
            note_type: None,
        },
    )
    .unwrap();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "note".to_string(),
            id: bible.notes[0].id.clone(),
            name: None,
            role: None,
            description: None,
            notes: None,
            title: Some("Schwur-Regel".to_string()),
            body: Some("Ein Schwur bindet bei Vollmond.".to_string()),
            note_type: Some("lore".to_string()),
        },
    )
    .unwrap();

    let bible = read_project_bible(&connection).unwrap();
    assert_eq!(bible.characters[0].name, "Mira");
    assert_eq!(bible.characters[0].role, "Hauptfigur");
    assert_eq!(bible.locations[0].name, "Alter Turm");
    assert_eq!(bible.items[0].name, "Schluessel");
    assert_eq!(bible.notes[0].title, "Schwur-Regel");
    assert_eq!(bible.notes[0].note_type, "lore");

    delete_project_bible_card_in_connection(
        &mut connection,
        DeleteProjectBibleCardInput {
            target_type: "item".to_string(),
            id: bible.items[0].id.clone(),
        },
    )
    .unwrap();
    let bible = read_project_bible(&connection).unwrap();
    assert!(bible.items.is_empty());
}

#[test]
fn replaces_scene_links_and_ignores_archived_targets() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    let chapter_id = tree.areas[0].chapters[0].id.clone();
    let first_scene_id = tree.areas[0].chapters[0].scenes[0].id.clone();

    create_scene_in_connection(
        &mut connection,
        CreateSceneInput {
            chapter_id: Some(chapter_id),
            title: Some("Zweite Szene".to_string()),
            after_scene_id: Some(first_scene_id.clone()),
        },
    )
    .unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "character".to_string(),
        },
    )
    .unwrap();

    let tree = read_manuscript_tree(&connection).unwrap();
    let second_scene_id = tree.areas[0].chapters[0].scenes[1].id.clone();
    let character_id = read_project_bible(&connection).unwrap().characters[0]
        .id
        .clone();
    replace_scene_links_for_card_in_connection(
        &mut connection,
        ReplaceSceneLinksForCardInput {
            target_type: "character".to_string(),
            target_id: character_id.clone(),
            scene_ids: vec![
                first_scene_id.clone(),
                second_scene_id.clone(),
                second_scene_id.clone(),
            ],
        },
    )
    .unwrap();
    let links = read_scene_links_for_card(&connection, "character", &character_id).unwrap();
    assert_eq!(links.len(), 2);

    replace_scene_links_for_card_in_connection(
        &mut connection,
        ReplaceSceneLinksForCardInput {
            target_type: "character".to_string(),
            target_id: character_id.clone(),
            scene_ids: vec![second_scene_id.clone()],
        },
    )
    .unwrap();
    let links = read_scene_links_for_card(&connection, "character", &character_id).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].scene_id, second_scene_id);

    delete_project_bible_card_in_connection(
        &mut connection,
        DeleteProjectBibleCardInput {
            target_type: "character".to_string(),
            id: character_id.clone(),
        },
    )
    .unwrap();
    let links = read_scene_links_for_card(&connection, "character", &character_id).unwrap();
    assert!(links.is_empty());
}

#[test]
fn creates_ai_summary_suggestion_with_provider_response() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id.clone(),
            title: "Turmszene".to_string(),
            text: "Mira betritt den alten Turm und findet eine verschlossene Kammer.".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
        },
    )
    .unwrap();

    let suggestion = summarize_scene_with_provider(
        &connection,
        &SummarizeSceneInput {
            scene_id: scene.id.clone(),
        },
        |request| {
            assert_eq!(request.scene_title, "Turmszene");
            assert!(request.scene_text.contains("verschlossene Kammer"));
            Ok("Mira findet im alten Turm eine verschlossene Kammer.".to_string())
        },
    )
    .unwrap();

    assert_eq!(suggestion.action_type, AI_ACTION_SUMMARIZE_SCENE);
    assert_eq!(suggestion.status, AI_STATUS_DRAFT);
    assert_eq!(
        suggestion.result,
        "Mira findet im alten Turm eine verschlossene Kammer."
    );
}

#[test]
fn accepts_ai_summary_suggestion_into_scene_summary() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id.clone(),
            title: scene.title,
            text: "Mira öffnet die Kammer.".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
        },
    )
    .unwrap();
    let suggestion = summarize_scene_with_provider(
        &connection,
        &SummarizeSceneInput {
            scene_id: scene.id.clone(),
        },
        |_| Ok("Mira öffnet eine verborgene Kammer.".to_string()),
    )
    .unwrap();

    let accepted = accept_ai_suggestion_in_connection(
        &mut connection,
        AiSuggestionInput { id: suggestion.id },
    )
    .unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();

    assert_eq!(accepted.status, AI_STATUS_ACCEPTED);
    assert_eq!(
        tree.areas[0].chapters[0].scenes[0].summary,
        "Mira öffnet eine verborgene Kammer."
    );
}

#[test]
fn rejects_empty_scene_for_ai_summary() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();

    let error = summarize_scene_with_provider(
        &connection,
        &SummarizeSceneInput { scene_id: scene.id },
        |_| Ok("Soll nicht aufgerufen werden.".to_string()),
    )
    .unwrap_err();

    assert_eq!(error.code, "ai_scene_empty");
}

#[test]
fn creates_ai_contradiction_suggestion_with_linked_context_only() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id.clone(),
            title: "Turmszene".to_string(),
            text: "Mira wirft den zerbrochenen Kompass weg und findet ihn spaeter intakt."
                .to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
        },
    )
    .unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "item".to_string(),
        },
    )
    .unwrap();
    let bible = read_project_bible(&connection).unwrap();
    let item_id = bible.items[0].id.clone();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "item".to_string(),
            id: item_id.clone(),
            name: Some("Kompass".to_string()),
            role: None,
            description: Some("Der Kompass ist irreparabel zerbrochen.".to_string()),
            notes: Some("Kann nicht wieder funktionieren.".to_string()),
            title: None,
            body: None,
            note_type: None,
        },
    )
    .unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "location".to_string(),
        },
    )
    .unwrap();
    replace_scene_links_for_card_in_connection(
        &mut connection,
        ReplaceSceneLinksForCardInput {
            target_type: "item".to_string(),
            target_id: item_id,
            scene_ids: vec![scene.id.clone()],
        },
    )
    .unwrap();

    let suggestion = check_scene_contradictions_with_provider(
            &connection,
            &SummarizeSceneInput {
                scene_id: scene.id.clone(),
            },
            |request| {
                assert_eq!(request.scene_title, "Turmszene");
                assert!(request.scene_text.contains("Kompass"));
                assert!(request.context_summary.contains("Kompass"));
                assert!(!request.context_summary.contains("Neuer Ort"));
                Ok("- Kategorie: Gegenstand\n  Befund: Der Kompass ist in der Szene wieder intakt, obwohl er laut Kontext irreparabel zerbrochen ist.".to_string())
            },
        )
        .unwrap();

    assert_eq!(suggestion.action_type, AI_ACTION_CHECK_SCENE_CONTRADICTIONS);
    assert_eq!(suggestion.status, AI_STATUS_DRAFT);
    assert!(suggestion.result.contains("Kompass"));
}

#[test]
fn rejects_ai_contradiction_check_without_context_cards() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id.clone(),
            title: scene.title,
            text: "Mira geht zum Turm.".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
        },
    )
    .unwrap();

    let error = check_scene_contradictions_with_provider(
        &connection,
        &SummarizeSceneInput { scene_id: scene.id },
        |_| Ok("Soll nicht aufgerufen werden.".to_string()),
    )
    .unwrap_err();

    assert_eq!(error.code, "ai_scene_context_missing");
}

#[test]
fn rejects_empty_scene_for_ai_contradiction_check() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();

    let error = check_scene_contradictions_with_provider(
        &connection,
        &SummarizeSceneInput { scene_id: scene.id },
        |_| Ok("Soll nicht aufgerufen werden.".to_string()),
    )
    .unwrap_err();

    assert_eq!(error.code, "ai_scene_empty");
}

#[test]
fn does_not_accept_ai_contradiction_suggestion_into_scene_summary() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
    update_scene_in_connection(
        &mut connection,
        UpdateSceneInput {
            id: scene.id.clone(),
            title: scene.title,
            text: "Mira findet den Kompass.".to_string(),
            status: DEFAULT_SCENE_STATUS.to_string(),
            summary: String::new(),
        },
    )
    .unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "item".to_string(),
        },
    )
    .unwrap();
    let bible = read_project_bible(&connection).unwrap();
    replace_scene_links_for_card_in_connection(
        &mut connection,
        ReplaceSceneLinksForCardInput {
            target_type: "item".to_string(),
            target_id: bible.items[0].id.clone(),
            scene_ids: vec![scene.id.clone()],
        },
    )
    .unwrap();
    let suggestion = check_scene_contradictions_with_provider(
        &connection,
        &SummarizeSceneInput { scene_id: scene.id },
        |_| Ok("Keine konkreten Widersprüche gefunden.".to_string()),
    )
    .unwrap();

    let error = accept_ai_suggestion_in_connection(
        &mut connection,
        AiSuggestionInput { id: suggestion.id },
    )
    .unwrap_err();

    assert_eq!(error.code, "ai_action_not_acceptible");
}

#[test]
fn creates_factions_events_and_validated_relationships() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Netz"), "Netz")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "character".into(),
        },
    )
    .unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "faction".into(),
        },
    )
    .unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "event".into(),
        },
    )
    .unwrap();
    let bible = read_project_bible(&connection).unwrap();
    upsert_bible_relationship_in_connection(
        &mut connection,
        UpsertBibleRelationshipInput {
            id: None,
            source_type: "character".into(),
            source_id: bible.characters[0].id.clone(),
            target_type: "faction".into(),
            target_id: bible.factions[0].id.clone(),
            relationship_type: "member_of".into(),
            description: "Gründungsmitglied".into(),
        },
    )
    .unwrap();
    let bible = read_project_bible(&connection).unwrap();
    assert_eq!(bible.factions.len(), 1);
    assert_eq!(bible.events.len(), 1);
    assert_eq!(bible.relationships[0].description, "Gründungsmitglied");
    let error = upsert_bible_relationship_in_connection(
        &mut connection,
        UpsertBibleRelationshipInput {
            id: None,
            source_type: "character".into(),
            source_id: bible.characters[0].id.clone(),
            target_type: "event".into(),
            target_id: bible.events[0].id.clone(),
            relationship_type: "member_of".into(),
            description: String::new(),
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "bible_relationship_type_invalid");
}

#[test]
fn creates_updates_searches_and_archives_world_system() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Welt"), "Welt")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    create_world_system_in_connection(
        &mut connection,
        CreateWorldSystemInput {
            name: "Aetherkraft".into(),
            category: "magic".into(),
            template_id: "missing".into(),
        },
    )
    .unwrap();
    let system = read_world_systems(&connection).unwrap()[0].clone();
    assert_eq!(system.template_id, "general");
    update_world_system_in_connection(
        &mut connection,
        UpdateWorldSystemInput {
            id: system.id.clone(),
            name: system.name,
            category: "magic".into(),
            template_id: "magic".into(),
            principle: "Aether wird gebunden".into(),
            rules: "Nur bei Nacht".into(),
            capabilities: "Licht formen".into(),
            costs: "Erinnerung".into(),
            limits: "Kein Eisen".into(),
            access: "Ausgebildete".into(),
            distribution: "Selten".into(),
            world_impact: "Tempel kontrollieren Wissen".into(),
            open_questions: "Ursprung".into(),
            notes: "Test".into(),
        },
    )
    .unwrap();
    assert!(
        search_project_bible_in_connection(&connection, "Erinnerung")
            .unwrap()
            .iter()
            .any(|result| result.target_type.as_deref() == Some("world_system"))
    );
    delete_world_system_in_connection(&mut connection, DeleteWorldSystemInput { id: system.id })
        .unwrap();
    assert!(read_world_systems(&connection).unwrap().is_empty());
}

#[test]
fn world_system_uses_scene_links_relationships_and_context() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Kontext"), "Kontext")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    create_world_system_in_connection(
        &mut connection,
        CreateWorldSystemInput {
            name: "Sprungtechnik".into(),
            category: "technology".into(),
            template_id: "technology".into(),
        },
    )
    .unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "faction".into(),
        },
    )
    .unwrap();
    let system_id = read_world_systems(&connection).unwrap()[0].id.clone();
    let bible = read_project_bible(&connection).unwrap();
    let bible_json = serde_json::to_value(&bible).unwrap();
    assert!(bible_json.get("worldSystems").is_none());
    let faction_id = bible.factions[0].id.clone();
    let scene_id: String = connection
        .query_row(
            "SELECT id FROM scene WHERE deleted_at IS NULL LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    replace_scene_links_for_card_in_connection(
        &mut connection,
        ReplaceSceneLinksForCardInput {
            target_type: "world_system".into(),
            target_id: system_id.clone(),
            scene_ids: vec![scene_id.clone()],
        },
    )
    .unwrap();
    upsert_bible_relationship_in_connection(
        &mut connection,
        UpsertBibleRelationshipInput {
            id: None,
            source_type: "faction".into(),
            source_id: faction_id,
            target_type: "world_system".into(),
            target_id: system_id.clone(),
            relationship_type: "uses_system".into(),
            description: String::new(),
        },
    )
    .unwrap();
    assert!(build_scene_project_bible_context(&connection, &scene_id)
        .unwrap()
        .contains("Sprungtechnik"));
    delete_world_system_in_connection(&mut connection, DeleteWorldSystemInput { id: system_id })
        .unwrap();
    let bible = read_project_bible(&connection).unwrap();
    assert!(bible
        .scene_links
        .iter()
        .all(|link| link.target_type != "world_system"));
    assert!(bible.relationships.is_empty());
    assert_eq!(bible.factions.len(), 1);
}

#[test]
fn rejects_invalid_world_system_inputs() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Fehler"), "Fehler")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    assert_eq!(
        create_world_system_in_connection(
            &mut connection,
            CreateWorldSystemInput {
                name: " ".into(),
                category: "magic".into(),
                template_id: "magic".into()
            }
        )
        .unwrap_err()
        .code,
        "world_system_name_empty"
    );
    assert_eq!(
        create_world_system_in_connection(
            &mut connection,
            CreateWorldSystemInput {
                name: "X".into(),
                category: "unknown".into(),
                template_id: "general".into()
            }
        )
        .unwrap_err()
        .code,
        "world_system_category_invalid"
    );
}

#[test]
fn migrates_typed_notes_to_factions_and_events_with_scene_links() {
    let dir = tempdir().unwrap();
    let snapshot =
        create_project_file(input(dir.path().join("Migration11"), "Migration11")).unwrap();
    let connection = Connection::open(&snapshot.path).unwrap();
    let project_id: String = connection
        .query_row("SELECT id FROM project LIMIT 1", [], |row| row.get(0))
        .unwrap();
    let scene_id: String = connection
        .query_row(
            "SELECT id FROM scene WHERE deleted_at IS NULL LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    connection.execute("INSERT INTO note (id, project_id, title, body, note_type, created_at, updated_at, deleted_at) VALUES ('f1',?1,'Rat','Geheime Ziele','faction','1','2',NULL), ('e1',?1,'Krieg','Große Schlacht','event','3','4',NULL)", params![project_id]).unwrap();
    connection.execute("INSERT INTO scene_link (id, scene_id, target_type, target_id, relation, sort_order, created_at, updated_at, deleted_at) VALUES ('sl',?1,'note','f1','assigned',1,'1','1',NULL)", params![scene_id]).unwrap();
    connection.execute_batch("DROP TABLE faction; DROP TABLE event; DROP TABLE bible_relationship; DELETE FROM schema_migrations; INSERT INTO schema_migrations (version, applied_at) VALUES (11, 'legacy');").unwrap();
    drop(connection);
    open_project_file(snapshot.path.clone()).unwrap();
    let migrated = Connection::open(snapshot.path).unwrap();
    let bible = read_project_bible(&migrated).unwrap();
    assert_eq!(bible.factions[0].name, "Rat");
    assert_eq!(bible.events[0].summary, "Große Schlacht");
    assert_eq!(bible.scene_links[0].target_type, "faction");
    assert!(bible
        .notes
        .iter()
        .all(|note| note.id != "f1" && note.id != "e1"));
    ensure_schema_version(&migrated).unwrap();
    assert_eq!(read_project_bible(&migrated).unwrap().factions.len(), 1);
}

#[test]
fn migrates_schema_v12_to_worldbuilding_schema_v13() {
    let dir = tempdir().unwrap();
    let snapshot =
        create_project_file(input(dir.path().join("Migration12"), "Migration12")).unwrap();
    let connection = Connection::open(&snapshot.path).unwrap();
    connection.execute_batch("DROP TABLE world_system; DELETE FROM schema_migrations; INSERT INTO schema_migrations (version, applied_at) VALUES (12, 'legacy');").unwrap();
    drop(connection);
    open_project_file(snapshot.path.clone()).unwrap();
    let migrated = Connection::open(snapshot.path).unwrap();
    assert!(table_exists(&migrated, "world_system"));
    assert!(read_world_systems(&migrated).unwrap().is_empty());
    let version: i64 = migrated
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
}

#[test]
fn migrates_schema_v13_and_repairs_typed_note_relationships() {
    let dir = tempdir().unwrap();
    let snapshot =
        create_project_file(input(dir.path().join("Migration13"), "Migration13")).unwrap();
    let connection = Connection::open(&snapshot.path).unwrap();
    let project_id: String = connection
        .query_row("SELECT id FROM project LIMIT 1", [], |row| row.get(0))
        .unwrap();
    connection.execute("INSERT INTO note (id, project_id, title, body, note_type, created_at, updated_at, deleted_at) VALUES ('f-old',?1,'Rat','','faction','1','2','3'), ('e-old',?1,'Krieg','','event','1','2','3'), ('n-active',?1,'Notiz','','general','1','2',NULL)", params![project_id]).unwrap();
    connection.execute("INSERT INTO faction (id, project_id, name, faction_type, description, goals, notes, created_at, updated_at, deleted_at) VALUES ('f-old',?1,'Rat','','','','','1','2',NULL)", params![project_id]).unwrap();
    connection.execute("INSERT INTO event (id, project_id, title, time_label, summary, outcome, notes, sort_order, created_at, updated_at, deleted_at) VALUES ('e-old',?1,'Krieg','','','','',1,'1','2',NULL)", params![project_id]).unwrap();
    connection.execute_batch("INSERT INTO bible_relationship (id, source_type, source_id, target_type, target_id, relationship_type, description, created_at, updated_at, deleted_at) VALUES
            ('r1','note','f-old','note','e-old','related','alt','10','10',NULL),
            ('r2','note','n-active','faction','f-old','related','normal','11','11',NULL),
            ('r3','event','e-old','faction','f-old','related','korrekt','12','12',NULL);
            DELETE FROM schema_migrations;
            INSERT INTO schema_migrations (version, applied_at) VALUES (13, 'legacy');").unwrap();
    drop(connection);

    open_project_file(snapshot.path.clone()).unwrap();
    let migrated = Connection::open(snapshot.path).unwrap();
    let repaired: (String, String, String, String) = migrated.query_row("SELECT source_type, target_type, created_at, description FROM bible_relationship WHERE id='r1'", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).unwrap();
    assert_eq!(
        repaired,
        ("faction".into(), "event".into(), "10".into(), "alt".into())
    );
    let unchanged: (String, String, String) = migrated
        .query_row(
            "SELECT source_type, target_type, updated_at FROM bible_relationship WHERE id='r2'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(unchanged, ("note".into(), "faction".into(), "11".into()));
    let already_correct: String = migrated
        .query_row(
            "SELECT updated_at FROM bible_relationship WHERE id='r3'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(already_correct, "12");
    ensure_schema_version(&migrated).unwrap();
    assert_eq!(
        migrated
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        SCHEMA_VERSION
    );
}

#[test]
fn migrates_schema_v14_to_structure_planning_schema_v15() {
    let dir = tempdir().unwrap();
    let snapshot =
        create_project_file(input(dir.path().join("Migration14"), "Migration14")).unwrap();
    let connection = Connection::open(&snapshot.path).unwrap();
    connection.execute_batch("DROP TABLE structure_scene_link; DROP TABLE structure_character_link; ALTER TABLE project_structure_beat DROP COLUMN summary; ALTER TABLE project_structure_beat DROP COLUMN notes; DELETE FROM schema_migrations; INSERT INTO schema_migrations (version, applied_at) VALUES (14, 'legacy');").unwrap();
    drop(connection);
    open_project_file(snapshot.path.clone()).unwrap();
    let migrated = Connection::open(snapshot.path).unwrap();
    assert!(table_exists(&migrated, "structure_scene_link"));
    assert!(table_exists(&migrated, "structure_character_link"));
    assert!(table_has_column(&migrated, "project_structure_beat", "summary").unwrap());
    assert!(table_has_column(&migrated, "project_structure_beat", "notes").unwrap());
    assert_eq!(
        migrated
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        SCHEMA_VERSION
    );
}

#[test]
fn migrates_schema_v15_languages_and_scene_statuses_to_v16() {
    let dir = tempdir().unwrap();
    let snapshot =
        create_project_file(input(dir.path().join("Migration15"), "Migration15")).unwrap();
    let connection = Connection::open(&snapshot.path).unwrap();
    connection.execute_batch("DELETE FROM schema_migrations; INSERT INTO schema_migrations (version, applied_at) VALUES (15, 'legacy'); UPDATE project SET default_language='en-GB'; UPDATE scene SET status='Überarbeitung';").unwrap();
    drop(connection);
    open_project_file(snapshot.path.clone()).unwrap();
    let migrated = Connection::open(snapshot.path).unwrap();
    assert_eq!(
        migrated
            .query_row("SELECT default_language FROM project", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "en"
    );
    assert_eq!(
        migrated
            .query_row("SELECT status FROM scene", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "revision"
    );
    assert_eq!(
        migrated
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        SCHEMA_VERSION
    );
}

#[test]
fn creates_ai_bible_suggestion_from_keywords() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();

    let suggestion = generate_bible_card_with_provider(
        &connection,
        GenerateBibleCardInput {
            target_type: "character".to_string(),
            keywords: "Mira, junge Kartografin, sucht den alten Turm".to_string(),
        },
        |request| {
            assert_eq!(request.mode, AI_BIBLE_MODE_GENERATE);
            assert_eq!(request.target_type, "character");
            assert!(request.keywords.contains("Kartografin"));
            Ok(AiBibleCardDraft {
                name: "Mira".to_string(),
                role: "Kartografin".to_string(),
                description: "Junge Kartografin auf der Suche nach dem alten Turm.".to_string(),
                notes: "Misstraut offiziellen Karten.".to_string(),
                ..AiBibleCardDraft::default()
            })
        },
    )
    .unwrap();

    assert_eq!(suggestion.mode, AI_BIBLE_MODE_GENERATE);
    assert_eq!(suggestion.target_type, "character");
    assert_eq!(suggestion.status, AI_STATUS_DRAFT);
    assert_eq!(suggestion.draft.name, "Mira");
}

#[test]
fn accepts_generated_ai_bible_suggestion_as_new_card() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let suggestion = generate_bible_card_with_provider(
        &connection,
        GenerateBibleCardInput {
            target_type: "location".to_string(),
            keywords: "Alter Turm, verlassen, Nordrand".to_string(),
        },
        |_| {
            Ok(AiBibleCardDraft {
                name: "Alter Turm".to_string(),
                description: "Verlassener Turm am Nordrand.".to_string(),
                notes: "Die oberen Stockwerke sind eingestuerzt.".to_string(),
                ..AiBibleCardDraft::default()
            })
        },
    )
    .unwrap();

    let (target_type, target_id) = accept_ai_bible_suggestion_in_connection(
        &mut connection,
        AcceptAiBibleSuggestionInput {
            id: suggestion.id,
            draft: suggestion.draft,
            selected_fields: vec![],
        },
    )
    .unwrap();
    let bible = read_project_bible(&connection).unwrap();

    assert_eq!(target_type, "location");
    assert_eq!(bible.locations.len(), 1);
    assert_eq!(bible.locations[0].id, target_id);
    assert_eq!(bible.locations[0].name, "Alter Turm");
    assert_eq!(
        bible.locations[0].description,
        "Verlassener Turm am Nordrand."
    );
}

#[test]
fn completes_ai_bible_card_without_overwriting_filled_fields() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "character".to_string(),
        },
    )
    .unwrap();
    let character_id = read_project_bible(&connection).unwrap().characters[0]
        .id
        .clone();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "character".to_string(),
            id: character_id.clone(),
            name: Some("Mira".to_string()),
            role: Some("Hauptfigur".to_string()),
            description: None,
            notes: None,
            title: None,
            body: None,
            note_type: None,
        },
    )
    .unwrap();

    let suggestion = complete_bible_card_with_provider(
        &connection,
        CompleteBibleCardInput {
            target_type: "character".to_string(),
            target_id: character_id.clone(),
            keywords: "Kartografin, kennt geheime Wege".to_string(),
        },
        |request| {
            assert_eq!(request.mode, AI_BIBLE_MODE_COMPLETE);
            assert!(request.existing_card_summary.contains("Mira"));
            Ok(AiBibleCardDraft {
                name: "Mira die Kartenkundige".to_string(),
                role: "Spionin".to_string(),
                description: "Kennt geheime Wege durch die Grenzlande.".to_string(),
                notes: "Bewahrt alte Karten in einer Wachstuchrolle auf.".to_string(),
                ..AiBibleCardDraft::default()
            })
        },
    )
    .unwrap();

    accept_ai_bible_suggestion_in_connection(
        &mut connection,
        AcceptAiBibleSuggestionInput {
            id: suggestion.id,
            draft: suggestion.draft,
            selected_fields: vec![],
        },
    )
    .unwrap();
    let character = read_project_bible(&connection).unwrap().characters[0].clone();

    assert_eq!(character.name, "Mira");
    assert_eq!(character.role, "Hauptfigur");
    assert_eq!(
        character.description,
        "Kennt geheime Wege durch die Grenzlande."
    );
    assert_eq!(
        character.notes,
        "Bewahrt alte Karten in einer Wachstuchrolle auf."
    );
}

#[test]
fn edits_only_selected_ai_bible_fields() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "character".to_string(),
        },
    )
    .unwrap();
    let character_id = read_project_bible(&connection).unwrap().characters[0]
        .id
        .clone();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "character".to_string(),
            id: character_id.clone(),
            name: Some("Mira".to_string()),
            role: Some("Kartografin".to_string()),
            description: Some("Kennt die Nordroute.".to_string()),
            notes: Some("Alte Karte.".to_string()),
            title: None,
            body: None,
            note_type: None,
        },
    )
    .unwrap();
    let request = build_ai_bible_edit_request(
        &connection,
        EditBibleCardInput {
            target_type: "character".to_string(),
            target_id: character_id.clone(),
            instruction: "Vertiefe Beschreibung und Notizen".to_string(),
        },
    )
    .unwrap();
    assert_eq!(request.mode, AI_BIBLE_MODE_EDIT);
    let suggestion = insert_ai_bible_suggestion(
        &connection,
        &request,
        AiBibleCardDraft {
            name: "Mira die Weise".to_string(),
            role: "Spionin".to_string(),
            description: "Kennt geheime Wege entlang der Nordroute.".to_string(),
            notes: "Bewahrt eine alte Karte im Mantel.".to_string(),
            ..AiBibleCardDraft::default()
        },
    )
    .unwrap();
    accept_ai_bible_suggestion_in_connection(
        &mut connection,
        AcceptAiBibleSuggestionInput {
            id: suggestion.id,
            draft: suggestion.draft,
            selected_fields: vec!["description".to_string(), "notes".to_string()],
        },
    )
    .unwrap();
    let character = &read_project_bible(&connection).unwrap().characters[0];
    assert_eq!(character.name, "Mira");
    assert_eq!(character.role, "Kartografin");
    assert!(character.description.contains("geheime Wege"));
    assert!(character.notes.contains("Mantel"));
}

#[test]
fn rejects_ai_bible_edit_after_target_changed() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    create_project_bible_card_in_connection(
        &mut connection,
        CreateProjectBibleCardInput {
            target_type: "note".to_string(),
        },
    )
    .unwrap();
    let note_id = read_project_bible(&connection).unwrap().notes[0].id.clone();
    let request = build_ai_bible_edit_request(
        &connection,
        EditBibleCardInput {
            target_type: "note".to_string(),
            target_id: note_id.clone(),
            instruction: "Erweitern".to_string(),
        },
    )
    .unwrap();
    let suggestion = insert_ai_bible_suggestion(
        &connection,
        &request,
        AiBibleCardDraft {
            title: "Neue Chronik".to_string(),
            body: "KI-Inhalt".to_string(),
            note_type: "lore".to_string(),
            ..AiBibleCardDraft::default()
        },
    )
    .unwrap();
    update_project_bible_card_in_connection(
        &mut connection,
        UpdateProjectBibleCardInput {
            target_type: "note".to_string(),
            id: note_id,
            title: Some("Manuell geändert".to_string()),
            body: None,
            note_type: None,
            name: None,
            role: None,
            description: None,
            notes: None,
        },
    )
    .unwrap();
    let error = accept_ai_bible_suggestion_in_connection(
        &mut connection,
        AcceptAiBibleSuggestionInput {
            id: suggestion.id,
            draft: suggestion.draft,
            selected_fields: vec!["body".to_string()],
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "ai_bible_edit_conflict");
}

#[test]
fn rejects_empty_keywords_for_ai_bible() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();

    let error = generate_bible_card_with_provider(
        &connection,
        GenerateBibleCardInput {
            target_type: "note".to_string(),
            keywords: "   ".to_string(),
        },
        |_| Ok(AiBibleCardDraft::default()),
    )
    .unwrap_err();

    assert_eq!(error.code, "ai_bible_keywords_empty");
}

#[test]
fn parses_ai_bible_json_response_from_code_fence() {
    let draft = parse_ai_bible_draft(
            "note",
            "```json\n{\"title\":\"Magieregel\",\"body\":\"Feuer kostet Erinnerung.\",\"noteType\":\"lore\"}\n```",
        )
        .unwrap();

    assert_eq!(draft.title, "Magieregel");
    assert_eq!(draft.body, "Feuer kostet Erinnerung.");
    assert_eq!(draft.note_type, "lore");
}

#[test]
fn parses_ai_faction_and_event_drafts() {
    let faction = parse_ai_bible_draft("faction", r#"{"name":"Der Rat","role":"Geheimbund","description":"Lenkt die Stadt.","body":"Macht sichern.","notes":"Sieben Sitze."}"#).unwrap();
    assert_eq!(faction.name, "Der Rat");
    assert_eq!(faction.body, "Macht sichern.");
    let event = parse_ai_bible_draft("event", r#"{"title":"Der Fall","role":"Dritter Winter","description":"Die Mauer fällt.","body":"Die Stadt wird geöffnet.","notes":"Folgen ungeklärt."}"#).unwrap();
    assert_eq!(event.title, "Der Fall");
    assert_eq!(event.role, "Dritter Winter");
}

#[test]
fn builds_strict_ollama_json_schemas_for_all_bible_card_types() {
    for target_type in ["character", "location", "item", "note", "faction", "event"] {
        let schema = bible_json_schema(target_type);

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"], false);
        assert!(schema["properties"].is_object());
        assert!(schema["required"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
    }

    assert_eq!(
        bible_json_schema("note")["properties"]["noteType"]["enum"],
        serde_json::json!(["general", "research", "lore", "faction", "event", "todo"])
    );
}

#[test]
fn parses_first_ai_bible_json_object_from_qwen_ramble() {
    let draft = parse_ai_bible_draft(
            "location",
            "```json\n{\n  \"name\": \"Feldspalte\",\n  \"description\": \"Ein winziger Ort an der Kueste.\",\n  \"notes\": \"Die Bewohner meiden das Kliff.\",\n  \"notices\": null\n}\n```\n\n```\nDer Ort: Feldspalte\nBeschreibung: Qwen schreibt weiter.\n```\n```json\n{\"name\":\"Zweiter Ort\",\"description\":\"Soll ignoriert werden\",\"notes\":\"\"}\n```",
        )
        .unwrap();

    assert_eq!(draft.name, "Feldspalte");
    assert_eq!(draft.description, "Ein winziger Ort an der Kueste.");
    assert_eq!(draft.notes, "Die Bewohner meiden das Kliff.");
}

#[test]
fn migrates_legacy_ai_settings_to_local_ollama_only() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ai-settings.json");
    fs::write(
        &path,
        r#"{"baseUrl":"https://example.invalid/v1","model":" local-model ","apiKey":"secret"}"#,
    )
    .unwrap();

    let settings = read_ai_settings(&path).unwrap();

    assert_eq!(settings.base_url, DEFAULT_AI_BASE_URL);
    assert_eq!(settings.model, "local-model");
    write_ai_settings(&path, &settings).unwrap();
    assert_eq!(
        fs::read_to_string(path).unwrap(),
        "{\n  \"model\": \"local-model\"\n}"
    );
}

#[test]
fn requires_explicit_ai_model_selection() {
    let settings = AiSettings::default();

    assert_eq!(settings.base_url, DEFAULT_AI_BASE_URL);
    assert!(settings.model.is_empty());
    assert_eq!(serde_json::to_string(&settings).unwrap(), r#"{"model":""}"#);
}

#[test]
fn reports_ai_provider_reachable_from_models_endpoint() {
    let base_url = mock_models_server(200, "{\"data\":[]}");
    let settings = AiSettings {
        base_url,
        model: "test-model".to_string(),
    };

    let status = get_ai_provider_status_for_settings(&settings).unwrap();

    assert_eq!(status.state, AI_PROVIDER_REACHABLE);
    assert!(!status.can_start);
}

#[test]
fn reports_ai_provider_unreachable_without_start_for_custom_endpoint() {
    let settings = AiSettings {
        base_url: "http://127.0.0.1:9/v1".to_string(),
        model: "test-model".to_string(),
    };

    let status = get_ai_provider_status_for_settings(&settings).unwrap();

    assert_eq!(status.state, AI_PROVIDER_UNREACHABLE);
    assert!(!status.can_start);
}

#[test]
fn rejects_empty_text_alternative_selection() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();

    let error = build_text_alternatives_request(
        &connection,
        GenerateTextAlternativesInput {
            scene_id: scene.id,
            selected_text: "   ".to_string(),
            context_before: String::new(),
            context_after: String::new(),
        },
    )
    .unwrap_err();

    assert_eq!(error.code, "empty_text_selection");
}

#[test]
fn parses_text_alternatives_from_json_response() {
    let alternatives =
        parse_text_alternatives_response("```json\n[\"still\", \"leise\", \"verhalten\"]\n```")
            .unwrap();

    assert_eq!(alternatives, vec!["still", "leise", "verhalten"]);
}

#[test]
fn parses_text_alternatives_after_reasoning_block() {
    let alternatives = parse_text_alternatives_response(
            "<reasoning>\nI need to find concise synonyms.\n</reasoning>\n[\"Das Haus blieb in der Obhut seiner Oma.\", \"Seine Oma wuerde das Haus hueten.\"]",
        )
        .unwrap();

    assert_eq!(
        alternatives,
        vec![
            "Das Haus blieb in der Obhut seiner Oma.",
            "Seine Oma wuerde das Haus hueten."
        ]
    );
}

#[test]
fn rejects_reasoning_only_text_alternatives_response() {
    let error = parse_text_alternatives_response(
        "<reasoning>\nI need to find concise synonyms for the marked sentence.\n</reasoning>",
    )
    .unwrap_err();

    assert_eq!(error.code, "ai_empty_response");
}

#[test]
fn generates_text_alternatives_with_mock_provider() {
    let base_url = mock_models_server(
        200,
        r#"{"choices":[{"message":{"content":"[\"grau\", \"trueb\", \"fahl\"]"}}]}"#,
    );
    let settings = AiSettings {
        base_url,
        model: "test-model".to_string(),
    };
    let request = TextAlternativesRequest {
        language: "de".to_string(),
        scene_id: "scene-id".to_string(),
        selected_text: "kalt".to_string(),
        context_before: "Das Meer war".to_string(),
        context_after: "und endlos.".to_string(),
        is_phrase: false,
    };

    let alternatives = call_openai_compatible_text_alternatives(&settings, &request).unwrap();

    assert_eq!(alternatives, vec!["grau", "trueb", "fahl"]);
}

#[test]
fn reports_unreachable_text_alternatives_provider() {
    let settings = AiSettings {
        base_url: "http://127.0.0.1:9/v1".to_string(),
        model: "test-model".to_string(),
    };
    let request = TextAlternativesRequest {
        language: "de".to_string(),
        scene_id: "scene-id".to_string(),
        selected_text: "kalt".to_string(),
        context_before: String::new(),
        context_after: String::new(),
        is_phrase: false,
    };

    let error = call_openai_compatible_text_alternatives(&settings, &request).unwrap_err();

    assert!(!error.message.is_empty());
}

#[test]
fn rejects_empty_text_improvement_selection() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();

    let error = build_improve_text_request(
        &connection,
        ImproveTextInput {
            scene_id: scene.id,
            selected_text: "   ".to_string(),
            context_before: String::new(),
            context_after: String::new(),
            mode: "selection".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(error.code, "empty_text_selection");
}

#[test]
fn rejects_empty_scene_for_text_improvement() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let connection = Connection::open(snapshot.path).unwrap();
    let scene = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();

    let error = build_improve_text_request(
        &connection,
        ImproveTextInput {
            scene_id: scene.id,
            selected_text: String::new(),
            context_before: String::new(),
            context_after: String::new(),
            mode: "scene".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(error.code, "empty_scene_text");
}

#[test]
fn parses_text_improvements_after_reasoning_block() {
    let improvements = parse_text_improvements_response(
            "<think>\nI need to improve this text.\n</think>\n[\"Das Meer lag grau und kalt unter den Klippen.\", \"Grau und kalt lag das Meer unter den Klippen.\"]",
        )
        .unwrap();

    assert_eq!(
        improvements,
        vec![
            "Das Meer lag grau und kalt unter den Klippen.",
            "Grau und kalt lag das Meer unter den Klippen."
        ]
    );
}

#[test]
fn generates_text_improvements_with_mock_provider() {
    let base_url = mock_models_server(
        200,
        r#"{"choices":[{"message":{"content":"[\"Das Meer lag grau und kalt vor ihm.\", \"Vor ihm lag das Meer, grau und kalt.\"]"}}]}"#,
    );
    let settings = AiSettings {
        base_url,
        model: "test-model".to_string(),
    };
    let request = TextImprovementRequest {
        language: "de".to_string(),
        scene_id: "scene-id".to_string(),
        selected_text: "Das Meer. Grau. Kalt.".to_string(),
        context_before: String::new(),
        context_after: "Die Klippen waren sein Zuhause.".to_string(),
        mode: "selection".to_string(),
    };

    let improvements = call_openai_compatible_text_improvements(&settings, &request).unwrap();

    assert_eq!(
        improvements,
        vec![
            "Das Meer lag grau und kalt vor ihm.",
            "Vor ihm lag das Meer, grau und kalt."
        ]
    );
}

#[test]
fn reports_unreachable_text_improvements_provider() {
    let settings = AiSettings {
        base_url: "http://127.0.0.1:9/v1".to_string(),
        model: "test-model".to_string(),
    };
    let request = TextImprovementRequest {
        language: "de".to_string(),
        scene_id: "scene-id".to_string(),
        selected_text: "Das Meer. Grau. Kalt.".to_string(),
        context_before: String::new(),
        context_after: String::new(),
        mode: "selection".to_string(),
    };

    let error = call_openai_compatible_text_improvements(&settings, &request).unwrap_err();

    assert!(!error.message.is_empty());
}

#[test]
fn sends_ai_chat_message_to_openai_compatible_provider() {
    let base_url = mock_models_server(
        200,
        r#"{"choices":[{"message":{"content":"Das ist ein guter Ansatz."}}]}"#,
    );
    let settings = AiSettings {
        base_url,
        model: "test-model".to_string(),
    };
    let input = SendAiChatMessageInput {
        messages: vec![AiChatMessage {
            role: "user".to_string(),
            content: "Wie verbessere ich diese Szene?".to_string(),
        }],
        scene_id: None,
        include_scene_context: false,
        mode: AI_CHAT_MODE_IDEAS.to_string(),
    };

    let response = call_openai_compatible_free_chat(&settings, &input, None, "de").unwrap();

    assert_eq!(response, "Das ist ein guter Ansatz.");
}

#[test]
fn keeps_scene_context_in_the_initial_system_message() {
    let input = SendAiChatMessageInput {
        messages: vec![AiChatMessage {
            role: "user".to_string(),
            content: "Wie geht die Szene weiter?".to_string(),
        }],
        scene_id: Some("scene-id".to_string()),
        include_scene_context: true,
        mode: AI_CHAT_MODE_WRITING.to_string(),
    };

    let messages = build_openai_chat_messages(&input, Some("Atlas 42 scene text."), "de").unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[1]["role"], "user");
    assert!(messages[0]["content"]
        .as_str()
        .unwrap()
        .contains("Atlas 42 scene text."));
}

#[test]
fn uses_british_english_for_the_combined_scene_context_message() {
    let input = SendAiChatMessageInput {
        messages: vec![AiChatMessage {
            role: "user".to_string(),
            content: "Continue the scene.".to_string(),
        }],
        scene_id: Some("scene-id".to_string()),
        include_scene_context: true,
        mode: AI_CHAT_MODE_WRITING.to_string(),
    };

    let messages = build_openai_chat_messages(&input, Some("Atlas 42 scene text."), "en").unwrap();
    let system_content = messages[0]["content"].as_str().unwrap();

    assert_eq!(messages.len(), 2);
    assert!(system_content.contains("British-English writing assistant"));
    assert!(system_content.contains("Current scene context."));
}

#[test]
fn accepts_ai_chat_idea_and_writing_modes() {
    let ideas_input = SendAiChatMessageInput {
        messages: vec![AiChatMessage {
            role: "user".to_string(),
            content: "Wie verbessere ich diese Szene?".to_string(),
        }],
        scene_id: None,
        include_scene_context: false,
        mode: AI_CHAT_MODE_IDEAS.to_string(),
    };
    let writing_input = SendAiChatMessageInput {
        messages: vec![AiChatMessage {
            role: "user".to_string(),
            content: "Erweitere die Szene melancholischer.".to_string(),
        }],
        scene_id: None,
        include_scene_context: false,
        mode: AI_CHAT_MODE_WRITING.to_string(),
    };

    assert_eq!(
        normalize_ai_chat_mode(&ideas_input).unwrap(),
        AI_CHAT_MODE_IDEAS
    );
    assert_eq!(
        normalize_ai_chat_mode(&writing_input).unwrap(),
        AI_CHAT_MODE_WRITING
    );
    assert!(ai_chat_system_prompt(AI_CHAT_MODE_IDEAS, "de").contains("Ideenberater"));
    assert!(ai_chat_system_prompt(AI_CHAT_MODE_WRITING, "de").contains("Schreibassistent"));
    assert!(ai_chat_system_prompt(AI_CHAT_MODE_WRITING, "en").contains("British-English"));
}

#[test]
fn english_ai_prompts_request_british_english() {
    let alternatives = TextAlternativesRequest {
        language: "en".to_string(),
        scene_id: "scene-1".to_string(),
        selected_text: "looked".to_string(),
        context_before: String::new(),
        context_after: String::new(),
        is_phrase: false,
    };
    let improvements = TextImprovementRequest {
        language: "en".to_string(),
        scene_id: "scene-1".to_string(),
        selected_text: "A short scene.".to_string(),
        context_before: String::new(),
        context_after: String::new(),
        mode: "scene".to_string(),
    };
    assert!(build_text_alternatives_prompt(&alternatives).contains("British English"));
    assert!(build_text_improvement_prompt(&improvements).contains("British English"));
    assert!(bible_system_prompt("en", true).contains("British-English"));
    assert!(ai_language_instruction("de").contains("andere Sprache"));
}

#[test]
fn rejects_invalid_ai_chat_mode() {
    let input = SendAiChatMessageInput {
        messages: vec![AiChatMessage {
            role: "user".to_string(),
            content: "Hallo".to_string(),
        }],
        scene_id: None,
        include_scene_context: false,
        mode: "analyse".to_string(),
    };

    let error = normalize_ai_chat_mode(&input).unwrap_err();

    assert_eq!(error.code, "invalid_ai_chat_mode");
}

#[test]
fn rejects_empty_ai_chat_message() {
    let input = SendAiChatMessageInput {
        messages: vec![AiChatMessage {
            role: "user".to_string(),
            content: " ".to_string(),
        }],
        scene_id: None,
        include_scene_context: false,
        mode: AI_CHAT_MODE_IDEAS.to_string(),
    };

    let error = normalize_chat_messages(&input).unwrap_err();

    assert_eq!(error.code, "empty_ai_chat_message");
}

#[test]
fn cleans_ai_chat_stop_tokens() {
    let content = clean_ai_chat_content("Erweiterte Szene.\n<|im_end|>");

    assert_eq!(content, "Erweiterte Szene.");
}

#[test]
fn removes_reasoning_before_an_ai_chat_answer() {
    let content =
        clean_ai_chat_content("<think>Internal analysis.</think>: The atlas remains closed.");

    assert_eq!(content, "The atlas remains closed.");
}

#[test]
fn keeps_the_answer_before_a_trailing_reasoning_block() {
    let content =
        clean_ai_chat_content("The atlas remains closed.\n<think>Internal analysis.</think>");

    assert_eq!(content, "The atlas remains closed.");
}

#[test]
fn reports_unreachable_ai_chat_provider() {
    let settings = AiSettings {
        base_url: "http://127.0.0.1:9/v1".to_string(),
        model: "test-model".to_string(),
    };
    let input = SendAiChatMessageInput {
        messages: vec![AiChatMessage {
            role: "user".to_string(),
            content: "Hallo".to_string(),
        }],
        scene_id: None,
        include_scene_context: false,
        mode: AI_CHAT_MODE_WRITING.to_string(),
    };

    let error = call_openai_compatible_free_chat(&settings, &input, None, "de").unwrap_err();

    assert!(!error.message.is_empty());
}

#[test]
fn does_not_start_non_ollama_provider() {
    let settings = AiSettings {
        base_url: "http://127.0.0.1:9999/v1".to_string(),
        model: "test-model".to_string(),
    };

    let status = start_local_ai_provider_for_settings(&settings).unwrap();

    assert_eq!(status.state, AI_PROVIDER_UNSUPPORTED);
    assert!(!status.can_start);
}

#[test]
fn finds_ollama_executable_in_path_candidates() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("ollama.exe"), "").unwrap();
    let path_env = env::join_paths([dir.path()]).unwrap();

    assert!(command_exists_in_path("ollama", Some(path_env.as_os_str())));
}

#[test]
fn parses_ollama_model_tags() {
    let models = parse_ollama_tags_response(
        r#"{
                "models": [
                    {
                        "name": "qwen3:8b",
                        "modified_at": "2026-06-07T10:00:00Z",
                        "size": 5220000000
                    },
                    {
                        "name": "gemma:12b",
                        "modified_at": "2026-06-06T09:00:00Z",
                        "size": 6870000000
                    }
                ]
            }"#,
    )
    .unwrap();

    assert_eq!(models.len(), 2);
    assert_eq!(models[0].name, "qwen3:8b");
    assert_eq!(models[0].size, 5_220_000_000);
    assert_eq!(models[1].modified_at, "2026-06-06T09:00:00Z");
}

#[test]
fn parses_empty_ollama_model_tags() {
    let models = parse_ollama_tags_response(r#"{"models":[]}"#).unwrap();

    assert!(models.is_empty());
}

#[test]
fn parses_openai_model_list_fallback() {
    let models = parse_openai_models_response(
        r#"{
                "object": "list",
                "data": [
                    { "id": "local-model-a", "created": 1710000000 },
                    { "id": "local-model-b" }
                ]
            }"#,
    )
    .unwrap();

    assert_eq!(models.len(), 2);
    assert_eq!(models[0].name, "local-model-a");
    assert_eq!(models[0].modified_at, "1710000000");
}

#[test]
fn creates_updates_moves_and_deletes_areas() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let first_area_id = read_manuscript_tree(&connection).unwrap().areas[0]
        .id
        .clone();

    create_area_in_connection(
        &mut connection,
        CreateAreaInput {
            title: Some("Akt 2".to_string()),
            after_area_id: Some(first_area_id),
        },
    )
    .unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    let second_area_id = tree.areas[1].id.clone();

    update_area_in_connection(
        &mut connection,
        UpdateAreaInput {
            id: second_area_id.clone(),
            title: "Band 2".to_string(),
            summary: "zweiter Bereich".to_string(),
        },
    )
    .unwrap();
    move_area_in_connection(
        &mut connection,
        MoveAreaInput {
            area_id: second_area_id.clone(),
            target_index: 0,
        },
    )
    .unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    assert_eq!(tree.areas[0].title, "Band 2");
    assert_eq!(tree.areas[0].summary, "zweiter Bereich");

    delete_area_in_connection(&mut connection, DeleteAreaInput { id: second_area_id }).unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    assert_eq!(tree.areas.len(), 1);
    assert_eq!(tree.areas[0].title, "Bereich 1");
}

#[test]
fn moves_chapters_between_areas() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    let first_area_id = tree.areas[0].id.clone();
    let chapter_id = tree.areas[0].chapters[0].id.clone();

    create_area_in_connection(
        &mut connection,
        CreateAreaInput {
            title: Some("Bereich 2".to_string()),
            after_area_id: Some(first_area_id),
        },
    )
    .unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    let second_area_id = tree.areas[1].id.clone();

    move_chapter_in_connection(
        &mut connection,
        MoveChapterInput {
            chapter_id,
            target_area_id: second_area_id,
            target_index: 0,
        },
    )
    .unwrap();

    let tree = read_manuscript_tree(&connection).unwrap();
    assert!(tree.areas[0].chapters.is_empty());
    assert_eq!(tree.areas[1].chapters.len(), 1);
    assert_eq!(tree.areas[1].chapters[0].title, "Kapitel 1");
}

#[test]
fn creates_chapters_and_scenes_in_requested_positions() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let initial_tree = read_manuscript_tree(&connection).unwrap();
    let first_area_id = initial_tree.areas[0].id.clone();
    let first_chapter_id = initial_tree.areas[0].chapters[0].id.clone();
    let first_scene_id = initial_tree.areas[0].chapters[0].scenes[0].id.clone();

    create_chapter_in_connection(
        &mut connection,
        CreateChapterInput {
            area_id: Some(first_area_id),
            title: Some("Kapitel 2".to_string()),
            after_chapter_id: Some(first_chapter_id.clone()),
        },
    )
    .unwrap();
    create_scene_in_connection(
        &mut connection,
        CreateSceneInput {
            chapter_id: Some(first_chapter_id),
            title: Some("Zweite Szene".to_string()),
            after_scene_id: Some(first_scene_id),
        },
    )
    .unwrap();

    let tree = read_manuscript_tree(&connection).unwrap();
    assert_eq!(tree.areas[0].chapters.len(), 2);
    assert_eq!(tree.areas[0].chapters[1].title, "Kapitel 2");
    assert_eq!(tree.areas[0].chapters[0].scenes.len(), 2);
    assert_eq!(tree.areas[0].chapters[0].scenes[1].title, "Zweite Szene");
}

#[test]
fn moves_chapters_and_scenes_across_chapters() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let initial_tree = read_manuscript_tree(&connection).unwrap();
    let first_area_id = initial_tree.areas[0].id.clone();
    let first_chapter_id = initial_tree.areas[0].chapters[0].id.clone();

    create_chapter_in_connection(
        &mut connection,
        CreateChapterInput {
            area_id: Some(first_area_id.clone()),
            title: Some("Kapitel 2".to_string()),
            after_chapter_id: Some(first_chapter_id.clone()),
        },
    )
    .unwrap();
    create_chapter_in_connection(
        &mut connection,
        CreateChapterInput {
            area_id: Some(first_area_id.clone()),
            title: Some("Kapitel 3".to_string()),
            after_chapter_id: None,
        },
    )
    .unwrap();

    let tree = read_manuscript_tree(&connection).unwrap();
    let chapter_2_id = tree.areas[0].chapters[1].id.clone();
    let chapter_3_id = tree.areas[0].chapters[2].id.clone();
    let scene_id = tree.areas[0].chapters[0].scenes[0].id.clone();

    move_chapter_in_connection(
        &mut connection,
        MoveChapterInput {
            chapter_id: chapter_3_id,
            target_area_id: first_area_id,
            target_index: 0,
        },
    )
    .unwrap();
    move_scene_in_connection(
        &mut connection,
        MoveSceneInput {
            scene_id,
            target_chapter_id: chapter_2_id,
            target_index: 0,
        },
    )
    .unwrap();

    let tree = read_manuscript_tree(&connection).unwrap();
    assert_eq!(tree.areas[0].chapters[0].title, "Kapitel 3");
    assert!(tree.areas[0].chapters[0].scenes.is_empty());
    assert_eq!(tree.areas[0].chapters[2].title, "Kapitel 2");
    assert_eq!(tree.areas[0].chapters[2].scenes.len(), 1);
}

#[test]
fn soft_deletes_scenes_and_chapters() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    let chapter_id = tree.areas[0].chapters[0].id.clone();
    let scene_id = tree.areas[0].chapters[0].scenes[0].id.clone();
    connection.execute("INSERT INTO scene_link (id, scene_id, target_type, target_id, relation, sort_order, created_at, updated_at, deleted_at) VALUES ('scene-link',?1,'note','target','assigned',0,'1','1',NULL)", params![scene_id]).unwrap();

    create_scene_in_connection(
        &mut connection,
        CreateSceneInput {
            chapter_id: Some(chapter_id.clone()),
            title: Some("Bleibt kurz".to_string()),
            after_scene_id: None,
        },
    )
    .unwrap();
    delete_scene_in_connection(
        &mut connection,
        DeleteSceneInput {
            id: scene_id.clone(),
        },
    )
    .unwrap();

    let tree = read_manuscript_tree(&connection).unwrap();
    assert_eq!(tree.areas[0].chapters[0].scenes.len(), 1);
    assert_ne!(tree.areas[0].chapters[0].scenes[0].id, scene_id);
    assert!(connection
        .query_row(
            "SELECT deleted_at IS NOT NULL FROM scene_link WHERE id='scene-link'",
            [],
            |row| row.get::<_, bool>(0)
        )
        .unwrap());

    let remaining_scene_id = tree.areas[0].chapters[0].scenes[0].id.clone();
    connection.execute("INSERT INTO scene_link (id, scene_id, target_type, target_id, relation, sort_order, created_at, updated_at, deleted_at) VALUES ('chapter-link',?1,'note','target','assigned',0,'1','1',NULL)", params![remaining_scene_id]).unwrap();

    delete_chapter_in_connection(&mut connection, DeleteChapterInput { id: chapter_id }).unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    assert!(tree.areas[0].chapters.is_empty());
    assert!(connection
        .query_row(
            "SELECT deleted_at IS NOT NULL FROM scene_link WHERE id='chapter-link'",
            [],
            |row| row.get::<_, bool>(0)
        )
        .unwrap());
}

#[test]
fn soft_deleting_area_archives_descendant_scene_links() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("AreaLinks"), "AreaLinks")).unwrap();
    let mut connection = Connection::open(snapshot.path).unwrap();
    let tree = read_manuscript_tree(&connection).unwrap();
    let area_id = tree.areas[0].id.clone();
    let scene_id = tree.areas[0].chapters[0].scenes[0].id.clone();
    connection.execute("INSERT INTO scene_link (id, scene_id, target_type, target_id, relation, sort_order, created_at, updated_at, deleted_at) VALUES ('area-link',?1,'note','target','assigned',0,'1','1',NULL)", params![scene_id]).unwrap();

    delete_area_in_connection(&mut connection, DeleteAreaInput { id: area_id }).unwrap();

    assert!(read_manuscript_tree(&connection).unwrap().areas.is_empty());
    assert!(connection
        .query_row(
            "SELECT deleted_at IS NOT NULL FROM scene_link WHERE id='area-link'",
            [],
            |row| row.get::<_, bool>(0)
        )
        .unwrap());
}

#[test]
fn world_system_template_errors_fall_back_without_blocking_valid_templates() {
    let templates = parse_world_system_templates(
        &[
            "not json",
            r#"{"id":"magic","name":"Magie","category":"magic","guidingQuestions":["Frage"]}"#,
        ],
        "de",
    );
    assert!(templates.iter().any(|template| template.id == "magic"));
    assert!(templates.iter().any(|template| template.id == "general"));

    let fallback_only = parse_world_system_templates(&["broken"], "en");
    assert_eq!(fallback_only.len(), 1);
    assert_eq!(fallback_only[0].id, "general");
    assert_eq!(fallback_only[0].name, "General world system");
    assert!(!fallback_only[0].guiding_questions.is_empty());
}

#[test]
fn rejects_invalid_project_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("broken.autorenprojekt");
    fs::write(&path, "not sqlite").unwrap();

    let error = open_project_file(path.to_string_lossy().into_owned()).unwrap_err();

    assert_eq!(error.code, "invalid_project_file");
}

#[test]
fn migrates_schema_v7_to_current_schema() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Migration"), "Migration")).unwrap();
    let connection = Connection::open(&snapshot.path).unwrap();
    connection
        .execute_batch(
            "
                DROP TABLE project_structure_beat;
                DROP TABLE project_structure_act;
                DROP TABLE project_structure;
                DELETE FROM schema_migrations;
                INSERT INTO schema_migrations (version, applied_at) VALUES (7, 'legacy');
                ",
        )
        .unwrap();
    drop(connection);

    open_project_file(snapshot.path.clone()).unwrap();
    let migrated = Connection::open(snapshot.path).unwrap();
    let version: i64 = migrated
        .query_row(
            "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let table_count: i64 = migrated
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'project_structure_beat'",
                [],
                |row| row.get(0),
            )
            .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(table_count, 1);
}

#[test]
fn migrates_schema_v8_structure_links_to_v9() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Migration8"), "Migration8")).unwrap();
    let connection = Connection::open(&snapshot.path).unwrap();
    connection.execute_batch("ALTER TABLE project_structure_act DROP COLUMN manuscript_area_id; ALTER TABLE project_structure_beat DROP COLUMN manuscript_chapter_id; DELETE FROM schema_migrations; INSERT INTO schema_migrations (version, applied_at) VALUES (8, 'legacy');").unwrap();
    drop(connection);

    open_project_file(snapshot.path.clone()).unwrap();
    let migrated = Connection::open(snapshot.path).unwrap();
    assert!(table_has_column(&migrated, "project_structure_act", "manuscript_area_id").unwrap());
    assert!(
        table_has_column(&migrated, "project_structure_beat", "manuscript_chapter_id").unwrap()
    );
}

#[test]
fn migrates_schema_v9_to_export_profile_schema_v10() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file(input(dir.path().join("Migration9"), "Migration9")).unwrap();
    let connection = Connection::open(&snapshot.path).unwrap();
    connection.execute_batch("DROP TABLE export_profile; DELETE FROM schema_migrations; INSERT INTO schema_migrations (version, applied_at) VALUES (9, 'legacy');").unwrap();
    drop(connection);
    open_project_file(snapshot.path.clone()).unwrap();
    let migrated = Connection::open(snapshot.path).unwrap();
    assert!(table_exists(&migrated, "export_profile"));
    assert_eq!(
        read_export_profile(&migrated).unwrap(),
        ExportProfileSnapshot::default()
    );
}

#[test]
fn migrates_schema_v10_to_ai_bible_edit_schema_v11() {
    let dir = tempdir().unwrap();
    let snapshot =
        create_project_file(input(dir.path().join("Migration10"), "Migration10")).unwrap();
    let connection = Connection::open(&snapshot.path).unwrap();
    connection.execute_batch("ALTER TABLE ai_bible_suggestion DROP COLUMN source_json; ALTER TABLE ai_bible_suggestion DROP COLUMN target_updated_at; DELETE FROM schema_migrations; INSERT INTO schema_migrations (version, applied_at) VALUES (10, 'legacy');").unwrap();
    drop(connection);
    open_project_file(snapshot.path.clone()).unwrap();
    let migrated = Connection::open(snapshot.path).unwrap();
    assert!(table_has_column(&migrated, "ai_bible_suggestion", "source_json").unwrap());
    assert!(table_has_column(&migrated, "ai_bible_suggestion", "target_updated_at").unwrap());
}

#[test]
fn project_defaults_fall_back_to_freestyle_when_missing_or_invalid() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("project-defaults.json");
    assert_eq!(
        read_project_defaults(&path)
            .unwrap()
            .story_structure_template_id,
        "freestyle"
    );
    fs::write(&path, "not json").unwrap();
    assert_eq!(
        read_project_defaults(&path)
            .unwrap()
            .story_structure_template_id,
        "freestyle"
    );
    fs::write(&path, r#"{"storyStructureTemplateId":"removed-template"}"#).unwrap();
    assert_eq!(
        read_project_defaults(&path)
            .unwrap()
            .story_structure_template_id,
        "freestyle"
    );
}

#[test]
fn creates_new_project_with_freestyle_structure_and_scaffold() {
    let dir = tempdir().unwrap();
    let snapshot = create_project_file_with_template(
        input(dir.path().join("Freies Projekt"), "Freies Projekt"),
        "freestyle",
    )
    .unwrap();
    let connection = Connection::open(snapshot.path).unwrap();
    let template_id: String = connection
        .query_row(
            "SELECT template_id FROM project_structure WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let area_title: String = connection
        .query_row(
            "SELECT title FROM manuscript_area WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let chapter_title: String = connection
        .query_row(
            "SELECT title FROM chapter WHERE deleted_at IS NULL",
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
    assert_eq!(template_id, "freestyle");
    assert_eq!(area_title, "Freier Bereich");
    assert_eq!(chapter_title, "Freier Handlungspunkt");
    assert_eq!(scene_count, 1);
}

#[test]
fn changed_default_only_affects_projects_created_afterward() {
    let dir = tempdir().unwrap();
    let first =
        create_project_file_with_template(input(dir.path().join("Erstes"), "Erstes"), "freestyle")
            .unwrap();
    let defaults_path = dir.path().join("project-defaults.json");
    write_project_defaults(&defaults_path, "three-act").unwrap();
    let defaults = read_project_defaults(&defaults_path).unwrap();
    let second = create_project_file_with_template(
        input(dir.path().join("Zweites"), "Zweites"),
        &defaults.story_structure_template_id,
    )
    .unwrap();
    let first_connection = Connection::open(first.path).unwrap();
    let second_connection = Connection::open(second.path).unwrap();
    let first_template: String = first_connection
        .query_row(
            "SELECT template_id FROM project_structure WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let second_template: String = second_connection
        .query_row(
            "SELECT template_id FROM project_structure WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(first_template, "freestyle");
    assert_eq!(second_template, "three-act");
}

#[test]
fn removes_project_file_when_initial_template_application_fails() {
    let dir = tempdir().unwrap();
    let error = create_project_file_with_template(
        input(dir.path().join("Fehlerhaft"), "Fehlerhaft"),
        "unknown-template",
    )
    .unwrap_err();
    assert_eq!(error.code, "unknown_structure_template");
    assert!(!dir.path().join("Fehlerhaft.autorenprojekt").exists());
}

#[test]
fn stores_recent_projects_sorted_and_limited() {
    let dir = tempdir().unwrap();
    let recent_path = dir.path().join("recent-projects.json");

    for index in 0..12 {
        let project_path = dir.path().join(format!("project-{index}"));
        let snapshot =
            create_project_file(input(project_path, &format!("Project {index}"))).unwrap();
        touch_recent_project(&recent_path, &snapshot).unwrap();
    }

    let recent_projects = read_recent_projects(&recent_path).unwrap();

    assert_eq!(recent_projects.len(), RECENT_PROJECTS_LIMIT);
    assert_eq!(recent_projects[0].name, "Project 11");
    assert_eq!(recent_projects[9].name, "Project 2");
}
