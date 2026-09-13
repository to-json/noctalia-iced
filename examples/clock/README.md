# noctalia-clock-iced

libnoctalia-ui's `noctalia-clock` demo on iced and `noctalia-iced`: the same controls and
behaviour, drawn with iced widgets and canvas programs.

| File | What |
|---|---|
| `src/app.rs` | State, `Message`, `update`, `view`, `subscription` (seconds aligned to the wall clock, frames while animating) |
| `src/face.rs` | Clock face: analog dial on a canvas, digital readout from text widgets |
| `src/confetti.rs` | Seeded confetti particles on a canvas |
| `src/tests.rs` | `iced_test` UI tests following the C++ demo's `basic.nuis`, plus snapshots in `snapshots/` |
| `reference/` | Snapshots of the C++ demo in a window tall enough to show every control (`tall.nuis`) |

```sh
cargo run --release -p noctalia-clock-iced -- --fixed-time 2026-09-13T10:08:30Z --zone UTC
scripts/with-patches.sh run --release -p noctalia-clock-iced --features wayland-chrome   # Wayland chrome
```

`--screenshot OUT.png [--after-frames N]` writes one frame and exits; `--mode`, `--advanced` and
`--confetti` set up the states the C++ e2e snapshots cover; `--no-antialiasing` turns off MSAA.

## Deliberate differences from the C++ demo

- Controls keep the 38px `controlHeight`; the C++ scroll view stretches buttons and the segmented
  control to 60–76px. Setting rows keep its 70px rhythm.
- Setting labels are the 120px the C++ code asks for; the C++ layout ignores that width.
- The iced radio and checkbox are restyled rather than redrawn, so their geometry follows iced; the
  toggle is composed to match Noctalia's 40×24 medium size.
