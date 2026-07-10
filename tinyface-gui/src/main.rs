//! `tinyface-gui` — Graphical RME interface controller.
//!
//! ```bash
//! cargo run -p tinyface-gui              # with real hardware
//! cargo run -p tinyface-gui -- --mock    # simulation
//! ```

use eframe::egui;
use egui::{Color32, Frame, Margin, Rounding, Vec2};
use tinyface_core::{BabyfacePro, ChannelId, MockBabyfacePro, RmeDevice};

// ── Colors ───────────────────────────────────────────────────────

const BG_DEEP: Color32 = Color32::from_rgb(0x0d, 0x0d, 0x0d);
const SURFACE: Color32 = Color32::from_rgb(0x18, 0x18, 0x1a);
const BORDER: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x30);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xe8, 0xe8, 0xec);
const TEXT_SEC: Color32 = Color32::from_rgb(0x88, 0x88, 0x94);
const ACCENT: Color32 = Color32::from_rgb(0x4f, 0xc3, 0xf7);
const ACCENT_DIM: Color32 = Color32::from_rgb(0x2a, 0x6a, 0x88);
const MGREEN: Color32 = Color32::from_rgb(0x4c, 0xaf, 0x50);
const MYELLOW: Color32 = Color32::from_rgb(0xff, 0xeb, 0x3b);
const MRED: Color32 = Color32::from_rgb(0xf4, 0x43, 0x36);
const PHANTOM: Color32 = Color32::from_rgb(0xff, 0x45, 0x45);
const GCONN: Color32 = Color32::from_rgb(0x4c, 0xaf, 0x50);
const YSIM: Color32 = Color32::from_rgb(0xff, 0xc1, 0x07);

// ── Constants ────────────────────────────────────────────────────

const OUT_LABELS: [&str; 6] = ["AN1/2", "PH3/4", "AS1/2", "A3/A4", "A5/A6", "A7/A8"];

fn short_label(name: &str) -> &str {
    name.strip_prefix("PCM ").unwrap_or(name)
}

fn type_tag(t: tinyface_core::ChannelType) -> (&'static str, Color32) {
    use tinyface_core::ChannelType::*;
    match t {
        Mic => ("MIC", Color32::from_rgb(0xff, 0x6b, 0x6b)),
        Instrument => ("INST", Color32::from_rgb(0xff, 0xb7, 0x4d)),
        Line => ("LINE", ACCENT),
        SPDIF => ("SPDIF", Color32::from_rgb(0xba, 0x68, 0xc8)),
        ADAT => ("ADAT", Color32::from_rgb(0xba, 0x68, 0xc8)),
    }
}

fn parse_db_input(s: &str) -> Option<f32> {
    let raw = s.trim().to_lowercase();
    if raw.is_empty() || raw == "-inf" || raw == "-\u{221e}" {
        return Some(0.0);
    }
    raw.replace(',', ".")
        .parse::<f32>()
        .ok()
        .map(|db| (10f32.powf(db / 20.0)).clamp(0.0, 1.0))
}

#[derive(Default)]
struct StripAction {
    vol: Option<f32>,
    phantom: Option<bool>,
    pad: Option<bool>,
    edit: EditAction,
    set_drag_range: Option<(f32, f32)>,
    clear_drag_range: bool,
}

#[derive(Default, PartialEq)]
enum EditAction {
    #[default]
    None,
    Start,
    Commit,
    Cancel,
}

// ── Device enum ──────────────────────────────────────────────────

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

// ── Main ─────────────────────────────────────────────────────────

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    let mock = std::env::args().any(|a| a == "--mock");
    let device = if mock {
        DeviceHandle::open_mock()
    } else {
        DeviceHandle::open_real().unwrap_or_else(|| {
            eprintln!("No device found. Use --mock for simulation.");
            DeviceHandle::open_mock()
        })
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Tinyface — RME Mixer")
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Tinyface",
        options,
        Box::new(|cc| {
            let mut v = egui::Visuals::dark();
            v.override_text_color = Some(TEXT_PRIMARY);
            v.panel_fill = BG_DEEP;
            v.window_fill = BG_DEEP;
            v.faint_bg_color = SURFACE;
            v.extreme_bg_color = SURFACE;
            v.hyperlink_color = ACCENT;
            v.selection.bg_fill = ACCENT_DIM;
            v.selection.stroke = egui::Stroke::new(1.0, ACCENT);
            cc.egui_ctx.set_visuals(v);
            let mut s = (*cc.egui_ctx.style()).clone();
            s.spacing.item_spacing = Vec2::new(8.0, 6.0);
            s.spacing.window_margin = Margin::symmetric(12.0, 8.0);
            cc.egui_ctx.set_style(s);
            Ok(Box::new(TinyFaceApp::new(device)))
        }),
    )
}

// ── App ──────────────────────────────────────────────────────────

struct TinyFaceApp {
    device: DeviceHandle,
    sel_out: usize,
    show_matrix: bool,
    phantom: std::collections::HashMap<usize, bool>,
    pad: std::collections::HashMap<usize, bool>,
    editing: Option<ChannelId>,
    edit_buf: String,
    drag_range: Option<(ChannelId, f32, f32)>,
}

impl TinyFaceApp {
    fn new(device: DeviceHandle) -> Self {
        Self {
            device,
            sel_out: 0,
            show_matrix: false,
            phantom: std::collections::HashMap::new(),
            pad: std::collections::HashMap::new(),
            editing: None,
            edit_buf: String::new(),
            drag_range: None,
        }
    }

    fn draw_vu(p: &egui::Painter, r: egui::Rect, level: f32) {
        let l = level.clamp(0.0, 1.0);
        p.rect_filled(r, 2.0, Color32::from_rgb(0x08, 0x08, 0x0a));
        let ticks = [
            (
                -6.0_f32,
                Color32::from_rgba_premultiplied(200, 200, 180, 30),
            ),
            (-12.0, Color32::from_rgba_premultiplied(200, 200, 180, 22)),
            (-24.0, Color32::from_rgba_premultiplied(200, 200, 180, 15)),
        ];
        if l > 0.0 {
            let fill = egui::Rect::from_min_size(
                egui::pos2(r.left(), r.bottom() - r.height() * l),
                egui::vec2(r.width(), r.height() * l),
            );
            let c = if l < 0.6 {
                MGREEN
            } else if l < 0.85 {
                MYELLOW
            } else {
                MRED
            };
            let ga = (0.08 + l * 0.18).clamp(0.0, 0.35);
            let gc = Color32::from_rgba_premultiplied(c.r(), c.g(), c.b(), (ga * 255.0) as u8);
            p.rect_filled(fill.expand2(Vec2::new(4.0, 0.0)), 2.0, gc);
            p.rect_filled(fill, 2.0, c);
            if l >= 0.95 {
                let dot = egui::Rect::from_min_size(
                    egui::pos2(fill.left(), fill.top() - 2.0),
                    egui::vec2(fill.width(), 3.0),
                );
                p.rect_filled(dot, 1.0, MRED);
            }
        }
        for (db, color) in &ticks {
            let frac = 10f32.powf(db / 20.0);
            let y = r.bottom() - r.height() * frac;
            p.line_segment(
                [egui::pos2(r.left(), y), egui::pos2(r.right(), y)],
                egui::Stroke::new(1.0, *color),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn strip(
        ui: &mut egui::Ui,
        name: &str,
        type_tag: Option<(&str, Color32)>,
        vol: f32,
        pan: i8,
        meter: f32,
        has_48v: bool,
        has_pad: bool,
        ph: bool,
        pd: bool,
        editing: bool,
        edit_buf: &mut String,
        drag_range: Option<(f32, f32)>,
    ) -> StripAction {
        let mut action = StripAction::default();
        let fader_h = 120.0;

        Frame::none()
            .fill(SURFACE)
            .rounding(Rounding::same(7.0))
            .stroke(egui::Stroke::new(1.0, BORDER))
            .inner_margin(Margin::symmetric(6.0, 4.0))
            .show(ui, |ui| {
                ui.set_min_width(70.0);
                ui.set_max_width(86.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 2.0);

                    // Name + type tag
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                        ui.label(egui::RichText::new(short_label(name)).size(11.0).strong());
                        if let Some((tag, color)) = type_tag {
                            ui.label(egui::RichText::new(tag).color(color).size(9.0).strong());
                        }
                    });

                    // 48V / PAD toggles
                    if has_48v || has_pad {
                        let w = if has_48v && has_pad { 35.0 } else { 72.0 };
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                            if has_48v {
                                let p = ph;
                                let btn = egui::Button::new(
                                    egui::RichText::new("48V").size(10.0).strong().color(if p {
                                        Color32::from_rgb(0x1a, 0x08, 0x08)
                                    } else {
                                        TEXT_SEC
                                    }),
                                )
                                .fill(if p { PHANTOM } else { BG_DEEP })
                                .stroke(egui::Stroke::new(1.0, if p { PHANTOM } else { BORDER }));
                                if ui.add_sized([w, 18.0], btn).clicked() {
                                    action.phantom = Some(!p);
                                }
                            }
                            if has_pad {
                                let p = pd;
                                let btn = egui::Button::new(
                                    egui::RichText::new("PAD").size(10.0).strong().color(if p {
                                        BG_DEEP
                                    } else {
                                        TEXT_SEC
                                    }),
                                )
                                .fill(if p { ACCENT } else { BG_DEEP })
                                .stroke(egui::Stroke::new(1.0, if p { ACCENT } else { BORDER }));
                                if ui.add_sized([w, 18.0], btn).clicked() {
                                    action.pad = Some(!p);
                                }
                            }
                        });
                    }

                    // VU meter + Slider (fills remaining space)
                    let (lo, hi) = drag_range.unwrap_or((0.0, 1.0));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                        let mr =
                            ui.allocate_exact_size(Vec2::new(10.0, fader_h), egui::Sense::hover());
                        Self::draw_vu(ui.painter(), mr.0, meter);
                        let mut v = vol.clamp(lo, hi);
                        let resp = ui
                            .add_sized(
                                [24.0, fader_h],
                                egui::Slider::new(&mut v, lo..=hi)
                                    .vertical()
                                    .show_value(false),
                            )
                            .on_hover_text(db_text(vol));
                        if resp.changed() {
                            action.vol = Some(v);
                        }
                        if resp.drag_started() && ui.input(|i| i.modifiers.shift) {
                            const SPAN: f32 = 0.08;
                            let mut nl = (v - SPAN / 2.0).max(0.0);
                            let mut nh = nl + SPAN;
                            if nh > 1.0 {
                                nh = 1.0;
                                nl = nh - SPAN;
                            }
                            action.set_drag_range = Some((nl.max(0.0), nh));
                        }
                        if resp.drag_stopped() {
                            action.clear_drag_range = true;
                        }
                        if resp.hovered() {
                            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                            if scroll != 0.0 {
                                let step = if ui.input(|i| i.modifiers.shift) {
                                    0.0002
                                } else if ui.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                                    0.004
                                } else {
                                    0.001
                                };
                                action.vol = Some((vol + scroll * step).clamp(lo, hi));
                            }
                        }
                    });

                    // dB label
                    let db = db_text(vol);
                    let edit_id = egui::Id::new((name, "dbedit"));
                    if editing {
                        let resp = ui.add(
                            egui::TextEdit::singleline(edit_buf)
                                .id(edit_id)
                                .font(egui::FontId::proportional(10.0))
                                .desired_width(64.0),
                        );
                        if resp.lost_focus() {
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                action.edit = EditAction::Cancel;
                            } else {
                                action.edit = EditAction::Commit;
                            }
                        }
                    } else {
                        let lbl = ui.add(
                            egui::Label::new(egui::RichText::new(&db).color(TEXT_SEC).size(10.0))
                                .sense(egui::Sense::click()),
                        );
                        if lbl.double_clicked() {
                            *edit_buf = if vol > 0.0 {
                                format!("{:.1}", 20.0 * vol.log10())
                            } else {
                                "-inf".into()
                            };
                            ui.memory_mut(|m| m.request_focus(edit_id));
                            action.edit = EditAction::Start;
                        }
                    }

                    if pan != 0 {
                        let p = if pan < 0 {
                            format!("L{}", -pan)
                        } else {
                            format!("R{}", pan)
                        };
                        ui.label(egui::RichText::new(p).color(ACCENT).size(9.0));
                    }
                });
            });
        action
    }

    fn draw_matrix(&mut self, ui: &mut egui::Ui) {
        let ni = self.device.inputs().len();
        let np = self.device.playbacks().len();
        let total = ni + np;
        Frame::none()
            .fill(SURFACE)
            .rounding(Rounding::same(6.0))
            .stroke(egui::Stroke::new(1.0, BORDER))
            .inner_margin(Margin::symmetric(8.0, 8.0))
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("matrix")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(2.0, 2.0);
                            ui.vertical(|ui| {
                                ui.add_space(14.0);
                                for row in 0..6 {
                                    let a = row == self.sel_out;
                                    let mut rt = egui::RichText::new(OUT_LABELS[row])
                                        .color(if a { ACCENT } else { TEXT_SEC })
                                        .size(9.0);
                                    if a {
                                        rt = rt.strong();
                                    }
                                    ui.label(rt);
                                }
                            });
                            for col in 0..total {
                                ui.vertical(|ui| {
                                    let (n, is_in) = if col < ni {
                                        (self.device.inputs()[col].name.clone(), true)
                                    } else {
                                        (self.device.playbacks()[col - ni].name.clone(), false)
                                    };
                                    ui.label(egui::RichText::new(n).color(TEXT_SEC).size(9.0));
                                    for row in 0..6 {
                                        let v = if is_in {
                                            self.device.inputs()[col].volumes[row]
                                        } else {
                                            self.device.playbacks()[col - ni].volumes[row]
                                        };
                                        let mut mv = v;
                                        let r = ui.add_sized(
                                            [14.0, 28.0],
                                            egui::Slider::new(&mut mv, 0.0..=1.0)
                                                .vertical()
                                                .show_value(false),
                                        );
                                        if r.changed() {
                                            if is_in {
                                                let _ = self.device.set_volume(
                                                    ChannelId::Input(col),
                                                    row,
                                                    mv,
                                                );
                                            } else {
                                                let _ = self.device.set_volume(
                                                    ChannelId::Playback(col - ni),
                                                    row,
                                                    mv,
                                                );
                                            }
                                        }
                                    }
                                });
                            }
                        });
                    });
            });
    }
}

fn db_text(vol: f32) -> String {
    if vol > 0.0 {
        format!("{:.1} dB", 20.0 * vol.log10())
    } else {
        "-\u{221e} dB".into()
    }
}

impl eframe::App for TinyFaceApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let _ = self.device.poll_events();
        ctx.request_repaint_after(std::time::Duration::from_millis(50));

        if ctx.input_mut(|i| i.key_pressed(egui::Key::Tab)) {
            self.show_matrix = !self.show_matrix;
        }

        egui::TopBottomPanel::top("top")
            .frame(
                Frame::none()
                    .fill(SURFACE)
                    .inner_margin(Margin::symmetric(16.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("Tinyface")
                            .color(ACCENT)
                            .size(20.0)
                            .strong(),
                    );
                    ui.separator();
                    if self.device.is_mock() {
                        ui.label(egui::RichText::new(self.device.model_name()).size(14.0));
                        ui.colored_label(YSIM, "● Simulated");
                    } else {
                        ui.label(egui::RichText::new(self.device.model_name()).size(14.0));
                        ui.colored_label(GCONN, "● Connected");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ml = if self.show_matrix {
                            egui::RichText::new("[Tab: Matrix]")
                                .color(ACCENT)
                                .size(12.0)
                                .strong()
                        } else {
                            egui::RichText::new("[Tab: Mixer]")
                                .color(TEXT_SEC)
                                .size(12.0)
                        };
                        ui.label(ml);
                        ui.label("Submix:");
                        egui::ComboBox::from_id_salt("out_sel")
                            .selected_text(OUT_LABELS[self.sel_out])
                            .show_ui(ui, |ui| {
                                for (i, n) in OUT_LABELS.iter().enumerate() {
                                    ui.selectable_value(&mut self.sel_out, i, *n);
                                }
                            });
                        ui.label(
                            egui::RichText::new(format!(
                                "⏱ {}",
                                self.device.settings().clock_source
                            ))
                            .color(TEXT_SEC)
                            .size(12.0),
                        );
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(BG_DEEP)
                    .inner_margin(Margin::symmetric(12.0, 8.0)),
            )
            .show(ctx, |ui| {
                if self.show_matrix {
                    ui.label(
                        egui::RichText::new("MATRIX MIXER")
                            .color(TEXT_SEC)
                            .size(11.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Volume de chaque entrée vers chaque sortie — Tab pour retour",
                        )
                        .color(TEXT_SEC)
                        .size(10.0),
                    );
                    self.draw_matrix(ui);
                } else {
                    ui.label(
                        egui::RichText::new("HARDWARE INPUTS")
                            .color(TEXT_SEC)
                            .size(11.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "Submix: {} — Tab pour Matrix",
                            OUT_LABELS[self.sel_out]
                        ))
                        .color(TEXT_SEC)
                        .size(10.0),
                    );

                    egui::ScrollArea::horizontal().id_salt("in").show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut pt: Option<tinyface_core::ChannelType> = None;
                            for i in 0..self.device.inputs().len() {
                                let ch = self.device.inputs()[i].clone();
                                if pt.is_some_and(|t| t != ch.channel_type) {
                                    ui.separator();
                                }
                                pt = Some(ch.channel_type);
                                let m = self.device.input_meter(i);
                                let vol = ch.volumes[self.sel_out];
                                let pan = ch.pans[self.sel_out];
                                let h48 = ch.channel_type == tinyface_core::ChannelType::Mic;
                                let hp = ch.channel_type == tinyface_core::ChannelType::Mic;
                                let ph = *self.phantom.get(&i).unwrap_or(&ch.phantom);
                                let pd = *self.pad.get(&i).unwrap_or(&ch.pad);
                                let cid = ChannelId::Input(i);
                                let ed = self.editing == Some(cid);
                                let dr = self
                                    .drag_range
                                    .and_then(|(dc, lo, hi)| (dc == cid).then_some((lo, hi)));
                                let r = Self::strip(
                                    ui,
                                    &ch.name,
                                    Some(type_tag(ch.channel_type)),
                                    vol,
                                    pan,
                                    m,
                                    h48,
                                    hp,
                                    ph,
                                    pd,
                                    ed,
                                    &mut self.edit_buf,
                                    dr,
                                );
                                if let Some(v) = r.vol {
                                    let _ = self.device.set_volume(cid, self.sel_out, v);
                                }
                                if let Some(p) = r.phantom {
                                    self.phantom.insert(i, p);
                                }
                                if let Some(p) = r.pad {
                                    self.pad.insert(i, p);
                                }
                                if let Some(rl) = r.set_drag_range {
                                    self.drag_range = Some((cid, rl.0, rl.1));
                                }
                                if r.clear_drag_range {
                                    self.drag_range = None;
                                }
                                match r.edit {
                                    EditAction::Start => self.editing = Some(cid),
                                    EditAction::Commit => {
                                        if let Some(v) = parse_db_input(&self.edit_buf) {
                                            let _ = self.device.set_volume(cid, self.sel_out, v);
                                        }
                                        self.editing = None;
                                    }
                                    EditAction::Cancel => self.editing = None,
                                    EditAction::None => {}
                                }
                            }
                        });
                    });

                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("SOFTWARE PLAYBACK")
                            .color(TEXT_SEC)
                            .size(11.0)
                            .strong(),
                    );
                    egui::ScrollArea::horizontal().id_salt("pb").show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for i in 0..self.device.playbacks().len() {
                                let ch = self.device.playbacks()[i].clone();
                                let m = self.device.playback_meter(i);
                                let vol = ch.volumes[self.sel_out];
                                let pan = ch.pans[self.sel_out];
                                let cid = ChannelId::Playback(i);
                                let ed = self.editing == Some(cid);
                                let dr = self
                                    .drag_range
                                    .and_then(|(dc, lo, hi)| (dc == cid).then_some((lo, hi)));
                                let r = Self::strip(
                                    ui,
                                    &ch.name,
                                    Some(("PB", TEXT_SEC)),
                                    vol,
                                    pan,
                                    m,
                                    false,
                                    false,
                                    false,
                                    false,
                                    ed,
                                    &mut self.edit_buf,
                                    dr,
                                );
                                if let Some(v) = r.vol {
                                    let _ = self.device.set_volume(cid, self.sel_out, v);
                                }
                                if let Some(rl) = r.set_drag_range {
                                    self.drag_range = Some((cid, rl.0, rl.1));
                                }
                                if r.clear_drag_range {
                                    self.drag_range = None;
                                }
                                match r.edit {
                                    EditAction::Start => self.editing = Some(cid),
                                    EditAction::Commit => {
                                        if let Some(v) = parse_db_input(&self.edit_buf) {
                                            let _ = self.device.set_volume(cid, self.sel_out, v);
                                        }
                                        self.editing = None;
                                    }
                                    EditAction::Cancel => self.editing = None,
                                    EditAction::None => {}
                                }
                            }
                        });
                    });

                    ui.add_space(10.0);
                    Frame::none()
                        .fill(SURFACE)
                        .rounding(Rounding::same(6.0))
                        .stroke(egui::Stroke::new(1.0, BORDER))
                        .inner_margin(Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("SCENES")
                                        .color(TEXT_SEC)
                                        .size(11.0)
                                        .strong(),
                                );
                                if ui.button("📸 Capture").clicked() {
                                    let s = self.device.capture_scene();
                                    if let Ok(j) = s.to_json() {
                                        log::info!("Scene:\n{}", j);
                                    }
                                }
                                if ui.button("💾 Save...").clicked() {}
                                if ui.button("📂 Load...").clicked() {}
                            });
                        });
                }
            });
    }
}
