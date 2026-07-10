use iced::keyboard::{self, Key};
use iced::widget::{column, container, pick_list, row, scrollable, text};
use iced::{window, Element, Length, Subscription, Task};
use std::collections::HashMap;
use std::time::Duration;

use tinyface_core::{
    BabyfacePro, ChannelId, ChannelType, MockBabyfacePro, RmeDevice, Scene,
};

use crate::matrix;
use crate::scenes::{list_scene_files, load_scene_file, save_scene_file};
use crate::theme;
use crate::widgets::strip;

pub const OUT_LABELS: [&str; 6] = ["AN1/2", "PH3/4", "AS1/2", "A3/A4", "A5/A6", "A7/A8"];

pub fn short_label(name: &str) -> &str {
    name.strip_prefix("PCM ").unwrap_or(name)
}

pub fn type_tag(t: ChannelType) -> (&'static str, iced::Color) {
    match t {
        ChannelType::Mic => ("MIC", theme::MUTE_COLOR),
        ChannelType::Instrument => ("INST", iced::Color::from_rgb8(0xff, 0xb7, 0x4d)),
        ChannelType::Line => ("LINE", theme::ACCENT),
        ChannelType::SPDIF => ("SPDIF", iced::Color::from_rgb8(0xba, 0x68, 0xc8)),
        ChannelType::ADAT => ("ADAT", iced::Color::from_rgb8(0xba, 0x68, 0xc8)),
    }
}

pub fn parse_db_input(s: &str) -> Option<f32> {
    let raw = s.trim().to_lowercase();
    if raw.is_empty() || raw == "-inf" || raw == "-\u{221e}" {
        return Some(0.0);
    }
    raw.replace(',', ".")
        .parse::<f32>()
        .ok()
        .map(|db| (10f32.powf(db / 20.0)).clamp(0.0, 1.0))
}

pub fn db_text(vol: f32) -> String {
    if vol > 0.0 {
        format!("{:.1} dB", 20.0 * vol.log10())
    } else {
        "-\u{221e} dB".into()
    }
}

// ── Device enum ──────────────────────────────────────────────────

pub enum DeviceHandle {
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
    fn outputs(&self) -> &[tinyface_core::OutputChannel] {
        delegate!(self, outputs)
    }
    fn outputs_mut(&mut self) -> &mut [tinyface_core::OutputChannel] {
        delegate!(self, outputs_mut)
    }
    fn settings(&self) -> &tinyface_core::DeviceSettings {
        delegate!(self, settings)
    }
    fn settings_mut(&mut self) -> &mut tinyface_core::DeviceSettings {
        delegate!(self, settings_mut)
    }
    fn set_volume(&mut self, ch: ChannelId, out: usize, v: f32) -> Result<(), tinyface_core::Error> {
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
    fn set_mute(&mut self, ch: ChannelId, m: bool) -> Result<(), tinyface_core::Error> {
        delegate!(self, set_mute(ch, m))
    }
    fn mute(&self, ch: ChannelId) -> Result<bool, tinyface_core::Error> {
        delegate!(self, mute(ch))
    }
    fn set_solo(&mut self, ch: ChannelId, s: bool) -> Result<(), tinyface_core::Error> {
        delegate!(self, set_solo(ch, s))
    }
    fn solo(&self, ch: ChannelId) -> Result<bool, tinyface_core::Error> {
        delegate!(self, solo(ch))
    }
    fn capture_scene(&self) -> Scene {
        delegate!(self, capture_scene)
    }
    fn apply_scene(&mut self, s: &Scene) -> Result<(), tinyface_core::Error> {
        delegate!(self, apply_scene(s))
    }
    fn poll_events(&mut self) -> Result<(), tinyface_core::Error> {
        delegate!(self, poll_events)
    }
}

impl DeviceHandle {
    pub fn open_real() -> Option<Self> {
        BabyfacePro::open().ok().map(DeviceHandle::Real)
    }
    pub fn open_mock() -> Self {
        DeviceHandle::Mock(MockBabyfacePro::open().expect("mock opens"))
    }
    pub fn input_meter(&self, idx: usize) -> f32 {
        match self {
            DeviceHandle::Mock(d) => d.input_meter(idx),
            _ => 0.0,
        }
    }
    pub fn playback_meter(&self, idx: usize) -> f32 {
        match self {
            DeviceHandle::Mock(d) => d.playback_meter(idx),
            _ => 0.0,
        }
    }
    pub fn is_mock(&self) -> bool {
        matches!(self, DeviceHandle::Mock(_))
    }
}

// ── Messages ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    SelectOutput(usize),
    SceneNameChanged(String),
    SceneSave,
    SceneLoad(String),
    ModifiersChanged(keyboard::Modifiers),
    TabPressed,
    EscapePressed,

    Mute(ChannelId, bool),
    Solo(ChannelId, bool),
    Phantom(usize, bool),
    Pad(usize, bool),

    VolumeChanged(ChannelId, usize, f32),
    FaderPressed(ChannelId, usize, f32, Option<(f32, f32)>),
    RangeCleared(ChannelId),
    Reset(ChannelId, usize, f32),

    EditStart(ChannelId, String),
    EditChanged(String),
    EditCommit,
}

// ── App state ────────────────────────────────────────────────────

pub struct TinyFace {
    pub device: DeviceHandle,
    pub sel_out: usize,
    pub show_matrix: bool,
    pub phantom_overrides: HashMap<usize, bool>,
    pub pad_overrides: HashMap<usize, bool>,
    pub editing: Option<ChannelId>,
    pub edit_buf: String,
    pub drag_range: Option<(ChannelId, f32, f32)>,
    pub scene_name: String,
    pub scene_list: Vec<String>,
    pub modifiers: keyboard::Modifiers,
}

pub fn new(mock: bool) -> TinyFace {
    let device = if mock {
        DeviceHandle::open_mock()
    } else {
        DeviceHandle::open_real().unwrap_or_else(|| {
            eprintln!("No device found. Use --mock for simulation.");
            DeviceHandle::open_mock()
        })
    };
    TinyFace {
        device,
        sel_out: 0,
        show_matrix: false,
        phantom_overrides: HashMap::new(),
        pad_overrides: HashMap::new(),
        editing: None,
        edit_buf: String::new(),
        drag_range: None,
        scene_name: String::new(),
        scene_list: list_scene_files(),
        modifiers: keyboard::Modifiers::default(),
    }
}

pub fn title(state: &TinyFace) -> String {
    let _ = state;
    "Tinyface - RME Mixer".into()
}

pub fn update(state: &mut TinyFace, message: Message) -> Task<Message> {
    match message {
        Message::Tick => {
            let _ = state.device.poll_events();
        }
        Message::TabPressed => {
            state.show_matrix = !state.show_matrix;
        }
        Message::SelectOutput(i) => state.sel_out = i,
        Message::SceneNameChanged(s) => state.scene_name = s,
        Message::SceneSave => {
            let n = state.scene_name.trim().to_string();
            if !n.is_empty() && save_scene_file(&n, &state.device.capture_scene()).is_ok() {
                state.scene_name.clear();
                state.scene_list = list_scene_files();
            }
        }
        Message::SceneLoad(name) => {
            if let Some(scene) = load_scene_file(&name) {
                let _ = state.device.apply_scene(&scene);
            }
        }
        Message::ModifiersChanged(m) => state.modifiers = m,
        Message::EscapePressed => {
            if state.editing.is_some() {
                state.editing = None;
            }
        }
        Message::Mute(cid, m) => {
            let _ = state.device.set_mute(cid, m);
        }
        Message::Solo(cid, s) => {
            let _ = state.device.set_solo(cid, s);
        }
        Message::Phantom(idx, p) => {
            state.phantom_overrides.insert(idx, p);
        }
        Message::Pad(idx, p) => {
            state.pad_overrides.insert(idx, p);
        }
        Message::VolumeChanged(cid, out, v) => {
            let _ = state.device.set_volume(cid, out, v);
        }
        Message::FaderPressed(cid, out, v, range) => {
            if let Some((lo, hi)) = range {
                state.drag_range = Some((cid, lo, hi));
            }
            let _ = state.device.set_volume(cid, out, v);
        }
        Message::RangeCleared(cid) => {
            if state.drag_range.is_some_and(|(dc, _, _)| dc == cid) {
                state.drag_range = None;
            }
        }
        Message::Reset(cid, out, default_vol) => {
            let _ = state.device.set_volume(cid, out, default_vol);
            if state.drag_range.is_some_and(|(dc, _, _)| dc == cid) {
                state.drag_range = None;
            }
        }
        Message::EditStart(cid, buf) => {
            state.editing = Some(cid);
            state.edit_buf = buf;
        }
        Message::EditChanged(s) => state.edit_buf = s,
        Message::EditCommit => {
            if let Some(cid) = state.editing {
                if let Some(v) = parse_db_input(&state.edit_buf) {
                    let _ = state.device.set_volume(cid, state.sel_out, v);
                }
                state.editing = None;
            }
        }
    }
    Task::none()
}

pub fn subscription(_state: &TinyFace) -> Subscription<Message> {
    Subscription::batch([
        iced::time::every(Duration::from_millis(50)).map(|_| Message::Tick),
        iced::event::listen_with(handle_global_event),
    ])
}

fn handle_global_event(
    event: iced::Event,
    _status: iced::event::Status,
    _id: window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => match key {
            Key::Named(keyboard::key::Named::Tab) => Some(Message::TabPressed),
            Key::Named(keyboard::key::Named::Escape) => Some(Message::EscapePressed),
            _ => None,
        },
        iced::Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
            Some(Message::ModifiersChanged(m))
        }
        _ => None,
    }
}

// ── View ─────────────────────────────────────────────────────────

pub fn view(state: &TinyFace) -> Element<'_, Message> {
    let top = top_bar(state);
    let content = if state.show_matrix {
        matrix_view(state)
    } else {
        mixer_view(state)
    };

    column![top, content].into()
}

/// A section label (HARDWARE INPUTS, SOFTWARE PLAYBACK, ...) with an accent
/// tick and a rule trailing off to the right, instead of bare gray text that
/// blends into the background.
fn section_header(label: &str) -> Element<'_, Message> {
    row![
        container(iced::widget::Space::new().width(3).height(12)).style(theme::accent_bar),
        text(label).color(theme::TEXT_PRIMARY).size(11),
        iced::widget::rule::horizontal(1),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

/// Wraps a cluster of related controls in a recessed "chip" so the top bar
/// reads as grouped sections instead of one long undifferentiated row.
fn chip<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .style(theme::chip)
        .padding([5, 12])
        .into()
}

fn top_bar(state: &TinyFace) -> Element<'_, Message> {
    let status_color = if state.device.is_mock() {
        theme::YSIM
    } else {
        theme::GCONN
    };
    let status_label = if state.device.is_mock() {
        "Simulated"
    } else {
        "Connected"
    };

    let device_chip = chip(
        row![
            text("●").color(status_color).size(10),
            text(state.device.model_name())
                .color(theme::TEXT_PRIMARY)
                .size(13),
            text(status_label).color(status_color).size(11),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    );

    let tab_chip = chip(
        text(if state.show_matrix { "MATRIX" } else { "MIXER" })
            .color(theme::ACCENT)
            .size(11),
    );

    let scene_list = state.scene_list.clone();
    let scene_group = chip(
        row![
            text("Scene").color(theme::TEXT_SEC).size(11),
            iced::widget::text_input("name", &state.scene_name)
                .on_input(Message::SceneNameChanged)
                .on_submit(Message::SceneSave)
                .style(theme::text_input)
                .width(Length::Fixed(90.0))
                .size(11),
            iced::widget::button(text("Save").size(11))
                .style(theme::plain_button)
                .on_press(Message::SceneSave),
            pick_list(scene_list, None::<String>, Message::SceneLoad)
                .placeholder("load...")
                .style(theme::pick_list)
                .menu_style(theme::menu)
                .text_size(11),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    );

    let submix_group = chip(
        row![
            text("Submix").color(theme::TEXT_SEC).size(11),
            pick_list(
                OUT_LABELS.to_vec(),
                Some(OUT_LABELS[state.sel_out]),
                |label| {
                    let idx = OUT_LABELS.iter().position(|l| *l == label).unwrap_or(0);
                    Message::SelectOutput(idx)
                },
            )
            .style(theme::pick_list)
            .menu_style(theme::menu)
            .text_size(12),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    );

    let clock_chip = chip(
        text(state.device.settings().clock_source.clone())
            .color(theme::TEXT_SEC)
            .size(11),
    );

    let bar = row![
        text("Tinyface").color(theme::ACCENT).size(20),
        device_chip,
        tab_chip,
        iced::widget::Space::new().width(Length::Fill),
        scene_group,
        submix_group,
        clock_chip,
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    container(bar)
        .style(theme::top_bar)
        .padding([10, 16])
        .width(Length::Fill)
        .into()
}

fn mixer_view(state: &TinyFace) -> Element<'_, Message> {
    let mut input_strips = row![].spacing(6);
    let mut prev_type: Option<ChannelType> = None;
    for (i, ch) in state.device.inputs().iter().enumerate() {
        if prev_type.is_some_and(|t| t != ch.channel_type) {
            input_strips = input_strips.push(iced::widget::rule::vertical(1));
        }
        prev_type = Some(ch.channel_type);

        let cid = ChannelId::Input(i);
        let meter = state.device.input_meter(i);
        let has_48v = ch.channel_type == ChannelType::Mic;
        let phantom = *state.phantom_overrides.get(&i).unwrap_or(&ch.phantom);
        let pad = *state.pad_overrides.get(&i).unwrap_or(&ch.pad);
        let drag_range = state
            .drag_range
            .and_then(|(dc, lo, hi)| (dc == cid).then_some((lo, hi)));

        input_strips = input_strips.push(strip::strip(strip::StripParams {
            cid,
            output_idx: state.sel_out,
            name: ch.name.clone(),
            type_tag: Some(type_tag(ch.channel_type)),
            vol: ch.volumes[state.sel_out],
            pan: ch.pans[state.sel_out],
            meter,
            has_48v,
            has_pad: has_48v,
            phantom,
            pad,
            mute: ch.mute,
            solo: ch.solo,
            default_vol: 0.75,
            editing: state.editing == Some(cid),
            edit_buf: &state.edit_buf,
            drag_range,
            modifiers: state.modifiers,
        }));
    }

    let mut pb_strips = row![].spacing(6);
    for (i, ch) in state.device.playbacks().iter().enumerate() {
        let cid = ChannelId::Playback(i);
        let meter = state.device.playback_meter(i);
        let drag_range = state
            .drag_range
            .and_then(|(dc, lo, hi)| (dc == cid).then_some((lo, hi)));

        pb_strips = pb_strips.push(strip::strip(strip::StripParams {
            cid,
            output_idx: state.sel_out,
            name: ch.name.clone(),
            type_tag: Some(("PB", theme::TEXT_SEC)),
            vol: ch.volumes[state.sel_out],
            pan: ch.pans[state.sel_out],
            meter,
            has_48v: false,
            has_pad: false,
            phantom: false,
            pad: false,
            mute: ch.mute,
            solo: ch.solo,
            default_vol: 0.8,
            editing: state.editing == Some(cid),
            edit_buf: &state.edit_buf,
            drag_range,
            modifiers: state.modifiers,
        }));
    }

    let mut out_strips = row![].spacing(6);
    for (i, ch) in state.device.outputs().iter().enumerate() {
        let cid = ChannelId::Output(i);
        let drag_range = state
            .drag_range
            .and_then(|(dc, lo, hi)| (dc == cid).then_some((lo, hi)));

        out_strips = out_strips.push(strip::strip(strip::StripParams {
            cid,
            output_idx: state.sel_out,
            name: ch.name.clone(),
            type_tag: Some(("OUT", theme::TEXT_SEC)),
            vol: ch.volume,
            pan: 0,
            meter: 0.0,
            has_48v: false,
            has_pad: false,
            phantom: false,
            pad: false,
            mute: ch.mute,
            solo: ch.solo,
            default_vol: 1.0,
            editing: state.editing == Some(cid),
            edit_buf: &state.edit_buf,
            drag_range,
            modifiers: state.modifiers,
        }));
    }

    let body = column![
        section_header("HARDWARE INPUTS"),
        text(format!(
            "Submix: {} - Tab for Matrix",
            OUT_LABELS[state.sel_out]
        ))
        .color(theme::TEXT_SEC)
        .size(10),
        scrollable(input_strips).direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default()
        )),
        section_header("SOFTWARE PLAYBACK"),
        scrollable(pb_strips).direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default()
        )),
        section_header("HARDWARE OUTPUTS"),
        scrollable(out_strips).direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default()
        )),
    ]
    .spacing(8);

    container(body)
        .style(theme::root)
        .padding([8, 12])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn matrix_view(state: &TinyFace) -> Element<'_, Message> {
    let body = column![
        section_header("MATRIX MIXER"),
        text("Volume per input per output - Tab to return")
            .color(theme::TEXT_SEC)
            .size(10),
        matrix::view(state),
    ]
    .spacing(8);

    container(body)
        .style(theme::root)
        .padding([8, 12])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
