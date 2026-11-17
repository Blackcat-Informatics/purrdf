// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RDF canonicalization for the `purrdf` Python extension through
//! the `canonicalize_quads` wrapper and caller-selected search control.
//!
//! All canonicalization runs the default native engine, the nested-refinement
//! labelling (`purrdf_core::ir::canon`), under the caller's explicit search policy.

use purrdf_core::{CanonError, Canonicalized, FastHasher, FastMap, TermRef, try_canonicalize};

use super::term::quad_to_string;
use crate::{RdfDataset, RdfQuad, RdfTerm, RdfTriple, flat_dataset_from_quads};

/// Canonicalize a quad set's blank-node labels under the default labelling, returning the
/// quads with canonical (decimal) blanks, sorted by their N-Quads string. The
/// caller's literal/IRI term forms are preserved exactly (only blanks are relabeled).
/// The caller owns the search policy and cancellation control.
///
/// `quads` is wholly caller-supplied (`Dataset.canonicalize()`'s own content), so this
/// goes through the fallible, non-panicking [`try_canonicalize`] rather than
/// `canonicalize`: a quad set the labelling refuses must come back
/// as an `Err` value across the Python boundary, never abort the process.
///
/// # Errors
/// [`CanonError::Refused`] when the labelling refuses the input, including its
/// `limit` refusal under the caller's finite bound.
pub(super) fn canonicalize_quads(
    quads: &[RdfQuad],
    control: &mut purrdf_core::CanonicalizationControl<'_>,
) -> Result<Vec<RdfQuad>, CanonError> {
    let ds = flat_dataset_from_quads(quads)
        .expect("native canonicalization: flat freeze of valid quads");
    let canon = try_canonicalize(&ds, control)?;
    let map = label_map(&ds, &canon);
    let mut out: Vec<RdfQuad> = quads.iter().map(|q| relabel_quad(q, &map)).collect();
    out.sort_by_key(quad_to_string);
    out.dedup();
    Ok(out)
}

/// Map each original blank-node label to its canonical decimal label.
fn label_map<'a>(ds: &'a RdfDataset, c: &'a Canonicalized) -> FastMap<&'a str, &'a str> {
    let mut map = FastMap::with_capacity_and_hasher(c.label_count(), FastHasher::default());
    for (tid, label) in c.labels() {
        if let TermRef::Blank { label: orig, .. } = ds.resolve(tid) {
            map.insert(orig, label);
        }
    }
    map
}

fn relabel_quad(quad: &RdfQuad, map: &FastMap<&str, &str>) -> RdfQuad {
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
fn relabel_term(term: &RdfTerm, map: &FastMap<&str, &str>) -> RdfTerm {
    match term {
        RdfTerm::Iri(_) | RdfTerm::Literal(_) => term.clone(),
        RdfTerm::BlankNode(label) => match map.get(label.as_str()) {
            Some(canon) => RdfTerm::BlankNode((*canon).to_owned()),
            None => unreachable!(
                "The labelling labels every blank node; missing canonical label for _:{label}"
            ),
        },
        RdfTerm::Triple(t) => RdfTerm::triple(RdfTriple::new(
            relabel_term(&t.subject, map),
            t.predicate.clone(),
            relabel_term(&t.object, map),
        )),
    }
}
