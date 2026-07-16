#!/usr/bin/env bash
. "$HOME/.cargo/env"
set -euo pipefail

cd "$(dirname "$0")"

ARCH="${ARCH:-x86_64}"

case "$ARCH" in
    x86_64)
        TARGET="x86_64-unknown-none"
        FEATURES="arch_x86_64"
        ;;
    aarch64)
        TARGET="aarch64-unknown-none-softfloat"
        FEATURES="arch_aarch64"
        ;;
    *)
        echo "Unsupported ARCH: $ARCH (use x86_64 or aarch64)"
        exit 1
        ;;
esac

echo "Building Aperture OS kernel for $ARCH..."
cargo build -p kernel --no-default-features --features "$FEATURES" \
    -Z build-std=core,compiler_builtins,alloc \
    -Z build-std-features=compiler-builtins-mem \
    --target "$TARGET"

KERNEL_ELF="target/$TARGET/debug/kernel"
ISO_IMAGE="target/aperture-${ARCH}.iso"
DISK_IMAGE="target/aperture-${ARCH}-disk.img"

# The installer disk image is only useful on x86_64 where an ATA driver exists.
# On AArch64 the raw disk image adds ~100 MiB to the ISO as a Limine module
# and can exhaust the UEFI firmware's high-memory allocator, so omit it.
if [[ "$ARCH" == "x86_64" ]]; then
    echo "Building disk image..."
    tools/build-disk-image.sh "$ARCH" "$KERNEL_ELF" "$DISK_IMAGE"
    echo "Building Limine boot image..."
    tools/build-image.sh "$ARCH" "$KERNEL_ELF" "$ISO_IMAGE" "$DISK_IMAGE"
else
    echo "Building Limine boot image..."
    tools/build-image.sh "$ARCH" "$KERNEL_ELF" "$ISO_IMAGE"
fi
echo "Boot image: $ISO_IMAGE"