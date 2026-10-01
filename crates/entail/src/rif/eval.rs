// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The RIF-Core forward-chaining ("bottom-up") rule evaluator.
//!
//! A definite Horn rule set is materialized to its least fixpoint by
//! [`purrdf_datalog::seminaive`] — this module owns no fixpoint loop of its own. Each
//! RIF rule is translated into `DlClause`s over the engine's quad-shaped relation store
//! (one clause per head atom, every atom in the default graph, a frame `o[p->v]` being
//! the triple `(o, p, v)`). The seed fact set is the source dataset's default-graph
//! triples plus the rule set's ground facts. Blank nodes are preserved by identity
//! (interned by their `(label, scope)` value, rendered to a scope-qualified surface),
//! never skolemized. Output is `original + derived`, frozen into a fresh dataset in the
//! order of the terms' first sighting — fully deterministic.
//!
//! # This lane reads the DEFAULT GRAPH, and says so
//!
//! A quad outside the default graph is copied into the answer verbatim and is not a
//! PREMISE: it seeds no rule and licenses no conclusion. That is a defined reading rather
//! than an oversight — RDF has no standard entailment relation for a dataset — and it is
//! narrower than the reading the chase engine gives its lanes, which close each
//! named graph against the union of itself and the default graph. The difference is
//! exactly the kind of thing that must not live in prose alone, so
//! [`materialize_rif`] raises [`Construct::NamedGraph`] on the run's
//! [`ReasoningReport`] whenever the input holds such a quad: the caller is told, in data,
//! that part of their input was not reasoned over.

use std::sync::Arc;

use purrdf_core::{DatasetView, FastMap, FastSet, RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_datalog::StopSignal;
use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause};
use purrdf_datalog::guard::NoGuards;
use purrdf_datalog::seminaive::{BudgetReport, EvalOptions, compile, evaluate_guarded};
use purrdf_datalog::store::RelationStore;

use crate::engine::{copy_into, evaluate_error, surface_of};
use crate::interner::{Interner, intern_into};
use crate::report::{Boundary, Construct, ReasoningReport};
use crate::rif::model::{Atom, RifTerm, Rule, RuleSet};
use crate::{EntailError, Regime};

/// The evaluator's term table: the shared [`Interner`] (whose dense first-seen ids fix the
/// emission order of the answer) plus the surface each id is stored under in
/// `purrdf-datalog`'s relation store and the reverse map that reads a derived surface back.
///
/// The surface is rendered only when the id is NEW (ids are dense and assigned in
/// first-seen order, so `id >= surfaces.len()` is exactly the first-sighting test), so a
/// repeated term costs a hash lookup and nothing else.
#[derive(Default)]
struct Terms {
    /// The shared `TermValue → u32` table.
    interner: Interner,
    /// The store surface of each id, by id.
    surfaces: Vec<String>,
    /// The id of each store surface.
    by_surface: FastMap<String, u32>,
}

impl Terms {
    /// Intern `value`, returning its dense id and recording its surface on first sight.
    fn intern(&mut self, value: TermValue) -> u32 {
        let id = self.interner.intern(value);
        if id as usize >= self.surfaces.len() {
            let surface = surface_of(self.interner.value(id));
            self.by_surface.insert(surface.clone(), id);
            self.surfaces.push(surface);
        }
        id
    }

    /// The `TermValue` behind an id.
    fn value(&self, id: u32) -> &TermValue {
        self.interner.value(id)
    }

    /// The store surface of an id.
    fn surface(&self, id: u32) -> &str {
        &self.surfaces[id as usize]
    }

    /// The id a store surface was interned under.
    fn id_of_surface(&self, surface: &str) -> Option<u32> {
        self.by_surface.get(surface).copied()
    }
}

/// Materialize the RIF rule set over `ds`, returning `original quads + derived
/// triples` AND the [`ReasoningReport`] for the run.
///
/// The seed facts are `ds`'s default-graph triples plus `rules.facts`; the Horn
/// rules are forward-chained to a fixpoint. The result holds every original quad
/// (all graphs) plus every seeded or derived fact not already an original
/// default-graph triple, frozen into a new dataset.
///
/// # The report is not optional here either
///
/// There is no report-free variant of this function, for the reason
/// [`materialize`](crate::materialize) has none: a quad outside the default graph is
/// copied to the answer and reasoned over by NOTHING, and a signature with nowhere to say
/// so turns that into silence. [`Construct::NamedGraph`] is raised whenever the input holds
/// such a quad, so "the closure of this dataset" and "the closure of this dataset's default
/// graph, with the rest carried through untouched" stop being the same return value.
///
/// A triple term is NOT a boundary of this lane. The chase lanes raise
/// [`Construct::TripleTerm`] because `rdfs14`/`rdfs14a` are rules they state and cannot
/// fire; this evaluator states no rule of its own, and a caller's rule that names a triple
/// term as a constant matches it exactly like any other term.
///
/// # What the report can and cannot say about the RULES
///
/// [`ReasoningReport::rules_fired`] is empty for every RIF run, and
/// [`ReasoningReport::contract_hash`] is the hash of `calculus_program(Regime::Rif)` — the
/// EMPTY declared program — rather than a digest of `rules`. Both follow from the same
/// fact: the rule set is the CALLER's, and this crate mints neither a [`RuleId`](crate::RuleId)
/// for a rule it did not declare nor an identity for a document it did not author. So the
/// contract hash of a RIF run identifies the LANE, not the rule set, and two runs under
/// different rule sets carry the same hash; a consumer who needs to refuse a closure minted
/// under different rules must digest the rule document they supplied.
/// [`ReasoningReport::budget`] is measured rather than stubbed: candidate facts enumerated
/// by the joins, facts held at the fixpoint, and interned term surface bytes.
///
/// # Errors
///
/// [`EntailError::Parse`] if a rule is not range-restricted, and [`EntailError::Build`] if
/// the derived dataset cannot be frozen. Those are the only two: this lane's rule set is
/// definite Horn, so nothing in it can derive an inconsistency, and a report that
/// contradicts its own evidence is unrepresentable rather than refused — completeness is
/// computed from the boundary list by [`ReasoningReport::completeness`] and is not a field
/// that could disagree with it.
pub fn materialize_rif<D: DatasetView>(
    ds: &D,
    rules: &RuleSet,
) -> Result<(Arc<RdfDataset>, ReasoningReport), EntailError> {
    materialize_rif_until(ds, rules, None)
}

/// [`materialize_rif`], with a caller-owned latching stop signal polled once per semi-naive
/// round of the RIF fixpoint.
///
/// It is not a budget and it changes no answer: see
/// [`materialize_until`](crate::materialize_until) for the argument, which is this lane's
/// too. A run the signal never stops derives exactly what [`materialize_rif`] derives; a run
/// it stops derives nothing a caller can read.
///
/// # Errors
///
/// [`EntailError::Stopped`] if the signal fired, plus every error [`materialize_rif`]
/// returns.
pub fn materialize_rif_until<D: DatasetView>(
    ds: &D,
    rules: &RuleSet,
    stop: Option<&Arc<dyn StopSignal>>,
) -> Result<(Arc<RdfDataset>, ReasoningReport), EntailError> {
    materialize_rif_with(ds, rules, &EvalOptions::default(), stop)
}

/// [`materialize_rif_until`] under the caller's governors.
///
/// `options` carries the stored-fact and join-step limits of the semi-naive engine the
/// fixpoint runs on ([`EvalOptions::with_max_stored_facts`],
/// [`EvalOptions::with_max_join_steps`]), exactly as the chase lanes take them through
/// [`materialize_with`](crate::materialize_with). A limit only ever REFUSES: a run inside
/// the limits derives the same closure under every setting, and a run past one is
/// [`EntailError::Evaluate`] naming the limit — never a truncated closure.
///
/// # Errors
///
/// [`EntailError::Evaluate`] if the run passes a limit, plus every error
/// [`materialize_rif_until`] returns.
pub fn materialize_rif_with<D: DatasetView>(
    ds: &D,
    rules: &RuleSet,
    options: &EvalOptions,
    stop: Option<&Arc<dyn StopSignal>>,
) -> Result<(Arc<RdfDataset>, ReasoningReport), EntailError> {
    ds.checked_read(|ds| {
        // Refuse a run that was already stopped before interning source terms, ground facts,
        // or compiled rules. The round-level checks below remain the mid-fixpoint boundary.
        if stop.is_some_and(|signal| signal.stopped()) {
            return Err(EntailError::Stopped);
        }
        let mut terms = Terms::default();

        // Seed: the source dataset's default-graph triples, in dataset order. A quad outside
        // it is not a premise, and the boundary below is where the run says so.
        let mut named_graph = false;
        let mut edb = RelationStore::new();
        let mut original: FastSet<[u32; 3]> = FastSet::default();
        for q in ds.quads() {
            if q.g.is_some() {
                named_graph = true;
                continue; // entailment operates over the default graph
            }
            let s = terms.intern(ds.term_value(q.s)?);
            let p = terms.intern(ds.term_value(q.p)?);
            let o = terms.intern(ds.term_value(q.o)?);
            seed_fact(&mut edb, &terms, [s, p, o]);
            original.insert([s, p, o]);
        }

        // Seed: the rule set's ground facts (imported RDF + ground frames).
        for (s, p, o) in &rules.facts {
            let s = terms.intern(s.clone());
            let p = terms.intern(p.clone());
            let o = terms.intern(o.clone());
            seed_fact(&mut edb, &terms, [s, p, o]);
        }

        // Translate every rule into engine clauses, interning constants as they are met.
        let mut clauses: Vec<DlClause> = Vec::new();
        for rule in &rules.rules {
            clauses.extend(translate_rule(rule, &mut terms)?);
        }

        // The fixpoint is `purrdf-datalog`'s. The stop signal is polled at its round
        // boundaries; `Stopped` out means there is no partial fixpoint to hand back.
        let executable = compile(clauses).map_err(evaluate_error)?;
        let evaluation = evaluate_guarded(
            &executable,
            edb,
            &NoGuards,
            options,
            stop.map(|signal| &**signal),
        )
        .map_err(evaluate_error)?;

        // Emit: original quads (all graphs) + every seeded/derived fact that is not an
        // original default-graph triple, in a deterministic order.
        let mut b = RdfDatasetBuilder::new();
        // The original quads are copied verbatim, preserving blank-node scopes, so a derived
        // fact naming one of the input's blank nodes lands on the SAME term the copy carries —
        // `push_dataset` would have re-scoped the input and split the two apart.
        copy_into(&mut b, ds)?;
        // Sort the model by its interned term ids to get the deterministic first-seen
        // emission order.
        let mut ordered: Vec<[u32; 3]> = evaluation
            .facts()
            .facts_sorted()
            .iter()
            .map(|fact| {
                let id = |surface: &str| {
                    terms
                        .id_of_surface(surface)
                        .expect("the evaluator mints no terms, so every surface was interned")
                };
                [id(&fact.subject), id(&fact.predicate), id(&fact.object)]
            })
            .collect();
        ordered.sort_unstable();
        for t in ordered {
            if original.contains(&t) {
                continue;
            }
            let s = intern_into(&mut b, terms.value(t[0]));
            let p = intern_into(&mut b, terms.value(t[1]));
            let o = intern_into(&mut b, terms.value(t[2]));
            b.push_quad(s, p, o, None);
        }
        let closure = b.freeze().map_err(|e| EntailError::Build(e.to_string()))?;
        Ok((closure, rif_report(named_graph, evaluation.budget())))
    })
    .map_err(EntailError::source_read)?
}

/// Assemble the report for a RIF run whose fixpoint consumed `budget`.
///
/// The budget is the evaluator's own measurement — join steps, facts held at the fixpoint
/// and interned term surface bytes — not a second tally kept beside it.
fn rif_report(named_graph: bool, budget: BudgetReport) -> ReasoningReport {
    let boundaries: Vec<Boundary> = if named_graph {
        vec![Boundary::of(Construct::NamedGraph)]
    } else {
        Vec::new()
    };
    ReasoningReport::new(
        Regime::Rif,
        // The rules are the caller's and carry no `RuleId` this crate declares.
        Vec::new(),
        boundaries,
        budget,
        // A definite Horn rule set has no `false` head: nothing in this lane can derive an
        // inconsistency, so `None` here is a statement about the fragment rather than an
        // unfilled field.
        None,
        // The evaluator mints no term — a head variable not bound by the body is refused at
        // translation time — so there is no surrogate to withhold.
        0,
        // …and a rule set that invents no term needs no termination proof: this lane's
        // fixpoint is bounded by the active domain, so there is no acyclicity analysis to
        // report the verdict of.
        None,
    )
}

/// Insert one interned triple into the seed relation store's default graph.
fn seed_fact(edb: &mut RelationStore, terms: &Terms, [s, p, o]: [u32; 3]) {
    let _ = edb.insert(
        terms.surface(s),
        terms.surface(p),
        terms.surface(o),
        RelationStore::DEFAULT_GRAPH,
    );
}

/// Translate one RIF rule into engine clauses, one per head atom (each sharing the body).
///
/// A rule with an empty body derives nothing — the pre-existing lane semantics, kept here
/// rather than letting the engine fire a bodiless clause once — so it yields no clause.
///
/// # Errors
///
/// [`EntailError::Parse`] if the rule is not range-restricted (datalog safety):
/// a head variable that never appears in the body has no binding source, so the
/// rule is malformed rather than silently deriving an unbound term.
fn translate_rule(rule: &Rule, terms: &mut Terms) -> Result<Vec<DlClause>, EntailError> {
    let body_vars: FastSet<&str> = rule.body.iter().flat_map(atom_var_names).collect();
    for name in rule.head.iter().flat_map(atom_var_names) {
        if !body_vars.contains(name) {
            return Err(EntailError::Parse(format!(
                "RIF rule head variable ?{name} is not range-restricted \
                 (not bound by the rule body)"
            )));
        }
    }
    let body: Vec<ClauseAtom> = rule.body.iter().map(|a| clause_atom(a, terms)).collect();
    let head: Vec<ClauseAtom> = rule.head.iter().map(|a| clause_atom(a, terms)).collect();
    if body.is_empty() {
        return Ok(Vec::new());
    }
    Ok(head
        .into_iter()
        .map(|atom| DlClause::datalog(atom, body.clone()))
        .collect())
}

/// The variable names appearing in an atom's three slots, in slot order.
fn atom_var_names(atom: &Atom) -> impl Iterator<Item = &str> {
    [&atom.s, &atom.p, &atom.o]
        .into_iter()
        .filter_map(|t| match t {
            RifTerm::Var(name) => Some(name.as_str()),
            RifTerm::Const(_) => None,
        })
}

/// One RIF triple pattern as a default-graph engine atom.
fn clause_atom(atom: &Atom, terms: &mut Terms) -> ClauseAtom {
    ClauseAtom::quad(
        clause_term(&atom.s, terms),
        clause_term(&atom.p, terms),
        clause_term(&atom.o, terms),
        ClauseTerm::DefaultGraph,
    )
}

/// One RIF slot as an engine term. A constant is interned (fixing its emission order and
/// its readable surface) and carried as its exact store surface: an IRI whose bracketed
/// spelling is that surface as the IRI variant, every other term as a literal surface.
fn clause_term(term: &RifTerm, terms: &mut Terms) -> ClauseTerm {
    match term {
        RifTerm::Var(name) => ClauseTerm::var(name.clone()),
        RifTerm::Const(value) => {
            let id = terms.intern(value.clone());
            let surface = terms.surface(id);
            match value {
                TermValue::Iri(iri)
                    if surface.strip_prefix('<').and_then(|s| s.strip_suffix('>'))
                        == Some(iri.as_str()) =>
                {
                    ClauseTerm::iri(iri.clone())
                }
                _ => ClauseTerm::literal(surface.to_owned()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rif::model::Fact;
    use purrdf_core::{RdfDatasetBuilder, TermValue};

    const EX: &str = "http://example.org/ns#";

    fn iri(local: &str) -> TermValue {
        TermValue::iri(format!("{EX}{local}"))
    }

    fn var(name: &str) -> RifTerm {
        RifTerm::Var(name.to_owned())
    }

    fn con(v: TermValue) -> RifTerm {
        RifTerm::Const(v)
    }

    fn atom(s: RifTerm, p: RifTerm, o: RifTerm) -> Atom {
        Atom { s, p, o }
    }

    fn empty_ds() -> Arc<RdfDataset> {
        RdfDatasetBuilder::new().freeze().expect("freeze")
    }

    #[derive(Debug)]
    struct AlreadyStopped;

    impl StopSignal for AlreadyStopped {
        fn stopped(&self) -> bool {
            true
        }
    }

    #[test]
    fn an_already_stopped_rif_run_refuses_before_preparation() {
        let stop: Arc<dyn StopSignal> = Arc::new(AlreadyStopped);
        let result = materialize_rif_until(&empty_ds(), &RuleSet::default(), Some(&stop));
        assert!(matches!(result, Err(EntailError::Stopped)));
    }

    fn has(ds: &RdfDataset, s: &TermValue, p: &TermValue, o: &TermValue) -> bool {
        ds.quads().any(|q| {
            q.g.is_none()
                && &ds.term_value(q.s) == s
                && &ds.term_value(q.p) == p
                && &ds.term_value(q.o) == o
        })
    }

    #[test]
    fn uncle_rule_forward_chains() {
        // parent(x,y) ∧ brother(y,z) ⇒ uncle(x,z).
        let rule = Rule {
            body: vec![
                atom(var("x"), con(iri("parent")), var("y")),
                atom(var("y"), con(iri("brother")), var("z")),
            ],
            head: vec![atom(var("x"), con(iri("uncle")), var("z"))],
        };
        let rules = RuleSet {
            facts: vec![
                (iri("Emeka"), iri("parent"), iri("Okechukwu")),
                (iri("Okechukwu"), iri("brother"), iri("Chijoke")),
            ],
            rules: vec![rule],
        };
        let (out, report) = materialize_rif(&empty_ds(), &rules).expect("materialize");
        assert!(
            has(&out, &iri("Emeka"), &iri("uncle"), &iri("Chijoke")),
            "derived Emeka uncle Chijoke"
        );
        // The report is a measurement of THIS run, not a template: the joins enumerated
        // candidates, the store held facts, and the terms occupy bytes.
        assert_eq!(report.regime(), Regime::Rif);
        assert!(report.budget().join_steps() > 0);
        assert!(report.budget().stored_facts() >= 3);
        assert!(report.budget().term_arena_bytes() > 0);
    }

    #[test]
    fn frames_discount_rule() {
        // status "gold" ⇒ discount 10 ; the silver rule must not fire.
        let xsd_string = "http://www.w3.org/2001/XMLSchema#string";
        let xsd_int = "http://www.w3.org/2001/XMLSchema#integer";
        let gold = TermValue::typed_literal("gold", xsd_string);
        let silver = TermValue::typed_literal("silver", xsd_string);
        let ten = TermValue::typed_literal("10", xsd_int);
        let five = TermValue::typed_literal("5", xsd_int);
        let rules = RuleSet {
            facts: vec![(iri("customer017"), iri("status"), gold.clone())],
            rules: vec![
                Rule {
                    body: vec![atom(var("c"), con(iri("status")), con(gold))],
                    head: vec![atom(var("c"), con(iri("discount")), con(ten.clone()))],
                },
                Rule {
                    body: vec![atom(var("c"), con(iri("status")), con(silver))],
                    head: vec![atom(var("c"), con(iri("discount")), con(five.clone()))],
                },
            ],
        };
        let (out, report) = materialize_rif(&empty_ds(), &rules).expect("materialize");
        assert!(
            has(&out, &iri("customer017"), &iri("discount"), &ten),
            "gold ⇒ discount 10"
        );
        assert!(
            !has(&out, &iri("customer017"), &iri("discount"), &five),
            "silver rule must not fire"
        );
        // A default-graph-only input met no construct this lane could not handle.
        assert_eq!(report.boundaries(), []);
        assert_eq!(report.completeness(), crate::Completeness::Exact);
    }

    /// A QUAD OUTSIDE THE DEFAULT GRAPH IS NO LONGER DISCARDED IN SILENCE.
    ///
    /// It is still not a premise — this lane reads the default graph — and that is now a
    /// fact the caller can read off the run rather than one buried in a `continue`. The
    /// closure still carries the quad verbatim, so the boundary is about what was REASONED
    /// OVER, not about what was kept.
    #[test]
    fn a_named_graph_quad_raises_the_boundary_rather_than_vanishing() {
        let mut b = RdfDatasetBuilder::new();
        let emeka = b.intern_iri(&format!("{EX}Emeka"));
        let parent = b.intern_iri(&format!("{EX}parent"));
        let oke = b.intern_iri(&format!("{EX}Okechukwu"));
        let brother = b.intern_iri(&format!("{EX}brother"));
        let chijoke = b.intern_iri(&format!("{EX}Chijoke"));
        let g = b.intern_iri(&format!("{EX}g"));
        b.push_quad(emeka, parent, oke, None);
        b.push_quad(oke, brother, chijoke, Some(g));
        let ds = b.freeze().expect("freeze");

        let rules = RuleSet {
            facts: Vec::new(),
            rules: vec![Rule {
                body: vec![
                    atom(var("x"), con(iri("parent")), var("y")),
                    atom(var("y"), con(iri("brother")), var("z")),
                ],
                head: vec![atom(var("x"), con(iri("uncle")), var("z"))],
            }],
        };
        let (out, report) = materialize_rif(&ds, &rules).expect("materialize");

        // The premise in the named graph did not license the conclusion…
        assert!(
            !has(&out, &iri("Emeka"), &iri("uncle"), &iri("Chijoke")),
            "a named-graph quad is not a premise of this lane"
        );
        // …and the run SAYS so, naming the construct and carrying its reason.
        let constructs: Vec<Construct> = report
            .boundaries()
            .iter()
            .map(|boundary| boundary.construct())
            .collect();
        assert_eq!(constructs, vec![Construct::NamedGraph]);
        assert_ne!(report.boundaries()[0].reason(), "");
        // A boundary beside a rule table that has nothing missing is
        // `ExactWithinBoundaries`, never plain `Exact`: the completeness is DERIVED from
        // this very boundary list, so the two cannot come apart.
        assert_eq!(
            report.completeness(),
            crate::Completeness::ExactWithinBoundaries
        );
        // The quad itself is still in the answer: the boundary is about premises.
        assert!(
            out.quads()
                .any(|q| q.g.is_some() && out.term_value(q.p).unwrap() == iri("brother")),
            "the named-graph quad is carried through"
        );
        // Determinism: the same input renders the same report, field for field.
        let (_, again) = materialize_rif(&ds, &rules).expect("materialize");
        assert_eq!(format!("{report:?}"), format!("{again:?}"));
    }

    /// A default-graph-only run raises NOTHING — the boundary is evidence about an input,
    /// not a standing disclaimer.
    #[test]
    fn a_default_graph_run_raises_no_boundary() {
        let rules = RuleSet {
            facts: vec![(iri("Emeka"), iri("parent"), iri("Okechukwu"))],
            rules: Vec::new(),
        };
        let (_, report) = materialize_rif(&empty_ds(), &rules).expect("materialize");
        assert_eq!(report.boundaries(), []);
    }

    #[test]
    fn unbound_head_variable_is_rejected() {
        // parent(x,y) ⇒ uncle(x,z): ?z is in the head but never bound by the body,
        // so the rule is not range-restricted and must be a typed Parse error — not
        // a panic — when materialized over untrusted input.
        let rule = Rule {
            body: vec![atom(var("x"), con(iri("parent")), var("y"))],
            head: vec![atom(var("x"), con(iri("uncle")), var("z"))],
        };
        let rules = RuleSet {
            facts: vec![(iri("Emeka"), iri("parent"), iri("Okechukwu"))],
            rules: vec![rule],
        };
        let err = materialize_rif(&empty_ds(), &rules).expect_err("unbound head variable");
        // The refusal is typed; nothing is materialized and nothing is reported, because
        // there was no run.
        match err {
            EntailError::Parse(msg) => {
                assert!(
                    msg.contains("?z"),
                    "message names the offending variable: {msg}"
                );
            }
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn indexed_joins_avoid_cartesian_fact_scans() {
        // 1,000 `common` edges and one `rare` edge: a join that scanned the Cartesian
        // product would enumerate a million candidates; the engine's index-driven plan
        // inspects a small multiple of the fact count.
        let mut facts: Vec<Fact> = (0..1_000)
            .map(|n| {
                (
                    iri(&format!("n{n}")),
                    iri("common"),
                    iri(&format!("n{}", n + 1)),
                )
            })
            .collect();
        facts.push((iri("n500"), iri("rare"), iri("far")));
        let rules = RuleSet {
            facts,
            rules: vec![Rule {
                body: vec![
                    atom(var("x"), con(iri("common")), var("y")),
                    atom(var("y"), con(iri("rare")), var("z")),
                ],
                head: vec![atom(var("x"), con(iri("derived")), var("z"))],
            }],
        };
        let (out, report) = materialize_rif(&empty_ds(), &rules).expect("materialize");
        assert!(has(&out, &iri("n499"), &iri("derived"), &iri("far")));
        assert!(
            report.budget().join_steps() < 5_000,
            "indexed joins should inspect thousands, not the million-row Cartesian product: {}",
            report.budget().join_steps()
        );
    }

    #[test]
    fn a_repeated_variable_binds_one_value() {
        // `?x loves ?x` matches only the reflexive fact; the neighbouring non-reflexive
        // fact must not fire the rule.
        let rules = RuleSet {
            facts: vec![
                (iri("a"), iri("loves"), iri("a")),
                (iri("b"), iri("loves"), iri("c")),
            ],
            rules: vec![Rule {
                body: vec![atom(var("x"), con(iri("loves")), var("x"))],
                head: vec![atom(var("x"), con(iri("narcissist")), con(iri("yes")))],
            }],
        };
        let (out, _) = materialize_rif(&empty_ds(), &rules).expect("materialize");
        assert!(has(&out, &iri("a"), &iri("narcissist"), &iri("yes")));
        assert!(!has(&out, &iri("b"), &iri("narcissist"), &iri("yes")));
        assert!(!has(&out, &iri("c"), &iri("narcissist"), &iri("yes")));
    }

    #[test]
    fn a_variable_predicate_and_a_recursive_rule_reach_the_fixpoint() {
        // Transitive closure over a chain, plus a variable-predicate copy rule.
        let rules = RuleSet {
            facts: vec![
                (iri("a"), iri("next"), iri("b")),
                (iri("b"), iri("next"), iri("c")),
                (iri("c"), iri("next"), iri("d")),
            ],
            rules: vec![
                Rule {
                    body: vec![
                        atom(var("x"), con(iri("next")), var("y")),
                        atom(var("y"), con(iri("reach")), var("z")),
                    ],
                    head: vec![atom(var("x"), con(iri("reach")), var("z"))],
                },
                Rule {
                    body: vec![atom(var("x"), con(iri("next")), var("y"))],
                    head: vec![atom(var("x"), con(iri("reach")), var("y"))],
                },
                Rule {
                    body: vec![atom(var("x"), var("p"), var("y"))],
                    head: vec![atom(var("y"), con(iri("seen")), var("p"))],
                },
            ],
        };
        let (out, _) = materialize_rif(&empty_ds(), &rules).expect("materialize");
        assert!(has(&out, &iri("a"), &iri("reach"), &iri("d")));
        assert!(has(&out, &iri("d"), &iri("seen"), &iri("next")));
        assert!(has(&out, &iri("d"), &iri("seen"), &iri("reach")));
    }

    #[test]
    fn a_bodiless_rule_derives_nothing() {
        let rules = RuleSet {
            facts: Vec::new(),
            rules: vec![Rule {
                body: Vec::new(),
                head: vec![atom(con(iri("a")), con(iri("p")), con(iri("b")))],
            }],
        };
        let (out, _) = materialize_rif(&empty_ds(), &rules).expect("materialize");
        assert!(!has(&out, &iri("a"), &iri("p"), &iri("b")));
    }

    /// A chain of `n` edges closed transitively: `n(n+1)/2` `reach` facts.
    fn chain_closure_rules(n: u32) -> RuleSet {
        RuleSet {
            facts: (0..n)
                .map(|i| {
                    (
                        iri(&format!("n{i}")),
                        iri("edge"),
                        iri(&format!("n{}", i + 1)),
                    )
                })
                .collect(),
            rules: vec![
                Rule {
                    body: vec![atom(var("x"), con(iri("edge")), var("y"))],
                    head: vec![atom(var("x"), con(iri("reach")), var("y"))],
                },
                Rule {
                    body: vec![
                        atom(var("x"), con(iri("reach")), var("y")),
                        atom(var("y"), con(iri("edge")), var("z")),
                    ],
                    head: vec![atom(var("x"), con(iri("reach")), var("z"))],
                },
            ],
        }
    }

    /// THE VALID NEIGHBOUR of the limit refusal: a legitimately large program still runs
    /// under the DEFAULT limits. A 300-edge chain derives 300*301/2 = 45,150 `reach` facts.
    #[test]
    fn a_large_legitimate_closure_succeeds_under_the_default_limits() {
        let n = 300;
        let started = std::time::Instant::now();
        let (out, report) = materialize_rif(&empty_ds(), &chain_closure_rules(n)).expect("closure");
        let derived = out
            .quads()
            .filter(|q| out.term_value(q.p).unwrap() == iri("reach"))
            .count();
        eprintln!(
            "rif chain n={n}: derived {derived} in {:?}",
            started.elapsed()
        );
        assert_eq!(derived, 45_150);
        assert!(report.budget().stored_facts() >= 45_150 + 300);
    }

    /// THE INVALID CASE: a run past a limit is a typed `Evaluate` refusal naming the limit.
    #[test]
    fn a_run_past_a_limit_is_refused_and_a_raised_limit_admits_it() {
        let rules = chain_closure_rules(300);
        let tight = EvalOptions::default().with_max_stored_facts(1_000);
        let err = materialize_rif_with(&empty_ds(), &rules, &tight, None)
            .expect_err("45,150 facts exceed a 1,000-fact limit");
        match err {
            EntailError::Evaluate(inner) => {
                let text = inner.to_string();
                assert!(text.contains("stored"), "message names the limit: {text}");
            }
            other => panic!("expected Evaluate, got {other:?}"),
        }
        let steps = EvalOptions::default().with_max_join_steps(1_000);
        assert!(matches!(
            materialize_rif_with(&empty_ds(), &rules, &steps, None),
            Err(EntailError::Evaluate(_))
        ));
        // The same program under a raised limit is admitted, with the same closure.
        let raised = EvalOptions::default().with_max_stored_facts(1 << 24);
        let (out, _) = materialize_rif_with(&empty_ds(), &rules, &raised, None).expect("raised");
        let (baseline, _) = materialize_rif(&empty_ds(), &rules).expect("default");
        assert_eq!(out.quads().count(), baseline.quads().count());
    }
}
