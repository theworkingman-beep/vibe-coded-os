# ISO-Only Build Artifacts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove raw `.img` generation from the ApertureOS build system so that local builds, CI artifacts, and nightly releases produce only bootable `.iso` files.

**Architecture:** Update the shell build pipeline to delete the raw-disk-image script, skip the disk-image build step, and stop embedding an installer disk image in the ISO. Update QEMU launch tooling to boot from the ISO. Clean up CI upload paths, `.gitignore`, stray `.img` files, and stale README text.

**Tech Stack:** Bash, GitHub Actions YAML, Limine, xorriso, QEMU.

## Global Constraints

- The build must produce **only** `.iso` boot artifacts.
- `tools/build-disk-image.sh` must be deleted.
- `tools/build-image.sh` no longer accepts a disk image argument and no longer injects `module_path` into `limine.conf`.
- `build.sh` no longer calls `tools/build-disk-image.sh`.
- `.github/workflows/daily-build.yml` must not upload `target/aperture-x86_64-disk.img`.
- `run-qemu.sh` must boot `target/aperture-x86_64.iso` directly.
- Untracked `target-disk.img` and `target-disk-test.img` must be removed.
- `.gitignore` must ignore future `.img` files.
- Frequent commits; each task ends in a testable deliverable.

---

## File Structure

| File | Responsibility after change |
|------|------------------------------|
| `build.sh` | Builds the kernel ELF, then invokes `tools/build-image.sh` to produce an ISO only. |
| `tools/build-image.sh` | Stages the kernel, Limine files, and `limine.conf`, then builds a hybrid BIOS+UEFI ISO. No disk-image handling. |
| `.github/workflows/daily-build.yml` | Builds x86_64 and AArch64 ISOs, runs host tests, and uploads only ISO artifacts. |
| `run-qemu.sh` | Boots the x86_64 ISO in QEMU. |
| `.gitignore` | Ignores Rust `target/`, caches, and `*.img` files. |
| `README.md` | Reflects current build commands; removes stale AArch64 parenthetical. |
| `tools/build-disk-image.sh` | Deleted. |
| `target-disk.img` / `target-disk-test.img` | Deleted. |

---

### Task 1: Delete the raw disk-image builder

**Files:**
- Delete: `tools/build-disk-image.sh`

**Interfaces:**
- Consumes: nothing (script is self-contained and currently invoked by `build.sh`)
- Produces: nothing (the `.img` output path is removed)

- [ ] **Step 1: Delete the file**

  ```bash
  rm tools/build-disk-image.sh
  ```

- [ ] **Step 2: Verify the file is gone**

  Run: `ls tools/build-disk-image.sh`
  Expected: `No such file or directory`

- [ ] **Step 3: Commit**

  ```bash
  git rm tools/build-disk-image.sh
  git commit -m "build: remove raw disk image generator"
  ```

---

### Task 2: Simplify `build.sh` to produce only the ISO

**Files:**
- Modify: `build.sh`

**Interfaces:**
- Consumes: `tools/build-image.sh <arch> <kernel-elf> <output-iso>`
- Produces: `target/aperture-${ARCH}.iso`

- [ ] **Step 1: Read the current file**

  Run: `cat build.sh`

- [ ] **Step 2: Replace the body with the ISO-only version**

  The new file should read:

  ```bash
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

  echo "Building Limine boot image..."
  tools/build-image.sh "$ARCH" "$KERNEL_ELF" "$ISO_IMAGE"
  echo "Boot image: $ISO_IMAGE"
  ```

  Changes made:
  - Removed the `DISK_IMAGE` variable.
  - Removed the `tools/build-disk-image.sh` invocation.
  - Removed the AArch64 vs. x86_64 branch around `build-image.sh`; both now call it with exactly three arguments.
  - Removed the stale comment about the installer disk image.

- [ ] **Step 3: Run a syntax check**

  Run: `bash -n build.sh`
  Expected: no output (success)

- [ ] **Step 4: Build x86_64 and verify only the ISO appears**

  Run:
  ```bash
  ./build.sh x86_64
  ls target/*.iso target/*.img 2>/dev/null || true
  ```
  Expected: `target/aperture-x86_64.iso` exists; no `.img` file named `target/aperture-x86_64-disk.img` should exist. The `ls target/*.img` line may error, which is fine.

- [ ] **Step 5: Commit**

  ```bash
  git add build.sh
  git commit -m "build: produce only ISO boot images"
  ```

---

### Task 3: Simplify `tools/build-image.sh`

**Files:**
- Modify: `tools/build-image.sh`

**Interfaces:**
- Consumes: kernel ELF, Limine binaries, `tools/limine.conf`
- Produces: `target/aperture-${ARCH}.iso`

- [ ] **Step 1: Read the current file**

  Run: `cat tools/build-image.sh`

- [ ] **Step 2: Remove the disk-image argument and module handling**

  Replace the usage comment and argument lines at the top with:

  ```bash
  # Build a bootable Limine ISO for Aperture OS.
  #
  # Usage: build-image.sh <arch> <kernel-elf> <output-iso>
  #   arch: x86_64 | aarch64
  #
  # x86_64  -> hybrid BIOS + UEFI El Torito ISO
  # aarch64 -> UEFI-only El Torito ISO
  ```

  Replace the argument parsing:

  ```bash
  ARCH="${1:?arch required (x86_64 | aarch64)}"
  KERNEL_ELF="${2:?kernel elf path required}"
  OUTPUT_ISO="${3:?output iso path required}"
  LIMINE_VERSION="12.3.3"
  ```

  Remove the old `DISK_IMAGE` line entirely.

  Remove the staging block that copies/ injects the disk image:

  ```bash
  # If a disk image was built, add it to the ISO as a Limine boot module so
  # the live installer can write it to a target disk.  The raw MBR image is
  # small enough (64 MiB) to fit in a single module.
  if [[ -n "${DISK_IMAGE:-}" && -f "$DISK_IMAGE" ]]; then
      cp "$DISK_IMAGE" "$STAGE/boot/aperture-disk.img"
      echo "    module_path: boot():/boot/aperture-disk.img" >> "$STAGE/limine.conf"
  fi
  ```

  Delete that entire block.

- [ ] **Step 3: Verify the cleaned file**

  Run: `bash -n tools/build-image.sh`
  Expected: no output

  Run: `grep -n 'DISK_IMAGE\|module_path\|aperture-disk' tools/build-image.sh || true`
  Expected: no matches

- [ ] **Step 4: Rebuild x86_64 ISO and confirm it still works**

  Run:
  ```bash
  ./build.sh x86_64
  ls -lh target/aperture-x86_64.iso
  ```
  Expected: ISO exists and is smaller than before (no embedded 64 MiB disk image). Previously it was >64 MiB; now it should be under 10 MiB.

- [ ] **Step 5: Commit**

  ```bash
  git add tools/build-image.sh
  git commit -m "build: stop embedding installer disk image in ISO"
  ```

---

### Task 4: Update CI to stop uploading disk images

**Files:**
- Modify: `.github/workflows/daily-build.yml`

**Interfaces:**
- Consumes: `target/aperture-x86_64.iso`
- Produces: artifact `aperture-os-x86_64-iso`

- [ ] **Step 1: Read the current workflow**

  Run: `cat .github/workflows/daily-build.yml`

- [ ] **Step 2: Edit the x86_64 artifact upload step**

  Locate the step named `Upload x86_64 boot images` and change its `path:` list from:

  ```yaml
          path: |
            target/aperture-x86_64-disk.img
            target/aperture-x86_64.iso
  ```

  to:

  ```yaml
          path: |
            target/aperture-x86_64.iso
  ```

  Since this artifact now duplicates the `Upload x86_64 ISO` step, optionally rename the first upload step to `Upload x86_64 boot image` (singular) or remove it entirely. The minimal correct change is to drop the `.img` path. Keep both uploads if you want; `if-no-files-found: error` on the first will now succeed because the `.iso` still exists.

- [ ] **Step 3: Validate the YAML**

  Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/daily-build.yml'))"`
  Expected: no output (success)

- [ ] **Step 4: Commit**

  ```bash
  git add .github/workflows/daily-build.yml
  git commit -m "ci: drop disk image artifact upload"
  ```

---

### Task 5: Rewrite `run-qemu.sh` to boot the ISO

**Files:**
- Modify: `run-qemu.sh`

**Interfaces:**
- Consumes: `target/aperture-x86_64.iso`
- Produces: QEMU process booting the OS

- [ ] **Step 1: Read the current file**

  Run: `cat run-qemu.sh`

- [ ] **Step 2: Replace the file content**

  The new file should read:

  ```bash
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
  ```

- [ ] **Step 3: Syntax check**

  Run: `bash -n run-qemu.sh`
  Expected: no output

- [ ] **Step 4: Build and confirm QEMU would use the ISO**

  Run:
  ```bash
  ./build.sh x86_64
  head -5 run-qemu.sh
  ls target/aperture-x86_64.iso
  ```
  Expected: `run-qemu.sh` references `-cdrom target/aperture-x86_64.iso -boot d`, and the ISO exists.

- [ ] **Step 5: Commit**

  ```bash
  git add run-qemu.sh
  git commit -m "run-qemu: boot from ISO instead of raw disk image"
  ```

---

### Task 6: Update `.gitignore` to ignore `.img` files

**Files:**
- Modify: `.gitignore`

**Interfaces:**
- Consumes: nothing
- Produces: `.gitignore` rules preventing accidental `.img` commits

- [ ] **Step 1: Read the current file**

  Run: `cat .gitignore`

- [ ] **Step 2: Add `.img` ignore rules**

  Append to the file:

  ```gitignore
  # Raw disk images are no longer produced; ignore any stragglers.
  *.img
  ```

- [ ] **Step 3: Verify the rule matches the stray files**

  Run:
  ```bash
  git check-ignore -v target-disk.img target-disk-test.img target/aperture-x86_64-disk.img 2>&1 | head -5
  ```
  Expected: each path is matched by the new `*.img` rule.

- [ ] **Step 4: Commit**

  ```bash
  git add .gitignore
  git commit -m "gitignore: ignore raw disk images"
  ```

---

### Task 7: Remove stale README text

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: nothing
- Produces: updated build instructions

- [ ] **Step 1: Read the relevant section**

  Run: `sed -n '40,55p' README.md`

- [ ] **Step 2: Update the AArch64 build heading**

  Replace:

  ```markdown
  Build the AArch64 kernel ELF (boot image generation not yet implemented):

  ```bash
  ARCH=aarch64 ./build.sh
  ```
  ```

  with:

  ```markdown
  Build the AArch64 bootable ISO:

  ```bash
  ARCH=aarch64 ./build.sh
  ```
  ```

- [ ] **Step 3: Preview the change**

  Run: `sed -n '40,55p' README.md`
  Expected: the AArch64 heading now says "bootable ISO".

- [ ] **Step 4: Commit**

  ```bash
  git add README.md
  git commit -m "docs: aarch64 ISO generation is implemented"
  ```

---

### Task 8: Delete untracked `.img` files from the repository root

**Files:**
- Delete: `target-disk.img`
- Delete: `target-disk-test.img`

**Interfaces:**
- Consumes: nothing
- Produces: a clean repository root

- [ ] **Step 1: Confirm the files are untracked**

  Run: `git status --short`
  Expected: `?? target-disk-test.img` and `?? target-disk.img` (or already absent).

- [ ] **Step 2: Delete the files**

  ```bash
  rm -f target-disk.img target-disk-test.img
  ```

- [ ] **Step 3: Verify they are gone**

  Run: `ls target-disk*.img 2>/dev/null || echo "no stray .img files"`
  Expected: `no stray .img files`

- [ ] **Step 4: Commit**

  Untracked files do not need `git rm`; just verify `git status` no longer lists them.

  ```bash
  git status --short
  git commit --allow-empty -m "chore: remove stray raw disk images from repo root"
  ```

---

### Task 9: Final verification

**Files:**
- Test: build output, QEMU launch, git status

**Interfaces:**
- Consumes: all prior changes
- Produces: green verification checklist

- [ ] **Step 1: Clean and rebuild x86_64 from scratch**

  Run:
  ```bash
  rm -f target/aperture-x86_64.iso target/aperture-x86_64-disk.img
  ./build.sh x86_64
  ```
  Expected: build succeeds and produces `target/aperture-x86_64.iso`.

- [ ] **Step 2: Confirm no new `.img` files appear in the repo**

  Run:
  ```bash
  find . -maxdepth 1 -name '*.img' -print
  find target -maxdepth 1 -name '*.img' -print
  ```
  Expected: both commands return nothing.

- [ ] **Step 3: Check git status is clean**

  Run: `git status --short`
  Expected: no untracked `.img` files; only the committed changes are present.

- [ ] **Step 4: Optionally boot-test in QEMU**

  Run: `./run-qemu.sh` and interrupt with `Ctrl+A X` or `Ctrl+C` after confirming serial output begins.
  Expected: QEMU starts and the ISO is loaded.

- [ ] **Step 5: Final commit if any remaining changes exist**

  If `git status` shows uncommitted changes, commit them with an appropriate message; otherwise this step is a no-op.

  ```bash
  git status --short
  # if there are changes:
  # git add ... && git commit -m "..."
  ```

---

## Self-Review

- **Spec coverage:**
  - Delete `tools/build-disk-image.sh` → Task 1
  - Simplify `build.sh` → Task 2
  - Simplify `tools/build-image.sh` → Task 3
  - Update `.github/workflows/daily-build.yml` → Task 4
  - Rewrite `run-qemu.sh` → Task 5
  - Update `.gitignore` → Task 6
  - Update `README.md` → Task 7
  - Delete `target-disk.img` and `target-disk-test.img` → Task 8
  - Verification → Task 9

- **Placeholder scan:** All steps include exact file paths, exact code, and exact commands. No "TBD", "TODO", or vague guidance remains.

- **Type consistency:** This is a Bash/YAML project; interface contracts are shell command signatures, which are repeated exactly where used.
