// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Per-query non-deterministic inputs, read from the host: the wall clock `NOW()`
//! returns and the entropy seed for `RAND()`/`UUID()`/`STRUUID()`. Query options can
//! select these inputs explicitly before their corresponding host reads are sampled.
//!
//! Each read asks the installed [`crate::host_source`] first. With none installed a
//! native build reads the operating system; a wasm build has no `SystemTime` or OS
//! entropy syscall and imports nothing, so there the read is refused with
//! [`HostSourceMissing`] rather than answered with a value nobody sampled.
//!
//! These are two of the crate's three host reads. The third is the governor
//! path's deadline clock, which is confined to
//! [`crate::governor::WallDeadline`] and split exactly as
//! [`wall_clock_now`] is here. It is deliberately not routed through this module: a
//! deadline needs elapsed time against a snapshotted start, not an `xsd:dateTime`, and
//! the evaluator core itself never reads a clock — every governor check consults a
//! [`StopSignal`](crate::governor::StopSignal), which is a `WallDeadline` only when the
//! caller did not supply one of their own. Those two sites are the whole set; do not
//! open a third.

use crate::host_source::{self, HostSourceMissing};
use purrdf_xsd::temporal::DateTime;

/// The current wall-clock instant as an `xsd:dateTime` (seconds precision), sampled
/// once per query so all `NOW()` calls in one query agree (SPARQL 1.1 §17.4.5.1).
pub(crate) fn wall_clock_now() -> Result<DateTime, HostSourceMissing> {
    let seconds = match host_source::installed() {
        Some(source) => unix_seconds_from_millis(source.unix_millis()),
        None => os_unix_seconds()?,
    };
    Ok(purrdf_xsd::datetime_from_unix_seconds(seconds))
}

/// A fresh 64-bit entropy seed for one execution's random stream.
pub(crate) fn entropy_seed() -> Result<u64, HostSourceMissing> {
    host_source::installed().map_or_else(os_entropy_seed, |source| Ok(source.entropy_seed()))
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(
    clippy::unnecessary_wraps,
    reason = "the read has one shape on every target, and the wasm32 one refuses"
)]
fn os_unix_seconds() -> Result<i64, HostSourceMissing> {
    Ok(unix_seconds(std::time::SystemTime::now()))
}

#[cfg(target_arch = "wasm32")]
const fn os_unix_seconds() -> Result<i64, HostSourceMissing> {
    Err(HostSourceMissing::new(host_source::HostInput::Clock))
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
#[allow(
    clippy::unnecessary_wraps,
    reason = "the read has one shape on every target, and the wasm32 one refuses"
)]
fn os_entropy_seed() -> Result<u64, HostSourceMissing> {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).expect("OS entropy source is available");
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(target_arch = "wasm32")]
const fn os_entropy_seed() -> Result<u64, HostSourceMissing> {
    Err(HostSourceMissing::new(host_source::HostInput::Entropy))
}

// Floor establishes whole seconds; the cast preserves the existing saturating
// i64 range law for the host's floating-point millisecond representation.
#[allow(clippy::cast_possible_truncation)]
fn unix_seconds_from_millis(milliseconds: f64) -> i64 {
    (milliseconds / 1000.0).floor() as i64
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
