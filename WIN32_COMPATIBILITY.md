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
  **built-in native shims** — Rust code compiled natively per architecture,
  recognized by the PE loader so imports resolve without external `.dll`
  files. They are never translated. ntdll and kernel32 shims are implemented
  (see below); the rest are planned.

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
| Import resolution → built-in DLL shims | ✅ |
| Base relocations | ❌ |
| TLS callbacks / directory | ❌ |
| Export table resolution | ❌ |
| Delay-load imports | ❌ |
| Resource directory | ❌ |
| PEB / TEB setup | ✅ (native copy on `Process`) |
| DllMain `DLL_PROCESS_ATTACH` | ❌ |
| Entry point execution | ❌ (synthetic self-test only) |
| Architecture check (native vs translate) | ✅ (`requires_translation`) |

The loader maps sections into a per-process x86_64 page table (not yet
activated), parses the import directory, and resolves each import thunk
against the built-in shim registry (`shims.rs`), logging resolved/unresolved
counts. Base relocations, TLS, and external `.dll` loading remain.

## NT syscall coverage (`kernel/src/win32/nt.rs`)

All 16 dispatched syscalls are wired; handlers that are not yet fully
semantic return a best-effort result rather than `STATUS_NOT_IMPLEMENTED`.

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
| `NtCreateProcess` | ✅ |
| `NtCreateThread` | ✅ |
| `NtSetInformationProcess` | ✅ |
| `NtCreateKey` | ✅ |
| `NtSetValueKey` | ✅ |
| `NtQueryValueKey` | ✅ |
| `NtWaitForMultipleObjects` | ✅ |
| `NtProtectVirtualMemory` / `NtQueryVirtualMemory` | ❌ (not dispatched) |
| `NtCreateEvent` / `NtCreateMutant` / `NtCreateSemaphore` | ❌ (not dispatched) |
| `NtCreateSection` / `NtMapViewOfSection` | ❌ (not dispatched) |
| `NtCreatePort` / LPC / named pipes | 🚧 (port object + send/receive self-test) |
| `NtRaiseException` / `NtContinue` | ❌ (not dispatched) |

`dispatch` performs real user-pointer-to-physical translation. The registry
trio (`NtCreateKey`/`NtSetValueKey`/`NtQueryValueKey`) is verified by a boot
self-test (HKLM\Software\Aperture "Version" round-trip). The syscall numbers
and `NTSTATUS` values aim to match Windows; full fidelity is ongoing.

## Object manager (`kernel/src/win32/objects.rs`)

1024-slot handle table with allocate / lookup / close and object kinds
(Process, Thread, File, Key, Event, …). **Functional.**

## Registry (`kernel/src/win32/registry.rs`)

In-memory flat 256-slot shim exposing HKCU/HKLM-style keys with create/set/
query. Verified by a boot self-test (HKLM\Software\Aperture "Version" =
"1.0.0" REG_SZ round-trip). No hive files, no persistence, no full value-type
set. **Partial.**

## Process environment (`kernel/src/win32/process.rs`)

`Process` carries a native PEB/TEB copy and a case-insensitive environment
block (`get_env`/`set_env`, matching `GetEnvironmentVariableW` semantics).
Verified by a boot self-test. **Partial** (no `RTL_USER_PROCESS_PARAMETERS`
serialization yet).

## IPC message ports (`kernel/src/win32/port.rs`)

A minimal LPC-style message port: `send`/`try_receive` round-trip with a
closed-port reject path, verified by a boot self-test. **Partial.**

## System DLL shims (`kernel/src/win32/shims.rs`)

The built-in shim registry maps `(dll, export)` pairs to native Rust
handlers. Resolution is case-insensitive and path-stripped, so `kernel32`,
`KERNEL32.DLL`, and `C:\Windows\System32\kernel32.dll` all match. A boot
self-test resolves a representative import from each DLL and confirms an
unknown export does not resolve.

Implemented shims:

- **ntdll.dll**: `NtDelayExecution`, `NtQuerySystemInformation`,
  `NtAllocateVirtualMemory`, `NtFreeVirtualMemory`, `NtClose` (thin syscall
  stubs returning the NTSTATUS in RAX).
- **kernel32.dll**: `Sleep` (forwards to `NtDelayExecution`), `GetTickCount`,
  `GetTickCount64` (monotonic cycles → ms), `ExitProcess`, `GetLastError`.

Planned (not yet implemented):

- **user32.dll**: `CreateWindowExW`, `ShowWindow`, `GetMessage`,
  `DispatchMessageW`, `DefWindowProcW`, `MessageBoxW`, etc. (the window
  model exists in `win32k.rs`; the user32 shim surface is not wired).
- **gdi32.dll**: device contexts, GDI objects, BitBlt (GDI *primitives*
  exist in the compositor; the gdi32 shim surface is not wired).
- **comctl32.dll** / **comdlg32.dll**: common controls and dialogs.

## Win32 window model (`kernel/src/win32/win32k.rs`)

`WindowClass` / `Wnd` / `Message` with `register_class`, `create_window_ex`,
`post_message`, `get_message`, `def_window_proc`, `dispatch_message`, and
`WM_CREATE`/`WM_PAINT`/`WM_DESTROY`/`WM_CLOSE`/`WM_KEYDOWN`/`WM_LBUTTONDOWN`.
A boot self-test registers a class, creates a window, posts and retrieves a
`WM_PAINT` message, and dispatches it. **Partial** (no per-class wndproc,
no `ShowWindow`/`UpdateWindow`).

## Known limitations

- No real Windows `.exe` execution yet (only a synthetic self-test binary).
- No base relocations, TLS, or external `.dll` loading.
- No DllMain dispatch or entry-point execution.
- Registry and ports are non-persistent in-memory shims.
- user32/gdi32 shim surfaces not wired (primitives exist behind them).

## Testing Windows applications

Real Windows applications do not yet run (no base relocations, TLS, or
entry-point execution). The host-testable paths today are the `pe-parser`
and `part-parser` unit tests (`cargo test -p pe-parser -p part-parser`),
which validate PE header/import parsing and MBR/GPT parsing against
hand-crafted images. The `tools/gen_minimal_pe64.py` script generates a
minimal PE64 used for loader self-tests. In-kernel subsystems (ports, shims,
interpreter, registry, env, GDI, win32k) run boot-time self-tests logged on
the serial console.