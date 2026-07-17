//! AArch64 exception vector table and minimal IRQ dispatch.
//!
//! Provides a small EL1 vector table used to field the architectural timer
//! IRQ.  All other exceptions are logged and the CPU is halted for
//! diagnostics.

/// QEMU `virt` GICv2 distributor and CPU interface physical base addresses.
const GICD_BASE_PHYS: u64 = 0x0800_0000;
const GICC_BASE_PHYS: u64 = 0x0801_0000;

// GICv2 distributor register offsets (bytes).
const GICD_CTLR: usize = 0x000;
const GICD_ISENABLER: usize = 0x100;
const GICD_ICENABLER: usize = 0x180;
const GICD_IPRIORITYR: usize = 0x400;

// GICv2 CPU interface register offsets (bytes).
const GICC_CTLR: usize = 0x000;
const GICC_PMR: usize = 0x004;
const GICC_BPR: usize = 0x008;
const GICC_IAR: usize = 0x00C;

/// AArch64 virtual-timer PPI number.
const TIMER_IRQ: u32 = 27;

fn hhdm_ptr(phys: u64) -> *mut u32 {
    crate::mm::hhdm::phys_to_virt(phys) as *mut u32
}

unsafe fn gicd_write(offset: usize, value: u32) {
    core::ptr::write_volatile(hhdm_ptr(GICD_BASE_PHYS).add(offset / 4), value);
}

unsafe fn gicd_read(offset: usize) -> u32 {
    core::ptr::read_volatile(hhdm_ptr(GICD_BASE_PHYS).add(offset / 4))
}

unsafe fn gicc_write(offset: usize, value: u32) {
    core::ptr::write_volatile(hhdm_ptr(GICC_BASE_PHYS).add(offset / 4), value);
}

unsafe fn gicc_read(offset: usize) -> u32 {
    core::ptr::read_volatile(hhdm_ptr(GICC_BASE_PHYS).add(offset / 4))
}

/// Common IRQ dispatch.  Returns the acknowledged IRQ number in x0 for the
/// assembly wrapper to use when writing the end-of-interrupt register.
#[no_mangle]
unsafe extern "C" fn aarch64_irq_dispatch() -> u32 {
    let irq = gicc_read(GICC_IAR) & 0x3FF;
    if irq == TIMER_IRQ {
        let freq = crate::arch::cycles_per_second();
        let ticks = if freq == 0 { 1_000_000u64 } else { freq / 100 };
        core::arch::asm!(
            "msr cntv_tval_el0, {0}",
            in(reg) ticks,
            options(nomem, nostack)
        );
        crate::logln!("aarch64: timer tick");
    } else if irq < 1020 {
        crate::logln!("aarch64: unexpected IRQ {}", irq);
    }
    irq
}

/// Synchronous exception handler.  Logs and halts.
#[no_mangle]
unsafe extern "C" fn aarch64_sync_handler() {
    let elr: u64;
    let esr: u64;
    let far: u64;
    core::arch::asm!(
        "mrs {0}, elr_el1",
        "mrs {1}, esr_el1",
        "mrs {2}, far_el1",
        out(reg) elr,
        out(reg) esr,
        out(reg) far,
        options(nomem, nostack)
    );
    crate::logln!(
        "aarch64 sync exception at {:#x} esr={:#x} far={:#x}",
        elr,
        esr,
        far
    );
    crate::hlt();
}

// Assembly helpers referenced by the vector table below.
extern "C" {
    fn aarch64_irq_entry();
    fn aarch64_sync_entry();
}

core::arch::global_asm!(
    ".balign 2048",
    ".global aarch64_exception_vectors",
    "aarch64_exception_vectors:",
    // Current EL, SP_EL0: shouldn't happen; spin.
    "b aarch64_halt_entry\n .balign 0x80\n",
    "b aarch64_halt_entry\n .balign 0x80\n",
    "b aarch64_halt_entry\n .balign 0x80\n",
    "b aarch64_halt_entry\n .balign 0x80\n",
    // Current EL, SP_EL1: sync, IRQ, FIQ, SError.
    "b aarch64_sync_entry\n .balign 0x80\n",
    "b aarch64_irq_entry\n .balign 0x80\n",
    "b aarch64_halt_entry\n .balign 0x80\n",
    "b aarch64_halt_entry\n .balign 0x80\n",
    // Lower EL, AArch64: not used.
    "b aarch64_halt_entry\n .balign 0x80\n",
    "b aarch64_halt_entry\n .balign 0x80\n",
    "b aarch64_halt_entry\n .balign 0x80\n",
    "b aarch64_halt_entry\n .balign 0x80\n",
    // Lower EL, AArch32: not used.
    "b aarch64_halt_entry\n .balign 0x80\n",
    "b aarch64_halt_entry\n .balign 0x80\n",
    "b aarch64_halt_entry\n .balign 0x80\n",
    "b aarch64_halt_entry\n .balign 0x80\n",

    // Naked IRQ entry.
    ".balign 0x10",
    ".global aarch64_irq_entry",
    "aarch64_irq_entry:",
    "sub sp, sp, #160",
    "stp x0, x1, [sp, #0 * 16]",
    "stp x2, x3, [sp, #1 * 16]",
    "stp x4, x5, [sp, #2 * 16]",
    "stp x6, x7, [sp, #3 * 16]",
    "stp x8, x9, [sp, #4 * 16]",
    "stp x10, x11, [sp, #5 * 16]",
    "stp x12, x13, [sp, #6 * 16]",
    "stp x14, x15, [sp, #7 * 16]",
    "stp x16, x17, [sp, #8 * 16]",
    "str x18, [sp, #9 * 16]",
    "str x30, [sp, #9 * 16 + 8]",
    "bl aarch64_irq_dispatch",
    // x0 = IRQ id; write ICC_EOIR0_EL1.
    "msr s3_0_c12_c12_1, x0",
    "ldp x0, x1, [sp, #0 * 16]",
    "ldp x2, x3, [sp, #1 * 16]",
    "ldp x4, x5, [sp, #2 * 16]",
    "ldp x6, x7, [sp, #3 * 16]",
    "ldp x8, x9, [sp, #4 * 16]",
    "ldp x10, x11, [sp, #5 * 16]",
    "ldp x12, x13, [sp, #6 * 16]",
    "ldp x14, x15, [sp, #7 * 16]",
    "ldp x16, x17, [sp, #8 * 16]",
    "ldr x18, [sp, #9 * 16]",
    "ldr x30, [sp, #9 * 16 + 8]",
    "add sp, sp, #160",
    "eret",

    // Naked synchronous exception entry: logs and halts.
    ".balign 0x10",
    ".global aarch64_sync_entry",
    "aarch64_sync_entry:",
    "sub sp, sp, #160",
    "stp x0, x1, [sp, #0 * 16]",
    "stp x2, x3, [sp, #1 * 16]",
    "stp x4, x5, [sp, #2 * 16]",
    "stp x6, x7, [sp, #3 * 16]",
    "stp x8, x9, [sp, #4 * 16]",
    "stp x10, x11, [sp, #5 * 16]",
    "stp x12, x13, [sp, #6 * 16]",
    "stp x14, x15, [sp, #7 * 16]",
    "stp x16, x17, [sp, #8 * 16]",
    "str x18, [sp, #9 * 16]",
    "str x30, [sp, #9 * 16 + 8]",
    "bl aarch64_sync_handler",

    // Halt entry used by FIQ/SError/spurious vectors.
    ".balign 0x10",
    ".global aarch64_halt_entry",
    "aarch64_halt_entry:",
    "b {halt}",
    halt = sym crate::hlt,
);

/// Initialize GICv2 and the virtual timer, then install the EL1 vector table.
///
/// # Safety
///
/// Must be called once from EL1 on the bootstrap CPU.
pub unsafe fn init() {
    // Install the 2048-byte aligned vector table first.  The GIC and virtual
    // timer setup is disabled on this branch because accessing the GIC MMIO
    // region through the HHDM faults on some firmware configurations; it will
    // be re-enabled once the fault is root-caused.
    let vbar = aarch64_exception_vectors as *const () as u64;
    crate::logln!("aarch64: vector table at {:#x}", vbar);
    core::arch::asm!(
        "msr vbar_el1, {0}",
        "isb",
        in(reg) vbar,
        options(nomem, nostack)
    );
    crate::logln!("aarch64: vector table installed");

    if false {
        // Enable distributor and CPU interface.
        gicd_write(GICD_CTLR, 1);
        crate::logln!("aarch64: GIC distributor enabled");
        gicc_write(GICC_CTLR, 1);
        crate::logln!("aarch64: GIC CPU interface enabled");
        gicc_write(GICC_PMR, 0xFF);
        gicc_write(GICC_BPR, 0);

        let timer_id = TIMER_IRQ as usize;

        // Set priority to a middle value (byte within the 32-bit word).
        let pri_offset = GICD_IPRIORITYR + timer_id;
        let pri_word = (pri_offset / 4) * 4;
        let pri_byte = timer_id % 4;
        let pri_mask = 0xFFu32 << (pri_byte * 8);
        let pri_value = (0x80u32) << (pri_byte * 8);
        let pri_current = gicd_read(pri_word);
        gicd_write(pri_word, (pri_current & !pri_mask) | pri_value);

        // Enable the timer PPI.
        let enable_word = timer_id / 32;
        let enable_bit = timer_id % 32;
        gicd_write(GICD_ISENABLER + enable_word * 4, 1u32 << enable_bit);
        crate::logln!("aarch64: timer PPI enabled");

        // Load 10 ms into the virtual timer and enable it.
        let freq = crate::arch::cycles_per_second();
        let ticks = if freq == 0 { 1_000_000u64 } else { freq / 100 };
        core::arch::asm!(
            "msr cntv_tval_el0, {0}",
            "msr cntv_ctl_el0, {1}",
            in(reg) ticks,
            in(reg) 1u64,
            options(nomem, nostack)
        );
        crate::logln!("aarch64: timer armed ({} ticks)", ticks);
    }
}

// Reference to the aligned vector table symbol.
extern "C" {
    fn aarch64_exception_vectors();
}
