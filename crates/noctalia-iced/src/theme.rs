//! Noctalia design tokens (noctalia-shell `src/ui/palette.cpp`, `src/ui/style.h`) and the widget
//! styles built on them.
//!
//! Only `primary` is dynamic (the accent picker rewrites it, like `set_palette_role("primary")`), so
//! it travels in the iced [`Theme`] palette; every other role is a constant.

use iced::border::{self, Border};
use iced::widget::{button, checkbox, container, overlay::menu, pick_list, radio, scrollable, slider, text_input};
use iced::{Background, Color, Font, Shadow, Theme, Vector, color};

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

pub const RADIUS_SM: f32 = 3.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 9.0;
pub const RADIUS_XL: f32 = 12.0;
pub const BORDER: f32 = 1.0;
pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const FONT_CAPTION: f32 = 13.0;
pub const FONT_BODY: f32 = 14.0;
pub const FONT_TITLE: f32 = 16.0;
pub const FONT_HEADER: f32 = 20.0;
pub const CONTROL_HEIGHT: f32 = 38.0;
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
            background: SURFACE,
            text: ON_SURFACE,
            primary,
            success: TERTIARY,
            warning: SECONDARY,
            danger: ERROR,
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

pub fn button_style(variant: ButtonVariant) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let primary = primary(theme);
        let (background, border_color, text_color) = match (variant, status) {
            (ButtonVariant::Selected, _) => (primary, primary, ON_PRIMARY),
            (_, button::Status::Hovered) => (HOVER, Color::TRANSPARENT, ON_HOVER),
            (_, button::Status::Pressed) => (primary, primary, ON_PRIMARY),
            (ButtonVariant::Default, button::Status::Disabled) => (
                alpha(SURFACE_VARIANT, DISABLED_ALPHA),
                alpha(OUTLINE, DISABLED_ALPHA),
                alpha(ON_SURFACE, DISABLED_ALPHA),
            ),
            (ButtonVariant::Default, _) => (SURFACE_VARIANT, OUTLINE, ON_SURFACE),
            (ButtonVariant::Primary, button::Status::Disabled) => {
                (alpha(primary, DISABLED_ALPHA), Color::TRANSPARENT, ON_PRIMARY)
            }
            (ButtonVariant::Primary, _) => (primary, Color::TRANSPARENT, ON_PRIMARY),
            (ButtonVariant::Tab, button::Status::Disabled) => {
                (Color::TRANSPARENT, Color::TRANSPARENT, alpha(ON_SURFACE, DISABLED_ALPHA))
            }
            (ButtonVariant::Tab, _) => (Color::TRANSPARENT, Color::TRANSPARENT, ON_SURFACE),
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
    button::Style { background: None, text_color: ON_SURFACE, ..button::Style::default() }
}

/// The rounded surface behind segmented controls and steppers.
pub fn track(_: &Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE_VARIANT.into()),
        border: border::rounded(RADIUS_MD),
        ..container::Style::default()
    }
}

pub fn rule(_: &Theme) -> container::Style {
    container::Style { background: Some(OUTLINE.into()), ..container::Style::default() }
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
        background: alpha(if checked { primary } else { SURFACE }, fade).into(),
        icon_color: ON_PRIMARY,
        border: Border {
            color: alpha(
                if hovered {
                    HOVER
                } else if checked {
                    primary
                } else {
                    OUTLINE
                },
                fade,
            ),
            width: BORDER,
            radius: RADIUS_SM.into(),
        },
        text_color: Some(ON_SURFACE),
    }
}

pub fn radio_style(theme: &Theme, status: radio::Status) -> radio::Style {
    let (selected, hovered) = match status {
        radio::Status::Active { is_selected } => (is_selected, false),
        radio::Status::Hovered { is_selected } => (is_selected, true),
    };
    let primary = primary(theme);
    radio::Style {
        background: if selected { primary } else { SURFACE }.into(),
        dot_color: ON_PRIMARY,
        border_width: BORDER,
        border_color: if selected {
            primary
        } else if hovered {
            HOVER
        } else {
            OUTLINE
        },
        text_color: Some(ON_SURFACE),
    }
}

pub fn slider_style(theme: &Theme, status: slider::Status) -> slider::Style {
    let hot = !matches!(status, slider::Status::Active);
    slider::Style {
        rail: slider::Rail {
            backgrounds: (primary(theme).into(), OUTLINE.into()),
            width: 8.0,
            border: border::rounded(4.0),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 9.0 },
            background: ON_PRIMARY.into(),
            border_width: BORDER,
            border_color: if hot { HOVER } else { OUTLINE },
        },
    }
}

pub fn pick_list_style(_: &Theme, status: pick_list::Status) -> pick_list::Style {
    let hot = !matches!(status, pick_list::Status::Active);
    pick_list::Style {
        text_color: ON_SURFACE,
        placeholder_color: alpha(ON_SURFACE_VARIANT, 0.7),
        handle_color: ON_SURFACE_VARIANT,
        background: SURFACE_VARIANT.into(),
        border: Border { color: if hot { HOVER } else { OUTLINE }, width: BORDER, radius: RADIUS_MD.into() },
    }
}

pub fn menu_style(_: &Theme) -> menu::Style {
    menu::Style {
        background: SURFACE_VARIANT.into(),
        border: Border { color: OUTLINE, width: BORDER, radius: RADIUS_MD.into() },
        text_color: ON_SURFACE,
        selected_text_color: ON_HOVER,
        selected_background: HOVER.into(),
        shadow: Shadow { color: alpha(Color::BLACK, 0.45), offset: Vector::new(0.0, 4.0), blur_radius: 16.0 },
    }
}

pub fn scrollable_style(_: &Theme, status: scrollable::Status) -> scrollable::Style {
    let hot = !matches!(status, scrollable::Status::Active { .. });
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(alpha(ON_SURFACE_VARIANT, if hot { 0.85 } else { 0.55 })),
            border: border::rounded(RADIUS_SM),
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: SURFACE_VARIANT.into(),
            border: border::rounded(RADIUS_MD).color(OUTLINE).width(BORDER),
            shadow: Shadow::default(),
            icon: ON_SURFACE,
        },
    }
}

pub fn text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: SURFACE_VARIANT.into(),
        border: Border { color: if focused { HOVER } else { OUTLINE }, width: BORDER, radius: RADIUS_MD.into() },
        icon: ON_SURFACE_VARIANT,
        placeholder: alpha(ON_SURFACE_VARIANT, 0.7),
        value: ON_SURFACE,
        selection: alpha(primary(theme), 0.35),
    }
}
