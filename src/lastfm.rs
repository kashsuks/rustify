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

pub async fn get_now_playing(
    api_key: &str,
    username: &str
) -> Option<Track> {
    let url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks\
        &user={}&api_key={}&format=json&limit=1",
        username, api_key
    );

    let resp = reqwest::get(&url).await.ok()?;
    let data: RecentTrackResponse = resp.json().await.ok()?;

    let track = data.recenttracks.track.into_iter().next()?;

    // only return if it has a now playing tag
    if track.attr.as_ref().map(|a| a.nowplaying == "true").unwrap_or(false) {
        Some(track)
    } else {
        None
    }
}
