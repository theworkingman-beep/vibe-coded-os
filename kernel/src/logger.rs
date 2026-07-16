//! Minimal serial logger with an in-memory ring buffer.

extern crate alloc;

use core::fmt::{self, Write};
use spin::Mutex;

/// Size of the kernel log ring buffer in bytes.
const RING_SIZE: usize = 64 * 1024;

/// Writer that mirrors output to both the serial console and the ring buffer.
struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut ring = RING_BUFFER.lock();
        for byte in s.bytes() {
            crate::arch::debug_putchar(byte);
            ring.push(byte);
        }
        Ok(())
    }
}

/// Writer that only emits to the serial console, used by the dump path so the
/// ring buffer contents are not re-captured while being printed.
struct RawSerialWriter;

impl Write for RawSerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            crate::arch::debug_putchar(byte);
        }
        Ok(())
    }
}

struct RingBuffer {
    buf: [u8; RING_SIZE],
    head: usize,
    len: usize,
}

impl RingBuffer {
    const fn new() -> Self {
        Self {
            buf: [0; RING_SIZE],
            head: 0,
            len: 0,
        }
    }

    fn push(&mut self, byte: u8) {
        let pos = (self.head + self.len) % RING_SIZE;
        self.buf[pos] = byte;
        if self.len < RING_SIZE {
            self.len += 1;
        } else {
            self.head = (self.head + 1) % RING_SIZE;
        }
    }

    /// Write the buffered contents to `writer` in chronological order.
    fn dump(&self, writer: &mut dyn fmt::Write) {
        for i in 0..self.len {
            let byte = self.buf[(self.head + i) % RING_SIZE];
            // The ring buffer stores raw bytes; render as Latin-1-ish chars for
            // diagnostics.  Non-printable bytes are written as-is to preserve
            // the original log stream.
            let _ = writer.write_char(char::from(byte));
        }
    }
}

static LOGGER: Mutex<Option<SerialWriter>> = Mutex::new(None);
static RING_BUFFER: Mutex<RingBuffer> = Mutex::new(RingBuffer::new());

pub fn init() {
    *LOGGER.lock() = Some(SerialWriter);
}

pub fn _print(args: fmt::Arguments) {
    if let Some(writer) = LOGGER.lock().as_mut() {
        let _ = writer.write_fmt(args);
    }
}

/// Dump the contents of the kernel log ring buffer to the serial console.
pub fn dump_ring_buffer() {
    let mut writer = RawSerialWriter;
    let _ = writer.write_str("--- kernel log ring buffer ---\n");
    RING_BUFFER.lock().dump(&mut writer);
    let _ = writer.write_str("\n--- end ring buffer ---\n");
}

/// Invoke `f` with the ring buffer contents as a contiguous byte slice.
///
/// The callback receives the most recent `max_bytes` of the log (up to the
/// full ring buffer size).  Non-UTF-8 bytes are preserved as Latin-1.
pub fn with_contents(max_bytes: usize, f: impl FnOnce(&[u8])) {
    let ring = RING_BUFFER.lock();
    let take = ring.len.min(max_bytes);
    let mut temp = alloc::vec::Vec::with_capacity(take);
    for i in 0..take {
        temp.push(ring.buf[(ring.head + ring.len - take + i) % RING_SIZE]);
    }
    f(&temp);
}

/// Return the most recent `max_lines` of the log as a `String`.
#[allow(dead_code)]
pub fn recent_lines(max_lines: usize, max_bytes: usize) -> alloc::string::String {
    let mut text = alloc::string::String::new();
    with_contents(max_bytes, |bytes| {
        let s = alloc::string::String::from_utf8_lossy(bytes);
        let lines: alloc::vec::Vec<&str> = s.lines().collect();
        let start = lines.len().saturating_sub(max_lines);
        for line in &lines[start..] {
            text.push_str(line);
            text.push('\n');
        }
    });
    text
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::logger::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! logln {
    () => ($crate::log!("\n"));
    ($fmt:expr) => ($crate::log!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::log!(concat!($fmt, "\n"), $($arg)*));
}
