# 🎛 tinyface

> An open-source TotalMix replacement for RME audio interfaces on Linux.

**tinyface** gives you full control over your RME interface's hardware mixer
directly from Linux — no TotalMix, no Windows, no macOS required.

## Status

**Pre-alpha / Work in progress.**

The project is currently in early development. The author is waiting for
their Babyface Pro FS to arrive. In the meantime, the core architecture
is being laid out.

### Roadmap

- [x] Project scaffolding (workspace + core lib + TUI + GUI)
- [ ] ALSA device detection
- [ ] Volume / pan control
- [ ] 48V, PAD, sensitivity
- [ ] Output routing
- [ ] Clock & SPDIF settings
- [ ] Scene save/restore
- [ ] Matrix mixer view (submixes)
- [ ] Support other RME interfaces (Fireface, MADIface, UFX…)

## Architecture

```
tinyface/
├── tinyface-core/     ← Hardware-agnostic RME control library (Rust + ALSA)
├── tinyface-tui/      ← Terminal UI (ratatui) — great for SSH / scripts
└── tinyface-gui/      ← Desktop GUI (egui) — TotalMix-like experience
```

## Building

```bash
cd tinyface
cargo build --release
```

## Usage

```bash
# Terminal UI
cargo run -p tinyface-tui

# Desktop GUI
cargo run -p tinyface-gui
```

## Supported hardware

| Model | Status |
|---|---|
| Babyface Pro FS | 🚧 In progress |
| Babyface Pro | ⏳ Planned |
| Fireface UCX II | ⏳ Planned |
| Fireface UFX+ | ⏳ Planned |
| MADIface Pro | ⏳ Planned |

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).
