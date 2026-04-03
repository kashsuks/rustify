use iced::{Element, Task, Theme, Length, Color, Padding};
use iced::widget::{
    button, column, container, scrollable, horizontal_rule,
    row, text, Space, text_input,
};
use lucide_icons::{Icon, LUCIDE_FONT_BYTES};
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

mod cache;
mod matcher;
use matcher::{AutoMatchResult, SearchResult, SCAN_DELAY_MS};

pub fn main() -> iced::Result {
    iced::application("Rustify", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::Nord)
        .font(LUCIDE_FONT_BYTES)
        .run_with(|| (App::new(), Task::none()))
}

#[derive(Debug, Clone)]
enum MatchState {
    Idle,
    Scanning { total: usize, done: usize },
    Reviewing {
        pending: Vec<usize>,
        search_query: String,
        search_results: Vec<SearchResult>,
        search_loading: bool,
        preview_playing: bool,
    },
    Done,
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
    match_state: MatchState,
    link_cache: std::collections::HashMap<String, cache::CachedLink>,
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
        let link_cache = cache::load();

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
            match_state: MatchState::Idle,
            link_cache,
        }
    }
}

struct TrackMeta {
    path: std::path::PathBuf,
    title: String,
    artist: String,
    album: String,
    duration: String,
    duration_secs: u64,
    artwork: Option<Vec<u8>>,
    lastfm_title: Option<String>,
    lastfm_artist: Option<String>,
    linked: bool,   // false = skipped, won't scrobble
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
    AuthCompleted(Option<String>),
    ScrobbleTick,
    ScanTrack(usize),
    TrackScanned(usize, AutoMatchResult),
    SearchQueryChanged(String),
    SearchSubmitted,
    SearchResults(Vec<SearchResult>),
    LinkTrack(usize, SearchResult),
    SkipTrack(usize),
    PreviewToggle,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {

            // ── Folder ───────────────────────────────────────────────────────

            Message::OpenFolder => {
                return Task::perform(pick_folder(), Message::FolderPicked);
            }

            Message::FolderPicked(Some(path)) => {
                self.queue = scan_audio(&path);
                self.current = None;
                self.playing = false;

                if !self.scrobbler.is_authenticated() {
                    self.match_state = MatchState::Idle;
                    return Task::none();
                }

                // apply cache hits immediately
                for track in &mut self.queue {
                    let key = cache::cache_key(
                        track.path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(""),
                        track.duration_secs,
                    );
                    if let Some(cached) = self.link_cache.get(&key) {
                        if cached.skipped {
                            track.linked = false;
                        } else {
                            track.lastfm_title = Some(cached.lastfm_title.clone());
                            track.lastfm_artist = Some(cached.lastfm_artist.clone());
                            track.linked = true;
                        }
                    }
                }

                // find tracks that still need scanning
                let to_scan: Vec<usize> = self.queue.iter().enumerate()
                    .filter(|(_, t)| t.lastfm_title.is_none() && t.linked)
                    .map(|(i, _)| i)
                    .collect();

                if to_scan.is_empty() {
                    self.match_state = MatchState::Done;
                    return Task::none();
                }

                self.match_state = MatchState::Scanning {
                    total: to_scan.len(),
                    done: 0,
                };

                return Task::done(Message::ScanTrack(to_scan[0]));
            }

            Message::FolderPicked(None) => {}

            Message::ScanTrack(idx) => {
                let api_key = self.lastfm_api_key.clone();
                let title = self.queue[idx].title.clone();
                let artist = self.queue[idx].artist.clone();
                let dur = self.queue[idx].duration_secs;
                return Task::perform(
                    async move {
                        tokio::time::sleep(
                            std::time::Duration::from_millis(SCAN_DELAY_MS)
                        ).await;
                        let result = matcher::try_auto_match(&api_key, &title, &artist, dur).await;
                        (idx, result)
                    },
                    |(idx, result)| Message::TrackScanned(idx, result),
                );
            }

            Message::TrackScanned(idx, result) => {
                match &result {
                    AutoMatchResult::Matched { title, artist } => {
                        let filename = self.queue[idx].path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();
                        let key = cache::cache_key(&filename, self.queue[idx].duration_secs);
                        let link = cache::CachedLink {
                            lastfm_title: title.clone(),
                            lastfm_artist: artist.clone(),
                            skipped: false,
                        };
                        self.link_cache.insert(key.clone(), link.clone());
                        cache::insert(key, link);
                        self.queue[idx].lastfm_title = Some(title.clone());
                        self.queue[idx].lastfm_artist = Some(artist.clone());
                        self.queue[idx].linked = true;
                    }
                    AutoMatchResult::NeedsReview => {
                        // leave linked = true but lastfm fields empty for now
                    }
                }

                // advance scan
                if let MatchState::Scanning { total: _, done } = &mut self.match_state {
                    *done += 1;
                }

                if let Some(next_idx) = self.queue.iter().enumerate()
                    .find(|(i, t)| *i > idx && t.lastfm_title.is_none() && t.linked)
                    .map(|(i, _)| i)
                {
                    return Task::done(Message::ScanTrack(next_idx));
                }

                // all scanned — collect those needing review
                let pending: Vec<usize> = self.queue.iter().enumerate()
                    .filter(|(_, t)| t.lastfm_title.is_none() && t.linked)
                    .map(|(i, _)| i)
                    .collect();

                if pending.is_empty() {
                    self.match_state = MatchState::Done;
                } else {
                    let first = pending[0];
                    let query = format!(
                        "{} {}",
                        self.queue[first].title,
                        self.queue[first].artist
                    );
                    let api_key = self.lastfm_api_key.clone();
                    let title = self.queue[first].title.clone();
                    let artist = self.queue[first].artist.clone();
                    self.match_state = MatchState::Reviewing {
                        pending,
                        search_query: query.clone(),
                        search_results: vec![],
                        search_loading: true,
                        preview_playing: false,
                    };
                    return Task::perform(
                        async move {
                            matcher::search_tracks(&api_key, &title, &artist).await
                        },
                        Message::SearchResults,
                    );
                }
            }

            Message::SearchQueryChanged(q) => {
                if let MatchState::Reviewing { search_query, .. } = &mut self.match_state {
                    *search_query = q;
                }
            }

            Message::SearchSubmitted => {
                if let MatchState::Reviewing {
                    search_query,
                    search_loading,
                    ..
                } = &mut self.match_state {
                    *search_loading = true;
                    let query = search_query.clone();
                    let api_key = self.lastfm_api_key.clone();
                    return Task::perform(
                        async move {
                            matcher::search_tracks_by_query(&api_key, &query).await
                        },
                        Message::SearchResults,
                    );
                }
            }

            Message::SearchResults(results) => {
                if let MatchState::Reviewing {
                    search_results,
                    search_loading,
                    ..
                } = &mut self.match_state {
                    *search_results = results;
                    *search_loading = false;
                }
            }

            Message::LinkTrack(queue_idx, result) => {
                let filename = self.queue[queue_idx].path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                let key = cache::cache_key(&filename, self.queue[queue_idx].duration_secs);
                let link = cache::CachedLink {
                    lastfm_title: result.title.clone(),
                    lastfm_artist: result.artist.clone(),
                    skipped: false,
                };
                self.link_cache.insert(key.clone(), link.clone());
                cache::insert(key, link);
                self.queue[queue_idx].lastfm_title = Some(result.title);
                self.queue[queue_idx].lastfm_artist = Some(result.artist);
                self.queue[queue_idx].linked = true;

                return self.advance_review();
            }

            Message::SkipTrack(queue_idx) => {
                let filename = self.queue[queue_idx].path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                let key = cache::cache_key(&filename, self.queue[queue_idx].duration_secs);
                let link = cache::CachedLink {
                    lastfm_title: String::new(),
                    lastfm_artist: String::new(),
                    skipped: true,
                };
                self.link_cache.insert(key.clone(), link.clone());
                cache::insert(key, link);
                self.queue[queue_idx].linked = false;

                return self.advance_review();
            }

            Message::PreviewToggle => {
                if let MatchState::Reviewing { pending, preview_playing, .. } = &mut self.match_state {
                    let idx = pending[0];
                    if *preview_playing {
                        self.player.pause();
                        *preview_playing = false;
                    } else {
                        self.player.load(&self.queue[idx].path);
                        self.player.play();
                        *preview_playing = true;
                    }
                }
            }

            Message::SelectTrack(idx) => {
                // stop any modal preview
                if let MatchState::Reviewing { preview_playing, .. } = &mut self.match_state {
                    *preview_playing = false;
                }
                self.current = Some(idx);
                self.player.load(&self.queue[idx].path);
                self.player.play();
                self.playing = true;
                self.scrobble_timer = 0.0;
                self.scrobbled = false;
                self.current_duration_secs = self.queue[idx].duration_secs;

                if let Some(sk) = self.scrobbler.session_key.clone() {
                    let key = self.scrobbler.api_key.clone();
                    let secret = self.scrobbler.api_secret.clone();
                    let artist = self.queue[idx].lastfm_artist.clone()
                        .unwrap_or_else(|| self.queue[idx].artist.clone());
                    let title = self.queue[idx].lastfm_title.clone()
                        .unwrap_or_else(|| self.queue[idx].title.clone());
                    let album = self.queue[idx].album.clone();
                    return Task::perform(
                        async move {
                            let s = Scrobbler::new_with_session(key, secret, sk);
                            s.update_now_playing(&artist, &title, &album).await;
                        },
                        |_| Message::Play,
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
                self.current_duration_secs = self.queue[next].duration_secs;
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
                self.current_duration_secs = self.queue[prev].duration_secs;
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
                            if track.linked {
                                let artist = track.lastfm_artist.clone()
                                    .unwrap_or_else(|| track.artist.clone());
                                let title = track.lastfm_title.clone()
                                    .unwrap_or_else(|| track.title.clone());
                                let album = track.album.clone();
                                if let Some(sk) = self.scrobbler.session_key.clone() {
                                    let key = self.scrobbler.api_key.clone();
                                    let secret = self.scrobbler.api_secret.clone();
                                    return Task::perform(
                                        async move {
                                            let s = Scrobbler::new_with_session(key, secret, sk);
                                            s.scrobble(&artist, &title, &album).await;
                                        },
                                        |_| Message::ScrobbleTick,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Task::none()
    }

    // advance to next pending track or close modal
    fn advance_review(&mut self) -> Task<Message> {
        // stop preview
        self.player.pause();

        if let MatchState::Reviewing { pending, .. } = &mut self.match_state {
            pending.remove(0);
            if pending.is_empty() {
                self.match_state = MatchState::Done;
                return Task::none();
            }
            let next_idx = pending[0];
            let query = format!(
                "{} {}",
                self.queue[next_idx].title,
                self.queue[next_idx].artist
            );
            let api_key = self.lastfm_api_key.clone();
            let title = self.queue[next_idx].title.clone();
            let artist = self.queue[next_idx].artist.clone();
            *pending = pending.clone(); // satisfy borrow checker
            self.match_state = MatchState::Reviewing {
                pending: {
                    if let MatchState::Reviewing { pending, .. } = &self.match_state {
                        pending.clone()
                    } else { vec![] }
                },
                search_query: query,
                search_results: vec![],
                search_loading: true,
                preview_playing: false,
            };
            return Task::perform(
                async move {
                    matcher::search_tracks(&api_key, &title, &artist).await
                },
                Message::SearchResults,
            );
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
    fn view(&self) -> Element<'_, Message> {
        let is_reviewing = matches!(self.match_state, MatchState::Reviewing { .. });

        let main_ui = self.main_view(is_reviewing);

        if is_reviewing {
            if let MatchState::Reviewing {
                pending,
                search_query,
                search_results,
                search_loading,
                preview_playing,
            } = &self.match_state {
                let modal = self.review_modal(
                    pending,
                    search_query,
                    search_results,
                    *search_loading,
                    *preview_playing,
                );

                // Overlay the modal on top of the dimmed main UI.
                return iced::widget::stack([main_ui, modal])
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
            }
        }

        main_ui
    }

    fn main_view(&self, dimmed: bool) -> Element<'_, Message> {
        let track_list = self.track_list_view(dimmed);
        let now_playing = self.now_playing_view();

        let layout = row![track_list, now_playing].height(Length::Fill);

        // scanning progress banner
        let banner: Element<Message> = match &self.match_state {
            MatchState::Scanning { total, done } => {
                container(
                    text(format!("Matching tracks with Last.fm… {}/{}", done, total)).size(13)
                )
                .padding([6, 24])
                .width(Length::Fill)
                .into()
            }
            _ => Space::with_height(0).into(),
        };

        container(column![banner, layout])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme| {
                if dimmed {
                    container::Style {
                        background: Some(iced::Background::Color(
                            Color::from_rgba(0.0, 0.0, 0.0, 0.6)
                        )),
                        ..container::Style::default()
                    }
                } else {
                    container::Style::default()
                }
            })
            .into()
    }

    fn review_modal<'a>(
        &'a self,
        pending: &'a [usize],
        search_query: &'a str,
        search_results: &'a [SearchResult],
        search_loading: bool,
        preview_playing: bool,
    ) -> Element<'a, Message> {
        let queue_idx = pending[0];
        let track = &self.queue[queue_idx];

        let remaining = pending.len();

        let art: Element<Message> = match &track.artwork {
            Some(bytes) => {
                let handle = iced_image::Handle::from_bytes(bytes.clone());
                iced_image::Image::new(handle).width(140).height(140).into()
            }
            None => container(text("♪").size(48))
                .width(140)
                .height(140)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.15, 0.2))),
                    border: iced::Border { radius: 8.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .into(),
        };

        let preview_btn = button(
            container(
                if preview_playing {
                    iced::widget::Text::from(Icon::CirclePause).size(28)
                } else {
                    iced::widget::Text::from(Icon::CirclePlay).size(28)
                }
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
        )
        .on_press(Message::PreviewToggle)
        .width(140)
        .height(140)
        .style(|_theme, _status| button::Style {
            background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
            text_color: Color::WHITE,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
        });

        // hover overlay to always show preview button on top of art
        let art_with_preview = container(
            iced::widget::stack([
                art,
                container(preview_btn).width(140).height(140).into(),
            ])
        )
        .width(140)
        .height(140);

        let local_info = column![
            text(&track.title).size(15),
            text(&track.artist).size(13),
            text(&track.duration).size(12),
            text(
                track.path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
            ).size(11),
        ]
        .spacing(4)
        .padding([12, 0]);

        let left_panel = column![art_with_preview, local_info]
            .width(180)
            .spacing(0)
            .align_x(iced::Alignment::Center);

        let arrow = container(text("→").size(32))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(60);

        let search_box = text_input("Search Last.fm…", search_query)
            .on_input(Message::SearchQueryChanged)
            .on_submit(Message::SearchSubmitted)
            .padding(10)
            .size(14);

        let results_list: Element<Message> = if search_loading {
            container(text("Searching…").size(13))
                .padding(12)
                .into()
        } else if search_results.is_empty() {
            container(text("No results found").size(13))
                .padding(12)
                .into()
        } else {
            let items = column(
                search_results.iter().map(|r| {
                    let r_clone = r.clone();
                    let dur_str = if r.duration_secs > 0 {
                        format!("{}:{:02}", r.duration_secs / 60, r.duration_secs % 60)
                    } else {
                        "--:--".to_string()
                    };

                    let row_content = row![
                        column![
                            text(&r.title).size(14),
                            text(format!("{} · {}", r.artist, dur_str)).size(12),
                        ]
                        .width(Length::Fill)
                        .spacing(2),
                        button(" Link ")
                            .on_press(Message::LinkTrack(queue_idx, r_clone))
                            .style(|_theme, _status| button::Style {
                                background: Some(iced::Background::Color(
                                    Color::from_rgb(0.2, 0.6, 0.3)
                                )),
                                text_color: Color::WHITE,
                                border: iced::Border::default(),
                                shadow: iced::Shadow::default(),
                            }),
                    ]
                    .spacing(12)
                    .align_y(iced::Alignment::Center)
                    .padding([8, 4]);

                    button(row_content)
                        .on_press(Message::LinkTrack(queue_idx, r.clone()))
                        .width(Length::Fill)
                        .style(|_theme, status| {
                            let bg = if matches!(status, button::Status::Hovered) {
                                Color::from_rgba(1.0, 1.0, 1.0, 0.05)
                            } else {
                                Color::TRANSPARENT
                            };
                            button::Style {
                                background: Some(iced::Background::Color(bg)),
                                text_color: Color::WHITE,
                                border: iced::Border::default(),
                                shadow: iced::Shadow::default(),
                            }
                        })
                        .into()
                })
            )
            .spacing(2);

            scrollable(items).height(280).into()
        };

        let right_panel = column![
            search_box,
            results_list,
        ]
        .width(320)
        .spacing(8);

        let header = row![
            text(format!("Unrecognized track — {} remaining", remaining)).size(16),
            Space::with_width(Length::Fill),
            button(" Skip ")
                .on_press(Message::SkipTrack(queue_idx))
                .style(|_theme, _status| button::Style {
                    background: Some(iced::Background::Color(Color::from_rgba(1.0,1.0,1.0,0.1))),
                    text_color: Color::WHITE,
                    border: iced::Border::default(),
                    shadow: iced::Shadow::default(),
                }),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(12)
        .padding([0, 16]);

        let body = row![left_panel, arrow, right_panel]
            .spacing(16)
            .align_y(iced::Alignment::Start);

        let modal_box = container(
            column![header, horizontal_rule(1), body].spacing(16).padding(32)
        )
        .width(700)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.1, 0.1, 0.14))),
            border: iced::Border {
                radius: 12.0.into(),
                width: 1.0,
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.1),
            },
            ..Default::default()
        });

        // full-screen dark overlay with modal centered
        let overlay = container(
            container(modal_box)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.75))),
            ..Default::default()
        });

        iced::widget::opaque(overlay).into()
    }

    fn track_list_view(&self, _dimmed: bool) -> Element<'_, Message> {
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
            container(text("Open a folder to load music").size(14))
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
            scrollable(rows).height(Length::Fill).into()
        };

        column![toolbar, horizontal_rule(1), headers, horizontal_rule(1), body]
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn track_row<'a>(&'a self, idx: usize, track: &'a TrackMeta) -> Element<'a, Message> {
        let is_active = self.current == Some(idx);
        let is_reviewing = matches!(self.match_state, MatchState::Reviewing { .. });

        let num_or_indicator: Element<Message> = if is_active && self.playing {
            text(">").size(13).width(40).into()
        } else {
            text(format!("{}", idx + 1)).size(13).width(40).into()
        };

        // show a small dot if track is unlinked/skipped
        let title_display = if !track.linked {
            format!("⊘ {}", track.title)
        } else if track.lastfm_title.is_none() && self.scrobbler.is_authenticated() {
            format!("? {}", track.title)
        } else {
            track.title.clone()
        };

        let title_col = column![text(title_display).size(14)].width(Length::Fill);

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

        // disable clicks while reviewing
        let btn = button(row_content).width(Length::Fill);
        let btn = if is_reviewing {
            btn
        } else {
            btn.on_press(Message::SelectTrack(idx))
        };

        btn.style(move |_theme, status| {
            let bg = match (is_active, matches!(status, button::Status::Hovered)) {
                (_, true) if !is_reviewing => Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                (true, _) => Color::from_rgba(1.0, 1.0, 1.0, 0.03),
                _ => Color::TRANSPARENT,
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

    fn now_playing_view(&self) -> Element<'_, Message> {
        let current_track = self.current.and_then(|i| self.queue.get(i));

        let title = current_track.map(|t| t.title.as_str()).unwrap_or("No track selected");
        let artist = current_track.map(|t| t.artist.as_str()).unwrap_or("");
        let album = current_track.map(|t| t.album.as_str()).unwrap_or("");

        let art: Element<Message> = match current_track.and_then(|t| t.artwork.as_ref()) {
            Some(bytes) => {
                let handle = iced_image::Handle::from_bytes(bytes.clone());
                iced_image::Image::new(handle).width(260).height(260).into()
            }
            None => container(text("♪").size(64))
                .width(260).height(260)
                .center_x(Length::Fill).center_y(Length::Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.15, 0.2))),
                    border: iced::Border { radius: 8.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .into(),
        };

        let info = column![
            text(title).size(20),
            text(artist).size(14),
            text(album).size(12),
        ]
        .spacing(4)
        .padding(Padding { top: 16.0, right: 0.0, bottom: 0.0, left: 0.0 });

        let previous_btn = button(
            iced::widget::Text::from(Icon::SkipBack)
                .size(22)
        )
        .on_press(Message::Previous)
        .padding([10, 14]);

        let play_pause = if self.playing {
            button(
                iced::widget::Text::from(Icon::CirclePause)
                    .size(22)
            )
            .on_press(Message::Pause)
            .padding([10, 14])
        } else {
            button(
                iced::widget::Text::from(Icon::CirclePlay)
                    .size(22)
            )
            .on_press(Message::Play)
            .padding([10, 14])
        };

        let next_btn = button(
            iced::widget::Text::from(Icon::SkipForward)
                .size(22)
        )
        .on_press(Message::Next)
        .padding([10, 14]);

        let controls = row![
            previous_btn,
            play_pause,
            next_btn,
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

        let panel = column![art, info, controls, lastfm_status]
            .padding(24).width(300).height(Length::Fill)
            .align_x(iced::Alignment::Center);

        container(panel)
            .height(Length::Fill).width(300)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.1))),
                ..Default::default()
            })
            .into()
    }

    fn update_discord(&mut self) {
        let Some(client) = self.discord.as_mut() else { return };

        let (details, state) = if let Some(track) = &self.lastfm_track {
            (track.name.clone(), format!("{} — {}", track.artist.text, track.album.text))
        } else if self.playing {
            if let Some(track) = self.current.and_then(|i| self.queue.get(i)) {
                (track.title.clone(), format!("{} — {}", track.artist, track.album))
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
                .and_then(|s| s.to_str()).unwrap_or("Unknown").to_string();
            let mut artist = "Unknown Artist".to_string();
            let mut album = "Unknown Album".to_string();
            let mut duration = "--:--".to_string();
            let mut duration_secs = 0u64;
            let mut artwork = None;

            if let Ok(tagged_file) = Probe::open(&path).and_then(|p| p.read()) {
                let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());
                if let Some(tag) = tag {
                    if let Some(t)  = tag.title()  { title  = t.to_string(); }
                    if let Some(a)  = tag.artist() { artist = a.to_string(); }
                    if let Some(al) = tag.album()  { album  = al.to_string(); }
                    artwork = tag.pictures().first().map(|pic| pic.data().to_vec());
                }
                duration_secs = tagged_file.properties().duration().as_secs();
                duration = format!("{}:{:02}", duration_secs / 60, duration_secs % 60);
            }

            TrackMeta {
                path, title, artist, album, duration, duration_secs,
                artwork,
                lastfm_title: None,
                lastfm_artist: None,
                linked: true,
            }
        })
        .collect()
}
