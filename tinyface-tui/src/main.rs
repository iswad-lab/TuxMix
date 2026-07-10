//! `tinyface-tui` — Terminal-based RME interface controller.
//!
//! ```bash
//! cargo run -p tinyface-tui              # with hardware
//! cargo run -p tinyface-tui -- --mock    # simulation
//! ```

use std::io::{self, Stdout};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame, Terminal,
};

use tinyface_core::{BabyfacePro, ChannelId, MockBabyfacePro, RmeDevice};

// ── Device enum ─────────────────────────────────────────────────

enum DeviceHandle {
    Real(BabyfacePro),
    Mock(MockBabyfacePro),
}

macro_rules! delegate {
    ($self:expr, $method:ident($($arg:expr),*)) => {
        match $self {
            DeviceHandle::Real(d) => d.$method($($arg),*),
            DeviceHandle::Mock(d) => d.$method($($arg),*),
        }
    };
    ($self:expr, $method:ident) => {
        match $self {
            DeviceHandle::Real(d) => d.$method(),
            DeviceHandle::Mock(d) => d.$method(),
        }
    };
}

impl RmeDevice for DeviceHandle {
    fn model_name(&self) -> &str {
        delegate!(self, model_name)
    }
    fn output_pair_count(&self) -> usize {
        delegate!(self, output_pair_count)
    }
    fn open() -> Result<Self, tinyface_core::Error> {
        unreachable!()
    }
    fn inputs(&self) -> &[tinyface_core::InputChannel] {
        delegate!(self, inputs)
    }
    fn inputs_mut(&mut self) -> &mut [tinyface_core::InputChannel] {
        delegate!(self, inputs_mut)
    }
    fn playbacks(&self) -> &[tinyface_core::PlaybackChannel] {
        delegate!(self, playbacks)
    }
    fn playbacks_mut(&mut self) -> &mut [tinyface_core::PlaybackChannel] {
        delegate!(self, playbacks_mut)
    }
    fn settings(&self) -> &tinyface_core::DeviceSettings {
        delegate!(self, settings)
    }
    fn settings_mut(&mut self) -> &mut tinyface_core::DeviceSettings {
        delegate!(self, settings_mut)
    }
    fn set_volume(
        &mut self,
        ch: ChannelId,
        out: usize,
        v: f32,
    ) -> Result<(), tinyface_core::Error> {
        delegate!(self, set_volume(ch, out, v))
    }
    fn volume(&self, ch: ChannelId, out: usize) -> Result<f32, tinyface_core::Error> {
        delegate!(self, volume(ch, out))
    }
    fn set_pan(&mut self, ch: ChannelId, out: usize, p: i8) -> Result<(), tinyface_core::Error> {
        delegate!(self, set_pan(ch, out, p))
    }
    fn pan(&self, ch: ChannelId, out: usize) -> Result<i8, tinyface_core::Error> {
        delegate!(self, pan(ch, out))
    }
    fn capture_scene(&self) -> tinyface_core::Scene {
        delegate!(self, capture_scene)
    }
    fn apply_scene(&mut self, s: &tinyface_core::Scene) -> Result<(), tinyface_core::Error> {
        delegate!(self, apply_scene(s))
    }
    fn poll_events(&mut self) -> Result<(), tinyface_core::Error> {
        delegate!(self, poll_events)
    }
}

impl DeviceHandle {
    fn open_real() -> Option<Self> {
        BabyfacePro::open().ok().map(DeviceHandle::Real)
    }
    fn open_mock() -> Self {
        DeviceHandle::Mock(MockBabyfacePro::open().expect("mock opens"))
    }
    fn input_meter(&self, idx: usize) -> f32 {
        match self {
            DeviceHandle::Mock(d) => d.input_meter(idx),
            _ => 0.0,
        }
    }
    fn playback_meter(&self, idx: usize) -> f32 {
        match self {
            DeviceHandle::Mock(d) => d.playback_meter(idx),
            _ => 0.0,
        }
    }
    fn is_mock(&self) -> bool {
        matches!(self, DeviceHandle::Mock(_))
    }
}

// ── Constants ────────────────────────────────────────────────────

const OUT_LABELS: [&str; 6] = ["AN1/2", "PH3/4", "AS1/2", "A3/A4", "A5/A6", "A7/A8"];

// ── Main ─────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let mock = std::env::args().any(|a| a == "--mock");
    let mut device: DeviceHandle = if mock {
        DeviceHandle::open_mock()
    } else {
        DeviceHandle::open_real().unwrap_or_else(|| {
            eprintln!("No device found. Use --mock for simulation.");
            DeviceHandle::open_mock()
        })
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let res = run(&mut terminal, &mut device);
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    if let Err(e) = res {
        eprintln!("Error: {}", e);
    }
    Ok(())
}

fn run(term: &mut Terminal<CrosstermBackend<Stdout>>, dev: &mut DeviceHandle) -> io::Result<()> {
    let mut show_matrix = false;
    loop {
        let _ = dev.poll_events();
        term.draw(|f| ui(f, dev, show_matrix))?;
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Tab => show_matrix = !show_matrix,
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, dev: &DeviceHandle, show_matrix: bool) {
    let area = f.area();

    // Split into fixed top sections and flexible content + footer
    let top = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(area);

    let bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(Rect::new(
            area.left(),
            top[1].bottom(),
            area.width,
            area.bottom() - top[1].bottom(),
        ));

    let content = bottom[0];
    let footer_area = bottom[1];

    // Content sub-layout
    let (inputs_area, playbacks_area, matrix_area) = if show_matrix {
        (Rect::default(), Rect::default(), content)
    } else {
        let c = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Min(1)])
            .split(content);
        (c[0], c[1], Rect::default())
    };

    // Header
    let view_tag = if show_matrix {
        " [Matrix View]".to_string().yellow().bold().to_string()
    } else {
        String::new()
    };
    let mode = if dev.is_mock() {
        " [SIMULATED]".yellow().bold()
    } else {
        "".into()
    };
    let h = Paragraph::new(Line::from(vec![
        Span::styled("Tinyface", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!(" — {}  ", dev.model_name())),
        mode,
        Span::raw(format!("{}", view_tag)),
        Span::raw("  q:quit Tab:toggle"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(h, top[0]);

    // Summary
    let s = format!(
        "HW Inputs: {}  |  SW Playbacks: {}  |  Output pairs: {}  |  Clock: {}",
        dev.inputs().len(),
        dev.playbacks().len(),
        dev.output_pair_count(),
        dev.settings().clock_source,
    );
    f.render_widget(
        Paragraph::new(s).block(Block::default().borders(Borders::ALL).title("Overview")),
        top[1],
    );

    if show_matrix {
        render_matrix(f, "Matrix Mixer", matrix_area, dev);
    } else {
        render_strips(f, "Hardware Inputs", inputs_area, dev.inputs().len(), |i| {
            let ch = &dev.inputs()[i];
            let m = dev.input_meter(i);
            let mut label = format!("{} [{:?}]", ch.name, ch.channel_type);
            if ch.phantom {
                label.push_str(" 48V");
            }
            if ch.pad {
                label.push_str(" PAD");
            }
            (label, m)
        });

        render_strips(
            f,
            "Software Playbacks",
            playbacks_area,
            dev.playbacks().len(),
            |i| {
                let ch = &dev.playbacks()[i];
                let m = dev.playback_meter(i);
                (format!("{}", ch.name), m)
            },
        );
    }

    // Footer
    let footer = if show_matrix {
        "Tab: retour au mixer"
    } else {
        "q: quit  |  Tab: matrix view"
    };
    f.render_widget(
        Paragraph::new(footer).block(Block::default().borders(Borders::TOP)),
        footer_area,
    );
}

fn render_strips(
    f: &mut Frame,
    title: &str,
    area: Rect,
    count: usize,
    label_fn: impl Fn(usize) -> (String, f32),
) {
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = count.min(6) as u16;
    let rows = ((count as u16) + cols - 1) / cols;
    let row_h = (inner.height / rows.max(1)).max(3);

    for i in 0..count {
        let (label, meter) = label_fn(i);
        let col = i as u16 % cols;
        let row = i as u16 / cols;
        let x = inner.left() + col * (inner.width / cols);
        let y = inner.top() + row * row_h;
        let w = inner.width / cols;
        let ch_area = Rect::new(x, y, w, row_h - 1);

        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                label,
                Style::default().add_modifier(Modifier::BOLD),
            )])),
            ch_area,
        );

        if meter > 0.0 {
            let my = ch_area.bottom().saturating_sub(2);
            let ma = Rect::new(ch_area.left(), my, ch_area.width.min(20), 1);
            let c = if meter < 0.6 {
                Color::Green
            } else if meter < 0.85 {
                Color::Yellow
            } else {
                Color::Red
            };
            f.render_widget(
                Gauge::default()
                    .gauge_style(Style::default().fg(c))
                    .percent((meter * 100.0) as u16)
                    .label(format!("{:.0}%", meter * 100.0)),
                ma,
            );
        }
    }
}

fn render_matrix(f: &mut Frame, title: &str, area: Rect, dev: &DeviceHandle) {
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let ni = dev.inputs().len();
    let np = dev.playbacks().len();
    let total = ni + np;

    // Scrollable matrix is hard in TUI with ratatui without a proper table widget.
    // For now show a simplified overview: per-output volumes for selected channels.
    let mut lines = Vec::new();
    lines.push(format!("  {:>8}", ""));

    // Header
    let mut header = "  ".to_string();
    for col in 0..total.min(8) {
        let name = if col < ni {
            &dev.inputs()[col].name
        } else {
            &dev.playbacks()[col - ni].name
        };
        header.push_str(&format!(" {:>6}", &name[..name.len().min(6)]));
    }
    lines.push(header);

    // Rows
    for row in 0..6 {
        let mut line = format!("  {:>8}", OUT_LABELS[row]);
        for col in 0..total.min(8) {
            let v = if col < ni {
                dev.inputs()[col].volumes[row]
            } else {
                dev.playbacks()[col - ni].volumes[row]
            };
            line.push_str(&format!(" {:>5.0}%", v * 100.0));
        }
        lines.push(line);
    }

    let text = lines.join("\n");
    let p = Paragraph::new(text);
    f.render_widget(p, inner);
}
