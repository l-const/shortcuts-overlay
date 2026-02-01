mod args;
mod blur;
mod config;
mod input_listener;
mod keybinding_reader;
mod overlay;
mod singleton;
mod state;
mod util;
mod watcher;

use anyhow::{Context, Result};
use clap::Parser;
use keybinding_reader::load_cosmic_shortcuts;
use singleton::SingletonGuard;
use state::State;
use std::sync::{Arc, Mutex};
use util::{get_xdg_desktop, XDGDesktop};

fn main() -> Result<()> {
    env_logger::init();
    log::info!("Starting shortcuts-overlay");

    let opts = crate::args::Opt::parse();

    let overlay_config = config::OverlayConfig::load().context("Failed to load config")?;

    let config = crate::util::merge_cli_opts_config(&overlay_config, &opts);

    let width = config.width;
    let height = config.height;

    std::env::set_var("SHORTCUTS_OVERLAY_WIDTH", width.to_string());
    std::env::set_var("SHORTCUTS_OVERLAY_HEIGHT", height.to_string());

    // Ensure only one instance is running
    let _guard = SingletonGuard::acquire()?;
    log::info!("Starting shortcuts-overlay (size {}x{})", width, height);

    // Initialize XDG desktop environment
    let xdg_desktop = get_xdg_desktop();
    log::info!("Detected XDG desktop environment: {:?}", &xdg_desktop);
    // Load keyboard shortcuts based on XDG desktop environment
    let shortcuts = match xdg_desktop {
        XDGDesktop::COSMIC => load_cosmic_shortcuts().context("Failed to load cosmic shortcuts")?,
        XDGDesktop::NIRI => unimplemented!(),
    };
    // log::info!("Found {} shortcuts to display", shortcuts.len());

    let overlay_config = config::OverlayConfig::load().context("Failed to load config")?;

    log::info!("Loaded overlay config: {:?}", overlay_config);

    let state = Arc::new(State::new(
        Arc::new(Mutex::new(overlay_config.clone())),
        Arc::new(Mutex::new(shortcuts.clone())),
    ));

    // watcher code
    let state_clone = Arc::clone(&state);
    let handle = std::thread::spawn(|| {
        watcher::watch_overlay_config(state_clone).unwrap();
    });
    // start
    overlay::start(state)?;

    handle.join().unwrap();

    Ok(())
}
