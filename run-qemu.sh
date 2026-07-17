#!/usr/bin/env bash
# Run an ApertureOS ISO in QEMU for development/CI.
#
# Usage: ./run-qemu.sh [x86_64|aarch64]
set -euo pipefail

cd "$(dirname "$0")"

ARCH="${1:-x86_64}"
EFI_FD="/usr/share/qemu-efi-aarch64/QEMU_EFI.fd"

case "$ARCH" in
    x86_64)
        ISO="build/ApertureOS-x86_64.iso"
        [[ -f "$ISO" ]] || { echo "Missing $ISO -- run ./build.sh x86_64"; exit 1; }
        echo "Running ISO: $ISO"
        exec qemu-system-x86_64 -cdrom "$ISO" -serial stdio -m 512M
        ;;
    aarch64)
        ISO="build/ApertureOS-aarch64.iso"
        [[ -f "$ISO" ]] || { echo "Missing $ISO -- run ./build.sh aarch64"; exit 1; }
        BIOS="${QEMU_EFI_FD:-$EFI_FD}"
        [[ -f "$BIOS" ]] || { echo "Missing AArch64 UEFI firmware ($BIOS)"; exit 1; }
        echo "Running ISO: $ISO (UEFI)"
        # -semihosting: the AArch64 early console uses the semihosting
        # SYS_WRITEC channel until the kernel programs its own MMU to map the
        # PL011 UART MMIO region (Phase 1B ongoing). Output appears on stdout.
        # -device ramfb: edk2 RAM framebuffer so UEFI GOP (and thus Limine)
        # exposes a framebuffer the kernel compositor can drive.
        exec qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
            -bios "$BIOS" -cdrom "$ISO" -serial stdio -semihosting \
            -device ramfb
        ;;
    *)
        echo "Usage: $0 [x86_64|aarch64]"; exit 1
        ;;
esac