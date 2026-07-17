//! Higher-Half Direct Map (HHDM) support.
//!
//! Limine maps all of physical memory at a single high virtual offset (the
//! HHDM) and does not identity-map physical memory. Kernel code that needs to
//! dereference a physical address as a pointer must first translate it through
//! the HHDM using [`phys_to_virt`].

use core::sync::atomic::{AtomicU64, Ordering};

/// The HHDM virtual base offset, set once during early boot from the Limine
/// HHDM response. Zero until [`set_offset`] is called.
static HHDM_OFFSET: AtomicU64 = AtomicU64::new(0);

/// Record the HHDM offset reported by the bootloader.
///
/// Must be called exactly once before any [`phys_to_virt`] use.
pub fn set_offset(offset: u64) {
    HHDM_OFFSET.store(offset, Ordering::SeqCst);
}

/// Return the active HHDM offset.
pub fn offset() -> u64 {
    HHDM_OFFSET.load(Ordering::SeqCst)
}

/// Translate a physical address to a writable virtual pointer via the HHDM.
///
/// Returns the raw virtual address; callers are responsible for the validity
/// and lifetime of the referenced memory.
pub fn phys_to_virt(phys: u64) -> u64 {
    phys + offset()
}

/// Translate a virtual address inside the HHDM back to its physical address.
///
/// Panics if the address is not within the HHDM range.
pub fn virt_to_phys(virt: u64) -> u64 {
    let off = offset();
    assert!(
        virt >= off,
        "virt_to_phys: address {:#x} is not in HHDM",
        virt
    );
    virt - off
}
