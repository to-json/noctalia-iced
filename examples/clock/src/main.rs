//! noctalia-clock-iced: libnoctalia-ui's noctalia-clock demo on iced and noctalia-iced.

mod app;
mod confetti;
mod face;

#[cfg(test)]
mod tests;

use app::{Args, Clock, USAGE};
use iced::Size;
use noctalia_iced::{chrome, theme};
use std::process::ExitCode;

const APP_ID: &str = "dev.wawonaplay.NoctaliaClockIced";

fn main() -> ExitCode {
    let args = match Args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("noctalia-clock-iced: {error}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if args.help {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if args.version {
        println!("noctalia-clock-iced {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let scale = args.scale;
    let antialiasing = args.antialiasing;
    let window = chrome::settings(Size::new(args.width, args.height), Size::new(640.0, 480.0), APP_ID);
    let result = iced::application(move || Clock::new(&args), Clock::update, Clock::view)
        .title("Noctalia Clock")
        .theme(Clock::theme)
        .style(|_, _| iced::theme::Style { background_color: chrome::background(), text_color: theme::ON_SURFACE })
        .subscription(Clock::subscription)
        .scale_factor(move |_| scale)
        .font(theme::ICON_FONT_BYTES)
        .window(window)
        .antialiasing(antialiasing)
        .run();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("noctalia-clock-iced: {error}");
            ExitCode::FAILURE
        }
    }
}
