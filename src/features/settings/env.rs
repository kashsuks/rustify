use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

pub fn write_lastfm_settings(
    api_key: &str,
    api_secret: &str,
    username: &str,
) -> io::Result<()> {
    let path = Path::new(".env");
    let existing = fs::read_to_string(path).unwrap_or_default();

    let mut map = parse_env(&existing);
    map.insert("LASTFM_API_KEY".to_string(), api_key.to_string());
    map.insert("LASTFM_API_SECRET".to_string(), api_secret.to_string());
    map.insert("LASTFM_USERNAME".to_string(), username.to_string());

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
