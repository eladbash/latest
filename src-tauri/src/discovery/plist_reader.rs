use plist::Value;

pub struct PlistInfo {
    pub version: Option<String>,
    pub bundle_id: Option<String>,
    pub sparkle_feed_url: Option<String>,
}

pub fn read_plist(app_path: &str) -> Option<PlistInfo> {
    let plist_path = format!("{}/Contents/Info.plist", app_path);
    let val: Value = plist::from_file(&plist_path).ok()?;
    let dict = val.as_dictionary()?;

    let version = dict
        .get("CFBundleShortVersionString")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    let bundle_id = dict
        .get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    let sparkle_feed_url = dict
        .get("SUFeedURL")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    Some(PlistInfo {
        version,
        bundle_id,
        sparkle_feed_url,
    })
}
