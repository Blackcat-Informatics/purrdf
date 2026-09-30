// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE definition of the GTS test fixtures this crate's tests and the
//! `purrdf-rdf` GTS tests share.
//!
//! A `tests/` target of this crate and a `tests/` target of `purrdf-rdf` are
//! separate crates that can only share code through a library, and `purrdf-rdf`
//! depends on this crate, so the shared fixtures live here. It is `#[doc(hidden)]`
//! at the crate root: shipped, but not public API, part of no stability promise,
//! and called by no shipping code path (the `purrdf_rdf::gts_fixtures` precedent).

use purrdf_ed25519::SigningKey;

use crate::dict::raw_content_dict;
use crate::model::Graph;

/// A fixed, deterministic Ed25519 signing key whose 32 secret bytes are all
/// `byte` (RFC 8032 signing is deterministic per key and message, so tests stay
/// byte-reproducible).
#[must_use]
pub fn fixed_key(byte: u8) -> SigningKey {
    SigningKey::from_bytes(&[byte; 32])
}

/// The lexical value of the object of the first quad of `graph` whose predicate
/// is the IRI `predicate_iri`.
#[must_use]
pub fn object_literal(graph: &Graph, predicate_iri: &str) -> Option<String> {
    let p = graph
        .terms
        .iter()
        .position(|t| t.value.as_deref() == Some(predicate_iri))?;
    graph
        .quads
        .iter()
        .find(|&&(_, pred, _, _)| pred == p)
        .and_then(|&(_, _, o, _)| graph.terms[o].value.clone())
}

/// A caller's SHIPPED raw-content dictionary: derived from a vocabulary that has
/// nothing to do with any pack a test compacts, so "the pack pinned my bytes"
/// cannot pass by accidentally re-deriving the same dictionary.
///
/// # Panics
///
/// Never for this fixed corpus: the dictionary builder refuses only an empty one.
#[must_use]
pub fn shipped_dictionary() -> Vec<u8> {
    let corpus: Vec<Vec<u8>> = (0..400u32)
        .map(|i| {
            format!(
                "<https://example.org/slice/logic#c{}> <https://example.org/p/grounds> \
                 \"a shipped-vocabulary sentence unrelated to any packed content, {}\" .\n",
                i % 23,
                i
            )
            .into_bytes()
        })
        .collect();
    let refs: Vec<&[u8]> = corpus.iter().map(Vec::as_slice).collect();
    raw_content_dict(&refs, 8192).expect("the shipped dictionary builds")
}
