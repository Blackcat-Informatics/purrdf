// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The combined approach: certain answers to a basic graph pattern carrying a
//! NON-DISTINGUISHED variable, over the Horn fragment a restricted chase can certify.
//!
//! # The gap this closes
//!
//! [`materialize_dl_reported`]'s own module docs are exact
//! about what its query-independent augmentation delivers: certain answers for a basic
//! graph pattern all of whose variables are DISTINGUISHED (projected). A query variable
//! that is not projected — `?y` in `SELECT ?x WHERE { ?x r ?y . ?y a B }` — is
//! non-distinguished in exactly the sense a query BLANK NODE is, and the decomposition that
//! makes the whole-vocabulary augmentation exact does not hold for it: an open-world model
//! may satisfy `∃y. r(x, y) ∧ B(y)` through an element no finite augmentation over NAMED
//! terms can produce, because no NAMED individual need be `r`-related to `x` at all — the
//! TBox axiom `A ⊑ ∃r.B` only entails that SOME element is, not that a specific named one
//! is. Answering that query over the whole-vocabulary augmentation therefore silently
//! misses `x`, and nothing in the existing boundary machinery says so, because
//! [`materialize_dl_reported`] only recognizes a query BLANK NODE as non-distinguished, not
//! an unprojected ordinary variable.
//!
//! # What this module does instead (Lutz/Toman/Wolter; Stefanoni/Motik/Horrocks)
//!
//! For the Horn fragment a restricted chase can certify terminating — here, TBox axioms of
//! the shape `A ⊑ B` and `A ⊑ ∃r.B` over NAMED classes and a NAMED role, which the
//! module's own private TBox lowering recognizes syntactically — this module:
//!
//! 1. lowers those axioms into `purrdf_datalog::clause::DlClause`s (the atomic form for
//!    `A ⊑ B`, the existential form for `A ⊑ ∃r.B`);
//! 2. runs [`purrdf_datalog::chase::chase`], the crate's own restricted existential chase,
//!    seeded from the dataset's ABox — this MATERIALIZES the canonical model's existential
//!    witnesses as ordinary blank-node facts, frontier-addressed Skolem terms exactly as
//!    every other existential rule in this crate mints them;
//! 3. merges those witness-bearing facts with
//!    [`materialize_dl_reported`]'s own whole-vocabulary augmentation of the NAMED part
//!    (classification, realization, entailed roles, `owl:sameAs`), so ordinary SPARQL BGP
//!    matching over the union answers both the named and the anonymous parts of the query
//!    at once — a non-distinguished variable is free to bind to a minted witness, which is
//!    exactly the certain-answer semantics the axiom licenses.
//!
//! Filtration is the caller's remaining obligation, and it is simple BECAUSE this module
//! hands back exactly which blank terms are chase-minted witnesses
//! ([`CombinedMaterialization::surrogates`]): a solution binding a DISTINGUISHED (projected)
//! variable to one of them is not a certain answer — the regime draws its answers from the
//! scoping graph, and a minted witness is not in it — so the caller drops that row rather
//! than reporting it. See `crates/purrdf/src/reasoning.rs` for where that filter runs.
//!
//! # A note on "rolling up"
//!
//! The classical combined approach's "rolling up" step replaces a NON-DISTINGUISHED
//! variable's whole subtree of the query with a class expression, checked by ordinary DL
//! instance retrieval, PRECISELY so the (in general infinite) canonical model never has to
//! be built concretely. That symbolic step is not needed here: this module's fragment is
//! exactly the one whose canonical model [`purrdf_datalog::chase::certify`] proves FINITE, so
//! the "roll" is realized by materializing that finite canonical model directly (the chase)
//! and matching the query against it by ordinary homomorphism (ordinary SPARQL BGP
//! evaluation) — filtration is still the load-bearing step that keeps a non-distinguished
//! variable's witness from being mistaken for a certain answer of a DISTINGUISHED one.
//!
//! # Applicability is checked, never assumed
//!
//! [`materialize_combined`] returns `Ok(None)` — "not applicable, fall back to the
//! whole-vocabulary augmentation and its own boundary" — whenever the TBox holds anything
//! outside the two recognized shapes, or when [`purrdf_datalog::chase::certify`] cannot
//! prove the resulting clause set terminating (a genuine schema-level existential cycle,
//! e.g. `A ⊑ ∃r.A`, which is a real limit of this fragment rather than an oversight). The
//! caller discloses that fallback as [`crate::Construct::NonHornTBox`] on the report it
//! keeps using — see that construct's reason for the exact boundary this module's own
//! restriction draws.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_datalog::chase::{certify, chase};
use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct};
use purrdf_datalog::store::RelationStore;

use crate::engine::surface_of;
use crate::interner::intern_into;
use crate::report::ReasoningReport;
use crate::vocab::{
    OWL_ONPROPERTY, OWL_RESTRICTION, OWL_SOMEVALUESFROM, RDF_TYPE, RDFS_SUBCLASSOF,
};
use crate::{EntailError, QTriple, materialize_dl_reported};

/// Every OWL 2 TBox/property construct outside the two shapes [`lower_horn_tbox`]
/// recognizes. Any of these anywhere in the default graph disqualifies the WHOLE ontology
/// from the combined approach — this module does not attempt a partial lowering that skips
/// just the disqualifying axiom, because a caller reading "combined approach: applicable"
/// should be able to trust that EVERY TBox axiom was accounted for, not just the ones this
/// module happened to recognize.
const DISQUALIFYING_PREDICATES: &[&str] = &[
    "http://www.w3.org/2002/07/owl#equivalentClass",
    "http://www.w3.org/2002/07/owl#disjointWith",
    "http://www.w3.org/2002/07/owl#complementOf",
    "http://www.w3.org/2002/07/owl#intersectionOf",
    "http://www.w3.org/2002/07/owl#unionOf",
    "http://www.w3.org/2002/07/owl#oneOf",
    "http://www.w3.org/2002/07/owl#allValuesFrom",
    "http://www.w3.org/2002/07/owl#propertyChainAxiom",
    "http://www.w3.org/2002/07/owl#hasKey",
    "http://www.w3.org/2002/07/owl#minCardinality",
    "http://www.w3.org/2002/07/owl#maxCardinality",
    "http://www.w3.org/2002/07/owl#cardinality",
    "http://www.w3.org/2002/07/owl#minQualifiedCardinality",
    "http://www.w3.org/2002/07/owl#maxQualifiedCardinality",
    "http://www.w3.org/2002/07/owl#qualifiedCardinality",
    "http://www.w3.org/2002/07/owl#inverseOf",
    "http://www.w3.org/2002/07/owl#disjointObjectProperties",
    "http://www.w3.org/2002/07/owl#disjointUnionOf",
];

/// The result of running the combined approach: the augmented dataset ordinary SPARQL BGP
/// matching answers over, the report to carry alongside it, and every blank term the
/// restricted chase minted as an existential witness.
#[derive(Debug, Clone)]
pub struct CombinedMaterialization {
    /// The dataset the query's basic graph pattern is matched against: the data, the
    /// whole-vocabulary augmentation of the named part, and the chase's existential
    /// witnesses.
    pub dataset: Arc<RdfDataset>,
    /// The report to carry alongside the answer.
    pub report: ReasoningReport,
    /// The blank-node LABEL of every term the restricted chase minted as an existential
    /// witness (`TermValue` implements neither `Ord` nor `Hash`, so the label — which this
    /// module alone mints and therefore controls the uniqueness of — is the set's key
    /// rather than the term itself). A solution binding a DISTINGUISHED (projected) query
    /// variable to a blank node whose label is in this set is not a certain answer and must
    /// be dropped by the caller before the answer set is returned.
    pub surrogates: BTreeSet<String>,
}

/// Attempt the combined approach for `ds`'s basic graph pattern `query_bgp`.
///
/// Returns `Ok(None)` when the ontology's TBox is not in the fragment this module can
/// lower and chase — the caller falls back to
/// [`materialize_dl_reported`] and discloses
/// [`crate::Construct::NonHornTBox`] on the report it keeps using instead.
///
/// # Errors
///
/// Propagates [`EntailError`] from the underlying reverse mapping or the restricted chase
/// (an inconsistent knowledge base, a malformed class-expression graph, or — unreachable in
/// practice since [`certify`] gates every call — a chase budget refusal).
pub fn materialize_combined(
    ds: &RdfDataset,
    query_bgp: &[QTriple],
) -> Result<Option<CombinedMaterialization>, EntailError> {
    let Some(clauses) = lower_horn_tbox(ds) else {
        return Ok(None);
    };
    if !certify(&clauses).is_certified() {
        return Ok(None);
    }

    // The whole-vocabulary augmentation of the NAMED part is unchanged: it is already
    // proven exact for the distinguished-only decomposition, so it is reused rather than
    // reimplemented. Its own report (regime, boundaries, budget) is reused as the combined
    // report too, because the chase step below is a STRICT EXTENSION within a fragment this
    // function already certified terminating — it adds facts, it never revokes the
    // named-part guarantee the reused report already states.
    let (named_dataset, report) = materialize_dl_reported(ds, query_bgp)?;

    if clauses.is_empty() {
        // No existential axiom at all: there is nothing for the chase to add.
        return Ok(Some(CombinedMaterialization {
            dataset: named_dataset,
            report,
            surrogates: BTreeSet::new(),
        }));
    }

    // Seed the restricted chase from the dataset's own default-graph facts, recording the
    // surface -> value dictionary this run needs to read a NAMED term back afterward (a
    // witness surface is looked up nowhere in it — see `resolve_surface`).
    let mut by_surface: BTreeMap<String, TermValue> = BTreeMap::new();
    let mut edb = RelationStore::new();
    for quad in ds.quads() {
        if quad.g.is_some() {
            continue;
        }
        let (s, p, o) = (
            ds.term_value(quad.s),
            ds.term_value(quad.p),
            ds.term_value(quad.o),
        );
        let (ss, ps, os) = (surface_of(&s), surface_of(&p), surface_of(&o));
        by_surface.entry(ss.clone()).or_insert(s);
        by_surface.entry(ps.clone()).or_insert(p);
        by_surface.entry(os.clone()).or_insert(o);
        let _ = edb.insert(&ss, &ps, &os, RelationStore::DEFAULT_GRAPH);
    }

    let outcome = chase(&clauses, edb).map_err(EntailError::Chase)?;
    let witnesses: BTreeSet<&str> = outcome.witnesses().witnesses().collect();

    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(&named_dataset);
    let mut surrogates: BTreeSet<String> = BTreeSet::new();
    let mut witness_terms: BTreeMap<String, TermValue> = BTreeMap::new();

    for derivation in outcome.derivations() {
        let fact = derivation.fact();
        // A fact that touches no witness is already covered by `materialize_dl_reported`'s
        // own (complete, tableau-backed) realization/classification injections over the
        // named part — restating it here would be redundant, not wrong, but the whole point
        // of this pass is the witness-bearing facts the named augmentation cannot state.
        let touches_witness = [&fact.subject, &fact.predicate, &fact.object]
            .into_iter()
            .any(|s| witnesses.contains(s.as_str()));
        if !touches_witness {
            continue;
        }
        let s = resolve_surface(&fact.subject, &witnesses, &by_surface, &mut witness_terms);
        let p = resolve_surface(&fact.predicate, &witnesses, &by_surface, &mut witness_terms);
        let o = resolve_surface(&fact.object, &witnesses, &by_surface, &mut witness_terms);
        for surface in [&fact.subject, &fact.object] {
            if witnesses.contains(surface.as_str()) {
                surrogates.insert(witness_label(surface));
            }
        }
        let s_id = intern_into(&mut b, &s);
        let p_id = intern_into(&mut b, &p);
        let o_id = intern_into(&mut b, &o);
        b.push_quad(s_id, p_id, o_id, None);
    }

    let dataset = b
        .freeze()
        .map_err(|error| EntailError::Build(format!("freeze combined dataset: {error}")))?;
    Ok(Some(CombinedMaterialization {
        dataset,
        report,
        surrogates,
    }))
}

/// The blank-node label a chase witness surface becomes.
///
/// The label embeds the chase's own collision-resistant content digest (see
/// `purrdf_datalog::chase`'s `witness_surface`), so two occurrences of the same witness
/// surface always resolve to the same label, and a witness can never collide with a blank
/// node the dataset itself already held (a real label would have to equal this module's own
/// prefixed digest string verbatim).
fn witness_label(surface: &str) -> String {
    format!("purrdfCombinedWitness:{surface}")
}

/// The [`TermValue`] a chase fact's lexical surface denotes: a NAMED term looked up in the
/// seed dictionary, or a freshly minted (memoized) blank node for a witness surface.
fn resolve_surface(
    surface: &str,
    witnesses: &BTreeSet<&str>,
    by_surface: &BTreeMap<String, TermValue>,
    witness_terms: &mut BTreeMap<String, TermValue>,
) -> TermValue {
    if witnesses.contains(surface) {
        witness_terms
            .entry(surface.to_owned())
            .or_insert_with(|| TermValue::Blank {
                label: witness_label(surface),
                scope: BlankScope::DEFAULT,
            })
            .clone()
    } else {
        by_surface.get(surface).cloned().unwrap_or_else(|| {
            panic!(
                "the combined approach's chase named a surface {surface} that was never \
                 seeded and is not a witness — every non-witness term must have come from \
                 the seeded EDB"
            )
        })
    }
}

/// Lower `ds`'s default-graph TBox into a Horn `DlClause` program, or `None` if any
/// TBox/property construct outside the two recognized shapes is present anywhere.
///
/// The two shapes: `A rdfs:subClassOf B` between two NAMED classes (an atomic Datalog
/// rule `type(x, A) -> type(x, B)`), and `A rdfs:subClassOf [ a owl:Restriction ;
/// owl:onProperty p ; owl:someValuesFrom B ]` with `p` and `B` both named (an existential
/// rule `type(x, A) -> ∃y. p(x, y) ∧ type(y, B)`, one shared witness `y` per firing —
/// exactly the DL-clause shape `crate::datalog`'s existential head form was designed to
/// hold). A restriction node carrying anything beyond its type/onProperty/someValuesFrom
/// triple, or a `rdfs:subClassOf` object that is neither a named class nor such a
/// restriction, disqualifies the whole ontology — see [`DISQUALIFYING_PREDICATES`] and the
/// module docs for why this module refuses a partial lowering rather than skipping the one
/// axiom it cannot read.
fn lower_horn_tbox(ds: &RdfDataset) -> Option<Vec<DlClause>> {
    for quad in ds.quads() {
        if quad.g.is_some() {
            continue;
        }
        if let TermValue::Iri(iri) = ds.term_value(quad.p)
            && DISQUALIFYING_PREDICATES.contains(&iri.as_str())
        {
            return None;
        }
    }

    // subject surface -> (subject value, predicate iri -> objects), default graph only.
    let mut index: BTreeMap<String, (TermValue, BTreeMap<String, Vec<TermValue>>)> =
        BTreeMap::new();
    for quad in ds.quads() {
        if quad.g.is_some() {
            continue;
        }
        let subject = ds.term_value(quad.s);
        let TermValue::Iri(predicate) = ds.term_value(quad.p) else {
            continue;
        };
        let object = ds.term_value(quad.o);
        let entry = index
            .entry(surface_of(&subject))
            .or_insert_with(|| (subject, BTreeMap::new()));
        entry.1.entry(predicate).or_default().push(object);
    }

    let mut clauses = Vec::new();
    let mut fresh = 0usize;
    for (subject_value, predicates) in index.values() {
        let TermValue::Iri(subject_iri) = subject_value else {
            // The subclass LHS must be a named class in this fragment; a class-expression
            // subject is outside it. It is only a problem if it ACTUALLY carries a
            // subclass axiom — otherwise it is just data this pass ignores.
            if predicates.contains_key(RDFS_SUBCLASSOF) {
                return None;
            }
            continue;
        };
        let Some(objects) = predicates.get(RDFS_SUBCLASSOF) else {
            continue;
        };
        for object in objects {
            match object {
                TermValue::Iri(object_iri) => {
                    clauses.push(DlClause::datalog(
                        ClauseAtom::positive(
                            ClauseTerm::var("x"),
                            RDF_TYPE,
                            ClauseTerm::iri(object_iri.clone()),
                        ),
                        vec![ClauseAtom::positive(
                            ClauseTerm::var("x"),
                            RDF_TYPE,
                            ClauseTerm::iri(subject_iri.clone()),
                        )],
                    ));
                }
                TermValue::Blank { .. } => {
                    let restriction_surface = surface_of(object);
                    let (_, restriction_predicates) = index.get(&restriction_surface)?;
                    let recognized_predicates = [RDF_TYPE, OWL_ONPROPERTY, OWL_SOMEVALUESFROM];
                    if restriction_predicates
                        .keys()
                        .any(|p| !recognized_predicates.contains(&p.as_str()))
                    {
                        return None;
                    }
                    let is_restriction =
                        restriction_predicates.get(RDF_TYPE).is_some_and(|types| {
                            types
                                .iter()
                                .any(|t| matches!(t, TermValue::Iri(iri) if iri == OWL_RESTRICTION))
                        });
                    let property = restriction_predicates
                        .get(OWL_ONPROPERTY)
                        .and_then(|v| v.first());
                    let filler = restriction_predicates
                        .get(OWL_SOMEVALUESFROM)
                        .and_then(|v| v.first());
                    let (Some(TermValue::Iri(property_iri)), Some(TermValue::Iri(filler_iri))) =
                        (property, filler)
                    else {
                        return None;
                    };
                    if !is_restriction {
                        return None;
                    }
                    fresh += 1;
                    let witness = format!("y{fresh}");
                    clauses.push(DlClause::new(
                        vec![HeadDisjunct::new(vec![
                            ClauseAtom::positive(
                                ClauseTerm::var("x"),
                                property_iri.clone(),
                                ClauseTerm::var(witness.clone()),
                            ),
                            ClauseAtom::positive(
                                ClauseTerm::var(witness.clone()),
                                RDF_TYPE,
                                ClauseTerm::iri(filler_iri.clone()),
                            ),
                        ])],
                        vec![witness],
                        vec![ClauseAtom::positive(
                            ClauseTerm::var("x"),
                            RDF_TYPE,
                            ClauseTerm::iri(subject_iri.clone()),
                        )],
                    ));
                }
                TermValue::Literal { .. } | TermValue::Triple { .. } => return None,
            }
        }
    }
    Some(clauses)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NS: &str = "http://example.org/combined#";
    const RDF_TYPE_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
    const OWL_RESTRICTION_IRI: &str = "http://www.w3.org/2002/07/owl#Restriction";
    const OWL_ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
    const OWL_SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
    const RDFS_SUBCLASSOF_IRI: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

    /// `A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty r ; owl:someValuesFrom B ]`,
    /// `a : A` — the load-bearing shape: `a` is a certain answer of
    /// `SELECT ?x WHERE { ?x r ?y . ?y a B }` even though no named individual is ever
    /// asserted `r`-related to anything.
    fn some_values_from_ontology() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE_IRI);
        let class = b.intern_iri(OWL_CLASS);
        let subclass_of = b.intern_iri(RDFS_SUBCLASSOF_IRI);
        let a = b.intern_iri(&format!("{NS}A"));
        let big_b = b.intern_iri(&format!("{NS}B"));
        let r = b.intern_iri(&format!("{NS}r"));
        let little_a = b.intern_iri(&format!("{NS}a"));
        let restriction = b.intern_blank("restriction", BlankScope::DEFAULT);
        let restriction_class = b.intern_iri(OWL_RESTRICTION_IRI);
        let on_property = b.intern_iri(OWL_ON_PROPERTY);
        let some_values_from = b.intern_iri(OWL_SOME_VALUES_FROM);

        b.push_quad(a, ty, class, None);
        b.push_quad(big_b, ty, class, None);
        b.push_quad(restriction, ty, restriction_class, None);
        b.push_quad(restriction, on_property, r, None);
        b.push_quad(restriction, some_values_from, big_b, None);
        b.push_quad(a, subclass_of, restriction, None);
        b.push_quad(little_a, ty, a, None);

        b.freeze().expect("freeze")
    }

    #[test]
    fn a_some_values_from_axiom_lowers_to_one_existential_clause() {
        let clauses = lower_horn_tbox(&some_values_from_ontology()).expect("recognized shape");
        assert_eq!(clauses.len(), 1);
        assert!(certify(&clauses).is_certified());
    }

    /// The chase mints a witness typed `B` and `r`-related to `a` — the anonymous element
    /// the TBox axiom entails must exist, made concrete as an ordinary blank-node fact.
    #[test]
    fn the_combined_materialization_carries_a_witness_related_to_the_named_individual() {
        let ds = some_values_from_ontology();
        let combined = materialize_combined(&ds, &[])
            .expect("chase runs")
            .expect("the ontology is in the recognized fragment");
        assert_eq!(combined.surrogates.len(), 1, "{:?}", combined.surrogates);
        let witness_label = combined
            .surrogates
            .iter()
            .next()
            .expect("one witness")
            .clone();
        let witness = TermValue::Blank {
            label: witness_label,
            scope: BlankScope::DEFAULT,
        };
        let r = TermValue::iri(format!("{NS}r"));
        let big_b = TermValue::iri(format!("{NS}B"));
        let a_iri = TermValue::iri(format!("{NS}a"));
        let rdf_type = TermValue::iri(RDF_TYPE_IRI);
        let mut saw_role = false;
        let mut saw_type = false;
        for quad in combined.dataset.quads() {
            let (s, p, o) = (
                combined.dataset.term_value(quad.s),
                combined.dataset.term_value(quad.p),
                combined.dataset.term_value(quad.o),
            );
            if s == a_iri && p == r && o == witness {
                saw_role = true;
            }
            if s == witness && p == rdf_type && o == big_b {
                saw_type = true;
            }
        }
        assert!(saw_role, "expected a witness role assertion");
        assert!(saw_type, "expected the witness typed B");
    }

    /// A TBox axiom outside the two recognized shapes (here, `owl:equivalentClass`)
    /// disqualifies the whole ontology, and the caller must fall back.
    #[test]
    fn an_equivalent_class_axiom_is_outside_the_fragment() {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE_IRI);
        let class = b.intern_iri(OWL_CLASS);
        let equiv = b.intern_iri("http://www.w3.org/2002/07/owl#equivalentClass");
        let a = b.intern_iri(&format!("{NS}A"));
        let big_b = b.intern_iri(&format!("{NS}B"));
        b.push_quad(a, ty, class, None);
        b.push_quad(big_b, ty, class, None);
        b.push_quad(a, equiv, big_b, None);
        let ds = b.freeze().expect("freeze");

        assert!(lower_horn_tbox(&ds).is_none());
        assert!(
            materialize_combined(&ds, &[])
                .expect("no chase error")
                .is_none()
        );
    }

    /// A genuine schema-level existential cycle (`A ⊑ ∃r.A`) lowers, but `certify` refuses
    /// it as non-terminating — the combined approach reports "not applicable" rather than
    /// looping.
    #[test]
    fn a_cyclic_existential_schema_is_not_applicable() {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE_IRI);
        let class = b.intern_iri(OWL_CLASS);
        let subclass_of = b.intern_iri(RDFS_SUBCLASSOF_IRI);
        let a = b.intern_iri(&format!("{NS}A"));
        let r = b.intern_iri(&format!("{NS}r"));
        let restriction = b.intern_blank("restriction", BlankScope::DEFAULT);
        let restriction_class = b.intern_iri(OWL_RESTRICTION_IRI);
        let on_property = b.intern_iri(OWL_ON_PROPERTY);
        let some_values_from = b.intern_iri(OWL_SOME_VALUES_FROM);
        b.push_quad(a, ty, class, None);
        b.push_quad(restriction, ty, restriction_class, None);
        b.push_quad(restriction, on_property, r, None);
        b.push_quad(restriction, some_values_from, a, None);
        b.push_quad(a, subclass_of, restriction, None);
        let ds = b.freeze().expect("freeze");

        let clauses = lower_horn_tbox(&ds).expect("syntactically recognized");
        assert!(!certify(&clauses).is_certified());
        assert!(
            materialize_combined(&ds, &[])
                .expect("no chase error")
                .is_none()
        );
    }

    /// A TBox with no subclass axiom at all still succeeds, trivially: the named-part
    /// augmentation alone is the answer and no witness is ever minted.
    #[test]
    fn an_ontology_with_no_existential_axiom_succeeds_with_no_witnesses() {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE_IRI);
        let a = b.intern_iri(&format!("{NS}A"));
        let little_a = b.intern_iri(&format!("{NS}a"));
        b.push_quad(little_a, ty, a, None);
        let ds = b.freeze().expect("freeze");

        let combined = materialize_combined(&ds, &[])
            .expect("no chase error")
            .expect("trivially in the fragment");
        assert!(combined.surrogates.is_empty());
    }
}
