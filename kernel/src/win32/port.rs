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
