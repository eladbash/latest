use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct SystemProfilerOutput {
    #[serde(rename = "SPApplicationsDataType")]
    applications: Vec<RawApp>,
}

#[derive(Debug, Deserialize)]
pub struct RawApp {
    #[serde(rename = "_name")]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(rename = "version", default)]
    pub version: Option<String>,
    #[serde(rename = "obtained_from", default)]
    pub obtained_from: Option<String>,
    #[serde(rename = "info", default)]
    pub bundle_id: Option<String>,
}

pub async fn get_applications() -> Vec<RawApp> {
    let output = tokio::task::spawn_blocking(|| {
        Command::new("system_profiler")
            .args(["SPApplicationsDataType", "-json"])
            .output()
    })
    .await;

    let output = match output {
        Ok(Ok(o)) if o.status.success() => o,
        _ => return vec![],
    };

    let parsed: Result<SystemProfilerOutput, _> = serde_json::from_slice(&output.stdout);

    match parsed {
        Ok(data) => data.applications,
        Err(e) => {
            eprintln!("Failed to parse system_profiler JSON: {}", e);
            vec![]
        }
    }
}
