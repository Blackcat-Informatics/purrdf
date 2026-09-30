// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The genuine shapes product a product test starts from.
//!
//! This is test support, not API: it is hidden from the documentation and carries no
//! stability promise. It lives in the library because this crate's product tests and
//! the command-line crate's `shacl` tests both need the product a document packs to,
//! and a test target can share code with another crate's tests only through a library
//! both reach.

use std::sync::Arc;

use crate::engine::{PreparedShapes, parse_shapes};
use crate::product::ShapesProfile;

/// `shapes_ttl` parsed under no base, prepared and packed under
/// [`ShapesProfile::CORE`]: the product `shacl pack` writes for a document with no
/// derivable base.
///
/// # Panics
///
/// If the document does not parse or the product format cannot carry it.
#[must_use]
pub fn core_product(shapes_ttl: &str) -> Vec<u8> {
    let shapes = parse_shapes(shapes_ttl, None).expect("the fixture shapes parse");
    PreparedShapes::new(Arc::new(shapes))
        .to_product(&ShapesProfile::CORE)
        .expect("the fixture packs as a product under this build")
}
