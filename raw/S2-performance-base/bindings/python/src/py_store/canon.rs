// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RDF canonicalization for the `purrdf` Python extension: the
//! `CanonicalizationAlgorithm` pyclass and the `canonicalize_quads` wrapper.
//!
//! All canonicalization runs the **native full W3C RDFC-1.0** engine
//! (`purrdf_core::ir::canon`). The `CanonicalizationAlgorithm` pyclass is retained for Python API
//! compatibility, but both variants resolve to the one native canonicalizer
//! (greenfield: a single canonicalization algorithm).

use pyo3::prelude::*;

use purrdf_core::{CanonError, Canonicalized, FastHasher, FastMap, TermRef, try_canonicalize};

use super::term::quad_to_string;
use crate::{RdfDataset, RdfQuad, RdfTerm, RdfTriple, flat_dataset_from_quads};

/// The graph canonicalization algorithms, exposed to Python as
/// `CanonicalizationAlgorithm`.
#[pyclass(name = "CanonicalizationAlgorithm", eq, eq_int, skip_from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
#[allow(
    clippy::upper_case_acronyms,
    reason = "the variant spellings ARE the Python-visible enum members, so they must not be renamed"
)]
pub(super) enum PyCanonicalizationAlgorithm {
    /// The standard RDF Canonicalization 1.0 algorithm (SHA-256).
    RDFC_1_0,
    /// Retained for API compatibility; now an alias of the native RDFC-1.0 engine.
    UNSTABLE,
}

/// Canonicalize a quad set's blank-node labels under native RDFC-1.0, returning the
/// quads with canonical (`_:c14nN`) blanks, sorted by their N-Quads string. The
/// caller's literal/IRI term forms are preserved exactly (only blanks are relabeled).
/// The `algorithm` selector is retained for API compatibility; both variants map to
/// the one native engine.
///
/// `quads` is wholly caller-supplied (`Dataset.canonicalize()`'s own content), so this
/// goes through the fallible, non-panicking [`try_canonicalize`] rather than
/// `canonicalize`: a reserved-vocabulary or budget-exhausting quad set must come back
/// as an `Err` value across the Python boundary, never abort the process.
///
/// # Errors
/// [`CanonError::ReservedVocabulary`] if any term is an IRI in PurRDF's RDFC-1.0
/// reserved namespace; [`CanonError::BudgetExceeded`] if the n-degree search's
/// call/permutation budget is exhausted first.
pub(super) fn canonicalize_quads(
    quads: &[RdfQuad],
    _algorithm: PyCanonicalizationAlgorithm,
) -> Result<Vec<RdfQuad>, CanonError> {
    let ds = flat_dataset_from_quads(quads).expect("native RDFC-1.0: flat freeze of valid quads");
    let canon = try_canonicalize(&ds)?;
    let map = label_map(&ds, &canon);
    let mut out: Vec<RdfQuad> = quads.iter().map(|q| relabel_quad(q, &map)).collect();
    out.sort_by_key(quad_to_string);
    out.dedup();
    Ok(out)
}

/// Map each original blank-node label to its canonical `c14nN` label.
fn label_map(ds: &RdfDataset, c: &Canonicalized) -> FastMap<String, String> {
    let mut map = FastMap::with_capacity_and_hasher(c.labels.len(), FastHasher::default());
    for (&tid, label) in &c.labels {
        if let TermRef::Blank { label: orig, .. } = ds.resolve(tid) {
            map.insert(orig.to_owned(), label.to_string());
        }
    }
    map
}

fn relabel_quad(quad: &RdfQuad, map: &FastMap<String, String>) -> RdfQuad {
    let mut out = RdfQuad::new(
        relabel_term(&quad.subject, map),
        quad.predicate.clone(),
        relabel_term(&quad.object, map),
    );
    out.graph_name = quad.graph_name.as_ref().map(|g| relabel_term(g, map));
    out
}

/// Rewrite a term, replacing every blank-node label via `map` (recursing triple
/// terms). Canonicalization assigns a label to *every* blank in the dataset, so an
/// unmapped blank is a broken invariant — hard-fail rather than silently passing the
/// original id through (no degraded fallback; `.goals`).
fn relabel_term(term: &RdfTerm, map: &FastMap<String, String>) -> RdfTerm {
    match term {
        RdfTerm::Iri(_) | RdfTerm::Literal(_) => term.clone(),
        RdfTerm::BlankNode(label) => match map.get(label) {
            Some(canon) => RdfTerm::BlankNode(canon.clone()),
            None => unreachable!(
                "RDFC-1.0 labels every blank node; missing canonical label for _:{label}"
            ),
        },
        RdfTerm::Triple(t) => RdfTerm::triple(RdfTriple::new(
            relabel_term(&t.subject, map),
            t.predicate.clone(),
            relabel_term(&t.object, map),
        )),
    }
}

copy_pyclass_from_py_object!(PyCanonicalizationAlgorithm);
