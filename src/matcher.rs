use serde::Deserialize;

// How long we wait between lastfm scan requests (in ms)
// increase based on rate limits
pub const SCAN_DELAY_MS: u64 = 1000;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub artist: String,
    pub duration_secs: u64,
    pub mbid: String,
}

#[derive(Debug, Clone)]
pub enum AutoMatchResult {
    Matched { title: String, artist: String },
    NeedsReview,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    results: SearchResults,
}

#[derive(Debug, Deserialize)]
struct SearchResults {
    #[serde(rename = "trackmatches")]
    trackmatches: TrackMatches,
}

#[derive(Debug, Deserialize)]
struct TrackMatches {
    track: Vec<TrackMatch>,
}

#[derive(Debug, Deserialize)]
struct TrackMatch {
    name: String,
    artist: String,
    mbid: Option<String>,
    duration: Option<String>,
}

pub async fn search_tracks(
    api_key: &str,
    title: &str,
    artist: &str,
) -> Vec<SearchResult> {
    let url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=track.search\
         &track={}&artist={}&api_key={}&format=json&limit=8",
         urlencoding::encode(title),
         urlencoding::encode(artist),
         api_key
    );

    let Ok(resp) = reqwest::get(&url).await else { return vec![] };
    let Ok(data) = resp.json::<SearchResponse>().await else { return vec![] };
    
    data.results.trackmatches.track
        .into_iter()
        .map(|t| SearchResult {
            duration_secs: t.duration
                .as_deref()
                .and_then(|d| d.parse::<u64>().ok())
                .unwrap_or(0),
            mbid: t.mbid.unwrap_or_default(),
            title: t.name,
            artist: t.artist,
        })
        .collect()
}

pub async fn search_tracks_by_query(
    api_key: &str,
    query: &str,
) -> Vec<SearchResult> {
    let url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=track.search\
         &track={}&api_key={}&format=json&limit=8",
        urlencoding::encode(query),
        api_key
    );

    let Ok(resp) = reqwest::get(&url).await else { return vec![] };
    let Ok(data) = resp.json::<SearchResponse>().await else { return vec![] };

    data.results.trackmatches.track
        .into_iter()
        .map(|t| SearchResult {
            duration_secs: t.duration
                .as_deref()
                .and_then(|d| d.parse::<u64>().ok())
                .unwrap_or(0),
            mbid: t.mbid.unwrap_or_default(),
            title: t.name,
            artist: t.artist,
        })
        .collect()
}

pub async fn try_auto_match(
    api_key: &str,
    title: &str,
    artist: &str,
    duration_secs: u64,
) -> AutoMatchResult {
    let results = search_tracks(api_key, title, artist).await;

    for r in &results {
        let title_match = r.title.to_lowercase() == title.to_lowercase();
        let artist_match = r.artist.to_lowercase() == artist.to_lowercase();
        let duration_match = duration_secs == 0
            || r.duration_secs == 0
            || (r.duration_secs as i64 - duration_secs as i64).abs() <= 10;

        if title_match && artist_match && duration_match {
            return AutoMatchResult::Matched {
                title: r.title.clone(),
                artist: r.artist.clone(),
            };
        }
    }

    AutoMatchResult::NeedsReview
}
