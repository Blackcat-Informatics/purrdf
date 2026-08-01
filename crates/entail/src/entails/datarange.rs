// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **data-range** mechanism: a property's declared ranges INTERSECT, and the intersection
//! may be contained in a range the premise never mentions.
//!
//! # The gap this closes
//!
//! Three of W3C's cases have the same shape. `webont-i5-8-006` declares
//! `p rdfs:range xsd:byte` and concludes `p rdfs:range xsd:short`; `-008` declares
//! `xsd:short` and `xsd:unsignedInt` and concludes `xsd:unsignedShort`; `-009` declares
//! `xsd:nonNegativeInteger` and `xsd:nonPositiveInteger` and concludes `xsd:short`.
//!
//! All three are **widenings**, and that is why they are sound at all. `xsd:byte ⊑ xsd:short`,
//! so every value the premise's range admits the conclusion's range admits too; a NARROWING
//! would be the unsound direction and none of the three performs one. The last two need the
//! INTERSECTION of several declared ranges — `short ⊓ unsignedInt ⊑ unsignedShort`, and
//! `nonNegativeInteger ⊓ nonPositiveInteger = {0} ⊑ short` — neither of which is a containment
//! between any two of the datatypes named.
//!
//! No rule of OWL 2 Profiles §4.3 concludes an `rdfs:range` axiom over a datatype, and none
//! could: deciding this needs the XSD value spaces, and a rule table has no arithmetic.
//!
//! # The argument
//!
//! RDF Semantics fixes `rdfs:range` by
//!
//! > `<p,D> ∈ IEXT(rdfs:range)` implies `p ∈ IP`, `D ∈ IC`, and `v ∈ ICEXT(D)` for every
//! > `<u,v> ∈ IEXT(p)`.
//!
//! So the premise's declared ranges `D₁ … Dₖ` put every `p`-value in `ICEXT(D₁) ∩ … ∩
//! ICEXT(Dₖ)`. If that intersection is contained in `ICEXT(D)` for the conclusion's `D`, every
//! `p`-value is in `ICEXT(D)` and the range axiom holds in every model. ∎
//!
//! Both of the axiom's other conjuncts come free rather than being waved through:
//!
//! * `p ∈ IP` follows from the premise's OWN range declaration — the same semantic condition,
//!   applied to `D₁`. This module therefore requires at least one declared range and does not
//!   treat "no ranges" as the empty intersection, which would be `rdfs:Literal` and would let
//!   a property nothing constrains be given any range at all.
//! * `D ∈ IC` follows from the datatype MAP rather than from the premise: OWL 2's RDF-Based
//!   Semantics fixes the XSD datatypes as recognized datatypes, and every recognized datatype
//!   is in `ICEXT(rdfs:Datatype) ⊆ IC`. This module reads only conclusions whose range is such
//!   a datatype (or `rdfs:Literal`), so the conjunct is discharged by the semantics it is
//!   already working in.
//!
//! # `Undecided` is a THIRD answer, and collapsing it is the bug this module is arranged
//! against
//!
//! The containment question is three-valued: [`purrdf_xsd::range::containment`] answers
//! [`Satisfiability::Empty`] (contained — proved), `Inhabited` (not contained — proved) and
//! `Undecided`. The mapping used here is the only sound one:
//!
//! | containment | this module |
//! |---|---|
//! | `Empty` | the axiom is ENTAILED |
//! | `Inhabited`, over an exactly-decided counterexample range | not established, handed back to [`precondition`](super::precondition) |
//! | `Undecided`, or `Inhabited` over a range that is not exactly decided | [`super::UndecidedReason::DataRangeContainment`] |
//!
//! What this module deliberately does NOT use is the tableau's own
//! `DataRangeTable::conjunction_is_empty`. That predicate returns a `bool` and its own doc says
//! `Undecided` answers `false` — which is exactly right where it is used, because there it
//! guards a CLASH and the unsound direction is claiming emptiness. Read as a verdict the same
//! `false` says "not entailed", which silently converts "this decision procedure cannot say"
//! into a statement about the caller's datatypes. So the negative answer here is gated on
//! [`is_exactly_decided`] of the counterexample range, and everything else is
//! `Undecided`.
//!
//! `a_pattern_facet_is_undecided_rather_than_refuted` is the falsifiable form: a premise range
//! carrying an `xsd:pattern` facet is a regular-language question `purrdf-xsd` models as
//! [`DataRange::Opaque`], and the answer over it is `Undecided` and never "not entailed".
//!
//! # What a range term is read as
//!
//! | range term | read as |
//! |---|---|
//! | an XSD datatype IRI `purrdf-xsd` models | that datatype's value space |
//! | `rdfs:Literal` | the whole data domain |
//! | any other IRI, or a blank node | [`DataRange::Opaque`] |
//!
//! The last row is a deliberate under-claim in the safe direction. A blank node in range
//! position is a datatype RESTRICTION or an anonymous class, and either may be anything; an
//! unmodelled datatype IRI (`owl:real`, a user-defined datatype) may OVERLAP a modelled space,
//! so it cannot be assumed disjoint and cannot be dropped from the intersection. Reading both
//! as `Opaque` costs conclusions — a premise range of `[ owl:onDatatype xsd:integer ;
//! owl:withRestrictions ( [ xsd:minInclusive 0 ] ) ]` is answered `Undecided` where a finer
//! reading would answer `Empty` — and it never claims one. Dropping such a range from the
//! intersection instead would WIDEN the intersection, and a widened intersection that still
//! fails containment would be reported as not established: the wrong answer, arrived at
//! confidently.
//!
//! # Applicability is a WHITELIST
//!
//! A conclusion triple is read only when it is `p rdfs:range D` for a NAMED `p` and a `D` this
//! module can read as a data range, and only when the closure declares at least one range for
//! `p`. Everything else is residual and keeps its ordinary obligation to map.
//!
//! # Determinism
//!
//! Axioms are read in the conclusion's own frozen triple order and the premise's ranges in the
//! closure's own order, so two runs over one premise and one conclusion cite the same
//! declarations and reach the same verdict, on `wasm32` as on native.

use purrdf_core::{RdfDataset, TermValue};
use purrdf_xsd::XsdDatatype;
use purrdf_xsd::range::{
    DataRange, Satisfiability, containment, counterexample, is_exactly_decided,
};

use std::collections::BTreeSet;

use crate::entails::graph::{Triple, show};
use crate::entails::homomorphism::{Binding, Closure};
use crate::entails::warrant::{EntailmentWarrant, Replay};
use crate::entails::{Attempt, Established, Question, UndecidedReason};
use crate::vocab::{RDFS_LITERAL, RDFS_RANGE};
use crate::{EntailError, Regime};

/// WHY one `rdfs:range` axiom of the conclusion holds: the declarations it follows from.
#[derive(Debug, Clone)]
pub struct RangeContainment {
    /// The conclusion triple this establishes.
    axiom: Triple,
    /// The premise-closure `p rdfs:range Dᵢ` triples whose intersection is contained in it.
    declarations: Vec<Triple>,
}

impl RangeContainment {
    /// The conclusion triple this establishes.
    #[must_use]
    pub const fn axiom(&self) -> &Triple {
        &self.axiom
    }

    /// The premise-closure range declarations whose intersection establishes it.
    ///
    /// All of them, not the ones that happened to matter: the containment is a property of
    /// the whole intersection, and a caller that dropped one could reach a different verdict.
    #[must_use]
    pub fn declarations(&self) -> &[Triple] {
        &self.declarations
    }
}

impl std::fmt::Display for RangeContainment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} rdfs:range {} by containment of {} declared range{}",
            show(&self.axiom[0]),
            show(&self.axiom[2]),
            self.declarations.len(),
            if self.declarations.len() == 1 {
                ""
            } else {
                "s"
            }
        )
    }
}

/// The evidence that a premise entails a conclusion whose `rdfs:range` axioms follow by
/// datatype containment.
#[derive(Debug, Clone)]
pub struct DataRangeWarrant {
    /// The regime the closure was computed under.
    regime: Regime,
    /// What each existential of the RESIDUAL conclusion triples was bound to.
    binding: Binding,
    /// The premise's own closure, which the residual triples and the declarations lie in.
    closure: Closure,
    /// One containment per recognized axiom, in the conclusion's own triple order.
    containments: Vec<RangeContainment>,
}

impl DataRangeWarrant {
    /// The regime whose closure carried the range declarations.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// The mapping that discharged the conclusion's residual (non-range) triples.
    #[must_use]
    pub const fn binding(&self) -> &Binding {
        &self.binding
    }

    /// One containment per `rdfs:range` axiom of the conclusion, in reading order.
    #[must_use]
    pub fn containments(&self) -> &[RangeContainment] {
        &self.containments
    }

    /// How many distinct triples the premise closure this warrant is against holds.
    #[must_use]
    pub fn closure_size(&self) -> usize {
        self.closure.len()
    }

    /// The premise closure this warrant is against.
    pub(crate) const fn closure(&self) -> &Closure {
        &self.closure
    }

    /// This warrant with the fold's residual `binding` attached.
    pub(crate) fn with_binding(mut self, binding: Binding) -> Self {
        self.binding = binding;
        self
    }
}

/// What a range term denotes, as far as this module can tell.
///
/// See the module docs for why an unrecognized term is [`DataRange::Opaque`] rather than
/// dropped.
fn range_of(term: &TermValue) -> DataRange {
    match term {
        TermValue::Iri(iri) if iri == RDFS_LITERAL => DataRange::Any,
        TermValue::Iri(iri) => {
            XsdDatatype::from_iri(iri).map_or(DataRange::Opaque, DataRange::Datatype)
        }
        TermValue::Blank { .. } | TermValue::Literal { .. } | TermValue::Triple { .. } => {
            DataRange::Opaque
        }
    }
}

/// Whether `term` is a range this module reads as the TARGET of a containment.
///
/// Narrower than [`range_of`]: an opaque target would make every answer `Undecided`, so a
/// conclusion whose range this module cannot read is not this module's question at all.
fn is_readable_target(term: &TermValue) -> bool {
    matches!(term, TermValue::Iri(iri) if iri == RDFS_LITERAL || XsdDatatype::from_iri(iri).is_some())
}

/// One recognized `rdfs:range` axiom, with the declarations it is decided against.
struct Axiom {
    /// Its index in the conclusion's own frozen triple order.
    index: usize,
    /// The conclusion triple.
    triple: Triple,
    /// The premise-closure declarations for its property, in closure order.
    declarations: Vec<Triple>,
}

/// The conclusion's still-outstanding range axioms this mechanism decides.
///
/// `pending` is what no earlier lane discharged. `p rdfs:range D` is also a
/// [`freeze`](super::freeze) shape when `D` is a class, and a triple that lane already
/// established is not this one's to decide a second time — possibly the other way, which is
/// how a fold turns two right answers into one wrong one.
fn read(triples: &[Triple], pending: &BTreeSet<usize>, closure: &Closure) -> Vec<Axiom> {
    let mut axioms = Vec::new();
    for (index, triple) in triples.iter().enumerate() {
        if !pending.contains(&index) {
            continue;
        }
        let [subject, predicate, object] = triple;
        if !matches!(subject, TermValue::Iri(_))
            || !matches!(predicate, TermValue::Iri(iri) if iri == RDFS_RANGE)
            || !is_readable_target(object)
        {
            continue;
        }
        // The premise must DECLARE a range for the property. That is what establishes
        // `p ∈ IP`, and it is why an undeclared property is not given a range for free.
        let declarations: Vec<Triple> = closure
            .with_predicate(RDFS_RANGE)
            .iter()
            .filter(|declared| &declared[0] == subject)
            .cloned()
            .collect();
        if !declarations.is_empty() {
            axioms.push(Axiom {
                index,
                triple: triple.clone(),
                declarations,
            });
        }
    }
    axioms
}

/// The counterexample range for `axiom`: a value the declarations admit and the axiom's own
/// range does not.
fn counterexample_of(axiom: &Axiom) -> DataRange {
    let declared: Vec<DataRange> = axiom
        .declarations
        .iter()
        .map(|triple| range_of(&triple[2]))
        .collect();
    counterexample(&DataRange::And(declared), &range_of(&axiom.triple[2]))
}

/// Try to establish `conclusion` from `premise` by datatype containment.
///
/// # Errors
///
/// [`EntailError::MatchBudget`] if the residual match exhausts its budget.
pub(crate) fn attempt(q: &Question<'_>) -> Result<Attempt, EntailError> {
    let Question {
        regime,
        closure,
        triples,
        pending,
        ..
    } = *q;
    // WHITELIST, not blacklist. The containment argument is over the OWL 2 datatype map, and
    // `OWL-RL` is the lane this crate reads a closure against it for. `D` states the five
    // `dt-*` rules and no completeness theorem at all, and the three below it interpret no
    // datatype map; each falls out rather than being served by an argument this crate has not
    // made for it.
    if !matches!(regime, Regime::OwlRl) {
        return Ok(Attempt::NotApplicable);
    }
    let axioms = read(triples, pending, closure);
    if axioms.is_empty() {
        return Ok(Attempt::NotApplicable);
    }

    // THE THREE-VALUED ANSWER, kept three-valued. `Undecided` wins over "not established",
    // because a conclusion one of whose axioms is undecided is a conclusion the precondition
    // must not be allowed to refute.
    let mut undecided: Vec<String> = Vec::new();
    let mut established = true;
    for axiom in &axioms {
        let counter = counterexample_of(axiom);
        match containment(
            &DataRange::And(
                axiom
                    .declarations
                    .iter()
                    .map(|triple| range_of(&triple[2]))
                    .collect(),
            ),
            &range_of(&axiom.triple[2]),
        ) {
            Satisfiability::Empty => {}
            // A witness outside the target range REFUTES the containment — but only if the
            // decision procedure was exact here. An `Inhabited` over an inexact range is a
            // witness that a finer procedure might not exhibit.
            Satisfiability::Inhabited if is_exactly_decided(&counter) => established = false,
            Satisfiability::Inhabited | Satisfiability::Undecided => undecided.push(format!(
                "{} rdfs:range {}",
                show(&axiom.triple[0]),
                show(&axiom.triple[2])
            )),
        }
    }
    if !undecided.is_empty() {
        return Ok(Attempt::Undecided(UndecidedReason::DataRangeContainment(
            undecided,
        )));
    }
    if !established {
        return Ok(Attempt::NotEstablished);
    }

    Ok(Attempt::Entailed(Box::new(Established {
        discharged: axioms.iter().map(|axiom| axiom.index).collect(),
        warrant: EntailmentWarrant::DataRange(DataRangeWarrant {
            regime,
            // The residual is the FOLD's, not this lane's; `entails` fills it in at the end.
            binding: Binding::new(),
            closure: closure.clone(),
            containments: axioms
                .into_iter()
                .map(|axiom| RangeContainment {
                    axiom: axiom.triple,
                    declarations: axiom.declarations,
                })
                .collect(),
        }),
        minted: Vec::new(),
    })))
}

/// Re-decide a data-range warrant against the caller's own premise and conclusion.
///
/// Called by [`verify`](super::verify), which owns the doc comment a caller reads. It runs no
/// reasoner: the conclusion is READ again on the spot against the warrant's own closure, the
/// declarations are re-collected and compared, each containment is re-decided by
/// [`containment`] — arithmetic over value spaces, not inference — and the residual binding is
/// replayed.
pub(crate) fn verify_datarange(
    w: &DataRangeWarrant,
    _conclusion: &RdfDataset,
    triples: &[Triple],
    pending: &BTreeSet<usize>,
) -> Option<Replay> {
    let axioms = read(triples, pending, &w.closure);
    if axioms.len() != w.containments.len() {
        return None;
    }
    for (axiom, claimed) in axioms.iter().zip(&w.containments) {
        if axiom.triple != claimed.axiom {
            return None;
        }
        if axiom.declarations != claimed.declarations {
            return None;
        }
        if !claimed
            .declarations
            .iter()
            .all(|triple| w.closure.contains(triple))
        {
            return None;
        }
        if containment(
            &DataRange::And(
                axiom
                    .declarations
                    .iter()
                    .map(|triple| range_of(&triple[2]))
                    .collect(),
            ),
            &range_of(&axiom.triple[2]),
        ) != Satisfiability::Empty
        {
            return None;
        }
    }

    Some(Replay {
        discharged: axioms.iter().map(|axiom| axiom.index).collect(),
        minted: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};

    use crate::entails::graph::default_graph_triples;
    use crate::entails::{EntailmentOutcome, EntailmentWarrant, ImportMap, entails, verify};
    use crate::vocab::{
        OWL_DATATYPEPROPERTY, OWL_ONDATATYPE, OWL_WITHRESTRICTIONS, RDF_FIRST, RDF_NIL, RDF_REST,
        RDF_TYPE, RDFS_DATATYPE, RDFS_RANGE, XSD_BYTE, XSD_NONNEGATIVEINTEGER,
        XSD_NONPOSITIVEINTEGER, XSD_PATTERN, XSD_SHORT, XSD_STRING, XSD_UNSIGNEDINT,
        XSD_UNSIGNEDSHORT,
    };
    use crate::{
        Materialization, Regime, RuleId, UndecidedReason, extensions, implemented, materialize,
        rules,
    };

    const P: &str = "http://example.org/p";

    /// A default-graph dataset; a leading `_` names a blank node, anything else an IRI.
    fn graph(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let term = |b: &mut RdfDatasetBuilder, value: &str| match value.strip_prefix('_') {
                Some(label) => b.intern_blank(label, BlankScope::DEFAULT),
                None => b.intern_iri(value),
            };
            let s = term(&mut b, s);
            let p = term(&mut b, p);
            let o = term(&mut b, o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    /// `p` is a datatype property with these declared ranges.
    fn ranges(declared: &[&str]) -> Arc<RdfDataset> {
        let mut triples = vec![(P, RDF_TYPE, OWL_DATATYPEPROPERTY)];
        for datatype in declared {
            triples.push((P, RDFS_RANGE, datatype));
        }
        graph(&triples)
    }

    fn decide(premise: &RdfDataset, conclusion: &RdfDataset) -> EntailmentOutcome {
        entails(premise, conclusion, Regime::OwlRl, &ImportMap::new())
            .expect("a consistent premise")
            .into_parts()
            .0
    }

    // ── The three W3C cases, by shape ──────────────────────────────────────────────────

    /// A RANGE WIDENS ALONG THE INTEGER TOWER — W3C `webont-i5-8-006`.
    #[test]
    fn a_declared_range_widens_to_a_containing_datatype() {
        let premise = ranges(&[XSD_BYTE]);
        let conclusion = graph(&[(P, RDFS_RANGE, XSD_SHORT)]);
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("every xsd:byte is an xsd:short");
        };
        let EntailmentWarrant::DataRange(ranged) = &warrant else {
            panic!("no rule of Tables 4-9 concludes an rdfs:range axiom");
        };
        assert_eq!(ranged.regime(), Regime::OwlRl);
        assert_eq!(ranged.containments().len(), 1);
        assert_eq!(ranged.containments()[0].declarations().len(), 1);
        assert!(!ranged.containments()[0].to_string().is_empty());
        assert!(verify(&warrant, &premise, &conclusion));
    }

    /// TWO DECLARED RANGES INTERSECT into a third that contains NEITHER of them — W3C
    /// `webont-i5-8-008` and `-009`.
    #[test]
    fn declared_ranges_intersect_before_they_are_contained() {
        for (declared, target) in [
            ([XSD_SHORT, XSD_UNSIGNEDINT], XSD_UNSIGNEDSHORT),
            ([XSD_NONNEGATIVEINTEGER, XSD_NONPOSITIVEINTEGER], XSD_SHORT),
        ] {
            let premise = ranges(&declared);
            let conclusion = graph(&[(P, RDFS_RANGE, target)]);
            let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
                panic!("{declared:?} intersect to a range contained in {target}");
            };
            let EntailmentWarrant::DataRange(ranged) = &warrant else {
                panic!("reached by containment");
            };
            assert_eq!(
                ranged.containments()[0].declarations().len(),
                2,
                "the containment is a property of the WHOLE intersection"
            );
            assert!(verify(&warrant, &premise, &conclusion));

            // …and NEITHER declared range alone reaches it, so the intersection is doing the
            // work rather than a single containment hiding inside it.
            for single in declared {
                assert!(
                    !matches!(
                        decide(&ranges(&[single]), &conclusion),
                        EntailmentOutcome::Entailed(_)
                    ),
                    "{single} alone is not contained in {target}"
                );
            }
        }
    }

    // ── ADVERSARIAL: narrowing is unsound, and it is refused ───────────────────────────

    /// A NARROWING IS NOT ESTABLISHED. `xsd:short` does not fit in `xsd:byte`, and this is
    /// the direction that would be an unsoundness rather than an incompleteness.
    ///
    /// The answer is UNDECIDED and not a refutation, and the difference is a real one rather
    /// than caution: this lane proves that the DECLARED ranges do not intersect inside the
    /// target, which is not the same claim as `p` having a value outside it. A premise saying
    /// `p rdfs:domain owl:Nothing` makes `p` empty and every range axiom holds of it, so the
    /// containment failing decides nothing about entailment — and Theorem PR1 says nothing
    /// about a conclusion stating an `rdfs:range` axiom either.
    #[test]
    fn a_narrowing_is_undecided_rather_than_refuted() {
        let premise = ranges(&[XSD_SHORT]);
        let conclusion = graph(&[(P, RDFS_RANGE, XSD_BYTE)]);
        assert!(
            !matches!(
                decide(&premise, &conclusion),
                EntailmentOutcome::Entailed(_)
            ),
            "an xsd:short value need not be an xsd:byte"
        );
        assert!(matches!(
            decide(&premise, &conclusion),
            EntailmentOutcome::Undecided(UndecidedReason::ConclusionOutsideRl(_))
        ));
    }

    /// …and disjoint value spaces are not contained in one another either.
    #[test]
    fn a_disjoint_range_is_not_established() {
        assert!(matches!(
            decide(
                &ranges(&[XSD_SHORT]),
                &graph(&[(P, RDFS_RANGE, XSD_STRING)])
            ),
            EntailmentOutcome::Undecided(UndecidedReason::ConclusionOutsideRl(_))
        ));
    }

    /// A PROPERTY THE PREMISE DECLARES NO RANGE FOR gets no range for free — the axiom's
    /// `p ∈ IP` conjunct comes from the premise's own declaration and nowhere else.
    #[test]
    fn an_undeclared_property_gets_no_range() {
        let premise = graph(&[(P, RDF_TYPE, OWL_DATATYPEPROPERTY)]);
        assert!(!matches!(
            decide(&premise, &graph(&[(P, RDFS_RANGE, XSD_SHORT)])),
            EntailmentOutcome::Entailed(_)
        ));
    }

    /// AN UNDECIDED CONTAINMENT IS UNDECIDED, NOT REFUTED.
    ///
    /// A premise range carrying an `xsd:pattern` facet is a regular-language question the
    /// datatype decider models as an opaque range. Falsifiable against the failure mode the
    /// `bool`-returning `conjunction_is_empty` idiom invites: reading its `false` as a verdict
    /// would answer `NotEntailed` here, converting "cannot say" into a statement about the
    /// caller's datatypes.
    #[test]
    fn a_pattern_facet_is_undecided_rather_than_refuted() {
        let premise = graph(&[
            (P, RDF_TYPE, OWL_DATATYPEPROPERTY),
            (P, RDFS_RANGE, "_d"),
            ("_d", RDF_TYPE, RDFS_DATATYPE),
            ("_d", OWL_ONDATATYPE, XSD_STRING),
            ("_d", OWL_WITHRESTRICTIONS, "_l1"),
            ("_l1", RDF_FIRST, "_f"),
            ("_l1", RDF_REST, RDF_NIL),
            ("_f", XSD_PATTERN, XSD_STRING),
        ]);
        let conclusion = graph(&[(P, RDFS_RANGE, XSD_STRING)]);
        let EntailmentOutcome::Undecided(UndecidedReason::DataRangeContainment(axioms)) =
            decide(&premise, &conclusion)
        else {
            panic!("an opaque premise range decides nothing in either direction");
        };
        assert_eq!(axioms.len(), 1);
        assert!(
            axioms[0].contains("short") || axioms[0].contains("string"),
            "{axioms:?}"
        );
    }

    // ── Applicability, the inventory, and `verify` ─────────────────────────────────────

    /// A conclusion whose range this module cannot read is NOT its question, so it does not
    /// answer with a containment it never decided — it reports nothing, and the outcome is
    /// then the conclusion-side clause of Theorem PR1 rather than this lane's.
    #[test]
    fn an_unreadable_target_range_is_not_this_modules_question() {
        let premise = ranges(&[XSD_SHORT]);
        let conclusion = graph(&[(P, RDFS_RANGE, "http://example.org/SomeClass")]);
        let EntailmentOutcome::Undecided(reason) = decide(&premise, &conclusion) else {
            panic!("an rdfs:range axiom is outside PR1's conclusion-side hypothesis");
        };
        assert!(
            matches!(reason, UndecidedReason::ConclusionOutsideRl(_)),
            "this lane must not claim a containment it did not decide: {reason:?}"
        );
    }

    /// STRICT MATERIALIZATION GAINS NOTHING.
    #[test]
    fn materialization_still_does_not_produce_these_conclusions() {
        let (closure, _) =
            materialize(&ranges(&[XSD_BYTE]), Materialization::OwlRl).expect("consistent");
        assert!(
            !default_graph_triples(&closure).contains(&[
                TermValue::iri(P),
                TermValue::iri(RDFS_RANGE),
                TermValue::iri(XSD_SHORT),
            ]),
            "no rule of Tables 4-9 concludes an rdfs:range over a datatype"
        );
    }

    /// THE NORMATIVE INVENTORY IS UNTOUCHED.
    #[test]
    fn the_data_range_lane_adds_no_rule() {
        assert_eq!(rules(Regime::OwlRl).len(), 78);
        assert_eq!(implemented(Regime::OwlRl), rules(Regime::OwlRl));
        assert_eq!(extensions(Regime::OwlRl), [RuleId::ExtEqDiffSym]);
    }

    /// The lane is gated to `OWL-RL` by WHITELIST.
    #[test]
    fn only_the_owl_rl_lane_decides_containment() {
        let premise = ranges(&[XSD_BYTE]);
        let conclusion = graph(&[(P, RDFS_RANGE, XSD_SHORT)]);
        for regime in [Regime::Simple, Regime::Rdf, Regime::Rdfs, Regime::D] {
            assert!(
                !matches!(
                    entails(&premise, &conclusion, regime, &ImportMap::new())
                        .expect("consistent")
                        .outcome(),
                    EntailmentOutcome::Entailed(_)
                ),
                "{regime:?} interprets no datatype map this crate reads a closure against"
            );
        }
    }

    /// A data-range warrant does not replay against another premise or conclusion.
    #[test]
    fn a_data_range_warrant_does_not_replay() {
        let premise = ranges(&[XSD_BYTE]);
        let conclusion = graph(&[(P, RDFS_RANGE, XSD_SHORT)]);
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("entailed");
        };
        assert!(verify(&warrant, &premise, &conclusion));
        assert!(!verify(&warrant, &ranges(&[XSD_STRING]), &conclusion));
        assert!(!verify(
            &warrant,
            &premise,
            &graph(&[(P, RDFS_RANGE, XSD_STRING)])
        ));
        assert!(!verify(
            &warrant,
            &premise,
            &graph(&[(P, RDF_TYPE, OWL_DATATYPEPROPERTY)])
        ));
    }

    /// The whole answer is a function of the inputs: two runs cite the same declarations.
    #[test]
    fn the_data_range_lane_is_deterministic() {
        let run = || {
            let EntailmentOutcome::Entailed(EntailmentWarrant::DataRange(w)) = decide(
                &ranges(&[XSD_SHORT, XSD_UNSIGNEDINT]),
                &graph(&[(P, RDFS_RANGE, XSD_UNSIGNEDSHORT)]),
            ) else {
                panic!("entailed");
            };
            w.containments()
                .iter()
                .map(|c| (c.to_string(), c.declarations().to_vec()))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }
}
