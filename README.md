<p align="center">
  <img src="https://img.shields.io/badge/status-pre--alpha-yellow" alt="Status">
  <img src="https://img.shields.io/badge/license-GPL--3.0-blue" alt="License">
  <img src="https://img.shields.io/badge/Rust-❤-red" alt="Rust">
</p>

# TuxMix

### Open-source TotalMix replacement for RME audio interfaces on Linux.

TuxMix gives you full control over your RME interface's **hardware DSP mixer** directly from Linux — no TotalMix, no Windows, no macOS required.

---

## Why tuxmix exists

RME makes incredible audio interfaces. Their hardware is legendary on Linux thanks to solid ALSA drivers. **But TotalMix doesn't exist on Linux.** Users have been stuck:

- ⛔ Launching a Windows VM just to adjust a headphone mix
- ⛔ Dual-booting to change the 48V or routing
- ⛔ Using cryptic `amixer` scripts
- ⛔ Switching OS just to use their €1000+ interface

**tuxmix fixes that.**

When RME released TotalMix 2.0 in May 2026, the #1 request was loud and clear:

> *"We need a Linux version! Windows has definitely worn out its welcome."* — **70+ upvotes**
>
> *"Not having TotalMix is practically the only thing keeping me on Windows for recording and mixing."*
>
> *"If you make a Linux version I'm going to buy UFX3 + 12Mic and later MADIface XT and another 12Mic."*
>
> *"RME would be king if releasing Linux version of TotalMix."*

TuxMix is the answer.

---

## Features

| Feature | Status |
|---|---|
| ✅ ALSA device detection | Done |
| ✅ Per-output volume control (submixes) | Done |
| ✅ Per-output pan control | Done |
| ✅ 48V phantom power | Done |
| ✅ PAD | Done |
| ✅ Sensitivity (Lo Gain / +4dBu) | Done |
| ✅ Input & playback channel strips | Done |
| ✅ Animated VU meters (144Hz-smooth, ballistics-shaped) | Done |
| ✅ Matrix mixer view (submixes) | Done |
| ✅ Scene capture / save / restore (JSON), with device-model safety check | Done |
| ✅ Multi-select (Ctrl/Shift+click) with grouped mute/solo/collapse/volume/pan | Done |
| ✅ Collapsible strips, live UI zoom (Ctrl+=/Ctrl+-/Ctrl+0) | Done |
| ✅ Multi-device-ready core (`DeviceProfile`) — see [Supported hardware](#supported-hardware) | Done |
| ✅ Simulated mode (`--mock` for dev without hardware) | Done |
| ✅ Desktop GUI (iced) | Done |
| ✅ Terminal TUI (ratatui) | Done |
| ⏳ Loopback control | Planned |
| ⏳ AUR package | PKGBUILD written, not yet published — see [Release status](#release-status) |

---

## Quick start

### With hardware

```bash
cargo run -p tuxmix-gui          # Desktop GUI
cargo run -p tuxmix-tui          # Terminal TUI
```

### Without hardware (simulated mode)

Don't have your RME card yet? Develop the full UI with a simulated device:

```bash
cargo run -p tuxmix-gui -- --mock
cargo run -p tuxmix-tui -- --mock
```

The mock simulates all 12 inputs, 12 playbacks, animated VU meters, and all controls — perfect for development or evaluation.

### From AUR (Arch Linux)

Not published yet — see [Release status](#release-status). A `PKGBUILD`
already lives in `aur/tuxmix/` and will go up once the first tagged
release exists; until then, build from source (below).

---

## Architecture

```
TuxMix/
├── tuxmix-core/     Hardware-agnostic RME control library (Rust + ALSA)
│   ├── device.rs      RmeDevice trait — add support for any RME interface
│   ├── profile.rs      DeviceProfile — declarative per-model channel topology
│   ├── profiles/        One const DeviceProfile per supported model
│   ├── babyface.rs    Babyface Pro FS implementation (consumes a DeviceProfile)
│   ├── mock.rs        Simulated device for development & CI
│   ├── channel.rs     Input & playback channel model (per-output volumes)
│   ├── mixer.rs       ALSA mixer wrapper
│   └── scene.rs       Scene save/restore (JSON serialization)
├── tuxmix-tui/      Terminal UI (ratatui + crossterm)
└── tuxmix-gui/      Desktop GUI (iced)
```

The core library exposes a clean `RmeDevice` trait, and ALSA-selem devices
(the Babyface family and similar USB class-compliant interfaces) plug into
it via a `DeviceProfile` — a data description of a model's channel layout,
not hand-written per-device code. Adding support for a new ALSA-selem RME
interface means writing one `DeviceProfile` and verifying its ALSA control
names against real `amixer scontents` output — see
[Contributing](CONTRIBUTING.md).

RME's newer "TotalMix FX 2"-generation interfaces (Fireface UCX II, UFX+/
III) don't expose ALSA elements at all — they speak a proprietary MIDI
SysEx protocol instead, which needs a different I/O backend entirely. Not
implemented yet; tracked as a distinct, larger phase.

---

## Supported hardware

| Model | Status |
|---|---|
| **Babyface Pro FS** | 🟡 ALSA mapping written (`babyface.rs`), not yet verified against real hardware — see [Release status](#release-status) |
| Babyface Pro | 🟢 Planned (same `DeviceProfile` family, needs a real-device owner to verify `amixer scontents` output) |
| Fireface UCX II | 🔵 Needs the MIDI SysEx backend (not started) |
| Fireface UFX+ | 🔵 Needs the MIDI SysEx backend (not started) |
| MADIface Pro | 🔵 Needs the MIDI SysEx backend (not started) |
| Fireface UFX II / III | 🔵 Needs the MIDI SysEx backend (not started) |
| Any ALSA-selem RME interface via `DeviceProfile` | 🟢 Possible today |

🟡 in progress · 🟢 planned, reachable with the current architecture ·
🔵 blocked on a separate, larger MIDI SysEx I/O backend that doesn't
exist yet (these models don't expose ALSA controls at all).

---

## Release status

TuxMix doesn't have a tagged release or a published AUR package yet. The
`babyface.rs` ALSA control mapping was written from documentation and
community knowledge, not verified against a physical Babyface Pro FS —
the maintainer's own unit is expected in about a month, at which point the
mapping gets reverse-engineered/verified properly against real hardware
and the first release gets cut. Until then, `--mock` mode and
building from source are the way to run and evaluate TuxMix.

---

## Usage

### GUI mode

```
┌──────────────────────────────────────────────────────────────┐
│  tuxmix | Babyface Pro FS (mock) ● | [Tab: Mixer] | ⏱ ... │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│   HARDWARE INPUTS  (Submix: AN1/2)                          │
│   ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐                          │
│   │ AN1 │ │ AN2 │ │ IN3 │ │ IN4 │  ← per-channel strips     │
│   │ ██▌ │ │ ██▌ │ │ ██▌ │ │ ██▌ │    with VU meter         │
│   │ ║  │ │ ║  │ │ ║  │ │ ║  │    volume + 48V + PAD      │
│   └─────┘ └─────┘ └─────┘ └─────┘                          │
│                                                              │
│   SOFTWARE PLAYBACK                                         │
│   ┌──────┐ ┌──────┐ ┌──────┐                                │
│   │ PCM  │ │ PCM  │ │ PCM  │                                │
│   │ AN1  │ │ AN2  │ │ PH3  │                                │
│   └──────┘ └──────┘ └──────┘                                │
│                                                              │
│   SCENES:  [📸 Capture]  [💾 Save...]  [📂 Load...]          │
└──────────────────────────────────────────────────────────────┘
```

Press **Tab** to toggle between Mixer view and Matrix view.

### Matrix view (Tab)

```
┌──────────────────────────────────────────────────────────────┐
│       AN1  AN2  IN3  IN4  AS1  AS2  ...  PCM1  PCM2        │
│AN1/2   75   75   75   75   75   75  ...   80     80          │
│PH3/4   50   50   75   75   75   75  ...   80     80          │
│AS1/2   75   75   75   75   75   75  ...   80     80          │
│...                                                           │
└──────────────────────────────────────────────────────────────┘
```

---

## Building from source

```bash
git clone https://github.com/iswad-lab/tuxmix
cd tuxmix
cargo build --release
```

Dependencies: `rustc`, `cargo`, `alsa-lib` (dev headers).

---

## Contributing

TuxMix is in early development and contributions are welcome. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the concrete recipe to add a new
ALSA-selem device profile, plus:

- **Rust / ALSA**: Help verify the Babyface Pro FS ALSA control mapping against real hardware
- **UI / UX**: Improve the iced GUI or ratatui TUI
- **New hardware**: Add a `DeviceProfile` for your RME interface (ALSA-selem models only for now — see [Architecture](#architecture))
- **MIDI SysEx**: Help design the I/O backend for UCX II/UFX+/III-class devices
- **Testing**: Run `cargo test` and report issues

See the [open issues](https://github.com/iswad-lab/tuxmix/issues) for starter tasks.

---

## How it compares

| | TotalMix FX (Windows/Mac) | bbfpromix | **tuxmix** |
|---|---|---|---|
| Linux support | ❌ | ✅ (Babyface only) | ✅ |
| Multi-card | ✅ | ❌ | 🟢 designed for it |
| Matrix mixer | ✅ | ❌ | ✅ |
| Scenes / snapshots | ✅ | ❌ | ✅ |
| TUI (SSH/terminal) | ❌ | ❌ | ✅ |
| Simulated mode | ❌ | ❌ | ✅ (--mock) |
| Open source | ❌ | ✅ GPL | ✅ GPL-3.0 |
| Written in | C++/MFC | C/GTK3 | **Rust** |

---

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).

---

<p align="center">
  <b>tuxmix is not affiliated with RME GmbH.</b>
  <br>
  RME, TotalMix, Babyface, Fireface, MADIface are registered trademarks of RME GmbH.
  <br>
  TotalMix 2.0 is a product of RME GmbH — no copyright infringement intended.
</p>
