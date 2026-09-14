//! Window chrome in the Noctalia style.
//!
//! - Linux: client-side decorations: titlebar, frame, resize grips.
//!   - With the `wayland-chrome` feature (and the patched crates in third_party/rust) a Wayland
//!     window keeps its shadow in a margin outside the xdg window geometry, and [`Chrome`] follows
//!     the compositor's maximized, fullscreen and tiled states so the frame squares off.
//!   - Without it the frame fills the surface (no shadow margin) and only the maximized state is
//!     tracked, by querying the window after each resize.
//! - macOS: the native frame (AppKit can't start an interactive resize from a borderless window)
//!   with a transparent titlebar over the content; only the title row is drawn here.
//! - Elsewhere: native decorations.

use crate::theme;
use crate::widgets::icon;
use iced::border::{self, Border};
use iced::mouse::Interaction;
use iced::widget::{button, center, column, container, mouse_area, row, space, stack, text};
use iced::window::{self, Direction};
use iced::{Alignment, Color, Element, Length, Padding, Shadow, Size, Subscription, Task, Vector};

#[cfg(feature = "wayland-chrome")]
pub use iced::window::Chrome;

/// The window state the frame follows (the patched iced's `window::Chrome` with `wayland-chrome`).
#[cfg(not(feature = "wayland-chrome"))]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Chrome {
    pub maximized: bool,
    pub fullscreen: bool,
    pub tiled: bool,
    /// Transparent margin around the frame that the shadow is painted into.
    pub shadow_margin: f32,
    /// Width of the input band outside the frame that still starts resizes.
    pub resize_band: f32,
}

#[cfg(not(feature = "wayland-chrome"))]
impl Chrome {
    pub fn is_floating(&self) -> bool {
        !(self.maximized || self.fullscreen || self.tiled)
    }
}

pub const TITLEBAR: f32 = 40.0;
const CLIENT_SIDE: bool = cfg!(target_os = "linux");
const MACOS: bool = cfg!(target_os = "macos");
#[cfg(all(target_os = "linux", feature = "wayland-chrome"))]
const SHADOW_MARGIN: u32 = 16;
/// Resize grab inside the frame edge, on top of the band the compositor still routes to us outside it.
const INNER_GRAB: f32 = 4.0;
const TRAFFIC_LIGHTS: f32 = 78.0;

#[derive(Debug, Clone)]
pub enum Action {
    Drag,
    Resize(Direction),
    ToggleMaximize,
    Minimize,
    Close,
    SystemMenu,
    /// The window was resized; without `wayland-chrome` its maximized state is re-queried.
    Resized,
    Changed(Chrome),
}

/// Settings for a window whose content area (below the titlebar) is `content`.
pub fn settings(content: Size, min: Size, app_id: &str) -> window::Settings {
    let titlebar = if CLIENT_SIDE || MACOS { TITLEBAR } else { 0.0 };
    #[allow(unused_mut)]
    let mut settings = window::Settings {
        size: Size::new(content.width, content.height + titlebar),
        min_size: Some(Size::new(min.width, min.height + titlebar)),
        decorations: !CLIENT_SIDE,
        transparent: CLIENT_SIDE,
        ..window::Settings::default()
    };
    #[cfg(target_os = "linux")]
    {
        settings.platform_specific.application_id = app_id.into();
        #[cfg(feature = "wayland-chrome")]
        {
            settings.platform_specific.shadow_margin = SHADOW_MARGIN;
        }
    }
    #[cfg(target_os = "macos")]
    {
        settings.platform_specific.title_hidden = true;
        settings.platform_specific.titlebar_transparent = true;
        settings.platform_specific.fullsize_content_view = true;
    }
    let _ = app_id;
    settings
}

/// The chrome a new window has before its first [`Chrome`] event: floating with the shadow margin
/// on Wayland with `wayland-chrome` (so the first frame is laid out like the rest), no margin
/// otherwise.
pub fn initial() -> Chrome {
    #[cfg(all(target_os = "linux", feature = "wayland-chrome"))]
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return Chrome {
            shadow_margin: SHADOW_MARGIN as f32,
            resize_band: SHADOW_MARGIN.min(10) as f32,
            ..Chrome::default()
        };
    }
    Chrome::default()
}

/// The surface clear colour: transparent where the shadow margin is ours to paint.
pub fn background() -> Color {
    if CLIENT_SIDE { Color::TRANSPARENT } else { theme::palette().surface }
}

/// Window-state updates for [`perform`].
#[cfg(feature = "wayland-chrome")]
pub fn events() -> Subscription<Action> {
    iced::event::listen_with(|event, _, _| match event {
        iced::Event::Window(window::Event::ChromeChanged(chrome)) => Some(Action::Changed(chrome)),
        _ => None,
    })
}

/// Window-state updates for [`perform`].
#[cfg(not(feature = "wayland-chrome"))]
pub fn events() -> Subscription<Action> {
    if CLIENT_SIDE { window::resize_events().map(|_| Action::Resized) } else { Subscription::none() }
}

pub fn perform(action: Action) -> Task<Action> {
    match action {
        Action::Drag => window::latest().and_then(window::drag),
        Action::Resize(direction) => window::latest().and_then(move |id| window::drag_resize(id, direction)),
        Action::ToggleMaximize => window::latest().and_then(window::toggle_maximize),
        Action::Minimize => window::latest().and_then(|id| window::minimize(id, true)),
        Action::Close => window::latest().and_then(window::close),
        Action::SystemMenu => window::latest().and_then(window::show_system_menu),
        Action::Resized => window::latest()
            .and_then(window::is_maximized)
            .map(|maximized| Action::Changed(Chrome { maximized, ..Chrome::default() })),
        Action::Changed(_) => Task::none(),
    }
}

/// Wraps the window content in the titlebar and frame.
pub fn frame<'a, M: 'a>(
    chrome: Chrome,
    title: &'a str,
    content: impl Into<Element<'a, M>>,
    on_action: fn(Action) -> M,
) -> Element<'a, M> {
    dressed(chrome, titlebar(chrome, title, None, on_action), content, on_action)
}

/// [`frame`] with the application's own controls between the title and the window buttons.
///
/// `leading` emits the application's messages rather than an [`Action`], which is the whole reason
/// this exists: a control that switches what the window is showing is not a window operation. The
/// drag region is split around it, so an empty titlebar still drags from everywhere it used to.
pub fn frame_with<'a, M: 'a>(
    chrome: Chrome,
    title: &'a str,
    leading: impl Into<Element<'a, M>>,
    content: impl Into<Element<'a, M>>,
    on_action: fn(Action) -> M,
) -> Element<'a, M> {
    dressed(chrome, titlebar(chrome, title, Some(leading.into()), on_action), content, on_action)
}

fn dressed<'a, M: 'a>(
    chrome: Chrome,
    titlebar: Element<'a, M>,
    content: impl Into<Element<'a, M>>,
    on_action: fn(Action) -> M,
) -> Element<'a, M> {
    if !CLIENT_SIDE && !MACOS {
        return content.into();
    }
    let body = column![titlebar, content.into()];
    if MACOS {
        return body.into();
    }

    let floating = chrome.is_floating();
    let surface = container(body).width(Length::Fill).height(Length::Fill).style(move |_| container::Style {
        background: Some(theme::palette().surface.into()),
        text_color: Some(theme::palette().on_surface),
        border: if floating {
            border::rounded(theme::RADIUS_XL).color(theme::palette().outline).width(theme::BORDER)
        } else {
            Border::default()
        },
        shadow: if floating {
            Shadow { color: theme::alpha(Color::BLACK, 0.5), offset: Vector::new(0.0, 3.0), blur_radius: 14.0 }
        } else {
            Shadow::default()
        },
        snap: true,
    });
    let framed = container(surface).padding(chrome.shadow_margin);
    if floating { stack![framed, resize_handles(chrome).map(on_action)].into() } else { framed.into() }
}

/// The titlebar: the title, whatever the application put beside it, and the window controls.
///
/// The parts that operate the window are built as [`Action`] and mapped, so an application whose
/// own message type is not `Clone` still gets a draggable titlebar — `mouse_area` needs `Clone`,
/// and `Action` has it.
fn titlebar<'a, M: 'a>(
    chrome: Chrome,
    title: &'a str,
    leading: Option<Element<'a, M>>,
    on_action: fn(Action) -> M,
) -> Element<'a, M> {
    let label =
        text(title).size(theme::FONT_CAPTION).font(theme::semibold()).color(theme::palette().on_surface_variant);
    let lead = if MACOS { TRAFFIC_LIGHTS } else { 14.0 };
    // Every part of the bar that is not a button drags the window, so the region is built twice:
    // once hugging the title, once filling whatever is left after the application's controls.
    let grip = |content: Element<'a, Action>, width: Length| -> Element<'a, Action> {
        mouse_area(container(content).center_y(Length::Fill).width(width))
            .on_press(Action::Drag)
            .on_double_click(Action::ToggleMaximize)
            .on_right_press(Action::SystemMenu)
            .into()
    };
    let titled = container(label).padding(Padding { left: lead, right: theme::SPACE_SM, ..Padding::ZERO });

    let mut bar = row![].height(TITLEBAR).align_y(Alignment::Center);
    match leading {
        // With nothing beside it the title's own region fills the bar, exactly as it always did.
        None => bar = bar.push(grip(titled.into(), Length::Fill).map(on_action)),
        Some(leading) => {
            bar = bar.push(grip(titled.into(), Length::Shrink).map(on_action));
            bar = bar.push(leading);
            bar = bar.push(grip(space().into(), Length::Fill).map(on_action));
        }
    }
    if CLIENT_SIDE {
        let restore = if chrome.maximized { theme::icon::SQUARES } else { theme::icon::SQUARE };
        let controls = row![
            capsule(theme::icon::MINUS, Action::Minimize, false),
            capsule(restore, Action::ToggleMaximize, false),
            capsule(theme::icon::X, Action::Close, true),
        ]
        .spacing(theme::SPACE_XS)
        .padding([0.0, 6.0]);
        bar = bar.push(Element::from(controls).map(on_action));
    }
    bar.into()
}

/// A titlebar button: 28x28, bare at rest, the hover role under the pointer (error for close).
///
/// Public because a switcher beside the title should be the same button as the ones beside Close,
/// down to the corner radius — it is the one control the titlebar has.
pub fn capsule<'a, M: Clone + 'a>(glyph: char, message: M, close: bool) -> Element<'a, M> {
    button(center(icon(glyph, theme::FONT_BODY)))
        .width(28)
        .height(28)
        .padding(0)
        .style(move |theme, status| {
            let (background, text_color) = match status {
                button::Status::Hovered if close => (theme::palette().error, theme::palette().on_hover),
                button::Status::Hovered => (theme::palette().hover, theme::palette().on_hover),
                button::Status::Pressed => (theme::primary(theme), theme::palette().on_primary),
                _ => (Color::TRANSPARENT, theme::palette().on_surface_variant),
            };
            button::Style {
                background: Some(background.into()),
                text_color,
                border: border::rounded(theme::RADIUS_LG),
                ..button::Style::default()
            }
        })
        .on_press(message)
        .into()
}

/// Invisible edge and corner grabs straddling the frame edge.
fn resize_handles<'a>(chrome: Chrome) -> Element<'a, Action> {
    let grab = chrome.resize_band + INNER_GRAB;
    let corner = grab * 2.0;
    let strip = |direction: Direction, interaction: Interaction, width: Length, height: Length| {
        mouse_area(space().width(width).height(height)).interaction(interaction).on_press(Action::Resize(direction))
    };
    let fixed = Length::Fixed;
    let handles = column![
        row![
            strip(Direction::NorthWest, Interaction::ResizingDiagonallyDown, fixed(corner), fixed(grab)),
            strip(Direction::North, Interaction::ResizingVertically, Length::Fill, fixed(grab)),
            strip(Direction::NorthEast, Interaction::ResizingDiagonallyUp, fixed(corner), fixed(grab)),
        ],
        row![
            strip(Direction::West, Interaction::ResizingHorizontally, fixed(grab), Length::Fill),
            space().width(Length::Fill),
            strip(Direction::East, Interaction::ResizingHorizontally, fixed(grab), Length::Fill),
        ]
        .height(Length::Fill),
        row![
            strip(Direction::SouthWest, Interaction::ResizingDiagonallyUp, fixed(corner), fixed(grab)),
            strip(Direction::South, Interaction::ResizingVertically, Length::Fill, fixed(grab)),
            strip(Direction::SouthEast, Interaction::ResizingDiagonallyDown, fixed(corner), fixed(grab)),
        ],
    ];
    container(handles).padding(chrome.shadow_margin - chrome.resize_band).into()
}
