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

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    log::info!("Starting shortcuts-overlay");

    let opts = crate::args::Opt::parse();

    let overlay_config = config::OverlayConfig::load().context("Failed to load config")?;

    let config = crate::util::merge_cli_opts_config(&overlay_config, &opts);

    let width = config.width;
    let height = config.height;

    // TODO(l-const):
    // A. subscription to the dbus for shortcuts update
    // B. configurable line-height, font-size, font-family
    // overlapping text due to cosmic-text lack of ellipsis support
    std::env::set_var("SHORTCUTS_OVERLAY_WIDTH", width.to_string());
    std::env::set_var("SHORTCUTS_OVERLAY_HEIGHT", height.to_string());

    // Ensure only one instance is running
    let _guard = SingletonGuard::acquire()?;
    log::info!("Starting shortcuts-overlay (size {}x{})", width, height);

    // Load keyboard shortcuts from Cosmic settings (Pop!_OS)
    let shortcuts = load_cosmic_shortcuts().context("Failed to load cosmic shortcuts")?;
    // log::info!("Found {} shortcuts to display", shortcuts.len());

    let overlay_config = config::OverlayConfig::load().context("Failed to load config")?;

    log::info!("Loaded overlay config: {:?}", overlay_config);

    let _state = State::new(
        Arc::new(Mutex::new(overlay_config.clone())),
        Arc::new(Mutex::new(shortcuts.clone())),
    );
    // watcher code

    // start
    overlay::start(shortcuts, overlay_config)?;

    Ok(())
}
