use iced::keyboard::{self, Key};
use iced::widget::{column, container, mouse_area, pick_list, row, scrollable, text};
use iced::{mouse, window, Element, Length, Subscription, Task};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use tuxmix_core::{
    BabyfacePro, ChannelId, ChannelType, MockBabyfacePro, RmeDevice, Scene,
};

use crate::matrix;
use crate::scenes::{list_scene_files, load_scene_file, save_scene_file};
use crate::theme;
use crate::widgets::fader;
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

/// Tag colors for the bus rows (not real `ChannelType`s, so not part of
/// `type_tag`) — kept distinct from the input-type palette above so PB/OUT
/// don't just blend into the secondary-text gray everything else uses.
pub const PB_TAG: iced::Color = iced::Color::from_rgb8(0x4d, 0xb6, 0xac);
pub const OUT_TAG: iced::Color = iced::Color::from_rgb8(0x81, 0xc7, 0x84);

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
    fn set_volume(&mut self, ch: ChannelId, out: usize, v: f32) -> Result<(), tuxmix_core::Error> {
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
    fn capture_scene(&self) -> Scene {
        delegate!(self, capture_scene)
    }
    fn apply_scene(&mut self, s: &Scene) -> Result<(), tuxmix_core::Error> {
        delegate!(self, apply_scene(s))
    }
    fn poll_events(&mut self) -> Result<(), tuxmix_core::Error> {
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
    ScaleUp,
    ScaleDown,
    ScaleReset,
    /// Ctrl+scroll, anywhere in the window, zooms the whole interface —
    /// same as Ctrl+=/Ctrl+-. Dispatched from the global event stream (see
    /// `handle_global_event`), not from a specific widget, so it fires
    /// consistently no matter what's under the cursor.
    ScrollZoom(mouse::ScrollDelta),

    Mute(ChannelId, bool),
    Solo(ChannelId, bool),
    Phantom(usize, bool),
    Pad(usize, bool),

    VolumeChanged(ChannelId, usize, f32),
    FaderPressed(ChannelId, usize, f32, Option<(f32, f32)>),
    RangeCleared(ChannelId),
    Reset(ChannelId, usize, f32),

    PanChanged(ChannelId, usize, i8),
    PanReset(ChannelId, usize),
    ToggleCollapse(ChannelId),
    /// Fires only while `collapse_anim` is non-empty (see `subscription`) —
    /// exists purely to trigger a redraw at a much higher rate than the
    /// normal 50ms `Tick` so the width tween looks smooth, and to prune
    /// entries once they've settled so that faster timer can shut off.
    CollapseTick,

    EditStart(ChannelId, String),
    EditChanged(String),
    EditCommit,

    /// Ctrl/Shift+click on a strip's non-control area toggles its
    /// selection membership; a plain click there is a no-op (so
    /// double-click-to-collapse on a selected strip isn't disrupted by an
    /// intervening deselect on the first press).
    StripClicked(ChannelId),
    /// Plain click on genuinely empty page background clears the
    /// selection — see `page()`.
    ClearSelection,
}

// ── App state ────────────────────────────────────────────────────

pub struct TuxMix {
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
    /// Ballistics-smoothed meter values shown in the UI — the raw values
    /// from `device.input_meter`/`playback_meter` jump straight to their new
    /// reading every tick, which reads as flickery rather than a real meter
    /// needle. Smoothed here instead of at the device layer so it applies
    /// uniformly regardless of data source (mock or real hardware).
    pub input_meters: Vec<MeterAnim>,
    pub playback_meters: Vec<MeterAnim>,
    /// Strips the user has collapsed to save horizontal space — presence in
    /// the set means collapsed. This is the *target* state; the strip may
    /// still be mid-transition, tracked separately in `collapse_anim`.
    pub collapsed: HashSet<ChannelId>,
    /// In-flight collapse/expand width animations, keyed by strip — only
    /// holds an entry while a strip is actually transitioning; pruned once
    /// settled (see `Message::CollapseTick`), which also lets the extra
    /// high-frequency redraw timer in `subscription()` shut itself off.
    pub collapse_anim: HashMap<ChannelId, strip::CollapseAnim>,
    /// Live UI zoom (Ctrl+=/Ctrl+-/Ctrl+0), multiplied into every text size
    /// and widget dimension in the mixer/matrix views. `theme::SCALE_*`
    /// constants define the default/step/bounds.
    pub ui_scale: f32,
    /// Multi-selected strips — Ctrl+click toggles just the clicked strip,
    /// Shift+click selects the whole range from `select_anchor` (standard
    /// file-manager convention), click empty background to clear.
    /// Mute/solo/collapse/volume/pan applied to any selected strip apply
    /// to the whole selection at once.
    pub selected: HashSet<ChannelId>,
    /// The strip a future Shift+click's range is measured from — the
    /// most recent strip explicitly Ctrl- or Shift-clicked. Not moved by
    /// Shift+click itself, so repeated Shift+clicks pivot around the same
    /// point (letting you grow/shrink a range interactively) the way
    /// Explorer/Finder do.
    pub select_anchor: Option<ChannelId>,
}

/// Matches the `Tick` subscription interval below — the release curve is
/// timed in real milliseconds rather than "per tick" so it stays correct if
/// that interval ever changes.
const METER_TICK_MS: f32 = 50.0;
/// Fast rise — a meter should jump to a new peak almost instantly so
/// transients don't feel muted.
const METER_ATTACK: f32 = 0.7;
/// Release rate right after a peak: falls quickly at first...
const METER_RELEASE_START: f32 = 0.22;
/// ...decelerating to a gentle final approach as it settles, instead of
/// falling at one constant rate the whole way down. This ease-out shape
/// (fast-then-gentle) is the same curve easyeffects animates its meters
/// with (a 300ms cubic ease-out) — it's what reads as a real analog needle
/// settling rather than a value sliding down at a fixed speed.
const METER_RELEASE_END: f32 = 0.04;
/// Time to go from `METER_RELEASE_START` to `METER_RELEASE_END` after a peak.
const METER_RELEASE_MS: f32 = 300.0;

/// Per-channel VU ballistics state.
#[derive(Clone, Copy, Debug)]
pub struct MeterAnim {
    /// Value as of the *previous* `step` — the start of the current
    /// keyframe transition `MeterFrame` interpolates from.
    prev_value: f32,
    value: f32,
    /// When `value` was last computed — the display layer (`MeterFrame`)
    /// uses this to interpolate a smooth in-between value at full display
    /// refresh rate instead of jumping once per `Tick`.
    last_step_at: Instant,
    /// Time since the level last rose (i.e. since the last peak) — drives
    /// the release ease-out curve. Clamped at `METER_RELEASE_MS`, meaning
    /// "fully settled into the tail rate".
    release_elapsed_ms: f32,
}

impl MeterAnim {
    fn new() -> Self {
        Self {
            prev_value: 0.0,
            value: 0.0,
            last_step_at: Instant::now(),
            release_elapsed_ms: METER_RELEASE_MS,
        }
    }

    pub fn frame(&self) -> fader::MeterFrame {
        fader::MeterFrame {
            prev: self.prev_value,
            value: self.value,
            since: self.last_step_at,
        }
    }

    fn step(&mut self, target: f32) {
        self.prev_value = self.value;
        if target >= self.value {
            self.value += (target - self.value) * METER_ATTACK;
            self.release_elapsed_ms = 0.0;
        } else {
            self.release_elapsed_ms = (self.release_elapsed_ms + METER_TICK_MS).min(METER_RELEASE_MS);
            let t = self.release_elapsed_ms / METER_RELEASE_MS;
            let alpha = METER_RELEASE_END + (METER_RELEASE_START - METER_RELEASE_END) * (1.0 - t) * (1.0 - t);
            self.value += (target - self.value) * alpha;
        }
        self.last_step_at = Instant::now();
    }
}

pub fn new(mock: bool) -> TuxMix {
    let device = if mock {
        DeviceHandle::open_mock()
    } else {
        DeviceHandle::open_real().unwrap_or_else(|| {
            eprintln!("No device found. Use --mock for simulation.");
            DeviceHandle::open_mock()
        })
    };
    let n_inputs = device.inputs().len();
    let n_playbacks = device.playbacks().len();
    TuxMix {
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
        input_meters: vec![MeterAnim::new(); n_inputs],
        playback_meters: vec![MeterAnim::new(); n_playbacks],
        collapsed: HashSet::new(),
        collapse_anim: HashMap::new(),
        ui_scale: theme::SCALE_DEFAULT,
        selected: HashSet::new(),
        select_anchor: None,
    }
}

pub fn title(state: &TuxMix) -> String {
    let _ = state;
    "TuxMix - RME Mixer".into()
}

/// Floor used when converting silence (linear 0.0) to dB for group-delta
/// math — an actual `f32::NEG_INFINITY` would turn one dragged-to-zero
/// channel into an infinite delta that snaps every other selected channel
/// to 0.0 or 1.0 depending on direction. A large-but-finite floor keeps
/// the swing dramatic (as it should be) without the infinity/NaN edge
/// case.
const GROUP_SILENCE_DB: f32 = -100.0;

fn vol_to_db(v: f32) -> f32 {
    if v <= 0.0 {
        GROUP_SILENCE_DB
    } else {
        (20.0 * v.log10()).max(GROUP_SILENCE_DB)
    }
}

fn db_to_vol(db: f32) -> f32 {
    if db <= GROUP_SILENCE_DB {
        0.0
    } else {
        10f32.powf(db / 20.0)
    }
}

/// Every selectable channel, in the same order they're laid out on
/// screen (Hardware Inputs, then Software Playback, then Hardware
/// Outputs, each in index order) — the linear ordering Shift+click range
/// selection is measured against, so a range visually spans exactly the
/// strips between two clicks the way it would in a file manager.
fn channel_order(state: &TuxMix) -> Vec<ChannelId> {
    let d = &state.device;
    (0..d.inputs().len())
        .map(ChannelId::Input)
        .chain((0..d.playbacks().len()).map(ChannelId::Playback))
        .chain((0..d.outputs().len()).map(ChannelId::Output))
        .collect()
}

/// Sets `cid`'s volume to `v`. If `cid` is part of an active multi-selection,
/// every other selected channel moves by the same *relative* amount instead
/// of jumping to the same absolute level — preserving the balance between
/// them, the way dragging one fader in a DAW's multi-track selection moves
/// the whole group together rather than flattening it to one value.
///
/// The delta is computed in dB, not raw linear amplitude — the fader's own
/// travel is dB-tapered, so an equal *linear* delta applied to channels
/// sitting at different points on that curve produces wildly different dB
/// swings (a channel near the bottom barely moves while one near unity
/// swings hard). dB delta is what actually reads as "moving together."
fn apply_grouped_volume(state: &mut TuxMix, cid: ChannelId, out: usize, v: f32) {
    if state.selected.len() > 1 && state.selected.contains(&cid) {
        let old = state.device.volume(cid, out).unwrap_or(v);
        let delta_db = vol_to_db(v) - vol_to_db(old);
        for sel in state.selected.clone() {
            let cur = state.device.volume(sel, out).unwrap_or(0.0);
            let new_vol = db_to_vol(vol_to_db(cur) + delta_db).clamp(0.0, 1.0);
            let _ = state.device.set_volume(sel, out, new_vol);
        }
    } else {
        let _ = state.device.set_volume(cid, out, v);
    }
}

/// Same relative-delta grouping as `apply_grouped_volume`, for pan.
fn apply_grouped_pan(state: &mut TuxMix, cid: ChannelId, out: usize, pan: i8) {
    if state.selected.len() > 1 && state.selected.contains(&cid) {
        let old = i16::from(state.device.pan(cid, out).unwrap_or(pan));
        let delta = i16::from(pan) - old;
        for sel in state.selected.clone() {
            let cur = i16::from(state.device.pan(sel, out).unwrap_or(0));
            let new = (cur + delta).clamp(-100, 100) as i8;
            let _ = state.device.set_pan(sel, out, new);
        }
    } else {
        let _ = state.device.set_pan(cid, out, pan);
    }
}

/// Flips `cid`'s collapsed/expanded target and kicks off (or redirects, if
/// one was already in flight — e.g. double-clicking again before it
/// settles) the width animation that plays it out. A no-op if `cid` is
/// already at `target`, so re-applying the same target to a whole
/// selection doesn't restart every strip's animation from scratch.
fn set_collapsed(state: &mut TuxMix, cid: ChannelId, target: bool) {
    if state.collapsed.contains(&cid) == target {
        return;
    }
    let now = Instant::now();
    let current_w = state
        .collapse_anim
        .get(&cid)
        .map(|a| a.at(now))
        .unwrap_or(if state.collapsed.contains(&cid) {
            strip::COLLAPSED_W
        } else {
            strip::STRIP_W
        });
    let target_w = if target { strip::COLLAPSED_W } else { strip::STRIP_W };
    state.collapse_anim.insert(
        cid,
        strip::CollapseAnim { prev: current_w, value: target_w, since: now },
    );
    if target {
        state.collapsed.insert(cid);
    } else {
        state.collapsed.remove(&cid);
    }
}

pub fn update(state: &mut TuxMix, message: Message) -> Task<Message> {
    match message {
        Message::Tick => {
            let _ = state.device.poll_events();
            for (i, m) in state.input_meters.iter_mut().enumerate() {
                m.step(state.device.input_meter(i));
            }
            for (i, m) in state.playback_meters.iter_mut().enumerate() {
                m.step(state.device.playback_meter(i));
            }
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
                // Was previously `let _ = ...` — silently discarded a
                // scene/device model mismatch (or any other apply
                // failure) with no way for the user to ever find out
                // why nothing happened. No toast/notification system
                // exists yet, so a log line is the minimum fix that
                // makes the failure observable at all.
                if let Err(err) = state.device.apply_scene(&scene) {
                    log::warn!("Failed to apply scene '{name}': {err}");
                }
            }
        }
        Message::ModifiersChanged(m) => state.modifiers = m,
        Message::ScaleUp => {
            state.ui_scale = (state.ui_scale + theme::SCALE_STEP).min(theme::SCALE_MAX);
        }
        Message::ScaleDown => {
            state.ui_scale = (state.ui_scale - theme::SCALE_STEP).max(theme::SCALE_MIN);
        }
        Message::ScaleReset => state.ui_scale = theme::SCALE_DEFAULT,
        Message::ScrollZoom(delta) => {
            if state.modifiers.control() {
                // Same 20:1 ratio as the fader's own wheel handling — a
                // wheel "line" is one discrete detent, a trackpad "pixel"
                // stream needs a much smaller per-event step or a short
                // swipe would blow through the whole zoom range.
                let (dy, step) = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => (y, theme::SCALE_STEP),
                    mouse::ScrollDelta::Pixels { y, .. } => (y, theme::SCALE_STEP / 20.0),
                };
                state.ui_scale =
                    (state.ui_scale + dy * step).clamp(theme::SCALE_MIN, theme::SCALE_MAX);
            }
        }
        Message::EscapePressed => {
            if state.editing.is_some() {
                state.editing = None;
            }
        }
        Message::Mute(cid, m) => {
            if state.selected.len() > 1 && state.selected.contains(&cid) {
                for sel in state.selected.clone() {
                    let _ = state.device.set_mute(sel, m);
                }
            } else {
                let _ = state.device.set_mute(cid, m);
            }
        }
        Message::Solo(cid, s) => {
            if state.selected.len() > 1 && state.selected.contains(&cid) {
                for sel in state.selected.clone() {
                    let _ = state.device.set_solo(sel, s);
                }
            } else {
                let _ = state.device.set_solo(cid, s);
            }
        }
        Message::Phantom(idx, p) => {
            state.phantom_overrides.insert(idx, p);
        }
        Message::Pad(idx, p) => {
            state.pad_overrides.insert(idx, p);
        }
        Message::VolumeChanged(cid, out, v) => {
            apply_grouped_volume(state, cid, out, v);
        }
        Message::FaderPressed(cid, out, v, range) => {
            if let Some((lo, hi)) = range {
                state.drag_range = Some((cid, lo, hi));
            }
            apply_grouped_volume(state, cid, out, v);
        }
        Message::RangeCleared(cid) => {
            if state.drag_range.is_some_and(|(dc, _, _)| dc == cid) {
                state.drag_range = None;
            }
        }
        Message::Reset(cid, out, default_vol) => {
            if state.selected.len() > 1 && state.selected.contains(&cid) {
                // Reset means "back to default" for the whole group, not a
                // relative move — every selected fader snaps to the same
                // absolute value, unlike a drag which preserves balance.
                for sel in state.selected.clone() {
                    let _ = state.device.set_volume(sel, out, default_vol);
                }
            } else {
                let _ = state.device.set_volume(cid, out, default_vol);
            }
            if state.drag_range.is_some_and(|(dc, _, _)| dc == cid) {
                state.drag_range = None;
            }
        }
        Message::PanChanged(cid, out, pan) => {
            apply_grouped_pan(state, cid, out, pan);
        }
        Message::PanReset(cid, out) => {
            if state.selected.len() > 1 && state.selected.contains(&cid) {
                for sel in state.selected.clone() {
                    let _ = state.device.set_pan(sel, out, 0);
                }
            } else {
                let _ = state.device.set_pan(cid, out, 0);
            }
        }
        Message::ToggleCollapse(cid) => {
            let target = !state.collapsed.contains(&cid);
            if state.selected.len() > 1 && state.selected.contains(&cid) {
                // Uniform target for the whole group — the opposite of
                // what the clicked strip currently is — rather than each
                // toggling its own state independently, which would leave
                // them out of sync with each other.
                for sel in state.selected.clone() {
                    set_collapsed(state, sel, target);
                }
            } else {
                set_collapsed(state, cid, target);
            }
        }
        Message::CollapseTick => {
            let now = Instant::now();
            state.collapse_anim.retain(|_, a| a.is_settling(now));
        }
        Message::StripClicked(cid) => {
            if state.modifiers.control() {
                // Toggle just this one strip, leaving the rest of the
                // selection untouched — the standard Ctrl+click
                // convention. Becomes the pivot for the next Shift+click.
                if !state.selected.remove(&cid) {
                    state.selected.insert(cid);
                }
                state.select_anchor = Some(cid);
            } else if state.modifiers.shift() {
                // Select the whole visual range from the anchor through
                // cid, replacing the current selection — the standard
                // Shift+click convention. Doesn't move the anchor, so a
                // second Shift+click elsewhere re-measures from the same
                // start rather than the last endpoint.
                let order = channel_order(state);
                let anchor = state.select_anchor.unwrap_or(cid);
                if let (Some(from), Some(to)) = (
                    order.iter().position(|&c| c == anchor),
                    order.iter().position(|&c| c == cid),
                ) {
                    let (lo, hi) = (from.min(to), from.max(to));
                    state.selected = order[lo..=hi].iter().copied().collect();
                }
                if state.select_anchor.is_none() {
                    state.select_anchor = Some(cid);
                }
            }
        }
        Message::ClearSelection => {
            state.selected.clear();
            state.select_anchor = None;
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

pub fn subscription(state: &TuxMix) -> Subscription<Message> {
    let mut subs = vec![
        iced::time::every(Duration::from_millis(50)).map(|_| Message::Tick),
        iced::event::listen_with(handle_global_event),
    ];
    // Only running while a collapse/expand transition is actually in
    // flight — plain `column`/`container` widgets can't self-request a
    // redraw the way the canvas-based fader/meter animations do, so a
    // much faster timer stands in for that during the ~160ms transition,
    // then switches itself back off once `collapse_anim` empties out.
    if !state.collapse_anim.is_empty() {
        subs.push(iced::time::every(Duration::from_millis(8)).map(|_| Message::CollapseTick));
    }
    Subscription::batch(subs)
}

fn handle_global_event(
    event: iced::Event,
    _status: iced::event::Status,
    _id: window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            // Browser-style zoom shortcuts — Ctrl+= (no shift needed for
            // the unshifted "=" key) / Ctrl+- / Ctrl+0 to reset.
            if modifiers.control() {
                if let Key::Character(c) = &key {
                    match c.as_str() {
                        "=" | "+" => return Some(Message::ScaleUp),
                        "-" => return Some(Message::ScaleDown),
                        "0" => return Some(Message::ScaleReset),
                        _ => {}
                    }
                }
            }
            match key {
                Key::Named(keyboard::key::Named::Tab) => Some(Message::TabPressed),
                Key::Named(keyboard::key::Named::Escape) => Some(Message::EscapePressed),
                _ => None,
            }
        }
        iced::Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
            Some(Message::ModifiersChanged(m))
        }
        // Broadcast via the global event stream (not routed through the
        // widget tree), so Ctrl+scroll-to-zoom fires no matter what's
        // under the cursor — a fader, a horizontally-scrollable strip
        // row, or the page's own vertical scrollable all capture wheel
        // events locally for their own purpose first, which would
        // otherwise swallow the gesture before it could ever reach a
        // widget-tree handler placed on the background. `ScrollZoom`
        // itself still gates on Ctrl being held, so a plain scroll here
        // is a no-op — local widgets keep handling those normally.
        iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
            Some(Message::ScrollZoom(delta))
        }
        _ => None,
    }
}

// ── View ─────────────────────────────────────────────────────────

pub fn view(state: &TuxMix) -> Element<'_, Message> {
    let top = top_bar(state);
    let content = if state.show_matrix {
        matrix_view(state)
    } else {
        mixer_view(state)
    };

    // Explicit Fill — a Shrink parent doesn't actually grant a Fill-sized
    // child the real window height for layout/hit-testing, even though
    // the raw window clear color visually fills the gap identically to
    // our own background (same near-black), making a real empty area
    // indistinguishable on screen from a genuinely non-interactive one.
    // That's what made `page()`'s click-to-clear-selection silently miss
    // every click below the shortest section's natural content height.
    column![top, content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// A section label (HARDWARE INPUTS, SOFTWARE PLAYBACK, ...) with an accent
/// tick and a rule trailing off to the right, instead of bare gray text that
/// blends into the background.
fn section_header(label: &str, scale: f32) -> Element<'_, Message> {
    row![
        container(iced::widget::Space::new().width(3.0 * scale).height(12.0 * scale))
            .style(theme::accent_bar),
        text(label).color(theme::TEXT_PRIMARY).size(theme::TEXT_MD * scale),
        iced::widget::rule::horizontal(1),
    ]
    .spacing(theme::SPACE_LG)
    .align_y(iced::Alignment::Center)
    .into()
}

/// Wraps a view's body in the root background, filling the window, with a
/// vertical scrollbar for when the stacked sections (or the matrix grid)
/// don't fit the window height. Ctrl+scroll-to-zoom is *not* wired up here
/// — the vertical scrollable (and any strip row, fader, etc. inside it)
/// would capture the wheel event locally before it could ever bubble up
/// to this level. It's handled globally instead, in `handle_global_event`,
/// which taps the raw event stream independent of what the widget tree
/// did with it.
fn page<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    mouse_area(
        container(
            scrollable(body)
                .direction(scrollable::Direction::Vertical(theme::thin_scrollbar()))
                .width(Length::Fill)
                .style(theme::scrollable),
        )
        .style(theme::root)
        .padding([theme::SPACE_LG, theme::SPACE_XL])
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .on_press(Message::ClearSelection)
    .into()
}

/// Wraps a cluster of related controls in a recessed "chip" so the top bar
/// reads as grouped sections instead of one long undifferentiated row.
fn chip<'a>(content: impl Into<Element<'a, Message>>, scale: f32) -> Element<'a, Message> {
    container(content)
        .style(theme::chip)
        .padding([theme::SPACE_SM * scale, theme::SPACE_XL * scale])
        .into()
}

/// A thin vertical separator between sub-groups inside a merged chip —
/// lighter-weight than another chip boundary, just enough to break up
/// dense runs of controls (Scene tools / Submix / Clock) without adding a
/// third level of boxing.
fn v_divider<'a>(scale: f32) -> Element<'a, Message> {
    container(iced::widget::Space::new().width(1).height(16.0 * scale))
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(theme::BORDER)),
            ..container::Style::default()
        })
        .into()
}

fn top_bar(state: &TuxMix) -> Element<'_, Message> {
    let scale = state.ui_scale;
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

    // Primary identity: brand + connected device. The one element in the
    // bar that's meant to be visually loud — everything else is a tool,
    // this is "what am I even looking at".
    let device_chip = chip(
        row![
            text("●").color(status_color).size(theme::TEXT_SM * scale),
            text(state.device.model_name())
                .color(theme::TEXT_PRIMARY)
                .size(theme::TEXT_LG * scale),
            text(status_label).color(status_color).size(theme::TEXT_MD * scale),
        ]
        .spacing(theme::SPACE_MD)
        .align_y(iced::Alignment::Center),
        scale,
    );

    // View switch: a plain segmented toggle, not a chip — it's navigation,
    // not a status readout, so it shouldn't carry the same visual weight
    // as the identity chip. Both labels are always visible and clickable
    // (previously only the active view's name showed, with no click
    // target — Tab-key was the only way to switch).
    let tab_toggle = row![
        iced::widget::button(text("MIXER").size(theme::TEXT_MD * scale))
            .padding([theme::SPACE_SM * scale, theme::SPACE_XL * scale])
            .style(theme::tab_toggle(!state.show_matrix))
            .on_press(Message::TabPressed),
        iced::widget::button(text("MATRIX").size(theme::TEXT_MD * scale))
            .padding([theme::SPACE_SM * scale, theme::SPACE_XL * scale])
            .style(theme::tab_toggle(state.show_matrix))
            .on_press(Message::TabPressed),
    ]
    .spacing(theme::SPACE_TIGHT);

    // Secondary session tools: scene / submix / clock. These used to be
    // three separate chips carrying the same visual weight as the device
    // identity chip — merged into one quieter toolbar so the bar reads as
    // "one important thing, one toolbar" instead of five equal boxes.
    let scene_list = state.scene_list.clone();
    let session = chip(
        row![
            text("Scene").color(theme::TEXT_SEC).size(theme::TEXT_XS * scale),
            iced::widget::text_input("name", &state.scene_name)
                .on_input(Message::SceneNameChanged)
                .on_submit(Message::SceneSave)
                .style(theme::text_input)
                .width(Length::Fixed(90.0 * scale))
                .size(theme::TEXT_MD * scale),
            iced::widget::button(text("Save").size(theme::TEXT_MD * scale))
                .style(theme::plain_button)
                .on_press(Message::SceneSave),
            pick_list(scene_list, None::<String>, Message::SceneLoad)
                .placeholder("load...")
                .style(theme::pick_list)
                .menu_style(theme::menu)
                .text_size(theme::TEXT_MD * scale),
            v_divider(scale),
            text("Submix").color(theme::TEXT_SEC).size(theme::TEXT_XS * scale),
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
            .text_size(theme::TEXT_MD * scale),
            v_divider(scale),
            text(state.device.settings().clock_source.clone())
                .color(theme::TEXT_SEC)
                .size(theme::TEXT_XS * scale),
        ]
        .spacing(theme::SPACE_LG)
        .align_y(iced::Alignment::Center),
        scale,
    );

    let bar = row![
        text("TuxMix").color(theme::ACCENT).size(theme::TEXT_XL * scale),
        device_chip,
        tab_toggle,
        iced::widget::Space::new().width(Length::Fill),
        session,
    ]
    .spacing(theme::SPACE_XXL)
    .align_y(iced::Alignment::Center);

    container(bar)
        .style(theme::top_bar)
        .padding([theme::SPACE_LG * scale, theme::SPACE_XXL * scale])
        .width(Length::Fill)
        .into()
}

fn mixer_view(state: &TuxMix) -> Element<'_, Message> {
    let mut input_strips = row![].spacing(theme::SPACE_MD);
    let mut prev_type: Option<ChannelType> = None;
    for (i, ch) in state.device.inputs().iter().enumerate() {
        if prev_type.is_some_and(|t| t != ch.channel_type) {
            // `rule::vertical` hardcodes `height: Length::Fill` with no way
            // to override it — inside this row (itself `Length::Shrink`,
            // sized to its tallest strip), that Fill child was pulling the
            // *entire row* up to whatever space the window happened to
            // have, leaving a large empty gap below Hardware Inputs on any
            // window taller than its content. Wrapping it in a
            // `Length::Shrink` container stops the Fill from escaping
            // upward — it collapses to the container's own (content-sized)
            // height instead of the whole window's.
            input_strips = input_strips.push(
                container(iced::widget::rule::vertical(1)).height(Length::Shrink),
            );
        }
        prev_type = Some(ch.channel_type);

        let cid = ChannelId::Input(i);
        let meter = state
            .input_meters
            .get(i)
            .map(MeterAnim::frame)
            .unwrap_or_else(|| fader::MeterFrame::still(0.0));
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
            collapsed: state.collapsed.contains(&cid),
            collapse_anim: state.collapse_anim.get(&cid).copied(),
            scale: state.ui_scale,
            selected: state.selected.contains(&cid),
        }));
    }

    let mut pb_strips = row![].spacing(theme::SPACE_MD);
    for (i, ch) in state.device.playbacks().iter().enumerate() {
        let cid = ChannelId::Playback(i);
        let meter = state
            .playback_meters
            .get(i)
            .map(MeterAnim::frame)
            .unwrap_or_else(|| fader::MeterFrame::still(0.0));
        let drag_range = state
            .drag_range
            .and_then(|(dc, lo, hi)| (dc == cid).then_some((lo, hi)));

        pb_strips = pb_strips.push(strip::strip(strip::StripParams {
            cid,
            output_idx: state.sel_out,
            name: ch.name.clone(),
            type_tag: Some(("PB", PB_TAG)),
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
            collapsed: state.collapsed.contains(&cid),
            collapse_anim: state.collapse_anim.get(&cid).copied(),
            scale: state.ui_scale,
            selected: state.selected.contains(&cid),
        }));
    }

    let mut out_strips = row![].spacing(theme::SPACE_MD);
    for (i, ch) in state.device.outputs().iter().enumerate() {
        let cid = ChannelId::Output(i);
        let drag_range = state
            .drag_range
            .and_then(|(dc, lo, hi)| (dc == cid).then_some((lo, hi)));

        out_strips = out_strips.push(strip::strip(strip::StripParams {
            cid,
            output_idx: state.sel_out,
            name: ch.name.clone(),
            type_tag: Some(("OUT", OUT_TAG)),
            vol: ch.volume,
            pan: 0,
            meter: fader::MeterFrame::still(0.0),
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
            collapsed: state.collapsed.contains(&cid),
            collapse_anim: state.collapse_anim.get(&cid).copied(),
            scale: state.ui_scale,
            selected: state.selected.contains(&cid),
        }));
    }

    let scale = state.ui_scale;
    let body = column![
        section_header("HARDWARE INPUTS", scale),
        text(format!(
            "Submix: {} - Tab for Matrix",
            OUT_LABELS[state.sel_out]
        ))
        .color(theme::TEXT_SEC)
        .size(theme::TEXT_XS * scale),
        scrollable(input_strips)
            .direction(scrollable::Direction::Horizontal(
                theme::thin_scrollbar()
            ))
            .style(theme::scrollable),
        section_header("SOFTWARE PLAYBACK", scale),
        scrollable(pb_strips)
            .direction(scrollable::Direction::Horizontal(
                theme::thin_scrollbar()
            ))
            .style(theme::scrollable),
        section_header("HARDWARE OUTPUTS", scale),
        scrollable(out_strips)
            .direction(scrollable::Direction::Horizontal(
                theme::thin_scrollbar()
            ))
            .style(theme::scrollable),
    ]
    .spacing(theme::SPACE_LG)
    .width(Length::Fill);

    page(body)
}

fn matrix_view(state: &TuxMix) -> Element<'_, Message> {
    let scale = state.ui_scale;
    let body = column![
        section_header("MATRIX MIXER", scale),
        text("Volume per input per output - Tab to return")
            .color(theme::TEXT_SEC)
            .size(theme::TEXT_XS * scale),
        matrix::view(state),
    ]
    .spacing(theme::SPACE_LG)
    .width(Length::Fill);

    page(body)
}

#[cfg(test)]
mod tests {
    use super::MeterAnim;

    #[test]
    fn attack_rises_fast() {
        let mut m = MeterAnim::new();
        m.step(1.0);
        assert!(m.frame().value > 0.5, "one attack tick should jump most of the way: {}", m.frame().value);
    }

    #[test]
    fn release_decelerates_over_time() {
        let mut m = MeterAnim::new();
        m.step(1.0); // reach a peak first
        let peak = m.frame().value;

        m.step(0.0);
        let drop_1 = peak - m.frame().value;

        for _ in 0..10 {
            m.step(0.0);
        }
        let before_late = m.frame().value;
        m.step(0.0);
        let drop_late = before_late - m.frame().value;

        assert!(
            drop_1 > drop_late,
            "first release tick should fall faster than a tick late into the release: {drop_1} vs {drop_late}"
        );
    }

    #[test]
    fn rising_mid_release_cancels_it_and_resets_the_curve() {
        let mut m = MeterAnim::new();
        m.step(1.0);
        m.step(0.0);
        m.step(0.0);
        m.step(1.0); // new peak — release curve should restart from here
        assert_eq!(m.release_elapsed_ms, 0.0);
    }
}
