use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Track {
    pub name: String,
    #[serde(rename = "artist")]
    pub artist: Artist,
    #[serde(rename = "album")]
    pub album: Album,
    #[serde(rename = "@attr")]
    pub attr: Option<NowPlayingAttr>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Artist {
    #[serde(rename = "#text")]
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Album {
    #[serde(rename = "#text")]
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NowPlayingAttr {
    pub nowplaying: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RecentTracksInner {
    track: Vec<Track>,
}

#[derive(Debug, Clone, Deserialize)]
struct RecentTrackResponse {
    recenttracks: RecentTracksInner,
}

pub async fn get_now_playing(api_key: &str, username: &str) -> Option<Track> {
    let url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks\
        &user={}&api_key={}&format=json&limit=1",
        username, api_key
    );

    let resp = reqwest::get(&url).await.ok()?;
    let data: RecentTrackResponse = resp.json().await.ok()?;
    let track = data.recenttracks.track.into_iter().next()?;

    if track
        .attr
        .as_ref()
        .map(|attr| attr.nowplaying == "true")
        .unwrap_or(false)
    {
        Some(track)
    } else {
        None
    }
}

pub async fn get_track_info(api_key: &str, artist: &str, track: &str) -> Option<Vec<u8>> {
    let url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=track.getInfo\
         &api_key={}&artist={}&track={}&format=json",
         api_key,
         urlencoding::encode(artist),
         urlencoding::encode(track),
    );

    let json: serde_json::Value = reqwest::get(&url).await.ok()?.json().await.ok()?;

    let image_url = json["track"]["album"]["image"]
        .as_array()?
        .last()?["#text"]
        .as_str()?
        .to_string();

    if image_url.is_empty() {
        return None;
    }

    let bytes = reqwest::get(&image_url).await.ok()?.bytes().await.ok()?;
    Some(bytes.to_vec())
}

#[derive(Debug, Clone)]
pub struct SimilarTrack {
    pub title: String,
    pub artist: String,
}

pub async fn get_similar_tracks(api_key: &str, artist: &str, track: &str) -> Vec<SimilarTrack> {
    let url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=track.getSimilar\
         &api_key={}&artist={}&track={}&limit=20&format=json",
         api_key,
         urlencoding::encode(artist),
         urlencoding::encode(track),
    );

    let Ok(resp) = reqwest::get(&url).await else {
        return vec![];
    };
    let Ok(json) = resp.json::<serde_json::Value>().await else {
        return vec![];
    };

    json["similartracks"]["track"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|t| {
            Some(SimilarTrack {
                title: t["name"].as_str()?.to_string(),
                artist: t["artist"]["name"].as_str()?.to_string(),
            })
        })
        .collect()
}
