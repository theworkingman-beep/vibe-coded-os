#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# make-iso.sh builds a bootable ISO for the requested architecture.
# It reuses build.sh, which already produces the ISO as its final artifact.
ARCH="${1:-${ARCH:-x86_64}}"

./build.sh "$ARCH"
