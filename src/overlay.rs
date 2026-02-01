use crate::blur::box_blur_multi_pass;
use crate::config::OverlayConfig;
use crate::input_listener::{start_alt_listener, AltState};
use crate::state::State;
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
    Attrs, Buffer as CtBuffer, Color as CtColor, FontSystem, Metrics, Shaping, SwashCache, Weight,
};
use tiny_skia::{Color, FillRule, Mask, Paint, Path, PathBuilder, Pixmap, Transform};

use crate::keybinding_reader::KeyBinding;

/// Helper function to create a rounded rectangle path
fn build_rounded_rect_path(x: f32, y: f32, width: f32, height: f32, radius: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();

    // Clamp radius to not exceed half of the smallest dimension
    let radius = radius.min(width / 2.0).min(height / 2.0);

    // Starting point: top-left, after the corner radius
    pb.move_to(x + radius, y);

    // Top edge
    pb.line_to(x + width - radius, y);

    // Top-right corner (using quadratic bezier for smooth curve)
    pb.quad_to(x + width, y, x + width, y + radius);

    // Right edge
    pb.line_to(x + width, y + height - radius);

    // Bottom-right corner
    pb.quad_to(x + width, y + height, x + width - radius, y + height);

    // Bottom edge
    pb.line_to(x + radius, y + height);

    // Bottom-left corner
    pb.quad_to(x, y + height, x, y + height - radius);

    // Left edge
    pb.line_to(x, y + radius);

    // Top-left corner (back to start)
    pb.quad_to(x, y, x + radius, y);

    pb.close();
    pb.finish()
}

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

    config: crate::config::OverlayConfig,

    // Rendering helpers
    font_system: FontSystem,
    swash_cache: SwashCache,
    // Keep last raster size to know when to re-layout if desired
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
        config: OverlayConfig,
    ) -> Self {
        Self {
            registry_state,
            seat_state,
            output_state,
            compositor_state,
            shm_state,
            layer_shell,
            pool,
            width: config.width,
            height: config.height,
            layer: None,
            keyboard: None,
            keyboard_focus: false,
            shortcuts,
            visible: false,
            configured: false,
            config,
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
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
        if let Some(layer) = &self.layer {
            // If visible, apply immediately.
            if self.visible {
                // Apply configured anchor position
                layer.set_anchor(crate::util::to_anchor(Some(self.config.anchor.clone())));
                layer.set_size(width, height);
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

        // Fill background with transparent
        pixmap.fill(Color::from_rgba8(0, 0, 0, 0));

        // Draw a centered panel where we'll place text
        let panel_w = (self.config.width as f32 * 0.95).max(600.0);
        let panel_h = (self.config.height as f32 * 0.90).max(400.0);
        // Center the panel inside the client surface
        let panel_x = ((self.config.width as f32 - panel_w) / 2.0).max(12.0);
        let panel_y = ((self.config.height as f32 - panel_h) / 2.0).max(12.0);

        // Draw the background panel - with or without rounded corners
        let corner_radius = self.config.corner_radius;
        let bg_color = OverlayConfig::parse_hex_color(&self.config.background_color).unwrap();

        // Only use clipping/rounded corners if corner_radius is defined and > 0
        if corner_radius > 0.0 {
            // Create a clip mask for rounded corners
            let mut clip_mask = match Mask::new(self.width, self.height) {
                Some(m) => m,
                None => {
                    log::error!("Failed to create clip mask");
                    return;
                }
            };

            // Draw the rounded rectangle into the clip mask
            if let Some(rounded_path) =
                build_rounded_rect_path(panel_x, panel_y, panel_w, panel_h, corner_radius)
            {
                clip_mask.fill_path(
                    &rounded_path,
                    FillRule::Winding,
                    true, // anti-alias
                    Transform::identity(),
                );
            }

            // Draw the background panel with rounded corners using the clip mask
            if let Some(rounded_path) =
                build_rounded_rect_path(panel_x, panel_y, panel_w, panel_h, corner_radius)
            {
                let mut paint = Paint::default();
                paint.set_color(Color::from_rgba8(bg_color.0, bg_color.1, bg_color.2, 230));
                pixmap.fill_path(
                    &rounded_path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    Some(&clip_mask),
                );
            }
        } else {
            // Draw regular rectangle without rounded corners (more efficient)
            let mut pb = PathBuilder::new();
            pb.move_to(panel_x, panel_y);
            pb.line_to(panel_x + panel_w, panel_y);
            pb.line_to(panel_x + panel_w, panel_y + panel_h);
            pb.line_to(panel_x, panel_y + panel_h);
            pb.close();

            if let Some(path) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(Color::from_rgba8(bg_color.0, bg_color.1, bg_color.2, 230));
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }

        // Copy pixmap pixels into the wl_shm canvas.
        // tiny-skia pixmap stores pixels as RGBA (premultiplied) bytes.
        // wl_shm::Format::Argb8888 on little-endian expects [B, G, R, A] byte order in memory.
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

                // ARGB8888 on little-endian: byte order is [B, G, R, A]
                canvas[dst_idx] = b; // B
                canvas[dst_idx + 1] = g; // G
                canvas[dst_idx + 2] = r; // R
                canvas[dst_idx + 3] = a; // A
            }
        }

        // Apply blur effect to the background canvas (BGRA format)
        // Convert to RGBA for blur, then convert back
        let mut rgba_temp = vec![0u8; canvas.len()];
        for i in 0..(width * height) {
            let idx = i * 4;
            rgba_temp[idx] = canvas[idx + 2]; // R from canvas B
            rgba_temp[idx + 1] = canvas[idx + 1]; // G
            rgba_temp[idx + 2] = canvas[idx]; // B from canvas R
            rgba_temp[idx + 3] = canvas[idx + 3]; // A
        }

        if self.config.apply_blur {
            box_blur_multi_pass(&mut rgba_temp, width, height, 12, 5);
        }

        // Convert back to BGRA format
        for i in 0..(width * height) {
            let idx = i * 4;
            canvas[idx] = rgba_temp[idx + 2]; // B
            canvas[idx + 1] = rgba_temp[idx + 1]; // G
            canvas[idx + 2] = rgba_temp[idx]; // R
            canvas[idx + 3] = rgba_temp[idx + 3]; // A
        }

        // Render text using cosmic-text directly into the canvas.
        // We'll create a Buffer for each logical line, shape it, then use its
        // draw callback to composite glyph pixels into the canvas.
        let font_size: f32 = self.config.font_size;
        let line_height: f32 = font_size * self.config.line_height; // Further increased to prevent overlap with wrapped text
        let metrics = Metrics::new(font_size, line_height);

        // Padding for text area
        let vertical_padding = 30.0;
        let horizontal_padding = 30.0;

        // Calculate how many shortcuts fit in one column
        let available_height = panel_h - vertical_padding * 2.0;
        let lines_per_column = (available_height / line_height).floor() as usize;

        // Determine number of columns needed (max 3 columns)
        let total_shortcuts = self.shortcuts.len().min(300);
        let num_columns = if total_shortcuts > lines_per_column * 2 {
            3
        } else if total_shortcuts > lines_per_column {
            2
        } else {
            1
        };

        // Column width calculation
        let column_gap = 15.0;
        let max_text_width = if num_columns > 1 {
            ((panel_w - horizontal_padding * 2.0 - column_gap * (num_columns as f32 - 1.0))
                / num_columns as f32)
                .max(10.0)
        } else {
            (panel_w - horizontal_padding * 2.0).max(10.0)
        };

        // Render all shortcuts in column layout
        for (idx, binding) in self.shortcuts.iter().enumerate().take(total_shortcuts) {
            // Stop if we exceed the capacity of all columns
            if idx >= lines_per_column * num_columns {
                break;
            }

            // Determine which column and position
            let column = idx / lines_per_column;
            let row_in_column = idx % lines_per_column;
            let start_x = (panel_x
                + horizontal_padding
                + column as f32 * (max_text_width + column_gap)) as usize;

            let offset_y =
                (panel_y + vertical_padding + row_in_column as f32 * line_height) as usize;

            // Safety check: skip if text would go beyond panel bottom
            if offset_y + line_height as usize > (panel_y + panel_h - vertical_padding) as usize {
                continue;
            }

            // Truncate description with ellipsis if longer than 20 characters
            // Use char_indices to handle multi-byte UTF-8 characters properly
            // let description = if binding.description.chars().count() > 25 {
            //     let truncate_pos = binding
            //         .description
            //         .char_indices()
            //         .nth(25)
            //         .map(|(idx, _)| idx)
            //         .unwrap_or(binding.description.len());
            //     format!("{}..", &binding.description[..truncate_pos])
            // } else {
            //     binding.description.clone()
            // };
            let description = binding.description.clone();

            // Create cosmic-text buffer and shape
            let mut ct = CtBuffer::new(&mut self.font_system, metrics);
            let mut ct = ct.borrow_with(&mut self.font_system);

            ct.set_size(Some(max_text_width), None);

            // Build full text with bold binding and normal description
            let binding_text = format!("{}", binding);
            let separator = " : ";
            let full_text = format!("{}{}{}", binding_text, separator, description);

            // First set text with normal attributes
            let attrs_normal = Attrs::new();
            ct.set_text(&full_text, &attrs_normal, Shaping::Advanced, None);

            // Now modify the first line to make the binding portion bold
            let binding_end = binding_text.len();
            if !ct.lines.is_empty() {
                let line = &mut ct.lines[0];
                let attrs_bold = Attrs::new().weight(Weight::BOLD);

                // Create attrs list with bold for binding, normal for rest
                let mut attrs_list = cosmic_text::AttrsList::new(&attrs_bold);
                if binding_end < full_text.len() {
                    attrs_list.add_span(binding_end..full_text.len(), &attrs_normal);
                }
                line.set_attrs_list(attrs_list);
            }

            ct.shape_until_scroll(true);

            // Provide white color
            // TODO: read it from the overlay_config
            let text_color = OverlayConfig::parse_hex_color(&self.config.text_color).unwrap();
            let ct_color = CtColor::rgb(text_color.0, text_color.1, text_color.2);

            // Draw callback from cosmic-text emits pixel-alphas for glyph rasterization.
            // We'll composite those against what's already in `canvas`.
            ct.draw(&mut self.swash_cache, ct_color, |x, y, w, h, color| {
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
        log::debug!("Alt pressed :{}", alt_pressed);
        if alt_pressed {
            log::debug!("Showing overlay");
            self.show_overlay();
        } else {
            log::debug!("Hiding overlay");
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
fn run_single_overlay(
    shortcuts: Vec<KeyBinding>,
    exit_flag: Arc<AtomicBool>,
    config: OverlayConfig,
) -> Result<()> {
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
        config,
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

    log::debug!("Overlay instance exiting");
    Ok(())
}

pub fn start(state: Arc<State>) -> Result<()> {
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
                log::debug!("==> Alt pressed alone - spawning new overlay thread");

                let shortcuts_clone = state.clone_keybindings();
                let flag = Arc::new(AtomicBool::new(false));
                let flag_clone = Arc::clone(&flag);
                let config_clone = state.clone_config();

                let handle = std::thread::spawn(move || {
                    if let Err(e) = run_single_overlay(shortcuts_clone, flag_clone, config_clone) {
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
