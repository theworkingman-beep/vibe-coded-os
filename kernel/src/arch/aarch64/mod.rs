//! AArch64 hardware abstraction layer (stub for cross-architecture build).

pub mod context_switch;
pub mod gdt;
pub mod interrupts;

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

/// Output a single byte via QEMU semihosting.
///
/// The QEMU `virt` machine PL011 UART is in device memory that Limine does not
/// necessarily map into the HHDM, so the easiest reliable debug channel for
/// AArch64 bring-up is semihosting. This writes to the QEMU debug console and
/// will be replaced by a real UART/ framebuffer console once paging is set up.
pub fn debug_putchar(byte: u8) {
    semihost_putchar(byte);
}

/// Output a single byte to the PL011 UART if it is mapped.
///
/// Not currently used because the UART MMIO may not be part of the HHDM.
#[allow(dead_code)]
pub fn pl011_putchar(byte: u8) {
    let base = crate::mm::hhdm::phys_to_virt(PL011_BASE_PHYS as u64) as usize;
    let fr = (base + PL011_FR) as *mut u32;
    let dr = (base + PL011_UARTDR) as *mut u32;
    unsafe {
        // Spin until the transmit FIFO has room.
        while fr.read_volatile() & FR_TXFF != 0 {}
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
