#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod battery;
mod config;
mod icon;
mod logging;
mod startup;
mod tray;

fn main() {
    let _log_guard = match logging::init() {
        Ok(guard) => Some(guard),
        Err(error) => {
            eprintln!("failed to initialize logging: {error}");
            None
        }
    };

    if let Err(error) = app::run() {
        tracing::error!(%error, "application exited with an error");
        eprintln!("{error}");
    }
}
