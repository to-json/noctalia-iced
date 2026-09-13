//! Platform specific settings for Linux.

/// The platform specific window settings of an application.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlatformSpecific {
    /// Sets the application id of the window.
    ///
    /// As a best practice, it is suggested to select an application id that match
    /// the basename of the application’s .desktop file.
    pub application_id: String,

    /// Whether bypass the window manager mapping for x11 windows
    ///
    /// This flag is particularly useful for creating UI elements that need precise
    /// positioning and immediate display without window manager interference.
    pub override_redirect: bool,

    /// Wayland: a transparent margin, in logical pixels, kept around the window geometry while
    /// the window floats and decorations are off, for a client-drawn shadow.
    ///
    /// The window size in [`Settings`](crate::window::Settings) stays the geometry size; the
    /// reported window size includes the margin. See
    /// [`Event::ChromeChanged`](crate::window::Event::ChromeChanged).
    pub shadow_margin: u32,
}
