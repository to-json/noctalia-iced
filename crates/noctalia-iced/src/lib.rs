//! Noctalia's design language for [iced](https://iced.rs).
//!
//! - [`theme`]: palette roles and style tokens from noctalia-shell, and style functions for iced's
//!   widgets. `primary` travels in the iced [`Theme`](iced::Theme) palette, so restyling the accent
//!   is `theme::noctalia(color)`.
//! - [`motion`]: noctalia-shell's animation durations, and the soft spring curves this library
//!   animates on. Honours a reduced-motion environment.
//! - [`keymap`]: table-driven key sequences with counts and prefixes, for a modal interface.
//! - [`fuzzy`]: an `fzf`-shaped match scorer.
//! - [`picker`]: a fuzzy-filtered overlay — a command palette or a quick-open, over any `T`.
//! - [`list`]: a windowed list, for a list too long to lay out all of.
//! - [`widgets`]: Noctalia controls iced has no direct equivalent for (segmented, toggle, stepper,
//!   collapsible, countdown ring, spinner, graph, colour picker) plus the settings-row helpers.
//! - [`range_slider()`]: a two-handle slider.
//! - [`chrome`]: window chrome. Client-side on Linux, native frame on macOS.
//!
//! Register [`theme::ICON_FONT_BYTES`] with the application (`.font(...)`) for the icon glyphs.
//!
//! # Features
//!
//! - `wayland-chrome`: Wayland chrome with the shadow outside the window geometry and the
//!   compositor's maximized, fullscreen and tiled states. It uses `window::Chrome`,
//!   `window::Event::ChromeChanged` and the Linux `shadow_margin` setting from the patched
//!   iced_core, iced_winit and winit in this repository's `third_party/rust`, so applications
//!   enabling it must `[patch.crates-io]` those crates. Without it the crate builds against
//!   crates.io iced.

pub mod chrome;
pub mod fuzzy;
pub mod keymap;
pub mod list;
pub mod motion;
pub mod picker;
pub mod range_slider;
pub mod theme;
pub mod widgets;

pub use range_slider::{RangeSlider, range_slider};
