#!/usr/bin/env bash
set -euo pipefail

# Build a bootable Limine ISO for Aperture OS.
#
# Usage: build-image.sh <arch> <kernel-elf> <output-iso> [<disk-image>]
#   arch: x86_64 | aarch64
#
# x86_64  -> hybrid BIOS + UEFI El Torito ISO
# aarch64 -> UEFI-only El Torito ISO
# If <disk-image> is provided it is added to /boot/aperture-disk.img on the ISO
# so the live installer can write it to a target disk.

ARCH="${1:?arch required (x86_64 | aarch64)}"
KERNEL_ELF="${2:?kernel elf path required}"
OUTPUT_ISO="${3:?output iso path required}"
DISK_IMAGE="${4:-}"
LIMINE_VERSION="12.3.3"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
LIMINE_DIR="$SCRIPT_DIR/limine-cache"
LIMINE_URL="https://github.com/limine-bootloader/limine/releases/download/v${LIMINE_VERSION}/limine-binary.tar.xz"

for tool in xorriso; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Error: required tool '$tool' not installed." >&2
        exit 1
    fi
done

if [[ ! -f "$KERNEL_ELF" ]]; then
    echo "Error: kernel ELF not found: $KERNEL_ELF" >&2
    exit 1
fi

# Download + build the Limine binaries if missing.
if [[ ! -f "$LIMINE_DIR/BOOTX64.EFI" ]]; then
    echo "Downloading Limine v${LIMINE_VERSION} binaries..."
    mkdir -p "$LIMINE_DIR"
    curl -fsSL --max-time 120 --retry 3 "$LIMINE_URL" \
        | tar -xJf - -C "$LIMINE_DIR" --strip-components=1
fi
if [[ ! -x "$LIMINE_DIR/limine" ]]; then
    echo "Building Limine deploy tool..."
    (cd "$LIMINE_DIR" && make limine)
fi

EFI_BOOT_FILE="BOOTX64.EFI"
case "$ARCH" in
    x86_64)
        EFI_BOOT_FILE="BOOTX64.EFI"
        ;;
    aarch64)
        EFI_BOOT_FILE="BOOTAA64.EFI"
        ;;
    *)
        echo "Error: unsupported arch '$ARCH' (use x86_64 or aarch64)" >&2
        exit 1
        ;;
esac

STAGE="$ROOT_DIR/target/${ARCH}-iso-staging"
rm -rf "$STAGE"
mkdir -p "$STAGE/EFI/BOOT" "$STAGE/boot"

cp "$KERNEL_ELF" "$STAGE/boot/kernel.elf"
cp "$SCRIPT_DIR/limine.conf" "$STAGE/limine.conf"
cp "$LIMINE_DIR/limine-bios.sys" "$STAGE/limine-bios.sys"
cp "$LIMINE_DIR/limine-bios-cd.bin" "$STAGE/limine-bios-cd.bin"
cp "$LIMINE_DIR/limine-uefi-cd.bin" "$STAGE/limine-uefi-cd.bin"
cp "$LIMINE_DIR/$EFI_BOOT_FILE" "$STAGE/EFI/BOOT/"

# If a disk image was built, add it to the ISO as a Limine boot module so
# the live installer can write it to a target disk.  The raw MBR image is
# small enough (64 MiB) to fit in a single module.
if [[ -n "${DISK_IMAGE:-}" && -f "$DISK_IMAGE" ]]; then
    cp "$DISK_IMAGE" "$STAGE/boot/aperture-disk.img"
    echo "    module_path: boot():/boot/aperture-disk.img" >> "$STAGE/limine.conf"
fi

case "$ARCH" in
    x86_64)
        echo "Building hybrid BIOS+UEFI ISO: $OUTPUT_ISO"
        xorriso -as mkisofs -R -r -J -V 'APERTURE' \
            -b limine-bios-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table \
            -hfsplus -apm-block-size 2048 \
            --efi-boot limine-uefi-cd.bin -efi-boot-part --efi-boot-image --protective-msdos-label \
            -o "$OUTPUT_ISO" "$STAGE"
        ;;
    aarch64)
        echo "Building AArch64 UEFI ISO: $OUTPUT_ISO"
        xorriso -as mkisofs -R -r -J -V 'APERTURE' \
            -hfsplus -apm-block-size 2048 \
            --efi-boot limine-uefi-cd.bin -efi-boot-part --efi-boot-image --protective-msdos-label \
            -o "$OUTPUT_ISO" "$STAGE"
        ;;
esac

echo "Installing Limine boot code..."
"$LIMINE_DIR/limine" bios-install "$OUTPUT_ISO" >/dev/null

rm -rf "$STAGE"
echo "Done: $OUTPUT_ISO"