//! GUI subsystem.
//!
//! A simple software-rendered compositor. In the future this will support
//! hardware acceleration and a GPU-driven scene graph.

use crate::boot_info::FrameBufferInfo;
use spin::Mutex;

pub mod color;
pub mod compositor;
pub mod cursor;
pub mod desktop;
pub mod font;
pub mod widgets;

pub(crate) use compositor::Compositor;
pub use compositor::WindowId;

pub(crate) static COMPOSITOR: Mutex<Option<Compositor>> = Mutex::new(None);

/// Initialize the GUI with the bootloader-provided framebuffer.
pub fn init() {
    // The framebuffer pointer/info is passed from kernel_main at startup.
}

/// Set up the compositor once the framebuffer is known.
pub fn init_compositor(buffer: &'static mut [u8], info: FrameBufferInfo) {
    *COMPOSITOR.lock() = Some(Compositor::new(buffer, info));
    request_render();
}

static mut NEEDS_RENDER: bool = true;

/// Mark the framebuffer as dirty so the scene is redrawn on the next frame.
pub fn request_render() {
    unsafe {
        NEEDS_RENDER = true;
    }
}

/// Clear the render demand flag.  Called by the main loop after rendering.
pub fn clear_render_request() {
    unsafe {
        NEEDS_RENDER = false;
    }
}

/// Return true if the compositor has been asked to redraw.
pub fn needs_render() -> bool {
    unsafe { NEEDS_RENDER }
}

/// Render the current scene to the framebuffer, including the mouse cursor.
pub fn render() {
    if let Some(c) = COMPOSITOR.lock().as_mut() {
        // Large framebuffer fills are not interrupt-safe in the current debug
        // build; disable interrupts for the duration of the render to avoid
        // memory corruption from reentrant slice operations.
        crate::arch::without_interrupts(|| {
            c.render();
        });
        let (mx, my) = crate::arch::mouse_position();
        cursor::draw_cursor(c, mx, my);
    }
}

/// Create a new window and return its handle.
pub fn create_window(title: &str, x: i32, y: i32, width: i32, height: i32) -> Option<WindowId> {
    COMPOSITOR
        .lock()
        .as_mut()
        .map(|c| c.create_window(title, x, y, width, height))
}

/// Draw text onto the window identified by `id`.
pub fn draw_text(id: Option<WindowId>, text: &str, x: i32, y: i32, color: Color) {
    let Some(id) = id else { return };
    if let Some(c) = COMPOSITOR.lock().as_mut() {
        if let Some(window) = c.window_mut(id) {
            font::draw_text(window, text, x, y, color);
        }
    }
}

/// Fill the window backbuffer with `color`.
pub fn clear_window(id: Option<WindowId>, color: Color) {
    let Some(id) = id else { return };
    if let Some(c) = COMPOSITOR.lock().as_mut() {
        c.clear_window(id, color);
    }
}

// --- GDI drawing primitives (window-relative) -----------------------------

pub fn gdi_set_pixel(id: WindowId, x: i32, y: i32, color: Color) {
    if let Some(c) = COMPOSITOR.lock().as_mut() {
        c.window_set_pixel(id, x, y, color);
    }
}

pub fn gdi_draw_line(id: WindowId, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
    if let Some(c) = COMPOSITOR.lock().as_mut() {
        c.window_draw_line(id, x0, y0, x1, y1, color);
    }
}

pub fn gdi_fill_rect(id: WindowId, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
    if let Some(c) = COMPOSITOR.lock().as_mut() {
        c.window_fill_rect(id, x0, y0, x1, y1, color);
    }
}

pub fn gdi_draw_rect(id: WindowId, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
    if let Some(c) = COMPOSITOR.lock().as_mut() {
        c.window_draw_rect(id, x0, y0, x1, y1, color);
    }
}

pub fn gdi_draw_ellipse(id: WindowId, cx: i32, cy: i32, rx: i32, ry: i32, color: Color) {
    if let Some(c) = COMPOSITOR.lock().as_mut() {
        c.window_draw_ellipse(id, cx, cy, rx, ry, color);
    }
}

/// Phase 6 self-test: create a small window and render GDI primitives into it
/// (filled rect, rectangle outline, line, ellipse, and a label). Returns
/// `true` if the window was created and drawn. Verified by the boot log line
/// and the visible shapes on the desktop.
pub fn gdi_self_test() -> bool {
    use color::Color;
    let Some(id) = create_window("GDI", 40, 40, 160, 120) else {
        crate::logln!("gdi: self_test FAIL no window");
        return false;
    };
    clear_window(Some(id), Color::BLACK);
    gdi_fill_rect(id, 4, 4, 60, 40, Color::new(0x20, 0x40, 0xA0));
    gdi_draw_rect(id, 70, 4, 150, 40, Color::new(0xA0, 0xC0, 0x20));
    gdi_draw_line(id, 4, 50, 150, 110, Color::new(0xE0, 0xE0, 0x20));
    gdi_draw_ellipse(id, 110, 80, 40, 30, Color::new(0xE0, 0x40, 0x40));
    draw_text(Some(id), "GDI", 6, 100, Color::WHITE);
    crate::logln!("gdi: self_test OK (window {} drawn)", id.0);
    true
}

pub use color::Color;
