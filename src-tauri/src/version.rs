use semver::Version;

/// Returns true if `latest` is newer than `current`.
pub fn is_newer(current: &str, latest: &str) -> bool {
    // Try semver first
    if let (Ok(c), Ok(l)) = (Version::parse(current), Version::parse(latest)) {
        return l > c;
    }

    // Fallback: dot-segment numeric comparison
    let current_parts = parse_segments(current);
    let latest_parts = parse_segments(latest);

    for (c, l) in current_parts.iter().zip(latest_parts.iter()) {
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    // If all matching segments are equal, longer version is newer
    latest_parts.len() > current_parts.len()
}

fn parse_segments(version: &str) -> Vec<u64> {
    version
        .split('.')
        .filter_map(|s| {
            // Strip non-numeric suffixes (e.g., "1.2.3b4" -> "3")
            let numeric: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
            numeric.parse().ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver() {
        assert!(is_newer("1.0.0", "2.0.0"));
        assert!(is_newer("1.0.0", "1.0.1"));
        assert!(!is_newer("2.0.0", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
    }

    #[test]
    fn test_non_semver() {
        assert!(is_newer("1.2", "1.3"));
        assert!(is_newer("1.2.3.4", "1.2.3.5"));
        assert!(!is_newer("1.3", "1.2"));
    }

    #[test]
    fn test_mixed() {
        assert!(is_newer("1.2", "1.2.1"));
        assert!(is_newer("3.0.2b1", "3.0.3"));
    }

}
