//! `tinyface-tui` — Terminal-based RME interface controller.
//!
//! Useful for debugging and scripting without a display server.
//! Uses ratatui for a simple channel overview.

use std::io::{self, Stdout};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use tinyface_core::{BabyfacePro, RmeDevice};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Try to open the device
    let mut device = match BabyfacePro::open() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Could not open Babyface Pro: {}", e);
            eprintln!("Make sure the device is plugged in and recognized by ALSA.");
            return Ok(());
        }
    };

    println!("{} detected on ALSA", device.model_name());

    // ── Terminal setup ─────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── Main loop ──────────────────────────────────────────────
    let res = run(&mut terminal, &mut device);

    // ── Cleanup ────────────────────────────────────────────────
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    device: &mut BabyfacePro,
) -> io::Result<()> {
    loop {
        // Poll hardware events
        let _ = device.poll_events();

        terminal.draw(|f| ui(f, device))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, device: &BabyfacePro) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    // ── Header ─────────────────────────────────────────────────
    let header = Paragraph::new(Line::from(vec![
        Span::styled("Tinyface", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!(" — {}  ", device.model_name())),
        Span::raw("Press "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" to quit"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // ── Channel overview ───────────────────────────────────────
    let channel_text = format!(
        "HW Inputs: {}  |  SW Playbacks: {}  |  Output pairs: {}",
        device.inputs().len(),
        device.playbacks().len(),
        6
    );

    let body =
        Paragraph::new(channel_text).block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(body, chunks[1]);

    // ── Footer ─────────────────────────────────────────────────
    let footer = Paragraph::new("Device ready. Full TUI coming soon.")
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[2]);
}
