mod project;
mod story_structure;

use project::AppState;

#[derive(serde::Serialize)]
struct AppInfo {
    name: &'static str,
    version: &'static str,
    environment: &'static str,
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        name: "Autorenprogramm",
        version: env!("CARGO_PKG_VERSION"),
        environment: "desktop",
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            project::create_project,
            project::open_project,
            project::update_project,
            project::get_project_defaults,
            project::update_project_defaults,
            project::save_project_copy_as,
            project::list_recent_projects,
            project::remove_recent_project,
            project::get_ai_settings,
            project::update_ai_settings,
            project::get_ai_provider_status,
            project::start_local_ai_provider,
            project::list_ai_models,
            project::send_ai_chat_message,
            project::generate_text_alternatives,
            project::improve_text,
            project::search_scenes,
            project::search_project,
            project::export_manuscript_markdown,
            project::export_manuscript_docx,
            project::get_export_profile,
            project::update_export_profile,
            project::get_manuscript_tree,
            project::create_area,
            project::create_chapter,
            project::create_scene,
            project::update_area,
            project::update_chapter,
            project::update_scene,
            project::move_area,
            project::move_chapter,
            project::move_scene,
            project::delete_area,
            project::delete_chapter,
            project::delete_scene,
            project::get_project_bible,
            project::get_worldbuilding,
            project::create_world_system,
            project::update_world_system,
            project::delete_world_system,
            project::create_project_bible_card,
            project::update_project_bible_card,
            project::delete_project_bible_card,
            project::get_scene_links_for_card,
            project::replace_scene_links_for_card,
            project::upsert_bible_relationship,
            project::delete_bible_relationship,
            project::reorder_bible_events,
            project::summarize_scene,
            project::check_scene_contradictions,
            project::list_ai_suggestions_for_scene,
            project::accept_ai_suggestion,
            project::reject_ai_suggestion,
            project::generate_bible_card_from_keywords,
            project::complete_bible_card_from_keywords,
            project::edit_bible_card,
            project::list_ai_bible_suggestions,
            project::accept_ai_bible_suggestion,
            project::reject_ai_bible_suggestion,
            story_structure::list_story_structure_templates,
            story_structure::get_project_structure,
            story_structure::apply_story_structure_template,
            story_structure::generate_story_structure_manuscript,
            story_structure::update_structure_beat,
            story_structure::replace_structure_beat_scene_links,
            story_structure::replace_structure_beat_character_links
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::get_app_info;

    #[test]
    fn app_info_contains_product_identity() {
        let info = get_app_info();

        assert_eq!(info.name, "Autorenprogramm");
        assert_eq!(info.environment, "desktop");
        assert!(!info.version.is_empty());
    }
}
