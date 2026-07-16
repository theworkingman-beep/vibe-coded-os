#![allow(static_mut_refs)]

//! Block device abstraction.
//!
//! The kernel sees a small array of block devices.  On x86_64 these are
//! discovered by probing the legacy IDE controller using ATA PIO.  On AArch64
//! no driver is wired yet, so the array is empty.

#[cfg(feature = "arch_x86_64")]
use core::cmp::min;

pub mod ata;

const MAX_DEVICES: usize = 4;

/// Information about a discovered block device.
#[derive(Clone, Copy, Debug)]
pub struct BlockDeviceInfo {
    pub index: usize,
    pub model: [u8; 40],
    pub model_len: usize,
    pub size_sectors: u64,
}

impl BlockDeviceInfo {
    pub fn model(&self) -> &str {
        core::str::from_utf8(&self.model[..self.model_len]).unwrap_or("UNKNOWN")
    }
}

static mut DEVICES: [Option<BlockDeviceInfo>; MAX_DEVICES] = [const { None }; MAX_DEVICES];

/// Initialize the block device subsystem and probe available disks.
pub fn init() {
    #[cfg(feature = "arch_x86_64")]
    ata::probe_disks(unsafe { &mut DEVICES });
}

/// Return the number of detected block devices.
pub fn device_count() -> usize {
    let mut n = 0;
    unsafe {
        for d in DEVICES.iter() {
            if d.is_some() {
                n += 1;
            }
        }
    }
    n
}

/// Return device info by index, if present.
pub fn device_info(index: usize) -> Option<BlockDeviceInfo> {
    unsafe { DEVICES.get(index).copied().flatten() }
}

/// Read `buf.len() / 512` sectors starting at `lba` into `buf`.
/// Returns the number of bytes read.
pub fn read_sectors(device: usize, lba: u64, buf: &mut [u8]) -> Option<usize> {
    #[cfg(not(feature = "arch_x86_64"))]
    {
        let _ = (device, lba, buf);
        return None;
    }
    #[cfg(feature = "arch_x86_64")]
    {
        let info = device_info(device)?;
        let sectors = buf.len() / 512;
        if sectors == 0 {
            return Some(0);
        }
        let sectors = min(sectors, (info.size_sectors - lba) as usize);
        ata::read_sectors(device, lba, &mut buf[..sectors * 512])?;
        Some(sectors * 512)
    }
}

/// Write `buf.len() / 512` sectors starting at `lba` from `buf`.
/// Returns the number of bytes written.
pub fn write_sectors(device: usize, lba: u64, buf: &[u8]) -> Option<usize> {
    #[cfg(not(feature = "arch_x86_64"))]
    {
        let _ = (device, lba, buf);
        return None;
    }
    #[cfg(feature = "arch_x86_64")]
    {
        let info = device_info(device)?;
        let sectors = buf.len() / 512;
        if sectors == 0 {
            return Some(0);
        }
        let sectors = min(sectors, (info.size_sectors - lba) as usize);
        ata::write_sectors(device, lba, &buf[..sectors * 512])?;
        Some(sectors * 512)
    }
}
