use crate::discovery;
use crate::sources;
use crate::state::AppState;
use crate::tray;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;

pub fn start_scheduler(
    app_handle: AppHandle,
    interval_rx: watch::Receiver<u64>,
) {
    let handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let mut rx = interval_rx;

        loop {
            let interval_secs = *rx.borrow();
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;

            tray::set_tray_checking(&handle, true);

            // Run update check
            let apps = discovery::discover_apps().await;

            if apps.is_empty() {
                tray::set_tray_checking(&handle, false);
                continue;
            }

            let results = sources::check_all_updates(&apps).await;
            let update_count = results
                .iter()
                .filter(|r| r.has_update && !r.app_path.is_empty())
                .count();

            // Update state
            if let Some(state) = handle.try_state::<AppState>() {
                if let Ok(mut inner) = state.inner.lock() {
                    inner.apps = apps;
                    inner.update_results = results.clone();
                    inner.last_check = Some(chrono::Utc::now());
                }
            }

            // Emit event to frontend
            let _ = handle.emit("updates-found", &results);

            // Stop blink and set badge
            tray::set_tray_checking(&handle, false);
            tray::set_tray_update_count(&handle, update_count);

            // Check if interval changed
            if rx.has_changed().unwrap_or(false) {
                rx.mark_changed();
            }
        }
    });
}
