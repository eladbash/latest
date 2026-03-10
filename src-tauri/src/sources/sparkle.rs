use crate::discovery::AppInfo;
use crate::sources::{UpdateCheckResult, UpdateSourceType};

pub async fn check_sparkle_updates(apps: &[AppInfo]) -> Vec<UpdateCheckResult> {
    let sparkle_apps: Vec<&AppInfo> = apps
        .iter()
        .filter(|a| a.sparkle_feed_url.is_some())
        .collect();

    let futures: Vec<_> = sparkle_apps
        .into_iter()
        .map(|app| check_single_sparkle(app.clone()))
        .collect();

    futures::future::join_all(futures).await
}

async fn check_single_sparkle(app: AppInfo) -> UpdateCheckResult {
    let feed_url = app.sparkle_feed_url.as_deref().unwrap();

    let result = fetch_latest_version(feed_url).await;

    match result {
        Ok((latest_version, download_url)) => {
            let has_update =
                crate::version::is_newer(&app.current_version, &latest_version);
            UpdateCheckResult {
                app_name: app.name,
                app_path: app.path,
                bundle_id: app.bundle_id,
                current_version: app.current_version,
                latest_version: Some(latest_version),
                has_update,
                source: UpdateSourceType::Sparkle,
                download_url,
                error: None,
            }
        }
        Err(e) => UpdateCheckResult {
            app_name: app.name.clone(),
            app_path: app.path.clone(),
            bundle_id: app.bundle_id.clone(),
            current_version: app.current_version.clone(),
            latest_version: None,
            has_update: false,
            source: UpdateSourceType::Sparkle,
            download_url: None,
            error: Some(e),
        },
    }
}

async fn fetch_latest_version(
    feed_url: &str,
) -> Result<(String, Option<String>), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .get(feed_url)
        .header("User-Agent", "Latest/0.1")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let body = response
        .text()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    parse_appcast(&body)
}

fn parse_appcast(xml: &str) -> Result<(String, Option<String>), String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut latest_version: Option<String> = None;
    let mut download_url: Option<String> = None;
    let mut in_item = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                match e.name().as_ref() {
                    b"item" => {
                        in_item = true;
                    }
                    b"enclosure" if in_item => {
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"sparkle:shortVersionString" => {
                                    if latest_version.is_none() {
                                        latest_version = Some(
                                            String::from_utf8_lossy(&attr.value).to_string(),
                                        );
                                    }
                                }
                                b"sparkle:version" => {
                                    if latest_version.is_none() {
                                        latest_version = Some(
                                            String::from_utf8_lossy(&attr.value).to_string(),
                                        );
                                    }
                                }
                                b"url" => {
                                    if download_url.is_none() {
                                        download_url = Some(
                                            String::from_utf8_lossy(&attr.value).to_string(),
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"item" => {
                in_item = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
    }

    match latest_version {
        Some(v) => Ok((v, download_url)),
        None => Err("No version found in appcast".to_string()),
    }
}
