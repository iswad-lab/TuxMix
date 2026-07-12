# TODO

## ~~Perceived jank when zoomed out~~ — RESOLVED: was a debug build, not a real issue

Originally reported as possible rendering jank (fluid at max UI zoom,
choppy zoomed out) on real hardware (144Hz display, CachyOS). Chased
several theories — too many `Canvas` widgets self-requesting redraws
at once, iGPU vs. dGPU rendering path, Wayland vs. XWayland — none of
which held up cleanly, and the real `%CPU` deltas measured (4% → 5% →
6%) never supported a dramatic explanation either way.

**Actual cause**: testing was happening via `cargo run -p tuxmix-gui
-- --mock`, i.e. a Rust **debug** build, without `--release`. Confirmed
directly: `mangohud ./target/release/tuxmix-gui --mock` (release
binary) felt fully fluid at every zoom level; `cargo run --release -p
tuxmix-gui -- --mock` (debug → release, same invocation otherwise) was
also fluid once `--release` was added. A debug build is simply not
representative of real per-frame rendering performance in Rust — this
is normal and expected, not a TuxMix architecture problem.

**Lesson for next time**: always judge perceived performance/fluidity
against a `--release` build. Plain `cargo run` for this crate will
feel choppy even with a perfectly fine rendering pipeline behind it —
check the build profile *before* investigating widget counts, GPU
routing, or anything else.
