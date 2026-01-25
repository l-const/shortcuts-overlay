mod overlay;
mod shortcut_reader;
mod singleton;

use anyhow::Result;
use shortcut_reader::ShortcutReader;
use singleton::SingletonGuard;

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    // Ensure only one instance is running
    let _guard = SingletonGuard::acquire()?;
    log::info!("Starting wl-shortcuts-overlay");

    // Load desktop shortcuts from XDG directories
    let mut reader = ShortcutReader::new();
    reader.load_shortcuts()?;
    
    let shortcuts = reader.get_entries().to_vec();
    log::info!("Found {} shortcuts to display", shortcuts.len());

    // Run the Wayland overlay
    overlay::run_overlay(shortcuts)?;

    Ok(())
}
