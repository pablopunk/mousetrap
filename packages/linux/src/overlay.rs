//! Wayland layer-shell overlay, rendered into a `wl_shm` buffer.
//!
//! Drawing is done by hand: axis-aligned rectangles and text only, with
//! strictly bounded loops. A third-party rasterizer (tiny-skia) previously
//! hung in an infinite loop on degenerate grid geometry — manual blending
//! cannot.

use std::sync::OnceLock;
use std::time::Instant;

use fontdue::Font;
use sctk::{
    compositor::Surface,
    shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerSurface},
    shm::Shm,
    shm::slot::{Buffer, SlotPool},
};
use smithay_client_toolkit as sctk;
use wayland_client::{
    QueueHandle,
    protocol::{wl_shm, wl_surface::WlSurface},
};

use crate::geometry::{Bounds, ROWS};
use crate::state::SessionState;

const OVERLAY_ALPHA: f32 = 0.18;
const CELL_PADDING: f32 = 1.0;

static FONT: OnceLock<Font> = OnceLock::new();

fn font() -> &'static Font {
    FONT.get_or_init(|| {
        let bytes: &[u8] = include_bytes!("../assets/DejaVuSansMono.ttf");
        Font::from_bytes(bytes, fontdue::FontSettings::default()).expect("embedded font parses")
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
    /// Free-mouse indicator position in logical surface coordinates.
    indicator: Option<Indicator>,
}

struct Indicator {
    point: (i32, i32),
    started_at: Instant,
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
            indicator: None,
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

    /// Whether `layer` is the currently mapped layer surface (used to
    /// ignore stale configure/closed events for destroyed surfaces).
    pub fn is_current_surface(&self, layer: &LayerSurface) -> bool {
        self.layer_surface
            .as_ref()
            .map(|current| current == layer)
            .unwrap_or(false)
    }

    /// Map the surface: anchor it to all edges of the focused output.
    pub fn show(&mut self) {
        let Some(layer) = &self.layer_surface else {
            return;
        };
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
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
        self.indicator = None;
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

    /// Replace the grid with a small, animated indicator without changing
    /// the non-activating layer surface.
    pub fn show_indicator(&mut self, point: (i32, i32)) {
        self.state = None;
        self.indicator = Some(Indicator {
            point,
            started_at: Instant::now(),
        });
        self.needs_redraw = true;
    }

    pub fn update_indicator(&mut self, point: (i32, i32)) {
        if let Some(indicator) = &mut self.indicator {
            indicator.point = point;
            self.needs_redraw = true;
        }
    }

    pub fn has_indicator(&self) -> bool {
        self.indicator.is_some()
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
        let (buffer, canvas) =
            match pool.create_buffer(pw as i32, ph as i32, stride, wl_shm::Format::Argb8888) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("mousetrap: buffer creation failed: {e:?}");
                    return;
                }
            };
        self.buffer = Some(buffer);

        // SlotPool memory is reused and is not guaranteed to be initialized.
        // Always start from transparent black: blending over stale bytes can
        // expose contents from a previous frame or another shm allocation.
        canvas.fill(0);

        draw_grid(
            canvas,
            pw as usize,
            ph as usize,
            scale,
            self.state.as_ref(),
            self.indicator.as_ref(),
        );

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

// ---------------------------------------------------------------------------
// Manual rasterization (bounded loops only; can never spin).
// ---------------------------------------------------------------------------

/// Blend an unpremultiplied source color into a premultiplied ARGB8888
/// pixel. Wayland's ARGB8888 is a native-endian u32; on little-endian Linux
/// the bytes in memory are B, G, R, A (not A, R, G, B).
#[inline]
fn blend_pixel(dst: &mut [u8], r: u8, g: u8, b: u8, a: f32) {
    let a = (a.clamp(0.0, 1.0) * 255.0) as u32;
    let inv = 255 - a;
    let db = dst[0] as u32;
    let dg = dst[1] as u32;
    let dr = dst[2] as u32;
    let da = dst[3] as u32;
    dst[0] = ((b as u32 * a + db * inv) / 255) as u8;
    dst[1] = ((g as u32 * a + dg * inv) / 255) as u8;
    dst[2] = ((r as u32 * a + dr * inv) / 255) as u8;
    dst[3] = ((255 * a + da * inv) / 255) as u8;
}

/// Fill an axis-aligned rectangle (float coords), clipped to the canvas.
fn fill_rect(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: u8,
    g: u8,
    b: u8,
    a: f32,
) {
    if a <= 0.0 {
        return;
    }
    let x0 = x.floor().max(0.0) as usize;
    let y0 = y.floor().max(0.0) as usize;
    let x1 = ((x + w).ceil().max(0.0) as usize).min(width);
    let y1 = ((y + h).ceil().max(0.0) as usize).min(height);
    for py in y0..y1 {
        let row = py * width;
        for px in x0..x1 {
            blend_pixel(&mut canvas[(row + px) * 4..(row + px) * 4 + 4], r, g, b, a);
        }
    }
}

/// Stroke a rectangle by filling its four edges.
fn stroke_rect(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    thickness: f32,
    r: u8,
    g: u8,
    b: u8,
    a: f32,
) {
    if thickness <= 0.0 || w <= 0.0 || h <= 0.0 {
        return;
    }
    fill_rect(canvas, width, height, x, y, w, thickness.min(h), r, g, b, a); // top
    fill_rect(
        canvas,
        width,
        height,
        x,
        y + h - thickness,
        w,
        thickness.min(h),
        r,
        g,
        b,
        a,
    ); // bottom
    fill_rect(canvas, width, height, x, y, thickness.min(w), h, r, g, b, a); // left
    fill_rect(
        canvas,
        width,
        height,
        x + w - thickness,
        y,
        thickness.min(w),
        h,
        r,
        g,
        b,
        a,
    ); // right
}

fn draw_grid(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    scale: u32,
    state: Option<&SessionState>,
    indicator: Option<&Indicator>,
) {
    let s = scale as f32;
    let (w, h) = (width as f32, height as f32);

    if state.is_none() {
        if let Some(indicator) = indicator {
            draw_indicator(canvas, width, height, s, indicator);
        }
        return;
    }

    // Dim the whole output.
    fill_rect(
        canvas,
        width,
        height,
        0.0,
        0.0,
        w,
        h,
        0,
        0,
        0,
        OVERLAY_ALPHA,
    );

    let Some(state) = state else { return };
    let (bx, by, bw, bh) = state.current_bounds;
    // Clamp to sane, finite values inside the canvas.
    let bx = (bx as f32).clamp(-(w as f32), w as f32) * s;
    let by = (by as f32).clamp(-(h as f32), h as f32) * s;
    let bw = ((bw as f32).clamp(1.0, w as f32) * s).max(1.0);
    let bh = ((bh as f32).clamp(1.0, h as f32) * s).max(1.0);

    // Current region: subtle white fill + border.
    fill_rect(
        canvas,
        width,
        height,
        bx,
        by,
        bw,
        bh,
        255,
        255,
        255,
        20.0 / 255.0,
    );
    stroke_rect(
        canvas,
        width,
        height,
        bx,
        by,
        bw,
        bh,
        2.0 * s,
        255,
        255,
        255,
        115.0 / 255.0,
    );

    // Cells with key labels.
    let row_h = bh / ROWS.len() as f32;
    for (r, row) in ROWS.iter().enumerate() {
        let col_w = bw / row.chars().count() as f32;
        for (c, ch) in row.chars().enumerate() {
            let x = bx + c as f32 * col_w + CELL_PADDING * s;
            let y = by + r as f32 * row_h + CELL_PADDING * s;
            let cw = col_w - 2.0 * CELL_PADDING * s;
            let chh = row_h - 2.0 * CELL_PADDING * s;
            if cw <= 0.0 || chh <= 0.0 {
                continue;
            }
            fill_rect(
                canvas,
                width,
                height,
                x,
                y,
                cw,
                chh,
                255,
                255,
                255,
                31.0 / 255.0,
            );
            stroke_rect(
                canvas,
                width,
                height,
                x,
                y,
                cw,
                chh,
                1.0 * s,
                255,
                255,
                255,
                71.0 / 255.0,
            );
            let font_size = (10.0 * s).max(cw.min(chh) * 0.33);
            let label = ch.to_uppercase().to_string();
            draw_text(
                canvas,
                width,
                height,
                &label,
                font_size,
                x,
                y,
                cw,
                chh,
                255,
                255,
                255,
                242.0 / 255.0,
            );
        }
    }
}

fn draw_indicator(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    scale: f32,
    indicator: &Indicator,
) {
    let elapsed = indicator.started_at.elapsed().as_secs_f32();
    let pulse = 0.72 + 0.28 * ((elapsed * std::f32::consts::TAU / 1.1).sin() * 0.5 + 0.5);
    let x = indicator.point.0 as f32 * scale;
    let y = indicator.point.1 as f32 * scale;
    let cx = x + 22.0 * scale;
    let cy = y - 1.0 * scale;
    let unit = scale.max(1.0);
    let arm = 4.0 * unit;
    let core = 2.0 * unit;

    fill_rect(
        canvas,
        width,
        height,
        cx - core,
        cy - core,
        core * 2.0,
        core * 2.0,
        255,
        214,
        64,
        pulse,
    );
    fill_rect(
        canvas,
        width,
        height,
        cx - unit,
        cy - arm,
        unit * 2.0,
        arm - unit,
        255,
        214,
        64,
        pulse,
    );
    fill_rect(
        canvas,
        width,
        height,
        cx - unit,
        cy + unit,
        unit * 2.0,
        arm - unit,
        255,
        214,
        64,
        pulse,
    );
    fill_rect(
        canvas,
        width,
        height,
        cx - arm,
        cy - unit,
        arm - unit,
        unit * 2.0,
        255,
        214,
        64,
        pulse,
    );
    fill_rect(
        canvas,
        width,
        height,
        cx + unit,
        cy - unit,
        arm - unit,
        unit * 2.0,
        255,
        214,
        64,
        pulse,
    );
}

/// Rasterize `text` at position `cursor`/`baseline` (no centering).
fn blit_text(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    text: &str,
    size: f32,
    cursor: f32,
    baseline: f32,
    r: u8,
    g: u8,
    b: u8,
    a: f32,
) -> f32 {
    let font = font();
    let mut cursor = cursor;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        let gx = (cursor + metrics.xmin as f32) as i32;
        // fontdue's ymin is the bottom edge relative to the baseline, while
        // its bitmap rows run top-to-bottom. Convert that to a canvas top.
        let gy = (baseline - metrics.ymin as f32 - metrics.height as f32) as i32;
        for row in 0..metrics.height {
            let py = gy + row as i32;
            if py < 0 || py as usize >= height {
                continue;
            }
            for col in 0..metrics.width {
                let px = gx + col as i32;
                if px < 0 || px as usize >= width {
                    continue;
                }
                let coverage = bitmap[row * metrics.width + col] as f32 / 255.0;
                if coverage <= 0.0 {
                    continue;
                }
                let idx = (py as usize * width + px as usize) * 4;
                blend_pixel(&mut canvas[idx..idx + 4], r, g, b, a * coverage);
            }
        }
        cursor += metrics.advance_width;
    }
    cursor
}

fn draw_text(
    canvas: &mut [u8],
    width: usize,
    height: usize,
    text: &str,
    size: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: u8,
    g: u8,
    b: u8,
    a: f32,
) {
    let font = font();
    let mut total_width = 0.0;
    for ch in text.chars() {
        let (metrics, _) = font.rasterize(ch, size);
        total_width += metrics.advance_width;
    }
    let cursor = x + ((w - total_width) / 2.0).max(0.0);
    let baseline = y + h * 0.78;
    blit_text(
        canvas, width, height, text, size, cursor, baseline, r, g, b, a,
    );
}
