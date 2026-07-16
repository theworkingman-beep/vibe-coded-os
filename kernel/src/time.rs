//! Architecture-independent timing and timeout helpers.
//!
//! Provides monotonic-cycle-based polling timeouts so device drivers do not
//! spin forever on missing or misbehaving hardware.

/// A simple timeout measured in CPU monotonic cycles.
pub struct Timeout {
    start: u64,
    limit: u64,
}

impl Timeout {
    /// Create a timeout that expires after `milliseconds` (approximate).
    pub fn after_millis(milliseconds: u64) -> Self {
        let start = crate::arch::monotonic_cycles();
        let freq = crate::arch::cycles_per_second();
        let limit = if freq == 0 {
            // No known frequency: fall back to a cycle count that is very
            // unlikely to wrap within a boot session.
            u64::MAX / 2
        } else {
            start + (freq / 1000).saturating_mul(milliseconds)
        };
        Self { start, limit }
    }

    /// Returns `true` once the timeout has expired.
    pub fn expired(&self) -> bool {
        crate::arch::monotonic_cycles().wrapping_sub(self.start) >= self.limit.wrapping_sub(self.start)
    }
}

/// Poll `condition` until it returns `Some(value)` or `timeout_ms` elapse.
///
/// This is a cooperative busy-wait with an upper bound.  It must not be used
/// from contexts where sleeping is required; it is intended for very short
/// device register handshakes (e.g. UART FIFO ready).
pub fn poll_with_timeout<T>(
    timeout_ms: u64,
    mut condition: impl FnMut() -> Option<T>,
) -> Option<T> {
    let timeout = Timeout::after_millis(timeout_ms);
    loop {
        if let Some(value) = condition() {
            return Some(value);
        }
        if timeout.expired() {
            return None;
        }
        // A single pause/yield to reduce bus traffic while polling.
        #[cfg(feature = "arch_x86_64")]
        unsafe {
            core::arch::x86_64::_mm_pause();
        }
        #[cfg(feature = "arch_aarch64")]
        unsafe {
            core::arch::asm!("yield", options(nomem, nostack));
        }
    }
}
