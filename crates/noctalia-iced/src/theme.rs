//! Noctalia design tokens (noctalia-shell `src/ui/palette.cpp`, `src/ui/style.h`) and the widget
//! styles built on them.
//!
//! The sixteen colour roles live in a [`Palette`]. [`set_palette`] replaces the one in force, so an
//! application can follow the palette the user's Noctalia shell is actually running — the point of a
//! design language is that a theme change carries into every window. The constants below are
//! Noctalia's own defaults, and remain the palette until something replaces them.
//!
//! `primary` additionally travels in the iced [`Theme`] palette, where iced's own widgets find it.
//!
//! [`set_font`] does the same for typography. iced's default family is the literal name `Arial`,
//! and where that is missing the fallback is whatever the font database offers first — often a
//! serif. Tell the library which family the application runs in and every weight it draws, the
//! titlebar included, is built from that one answer.

use iced::border::{self, Border};
use iced::font::Weight;
use iced::widget::{
    button, checkbox, container, overlay::menu, pick_list, radio, scrollable, slider, text_editor, text_input,
};
use iced::{Background, Color, Font, Shadow, Theme, Vector, color};
use std::sync::RwLock;

pub const PRIMARY: Color = color!(0xfff59b);
pub const ON_PRIMARY: Color = color!(0x0e0e43);
pub const SECONDARY: Color = color!(0xa9aefe);
pub const TERTIARY: Color = color!(0x9bfece);
pub const ERROR: Color = color!(0xfd4663);
pub const SURFACE: Color = color!(0x070722);
pub const ON_SURFACE: Color = color!(0xf3edf7);
pub const SURFACE_VARIANT: Color = color!(0x11112d);
pub const ON_SURFACE_VARIANT: Color = color!(0x7c80b4);
pub const OUTLINE: Color = color!(0x21215f);
pub const HOVER: Color = color!(0x9bfece);
pub const ON_HOVER: Color = color!(0x0e0e43);
pub const ON_SECONDARY: Color = color!(0x0e0e43);
pub const ON_TERTIARY: Color = color!(0x0e0e43);
pub const ON_ERROR: Color = color!(0x0e0e43);
pub const SHADOW: Color = color!(0x000000);

/// Noctalia's sixteen colour roles. The names match the roles in a noctalia-shell palette file
/// (`mPrimary`, `mOnPrimary`, …), which is where an application usually reads them from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub primary: Color,
    pub on_primary: Color,
    pub secondary: Color,
    pub on_secondary: Color,
    pub tertiary: Color,
    pub on_tertiary: Color,
    pub error: Color,
    pub on_error: Color,
    pub surface: Color,
    pub on_surface: Color,
    pub surface_variant: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub shadow: Color,
    pub hover: Color,
    pub on_hover: Color,
}

/// Noctalia's own palette: the constants above, and the palette until [`set_palette`] replaces it.
pub const DEFAULT_PALETTE: Palette = Palette {
    primary: PRIMARY,
    on_primary: ON_PRIMARY,
    secondary: SECONDARY,
    on_secondary: ON_SECONDARY,
    tertiary: TERTIARY,
    on_tertiary: ON_TERTIARY,
    error: ERROR,
    on_error: ON_ERROR,
    surface: SURFACE,
    on_surface: ON_SURFACE,
    surface_variant: SURFACE_VARIANT,
    on_surface_variant: ON_SURFACE_VARIANT,
    outline: OUTLINE,
    shadow: SHADOW,
    hover: HOVER,
    on_hover: ON_HOVER,
};

static FONT: RwLock<Font> = RwLock::new(Font::DEFAULT);

/// The interface font. Defaults to iced's, which is worth replacing — see the module docs.
pub fn font() -> Font {
    match FONT.read() {
        Ok(font) => *font,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

/// Sets the family the library draws in. Pass the same font the application gives iced's
/// `default_font`, so the chrome matches the content.
pub fn set_font(font: Font) {
    match FONT.write() {
        Ok(mut current) => *current = font,
        Err(poisoned) => *poisoned.into_inner() = font,
    }
}

/// [`font`] at semibold. Building this from `Font::DEFAULT` instead is the trap: the weight changes
/// but the family reverts to iced's unresolved default, so headings come out in another typeface.
pub fn semibold() -> Font {
    Font { weight: Weight::Semibold, ..font() }
}

static PALETTE: RwLock<Palette> = RwLock::new(DEFAULT_PALETTE);

/// The palette in force. Every style function calls this, per widget per frame.
pub fn palette() -> Palette {
    // A panic while styling should not take the colours with it.
    match PALETTE.read() {
        Ok(palette) => *palette,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

/// Replaces the palette. Safe at any time: iced rebuilds the view after every message, so a change
/// made from `update` shows on the next frame. Rebuild the [`Theme`] from [`noctalia`] as well, since
/// iced keeps `primary` in its own palette.
pub fn set_palette(palette: Palette) {
    match PALETTE.write() {
        Ok(mut current) => *current = palette,
        Err(poisoned) => *poisoned.into_inner() = palette,
    }
}

pub const RADIUS_SM: f32 = 3.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 9.0;
pub const RADIUS_XL: f32 = 12.0;
pub const BORDER: f32 = 1.0;
/// The weight an element is marked with rather than outlined in: a selection bar, a live edge.
pub const BORDER_EMPHASIZED: f32 = 3.0;
pub const FOCUS_RING: f32 = 2.0;
pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const SPACE_LG: f32 = 16.0;
/// Inside a card or a panel, which want a little more room than [`SPACE_MD`] and less than a
/// doubled [`SPACE_SM`] would give the two axes separately.
pub const CARD_PADDING: f32 = 14.0;
pub const PANEL_PADDING: f32 = 14.0;
pub const FONT_MINI: f32 = 11.0;
pub const FONT_CAPTION: f32 = 13.0;
pub const FONT_BODY: f32 = 14.0;
pub const FONT_TITLE: f32 = 16.0;
pub const FONT_HEADER: f32 = 20.0;
pub const CONTROL_HEIGHT_SM: f32 = 32.0;
pub const CONTROL_HEIGHT: f32 = 38.0;
pub const CONTROL_HEIGHT_LG: f32 = 44.0;
const DISABLED_ALPHA: f32 = 0.5;

pub const ICON_FONT: Font = Font::with_name("noctalia-tabler-icons");
pub const ICON_FONT_BYTES: &[u8] = include_bytes!("../assets/noctalia-tabler.ttf");

/// Tabler codepoints (assets/fonts/tabler.json).
pub mod icon {
    pub const CHECK: char = '\u{ea5e}';
    pub const CHEVRON_DOWN: char = '\u{ea5f}';
    pub const CHEVRON_UP: char = '\u{ea62}';
    pub const MINUS: char = '\u{eaf2}';
    pub const PLUS: char = '\u{eb0b}';
    pub const SQUARE: char = '\u{eb2c}';
    pub const SQUARES: char = '\u{eef6}';
    pub const X: char = '\u{eb55}';
}

pub fn noctalia(primary: Color) -> Theme {
    Theme::custom(
        "Noctalia",
        iced::theme::Palette {
            background: palette().surface,
            text: palette().on_surface,
            primary,
            success: palette().tertiary,
            warning: palette().secondary,
            danger: palette().error,
        },
    )
}

pub fn primary(theme: &Theme) -> Color {
    theme.palette().primary
}

pub fn alpha(color: Color, factor: f32) -> Color {
    Color { a: color.a * factor, ..color }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Default,
    Primary,
    /// Borderless; the stepper's +/- and unselected segments.
    Tab,
    /// A selected segment.
    Selected,
}

/// Whether two roles resolve to the same colour, within what a channel can express.
fn indistinguishable(a: Color, b: Color) -> bool {
    let close = |x: f32, y: f32| (x - y).abs() * 255.0 < 1.0;
    close(a.r, b.r) && close(a.g, b.g) && close(a.b, b.b) && close(a.a, b.a)
}

/// The fill and text a control takes under the pointer, given what it already shows.
///
/// [`Palette::hover`] is the role for this, and is what it returns nearly always. But a palette is
/// free to set `hover` to the same colour as `primary` — Everforest-derived ones do — and then an
/// accent-filled button would have no hover at all: it would look identical whether the pointer was
/// on it or not. Where that happens the other accent stands in, so every control changes under the
/// pointer in every palette.
pub fn hover_pair(rest: Color) -> (Color, Color) {
    let palette = palette();
    if indistinguishable(palette.hover, rest) {
        (palette.secondary, palette.on_secondary)
    } else {
        (palette.hover, palette.on_hover)
    }
}

pub fn button_style(variant: ButtonVariant) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let primary = primary(theme);
        // What the button shows at rest is also what its hover has to differ from.
        let rest = match variant {
            ButtonVariant::Selected | ButtonVariant::Primary => (primary, Color::TRANSPARENT, palette().on_primary),
            ButtonVariant::Default => (palette().surface_variant, palette().outline, palette().on_surface),
            ButtonVariant::Tab => (Color::TRANSPARENT, Color::TRANSPARENT, palette().on_surface),
        };
        let (background, border_color, text_color) = match (variant, status) {
            // Selected is a state rather than an action, and stays put under the pointer.
            (ButtonVariant::Selected, _) => (primary, primary, palette().on_primary),
            (_, button::Status::Hovered) => {
                let (background, text) = hover_pair(rest.0);
                (background, Color::TRANSPARENT, text)
            }
            (_, button::Status::Pressed) => (primary, primary, palette().on_primary),
            (ButtonVariant::Default, button::Status::Disabled) => (
                alpha(palette().surface_variant, DISABLED_ALPHA),
                alpha(palette().outline, DISABLED_ALPHA),
                alpha(palette().on_surface, DISABLED_ALPHA),
            ),
            (ButtonVariant::Primary, button::Status::Disabled) => {
                (alpha(primary, DISABLED_ALPHA), Color::TRANSPARENT, palette().on_primary)
            }
            (ButtonVariant::Tab, button::Status::Disabled) => {
                (Color::TRANSPARENT, Color::TRANSPARENT, alpha(palette().on_surface, DISABLED_ALPHA))
            }
            _ => rest,
        };
        let width = if variant == ButtonVariant::Default { BORDER } else { 0.0 };
        button::Style {
            background: Some(background.into()),
            text_color,
            border: Border { color: border_color, width, radius: RADIUS_MD.into() },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

/// A button with no chrome of its own (collapsible headers).
pub fn bare_button(_: &Theme, _: button::Status) -> button::Style {
    button::Style { background: None, text_color: palette().on_surface, ..button::Style::default() }
}

/// The rounded surface behind segmented controls and steppers.
pub fn track(_: &Theme) -> container::Style {
    container::Style {
        background: Some(palette().surface_variant.into()),
        border: border::rounded(RADIUS_MD),
        ..container::Style::default()
    }
}

pub fn rule(_: &Theme) -> container::Style {
    container::Style { background: Some(palette().outline.into()), ..container::Style::default() }
}

pub fn checkbox_style(theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let (checked, hovered, disabled) = match status {
        checkbox::Status::Active { is_checked } => (is_checked, false, false),
        checkbox::Status::Hovered { is_checked } => (is_checked, true, false),
        checkbox::Status::Disabled { is_checked } => (is_checked, false, true),
    };
    let primary = primary(theme);
    let fade = if disabled { DISABLED_ALPHA } else { 1.0 };
    checkbox::Style {
        background: alpha(if checked { primary } else { palette().surface }, fade).into(),
        icon_color: palette().on_primary,
        border: Border {
            color: alpha(
                if hovered {
                    palette().hover
                } else if checked {
                    primary
                } else {
                    palette().outline
                },
                fade,
            ),
            width: BORDER,
            radius: RADIUS_SM.into(),
        },
        text_color: Some(palette().on_surface),
    }
}

pub fn radio_style(theme: &Theme, status: radio::Status) -> radio::Style {
    let (selected, hovered) = match status {
        radio::Status::Active { is_selected } => (is_selected, false),
        radio::Status::Hovered { is_selected } => (is_selected, true),
    };
    let primary = primary(theme);
    radio::Style {
        background: if selected { primary } else { palette().surface }.into(),
        dot_color: palette().on_primary,
        border_width: BORDER,
        border_color: if selected {
            primary
        } else if hovered {
            palette().hover
        } else {
            palette().outline
        },
        text_color: Some(palette().on_surface),
    }
}

pub fn slider_style(theme: &Theme, status: slider::Status) -> slider::Style {
    let hot = !matches!(status, slider::Status::Active);
    slider::Style {
        rail: slider::Rail {
            backgrounds: (primary(theme).into(), palette().outline.into()),
            width: 8.0,
            border: border::rounded(4.0),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 9.0 },
            background: palette().on_primary.into(),
            border_width: BORDER,
            border_color: if hot { palette().hover } else { palette().outline },
        },
    }
}

pub fn pick_list_style(_: &Theme, status: pick_list::Status) -> pick_list::Style {
    let hot = !matches!(status, pick_list::Status::Active);
    pick_list::Style {
        text_color: palette().on_surface,
        placeholder_color: alpha(palette().on_surface_variant, 0.7),
        handle_color: palette().on_surface_variant,
        background: palette().surface_variant.into(),
        border: Border {
            color: if hot { palette().hover } else { palette().outline },
            width: BORDER,
            radius: RADIUS_MD.into(),
        },
    }
}

pub fn menu_style(_: &Theme) -> menu::Style {
    menu::Style {
        background: palette().surface_variant.into(),
        border: Border { color: palette().outline, width: BORDER, radius: RADIUS_MD.into() },
        text_color: palette().on_surface,
        selected_text_color: palette().on_hover,
        selected_background: palette().hover.into(),
        shadow: Shadow { color: alpha(Color::BLACK, 0.45), offset: Vector::new(0.0, 4.0), blur_radius: 16.0 },
    }
}

pub fn scrollable_style(_: &Theme, status: scrollable::Status) -> scrollable::Style {
    let hot = !matches!(status, scrollable::Status::Active { .. });
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(alpha(palette().on_surface_variant, if hot { 0.85 } else { 0.55 })),
            border: border::rounded(RADIUS_SM),
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: palette().surface_variant.into(),
            border: border::rounded(RADIUS_MD).color(palette().outline).width(BORDER),
            shadow: Shadow::default(),
            icon: palette().on_surface,
        },
    }
}

/// A multi-line field. Same edge and same selection as [`text_input_style`], so a form that has
/// both does not look like it was assembled from two kits.
pub fn text_editor_style(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let focused = matches!(status, text_editor::Status::Focused { .. });
    text_editor::Style {
        background: palette().surface_variant.into(),
        border: Border {
            color: if focused { palette().hover } else { palette().outline },
            width: BORDER,
            radius: RADIUS_MD.into(),
        },
        placeholder: alpha(palette().on_surface_variant, 0.7),
        value: palette().on_surface,
        selection: alpha(primary(theme), 0.35),
    }
}

pub fn text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: palette().surface_variant.into(),
        border: Border {
            color: if focused { palette().hover } else { palette().outline },
            width: BORDER,
            radius: RADIUS_MD.into(),
        },
        icon: palette().on_surface_variant,
        placeholder: alpha(palette().on_surface_variant, 0.7),
        value: palette().on_surface,
        selection: alpha(primary(theme), 0.35),
    }
}
