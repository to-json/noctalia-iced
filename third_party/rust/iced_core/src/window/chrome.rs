/// The window states that decide how client-drawn chrome (frame, shadow, corners) should look.
///
/// Reported through [`Event::ChromeChanged`](crate::window::Event::ChromeChanged).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Chrome {
    /// The window is maximized.
    pub maximized: bool,
    /// The window is fullscreen.
    pub fullscreen: bool,
    /// The window is tiled against at least one edge.
    pub tiled: bool,
    /// The transparent margin, in logical pixels, currently kept around the window geometry
    /// for a client-drawn shadow. `0` unless the window floats.
    pub shadow_margin: f32,
    /// The band, in logical pixels, outside the window geometry that still receives pointer
    /// input, for resize handles. `0` unless the window floats.
    pub resize_band: f32,
}

impl Chrome {
    /// Whether the window floats: not maximized, fullscreen or tiled.
    pub fn is_floating(&self) -> bool {
        !(self.maximized || self.fullscreen || self.tiled)
    }
}
