use colored::Colorize;
use eyre::{eyre, Result};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MIN_REQUEST_INTERVAL: Duration = Duration::from_secs(1);

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

/// S2 throttle: ensures 1 RPS across processes via timestamp file
pub async fn s2_throttle() {
    let path = config_dir().join("s2-last-request");

    if let Ok(contents) = std::fs::read_to_string(&path) {
        if let Ok(epoch_millis) = contents.trim().parse::<u128>() {
            let now_millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();

            let elapsed =
                Duration::from_millis((now_millis.saturating_sub(epoch_millis)) as u64);
            if let Some(wait) = MIN_REQUEST_INTERVAL.checked_sub(elapsed) {
                tokio::time::sleep(wait).await;
            }
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    // ensure dir exists before writing
    let _ = std::fs::create_dir_all(config_dir());
    let _ = std::fs::write(&path, now.to_string());
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
        println!(
            "{} {}: {}",
            "✗".red(),
            name.bold(),
            "not configured".red()
        );
    }
}
