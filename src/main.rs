use iced::{Element, Task, Theme, Length};
use iced::widget::{button, column, container, scrollable, horizontal_rule, row, text};

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
    current: Option<usize>,
    playing: bool,
}

impl App {
    fn new() -> Self {
        Self {
            player: Player::new(),
            queue: vec![],
            current: None,
            playing: false,
        }
    }
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
                self.player.load(&self.queue[idx]);
                self.player.play();
                self.playing = true;
            }

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
                let next = self.current.map(|i| (i + 1) % self.queue.len()).unwrap_or(0);
                self.current = Some(next);                
                self.player.load(&self.queue[next]);
                self.player.play();
                self.playing = true;
            }

            Message::Previous => {
                if self.queue.is_empty() { return Task::none(); }
                let prev = self.current.map(|i| i.saturating_sub(1)).unwrap_or(0);
                self.current = Some(prev);
                self.player.load(&self.queue[prev]);
                self.player.play();
                self.playing = true;
            }
        }

        Task::none()
    }
}

impl App {
    fn view(&self) -> Element<Message> {
        let sidebar = self.playlist_view();
        let player_panel = self.player_view();

        let layout = row![
            sidebar,
            player_panel,
        ];

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn playlist_view(&self) -> Element<Message> {
        let header = row![
            text("Library").size(20),
            button("Open").on_press(Message::OpenFolder),
        ]
        .spacing(12)
        .padding(16);

        let track_list: Element<Message> = if self.queue.is_empty() {
            container(
                text("Open a folder to load music").size(14)
            )
            .padding(20)
            .into()
        } else {
            let items = column(
                self.queue.iter().enumerate().map(|(i, path)| {
                    self.track_row(i, path)
                })
            )
            .spacing(2)
            .padding([0, 8]);

            scrollable(items)
                .height(Length::Fill)
                .into()
        };

        let sidebar = column![
            header,
            horizontal_rule(1),
            track_list,
        ]
        .width(320)
        .height(Length::Fill);

        container(sidebar)
            .height(Length::Fill)
            .into()
    }

    fn track_row(&self, idx: usize, path: &std::path::Path) -> Element<Message> {
        let is_active = self.current == Some(idx);

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown");

        let ext = path 
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_uppercase();

        let playing_indicator = if is_active && self.playing {
            "> "
        } else {
            "  "
        };

        let label = column![
            text(format!("{}{}", playing_indicator, name))
                .size(14),
            text(ext).size(11),
        ]
        .spacing(2)
        .width(Length::Fill);

        let row_btn = button(label)
            .on_press(Message::SelectTrack(idx))
            .width(Length::Fill);

        container(row_btn)
            .width(Length::Fill)
            .padding([2, 4])
            .into()
    }

    fn player_view(&self) -> Element<Message> {
        let track_name = self.current
            .and_then(|i| self.queue.get(i))
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("No track selected");

        let track_count = if self.queue.is_empty() {
            String::new()
        } else {
            format!(
                "{} / {}",
                self.current.map(|i| i + 1).unwrap_or(0),
                self.queue.len()
            )
        };

        let play_pause = if self.playing {
            button("⏸").on_press(Message::Pause)
        } else {
            button("▶").on_press(Message::Play)
        };

        let controls = row![
            button("⏮").on_press(Message::Previous),
            play_pause,
            button("⏭").on_press(Message::Next),
        ]
        .spacing(16);

        let panel = column![
            text(track_name).size(22),
            text(track_count).size(13),
            controls,
        ]
        .spacing(16)
        .padding(40);

        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
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
