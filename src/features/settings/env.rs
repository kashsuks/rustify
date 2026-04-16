use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

fn config_dir() -> io::Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Could not find config directory")
    })?;
    Ok(base.join("rustify"))
}

fn config_path() -> io::Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

fn ensure_config_dir() -> io::Result<()> {
    let dir = config_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

fn load_config() -> BTreeMap<String, String> {
    let path = match config_path() {
        Ok(p) => p,
        Err(_) => return BTreeMap::new(),
    };

    match fs::read_to_string(&path) {
        Ok(content) => parse_toml(&content),
        Err(_) => BTreeMap::new(),
    }
}

fn parse_toml(input: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut in_section = false;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') {
            in_section = true;
            continue;
        }
        if !in_section {
            if let Some((key, value)) = trimmed.split_once('=') {
                map.insert(
                    key.trim().to_string(),
                    value.trim().to_string().trim_matches('"').to_string(),
                );
            }
        }
    }
    map
}

fn save_config(map: BTreeMap<String, String>) -> io::Result<()> {
    ensure_config_dir()?;
    let mut output = String::new();
    for (key, value) in &map {
        output.push_str(&format!("{} = \"{}\"\n", key, value));
    }
    fs::write(config_path()?, output)
}

pub fn read_theme() -> Option<String> {
    let map = load_config();
    map.get("theme").cloned()
}

pub fn write_theme(theme: &str) -> io::Result<()> {
    let mut map = load_config();
    map.insert("theme".to_string(), theme.to_string());
    save_config(map)
}

pub fn write_lastfm_settings(api_key: &str, api_secret: &str, username: &str) -> io::Result<()> {
    dotenvy::dotenv().ok();
    let path = std::env::current_dir()
        .ok()
        .map(|p| p.join(".env"))
        .unwrap_or_default();

    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut map = parse_env(&existing);
    map.insert("LASTFM_API_KEY".to_string(), api_key.to_string());
    map.insert("LASTFM_API_SECRET".to_string(), api_secret.to_string());
    map.insert("LASTFM_USERNAME".to_string(), username.to_string());

    let mut output = String::new();
    for (key, value) in &map {
        output.push_str(&format!("{}={}\n", key, value));
    }
    fs::write(path, output)
}

pub fn write_lastfm_session_key(session_key: &str) -> io::Result<()> {
    dotenvy::dotenv().ok();
    let path = std::env::current_dir()
        .ok()
        .map(|p| p.join(".env"))
        .unwrap_or_default();

    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut map = parse_env(&existing);
    map.insert("LASTFM_SESSION_KEY".to_string(), session_key.to_string());

    let mut output = String::new();
    for (key, value) in &map {
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
