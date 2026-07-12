# TODO

## Perceived jank when zoomed out, unconfirmed root cause

Reported on real hardware (144Hz display, CachyOS): dragging a fader or
watching VU meters feels like genuine 144Hz motion at max UI zoom, but
loses that fluid feel when zoomed out (more channel strips visible at
once).

**What's been ruled out (partially):** the working theory going in was
"more visible channels = more `Canvas` widgets independently
self-requesting redraws via `RedrawRequested` = more aggregate per-frame
work" (see `MeterFrame`/`canvas::Action::request_redraw()` in
`tuxmix-gui/src/widgets/fader.rs`). Real `%CPU` measurements on the
reporter's machine don't strongly support this: idle-at-max-zoom vs.
idle-zoomed-out was 4% vs. 5%, and dragging-at-max-zoom vs.
dragging-zoomed-out was 5% vs. 6%. That's a real but modest delta, not
the dramatic scaling the widget-count theory would predict — so it's
likely a contributing factor at most, not the dominant cause.

**Why this can't be diagnosed further with what's currently available:**

- All testing so far used `--mock`, whose `update_meters()`
  (`tuxmix-core/src/mock.rs`) drives *every* input and playback channel
  toward a new random target on *every* tick, unconditionally — a
  worst-case, unrealistic simulation of "everything is constantly loud
  and moving." Real audio has most channels sitting quiet most of the
  time. Any `%CPU` measured against `--mock` is inflated relative to
  real use, in a way that's hard to correct for without real signal.
- `%CPU` alone can't diagnose perceived stutter anyway — what actually
  causes visible jank is a single frame's work exceeding the ~6.9ms
  budget at 144Hz, which is a *frame-timing* question, not an aggregate
  utilization one. No frame-timing instrumentation exists in this repo
  yet.
- Headless/sandboxed testing (see `.claude/skills/run-gui-headless/`)
  always forces the X11 backend and software (llvmpipe) rendering — it
  cannot reproduce or rule out a real Wayland vs. XWayland difference,
  which is the leading hypothesis (no explicit Wayland handling exists
  in `tuxmix-gui/src/main.rs`; winit's auto-detected backend has never
  been confirmed on real hardware).

**To actually investigate, need:**

1. Real RME hardware connected (not `--mock`), so meter activity
   reflects real audio instead of constant worst-case simulation.
2. Confirm the session type at runtime (`echo $XDG_SESSION_TYPE`) and
   whether the app actually ends up on native Wayland or gets pushed
   through XWayland.
3. Ideally, some frame-timing instrumentation (even a rough "time spent
   in `draw()` per frame, logged periodically") rather than relying on
   `%CPU` alone.
4. Re-run the same max-zoom vs. zoomed-out comparison under those
   conditions to see if the gap widens, narrows, or disappears.

Until then: this is a known, open, *unconfirmed* question — not
something to keep speculating about without the ability to verify a
fix actually helps.
