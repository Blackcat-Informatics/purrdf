// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Parsed manifest metadata. Filesystem/network closure acquisition is a host job.

use std::collections::BTreeSet;

use purrdf_core::{TermId, TermValue};
use purrdf_iri::vocab::mf;

use crate::GradeError;
use crate::graph::Reader;

/// One document's manifest entries and include targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Entries in collection order, with a repeated case identity counted once.
    pub entries: Vec<TermId>,
    /// Include IRIs in lexical order; repetitions remain visible to closure guards.
    pub includes: Vec<String>,
}

/// Decode manifest lists and includes from the selected RDF graph.
///
/// Direct-IRI and RDF-list include spellings are accepted. A malformed or empty
/// entries relation remains distinguishable from an aggregator with no entries.
///
/// # Errors
/// Refuses malformed collections and non-resource entries.
pub fn decode(reader: Reader<'_>) -> Result<Document, GradeError> {
    let mut entries = Vec::new();
    let mut seen_entries = BTreeSet::new();
    let mut includes = Vec::new();
    for quad in reader.dataset.quads() {
        if !reader.graph.matches(quad.g) {
            continue;
        }
        match reader.dataset.term_value(quad.p) {
            TermValue::Iri(predicate) if predicate == mf::ENTRIES => {
                for entry in reader.list(quad.o)? {
                    if !matches!(
                        reader.dataset.term_value(entry),
                        TermValue::Iri(_) | TermValue::Blank { .. }
                    ) {
                        return Err(GradeError::Malformed(
                            "manifest entry is not a resource".to_owned(),
                        ));
                    }
                    if seen_entries.insert(entry) {
                        entries.push(entry);
                    }
                }
            }
            TermValue::Iri(predicate) if predicate == mf::INCLUDE => {
                let members = if matches!(reader.dataset.term_value(quad.o), TermValue::Iri(ref iri) if iri != purrdf_iri::vocab::rdf::NIL)
                {
                    vec![quad.o]
                } else {
                    reader.list(quad.o)?
                };
                for member in members {
                    includes.push(reader.iri(member)?);
                }
            }
            _ => {}
        }
    }
    includes.sort();
    Ok(Document { entries, includes })
}

/// Decode an inline inference expectation as a collection of three-member RDF
/// collections. Every term retains its native source identity and scope.
///
/// # Errors
/// Refuses malformed lists and triples of any other width.
pub fn inference_triples(
    reader: Reader<'_>,
    root: TermId,
) -> Result<Vec<[TermValue; 3]>, GradeError> {
    reader
        .list(root)?
        .into_iter()
        .map(|row| {
            let terms: Vec<_> = reader
                .list(row)?
                .into_iter()
                .map(|term| reader.dataset.term_value(term))
                .collect();
            terms.try_into().map_err(|_| {
                GradeError::Malformed(
                    "an expected inference triple must have exactly three terms".to_owned(),
                )
            })
        })
        .collect()
}
