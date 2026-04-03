use crate::app::App;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

pub struct DiscordRpc {
    client: Option<DiscordIpcClient>,
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

        let (details, state) = if let Some(track) = &app.lastfm_track {
            (
                track.name.clone(),
                format!("{} — {}", track.artist.text, track.album.text),
            )
        } else if app.playing {
            if let Some(track) = app.current.and_then(|i| app.queue.get(i)) {
                (
                    track.title.clone(),
                    format!("{} — {}", track.artist, track.album),
                )
            } else {
                let _ = client.clear_activity();
                return;
            }
        } else {
            let _ = client.clear_activity();
            return;
        };

        let _ = client.set_activity(
            activity::Activity::new()
                .details(&details)
                .state(&state)
                .assets(
                    activity::Assets::new()
                        .large_image("music")
                        .large_text(&details),
                )
                .buttons(vec![activity::Button::new(
                    "🎵 Rustify",
                    "https://github.com/kashsuks/Rustify",
                )]),
        );
    }
}
