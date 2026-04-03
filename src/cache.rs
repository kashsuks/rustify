use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedLink {
    pub lastfm_title: String,
    pub lastfm_artist: String,
    pub skipped: bool,
}

fn cache_path() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("rustify");
    std::fs::create_dir_all(&path).ok()?;
    path.push("links.json");
    Some(path)
}

pub fn cache_key(filename: &str, duration_secs: u64) -> String {
    format!("{}::{}", filename, duration_secs)
}

pub fn load() -> HashMap<String, CachedLink> {
    let Some(path) = cache_path() else { return HashMap::new() };
    let Ok(data) = std::fs::read_to_string(&path) else { return HashMap::new() };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save(cache: &HashMap<String, CachedLink>) {
    let Some(path) = cache_path() else { return };
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(path, json);
    }
}

pub fn insert(key: String, link: CachedLink) {
    let mut cache = load();
    cache.insert(key, link);
    save(&cache);
}
