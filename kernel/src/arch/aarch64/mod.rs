//! AArch64 hardware abstraction layer (stub for cross-architecture build).

pub mod context_switch;
pub mod gdt;
pub mod interrupts;
pub mod vectors;

/// QEMU "virt" machine PL011 UART base physical address.
const PL011_BASE_PHYS: usize = 0x0900_0000;
const PL011_UARTDR: usize = 0x00;
const PL011_FR: usize = 0x18;
const FR_TXFF: u32 = 1 << 5;

/// Initialize AArch64-specific hardware.
pub fn init() {
    // The PL011 UART on the QEMU virt machine is usable without explicit
    // baud/divisor configuration; the firmware already set it up. Real
    // hardware bring-up will configure GIC, timers, and the MMU here.
    unsafe {
        vectors::init();
        // Keep IRQs masked for now; the GIC/timer bring-up is still being
        // debugged on real hardware and the current build does not need
        // preemptive interrupts for the installer/GUI tests.
        // core::arch::asm!("msr daifclr, #0x3", options(nomem, nostack));
    }
}

/// Output a single byte via QEMU semihosting for early bring-up.
///
/// This works before the UART is known to be mapped and is the only reliable
/// way to see output from very early AArch64 boot.
#[inline(never)]
pub fn semihost_putchar(byte: u8) {
    unsafe {
        core::arch::asm!(
            "hlt #0xf000",
            in("x0") 0x03u64,                    // SYS_WRITEC
            in("x1") (&byte as *const u8) as u64,
            options(nomem, nostack)
        );
    }
}

/// Output a single byte to the serial console.
///
/// On QEMU `virt` we use the semihosting `SYS_WRITEC` channel: Limine's
/// higher-half direct map covers RAM but does *not* map device MMIO such as
/// the PL011 UART at `0x0900_0000`, so writing the PL011 through the HHDM
/// would data-abort before the kernel's own MMU is programmed. Semihosting
/// is the reliable early boot channel under QEMU and appears on the host
/// console. `pl011_putchar` is retained for the real-hardware path: once
/// the AArch64 MMU is programmed to map the UART MMIO region (Phase 1B
/// ongoing work), `debug_putchar` can switch to it for non-QEMU targets.
pub fn debug_putchar(byte: u8) {
    semihost_putchar(byte);
}

/// Output a single byte to the PL011 UART if it is mapped.
///
/// Uses a short timeout so a missing or unmapped PL011 does not hang the
/// kernel on real hardware.
#[allow(dead_code)]
pub fn pl011_putchar(byte: u8) {
    let base = crate::mm::hhdm::phys_to_virt(PL011_BASE_PHYS as u64) as usize;
    let fr = (base + PL011_FR) as *mut u32;
    let dr = (base + PL011_UARTDR) as *mut u32;
    unsafe {
        let _ = crate::time::poll_with_timeout(10, || {
            if fr.read_volatile() & FR_TXFF == 0 {
                Some(())
            } else {
                None
            }
        });
        dr.write_volatile(byte as u32);
    }
}

/// Return the current mouse cursor position.
pub fn mouse_position() -> (i32, i32) {
    interrupts::mouse_position()
}

/// Return the current mouse button state.
pub fn mouse_buttons() -> u8 {
    interrupts::mouse_buttons()
}

/// Run `f` with interrupts disabled, restoring the previous state afterwards.
///
/// AArch64: mask IRQ and FIQ around the closure. A real implementation will
/// also need to mask the GIC CPU interface, but this is sufficient for the
/// single-core bring-up used by the installer/GUI tests.
pub fn without_interrupts<R>(f: impl FnOnce() -> R) -> R {
    unsafe {
        let saved: u64;
        core::arch::asm!("mrs {0}, daif", out(reg) saved);
        core::arch::asm!("msr daifset, #0x3", options(nomem, nostack));
        let result = f();
        core::arch::asm!("msr daif, {0}", in(reg) saved, options(nomem, nostack));
        result
    }
}

/// Return a monotonic cycle counter for timeout/heartbeat purposes.
///
/// Reads the architected virtual counter.  CNTVCT is available at EL1 and is
/// configured by firmware; if it is not running this will return a static
/// value and software timeouts will not advance.
pub fn monotonic_cycles() -> u64 {
    let cntvct: u64;
    unsafe {
        core::arch::asm!("mrs {0}, cntvct_el0", out(reg) cntvct, options(nomem, nostack));
    }
    cntvct
}

/// Return the nominal counter frequency in Hz, or 0 if unknown.
pub fn cycles_per_second() -> u64 {
    let cntfrq: u64;
    unsafe {
        core::arch::asm!("mrs {0}, cntfrq_el0", out(reg) cntfrq, options(nomem, nostack));
    }
    cntfrq
}

/// Halt the CPU until the next interrupt, then return.
pub fn halt_once() {
    unsafe { core::arch::asm!("wfe", options(nomem, nostack)) };
}

/// Halt the CPU forever.
pub fn hlt() -> ! {
    loop {
        halt_once();
    }
}

/// Power off the machine (Phase 9 system service).
///
/// Issues a PSCI `SYSTEM_OFF` (`0x8400_0008`) hypercall via `HVC #0`. QEMU's
/// `virt` machine and most AArch64 firmwares implement PSCI, so this powers
/// off without requiring semihosting. Falls back to halting if PSCI is not
/// present.
pub fn shutdown() -> ! {
    crate::logln!("shutdown: PSCI SYSTEM_OFF");
    unsafe {
        core::arch::asm!(
            "hvc #0",
            in("x0") 0x8400_0008u64,
            options(nomem, nostack)
        );
    }
    hlt()
}
