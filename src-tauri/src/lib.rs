mod commands;
mod discovery;
mod scheduler;
mod settings;
mod sources;
mod state;
mod tray;
mod updater;
mod version;

use state::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                use tauri::ActivationPolicy;
                app.set_activation_policy(ActivationPolicy::Accessory);
            }

            tray::create_tray(app)?;

            // Load settings and start scheduler
            let loaded = settings::load_settings(&app.handle());
            let interval_secs = loaded.check_interval.to_secs();
            let (interval_tx, interval_rx) = tokio::sync::watch::channel(interval_secs);

            // Store the interval sender in state for settings changes
            {
                let app_state = app.state::<AppState>();
                let mut inner = app_state
                    .inner
                    .lock()
                    .expect("failed to lock state");
                inner.interval_tx = Some(interval_tx);
            }

            scheduler::start_scheduler(app.handle().clone(), interval_rx);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_apps,
            commands::check_updates_now,
            commands::trigger_update,
            commands::get_settings,
            commands::set_check_interval,
            commands::toggle_ignore_app,
            commands::set_show_notifications,
            commands::set_show_all_apps,
            commands::is_app_running,
            commands::quit_app,
            commands::reopen_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
