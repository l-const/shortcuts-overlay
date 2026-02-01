use crate::state::State;
use notify::{Event, RecursiveMode, Result, Watcher};
use std::sync::Arc;
use std::{path::Path, sync::mpsc};

const OVERLAY_CONFIG_FILE: &str = "/usr/share/shortcuts-overlay/overlay-config.toml";

pub(crate) fn watch_overlay_config(state: Arc<State>) -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();

    let mut watcher = notify::recommended_watcher(tx)?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(Path::new(OVERLAY_CONFIG_FILE), RecursiveMode::Recursive)?;
    // Block forever, printing out events as they come in
    for res in rx {
        match res {
            Ok(event) => {
                log::debug!("event: {:?}", event);
                if event.kind.is_modify() {
                    state.reload_overlay_config();
                }
            }
            Err(e) => log::error!("watch error: {:?}", e),
        }
    }

    Ok(())
}
