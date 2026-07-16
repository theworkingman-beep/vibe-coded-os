//! Minimal I/O APIC driver for routing external IRQs on x86_64.
//!
//! QEMU's `pc` machine delivers legacy interrupts through the I/O APIC.  The
//! firmware often leaves the timer (IRQ0) at vector 0x08, which collides with
//! the CPU's double-fault vector.  We reprogram the I/O APIC so that IRQ0,
//! IRQ1 and IRQ12 are delivered at vectors matching our IDT entries.

use x86_64::instructions::port::Port;
use x86_64::registers::model_specific::{ApicBase, ApicBaseFlags};

use crate::mm::hhdm::phys_to_virt;

const IOAPIC_BASE_PHYS: u64 = 0xFEC0_0000;
const LAPIC_BASE_PHYS: u64 = 0xFEE0_0000;

// Offsets within the MMIO window.
const REGSEL_OFFSET: usize = 0x00;
const WINDOW_OFFSET: usize = 0x10;

// Local APIC register offsets (in bytes).
const LAPIC_TPR_OFFSET: usize = 0x080;
const LAPIC_SVR_OFFSET: usize = 0x0F0;
const LAPIC_EOI_OFFSET: usize = 0x0B0;
const LAPIC_LVT_LINT0_OFFSET: usize = 0x350;
const LAPIC_ENABLE: u32 = 0x100;

const PIC1_COMMAND: u16 = 0x20;
const PIC_EOI: u8 = 0x20;

// Index registers.
const REG_ID: u32 = 0x00;
const REG_VER: u32 = 0x01;
const REDIRECTION_BASE: u32 = 0x10;

/// Driver state for the first I/O APIC.
pub struct IoApic {
    regsel: *mut u32,
    window: *mut u32,
    max_entry: u8,
}

fn ensure_apic_pages_mapped() {
    if let Some(mut pt) = crate::mm::page_table::kernel_page_table() {
        unsafe {
            pt.map_device(phys_to_virt(IOAPIC_BASE_PHYS), IOAPIC_BASE_PHYS);
            pt.map_device(phys_to_virt(LAPIC_BASE_PHYS), LAPIC_BASE_PHYS);
        }
    }
}

impl IoApic {
    /// Map the first I/O APIC and read its version / number of entries.
    ///
    /// # Safety
    ///
    /// Caller must ensure the physical base is mapped through the HHDM and
    /// that only one instance is created.
    pub unsafe fn primary() -> Option<Self> {
        ensure_apic_pages_mapped();
        let base = phys_to_virt(IOAPIC_BASE_PHYS) as *mut u32;
        let regsel = base.add(REGSEL_OFFSET / 4);
        let window = base.add(WINDOW_OFFSET / 4);

        let mut apic = Self {
            regsel,
            window,
            max_entry: 0,
        };

        let ver = apic.read(REG_VER);
        // Version field is bits 0..7; max redirection entry is bits 16..23.
        let max = ((ver >> 16) & 0xFF) as u8;
        if max == 0 {
            return None;
        }
        apic.max_entry = max;
        Some(apic)
    }

    unsafe fn read(&mut self, reg: u32) -> u32 {
        core::ptr::write_volatile(self.regsel, reg);
        core::ptr::read_volatile(self.window)
    }

    unsafe fn write(&mut self, reg: u32, value: u32) {
        core::ptr::write_volatile(self.regsel, reg);
        core::ptr::write_volatile(self.window, value);
    }

    /// Program a redirection entry.
    ///
    /// `irq` is the ISA IRQ number (0..23).  `vector` is the IDT vector.
    /// The entry is unmasked and targeted at the bootstrap processor (APIC ID 0).
    pub fn entry_count(&self) -> u8 {
        self.max_entry + 1
    }

    pub fn route_irq(&mut self, irq: u8, vector: u8) {
        if irq > self.max_entry {
            return;
        }
        let low_index = REDIRECTION_BASE + 2 * irq as u32;
        let high_index = low_index + 1;

        // Read-modify-write the high register to broadcast to all local APICs
        // so we do not have to discover the bootstrap processor's APIC ID.
        let high = unsafe { self.read(high_index) } & 0x00FFFFFF;
        unsafe {
            self.write(high_index, high | (0xFFu32 << 24));
        }

        // Low register: fixed delivery, physical destination, active high,
        // edge triggered, unmasked, with the requested vector.
        let low = (vector as u32) & 0xFF;
        unsafe {
            self.write(low_index, low);
            let low_after = self.read(low_index);
            let high_after = self.read(high_index);
            crate::logln!(
                "ioapic: irq{} routed low={:#x} high={:#x}",
                irq,
                low_after,
                high_after
            );
        }
    }
}

/// Mask every IRQ on the legacy 8259 PICs so they do not conflict with the
/// I/O APIC.  This is safe to call even if the I/O APIC is not present.
pub fn disable_pic() {
    unsafe {
        let mut pic1_data: Port<u8> = Port::new(0x21);
        let mut pic2_data: Port<u8> = Port::new(0xA1);
        pic1_data.write(0xFF);
        pic2_data.write(0xFF);
    }
}

/// Enable the bootstrap local APIC so it can receive interrupts delivered
/// by the I/O APIC.  The firmware usually leaves it enabled, but Limine may
/// have put it in a state where external interrupts are ignored.
pub fn enable_lapic() {
    ensure_apic_pages_mapped();
    unsafe {
        // Ensure the local APIC is globally enabled in the APIC_BASE MSR.
        let (frame, raw) = ApicBase::read_raw();
        crate::logln!("lapic: apic_base raw={:#x}", raw);
        let new_flags = (raw & !ApicBaseFlags::X2APIC_ENABLE.bits())
            | ApicBaseFlags::LAPIC_ENABLE.bits();
        ApicBase::write_raw(frame, new_flags);

        let base = phys_to_virt(LAPIC_BASE_PHYS) as *mut u32;
        crate::logln!("lapic: id reg={:#x}", core::ptr::read_volatile(base.add(0x020 / 4)));
        // Task Priority Register: priority 0 (accept all interrupts).
        core::ptr::write_volatile(base.add(LAPIC_TPR_OFFSET / 4), 0);
        // Spurious Interrupt Vector Register: enable APIC, vector 0xFF.
        let svr = core::ptr::read_volatile(base.add(LAPIC_SVR_OFFSET / 4));
        core::ptr::write_volatile(base.add(LAPIC_SVR_OFFSET / 4), svr | LAPIC_ENABLE | 0xFF);
        // LINT0: accept interrupts from the legacy 8259 PIC (ExtINT mode).
        core::ptr::write_volatile(base.add(LAPIC_LVT_LINT0_OFFSET / 4), 0x700);
    }
}

/// Send an end-of-interrupt to the local APIC (for interrupts delivered
/// through the I/O APIC) and, for safety, also to the legacy 8259 PIC.
///
/// This is `extern "C"` so it can be called from the naked timer handler.
pub extern "C" fn eoi() {
    unsafe {
        let base = phys_to_virt(LAPIC_BASE_PHYS) as *mut u32;
        core::ptr::write_volatile(base.add(LAPIC_EOI_OFFSET / 4), 0);
        let mut pic1_command: Port<u8> = Port::new(PIC1_COMMAND);
        pic1_command.write(PIC_EOI);
    }
}
