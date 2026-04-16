use iced::{Color, Theme};
use iced::theme::Palette;

pub fn catppuccin_macchiato() -> Theme {
    Theme::custom(
        "Catppuccin Macchiato".to_string(),
        Palette {
            background: Color::from_rgb(0.145, 0.157, 0.208),
            text: Color::from_rgb(0.796, 0.839, 0.957),
            primary: Color::from_rgb(0.494, 0.655, 0.945),
            success: Color::from_rgb(0.651, 0.890, 0.631),
            danger: Color::from_rgb(0.957, 0.545, 0.659),
        },
    )
}

pub fn catppuccin_latte() -> Theme {
    Theme::custom(
        "Catppuccin Latte".to_string(),
        Palette {
            background: Color::from_rgb(0.937, 0.937, 0.957),
            text: Color::from_rgb(0.298, 0.306, 0.408),
            primary: Color::from_rgb(0.114, 0.447, 0.906),
            success: Color::from_rgb(0.251, 0.612, 0.357),
            danger: Color::from_rgb(0.816, 0.176, 0.259),
        },
    )
}

pub fn tokyo_night() -> Theme {
    Theme::custom(
        "Tokyo Night".to_string(),
        Palette {
            background: Color::from_rgb(0.063, 0.067, 0.098),           
            text: Color::from_rgb(0.694, 0.722, 0.996),
            primary: Color::from_rgb(0.494, 0.596, 0.918),             success: Color::from_rgb(0.588, 0.741, 0.475),             danger: Color::from_rgb(0.957, 0.529, 0.624),
        },
    )
}

pub fn ayu_dark() -> Theme {
    Theme::custom(
        "Ayu Dark".to_string(),
        Palette {
            background: Color::from_rgb(0.121, 0.141, 0.188),
            text: Color::from_rgb(0.796, 0.800, 0.776), 
            primary: Color::from_rgb(0.451, 0.816, 1.000),
            success: Color::from_rgb(0.729, 0.902, 0.494),
            danger: Color::from_rgb(0.949, 0.529, 0.475),
        },
    )
}
