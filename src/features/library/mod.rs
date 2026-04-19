/// This file is responsible for finding song metadata
/// And assigning default values of no real ones are found

use crate::app::TrackMeta;
use lofty::prelude::*;
use lofty::probe::Probe;
use std::path::{Path, PathBuf};

pub async fn pick_folder() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .pick_folder()
        .await
        .map(|f| f.path().to_path_buf())
}

/// Scans the audio for metadata and filetype
/// 
/// # Arguments
/// 
/// - `dir` (`&Path`) - Parent directory path with all the files.
/// 
/// # Returns
/// 
/// - `Vec<TrackMeta>` - Track metadata after scanning the song.
/// 
/// # Examples
/// 
/// ```
/// use crate::...;
/// 
/// let _ = scan_audio();
/// ```
pub fn scan_audio(dir: &Path) -> Vec<TrackMeta> {
    let extensions = ["mp3", "flac", "ogg", "wav", "m4a"];

    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| extensions.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .map(|entry| {
            let path = entry.path().to_path_buf();
            let mut title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string();

            // fallback cases if no data is found
            let mut artist = "Unknown Artist".to_string();
            let mut album = "Unknown Album".to_string();
            let mut duration = "--:--".to_string();
            let mut duration_secs = 0u64;
            let mut artwork = None;

            if let Ok(tagged_file) = Probe::open(&path).and_then(|probe| probe.read()) {
                let tag = tagged_file
                    .primary_tag()
                    .or_else(|| tagged_file.first_tag());
                if let Some(tag) = tag {
                    if let Some(tag_title) = tag.title() {
                        title = tag_title.to_string();
                    }
                    if let Some(tag_artist) = tag.artist() {
                        artist = tag_artist.to_string();
                    }
                    if let Some(tag_album) = tag.album() {
                        album = tag_album.to_string();
                    }
                    artwork = tag.pictures().first().map(|pic| pic.data().to_vec());
                }

                duration_secs = tagged_file.properties().duration().as_secs();
                duration = format!("{}:{:02}", duration_secs / 60, duration_secs % 60); // minutes:seconds (rounded to 2 dp)
            }

            TrackMeta {
                path,
                title,
                artist,
                album,
                duration,
                duration_secs,
                artwork,
                lastfm_title: None,
                lastfm_artist: None,
                linked: true,
            }
        })
        .collect()
}
