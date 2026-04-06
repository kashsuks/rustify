use crate::app::{App, MatchState, Message};
use crate::features::library;
use crate::features::scrobbling::{cache, lastfm, matcher, scrobbler::Scrobbler};
use iced::Task;

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenFolder => Task::perform(library::pick_folder(), Message::FolderPicked),
            Message::FolderPicked(Some(path)) => self.handle_folder_picked(path),
            Message::FolderPicked(None) => Task::none(),
            Message::ScanTrack(idx) => self.scan_track(idx),
            Message::TrackScanned(idx, result) => self.handle_track_scanned(idx, result),
            Message::SearchQueryChanged(query) => {
                if let MatchState::Reviewing { search_query, .. } = &mut self.match_state {
                    *search_query = query;
                }
                Task::none()
            }
            Message::SearchSubmitted => self.submit_review_search(),
            Message::SearchResults(results) => {
                if let MatchState::Reviewing {
                    search_results,
                    search_loading,
                    ..
                } = &mut self.match_state
                {
                    *search_results = results;
                    *search_loading = false;
                }
                Task::none()
            }
            Message::LinkTrack(queue_idx, result) => {
                self.store_link(queue_idx, result.title, result.artist, false);
                self.advance_review()
            }
            Message::SkipTrack(queue_idx) => {
                self.store_link(queue_idx, String::new(), String::new(), true);
                self.queue[queue_idx].linked = false;
                self.advance_review()
            }
            Message::PreviewToggle => {
                if let MatchState::Reviewing {
                    pending,
                    preview_playing,
                    ..
                } = &mut self.match_state
                {
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
                Task::none()
            }
            Message::SelectTrack(idx) => self.select_track(idx),
            Message::Play => {
                self.player.play();
                self.playing = true;
                self.update_discord();
                Task::none()
            }
            Message::Pause => {
                self.player.pause();
                self.playing = false;
                self.update_discord();
                Task::none()
            }
            Message::Next => self.play_next(),
            Message::Previous => self.play_previous(),
            Message::LastfmTick => {
                let api_key = self.lastfm_api_key.clone();
                let username = self.lastfm_username.clone();
                Task::perform(
                    async move { lastfm::get_now_playing(&api_key, &username).await },
                    Message::LastfmUpdated,
                )
            }
            Message::LastfmUpdated(track) => {
                self.lastfm_track = track;
                self.update_discord();
                Task::none()
            }
            Message::StartAuth => {
                let key = self.scrobbler.api_key.clone();
                let secret = self.scrobbler.api_secret.clone();
                Task::perform(
                    async move { Scrobbler::new(key, secret).get_token().await },
                    Message::AuthTokenReceived,
                )
            }
            Message::AuthTokenReceived(Some(token)) => {
                let url = self.scrobbler.auth_url(&token);
                let _ = open::that(url);
                self.auth_token = Some(token);
                self.auth_poll_attempts_left = 15;
                Task::done(Message::AuthPollTick)
            }
            Message::AuthTokenReceived(None) => Task::none(),
            Message::AuthCompleted(Some(session_key)) => {
                self.scrobbler.session_key = Some(session_key);
                self.auth_token = None;
                self.auth_poll_attempts_left = 0;
                println!("Last.fm auth successful!");
                Task::none()
            }
            Message::AuthCompleted(None) => {
                if self.auth_poll_attempts_left > 0 && self.auth_token.is_some() {
                    Task::done(Message::AuthPollTick)
                } else {
                    println!("Last.fm auth timed out or was not approved.");
                    self.auth_token = None;
                    Task::none()
                }
            }
            Message::AuthPollTick => {
                let Some(token) = self.auth_token.clone() else {
                    return Task::none();
                };

                if self.auth_poll_attempts_left == 0 {
                    return Task::none();
                }

                self.auth_poll_attempts_left -= 1;

                let key = self.scrobbler.api_key.clone();
                let secret = self.scrobbler.api_secret.clone();

                Task::perform(
                    async move {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        let mut scrobbler = Scrobbler::new(key, secret);
                        let ok = scrobbler.get_session(&token).await;
                        if ok {
                            scrobbler.session_key
                        } else {
                            None
                        }
                    },
                    Message::AuthCompleted,
                )
            }
            Message::ScrobbleTick => self.handle_scrobble_tick(),
            Message::OpenSettings => {
                self.screen = crate::app::state::Screen::Settings;
                Task::none()
            }
            Message::CloseSettings => {
                self.screen = crate::app::state::Screen::Library;
                Task::none()
            }
            Message::SettingsLastfmUsernameChanged(value) => {
                self.settings_lastfm_username = value;
                Task::none()
            }
            Message::SettingsLastfmApiKeyChanged(value) => {
                self.settings_lastfm_api_key = value;
                Task::none()
            }
            Message::SettingsLastfmApiSecretChanged(value) => {
                self.settings_lastfm_api_secret = value;
                Task::none()
            }
            Message::SaveSettings => {
                if let Err(err) = crate::features::settings::env::write_lastfm_settings(
                    &self.settings_lastfm_api_key,
                    &self.settings_lastfm_api_secret,
                    &self.settings_lastfm_username,
                ) {
                    eprintln!("Failed to save settings: {}", err);
                    return Task::none();
                }

                self.lastfm_api_key = self.settings_lastfm_api_key.clone();
                self.scrobbler.api_key = self.settings_lastfm_api_key.clone();
                self.scrobbler.api_secret = self.settings_lastfm_api_secret.clone();
                self.lastfm_username = self.settings_lastfm_username.clone();
                self.scrobbler = Scrobbler::new(
                    self.lastfm_api_key.clone(),
                    self.settings_lastfm_api_secret.clone(),
                );

                Task::none()
            }
            Message::SettingsApiKeyHoverChanged(value) => {
                self.hover_show_lastfm_api_key = value;
                Task::none()
            }
            Message::SettingsApiSecretHoverChanged(value) => {
                self.hover_show_lastfm_api_secret = value;
                Task::none()
            }
            Message::LibrarySearchChanged(value) => {
                self.library_search = value;
                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let lastfm = iced::time::every(std::time::Duration::from_secs(10))
            .map(|_| Message::LastfmTick);
        let scrobble = iced::time::every(std::time::Duration::from_secs(1))
            .map(|_| Message::ScrobbleTick);

        iced::Subscription::batch(vec![lastfm, scrobble])
    }

    fn handle_folder_picked(&mut self, path: std::path::PathBuf) -> Task<Message> {
        self.queue = library::scan_audio(&path);
        self.current = None;
        self.playing = false;

        if !self.scrobbler.is_authenticated() {
            self.match_state = MatchState::Idle;
            return Task::none();
        }

        for track in &mut self.queue {
            let key = cache::cache_key(
                track
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
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

        let to_scan: Vec<usize> = self
            .queue
            .iter()
            .enumerate()
            .filter(|(_, track)| track.lastfm_title.is_none() && track.linked)
            .map(|(idx, _)| idx)
            .collect();

        if to_scan.is_empty() {
            self.match_state = MatchState::Done;
            return Task::none();
        }

        self.match_state = MatchState::Scanning {
            total: to_scan.len(),
            done: 0,
        };

        Task::done(Message::ScanTrack(to_scan[0]))
    }

    fn scan_track(&self, idx: usize) -> Task<Message> {
        let api_key = self.lastfm_api_key.clone();
        let title = self.queue[idx].title.clone();
        let artist = self.queue[idx].artist.clone();
        let duration_secs = self.queue[idx].duration_secs;

        Task::perform(
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(matcher::SCAN_DELAY_MS)).await;
                let result =
                    matcher::try_auto_match(&api_key, &title, &artist, duration_secs).await;
                (idx, result)
            },
            |(idx, result)| Message::TrackScanned(idx, result),
        )
    }

    fn handle_track_scanned(
        &mut self,
        idx: usize,
        result: matcher::AutoMatchResult,
    ) -> Task<Message> {
        if let matcher::AutoMatchResult::Matched { title, artist } = result {
            self.store_link(idx, title, artist, false);
        }

        if let MatchState::Scanning { done, .. } = &mut self.match_state {
            *done += 1;
        }

        if let Some(next_idx) = self
            .queue
            .iter()
            .enumerate()
            .find(|(next_idx, track)| *next_idx > idx && track.lastfm_title.is_none() && track.linked)
            .map(|(next_idx, _)| next_idx)
        {
            return Task::done(Message::ScanTrack(next_idx));
        }

        let pending: Vec<usize> = self
            .queue
            .iter()
            .enumerate()
            .filter(|(_, track)| track.lastfm_title.is_none() && track.linked)
            .map(|(queue_idx, _)| queue_idx)
            .collect();

        self.start_review(pending)
    }

    fn submit_review_search(&mut self) -> Task<Message> {
        if let MatchState::Reviewing {
            search_query,
            search_loading,
            ..
        } = &mut self.match_state
        {
            *search_loading = true;
            let query = search_query.clone();
            let api_key = self.lastfm_api_key.clone();

            return Task::perform(
                async move { matcher::search_tracks_by_query(&api_key, &query).await },
                Message::SearchResults,
            );
        }

        Task::none()
    }

    fn start_review(&mut self, pending: Vec<usize>) -> Task<Message> {
        if pending.is_empty() {
            self.match_state = MatchState::Done;
            return Task::none();
        }

        let first = pending[0];
        let query = format!("{} {}", self.queue[first].title, self.queue[first].artist);
        let api_key = self.lastfm_api_key.clone();
        let title = self.queue[first].title.clone();
        let artist = self.queue[first].artist.clone();

        self.match_state = MatchState::Reviewing {
            pending,
            search_query: query,
            search_results: vec![],
            search_loading: true,
            preview_playing: false,
        };

        Task::perform(
            async move { matcher::search_tracks(&api_key, &title, &artist).await },
            Message::SearchResults,
        )
    }

    fn advance_review(&mut self) -> Task<Message> {
        self.player.pause();

        let mut pending = match &self.match_state {
            MatchState::Reviewing { pending, .. } => pending.clone(),
            _ => return Task::none(),
        };

        if !pending.is_empty() {
            pending.remove(0);
        }

        self.start_review(pending)
    }

    fn store_link(&mut self, queue_idx: usize, title: String, artist: String, skipped: bool) {
        let filename = self.queue[queue_idx]
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();
        let key = cache::cache_key(&filename, self.queue[queue_idx].duration_secs);
        let link = cache::CachedLink {
            lastfm_title: title.clone(),
            lastfm_artist: artist.clone(),
            skipped,
        };

        self.link_cache.insert(key.clone(), link.clone());
        cache::insert(key, link);

        if !skipped {
            self.queue[queue_idx].lastfm_title = Some(title);
            self.queue[queue_idx].lastfm_artist = Some(artist);
            self.queue[queue_idx].linked = true;
        }
    }

    fn select_track(&mut self, idx: usize) -> Task<Message> {
        if let MatchState::Reviewing {
            preview_playing, ..
        } = &mut self.match_state
        {
            *preview_playing = false;
        }

        self.current = Some(idx);
        self.player.load(&self.queue[idx].path);
        self.player.play();
        self.playing = true;
        self.scrobble_timer = 0.0;
        self.scrobbled = false;
        self.current_duration_secs = self.queue[idx].duration_secs;

        if let Some(session_key) = self.scrobbler.session_key.clone() {
            let api_key = self.scrobbler.api_key.clone();
            let api_secret = self.scrobbler.api_secret.clone();
            let artist = self.queue[idx]
                .lastfm_artist
                .clone()
                .unwrap_or_else(|| self.queue[idx].artist.clone());
            let title = self.queue[idx]
                .lastfm_title
                .clone()
                .unwrap_or_else(|| self.queue[idx].title.clone());
            let album = self.queue[idx].album.clone();

            return Task::perform(
                async move {
                    let scrobbler = Scrobbler::new_with_session(api_key, api_secret, session_key);
                    scrobbler.update_now_playing(&artist, &title, &album).await;
                },
                |_| Message::Play,
            );
        }

        self.update_discord();
        Task::none()
    }

    fn play_next(&mut self) -> Task<Message> {
        if self.queue.is_empty() {
            return Task::none();
        }

        let next = self.current.map(|idx| (idx + 1) % self.queue.len()).unwrap_or(0);
        self.current = Some(next);
        self.player.load(&self.queue[next].path);
        self.player.play();
        self.playing = true;
        self.scrobble_timer = 0.0;
        self.scrobbled = false;
        self.current_duration_secs = self.queue[next].duration_secs;
        self.update_discord();
        Task::none()
    }

    fn play_previous(&mut self) -> Task<Message> {
        if self.queue.is_empty() {
            return Task::none();
        }

        let previous = self.current.map(|idx| idx.saturating_sub(1)).unwrap_or(0);
        self.current = Some(previous);
        self.player.load(&self.queue[previous].path);
        self.player.play();
        self.playing = true;
        self.scrobble_timer = 0.0;
        self.scrobbled = false;
        self.current_duration_secs = self.queue[previous].duration_secs;
        self.update_discord();
        Task::none()
    }

    fn handle_scrobble_tick(&mut self) -> Task<Message> {
        if !self.playing {
            return Task::none();
        }

        self.scrobble_timer += 1.0;
        let threshold = (self.current_duration_secs as f32 * 0.5)
            .min(240.0)
            .max(30.0);

        if self.scrobbled || self.scrobble_timer < threshold {
            return Task::none();
        }

        self.scrobbled = true;

        let Some(track) = self.current.and_then(|idx| self.queue.get(idx)) else {
            return Task::none();
        };

        if !track.linked {
            return Task::none();
        }

        let Some(session_key) = self.scrobbler.session_key.clone() else {
            return Task::none();
        };

        let artist = track
            .lastfm_artist
            .clone()
            .unwrap_or_else(|| track.artist.clone());
        let title = track
            .lastfm_title
            .clone()
            .unwrap_or_else(|| track.title.clone());
        let album = track.album.clone();
        let api_key = self.scrobbler.api_key.clone();
        let api_secret = self.scrobbler.api_secret.clone();

        Task::perform(
            async move {
                let scrobbler = Scrobbler::new_with_session(api_key, api_secret, session_key);
                scrobbler.scrobble(&artist, &title, &album).await;
            },
            |_| Message::ScrobbleTick,
        )
    }

    pub(crate) fn update_discord(&mut self) {
        let mut discord = std::mem::replace(
            &mut self.discord,
            crate::features::discord_rpc::DiscordRpc::connect(""),
        );
        discord.update(self);
        self.discord = discord;
    }
}
