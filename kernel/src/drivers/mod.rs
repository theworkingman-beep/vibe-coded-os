//! Native device drivers.
//!
//! Drivers run natively on both architectures. Architecture-specific bus
//! access (x86_64 I/O ports here) is kept inside `cfg`-gated modules so the
//! shared driver surface stays portable. Every device is discovered
//! dynamically, never hardcoded, and absence is logged and tolerated.

#[cfg(feature = "arch_x86_64")]
pub mod pci;
#[cfg(feature = "arch_x86_64")]
pub mod rtc;

/// Initialize the driver stack. On x86_64 this enumerates PCI and reads the
/// CMOS RTC; on AArch64 the equivalent discovery (Device Tree / ECAM) is not
/// yet wired, so this is a no-op that logs the state honestly.
pub fn init() {
    #[cfg(feature = "arch_x86_64")]
    {
        pci::enumerate_bus0();
        rtc::log_rtc_time();
    }
    #[cfg(not(feature = "arch_x86_64"))]
    {
        crate::logln!("drivers: no bus discovery on this architecture yet");
    }
}
