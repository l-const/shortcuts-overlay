mod blur;
mod config;
mod input_listener;
mod keybinding_reader;
mod overlay;
mod singleton;
mod util;

use smithay_client_toolkit::shell::wlr_layer::Anchor;

// README note:
// The overlay default size and position can be configured via CLI flags or environment
// variables. CLI flags:
//   --width <PX>     overlay client width (default: 1200)
//   --height <PX>    overlay client height (default: 800)
//   --anchor <POS>   overlay anchor position (default: center)
//                    Available: center, topleft, topright, bottomleft, bottomright,
//                               top, bottom, left, right
// Environment variables (alternative):
//   SHORTCUTS_OVERLAY_WIDTH
//   SHORTCUTS_OVERLAY_HEIGHT
//
use anyhow::{Context, Result};
use keybinding_reader::load_cosmic_shortcuts;
use singleton::SingletonGuard;

use clap::Parser;

/// CLI options for the shortcuts overlay.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Opt {
    /// Overlay width in pixels (client size). If omitted, default 1200 is used.
    #[arg(long)]
    width: Option<u32>,

    /// Overlay height in pixels (client size). If omitted, default 800 is used.
    #[arg(long)]
    height: Option<u32>,
    /// Overlay anchor position. If omitted, default is center.
    /// Available: center, topleft, topright, bottomleft, bottomright,
    ///                               top, bottom, left, right
    #[arg(long)]
    anchor: Option<String>,
}

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    log::info!("Starting shortcuts-overlay");

    let opts = Opt::parse();

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

    // start
    overlay::start(shortcuts, overlay_config)?;

    Ok(())
}
