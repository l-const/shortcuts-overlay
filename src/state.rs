use crate::config;
use crate::keybinding_reader::KeyBinding;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) struct State {
    _config: Arc<Mutex<config::OverlayConfig>>,
    _keybindings: Arc<Mutex<Vec<KeyBinding>>>,
}

impl State {
    pub(crate) fn new(
        config: Arc<Mutex<config::OverlayConfig>>,
        keybindings: Arc<Mutex<Vec<KeyBinding>>>,
    ) -> Self {
        State {
            _config: config,
            _keybindings: keybindings,
        }
    }

    pub(crate) fn _set_config(&self, new_config: config::OverlayConfig) {
        let mut config = self._config.lock().unwrap();
        *config = new_config;
    }

    pub(crate) fn reload_overlay_config(&self) {
        let mut config = self._config.lock().unwrap();
        *config = config::OverlayConfig::load().unwrap();
    }

    pub(crate) fn _set_keybindings(&self, new_keybindings: Vec<KeyBinding>) {
        let mut keybindings = self._keybindings.lock().unwrap();
        *keybindings = new_keybindings;
    }

    pub(crate) fn clone_config(&self) -> config::OverlayConfig {
        let config = self._config.lock().unwrap();
        config.clone()
    }

    pub(crate) fn clone_keybindings(&self) -> Vec<KeyBinding> {
        let keybindings = self._keybindings.lock().unwrap();
        keybindings.clone()
    }
}
