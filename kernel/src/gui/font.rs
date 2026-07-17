//! Minimal 5x7 bitmap font for the GUI.
//!
//! Includes digits 0-9, uppercase A-Z, space, and a few punctuation marks.
//! Each glyph is stored as 7 bytes; the low 5 bits of each byte encode one
//! row from top to bottom.

use super::color::Color;
use super::compositor::Window;
use crate::boot_info::{FrameBufferInfo, PixelFormat};

const GLYPH_WIDTH: i32 = 5;
const GLYPH_HEIGHT: i32 = 7;
const GLYPH_BYTES: usize = GLYPH_HEIGHT as usize;

struct FontGlyph {
    ch: char,
    rows: [u8; GLYPH_BYTES],
}

const FONT: &[FontGlyph] = &[
    FontGlyph {
        ch: '0',
        rows: [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
    },
    FontGlyph {
        ch: '1',
        rows: [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
    },
    FontGlyph {
        ch: '2',
        rows: [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
    },
    FontGlyph {
        ch: '3',
        rows: [0x0E, 0x11, 0x01, 0x06, 0x01, 0x11, 0x0E],
    },
    FontGlyph {
        ch: '4',
        rows: [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
    },
    FontGlyph {
        ch: '5',
        rows: [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
    },
    FontGlyph {
        ch: '6',
        rows: [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
    },
    FontGlyph {
        ch: '7',
        rows: [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
    },
    FontGlyph {
        ch: '8',
        rows: [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
    },
    FontGlyph {
        ch: '9',
        rows: [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
    },
    FontGlyph {
        ch: 'A',
        rows: [0x04, 0x0A, 0x11, 0x11, 0x1F, 0x11, 0x11],
    },
    FontGlyph {
        ch: 'B',
        rows: [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
    },
    FontGlyph {
        ch: 'C',
        rows: [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
    },
    FontGlyph {
        ch: 'D',
        rows: [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
    },
    FontGlyph {
        ch: 'E',
        rows: [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
    },
    FontGlyph {
        ch: 'F',
        rows: [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
    },
    FontGlyph {
        ch: 'G',
        rows: [0x0E, 0x11, 0x10, 0x13, 0x11, 0x11, 0x0E],
    },
    FontGlyph {
        ch: 'H',
        rows: [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
    },
    FontGlyph {
        ch: 'I',
        rows: [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
    },
    FontGlyph {
        ch: 'J',
        rows: [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E],
    },
    FontGlyph {
        ch: 'K',
        rows: [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
    },
    FontGlyph {
        ch: 'L',
        rows: [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
    },
    FontGlyph {
        ch: 'M',
        rows: [0x11, 0x1B, 0x15, 0x11, 0x11, 0x11, 0x11],
    },
    FontGlyph {
        ch: 'N',
        rows: [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
    },
    FontGlyph {
        ch: 'O',
        rows: [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
    },
    FontGlyph {
        ch: 'P',
        rows: [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
    },
    FontGlyph {
        ch: 'Q',
        rows: [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
    },
    FontGlyph {
        ch: 'R',
        rows: [0x1E, 0x11, 0x11, 0x1E, 0x12, 0x11, 0x11],
    },
    FontGlyph {
        ch: 'S',
        rows: [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
    },
    FontGlyph {
        ch: 'T',
        rows: [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
    },
    FontGlyph {
        ch: 'U',
        rows: [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
    },
    FontGlyph {
        ch: 'V',
        rows: [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
    },
    FontGlyph {
        ch: 'W',
        rows: [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
    },
    FontGlyph {
        ch: 'X',
        rows: [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
    },
    FontGlyph {
        ch: 'Y',
        rows: [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
    },
    FontGlyph {
        ch: 'Z',
        rows: [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
    },
    FontGlyph {
        ch: ' ',
        rows: [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    },
    FontGlyph {
        ch: '.',
        rows: [0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x06],
    },
    FontGlyph {
        ch: ',',
        rows: [0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x04],
    },
    FontGlyph {
        ch: ':',
        rows: [0x00, 0x06, 0x06, 0x00, 0x06, 0x06, 0x00],
    },
    FontGlyph {
        ch: '-',
        rows: [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
    },
    FontGlyph {
        ch: '!',
        rows: [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
    },
    FontGlyph {
        ch: '?',
        rows: [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
    },
];

fn glyph_for(c: char) -> &'static [u8; GLYPH_BYTES] {
    for glyph in FONT {
        if glyph.ch == c {
            return &glyph.rows;
        }
    }
    // Unknown characters render as space.
    &FONT[FONT.len() - 6].rows
}

/// Draw `text` onto `window` at `(x, y)` in `color`.
pub fn draw_text(window: &mut Window, text: &str, x: i32, y: i32, color: Color) {
    let backbuffer =
        unsafe { core::slice::from_raw_parts_mut(window.backbuffer, window.pixel_count) };
    let mut cursor_x = x;
    for c in text.chars() {
        if cursor_x + GLYPH_WIDTH > window.width {
            break;
        }
        let rows = glyph_for(c);
        for row in 0..GLYPH_HEIGHT {
            let row_bits = rows[row as usize];
            for col in 0..GLYPH_WIDTH {
                if (row_bits >> (GLYPH_WIDTH - 1 - col)) & 1 != 0 {
                    let px = cursor_x + col;
                    let py = y + row;
                    if px >= 0 && px < window.width && py >= 0 && py < window.height {
                        let index = (py * window.width + px) as usize;
                        backbuffer[index] = color;
                    }
                }
            }
        }
        cursor_x += GLYPH_WIDTH + 1;
    }
}

/// Encode a `Color` for the framebuffer's native byte layout.
fn encode_color(info: &FrameBufferInfo, color: Color) -> [u8; 4] {
    match info.pixel_format {
        PixelFormat::Rgb => [color.r, color.g, color.b, 0],
        PixelFormat::Bgr => [color.b, color.g, color.r, 0],
        PixelFormat::U8 => [color.r, 0, 0, 0],
        PixelFormat::Unknown {
            red_position,
            green_position,
            blue_position,
        } => {
            let mut pixel = [0u8; 4];
            pixel[(red_position as usize / 8).min(3)] = color.r;
            pixel[(green_position as usize / 8).min(3)] = color.g;
            pixel[(blue_position as usize / 8).min(3)] = color.b;
            pixel
        }
    }
}

/// Draw `text` directly onto the physical framebuffer at `(x, y)` in `color`.
///
/// This is intended for emergency diagnostic output (e.g. the panic handler)
/// when a full compositor/window stack may not be available.
pub unsafe fn draw_text_framebuffer(
    buffer: &mut [u8],
    info: FrameBufferInfo,
    text: &str,
    mut x: i32,
    y: i32,
    color: Color,
) {
    let bytes_per_pixel = info.bytes_per_pixel as usize;
    let stride = info.stride;
    let encoded = encode_color(&info, color);

    for c in text.chars() {
        if x + GLYPH_WIDTH > info.width as i32 {
            break;
        }
        let rows = glyph_for(c);
        for row in 0..GLYPH_HEIGHT {
            let row_bits = rows[row as usize];
            let py = y + row;
            if py < 0 || py >= info.height as i32 {
                continue;
            }
            for col in 0..GLYPH_WIDTH {
                if (row_bits >> (GLYPH_WIDTH - 1 - col)) & 1 != 0 {
                    let px = x + col;
                    if px < 0 || px >= info.width as i32 {
                        continue;
                    }
                    let offset = (py as usize * stride) + (px as usize * bytes_per_pixel);
                    buffer[offset..offset + bytes_per_pixel]
                        .copy_from_slice(&encoded[..bytes_per_pixel]);
                }
            }
        }
        x += GLYPH_WIDTH + 1;
    }
}
