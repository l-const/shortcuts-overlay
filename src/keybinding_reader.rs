use anyhow::{Context, Result};
use cosmic_settings_config::shortcuts as cs;
use std::fmt;
use xkbcommon::xkb;

//
// This reader exclusively loads shortcuts from the Cosmic Settings config
// (com.system76.CosmicSettings.Shortcuts) using the `cosmic-settings-config`
//
// Behavior:
// - Loads the combined (system + user) shortcuts via the helper exposed by
//   that crate and converts each `(Binding, Action)` entry into this
//   repository's `KeyBinding` structure.
//

#[derive(Debug, Clone)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool, // Super/Win key
}

impl Modifiers {
    pub fn new() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            logo: false,
        }
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.logo {
            parts.push("Super");
        }
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        write!(f, "{}", parts.join(" + "))
    }
}

/// Representation used by the overlay renderer
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub modifiers: Modifiers,
    pub key: Option<xkb::Keysym>,
    pub description: String,
    /// Best-effort textual representation of the underlying action/command.
    pub command: String,
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        let mod_str = self.modifiers.to_string();
        if !mod_str.is_empty() {
            parts.push(mod_str);
        }

        if let Some(keysym) = self.key {
            let key_name = xkb::keysym_get_name(keysym);
            // Clean up the key name if it follows KEY_ prefix convention
            let key_name = key_name.strip_prefix("KEY_").unwrap_or(&key_name);
            parts.push(key_name.to_string());
        }

        write!(f, "{}", parts.join(" + "))
    }
}

/// Primary loader: reads cosmic shortcuts and converts them into KeyBinding list.
///
/// Errors if the cosmic settings context cannot be opened or the shortcuts
/// helper cannot be executed. The returned Vec may be empty if no shortcuts
/// are configured.
pub fn load_cosmic_shortcuts() -> Result<Vec<KeyBinding>> {
    // Try to obtain a cosmic-config context for the Pop!_OS shortcuts schema.
    // The `cosmic-settings-config` crate exposes a small API in its shortcuts module:
    // - `context()` -> Result<cosmic_config::Config, _>
    // - `shortcuts(&cosmic_config::Config) -> Shortcuts`
    //
    // We call those here and convert their Shortcuts map into our KeyBinding list.
    let ctx = cs::context().context("failed to open cosmic settings config context")?;

    // This returns the merged system + user shortcuts
    let cs_shortcuts = cs::shortcuts(&ctx);

    // `cs_shortcuts` is defined in the upstream crate as:
    //   pub struct Shortcuts(pub HashMap<Binding, Action>);
    // where `Binding` and `Action` are re-exported types from that crate.
    //
    // We'll iterate over the entries and convert each `Binding` -> `KeyBinding`.
    let mut out: Vec<KeyBinding> = Vec::new();

    // Iterate by value over the merged shortcuts map (Binding, Action)
    for (binding, action) in cs_shortcuts.0.into_iter() {
        // Map modifiers
        let mut m = Modifiers::new();
        // The upstream `Modifiers` type uses similarly-named boolean fields.
        m.ctrl = binding.modifiers.ctrl;
        m.alt = binding.modifiers.alt;
        m.shift = binding.modifiers.shift;
        m.logo = binding.modifiers.logo;

        // Prefer `binding.key` (xkb::Keysym) if present. If absent but keycode exists,
        // we don't try to map keycode -> keysym here.
        let keysym: Option<xkb::Keysym> = binding.key;

        // If this binding is explicitly disabled in user/system config, skip it.
        if let cs::Action::Disable = action {
            continue;
        }

        // Description: prefer the binding description if present; otherwise synthesize
        // a human-friendly label from the Action variant where possible.
        let description = if let Some(desc) = &binding.description {
            desc.clone()
        } else {
            match &action {
                cs::Action::Close => "Close window".to_string(),
                cs::Action::Debug => "Debug overlay".to_string(),
                cs::Action::Focus(dir) => format!("Focus {:?}", dir),
                cs::Action::LastWorkspace => "Switch to last workspace".to_string(),
                cs::Action::Maximize => "Maximize window".to_string(),
                cs::Action::Fullscreen => "Toggle fullscreen".to_string(),
                cs::Action::Minimize => "Minimize window".to_string(),
                cs::Action::Move(dir) => format!("Move {:?}", dir),
                cs::Action::MoveToWorkspace(n) => format!("Move to workspace {}", n),
                cs::Action::NextWorkspace => "Next workspace".to_string(),
                cs::Action::PreviousWorkspace => "Previous workspace".to_string(),
                cs::Action::Resizing(rd) => format!("Resize {:?}", rd),
                cs::Action::SwapWindow => "Swap window".to_string(),
                cs::Action::SendToWorkspace(n) => format!("Send to workspace {}", n),
                cs::Action::SwitchOutput(dir) => format!("Switch output {:?}", dir),
                cs::Action::System(s) => format!("System action: {:?}", s),
                cs::Action::Spawn(cmd) => {
                    // Use the spawn string as a short description
                    if cmd.len() > 0 {
                        format!("Spawn {}", cmd)
                    } else {
                        "Spawn command".to_string()
                    }
                }
                cs::Action::Terminate => "Terminate compositor".to_string(),
                cs::Action::ToggleOrientation => "Toggle orientation".to_string(),
                cs::Action::ToggleStacking => "Toggle stacking".to_string(),
                cs::Action::ToggleSticky => "Toggle sticky".to_string(),
                cs::Action::ToggleTiling => "Toggle tiling".to_string(),
                cs::Action::ToggleWindowFloating => "Toggle floating".to_string(),
                cs::Action::Workspace(n) => format!("Go to workspace {}", n),
                cs::Action::ZoomIn => "Zoom in".to_string(),
                cs::Action::ZoomOut => "Zoom out".to_string(),
                // Fallback for variants not explicitly handled above:
                _ => format!("{:?}", action),
            }
        };

        // Command: extract a useful command string where possible (e.g., Spawn),
        // otherwise fall back to a debug representation for display/logging.
        let command = match &action {
            cs::Action::Spawn(cmd) => cmd.clone(),
            cs::Action::System(s) => format!("{:?}", s),
            // We already skipped Disable above, so map other variants to debug strings.
            _ => format!("{:?}", action),
        };

        out.push(KeyBinding {
            modifiers: m,
            key: keysym,
            description,
            command,
        });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_format_modifiers_and_keybinding() {
        let mut m = Modifiers::new();
        m.ctrl = true;
        m.logo = true;
        let kb = KeyBinding {
            modifiers: m,
            key: Some(xkb::keysym_from_name("Return", xkb::KEYSYM_NO_FLAGS)),
            description: "Test".to_string(),
            command: "exec test".to_string(),
        };
        let s = format!("{}", kb);
        assert!(s.contains("Ctrl") && s.contains("Super"));
    }
}
