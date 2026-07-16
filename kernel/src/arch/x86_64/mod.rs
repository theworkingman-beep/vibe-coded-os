//! x86_64 hardware abstraction layer.

use uart_16550::SerialPort;
use x86_64::instructions::port::Port;

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

        // COM1 UART: poll THR empty and send.
        let mut lcr: Port<u8> = Port::new(LCR);
        lcr.write(LCR_8N1);
        let mut lsr: Port<u8> = Port::new(LSR);
        while lsr.read() & LSR_THR_EMPTY == 0 {}
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

/// Halt the CPU forever.
pub fn hlt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Run `f` with interrupts disabled, restoring the previous state afterwards.
pub fn without_interrupts<R, F: FnOnce() -> R>(f: F) -> R {
    x86_64::instructions::interrupts::without_interrupts(f)
}
