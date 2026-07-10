//! `tinyface-gui` — Graphical RME interface controller.
//!
//! ```bash
//! cargo run -p tinyface-gui              # with real hardware
//! cargo run -p tinyface-gui -- --mock    # simulation
//! ```

mod app;
mod matrix;
mod scenes;
mod theme;
mod widgets;

use iced::window;
use iced::Size;

fn main() -> iced::Result {
    env_logger::init();
    let mock = std::env::args().any(|a| a == "--mock");

    iced::application(move || app::new(mock), app::update, app::view)
        .title(app::title)
        .subscription(app::subscription)
        .theme(|_state: &app::TinyFace| iced::Theme::Dark)
        .window(window::Settings {
            size: Size::new(1280.0, 800.0),
            min_size: Some(Size::new(960.0, 600.0)),
            ..window::Settings::default()
        })
        .run()
}
