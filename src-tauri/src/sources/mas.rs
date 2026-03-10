use crate::sources::{UpdateCheckResult, UpdateSourceType};
use std::process::Command;

pub async fn check_mas_updates() -> Vec<UpdateCheckResult> {
    tokio::task::spawn_blocking(|| {
        // Check if mas is installed
        if !Command::new("which")
            .arg("mas")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return vec![];
        }

        let output = match Command::new("mas").arg("outdated").output() {
            Ok(o) if o.status.success() => o,
            _ => return vec![],
        };

        let stdout = String::from_utf8_lossy(&output.stdout);

        // mas outdated format: "123456789 AppName (1.0 -> 2.0)"
        stdout
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                // Parse: ID Name (current -> latest)
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() < 2 {
                    return None;
                }

                let _app_id = parts[0];
                let rest = parts[1];

                let (name, versions) = if let Some(paren_idx) = rest.rfind('(') {
                    let name = rest[..paren_idx].trim();
                    let versions = rest[paren_idx..].trim_matches(|c| c == '(' || c == ')');
                    (name, versions)
                } else {
                    (rest.trim(), "")
                };

                let (current, latest) = if let Some(arrow) = versions.find("->") {
                    let current = versions[..arrow].trim();
                    let latest = versions[arrow + 2..].trim();
                    (current.to_string(), latest.to_string())
                } else {
                    ("unknown".to_string(), "unknown".to_string())
                };

                Some(UpdateCheckResult {
                    app_name: name.to_string(),
                    app_path: String::new(),
                    bundle_id: None,
                    current_version: current,
                    latest_version: Some(latest),
                    has_update: true,
                    source: UpdateSourceType::MacAppStore,
                    download_url: None,
                    error: None,
                })
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}
