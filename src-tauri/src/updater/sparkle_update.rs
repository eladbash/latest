use std::path::Path;
use std::process::Command;
use tauri::AppHandle;
use crate::updater::github_update::emit_progress;

pub async fn update(app_path: &str, download_url: Option<&str>, app: &AppHandle) -> Result<String, String> {
    // If we have a direct download URL from the appcast, download and install
    if let Some(url) = download_url {
        if !url.is_empty() {
            return download_and_install(url, app_path, app).await;
        }
    }

    // Fallback: open the app and try to trigger its built-in updater
    let path = app_path.to_string();
    tokio::task::spawn_blocking(move || {
        let app_name = Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("App")
            .to_string();

        // Try sparkle-cli first
        if let Ok(output) = Command::new("sparkle-cli")
            .arg("bundle")
            .arg(&path)
            .output()
        {
            if output.status.success() {
                return Ok(format!("Updating {} via Sparkle", app_name));
            }
        }

        // Open the app and trigger "Check for Updates" via AppleScript
        let _ = Command::new("open").arg(&path).output();
        std::thread::sleep(std::time::Duration::from_secs(2));

        let script = format!(
            r#"
            tell application "System Events"
                tell process "{}"
                    try
                        click menu item "Check for Updates…" of menu 1 of menu bar item "{}" of menu bar 1
                    end try
                    try
                        click menu item "Check for Updates..." of menu 1 of menu bar item "{}" of menu bar 1
                    end try
                end tell
            end tell
            "#,
            app_name, app_name, app_name
        );

        let _ = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output();

        Ok(format!("Opened {} — checking for updates", app_name))
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

async fn download_and_install(url: &str, app_path: &str, app: &AppHandle) -> Result<String, String> {
    eprintln!("[Latest] Sparkle download: {}", url);
    emit_progress(app, app_path, "downloading", 0);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .get(url)
        .header("User-Agent", "Latest/0.1")
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed: HTTP {}", response.status()));
    }

    // Stream download with progress
    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut bytes = Vec::new();
    let mut last_percent: u32 = 0;

    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Download error: {}", e))?
    {
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);

        if total > 0 {
            let percent = ((downloaded as f64 / total as f64) * 100.0).min(100.0) as u32;
            if percent != last_percent {
                last_percent = percent;
                emit_progress(app, app_path, "downloading", percent);
            }
        }
    }

    eprintln!("[Latest] Downloaded {} bytes", bytes.len());

    let url_lower = url.to_lowercase();
    let ext = if url_lower.ends_with(".dmg") {
        "dmg"
    } else if url_lower.ends_with(".zip") {
        "zip"
    } else if url_lower.ends_with(".pkg") {
        "pkg"
    } else {
        // Default to zip for Sparkle (common)
        "zip"
    };

    emit_progress(app, app_path, "installing", 0);

    let tmp_dir = std::env::temp_dir().join("latest-updates");
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let tmp_file = tmp_dir.join(format!("sparkle-update.{}", ext));
    std::fs::write(&tmp_file, &bytes)
        .map_err(|e| format!("Failed to save download: {}", e))?;

    let dest = app_path.to_string();
    let result = tokio::task::spawn_blocking(move || {
        // Use the same install logic as GitHub updater
        match ext {
            "dmg" => crate::updater::github_update::install_dmg_pub(&tmp_file, &dest),
            "zip" => crate::updater::github_update::install_zip_pub(&tmp_file, &dest),
            _ => {
                let _ = Command::new("open").arg(&tmp_file).output();
                Ok("Opened installer".to_string())
            }
        }
    })
    .await
    .map_err(|e| format!("Install error: {}", e))?;

    emit_progress(app, app_path, "installing", 100);

    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("latest-updates"));
    result
}
