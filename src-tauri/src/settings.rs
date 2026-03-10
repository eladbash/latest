use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_PATH: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum CheckInterval {
    ThirtyMinutes,
    #[default]
    OneHour,
    SixHours,
    Daily,
}

impl CheckInterval {
    pub fn to_secs(&self) -> u64 {
        match self {
            CheckInterval::ThirtyMinutes => 30 * 60,
            CheckInterval::OneHour => 60 * 60,
            CheckInterval::SixHours => 6 * 60 * 60,
            CheckInterval::Daily => 24 * 60 * 60,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub check_interval: CheckInterval,
    pub ignored_apps: Vec<String>,
    pub show_notifications: bool,
    #[serde(default)]
    pub show_all_apps: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            check_interval: CheckInterval::default(),
            ignored_apps: vec![],
            show_notifications: true,
            show_all_apps: false,
        }
    }
}

pub fn load_settings(app: &AppHandle) -> Settings {
    let store = match app.store(STORE_PATH) {
        Ok(s) => s,
        Err(_) => return Settings::default(),
    };

    let settings: Settings = match store.get("settings") {
        Some(val) => serde_json::from_value(val).unwrap_or_default(),
        None => Settings::default(),
    };

    settings
}

pub fn save_settings(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;
    let val = serde_json::to_value(settings).map_err(|e| e.to_string())?;
    store.set("settings", val);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}
