use std::process::Command;

pub async fn update(app_name: &str) -> Result<String, String> {
    let name = app_name.to_string();
    tokio::task::spawn_blocking(move || {
        // First get the app ID from mas list
        let list_output = Command::new("mas")
            .arg("list")
            .output()
            .map_err(|e| format!("Failed to run mas: {e}"))?;

        let list_str = String::from_utf8_lossy(&list_output.stdout);
        let app_id = list_str
            .lines()
            .find(|line| line.contains(&name))
            .and_then(|line| line.split_whitespace().next())
            .ok_or_else(|| format!("Could not find app ID for {name}"))?
            .to_string();

        let output = Command::new("mas")
            .args(["upgrade", &app_id])
            .output()
            .map_err(|e| format!("Failed to run mas upgrade: {e}"))?;

        if output.status.success() {
            Ok(format!("Successfully updated {name} via Mac App Store"))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("MAS upgrade failed: {stderr}"))
        }
    })
    .await
    .map_err(|e| format!("Task error: {e}"))?
}
