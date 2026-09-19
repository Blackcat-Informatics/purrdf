// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The allocator counters, and the per-leg metering built on them.
//!
//! The counting allocator itself must live in the binary — `#[global_allocator]`
//! is a program-level choice, not a library's to make — but the COUNTERS live here
//! so a workload can meter a phase of its own work rather than only reporting one
//! number for everything it did. That is what lets a workload compare two ways of
//! doing the same thing at the same scale, which is the only form of evidence that
//! attributes a change to the thing that was changed.
//!
//! These are byte counts, not timings. They do not vary with machine load, so
//! unlike a wall-clock figure they can legitimately be asserted on.

use core::sync::atomic::{AtomicI64, Ordering};

static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static PEAK_BYTES: AtomicI64 = AtomicI64::new(0);

/// Record an allocation of `size` bytes, updating the high-water mark.
///
/// Called from the binary's `GlobalAlloc`.
pub fn record_allocation(size: usize) {
    let size = i64::try_from(size).unwrap_or(i64::MAX);
    let live = LIVE_BYTES.fetch_add(size, Ordering::Relaxed) + size;
    let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
    while live > peak {
        match PEAK_BYTES.compare_exchange_weak(peak, live, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(current) => peak = current,
        }
    }
}

/// Record a deallocation of `size` bytes.
pub fn record_deallocation(size: usize) {
    let size = i64::try_from(size).unwrap_or(i64::MAX);
    LIVE_BYTES.fetch_sub(size, Ordering::Relaxed);
}

/// Reset the high-water mark to the current live figure, returning that baseline.
pub fn reset_peak() -> i64 {
    let live = LIVE_BYTES.load(Ordering::Relaxed);
    PEAK_BYTES.store(live, Ordering::Relaxed);
    live
}

/// The current high-water mark.
#[must_use]
pub fn peak_bytes() -> i64 {
    PEAK_BYTES.load(Ordering::Relaxed)
}

/// The currently live byte count.
#[must_use]
pub fn live_bytes() -> i64 {
    LIVE_BYTES.load(Ordering::Relaxed)
}

/// Run `leg`, returning its result and the bytes it peaked at ABOVE whatever was
/// already live when it started.
///
/// The baseline subtraction is what makes two legs comparable: both run with the
/// same dataset resident, so what is measured is the additional residency each way
/// of working requires, not the residency they share.
pub fn metered<T>(leg: impl FnOnce() -> T) -> (T, u64) {
    // The enclosing workload is measuring its own peak across everything it does,
    // and this resets the mark. Restoring the higher of the two afterwards is what
    // keeps that outer measurement honest — without it, a workload's reported peak
    // silently becomes "whatever happened after the last leg", which reads as a
    // large improvement produced entirely by the act of measuring.
    let prior_peak = peak_bytes();
    let baseline = reset_peak();
    let value = leg();
    let leg_peak = peak_bytes();
    PEAK_BYTES.store(leg_peak.max(prior_peak), Ordering::Relaxed);
    let peak = leg_peak.saturating_sub(baseline).max(0);
    (value, u64::try_from(peak).unwrap_or(0))
}
