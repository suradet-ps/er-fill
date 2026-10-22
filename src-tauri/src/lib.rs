//! Tauri 2 shell for ER-Refill - thin command adapters only (AGENTS.md section 3).
//!
//! No SQL and no business logic live here: everything HOSxP-related goes
//! through `er-refill-hosxp-connector`, the only crate allowed to touch MySQL.

mod commands;
mod error;
mod settings;
mod state;

use state::AppState;
use tauri::Manager;

/// Entry point used by `main.rs`.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let settings_path = config_dir.join(settings::SETTINGS_FILE_NAME);
            let loaded = tauri::async_runtime::block_on(settings::load(&settings_path));
            let app_settings = match loaded {
                Ok(app_settings) => app_settings,
                Err(err) => {
                    error::dev_log(&format!("settings load failed, using defaults: {err}"));
                    settings::AppSettings::default()
                }
            };

            let state = AppState::new(config_dir, app_settings);
            // Warms the pool now, then pings every 30 s for the app's
            // lifetime. Feeds connection_health (the status dot).
            let monitor_state = state.clone();
            tauri::async_runtime::spawn(async move {
                commands::run_health_monitor(monitor_state).await;
            });
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connection_status,
            commands::connection_health,
            commands::connection_summary,
            commands::configure_connection,
            commands::connect,
            commands::reconnect,
            commands::login,
            commands::logout,
            commands::current_session,
            commands::test_connection,
            commands::load_settings,
            commands::save_schedule,
            commands::save_catalog,
            commands::save_report,
            commands::search_drugs,
            commands::fetch_usage,
        ])
        .run(tauri::generate_context!())
        .expect("invariant: tauri::run() fails only when the platform cannot launch the app window - there is no fallback path for a GUI app")
}
