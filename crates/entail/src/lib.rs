// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]

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
//!
//! What each regime *is* and what this crate currently *does* are both data, not
//! prose: [`rules`] returns the specification rule table a [`Regime`] is defined by
//! (78 [`RuleId`]s for `OWL-RL`, 18 for `RDFS`), and [`implemented`] returns the
//! subset the chase fires today. The difference is the regime's measurable gap.
//!
//! # Every call says what it did
//!
//! [`materialize`] returns a [`ReasoningReport`] with every closure — not on request, not
//! behind a second entry point. The report carries the regime's [`Completeness`] (derived
//! from the inventory above, so it improves by itself as rules are added), which rules
//! actually fired and how many conclusions each contributed, the [`Boundary`]s the run
//! met, what it consumed of the evaluation ceilings, and the
//! [`contract_hash`](ReasoningReport::contract_hash) of the calculus it ran — so a
//! consumer can refuse a cached closure minted under a different rule set instead of
//! trusting a sentence about it. See [`report`] for the whole shape and for the overclaim
//! gate it must never trip.

use std::sync::Arc;

use purrdf_core::RdfDataset;

pub(crate) mod calculus;
pub(crate) mod interner;
pub(crate) mod owl_dl;
pub(crate) mod rdfs;
pub mod report;
pub mod rif;
mod rif_xml;
pub(crate) mod rules;
pub(crate) mod vocab;

pub use calculus::calculus_program;
pub use owl_dl::query::{QNode, QTriple, materialize_dl};
pub use report::{
    Boundary, Completeness, Construct, InconsistencyWitness, ReasoningReport, WitnessTriple,
};
pub use rif::{Atom, Fact, RifTerm, Rule, RuleSet, materialize_rif};
pub use rif_xml::{ParsedRifDocument, RifImport, parse_rif_xml, resolve_rif_imports};
pub use rules::{ParseRuleIdError, RuleId, implemented, rules};

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

/// Compute the entailment closure of `ds` under `regime`, and say what was done.
///
/// Returns the closure — a new dataset holding every original quad plus the inferred
/// triples, in the default graph; `Simple` returns a faithful copy — together with the
/// [`ReasoningReport`] for the run.
///
/// # The report is not optional
///
/// There is deliberately no report-free variant of this function. A caller that ignores
/// the report must still bind it, because the alternative — two entry points, one of which
/// discards the evidence — is exactly how a partial rule set comes to be described as
/// complete: the cheap call wins, and nothing downstream can tell that "OWL 2 RL
/// entailment" meant twelve of seventy-eight rules. Binding it costs one `_`; not having
/// it cost this repository a documented overclaim.
///
/// `OWL-Direct` is not reachable here — it requires the query's class expressions.
/// `RIF` requires a parsed rule set. Both are served by dedicated entry points.
///
/// # Errors
///
/// [`EntailError::Unsupported`] for `OWL-Direct`/`RIF`/`D` (regimes that need extra
/// inputs or are a spec-inherent boundary); [`EntailError::Build`] if the derived
/// dataset cannot be frozen. An error is the absence of a run, so it carries no report:
/// [`EntailError::Unsupported`] already names the regime it refused, and nothing was
/// closed for a report to describe.
///
/// ```
/// use purrdf_entail::{Regime, materialize};
/// use purrdf_core::RdfDatasetBuilder;
///
/// let mut builder = RdfDatasetBuilder::new();
/// let cat = builder.intern_iri("http://example.org/Cat");
/// let animal = builder.intern_iri("http://example.org/Animal");
/// let sub = builder.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// builder.push_quad(cat, sub, animal, None);
/// let dataset = builder.freeze().expect("freeze");
///
/// let (closure, report) = materialize(&dataset, Regime::Rdfs).expect("rdfs");
/// assert!(closure.quad_refs().count() > 1);
/// // RDFS defines 18 patterns; this crate fires 9, and the report says so.
/// assert_eq!(report.completeness().missing().len(), 9);
/// assert!(!report.overclaims());
/// ```
pub fn materialize(
    ds: &RdfDataset,
    regime: Regime,
) -> Result<(Arc<RdfDataset>, ReasoningReport), EntailError> {
    let (closure, stats) = match regime {
        Regime::Simple => (rdfs::copy_of(ds)?, report::ChaseStats::none()),
        Regime::Rdf => rdfs::close_rdf(ds)?,
        Regime::Rdfs => rdfs::close(ds, false)?,
        Regime::OwlRl => rdfs::close(ds, true)?,
        Regime::OwlDirect | Regime::Rif | Regime::D => {
            return Err(EntailError::Unsupported(regime));
        }
    };
    Ok((closure, ReasoningReport::of_run(ds, regime, &stats)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab::{
        OWL_SYMMETRICPROPERTY, OWL_TRANSITIVEPROPERTY, RDF_PROPERTY, RDF_TYPE, RDFS_SUBCLASSOF,
    };
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermRef, TermValue};

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
        let (closed, _report) = materialize(&ds, Regime::Rdfs).expect("rdfs");
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
        let (closed, _report) = materialize(&ds, Regime::Rdfs).expect("rdfs");
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
        let (closed, _report) = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        assert!(has(&closed, X, p, z), "transitive closure");
        assert!(has(&closed, y, p, X), "symmetric mirror");
        // RDFS-only must NOT apply the OWL rules.
        let (rdfs, _report) = materialize(&ds, Regime::Rdfs).expect("rdfs");
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
        // (rule `rdfD2`, spelled `rdf1` in RDF 1.0), even when the predicate is not
        // otherwise typed.
        let p = "http://example.org/ns#b";
        let y = "http://example.org/ns#c";
        let ds = dataset(&[(X, p, y)]);
        let (closed, _report) = materialize(&ds, Regime::Rdf).expect("rdf");
        assert!(
            has(&closed, p, RDF_TYPE, RDF_PROPERTY),
            "predicate typed rdf:Property"
        );
        // Simple entailment must NOT derive it.
        let (simple, _report) = materialize(&ds, Regime::Simple).expect("simple");
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
        let (first, first_report) = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        let (second, second_report) = materialize(&ds, Regime::OwlRl).expect("owl-rl");

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
        // The REPORT is deterministic for the same reason and by the same evidence: two
        // independently-seeded runs of one input must render identically, field for field.
        assert_eq!(
            format!("{first_report:?}"),
            format!("{second_report:?}"),
            "the reasoning report must be deterministic across runs"
        );
        // Prove inference actually happened, guarding against an empty-closure
        // false-positive (equal-but-trivial fingerprints).
        assert!(
            fp_first.len() > input.len(),
            "closure must derive novel triples for the guard to be meaningful"
        );
    }

    /// `owl:inverseOf` derives both directions, from the schema side and the data side.
    ///
    /// A golden by construction rather than by the engine: the closure of `p inverseOf q`
    /// over `(x p y)` and `(u q v)` is exactly the two mirrored triples, whichever premise
    /// arrives first. It guards the split of the inverse index into its `prp-inv1` and
    /// `prp-inv2` halves — a split that must move which RULE is credited and nothing else.
    #[test]
    fn inverse_of_mirrors_both_directions() {
        let p = "http://example.org/p";
        let q = "http://example.org/q";
        let y = "http://example.org/y";
        let u = "http://example.org/u";
        let v = "http://example.org/v";
        let ds = dataset(&[
            (p, "http://www.w3.org/2002/07/owl#inverseOf", q),
            (X, p, y),
            (u, q, v),
        ]);
        let (closed, report) = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        assert!(has(&closed, y, q, X), "prp-inv1 mirrors a p-triple into q");
        assert!(has(&closed, v, p, u), "prp-inv2 mirrors a q-triple into p");
        // Both halves are credited, under their own ids.
        let fired: Vec<&str> = report
            .rules_fired()
            .iter()
            .map(|&(rule, _)| rule.as_str())
            .collect();
        assert!(fired.contains(&"prp-inv1"), "{fired:?}");
        assert!(fired.contains(&"prp-inv2"), "{fired:?}");
        // A self-inverse property still mirrors, and still terminates.
        let selfish = dataset(&[(p, "http://www.w3.org/2002/07/owl#inverseOf", p), (X, p, y)]);
        let (closed, _) = materialize(&selfish, Regime::OwlRl).expect("owl-rl");
        assert!(has(&closed, y, p, X));
    }

    // ── The reasoning report ────────────────────────────────────────────────────

    /// The four regimes `materialize` can actually run, for the cross-cutting report
    /// invariants below.
    const RUNNABLE: [Regime; 4] = [Regime::Simple, Regime::Rdf, Regime::Rdfs, Regime::OwlRl];

    /// A fixture with enough schema to make every RDFS-lane rule fire at least once.
    fn schema_fixture() -> Arc<RdfDataset> {
        let p = "http://example.org/p";
        let q = "http://example.org/q";
        let y = "http://example.org/y";
        dataset(&[
            (A, RDFS_SUBCLASSOF, B),
            (B, RDFS_SUBCLASSOF, C),
            (A, RDF_TYPE, "http://www.w3.org/2000/01/rdf-schema#Class"),
            (p, RDF_TYPE, RDF_PROPERTY),
            (p, RDFS_DOMAIN, A),
            (p, RDFS_RANGE, B),
            (p, "http://www.w3.org/2000/01/rdf-schema#subPropertyOf", q),
            (X, p, y),
            (X, RDF_TYPE, A),
        ])
    }

    /// `completeness` is `rules(r)` minus `implemented(r)`, COMPUTED — and today's gap is
    /// additionally pinned so a later change that closes one has to say so here.
    #[test]
    fn completeness_is_derived_from_the_inventory_and_pinned() {
        let ds = schema_fixture();
        for regime in RUNNABLE {
            let (_, report) = materialize(&ds, regime).expect("runnable regime");
            // Computed, not asserted: the expected value is the inventory difference.
            let expected: Vec<RuleId> = rules(regime)
                .iter()
                .copied()
                .filter(|rule| !implemented(regime).contains(rule))
                .collect();
            assert_eq!(report.completeness().missing(), expected, "{regime:?}");
            assert_eq!(
                report.completeness().is_exact(),
                expected.is_empty(),
                "{regime:?}"
            );
            assert_eq!(report.regime(), regime);
        }

        // The ratchet. When a later change teaches the chase a rule these numbers MUST
        // fall, and this assertion is where that has to be acknowledged. Never widen it to
        // an inequality: "at most 66 missing" would pass forever without anyone noticing a
        // regression back up to 66.
        let gaps: Vec<(&str, usize)> = RUNNABLE
            .iter()
            .map(|&r| {
                let (_, report) = materialize(&ds, r).expect("runnable regime");
                (
                    match r {
                        Regime::Simple => "Simple",
                        Regime::Rdf => "RDF",
                        Regime::Rdfs => "RDFS",
                        _ => "OWL-RL",
                    },
                    report.completeness().missing().len(),
                )
            })
            .collect();
        assert_eq!(
            gaps,
            vec![("Simple", 0), ("RDF", 2), ("RDFS", 9), ("OWL-RL", 66)],
            "(regime, rules the regime defines that the chase does not fire)"
        );

        // Only `Simple` is exact today, and it is exact because it has no rule table —
        // not because the chase is complete for anything.
        let (_, simple) = materialize(&ds, Regime::Simple).expect("simple");
        assert!(simple.completeness().is_exact());
        assert!(rules(Regime::Simple).is_empty());
    }

    /// The named missing rules are the right ones, not merely the right count.
    #[test]
    fn the_missing_rules_are_named() {
        let ds = schema_fixture();
        let (_, report) = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        let missing = report.completeness().missing();
        // Fired, so absent from the gap.
        for present in [RuleId::CaxSco, RuleId::PrpTrp, RuleId::ScmSco] {
            assert!(!missing.contains(&present), "{present} is implemented");
        }
        // Not fired, so present in the gap — one from each of Tables 4, 6, 8 and 9.
        for absent in [
            RuleId::EqRef,
            RuleId::ClsSvf1,
            RuleId::DtType1,
            RuleId::ScmCls,
        ] {
            assert!(missing.contains(&absent), "{absent} is missing");
        }
        // The gap is in specification table order, like the table it is drawn from.
        let mut sorted = missing.to_vec();
        sorted.sort_unstable();
        assert_eq!(missing, sorted.as_slice());
    }

    /// THE OVERCLAIM GATE: a report may never say `Exact` while naming a boundary.
    ///
    /// The absence of this gate is what let plain "OWL 2 RL entailment" stand in the
    /// documentation of a twelve-rule chase. It runs over every regime and over inputs
    /// chosen to trip every boundary the crate can emit.
    #[test]
    fn no_report_ever_overclaims() {
        for ds in [
            schema_fixture(),
            triple_term_fixture(),
            named_graph_fixture(),
            literal_object_fixture(),
            dataset(&[]),
        ] {
            for regime in RUNNABLE {
                let (_, report) = materialize(&ds, regime).expect("runnable regime");
                assert!(
                    !report.overclaims(),
                    "{regime:?} reported Exact alongside {:?}",
                    report.boundaries()
                );
                // Spelled out, so the gate does not depend on `overclaims` being right.
                assert!(
                    !report.completeness().is_exact() || report.boundaries().is_empty(),
                    "{regime:?}"
                );
            }
        }
    }

    /// A dataset whose object position holds an RDF 1.2 triple term.
    fn triple_term_fixture() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(X);
        let p = b.intern_iri("http://example.org/says");
        let inner_s = b.intern_iri(A);
        let inner_p = b.intern_iri(RDFS_SUBCLASSOF);
        let inner_o = b.intern_iri(B);
        let quoted = b.intern_triple(inner_s, inner_p, inner_o);
        b.push_quad(s, p, quoted, None);
        let sub = b.intern_iri(RDFS_SUBCLASSOF);
        b.push_quad(inner_s, sub, inner_o, None);
        b.freeze().expect("freeze")
    }

    /// A dataset with a quad outside the default graph.
    fn named_graph_fixture() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(X);
        let ty = b.intern_iri(RDF_TYPE);
        let a = b.intern_iri(A);
        let g = b.intern_iri("http://example.org/g");
        b.push_quad(s, ty, a, None);
        b.push_quad(s, ty, a, Some(g));
        b.freeze().expect("freeze")
    }

    /// A dataset where a ranged property points at a LITERAL, so `rdfs3` would have to
    /// conclude into subject position and cannot.
    fn literal_object_fixture() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(X);
        let p = b.intern_iri("http://example.org/label");
        let rng = b.intern_iri(RDFS_RANGE);
        let a = b.intern_iri(A);
        let literal = b.intern_literal(purrdf_core::RdfLiteral::typed(
            "cat",
            "http://www.w3.org/2001/XMLSchema#string",
        ));
        b.push_quad(p, rng, a, None);
        b.push_quad(s, p, literal, None);
        b.freeze().expect("freeze")
    }

    /// Boundaries are not decorative: a real RL-lane construct emits one, with a reason.
    #[test]
    fn boundaries_are_emitted_for_real_constructs() {
        let has = |ds: &RdfDataset, regime: Regime, construct: Construct| {
            let (_, report) = materialize(ds, regime).expect("runnable regime");
            report
                .boundaries()
                .iter()
                .any(|boundary| boundary.construct() == construct)
        };

        // A triple term the chase cannot look inside — the RL lane's own boundary, not
        // the DL lane's.
        let quoted = triple_term_fixture();
        assert!(has(&quoted, Regime::OwlRl, Construct::TripleTerm));
        assert!(has(&quoted, Regime::Rdfs, Construct::TripleTerm));
        // …and the plain fixture, which has none, does not claim one.
        assert!(!has(
            &schema_fixture(),
            Regime::OwlRl,
            Construct::TripleTerm
        ));

        // A quad outside the default graph.
        assert!(has(
            &named_graph_fixture(),
            Regime::OwlRl,
            Construct::NamedGraph
        ));
        assert!(!has(
            &schema_fixture(),
            Regime::OwlRl,
            Construct::NamedGraph
        ));

        // A conclusion that would need a literal in subject position.
        assert!(has(
            &literal_object_fixture(),
            Regime::Rdfs,
            Construct::GeneralizedRdf
        ));
        assert!(!has(
            &schema_fixture(),
            Regime::Rdfs,
            Construct::GeneralizedRdf
        ));

        // The two inherent boundaries hold for every input of their lane.
        assert!(has(
            &dataset(&[]),
            Regime::Rdfs,
            Construct::DatatypeValueSpace
        ));
        assert!(has(
            &dataset(&[]),
            Regime::Rdfs,
            Construct::AxiomaticTriples
        ));
        // OWL 2 RL/RDF omits the RDF/RDFS axiomatic triples, so its lane does not meet
        // that one.
        assert!(!has(
            &dataset(&[]),
            Regime::OwlRl,
            Construct::AxiomaticTriples
        ));
        // `Simple` copies faithfully, so it meets none of them — which is what makes its
        // `Exact` honest.
        let (_, simple) = materialize(&quoted, Regime::Simple).expect("simple");
        assert!(simple.boundaries().is_empty());

        // Every boundary carries a technical reason naming what it blocks.
        let (_, report) = materialize(&quoted, Regime::Rdfs).expect("rdfs");
        assert!(!report.boundaries().is_empty());
        for boundary in report.boundaries() {
            assert!(!boundary.reason().is_empty());
            assert_eq!(boundary.reason(), boundary.construct().reason());
        }
        // In `Construct` declaration order, deduplicated.
        let constructs: Vec<Construct> = report
            .boundaries()
            .iter()
            .map(|boundary| boundary.construct())
            .collect();
        let mut sorted = constructs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(constructs, sorted);
    }

    /// `rules_fired` names rules that really fired, in specification table order, and its
    /// counts sum to exactly the number of inferred triples.
    #[test]
    fn rules_fired_is_ordered_attributed_and_adds_up() {
        let ds = schema_fixture();
        for regime in RUNNABLE {
            let (closed, report) = materialize(&ds, regime).expect("runnable regime");
            let fired = report.rules_fired();

            // Specification table order, no repeats, no zero entries.
            let mut previous: Option<RuleId> = None;
            for &(rule, count) in fired {
                assert!(count > 0, "{regime:?} listed {rule} with a zero count");
                if let Some(previous) = previous {
                    assert!(
                        previous < rule,
                        "{regime:?} is out of table order at {rule}"
                    );
                }
                previous = Some(rule);
                // Every named rule is one the regime implements, or — under OWL-RL only —
                // one of the three RDFS-shaped rules that lane fires under no OWL name.
                assert!(
                    implemented(regime).contains(&rule)
                        || calculus::is_rdfs_shaped_extra(regime, rule),
                    "{regime:?} credited {rule}, which it does not implement"
                );
            }

            // The counts are conclusions COMMITTED, so they sum to the inferred triples.
            let inferred = closed.quad_refs().count() - ds.quad_refs().count();
            let total: u64 = fired.iter().map(|&(_, count)| count).sum();
            assert_eq!(
                usize::try_from(total).expect("count fits usize"),
                inferred,
                "{regime:?}: per-rule counts must sum to the inferred triples"
            );
        }

        // `Simple` infers nothing, so nothing fired — an empty list, not a zeroed one.
        let (_, simple) = materialize(&ds, Regime::Simple).expect("simple");
        assert!(simple.rules_fired().is_empty());

        // The OWL-RL lane really does credit the three RDFS-shaped rules by their RDFS
        // names, which is the honest reading of what it fires.
        let (_, owl) = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        let names: Vec<&str> = owl.rules_fired().iter().map(|&(r, _)| r.as_str()).collect();
        assert!(names.contains(&"rdfs6"), "{names:?}");
        assert!(names.contains(&"cax-sco"), "{names:?}");
        assert!(!names.contains(&"rdfs9"), "the OWL lane uses the OWL name");
    }

    /// The report's contract hash is `purrdf-datalog`'s over this crate's declared
    /// calculus — recomputable by a consumer, and different for different rule sets.
    #[test]
    fn the_contract_hash_names_the_calculus() {
        let ds = schema_fixture();
        let mut seen = Vec::new();
        for regime in RUNNABLE {
            let (_, report) = materialize(&ds, regime).expect("runnable regime");
            assert_eq!(
                report.contract_hash(),
                purrdf_datalog::cache::contract_hash(&calculus_program(regime)),
                "{regime:?}"
            );
            seen.push((regime, report.contract_hash()));
        }
        // The three rule-bearing lanes are three different calculi.
        assert_ne!(seen[1].1, seen[2].1);
        assert_ne!(seen[2].1, seen[3].1);
        assert_ne!(seen[1].1, seen[3].1);
        // The hash is a property of the CALCULUS, not of the data it ran over.
        let (_, other) = materialize(&triple_term_fixture(), Regime::Rdfs).expect("rdfs");
        assert_eq!(other.contract_hash(), seen[2].1);
    }

    /// The budget report carries real measurements, and `Simple` — which evaluates
    /// nothing — reports zero for all three.
    #[test]
    fn the_budget_reports_what_the_run_consumed() {
        let ds = schema_fixture();
        let (_, simple) = materialize(&ds, Regime::Simple).expect("simple");
        assert_eq!(simple.budget().join_steps(), 0);
        assert_eq!(simple.budget().stored_facts(), 0);
        assert_eq!(simple.budget().term_arena_bytes(), 0);

        let (_, rdfs) = materialize(&ds, Regime::Rdfs).expect("rdfs");
        assert!(
            rdfs.budget().join_steps() > 0,
            "the chase enumerated nothing"
        );
        assert!(
            rdfs.budget().stored_facts() >= ds.quad_refs().count(),
            "the store holds at least the seeded facts"
        );
        assert!(rdfs.budget().term_arena_bytes() > 0);
        // A candidate is enumerated for every committed conclusion and then some, so the
        // step count bounds the conclusion count.
        let committed: u64 = rdfs.rules_fired().iter().map(|&(_, n)| n).sum();
        assert!(rdfs.budget().join_steps() >= committed);
    }

    /// An inconsistency witness is `None`, and that is a fact about the rule set: every
    /// rule that could conclude `false` is in the regime's missing list.
    #[test]
    fn no_run_reports_an_inconsistency_and_none_could() {
        let ds = schema_fixture();
        for regime in RUNNABLE {
            let (_, report) = materialize(&ds, regime).expect("runnable regime");
            assert!(report.inconsistency().is_none(), "{regime:?}");
        }
        // The reason, asserted rather than asserted-in-a-comment: not one of OWL 2 RL's
        // inconsistency rules is implemented, so no chase path can detect a clash.
        for rule in [
            RuleId::EqDiff1,
            RuleId::PrpIrp,
            RuleId::PrpAsyp,
            RuleId::PrpPdw,
            RuleId::ClsNothing2,
            RuleId::ClsCom,
            RuleId::CaxDw,
            RuleId::DtNotType,
        ] {
            assert!(
                !implemented(Regime::OwlRl).contains(&rule),
                "{rule} became implemented; the witness is now reachable and must be wired"
            );
        }
        // The type is complete and constructible today, so wiring it later moves nothing
        // else.
        let witness = InconsistencyWitness::new(
            RuleId::CaxDw,
            vec![
                WitnessTriple::new(
                    TermValue::iri(A),
                    TermValue::iri("http://www.w3.org/2002/07/owl#disjointWith"),
                    TermValue::iri(B),
                ),
                WitnessTriple::new(
                    TermValue::iri(X),
                    TermValue::iri(RDF_TYPE),
                    TermValue::iri(A),
                ),
            ],
            None,
        );
        assert_eq!(witness.rule(), RuleId::CaxDw);
        assert_eq!(witness.premises().len(), 2);
        assert_eq!(witness.premises()[1].object(), &TermValue::iri(A));
        assert!(witness.graph().is_none());
    }

    /// Two runs of the same input render byte-identically, across every regime and every
    /// fixture — the whole report, not just the closure.
    #[test]
    fn reports_are_byte_identical_across_runs() {
        for ds in [
            schema_fixture(),
            triple_term_fixture(),
            named_graph_fixture(),
            literal_object_fixture(),
        ] {
            for regime in RUNNABLE {
                let (_, first) = materialize(&ds, regime).expect("runnable regime");
                let (_, second) = materialize(&ds, regime).expect("runnable regime");
                assert_eq!(
                    format!("{first:?}"),
                    format!("{second:?}"),
                    "{regime:?} report is not reproducible"
                );
            }
        }
    }

    #[test]
    fn simple_regime_is_identity() {
        let ds = dataset(&[(A, RDFS_SUBCLASSOF, B), (X, RDF_TYPE, A)]);
        let (closed, _report) = materialize(&ds, Regime::Simple).expect("simple");
        // No inference: x is not typed B.
        assert!(!has(&closed, X, RDF_TYPE, B));
        assert!(has(&closed, X, RDF_TYPE, A));
    }
}
