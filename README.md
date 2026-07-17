# Aperture OS

Aperture OS is an experimental operating system written in Rust, targeting
**native execution on both x86_64 and AArch64** with the long-term goal of
running real Windows PE executables as first-class processes — without Wine,
Proton, or a host Windows installation — and running externally downloaded
Windows binaries across architectures via a built-in binary translator.

> **Status (v1.0.0): bootable dual-architecture kernel with Win32 subsystem
> scaffolding.** Both architecture kernels compile and boot in QEMU to a
> software-rendered framebuffer desktop with a taskbar, dmesg window,
> install/terminal buttons, and a Task Manager. The NT syscall table is fully
> wired (16/16), PE imports resolve against built-in system DLL shims, the
> x86_64 guest interpreter, registry, process environment, IPC ports, GDI
> primitives, and Win32 window model all run boot-time self-tests on both
> architectures. ACPI (x86_64) / PSCI (AArch64) power-off works. Real Windows
> PE execution, JIT translation, and real-hardware drivers remain future work.
> See [What Works](#what-works) and [Roadmap](#roadmap) for the honest,
> current capability matrix.

The OS itself always runs **natively** on whichever architecture it boots on.
Binary translation is a planned userland compatibility feature for
*externally downloaded* Windows PE binaries whose machine type does not match
the host — it is never applied to the OS's own kernel, drivers, system DLLs,
desktop, or built-in applications.

## What Works

Both ISOs build and boot in QEMU:

| Capability | x86_64 | AArch64 |
|---|---|---|
| Boots via Limine (BIOS + UEFI / UEFI) | ✅ | ✅ (UEFI) |
| Serial/early console | ✅ COM1 | ✅ semihosting¹ |
| Framebuffer + software compositor | ✅ 800×600 | ✅ 800×600 (ramfb) |
| Desktop: taskbar, dmesg window, buttons | ✅ | ✅ |
| PS/2 keyboard + mouse input | ✅ | ❌ (no input driver yet) |
| ACPI RSDP/RSDT/XSDT header parse | ✅ (partial) | ➖ (DTB/ACPI not parsed) |
| Bitmap frame allocator + HHDM | ✅ | ✅ |
| x86_64 4-level page tables | ✅ | ❌ (AArch64 MMU not programmed) |
| Preemptive timer scheduler | ✅ (PIT) | ❌ (GIC/timer disabled²) |
| Cooperative context switch | ✅ | ✅ |
| SYSCALL/SYSRET entry | ✅ | ❌ (SVC stub) |
| NT syscall dispatch (16 of 16 wired) | ✅ | ✅ (shared) |
| PE loader: section mapping + import parse | ✅ (shared) | ✅ (shared) |
| PE import resolution → built-in DLL shims | ✅ (shared) | ✅ (shared) |
| Built-in system DLL shim registry (ntdll/kernel32) | ✅ (shared) | ✅ (shared) |
| x86_64 guest interpreter (decoder + semantics) | ✅ (shared) | ✅ (shared) |
| Registry: create/set/query (NtCreateKey/SetValue/Query) | ✅ (shared) | ✅ (shared) |
| Process env block (Get/SetEnvironmentVariable) | ✅ (shared) | ✅ (shared) |
| IPC message ports (send/receive round-trip) | ✅ (shared) | ✅ (shared) |
| GDI primitives (line/rect/fill/ellipse/setpixel) | ✅ (shared) | ✅ (shared) |
| Win32 window model (RegisterClass/CreateWindow/GetMessage) | ✅ (shared) | ✅ (shared) |
| Task Manager window | ✅ | ✅ |
| Power off (ACPI PM1a / PSCI) | ✅ | ✅ |
| Object manager / handle table | ✅ (shared) | ✅ (shared) |
| In-memory VFS | ✅ (shared) | ✅ (shared) |
| MBR + GPT partition parser (`part-parser` crate) | ✅ (shared) | ✅ (shared) |
| PCI bus 0 enumeration | ✅ | ➖ (ECAM not wired) |
| CMOS RTC wall clock | ✅ | ➖ (no aarch64 RTC driver) |
| ATA PIO disk (x86_64) | ✅ | ❌ |

Every shared subsystem above ships with a boot-time self-test that exercises
the real code path and logs `OK` on both architectures (search the boot log
for `self_test`). The host crates (`pe-parser`, `x86-decode`,
`aarch64-decode`, `part-parser`) have unit tests run by `cargo test`.

¹ AArch64 early console uses QEMU semihosting (`SYS_WRITEC`) because Limine's
higher-half direct map covers RAM but not the PL011 UART MMIO region; the
kernel's own AArch64 MMU is not yet programmed to map it. Real hardware will
use the PL011 once the MMU maps the UART (Phase 1B ongoing).

² The AArch64 GIC + architectural timer init is scaffolded but gated off
(accessing GIC MMIO through the HHDM faults before the MMU is programmed).

See [HARDWARE_COMPATIBILITY.md](HARDWARE_COMPATIBILITY.md) for the QEMU
configurations verified and the real-hardware roadmap.

## Build

Requires the nightly Rust toolchain with `rust-src` and `llvm-tools-preview`,
plus `xorriso` (and `qemu` to run). `build.sh` downloads and builds the
Limine bootloader binaries on first use.

```bash
rustup toolchain install nightly --component rust-src,llvm-tools-preview
rustup target add x86_64-unknown-none --toolchain nightly
rustup target add aarch64-unknown-none-softfloat --toolchain nightly

# Build the bootable ISO for each architecture:
./build.sh x86_64        # -> build/ApertureOS-x86_64.iso
./build.sh aarch64       # -> build/ApertureOS-aarch64.iso

# make-iso.sh is a thin wrapper over build.sh:
./make-iso.sh x86_64 && ./make-iso.sh aarch64
```

See [BUILD.md](BUILD.md) for detailed step-by-step instructions, cross-build
notes, and real-hardware boot guidance.

## Run in QEMU

```bash
# x86_64 (BIOS, serial to stdio)
./run-qemu.sh x86_64
# equivalently:
qemu-system-x86_64 -cdrom build/ApertureOS-x86_64.iso -serial stdio -m 512M

# AArch64 (UEFI virt, semihosting console, ramfb framebuffer)
./run-qemu.sh aarch64
# equivalently:
qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
  -bios /usr/share/qemu-efi-aarch64/QEMU_EFI.fd \
  -cdrom build/ApertureOS-aarch64.iso -serial stdio -semihosting -device ramfb
```

## Host Tests

The architecture-independent crates have host-testable unit tests:

```bash
cargo test -p pe-parser -p x86-decode -p aarch64-decode -p part-parser
```

## Architecture

```text
kernel/
  arch/        Hardware abstraction layer
    x86_64/    IDT/PIC, LAPIC/IOAPIC, GDT+TSS, SYSCALL/SYSRET, context switch, ACPI
    aarch64/   EL1 vector table, context switch, PL011/semihosting console (MMU/GIC: scaffold)
  boot_info.rs Architecture-independent boot metadata (Limine memmap/framebuffer)
  gui/         Software compositor, desktop, widgets, 5x7 bitmap font, cursor
  mm/          Bitmap frame allocator, free-list heap, HHDM, x86_64 page tables
  vfs/         In-memory virtual filesystem backing NT file syscalls
  disk/        ATA PIO driver (x86_64) + MBR/GPT partition logging
  drivers/     PCI bus-0 enumeration + CMOS RTC (x86_64)
  installer/   GUI installer that writes a disk image to storage
  win32/       Windows compatibility subsystem
    abi/       x86/ARM JIT + interpreter translation layers and syscall helper
    loader.rs  PE/COFF loader: section mapping + import resolution
    shims.rs   Built-in ntdll/kernel32 DLL shim registry
    nt.rs      NT syscall numbers, dispatch table, and handlers
    objects.rs Object manager / handle table
    process.rs Process model + PEB/TEB + environment block
    thread.rs  Thread model + register file
    scheduler.rs Cooperative + preemptive (x86_64 timer) scheduler
    registry.rs In-memory registry shim
    win32k.rs  Win32 desktop/GUI bridge + window-manager model
crates/
  pe-parser/       PE32/PE32+ header, section, import-directory, thunk parser
  x86-decode/       x86_64 instruction decoder (NOP/RET/JMP/SYSCALL + more)
  aarch64-decode/   AArch64 instruction decoder (NOP/RET/SVC/BL/MOVZ/ADRP + more)
  part-parser/      MBR + GPT partition-table parser (no_std, host-tested)
tools/
  build-image.sh   Wraps kernel ELF into a Limine hybrid BIOS+UEFI / UEFI ISO
  limine.conf       Limine boot menu (5s timeout, auto-boot the kernel)
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full design,
[WIN32_COMPATIBILITY.md](WIN32_COMPATIBILITY.md) for the Win32/NT API
coverage matrix, and [TRANSLATION.md](TRANSLATION.md) for the binary
translation design and status.

## Roadmap

The full vision (a complete Windows-compatible dual-architecture OS with
FEX-style cross-architecture translation, real-hardware drivers, and a full
desktop) is a multi-year effort. v1.0.0 establishes the bootable
dual-architecture kernel foundation. Phases and their honest status:

1. **Boot path (both arches, QEMU)** — ✅ done. Real-hardware robustness
   (ACPI MADT/FADT body, DTB, AArch64 MMU/GIC/timer) — 🚧 in progress.
2. **Kernel stability & core infra** — ✅ done (QEMU). Preemptive scheduler
   (x86_64 PIT), context switch (both), object manager, 16/16 NT syscalls,
   non-reentrant heap large-alloc deadlock fixed. AArch64 preemptive
   interrupts disabled pending MMU.
3. **PE loading & DLL resolution** — ✅ done (scaffold). Section mapping,
   import parsing, and import resolution against a built-in ntdll/kernel32
   shim registry. Base relocations, TLS, and external `.dll` loading remain.
4. **Cross-architecture binary translation** — 🚧 scaffold. x86/aarch64
   decoders + an x86_64 interpreter with a passing self-test
   (`mov`/`xor` round-trip); JIT emitters are placeholders.
5. **Storage, filesystems, persistence** — 🚧 partial. In-memory VFS, ATA PIO
   (x86_64), and an MBR+GPT partition parser (`part-parser` crate, tested).
   No FAT32/NTFS or real-hardware storage drivers.
6. **Graphical desktop with Win32 windowing** — ✅ done (scaffold). Compositor
   + desktop + widgets render; GDI primitives (line/rect/fill/ellipse/pixel)
   and a Win32 window-manager model (RegisterClass/CreateWindow/GetMessage)
   pass boot self-tests. No file-manager/terminal/editor apps yet.
7. **Windows API coverage** — ✅ done (scaffold). 16/16 NT syscalls wired,
   in-memory registry (create/set/query) with a passing self-test, process
   environment block, IPC message ports. No common controls/dialogs or
   cmd.exe.
8. **Driver stack** — 🚧 partial. ATA PIO + PCI bus-0 enumeration + CMOS RTC
   (x86_64); PSCI power-off (AArch64). No USB, network, audio, GPU, or
   real-hardware storage.
9. **System services & usability** — ✅ done (scaffold). Task Manager window
   (live process/thread/file/key/total handle counts), process environment
   variables, and ACPI/PSCI shutdown (Esc key). No service manager yet.
10. **Polish & documentation** — ✅ this release.
11. **Release** — ✅ this release (ISOs, tag, GitHub release).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT OR Apache-2.0