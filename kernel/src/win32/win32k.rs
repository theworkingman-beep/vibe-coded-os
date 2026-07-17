//! Win32 subsystem (win32k) server.
//!
//! Bridges the NT kernel to the Aperture OS GUI compositor. Each Win32 desktop
//! maps to a compositor window tree; window messages are dispatched here.

use crate::gui::{create_window, WindowId};
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

/// A Win32 desktop maps to a root compositor window.
#[derive(Clone, Copy)]
pub struct Desktop {
    pub name: [u8; 32],
    pub root: WindowId,
}

const MAX_DESKTOPS: usize = 16;
static DESKTOPS: Mutex<[Option<Desktop>; MAX_DESKTOPS]> =
    Mutex::new([const { None }; MAX_DESKTOPS]);

pub fn init() {
    // The default interactive desktop is created on demand.
}

/// Create a new Win32 desktop.
pub fn create_desktop(name: &str, width: i32, height: i32) -> Option<Desktop> {
    let root = create_window(name, 0, 0, width, height)?;
    let mut desktop_name = [0u8; 32];
    let len = name.len().min(31);
    desktop_name[..len].copy_from_slice(&name.as_bytes()[..len]);

    let desktop = Desktop {
        name: desktop_name,
        root,
    };

    let mut desktops = DESKTOPS.lock();
    let slot = desktops.iter_mut().find(|d| d.is_none())?;
    *slot = Some(desktop);
    Some(desktop)
}

// --- Win32 window-manager model -------------------------------------------
//
// A minimal `WindowClass` / `Wnd` / message-queue model mirroring the Win32
// `RegisterClass` -> `CreateWindowEx` -> `GetMessage`/`DispatchMessage` loop.
// Each `Wnd` owns a compositor window (its HWND backbuffer) and a FIFO
// message queue. `DefWindowProcW` provides default handling.

/// A registered window class (Win32 `WNDCLASSW`).
#[derive(Clone)]
pub struct WindowClass {
    pub name: String,
    pub style: u32,
    pub background: u32,
}

const MAX_CLASSES: usize = 32;
static CLASSES: Mutex<Vec<Option<WindowClass>>> = Mutex::new(Vec::new());

/// A window message (Win32 `MSG`).
#[derive(Clone, Copy, Debug)]
pub struct Message {
    pub hwnd: u64,
    pub msg: u32,
    pub wparam: u64,
    pub lparam: u64,
}

/// Common Win32 message ids.
pub mod wm {
    pub const WM_CREATE: u32 = 0x0001;
    pub const WM_PAINT: u32 = 0x000F;
    pub const WM_DESTROY: u32 = 0x0002;
    pub const WM_CLOSE: u32 = 0x0010;
    pub const WM_KEYDOWN: u32 = 0x0100;
    pub const WM_LBUTTONDOWN: u32 = 0x0201;
}

/// A Win32 window (HWND).
#[derive(Clone)]
pub struct Wnd {
    pub hwnd: u64,
    pub class: String,
    pub title: String,
    pub compositor: WindowId,
    pub queue: VecDeque<Message>,
}

const MAX_WNDS: usize = 64;
static WNDS: Mutex<Vec<Option<Wnd>>> = Mutex::new(Vec::new());
static NEXT_HWND: Mutex<u64> = Mutex::new(1);

/// Register a window class (Win32 `RegisterClassW`). Returns `true` on
/// success or if the class is already registered.
pub fn register_class(name: &str, style: u32, background: u32) -> bool {
    let mut classes = CLASSES.lock();
    if classes.is_empty() {
        classes.resize_with(MAX_CLASSES, || None);
    }
    if classes.iter().flatten().any(|c| c.name == name) {
        return true;
    }
    let slot = match classes.iter_mut().find(|c| c.is_none()) {
        Some(s) => s,
        None => return false,
    };
    *slot = Some(WindowClass {
        name: String::from(name),
        style,
        background,
    });
    true
}

/// Create a window of `class` (Win32 `CreateWindowExW`). Allocates an HWND, a
/// compositor backbuffer, and an empty message queue.
pub fn create_window_ex(
    class: &str,
    title: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Option<u64> {
    let mut wnds = WNDS.lock();
    if wnds.is_empty() {
        wnds.resize_with(MAX_WNDS, || None);
    }
    let compositor = create_window(title, x, y, width, height)?;
    let mut next = NEXT_HWND.lock();
    let hwnd = *next;
    *next += 1;
    let slot = wnds.iter_mut().find(|w| w.is_none())?;
    *slot = Some(Wnd {
        hwnd,
        class: String::from(class),
        title: String::from(title),
        compositor,
        queue: VecDeque::new(),
    });
    Some(hwnd)
}

fn with_wnd<R>(hwnd: u64, f: impl FnOnce(&mut Wnd) -> R) -> Option<R> {
    let mut wnds = WNDS.lock();
    wnds.iter_mut().flatten().find(|w| w.hwnd == hwnd).map(f)
}

/// Post a message to a window's queue (Win32 `PostMessage`).
pub fn post_message(hwnd: u64, msg: u32, wparam: u64, lparam: u64) -> bool {
    with_wnd(hwnd, |w| {
        w.queue.push_back(Message {
            hwnd,
            msg,
            wparam,
            lparam,
        })
    })
    .is_some()
}

/// Retrieve and remove the next queued message (Win32 `GetMessage`).
pub fn get_message(hwnd: u64) -> Option<Message> {
    with_wnd(hwnd, |w| w.queue.pop_front())?
}

/// Default window procedure (Win32 `DefWindowProcW`). Handles `WM_CREATE`,
/// `WM_DESTROY`, and `WM_CLOSE`; all others are ignored (return 0).
pub fn def_window_proc(hwnd: u64, msg: u32, _wparam: u64, _lparam: u64) -> u64 {
    match msg {
        wm::WM_CREATE => 0,
        wm::WM_DESTROY => 0,
        wm::WM_CLOSE => {
            let _ = post_message(hwnd, wm::WM_DESTROY, 0, 0);
            0
        }
        _ => 0,
    }
}

/// Dispatch a window message (Win32 `DispatchMessage`): route to
/// `def_window_proc`. Real applications supply a per-class `wndproc`; the
/// baseline model uses the default for all classes.
pub fn dispatch_message(hwnd: u64, msg: u32, wparam: u64, lparam: u64) {
    def_window_proc(hwnd, msg, wparam, lparam);
}

/// Phase 6 self-test: register a class, create a window, post and retrieve a
/// `WM_PAINT` message, and dispatch it through the default window procedure.
pub fn self_test() -> bool {
    if !register_class("ApertureMain", 0, 0x00_20_40_A0) {
        crate::logln!("win32k: self_test FAIL register");
        return false;
    }
    // Registering the same class twice must be idempotent.
    if !register_class("ApertureMain", 0, 0) {
        crate::logln!("win32k: self_test FAIL re-register");
        return false;
    }
    let Some(hwnd) = create_window_ex("ApertureMain", "Main", 200, 60, 120, 90) else {
        crate::logln!("win32k: self_test FAIL create");
        return false;
    };
    if !post_message(hwnd, wm::WM_PAINT, 0, 0) {
        crate::logln!("win32k: self_test FAIL post");
        return false;
    }
    let Some(msg) = get_message(hwnd) else {
        crate::logln!("win32k: self_test FAIL get");
        return false;
    };
    if msg.msg != wm::WM_PAINT || msg.hwnd != hwnd {
        crate::logln!("win32k: self_test FAIL msg {:?}", msg);
        return false;
    }
    dispatch_message(hwnd, msg.msg, msg.wparam, msg.lparam);
    crate::logln!("win32k: self_test OK (hwnd={} WM_PAINT dispatched)", hwnd);
    true
}
