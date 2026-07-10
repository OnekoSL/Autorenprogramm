use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    env, fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

const PROJECT_EXTENSION: &str = "autorenprojekt";
const RECENT_PROJECTS_FILE: &str = "recent-projects.json";
const AI_SETTINGS_FILE: &str = "ai-settings.json";
const RECENT_PROJECTS_LIMIT: usize = 10;
const SCHEMA_VERSION: i64 = 7;
const DEFAULT_SCENE_STATUS: &str = "Entwurf";
const DEFAULT_EDITOR_FONT_FAMILY: &str = "serif";
const DEFAULT_EDITOR_FONT_SIZE: i64 = 19;
const SCENE_LINK_RELATION_ASSIGNED: &str = "assigned";
const AI_ACTION_SUMMARIZE_SCENE: &str = "summarize_scene";
const AI_ACTION_CHECK_SCENE_CONTRADICTIONS: &str = "check_scene_contradictions";
const AI_BIBLE_MODE_GENERATE: &str = "generate";
const AI_BIBLE_MODE_COMPLETE: &str = "complete";
const AI_STATUS_DRAFT: &str = "draft";
const AI_STATUS_ACCEPTED: &str = "accepted";
const AI_STATUS_REJECTED: &str = "rejected";
const DEFAULT_AI_BASE_URL: &str = "http://localhost:11434/v1";
const AI_PROVIDER_REACHABLE: &str = "reachable";
const AI_PROVIDER_STARTING: &str = "starting";
const AI_PROVIDER_UNREACHABLE: &str = "unreachable";
const AI_PROVIDER_UNSUPPORTED: &str = "unsupported";
const AI_CHAT_MODE_IDEAS: &str = "ideas";
const AI_CHAT_MODE_WRITING: &str = "writing";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Default)]
pub struct AppState {
    active_project: Mutex<Option<ProjectSession>>,
}

struct ProjectSession {
    _snapshot: ProjectSnapshot,
    connection: Connection,
}

#[derive(Debug, Serialize)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
}

impl AppError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn io(error: std::io::Error) -> Self {
        Self::new("io_error", error.to_string())
    }

    fn storage(error: rusqlite::Error) -> Self {
        Self::new("storage_error", error.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::io(error)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        AppError::storage(error)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::new("json_error", error.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(error: tauri::Error) -> Self {
        AppError::new("tauri_error", error.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() || error.is_connect() {
            return AppError::new(
                "ai_provider_unreachable",
                "Der lokale KI-Dienst ist nicht erreichbar.",
            );
        }

        AppError::new("ai_provider_error", error.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(error: zip::result::ZipError) -> Self {
        AppError::new("zip_error", error.to_string())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub name: String,
    pub description: Option<String>,
    pub default_language: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectInput {
    pub name: String,
    pub description: String,
    pub default_language: String,
    pub editor_font_family: String,
    pub editor_font_size: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProjectCopyAsInput {
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSnapshot {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_language: String,
    pub editor_font_family: String,
    pub editor_font_size: i64,
    pub path: String,
    pub storage_kind: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecentProject {
    pub name: String,
    pub path: String,
    pub storage_kind: String,
    pub last_opened_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManuscriptTree {
    pub novel: NovelSnapshot,
    pub areas: Vec<AreaSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NovelSnapshot {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub author_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaSnapshot {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub sort_order: i64,
    pub chapters: Vec<ChapterSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChapterSnapshot {
    pub id: String,
    pub area_id: String,
    pub title: String,
    pub summary: String,
    pub sort_order: i64,
    pub scenes: Vec<SceneSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SceneSnapshot {
    pub id: String,
    pub chapter_id: String,
    pub title: String,
    pub text: String,
    pub status: String,
    pub summary: String,
    pub sort_order: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchScenesInput {
    pub query: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchProjectInput {
    pub query: String,
    pub scope: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchSceneResult {
    pub scene_id: String,
    pub scene_title: String,
    pub chapter_title: String,
    pub area_title: String,
    pub matched_fields: Vec<String>,
    pub snippets: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchProjectResult {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub matched_fields: Vec<String>,
    pub snippets: Vec<String>,
    pub scene_id: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportManuscriptInput {
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAreaInput {
    pub title: Option<String>,
    pub after_area_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChapterInput {
    pub area_id: Option<String>,
    pub title: Option<String>,
    pub after_chapter_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSceneInput {
    pub chapter_id: Option<String>,
    pub title: Option<String>,
    pub after_scene_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateChapterInput {
    pub id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAreaInput {
    pub id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSceneInput {
    pub id: String,
    pub title: String,
    pub text: String,
    pub status: String,
    pub summary: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveChapterInput {
    pub chapter_id: String,
    pub target_area_id: String,
    pub target_index: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveAreaInput {
    pub area_id: String,
    pub target_index: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveSceneInput {
    pub scene_id: String,
    pub target_chapter_id: String,
    pub target_index: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteChapterInput {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAreaInput {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSceneInput {
    pub id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBibleSnapshot {
    pub characters: Vec<CharacterSnapshot>,
    pub locations: Vec<LocationSnapshot>,
    pub items: Vec<ItemSnapshot>,
    pub notes: Vec<NoteSnapshot>,
    pub scene_links: Vec<SceneLinkSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSnapshot {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocationSnapshot {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ItemSnapshot {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NoteSnapshot {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub body: String,
    pub note_type: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SceneLinkSnapshot {
    pub id: String,
    pub scene_id: String,
    pub target_type: String,
    pub target_id: String,
    pub relation: String,
    pub sort_order: i64,
    pub scene_title: String,
    pub chapter_title: String,
    pub area_title: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectBibleCardInput {
    pub target_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectBibleCardInput {
    pub target_type: String,
    pub id: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub note_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteProjectBibleCardInput {
    pub target_type: String,
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBibleCardLinkInput {
    pub target_type: String,
    pub target_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateBibleCardInput {
    pub target_type: String,
    pub keywords: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteBibleCardInput {
    pub target_type: String,
    pub target_id: String,
    pub keywords: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAiBibleSuggestionsInput {
    pub target_type: Option<String>,
    pub target_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptAiBibleSuggestionInput {
    pub id: String,
    pub draft: AiBibleCardDraft,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectAiBibleSuggestionInput {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceSceneLinksForCardInput {
    pub target_type: String,
    pub target_id: String,
    pub scene_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    #[serde(skip, default = "default_ai_base_url")]
    pub base_url: String,
    pub model: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_AI_BASE_URL.to_string(),
            model: String::new(),
        }
    }
}

fn default_ai_base_url() -> String {
    DEFAULT_AI_BASE_URL.to_string()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAiSettingsInput {
    pub model: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAiChatMessageInput {
    pub messages: Vec<AiChatMessage>,
    pub scene_id: Option<String>,
    pub include_scene_context: bool,
    pub mode: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiChatResponse {
    pub message: AiChatMessage,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateTextAlternativesInput {
    pub scene_id: String,
    pub selected_text: String,
    pub context_before: String,
    pub context_after: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextAlternativesResponse {
    pub alternatives: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImproveTextInput {
    pub scene_id: String,
    pub selected_text: String,
    pub context_before: String,
    pub context_after: String,
    pub mode: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextImprovementsResponse {
    pub improvements: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderStatus {
    pub state: String,
    pub message: String,
    pub can_start: bool,
    pub base_url: String,
    pub model: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiModelInfo {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub modified_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummarizeSceneInput {
    pub scene_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSuggestionInput {
    pub id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiSuggestionSnapshot {
    pub id: String,
    pub project_id: String,
    pub scene_id: String,
    pub action_type: String,
    pub input_excerpt: String,
    pub context_summary: String,
    pub result: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiBibleCardDraft {
    pub name: String,
    pub role: String,
    pub description: String,
    pub notes: String,
    pub title: String,
    pub body: String,
    pub note_type: String,
}

impl Default for AiBibleCardDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            role: String::new(),
            description: String::new(),
            notes: String::new(),
            title: String::new(),
            body: String::new(),
            note_type: "general".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiBibleSuggestionSnapshot {
    pub id: String,
    pub project_id: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub mode: String,
    pub keywords: String,
    pub draft: AiBibleCardDraft,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiBibleAcceptResult {
    pub snapshot: ProjectBibleSnapshot,
    pub target_type: String,
    pub target_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiSceneRequest {
    project_id: String,
    scene_id: String,
    scene_title: String,
    scene_summary: String,
    scene_text: String,
    context_summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiContradictionRequest {
    project_id: String,
    scene_id: String,
    scene_title: String,
    scene_summary: String,
    scene_text: String,
    context_summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AiBibleRequest {
    project_id: String,
    target_type: String,
    target_id: Option<String>,
    mode: String,
    keywords: String,
    existing_card_summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TextAlternativesRequest {
    scene_id: String,
    selected_text: String,
    context_before: String,
    context_after: String,
    is_phrase: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TextImprovementRequest {
    scene_id: String,
    selected_text: String,
    context_before: String,
    context_after: String,
    mode: String,
}

pub fn recent_projects_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app.path().app_config_dir()?.join(RECENT_PROJECTS_FILE))
}

pub fn ai_settings_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app.path().app_config_dir()?.join(AI_SETTINGS_FILE))
}

#[tauri::command]
pub fn create_project(
    input: CreateProjectInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<ProjectSnapshot, AppError> {
    let snapshot = create_project_file(input)?;
    touch_recent_project(&recent_projects_path(&app)?, &snapshot)?;
    set_active_project(&state, snapshot.clone())?;

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
        AppError::new(
            "no_active_project",
            "Es ist kein aktives Projekt geoeffnet.",
        )
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
        AppError::new(
            "no_active_project",
            "Es ist kein aktives Projekt geoeffnet.",
        )
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

#[tauri::command]
pub fn send_ai_chat_message(
    input: SendAiChatMessageInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<AiChatResponse, AppError> {
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    with_active_connection(&state, |connection| {
        let context = build_ai_chat_context(connection, &input)?;
        let content = call_openai_compatible_free_chat(&settings, &input, context.as_deref())?;

        Ok(AiChatResponse {
            message: AiChatMessage {
                role: "assistant".to_string(),
                content,
            },
        })
    })
}

#[tauri::command]
pub fn generate_text_alternatives(
    input: GenerateTextAlternativesInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<TextAlternativesResponse, AppError> {
    let request = with_active_connection(&state, |connection| {
        build_text_alternatives_request(connection, input)
    })?;
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    let alternatives = call_openai_compatible_text_alternatives(&settings, &request)?;

    Ok(TextAlternativesResponse { alternatives })
}

#[tauri::command]
pub fn improve_text(
    input: ImproveTextInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<TextImprovementsResponse, AppError> {
    let request = with_active_connection(&state, |connection| {
        build_improve_text_request(connection, input)
    })?;
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    let improvements = call_openai_compatible_text_improvements(&settings, &request)?;

    Ok(TextImprovementsResponse { improvements })
}

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

#[tauri::command]
pub fn export_manuscript_markdown(
    input: ExportManuscriptInput,
    state: State<AppState>,
) -> Result<(), AppError> {
    with_active_connection(&state, |connection| {
        export_manuscript_markdown_in_connection(connection, &input.path)
    })
}

#[tauri::command]
pub fn export_manuscript_docx(
    input: ExportManuscriptInput,
    state: State<AppState>,
) -> Result<(), AppError> {
    with_active_connection(&state, |connection| {
        export_manuscript_docx_in_connection(connection, &input.path)
    })
}

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

#[tauri::command]
pub fn generate_bible_card_from_keywords(
    input: GenerateBibleCardInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let request = with_active_connection(&state, |connection| {
        build_ai_bible_generate_request(connection, input)
    })?;
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    let draft = call_openai_compatible_bible_chat(&settings, &request)?;

    with_active_connection(&state, |connection| {
        insert_ai_bible_suggestion(connection, &request, draft)
    })
}

#[tauri::command]
pub fn complete_bible_card_from_keywords(
    input: CompleteBibleCardInput,
    app: AppHandle,
    state: State<AppState>,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let request = with_active_connection(&state, |connection| {
        build_ai_bible_complete_request(connection, input)
    })?;
    let settings = read_ai_settings(&ai_settings_path(&app)?)?;
    let draft = call_openai_compatible_bible_chat(&settings, &request)?;

    with_active_connection(&state, |connection| {
        insert_ai_bible_suggestion(connection, &request, draft)
    })
}

#[tauri::command]
pub fn list_ai_bible_suggestions(
    input: ListAiBibleSuggestionsInput,
    state: State<AppState>,
) -> Result<Vec<AiBibleSuggestionSnapshot>, AppError> {
    with_active_connection(&state, |connection| {
        read_ai_bible_suggestions(
            connection,
            input.target_type.as_deref(),
            input.target_id.as_deref(),
        )
    })
}

#[tauri::command]
pub fn accept_ai_bible_suggestion(
    input: AcceptAiBibleSuggestionInput,
    state: State<AppState>,
) -> Result<AiBibleAcceptResult, AppError> {
    with_active_connection(&state, |connection| {
        let (target_type, target_id) = accept_ai_bible_suggestion_in_connection(connection, input)?;
        Ok(AiBibleAcceptResult {
            snapshot: read_project_bible(connection)?,
            target_type,
            target_id,
        })
    })
}

#[tauri::command]
pub fn reject_ai_bible_suggestion(
    input: RejectAiBibleSuggestionInput,
    state: State<AppState>,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    with_active_connection(&state, |connection| {
        update_ai_bible_suggestion_status(connection, &input.id, AI_STATUS_REJECTED)
    })
}

fn set_active_project(state: &State<AppState>, snapshot: ProjectSnapshot) -> Result<(), AppError> {
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

fn with_active_connection<T>(
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
        AppError::new(
            "no_active_project",
            "Es ist kein aktives Projekt geoeffnet.",
        )
    })?;

    action(&mut session.connection)
}

pub fn create_project_file(input: CreateProjectInput) -> Result<ProjectSnapshot, AppError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::new(
            "invalid_project_name",
            "Der Projektname darf nicht leer sein.",
        ));
    }

    let default_language = input.default_language.trim();
    if default_language.is_empty() {
        return Err(AppError::new(
            "invalid_project_language",
            "Die Standardsprache darf nicht leer sein.",
        ));
    }

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

fn update_project_in_connection(
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

    let default_language = input.default_language.trim();
    if default_language.is_empty() {
        return Err(AppError::new(
            "invalid_project_language",
            "Die Standardsprache darf nicht leer sein.",
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

fn normalize_editor_font_family(font_family: &str) -> String {
    match font_family.trim() {
        "serif" | "sans" | "monospace" => font_family.trim().to_string(),
        _ => DEFAULT_EDITOR_FONT_FAMILY.to_string(),
    }
}

fn normalize_editor_font_size(font_size: i64) -> i64 {
    if (16..=24).contains(&font_size) {
        font_size
    } else {
        DEFAULT_EDITOR_FONT_SIZE
    }
}

fn validate_editor_font_family(font_family: &str) -> Result<String, AppError> {
    let font_family = font_family.trim();
    if matches!(font_family, "serif" | "sans" | "monospace") {
        return Ok(font_family.to_string());
    }

    Err(AppError::new(
        "invalid_editor_font_family",
        "Die Editor-Schriftart ist ungueltig.",
    ))
}

fn validate_editor_font_size(font_size: i64) -> Result<i64, AppError> {
    if (16..=24).contains(&font_size) {
        return Ok(font_size);
    }

    Err(AppError::new(
        "invalid_editor_font_size",
        "Die Editor-Schriftgroesse muss zwischen 16 und 24 liegen.",
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

fn write_recent_projects(path: &Path, recent_projects: &[RecentProject]) -> Result<(), AppError> {
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

fn write_ai_settings(path: &Path, settings: &AiSettings) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(settings)?;
    fs::write(path, content)?;

    Ok(())
}

fn normalize_ai_settings(input: UpdateAiSettingsInput) -> AiSettings {
    AiSettings {
        base_url: DEFAULT_AI_BASE_URL.to_string(),
        model: normalize_model_name(&input.model),
    }
}

fn normalize_base_url(base_url: &str) -> String {
    let base_url = base_url.trim().trim_end_matches('/');
    if base_url.is_empty() {
        DEFAULT_AI_BASE_URL.to_string()
    } else {
        base_url.to_string()
    }
}

fn normalize_model_name(model: &str) -> String {
    model.trim().to_string()
}

fn get_ai_provider_status_for_settings(
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

fn start_local_ai_provider_for_settings(
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

fn list_ai_models_for_settings(settings: &AiSettings) -> Result<Vec<AiModelInfo>, AppError> {
    if is_local_ollama_default(&settings.base_url) {
        match query_ollama_tags_endpoint(settings, Duration::from_secs(4)) {
            Ok(models) => return Ok(models),
            Err(_) => {}
        }
    }

    query_openai_models_endpoint(settings, Duration::from_secs(4))
}

fn query_ollama_tags_endpoint(
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

fn query_openai_models_endpoint(
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

fn parse_ollama_tags_response(body: &str) -> Result<Vec<AiModelInfo>, AppError> {
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

fn parse_openai_models_response(body: &str) -> Result<Vec<AiModelInfo>, AppError> {
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

fn ollama_root_url(base_url: &str) -> String {
    let normalized = normalize_base_url(base_url);
    normalized
        .strip_suffix("/v1")
        .unwrap_or(&normalized)
        .to_string()
}

fn query_ai_models_endpoint(settings: &AiSettings, timeout: Duration) -> Result<(), AppError> {
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

fn ai_unreachable_status(settings: &AiSettings, detail: &str) -> AiProviderStatus {
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

fn ai_provider_status(
    state: &str,
    message: impl Into<String>,
    can_start: bool,
    settings: &AiSettings,
) -> AiProviderStatus {
    AiProviderStatus {
        state: state.to_string(),
        message: message.into(),
        can_start,
        base_url: settings.base_url.clone(),
        model: settings.model.clone(),
    }
}

fn is_local_ollama_default(base_url: &str) -> bool {
    normalize_base_url(base_url).eq_ignore_ascii_case(DEFAULT_AI_BASE_URL)
}

fn resolve_ollama_command() -> Option<PathBuf> {
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

fn command_exists_in_path(command_name: &str, path_env: Option<&std::ffi::OsStr>) -> bool {
    let Some(path_env) = path_env else {
        return false;
    };

    env::split_paths(path_env).any(|path| {
        ollama_path_candidates(&path, command_name).any(|candidate| candidate.is_file())
    })
}

fn ollama_path_candidates<'a>(
    directory: &'a Path,
    command_name: &'a str,
) -> impl Iterator<Item = PathBuf> + 'a {
    let base = directory.join(command_name);
    let exe = directory.join(format!("{command_name}.exe"));

    [base, exe].into_iter()
}

fn create_manuscript_tables(connection: &Connection) -> Result<(), AppError> {
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

fn create_project_bible_tables(connection: &Connection) -> Result<(), AppError> {
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

fn create_ai_tables(connection: &Connection) -> Result<(), AppError> {
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

fn seed_initial_manuscript(connection: &Connection) -> Result<(), AppError> {
    let project: Option<(String, String)> = connection
        .query_row(
            "
            SELECT id, name
            FROM project
            WHERE deleted_at IS NULL
            ORDER BY created_at ASC
            LIMIT 1
            ",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let Some((project_id, project_name)) = project else {
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
        VALUES (?1, ?2, 'Bereich 1', '', 1, ?3, ?3, NULL)
        ",
        params![area_id, novel_id, now],
    )?;
    connection.execute(
        "
        INSERT INTO chapter (
            id, novel_id, area_id, title, summary, sort_order, created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, 'Kapitel 1', '', 1, ?4, ?4, NULL)
        ",
        params![chapter_id, novel_id, area_id, now],
    )?;
    connection.execute(
        "
        INSERT INTO scene (
            id, chapter_id, title, text, status, summary, sort_order,
            created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, 'Neue Szene', '', ?3, '', 1, ?4, ?4, NULL)
        ",
        params![scene_id, chapter_id, DEFAULT_SCENE_STATUS, now],
    )?;

    Ok(())
}

fn initialize_project_schema(
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

fn read_project_snapshot(
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

fn ensure_schema_version(connection: &Connection) -> Result<(), AppError> {
    let version: i64 = connection.query_row(
        "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
        [],
        |row| row.get(0),
    )?;

    if version == 1 {
        migrate_schema_v1_to_v2(connection)?;
        return Ok(());
    }

    if version == 2 {
        migrate_schema_v2_to_v3(connection)?;
        migrate_schema_v3_to_v4(connection)?;
        migrate_schema_v4_to_v5(connection)?;
        migrate_schema_v5_to_v6(connection)?;
        migrate_schema_v6_to_v7(connection)?;
        return Ok(());
    }

    if version == 3 {
        migrate_schema_v3_to_v4(connection)?;
        migrate_schema_v4_to_v5(connection)?;
        migrate_schema_v5_to_v6(connection)?;
        migrate_schema_v6_to_v7(connection)?;
        return Ok(());
    }

    if version == 4 {
        migrate_schema_v4_to_v5(connection)?;
        migrate_schema_v5_to_v6(connection)?;
        migrate_schema_v6_to_v7(connection)?;
        return Ok(());
    }

    if version == 5 {
        migrate_schema_v5_to_v6(connection)?;
        migrate_schema_v6_to_v7(connection)?;
        return Ok(());
    }

    if version == 6 {
        migrate_schema_v6_to_v7(connection)?;
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

fn migrate_schema_v1_to_v2(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        create_manuscript_tables(connection)?;
        create_project_bible_tables(connection)?;
        create_ai_tables(connection)?;
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

fn migrate_schema_v2_to_v3(connection: &Connection) -> Result<(), AppError> {
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

fn migrate_schema_v3_to_v4(connection: &Connection) -> Result<(), AppError> {
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

fn migrate_schema_v4_to_v5(connection: &Connection) -> Result<(), AppError> {
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

fn migrate_schema_v5_to_v6(connection: &Connection) -> Result<(), AppError> {
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

fn migrate_schema_v6_to_v7(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch("PRAGMA foreign_keys = ON; BEGIN;")?;
    let result = (|| {
        ensure_project_editor_settings_columns(connection)?;
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

fn ensure_project_editor_settings_columns(connection: &Connection) -> Result<(), AppError> {
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

fn table_has_column(
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

fn read_manuscript_tree(connection: &Connection) -> Result<ManuscriptTree, AppError> {
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

fn read_active_novel(connection: &Connection) -> Result<NovelSnapshot, AppError> {
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

fn read_scenes_for_chapter(
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

fn read_chapters_for_area(
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

fn search_scenes_in_connection(
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

fn search_project_in_connection(
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

fn normalize_search_scope(scope: &str) -> Result<&'static str, AppError> {
    match scope.trim() {
        "" | "all" => Ok("all"),
        "scenes" => Ok("scenes"),
        "bible" => Ok("bible"),
        _ => Err(AppError::new(
            "invalid_search_scope",
            "Der Suchbereich ist ungueltig.",
        )),
    }
}

fn search_project_scenes_in_connection(
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

fn search_project_bible_in_connection(
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
            "character",
            &id,
            &name,
            "Figur",
            &[
                ("Name", name.as_str()),
                ("Rolle", role.as_str()),
                ("Beschreibung", description.as_str()),
                ("Notizen", notes.as_str()),
            ],
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
            "location",
            &id,
            &name,
            "Ort",
            &[
                ("Name", name.as_str()),
                ("Beschreibung", description.as_str()),
                ("Notizen", notes.as_str()),
            ],
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
            "item",
            &id,
            &name,
            "Gegenstand",
            &[
                ("Name", name.as_str()),
                ("Beschreibung", description.as_str()),
                ("Notizen", notes.as_str()),
            ],
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
            "note",
            &id,
            &title,
            "Notiz",
            &[
                ("Titel", title.as_str()),
                ("Inhalt", body.as_str()),
                ("Notiztyp", note_type.as_str()),
            ],
        );
    }

    Ok(results)
}

fn push_bible_search_result(
    results: &mut Vec<SearchProjectResult>,
    query: &str,
    query_lower: &str,
    target_type: &str,
    target_id: &str,
    title: &str,
    subtitle: &str,
    fields: &[(&str, &str)],
) {
    let mut matched_fields = Vec::new();
    let mut snippets = Vec::new();

    for (label, value) in fields {
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
        id: format!("{target_type}:{target_id}"),
        title: title.to_string(),
        subtitle: subtitle.to_string(),
        matched_fields,
        snippets,
        scene_id: None,
        target_type: Some(target_type.to_string()),
        target_id: Some(target_id.to_string()),
    });
}

fn find_case_insensitive(haystack: &str, query_lower: &str) -> Option<usize> {
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

fn compact_search_snippet(value: &str, query: &str) -> String {
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

fn validate_export_path(path: &str) -> Result<PathBuf, AppError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(AppError::new(
            "invalid_export_path",
            "Bitte waehle einen Exportpfad aus.",
        ));
    }

    Ok(PathBuf::from(path))
}

fn build_markdown_export(tree: &ManuscriptTree) -> String {
    let mut markdown = String::new();

    for area in &tree.areas {
        let area_has_text = area.chapters.iter().any(|chapter| {
            chapter
                .scenes
                .iter()
                .any(|scene| !scene.text.trim().is_empty())
        });
        if !area_has_text {
            continue;
        }

        markdown.push_str("# ");
        markdown.push_str(area.title.trim());
        markdown.push_str("\n\n");

        for chapter in &area.chapters {
            let chapter_has_text = chapter
                .scenes
                .iter()
                .any(|scene| !scene.text.trim().is_empty());
            if !chapter_has_text {
                continue;
            }

            markdown.push_str("## ");
            markdown.push_str(chapter.title.trim());
            markdown.push_str("\n\n");

            for scene in &chapter.scenes {
                let text = scene.text.trim();
                if text.is_empty() {
                    continue;
                }

                markdown.push_str(text);
                markdown.push_str("\n\n");
            }
        }
    }

    markdown.trim_end().to_string() + "\n"
}

fn export_manuscript_markdown_in_connection(
    connection: &Connection,
    path: &str,
) -> Result<(), AppError> {
    let path = validate_export_path(path)?;
    let tree = read_manuscript_tree(connection)?;
    fs::write(path, build_markdown_export(&tree))?;

    Ok(())
}

fn export_manuscript_docx_in_connection(
    connection: &Connection,
    path: &str,
) -> Result<(), AppError> {
    let path = validate_export_path(path)?;
    let tree = read_manuscript_tree(connection)?;
    fs::write(path, build_docx_export(&tree)?)?;

    Ok(())
}

fn build_docx_export(tree: &ManuscriptTree) -> Result<Vec<u8>, AppError> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let stored = FileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = FileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", stored)?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>"#,
    )?;

    zip.add_directory("_rels/", stored)?;
    zip.start_file("_rels/.rels", stored)?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
    )?;

    zip.add_directory("word/", stored)?;
    zip.start_file("word/document.xml", deflated)?;
    zip.write_all(build_docx_document_xml(tree).as_bytes())?;

    zip.add_directory("word/_rels/", stored)?;
    zip.start_file("word/_rels/document.xml.rels", stored)?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#,
    )?;

    zip.start_file("word/styles.xml", deflated)?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
    <w:qFormat/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:qFormat/>
    <w:pPr><w:outlineLvl w:val="0"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="32"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:qFormat/>
    <w:pPr><w:outlineLvl w:val="1"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="26"/></w:rPr>
  </w:style>
</w:styles>"#,
    )?;

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

fn build_docx_document_xml(tree: &ManuscriptTree) -> String {
    let mut body = String::new();

    for area in &tree.areas {
        let area_has_text = area.chapters.iter().any(|chapter| {
            chapter
                .scenes
                .iter()
                .any(|scene| !scene.text.trim().is_empty())
        });
        if !area_has_text {
            continue;
        }

        body.push_str(&docx_paragraph(Some("Heading1"), area.title.trim()));

        for chapter in &area.chapters {
            let chapter_has_text = chapter
                .scenes
                .iter()
                .any(|scene| !scene.text.trim().is_empty());
            if !chapter_has_text {
                continue;
            }

            body.push_str(&docx_paragraph(Some("Heading2"), chapter.title.trim()));

            for scene in &chapter.scenes {
                for paragraph in split_export_paragraphs(&scene.text) {
                    body.push_str(&docx_paragraph(None, &paragraph));
                }
            }
        }
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    {body}
    <w:sectPr>
      <w:pgSz w:w="11906" w:h="16838"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/>
    </w:sectPr>
  </w:body>
</w:document>"#
    )
}

fn split_export_paragraphs(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn docx_paragraph(style: Option<&str>, text: &str) -> String {
    let style_xml = style
        .map(|style| format!(r#"<w:pPr><w:pStyle w:val="{style}"/></w:pPr>"#))
        .unwrap_or_default();
    let runs = text
        .split('\n')
        .enumerate()
        .map(|(index, line)| {
            let prefix = if index == 0 { "" } else { "<w:br/>" };
            format!(
                r#"{prefix}<w:t xml:space="preserve">{}</w:t>"#,
                xml_escape(line)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!("<w:p>{style_xml}<w:r>{runs}</w:r></w:p>")
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn read_project_bible(connection: &Connection) -> Result<ProjectBibleSnapshot, AppError> {
    Ok(ProjectBibleSnapshot {
        characters: read_characters(connection)?,
        locations: read_locations(connection)?,
        items: read_items(connection)?,
        notes: read_notes(connection)?,
        scene_links: read_active_scene_links(connection)?,
    })
}

fn read_characters(connection: &Connection) -> Result<Vec<CharacterSnapshot>, AppError> {
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

fn read_locations(connection: &Connection) -> Result<Vec<LocationSnapshot>, AppError> {
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

fn read_items(connection: &Connection) -> Result<Vec<ItemSnapshot>, AppError> {
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

fn read_notes(connection: &Connection) -> Result<Vec<NoteSnapshot>, AppError> {
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

fn read_active_scene_links(connection: &Connection) -> Result<Vec<SceneLinkSnapshot>, AppError> {
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

fn read_scene_links_for_card(
    connection: &Connection,
    target_type: &str,
    target_id: &str,
) -> Result<Vec<SceneLinkSnapshot>, AppError> {
    Ok(read_active_scene_links(connection)?
        .into_iter()
        .filter(|link| link.target_type == target_type && link.target_id == target_id)
        .collect())
}

fn create_project_bible_card_in_connection(
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
        _ => unreachable!(),
    }

    Ok(())
}

fn update_project_bible_card_in_connection(
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
        _ => unreachable!(),
    };

    require_changed(
        changed,
        "project_bible_card_missing",
        "Die Projektbibel-Karte wurde nicht gefunden.",
    )
}

fn delete_project_bible_card_in_connection(
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
    tx.commit()?;

    Ok(())
}

fn replace_scene_links_for_card_in_connection(
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

fn create_area_in_connection(
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
    let title = normalize_title(input.title.as_deref(), "Neuer Bereich");
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

fn create_chapter_in_connection(
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
    let title = normalize_title(input.title.as_deref(), "Neues Kapitel");
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

fn create_scene_in_connection(
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
    let title = normalize_title(input.title.as_deref(), "Neue Szene");
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

fn update_chapter_in_connection(
    connection: &mut Connection,
    input: UpdateChapterInput,
) -> Result<(), AppError> {
    let title = normalize_title(Some(&input.title), "Neues Kapitel");
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

fn update_area_in_connection(
    connection: &mut Connection,
    input: UpdateAreaInput,
) -> Result<(), AppError> {
    let title = normalize_title(Some(&input.title), "Neuer Bereich");
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

fn update_scene_in_connection(
    connection: &mut Connection,
    input: UpdateSceneInput,
) -> Result<(), AppError> {
    let title = normalize_title(Some(&input.title), "Neue Szene");
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

fn move_chapter_in_connection(
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

fn move_area_in_connection(
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

fn move_scene_in_connection(
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

fn delete_chapter_in_connection(
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

fn delete_area_in_connection(
    connection: &mut Connection,
    input: DeleteAreaInput,
) -> Result<(), AppError> {
    let tx = connection.transaction()?;
    let novel = read_active_novel(&tx)?;
    ensure_active_area(&tx, &input.id)?;
    let now = timestamp();

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

fn delete_scene_in_connection(
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
        "UPDATE scene SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        params![now, input.id],
    )?;
    normalize_scene_order(&tx, &chapter_id, None)?;
    tx.commit()?;

    Ok(())
}

fn read_area_ids(connection: &Connection, novel_id: &str) -> Result<Vec<String>, AppError> {
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

fn read_chapter_ids(connection: &Connection, area_id: &str) -> Result<Vec<String>, AppError> {
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

fn read_scene_ids(connection: &Connection, chapter_id: &str) -> Result<Vec<String>, AppError> {
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

fn first_chapter_id(connection: &Connection) -> Result<Option<String>, AppError> {
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

fn first_area_id(connection: &Connection) -> Result<Option<String>, AppError> {
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

fn ensure_active_area(connection: &Connection, area_id: &str) -> Result<(), AppError> {
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

fn ensure_active_chapter(connection: &Connection, chapter_id: &str) -> Result<(), AppError> {
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

fn normalize_area_order(
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

fn normalize_chapter_order(
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

fn normalize_scene_order(
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

fn normalize_title(value: Option<&str>, fallback: &str) -> String {
    let title = value.unwrap_or("").trim();
    if title.is_empty() {
        fallback.to_string()
    } else {
        title.to_string()
    }
}

fn normalize_scene_status(value: &str) -> String {
    match value.trim() {
        "Ueberarbeitung" => "Ueberarbeitung".to_string(),
        "Fertig" => "Fertig".to_string(),
        _ => DEFAULT_SCENE_STATUS.to_string(),
    }
}

fn require_changed(
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

fn read_active_project_id(connection: &Connection) -> Result<String, AppError> {
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

fn validate_target_type(target_type: &str) -> Result<(), AppError> {
    match target_type {
        "character" | "location" | "item" | "note" => Ok(()),
        _ => Err(AppError::new(
            "invalid_project_bible_card_type",
            "Dieser Projektbibel-Kartentyp wird nicht unterstuetzt.",
        )),
    }
}

fn normalize_note_type(value: Option<&str>) -> String {
    match value.unwrap_or("").trim() {
        "research" => "research".to_string(),
        "lore" => "lore".to_string(),
        "faction" => "faction".to_string(),
        "event" => "event".to_string(),
        "todo" => "todo".to_string(),
        _ => "general".to_string(),
    }
}

fn ensure_active_scene(connection: &Connection, scene_id: &str) -> Result<(), AppError> {
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

fn ensure_active_project_bible_card(
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

fn build_ai_scene_request(
    connection: &Connection,
    input: &SummarizeSceneInput,
) -> Result<AiSceneRequest, AppError> {
    let (project_id, scene_id, scene_title, scene_summary, scene_text): (
        String,
        String,
        String,
        String,
        String,
    ) = connection
        .query_row(
            "
            SELECT project.id, scene.id, scene.title, scene.summary, scene.text
            FROM scene
            INNER JOIN chapter ON chapter.id = scene.chapter_id AND chapter.deleted_at IS NULL
            INNER JOIN novel ON novel.id = chapter.novel_id AND novel.deleted_at IS NULL
            INNER JOIN project ON project.id = novel.project_id AND project.deleted_at IS NULL
            WHERE scene.id = ?1 AND scene.deleted_at IS NULL
            LIMIT 1
            ",
            params![input.scene_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::new("scene_missing", "Die Szene wurde nicht gefunden."))?;

    if scene_text.trim().is_empty() {
        return Err(AppError::new(
            "ai_scene_empty",
            "Die aktuelle Szene enthaelt noch keinen Text.",
        ));
    }

    Ok(AiSceneRequest {
        project_id,
        scene_id: scene_id.clone(),
        scene_title,
        scene_summary,
        scene_text,
        context_summary: build_scene_project_bible_context(connection, &scene_id)?,
    })
}

fn build_ai_contradiction_request(
    connection: &Connection,
    input: &SummarizeSceneInput,
) -> Result<AiContradictionRequest, AppError> {
    let request = build_ai_scene_request(connection, input)?;
    if !scene_has_project_bible_context(connection, &request.scene_id)? {
        return Err(AppError::new(
            "ai_scene_context_missing",
            "Verknuepfe zuerst mindestens eine Projektbibel-Karte mit dieser Szene.",
        ));
    }

    Ok(AiContradictionRequest {
        project_id: request.project_id,
        scene_id: request.scene_id,
        scene_title: request.scene_title,
        scene_summary: request.scene_summary,
        scene_text: request.scene_text,
        context_summary: request.context_summary,
    })
}

fn scene_has_project_bible_context(
    connection: &Connection,
    scene_id: &str,
) -> Result<bool, AppError> {
    let count: i64 = connection.query_row(
        "
        SELECT COUNT(*)
        FROM scene_link
        WHERE scene_id = ?1 AND deleted_at IS NULL
        ",
        params![scene_id],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

fn build_scene_project_bible_context(
    connection: &Connection,
    scene_id: &str,
) -> Result<String, AppError> {
    let mut statement = connection.prepare(
        "
        SELECT target_type, target_id
        FROM scene_link
        WHERE scene_id = ?1 AND deleted_at IS NULL
        ORDER BY sort_order ASC, created_at ASC
        ",
    )?;
    let links = statement
        .query_map(params![scene_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut context_parts = Vec::new();
    for (target_type, target_id) in links {
        if let Some(context) =
            read_project_bible_context_card(connection, &target_type, &target_id)?
        {
            context_parts.push(context);
        }
    }

    if context_parts.is_empty() {
        Ok("Keine Projektbibel-Karten mit dieser Szene verknuepft.".to_string())
    } else {
        Ok(context_parts.join("\n\n"))
    }
}

fn build_ai_chat_context(
    connection: &Connection,
    input: &SendAiChatMessageInput,
) -> Result<Option<String>, AppError> {
    if !input.include_scene_context {
        return Ok(None);
    }

    let scene_id = input.scene_id.as_deref().unwrap_or("").trim();
    if scene_id.is_empty() {
        return Ok(None);
    }

    ensure_active_scene(connection, scene_id)?;
    let (title, status, summary, text): (String, String, String, String) = connection.query_row(
        "
        SELECT title, status, summary, text
        FROM scene
        WHERE id = ?1 AND deleted_at IS NULL
        ",
        params![scene_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let bible_context = build_scene_project_bible_context(connection, scene_id)?;
    let excerpt = if text.chars().count() > 4000 {
        format!("{}...", text.chars().take(4000).collect::<String>())
    } else {
        text
    };

    Ok(Some(format!(
        "Aktuelle Szene:\nTitel: {title}\nStatus: {status}\nKurzbeschreibung: {}\n\nSzenentext-Auszug:\n{excerpt}\n\nProjektbibel-Kontext:\n{bible_context}",
        if summary.trim().is_empty() {
            "Keine Kurzbeschreibung"
        } else {
            summary.trim()
        }
    )))
}

fn build_text_alternatives_request(
    connection: &Connection,
    input: GenerateTextAlternativesInput,
) -> Result<TextAlternativesRequest, AppError> {
    ensure_active_scene(connection, &input.scene_id)?;
    let selected_text = input.selected_text.trim();
    if selected_text.is_empty() {
        return Err(AppError::new(
            "empty_text_selection",
            "Bitte markiere zuerst ein Wort oder einen Satz.",
        ));
    }

    let word_count = selected_text.split_whitespace().count();
    let is_phrase = word_count > 1 || selected_text.chars().any(|ch| ".!?;,:".contains(ch));

    Ok(TextAlternativesRequest {
        scene_id: input.scene_id,
        selected_text: selected_text.to_string(),
        context_before: excerpt(input.context_before.trim(), 700),
        context_after: excerpt(input.context_after.trim(), 700),
        is_phrase,
    })
}

fn build_improve_text_request(
    connection: &Connection,
    input: ImproveTextInput,
) -> Result<TextImprovementRequest, AppError> {
    ensure_active_scene(connection, &input.scene_id)?;
    let mode = match input.mode.trim() {
        "selection" => "selection",
        "scene" => "scene",
        _ => {
            return Err(AppError::new(
                "invalid_improve_text_mode",
                "Der Textverbesserungsmodus ist ungueltig.",
            ));
        }
    };

    let selected_text = if mode == "selection" {
        let text = input.selected_text.trim();
        if text.is_empty() {
            return Err(AppError::new(
                "empty_text_selection",
                "Bitte markiere zuerst Text im Szeneneditor.",
            ));
        }

        text.to_string()
    } else {
        let scene_text: String = connection.query_row(
            "
            SELECT text
            FROM scene
            WHERE id = ?1 AND deleted_at IS NULL
            ",
            params![input.scene_id],
            |row| row.get(0),
        )?;
        let text = scene_text.trim();
        if text.is_empty() {
            return Err(AppError::new(
                "empty_scene_text",
                "Die Szene braucht Text, bevor sie verbessert werden kann.",
            ));
        }

        excerpt(text, 6000)
    };

    Ok(TextImprovementRequest {
        scene_id: input.scene_id,
        selected_text,
        context_before: excerpt(input.context_before.trim(), 900),
        context_after: excerpt(input.context_after.trim(), 900),
        mode: mode.to_string(),
    })
}

fn read_project_bible_context_card(
    connection: &Connection,
    target_type: &str,
    target_id: &str,
) -> Result<Option<String>, AppError> {
    match target_type {
        "character" => connection
            .query_row(
                "
                SELECT name, role, description, notes
                FROM character
                WHERE id = ?1 AND deleted_at IS NULL
                ",
                params![target_id],
                |row| {
                    Ok(format!(
                        "Figur: {}\nRolle: {}\nBeschreibung: {}\nNotizen: {}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?
                    ))
                },
            )
            .optional()
            .map_err(AppError::from),
        "location" => connection
            .query_row(
                "
                SELECT name, description, notes
                FROM location
                WHERE id = ?1 AND deleted_at IS NULL
                ",
                params![target_id],
                |row| {
                    Ok(format!(
                        "Ort: {}\nBeschreibung: {}\nNotizen: {}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ))
                },
            )
            .optional()
            .map_err(AppError::from),
        "item" => connection
            .query_row(
                "
                SELECT name, description, notes
                FROM item
                WHERE id = ?1 AND deleted_at IS NULL
                ",
                params![target_id],
                |row| {
                    Ok(format!(
                        "Gegenstand: {}\nBeschreibung: {}\nNotizen: {}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ))
                },
            )
            .optional()
            .map_err(AppError::from),
        "note" => connection
            .query_row(
                "
                SELECT title, note_type, body
                FROM note
                WHERE id = ?1 AND deleted_at IS NULL
                ",
                params![target_id],
                |row| {
                    Ok(format!(
                        "Notiz: {}\nTyp: {}\nInhalt: {}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?
                    ))
                },
            )
            .optional()
            .map_err(AppError::from),
        _ => Ok(None),
    }
}

fn call_openai_compatible_chat(
    settings: &AiSettings,
    request: &AiSceneRequest,
) -> Result<String, AppError> {
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let prompt = build_summarize_scene_prompt(request);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": [
            {
                "role": "system",
                "content": "Du bist eine deutsche Schreibassistenz fuer Romanautoren. Nutze nur den gegebenen Szenentext und Kontext. Erfinde keine neuen Fakten."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.2,
        "stream": false,
        "max_tokens": 220
    }));

    let response = request_builder.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Der lokale KI-Dienst antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    let content = parsed["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if content.is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keine Zusammenfassung geliefert.",
        ));
    }

    Ok(content)
}

fn call_openai_compatible_contradictions(
    settings: &AiSettings,
    request: &AiContradictionRequest,
) -> Result<String, AppError> {
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let prompt = build_check_contradictions_prompt(request);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": [
            {
                "role": "system",
                "content": "Du bist eine deutsche Kontinuitaetsassistenz fuer Romanautoren. Pruefe nur den gegebenen Szenentext gegen den gegebenen Projektkontext. Erfinde keine Fakten und formuliere Unsicherheiten klar."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.1,
        "stream": false,
        "max_tokens": 520
    }));

    let response = request_builder.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Der lokale KI-Dienst antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    let content = clean_ai_chat_content(
        parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or(""),
    );

    if content.trim().is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keine Widerspruchspruefung geliefert.",
        ));
    }

    Ok(content.trim().to_string())
}

fn build_check_contradictions_prompt(request: &AiContradictionRequest) -> String {
    format!(
        "Pruefe die aktuelle Szene auf moegliche Widersprueche, Unklarheiten oder fehlende Anschlussstellen gegen den verknuepften Projektkontext.\n\
        Begrenzung:\n\
        - Pruefe nur diese Szene und den unten gegebenen Projektkontext.\n\
        - Keine globale Romananalyse.\n\
        - Keine neuen Fakten erfinden.\n\
        - Melde nur konkrete Auffaelligkeiten oder schreibe: \"Keine konkreten Widersprueche gefunden.\"\n\
        - Wenn etwas nur unsicher ist, markiere es als Unsicherheit.\n\n\
        Ausgabeformat:\n\
        - Kurze Markdown-Liste.\n\
        - Pro Punkt: Kategorie, Befund, Bezug aus Szene, Bezug aus Projektkontext.\n\
        - Maximal 5 Punkte.\n\n\
        Szenentitel: {}\n\n\
        Kurzbeschreibung: {}\n\n\
        Projektkontext:\n{}\n\n\
        Szenentext:\n{}",
        request.scene_title,
        if request.scene_summary.trim().is_empty() {
            "Keine Kurzbeschreibung"
        } else {
            request.scene_summary.trim()
        },
        request.context_summary,
        request.scene_text
    )
}

fn build_text_alternatives_prompt(request: &TextAlternativesRequest) -> String {
    let task = if request.is_phrase {
        "Erzeuge 3 bis 5 umformulierte Alternativen fuer den markierten Satz oder Abschnitt."
    } else {
        "Erzeuge 5 kurze Synonyme oder sehr kurze Alternativen fuer das markierte Wort."
    };

    format!(
        "{task}\n\nKontext davor:\n{}\n\nMarkierung:\n{}\n\nKontext danach:\n{}\n\nAntworte ausschliesslich als JSON-Array aus deutschen Strings. Keine Erklaerungen, keine Nummerierung, keine Analyse, kein Reasoning, keine Gedanken, keine neuen Fakten.",
        request.context_before, request.selected_text, request.context_after
    )
}

fn build_text_improvement_prompt(request: &TextImprovementRequest) -> String {
    let scope = if request.mode == "selection" {
        "Verbessere die markierte Textstelle."
    } else {
        "Verbessere die gesamte Szene."
    };

    format!(
        "{scope}\n\nZiel: Erzeuge 2 bis 3 verbesserte Fassungen. Der Text soll klarer, fluessiger und stilistisch sauberer werden. Reduziere Wiederholungen und holprige Formulierungen. Bewahre Inhalt, Ton, Perspektive, Fakten und Figuren. Erfinde keine neuen Details.\n\nKontext davor:\n{}\n\nZu verbessernder Text:\n{}\n\nKontext danach:\n{}\n\nAntworte ausschliesslich als JSON-Array aus deutschen Strings. Keine Erklaerungen, keine Nummerierung, keine Analyse, kein Reasoning, keine Gedanken, keine Markdown-Codezaeune.",
        request.context_before, request.selected_text, request.context_after
    )
}

fn parse_text_alternatives_response(content: &str) -> Result<Vec<String>, AppError> {
    let cleaned = strip_ai_reasoning_blocks(&clean_ai_chat_content(content));
    let json_candidate = cleaned
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Some(alternatives) = parse_text_alternatives_json(json_candidate) {
        return Ok(alternatives);
    }

    if let Some(candidate) = extract_json_array(json_candidate) {
        if let Some(alternatives) = parse_text_alternatives_json(candidate) {
            return Ok(alternatives);
        }
    }

    let alternatives = normalize_text_alternatives(
        cleaned
            .lines()
            .filter(|line| !looks_like_reasoning_line(line))
            .map(|line| {
                line.trim()
                    .trim_start_matches(|ch: char| {
                        ch.is_ascii_digit()
                            || ch == '.'
                            || ch == ')'
                            || ch == '-'
                            || ch == '*'
                            || ch.is_whitespace()
                    })
                    .trim_matches('"')
                    .trim()
                    .to_string()
            })
            .collect(),
    );

    if alternatives.is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keine verwertbaren Alternativen geliefert.",
        ));
    }

    Ok(alternatives)
}

fn parse_text_improvements_response(content: &str) -> Result<Vec<String>, AppError> {
    let mut improvements = parse_text_alternatives_response(content)?;
    improvements.truncate(3);
    if improvements.is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keine verwertbaren Textverbesserungen geliefert.",
        ));
    }

    Ok(improvements)
}

fn parse_text_alternatives_json(candidate: &str) -> Option<Vec<String>> {
    if let Ok(values) = serde_json::from_str::<Vec<String>>(candidate) {
        let alternatives = normalize_text_alternatives(values);
        if !alternatives.is_empty() {
            return Some(alternatives);
        }
    }

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(candidate) {
        if let Some(values) = parsed
            .get("alternatives")
            .and_then(|value| value.as_array())
        {
            let alternatives = normalize_text_alternatives(
                values
                    .iter()
                    .filter_map(|value| value.as_str().map(ToString::to_string))
                    .collect(),
            );
            if !alternatives.is_empty() {
                return Some(alternatives);
            }
        }
    }

    None
}

fn extract_json_array(content: &str) -> Option<&str> {
    let start = content.find('[')?;
    let end = content.rfind(']')?;
    if start >= end {
        return None;
    }

    Some(&content[start..=end])
}

fn strip_ai_reasoning_blocks(content: &str) -> String {
    let mut cleaned = content.to_string();
    for tag in ["reasoning", "think"] {
        loop {
            let lower = cleaned.to_lowercase();
            let Some(start) = lower.find(&format!("<{tag}>")) else {
                break;
            };
            let end_tag = format!("</{tag}>");
            if let Some(relative_end) = lower[start..].find(&end_tag) {
                let end = start + relative_end + end_tag.len();
                cleaned.replace_range(start..end, "");
            } else {
                cleaned.replace_range(start.., "");
                break;
            }
        }
    }

    cleaned.trim().to_string()
}

fn looks_like_reasoning_line(line: &str) -> bool {
    let lower = line.trim().to_lowercase();
    lower.is_empty()
        || lower.contains("<reasoning")
        || lower.contains("</reasoning")
        || lower.contains("<think")
        || lower.contains("</think")
        || lower.starts_with("i need")
        || lower.starts_with("first,")
        || lower.starts_with("possible synonyms")
        || lower.contains("json format")
        || lower.contains("the user wants")
        || lower.contains("i'll analyze")
}

fn normalize_text_alternatives(values: Vec<String>) -> Vec<String> {
    let mut alternatives = Vec::new();
    for value in values {
        let alternative = value.trim();
        if alternative.is_empty()
            || looks_like_reasoning_line(alternative)
            || alternatives.iter().any(|item| item == alternative)
        {
            continue;
        }
        alternatives.push(alternative.to_string());
        if alternatives.len() >= 5 {
            break;
        }
    }

    alternatives
}

fn call_openai_compatible_text_alternatives(
    settings: &AiSettings,
    request: &TextAlternativesRequest,
) -> Result<Vec<String>, AppError> {
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let prompt = build_text_alternatives_prompt(request);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": [
            {
                "role": "system",
                "content": "Du bist eine deutsche Schreibassistenz. Liefere nur finale deutsche Synonyme oder Umformulierungen. Bewahre Ton, Bedeutung und Fakten. Gib keine Analyse, kein Reasoning, keine Gedanken und keine englischen Erklaerungen aus. Antworte ausschliesslich im verlangten JSON-Format."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.55,
        "stream": false,
        "max_tokens": 300,
        "stop": ["<|im_end|>", "<|im_start|>", "<|endoftext|>"]
    }));

    let response = request_builder.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Der lokale KI-Dienst antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    parse_text_alternatives_response(
        parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or(""),
    )
}

fn call_openai_compatible_text_improvements(
    settings: &AiSettings,
    request: &TextImprovementRequest,
) -> Result<Vec<String>, AppError> {
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let prompt = build_text_improvement_prompt(request);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": [
            {
                "role": "system",
                "content": "Du bist eine deutsche Schreibassistenz fuer Romanautoren. Verbessere Text behutsam und textnah. Bewahre Inhalt, Ton, Perspektive und Fakten. Erfinde keine neuen Details. Gib keine Analyse, kein Reasoning, keine Gedanken und keine englischen Erklaerungen aus. Antworte ausschliesslich im verlangten JSON-Format."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.45,
        "stream": false,
        "max_tokens": 1200,
        "stop": ["<|im_end|>", "<|im_start|>", "<|endoftext|>"]
    }));

    let response = request_builder.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Der lokale KI-Dienst antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    parse_text_improvements_response(
        parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or(""),
    )
}

fn normalize_chat_messages(input: &SendAiChatMessageInput) -> Result<Vec<AiChatMessage>, AppError> {
    let mut messages = Vec::new();

    for message in &input.messages {
        let role = message.role.trim();
        let content = message.content.trim();
        if content.is_empty() {
            continue;
        }

        if role != "user" && role != "assistant" {
            return Err(AppError::new(
                "invalid_ai_chat_role",
                "Der KI-Chat unterstuetzt nur Nutzer- und Assistenznachrichten.",
            ));
        }

        messages.push(AiChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    if messages
        .last()
        .filter(|message| message.role == "user")
        .is_none()
    {
        return Err(AppError::new(
            "empty_ai_chat_message",
            "Bitte gib eine Nachricht fuer den KI-Chat ein.",
        ));
    }

    Ok(messages
        .into_iter()
        .rev()
        .take(24)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect())
}

fn normalize_ai_chat_mode(input: &SendAiChatMessageInput) -> Result<&'static str, AppError> {
    match input.mode.trim() {
        AI_CHAT_MODE_IDEAS => Ok(AI_CHAT_MODE_IDEAS),
        AI_CHAT_MODE_WRITING => Ok(AI_CHAT_MODE_WRITING),
        _ => Err(AppError::new(
            "invalid_ai_chat_mode",
            "Der KI-Chat-Modus wird nicht unterstuetzt.",
        )),
    }
}

fn ai_chat_system_prompt(mode: &str) -> &'static str {
    match mode {
        AI_CHAT_MODE_WRITING => {
            "Du bist eine deutsche Schreibassistenz fuer Romanautoren im Modus Schreibassistent. Deine Aufgabe ist konkrete Textarbeit: Szenen erweitern, umschreiben, fortsetzen, verdichten, Dialog schreiben, Stil anpassen und Prosa liefern. Wenn die letzte Nutzeranfrage eine konkrete Schreibanweisung ist, z.B. erweitere, schreibe, ueberarbeite, formuliere um, mach melancholischer, dramatischer oder aehnlich, dann liefere direkt den neuen oder ueberarbeiteten Text. Gib dann keine nummerierte Liste, keine allgemeinen Tipps, keine Methodenerklaerung und keine Meta-Einleitung aus. Wiederhole keine Kontextbloecke wie 'Aktuelle Szene', 'Titel', 'Status' oder 'Kurzbeschreibung'. Bleibe textnah: bewahre Figuren, Ort, Situation und Fakten, fuege keine neuen Namen, Beziehungen oder Orte hinzu, wenn sie nicht im Kontext stehen. Setze Stimmung dosiert ein; vermeide wiederholte Motive, Pathos-Kippmomente, unpassende Neubeginns-Rhetorik und mehrfach wiederholte Saetze. Wenn der Nutzer mit 'die Szene' oder 'dahingehend' auf vorherige Nachrichten verweist, beziehe dich auf die aktuelle Szene und die letzte Stilvorgabe. Nutze vorhandenen Kontext, erfinde aber keine Projektfakten. Gib keine ChatML-, Rollen- oder Stop-Tokens aus."
        }
        _ => {
            "Du bist eine deutsche Schreibassistenz fuer Romanautoren im Modus Ideenberater. Deine Aufgabe ist Beratung: analysiere Schreibprobleme, liefere kreative Optionen, Strukturideen, Varianten, Fragen und konkrete Ansatzpunkte. Du darfst Listen verwenden, wenn sie hilfreich sind. Schreibe nur dann laengere Manuskriptprosa, wenn der Nutzer ausdruecklich einen Textentwurf verlangt. Nutze vorhandenen Kontext, erfinde aber keine Projektfakten. Gib keine ChatML-, Rollen- oder Stop-Tokens aus."
        }
    }
}

fn clean_ai_chat_content(content: &str) -> String {
    content
        .replace("<|im_end|>", "")
        .replace("<|im_start|>", "")
        .replace("<|endoftext|>", "")
        .trim()
        .to_string()
}

fn call_openai_compatible_free_chat(
    settings: &AiSettings,
    input: &SendAiChatMessageInput,
    scene_context: Option<&str>,
) -> Result<String, AppError> {
    let mode = normalize_ai_chat_mode(input)?;
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": ai_chat_system_prompt(mode)
    })];

    if let Some(context) = scene_context.filter(|context| !context.trim().is_empty()) {
        messages.push(serde_json::json!({
            "role": "system",
            "content": format!("Aktueller Szenenkontext. Wenn der Nutzer eine Szene erweitern oder ueberarbeiten will, nutze diesen Text als Arbeitsgrundlage und gib eine konkrete neue Fassung oder Erweiterung aus:\n\n{context}")
        }));
    }

    for message in normalize_chat_messages(input)? {
        messages.push(serde_json::json!({
            "role": message.role,
            "content": message.content,
        }));
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": messages,
        "temperature": 0.7,
        "stream": false,
        "max_tokens": 1200,
        "stop": ["<|im_end|>", "<|im_start|>", "<|endoftext|>"]
    }));

    let response = request_builder.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Der lokale KI-Dienst antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    let content = clean_ai_chat_content(
        parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or(""),
    );

    if content.is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keine Chat-Antwort geliefert.",
        ));
    }

    Ok(content)
}

#[cfg(test)]
fn summarize_scene_with_provider(
    connection: &Connection,
    input: &SummarizeSceneInput,
    provider: impl FnOnce(&AiSceneRequest) -> Result<String, AppError>,
) -> Result<AiSuggestionSnapshot, AppError> {
    let request = build_ai_scene_request(connection, input)?;
    let result = provider(&request)?;

    insert_ai_suggestion(connection, &request, result)
}

#[cfg(test)]
fn check_scene_contradictions_with_provider(
    connection: &Connection,
    input: &SummarizeSceneInput,
    provider: impl FnOnce(&AiContradictionRequest) -> Result<String, AppError>,
) -> Result<AiSuggestionSnapshot, AppError> {
    let request = build_ai_contradiction_request(connection, input)?;
    let result = provider(&request)?;

    insert_ai_contradiction_suggestion(connection, &request, result)
}

#[cfg(test)]
fn generate_bible_card_with_provider(
    connection: &Connection,
    input: GenerateBibleCardInput,
    provider: impl FnOnce(&AiBibleRequest) -> Result<AiBibleCardDraft, AppError>,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let request = build_ai_bible_generate_request(connection, input)?;
    let draft = provider(&request)?;

    insert_ai_bible_suggestion(connection, &request, draft)
}

#[cfg(test)]
fn complete_bible_card_with_provider(
    connection: &Connection,
    input: CompleteBibleCardInput,
    provider: impl FnOnce(&AiBibleRequest) -> Result<AiBibleCardDraft, AppError>,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let request = build_ai_bible_complete_request(connection, input)?;
    let draft = provider(&request)?;

    insert_ai_bible_suggestion(connection, &request, draft)
}

fn build_summarize_scene_prompt(request: &AiSceneRequest) -> String {
    format!(
        "Fasse die folgende Szene fuer das Feld Kurzbeschreibung zusammen.\n\
        Anforderungen:\n\
        - Deutsch.\n\
        - 2 bis 4 kurze Saetze.\n\
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

fn insert_ai_suggestion(
    connection: &Connection,
    request: &AiSceneRequest,
    result: String,
) -> Result<AiSuggestionSnapshot, AppError> {
    insert_ai_suggestion_record(
        connection,
        &request.project_id,
        &request.scene_id,
        AI_ACTION_SUMMARIZE_SCENE,
        &request.scene_text,
        &request.context_summary,
        result,
        "Der lokale KI-Dienst hat keine Zusammenfassung geliefert.",
    )
}

fn insert_ai_contradiction_suggestion(
    connection: &Connection,
    request: &AiContradictionRequest,
    result: String,
) -> Result<AiSuggestionSnapshot, AppError> {
    insert_ai_suggestion_record(
        connection,
        &request.project_id,
        &request.scene_id,
        AI_ACTION_CHECK_SCENE_CONTRADICTIONS,
        &request.scene_text,
        &request.context_summary,
        result,
        "Der lokale KI-Dienst hat keine Widerspruchspruefung geliefert.",
    )
}

fn insert_ai_suggestion_record(
    connection: &Connection,
    project_id: &str,
    scene_id: &str,
    action_type: &str,
    input_text: &str,
    context_summary: &str,
    result: String,
    empty_message: &str,
) -> Result<AiSuggestionSnapshot, AppError> {
    ensure_active_scene(connection, scene_id)?;
    let now = timestamp();
    let id = Uuid::new_v4().to_string();
    let input_excerpt = excerpt(input_text, 800);
    let result = result.trim();
    if result.is_empty() {
        return Err(AppError::new("ai_empty_response", empty_message));
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
            project_id,
            scene_id,
            action_type,
            input_excerpt,
            context_summary,
            result,
            AI_STATUS_DRAFT,
            now
        ],
    )?;

    read_ai_suggestion(connection, &id)
}

fn read_ai_suggestion(
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

fn read_ai_suggestions_for_scene(
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

fn map_ai_suggestion_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AiSuggestionSnapshot> {
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

fn accept_ai_suggestion_in_connection(
    connection: &mut Connection,
    input: AiSuggestionInput,
) -> Result<AiSuggestionSnapshot, AppError> {
    let suggestion = read_ai_suggestion(connection, &input.id)?;
    if suggestion.action_type != AI_ACTION_SUMMARIZE_SCENE {
        return Err(AppError::new(
            "ai_action_not_acceptible",
            "Dieser KI-Vorschlag kann hier nicht uebernommen werden.",
        ));
    }

    if suggestion.status != AI_STATUS_DRAFT {
        return Err(AppError::new(
            "ai_suggestion_not_draft",
            "Nur offene KI-Vorschlaege koennen uebernommen werden.",
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

fn update_ai_suggestion_status(
    connection: &Connection,
    suggestion_id: &str,
    status: &str,
) -> Result<AiSuggestionSnapshot, AppError> {
    let suggestion = read_ai_suggestion(connection, suggestion_id)?;
    if suggestion.status != AI_STATUS_DRAFT {
        return Err(AppError::new(
            "ai_suggestion_not_draft",
            "Nur offene KI-Vorschlaege koennen verworfen werden.",
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

fn build_ai_bible_generate_request(
    connection: &Connection,
    input: GenerateBibleCardInput,
) -> Result<AiBibleRequest, AppError> {
    validate_target_type(&input.target_type)?;
    let keywords = normalize_keywords(&input.keywords)?;

    Ok(AiBibleRequest {
        project_id: read_active_project_id(connection)?,
        target_type: input.target_type,
        target_id: None,
        mode: AI_BIBLE_MODE_GENERATE.to_string(),
        keywords,
        existing_card_summary: "Keine bestehende Karte.".to_string(),
    })
}

fn build_ai_bible_complete_request(
    connection: &Connection,
    input: CompleteBibleCardInput,
) -> Result<AiBibleRequest, AppError> {
    validate_target_type(&input.target_type)?;
    ensure_active_project_bible_card(connection, &input.target_type, &input.target_id)?;
    let keywords = normalize_keywords(&input.keywords)?;

    Ok(AiBibleRequest {
        project_id: read_active_project_id(connection)?,
        target_type: input.target_type.clone(),
        target_id: Some(input.target_id.clone()),
        mode: AI_BIBLE_MODE_COMPLETE.to_string(),
        keywords,
        existing_card_summary: read_project_bible_context_card(
            connection,
            &input.target_type,
            &input.target_id,
        )?
        .unwrap_or_else(|| "Keine bestehende Karte.".to_string()),
    })
}

fn call_openai_compatible_bible_chat(
    settings: &AiSettings,
    request: &AiBibleRequest,
) -> Result<AiBibleCardDraft, AppError> {
    if is_local_ollama_default(&settings.base_url) {
        return call_ollama_bible_chat(settings, request);
    }

    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let prompt = build_bible_card_prompt(request);
    let temperature = if request.mode == AI_BIBLE_MODE_GENERATE {
        0.75
    } else {
        0.4
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let request_builder = client.post(endpoint).json(&serde_json::json!({
        "model": settings.model,
        "messages": [
            {
                "role": "system",
                "content": "Du bist eine deutsche Schreibassistenz fuer Fantasy- und Science-Fiction-Autoren. Entwickle aus knappen Stichworten konkrete, atmosphaerische Projektbibel-Daten. Bleibe plausibel und kontrolliert, aber kopiere Stichworte nicht nur um. Antworte ohne Thinking, reasoning oder Analyse."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": temperature,
        "stream": false,
        "max_tokens": 900,
        "response_format": { "type": "json_object" }
    }));

    let response = request_builder.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Der lokale KI-Dienst antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    let message = &parsed["choices"][0]["message"];
    let content = message["content"].as_str().unwrap_or("").trim();

    if content.is_empty() {
        if message["reasoning"]
            .as_str()
            .unwrap_or("")
            .trim()
            .is_empty()
            && message["reasoning_content"]
                .as_str()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            return Err(AppError::new(
                "ai_empty_response",
                "Der lokale KI-Dienst hat keinen Kartenentwurf geliefert.",
            ));
        }

        return Err(AppError::new(
            "ai_empty_response",
            "Das lokale KI-Modell hat nur Denktext, aber keinen Kartenentwurf geliefert. Bitte erneut versuchen oder ein Nicht-Reasoning-Modell waehlen.",
        ));
    }

    parse_ai_bible_draft(&request.target_type, content)
}

fn call_ollama_bible_chat(
    settings: &AiSettings,
    request: &AiBibleRequest,
) -> Result<AiBibleCardDraft, AppError> {
    let endpoint = format!("{}/api/chat", ollama_root_url(&settings.base_url));
    let prompt = build_bible_card_prompt(request);
    let temperature = if request.mode == AI_BIBLE_MODE_GENERATE {
        0.75
    } else {
        0.4
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let response = client
        .post(endpoint)
        .json(&serde_json::json!({
            "model": settings.model,
            "messages": [
                {
                    "role": "system",
                    "content": "Du bist eine deutsche Schreibassistenz fuer Fantasy- und Science-Fiction-Autoren. Entwickle aus knappen Stichworten konkrete, atmosphaerische Projektbibel-Daten. Bleibe plausibel und kontrolliert, aber kopiere Stichworte nicht nur um. Antworte nur mit validem JSON."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "think": false,
            "format": "json",
            "stream": false,
            "options": {
                "temperature": temperature,
                "num_predict": 900,
                "stop": ["\n```"]
            }
        }))
        .send()?;

    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::new(
                "ai_model_missing",
                "Das konfigurierte KI-Modell wurde vom lokalen Dienst nicht gefunden.",
            ));
        }

        return Err(AppError::new(
            "ai_provider_error",
            format!("Ollama antwortete mit Status {status}: {body}"),
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    let content = parsed["message"]["content"].as_str().unwrap_or("").trim();

    if content.is_empty() {
        return Err(AppError::new(
            "ai_empty_response",
            "Der lokale KI-Dienst hat keinen Kartenentwurf geliefert.",
        ));
    }

    parse_ai_bible_draft(&request.target_type, content)
}

fn build_bible_card_prompt(request: &AiBibleRequest) -> String {
    let schema_hint = match request.target_type.as_str() {
        "character" => r#"{"name":"...","role":"...","description":"...","notes":"..."}"#,
        "location" | "item" => r#"{"name":"...","description":"...","notes":"..."}"#,
        "note" => {
            r#"{"title":"...","body":"...","noteType":"general|research|lore|faction|event|todo"}"#
        }
        _ => "{}",
    };
    let mode_hint = if request.mode == AI_BIBLE_MODE_COMPLETE {
        "Vervollstaendige die bestehende Karte. Schlage vor allem fehlende Informationen vor. Bewahre vorhandene Fakten und Ton."
    } else {
        "Erzeuge genau eine neue Projektbibel-Karte. Interpretiere die Stichworte kreativ und mache daraus eine brauchbare Romanfigur, einen Ort, Gegenstand oder eine Notiz."
    };
    let craft_hint = match request.target_type.as_str() {
        "character" => {
            "- Name: Wenn nur Herkunft, Klang oder Typ genannt ist, erfinde einen passenden Eigennamen; nutze niemals Adjektive wie Brutale als Namen.\n\
        - Rolle: kurz und funktional, z.B. Antagonist, Mentorin, Schmugglerkoenigin.\n\
        - Beschreibung: 2 bis 4 konkrete Saetze mit Aussehen, Auftreten, innerem Antrieb und Konfliktpotenzial.\n\
        - Notizen: 2 bis 4 kurze, autorennuetliche Details wie Geheimnis, Schwachpunkt, Beziehung, Motiv oder wiederkehrendes Merkmal."
        }
        "location" => {
            "- Name: einen praegnanten Ortsnamen erfinden, falls keiner genannt ist.\n\
        - Beschreibung: 2 bis 4 konkrete Saetze mit Atmosphaere, Funktion im Roman und sichtbaren Details.\n\
        - Notizen: 2 bis 4 kurze Details zu Geschichte, Gefahr, Bewohnern, Geheimnis oder Szenenpotenzial."
        }
        "item" => {
            "- Name: einen praegnanten Gegenstandsnamen erfinden, falls keiner genannt ist.\n\
        - Beschreibung: 2 bis 4 konkrete Saetze mit Aussehen, Funktion, Herkunft und erzählerischem Nutzen.\n\
        - Notizen: 2 bis 4 kurze Details zu Preis, Risiko, Besitzer, Geheimnis oder Begrenzung."
        }
        "note" => {
            "- Titel: kurz und eindeutig.\n\
        - Inhalt: 3 bis 6 konkrete Saetze, die aus den Stichworten nutzbares Projektwissen machen.\n\
        - noteType: passend aus general, research, lore, faction, event oder todo wählen."
        }
        _ => "",
    };

    format!(
        "/no_think\n\
        {mode_hint}\n\
        Anforderungen:\n\
        - Deutsch.\n\
        - Genau ein JSON-Objekt ausgeben, ohne Markdown und ohne Erklaerung.\n\
        - Keine Analyse, kein Denktext, kein reasoning.\n\
        - Schema exakt passend zum Kartentyp verwenden: {schema_hint}\n\
        - Leere Felder als leere Strings ausgeben.\n\
        - Keine Liste und keine zweite Karte erzeugen.\n\
        - Wiederhole die Stichworte nicht mechanisch; verdichte, konkretisiere und ergaenze plausibel.\n\
        - Keine Meta-Kommentare wie \"aus den Stichworten\".\n\n\
        Ausarbeitungsregeln:\n{craft_hint}\n\n\
        Kartentyp: {}\n\n\
        Bestehende Karte:\n{}\n\n\
        Stichworte:\n{}",
        request.target_type, request.existing_card_summary, request.keywords
    )
}

fn parse_ai_bible_draft(target_type: &str, content: &str) -> Result<AiBibleCardDraft, AppError> {
    let value = parse_bible_json_value(content)?;
    if !value.is_object() {
        return Err(AppError::new(
            "ai_invalid_response",
            "Der lokale KI-Dienst hat keinen gueltigen Kartenentwurf geliefert.",
        ));
    }

    let mut draft = AiBibleCardDraft::default();
    match target_type {
        "character" => {
            draft.name = json_string(&value, "name");
            draft.role = json_string(&value, "role");
            draft.description = json_string(&value, "description");
            draft.notes = json_string(&value, "notes");
        }
        "location" | "item" => {
            draft.name = json_string(&value, "name");
            draft.description = json_string(&value, "description");
            draft.notes = json_string(&value, "notes");
        }
        "note" => {
            draft.title = json_string(&value, "title");
            draft.body = json_string(&value, "body");
            draft.note_type = normalize_note_type(
                value["noteType"]
                    .as_str()
                    .or_else(|| value["note_type"].as_str()),
            );
        }
        _ => {
            return Err(AppError::new(
                "invalid_project_bible_card_type",
                "Dieser Projektbibel-Kartentyp wird nicht unterstuetzt.",
            ));
        }
    }

    Ok(normalize_ai_bible_draft(target_type, draft))
}

fn parse_bible_json_value(content: &str) -> Result<serde_json::Value, AppError> {
    let stripped = strip_json_code_fence(content);
    let candidate = extract_first_json_object(stripped).unwrap_or(stripped);
    match serde_json::from_str(candidate) {
        Ok(value) => Ok(value),
        Err(_) => {
            let repaired = quote_known_json_keys(candidate);
            serde_json::from_str(&repaired).map_err(|_| {
                AppError::new(
                    "ai_invalid_response",
                    "Der lokale KI-Dienst hat keinen gueltigen JSON-Kartenentwurf geliefert. Bitte erneut versuchen oder ein anderes Modell waehlen.",
                )
            })
        }
    }
}

fn extract_first_json_object(content: &str) -> Option<&str> {
    let mut depth = 0;
    let mut start = None;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in content.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        if character == '"' {
            in_string = true;
        } else if character == '{' {
            if depth == 0 {
                start = Some(index);
            }
            depth += 1;
        } else if character == '}' && depth > 0 {
            depth -= 1;
            if depth == 0 {
                return start.map(|start| &content[start..=index]);
            }
        }
    }

    None
}

fn quote_known_json_keys(content: &str) -> String {
    let mut repaired = content.to_string();
    for key in [
        "name",
        "role",
        "description",
        "notes",
        "title",
        "body",
        "noteType",
        "note_type",
    ] {
        for prefix in ["{", ","] {
            repaired = repaired.replace(&format!("{prefix}{key}:"), &format!("{prefix}\"{key}\":"));
            repaired =
                repaired.replace(&format!("{prefix} {key}:"), &format!("{prefix} \"{key}\":"));
            repaired = repaired.replace(
                &format!("{prefix}\n  {key}:"),
                &format!("{prefix}\n  \"{key}\":"),
            );
            repaired = repaired.replace(
                &format!("{prefix}\n    {key}:"),
                &format!("{prefix}\n    \"{key}\":"),
            );
        }
    }

    repaired
}

fn strip_json_code_fence(content: &str) -> &str {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed;
    }

    let without_opening = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```JSON")
        .trim_start_matches("```")
        .trim();

    without_opening
        .strip_suffix("```")
        .unwrap_or(without_opening)
        .trim()
}

fn json_string(value: &serde_json::Value, key: &str) -> String {
    value[key].as_str().unwrap_or("").trim().to_string()
}

fn normalize_ai_bible_draft(target_type: &str, mut draft: AiBibleCardDraft) -> AiBibleCardDraft {
    draft.name = draft.name.trim().to_string();
    draft.role = draft.role.trim().to_string();
    draft.description = draft.description.trim().to_string();
    draft.notes = draft.notes.trim().to_string();
    draft.title = draft.title.trim().to_string();
    draft.body = draft.body.trim().to_string();
    draft.note_type = normalize_note_type(Some(&draft.note_type));

    if target_type != "note" {
        draft.title.clear();
        draft.body.clear();
        draft.note_type = "general".to_string();
    }

    if target_type != "character" {
        draft.role.clear();
    }

    if target_type == "note" {
        draft.name.clear();
        draft.role.clear();
        draft.description.clear();
        draft.notes.clear();
    }

    draft
}

fn normalize_keywords(keywords: &str) -> Result<String, AppError> {
    let keywords = keywords.trim();
    if keywords.is_empty() {
        return Err(AppError::new(
            "ai_bible_keywords_empty",
            "Bitte gib Stichworte fuer die Projektbibel-KI ein.",
        ));
    }

    Ok(keywords.to_string())
}

fn insert_ai_bible_suggestion(
    connection: &Connection,
    request: &AiBibleRequest,
    draft: AiBibleCardDraft,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let now = timestamp();
    let id = Uuid::new_v4().to_string();
    let draft = normalize_ai_bible_draft(&request.target_type, draft);
    let result_json = serde_json::to_string(&draft)?;

    connection.execute(
        "
        INSERT INTO ai_bible_suggestion (
            id, project_id, target_type, target_id, mode, keywords, result_json, status,
            created_at, updated_at, deleted_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, NULL)
        ",
        params![
            id,
            request.project_id,
            request.target_type,
            request.target_id,
            request.mode,
            request.keywords,
            result_json,
            AI_STATUS_DRAFT,
            now
        ],
    )?;

    read_ai_bible_suggestion(connection, &id)
}

fn read_ai_bible_suggestion(
    connection: &Connection,
    suggestion_id: &str,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    connection
        .query_row(
            "
            SELECT id, project_id, target_type, target_id, mode, keywords, result_json,
                status, created_at, updated_at, deleted_at
            FROM ai_bible_suggestion
            WHERE id = ?1 AND deleted_at IS NULL
            ",
            params![suggestion_id],
            map_ai_bible_suggestion_row,
        )
        .optional()?
        .ok_or_else(|| {
            AppError::new(
                "ai_bible_suggestion_missing",
                "Der Projektbibel-KI-Vorschlag wurde nicht gefunden.",
            )
        })
}

fn read_ai_bible_suggestions(
    connection: &Connection,
    target_type: Option<&str>,
    target_id: Option<&str>,
) -> Result<Vec<AiBibleSuggestionSnapshot>, AppError> {
    if let Some(target_type) = target_type {
        validate_target_type(target_type)?;
    }

    let mut statement = connection.prepare(
        "
        SELECT id, project_id, target_type, target_id, mode, keywords, result_json,
            status, created_at, updated_at, deleted_at
        FROM ai_bible_suggestion
        WHERE deleted_at IS NULL
            AND (?1 IS NULL OR target_type = ?1)
            AND (
                ?2 IS NULL
                OR target_id = ?2
                OR (target_id IS NULL AND ?2 = '')
            )
        ORDER BY created_at DESC
        ",
    )?;

    let suggestions = statement
        .query_map(params![target_type, target_id], map_ai_bible_suggestion_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(suggestions)
}

fn map_ai_bible_suggestion_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<AiBibleSuggestionSnapshot> {
    let target_type: String = row.get(2)?;
    let result_json: String = row.get(6)?;
    let draft = serde_json::from_str::<AiBibleCardDraft>(&result_json)
        .map(|draft| normalize_ai_bible_draft(&target_type, draft))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;

    Ok(AiBibleSuggestionSnapshot {
        id: row.get(0)?,
        project_id: row.get(1)?,
        target_type,
        target_id: row.get(3)?,
        mode: row.get(4)?,
        keywords: row.get(5)?,
        draft,
        status: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        deleted_at: row.get(10)?,
    })
}

fn accept_ai_bible_suggestion_in_connection(
    connection: &mut Connection,
    input: AcceptAiBibleSuggestionInput,
) -> Result<(String, String), AppError> {
    let suggestion = read_ai_bible_suggestion(connection, &input.id)?;
    if suggestion.status != AI_STATUS_DRAFT {
        return Err(AppError::new(
            "ai_suggestion_not_draft",
            "Nur offene KI-Vorschlaege koennen uebernommen werden.",
        ));
    }

    validate_target_type(&suggestion.target_type)?;
    let draft = normalize_ai_bible_draft(&suggestion.target_type, input.draft);
    let now = timestamp();
    let transaction = connection.transaction()?;
    let target_id = if suggestion.mode == AI_BIBLE_MODE_GENERATE {
        create_project_bible_card_from_draft(&transaction, &suggestion, &draft, &now)?
    } else if suggestion.mode == AI_BIBLE_MODE_COMPLETE {
        let target_id = suggestion.target_id.clone().ok_or_else(|| {
            AppError::new(
                "ai_bible_target_missing",
                "Der Projektbibel-KI-Vorschlag hat keine Zielkarte.",
            )
        })?;
        complete_project_bible_card_from_draft(
            &transaction,
            &suggestion.target_type,
            &target_id,
            &draft,
            &now,
        )?;
        target_id
    } else {
        return Err(AppError::new(
            "ai_bible_mode_invalid",
            "Dieser Projektbibel-KI-Modus wird nicht unterstuetzt.",
        ));
    };

    transaction.execute(
        "
        UPDATE ai_bible_suggestion
        SET status = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        ",
        params![AI_STATUS_ACCEPTED, now, input.id],
    )?;
    transaction.commit()?;

    Ok((suggestion.target_type, target_id))
}

fn create_project_bible_card_from_draft(
    connection: &Connection,
    suggestion: &AiBibleSuggestionSnapshot,
    draft: &AiBibleCardDraft,
    now: &str,
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();

    match suggestion.target_type.as_str() {
        "character" => {
            connection.execute(
                "
                INSERT INTO character (
                    id, project_id, name, role, description, notes, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, NULL)
                ",
                params![
                    id,
                    suggestion.project_id,
                    normalize_title(Some(&draft.name), "Neue Figur"),
                    draft.role,
                    draft.description,
                    draft.notes,
                    now
                ],
            )?;
        }
        "location" => {
            connection.execute(
                "
                INSERT INTO location (
                    id, project_id, name, description, notes, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, NULL)
                ",
                params![
                    id,
                    suggestion.project_id,
                    normalize_title(Some(&draft.name), "Neuer Ort"),
                    draft.description,
                    draft.notes,
                    now
                ],
            )?;
        }
        "item" => {
            connection.execute(
                "
                INSERT INTO item (
                    id, project_id, name, description, notes, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, NULL)
                ",
                params![
                    id,
                    suggestion.project_id,
                    normalize_title(Some(&draft.name), "Neuer Gegenstand"),
                    draft.description,
                    draft.notes,
                    now
                ],
            )?;
        }
        "note" => {
            connection.execute(
                "
                INSERT INTO note (
                    id, project_id, title, body, note_type, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, NULL)
                ",
                params![
                    id,
                    suggestion.project_id,
                    normalize_title(Some(&draft.title), "Neue Notiz"),
                    draft.body,
                    normalize_note_type(Some(&draft.note_type)),
                    now
                ],
            )?;
        }
        _ => unreachable!(),
    }

    Ok(id)
}

fn complete_project_bible_card_from_draft(
    connection: &Connection,
    target_type: &str,
    target_id: &str,
    draft: &AiBibleCardDraft,
    now: &str,
) -> Result<(), AppError> {
    let changed = match target_type {
        "character" => {
            let current = connection
                .query_row(
                    "
                    SELECT name, role, description, notes
                    FROM character
                    WHERE id = ?1 AND deleted_at IS NULL
                    ",
                    params![target_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| {
                    AppError::new(
                        "project_bible_card_missing",
                        "Die Projektbibel-Karte wurde nicht gefunden.",
                    )
                })?;
            connection.execute(
                "
                UPDATE character
                SET name = ?1, role = ?2, description = ?3, notes = ?4, updated_at = ?5
                WHERE id = ?6 AND deleted_at IS NULL
                ",
                params![
                    fill_if_blank_or_default(&current.0, &draft.name, "Neue Figur"),
                    fill_if_blank(&current.1, &draft.role),
                    fill_if_blank(&current.2, &draft.description),
                    fill_if_blank(&current.3, &draft.notes),
                    now,
                    target_id
                ],
            )?
        }
        "location" => {
            let current = connection
                .query_row(
                    "
                    SELECT name, description, notes
                    FROM location
                    WHERE id = ?1 AND deleted_at IS NULL
                    ",
                    params![target_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| {
                    AppError::new(
                        "project_bible_card_missing",
                        "Die Projektbibel-Karte wurde nicht gefunden.",
                    )
                })?;
            connection.execute(
                "
                UPDATE location
                SET name = ?1, description = ?2, notes = ?3, updated_at = ?4
                WHERE id = ?5 AND deleted_at IS NULL
                ",
                params![
                    fill_if_blank_or_default(&current.0, &draft.name, "Neuer Ort"),
                    fill_if_blank(&current.1, &draft.description),
                    fill_if_blank(&current.2, &draft.notes),
                    now,
                    target_id
                ],
            )?
        }
        "item" => {
            let current = connection
                .query_row(
                    "
                    SELECT name, description, notes
                    FROM item
                    WHERE id = ?1 AND deleted_at IS NULL
                    ",
                    params![target_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| {
                    AppError::new(
                        "project_bible_card_missing",
                        "Die Projektbibel-Karte wurde nicht gefunden.",
                    )
                })?;
            connection.execute(
                "
                UPDATE item
                SET name = ?1, description = ?2, notes = ?3, updated_at = ?4
                WHERE id = ?5 AND deleted_at IS NULL
                ",
                params![
                    fill_if_blank_or_default(&current.0, &draft.name, "Neuer Gegenstand"),
                    fill_if_blank(&current.1, &draft.description),
                    fill_if_blank(&current.2, &draft.notes),
                    now,
                    target_id
                ],
            )?
        }
        "note" => {
            let current = connection
                .query_row(
                    "
                    SELECT title, body, note_type
                    FROM note
                    WHERE id = ?1 AND deleted_at IS NULL
                    ",
                    params![target_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| {
                    AppError::new(
                        "project_bible_card_missing",
                        "Die Projektbibel-Karte wurde nicht gefunden.",
                    )
                })?;
            connection.execute(
                "
                UPDATE note
                SET title = ?1, body = ?2, note_type = ?3, updated_at = ?4
                WHERE id = ?5 AND deleted_at IS NULL
                ",
                params![
                    fill_if_blank_or_default(&current.0, &draft.title, "Neue Notiz"),
                    fill_if_blank(&current.1, &draft.body),
                    current.2,
                    now,
                    target_id
                ],
            )?
        }
        _ => unreachable!(),
    };

    require_changed(
        changed,
        "project_bible_card_missing",
        "Die Projektbibel-Karte wurde nicht gefunden.",
    )
}

fn fill_if_blank(current: &str, proposed: &str) -> String {
    if current.trim().is_empty() {
        proposed.trim().to_string()
    } else {
        current.to_string()
    }
}

fn fill_if_blank_or_default(current: &str, proposed: &str, default_value: &str) -> String {
    if current.trim().is_empty() || current.trim() == default_value {
        normalize_title(Some(proposed), default_value)
    } else {
        current.to_string()
    }
}

fn update_ai_bible_suggestion_status(
    connection: &Connection,
    suggestion_id: &str,
    status: &str,
) -> Result<AiBibleSuggestionSnapshot, AppError> {
    let suggestion = read_ai_bible_suggestion(connection, suggestion_id)?;
    if suggestion.status != AI_STATUS_DRAFT {
        return Err(AppError::new(
            "ai_suggestion_not_draft",
            "Nur offene KI-Vorschlaege koennen verworfen werden.",
        ));
    }

    let now = timestamp();
    connection.execute(
        "
        UPDATE ai_bible_suggestion
        SET status = ?1, updated_at = ?2
        WHERE id = ?3 AND deleted_at IS NULL
        ",
        params![status, now, suggestion_id],
    )?;

    read_ai_bible_suggestion(connection, suggestion_id)
}

fn excerpt(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn normalize_project_path(path: &str) -> Result<PathBuf, AppError> {
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

fn absolute_project_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    env::current_dir()
        .map(|current_dir| current_dir.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn same_project_path(left: &Path, right: &Path) -> bool {
    let left = left
        .canonicalize()
        .unwrap_or_else(|_| absolute_project_path(left));
    let right = right
        .canonicalize()
        .unwrap_or_else(|_| absolute_project_path(right));

    left == right
}

fn normalize_existing_project_path(path: &str) -> Result<PathBuf, AppError> {
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

fn map_invalid_project_file(error: rusqlite::Error) -> AppError {
    AppError::new(
        "invalid_project_file",
        format!("Die Datei ist keine gueltige Autorenprojekt-Datei: {error}"),
    )
}

fn map_invalid_project_error(error: AppError) -> AppError {
    AppError::new(
        "invalid_project_file",
        format!(
            "Die Datei ist keine gueltige Autorenprojekt-Datei: {}",
            error.message
        ),
    )
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
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
                default_language: "de-DE".to_string(),
                editor_font_family: "monospace".to_string(),
                editor_font_size: 22,
            },
        )
        .unwrap();

        assert_eq!(updated.name, "Die blutigen drei");
        assert_eq!(updated.description, "Ein blutiger Fantasyroman");
        assert_eq!(updated.default_language, "de-DE");
        assert_eq!(updated.editor_font_family, "monospace");
        assert_eq!(updated.editor_font_size, 22);

        let reopened = open_project_file(created.path).unwrap();
        assert_eq!(reopened.name, "Die blutigen drei");
        assert_eq!(reopened.description, "Ein blutiger Fantasyroman");
        assert_eq!(reopened.default_language, "de-DE");
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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
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

        let scene_results =
            search_project_in_connection(&connection, "Suchwort", "scenes").unwrap();
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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
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
        export_manuscript_markdown_in_connection(&connection, export_path.to_str().unwrap())
            .unwrap();
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
        let archived_scene_id = read_manuscript_tree(&connection).unwrap().areas[0].chapters[0]
            .scenes[0]
            .id
            .clone();
        delete_scene_in_connection(
            &mut connection,
            DeleteSceneInput {
                id: archived_scene_id,
            },
        )
        .unwrap();

        let exported = build_markdown_export(&read_manuscript_tree(&connection).unwrap());

        assert!(!exported.contains("Soll verschwinden."));
        assert!(exported.contains("Bleibt sichtbar."));
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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
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
                description: Some("Oeffnet die Kammer".to_string()),
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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
        update_scene_in_connection(
            &mut connection,
            UpdateSceneInput {
                id: scene.id.clone(),
                title: "Turmszene".to_string(),
                text: "Mira betritt den alten Turm und findet eine verschlossene Kammer."
                    .to_string(),
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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
        update_scene_in_connection(
            &mut connection,
            UpdateSceneInput {
                id: scene.id.clone(),
                title: scene.title,
                text: "Mira oeffnet die Kammer.".to_string(),
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
            |_| Ok("Mira oeffnet eine verborgene Kammer.".to_string()),
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
            "Mira oeffnet eine verborgene Kammer."
        );
    }

    #[test]
    fn rejects_empty_scene_for_ai_summary() {
        let dir = tempdir().unwrap();
        let snapshot = create_project_file(input(dir.path().join("Eldara"), "Eldara")).unwrap();
        let connection = Connection::open(snapshot.path).unwrap();
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();

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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();

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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();
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
            |_| Ok("Keine konkreten Widersprueche gefunden.".to_string()),
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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();

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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();

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
        let scene =
            read_manuscript_tree(&connection).unwrap().areas[0].chapters[0].scenes[0].clone();

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

        let response = call_openai_compatible_free_chat(&settings, &input, None).unwrap();

        assert_eq!(response, "Das ist ein guter Ansatz.");
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
        assert!(ai_chat_system_prompt(AI_CHAT_MODE_IDEAS).contains("Ideenberater"));
        assert!(ai_chat_system_prompt(AI_CHAT_MODE_WRITING).contains("Schreibassistent"));
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

        let error = call_openai_compatible_free_chat(&settings, &input, None).unwrap_err();

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

        delete_chapter_in_connection(&mut connection, DeleteChapterInput { id: chapter_id })
            .unwrap();
        let tree = read_manuscript_tree(&connection).unwrap();
        assert!(tree.areas[0].chapters.is_empty());
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
}
