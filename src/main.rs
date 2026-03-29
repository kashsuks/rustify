use iced::{Element, Task, Theme, Length, Color, Padding};
use iced::widget::{button, column, container, scrollable, horizontal_rule, row, text, Space};
use iced::widget::image as iced_image;
use lofty::prelude::*;
use lofty::probe::Probe;

mod player;
use player::Player;

pub fn main() -> iced::Result {
    iced::application("Rustify", App::update, App::view)
        .theme(|_| Theme::Nord)
        .run_with(|| (App::new(), Task::none()))
}

struct App {
    player: Player,
    queue: Vec<TrackMeta>,
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
                self.player.load(&self.queue[next].path);
                self.player.play();
                self.playing = true;
            }

            Message::Previous => {
                if self.queue.is_empty() { return Task::none(); }
                let prev = self.current.map(|i| i.saturating_sub(1)).unwrap_or(0);
                self.current = Some(prev);
                self.player.load(&self.queue[prev].path);
                self.player.play();
                self.playing = true;
            }
        }

        Task::none()
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
        let toolbar = row![
            text("Library").size(22),
            Space::with_width(Length::Fill),
            button(" Open Folder ").on_press(Message::OpenFolder),
        ]
        .padding([16, 24])
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

        let panel = column![
            art,
            info,
            controls,
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
