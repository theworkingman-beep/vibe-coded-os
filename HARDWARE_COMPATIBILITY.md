# Hardware Compatibility

This document lists the hardware/firmware configurations Aperture OS has been
verified against and the real-hardware roadmap. It is honest about what has
and has not been tested.

## Verified: QEMU (development / CI)

Both ISOs are verified to boot in QEMU as part of this release and in the
`daily-build.yml` CI.

### x86_64 — `build/ApertureOS-x86_64.iso` (hybrid BIOS + UEFI)

```bash
qemu-system-x86_64 -cdrom build/ApertureOS-x86_64.iso -serial stdio -m 512M
```

- **SeaBIOS (BIOS)**: ✅ boots to `Kernel idle; reading input.` with
  framebuffer compositor and PS/2 input. Win32 phase self-tests
  (port/shim/interp/registry/env), GDI self-test, and win32k self-test all
  log `OK`. PCI bus-0 enumeration logs 6 QEMU devices; CMOS RTC reads the
  wall clock. Esc triggers ACPI power-off (QEMU exits 0).
- **OVMF (UEFI)**: boots via the Limine UEFI binary (validated in CI).
- ACPI RSDP/RSDT/XSDT headers parsed; memory map reconciled (17 regions in
  the default 512 MiB QEMU config). ACPI PM1a_CNT power-off (`0x604 ←
  0x2000`) verified working in QEMU.

### AArch64 — `build/ApertureOS-aarch64.iso` (UEFI)

```bash
qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
  -bios /usr/share/qemu-efi-aarch64/QEMU_EFI.fd \
  -cdrom build/ApertureOS-aarch64.iso -serial stdio -semihosting -device ramfb
```

- **QEMU `virt`, Cortex-A72, EDK2 UEFI**: ✅ boots to `Kernel idle; reading
  input.` with a ramfb framebuffer and the semihosting early console. Win32
  phase self-tests, GDI self-test, and win32k self-test all log `OK`. Esc
  triggers PSCI `SYSTEM_OFF` (QEMU exits 0).
- The EL1 exception vector table is installed; GIC + architectural timer are
  scaffolded but disabled (see AArch64 notes below).

## Real hardware

> ⚠️ **Not yet validated on real hardware in this release.** The kernel does
> not yet parse ACPI MADT/FADT bodies or a Device Tree, does not program the
> AArch64 MMU, and has no real-hardware storage/input/network drivers. The
> items below are the target list for Phase 1C / Phase 8.

### Target x86_64 hardware

- UEFI desktops/laptops (common AMI/Insyde/Dell/HP/Lenovo firmware).
- Legacy BIOS machines (the ISO is hybrid BIOS+UEFI).
- Requires: ACPI MADT/FADT/HPET/MCFG parsing, APIC (not PIC) interrupts,
  real storage (AHCI/NVMe), USB HID input. All on the roadmap.

### Target AArch64 hardware

- Raspberry Pi 5 (needs a UEFI firmware shim — EDK2 for RPi or U-Boot — to
  load Limine; the kernel must parse the Device Tree, not yet implemented).
- ARM64 SBCs and laptops using Device Tree.
- ARM servers using ACPI (IORT/GTDT/SPCR/MADT).

## Booting in UTM (macOS)

Both ISOs have been verified to boot to a visible desktop and stay up (no
black screen, no spontaneous reset) in QEMU configurations that match UTM:

- **AArch64 ISO**: UEFI firmware + a `virtio-ramfb` display. UTM's ARM VMs
  historically use `virtio-ramfb` as the UEFI GOP framebuffer device; the kernel
  renders the desktop to that framebuffer. (UTM's `virtio-gpu` display does not
  expose a UEFI GOP to Limine with stock firmware, so use `virtio-ramfb`.)
- **x86_64 ISO**: UEFI firmware (OVMF, which UTM bundles) or legacy BIOS, with
  any VGA/VirtIO display. Both BIOS and UEFI boot paths are verified.

### What was wrong (black screen) and how it was fixed

1. **AArch64 reset after ~12 s.** The bootloader/firmware left the architectural
   timer IRQ unmasked and the kernel never masked `DAIF`, so timer IRQs
   stormed the CPU before the GIC/timer bring-up was wired. After thousands of
   unacked IRQs the CPU corrupted state, took a prefetch abort, and PSCI
   powered the machine off — leaving a black screen. Fixed by masking all
   interrupts (`msr daifset, #0xf`) at the very start of boot in
  `kernel/src/arch/aarch64/vectors.rs::install_vectors`.
2. **Black screen during slow boot.** The framebuffer was only initialized
   *after* the Win32 phase self-tests, which are slow under software emulation
   (UTM TCG). The screen stayed black for the entire boot. Fixed by acquiring
   the Limine framebuffer and rendering an early splash *before* the
   self-tests in `kernel/src/main.rs`, so the desktop appears early and the
   display is never black during bring-up.
3. **AArch64 semihosting on hosts without it.** The early console uses the
   semihosting `HLT #0xF000`. UTM does not enable semihosting, so that
   instruction would trap and halt the CPU before any display. Fixed by
   installing the EL1 vector table before the first `debug_putchar` and
   recovering the trap (skip the instruction) in the sync handler, so a
   disabled semihosting console degrades to a silent no-op instead of
   halting. The on-screen dmesg ring buffer still shows boot logs.

### UTM setup (recommended)

- Create an ARM VM, select the `ApertureOS-aarch64-v1.0.0.iso` as the CD/DVD,
  set the system architecture to `aarch64` and the boot to UEFI.
- Under Display, select the **virtio-ramfb** display adapter (not
  `virtio-gpu`). 512 MB RAM is enough.
- Create an x86_64 VM, attach `ApertureOS-x86_64-v1.0.0.iso`, boot UEFI or BIOS.
  Default VGA/VirtIO display works; 512 MB RAM.

## AArch64 notes (why semihosting + ramfb today)

- Limine's higher-half direct map covers RAM but **not** the PL011 UART MMIO
  region (`0x0900_0000`). The kernel's own AArch64 MMU is not yet programmed,
  so the early console uses QEMU semihosting (`SYS_WRITEC`) and `-device
  ramfb` provides a UEFI GOP framebuffer. Real hardware will use the PL011
  once the MMU maps the UART (Phase 1B).
- The GIC + architectural timer init is scaffolded in
  `kernel/src/arch/aarch64/vectors.rs` but gated off, because accessing GIC
  MMIO through the HHDM faults before the MMU is programmed.

## Adding support for a new device

1. Discover the device from firmware-provided information (ACPI table, Device
   Tree node, UEFI, or PCI/PCIe ECAM enumeration) — never hardcode its
   address.
2. Implement the driver natively (it runs on both architectures).
3. Give every device access a timeout, null check, error-recovery path, and
   log message (`kernel/src/time.rs::poll_with_timeout`).
4. Handle device absence gracefully — log a warning and continue booting.

## ACPI / DTB parsing status

- x86_64: RSDP located (from Limine), RSDT/XSDT header walk + checksum
  verified. **MADT/FADT/HPET/MCFG body parsing is not yet implemented.**
- AArch64: **Device Tree parsing is not yet implemented.** ACPI-for-ARM
  tables (IORT/GTDT/SPCR/MADT) are not yet parsed.

## Driver status

| Driver | x86_64 | AArch64 |
|---|---|---|
| ATA PIO storage | ✅ | ❌ |
| AHCI / NVMe / SD-MMC | ❌ | ❌ |
| PCI bus-0 enumeration (config-space I/O) | ✅ (QEMU) | ❌ (ECAM not wired) |
| USB (xHCI / HID) | ❌ | ❌ |
| Network (E1000 / Realtek / virtio-net) | ❌ | ❌ |
| Audio (Intel HDA / virtio-snd) | ❌ | ❌ |
| GPU acceleration | ❌ (software compositor only) | ❌ |
| RTC / clock sources | ✅ CMOS RTC (QEMU) | ❌ (architectural counter read only) |
| Power (ACPI PM1 / PSCI) | ✅ ACPI PM1a_CNT | ✅ PSCI SYSTEM_OFF |