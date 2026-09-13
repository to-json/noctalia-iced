//! Probe for the patched window chrome: a 600x400 geometry window with a 16px shadow margin.
//!
//!   chrome-probe [--maximize-after-first-chrome] [--screenshot OUT.png]

use iced::widget::{column, container, text};
use iced::window::{self, Chrome};
use iced::{Color, Element, Length, Shadow, Size, Subscription, Task, Vector, border};
use std::path::PathBuf;
use std::time::Duration;

const MARGIN: u32 = 16;

#[derive(Debug, Clone)]
enum Message {
    Chrome(Chrome),
    Resized(Size),
    Tick,
    Shot(window::Screenshot),
}

struct Probe {
    chrome: Chrome,
    size: Size,
    maximize: bool,
    screenshot: Option<PathBuf>,
    chrome_events: u32,
    ticks: u32,
    shot_requested: bool,
}

fn main() -> iced::Result {
    let mut maximize = false;
    let mut screenshot = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--maximize-after-first-chrome" => maximize = true,
            "--screenshot" => screenshot = Some(PathBuf::from(args.next().expect("--screenshot PATH"))),
            other => panic!("unknown argument {other}"),
        }
    }

    let boot = move || Probe {
        chrome: Chrome::default(),
        size: Size::ZERO,
        maximize,
        screenshot: screenshot.clone(),
        chrome_events: 0,
        ticks: 0,
        shot_requested: false,
    };

    #[allow(unused_mut)]
    let mut settings = window::Settings {
        size: Size::new(600.0, 400.0),
        decorations: false,
        transparent: true,
        ..window::Settings::default()
    };
    #[cfg(target_os = "linux")]
    {
        settings.platform_specific.application_id = "dev.wawonaplay.ChromeProbe".into();
        settings.platform_specific.shadow_margin = MARGIN;
    }

    iced::application(boot, update, view)
        .title("chrome-probe")
        .style(|_, _| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: Color::WHITE })
        .subscription(subscription)
        .window(settings)
        .run()
}

/// Settled: floating after the first chrome event, or maximized when that was requested.
fn settled(p: &Probe) -> bool {
    p.chrome_events > 0 && (!p.maximize || p.chrome.maximized)
}

fn update(p: &mut Probe, message: Message) -> Task<Message> {
    match message {
        Message::Chrome(chrome) => {
            println!("ChromeChanged {chrome:?}");
            p.chrome = chrome;
            p.chrome_events += 1;
            if p.maximize && p.chrome_events == 1 {
                return window::latest().and_then(|id| window::maximize(id, true));
            }
        }
        Message::Resized(size) => {
            println!("Resized {}x{}", size.width, size.height);
            p.size = size;
        }
        Message::Tick => {
            p.ticks += 1;
            // Give the settled state a tick to render; give up waiting after ~2s (e.g. macOS,
            // which never reports chrome).
            let ready = (settled(p) && p.ticks >= 2) || p.ticks >= 5;
            if ready && !p.shot_requested && p.screenshot.is_some() {
                p.shot_requested = true;
                return window::latest().and_then(window::screenshot).map(Message::Shot);
            }
        }
        Message::Shot(shot) => {
            let path = p.screenshot.clone().expect("screenshot path");
            write_png(&path, &shot).expect("write screenshot");
            println!("screenshot {} {}x{} scale {}", path.display(), shot.size.width, shot.size.height, shot.scale_factor);
            return iced::exit();
        }
    }
    Task::none()
}

fn subscription(p: &Probe) -> Subscription<Message> {
    let events = iced::event::listen_with(|event, _, _| match event {
        iced::Event::Window(window::Event::ChromeChanged(chrome)) => Some(Message::Chrome(chrome)),
        iced::Event::Window(window::Event::Resized(size)) => Some(Message::Resized(size)),
        _ => None,
    });
    if p.screenshot.is_some() {
        Subscription::batch([events, iced::time::every(Duration::from_millis(400)).map(|_| Message::Tick)])
    } else {
        events
    }
}

fn view(p: &Probe) -> Element<'_, Message> {
    let floating = p.chrome.is_floating();
    let body = container(
        column![
            text(format!("{:?}", p.chrome)).size(13),
            text(format!("surface {}x{}", p.size.width, p.size.height)).size(13),
        ]
        .spacing(6),
    )
    .padding(16)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_| container::Style {
        background: Some(Color::from_rgb8(0x07, 0x07, 0x22).into()),
        text_color: Some(Color::WHITE),
        border: border::rounded(if floating { 12 } else { 0 }).color(Color::from_rgb8(0x21, 0x21, 0x5f)).width(1),
        shadow: if floating {
            Shadow { color: Color { a: 0.55, ..Color::BLACK }, offset: Vector::new(0.0, 4.0), blur_radius: 14.0 }
        } else {
            Shadow::default()
        },
        snap: true,
    });
    container(body).padding(p.chrome.shadow_margin).into()
}

fn write_png(path: &std::path::Path, shot: &window::Screenshot) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, shot.size.width, shot.size.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&shot.rgba)?;
    Ok(())
}
