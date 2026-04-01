use iced::{Element, Task, Theme, Length, Color, Padding};
use iced::widget::{button, column, container, scrollable, horizontal_rule, row, text, Space};
use iced::widget::image as iced_image;
use lofty::prelude::*;
use lofty::probe::Probe;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

mod player;
use player::Player;

mod lastfm;
use lastfm::Track as LastfmTrack;

mod scrobbler;
use scrobbler::Scrobbler;

pub fn main() -> iced::Result {
    iced::application("Rustify", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::Nord)
        .run_with(|| (App::new(), Task::none()))
}

struct App {
    player: Player,
    queue: Vec<TrackMeta>,
    current: Option<usize>,
    playing: bool,
    discord: Option<DiscordIpcClient>,
    lastfm_track: Option<LastfmTrack>,
    lastfm_api_key: String,
    lastfm_username: String,
    scrobbler: Scrobbler,
    auth_token: Option<String>,
    scrobble_timer: f32,
    current_duration_secs: u64,
    scrobbled: bool,
}

impl App {
    fn new() -> Self {
        dotenvy::dotenv().ok();
        let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_default();

        let discord = DiscordIpcClient::new(&client_id)
            .ok()
            .and_then(|mut client| {
                client.connect().ok()?;
                Some(client)
            });

        let lastfm_api_key = std::env::var("LASTFM_API_KEY").unwrap_or_default();
        let lastfm_username = std::env::var("LASTFM_USERNAME").unwrap_or_default();
        let api_key = std::env::var("LASTFM_API_KEY").unwrap_or_default();
        let api_secret = std::env::var("LASTFM_API_SECRET").unwrap_or_default();
        eprintln!("DEBUG: api_key len={}, api_secret len={}", api_key.len(), api_secret.len());

        Self {
            player: Player::new(),
            queue: vec![],
            current: None,
            playing: false,
            discord,
            lastfm_track: None,
            lastfm_api_key,
            lastfm_username,
            scrobbler: Scrobbler::new(api_key, api_secret),
            auth_token: None,
            scrobble_timer: 0.0,
            current_duration_secs: 0,
            scrobbled: false,
        }
    }
}

struct TrackMeta {
    path: std::path::PathBuf,
    title: String,
    artist: String,
    album: String,
    duration: String,
    artwork: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
enum Message {
    OpenFolder,
    FolderPicked(Option<std::path::PathBuf>),
    SelectTrack(usize),
    Play,
    Pause,
    Next,
    Previous,
    LastfmTick,
    LastfmUpdated(Option<LastfmTrack>),
    StartAuth,
    AuthTokenReceived(Option<String>),
    CompleteAuth,
    AuthCompleted(Option<String>),  // carries the session key back
    ScrobbleTick,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenFolder => {
                return Task::perform(pick_folder(), Message::FolderPicked);
            }

            Message::FolderPicked(Some(path)) => {
                self.queue = scan_audio(&path);
                self.current = None;
                self.playing = false;
            }

            Message::FolderPicked(None) => {}

            Message::SelectTrack(idx) => {
                self.current = Some(idx);
                self.player.load(&self.queue[idx].path);
                self.player.play();
                self.playing = true;
                self.scrobble_timer = 0.0;
                self.scrobbled = false;
                self.current_duration_secs = parse_duration(&self.queue[idx].duration);

                // send now playing to last.fm immediately
                let artist = self.queue[idx].artist.clone();
                let title = self.queue[idx].title.clone();
                let album = self.queue[idx].album.clone();
                if let Some(sk) = self.scrobbler.session_key.clone() {
                    let key = self.scrobbler.api_key.clone();
                    let secret = self.scrobbler.api_secret.clone();
                    return Task::perform(
                        async move {
                            let s = Scrobbler::new_with_session(key, secret, sk);
                            s.update_now_playing(&artist, &title, &album).await;
                            Message::Play  // dummy — we don't need a response
                        },
                        |m| m,
                    );
                }
                self.update_discord();
            }

            Message::Play => {
                self.player.play();
                self.playing = true;
                self.update_discord();
            }

            Message::Pause => {
                self.player.pause();
                self.playing = false;
                self.update_discord();
            }

            Message::Next => {
                if self.queue.is_empty() { return Task::none(); }
                let next = self.current.map(|i| (i + 1) % self.queue.len()).unwrap_or(0);
                self.current = Some(next);
                self.player.load(&self.queue[next].path);
                self.player.play();
                self.playing = true;
                self.scrobble_timer = 0.0;
                self.scrobbled = false;
                self.current_duration_secs = parse_duration(&self.queue[next].duration);
                self.update_discord();
            }

            Message::Previous => {
                if self.queue.is_empty() { return Task::none(); }
                let prev = self.current.map(|i| i.saturating_sub(1)).unwrap_or(0);
                self.current = Some(prev);
                self.player.load(&self.queue[prev].path);
                self.player.play();
                self.playing = true;
                self.scrobble_timer = 0.0;
                self.scrobbled = false;
                self.current_duration_secs = parse_duration(&self.queue[prev].duration);
                self.update_discord();
            }

            Message::LastfmTick => {
                let api_key = self.lastfm_api_key.clone();
                let username = self.lastfm_username.clone();
                return Task::perform(
                    async move { lastfm::get_now_playing(&api_key, &username).await },
                    Message::LastfmUpdated,
                );
            }

            Message::LastfmUpdated(track) => {
                self.lastfm_track = track;
                self.update_discord();
            }

            Message::StartAuth => {
                let key = self.scrobbler.api_key.clone();
                let secret = self.scrobbler.api_secret.clone();
                return Task::perform(
                    async move {
                        let s = Scrobbler::new(key, secret);
                        s.get_token().await
                    },
                    Message::AuthTokenReceived,
                );
            }

            Message::AuthTokenReceived(Some(token)) => {
                let url = self.scrobbler.auth_url(&token);
                let _ = open::that(url);
                self.auth_token = Some(token);
            }

            Message::AuthTokenReceived(None) => {}

            Message::CompleteAuth => {
                if let Some(token) = self.auth_token.clone() {
                    let key = self.scrobbler.api_key.clone();
                    let secret = self.scrobbler.api_secret.clone();
                    return Task::perform(
                        async move {
                            let mut s = Scrobbler::new(key, secret);
                            let ok = s.get_session(&token).await;
                            if ok { s.session_key } else { None }
                        },
                        Message::AuthCompleted,
                    );
                }
            }

            // session key comes back from the async block, store it on self
            Message::AuthCompleted(Some(sk)) => {
                self.scrobbler.session_key = Some(sk);
                println!("Last.fm auth successful!");
            }

            Message::AuthCompleted(None) => {
                println!("Last.fm auth failed — did you approve it in the browser?");
            }

            Message::ScrobbleTick => {
                if self.playing {
                    self.scrobble_timer += 1.0;

                    let threshold = (self.current_duration_secs as f32 * 0.5)
                        .min(240.0)
                        .max(30.0);

                    if !self.scrobbled && self.scrobble_timer >= threshold {
                        self.scrobbled = true;
                        if let Some(track) = self.current.and_then(|i| self.queue.get(i)) {
                            let artist = track.artist.clone();
                            let title = track.title.clone();
                            let album = track.album.clone();  // fixed: was using title twice
                            if let Some(sk) = self.scrobbler.session_key.clone() {
                                let key = self.scrobbler.api_key.clone();
                                let secret = self.scrobbler.api_secret.clone();
                                return Task::perform(
                                    async move {
                                        let s = Scrobbler::new_with_session(key, secret, sk);
                                        s.scrobble(&artist, &title, &album).await;
                                    },
                                    |_| Message::ScrobbleTick,  // no-op response
                                );
                            }
                        }
                    }
                }
            }
        }

        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let lastfm = iced::time::every(std::time::Duration::from_secs(10))
            .map(|_| Message::LastfmTick);
        let scrobble = iced::time::every(std::time::Duration::from_secs(1))
            .map(|_| Message::ScrobbleTick);
        iced::Subscription::batch(vec![lastfm, scrobble])
    }
}

impl App {
    fn view(&self) -> Element<Message> {
        let track_list = self.track_list_view();
        let now_playing = self.now_playing_view();

        let layout = row![
            track_list,
            now_playing,
        ]
        .height(Length::Fill);

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn track_list_view(&self) -> Element<Message> {
        let auth_btn: Element<Message> = if self.scrobbler.is_authenticated() {
            text("● Last.fm").size(13).into()
        } else {
            button(" Connect Last.fm ").on_press(Message::StartAuth).into()
        };

        let confirm_btn: Element<Message> = if self.auth_token.is_some()
            && !self.scrobbler.is_authenticated()
        {
            button(" I approved it ").on_press(Message::CompleteAuth).into()
        } else {
            Space::with_width(0).into()
        };

        let toolbar = row![
            text("Library").size(22),
            Space::with_width(Length::Fill),
            confirm_btn,
            auth_btn,
            button(" Open Folder ").on_press(Message::OpenFolder),
        ]
        .padding([16, 24])
        .spacing(12)
        .align_y(iced::Alignment::Center);

        let headers = row![
            text("#").size(12).width(40),
            text("Title").size(12).width(Length::Fill),
            text("Artist").size(12).width(160),
            text("Album").size(12).width(180),
            text("Duration").size(12).width(70),
        ]
        .padding([8, 24])
        .spacing(12);

        let body: Element<Message> = if self.queue.is_empty() {
            container(
                text("Open a folder to load music").size(14)
            )
            .padding(40)
            .center_x(Length::Fill)
            .into()
        } else {
            let rows = column(
                self.queue.iter().enumerate().map(|(i, track)| {
                    self.track_row(i, track)
                })
            )
            .spacing(0);

            scrollable(rows)
                .height(Length::Fill)
                .into()
        };

        column![
            toolbar,
            horizontal_rule(1),
            headers,
            horizontal_rule(1),
            body,
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

    fn track_row<'a>(&'a self, idx: usize, track: &'a TrackMeta) -> Element<'a, Message> {
        let is_active = self.current == Some(idx);

        let num_or_indicator: Element<Message> = if is_active && self.playing {
            text(">").size(13).width(40).into()
        } else {
            text(format!("{}", idx + 1)).size(13).width(40).into()
        };

        let title_col = column![
            text(&track.title).size(14),
        ]
        .width(Length::Fill);

        let row_content = row![
            num_or_indicator,
            title_col,
            text(&track.artist).size(13).width(160),
            text(&track.album).size(13).width(180),
            text(&track.duration).size(13).width(70),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .padding([10, 24]);

        button(row_content)
            .on_press(Message::SelectTrack(idx))
            .width(Length::Fill)
            .style(move |_theme, status| {
                let bg = match (is_active, matches!(status, button::Status::Hovered)) {
                    (_, true) => Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                    (true, _) => Color::from_rgba(1.0, 1.0, 1.0, 0.03),
                    _         => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(iced::Background::Color(bg)),
                    text_color: if is_active {
                        Color::WHITE
                    } else {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.85)
                    },
                    border: iced::Border::default(),
                    shadow: iced::Shadow::default(),
                }
            })
            .into()
    }

    fn now_playing_view(&self) -> Element<Message> {
        let current_track = self.current.and_then(|i| self.queue.get(i));

        let title = current_track
            .map(|t| t.title.as_str())
            .unwrap_or("No track selected");

        let artist = current_track
            .map(|t| t.artist.as_str())
            .unwrap_or("");

        let album = current_track
            .map(|t| t.album.as_str())
            .unwrap_or("");

        let art: Element<Message> = match current_track.and_then(|t| t.artwork.as_ref()) {
            Some(bytes) => {
                let handle = iced_image::Handle::from_bytes(bytes.clone());
                iced_image::Image::new(handle)
                    .width(260)
                    .height(260)
                    .into()
            }
            None => {
                container(text("♪").size(64))
                    .width(260)
                    .height(260)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(|_theme| container::Style {
                        background: Some(iced::Background::Color(
                            Color::from_rgb(0.15, 0.15, 0.2)
                        )),
                        border: iced::Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into()
            }
        };

        let info = column![
            text(title).size(20),
            text(artist).size(14),
            text(album).size(12),
        ]
        .spacing(4)
        .padding(Padding { top: 16.0, right: 0.0, bottom: 0.0, left: 0.0 });

        let play_pause = if self.playing {
            button("  ⏸  ").on_press(Message::Pause)
        } else {
            button("  ▶  ").on_press(Message::Play)
        };

        let controls = row![
            button("  ⏮  ").on_press(Message::Previous),
            play_pause,
            button("  ⏭  ").on_press(Message::Next),
        ]
        .spacing(12)
        .padding(Padding { top: 20.0, right: 0.0, bottom: 0.0, left: 0.0 });

        let lastfm_status: Element<Message> = if let Some(track) = &self.lastfm_track {
            column![
                text("▸ Last.fm").size(11),
                text(&track.name).size(13),
                text(&track.artist.text).size(11),
            ]
            .spacing(2)
            .padding(Padding { top: 12.0, right: 0.0, bottom: 0.0, left: 0.0 })
            .into()
        } else {
            Space::with_height(0).into()
        };

        let panel = column![
            art,
            info,
            controls,
            lastfm_status,
        ]
        .padding(24)
        .width(300)
        .height(Length::Fill)
        .align_x(iced::Alignment::Center);

        container(panel)
            .height(Length::Fill)
            .width(300)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.1))),
                ..Default::default()
            })
            .into()
    }

    fn update_discord(&mut self) {
        let Some(client) = self.discord.as_mut() else { return };

        let (details, state) = if let Some(track) = &self.lastfm_track {
            (
                track.name.clone(),
                format!("{} — {}", track.artist.text, track.album.text),
            )
        } else if self.playing {
            if let Some(track) = self.current.and_then(|i| self.queue.get(i)) {
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
                        .large_text(&details)
                )
                .buttons(vec![
                    activity::Button::new("🎵 Rustify", "https://github.com/kashsuks/Rustify")
                ])
        );
    }
}

fn parse_duration(s: &str) -> u64 {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.as_slice() {
        [m, s] => m.parse::<u64>().unwrap_or(0) * 60 + s.parse::<u64>().unwrap_or(0),
        _ => 0,
    }
}

async fn pick_folder() -> Option<std::path::PathBuf> {
    rfd::AsyncFileDialog::new()
        .pick_folder()
        .await
        .map(|f| f.path().to_path_buf())
}

fn scan_audio(dir: &std::path::Path) -> Vec<TrackMeta> {
    let extensions = ["mp3", "flac", "ogg", "wav", "m4a"];
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| extensions.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .map(|e| {
            let path = e.path().to_path_buf();

            let mut title = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let mut artist = "Unknown Artist".to_string();
            let mut album = "Unknown Album".to_string();
            let mut duration = "--:--".to_string();
            let mut artwork = None;

            if let Ok(tagged_file) = Probe::open(&path).and_then(|p| p.read()) {
                let tag = tagged_file
                    .primary_tag()
                    .or_else(|| tagged_file.first_tag());

                if let Some(tag) = tag {
                    if let Some(t)  = tag.title()  { title  = t.to_string(); }
                    if let Some(a)  = tag.artist() { artist = a.to_string(); }
                    if let Some(al) = tag.album()  { album  = al.to_string(); }

                    artwork = tag.pictures().first()
                        .map(|pic| pic.data().to_vec());
                }

                let secs = tagged_file.properties().duration().as_secs();
                duration = format!("{}:{:02}", secs / 60, secs % 60);
            }

            TrackMeta { path, title, artist, album, duration, artwork }
        })
        .collect()
}
