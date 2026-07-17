# Contributing to Aperture OS

Aperture OS is a Rust `no_std` kernel targeting `x86_64-unknown-none` and
`aarch64-unknown-none-softfloat`. Contributions that advance the roadmap in
[README.md](README.md) are welcome.

## Code style

- Run `cargo fmt --all` before committing. rustfmt defaults are the project
  style.
- Run `cargo clippy` on the host crates (`pe-parser`, `x86-decode`,
  `aarch64-decode`) and keep them warning-free. The kernel crate silences
  `dead_code` crate-wide because many register constants and driver methods
  are defined ahead of the phases that wire them up; prefer a targeted
  `#[allow(dead_code)]` only when an item is genuinely temporary.
- Use `Result` types and `NTSTATUS`-style error codes in the Win32/syscall
  layer. Reserve `panic!` for unrecoverable kernel-state corruption; drivers
  and the translator must degrade gracefully (log + continue, or interpreter
  fallback) rather than panic.
- Mirror Windows struct/function names where they correspond to Windows
  types (`EPROCESS`, `PEB`, `TEB`, `NtCreateFile`, `CONTEXT`, etc.).
- Document public items with `///` doc comments.

## Architecture rule (critical)

Architecture-specific code lives in `kernel/src/arch/x86_64/` or
`kernel/src/arch/aarch64/`. Architecture-independent code (scheduler core,
VFS, Win32 subsystem, NT syscall semantics, PE parsing, GDI, compositor)
lives in shared modules and is compiled natively for whichever target is
selected. **Never put architecture-specific code in shared modules** — use
`cfg`-gated dispatch or trait objects. The OS is never translated; only
externally downloaded Windows PE binaries are.

## Adding a new NT syscall

1. Add the syscall number constant in `kernel/src/win32/nt.rs` (match the
   Windows NT numbering — see the WDK headers / ReactOS for reference).
2. Add an entry to the `SYSCALL_TABLE` dispatch array and a handler function.
   Handlers receive raw user pointers; translate them to physical/HHDM
   addresses via the existing helpers in `nt.rs::dispatch`.
3. Return an `NTSTATUS` code matching Windows semantics.
4. The same handler serves both architectures (x86_64 `SYSCALL` and AArch64
   `SVC` route to the same table). Only the entry/exit glue is arch-specific.

## Adding a new Win32 API function

System DLLs (ntdll, kernel32, user32, gdi32, comctl32, comdlg32) are planned
as **built-in native shims** recognized by the PE loader — not external PE
files. To add a function:

1. Implement it in the relevant `kernel/src/win32/` module as native Rust.
2. Export it from the built-in shim so the PE loader resolves imports against
   it. When a translated external binary calls the function, the import thunk
   routes to this native implementation (it runs at full native speed).
3. If the function needs a new NT syscall, add that first (above).

## Adding a new GDI operation

Add the primitive to `kernel/src/gui/compositor.rs` (or a future `gdi.rs`).
GDI is architecture-independent — it operates on the framebuffer / device
contexts, not on the CPU. Keep it in shared code.

## Adding a new instruction translation

The binary translators live in `kernel/src/win32/abi/`:

- `x86-decode` / `aarch64-decode` crates: extend the decoder for the new
  instruction and add a unit test in the crate (`cargo test -p <crate>`).
- `interpreter.rs` (x86_64 guest): add the instruction's semantics to the
  interpreter loop, updating the guest register file. This is the fallback
  for anything the JIT cannot yet emit.
- `x86_jit.rs` / `aarch64_jit.rs`: add the JIT emit path. The JIT must
  translate the guest's syscall instruction to the host's
  (`SYSCALL` <-> `SVC #0`) with the same syscall number, and route calls to
  imported system DLL functions to the native shims.

Never translate the OS's own components — translation is only for externally
loaded PE binaries whose machine type does not match the host.

## Adding a new hardware driver

1. Put the driver under `kernel/src/disk/`, a future `kernel/src/drivers/`,
   or the arch HAL as appropriate. Drivers run **natively** on both
   architectures.
2. Discover hardware dynamically from firmware-provided information (ACPI,
   DTB, UEFI, PCI enumeration) — never hardcode device addresses.
3. Every device access must have a timeout, a null check, an error-recovery
   path, and a log message. See `kernel/src/time.rs::poll_with_timeout`.
4. Handle device absence gracefully: log a warning and continue boot.

## Testing

- Add host-testable unit tests to the `pe-parser`, `x86-decode`, and
  `aarch64-decode` crates where possible (`cargo test`).
- For kernel behavior, boot the ISO in QEMU and check the serial/semihosting
  log for the expected `logln!` lines.
- The `daily-build.yml` CI builds both ISOs and boots them in QEMU.

## Committing

Commit after each significant sub-task with a descriptive message. The project
uses `Phase N WIP:` prefixes during bring-up. Keep `build/` and `target/`
untracked (they are in `.gitignore`).