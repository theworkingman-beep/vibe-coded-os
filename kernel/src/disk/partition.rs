//! Partition-table discovery.
//!
//! Uses the architecture-independent `part-parser` crate to read the MBR
//! (and, when protective, the GPT) of a raw disk image and log the layout.
//! On real hardware this runs against the first sector read from a storage
//! device; at boot it runs against the installer disk module so the layout
//! is visible in the boot log.

use part_parser::{is_protective_mbr, parse_gpt_entries, parse_gpt_header, parse_mbr};

/// Parse `data` (a raw disk image, at least 512 bytes) and log every
/// discovered partition. Handles both legacy MBR and GPT (protective MBR)
/// layouts. Returns the number of partitions found.
pub fn log_partitions(data: &[u8]) -> usize {
    let Some(parts) = parse_mbr(data) else {
        crate::logln!(
            "part: disk image too small for an MBR ({} bytes)",
            data.len()
        );
        return 0;
    };

    if is_protective_mbr(data) {
        crate::logln!("part: protective MBR -> GPT layout");
        return log_gpt(data);
    }

    let mut count = 0usize;
    for (i, entry) in parts.iter().enumerate() {
        let Some(p) = entry else { continue };
        if p.is_empty() {
            continue;
        }
        crate::logln!(
            "part: mbr[{}] type={:#x} lba={} sectors={}",
            i,
            p.kind,
            p.lba_start,
            p.sectors
        );
        count += 1;
    }
    crate::logln!("part: {} MBR partition(s)", count);
    count
}

fn log_gpt(data: &[u8]) -> usize {
    let Some(header) = parse_gpt_header(data) else {
        crate::logln!("part: GPT header missing/invalid");
        return 0;
    };
    crate::logln!(
        "part: gpt rev={:#x} entries={} entry_size={} backup_lba={}",
        header.revision,
        header.num_entries,
        header.entry_size,
        header.backup_lba
    );
    let entries = parse_gpt_entries(data, &header);
    let mut count = 0usize;
    let mut name_buf = [0u8; 36];
    for (i, entry) in entries.iter().enumerate() {
        let Some(e) = entry else { break };
        if e.is_empty() {
            continue;
        }
        let nlen = e.name_to_ascii(&mut name_buf);
        let name = core::str::from_utf8(&name_buf[..nlen]).unwrap_or("");
        crate::logln!(
            "part: gpt[{}] lba={}-{} attr={:#x} name=\"{}\"",
            i,
            e.first_lba,
            e.last_lba,
            e.attributes,
            name
        );
        count += 1;
    }
    crate::logln!("part: {} GPT partition(s)", count);
    count
}
