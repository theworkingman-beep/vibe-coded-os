//! Panic handler with optional framebuffer output.

use core::fmt::{self, Write};
use core::panic::PanicInfo;
use spin::Mutex;

use crate::boot_info::FrameBufferInfo;
use crate::gui::color::Color;

struct PanicFramebuffer {
    ptr: *mut u8,
    len: usize,
    info: FrameBufferInfo,
}

unsafe impl Send for PanicFramebuffer {}
unsafe impl Sync for PanicFramebuffer {}

static PANIC_FB: Mutex<Option<PanicFramebuffer>> = Mutex::new(None);

/// Register the physical framebuffer for use by the panic handler.
///
/// # Safety
///
/// The caller must ensure `ptr` points to a valid, writable framebuffer of
/// `len` bytes described by `info` for the lifetime of the kernel.
pub unsafe fn register_framebuffer(ptr: *mut u8, len: usize, info: FrameBufferInfo) {
    *PANIC_FB.lock() = Some(PanicFramebuffer { ptr, len, info });
}

/// Fixed-size formatter used by the panic handler so it never allocates.
struct PanicFormatter {
    buf: [u8; 256],
    pos: usize,
}

impl PanicFormatter {
    fn new() -> Self {
        Self {
            buf: [0; 256],
            pos: 0,
        }
    }

    fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.pos]) }
    }
}

impl Write for PanicFormatter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let take = bytes.len().min(self.buf.len() - self.pos);
        self.buf[self.pos..self.pos + take].copy_from_slice(&bytes[..take]);
        self.pos += take;
        Ok(())
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::logln!("PANIC: {}", info);

    if let Some(fb) = PANIC_FB.lock().as_ref() {
        let mut fmt = PanicFormatter::new();
        let _ = fmt.write_fmt(format_args!("{}", info));
        let message = fmt.as_str();

        let buffer = unsafe { core::slice::from_raw_parts_mut(fb.ptr, fb.len) };
        unsafe {
            crate::gui::font::draw_text_framebuffer(
                buffer,
                fb.info,
                "ApertureOS Kernel Panic",
                16,
                16,
                Color::WHITE,
            );
            crate::gui::font::draw_text_framebuffer(
                buffer,
                fb.info,
                message,
                16,
                32,
                Color::new(255, 128, 128),
            );
        }
    }

    crate::hlt();
}
