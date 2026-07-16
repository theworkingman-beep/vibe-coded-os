#!/usr/bin/env bash
set -euo pipefail

# Build a bootable raw disk image for Aperture OS.
#
# Usage: build-disk-image.sh <arch> <kernel-elf> <output-img>
#   arch: x86_64 | aarch64
#
# The produced image is an MBR-partitioned disk with a single large FAT32
# ESP.  It boots in both BIOS and UEFI mode on x86_64, and in UEFI mode on
# AArch64 (where no BIOS path exists).

ARCH="${1:?arch required (x86_64 | aarch64)}"
KERNEL_ELF="${2:?kernel elf path required}"
OUTPUT_IMG="${3:?output image path required}"
LIMINE_VERSION="12.3.3"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
LIMINE_DIR="$SCRIPT_DIR/limine-cache"
LIMINE_URL="https://github.com/limine-bootloader/limine/releases/download/v${LIMINE_VERSION}/limine-binary.tar.xz"

for tool in parted mkfs.fat mmd mcopy dd; do
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

# Build a 64 MiB disk image with a single 48 MiB FAT32 ESP at 1 MiB.
# This keeps the raw image below Limine's practical single-module size
# limit while still giving the ESP enough room for the kernel and Limine
# files.
DISK_SIZE_MB=64
ESP_SIZE_MB=48
ESP_SECTORS=$((ESP_SIZE_MB * 1024 * 1024 / 512))
ESP_OFFSET_SECTORS=2048

STAGE="$ROOT_DIR/target/${ARCH}-disk-staging"
rm -rf "$STAGE"
mkdir -p "$STAGE"
FAT_IMG="$STAGE/esp.fat"

# Create the FAT32 ESP image.  Use mkfs.fat's -C option so it creates the
# file and formats it in one go, matching the geometry Limine expects.
mkfs.fat -F32 -S 512 -s 2 -C "$FAT_IMG" "$ESP_SECTORS" >/dev/null 2>&1

mmd -i "$FAT_IMG" ::/boot >/dev/null 2>&1 || true
mmd -i "$FAT_IMG" ::/EFI >/dev/null 2>&1 || true
mmd -i "$FAT_IMG" ::/EFI/BOOT >/dev/null 2>&1 || true

mcopy -o -i "$FAT_IMG" "$KERNEL_ELF" ::/boot/kernel.elf >/dev/null 2>&1
mcopy -o -i "$FAT_IMG" "$SCRIPT_DIR/limine.conf" ::/limine.conf >/dev/null 2>&1

if [[ "$ARCH" == "x86_64" ]]; then
    # Stage3 file needed by Limine's BIOS boot path.
    mcopy -o -i "$FAT_IMG" "$LIMINE_DIR/limine-bios.sys" ::/limine-bios.sys >/dev/null 2>&1
fi

mcopy -o -i "$FAT_IMG" "$LIMINE_DIR/$EFI_BOOT_FILE" ::/EFI/BOOT/$EFI_BOOT_FILE >/dev/null 2>&1

# Create the raw disk image and insert the ESP.
dd if=/dev/zero of="$OUTPUT_IMG" bs=1M count="$DISK_SIZE_MB" status=none
parted -s "$OUTPUT_IMG" mklabel msdos \
    mkpart primary fat32 1MiB $((ESP_SIZE_MB + 1))MiB \
    set 1 boot on
dd if="$FAT_IMG" of="$OUTPUT_IMG" bs=512 seek="$ESP_OFFSET_SECTORS" conv=notrunc status=none

# Install Limine's BIOS boot code into the MBR and post-MBR gap.
if [[ "$ARCH" == "x86_64" ]]; then
    "$LIMINE_DIR/limine" bios-install "$OUTPUT_IMG" >/dev/null
fi

rm -rf "$STAGE"
echo "Done: $OUTPUT_IMG"
