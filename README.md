<p align="center">
  <img src="https://img.shields.io/badge/status-pre--alpha-yellow" alt="Status">
  <img src="https://img.shields.io/badge/license-GPL--3.0-blue" alt="License">
  <img src="https://img.shields.io/badge/Rust-❤-red" alt="Rust">
</p>

# Tinyface

### Open-source TotalMix replacement for RME audio interfaces on Linux.

Tinyface gives you full control over your RME interface's **hardware DSP mixer** directly from Linux — no TotalMix, no Windows, no macOS required.

---

## Why Tinyface exists

RME makes incredible audio interfaces. Their hardware is legendary on Linux thanks to solid ALSA drivers. **But TotalMix doesn't exist on Linux.** Users have been stuck:

- ⛔ Launching a Windows VM just to adjust a headphone mix
- ⛔ Dual-booting to change the 48V or routing
- ⛔ Using cryptic `amixer` scripts
- ⛔ Switching OS just to use their €1000+ interface

**Tinyface fixes that.**

When RME released TotalMix 2.0 in May 2026, the #1 request was loud and clear:

> *"We need a Linux version! Windows has definitely worn out its welcome."* — **70+ upvotes**
>
> *"Not having TotalMix is practically the only thing keeping me on Windows for recording and mixing."*
>
> *"If you make a Linux version I'm going to buy UFX3 + 12Mic and later MADIface XT and another 12Mic."*
>
> *"RME would be king if releasing Linux version of TotalMix."*

Tinyface is the answer.

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
| ✅ Matrix mixer view (submixes) | Done |
| ✅ Scene capture / save / restore (JSON) | Done |
| ✅ Simulated mode (`--mock` for dev without hardware) | Done |
| ✅ Desktop GUI (egui) | Done |
| ✅ Terminal TUI (ratatui) | Done |
| ⏳ VU meters (needs USB RE) | Planned |
| ⏳ Loopback control | Planned |
| ⏳ AUR package | Done |

---

## Quick start

### With hardware

```bash
cargo run -p tinyface-gui          # Desktop GUI
cargo run -p tinyface-tui          # Terminal TUI
```

### Without hardware (simulated mode)

Don't have your RME card yet? Develop the full UI with a simulated device:

```bash
cargo run -p tinyface-gui -- --mock
cargo run -p tinyface-tui -- --mock
```

The mock simulates all 12 inputs, 12 playbacks, animated VU meters, and all controls — perfect for development or evaluation.

### From AUR (Arch Linux)

```bash
yay -S tinyface
```

---

## Architecture

```
Tinyface/
├── tinyface-core/     Hardware-agnostic RME control library (Rust + ALSA)
│   ├── device.rs      RmeDevice trait — add support for any RME interface
│   ├── babyface.rs    Babyface Pro FS implementation
│   ├── mock.rs        Simulated device for development & CI
│   ├── channel.rs     Input & playback channel model (per-output volumes)
│   ├── mixer.rs       ALSA mixer wrapper
│   └── scene.rs       Scene save/restore (JSON serialization)
├── tinyface-tui/      Terminal UI (ratatui + crossterm)
└── tinyface-gui/      Desktop GUI (egui / eframe)
```

The core library exposes a clean `RmeDevice` trait. Adding support for a new RME interface is as simple as implementing the trait with the right ALSA control mapping.

---

## Supported hardware

| Model | Status |
|---|---|
| **Babyface Pro FS** | 🟡 In progress (primary target) |
| Babyface Pro | 🟢 Planned |
| Fireface UCX II | 🟢 Planned |
| Fireface UFX+ | 🟢 Planned |
| MADIface Pro | 🟢 Planned |
| Fireface UFX II / III | 🟢 Planned |
| Any RME interface via trait impl | 🟢 Possible |

---

## Usage

### GUI mode

```
┌──────────────────────────────────────────────────────────────┐
│  Tinyface | Babyface Pro FS (mock) ● | [Tab: Mixer] | ⏱ ... │
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
git clone https://github.com/iswad-lab/Tinyface
cd Tinyface
cargo build --release
```

Dependencies: `rustc`, `cargo`, `alsa-lib` (dev headers).

---

## Contributing

Tinyface is in early development and contributions are welcome.

- **Rust / ALSA**: Help implement the real Babyface Pro FS ALSA controls
- **UI / UX**: Improve the egui GUI or ratatui TUI
- **New hardware**: Add support for your RME interface (implement `RmeDevice`)
- **USB reverse engineering**: Help discover the protocol for VU meters
- **Testing**: Run `cargo test` and report issues

See the [open issues](https://github.com/iswad-lab/Tinyface/issues) for starter tasks.

---

## How it compares

| | TotalMix FX (Windows/Mac) | bbfpromix | **Tinyface** |
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
  <b>Tinyface is not affiliated with RME GmbH.</b>
  <br>
  RME, TotalMix, Babyface, Fireface, MADIface are registered trademarks of RME GmbH.
  <br>
  TotalMix 2.0 is a product of RME GmbH — no copyright infringement intended.
</p>
