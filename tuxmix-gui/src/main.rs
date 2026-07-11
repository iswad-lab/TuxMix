//! `tuxmix-gui` — Graphical RME interface controller.
//!
//! ```bash
//! cargo run -p tuxmix-gui              # with real hardware
//! cargo run -p tuxmix-gui -- --mock    # simulation
//! ```

mod app;
mod matrix;
mod scenes;
mod theme;
mod widgets;

use iced::window;
use iced::{Font, Size};

/// Inter (SIL OFL 1.1, see assets/fonts/Inter-LICENSE) replaces whatever
/// sans-serif the host system happens to default to — a plain-looking UI is
/// the single biggest thing separating this from a "finished" app, for very
/// little effort.
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");

fn main() -> iced::Result {
    env_logger::init();
    let mock = std::env::args().any(|a| a == "--mock");

    iced::application(move || app::new(mock), app::update, app::view)
        .title(app::title)
        .subscription(app::subscription)
        .theme(|_state: &app::TuxMix| iced::Theme::Dark)
        .font(INTER_REGULAR)
        .default_font(Font::with_name("Inter"))
        .window(window::Settings {
            size: Size::new(1280.0, 800.0),
            min_size: Some(Size::new(960.0, 600.0)),
            ..window::Settings::default()
        })
        .run()
}
