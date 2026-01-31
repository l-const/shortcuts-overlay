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

    fn _set_config(&self, new_config: config::OverlayConfig) {
        let mut config = self._config.lock().unwrap();
        *config = new_config;
    }

    fn _set_keybindings(&self, new_keybindings: Vec<KeyBinding>) {
        let mut keybindings = self._keybindings.lock().unwrap();
        *keybindings = new_keybindings;
    }
}
