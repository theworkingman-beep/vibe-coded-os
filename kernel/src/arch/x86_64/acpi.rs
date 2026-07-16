//! Minimal ACPI table parser for x86_64.
//!
//! The parser relies on Limine to provide the physical address of the Root
//! System Description Pointer (RSDP).  It walks the RSDT (ACPI 1.0) or XSDT
//! (ACPI 2.0+) and validates table checksums.  Only the table headers are
//! parsed here; individual tables such as the MADT are left to later phases.

use crate::logln;
use crate::mm::hhdm::phys_to_virt;

/// Raw RSDP layout (ACPI 1.0 / 2.0).
#[repr(C, packed)]
struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    // ACPI 2.0+ fields follow when revision >= 2.
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    _reserved: [u8; 3],
}

fn read_u8(ptr: *const u8) -> u8 {
    unsafe { core::ptr::read_unaligned(ptr) }
}

fn read_u32(ptr: *const u8) -> u32 {
    unsafe { core::ptr::read_unaligned(ptr as *const u32) }
}

fn read_u64(ptr: *const u8) -> u64 {
    unsafe { core::ptr::read_unaligned(ptr as *const u64) }
}

fn bytes_checksum(bytes: &[u8]) -> u8 {
    let sum: u64 = bytes.iter().map(|b| *b as u64).sum();
    (sum & 0xff) as u8
}

fn validate_checksum(bytes: &[u8]) -> bool {
    bytes_checksum(bytes) == 0
}

/// Initialize ACPI parsing from the provided RSDP virtual address.
///
/// The RSDP pointer is virtual because Limine (base revision >= 4) returns a
/// virtual address.  The addresses embedded inside the RSDP (RSDT/XSDT) are
/// physical and are translated through the HHDM.
///
/// # Safety
///
/// `rsdp_virt` must point to a valid ACPI RSDP structure.
pub unsafe fn init(rsdp_virt: u64) {
    let rsdp_virt = rsdp_virt as *const Rsdp;
    let signature = core::slice::from_raw_parts(
        core::ptr::addr_of!((*rsdp_virt).signature) as *const u8,
        8,
    );
    if signature != b"RSD PTR " {
        logln!("acpi: invalid RSDP signature");
        return;
    }

    let revision = read_u8(core::ptr::addr_of!((*rsdp_virt).revision) as *const u8);
    let oem = core::slice::from_raw_parts(
        core::ptr::addr_of!((*rsdp_virt).oem_id) as *const u8,
        6,
    );
    logln!("acpi: RSDP revision={} oem={:?}", revision, oem);

    let valid = if revision >= 2 {
        let len = read_u32(core::ptr::addr_of!((*rsdp_virt).length) as *const u8);
        let bytes = core::slice::from_raw_parts(rsdp_virt as *const u8, len as usize);
        validate_checksum(bytes)
    } else {
        let bytes = core::slice::from_raw_parts(rsdp_virt as *const u8, 20);
        validate_checksum(bytes)
    };
    if !valid {
        logln!("acpi: RSDP checksum invalid");
        return;
    }

    if revision >= 2 {
        let xsdt_phys = read_u64(core::ptr::addr_of!((*rsdp_virt).xsdt_address) as *const u8);
        parse_sdt_root(xsdt_phys, true);
    } else {
        let rsdt_phys = read_u32(core::ptr::addr_of!((*rsdp_virt).rsdt_address) as *const u8) as u64;
        parse_sdt_root(rsdt_phys, false);
    }
}

fn sdt_header(virt: usize) -> Option<([u8; 4], u32, u8)> {
    let sig = unsafe { core::ptr::read_unaligned(virt as *const [u8; 4]) };
    let len = unsafe { core::ptr::read_unaligned((virt + 4) as *const u32) };
    let revision = unsafe { core::ptr::read_unaligned((virt + 8) as *const u8) };
    let bytes = unsafe { core::slice::from_raw_parts(virt as *const u8, len as usize) };
    if !validate_checksum(bytes) {
        return None;
    }
    Some((sig, len, revision))
}

fn parse_sdt_root(root_phys: u64, is_xsdt: bool) {
    let virt = phys_to_virt(root_phys) as usize;
    let Some((sig, len, revision)) = sdt_header(virt) else {
        logln!("acpi: {} checksum invalid", if is_xsdt { "XSDT" } else { "RSDT" });
        return;
    };
    let kind = if is_xsdt { "XSDT" } else { "RSDT" };
    logln!("acpi: {} sig={:?} len={} revision={}", kind, sig, len, revision);

    let entries_start = virt + 36;
    let entry_size = if is_xsdt { 8 } else { 4 };
    let entry_count = (len as usize - 36) / entry_size;
    logln!("acpi: {} entries={}", kind, entry_count);
    for i in 0..entry_count {
        let entry_ptr = (entries_start + i * entry_size) as *const u8;
        let table_phys = if is_xsdt {
            read_u64(entry_ptr)
        } else {
            read_u32(entry_ptr) as u64
        };
        describe_table(table_phys);
    }
}

fn describe_table(table_phys: u64) {
    let virt = phys_to_virt(table_phys) as usize;
    let Some((sig, len, revision)) = sdt_header(virt) else {
        logln!("acpi: table at {:#x} checksum invalid", table_phys);
        return;
    };
    logln!(
        "acpi: table sig={:?} len={} revision={}",
        sig,
        len,
        revision
    );
}
