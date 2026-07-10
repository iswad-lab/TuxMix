//! Custom vertical fader + VU meter, drawn and driven entirely through a
//! [`canvas::Program`] — the one widget egui's built-in slider couldn't give
//! us (shift-drag fine range, scroll-wheel nudge, double-click reset).

use iced::keyboard::Modifiers;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};
use std::time::{Duration, Instant};

use crate::theme;

const METER_W: f32 = 10.0;
const GAP: f32 = 3.0;
const TRACK_W: f32 = 24.0;
const RULER_W: f32 = 15.0;
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
const FINE_SPAN: f32 = 0.08;

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
            METER_W + GAP
        } else {
            0.0
        }
    }
}

#[derive(Default)]
pub struct State {
    dragging: bool,
    last_click: Option<Instant>,
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
                    return Some(canvas::Action::publish((self.on_reset)()).and_capture());
                }
                state.last_click = Some(now);
                state.dragging = true;

                let value = locate(pos.y);
                let fine_range = if self.modifiers.shift() {
                    let span = FINE_SPAN;
                    let mut lo = (value - span / 2.0).max(0.0);
                    let mut hi = lo + span;
                    if hi > 1.0 {
                        hi = 1.0;
                        lo = hi - span;
                    }
                    Some((lo.max(0.0), hi))
                } else {
                    None
                };
                Some(canvas::Action::publish((self.on_press)(value, fine_range)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if !state.dragging {
                    return None;
                }
                let pos = cursor.land().position()?;
                let value = locate(pos.y);
                Some(canvas::Action::publish((self.on_drag)(value)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if !state.dragging {
                    return None;
                }
                state.dragging = false;
                Some(canvas::Action::publish((self.on_release)()).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(bounds) {
                    return None;
                }
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };
                if dy == 0.0 {
                    return None;
                }
                let step = if self.modifiers.shift() {
                    0.0002
                } else if self.modifiers.control() {
                    0.004
                } else {
                    0.001
                };
                let value = (self.value + dy * step).clamp(0.0, 1.0);
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
            draw_meter(
                &mut frame,
                Rectangle::new(Point::ORIGIN, Size::new(METER_W, bounds.height)),
                self.meter,
            );
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
        if self.show_meter {
            draw_ruler(
                &mut frame,
                Rectangle::new(
                    Point::new(self.track_x() + TRACK_W + GAP, 0.0),
                    Size::new(RULER_W, bounds.height),
                ),
            );
        }
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

const METER_SEGMENTS: usize = 18;
const METER_GAP: f32 = 1.0;

/// A segmented LED-ladder meter, like real hardware — each segment is
/// either lit or dark (dimly visible when dark, so the ladder itself is
/// always readable), rather than one continuous fill.
fn draw_meter(frame: &mut Frame, r: Rectangle, level: f32) {
    let l = level.clamp(0.0, 1.0);
    frame.fill_rectangle(r.position(), r.size(), Color::from_rgb8(0x08, 0x08, 0x0a));

    let n = METER_SEGMENTS as f32;
    let seg_h = (r.height - METER_GAP * (n - 1.0)) / n;

    for i in 0..METER_SEGMENTS {
        let seg_bottom_frac = i as f32 / n;
        let seg_top_frac = (i as f32 + 1.0) / n;
        let lit = l > seg_bottom_frac;

        let color = if seg_top_frac < 0.6 {
            theme::MGREEN
        } else if seg_top_frac < 0.85 {
            theme::MYELLOW
        } else {
            theme::MRED
        };

        let y = r.y + r.height - ((i as f32 + 1.0) * seg_h + i as f32 * METER_GAP);
        let seg_color = if lit {
            color
        } else {
            Color { a: 0.10, ..color }
        };
        frame.fill_rectangle(Point::new(r.x, y), Size::new(r.width, seg_h), seg_color);
    }
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

fn draw_ruler(frame: &mut Frame, r: Rectangle) {
    const TICKS: [f32; 6] = [0.0, -6.0, -10.0, -20.0, -40.0, -60.0];

    for db in TICKS {
        let t = db_to_t(db);
        let y = r.y + r.height - r.height * t;
        let y = y.clamp(r.y + 4.0, r.y + r.height - 4.0);

        let tick = Path::line(Point::new(r.x, y), Point::new(r.x + 3.0, y));
        frame.stroke(
            &tick,
            Stroke::default().with_color(theme::TEXT_SEC).with_width(1.0),
        );

        let label = if db == 0.0 {
            "0".to_string()
        } else {
            format!("{}", -db as i32)
        };
        frame.fill_text(canvas::Text {
            content: label,
            position: Point::new(r.x + 5.0, y - 4.0),
            color: theme::TEXT_SEC,
            size: 6.5.into(),
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
        METER_W + GAP + TRACK_W + GAP + RULER_W
    } else {
        TRACK_W
    };
    Canvas::new(fader)
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .into()
}
