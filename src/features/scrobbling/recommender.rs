use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::TrackMeta;
use crate::features::scrobbling::cache::CachedLink;
use crate::features::scrobbling::lastfm::SimilarTrack;

pub fn rank_candidates(
    queue: &[TrackMeta],
    link_cache: &HashMap<String, CachedLink>,
    similar: &[SimilarTrack],
    current_idx: usize,
) -> Option<usize> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut scores: Vec<(usize, f64)> = queue
        .iter()
        .enumerate()
        .filter(|(idx, track)| {
            *idx != current_idx
                && track.linked
                && !cache_entry(track, link_cache)
                    .map(|e| e.skipped)
                    .unwrap_or(false)
        })
        .map(|(idx, track)| {
            let lastfm_match = similar.iter().any(|s| {
                s.title.to_lowercase() == track.lastfm_title.as_deref().unwrap_or("").to_lowercase()
                    && s.artist.to_lowercase()
                        == track.lastfm_artist.as_deref().unwrap_or("").to_lowercase()
            });

            let recency_bonus = cache_entry(track, link_cache)
                .and_then(|e| e.last_played)
                .map(|ts| {
                    let age_secs = now.saturating_sub(ts) as f64;
                    let age_hours = age_secs / 3600.0;
                    (1.0 - (age_hours / 24.0).min(1.0)) * 0.5
                })
                .unwrap_or(0.0);

            let score = if lastfm_match { 1.0 } else { 0.0 } + recency_bonus;
            (idx, score)
        })
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    scores.into_iter().next().map(|(idx, _)| idx)
}

fn cache_entry<'a>(
    track: &TrackMeta,
    link_cache: &'a HashMap<String, CachedLink>,
) -> Option<&'a CachedLink> {
    let filename = track
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let key = crate::features::scrobbling::cache::cache_key(filename, track.duration_secs);
    link_cache.get(&key)
}
