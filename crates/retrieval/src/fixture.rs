// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixture helpers the retrieval tests and benches share with the facade's.
//!
//! Not part of the API: `#[doc(hidden)]`, like `purrdf_core::term_fixture`. They
//! live in the library because an integration test of `purrdf` cannot include a
//! file of this package, and a helper each package retyped would be a second copy
//! of the same fixture.

use crate::{DomainTag, FusedRow, Iri, Score, Term};

/// Explicit decimal units of the original frozen fusion fixtures.
pub const PRECISION_12: crate::Precision = crate::Precision::FROZEN_12;

/// A fixture IRI, which is valid by construction.
///
/// # Panics
///
/// When `text` is not an absolute IRI.
#[must_use]
pub fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

/// Immutable fixture membership through the same storage as a returned score.
/// # Panics
/// When the resident allocator cannot acquire the fixture's storage.
#[must_use]
pub fn iri_members(
    values: impl IntoIterator<Item = Iri>,
) -> purrdf_sparql_eval::SharedWorkspace<purrdf_sparql_eval::AdmittedOrderedSet<Iri>> {
    let workspace = purrdf_sparql_eval::WorkspaceCapability::resident();
    let members = purrdf_sparql_eval::AdmittedOrderedSet::from_iter_admitted(values, &workspace)
        .expect("resident fixture membership storage");
    purrdf_sparql_eval::SharedWorkspace::new_admitted(members, &workspace)
        .expect("resident fixture membership owner")
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
pub fn reduce(row: &FusedRow) -> (Term, Score, Vec<(Iri, u64, Score)>) {
    (
        row.entity.clone(),
        row.score,
        row.contributions
            .iter()
            .map(|(stratum, rank, contribution)| (stratum.clone(), *rank, *contribution))
            .collect(),
    )
}
