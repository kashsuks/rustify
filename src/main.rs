use iced::{Element, Task, Theme};
use iced::widget::{button, column, container, row, text};

mod player;
use player::Player;

pub fn main() -> iced::Result {
    iced::application("Rustify", App::update, App::view)
        .theme(|_| Theme::TokyoNight)
        .run_with(|| (App::new(), Task::none()))
}

struct App {
    player: Player,
    queue: Vec<std::path::PathBuf>,
    current: usize,
    playing: bool,
}

impl App {
    fn new() -> Self {
        Self {
            player: Player::new(),
            queue: vec![],
            current: 0,
            playing: false,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    OpenFolder,
    FolderPicked(Option<std::path::PathBuf>),
    Play,
    Pause,
    Next,
    Previous,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenFolder => {
                return Task::perform(pick_folder(), Message::FolderPicked);
            }

            Message::FolderPicked(Some(path)) => {
                self.queue = scan_audio(&path);
                self.current = 0;
                if !self.queue.is_empty() {
                    self.player.load(&self.queue[self.current]);
                    self.player.play();
                    self.playing = true;
                }
            }

            Message::FolderPicked(None) => {}

            Message::Play => {
                self.player.play();
                self.playing = true;
            }

            Message::Pause => {
                self.player.pause();
                self.playing = false;
            }

            Message::Next => {
                if self.queue.is_empty() { return Task::none(); }
                self.current = (self.current + 1) % self.queue.len();
                self.player.load(&self.queue[self.current]);
                self.player.play();
                self.playing = true;
            }

            Message::Previous => {
                if self.queue.is_empty() { return Task::none(); }
                self.current = self.current.saturating_sub(1);
                self.player.load(&self.queue[self.current]);
                self.player.play();
                self.playing = true;
            }
        }

        Task::none()
    }
}

impl App {
    fn view(&self) -> Element<Message> {
        let track_name = self.queue.get(self.current)
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("No track loaded");

        let track_info = text(track_name).size(18);

        let play_pause = if self.playing {
            button("⏸  Pause").on_press(Message::Pause)
        } else {
            button("▶  Play").on_press(Message::Play)
        };

        let controls = row![
            button("⏮  Prev").on_press(Message::Previous),
            play_pause,
            button("⏭  Next").on_press(Message::Next),
        ]
        .spacing(12);

        let open_btn = button("Open Folder").on_press(Message::OpenFolder);

        let content = column![
            open_btn,
            track_info,
            controls,
        ]
        .spacing(20)
        .padding(40);

        container(content)
            .center_x(iced::Fill)
            .center_y(iced::Fill)
            .into()
    }
}

async fn pick_folder() -> Option<std::path::PathBuf> {
    rfd::AsyncFileDialog::new()
        .pick_folder()
        .await
        .map(|f| f.path().to_path_buf())
}

fn scan_audio(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
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
        .map(|e| e.path().to_path_buf())
        .collect()
}
