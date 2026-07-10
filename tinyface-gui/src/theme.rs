//! Color palette shared across the whole GUI, ported 1:1 from the v1 (egui) palette.

use iced::widget::overlay::menu;
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
/// when `active` — with a soft glow to sell the "lit" look — and responds to
/// hover when inactive so it reads as clickable before you ever press it.
pub fn toggle_button(
    active: bool,
    active_color: Color,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(
            status,
            button::Status::Hovered | button::Status::Pressed
        );

        let background = if active {
            active_color
        } else if hovered {
            SURFACE
        } else {
            BG_DEEP
        };
        let border_color = if active {
            active_color
        } else if hovered {
            FADER
        } else {
            BORDER
        };
        let shadow = if active {
            Shadow {
                color: Color {
                    a: 0.45,
                    ..active_color
                },
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 6.0,
            }
        } else {
            Shadow::default()
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color: if active { ON_ACTIVE } else { TEXT_SEC },
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow,
            snap: false,
        }
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

/// The small colored tick to the left of a section header label.
pub fn accent_bar(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(ACCENT)),
        border: Border {
            radius: 1.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// A recessed group container — used to visually cluster related top-bar
/// controls (e.g. the scene name/save/load trio) against the flatter
/// top-bar background.
pub fn chip(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_DEEP)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        text_color: Some(TEXT_PRIMARY),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn text_input(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    use iced::widget::text_input::Status;
    let border_color = match status {
        Status::Focused { .. } => FADER,
        _ => BORDER,
    };
    iced::widget::text_input::Style {
        background: Background::Color(SURFACE),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 4.0.into(),
        },
        icon: TEXT_SEC,
        placeholder: TEXT_SEC,
        value: TEXT_PRIMARY,
        selection: ACCENT_DIM,
    }
}

pub fn pick_list(
    _theme: &iced::Theme,
    status: iced::widget::pick_list::Status,
) -> iced::widget::pick_list::Style {
    use iced::widget::pick_list::Status;
    let border_color = match status {
        Status::Opened { .. } | Status::Hovered => FADER,
        Status::Active => BORDER,
    };
    iced::widget::pick_list::Style {
        text_color: TEXT_PRIMARY,
        placeholder_color: TEXT_SEC,
        handle_color: TEXT_SEC,
        background: Background::Color(SURFACE),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 4.0.into(),
        },
    }
}

/// A thin, minimal scrollbar rail — the scroller brightens on hover/drag
/// instead of the default Iced chrome, matching the rest of the app.
pub fn scrollable(
    _theme: &iced::Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    use iced::widget::scrollable::{AutoScroll, Rail, Scroller, Status};

    let (h_active, v_active) = match status {
        Status::Active { .. } => (false, false),
        Status::Hovered {
            is_horizontal_scrollbar_hovered,
            is_vertical_scrollbar_hovered,
            ..
        } => (
            is_horizontal_scrollbar_hovered,
            is_vertical_scrollbar_hovered,
        ),
        Status::Dragged {
            is_horizontal_scrollbar_dragged,
            is_vertical_scrollbar_dragged,
            ..
        } => (
            is_horizontal_scrollbar_dragged,
            is_vertical_scrollbar_dragged,
        ),
    };

    let rail = |active: bool| Rail {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border::default(),
        scroller: Scroller {
            background: Background::Color(if active { FADER } else { BORDER }),
            border: Border {
                radius: 3.0.into(),
                ..Border::default()
            },
        },
    };

    iced::widget::scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail(v_active),
        horizontal_rail: rail(h_active),
        gap: None,
        auto_scroll: AutoScroll {
            background: Background::Color(SURFACE),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: TEXT_PRIMARY,
        },
    }
}

pub fn menu(_theme: &iced::Theme) -> menu::Style {
    menu::Style {
        background: Background::Color(SURFACE),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: TEXT_PRIMARY,
        selected_text_color: TEXT_PRIMARY,
        selected_background: Background::Color(ACCENT_DIM),
        shadow: Shadow::default(),
    }
}
