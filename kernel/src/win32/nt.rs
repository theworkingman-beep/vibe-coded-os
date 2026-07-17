//! NT system call dispatch and implementations.
//!
//! This module defines the syscall numbers, dispatch stub, and concrete
//! implementations for the most important NT kernel calls. The implementations
//! route to the Aperture OS VFS and memory allocator.

use super::{loader, objects, process, registry, thread};
use crate::vfs::{self, FileHandle, NodeKind};
use core::sync::atomic::{AtomicBool, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init() {
    INITIALIZED.store(true, Ordering::Relaxed);
}

/// NT status codes used by the dispatch layer.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NtStatus {
    Success = 0x00000000,
    Pending = 0x00000103,
    NotImplemented = 0xC0000002,
    InvalidParameter = 0xC000000D,
    AccessDenied = 0xC0000022,
    BufferTooSmall = 0xC0000023,
    ObjectNameNotFound = 0xC0000034,
    ObjectNameCollision = 0xC0000035,
    EndOfFile = 0xC0000011,
}

/// NT system call numbers (subset). Windows keeps these stable per architecture.
#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyscallNumber {
    NtCreateFile = 0x0055,
    NtClose = 0x000F,
    NtReadFile = 0x006F,
    NtWriteFile = 0x0008,
    NtAllocateVirtualMemory = 0x0018,
    NtFreeVirtualMemory = 0x001E,
    NtCreateProcess = 0x004F,
    NtCreateThread = 0x004E,
    NtQuerySystemInformation = 0x0036,
    NtQueryInformationProcess = 0x0019,
    NtSetInformationProcess = 0x001C,
    NtDelayExecution = 0x0034,
    NtCreateKey = 0x001D,
    NtQueryValueKey = 0x0049,
    NtSetValueKey = 0x005D,
    NtWaitForMultipleObjects = 0x0001,
}

impl SyscallNumber {
    /// Convert a raw syscall number to the enum, if known.
    pub fn from_raw(value: usize) -> Option<Self> {
        use SyscallNumber::*;
        Some(match value {
            0x0055 => NtCreateFile,
            0x000F => NtClose,
            0x006F => NtReadFile,
            0x0008 => NtWriteFile,
            0x0018 => NtAllocateVirtualMemory,
            0x001E => NtFreeVirtualMemory,
            0x004F => NtCreateProcess,
            0x004E => NtCreateThread,
            0x0036 => NtQuerySystemInformation,
            0x0019 => NtQueryInformationProcess,
            0x001C => NtSetInformationProcess,
            0x0034 => NtDelayExecution,
            0x001D => NtCreateKey,
            0x0049 => NtQueryValueKey,
            0x005D => NtSetValueKey,
            0x0001 => NtWaitForMultipleObjects,
            _ => return None,
        })
    }
}

impl From<usize> for SyscallNumber {
    fn from(value: usize) -> Self {
        Self::from_raw(value).unwrap_or(Self::NtWaitForMultipleObjects)
    }
}

/// Syscall dispatch table entry.
#[derive(Clone, Copy)]
struct SyscallHandler {
    func: fn([usize; 16]) -> NtStatus,
}

macro_rules! handler_table {
    ($(($num:expr, $fn:ident)),* $(,)?) => {
        {
            let mut table: [Option<SyscallHandler>; 256] = [None; 256];
            $(
                table[$num as usize] = Some(SyscallHandler { func: $fn });
            )*
            table
        }
    };
}

static mut SYSCALL_TABLE: [Option<SyscallHandler>; 256] = [None; 256];

/// Initialize the syscall dispatch table.
pub fn init_syscall_table() {
    unsafe {
        SYSCALL_TABLE = handler_table!(
            (SyscallNumber::NtClose, handle_close),
            (SyscallNumber::NtCreateFile, handle_create_file),
            (SyscallNumber::NtReadFile, handle_read_file),
            (SyscallNumber::NtWriteFile, handle_write_file),
            (
                SyscallNumber::NtAllocateVirtualMemory,
                handle_allocate_virtual_memory
            ),
            (
                SyscallNumber::NtFreeVirtualMemory,
                handle_free_virtual_memory
            ),
            (
                SyscallNumber::NtQuerySystemInformation,
                handle_query_system_information
            ),
            (
                SyscallNumber::NtQueryInformationProcess,
                handle_query_information_process
            ),
            (SyscallNumber::NtDelayExecution, handle_delay_execution),
            (SyscallNumber::NtCreateKey, handle_create_key),
            (SyscallNumber::NtSetValueKey, handle_set_value_key),
            (SyscallNumber::NtQueryValueKey, handle_query_value_key),
            (SyscallNumber::NtCreateProcess, handle_create_process),
            (SyscallNumber::NtCreateThread, handle_create_thread),
            (
                SyscallNumber::NtSetInformationProcess,
                handle_set_information_process
            ),
            (
                SyscallNumber::NtWaitForMultipleObjects,
                handle_wait_for_multiple_objects
            ),
        );
    }
}

/// Syscall dispatch stub. The real entry point marshals arguments from the
/// user-mode trap frame and calls the appropriate handler.
pub fn dispatch(number: SyscallNumber, args: [usize; 16]) -> NtStatus {
    if !INITIALIZED.load(Ordering::Relaxed) {
        return NtStatus::AccessDenied;
    }
    unsafe {
        if let Some(handler) = SYSCALL_TABLE[number as usize] {
            (handler.func)(args)
        } else {
            NtStatus::NotImplemented
        }
    }
}

fn handle_close(args: [usize; 16]) -> NtStatus {
    let handle = objects::Handle(args[0] as u32);
    close(handle)
}

fn handle_create_file(args: [usize; 16]) -> NtStatus {
    // Simplified bring-up ABI (not the full Windows NtCreateFile signature):
    //   args[0] = pointer to u64 that receives the handle
    //   args[1] = pointer to null-terminated path
    //   args[2] = create disposition (nonzero = create)
    //   args[3] = access mask (write bit enables writing)
    let out_handle_ptr = args[0] as u64;
    let path_ptr = args[1] as u64;
    if path_ptr == 0 || out_handle_ptr == 0 {
        return NtStatus::InvalidParameter;
    }

    let path_phys = match unsafe { user_ptr_to_phys(path_ptr) } {
        Some(p) => p,
        None => return NtStatus::InvalidParameter,
    };
    let out_phys = match unsafe { user_ptr_to_phys(out_handle_ptr) } {
        Some(p) => p,
        None => return NtStatus::InvalidParameter,
    };

    let path = unsafe { guest_cstr(path_phys, 128) };
    let create = args[2] != 0;
    let write_access = (args[3] & 0x4000_0000) != 0 || create;

    match create_file(path, create, false, write_access) {
        Ok(handle) => {
            unsafe { core::ptr::write_volatile(out_phys as *mut u64, handle.0 as u64) };
            NtStatus::Success
        }
        Err(status) => status,
    }
}

fn handle_read_file(args: [usize; 16]) -> NtStatus {
    // Simplified bring-up ABI:
    //   args[0] = handle
    //   args[1] = buffer pointer
    //   args[2] = length
    let handle = objects::Handle(args[0] as u32);
    let buf_ptr = args[1] as u64;
    let len = args[2];
    if buf_ptr == 0 || len == 0 {
        return NtStatus::Success;
    }
    let buf_phys = match unsafe { user_ptr_to_phys(buf_ptr) } {
        Some(p) => p,
        None => return NtStatus::InvalidParameter,
    };
    let buf = unsafe { core::slice::from_raw_parts_mut(buf_phys as *mut u8, len) };
    match read_file(handle, buf) {
        Ok(_read) => NtStatus::Success,
        Err(status) => status,
    }
}

fn handle_write_file(args: [usize; 16]) -> NtStatus {
    // Simplified bring-up ABI:
    //   args[0] = handle
    //   args[1] = buffer pointer
    //   args[2] = length
    let handle = objects::Handle(args[0] as u32);
    let buf_ptr = args[1] as u64;
    let len = args[2];
    if buf_ptr == 0 || len == 0 {
        return NtStatus::Success;
    }
    let buf_phys = match unsafe { user_ptr_to_phys(buf_ptr) } {
        Some(p) => p,
        None => return NtStatus::InvalidParameter,
    };
    let buf = unsafe { core::slice::from_raw_parts(buf_phys as *const u8, len) };
    match write_file(handle, buf) {
        Ok(_written) => NtStatus::Success,
        Err(status) => status,
    }
}

fn handle_allocate_virtual_memory(args: [usize; 16]) -> NtStatus {
    // Windows x64 ABI: RDI=handle, RSI=BaseAddress pointer, RDX=ZeroBits,
    // R10=RegionSize pointer/value, R8=AllocationType, R9=Protect.
    let size = args[3];
    match allocate_virtual_memory(size) {
        Ok(base) => {
            let base_ptr = args[1] as *mut u64;
            if !base_ptr.is_null() {
                // The synthetic test fixture places the output variable at an
                // unaligned address; use unaligned write to tolerate it while
                // still validating the ring-3 syscall round trip.
                unsafe {
                    base_ptr.write_unaligned(base);
                }
            }
            NtStatus::Success
        }
        Err(status) => status,
    }
}

fn handle_free_virtual_memory(args: [usize; 16]) -> NtStatus {
    let base = args[0] as u64;
    let size = args[1];
    free_virtual_memory(base, size)
}

fn handle_query_system_information(args: [usize; 16]) -> NtStatus {
    // Windows x64 ABI: RDI=class, RSI=buffer, RDX=length, R10=return-length ptr.
    let class = args[0] as u32;
    let buf_ptr = args[1] as u64;
    let len = args[2];
    let ret_len_ptr = args[3] as u64;

    if buf_ptr == 0 || len == 0 {
        return NtStatus::InvalidParameter;
    }
    let buf_phys = match unsafe { user_ptr_to_phys(buf_ptr) } {
        Some(p) => p,
        None => return NtStatus::InvalidParameter,
    };
    let buf = unsafe { core::slice::from_raw_parts_mut(buf_phys as *mut u8, len) };

    match class {
        // SystemBasicInformation
        0 => {
            if len < 64 {
                return NtStatus::BufferTooSmall;
            }
            let page_size: u32 = 4096;
            let phys_pages: u32 = 0x8000; // 128 MiB worth of 4 KiB pages (placeholder)
            buf[0..4].copy_from_slice(&page_size.to_le_bytes());
            buf[4..8].copy_from_slice(&phys_pages.to_le_bytes());
            // Zero the rest; the Windows struct has many more fields.
            buf[8..64].fill(0);
            if ret_len_ptr != 0 {
                if let Some(p) = unsafe { user_ptr_to_phys(ret_len_ptr) } {
                    unsafe { core::ptr::write_volatile(p as *mut u32, 64) };
                }
            }
            NtStatus::Success
        }
        // SystemProcessorInformation
        1 => {
            if len < 2 {
                return NtStatus::BufferTooSmall;
            }
            // ProcessorArchitecture: AMD64 = 9, ARM64 = 12.
            let arch: u16 = 12;
            buf[0..2].copy_from_slice(&arch.to_le_bytes());
            if ret_len_ptr != 0 {
                if let Some(p) = unsafe { user_ptr_to_phys(ret_len_ptr) } {
                    unsafe { core::ptr::write_volatile(p as *mut u32, 2) };
                }
            }
            NtStatus::Success
        }
        _ => NtStatus::NotImplemented,
    }
}

fn handle_query_information_process(args: [usize; 16]) -> NtStatus {
    // Windows x64 ABI: RDI=handle, RSI=class, RDX=buffer, R10=length, R8=return-length ptr.
    let _handle = args[0] as u64;
    let class = args[1] as u32;
    let buf_ptr = args[2] as u64;
    let len = args[3];
    let ret_len_ptr = args[4] as u64;

    if buf_ptr == 0 || len == 0 {
        return NtStatus::InvalidParameter;
    }
    let buf_phys = match unsafe { user_ptr_to_phys(buf_ptr) } {
        Some(p) => p,
        None => return NtStatus::InvalidParameter,
    };

    match class {
        // ProcessBasicInformation
        0 => {
            if len < 48 {
                return NtStatus::BufferTooSmall;
            }
            let buf = unsafe { core::slice::from_raw_parts_mut(buf_phys as *mut u8, 48) };
            let pid: u64 = 1;
            buf[0..8].copy_from_slice(&pid.to_le_bytes()); // UniqueProcessId
            buf[8..48].fill(0);
            if ret_len_ptr != 0 {
                if let Some(p) = unsafe { user_ptr_to_phys(ret_len_ptr) } {
                    unsafe { core::ptr::write_volatile(p as *mut u32, 48) };
                }
            }
            NtStatus::Success
        }
        _ => NtStatus::NotImplemented,
    }
}

fn handle_delay_execution(args: [usize; 16]) -> NtStatus {
    // Windows x64 ABI: RDI=Alertable, RSI=DelayInterval (LARGE_INTEGER*)
    let _alertable = args[0] != 0;
    let interval_ptr = args[1] as u64;
    let _interval = if interval_ptr != 0 {
        if let Some(p) = unsafe { user_ptr_to_phys(interval_ptr) } {
            unsafe { core::ptr::read_volatile(p as *const i64) }
        } else {
            0
        }
    } else {
        0
    };
    // Cooperative yield. A real implementation would program a timer and
    // block the thread, but the baseline scheduler only supports round-robin.
    crate::win32::scheduler::yield_current();
    NtStatus::Success
}

fn handle_create_key(args: [usize; 16]) -> NtStatus {
    // Simplified bring-up ABI:
    //   args[0] = pointer to u64 that receives the key handle
    //   args[1] = pointer to null-terminated key path
    let out_handle_ptr = args[0] as u64;
    let path_ptr = args[1] as u64;
    if path_ptr == 0 || out_handle_ptr == 0 {
        return NtStatus::InvalidParameter;
    }
    let path_phys = match unsafe { user_ptr_to_phys(path_ptr) } {
        Some(p) => p,
        None => return NtStatus::InvalidParameter,
    };
    let out_phys = match unsafe { user_ptr_to_phys(out_handle_ptr) } {
        Some(p) => p,
        None => return NtStatus::InvalidParameter,
    };
    let path = unsafe { guest_cstr(path_phys, 256) };
    // Leak a 'static copy of the path so the object handle can own it.
    let leaked = leak_str(path);
    let handle =
        match objects::allocate(objects::ObjectKind::RegistryKey, leaked.as_ptr() as *mut ()) {
            Some(h) => h,
            None => return NtStatus::InvalidParameter,
        };
    unsafe { core::ptr::write_volatile(out_phys as *mut u64, handle.0 as u64) };
    NtStatus::Success
}

fn handle_set_value_key(args: [usize; 16]) -> NtStatus {
    // Simplified bring-up ABI:
    //   args[0] = key handle
    //   args[1] = value name (cstr)
    //   args[2] = REG_* type
    //   args[3] = data pointer
    //   args[4] = data length (bytes)
    let handle = objects::Handle(args[0] as u32);
    let name_ptr = args[1] as u64;
    let ty_raw = args[2] as u32;
    let data_ptr = args[3] as u64;
    let data_len = args[4];

    let header = match objects::lookup(handle) {
        Some(h) if h.kind == objects::ObjectKind::RegistryKey => h,
        _ => return NtStatus::InvalidParameter,
    };
    let key_path: &str = unsafe {
        let p = header.data as *const u8;
        let mut len = 0usize;
        while *p.add(len) != 0 {
            len += 1;
        }
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(p, len))
    };

    let value_name = if name_ptr == 0 {
        ""
    } else {
        match unsafe { user_ptr_to_phys(name_ptr) } {
            Some(p) => unsafe { guest_cstr(p, 256) },
            None => return NtStatus::InvalidParameter,
        }
    };

    let ty = match ty_raw {
        1 => registry::RegValueType::String,
        4 => registry::RegValueType::Dword,
        11 => registry::RegValueType::Qword,
        _ => registry::RegValueType::Binary,
    };

    let data = if data_ptr == 0 || data_len == 0 {
        &[][..]
    } else {
        match unsafe { user_ptr_to_phys(data_ptr) } {
            Some(p) => unsafe { core::slice::from_raw_parts(p as *const u8, data_len) },
            None => return NtStatus::InvalidParameter,
        }
    };

    if registry::set_value(key_path, value_name, ty, data) {
        NtStatus::Success
    } else {
        NtStatus::InvalidParameter
    }
}

fn handle_query_value_key(args: [usize; 16]) -> NtStatus {
    // Simplified bring-up ABI:
    //   args[0] = key handle
    //   args[1] = value name (cstr)
    //   args[2] = output buffer pointer
    //   args[3] = output buffer length
    //   args[4] = pointer to u32 that receives the written length
    let handle = objects::Handle(args[0] as u32);
    let name_ptr = args[1] as u64;
    let out_ptr = args[2] as u64;
    let out_len = args[3];
    let ret_len_ptr = args[4] as u64;

    let header = match objects::lookup(handle) {
        Some(h) if h.kind == objects::ObjectKind::RegistryKey => h,
        _ => return NtStatus::InvalidParameter,
    };
    let key_path: &str = unsafe {
        let p = header.data as *const u8;
        let mut len = 0usize;
        while *p.add(len) != 0 {
            len += 1;
        }
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(p, len))
    };

    let value_name = if name_ptr == 0 {
        ""
    } else {
        match unsafe { user_ptr_to_phys(name_ptr) } {
            Some(p) => unsafe { guest_cstr(p, 256) },
            None => return NtStatus::InvalidParameter,
        }
    };

    let Some(value) = registry::get_value(key_path, value_name) else {
        return NtStatus::ObjectNameNotFound;
    };

    let written = value.len.min(out_len);
    if out_ptr != 0 && written > 0 {
        match unsafe { user_ptr_to_phys(out_ptr) } {
            Some(p) => unsafe {
                core::ptr::copy_nonoverlapping(value.data.as_ptr(), p as *mut u8, written);
            },
            None => return NtStatus::InvalidParameter,
        }
    }
    if ret_len_ptr != 0 {
        if let Some(p) = unsafe { user_ptr_to_phys(ret_len_ptr) } {
            unsafe { core::ptr::write_volatile(p as *mut u32, value.len as u32) };
        }
    }
    NtStatus::Success
}

fn handle_create_process(args: [usize; 16]) -> NtStatus {
    // Simplified bring-up ABI: args[0] = pointer to u64 receiving the handle.
    let out_handle_ptr = args[0] as u64;
    if out_handle_ptr == 0 {
        return NtStatus::InvalidParameter;
    }
    let out_phys = match unsafe { user_ptr_to_phys(out_handle_ptr) } {
        Some(p) => p,
        None => return NtStatus::InvalidParameter,
    };
    let size = core::mem::size_of::<process::Process>();
    let align = core::mem::align_of::<process::Process>();
    let ptr = crate::mm::alloc_early(size, align);
    let Some(ptr) = ptr else {
        return NtStatus::InvalidParameter;
    };
    unsafe { core::ptr::write(ptr as *mut process::Process, process::Process::new(0)) };
    let handle = match objects::allocate(objects::ObjectKind::Process, ptr as *mut ()) {
        Some(h) => h,
        None => return NtStatus::InvalidParameter,
    };
    unsafe { core::ptr::write_volatile(out_phys as *mut u64, handle.0 as u64) };
    NtStatus::Success
}

fn handle_create_thread(args: [usize; 16]) -> NtStatus {
    // Simplified bring-up ABI: args[0] = pointer to u64 receiving the handle.
    let out_handle_ptr = args[0] as u64;
    if out_handle_ptr == 0 {
        return NtStatus::InvalidParameter;
    }
    let out_phys = match unsafe { user_ptr_to_phys(out_handle_ptr) } {
        Some(p) => p,
        None => return NtStatus::InvalidParameter,
    };
    let size = core::mem::size_of::<thread::Thread>();
    let align = core::mem::align_of::<thread::Thread>();
    let ptr = crate::mm::alloc_early(size, align);
    let Some(ptr) = ptr else {
        return NtStatus::InvalidParameter;
    };
    unsafe { core::ptr::write(ptr as *mut thread::Thread, thread::Thread::new(0, 0, 0)) };
    let handle = match objects::allocate(objects::ObjectKind::Thread, ptr as *mut ()) {
        Some(h) => h,
        None => return NtStatus::InvalidParameter,
    };
    unsafe { core::ptr::write_volatile(out_phys as *mut u64, handle.0 as u64) };
    NtStatus::Success
}

fn handle_set_information_process(_args: [usize; 16]) -> NtStatus {
    // Accepted and acknowledged; the baseline process model has no mutable
    // per-process information classes wired yet.
    NtStatus::Success
}

fn handle_wait_for_multiple_objects(_args: [usize; 16]) -> NtStatus {
    // No dispatchable waitable objects yet; yield the quantum and report
    // success so callers do not spin in a hard loop.
    crate::win32::scheduler::yield_current();
    NtStatus::Success
}

/// Leak a 'static copy of `s` so an object handle can own it without a
/// lifetime. Uses the early heap; the handle table never frees these.
fn leak_str(s: &str) -> &'static str {
    let buf = crate::mm::alloc_early(s.len(), 1).expect("nt leak_str alloc");
    unsafe {
        core::ptr::copy_nonoverlapping(s.as_ptr(), buf as *mut u8, s.len());
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(buf as *const u8, s.len()))
    }
}

/// Phase 7 self-test: exercise the registry syscall path end-to-end.
///
/// Creates a registry key, sets a `REG_SZ` value, queries it back, and
/// verifies the bytes round-trip intact. Runs in kernel context (cr3 == 0)
/// so user pointers are identity-mapped.
pub fn registry_self_test() -> bool {
    use crate::mm::alloc_early;

    // NtCreateKey: out handle + path.
    let mut key_handle: u64 = 0;
    let path = b"HKLM\\Software\\Aperture\0";
    let path_phys = alloc_early(path.len(), 1).expect("regtest path");
    unsafe { core::ptr::copy_nonoverlapping(path.as_ptr(), path_phys as *mut u8, path.len()) };
    let create_status = dispatch(
        SyscallNumber::NtCreateKey,
        [
            &mut key_handle as *mut u64 as usize,
            path_phys as usize,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ],
    );
    if create_status != NtStatus::Success || key_handle == 0 {
        crate::logln!(
            "reg: self_test FAIL create ({:?}, h={})",
            create_status,
            key_handle
        );
        return false;
    }

    // NtSetValueKey: value "Version" = "1.0.0" (REG_SZ).
    let vname = b"Version\0";
    let vname_phys = alloc_early(vname.len(), 1).expect("regtest vname");
    unsafe { core::ptr::copy_nonoverlapping(vname.as_ptr(), vname_phys as *mut u8, vname.len()) };
    let vdata = b"1.0.0";
    let vdata_phys = alloc_early(vdata.len(), 1).expect("regtest vdata");
    unsafe { core::ptr::copy_nonoverlapping(vdata.as_ptr(), vdata_phys as *mut u8, vdata.len()) };
    let set_status = dispatch(
        SyscallNumber::NtSetValueKey,
        [
            key_handle as usize,
            vname_phys as usize,
            1, // REG_SZ
            vdata_phys as usize,
            vdata.len(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ],
    );
    if set_status != NtStatus::Success {
        crate::logln!("reg: self_test FAIL set {:?}", set_status);
        return false;
    }

    // NtQueryValueKey: read it back.
    let mut out_buf = [0u8; 32];
    let mut out_len: u32 = 0;
    let qname = b"Version\0";
    let qname_phys = alloc_early(qname.len(), 1).expect("regtest qname");
    unsafe { core::ptr::copy_nonoverlapping(qname.as_ptr(), qname_phys as *mut u8, qname.len()) };
    let query_status = dispatch(
        SyscallNumber::NtQueryValueKey,
        [
            key_handle as usize,
            qname_phys as usize,
            out_buf.as_mut_ptr() as usize,
            out_buf.len(),
            &mut out_len as *mut u32 as usize,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ],
    );
    if query_status != NtStatus::Success {
        crate::logln!("reg: self_test FAIL query {:?}", query_status);
        return false;
    }
    let got = &out_buf[..(out_len as usize).min(out_buf.len())];
    if got == b"1.0.0" {
        crate::logln!(
            "reg: self_test OK (Version={})",
            core::str::from_utf8(got).unwrap_or("?")
        );
        true
    } else {
        crate::logln!("reg: self_test FAIL roundtrip len={}", out_len);
        false
    }
}

/// Open or create a file and return an object-manager handle.
pub fn create_file(
    path: &str,
    create: bool,
    _directory: bool,
    write_access: bool,
) -> Result<objects::Handle, NtStatus> {
    let vfs_handle = open_vfs_file(path, create, write_access)?;
    objects::allocate(objects::ObjectKind::File, vfs_handle.0 as *mut ())
        .ok_or(NtStatus::InvalidParameter)
}

/// Read from an open file object handle.
pub fn read_file(handle: objects::Handle, buffer: &mut [u8]) -> Result<usize, NtStatus> {
    let vfs_handle = vfs_file_from_object(handle)?;
    vfs::read(vfs_handle, buffer).ok_or(NtStatus::InvalidParameter)
}

/// Write to an open file object handle.
pub fn write_file(handle: objects::Handle, buffer: &[u8]) -> Result<usize, NtStatus> {
    let vfs_handle = vfs_file_from_object(handle)?;
    vfs::write(vfs_handle, buffer).ok_or(NtStatus::InvalidParameter)
}

/// Close an open object handle.
pub fn close(handle: objects::Handle) -> NtStatus {
    let Some(header) = objects::lookup(handle) else {
        return NtStatus::InvalidParameter;
    };
    if header.kind != objects::ObjectKind::File {
        return NtStatus::InvalidParameter;
    }
    let _ = vfs::close(FileHandle(header.data));
    if objects::close(handle) {
        NtStatus::Success
    } else {
        NtStatus::InvalidParameter
    }
}

/// Open or create a VFS file node and return its raw VFS handle.
fn open_vfs_file(path: &str, create: bool, write_access: bool) -> Result<FileHandle, NtStatus> {
    let node = vfs::lookup(path);
    let node = match node {
        Some(n) => {
            if create {
                return Err(NtStatus::ObjectNameCollision);
            }
            n
        }
        None => {
            if !create {
                return Err(NtStatus::ObjectNameNotFound);
            }
            let parent_path = parent(path);
            let parent_node = vfs::lookup(parent_path).ok_or(NtStatus::ObjectNameNotFound)?;
            let name = file_name(path);
            vfs::create(parent_node, name, NodeKind::File).ok_or(NtStatus::InvalidParameter)?
        }
    };
    vfs::open(node, write_access).ok_or(NtStatus::InvalidParameter)
}

/// Extract the raw VFS file handle stored in a file object.
fn vfs_file_from_object(handle: objects::Handle) -> Result<FileHandle, NtStatus> {
    let header = objects::lookup(handle).ok_or(NtStatus::InvalidParameter)?;
    if header.kind != objects::ObjectKind::File {
        return Err(NtStatus::InvalidParameter);
    }
    Ok(FileHandle(header.data))
}

/// Allocate virtual memory for a process.
pub fn allocate_virtual_memory(size: usize) -> Result<u64, NtStatus> {
    let frames_needed = (size + 4095) / 4096;
    let mut base = 0u64;
    for _ in 0..frames_needed {
        let frame = crate::mm::frame_allocator::allocate().ok_or(NtStatus::InvalidParameter)?;
        if base == 0 {
            base = frame;
        }
    }
    Ok(base)
}

/// Free virtual memory allocated by `allocate_virtual_memory`.
pub fn free_virtual_memory(base: u64, size: usize) -> NtStatus {
    let frames = (size + 4095) / 4096;
    for i in 0..frames {
        crate::mm::frame_allocator::free(base + i as u64 * 4096);
    }
    NtStatus::Success
}

/// Load a PE executable from the VFS into a new process.
///
/// Returns the object handle for the created process and whether the guest
/// architecture requires binary translation on the host.
pub fn create_user_process(path: &str) -> Option<(objects::Handle, bool)> {
    let file = open_vfs_file(path, false, false).ok()?;
    let size = vfs::file_size(file)?;
    let buf = crate::mm::alloc_early(size, 1)?;

    let slice = unsafe { core::slice::from_raw_parts_mut(buf, size) };
    let read = vfs::read(file, slice)?;
    if read != size {
        return None;
    }

    let _ = vfs::close(file);
    loader::load_pe(slice, 1)
}

fn parent(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) => "/",
        Some(idx) => &path[..idx],
        None => "/",
    }
}

fn file_name(path: &str) -> &str {
    path.rfind('/').map(|idx| &path[idx + 1..]).unwrap_or(path)
}

/// Translate a guest pointer to a physical address the kernel can read/write.
///
/// For interpreted threads the page-table root is zero and guest addresses
/// are already physical. For native threads we walk the current process page
/// table.
unsafe fn user_ptr_to_phys(addr: u64) -> Option<u64> {
    if addr == 0 {
        return None;
    }
    let cr3 =
        crate::win32::scheduler::with_current_thread(|t| t.process_page_table_root).unwrap_or(0);
    if cr3 == 0 {
        return Some(addr);
    }
    let pt = crate::mm::page_table::page_table_root(cr3)?;
    pt.translate(addr)
}

/// Read a null-terminated ASCII string of at most `max` bytes from `phys`.
unsafe fn guest_cstr(phys: u64, max: usize) -> &'static str {
    let mut len = 0usize;
    let bytes = core::slice::from_raw_parts(phys as *const u8, max);
    while len < max && bytes[len] != 0 {
        len += 1;
    }
    core::str::from_utf8_unchecked(&bytes[..len])
}
