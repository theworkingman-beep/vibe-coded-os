# Binary Translation

Aperture OS includes a built-in binary translator so that **externally
downloaded** Windows PE binaries run on any host architecture, regardless of
the binary's machine type. This is a compatibility feature for third-party
software — **the OS itself is never translated.**

## The critical distinction

- The OS's own components (kernel, drivers, system DLLs, desktop, built-in
  apps) are compiled for and run natively on the host architecture. They are
  never translated.
- Translation is invoked **only** when an externally loaded PE binary's
  machine type does not match the host.

| Guest PE | x86_64 host | AArch64 host |
|---|---|---|
| x86_64 (AMD64) | native | JIT / interpreter |
| x86 (32-bit) | WoW64 | nested translation |
| ARM64 | JIT / interpreter | native |
| ARM (32-bit) | JIT / interpreter | JIT / interpreter |

## How it works (design)

1. The PE loader checks the machine type (`IMAGE_FILE_MACHINE_AMD64`,
   `I386`, `ARM64`, `ARM`) via `requires_translation`.
2. If it matches the host → native execution. If not → the translator is
   invoked.
3. The translator reads guest instructions, translates them to host
   instructions (JIT per basic block, cached), or interprets them, and
   executes in the same process address space.
4. **Syscalls**: the guest's syscall instruction is translated to the host's
   (`x86_64 SYSCALL` ↔ `AArch64 SVC #0`) with the same syscall number; the
   kernel's NT syscall handler is architecture-independent at the semantic
   level.
5. **System DLL calls**: calls to imported system DLL functions route to the
   native built-in shims, which run at full native speed. Only the external
   binary's own code is translated. This is the FEX-Emu / Rosetta 2 model.
6. The process looks identical to the kernel whether guest code runs
   natively or translated (same PEB/TEB, handles, memory, syscalls).

## Current status

The translation infrastructure is **scaffolded**, not functional. v1.0.0
does not yet run translated Windows binaries.

| Component | File | Status |
|---|---|---|
| x86_64 decoder | `crates/x86-decode` | Partial (NOP/RET/JMP/SYSCALL + more), unit-tested |
| AArch64 decoder | `crates/aarch64-decode` | Partial (NOP/RET/SVC/BL/MOVZ/ADRP), unit-tested |
| x86_64 interpreter | `win32/abi/interpreter.rs` | Partial — decodes NOP/RET/JMP/CALL/MOV imm/XOR/LEA/SYSCALL, updates guest regs, halts on unsupported. Boot self-test runs `mov rax,5; mov rbx,3; xor rax,rbx` and verifies steps/rax/rbx |
| AArch64 interpreter | `win32/abi/aarch64_interpreter.rs` | Scaffold — decodes + logs a few instructions, no reg emulation |
| x86_64→AArch64 JIT | `win32/abi/x86_jit.rs` | Scaffold — `translate_block` placeholder, no emission |
| ARM64→x86_64 JIT | `win32/abi/aarch64_jit.rs` | Scaffold — placeholder |
| Syscall helper | `win32/abi/syscall.rs` | Functional inline-asm user-mode syscall |
| Translation manager | — | Not yet implemented |
| Code caching | — | Not yet implemented |
| Self-modifying-code handling | — | Not yet implemented |
| PGO / AOT | — | Stretch goals |

## Priority order (roadmap)

1. x86_64 → AArch64 JIT (runs the majority of Windows apps on ARM64 hosts).
2. x86 (32-bit) → x86_64 WoW64 and → AArch64 nested translation.
3. ARM64 → x86_64 JIT (reverse direction).
4. Interpreter fallback completeness so unsupported JIT instructions still
   run (slowly) instead of faulting.
5. Code caching, then PGO / AOT for performance.

## Adding a new instruction translation

1. Extend the relevant decoder crate (`x86-decode` / `aarch64-decode`) and
   add a unit test (`cargo test -p <crate>`).
2. Add the instruction's semantics to the interpreter loop (the fallback).
3. Add the JIT emit path, including flag-register handling (x86_64 RFLAGS ↔
   AArch64 NZCV do not map 1:1), segmented TLS access (FS/GS ↔
   TPIDR_EL0), and the guest stack-based call/return convention.
4. Translate the guest syscall instruction to the host's with the same
   syscall number, and route imported system DLL calls to the native shims.

## Performance

Native execution (arch matches host) is full speed. Translated execution
will be slower until the JIT matures; the interpreter fallback ensures
correctness first. The roadmap follows FEX-Emu: JIT basic blocks, cache
translations, profile-guided optimization, and optional AOT pre-translation.