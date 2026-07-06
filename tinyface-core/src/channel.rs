use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Identifies a specific channel on an RME device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
pub enum ChannelId {
    Input(usize),
    Playback(usize),
}

/// The type of an input channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum ChannelType {
    Mic,
    Instrument,
    Line,
    SPDIF,
    ADAT,
}

/// A single physical hardware input channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputChannel {
    pub id: usize,
    pub name: String,
    pub channel_type: ChannelType,
    pub volume: f32,           // 0.0 – 1.0
    pub pan: i8,               // -100 .. 100
    pub phantom: bool,         // 48V
    pub pad: bool,
    pub sensitivity: Option<Sensitivity>,
    pub routing: usize,        // index into the device's output list
}

/// Sensitivity setting for instrument inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum Sensitivity {
    Minus10dBV,
    Plus4dBu,
}

/// A single software playback channel (from the computer to the device).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackChannel {
    pub id: usize,
    pub name: String,
    pub volume: f32,
    pub pan: i8,
    pub routing: usize,
}
