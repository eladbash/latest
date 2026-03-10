mod commands;
mod state;
mod tray;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            // Hide dock icon — menu bar only
            #[cfg(target_os = "macos")]
            {
                use tauri::ActivationPolicy;
                app.set_activation_policy(ActivationPolicy::Accessory);
            }

            tray::create_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_apps,
            commands::check_updates_now,
            commands::trigger_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
