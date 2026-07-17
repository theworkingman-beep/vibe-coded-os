//! Minimal IPC message ports.
//!
//! Provides a simple asynchronous message queue between threads.  This is the
//! precursor to the NT LPC/ALPC port mechanism used internally by the Win32
//! subsystem and by native ApertureOS programs.

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use spin::Mutex;

const MAX_MESSAGE_SIZE: usize = 1024;
const MAX_QUEUE_DEPTH: usize = 16;

/// A one-directional message queue.
pub struct MessagePort {
    queue: Mutex<VecDeque<Vec<u8>>>,
    closed: Mutex<bool>,
}

impl MessagePort {
    /// Create a new empty message port.
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            closed: Mutex::new(false),
        }
    }

    /// Send a message.  Returns `false` if the port is closed or the queue is
    /// full, without blocking.
    pub fn send(&self, message: &[u8]) -> bool {
        if *self.closed.lock() {
            return false;
        }
        if message.len() > MAX_MESSAGE_SIZE {
            return false;
        }
        let mut queue = self.queue.lock();
        if queue.len() >= MAX_QUEUE_DEPTH {
            return false;
        }
        queue.push_back(message.into());
        true
    }

    /// Receive a message, returning it in a freshly allocated Vec.  Returns
    /// `None` if no message is available.  A `Some(Vec::new())` indicates the
    /// port has been closed and no further messages will arrive.
    pub fn try_receive(&self) -> Option<Vec<u8>> {
        let mut queue = self.queue.lock();
        queue.pop_front().map(|msg| {
            if msg.is_empty() {
                // Empty sentinel marks a closed port.
            }
            msg
        })
    }

    /// Close the port.  After closing, sends fail and a zero-length sentinel
    /// message is appended so receivers can detect end-of-stream.
    pub fn close(&self) {
        let mut closed = self.closed.lock();
        if !*closed {
            *closed = true;
            let mut queue = self.queue.lock();
            queue.push_back(Vec::new());
        }
    }

    /// Returns true if the port has been closed.
    pub fn is_closed(&self) -> bool {
        *self.closed.lock()
    }
}

/// A pair of unidirectional ports forming a bidirectional channel.
pub struct Channel {
    pub a_to_b: MessagePort,
    pub b_to_a: MessagePort,
}

impl Channel {
    /// Create a new bidirectional channel.
    pub fn new() -> Self {
        Self {
            a_to_b: MessagePort::new(),
            b_to_a: MessagePort::new(),
        }
    }
}

impl Default for MessagePort {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Channel {
    fn default() -> Self {
        Self::new()
    }
}

/// Boot-time self-test for the IPC message-port path.
///
/// Exercises a real send/receive round-trip on a bidirectional channel and
/// verifies the payload survives intact, plus the closed-port send-rejects
/// behavior. Returns `true` on success. Called from the Win32 phase
/// self-tests; never blocks.
pub fn self_test() -> bool {
    let chan = Channel::new();
    let payload: &[u8] = b"aperture-ipc-ping";
    if !chan.a_to_b.send(payload) {
        crate::logln!("port: self_test FAIL send rejected");
        return false;
    }
    let received = match chan.a_to_b.try_receive() {
        Some(m) => m,
        None => {
            crate::logln!("port: self_test FAIL no message");
            return false;
        }
    };
    if received.as_slice() != payload {
        crate::logln!("port: self_test FAIL payload mismatch");
        return false;
    }
    chan.a_to_b.close();
    if chan.a_to_b.send(b"after-close") {
        crate::logln!("port: self_test FAIL send after close accepted");
        return false;
    }
    crate::logln!("port: self_test OK ({}-byte round-trip)", payload.len());
    true
}
