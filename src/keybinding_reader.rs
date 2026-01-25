use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use xkbcommon::xkb;

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

impl std::fmt::Display for Modifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct KeyBinding {
    pub modifiers: Modifiers,
    pub key: Option<xkb::Keysym>,
    pub description: String,
    pub command: String,
}

impl std::fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        let mod_str = self.modifiers.to_string();
        if !mod_str.is_empty() {
            parts.push(mod_str);
        }
        
        if let Some(keysym) = self.key {
            let key_name = xkb::keysym_get_name(keysym);
            // Clean up the key name
            let key_name = key_name.strip_prefix("KEY_").unwrap_or(&key_name);
            parts.push(key_name.to_string());
        }
        
        write!(f, "{}", parts.join(" + "))
    }
}

pub struct ShortcutReader {
    bindings: Vec<KeyBinding>,
}

impl ShortcutReader {
    pub fn new() -> Self {
        ShortcutReader {
            bindings: Vec::new(),
        }
    }

    pub fn load_shortcuts(&mut self) -> Result<()> {
        // Try to detect the compositor and load its config
        let config_paths = self.get_config_paths();
        
        for path in config_paths {
            if path.exists() {
                log::info!("Reading shortcuts from: {:?}", path);
                self.parse_config_file(&path)?;
            }
        }

        // If no shortcuts found, add some common defaults
        if self.bindings.is_empty() {
            log::info!("No config found, using common default shortcuts");
            self.add_common_defaults();
        }

        log::info!("Loaded {} keyboard shortcuts", self.bindings.len());
        Ok(())
    }

    pub fn get_bindings(&self) -> &[KeyBinding] {
        &self.bindings
    }

    fn get_config_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        
        if let Some(home) = dirs::home_dir() {
            let config_dir = home.join(".config");
            
            // Sway config
            paths.push(config_dir.join("sway/config"));
            
            // Hyprland config
            paths.push(config_dir.join("hyprland/hyprland.conf"));
            
            // i3 config (for X11, but some people use it)
            paths.push(config_dir.join("i3/config"));
            
            // River config
            paths.push(config_dir.join("river/init"));
            
            // Wayfire config
            paths.push(config_dir.join("wayfire.ini"));
        }
        
        paths
    }

    fn parse_config_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {:?}", path))?;

        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        match filename {
            "config" => {
                // Sway or i3 config format
                self.parse_sway_config(&content);
            }
            "hyprland.conf" => {
                self.parse_hyprland_config(&content);
            }
            "wayfire.ini" => {
                self.parse_wayfire_config(&content);
            }
            "init" => {
                self.parse_river_config(&content);
            }
            _ => {
                // Try sway format as fallback
                self.parse_sway_config(&content);
            }
        }

        Ok(())
    }

    fn parse_sway_config(&mut self, content: &str) {
        for line in content.lines() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse bindsym lines: bindsym $mod+Return exec $term
            if line.starts_with("bindsym ") {
                if let Some(rest) = line.strip_prefix("bindsym ") {
                    if let Some((keys, command)) = rest.split_once(' ') {
                        let keys = keys.trim();
                        let command = command.trim();
                        
                        if let Some(binding) = self.parse_binding_string(keys, command) {
                            self.bindings.push(binding);
                        }
                    }
                }
            }
        }
    }

    fn parse_hyprland_config(&mut self, content: &str) {
        for line in content.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse bind lines: bind = $mainMod, Q, exec, kitty
            if line.starts_with("bind ") {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 3 {
                    // Extract modifiers and key from first part
                    if let Some(mods_part) = parts[0].strip_prefix("bind = ") {
                        let mods_part = mods_part.trim();
                        let key = parts[1].trim();
                        let action = parts[2].trim();
                        let command = parts.get(3).map(|s| s.trim()).unwrap_or("");
                        
                        let keys = format!("{}+{}", mods_part, key);
                        let full_command = if !command.is_empty() {
                            format!("{} {}", action, command)
                        } else {
                            action.to_string()
                        };
                        
                        if let Some(binding) = self.parse_binding_string(&keys, &full_command) {
                            self.bindings.push(binding);
                        }
                    }
                }
            }
        }
    }

    fn parse_wayfire_config(&mut self, content: &str) {
        // Wayfire uses INI format with [command] sections
        let mut current_section = String::new();
        
        for line in content.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len()-1].to_string();
            } else if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                
                if key.starts_with("binding_") {
                    if let Some(binding) = self.parse_binding_string(value, &current_section) {
                        self.bindings.push(binding);
                    }
                }
            }
        }
    }

    fn parse_river_config(&mut self, content: &str) {
        for line in content.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse riverctl map lines: riverctl map normal Super Return spawn foot
            if line.contains("riverctl map") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 6 {
                    let modifier = parts.get(3).unwrap_or(&"");
                    let key = parts.get(4).unwrap_or(&"");
                    let command = parts[5..].join(" ");
                    
                    let keys = format!("{}+{}", modifier, key);
                    if let Some(binding) = self.parse_binding_string(&keys, &command) {
                        self.bindings.push(binding);
                    }
                }
            }
        }
    }

    fn parse_binding_string(&self, keys_str: &str, command: &str) -> Option<KeyBinding> {
        let mut modifiers = Modifiers::new();
        let mut key: Option<xkb::Keysym> = None;

        for token in keys_str.split('+') {
            let token = token.trim().replace("$mod", "Super").replace("$mainMod", "Super");
            match token.to_ascii_lowercase().as_str() {
                "super" | "mod4" => modifiers.logo = true,
                "ctrl" | "control" => modifiers.ctrl = true,
                "alt" | "mod1" => modifiers.alt = true,
                "shift" => modifiers.shift = true,
                lowercased => {
                    // Try to convert to keysym
                    // First try as single character
                    if lowercased.chars().count() == 1 {
                        if let Some(ch) = lowercased.chars().next() {
                            key = Some(xkb::Keysym::from_char(ch));
                        }
                    } else {
                        // Try as keysym name
                        let keysym = xkb::keysym_from_name(&token, xkb::KEYSYM_NO_FLAGS);
                        if keysym.raw() != 0 {
                            key = Some(keysym);
                        } else {
                            // Try case-insensitive
                            let keysym = xkb::keysym_from_name(&token, xkb::KEYSYM_CASE_INSENSITIVE);
                            if keysym.raw() != 0 {
                                key = Some(keysym);
                            }
                        }
                    }
                }
            }
        }

        let description = self.extract_description(command);
        
        Some(KeyBinding {
            modifiers,
            key,
            description,
            command: command.to_string(),
        })
    }

    fn extract_description(&self, command: &str) -> String {
        // Try to extract meaningful description from command
        if command.starts_with("exec ") {
            let cmd = command.strip_prefix("exec ").unwrap_or(command);
            // Get the program name (first word)
            let prog = cmd.split_whitespace().next().unwrap_or(cmd);
            // Remove path if present
            let prog = prog.split('/').next_back().unwrap_or(prog);
            format!("Launch {}", prog)
        } else if command.contains("kill") || command.contains("close") {
            "Close window".to_string()
        } else if command.contains("focus") {
            "Focus window".to_string()
        } else if command.contains("move") {
            "Move window".to_string()
        } else if command.contains("split") {
            "Split container".to_string()
        } else if command.contains("layout") {
            "Change layout".to_string()
        } else if command.contains("fullscreen") {
            "Toggle fullscreen".to_string()
        } else if command.contains("floating") {
            "Toggle floating".to_string()
        } else if command.contains("workspace") {
            "Switch workspace".to_string()
        } else if command.contains("reload") {
            "Reload config".to_string()
        } else if command.contains("exit") {
            "Exit compositor".to_string()
        } else {
            // Return first few words of command
            command.split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    fn add_common_defaults(&mut self) {
        // Add common default shortcuts with proper keysyms
        let defaults = vec![
            ("Super+Return", "Launch terminal", "exec $terminal"),
            ("Super+d", "Launch application launcher", "exec $menu"),
            ("Super+Shift+q", "Close window", "kill"),
            ("Super+Shift+e", "Exit compositor", "exit"),
            ("Super+Shift+c", "Reload config", "reload"),
            ("Super+f", "Toggle fullscreen", "fullscreen"),
            ("Super+space", "Toggle floating", "floating toggle"),
            ("Super+1", "Switch to workspace 1", "workspace 1"),
            ("Super+2", "Switch to workspace 2", "workspace 2"),
            ("Super+3", "Switch to workspace 3", "workspace 3"),
            ("Super+4", "Switch to workspace 4", "workspace 4"),
            ("Super+h", "Focus left", "focus left"),
            ("Super+j", "Focus down", "focus down"),
            ("Super+k", "Focus up", "focus up"),
            ("Super+l", "Focus right", "focus right"),
            ("Alt+Tab", "Cycle windows", "focus next"),
        ];

        for (keys, desc, cmd) in defaults {
            if let Some(binding) = self.parse_binding_string(keys, cmd) {
                let mut binding = binding;
                binding.description = desc.to_string();
                self.bindings.push(binding);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_binding_string() {
        let reader = ShortcutReader::new();
        let binding = reader.parse_binding_string("Super+Return", "exec kitty").unwrap();
        assert!(binding.modifiers.logo);
        assert!(binding.key.is_some());
    }

    #[test]
    fn test_extract_description() {
        let reader = ShortcutReader::new();
        assert_eq!(reader.extract_description("exec kitty"), "Launch kitty");
        assert_eq!(reader.extract_description("kill"), "Close window");
    }
}
