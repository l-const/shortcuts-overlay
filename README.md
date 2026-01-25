# wl-shortcuts-overlay

A keyboard shortcuts overlay for Wayland desktops that displays your compositor's keybindings in a semi-transparent overlay surface.

## Features

- **Wayland Native**: Built using smithay-client-toolkit for native Wayland support
- **Layer Shell**: Uses wlr-layer-shell protocol for overlay functionality
- **Singleton Instance**: Ensures only one instance runs at a time using file locking
- **Compositor Integration**: Automatically reads keybindings from popular Wayland compositor configs
- **XKB Support**: Uses xkbcommon for proper keyboard symbol handling (following COSMIC patterns)
- **Keyboard Control**: Toggle overlay visibility with the Escape key
- **Semi-transparent UI**: Displays shortcuts with a blurred background effect

## Supported Compositors

The overlay automatically detects and reads keyboard shortcuts from:

- **Sway**: `~/.config/sway/config`
- **Hyprland**: `~/.config/hyprland/hyprland.conf`  
- **River**: `~/.config/river/init`
- **Wayfire**: `~/.config/wayfire.ini`
- **i3**: `~/.config/i3/config` (for compatibility)

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

## Usage

Run the overlay daemon:

```bash
./target/release/wl-shortcuts-overlay
```

Or install it system-wide:

```bash
cargo install --path .
wl-shortcuts-overlay
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

## Configuration

The application automatically discovers shortcuts from your compositor's config file. No additional configuration is needed.

## Architecture

The project consists of several modules:

- **singleton**: Implements file-based locking to ensure single instance
- **keybinding_reader**: Parses compositor configs and uses xkbcommon for proper key representation
- **shortcut_reader**: Legacy XDG desktop entry parser (kept for reference)
- **overlay**: Manages the Wayland layer shell surface and rendering  
- **main**: Coordinates initialization and event loop

## Inspiration

This project follows keyboard shortcut handling patterns from:
- [COSMIC Settings Daemon](https://github.com/pop-os/cosmic-settings-daemon) - For proper xkbcommon integration
- [Smithay Client Toolkit](https://github.com/Smithay/client-toolkit) - For Wayland layer shell examples
- [Vibe](https://github.com/TornaxO7/vibe) - For Wayland application architecture

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
