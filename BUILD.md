# Building Aperture OS

This document covers building both architecture ISOs, running them in QEMU,
cross-building, and the path to booting on real hardware.

## Prerequisites

- Rust **nightly** toolchain with `rust-src` and `llvm-tools-preview`.
- `xorriso` (creates the El Torito bootable ISO).
- `curl` and `make` (the build downloads and builds Limine on first run).
- `qemu-system-x86_64` and `qemu-system-aarch64` to run the ISOs.
- For AArch64 QEMU: the `qemu-efi-aarch64` firmware package
  (`/usr/share/qemu-efi-aarch64/QEMU_EFI.fd` on Debian/Ubuntu).

```bash
rustup toolchain install nightly --component rust-src,llvm-tools-preview
rustup target add x86_64-unknown-none --toolchain nightly
rustup target add aarch64-unknown-none-softfloat --toolchain nightly

# Debian/Ubuntu host dependencies:
sudo apt-get install -y xorriso curl make \
  qemu-system-x86 qemu-system-arm qemu-efi-aarch64
```

## Build the ISOs

```bash
./build.sh x86_64      # produces build/ApertureOS-x86_64.iso  (hybrid BIOS+UEFI)
./build.sh aarch64     # produces build/ApertureOS-aarch64.iso (UEFI)

# make-iso.sh is a thin wrapper over build.sh:
./make-iso.sh x86_64 && ./make-iso.sh aarch64
```

`build.sh`:
1. Cross-compiles the `kernel` crate for the requested target
   (`x86_64-unknown-none` or `aarch64-unknown-none-softfloat`) with
   `-Z build-std=core,compiler_builtins,alloc`.
2. Invokes `tools/build-image.sh`, which stages the kernel ELF plus the
   Limine bootloader and a `limine.conf` menu, builds the ISO with `xorriso`,
   and runs `limine bios-install` for BIOS boot support.

## Run in QEMU

```bash
./run-qemu.sh x86_64     # BIOS, serial to stdio
./run-qemu.sh aarch64    # UEFI virt, semihosting console, ramfb framebuffer
```

Equivalent manual commands:

```bash
# x86_64 (BIOS)
qemu-system-x86_64 -cdrom build/ApertureOS-x86_64.iso -serial stdio -m 512M

# x86_64 (UEFI, optional)
qemu-system-x86_64 -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=OVMF_VARS.fd \
  -cdrom build/ApertureOS-x86_64.iso -serial stdio -m 512M

# AArch64 (UEFI virt)
qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
  -bios /usr/share/qemu-efi-aarch64/QEMU_EFI.fd \
  -cdrom build/ApertureOS-aarch64.iso -serial stdio -semihosting -device ramfb
```

A successful boot prints `Aperture OS x86_64 kernel booting...` /
`Aperture OS AArch64 kernel booting...` followed by subsystem init lines and
`Kernel idle; reading input.`

### AArch64 console note

The AArch64 early console uses QEMU **semihosting** (`SYS_WRITEC`), so the
`-semihosting` flag is required and output appears on QEMU stdout. Limine's
higher-half direct map covers RAM but not the PL011 UART MMIO region
(`0x0900_0000`); the kernel's own AArch64 MMU is not yet programmed to map it.
`-device ramfb` provides a UEFI GOP framebuffer so Limine hands the kernel a
display. Once the AArch64 MMU maps the UART, the plain `-serial stdio` path
will work without semihosting.

## Cross-compilation

Building either target from either host works because `build.sh` uses
`-Z build-std` (the core/library crates are built for the target, not taken
from the host toolchain). No separate cross-gcc is required for the kernel
itself; `gcc-aarch64-linux-gnu` is only needed if you extend the build with
C/assembly components.

## Host tests

```bash
cargo test -p pe-parser -p x86-decode -p aarch64-decode
cargo fmt --all -- --check
cargo clippy -p pe-parser -p x86-decode -p aarch64-decode
```

## Booting on real hardware

> ⚠️ Not yet validated on real hardware in this release. The notes below are
> the intended procedure once the real-hardware robustness work (Phase 1C)
> lands.

1. Write the ISO to a USB stick:
   ```bash
   sudo dd if=build/ApertureOS-x86_64.iso of=/dev/sdX bs=4M status=progress conv=fsync
   ```
   (On Windows use Rufus or Etcher in dd / image mode.)
2. Boot from USB. The x86_64 ISO is hybrid BIOS+UEFI; the AArch64 ISO is
   UEFI-only. Disable Secure Boot (the Limine bootloader is not signed).
3. For AArch64 SBCs (e.g. Raspberry Pi 5): a UEFI firmware shim (EDK2 for RPi
   or U-Boot) is required to load the Limine UEFI binary; the kernel does not
   yet parse the Device Tree, so SBC bring-up depends on Phase 1B DTB work.

See [HARDWARE_COMPATIBILITY.md](HARDWARE_COMPATIBILITY.md) for the tested /
targeted hardware list.