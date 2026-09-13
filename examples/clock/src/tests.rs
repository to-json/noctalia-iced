//! Headless UI tests (iced_test), following examples/clock/e2e/basic.nuis.

use crate::app::{Args, Clock, Message, format_offset, format_stopwatch};
use crate::face::Mode;
use noctalia_iced::theme;
use iced_test::Simulator;
use std::time::Duration;

/// Tall enough that every control is on screen without scrolling.
const SIZE: (f32, f32) = (900.0, 1600.0);

fn clock(extra: &[&str]) -> Clock {
    let args = ["--fixed-time", "2026-09-13T10:08:30Z", "--zone", "UTC"].iter().chain(extra).map(|s| s.to_string());
    Clock::new(&Args::parse(args).expect("args"))
}

fn ui(clock: &Clock) -> Simulator<'_, Message> {
    let settings = iced::Settings { fonts: vec![theme::ICON_FONT_BYTES.into()], ..iced::Settings::default() };
    Simulator::with_size(settings, SIZE, clock.view())
}

/// Clicks the widget showing `label` and feeds the resulting messages back through `update`.
fn click(clock: &mut Clock, label: &str) -> Vec<Message> {
    let messages: Vec<Message> = {
        let mut ui = ui(clock);
        ui.click(label).unwrap_or_else(|error| panic!("click {label:?}: {error:?}"));
        ui.into_messages().collect()
    };
    for message in &messages {
        let _ = clock.update(message.clone());
    }
    messages
}

fn shows(clock: &Clock, label: &str) -> bool {
    ui(clock).find(label).is_ok()
}

#[test]
fn basic_flow() {
    let mut clock = clock(&[]);
    assert!(shows(&clock, "Noctalia Clock"));
    assert!(shows(&clock, "UTC · UTC+00:00 · Analog"));

    assert!(matches!(click(&mut clock, "Digital").as_slice(), [Message::SetMode(Mode::Digital)]));
    assert!(shows(&clock, "Sun 13 Sep"));
    assert!(shows(&clock, "UTC · UTC+00:00 · Digital"));

    assert!(!shows(&clock, "Numerals"));
    assert!(matches!(click(&mut clock, "Advanced").as_slice(), [Message::Advanced(true)]));
    assert!(shows(&clock, "Numerals"));
    assert!(matches!(click(&mut clock, "Fire confetti").as_slice(), [Message::FireConfetti]));
}

#[test]
fn timer_and_stopwatch() {
    let mut clock = clock(&[]);
    assert!(shows(&clock, "300"));
    assert!(matches!(click(&mut clock, &theme::icon::PLUS.to_string()).as_slice(), [Message::TimerMinutes(6)]));
    assert!(shows(&clock, "6 min"));
    assert!(shows(&clock, "360"));

    // The timer's Start comes first; once it reads Pause, Start is the stopwatch's.
    assert!(matches!(click(&mut clock, "Start").as_slice(), [Message::TimerStartPause]));
    assert!(shows(&clock, "Pause"));
    assert!(matches!(click(&mut clock, "Start").as_slice(), [Message::StopwatchStartStop]));
    assert!(shows(&clock, "Stop"));
    std::thread::sleep(Duration::from_millis(20));
    assert!(matches!(click(&mut clock, "Lap").as_slice(), [Message::StopwatchLap]));
    assert!(matches!(click(&mut clock, "Stop").as_slice(), [Message::StopwatchStartStop]));
}

#[test]
fn snapshots() {
    for (name, extra) in [("analog", &[][..]), ("digital", &["--mode", "digital"][..]), ("both", &["--mode", "both", "--advanced"][..])] {
        let clock = clock(extra);
        let snapshot = ui(&clock).snapshot(&clock.theme()).expect("snapshot");
        assert!(snapshot.matches_image(format!("snapshots/{name}")).expect("compare"), "{name} snapshot changed");
    }
}

#[test]
fn formats() {
    assert_eq!(format_offset(-270), "-04:30");
    assert_eq!(format_offset(330), "+05:30");
    assert_eq!(format_stopwatch(Duration::from_millis(83_456)), "01:23.45");
}
