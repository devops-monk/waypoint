mod commands;
mod secrets;

use std::time::Duration;

use tauri::Manager;
use waypoint_ai::Llm;
use waypoint_core::Db;
use waypoint_sources::Http;

pub struct AppState {
    pub db: Db,
    pub http: Http,
    pub llm: Llm,
}

/// Native translucency behind the sidebar: the macOS sidebar material, Mica on Windows 11.
/// Linux keeps the solid background the CSS provides.
fn apply_window_material(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};
        let _ = apply_vibrancy(window, NSVisualEffectMaterial::Sidebar, Some(NSVisualEffectState::FollowsWindowActiveState), None);
    }
    #[cfg(target_os = "windows")]
    {
        let _ = window_vibrancy::apply_mica(window, None);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = window;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let db = tauri::async_runtime::block_on(Db::open(&data_dir.join("waypoint.db")))?;
            app.manage(AppState { db, http: Http::new(Duration::from_millis(1000)), llm: Llm::new() });
            if let Some(window) = app.get_webview_window("main") {
                apply_window_material(&window);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_jobs,
            commands::get_job,
            commands::set_status,
            commands::set_starred,
            commands::set_notes,
            commands::pipeline_counts,
            commands::add_job_manual,
            commands::providers,
            commands::seed_boards,
            commands::list_boards,
            commands::detect_board,
            commands::add_board,
            commands::remove_board,
            commands::set_board_enabled,
            commands::scan_all,
            commands::get_profile,
            commands::save_profile,
            commands::import_resume,
            commands::get_ai_settings,
            commands::save_ai_settings,
            commands::delete_api_key,
            commands::latest_evaluation,
            commands::evaluate_job,
            commands::token_usage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Waypoint");
}
