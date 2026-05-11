use std::{env, fs, path::PathBuf};

use color_eyre::eyre::{eyre, Result, WrapErr};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct RealtimeConfig {
    pub api_key: String,
    pub model: String,
    pub voice: String,
}

#[derive(Debug, Deserialize)]
struct CodexAuth {
    #[serde(rename = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,
}

impl RealtimeConfig {
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(read_codex_api_key)
            .ok_or_else(|| {
                eyre!("OPENAI_API_KEY is not set and no key was found in ~/.codex/auth.json")
            })?;
        let model =
            env::var("CODEX_VOICE_REALTIME_MODEL").unwrap_or_else(|_| "gpt-realtime".to_owned());
        let voice = env::var("CODEX_VOICE_REALTIME_VOICE").unwrap_or_else(|_| "marin".to_owned());

        Ok(Self {
            api_key,
            model,
            voice,
        })
    }
}

fn read_codex_api_key() -> Option<String> {
    let auth_path = home_dir().ok()?.join(".codex/auth.json");
    let auth = fs::read_to_string(&auth_path).ok()?;
    let auth = serde_json::from_str::<CodexAuth>(&auth).ok()?;
    auth.openai_api_key.filter(|value| !value.trim().is_empty())
}

pub fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| eyre!("HOME is not set"))
        .wrap_err("Failed to resolve home directory")
}
