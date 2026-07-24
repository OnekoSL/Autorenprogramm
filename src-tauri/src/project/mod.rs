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
const PROJECT_DEFAULTS_FILE: &str = "project-defaults.json";
const DEFAULT_STORY_STRUCTURE_TEMPLATE: &str = "freestyle";
const RECENT_PROJECTS_LIMIT: usize = 10;
const SCHEMA_VERSION: i64 = 16;
const DEFAULT_SCENE_STATUS: &str = "draft";
const DEFAULT_EDITOR_FONT_FAMILY: &str = "serif";
const DEFAULT_EDITOR_FONT_SIZE: i64 = 19;
const SCENE_LINK_RELATION_ASSIGNED: &str = "assigned";
const AI_ACTION_SUMMARIZE_SCENE: &str = "summarize_scene";
const AI_ACTION_CHECK_SCENE_CONTRADICTIONS: &str = "check_scene_contradictions";
const AI_BIBLE_MODE_GENERATE: &str = "generate";
const AI_BIBLE_MODE_COMPLETE: &str = "complete";
const AI_BIBLE_MODE_EDIT: &str = "edit";
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
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
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
pub struct ProjectDefaultsSnapshot {
    pub story_structure_template_id: String,
    pub story_structure_template_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectDefaultsInput {
    pub story_structure_template_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredProjectDefaults {
    story_structure_template_id: String,
}

impl Default for StoredProjectDefaults {
    fn default() -> Self {
        Self {
            story_structure_template_id: DEFAULT_STORY_STRUCTURE_TEMPLATE.to_string(),
        }
    }
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportProfileSnapshot {
    pub include_novel_title: bool,
    pub include_area_titles: bool,
    pub include_chapter_titles: bool,
    pub include_scene_titles: bool,
    pub include_scene_text: bool,
}

impl Default for ExportProfileSnapshot {
    fn default() -> Self {
        Self {
            include_novel_title: false,
            include_area_titles: true,
            include_chapter_titles: true,
            include_scene_titles: false,
            include_scene_text: true,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateExportProfileInput {
    pub include_novel_title: bool,
    pub include_area_titles: bool,
    pub include_chapter_titles: bool,
    pub include_scene_titles: bool,
    pub include_scene_text: bool,
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
    pub factions: Vec<FactionSnapshot>,
    pub events: Vec<EventSnapshot>,
    pub scene_links: Vec<SceneLinkSnapshot>,
    pub relationships: Vec<BibleRelationshipSnapshot>,
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
pub struct FactionSnapshot {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub faction_type: String,
    pub description: String,
    pub goals: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventSnapshot {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub time_label: String,
    pub summary: String,
    pub outcome: String,
    pub notes: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldSystemSnapshot {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub category: String,
    pub template_id: String,
    pub principle: String,
    pub rules: String,
    pub capabilities: String,
    pub costs: String,
    pub limits: String,
    pub access: String,
    pub distribution: String,
    pub world_impact: String,
    pub open_questions: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldSystemTemplateSnapshot {
    pub id: String,
    pub name: String,
    pub category: String,
    pub guiding_questions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldbuildingSnapshot {
    pub systems: Vec<WorldSystemSnapshot>,
    pub templates: Vec<WorldSystemTemplateSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BibleRelationshipSnapshot {
    pub id: String,
    pub source_type: String,
    pub source_id: String,
    pub target_type: String,
    pub target_id: String,
    pub relationship_type: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
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
pub struct UpsertBibleRelationshipInput {
    pub id: Option<String>,
    pub source_type: String,
    pub source_id: String,
    pub target_type: String,
    pub target_id: String,
    pub relationship_type: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteBibleRelationshipInput {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderBibleEventsInput {
    pub event_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorldSystemInput {
    pub name: String,
    pub category: String,
    pub template_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorldSystemInput {
    pub id: String,
    pub name: String,
    pub category: String,
    pub template_id: String,
    pub principle: String,
    pub rules: String,
    pub capabilities: String,
    pub costs: String,
    pub limits: String,
    pub access: String,
    pub distribution: String,
    pub world_impact: String,
    pub open_questions: String,
    pub notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWorldSystemInput {
    pub id: String,
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
pub struct EditBibleCardInput {
    pub target_type: String,
    pub target_id: String,
    pub instruction: String,
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
    #[serde(default)]
    pub selected_fields: Vec<String>,
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
    pub code: String,
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
    pub source_draft: Option<AiBibleCardDraft>,
    pub target_updated_at: Option<String>,
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
    language: String,
    scene_id: String,
    scene_title: String,
    scene_summary: String,
    scene_text: String,
    context_summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiContradictionRequest {
    project_id: String,
    language: String,
    scene_id: String,
    scene_title: String,
    scene_summary: String,
    scene_text: String,
    context_summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AiBibleRequest {
    project_id: String,
    language: String,
    target_type: String,
    target_id: Option<String>,
    mode: String,
    keywords: String,
    existing_card_summary: String,
    source_draft: Option<AiBibleCardDraft>,
    target_updated_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextAlternativesRequest {
    language: String,
    scene_id: String,
    selected_text: String,
    context_before: String,
    context_after: String,
    is_phrase: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextImprovementRequest {
    language: String,
    scene_id: String,
    selected_text: String,
    context_before: String,
    context_after: String,
    mode: String,
}

mod ai_bible;
mod ai_provider;
mod ai_scene;
mod ai_settings;
mod bible;
mod export;
mod lifecycle;
mod manuscript;
mod schema;
mod search;
mod worldbuilding;

pub(crate) use ai_bible::*;
pub(crate) use ai_provider::*;
pub(crate) use ai_scene::*;
pub(crate) use ai_settings::*;
pub(crate) use bible::*;
pub(crate) use export::*;
pub(crate) use lifecycle::*;
pub(crate) use manuscript::*;
pub(crate) use schema::*;
pub(crate) use search::*;
pub(crate) use worldbuilding::*;

#[cfg(test)]
mod tests;
