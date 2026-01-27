use crate::input_listener::{start_alt_listener, AltState};
use anyhow::{Context, Result};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use cosmic_text::{
    Attrs, Buffer as CtBuffer, Color as CtColor, FontSystem, Metrics, Shaping, SwashCache,
};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};

use crate::keybinding_reader::KeyBinding;

pub struct OverlayApp {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: Shm,
    layer_shell: LayerShell,

    pool: SlotPool,
    width: u32,
    height: u32,
    layer: Option<LayerSurface>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,

    shortcuts: Vec<KeyBinding>,
    visible: bool,
    configured: bool,

    // When true we prefer the client-specified size and ignore compositor
    // configure suggested sizes. This allows a fixed-size overlay that does
    // not cover the whole display.
    use_client_size: bool,

    // Rendering helpers
    font_system: FontSystem,
    swash_cache: SwashCache,
    // Keep last raster size to know when to re-layout if desired
    cached_size: (u32, u32),
}

impl OverlayApp {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry_state: RegistryState,
        seat_state: SeatState,
        output_state: OutputState,
        compositor_state: CompositorState,
        shm_state: Shm,
        layer_shell: LayerShell,
        pool: SlotPool,
        shortcuts: Vec<KeyBinding>,
    ) -> Self {
        Self {
            registry_state,
            seat_state,
            output_state,
            compositor_state,
            shm_state,
            layer_shell,
            pool,
            width: 800,
            height: 600,
            layer: None,
            keyboard: None,
            keyboard_focus: false,
            shortcuts,
            visible: false,
            configured: false,
            use_client_size: true,
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            cached_size: (0, 0),
        }
    }

    pub fn create_layer_surface(&mut self, qh: &QueueHandle<Self>) {
        let surface = self.compositor_state.create_surface(qh);

        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Overlay,
            Some("shortcuts-overlay"),
            None,
        );

        // Start hidden in an un-mapped state: per protocol, clients should
        // perform an initial commit without buffer attached. We set default
        // keyboard interactivity to OnDemand so the compositor can focus/unfocus
        // normally (click to focus) and the overlay behaves like a popup.
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);

        // Start hidden with 0x0 size. Many compositors allow this only when
        // anchored to opposing edges; to avoid protocol errors we will ensure
        // opposing anchors are set before requesting a 0x0 size. The surface
        // remains unmapped until we attach a buffer after handling configure.
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_size(0, 0);
        layer.commit();

        self.layer = Some(layer);
    }

    pub fn destroy(&mut self) {
        if let Some(layer) = self.layer.take() {
            layer.wl_surface().destroy();
        }
    }

    /// Set overlay size with validation and defaults.
    ///
    /// If caller passes 0 for width/height we substitute safe defaults so the
    /// overlay remains a client-sized popup and does not require opposing
    /// anchors for zero dimensions.
    pub fn set_overlay_size(&mut self, width: u32, height: u32) {
        // Defaults used when caller passes zero.
        const DEFAULT_WIDTH: u32 = 800;
        const DEFAULT_HEIGHT: u32 = 600;

        let in_w = width;
        let in_h = height;

        // If the caller explicitly passed zero and intends the compositor to
        // assign that dimension, they'd need to set anchors appropriately.
        // For simplicity and safety we convert zero -> default client size so
        // we can clear anchors and request a client-sized popup.
        let w = if in_w == 0 { DEFAULT_WIDTH } else { in_w };
        let h = if in_h == 0 { DEFAULT_HEIGHT } else { in_h };

        println!(
            "set_overlay_size: requested {}x{}, using {}x{}",
            in_w, in_h, w, h
        );

        self.width = w;
        self.height = h;

        if let Some(layer) = &self.layer {
            // If visible, apply immediately.
            if self.visible {
                // Clear anchors so compositor treats this as a popup (often centered).
                layer.set_anchor(Anchor::empty());
                layer.set_size(self.width, self.height);
                layer.commit();
            } else {
                // Not visible: keep values for next time we show. No commit to avoid mapping.
            }
        }
    }

    pub fn show_overlay(&mut self) {
        log::debug!("Show overlay");
        if self.visible {
            return;
        }
        self.visible = true;
        log::debug!("Overlay visibility: {}", self.visible);

        // Ensure we prefer client size when visible.
        self.use_client_size = true;

        // Ensure width/height are valid and apply via helper (applies commit if visible).
        self.set_overlay_size(self.width, self.height);

        // If we've already been configured, draw immediately.
        if self.configured {
            self.draw();
        }
    }

    pub fn hide_overlay(&mut self) {
        if !self.visible {
            return;
        }
        self.visible = false;
        log::debug!("Overlay visibility: {}", self.visible);

        if let Some(layer) = &self.layer {
            // Robust unmap sequence:
            // 1) Anchor to all edges so any zero-size requests are legal per protocol.
            // 2) Request a 0x0 size to indicate we don't want a mapped surface.
            // 3) Attach a null buffer to unmap the surface.
            // 4) Commit the state to apply.
            //
            // This avoids relying on 1x1 hacks or leaving a damaged buffer that
            // compositors may show at the top-left.
            layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
            layer.set_size(0, 0);
            // Attach null buffer to unmap
            layer.wl_surface().attach(None, 0, 0);
            layer.commit();
        }
    }

    pub fn draw(&mut self) {
        log::debug!("Drawing overlay");
        let Some(layer) = self.layer.as_ref() else {
            return;
        };

        // stride in bytes per row (4 bytes per pixel)
        let stride = self.width as i32 * 4;
        let width = self.width as usize;
        let height = self.height as usize;

        // Create wl_shm buffer and canvas
        let (shm_buffer, canvas) = match self.pool.create_buffer(
            self.width as i32,
            self.height as i32,
            stride,
            wl_shm::Format::Argb8888,
        ) {
            Ok(pair) => pair,
            Err(e) => {
                log::error!("Failed to create buffer: {:?}", e);
                return;
            }
        };

        // Create tiny-skia pixmap and draw background + panel
        let mut pixmap = match Pixmap::new(self.width, self.height) {
            Some(p) => p,
            None => {
                log::error!("Failed to create tiny-skia Pixmap");
                // Fallback: fill canvas with a simple translucent background
                for pixel in canvas.chunks_exact_mut(4) {
                    pixel[0] = 200u8; // A
                    pixel[1] = 30u8; // B
                    pixel[2] = 30u8; // G
                    pixel[3] = 30u8; // R
                }
                layer
                    .wl_surface()
                    .damage_buffer(0, 0, self.width as i32, self.height as i32);
                layer
                    .wl_surface()
                    .attach(Some(shm_buffer.wl_buffer()), 0, 0);
                layer.commit();
                return;
            }
        };

        // Fill background with semi-transparent gray
        pixmap.fill(Color::from_rgba8(119, 119, 119, 250));

        // Draw a rounded panel where we'll place text
        let panel_w = (self.width as f32 * 0.6).max(200.0);
        let panel_h = (self.height as f32 * 0.8).max(200.0);
        // Center the panel inside the client surface
        let panel_x = ((self.width as f32 - panel_w) / 2.0).max(12.0);
        let panel_y = ((self.height as f32 - panel_h) / 2.0).max(12.0);

        let mut pb = PathBuilder::new();
        // Use explicit path construction for a rectangle so we don't rely on
        // APIs that may be unavailable in some tiny-skia versions.
        pb.move_to(panel_x, panel_y);
        pb.line_to(panel_x + panel_w, panel_y);
        pb.line_to(panel_x + panel_w, panel_y + panel_h);
        pb.line_to(panel_x, panel_y + panel_h);
        pb.close();
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(Color::from_rgba8(119, 119, 119, 250));
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }

        // Copy pixmap pixels into the wl_shm canvas.
        // tiny-skia pixmap stores pixels as RGBA (premultiplied) bytes.
        // The original canvas format used in this project expects [A, B, G, R].
        let pixdata = pixmap.data();
        // pixdata length should be width * height * 4
        if pixdata.len() >= width * height * 4 {
            for i in 0..(width * height) {
                let src_idx = i * 4;
                let dst_idx = i * 4;
                let r = pixdata[src_idx];
                let g = pixdata[src_idx + 1];
                let b = pixdata[src_idx + 2];
                let a = pixdata[src_idx + 3];

                canvas[dst_idx] = a; // A
                canvas[dst_idx + 1] = b; // B
                canvas[dst_idx + 2] = g; // G
                canvas[dst_idx + 3] = r; // R
            }
        }

        // Render text using cosmic-text directly into the canvas.
        // We'll create a Buffer for each logical line, shape it, then use its
        // draw callback to composite glyph pixels into the canvas.
        const FONT_SIZE: f32 = 15.0;
        const LINE_HEIGHT: f32 = FONT_SIZE * 1.5;
        let metrics = Metrics::new(FONT_SIZE, LINE_HEIGHT);

        let mut offset_y = (panel_y + 12.0) as usize;
        let start_x = (panel_x + 12.0) as usize;
        let max_text_width = (panel_w - 24.0).max(10.0) as f32;

        for binding in self.shortcuts.iter().take(100) {
            let text = format!("{} — {}", binding, binding.description);

            // Create cosmic-text buffer and shape
            let mut ct = CtBuffer::new(&mut self.font_system, metrics);
            let mut ct = ct.borrow_with(&mut self.font_system);

            ct.set_size(Some(max_text_width), None);
            let attrs = Attrs::new();
            ct.set_text(&text, &attrs, Shaping::Advanced, None);
            ct.shape_until_scroll(true);

            // Provide white color
            const CT_WHITE: CtColor = CtColor::rgb(0xFF, 0xFF, 0xFF);

            // Draw callback from cosmic-text emits pixel-alphas for glyph rasterization.
            // We'll composite those against what's already in `canvas`.
            ct.draw(&mut self.swash_cache, CT_WHITE, |x, y, w, h, color| {
                // The example rasterizer emits 1x1 pixels; guard other sizes.
                if w != 1 || h != 1 {
                    return;
                }

                // Coordinates emitted by cosmic-text are relative to the buffer.
                // Map them into our canvas coordinates.
                let cx = start_x as isize + x as isize;
                let cy = offset_y as isize + y as isize;
                if cx < 0 || cy < 0 {
                    return;
                }
                let ux = cx as usize;
                let uy = cy as usize;
                if ux >= width || uy >= height {
                    return;
                }

                let idx = (uy * width + ux) * 4;
                if idx + 3 >= canvas.len() {
                    return;
                }

                // Source color (non-premultiplied) from cosmic-text
                let sa = color.a() as f32 / 255.0;
                let sr = color.r() as f32;
                let sg = color.g() as f32;
                let sb = color.b() as f32;

                // Convert source to premultiplied components
                let src_r_p = sr * sa;
                let src_g_p = sg * sa;
                let src_b_p = sb * sa;

                // Destination color (we store as BGRA in canvas memory, premultiplied)
                let dst_b_p = canvas[idx] as f32;
                let dst_g_p = canvas[idx + 1] as f32;
                let dst_r_p = canvas[idx + 2] as f32;
                let dst_a = canvas[idx + 3] as f32 / 255.0;

                // Composite using premultiplied alpha: out = src + dst * (1 - sa)
                let out_r_p = (src_r_p + dst_r_p * (1.0 - sa)).clamp(0.0, 255.0);
                let out_g_p = (src_g_p + dst_g_p * (1.0 - sa)).clamp(0.0, 255.0);
                let out_b_p = (src_b_p + dst_b_p * (1.0 - sa)).clamp(0.0, 255.0);
                let out_a = ((sa + dst_a * (1.0 - sa)) * 255.0).clamp(0.0, 255.0);

                // Write back as BGRA (premultiplied)
                canvas[idx] = out_b_p as u8; // B
                canvas[idx + 1] = out_g_p as u8; // G
                canvas[idx + 2] = out_r_p as u8; // R
                canvas[idx + 3] = out_a as u8; // A
            });

            // Advance down by the buffer's laid-out height (number of lines * line height)
            // For safety, we advance by one LINE_HEIGHT per buffer in most cases.
            offset_y += LINE_HEIGHT as usize;
            // Stop if panel bottom reached
            if offset_y > (panel_y + panel_h - 12.0) as usize {
                break;
            }
        }

        // Inform compositor which region changed and attach buffer
        layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);
        layer
            .wl_surface()
            .attach(Some(shm_buffer.wl_buffer()), 0, 0);
        layer.commit();
    }
}

impl CompositorHandler for OverlayApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for OverlayApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for OverlayApp {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        log::debug!("Layer surface closed");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // Log configure details to help debugging compositor reconfigures.
        log::debug!(
            "configure: serial={}, suggested={}x{}",
            _serial,
            configure.new_size.0,
            configure.new_size.1
        );

        // If the overlay is visible, perform the draw now (which will attach a
        // buffer and commit). This ensures we only attach after an initial
        // configure has been received.
        if self.visible {
            self.draw();
        }
    }
}

impl SeatHandler for OverlayApp {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard = self
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");
            self.keyboard = Some(keyboard);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(keyboard) = self.keyboard.take() {
                keyboard.release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for OverlayApp {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _keysyms: &[Keysym],
    ) {
        self.keyboard_focus = true;
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        self.keyboard_focus = false;
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        // Keep minimal: overlay visibility is controlled by modifier state
        if event.keysym == Keysym::Control_L || event.keysym == Keysym::Control_R {
            log::trace!("Keyboard focus gained");
            self.keyboard_focus = true;
        }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if event.keysym == Keysym::Control_L || event.keysym == Keysym::Control_R {
            log::trace!("Keyboard focus lost");
            self.keyboard_focus = false;
            self.hide_overlay();
        }
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        // Show overlay only while Alt is held down. Hide it when released.
        let alt_pressed = modifiers.alt;
        log::info!("Alt pressed :{}", alt_pressed);
        if alt_pressed {
            println!("Showing overlay");
            self.show_overlay();
        } else {
            println!("Hiding overlay");
            self.hide_overlay();
        }
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _: u32,
        _event: KeyEvent,
    ) {
    }
}

impl PointerHandler for OverlayApp {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        _events: &[PointerEvent],
    ) {
    }
}

impl ShmHandler for OverlayApp {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl ProvidesRegistryState for OverlayApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}

delegate_compositor!(OverlayApp);
delegate_output!(OverlayApp);
delegate_shm!(OverlayApp);
delegate_seat!(OverlayApp);
delegate_keyboard!(OverlayApp);
delegate_pointer!(OverlayApp);
delegate_layer!(OverlayApp);
delegate_registry!(OverlayApp);

// Run a single overlay instance in its own thread
fn run_single_overlay(shortcuts: Vec<KeyBinding>, exit_flag: Arc<AtomicBool>) -> Result<()> {
    log::trace!("Starting new overlay instance...");

    let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let (globals, mut event_queue) =
        registry_queue_init(&conn).context("Failed to init registry")?;
    let qh = event_queue.handle();

    let compositor_state =
        CompositorState::bind(&globals, &qh).context("wl_compositor not available")?;
    let layer_shell = LayerShell::bind(&globals, &qh).context("layer_shell not available")?;
    let shm_state = Shm::bind(&globals, &qh).context("wl_shm not available")?;

    let pool = SlotPool::new(800 * 600 * 4, &shm_state).context("Failed to create slot pool")?;

    let mut app = OverlayApp::new(
        RegistryState::new(&globals),
        SeatState::new(&globals, &qh),
        OutputState::new(&globals, &qh),
        compositor_state,
        shm_state,
        layer_shell,
        pool,
        shortcuts,
    );

    let env_width = std::env::var("SHORTCUTS_OVERLAY_WIDTH")
        .ok()
        .and_then(|s| s.parse::<u32>().ok());
    let env_height = std::env::var("SHORTCUTS_OVERLAY_HEIGHT")
        .ok()
        .and_then(|s| s.parse::<u32>().ok());

    let apply_width = env_width.unwrap_or(app.width);
    let apply_height = env_height.unwrap_or(app.height);

    app.set_overlay_size(apply_width, apply_height);
    app.create_layer_surface(&qh);
    app.show_overlay();

    log::trace!("Overlay instance running, entering event loop...");

    // Run event loop - check atomic boolean to exit on Alt release
    loop {
        if exit_flag.load(Ordering::Relaxed) {
            log::debug!("Exit signal received, destroying overlay...");
            app.destroy();
            break;
        }

        // Use roundtrip to ensure we receive all events including configure
        if let Err(e) = event_queue.roundtrip(&mut app) {
            log::error!("Failed to roundtrip Wayland events: {}", e);
            break;
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    println!("Overlay instance exiting");
    Ok(())
}

pub fn start(shortcuts: Vec<KeyBinding>) -> Result<()> {
    let ctrl_receiver = match start_alt_listener() {
        Ok(rx) => {
            log::info!("Successfully started libinput Alt key listener");
            rx
        }
        Err(e) => {
            log::error!("Failed to start input listener: {}", e);
            return Err(e).context("Failed to start input listener");
        }
    };

    log::debug!("Main thread: listening for Alt key events...");

    let mut overlay_thread: Option<std::thread::JoinHandle<()>> = None;
    let mut exit_flag: Option<Arc<AtomicBool>> = None;

    loop {
        match ctrl_receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(AltState::Pressed) => {
                log::debug!("==> pressed - spawning new overlay thread");

                let shortcuts_clone = shortcuts.clone();
                let flag = Arc::new(AtomicBool::new(false));
                let flag_clone = Arc::clone(&flag);

                let handle = std::thread::spawn(move || {
                    if let Err(e) = run_single_overlay(shortcuts_clone, flag_clone) {
                        log::error!("Overlay thread error: {}", e);
                    }
                });

                overlay_thread = Some(handle);
                exit_flag = Some(flag);
                log::debug!("==> New overlay thread spawned");
            }
            Ok(AltState::Released) => {
                log::debug!("==> released - signaling overlay to exit");

                if let Some(flag) = &exit_flag {
                    flag.store(true, Ordering::Relaxed);
                    log::debug!("==> Exit signal sent");
                }

                if let Some(handle) = overlay_thread.take() {
                    log::debug!("==> Waiting for overlay thread to finish...");
                    let _ = handle.join();
                    log::debug!("==> Overlay thread finished");
                }

                exit_flag = None;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                log::error!("Input listener thread disconnected");
                anyhow::bail!("Input listener thread disconnected");
            }
        }
    }
}
