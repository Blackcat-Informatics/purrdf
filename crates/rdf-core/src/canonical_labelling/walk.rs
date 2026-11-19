// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The walk of 8.3: the one sequential computation that decides whether an
//! input is refused with `limit`, and the fields of that refusal.
//!
//! The walk charges the limited units of 8.1 to a running total `R` in a fixed
//! order: the split of the input, then each top component in ascending order
//! of its smallest source identifier (START, the REFINE of its fresh start,
//! its top call). At a branching call it visits the members of the target cell
//! in source order, skips a member related by `~` to an earlier one, stops a
//! child whose `s` or entries exceed the best's, and makes the twin tests of
//! the call after the fixing of its first child and before that child's
//! nested calls. A limit test follows every charge.
//!
//! This module holds what the walk owns: the running total and its trip, the
//! totals by kind, the twin classes `TW` of a component and its tests, the
//! relation `~` of a call, the witnesses of equal children with their cap,
//! the check of a known map, and the maps a call carries (exchanges of tied
//! nested calls). An engine supplies the calls; the order of the charges and
//! every decision that changes one is made here.

use std::collections::BTreeMap;

use super::certificate::Certificate;
use super::lines::{LineSet, Tok};
use super::reference::{Classes, Comp, Stop, Tree, TwinTest};
use super::{FailureRecord, Refusal};

/// A known map: the nodes it moves, each with its image, as local nodes of
/// the top component. It fixes every node it does not name.
pub(crate) type Map = Vec<(usize, usize)>;

/// The kinds of charge whose totals 13.2 prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// The start charge of the lines of a top component (8.1(a)).
    Start,
    /// A refinement: of a fresh start, or of a child after its
    /// individualisation (8.1(b)).
    Refine,
    /// A fixing without the split of its remainder, with its entries
    /// (8.1(c) and (d)).
    FixSum,
    /// The split of the input or of a remainder into components (8.1(c)).
    Split,
    /// The twin tests of one branching call (8.1(f)).
    Twin,
}

/// The limited units of a walk by kind, each saturating at `2^64 - 1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Totals {
    pub(crate) start: u64,
    pub(crate) refine: u64,
    pub(crate) fix_sum: u64,
    pub(crate) split: u64,
    pub(crate) twin: u64,
}

impl Totals {
    fn add(&mut self, kind: Kind, units: u64) {
        let slot = match kind {
            Kind::Start => &mut self.start,
            Kind::Refine => &mut self.refine,
            Kind::FixSum => &mut self.fix_sum,
            Kind::Split => &mut self.split,
            Kind::Twin => &mut self.twin,
        };
        *slot = slot.saturating_add(units);
    }
}

/// One charge of the walk, as the tests read it: its parts and `R` after it.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Charge {
    pub(crate) parts: Vec<(Kind, u64)>,
    pub(crate) total: u64,
}

/// The running total of a walk and its limit test (8.3, steps 1 and 6). The
/// default walk has no bound and trips on nothing.
#[derive(Default)]
pub(crate) struct Walk {
    /// `R`, saturating at `2^64 - 1` (7.11).
    total: u64,
    /// `max(L, A)`, or nothing when `A` is saturated and the walk never trips.
    bound: Option<u64>,
    totals: Totals,
    /// The smallest source identifier and the node count of the top component
    /// whose step 2 is running: the fields of a refusal.
    source: Box<[u8]>,
    nodes: u64,
    /// Every charge in order.
    #[cfg(test)]
    pub(crate) charges: Vec<Charge>,
}

impl Walk {
    /// A walk under the limit `limit` of an input of allowance `allowance`,
    /// with the charge of step 1, the split of the input, made. That charge
    /// never ends the walk (Lemma W (4)(c)): the allowance contains it, so it
    /// is at most `max(L, A)`, and once `A` is saturated nothing trips.
    pub(crate) fn new(limit: u64, allowance: u64, split: u64) -> Self {
        let mut walk = Self {
            total: 0,
            bound: (allowance != u64::MAX).then_some(limit.max(allowance)),
            totals: Totals::default(),
            source: Box::default(),
            nodes: 0,
            #[cfg(test)]
            charges: Vec::new(),
        };
        walk.add(&[(Kind::Split, split)]);
        walk
    }

    /// Step 2 begins on the top component of smallest source identifier
    /// `source` and `nodes` nodes.
    pub(crate) fn enter(&mut self, source: &[u8], nodes: u64) {
        self.source = source.into();
        self.nodes = nodes;
    }

    /// One charge of one kind, then the limit test.
    ///
    /// # Errors
    /// [`Stop::Limit`] when the charge carries `R` above `max(L, A)`.
    pub(crate) fn charge(&mut self, kind: Kind, units: u64) -> Result<(), Stop> {
        self.add(&[(kind, units)]);
        self.test()
    }

    /// The one charge of a fixing (step 3(a) and (b)(vi)): its FIX-SUM, and
    /// its SPLIT unless the child was stopped, then the limit test.
    ///
    /// # Errors
    /// As [`Self::charge`].
    pub(crate) fn charge_fixing(&mut self, fix_sum: u64, split: u64) -> Result<(), Stop> {
        self.add(&[(Kind::FixSum, fix_sum), (Kind::Split, split)]);
        self.test()
    }

    /// Add one charge; a charge of `0` is not made.
    fn add(&mut self, parts: &[(Kind, u64)]) {
        let units = parts
            .iter()
            .fold(0u64, |sum, &(_, units)| sum.saturating_add(units));
        if units == 0 {
            return;
        }
        for &(kind, units) in parts {
            self.totals.add(kind, units);
        }
        self.total = self.total.saturating_add(units);
        #[cfg(test)]
        self.charges.push(Charge {
            parts: parts.iter().copied().filter(|&(_, u)| u != 0).collect(),
            total: self.total,
        });
    }

    /// Step 6: the walk ends when `R` exceeds `max(L, A)`.
    fn test(&self) -> Result<(), Stop> {
        match self.bound {
            Some(bound) if self.total > bound => Err(Stop::Limit),
            _ => Ok(()),
        }
    }

    /// The refusal of a walk that ended at its last charge, with the first
    /// failures of the search-free run.
    pub(crate) fn refusal(&self, broken_by: Vec<FailureRecord>) -> Refusal {
        Refusal::Limit {
            broken_by,
            source: self.source.clone(),
            nodes: self.nodes,
            units: self.total,
        }
    }

    /// The totals by kind; their sum is `R`.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the totals by kind are read by the tests of the rows of 13.2"
        )
    )]
    pub(crate) const fn totals(&self) -> Totals {
        self.totals
    }

    /// `R`.
    #[cfg(test)]
    pub(crate) const fn total(&self) -> u64 {
        self.total
    }
}

/// Step 4: the twin tests of a branching call whose target cell, in source
/// order, is `target` (local nodes of the top component). `u` is the first
/// member; each later member not in the class of `u` in `TW` is tested, a
/// passing test joins the two classes, and a failing one does not end the
/// tests. Returns the twin units of the tests (8.1(f)).
pub(crate) fn twin_tests(
    twin: &TwinTest,
    set: &LineSet,
    comp: &Comp,
    tw: &mut Classes,
    target: &[usize],
) -> u64 {
    let Some((&u, rest)) = target.split_first() else {
        return 0;
    };
    let mut units = 0u64;
    for &w in rest {
        let (ru, rw) = (tw.find(u), tw.find(w));
        if ru == rw {
            continue;
        }
        let (passed, charge) = twin.test(set, comp, u, w);
        units = units.saturating_add(charge);
        if passed {
            tw.join(ru, rw);
        }
    }
    units
}

/// The relation `~` of one branching call (step 5), over the indices of its
/// target cell in source order.
pub(crate) struct Relation {
    classes: Classes,
    len: usize,
}

impl Relation {
    pub(crate) fn new(len: usize) -> Self {
        Self {
            classes: Classes::new(len),
            len,
        }
    }

    /// Relate the members of indices `i` and `j`.
    pub(crate) fn relate(&mut self, i: usize, j: usize) {
        let (ri, rj) = (self.classes.find(i), self.classes.find(j));
        if ri != rj {
            self.classes.join(ri, rj);
        }
    }

    /// (K1): every two members of `target` in one class of `TW` are related.
    pub(crate) fn twins(&mut self, target: &[usize], tw: &mut Classes) {
        let roots: Vec<usize> = target.iter().map(|&v| tw.find(v)).collect();
        for i in 1..self.len {
            if let Some(j) = (0..i).find(|&j| roots[j] == roots[i]) {
                self.relate(i, j);
            }
        }
    }

    /// Step 3(b)(i): whether the member of index `i` is related to an earlier
    /// one, so that its child is skipped.
    pub(crate) fn skipped(&mut self, i: usize) -> bool {
        let root = self.classes.find(i);
        (0..i).any(|j| self.classes.find(j) == root)
    }
}

/// The most witnesses a branching call of `n` free nodes keeps (step 5,
/// (K2)): `max(floor(2^22 / n), 8)`. A bound on memory; it withholds maps
/// only above `2,048` free nodes, and never changes a result (Lemma W (2)).
pub(crate) const fn witness_cap(n: usize) -> usize {
    let cap = (1usize << 22) / if n == 0 { 1 } else { n };
    if cap > 8 { cap } else { 8 }
}

/// The witnesses of one branching call (step 5, (K2)): completed children
/// keyed by their certificates, the first of each key, at most the cap.
pub(crate) struct Witnesses {
    kept: BTreeMap<Certificate, Vec<usize>>,
    cap: usize,
}

impl Witnesses {
    /// The witnesses of a call of `n` free nodes.
    pub(crate) fn new(n: usize) -> Self {
        Self {
            kept: BTreeMap::new(),
            cap: witness_cap(n),
        }
    }

    /// A child completed with the certificate `key` and the free nodes of the
    /// call `by_position` in the order of their final positions below it. The
    /// known map from the kept witness of an equal key, which sends each node
    /// to the node of the same position below this child; otherwise the child
    /// is kept when fewer than the cap are.
    pub(crate) fn complete(&mut self, key: &Certificate, by_position: Vec<usize>) -> Option<Map> {
        if let Some(first) = self.kept.get(key) {
            return Some(
                first
                    .iter()
                    .zip(&by_position)
                    .filter(|(x, y)| x != y)
                    .map(|(&x, &y)| (x, y))
                    .collect(),
            );
        }
        if self.kept.len() < self.cap {
            self.kept.insert(key.clone(), by_position);
        }
        None
    }
}

/// Whether `map` is a map of the call whose free nodes satisfy `free`, whose
/// lines are `lines` (local lines of the top component) and whose target cell
/// satisfies `in_target`: it names only free nodes, keeps the target cell,
/// and carries every line of the call onto a line of the component, every
/// node it does not name held fixed (step 5, the check). `image` is scratch
/// of one entry per node of the component, the identity between calls.
#[expect(
    clippy::too_many_arguments,
    reason = "the check reads the call's nodes, lines and cell and the component's lines, all borrowed"
)]
pub(crate) fn carries(
    set: &LineSet,
    comp: &Comp,
    twin: &TwinTest,
    lines: &[usize],
    free: impl Fn(usize) -> bool,
    in_target: impl Fn(usize) -> bool,
    map: &[(usize, usize)],
    image: &mut [usize],
) -> bool {
    if map
        .iter()
        .any(|&(x, y)| !free(x) || !free(y) || in_target(x) != in_target(y))
    {
        return false;
    }
    for &(x, y) in map {
        image[x] = y;
    }
    let ok = lines.iter().all(|&l| {
        let mut occurrences = comp.occ[l].iter();
        let mut tokens = Vec::with_capacity(set.lines[comp.lines[l]].len());
        for token in &*set.lines[comp.lines[l]] {
            tokens.push(match *token {
                Tok::Blank(_) => {
                    let Some(&node) = occurrences.next() else {
                        return false;
                    };
                    Tok::Blank(comp.nodes[image[node]])
                }
                other => other,
            });
        }
        twin.holds(&tokens)
    });
    for &(x, _) in map {
        image[x] = x;
    }
    ok
}

/// The nodes below a call or a child that fixed `fixed` at the argument `p`
/// and nested `nested`, with their final positions, the offset taken as `0`
/// (6.5), ascending by position.
pub(crate) fn nodes_below(fixed: &[(usize, u64)], p: u64, nested: &[Tree]) -> Vec<(u64, usize)> {
    let mut out: Vec<(u64, usize)> = fixed.iter().map(|&(v, q)| (q - p, v)).collect();
    let s = fixed.len() as u64;
    let mut work: Vec<(&Tree, u64)> = Vec::with_capacity(nested.len());
    let mut k = 0;
    for tree in nested {
        work.push((tree, s + k));
        k += tree.n_k;
    }
    while let Some((t, o)) = work.pop() {
        for &(v, q) in &t.fixed {
            out.push((o + (q - t.p), v));
        }
        let s = t.fixed.len() as u64;
        let mut k = 0;
        for nested in &t.nested {
            work.push((nested, o + s + k));
            k += nested.n_k;
        }
    }
    out.sort_unstable();
    out
}

/// The nodes of a result path below `tree` with their final positions, the
/// offset of `tree` taken as `0`, ascending.
pub(crate) fn tree_nodes(tree: &Tree) -> Vec<(u64, usize)> {
    nodes_below(&tree.fixed, tree.p, &tree.nested)
}

/// The exchanges of consecutive tied calls among `nested` (equal certificates,
/// in the order of 6.4), each as moved `(node, image)` pairs: each node of the
/// first goes to the node of the second with the same final position, and
/// back (10.1 (2)).
pub(crate) fn tied_exchanges(nested: &[Tree]) -> Vec<Map> {
    let mut out = Vec::new();
    for pair in nested.windows(2) {
        if pair[0].cert != pair[1].cert {
            continue;
        }
        let (a, b) = (tree_nodes(&pair[0]), tree_nodes(&pair[1]));
        if a.len() != b.len() || a.iter().zip(&b).any(|(x, y)| x.0 != y.0) {
            continue;
        }
        let mut moved = Vec::with_capacity(2 * a.len());
        for (x, y) in a.iter().zip(&b) {
            moved.push((x.1, y.1));
            moved.push((y.1, x.1));
        }
        out.push(moved);
    }
    out
}
