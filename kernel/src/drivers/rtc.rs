//! CMOS Real-Time Clock driver (x86_64).
//!
//! Reads the hardware clock from CMOS via I/O ports `0x70` (index, with the
//! NMI-disable bit `0x80`) and `0x71` (data). Values are BCD-encoded unless
//! the status-B DM bit is set; this driver decodes both. Used for Phase 9
//! wall-clock time and `GetSystemTime`-style Win32 APIs.

use x86_64::instructions::port::Port;

const CMOS_INDEX: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

/// Read the CMOS register at `reg`.
fn cmos_read(reg: u8) -> u8 {
    unsafe {
        let mut idx: Port<u8> = Port::new(CMOS_INDEX);
        // Bit 7 disables NMI during the access; keep it set to match firmware.
        idx.write(reg | 0x80);
        let mut data: Port<u8> = Port::new(CMOS_DATA);
        data.read()
    }
}

/// Decode a BCD-encoded byte to binary.
fn bcd_to_bin(v: u8) -> u8 {
    (v >> 4) * 10 + (v & 0x0F)
}

/// A wall-clock time read from the RTC.
#[derive(Clone, Copy, Debug)]
pub struct RtcTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
}

/// Read the current RTC time. Waits for the update-in-progress bit to clear
/// so the fields are consistent.
pub fn read_time() -> Option<RtcTime> {
    // Wait while the clock is updating (status A, bit 7).
    let mut timeout = 0u32;
    while cmos_read(0x0A) & 0x80 != 0 {
        timeout += 1;
        if timeout > 1_000_000 {
            return None;
        }
        core::hint::spin_loop();
    }

    let seconds = cmos_read(0x00);
    let minutes = cmos_read(0x02);
    let hours = cmos_read(0x04);
    let day = cmos_read(0x07);
    let month = cmos_read(0x08);
    let year = cmos_read(0x09);

    // Status register B bit 2 selects binary (set) vs BCD (clear) mode.
    let binary = cmos_read(0x0B) & 0x04 != 0;
    let (seconds, minutes, hours, day, month, year) = if binary {
        (seconds, minutes, hours, day, month, year)
    } else {
        (
            bcd_to_bin(seconds),
            bcd_to_bin(minutes),
            bcd_to_bin(hours),
            bcd_to_bin(day),
            bcd_to_bin(month),
            bcd_to_bin(year),
        )
    };

    // The 24/12-hour flag (status B bit 1): if clear, hours are in 12-hour
    // BCD with the high bit marking PM. Keep it simple: if the AM/PM bit is
    // set and 24-hour mode is off, normalize.
    let hours = if binary || cmos_read(0x0B) & 0x02 != 0 {
        hours
    } else if hours & 0x80 != 0 {
        (hours & 0x7F) + 12
    } else {
        hours
    };

    let year_full = 2000u16 + year as u16;
    Some(RtcTime {
        year: year_full,
        month,
        day,
        hours,
        minutes,
        seconds,
    })
}

/// Read the RTC and log the wall-clock time.
pub fn log_rtc_time() {
    match read_time() {
        Some(t) => {
            crate::logln!(
                "rtc: {}/{:02}/{} {:02}:{:02}:{:02}",
                t.year,
                t.month,
                t.day,
                t.hours,
                t.minutes,
                t.seconds
            );
        }
        None => {
            crate::logln!("rtc: read timed out");
        }
    }
}
