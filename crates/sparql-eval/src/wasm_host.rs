// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The wasm32 host clock and random draw, as this module has always offered them.
//!
//! This crate imports nothing from a host. Both functions read the source the embedder
//! installed through [`crate::host_source`]; a browser binding installs one that reads
//! `Date.now` and `Math.random`. They are kept, with their signatures, for callers of
//! earlier releases. [`crate::host_source::installed`] is the read that can answer
//! "there is no source" as a value.

use crate::host_source::{self, HostInput, HostSource, HostSourceMissing};

/// The installed source, or the refusal a signature without an error channel can only
/// raise.
fn source(input: HostInput) -> &'static dyn HostSource {
    host_source::installed().unwrap_or_else(|| panic!("{}", HostSourceMissing::new(input)))
}

/// Unix epoch milliseconds from the installed host source.
///
/// # Panics
///
/// When no source is installed: there is no clock to read and no value to stand in
/// for one.
#[inline]
#[must_use]
pub fn date_now() -> f64 {
    source(HostInput::Clock).unix_millis()
}

/// One pseudorandom draw in `[0, 1)`, from the installed host source's entropy.
///
/// # Panics
///
/// When no source is installed: there is no entropy to read and no value to stand in
/// for it.
#[inline]
#[must_use]
pub fn math_random() -> f64 {
    // The top 53 bits are exactly representable, so the quotient is in `[0, 1)`.
    #[allow(clippy::cast_precision_loss)]
    let draw = (source(HostInput::Entropy).entropy_seed() >> 11) as f64;
    draw / 9_007_199_254_740_992.0
}
