//! The clock application: state, messages, update, view and subscriptions.

use noctalia_iced::chrome;
use crate::confetti::{self, Confetti};
use crate::face::{Face, HandStyle, Mode};
use noctalia_iced::range_slider::range_slider;
use noctalia_iced::theme::{self, ButtonVariant};
use noctalia_iced::widgets::{self, PickerEvent, action, setting};
use chrono::{DateTime, Offset, TimeDelta, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{canvas, checkbox, column, pick_list, radio, row, scrollable, slider, text};
use iced::{Alignment, Color, Element, Length, Subscription, Task, Theme, color, window};
use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const DEFAULT_ACCENT: Color = color!(0xff5a36);
const BASE_FACE_SIZE: f32 = 220.0;
const JITTER_SAMPLES: usize = 120;
const SPINNER_REVOLUTION: Duration = Duration::from_millis(1200);

/// Display name and zone; `None` is the system's local zone.
pub const ZONES: [(&str, Option<Tz>); 7] = [
    ("Local", None),
    ("UTC", Some(Tz::UTC)),
    ("Tokyo", Some(Tz::Asia__Tokyo)),
    ("New York", Some(Tz::America__New_York)),
    ("London", Some(Tz::Europe__London)),
    ("Sydney", Some(Tz::Australia__Sydney)),
    ("Kolkata", Some(Tz::Asia__Kolkata)),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zone(pub usize);

impl Zone {
    pub const ALL: [Zone; ZONES.len()] = [Zone(0), Zone(1), Zone(2), Zone(3), Zone(4), Zone(5), Zone(6)];
}

impl fmt::Display for Zone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(ZONES[self.0].0)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SetMode(Mode),
    SetZone(Zone),
    Use24h(bool),
    ShowSeconds(bool),
    SmoothSweep(bool),
    Hand(HandStyle),
    FaceScale(f32),
    DimRange(f32, f32),
    Advanced(bool),
    ShowNumerals(bool),
    FireConfetti,
    TimerMinutes(i32),
    TimerStartPause,
    TimerReset,
    StopwatchStartStop,
    StopwatchLap,
    StopwatchReset,
    Picker(PickerEvent),
    Tick,
    StopwatchTick,
    Frame(Instant),
    Screenshot(window::Screenshot),
    Chrome(chrome::Action),
}

// ── Timer and stopwatch ─────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct Countdown {
    left: Duration,
    since: Option<Instant>,
}

impl Countdown {
    fn remaining(&self, at: Instant) -> Duration {
        match self.since {
            Some(since) => self.left.saturating_sub(at.saturating_duration_since(since)),
            None => self.left,
        }
    }

    fn running(&self) -> bool {
        self.since.is_some()
    }

    fn start(&mut self, at: Instant) {
        if self.since.is_none() && !self.left.is_zero() {
            self.since = Some(at);
        }
    }

    fn pause(&mut self, at: Instant) {
        self.left = self.remaining(at);
        self.since = None;
    }

    fn reset(&mut self, total: Duration) {
        self.left = total;
        self.since = None;
    }
}

#[derive(Debug, Default)]
struct Stopwatch {
    accumulated: Duration,
    since: Option<Instant>,
    laps: Vec<Duration>,
}

impl Stopwatch {
    fn elapsed(&self, at: Instant) -> Duration {
        self.accumulated + self.since.map_or(Duration::ZERO, |since| at.saturating_duration_since(since))
    }

    fn running(&self) -> bool {
        self.since.is_some()
    }
}

// ── Command line ────────────────────────────────────────────────────────────

pub const USAGE: &str = "usage: noctalia-clock-iced [--fixed-time RFC3339] [--zone NAME] [--accent RRGGBB] [--smooth]
                           [--size WxH] [--scale F] [--mode analog|digital|both] [--advanced]
                           [--confetti] [--screenshot OUT.png [--after-frames N]] [--no-antialiasing]
                           [--version]";

#[derive(Debug, Clone)]
pub struct Args {
    pub fixed_time: Option<DateTime<Utc>>,
    pub zone: usize,
    pub accent: Color,
    pub smooth: bool,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    pub mode: Mode,
    pub advanced: bool,
    pub confetti: bool,
    pub screenshot: Option<PathBuf>,
    pub after_frames: u32,
    pub antialiasing: bool,
    pub version: bool,
    pub help: bool,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            fixed_time: None,
            zone: 0,
            accent: DEFAULT_ACCENT,
            smooth: false,
            width: 900.0,
            height: 700.0,
            scale: 1.0,
            mode: Mode::Analog,
            advanced: false,
            confetti: false,
            screenshot: None,
            after_frames: 3,
            antialiasing: true,
            version: false,
            help: false,
        }
    }
}

impl Args {
    pub fn parse(mut raw: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = Args::default();
        while let Some(flag) = raw.next() {
            let mut value = || raw.next().ok_or_else(|| format!("{flag} needs a value"));
            match flag.as_str() {
                "--fixed-time" => {
                    let text = value()?;
                    let parsed = DateTime::parse_from_rfc3339(&text).map_err(|e| format!("--fixed-time {text}: {e}"))?;
                    args.fixed_time = Some(parsed.with_timezone(&Utc));
                }
                "--zone" => {
                    let text = value()?;
                    args.zone = ZONES
                        .iter()
                        .position(|(name, _)| name.eq_ignore_ascii_case(&text))
                        .ok_or_else(|| format!("unknown zone {text:?}"))?;
                }
                "--accent" => {
                    let text = value()?;
                    args.accent = widgets::parse_hex(&text).ok_or_else(|| format!("--accent {text}: expected RRGGBB"))?;
                }
                "--smooth" => args.smooth = true,
                "--size" => {
                    let text = value()?;
                    let parsed = text.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)));
                    (args.width, args.height) = parsed.ok_or_else(|| format!("--size {text}: expected WxH"))?;
                }
                "--scale" => {
                    let text = value()?;
                    args.scale = text.parse().map_err(|_| format!("--scale {text}: expected a number"))?;
                }
                "--mode" => {
                    let text = value()?;
                    args.mode = Mode::ALL
                        .into_iter()
                        .find(|m| m.label().eq_ignore_ascii_case(&text))
                        .ok_or_else(|| format!("--mode {text}: expected analog, digital or both"))?;
                }
                "--advanced" => args.advanced = true,
                "--confetti" => args.confetti = true,
                "--screenshot" => args.screenshot = Some(value()?.into()),
                "--after-frames" => {
                    let text = value()?;
                    args.after_frames = text.parse().map_err(|_| format!("--after-frames {text}: expected a count"))?;
                }
                "--no-antialiasing" => args.antialiasing = false,
                "--version" => args.version = true,
                "-h" | "--help" => args.help = true,
                other => return Err(format!("unknown argument {other:?}")),
            }
        }
        Ok(args)
    }
}

// ── App ─────────────────────────────────────────────────────────────────────

pub struct Clock {
    fixed_time: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    /// The instant update last ran; the view reads durations against it rather than the clock.
    at: Instant,
    mode: Mode,
    zone: Zone,
    use24h: bool,
    show_seconds: bool,
    smooth: bool,
    hand: HandStyle,
    scale: f32,
    dim_hours: (f32, f32),
    advanced: bool,
    numerals: bool,
    accent: Color,
    /// The theme's primary role: the Noctalia default until the accent picker changes it.
    primary: Color,
    hsv: [f32; 3],
    hex_draft: String,
    channel_drafts: [String; 3],
    timer_minutes: i32,
    timer: Countdown,
    stopwatch: Stopwatch,
    confetti: Option<Confetti>,
    confetti_seed: u64,
    last_frame: Option<Instant>,
    jitter: VecDeque<f32>,
    screenshot: Option<(PathBuf, u32)>,
    chrome: chrome::Chrome,
}

impl Clock {
    pub fn new(args: &Args) -> Self {
        let at = Instant::now();
        let timer_minutes = 5;
        let mut timer = Countdown::default();
        timer.reset(minutes(timer_minutes));
        let mut clock = Clock {
            fixed_time: args.fixed_time,
            now: args.fixed_time.unwrap_or_else(Utc::now),
            at,
            mode: args.mode,
            zone: Zone(args.zone),
            use24h: true,
            show_seconds: true,
            smooth: args.smooth,
            hand: HandStyle::Classic,
            scale: 1.0,
            dim_hours: (22.0, 6.0),
            advanced: args.advanced,
            numerals: true,
            accent: args.accent,
            primary: theme::PRIMARY,
            hsv: widgets::color_to_hsv(args.accent),
            hex_draft: String::new(),
            channel_drafts: Default::default(),
            timer_minutes,
            timer,
            stopwatch: Stopwatch::default(),
            confetti: None,
            confetti_seed: 1,
            last_frame: None,
            jitter: VecDeque::with_capacity(JITTER_SAMPLES),
            screenshot: args.screenshot.clone().map(|path| (path, args.after_frames)),
            chrome: chrome::initial(),
        };
        clock.sync_drafts();
        if args.confetti {
            clock.fire_confetti();
        }
        clock
    }

    pub fn theme(&self) -> Theme {
        theme::noctalia(self.primary)
    }

    fn offset_minutes(&self) -> i32 {
        let naive = self.now.naive_utc();
        let seconds = match ZONES[self.zone.0].1 {
            Some(tz) => tz.offset_from_utc_datetime(&naive).fix().local_minus_utc(),
            None => chrono::Local.offset_from_utc_datetime(&naive).fix().local_minus_utc(),
        };
        seconds / 60
    }

    fn zone_hour(&self) -> f32 {
        let local = self.now + TimeDelta::minutes(i64::from(self.offset_minutes()));
        local.hour() as f32 + local.minute() as f32 / 60.0
    }

    fn dimmed(&self) -> bool {
        let (low, high) = self.dim_hours;
        let hour = self.zone_hour();
        if low <= high { hour >= low && hour < high } else { hour >= low || hour < high }
    }

    fn fire_confetti(&mut self) {
        self.confetti = Some(Confetti::new(self.confetti_seed));
        self.confetti_seed += 1;
        self.last_frame = None;
    }

    fn record_jitter(&mut self) {
        let millis = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.subsec_millis());
        let jitter = if millis >= 500 { millis as f32 - 1000.0 } else { millis as f32 };
        if self.jitter.len() == JITTER_SAMPLES {
            self.jitter.pop_front();
        }
        self.jitter.push_back(jitter);
    }

    fn jitter_series(&self) -> Vec<f32> {
        let mut series: Vec<f32> = self.jitter.iter().map(|ms| ((ms + 50.0) / 100.0).clamp(0.0, 1.0)).collect();
        while series.len() < 2 {
            series.push(0.5);
        }
        series
    }

    fn status(&self) -> String {
        format!(
            "{} · UTC{} · {}{}",
            self.zone,
            format_offset(self.offset_minutes()),
            self.mode.label(),
            if self.dimmed() { " · night" } else { "" }
        )
    }

    fn set_accent(&mut self, color: Color, hsv: Option<[f32; 3]>) {
        self.accent = color;
        self.primary = color;
        self.hsv = hsv.unwrap_or_else(|| widgets::color_to_hsv(color));
        self.sync_drafts();
    }

    fn sync_drafts(&mut self) {
        self.hex_draft = widgets::to_hex(self.accent);
        self.channel_drafts = widgets::to_rgb8(self.accent).map(|c| c.to_string());
    }

    fn busy(&self) -> bool {
        self.timer.running() || self.stopwatch.running()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        self.at = Instant::now();
        match message {
            Message::SetMode(mode) => self.mode = mode,
            Message::SetZone(zone) => self.zone = zone,
            Message::Use24h(on) => self.use24h = on,
            Message::ShowSeconds(on) => self.show_seconds = on,
            Message::SmoothSweep(on) => self.smooth = on,
            Message::Hand(style) => self.hand = style,
            Message::FaceScale(scale) => self.scale = scale.clamp(0.5, 2.0),
            Message::DimRange(low, high) => self.dim_hours = (low.round(), high.round()),
            Message::Advanced(expanded) => self.advanced = expanded,
            Message::ShowNumerals(on) => self.numerals = on,
            Message::FireConfetti => self.fire_confetti(),
            Message::TimerMinutes(value) => {
                self.timer_minutes = value.clamp(1, 120);
                if !self.timer.running() {
                    self.timer.reset(minutes(self.timer_minutes));
                }
            }
            Message::TimerStartPause => {
                if self.timer.running() {
                    self.timer.pause(self.at);
                } else {
                    if self.timer.remaining(self.at).is_zero() {
                        self.timer.reset(minutes(self.timer_minutes));
                    }
                    self.timer.start(self.at);
                }
            }
            Message::TimerReset => self.timer.reset(minutes(self.timer_minutes)),
            Message::StopwatchStartStop => match self.stopwatch.since.take() {
                Some(since) => self.stopwatch.accumulated += self.at.saturating_duration_since(since),
                None => self.stopwatch.since = Some(self.at),
            },
            Message::StopwatchLap => {
                if self.stopwatch.running() {
                    let elapsed = self.stopwatch.elapsed(self.at);
                    self.stopwatch.laps.push(elapsed);
                }
            }
            Message::StopwatchReset => self.stopwatch = Stopwatch::default(),
            Message::Picker(PickerEvent::Hsv(h, s, v)) => self.set_accent(widgets::hsv_to_color(h, s, v), Some([h, s, v])),
            Message::Picker(PickerEvent::Hex(draft)) => {
                match widgets::parse_hex(&draft) {
                    Some(color) if draft.trim().len() == 7 => self.set_accent(color, None),
                    _ => {}
                }
                self.hex_draft = draft;
            }
            Message::Picker(PickerEvent::HexSubmit) => match widgets::parse_hex(&self.hex_draft) {
                Some(color) => self.set_accent(color, None),
                None => self.sync_drafts(),
            },
            Message::Picker(PickerEvent::Channel(index, draft)) => {
                if let Ok(value) = draft.trim().parse::<u8>() {
                    let mut rgb = widgets::to_rgb8(self.accent);
                    rgb[index] = value;
                    self.set_accent(Color::from_rgb8(rgb[0], rgb[1], rgb[2]), None);
                }
                self.channel_drafts[index] = draft;
            }
            Message::Tick => {
                self.now = self.fixed_time.unwrap_or_else(Utc::now);
                self.record_jitter();
                if self.timer.running() && self.timer.remaining(self.at).is_zero() {
                    self.timer.pause(self.at);
                    self.fire_confetti();
                }
            }
            Message::StopwatchTick => {}
            Message::Frame(at) => {
                if self.smooth {
                    self.now = self.fixed_time.unwrap_or_else(Utc::now);
                }
                let dt_ms = self.last_frame.map_or(0.0, |last| at.saturating_duration_since(last).as_secs_f64() * 1000.0);
                self.last_frame = Some(at);
                if let Some(confetti) = self.confetti.as_mut() {
                    confetti.step(dt_ms);
                    if confetti.done() {
                        self.confetti = None;
                    }
                }
                if let Some((_, frames)) = self.screenshot.as_mut() {
                    if *frames == 0 {
                        return window::latest().and_then(window::screenshot).map(Message::Screenshot);
                    }
                    *frames -= 1;
                }
            }
            Message::Chrome(chrome::Action::Changed(chrome)) => self.chrome = chrome,
            Message::Chrome(action) => return chrome::perform(action).map(Message::Chrome),
            Message::Screenshot(shot) => {
                if let Some((path, _)) = self.screenshot.take() {
                    match write_png(&path, &shot) {
                        Ok(()) => println!(
                            "noctalia-clock-iced: {} ({}x{} at scale {})",
                            path.display(),
                            shot.size.width,
                            shot.size.height,
                            shot.scale_factor
                        ),
                        Err(error) => eprintln!("noctalia-clock-iced: {}: {error}", path.display()),
                    }
                    return iced::exit();
                }
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let total = minutes(self.timer_minutes);
        let left = self.timer.remaining(self.at);
        let fraction = (left.as_secs_f64() / total.as_secs_f64()) as f32;
        let timer_active = self.timer.running() || left < total;

        let face = Face {
            mode: self.mode,
            use24h: self.use24h,
            show_seconds: self.show_seconds,
            smooth: self.smooth,
            numerals: self.numerals,
            show_date: true,
            utc_ms: self.now.timestamp_millis() as f64,
            offset_minutes: self.offset_minutes(),
            size: BASE_FACE_SIZE * self.scale,
            hand: self.hand,
            arc: if timer_active { fraction } else { 0.0 },
            dim: if self.dimmed() { 0.45 } else { 0.0 },
            accent: self.accent,
        };

        let face_column = column![
            text("Noctalia Clock").size(theme::FONT_HEADER),
            face.view(),
            canvas(confetti::View(self.confetti.as_ref())).width(confetti::WIDTH).height(confetti::HEIGHT),
            text(self.status()).size(theme::FONT_BODY),
        ]
        .spacing(theme::SPACE_MD)
        .align_x(Alignment::Center)
        .width(confetti::WIDTH + 20.0);

        let controls = column![
            setting(
                "Mode",
                widgets::segmented(&Mode::ALL.map(Mode::label), self.mode as usize, |i| Message::SetMode(Mode::ALL[i])),
            ),
            setting(
                "Timezone",
                pick_list(Zone::ALL, Some(self.zone), Message::SetZone)
                    .width(200)
                    .padding([10.0, theme::SPACE_MD])
                    .text_size(theme::FONT_BODY)
                    .handle(pick_list::Handle::Dynamic {
                        closed: chevron(theme::icon::CHEVRON_DOWN),
                        open: chevron(theme::icon::CHEVRON_UP),
                    })
                    .style(theme::pick_list_style)
                    .menu_style(theme::menu_style),
            ),
            setting("24-hour", widgets::toggle(self.use24h, Message::Use24h)),
            setting("Show seconds", check(self.show_seconds, Message::ShowSeconds)),
            setting("Smooth sweep", widgets::toggle(self.smooth, Message::SmoothSweep)),
            setting("Hands", self.hands()),
            setting(
                "Face size",
                slider(0.5..=2.0, self.scale, Message::FaceScale).step(0.05_f32).width(200).style(theme::slider_style),
            ),
            setting(
                format!("Night {:02}–{:02}h", self.dim_hours.0, self.dim_hours.1),
                range_slider(0.0..=24.0, self.dim_hours, Message::DimRange).step(1.0).width(200),
            ),
            widgets::collapsible("Advanced", self.advanced, Message::Advanced, self.advanced_body()),
            text("Timer").size(theme::FONT_TITLE),
            self.timer_row(fraction, left),
            text("Stopwatch").size(theme::FONT_TITLE),
            self.stopwatch_rows(),
            setting(
                "Accent",
                widgets::color_picker(self.hsv, &self.hex_draft, &self.channel_drafts, 300.0, Message::Picker),
            ),
        ]
        .spacing(10)
        .padding(4);

        let controls = scrollable(controls)
            .direction(Direction::Vertical(Scrollbar::new().width(6).scroller_width(6).spacing(theme::SPACE_SM)))
            .style(theme::scrollable_style)
            .width(Length::Fill)
            .height(Length::Fill);

        chrome::frame(
            self.chrome,
            "Noctalia Clock",
            row![face_column, controls].spacing(20).padding(20),
            Message::Chrome,
        )
    }

    fn hands(&self) -> Element<'_, Message> {
        row(HandStyle::ALL.map(|style| {
            radio(style.to_string(), style, Some(self.hand), Message::Hand)
                .size(20)
                .spacing(10)
                .text_size(theme::FONT_BODY)
                .style(theme::radio_style)
                .into()
        }))
        .spacing(14)
        .align_y(Alignment::Center)
        .into()
    }

    fn advanced_body(&self) -> Element<'_, Message> {
        column![
            setting("Numerals", check(self.numerals, Message::ShowNumerals)),
            text("Tick jitter (ms, ±50)").size(theme::FONT_BODY),
            widgets::graph(self.jitter_series(), 320.0, 56.0, self.accent),
            action("Fire confetti", ButtonVariant::Primary, Some(Message::FireConfetti)),
        ]
        .spacing(theme::SPACE_SM)
        .into()
    }

    fn timer_row(&self, fraction: f32, left: Duration) -> Element<'_, Message> {
        let running = self.timer.running();
        row![
            widgets::countdown_ring(fraction, left.as_secs_f64().ceil() as u64, 56.0, 5.0, self.accent),
            widgets::stepper(self.timer_minutes, 1, 120, " min", Message::TimerMinutes),
            action(if running { "Pause" } else { "Start" }, ButtonVariant::Default, Some(Message::TimerStartPause)),
            action("Reset", ButtonVariant::Default, Some(Message::TimerReset)),
        ]
        .spacing(theme::SPACE_MD)
        .align_y(Alignment::Center)
        .into()
    }

    fn stopwatch_rows(&self) -> Element<'_, Message> {
        let running = self.stopwatch.running();
        let mut controls = row![
            text(format_stopwatch(self.stopwatch.elapsed(self.at))).size(18).width(110),
            action(if running { "Stop" } else { "Start" }, ButtonVariant::Default, Some(Message::StopwatchStartStop)),
            action("Lap", ButtonVariant::Default, running.then_some(Message::StopwatchLap)),
            action("Reset", ButtonVariant::Default, Some(Message::StopwatchReset)),
        ]
        .spacing(theme::SPACE_MD)
        .align_y(Alignment::Center);
        if self.busy() {
            let phase = (self.at.elapsed() + epoch_phase()).as_secs_f32() / SPINNER_REVOLUTION.as_secs_f32();
            controls = controls.push(widgets::spinner(phase.fract(), 18.0, self.primary));
        }
        let laps = self.stopwatch.laps.iter().enumerate().rev().take(5).map(|(index, lap)| {
            text(format!("Lap {}  {}", index + 1, format_stopwatch(*lap))).size(theme::FONT_BODY).into()
        });
        column![controls].extend(laps).spacing(6).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![Subscription::run(aligned_seconds), chrome::events().map(Message::Chrome)];
        if self.stopwatch.running() {
            subscriptions.push(iced::time::every(Duration::from_millis(50)).map(|_| Message::StopwatchTick));
        }
        let animating = self.confetti.is_some()
            || self.busy()
            || self.screenshot.is_some()
            || (self.smooth && self.fixed_time.is_none());
        if animating {
            subscriptions.push(window::frames().map(Message::Frame));
        }
        Subscription::batch(subscriptions)
    }
}

fn check(on: bool, on_toggle: fn(bool) -> Message) -> Element<'static, Message> {
    checkbox(on)
        .size(20)
        .icon(checkbox::Icon {
            font: theme::ICON_FONT,
            code_point: theme::icon::CHECK,
            size: Some(16.into()),
            line_height: text::LineHeight::Relative(1.0),
            shaping: text::Shaping::Basic,
        })
        .style(theme::checkbox_style)
        .on_toggle(on_toggle)
        .into()
}

fn chevron(code_point: char) -> pick_list::Icon<iced::Font> {
    pick_list::Icon {
        font: theme::ICON_FONT,
        code_point,
        size: Some(theme::FONT_BODY.into()),
        line_height: text::LineHeight::Relative(1.0),
        shaping: text::Shaping::Basic,
    }
}

/// Spinner phase is wall-clock based so every redraw agrees on the angle.
fn epoch_phase() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()
}

/// Ticks just after each wall-clock second, so the second hand moves on the second.
fn aligned_seconds() -> impl iced::futures::Stream<Item = Message> {
    iced::futures::stream::unfold((), |()| async {
        let millis = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.subsec_millis());
        tokio::time::sleep(Duration::from_millis(u64::from(1000 - millis) + 1)).await;
        Some((Message::Tick, ()))
    })
}

fn minutes(count: i32) -> Duration {
    Duration::from_secs(u64::from(count.unsigned_abs()) * 60)
}

pub fn format_stopwatch(elapsed: Duration) -> String {
    let centis = elapsed.as_millis() / 10;
    format!("{:02}:{:02}.{:02}", centis / 6000, (centis / 100) % 60, centis % 100)
}

pub fn format_offset(minutes: i32) -> String {
    let sign = if minutes < 0 { '-' } else { '+' };
    let magnitude = minutes.unsigned_abs();
    format!("{sign}{:02}:{:02}", magnitude / 60, magnitude % 60)
}

fn write_png(path: &std::path::Path, shot: &window::Screenshot) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, shot.size.width, shot.size.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&shot.rgba)?;
    Ok(())
}
