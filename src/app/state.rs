/// This file is responsible for the differnt states
/// Examples are metadata of songs that help derive its data
/// Different states for controls like volume, play, pause, etc

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
    Scanning {
        total: usize,
        done: usize,
    },
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
    VolumeChanged(f32),
    Next,
    Previous,

    LastfmTick,
    LastfmUpdated(Option<LastfmTrack>),
    LastfmArtworkFetched(Option<Vec<u8>>),

    StartAuth,
    AuthTokenReceived(Option<String>),
    AuthPollTick,
    AuthCompleted(Option<String>),
    ScrobbleTick,

    RecommendationReady(Vec<crate::features::scrobbling::lastfm::SimilarTrack>),

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

    DiscordArtworkReady(Option<String>),
    ThemeChanged(AppTheme),
}

/// Tracks all metadata of a given song.
/// 
/// # Fields
/// 
/// - `path` (`PathBuf`) - Absolute path of the song.
/// - `title` (`String`) - Song title.
/// - `artist` (`String`) - Artist of the song.
/// - `album` (`String`) - Album that the song is from.
/// - `duration` (`String`) - Duration of the song in hours:minutes.
/// - `duration_secs` (`u64`) - Total duration in seconds (minutes x 60 + seconds).
/// - `artwork` (`Option<Vec<u8>>`) - Song cover art.
/// - `lastfm_title` (`Option<String>`) - Song name on Last.fm.
/// - `lastfm_artist` (`Option<String>`) - Artist name on Last.fm.
/// - `linked` (`bool`) - Whether it is an unrecognized song that is linked to last.fm.
/// 
/// # Examples
/// 
/// ```
/// use crate::...;
/// 
/// let s = TrackMeta {
///     path: value,
///     title: value,
///     artist: value,
///     album: value,
///     duration: value,
///     duration_secs: value,
///     artwork: value,
///     lastfm_title: value,
///     lastfm_artist: value,
///     linked: value,
/// };
/// ```
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

/// This enum provides the different theme options for usage.
/// 
/// # Variants
/// 
/// - `Nord` - Nord theme colour scheme.
/// - `CatppuccinMacchiato` - Catppuccin Macchiato colour scheme.
/// - `CatppuccinLatte` - Catppuccin Latte colour scheme.
/// - `TokyoNight` - Tokyo Night colour scheme.
/// - `AyuDark` - Ayu Dark colour scheme.
/// 
/// # Examples
/// 
/// ```
/// use crate::...;
/// 
/// let apptheme = AppTheme::Nord;
/// match apptheme {
///     AppTheme::Nord => handle_unit,
///     AppTheme::CatppuccinMacchiato => handle_unit,
///     AppTheme::CatppuccinLatte => handle_unit,
///     AppTheme::TokyoNight => handle_unit,
///     AppTheme::AyuDark => handle_unit,
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTheme {
    Nord,
    CatppuccinMacchiato,
    CatppuccinLatte,
    TokyoNight,
    AyuDark,
}

pub struct App {
    pub(crate) player: Player,
    pub(crate) queue: Vec<TrackMeta>,
    pub(crate) current: Option<usize>,
    pub(crate) playing: bool,
    pub(crate) discord: DiscordRpc,
    pub(crate) lastfm_track: Option<LastfmTrack>,
    pub(crate) lastfm_artwork: Option<Vec<u8>>,
    pub(crate) next_up: Option<usize>,
    pub(crate) lastfm_api_key: String,
    pub(crate) lastfm_username: String,
    pub(crate) scrobbler: Scrobbler,
    pub(crate) auth_token: Option<String>,
    pub(crate) auth_poll_attempts_left: u8,
    pub(crate) lastfm_auth_status: Option<String>,
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
    pub(crate) discord_artwork_url: Option<String>,
    pub(crate) app_theme: AppTheme,
}

impl App {
    pub fn new() -> Self {
        let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
        dotenvy::from_path(env_path).ok();

        // loaded from .env for now, change to hardcode when building and revert back
        // DO NOT: commit when hardoded values
        let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_default();
        let lastfm_api_key = std::env::var("LASTFM_API_KEY").unwrap_or_default();
        let lastfm_username = std::env::var("LASTFM_USERNAME").unwrap_or_default();
        let api_key = std::env::var("LASTFM_API_KEY").unwrap_or_default();
        let api_secret = std::env::var("LASTFM_API_SECRET").unwrap_or_default();
        let session_key = std::env::var("LASTFM_SESSION_KEY").ok();

        let scrobbler = match session_key {
            Some(ref sk) => {
                Scrobbler::new_with_session(api_key.clone(), api_secret.clone(), sk.clone())
            }
            None => Scrobbler::new(api_key.clone(), api_secret.clone()),
        };

        Self {
            player: Player::new(),
            queue: vec![],
            current: None,
            playing: false,
            discord: DiscordRpc::connect(&client_id),
            lastfm_track: None,
            lastfm_artwork: None,
            next_up: None,
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
            lastfm_auth_status: None,
            scrobble_timer: 0.0,
            current_duration_secs: 0,
            scrobbled: false,
            match_state: MatchState::Idle,
            link_cache: cache::load(),
            discord_artwork_url: None,
            app_theme: crate::features::settings::env::read_theme()
                .and_then(|s| AppTheme::from_label(&s))
                .unwrap_or(AppTheme::Nord),
        }
    }
}

impl AppTheme {
    pub fn label(&self) -> &'static str {
        match self {
            AppTheme::Nord => "Nord",
            AppTheme::CatppuccinMacchiato => "Catppuccin Macchiato",
            AppTheme::TokyoNight => "Tokyo Night",
            AppTheme::CatppuccinLatte => "Catppuccin Latte",
            AppTheme::AyuDark => "Ayu Dark",
        }
    }

    pub fn all() -> &'static [AppTheme] {
        &[
            AppTheme::Nord,
            AppTheme::CatppuccinMacchiato,
            AppTheme::CatppuccinLatte,
            AppTheme::TokyoNight,
            AppTheme::AyuDark,
        ]
    }

    pub fn to_iced_theme(&self) -> iced::Theme {
        use crate::features::settings::theme;
        match self {
            AppTheme::Nord => iced::Theme::Nord,
            AppTheme::CatppuccinMacchiato => theme::catppuccin_macchiato(),
            AppTheme::CatppuccinLatte => theme::catppuccin_latte(),
            AppTheme::TokyoNight => theme::tokyo_night(),
            AppTheme::AyuDark => theme::ayu_dark(),
        }
    }

    pub fn from_label(s: &str) -> Option<AppTheme> {
        AppTheme::all().iter().find(|t| t.label() == s).copied()
    }
}

// needed to display the options properly
impl std::fmt::Display for AppTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}
