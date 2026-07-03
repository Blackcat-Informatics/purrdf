// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Triple-expression matching (ShEx 2.1 spec §5.5): cardinalities on triple
//! constraints AND groups, `EachOf` partitioning, `OneOf` choice, and
//! `TripleExprRef` inclusion.
//!
//! # Design
//!
//! A [`TripleExpr`] is compiled (with inclusions inlined) into a tree whose
//! leaves are **slots** — one per `TripleConstraint` *occurrence*. Matching
//! then happens in two layers:
//!
//! 1. **Arc→slot assignment.** Every triple in the focus node's
//!    neighbourhood whose `(predicate, direction)` is mentioned by the
//!    expression must be assigned to a slot whose value expression it
//!    satisfies, or diverted to `EXTRA`. When each `(predicate, direction)`
//!    appears in exactly ONE slot (the overwhelmingly common case) the
//!    assignment is forced up to EXTRA-diversion and collapses to a per-slot
//!    count *interval*; otherwise a bounded backtracking search enumerates
//!    assignments (deterministically, in sorted-arc / slot-index order).
//! 2. **Count checking.** For fixed (or interval) per-slot counts, the
//!    expression matches iff the counts are derivable by the grammar. Since
//!    slots are tree positions, the compiled expression is single-occurrence
//!    and the classic interval calculus is exact: bottom-up, compute for
//!    every subexpression the interval of possible repetition counts —
//!    `TripleConstraint` counts divide by the leaf cardinality, `EachOf`
//!    intersects its children, `OneOf` Minkowski-sums them, and group
//!    cardinalities divide the result. The expression matches iff the root
//!    interval contains 1.

use std::collections::HashMap;

use crate::ast::{SemAct, ShapeExpr, TripleConstraint, TripleExpr};

/// A `{min, max}` cardinality; `max == None` is unbounded.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Card {
    pub min: u64,
    pub max: Option<u64>,
}

impl Card {
    /// From the AST's optional `min`/`max` (absent means `{1,1}`, `-1` means
    /// unbounded).
    fn from_ast(min: Option<i64>, max: Option<i64>) -> Self {
        let min = min.map_or(1, |m| u64::try_from(m).unwrap_or(0));
        let max = match max {
            None => Some(1),
            Some(m) if m < 0 => None,
            Some(m) => Some(m.unsigned_abs()),
        };
        Self { min, max }
    }
}

/// One `TripleConstraint` occurrence in the compiled tree.
#[derive(Debug)]
pub(crate) struct Slot<'a> {
    /// The predicate IRI.
    pub predicate: &'a str,
    /// `true` for `^` inverse constraints.
    pub inverse: bool,
    /// The value expression (`None` is the `.` wildcard).
    pub value_expr: Option<&'a ShapeExpr>,
    /// The constraint's own cardinality.
    pub card: Card,
    /// The constraint's semantic actions, fired when it matches ≥1 triple.
    pub sem_acts: &'a [SemAct],
}

/// A compiled triple-expression node.
#[derive(Debug)]
pub(crate) enum CNode<'a> {
    /// A leaf [`Slot`], by index.
    Slot(usize),
    /// `EachOf` with a group cardinality and the group's own semantic
    /// actions (fired only when this node actually participated in the
    /// winning match — see [`participating_group_acts`]).
    Each(Vec<Self>, Card, Vec<&'a SemAct>),
    /// `OneOf` with a group cardinality and the group's own semantic
    /// actions (fired only for the selected branch(es) — see
    /// [`participating_group_acts`]).
    One(Vec<Self>, Card, Vec<&'a SemAct>),
}

/// A compiled triple expression: the tree and its slot table.
#[derive(Debug)]
pub(crate) struct Compiled<'a> {
    pub root: CNode<'a>,
    pub slots: Vec<Slot<'a>>,
}

/// Compile a triple expression, inlining `TripleExprRef`s via `te_map`.
pub(crate) fn compile<'a>(
    expr: &'a TripleExpr,
    te_map: &HashMap<&'a str, &'a TripleExpr>,
) -> Result<Compiled<'a>, String> {
    let mut slots = Vec::new();
    let mut stack: Vec<&'a str> = Vec::new();
    let root = compile_node(expr, te_map, &mut slots, &mut stack)?;
    Ok(Compiled { root, slots })
}

fn compile_node<'a>(
    expr: &'a TripleExpr,
    te_map: &HashMap<&'a str, &'a TripleExpr>,
    slots: &mut Vec<Slot<'a>>,
    stack: &mut Vec<&'a str>,
) -> Result<CNode<'a>, String> {
    match expr {
        TripleExpr::TripleConstraint(tc) => Ok(CNode::Slot(push_slot(tc, slots))),
        TripleExpr::EachOf(group) | TripleExpr::OneOf(group) => {
            let card = Card::from_ast(group.min, group.max);
            let acts: Vec<&'a SemAct> = group.sem_acts.iter().collect();
            let children = group
                .expressions
                .iter()
                .map(|child| compile_node(child, te_map, slots, stack))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(match expr {
                TripleExpr::EachOf(_) => CNode::Each(children, card, acts),
                _ => CNode::One(children, card, acts),
            })
        }
        TripleExpr::Ref(label) => {
            let Some(target) = te_map.get(label.as_str()) else {
                return Err(format!("inclusion of undeclared triple expression {label}"));
            };
            if stack.contains(&label.as_str()) {
                return Err(format!("cyclic triple-expression inclusion of {label}"));
            }
            stack.push(label.as_str());
            let node = compile_node(target, te_map, slots, stack)?;
            stack.pop();
            Ok(node)
        }
    }
}

fn push_slot<'a>(tc: &'a TripleConstraint, slots: &mut Vec<Slot<'a>>) -> usize {
    slots.push(Slot {
        predicate: tc.predicate.as_str(),
        inverse: tc.inverse == Some(true),
        value_expr: tc.value_expr.as_deref(),
        card: Card::from_ast(tc.min, tc.max),
        sem_acts: &tc.sem_acts,
    });
    slots.len() - 1
}

// ── interval calculus ───────────────────────────────────────────────────────

/// A non-empty interval of repetition counts; `hi == None` is unbounded.
/// The *empty* interval is represented as `Option<Iv>::None` by the calculus.
#[derive(Clone, Copy, Debug)]
struct Iv {
    lo: u64,
    hi: Option<u64>,
}

fn intersect(a: Iv, b: Iv) -> Option<Iv> {
    let lo = a.lo.max(b.lo);
    let hi = match (a.hi, b.hi) {
        (None, h) | (h, None) => h,
        (Some(x), Some(y)) => Some(x.min(y)),
    };
    match hi {
        Some(h) if lo > h => None,
        _ => Some(Iv { lo, hi }),
    }
}

/// Minkowski sum (`OneOf` distributes its repetitions across branches).
fn sum(a: Iv, b: Iv) -> Iv {
    Iv {
        lo: a.lo.saturating_add(b.lo),
        hi: match (a.hi, b.hi) {
            (Some(x), Some(y)) => Some(x.saturating_add(y)),
            _ => None,
        },
    }
}

/// The repetition counts `r` under which `r` uses of a subexpression with
/// cardinality `card` can consume a total in `inner`: `r` is valid iff
/// `r*min <= inner.hi` and `r*max >= inner.lo`.
fn wrap(inner: Iv, card: Card) -> Option<Iv> {
    let hi = if card.min == 0 {
        None
    } else {
        inner.hi.map(|h| h / card.min)
    };
    let lo = if inner.lo == 0 {
        0
    } else {
        match card.max {
            None => 1,
            Some(0) => return None,
            Some(m) => inner.lo.div_ceil(m),
        }
    };
    match hi {
        Some(h) if lo > h => None,
        _ => Some(Iv { lo, hi }),
    }
}

/// Bottom-up repetition interval of `node` for per-slot count ranges.
fn reps(node: &CNode<'_>, slots: &[Slot<'_>], counts: &[(u64, u64)]) -> Option<Iv> {
    match node {
        CNode::Slot(i) => wrap(
            Iv {
                lo: counts[*i].0,
                hi: Some(counts[*i].1),
            },
            slots[*i].card,
        ),
        CNode::Each(children, card, _) => {
            let mut inner = Iv { lo: 0, hi: None };
            for child in children {
                inner = intersect(inner, reps(child, slots, counts)?)?;
            }
            wrap(inner, *card)
        }
        CNode::One(children, card, _) => {
            let mut inner = Iv { lo: 0, hi: Some(0) };
            for child in children {
                inner = sum(inner, reps(child, slots, counts)?);
            }
            wrap(inner, *card)
        }
    }
}

// ── group semantic-action participation (ShEx 2.1 §5.5.2) ──────────────────

/// The total number of triples the winning assignment routed to the slots
/// reachable under `node`.
fn leaf_triple_count(node: &CNode<'_>, counts: &[(u64, u64)]) -> u64 {
    match node {
        CNode::Slot(i) => counts[*i].0,
        CNode::Each(children, _, _) | CNode::One(children, _, _) => children
            .iter()
            .map(|child| leaf_triple_count(child, counts))
            .sum(),
    }
}

/// Collect the `EachOf`/`OneOf` group semantic actions that actually
/// participated in the winning match, in deterministic (pre-order,
/// parent-before-children) traversal order.
///
/// A group's own actions fire only when the group's triple expression
/// actually took part in the match: at least one triple was routed, by the
/// winning assignment, to a slot reachable underneath it. For `OneOf` this
/// means only the selected branch(es) — an alternative that consumed no
/// triples did not fire, even though the overall choice matched (ShEx 2.1
/// §5.5.2; contrast the flat, ungated firing this replaces).
pub(crate) fn participating_group_acts<'a>(
    compiled: &'a Compiled<'a>,
    counts: &[(u64, u64)],
) -> Vec<&'a SemAct> {
    let mut acts = Vec::new();
    collect_participating(&compiled.root, counts, &mut acts);
    acts
}

fn collect_participating<'a>(
    node: &'a CNode<'a>,
    counts: &[(u64, u64)],
    out: &mut Vec<&'a SemAct>,
) {
    let (children, group_acts) = match node {
        CNode::Slot(_) => return,
        CNode::Each(children, _, acts) | CNode::One(children, _, acts) => (children, acts),
    };
    if leaf_triple_count(node, counts) == 0 {
        return;
    }
    out.extend(group_acts.iter().copied());
    for child in children {
        collect_participating(child, counts, out);
    }
}

/// Whether the expression (used exactly once) can consume per-slot counts in
/// the given `[lo, hi]` ranges.
pub(crate) fn counts_match(compiled: &Compiled<'_>, counts: &[(u64, u64)]) -> bool {
    reps(&compiled.root, &compiled.slots, counts)
        .is_some_and(|iv| iv.lo <= 1 && iv.hi.is_none_or(|h| h >= 1))
}

// ── general assignment search ───────────────────────────────────────────────

/// Per-arc assignment options: candidate slot indices (in deterministic
/// order) plus whether the arc may be diverted to `EXTRA`.
#[derive(Debug)]
pub(crate) struct ArcOptions {
    pub candidates: Vec<usize>,
    pub extra_allowed: bool,
}

/// Search state budget: candidate-assignment steps before giving up.
const SEARCH_BUDGET: u64 = 200_000;

/// A winning assignment search result: per-slot `(lo, hi)` counts, plus the
/// slot each arc was routed to (`None` when diverted to `EXTRA`).
pub(crate) type Assignment = (Vec<(u64, u64)>, Vec<Option<usize>>);

/// Backtracking search over arc→slot assignments for expressions where a
/// `(predicate, direction)` occurs in more than one slot. Deterministic:
/// arcs in the caller's (sorted) order, candidates in slot order, `EXTRA`
/// diversion tried last. Returns `Ok(Some((counts, assignment)))` on a match,
/// where `assignment[i]` is the slot arc `i` was routed to (`None` when
/// diverted to `EXTRA`); `Ok(None)` when no assignment matches; `Err` when
/// the budget is exhausted.
pub(crate) fn assignment_search(
    compiled: &Compiled<'_>,
    arcs: &[ArcOptions],
) -> Result<Option<Assignment>, String> {
    let mut counts = vec![(0u64, 0u64); compiled.slots.len()];
    let mut assignment = vec![None; arcs.len()];
    let mut budget = SEARCH_BUDGET;
    let found = search(compiled, arcs, 0, &mut counts, &mut assignment, &mut budget)?;
    Ok(found.then_some((counts, assignment)))
}

fn search(
    compiled: &Compiled<'_>,
    arcs: &[ArcOptions],
    index: usize,
    counts: &mut Vec<(u64, u64)>,
    assignment: &mut Vec<Option<usize>>,
    budget: &mut u64,
) -> Result<bool, String> {
    if *budget == 0 {
        return Err("triple-expression matcher budget exhausted".to_owned());
    }
    *budget -= 1;
    let Some(arc) = arcs.get(index) else {
        return Ok(counts_match(compiled, counts));
    };
    for &slot in &arc.candidates {
        counts[slot].0 += 1;
        counts[slot].1 += 1;
        assignment[index] = Some(slot);
        if search(compiled, arcs, index + 1, counts, assignment, budget)? {
            return Ok(true);
        }
        counts[slot].0 -= 1;
        counts[slot].1 -= 1;
        assignment[index] = None;
    }
    if arc.extra_allowed {
        assignment[index] = None;
        return search(compiled, arcs, index + 1, counts, assignment, budget);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tc(predicate: &str, min: Option<i64>, max: Option<i64>) -> TripleExpr {
        TripleExpr::TripleConstraint(TripleConstraint {
            predicate: predicate.to_owned(),
            min,
            max,
            ..TripleConstraint::default()
        })
    }

    fn compile_simple(expr: &TripleExpr) -> Compiled<'_> {
        compile(expr, &HashMap::new()).expect("compile")
    }

    #[test]
    fn plain_cardinality_intervals() {
        // <p> . {1,2}
        let expr = tc("p", Some(1), Some(2));
        let compiled = compile_simple(&expr);
        assert!(!counts_match(&compiled, &[(0, 0)]));
        assert!(counts_match(&compiled, &[(1, 1)]));
        assert!(counts_match(&compiled, &[(2, 2)]));
        assert!(!counts_match(&compiled, &[(3, 3)]));
        // A flexible (EXTRA-divertible) range only needs one workable count.
        assert!(counts_match(&compiled, &[(0, 3)]));
    }

    #[test]
    fn eachof_intersects_and_oneof_sums() {
        use crate::ast::TripleExprGroup;
        // (<a> . ; <b> .) — both required.
        let each = TripleExpr::EachOf(TripleExprGroup {
            expressions: vec![tc("a", None, None), tc("b", None, None)],
            ..TripleExprGroup::default()
        });
        let compiled = compile_simple(&each);
        assert!(counts_match(&compiled, &[(1, 1), (1, 1)]));
        assert!(!counts_match(&compiled, &[(1, 1), (0, 0)]));

        // (<a> . | <b> .) — exactly one.
        let one = TripleExpr::OneOf(TripleExprGroup {
            expressions: vec![tc("a", None, None), tc("b", None, None)],
            ..TripleExprGroup::default()
        });
        let compiled = compile_simple(&one);
        assert!(counts_match(&compiled, &[(1, 1), (0, 0)]));
        assert!(counts_match(&compiled, &[(0, 0), (1, 1)]));
        assert!(!counts_match(&compiled, &[(1, 1), (1, 1)]));
        assert!(!counts_match(&compiled, &[(0, 0), (0, 0)]));
    }

    #[test]
    fn group_repetition_distributes() {
        use crate::ast::TripleExprGroup;
        // (<a> . | <b> .){2} — two picks across the branches.
        let one = TripleExpr::OneOf(TripleExprGroup {
            expressions: vec![tc("a", None, None), tc("b", None, None)],
            min: Some(2),
            max: Some(2),
            ..TripleExprGroup::default()
        });
        let compiled = compile_simple(&one);
        assert!(counts_match(&compiled, &[(1, 1), (1, 1)]));
        assert!(counts_match(&compiled, &[(2, 2), (0, 0)]));
        assert!(!counts_match(&compiled, &[(1, 1), (0, 0)]));
        assert!(!counts_match(&compiled, &[(2, 2), (1, 1)]));
    }

    #[test]
    fn assignment_search_resolves_repeated_predicates() {
        use crate::ast::TripleExprGroup;
        // <p> [v1]; <p> [v2] — two slots, same predicate.
        let each = TripleExpr::EachOf(TripleExprGroup {
            expressions: vec![tc("p", None, None), tc("p", None, None)],
            ..TripleExprGroup::default()
        });
        let compiled = compile_simple(&each);
        // Arc 1 can only fill slot 0; arc 2 fits both: search must route
        // arc 2 to slot 1.
        let arcs = vec![
            ArcOptions {
                candidates: vec![0],
                extra_allowed: false,
            },
            ArcOptions {
                candidates: vec![0, 1],
                extra_allowed: false,
            },
        ];
        assert_eq!(
            assignment_search(&compiled, &arcs),
            Ok(Some((vec![(1, 1), (1, 1)], vec![Some(0), Some(1)])))
        );
        // Both arcs only fit slot 0 → slot 1 starves.
        let arcs = vec![
            ArcOptions {
                candidates: vec![0],
                extra_allowed: false,
            },
            ArcOptions {
                candidates: vec![0],
                extra_allowed: false,
            },
        ];
        assert_eq!(assignment_search(&compiled, &arcs), Ok(None));
    }
}
