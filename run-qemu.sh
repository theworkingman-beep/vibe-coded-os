#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

ISO_IMAGE="target/aperture-x86_64.iso"

if command -v qemu-system-x86_64 >/dev/null 2>&1; then
    QEMU="qemu-system-x86_64"
elif command -v qemu >/dev/null 2>&1; then
    QEMU="qemu"
else
    echo "qemu-system-x86_64 not found; cannot run OS."
    exit 1
fi

if [[ ! -f "$ISO_IMAGE" ]]; then
    echo "No bootable ISO found: $ISO_IMAGE"
    echo "Run ./build.sh x86_64 first."
    exit 1
fi

echo "Running ISO: $ISO_IMAGE"
$QEMU -cdrom "$ISO_IMAGE" -boot d -serial stdio -m 256M
