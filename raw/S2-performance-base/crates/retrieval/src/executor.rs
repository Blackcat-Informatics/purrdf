// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one-thread executor a synchronous caller drives the ladder with.

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

/// Drive one future to completion on the calling thread.
///
/// The ladder's futures ([`crate::search`], [`crate::execute`], a fused
/// stream's `next`) are awaited in a single task and never cross a thread
/// boundary, so a waker that unparks the calling thread is the whole runtime
/// they need. A host without an async runtime of its own — a language binding,
/// a benchmark, a test — drives the real async entry points through this
/// rather than asking for a second, synchronous surface.
///
/// A future that returns [`Poll::Pending`] parks the thread until its waker
/// fires; one that never wakes it blocks forever, exactly as it would under any
/// executor.
pub fn block_on<F: Future>(future: F) -> F::Output {
    struct ParkWaker(std::thread::Thread);
    impl Wake for ParkWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(ParkWaker(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}
