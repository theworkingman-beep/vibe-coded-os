//! Bootloader-independent boot information types.
//!
//! These types are populated from the Limine boot protocol responses in
//! `kernel_main` and consumed by the architecture-independent kernel code.

use limine::framebuffer::{Framebuffer, FRAMEBUFFER_RGB};
use limine::memmap;

/// Physical memory region reported by the bootloader/firmware.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub kind: MemoryRegionKind,
}

impl Default for MemoryRegion {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            kind: MemoryRegionKind::Reserved,
        }
    }
}

/// Kind of memory region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    BadMemory,
    BootloaderReclaimable,
    ExecutableAndModules,
    Framebuffer,
    MappedReserved,
    Unknown,
}

impl MemoryRegionKind {
    /// Map a Limine memory map entry type to the kernel's region kind.
    pub fn from_limine(type_: u64) -> Self {
        match type_ {
            memmap::MEMMAP_USABLE => MemoryRegionKind::Usable,
            memmap::MEMMAP_RESERVED => MemoryRegionKind::Reserved,
            memmap::MEMMAP_ACPI_RECLAIMABLE => MemoryRegionKind::AcpiReclaimable,
            memmap::MEMMAP_ACPI_NVS => MemoryRegionKind::AcpiNvs,
            memmap::MEMMAP_BAD_MEMORY => MemoryRegionKind::BadMemory,
            memmap::MEMMAP_BOOTLOADER_RECLAIMABLE => MemoryRegionKind::BootloaderReclaimable,
            memmap::MEMMAP_EXECUTABLE_AND_MODULES => MemoryRegionKind::ExecutableAndModules,
            memmap::MEMMAP_FRAMEBUFFER => MemoryRegionKind::Framebuffer,
            memmap::MEMMAP_MAPPED_RESERVED => MemoryRegionKind::MappedReserved,
            _ => MemoryRegionKind::Unknown,
        }
    }

    /// Human-readable name for logging.
    pub fn name(self) -> &'static str {
        match self {
            MemoryRegionKind::Usable => "usable",
            MemoryRegionKind::Reserved => "reserved",
            MemoryRegionKind::AcpiReclaimable => "acpi-reclaimable",
            MemoryRegionKind::AcpiNvs => "acpi-nvs",
            MemoryRegionKind::BadMemory => "bad-memory",
            MemoryRegionKind::BootloaderReclaimable => "bootloader-reclaimable",
            MemoryRegionKind::ExecutableAndModules => "executable/modules",
            MemoryRegionKind::Framebuffer => "framebuffer",
            MemoryRegionKind::MappedReserved => "mapped-reserved",
            MemoryRegionKind::Unknown => "unknown",
        }
    }
}

impl MemoryRegion {
    /// Build a region from a Limine memory map entry.
    pub fn from_limine(entry: &memmap::Entry) -> Self {
        Self {
            start: entry.base,
            end: entry.base + entry.length,
            kind: MemoryRegionKind::from_limine(entry.type_),
        }
    }
}

/// Pixel format of the framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb,
    Bgr,
    U8,
    Unknown {
        red_position: u8,
        green_position: u8,
        blue_position: u8,
    },
}

/// Framebuffer metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameBufferInfo {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub bytes_per_pixel: usize,
    pub pixel_format: PixelFormat,
}

impl FrameBufferInfo {
    /// Build framebuffer metadata from a Limine framebuffer descriptor.
    pub fn from_limine(fb: &Framebuffer) -> Self {
        let pixel_format = if fb.memory_model == FRAMEBUFFER_RGB {
            // Limine reports an RGB memory model; on little-endian x86/aarch64
            // the 32-bit pixel layout in memory is BGR, matching QEMU.
            PixelFormat::Bgr
        } else {
            PixelFormat::Unknown {
                red_position: fb.red_mask_shift,
                green_position: fb.green_mask_shift,
                blue_position: fb.blue_mask_shift,
            }
        };
        Self {
            width: fb.width as usize,
            height: fb.height as usize,
            stride: fb.pitch as usize,
            bytes_per_pixel: (fb.bpp as usize + 7) / 8,
            pixel_format,
        }
    }
}
