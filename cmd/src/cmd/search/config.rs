use colored::Colorize;
use eyre::{eyre, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(1100);

pub fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME env var must be set");
    PathBuf::from(home).join(".config/aps")
}

fn ensure_config_dir() -> Result<PathBuf> {
    let dir = config_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// Read an API key, checking env var first then config file
pub fn get_api_key(config_filename: &str, env_var: &str) -> Option<String> {
    if let Ok(key) = std::env::var(env_var) {
        if !key.is_empty() {
            return Some(key);
        }
    }

    let config_path = config_dir().join(config_filename);
    if let Ok(key) = std::fs::read_to_string(&config_path) {
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Some(key);
        }
    }

    None
}

/// Save an API key to the config dir
pub fn save_api_key(config_filename: &str, key: &str) -> Result<()> {
    let dir = ensure_config_dir()?;
    std::fs::write(dir.join(config_filename), key)?;
    Ok(())
}

/// Data directory for persistent storage (papers, DB)
pub fn data_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").expect("HOME env var must be set");
    std::path::PathBuf::from(home).join(".local/share/aps")
}

/// Read the Sci-Hub base URL from env var or config file
pub fn get_scihub_url() -> Option<String> {
    if let Ok(url) = std::env::var("SCIHUB_URL") {
        if !url.is_empty() {
            return Some(url);
        }
    }

    let config_path = config_dir().join("scihub-url");
    if let Ok(url) = std::fs::read_to_string(&config_path) {
        let url = url.trim().to_string();
        if !url.is_empty() {
            return Some(url);
        }
    }

    None
}

/// Save Sci-Hub base URL to config
pub fn save_scihub_url(url: &str) -> Result<()> {
    let dir = ensure_config_dir()?;
    std::fs::write(dir.join("scihub-url"), url)?;
    Ok(())
}

/// S2 throttle: ensures ~1 RPS across processes via file lock + timestamp
pub async fn s2_throttle() {
    s2_throttle_with_dir(&config_dir()).await;
}

/// Testable version that accepts a custom directory
pub async fn s2_throttle_with_dir(dir: &Path) {
    let dir = dir.to_path_buf();
    tokio::task::spawn_blocking(move || s2_throttle_blocking(&dir))
        .await
        .expect("throttle task panicked");
}

fn s2_throttle_blocking(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);

    let lock_path = dir.join("s2-last-request.lock");
    let ts_path = dir.join("s2-last-request");

    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .expect("failed to open lock file");

    lock_file.lock().expect("failed to acquire lock");

    if let Ok(contents) = std::fs::read_to_string(&ts_path) {
        if let Ok(epoch_millis) = contents.trim().parse::<u128>() {
            let now_millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();

            let elapsed = Duration::from_millis((now_millis.saturating_sub(epoch_millis)) as u64);
            if let Some(wait) = MIN_REQUEST_INTERVAL.checked_sub(elapsed) {
                std::thread::sleep(wait);
            }
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let _ = std::fs::write(&ts_path, now.to_string());
    // lock released on drop
}

/// Run `aps login` — saves available API keys from env vars to config
pub fn login() -> Result<()> {
    let mut saved = 0;

    if let Ok(key) = std::env::var("SEMANTIC_SCHOLAR_API_KEY") {
        if !key.is_empty() {
            save_api_key("s2-api-key", &key)?;
            println!("{} saved Semantic Scholar API key", "✓".green());
            saved += 1;
        }
    } else {
        println!(
            "{} SEMANTIC_SCHOLAR_API_KEY not set, skipping",
            "·".dimmed()
        );
    }

    if let Ok(key) = std::env::var("OPENALEX_API_KEY") {
        if !key.is_empty() {
            save_api_key("oa-api-key", &key)?;
            println!("{} saved OpenAlex API key", "✓".green());
            saved += 1;
        }
    } else {
        println!("{} OPENALEX_API_KEY not set, skipping", "·".dimmed());
    }

    if saved == 0 {
        return Err(eyre!(
            "no API keys found — set SEMANTIC_SCHOLAR_API_KEY and/or OPENALEX_API_KEY env vars"
        ));
    }

    println!(
        "\n{} {}",
        "keys saved to".dimmed(),
        config_dir().display().to_string().cyan()
    );
    Ok(())
}

/// Show current auth status
pub fn status() -> Result<()> {
    let s2 = get_api_key("s2-api-key", "SEMANTIC_SCHOLAR_API_KEY");
    let oa = get_api_key("oa-api-key", "OPENALEX_API_KEY");

    let s2_source = if std::env::var("SEMANTIC_SCHOLAR_API_KEY").is_ok() {
        "env"
    } else if config_dir().join("s2-api-key").exists() {
        "config"
    } else {
        "none"
    };

    let oa_source = if std::env::var("OPENALEX_API_KEY").is_ok() {
        "env"
    } else if config_dir().join("oa-api-key").exists() {
        "config"
    } else {
        "none"
    };

    print_auth_line("Semantic Scholar", s2.is_some(), s2_source);
    print_auth_line("OpenAlex        ", oa.is_some(), oa_source);

    println!(
        "\n{} {}",
        "config dir:".dimmed(),
        config_dir().display().to_string().cyan()
    );
    Ok(())
}

fn print_auth_line(name: &str, authenticated: bool, source: &str) {
    if authenticated {
        println!(
            "{} {}: {} {}",
            "✓".green(),
            name.bold(),
            "authenticated".green(),
            format!("({source})").dimmed()
        );
    } else {
        println!("{} {}: {}", "✗".red(), name.bold(), "not configured".red());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Barrier;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_concurrent_throttle() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = Arc::new(dir.path().to_path_buf());
        let barrier = Arc::new(Barrier::new(5));

        let mut handles = vec![];
        for _ in 0..5 {
            let dir = Arc::clone(&dir_path);
            let barrier = Arc::clone(&barrier);
            handles.push(tokio::spawn(async move {
                barrier.wait().await;
                s2_throttle_with_dir(&dir).await;
                Instant::now()
            }));
        }

        let mut times = vec![];
        for h in handles {
            times.push(h.await.unwrap());
        }

        let earliest = *times.iter().min().unwrap();
        let latest = *times.iter().max().unwrap();
        let spread = latest - earliest;

        // 5 serialized waits of 1.1s each: first returns immediately, last waits ~4.4s
        // use 3.5s as lower bound to account for timing variance
        assert!(
            spread >= Duration::from_millis(3500),
            "expected >= 3.5s spread but got {spread:?} — throttle is not serializing"
        );
    }
}
