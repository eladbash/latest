pub mod homebrew;
pub mod mas;
pub mod sparkle;

use crate::discovery::AppInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateSourceType {
    Sparkle,
    Homebrew,
    MacAppStore,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub app_name: String,
    pub app_path: String,
    pub bundle_id: Option<String>,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub source: UpdateSourceType,
    pub download_url: Option<String>,
    pub error: Option<String>,
}

pub async fn check_all_updates(apps: &[AppInfo]) -> Vec<UpdateCheckResult> {
    let (sparkle_results, brew_outdated, brew_cask_results, mas_results) = tokio::join!(
        sparkle::check_sparkle_updates(apps),
        homebrew::check_homebrew_updates(),
        homebrew::check_brew_cask_versions(apps),
        mas::check_mas_updates(),
    );

    // Merge and deduplicate — prefer Sparkle > Brew Cask > Brew Outdated > MAS
    let mut seen: HashSet<String> = HashSet::new();
    let mut results = Vec::new();

    for r in sparkle_results {
        seen.insert(r.app_path.clone());
        results.push(r);
    }

    // Brew cask generic check (has app_path, most comprehensive)
    for r in brew_cask_results {
        if !seen.contains(&r.app_path) {
            seen.insert(r.app_path.clone());
            results.push(r);
        }
    }

    // Brew outdated (for brew-installed casks, no app_path)
    for r in brew_outdated {
        let key = r.app_name.to_lowercase();
        if !seen.contains(&key) {
            seen.insert(key);
            results.push(r);
        }
    }

    for r in mas_results {
        let key = r.app_name.to_lowercase();
        if !seen.contains(&key) {
            seen.insert(key);
            results.push(r);
        }
    }

    results
}
