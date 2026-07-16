#!/usr/bin/env bash
set -euo pipefail

# Backwards-compatible wrapper: build the x86_64 bootable ISO.
# The real work is in tools/build-image.sh, invoked by ./build.sh x86_64.
cd "$(dirname "$0")"

KERNEL_ELF="target/x86_64-unknown-none/debug/kernel"
if [[ ! -f "$KERNEL_ELF" ]]; then
    echo "Error: $KERNEL_ELF not found. Run ./build.sh x86_64 first." >&2
    exit 1
fi

tools/build-image.sh x86_64 "$KERNEL_ELF" "target/aperture-x86_64.iso"
echo "ISO: target/aperture-x86_64.iso"