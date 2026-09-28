// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! JSON numbers retain their decimal representation through the workspace's
//! arbitrary-precision serde configuration. Strict readers frame containers
//! themselves and decode typed scalars, preserving ordinary object keys and
//! counting each number once against the caller's value budget.
//!
//! Reads into binary64 use serde's correctly rounded `float_roundtrip` path.
//! Its arithmetic fast path must round once even on x87: `Binary64Scope` sets
//! precision control to 53 bits for the read; elsewhere the scope is empty.
//! Keeping the scope around a complete read avoids per-number control-word loads.

use purrdf_xsd::ieee::Binary64Scope;

pub(crate) fn parse_strict(
    bytes: &[u8],
    values: usize,
    depth: usize,
) -> Result<serde_json::Value, serde_json::Error> {
    crate::json_value::parse(
        bytes,
        crate::json_value::Limits {
            bytes: usize::MAX,
            values,
            depth,
            string_bytes: usize::MAX,
        },
    )
    .map_err(<serde_json::Error as serde::de::Error>::custom)
}

/// Run a `serde_json` read whose numbers reach a literal, serialized bytes or an identity,
/// with every binary64 operation inside it rounded once.
#[inline]
pub(crate) fn read_json<T>(read: impl FnOnce() -> T) -> T {
    let _binary64 = Binary64Scope::enter();
    read()
}
