mod state;
mod update;
mod view;

pub use state::{App, AppTheme, MatchState, Message, TrackMeta};

use iced::{Task};
use lucide_icons::LUCIDE_FONT_BYTES;

pub fn run() -> iced::Result {
    iced::application("Rustify", App::update, App::view)
        .subscription(App::subscription)
        .theme(|app: &App| app.app_theme.to_iced_theme())
        .font(LUCIDE_FONT_BYTES)
        .run_with(|| (App::new(), Task::none()))
}
