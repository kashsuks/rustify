mod state;
mod update;
mod view;

pub use state::{App, MatchState, Message, TrackMeta};

use iced::{Task, Theme};
use lucide_icons::LUCIDE_FONT_BYTES;

pub fn run() -> iced::Result {
    iced::application("Rustify", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::Nord)
        .font(LUCIDE_FONT_BYTES)
        .run_with(|| (App::new(), Task::none()))
}
