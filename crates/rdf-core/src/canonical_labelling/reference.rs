// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The reference engine: the definition written for clarity.
//!
//! Partitions are ordered maps from colour to cell; every split, label and key
//! is computed exactly as the definition states it, with no shortcut beyond
//! the twin test of the search-free run. A component the search-free run
//! cannot label is searched by exploring every child of every branching call.
//! Calls recurse on the machine stack; the depth is at most the number of
//! nodes of the component.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use super::certificate::{Certificate, entry_of};
use super::lines::{Leaf, LineSet, Tok};
use super::{
    COMPONENT_CONTEXT, Component, ComponentRecord, Factor, FactorKind, FailureDiagnostic,
    FailureRecord, Halt, Identity, LINES_CONTEXT, Labelled, Placement, Quantity, Refusal,
    StatisticsRecord, Units, bits, digest_of,
};
use crate::FastMap;
use crate::{StopSignal, governor::is_stopped};

/// The reference engine's own limit on the limited units of a search.
pub const ENGINE_LIMIT: u64 = 1 << 34;

/// A top component in local indices.
pub(crate) struct Comp {
    /// Global node index of each local node.
    pub(crate) nodes: Vec<usize>,
    /// Global line index of each local line.
    pub(crate) lines: Vec<usize>,
    /// Per local line, the local node of each blank occurrence, walk order.
    pub(crate) occ: Vec<Vec<usize>>,
    /// Per local node, the distinct local lines holding it, ascending.
    pub(crate) node_lines: Vec<Vec<usize>>,
    /// Per local line, the distinct local nodes it holds, ascending.
    pub(crate) line_nodes: Vec<Vec<usize>>,
}

/// One side of a pair of ordered partitions: colour to cell.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Side {
    cells: BTreeMap<u64, Vec<usize>>,
    colour: FastMap<usize, u64>,
}

impl Side {
    fn new(cells: impl IntoIterator<Item = Vec<usize>>) -> Self {
        let mut side = Self {
            cells: BTreeMap::new(),
            colour: FastMap::default(),
        };
        let mut c = 0;
        for cell in cells {
            for &v in &cell {
                side.colour.insert(v, c);
            }
            let size = cell.len() as u64;
            side.cells.insert(c, cell);
            c += size;
        }
        side
    }

    /// The cells in colour order, restricted to `keep`, re-offset.
    fn restrict(&self, keep: &BTreeSet<usize>) -> Self {
        Self::new(self.cells.values().filter_map(|cell| {
            let kept: Vec<usize> = cell.iter().copied().filter(|v| keep.contains(v)).collect();
            (!kept.is_empty()).then_some(kept)
        }))
    }
}

/// The node side (0) and the line side (1).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Pair {
    sides: [Side; 2],
}

type Queue = BTreeSet<(usize, u64)>;

/// The result path of one call, with what the record and positions need.
#[derive(Clone, Debug)]
pub(crate) struct Tree {
    pub(crate) rule: u8,
    pub(crate) p: u64,
    pub(crate) n_k: u64,
    /// The nodes fixed (by the call, or by its chosen child) and their path
    /// positions.
    pub(crate) fixed: Vec<(usize, u64)>,
    pub(crate) nested: Vec<Self>,
    pub(crate) cert: Certificate,
    /// Branching calls: the target cell, the attaining count and the rule
    /// of the chosen child.
    pub(crate) target: Vec<usize>,
    pub(crate) attaining: u64,
    pub(crate) child_rule: u8,
    /// Units of the fixing (and of the chosen child's refinement).
    pub(crate) units: Units,
    /// Traced searches only: every child of a branching call, in ascending
    /// source order, and whether the target cell is one twin class.
    pub(crate) children: Vec<ChildTrace>,
    pub(crate) one_twin_class: bool,
    /// Children of a branching call stopped by the strict stop on `s` and on
    /// `E` (10.4). Always zero from the reference engine, which explores every
    /// child.
    pub(crate) stopped_s: u64,
    pub(crate) stopped_e: u64,
}

impl Tree {
    /// A call that took rule 1 or 2 (or a child, which takes one of them).
    pub(crate) fn of_child(child: Child, p: u64, n_k: u64) -> Self {
        Self {
            rule: child.rule,
            p,
            n_k,
            fixed: child.fixed,
            nested: child.nested,
            cert: child.cert,
            target: Vec::new(),
            attaining: 0,
            child_rule: 0,
            units: child.units,
            children: Vec::new(),
            one_twin_class: false,
            stopped_s: 0,
            stopped_e: 0,
        }
    }
}

/// One refinement step as the worked examples print it: the partitions in
/// colour order (local nodes and local lines) and the queue after the step.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "a trace is read only by the tests that render the worked examples"
    )
)]
#[derive(Clone, Debug)]
pub(crate) struct Step {
    pub(crate) kind: StepKind,
    pub(crate) nodes: Vec<Vec<usize>>,
    pub(crate) lines: Vec<Vec<usize>>,
    pub(crate) queue: Vec<(usize, u64)>,
}

/// What a [`Step`] did.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "a trace is read only by the tests that render the worked examples"
    )
)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum StepKind {
    /// The fresh start, before REFINE.
    Start,
    /// Steps 1 and 2 of individualising a local node.
    Indiv(usize),
    /// A processed queue pair `(side, colour)`.
    Pop(usize, u64),
}

/// One child of a branching call in a traced search.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "a trace is read only by the tests that render the worked examples"
    )
)]
#[derive(Clone, Debug)]
pub(crate) struct ChildTrace {
    /// The individualised local node.
    pub(crate) v: usize,
    /// Its refinement.
    pub(crate) steps: Vec<Step>,
    /// The child as a rule 1 or rule 2 call.
    pub(crate) tree: Tree,
}

/// A traced top component: its fresh refinement and its call structure.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "a trace is read only by the tests that render the worked examples"
    )
)]
#[derive(Clone, Debug)]
pub(crate) struct ComponentTrace {
    /// Global node index of each local node.
    pub(crate) nodes: Vec<usize>,
    /// Global line index of each local line.
    pub(crate) lines: Vec<usize>,
    /// The fresh start and its REFINE.
    pub(crate) steps: Vec<Step>,
    /// The call structure, every child of every branching call included.
    pub(crate) tree: Tree,
}

impl Pair {
    fn step(&self, kind: StepKind, queue: &Queue) -> Step {
        Step {
            kind,
            nodes: self.sides[0].cells.values().cloned().collect(),
            lines: self.sides[1].cells.values().cloned().collect(),
            queue: queue.iter().copied().collect(),
        }
    }
}

/// What stopped a call.
pub(crate) enum Stop {
    /// The search-free run met a target cell that is not one twin class.
    Failed,
    /// The engine's limit tripped.
    Limit,
    /// The caller's stop signal was set.
    Stopped,
}

/// A completed child of a call: what its fixing and nested calls produced.
#[derive(Clone)]
pub(crate) struct Child {
    pub(crate) fixed: Vec<(usize, u64)>,
    pub(crate) nested: Vec<Tree>,
    pub(crate) cert: Certificate,
    pub(crate) rule: u8,
    pub(crate) units: Units,
}

/// The labelling of one top component.
struct Engine<'a> {
    set: &'a LineSet,
    comp: &'a Comp,
    /// The units spent and the limit.
    meter: Meter,
    /// Record refinement steps and every child (traced searches only).
    tracing: bool,
    /// Comparison units (8.1(e)) of the method the examples state: children
    /// against the best so far, nested calls by insertion sort.
    comparison_units: u64,
    /// The twin test and the twin classes found, persisted over the component.
    twin: TwinTest,
    failures: Vec<(u64, u64, usize)>,
    /// The caller's stop signal, polled at each call.
    stop: Option<&'a dyn StopSignal>,
}

impl<'a> Engine<'a> {
    /// The label of the incidence of node `u` and line `l`: its free
    /// occurrence numbers, where `fixed` marks the nodes that are not free.
    fn label(&self, fixed: &[Option<u64>], u: usize, l: usize) -> Vec<u64> {
        let mut out = Vec::new();
        let mut number = 0;
        for &x in &self.comp.occ[l] {
            if fixed[x].is_some() {
                continue;
            }
            if x == u {
                out.push(number);
            }
            number += 1;
        }
        out
    }

    /// The incidences of vertex `x` of side `side` in the call.
    fn neighbours<'b>(
        &'b self,
        side: usize,
        x: usize,
        fixed: &'b [Option<u64>],
    ) -> impl Iterator<Item = usize> + 'b {
        let list = if side == 0 {
            &self.comp.node_lines[x]
        } else {
            &self.comp.line_nodes[x]
        };
        list.iter()
            .copied()
            .filter(move |&y| side == 0 || fixed[y].is_none())
    }

    fn incidence_label(&self, fixed: &[Option<u64>], side: usize, x: usize, y: usize) -> Vec<u64> {
        if side == 0 {
            self.label(fixed, x, y)
        } else {
            self.label(fixed, y, x)
        }
    }

    /// `KEY(x, W)`.
    fn key(
        &self,
        fixed: &[Option<u64>],
        side: usize,
        x: usize,
        w: &BTreeSet<usize>,
    ) -> Vec<Vec<u64>> {
        let mut key: Vec<Vec<u64>> = self
            .neighbours(side, x, fixed)
            .filter(|y| w.contains(y))
            .map(|y| self.incidence_label(fixed, side, x, y))
            .collect();
        key.sort();
        key
    }

    /// `SPLIT(X, W)` with the queue rule.
    fn split(
        &self,
        pair: &mut Pair,
        queue: &mut Queue,
        fixed: &[Option<u64>],
        side: usize,
        c: u64,
        w: &BTreeSet<usize>,
    ) {
        let cell = pair.sides[side].cells[&c].clone();
        let mut groups: BTreeMap<Vec<Vec<u64>>, Vec<usize>> = BTreeMap::new();
        for &x in &cell {
            groups
                .entry(self.key(fixed, side, x, w))
                .or_default()
                .push(x);
        }
        if groups.len() < 2 {
            return;
        }
        let mut fragments = Vec::new();
        let mut colour = c;
        let sd = &mut pair.sides[side];
        for (_, members) in groups {
            for &x in &members {
                sd.colour.insert(x, colour);
            }
            let size = members.len() as u64;
            fragments.push((colour, size));
            sd.cells.insert(colour, members);
            colour += size;
        }
        apply_queue_rule(queue, side, c, &fragments);
    }

    /// REFINE until the queue is empty; returns the units read. A traced
    /// refinement records each pop in `steps`.
    fn refine(
        &self,
        pair: &mut Pair,
        queue: &mut Queue,
        fixed: &[Option<u64>],
        mut steps: Option<&mut Vec<Step>>,
    ) -> u64 {
        let mut units = 0;
        let vertices = pair.sides[0].colour.len() + pair.sides[1].colour.len();
        let mut pops = 0usize;
        while let Some((side, c)) = queue.pop_first() {
            pops += 1;
            let w: BTreeSet<usize> = pair.sides[side].cells[&c].iter().copied().collect();
            let other = 1 - side;
            let mut listed = BTreeSet::new();
            for &x in &w {
                for y in self.neighbours(side, x, fixed) {
                    units += self.incidence_label(fixed, side, x, y).len() as u64;
                    let colour = pair.sides[other].colour[&y];
                    if pair.sides[other].cells[&colour].len() >= 2 {
                        listed.insert(colour);
                    }
                }
            }
            for colour in listed {
                self.split(pair, queue, fixed, other, colour, &w);
            }
            if let Some(steps) = steps.as_deref_mut() {
                steps.push(pair.step(StepKind::Pop(side, c), queue));
            }
        }
        debug_assert!(pops <= 2 * vertices, "at most 2V pops in one REFINE");
        units
    }

    /// Individualise node `v` and refine; returns the units read.
    fn individualise(
        &self,
        pair: &mut Pair,
        fixed: &[Option<u64>],
        v: usize,
        mut steps: Option<&mut Vec<Step>>,
    ) -> u64 {
        let side = &mut pair.sides[0];
        let c = side.colour[&v];
        let mut rest = side.cells[&c].clone();
        rest.retain(|&x| x != v);
        for &x in &rest {
            side.colour.insert(x, c + 1);
        }
        let size = rest.len() as u64;
        side.cells.insert(c, vec![v]);
        side.cells.insert(c + 1, rest);
        let mut queue = Queue::new();
        apply_queue_rule(&mut queue, 0, c, &[(c, 1), (c + 1, size)]);
        if let Some(steps) = steps.as_deref_mut() {
            steps.push(pair.step(StepKind::Indiv(v), &queue));
        }
        self.refine(pair, &mut queue, fixed, steps)
    }

    /// FIX the singleton node cells of `pair`, form the entries, and NEST.
    fn fix_and_nest(
        &mut self,
        pair: &Pair,
        lines: &[usize],
        p: u64,
        constants: &[Option<u64>],
    ) -> Result<Child, Stop> {
        let mut fixed_now = constants.to_vec();
        let mut fixed = Vec::new();
        for cell in pair.sides[0].cells.values() {
            if let [v] = cell.as_slice() {
                let q = p + fixed.len() as u64;
                fixed_now[*v] = Some(q);
                fixed.push((*v, q));
            }
        }
        let s = fixed.len() as u64;
        let mut units = Units::default();
        let mut entries = Vec::new();
        for &l in lines {
            // A line is complete when every occurrence has a path position.
            let positions: Option<Vec<u64>> =
                self.comp.occ[l].iter().map(|&x| fixed_now[x]).collect();
            if let Some(positions) = positions {
                let entry = entry_of(self.set, self.comp.lines[l], &positions);
                units.entries += 1 + (entry.encoding().len() as u64).div_ceil(8);
                entries.push(entry);
            }
        }
        entries.sort();
        let removed_incidences: u64 = fixed
            .iter()
            .map(|&(v, _)| self.comp.node_lines[v].len() as u64)
            .sum();
        units.fix += s + entries.len() as u64 + removed_incidences;
        // The free components of the remainder.
        let free: Vec<usize> = pair.sides[0]
            .cells
            .values()
            .filter(|cell| cell.len() >= 2)
            .flatten()
            .copied()
            .collect();
        let components = self.components_of(&free, &fixed_now);
        if components.len() >= 2 {
            let sizes: Vec<u64> = components
                .iter()
                .map(|(n, l)| (n.len() + l.len()) as u64)
                .collect();
            let largest = sizes.iter().copied().max().unwrap_or(0);
            units.fix += sizes.iter().sum::<u64>() - largest;
        }
        self.meter.charge(units)?;
        // Each nested call with the smallest source identifier of its nodes.
        let mut sourced: Vec<(&[u8], Tree)> = Vec::new();
        let mut failed = false;
        for (nodes, lines) in components {
            let inherited = Pair {
                sides: [
                    pair.sides[0].restrict(&nodes),
                    pair.sides[1].restrict(&lines),
                ],
            };
            let smallest = nodes
                .iter()
                .map(|&v| self.source_of(v))
                .min()
                .unwrap_or_default();
            let lines: Vec<usize> = lines.into_iter().collect();
            match self.call(&inherited, &lines, p + s, &fixed_now) {
                Ok(tree) => sourced.push((smallest, tree)),
                Err(Stop::Failed) => failed = true,
                Err(halt @ (Stop::Limit | Stop::Stopped)) => return Err(halt),
            }
        }
        if failed {
            return Err(Stop::Failed);
        }
        // Ascending by certificate, equal certificates by smallest source
        // (6.4, M2): an insertion sort, stable over the source order, whose
        // certificate comparisons are counted.
        sourced.sort_by(|a, b| a.0.cmp(b.0));
        let mut nested: Vec<Tree> = Vec::with_capacity(sourced.len());
        for (_, tree) in sourced {
            let mut at = nested.len();
            while at > 0 && self.compare(&nested[at - 1].cert, &tree.cert) == Ordering::Greater {
                at -= 1;
            }
            nested.insert(at, tree);
        }
        let cert = Certificate {
            s,
            entries,
            nested: nested.iter().map(|t| t.cert.clone()).collect(),
        };
        let rule = if nested.is_empty() && free.is_empty() {
            1
        } else {
            2
        };
        Ok(Child {
            fixed,
            nested,
            cert,
            rule,
            units,
        })
    }

    /// The order of two certificates (6.3), counting comparison units
    /// (8.1(e)): per pair of entries one unit for `m` and, when the values of
    /// `m` are equal, one per eight bytes of one encoding read up to the first
    /// differing byte, or the whole encoding when the two are equal.
    fn compare(&mut self, a: &Certificate, b: &Certificate) -> Ordering {
        let order = a.s.cmp(&b.s);
        if order != Ordering::Equal {
            return order;
        }
        for (x, y) in a.entries.iter().zip(&b.entries) {
            self.comparison_units += 1;
            let order = x.m().cmp(&y.m());
            if order != Ordering::Equal {
                return order;
            }
            let read = x
                .encoding()
                .iter()
                .zip(y.encoding())
                .position(|(p, q)| p != q)
                .map_or(x.encoding().len(), |i| i + 1);
            self.comparison_units += (read as u64).div_ceil(8);
            let order = x.encoding().cmp(y.encoding());
            if order != Ordering::Equal {
                return order;
            }
        }
        let order = a.entries.len().cmp(&b.entries.len());
        if order != Ordering::Equal {
            return order;
        }
        for (x, y) in a.nested.iter().zip(&b.nested) {
            let order = self.compare(x, y);
            if order != Ordering::Equal {
                return order;
            }
        }
        a.nested.len().cmp(&b.nested.len())
    }

    /// The connected components of `free` through lines, with the lines that
    /// hold at least one of their nodes.
    fn components_of(
        &self,
        free: &[usize],
        fixed: &[Option<u64>],
    ) -> Vec<(BTreeSet<usize>, BTreeSet<usize>)> {
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for &start in free {
            if !seen.insert(start) {
                continue;
            }
            let mut nodes = BTreeSet::from([start]);
            let mut lines = BTreeSet::new();
            let mut stack = vec![start];
            while let Some(u) = stack.pop() {
                for &l in &self.comp.node_lines[u] {
                    lines.insert(l);
                    for &x in &self.comp.line_nodes[l] {
                        if fixed[x].is_none() && seen.insert(x) {
                            nodes.insert(x);
                            stack.push(x);
                        }
                    }
                }
            }
            out.push((nodes, lines));
        }
        out
    }

    fn source_of(&self, v: usize) -> &'a [u8] {
        self.set.source(self.comp.nodes[v])
    }

    /// A call on the component of `pair` with argument `p` and constants.
    fn call(
        &mut self,
        pair: &Pair,
        lines: &[usize],
        p: u64,
        constants: &[Option<u64>],
    ) -> Result<Tree, Stop> {
        if is_stopped(self.stop) {
            return Err(Stop::Stopped);
        }
        let n_k = pair.sides[0].colour.len() as u64;
        let has_singleton = pair.sides[0].cells.values().any(|cell| cell.len() == 1);
        if has_singleton {
            let child = self.fix_and_nest(pair, lines, p, constants)?;
            return Ok(Tree::of_child(child, p, n_k));
        }
        // Rule 3: the target cell is the node cell of colour 0.
        let mut target = pair.sides[0].cells[&0].clone();
        target.sort_by(|&a, &b| self.source_of(a).cmp(self.source_of(b)));
        let explore: Vec<usize> = if self.meter.search_free {
            let u = target[0];
            for &w in &target[1..] {
                let (ru, rw) = (self.twin.find(u), self.twin.find(w));
                if ru == rw {
                    continue;
                }
                if self.twin.test(self.set, self.comp, &mut self.meter, u, w) {
                    self.twin.join(ru, rw);
                } else {
                    self.failures.push((p, target.len() as u64, u));
                    return Err(Stop::Failed);
                }
            }
            vec![u]
        } else {
            target.clone()
        };
        // Children in ascending source order; the best is replaced only by a
        // strictly smaller certificate, so it ends as the attaining child of
        // smallest source identifier, the chosen child (M1, Lemma M (iv)).
        let mut best: Option<Child> = None;
        let mut attaining = 0u64;
        let mut children = Vec::new();
        for &v in &explore {
            let mut child_pair = pair.clone();
            let mut steps = Vec::new();
            let refine_units = self.individualise(
                &mut child_pair,
                constants,
                v,
                self.tracing.then_some(&mut steps),
            );
            self.meter.charge(Units {
                refine: refine_units,
                ..Units::default()
            })?;
            let mut child = self.fix_and_nest(&child_pair, lines, p, constants)?;
            child.units.refine += refine_units;
            if self.tracing {
                children.push(ChildTrace {
                    v,
                    steps,
                    tree: Tree::of_child(child.clone(), p, n_k),
                });
            }
            let order = match &best {
                None => Ordering::Less,
                Some(b) => self.compare(&child.cert, &b.cert),
            };
            match order {
                Ordering::Less => {
                    attaining = 1;
                    best = Some(child);
                }
                Ordering::Equal => attaining += 1,
                Ordering::Greater => {}
            }
        }
        if self.meter.search_free {
            attaining = target.len() as u64;
        }
        // INVARIANT: a target cell of rule 3 has at least two members and
        // `explore` at least one, so a best child exists.
        let Some(child) = best else {
            return Err(Stop::Failed);
        };
        let one_twin_class = self.tracing
            && target[1..]
                .iter()
                .all(|&w| self.twin.is_pair(self.set, self.comp, target[0], w));
        let mut tree = Tree::of_child(child, p, n_k);
        tree.child_rule = tree.rule;
        tree.rule = 3;
        tree.target = target;
        tree.attaining = attaining;
        tree.children = children;
        tree.one_twin_class = one_twin_class;
        Ok(tree)
    }
}

/// The fragments of a split cell that go to the queue (5.5 of the
/// definition): all of them when the split cell is already queued, otherwise
/// all but the largest, which is the one of smallest colour among the largest.
/// The one home of the rule, for every engine.
pub(crate) fn fragments_to_queue<C: Copy + Ord, S: Copy + Ord>(
    split_is_queued: bool,
    fragments: &[(C, S)],
) -> Vec<C> {
    if split_is_queued {
        return fragments.iter().map(|&(colour, _)| colour).collect();
    }
    let largest = fragments
        .iter()
        .copied()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|f| f.0);
    fragments
        .iter()
        .map(|&(colour, _)| colour)
        .filter(|&colour| Some(colour) != largest)
        .collect()
}

fn apply_queue_rule(queue: &mut Queue, side: usize, c: u64, fragments: &[(u64, u64)]) {
    for colour in fragments_to_queue(queue.contains(&(side, c)), fragments) {
        queue.insert((side, colour));
    }
}

/// The units one component's engine has spent: the figures it reports and the
/// limited units its limit is compared with (8.1, 8.3). Both engines meter
/// through it.
pub(crate) struct Meter {
    /// Units over the calls explored.
    pub(crate) explored: Units,
    /// Twin units of the search-free run (8.1(f)).
    pub(crate) twin_units: u64,
    /// The limited units spent on the input so far.
    pub(crate) limited: u64,
    limit: u64,
    /// Whether the run is the search-free run, which no limit may refuse.
    pub(crate) search_free: bool,
}

impl Meter {
    pub(crate) fn new(limit: u64) -> Self {
        Self {
            explored: Units::default(),
            twin_units: 0,
            limited: 0,
            limit,
            search_free: true,
        }
    }

    /// Charge `units` of a fixing or a refinement.
    ///
    /// # Errors
    /// [`Stop::Limit`] when a search passes its limit.
    pub(crate) fn charge(&mut self, units: Units) -> Result<(), Stop> {
        self.explored = self.explored.saturating_add(units);
        self.limited += units.refine + units.fix + units.entries;
        // The search-free run is bounded by the allowance (Lemma A) and no
        // limit may refuse it; only a search is limited.
        if !self.search_free && self.limited > self.limit {
            return Err(Stop::Limit);
        }
        Ok(())
    }
}

/// The twin test of 7.10 over the lines of one top component, and the twin
/// classes found so far, kept for the whole component (a union-find). Both
/// engines test through it.
pub(crate) struct TwinTest {
    classes: Classes,
    /// The component's lines as typed values.
    lines: FastMap<Box<[Tok]>, ()>,
    /// The start charge of each line of the component (8.1(a)).
    start_charge: Vec<u64>,
}

/// A union-find over `0..n` with path halving.
pub(crate) struct Classes {
    parent: Vec<usize>,
}

impl Classes {
    pub(crate) fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    /// The representative of `x`'s class, halving the path on the way up.
    pub(crate) fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    /// Join the class of representative `other` into that of `into`.
    pub(crate) fn join(&mut self, into: usize, other: usize) {
        self.parent[other] = into;
    }
}

impl TwinTest {
    pub(crate) fn new(set: &LineSet, comp: &Comp, start_charge: Vec<u64>) -> Self {
        Self {
            classes: Classes::new(comp.nodes.len()),
            lines: comp
                .lines
                .iter()
                .map(|&l| (set.lines[l].clone(), ()))
                .collect(),
            start_charge,
        }
    }

    /// The class representative of local node `x`.
    pub(crate) fn find(&mut self, x: usize) -> usize {
        self.classes.find(x)
    }

    /// Join the class of representative `into` with that of `other`.
    pub(crate) fn join(&mut self, into: usize, other: usize) {
        self.classes.join(into, other);
    }

    /// The twin test of local nodes `u` and `w` with its twin units (7.10,
    /// 8.1(f)): a test, passing or failing, is charged the start charge of
    /// every line that holds `u` or `w`; the outcome and the order of the
    /// lines play no part. The figure depends on the pairs tested, hence on
    /// the source order: an engine figure only.
    pub(crate) fn test(
        &self,
        set: &LineSet,
        comp: &Comp,
        meter: &mut Meter,
        u: usize,
        w: usize,
    ) -> bool {
        let mut holding: Vec<usize> = comp.node_lines[u]
            .iter()
            .chain(&comp.node_lines[w])
            .copied()
            .collect();
        holding.sort_unstable();
        holding.dedup();
        let charge: u64 = holding.iter().map(|&l| self.start_charge[l]).sum();
        // The figure is that of the search-free run (8.1(f)): the tests an
        // engine makes during a search spend limited units and no more.
        if meter.search_free {
            meter.twin_units += charge;
        }
        meter.limited += charge;
        self.is_pair(set, comp, u, w)
    }

    /// Whether the component holds a line of exactly these tokens.
    pub(crate) fn holds(&self, tokens: &[Tok]) -> bool {
        self.lines.contains_key(tokens)
    }

    /// Whether exchanging `u` and `w` maps the lines of the component onto
    /// themselves (Lemma TW (a): each line holding either has its image).
    pub(crate) fn is_pair(&self, set: &LineSet, comp: &Comp, u: usize, w: usize) -> bool {
        let (gu, gw) = (comp.nodes[u], comp.nodes[w]);
        comp.node_lines[u]
            .iter()
            .chain(&comp.node_lines[w])
            .all(|&l| {
                let image: Box<[Tok]> = set.lines[comp.lines[l]]
                    .iter()
                    .map(|t| match *t {
                        Tok::Blank(b) if b == gu => Tok::Blank(gw),
                        Tok::Blank(b) if b == gw => Tok::Blank(gu),
                        other => other,
                    })
                    .collect();
                self.lines.contains_key(&image)
            })
    }
}

/// Union-find parents of the nodes of `set`: two nodes of one line share a root.
fn joined_nodes(set: &LineSet) -> Vec<usize> {
    let mut parent: Vec<usize> = (0..set.nodes.len()).collect();
    for l in 0..set.lines.len() {
        let mut first = None;
        for b in set.occurrences(l) {
            match first {
                None => first = Some(b),
                Some(a) => {
                    let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                    if ra != rb {
                        parent[rb] = ra;
                    }
                }
            }
        }
    }
    parent
}

/// The root of `x` in `parent`, compressing the path.
fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

/// The number of top components of `set`.
pub(crate) fn component_count(set: &LineSet) -> usize {
    joined_nodes(set)
        .iter()
        .enumerate()
        .filter(|&(x, &parent)| x == parent)
        .count()
}

/// The top components of `set`, each with its nodes in node order.
pub(crate) fn top_components(set: &LineSet) -> Vec<Comp> {
    let n = set.nodes.len();
    let mut parent = joined_nodes(set);
    let mut index: BTreeMap<usize, usize> = BTreeMap::new();
    let mut comps: Vec<(Vec<usize>, Vec<usize>)> = Vec::new();
    for b in 0..n {
        let root = find(&mut parent, b);
        let next = comps.len();
        let i = *index.entry(root).or_insert(next);
        if i == comps.len() {
            comps.push((Vec::new(), Vec::new()));
        }
        comps[i].0.push(b);
    }
    for l in 0..set.lines.len() {
        if let Some(b) = set.occurrences(l).next() {
            let i = index[&find(&mut parent, b)];
            comps[i].1.push(l);
        }
    }
    comps
        .into_iter()
        .map(|(nodes, lines)| {
            let local: FastMap<usize, usize> =
                nodes.iter().enumerate().map(|(i, &g)| (g, i)).collect();
            let occ: Vec<Vec<usize>> = lines
                .iter()
                .map(|&l| set.occurrences(l).map(|b| local[&b]).collect())
                .collect();
            let mut node_lines = vec![Vec::new(); nodes.len()];
            let mut line_nodes = Vec::with_capacity(lines.len());
            for (j, o) in occ.iter().enumerate() {
                let mut distinct = o.clone();
                distinct.sort_unstable();
                distinct.dedup();
                for &u in &distinct {
                    node_lines[u].push(j);
                }
                line_nodes.push(distinct);
            }
            Comp {
                nodes,
                lines,
                occ,
                node_lines,
                line_nodes,
            }
        })
        .collect()
}

/// A sum in the allowance: it saturates at `2^64 - 1` (7.10), where it means
/// that the engine's own limit never trips; nothing is refused for it.
pub(crate) const fn add(a: u64, b: u64) -> u64 {
    a.saturating_add(b)
}

/// A product in the allowance, saturating like [`add`].
pub(crate) const fn mul(a: u64, b: u64) -> u64 {
    a.saturating_mul(b)
}

/// The part of a component's record known before any search.
pub(crate) struct Static {
    /// The record with its static fields and the START units filled.
    pub(crate) record: ComponentRecord,
    /// The start charge of each local line.
    pub(crate) start: Vec<u64>,
    /// `W`: the sum over lines of distinct nodes times start charge.
    pub(crate) w: u64,
}

/// The static part of a component's record and its allowance.
pub(crate) fn static_record(set: &LineSet, comp: &Comp) -> Result<Static, Refusal> {
    let n = comp.nodes.len() as u64;
    let m = comp.lines.len() as u64;
    let o: u64 = comp.occ.iter().map(|x| x.len() as u64).sum();
    let incidences: u64 = comp.line_nodes.iter().map(|x| x.len() as u64).sum();
    let mut line_bytes = 0u64;
    let mut start = Vec::with_capacity(comp.lines.len());
    let mut w = 0u64;
    for (j, &l) in comp.lines.iter().enumerate() {
        let b = set.free_len(l);
        line_bytes = line_bytes.checked_add(b).ok_or(Refusal::Width {
            quantity: Quantity::Length,
        })?;
        let c = 1 + b.div_ceil(8);
        start.push(c);
        w = add(w, mul(comp.line_nodes[j].len() as u64, c));
    }
    // `S` and `W` belong to the allowance and saturate with it.
    let s = start.iter().fold(0, |s, &c| add(s, c));
    let v = n + m;
    let bv = bits(v);
    let mut allowance = mul(2, s);
    allowance = add(allowance, o);
    allowance = add(allowance, mul(mul(2, o), bv));
    allowance = add(allowance, incidences);
    allowance = add(allowance, mul(v, bv));
    allowance = add(allowance, mul(4, w));
    let record = ComponentRecord {
        blank_nodes: n,
        lines: m,
        label_elements: o,
        incidences,
        line_bytes,
        forest: incidences + 1 == n + m,
        allowance,
        units: Units {
            start: s,
            ..Units::default()
        },
        ..ComponentRecord::default()
    };
    Ok(Static { record, start, w })
}

/// The fresh start of a top component, refined.
fn fresh_pair(engine: &Engine<'_>, mut steps: Option<&mut Vec<Step>>) -> (Pair, u64) {
    let comp = engine.comp;
    let free = vec![None; comp.nodes.len()];
    let mut by_encoding: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
    for (j, &l) in comp.lines.iter().enumerate() {
        let mut enc = Vec::new();
        engine.set.encode(l, |_| Leaf::Free, &mut enc);
        by_encoding.entry(enc).or_default().push(j);
    }
    let mut pair = Pair {
        sides: [
            Side::new([(0..comp.nodes.len()).collect::<Vec<_>>()]),
            Side::new(by_encoding.into_values()),
        ],
    };
    let mut queue = Queue::new();
    for (side, part) in pair.sides.iter().enumerate() {
        for &c in part.cells.keys() {
            queue.insert((side, c));
        }
    }
    if let Some(steps) = steps.as_deref_mut() {
        steps.push(pair.step(StepKind::Start, &queue));
    }
    let units = engine.refine(&mut pair, &mut queue, &free, steps);
    (pair, units)
}

/// The record fields read along the result path.
pub(crate) fn path_record(
    tree: &Tree,
    depth: u64,
    branching: u64,
    record: &mut ComponentRecord,
) -> Option<()> {
    match tree.rule {
        1 => record.calls.discrete += 1,
        2 => record.calls.nest += 1,
        _ => {
            record.calls.branch += 1;
            match tree.child_rule {
                1 => record.chosen_children.discrete += 1,
                _ => record.chosen_children.nest += 1,
            }
            record.target_sizes.push(tree.target.len() as u64);
            record.attaining.push(tree.attaining);
        }
    }
    let branching = branching + u64::from(tree.rule == 3);
    record.nesting_depth = record.nesting_depth.max(depth);
    record.branching_depth = record.branching_depth.max(branching);
    record.units = record.units.checked_add(Units {
        start: 0,
        ..tree.units
    })?;
    if !tree.nested.is_empty() {
        record.nestings.push(classes(&tree.nested));
    }
    for nested in &tree.nested {
        path_record(nested, depth + 1, branching, record)?;
    }
    Some(())
}

/// The multiplicities of the runs of equal certificate among ordered calls.
fn classes(nested: &[Tree]) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new();
    for (i, t) in nested.iter().enumerate() {
        if i > 0 && nested[i - 1].cert == t.cert {
            if let Some(last) = out.last_mut() {
                *last += 1;
            }
        } else {
            out.push(1);
        }
    }
    out
}

/// `process(X, e)` of the factorisation list.
pub(crate) fn process(tree: &Tree, e: u64, out: &mut Vec<Factor>) {
    if tree.rule == 3 && tree.attaining >= 2 {
        out.push(Factor {
            kind: FactorKind::Attain,
            value: tree.attaining,
            exponent: e,
        });
    }
    let mut i = 0;
    while i < tree.nested.len() {
        let mut k = 1;
        while i + k < tree.nested.len() && tree.nested[i + k].cert == tree.nested[i].cert {
            k += 1;
        }
        if k >= 2 {
            out.push(Factor {
                kind: FactorKind::Sym,
                value: k as u64,
                exponent: e,
            });
            process(&tree.nested[i], e * k as u64, out);
        } else {
            process(&tree.nested[i], e, out);
        }
        i += k;
    }
}

/// Merge equal `(kind, value)` and sort.
pub(crate) fn merge_factors(mut factors: Vec<Factor>) -> Vec<Factor> {
    factors.sort();
    let mut out: Vec<Factor> = Vec::new();
    for f in factors {
        match out.last_mut() {
            Some(last) if last.kind == f.kind && last.value == f.value => {
                last.exponent += f.exponent;
            }
            _ => out.push(f),
        }
    }
    out
}

/// Final positions of the nodes of a result path, by local node.
pub(crate) fn positions(tree: &Tree, o: u64, out: &mut [u64]) {
    for &(v, q) in &tree.fixed {
        out[v] = o + (q - tree.p);
    }
    let s = tree.fixed.len() as u64;
    let mut t = 0;
    for nested in &tree.nested {
        positions(nested, o + s + t, out);
        t += nested.n_k;
    }
}

/// One top component between its search-free run and its search.
struct Run<'a> {
    engine: Engine<'a>,
    comp: &'a Comp,
    pair: Pair,
    fresh_units: u64,
    record: ComponentRecord,
    w: u64,
    smallest_node: usize,
    tree: Option<Tree>,
    /// The fresh start and its REFINE, when traced.
    steps: Vec<Step>,
}

impl<'a> Run<'a> {
    /// The fresh start of `comp`, refined, with the record fields it fixes.
    fn new(
        set: &'a LineSet,
        comp: &'a Comp,
        stat: Static,
        limit: u64,
        tracing: bool,
        stop: Option<&'a dyn StopSignal>,
    ) -> Self {
        let Static {
            mut record,
            start,
            w,
        } = stat;
        let mut engine = Engine {
            set,
            comp,
            meter: Meter::new(limit),
            tracing,
            comparison_units: 0,
            twin: TwinTest::new(set, comp, start),
            failures: Vec::new(),
            stop,
        };
        let mut steps = Vec::new();
        let (pair, fresh_units) = fresh_pair(&engine, tracing.then_some(&mut steps));
        let sizes: Vec<usize> = pair.sides[0].cells.values().map(Vec::len).collect();
        record.singled_out = sizes.iter().filter(|&&s| s == 1).count() as u64;
        record.unsettled = record.blank_nodes - record.singled_out;
        record.largest_cell = sizes
            .iter()
            .copied()
            .filter(|&s| s >= 2)
            .max()
            .map(|s| s as u64);
        record.units.refine = fresh_units;
        engine.meter.explored.start = record.units.start;
        engine.meter.explored.refine = fresh_units;
        let smallest_node = (0..comp.nodes.len())
            .min_by(|&a, &b| engine.source_of(a).cmp(engine.source_of(b)))
            .unwrap_or(0);
        Self {
            engine,
            comp,
            pair,
            fresh_units,
            record,
            w,
            smallest_node,
            tree: None,
            steps,
        }
    }
}

impl ComponentRun for Run<'_> {
    fn base_units(&self) -> u64 {
        self.record.units.start + self.fresh_units
    }

    fn top_call(&mut self, search_free: bool, spent: &mut u64) -> CallOutcome {
        self.engine.meter.search_free = search_free;
        self.engine.meter.limited = *spent;
        let lines: Vec<usize> = (0..self.comp.lines.len()).collect();
        let constants = vec![None; self.comp.nodes.len()];
        let result = self.engine.call(&self.pair, &lines, 0, &constants);
        *spent = self.engine.meter.limited;
        match result {
            Ok(tree) => {
                self.tree = Some(tree);
                CallOutcome::Done
            }
            Err(Stop::Failed) => CallOutcome::Failed,
            Err(Stop::Limit) => CallOutcome::Limit,
            Err(Stop::Stopped) => CallOutcome::Stopped,
        }
    }

    fn is_labelled(&self) -> bool {
        self.tree.is_some()
    }

    fn failures(&self) -> Vec<FailureDiagnostic> {
        failure_diagnostics(
            self.engine.set,
            self.comp,
            &self.record,
            self.smallest_node,
            &self.engine.failures,
        )
    }

    fn source(&self) -> &[u8] {
        self.engine.source_of(self.smallest_node)
    }

    fn blank_nodes(&self) -> u64 {
        self.record.blank_nodes
    }

    fn finish(mut self) -> Result<Option<Labelling>, Refusal> {
        let Some(tree) = self.tree.take() else {
            return Ok(None);
        };
        let parts = LabellingParts {
            smallest_node: self.smallest_node,
            twin_units: self.engine.meter.twin_units,
            w: self.w,
            steps: self.steps,
            comparison_units: self.engine.comparison_units,
            explored: self.engine.meter.explored,
        };
        finish_labelling(self.record, tree, parts).map(Some)
    }
}

/// What the labelling of one top component yields.
pub(crate) struct Labelling {
    pub(crate) tree: Tree,
    pub(crate) record: ComponentRecord,
    pub(crate) identity: [u8; 32],
    pub(crate) smallest_node: usize,
    pub(crate) twin_units: u64,
    pub(crate) w: u64,
    pub(crate) steps: Vec<Step>,
    pub(crate) comparison_units: u64,
    /// Units over the calls the engine explored on this component.
    pub(crate) explored: Units,
    /// Children stopped on `s` and on `E` at the branching calls of the
    /// result path (10.4).
    pub(crate) stopped: (u64, u64),
}

/// The record fields of the result path, the group order and the identity.
pub(crate) fn finish_labelling(
    mut record: ComponentRecord,
    tree: Tree,
    parts: LabellingParts,
) -> Result<Labelling, Refusal> {
    path_record(&tree, 0, 0, &mut record).ok_or(UNITS_WIDTH)?;
    let mut factors = Vec::new();
    process(&tree, 1, &mut factors);
    record.group_order = merge_factors(factors);
    let identity = digest_of(COMPONENT_CONTEXT, &tree.cert.encoding());
    let mut stopped = (0, 0);
    let mut pending = vec![&tree];
    while let Some(call) = pending.pop() {
        stopped.0 += call.stopped_s;
        stopped.1 += call.stopped_e;
        pending.extend(&call.nested);
    }
    Ok(Labelling {
        stopped,
        tree,
        record,
        identity,
        smallest_node: parts.smallest_node,
        twin_units: parts.twin_units,
        w: parts.w,
        steps: parts.steps,
        comparison_units: parts.comparison_units,
        explored: parts.explored,
    })
}

/// What an engine reports besides the tree and the record of a component.
pub(crate) struct LabellingParts {
    pub(crate) smallest_node: usize,
    pub(crate) twin_units: u64,
    pub(crate) w: u64,
    pub(crate) steps: Vec<Step>,
    pub(crate) comparison_units: u64,
    pub(crate) explored: Units,
}

/// The top components of a set with their static records and the allowance.
pub(crate) struct Prepared {
    pub(crate) comps: Vec<Comp>,
    pub(crate) statics: Vec<Static>,
    /// `A`, including the charge of the split into top components.
    pub(crate) allowance: u64,
    /// The charge of the split into top components (8.1(c)).
    pub(crate) split_charge: u64,
}

/// The components of `set` with their static parts, the allowance first so
/// that a width refusal precedes any search.
pub(crate) fn prepare(set: &LineSet) -> Result<Prepared, Refusal> {
    let comps = top_components(set);
    let mut allowance = 0u64;
    let mut statics = Vec::with_capacity(comps.len());
    let mut split_sizes = Vec::with_capacity(comps.len());
    for comp in &comps {
        let stat = static_record(set, comp)?;
        allowance = add(allowance, stat.record.allowance);
        split_sizes.push((comp.nodes.len() + comp.lines.len()) as u64);
        statics.push(stat);
    }
    let split_charge = if split_sizes.len() >= 2 {
        split_sizes.iter().sum::<u64>() - split_sizes.iter().copied().max().unwrap_or(0)
    } else {
        0
    };
    allowance = add(allowance, split_charge);
    Ok(Prepared {
        comps,
        statics,
        allowance,
        split_charge,
    })
}

/// A sum of the units of the result path past 64 bits (7.11).
pub(crate) const UNITS_WIDTH: Refusal = Refusal::Width {
    quantity: Quantity::Units,
};

/// Label an admitted set with the reference engine. `force_search` skips the
/// search-free run and searches every component (a test of their agreement).
///
/// # Errors
/// `width` when the allowance overflows; `limit` when a search exceeds
/// `max(limit, A)` limited units.
pub(crate) fn label_set(
    set: &LineSet,
    limit: u64,
    force_search: bool,
) -> Result<Labelled, Refusal> {
    label_traced(set, limit, force_search, false, None)
        .map(|(labelled, _)| labelled)
        .map_err(Halt::expect_refusal)
}

/// [`label_set`], and with `tracing` the refinement steps and every child of
/// every branching call of each top component, in the order of 7.3.
///
/// # Errors
/// As [`label_set`]; [`Halt::Stopped`] when `stop` is set, polled before the
/// first call and at every call.
pub(crate) fn label_traced(
    set: &LineSet,
    limit: u64,
    force_search: bool,
    tracing: bool,
    stop: Option<&dyn StopSignal>,
) -> Result<(Labelled, Vec<ComponentTrace>), Halt> {
    if is_stopped(stop) {
        return Err(Halt::Stopped);
    }
    let Prepared {
        comps,
        statics,
        allowance,
        split_charge,
    } = prepare(set).map_err(Halt::Refused)?;
    let limit = limit.max(allowance);
    let runs: Vec<Run<'_>> = comps
        .iter()
        .zip(statics)
        .map(|(comp, stat)| Run::new(set, comp, stat, limit, tracing, stop))
        .collect();
    drive(
        set,
        &comps,
        runs,
        allowance,
        split_charge,
        force_search,
        tracing,
    )
}

/// How a top call of a component ended.
pub(crate) enum CallOutcome {
    /// The component is labelled.
    Done,
    /// The search-free run met a target cell that is not one twin class.
    Failed,
    /// The engine's limit tripped.
    Limit,
    /// The caller's stop signal was set.
    Stopped,
}

/// One top component of an engine, between its search-free run and its search.
pub(crate) trait ComponentRun {
    /// The limited units spent before any call: START and the fresh refinement.
    fn base_units(&self) -> u64;
    /// The top call, the search-free run or a search, continuing the limited
    /// units `spent` on the input so far.
    fn top_call(&mut self, search_free: bool, spent: &mut u64) -> CallOutcome;
    /// Whether a top call has labelled the component.
    fn is_labelled(&self) -> bool;
    /// The first failures recorded by the search-free run.
    fn failures(&self) -> Vec<FailureDiagnostic>;
    /// The smallest source identifier of the component.
    fn source(&self) -> &[u8];
    /// The number of nodes of the component.
    fn blank_nodes(&self) -> u64;
    /// The labelling, when the component is labelled.
    ///
    /// # Errors
    /// `width` when a sum of units of the result path passes 64 bits.
    fn finish(self) -> Result<Option<Labelling>, Refusal>;
}

/// The first failures of one component as typed values with their sources.
pub(crate) fn failure_diagnostics(
    set: &LineSet,
    comp: &Comp,
    record: &ComponentRecord,
    smallest_node: usize,
    failures: &[(u64, u64, usize)],
) -> Vec<FailureDiagnostic> {
    failures
        .iter()
        .map(|&(p, size, u)| FailureDiagnostic {
            record: FailureRecord {
                path_position: p,
                tied_cell: size,
                component_nodes: record.blank_nodes,
                component_lines: record.lines,
            },
            component_source: set.source(comp.nodes[smallest_node]).into(),
            cell_source: set.source(comp.nodes[u]).into(),
        })
        .collect()
}

/// The labelling of every top component by the runs of an engine: the
/// search-free run on every component before any search, so that every first
/// failure is known before a limit can trip (7.10); then a search of each
/// component the search-free run did not label; then the result.
///
/// # Errors
/// The refusal `limit` when a search trips the limit, `width` when a sum
/// overflows; [`Halt::Stopped`] when the caller's signal is set.
pub(crate) fn drive<R: ComponentRun>(
    set: &LineSet,
    comps: &[Comp],
    mut runs: Vec<R>,
    allowance: u64,
    split_charge: u64,
    force_search: bool,
    tracing: bool,
) -> Result<(Labelled, Vec<ComponentTrace>), Halt> {
    let mut spent = runs
        .iter()
        .fold(0u64, |spent, run| spent.saturating_add(run.base_units()));
    if !force_search {
        for run in &mut runs {
            // A first failure is recorded in the engine; the search-free run
            // never checks the limit (Lemma A bounds it by the allowance).
            if matches!(run.top_call(true, &mut spent), CallOutcome::Stopped) {
                return Err(Halt::Stopped);
            }
        }
    }
    let mut failures: Vec<FailureDiagnostic> =
        runs.iter().flat_map(ComponentRun::failures).collect();
    failures.sort();
    for run in &mut runs {
        if run.is_labelled() {
            continue;
        }
        // A search never stops on a failure (it explores every child of a
        // non-empty target cell), so a stop is the limit.
        match run.top_call(false, &mut spent) {
            CallOutcome::Done => {}
            CallOutcome::Stopped => return Err(Halt::Stopped),
            CallOutcome::Limit | CallOutcome::Failed => {
                return Err(Halt::Refused(Refusal::Limit {
                    broken_by: failures.iter().map(|f| f.record).collect(),
                    source: run.source().into(),
                    nodes: run.blank_nodes(),
                    units: spent,
                }));
            }
        }
    }
    let mut labellings = Vec::with_capacity(runs.len());
    for run in runs {
        // INVARIANT: every run is labelled here (the search-free run or a
        // search completed it), so `labellings` stays aligned with `comps`.
        if let Some(labelling) = run.finish().map_err(Halt::Refused)? {
            labellings.push(labelling);
        }
    }
    if force_search {
        failures = full_search_failures(set, comps, &labellings);
    }
    assemble(
        set,
        comps,
        &labellings,
        failures,
        allowance,
        split_charge,
        tracing,
    )
}

/// The result over the labellings of every top component: components in their
/// order, positions, identities, the structural record, the canonical line
/// sequence and its digest. `failures` are the first failures, ascending.
pub(crate) fn assemble(
    set: &LineSet,
    comps: &[Comp],
    labellings: &[Labelling],
    failures: Vec<FailureDiagnostic>,
    allowance: u64,
    split_charge: u64,
    tracing: bool,
) -> Result<(Labelled, Vec<ComponentTrace>), Halt> {
    let mut explored = Units {
        fix: split_charge,
        ..Units::default()
    };
    let mut twin_units = 0;
    let mut comparison_units = 0;
    let mut stopped = (0, 0);
    for l in labellings {
        explored = explored.saturating_add(l.explored);
        twin_units += l.twin_units;
        comparison_units += l.comparison_units;
        stopped.0 += l.stopped.0;
        stopped.1 += l.stopped.1;
    }
    // Order the top components by identity, then smallest source.
    let mut order: Vec<usize> = (0..labellings.len()).collect();
    order.sort_by(|&a, &b| {
        let (la, lb) = (&labellings[a], &labellings[b]);
        la.identity.cmp(&lb.identity).then_with(|| {
            set.source(comps[a].nodes[la.smallest_node])
                .cmp(set.source(comps[b].nodes[lb.smallest_node]))
        })
    });
    let mut placements = vec![
        Placement {
            position: 0,
            identity: Identity {
                component: [0; 32],
                local_position: 0,
                copy_index: 0,
            },
        };
        set.nodes.len()
    ];
    let mut components = Vec::with_capacity(order.len());
    let mut record = StatisticsRecord {
        top_components: comps.len() as u64,
        allowance,
        ..StatisticsRecord::default()
    };
    let mut offset = 0u64;
    let mut histogram: BTreeMap<u64, u64> = BTreeMap::new();
    let mut groups: Vec<Factor> = Vec::new();
    let mut traces = Vec::new();
    let mut totals: BTreeMap<[u8; 32], u64> = BTreeMap::new();
    for l in labellings.iter() {
        *totals.entry(l.identity).or_default() += 1;
    }
    let mut seen: BTreeMap<[u8; 32], u64> = BTreeMap::new();
    for &i in &order {
        let (l, comp) = (&labellings[i], &comps[i]);
        if tracing {
            traces.push(ComponentTrace {
                nodes: comp.nodes.clone(),
                lines: comp.lines.clone(),
                steps: l.steps.clone(),
                tree: l.tree.clone(),
            });
        }
        let seen = seen.entry(l.identity).or_default();
        let copy_index = *seen;
        *seen += 1;
        let mut local = vec![0u64; comp.nodes.len()];
        positions(&l.tree, 0, &mut local);
        for (v, &lp) in local.iter().enumerate() {
            placements[comp.nodes[v]] = Placement {
                position: offset + lp,
                identity: Identity {
                    component: l.identity,
                    local_position: lp,
                    copy_index,
                },
            };
        }
        components.push(Component {
            identity: l.identity,
            offset,
            nodes: comp.nodes.len() as u64,
            copy_index,
            smallest_node: comp.nodes[l.smallest_node],
            certificate: l.tree.cert.clone(),
            twin_units: l.twin_units,
            w: l.w,
        });
        offset += comp.nodes.len() as u64;
        let r = &l.record;
        *histogram.entry(bits(r.blank_nodes) - 1).or_default() += 1;
        record.single_node_components += u64::from(r.blank_nodes == 1);
        record.blank_nodes += r.blank_nodes;
        record.label_elements += r.label_elements;
        record.incidences += r.incidences;
        record.singled_out += r.singled_out;
        record.unsettled += r.unsettled;
        record.blank_bearing_lines += r.lines;
        record.blank_bearing_bytes += r.line_bytes;
        record.calls.discrete += r.calls.discrete;
        record.calls.nest += r.calls.nest;
        record.calls.branch += r.calls.branch;
        record.chosen_children.discrete += r.chosen_children.discrete;
        record.chosen_children.nest += r.chosen_children.nest;
        record.units = record
            .units
            .checked_add(r.units)
            .ok_or(Halt::Refused(UNITS_WIDTH))?;
        if copy_index == 0 {
            let k = totals[&l.identity];
            record.top_multiplicities.push(k);
            if k >= 2 {
                groups.push(Factor {
                    kind: FactorKind::Sym,
                    value: k,
                    exponent: 1,
                });
            }
            let mut factors = Vec::new();
            process(&l.tree, k, &mut factors);
            groups.extend(factors);
        }
        record.components.push(l.record.clone());
    }
    record.units.fix = record
        .units
        .fix
        .checked_add(split_charge)
        .ok_or(Halt::Refused(UNITS_WIDTH))?;
    record.size_histogram = histogram.into_iter().collect();
    record.group_order = merge_factors(groups);
    record.broken_by = failures.iter().map(|f| f.record).collect();
    record.guaranteed = record.broken_by.is_empty();
    // The canonical line sequence and its digest.
    let mut lines: Vec<Box<[u8]>> = (0..set.lines.len())
        .map(|l| {
            let mut enc = Vec::new();
            set.encode(l, |b| Leaf::Fixed(placements[b].position), &mut enc);
            enc.into_boxed_slice()
        })
        .collect();
    lines.sort();
    let mut message = Vec::new();
    message.extend_from_slice(&(lines.len() as u64).to_be_bytes());
    for line in &lines {
        message.extend_from_slice(line);
    }
    let digest = digest_of(LINES_CONTEXT, &message);
    let labelled = Labelled {
        placements,
        lines,
        digest,
        components,
        record,
        diagnostic: failures,
        explored,
        twin_units,
        comparison_units,
        stopped_on_s: stopped.0,
        stopped_on_e: stopped.1,
    };
    Ok((labelled, traces))
}

/// The first failures read from the result path of a full search: branching
/// calls whose target cell is not one twin class, with no such ancestor. An
/// independent computation of `broken_by`, by pairwise transposition tests.
pub(crate) fn full_search_failures(
    set: &LineSet,
    comps: &[Comp],
    labellings: &[Labelling],
) -> Vec<FailureDiagnostic> {
    let mut out = Vec::new();
    for (comp, l) in comps.iter().zip(labellings) {
        let lines: FastMap<Box<[Tok]>, ()> = comp
            .lines
            .iter()
            .map(|&x| (set.lines[x].clone(), ()))
            .collect();
        let twins = |a: usize, b: usize| {
            comp.lines.iter().all(|&x| {
                let image: Box<[Tok]> = set.lines[x]
                    .iter()
                    .map(|t| match *t {
                        Tok::Blank(y) if y == a => Tok::Blank(b),
                        Tok::Blank(y) if y == b => Tok::Blank(a),
                        other => other,
                    })
                    .collect();
                lines.contains_key(&image)
            })
        };
        let source = |v: usize| -> Box<[u8]> { set.source(comp.nodes[v]).into() };
        let smallest = source(l.smallest_node);
        let mut stack = vec![&l.tree];
        while let Some(tree) = stack.pop() {
            if tree.rule == 3 {
                let t = &tree.target;
                let one_class = t.iter().all(|&a| {
                    t.iter()
                        .all(|&b| a == b || twins(comp.nodes[a], comp.nodes[b]))
                });
                if !one_class {
                    out.push(FailureDiagnostic {
                        record: FailureRecord {
                            path_position: tree.p,
                            tied_cell: t.len() as u64,
                            component_nodes: comp.nodes.len() as u64,
                            component_lines: comp.lines.len() as u64,
                        },
                        component_source: smallest.clone(),
                        cell_source: t.iter().map(|&v| source(v)).min().unwrap_or_default(),
                    });
                    continue;
                }
            }
            stack.extend(tree.nested.iter());
        }
    }
    out.sort();
    out
}
