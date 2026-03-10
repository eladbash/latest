use crate::discovery::{self, AppInfo};
use crate::settings::{self, CheckInterval, Settings};
use crate::sources::{self, UpdateCheckResult};
use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn get_apps(state: State<'_, AppState>) -> Result<Vec<AppInfo>, String> {
    let apps = discovery::discover_apps().await;
    {
        let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
        inner.apps = apps.clone();
    }
    Ok(apps)
}

#[tauri::command]
pub async fn check_updates_now(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<UpdateCheckResult>, String> {
    let apps = {
        let inner = state.inner.lock().map_err(|e| e.to_string())?;
        inner.apps.clone()
    };

    if apps.is_empty() {
        return Err("No apps discovered yet. Call get_apps first.".into());
    }

    crate::tray::set_tray_checking(&app, true);

    let results = sources::check_all_updates(&apps).await;
    // Only count results the frontend can display (those with a real app_path)
    let update_count = results
        .iter()
        .filter(|r| r.has_update && !r.app_path.is_empty())
        .count();

    {
        let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
        inner.update_results = results.clone();
        inner.last_check = Some(chrono::Utc::now());
    }

    crate::tray::set_tray_checking(&app, false);
    crate::tray::set_tray_update_count(&app, update_count);

    Ok(results)
}

#[tauri::command]
pub async fn trigger_update(
    app: AppHandle,
    state: State<'_, AppState>,
    app_path: String,
    source: String,
) -> Result<String, String> {
    use crate::sources::UpdateSourceType;
    use crate::updater;

    let source_type = match source.as_str() {
        "Sparkle" => UpdateSourceType::Sparkle,
        "Homebrew" => UpdateSourceType::Homebrew,
        "MacAppStore" => UpdateSourceType::MacAppStore,
        _ => return Err(format!("Unknown source type: {source}")),
    };

    // Find download URL from stored results
    let download_url = {
        let inner = state.inner.lock().map_err(|e| e.to_string())?;
        inner
            .update_results
            .iter()
            .find(|r| r.app_path == app_path)
            .and_then(|r| r.download_url.clone())
    };

    // Find app name
    let app_name = {
        let inner = state.inner.lock().map_err(|e| e.to_string())?;
        inner
            .update_results
            .iter()
            .find(|r| r.app_path == app_path)
            .map(|r| r.app_name.clone())
            .unwrap_or_default()
    };

    updater::dispatch_update(
        &app_name,
        &app_path,
        &source_type,
        download_url.as_deref(),
        &app,
    )
    .await
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<Settings, String> {
    Ok(settings::load_settings(&app))
}

#[tauri::command]
pub fn set_check_interval(
    app: AppHandle,
    state: State<'_, AppState>,
    interval: String,
) -> Result<(), String> {
    let check_interval = match interval.as_str() {
        "ThirtyMinutes" => CheckInterval::ThirtyMinutes,
        "OneHour" => CheckInterval::OneHour,
        "SixHours" => CheckInterval::SixHours,
        "Daily" => CheckInterval::Daily,
        _ => return Err(format!("Unknown interval: {interval}")),
    };

    let mut s = settings::load_settings(&app);
    s.check_interval = check_interval;
    settings::save_settings(&app, &s)?;

    // Notify scheduler of new interval
    let inner = state.inner.lock().map_err(|e| e.to_string())?;
    if let Some(tx) = &inner.interval_tx {
        let _ = tx.send(s.check_interval.to_secs());
    }

    Ok(())
}

#[tauri::command]
pub fn toggle_ignore_app(
    app: AppHandle,
    bundle_id: String,
) -> Result<Settings, String> {
    let mut s = settings::load_settings(&app);

    if let Some(pos) = s.ignored_apps.iter().position(|id| *id == bundle_id) {
        s.ignored_apps.remove(pos);
    } else {
        s.ignored_apps.push(bundle_id);
    }

    settings::save_settings(&app, &s)?;
    Ok(s)
}

#[tauri::command]
pub fn set_show_notifications(
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    let mut s = settings::load_settings(&app);
    s.show_notifications = enabled;
    settings::save_settings(&app, &s)?;
    Ok(())
}

#[tauri::command]
pub fn set_show_all_apps(
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    let mut s = settings::load_settings(&app);
    s.show_all_apps = enabled;
    settings::save_settings(&app, &s)?;
    Ok(())
}

/// Check if an app is currently running by its bundle ID or path.
#[tauri::command]
pub async fn is_app_running(app_path: String) -> Result<bool, String> {
    let path = app_path.clone();
    tokio::task::spawn_blocking(move || {
        let bundle_id = get_bundle_id_from_path(&path);
        if let Some(bid) = &bundle_id {
            // Check by bundle ID (most reliable)
            let output = std::process::Command::new("pgrep")
                .args(["-f", bid])
                .output();
            if let Ok(o) = output {
                if o.status.success() {
                    return Ok(true);
                }
            }
        }
        // Fallback: check by app path
        let output = std::process::Command::new("pgrep")
            .args(["-f", &path])
            .output();
        match output {
            Ok(o) => Ok(o.status.success()),
            Err(_) => Ok(false),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Gracefully quit a running app, wait for it to close.
#[tauri::command]
pub async fn quit_app(app_path: String) -> Result<(), String> {
    let path = app_path.clone();
    tokio::task::spawn_blocking(move || {
        let app_name = std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if app_name.is_empty() {
            return Err("Could not determine app name".to_string());
        }

        eprintln!("[Latest] Quitting: {app_name}");

        // Use osascript to gracefully quit the app
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "tell application \"{app_name}\" to quit"
            ))
            .output();

        // Wait up to 10 seconds for the app to close
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let still_running = std::process::Command::new("pgrep")
                .args(["-f", &path])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !still_running {
                eprintln!("[Latest] {app_name} has quit");
                return Ok(());
            }
        }

        // Force kill if still running
        eprintln!("[Latest] Force killing {app_name}");
        let _ = std::process::Command::new("pkill")
            .args(["-f", &path])
            .output();
        std::thread::sleep(std::time::Duration::from_secs(1));

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Reopen an app after update.
#[tauri::command]
pub async fn reopen_app(app_path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        eprintln!("[Latest] Reopening: {app_path}");
        let output = std::process::Command::new("open")
            .arg(&app_path)
            .output()
            .map_err(|e| format!("Failed to open app: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err("Failed to reopen app".to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

fn get_bundle_id_from_path(app_path: &str) -> Option<String> {
    let plist_path = format!("{app_path}/Contents/Info.plist");
    let val = plist::from_file::<_, plist::Value>(&plist_path).ok()?;
    val.as_dictionary()?
        .get("CFBundleIdentifier")?
        .as_string()
        .map(|s| s.to_string())
}
