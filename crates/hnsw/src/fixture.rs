// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Index parameters the HNSW tests of this crate and of the facade build under.
//!
//! This is test support, not API: it is hidden from the documentation and carries no
//! stability promise. It lives in the library because a test target can share code
//! with another crate's tests only through a library both reach.

use crate::params::Params;

/// The modest parameter set most suites build under, `Params::new(4, 8, 16, 8)`.
///
/// # Panics
///
/// Never: the four values are valid parameters.
#[must_use]
pub fn params() -> Params {
    Params::new(4, 8, 16, 8).expect("the fixture parameters are valid")
}
