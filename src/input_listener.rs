use anyhow::Result;
use input::event::keyboard::{KeyState, KeyboardEventTrait};
use input::event::{Event, KeyboardEvent};
use input::{Libinput, LibinputInterface};
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::OwnedFd;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

/// Represents the state of the Alt modifier key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AltState {
    Pressed,
    Released,
}

/// Interface implementation for libinput
struct Interface;

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        OpenOptions::new()
            .custom_flags(flags)
            .read(true)
            .write((flags & libc::O_WRONLY != 0) || (flags & libc::O_RDWR != 0))
            .open(path)
            .map(|file| file.into())
            .map_err(|err| err.raw_os_error().unwrap_or(-1))
    }

    fn close_restricted(&mut self, fd: OwnedFd) {
        drop(fd);
    }
}

/// Starts listening for Alt key events from all available keyboard devices using libinput.
/// Returns a receiver that will get AltState updates when Alt is pressed or released.
pub fn start_alt_listener() -> Result<Receiver<AltState>> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        if let Err(e) = run_input_listener(tx) {
            log::error!("Input listener error: {}", e);
        }
    });

    Ok(rx)
}

fn run_input_listener(tx: Sender<AltState>) -> Result<()> {
    println!("Input listener thread started");
    log::info!("Input listener thread started");
    println!("Initializing libinput with udev backend...");
    log::info!("Initializing libinput with udev backend...");

    // Initialize libinput with udev backend
    let mut input = Libinput::new_with_udev(Interface);

    // Assign seat - this will discover all input devices
    if let Err(_) = input.udev_assign_seat("seat0") {
        log::error!(
            "Make sure you have permission to access /dev/input and are in the 'input' group"
        );
        log::error!("Run: sudo usermod -a -G input $USER");
        log::error!("Then log out and log back in");
        anyhow::bail!("Failed to assign seat to libinput");
    }

    log::info!("Successfully initialized libinput on seat0");

    // Track Alt state (either left or right Alt being pressed)
    let mut left_alt_pressed = false;
    let mut right_alt_pressed = false;

    log::info!("Starting event loop...");

    loop {
        // Dispatch events - this reads from the file descriptors
        match input.dispatch() {
            Ok(_) => {}
            Err(e) => {
                log::error!("Error dispatching libinput events: {}", e);
                thread::sleep(Duration::from_millis(100));
                continue;
            }
        }

        // Process all available events
        for event in &mut input {
            // We only care about keyboard events
            if let Event::Keyboard(keyboard_event) = event {
                process_keyboard_event(
                    keyboard_event,
                    &mut left_alt_pressed,
                    &mut right_alt_pressed,
                    &tx,
                )?;
            }
        }

        // Small sleep to avoid busy-waiting
        thread::sleep(Duration::from_millis(10));
    }
}

fn process_keyboard_event(
    event: KeyboardEvent,
    left_alt_pressed: &mut bool,
    right_alt_pressed: &mut bool,
    tx: &Sender<AltState>,
) -> Result<()> {
    let key = event.key();
    let state = event.key_state();

    // KEY_LEFTALT = 56, KEY_RIGHTALT = 100 (Linux input event codes)
    let is_left_alt = key == 56;
    let is_right_alt = key == 100;

    if !is_left_alt && !is_right_alt {
        return Ok(());
    }

    let pressed = matches!(state, KeyState::Pressed);
    let released = matches!(state, KeyState::Released);

    // Update state tracking
    if is_left_alt {
        *left_alt_pressed = pressed;
    } else if is_right_alt {
        *right_alt_pressed = pressed;
    }

    let any_alt_pressed = *left_alt_pressed || *right_alt_pressed;

    // Send state update
    if pressed {
        if tx.send(AltState::Pressed).is_err() {
            log::warn!("Receiver dropped, stopping input listener");
            anyhow::bail!("Receiver disconnected");
        }
    } else if released && !any_alt_pressed {
        // Only send Released when both Alt keys are released
        if tx.send(AltState::Released).is_err() {
            log::warn!("Receiver dropped, stopping input listener");
            anyhow::bail!("Receiver disconnected");
        }
    }

    Ok(())
}
