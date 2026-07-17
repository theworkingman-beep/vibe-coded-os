//! ATA PIO driver for legacy IDE controllers.
//!
//! Only the primary and secondary channels are probed.  This is enough for
//! QEMU's PIIX3 IDE controller and many real PCs in legacy mode.

#[cfg(feature = "arch_x86_64")]
use x86_64::instructions::port::Port;

use super::BlockDeviceInfo;

/// Channel/port pair.
#[cfg(feature = "arch_x86_64")]
struct Channel {
    base: u16,
    control: u16,
}

#[cfg(feature = "arch_x86_64")]
const PRIMARY: Channel = Channel {
    base: 0x1F0,
    control: 0x3F6,
};
#[cfg(feature = "arch_x86_64")]
const SECONDARY: Channel = Channel {
    base: 0x170,
    control: 0x376,
};

#[cfg(feature = "arch_x86_64")]
fn select_device(channel: &Channel, master: bool) {
    let mut dev: Port<u8> = Port::new(channel.base + 6);
    unsafe { dev.write(if master { 0xA0 } else { 0xB0 }) };
}

/// Wait until BSY is clear and RDY is set.  Returns the status byte.
#[cfg(feature = "arch_x86_64")]
fn wait_status(channel: &Channel, timeout_us: u32) -> Option<u8> {
    let mut status: Port<u8> = Port::new(channel.base + 7);
    for _ in 0..timeout_us {
        let s = unsafe { status.read() };
        if (s & 0x80) == 0 && (s & 0x40) != 0 {
            return Some(s);
        }
        tiny_delay();
    }
    None
}

/// Wait until DRQ is set (or ERR is set).  Returns the status byte.
#[cfg(feature = "arch_x86_64")]
fn wait_drq(channel: &Channel, timeout_us: u32) -> Option<u8> {
    let mut status: Port<u8> = Port::new(channel.base + 7);
    for _ in 0..timeout_us {
        let s = unsafe { status.read() };
        if (s & 0x01) != 0 {
            return None; // error
        }
        if (s & 0x08) != 0 {
            return Some(s);
        }
        tiny_delay();
    }
    None
}

#[cfg(feature = "arch_x86_64")]
fn tiny_delay() {
    // A handful of port reads is enough for ATA PIO timing on emulated and
    // most real hardware.  Reading the alternate status port is the classic
    // way to delay ~400ns.
    let mut alt: Port<u8> = Port::new(PRIMARY.control);
    unsafe {
        alt.read();
    }
}

#[cfg(feature = "arch_x86_64")]
fn identify(channel: &Channel, master: bool) -> Option<BlockDeviceInfo> {
    select_device(channel, master);
    // Wait for the previous command to finish.
    wait_status(channel, 1000)?;

    // Send IDENTIFY (0xEC).
    let mut cmd: Port<u8> = Port::new(channel.base + 7);
    unsafe { cmd.write(0xEC) };

    let status = wait_status(channel, 10000)?;
    if status == 0 {
        // No device on this selector.
        return None;
    }

    // Wait for DRQ or error.
    wait_drq(channel, 10000)?;

    let mut data: Port<u16> = Port::new(channel.base + 0);
    let mut buf = [0u16; 256];
    for i in 0..256 {
        buf[i] = unsafe { data.read() };
    }

    // Model string is at words 27..46, byte-swapped.
    let mut model = [0u8; 40];
    for (i, word) in buf[27..47].iter().enumerate() {
        let bytes = word.to_be_bytes();
        model[i * 2] = bytes[0];
        model[i * 2 + 1] = bytes[1];
    }
    // Trim trailing spaces.
    let mut model_len = 40;
    while model_len > 0 && model[model_len - 1] == b' ' {
        model_len -= 1;
    }

    let lba_sectors = ((buf[61] as u32) << 16) | (buf[60] as u32);
    crate::logln!(
        "ata: {} {} sectors ({} MiB)",
        if master { "master" } else { "slave" },
        lba_sectors,
        lba_sectors * 512 / 1024 / 1024
    );
    Some(BlockDeviceInfo {
        index: 0,
        model,
        model_len,
        size_sectors: lba_sectors as u64,
    })
}

/// Probe both IDE channels and fill `out` with up to four devices.
#[cfg(feature = "arch_x86_64")]
pub fn probe_disks(out: &mut [Option<BlockDeviceInfo>; 4]) {
    let channels = [&PRIMARY, &SECONDARY];
    let mut idx = 0;
    for ch_i in 0..channels.len() {
        let ch = channels[ch_i];
        for master_i in 0..2 {
            let master = master_i == 0;
            if let Some(mut info) = identify(ch, master) {
                info.index = idx;
                out[idx] = Some(info);
                idx += 1;
                if idx >= out.len() {
                    return;
                }
            }
        }
    }
}

/// Read sectors using 28-bit LBA PIO.
#[cfg(feature = "arch_x86_64")]
pub fn read_sectors(device: usize, lba: u64, buf: &mut [u8]) -> Option<()> {
    let (channel, master) = device_channel(device)?;
    if lba >= 0x0FFF_FFFF || buf.len() % 512 != 0 {
        return None;
    }
    let sectors = (buf.len() / 512) as u8;

    select_device(channel, master);
    wait_status(channel, 1000)?;

    unsafe {
        Port::<u8>::new(channel.base + 2).write(sectors);
        Port::<u8>::new(channel.base + 3).write((lba & 0xFF) as u8);
        Port::<u8>::new(channel.base + 4).write(((lba >> 8) & 0xFF) as u8);
        Port::<u8>::new(channel.base + 5).write(((lba >> 16) & 0xFF) as u8);
        Port::<u8>::new(channel.base + 6)
            .write((if master { 0xE0 } else { 0xF0 }) | (((lba >> 24) & 0x0F) as u8));
        Port::<u8>::new(channel.base + 7).write(0x20);
    }

    let mut data_port: Port<u16> = Port::new(channel.base + 0);
    let mut offset = 0usize;
    for _ in 0..sectors {
        wait_drq(channel, 10000)?;
        for _ in 0..256 {
            let word = unsafe { data_port.read() };
            buf[offset] = word as u8;
            buf[offset + 1] = (word >> 8) as u8;
            offset += 2;
        }
    }
    Some(())
}

/// Write sectors using 28-bit LBA PIO.
#[cfg(feature = "arch_x86_64")]
pub fn write_sectors(device: usize, lba: u64, buf: &[u8]) -> Option<()> {
    let (channel, master) = device_channel(device)?;
    if lba >= 0x0FFF_FFFF || buf.len() % 512 != 0 {
        return None;
    }
    let sectors = (buf.len() / 512) as u8;

    select_device(channel, master);
    wait_status(channel, 1000)?;

    unsafe {
        Port::<u8>::new(channel.base + 2).write(sectors);
        Port::<u8>::new(channel.base + 3).write((lba & 0xFF) as u8);
        Port::<u8>::new(channel.base + 4).write(((lba >> 8) & 0xFF) as u8);
        Port::<u8>::new(channel.base + 5).write(((lba >> 16) & 0xFF) as u8);
        Port::<u8>::new(channel.base + 6)
            .write((if master { 0xE0 } else { 0xF0 }) | (((lba >> 24) & 0x0F) as u8));
        Port::<u8>::new(channel.base + 7).write(0x30);
    }

    // Wait for the device to accept the write command and ask for data.
    wait_drq(channel, 10000)?;

    let mut data_port: Port<u16> = Port::new(channel.base + 0);
    let mut offset = 0usize;
    for i in 0..sectors {
        for _ in 0..256 {
            let word = (buf[offset] as u16) | ((buf[offset + 1] as u16) << 8);
            unsafe { data_port.write(word) };
            offset += 2;
        }
        // After each sector except the last, wait for DRQ before writing the next one.
        if (i as usize) + 1 < sectors as usize {
            wait_drq(channel, 10000)?;
        }
    }
    // Wait for the command to finish before returning.
    wait_status(channel, 10000)?;
    Some(())
}

#[cfg(feature = "arch_x86_64")]
fn device_channel(device: usize) -> Option<(&'static Channel, bool)> {
    match device {
        0 => Some((&PRIMARY, true)),
        1 => Some((&PRIMARY, false)),
        2 => Some((&SECONDARY, true)),
        3 => Some((&SECONDARY, false)),
        _ => None,
    }
}

// AArch64 stubs: the driver compiles but does nothing.
#[cfg(not(feature = "arch_x86_64"))]
pub fn probe_disks(_out: &mut [Option<BlockDeviceInfo>; 4]) {}
#[cfg(not(feature = "arch_x86_64"))]
pub fn read_sectors(_device: usize, _lba: u64, _buf: &mut [u8]) -> Option<()> {
    None
}
#[cfg(not(feature = "arch_x86_64"))]
pub fn write_sectors(_device: usize, _lba: u64, _buf: &[u8]) -> Option<()> {
    None
}
