//! `tuxmix-tui` — Terminal-based RME interface controller.
//!
//! ```bash
//! cargo run -p tuxmix-tui              # with hardware
//! cargo run -p tuxmix-tui -- --mock    # simulation
//! ```

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
use std::io::{self, Stdout};
use tuxmix_core::{BabyfacePro, ChannelId, MockBabyfacePro, RmeDevice};

enum DeviceHandle {
    Real(BabyfacePro),
    Mock(MockBabyfacePro),
}
macro_rules! delegate {
    ($self:expr, $method:ident($($arg:expr),*)) => { match $self { DeviceHandle::Real(d) => d.$method($($arg),*), DeviceHandle::Mock(d) => d.$method($($arg),*), } };
    ($self:expr, $method:ident) => { match $self { DeviceHandle::Real(d) => d.$method(), DeviceHandle::Mock(d) => d.$method(), } };
}
impl RmeDevice for DeviceHandle {
    fn model_name(&self) -> &str {
        delegate!(self, model_name)
    }
    fn output_pair_count(&self) -> usize {
        delegate!(self, output_pair_count)
    }
    fn open() -> Result<Self, tuxmix_core::Error> {
        unreachable!()
    }
    fn inputs(&self) -> &[tuxmix_core::InputChannel] {
        delegate!(self, inputs)
    }
    fn inputs_mut(&mut self) -> &mut [tuxmix_core::InputChannel] {
        delegate!(self, inputs_mut)
    }
    fn playbacks(&self) -> &[tuxmix_core::PlaybackChannel] {
        delegate!(self, playbacks)
    }
    fn playbacks_mut(&mut self) -> &mut [tuxmix_core::PlaybackChannel] {
        delegate!(self, playbacks_mut)
    }
    fn outputs(&self) -> &[tuxmix_core::OutputChannel] {
        delegate!(self, outputs)
    }
    fn outputs_mut(&mut self) -> &mut [tuxmix_core::OutputChannel] {
        delegate!(self, outputs_mut)
    }
    fn settings(&self) -> &tuxmix_core::DeviceSettings {
        delegate!(self, settings)
    }
    fn settings_mut(&mut self) -> &mut tuxmix_core::DeviceSettings {
        delegate!(self, settings_mut)
    }
    fn set_volume(
        &mut self,
        ch: ChannelId,
        out: usize,
        v: f32,
    ) -> Result<(), tuxmix_core::Error> {
        delegate!(self, set_volume(ch, out, v))
    }
    fn volume(&self, ch: ChannelId, out: usize) -> Result<f32, tuxmix_core::Error> {
        delegate!(self, volume(ch, out))
    }
    fn set_pan(&mut self, ch: ChannelId, out: usize, p: i8) -> Result<(), tuxmix_core::Error> {
        delegate!(self, set_pan(ch, out, p))
    }
    fn pan(&self, ch: ChannelId, out: usize) -> Result<i8, tuxmix_core::Error> {
        delegate!(self, pan(ch, out))
    }
    fn set_mute(&mut self, ch: ChannelId, m: bool) -> Result<(), tuxmix_core::Error> {
        delegate!(self, set_mute(ch, m))
    }
    fn mute(&self, ch: ChannelId) -> Result<bool, tuxmix_core::Error> {
        delegate!(self, mute(ch))
    }
    fn set_solo(&mut self, ch: ChannelId, s: bool) -> Result<(), tuxmix_core::Error> {
        delegate!(self, set_solo(ch, s))
    }
    fn solo(&self, ch: ChannelId) -> Result<bool, tuxmix_core::Error> {
        delegate!(self, solo(ch))
    }
    fn capture_scene(&self) -> tuxmix_core::Scene {
        delegate!(self, capture_scene)
    }
    fn apply_scene(&mut self, s: &tuxmix_core::Scene) -> Result<(), tuxmix_core::Error> {
        delegate!(self, apply_scene(s))
    }
    fn poll_events(&mut self) -> Result<(), tuxmix_core::Error> {
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
const OUT_LABELS: [&str; 6] = ["AN1/2", "PH3/4", "AS1/2", "A3/A4", "A5/A6", "A7/A8"];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let mock = std::env::args().any(|a| a == "--mock");
    let mut device: DeviceHandle = if mock {
        DeviceHandle::open_mock()
    } else {
        DeviceHandle::open_real().unwrap_or_else(|| {
            eprintln!("No device found. Use --mock.");
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
    let mut section: usize = 0; // 0=inputs, 1=playbacks, 2=outputs // 0=inputs, 1=playbacks
    let mut channel: usize = 0;
    loop {
        let _ = dev.poll_events();
        term.draw(|f| ui(f, dev, show_matrix, section, channel))?;
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Tab => show_matrix = !show_matrix,
                        KeyCode::Left => {
                            if channel > 0 {
                                channel -= 1;
                            }
                        }
                        KeyCode::Right => {
                            let max = match section {
                                0 => dev.inputs().len(),
                                1 => dev.playbacks().len(),
                                _ => dev.outputs().len(),
                            };
                            if channel + 1 < max {
                                channel += 1;
                            }
                        }
                        KeyCode::Up => {
                            if section > 0 {
                                section -= 1;
                                channel = 0;
                            }
                        }
                        KeyCode::Down => {
                            let max_sec = 2;
                            if section < max_sec {
                                section += 1;
                                channel = 0;
                            }
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            let cid = match section {
                                0 => ChannelId::Input(channel),
                                1 => ChannelId::Playback(channel),
                                _ => ChannelId::Output(channel),
                            };
                            if let Ok(v) = dev.volume(cid, 0) {
                                let _ = dev.set_volume(cid, 0, (v + 0.05).min(1.0));
                            }
                        }
                        KeyCode::Char('-') => {
                            let cid = match section {
                                0 => ChannelId::Input(channel),
                                1 => ChannelId::Playback(channel),
                                _ => ChannelId::Output(channel),
                            };
                            if let Ok(v) = dev.volume(cid, 0) {
                                let _ = dev.set_volume(cid, 0, (v - 0.05).max(0.0));
                            }
                        }
                        KeyCode::Char('m') => {
                            let cid = match section {
                                0 => ChannelId::Input(channel),
                                1 => ChannelId::Playback(channel),
                                _ => ChannelId::Output(channel),
                            };
                            if let Ok(m) = dev.mute(cid) {
                                let _ = dev.set_mute(cid, !m);
                            }
                        }
                        KeyCode::Char('s') => {
                            let cid = match section {
                                0 => ChannelId::Input(channel),
                                1 => ChannelId::Playback(channel),
                                _ => ChannelId::Output(channel),
                            };
                            if let Ok(s) = dev.solo(cid) {
                                let _ = dev.set_solo(cid, !s);
                            }
                        }
                        KeyCode::Char('p') => {
                            if section == 0 {
                                if let Some(ic) = dev.inputs_mut().get_mut(channel) {
                                    if ic.channel_type == tuxmix_core::ChannelType::Mic {
                                        ic.phantom = !ic.phantom;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, dev: &DeviceHandle, show_matrix: bool, sel_sec: usize, sel_chan: usize) {
    let area = f.area();
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
    let (inputs_area, playbacks_area, outputs_area, matrix_area) = if show_matrix {
        (Rect::default(), Rect::default(), Rect::default(), content)
    } else {
        let c = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Min(1), Constraint::Min(1)])
            .split(content);
        (c[0], c[1], c[2], Rect::default())
    };
    let view_tag = if show_matrix {
        " [Matrix]".yellow().bold().to_string()
    } else {
        String::new()
    };
    let mode = if dev.is_mock() {
        " [SIMULATED]".yellow().bold()
    } else {
        "".into()
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("TuxMix", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" - {}  ", dev.model_name())),
            mode,
            Span::raw(format!("{}", view_tag)),
            Span::raw("  q:quit Tab:toggle"),
        ]))
        .block(Block::default().borders(Borders::ALL)),
        top[0],
    );

    let s = format!(
        "HW Inputs: {}  |  SW Playbacks: {}  |  Output pairs: {}  |  Clock: {}",
        dev.inputs().len(),
        dev.playbacks().len(),
        dev.output_pair_count(),
        dev.settings().clock_source
    );
    f.render_widget(
        Paragraph::new(s).block(Block::default().borders(Borders::ALL).title("Overview")),
        top[1],
    );

    if show_matrix {
        render_matrix(f, "Matrix Mixer", matrix_area, dev);
    } else {
        render_strips(
            f,
            "Hardware Inputs",
            inputs_area,
            dev.inputs().len(),
            sel_sec == 0,
            sel_chan,
            |i| {
                let ch = &dev.inputs()[i];
                let m = dev.input_meter(i);
                let mut label = format!("{} [{:?}]", ch.name, ch.channel_type);
                if ch.mute {
                    label.push_str(" [M]");
                }
                if ch.solo {
                    label.push_str(" [S]");
                }
                if ch.phantom {
                    label.push_str(" 48V");
                }
                if ch.pad {
                    label.push_str(" PAD");
                }
                (label, m)
            },
        );
        render_strips(
            f,
            "Software Playbacks",
            playbacks_area,
            dev.playbacks().len(),
            sel_sec == 1,
            sel_chan,
            |i| {
                let ch = &dev.playbacks()[i];
                let m = dev.playback_meter(i);
                let mut label = format!("{}", ch.name);
                if ch.mute {
                    label.push_str(" [M]");
                }
                if ch.solo {
                    label.push_str(" [S]");
                }
                (label, m)
            },
        );
        render_strips(
            f,
            "Hardware Outputs",
            outputs_area,
            dev.outputs().len(),
            sel_sec == 2,
            sel_chan,
            |i| {
                let ch = &dev.outputs()[i];
                let mut label = format!("{}", ch.name);
                if ch.mute {
                    label.push_str(" [M]");
                }
                if ch.solo {
                    label.push_str(" [S]");
                }
                (label, 0.0)
            },
        );
    }
    let footer: String = if show_matrix {
        "Tab: return to mixer".into()
    } else {
        format!(
            "{}:{}  +/-:vol  m:mute  s:solo  p:48V  arrows:navigate  q:quit",
            match sel_sec {
                0 => "IN",
                1 => "PB",
                _ => "OUT",
            },
            sel_chan
        )
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
    is_focused: bool,
    selected: usize,
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
        let w = inner.width / cols;
        let ch_area = Rect::new(
            inner.left() + col * w,
            inner.top() + row * row_h,
            w,
            row_h - 1,
        );
        let is_sel = is_focused && i == selected;
        let mut style = Style::default();
        if is_sel {
            style = style
                .bg(Color::Rgb(0x2a, 0x6a, 0x88))
                .add_modifier(Modifier::BOLD);
        }
        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                if is_sel {
                    format!("> {} <", label)
                } else {
                    label
                },
                style,
            )])),
            ch_area,
        );
        if meter > 0.0 {
            let ma = Rect::new(
                ch_area.left(),
                ch_area.bottom().saturating_sub(2),
                ch_area.width.min(20),
                1,
            );
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
    let mut lines = Vec::new();
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
    f.render_widget(Paragraph::new(lines.join("\n")), inner);
}
