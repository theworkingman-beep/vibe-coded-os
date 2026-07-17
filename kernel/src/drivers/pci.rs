//! PCI configuration-space enumeration (x86_64 legacy I/O ports).
//!
//! Scans bus 0 using the type-0 configuration-access mechanism (I/O ports
//! `0xCF8` address / `0xCFC` data) and logs every present device's
//! vendor/device/class codes. This is the discovery foundation for Phase 8
//! storage, network, and graphics drivers; device addresses are never
//! hardcoded — they come from the enumeration.

use x86_64::instructions::port::Port;

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

/// Read a 32-bit PCI configuration word at `(bus, dev, func, offset)`.
/// `offset` must be 4-byte aligned.
fn config_read(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    let address = 0x8000_0000
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xFC);
    unsafe {
        let mut addr: Port<u32> = Port::new(CONFIG_ADDRESS);
        addr.write(address);
        let mut data: Port<u32> = Port::new(CONFIG_DATA);
        data.read()
    }
}

/// A discovered PCI device on bus 0.
#[derive(Clone, Copy, Debug)]
pub struct PciDevice {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor: u16,
    pub device: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub revision: u8,
    pub header_type: u8,
}

impl PciDevice {
    /// Base-class name for logging.
    pub fn class_name(&self) -> &'static str {
        match self.class_code {
            0x00 => "Unclassified",
            0x01 => "Mass storage",
            0x02 => "Network",
            0x03 => "Display",
            0x04 => "Multimedia",
            0x06 => "Bridge",
            0x08 => "Generic system peripheral",
            0x09 => "Input device",
            0x0C => "Serial bus",
            _ => "Other",
        }
    }
}

const INVALID_VENDOR: u16 = 0xFFFF;

/// Enumerate all devices on PCI bus 0 (32 devices, 8 functions each) and
/// log each present one. Multi-function devices are followed. Returns the
/// count of devices found.
pub fn enumerate_bus0() -> usize {
    let mut count = 0usize;
    crate::logln!("pci: enumerating bus 0");
    for dev in 0..32u8 {
        let mut funcs = 0u8;
        for func in 0..8u8 {
            let vendor_device = config_read(0, dev, func, 0x00);
            let vendor = (vendor_device & 0xFFFF) as u16;
            let device = (vendor_device >> 16) as u16;
            if vendor == INVALID_VENDOR || vendor == 0 {
                continue;
            }
            let class_rev = config_read(0, dev, func, 0x08);
            let revision = (class_rev & 0xFF) as u8;
            let prog_if = ((class_rev >> 8) & 0xFF) as u8;
            let subclass = ((class_rev >> 16) & 0xFF) as u8;
            let class_code = ((class_rev >> 24) & 0xFF) as u8;
            let header_type_raw = config_read(0, dev, func, 0x0C);
            let header_type = ((header_type_raw >> 16) & 0xFF) as u8;

            let d = PciDevice {
                bus: 0,
                dev,
                func,
                vendor,
                device,
                class_code,
                subclass,
                prog_if,
                revision,
                header_type,
            };
            crate::logln!(
                "pci: 00:{:02x}.{} {:#06x}:{:#06x} {} (class {:#x}/{:#x} rev {:#x} hdr {:#x})",
                dev,
                func,
                vendor,
                device,
                d.class_name(),
                class_code,
                subclass,
                revision,
                header_type & 0x7F
            );
            count += 1;
            funcs += 1;
            // Multi-function devices have bit 7 of header type set on func 0.
            if func == 0 && (header_type & 0x80) == 0 {
                break;
            }
        }
        let _ = funcs;
    }
    crate::logln!("pci: {} device(s) on bus 0", count);
    count
}
