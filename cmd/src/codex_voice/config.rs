use std::{
    env, fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{eyre, Result, WrapErr};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct RealtimeConfig {
    pub api_key: String,
    pub model: String,
    pub voice: String,
}

pub const DEFAULT_REALTIME_MODEL: &str = "gpt-realtime-2";
const DEFAULT_API_KEY_FILE: &str = ".config/codex-voice/openai-api-key";

#[derive(Debug, Deserialize)]
struct CodexAuth {
    #[serde(rename = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,
}

impl RealtimeConfig {
    pub fn from_env() -> Result<Self> {
        let home = home_dir()?;
        let api_key = resolve_api_key(&home).ok_or_else(|| {
            eyre!(
                "OpenAI API key not found. Set OPENAI_API_KEY, set CODEX_VOICE_OPENAI_API_KEY_FILE, create ~/.config/codex-voice/openai-api-key, or add OPENAI_API_KEY to ~/.codex/auth.json."
            )
        })?;
        let model = env::var("CODEX_VOICE_REALTIME_MODEL")
            .unwrap_or_else(|_| DEFAULT_REALTIME_MODEL.to_owned());
        let voice = env::var("CODEX_VOICE_REALTIME_VOICE").unwrap_or_else(|_| "marin".to_owned());

        Ok(Self {
            api_key,
            model,
            voice,
        })
    }
}

fn resolve_api_key(home: &Path) -> Option<String> {
    resolve_api_key_from_sources(
        env::var("OPENAI_API_KEY").ok(),
        env::var_os("CODEX_VOICE_OPENAI_API_KEY_FILE").map(PathBuf::from),
        home.join(DEFAULT_API_KEY_FILE),
        home.join(".codex/auth.json"),
    )
}

fn resolve_api_key_from_sources(
    env_key: Option<String>,
    env_file: Option<PathBuf>,
    default_file: PathBuf,
    codex_auth_file: PathBuf,
) -> Option<String> {
    env_key
        .and_then(nonempty_trimmed)
        .or_else(|| env_file.and_then(read_api_key_file))
        .or_else(|| read_api_key_file(default_file))
        .or_else(|| read_codex_api_key(codex_auth_file))
}

fn read_api_key_file(path: impl AsRef<Path>) -> Option<String> {
    let key = fs::read_to_string(path).ok()?;
    nonempty_trimmed(key)
}

fn read_codex_api_key(auth_path: impl AsRef<Path>) -> Option<String> {
    let auth = fs::read_to_string(auth_path).ok()?;
    let auth = serde_json::from_str::<CodexAuth>(&auth).ok()?;
    auth.openai_api_key.and_then(nonempty_trimmed)
}

fn nonempty_trimmed(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

pub fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| eyre!("HOME is not set"))
        .wrap_err("Failed to resolve home directory")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        read_api_key_file, read_codex_api_key, resolve_api_key, resolve_api_key_from_sources,
        DEFAULT_API_KEY_FILE,
    };

    #[test]
    fn reads_api_key_file_with_surrounding_whitespace_trimmed() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("key");
        fs::write(&path, "\n sk-test \n").unwrap();

        assert_eq!(read_api_key_file(&path).as_deref(), Some("sk-test"));
    }

    #[test]
    fn ignores_empty_api_key_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("key");
        fs::write(&path, "\n \t\n").unwrap();

        assert_eq!(read_api_key_file(&path), None);
    }

    #[test]
    fn reads_nonempty_codex_auth_api_key() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.json");
        fs::write(
            &path,
            r#"{"OPENAI_API_KEY":" sk-codex ","auth_mode":"apikey"}"#,
        )
        .unwrap();

        assert_eq!(read_codex_api_key(&path).as_deref(), Some("sk-codex"));
    }

    #[test]
    fn ignores_null_codex_auth_api_key() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("auth.json");
        fs::write(&path, r#"{"OPENAI_API_KEY":null,"tokens":{}}"#).unwrap();

        assert_eq!(read_codex_api_key(&path), None);
    }

    #[test]
    fn default_key_file_wins_over_codex_auth() {
        let dir = TempDir::new().unwrap();
        let default_path = dir.path().join(DEFAULT_API_KEY_FILE);
        fs::create_dir_all(default_path.parent().unwrap()).unwrap();
        fs::write(&default_path, "sk-default").unwrap();
        let codex_auth_path = dir.path().join(".codex/auth.json");
        fs::create_dir_all(codex_auth_path.parent().unwrap()).unwrap();
        fs::write(&codex_auth_path, r#"{"OPENAI_API_KEY":"sk-codex"}"#).unwrap();

        assert_eq!(resolve_api_key(dir.path()).as_deref(), Some("sk-default"));
    }

    #[test]
    fn env_key_wins_over_key_files() {
        let dir = TempDir::new().unwrap();
        let env_path = dir.path().join("env-key");
        let default_path = dir.path().join("default-key");
        let codex_auth_path = dir.path().join("auth.json");
        fs::write(&env_path, "sk-env-file").unwrap();
        fs::write(&default_path, "sk-default").unwrap();
        fs::write(&codex_auth_path, r#"{"OPENAI_API_KEY":"sk-codex"}"#).unwrap();

        let key = resolve_api_key_from_sources(
            Some(" sk-env ".to_owned()),
            Some(env_path),
            default_path,
            codex_auth_path,
        );

        assert_eq!(key.as_deref(), Some("sk-env"));
    }

    #[test]
    fn explicit_key_file_wins_over_default_key_file() {
        let dir = TempDir::new().unwrap();
        let env_path = dir.path().join("env-key");
        let default_path = dir.path().join("default-key");
        let codex_auth_path = dir.path().join("auth.json");
        fs::write(&env_path, "sk-env-file").unwrap();
        fs::write(&default_path, "sk-default").unwrap();
        fs::write(&codex_auth_path, r#"{"OPENAI_API_KEY":"sk-codex"}"#).unwrap();

        let key = resolve_api_key_from_sources(None, Some(env_path), default_path, codex_auth_path);

        assert_eq!(key.as_deref(), Some("sk-env-file"));
    }
}
