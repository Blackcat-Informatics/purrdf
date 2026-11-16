// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The RDFC-1.0 family under its own name.
//!
//! RDFC-1.0 is a standard algorithm with its own conformance suite, and it is
//! the algorithm that produced every canonical form and digest recorded before
//! the nested-refinement labelling became the canonical output. Both uses
//! remain: a caller runs the W3C algorithm by name, and a reader recomputes a
//! recorded digest of an older profile (the overlay and flat presentations of
//! the profile [`CANON_PROFILE_ID`], at [`CANON_PROFILE_VERSION`]) with the
//! function that wrote it.
//!
//! The functions here behave exactly as they always did: blank nodes are
//! labelled `c14nN`, the hash is SHA-256 or SHA-384 as the caller selects, and
//! a dataset that holds the profile's reserved vocabulary is refused. Nothing
//! on this path reads or writes the canonical labelling of
//! [`crate::canonical_labelling`].

pub use crate::ir::canon::{
    CANON_CORPUS_DIGEST, CANON_PRESENTATION_FLAT_ASSERTION_ID,
    CANON_PRESENTATION_FLAT_ASSERTION_VERSION, CANON_PRESENTATION_OVERLAY_ID,
    CANON_PRESENTATION_OVERLAY_VERSION, CANON_PROFILE_ID, CANON_PROFILE_VERSION, CanonHash,
    CanonPresentation, RDFC_V2_CORPUS_POLICY, RESERVED_NAMESPACE, RdfcActiveGroup, RdfcActivePath,
    RdfcCandidateProgress, RdfcGroupLabels, RdfcGroupObservation, RdfcGroupObservations,
    RdfcGroupStats, RdfcRefusalObservation, RdfcRootProgress, RdfcSearchContext,
    ReservedVocabulary, canonical_relabel, canonical_relabel_with_mapping, check_admissible,
    check_admissible_flat_view, check_admissible_view, try_canonical_blake3_view,
    try_canonical_sha256_view, try_canonicalize, try_canonicalize_flat_graph_view,
    try_canonicalize_flat_view, try_canonicalize_graph_view, try_canonicalize_sha256_view,
    try_canonicalize_view, try_canonicalize_with, try_flat_blake3_view, try_flat_digest_view,
    try_graph_digest_view,
};
