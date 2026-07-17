#![no_std]
#![cfg_attr(all(feature = "arch_x86_64", test), no_main)]
#![cfg_attr(feature = "arch_x86_64", feature(abi_x86_interrupt))]
// ApertureOS is an in-development OS kernel. A number of register constants,
// driver methods, and ABI fields are defined ahead of the phases that wire
// them up; silence dead-code lints crate-wide rather than scattering
// per-item allows across the scaffolding.
#![allow(dead_code)]

extern crate alloc;

pub mod arch;
pub mod boot_info;
pub mod disk;
pub mod gui;
pub mod installer;
pub mod logger;
pub mod mm;
pub mod panic;
pub mod time;
pub mod vfs;
pub mod win32;

/// Kernel initialization entry common to all architectures.
pub fn init() {
    #[cfg(feature = "arch_x86_64")]
    x86_64::instructions::interrupts::disable();

    logger::init();
    logln!("init: logger");
    arch::init();
    #[cfg(feature = "arch_x86_64")]
    x86_64::instructions::interrupts::disable();
    logln!("init: arch done");
    mm::init();
    logln!("init: mm done");
    disk::init();
    logln!("init: disk done ({} devices)", disk::device_count());
    vfs::init();
    logln!("init: vfs done");
    gui::init();
    logln!("init: gui done");
    win32::init();
    logln!("init: win32 done");

    #[cfg(feature = "arch_x86_64")]
    x86_64::instructions::interrupts::enable();
}

/// Halt the CPU forever.
pub fn hlt() -> ! {
    arch::hlt()
}
