# Win32 / NT Compatibility

This document describes the Win32 subsystem implementation status, the PE
loader, and how DLL import resolution is intended to work. It is honest about
what is implemented versus planned.

## Design

Aperture OS does **not** wrap Wine, Proton, or a host Windows installation.
It implements the Windows NT kernel ABI natively in Rust:

- A **PE loader** parses PE32/PE32+ images and maps them into per-process
  address spaces.
- An **NT syscall dispatch table** routes user-mode traps to native handlers.
- An **object manager** provides handles for processes, threads, files, keys,
  and desktops.
- **System DLLs** (ntdll, kernel32, user32, gdi32, comctl32, comdlg32) are
  planned as **built-in native shims** — Rust code compiled natively per
  architecture, recognized by the PE loader so imports resolve without
  external `.dll` files. They are never translated.

When a translated external binary imports a system DLL, the import thunk
routes to the native shim, which runs at full native speed (the FEX-Emu /
Rosetta 2 model). Only the external binary's own code is translated.

## PE loader (`kernel/src/win32/loader.rs`, `crates/pe-parser/`)

| Feature | Status |
|---|---|
| MZ / PE / optional header parse | ✅ |
| PE32 and PE32+ (64-bit) headers | ✅ |
| Section table parse + mapping to virtual addresses | ✅ |
| Import directory parse + logging | ✅ |
| Import thunk (IAT/INT) parse | ✅ (parser) |
| Base relocations | ❌ |
| TLS callbacks / directory | ❌ |
| Export table resolution | ❌ |
| Delay-load imports | ❌ |
| Resource directory | ❌ |
| PEB / TEB setup | ❌ |
| DllMain `DLL_PROCESS_ATTACH` | ❌ |
| Entry point execution | ❌ (synthetic self-test only) |
| Architecture check (native vs translate) | ✅ (`requires_translation`) |

The loader maps sections into a per-process x86_64 page table (not yet
activated) and now parses + logs the import directory. Import *resolution*
to built-in shims is the next step.

## NT syscall coverage (`kernel/src/win32/nt.rs`)

9 of 16 dispatched syscalls are wired; the remainder return
`STATUS_NOT_IMPLEMENTED`.

| Syscall | Status |
|---|---|
| `NtClose` | ✅ |
| `NtCreateFile` | ✅ |
| `NtReadFile` | ✅ |
| `NtWriteFile` | ✅ |
| `NtAllocateVirtualMemory` | ✅ |
| `NtFreeVirtualMemory` | ✅ |
| `NtQuerySystemInformation` | ✅ |
| `NtQueryInformationProcess` | ✅ |
| `NtDelayExecution` | ✅ |
| `NtCreateProcess` / `NtCreateThread` / `NtTerminate*` | ❌ |
| `NtProtectVirtualMemory` / `NtQueryVirtualMemory` | ❌ |
| `NtCreateEvent` / `NtCreateMutant` / `NtCreateSemaphore` / `NtWait*` | ❌ |
| `NtCreateKey` / `NtOpenKey` / `NtSetValueKey` / `NtQueryValueKey` | ❌ |
| `NtCreateSection` / `NtMapViewOfSection` | ❌ |
| `NtCreatePort` / LPC / named pipes | ❌ |
| `NtRaiseException` / `NtContinue` | ❌ |

`dispatch` performs real user-pointer-to-physical translation. The syscall
numbers and `NTSTATUS` values aim to match Windows; full fidelity is ongoing.

## Object manager (`kernel/src/win32/objects.rs`)

1024-slot handle table with allocate / lookup / close and object kinds
(Process, Thread, File, Key, Event, …). **Functional.**

## Registry (`kernel/src/win32/registry.rs`)

In-memory flat 256-slot shim exposing HKCU/HKLM-style keys. No hive files, no
persistence, no full value-type set. **Partial.**

## System DLL shims

**Not yet implemented.** The intended surface (per Windows):

- **ntdll.dll**: `Nt*` syscall stubs (x86_64 `syscall` / AArch64 `svc #0`).
- **kernel32.dll**: `CreateFileW`, `ReadFile`, `WriteFile`, `CloseHandle`,
  `CreateProcessW`, `CreateThread`, `LoadLibraryW`, `GetProcAddress`,
  `ExitProcess`, `Sleep`, `GetTickCount`, console APIs, etc.
- **user32.dll**: `CreateWindowExW`, `ShowWindow`, `GetMessage`,
  `DispatchMessageW`, `DefWindowProcW`, `MessageBoxW`, etc.
- **gdi32.dll**: device contexts, drawing primitives, GDI objects, BitBlt.
- **comctl32.dll** / **comdlg32.dll**: common controls and dialogs.

## Known limitations

- No real Windows `.exe` execution yet (only a synthetic self-test binary).
- No import resolution, base relocations, or TLS.
- No PEB/TEB, no DllMain dispatch.
- Registry is a non-persistent in-memory shim.
- 7 of 16 NT syscall classes are unimplemented.

## Testing Windows applications

Until import resolution and system DLL shims land, real Windows applications
do not yet run. The host-testable path today is the `pe-parser` unit tests
(`cargo test -p pe-parser`), which validate PE header/import parsing against
hand-crafted PE images. The `tools/gen_minimal_pe64.py` script generates a
minimal PE64 used for loader self-tests.