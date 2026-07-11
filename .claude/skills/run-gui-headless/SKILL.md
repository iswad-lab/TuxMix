---
name: run-gui-headless
description: Launch tuxmix-gui (Iced/wgpu desktop app) in an isolated Xvfb display and drive it with xdotool + ImageMagick screenshots, without ever touching the real desktop session. Use whenever you need to visually verify a GUI change in this repo.
---

# Running tuxmix-gui headlessly for visual testing

This sandbox has no authenticated access to the user's real X11/Wayland
session (`xdpyinfo` on the inherited `DISPLAY` fails with "Authorization
required"). Requires `xorg-server-xvfb`, `xdotool`, and `imagemagick`
(`import`) installed system-wide (`sudo pacman -S xorg-server-xvfb
xdotool`; `imagemagick` is usually already present).

## Gotchas discovered empirically

1. **The shell inherits `WAYLAND_DISPLAY`/`XDG_SESSION_TYPE` from the
   user's real session.** wgpu-hal's EGL backend prefers Wayland
   whenever `libwayland-client` can connect, *even if you set
   `DISPLAY` to the Xvfb screen* — `wl_display_connect(NULL)` falls
   back to the default `wayland-0` socket via `XDG_RUNTIME_DIR`
   regardless of `WAYLAND_DISPLAY`. Left unhandled, the app either
   crashes (`incompatible window kind` / `Invalid surface` panic) or —
   worse — actually opens a window on the user's real desktop.
   **Always launch with a scratch `XDG_RUNTIME_DIR` that has no
   `wayland-0` socket in it**, and unset the Wayland env vars, to force
   the GL/X11 path onto the Xvfb display.
2. **No window manager runs in bare Xvfb**, so the window never gets
   input focus automatically. `xdotool click`/`mousedown` land on the
   window but every button reads as hover-only (border lights up, but
   state never toggles) until you `xdotool windowfocus --sync
   <winid>` first. `windowactivate` will error
   ("`_NET_ACTIVE_WINDOW` not supported") — that's expected and
   harmless, `windowfocus` alone is sufficient.
3. **llvmpipe (software GL) needs `LIBGL_ALWAYS_SOFTWARE=1`** and
   forcing `WGPU_BACKEND=gl` (Vulkan enumerates a real adapter node
   but this sandbox has no accel permission on it — `amdgpu_query_info
   ACCEL_WORKING failed (-13)`).
4. Known rendering quirk under this llvmpipe/GL path: of several
   `Canvas` widgets in the same row (e.g. one `fader`/`pan_indicator`
   per channel strip), only the **last one built in widget-tree
   order** actually paints its `Frame::fill`/`stroke` geometry — the
   others show their `text()` siblings fine but the canvas itself is
   blank. Not confirmed whether this reproduces on a real GPU/Vulkan
   backend — cross-check with the user before treating it as an app
   bug. Doesn't block interaction testing: click/drag still works on
   widgets whose geometry isn't visibly painted, since input hit-testing
   uses layout bounds, not rendered pixels.

## Recipe

```bash
SCRATCH=<your scratchpad dir>   # e.g. from the harness system prompt
REPO=/home/iswad/DATA/05_Code/Projects/TuxMix

# 1. Start Xvfb once per session (leave it running across launches).
pkill -f "Xvfb :99" 2>/dev/null
nohup Xvfb :99 -screen 0 1280x800x24 > /tmp/xvfb99.log 2>&1 &
disown
sleep 1

# 2. A short scratch XDG_RUNTIME_DIR (must be short enough that
#    "<dir>/wayland-0" stays under the 108-byte AF_UNIX path limit —
#    that's what makes the Wayland connect attempt fail closed).
mkdir -p "$SCRATCH/fake-xdg-runtime" && chmod 700 "$SCRATCH/fake-xdg-runtime"

# 3. Build + launch against the virtual display only.
cd "$REPO" && cargo build --release -p tuxmix-gui
nohup env -u WAYLAND_DISPLAY -u XDG_SESSION_TYPE \
  XDG_RUNTIME_DIR="$SCRATCH/fake-xdg-runtime" \
  DISPLAY=:99 WINIT_UNIX_BACKEND=x11 \
  LIBGL_ALWAYS_SOFTWARE=1 WGPU_BACKEND=gl \
  ./target/release/tuxmix-gui --mock > /tmp/tuxmix-run.log 2>&1 &
disown
sleep 2
tail -5 /tmp/tuxmix-run.log   # confirm "Using X11 platform", no panic

# 4. Screenshot.
WID=$(DISPLAY=:99 xdotool search --name "TuxMix" | head -1)
DISPLAY=:99 import -window "$WID" "$SCRATCH/screenshot.png"

# 5. Interact — ALWAYS focus first.
DISPLAY=:99 xdotool windowfocus --sync "$WID"
DISPLAY=:99 xdotool mousemove --window "$WID" <x> <y>
DISPLAY=:99 xdotool click 1                 # or: mousedown 1 / mousemove.../ mouseup 1 for drags

# 6. Read the Read tool over the .png to actually look at it.

# Cleanup when done:
pkill -f "target/release/tuxmix-gui"
# (Xvfb can be left running for the rest of the session — cheap.)
```

To find exact click coordinates for a themed color you know (e.g.
`theme::ACCENT` = `#4FC3F7`), scan rows of the screenshot with
ImageMagick instead of eyeballing pixels:

```bash
for y in $(seq <y0> <y1>); do
  n=$(magick screenshot.png -crop 1280x1+0+$y txt:- 2>/dev/null | grep -ic "4FC3F7")
  [ "$n" -gt 0 ] && echo "y=$y matches=$n"
done
magick screenshot.png -crop 1280x1+0+<found_y> txt:- | grep -i "4FC3F7"
```
