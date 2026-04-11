use crate::discovery::AppInfo;
use crate::sources::{UpdateCheckResult, UpdateSourceType};
use std::collections::{HashMap, HashSet};
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
///
/// To avoid false matches (e.g. two different apps both named "Latest.app"),
/// we verify each match by comparing bundle identifiers and checking whether
/// the cask is actually installed via Homebrew.
pub async fn check_brew_cask_versions(apps: &[AppInfo]) -> Vec<UpdateCheckResult> {
    let (cask_map, brew_installed) = tokio::join!(
        fetch_cask_map(),
        get_brew_installed_tokens(),
    );

    let cask_map = match cask_map {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[Latest] Failed to fetch cask data: {e}");
            return vec![];
        }
    };

    let brew_installed = brew_installed.unwrap_or_default();

    let mut results = Vec::new();
    for app in apps {
        // Match by .app filename (e.g. "Docker.app")
        let app_file = std::path::Path::new(&app.path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("")
            .to_lowercase();

        if let Some(info) = cask_map.get(&app_file) {
            // Verify this is actually the same app, not a name collision
            if !is_same_app(app, info, &brew_installed) {
                eprintln!(
                    "[Latest] Skipping cask '{}' for '{}' — bundle ID mismatch (app: {:?}, cask hints: {:?})",
                    info.token, app.name, app.bundle_id, info.bundle_id_hints
                );
                continue;
            }

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

/// Verify that a discovered app matches a Homebrew cask by more than just filename.
fn is_same_app(app: &AppInfo, cask: &CaskInfo, brew_installed: &HashSet<String>) -> bool {
    // If this cask is installed via brew, trust the match — brew manages it
    if brew_installed.contains(&cask.token) {
        return true;
    }

    // If both have bundle IDs, check if any cask hint matches the app
    if let Some(app_bid) = &app.bundle_id {
        if !cask.bundle_id_hints.is_empty() {
            return cask.bundle_id_hints.iter().any(|hint| bundle_id_matches(app_bid, hint));
        }
    }

    // Can't verify either way — include the match (best effort)
    true
}

/// Check if an app's bundle ID matches a cask's bundle ID hint.
/// Handles glob patterns (e.g. "com.todesktop.*") and sub-process identifiers
/// (e.g. "com.postmanlabs.mac.ShipIt" matching "com.postmanlabs.mac").
fn bundle_id_matches(app_bid: &str, hint: &str) -> bool {
    let app_lower = app_bid.to_lowercase();
    let hint_lower = hint.to_lowercase();

    // Handle glob patterns (e.g., "com.todesktop.*", "com.hnc.discord.sfl*")
    if let Some(prefix) = hint_lower.strip_suffix('*') {
        let prefix = prefix.trim_end_matches('.');
        // App bundle ID starts with the glob prefix: com.todesktop.xxx matches com.todesktop.*
        if app_lower == prefix
            || (app_lower.starts_with(prefix)
                && app_lower.as_bytes().get(prefix.len()) == Some(&b'.'))
        {
            return true;
        }
        // Glob prefix starts with app bundle ID: com.hnc.discord.sfl matches com.hnc.discord
        if prefix.starts_with(&app_lower)
            && prefix.as_bytes().get(app_lower.len()) == Some(&b'.')
        {
            return true;
        }
        return false;
    }

    // Exact match
    if app_lower == hint_lower {
        return true;
    }

    // Sub-process match: one is a sub-identifier of the other
    // e.g., "com.postmanlabs.mac" matches "com.postmanlabs.mac.ShipIt"
    // Require at least 3 dot-separated segments in the shared prefix to avoid
    // false positives like "com.google.Chrome" matching "com.google.SoftwareUpdate"
    let min_segments = 3;
    if hint_lower.starts_with(&app_lower)
        && hint_lower.as_bytes().get(app_lower.len()) == Some(&b'.')
        && app_lower.matches('.').count() + 1 >= min_segments
    {
        return true;
    }
    if app_lower.starts_with(&hint_lower)
        && app_lower.as_bytes().get(hint_lower.len()) == Some(&b'.')
        && hint_lower.matches('.').count() + 1 >= min_segments
    {
        return true;
    }

    false
}

/// Get the set of cask tokens currently installed via Homebrew.
async fn get_brew_installed_tokens() -> Result<HashSet<String>, String> {
    tokio::task::spawn_blocking(|| {
        if !brew_available() {
            return Ok(HashSet::new());
        }

        let output = Command::new("brew")
            .args(["list", "--cask"])
            .output()
            .map_err(|e| format!("Failed to run brew list: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
    })
    .await
    .map_err(|e| format!("Task error: {e}"))?
}

struct CaskInfo {
    version: String,
    url: String,
    token: String,
    /// All candidate bundle IDs extracted from cask artifacts (uninstall.quit, launchctl, zap.trash)
    bundle_id_hints: Vec<String>,
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
        let token = cask
            .get("token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
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

        let bundle_id_hints = extract_bundle_ids_from_cask(cask);

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
                                        token: token.clone(),
                                        bundle_id_hints: bundle_id_hints.clone(),
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

/// Collect all candidate bundle identifiers from a cask's artifact metadata.
/// Gathers from: uninstall.quit, uninstall.launchctl, and zap.trash plist paths.
/// Multiple candidates increase the chance of matching the installed app's real bundle ID,
/// since quit/launchctl values often reference helper processes rather than the main app.
fn extract_bundle_ids_from_cask(cask: &serde_json::Value) -> Vec<String> {
    let mut hints = Vec::new();
    let artifacts = match cask.get("artifacts").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return hints,
    };

    for artifact in artifacts {
        let obj = match artifact.as_object() {
            Some(o) => o,
            None => continue,
        };

        // Collect from uninstall.quit and uninstall.launchctl
        if let Some(uninstalls) = obj.get("uninstall").and_then(|v| v.as_array()) {
            for entry in uninstalls {
                for key in &["quit", "launchctl"] {
                    if let Some(val) = entry.get(*key) {
                        collect_string_or_array(val, &mut hints);
                    }
                }
            }
        }

        // Collect from zap.trash plist paths
        if let Some(zaps) = obj.get("zap").and_then(|v| v.as_array()) {
            for entry in zaps {
                if let Some(trash) = entry.get("trash") {
                    let paths: Vec<&str> = if let Some(s) = trash.as_str() {
                        vec![s]
                    } else if let Some(arr) = trash.as_array() {
                        arr.iter().filter_map(|v| v.as_str()).collect()
                    } else {
                        vec![]
                    };
                    for p in paths {
                        if let Some(bid) = extract_bundle_id_from_path(p) {
                            if !hints.contains(&bid) {
                                hints.push(bid);
                            }
                        }
                    }
                }
            }
        }
    }

    hints
}

/// Collect string values from a JSON value that is either a string or array of strings.
fn collect_string_or_array(val: &serde_json::Value, out: &mut Vec<String>) {
    if let Some(s) = val.as_str() {
        if !out.contains(&s.to_string()) {
            out.push(s.to_string());
        }
    } else if let Some(arr) = val.as_array() {
        for item in arr {
            if let Some(s) = item.as_str() {
                if !out.contains(&s.to_string()) {
                    out.push(s.to_string());
                }
            }
        }
    }
}

/// Try to extract a bundle ID from a file path like
/// "~/Library/Preferences/com.example.app.plist" → "com.example.app"
fn extract_bundle_id_from_path(path: &str) -> Option<String> {
    let filename = path.rsplit('/').next()?;
    let name = filename.strip_suffix(".plist").unwrap_or(filename);

    // Must look like a reverse-DNS bundle ID (at least 2 dots)
    if (name.starts_with("com.")
        || name.starts_with("org.")
        || name.starts_with("io.")
        || name.starts_with("net.")
        || name.starts_with("me.")
        || name.starts_with("dev."))
        && name.matches('.').count() >= 2
    {
        Some(name.to_string())
    } else {
        None
    }
}

fn brew_available() -> bool {
    Command::new("which")
        .arg("brew")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
