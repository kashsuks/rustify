mod state;
mod update;
mod view;

pub use state::{App, AppTheme, MatchState, Message, TrackMeta};

use iced::{Font, Task};
use lucide_icons::LUCIDE_FONT_BYTES;

const MULTI_LANG_BYTES: &[u8] = include_bytes!("../assets/NotoSansMultilanguage-Regular.ttf");

pub fn run() -> iced::Result {
    iced::application("Rustify", App::update, App::view)
        .subscription(App::subscription)
        .theme(|app: &App| app.app_theme.to_iced_theme())
        .font(LUCIDE_FONT_BYTES)
        .font(MULTI_LANG_BYTES)
        .default_font(Font::with_name("NotoSans Multilanguage Regular"))
        .window_size(iced::Size::new(1280.0, 800.0))
        .run_with(|| {
            let app = App::new();

            let startup_task = match crate::features::settings::env::read_last_library_dir() {
                Some(path) if path.is_dir() => Task::done(Message::FolderPicked(Some(path))),
                _ => Task::none(),
            };

            (app, startup_task)
        })
}
