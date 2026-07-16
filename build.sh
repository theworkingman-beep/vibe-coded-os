#!/usr/bin/env bash
. "$HOME/.cargo/env"
set -euo pipefail

cd "$(dirname "$0")"

# Accept architecture as an argument or via the ARCH environment variable.
ARCH="${1:-${ARCH:-x86_64}}"

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

echo "Building Limine boot image..."
tools/build-image.sh "$ARCH" "$KERNEL_ELF" "$ISO_IMAGE"
echo "Boot image: $ISO_IMAGE"
