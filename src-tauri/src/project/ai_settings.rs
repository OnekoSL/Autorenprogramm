use super::*;

#[tauri::command]
pub fn get_ai_settings(app: AppHandle) -> Result<AiSettings, AppError> {
    read_ai_settings(&ai_settings_path(&app)?)
}

#[tauri::command]
pub fn update_ai_settings(
    input: UpdateAiSettingsInput,
    app: AppHandle,
) -> Result<AiSettings, AppError> {
    let settings = normalize_ai_settings(input);
    write_ai_settings(&ai_settings_path(&app)?, &settings)?;

    Ok(settings)
}

#[tauri::command]
pub fn get_ai_provider_status(app: AppHandle) -> Result<AiProviderStatus, AppError> {
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    get_ai_provider_status_for_settings(&settings)
}

#[tauri::command]
pub fn start_local_ai_provider(app: AppHandle) -> Result<AiProviderStatus, AppError> {
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    start_local_ai_provider_for_settings(&settings)
}

#[tauri::command]
pub fn list_ai_models(app: AppHandle) -> Result<Vec<AiModelInfo>, AppError> {
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    list_ai_models_for_settings(&settings)
}

pub(crate) fn normalize_ai_settings(input: UpdateAiSettingsInput) -> AiSettings {
    AiSettings {
        base_url: DEFAULT_AI_BASE_URL.to_string(),
        model: normalize_model_name(&input.model),
    }
}

pub(crate) fn normalize_base_url(base_url: &str) -> String {
    let base_url = base_url.trim().trim_end_matches('/');
    if base_url.is_empty() {
        DEFAULT_AI_BASE_URL.to_string()
    } else {
        base_url.to_string()
    }
}

pub(crate) fn normalize_model_name(model: &str) -> String {
    model.trim().to_string()
}

pub(crate) fn get_ai_provider_status_for_settings(
    settings: &AiSettings,
) -> Result<AiProviderStatus, AppError> {
    match query_ai_models_endpoint(settings, Duration::from_secs(2)) {
        Ok(()) => Ok(ai_provider_status(
            AI_PROVIDER_REACHABLE,
            "Lokale KI erreichbar.",
            false,
            settings,
        )),
        Err(error) => Ok(ai_unreachable_status(settings, &error.message)),
    }
}

pub(crate) fn start_local_ai_provider_for_settings(
    settings: &AiSettings,
) -> Result<AiProviderStatus, AppError> {
    if !is_local_ollama_default(&settings.base_url) {
        return Ok(ai_provider_status(
            AI_PROVIDER_UNSUPPORTED,
            "Dieser KI-Provider kann nicht automatisch gestartet werden.",
            false,
            settings,
        ));
    }

    if query_ai_models_endpoint(settings, Duration::from_secs(2)).is_ok() {
        return Ok(ai_provider_status(
            AI_PROVIDER_REACHABLE,
            "Lokale KI erreichbar.",
            false,
            settings,
        ));
    }

    let Some(ollama_command) = resolve_ollama_command() else {
        return Ok(ai_provider_status(
            AI_PROVIDER_UNSUPPORTED,
            "Ollama wurde auf diesem Rechner nicht gefunden.",
            false,
            settings,
        ));
    };

    let mut command = Command::new(ollama_command);
    command
        .arg("serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    command.spawn().map_err(|error| {
        AppError::new(
            "ai_provider_start_failed",
            format!("Ollama konnte nicht gestartet werden: {error}"),
        )
    })?;

    for _ in 0..12 {
        thread::sleep(Duration::from_millis(500));
        if query_ai_models_endpoint(settings, Duration::from_secs(2)).is_ok() {
            return Ok(ai_provider_status(
                AI_PROVIDER_REACHABLE,
                "Lokale KI wurde gestartet.",
                false,
                settings,
            ));
        }
    }

    Ok(ai_provider_status(
        AI_PROVIDER_STARTING,
        "Ollama wurde gestartet, antwortet aber noch nicht.",
        false,
        settings,
    ))
}

pub(crate) fn list_ai_models_for_settings(
    settings: &AiSettings,
) -> Result<Vec<AiModelInfo>, AppError> {
    if is_local_ollama_default(&settings.base_url) {
        if let Ok(models) = query_ollama_tags_endpoint(settings, Duration::from_secs(4)) {
            return Ok(models);
        }
    }

    query_openai_models_endpoint(settings, Duration::from_secs(4))
}

pub(crate) fn query_ollama_tags_endpoint(
    settings: &AiSettings,
    timeout: Duration,
) -> Result<Vec<AiModelInfo>, AppError> {
    let endpoint = format!("{}/api/tags", ollama_root_url(&settings.base_url));
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    let response = client.get(endpoint).send()?;
    let status = response.status();
    let body = response.text()?;

    if !status.is_success() {
        return Err(AppError::new(
            "ai_provider_error",
            format!("Ollama antwortete mit Status {status}."),
        ));
    }

    parse_ollama_tags_response(&body)
}

pub(crate) fn query_openai_models_endpoint(
    settings: &AiSettings,
    timeout: Duration,
) -> Result<Vec<AiModelInfo>, AppError> {
    let endpoint = format!("{}/models", settings.base_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    let response = client.get(endpoint).send()?;
    let status = response.status();
    let body = response.text()?;

    if !status.is_success() {
        return Err(AppError::new(
            "ai_provider_error",
            format!("Der KI-Dienst antwortete mit Status {status}."),
        ));
    }

    parse_openai_models_response(&body)
}

pub(crate) fn parse_ollama_tags_response(body: &str) -> Result<Vec<AiModelInfo>, AppError> {
    let parsed: serde_json::Value = serde_json::from_str(body)?;
    let models = parsed["models"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let name = item["name"].as_str()?.trim();
                    if name.is_empty() {
                        return None;
                    }

                    Some(AiModelInfo {
                        id: name.to_string(),
                        name: name.to_string(),
                        size: item["size"].as_u64().unwrap_or(0),
                        modified_at: item["modified_at"].as_str().unwrap_or("").to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(models)
}

pub(crate) fn parse_openai_models_response(body: &str) -> Result<Vec<AiModelInfo>, AppError> {
    let parsed: serde_json::Value = serde_json::from_str(body)?;
    let models = parsed["data"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let id = item["id"]
                        .as_str()
                        .or_else(|| item["name"].as_str())?
                        .trim();
                    if id.is_empty() {
                        return None;
                    }

                    Some(AiModelInfo {
                        id: id.to_string(),
                        name: id.to_string(),
                        size: item["size"].as_u64().unwrap_or(0),
                        modified_at: item["modified_at"]
                            .as_str()
                            .map(|value| value.to_string())
                            .or_else(|| item["created"].as_i64().map(|value| value.to_string()))
                            .unwrap_or_default(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(models)
}

pub(crate) fn ollama_root_url(base_url: &str) -> String {
    let normalized = normalize_base_url(base_url);
    normalized
        .strip_suffix("/v1")
        .unwrap_or(&normalized)
        .to_string()
}

pub(crate) fn query_ai_models_endpoint(
    settings: &AiSettings,
    timeout: Duration,
) -> Result<(), AppError> {
    let endpoint = format!("{}/models", settings.base_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    let response = client.get(endpoint).send()?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(AppError::new(
            "ai_provider_error",
            format!("Der KI-Dienst antwortete mit Status {}.", response.status()),
        ))
    }
}

pub(crate) fn ai_unreachable_status(settings: &AiSettings, detail: &str) -> AiProviderStatus {
    if !is_local_ollama_default(&settings.base_url) {
        return ai_provider_status(
            AI_PROVIDER_UNREACHABLE,
            format!("KI-Provider nicht erreichbar: {detail}"),
            false,
            settings,
        );
    }

    if resolve_ollama_command().is_none() {
        return ai_provider_status(
            AI_PROVIDER_UNSUPPORTED,
            "Ollama wurde auf diesem Rechner nicht gefunden.",
            false,
            settings,
        );
    }

    ai_provider_status(
        AI_PROVIDER_UNREACHABLE,
        "Lokale KI nicht erreichbar.",
        true,
        settings,
    )
}

pub(crate) fn ai_provider_status(
    state: &str,
    message: impl Into<String>,
    can_start: bool,
    settings: &AiSettings,
) -> AiProviderStatus {
    AiProviderStatus {
        state: state.to_string(),
        code: state.to_string(),
        message: message.into(),
        can_start,
        base_url: settings.base_url.clone(),
        model: settings.model.clone(),
    }
}

pub(crate) fn is_local_ollama_default(base_url: &str) -> bool {
    normalize_base_url(base_url).eq_ignore_ascii_case(DEFAULT_AI_BASE_URL)
}

pub(crate) fn resolve_ollama_command() -> Option<PathBuf> {
    if command_exists_in_path("ollama", env::var_os("PATH").as_deref()) {
        return Some(PathBuf::from("ollama"));
    }

    let local_app_data = env::var_os("LOCALAPPDATA")?;
    let candidate = PathBuf::from(local_app_data)
        .join("Programs")
        .join("Ollama")
        .join("ollama.exe");
    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

pub(crate) fn command_exists_in_path(
    command_name: &str,
    path_env: Option<&std::ffi::OsStr>,
) -> bool {
    let Some(path_env) = path_env else {
        return false;
    };

    env::split_paths(path_env).any(|path| {
        ollama_path_candidates(&path, command_name).any(|candidate| candidate.is_file())
    })
}

pub(crate) fn ollama_path_candidates<'a>(
    directory: &'a Path,
    command_name: &'a str,
) -> impl Iterator<Item = PathBuf> + 'a {
    let base = directory.join(command_name);
    let exe = directory.join(format!("{command_name}.exe"));

    [base, exe].into_iter()
}
