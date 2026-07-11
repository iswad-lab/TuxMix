//! A single channel strip: label + type tag, mute/solo, 48V/PAD, fader+VU,
//! dB readout (double-click to edit), pan readout.

use iced::keyboard::Modifiers;
use iced::widget::{button, column, container, mouse_area, row, text, text_input};
use iced::{Color, Element, Length};
use tuxmix_core::ChannelId;

use crate::app::{db_text, short_label, Message};
use crate::theme;
use crate::widgets::fader::{fader, pan_indicator, Fader};

const FADER_H: f32 = 150.0;
const STRIP_W: f32 = 108.0;

pub struct StripParams<'a> {
    pub cid: ChannelId,
    pub output_idx: usize,
    pub name: String,
    pub type_tag: Option<(&'static str, Color)>,
    pub vol: f32,
    pub pan: i8,
    pub meter: f32,
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
}

/// A button's own padding-based centering isn't reliable across glyphs of
/// different intrinsic width (e.g. "S" sat visibly left of center while "M"
/// looked fine) — force it explicitly instead of trusting the default. The
/// default 1.2x line-height also reserves descender space these glyphs
/// (M, S, no descenders) never use, which reads as "sitting too high" once
/// centered — tightening it to 1:1 removes that residual vertical bias.
fn centered_label<'a>(s: &'a str, size: u32) -> Element<'a, Message> {
    container(
        text(s)
            .size(size)
            .line_height(iced::widget::text::LineHeight::Absolute(
                iced::Pixels(size as f32),
            )),
    )
    .center(Length::Fill)
    .into()
}

pub fn strip<'a>(p: StripParams<'a>) -> Element<'a, Message> {
    let cid = p.cid;
    let out = p.output_idx;

    let mut header = row![text(short_label(&p.name).to_string()).size(11)].spacing(2);
    if let Some((tag, color)) = p.type_tag {
        header = header.push(text(tag).color(color).size(9));
    }

    let mute_btn = button(centered_label("M", 10))
        .width(30)
        .height(18)
        .style(theme::toggle_button(p.mute, theme::MUTE_COLOR))
        .on_press(Message::Mute(cid, !p.mute));
    let solo_btn = button(centered_label("S", 10))
        .width(30)
        .height(18)
        .style(theme::toggle_button(p.solo, theme::SOLO_COLOR))
        .on_press(Message::Solo(cid, !p.solo));
    let ms_row = row![mute_btn, solo_btn].spacing(2);

    let mut rows = column![header, ms_row].spacing(1);

    if let ChannelId::Input(idx) = cid {
        if p.has_48v || p.has_pad {
            let mut tg_row = row![].spacing(2);
            if p.has_48v {
                tg_row = tg_row.push(
                    button(centered_label("48V", 10))
                        .width(37)
                        .height(18)
                        .style(theme::toggle_button(p.phantom, theme::PHANTOM))
                        .on_press(Message::Phantom(idx, !p.phantom)),
                );
            }
            if p.has_pad {
                tg_row = tg_row.push(
                    button(centered_label("PAD", 10))
                        .width(37)
                        .height(18)
                        .style(theme::toggle_button(p.pad, theme::ACCENT))
                        .on_press(Message::Pad(idx, !p.pad)),
                );
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
        height: FADER_H,
        show_meter: true,
        modifiers: p.modifiers,
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
            .size(10)
            .width(Length::Fixed(64.0))
            .into()
    } else {
        let initial = if p.vol > 0.0 {
            format!("{:.1}", 20.0 * p.vol.log10())
        } else {
            "-inf".into()
        };
        mouse_area(text(db_text(p.vol)).color(theme::TEXT_SEC).size(9))
            .on_double_click(Message::EditStart(cid, initial))
            .into()
    };
    rows = rows.push(db_row);

    if p.pan != 0 {
        let pan_str = if p.pan < 0 {
            format!("L{}", -p.pan)
        } else {
            format!("R{}", p.pan)
        };
        rows = rows.push(
            column![
                pan_indicator(p.pan),
                text(pan_str).color(theme::TEXT_SEC).size(8),
            ]
            .spacing(1)
            .align_x(iced::Alignment::Center),
        );
    }

    container(
        rows.width(Length::Fill)
            .align_x(iced::Alignment::Center),
    )
    .style(theme::panel)
    .padding([3, 6])
    .width(Length::Fixed(STRIP_W))
    .into()
}
