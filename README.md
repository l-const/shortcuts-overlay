# A shortcuts overlay for the COSMIC DE.

A keyboard shortcuts overlay for COSMIC DE in a semi-transparent overlay surface.

## Features

- **Wayland Native**: Built using smithay-client-toolkit for native Wayland support
- **Layer Shell**: Uses wlr-layer-shell protocol for overlay functionality
- **Singleton Instance**: Ensures only one instance runs at a time using file locking
- **Keyboard Control**: Toggle overlay visibility with the Escape key
- **Semi-transparent UI**: Displays shortcuts with a blurred background effect

## Supported Compositors

The overlay automatically detects and reads keyboard shortcuts from:

- **COSMIC**: `~/.config/cosmic/config`
If no config is found, common default shortcuts are displayed.

## Requirements

- A Wayland compositor that supports the wlr-layer-shell protocol (e.g., Sway, Hyprland, River)
- Rust 1.70 or later
- libwayland-dev
- libxkbcommon-dev

## Installation

### From Source

```bash
# Install system dependencies (Ubuntu/Debian)
sudo apt-get install libwayland-dev libxkbcommon-dev

# Clone and build
git clone https://github.com/l-const/wl-shortcuts-overlay.git
cd wl-shortcuts-overlay
cargo build --release
```

### Usage

- Run the application:
```bash
./target/release/wl-shortcuts-overlay
```

- CLI options
  - `--width <PX>`  — overlay client width in pixels (default: 800)
  - `--height <PX>` — overlay client height in pixels (default: 600)

- Environment variables (alternative to CLI)
  - `SHORTCUTS_OVERLAY_WIDTH` — overlay client width in pixels
  - `SHORTCUTS_OVERLAY_HEIGHT` — overlay client height in pixels

- Examples:
```bash
# Run with explicit size via CLI
./target/release/wl-shortcuts-overlay --width 800 --height 600

# Run with env vars
SHORTCUTS_OVERLAY_WIDTH=900 SHORTCUTS_OVERLAY_HEIGHT=500 ./target/release/wl-shortcuts-overlay
```

### Keyboard Shortcuts

- **Escape**: Toggle overlay visibility
- The overlay displays keyboard shortcuts found in your compositor's config file

### Example Output

The overlay displays shortcuts like:
```
Super + Return: Launch terminal
Super + d: Launch application launcher  
Super + Shift + q: Close window
Super + f: Toggle fullscreen
Alt + Tab: Cycle windows
```

## How It Works

1. **Singleton Pattern**: On startup, the application acquires an exclusive file lock to prevent multiple instances
2. **Config Discovery**: Scans `~/.config/` for supported compositor configuration files
3. **XKB Integration**: Uses xkbcommon to properly parse and represent keyboard symbols
4. **Wayland Surface**: Creates a layer shell surface with overlay layer priority
5. **Interactive Display**: Shows discovered shortcuts and responds to keyboard input
