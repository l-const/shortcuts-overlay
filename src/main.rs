mod overlay;
mod keybinding_reader;
mod shortcut_reader;
mod singleton;

use anyhow::Result;
use keybinding_reader::ShortcutReader;
use singleton::SingletonGuard;

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    // Ensure only one instance is running
    let _guard = SingletonGuard::acquire()?;
    log::info!("Starting wl-shortcuts-overlay");

    // Load keyboard shortcuts from compositor configs
    let mut reader = ShortcutReader::new();
    reader.load_shortcuts()?;
    
    let shortcuts = reader.get_bindings().to_vec();
    log::info!("Found {} shortcuts to display", shortcuts.len());

    // Run the Wayland overlay
    overlay::run_overlay(shortcuts)?;

    Ok(())
}
