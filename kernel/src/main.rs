#![no_std]
#![no_main]

//! Kernel entry point.
//!
//! Both architectures (x86_64 and AArch64) boot via the Limine boot protocol.
//! The Limine requests below are placed in the `.requests_*` linker sections
//! (kept by the linker scripts in `kernel/arch/<arch>/linker.ld`) and scanned
//! by the bootloader. The single `_start` entry parses the framebuffer, memory
//! map, and HHDM responses and hands them off to the architecture-independent
//! kernel.

extern crate alloc;

use limine::{
    request::{EntryPointRequest, FramebufferRequest, HhdmRequest, MemmapRequest, ModulesRequest, PagingModeRequest, RsdpRequest, StackSizeRequest},
    paging::PagingMode,
    BaseRevision, RequestsEndMarker, RequestsStartMarker,
};

// --- Limine request block ---------------------------------------------------

#[used]
#[link_section = ".requests_start"]
static REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[link_section = ".requests_end"]
static REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

/// Request Limine base revision 6 (the v3 protocol used by Limine 8.x+).
#[used]
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[link_section = ".requests"]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".requests"]
static MEMMAP_REQUEST: MemmapRequest = MemmapRequest::new();

#[used]
#[link_section = ".requests"]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section = ".requests"]
static PAGING_MODE_REQUEST: PagingModeRequest = PagingModeRequest::new_exact(PagingMode::MIN);

#[used]
#[link_section = ".requests"]
static STACK_SIZE_REQUEST: StackSizeRequest = StackSizeRequest::new(64 * 1024);

#[used]
#[link_section = ".requests"]
static ENTRY_POINT_REQUEST: EntryPointRequest = EntryPointRequest::new(_start);

/// Generic modules request.  The disk image is declared as a module in
/// `limine.conf` via `module_path` so the installer can retrieve it.
#[used]
#[link_section = ".requests"]
static MODULES_REQUEST: ModulesRequest = ModulesRequest::new();

#[used]
#[link_section = ".requests"]
static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

// --- Entry point ------------------------------------------------------------

#[no_mangle]
extern "C" fn _start() -> ! {
    // Establish the higher-half direct map first: Limine does not identity-map
    // physical memory, so any physical address we dereference as a pointer
    // (including the AArch64 PL011 UART MMIO) must go through the HHDM.
    let hhdm = HHDM_REQUEST
        .response()
        .map(|r| r.offset)
        .unwrap_or(0);
    kernel::mm::hhdm::set_offset(hhdm);

    #[cfg(feature = "arch_aarch64")]
    {
        kernel::arch::semihost_putchar(b'S');
        kernel::arch::semihost_putchar(b'\n');
    }
    kernel::arch::debug_putchar(b'K');
    kernel::arch::debug_putchar(b'\n');

    // Collect the memory map into a fixed-size buffer before initializing
    // subsystems; the physical allocator and the captured kernel page table
    // are needed by x86_64 interrupt setup.
    let mut regions = [kernel::boot_info::MemoryRegion::default(); 64];
    let mut region_count = 0usize;
    let mut usable = None;
    if let Some(memmap) = MEMMAP_REQUEST.response() {
        for entry in memmap.entries() {
            if region_count >= regions.len() {
                break;
            }
            let region = kernel::boot_info::MemoryRegion::from_limine(entry);
            if region.kind == kernel::boot_info::MemoryRegionKind::Usable
                && region.end - region.start >= 0x10_0000
            {
                if usable.map_or(true, |r: kernel::boot_info::MemoryRegion| {
                    region.end - region.start > r.end - r.start
                }) {
                    usable = Some(region);
                }
            }
            regions[region_count] = region;
            region_count += 1;
        }
    }
    unsafe {
        kernel::mm::init_physical_allocator(&regions[..region_count]);
        #[cfg(feature = "arch_x86_64")]
        kernel::mm::page_table::capture_kernel_page_table();
    }
    kernel::logln!("Physical frame allocator initialized ({} regions).", region_count);

    kernel::init();
    kernel::logln!("Aperture OS {} kernel booting...",
        if cfg!(feature = "arch_x86_64") { "x86_64" } else { "AArch64" });
    kernel::logln!("HHDM offset: {:#x}", hhdm);

    if let Some(rsdp) = RSDP_REQUEST.response() {
        let rsdp_addr = rsdp.address as usize as u64;
        kernel::logln!("RSDP at {:#x}", rsdp_addr);
        #[cfg(feature = "arch_x86_64")]
        unsafe {
            kernel::arch::acpi::init(rsdp_addr);
        }
    } else {
        kernel::logln!("No RSDP provided by bootloader.");
    }

    if let Some(region) = usable {
        // The bump heap dereferences its pointers directly, so hand it virtual
        // (HHDM) addresses rather than physical ones.
        kernel::mm::init_heap(
            kernel::mm::hhdm::phys_to_virt(region.start),
            kernel::mm::hhdm::phys_to_virt(region.end),
        );
        kernel::logln!(
            "Early heap: {:#x} - {:#x} ({} MiB)",
            region.start,
            region.end,
            (region.end - region.start) / 1024 / 1024
        );
    } else {
        kernel::logln!("WARNING: no usable memory region found.");
    }

    // Use the first Limine module as the installer disk image.  It is the
    // raw MBR disk image built by tools/build-disk-image.sh; we just leak it
    // so the installer can read it without copying.
    if let Some(modules) = MODULES_REQUEST.response() {
        if let Some(file) = modules.modules().first() {
            let image = file.data();
            kernel::logln!("Installer disk module: {} bytes", image.len());
            kernel::installer::set_image(image.as_ptr(), image.len());
        } else {
            kernel::logln!("No boot modules were loaded; installer disabled.");
        }
    } else {
        kernel::logln!("Modules request returned no response; installer disabled.");
    }

    // The Win32 subsystem is initialized; we do not run the synthetic PE
    // self-test here because it enters user mode and never returns, which
    // would prevent the GUI from starting.

    // Bring up the framebuffer / GUI if Limine provided one.
    if let Some(fb_resp) = FRAMEBUFFER_REQUEST.response() {
        if let Some(fb) = fb_resp.framebuffers().first() {
            let info = kernel::boot_info::FrameBufferInfo::from_limine(fb);
            let len = fb.size();
            let fb_addr = fb.address() as usize;
            kernel::logln!("fb addr={:#x} len={} bpp={} stride={}", fb_addr, len, info.bytes_per_pixel, info.stride);
            // Limine already exposes the framebuffer as a virtual pointer; do
            // not translate it through the HHDM.
            let buffer = unsafe {
                core::slice::from_raw_parts_mut(fb_addr as *mut u8, len)
            };
            unsafe {
                kernel::panic::register_framebuffer(buffer.as_mut_ptr(), len, info);
            }
            kernel::gui::init_compositor(buffer, info);
            kernel::gui::desktop::init(info.width as i32, info.height as i32);
            kernel::gui::render();
            kernel::logln!(
                "Framebuffer: {}x{} stride={} bpp={}",
                info.width,
                info.height,
                info.stride,
                info.bytes_per_pixel * 8
            );
        } else {
            kernel::logln!("Framebuffer request returned no framebuffers.");
        }
    } else {
        kernel::logln!("No framebuffer available.");
    }

    kernel::logln!("Kernel idle; reading input.");

    loop {
        let mut activity = false;
        while let Some(ch) = kernel::arch::interrupts::read_char() {
            kernel::gui::desktop::type_char(ch);
            activity = true;
        }

        if kernel::gui::desktop::handle_mouse() {
            activity = true;
        }
        kernel::installer::update();
        if activity || kernel::gui::needs_render() {
            kernel::gui::render();
            kernel::gui::clear_render_request();
        }

        kernel::arch::halt_once();
    }
}