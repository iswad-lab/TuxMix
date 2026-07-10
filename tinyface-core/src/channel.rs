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

/// Default number of output pairs for a device (Babyface Pro FS).
pub const DEFAULT_OUTPUT_PAIRS: usize = 6;

/// A single physical hardware input channel.
///
/// Each channel can be routed to every hardware output pair with
/// its own volume and pan — this is the submix (matrix) model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputChannel {
    pub id: usize,
    pub name: String,
    pub channel_type: ChannelType,
    /// Volume per output pair (0.0 – 1.0). Length = number of output pairs.
    pub volumes: Vec<f32>,
    /// Pan per output pair (-100 .. 100). Length = number of output pairs.
    pub pans: Vec<i8>,
    pub phantom: bool, // 48V
    pub pad: bool,
    pub sensitivity: Option<Sensitivity>,
}

/// Sensitivity setting for instrument inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum Sensitivity {
    Minus10dBV,
    Plus4dBu,
}

/// A single software playback channel (from the computer to the device).
///
/// Same submix model: one volume + pan per hardware output pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackChannel {
    pub id: usize,
    pub name: String,
    /// Volume per output pair (0.0 – 1.0).
    pub volumes: Vec<f32>,
    /// Pan per output pair (-100 .. 100).
    pub pans: Vec<i8>,
}

impl InputChannel {
    /// Create a new input channel with default values for `outputs` output pairs.
    pub fn new(id: usize, name: &str, channel_type: ChannelType, outputs: usize) -> Self {
        Self {
            id,
            name: name.to_string(),
            channel_type,
            volumes: vec![0.75; outputs],
            pans: vec![0; outputs],
            phantom: false,
            pad: false,
            sensitivity: None,
        }
    }
}

impl PlaybackChannel {
    /// Create a new playback channel with default values for `outputs` output pairs.
    pub fn new(id: usize, name: &str, outputs: usize) -> Self {
        Self {
            id,
            name: name.to_string(),
            volumes: vec![0.8; outputs],
            pans: vec![0; outputs],
        }
    }
}
