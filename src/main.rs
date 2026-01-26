mod keybinding_reader;
mod overlay;
mod singleton;

// README note:
// The overlay default size can be configured via CLI flags or environment
// variables. CLI flags:
//   --width <PX>     overlay client width (default: 800)
//   --height <PX>    overlay client height (default: 600)
// Environment variables (alternative):
//   SHORTCUTS_OVERLAY_WIDTH
//   SHORTCUTS_OVERLAY_HEIGHT
//
use anyhow::{Context, Result};
use keybinding_reader::load_cosmic_shortcuts;
use singleton::SingletonGuard;

// Use clap for CLI parsing (added as a dependency)
use clap::Parser;

/// CLI options for the shortcuts overlay.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Opt {
    /// Overlay width in pixels (client size). If omitted, default 800 is used.
    #[arg(long)]
    width: Option<u32>,

    /// Overlay height in pixels (client size). If omitted, default 600 is used.
    #[arg(long)]
    height: Option<u32>,
}

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    log::info!("Starting shortcuts-overlay");

    // Parse CLI options
    let opts = Opt::parse();
    // Default size if not provided
    let width = opts.width.unwrap_or(800);
    let height = opts.height.unwrap_or(600);

    // Export chosen size via environment variables so the overlay (which runs
    // in the same process) can read them if implemented to do so. This keeps
    // the wiring simple without changing the overlay API.
    std::env::set_var("SHORTCUTS_OVERLAY_WIDTH", width.to_string());
    std::env::set_var("SHORTCUTS_OVERLAY_HEIGHT", height.to_string());

    // Ensure only one instance is running
    let _guard = SingletonGuard::acquire()?;
    log::info!("Starting shortcuts-overlay (size {}x{})", width, height);

    // Load keyboard shortcuts from Cosmic settings (Pop!_OS)
    let shortcuts = load_cosmic_shortcuts().context("Failed to load cosmic shortcuts")?;
    log::info!("Found {} shortcuts to display", shortcuts.len());

    // Run the Wayland overlay
    overlay::run_overlay(shortcuts)?;

    Ok(())
}
