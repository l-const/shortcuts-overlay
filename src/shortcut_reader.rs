use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: String,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub comment: Option<String>,
    pub categories: Vec<String>,
}

#[allow(dead_code)]
pub struct ShortcutReader {
    entries: Vec<DesktopEntry>,
}

#[allow(dead_code)]
impl ShortcutReader {
    pub fn new() -> Self {
        ShortcutReader {
            entries: Vec::new(),
        }
    }

    pub fn load_shortcuts(&mut self) -> Result<()> {
        let data_dirs = Self::get_xdg_data_dirs();
        
        for dir in data_dirs {
            let applications_dir = dir.join("applications");
            if applications_dir.exists() {
                self.scan_directory(&applications_dir)?;
            }
        }

        log::info!("Loaded {} desktop entries", self.entries.len());
        Ok(())
    }

    pub fn get_entries(&self) -> &[DesktopEntry] {
        &self.entries
    }

    fn get_xdg_data_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // User's local applications directory
        if let Some(home_dir) = dirs::home_dir() {
            dirs.push(home_dir.join(".local/share"));
        }

        // System directories
        let xdg_data_dirs = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());

        for dir in xdg_data_dirs.split(':') {
            dirs.push(PathBuf::from(dir));
        }

        dirs
    }

    fn scan_directory(&mut self, dir: &Path) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {:?}", dir))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                if let Ok(entry) = Self::parse_desktop_file(&path) {
                    self.entries.push(entry);
                }
            }
        }

        Ok(())
    }

    fn parse_desktop_file(path: &Path) -> Result<DesktopEntry> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {:?}", path))?;

        let mut properties: HashMap<String, String> = HashMap::new();
        let mut in_desktop_entry = false;

        for line in content.lines() {
            let line = line.trim();
            
            if line == "[Desktop Entry]" {
                in_desktop_entry = true;
                continue;
            }
            
            if line.starts_with('[') {
                in_desktop_entry = false;
                continue;
            }

            if !in_desktop_entry || line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                properties.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        let name = properties
            .get("Name")
            .context("Desktop entry missing Name field")?
            .clone();

        let categories = properties
            .get("Categories")
            .map(|c| c.split(';').filter(|s| !s.is_empty()).map(String::from).collect())
            .unwrap_or_default();

        Ok(DesktopEntry {
            name,
            exec: properties.get("Exec").cloned(),
            icon: properties.get("Icon").cloned(),
            comment: properties.get("Comment").cloned(),
            categories,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_xdg_data_dirs() {
        let dirs = ShortcutReader::get_xdg_data_dirs();
        assert!(!dirs.is_empty(), "Should have at least one data directory");
    }
}
