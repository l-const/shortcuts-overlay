use anyhow::{Context, Result};
use cosmic_settings_config::shortcuts as cs;
use cosmic_settings_config::shortcuts::action::System as SystemAction;
use cosmic_settings_config::shortcuts::action::{
    Direction, FocusDirection, Orientation, ResizeDirection,
};

use cosmic_settings_config::shortcuts::Action;
use cosmic_settings_config::Binding;
use std::collections::HashSet;
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

/// Reference:
/// https://github.com/pop-os/cosmic-settings/blob/eec172cdae62cf8b937346113521e5c5a5677580/cosmic-settings/src/pages/input/keyboard/shortcuts/mod.rs#L629

pub fn localize_action(action: &Action) -> &str {
    #[allow(deprecated)]
    match action {
        Action::Close => "Close window",
        Action::Disable => "Disable",
        Action::Focus(FocusDirection::Down) => "Focus down",
        Action::Focus(FocusDirection::In) => "Focus in",
        Action::Focus(FocusDirection::Left) => "Focus left",
        Action::Focus(FocusDirection::Out) => "Focus out",
        Action::Focus(FocusDirection::Right) => "Focus right",
        Action::Focus(FocusDirection::Up) => "Focus up",
        Action::Workspace(i) => "Workspace n",
        Action::LastWorkspace => "Last workspace",
        Action::Maximize => "Maximize window",
        Action::Fullscreen => "Fullscreen window",
        Action::Minimize => "Minimize window",
        Action::Move(Direction::Down) => "Move window down",
        Action::Move(Direction::Right) => "Move window right",
        Action::Move(Direction::Left) => "Move window left",
        Action::Move(Direction::Up) => "Move window up",
        Action::MoveToLastWorkspace | Action::SendToLastWorkspace => "Move window to last wrkspace",
        Action::MoveToNextOutput | Action::SendToNextOutput => "Move window to next display",
        Action::MoveToNextWorkspace | Action::SendToNextWorkspace => "Move window to next wrkspace",
        Action::MoveToPreviousWorkspace | Action::SendToPreviousWorkspace => {
            "Move window to prev wrkspace"
        }
        Action::MoveToOutput(Direction::Down) | Action::SendToOutput(Direction::Down) => {
            "Move window one monitor down"
        }
        Action::MoveToOutput(Direction::Left) | Action::SendToOutput(Direction::Left) => {
            "Move window one monitor left"
        }
        Action::MoveToOutput(Direction::Right) | Action::SendToOutput(Direction::Right) => {
            "Move window one monitor right"
        }
        Action::MoveToOutput(Direction::Up) | Action::SendToOutput(Direction::Up) => {
            "Move window one monitor up"
        }
        Action::MoveToPreviousOutput | Action::SendToPreviousOutput => {
            "Move window to prev display"
        }
        Action::MoveToWorkspace(i) | Action::SendToWorkspace(i) => "Move window to workspace n",
        Action::NextOutput => "Focus next output",
        Action::NextWorkspace => "Focus next workspace",
        Action::Orientation(Orientation::Horizontal) => "Set horizontal orientation",
        Action::Orientation(Orientation::Vertical) => "Set vertical orientation",
        Action::PreviousOutput => "Focus previous output",
        Action::PreviousWorkspace => "Focus previous workspace",
        Action::Resizing(ResizeDirection::Inwards) => "Resize window inwards",
        Action::Resizing(ResizeDirection::Outwards) => "Resize window outwards",
        Action::SwapWindow => "Swap window",
        Action::SwitchOutput(Direction::Down) => "Switch to output down",
        Action::SwitchOutput(Direction::Left) => "Switch to output left",
        Action::SwitchOutput(Direction::Right) => "Switch to output right",
        Action::SwitchOutput(Direction::Up) => "Switch to output up",
        Action::ToggleOrientation => "Toggle orientation",
        Action::ToggleStacking => "Toggle window stacking",
        Action::ToggleSticky => "Toggle sticky window",
        Action::ToggleTiling => "Toggle window tiling",
        Action::ToggleWindowFloating => "Toggle window floating",

        // Currently unused by any settings pages
        Action::Debug => "Debug",

        Action::MigrateWorkspaceToNextOutput => "Migrate workspace to next output",
        Action::MigrateWorkspaceToOutput(Direction::Down) => "Migrate workspace down",
        Action::MigrateWorkspaceToOutput(Direction::Left) => "Migrate workspace left",
        Action::MigrateWorkspaceToOutput(Direction::Right) => "Migrate workspace right",
        Action::MigrateWorkspaceToOutput(Direction::Up) => "Migrate workspace up",
        Action::MigrateWorkspaceToPreviousOutput => "Migrate workspace to previous output",

        Action::Terminate => "Terminate",

        Action::System(system) => match system {
            SystemAction::AppLibrary => "Open the app library",
            SystemAction::BrightnessDown => "Decrease display brightness",
            SystemAction::BrightnessUp => "Increase display brightness",
            SystemAction::InputSourceSwitch => "Switch input source",
            SystemAction::HomeFolder => "Open home folder",
            SystemAction::KeyboardBrightnessDown => "Decrease keyboard brightness",
            SystemAction::KeyboardBrightnessUp => "Increase keyboard brightness",
            SystemAction::Launcher => "Open the Launcher",
            SystemAction::LogOut => "Log Out",
            SystemAction::LockScreen => "Lock the screen",
            SystemAction::Mute => "Mute audio output",
            SystemAction::MuteMic => "Mutes microphone input",
            SystemAction::PlayPause => "Play/pause",
            SystemAction::PlayNext => "Next track",
            SystemAction::PlayPrev => "Previous track",
            SystemAction::PowerOff => "Power off",
            SystemAction::Screenshot => "Take a screenshot",
            SystemAction::Suspend => "Suspend",
            SystemAction::ScreenReader => "Toggle screen reader",
            SystemAction::Terminal => "Open a terminal",
            SystemAction::TouchpadToggle => "Toggle touchpad",
            SystemAction::VolumeLower => "Decrease audio output volume",
            SystemAction::VolumeRaise => "Increase audio output volume",
            SystemAction::WebBrowser => "Open a web browser",
            SystemAction::WindowSwitcher => "Switch between open windows",
            SystemAction::WindowSwitcherPrevious => "Switch between open windows reversed",
            SystemAction::WorkspaceOverview => "Open the workspace overview",
            SystemAction::DisplayToggle => "Toggle internal display",
        },

        Action::ZoomIn => "Zoom in",

        Action::ZoomOut => "Zoom out",

        Action::Spawn(task) => task,
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

    // This returns the user shortcuts only

    log::info!(
        "number of user_shortcuts: {:?}",
        cs_shortcuts.0.iter().len()
    );

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
            localize_action(&action).to_string()
        };

        log::trace!(
            "binding: {:?}, action: {:?}, description: {}",
            binding,
            action,
            description
        );

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

    // Deduplicate by description - keep only the first occurrence of each unique description
    let mut seen_descriptions = HashSet::new();
    out.retain(|binding| seen_descriptions.insert(binding.description.clone()));

    log::info!(
        "Loaded {} unique shortcuts (after deduplication)",
        out.len()
    );

    for binding in &out {
        log::trace!(
            "binding: {:?}, action: {:?}, description: {}",
            binding,
            binding.command,
            binding.description
        );
    }

    // remove the ones whose binding starts with XF86
    out.retain(|binding| {
        !binding
            .key
            .is_some_and(|x| x.name().is_some_and(|x| x.starts_with("XF86")))
    });

    log::info!(
        "number of shortcuts after XF86 removal: {}",
        out.iter().len()
    );

    for binding in &out {
        log::info!(
            "binding: {:?}, action: {:?}, description: {}",
            binding,
            binding.command,
            binding.description
        );
    }

    // sort by the description
    out.sort_by(|a, b| a.description.cmp(&b.description));

    Ok(out)
}
