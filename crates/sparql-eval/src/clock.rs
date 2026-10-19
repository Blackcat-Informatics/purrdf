// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Per-query non-deterministic inputs, read from the host platform: the wall clock
//! `NOW()` returns and the entropy seed for `RAND()`/`UUID()`/`STRUUID()`. The read is
//! written per target (a wasm build has no `SystemTime`/OS entropy syscall) but every
//! target returns the correct live value — there is no caller-visible knob.
//!
//! These are two of the crate's three host-platform reads. The third is the governor
//! path's deadline clock, which is confined to
//! [`crate::governor::WallDeadline`] and split per target exactly as
//! [`wall_clock_now`] is here. It is deliberately not routed through this module: a
//! deadline needs elapsed time against a snapshotted start, not an `xsd:dateTime`, and
//! the evaluator core itself never reads a clock — every governor check consults a
//! [`StopSignal`](crate::governor::StopSignal), which is a `WallDeadline` only when the
//! caller did not supply one of their own. Those two sites are the whole set; do not
//! open a third.

use purrdf_xsd::temporal::DateTime;

/// The current wall-clock instant as an `xsd:dateTime` (seconds precision), sampled
/// once per query so all `NOW()` calls in one query agree (SPARQL 1.1 §17.4.5.1).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn wall_clock_now() -> DateTime {
    purrdf_xsd::datetime_from_unix_seconds(unix_seconds(std::time::SystemTime::now()))
}

#[cfg(not(target_arch = "wasm32"))]
fn unix_seconds(time: std::time::SystemTime) -> i64 {
    use std::time::UNIX_EPOCH;
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
        Err(before_epoch) => {
            let duration = before_epoch.duration();
            // Whole seconds are rounded down on both sides of the epoch. A
            // negative fractional second belongs to the preceding whole second.
            let seconds =
                -i128::from(duration.as_secs()) - i128::from(duration.subsec_nanos() != 0);
            i64::try_from(seconds).unwrap_or(i64::MIN)
        }
    }
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
    purrdf_xsd::datetime_from_unix_seconds(unix_seconds_from_millis(crate::wasm_host::date_now()))
}

#[cfg(any(test, target_arch = "wasm32"))]
// Floor establishes whole seconds; the cast preserves the existing saturating
// i64 range law for the host's floating-point millisecond representation.
#[allow(clippy::cast_possible_truncation)]
fn unix_seconds_from_millis(milliseconds: f64) -> i64 {
    (milliseconds / 1000.0).floor() as i64
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn entropy_seed() -> u64 {
    // Compose 64 bits from two host Math.random() draws in [0,1).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let draw = || (crate::wasm_host::math_random() * f64::from(u32::MAX)) as u64;
    (draw() << 32) | draw()
}

#[cfg(test)]
mod tests {
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_clock_preserves_instants_before_the_epoch() {
        use std::time::{Duration, UNIX_EPOCH};
        for (duration, expected) in [
            (Duration::from_nanos(1), -1),
            (Duration::from_millis(999), -1),
            (Duration::from_secs(1), -1),
            (Duration::new(1, 1), -2),
        ] {
            assert_eq!(super::unix_seconds(UNIX_EPOCH - duration), expected);
        }
        assert_eq!(super::unix_seconds(UNIX_EPOCH), 0);
        assert_eq!(
            super::unix_seconds(UNIX_EPOCH + Duration::from_millis(999)),
            0
        );
        assert_eq!(super::unix_seconds(UNIX_EPOCH + Duration::from_secs(1)), 1);
        assert_eq!(
            purrdf_xsd::datetime_from_unix_seconds(super::unix_seconds(
                UNIX_EPOCH - Duration::from_nanos(1)
            ))
            .canonical_lexical(),
            "1969-12-31T23:59:59Z"
        );
    }

    #[test]
    fn host_milliseconds_round_down_at_the_epoch_boundary() {
        for (milliseconds, expected) in [
            (-1500.0, -2),
            (-1000.0, -1),
            (-1.0, -1),
            (0.0, 0),
            (1.0, 0),
            (999.0, 0),
            (1000.0, 1),
            (1500.0, 1),
        ] {
            assert_eq!(super::unix_seconds_from_millis(milliseconds), expected);
        }
        assert_eq!(
            purrdf_xsd::datetime_from_unix_seconds(super::unix_seconds_from_millis(-1500.0))
                .canonical_lexical(),
            "1969-12-31T23:59:58Z"
        );
    }
}
