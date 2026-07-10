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
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
const FINE_SPAN: f32 = 0.08;

pub struct Fader<Message> {
    pub value: f32,
    pub range: (f32, f32),
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
            let t = 1.0 - ((y - bounds.y) / bounds.height).clamp(0.0, 1.0);
            lo + t * (hi - lo)
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

fn draw_meter(frame: &mut Frame, r: Rectangle, level: f32) {
    let l = level.clamp(0.0, 1.0);
    frame.fill_rectangle(r.position(), r.size(), Color::from_rgb8(0x08, 0x08, 0x0a));

    if l > 0.0 {
        let fill_h = r.height * l;
        let fill_pos = Point::new(r.x, r.y + r.height - fill_h);
        let fill_size = Size::new(r.width, fill_h);

        let color = if l < 0.6 {
            theme::MGREEN
        } else if l < 0.85 {
            theme::MYELLOW
        } else {
            theme::MRED
        };
        let glow_alpha = (0.08 + l * 0.18).clamp(0.0, 0.35);
        frame.fill_rectangle(
            Point::new(fill_pos.x - 2.0, fill_pos.y),
            Size::new(fill_size.width + 4.0, fill_size.height),
            Color {
                a: glow_alpha,
                ..color
            },
        );
        frame.fill_rectangle(fill_pos, fill_size, color);

        if l >= 0.95 {
            frame.fill_rectangle(
                Point::new(fill_pos.x, fill_pos.y - 2.0),
                Size::new(fill_size.width, 3.0),
                theme::MRED,
            );
        }
    }

    for db in [-6.0_f32, -12.0, -24.0] {
        let y = r.y + r.height - r.height * 10f32.powf(db / 20.0);
        let alpha = if db == -6.0 {
            0.14
        } else if db == -12.0 {
            0.10
        } else {
            0.07
        };
        let path = Path::line(Point::new(r.x, y), Point::new(r.x + r.width, y));
        frame.stroke(
            &path,
            Stroke::default()
                .with_color(Color { a: alpha, ..Color::WHITE })
                .with_width(1.0),
        );
    }
}

const RAIL_W: f32 = 3.0;
const HANDLE_R: f32 = 7.0;

fn draw_track(frame: &mut Frame, r: Rectangle, value: f32, range: (f32, f32), dragging: bool) {
    let (lo, hi) = range;
    let t = if hi > lo {
        ((value - lo) / (hi - lo)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let cx = r.x + r.width / 2.0;
    let raw_y = r.y + r.height - r.height * t;
    let handle_y = raw_y.clamp(r.y + HANDLE_R, r.y + r.height - HANDLE_R);

    // Groove — the full-length rail, unfilled color.
    let groove = Path::rectangle(
        Point::new(cx - RAIL_W / 2.0, r.y),
        Size::new(RAIL_W, r.height),
    );
    frame.fill(&groove, theme::BORDER);

    // Filled portion of the rail, from the bottom up to the handle.
    if t > 0.0 {
        let fill = Path::rectangle(
            Point::new(cx - RAIL_W / 2.0, handle_y),
            Size::new(RAIL_W, r.y + r.height - handle_y),
        );
        frame.fill(&fill, theme::FADER);
    }

    // Handle — flat, modern: a soft drop shadow, a focus glow while
    // dragging, and a solid circle. No bevels, no gradients.
    let shadow = Path::circle(Point::new(cx, handle_y + 1.5), HANDLE_R + 1.0);
    frame.fill(&shadow, Color::from_rgba8(0x00, 0x00, 0x00, 0.35));

    if dragging {
        let glow = Path::circle(Point::new(cx, handle_y), HANDLE_R + 5.0);
        frame.fill(&glow, Color { a: 0.20, ..theme::FADER });
    }

    let handle = Path::circle(Point::new(cx, handle_y), HANDLE_R);
    frame.fill(&handle, if dragging { theme::FADER } else { Color::WHITE });

    frame.stroke(
        &handle,
        Stroke::default()
            .with_color(if dragging { Color::WHITE } else { theme::FADER })
            .with_width(2.0),
    );
}

pub fn fader<'a, Message: 'a>(fader: Fader<Message>) -> Element<'a, Message>
where
    Message: Clone,
{
    let height = fader.height;
    let width = if fader.show_meter {
        METER_W + GAP + TRACK_W
    } else {
        TRACK_W
    };
    Canvas::new(fader)
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .into()
}
