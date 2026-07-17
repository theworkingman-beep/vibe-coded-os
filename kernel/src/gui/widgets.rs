//! Simple widget toolkit for Aperture OS.
//!
//! Provides buttons, labels, a list box, and a progress bar.  Widgets draw into
//! a window backbuffer and are driven by raw mouse/keyboard events from the
//! desktop loop.

use super::color::Color;
use super::compositor::Window;
use super::font;

/// 2-D rectangle used for hit testing and drawing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// True if the point lies inside the rectangle.
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

/// Fill a rectangle with `color`.
pub fn fill_rect(window: &mut Window, rect: Rect, color: Color) {
    let x0 = rect.x.max(0);
    let y0 = rect.y.max(0);
    let x1 = (rect.x + rect.width).min(window.width);
    let y1 = (rect.y + rect.height).min(window.height);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let pixels = unsafe { core::slice::from_raw_parts_mut(window.backbuffer, window.pixel_count) };
    for y in y0..y1 {
        let row_start = (y * window.width) as usize;
        for x in x0..x1 {
            pixels[row_start + x as usize] = color;
        }
    }
}

/// Draw a 1-pixel border with `color`.
pub fn draw_rect(window: &mut Window, rect: Rect, color: Color) {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.width - 1;
    let y1 = rect.y + rect.height - 1;
    for x in x0..=x1 {
        write_pixel(window, x, y0, color);
        write_pixel(window, x, y1, color);
    }
    for y in y0..=y1 {
        write_pixel(window, x0, y, color);
        write_pixel(window, x1, y, color);
    }
}

/// Write a single pixel into `window` if it is within bounds.
pub fn write_pixel(window: &mut Window, x: i32, y: i32, color: Color) {
    if x < 0 || y < 0 || x >= window.width || y >= window.height {
        return;
    }
    let pixels = unsafe { core::slice::from_raw_parts_mut(window.backbuffer, window.pixel_count) };
    pixels[(y * window.width + x) as usize] = color;
}

/// A clickable button.
pub struct Button {
    pub rect: Rect,
    pub label: [u8; 64],
    pub label_len: usize,
    pub enabled: bool,
    pressed: bool,
    clicked: bool,
}

impl Button {
    pub fn new(rect: Rect, label: &str) -> Self {
        let mut buf = [0u8; 64];
        let len = label.len().min(63);
        buf[..len].copy_from_slice(&label.as_bytes()[..len]);
        Self {
            rect,
            label: buf,
            label_len: len,
            enabled: true,
            pressed: false,
            clicked: false,
        }
    }

    pub fn label(&self) -> &str {
        core::str::from_utf8(&self.label[..self.label_len]).unwrap_or("")
    }

    /// Draw the button into `window`.
    pub fn draw(&self, window: &mut Window) {
        let bg = if self.enabled {
            if self.pressed {
                Color::new(0x60, 0x60, 0x60)
            } else {
                Color::new(0x40, 0x40, 0x40)
            }
        } else {
            Color::new(0x28, 0x28, 0x28)
        };
        fill_rect(window, self.rect, bg);
        draw_rect(
            window,
            self.rect,
            if self.enabled {
                Color::WHITE
            } else {
                Color::GRAY
            },
        );
        let text_color = if self.enabled {
            Color::WHITE
        } else {
            Color::GRAY
        };
        let text_x = self.rect.x + (self.rect.width - (self.label_len as i32 * 6)) / 2;
        let text_y = self.rect.y + (self.rect.height - 7) / 2;
        font::draw_text(window, self.label(), text_x, text_y, text_color);
    }

    /// Call on mouse move/update.  `down` is the current left-button state.
    pub fn update(&mut self, x: i32, y: i32, down: bool) {
        if !self.enabled {
            self.pressed = false;
            return;
        }
        let inside = self.rect.contains(x, y);
        if down {
            if inside {
                self.pressed = true;
            }
        } else if self.pressed {
            self.pressed = false;
            if inside {
                self.clicked = true;
            }
        }
    }

    /// Returns true once if the button was clicked and releases the flag.
    pub fn take_clicked(&mut self) -> bool {
        core::mem::take(&mut self.clicked)
    }
}

/// Static text label.
pub struct Label {
    pub rect: Rect,
    pub text: [u8; 128],
    pub text_len: usize,
    pub color: Color,
}

impl Label {
    pub fn new(rect: Rect, text: &str, color: Color) -> Self {
        let mut buf = [0u8; 128];
        let len = text.len().min(127);
        buf[..len].copy_from_slice(&text.as_bytes()[..len]);
        Self {
            rect,
            text: buf,
            text_len: len,
            color,
        }
    }

    pub fn text(&self) -> &str {
        core::str::from_utf8(&self.text[..self.text_len]).unwrap_or("")
    }

    pub fn draw(&self, window: &mut Window) {
        font::draw_text(window, self.text(), self.rect.x, self.rect.y, self.color);
    }

    /// Replace the displayed text.
    pub fn set_text(&mut self, text: &str) {
        let mut buf = [0u8; 128];
        let len = text.len().min(127);
        buf[..len].copy_from_slice(&text.as_bytes()[..len]);
        self.text = buf;
        self.text_len = len;
    }
}

/// Vertical list box with selectable string items.
pub struct ListBox {
    pub rect: Rect,
    pub items: [([u8; 64], usize); 16],
    pub item_count: usize,
    selected: Option<usize>,
    down_last: bool,
}

impl ListBox {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            items: [([0u8; 64], 0); 16],
            item_count: 0,
            selected: None,
            down_last: false,
        }
    }

    pub fn add_item(&mut self, text: &str) {
        if self.item_count >= self.items.len() {
            return;
        }
        let i = self.item_count;
        let len = text.len().min(63);
        self.items[i].0[..len].copy_from_slice(text.as_bytes());
        self.items[i].1 = len;
        self.item_count += 1;
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn select_first(&mut self) {
        if self.item_count > 0 {
            self.selected = Some(0);
        }
    }

    pub fn select_next(&mut self) {
        if self.item_count == 0 {
            return;
        }
        let next = self
            .selected
            .map(|s| (s + 1) % self.item_count)
            .unwrap_or(0);
        self.selected = Some(next);
    }

    pub fn select_prev(&mut self) {
        if self.item_count == 0 {
            return;
        }
        let prev = self
            .selected
            .map(|s| if s == 0 { self.item_count - 1 } else { s - 1 })
            .unwrap_or(0);
        self.selected = Some(prev);
    }

    pub fn item_text(&self, i: usize) -> &str {
        if i >= self.item_count {
            return "";
        }
        core::str::from_utf8(&self.items[i].0[..self.items[i].1]).unwrap_or("")
    }

    /// Height of one item in pixels (font height + padding).
    const ITEM_HEIGHT: i32 = 12;

    pub fn draw(&self, window: &mut Window) {
        fill_rect(window, self.rect, Color::new(0x20, 0x20, 0x20));
        draw_rect(window, self.rect, Color::WHITE);
        for i in 0..self.item_count {
            let y = self.rect.y + 2 + i as i32 * Self::ITEM_HEIGHT;
            if y + 10 >= self.rect.y + self.rect.height {
                break;
            }
            if Some(i) == self.selected {
                fill_rect(
                    window,
                    Rect::new(
                        self.rect.x + 1,
                        y - 1,
                        self.rect.width - 2,
                        Self::ITEM_HEIGHT,
                    ),
                    Color::new(0x00, 0x60, 0xC0),
                );
            }
            font::draw_text(window, self.item_text(i), self.rect.x + 4, y, Color::WHITE);
        }
    }

    pub fn update(&mut self, x: i32, y: i32, down: bool) {
        if down && !self.down_last && self.rect.contains(x, y) {
            let rel = y - self.rect.y - 2;
            if rel >= 0 {
                let idx = rel / Self::ITEM_HEIGHT;
                if (idx as usize) < self.item_count {
                    self.selected = Some(idx as usize);
                }
            }
        }
        self.down_last = down;
    }
}

/// Horizontal progress bar (0.0 ..= 1.0).
pub struct ProgressBar {
    pub rect: Rect,
    pub progress: f32,
}

impl ProgressBar {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            progress: 0.0,
        }
    }

    pub fn draw(&self, window: &mut Window) {
        fill_rect(window, self.rect, Color::new(0x10, 0x10, 0x10));
        draw_rect(window, self.rect, Color::WHITE);
        let fill_width = ((self.rect.width - 4) as f32 * self.progress.clamp(0.0, 1.0)) as i32;
        if fill_width > 0 {
            fill_rect(
                window,
                Rect::new(
                    self.rect.x + 2,
                    self.rect.y + 2,
                    fill_width,
                    self.rect.height - 4,
                ),
                Color::new(0x00, 0x80, 0x00),
            );
        }
    }
}
