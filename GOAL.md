📋 Master Prompt: Complete ApertureOS from Boot-to-Desktop with Full Windows Program Support, Dual-Architecture (x86_64 + AArch64) Native Execution, and Cross-Architecture Translation for External Programs
Context
You are working on ApertureOS (https://github.com/theworkingman-beep/ApertureOS), an experimental operating system written in Rust targeting both x86_64 and AArch64 architectures. The project is currently in early bring-up. The latest ISO barely boots into a GRUB-like interface and nothing further happens. The goal is to transform this into a complete, usable, general-purpose operating system comparable in user experience to Linux, Windows, or macOS — one that boots from an ISO to a working graphical desktop, can run programs, manage files, and handle user interaction.
Four core, non-negotiable goals define this project:
Goal 1: Full Native Windows Application Compatibility
ApertureOS does not wrap Wine, Proton, or another OS. It implements the NT kernel ABI natively in Rust with a Win32 subsystem, so that real Windows PE executables (.exe) run as first-class processes — the same way they would on Windows itself, but with a clean Rust-native kernel underneath. This means the OS must be able to load, execute, and support real-world Windows applications (games, utilities, productivity software) using their native PE binary format and Win32 API calls, dispatched through a native NT syscall layer.
Goal 2: First-Class Dual-Architecture Support — The OS Itself Runs Natively on Both x86_64 and AArch64
The OS must boot, run, and be fully functional on both x86_64 and AArch64 hardware — natively, with no translation or emulation of the OS's own components. This is not "x86_64 with AArch64 stubs" — both architectures are first-class targets. The kernel, drivers, system services, desktop environment, system DLLs (ntdll, kernel32, user32, gdi32, etc.), built-in applications (shell, file manager, task manager, text editor, terminal), and all OS components are compiled natively for each target architecture. On x86_64 hardware, everything runs as x86_64 code. On AArch64 hardware, everything runs as AArch64 code. No part of the OS itself is ever translated or emulated. The kernel must compile and boot on both, with architecture-specific boot paths, HAL implementations, and interrupt handling. The AArch64 port must be a real, working port — not a compilation stub. The build system must produce bootable ISOs/disk images for both architectures.
Goal 3: Cross-Architecture Binary Translation (FEX-Style) — Only for Externally Downloaded Programs
The OS must run any externally downloaded Windows PE binary on any host architecture, regardless of what architecture that binary was compiled for. This means:
	•	x86 PE (32-bit) on x86_64 host — via WoW64-style translation
	•	x86_64 PE on AArch64 host — via JIT translation or interpreter (like FEX-Emu does for Linux)
	•	x86 PE on AArch64 host — via nested translation (x86 → x86_64 → AArch64) or direct x86-to-AArch64 translation
	•	ARM64 PE on x86_64 host — via JIT translation or interpreter (reverse direction)
	•	x86_64 PE on x86_64 host — native execution (fastest path)
	•	ARM64 PE on AArch64 host — native execution (fastest path)
Critical distinction: The translation layers are ONLY for externally downloaded/third-party Windows programs that don't match the host architecture. The OS itself — its kernel, drivers, system DLLs, desktop environment, and built-in applications — always runs natively. The translation is a userland compatibility feature for running third-party software, not a mechanism for running the OS's own components. Think of it this way:
	•	The OS is a native x86_64 OS on x86_64 hardware. It happens to be able to also run x86 (32-bit), ARM64, and (trivially) x86_64 Windows .exe files.
	•	The OS is a native AArch64 OS on AArch64 hardware. It happens to be able to also run x86, x86_64, and (trivially) ARM64 Windows .exe files.
	•	The OS's own .exe and .dll files are always compiled for and run natively on the host architecture. They are never translated.
This is what makes ApertureOS unique: it is not tied to one CPU architecture, and it does not need an x86_64 host to run the majority of its functionality. A user running ApertureOS on an ARM64 laptop (e.g., Apple Silicon, Snapdragon, Raspberry Pi 5) gets a fully native AArch64 OS that can also run Windows x86_64 applications via translation. A user running on x86_64 gets a fully native x86_64 OS that can also run ARM64 Windows applications via translation. The compatibility layer architecture is inspired by FEX-Emu (which runs x86/x86_64 Linux binaries on AArch64) and Apple's Rosetta 2 (which runs x86_64 binaries on ARM64) — but built into the OS at the kernel level rather than as a user-space emulation layer, and only invoked when a PE binary's machine type doesn't match the host.
Goal 4: Boot and Run on Real Hardware (Not Just VMs)
The OS must boot and be fully functional on real physical hardware, not just in QEMU/VirtualBox/VMware. This means the kernel must handle the complexity and diversity of real-world hardware: ACPI tables, UEFI firmware variants, real PCI/PCIe device enumeration, real USB host controllers, real storage controllers (AHCI/NVMe/SD/eMMC), real GPU/framebuffer discovery, real interrupt controllers (APIC/GIC), real timers (HPET/LAPIC timer/ARM architectural timer), real clock sources (RTC/UEFI runtime), and the wide range of memory maps, IOMMU configurations, and firmware quirks that real machines present. The kernel cannot assume the clean, predictable environment that QEMU provides. Every driver and subsystem must be written to handle real hardware behavior, including device timeouts, missing devices, unexpected ACPI table formats, and firmware bugs. QEMU is a development and testing tool, not the deployment target. The deployment target is real hardware.
The repository is written in Rust (90.9%) with some Python and Shell tooling. It uses the nightly Rust toolchain, x86_64-unknown-none and aarch64-unknown-none targets, and builds bootable disk images via tools/bootimage/. The existing kernel/win32/directory contains skeleton code for the PE loader, NT syscall dispatch, object manager, process model, scheduler, thread model, registry shim, and Win32k GUI bridge. The existing kernel/arch/aarch64/ contains stubs that must be fully fleshed out.
Current State
	•	✅ Bootable x86_64 skeleton with serial output and framebuffer
	•	✅ HAL with IDT/PIC, early heap, GUI compositor skeleton, Windows subsystem skeleton
	•	✅ Keyboard input, timer, bitmap frame allocator, virtual memory page tables (x86_64 only)
	•	✅ Cooperative scheduler and x86_64 context switch (x86_64 only)
	•	✅ SYSCALL/SYSRET entry and NT syscall dispatch table (x86_64 only)
	•	✅ PE loader that maps images into per-process address spaces (synthetic test executable only)
	•	✅ In-memory VFS backing NT file syscalls
	•	❌ Real hardware boot support (ACPI, UEFI runtime, real device enumeration) — not implemented
	•	❌ AArch64 boot path (UART, MMU, framebuffer, interrupt controller) — stubs only, does not boot
	•	❌ AArch64 HAL (GIC, timer, exception handling, page tables) — not implemented
	•	❌ AArch64 SYSCALL entry/exit — not implemented
	•	❌ AArch64 PE loading and execution — not implemented
	•	❌ AArch64 native system DLLs and built-in applications — not implemented (the OS must run natively on AArch64, meaning all system components must be compiled for and run as AArch64 code)
	•	❌ Preemptive multitasking and SMP — not implemented on either arch
	•	❌ Cross-architecture binary translation (JIT + interpreter) — not implemented (needed only for external/third-party PE binaries that don't match host arch)
	•	❌ Full NT syscall coverage and Win32 API server — not implemented
	•	❌ VFS persistence, registry, and driver model — not implemented
	•	❌ Hardware-accelerated GUI compositor — not implemented
	•	❌ Userland desktop environment — not implemented
	•	❌ Real driver stack (storage, network, USB, audio, GPU) — not implemented
	•	❌ Interactive shell / terminal / file manager — not implemented
	•	❌ Real Windows PE executable loading and execution — only synthetic test executable
	•	❌ Win32 API surface (GDI, User32, Kernel32, etc.) — not implemented
	•	❌ Windows registry (real implementation, not shim) — not implemented
	•	❌ DLL loading and PE import resolution — not implemented
	•	❌ Windows-compatible process/thread model — not implemented
	•	❌ FEX-style JIT translator for x86_64 → AArch64 — not implemented
	•	❌ FEX-style JIT translator for x86 → AArch64 — not implemented
	•	❌ JIT translator for ARM64 → x86_64 — not implemented
	•	❌ WoW64-style x86 → x86_64 translation — not implemented
Primary Objective
Deliver a bootable, functioning operating system that:
	1	Boots from an ISO (on a USB stick or CD) into a graphical desktop environment on BOTH x86_64 and AArch64 real hardware, where a user can interact with windows, launch programs, browse files, and see the system responding in real time. The OS must also work in QEMU for development and CI, but the goal is real hardware boot. The OS itself runs natively on whichever architecture it's booted on — no translation of OS components.
	2	Can load and execute real Windows PE executables (.exe files) — not just synthetic test binaries — including programs that use the Win32 API (message boxes, file I/O, window creation, GDI drawing, console output, etc.). When the PE binary matches the host architecture (e.g., x86_64 PE on x86_64 host, or ARM64 PE on AArch64 host), it runs natively with full performance. When it does not match (e.g., x86_64 PE on AArch64 host), it runs via the binary translator.
	3	Provides a functioning Win32 subsystem with enough API coverage that common Windows applications (console apps, simple GUI apps, system utilities) run natively (when arch matches) or via translation (when arch doesn't match) — without Wine, without a host Windows installation, and without any external compatibility layer.
	4	Runs externally downloaded Windows PE binaries across architectures: A user on an AArch64 ApertureOS machine downloads an x86_64 Windows .exe from the internet and runs it via the JIT translator. A user on an x86_64 ApertureOS machine downloads an ARM64 Windows .exe and runs it via the JIT translator. An x86 (32-bit) .exe runs on either architecture via the WoW64 translation layer. The architecture of the host does not limit which third-party Windows applications can be run.
	5	Is a real dual-architecture OS: The same source tree compiles, boots, and runs full-featured on both x86_64 and AArch64. The build system produces bootable ISOs for both. The AArch64 port is not a second-class citizen — it has the same features as x86_64. All OS components (kernel, drivers, DLLs, apps) are compiled natively per architecture.
	6	Boots on real hardware: The kernel properly parses ACPI tables (x86_64) or Device Tree (AArch64), enumerates real PCI/PCIe devices, initializes real interrupt controllers (APIC/GIC), discovers real storage controllers and mounts real filesystems, initializes real USB controllers and handles real input devices, and drives real framebuffers or GPUs. The kernel does not crash on unexpected hardware, does not hang on missing devices, and degrades gracefully when optional hardware is absent.
The ISO must boot (via BIOS and/or UEFI for x86_64; via UEFI for AArch64) all the way into a usable GUI shell without dropping to a GRUB prompt or hanging. Testing in QEMU is required for CI, but the final product must be validated on at least one real x86_64 machine and at least one real AArch64 machine (e.g., Raspberry Pi 5, or an ARM64 laptop/board).

Phased Work Plan
PHASE 1: Fix the Boot Path on Both Architectures — Including Real Hardware (Critical / Blocking)
Goal: The ISO boots reliably into the kernel on BOTH x86_64 and AArch64, in both QEMU AND on real hardware. Initializes all hardware, and displays output. No GRUB prompt, no hang, no blank screen. The kernel runs natively on each architecture.
1A: x86_64 Boot Fix (QEMU + Real Hardware)
	1	Audit and fix tools/bootimage/ to ensure the bootimage tool correctly wraps the kernel ELF into both BIOS (Multiboot2) and UEFI bootable disk images. Test both paths in QEMU first, then on real hardware.
	2	Ensure GRUB or Limine is properly configured in the generated ISO. If using GRUB, embed a correct grub.cfg that has a default menu entry with zero-second timeout (or 1-second timeout with a default entry) that boots the kernel. If the ISO drops to a GRUB shell, this is the #1 bug to fix. If using Limine, ensure the limine.cfg is correct and the kernel is properly identified.
	3	Verify the boot protocol: The kernel's entry point must match what the bootloader expects. If using Multiboot2, ensure the Multiboot2 header is properly placed in the first 32KB of the ELF. If using UEFI, ensure the .efi PE entry point is correct.
	4	Implement proper boot information parsing:
	◦	Multiboot2: Parse the Multiboot2 info structure passed by GRUB — memory map, framebuffer info, ACPI RSDP, boot modules, ELF sections, cmdline.
	◦	UEFI: Parse UEFI memory map, UEFI GOP framebuffer info, ACPI RSDP (from UEFI configuration table), UEFI runtime services (for time, variables, reset). The kernel must correctly transition from UEFI boot services environment to its own environment, including taking ownership of memory regions, the framebuffer, and runtime services.
	◦	Real hardware consideration: Real UEFI firmware varies significantly between vendors (AMI, Insyde, Dell, HP, Lenovo, Apple). The kernel must not assume any particular UEFI implementation behavior. Handle missing/extra configuration tables, non-standard memory map entries, and firmware that leaves devices in unexpected states. Call gBS->ExitBootServices() correctly and handle the case where the firmware requires a second attempt (timer map key changed).
	5	Implement ACPI table parsing (critical for real hardware):
	◦	Find RSDP (from UEFI config table, or from EBDA / BIOS ROM area scan for legacy BIOS boot)
	◦	Parse RSDT/XSDT to enumerate all ACPI tables
	◦	Parse FADT (for ACPI register base, PM1a/PM1b control, hardware reduced flag, reset register)
	◦	Parse DSDT/SSDT (AML bytecode — at minimum, parse to find device scopes and _ADR/_HID/_UID; full AML interpreter is a later phase but basic parsing is needed for device discovery)
	◦	Parse MADT (for APIC configuration: local APIC addresses, IOAPIC entries, interrupt source overrides, local APIC NMI, processor LAPIC IDs for SMP bring-up)
	◦	Parse HPET table (for high-precision event timer base address)
	◦	Parse MCFG (for PCIe ECAM base address — critical for real PCI enumeration)
	◦	Parse SRAT (for NUMA topology — optional, but good for real server hardware)
	◦	Parse BGRT (for UEFI boot logo / splash screen — optional)
	◦	Real hardware consideration: ACPI tables on real hardware are complex, may have vendor-specific tables, may have malformed entries, may have multiple IOAPICs, may have _OSC method requirements for PCIe access. The parser must be robust — skip unknown tables, handle truncated tables, don't panic on unexpected data.
	6	Add a fallback kernel panic handler with a visible error message drawn to the framebuffer (not just serial) so that if the kernel crashes during boot, the user sees a diagnostic screen rather than a blank/frozen display.
	7	Test boot on QEMU with: qemu-system-x86_64 -cdrom build/ApertureOS-x86_64.iso -serial stdio and ensure the kernel prints boot messages and reaches the idle loop.
	8	Test boot on QEMU UEFI with: qemu-system-x86_64 -cdrom build/ApertureOS-x86_64.iso -bios OVMF.fd -serial stdio.
	9	Test boot on real hardware: Burn the ISO to a USB stick and boot on at least two real x86_64 machines (e.g., a desktop with UEFI, a laptop with UEFI, and/or a legacy BIOS machine). Verify the kernel boots, parses ACPI tables, initializes the framebuffer, and reaches the idle loop. Document which hardware was tested, any issues encountered, and workarounds applied.
1B: AArch64 Boot Implementation (QEMU + Real Hardware)
	1	Implement the AArch64 boot path: The kernel must boot from a UEFI bootloader (GRUB EFI for AArch64, or Limine, or the kernel's own PE/EFI stub). The AArch64 entry point must set up exception level (EL1 for kernel, handling EL2/EL1 transition if started at EL2 by hypervisor firmware), initialize the stack pointer, and set up exception vectors.
	◦	This is the AArch64 native kernel — all kernel code runs as native AArch64 instructions. No translation is involved in booting or running the OS itself.
	◦	Real hardware consideration: AArch64 hardware (Raspberry Pi, Snapdragon, Apple Silicon) starts at different exception levels and with different boot mechanisms. Raspberry Pi uses a custom bootloader (not UEFI unless using a UEFI firmware shim like EDK2 for RPi). Apple Silicon uses m1n1/U-Boot. The kernel should support being loaded by a standard UEFI bootloader as the primary path, with the understanding that on some AArch64 boards a UEFI firmware shim may be needed.
	2	Implement AArch64 early hardware initialization:
	◦	UART output (PL011 for QEMU virt and Raspberry Pi, or a generic serial driver). Support PL011, 16550 (common on x86-style serial ports on ARM boards), and mini UART (Raspberry Pi).
	◦	MMU and page table setup (AArch64 uses 4KB or 64KB pages, 3-4 level page tables with TTBR0_EL1/TTBR1_EL1 for user/kernel split, TCR_EL1 configuration, MAIR_EL1 for memory attributes). Must handle real hardware memory maps — the memory map comes from UEFI (for UEFI boot) or from the Device Tree (for non-UEFI boot).
	◦	Framebuffer initialization (via UEFI GOP, or via Device Tree /devicetree framebuffer node, or via virtio-gpu for QEMU).
	◦	GIC (Generic Interrupt Controller) initialization — detect GIC version (v2 vs v3) from ACPI/DTB, initialize distributor and redistributor interfaces, set up interrupt routing.
	◦	Architectural timer setup (CNTV_CTL_EL0 / CNTV_TVAL_EL0 for the virtual timer, or CNTP_CTL_EL0 for the physical timer). The timer frequency comes from CNTFRQ_EL0 (set by firmware) or from the Device Tree.
	3	Implement AArch64 exception handling: Set up the exception vector table (VBAR_EL1), with handlers for synchronous exceptions (data abort, instruction abort, SVC instruction, alignment fault, SError), IRQs, FIQs. Map these to the same kernel-level exception handling infrastructure as x86_64.
	4	Implement AArch64 SYSCALL entry/exit: Use the SVC #0 instruction as the syscall entry point. The handler reads the syscall number from x8 (or a convention register) and dispatches to the NT syscall table. Implement the equivalent of x86_64's SYSCALL/SYSRET using AArch64's ERET to return to EL0.
	5	Implement Device Tree (FDT) parsing (critical for real AArch64 hardware):
	◦	Parse the Flattened Device Tree (DTB) blob passed by the bootloader or firmware
	◦	Extract memory regions, timer frequency, interrupt controller (GIC) base addresses, UART base addresses, framebuffer info, PCI ECAM base, USB controller nodes, storage controller nodes
	◦	The DTB parser must handle standard bindings per the Device Tree Specification
	◦	Many real AArch64 boards (Raspberry Pi, most ARM SBCs) use Device Tree rather than ACPI. The kernel must support both ACPI (for server-class ARM hardware) and DTB (for SBCs and embedded).
	6	Implement ACPI parsing for AArch64 (for server-class ARM hardware that uses ACPI instead of DTB):
	◦	ARM ACPI tables include IORT (Interrupt Remapping Table), GTDT (Generic Timer Description Table), SPCR (Serial Port Console Redirection), DBG2 (Debug Port Table), FADT (with ARM-specific fields), MADT (GIC entries)
	◦	Some AArch64 platforms (e.g., Ampere, AWS Graviton, real ARM servers) use ACPI
	7	Test boot on QEMU AArch64 with: qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M -bios QEMU_EFI.fd -cdrom build/ApertureOS-aarch64.iso -serial stdio.
	8	Test boot on real AArch64 hardware: Boot on at least one real AArch64 machine — ideally a Raspberry Pi 5 (using UEFI firmware shim if needed), or another ARM64 SBC, or an ARM64 laptop. Verify the kernel boots, parses DTB/ACPI, initializes the framebuffer, and reaches the idle loop. Document which hardware was tested and any issues.
1C: Real Hardware Robustness Layer
Goal: The kernel does not crash or hang on real hardware, even with unexpected or missing devices.
	1	Implement graceful device absence handling: If a device is not found (no AHCI controller, no USB controller, no network card), the kernel logs a warning and continues boot. The system should still boot to a desktop even if there is no storage device (using in-memory filesystem), no network, or no audio.
	2	Implement device timeout handling: Every device driver operation must have a timeout. If a device does not respond within a reasonable time, the driver logs an error and continues. No infinite loops waiting on hardware.
	3	Implement firmware quirk handling: UEFI and ACPI firmware on real hardware is buggy. The kernel must handle:
	◦	UEFI memory map entries with unexpected types
	◦	ACPI tables with vendor-specific or non-standard formats
	◦	UEFI ExitBootServices requiring retry (timer map key changed)
	◦	UEFI runtime services that fail (fall back to software clock if UEFI time doesn't work)
	◦	BIOS/UEFI that leaves devices in non-default states
	◦	ACPI _OSC method negotiation for PCIe access (some BIOSes require _OSC to be called before PCIe config space is accessible)
	4	Implement memory map reconciliation: The kernel must reconcile the UEFI/Multiboot2/DTB memory map with the physical memory available, marking regions as usable, reserved, ACPI reclaimable, ACPI NVS, MMIO, etc. Handle holes in physical memory (common on real hardware). Don't assume contiguous physical memory.
	5	Implement a kernel log / dmesg equivalent: All boot messages, device discovery, warnings, and errors are logged. This is critical for debugging on real hardware where serial output may not be available. The log should be viewable from the desktop environment (a "dmesg" equivalent command or GUI app).
PHASE 2: Kernel Stability and Core Infrastructure (Both Architectures, Real Hardware, Native Execution)
Goal: The kernel is robust on both x86_64 and AArch64, handles interrupts correctly on real hardware, manages memory safely, and provides a solid base for userland — including the Windows NT process model. All kernel code runs natively.
	1	Implement preemptive multitasking on both architectures: Replace the cooperative scheduler with a preemptive round-robin scheduler. On x86_64, use the LAPIC timer (preferred for real hardware — the PIT is legacy) or HPET. On AArch64, use the architectural timer (CNTV_CTL_EL0). Implement proper context saving/restoring on timer interrupts — the register set and calling convention differ between architectures, so each HAL implements its own Context struct and context_switch function. Each process gets a time quantum.
	◦	Real hardware consideration: LAPIC timer calibration differs between machines. The kernel must calibrate the LAPIC timer frequency (using the TSC or PIT as a reference). HPET discovery is via ACPI. On some hardware, the HPET is missing or at a non-standard address.
	2	Implement proper process isolation on both architectures: Each process has its own page tables. On x86_64, CR3 holds the PML4 root. On AArch64, TTBR0_EL1 holds the user-space page table root and TTBR1_EL1 holds the kernel-space root. User processes run in EL0 (AArch64) / ring 3 (x86_64). The kernel cannot be written to from user space on either architecture.
	◦	Real hardware consideration: TLB flushing semantics differ between architectures. x86_64 uses INVLPG / MOV CR3. AArch64 uses TLBI instructions. On SMP systems (later phase), TLB shootdown IPIs are needed. Some CPUs have PCID (x86_64) or ASID (AArch64) for avoiding full TLB flushes on context switch — use these for performance.
	3	Implement a robust interrupt system on both architectures: All exceptions and IRQs are handled.
	◦	x86_64 real hardware: Use the APIC (LAPIC + IOAPIC), not the legacy PIC. Parse the ACPI MADT for LAPIC base, IOAPIC base, and interrupt source overrides. Handle MSI/MSI-X interrupts for PCIe devices. Legacy PIC (8259) should be disabled (masked) — do not use it on real hardware. The legacy PIC is only needed for very old BIOS systems.
	◦	AArch64 real hardware: Use GIC v2 or v3 (detected from ACPI/DTB). GICv3 uses redistributor bases and distributor base. GICv2 uses a single distributor + CPU interface. Handle GICv3's affinity routing (SPIs go to specific CPU targets, PPIs are per-CPU, SGIs are inter-processor).
	◦	Page faults (x86_64) / data & instruction aborts (AArch64), general protection faults / synchronous exceptions, and invalid opcodes kill the offending process cleanly (not the whole system). Timer interrupts drive the scheduler on both architectures.
	4	Implement a basic IPC mechanism: Pipes or message ports between processes — needed for both native ApertureOS programs and the Win32 subsystem (which uses LPC/ALPC ports internally). This must work identically on both architectures.
	5	Implement the Windows NT process and thread model natively (architecture-independent core, arch-specific context):
	◦	EPROCESS / KPROCESS / ETHREAD / KTHREAD equivalents in Rust (architecture-independent in the shared kernel code, with architecture-specific register context stored in the arch HAL)
	◦	CreateProcessW semantics: load PE, create process object, create initial thread, set up PEB and TEB in the process's address space
	◦	CreateThread semantics: create additional threads in an existing process
	◦	Thread-local storage (TLS) support as used by Windows programs
	◦	Process/thread handle management via the object manager
	◦	The CONTEXT struct (register state for GetThreadContext/SetThreadContext) must be architecture-specific — CONTEXT_X86_64 and CONTEXT_ARM64 — matching Windows definitions.
	6	Implement a real heap allocator (replace bump allocator with a proper allocator — e.g., slab or buddy) on both architectures. Implement NtAllocateVirtualMemory, NtFreeVirtualMemory, and NtProtectVirtualMemory as the primary memory management syscalls used by Windows programs. The page table management code is architecture-specific, but the VFS-level memory management API is shared.
	◦	Real hardware consideration: Real hardware has memory holes (e.g., 0xA0000-0xFFFFF on x86, MMIO regions, reserved BIOS/UEFI areas). The allocator must only use memory marked as usable in the memory map. Handle >4GB memory on x86_64 (PAE is implicit in long mode). Handle large physical memory (use 2MB/1GB pages on x86_64, 2MB block mappings on AArch64 for performance).
	7	Implement the Windows-compatible system call interface on both architectures: On x86_64, the SYSCALL/SYSRET instruction pair dispatches to the NT syscall handler. On AArch64, the SVC instruction dispatches to the same NT syscall handler. The syscall numbers and semantics must match the Windows NT kernel ABI. For native processes (OS components and PE binaries matching the host arch), the syscall is a direct hardware instruction (SYSCALL on x86_64, SVC on AArch64). For translated processes (external PE binaries not matching host arch), the translator translates the guest's syscall instruction into the host's syscall instruction with the same syscall number.
PHASE 3: PE Loading, DLL Resolution, and Windows Executable Execution (Native + System DLLs)
Goal: Real Windows .exe and .dll files can be loaded, their imports resolved, and their code executed in user space. The OS's own system DLLs run natively. External Windows .exe files run natively if arch matches, or via translation if it doesn't (Phase 4).
	1	Upgrade the PE loader (kernel/win32/loader.rs) to handle real-world PE files (this code is architecture-independent — it parses PE headers and maps sections, which works the same regardless of host arch):
	◦	Parse PE32 and PE32+ (64-bit) headers correctly
	◦	Map sections to their specified virtual addresses with correct permissions (R, RX, RW, RWX as specified in section headers)
	◦	Process the import table: for each imported DLL, load that DLL (recursively), and resolve each imported function to its actual address
	◦	Apply base relocations if the PE is not loaded at its preferred image base
	◦	Handle TLS callbacks
	◦	Set up the PEB and TEB with correct pointers (LoaderData, ProcessParameters, TLS arrays, etc.)
	◦	Call the DLL entry point (DllMain) for each loaded DLL with DLL_PROCESS_ATTACH
	◦	Call the executable entry point
	◦	Architecture check: The loader checks the PE machine type. If it matches the host architecture (IMAGE_FILE_MACHINE_AMD64 on x86_64, IMAGE_FILE_MACHINE_ARM64 on AArch64), the code is executed natively. If it does not match, the binary translator (Phase 4) is invoked. If the translator is not yet implemented, log an error and refuse to load (until Phase 4 is complete).
	2	Implement the OS's own system DLLs — compiled and running natively per architecture:
	◦	These are NOT external Windows DLLs. They are ApertureOS's own implementations of the Win32 API, written in Rust, and they run as native code on whichever architecture the OS is booted on. They are built-in to the kernel as resident shims (the PE loader recognizes built-in DLL names and resolves their exports without needing external .dll files on disk).
	◦	ntdll.dll — the lowest-level user-mode DLL, exports the Nt* syscall stubs. On x86_64, the built-in shim contains syscall instructions. On AArch64, the built-in shim contains SVC #0 instructions. This is native code — the syscall instruction matches the host architecture.
	◦	kernel32.dll — exports CreateFileW, ReadFile, WriteFile, CloseHandle, CreateProcessW, CreateThread, GetModuleHandleW, LoadLibraryW, FreeLibrary, GetProcAddress, ExitProcess, GetLastError, Sleep, GetTickCount, GetSystemTime, MultiByteToWideChar, WideCharToMultiByte, GetConsoleMode, SetConsoleMode, AllocConsole, GetStdHandle, SetConsoleCursorPosition, WriteConsoleW, ReadConsoleW, and more
	◦	user32.dll — exports window management functions: CreateWindowExW, ShowWindow, UpdateWindow, GetMessage, DispatchMessageW, TranslateMessage, DefWindowProcW, RegisterClassW, PostQuitMessage, SendMessageW, PostMessageW, GetCursorPos, SetCursorPos, GetAsyncKeyState, MessageBoxW, BeginPaint, EndPaint, GetDC, ReleaseDC, TextOutW, FillRect, MoveToEx, LineTo, SelectObject, DeleteObject, CreateSolidBrush, CreatePen, SetPixelV, Rectangle, Ellipse, GetClientRect, GetWindowRect, SetWindowTextW, GetWindowTextW, EnableWindow, IsWindowEnabled, DestroyWindow, InvalidateRect, etc.
	◦	gdi32.dll — exports graphics device interface functions: CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, DeleteDC, DeleteObject, BitBlt, StretchBlt, TextOutW, ExtTextOutW, MoveToEx, LineTo, Polyline, Polygon, Rectangle, Ellipse, RoundRect, FillRect, FrameRect, SetPixelV, GetPixel, CreateSolidBrush, CreatePen, CreateFontW, CreateBitmap, CreateDIBSection, GetDIBits, SetDIBits, RGB, GetDeviceCaps, SaveDC, RestoreDC, etc.
	◦	These DLLs are implemented in Rust as part of the Win32 subsystem and compiled natively for each target architecture. On x86_64, they are x86_64 code. On AArch64, they are AArch64 code. They are never translated.
	◦	When an external PE binary (e.g., an x86_64 Windows .exe running on AArch64 via translation) imports kernel32.dll, the PE loader resolves the imports against the built-in native kernel32.dll shim. The external binary's import thunks point to the native DLL's export functions. When the translated code calls an imported function, the translator jumps to the native DLL function, which runs natively. The return value is translated back to the guest register state. This is how FEX-Emu and Rosetta 2 handle library calls — the guest code is translated, but the host's own libraries run natively.
	3	Implement the NT object manager and handle table (kernel/win32/objects.rs): Every kernel resource (process, thread, file, event, mutex, semaphore, section, key, etc.) is an object with a handle. Implement NtCreate*, NtOpen*, NtClose, NtQuery*, NtSet* for all object types. The object manager is architecture-independent.
	4	Test with real Windows console applications on x86_64 (native): Build or obtain simple Windows console programs (e.g., a "Hello World" compiled with MSVC or MinGW for x86_64) and verify they execute under ApertureOS x86_64 (in QEMU first, then on real hardware). Test that WriteConsoleW output appears on the framebuffer console. These run natively because the PE machine type matches the host.
	5	Test with simple Windows GUI applications on x86_64 (native): Build or obtain a simple Win32 GUI app (e.g., a program that calls MessageBoxW or creates a window with CreateWindowExW) and verify it creates a window in the ApertureOS desktop environment (in QEMU first, then on real hardware). These run natively.
	6	Test the same on AArch64 (native for ARM64 PE, translated for x86_64 PE): Once Phase 4 is complete, test ARM64 PE natively on AArch64 and x86_64 PE via translation on AArch64. For now (before Phase 4), test with ARM64 PE binaries running natively on AArch64 if available.
PHASE 4: Cross-Architecture Binary Translation (FEX-Style) — For External/Third-Party Programs Only
Goal: Externally downloaded Windows PE binaries compiled for any architecture can run on any ApertureOS host architecture. This is a compatibility feature for third-party software, not a mechanism for running OS components. The OS itself always runs natively.
This phase is REQUIRED — not optional. It is what makes ApertureOS able to run the entire Windows software ecosystem regardless of host architecture.
4A: Architecture Overview and Design
The translation system works as follows:
	•	The OS's own components (kernel, drivers, system DLLs, built-in apps) always run natively. They are compiled for the host architecture and execute directly on the CPU. No translation is ever applied to OS components.
	•	When a user launches an externally downloaded Windows .exe file (from the internet, a USB drive, or any external source), the PE loader checks the binary's machine type:
	◦	If the machine type matches the host architecture → native execution (e.g., x86_64 PE on x86_64 host, ARM64 PE on AArch64 host). No translation needed. Full speed.
	◦	If the machine type does NOT match the host architecture → translation is invoked. The translator reads the guest architecture's instructions, translates them to host architecture instructions (either ahead-of-time into a cache, or just-in-time per basic block), and executes the translated code in the same process address space.
	•	Syscalls in the guest code (e.g., x86_64 syscall instruction) are translated to host syscalls (e.g., AArch64 SVC #0) with the same syscall number — the kernel's NT syscall handler is architecture-independent at the semantic level.
	•	When the translated guest code calls an imported function from a system DLL (e.g., kernel32!CreateFileW), the call is routed to the native system DLL function. The system DLL runs natively. The return value is translated back to the guest register state. This is the same approach as FEX-Emu (which calls host-native Linux libraries) and Rosetta 2 (which calls macOS-native frameworks).
	•	The process appears identical from the kernel's perspective regardless of whether the guest code is running natively or via translation. The PEB, TEB, handles, memory, and syscalls all work the same way.
4B: x86_64 → AArch64 Translator (Primary Cross-Arch Target)
This is the most important translator: it enables running the vast majority of Windows applications (which are x86/x86_64) on ARM64 hardware.
	1	Implement a baseline JIT translator that converts x86_64 instructions to AArch64 instructions:
	◦	Register mapping: Map x86_64 general-purpose registers (RAX, RBX, RCX, RDX, RSI, RDI, RBP, RSP, R8-R15) to AArch64 registers (X0-X30). Maintain a register state struct that tracks the guest register file.
	◦	Instruction decoding: Parse x86_64 instruction stream (REX prefixes, ModR/M, SIB, displacement, immediate). Implement decoders for the most common instruction classes first: MOV, ADD, SUB, CMP, PUSH, POP, LEA, CALL, RET, JMP, Jcc, TEST, AND, OR, XOR, NOT, NEG, INC, DEC, MUL, IMUL, DIV, IDIV, SHL, SHR, SAR, ROL, ROR, MOVZX, MOVSX, NOP, SYSCALL, INT3, UD2.
	◦	Instruction translation: Emit AArch64 equivalent instructions. Some x86_64 instructions map directly (e.g., MOV RAX, RCX → MOV X0, X1). Others require multiple AArch64 instructions (e.g., x86_64's complex addressing modes, flag-setting instructions, or instructions with implicit operands).
	◦	Flags register: x86_64's RFLAGS (CF, ZF, SF, OF, PF, AF, DF) must be tracked. Some AArch64 instructions set flags (NZCV), but x86_64 flags don't map 1:1. Maintain a software flags register or use AArch64 condition codes creatively.
	◦	Memory access: x86_64's segmented memory model (FS/GS for TLS) must be handled. FS_BASE/GS_BASE MSR equivalents map to AArch64's TPIDR_EL0 / TPIDRRO_EL0 system registers. Segment-relative accesses are translated to absolute addresses using the TLS base.
	◦	Function calls: x86_64 CALL pushes the return address and jumps; RET pops and jumps. AArch64 BL/RETwork differently (LR register). The translator must handle the x86_64 stack-based return convention using the guest RSP.
	◦	Floating point / SSE: x86_64 XMM registers (SSE/AVX) map to AArch64 SIMD registers (V0-V31, NEON). Implement basic SSE instruction translation (MOVSS, ADDSS, SUBSS, MULSS, DIVSS, CVTSS2SI, CVTSI2SS, etc.). Full AVX is a later stretch goal.
	◦	Syscall translation: x86_64 SYSCALL (with RAX = syscall number, RCX = return address, R11 = flags) translates to AArch64 SVC #0 (with X8 = syscall number). The translator emits a special host call sequence that invokes the kernel's syscall handler with the translated register mapping.
	◦	Imported function calls to native DLLs: When the translated guest code calls an imported function (e.g., a function from kernel32.dll), the import thunk points to the native AArch64 system DLL function. The translator detects calls to import thunks and emits a direct call to the native function. The native function runs natively (as AArch64 code). On return, the translator translates the native return value back to the guest register state. This is critical for performance — system DLL calls are not translated; they run at full native speed.
	◦	Code caching: Translated blocks are cached so they don't need to be retranslated on every execution. Use a hash table keyed by guest instruction address.
	◦	Self-modifying code: Implement a basic write-protect mechanism for translated code pages. If the guest writes to a code page, invalidate the translation cache for that page.
	2	Implement an interpreter fallback for instructions that the JIT cannot yet handle (rare or complex x86_64 instructions). The interpreter runs the instruction's semantics in software and updates the guest register state. This ensures that even if the JIT is incomplete, programs still run (slowly).
	3	Test x86_64 PE on AArch64 host: Boot ApertureOS on QEMU AArch64 (qemu-system-aarch64 -machine virt -cpu cortex-a72), load an x86_64 Windows console "Hello World" .exe (externally downloaded), and verify it executes and produces output. This is the landmark demonstration of cross-architecture compatibility. The OS itself (kernel, DLLs, desktop) runs natively as AArch64. Only the external .exe is translated.
	4	Test on real AArch64 hardware: Boot ApertureOS on a real AArch64 machine (e.g., Raspberry Pi 5), load the same x86_64 Windows "Hello World" .exe via the translator, and verify it produces output.
4C: x86 (32-bit) → x86_64 and x86 (32-bit) → AArch64 Translators (WoW64)
	1	Implement WoW64 (Windows-on-Windows 64) for running 32-bit x86 PE binaries on a 64-bit host:
	◦	On x86_64 host: 32-bit external processes get a 32-bit address space (4GB max, low 2GB user). The kernel's NT syscall handler recognizes 32-bit processes and translates 32-bit pointer-width structures to 64-bit kernel structures. The WOW64 layer in user mode translates 32-bit Win32 API calls to 64-bit ntdll calls. The system DLLs run natively (as 64-bit code), but the 32-bit process's own code is executed via the WoW64 thunking layer.
	◦	On AArch64 host: 32-bit x86 instructions are translated to AArch64 by the JIT translator (same as 4B, but with 32-bit register width — EAX instead of RAX, etc.). The address space is 32-bit. The WoW64 user-mode translation layer handles struct width conversion. The OS's native AArch64 system DLLs are called directly (same as 4B).
	2	Test x86 PE on x86_64 host: Run a 32-bit Windows console app (externally downloaded) on ApertureOS x86_64 (QEMU then real hardware).
	3	Test x86 PE on AArch64 host: Run a 32-bit Windows console app (externally downloaded) on ApertureOS AArch64 (nested translation: x86 → AArch64) (QEMU then real hardware).
4D: ARM64 → x86_64 Translator (Reverse Cross-Arch)
	1	Implement an ARM64 → x86_64 JIT translator that converts AArch64 instructions to x86_64 instructions (for running externally downloaded ARM64 Windows .exe files on an x86_64 host):
	◦	Register mapping: Map AArch64 X0-X30, SP, PC to x86_64 registers.
	◦	Instruction decoding: Parse AArch64 instruction encoding (fixed-width 32-bit instructions). Implement decoders for: ADRP, ADD, SUB, MOV, LDR, STR, LDP, STP, B, BL, BR, RET, B.cond, CBZ, CBNZ, TBZ, TBNZ, CMP, AND, ORR, EOR, MVN, LSL, LSR, ASR, ROR, MUL, UMULH, SDIV, UDIV, SVC, MRS, MSR, BRK.
	◦	Condition flags: AArch64 NZCV flags map reasonably to x86_64 FLAGS (with some differences).
	◦	NEON/SIMD: AArch64 SIMD registers map to x86_64 XMM registers.
	◦	Syscall translation: AArch64 SVC #0 (with X8 = syscall number) translates to x86_64 SYSCALL (with RAX = syscall number).
	◦	Imported function calls to native DLLs: Same mechanism as 4B — calls to native system DLL functions run natively (as x86_64 code on x86_64 host).
	2	Test ARM64 PE on x86_64 host: Load an ARM64 Windows .exe (externally downloaded) on ApertureOS x86_64 and verify execution (QEMU then real hardware).
4E: Translation Infrastructure and Management
	1	Implement a translation manager that:
	◦	Detects the PE machine type (IMAGE_FILE_MACHINE_AMD64, IMAGE_FILE_MACHINE_I386, IMAGE_FILE_MACHINE_ARM64, IMAGE_FILE_MACHINE_ARM) and selects native execution or the appropriate translator
	◦	Native path: If PE machine type matches host → native execution. No translator involved. This is the fast path and applies to all OS components and matching external PE files.
	◦	Translation path: If PE machine type does not match host → translator is invoked. This only applies to externally downloaded PE files.
	◦	Manages translation caches (per-process, invalidated on process exit)
	◦	Handles translation faults (if the translator encounters an unsupported instruction, it raises a diagnostic exception with the guest instruction address and bytes)
	◦	Provides a /proc-like or NtQueryInformation interface for monitoring translation statistics (cache hit rate, blocks translated, interpreter fallback count)
	2	Implement a profile-guided optimization (PGO) path (stretch goal): Hot code paths are identified and optimized with more aggressive translation. This is how FEX-Emu achieves near-native performance.
	3	Implement AOT translation (stretch goal): Pre-translate PE binaries ahead of time and cache the translated code on disk. This is FEX-Emu's approach — pre-warmed translation caches.
PHASE 5: Storage, Filesystems, and Persistence (Real Hardware Drivers)
Goal: The OS can read from and write to real disk media on both architectures — using real hardware drivers, not just virtio-blk. Expose a Windows-compatible file system namespace (drive letters, UNC paths). All drivers run natively.
	1	Implement storage drivers for both architectures — supporting real hardware, running natively:
	◦	x86_64 (native): AHCI/SATA driver (the most common real-world storage controller — parse PCI vendor/device IDs, initialize HBA registers, command list, FIS receive, port enumeration, NCQ support), NVMe driver (PCIe NVMe SSDs — admin submission/completion queues, I/O submission/completion queues, PRP/SGL), IDE/PATA driver (legacy), virtio-blk driver (for QEMU)
	◦	AArch64 (native): virtio-blk driver (for QEMU virt), SD/MMC driver (for Raspberry Pi and ARM SBCs — SDHCI/SDXC controller, command/response sequences, DMA transfer), NVMe driver (for Apple Silicon and modern ARM platforms with NVMe SSDs), UFS driver (for some ARM platforms), eMMC driver
	◦	Real hardware consideration: Real AHCI controllers need proper PCI BAR mapping, memory-mapped register access, port power management, and may need BIOS/UEFI handoff (the AHCI controller may be in IDE emulation mode — switch to AHCI mode). NVMe controllers vary significantly between vendors. SD/MMC controllers on Raspberry Pi have specific quirks. Every driver must handle device timeouts, link power management, hotplug (for USB), and error recovery.
	2	Implement a partition table parser (GPT and MBR) — architecture-independent. Handle real-world GPT tables with protective MBR, GPT header, partition entries, and backup GPT. Handle hybrid MBR/GPT (common on real hardware). Handle corrupted partition tables gracefully.
	3	Implement filesystem drivers: Start with FAT32 (simplest, well-documented, broadly compatible, works on both architectures, readable by UEFI firmware). Then implement NTFS read support (needed for reading Windows-formatted partitions — at minimum read-only NTFS, later read-write). Then optionally exFAT (common on USB drives and SD cards, important for real hardware). The system should be able to mount partitions from real disks and read/write files.
	◦	Real hardware consideration: Real FAT32 filesystems may have non-standard cluster sizes, long filename entries with non-ASCII characters, and may be slightly corrupted. The driver must be robust. NTFS is complex ($MFT, $LogFile, attribute lists, compression, sparse files) — start with read-only basic file/directory access.
	4	Extend the VFS layer to support multiple mount points, directory traversal, file metadata (size, timestamps, permissions), and basic file operations (open, read, write, close, seek, stat, mkdir, readdir, unlink). This is architecture-independent.
	5	Implement Windows-compatible path semantics: The VFS must expose drive-lettered paths (e.g., C:\Windows\System32\, C:\Users\, D:\) and UNC paths (\\Server\Share\). The Win32 file API (CreateFileW, NtCreateFile, NtOpenFile) must translate these to internal VFS paths. Implement \??\C: and \Device\ NT namespace prefixes.
	6	Add a root filesystem: The boot disk image should contain a FAT32 root partition with system files, DLLs, executables, and configuration. The kernel auto-mounts this as C:\ at boot. Provide a \Windows\System32\ directory structure. Both x86_64 and AArch64 ISOs include this filesystem. The FAT32 partition must be readable by UEFI firmware (for UEFI boot) and by the kernel.
PHASE 6: Graphical Desktop Environment with Win32 Window Management (Both Architectures, Real Hardware, Native)
Goal: The user boots to a graphical desktop with windows, a mouse cursor, and the ability to interact with applications — including Windows GUI apps — on BOTH x86_64 and AArch64 hardware, including real hardware displays. The desktop environment, window manager, and compositor all run natively.
	1	Implement a real framebuffer compositor (native on both architectures): Support multiple windows, z-ordering, overlapping, and window regions. The compositor should handle dirty rectangles and double-buffering to avoid tearing. The compositor is architecture-independent (it works on any framebuffer) and runs natively on whichever arch the OS is booted on.
	◦	Real hardware consideration: Real GPUs have varying framebuffer layouts, pitch, pixel formats (RGB565, XRGB8888, ARGB8888, etc.). The kernel must query the framebuffer info from the bootloader (UEFI GOP / Multiboot2 / DTB) and adapt. Some real hardware requires specific resolution or has limitations on resolution. Some hardware supports hardware cursors (via GPU) — use those when available, otherwise use software cursor.
	2	Implement a mouse driver on both architectures — supporting real hardware, running natively:
	◦	x86_64 (native): PS/2 mouse (i8042 controller — the most universally available input on real x86_64 hardware), USB HID via xHCI (modern USB mice), PS/2 scroll mouse extensions
	◦	AArch64 (native): USB HID via xHCI (for QEMU virt, Raspberry Pi 4/5 USB ports), PL050 (some ARM platforms)
	◦	Real hardware consideration: PS/2 mouse initialization on real hardware requires proper controller reset, device detection, and handling of auxiliary device not present. USB HID mouse requires full xHCI enumeration, HID report descriptor parsing, and interrupt transfer handling. The mouse driver must handle device disconnect/reconnect.
	3	Implement a keyboard driver on both architectures — supporting real hardware, running natively:
	◦	x86_64 (native): PS/2 keyboard (i8042 controller), USB HID keyboard via xHCI
	◦	AArch64 (native): USB HID keyboard via xHCI
	◦	Real hardware consideration: PS/2 keyboard scancode sets (set 1 is most common on real hardware, set 2 on some). USB HID keyboard needs HID report parsing. Handle both scancode sets. Handle key repeat, modifiers (Shift, Ctrl, Alt, Win), and special keys.
	4	Implement the Win32 windowing model natively (architecture-independent — runs natively on both archs):
	◦	Window class registration (RegisterClassW / RegisterClassExW): Store window procedure callbacks, class styles, icon, cursor, background brush.
	◦	Window creation (CreateWindowExW): Create a top-level or child window with position, size, style, extended style, parent, menu, instance handle, and creation data. Assign an HWND.
	◦	Message queue per thread: Each GUI thread has a message queue. Implement GetMessage, PeekMessage, DispatchMessage, TranslateMessage, PostMessage, SendMessage, PostQuitMessage.
	◦	Window messages: WM_CREATE, WM_DESTROY, WM_PAINT, WM_SIZE, WM_MOVE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_MOUSEMOVE, WM_KEYDOWN, WM_KEYUP, WM_CHAR, WM_CLOSE, WM_QUIT, WM_TIMER, WM_COMMAND, WM_NOTIFY, WM_SETFOCUS, WM_KILLFOCUS, WM_ACTIVATE, WM_SHOWWINDOW, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_NCCREATE, WM_NCDESTROY, WM_NCPAINT, WM_NCCALCSIZE, WM_NCHITTEST, etc.
	◦	Default window procedure (DefWindowProcW): Handle standard messages.
	◦	Show/hide/move/resize (ShowWindow, MoveWindow, SetWindowPos, GetWindowRect, GetClientRect, InvalidateRect, UpdateWindow, RedrawWindow).
	◦	Hit testing and focus: Mouse clicks determine which window receives input. Keyboard input goes to the focused window.
	◦	Important for cross-arch with external translated programs: When a translated external process (e.g., an x86_64 Windows .exe running on AArch64 via translation) creates a window, the window procedure callback address is in the guest address space. The translator must correctly handle window procedure callbacks — when DispatchMessage calls the window procedure, the translator must jump to the translated guest code at that address. The OS's own window manager and compositor run natively. Only the external program's window procedure code is translated.
	5	Implement GDI (Graphics Device Interface) (architecture-independent — runs natively on both archs):
	◦	Device contexts (DC): GetDC, ReleaseDC, BeginPaint/EndPaint, CreateCompatibleDC, DeleteDC.
	◦	Drawing primitives: TextOutW, ExtTextOutW, MoveToEx, LineTo, Polyline, Polygon, Rectangle, Ellipse, RoundRect, Arc, Pie, Chord, SetPixelV, GetPixel, FillRect, FrameRect, DrawText, DrawTextEx.
	◦	GDI objects: CreateSolidBrush, CreateHatchBrush, CreatePatternBrush, CreatePen, CreateFontW, CreateBitmap, CreateDIBSection, SelectObject, DeleteObject, GetObjectW.
	◦	Bitmap operations: BitBlt, StretchBlt, PatBlt, TransparentBlt, AlphaBlend, MaskBlt, PlgBlt.
	◦	Region and clipping: CreateRectRgn, CreateRectRgnIndirect, CombineRgn, SelectClipRgn, SelectClipPath, ExtSelectClipRgn, GetClipBox.
	◦	Text and fonts: Font enumeration, font rendering, text metrics, SetTextAlign, SetTextColor, SetBkColor, SetBkMode.
	◦	Coordinate mapping: SetMapMode, DPtoLP, LPtoDP, SetWindowExtEx, SetViewportExtEx, SetViewportOrgEx, OffsetViewportOrgEx.
	6	Implement a window manager for the desktop with:
	◦	Window creation, movement, resizing, and closing (driven by the Win32 windowing model above)
	◦	Title bars with close/minimize/maximize buttons (rendered by the window manager as non-client area)
	◦	Focus management (click-to-focus, matching Win32 SetFocus semantics)
	◦	A taskbar/dock showing open windows (equivalent to the Windows taskbar)
	◦	Window menu (system menu / Alt+Space menu)
	7	Implement a desktop environment (runs natively on both architectures):
	◦	A desktop background (solid color or gradient to start; bitmap image later)
	◦	A taskbar with a Start-menu equivalent, a clock (reading from RTC — architecture-independent syscall), and system tray
	◦	A right-click desktop context menu
	◦	A "Start" menu or launcher that lists available programs on the disk (including externally downloaded Windows .exe files) and can launch them via CreateProcessW. The launcher auto-detects the PE machine type and indicates whether native execution or translation will be used. Both native OS apps and translated external Windows apps appear in the launcher.
	8	Implement a graphical file manager (native OS app): A window-based file browser that reads from the VFS using Win32 file APIs, displays icons/list for files and folders, and supports double-click to open. Double-clicking an .exe should launch it via CreateProcessW — the system auto-detects the PE architecture and uses native execution (if arch matches) or translation (if arch doesn't match). The file manager itself is a native OS application running natively.
	9	Implement a graphical console/terminal (native OS app): A window that emulates a Windows console. This should support AllocConsole, GetStdHandle, SetConsoleMode, GetConsoleMode, WriteConsoleW/WriteConsoleOutputW, ReadConsoleW, SetConsoleCursorPosition, GetConsoleScreenBufferInfo, FillConsoleOutputCharacterW, SetConsoleTextAttribute, console window resize, and scrollback. This enables Windows console applications (both native and translated) to run and display output in a GUI window. The terminal itself is a native OS application.
	10	Implement a text editor (native OS app): A simple GUI text editor (like Notepad) that can open, edit, and save files. This is a native ApertureOS application.
	11	Test the full desktop on real hardware: Boot ApertureOS on a real x86_64 machine (with a real display and real USB mouse/keyboard) and verify the desktop appears, the mouse moves, keyboard input works, and windows can be created/moved/closed. Do the same on a real AArch64 machine. All OS components (desktop, file manager, terminal, editor) run natively. If you also launch an external x86_64 Windows .exe on the AArch64 machine, it runs via translation while the OS remains native.
PHASE 7: Windows API Coverage Expansion
Goal: The Win32 subsystem (running natively as part of the OS) has broad enough API coverage that real-world Windows applications run correctly on both architectures — natively when arch matches, translated when it doesn't.
	1	Implement the full NT syscall surface (kernel/win32/nt.rs) — architecture-independent syscall semantics, architecture-specific entry points. All syscall handlers run natively as part of the kernel:
	◦	Process/thread: NtCreateProcess, NtCreateThread, NtTerminateProcess, NtTerminateThread, NtQueryInformationProcess, NtSetInformationProcess, NtQueryInformationThread, NtSetInformationThread, NtSuspendThread, NtResumeThread, NtGetContextThread, NtSetContextThread, NtCreateProcessEx
	◦	Memory: NtAllocateVirtualMemory, NtFreeVirtualMemory, NtProtectVirtualMemory, NtQueryVirtualMemory, NtFlushInstructionCache, NtMapViewOfSection, NtUnmapViewOfSection, NtCreateSection, NtOpenSection
	◦	Files/IO: NtCreateFile, NtOpenFile, NtReadFile, NtWriteFile, NtQueryInformationFile, NtSetInformationFile, NtDeleteFile, NtQueryDirectoryFile, NtFsControlFile, NtDeviceIoControlFile, NtCancelIoFile, NtFlushBuffersFile
	◦	Synchronization: NtCreateEvent, NtOpenEvent, NtSetEvent, NtResetEvent, NtClearEvent, NtPulseEvent, NtCreateMutant, NtOpenMutant, NtReleaseMutant, NtCreateSemaphore, NtOpenSemaphore, NtReleaseSemaphore, NtCreateTimer, NtSetTimer, NtCancelTimer, NtWaitForSingleObject, NtWaitForMultipleObjects, NtSignalAndWaitForSingleObject, NtDelayExecution, NtYieldExecution
	◦	Objects/handles: NtClose, NtDuplicateObject, NtQueryObject, NtQuerySystemInformation, NtSetSystemInformation
	◦	Registry: NtCreateKey, NtOpenKey, NtSetValueKey, NtQueryValueKey, NtEnumerateKey, NtEnumerateValueKey, NtDeleteKey, NtDeleteValueKey, NtFlushKey, NtNotifyChangeKey, NtLoadKey, NtUnloadKey
	◦	IPC: NtCreatePort, NtConnectToPort, NtSendWaitReplyPort, NtAcceptConnectPort, NtCompleteConnectPort, NtCreateNamedPipeFile, NtCreateMailslotFile
	◦	Exception handling: NtRaiseException, NtRaiseHardError, NtContinue, NtSetInformationProcess (for exception port)
	◦	Info/query: NtQuerySystemTime, NtSetSystemTime, NtQueryPerformanceCounter, NtQuerySystemInformation (various info classes), NtQueryInformationProcess
	◦	Security: NtCreateToken, NtOpenProcessToken, NtQueryInformationToken, NtSetInformationToken, NtAdjustPrivilegesToken, NtAccessCheck, NtCreateSecurityObject, NtSetSecurityObject, NtQuerySecurityObject
	◦	Environment: NtSetInformationProcess (PEB updates), RtlCreateEnvironment, RtlDestroyEnvironment
	◦	Cross-arch note: The CONTEXT struct passed to NtGetContextThread/NtSetContextThread is architecture-specific. For translated external processes, the context reflects the guest architecture (e.g., an x86_64 PE running on AArch64 returns an x86_64 CONTEXT), not the host. This is critical for debuggers and exception handling in translated processes. For native processes (OS components and arch-matching PE files), the context matches the host architecture.
	2	Implement the Windows Registry (real implementation, not shim) — architecture-independent, runs natively:
	◦	Hive-based storage (or simpler: a flat file or in-memory tree backed by VFS)
	◦	Root keys: HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER, HKEY_CLASSES_ROOT, HKEY_USERS, HKEY_CURRENT_CONFIG
	◦	Standard paths: HKLM\SOFTWARE\, HKLM\SYSTEM\CurrentControlSet\, HKCU\Software\, HKCR\.exe\, HKCR\exefile\
	◦	Value types: REG_SZ, REG_EXPAND_SZ, REG_DWORD, REG_BINARY, REG_MULTI_SZ, REG_QWORD
	◦	Win32 API wrappers: RegCreateKeyExW, RegOpenKeyExW, RegSetValueExW, RegQueryValueExW, RegDeleteKeyW, RegDeleteValueW, RegEnumKeyExW, RegEnumValueW, RegCloseKey, RegFlushKey
	◦	The registry must persist to disk (in the FAT32 root filesystem) so changes survive reboot.
	3	Implement Common Controls (comctl32.dll subset) — architecture-independent, runs natively as part of the OS:
	◦	CreateWindowExW with class names like BUTTON, EDIT, STATIC, LISTBOX, COMBOBOX, SCROLLBAR, PROGRESS_CLASS, TRACKBAR_CLASS, UPDOWN_CLASS, WC_TABCONTROL, WC_TREEVIEW, WC_LISTVIEW, STATUSCLASSNAME, TOOLTIPS_CLASS
	◦	Each control processes its own messages and paints itself via GDI
	◦	InitCommonControlsEx
	4	Implement Common Dialogs (comdlg32.dll subset):
	◦	GetOpenFileNameW, GetSaveFileNameW — file open/save dialogs
	◦	ChooseColorW — color picker dialog
	◦	MessageBoxW, MessageBoxExW — message box
	5	Implement cmd.exe-equivalent (native OS command interpreter): Write a native command interpreter that supports common commands (dir, cd, type, copy, del, mkdir, rmdir, echo, set, start, exit, cls, help, ver). This runs natively on whichever architecture the OS is booted on.
	6	Test with increasingly complex real external Windows applications on BOTH architectures (QEMU + real hardware):
	◦	Simple console apps (Hello World, echo, file copy utilities) — native on x86_64, translated on AArch64
	◦	Simple GUI apps (MessageBox, single window with GDI drawing) — native on x86_64, translated on AArch64
	◦	Apps using common controls (buttons, edit fields, list boxes) — native on x86_64, translated on AArch64
	◦	Apps using file I/O (open/save dialogs, file read/write) — native on x86_64, translated on AArch64
	◦	Open-source Windows applications (e.g., simple editors, viewers, tools compiled for Windows x86_64) running on both x86_64 (native) and AArch64 (translated)
	◦	Test on real hardware: Run these applications on a real x86_64 machine and a real AArch64 machine, not just QEMU. The OS itself runs natively in all cases. Only the external Windows .exe files that don't match the host arch are translated.
PHASE 8: Driver Stack and Hardware Support (Both Architectures, Real Hardware, Native)
Goal: The OS supports common hardware peripherals on real x86_64 and AArch64 hardware — needed for real Windows apps that access hardware, and for the OS to be usable on real machines. All drivers run natively.
	1	Implement PCI/PCIe bus enumeration on both architectures — supporting real hardware, running natively:
	◦	x86_64 (native): PCI config space via I/O ports (0xCF8/0xCFC) for legacy, MMIO ECAM for PCIe (base address from ACPI MCFG table). Enumerate all bus/device/function combinations. Read vendor/device/class IDs, BARs, interrupt pin/line, MSI/MSI-X capabilities. Handle PCI bridges, multifunction devices, and devices behind bridges. Negotiate _OSC method if required by ACPI for PCIe access.
	◦	AArch64 (native): PCI config space via MMIO ECAM (from ACPI MCFG or DTB pci node). Same enumeration logic. ARM platforms may have PCIe controllers that need specific initialization.
	◦	Real hardware consideration: Real PCI/PCIe enumeration must handle devices that don't respond (return 0xFFFF for vendor ID), devices with multiple BARs, devices requiring BAR sizing (write all 1s, read back), and PCIe extended capabilities. MSI/MSI-X setup requires allocating interrupt vectors and mapping them to the interrupt controller.
	2	Implement USB stack on both architectures — supporting real hardware, running natively:
	◦	xHCI host controller driver (the modern USB standard, used by essentially all real hardware since ~2012). Initialize via PCI (x86_64) or DTB/ACPI (AArch64). Implement: controller reset, operational registers, command ring, event ring, transfer rings, port power management, device enumeration (SET_ADDRESS, GET_DESCRIPTOR, SET_CONFIGURATION), endpoint management, interrupt transfers (for HID), bulk transfers (for mass storage).
	◦	x86_64 real hardware: Also support EHCI/OHCI/UHCI for older hardware (legacy USB 2.0 controllers on older machines).
	◦	HID class driver: Parse HID report descriptors, handle keyboard, mouse, gamepad input. Route keyboard/mouse events to the input subsystem.
	◦	Mass storage class driver: USB mass storage (BBB protocol and UAS protocol), SCSI command layer. Mount USB drives via the VFS.
	◦	Real hardware consideration: Real xHCI controllers vary significantly (Intel, AMD, VIA, ASMedia). USB device enumeration timing is critical. Handle device removal during operation. Handle USB hubs.
	3	Implement a network driver on both architectures — supporting real hardware, running natively:
	◦	x86_64 (native): Intel E1000/E1000e (very common on real hardware), Realtek RTL8139/RTL8169/RTL8111 (extremely common on consumer motherboards), virtio-net (for QEMU)
	◦	AArch64 (native): virtio-net (for QEMU), Realtek RTL8111 (common on ARM SBCs with PCIe), Broadcom BCM2711/BCM2712 (Raspberry Pi built-in Ethernet)
	◦	Basic TCP/IP stack (ARP, IP, ICMP, TCP, UDP). Enable ping, DHCP, DNS resolution, and optionally a simple HTTP client. Windows network APIs (WSASocket, connect, send, recv, WSAStartup, gethostbyname) should work.
	◦	Real hardware consideration: Real NIC drivers require proper PCI BAR mapping, DMA buffer setup, interrupt handling, and PHY management. Start with Intel E1000 and Realtek RTL8139.
	4	Implement an RTC driver on both architectures (native):
	◦	x86_64: CMOS/RTC (index/data ports 0x70/0x71), read BCD time. Also implement TSC-based monotonic clock (rdtsc) and HPET for high-resolution timing.
	◦	AArch64: Architectural timer (CNTV_CT_EL0) for monotonic time. UEFI runtime services (GetTime) for wall-clock time. Some ARM boards have an RTC device accessible via I2C.
	5	Implement an audio driver on both architectures — supporting real hardware, running natively:
	◦	x86_64 (native): Intel HDA (High Definition Audio — the standard on essentially all modern x86_64 motherboards and laptops), AC97 (older), Sound Blaster (legacy)
	◦	AArch64 (native): virtio-snd (QEMU), USB audio class driver, I2S audio (for Raspberry Pi and ARM SBCs)
	◦	Basic PCM playback: waveOutOpen, waveOutWrite, waveOutClose, PlaySoundW, Beep
	◦	Real hardware consideration: Intel HDA requires CORB/RIRB setup, stream allocation, BDL management, codec initialization. Real HDA codecs vary significantly between vendors (Realtek, Conexant, Cirrus Logic).
	6	Implement GPU/display driver (optional, lower priority but important for real hardware):
	◦	Software rendering fallback: Always available — the compositor uses CPU-based rendering to the framebuffer if no GPU driver is present. This works on all hardware but is slower. The software renderer runs natively on both architectures.
	◦	Basic GPU acceleration: Implement a basic GPU driver for common GPUs — at minimum, 2D acceleration (solid fill, bit blit) for the compositor. Candidates: virtio-gpu (QEMU), Intel GMA / Intel HD Graphics (very common on x86_64), basic framebuffer-only mode (works everywhere but no acceleration).
	◦	Real hardware consideration: Real GPU drivers are extremely complex. The realistic approach is: (1) use UEFI GOP framebuffer for basic display, (2) implement virtio-gpu for QEMU testing, (3) implement basic Intel GMA/HD 2D acceleration for x86_64, (4) leave full 3D acceleration as a long-term goal.
	7	Implement ACPI/PSCI power management (for real hardware shutdown/reboot/sleep, running natively):
	◦	x86_64 shutdown: Write to PM1a_CNT register (SLP_TYP from FADT), or use UEFI runtime ResetSystemservice, or ACPI reset register.
	◦	x86_64 reboot: ACPI reset register, keyboard controller reset (0x64 port), or triple fault.
	◦	AArch64 power management: PSCI (Power State Coordination Interface) — PSCI_SYSTEM_OFF, PSCI_SYSTEM_RESET, PSCI_CPU_ON. PSCI is invoked via HVC/SMC instructions.
	◦	Sleep/resume: (stretch goal) ACPI S3/S4 sleep states, save/restore device state.
PHASE 9: System Services and Usability
Goal: The OS feels like a real operating system with system services, configuration, and polish — comparable to a real Windows installation, on both architectures, on real hardware. All system services run natively.
	1	Implement a service manager (native, architecture-independent) equivalent to the Windows Service Control Manager (SCM): Start services at boot, manage service lifecycles. Implement StartServiceCtrlDispatcherW, RegisterServiceCtrlHandlerW, SetServiceStatus, ControlService.
	2	Implement a configuration system: Windows-style configuration via the Registry and INI files.
	3	Implement environment variables and process environment blocks: GetEnvironmentVariableW, SetEnvironmentVariableW, ExpandEnvironmentStringsW. Standard variables: PATH, SystemRoot, TEMP, USERPROFILE, COMPUTERNAME, OS, PROCESSOR_ARCHITECTURE (returns AMD64 on x86_64 host, ARM64 on AArch64 host — or the guest architecture for translated external processes).
	4	Implement a package/boot image builder for both architectures: Host-side tools that assemble the ISO with the kernel, bootloader, FAT32 root filesystem, system DLLs (built-in shims, not external files), system programs, and system configuration. The root filesystem contains:
	◦	\Windows\System32\ — system DLLs (built-in shims recognized by the PE loader)
	◦	\Windows\System32\config\ — registry hives
	◦	\Windows\ — system executables
	◦	\Users\Public\ or \Users\Default\ — default user profile
	◦	\Program Files\ — installed applications (externally downloaded Windows .exe files go here)
	◦	Both x86_64 and AArch64 ISOs share the same root filesystem structure (since the system DLLs are built-in shims, not real PE files). If real PE DLLs are used, the ISO must include the correct architecture's DLLs.
	◦	Real hardware consideration: The ISO must be bootable on real hardware. This means the UEFI boot partition must be correctly formatted (FAT32, EFI Boot Partition), the boot loader must be a real .efi file that UEFI firmware can load, and the ISO must use the El Torito boot catalog for both BIOS and UEFI boot. Use xorriso or mkisofs with correct boot catalog entries. Test on real hardware.
	5	Implement a system monitor / Task Manager (native OS app): A GUI app showing CPU usage, memory usage, process list (with NtQuerySystemInformation), and uptime. Show whether each process is native or translated, and the guest/host architecture. Show hardware information (CPU model from CPUID on x86_64 or MIDR_EL1 on AArch64, total RAM, disk usage). Allow killing processes.
	6	Add a boot splash screen and a clean shutdown procedure: ExitWindowsEx, InitiateSystemShutdown, ACPI power-off (NtShutdownSystem). AArch64 uses PSCI (PSCI_SYSTEM_OFF). The boot splash should display on real hardware displays.
	7	Implement Windows-style path environment: GetSystemDirectoryW, GetWindowsDirectoryW, GetTempPathW, GetCurrentDirectoryW, SetCurrentDirectoryW.
	8	Implement architecture detection and reporting: The OS should know its host architecture and report it correctly. Translated external processes should report the guest architecture to maintain Windows compatibility (e.g., IsWow64Process, GetNativeSystemInfo).
	9	Implement a hardware info / device manager (native OS app): A GUI app or command that lists all discovered hardware (PCI devices, USB devices, storage controllers, network adapters) and their status. This is essential for debugging on real hardware.
PHASE 10: Polish and Documentation
Goal: The project is presentable, documented, reproducible, and demonstrably runs real Windows applications on both architectures on real hardware.
	1	Update the README with:
	◦	Screenshots of the running desktop with multiple Windows applications open on both x86_64 and AArch64
	◦	A "Supported Applications" section listing tested Windows programs that run correctly on each architecture
	◦	A "Cross-Architecture Compatibility" section showing the same x86_64 Windows app running natively on x86_64 and translated on AArch64
	◦	A "Hardware Compatibility" section listing tested real hardware
	◦	A clear statement that the OS itself runs natively on both architectures and that translation is only used for externally downloaded Windows programs that don't match the host architecture
	◦	Build instructions for both architectures
	◦	A "Win32 API Coverage" matrix
	◦	A "Binary Translation Status" section showing which guest/host combinations are supported
	◦	A "Hardware Support" matrix
	2	Write a CONTRIBUTING.md with code style, testing guidelines, how to add new NT syscalls, how to add new Win32 API functions, how to add new GDI operations, how to add new instruction translations to the JIT translator, and how to add new hardware drivers.
	3	Write a BUILD.md with detailed step-by-step build instructions for:
	◦	x86_64 BIOS boot (QEMU, real hardware)
	◦	x86_64 UEFI boot (QEMU, real hardware)
	◦	AArch64 UEFI boot (QEMU virt, Raspberry Pi 5)
	◦	Cross-compilation from x86_64 host to AArch64 target
	◦	How to create a bootable USB stick from the ISO (using dd, Rufus, Etcher)
	◦	How to boot on real hardware (BIOS key, UEFI boot menu, secure boot considerations)
	4	Write a WIN32_COMPATIBILITY.md documenting:
	◦	Which Win32 APIs are implemented and to what degree
	◦	Known limitations and unsupported features
	◦	How to test Windows applications
	◦	How the PE loader works
	◦	How DLL import resolution works (built-in native shims for system DLLs, external PE DLLs for user-provided DLLs)
	5	Write a TRANSLATION.md documenting:
	◦	The binary translation architecture (FEX-style JIT + interpreter fallback)
	◦	The critical distinction: the OS itself runs natively; only external PE binaries are translated
	◦	How translated programs call native system DLLs: import thunks route to native DLL functions, which run at full native speed
	◦	Supported guest/host combinations and their status
	◦	How to add new instruction translations
	◦	Performance characteristics and optimization roadmap
	◦	How translation interacts with the Win32 subsystem (window procedures, callbacks, exceptions, syscalls)
	6	Write a ARCHITECTURE.md documenting:
	◦	The x86_64 HAL design (native)
	◦	The AArch64 HAL design (native)
	◦	The architecture-independent kernel core
	◦	The Win32 subsystem architecture (native, part of the OS)
	◦	The binary translation subsystem architecture (for external programs only)
	◦	The hardware driver architecture and device tree / ACPI integration
	◦	How the four pillars (native dual-arch kernel, native Win32 compatibility, cross-arch translation for external programs, real hardware support) interconnect
	7	Write a HARDWARE_COMPATIBILITY.md documenting:
	◦	Tested hardware (x86_64 and AArch64)
	◦	Known issues and workarounds per hardware
	◦	How to add support for a new hardware device
	◦	ACPI/DTB parsing details and quirks
	◦	Driver development guide
	8	Add automated testing: CI pipeline that:
	◦	Builds the ISO for both x86_64 and AArch64
	◦	Boots the x86_64 ISO in QEMU and verifies the kernel reaches the desktop
	◦	Boots the AArch64 ISO in QEMU and verifies the kernel reaches the desktop
	◦	Launches a known x86_64 Windows test executable on x86_64 QEMU (native) and verifies output
	◦	Launches a known x86_64 Windows test executable on AArch64 QEMU (translated) and verifies output
	9	Create a release pipeline that builds ISOs for both architectures and publishes them as GitHub release artifacts.
	10	Include sample Windows test programs in the repository (compiled .exe files for x86, x86_64, and ARM64, or source that can be cross-compiled) to demonstrate native and translated execution.
	11	Create a hardware testing checklist for users to verify ApertureOS on their own hardware.

What "Done" Looks Like
On x86_64 Real Hardware
A user downloads the x86_64 ISO, writes it to a USB stick, boots their PC from USB, and within 15 seconds sees:
	1	A boot splash screen on their real monitor
	2	A graphical desktop (running natively as x86_64 code) with a real mouse cursor, a wallpaper, a taskbar with a clock and a Start menu
	3	A terminal/console window (native OS app) they can type into, which can execute built-in commands
	4	A file manager window (native OS app) showing files on the boot disk (C:\) and optionally on other detected storage devices
	5	The ability to launch and run externally downloaded real Windows x86_64 .exe files natively — console apps produce output, GUI apps create windows, file I/O works, message boxes appear, GDI drawing works. These run at full native x86_64 speed.
	6	The ability to launch and run externally downloaded real Windows x86 (32-bit) .exe files via WoW64
	7	The ability to launch and run externally downloaded real Windows ARM64 .exe files via the ARM64→x86_64 translator
	8	Windows applications that use common controls display and function correctly
	9	The Windows Registry is functional — programs can read/write registry keys
	10	A system monitor / Task Manager (native OS app) showing CPU usage, memory usage, process list, and hardware info
	11	A clean shutdown option that powers off the machine via ACPI
On AArch64 Real Hardware
A user downloads the AArch64 ISO, writes it to a USB stick or SD card, boots their ARM64 device, and within 15 seconds sees:
	1	A boot splash screen on their real monitor
	2	A graphical desktop (running natively as AArch64 code) with a real mouse cursor, a wallpaper, a taskbar with a clock and a Start menu
	3	A terminal/console window (native OS app) they can type into
	4	A file manager window (native OS app) showing files on the boot disk (C:\)
	5	The ability to launch and run externally downloaded real Windows x86_64 .exe files via the JIT translator — console apps produce output, GUI apps create windows, file I/O works, message boxes appear, GDI drawing works. The OS itself (desktop, file manager, terminal, DLLs) runs natively as AArch64. Only the external .exe's code is translated. System DLL calls from the translated program route to the native AArch64 system DLLs.
	6	The ability to launch and run externally downloaded real Windows x86 (32-bit) .exe files via nested translation (x86 → AArch64)
	7	The ability to launch and run externally downloaded real Windows ARM64 .exe files natively — these run at full native AArch64 speed, same as on Windows on ARM
	8	Windows applications that use common controls display and function correctly
	9	The Windows Registry is functional
	10	A system monitor / Task Manager (native OS app) showing CPU usage, memory usage, process list, and translation statistics
	11	A clean shutdown option that powers off the machine via PSCI
Cross-Verification
The same externally downloaded Windows x86_64 application runs on both x86_64 real hardware (native) and AArch64 real hardware (translated), producing identical output and behavior. This is the ultimate demonstration of ApertureOS's cross-architecture vision. In both cases, the OS itself runs natively.
Hardware Independence Verification
The OS boots on at least 3 different real hardware configurations:
	•	One x86_64 desktop or laptop (UEFI boot)
	•	One x86_64 machine with legacy BIOS (if possible)
	•	One AArch64 device (Raspberry Pi 5, ARM64 SBC, or ARM64 laptop)
The system must run in QEMU for development and CI, and on real hardware for the final product. The build process must be fully automated: ./build.sh x86_64 && ./make-iso.sh x86_64 and ./build.sh aarch64 && ./make-iso.sh aarch64. Both ISOs must include at least one real Windows test application for each supported PE architecture.
The ultimate success criteria:
	1	A real Windows application (not a synthetic test) runs correctly under ApertureOS x86_64 on real hardware with visible output and correct behavior. The OS and all its components run natively as x86_64.
	2	The same real Windows application runs correctly under ApertureOS AArch64 via the binary translator, on real hardware, with visible output and correct behavior. The OS and all its components run natively as AArch64. Only the external .exe is translated.
	3	The AArch64 ISO boots independently to a full desktop on real hardware — it is a fully native AArch64 OS, not dependent on x86_64 in any way.
	4	The x86_64 ISO boots independently to a full desktop on real hardware — it is a fully native x86_64 OS, not dependent on QEMU in any way.
	5	The OS's own components (kernel, drivers, DLLs, desktop, apps) are never translated on any architecture. Translation is exclusively a compatibility feature for externally downloaded Windows PE binaries that don't match the host architecture.
Technical Constraints and Guidelines
	1	Language: All kernel and driver code must be written in Rust using no_std and the appropriate target (x86_64-unknown-none or aarch64-unknown-none). Use unsafe sparingly and wrap it in safe abstractions. The Win32 subsystem DLLs are implemented as built-in kernel-resident shims that the PE loader recognizes and resolves transparently. They are compiled natively per architecture. They are NOT external PE DLLs and are never translated.
	2	Dual-architecture is a hard requirement — the OS runs natively on both: Every feature must work on both x86_64 and AArch64. Architecture-specific code goes in kernel/arch/x86_64/ or kernel/arch/aarch64/. Architecture-independent code (kernel core, scheduler, memory management API, VFS, Win32 subsystem, NT syscall semantics, GDI, window manager, compositor) goes in shared modules and is compiled natively for whichever target. Never write architecture-specific code in shared modules — use trait objects or cfg-gated dispatch. The OS is NOT a translated/emulated system on AArch64. It is a real native AArch64 OS that happens to also be able to run x86/x86_64 Windows binaries via translation.
	3	Binary translation is a hard requirement — but only for external programs: The cross-architecture translators (x86_64→AArch64, x86→AArch64, x86→x86_64, ARM64→x86_64) must be implemented. The x86_64→AArch64 translator is the highest priority. The translation system is inspired by FEX-Emu's architecture: JIT compilation of basic blocks with an interpreter fallback, code caching, and profile-guided optimization. Translation is ONLY invoked when an externally loaded PE binary's machine type does not match the host architecture. The OS's own components are always native. Reference FEX-Emu's source code, documentation, and design for implementation guidance.
	4	Native system DLLs: The system DLLs (ntdll, kernel32, user32, gdi32, comctl32, comdlg32, etc.) are implemented in Rust as part of the Win32 subsystem and compiled natively per architecture. When a translated external program imports these DLLs, the PE loader resolves the imports against the built-in native shims. The translated program's calls to system DLL functions are routed to the native implementation, which runs at full native speed. Only the external program's own code is translated. This is the same approach as FEX-Emu (host-native libraries) and Rosetta 2 (host-native frameworks).
	5	Real hardware boot is a hard requirement: The kernel must boot on real physical hardware, not just VMs. This means:
	◦	Proper ACPI table parsing (x86_64) and ACPI/DTB parsing (AArch64) — not hardcoded addresses
	◦	Proper UEFI boot services handling and UEFI runtime services usage
	◦	Real PCI/PCIe enumeration via ECAM (not hardcoded device addresses)
	◦	Real interrupt controller initialization (APIC on x86_64, GIC on AArch64) — from ACPI MADT or DTB
	◦	Real timer calibration (LAPIC timer calibration via TSC, AArch64 timer frequency from CNTFRQ_EL0 or DTB)
	◦	Real framebuffer discovery and pixel format handling (from UEFI GOP, Multiboot2, or DTB)
	◦	Real storage controller drivers (AHCI, NVMe, SD/MMC) — not just virtio-blk
	◦	Real input device drivers (PS/2 keyboard/mouse, USB HID) — not just QEMU's emulated devices
	◦	Real RTC/clock sources (CMOS RTC, UEFI runtime, AArch64 architectural timer)
	◦	Real power management (ACPI PM1, PSCI)
	◦	Graceful handling of missing devices, device timeouts, and firmware quirks
	◦	The kernel must never assume QEMU-specific behavior. Every address, register, and device must be discovered dynamically at boot time from firmware-provided information (ACPI, DTB, UEFI, PCI enumeration).
	6	Windows ABI fidelity: The NT syscall numbers, struct layouts, calling conventions, and error codes (NTSTATUSvalues) must match real Windows as closely as possible. Reference the Windows NT internals documentation, the ReactOS source code, and the WDK headers for correct definitions. The CONTEXT struct, PEB, TEB, and UNICODE_STRING layouts are architecture-specific.
	7	PE compatibility: The PE loader must handle real PE files produced by MSVC, MinGW/GCC, and Clang for Windows. This includes PE32 (32-bit), PE32+ (64-bit), base relocations, import tables, export tables, TLS directories, resource directories, and delay-load imports. The PE machine type (IMAGE_FILE_MACHINE_AMD64, IMAGE_FILE_MACHINE_I386, IMAGE_FILE_MACHINE_ARM64, IMAGE_FILE_MACHINE_ARM) determines whether native execution or translation is used. OS-internal PE files (if any are used as real PE binaries) always match the host architecture.
	8	No external Windows files required: The system must NOT require a Windows installation, Windows DLLs from a real Windows system, or any Microsoft-licensed files to function. All system DLLs are reimplemented in Rust and compiled natively.
	9	No external translation layers required: The binary translation is built into the OS kernel — it does not depend on FEX-Emu, QEMU user-mode, Rosetta 2, or any external emulation tool. The translator is part of ApertureOS.
	10	Bootloader: Use either Limine (modern, simpler, supports both x86_64 and AArch64, excellent real hardware support) or GRUB. If the current tools/bootimage/ approach is broken, consider switching to Limine for reliability and cross-architecture support. The bootloader must work on real hardware — test the ISO on real machines.
	11	No external runtime dependencies: The kernel must not depend on std. All functionality must be implemented from scratch or using no_std crates like spin, linked_list_allocator, bitflags, etc.
	12	Testing: Every major component must have host-testable unit tests using cargo test (where possible) or QEMU integration tests. Windows application compatibility must be tested with real PE executables. Cross-architecture translation must be tested by running x86_64 PE on AArch64 QEMU and ARM64 PE on x86_64 QEMU. Real hardware boot must be tested on at least one real x86_64 machine and one real AArch64 machine.
	13	Error handling: Use Result types throughout the kernel. The NT syscall layer must return NTSTATUS codes matching Windows semantics. The translator must handle unsupported instructions gracefully (interpreter fallback or diagnostic exception, never kernel panic). Drivers must handle device timeouts, missing devices, and hardware errors gracefully. Panics are reserved for unrecoverable kernel state corruption.
	14	Code style: Follow rustfmt defaults. Use meaningful names. Document public items with /// doc comments. Mirror Windows struct/function names where they correspond to Windows types.
	15	Defensive programming for real hardware: Every device access must have a timeout, a null check, an error recovery path, and a log message. Never assume a device responds instantly. Never assume a register contains expected values. Never assume the memory map is what you expect. Real hardware is unpredictable and buggy — the kernel must be robust.
Execution Order
Work through phases sequentially. Do not start Phase N+1 until Phase N is demonstrably working (testable in QEMU, and for hardware-related phases, on real hardware). Commit after each significant sub-task with a descriptive message. Push working builds. Release an ISO after each completed phase.
Priority ordering:
	1	Phase 1A + 1B + 1C (Boot fix on both architectures, including real hardware robustness) — the absolute #1 priority. Fix x86_64 boot first (native), then implement AArch64 boot (native), then add real hardware robustness (ACPI, graceful device absence, timeout handling).
	2	Phase 2 (Kernel stability on both architectures, native) — make both architectures stable, including real hardware interrupt controllers and timers.
	3	Phase 3 (PE loading and Windows execution on x86_64, native) — get real Windows programs running on x86_64 first (native execution, no translation needed). Implement system DLLs as native built-in shims. Test on real hardware.
	4	Phase 4 (Binary translation for external programs) — implement the x86_64→AArch64 translator and get the same externally downloaded Windows programs running on AArch64 via translation. The OS itself remains fully native. This is the project's defining technical achievement. Test on real AArch64 hardware.
	5	Phase 5 (Storage on real hardware, native drivers) — implement real hardware storage drivers (AHCI, NVMe, SD/MMC). Test on real hardware with real disks.
	6	Phase 6 (GUI desktop on real hardware, native) — implement the full desktop environment (native). Test on real hardware with real displays and real input devices.
	7	Phases 7-10 — Win32 API expansion, real hardware drivers (USB, network, audio), system services, polish — on both architectures, with real hardware validation at each step. The OS itself always runs natively. Translation is only for external Windows .exe files that don't match the host architecture.
After Phase 4 is complete, every subsequent phase must be validated on BOTH x86_64 (native OS) and AArch64 (native OS), and on BOTH QEMU and real hardware. Do not develop a feature only on x86_64 and leave AArch64 as an afterthought. Do not test only in QEMU and leave real hardware as an afterthought. Remember: the OS is always native; only external programs are translated.
Start immediately with Phase 1A. The #1 priority is fixing the x86_64 boot path so the ISO boots into the kernel instead of dropping to a GRUB prompt. Then immediately tackle Phase 1B to get AArch64 booting natively. Then tackle Phase 1C to make the boot path robust on real hardware. The native dual-architecture kernel, native Win32 compatibility, cross-arch translation for external programs, and real hardware vision is what makes this project unique — it must be central to every design decision.

Reference materials:
	•	Windows Internals (Russinovich) — NT kernel architecture, PEB/TEB, object manager, syscall dispatch
	•	ReactOS source code (reactos.org) — real-world NT-compatible implementation reference
	•	WDK/SDK headers — struct layouts, syscall numbers, NTSTATUS codes, PE format
	•	OSDev wiki — low-level x86_64 and AArch64 bring-up, ACPI, PCI, UEFI, real hardware guides
	•	Phil Opp blog series — Rust no_std kernel development
	•	FEX-Emu source code (github.com/FEX-Emu/FEX) — JIT translation architecture, block caching, interpreter design, native library call mechanism (how FEX calls host-native libraries from translated guest code)
	•	Apple Rosetta 2 technical discussions — translation-based execution design, native framework calls
	•	ARM Architecture Reference Manual (ARMv8-A) — AArch64 instruction encoding, exception handling, memory management, GIC
	•	Intel SDM — x86_64 instruction encoding, SYSCALL/SYSRET, paging, IDT, APIC, HPET
	•	UEFI Specification — boot protocol, GOP, runtime services, memory map (both x86_64 and AArch64)
	•	ACPI Specification — table formats, AML, device enumeration, power management
	•	Device Tree Specification — FDT format, bindings for ARM hardware
	•	PCIe Specification — ECAM, config space, BARs, MSI/MSI-X, capabilities
	•	xHCI Specification — USB 3.0 host controller, command/event rings, transfer descriptors
	•	AHCI Specification — SATA host bus controller, command list, FIS
	•	NVMe Specification — NVMe admin commands, I/O queues, PRP/SGL
	•	Intel HDA Specification — HD Audio controller, CORB/RIRB, codec verbs


PHASE 11: Final Commit, Push, and GitHub Release (Mandatory Final Step)
This phase is mandatory. The project is not "done" until the code is committed, pushed, and a release is published on GitHub with both ISOs downloadable. Do not skip this. Do not leave it as a TODO. Execute it.
Prerequisites Checklist (verify ALL pass before proceeding)
Before committing and releasing, verify that every single one of these is true. If any item fails, go back and fix it — do not release a broken product:
	1	x86_64 ISO builds successfully: ./build.sh x86_64 && ./make-iso.sh x86_64 produces build/ApertureOS-x86_64.iso with zero errors.
	2	AArch64 ISO builds successfully: ./build.sh aarch64 && ./make-iso.sh aarch64 produces build/ApertureOS-aarch64.iso with zero errors.
	3	x86_64 ISO boots in QEMU: qemu-system-x86_64 -cdrom build/ApertureOS-x86_64.iso -serial stdio -m 512M boots into the graphical desktop. No GRUB prompt, no hang, no crash.
	4	AArch64 ISO boots in QEMU: qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M -bios QEMU_EFI.fd -cdrom build/ApertureOS-aarch64.iso -serial stdio boots into the graphical desktop. No hang, no crash.
	5	x86_64 ISO boots on real hardware: Burned to USB and booted on at least one real x86_64 machine — reaches the desktop with mouse, keyboard, and display working.
	6	AArch64 ISO boots on real hardware: Booted on at least one real AArch64 machine (e.g., Raspberry Pi 5) — reaches the desktop with input and display working.
	7	A real Windows x86_64 .exe runs on x86_64 (native): Verified in QEMU and on real hardware. The program produces visible correct output.
	8	The same real Windows x86_64 .exe runs on AArch64 (translated): Verified in QEMU and on real hardware. The program produces visible correct output identical to the x86_64 native run.
	9	The OS itself runs natively on both architectures: The kernel, drivers, system DLLs, desktop environment, file manager, terminal, and task manager all run natively (x86_64 code on x86_64, AArch64 code on AArch64). No OS component is translated. Only externally loaded Windows .exe files that don't match the host arch are translated.
	10	The README is updated: Contains screenshots, build instructions, architecture overview, Win32 API coverage, translation status, hardware support matrix, and a clear statement that the OS runs natively on both architectures with translation only for external Windows programs.
	11	All documentation files exist and are accurate: CONTRIBUTING.md, BUILD.md, WIN32_COMPATIBILITY.md, TRANSLATION.md, ARCHITECTURE.md, HARDWARE_COMPATIBILITY.md.
	12	The build process is fully automated: A single ./build.sh x86_64 && ./make-iso.sh x86_64 and ./build.sh aarch64 && ./make-iso.sh aarch64 produces the ISOs with no manual intervention.
	13	All tests pass: cargo test passes for all host-testable unit tests. QEMU integration tests pass for both architectures.
	14	No panics on expected error conditions: The kernel does not panic when a device is missing, a file is not found, a Windows API returns an error, or a translated instruction is unsupported (interpreter fallback is used instead).
Step 1: Final Code Audit and Cleanup
	1	Run cargo fmt on the entire workspace. Ensure all code is formatted.
	2	Run cargo clippy on the entire workspace. Fix or justify all warnings. No clippy warnings in the final commit.
	3	Remove all dead code, unused imports, TODO comments that are not tracked issues, and temporary debug code.
	4	Verify the CHANGELOG (if one exists) is up to date with all features implemented in this release.
	5	Verify the version number in Cargo.toml and all workspace manifests is set to the release version (e.g., 1.0.0).
	6	Verify .gitignore excludes build artifacts (build/, target/) but includes source for all sample test programs and documentation.
Step 2: Final Commit
	1	Stage all changes: git add -A
	2	Create the final commit with a comprehensive message:

git commit -m "release: ApertureOS v1.0.0 — Complete dual-architecture OS with Win32 compatibility and cross-arch translation

This release delivers a complete, bootable, general-purpose operating system
that runs natively on both x86_64 and AArch64 hardware.

Key achievements:
- Native x86_64 and AArch64 kernel with preemptive multitasking, real hardware
  support (ACPI/DTB, PCI/PCIe, APIC/GIC, real storage/input/network/audio drivers)
- Full Win32 subsystem implementing the NT kernel ABI natively in Rust:
  PE loader, NT syscalls, object manager, process/thread model, registry,
  GDI, window manager, compositor, common controls, common dialogs
- System DLLs (ntdll, kernel32, user32, gdi32, comctl32, comdlg32) implemented
  as native built-in shims — compiled and running natively per architecture
- Binary translation (FEX-style) for externally downloaded Windows PE binaries:
  x86_64→AArch64, x86→AArch64, x86→x86_64 (WoW64), ARM64→x86_64
  JIT compilation with interpreter fallback, code caching, PGO
- Real Windows .exe files run natively (arch matches) or via translation (arch differs)
- The OS itself always runs natively — only external PE binaries are translated
- Full graphical desktop: window manager, taskbar, start menu, file manager,
  terminal, text editor, task manager, device manager
- Boots on real hardware (tested on x86_64 UEFI, AArch64 Raspberry Pi 5)
- FAT32/NTFS filesystem support with Windows-compatible path semantics (C:\, UNC)
- USB stack (xHCI), network stack (Intel E1000, Realtek, virtio-net),
  audio (Intel HDA, virtio-snd), ACPI/PSCI power management
- Windows-compatible: registry, environment variables, services, console,
  file I/O, networking APIs all functional

Tested:
- x86_64 ISO boots on QEMU (BIOS + UEFI) and real hardware
- AArch64 ISO boots on QEMU (UEFI) and real hardware (Raspberry Pi 5)
- Real Windows x86_64 .exe runs natively on x86_64 and translated on AArch64
- Same output and behavior on both architectures"
git commit -m "release: ApertureOS v1.0.0 — Complete dual-architecture OS with Win32 compatibility and cross-arch translation

This release delivers a complete, bootable, general-purpose operating system
that runs natively on both x86_64 and AArch64 hardware.

Key achievements:
- Native x86_64 and AArch64 kernel with preemptive multitasking, real hardware
  support (ACPI/DTB, PCI/PCIe, APIC/GIC, real storage/input/network/audio drivers)
- Full Win32 subsystem implementing the NT kernel ABI natively in Rust:
  PE loader, NT syscalls, object manager, process/thread model, registry,
  GDI, window manager, compositor, common controls, common dialogs
- System DLLs (ntdll, kernel32, user32, gdi32, comctl32, comdlg32) implemented
  as native built-in shims — compiled and running natively per architecture
- Binary translation (FEX-style) for externally downloaded Windows PE binaries:
  x86_64→AArch64, x86→AArch64, x86→x86_64 (WoW64), ARM64→x86_64
  JIT compilation with interpreter fallback, code caching, PGO
- Real Windows .exe files run natively (arch matches) or via translation (arch differs)
- The OS itself always runs natively — only external PE binaries are translated
- Full graphical desktop: window manager, taskbar, start menu, file manager,
  terminal, text editor, task manager, device manager
- Boots on real hardware (tested on x86_64 UEFI, AArch64 Raspberry Pi 5)
- FAT32/NTFS filesystem support with Windows-compatible path semantics (C:\, UNC)
- USB stack (xHCI), network stack (Intel E1000, Realtek, virtio-net),
  audio (Intel HDA, virtio-snd), ACPI/PSCI power management
- Windows-compatible: registry, environment variables, services, console,
  file I/O, networking APIs all functional

Tested:
- x86_64 ISO boots on QEMU (BIOS + UEFI) and real hardware
- AArch64 ISO boots on QEMU (UEFI) and real hardware (Raspberry Pi 5)
- Real Windows x86_64 .exe runs natively on x86_64 and translated on AArch64
- Same output and behavior on both architectures"
	3	Verify the commit is clean: git log --stat -1 shows all expected files changed, nothing missing, nothing unexpected.
Step 3: Push to GitHub
	1	Push the main branch: git push origin main
	2	Verify the push succeeded: Check https://github.com/theworkingman-beep/ApertureOS shows the latest commit.
	3	If the push fails (e.g., behind remote, force needed), resolve the conflict and retry. Do not skip the push.
Step 4: Create Git Tag
	1	Create an annotated tag: git tag -a v1.0.0 -m "ApertureOS v1.0.0 — Complete dual-architecture OS with Win32 compatibility and cross-arch translation"
	2	Push the tag: git push origin v1.0.0
Step 5: Create GitHub Release via gh CLI
	1	Verify gh CLI is installed and authenticated: bash  gh auth status
	2	 gh auth status
	3	  If not authenticated, authenticate: gh auth login
	4	Copy both ISOs to a known location: bash  cp build/ApertureOS-x86_64.iso /tmp/ApertureOS-x86_64-v1.0.0.iso
	5	cp build/ApertureOS-aarch64.iso /tmp/ApertureOS-aarch64-v1.0.0.iso
	6	 cp build/ApertureOS-x86_64.iso /tmp/ApertureOS-x86_64-v1.0.0.iso
	7	cp build/ApertureOS-aarch64.iso /tmp/ApertureOS-aarch64-v1.0.0.iso
	8	 
	9	Create the GitHub release with both ISOs attached as release assets: bash  gh release create v1.0.0 \
	10	  /tmp/ApertureOS-x86_64-v1.0.0.iso \
	11	  /tmp/ApertureOS-aarch64-v1.0.0.iso \
	12	  --repo theworkingman-beep/ApertureOS \
	13	  --title "ApertureOS v1.0.0 — Complete OS with Win32 Compatibility & Cross-Architecture Support" \
	14	  --notes "## ApertureOS v1.0.0
	15	
	16	### What is ApertureOS?
	17	
	18	ApertureOS is a complete, general-purpose operating system written in Rust
	19	that runs **natively on both x86_64 and AArch64 hardware**. It implements
	20	the **Windows NT kernel ABI natively**, allowing real Windows PE executables
	21	to run as first-class processes — without Wine, without a host Windows
	22	installation, and without any external compatibility layer.
	23	
	24	### Key Features
	25	
	26	- **Native dual-architecture**: The OS itself runs natively on x86_64 and AArch64.
	27	  The kernel, drivers, system DLLs, and all OS components are compiled natively
	28	  per architecture. No translation of OS components.
	29	- **Full Win32 subsystem**: NT syscalls, object manager, process/thread model,
	30	  registry, GDI, window manager, common controls, common dialogs — all
	31	  implemented natively in Rust.
	32	- **Cross-architecture binary translation (FEX-style)**: Externally downloaded
	33	  Windows PE binaries run on any host architecture:
	34	  - x86_64 PE on AArch64 (JIT translation)
	35	  - x86 PE on x86_64 (WoW64) and on AArch64 (nested translation)
	36	  - ARM64 PE on x86_64 (JIT translation)
	37	  - x86_64 PE on x86_64 and ARM64 PE on AArch64 (native execution, no translation)
	38	- **Real hardware support**: ACPI, UEFI, PCI/PCIe, real storage (AHCI/NVMe/SD),
	39	  USB (xHCI), network (E1000/Realtek/virtio-net), audio (HDA/virtio-snd),
	40	  PS/2 and USB input devices, ACPI/PSCI power management.
	41	- **Full graphical desktop**: Window manager, compositor, taskbar, start menu,
	42	  file manager, terminal, text editor, task manager, device manager.
	43	- **Windows-compatible filesystem**: FAT32 and NTFS, drive letters (C:\\),
	44	  UNC paths, \\??\\C: namespace, Windows-style paths.
	45	
	46	### Downloads
	47	
	48	- **ApertureOS-x86_64-v1.0.0.iso** — Boot on any x86_64 PC (BIOS or UEFI).
	49	  Runs x86_64 and x86 (32-bit) Windows .exe files natively.
	50	  Runs ARM64 Windows .exe files via translation.
	51	- **ApertureOS-aarch64-v1.0.0.iso** — Boot on any AArch64 device (Raspberry Pi 5,
	52	  ARM64 SBC, ARM64 laptop) via UEFI. Runs ARM64 Windows .exe files natively.
	53	  Runs x86_64 and x86 (32-bit) Windows .exe files via translation.
	54	
	55	### How to Boot
	56	
	57	1. Download the ISO for your architecture.
	58	2. Write to a USB stick: \`dd if=ApertureOS-x86_64-v1.0.0.iso of=/dev/sdX bs=4M status=progress\`
	59	   (or use Rufus/Etcher on Windows).
	60	3. Boot from USB on your machine. The OS boots to a graphical desktop.
	61	
	62	### Build from Source
	63	
	64	\`\`\`bash
	65	# x86_64
	66	./build.sh x86_64 && ./make-iso.sh x86_64
	67	
	68	# AArch64
	69	./build.sh aarch64 && ./make-iso.sh aarch64
	70	\`\`\`
	71	
	72	See BUILD.md for detailed instructions.
	73	
	74	### Tested Hardware
	75	
	76	See HARDWARE_COMPATIBILITY.md for the full list of tested hardware.
	77	
	78	### Win32 API Coverage
	79	
	80	See WIN32_COMPATIBILITY.md for the API coverage matrix.
	81	
	82	### Translation Status
	83	
	84	See TRANSLATION.md for the binary translation status and supported combinations.
	85	
	86	### Architecture
	87	
	88	See ARCHITECTURE.md for the full system architecture documentation.
	89	
	90	### License
	91	
	92	MIT OR Apache-2.0
	93	" \
	94	  --latest
	95	 gh release create v1.0.0 \
	96	  /tmp/ApertureOS-x86_64-v1.0.0.iso \
	97	  /tmp/ApertureOS-aarch64-v1.0.0.iso \
	98	  --repo theworkingman-beep/ApertureOS \
	99	  --title "ApertureOS v1.0.0 — Complete OS with Win32 Compatibility & Cross-Architecture Support" \
	100	  --notes "## ApertureOS v1.0.0
	101	
	102	### What is ApertureOS?
	103	
	104	ApertureOS is a complete, general-purpose operating system written in Rust
	105	that runs **natively on both x86_64 and AArch64 hardware**. It implements
	106	the **Windows NT kernel ABI natively**, allowing real Windows PE executables
	107	to run as first-class processes — without Wine, without a host Windows
	108	installation, and without any external compatibility layer.
	109	
	110	### Key Features
	111	
	112	- **Native dual-architecture**: The OS itself runs natively on x86_64 and AArch64.
	113	  The kernel, drivers, system DLLs, and all OS components are compiled natively
	114	  per architecture. No translation of OS components.
	115	- **Full Win32 subsystem**: NT syscalls, object manager, process/thread model,
	116	  registry, GDI, window manager, common controls, common dialogs — all
	117	  implemented natively in Rust.
	118	- **Cross-architecture binary translation (FEX-style)**: Externally downloaded
	119	  Windows PE binaries run on any host architecture:
	120	  - x86_64 PE on AArch64 (JIT translation)
	121	  - x86 PE on x86_64 (WoW64) and on AArch64 (nested translation)
	122	  - ARM64 PE on x86_64 (JIT translation)
	123	  - x86_64 PE on x86_64 and ARM64 PE on AArch64 (native execution, no translation)
	124	- **Real hardware support**: ACPI, UEFI, PCI/PCIe, real storage (AHCI/NVMe/SD),
	125	  USB (xHCI), network (E1000/Realtek/virtio-net), audio (HDA/virtio-snd),
	126	  PS/2 and USB input devices, ACPI/PSCI power management.
	127	- **Full graphical desktop**: Window manager, compositor, taskbar, start menu,
	128	  file manager, terminal, text editor, task manager, device manager.
	129	- **Windows-compatible filesystem**: FAT32 and NTFS, drive letters (C:\\),
	130	  UNC paths, \\??\\C: namespace, Windows-style paths.
	131	
	132	### Downloads
	133	
	134	- **ApertureOS-x86_64-v1.0.0.iso** — Boot on any x86_64 PC (BIOS or UEFI).
	135	  Runs x86_64 and x86 (32-bit) Windows .exe files natively.
	136	  Runs ARM64 Windows .exe files via translation.
	137	- **ApertureOS-aarch64-v1.0.0.iso** — Boot on any AArch64 device (Raspberry Pi 5,
	138	  ARM64 SBC, ARM64 laptop) via UEFI. Runs ARM64 Windows .exe files natively.
	139	  Runs x86_64 and x86 (32-bit) Windows .exe files via translation.
	140	
	141	### How to Boot
	142	
	143	1. Download the ISO for your architecture.
	144	2. Write to a USB stick: \`dd if=ApertureOS-x86_64-v1.0.0.iso of=/dev/sdX bs=4M status=progress\`
	145	   (or use Rufus/Etcher on Windows).
	146	3. Boot from USB on your machine. The OS boots to a graphical desktop.
	147	
	148	### Build from Source
	149	
	150	\`\`\`bash
	151	# x86_64
	152	./build.sh x86_64 && ./make-iso.sh x86_64
	153	
	154	# AArch64
	155	./build.sh aarch64 && ./make-iso.sh aarch64
	156	\`\`\`
	157	
	158	See BUILD.md for detailed instructions.
	159	
	160	### Tested Hardware
	161	
	162	See HARDWARE_COMPATIBILITY.md for the full list of tested hardware.
	163	
	164	### Win32 API Coverage
	165	
	166	See WIN32_COMPATIBILITY.md for the API coverage matrix.
	167	
	168	### Translation Status
	169	
	170	See TRANSLATION.md for the binary translation status and supported combinations.
	171	
	172	### Architecture
	173	
	174	See ARCHITECTURE.md for the full system architecture documentation.
	175	
	176	### License
	177	
	178	MIT OR Apache-2.0
	179	" \
	180	  --latest
	181	 
	182	Verify the release was created:
	◦	Check gh release view v1.0.0 --repo theworkingman-beep/ApertureOS shows the release with both ISO assets.
	◦	Check https://github.com/theworkman-beep/ApertureOS/releases/tag/v1.0.0 in a browser.
	◦	Verify both ISO files are listed as downloadable assets.
	◦	Verify the release body text renders correctly (markdown formatting, no raw markdown showing).
	183	If the release creation fails, debug the error:
	◦	If gh is not authenticated: gh auth login and retry.
	◦	If the tag doesn't exist remotely: git push origin v1.0.0 and retry.
	◦	If the ISO files are too large for GitHub release limits (2GB per asset): Consider compressing them (gzip or xz) or splitting the release. Attach .iso.gz or .iso.xz files instead and note the compression in the release notes.
	◦	If gh release create fails with a network error: Retry with --retry or manually retry the command.
	◦	Do not skip this step. The release must be published.
Step 6: Post-Release Verification
	1	Download both ISOs from the GitHub release page (not from local build) and verify they boot in QEMU: bash  wget https://github.com/theworkman-beep/ApertureOS/releases/download/v1.0.0/ApertureOS-x86_64-v1.0.0.iso
	2	qemu-system-x86_64 -cdrom ApertureOS-x86_64-v1.0.0.iso -serial stdio -m 512M
	3	
	4	wget https://github.com/theworkingman-beep/ApertureOS/releases/download/v1.0.0/ApertureOS-aarch64-v1.0.0.iso
	5	qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M -bios QEMU_EFI.fd -cdrom ApertureOS-aarch64-v1.0.0.iso -serial stdio
	6	 wget https://github.com/theworkman-beep/ApertureOS/releases/download/v1.0.0/ApertureOS-x86_64-v1.0.0.iso
	7	qemu-system-x86_64 -cdrom ApertureOS-x86_64-v1.0.0.iso -serial stdio -m 512M
	8	
	9	wget https://github.com/theworkingman-beep/ApertureOS/releases/download/v1.0.0/ApertureOS-aarch64-v1.0.0.iso
	10	qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M -bios QEMU_EFI.fd -cdrom ApertureOS-aarch64-v1.0.0.iso -serial stdio
	11	  Verify both boot to the desktop. If the downloaded ISOs don't boot, the release is broken — fix the issue, rebuild, and recreate the release.
	12	Verify the release page looks professional: Title, description, screenshots (if any were added to the release notes), and both download links are present and working.
	13	Announce the release (optional but recommended): If the project has any social media presence or community channels, post the release link.
What "Done" Means
The project is "done" when ALL of the following are true:
	1	✅ All code is committed to main and pushed to GitHub
	2	✅ A git tag v1.0.0 is created and pushed
	3	✅ A GitHub release v1.0.0 is published via gh CLI
	4	✅ The release contains both ApertureOS-x86_64-v1.0.0.iso and ApertureOS-aarch64-v1.0.0.iso as downloadable assets
	5	✅ Both ISOs downloaded from the GitHub release boot to the desktop in QEMU
	6	✅ Both ISOs have been verified on real hardware
	7	✅ The release notes accurately describe the project's capabilities
	8	✅ All documentation files (README, BUILD, CONTRIBUTING, WIN32_COMPATIBILITY, TRANSLATION, ARCHITECTURE, HARDWARE_COMPATIBILITY) are present in the repository and linked from the release notes
	9	✅ The OS runs natively on both x86_64 and AArch64
	10	✅ Real Windows .exe files run natively (arch matches) and via translation (arch doesn't match)
	11	✅ The OS's own components are never translated — only external PE binaries are
If any of these are not done, the project is not done. Go back and complete them.


