pub mod plist_reader;
pub mod system_profiler;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppSource {
    Apple,
    MacAppStore,
    Identified,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub bundle_id: Option<String>,
    pub path: String,
    pub current_version: String,
    pub obtained_from: AppSource,
    pub sparkle_feed_url: Option<String>,
    pub icon_path: Option<String>,
}

pub async fn discover_apps() -> Vec<AppInfo> {
    let raw_apps = system_profiler::get_applications().await;

    let mut apps: Vec<AppInfo> = Vec::new();

    for raw in raw_apps {
        // Skip Apple system apps
        if matches!(raw.obtained_from.as_deref(), Some("apple")) {
            continue;
        }

        // Skip system paths and non-app bundles
        if raw.path.starts_with("/System")
            || raw.path.starts_with("/Library/Apple")
            || raw.path.starts_with("/usr")
            || !raw.path.ends_with(".app")
        {
            continue;
        }

        // Skip helper/agent apps nested inside other .app bundles
        if raw.path.matches(".app/").count() > 0
            && !raw.path.starts_with("/Applications")
        {
            continue;
        }

        // Skip known non-user apps
        let name_lower = raw.name.to_lowercase();
        if name_lower.contains("helper")
            || name_lower.contains("agent")
            || name_lower.contains("daemon")
            || name_lower.contains("droplet")
            || name_lower.contains("uninstaller")
            || name_lower.contains("crash reporter")
            || name_lower.ends_with("process")
            || name_lower.ends_with("service")
        {
            continue;
        }

        // Skip junk entries (too short, or looks like a bundle ID fragment)
        if raw.name.len() <= 3 || raw.name.starts_with("com.") || raw.name.starts_with("org.") || raw.name.starts_with("io.") {
            continue;
        }

        let plist_info = plist_reader::read_plist(&raw.path);

        let version = plist_info
            .as_ref()
            .and_then(|p| p.version.clone())
            .or(raw.version.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let bundle_id = plist_info
            .as_ref()
            .and_then(|p| p.bundle_id.clone())
            .or(raw.bundle_id.clone());

        let sparkle_feed_url = plist_info.as_ref().and_then(|p| p.sparkle_feed_url.clone());

        let obtained_from = match raw.obtained_from.as_deref() {
            Some("mac_app_store") => AppSource::MacAppStore,
            Some("apple") => AppSource::Apple,
            Some("identified_developer") => AppSource::Identified,
            _ => AppSource::Unknown,
        };

        let icon_path = find_icon_path(&raw.path);

        apps.push(AppInfo {
            name: raw.name,
            bundle_id,
            path: raw.path,
            current_version: version,
            obtained_from,
            sparkle_feed_url,
            icon_path,
        });
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

fn find_icon_path(app_path: &str) -> Option<String> {
    let resources = format!("{app_path}/Contents/Resources");
    let plist_path = format!("{app_path}/Contents/Info.plist");

    // Try reading icon name from Info.plist
    if let Ok(plist_val) = plist::from_file::<_, plist::Value>(&plist_path) {
        if let Some(dict) = plist_val.as_dictionary() {
            if let Some(icon_name) = dict
                .get("CFBundleIconFile")
                .and_then(|v| v.as_string())
            {
                let name = if icon_name.ends_with(".icns") {
                    icon_name.to_string()
                } else {
                    format!("{icon_name}.icns")
                };
                let icon = format!("{resources}/{name}");
                if std::path::Path::new(&icon).exists() {
                    return Some(icon);
                }
            }
        }
    }

    // Fallback: AppIcon.icns
    let fallback = format!("{resources}/AppIcon.icns");
    if std::path::Path::new(&fallback).exists() {
        return Some(fallback);
    }

    None
}
