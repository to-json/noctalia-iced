# Patched Rust crates

Vendored copies of crates.io releases with small patches for client-drawn window chrome on
Wayland: a shadow margin kept outside the xdg window geometry, and the window states (maximized,
fullscreen, tiled) reported to the application so it can square its corners and drop the shadow.

| Crate | Version | License |
|---|---|---|
| `winit` | 0.30.13 | Apache-2.0 (`winit/LICENSE`) |
| `iced_core` | 0.14.0 | MIT (declared in `Cargo.toml`; the published crate ships no LICENSE file) |
| `iced_winit` | 0.14.1 | MIT (declared in `Cargo.toml`; the published crate ships no LICENSE file) |

They are optional: noctalia-iced builds against crates.io iced and winit unless its
`wayland-chrome` feature is enabled, and only that feature uses the patched API.

- `patch.toml` applies them as a Cargo `--config` file; `scripts/with-patches.sh <cargo args>`
  runs cargo with it and restores `Cargo.lock` afterwards.
- `Cargo.patched.lock` is the workspace lockfile with the patches applied, for the Nix
  `wayland-chrome` build (`scripts/patched-lock.sh` regenerates it).
- Another workspace uses them with its own `[patch.crates-io]` for `winit`, `iced_core` and
  `iced_winit`.

## winit (Wayland backend only)

- `WindowAttributesExtWayland::with_shadow_margin(margin: u32)`: a transparent margin, in logical
  pixels, around the window geometry. It applies to windows created without decorations while
  they float (not maximized, fullscreen or tiled):
  - `xdg_surface.set_window_geometry(m, m, w - 2m, h - 2m)`; the surface is the geometry plus 2m.
  - Configure sizes, and requested inner, min and max sizes, are geometry sizes. `inner_size()`
    and `Resized` report the surface size, which is what the application lays out.
  - The input region is the geometry grown by `min(m, 10)` pixels: resize handles keep working
    and clicks on the rest of the shadow reach the windows below. `set_cursor_hittest(false)`
    still empties it.
  - Maximized, fullscreen or tiled: no margin, geometry and input region cover the surface.
- `WindowEvent::ChromeStateChanged { maximized, fullscreen, tiled, shadow_margin, resize_band }`,
  emitted after the first configure and whenever a field changes, before the matching `Resized`.
  Never emitted on other platforms.
- No CSD frame (sctk-adwaita) is created while decorations are off; the hidden frame used to
  cost five subsurfaces. Turning decorations on later creates it on the next configure.
- Why not upstream-first: winit 0.30 is in maintenance while 0.31 (Window as a trait) is in beta,
  and iced 0.14 pins 0.30.

Known limits: the margin is fixed at creation, so toggling decorations at runtime keeps it.

## iced_core

- `window::Chrome { maximized, fullscreen, tiled, shadow_margin: f32, resize_band: f32 }` with
  `is_floating()`, re-exported as `iced::window::Chrome`.
- `window::Event::ChromeChanged(Chrome)`. No iced 0.14 crate matches `window::Event`
  exhaustively, so nothing else needs patching.
- Linux `window::settings::PlatformSpecific::shadow_margin: u32` (default 0).

## iced_winit

- Passes `platform_specific.shadow_margin` to `with_shadow_margin` (Linux, `wayland` feature).
- Converts `ChromeStateChanged` into `window::Event::ChromeChanged`.

## Verifying

`chrome-probe/` is a 600x400 window with a 16px margin. `scripts/linux.sh chrome-probe` runs it under
headless weston twice (floating, and maximized after the first chrome event) with `WAYLAND_DEBUG=1`;
outputs land in `out/chrome-probe/{floating,maximized}/` (`protocol.txt` is the filtered trace).

## Regenerating the patches

`patches/*.patch` are `diff -ruN` against the pristine crates. After changing a vendored crate run
`third_party/rust/regen-patches.sh`. To move to a new upstream release, copy the pristine crate
from `~/.cargo/registry/src/*/`, apply the old patch with `patch -p1 -d third_party/rust`, and fix
the rejects, then run `scripts/patched-lock.sh`.
