//! Windows application compatibility subsystem.
//!
//! The design goal is first-class Windows binary compatibility, not a wrapper
//! around Wine. Aperture OS implements the core NT kernel ABI natively in Rust
//! and provides a clean Win32 subsystem on top of it.
//!
//! Architecture:
//!   - PE loader (`loader`)
//!   - NT system call dispatch table (`nt`)
//!   - Object manager / handle table (`objects`)
//!   - Process/thread model (`process`, `thread`)
//!   - Registry and filesystem shims (`registry`, `fs`)
//!   - User-mode Win32 API server (`win32k`)
//!   - x86-on-ARM and ARM-on-x86 dynamic binary translation (`abi::translate`)

pub mod abi;
pub mod fs;
pub mod loader;
pub mod nt;
pub mod objects;
pub mod port;
pub mod process;
pub mod registry;
pub mod scheduler;
pub mod shims;
pub mod thread;
pub mod win32k;

/// Initialize the Windows subsystem (no-op until the first PE is loaded).
pub fn init() {
    objects::init();
    registry::init();
    nt::init();
    nt::init_syscall_table();
}

/// Run the per-phase self-tests for the Win32 subsystem.
///
/// Each test exercises a real, implemented feature and logs its result. None
/// of them enter user mode or block, so the GUI main loop still runs. Called
/// from `kernel_main` after the heap and Win32 subsystem are initialized.
pub fn phase_self_tests() {
    crate::logln!("win32: phase self-tests start");
    // Phase 2: IPC message ports.
    let port_ok = port::self_test();
    // Phase 3: built-in DLL shim import resolution.
    let shim_ok = shims::self_test();
    // Phase 4: x86_64 guest interpreter runs a tiny program.
    let interp_ok = abi::interpreter::self_test();
    // Phase 7: NT registry create/set/query round-trip.
    let reg_ok = nt::registry_self_test();
    // Phase 9: process environment variables.
    let env_ok = process_env_self_test();
    crate::logln!(
        "win32: phase self-tests done (port={} shim={} interp={} reg={} env={})",
        port_ok,
        shim_ok,
        interp_ok,
        reg_ok,
        env_ok
    );
}

/// Phase 9 self-test: a process environment block get/set round-trip.
fn process_env_self_test() -> bool {
    let mut p = process::Process::new(99);
    p.set_env("OS", "ApertureOS");
    p.set_env(
        "PROCESSOR_ARCHITECTURE",
        if cfg!(feature = "arch_x86_64") {
            "AMD64"
        } else {
            "ARM64"
        },
    );
    // Case-insensitive overwrite returns the previous value.
    let prev = p.set_env("os", "ApertureOS-v1");
    match (p.get_env("OS"), prev.as_deref()) {
        (Some(v), Some("ApertureOS")) if v == "ApertureOS-v1" => {
            crate::logln!(
                "env: self_test OK (OS={}, arch={})",
                v,
                p.get_env("PROCESSOR_ARCHITECTURE").unwrap_or("?")
            );
            true
        }
        other => {
            crate::logln!("env: self_test FAIL {:?}", other);
            false
        }
    }
}

/// Load a small synthetic PE image from the VFS to verify the loader, VFS,
/// and NT process creation path.
///
/// Must be called after the physical frame allocator and early heap have been
/// initialized.
pub fn self_test() {
    crate::logln!("win32: self_test start");

    // Exercise the AArch64 decoder on every boot to verify the ARM→x86
    // translation layer is present even on x86_64 builds.
    abi::aarch64_interpreter::self_test();

    #[cfg(feature = "arch_x86_64")]
    {
        x86_64_raw_user_test();
    }

    #[cfg(feature = "arch_aarch64")]
    {
        crate::logln!("win32: AArch64 user-mode smoke test not yet implemented; halting.");
        crate::hlt();
    }
}

/// Minimal x86_64 user-mode smoke test: map a single code page containing a
/// `syscall` followed by a tight loop, create a thread, and enter ring 3.
/// This validates SYSCALL/SYSRET and the preemptive timer from user space.
#[cfg(feature = "arch_x86_64")]
fn x86_64_raw_user_test() {
    use crate::mm::frame_allocator;
    use crate::mm::hhdm::phys_to_virt;
    use crate::mm::page_table::{PageTable, PAGE_PRESENT, PAGE_USER, PAGE_WRITABLE};
    use crate::win32::scheduler;

    const USER_CODE_VIRT: u64 = 0x10000;
    const USER_ENTRY: u64 = 0x10000;

    // Allocate and fill a user code page with:
    //   mov rax, 0x36          ; NtQuerySystemInformation
    //   xor rdi, rdi
    //   xor rsi, rsi
    //   xor rdx, rdx
    //   xor r10, r10
    //   syscall
    //   jmp $                  ; loop forever
    let code_phys = frame_allocator::allocate().expect("user code frame");
    let code_ptr = phys_to_virt(code_phys) as *mut u8;
    let code: &[u8] = &[
        0x48, 0xc7, 0xc0, 0x36, 0x00, 0x00, 0x00, // mov rax, 0x36
        0x48, 0x31, 0xff, // xor rdi, rdi
        0x48, 0x31, 0xf6, // xor rsi, rsi
        0x48, 0x31, 0xd2, // xor rdx, rdx
        0x49, 0x31, 0xd2, // xor r10, r10
        0x0f, 0x05, // syscall
        0xeb, 0xfe, // jmp $
    ];
    unsafe {
        core::ptr::write_bytes(code_ptr, 0, 4096);
        core::ptr::copy_nonoverlapping(code.as_ptr(), code_ptr, code.len());
    }

    let Some(mut pt) = PageTable::new() else {
        crate::logln!("win32: failed to allocate page table for user test");
        return;
    };
    unsafe {
        pt.map(
            USER_CODE_VIRT,
            code_phys,
            PAGE_PRESENT | PAGE_USER | PAGE_WRITABLE,
        );
    }
    let cr3 = pt.cr3();

    crate::logln!(
        "win32: raw user test code={:#x} cr3={:#x} entry={:#x}",
        code_phys,
        cr3,
        USER_ENTRY
    );

    let slot = scheduler::create_thread(1, USER_ENTRY, cr3).expect("create user test thread");
    let thread = scheduler::thread(slot).expect("lookup user test thread");
    crate::logln!(
        "win32: raw user thread tid={} entry={:#x} slot={} cr3={:#x}",
        thread.tid,
        thread.entry_point,
        slot,
        cr3
    );

    unsafe {
        scheduler::enter_user_mode(slot);
    }
}
