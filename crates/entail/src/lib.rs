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
//! The forward-materialization ("chase") engine closes a dataset's default graph under a
//! fixed RDF / RDFS / OWL-RL rule set to a fixpoint. That rule set is not written twice:
//! [`calculus_program`] renders it as DL clauses, and [`materialize`] evaluates exactly
//! those clauses through `purrdf-datalog`'s native semi-naive evaluator (no Nemo, no
//! `tokio`, no external reasoner), so this crate stays `wasm32`-clean and MIT/Apache.
//! `Simple` is the identity closure; `RDF`, `RDFS`, `OWL-RL` and `D` run the declared
//! program. The `OWL-RL` lane states the WHOLE of OWL 2 Profiles §4.3 Tables 4–9 — all 78
//! rules — and the seventeen of them whose conclusion is `false` are DECIDED rather than
//! drawn: a body match is [`EntailError::Inconsistent`] carrying an
//! [`InconsistencyWitness`], because an inconsistent knowledge base entails every triple
//! and a closure over it would answer a question nobody asked.
//!
//! The open-world `OWL-Direct` (Description-Logic tableau) and `RIF` (rule engine)
//! regimes need inputs the plain [`materialize`] façade does not have (the query's
//! class expressions; a parsed rule set) and are served by dedicated entry points.
//!
//! # The Description-Logic services
//!
//! [`Reasoner`] is the tableau's own surface: consistency, class satisfiability,
//! classification, realization, instance retrieval and axiom entailment, each answering a
//! [`Certified<T>`](Certified) whose [`DlCertificate`] says how complete the answer is.
//! Beside it sit two services that need no reasoning at all — [`extract_module`], which
//! computes a syntactic-locality module for a signature, and [`profile()`], which certifies
//! an ontology against the OWL 2 profiles. See [`reasoner`] for why a tableau needs a
//! completeness notion of its own rather than the chase's [`Completeness`].
//!
//! It mints **no** vocabulary IRIs: every constant in `vocab` is a standard
//! `rdf:`/`rdfs:`/`owl:` IRI from the entailment spec itself. `D` (datatype)
//! entailment IS materializable: this crate realizes it as Simple entailment plus the
//! five `dt-*` rules of OWL 2 Profiles §4.3 Table 8, which is the part of D-entailment a
//! forward chase can produce, and reports the value-space boundary the rest of it lives
//! behind. Only `OWL-Direct` and `RIF` remain [`EntailError::Unsupported`] through this
//! façade, and both because they need an input it does not have.
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

pub(crate) mod axioms;
pub(crate) mod calculus;
pub(crate) mod datatypes;
pub(crate) mod engine;
pub(crate) mod interner;
pub(crate) mod lists;
pub(crate) mod owl_dl;
pub mod reasoner;
pub mod report;
pub mod rif;
mod rif_xml;
pub(crate) mod rules;
pub(crate) mod surrogates;
pub(crate) mod vocab;

pub use calculus::calculus_program;
pub use owl_dl::query::{QNode, QTriple, materialize_dl, materialize_dl_reported};
pub use reasoner::{
    Certified, ClassHierarchy, ConservativeKeep, DlAxiom, DlCertificate, DlCompleteness,
    ModuleExtraction, ModuleMethod, OwlProfile, ProfileCertificate, ProfileViolation, Realization,
    Reasoner, Verdict, extract_module, profile,
};
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
    /// `entailment/D` — datatype entailment.
    ///
    /// Realized as Simple entailment plus the five `dt-*` rules of OWL 2 Profiles §4.3
    /// Table 8, which is the fixed rule table this crate can enumerate for it: the rest of
    /// D-entailment quantifies over infinite value spaces and is reported as the
    /// [`Construct::DatatypeValueSpace`] boundary rather than claimed.
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
    /// The declared calculus could not be evaluated to a fixpoint.
    ///
    /// [`materialize`] runs [`calculus_program`] through `purrdf-datalog`'s semi-naive
    /// evaluator, and that evaluator refuses rather than approximates: a program it has no
    /// semantics for, and — the case a caller will actually meet — an input that passes
    /// one of its three fixed evaluation ceilings. A budget refusal is TOTAL, which is why
    /// it is an error and not a boundary: there is no partial closure to hand back with a
    /// note attached, and a truncated closure presented as a complete one is exactly the
    /// failure a [`ReasoningReport`] exists to prevent. The carried
    /// [`EvalError`](purrdf_datalog::seminaive::EvalError) names which ceiling and what
    /// the run had consumed when it stopped.
    Evaluate(purrdf_datalog::seminaive::EvalError),
    /// The declared calculus states an EXISTENTIAL rule the restricted chase refused.
    ///
    /// `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` conclude about a FRESH blank node, and a
    /// least-fixpoint evaluator over definite clauses has no semantics for that head form,
    /// so the `RDF` and `RDFS` lanes run through `purrdf-datalog`'s restricted chase
    /// instead. The chase refuses rather than approximates, and the refusal a caller will
    /// actually meet is one of the three fixed evaluation ceilings —
    /// [`ChaseError::BudgetExhausted`](purrdf_datalog::chase::ChaseError::BudgetExhausted)
    /// — carrying an accurate report. The one refusal that is about the CALCULUS rather
    /// than the input is
    /// [`ChaseError::NonTerminating`](purrdf_datalog::chase::ChaseError::NonTerminating):
    /// the chase computes its own termination class from the clause set and runs only a
    /// program it certified, so a rule set whose position dependency graph puts an
    /// existential edge in a cycle is named rather than looped on. Neither declared lane
    /// is such a program, which is a CHECKED fact rather than a hope.
    Chase(purrdf_datalog::chase::ChaseError),
    /// An RDF collection an OWL 2 axiom points at is not a well-formed collection.
    ///
    /// `owl:intersectionOf`, `owl:unionOf`, `owl:oneOf`, `owl:members`,
    /// `owl:distinctMembers`, `owl:propertyChainAxiom` and `owl:hasKey` all REQUIRE their
    /// object to be an RDF collection, and the `OWL-RL` lane walks each one into an
    /// internal relation before evaluating. A cell with no `rdf:first`, with two, with no
    /// `rdf:rest`, with two, a walk that never reaches `rdf:nil`, or a cycle is a refusal
    /// rather than a truncation: reasoning over the well-formed PREFIX of a broken
    /// collection would answer a question the caller did not ask, and it would do so
    /// silently. The message names the collection's head, the cell the walk stopped at,
    /// and the fault.
    MalformedList(String),
    /// The knowledge base is inconsistent: every query would be entailed, so no
    /// meaningful answer set exists. A hard failure rather than a silent default.
    ///
    /// # The witness is not optional
    ///
    /// Seventeen OWL 2 RL rules conclude `false`, and turning a body match on one of them
    /// into an error is a real behaviour change for a caller: ordinary dirty data — ONE
    /// `owl:disjointWith` violation, ONE ill-typed literal — stops being a closure that
    /// returns answers and becomes a refusal. That is correct, because an inconsistent
    /// knowledge base entails every triple and a closure over it would be an answer to a
    /// question nobody asked. It is also unusable without evidence, so the evidence is
    /// carried rather than offered: [`InconsistencyWitness`] names the rule whose premises
    /// were all satisfied, the asserted triples that satisfied them in that rule's own
    /// premise order, and the graph they were read from.
    ///
    /// Boxed because it is the one variant with a non-trivial payload and an error type is
    /// returned by value from every entailment entry point.
    Inconsistent(Box<InconsistencyWitness>),
    /// The `OWL-Direct` knowledge base is unsatisfiable, as the tableau found it.
    ///
    /// Distinct from [`Self::Inconsistent`] because the evidence is of a different kind: a
    /// tableau closes every branch of a search, it does not fire a named rule on named
    /// premises, so there is no [`RuleId`] to carry and no triple set that is THE witness.
    /// Reporting it under the chase's variant would mean inventing a rule id, which this
    /// crate does not do.
    Unsatisfiable,
}

impl std::fmt::Display for EntailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(r) => write!(f, "entailment regime {r:?} is not materializable"),
            Self::Build(msg) => write!(f, "entailment build error: {msg}"),
            Self::Parse(msg) => write!(f, "entailment parse error: {msg}"),
            Self::Evaluate(error) => write!(f, "entailment evaluation error: {error}"),
            Self::Chase(error) => write!(f, "entailment chase error: {error}"),
            Self::MalformedList(msg) => write!(f, "entailment collection error: {msg}"),
            Self::Inconsistent(witness) => write!(
                f,
                "knowledge base is inconsistent: {} was satisfied by {} asserted {}",
                witness.rule(),
                witness.premises().len(),
                if witness.premises().len() == 1 {
                    "triple"
                } else {
                    "triples"
                }
            ),
            Self::Unsatisfiable => {
                write!(f, "the OWL-Direct knowledge base is unsatisfiable")
            }
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
/// [`EntailError::Unsupported`] for `OWL-Direct`/`RIF` (the two regimes that need an
/// input this façade does not have); [`EntailError::Inconsistent`] if a rule that
/// concludes `false` matched, carrying the witness; [`EntailError::Evaluate`] if the run passes
/// one of `purrdf-datalog`'s three fixed evaluation ceilings; [`EntailError::Build`] if
/// the derived dataset cannot be frozen. An error is the absence of a run, so it carries
/// no report: [`EntailError::Unsupported`] already names the regime it refused, an
/// exhausted budget carries its own accurate consumption figures, and nothing was closed
/// for a report to describe.
///
/// ```
/// use purrdf_entail::{Regime, RuleId, materialize};
/// use purrdf_core::RdfDatasetBuilder;
///
/// let mut builder = RdfDatasetBuilder::new();
/// let cat = builder.intern_iri("http://example.org/Cat");
/// let animal = builder.intern_iri("http://example.org/Animal");
/// let tom = builder.intern_iri("http://example.org/tom");
/// let sub = builder.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let ty = builder.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// builder.push_quad(cat, sub, animal, None);
/// builder.push_quad(tom, ty, cat, None);
/// let dataset = builder.freeze().expect("freeze");
///
/// // rdfs9 re-types the instance — `tom` is an `Animal` as well as a `Cat`.
/// let (closure, report) = materialize(&dataset, Regime::Rdfs).expect("rdfs");
/// assert!(report.rules_fired().iter().any(|&(r, n)| r == RuleId::Rdfs9 && n >= 1));
/// // …but it is far from the only conclusion: the RDFS lane asserts the axiomatic
/// // triples, so `Cat` and `Animal` are `rdfs:Class`es (rdfs2 / rdfs3 over the
/// // axiomatic domain and range of `rdfs:subClassOf`), each is therefore a sub-class of
/// // itself and of `rdfs:Resource`, and rdfs4 types every term an `rdfs:Resource`.
/// assert!(closure.quad_refs().count() > 3);
/// // RDFS defines 18 patterns and this crate fires all 18 — the four that conclude about
/// // a fresh blank node through `purrdf-datalog`'s restricted chase. The closure is still
/// // not everything the regime entails, and the report says so with a BOUNDARY rather
/// // than with a missing rule: a surrogate blank node is not an answer a SPARQL
/// // entailment regime admits, so every conclusion mentioning one is withheld.
/// assert!(report.completeness().missing().is_empty());
/// assert!(!report.overclaims());
/// ```
pub fn materialize(
    ds: &RdfDataset,
    regime: Regime,
) -> Result<(Arc<RdfDataset>, ReasoningReport), EntailError> {
    let (closure, stats) = match regime {
        Regime::Simple => (engine::copy_of(ds)?, report::RunStats::none()),
        Regime::Rdf | Regime::Rdfs | Regime::OwlRl | Regime::D => engine::close(ds, regime)?,
        Regime::OwlDirect | Regime::Rif => {
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
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfTextDirection, TermRef, TermValue};

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

    /// The two regimes this façade cannot serve are the two that need an input it does not
    /// have — and `D`, which used to be a third, is now a lane like any other.
    #[test]
    fn owl_direct_and_rif_are_unsupported_via_facade_but_d_is_not() {
        let ds = dataset(&[(X, RDF_TYPE, A)]);
        assert!(matches!(
            materialize(&ds, Regime::OwlDirect),
            Err(EntailError::Unsupported(Regime::OwlDirect))
        ));
        assert!(matches!(
            materialize(&ds, Regime::Rif),
            Err(EntailError::Unsupported(Regime::Rif))
        ));
        // `D` is Simple entailment plus OWL 2 Profiles §4.3 Table 8, and it runs.
        let (closed, report) = materialize(&ds, Regime::D).expect("d is materializable");
        assert_eq!(report.regime(), Regime::D);
        assert_eq!(rules(Regime::D).len(), 5);
        assert_eq!(implemented(Regime::D).len(), 5);
        // `dt-type1` is premise-free, so every supported datatype is typed in every `D`
        // closure — including the empty graph's.
        assert!(
            has(
                &closed,
                "http://www.w3.org/2001/XMLSchema#integer",
                RDF_TYPE,
                "http://www.w3.org/2000/01/rdf-schema#Datatype"
            ),
            "dt-type1 must type every datatype supported in OWL 2 RL"
        );
        assert!(!report.overclaims());
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
            vec![("Simple", 0), ("RDF", 0), ("RDFS", 0), ("OWL-RL", 0)],
            "(regime, rules the regime defines that the chase does not fire)"
        );

        // `Simple` is exact because it has no rule table. `OWL-RL` is exact because the
        // chase fires all seventy-eight rules of Tables 4-9 — and it is exact WITHIN
        // BOUNDARIES, not flatly exact, because three of those rules quantify over all
        // literals and one concludes about literal subjects. The two are different claims
        // and the report makes both of them.
        let (_, simple) = materialize(&ds, Regime::Simple).expect("simple");
        assert_eq!(simple.completeness(), &Completeness::Exact);
        assert!(rules(Regime::Simple).is_empty());
        let (_, owl) = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        assert_eq!(
            owl.completeness(),
            &Completeness::ExactWithinBoundaries,
            "a complete rule table beside a boundary is not a contradiction, and it is \
             not `Exact` either"
        );
        assert!(owl.completeness().is_exact());
        assert!(!owl.boundaries().is_empty());
    }

    /// The named missing rules are the right ones, not merely the right count — and for
    /// `OWL-RL` there are none left to name.
    #[test]
    fn the_missing_rules_are_named() {
        let ds = schema_fixture();
        let (_, report) = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        assert!(
            report.completeness().missing().is_empty(),
            "OWL 2 RL is complete: {:?}",
            report.completeness().missing()
        );
        // One rule from each of the six tables, named, so "complete" is checked against
        // the tables rather than against a count.
        for present in [
            RuleId::EqRef,
            RuleId::PrpTrp,
            RuleId::ClsSvf1,
            RuleId::CaxSco,
            RuleId::DtType1,
            RuleId::ScmSco,
        ] {
            assert!(implemented(Regime::OwlRl).contains(&present), "{present}");
        }

        // `RDFS` has NO gap left: the four rules that conclude about a fresh blank node
        // are evaluated by the restricted chase, which is the consumer the existential
        // head form was represented for. That is a claim about the RULE TABLE, and the
        // report makes the other claim separately — the surrogates those four invent do
        // not reach the answer, so the run is `ExactWithinBoundaries` and names the
        // `surrogate` boundary rather than saying `Exact`.
        let (_, rdfs) = materialize(&ds, Regime::Rdfs).expect("rdfs");
        assert!(rdfs.completeness().missing().is_empty());
        assert_eq!(rdfs.completeness(), &Completeness::ExactWithinBoundaries);
        for rule in [
            RuleId::RdfD1,
            RuleId::RdfD1a,
            RuleId::Rdfs14,
            RuleId::Rdfs14a,
        ] {
            assert!(implemented(Regime::Rdfs).contains(&rule), "{rule}");
        }
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
                // Spelled out, so the gate does not depend on `overclaims` being right:
                // the FLATLY-exact variant is the one that may not sit beside a boundary,
                // and `ExactWithinBoundaries` is the honest way to say the other half.
                assert!(
                    *report.completeness() != Completeness::Exact || report.boundaries().is_empty(),
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

    /// A CONSISTENT run reports no inconsistency, and that is now a CHECKED fact rather
    /// than a vacuous one.
    ///
    /// Before the seventeen `false`-headed rules were wired, `inconsistency() == None`
    /// meant "nothing looked"; it now means "seventeen rules looked and found nothing",
    /// which is the difference between an unchecked field and evidence.
    #[test]
    fn a_consistent_run_reports_no_inconsistency() {
        let ds = schema_fixture();
        for regime in RUNNABLE {
            let (_, report) = materialize(&ds, regime).expect("runnable regime");
            assert!(report.inconsistency().is_none(), "{regime:?}");
        }
        // And every rule that could have found one really is in the lane's rule set.
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
            assert!(implemented(Regime::OwlRl).contains(&rule), "{rule}");
        }
    }

    /// AN INCONSISTENCY IS A REFUSAL, AND IT CARRIES ITS WITNESS.
    ///
    /// This is the behaviour change the seventeen `false`-headed rules bring: ONE
    /// `owl:disjointWith` violation turns a materialization from "returns answers" into an
    /// error. Correct — an inconsistent knowledge base entails every triple, so a closure
    /// over it answers a question nobody asked — and unusable without evidence, which is
    /// why the witness is carried rather than offered.
    #[test]
    fn a_disjointness_violation_is_a_refusal_with_a_witness() {
        let disjoint = "http://www.w3.org/2002/07/owl#disjointWith";
        let ds = dataset(&[(A, disjoint, B), (X, RDF_TYPE, A), (X, RDF_TYPE, B)]);

        let Err(EntailError::Inconsistent(witness)) = materialize(&ds, Regime::OwlRl) else {
            panic!("two disjoint classes with a shared instance is `cax-dw`");
        };
        assert_eq!(witness.rule(), RuleId::CaxDw);
        // The premises are the specification's own, in the specification's own order.
        let premises: Vec<(TermValue, TermValue, TermValue)> = witness
            .premises()
            .iter()
            .map(|t| {
                (
                    t.subject().clone(),
                    t.predicate().clone(),
                    t.object().clone(),
                )
            })
            .collect();
        assert_eq!(
            premises,
            vec![
                (
                    TermValue::iri(A),
                    TermValue::iri(disjoint),
                    TermValue::iri(B)
                ),
                (
                    TermValue::iri(X),
                    TermValue::iri(RDF_TYPE),
                    TermValue::iri(A)
                ),
                (
                    TermValue::iri(X),
                    TermValue::iri(RDF_TYPE),
                    TermValue::iri(B)
                ),
            ]
        );
        // The chase reads the default graph only, so the witness came from it.
        assert!(witness.graph().is_none());
        // The message names the rule, so a caller who only logs the error still learns
        // which axiom their data broke.
        let rendered = EntailError::Inconsistent(witness).to_string();
        assert!(rendered.contains("cax-dw"), "{rendered}");

        // The RDFS lane says nothing about `owl:disjointWith`, so the same graph is
        // ordinary data there and closes without complaint. An inconsistency is a property
        // of a CALCULUS and a graph, never of a graph alone.
        assert!(materialize(&ds, Regime::Rdfs).is_ok());
    }

    /// The witness is DETERMINISTIC: the same input names the same rule and the same
    /// premises on every run, which is what makes it usable in a golden.
    #[test]
    fn the_inconsistency_witness_is_deterministic() {
        let ds = dataset(&[
            (
                "http://example.org/irreflexive",
                RDF_TYPE,
                "http://www.w3.org/2002/07/owl#IrreflexiveProperty",
            ),
            (X, "http://example.org/irreflexive", X),
        ]);
        let render = || {
            let Err(EntailError::Inconsistent(witness)) = materialize(&ds, Regime::OwlRl) else {
                panic!("an irreflexive property relating something to itself is `prp-irp`");
            };
            format!("{witness:?}")
        };
        assert_eq!(render(), render());
        let Err(EntailError::Inconsistent(witness)) = materialize(&ds, Regime::OwlRl) else {
            unreachable!("just asserted")
        };
        assert_eq!(witness.rule(), RuleId::PrpIrp);
        assert_eq!(witness.premises().len(), 2);
    }

    /// An ILL-TYPED LITERAL is an inconsistency under `D` as well as under `OWL-RL`, and
    /// it is `dt-not-type` that says so.
    ///
    /// This is the second half of the behaviour change: ordinary dirty data — one literal
    /// whose lexical form its own datatype does not accept — refuses the run.
    #[test]
    fn an_ill_typed_literal_is_a_refusal_under_the_datatype_lanes() {
        let mut b = RdfDatasetBuilder::new();
        let s = iri(&mut b, X);
        let p = iri(&mut b, "http://example.org/age");
        let bad = b.intern_literal(purrdf_core::RdfLiteral::typed(
            "cat",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        b.push_quad(s, p, bad, None);
        let ds = b.freeze().expect("freeze");

        for regime in [Regime::OwlRl, Regime::D] {
            let Err(EntailError::Inconsistent(witness)) = materialize(&ds, regime) else {
                panic!("{regime:?}: an ill-typed literal is `dt-not-type`");
            };
            assert_eq!(witness.rule(), RuleId::DtNotType, "{regime:?}");
            // The witness names a TRIPLE that carries the bad literal, not merely the
            // literal: the internal `DT_ILL_TYPED` premise is bookkeeping, not an
            // asserted triple, so it is filtered out of the evidence. Which occurrence it
            // names is whichever the evaluator's total order reached first — under
            // `OWL-RL` that is `eq-ref`'s own `lt owl:sameAs lt`, which is an occurrence
            // of the literal like any other — so the check is on the position the rule
            // binds rather than on a particular carrier.
            assert_eq!(witness.premises().len(), 1, "{regime:?}");
            assert_eq!(
                witness.premises()[0].object(),
                &TermValue::typed_literal("cat", "http://www.w3.org/2001/XMLSchema#integer"),
                "{regime:?}"
            );
        }
        // A well-typed literal in the same shape closes fine.
        let mut b = RdfDatasetBuilder::new();
        let s = iri(&mut b, X);
        let p = iri(&mut b, "http://example.org/age");
        let good = b.intern_literal(purrdf_core::RdfLiteral::typed(
            "7",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        b.push_quad(s, p, good, None);
        let ds = b.freeze().expect("freeze");
        assert!(materialize(&ds, Regime::D).is_ok());
        assert!(materialize(&ds, Regime::OwlRl).is_ok());
    }

    /// A FUNCTIONAL DATA PROPERTY with two value-different values is an inconsistency, and
    /// that is the whole of Table 8 working with Tables 4 and 5 at once.
    ///
    /// `prp-fp` concludes `"1"^^xsd:integer owl:sameAs "2"^^xsd:integer` — a triple with a
    /// literal SUBJECT, so it is generalized RDF and never reaches the closure — `dt-diff`
    /// concludes the two are different, and `eq-diff1` puts the two together. It is the
    /// classic OWL 2 RL clash, it is unreachable without all three tables, and it is the
    /// only way a `owl:sameAs` between two literals can arise at all: the RDF 1.2 IR
    /// cannot hold one as an ASSERTION, because a literal may not be a subject.
    ///
    /// It also exercises the one-orientation `DT_DIFFERENT` relation end to end. The
    /// pre-pass emits `lt1 ≠ lt2` for `lt1 < lt2` only — halving the largest relation this
    /// crate materializes — and `eq-sym` supplies the mirror, so the clash is found
    /// whichever way round the derived equality happens to be committed.
    #[test]
    fn a_functional_data_property_with_two_values_is_inconsistent() {
        let functional = "http://www.w3.org/2002/07/owl#FunctionalProperty";
        let integer = "http://www.w3.org/2001/XMLSchema#integer";
        let build = |left: &str, right: &str| {
            let mut b = RdfDatasetBuilder::new();
            let p = iri(&mut b, "http://example.org/age");
            let ty = iri(&mut b, RDF_TYPE);
            let fp = iri(&mut b, functional);
            let x = iri(&mut b, X);
            let one = b.intern_literal(purrdf_core::RdfLiteral::typed(left, integer));
            let two = b.intern_literal(purrdf_core::RdfLiteral::typed(right, integer));
            b.push_quad(p, ty, fp, None);
            b.push_quad(x, p, one, None);
            b.push_quad(x, p, two, None);
            b.freeze().expect("freeze")
        };

        // Two DIFFERENT values: `prp-fp` then `dt-diff` then `eq-diff1`.
        let Err(EntailError::Inconsistent(witness)) = materialize(&build("1", "2"), Regime::OwlRl)
        else {
            panic!("a functional property with two value-different values must clash");
        };
        assert_eq!(witness.rule(), RuleId::EqDiff1);
        assert_eq!(witness.premises().len(), 2, "{:?}", witness.premises());

        // Two SPELLINGS OF ONE value: `dt-eq` says they are the same thing, so there is
        // nothing to clash — and `eq-rep-o` carries the value across the spellings.
        let (closed, report) =
            materialize(&build("1", "01"), Regime::OwlRl).expect("one value, two spellings");
        assert!(report.inconsistency().is_none());
        assert!(
            closed.quads().any(|q| {
                closed.term_value(q.s) == TermValue::iri(X)
                    && closed.term_value(q.o) == TermValue::typed_literal("01", integer)
            }),
            "dt-eq and eq-rep-o must keep the equal-valued spelling on the subject"
        );
        // The `owl:sameAs` between the two literals is licensed and UNREPRESENTABLE, so it
        // is dropped at the boundary and the drop is reported rather than fabricated
        // around.
        assert!(
            report
                .boundaries()
                .iter()
                .any(|boundary| boundary.construct() == Construct::GeneralizedRdf),
            "{:?}",
            report.boundaries()
        );
        assert!(!report.overclaims());
    }

    /// `owl:sameAs` substitutes in the PREDICATE position, and it is `eq-rep-p` that does
    /// it — the one rule of the calculus that rewrites a triple's predicate from a term
    /// bound in another atom's OBJECT position.
    ///
    /// It gets a test of its own because it is the rule an IR that addressed relations by
    /// predicate symbol could not express at all: `?p2` is data in the `owl:sameAs` triple
    /// and a relation name in the conclusion.
    #[test]
    fn equality_substitutes_in_the_predicate_position() {
        let same_as = "http://www.w3.org/2002/07/owl#sameAs";
        let p = "http://example.org/p";
        let q = "http://example.org/q";
        let y = "http://example.org/y";
        let ds = dataset(&[(p, same_as, q), (X, p, y)]);
        let (closed, report) = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        assert!(has(&closed, X, q, y), "eq-rep-p must rewrite the predicate");
        assert!(
            report
                .rules_fired()
                .iter()
                .any(|&(rule, count)| rule == RuleId::EqRepP && count >= 1),
            "{:?}",
            report.rules_fired()
        );
        // And the equivalence relation itself is closed: `eq-sym` mirrors the assertion
        // and `eq-ref` makes every term the same as itself.
        assert!(has(&closed, q, same_as, p), "eq-sym");
        assert!(has(&closed, X, same_as, X), "eq-ref");
    }

    /// `owl:sameAs` does NOT substitute inside an RDF 1.2 TRIPLE TERM, and the run says so.
    ///
    /// The chase interns a triple term as ONE atomic term and never looks inside it, so
    /// `<<( :x :p :y )>>` and `<<( :x :p :z )>>` stay two terms even when `:y owl:sameAs
    /// :z`. That is a documented boundary rather than silence: an implementation that
    /// substituted inside would be doing something the chase cannot see, and one that said
    /// nothing would let a caller believe the congruence was complete.
    #[test]
    fn equality_does_not_substitute_inside_a_triple_term() {
        let same_as = "http://www.w3.org/2002/07/owl#sameAs";
        let p = "http://example.org/p";
        let y = "http://example.org/y";
        let z = "http://example.org/z";

        let mut b = RdfDatasetBuilder::new();
        let x = iri(&mut b, X);
        let says = iri(&mut b, SAYS);
        let same = iri(&mut b, same_as);
        let yy = iri(&mut b, y);
        let zz = iri(&mut b, z);
        let pp = iri(&mut b, p);
        let quoted = b.intern_triple(x, pp, yy);
        b.push_quad(yy, same, zz, None);
        b.push_quad(x, says, quoted, None);
        let ds = b.freeze().expect("freeze");

        let (closed, report) = materialize(&ds, Regime::OwlRl).expect("owl-rl");
        let substituted = quoted_value(X, p, TermValue::iri(z));
        assert!(
            !objects_of(&closed, X, SAYS).contains(&substituted),
            "the chase substituted inside a triple term"
        );
        // The original is carried through untouched…
        assert!(objects_of(&closed, X, SAYS).contains(&quoted_value(X, p, TermValue::iri(y))));
        // …and the boundary that licenses the omission is reported.
        assert!(
            report
                .boundaries()
                .iter()
                .any(|boundary| boundary.construct() == Construct::TripleTerm),
            "{:?}",
            report.boundaries()
        );
        assert!(!report.overclaims());
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

    /// A BLANK NODE survives the closure, in both positions it may occupy, and two blank
    /// nodes that differ only in SCOPE stay two nodes.
    ///
    /// The evaluator interns a term by its lexical surface, so a blank node has to be
    /// rendered to one and read back — and a scope is part of a blank node's identity
    /// (C0.2) while being absent from any standard surface syntax. Collapsing two scopes
    /// into one label would silently merge two individuals, which is unsound rather than
    /// merely lossy, so the fixture asserts the pair stays distinct through a rule that
    /// touches both positions.
    #[test]
    fn blank_nodes_survive_the_closure_and_their_scopes_do_not_collapse() {
        use purrdf_core::BlankScope;

        let mut b = RdfDatasetBuilder::new();
        let sub = iri(&mut b, RDFS_SUBCLASSOF);
        let ty = iri(&mut b, RDF_TYPE);
        let bb = iri(&mut b, B);
        let first = b.intern_blank("shared", BlankScope::DEFAULT);
        let second = b.intern_blank("shared", BlankScope(7));
        let class = b.intern_blank("class", BlankScope::DEFAULT);
        // `_:class ⊑ B`, and two same-labelled blanks from different scopes typed by it.
        b.push_quad(class, sub, bb, None);
        b.push_quad(first, ty, class, None);
        b.push_quad(second, ty, class, None);
        let ds = b.freeze().expect("freeze");

        let (closed, report) = materialize(&ds, Regime::Rdfs).expect("rdfs");
        // rdfs9 re-types BOTH blank subjects. The interesting evidence is WHICH subjects
        // it produced, asserted below over the closure itself; the rule's tally is not
        // pinned here because the RDFS lane also asserts the axiomatic triples, so rdfs9
        // additionally re-types these nodes through `rdfs:Resource` and this fixture is
        // not about that arithmetic.
        assert!(
            report
                .rules_fired()
                .iter()
                .any(|&(rule, count)| rule == RuleId::Rdfs9 && count >= 2),
            "rdfs9 must be credited for both re-typings: {:?}",
            report.rules_fired()
        );
        let typed: Vec<TermValue> = closed
            .quads()
            .filter(|q| {
                closed.term_value(q.p) == TermValue::iri(RDF_TYPE)
                    && closed.term_value(q.o) == TermValue::iri(B)
            })
            .map(|q| closed.term_value(q.s))
            .collect();
        assert_eq!(
            typed.len(),
            2,
            "two scopes must stay two subjects: {typed:?}"
        );
        for value in &typed {
            let (label, _) = value.as_blank().expect("a blank subject stayed blank");
            assert_eq!(label, "shared");
        }
        assert_ne!(typed[0], typed[1], "the two scopes collapsed into one node");
        // The blank OBJECT position round-trips too: `_:class ⊑ B` is still about `_:class`.
        assert!(
            closed.quads().any(|q| {
                closed.term_value(q.p) == TermValue::iri(RDFS_SUBCLASSOF)
                    && closed.term_value(q.s).as_blank().map(|(l, _)| l) == Some("class")
            }),
            "the blank subject of the schema triple was not carried through"
        );
    }

    /// A ceiling is a REFUSAL, and it reaches the caller as one.
    ///
    /// `materialize` evaluates the declared program through `purrdf-datalog`, and that
    /// evaluator holds three fixed ceilings. There is no partial answer behind one: a
    /// truncated closure returned as a complete one is precisely the failure a
    /// [`ReasoningReport`] exists to prevent, so an exhausted budget is
    /// [`EntailError::Evaluate`] and the closure is not produced at all.
    ///
    /// The input is the smallest cross product that passes a ceiling: `p` carries 360
    /// `rdfs:domain` declarations and 380 triples use `p`, so rdfs2 alone must conclude
    /// 136 800 typings — more than [`MAX_STORED_FACTS`](purrdf_datalog::seminaive::MAX_STORED_FACTS)
    /// admits. The report is asserted to carry the OBSERVATION that proved the ceiling was
    /// passed rather than the ceiling itself, because a figure rounded down to the limit
    /// would tell a caller nothing about how far over they are.
    #[test]
    fn an_exhausted_budget_is_a_refusal_with_an_accurate_report() {
        use purrdf_datalog::chase::ChaseError;
        use purrdf_datalog::seminaive::{BudgetResource, MAX_STORED_FACTS};

        /// `rdfs:domain` declarations on `p`.
        const CLASSES: usize = 360;
        /// Triples that use `p`.
        const TRIPLES: usize = 380;

        let mut b = RdfDatasetBuilder::new();
        let p = iri(&mut b, "http://example.org/p");
        let domain = iri(&mut b, RDFS_DOMAIN);
        for index in 0..CLASSES {
            let class = iri(&mut b, &format!("http://example.org/C{index}"));
            b.push_quad(p, domain, class, None);
        }
        for index in 0..TRIPLES {
            let subject = iri(&mut b, &format!("http://example.org/x{index}"));
            let object = iri(&mut b, &format!("http://example.org/y{index}"));
            b.push_quad(subject, p, object, None);
        }
        let ds = b.freeze().expect("freeze");

        // The `RDFS` lane runs through the restricted chase (it states four existential
        // rules), so its ceiling refusal is the chase's — the SAME three fixed constants,
        // charged the same way, refused by name rather than truncated.
        let Err(EntailError::Chase(ChaseError::BudgetExhausted { resource, report })) =
            materialize(&ds, Regime::Rdfs)
        else {
            panic!("a cross product past a fixed ceiling must be refused, not truncated");
        };
        assert_eq!(resource, BudgetResource::StoredFacts);
        assert!(
            report.stored_facts() > MAX_STORED_FACTS,
            "the report must carry the observation that passed the ceiling, not the \
             ceiling: {} vs {MAX_STORED_FACTS}",
            report.stored_facts()
        );
        // The refusal is the EVALUATOR's, not the façade's: the same input copies fine.
        let (copied, simple) = materialize(&ds, Regime::Simple).expect("simple");
        assert_eq!(copied.quad_refs().count(), CLASSES + TRIPLES);
        assert_eq!(simple.budget().stored_facts(), 0);
    }

    #[test]
    fn simple_regime_is_identity() {
        let ds = dataset(&[(A, RDFS_SUBCLASSOF, B), (X, RDF_TYPE, A)]);
        let (closed, _report) = materialize(&ds, Regime::Simple).expect("simple");
        // No inference: x is not typed B.
        assert!(!has(&closed, X, RDF_TYPE, B));
        assert!(has(&closed, X, RDF_TYPE, A));
    }

    // ── Rebuilding a conclusion AROUND a term the rules cannot look inside ──────
    //
    // rdfs7 / prp-spo1 rewrites a triple's PREDICATE and copies its object through
    // unchanged, so the object of the conclusion has to be re-interned into the emitted
    // dataset whatever kind of term it is. Substituting a different term there is
    // unsound: it asserts a triple nothing entails. These tests pin the round trip for
    // each object kind the rewrite can carry.

    /// Fixture property `example.org/says`.
    const SAYS: &str = "http://example.org/says";
    /// Fixture property `example.org/mentions`, the super-property of `says`.
    const MENTIONS: &str = "http://example.org/mentions";
    /// `rdfs:subPropertyOf`, the axiom that drives the rewrite.
    const RDFS_SUBPROPERTYOF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
    /// `rdfs:Resource` — the IRI the old fold substituted for a triple term. Named here
    /// only so its ABSENCE can be asserted.
    const RDFS_RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";

    /// `says ⊑ mentions` plus `x says <o>`, where `o` is whatever term `object` interns.
    ///
    /// The smallest input that makes rdfs7 / prp-spo1 build a conclusion AROUND `o`:
    /// the predicate changes, the object is carried through, and the emitted triple can
    /// only be right if `o` re-materializes as itself.
    fn rewrite_fixture(
        object: impl FnOnce(&mut RdfDatasetBuilder) -> purrdf_core::TermId,
    ) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let x = iri(&mut b, X);
        let says = iri(&mut b, SAYS);
        let mentions = iri(&mut b, MENTIONS);
        let spo = iri(&mut b, RDFS_SUBPROPERTYOF);
        let o = object(&mut b);
        b.push_quad(says, spo, mentions, None);
        b.push_quad(x, says, o, None);
        b.freeze().expect("freeze")
    }

    /// Every default-graph object of `(s, p)`, by value.
    fn objects_of(ds: &RdfDataset, s: &str, p: &str) -> Vec<TermValue> {
        ds.quads()
            .filter(|q| {
                q.g.is_none()
                    && ds.term_value(q.s) == TermValue::iri(s)
                    && ds.term_value(q.p) == TermValue::iri(p)
            })
            .map(|q| ds.term_value(q.o))
            .collect()
    }

    /// A triple term over three IRIs, by value.
    fn quoted_value(s: &str, p: &str, o: TermValue) -> TermValue {
        TermValue::Triple {
            s: Box::new(TermValue::iri(s)),
            p: Box::new(TermValue::iri(p)),
            o: Box::new(o),
        }
    }

    /// rdfs7 / prp-spo1 carries a triple-term object through the rewrite intact.
    ///
    /// `x says <<( A ⊑ B )>>` with `says ⊑ mentions` entails
    /// `x mentions <<( A ⊑ B )>>` and nothing else about `x mentions`. The engine used
    /// to emit `x mentions rdfs:Resource` here, which is not entailed by this input
    /// under any regime — a wrong triple, not a missing one.
    #[test]
    fn a_subproperty_rewrite_carries_a_triple_term_object_through() {
        let ds = rewrite_fixture(|b| {
            let s = b.intern_iri(A);
            let p = b.intern_iri(RDFS_SUBCLASSOF);
            let o = b.intern_iri(B);
            b.intern_triple(s, p, o)
        });
        let expected = quoted_value(A, RDFS_SUBCLASSOF, TermValue::iri(B));
        for regime in [Regime::Rdfs, Regime::OwlRl] {
            let (closed, report) = materialize(&ds, regime).expect("runnable regime");
            assert_eq!(
                objects_of(&closed, X, MENTIONS),
                vec![expected.clone()],
                "{regime:?}: the rewrite must conclude exactly the triple term"
            );
            assert!(
                !has(&closed, X, MENTIONS, RDFS_RESOURCE),
                "{regime:?}: a term was fabricated for the triple term"
            );
            // Opacity is the licensed part, and it is REPORTED: the chase never reasons
            // into the quoted triple (rdfs14 / rdfs14a do not fire), so the closure is
            // sound-incomplete and says so.
            assert!(
                report
                    .boundaries()
                    .iter()
                    .any(|boundary| boundary.construct() == Construct::TripleTerm),
                "{regime:?}: the triple-term boundary must be reported"
            );
            assert!(!report.overclaims(), "{regime:?}");
        }
    }

    /// The reconstruction NESTS: a triple term whose object is itself a triple term
    /// round-trips to full depth through the same rewrite.
    #[test]
    fn a_subproperty_rewrite_carries_a_nested_triple_term_through() {
        let ds = rewrite_fixture(|b| {
            let a = b.intern_iri(A);
            let sco = b.intern_iri(RDFS_SUBCLASSOF);
            let bb = b.intern_iri(B);
            let inner = b.intern_triple(a, sco, bb);
            let p = b.intern_iri("http://example.org/p");
            b.intern_triple(a, p, inner)
        });
        let expected = quoted_value(
            A,
            "http://example.org/p",
            quoted_value(A, RDFS_SUBCLASSOF, TermValue::iri(B)),
        );
        for regime in [Regime::Rdfs, Regime::OwlRl] {
            let (closed, _report) = materialize(&ds, regime).expect("runnable regime");
            assert_eq!(
                objects_of(&closed, X, MENTIONS),
                vec![expected.clone()],
                "{regime:?}: the nested triple term was not rebuilt to depth"
            );
        }
    }

    /// A directional language-tagged literal keeps its base direction across the rewrite.
    ///
    /// Direction participates in literal identity (C0.1), so a conclusion that dropped it
    /// would be about a DIFFERENT literal than the premise was.
    #[test]
    fn a_subproperty_rewrite_preserves_a_literal_base_direction() {
        for direction in [RdfTextDirection::Ltr, RdfTextDirection::Rtl] {
            let ds = rewrite_fixture(|b| {
                b.intern_literal(purrdf_core::RdfLiteral {
                    lexical_form: "hello".to_owned(),
                    datatype: None,
                    language: Some("en".to_owned()),
                    direction: Some(direction),
                })
            });
            let expected = TermValue::Literal {
                lexical_form: "hello".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some("en".to_owned()),
                direction: Some(direction),
            };
            for regime in [Regime::Rdfs, Regime::OwlRl] {
                let (closed, _report) = materialize(&ds, regime).expect("runnable regime");
                assert_eq!(
                    objects_of(&closed, X, MENTIONS),
                    vec![expected.clone()],
                    "{regime:?}: {direction:?} was not preserved through the rewrite"
                );
            }
        }
    }

    /// A triple term in a position the rules CANNOT use fabricates nothing.
    ///
    /// `p rdfs:range A` with `x p <<( A ⊑ B )>>` would have rdfs3 / prp-rng conclude
    /// `<<( A ⊑ B )>> rdf:type A`, whose subject is a triple term — a generalized-RDF
    /// triple the IR cannot hold. The conclusion is abandoned and the drop is reported;
    /// what may never happen is a stand-in term being invented so the triple can be
    /// emitted anyway.
    #[test]
    fn a_triple_term_the_rules_cannot_conclude_into_fabricates_no_term() {
        let mut b = RdfDatasetBuilder::new();
        let x = iri(&mut b, X);
        let p = iri(&mut b, "http://example.org/p");
        let rng = iri(&mut b, RDFS_RANGE);
        let a = iri(&mut b, A);
        let sco = iri(&mut b, RDFS_SUBCLASSOF);
        let bb = iri(&mut b, B);
        let quoted = b.intern_triple(a, sco, bb);
        b.push_quad(p, rng, a, None);
        b.push_quad(x, p, quoted, None);
        let ds = b.freeze().expect("freeze");

        for regime in [Regime::Rdfs, Regime::OwlRl] {
            let (closed, report) = materialize(&ds, regime).expect("runnable regime");
            // Nothing was concluded ABOUT the triple term…
            assert!(
                !closed
                    .quads()
                    .any(|q| matches!(closed.term_value(q.s), TermValue::Triple { .. })),
                "{regime:?}: a triple term reached subject position"
            );
            // …and no stand-in was minted to carry the abandoned conclusion.
            assert!(
                !has(&closed, X, RDF_TYPE, A) && !has(&closed, RDFS_RESOURCE, RDF_TYPE, A),
                "{regime:?}: a term was fabricated for the abandoned conclusion"
            );
            assert!(
                report
                    .boundaries()
                    .iter()
                    .any(|boundary| boundary.construct() == Construct::GeneralizedRdf),
                "{regime:?}: the abandoned conclusion must be reported"
            );
            assert!(!report.overclaims(), "{regime:?}");
        }
    }
}
