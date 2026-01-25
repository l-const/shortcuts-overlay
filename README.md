# wl-shortcuts-overlay

A shortcuts overlay for Wayland desktops that displays desktop applications and their shortcuts in a semi-transparent overlay surface.

## Features

- **Wayland Native**: Built using smithay-client-toolkit for native Wayland support
- **Layer Shell**: Uses wlr-layer-shell protocol for overlay functionality
- **Singleton Instance**: Ensures only one instance runs at a time using file locking
- **XDG Compliant**: Reads desktop entries from standard XDG directories
- **Keyboard Control**: Toggle overlay visibility with the Escape key
- **Semi-transparent UI**: Displays shortcuts with a blurred background effect

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
- The overlay displays desktop applications found in your XDG application directories

## How It Works

1. **Singleton Pattern**: On startup, the application acquires an exclusive file lock to prevent multiple instances
2. **XDG Discovery**: Scans `~/.local/share/applications` and `/usr/share/applications` for .desktop files
3. **Wayland Surface**: Creates a layer shell surface with overlay layer priority
4. **Interactive Display**: Shows discovered applications and responds to keyboard input

## Configuration

The application automatically discovers desktop entries from:
- `$HOME/.local/share/applications`
- `/usr/local/share/applications`
- `/usr/share/applications`
- Custom paths defined in `$XDG_DATA_DIRS`

## Development

### Running Tests

```bash
cargo test
```

### Building for Development

```bash
cargo build
RUST_LOG=info ./target/debug/wl-shortcuts-overlay
```

## Architecture

The project consists of several modules:

- **singleton**: Implements file-based locking to ensure single instance
- **shortcut_reader**: Parses XDG desktop entries to discover applications
- **overlay**: Manages the Wayland layer shell surface and rendering
- **main**: Coordinates initialization and event loop

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
