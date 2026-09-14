//! Noctalia controls that iced has no direct equivalent for, composed from iced widgets.

use crate::theme::{self, ButtonVariant, icon};
use iced::alignment::Vertical;
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, path::Arc};
use iced::widget::{button, center, column, container, row, space, stack, text, text_input};
use iced::{Alignment, Color, Element, Length, Padding, Point, Radians, Rectangle, Renderer, Size, Theme, Vector};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// Room around the saturation/value square so its marker isn't clipped at the edges.
const MARKER: f32 = 10.0;
const HUE_THUMB: f32 = 10.0;
/// Setting rows keep the C++ demo's 70px rhythm (60px row + 10px gap) around 38px controls.
pub const ROW_HEIGHT: f32 = 60.0;

pub fn icon<'a>(code_point: char, size: f32) -> text::Text<'a> {
    text(code_point.to_string()).font(theme::ICON_FONT).size(size).line_height(1.0)
}

/// A labelled row: the fixed-width title the clock uses for every setting.
pub fn setting<'a, M: 'a>(title: impl text::IntoFragment<'a>, control: impl Into<Element<'a, M>>) -> Element<'a, M> {
    // The label cell sets the minimum row height; taller controls (the colour picker) grow the row.
    let label = container(text(title).size(theme::FONT_BODY)).width(120).height(ROW_HEIGHT).center_y(ROW_HEIGHT);
    row![label, control.into()].spacing(theme::SPACE_MD).align_y(Alignment::Center).into()
}

/// A standard 38px button; `None` disables it.
pub fn action<'a, M: Clone + 'a>(
    label: impl text::IntoFragment<'a>,
    variant: ButtonVariant,
    on_press: Option<M>,
) -> button::Button<'a, M> {
    button(text(label).size(theme::FONT_BODY).height(Length::Fill).align_y(Vertical::Center))
        .height(theme::CONTROL_HEIGHT)
        .padding([0.0, theme::SPACE_MD])
        .style(theme::button_style(variant))
        .on_press_maybe(on_press)
}

fn vertical_rule<'a, M: 'a>(inset: f32) -> Element<'a, M> {
    container(container(space()).width(1).height(Length::Fill).style(theme::rule))
        .padding([inset, 0.0])
        .height(Length::Fill)
        .into()
}

/// Options on one track; the selected one is filled with primary, and a rule separates two
/// neighbouring unselected options.
pub fn segmented<'a, M: Clone + 'a>(options: &[&'a str], selected: usize, on_select: impl Fn(usize) -> M) -> Element<'a, M> {
    let mut items: Vec<Element<'a, M>> = Vec::with_capacity(options.len() * 2);
    for (index, label) in options.iter().enumerate() {
        if index > 0 && index != selected && index - 1 != selected {
            items.push(vertical_rule(theme::SPACE_SM));
        }
        let variant = if index == selected { ButtonVariant::Selected } else { ButtonVariant::Tab };
        items.push(
            button(text(*label).size(theme::FONT_BODY).height(Length::Fill).align_y(Vertical::Center))
                .height(Length::Fill)
                .padding([0.0, 20.0])
                .style(theme::button_style(variant))
                .on_press(on_select(index))
                .into(),
        );
    }
    container(row(items)).height(theme::CONTROL_HEIGHT).style(theme::track).into()
}

/// Medium toggle (style.h): 18px thumb, 3px inset, 16px travel.
pub fn toggle<'a, M: Clone + 'a>(on: bool, on_toggle: impl Fn(bool) -> M) -> Element<'a, M> {
    const THUMB: f32 = 18.0;
    const INSET: f32 = 3.0;
    const TRAVEL: f32 = 16.0;
    let thumb = container(space()).width(THUMB).height(THUMB).style(move |theme: &Theme| container::Style {
        background: Some(if on { theme::palette().on_primary } else { theme::palette().surface_variant }.into()),
        border: iced::border::rounded(THUMB / 2.0).color(if on { theme::primary(theme) } else { theme::palette().outline }).width(1),
        ..container::Style::default()
    });
    let track = container(thumb)
        .width(THUMB + INSET * 2.0 + TRAVEL)
        .height(THUMB + INSET * 2.0)
        .padding(Padding { left: if on { INSET + TRAVEL } else { INSET }, top: INSET, ..Padding::ZERO });
    button(track)
        .padding(0)
        .style(move |theme, status| {
            let primary = theme::primary(theme);
            let hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(if on { primary } else { theme::palette().outline }.into()),
                border: iced::border::rounded((THUMB + INSET * 2.0) / 2.0)
                    .color(if hovered { theme::palette().hover } else if on { primary } else { theme::palette().outline })
                    .width(theme::BORDER),
                ..button::Style::default()
            }
        })
        .on_press(on_toggle(!on))
        .into()
}

/// `[-] value [+]` on a track (stepper.cpp).
pub fn stepper<'a, M: Clone + 'a>(
    value: i32,
    min: i32,
    max: i32,
    suffix: &str,
    on_value: impl Fn(i32) -> M,
) -> Element<'a, M> {
    let step = |glyph: char, next: i32, enabled: bool| {
        button(center(icon(glyph, theme::FONT_BODY)))
            .width(theme::CONTROL_HEIGHT)
            .height(Length::Fill)
            .padding(0)
            .style(theme::button_style(ButtonVariant::Tab))
            .on_press_maybe(enabled.then(|| on_value(next)))
    };
    container(row![
        step(icon::MINUS, value - 1, value > min),
        vertical_rule(0.0),
        center(text(format!("{value}{suffix}")).size(theme::FONT_BODY)),
        vertical_rule(0.0),
        step(icon::PLUS, value + 1, value < max),
    ])
    .width(150)
    .height(theme::CONTROL_HEIGHT)
    .style(theme::track)
    .into()
}

/// A header that expands a body below it (collapsible.cpp).
pub fn collapsible<'a, M: Clone + 'a>(
    title: &'a str,
    expanded: bool,
    on_toggle: impl Fn(bool) -> M,
    body: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    let chevron = icon(if expanded { icon::CHEVRON_UP } else { icon::CHEVRON_DOWN }, theme::FONT_BODY)
        .color(theme::palette().on_surface_variant);
    let header = button(
        row![text(title).size(theme::FONT_BODY), space().width(Length::Fill), chevron]
            .spacing(theme::SPACE_XS)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(ROW_HEIGHT)
    .padding([0.0, theme::SPACE_MD])
    .style(theme::bare_button)
    .on_press(on_toggle(!expanded));

    let mut content = column![header];
    if expanded {
        content = content.push(body);
    }
    content.into()
}

// ── Canvas-drawn controls ───────────────────────────────────────────────────

struct Ring {
    fraction: f32,
    thickness: f32,
    color: Color,
}

impl<M> canvas::Program<M> for Ring {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let fraction = self.fraction.clamp(0.0, 1.0);
        if fraction > 0.0 {
            let center = frame.center();
            let radius = bounds.width.min(bounds.height) * 0.5 - self.thickness * 0.5;
            let arc = Path::new(|b| {
                b.arc(Arc {
                    center,
                    radius,
                    start_angle: Radians(-FRAC_PI_2),
                    end_angle: Radians(-FRAC_PI_2 + TAU * fraction),
                });
            });
            frame.stroke(&arc, Stroke::default().with_color(self.color).with_width(self.thickness));
        }
        vec![frame.into_geometry()]
    }
}

/// The timer's progress arc with the remaining seconds inside (countdown_ring.cpp).
pub fn countdown_ring<'a, M: 'a>(fraction: f32, seconds: u64, size: f32, thickness: f32, color: Color) -> Element<'a, M> {
    stack![
        canvas::Canvas::new(Ring { fraction, thickness, color }).width(size).height(size),
        center(text(seconds.to_string()).size(size * 0.36).font(theme::semibold()).color(color)),
    ]
    .width(size)
    .height(size)
    .into()
}

struct Spin {
    phase: f32,
    thickness: f32,
    color: Color,
}

impl<M> canvas::Program<M> for Spin {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        const SEGMENTS: usize = 40;
        let mut frame = Frame::new(renderer, bounds.size());
        let center = frame.center();
        let radius = bounds.width.min(bounds.height) * 0.5 - self.thickness * 0.5;
        let head = self.phase * TAU;
        let stroke = |alpha: f32| Stroke::default().with_color(theme::alpha(self.color, alpha)).with_width(self.thickness);

        // A faint track, then a comet trail that brightens toward its head (spinner_program.cpp).
        frame.stroke(&Path::circle(center, radius), stroke(0.12));
        let trail = PI * 1.48;
        for i in 0..SEGMENTS {
            let t0 = i as f32 / SEGMENTS as f32;
            let t1 = (i + 1) as f32 / SEGMENTS as f32;
            let progress = 1.0 - t0;
            let opacity = 0.08 + (0.88 - 0.08) * progress.powf(0.72);
            let arc = Path::new(|b| {
                b.arc(Arc {
                    center,
                    radius,
                    start_angle: Radians(head - trail * t1),
                    end_angle: Radians(head - trail * t0),
                });
            });
            frame.stroke(&arc, stroke(0.88 * opacity));
        }
        let tip = Point::new(center.x + radius * head.cos(), center.y + radius * head.sin());
        frame.fill(&Path::circle(tip, self.thickness * 0.5), self.color);
        vec![frame.into_geometry()]
    }
}

/// `phase` is the fraction of a 1.2s revolution.
pub fn spinner<'a, M: 'a>(phase: f32, size: f32, color: Color) -> Element<'a, M> {
    canvas::Canvas::new(Spin { phase, thickness: 2.0, color }).width(size).height(size).into()
}

struct Series {
    values: Vec<f32>,
    color: Color,
}

impl<M> canvas::Program<M> for Series {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        const LINE: f32 = 1.5;
        let mut frame = Frame::new(renderer, bounds.size());
        let n = self.values.len();
        if n >= 2 {
            let h = bounds.height - LINE;
            let point = |i: usize, v: f32| {
                Point::new(i as f32 / (n - 1) as f32 * bounds.width, LINE * 0.5 + (1.0 - v.clamp(0.0, 1.0)) * h)
            };
            let line = Path::new(|b| {
                for (i, v) in self.values.iter().enumerate() {
                    if i == 0 { b.move_to(point(i, *v)) } else { b.line_to(point(i, *v)) }
                }
            });
            let area = Path::new(|b| {
                b.move_to(Point::new(0.0, bounds.height));
                for (i, v) in self.values.iter().enumerate() {
                    b.line_to(point(i, *v));
                }
                b.line_to(Point::new(bounds.width, bounds.height));
                b.close();
            });
            frame.fill(&area, theme::alpha(self.color, 0.15));
            frame.stroke(&line, Stroke::default().with_color(self.color).with_width(LINE));
        }
        vec![frame.into_geometry()]
    }
}

/// A line graph of values in 0..=1 with a translucent fill beneath (graph.cpp).
pub fn graph<'a, M: 'a>(values: Vec<f32>, width: f32, height: f32, color: Color) -> Element<'a, M> {
    canvas::Canvas::new(Series { values, color }).width(width).height(height).into()
}

// ── Color picker ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum PickerEvent {
    Hsv(f32, f32, f32),
    Hex(String),
    HexSubmit,
    Channel(usize, String),
}

#[derive(Clone, Copy)]
enum Area {
    SatVal,
    Hue,
}

struct PickerArea {
    area: Area,
    hsv: [f32; 3],
}

impl PickerArea {
    fn pick(&self, bounds: Rectangle, position: Point) -> PickerEvent {
        let area = bounds.shrink(MARKER);
        let x = ((position.x - area.x) / area.width).clamp(0.0, 1.0);
        let y = ((position.y - area.y) / area.height).clamp(0.0, 1.0);
        let [h, s, v] = self.hsv;
        match self.area {
            Area::SatVal => PickerEvent::Hsv(h, x, 1.0 - y),
            Area::Hue => PickerEvent::Hsv(x, s, v),
        }
    }
}

impl canvas::Program<PickerEvent> for PickerArea {
    type State = bool;

    fn update(
        &self,
        dragging: &mut bool,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<PickerEvent>> {
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_over(bounds)?;
                *dragging = true;
                Some(canvas::Action::publish(self.pick(bounds, position)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) if *dragging => {
                let position = cursor.position()?;
                Some(canvas::Action::publish(self.pick(bounds, position)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if *dragging => {
                *dragging = false;
                Some(canvas::Action::capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _: &bool, renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.translate(Vector::new(MARKER, MARKER));
        let size = bounds.shrink(MARKER).size();
        let [h, s, v] = self.hsv;
        match self.area {
            Area::SatVal => {
                // Rows of a horizontal white→hue gradient, darkened toward black at the bottom.
                const ROWS: usize = 48;
                let row_h = size.height / ROWS as f32;
                for r in 0..ROWS {
                    let value = 1.0 - (r as f32 + 0.5) / ROWS as f32;
                    let gradient = canvas::gradient::Linear::new(Point::ORIGIN, Point::new(size.width, 0.0))
                        .add_stop(0.0, hsv_to_color(h, 0.0, value))
                        .add_stop(1.0, hsv_to_color(h, 1.0, value));
                    frame.fill_rectangle(Point::new(0.0, r as f32 * row_h), Size::new(size.width, row_h + 0.5), gradient);
                }
                let marker = Point::new(s * size.width, (1.0 - v) * size.height);
                frame.stroke(&Path::circle(marker, 10.0), Stroke::default().with_color(Color::WHITE).with_width(2.0));
                frame.stroke(&Path::rectangle(Point::ORIGIN, size), Stroke::default().with_color(theme::palette().outline).with_width(theme::BORDER * 2.0));
            }
            Area::Hue => {
                // A 12px strip of 36 swatches, centred in a canvas tall enough for the round thumb.
                const SEGMENTS: usize = 36;
                const STRIP: f32 = 12.0;
                let top = (size.height - STRIP) * 0.5;
                let w = size.width / SEGMENTS as f32;
                for i in 0..SEGMENTS {
                    let t = i as f32 / (SEGMENTS - 1) as f32;
                    frame.fill_rectangle(Point::new(i as f32 * w, top), Size::new(w + 0.5, STRIP), hsv_to_color(t, 1.0, 1.0));
                }
                let thumb = Point::new((h * size.width).clamp(HUE_THUMB, size.width - HUE_THUMB), size.height * 0.5);
                frame.fill(&Path::circle(thumb, HUE_THUMB - 1.0), hsv_to_color(h, 1.0, 1.0));
                frame.stroke(&Path::circle(thumb, HUE_THUMB - 1.0), Stroke::default().with_color(Color::WHITE).with_width(2.0));
            }
        }
        vec![frame.into_geometry()]
    }
}

/// Saturation/value square, a 36-segment hue strip, and Hex/R/G/B fields (color_picker.cpp).
pub fn color_picker<'a, M: Clone + 'a>(
    hsv: [f32; 3],
    hex: &'a str,
    channels: &'a [String; 3],
    width: f32,
    on_event: fn(PickerEvent) -> M,
) -> Element<'a, M> {
    let field = |title: &'a str, input: text_input::TextInput<'a, PickerEvent>, w: f32| -> Element<'a, PickerEvent> {
        column![
            text(title).size(theme::FONT_CAPTION).color(theme::palette().on_surface_variant),
            input
                .size(theme::FONT_CAPTION)
                .padding([7.0, theme::SPACE_SM])
                .width(w)
                .style(theme::text_input_style),
        ]
        .spacing(theme::SPACE_XS * 0.5)
        .into()
    };
    let channel = |index: usize, title: &'a str| {
        field(title, text_input("", &channels[index]).on_input(move |s| PickerEvent::Channel(index, s)), 56.0)
    };
    let picker: Element<'a, PickerEvent> = column![
        canvas::Canvas::new(PickerArea { area: Area::SatVal, hsv })
            .width(width + MARKER * 2.0)
            .height(width * 0.8 + MARKER * 2.0),
        canvas::Canvas::new(PickerArea { area: Area::Hue, hsv })
            .width(width + MARKER * 2.0)
            .height(HUE_THUMB * 2.0 + MARKER * 2.0),
        row![
            field(
                "Hex",
                text_input("#rrggbb", hex).on_input(PickerEvent::Hex).on_submit(PickerEvent::HexSubmit),
                108.0
            ),
            channel(0, "R"),
            channel(1, "G"),
            channel(2, "B"),
        ]
        .spacing(theme::SPACE_SM)
        .padding(Padding { left: MARKER, ..Padding::ZERO }),
    ]
    .spacing(theme::SPACE_SM - MARKER)
    .into();
    picker.map(on_event)
}

pub fn hsv_to_color(h: f32, s: f32, v: f32) -> Color {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    Color::from_rgb(r + m, g + m, b + m)
}

pub fn color_to_hsv(color: Color) -> [f32; 3] {
    let (r, g, b) = (color.r, color.g, color.b);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    [h, if max == 0.0 { 0.0 } else { d / max }, max]
}

pub fn to_rgb8(color: Color) -> [u8; 3] {
    let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [c(color.r), c(color.g), c(color.b)]
}

pub fn to_hex(color: Color) -> String {
    let [r, g, b] = to_rgb8(color);
    format!("#{r:02X}{g:02X}{b:02X}")
}

pub fn parse_hex(text: &str) -> Option<Color> {
    let digits = text.trim().strip_prefix('#').unwrap_or(text.trim());
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some(Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_round_trip() {
        for hex in ["#FF5A36", "#FFF59B", "#33A1FF", "#000000", "#FFFFFF"] {
            let color = parse_hex(hex).unwrap();
            let [h, s, v] = color_to_hsv(color);
            assert_eq!(to_hex(hsv_to_color(h, s, v)), hex);
        }
        assert_eq!(parse_hex("12345"), None);
    }
}
