# Design and Standards Compliance

This document outlines the design decisions and standards compliance for the wl-shortcuts-overlay project.

## FreeDesktop.org Standards

### XDG Specifications

While there isn't a formal FreeDesktop.org "shortcuts specification" URL at the path you mentioned, we follow several related standards:

1. **XDG Base Directory Specification**
   - We check `~/.config` for compositor configuration files
   - We respect `$XDG_CONFIG_HOME` if set
   - Lock file is placed in `$XDG_RUNTIME_DIR` or fallback to `/tmp`

2. **Desktop Entry Specification**
   - The legacy `shortcut_reader.rs` module demonstrates XDG desktop entry parsing
   - Located in standard paths: `~/.local/share/applications`, `/usr/share/applications`

### XKB (X Keyboard Extension) Standards

We use `xkbcommon` which is the standard for keyboard handling in Wayland compositors:

1. **Keysym Representation**
   - Uses `xkb::Keysym` for keyboard symbols
   - Follows xkbcommon naming conventions
   - Supports both case-sensitive and case-insensitive keysym lookup

2. **Modifier Keys**
   - Standard modifiers: Ctrl, Alt, Shift, Super (Logo)
   - Follows common conventions from X11 and Wayland compositors

## Design Patterns

### Inspired by COSMIC Settings Daemon

Our implementation follows patterns from [COSMIC Settings Daemon](https://github.com/pop-os/cosmic-settings-daemon):

```rust
// From cosmic-settings-daemon/config/src/shortcuts/binding.rs
pub struct Binding {
    pub modifiers: Modifiers,
    pub key: Option<xkb::Keysym>,
    pub description: Option<String>,
}
```

Our implementation:
```rust
// src/keybinding_reader.rs
pub struct KeyBinding {
    pub modifiers: Modifiers,
    pub key: Option<xkb::Keysym>,
    pub description: String,
    pub command: String,
}
```

### Compositor Config Formats

We parse shortcuts from multiple compositor configuration formats:

#### 1. Sway/i3 Format
```
bindsym $mod+Return exec $term
bindsym $mod+Shift+q kill
```

#### 2. Hyprland Format
```
bind = $mainMod, Q, exec, kitty
bind = SUPER, F, fullscreen
```

#### 3. River Format
```
riverctl map normal Super Return spawn foot
riverctl map normal Super Q close
```

#### 4. Wayfire Format
```ini
[command]
binding_terminal = <super> KEY_T
```

## Wayland Protocol Compliance

### Layer Shell Protocol

We use the `wlr-layer-shell` protocol from wlroots:

- **Layer**: Overlay (topmost)
- **Keyboard Interactivity**: Exclusive when visible
- **Anchor**: None (centered on screen)
- **Size**: Configurable (default 800x600)

### Smithay Client Toolkit

We follow the patterns from [smithay-client-toolkit examples](https://github.com/Smithay/client-toolkit/tree/master/examples):

- Proper delegate implementations for all protocols
- Correct event handling patterns
- Proper surface lifecycle management

## Architecture

### Module Structure

```
src/
├── main.rs                 - Entry point, initialization
├── singleton.rs            - Single instance enforcement
├── keybinding_reader.rs    - Parse compositor configs with xkbcommon
├── shortcut_reader.rs      - Legacy XDG desktop entry parser (kept for reference)
└── overlay.rs              - Wayland layer shell surface and rendering
```

### Key Abstractions

1. **Modifiers struct**: Represents keyboard modifiers (Ctrl, Alt, Shift, Super)
2. **KeyBinding struct**: Combines modifiers, keysym, description, and command
3. **ShortcutReader**: Discovers and parses compositor configuration files
4. **OverlayApp**: Manages Wayland surface and event loop

## Future Enhancements

### Potential Standards to Adopt

1. **DBus Interface**
   - Expose shortcuts via DBus
   - Allow external tools to query available shortcuts
   - Follow `org.freedesktop.*` naming conventions

2. **Portal Integration**
   - Use XDG Desktop Portal for better sandboxing support
   - Access compositor settings through portals

3. **Accessibility**
   - Add screen reader support
   - Ensure keyboard navigation
   - High contrast themes

### Additional Compositor Support

- **Wayfire**: Enhanced INI parsing
- **KWin**: KDE Plasma Wayland support
- **GNOME Shell**: Mutter/GNOME shortcuts (via GSettings)
- **Labwc**: Openbox-style config support

## Security Considerations

- **File Locking**: Prevents multiple instances using POSIX file locks
- **No Elevated Privileges**: Runs as regular user
- **Config Parsing**: Defensive parsing of user config files
- **No Remote Code Execution**: Commands are displayed, not executed

## Testing

- Unit tests for config parsing
- Unit tests for keysym conversion
- Integration tests with mock compositor configs
- No tests run actual Wayland connections (requires compositor)

## References

- [Wayland Protocol](https://wayland.freedesktop.org/docs/html/)
- [wlr-protocols](https://gitlab.freedesktop.org/wlroots/wlr-protocols)
- [xkbcommon Documentation](https://xkbcommon.org/doc/current/)
- [Smithay Client Toolkit](https://github.com/Smithay/client-toolkit)
- [COSMIC Settings Daemon](https://github.com/pop-os/cosmic-settings-daemon)
- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/latest/)
