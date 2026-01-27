use anyhow::{Context, Result};
use nix::fcntl::{flock, FlockArg};
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

pub struct SingletonGuard {
    _file: File,
}

impl SingletonGuard {
    pub fn acquire() -> Result<Self> {
        let lock_path = Self::get_lock_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create lock directory")?;
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .context("Failed to open lock file")?;

        // Try to acquire exclusive lock (non-blocking)
        flock(file.as_raw_fd(), FlockArg::LockExclusiveNonblock)
            .context("Another instance is already running")?;

        log::info!("Acquired singleton lock at {:?}", lock_path);

        Ok(SingletonGuard { _file: file })
    }

    fn get_lock_path() -> Result<PathBuf> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .or_else(|_| std::env::var("TMPDIR"))
            .unwrap_or_else(|_| "/tmp".to_string());

        Ok(PathBuf::from(runtime_dir).join("wl-shortcuts-overlay.lock"))
    }
}
