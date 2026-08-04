// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The combined approach: certain answers to a basic graph pattern carrying a
//! NON-DISTINGUISHED variable, over the Horn fragment a restricted chase can certify.
//!
//! # The gap this closes
//!
//! [`materialize_dl_reported`](crate::materialize_dl_reported)'s own module docs are exact
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
//! [`materialize_dl_reported`](crate::materialize_dl_reported) only recognizes a query BLANK NODE as non-distinguished, not
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
//!    [`materialize_dl_reported`](crate::materialize_dl_reported)'s own whole-vocabulary augmentation of the NAMED part
//!    (classification, realization, entailed roles, `owl:sameAs`), so ordinary SPARQL BGP
//!    matching over the union answers both the named and the anonymous parts of the query
//!    at once — a non-distinguished variable is free to bind to a minted witness, which is
//!    exactly the certain-answer semantics the axiom licenses.
//!
//! Filtration is the caller's remaining obligation, and it is tractable BECAUSE this module
//! hands back exactly which blank terms are chase-minted witnesses
//! ([`CombinedMaterialization::surrogates`]): a solution binding an OBSERVABLE variable — one
//! whose binding is projected, or is read by an aggregate, a `BIND` or a `CONSTRUCT` template
//! — to one of them is not a certain answer, because the regime draws its answers from the
//! scoping graph and a minted witness is not in it.
//!
//! What the caller must NOT do with that set is censor the returned rows. `purrdf`'s
//! `reasoning` module states the reading and carries it out: it forbids the BINDING before
//! evaluation (a `MINUS` against the witness set at every pattern leaf that binds an
//! observable variable) rather than deleting rows afterward, so `OPTIONAL` left-joins the
//! variable UNBOUND instead of losing the row, an aggregate sees the restricted sequence
//! instead of counting witnesses, and a `CONSTRUCT` template is never handed a term it must
//! not emit. Dropping whole rows lost correct answers on all three counts.
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
//! variable's witness from being mistaken for a certain answer of an OBSERVABLE one.
//!
//! # Applicability is checked, never assumed — by WHITELIST
//!
//! [`materialize_combined`] returns `Ok(None)` — "not applicable, fall back to the
//! whole-vocabulary augmentation and its own boundary" — whenever the input holds anything
//! outside what this module lowers, or when [`purrdf_datalog::chase::certify`] cannot prove
//! the resulting clause set terminating (a genuine schema-level existential cycle, e.g.
//! `A ⊑ ∃r.A`, which is a real limit of this fragment rather than an oversight).
//!
//! "Outside what this module lowers" is decided by a WHITELIST — the module's own
//! `RECOGNIZED_PREDICATES` / `RECOGNIZED_TYPES`, applied by `every_statement_is_recognized`
//! — and the direction matters. The blacklist that stood
//! here before could not support the claim the next paragraph makes and did not: it omitted
//! `rdfs:subPropertyOf`, so `r rdfs:subPropertyOf q` was neither lowered nor refused — the
//! chase never derived the `q`-edge the axiom licenses, a certain answer through `q` was
//! lost, and the run still claimed the combined approach applied. Under a whitelist an
//! unrecognized construct falls the other way: it disqualifies.
//!
//! A caller reading "combined approach: applicable" can therefore trust that EVERY statement
//! of the input was accounted for — lowered, or recognized as stating nothing (a declaration,
//! an annotation, an ordinary property assertion) — and not merely that none of a list of
//! constructs someone remembered to enumerate was present. This module still refuses a
//! PARTIAL lowering that skips just the axiom it cannot read, for the same reason.
//!
//! The caller discloses that fallback as [`crate::Construct::NonHornTBox`] on the report it
//! keeps using, by way of [`ReasoningReport::with_boundary`] — see that construct's reason
//! for the exact boundary this module's own restriction draws.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_datalog::StopSignal;
use purrdf_datalog::chase::{ChaseError, certify, chase_until};
use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct};
use purrdf_datalog::store::RelationStore;

use crate::engine::surface_of;
use crate::interner::intern_into;
use crate::owl_dl::constructs::is_reserved;
use crate::report::ReasoningReport;
use crate::vocab::{
    OWL_ANNOTATIONPROPERTY, OWL_CLASS, OWL_DATATYPEPROPERTY, OWL_NAMEDINDIVIDUAL,
    OWL_OBJECTPROPERTY, OWL_ONPROPERTY, OWL_ONTOLOGY, OWL_RESTRICTION, OWL_SOMEVALUESFROM,
    OWL_VERSIONINFO, RDF_PROPERTY, RDF_TYPE, RDFS_CLASS, RDFS_COMMENT, RDFS_DATATYPE,
    RDFS_ISDEFINEDBY, RDFS_LABEL, RDFS_SEEALSO, RDFS_SUBCLASSOF,
};
use crate::{EntailError, QTriple, materialize_dl_reported_until};

/// The reserved-vocabulary PREDICATES this module reads — the whole list, and the reason the
/// applicability claim is checkable.
///
/// This used to be the other list: a BLACKLIST of eighteen constructs whose presence
/// disqualified an ontology. A blacklist cannot support the claim the module makes, and it
/// did not: `rdfs:subPropertyOf` was absent from it, so `r rdfs:subPropertyOf q` was
/// silently ignored — the lowering emitted no clause for it, the chase never derived the
/// `q`-edge it licenses, a certain answer through `q` was lost, no boundary was raised, and
/// "combined approach: applicable" was still claimed. One entry was worse than absent:
/// `owl:disjointObjectProperties` is a FUNCTIONAL-SYNTAX axiom name and not an RDF predicate
/// at all, so it could never match anything, while the real RDF predicate for the same axiom
/// — `owl:propertyDisjointWith` — was one of the terms the list omitted.
///
/// A whitelist cannot fail that way. The lowering recognizes exactly the constructs it
/// lowers, plus the vocabulary that states nothing at all; any OTHER reserved term, in any
/// position that could carry an axiom, disqualifies the whole ontology. A construct nobody
/// thought of is therefore refused rather than dropped, which is the direction an unknown
/// has to fall for the applicability claim to mean anything.
///
/// The four load-bearing entries are `rdf:type` (a class assertion, and the restriction
/// scaffold's own typing), `rdfs:subClassOf` (the axiom), and `owl:onProperty` /
/// `owl:someValuesFrom` (the scaffold's two slots). The rest are the ANNOTATION vocabulary,
/// admitted because an annotation is not an axiom: it constrains no interpretation, so
/// leaving it unlowered leaves nothing unaccounted for.
const RECOGNIZED_PREDICATES: &[&str] = &[
    RDF_TYPE,
    RDFS_SUBCLASSOF,
    OWL_ONPROPERTY,
    OWL_SOMEVALUESFROM,
    RDFS_LABEL,
    RDFS_COMMENT,
    RDFS_SEEALSO,
    RDFS_ISDEFINEDBY,
    OWL_VERSIONINFO,
];

/// The reserved-vocabulary `rdf:type` OBJECTS this module reads.
///
/// `rdf:type` is whitelisted as a predicate because a class assertion is the chase's seed,
/// but the object decides whether the triple is an assertion or an AXIOM: `x a ex:A` is data,
/// `x a owl:Restriction` is the scaffold, and `p a owl:TransitiveProperty` is a property
/// characteristic with real logical force that this lowering does not express. Only
/// declarations and the scaffold are admitted; every other reserved class — the seven
/// property characteristics, `owl:AllDisjointClasses`, `owl:AllDifferent`,
/// `owl:NegativePropertyAssertion`, `owl:Nothing`, and anything later added to the
/// vocabulary — disqualifies.
const RECOGNIZED_TYPES: &[&str] = &[
    OWL_CLASS,
    RDFS_CLASS,
    OWL_RESTRICTION,
    OWL_OBJECTPROPERTY,
    OWL_DATATYPEPROPERTY,
    OWL_ANNOTATIONPROPERTY,
    RDF_PROPERTY,
    OWL_NAMEDINDIVIDUAL,
    OWL_ONTOLOGY,
    RDFS_DATATYPE,
];

/// Whether every statement of `ds` is one this module accounts for.
///
/// The check ranges over the WHOLE dataset, not just the default graph. The chase is seeded
/// from the default graph alone, so an axiom in a named graph is an axiom this lowering did
/// not read — and "every TBox axiom was accounted for" is either true of the input the
/// caller handed over or it is not a claim worth making. A dataset with any quad outside the
/// default graph therefore falls back, and the fallback discloses
/// [`crate::Construct::NonHornTBox`] like any other disqualification.
///
/// A predicate OUTSIDE the reserved namespaces is an ordinary property assertion: it is ABox
/// data the chase seeds from and matches, it states no axiom, and admitting it is what keeps
/// a caller's own vocabulary readable rather than turning every property into a boundary.
fn every_statement_is_recognized(ds: &RdfDataset) -> bool {
    for quad in ds.quads() {
        if quad.g.is_some() {
            return false;
        }
        let TermValue::Iri(predicate) = ds.term_value(quad.p) else {
            // A blank node or literal in predicate position is generalized RDF, which no
            // OWL 2 axiom is written in.
            return false;
        };
        if !is_reserved(&predicate) {
            continue;
        }
        if !RECOGNIZED_PREDICATES.contains(&predicate.as_str()) {
            return false;
        }
        if predicate == RDF_TYPE {
            let object = ds.term_value(quad.o);
            let TermValue::Iri(class) = &object else {
                return false;
            };
            if is_reserved(class) && !RECOGNIZED_TYPES.contains(&class.as_str()) {
                return false;
            }
        }
    }
    true
}

/// Whether `term` is a NAMED class, property or individual of the caller's own vocabulary —
/// the only kind of term the two lowered axiom shapes quantify over.
///
/// A reserved term in one of those positions is not that: `owl:Thing` and `owl:Nothing` have
/// interpretations the semantics FIXES, and `owl:topObjectProperty` /
/// `owl:bottomObjectProperty` are the two built-in roles whose extension is likewise fixed
/// (the reverse mapping raises [`crate::Construct::BuiltinRole`] for exactly that reason).
/// Lowering `A ⊑ owl:Nothing` to an ordinary Datalog rule over an ordinary class would be
/// arithmetic on a symbol whose meaning this module does not implement, so it disqualifies.
fn is_user_named_term(term: &TermValue) -> bool {
    matches!(term, TermValue::Iri(iri) if !is_reserved(iri))
}

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
    /// rather than the term itself). A solution binding an OBSERVABLE query variable to a
    /// blank node whose label is in this set is not a certain answer, and the caller's
    /// obligation is to make that binding UNREACHABLE — see the module docs for why deleting
    /// the row instead loses correct answers.
    pub surrogates: BTreeSet<String>,
}

/// Attempt the combined approach for `ds`'s basic graph pattern `query_bgp`.
///
/// Returns `Ok(None)` when the ontology's TBox is not in the fragment this module can
/// lower and chase — the caller falls back to
/// [`materialize_dl_reported`](crate::materialize_dl_reported) and discloses
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
    materialize_combined_until(ds, query_bgp, None)
}

/// [`materialize_combined`], with a caller-owned latching stop signal polled across BOTH
/// halves of the combined approach — the named part's augmentation and the anonymous part's
/// restricted chase.
///
/// It is not a budget and it changes no answer: see
/// [`materialize_until`](crate::materialize_until). A stopped run mints no witness a caller
/// can see, because it returns no [`CombinedMaterialization`] at all.
///
/// # Errors
///
/// [`EntailError::Stopped`] if the signal fired, plus every error [`materialize_combined`]
/// returns.
pub fn materialize_combined_until(
    ds: &RdfDataset,
    query_bgp: &[QTriple],
    stop: Option<&Arc<dyn StopSignal>>,
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
    let (named_dataset, report) = materialize_dl_reported_until(ds, query_bgp, stop)?;

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

    let outcome = chase_until(&clauses, edb, stop.map(|stop| &**stop as &dyn StopSignal)).map_err(
        |error| match error {
            ChaseError::Stopped { .. } => EntailError::Stopped,
            other => EntailError::Chase(other),
        },
    )?;
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

/// Lower `ds`'s default-graph TBox into a Horn `DlClause` program, or `None` if `ds` holds
/// ANY statement outside what this module accounts for.
///
/// The two shapes: `A rdfs:subClassOf B` between two NAMED classes (an atomic Datalog
/// rule `type(x, A) -> type(x, B)`), and `A rdfs:subClassOf [ a owl:Restriction ;
/// owl:onProperty p ; owl:someValuesFrom B ]` with `p` and `B` both named (an existential
/// rule `type(x, A) -> ∃y. p(x, y) ∧ type(y, B)`, one shared witness `y` per firing —
/// exactly the DL-clause shape `crate::datalog`'s existential head form was designed to
/// hold). "Named" means a term of the CALLER's own vocabulary — see [`is_user_named_term`].
///
/// Disqualification is a WHITELIST decision taken in two places, and between them they leave
/// no third answer:
///
/// * [`every_statement_is_recognized`] refuses any reserved-vocabulary statement this module
///   does not lower — a property axiom, a class axiom other than `rdfs:subClassOf`, a
///   property characteristic, an `owl:members`-based axiom, an equality assertion, or a term
///   nobody has written yet;
/// * the loop below refuses a `rdfs:subClassOf` whose sides are not both readable in the two
///   shapes — a class-expression subclass side, a restriction node carrying anything beyond
///   its type/onProperty/someValuesFrom triple, a restriction that is not a plain
///   `owl:someValuesFrom` of a named class over a named property, a literal or triple-term
///   object, or a built-in class or role in any of those positions.
///
/// See the module docs for why this module refuses a PARTIAL lowering rather than skipping
/// the one axiom it cannot read.
fn lower_horn_tbox(ds: &RdfDataset) -> Option<Vec<DlClause>> {
    if !every_statement_is_recognized(ds) {
        return None;
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
        // The axiom's SUBCLASS side has to be a class of the caller's own vocabulary: a
        // built-in class's extension is fixed by the semantics rather than by the data, so a
        // Datalog rule over it would be a rule about a symbol this module does not implement.
        if !is_user_named_term(subject_value) {
            return None;
        }
        for object in objects {
            match object {
                TermValue::Iri(object_iri) if !is_reserved(object_iri) => {
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
                    let (Some(property), Some(filler)) = (property, filler) else {
                        return None;
                    };
                    // Both slots must name a term of the caller's own vocabulary: a built-in
                    // role (`owl:topObjectProperty` and its siblings) or a built-in class has
                    // a fixed extension the existential rule below would misstate.
                    if !is_user_named_term(property) || !is_user_named_term(filler) {
                        return None;
                    }
                    let (TermValue::Iri(property_iri), TermValue::Iri(filler_iri)) =
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
                // A RESERVED IRI superclass (a built-in class), a literal, or a triple term:
                // none is a class of the caller's vocabulary, so none is in the fragment.
                TermValue::Iri(_) | TermValue::Literal { .. } | TermValue::Triple { .. } => {
                    return None;
                }
            }
        }
    }
    Some(clauses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab::{
        OWL_ALLDISJOINTCLASSES, OWL_ALLDISJOINTPROPERTIES, OWL_ASYMMETRICPROPERTY,
        OWL_DIFFERENTFROM, OWL_DISJOINTWITH, OWL_DISTINCTMEMBERS, OWL_EQUIVALENTCLASS,
        OWL_EQUIVALENTPROPERTY, OWL_FUNCTIONALPROPERTY, OWL_HASKEY, OWL_HASSELF, OWL_IMPORTS,
        OWL_INVERSEFUNCTIONALPROPERTY, OWL_INVERSEOF, OWL_IRREFLEXIVEPROPERTY, OWL_MEMBERS,
        OWL_NEGATIVEPROPERTYASSERTION, OWL_NOTHING, OWL_PROPERTYCHAINAXIOM,
        OWL_PROPERTYDISJOINTWITH, OWL_REFLEXIVEPROPERTY, OWL_SAMEAS, OWL_SYMMETRICPROPERTY,
        OWL_THING, OWL_TOPOBJECTPROPERTY, OWL_TRANSITIVEPROPERTY, RDFS_DOMAIN, RDFS_RANGE,
        RDFS_SUBPROPERTYOF, XSD_STRING,
    };

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

    // ── The whitelist: an unrecognized construct DISQUALIFIES ─────────────────────────

    /// A one-triple ontology whose single statement is `subject predicate object`, plus a
    /// class assertion so the dataset is never trivially empty.
    fn one_statement(subject: &str, predicate: &str, object: &str) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE_IRI);
        let a = b.intern_iri(&format!("{NS}A"));
        let little_a = b.intern_iri(&format!("{NS}a"));
        b.push_quad(little_a, ty, a, None);
        let s = b.intern_iri(subject);
        let p = b.intern_iri(predicate);
        let o = b.intern_iri(object);
        b.push_quad(s, p, o, None);
        b.freeze().expect("freeze")
    }

    /// `r rdfs:subPropertyOf q` — THE demonstration the blacklist could not make.
    ///
    /// The old blacklist did not name `rdfs:subPropertyOf`, so this axiom was neither lowered
    /// nor refused: the chase emitted no `q`-edge, a certain answer through `q` was lost, and
    /// the run still reported the combined approach as applicable. Under the whitelist it
    /// disqualifies, the caller falls back to the whole-vocabulary augmentation — which DOES
    /// read `rdfs:subPropertyOf` — and the answer arrives there instead.
    #[test]
    fn a_sub_property_axiom_disqualifies_the_ontology() {
        let ds = one_statement(&format!("{NS}r"), RDFS_SUBPROPERTYOF, &format!("{NS}q"));
        assert!(lower_horn_tbox(&ds).is_none());
        assert!(
            materialize_combined(&ds, &[])
                .expect("no chase error")
                .is_none()
        );
    }

    /// EVERY construct class the blacklist missed now disqualifies — one entry per class of
    /// axiom, driven through the real applicability check.
    ///
    /// `owl:propertyDisjointWith` is in this list on purpose: it is the RDF predicate for the
    /// axiom whose FUNCTIONAL-SYNTAX name (`owl:disjointObjectProperties`) the blacklist
    /// carried instead — an entry that could never match an RDF triple, standing in for the
    /// real predicate, which the blacklist omitted.
    #[test]
    fn every_formerly_missed_construct_class_disqualifies() {
        let ex = |local: &str| format!("{NS}{local}");
        // Property axioms written with a reserved PREDICATE.
        for predicate in [
            RDFS_SUBPROPERTYOF,
            RDFS_DOMAIN,
            RDFS_RANGE,
            OWL_EQUIVALENTPROPERTY,
            OWL_PROPERTYDISJOINTWITH,
            OWL_INVERSEOF,
            OWL_SAMEAS,
            OWL_DIFFERENTFROM,
            OWL_MEMBERS,
            OWL_DISTINCTMEMBERS,
            OWL_HASSELF,
            OWL_IMPORTS,
            OWL_PROPERTYCHAINAXIOM,
            OWL_HASKEY,
            OWL_EQUIVALENTCLASS,
            OWL_DISJOINTWITH,
        ] {
            let ds = one_statement(&ex("r"), predicate, &ex("q"));
            assert!(
                lower_horn_tbox(&ds).is_none(),
                "{predicate} must disqualify the ontology"
            );
            assert!(
                materialize_combined(&ds, &[])
                    .expect("no chase error")
                    .is_none(),
                "{predicate} must send the caller to the fallback"
            );
        }
        // Property CHARACTERISTICS, written as a reserved `rdf:type` OBJECT — the shape a
        // predicate whitelist alone would have admitted, because `rdf:type` is whitelisted.
        for class in [
            OWL_TRANSITIVEPROPERTY,
            OWL_SYMMETRICPROPERTY,
            OWL_ASYMMETRICPROPERTY,
            OWL_REFLEXIVEPROPERTY,
            OWL_IRREFLEXIVEPROPERTY,
            OWL_FUNCTIONALPROPERTY,
            OWL_INVERSEFUNCTIONALPROPERTY,
            OWL_ALLDISJOINTCLASSES,
            OWL_ALLDISJOINTPROPERTIES,
            OWL_NEGATIVEPROPERTYASSERTION,
            OWL_NOTHING,
        ] {
            let ds = one_statement(&ex("r"), RDF_TYPE_IRI, class);
            assert!(
                lower_horn_tbox(&ds).is_none(),
                "a {class} typing must disqualify the ontology"
            );
        }
    }

    /// A term nobody enumerated — the case a blacklist cannot decide at all.
    #[test]
    fn an_unknown_reserved_term_disqualifies() {
        let invented = "http://www.w3.org/2002/07/owl#purrdfNoSuchTerm";
        let ds = one_statement(&format!("{NS}r"), invented, &format!("{NS}q"));
        assert!(lower_horn_tbox(&ds).is_none());
        let ds = one_statement(&format!("{NS}r"), RDF_TYPE_IRI, invented);
        assert!(lower_horn_tbox(&ds).is_none());
    }

    /// The whitelist does NOT turn a caller's own vocabulary into a boundary: an ordinary
    /// property assertion and a class declaration are admitted, and the load-bearing fixture
    /// still lowers.
    #[test]
    fn ordinary_data_and_annotations_stay_in_the_fragment() {
        let ds = one_statement(&format!("{NS}a"), &format!("{NS}p"), &format!("{NS}b"));
        assert!(lower_horn_tbox(&ds).is_some());

        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE_IRI);
        let a = b.intern_iri(&format!("{NS}A"));
        let class = b.intern_iri(OWL_CLASS);
        let label = b.intern_iri(RDFS_LABEL);
        let text = b.intern_literal(purrdf_core::RdfLiteral::typed("A", XSD_STRING));
        let named_individual = b.intern_iri(OWL_NAMEDINDIVIDUAL);
        let little_a = b.intern_iri(&format!("{NS}a"));
        b.push_quad(a, ty, class, None);
        b.push_quad(a, label, text, None);
        b.push_quad(little_a, ty, named_individual, None);
        b.push_quad(little_a, ty, a, None);
        let ds = b.freeze().expect("freeze");
        assert!(
            lower_horn_tbox(&ds).is_some(),
            "a declaration and an annotation state no axiom, so they must not disqualify"
        );

        assert!(lower_horn_tbox(&some_values_from_ontology()).is_some());
    }

    /// A BUILT-IN class on either side of `rdfs:subClassOf` disqualifies: `owl:Thing`'s and
    /// `owl:Nothing`'s extensions are fixed by the semantics, and the atomic Datalog rule the
    /// lowering would emit is a rule about a symbol this module does not implement.
    #[test]
    fn a_builtin_class_in_a_subclass_axiom_disqualifies() {
        let ds = one_statement(&format!("{NS}A"), RDFS_SUBCLASSOF_IRI, OWL_NOTHING);
        assert!(lower_horn_tbox(&ds).is_none());
        let ds = one_statement(OWL_THING, RDFS_SUBCLASSOF_IRI, &format!("{NS}A"));
        assert!(lower_horn_tbox(&ds).is_none());
    }

    /// A BUILT-IN role in the restriction's `owl:onProperty` slot disqualifies, for the same
    /// reason the reverse mapping raises `Construct::BuiltinRole` for it.
    #[test]
    fn a_builtin_role_in_a_restriction_disqualifies() {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE_IRI);
        let class = b.intern_iri(OWL_CLASS);
        let subclass_of = b.intern_iri(RDFS_SUBCLASSOF_IRI);
        let a = b.intern_iri(&format!("{NS}A"));
        let big_b = b.intern_iri(&format!("{NS}B"));
        let top = b.intern_iri(OWL_TOPOBJECTPROPERTY);
        let restriction = b.intern_blank("restriction", BlankScope::DEFAULT);
        let restriction_class = b.intern_iri(OWL_RESTRICTION_IRI);
        let on_property = b.intern_iri(OWL_ON_PROPERTY);
        let some_values_from = b.intern_iri(OWL_SOME_VALUES_FROM);
        b.push_quad(a, ty, class, None);
        b.push_quad(big_b, ty, class, None);
        b.push_quad(restriction, ty, restriction_class, None);
        b.push_quad(restriction, on_property, top, None);
        b.push_quad(restriction, some_values_from, big_b, None);
        b.push_quad(a, subclass_of, restriction, None);
        let ds = b.freeze().expect("freeze");
        assert!(lower_horn_tbox(&ds).is_none());
    }

    /// A quad OUTSIDE the default graph disqualifies: the chase is seeded from the default
    /// graph alone, so an axiom in a named graph is one this lowering never read — and the
    /// applicability claim is about the input the caller handed over, not about a subgraph of
    /// it.
    #[test]
    fn a_quad_outside_the_default_graph_disqualifies() {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE_IRI);
        let a = b.intern_iri(&format!("{NS}A"));
        let little_a = b.intern_iri(&format!("{NS}a"));
        let g = b.intern_iri(&format!("{NS}g"));
        b.push_quad(little_a, ty, a, Some(g));
        let ds = b.freeze().expect("freeze");
        assert!(lower_horn_tbox(&ds).is_none());
    }
}
