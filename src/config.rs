use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Configuration for the shortcuts overlay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    /// Background color in hex format (e.g., "#000000")
    #[serde(default = "default_background_color")]
    pub background_color: String,

    /// Text color in hex format (e.g., "#ffffff")
    #[serde(default = "default_text_color")]
    pub text_color: String,

    /// Font size in pixels
    #[serde(default = "default_font_size")]
    pub font_size: f32,

    /// Line height in pixels
    #[serde(default = "default_line_height")]
    pub line_height: f32,

    /// Whether to apply blur effect to the background
    #[serde(default = "default_apply_blur", rename = "apply-blur")]
    pub apply_blur: bool,

    /// Anchor point for the overlay
    #[serde(default = "default_anchor")]
    pub anchor: String,

    /// Overlay width in pixels
    #[serde(default = "default_width")]
    pub width: u32,

    /// Overlay height in pixels
    #[serde(default = "default_height")]
    pub height: u32,

    /// Corner radius in pixels for rounded corners (optional, 0 = square corners)
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f32,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            background_color: default_background_color(),
            text_color: default_text_color(),
            font_size: default_font_size(),
            line_height: default_line_height(),
            apply_blur: default_apply_blur(),
            anchor: default_anchor(),
            width: default_width(),
            height: default_height(),
            corner_radius: default_corner_radius(),
        }
    }
}

// Default value functions
fn default_background_color() -> String {
    "#000000".to_string()
}

fn default_text_color() -> String {
    "#ffffff".to_string()
}

fn default_font_size() -> f32 {
    12.0
}

fn default_line_height() -> f32 {
    1.5
}

fn default_apply_blur() -> bool {
    true
}

fn default_anchor() -> String {
    "center".to_string()
}

fn default_width() -> u32 {
    1200
}

fn default_height() -> u32 {
    800
}

fn default_corner_radius() -> f32 {
    16.0
}

impl OverlayConfig {
    /// Load configuration from a TOML file
    pub fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: OverlayConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Load configuration from default location
    /// Searches: /usr/share/shortcuts-overlay/overlay-config.toml
    ///
    /// If no config file is found, returns default configuration
    pub fn load() -> Result<Self> {
        let etc_config = PathBuf::from("/usr/share")
            .join("shortcuts-overlay")
            .join("overlay-config.toml");
        if etc_config.exists() {
            log::info!("Loading config from: {}", etc_config.display());
            return Self::load_from_file(&etc_config);
        }

        // No config found, use defaults
        log::info!(
            "No config file found at /usr/share/shortcuts-overlay/overlay-config.toml, using defaults"
        );
        Ok(Self::default())
    }

    /// Parse hex color string to RGB values
    pub fn parse_hex_color(hex: &str) -> Result<(u8, u8, u8)> {
        let hex = hex.trim_start_matches('#');

        if hex.len() != 6 {
            anyhow::bail!("Invalid hex color format. Expected #RRGGBB, got: #{}", hex);
        }

        let r = u8::from_str_radix(&hex[0..2], 16)
            .with_context(|| format!("Failed to parse red component from: {}", hex))?;
        let g = u8::from_str_radix(&hex[2..4], 16)
            .with_context(|| format!("Failed to parse green component from: {}", hex))?;
        let b = u8::from_str_radix(&hex[4..6], 16)
            .with_context(|| format!("Failed to parse blue component from: {}", hex))?;

        Ok((r, g, b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(
            OverlayConfig::parse_hex_color("#000000").unwrap(),
            (0, 0, 0)
        );
        assert_eq!(
            OverlayConfig::parse_hex_color("#ffffff").unwrap(),
            (255, 255, 255)
        );
        assert_eq!(
            OverlayConfig::parse_hex_color("#ff0000").unwrap(),
            (255, 0, 0)
        );
        assert_eq!(
            OverlayConfig::parse_hex_color("#00ff00").unwrap(),
            (0, 255, 0)
        );
        assert_eq!(
            OverlayConfig::parse_hex_color("#0000ff").unwrap(),
            (0, 0, 255)
        );
        assert_eq!(
            OverlayConfig::parse_hex_color("32373c").unwrap(),
            (50, 55, 60)
        );
    }

    #[test]
    fn test_default_config() {
        let config = OverlayConfig::default();
        assert_eq!(config.background_color, "#000000");
        assert_eq!(config.text_color, "#ffffff");
        assert_eq!(config.font_size, 12.0);
        assert_eq!(config.apply_blur, true);
        assert_eq!(config.anchor, "center");
        assert_eq!(config.width, 1200);
        assert_eq!(config.height, 800);
    }

    #[test]
    fn test_parse_toml() {
        let toml_str = r##"
background_color = "#000000"
text_color = "#ffffff"
font_size = 12.0
apply-blur = true
anchor = "center"
width = 1200
height = 800
"##;

        let config: OverlayConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.background_color, "#000000");
        assert_eq!(config.text_color, "#ffffff");
        assert_eq!(config.font_size, 12.0);
        assert_eq!(config.apply_blur, true);
        assert_eq!(config.anchor, "center");
        assert_eq!(config.corner_radius, 16.0);
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml_str = r##"
font_size = 16.0
"##;

        let config: OverlayConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.background_color, "#000000"); // default
        assert_eq!(config.text_color, "#ffffff"); // default
        assert_eq!(config.font_size, 16.0); // custom
        assert_eq!(config.apply_blur, true); // default
        assert_eq!(config.anchor, "center"); // default
        assert_eq!(config.width, 1200); // default
        assert_eq!(config.height, 800); // default
        assert_eq!(config.corner_radius, 16.0); // default
    }

    #[test]
    fn test_corner_radius_omitted() {
        // Test that omitted corner_radius uses default
        let toml_str = r##"
width = 800
"##;

        let config: OverlayConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.corner_radius, 16.0); // should use default
    }

    #[test]
    fn test_corner_radius_zero() {
        // Test that corner_radius = 0 disables rounded corners
        let toml_str = r##"
corner_radius = 0.0
"##;

        let config: OverlayConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.corner_radius, 0.0);
    }

    #[test]
    fn test_corner_radius_custom() {
        // Test custom corner_radius value
        let toml_str = r##"
corner_radius = 24.0
"##;

        let config: OverlayConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.corner_radius, 24.0);
    }
}
