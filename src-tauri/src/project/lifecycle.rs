use super::*;

pub fn recent_projects_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app.path().app_config_dir()?.join(RECENT_PROJECTS_FILE))
}

pub fn ai_settings_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app.path().app_config_dir()?.join(AI_SETTINGS_FILE))
}

pub fn project_defaults_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app.path().app_config_dir()?.join(PROJECT_DEFAULTS_FILE))
}

#[tauri::command]
pub fn create_project(
    input: CreateProjectInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<ProjectSnapshot, AppError> {
    let defaults = read_project_defaults(&project_defaults_path(&app)?)?;
    let snapshot = create_project_file_with_template(input, &defaults.story_structure_template_id)?;
    touch_recent_project(&recent_projects_path(&app)?, &snapshot)?;
    set_active_project(&state, snapshot.clone())?;

    Ok(snapshot)
}

#[tauri::command]
pub fn get_project_defaults(app: AppHandle) -> Result<ProjectDefaultsSnapshot, AppError> {
    read_project_defaults(&project_defaults_path(&app)?)
}

#[tauri::command]
pub fn update_project_defaults(
    input: UpdateProjectDefaultsInput,
    app: AppHandle,
) -> Result<ProjectDefaultsSnapshot, AppError> {
    let snapshot = project_defaults_snapshot(&input.story_structure_template_id)?;
    write_project_defaults(
        &project_defaults_path(&app)?,
        &snapshot.story_structure_template_id,
    )?;
    Ok(snapshot)
}

#[tauri::command]
pub fn open_project(
    path: String,
    app: AppHandle,
    state: State<AppState>,
) -> Result<ProjectSnapshot, AppError> {
    let snapshot = open_project_file(path)?;
    touch_recent_project(&recent_projects_path(&app)?, &snapshot)?;
    set_active_project(&state, snapshot.clone())?;

    Ok(snapshot)
}

#[tauri::command]
pub fn update_project(
    input: UpdateProjectInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<ProjectSnapshot, AppError> {
    let recent_projects_path = recent_projects_path(&app)?;
    let mut active_project = state.active_project.lock().map_err(|_| {
        AppError::new(
            "state_lock_failed",
            "Der aktive Projektzustand konnte nicht gesperrt werden.",
        )
    })?;
    let session = active_project.as_mut().ok_or_else(|| {
        AppError::new("no_active_project", "Es ist kein aktives Projekt geöffnet.")
    })?;
    let path = PathBuf::from(&session._snapshot.path);

    update_project_in_connection(&session.connection, input)?;
    let snapshot = read_project_snapshot(&session.connection, &path)?;
    session._snapshot = snapshot.clone();
    touch_recent_project(&recent_projects_path, &snapshot)?;

    Ok(snapshot)
}

#[tauri::command]
pub fn save_project_copy_as(
    input: SaveProjectCopyAsInput,
    state: State<AppState>,
) -> Result<String, AppError> {
    let mut active_project = state.active_project.lock().map_err(|_| {
        AppError::new(
            "state_lock_failed",
            "Der aktive Projektzustand konnte nicht gesperrt werden.",
        )
    })?;
    let session = active_project.as_mut().ok_or_else(|| {
        AppError::new("no_active_project", "Es ist kein aktives Projekt geöffnet.")
    })?;

    session
        .connection
        .execute_batch("PRAGMA wal_checkpoint(FULL);")?;
    let target = save_project_copy_as_file(&session._snapshot.path, &input.path)?;

    Ok(target.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn list_recent_projects(app: AppHandle) -> Result<Vec<RecentProject>, AppError> {
    read_recent_projects(&recent_projects_path(&app)?)
}

#[tauri::command]
pub fn remove_recent_project(path: String, app: AppHandle) -> Result<(), AppError> {
    remove_recent_project_at(&recent_projects_path(&app)?, &path)
}

#[tauri::command]

pub(crate) fn set_active_project(
    state: &State<AppState>,
    snapshot: ProjectSnapshot,
) -> Result<(), AppError> {
    let connection = Connection::open(&snapshot.path)?;
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    let mut active_project = state.active_project.lock().map_err(|_| {
        AppError::new(
            "state_lock_failed",
            "Der aktive Projektzustand konnte nicht gesperrt werden.",
        )
    })?;

    *active_project = Some(ProjectSession {
        _snapshot: snapshot,
        connection,
    });

    Ok(())
}

pub(crate) fn with_active_connection<T>(
    state: &State<AppState>,
    action: impl FnOnce(&mut Connection) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let mut active_project = state.active_project.lock().map_err(|_| {
        AppError::new(
            "state_lock_failed",
            "Der aktive Projektzustand konnte nicht gesperrt werden.",
        )
    })?;
    let session = active_project.as_mut().ok_or_else(|| {
        AppError::new("no_active_project", "Es ist kein aktives Projekt geöffnet.")
    })?;

    action(&mut session.connection)
}

#[cfg(test)]
pub fn create_project_file(input: CreateProjectInput) -> Result<ProjectSnapshot, AppError> {
    create_project_file_without_template(input)
}

pub(crate) fn create_project_file_without_template(
    input: CreateProjectInput,
) -> Result<ProjectSnapshot, AppError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::new(
            "invalid_project_name",
            "Der Projektname darf nicht leer sein.",
        ));
    }

    let default_language = validate_project_language(&input.default_language)?;

    let path = normalize_project_path(&input.path)?;
    if path.exists() {
        return Err(AppError::new(
            "project_file_exists",
            "An diesem Speicherort existiert bereits eine Projektdatei.",
        ));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let connection = Connection::open(&path)?;
    initialize_project_schema(
        &connection,
        name,
        input.description.as_deref().unwrap_or("").trim(),
        default_language,
    )?;

    read_project_snapshot(&connection, &path)
}

pub fn create_project_file_with_template(
    input: CreateProjectInput,
    template_id: &str,
) -> Result<ProjectSnapshot, AppError> {
    let path = normalize_project_path(&input.path)?;
    let _snapshot = create_project_file_without_template(input)?;
    let result = (|| {
        let mut connection = Connection::open(&path)?;
        crate::story_structure::apply_initial_template(&mut connection, template_id)?;
        read_project_snapshot(&connection, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&path);
    }
    result
}

pub fn open_project_file(path: String) -> Result<ProjectSnapshot, AppError> {
    let path = normalize_existing_project_path(&path)?;
    let connection = Connection::open(&path).map_err(map_invalid_project_file)?;

    read_project_snapshot(&connection, &path).map_err(map_invalid_project_error)
}

pub fn save_project_copy_as_file(
    source_path: &str,
    target_path: &str,
) -> Result<PathBuf, AppError> {
    let source = normalize_existing_project_path(source_path)?;
    let target = normalize_project_path(target_path)?;

    if same_project_path(&source, &target) {
        return Err(AppError::new(
            "same_project_copy_path",
            "Die Backup-Datei darf nicht dieselbe Datei wie das aktive Projekt sein.",
        ));
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(&source, &target)?;

    Ok(target)
}

pub(crate) fn update_project_in_connection(
    connection: &Connection,
    input: UpdateProjectInput,
) -> Result<(), AppError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::new(
            "invalid_project_name",
            "Der Projektname darf nicht leer sein.",
        ));
    }

    let default_language = validate_project_language(&input.default_language)?;
    let stored_language: String = connection.query_row(
        "SELECT default_language FROM project WHERE deleted_at IS NULL LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    if default_language != stored_language {
        return Err(AppError::new(
            "project_language_locked",
            "Die Projektsprache kann nach der Erstellung nicht geändert werden.",
        ));
    }

    let editor_font_family = validate_editor_font_family(&input.editor_font_family)?;
    let editor_font_size = validate_editor_font_size(input.editor_font_size)?;

    let changed = connection.execute(
        "
        UPDATE project
        SET name = ?1,
            description = ?2,
            default_language = ?3,
            editor_font_family = ?4,
            editor_font_size = ?5,
            updated_at = ?6
        WHERE deleted_at IS NULL
        ",
        params![
            name,
            input.description.trim(),
            default_language,
            editor_font_family,
            editor_font_size,
            timestamp()
        ],
    )?;

    if changed == 0 {
        return Err(AppError::new(
            "project_missing",
            "Das aktive Projekt wurde nicht gefunden.",
        ));
    }

    Ok(())
}

pub(crate) fn normalize_editor_font_family(font_family: &str) -> String {
    match font_family.trim() {
        "serif" | "sans" | "monospace" => font_family.trim().to_string(),
        _ => DEFAULT_EDITOR_FONT_FAMILY.to_string(),
    }
}

pub(crate) fn normalize_editor_font_size(font_size: i64) -> i64 {
    if (16..=24).contains(&font_size) {
        font_size
    } else {
        DEFAULT_EDITOR_FONT_SIZE
    }
}

pub(crate) fn validate_editor_font_family(font_family: &str) -> Result<String, AppError> {
    let font_family = font_family.trim();
    if matches!(font_family, "serif" | "sans" | "monospace") {
        return Ok(font_family.to_string());
    }

    Err(AppError::new(
        "invalid_editor_font_family",
        "Die Editor-Schriftart ist ungültig.",
    ))
}

pub(crate) fn validate_editor_font_size(font_size: i64) -> Result<i64, AppError> {
    if (16..=24).contains(&font_size) {
        return Ok(font_size);
    }

    Err(AppError::new(
        "invalid_editor_font_size",
        "Die Editor-Schriftgröße muss zwischen 16 und 24 liegen.",
    ))
}

#[cfg(test)]
pub fn update_project_file(
    path: String,
    input: UpdateProjectInput,
) -> Result<ProjectSnapshot, AppError> {
    let path = normalize_existing_project_path(&path)?;
    let connection = Connection::open(&path).map_err(map_invalid_project_file)?;
    update_project_in_connection(&connection, input)?;
    read_project_snapshot(&connection, &path).map_err(map_invalid_project_error)
}

pub fn read_recent_projects(path: &Path) -> Result<Vec<RecentProject>, AppError> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    Ok(serde_json::from_str(&content)?)
}

pub fn touch_recent_project(path: &Path, project: &ProjectSnapshot) -> Result<(), AppError> {
    let mut recent_projects = read_recent_projects(path).unwrap_or_default();
    recent_projects.retain(|recent| recent.path != project.path);
    recent_projects.insert(
        0,
        RecentProject {
            name: project.name.clone(),
            path: project.path.clone(),
            storage_kind: project.storage_kind.clone(),
            last_opened_at: timestamp(),
        },
    );
    recent_projects.truncate(RECENT_PROJECTS_LIMIT);
    write_recent_projects(path, &recent_projects)
}

pub fn remove_recent_project_at(path: &Path, project_path: &str) -> Result<(), AppError> {
    let mut recent_projects = read_recent_projects(path)?;
    recent_projects.retain(|recent| recent.path != project_path);
    write_recent_projects(path, &recent_projects)
}

pub(crate) fn write_recent_projects(
    path: &Path,
    recent_projects: &[RecentProject],
) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(recent_projects)?;
    fs::write(path, content)?;

    Ok(())
}

pub fn read_ai_settings(path: &Path) -> Result<AiSettings, AppError> {
    if !path.exists() {
        return Ok(AiSettings::default());
    }

    let content = fs::read_to_string(path)?;
    if content.trim().is_empty() {
        return Ok(AiSettings::default());
    }

    // Unknown legacy fields such as `baseUrl` and `apiKey` are intentionally
    // ignored. Public builds only connect to the local Ollama loopback URL.
    let settings: AiSettings = serde_json::from_str(&content)?;
    Ok(AiSettings {
        base_url: DEFAULT_AI_BASE_URL.to_string(),
        model: normalize_model_name(&settings.model),
    })
}

pub(crate) fn write_ai_settings(path: &Path, settings: &AiSettings) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(settings)?;
    fs::write(path, content)?;

    Ok(())
}

pub fn read_project_defaults(path: &Path) -> Result<ProjectDefaultsSnapshot, AppError> {
    let stored = if !path.exists() {
        StoredProjectDefaults::default()
    } else {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).unwrap_or_default()
    };
    project_defaults_snapshot(&stored.story_structure_template_id)
        .or_else(|_| project_defaults_snapshot(DEFAULT_STORY_STRUCTURE_TEMPLATE))
}

pub(crate) fn project_defaults_snapshot(
    template_id: &str,
) -> Result<ProjectDefaultsSnapshot, AppError> {
    let template_name =
        crate::story_structure::template_display_name(template_id)?.ok_or_else(|| {
            AppError::new(
                "unknown_structure_template",
                "Diese Strukturvorlage ist nicht verfügbar.",
            )
        })?;
    Ok(ProjectDefaultsSnapshot {
        story_structure_template_id: template_id.to_string(),
        story_structure_template_name: template_name,
    })
}

pub(crate) fn write_project_defaults(path: &Path, template_id: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(&StoredProjectDefaults {
        story_structure_template_id: template_id.to_string(),
    })?;
    fs::write(path, content)?;
    Ok(())
}

pub(crate) fn read_active_project_language(connection: &Connection) -> Result<String, AppError> {
    connection.query_row("SELECT default_language FROM project WHERE deleted_at IS NULL ORDER BY created_at LIMIT 1", [], |row| row.get(0)).map_err(AppError::storage)
}

pub(crate) fn excerpt(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

pub(crate) fn normalize_project_path(path: &str) -> Result<PathBuf, AppError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "invalid_project_path",
            "Der Projektpfad darf nicht leer sein.",
        ));
    }

    let mut path = PathBuf::from(trimmed);
    if path.extension().is_none() {
        path.set_extension(PROJECT_EXTENSION);
    }

    Ok(path)
}

pub(crate) fn absolute_project_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    env::current_dir()
        .map(|current_dir| current_dir.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn same_project_path(left: &Path, right: &Path) -> bool {
    let left = left
        .canonicalize()
        .unwrap_or_else(|_| absolute_project_path(left));
    let right = right
        .canonicalize()
        .unwrap_or_else(|_| absolute_project_path(right));

    left == right
}

pub(crate) fn normalize_existing_project_path(path: &str) -> Result<PathBuf, AppError> {
    let path = normalize_project_path(path)?;
    if !path.exists() {
        return Err(AppError::new(
            "project_file_missing",
            "Die Projektdatei wurde nicht gefunden.",
        ));
    }

    if !path.is_file() {
        return Err(AppError::new(
            "invalid_project_path",
            "Der Projektpfad verweist nicht auf eine Datei.",
        ));
    }

    Ok(path)
}

pub(crate) fn map_invalid_project_file(error: rusqlite::Error) -> AppError {
    AppError::new(
        "invalid_project_file",
        format!("Die Datei ist keine gültige Autorenprojekt-Datei: {error}"),
    )
}

pub(crate) fn map_invalid_project_error(error: AppError) -> AppError {
    AppError::new(
        "invalid_project_file",
        format!(
            "Die Datei ist keine gültige Autorenprojekt-Datei: {}",
            error.message
        ),
    )
}

pub(crate) fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
