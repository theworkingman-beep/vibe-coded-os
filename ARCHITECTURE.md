# Aperture OS Architecture

This document describes the design of Aperture OS and the honest current
state of each subsystem. The four pillars of the project are:

1. **Native dual-architecture kernel** — the OS itself runs natively on
   x86_64 and AArch64. No OS component is ever translated.
2. **Native Win32 compatibility** — the NT kernel ABI is implemented in Rust;
   real Windows PE executables run as first-class processes.
3. **Cross-architecture binary translation (FEX-style)** — *only* for
   externally downloaded Windows PE binaries whose machine type does not
   match the host.
4. **Real hardware support** — discover devices from ACPI/DTB/UEFI/PCI, not
   hardcoded addresses; degrade gracefully when hardware is absent.

Pillars 1 is the foundation delivered in v1.0.0. Pillars 2–4 are scaffolded
to varying degrees (see status below).

## 1. Boot path

Both architectures boot via the **Limine** boot protocol (`kernel/src/main.rs`
places Limine requests in `.requests_*` linker sections). Limine provides the
memory map, HHDM offset, framebuffer, RSDP, and boot modules.

- **x86_64**: hybrid BIOS + UEFI ISO. `_start` sets the HHDM, runs
  `kernel::init()`, parses the ACPI RSDP/RSDT/XSDT headers, initializes the
  framebuffer compositor, and idles reading PS/2 input.
- **AArch64**: UEFI ISO. `_start` sets the HHDM and uses semihosting for the
  early console (the PL011 UART MMIO is not yet mapped). The EL1 exception
  vector table is installed; the GIC + architectural timer are scaffolded but
  disabled pending MMU programming.

## 2. Hardware abstraction layer (`kernel/src/arch/`)

Each architecture implements a uniform interface: `init()`, `debug_putchar(u8)`,
`hlt()`, `halt_once()`, `without_interrupts()`, `monotonic_cycles()`,
`mouse_position()`, `mouse_buttons()`, and `read_char()` (via
`interrupts::read_char`).

- **x86_64** (`arch/x86_64/`): GDT + TSS (ring 0/3 selectors, IST1), IDT with
  PIC remap and a 1000 Hz PIT preemptive timer, PS/2 keyboard + mouse,
  LAPIC/IOAPIC MMIO drivers, SYSCALL/SYSRET entry, naked-asm context switch,
  ACPI header parser. **Status: functional.**
- **AArch64** (`arch/aarch64/`): EL1 vector table (sync/IRQ/halt entries),
  naked-asm callee-saved context switch, PL011 + semihosting console,
  architectural counter access. **Status: partial.** MMU page tables, GIC,
  timer, and SVC syscall entry are scaffolded/not wired.

## 3. Memory management (`kernel/src/mm/`)

- `frame_allocator.rs`: bitmap physical frame allocator with CAS allocation
  and `reserve_range`. **Functional** on both arches (driven by the Limine
  memmap).
- `hhdm.rs`: higher-half direct-map offset translation. **Functional.**
- `heap.rs`: 9 size-class free-list allocator (large allocations currently
  leak — no side table). **Partial.**
- `page_table.rs`: x86_64 4-level page-table walker (`map`/`map_region`/
  `translate`, copies the kernel mapping). AArch64 is a stub. **x86_64
  functional, AArch64 scaffold.**

## 4. Scheduler (`kernel/src/win32/scheduler.rs`)

Cooperative round-robin scheduler with a naked-asm context switch on both
architectures, plus a preemptive timer-driven path (x86_64 PIT). Each thread
gets a register file and state. **Partial** — no SMP, no per-process address
space activation on AArch64.

## 5. Windows compatibility subsystem (`kernel/src/win32/`)

- `loader.rs`: PE/COFF loader. Parses headers (via `pe-parser`), allocates a
  process, maps sections into per-process page tables (x86_64), and now
  **parses + logs the import directory**. Import *resolution* (binding to
  built-in native system DLL shims), base relocations, and TLS are not yet
  implemented. **Partial.**
- `nt.rs`: NT syscall dispatch. 9 of 16 syscall numbers are wired
  (`NtClose`, `NtCreateFile`, `NtReadFile`, `NtWriteFile`,
  `NtAllocateVirtualMemory`, `NtFreeVirtualMemory`,
  `NtQuerySystemInformation`, `NtQueryInformationProcess`,
  `NtDelayExecution`); the rest return `NotImplemented`. `dispatch` does real
  user-pointer-to-physical translation. **Partial.**
- `objects.rs`: 1024-slot handle table with allocate/lookup/close and object
  kinds. **Functional.**
- `process.rs` / `thread.rs`: `Process` and `Thread` structs with a register
  file; no PEB/TEB or reference counting yet. **Scaffold/partial.**
- `registry.rs`: in-memory flat 256-slot registry shim; no hives or
  persistence. **Partial.**
- `win32k.rs`: maps a desktop to a compositor window; message dispatch is a
  TODO. **Scaffold.**

## 6. Binary translation (`kernel/src/win32/abi/`)

- `interpreter.rs`: x86_64 guest interpreter — decodes NOP/RET/JMP/CALL/MOV
  imm/XOR/LEA/SYSCALL and updates the guest register file; halts on
  unsupported instructions. **Partial.**
- `aarch64_interpreter.rs`: decodes + logs a few AArch64 instructions; no
  register emulation. **Scaffold.**
- `x86_jit.rs` / `aarch64_jit.rs`: `translate_block` returns a placeholder;
  no code emission. **Scaffold.**
- `syscall.rs`: inline-asm user-mode syscall helper. **Functional.**

The decoders (`crates/x86-decode`, `crates/aarch64-decode`) are unit-tested.
See [TRANSLATION.md](TRANSLATION.md).

## 7. GUI (`kernel/src/gui/`)

Software-rendered compositor with up to 32 windows, premultiplied RGBA
backbuffers, back-to-front blending, and RGB/BGR/unknown pixel-format
handling. The desktop (`desktop.rs`) renders a taskbar, a dmesg window, and
Terminal/Install buttons, with mouse + keyboard input (x86_64). `widgets.rs`
(Button/Label/ListBox/ProgressBar), `font.rs` (5×7 bitmap glyphs), and
`cursor.rs` (16×16 arrow) are functional. **No Win32 window-manager model or
GDI yet.**

## 8. VFS and storage (`kernel/src/vfs/`, `kernel/src/disk/`)

In-memory virtual filesystem (tree of nodes, 64 KiB per-file cap) backing NT
file syscalls. ATA PIO driver (x86_64) for disk I/O. **No on-disk filesystem
(FAT32/NTFS), GPT/MBR parsing, or real-hardware storage drivers yet.**

## 9. Installer (`kernel/src/installer/`)

GUI installer that lists disks, writes a disk image in 64 KiB chunks, and
shows progress. **Functional** (x86_64, where disks are visible).

## How the pillars interconnect

The kernel boots natively (pillar 1). The Win32 subsystem (pillar 2) loads
PE binaries; if the machine type matches the host, code runs natively, and
imported system DLL calls route to built-in native shims. If the machine type
differs, the translator (pillar 3) translates only the external binary's
code, still routing system DLL calls to the native shims. Real-hardware
support (pillar 4) means every address and device is discovered from
firmware tables, with graceful degradation. Pillars 2–4 are the active
roadmap on top of the pillar-1 foundation delivered here.