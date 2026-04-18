use crate::app::App;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

pub struct DiscordRpc {
    client: Option<DiscordIpcClient>,
}

pub async fn upload_artwork(bytes: Vec<u8>) -> Option<String> {
    let api_key = std::env::var("IMGBB_API_KEY").ok()?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    let resp = reqwest::Client::new()
        .post("https://api.imgbb.com/1/upload")
        .form(&[("key", api_key.as_str()), ("image", b64.as_str())])
        .send()
        .await
        .ok()?;

    let json: serde_json::Value = resp.json().await.ok()?;
    json["data"]["url"].as_str().map(|s| s.to_string())
}

impl DiscordRpc {
    pub fn connect(client_id: &str) -> Self {
        let client = DiscordIpcClient::new(client_id)
            .ok()
            .and_then(|mut client| {
                client.connect().ok()?;
                Some(client)
            });

        Self { client }
    }

    pub fn update(&mut self, app: &App) {
        let Some(client) = self.client.as_mut() else {
            return;
        };

        let (title, artist, album, duration_secs) = if let Some(track) = &app.lastfm_track {
            (
                track.name.clone(),
                track.artist.text.clone(),
                track.album.text.clone(),
                app.current_duration_secs,
            )
        } else if app.playing {
            if let Some(track) = app.current.and_then(|i| app.queue.get(i)) {
                (
                    track
                        .lastfm_title
                        .clone()
                        .unwrap_or_else(|| track.title.clone()),
                    track
                        .lastfm_artist
                        .clone()
                        .unwrap_or_else(|| track.artist.clone()),
                    track.album.clone(),
                    track.duration_secs,
                )
            } else {
                let _ = client.clear_activity();
                return;
            }
        } else {
            let _ = client.clear_activity();
            return;
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let elapsed_secs = app.scrobble_timer as i64;
        let remaining_secs = (duration_secs as i64 - elapsed_secs).max(0);
        let start_timestamp = now - elapsed_secs;
        let end_timestamp = now + remaining_secs;

        let details = title.clone();
        let state = format!("{} - {}", artist, album);
        let large_text = format!("{} · {}", title, artist);
        let art = app.discord_artwork_url.as_deref().unwrap_or("music");

        let _ = client.set_activity(
            activity::Activity::new()
                .details(&details)
                .state(&state)
                .assets(
                    activity::Assets::new()
                        .large_image(art)
                        .large_text(&large_text)
                        .small_image("rustify_logo")
                        .small_text("Rustify"),
                )
                .timestamps(
                    activity::Timestamps::new()
                        .start(start_timestamp)
                        .end(end_timestamp),
                )
                .buttons(vec![activity::Button::new(
                    "🎵 Rustify",
                    "https://github.com/kashsuks/Rustify",
                )]),
        );
    }
}
