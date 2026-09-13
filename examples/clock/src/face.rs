//! The clock face (src/toolkit/clock_face.cpp): an analog dial on a canvas and a digital readout
//! built from text widgets.

use noctalia_iced::theme;
use chrono::DateTime;
use iced::alignment::{self, Vertical};
use iced::font::{Font, Weight};
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, path::Arc};
use iced::widget::{column, container, row, space, text};
use iced::{Alignment, Color, Element, Length, Padding, Point, Radians, Rectangle, Renderer, Size, Theme, Vector};
use std::f32::consts::{FRAC_PI_2, TAU};
use std::fmt;

const DAY_MS: f64 = 86_400_000.0;
const SEMIBOLD: Font = Font { weight: Weight::Semibold, ..Font::DEFAULT };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Analog,
    Digital,
    Both,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Analog, Mode::Digital, Mode::Both];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Analog => "Analog",
            Mode::Digital => "Digital",
            Mode::Both => "Both",
        }
    }

    fn analog(self) -> bool {
        self != Mode::Digital
    }

    fn digital(self) -> bool {
        self != Mode::Analog
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandStyle {
    Classic,
    Slim,
    Rounded,
}

impl HandStyle {
    pub const ALL: [HandStyle; 3] = [HandStyle::Classic, HandStyle::Slim, HandStyle::Rounded];
}

impl fmt::Display for HandStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            HandStyle::Classic => "Classic",
            HandStyle::Slim => "Slim",
            HandStyle::Rounded => "Rounded",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Face {
    pub mode: Mode,
    pub use24h: bool,
    pub show_seconds: bool,
    pub smooth: bool,
    pub numerals: bool,
    pub show_date: bool,
    pub utc_ms: f64,
    pub offset_minutes: i32,
    pub size: f32,
    pub hand: HandStyle,
    /// Timer progress drawn as an arc inside the rim; `0` draws nothing.
    pub arc: f32,
    /// 0..=1; the face fades to `1 - dim * 0.75`.
    pub dim: f32,
    pub accent: Color,
}

impl Face {
    fn local_ms(&self) -> f64 {
        self.utc_ms + f64::from(self.offset_minutes) * 60_000.0
    }

    fn day_seconds(&self) -> f64 {
        let seconds = self.local_ms().rem_euclid(DAY_MS) / 1000.0;
        if self.smooth { seconds } else { seconds.floor() }
    }

    fn brightness(&self) -> f32 {
        1.0 - self.dim * 0.75
    }

    fn digital_font(&self) -> f32 {
        self.size * if self.mode == Mode::Digital { 0.2 } else { 0.11 }
    }

    pub fn digital_text(&self) -> (String, Option<&'static str>) {
        let whole = self.local_ms().rem_euclid(DAY_MS) as i64 / 1000;
        let (hours, minutes, seconds) = (whole / 3600, whole / 60 % 60, whole % 60);
        let (mut line, meridiem) = if self.use24h {
            (format!("{hours:02}:{minutes:02}"), None)
        } else {
            let h12 = if hours % 12 == 0 { 12 } else { hours % 12 };
            (format!("{h12}:{minutes:02}"), Some(if hours < 12 { "AM" } else { "PM" }))
        };
        if self.show_seconds {
            line += &format!(":{seconds:02}");
        }
        (line, meridiem)
    }

    pub fn date_text(&self) -> String {
        DateTime::from_timestamp_millis(self.local_ms() as i64)
            .map(|t| t.format("%a %-d %b").to_string())
            .unwrap_or_default()
    }

    pub fn view<'a, M: 'a>(self) -> Element<'a, M> {
        let mut face = column![].width(self.size).align_x(Alignment::Center);
        if self.mode.analog() {
            face = face.push(canvas::Canvas::new(self).width(self.size).height(self.size));
        }
        if self.mode.digital() {
            if self.mode.analog() {
                face = face.push(space().height(self.size * 0.04));
            }
            face = face.push(self.digital());
        }
        face.into()
    }

    /// Fixed-width cells so the readout doesn't shift as digits change.
    fn digital<'a, M: 'a>(&self) -> Element<'a, M> {
        let font = self.digital_font();
        let fade = |c: Color| theme::alpha(c, self.brightness());
        let (line, meridiem) = self.digital_text();

        let cells = line.chars().map(|c| {
            let width = if c == ':' { font * 0.34 } else { font * 0.64 };
            container(text(c.to_string()).size(font).font(SEMIBOLD).color(fade(theme::ON_SURFACE)))
                .width(width)
                .align_x(alignment::Horizontal::Center)
                .into()
        });
        let mut readout = row(cells).align_y(Vertical::Top);
        if let Some(meridiem) = meridiem {
            readout = readout.push(
                container(text(meridiem).size(font * 0.4).font(SEMIBOLD).color(fade(self.accent)))
                    .padding(Padding { left: font * 0.15, top: font * 0.15, ..Padding::ZERO }),
            );
        }

        let mut block = column![container(readout).center_x(Length::Fill).height(font * 1.3)].width(self.size);
        if self.show_date {
            block = block.push(
                container(text(self.date_text()).size(font * 0.4).font(SEMIBOLD).color(fade(theme::ON_SURFACE)))
                    .center_x(Length::Fill)
                    .height(font * 0.55),
            );
        }
        block.into()
    }
}

fn hand(frame: &mut Frame, center: Point, angle: f32, width: f32, length: f32, tail: f32, radius: f32, color: Color) {
    frame.with_save(|f| {
        f.translate(Vector::new(center.x, center.y));
        f.rotate(angle);
        let body = Path::rounded_rectangle(Point::new(-width * 0.5, -length), Size::new(width, length + tail), radius.into());
        f.fill(&body, color);
    });
}

impl<M> canvas::Program<M> for Face {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let size = self.size;
        let r = size * 0.5;
        let c = Point::new(bounds.width * 0.5, r);
        let fade = |color: Color| theme::alpha(color, self.brightness());

        // Face and rim; the rim is the tick colour at 35%.
        let rim = (size * 0.006).max(1.0);
        frame.fill(&Path::circle(c, r), fade(theme::SURFACE_VARIANT));
        frame.stroke(
            &Path::circle(c, r - rim * 0.5),
            Stroke::default().with_color(fade(theme::alpha(theme::ON_SURFACE_VARIANT, 0.35))).with_width(rim),
        );

        if self.arc > 0.0 {
            let inset = size * 0.035;
            let thickness = (size * 0.02).max(2.0);
            let arc = Path::new(|b| {
                b.arc(Arc {
                    center: c,
                    radius: r - inset - thickness * 0.5,
                    start_angle: Radians(-FRAC_PI_2),
                    end_angle: Radians(-FRAC_PI_2 + TAU * self.arc.min(1.0)),
                });
            });
            frame.stroke(&arc, Stroke::default().with_color(fade(self.accent)).with_width(thickness));
        }

        let tick_inset = r * 0.07;
        for i in 0..60 {
            let major = i % 5 == 0;
            let w = if major { (r * 0.03).max(2.0) } else { (r * 0.012).max(1.0) };
            let len = if major { r * 0.1 } else { r * 0.05 };
            let color = fade(theme::alpha(theme::ON_SURFACE_VARIANT, if major { 1.0 } else { 0.55 }));
            frame.with_save(|f| {
                f.translate(Vector::new(c.x, c.y));
                f.rotate(i as f32 * TAU / 60.0);
                f.fill(&Path::rounded_rectangle(Point::new(-w * 0.5, -r + tick_inset), Size::new(w, len), (w * 0.5).into()), color);
            });
        }

        if self.numerals {
            let radius = r * 0.7;
            for i in 0..12 {
                let angle = i as f32 * TAU / 12.0;
                frame.fill_text(canvas::Text {
                    content: if i == 0 { "12".into() } else { i.to_string() },
                    position: Point::new(c.x + angle.sin() * radius, c.y - angle.cos() * radius),
                    color: fade(theme::ON_SURFACE),
                    size: (size * 0.075).into(),
                    font: SEMIBOLD,
                    align_x: alignment::Horizontal::Center.into(),
                    align_y: Vertical::Center,
                    ..canvas::Text::default()
                });
            }
        }

        let seconds = self.day_seconds();
        let second_angle = (seconds % 60.0 / 60.0) as f32 * TAU;
        let minute_angle = (seconds / 60.0 % 60.0 / 60.0) as f32 * TAU;
        let hour_angle = (seconds / 3600.0 % 12.0 / 12.0) as f32 * TAU;

        let (width_scale, round) = match self.hand {
            HandStyle::Classic => (1.0, false),
            HandStyle::Slim => (0.55, false),
            HandStyle::Rounded => (1.0, true),
        };
        let radius = |w: f32| if round { w * 0.5 } else { (w * 0.25).min(2.0) };
        let hour_w = r * 0.07 * width_scale;
        let minute_w = r * 0.045 * width_scale;
        let second_w = (r * 0.018 * width_scale).max(1.0);
        let hand_color = fade(theme::ON_SURFACE);
        hand(&mut frame, c, hour_angle, hour_w, r * 0.5, r * 0.1, radius(hour_w), hand_color);
        hand(&mut frame, c, minute_angle, minute_w, r * 0.76, r * 0.1, radius(minute_w), hand_color);
        if self.show_seconds {
            hand(&mut frame, c, second_angle, second_w, r * 0.86, r * 0.2, radius(second_w), fade(self.accent));
        }
        let cap = r * if round { 0.14 } else { 0.1 };
        frame.fill(&Path::circle(c, cap * 0.5), fade(self.accent));

        vec![frame.into_geometry()]
    }
}
