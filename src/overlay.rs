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
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};

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
        }
    }

    pub fn create_layer_surface(&mut self, qh: &QueueHandle<Self>) {
        let surface = self.compositor_state.create_surface(qh);
        
        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Overlay,
            Some("wl-shortcuts-overlay"),
            None,
        );

        layer.set_anchor(Anchor::empty());
        layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        layer.set_size(self.width, self.height);

        layer.commit();
        self.layer = Some(layer);
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
        log::info!("Overlay visibility: {}", self.visible);
        
        if let Some(layer) = &self.layer {
            if self.visible {
                // Restore original size when becoming visible
                layer.set_size(self.width, self.height);
            } else {
                // Hide by setting size to 0
                layer.set_size(0, 0);
            }
            layer.commit();
        }
    }

    pub fn draw(&mut self) {
        if !self.visible {
            return;
        }

        let Some(layer) = self.layer.as_ref() else {
            return;
        };

        let stride = self.width as i32 * 4;
        let width = self.width as usize;

        let (buffer, canvas) = self
            .pool
            .create_buffer(
                self.width as i32,
                self.height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Failed to create buffer");

        // Draw semi-transparent background with blur effect simulation
        for pixel in canvas.chunks_exact_mut(4) {
            // ARGB format: Alpha, Red, Green, Blue
            pixel[0] = 200; // Alpha (transparency)
            pixel[1] = 30;  // Blue
            pixel[2] = 30;  // Green
            pixel[3] = 30;  // Red
        }

        // Draw shortcut text (simple rendering)
        let start_y: usize = 50;
        let line_height: usize = 30;
        
        for (i, binding) in self.shortcuts.iter().take(20).enumerate() {
            let y = start_y + (i * line_height);
            let text = format!("{}: {}", binding, binding.description);
            Self::draw_text_static(canvas, 20, y, &text, width);
        }

        layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);
        layer.wl_surface().attach(Some(buffer.wl_buffer()), 0, 0);
        layer.commit();
    }

    fn draw_text_static(canvas: &mut [u8], x: usize, y: usize, text: &str, width: usize) {
        // Simple text rendering - just draw white pixels in a basic pattern
        // In a real application, use a proper text rendering library
        const MAX_CHARS: usize = 60;
        let stride = width * 4;
        
        for (char_offset, _ch) in text.chars().enumerate().take(MAX_CHARS) {
            let px = x + char_offset * 8;
            if px + 8 > width || y + 12 > canvas.len() / stride {
                break;
            }

            // Draw a simple rectangle for each character
            for dy in 0..12 {
                for dx in 0..6 {
                    let offset = ((y + dy) * stride) + ((px + dx) * 4);
                    if offset + 3 < canvas.len() {
                        canvas[offset] = 255;     // Alpha
                        canvas[offset + 1] = 255; // Blue
                        canvas[offset + 2] = 255; // Green
                        canvas[offset + 3] = 255; // Red
                    }
                }
            }
        }
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
        log::info!("Layer surface closed");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let mut size_changed = false;
        
        if configure.new_size.0 != 0 && configure.new_size.0 != self.width {
            self.width = configure.new_size.0;
            size_changed = true;
        }
        if configure.new_size.1 != 0 && configure.new_size.1 != self.height {
            self.height = configure.new_size.1;
            size_changed = true;
        }

        if size_changed {
            layer.set_size(self.width, self.height);
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
        // Toggle visibility on Escape or Super+/ key
        if event.keysym == Keysym::Escape {
            self.toggle_visibility();
        }
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
    }

    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
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

pub fn run_overlay(shortcuts: Vec<KeyBinding>) -> Result<()> {
    let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let (globals, mut event_queue) = registry_queue_init(&conn).context("Failed to init registry")?;
    let qh = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh)
        .context("wl_compositor not available")?;
    let layer_shell = LayerShell::bind(&globals, &qh)
        .context("layer_shell not available")?;
    let shm_state = Shm::bind(&globals, &qh)
        .context("wl_shm not available")?;

    let pool = SlotPool::new(800 * 600 * 4, &shm_state)
        .context("Failed to create slot pool")?;

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

    app.create_layer_surface(&qh);
    // Start with overlay visible
    app.toggle_visibility();
    app.draw();

    log::info!("Starting Wayland overlay event loop");

    loop {
        event_queue.blocking_dispatch(&mut app)
            .context("Failed to dispatch events")?;
    }
}
