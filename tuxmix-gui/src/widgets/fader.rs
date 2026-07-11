//! Custom vertical fader + VU meter, drawn and driven entirely through a
//! [`canvas::Program`] — the one widget egui's built-in slider couldn't give
//! us (shift-drag fine range, scroll-wheel nudge, double-click reset).

use iced::keyboard::Modifiers;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};
use std::time::{Duration, Instant};

use crate::theme;

/// The meter and the dB ruler share one column — the meter is a
/// translucent color wash filling the whole column, and the ruler ticks
/// are drawn on top of it, like TotalMix — rather than two separate
/// side-by-side strips. The fader itself gets its own centered column.
const METER_RULER_W: f32 = 27.0;
const GAP: f32 = 5.0;
const TRACK_W: f32 = 24.0;
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
/// Shift-drag sensitivity reduction: cursor travel over the whole track
/// only moves the value by this fraction of what a normal drag would.
/// Earlier this remapped `range` to a narrow absolute value window instead
/// (cap tracking the cursor 1:1 inside a "zoomed" scale) — that made the
/// cap's screen position discontinuous the instant the drag ended and the
/// range snapped back to full, and moved the unity reference dot along
/// with it. A plain relative-delta scale-down has no such range to snap.
const FINE_SENSITIVITY: f32 = 0.15;

/// Fader travel is dB-tapered (like real hardware / TotalMix), not linear
/// amplitude — a power curve (t = 1 - x^K, x = db/FLOOR_DB) gives generous
/// resolution near unity and compresses the bottom into silence, without
/// the sharp knee a two-segment mapping produces (which crowded the
/// 0/-6/-10 ruler ticks together).
const FLOOR_DB: f32 = -60.0;
const TAPER_K: f32 = 0.6;

fn db_to_t(db: f32) -> f32 {
    let db = db.clamp(FLOOR_DB, 0.0);
    let x = (db / FLOOR_DB).clamp(0.0, 1.0);
    (1.0 - x.powf(TAPER_K)).clamp(0.0, 1.0)
}

fn t_to_db(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let x = (1.0 - t).powf(1.0 / TAPER_K);
    (x * FLOOR_DB).clamp(FLOOR_DB, 0.0)
}

fn vol_to_t(vol: f32) -> f32 {
    if vol <= 0.0 {
        0.0
    } else {
        db_to_t(20.0 * vol.log10())
    }
}

fn t_to_vol(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else {
        (10f32.powf(t_to_db(t) / 20.0)).clamp(0.0, 1.0)
    }
}

pub struct Fader<Message> {
    pub value: f32,
    pub range: (f32, f32),
    pub default_value: f32,
    pub meter: f32,
    pub height: f32,
    pub show_meter: bool,
    pub modifiers: Modifiers,
    pub on_press: Box<dyn Fn(f32, Option<(f32, f32)>) -> Message>,
    pub on_drag: Box<dyn Fn(f32) -> Message>,
    pub on_release: Box<dyn Fn() -> Message>,
    pub on_reset: Box<dyn Fn() -> Message>,
}

impl<Message> Fader<Message> {
    fn track_x(&self) -> f32 {
        if self.show_meter {
            METER_RULER_W + GAP
        } else {
            0.0
        }
    }
}

#[derive(Default)]
pub struct State {
    dragging: bool,
    last_click: Option<Instant>,
    /// Last cursor Y seen during an active drag. Each move applies the
    /// delta since this point (scaled by `FINE_SENSITIVITY` while Shift is
    /// held) rather than an absolute cursor-to-value mapping, and is
    /// re-read fresh every move — so Shift can be pressed or released
    /// mid-drag and the cap keeps going from wherever it already was,
    /// instead of jumping (which a fixed press-time anchor, or falling
    /// back to absolute cursor tracking, would both cause).
    drag_pos: Option<f32>,
}

impl<Message> canvas::Program<Message> for Fader<Message> {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let locate = |y: f32| -> f32 {
            let (lo, hi) = self.range;
            let screen_t = 1.0 - ((y - bounds.y) / bounds.height).clamp(0.0, 1.0);
            let (t_lo, t_hi) = (vol_to_t(lo), vol_to_t(hi));
            t_to_vol(t_lo + screen_t * (t_hi - t_lo))
        };

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pos = cursor.position_over(bounds)?;
                let now = Instant::now();
                let is_double = state
                    .last_click
                    .is_some_and(|t| now.duration_since(t) < DOUBLE_CLICK);

                if is_double {
                    state.dragging = false;
                    state.last_click = None;
                    state.drag_pos = None;
                    return Some(canvas::Action::publish((self.on_reset)()).and_capture());
                }
                state.last_click = Some(now);
                state.dragging = true;

                let value = locate(pos.y);
                state.drag_pos = Some(pos.y);
                Some(canvas::Action::publish((self.on_press)(value, None)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if !state.dragging {
                    return None;
                }
                let pos = cursor.land().position()?;
                let prev_y = state.drag_pos.unwrap_or(pos.y);
                let screen_dt = -(pos.y - prev_y) / bounds.height;
                let mult = if self.modifiers.shift() {
                    FINE_SENSITIVITY
                } else {
                    1.0
                };
                let t = (vol_to_t(self.value) + screen_dt * mult).clamp(0.0, 1.0);
                state.drag_pos = Some(pos.y);
                Some(canvas::Action::publish((self.on_drag)(t_to_vol(t))).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if !state.dragging {
                    return None;
                }
                state.dragging = false;
                state.drag_pos = None;
                Some(canvas::Action::publish((self.on_release)()).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(bounds) {
                    return None;
                }
                // Lines are discrete wheel detents (dy is usually ±1 per
                // click); Pixels are a continuous trackpad/high-res stream
                // (many small events per gesture) — they need very
                // different per-event magnitudes or one feels dead and the
                // other feels like it teleports. Stepping in tapered
                // t-space (not raw linear volume) also keeps the nudge feel
                // consistent across the whole dB range instead of vanishing
                // near unity, where a fixed amplitude step is only a
                // fraction of a dB.
                let (dy, base_step) = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => (*y, 0.03),
                    mouse::ScrollDelta::Pixels { y, .. } => (*y, 0.0015),
                };
                if dy == 0.0 {
                    return None;
                }
                let mult = if self.modifiers.shift() {
                    0.25
                } else if self.modifiers.control() {
                    3.0
                } else {
                    1.0
                };
                let t = vol_to_t(self.value);
                let new_t = (t + dy * base_step * mult).clamp(0.0, 1.0);
                let value = t_to_vol(new_t);
                Some(canvas::Action::publish((self.on_drag)(value)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if self.show_meter {
            let meter_rect =
                Rectangle::new(Point::ORIGIN, Size::new(METER_RULER_W, bounds.height));
            // Meter first, as a translucent wash; ruler ticks drawn on top
            // of it so both share the column instead of splitting the strip.
            draw_meter(&mut frame, meter_rect, self.meter);
            draw_ruler(&mut frame, meter_rect);
        }
        draw_track(
            &mut frame,
            Rectangle::new(
                Point::new(self.track_x(), 0.0),
                Size::new(TRACK_W, bounds.height),
            ),
            self.value,
            self.default_value,
            self.range,
            state.dragging,
        );
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.dragging {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::Idle
        }
    }
}

const METER_PILL_W: f32 = 6.0;
const METER_RADIUS: f32 = 3.0;
const CLIP_H: f32 = 5.0;
const CLIP_GAP: f32 = 3.0;

/// Linear interpolation between two colors — `t` is clamped to [0, 1].
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

/// Above this level the fill starts tinting toward red — a continuous
/// warning rather than a binary clip light, so the meter reads as "getting
/// hot" well before it actually clips.
const HOT_THRESHOLD: f32 = 0.85;

/// A slim rounded pill instead of a wide block — one calm color for the
/// signal, tinting progressively from green toward red above
/// `HOT_THRESHOLD`, plus a separate clip "LED" above the track that lights
/// up near 0 dBFS. Reads as a minimal modern level indicator, not a
/// traffic light, while still giving continuous feedback as it climbs.
fn draw_meter(frame: &mut Frame, r: Rectangle, level: f32) {
    let l = level.clamp(0.0, 1.0);

    let track = Rectangle::new(
        Point::new(r.x, r.y + CLIP_H + CLIP_GAP),
        Size::new(METER_PILL_W, r.height - CLIP_H - CLIP_GAP),
    );

    frame.fill(
        &Path::new(|b| b.rounded_rectangle(track.position(), track.size(), METER_RADIUS.into())),
        Color::from_rgb8(0x08, 0x08, 0x0a),
    );

    if l > 0.0 {
        let fill_h = track.height * l;
        let fill_pos = Point::new(track.x, track.y + track.height - fill_h);
        let hot_t = (l - HOT_THRESHOLD) / (1.0 - HOT_THRESHOLD);
        let fill_color = lerp_color(theme::MGREEN, theme::MRED, hot_t);
        frame.fill(
            &Path::new(|b| {
                b.rounded_rectangle(
                    fill_pos,
                    Size::new(METER_PILL_W, fill_h),
                    METER_RADIUS.into(),
                )
            }),
            fill_color,
        );
    }

    // Clip LED — a fixed indicator above the track, dim until triggered.
    let clip_rect = Rectangle::new(Point::new(r.x, r.y), Size::new(METER_PILL_W, CLIP_H));
    let clipping = l >= 0.95;
    let clip_color = if clipping {
        theme::MRED
    } else {
        Color::from_rgb8(0x3a, 0x16, 0x16)
    };
    if clipping {
        let glow = Rectangle::new(
            Point::new(r.x - 2.0, r.y - 2.0),
            Size::new(METER_PILL_W + 4.0, CLIP_H + 4.0),
        );
        frame.fill(
            &Path::new(|b| {
                b.rounded_rectangle(glow.position(), glow.size(), (CLIP_H / 2.0 + 2.0).into())
            }),
            Color { a: 0.35, ..theme::MRED },
        );
    }
    frame.fill(
        &Path::new(|b| {
            b.rounded_rectangle(clip_rect.position(), clip_rect.size(), (CLIP_H / 2.0).into())
        }),
        clip_color,
    );
}

const RAIL_W: f32 = 3.0;
const CAP_W: f32 = 20.0;
const CAP_H: f32 = 13.0;
const CAP_RADIUS: f32 = 2.5;
const REF_R: f32 = 2.5;

fn draw_track(
    frame: &mut Frame,
    r: Rectangle,
    value: f32,
    default_value: f32,
    range: (f32, f32),
    dragging: bool,
) {
    let (lo, hi) = range;
    let (t_lo, t_hi) = (vol_to_t(lo), vol_to_t(hi));

    let pos_of = |v: f32| -> f32 {
        let t = if t_hi > t_lo {
            ((vol_to_t(v) - t_lo) / (t_hi - t_lo)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        r.y + r.height - r.height * t
    };

    let cx = r.x + r.width / 2.0;
    let half_cap = CAP_H / 2.0;
    let raw_y = pos_of(value);
    let cap_y = raw_y.clamp(r.y + half_cap, r.y + r.height - half_cap);

    // Groove — the full-length rail, unfilled color.
    let groove = Path::rectangle(
        Point::new(cx - RAIL_W / 2.0, r.y),
        Size::new(RAIL_W, r.height),
    );
    frame.fill(&groove, theme::BORDER);

    // Filled portion of the rail, from the bottom up to the cap.
    if raw_y < r.y + r.height {
        let fill = Path::rectangle(
            Point::new(cx - RAIL_W / 2.0, cap_y),
            Size::new(RAIL_W, r.y + r.height - cap_y),
        );
        frame.fill(&fill, theme::FADER);
    }

    // Unity/default reference — a small hollow ring on the rail, like the
    // 0 dB mark on a real console strip.
    let ref_y = pos_of(default_value).clamp(r.y + REF_R, r.y + r.height - REF_R);
    frame.stroke(
        &Path::circle(Point::new(cx, ref_y), REF_R),
        Stroke::default().with_color(theme::TEXT_SEC).with_width(1.0),
    );

    // Cap — flat + ridged, like a real fader cap grip, no heavy bevel.
    let cap_left = cx - CAP_W / 2.0;
    let cap_top = cap_y - half_cap;

    if dragging {
        let glow = Path::rectangle(
            Point::new(cap_left - 3.0, cap_top - 3.0),
            Size::new(CAP_W + 6.0, CAP_H + 6.0),
        );
        frame.fill(&glow, Color { a: 0.18, ..theme::FADER });
    }

    let body = Path::new(|b| {
        b.rounded_rectangle(
            Point::new(cap_left, cap_top),
            Size::new(CAP_W, CAP_H),
            CAP_RADIUS.into(),
        )
    });
    let body_color = if dragging {
        Color::from_rgb8(0xf0, 0xf1, 0xf4)
    } else {
        theme::FADER
    };
    frame.fill(&body, body_color);

    // Ridge lines — the grip texture.
    for i in 1..=3 {
        let y = cap_top + (CAP_H / 4.0) * i as f32;
        let ridge = Path::rectangle(
            Point::new(cap_left + 3.0, y - 0.5),
            Size::new(CAP_W - 6.0, 1.0),
        );
        frame.fill(&ridge, Color::from_rgba8(0x00, 0x00, 0x00, 0.25));
    }

    frame.stroke(
        &body,
        Stroke::default().with_color(theme::BORDER).with_width(1.0),
    );
}

/// Drawn on top of the (translucent) meter wash, so it needs to stay
/// legible against whatever color is lit behind it — brighter than the
/// usual secondary text color, with a tick + number per breakpoint.
fn draw_ruler(frame: &mut Frame, r: Rectangle) {
    const TICKS: [f32; 6] = [0.0, -6.0, -10.0, -20.0, -40.0, -60.0];
    let label_color = theme::TEXT_SEC;
    let x0 = r.x + METER_PILL_W + 4.0;

    for db in TICKS {
        let t = db_to_t(db);
        let y = r.y + r.height - r.height * t;
        let y = y.clamp(r.y + 4.0, r.y + r.height - 4.0);

        let tick = Path::line(Point::new(x0, y), Point::new(x0 + 3.0, y));
        frame.stroke(
            &tick,
            Stroke::default().with_color(label_color).with_width(1.0),
        );

        let label = if db == 0.0 {
            "0".to_string()
        } else {
            format!("{}", -db as i32)
        };
        frame.fill_text(canvas::Text {
            content: label,
            position: Point::new(x0 + 5.0, y - 4.0),
            color: label_color,
            size: theme::TEXT_MICRO.into(),
            ..canvas::Text::default()
        });
    }
}

pub fn fader<'a, Message: 'a>(fader: Fader<Message>) -> Element<'a, Message>
where
    Message: Clone,
{
    let height = fader.height;
    let width = if fader.show_meter {
        METER_RULER_W + GAP + TRACK_W
    } else {
        TRACK_W
    };
    Canvas::new(fader)
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .into()
}

/// The meter+ruler column on its own, with no track/cap and no mouse
/// interaction at all — for a collapsed strip, which trades every control
/// (fader, mute/solo, pan) for a glance-only level readout.
struct VuMeter {
    level: f32,
}

impl<Message> canvas::Program<Message> for VuMeter {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let meter_rect = Rectangle::new(Point::ORIGIN, Size::new(METER_RULER_W, bounds.height));
        draw_meter(&mut frame, meter_rect, self.level);
        draw_ruler(&mut frame, meter_rect);
        vec![frame.into_geometry()]
    }
}

pub fn vu_meter<'a, Message: 'a>(level: f32, height: f32) -> Element<'a, Message> {
    Canvas::new(VuMeter { level })
        .width(Length::Fixed(METER_RULER_W))
        .height(Length::Fixed(height))
        .into()
}

const PAN_W: f32 = 44.0;
const PAN_H: f32 = 12.0;
const PAN_DOT_R: f32 = 2.75;

/// A groove with a dot marking where the pan sits — click/drag horizontally
/// to set it, shift-drag for a fine (reduced-sensitivity) adjustment,
/// scroll-wheel to nudge, double-click to reset to center. Mirrors the
/// fader's own press/drag/wheel/reset interaction, just on the x axis and
/// over a linear (not dB-tapered) range.
pub struct PanIndicator<Message> {
    pub pan: i8,
    pub modifiers: Modifiers,
    pub on_change: Box<dyn Fn(i8) -> Message>,
    pub on_reset: Box<dyn Fn() -> Message>,
}

#[derive(Default)]
pub struct PanState {
    dragging: bool,
    last_click: Option<Instant>,
    /// Last cursor X seen during an active drag — see `Fader::State::drag_pos`
    /// for why this is re-read fresh every move instead of a fixed anchor.
    drag_pos: Option<f32>,
}

fn pan_usable_width(bounds_width: f32) -> f32 {
    bounds_width / 2.0 - PAN_DOT_R - 2.0
}

fn pan_to_t(pan: i8) -> f32 {
    (pan as f32 / 100.0).clamp(-1.0, 1.0)
}

fn t_to_pan(t: f32) -> i8 {
    (t.clamp(-1.0, 1.0) * 100.0).round() as i8
}

fn locate_pan(x: f32, bounds: Rectangle) -> i8 {
    let cx = bounds.x + bounds.width / 2.0;
    let usable = pan_usable_width(bounds.width);
    let t = (x - cx) / usable;
    t_to_pan(t)
}

impl<Message> canvas::Program<Message> for PanIndicator<Message> {
    type State = PanState;

    fn update(
        &self,
        state: &mut PanState,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pos = cursor.position_over(bounds)?;
                let now = Instant::now();
                let is_double = state
                    .last_click
                    .is_some_and(|t| now.duration_since(t) < DOUBLE_CLICK);

                if is_double {
                    state.dragging = false;
                    state.last_click = None;
                    state.drag_pos = None;
                    return Some(canvas::Action::publish((self.on_reset)()).and_capture());
                }
                state.last_click = Some(now);
                state.dragging = true;

                let pan = locate_pan(pos.x, bounds);
                state.drag_pos = Some(pos.x);
                Some(canvas::Action::publish((self.on_change)(pan)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if !state.dragging {
                    return None;
                }
                let pos = cursor.land().position()?;
                let prev_x = state.drag_pos.unwrap_or(pos.x);
                let screen_dt = (pos.x - prev_x) / bounds.width;
                let mult = if self.modifiers.shift() {
                    FINE_SENSITIVITY
                } else {
                    1.0
                };
                let t = pan_to_t(self.pan) + screen_dt * mult;
                state.drag_pos = Some(pos.x);
                Some(canvas::Action::publish((self.on_change)(t_to_pan(t))).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if !state.dragging {
                    return None;
                }
                state.dragging = false;
                state.drag_pos = None;
                None
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(bounds) {
                    return None;
                }
                let (dy, base_step) = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => (*y, 0.03),
                    mouse::ScrollDelta::Pixels { y, .. } => (*y, 0.0015),
                };
                if dy == 0.0 {
                    return None;
                }
                let mult = if self.modifiers.shift() {
                    0.25
                } else if self.modifiers.control() {
                    3.0
                } else {
                    1.0
                };
                let t = pan_to_t(self.pan) + dy * base_step * mult;
                Some(canvas::Action::publish((self.on_change)(t_to_pan(t))).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &PanState,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let cy = bounds.height / 2.0;
        let cx = bounds.width / 2.0;

        let groove = Path::line(Point::new(2.0, cy), Point::new(bounds.width - 2.0, cy));
        frame.stroke(
            &groove,
            Stroke::default().with_color(theme::BORDER).with_width(1.5),
        );

        let tick = Path::line(Point::new(cx, cy - 3.0), Point::new(cx, cy + 3.0));
        frame.stroke(
            &tick,
            Stroke::default().with_color(theme::TEXT_SEC).with_width(1.0),
        );

        let t = (self.pan as f32 / 100.0).clamp(-1.0, 1.0);
        let usable = pan_usable_width(bounds.width);
        let dot_x = cx + t * usable;
        let dot_color = if state.dragging {
            Color::from_rgb8(0xf0, 0xf1, 0xf4)
        } else {
            theme::ACCENT
        };
        frame.fill(&Path::circle(Point::new(dot_x, cy), PAN_DOT_R), dot_color);

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &PanState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.dragging {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::Idle
        }
    }
}

pub fn pan_indicator<'a, Message: 'a>(pan: PanIndicator<Message>) -> Element<'a, Message>
where
    Message: Clone,
{
    Canvas::new(pan)
        .width(Length::Fixed(PAN_W))
        .height(Length::Fixed(PAN_H))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_stays_green_below_hot_threshold() {
        let c = lerp_color(theme::MGREEN, theme::MRED, (0.5 - HOT_THRESHOLD) / (1.0 - HOT_THRESHOLD));
        assert_eq!(c, theme::MGREEN);
    }

    #[test]
    fn fill_turns_fully_red_at_full_level() {
        let hot_t = (1.0 - HOT_THRESHOLD) / (1.0 - HOT_THRESHOLD);
        let c = lerp_color(theme::MGREEN, theme::MRED, hot_t);
        assert!((c.r - theme::MRED.r).abs() < 1e-4);
        assert!((c.g - theme::MRED.g).abs() < 1e-4);
        assert!((c.b - theme::MRED.b).abs() < 1e-4);
    }

    #[test]
    fn fill_is_between_green_and_red_mid_hot_zone() {
        let hot_t = ((HOT_THRESHOLD + 1.0) / 2.0 - HOT_THRESHOLD) / (1.0 - HOT_THRESHOLD);
        let c = lerp_color(theme::MGREEN, theme::MRED, hot_t);
        assert!(c.r > theme::MGREEN.r && c.r < theme::MRED.r);
        assert!(c.g < theme::MGREEN.g && c.g > theme::MRED.g);
    }
}
