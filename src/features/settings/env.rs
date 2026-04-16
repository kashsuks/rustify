use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

pub fn read_theme() -> Option<String> {
    dotenvy::dotenv().ok();
    std::env::var("APP_THEME").ok()
}

pub fn write_theme(theme: &str) -> io::Result<()> {
    write_env_keys(&[("APP_THEME", theme)])
}

pub fn write_lastfm_settings(api_key: &str, api_secret: &str, username: &str) -> io::Result<()> {
    write_env_keys(&[
        ("LASTFM_API_KEY", api_key),
        ("LASTFM_API_SECRET", api_secret),
        ("LASTFM_USERNAME", username),
    ])
}

pub fn write_lastfm_session_key(session_key: &str) -> io::Result<()> {
    write_env_keys(&[("LASTFM_SESSION_KEY", session_key)])
}

fn find_env_path() -> Option<PathBuf> {
    dotenvy::dotenv().ok()?;
    std::env::var("DOTENV_FILENAME")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::current_dir().ok().map(|p| p.join(".env")))
}

fn write_env_keys(entries: &[(&str, &str)]) -> io::Result<()> {
    let path = find_env_path()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, ".env file not found"))?;

    let existing = fs::read_to_string(&path).unwrap_or_default();

    let mut map = parse_env(&existing);
    for (key, value) in entries {
        map.insert(key.to_string(), value.to_string());
    }

    let mut output = String::new();
    for (key, value) in map {
        output.push_str(&format!("{}={}\n", key, value));
    }

    fs::write(path, output)
}

fn parse_env(input: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    map
}
