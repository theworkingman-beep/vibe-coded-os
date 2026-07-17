//! Built-in native system DLL shim registry.
//!
//! Aperture OS implements the Windows system DLLs (ntdll, kernel32, user32,
//! gdi32, ...) as **built-in native shims** rather than external `.dll`
//! files. The PE loader resolves a binary's imports against this registry:
//! each `(dll, export)` pair maps to a native Rust function pointer. When a
//! translated external binary calls an imported system function, the import
//! thunk routes here and runs at full native speed (the FEX-Emu / Rosetta 2
//! model). Only the external binary's own code is ever translated.
//!
//! This is the resolution half of Phase 3. The shims themselves are thunks
//! that either implement the semantics directly or forward to the matching
//! NT syscall handler in `nt.rs`.

use crate::win32::nt;

/// A native shim entry: maps a `(dll, export-name)` pair to a handler.
#[derive(Clone, Copy)]
pub struct ShimEntry {
    pub dll: &'static str,
    pub export: &'static str,
    pub handler: fn([usize; 16]) -> u64,
}

/// Built-in shim table. Order does not matter; lookup is linear (the table is
/// small and resolution happens once at load time).
static SHIMS: &[ShimEntry] = &[
    // ntdll.dll: thin syscall stubs. The handler invokes the real NT
    // dispatcher and returns the NTSTATUS in RAX, matching the Windows ntdll
    // stub convention.
    ShimEntry {
        dll: "ntdll.dll",
        export: "NtDelayExecution",
        handler: shim_nt_delay_execution,
    },
    ShimEntry {
        dll: "ntdll.dll",
        export: "NtQuerySystemInformation",
        handler: shim_nt_query_system_information,
    },
    ShimEntry {
        dll: "ntdll.dll",
        export: "NtAllocateVirtualMemory",
        handler: shim_nt_allocate_virtual_memory,
    },
    ShimEntry {
        dll: "ntdll.dll",
        export: "NtFreeVirtualMemory",
        handler: shim_nt_free_virtual_memory,
    },
    ShimEntry {
        dll: "ntdll.dll",
        export: "NtClose",
        handler: shim_nt_close,
    },
    // kernel32.dll: higher-level wrappers. Sleep forwards to NtDelayExecution;
    // GetTickCount returns the monotonic cycle count as milliseconds.
    ShimEntry {
        dll: "kernel32.dll",
        export: "Sleep",
        handler: shim_sleep,
    },
    ShimEntry {
        dll: "kernel32.dll",
        export: "GetTickCount",
        handler: shim_get_tick_count,
    },
    ShimEntry {
        dll: "kernel32.dll",
        export: "GetTickCount64",
        handler: shim_get_tick_count,
    },
    ShimEntry {
        dll: "kernel32.dll",
        export: "ExitProcess",
        handler: shim_exit_process,
    },
    ShimEntry {
        dll: "kernel32.dll",
        export: "GetLastError",
        handler: shim_get_last_error,
    },
];

/// Resolve an imported `(dll, export)` pair to a native function pointer.
///
/// `dll` matching is case-insensitive and ignores a trailing `.dll`/path
/// component, so `kernel32`, `KERNEL32.DLL`, and `C:\Windows\System32\kernel32.dll`
/// all resolve to the same entry.
pub fn resolve_import(dll: &str, export: &str) -> Option<usize> {
    let entry = SHIMS
        .iter()
        .find(|s| dll_matches(s.dll, dll) && s.export == export)?;
    Some(entry.handler as usize)
}

/// Return the number of registered shims (for boot logging).
pub fn registered_count() -> usize {
    SHIMS.len()
}

fn dll_matches(registered: &str, requested: &str) -> bool {
    // Strip any path prefix from the requested name.
    let base = requested.rsplit(['/', '\\']).next().unwrap_or(requested);
    strip_dll_ci(registered).eq_ignore_ascii_case(strip_dll_ci(base))
}

/// Strip a trailing `.dll` suffix (case-insensitive) without allocating.
fn strip_dll_ci(s: &str) -> &str {
    if s.len() >= 4 && s[s.len() - 4..].eq_ignore_ascii_case(".dll") {
        &s[..s.len() - 4]
    } else {
        s
    }
}

// --- shim handlers ---------------------------------------------------------

fn shim_nt_delay_execution(args: [usize; 16]) -> u64 {
    nt::dispatch(nt::SyscallNumber::NtDelayExecution, args) as u64
}

fn shim_nt_query_system_information(args: [usize; 16]) -> u64 {
    nt::dispatch(nt::SyscallNumber::NtQuerySystemInformation, args) as u64
}

fn shim_nt_allocate_virtual_memory(args: [usize; 16]) -> u64 {
    nt::dispatch(nt::SyscallNumber::NtAllocateVirtualMemory, args) as u64
}

fn shim_nt_free_virtual_memory(args: [usize; 16]) -> u64 {
    nt::dispatch(nt::SyscallNumber::NtFreeVirtualMemory, args) as u64
}

fn shim_nt_close(args: [usize; 16]) -> u64 {
    nt::dispatch(nt::SyscallNumber::NtClose, args) as u64
}

fn shim_sleep(_args: [usize; 16]) -> u64 {
    // Forward to NtDelayExecution with a zero interval (cooperative yield).
    nt::dispatch(
        nt::SyscallNumber::NtDelayExecution,
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ) as u64
}

fn shim_get_tick_count(_args: [usize; 16]) -> u64 {
    // Monotonic cycles converted to ms at a nominal 1 GHz counter rate.
    crate::arch::monotonic_cycles() / 1_000_000
}

fn shim_exit_process(_args: [usize; 16]) -> u64 {
    // A real implementation terminates the process and reaps handles. For the
    // shim registry we log and return; the scheduler reaps exited threads.
    crate::logln!("kernel32: ExitProcess requested");
    0
}

fn shim_get_last_error(_args: [usize; 16]) -> u64 {
    0
}

/// Boot-time self-test: resolve a representative import from each system DLL
/// and log the result. Returns `true` if all expected exports resolve.
pub fn self_test() -> bool {
    let cases = [
        ("ntdll.dll", "NtDelayExecution"),
        ("kernel32.dll", "Sleep"),
        ("KERNEL32.DLL", "GetTickCount"),
        ("C:\\Windows\\System32\\kernel32.dll", "ExitProcess"),
    ];
    let mut ok = true;
    for (dll, exp) in cases {
        match resolve_import(dll, exp) {
            Some(addr) => {
                crate::logln!("shim: {}!{} -> {:#x}", dll, exp, addr);
            }
            None => {
                crate::logln!("shim: {}!{} UNRESOLVED", dll, exp);
                ok = false;
            }
        }
    }
    // A negative case: an unknown export must not resolve.
    if resolve_import("kernel32.dll", "DefinitelyNotAnExport").is_some() {
        crate::logln!("shim: self_test FAIL unknown export resolved");
        return false;
    }
    if ok {
        crate::logln!(
            "shim: self_test OK ({} shims registered)",
            registered_count()
        );
    }
    ok
}
