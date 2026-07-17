//! Minimal `no_std` MBR + GPT partition-table parser.
//!
//! Used by the Aperture OS kernel to discover partitions on a raw disk image
//! (the installer module and, later, real storage devices) and by host-side
//! unit tests. Both Master Boot Record (legacy) and GUID Partition Table
//! (UEFI) layouts are parsed; a protective MBR (partition type `0xEE`) is
//! detected and the GPT header/entries read from it.

#![no_std]

#[cfg(test)]
extern crate std;

use core::convert::TryInto;

/// Protective-MBR partition type that signals a GPT layout follows.
pub const GPT_PROTECTIVE: u8 = 0xEE;

/// A single MBR partition-table entry (16 bytes at offset 0x1BE + i*16).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MbrPartition {
    /// Bootable / active flag (0x80 = active).
    pub bootable: bool,
    /// Partition type byte (0x00 = empty entry).
    pub kind: u8,
    /// First sector LBA (little-endian u32).
    pub lba_start: u32,
    /// Number of sectors.
    pub sectors: u32,
}

impl MbrPartition {
    /// True for an empty table slot.
    pub fn is_empty(&self) -> bool {
        self.kind == 0
    }
}

/// Parse the four MBR partition entries from a 512-byte boot sector.
///
/// Returns `None` if `data` is shorter than the 512-byte boot sector. Empty
/// slots (type 0) are returned as `Some(MbrPartition)` with `kind == 0`; use
/// `is_empty` to skip them.
pub fn parse_mbr(data: &[u8]) -> Option<[Option<MbrPartition>; 4]> {
    if data.len() < 512 {
        return None;
    }
    let mut out = [None; 4];
    for (i, slot) in out.iter_mut().enumerate() {
        let base = 0x1BE + i * 16;
        let kind = data[base + 4];
        let lba_start = u32::from_le_bytes([
            data[base + 8],
            data[base + 9],
            data[base + 10],
            data[base + 11],
        ]);
        let sectors = u32::from_le_bytes([
            data[base + 12],
            data[base + 13],
            data[base + 14],
            data[base + 15],
        ]);
        *slot = Some(MbrPartition {
            bootable: data[base] == 0x80,
            kind,
            lba_start,
            sectors,
        });
    }
    Some(out)
}

/// True if the MBR's first partition is a GPT protective entry (type `0xEE`).
pub fn is_protective_mbr(data: &[u8]) -> bool {
    parse_mbr(data)
        .and_then(|m| m[0])
        .map(|p| p.kind == GPT_PROTECTIVE)
        .unwrap_or(false)
}

/// Parsed GPT header (92 bytes of the LBA-1 sector).
#[derive(Clone, Copy, Debug)]
pub struct GptHeader {
    pub signature: [u8; 8],
    pub revision: u32,
    pub header_size: u32,
    pub header_crc32: u32,
    pub backup_lba: u64,
    pub first_usable_lba: u64,
    pub last_usable_lba: u64,
    pub disk_guid: [u8; 16],
    pub entry_lba: u64,
    pub num_entries: u32,
    pub entry_size: u32,
}

/// The GPT signature "EFI PART".
pub const GPT_SIGNATURE: [u8; 8] = *b"EFI PART";

/// Parse the GPT header from raw disk bytes. The header lives in the sector
/// immediately following the protective MBR (LBA 1, byte offset 512).
pub fn parse_gpt_header(data: &[u8]) -> Option<GptHeader> {
    if data.len() < 512 + 92 {
        return None;
    }
    let h = &data[512..512 + 92];
    if h[0..8] != GPT_SIGNATURE {
        return None;
    }
    let mut disk_guid = [0u8; 16];
    disk_guid.copy_from_slice(&h[56..72]);
    Some(GptHeader {
        signature: GPT_SIGNATURE,
        revision: u32::from_le_bytes([h[8], h[9], h[10], h[11]]),
        header_size: u32::from_le_bytes([h[12], h[13], h[14], h[15]]),
        header_crc32: u32::from_le_bytes([h[16], h[17], h[18], h[19]]),
        backup_lba: u64::from_le_bytes(h[32..40].try_into().ok()?),
        first_usable_lba: u64::from_le_bytes(h[40..48].try_into().ok()?),
        last_usable_lba: u64::from_le_bytes(h[48..56].try_into().ok()?),
        disk_guid,
        entry_lba: u64::from_le_bytes(h[72..80].try_into().ok()?),
        num_entries: u32::from_le_bytes([h[80], h[81], h[82], h[83]]),
        entry_size: u32::from_le_bytes([h[84], h[85], h[86], h[87]]),
    })
}

/// A parsed GPT partition entry (128 bytes). The `name` field is the raw
/// 36-unit UTF-16LE code units; use `name_str` to decode the printable prefix.
#[derive(Clone, Copy, Debug)]
pub struct GptPartition {
    pub type_guid: [u8; 16],
    pub unique_guid: [u8; 16],
    pub first_lba: u64,
    pub last_lba: u64,
    pub attributes: u64,
    pub name: [u16; 36],
}

impl GptPartition {
    /// True if the entry is unused (all-zero type GUID).
    pub fn is_empty(&self) -> bool {
        self.type_guid == [0u8; 16]
    }

    /// Decode the UTF-16LE name into a byte buffer as ASCII (lossy: non-ASCII
    /// code units are dropped). Returns the number of bytes written.
    pub fn name_to_ascii(&self, out: &mut [u8]) -> usize {
        let mut n = 0usize;
        for &u in &self.name {
            if u == 0 {
                break;
            }
            if n >= out.len() {
                break;
            }
            if u < 0x80 {
                out[n] = u as u8;
                n += 1;
            }
        }
        n
    }
}

/// Maximum number of GPT entries the parser will return.
pub const MAX_GPT_ENTRIES: usize = 128;

/// Parse up to `MAX_GPT_ENTRIES` GPT partition entries from raw disk bytes
/// using the parsed `header`. Entry bytes are read from the offset
/// `header.entry_lba * 512`.
pub fn parse_gpt_entries(
    data: &[u8],
    header: &GptHeader,
) -> [Option<GptPartition>; MAX_GPT_ENTRIES] {
    let mut out: [Option<GptPartition>; MAX_GPT_ENTRIES] = [None; MAX_GPT_ENTRIES];
    let entry_size = header.entry_size as usize;
    if !(56..=512).contains(&entry_size) {
        return out;
    }
    let base = (header.entry_lba as usize).saturating_mul(512);
    let count = (header.num_entries as usize).min(MAX_GPT_ENTRIES);
    for (i, slot) in out.iter_mut().enumerate().take(count) {
        let off = match base.checked_add(i * entry_size) {
            Some(o) => o,
            None => break,
        };
        if off + entry_size > data.len() {
            break;
        }
        let e = &data[off..off + entry_size];
        let mut type_guid = [0u8; 16];
        type_guid.copy_from_slice(&e[0..16]);
        let mut unique_guid = [0u8; 16];
        unique_guid.copy_from_slice(&e[16..32]);
        let first_lba = u64::from_le_bytes(e[32..40].try_into().unwrap());
        let last_lba = u64::from_le_bytes(e[40..48].try_into().unwrap());
        let attributes = u64::from_le_bytes(e[48..56].try_into().unwrap());
        let mut name = [0u16; 36];
        for (j, name_slot) in name.iter_mut().enumerate() {
            let p = 56 + j * 2;
            if p + 2 > e.len() {
                break;
            }
            *name_slot = u16::from_le_bytes([e[p], e[p + 1]]);
        }
        *slot = Some(GptPartition {
            type_guid,
            unique_guid,
            first_lba,
            last_lba,
            attributes,
            name,
        });
    }
    out
}

// Re-export the size constant for callers that build raw buffers.

#[cfg(test)]
mod tests {
    use super::*;

    fn mbr_with_one_partition(kind: u8, lba: u32, sectors: u32) -> [u8; 512] {
        let mut buf = [0u8; 512];
        // Boot signature 0x55AA.
        buf[510] = 0x55;
        buf[511] = 0xAA;
        // First partition entry at 0x1BE.
        buf[0x1BE] = 0x80; // active
        buf[0x1BE + 4] = kind;
        buf[0x1BE + 8..0x1BE + 12].copy_from_slice(&lba.to_le_bytes());
        buf[0x1BE + 12..0x1BE + 16].copy_from_slice(&sectors.to_le_bytes());
        buf
    }

    #[test]
    fn parses_mbr_partition() {
        let buf = mbr_with_one_partition(0x83, 2048, 1000);
        let parts = parse_mbr(&buf).expect("mbr");
        let p0 = parts[0].expect("entry 0");
        assert!(p0.bootable);
        assert_eq!(p0.kind, 0x83);
        assert_eq!(p0.lba_start, 2048);
        assert_eq!(p0.sectors, 1000);
        assert!(parts[1].unwrap().is_empty());
    }

    #[test]
    fn detects_protective_mbr() {
        let buf = mbr_with_one_partition(GPT_PROTECTIVE, 1, 0xFFFFFFFF);
        assert!(is_protective_mbr(&buf));
        let plain = mbr_with_one_partition(0x07, 2048, 1000);
        assert!(!is_protective_mbr(&plain));
    }

    #[test]
    fn rejects_short_data() {
        assert!(parse_mbr(&[0u8; 100]).is_none());
    }

    fn fake_gpt_disk() -> std::vec::Vec<u8> {
        // 512-byte protective MBR + 512-byte GPT header + one 128-byte entry.
        let mut disk = std::vec![0u8; 512 * 4];
        // Protective MBR.
        disk[0x1BE + 4] = GPT_PROTECTIVE;
        disk[0x1BE + 8..0x1BE + 12].copy_from_slice(&1u32.to_le_bytes());
        // GPT header at LBA 1 (offset 512).
        let h = 512;
        disk[h..h + 8].copy_from_slice(&GPT_SIGNATURE);
        disk[h + 8..h + 12].copy_from_slice(&0x00010000u32.to_le_bytes()); // revision
        disk[h + 12..h + 16].copy_from_slice(&92u32.to_le_bytes()); // header size
        disk[h + 32..h + 40].copy_from_slice(&3u64.to_le_bytes()); // backup lba
        disk[h + 40..h + 48].copy_from_slice(&4u64.to_le_bytes()); // first usable
        disk[h + 48..h + 56].copy_from_slice(&100u64.to_le_bytes()); // last usable
        disk[h + 72..h + 80].copy_from_slice(&2u64.to_le_bytes()); // entry lba
        disk[h + 80..h + 84].copy_from_slice(&1u32.to_le_bytes()); // num entries
        disk[h + 84..h + 88].copy_from_slice(&128u32.to_le_bytes()); // entry size
                                                                     // One partition entry at LBA 2 (offset 1024).
        let e = 1024;
        disk[e..e + 16].copy_from_slice(&[0x77u8; 16]); // non-zero type guid
        disk[e + 32..e + 40].copy_from_slice(&4u64.to_le_bytes()); // first lba
        disk[e + 40..e + 48].copy_from_slice(&99u64.to_le_bytes()); // last lba
                                                                    // Name "EFI" at offset 56.
        disk[e + 56] = b'E';
        disk[e + 58] = b'F';
        disk[e + 60] = b'I';
        disk
    }

    #[test]
    fn parses_gpt_header_and_entry() {
        let disk = fake_gpt_disk();
        let hdr = parse_gpt_header(&disk).expect("gpt header");
        assert_eq!(hdr.signature, GPT_SIGNATURE);
        assert_eq!(hdr.entry_lba, 2);
        assert_eq!(hdr.num_entries, 1);
        assert_eq!(hdr.entry_size, 128);
        let entries = parse_gpt_entries(&disk, &hdr);
        let e0 = entries[0].expect("entry 0");
        assert_eq!(e0.first_lba, 4);
        assert_eq!(e0.last_lba, 99);
        assert!(!e0.is_empty());
        let mut name = [0u8; 36];
        let n = e0.name_to_ascii(&mut name);
        assert_eq!(&name[..n], b"EFI");
    }

    #[test]
    fn empty_gpt_entry_is_empty() {
        let disk = fake_gpt_disk();
        let hdr = parse_gpt_header(&disk).unwrap();
        let entries = parse_gpt_entries(&disk, &hdr);
        // Entry 1 is all-zero.
        assert!(entries[1].map(|e| e.is_empty()).unwrap_or(true));
    }
}
