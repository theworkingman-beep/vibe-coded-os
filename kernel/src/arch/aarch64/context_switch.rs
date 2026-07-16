//! AArch64 cooperative context switch.
//!
//! Saves the callee-saved registers (x19-x29) and the link register on the
//! current kernel stack, swaps SP_EL1 to the new stack, and restores the new
//! context.  New threads receive an artificial initial frame that returns into
//! [`thread_entry_stub`].

const STACK_SIZE: usize = 64 * 1024; // 64 KiB kernel stacks

/// Return the default kernel thread stack size.
pub const fn stack_size() -> usize {
    STACK_SIZE
}

/// Architecture-defined entry stub placed on every new thread stack.
///
/// Called once when a thread runs for the first time; it then calls the
/// architecture-independent [`crate::win32::scheduler::thread_exit`] if the
/// thread body ever returns.
#[no_mangle]
unsafe extern "C" fn thread_entry_stub() {
    crate::win32::scheduler::thread_exit();
}

/// Switch from the current kernel stack to `new_rsp`, saving the current
/// callee-saved state to `*old_rsp`.
///
/// # Safety
/// Must be called with interrupts masked.  `old_rsp` must point to a valid
/// writable `u64` and `new_rsp` must be the top of an initialized AArch64
/// kernel stack.
pub unsafe extern "C" fn switch(old_rsp: *mut u64, new_rsp: u64) {
    core::arch::asm!(
        // Save callee-saved registers and LR on the outgoing stack.
        "stp x19, x20, [sp, #-16]!",
        "stp x21, x22, [sp, #-16]!",
        "stp x23, x24, [sp, #-16]!",
        "stp x25, x26, [sp, #-16]!",
        "stp x27, x28, [sp, #-16]!",
        "stp x29, x30, [sp, #-16]!",
        // Record outgoing SP for later resume.
        "mov x19, sp",
        "str x19, [{old_rsp}]",
        // Load the incoming stack pointer and restore its context.
        "mov sp, {new_rsp}",
        "ldp x29, x30, [sp], #16",
        "ldp x27, x28, [sp], #16",
        "ldp x25, x26, [sp], #16",
        "ldp x23, x24, [sp], #16",
        "ldp x21, x22, [sp], #16",
        "ldp x19, x20, [sp], #16",
        "ret",
        old_rsp = in(reg) old_rsp,
        new_rsp = in(reg) new_rsp,
        options(nomem, nostack)
    );
}

/// Prepare a brand-new AArch64 kernel stack so that the first context switch
/// to it returns into [`thread_entry_stub`], which then runs the thread entry.
///
/// `entry_point` is ignored for the pure stack-switch path; the scheduler
/// stores the real entry point in the `Thread` structure and the stub will
/// dispatch to it.  We still accept the argument to keep the HAL signature
/// symmetric with the x86_64 implementation.
pub fn initial_stack(_entry_point: u64, stack_top: u64) -> u64 {
    // AArch64 stack grows downward.  Push a single frame containing the
    // address of the entry stub as the link register; `switch` will pop this
    // and "return" into the stub.
    let top = stack_top as *mut u64;
    unsafe {
        top.offset(-1).write(thread_entry_stub as *const () as u64);
    }
    stack_top - 8
}
