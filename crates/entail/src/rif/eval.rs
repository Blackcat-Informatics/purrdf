// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RIF-Core forward-chaining ("bottom-up") rule evaluator.
//!
//! A definite Horn rule set is materialized to its least fixpoint by a
//! deterministic semi-naive chase over interned `u32` triple ids, mirroring the
//! frontier/delta discipline of `crate::rdfs`. The seed fact set is the source
//! dataset's default-graph triples plus the rule set's ground facts; each round
//! fires every rule where at least one body atom can bind a *frontier* (newly
//! derived) fact, joining the remaining atoms against the whole accumulated set.
//! The next frontier is the round's genuinely-new triples; the chase halts when
//! the frontier empties. Blank nodes are preserved by identity (interned by their
//! `(label, scope)` value), never skolemized. Output is `original + derived`,
//! frozen into a fresh dataset — fully deterministic.

use std::sync::Arc;

use purrdf_core::{FastMap, FastSet, RdfDataset, RdfDatasetBuilder};

use crate::EntailError;
use crate::interner::{Interner, intern_into};
use crate::rif::model::{Atom, RifTerm, RuleSet};

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

/// Materialize the RIF rule set over `ds`, returning `original quads + derived
/// triples`.
///
/// The seed facts are `ds`'s default-graph triples plus `rules.facts`; the Horn
/// rules are forward-chained to a fixpoint. The result holds every original quad
/// (all graphs) plus every seeded or derived fact not already an original
/// default-graph triple, frozen into a new dataset.
///
/// # Errors
///
/// [`EntailError::Build`] if the derived dataset cannot be frozen.
pub fn materialize_rif(ds: &RdfDataset, rules: &RuleSet) -> Result<Arc<RdfDataset>, EntailError> {
    let mut interner = Interner::default();

    // Seed: the source dataset's default-graph triples, in dataset order.
    let mut facts: FastSet<[u32; 3]> = FastSet::default();
    let mut seed: Vec<[u32; 3]> = Vec::new();
    for q in ds.quads() {
        if q.g.is_some() {
            continue; // entailment operates over the default graph
        }
        let s = interner.intern(ds.term_value(q.s));
        let p = interner.intern(ds.term_value(q.p));
        let o = interner.intern(ds.term_value(q.o));
        push_fact(&mut facts, &mut seed, [s, p, o]);
    }
    let original: FastSet<[u32; 3]> = facts.clone();

    // Seed: the rule set's ground facts (imported RDF + ground frames).
    for (s, p, o) in &rules.facts {
        let s = interner.intern(s.clone());
        let p = interner.intern(p.clone());
        let o = interner.intern(o.clone());
        push_fact(&mut facts, &mut seed, [s, p, o]);
    }

    // Compile every rule against interned terms and a dense variable table.
    let compiled: Vec<CompiledRule> = rules
        .rules
        .iter()
        .map(|r| compile_rule(r, &mut interner))
        .collect::<Result<Vec<_>, _>>()?;

    let _stats = chase(&mut facts, seed, &compiled);

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
        let s = intern_into(&mut b, interner.value(t[0]));
        let p = intern_into(&mut b, interner.value(t[1]));
        let o = intern_into(&mut b, interner.value(t[2]));
        b.push_quad(s, p, o, None);
    }
    b.freeze().map_err(|e| EntailError::Build(e.to_string()))
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
    interner: &mut Interner,
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
        .map(|a| compile_atom(a, interner, &mut vars))
        .collect();
    let head: Vec<PatternAtom> = rule
        .head
        .iter()
        .map(|a| compile_atom(a, interner, &mut vars))
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
fn compile_atom(atom: &Atom, interner: &mut Interner, vars: &mut Vec<String>) -> PatternAtom {
    PatternAtom {
        s: compile_slot(&atom.s, interner, vars),
        p: compile_slot(&atom.p, interner, vars),
        o: compile_slot(&atom.o, interner, vars),
    }
}

/// Compile one slot: a ground term interns to [`Slot::Const`]; a variable maps to
/// its local index (allocated on first sight) as [`Slot::Var`].
fn compile_slot(term: &RifTerm, interner: &mut Interner, vars: &mut Vec<String>) -> Slot {
    match term {
        RifTerm::Const(v) => Slot::Const(interner.intern(v.clone())),
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
        let out = materialize_rif(&empty_ds(), &rules).expect("materialize");
        assert!(
            has(&out, &iri("Emeka"), &iri("uncle"), &iri("Chijoke")),
            "derived Emeka uncle Chijoke"
        );
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
        let out = materialize_rif(&empty_ds(), &rules).expect("materialize");
        assert!(
            has(&out, &iri("customer017"), &iri("discount"), &ten),
            "gold ⇒ discount 10"
        );
        assert!(
            !has(&out, &iri("customer017"), &iri("discount"), &five),
            "silver rule must not fire"
        );
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
