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

use std::collections::HashSet;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder};

use crate::interner::{intern_into, Interner};
use crate::rif::model::{Atom, RifTerm, RuleSet};
use crate::EntailError;

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
    let mut facts: HashSet<[u32; 3]> = HashSet::new();
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
    let original: HashSet<[u32; 3]> = facts.clone();

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

    chase(&mut facts, seed, &compiled);

    // Emit: original quads (all graphs) + every seeded/derived fact that is not an
    // original default-graph triple, in a deterministic order.
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    // `HashSet` iteration order is not stable across runs, so sort the accumulated
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
fn push_fact(facts: &mut HashSet<[u32; 3]>, order: &mut Vec<[u32; 3]>, t: [u32; 3]) {
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
    let body_vars: HashSet<&str> = rule.body.iter().flat_map(atom_var_names).collect();
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
fn chase(facts: &mut HashSet<[u32; 3]>, seed: Vec<[u32; 3]>, rules: &[CompiledRule]) {
    let mut all: Vec<[u32; 3]> = seed.clone();
    let mut delta: Vec<[u32; 3]> = seed;
    let mut derived: Vec<[u32; 3]> = Vec::new();
    let mut next: Vec<[u32; 3]> = Vec::new();
    while !delta.is_empty() {
        derived.clear();
        next.clear();
        for rule in rules {
            fire_rule(rule, &all, &delta, &mut derived);
        }
        for &t in &derived {
            if facts.insert(t) {
                all.push(t);
                next.push(t);
            }
        }
        std::mem::swap(&mut delta, &mut next);
    }
}

/// Fire one rule semi-naively: for each body position `pivot`, bind that atom only
/// against the frontier `delta` and the remaining atoms against the whole `all`
/// set, then instantiate the head. Firing from every pivot position catches a new
/// fact wherever it lands in the body; the fixpoint deduplicates re-derivations.
fn fire_rule(
    rule: &CompiledRule,
    all: &[[u32; 3]],
    delta: &[[u32; 3]],
    derived: &mut Vec<[u32; 3]>,
) {
    for pivot in 0..rule.body.len() {
        let mut bindings: Vec<Binding> = Vec::new();
        // The pivot atom binds against the frontier only.
        for &fact in delta {
            let mut b = vec![None; rule.num_vars];
            if match_atom(&rule.body[pivot], fact, &mut b) {
                bindings.push(b);
            }
        }
        // The remaining atoms (in body order, skipping the pivot) join against all.
        for (i, atom) in rule.body.iter().enumerate() {
            if i == pivot {
                continue;
            }
            if bindings.is_empty() {
                break;
            }
            let mut next: Vec<Binding> = Vec::new();
            for b in &bindings {
                for &fact in all {
                    let mut nb = b.clone();
                    if match_atom(atom, fact, &mut nb) {
                        next.push(nb);
                    }
                }
            }
            bindings = next;
        }
        for b in &bindings {
            for h in &rule.head {
                derived.push(instantiate(h, b));
            }
        }
    }
}

/// Try to bind `atom`'s slots against `fact`, extending `b`. Returns `false` (and
/// leaves `b` in a possibly-partially-extended state, which the caller discards)
/// on any conflict.
fn match_atom(atom: &PatternAtom, fact: [u32; 3], b: &mut Binding) -> bool {
    bind_slot(atom.s, fact[0], b) && bind_slot(atom.p, fact[1], b) && bind_slot(atom.o, fact[2], b)
}

/// Unify one slot with a term id: a constant must equal it; a free variable binds
/// to it; an already-bound variable must equal its binding.
fn bind_slot(slot: Slot, value: u32, b: &mut Binding) -> bool {
    match slot {
        Slot::Const(c) => c == value,
        Slot::Var(i) => match b[i] {
            Some(existing) => existing == value,
            None => {
                b[i] = Some(value);
                true
            }
        },
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
}
