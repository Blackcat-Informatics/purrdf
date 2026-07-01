// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RDF canonicalization for the `purrdf` Python extension: the
//! `CanonicalizationAlgorithm` pyclass and the `canonicalize_quads` wrapper.
//!
//! All canonicalization runs the **native full W3C RDFC-1.0** engine
//! (`purrdf_core::ir::canon`); there is no oxigraph on this path (#910 / EPIC
//! #906). The `CanonicalizationAlgorithm` pyclass is retained for Python API
//! compatibility, but both variants resolve to the one native canonicalizer
//! (greenfield: a single canonicalization algorithm).

use std::collections::HashMap;

use pyo3::prelude::*;

use purrdf_core::{canonicalize as core_canonicalize, Canonicalized, TermRef};

use crate::{flat_dataset_from_quads, RdfDataset, RdfQuad, RdfTerm, RdfTriple};

/// The graph canonicalization algorithms. Mirrors the oxigraph Python
/// `CanonicalizationAlgorithm` so the Python surface is unchanged.
#[pyclass(name = "CanonicalizationAlgorithm", eq, eq_int, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum PyCanonicalizationAlgorithm {
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
pub fn canonicalize_quads(
    quads: Vec<RdfQuad>,
    _algorithm: PyCanonicalizationAlgorithm,
) -> Vec<RdfQuad> {
    let ds = flat_dataset_from_quads(&quads).expect("native RDFC-1.0: flat freeze of valid quads");
    let canon = core_canonicalize(&ds);
    let map = label_map(&ds, &canon);
    let mut out: Vec<RdfQuad> = quads.iter().map(|q| relabel_quad(q, &map)).collect();
    out.sort_by_key(quad_sort_key);
    out.dedup();
    out
}

/// Map each original blank-node label to its canonical `c14nN` label.
fn label_map(ds: &RdfDataset, c: &Canonicalized) -> HashMap<String, String> {
    let mut map = HashMap::with_capacity(c.labels.len());
    for (&tid, label) in &c.labels {
        if let TermRef::Blank { label: orig, .. } = ds.resolve(tid) {
            map.insert(orig.to_owned(), label.to_string());
        }
    }
    map
}

/// The N-Quads-string sort key for a native quad (deterministic ordering parity with
/// the prior oxigraph `Quad::to_string` sort).
fn quad_sort_key(quad: &RdfQuad) -> String {
    let triple = format!("{} <{}> {}", quad.subject, quad.predicate, quad.object);
    match &quad.graph_name {
        None => triple,
        Some(g) => format!("{triple} {g}"),
    }
}

fn relabel_quad(quad: &RdfQuad, map: &HashMap<String, String>) -> RdfQuad {
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
fn relabel_term(term: &RdfTerm, map: &HashMap<String, String>) -> RdfTerm {
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

#[cfg(test)]
mod tests {
    use crate::NativeRdfFormat;

    use super::*;
    use crate::py_store::io::parse_quads;

    #[test]
    fn canonicalize_quads_is_deterministic_rdfc10() {
        // Two isomorphic graphs with different blank-node labels must canonicalize
        // to byte-identical quad strings under RDFC-1.0.
        let g1 = "_:a <https://example.org/p> _:b .\n_:b <https://example.org/q> _:a .";
        let g2 = "_:x <https://example.org/p> _:y .\n_:y <https://example.org/q> _:x .";
        let c1 = canonicalize_quads(
            parse_quads(g1.as_bytes(), NativeRdfFormat::NTriples).unwrap(),
            PyCanonicalizationAlgorithm::RDFC_1_0,
        );
        let c2 = canonicalize_quads(
            parse_quads(g2.as_bytes(), NativeRdfFormat::NTriples).unwrap(),
            PyCanonicalizationAlgorithm::RDFC_1_0,
        );
        let s1: Vec<String> = c1.iter().map(quad_sort_key).collect();
        let s2: Vec<String> = c2.iter().map(quad_sort_key).collect();
        assert_eq!(s1, s2, "isomorphic graphs must canonicalize identically");
        assert!(
            s1.iter().any(|q| q.contains("_:c14n")),
            "canonical labels present: {s1:?}"
        );
    }

    #[test]
    fn canonicalize_quads_unstable_is_self_consistent() {
        let g = "_:a <https://example.org/p> _:b .";
        let c1 = canonicalize_quads(
            parse_quads(g.as_bytes(), NativeRdfFormat::NTriples).unwrap(),
            PyCanonicalizationAlgorithm::UNSTABLE,
        );
        let c2 = canonicalize_quads(
            parse_quads(g.as_bytes(), NativeRdfFormat::NTriples).unwrap(),
            PyCanonicalizationAlgorithm::UNSTABLE,
        );
        let s1: Vec<String> = c1.iter().map(quad_sort_key).collect();
        let s2: Vec<String> = c2.iter().map(quad_sort_key).collect();
        assert_eq!(s1, s2);
    }
}
