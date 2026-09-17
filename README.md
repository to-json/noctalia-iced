# noctalia-iced

what if your iced apps matched your noctalia shell

| Path | What |
|---|---|
| `crates/noctalia-iced` | library |
| `examples/clock` | `noctalia-clock-iced`, the reference app |
| `third_party/rust` | Optional winit, iced_core and iced_winit patches for Wayland chrome |
| `nix/wayland-check.nix` | clock-based integration test |

## Using the library

```rust
use noctalia_iced::{chrome, theme, widgets};

iced::application(App::new, App::update, App::view)
    .theme(|app: &App| theme::noctalia(app.accent))
    .font(theme::ICON_FONT_BYTES)
    .window(chrome::settings(Size::new(900.0, 700.0), Size::new(640.0, 480.0), "org.example.App"))
    .run()
```

you can pass an accent color that isn't used for a ton, but otherwise you get:

### Palette

the palette passed by the noctalia template. you may need to go enable this in noctalia for your
app. sometime soon (possibly already) this will be one template for all the noctalia-iced apps
on a box

```rust
theme::set_palette(theme::Palette { primary, surface, on_surface, ..theme::DEFAULT_PALETTE });
```

### Motion

we got some lil' motion guides

| Curve | Overshoot | For |
|---|---|---|
| `SPRING` | ~7% | lil stuff |
| `SETTLE` | ~2% | big stuff |
| `GLIDE` | none | precise stuff |


```rust
let mut selected = motion::spring_animation(0.0_f32);   // moves from wherever it is
selected.go_mut(3.0, now);

let mut arrival = motion::Replay::settled(motion::SETTLE, motion::NORMAL);
arrival.restart(now);                                   // plays again from the start
```

### Keys

We got some kinda convoluted key handling available. you may prefer to just use iced here;
the idea was maybe you want to integrate with noctalia keyboard shortcuts somehow, but, it's
not there yet

### Long lists

lil pagination trick that came in handy

```rust
let window = list::window(rows.len(), pitch, offset, viewport);
let body = list::windowed(window, pitch, gap, window.range().map(|i| row(i)));
// `list::reveal` is the shortest scroll that brings a row fully into view, or None.
```


## Build and test

```sh
cargo run --release -p noctalia-clock-iced -- --fixed-time 2026-09-13T10:08:30Z --zone UTC
cargo test --workspace

# With the patched crates (applied through --config; Cargo.lock is restored afterwards)
scripts/with-patches.sh run --release -p noctalia-clock-iced --features wayland-chrome
scripts/with-patches.sh test --workspace --features noctalia-clock-iced/wayland-chrome

# Linux checks (Docker on macOS); output in out/<check>
scripts/linux.sh clock-wayland          # patched
scripts/linux.sh clock-wayland-stock    # from crates.io
scripts/linux.sh chrome-probe           # the windowing patches alone
```

`nix develop` devshell; 
`nix build` build the clock 

## Provenance

so i stole this whole style wholesale from noctalia's source. i'm not sure what to do here;
i needed a way to make apps that match, i don't want to push them in a direction. so, it's just,
here.

I don't feel like rewriting the following llm generated text, nor deleting it.

begin

### Window chrome

- **Linux:** client-side titlebar, frame and resize grips in Noctalia's style.
- **macOS:** the native frame with a transparent titlebar (AppKit can't interactively resize a
  borderless window); the title row is drawn under the traffic lights.
- **Elsewhere:** native decorations.

`chrome::frame` wraps the window content. `chrome::frame_with` is the same with the application's
own controls between the title and the window buttons — a surface switcher, say — built from
`chrome::capsule`, the 28x28 button the titlebar is already made of. The drag region splits around
them, so an empty titlebar still drags from everywhere.

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

end
