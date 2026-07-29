// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RIF-Core forward-chaining ("bottom-up") rule evaluator.
//!
//! A definite Horn rule set is materialized to its least fixpoint by a
//! deterministic semi-naive chase over interned `u32` triple ids. The seed fact set is the source
//! dataset's default-graph triples plus the rule set's ground facts; each round
//! fires every rule where at least one body atom can bind a *frontier* (newly
//! derived) fact, joining the remaining atoms against the whole accumulated set.
//! The next frontier is the round's genuinely-new triples; the chase halts when
//! the frontier empties. Blank nodes are preserved by identity (interned by their
//! `(label, scope)` value), never skolemized. Output is `original + derived`,
//! frozen into a fresh dataset — fully deterministic.
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

use purrdf_core::{FastMap, FastSet, RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_datalog::seminaive::BudgetReport;

use crate::engine::surface_of;
use crate::interner::{Interner, intern_into};
use crate::report::{Boundary, Construct, ReasoningReport};
use crate::rif::model::{Atom, RifTerm, RuleSet};
use crate::{EntailError, Regime};

/// One triple-pattern slot compiled against a rule's local variable table: a
/// bound term id, or a variable's dense local index.
#[derive(Clone, Copy)]
enum Slot {
    /// A ground term, pre-interned to its id.
    Const(u32),
    /// A variable, by its per-rule local index.
    Var(usize),
}

/// A triple pattern compiled for matching.
#[derive(Clone, Copy)]
struct PatternAtom {
    s: Slot,
    p: Slot,
    o: Slot,
}

/// A rule compiled against interned terms and a dense per-rule variable table.
struct CompiledRule {
    body: Vec<PatternAtom>,
    head: Vec<PatternAtom>,
    /// Number of distinct variables (the binding vector's length).
    num_vars: usize,
}

/// A partial variable binding: `slot i` holds the term id bound to local var `i`,
/// or `None` if still free.
type Binding = Vec<Option<u32>>;

/// Append-only fact rows plus one posting list per RDF position. Candidate order
/// follows fact insertion order, so indexing changes work performed, not result
/// determinism.
#[derive(Default)]
struct FactIndex {
    facts: Vec<[u32; 3]>,
    by_subject: FastMap<u32, Vec<usize>>,
    by_predicate: FastMap<u32, Vec<usize>>,
    by_object: FastMap<u32, Vec<usize>>,
}

impl FactIndex {
    fn from_facts(facts: Vec<[u32; 3]>) -> Self {
        let mut index = Self::default();
        for fact in facts {
            index.push(fact);
        }
        index
    }

    fn push(&mut self, fact: [u32; 3]) {
        let ordinal = self.facts.len();
        self.facts.push(fact);
        self.by_subject.entry(fact[0]).or_default().push(ordinal);
        self.by_predicate.entry(fact[1]).or_default().push(ordinal);
        self.by_object.entry(fact[2]).or_default().push(ordinal);
    }

    fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// Drop all facts and postings while retaining allocated capacity, so the
    /// next chase iteration's frontier can be rebuilt without reallocating.
    fn clear(&mut self) {
        self.facts.clear();
        self.by_subject.clear();
        self.by_predicate.clear();
        self.by_object.clear();
    }

    /// The shortest posting list selected by constants and already-bound
    /// variables. `None` means no slot is bound and the caller scans all facts.
    fn candidate_ordinals(&self, atom: &PatternAtom, binding: &Binding) -> Option<&[usize]> {
        let mut best: Option<&[usize]> = None;
        for (slot, postings) in [
            (atom.s, &self.by_subject),
            (atom.p, &self.by_predicate),
            (atom.o, &self.by_object),
        ] {
            let Some(value) = bound_value(slot, binding) else {
                continue;
            };
            let candidate = postings.get(&value).map_or(&[][..], Vec::as_slice);
            if best.is_none_or(|current| candidate.len() < current.len()) {
                best = Some(candidate);
            }
        }
        best
    }

    fn estimate(&self, atom: &PatternAtom, binding: &Binding) -> usize {
        self.candidate_ordinals(atom, binding)
            .map_or(self.facts.len(), <[usize]>::len)
    }
}

#[derive(Default)]
struct ChaseStats {
    candidate_facts_examined: usize,
}

/// The evaluator's term table, plus the one occupancy figure the report must state.
///
/// A thin wrapper over [`Interner`] rather than a second interner: every id still comes
/// from the same table in the same first-seen order, so nothing about the evaluation
/// moves. What it adds is the byte tally
/// [`BudgetReport::term_arena_bytes`] is defined as — interned term SURFACE bytes, under
/// `purrdf-datalog`'s own definition of the coordinate, measured with the same
/// [`surface_of`] rendering the chase lanes' store uses. Reporting a zero there would be a
/// misreport rather than a modest one: this lane really does hold interned terms.
///
/// The surface is rendered only when the id is NEW (ids are dense and assigned in
/// first-seen order, so `id >= interned` is exactly the first-sighting test), so a repeated
/// term costs a hash lookup and nothing else.
#[derive(Default)]
struct Terms {
    /// The shared `TermValue → u32` table.
    interner: Interner,
    /// How many ids have been handed out.
    interned: u32,
    /// Interned term surface bytes.
    surface_bytes: usize,
}

impl Terms {
    /// Intern `value`, returning its dense id and tallying its surface on first sight.
    fn intern(&mut self, value: TermValue) -> u32 {
        let id = self.interner.intern(value);
        if id >= self.interned {
            self.interned = id + 1;
            self.surface_bytes += surface_of(self.interner.value(id)).len();
        }
        id
    }

    /// The `TermValue` behind an id.
    fn value(&self, id: u32) -> &TermValue {
        self.interner.value(id)
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
/// [`EntailError::Parse`] if a rule is not range-restricted; [`EntailError::Build`] if the
/// derived dataset cannot be frozen; [`EntailError::Overclaim`] if the assembled report
/// contradicts its own evidence (the same gate [`materialize`](crate::materialize) applies).
pub fn materialize_rif(
    ds: &RdfDataset,
    rules: &RuleSet,
) -> Result<(Arc<RdfDataset>, ReasoningReport), EntailError> {
    let mut terms = Terms::default();

    // Seed: the source dataset's default-graph triples, in dataset order. A quad outside
    // it is not a premise, and the boundary below is where the run says so.
    let mut named_graph = false;
    let mut facts: FastSet<[u32; 3]> = FastSet::default();
    let mut seed: Vec<[u32; 3]> = Vec::new();
    for q in ds.quads() {
        if q.g.is_some() {
            named_graph = true;
            continue; // entailment operates over the default graph
        }
        let s = terms.intern(ds.term_value(q.s));
        let p = terms.intern(ds.term_value(q.p));
        let o = terms.intern(ds.term_value(q.o));
        push_fact(&mut facts, &mut seed, [s, p, o]);
    }
    let original: FastSet<[u32; 3]> = facts.clone();

    // Seed: the rule set's ground facts (imported RDF + ground frames).
    for (s, p, o) in &rules.facts {
        let s = terms.intern(s.clone());
        let p = terms.intern(p.clone());
        let o = terms.intern(o.clone());
        push_fact(&mut facts, &mut seed, [s, p, o]);
    }

    // Compile every rule against interned terms and a dense variable table.
    let compiled: Vec<CompiledRule> = rules
        .rules
        .iter()
        .map(|r| compile_rule(r, &mut terms))
        .collect::<Result<Vec<_>, _>>()?;

    let stats = chase(&mut facts, seed, &compiled);

    // Emit: original quads (all graphs) + every seeded/derived fact that is not an
    // original default-graph triple, in a deterministic order.
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    // Set iteration order is not stable across runs, so sort the accumulated
    // facts by their interned term ids to get a deterministic (not insertion-order)
    // emission order.
    let mut ordered: Vec<[u32; 3]> = facts.iter().copied().collect();
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
    Ok((closure, rif_report(named_graph, &facts, &terms, &stats)))
}

/// Assemble the report for a RIF run that held `facts` and consumed `stats`.
fn rif_report(
    named_graph: bool,
    facts: &FastSet<[u32; 3]>,
    terms: &Terms,
    stats: &ChaseStats,
) -> ReasoningReport {
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
        BudgetReport::new(
            u64::try_from(stats.candidate_facts_examined).expect("candidate count fits u64"),
            facts.len(),
            terms.surface_bytes,
        ),
        // A definite Horn rule set has no `false` head: nothing in this lane can derive an
        // inconsistency, so `None` here is a statement about the fragment rather than an
        // unfilled field.
        None,
        // The evaluator mints no term — a head variable not bound by the body is refused at
        // compile time — so there is no surrogate to withhold.
        0,
    )
}

/// Insert `t` into the accumulated set and, if new, the ordered frontier seed.
fn push_fact(facts: &mut FastSet<[u32; 3]>, order: &mut Vec<[u32; 3]>, t: [u32; 3]) {
    if facts.insert(t) {
        order.push(t);
    }
}

/// Compile one rule: intern each atom's ground slots and assign each variable a
/// dense local index (assigned in first-seen order across body then head).
///
/// # Errors
///
/// [`EntailError::Parse`] if the rule is not range-restricted (datalog safety):
/// a head variable that never appears in the body has no binding source, so the
/// rule is malformed rather than silently deriving an unbound term.
fn compile_rule(
    rule: &crate::rif::model::Rule,
    terms: &mut Terms,
) -> Result<CompiledRule, EntailError> {
    // Range-restriction (safety) check up front: every head variable must be
    // bound by some body atom. Walk the model terms directly so that valid-rule
    // compilation below is byte-identical (same interned ids, same var indices).
    let body_vars: FastSet<&str> = rule.body.iter().flat_map(atom_var_names).collect();
    for name in rule.head.iter().flat_map(atom_var_names) {
        if !body_vars.contains(name) {
            return Err(EntailError::Parse(format!(
                "RIF rule head variable ?{name} is not range-restricted \
                 (not bound by the rule body)"
            )));
        }
    }

    let mut vars: Vec<String> = Vec::new();
    let body: Vec<PatternAtom> = rule
        .body
        .iter()
        .map(|a| compile_atom(a, terms, &mut vars))
        .collect();
    let head: Vec<PatternAtom> = rule
        .head
        .iter()
        .map(|a| compile_atom(a, terms, &mut vars))
        .collect();
    Ok(CompiledRule {
        body,
        head,
        num_vars: vars.len(),
    })
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

/// Compile one atom, interning constants and mapping variables to local indices.
fn compile_atom(atom: &Atom, terms: &mut Terms, vars: &mut Vec<String>) -> PatternAtom {
    PatternAtom {
        s: compile_slot(&atom.s, terms, vars),
        p: compile_slot(&atom.p, terms, vars),
        o: compile_slot(&atom.o, terms, vars),
    }
}

/// Compile one slot: a ground term interns to [`Slot::Const`]; a variable maps to
/// its local index (allocated on first sight) as [`Slot::Var`].
fn compile_slot(term: &RifTerm, terms: &mut Terms, vars: &mut Vec<String>) -> Slot {
    match term {
        RifTerm::Const(v) => Slot::Const(terms.intern(v.clone())),
        RifTerm::Var(name) => {
            let idx = vars.iter().position(|v| v == name).unwrap_or_else(|| {
                vars.push(name.clone());
                vars.len() - 1
            });
            Slot::Var(idx)
        }
    }
}

/// Semi-naive forward chase to the least fixpoint.
fn chase(facts: &mut FastSet<[u32; 3]>, seed: Vec<[u32; 3]>, rules: &[CompiledRule]) -> ChaseStats {
    let mut all = FactIndex::from_facts(seed.clone());
    let mut delta = FactIndex::from_facts(seed);
    let mut derived: Vec<[u32; 3]> = Vec::new();
    let mut stats = ChaseStats::default();
    while !delta.is_empty() {
        derived.clear();
        for rule in rules {
            fire_rule(rule, &all, &delta, &mut derived, &mut stats);
        }
        delta.clear();
        for &t in &derived {
            if facts.insert(t) {
                all.push(t);
                delta.push(t);
            }
        }
    }
    stats
}

/// Fire one rule semi-naively: for each body position `pivot`, bind that atom only
/// against the frontier `delta` and the remaining atoms against the whole `all`
/// set, then instantiate the head. Firing from every pivot position catches a new
/// fact wherever it lands in the body; the fixpoint deduplicates re-derivations.
fn fire_rule(
    rule: &CompiledRule,
    all: &FactIndex,
    delta: &FactIndex,
    derived: &mut Vec<[u32; 3]>,
    stats: &mut ChaseStats,
) {
    for pivot in 0..rule.body.len() {
        let mut binding = vec![None; rule.num_vars];
        let mut remaining: Vec<usize> = (0..rule.body.len()).filter(|&i| i != pivot).collect();
        match delta.candidate_ordinals(&rule.body[pivot], &binding) {
            Some(ordinals) => {
                for &ordinal in ordinals {
                    match_pivot(
                        rule,
                        pivot,
                        delta.facts[ordinal],
                        all,
                        &mut remaining,
                        &mut binding,
                        derived,
                        stats,
                    );
                }
            }
            None => {
                for &fact in &delta.facts {
                    match_pivot(
                        rule,
                        pivot,
                        fact,
                        all,
                        &mut remaining,
                        &mut binding,
                        derived,
                        stats,
                    );
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn match_pivot(
    rule: &CompiledRule,
    pivot: usize,
    fact: [u32; 3],
    all: &FactIndex,
    remaining: &mut Vec<usize>,
    binding: &mut Binding,
    derived: &mut Vec<[u32; 3]>,
    stats: &mut ChaseStats,
) {
    stats.candidate_facts_examined += 1;
    let mut changed = [0usize; 3];
    let Some(changed_count) = match_atom(&rule.body[pivot], fact, binding, &mut changed) else {
        return;
    };
    join_remaining(rule, all, remaining, binding, derived, stats);
    rollback(binding, &changed[..changed_count]);
}

fn join_remaining(
    rule: &CompiledRule,
    all: &FactIndex,
    remaining: &mut Vec<usize>,
    binding: &mut Binding,
    derived: &mut Vec<[u32; 3]>,
    stats: &mut ChaseStats,
) {
    if remaining.is_empty() {
        for head in &rule.head {
            derived.push(instantiate(head, binding));
        }
        return;
    }

    let choice = remaining
        .iter()
        .enumerate()
        .min_by_key(|(_, atom)| (all.estimate(&rule.body[**atom], binding), **atom))
        .map(|(position, _)| position)
        .expect("remaining atoms is non-empty");
    let atom_index = remaining.swap_remove(choice);
    let atom = &rule.body[atom_index];
    match all.candidate_ordinals(atom, binding) {
        Some(ordinals) => {
            for &ordinal in ordinals {
                match_join_candidate(
                    rule,
                    atom,
                    all.facts[ordinal],
                    all,
                    remaining,
                    binding,
                    derived,
                    stats,
                );
            }
        }
        None => {
            for &fact in &all.facts {
                match_join_candidate(rule, atom, fact, all, remaining, binding, derived, stats);
            }
        }
    }
    remaining.push(atom_index);
}

#[allow(clippy::too_many_arguments)]
fn match_join_candidate(
    rule: &CompiledRule,
    atom: &PatternAtom,
    fact: [u32; 3],
    all: &FactIndex,
    remaining: &mut Vec<usize>,
    binding: &mut Binding,
    derived: &mut Vec<[u32; 3]>,
    stats: &mut ChaseStats,
) {
    stats.candidate_facts_examined += 1;
    let mut changed = [0usize; 3];
    if let Some(changed_count) = match_atom(atom, fact, binding, &mut changed) {
        join_remaining(rule, all, remaining, binding, derived, stats);
        rollback(binding, &changed[..changed_count]);
    }
}

/// Try to bind `atom` against `fact`, recording newly-bound variable slots so the
/// caller can restore the reusable binding after recursive descent.
fn match_atom(
    atom: &PatternAtom,
    fact: [u32; 3],
    binding: &mut Binding,
    changed: &mut [usize; 3],
) -> Option<usize> {
    let mut changed_count = 0;
    for (slot, value) in [(atom.s, fact[0]), (atom.p, fact[1]), (atom.o, fact[2])] {
        if !bind_slot(slot, value, binding, changed, &mut changed_count) {
            rollback(binding, &changed[..changed_count]);
            return None;
        }
    }
    Some(changed_count)
}

/// Unify one slot with a term id: a constant must equal it; a free variable binds
/// to it; an already-bound variable must equal its binding.
fn bind_slot(
    slot: Slot,
    value: u32,
    binding: &mut Binding,
    changed: &mut [usize; 3],
    changed_count: &mut usize,
) -> bool {
    match slot {
        Slot::Const(c) => c == value,
        Slot::Var(i) => match binding[i] {
            Some(existing) => existing == value,
            None => {
                binding[i] = Some(value);
                changed[*changed_count] = i;
                *changed_count += 1;
                true
            }
        },
    }
}

fn rollback(binding: &mut Binding, changed: &[usize]) {
    for &index in changed {
        binding[index] = None;
    }
}

fn bound_value(slot: Slot, binding: &Binding) -> Option<u32> {
    match slot {
        Slot::Const(value) => Some(value),
        Slot::Var(index) => binding[index],
    }
}

/// Instantiate a head atom under a complete binding. Head variables are
/// range-restricted — `compile_rule` rejects any head variable not bound by the
/// body — so by construction every head variable is bound here.
fn instantiate(atom: &PatternAtom, b: &Binding) -> [u32; 3] {
    [resolve(atom.s, b), resolve(atom.p, b), resolve(atom.o, b)]
}

/// Resolve a head slot to a concrete term id under `b`. The `.expect(...)` is
/// unreachable: `compile_rule`'s range-restriction check guarantees every head
/// variable is body-bound, so its slot is set before the head is instantiated.
fn resolve(slot: Slot, b: &Binding) -> u32 {
    match slot {
        Slot::Const(c) => c,
        Slot::Var(i) => b[i].expect("range-restricted head variable is bound by the body"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rif::model::{Atom, RifTerm, Rule};
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
        assert!(report.boundaries().is_empty());
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
        assert!(!report.boundaries()[0].reason().is_empty());
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
                .any(|q| q.g.is_some() && out.term_value(q.p) == iri("brother")),
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
        assert!(report.boundaries().is_empty());
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
    fn indexed_backtracking_avoids_cartesian_fact_scans() {
        const COMMON: u32 = 10;
        const RARE: u32 = 11;
        const DERIVED: u32 = 12;

        let mut seed: Vec<[u32; 3]> = (0..1_000).map(|n| [n, COMMON, n + 1]).collect();
        seed.push([500, RARE, 2_000]);
        let mut facts: FastSet<[u32; 3]> = seed.iter().copied().collect();
        let rule = CompiledRule {
            body: vec![
                PatternAtom {
                    s: Slot::Var(0),
                    p: Slot::Const(COMMON),
                    o: Slot::Var(1),
                },
                PatternAtom {
                    s: Slot::Var(1),
                    p: Slot::Const(RARE),
                    o: Slot::Var(2),
                },
            ],
            head: vec![PatternAtom {
                s: Slot::Var(0),
                p: Slot::Const(DERIVED),
                o: Slot::Var(2),
            }],
            num_vars: 3,
        };

        let stats = chase(&mut facts, seed, &[rule]);
        assert!(facts.contains(&[499, DERIVED, 2_000]));
        assert!(
            stats.candidate_facts_examined < 5_000,
            "posting-list joins should inspect thousands, not the million-row Cartesian product: {}",
            stats.candidate_facts_examined
        );
    }

    #[test]
    fn failed_repeated_variable_match_rolls_back_binding() {
        let atom = PatternAtom {
            s: Slot::Var(0),
            p: Slot::Const(7),
            o: Slot::Var(0),
        };
        let mut binding = vec![None];
        let mut changed = [0usize; 3];

        assert_eq!(
            match_atom(&atom, [1, 7, 2], &mut binding, &mut changed),
            None
        );
        assert_eq!(binding, vec![None]);
        assert_eq!(
            match_atom(&atom, [3, 7, 3], &mut binding, &mut changed),
            Some(1)
        );
    }
}
