//! Matrix (submix) view — every input/playback channel against every
//! hardware output pair, one small fader per cell.

use iced::widget::{column, container, row, scrollable, text};
use iced::Element;
use tinyface_core::{ChannelId, RmeDevice};

use crate::app::{Message, TinyFace, OUT_LABELS};
use crate::theme;
use crate::widgets::fader::{fader, Fader};

const CELL_H: f32 = 26.0;

pub fn view(state: &TinyFace) -> Element<'_, Message> {
    let ni = state.device.inputs().len();
    let np = state.device.playbacks().len();

    let mut row_labels = column![text("").size(9)].spacing(2);
    for (out, label) in OUT_LABELS.iter().enumerate() {
        let active = out == state.sel_out;
        let color = if active { theme::ACCENT } else { theme::TEXT_SEC };
        row_labels = row_labels.push(text(*label).color(color).size(9));
    }

    let mut cols = row![row_labels].spacing(2);

    for col in 0..(ni + np) {
        let (name, cid) = if col < ni {
            (
                state.device.inputs()[col].name.clone(),
                ChannelId::Input(col),
            )
        } else {
            (
                state.device.playbacks()[col - ni].name.clone(),
                ChannelId::Playback(col - ni),
            )
        };

        let mut col_widget = column![text(name).color(theme::TEXT_SEC).size(9)].spacing(2);
        for out in 0..OUT_LABELS.len() {
            let vol = if col < ni {
                state.device.inputs()[col].volumes[out]
            } else {
                state.device.playbacks()[col - ni].volumes[out]
            };
            col_widget = col_widget.push(fader(Fader {
                value: vol,
                range: (0.0, 1.0),
                meter: 0.0,
                height: CELL_H,
                show_meter: false,
                modifiers: state.modifiers,
                on_press: Box::new(move |v, _| Message::VolumeChanged(cid, out, v)),
                on_drag: Box::new(move |v| Message::VolumeChanged(cid, out, v)),
                on_release: Box::new(move || Message::RangeCleared(cid)),
                on_reset: Box::new(move || Message::VolumeChanged(cid, out, 0.75)),
            }));
        }
        cols = cols.push(col_widget);
    }

    let scroller = scrollable(cols).direction(scrollable::Direction::Horizontal(
        scrollable::Scrollbar::default(),
    ));

    container(scroller).style(theme::panel).padding(8).into()
}
