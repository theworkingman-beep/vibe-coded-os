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

pub use color::Color;
