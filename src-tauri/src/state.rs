use crate::discovery::AppInfo;
use crate::sources::UpdateCheckResult;
use std::sync::Mutex;
use tokio::sync::watch;

pub struct AppState {
    pub inner: Mutex<AppStateInner>,
}

pub struct AppStateInner {
    pub apps: Vec<AppInfo>,
    pub update_results: Vec<UpdateCheckResult>,
    pub last_check: Option<chrono::DateTime<chrono::Utc>>,
    pub interval_tx: Option<watch::Sender<u64>>,
    pub blink_abort: Option<tokio::task::AbortHandle>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(AppStateInner {
                apps: vec![],
                update_results: vec![],
                last_check: None,
                interval_tx: None,
                blink_abort: None,
            }),
        }
    }
}
