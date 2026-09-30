// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixture helpers the retrieval tests and benches share with the facade's.
//!
//! Not part of the API: `#[doc(hidden)]`, like `purrdf_core::term_fixture`. They
//! live in the library because an integration test of `purrdf` cannot include a
//! file of this package, and a helper each package retyped would be a second copy
//! of the same fixture.

use crate::{DomainTag, Fixed, FusedRow, Iri, Term};

/// A fixture IRI, which is valid by construction.
///
/// # Panics
///
/// When `text` is not an absolute IRI.
#[must_use]
pub fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

/// The one block domain the multimodal fixtures' producers share,
/// `http://example.org/domain/shared`.
///
/// # Panics
///
/// Never: the IRI is a valid domain tag.
#[must_use]
pub fn shared_block() -> DomainTag {
    DomainTag::parse("http://example.org/domain/shared")
        .expect("the fixture domain tag is a valid IRI")
}

/// A fused row reduced to what a comparison between two runs is about: the
/// entity, its score and every stratum's `(stratum, rank, contribution)`.
#[must_use]
pub fn reduce(row: &FusedRow) -> (Term, Fixed, Vec<(Iri, u64, Fixed)>) {
    (
        row.entity.clone(),
        row.score,
        row.contributions
            .iter()
            .map(|(stratum, rank, contribution)| (stratum.clone(), *rank, *contribution))
            .collect(),
    )
}
