use std::sync::Mutex;

pub struct AppState {
    pub inner: Mutex<AppStateInner>,
}

pub struct AppStateInner {
    pub last_check: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(AppStateInner { last_check: None }),
        }
    }
}
