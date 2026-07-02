// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Per-query non-deterministic inputs, read from the host platform: the wall clock
//! `NOW()` returns and the entropy seed for `RAND()`/`UUID()`/`STRUUID()`. The read is
//! written per target (a wasm build has no `SystemTime`/OS entropy syscall) but every
//! target returns the correct live value — there is no caller-visible knob.

use purrdf_xsd::temporal::DateTime;

/// The current wall-clock instant as an `xsd:dateTime` (seconds precision), sampled
/// once per query so all `NOW()` calls in one query agree (SPARQL 1.1 §17.4.5.1).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn wall_clock_now() -> DateTime {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    purrdf_xsd::datetime_from_unix_seconds(secs)
}

/// A fresh 64-bit entropy seed from the OS CSPRNG.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn entropy_seed() -> u64 {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).expect("OS entropy source is available");
    u64::from_le_bytes(bytes)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn wall_clock_now() -> DateTime {
    // js_sys::Date::now() is milliseconds since the Unix epoch.
    #[allow(clippy::cast_possible_truncation)]
    let secs = (js_sys::Date::now() / 1000.0) as i64;
    purrdf_xsd::datetime_from_unix_seconds(secs)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn entropy_seed() -> u64 {
    // Compose 64 bits from two js_sys::Math::random() draws in [0,1).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let draw = || (js_sys::Math::random() * f64::from(u32::MAX)) as u64;
    (draw() << 32) | draw()
}
