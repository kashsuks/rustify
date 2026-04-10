use crate::audio::player::Player;
use crate::features::discord_rpc::DiscordRpc;
use crate::features::scrobbling::cache::{self, CachedLink};
use crate::features::scrobbling::lastfm::Track as LastfmTrack;
use crate::features::scrobbling::matcher::{AutoMatchResult, SearchResult};
use crate::features::scrobbling::scrobbler::Scrobbler;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum MatchState {
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

#[derive(Debug, Clone)]
pub enum Message {
    OpenFolder,
    FolderPicked(Option<PathBuf>),
    SelectTrack(usize),
    Play,
    Pause,
    Next,
    Previous,
    LastfmTick,
    LastfmUpdated(Option<LastfmTrack>),
    StartAuth,
    AuthTokenReceived(Option<String>),
    AuthPollTick,
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
    OpenSettings,
    CloseSettings,
    SettingsLastfmUsernameChanged(String),
    SettingsLastfmApiKeyChanged(String),
    SettingsLastfmApiSecretChanged(String),
    SettingsApiKeyHoverChanged(bool),
    SettingsApiSecretHoverChanged(bool),
    LibrarySearchChanged(String),
    SaveSettings,
}


pub struct TrackMeta {
    pub(crate) path: PathBuf,
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) album: String,
    pub(crate) duration: String,
    pub(crate) duration_secs: u64,
    pub(crate) artwork: Option<Vec<u8>>,
    pub(crate) lastfm_title: Option<String>,
    pub(crate) lastfm_artist: Option<String>,
    pub(crate) linked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Library,
    Settings,
}

pub struct App {
    pub(crate) player: Player,
    pub(crate) queue: Vec<TrackMeta>,
    pub(crate) current: Option<usize>,
    pub(crate) playing: bool,
    pub(crate) discord: DiscordRpc,
    pub(crate) lastfm_track: Option<LastfmTrack>,
    pub(crate) lastfm_api_key: String,
    pub(crate) lastfm_username: String,
    pub(crate) scrobbler: Scrobbler,
    pub(crate) auth_token: Option<String>,
    pub(crate) auth_poll_attempts_left: u8,
    pub(crate) scrobble_timer: f32,
    pub(crate) current_duration_secs: u64,
    pub(crate) scrobbled: bool,
    pub(crate) match_state: MatchState,
    pub(crate) link_cache: HashMap<String, CachedLink>,
    pub(crate) screen: Screen,
    pub(crate) settings_lastfm_username: String,
    pub(crate) settings_lastfm_api_key: String,
    pub(crate) settings_lastfm_api_secret: String,
    pub(crate) hover_show_lastfm_api_key: bool,
    pub(crate) hover_show_lastfm_api_secret: bool,
    pub(crate) library_search: String,
}

impl App {
    pub fn new() -> Self {
        dotenvy::dotenv().ok();

        let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_default();
        let lastfm_api_key = std::env::var("LASTFM_API_KEY").unwrap_or_default();
        let lastfm_username = std::env::var("LASTFM_USERNAME").unwrap_or_default();
        let api_key = std::env::var("LASTFM_API_KEY").unwrap_or_default();
        let api_secret = std::env::var("LASTFM_API_SECRET").unwrap_or_default();
        let session_key = std::env::var("LASTFM_SESSION_KEY").ok();

        let scrobbler = match session_key {
            Some(ref sk) => Scrobbler::new_with_session(api_key.clone(), api_secret.clone(), sk.clone()),
            None => Scrobbler::new(api_key.clone(), api_secret.clone()),
        };

        Self {
            player: Player::new(),
            queue: vec![],
            current: None,
            playing: false,
            discord: DiscordRpc::connect(&client_id),
            lastfm_track: None,
            lastfm_api_key,
            lastfm_username: lastfm_username.clone(),
            scrobbler,
            screen: Screen::Library,
            settings_lastfm_username: lastfm_username,
            settings_lastfm_api_key: api_key,
            settings_lastfm_api_secret: api_secret,
            hover_show_lastfm_api_key: false,
            hover_show_lastfm_api_secret: false,
            library_search: String::new(),
            auth_token: None,
            auth_poll_attempts_left: 0,
            scrobble_timer: 0.0,
            current_duration_secs: 0,
            scrobbled: false,
            match_state: MatchState::Idle,
            link_cache: cache::load(),
        }
    }
}
