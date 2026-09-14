//! Motion tokens: how long a Noctalia interface takes to change its mind, and the shape of the
//! change.
//!
//! The durations are noctalia-shell's own (`src/ui/style.h`: `animFast` 100 ms, `animNormal`
//! 200 ms, `animSlow` 400 ms). The curves are this library's: noctalia-shell animates through Qt,
//! whose easing curves have no iced equivalent to port, so [`SPRING`] and [`SETTLE`] are damped
//! springs written here and [`GLIDE`] is the one stock curve that never overshoots.
//!
//! Motion in this design language is *soft*: things arrive slightly past where they are going and
//! settle back, rather than snapping or bouncing. Two amplitudes, because overshoot that reads as
//! lively on a 3 px indicator reads as a wobble on a whole pane:
//!
//! | Curve | Overshoot | For |
//! |---|---|---|
//! | [`SPRING`] | ~7% | Short travel: indicators, badges, avatars, anything under ~60 px |
//! | [`SETTLE`] | ~2% | Whole surfaces: panes arriving, banners opening |
//! | [`GLIDE`] | none | Colour, opacity, and anything that must land exactly |
//!
//! ```
//! use noctalia_iced::motion;
//! use std::time::Instant;
//!
//! let now = Instant::now();
//!
//! // A value that animates from wherever it currently is when interrupted.
//! let mut selected = motion::spring_animation(0.0_f32);
//! selected.go_mut(3.0, now);
//! let travel = selected.interpolate_with(|row| row * 55.0, now);
//!
//! // A one-shot that replays from the start every time something changes.
//! let mut arrival = motion::Replay::settled(motion::SETTLE, motion::NORMAL);
//! arrival.restart(now);
//! assert_eq!(arrival.at(now), 0.0);
//! ```
//!
//! Every constructor here honours [`reduced`], so an application that builds its animations through
//! them gets a working reduced-motion mode for free.

use iced::animation::{Easing, Float};
use iced::{Animation, Color};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// noctalia-shell's `animFast`: hover and press feedback, anything the pointer is waiting on.
pub const FAST: Duration = Duration::from_millis(100);
/// noctalia-shell's `animNormal`: the default. Selection moving, a pane arriving.
pub const NORMAL: Duration = Duration::from_millis(200);
/// noctalia-shell's `animSlow`: staggered reveals and anything crossing the whole window.
pub const SLOW: Duration = Duration::from_millis(400);

/// Damping and frequency of [`SPRING`]. Overshoot is `exp(-d * PI / w)` — about 7%.
const SPRING_DAMPING: f32 = 7.5;
const SPRING_FREQUENCY: f32 = 9.0;
/// [`SETTLE`] is the same spring held tighter: about 2% overshoot.
const SETTLE_DAMPING: f32 = 9.0;
const SETTLE_FREQUENCY: f32 = 7.0;

/// A soft spring with about 7% overshoot, settled by the end of its duration. For short travel.
pub const SPRING: Easing = Easing::Custom(spring);
/// A tighter spring, about 2% overshoot. For whole surfaces, where [`SPRING`] reads as a wobble.
pub const SETTLE: Easing = Easing::Custom(settle);
/// No overshoot. For colour, and for anything whose end value must be exact.
pub const GLIDE: Easing = Easing::EaseOutCubic;

/// The position of a damped spring released at 0 and pulled to 1.
///
/// `1 - e^(-dx) (cos wx + d/w sin wx)`: zero and flat at `x = 0`, one overshoot, settled well
/// inside `x = 1` for both tunings here.
fn damped(damping: f32, frequency: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let decay = (-damping * x).exp();
    let phase = frequency * x;
    1.0 - decay * (phase.cos() + (damping / frequency) * phase.sin())
}

fn spring(x: f32) -> f32 {
    damped(SPRING_DAMPING, SPRING_FREQUENCY, x)
}

fn settle(x: f32) -> f32 {
    damped(SETTLE_DAMPING, SETTLE_FREQUENCY, x)
}

/// Whether the user asked for less motion.
///
/// True when `NOCTALIA_REDUCE_MOTION` or `NOCTMALIA_REDUCE_MOTION` is set to anything but `0`. Read
/// once: an application that changed it mid-run would have animations half in one mode and half in
/// the other, which is worse than either.
pub fn reduced() -> bool {
    static REDUCED: OnceLock<bool> = OnceLock::new();
    *REDUCED.get_or_init(|| {
        ["NOCTALIA_REDUCE_MOTION", "NOCTMALIA_REDUCE_MOTION"]
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .any(|value| value != "0")
    })
}

/// A multiplier on every duration, from `NOCTALIA_MOTION_SCALE`, clamped to 0.1–20.
///
/// Slow motion, for tuning a curve by eye — an overshoot that is too much at 200 ms is obvious at
/// 2 s and invisible in a screenshot at either. Read once, like [`reduced`].
pub fn scale() -> f32 {
    static SCALE: OnceLock<f32> = OnceLock::new();
    *SCALE.get_or_init(|| {
        std::env::var("NOCTALIA_MOTION_SCALE")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|scale| scale.is_finite())
            .map_or(1.0, |scale| scale.clamp(0.1, 20.0))
    })
}

/// The duration an animation actually runs for: `duration` at the current [`scale`], or as near to
/// nothing as makes no difference when [`reduced`].
pub fn duration(duration: Duration) -> Duration {
    if reduced() { Duration::from_millis(1) } else { duration.mul_f32(scale()) }
}

/// An [`Animation`] on [`SPRING`] at [`NORMAL`]. Short travel that should feel alive.
pub fn spring_animation<T: Clone + Copy + PartialEq + Float>(state: T) -> Animation<T> {
    Animation::new(state).easing(SPRING).duration(duration(NORMAL))
}

/// An [`Animation`] on [`SETTLE`] at [`NORMAL`]. Whole surfaces.
pub fn settle_animation<T: Clone + Copy + PartialEq + Float>(state: T) -> Animation<T> {
    Animation::new(state).easing(SETTLE).duration(duration(NORMAL))
}

/// An [`Animation`] on [`GLIDE`] at [`FAST`]. Colour and presence.
pub fn glide_animation<T: Clone + Copy + PartialEq + Float>(state: T) -> Animation<T> {
    Animation::new(state).easing(GLIDE).duration(duration(FAST))
}

/// A one-shot curve replayed from a moment in time.
///
/// [`Animation`] tracks a *value*, and transitions from wherever it currently is — right for a
/// selection that can be moved again mid-flight, wrong for "play this again from the beginning"
/// every time the content underneath changes. A [`Replay`] has no value: it is a clock and a curve.
#[derive(Debug, Clone, Copy)]
pub struct Replay {
    /// When the current play started, or `None` once it has nothing left to do. An `Instant` far
    /// enough in the past would say the same thing, but `Instant` has no portable way to go
    /// backwards from now without risking an underflow at process start.
    since: Option<Instant>,
    duration: Duration,
    easing: Easing,
}

impl Replay {
    /// A [`Replay`] that has already finished, so a first frame draws the settled state.
    pub fn settled(easing: Easing, span: Duration) -> Replay {
        Replay { since: None, duration: duration(span), easing }
    }

    /// A [`Replay`] starting now.
    pub fn starting(easing: Easing, span: Duration) -> Replay {
        Replay { since: Some(Instant::now()), duration: duration(span), easing }
    }

    /// Plays it again from the beginning.
    pub fn restart(&mut self, now: Instant) {
        self.since = Some(now);
    }

    /// Linear progress, 0 to 1. For staggering, where the curve is applied per item.
    pub fn linear(&self, now: Instant) -> f32 {
        let (Some(since), false) = (self.since, self.duration.is_zero()) else {
            return 1.0;
        };
        let elapsed = now.saturating_duration_since(since).as_secs_f32();
        (elapsed / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Eased progress, 0 to 1 — though a spring curve passes 1 on the way.
    pub fn at(&self, now: Instant) -> f32 {
        self.easing.value(self.linear(now))
    }

    pub fn is_animating(&self, now: Instant) -> bool {
        self.linear(now) < 1.0
    }
}

/// Progress of one item in a staggered reveal.
///
/// `linear` is the whole reveal's [`Replay::linear`]. Each item starts `step` later than the one
/// before it and takes `window` to arrive, both as fractions of the reveal. The returned progress
/// is already eased.
///
/// Leaving room for the tail matters: `index * step + window` should stay at or under 1.0 for the
/// last item that is actually worth animating, or it will still be moving when the reveal is over.
pub fn stagger(easing: Easing, linear: f32, index: usize, step: f32, window: f32) -> f32 {
    if window <= 0.0 {
        return 1.0;
    }
    let start = index as f32 * step;
    easing.value(((linear - start) / window).clamp(0.0, 1.0))
}

/// Blends two colours, including their alpha. `t` of 0 is `from`, 1 is `to`.
///
/// Overshooting curves hand back `t` slightly past 1, which would push a channel out of range, so
/// the result is clamped.
pub fn mix(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let channel = |a: f32, b: f32| a + (b - a) * t;
    Color { r: channel(from.r, to.r), g: channel(from.g, to.g), b: channel(from.b, to.b), a: channel(from.a, to.a) }
}

/// Interpolates a scalar the same way [`mix`] does colour, clamping an overshoot away.
///
/// For sizes and padding, where a value past the end is a layout glitch rather than a flourish.
pub fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The curves have to start at rest and end at rest, or an animation lands off its target.
    #[test]
    fn the_springs_start_at_zero_and_end_at_one() {
        for easing in [SPRING, SETTLE, GLIDE] {
            assert_eq!(easing.value(0.0), 0.0);
            assert_eq!(easing.value(1.0), 1.0);
        }
    }

    /// Soft, not bouncy: one overshoot each, within the amplitudes the module documents.
    #[test]
    fn the_springs_overshoot_once_and_by_the_documented_amount() {
        let peak = |easing: Easing| (0..=1000).map(|step| easing.value(step as f32 / 1000.0)).fold(0.0_f32, f32::max);
        let spring = peak(SPRING);
        let settle = peak(SETTLE);
        assert!((1.06..1.09).contains(&spring), "SPRING peaked at {spring}");
        assert!((1.01..1.03).contains(&settle), "SETTLE peaked at {settle}");
        assert!(settle < spring, "SETTLE is the tighter of the two");
        assert_eq!(peak(GLIDE), 1.0, "GLIDE never overshoots");
    }

    /// A spring that has not settled by the end of its duration keeps animating into the next one.
    #[test]
    fn the_springs_are_settled_well_before_their_duration_is_up() {
        for easing in [SPRING, SETTLE] {
            let residual = (easing.value(0.85) - 1.0).abs();
            assert!(residual < 0.005, "still {residual} from home at 85%");
        }
    }

    #[test]
    fn a_stagger_walks_its_items_in_and_leaves_them_there() {
        let step = 0.05;
        let window = 0.5;
        // Nothing has started at the very beginning...
        assert_eq!(stagger(GLIDE, 0.0, 0, step, window), 0.0);
        assert_eq!(stagger(GLIDE, 0.0, 4, step, window), 0.0);
        // ...the first item leads...
        let early = 0.2;
        assert!(stagger(GLIDE, early, 0, step, window) > stagger(GLIDE, early, 3, step, window));
        // ...and by the end everyone has arrived and stays.
        for index in 0..10 {
            assert_eq!(stagger(GLIDE, 1.0, index, step, window), 1.0);
        }
    }

    /// Nothing here reads the environment during the test run, so this pins the arithmetic and the
    /// clamp rather than the parsing: an unset scale has to leave the tokens exactly as written.
    #[test]
    fn an_unscaled_duration_is_the_token_itself() {
        assert_eq!(scale(), 1.0, "no NOCTALIA_MOTION_SCALE is set in the test environment");
        assert_eq!(duration(NORMAL), NORMAL);
        assert_eq!(duration(SLOW), SLOW);
    }

    #[test]
    fn a_settled_replay_reads_as_finished_on_its_first_frame() {
        let replay = Replay::settled(SETTLE, NORMAL);
        let now = Instant::now();
        assert_eq!(replay.at(now), 1.0);
        assert!(!replay.is_animating(now));
    }

    #[test]
    fn restarting_a_replay_puts_it_back_at_the_beginning() {
        let now = Instant::now();
        let mut replay = Replay::settled(SETTLE, NORMAL);
        replay.restart(now);
        assert_eq!(replay.linear(now), 0.0);
        assert!(replay.is_animating(now));
        assert_eq!(replay.linear(now + NORMAL), 1.0);
        assert!(!replay.is_animating(now + NORMAL));
    }

    /// An overshooting curve hands back more than 1; neither blend may leave its range.
    #[test]
    fn blending_clamps_what_a_spring_overshoots() {
        let black = Color::BLACK;
        let white = Color::WHITE;
        assert_eq!(mix(black, white, 1.07), white);
        assert_eq!(mix(black, white, -0.2), black);
        assert_eq!(lerp(0.0, 10.0, 1.07), 10.0);
    }
}
