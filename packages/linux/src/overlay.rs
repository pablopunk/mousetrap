//! Wayland layer-shell overlay, rendered into a `wl_shm` buffer with
//! tiny-skia (pure Rust, no C dependencies).
//!
//! The layer surface is created without an explicit output, so the
//! compositor assigns it to the focused output — this is what makes the
//! grid appear on the active monitor on any wlr-layer-shell compositor.

use std::sync::OnceLock;

use fontdue::Font;
use smithay_client_toolkit as sctk;
use sctk::{
    compositor::Surface,
    shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerSurface},
    shm::slot::{Buffer, SlotPool},
    shm::Shm,
};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, PixmapMut, Rect, Stroke, Transform};
use wayland_client::{
    protocol::{wl_shm, wl_surface::WlSurface},
    QueueHandle,
};

use crate::geometry::{Bounds, ROWS};
use crate::state::SessionState;

const OVERLAY_ALPHA: f32 = 0.18;
const CELL_PADDING: f32 = 1.0;

static FONT: OnceLock<Font> = OnceLock::new();

fn font() -> &'static Font {
    FONT.get_or_init(|| {
        let bytes: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");
        Font::from_bytes(bytes, fontdue::FontSettings::default())
            .expect("embedded font parses")
    })
}

pub struct Overlay {
    surface: Option<WlSurface>,
    layer_surface: Option<LayerSurface>,
    pool: Option<SlotPool>,
    buffer: Option<Buffer>,
    /// Logical size from the last configure event.
    pub size: (u32, u32),
    /// Buffer scale of the surface (from preferred_buffer_scale).
    pub scale: i32,
    /// Configure received since the surface was last (re)created.
    configured: bool,
    /// Whether the grid is currently being shown.
    shown: bool,
    /// Redraw requested (state or size changed); consumed by `redraw`.
    needs_redraw: bool,
    /// Grid bounds in logical surface coordinates.
    pub bounds: Bounds,
    /// Current session state (drives drawing).
    pub state: Option<SessionState>,
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            surface: None,
            layer_surface: None,
            pool: None,
            buffer: None,
            size: (0, 0),
            scale: 1,
            configured: false,
            shown: false,
            needs_redraw: false,
            bounds: (0, 0, 0, 0),
            state: None,
        }
    }

    /// Create the layer surface (or recreate it after teardown).
    pub fn ensure_surface(
        &mut self,
        compositor: &sctk::compositor::CompositorState,
        layer_shell: &sctk::shell::wlr_layer::LayerShell,
        qh: &QueueHandle<crate::daemon::Daemon>,
    ) {
        if self.surface.is_some() {
            return;
        }
        let surface: WlSurface = compositor.create_surface(qh);
        let wl_surface = surface.clone();
        let layer_surface = layer_shell.create_layer_surface(
            qh,
            Surface::from(surface),
            Layer::Overlay,
            Some("mousetrap"),
            None, // no explicit output → compositor picks the focused one
        );
        self.surface = Some(wl_surface);
        self.layer_surface = Some(layer_surface);
        self.configured = false;
        self.pool = None;
        self.buffer = None;
    }

    /// Map the surface: anchor it to all edges of the focused output.
    pub fn show(&mut self) {
        let Some(layer) = &self.layer_surface else { return };
        layer.set_anchor(
            Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        );
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        self.shown = true;
        self.needs_redraw = true;
        self.commit();
    }

    /// Unmap and tear down the layer surface.
    pub fn hide(&mut self) {
        self.shown = false;
        self.state = None;
        self.needs_redraw = false;
        self.layer_surface = None;
        self.surface = None;
        self.buffer = None;
        self.pool = None;
        self.configured = false;
    }

    fn commit(&self) {
        if let Some(surface) = &self.surface {
            surface.commit();
        }
    }

    /// Handle a configure event: record the suggested size.
    /// (sctk already acks the configure serial before calling us.)
    /// Only marks a redraw if the size actually changed — redrawing on every
    /// configure would ping-pong commits with the compositor.
    pub fn configure(&mut self, size: (u32, u32)) {
        if size != self.size {
            self.needs_redraw = true;
        }
        self.size = size;
        self.configured = true;
        self.bounds = (0, 0, size.0 as i32, size.1 as i32);
    }

    /// Request a redraw on the next `redraw` call.
    pub fn invalidate(&mut self) {
        self.needs_redraw = true;
    }

    /// Redraw the grid into a fresh shm buffer and commit it.
    pub fn redraw(&mut self, shm: &Shm) {
        if !self.shown || !self.configured || self.size == (0, 0) {
            return;
        }
        if !self.needs_redraw {
            return;
        }
        self.needs_redraw = false;
        let (w, h) = self.size;
        let scale = self.scale.max(1) as u32;
        let pw = w * scale;
        let ph = h * scale;
        let stride = (pw * 4) as i32;

        let pool = self.pool.get_or_insert_with(|| {
            SlotPool::new((stride as usize) * ph as usize, shm).expect("shm pool")
        });
        let (buffer, canvas) = match pool.create_buffer(pw as i32, ph as i32, stride, wl_shm::Format::Argb8888) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("mousetrap: buffer creation failed: {e:?}");
                return;
            }
        };
        self.buffer = Some(buffer);

        let mut pixmap = match PixmapMut::from_bytes(canvas, pw, ph) {
            Some(p) => p,
            None => return,
        };
        draw_grid(&mut pixmap, pw, ph, scale, self.bounds, self.state.as_ref());

        if let Some(surface) = &self.surface {
            if let Some(buffer) = &self.buffer {
                if let Err(e) = buffer.attach_to(surface) {
                    eprintln!("mousetrap: attach failed: {e:?}");
                    return;
                }
            }
            surface.damage_buffer(0, 0, pw as i32, ph as i32);
            surface.set_buffer_scale(scale as i32);
        }
        self.commit();
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_xywh(x, y, w, h).expect("valid rect")
}

fn draw_grid(
    pix: &mut PixmapMut,
    pw: u32,
    ph: u32,
    scale: u32,
    bounds: Bounds,
    state: Option<&SessionState>,
) {
    let s = scale as f32;
    // Clear to transparent.
    pix.fill(Color::TRANSPARENT);

    // Dim the whole output.
    fill_rect(
        pix,
        rect(0.0, 0.0, pw as f32, ph as f32),
        Color::from_rgba8(0, 0, 0, (OVERLAY_ALPHA * 255.0) as u8),
    );

    let Some(state) = state else { return };
    let (bx, by, bw, bh) = state.current_bounds;
    let bx = bx as f32 * s;
    let by = by as f32 * s;
    let bw = bw as f32 * s;
    let bh = bh as f32 * s;

    // Current region: subtle white fill + border.
    fill_rect(pix, rect(bx, by, bw, bh), Color::from_rgba8(255, 255, 255, 20));
    stroke_rect(pix, rect(bx, by, bw, bh), Color::from_rgba8(255, 255, 255, 115), 2.0);

    // Cells with key labels.
    let row_h = bh / ROWS.len() as f32;
    for (r, row) in ROWS.iter().enumerate() {
        let col_w = bw / row.chars().count() as f32;
        for (c, ch) in row.chars().enumerate() {
            let x = bx + c as f32 * col_w + CELL_PADDING * s;
            let y = by + r as f32 * row_h + CELL_PADDING * s;
            let w = col_w - 2.0 * CELL_PADDING * s;
            let h = row_h - 2.0 * CELL_PADDING * s;
            fill_rect(pix, rect(x, y, w, h), Color::from_rgba8(255, 255, 255, 31));
            stroke_rect(pix, rect(x, y, w, h), Color::from_rgba8(255, 255, 255, 71), 1.0);
            let font_size = (10.0 * s).max(w.min(h) * 0.33);
            let label = ch.to_uppercase().to_string();
            draw_text(pix, &label, font_size, x, y, w, h, Color::from_rgba8(255, 255, 255, 242));
        }
    }
    let _ = bounds;
}

fn fill_rect(pix: &mut PixmapMut, rect: Rect, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let path = PathBuilder::from_rect(rect);
    pix.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

fn stroke_rect(pix: &mut PixmapMut, rect: Rect, color: Color, width: f32) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let path = PathBuilder::from_rect(rect);
    let stroke = Stroke { width, ..Stroke::default() };
    pix.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

fn draw_text(pix: &mut PixmapMut, text: &str, size: f32, x: f32, y: f32, w: f32, h: f32, color: Color) {
    let font = font();
    // Measure.
    let mut total_width = 0.0;
    for ch in text.chars() {
        let (metrics, _) = font.rasterize(ch, size);
        total_width += metrics.advance_width;
    }
    let mut cursor = x + ((w - total_width) / 2.0).max(0.0);
    let baseline = y + h * 0.78;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        let gx = cursor + metrics.xmin as f32;
        let gy = baseline + metrics.ymin as f32;
        blend_bitmap(pix, &bitmap, metrics.width, metrics.height, gx, gy, color);
        cursor += metrics.advance_width;
    }
}

/// Blend a coverage bitmap (alpha channel) into a premultiplied pixmap.
fn blend_bitmap(pix: &mut PixmapMut, bitmap: &[u8], w: usize, h: usize, x: f32, y: f32, color: Color) {
    let src_r = color.red();
    let src_g = color.green();
    let src_b = color.blue();
    let color_alpha = color.alpha();
    let pw = pix.width() as usize;
    let ph = pix.height() as usize;
    let data = pix.data_mut();
    for row in 0..h {
        let py = (y as i32) + row as i32;
        if py < 0 || py as usize >= ph {
            continue;
        }
        for col in 0..w {
            let px = (x as i32) + col as i32;
            if px < 0 || px as usize >= pw {
                continue;
            }
            let a = bitmap[row * w + col] as f32 / 255.0 * color_alpha;
            if a <= 0.0 {
                continue;
            }
            let idx = (py as usize * pw + px as usize) * 4;
            let dst = &mut data[idx..idx + 4];
            dst[0] = (src_r * a + dst[0] as f32 * (1.0 - a)) as u8;
            dst[1] = (src_g * a + dst[1] as f32 * (1.0 - a)) as u8;
            dst[2] = (src_b * a + dst[2] as f32 * (1.0 - a)) as u8;
            dst[3] = (a * 255.0 + dst[3] as f32 * (1.0 - a)) as u8;
        }
    }
}
