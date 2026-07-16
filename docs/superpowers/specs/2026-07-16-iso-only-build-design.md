---
title: ISO-Only Build Artifacts
date: 2026-07-16
---

# ISO-Only Build Artifacts

## Goal

Make `ApertureOS` builds and CI produce **only bootable `.iso` files**. Remove raw disk image (`.img`) generation, remove the embedded installer disk image from the x86_64 ISO, and update local QEMU boot tooling to use the ISO directly.

## Background

Currently `build.sh x86_64` produces two artifacts:

- `target/aperture-x86_64-disk.img` — a 64 MiB raw MBR disk image created by `tools/build-disk-image.sh`
- `target/aperture-x86_64.iso` — a hybrid BIOS+UEFI ISO created by `tools/build-image.sh`, which embeds the raw disk image as a Limine module (`boot():/boot/aperture-disk.img`) so a live installer can write it to a target disk.

AArch64 already builds only an ISO (the disk image is omitted there because it exhausts UEFI high memory).

Two untracked `.img` files (`target-disk.img`, `target-disk-test.img`) also exist in the repository root; they are not produced or referenced by any current script.

`run-qemu.sh` currently boots raw `.img` files (`target/aperture-uefi.img` or `target/aperture-bios.img`) that are no longer produced.

CI currently uploads both `.iso` and `.img` artifacts in the x86_64 job, but nightly releases already ship only `.iso` files.

## Design

### Scope

This is a build-system cleanup, not a feature addition. The live installer disk-image path is being removed because it is currently unused and the project now boots and tests exclusively from ISO.

### Changes

1. **Delete `tools/build-disk-image.sh`**  
   This entire script is dedicated to raw `.img` creation. Removing it eliminates the `.img` output path entirely.

2. **Simplify `build.sh`**  
   - Remove `DISK_IMAGE` variable and the call to `tools/build-disk-image.sh`.
   - Remove the `tools/build-image.sh` optional fourth argument; always invoke it with just `arch`, `kernel-elf`, and `output-iso`.
   - Keep the conditional logic that already skips the disk image on AArch64, but make it unconditional (x86_64 now also skips it).

3. **Simplify `tools/build-image.sh`**  
   - Remove the optional `<disk-image>` argument.
   - Remove the copy of `aperture-disk.img` into the ISO staging directory.
   - Remove the dynamic appending of `module_path: boot():/boot/aperture-disk.img` to `limine.conf`.

4. **Update `.github/workflows/daily-build.yml`**  
   - Remove `target/aperture-x86_64-disk.img` from the `aperture-os-x86_64-boot-images` artifact path list. Since the `.img` no longer exists, that artifact upload would fail.
   - Keep the `aperture-os-x86_64-iso` artifact and the nightly release `.iso` assets unchanged.

5. **Rewrite `run-qemu.sh`**  
   - Remove the old logic that searches for raw UEFI/BIOS `.img` files.
   - Boot `target/aperture-x86_64.iso` directly with `qemu-system-x86_64` using `-cdrom target/aperture-x86_64.iso -boot d`.
   - Preserve `-serial stdio -m 256M` defaults and fall back to plain `qemu` if `qemu-system-x86_64` is unavailable.

6. **Clean up untracked `.img` files**  
   - Delete `target-disk.img` and `target-disk-test.img` from the repository root.
   - These files are not produced by the build and are not referenced anywhere.

7. **Update `.gitignore`**  
   - Add `*.img` and any generated disk image names (e.g., `target-disk*.img`) so future accidental `.img` files are not committed.

8. **Update documentation**  
   - In `README.md`, remove the outdated line "Build the AArch64 kernel ELF (boot image generation not yet implemented):". AArch64 ISO generation is already implemented, and the parenthetical is stale. (Optional if unrelated; but it is immediately adjacent to the build commands.)

### Files touched

- `tools/build-disk-image.sh` — delete
- `build.sh` — edit
- `tools/build-image.sh` — edit
- `.github/workflows/daily-build.yml` — edit
- `run-qemu.sh` — edit
- `.gitignore` — possibly edit
- `README.md` — possibly edit
- `target-disk.img` — delete (untracked)
- `target-disk-test.img` — delete (untracked)

### Verification

After the change:

1. Run `./build.sh x86_64` and confirm only `target/aperture-x86_64.iso` is produced.
2. Run `./run-qemu.sh` and confirm QEMU boots from the ISO.
3. Run CI-style build and confirm the x86_64 artifact upload step no longer expects a `.img` file.
4. Confirm no stray `.img` files remain in the repository root.

## Trade-offs

- **Pros:** Build system is simpler; CI artifacts are consistent with release assets; local QEMU path matches CI/test path; removes dead installer code.
- **Cons:** The live installer can no longer write a pre-built disk image. If a self-installing ISO is needed later, it must be redesigned (e.g., generate the disk image at install time or embed a smaller archive).

## Decision

Proceed with **Option A**: remove raw disk image generation entirely.
