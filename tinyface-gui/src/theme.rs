//! Color palette shared across the whole GUI, ported 1:1 from the v1 (egui) palette.

use iced::widget::{button, container};
use iced::{Background, Border, Color, Shadow};

pub const BG_DEEP: Color = Color::from_rgb8(0x0d, 0x0d, 0x0d);
pub const SURFACE: Color = Color::from_rgb8(0x18, 0x18, 0x1a);
pub const BORDER: Color = Color::from_rgb8(0x2a, 0x2a, 0x30);
pub const TEXT_PRIMARY: Color = Color::from_rgb8(0xe8, 0xe8, 0xec);
pub const TEXT_SEC: Color = Color::from_rgb8(0x88, 0x88, 0x94);
pub const ACCENT: Color = Color::from_rgb8(0x4f, 0xc3, 0xf7);
pub const ACCENT_DIM: Color = Color::from_rgb8(0x2a, 0x6a, 0x88);
/// Sober neutral used for the fader rail/handle — kept separate from
/// `ACCENT` so the fader doesn't compete visually with the blue brand color.
pub const FADER: Color = Color::from_rgb8(0xa8, 0xac, 0xb4);
pub const MGREEN: Color = Color::from_rgb8(0x4c, 0xaf, 0x50);
pub const MYELLOW: Color = Color::from_rgb8(0xff, 0xeb, 0x3b);
pub const MRED: Color = Color::from_rgb8(0xf4, 0x43, 0x36);
pub const PHANTOM: Color = Color::from_rgb8(0xff, 0x45, 0x45);
pub const GCONN: Color = Color::from_rgb8(0x4c, 0xaf, 0x50);
pub const YSIM: Color = Color::from_rgb8(0xff, 0xc1, 0x07);
pub const MUTE_COLOR: Color = Color::from_rgb8(0xff, 0x6b, 0x6b);
pub const SOLO_COLOR: Color = Color::from_rgb8(0xff, 0xc1, 0x07);
pub const ON_ACTIVE: Color = Color::from_rgb8(0x1a, 0x08, 0x08);

pub fn panel(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 7.0.into(),
        },
        text_color: Some(TEXT_PRIMARY),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn root(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_DEEP)),
        text_color: Some(TEXT_PRIMARY),
        ..container::Style::default()
    }
}

pub fn top_bar(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        text_color: Some(TEXT_PRIMARY),
        ..container::Style::default()
    }
}

/// A small square toggle button (M / S / 48V / PAD) that lights up `active_color`
/// when `active`, and stays flat/bordered otherwise.
pub fn toggle_button(
    active: bool,
    active_color: Color,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, _status| button::Style {
        background: Some(Background::Color(if active { active_color } else { BG_DEEP })),
        text_color: if active { ON_ACTIVE } else { TEXT_SEC },
        border: Border {
            color: if active { active_color } else { BORDER },
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn plain_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => ACCENT_DIM,
        _ => SURFACE,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}
