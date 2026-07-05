// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
#![forbid(unsafe_code)]

//! Native, wasm-clean entailment for the PurRDF [`RdfDataset`] IR.
//!
//! A family of engines sits behind one façade, each the right tool for its regime.
//! The `rdfs` engine is a forward-materialization ("chase") reasoner: it closes a
//! dataset's default graph under a fixed RDFS / OWL-RL rule set to a fixpoint via a
//! native semi-naive evaluator over [`RdfDataset`] terms (no Nemo, no `tokio`, no
//! string round-trip), so this crate stays `wasm32`-clean and MIT/Apache. `Simple`
//! is the identity closure; `RDFS` and `OWL-RL` run the chase.
//!
//! The open-world `OWL-Direct` (Description-Logic tableau) and `RIF` (rule engine)
//! regimes need inputs the plain [`materialize`] façade does not have (the query's
//! class expressions; a parsed rule set) and are served by dedicated entry points.
//!
//! It mints **no** vocabulary IRIs: every constant in `vocab` is a standard
//! `rdf:`/`rdfs:`/`owl:` IRI from the entailment spec itself. `D` (datatype)
//! entailment remains an [`EntailError::Unsupported`] boundary, which the caller
//! records as a typed, spec-inherent gap.

use std::sync::Arc;

use purrdf_core::RdfDataset;

pub(crate) mod interner;
pub(crate) mod owl_dl;
pub(crate) mod rdfs;
pub mod rif;
pub(crate) mod vocab;

pub use owl_dl::query::{QNode, QTriple, materialize_dl};
pub use rif::{Atom, Fact, RifTerm, Rule, RuleSet, materialize_rif};

/// A SPARQL entailment regime (`sparql:entailmentRegime`), by its W3C IRI's local
/// name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    /// `entailment/Simple` — no entailment; the graph is its own closure.
    Simple,
    /// `entailment/RDF` — RDF entailment (the predicate-typing axiomatic rule: every
    /// resource in predicate position is an `rdf:Property`).
    Rdf,
    /// `entailment/RDFS` — RDFS entailment via the native chase.
    Rdfs,
    /// `entailment/OWL-RL` (a.k.a. OWL 2 RL) — RDFS + the OWL-RL-shaped rules.
    OwlRl,
    /// `entailment/OWL-Direct` — open-world OWL DL via the ALCOIQ tableau. Not a
    /// materialize-and-match affair; it needs the query's class expressions.
    OwlDirect,
    /// `entailment/RIF` — RIF-Core rule entailment; needs a parsed rule set.
    Rif,
    /// `entailment/D` — datatype entailment; not materialize-and-match.
    D,
}

impl Regime {
    /// Parse a regime IRI (e.g. `http://www.w3.org/ns/entailment/RDFS`).
    #[must_use]
    pub fn from_iri(iri: &str) -> Option<Self> {
        match iri.rsplit('/').next()? {
            "Simple" => Some(Self::Simple),
            "RDF" => Some(Self::Rdf),
            "RDFS" => Some(Self::Rdfs),
            "OWL-RL" | "OWL-RDF-Based" => Some(Self::OwlRl),
            "OWL-Direct" => Some(Self::OwlDirect),
            "RIF" => Some(Self::Rif),
            "D" => Some(Self::D),
            _ => None,
        }
    }
}

/// Why a closure could not be produced.
#[derive(Debug, Clone)]
pub enum EntailError {
    /// The regime is a spec-inherent boundary for this crate (`D`-entailment, or
    /// `OWL-Direct` reached without a query through the plain [`materialize`] façade).
    Unsupported(Regime),
    /// Building the derived dataset failed.
    Build(String),
    /// A knowledge-base or rule document was malformed (e.g. an ill-formed OWL
    /// class-expression graph or an unrecognized RIF construct).
    Parse(String),
    /// The knowledge base is inconsistent: every query would be entailed, so no
    /// meaningful answer set exists. A hard failure rather than a silent default.
    Inconsistent,
}

impl std::fmt::Display for EntailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(r) => write!(f, "entailment regime {r:?} is not materializable"),
            Self::Build(msg) => write!(f, "entailment build error: {msg}"),
            Self::Parse(msg) => write!(f, "entailment parse error: {msg}"),
            Self::Inconsistent => write!(f, "knowledge base is inconsistent"),
        }
    }
}

impl std::error::Error for EntailError {}

/// Compute the entailment closure of `ds` under `regime`.
///
/// Returns a new dataset holding every original quad plus the inferred triples
/// (in the default graph). `Simple` returns a faithful copy.
///
/// `OWL-Direct` is not reachable here — it requires the query's class expressions.
/// `RIF` requires a parsed rule set. Both are served by dedicated entry points.
///
/// # Errors
///
/// [`EntailError::Unsupported`] for `OWL-Direct`/`RIF`/`D` (regimes that need extra
/// inputs or are a spec-inherent boundary); [`EntailError::Build`] if the derived
/// dataset cannot be frozen.
pub fn materialize(ds: &RdfDataset, regime: Regime) -> Result<Arc<RdfDataset>, EntailError> {
    match regime {
        Regime::Simple => rdfs::copy_of(ds),
        Regime::Rdf => rdfs::close_rdf(ds),
        Regime::Rdfs => rdfs::close(ds, false),
        Regime::OwlRl => rdfs::close(ds, true),
        Regime::OwlDirect | Regime::Rif | Regime::D => Err(EntailError::Unsupported(regime)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab::{
        OWL_SYMMETRICPROPERTY, OWL_TRANSITIVEPROPERTY, RDF_PROPERTY, RDF_TYPE, RDFS_SUBCLASSOF,
    };
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermRef};

    fn iri(b: &mut RdfDatasetBuilder, s: &str) -> purrdf_core::TermId {
        b.intern_iri(s)
    }

    /// Build a dataset from `(s, p, o)` IRI triples in the default graph.
    fn dataset(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let s = iri(&mut b, s);
            let p = iri(&mut b, p);
            let o = iri(&mut b, o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    fn has(ds: &RdfDataset, s: &str, p: &str, o: &str) -> bool {
        ds.quad_refs().any(|q| {
            matches!(q.s, TermRef::Iri(si) if si == s)
                && matches!(q.p, TermRef::Iri(pi) if pi == p)
                && matches!(q.o, TermRef::Iri(oi) if oi == o)
        })
    }

    const A: &str = "http://example.org/A";
    const B: &str = "http://example.org/B";
    const C: &str = "http://example.org/C";
    const X: &str = "http://example.org/x";

    const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
    const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";

    #[test]
    fn rdfs_subclass_is_transitive_and_types_instances() {
        // A ⊑ B ⊑ C, x a A  ⇒  A ⊑ C, x a B, x a C.
        let ds = dataset(&[
            (A, RDFS_SUBCLASSOF, B),
            (B, RDFS_SUBCLASSOF, C),
            (X, RDF_TYPE, A),
        ]);
        let closed = materialize(&ds, Regime::Rdfs).expect("rdfs");
        assert!(
            has(&closed, A, RDFS_SUBCLASSOF, C),
            "subClassOf transitivity"
        );
        assert!(has(&closed, X, RDF_TYPE, B), "rdfs9 one hop");
        assert!(has(&closed, X, RDF_TYPE, C), "rdfs9 transitive typing");
    }

    #[test]
    fn rdfs_domain_and_range_type_endpoints() {
        // (p domain A),(p range B),(x p y) ⇒ (x a A),(y a B).
        let p = "http://example.org/p";
        let y = "http://example.org/y";
        let ds = dataset(&[(p, RDFS_DOMAIN, A), (p, RDFS_RANGE, B), (X, p, y)]);
        let closed = materialize(&ds, Regime::Rdfs).expect("rdfs");
        assert!(has(&closed, X, RDF_TYPE, A), "domain types subject");
        assert!(has(&closed, y, RDF_TYPE, B), "range types object");
    }

    #[test]
    fn owl_transitive_and_symmetric() {
        let p = "http://example.org/rel";
        let y = "http://example.org/y";
        let z = "http://example.org/z";
        let ds = dataset(&[
            (p, RDF_TYPE, OWL_TRANSITIVEPROPERTY),
            (p, RDF_TYPE, OWL_SYMMETRICPROPERTY),
            (X, p, y),
            (y, p, z),
        ]);
        let closed = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        assert!(has(&closed, X, p, z), "transitive closure");
        assert!(has(&closed, y, p, X), "symmetric mirror");
        // RDFS-only must NOT apply the OWL rules.
        let rdfs = materialize(&ds, Regime::Rdfs).expect("rdfs");
        assert!(!has(&rdfs, X, p, z), "no transitive under RDFS regime");
    }

    #[test]
    fn owl_direct_rif_and_d_are_unsupported_via_facade() {
        let ds = dataset(&[(X, RDF_TYPE, A)]);
        assert!(matches!(
            materialize(&ds, Regime::OwlDirect),
            Err(EntailError::Unsupported(Regime::OwlDirect))
        ));
        assert!(matches!(
            materialize(&ds, Regime::Rif),
            Err(EntailError::Unsupported(Regime::Rif))
        ));
        assert!(matches!(
            materialize(&ds, Regime::D),
            Err(EntailError::Unsupported(Regime::D))
        ));
    }

    #[test]
    fn rdf_regime_types_predicates_as_property() {
        // Bare RDF entailment: the predicate of every triple is an rdf:Property
        // (rule rdf1 / rdfs4a), even when the predicate is not otherwise typed.
        let p = "http://example.org/ns#b";
        let y = "http://example.org/ns#c";
        let ds = dataset(&[(X, p, y)]);
        let closed = materialize(&ds, Regime::Rdf).expect("rdf");
        assert!(
            has(&closed, p, RDF_TYPE, RDF_PROPERTY),
            "predicate typed rdf:Property"
        );
        // Simple entailment must NOT derive it.
        let simple = materialize(&ds, Regime::Simple).expect("simple");
        assert!(
            !has(&simple, p, RDF_TYPE, RDF_PROPERTY),
            "no typing under Simple"
        );
    }

    #[test]
    fn rdfs_emission_order_is_deterministic() {
        // Each `close` call seeds a fresh, randomly-hashed `HashSet` of facts, so a
        // hash-order-dependent emission (the bug just fixed) would assign the novel
        // inferred vocabulary terms (e.g. `rdf:Property` from predicate typing) new
        // ids in different orders across two runs, diverging the id-sorted output.
        // A closure that introduces novel terms + an order-sensitive fingerprint of
        // the emitted quads therefore locks in the deterministic-emission contract.
        let p = "http://example.org/p";
        let q = "http://example.org/q";
        let y = "http://example.org/y";
        let input = &[
            (A, RDFS_SUBCLASSOF, B),
            (B, RDFS_SUBCLASSOF, C),
            (p, RDFS_DOMAIN, A),
            (p, RDFS_RANGE, B),
            (q, RDFS_DOMAIN, C),
            (X, p, y),
            (X, RDF_TYPE, A),
        ];
        let ds = dataset(input);

        // Two independently-seeded materializations of the SAME input.
        let first = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        let second = materialize(&ds, Regime::OwlRl).expect("owl-rl");

        let fingerprint = |closed: &RdfDataset| -> Vec<String> {
            closed
                .quad_refs()
                .map(|q| format!("{:?}|{:?}|{:?}", q.s, q.p, q.o))
                .collect()
        };
        let fp_first = fingerprint(&first);
        let fp_second = fingerprint(&second);

        assert_eq!(
            fp_first, fp_second,
            "inferred-triple emission order must be deterministic across runs"
        );
        // Prove inference actually happened, guarding against an empty-closure
        // false-positive (equal-but-trivial fingerprints).
        assert!(
            fp_first.len() > input.len(),
            "closure must derive novel triples for the guard to be meaningful"
        );
    }

    #[test]
    fn simple_regime_is_identity() {
        let ds = dataset(&[(A, RDFS_SUBCLASSOF, B), (X, RDF_TYPE, A)]);
        let closed = materialize(&ds, Regime::Simple).expect("simple");
        // No inference: x is not typed B.
        assert!(!has(&closed, X, RDF_TYPE, B));
        assert!(has(&closed, X, RDF_TYPE, A));
    }
}
