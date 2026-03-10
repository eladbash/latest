use std::process::Command;

pub async fn update(app_name: &str) -> Result<String, String> {
    let name = app_name.to_string();
    tokio::task::spawn_blocking(move || {
        // First try to find the exact cask token via brew list
        let token = find_cask_token(&name)
            .unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));

        eprintln!("[Latest] Running: brew upgrade --cask {token}");

        let output = Command::new("brew")
            .args(["upgrade", "--cask", &token])
            .output()
            .map_err(|e| format!("Failed to run brew: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        eprintln!("[Latest] brew stdout: {stdout}");
        eprintln!("[Latest] brew stderr: {stderr}");

        if output.status.success() {
            Ok(format!("Updated {name} via Homebrew"))
        } else {
            Err(format!("Brew upgrade failed: {}", stderr.trim()))
        }
    })
    .await
    .map_err(|e| format!("Task error: {e}"))?
}

fn find_cask_token(app_name: &str) -> Option<String> {
    let output = Command::new("brew")
        .args(["list", "--cask"])
        .output()
        .ok()?;

    let list = String::from_utf8_lossy(&output.stdout);
    let name_lower = app_name.to_lowercase();

    // Try exact match first, then fuzzy
    for line in list.lines() {
        let token = line.trim();
        if token == name_lower.replace(' ', "-") {
            return Some(token.to_string());
        }
    }

    // Fuzzy: check if any token contains the app name
    for line in list.lines() {
        let token = line.trim();
        if token.contains(&name_lower.replace(' ', "-"))
            || name_lower.contains(token)
        {
            return Some(token.to_string());
        }
    }

    None
}
