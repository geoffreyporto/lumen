use std::panic;
use std::ptr;

use liblumen_alloc::erts::message::MessageType;
use liblumen_alloc::erts::term::prelude::*;
use liblumen_alloc::erts::timeout::{ReceiveTimeout, Timeout};

use lumen_rt_core::process::current_process;
use lumen_rt_core::time::monotonic;

extern "C" {
    #[link_name = "__lumen_builtin_yield"]
    fn builtin_yield() -> bool;
}

#[repr(u8)]
pub enum ReceiveState {
    // Indicates to the caller that an unrecoverable error occurred
    Error = 0,
    // Used to indicate initialized state, prior to first attempt to receive
    Ready = 1,
    // Indicates to the caller that a message was received
    Received = 2,
    // Indicates to the caller that the receive timed out
    Timeout = 3,
}

/// This structure manages the context for a single receive operation,
/// it is created by `receive_start`, modified during `receive_wait`,
/// and used during cleanup in `receive_done` to determine what, if any,
/// cleanup needs to be performed.
///
/// It is read from non-Rust code, so we use `repr(C)`.
#[repr(C)]
pub struct ReceiveContext {
    timeout: ReceiveTimeout,
    message: Term,
    message_needs_move: bool,
    state: ReceiveState,
}
impl ReceiveContext {
    #[inline]
    fn new(timeout: Timeout) -> Self {
        let now = monotonic::time();
        let timeout = ReceiveTimeout::new(now, timeout);
        Self {
            state: ReceiveState::Ready,
            message_needs_move: false,
            message: Term::NONE,
            timeout,
        }
    }

    #[allow(dead_code)]
    #[inline]
    fn failed() -> Self {
        Self {
            state: ReceiveState::Error,
            message_needs_move: false,
            message: Term::NONE,
            timeout: Default::default(),
        }
    }

    #[inline]
    fn with_message(&mut self, message: Term, message_type: MessageType) {
        self.state = ReceiveState::Received;
        self.message = message;
        if message_type == MessageType::HeapFragment {
            self.message_needs_move = true;
        }
    }

    #[inline]
    fn with_timeout(&mut self) {
        self.state = ReceiveState::Timeout;
        self.message = Term::NONE;
        self.message_needs_move = false;
    }

    #[inline]
    fn should_time_out(&self) -> bool {
        let now = monotonic::time();
        self.timeout.is_timed_out(now)
    }
}

#[export_name = "__lumen_builtin_receive_start"]
pub extern "C" fn builtin_receive_start(timeout: Term) -> *mut ReceiveContext {
    let result = panic::catch_unwind(move || {
        let to = match timeout.decode().unwrap() {
            TypedTerm::Atom(atom) if atom == "infinity" => Timeout::Infinity,
            TypedTerm::SmallInteger(si) => Timeout::from_millis(si).expect("invalid timeout value"),
            _ => unreachable!("should never get non-atom/non-integer receive timeout"),
        };
        // TODO: It would be best if ReceiveContext was repr(C) so we
        // could keep it on the stack rather than heap allocate here
        let context = Box::new(ReceiveContext::new(to));
        let p = current_process();
        let mbox = p.mailbox.lock();
        mbox.borrow().recv_start();
        Box::into_raw(context)
    });
    if let Ok(res) = result {
        res
    } else {
        ptr::null_mut()
    }
}

#[export_name = "__lumen_builtin_receive_wait"]
pub extern "C" fn builtin_receive_wait(ctx: *mut ReceiveContext) -> ReceiveState {
    let result = panic::catch_unwind(move || {
        let context = unsafe { &mut *ctx };
        loop {
            {
                let p = current_process();
                let mbox_lock = p.mailbox.lock();
                let mut mbox = mbox_lock.borrow_mut();
                if let Some((msg, msg_type)) = mbox.recv_peek_with_type() {
                    mbox.recv_increment();
                    context.with_message(msg, msg_type);
                    break ReceiveState::Received;
                } else if context.should_time_out() {
                    context.with_timeout();
                    break ReceiveState::Timeout;
                } else {
                    // If there are no messages, wait and yield
                    p.wait();
                }
            }
            // We put our yield here to ensure that we're not holding
            // the mailbox lock while waiting, when resuming from the
            // yield, we'll continue looping
            unsafe {
                builtin_yield();
            }
        }
    });
    if let Ok(res) = result {
        res
    } else {
        ReceiveState::Error
    }
}

#[export_name = "__lumen_builtin_receive_message"]
pub extern "C" fn builtin_receive_message(ctx: *mut ReceiveContext) -> Term {
    let context = unsafe { &*ctx };
    context.message
}

#[export_name = "__lumen_builtin_receive_done"]
pub extern "C" fn builtin_receive_done(ctx: *mut ReceiveContext) -> bool {
    let context = unsafe { Box::from_raw(ctx) };
    let result = panic::catch_unwind(|| {
        let p = current_process();
        let mbox_lock = p.mailbox.lock();
        let mut mbox = mbox_lock.borrow_mut();

        match context.state {
            ReceiveState::Received if context.message_needs_move => {
                // Copy to process heap
                unimplemented!();
            }
            ReceiveState::Received | ReceiveState::Timeout => {
                mbox.recv_finish(&p);
                true
            }
            _ => {
                unreachable!();
            }
        }
    });
    result.is_ok()
}
