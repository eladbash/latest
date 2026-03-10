use std::path::Path;
use std::process::Command;
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
pub struct ProgressPayload {
    pub app_path: String,
    pub phase: String,
    pub percent: u32,
}

pub fn emit_progress(app: &AppHandle, app_path: &str, phase: &str, percent: u32) {
    let _ = app.emit(
        "update-progress",
        ProgressPayload {
            app_path: app_path.to_string(),
            phase: phase.to_string(),
            percent,
        },
    );
}

pub async fn update(download_url: &str, app_path: &str, app: &AppHandle) -> Result<String, String> {
    if download_url.is_empty() {
        return Err("No download URL available for this app".to_string());
    }

    let url = download_url.to_string();
    let app_dest = app_path.to_string();

    eprintln!("[Latest] Downloading: {}", url);
    emit_progress(app, app_path, "downloading", 0);

    // Download the file
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .get(&url)
        .header("User-Agent", "Latest/0.1")
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    // Determine file extension from final URL (after redirects), Content-Disposition, or original URL
    let final_url = response.url().to_string().to_lowercase();
    let content_disp = response
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let url_lower = url.to_lowercase();

    let ext = detect_extension(&final_url, &content_disp, &url_lower)
        .ok_or_else(|| "Unsupported file format (not dmg/zip/pkg)".to_string())?;

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

    eprintln!("[Latest] Downloaded {} bytes as .{}", bytes.len(), ext);

    // Save to temp file
    emit_progress(app, app_path, "installing", 0);

    let tmp_dir = std::env::temp_dir().join("latest-updates");
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let tmp_file = tmp_dir.join(format!("update.{}", ext));
    std::fs::write(&tmp_file, &bytes)
        .map_err(|e| format!("Failed to save download: {}", e))?;

    // Install based on type
    let result = tokio::task::spawn_blocking(move || {
        match ext {
            "dmg" => install_dmg(&tmp_file, &app_dest),
            "zip" => install_zip(&tmp_file, &app_dest),
            "pkg" => install_pkg(&tmp_file),
            _ => Err("Unsupported format".to_string()),
        }
    })
    .await
    .map_err(|e| format!("Install task error: {}", e))?;

    emit_progress(app, app_path, "installing", 100);

    // Clean up temp files
    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("latest-updates"));

    result
}

fn detect_extension(final_url: &str, content_disp: &str, original_url: &str) -> Option<&'static str> {
    // Check all sources for a known extension
    for source in &[final_url, content_disp, original_url] {
        if source.contains(".dmg") {
            return Some("dmg");
        }
        if source.contains(".zip") {
            return Some("zip");
        }
        if source.contains(".pkg") {
            return Some("pkg");
        }
    }
    None
}

pub fn install_dmg_pub(dmg_path: &Path, app_dest: &str) -> Result<String, String> {
    install_dmg(dmg_path, app_dest)
}

pub fn install_zip_pub(zip_path: &Path, app_dest: &str) -> Result<String, String> {
    install_zip(zip_path, app_dest)
}

fn install_dmg(dmg_path: &Path, app_dest: &str) -> Result<String, String> {
    eprintln!("[Latest] Mounting DMG: {:?}", dmg_path);

    // Mount the DMG (no -quiet, we need stdout to find the mount point)
    let mount_output = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-noverify", "-noautoopen"])
        .arg(dmg_path)
        .output()
        .map_err(|e| format!("Failed to mount DMG: {}", e))?;

    if !mount_output.status.success() {
        let stderr = String::from_utf8_lossy(&mount_output.stderr);
        return Err(format!("Failed to mount DMG: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&mount_output.stdout);
    eprintln!("[Latest] hdiutil output: {}", stdout);

    // Find the mount point (last column of last line with a /Volumes path)
    let mount_point = stdout
        .lines()
        .filter_map(|line| {
            // hdiutil output is tab-separated, mount point is the last column
            line.split('\t')
                .last()
                .map(|s| s.trim().to_string())
                .filter(|s| s.starts_with("/Volumes/"))
        })
        .last()
        .ok_or("Could not find DMG mount point")?;

    eprintln!("[Latest] Mounted at: {}", mount_point);

    // Find .app in the mounted volume
    let app_name = find_app_in_dir(&mount_point);

    let result = if let Some(app_name) = app_name {
        let source = format!("{}/{}", mount_point, app_name);
        let dest_dir = Path::new(app_dest)
            .parent()
            .unwrap_or(Path::new("/Applications"));

        eprintln!("[Latest] Copying {} to {:?}", source, dest_dir);

        // Remove old version first
        if Path::new(app_dest).exists() {
            let _ = Command::new("rm")
                .args(["-rf", app_dest])
                .output();
        }

        let cp = Command::new("cp")
            .args(["-R", &source, &dest_dir.to_string_lossy()])
            .output()
            .map_err(|e| format!("Failed to copy app: {}", e))?;

        if cp.status.success() {
            Ok(format!("Updated {}", app_name))
        } else {
            let stderr = String::from_utf8_lossy(&cp.stderr);
            Err(format!("Copy failed: {}", stderr))
        }
    } else {
        Err("No .app found in DMG".to_string())
    };

    // Always detach
    let _ = Command::new("hdiutil")
        .args(["detach", &mount_point, "-quiet"])
        .output();

    result
}

fn install_zip(zip_path: &Path, app_dest: &str) -> Result<String, String> {
    eprintln!("[Latest] Extracting ZIP: {:?}", zip_path);

    let extract_dir = zip_path.parent().unwrap().join("extracted");
    let _ = std::fs::create_dir_all(&extract_dir);

    let unzip = Command::new("unzip")
        .args(["-o", "-q"])
        .arg(zip_path)
        .arg("-d")
        .arg(&extract_dir)
        .output()
        .map_err(|e| format!("Failed to unzip: {}", e))?;

    if !unzip.status.success() {
        let stderr = String::from_utf8_lossy(&unzip.stderr);
        return Err(format!("Unzip failed: {}", stderr));
    }

    // Find .app in extracted dir
    let app_name = find_app_in_dir(&extract_dir.to_string_lossy());

    if let Some(app_name) = app_name {
        let source = extract_dir.join(&app_name);
        let dest_dir = Path::new(app_dest)
            .parent()
            .unwrap_or(Path::new("/Applications"));

        // Remove old version
        if Path::new(app_dest).exists() {
            let _ = Command::new("rm")
                .args(["-rf", app_dest])
                .output();
        }

        let cp = Command::new("cp")
            .args(["-R"])
            .arg(&source)
            .arg(&dest_dir)
            .output()
            .map_err(|e| format!("Failed to copy app: {}", e))?;

        if cp.status.success() {
            Ok(format!("Updated {}", app_name))
        } else {
            let stderr = String::from_utf8_lossy(&cp.stderr);
            Err(format!("Copy failed: {}", stderr))
        }
    } else {
        Err("No .app found in ZIP".to_string())
    }
}

fn install_pkg(pkg_path: &Path) -> Result<String, String> {
    eprintln!("[Latest] Installing PKG: {:?}", pkg_path);

    // Use `open` to launch the .pkg installer (prompts the user)
    let output = Command::new("open")
        .arg(pkg_path)
        .output()
        .map_err(|e| format!("Failed to open installer: {}", e))?;

    if output.status.success() {
        Ok("Opened installer — follow the prompts to complete".to_string())
    } else {
        Err("Failed to open installer".to_string())
    }
}

fn find_app_in_dir(dir: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".app") {
            return Some(name);
        }
    }
    None
}
