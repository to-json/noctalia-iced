# noctalia-iced

Noctalia's design language for [iced](https://iced.rs) 0.14: the palette roles and style tokens of
[noctalia-shell](https://github.com/noctalia-dev/noctalia-shell), Noctalia's controls built from iced
widgets and canvas programs, and window chrome that looks like the rest of a Noctalia desktop. No
shaders or SDFs; everything is stock iced.

| Path | What |
|---|---|
| `crates/noctalia-iced` | The library: `theme`, `widgets`, `range_slider`, `chrome`, the Tabler icon font |
| `examples/clock` | `noctalia-clock-iced`: libnoctalia-ui's clock demo on the library, with `iced_test` UI tests |
| `third_party/rust` | Optional winit, iced_core and iced_winit patches for Wayland chrome ([`PATCHES.md`](third_party/rust/PATCHES.md)) |
| `nix/wayland-check.nix` | The clock under headless weston: screenshots, a compositor capture, the protocol trace |

## Using the library

```rust
use noctalia_iced::{chrome, theme, widgets};

iced::application(App::new, App::update, App::view)
    .theme(|app: &App| theme::noctalia(app.accent))
    .font(theme::ICON_FONT_BYTES)
    .window(chrome::settings(Size::new(900.0, 700.0), Size::new(640.0, 480.0), "org.example.App"))
    .run()
```

`theme::hover_pair` is the one piece of logic in here rather than a lookup: `hover` is the role for
what a control shows under the pointer, but a palette may set it to the same colour as `primary`
(Everforest-derived ones do), which would leave an accent-filled button looking identical hovered or
not. Where that happens the other accent stands in, so every control changes under the pointer in
every palette. `button_style` uses it.

Style iced widgets with the `theme::*_style` functions, compose the Noctalia controls from
`widgets`, and wrap the view in `chrome::frame` (forwarding `chrome::events()` and
`chrome::perform`); `examples/clock/src/app.rs` does all three.

### Palette

The sixteen colour roles live in `theme::Palette`. `theme::palette()` returns the one in force and
`theme::set_palette` replaces it, so an application can follow the palette the user's Noctalia shell
is actually running instead of the constants compiled in here:

```rust
theme::set_palette(theme::Palette { primary, surface, on_surface, ..theme::DEFAULT_PALETTE });
```

Every style function reads it, so a change reaches the window chrome and the controls alike on the
next frame; call it again from `update` to follow a live theme change. Until something replaces it
the palette is `theme::DEFAULT_PALETTE`. The `theme::PRIMARY`-style constants are those defaults, not
the live values — application code that should follow the user's theme reads `theme::palette()`.

### Motion

`motion` carries noctalia-shell's animation durations (`src/ui/style.h`: `animFast` 100 ms,
`animNormal` 200 ms, `animSlow` 400 ms) and the curves this library animates on. noctalia-shell
animates through Qt, whose easing curves have no iced equivalent to port, so the curves are this
library's own. Noctalia's motion is *soft*: things arrive slightly past where they are going and
settle back, rather than snapping or bouncing.

| Curve | Overshoot | For |
|---|---|---|
| `SPRING` | ~7% | Short travel: indicators, badges, avatars, anything under ~60 px |
| `SETTLE` | ~2% | Whole surfaces: panes arriving, banners opening |
| `GLIDE` | none | Colour, and anything that must land exactly |

`motion::spring_animation` and its two siblings build an `iced::Animation` already on the right
curve and duration. `motion::Replay` is the one-shot counterpart: an arrival that plays again from
the beginning every time its content changes, where an `Animation` would transition from wherever it
had got to. `mix`, `lerp` and `stagger` are the value helpers.

```rust
let mut selected = motion::spring_animation(0.0_f32);   // moves from wherever it is
selected.go_mut(3.0, now);

let mut arrival = motion::Replay::settled(motion::SETTLE, motion::NORMAL);
arrival.restart(now);                                   // plays again from the start
```

Every constructor honours `motion::reduced()`, so setting `NOCTALIA_REDUCE_MOTION` collapses each
animation to a millisecond and an application built on these gets a reduced-motion mode for free.
`NOCTALIA_MOTION_SCALE` multiplies every duration (0.1–20): an overshoot that is too much at 200 ms
is obvious at 2 s, and invisible in a screenshot at either.

iced has no opacity, and no transform that survives clipping — `float` moves its content into an
overlay, where it draws over its neighbours. So the vocabulary here is layout and colour: animate
padding, width and height, and blend towards the surface colour with `mix`. The renderer clips and
hit-tests the result exactly as it does a still frame.

### Window chrome

- **Linux:** client-side titlebar, frame and resize grips in Noctalia's style.
- **macOS:** the native frame with a transparent titlebar (AppKit can't interactively resize a
  borderless window); the title row is drawn under the traffic lights.
- **Elsewhere:** native decorations.

### Features

- `wayland-chrome` (off by default). On Wayland the frame gets its rounded corners and shadow in a
  margin outside the xdg window geometry (tiling, snapping and resize edges see the real window),
  the input region extends a resize band past it, and the frame squares off when the compositor
  reports the window maximized, fullscreen or tiled. It needs the patched crates in
  `third_party/rust`; in your own workspace, add a `[patch.crates-io]` for `winit`, `iced_core`
  and `iced_winit` pointing at them.

Without the feature noctalia-iced builds against crates.io iced and winit: the Linux frame fills the
surface (no shadow margin) and only the maximized state is tracked.

Turning the feature on or off needs no application code changes, as long as the application:

- names the window state `noctalia_iced::chrome::Chrome`, not `iced::window::Chrome` (which only
  exists in the patched iced);
- matches `iced::window::Event` with a `_` arm, since the patched iced adds `ChromeChanged`;
- builds Linux `window::Settings::platform_specific` with `..Default::default()`, since the patched
  iced adds `shadow_margin`.

The last two apply to the whole application once the patch is in place, whether or not
noctalia-iced's feature is enabled.

## Build and test

```sh
cargo run --release -p noctalia-clock-iced -- --fixed-time 2026-09-13T10:08:30Z --zone UTC
cargo test --workspace

# With the patched crates (applied through --config; Cargo.lock is restored afterwards)
scripts/with-patches.sh run --release -p noctalia-clock-iced --features wayland-chrome
scripts/with-patches.sh test --workspace --features noctalia-clock-iced/wayland-chrome

# Linux checks (Docker on macOS); output in out/<check>
scripts/linux.sh clock-wayland          # wayland-chrome
scripts/linux.sh clock-wayland-stock    # crates.io iced and winit
scripts/linux.sh chrome-probe           # the windowing patches alone
```

`nix develop` gives a Linux devshell; `nix build` builds the clock (`.#noctalia-clock-iced-chrome`
with the feature). After changing dependencies, `scripts/patched-lock.sh` refreshes
`third_party/rust/Cargo.patched.lock`, the lockfile the patched Nix build uses.

## Known issues

- Mesa's software rasterizers (llvmpipe GL and lavapipe Vulkan, as in the headless weston check)
  drop MSAA canvas geometry and canvas text once the surface is taller than about 1024px; width
  doesn't matter. Tall windows render correctly with `--no-antialiasing`, and on Metal with MSAA.
  Hardware Linux drivers are untested.
- Wayland chrome is verified in headless weston only: tiled state, un-maximizing, fractional scale
  and interactive resize in the shadow band are untested.
- The `iced_test` snapshots were recorded on macOS/Metal; delete `examples/clock/snapshots/` to
  re-baseline on another renderer.

## Provenance

Extracted from libnoctalia-ui (`examples/clock-iced` and `third_party/rust` at `ea8e6ab`), where the
C++ toolkit and its clock demo live. See `NOTICE` for bundled components.
