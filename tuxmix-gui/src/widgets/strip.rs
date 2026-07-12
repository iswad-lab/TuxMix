//! A single channel strip: label + type tag, mute/solo, 48V/PAD, fader+VU,
//! dB readout (double-click to edit), pan readout.

use iced::keyboard::Modifiers;
use iced::widget::{button, column, container, mouse_area, row, text, text_input, tooltip};
use iced::{Color, Element, Length};
use std::time::Instant;
use tuxmix_core::ChannelId;

use crate::app::{db_text, short_label, Message};
use crate::theme;
use crate::widgets::fader::{fader, pan_indicator, vu_meter, Fader, MeterFrame, PanIndicator};

/// Base sizes at `scale == 1.0` (`theme::SCALE_DEFAULT`) — every dimension
/// in a strip is one of these times `StripParams::scale`, so the live UI
/// zoom (Ctrl+=/Ctrl+-/Ctrl+0) resizes strips the same way it resizes text.
const FADER_H: f32 = 168.0;
pub(crate) const STRIP_W: f32 = 104.0;
/// Collapsed strips are a glance-only readout: name + VU meter, nothing
/// else — no fader, no mute/solo, no pan. Trading away every control for
/// space is the point; a strip you still need to touch shouldn't be
/// collapsed. Width is set by the header (name + expand button), not the
/// meter, which is narrower than that on its own.
pub(crate) const COLLAPSED_W: f32 = 60.0;

/// How long a strip's collapse/expand width transition takes — longer than
/// the meter's 50ms interp window since this is a much bigger, structural
/// change (the whole card growing or shrinking), not a small value nudge.
/// Same linear ease-style interpolation as `fader::MeterFrame`, just a
/// dedicated type since the duration and the thing being interpolated
/// (a pixel width, not a volume/pan value) are both different.
const COLLAPSE_INTERP_MS: f32 = 160.0;

#[derive(Clone, Copy, Debug)]
pub struct CollapseAnim {
    pub prev: f32,
    pub value: f32,
    pub since: Instant,
}

impl CollapseAnim {
    pub fn at(&self, now: Instant) -> f32 {
        let t = (now.duration_since(self.since).as_secs_f32() * 1000.0 / COLLAPSE_INTERP_MS)
            .clamp(0.0, 1.0);
        self.prev + (self.value - self.prev) * t
    }

    pub fn is_settling(&self, now: Instant) -> bool {
        (self.value - self.prev).abs() > f32::EPSILON
            && now.duration_since(self.since).as_secs_f32() * 1000.0 < COLLAPSE_INTERP_MS
    }
}

pub struct StripParams<'a> {
    pub cid: ChannelId,
    pub output_idx: usize,
    pub name: String,
    pub type_tag: Option<(&'static str, Color)>,
    pub vol: f32,
    pub pan: i8,
    pub meter: MeterFrame,
    pub has_48v: bool,
    pub has_pad: bool,
    pub phantom: bool,
    pub pad: bool,
    pub mute: bool,
    pub solo: bool,
    pub default_vol: f32,
    pub editing: bool,
    pub edit_buf: &'a str,
    pub drag_range: Option<(f32, f32)>,
    pub modifiers: Modifiers,
    pub collapsed: bool,
    pub collapse_anim: Option<CollapseAnim>,
    pub scale: f32,
    pub selected: bool,
}

/// A button's own padding-based centering isn't reliable across glyphs of
/// different intrinsic width (e.g. "S" sat visibly left of center while "M"
/// looked fine) — force it explicitly instead of trusting the default. The
/// default 1.2x line-height also reserves descender space these glyphs
/// (M, S, no descenders) never use, which reads as "sitting too high" once
/// centered — tightening it to 1:1 removes that residual vertical bias.
fn centered_label<'a>(s: &'a str, size: f32) -> Element<'a, Message> {
    container(
        text(s)
            .size(size)
            .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(
                size,
            ))),
    )
    .center(Length::Fill)
    .into()
}

/// Wraps a control in a hover tooltip — for the abbreviations (M, S, 48V,
/// PAD) that read as pro-audio jargon to anyone not already fluent in it.
/// A short delay so it doesn't flash on every incidental mouse-over while
/// moving across the strip toward something else.
fn hint<'a>(content: impl Into<Element<'a, Message>>, label: &'a str, scale: f32) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(label).size(theme::TEXT_XS * scale).color(theme::TEXT_PRIMARY))
            .padding(theme::SPACE_SM * scale)
            .style(theme::panel),
        tooltip::Position::Top,
    )
    .gap(4.0 * scale)
    .delay(std::time::Duration::from_millis(400))
    .into()
}

fn header_row<'a>(
    cid: ChannelId,
    name: &str,
    collapsed: bool,
    type_tag: Option<(&'static str, Color)>,
    scale: f32,
) -> Element<'a, Message> {
    // "-"/"+" rather than a chevron glyph — guaranteed to render on any
    // font, no risk of tofu boxes for a symbol the default sans might lack.
    let collapse_btn = hint(
        button(centered_label(
            if collapsed { "+" } else { "-" },
            theme::TEXT_SM * scale,
        ))
        .padding(0)
        .width(18.0 * scale)
        .height(16.0 * scale)
        .style(theme::plain_button)
        .on_press(Message::ToggleCollapse(cid)),
        if collapsed { "Expand" } else { "Collapse" },
        scale,
    );

    let mut header = row![text(short_label(name).to_string()).size(theme::TEXT_MD * scale)]
        .spacing(theme::SPACE_TIGHT);
    if !collapsed {
        if let Some((tag, color)) = type_tag {
            header = header.push(text(tag).color(color).size(theme::TEXT_XS * scale));
        }
    }
    header
        .push(iced::widget::Space::new().width(Length::Fill))
        .push(collapse_btn)
        .width(Length::Fill)
        .align_y(iced::Alignment::Center)
        .into()
}

/// A collapsed strip is a glance-only readout — trading every control away
/// (fader, mute/solo, pan) is the point of collapsing it, not a side effect.
/// Only ever rendered fully settled (see `strip()`'s dispatch), so `w` is
/// always `COLLAPSED_W`, but it's threaded through rather than hardcoded to
/// keep this in lockstep with `full_strip`'s signature.
fn collapsed_strip<'a>(p: StripParams<'a>, w: f32) -> Element<'a, Message> {
    let rows = column![
        header_row(p.cid, &p.name, true, p.type_tag, p.scale),
        vu_meter(p.meter, FADER_H * p.scale, p.scale),
    ]
    .spacing(theme::SPACE_HAIRLINE)
    .width(Length::Fill)
    .align_x(iced::Alignment::Center);

    mouse_area(
        container(rows)
            .style(theme::strip_panel(p.selected, p.type_tag.map(|(_, c)| c)))
            .padding([theme::SPACE_SM * p.scale, theme::SPACE_MD * p.scale])
            .width(Length::Fixed(w * p.scale))
            .clip(true),
    )
    .on_press(Message::StripClicked(p.cid))
    .on_double_click(Message::ToggleCollapse(p.cid))
    .into()
}

/// Picks between the two strip layouts and, while a collapse/expand
/// animation is in flight, the width the outer card should be drawn at
/// this frame. The full (uncollapsed) content is shown not just when
/// resting expanded but for the whole transition in *either* direction —
/// shrinking, it's the thing visibly getting clipped down to
/// `COLLAPSED_W`; growing, it's what's being revealed. Only once a
/// collapse has fully settled does rendering switch to the lighter,
/// control-free `collapsed_strip`.
pub fn strip<'a>(p: StripParams<'a>) -> Element<'a, Message> {
    let now = Instant::now();
    let (w, show_full) = match &p.collapse_anim {
        Some(a) => (a.at(now), a.is_settling(now) || !p.collapsed),
        None => {
            if p.collapsed {
                (COLLAPSED_W, false)
            } else {
                (STRIP_W, true)
            }
        }
    };

    if show_full {
        full_strip(p, w)
    } else {
        collapsed_strip(p, w)
    }
}

fn full_strip<'a>(p: StripParams<'a>, w: f32) -> Element<'a, Message> {
    let cid = p.cid;
    let out = p.output_idx;
    let scale = p.scale;

    let header = header_row(cid, &p.name, false, p.type_tag, scale);

    let mute_btn = hint(
        button(centered_label("M", theme::TEXT_SM * scale))
            .width(Length::Fill)
            .height(18.0 * scale)
            .style(theme::toggle_button(p.mute, theme::MUTE_COLOR))
            .on_press(Message::Mute(cid, !p.mute)),
        "Mute",
        scale,
    );
    let solo_btn = hint(
        button(centered_label("S", theme::TEXT_SM * scale))
            .width(Length::Fill)
            .height(18.0 * scale)
            .style(theme::toggle_button(p.solo, theme::SOLO_COLOR))
            .on_press(Message::Solo(cid, !p.solo)),
        "Solo",
        scale,
    );
    // Fixed-width buttons left dead space flanking them whenever the card
    // was sized for a wider sibling row (48V/PAD, or just a long channel
    // name) — filling the row makes every row use the card's full width
    // instead of only the widest one.
    let ms_row = row![mute_btn, solo_btn].spacing(theme::SPACE_TIGHT).width(Length::Fill);

    let mut rows = column![header, ms_row].spacing(theme::SPACE_HAIRLINE);

    if let ChannelId::Input(idx) = cid {
        if p.has_48v || p.has_pad {
            let mut tg_row = row![].spacing(theme::SPACE_TIGHT).width(Length::Fill);
            if p.has_48v {
                tg_row = tg_row.push(hint(
                    button(centered_label("48V", theme::TEXT_SM * scale))
                        .width(Length::Fill)
                        .height(18.0 * scale)
                        .style(theme::toggle_button(p.phantom, theme::PHANTOM))
                        .on_press(Message::Phantom(idx, !p.phantom)),
                    "48V phantom power",
                    scale,
                ));
            }
            if p.has_pad {
                tg_row = tg_row.push(hint(
                    button(centered_label("PAD", theme::TEXT_SM * scale))
                        .width(Length::Fill)
                        .height(18.0 * scale)
                        .style(theme::toggle_button(p.pad, theme::ACCENT))
                        .on_press(Message::Pad(idx, !p.pad)),
                    "-20 dB pad",
                    scale,
                ));
            }
            rows = rows.push(tg_row);
        }
    }

    let default_vol = p.default_vol;
    let fader_widget = fader(Fader {
        value: p.vol,
        range: p.drag_range.unwrap_or((0.0, 1.0)),
        default_value: default_vol,
        meter: p.meter,
        height: FADER_H * scale,
        show_meter: true,
        modifiers: p.modifiers,
        scale,
        on_press: Box::new(move |v, range| Message::FaderPressed(cid, out, v, range)),
        on_drag: Box::new(move |v| Message::VolumeChanged(cid, out, v)),
        on_release: Box::new(move || Message::RangeCleared(cid)),
        on_reset: Box::new(move || Message::Reset(cid, out, default_vol)),
    });
    rows = rows.push(fader_widget);

    let db_row: Element<'a, Message> = if p.editing {
        text_input("", p.edit_buf)
            .on_input(Message::EditChanged)
            .on_submit(Message::EditCommit)
            .style(theme::text_input)
            .size(theme::TEXT_SM * scale)
            .width(Length::Fixed(64.0 * scale))
            .into()
    } else {
        let initial = if p.vol > 0.0 {
            format!("{:.1}", 20.0 * p.vol.log10())
        } else {
            "-inf".into()
        };
        mouse_area(
            text(db_text(p.vol))
                .color(theme::TEXT_SEC)
                .size(theme::TEXT_XS * scale),
        )
        .on_double_click(Message::EditStart(cid, initial))
        .into()
    };
    rows = rows.push(db_row);

    // Outputs have no per-channel pan in the device model (a single master
    // volume covers the stereo pair) — only inputs/playbacks route to a pan
    // position within each output.
    if !matches!(cid, ChannelId::Output(_)) {
        let pan_str = match p.pan.cmp(&0) {
            std::cmp::Ordering::Less => format!("L{}", -p.pan),
            std::cmp::Ordering::Greater => format!("R{}", p.pan),
            std::cmp::Ordering::Equal => "C".to_string(),
        };
        rows = rows.push(
            column![
                pan_indicator(PanIndicator {
                    pan: p.pan,
                    modifiers: p.modifiers,
                    scale,
                    on_change: Box::new(move |pan| Message::PanChanged(cid, out, pan)),
                    on_reset: Box::new(move || Message::PanReset(cid, out)),
                }),
                text(pan_str).color(theme::TEXT_SEC).size(theme::TEXT_XS * scale),
            ]
            .spacing(theme::SPACE_HAIRLINE)
            .align_x(iced::Alignment::Center),
        );
    }

    // Double-click anywhere on the card that isn't already claimed by a
    // specific control (the fader/pan canvases capture their own
    // double-click for reset-to-default, the dB readout for its edit
    // field, buttons for their own press) collapses the strip — a bigger,
    // more discoverable target than the tiny "-" button alone. A plain
    // click there is a no-op; Ctrl/Shift+click toggles multi-selection
    // (see `Message::StripClicked`) — mute/solo/collapse on any selected
    // strip then apply to the whole selection at once.
    mouse_area(
        container(
            rows.width(Length::Fill)
                .align_x(iced::Alignment::Center),
        )
        .style(theme::strip_panel(p.selected, p.type_tag.map(|(_, c)| c)))
        .padding([theme::SPACE_SM * scale, theme::SPACE_MD * scale])
        .width(Length::Fixed(w * scale))
        .clip(true),
    )
    .on_press(Message::StripClicked(cid))
    .on_double_click(Message::ToggleCollapse(cid))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn settled_anim_is_never_settling() {
        let a = CollapseAnim { prev: STRIP_W, value: STRIP_W, since: Instant::now() };
        assert!(!a.is_settling(a.since));
        assert!(!a.is_settling(a.since + Duration::from_millis(10)));
    }

    #[test]
    fn transitioning_anim_settles_after_the_interp_window() {
        let a = CollapseAnim { prev: STRIP_W, value: COLLAPSED_W, since: Instant::now() };
        assert!(a.is_settling(a.since), "just started — should still be settling");
        assert!(
            a.is_settling(a.since + Duration::from_millis(80)),
            "mid-transition — should still be settling"
        );
        assert!(
            !a.is_settling(a.since + Duration::from_millis(200)),
            "past the interp window — should have stopped requesting redraws"
        );
    }

    #[test]
    fn at_interpolates_linearly_from_prev_to_value() {
        let a = CollapseAnim { prev: STRIP_W, value: COLLAPSED_W, since: Instant::now() };
        assert_eq!(a.at(a.since), STRIP_W);
        assert_eq!(a.at(a.since + Duration::from_millis(160)), COLLAPSED_W);
        let mid = a.at(a.since + Duration::from_millis(80));
        let expected_mid = (STRIP_W + COLLAPSED_W) / 2.0;
        assert!(
            (mid - expected_mid).abs() < 0.5,
            "expected ~{expected_mid} at the midpoint, got {mid}"
        );
    }
}
