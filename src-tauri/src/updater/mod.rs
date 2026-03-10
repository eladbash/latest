pub mod brew_update;
pub mod github_update;
pub mod mas_update;
pub mod sparkle_update;

use crate::sources::UpdateSourceType;
use tauri::AppHandle;

pub async fn dispatch_update(
    app_name: &str,
    app_path: &str,
    source: &UpdateSourceType,
    download_url: Option<&str>,
    app: &AppHandle,
) -> Result<String, String> {
    match source {
        UpdateSourceType::Sparkle => sparkle_update::update(app_path, download_url, app).await,
        UpdateSourceType::Homebrew => {
            // If we have a download URL (from brew cask info), download and install directly.
            // Otherwise fall back to `brew upgrade --cask`.
            if let Some(url) = download_url {
                if !url.is_empty() {
                    return github_update::update(url, app_path, app).await;
                }
            }
            brew_update::update(app_name).await
        }
        UpdateSourceType::MacAppStore => mas_update::update(app_name).await,
        UpdateSourceType::Unknown => Err("Unknown update source".to_string()),
    }
}
