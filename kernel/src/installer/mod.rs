//! GUI installer for Aperture OS.
//!
//! The installer is shipped with a pre-built disk image embedded in the live
//! ISO as a Limine module.  It lists detected ATA disks, lets the user pick
//! one, and writes the image to the target disk sector by sector while
//! updating a progress bar.

#![allow(static_mut_refs)]

use crate::arch::{mouse_buttons, mouse_position};
use crate::disk::{device_count, device_info, write_sectors};
use crate::gui::widgets::{Button, Label, ListBox, ProgressBar, Rect};
use crate::gui::{clear_window, create_window, draw_text, Color, WindowId, COMPOSITOR};

const BYTES_PER_WRITE: usize = 64 * 1024; // 128 sectors per chunk

static mut INSTALLER_WINDOW: Option<WindowId> = None;
static mut LAST_LOG_PCT: usize = 0;
static mut LIST_BOX: Option<ListBox> = None;
static mut INSTALL_BUTTON: Option<Button> = None;
static mut CANCEL_BUTTON: Option<Button> = None;
static mut STATUS_LABEL: Option<Label> = None;
static mut PROGRESS: Option<ProgressBar> = None;
static mut DISK_IMAGE: Option<(*const u8, usize)> = None;
static mut STATE: InstallerState = InstallerState::Idle;
static mut BUTTON_DOWN_LAST: bool = false;

#[derive(Clone, Copy)]
enum InstallerState {
    Idle,
    Copying { device: usize, done_bytes: usize },
    Done { device: usize },
    Error,
}

/// Store the Limine-loaded disk image that the installer will write.
pub fn set_image(ptr: *const u8, len: usize) {
    unsafe {
        DISK_IMAGE = Some((ptr, len));
    }
}

/// Create the installer window and populate the disk list.
pub fn open() {
    unsafe {
        if INSTALLER_WINDOW.is_some() {
            redraw();
            return;
        }
        let id = create_window("Install Aperture OS", 120, 80, 560, 360);
        INSTALLER_WINDOW = id;
        if id.is_none() {
            return;
        }

        LIST_BOX = Some(ListBox::new(Rect::new(20, 40, 420, 140)));
        let list = LIST_BOX.as_mut().unwrap();
        for i in 0..device_count() {
            if let Some(info) = device_info(i) {
                let mut label = [0u8; 96];
                let model = info.model();
                let size_mb = info.size_sectors * 512 / 1024 / 1024;
                let text = format_to(
                    &mut label,
                    &alloc::format!("{} - {} - {} MiB", i, model, size_mb),
                );
                list.add_item(text);
            }
        }
        if list.item_count == 0 {
            list.add_item("No disks detected");
        } else {
            list.select_first();
        }

        INSTALL_BUTTON = Some(Button::new(Rect::new(20, 280, 120, 28), "Install"));
        CANCEL_BUTTON = Some(Button::new(Rect::new(160, 280, 120, 28), "Close"));
        STATUS_LABEL = Some(Label::new(
            Rect::new(20, 240, 500, 20),
            "Select a target disk.",
            Color::WHITE,
        ));
        PROGRESS = Some(ProgressBar::new(Rect::new(20, 210, 520, 20)));

        redraw();
    }
}

/// Update mouse interaction and advance any in-progress copy.
pub fn update() {
    unsafe {
        let Some(id) = INSTALLER_WINDOW else { return };
        let (mx, my) = mouse_position();
        let down = (mouse_buttons() & 1) != 0;

        if let Some(list) = LIST_BOX.as_mut() {
            list.update(mx, my, down);
        }
        if let Some(btn) = INSTALL_BUTTON.as_mut() {
            btn.update(mx, my, down);
            if btn.take_clicked() {
                start_install();
            }
        }
        if let Some(btn) = CANCEL_BUTTON.as_mut() {
            btn.update(mx, my, down);
            if btn.take_clicked() {
                close();
                return;
            }
        }
        BUTTON_DOWN_LAST = down;

        // Advance the copy if busy.  Write several chunks per frame so a
        // full disk image finishes in a few seconds while still refreshing
        // the progress bar.
        if let InstallerState::Copying { device, done_bytes } = STATE {
            let mut done = done_bytes;
            for _ in 0..32 {
                if !matches!(STATE, InstallerState::Copying { .. }) {
                    break;
                }
                done = copy_chunk(device, done);
            }
        }

        // Redraw window contents.  Only request a full framebuffer composite
        // every 5% of progress (or on state transitions) because the debug
        // build's per-pixel render is very slow.
        draw_frame(id);
        let pct = PROGRESS
            .as_ref()
            .map(|p| (p.progress * 100.0) as usize)
            .unwrap_or(0);
        static mut LAST_RENDER_PCT: usize = 101;
        if LAST_RENDER_PCT == 101 || pct >= LAST_RENDER_PCT + 5 || pct == 100 {
            LAST_RENDER_PCT = pct;
            crate::gui::request_render();
        }
    }
}

/// Return true if the installer window is currently open.
pub fn is_open() -> bool {
    unsafe { INSTALLER_WINDOW.is_some() }
}

/// Return true if a disk image has been supplied via Limine.
pub fn has_image() -> bool {
    unsafe { DISK_IMAGE.is_some() }
}

/// Handle a keyboard event while the installer is open.
/// Arrow keys / j,k move the selection; Enter starts the install; q closes.
pub fn handle_key(ch: char) {
    unsafe {
        let Some(_id) = INSTALLER_WINDOW else { return };
        match ch {
            '\n' | '\r' => {
                start_install();
            }
            'j' | 'J' => {
                if let Some(list) = LIST_BOX.as_mut() {
                    list.select_next();
                }
            }
            'k' | 'K' => {
                if let Some(list) = LIST_BOX.as_mut() {
                    list.select_prev();
                }
            }
            'q' | 'Q' => {
                close();
                crate::gui::request_render();
            }
            _ => {}
        }
        redraw();
    }
}

fn close() {
    unsafe {
        INSTALLER_WINDOW = None;
        LIST_BOX = None;
        INSTALL_BUTTON = None;
        CANCEL_BUTTON = None;
        STATUS_LABEL = None;
        PROGRESS = None;
        STATE = InstallerState::Idle;
    }
}

fn start_install() {
    unsafe {
        let Some(list) = LIST_BOX.as_ref() else {
            return;
        };
        let Some((ptr, len)) = DISK_IMAGE else {
            set_status("No disk image available.");
            STATE = InstallerState::Error;
            crate::gui::request_render();
            return;
        };
        let Some(device) = list.selected() else {
            set_status("Please select a disk first.");
            crate::gui::request_render();
            return;
        };
        let Some(info) = device_info(device) else {
            return;
        };
        let image_sectors = len / 512;
        if info.size_sectors < image_sectors as u64 {
            set_status("Target disk is too small.");
            STATE = InstallerState::Error;
            crate::gui::request_render();
            return;
        }
        if let Some(btn) = INSTALL_BUTTON.as_mut() {
            btn.enabled = false;
        }
        crate::logln!(
            "installer: starting write to device {} ({} bytes)",
            device,
            len
        );
        STATE = InstallerState::Copying {
            device,
            done_bytes: 0,
        };
        crate::gui::request_render();
        let _ = (ptr, len);
    }
}

fn copy_chunk(device: usize, done_bytes: usize) -> usize {
    unsafe {
        let Some((ptr, total)) = DISK_IMAGE else {
            return done_bytes;
        };
        let remaining = total - done_bytes;
        if remaining == 0 {
            STATE = InstallerState::Done { device };
            set_status("Installation complete.");
            crate::logln!("installer: complete");
            crate::gui::request_render();
            return done_bytes;
        }
        let chunk = remaining.min(BYTES_PER_WRITE);
        let src = core::slice::from_raw_parts(ptr.add(done_bytes), chunk);
        let lba = (done_bytes / 512) as u64;
        match write_sectors(device, lba, src) {
            Some(written) if written == chunk => {
                let new_done = done_bytes + written;
                STATE = InstallerState::Copying {
                    device,
                    done_bytes: new_done,
                };
                if let Some(p) = PROGRESS.as_mut() {
                    p.progress = new_done as f32 / total as f32;
                }
                let pct = (new_done * 100 / total) as usize;
                let mut buf = [0u8; 32];
                set_status_str(format_to(&mut buf, &alloc::format!("{}% written", pct)));
                if pct >= LAST_LOG_PCT + 10 {
                    crate::logln!("installer: {}%", pct);
                    LAST_LOG_PCT = pct;
                }
                return new_done;
            }
            _ => {
                STATE = InstallerState::Error;
                set_status("Disk write failed.");
                crate::logln!("installer: write failed at LBA {}", lba);
                crate::gui::request_render();
                return done_bytes;
            }
        }
    }
}

fn draw_frame(id: WindowId) {
    clear_window(Some(id), Color::new(0x30, 0x30, 0x30));
    draw_text(Some(id), "Install Aperture OS", 20, 14, Color::WHITE);

    unsafe {
        let mut guard = COMPOSITOR.lock();
        let Some(c) = guard.as_mut() else { return };
        let Some(window) = c.window_mut(id) else {
            return;
        };

        if let Some(list) = LIST_BOX.as_ref() {
            list.draw(window);
        }
        if let Some(p) = PROGRESS.as_ref() {
            p.draw(window);
        }
        if let Some(lbl) = STATUS_LABEL.as_ref() {
            lbl.draw(window);
        }
        if let Some(btn) = INSTALL_BUTTON.as_ref() {
            btn.draw(window);
        }
        if let Some(btn) = CANCEL_BUTTON.as_ref() {
            btn.draw(window);
        }
    }
}

fn redraw() {
    unsafe {
        if let Some(id) = INSTALLER_WINDOW {
            draw_frame(id);
            crate::gui::request_render();
        }
    }
}

fn set_status(text: &str) {
    unsafe {
        if let Some(lbl) = STATUS_LABEL.as_mut() {
            lbl.set_text(text);
        }
    }
}

fn set_status_str(text: &str) {
    set_status(text);
}

/// Render `format!` output into a fixed-size stack buffer and return a `&str`.
fn format_to<'a>(buf: &'a mut [u8], s: &str) -> &'a str {
    let len = s.len().min(buf.len());
    buf[..len].copy_from_slice(&s.as_bytes()[..len]);
    core::str::from_utf8(&buf[..len]).unwrap_or("")
}
