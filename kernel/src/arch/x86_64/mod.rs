//! x86_64 hardware abstraction layer.

use uart_16550::SerialPort;
use x86_64::instructions::port::Port;

pub mod acpi;
pub mod context_switch;
pub mod gdt;
pub mod interrupts;
pub mod ioapic;
pub mod syscall;

/// Initialize x86_64-specific hardware.
pub fn init() {
    crate::logln!("arch: com1");
    unsafe {
        let mut com1 = SerialPort::new(0x3F8);
        com1.init();
    }
    crate::logln!("arch: gdt");
    unsafe {
        gdt::init();
    }
    crate::logln!("arch: interrupts");
    interrupts::init();
    crate::logln!("arch: syscall");
    unsafe {
        syscall::init();
    }
    crate::logln!("arch: done");
}

/// Output a single byte to the debug console.
///
/// Writes to both the ISA debug port (QEMU `-debugcon`) at 0xE9 and the
/// COM1 UART at 0x3F8. The debug port is the most reliable early output path
/// under both BIOS and UEFI firmwares.
pub fn debug_putchar(byte: u8) {
    const THR: u16 = 0x3F8;
    const LCR: u16 = 0x3F8 + 3;
    const LSR: u16 = 0x3F8 + 5;
    const LCR_8N1: u8 = 0x03;
    const LSR_THR_EMPTY: u8 = 0x20;
    unsafe {
        // QEMU `-debugcon file:/tmp/debugcon.log` captures this port.
        let mut debug_port: Port<u8> = Port::new(0xE9);
        debug_port.write(byte);

        // COM1 UART: configure 8N1, then poll THR empty with a timeout so a
        // missing UART does not hang the kernel on real hardware.
        let mut lcr: Port<u8> = Port::new(LCR);
        lcr.write(LCR_8N1);
        let _ = crate::time::poll_with_timeout(10, || {
            let mut lsr: Port<u8> = Port::new(LSR);
            if lsr.read() & LSR_THR_EMPTY != 0 {
                Some(())
            } else {
                None
            }
        });
        let mut thr: Port<u8> = Port::new(THR);
        thr.write(byte);
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

/// Halt the CPU until the next interrupt, then return.
pub fn halt_once() {
    x86_64::instructions::hlt();
}

/// Return a monotonic cycle counter for timeout/heartbeat purposes.
///
/// Uses the Time Stamp Counter.  The TSC is guaranteed to be available in
/// x86_64 long mode, though it may not be invariant on very old CPUs; for
/// simple timeouts that only requires monotonicity within a boot session.
pub fn monotonic_cycles() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

/// Return the nominal TSC frequency in Hz, or 0 if unknown.
pub fn cycles_per_second() -> u64 {
    // Without CPUID leaf 0x15/0x16 support this is a best-guess.  On QEMU and
    // most real hardware 1 GHz is close enough for millisecond-scale timeouts.
    1_000_000_000
}

/// Halt the CPU forever.
pub fn hlt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Power off the machine (Phase 9 system service).
///
/// Writes `SLP_EN | S5` (`0x2000`) to the ACPI PM1a control register at I/O
/// port `0x604`, the standard QEMU/ACPI poweroff. On real hardware the PM1a
/// base is discovered from the ACPI FADT (not yet parsed); `0x604` is the
/// QEMU default and a common PC value. Falls back to halting if the write
/// does not power off.
pub fn shutdown() -> ! {
    x86_64::instructions::interrupts::disable();
    crate::logln!("shutdown: ACPI poweroff (PM1a_CNT <- 0x2000)");
    unsafe {
        let mut pm1a_cnt: Port<u16> = Port::new(0x604);
        pm1a_cnt.write(0x2000);
    }
    // If the platform did not power off, halt forever.
    hlt()
}

/// Run `f` with interrupts disabled, restoring the previous state afterwards.
pub fn without_interrupts<R, F: FnOnce() -> R>(f: F) -> R {
    x86_64::instructions::interrupts::without_interrupts(f)
}
