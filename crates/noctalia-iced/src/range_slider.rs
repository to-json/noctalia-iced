//! A two-handle slider (range_slider.cpp); iced ships only single-value sliders.

use crate::theme;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::border::Border;
use iced::{Color, Element, Event, Length, Rectangle, Size, Theme, mouse, touch};
use std::ops::RangeInclusive;

const HEIGHT: f32 = 22.0;
const RAIL: f32 = 8.0;
const HANDLE: f32 = 18.0;
const PADDING: f32 = 2.0;

pub struct RangeSlider<'a, Message> {
    range: RangeInclusive<f32>,
    low: f32,
    high: f32,
    step: f32,
    width: Length,
    on_change: Box<dyn Fn(f32, f32) -> Message + 'a>,
}

pub fn range_slider<'a, Message>(
    range: RangeInclusive<f32>,
    (low, high): (f32, f32),
    on_change: impl Fn(f32, f32) -> Message + 'a,
) -> RangeSlider<'a, Message> {
    // Like range_slider.cpp, an inverted pair is shown (and reported back) sorted.
    let (low, high) = (low.min(high), low.max(high));
    RangeSlider { range, low, high, step: 0.0, width: Length::Fixed(180.0), on_change: Box::new(on_change) }
}

impl<Message> RangeSlider<'_, Message> {
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    fn track(bounds: Rectangle) -> (f32, f32) {
        (bounds.x + PADDING + HANDLE * 0.5, (bounds.width - PADDING * 2.0 - HANDLE).max(1.0))
    }

    fn value_at(&self, bounds: Rectangle, x: f32) -> f32 {
        let (start, width) = Self::track(bounds);
        let (min, max) = (*self.range.start(), *self.range.end());
        let raw = min + ((x - start) / width).clamp(0.0, 1.0) * (max - min);
        if self.step > 0.0 { ((raw - min) / self.step).round() * self.step + min } else { raw }
    }

    fn x_of(&self, bounds: Rectangle, value: f32) -> f32 {
        let (start, width) = Self::track(bounds);
        let (min, max) = (*self.range.start(), *self.range.end());
        start + ((value - min) / (max - min).max(f32::EPSILON)).clamp(0.0, 1.0) * width
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Handle {
    Low,
    High,
}

#[derive(Default)]
struct State {
    dragging: Option<Handle>,
    hovered: Option<Handle>,
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for RangeSlider<'_, Message>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(&mut self, _: &mut widget::Tree, _: &Renderer, limits: &layout::Limits) -> layout::Node {
        layout::atomic(limits, self.width, HEIGHT)
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _: &Renderer,
        _: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        let nearest = |x: f32| {
            let low = (x - self.x_of(bounds, self.low)).abs();
            let high = (x - self.x_of(bounds, self.high)).abs();
            // Coincident handles: move whichever way the pointer is going.
            if low < high || (low == high && x < self.x_of(bounds, self.low)) { Handle::Low } else { Handle::High }
        };
        let publish = |handle: Handle, x: f32, shell: &mut Shell<'_, Message>| {
            let value = self.value_at(bounds, x);
            let (low, high) = match handle {
                Handle::Low => (value.min(self.high), self.high),
                Handle::High => (self.low, value.max(self.low)),
            };
            if (low, high) != (self.low, self.high) {
                shell.publish((self.on_change)(low, high));
            }
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if let Some(position) = cursor.position_over(bounds) {
                    let handle = nearest(position.x);
                    state.dragging = Some(handle);
                    publish(handle, position.x, shell);
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) | Event::Touch(touch::Event::FingerMoved { .. }) => {
                if let (Some(handle), Some(position)) = (state.dragging, cursor.position()) {
                    publish(handle, position.x, shell);
                    shell.capture_event();
                } else {
                    let hovered = cursor.position_over(bounds).map(|p| nearest(p.x));
                    if hovered != state.hovered {
                        state.hovered = hovered;
                        shell.request_redraw();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. }) => {
                if state.dragging.take().is_some() {
                    shell.capture_event();
                }
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _: &renderer::Style,
        layout: Layout<'_>,
        _: mouse::Cursor,
        _: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let primary = theme::primary(theme);
        let rail_y = bounds.center_y() - RAIL * 0.5;
        let quad = |bounds: Rectangle, border: Border| renderer::Quad { bounds, border, ..renderer::Quad::default() };

        let rail = Rectangle::new(
            iced::Point::new(bounds.x + PADDING, rail_y),
            Size::new(bounds.width - PADDING * 2.0, RAIL),
        );
        renderer.fill_quad(quad(rail, iced::border::rounded(RAIL * 0.5)), theme::OUTLINE);

        let (low_x, high_x) = (self.x_of(bounds, self.low), self.x_of(bounds, self.high));
        let active = Rectangle::new(iced::Point::new(low_x, rail_y), Size::new(high_x - low_x, RAIL));
        renderer.fill_quad(quad(active, Border::default()), primary);

        for (handle, x) in [(Handle::Low, low_x), (Handle::High, high_x)] {
            let hot = state.dragging == Some(handle) || (state.dragging.is_none() && state.hovered == Some(handle));
            let knob = Rectangle::new(
                iced::Point::new(x - HANDLE * 0.5, bounds.center_y() - HANDLE * 0.5),
                Size::new(HANDLE, HANDLE),
            );
            let border = iced::border::rounded(HANDLE * 0.5)
                .color(if hot { theme::HOVER } else { theme::OUTLINE })
                .width(theme::BORDER);
            renderer.fill_quad(quad(knob, border), Color { a: 1.0, ..theme::ON_PRIMARY });
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _: &Rectangle,
        _: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if state.dragging.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, Message: 'a> From<RangeSlider<'a, Message>> for Element<'a, Message> {
    fn from(slider: RangeSlider<'a, Message>) -> Self {
        Element::new(slider)
    }
}
