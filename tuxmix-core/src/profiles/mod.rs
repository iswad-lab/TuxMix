//! Per-model device profiles — see [`crate::profile::DeviceProfile`].
//!
//! To add a new ALSA-selem RME model: copy `babyface_pro.rs` as a
//! template for the profile data, then copy `babyface.rs` (the struct
//! implementing [`crate::device::RmeDevice`]) as a template for the
//! device, pointing it at your new profile. Explicitly re-verify
//! `selem_name`/`ch_type_str` in `babyface.rs` against real `amixer
//! scontents` output for your device first — that naming grammar is
//! only confirmed for the Babyface Pro so far.

pub mod babyface_pro;
