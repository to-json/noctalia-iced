//! Confetti: seeded particles under gravity, drawn as rotated rounded rectangles.

use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme, Vector};
use std::f64::consts::TAU;

pub const WIDTH: f32 = 400.0;
pub const HEIGHT: f32 = 110.0;
const LIFETIME_MS: f64 = 2500.0;
const PARTICLES: usize = 150;
const PALETTE: [u32; 6] = [0xff5a36, 0xffc233, 0x3ddc84, 0x33a1ff, 0xc86bff, 0xff6fb5];

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[derive(Debug, Clone)]
struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    rotation: f64,
    spin: f64,
    size: f64,
    color: Color,
}

#[derive(Debug, Clone)]
pub struct Confetti {
    particles: Vec<Particle>,
    age_ms: f64,
}

impl Confetti {
    pub fn new(seed: u64) -> Self {
        let mut rng = Lcg(seed ^ 0x9e37_79b9_7f4a_7c15);
        let (w, h) = (f64::from(WIDTH), f64::from(HEIGHT));
        let particles = (0..PARTICLES)
            .map(|i| {
                let rgb = PALETTE[i % PALETTE.len()];
                Particle {
                    x: rng.next() * w,
                    y: rng.next() * h * 0.7,
                    vx: (rng.next() - 0.5) * 120.0,
                    vy: -40.0 - rng.next() * 80.0,
                    rotation: rng.next() * TAU,
                    spin: (rng.next() - 0.5) * 12.0,
                    size: 5.0 + rng.next() * 6.0,
                    color: Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8),
                }
            })
            .collect();
        Confetti { particles, age_ms: 0.0 }
    }

    pub fn step(&mut self, dt_ms: f64) {
        let dt_ms = dt_ms.clamp(0.0, 50.0);
        let dt = dt_ms / 1000.0;
        for p in &mut self.particles {
            p.vy += 240.0 * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.rotation += p.spin * dt;
        }
        self.age_ms += dt_ms;
    }

    pub fn done(&self) -> bool {
        self.age_ms >= LIFETIME_MS
    }
}

/// The canvas program; `None` draws an empty (but laid out) area.
pub struct View<'a>(pub Option<&'a Confetti>);

impl<M> canvas::Program<M> for View<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if let Some(confetti) = self.0 {
            let opacity = ((LIFETIME_MS - confetti.age_ms) / 500.0).clamp(0.0, 1.0) as f32;
            let (w, h) = (f64::from(WIDTH), f64::from(HEIGHT));
            for p in confetti.particles.iter().filter(|p| p.y > -p.size && p.y < h && p.x > -p.size && p.x < w) {
                let (pw, ph) = (p.size as f32, (p.size * 0.6) as f32);
                frame.with_save(|f| {
                    // Rotate about the rectangle's centre, as the canvas op does.
                    f.translate(Vector::new(p.x as f32 + pw * 0.5, p.y as f32 + ph * 0.5));
                    f.rotate(p.rotation as f32);
                    let body = Path::rounded_rectangle(Point::new(-pw * 0.5, -ph * 0.5), Size::new(pw, ph), 1.5.into());
                    f.fill(&body, Color { a: p.color.a * opacity, ..p.color });
                });
            }
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_and_expires() {
        let mut confetti = Confetti::new(7);
        let before = confetti.particles[0].y;
        for _ in 0..60 {
            confetti.step(50.0);
        }
        assert!(confetti.particles[0].y != before);
        assert!(confetti.done());
    }
}
