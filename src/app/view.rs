use crate::app::{App, MatchState, Message};
use crate::features::scrobbling::matcher::SearchResult;
use iced::widget::image as iced_image;
use iced::widget::{
    button, column, container, horizontal_rule, pick_list, row, scrollable, slider, text,
    text_input, Space,
};
use iced::{Color, Element, Length, Padding};
use lucide_icons::Icon;

impl App {
    pub fn view(&self) -> Element<'_, Message> {
        match self.screen {
            crate::app::state::Screen::Library => {
                let is_reviewing = matches!(self.match_state, MatchState::Reviewing { .. });
                let main_ui = self.main_view(is_reviewing);

                if let MatchState::Reviewing {
                    pending,
                    search_query,
                    search_results,
                    search_loading,
                    preview_playing,
                } = &self.match_state
                {
                    let modal = self.review_modal(
                        pending,
                        search_query,
                        search_results,
                        *search_loading,
                        *preview_playing,
                    );

                    return iced::widget::stack([main_ui, modal])
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into();
                }

                main_ui
            }
            crate::app::state::Screen::Settings => self.settings_view(),
        }
    }

    fn main_view(&self, dimmed: bool) -> Element<'_, Message> {
        let track_list = self.track_list_view();
        let now_playing = self.now_playing_view();
        let layout = row![track_list, now_playing].height(Length::Fill);

        let banner: Element<Message> = match &self.match_state {
            MatchState::Scanning { total, done } => container(
                text(format!("Matching tracks with Last.fm… {}/{}", done, total)).size(13),
            )
            .padding([6, 24])
            .width(Length::Fill)
            .into(),
            _ => Space::with_height(0).into(),
        };

        container(column![banner, layout])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| {
                if dimmed {
                    container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(
                            0.0, 0.0, 0.0, 0.6,
                        ))),
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
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into(),
        };

        let preview_btn = button(
            container(if preview_playing {
                iced::widget::Text::from(Icon::CirclePause).size(28)
            } else {
                iced::widget::Text::from(Icon::CirclePlay).size(28)
            })
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        )
        .on_press(Message::PreviewToggle)
        .width(140)
        .height(140)
        .style(|_, _| button::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.0, 0.0, 0.0, 0.5,
            ))),
            text_color: Color::WHITE,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
        });

        let art_with_preview = container(iced::widget::stack([
            art,
            container(preview_btn).width(140).height(140).into(),
        ]))
        .width(140)
        .height(140);

        let local_info = column![
            text(&track.title).size(15),
            text(&track.artist).size(13),
            text(&track.duration).size(12),
            text(
                track
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
            )
            .size(11),
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
            container(text("Searching…").size(13)).padding(12).into()
        } else if search_results.is_empty() {
            container(text("No results found").size(13))
                .padding(12)
                .into()
        } else {
            let items = column(search_results.iter().map(|result| {
                let link_result = result.clone();
                let duration_text = if result.duration_secs > 0 {
                    format!(
                        "{}:{:02}",
                        result.duration_secs / 60,
                        result.duration_secs % 60
                    )
                } else {
                    "--:--".to_string()
                };

                let row_content = row![
                    column![
                        text(&result.title).size(14),
                        text(format!("{} · {}", result.artist, duration_text)).size(12),
                    ]
                    .width(Length::Fill)
                    .spacing(2),
                    button(" Link ")
                        .on_press(Message::LinkTrack(queue_idx, link_result))
                        .style(|_, _| button::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(
                                0.2, 0.6, 0.3,
                            ))),
                            text_color: Color::WHITE,
                            border: iced::Border {
                                radius: 10.0.into(),
                                ..Default::default()
                            },
                            shadow: iced::Shadow::default(),
                        }),
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center)
                .padding([8, 4]);

                button(row_content)
                    .on_press(Message::LinkTrack(queue_idx, result.clone()))
                    .width(Length::Fill)
                    .style(|_, status| {
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
            }))
            .spacing(2);

            scrollable(items).height(280).into()
        };

        let right_panel = column![search_box, results_list].width(320).spacing(8);

        let header = row![
            text(format!("Unrecognized track — {} remaining", remaining)).size(16),
            Space::with_width(Length::Fill),
            button(" Skip ")
                .on_press(Message::SkipTrack(queue_idx))
                .style(|_, _| button::Style {
                    background: Some(iced::Background::Color(Color::from_rgba(
                        1.0, 1.0, 1.0, 0.1,
                    ))),
                    text_color: Color::WHITE,
                    border: iced::Border {
                        radius: 10.0.into(),
                        ..Default::default()
                    },
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
            column![header, horizontal_rule(1), body]
                .spacing(16)
                .padding(32),
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

        let overlay = container(
            container(modal_box)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.0, 0.0, 0.0, 0.75,
            ))),
            ..Default::default()
        });

        iced::widget::opaque(overlay).into()
    }

    fn track_list_view(&self) -> Element<'_, Message> {
        let open_folder_btn = button(
            container(iced::widget::Text::from(Icon::FolderOpen).size(18))
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(44)
        .height(44)
        .style(|theme, status| {
            let mut style = button::primary(theme, status);
            style.border.radius = 12.0.into();
            style
        })
        .on_press(Message::OpenFolder);

        let settings_btn = button(
            container(iced::widget::Text::from(Icon::Settings).size(18))
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(44)
        .height(44)
        .style(|theme, status| {
            let mut style = button::primary(theme, status);
            style.border.radius = 12.0.into();
            style
        })
        .on_press(Message::OpenSettings);

        // The search bar fills available space but is capped so it never
        // crowds the flanking buttons on wide windows.
        let search_bar = container(
            row![
                iced::widget::Text::from(Icon::Search).size(16),
                text_input("What do you want to play?", &self.library_search)
                    .on_input(Message::LibrarySearchChanged)
                    .padding([10, 0])
                    .size(16)
                    .style(|theme, status| {
                        let mut style = text_input::default(theme, status);
                        style.background = iced::Background::Color(Color::TRANSPARENT);
                        style.border.width = 0.0;
                        style
                    }),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        )
        .padding([0, 16])
        .height(44)
        .width(Length::Fill)
        .max_width(560)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.16, 0.17, 0.24))),
            border: iced::Border {
                radius: 14.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        // Equal spacers on both sides of the search bar keep it centred and
        // give natural breathing room away from the icon buttons.
        let toolbar = row![
            open_folder_btn,
            text("Library").size(22),
            Space::with_width(Length::Fill),
            search_bar,
            Space::with_width(Length::Fill),
            settings_btn,
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
            let query = self.library_search.to_lowercase();

            let rows = column(
                self.queue
                    .iter()
                    .enumerate()
                    .filter(|(_, track)| {
                        query.is_empty()
                            || track.title.to_lowercase().contains(&query)
                            || track.artist.to_lowercase().contains(&query)
                            || track.album.to_lowercase().contains(&query)
                    })
                    .map(|(idx, track)| self.track_row(idx, track)),
            )
            .spacing(0);

            scrollable(rows)
                .height(Length::Fill)
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new()
                        .width(12)
                        .scroller_width(6)
                        .margin(6),
                ))
                .style(move |theme, status| app_scrollbar_style(theme, status, self.app_theme))
                .into()
        };

        column![
            toolbar,
            horizontal_rule(1),
            headers,
            horizontal_rule(1),
            body
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

    fn track_row<'a>(
        &'a self,
        idx: usize,
        track: &'a crate::app::TrackMeta,
    ) -> Element<'a, Message> {
        let is_active = self.current == Some(idx);
        let is_reviewing = matches!(self.match_state, MatchState::Reviewing { .. });

        let num_or_indicator: Element<Message> = if is_active && self.playing {
            text(">").size(13).width(40).into()
        } else {
            text(format!("{}", idx + 1)).size(13).width(40).into()
        };

        let title_display = if !track.linked {
            format!("⊘ {}", track.title)
        } else if track.lastfm_title.is_none() && self.scrobbler.is_authenticated() {
            format!("? {}", track.title)
        } else {
            track.title.clone()
        };

        let row_content = row![
            num_or_indicator,
            column![text(title_display).size(14)].width(Length::Fill),
            text(&track.artist).size(13).width(160),
            text(&track.album).size(13).width(180),
            text(&track.duration).size(13).width(70),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .padding([10, 24]);

        let button = button(row_content).width(Length::Fill);
        let button = if is_reviewing {
            button
        } else {
            button.on_press(Message::SelectTrack(idx))
        };

        let app_theme = self.app_theme;

        button
            .style(move |_, status| {
                let is_latte = matches!(app_theme, crate::app::AppTheme::CatppuccinLatte);
                let bg = match (
                    is_latte,
                    is_active,
                    matches!(status, button::Status::Hovered),
                ) {
                    (true, _, true) if !is_reviewing => Color::from_rgba(0.20, 0.22, 0.30, 0.08),
                    (true, true, _) => Color::from_rgba(0.20, 0.22, 0.30, 0.06),
                    (false, _, true) if !is_reviewing => Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                    (false, true, _) => Color::from_rgba(1.0, 1.0, 1.0, 0.03),
                    _ => Color::TRANSPARENT,
                };

                button::Style {
                    background: Some(iced::Background::Color(bg)),
                    text_color: if is_latte {
                        if is_active {
                            Color::from_rgb(0.12, 0.14, 0.20)
                        } else {
                            Color::from_rgb(0.30, 0.31, 0.41)
                        }
                    } else if is_active {
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

    fn settings_view(&self) -> Element<'_, Message> {
        use crate::app::state::AppTheme;

        let header = row![
            button(iced::widget::Text::from(Icon::ArrowLeft).size(22))
                .width(44)
                .height(44)
                .style(|theme, status| {
                    let mut style = button::secondary(theme, status);
                    style.border.radius = 14.0.into();
                    style
                })
                .on_press(Message::CloseSettings),
            text("Settings").size(28),
            Space::with_width(Length::Fill),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(12);

        let theme_section = column![
            text("Appearance").size(22),
            text("Theme").size(18),
            pick_list(AppTheme::all(), Some(self.app_theme), Message::ThemeChanged,)
                .style(|theme, status| {
                    let mut style = pick_list::default(theme, status);
                    style.border.radius = 10.0.into();
                    style
                })
                .padding([10, 14]),
        ]
        .spacing(12);

        let lastfm_connected = self.scrobbler.is_authenticated();

        let lastfm_auth_message: Element<Message> = match &self.lastfm_auth_status {
            Some(message) => text(message).size(12).into(),
            None => Space::with_height(0).into(),
        };

        let lastfm_section = column![
            text("Connections").size(22),
            row![
                text("Last.fm").size(18),
                Space::with_width(Length::Fill),
                button(" Connect Last.fm ")
                    .padding([10, 18])
                    .style(|theme, status| {
                        let mut style = button::primary(theme, status);
                        style.border.radius = 16.0.into();
                        style
                    })
                    .on_press(Message::StartAuth),
                container(
                    text(if lastfm_connected {
                        "✓ Connected"
                    } else {
                        "Not Connected"
                    })
                    .size(13),
                )
                .padding([8, 12])
                .style(move |_| container::Style {
                    text_color: Some(Color::WHITE),
                    background: Some(iced::Background::Color(if lastfm_connected {
                        Color::from_rgb(0.18, 0.60, 0.30)
                    } else {
                        Color::from_rgb(0.55, 0.18, 0.18)
                    })),
                    border: iced::Border {
                        radius: 10.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
            lastfm_auth_message,
        ]
        .spacing(12);

        container(
            column![
                header,
                horizontal_rule(1),
                theme_section,
                horizontal_rule(1),
                lastfm_section
            ]
            .spacing(24)
            .padding(32),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn now_playing_view(&self) -> Element<'_, Message> {
        let (title, artist, album, artwork): (&str, &str, &str, Option<&Vec<u8>>) =
            if let Some(ref t) = self.lastfm_track {
                (
                    t.name.as_str(),
                    t.artist.text.as_str(),
                    t.album.text.as_str(),
                    self.lastfm_artwork.as_ref(),
                )
            } else {
                let current_track = self.current.and_then(|idx| self.queue.get(idx));
                (
                    current_track
                        .and_then(|t| t.lastfm_title.as_deref().or(Some(t.title.as_str())))
                        .unwrap_or("No track selected"),
                    current_track
                        .and_then(|t| t.lastfm_artist.as_deref().or(Some(t.artist.as_str())))
                        .unwrap_or(""),
                    current_track.map(|t| t.album.as_str()).unwrap_or(""),
                    current_track.and_then(|t| t.artwork.as_ref()),
                )
            };

        let art: Element<Message> = match artwork {
            Some(bytes) => {
                let handle = iced_image::Handle::from_bytes(bytes.clone());
                iced_image::Image::new(handle).width(260).height(260).into()
            }
            None => container(text("♪").size(64))
                .width(260)
                .height(260)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.15, 0.2))),
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
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
        .padding(Padding {
            top: 16.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        });

        let previous_btn = button(iced::widget::Text::from(Icon::SkipBack).size(22))
            .width(48)
            .height(48)
            .style(|theme, status| {
                let mut style = button::primary(theme, status);
                style.border.radius = 14.0.into();
                style
            })
            .on_press(Message::Previous);

        let play_pause = if self.playing {
            button(iced::widget::Text::from(Icon::CirclePause).size(22))
                .width(48)
                .height(48)
                .style(|theme, status| {
                    let mut style = button::primary(theme, status);
                    style.border.radius = 14.0.into();
                    style
                })
                .on_press(Message::Pause)
        } else {
            button(iced::widget::Text::from(Icon::CirclePlay).size(22))
                .width(48)
                .height(48)
                .style(|theme, status| {
                    let mut style = button::primary(theme, status);
                    style.border.radius = 14.0.into();
                    style
                })
                .on_press(Message::Play)
        };

        let next_btn = button(iced::widget::Text::from(Icon::SkipForward).size(22))
            .width(48)
            .height(48)
            .style(|theme, status| {
                let mut style = button::primary(theme, status);
                style.border.radius = 14.0.into();
                style
            })
            .on_press(Message::Next);

        let controls = row![
            previous_btn.padding([10, 14]),
            play_pause.padding([10, 14]),
            next_btn.padding([10, 14]),
        ]
        .spacing(12)
        .padding(Padding {
            top: 20.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        });

        let volume_icon = if self.player.volume() <= 0.0 {
            Icon::VolumeX // if volume is set to 0/muted
        } else {
            Icon::Volume2 // if volume != 0 (not muted)
        };

        let volume = row![
            iced::widget::Text::from(volume_icon).size(18),
            slider(0.0..=1.0, self.player.volume(), Message::VolumeChanged)
                .step(0.1) // change step if needed for more speed
                .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .padding(Padding {
            top: 8.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        });

        let lastfm_status: Element<Message> = if let Some(track) = &self.lastfm_track {
            column![
                text("▸ Last.fm").size(11),
                text(&track.name).size(13),
                text(&track.artist.text).size(11),
            ]
            .spacing(2)
            .padding(Padding {
                top: 12.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            })
            .into()
        } else {
            Space::with_height(0).into()
        };

        let panel = column![art, info, controls, volume, lastfm_status]
            .padding(24)
            .width(300)
            .height(Length::Fill)
            .align_x(iced::Alignment::Center);

        container(panel)
            .height(Length::Fill)
            .width(300)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.1))),
                ..Default::default()
            })
            .into()
    }
}

fn app_scrollbar_style(
    theme: &iced::Theme,
    status: scrollable::Status,
    app_theme: crate::app::AppTheme,
) -> scrollable::Style {
    let mut style = scrollable::default(theme, status);

    let (base_thumb, hover_thumb, drag_thumb, rail) = match app_theme {
        crate::app::AppTheme::Nord => (
            Color::from_rgba(0.53, 0.75, 0.82, 0.35),
            Color::from_rgba(0.53, 0.75, 0.82, 0.65),
            Color::from_rgba(0.53, 0.75, 0.82, 0.95),
            Color::from_rgba(0.93, 0.96, 0.99, 0.04),
        ),
        crate::app::AppTheme::CatppuccinMacchiato => (
            Color::from_rgba(0.54, 0.63, 0.94, 0.35),
            Color::from_rgba(0.54, 0.63, 0.94, 0.65),
            Color::from_rgba(0.54, 0.63, 0.94, 0.95),
            Color::from_rgba(0.94, 0.95, 1.0, 0.04),
        ),
        crate::app::AppTheme::CatppuccinLatte => (
            Color::from_rgba(0.45, 0.53, 0.75, 0.45),
            Color::from_rgba(0.45, 0.53, 0.75, 0.75),
            Color::from_rgba(0.45, 0.53, 0.75, 0.95),
            Color::from_rgba(0.20, 0.22, 0.30, 0.08),
        ),
        crate::app::AppTheme::TokyoNight => (
            Color::from_rgba(0.48, 0.60, 0.98, 0.35),
            Color::from_rgba(0.48, 0.60, 0.98, 0.65),
            Color::from_rgba(0.48, 0.60, 0.98, 0.95),
            Color::from_rgba(0.75, 0.80, 1.0, 0.04),
        ),
        crate::app::AppTheme::AyuDark => (
            Color::from_rgba(0.45, 0.82, 1.0, 0.35),
            Color::from_rgba(0.45, 0.82, 1.0, 0.65),
            Color::from_rgba(0.45, 0.82, 1.0, 0.95),
            Color::from_rgba(0.75, 0.90, 1.0, 0.04),
        ),
    };

    let thumb = match status {
        scrollable::Status::Hovered { .. } => hover_thumb,
        scrollable::Status::Dragged { .. } => drag_thumb,
        _ => base_thumb,
    };

    style.vertical_rail.background = Some(iced::Background::Color(rail));
    style.vertical_rail.border.radius = 999.0.into();
    style.vertical_rail.scroller.color = thumb;
    style.vertical_rail.scroller.border.radius = 999.0.into();

    style
}
