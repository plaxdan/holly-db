use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const REPO: &str = "plaxdan/holly-db";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CACHE_TTL_SECS: u64 = 86_400; // 24 hours

pub fn run(verbose: bool) -> Result<()> {
    let cache_path = cache_file();

    let cached = read_cache(&cache_path);
    let now = unix_now();

    // Use cache if it's still fresh
    let latest = if let Some((checked_at, version)) = cached {
        if now.saturating_sub(checked_at) < CACHE_TTL_SECS {
            version
        } else {
            fetch_latest().unwrap_or(version)
        }
    } else {
        match fetch_latest() {
            Ok(v) => v,
            Err(_) => {
                // Network unavailable — stay silent
                if verbose {
                    eprintln!("holly update-check: could not reach GitHub (offline?)");
                }
                return Ok(());
            }
        }
    };

    write_cache(&cache_path, now, &latest);

    if is_newer(&latest, CURRENT_VERSION) {
        println!(
            "holly update available: {} → {}  (https://github.com/{}/releases/latest)",
            CURRENT_VERSION, latest, REPO
        );
    } else if verbose {
        println!("holly is up to date ({})", CURRENT_VERSION);
    }

    Ok(())
}

fn fetch_latest() -> Result<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let response = ureq::get(&url)
        .set("User-Agent", &format!("holly-db/{}", CURRENT_VERSION))
        .set("Accept", "application/vnd.github+json")
        .call();

    let body: Value = match response {
        Ok(r) => r.into_json()?,
        Err(ureq::Error::Status(404, _)) => {
            // No releases published yet — treat as up to date
            return Ok(CURRENT_VERSION.to_string());
        }
        Err(e) => return Err(e.into()),
    };

    let tag = body["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("no tag_name in response"))?
        .trim_start_matches('v')
        .to_string();

    Ok(tag)
}

/// Parse "M.m.p" into (major, minor, patch) for comparison.
fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let mut parts = v.trim_start_matches('v').splitn(3, '.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Some((major, minor, patch))
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

fn cache_file() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("holly")
        .join("update-check.json")
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_cache(path: &PathBuf) -> Option<(u64, String)> {
    let content = fs::read_to_string(path).ok()?;
    let v: Value = serde_json::from_str(&content).ok()?;
    let checked_at = v["checked_at"].as_u64()?;
    let version = v["latest_version"].as_str()?.to_string();
    Some((checked_at, version))
}

fn write_cache(path: &PathBuf, now: u64, latest: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::json!({
        "checked_at": now,
        "latest_version": latest,
        "current_version": CURRENT_VERSION,
    });
    let _ = fs::write(path, content.to_string());
}
