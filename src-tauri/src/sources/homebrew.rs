use crate::discovery::AppInfo;
use crate::sources::{UpdateCheckResult, UpdateSourceType};
use std::collections::HashMap;
use std::process::Command;

/// Check for outdated brew-installed casks (fast, only brew-managed apps).
pub async fn check_homebrew_updates() -> Vec<UpdateCheckResult> {
    tokio::task::spawn_blocking(|| {
        if !brew_available() {
            return vec![];
        }

        let output = match Command::new("brew")
            .args(["outdated", "--cask", "--json"])
            .output()
        {
            Ok(o) if o.status.success() => o,
            _ => return vec![],
        };

        let json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[Latest] Failed to parse brew outdated JSON: {e}");
                return vec![];
            }
        };

        let casks = match json.get("casks").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => return vec![],
        };

        casks
            .iter()
            .filter_map(|cask| {
                let name = cask.get("name")?.as_str()?;
                let current = cask.get("installed_versions")?.as_str().unwrap_or("unknown");
                let latest = cask.get("current_version")?.as_str()?;

                Some(UpdateCheckResult {
                    app_name: name.to_string(),
                    app_path: String::new(),
                    bundle_id: None,
                    current_version: current.to_string(),
                    latest_version: Some(latest.to_string()),
                    has_update: true,
                    source: UpdateSourceType::Homebrew,
                    download_url: None,
                    error: None,
                })
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// Generic version check using Homebrew's cask API.
/// Fetches the full cask database (~3900 apps), builds a map by .app filename,
/// then matches against all discovered apps. Works for any app with a brew cask.
pub async fn check_brew_cask_versions(apps: &[AppInfo]) -> Vec<UpdateCheckResult> {
    let cask_map = match fetch_cask_map().await {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[Latest] Failed to fetch cask data: {e}");
            return vec![];
        }
    };

    let mut results = Vec::new();
    for app in apps {
        // Match by .app filename (e.g. "Docker.app")
        let app_file = std::path::Path::new(&app.path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("")
            .to_lowercase();

        if let Some(info) = cask_map.get(&app_file) {
            let has_update = crate::version::is_newer(&app.current_version, &info.version);
            results.push(UpdateCheckResult {
                app_name: app.name.clone(),
                app_path: app.path.clone(),
                bundle_id: app.bundle_id.clone(),
                current_version: app.current_version.clone(),
                latest_version: Some(info.version.clone()),
                has_update,
                source: UpdateSourceType::Homebrew,
                download_url: if info.url.is_empty() {
                    None
                } else {
                    Some(info.url.clone())
                },
                error: None,
            });
        }
    }

    results
}

struct CaskInfo {
    version: String,
    url: String,
}

/// Fetch all cask data from Homebrew's public API and build a lookup map
/// keyed by .app filename (lowercased).
async fn fetch_cask_map() -> Result<HashMap<String, CaskInfo>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let response = client
        .get("https://formulae.brew.sh/api/cask.json")
        .header("User-Agent", "Latest/0.1")
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let casks: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {e}"))?;

    let mut map = HashMap::new();

    for cask in &casks {
        let version_raw = match cask.get("version").and_then(|v| v.as_str()) {
            Some(v) => v,
            None => continue,
        };
        // Strip build numbers after comma (e.g. "4.63.0,220185" -> "4.63.0")
        let version = version_raw.split(',').next().unwrap_or(version_raw).to_string();
        let url = cask
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if let Some(artifacts) = cask.get("artifacts").and_then(|v| v.as_array()) {
            for artifact in artifacts {
                if let Some(obj) = artifact.as_object() {
                    if let Some(apps) = obj.get("app").and_then(|v| v.as_array()) {
                        for app in apps {
                            // App entries can be strings or objects like {"target": "..."}
                            if let Some(name) = app.as_str() {
                                map.insert(
                                    name.to_lowercase(),
                                    CaskInfo {
                                        version: version.clone(),
                                        url: url.clone(),
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    eprintln!("[Latest] Loaded {} app entries from Homebrew cask API", map.len());
    Ok(map)
}

fn brew_available() -> bool {
    Command::new("which")
        .arg("brew")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
