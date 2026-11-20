// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The optimised engine: the result of [`super::reference`], computed on dense
//! arrays with an explicit stack of calls.
//!
//! A cell is a range of an array of its side, and its colour is the index at
//! which the range starts, so splitting a cell rewrites only the range of the
//! cell. An incidence carries its label as a slice of occurrence numbers, kept
//! as the input numbered them (8.2, item 4). Every call has its own compact
//! instance, rebuilt for each nested component, so the work of a call is
//! proportional to its own size. Calls are frames of one heap stack: nothing
//! recurses (6.6).
//!
//! The search is the walk of 8.3 and nothing besides it: a branching call
//! visits the members of its target cell in source order, skips a member the
//! relation `~` of [`super::walk`] relates to an earlier one, stops a child
//! whose `s` or entries exceed the best's, and completes every other child.
//! Every charge of the walk is made where the walk makes it, so the engine
//! refuses with `limit` exactly where the walk passes `max(L, A)`, with the
//! walk's fields. The chosen child is the first completed child that attains
//! the best certificate, and the attaining count is the number of members
//! whose class of `~` holds a completed child equal to the best.

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BinaryHeap};

use super::certificate::{Certificate, Entry, entry_of};
use super::lines::{Leaf, LineSet};
use super::reference::{
    Child, Classes, Comp, ComponentRun, Labelling, LabellingParts, Prepared, Static, Stop, Tree,
    TwinTest, component_count, drive, failure_diagnostics, finish_labelling, fragments_to_queue,
    prepare,
};
use super::walk::{
    Kind, Map, Relation, Walk, Witnesses, carries, nodes_below, tied_exchanges, twin_tests,
};
use super::{Component, ComponentRecord, FailureDiagnostic, Halt, Labelled, Refusal, Units};
use crate::StopSignal;
use crate::governor::is_stopped;

/// The optimised engine's own limit `L` on the running total of the walk.
pub(super) const ENGINE_LIMIT: u64 = 1 << 34;

/// One incidence: the other end, and the label of the incidence as the
/// occurrence numbers `occ_nums[off..off + len]` of the node in the line.
#[derive(Clone, Copy, Debug)]
struct Inc {
    other: usize,
    off: usize,
    len: usize,
}

/// The incidences of a call: `[0]` of each node (its lines), `[1]` of each
/// line (its free nodes).
type Adj = [Vec<Vec<Inc>>; 2];

/// The incidences of a top component, built once.
struct Index {
    occ_nums: Vec<usize>,
    adj: Adj,
}

impl Index {
    fn new(comp: &Comp) -> Self {
        let mut occ_nums = Vec::new();
        let mut node_inc: Vec<Vec<Inc>> = vec![Vec::new(); comp.nodes.len()];
        let mut line_inc: Vec<Vec<Inc>> = Vec::with_capacity(comp.lines.len());
        for (l, occ) in comp.occ.iter().enumerate() {
            let mut pairs: Vec<(usize, usize)> =
                occ.iter().enumerate().map(|(i, &node)| (node, i)).collect();
            pairs.sort_unstable();
            let mut row = Vec::new();
            let mut at = 0;
            while at < pairs.len() {
                let node = pairs[at].0;
                let off = occ_nums.len();
                while at < pairs.len() && pairs[at].0 == node {
                    occ_nums.push(pairs[at].1);
                    at += 1;
                }
                let len = occ_nums.len() - off;
                row.push(Inc {
                    other: node,
                    off,
                    len,
                });
                node_inc[node].push(Inc { other: l, off, len });
            }
            line_inc.push(row);
        }
        Self {
            occ_nums,
            adj: [node_inc, line_inc],
        }
    }
}

/// One side of a pair of ordered partitions, on arrays: the vertices in cell
/// order, the index of each vertex, the colour (start index) of the cell of
/// each vertex, and the size of the cell that starts at an index.
#[derive(Clone, Debug)]
struct Side {
    elems: Vec<usize>,
    pos: Vec<usize>,
    start: Vec<usize>,
    size: Vec<usize>,
}

impl Side {
    /// The cells in the order given.
    fn from_cells(cells: impl IntoIterator<Item = Vec<usize>>, vertices: usize) -> Self {
        let mut side = Self {
            elems: Vec::with_capacity(vertices),
            pos: vec![0; vertices],
            start: vec![0; vertices],
            size: vec![0; vertices],
        };
        for cell in cells {
            let c = side.elems.len();
            side.size[c] = cell.len();
            for v in cell {
                side.pos[v] = side.elems.len();
                side.start[v] = c;
                side.elems.push(v);
            }
        }
        side
    }

    /// The colours of the cells, ascending.
    fn starts(&self) -> impl Iterator<Item = usize> + '_ {
        let mut at = 0;
        std::iter::from_fn(move || {
            if at >= self.elems.len() {
                return None;
            }
            let c = at;
            at += self.size[c];
            Some(c)
        })
    }

    fn cell_size_of(&self, v: usize) -> usize {
        self.size[self.start[v]]
    }
}

/// The queue of `(side, colour)` pairs: the smallest first, each at most once.
struct Queue {
    heap: BinaryHeap<Reverse<(usize, usize)>>,
    queued: [Vec<bool>; 2],
}

impl Queue {
    fn new(vertices: [usize; 2]) -> Self {
        Self {
            heap: BinaryHeap::new(),
            queued: [vec![false; vertices[0]], vec![false; vertices[1]]],
        }
    }

    fn push(&mut self, side: usize, colour: usize) {
        if !self.queued[side][colour] {
            self.queued[side][colour] = true;
            self.heap.push(Reverse((side, colour)));
        }
    }

    fn pop(&mut self) -> Option<(usize, usize)> {
        let Reverse((side, colour)) = self.heap.pop()?;
        self.queued[side][colour] = false;
        Some((side, colour))
    }

    /// The queue rule after a split of the cell of `colour` into `fragments`.
    fn after_split(&mut self, side: usize, colour: usize, fragments: &[(usize, usize)]) {
        for fragment in fragments_to_queue(self.queued[side][colour], fragments) {
            self.push(side, fragment);
        }
    }
}

/// Scratch space of the refinement, reused across pops.
#[derive(Default)]
struct Scratch {
    /// `(colour of y, y, label offset, label length)` of each incidence read.
    touches: Vec<(usize, usize, usize, usize)>,
}

/// A call's instance: its nodes and lines in compact local indices, their
/// incidences, and the pair of partitions.
#[derive(Clone)]
struct Inst {
    /// The top-component index of each local node.
    node_g: Vec<usize>,
    /// The top-component index of each local line.
    line_g: Vec<usize>,
    adj: Adj,
    pair: [Side; 2],
}

/// Refine `pair` until `queue` is empty; returns the label elements read.
fn refine(
    adj: &Adj,
    occ_nums: &[usize],
    pair: &mut [Side; 2],
    queue: &mut Queue,
    scratch: &mut Scratch,
) -> u64 {
    let mut units = 0u64;
    // The new order of the members of the cell being split, from its first
    // touched member on. It is as large as the cell, so it lives across the
    // splits of one refinement: peeling a few members off a large cell again
    // and again then requests no storage per split.
    let mut order: Vec<usize> = Vec::new();
    while let Some((side, c)) = queue.pop() {
        let other = 1 - side;
        scratch.touches.clear();
        for i in c..c + pair[side].size[c] {
            let x = pair[side].elems[i];
            for inc in &adj[side][x] {
                units += inc.len as u64;
                let cy = pair[other].start[inc.other];
                if pair[other].size[cy] >= 2 {
                    scratch.touches.push((cy, inc.other, inc.off, inc.len));
                }
            }
        }
        scratch.touches.sort_unstable_by(|a, b| {
            (a.0, a.1)
                .cmp(&(b.0, b.1))
                .then_with(|| occ_nums[a.2..a.2 + a.3].cmp(&occ_nums[b.2..b.2 + b.3]))
        });
        let mut at = 0;
        while at < scratch.touches.len() {
            let cy = scratch.touches[at].0;
            let mut end = at;
            while end < scratch.touches.len() && scratch.touches[end].0 == cy {
                end += 1;
            }
            split_cell(
                &mut pair[other],
                cy,
                &scratch.touches[at..end],
                occ_nums,
                queue,
                other,
                &mut order,
            );
            at = end;
        }
    }
    units
}

/// Split the cell of colour `cy` of `side` by the keys of its touched members
/// `run` (sorted by member, then label): the members with no incidence into the
/// popped cell come first, then the groups of equal key, ascending.
fn split_cell(
    side: &mut Side,
    cy: usize,
    run: &[(usize, usize, usize, usize)],
    occ_nums: &[usize],
    queue: &mut Queue,
    side_index: usize,
    order: &mut Vec<usize>,
) {
    // The touched members, ascending, each with the range of its labels.
    let mut touched: Vec<(usize, usize, usize)> = Vec::new();
    let mut at = 0;
    while at < run.len() {
        let y = run[at].1;
        let from = at;
        while at < run.len() && run[at].1 == y {
            at += 1;
        }
        touched.push((y, from, at));
    }
    let mut at_positions: Vec<usize> = touched.iter().map(|t| t.0).collect();
    let key = |t: &(usize, usize, usize)| run[t.1..t.2].iter().map(|r| &occ_nums[r.2..r.2 + r.3]);
    touched.sort_by(|a, b| key(a).cmp(key(b)));
    let size = side.size[cy];
    let untouched = size - at_positions.len();
    let mut groups = 1;
    for pair in touched.windows(2) {
        if key(&pair[0]).cmp(key(&pair[1])) != Ordering::Equal {
            groups += 1;
        }
    }
    if untouched == 0 && groups == 1 {
        return;
    }
    // The members before the first touched one keep their place, so the new
    // order is built from there: the untouched members after it in their
    // order, then the touched groups.
    for y in &mut at_positions {
        *y = side.pos[*y];
    }
    at_positions.sort_unstable();
    // INVARIANT: the one caller passes a run of at least one touched member.
    let first = at_positions[0];
    order.clear();
    let mut from = first;
    for &q in &at_positions {
        order.extend_from_slice(&side.elems[from..q]);
        from = q + 1;
    }
    order.extend_from_slice(&side.elems[from..cy + size]);
    let mut fragments: Vec<(usize, usize)> = Vec::with_capacity(groups + 1);
    if untouched > 0 {
        fragments.push((cy, untouched));
    }
    let mut at = 0;
    while at < touched.len() {
        let mut end = at + 1;
        while end < touched.len() && key(&touched[at]).cmp(key(&touched[end])) == Ordering::Equal {
            end += 1;
        }
        fragments.push((first + order.len(), end - at));
        order.extend(touched[at..end].iter().map(|t| t.0));
        at = end;
    }
    for (i, &v) in order.iter().enumerate() {
        side.elems[first + i] = v;
        side.pos[v] = first + i;
    }
    for &(colour, count) in &fragments {
        side.size[colour] = count;
        // The untouched members already carry the colour of the cell.
        if untouched > 0 && colour == cy {
            continue;
        }
        for i in colour..colour + count {
            let v = side.elems[i];
            side.start[v] = colour;
        }
    }
    queue.after_split(side_index, cy, &fragments);
}

/// Individualise node `v`: it leaves its cell for a cell of its own in front
/// of the rest, and the queue rule applies to the two fragments.
fn individualise(side: &mut Side, v: usize, queue: &mut Queue) {
    let c = side.start[v];
    let k = side.size[c];
    let at = side.pos[v];
    side.elems.swap(at, c);
    let moved = side.elems[at];
    side.pos[moved] = at;
    side.pos[v] = c;
    side.size[c] = 1;
    for i in c + 1..c + k {
        let x = side.elems[i];
        side.start[x] = c + 1;
    }
    side.size[c + 1] = k - 1;
    queue.after_split(0, c, &[(c, 1), (c + 1, k - 1)]);
}

/// What a nest is the body of.
enum NestKind {
    /// A call that took rule 1 or 2.
    Call { n_k: u64 },
    /// The child of the target member of index `target_index`, whose
    /// individualisation read `refine` label elements.
    Child { target_index: usize, refine: u64 },
}

/// A fixing in progress: the nested calls still to run and those returned.
struct Nest<'a> {
    kind: NestKind,
    p: u64,
    fixed: Vec<(usize, u64)>,
    entries: Vec<Entry>,
    units: Units,
    /// Whether no node remains free (rule 1).
    discrete: bool,
    /// Nested instances still to run, the smallest source last.
    pending: Vec<(&'a [u8], Inst)>,
    /// The source of the nested call now running.
    current: &'a [u8],
    sourced: Vec<(&'a [u8], Tree)>,
    /// The maps the nested calls own (step 5, (K3)).
    carried: Vec<Map>,
    failed: bool,
}

/// A rule 3 call in progress: the walk of step 3(b) over its target cell, or
/// the one child of the search-free run.
struct Branch {
    adj: Adj,
    pair: [Side; 2],
    node_g: Vec<usize>,
    line_g: Vec<usize>,
    p: u64,
    n_k: u64,
    /// The target cell, local nodes in ascending source order.
    target: Vec<usize>,
    /// The same members as nodes of the top component.
    members: Vec<usize>,
    /// Target index by local node, `usize::MAX` off the target cell.
    target_index: Vec<usize>,
    /// The members visited: every member in a walk, the first in the
    /// search-free run.
    visits: usize,
    next: usize,
    /// The best completed child: the first of smallest certificate.
    best: Option<Child>,
    /// Target indices of the completed children equal to the best.
    attain: Vec<usize>,
    relation: Relation,
    witnesses: Witnesses,
    /// The known maps that passed their check here (`OWN`).
    own: Vec<Map>,
    /// Whether the twin tests of the call (step 4) are made.
    tested: bool,
    stopped_s: u64,
    stopped_e: u64,
}

enum Frame<'a> {
    Branch(Box<Branch>),
    Nest(Box<Nest<'a>>),
}

/// What a frame delivers to the frame below it.
enum Event {
    /// A call's tree and the maps it owns.
    Tree(Tree, Vec<Map>),
    /// A completed child, the maps its nested calls carry, its target index.
    Child(Child, Vec<Map>, usize),
    Failed,
}

/// The stop of 10.4 that fired.
#[derive(Clone, Copy)]
enum Cut {
    OnS,
    OnE,
}

/// The labelling of one top component.
struct Engine<'a> {
    set: &'a LineSet,
    comp: &'a Comp,
    index: &'a Index,
    /// The twin test and the twin classes of the search-free run.
    twin: TwinTest,
    /// Whether the calls run are the walk's, or the search-free run's.
    walking: bool,
    /// The walk while this component's step 2 runs. Before it, the walk of
    /// the search-free run, which has no bound: when that run labels the
    /// component its charges, kind by kind, are those the walk makes there
    /// (one child at each branching call with the same twin tests, Lemma W
    /// (4)(d)).
    walk: Walk,
    /// `TW`: the twin classes of the tests of the walk.
    tw: Classes,
    /// Engine figure: units over the calls explored.
    explored: Units,
    /// Engine figure: twin units of the search-free run (8.1(f)).
    twin_units: u64,
    /// First failures: `(p, size of the target cell, local node)`.
    failures: Vec<(u64, u64, usize)>,
    stop: Option<&'a dyn StopSignal>,
    /// The path position of each fixed node of the top component.
    fixed_pos: Vec<Option<u64>>,
    /// Local node of the call under a map check, by node of the top
    /// component; `usize::MAX` between checks.
    local: Vec<usize>,
    /// The identity on the nodes of the top component, scratch of the check.
    image: Vec<usize>,
    scratch: Scratch,
}

impl<'a> Engine<'a> {
    /// An engine over `comp` with nothing run yet; `start` is the start charge
    /// of each line of the component.
    fn new(
        set: &'a LineSet,
        comp: &'a Comp,
        index: &'a Index,
        stop: Option<&'a dyn StopSignal>,
        start: Vec<u64>,
    ) -> Self {
        let n = comp.nodes.len();
        Self {
            set,
            comp,
            index,
            twin: TwinTest::new(set, comp, start),
            walking: false,
            walk: Walk::default(),
            tw: Classes::new(n),
            explored: Units::default(),
            twin_units: 0,
            failures: Vec::new(),
            stop,
            fixed_pos: vec![None; n],
            local: vec![usize::MAX; n],
            image: (0..n).collect(),
            scratch: Scratch::default(),
        }
    }

    fn source_of(&self, v: usize) -> &'a [u8] {
        self.set.source(self.comp.nodes[v])
    }

    /// A charge of the walk; the walk of the search-free run has no bound.
    fn charge(&mut self, kind: Kind, units: u64) -> Result<(), Stop> {
        self.walk.charge(kind, units)
    }

    /// The one charge of a fixing that is not stopped: its FIX-SUM and SPLIT.
    fn charge_fixing(&mut self, fix_sum: u64, split: u64) -> Result<(), Stop> {
        self.walk.charge_fixing(fix_sum, split)
    }

    /// The fresh start of the top component, refined.
    fn fresh(&mut self) -> (Inst, u64) {
        let comp = self.comp;
        let (n, m) = (comp.nodes.len(), comp.lines.len());
        let mut by_encoding: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
        for (j, &l) in comp.lines.iter().enumerate() {
            let mut enc = Vec::new();
            self.set.encode(l, |_| Leaf::Free, &mut enc);
            by_encoding.entry(enc).or_default().push(j);
        }
        let mut inst = Inst {
            node_g: (0..n).collect(),
            line_g: (0..m).collect(),
            adj: self.index.adj.clone(),
            pair: [
                Side::from_cells([(0..n).collect()], n),
                Side::from_cells(by_encoding.into_values(), m),
            ],
        };
        let mut queue = Queue::new([n, m]);
        for (side, part) in inst.pair.iter().enumerate() {
            for c in part.starts() {
                queue.push(side, c);
            }
        }
        let units = refine(
            &inst.adj,
            &self.index.occ_nums,
            &mut inst.pair,
            &mut queue,
            &mut self.scratch,
        );
        (inst, units)
    }

    /// Begin the call on `inst` with argument `p`: a frame to run.
    fn begin(&mut self, inst: Inst, p: u64) -> Result<Frame<'a>, Stop> {
        if is_stopped(self.stop) {
            return Err(Stop::Stopped);
        }
        let n_k = inst.node_g.len() as u64;
        if inst.pair[0].starts().any(|c| inst.pair[0].size[c] == 1) {
            let Inst {
                node_g,
                line_g,
                adj,
                pair,
            } = inst;
            let call = CallRef {
                adj: &adj,
                pair: &pair,
                node_g: &node_g,
                line_g: &line_g,
                p,
            };
            return match self.nest(&call, NestKind::Call { n_k }, None)? {
                Nested::Frame(nest) => Ok(Frame::Nest(nest)),
                // INVARIANT: a nest with no bound is never cut.
                Nested::Cut(_) => Err(Stop::Failed),
            };
        }
        // Rule 3: the target cell is the node cell of colour 0.
        let mut target: Vec<usize> = inst.pair[0].elems[..inst.pair[0].size[0]].to_vec();
        target.sort_by(|&a, &b| {
            self.source_of(inst.node_g[a])
                .cmp(self.source_of(inst.node_g[b]))
        });
        let k = target.len();
        let members: Vec<usize> = target.iter().map(|&v| inst.node_g[v]).collect();
        if !self.walking {
            // The search-free run: the target cell is one twin class, or the
            // component fails here (7.10).
            let u = members[0];
            for &w in &members[1..] {
                let (ru, rw) = (self.twin.find(u), self.twin.find(w));
                if ru == rw {
                    continue;
                }
                let (passed, units) = self.twin.test(self.set, self.comp, u, w);
                self.twin_units = self.twin_units.saturating_add(units);
                self.charge(Kind::Twin, units)?;
                if passed {
                    self.twin.join(ru, rw);
                } else {
                    self.failures.push((p, k as u64, u));
                    return Err(Stop::Failed);
                }
            }
        }
        let mut target_index = vec![usize::MAX; inst.node_g.len()];
        for (i, &v) in target.iter().enumerate() {
            target_index[v] = i;
        }
        let Inst {
            node_g,
            line_g,
            adj,
            pair,
        } = inst;
        let witnesses = Witnesses::new(node_g.len());
        Ok(Frame::Branch(Box::new(Branch {
            adj,
            pair,
            node_g,
            line_g,
            p,
            n_k,
            target,
            members,
            target_index,
            visits: if self.walking { k } else { 1 },
            next: 0,
            best: None,
            attain: Vec::new(),
            relation: Relation::new(k),
            witnesses,
            own: Vec::new(),
            tested: false,
            stopped_s: 0,
            stopped_e: 0,
        })))
    }

    /// FIX the singleton node cells of the call's pair, form the entries,
    /// split the remainder into its free components and build their instances,
    /// with the charge of the fixing (step 3(a) and (b)(vi)). With a `bound`,
    /// the stops of step 3(b)(iv) and (v) apply once `s`, and then `E`, are
    /// known, and a stopped child is charged its FIX-SUM alone.
    fn nest(
        &mut self,
        call: &CallRef<'_>,
        kind: NestKind,
        bound: Option<&Certificate>,
    ) -> Result<Nested<'a>, Stop> {
        let nodes = &call.pair[0];
        let mut fixed: Vec<(usize, u64)> = Vec::new();
        let mut fixed_local = Vec::new();
        for c in nodes.starts() {
            if nodes.size[c] == 1 {
                let v = nodes.elems[c];
                let q = call.p + fixed.len() as u64;
                fixed.push((call.node_g[v], q));
                fixed_local.push(v);
                self.fixed_pos[call.node_g[v]] = Some(q);
            }
        }
        let s = fixed.len() as u64;
        let mut units = Units::default();
        let mut entries = Vec::new();
        for &lg in call.line_g {
            // A line is complete when every occurrence has a path position.
            let positions: Option<Vec<u64>> = self.comp.occ[lg]
                .iter()
                .map(|&x| self.fixed_pos[x])
                .collect();
            if let Some(positions) = positions {
                let entry = entry_of(self.set, self.comp.lines[lg], &positions);
                units.entries += 1 + (entry.encoding().len() as u64).div_ceil(8);
                entries.push(entry);
            }
        }
        entries.sort();
        let removed_incidences: u64 = fixed_local
            .iter()
            .map(|&v| call.adj[0][v].len() as u64)
            .sum();
        units.fix += s + entries.len() as u64 + removed_incidences;
        // FIX-SUM: the fixing without the split of its remainder, with its
        // entries.
        let fix_sum = units.fix.saturating_add(units.entries);
        // A child is stopped when its `s` is greater than the best's, or when
        // its `s` is equal and its sorted entries come after the best's in list
        // order (10.4).
        let cut = bound.and_then(|best| match s.cmp(&best.s) {
            Ordering::Greater => Some(Cut::OnS),
            Ordering::Equal if entries > best.entries => Some(Cut::OnE),
            _ => None,
        });
        if let Some(cut) = cut {
            self.undo(&fixed);
            self.explored = self.explored.saturating_add(units);
            self.charge(Kind::FixSum, fix_sum)?;
            return Ok(Nested::Cut(cut));
        }
        let free_nodes = call.pair[0]
            .starts()
            .filter(|&c| call.pair[0].size[c] >= 2)
            .count();
        let (components, split) = self.split(call);
        units.fix += split;
        self.explored = self.explored.saturating_add(units);
        self.charge_fixing(fix_sum, split)?;
        Ok(Nested::Frame(Box::new(Nest {
            kind,
            p: call.p,
            fixed,
            entries,
            units,
            discrete: free_nodes == 0,
            pending: components,
            current: &[],
            sourced: Vec::new(),
            carried: Vec::new(),
            failed: false,
        })))
    }

    fn undo(&mut self, fixed: &[(usize, u64)]) {
        for &(v, _) in fixed {
            self.fixed_pos[v] = None;
        }
    }

    /// The free components of the call after its fixing, as instances, the
    /// smallest source last, with the charge for the split (8.1(c)).
    fn split(&self, call: &CallRef<'_>) -> (Vec<(&'a [u8], Inst)>, u64) {
        let (nodes, lines) = (&call.pair[0], &call.pair[1]);
        let free = |v: usize| nodes.cell_size_of(v) >= 2;
        let mut comp_of_node = vec![usize::MAX; call.node_g.len()];
        let mut comp_of_line = vec![usize::MAX; call.line_g.len()];
        let mut count = 0;
        let mut stack = Vec::new();
        for c in nodes.starts() {
            if nodes.size[c] < 2 {
                continue;
            }
            for &root in &nodes.elems[c..c + nodes.size[c]] {
                if comp_of_node[root] != usize::MAX {
                    continue;
                }
                comp_of_node[root] = count;
                stack.push(root);
                while let Some(x) = stack.pop() {
                    for inc in &call.adj[0][x] {
                        let l = inc.other;
                        if comp_of_line[l] != usize::MAX {
                            continue;
                        }
                        comp_of_line[l] = count;
                        for back in &call.adj[1][l] {
                            let y = back.other;
                            if free(y) && comp_of_node[y] == usize::MAX {
                                comp_of_node[y] = count;
                                stack.push(y);
                            }
                        }
                    }
                }
                count += 1;
            }
        }
        if count == 0 {
            return (Vec::new(), 0);
        }
        let mut node_cells: Vec<Vec<(usize, usize)>> = vec![Vec::new(); count];
        let mut line_cells: Vec<Vec<(usize, usize)>> = vec![Vec::new(); count];
        let mut node_g: Vec<Vec<usize>> = vec![Vec::new(); count];
        let mut line_g: Vec<Vec<usize>> = vec![Vec::new(); count];
        let mut new_node = vec![usize::MAX; call.node_g.len()];
        let mut new_line = vec![usize::MAX; call.line_g.len()];
        let mut last: Vec<usize> = vec![usize::MAX; count];
        for c in nodes.starts() {
            for &v in &nodes.elems[c..c + nodes.size[c]] {
                let ci = comp_of_node[v];
                if ci == usize::MAX {
                    continue;
                }
                let id = node_g[ci].len();
                if last[ci] != c {
                    last[ci] = c;
                    node_cells[ci].push((id, 0));
                }
                if let Some(cell) = node_cells[ci].last_mut() {
                    cell.1 += 1;
                }
                node_g[ci].push(call.node_g[v]);
                new_node[v] = id;
            }
        }
        last.fill(usize::MAX);
        for c in lines.starts() {
            for &l in &lines.elems[c..c + lines.size[c]] {
                let ci = comp_of_line[l];
                if ci == usize::MAX {
                    continue;
                }
                let id = line_g[ci].len();
                if last[ci] != c {
                    last[ci] = c;
                    line_cells[ci].push((id, 0));
                }
                if let Some(cell) = line_cells[ci].last_mut() {
                    cell.1 += 1;
                }
                line_g[ci].push(call.line_g[l]);
                new_line[l] = id;
            }
        }
        let mut node_adj: Vec<Vec<Vec<Inc>>> = vec![Vec::new(); count];
        let mut line_adj: Vec<Vec<Vec<Inc>>> = vec![Vec::new(); count];
        for c in nodes.starts() {
            for &v in &nodes.elems[c..c + nodes.size[c]] {
                let ci = comp_of_node[v];
                if ci == usize::MAX {
                    continue;
                }
                node_adj[ci].push(
                    call.adj[0][v]
                        .iter()
                        .map(|inc| Inc {
                            other: new_line[inc.other],
                            ..*inc
                        })
                        .collect(),
                );
            }
        }
        for c in lines.starts() {
            for &l in &lines.elems[c..c + lines.size[c]] {
                let ci = comp_of_line[l];
                if ci == usize::MAX {
                    continue;
                }
                line_adj[ci].push(
                    call.adj[1][l]
                        .iter()
                        .filter(|inc| comp_of_node[inc.other] != usize::MAX)
                        .map(|inc| Inc {
                            other: new_node[inc.other],
                            ..*inc
                        })
                        .collect(),
                );
            }
        }
        let sizes: Vec<u64> = (0..count)
            .map(|ci| (node_g[ci].len() + line_g[ci].len()) as u64)
            .collect();
        let split = if count >= 2 {
            sizes.iter().sum::<u64>() - sizes.iter().copied().max().unwrap_or(0)
        } else {
            0
        };
        let mut out: Vec<(&'a [u8], Inst)> = Vec::with_capacity(count);
        for ci in 0..count {
            let n = node_g[ci].len();
            let m = line_g[ci].len();
            let set = self.set;
            let source = node_g[ci]
                .iter()
                .map(|&g| set.source(self.comp.nodes[g]))
                .min()
                .unwrap_or_default();
            let pair = [
                identity_side(&node_cells[ci], n),
                identity_side(&line_cells[ci], m),
            ];
            let inst = Inst {
                node_g: std::mem::take(&mut node_g[ci]),
                line_g: std::mem::take(&mut line_g[ci]),
                adj: [
                    std::mem::take(&mut node_adj[ci]),
                    std::mem::take(&mut line_adj[ci]),
                ],
                pair,
            };
            out.push((source, inst));
        }
        // The smallest source last, so that popping runs them in ascending
        // order of source.
        out.sort_by(|a, b| b.0.cmp(a.0));
        (out, split)
    }

    /// Relate the members of the target cell that a known map moves onto one
    /// another, when the map passes its check against the call (step 5).
    /// Whether it passed; a map that fails is dropped whole.
    fn relate_by(&mut self, b: &mut Branch, map: &[(usize, usize)]) -> bool {
        for (x, &g) in b.node_g.iter().enumerate() {
            self.local[g] = x;
        }
        let (local, target_index) = (&self.local, &b.target_index);
        let in_target = |g: usize| local[g] != usize::MAX && target_index[local[g]] != usize::MAX;
        let checked = carries(
            self.set,
            self.comp,
            &self.twin,
            &b.line_g,
            |g| local[g] != usize::MAX,
            in_target,
            map,
            &mut self.image,
        );
        if checked {
            for &(x, y) in map {
                let (i, j) = (target_index[local[x]], target_index[local[y]]);
                if i != usize::MAX && j != usize::MAX {
                    b.relation.relate(i, j);
                }
            }
        }
        for &g in &b.node_g {
            self.local[g] = usize::MAX;
        }
        checked
    }
}

impl<'a> Engine<'a> {
    /// Run the call on `top` with argument `p` to its tree, on the heap stack.
    fn run(&mut self, top: Inst, p: u64) -> Result<Tree, Stop> {
        let mut stack: Vec<Frame<'a>> = vec![self.begin(top, p)?];
        let mut event: Option<Event> = None;
        loop {
            if let Some(delivered) = event.take() {
                let Some(frame) = stack.last_mut() else {
                    return match delivered {
                        Event::Tree(tree, _) => Ok(tree),
                        _ => Err(Stop::Failed),
                    };
                };
                match (frame, delivered) {
                    (Frame::Nest(nest), Event::Tree(tree, own)) => {
                        nest.sourced.push((nest.current, tree));
                        nest.carried.extend(own);
                    }
                    (Frame::Nest(nest), Event::Failed) => nest.failed = true,
                    (Frame::Branch(branch), Event::Child(child, carried, target_index)) => {
                        self.child_done(branch, child, carried, target_index);
                    }
                    (Frame::Branch(_), Event::Failed) => {
                        stack.pop();
                        event = Some(Event::Failed);
                        continue;
                    }
                    // INVARIANT: a nest receives trees and failures only, a
                    // branch children and failures only.
                    _ => return Err(Stop::Failed),
                }
            }
            let top = stack.len() - 1;
            let act = match &mut stack[top] {
                Frame::Nest(nest) => match nest.pending.pop() {
                    Some((source, inst)) => {
                        nest.current = source;
                        let at = nest.p + nest.fixed.len() as u64;
                        match self.begin(inst, at) {
                            Ok(frame) => Act::Push(frame),
                            Err(Stop::Failed) => {
                                nest.failed = true;
                                Act::Again
                            }
                            Err(other) => return Err(other),
                        }
                    }
                    None => Act::Finish,
                },
                Frame::Branch(branch) => self.step_branch(branch)?,
            };
            match act {
                Act::Push(frame) => stack.push(frame),
                Act::Again => {}
                Act::Finish => match stack.pop() {
                    Some(Frame::Nest(nest)) => event = Some(self.finish_nest(*nest)),
                    Some(Frame::Branch(branch)) => event = Some(self.finish_branch(*branch)?),
                    None => return Err(Stop::Failed),
                },
            }
        }
    }

    /// The next child of a branching call (step 3(b)): its frame, or the end
    /// of the call. A walk skips a member related to an earlier one, charges
    /// the REFINE of every child it visits and the fixing of each, and makes
    /// the twin tests of the call after the fixing of its first completed
    /// child and before that child's nested calls.
    fn step_branch(&mut self, b: &mut Branch) -> Result<Act<'a>, Stop> {
        while b.next < b.visits {
            let ti = b.next;
            b.next += 1;
            if self.walking {
                // (K1) reads `TW` as it stands at the visit, after step 4.
                if b.tested {
                    b.relation.twins(&b.members, &mut self.tw);
                }
                if b.relation.skipped(ti) {
                    continue;
                }
            }
            let v = b.target[ti];
            let mut pair = b.pair.clone();
            let mut queue = Queue::new([b.node_g.len(), b.line_g.len()]);
            individualise(&mut pair[0], v, &mut queue);
            let refine_units = refine(
                &b.adj,
                &self.index.occ_nums,
                &mut pair,
                &mut queue,
                &mut self.scratch,
            );
            self.explored.refine = self.explored.refine.saturating_add(refine_units);
            self.charge(Kind::Refine, refine_units)?;
            let call = CallRef {
                adj: &b.adj,
                pair: &pair,
                node_g: &b.node_g,
                line_g: &b.line_g,
                p: b.p,
            };
            let bound = if self.walking {
                b.best.as_ref().map(|child| &child.cert)
            } else {
                None
            };
            let kind = NestKind::Child {
                target_index: ti,
                refine: refine_units,
            };
            match self.nest(&call, kind, bound)? {
                Nested::Frame(nest) => {
                    if self.walking && !b.tested {
                        let units =
                            twin_tests(&self.twin, self.set, self.comp, &mut self.tw, &b.members);
                        self.charge(Kind::Twin, units)?;
                        b.tested = true;
                    }
                    return Ok(Act::Push(Frame::Nest(nest)));
                }
                Nested::Cut(Cut::OnS) => b.stopped_s += 1,
                Nested::Cut(Cut::OnE) => b.stopped_e += 1,
            }
        }
        Ok(Act::Finish)
    }

    /// A child completed (step 3(b)(viii) and (ix)): in a walk, its known map
    /// from an earlier witness of its certificate (K2) and the maps it carries
    /// (K3), each used only when it passes its check; then the best so far and
    /// the attaining children. The best is replaced only by a strictly smaller
    /// certificate, so it ends as the attaining child of smallest source.
    fn child_done(&mut self, b: &mut Branch, child: Child, carried: Vec<Map>, ti: usize) {
        if self.walking {
            let by_position: Vec<usize> = nodes_below(&child.fixed, b.p, &child.nested)
                .into_iter()
                .map(|(_, node)| node)
                .collect();
            let known = b.witnesses.complete(&child.cert, by_position);
            for map in known.into_iter().chain(carried) {
                if self.relate_by(b, &map) {
                    b.own.push(map);
                }
            }
        }
        let order = b
            .best
            .as_ref()
            .map_or(Ordering::Less, |best| child.cert.cmp(&best.cert));
        match order {
            Ordering::Less => {
                b.attain.clear();
                b.attain.push(ti);
                b.best = Some(child);
            }
            Ordering::Equal => b.attain.push(ti),
            Ordering::Greater => {}
        }
    }

    /// The nest is complete: undo its fixing and deliver what it made, with
    /// the maps it carries in a walk: those its nested calls own and the
    /// exchanges of its tied nested calls.
    fn finish_nest(&mut self, nest: Nest<'a>) -> Event {
        self.undo(&nest.fixed);
        if nest.failed {
            return Event::Failed;
        }
        // Ascending by certificate, equal certificates by smallest source
        // (6.4): a stable sort of the source order.
        let mut sourced = nest.sourced;
        sourced.sort_by(|a, b| a.0.cmp(b.0));
        sourced.sort_by(|a, b| a.1.cert.cmp(&b.1.cert));
        let nested: Vec<Tree> = sourced.into_iter().map(|(_, tree)| tree).collect();
        let mut carried = nest.carried;
        if self.walking {
            carried.extend(tied_exchanges(&nested));
        }
        let cert = Certificate {
            s: nest.fixed.len() as u64,
            entries: nest.entries,
            nested: nested.iter().map(|t| t.cert.clone()).collect(),
        };
        let mut child = Child {
            fixed: nest.fixed,
            nested,
            cert,
            rule: if nest.discrete { 1 } else { 2 },
            units: nest.units,
        };
        match nest.kind {
            NestKind::Call { n_k } => Event::Tree(Tree::of_child(child, nest.p, n_k), carried),
            NestKind::Child {
                target_index,
                refine,
            } => {
                child.units.refine += refine;
                Event::Child(child, carried, target_index)
            }
        }
    }

    /// The branching call is complete: its tree, the chosen child being the
    /// best, with the maps it owns. Every member of the target cell attains in
    /// the search-free run, being one twin class; in a walk the attaining members are those
    /// whose class of `~` holds a completed child equal to the best.
    fn finish_branch(&self, mut b: Branch) -> Result<Event, Stop> {
        let Some(child) = b.best.take() else {
            return Err(Stop::Failed);
        };
        let attaining = if self.walking {
            b.relation.related_to(&b.attain)
        } else {
            b.target.len() as u64
        };
        let mut tree = Tree::of_child(child, b.p, b.n_k);
        tree.child_rule = tree.rule;
        tree.rule = 3;
        tree.target = b.members;
        tree.attaining = attaining;
        tree.stopped_s = b.stopped_s;
        tree.stopped_e = b.stopped_e;
        Ok(Event::Tree(tree, b.own))
    }
}

/// What a call is run on: the instance's parts, borrowed.
struct CallRef<'x> {
    adj: &'x Adj,
    pair: &'x [Side; 2],
    node_g: &'x [usize],
    line_g: &'x [usize],
    p: u64,
}

/// The outcome of a fixing: a nest to run, or the stop of 10.4 that fired.
enum Nested<'a> {
    Frame(Box<Nest<'a>>),
    Cut(Cut),
}

/// What the top frame does next.
enum Act<'a> {
    /// Push this frame.
    Push(Frame<'a>),
    /// The top frame is finished.
    Finish,
    /// Look at the top frame again.
    Again,
}

/// A side whose cells are the given `(start, size)` ranges and whose vertices
/// are in index order.
fn identity_side(cells: &[(usize, usize)], n: usize) -> Side {
    let mut side = Side {
        elems: (0..n).collect(),
        pos: (0..n).collect(),
        start: vec![0; n],
        size: vec![0; n],
    };
    for &(c, k) in cells {
        side.size[c] = k;
        side.start[c..c + k].fill(c);
    }
    side
}

/// One top component: its search-free run, then its step of the walk.
struct Run<'a> {
    engine: Engine<'a>,
    fresh: Inst,
    fresh_units: u64,
    record: ComponentRecord,
    w: u64,
    smallest_node: usize,
    tree: Option<Tree>,
}

impl<'a> Run<'a> {
    fn new(
        set: &'a LineSet,
        comp: &'a Comp,
        index: &'a Index,
        stat: Static,
        stop: Option<&'a dyn StopSignal>,
    ) -> Self {
        let Static {
            mut record,
            start,
            w,
        } = stat;
        let mut engine = Engine::new(set, comp, index, stop, start);
        let (fresh, fresh_units) = engine.fresh();
        let sizes: Vec<usize> = fresh.pair[0]
            .starts()
            .map(|c| fresh.pair[0].size[c])
            .collect();
        record.singled_out = sizes.iter().filter(|&&s| s == 1).count() as u64;
        record.unsettled = record.blank_nodes - record.singled_out;
        record.largest_cell = sizes
            .iter()
            .copied()
            .filter(|&s| s >= 2)
            .max()
            .map(|s| s as u64);
        record.units.refine = fresh_units;
        engine.explored.start = record.units.start;
        engine.explored.refine = fresh_units;
        let smallest_node = (0..comp.nodes.len())
            .min_by(|&a, &b| engine.source_of(a).cmp(engine.source_of(b)))
            .unwrap_or(0);
        Self {
            engine,
            fresh,
            fresh_units,
            record,
            w,
            smallest_node,
            tree: None,
        }
    }
}

impl ComponentRun for Run<'_> {
    fn search_free(&mut self) -> Result<(), Stop> {
        self.engine.walking = false;
        match self.engine.run(self.fresh.clone(), 0) {
            Ok(tree) => {
                self.tree = Some(tree);
                Ok(())
            }
            Err(Stop::Failed) => Ok(()),
            Err(halt) => Err(halt),
        }
    }

    fn walk(&mut self, mut walk: Walk, label: bool) -> (Walk, Result<(), Stop>) {
        if !label {
            // The search-free run labelled the component: the walk's charges
            // here are its own, made at once unless the walk ends among them.
            let mut totals = std::mem::take(&mut self.engine.walk).totals();
            totals.add(Kind::Start, self.record.units.start);
            totals.add(Kind::Refine, self.fresh_units);
            if walk.absorb(&totals) {
                return (walk, Ok(()));
            }
        }
        let saved = self.engine.explored;
        self.engine.walk = walk;
        self.engine.walking = true;
        let result = self
            .engine
            .charge(Kind::Start, self.record.units.start)
            .and_then(|()| self.engine.charge(Kind::Refine, self.fresh_units))
            .and_then(|()| self.engine.run(self.fresh.clone(), 0));
        let outcome = result.map(|tree| {
            if label {
                self.tree = Some(tree);
            } else {
                self.engine.explored = saved;
            }
        });
        self.engine.walking = false;
        (std::mem::take(&mut self.engine.walk), outcome)
    }

    fn is_labelled(&self) -> bool {
        self.tree.is_some()
    }

    fn failures(&self) -> Vec<FailureDiagnostic> {
        failure_diagnostics(
            self.engine.set,
            self.engine.comp,
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

    fn finish(self) -> Result<Option<Labelling>, Refusal> {
        let Some(tree) = self.tree else {
            return Ok(None);
        };
        let parts = LabellingParts {
            smallest_node: self.smallest_node,
            twin_units: self.engine.twin_units,
            w: self.w,
            steps: Vec::new(),
            comparison_units: 0,
            explored: self.engine.explored,
        };
        finish_labelling(self.record, tree, parts).map(Some)
    }
}

/// Label an admitted set with the optimised engine. `force_search` skips the
/// search-free run and labels every component by its walk.
///
/// # Errors
/// `width` when the allowance overflows; `limit` when the walk of 8.3 carries
/// its running total above `max(limit, A)`; [`Halt::Stopped`] when `stop` is
/// set, polled before the first call and at every call.
pub(crate) fn label_set(
    set: &LineSet,
    limit: u64,
    force_search: bool,
    stop: Option<&dyn StopSignal>,
) -> Result<Labelled, Halt> {
    label_walked(set, limit, force_search, stop).map(|(labelled, _)| labelled)
}

/// [`label_set`] with the walk of 8.3 that decided the outcome.
///
/// # Errors
/// As [`label_set`].
pub(crate) fn label_walked(
    set: &LineSet,
    limit: u64,
    force_search: bool,
    stop: Option<&dyn StopSignal>,
) -> Result<(Labelled, Walk), Halt> {
    if is_stopped(stop) {
        return Err(Halt::Stopped);
    }
    let Prepared {
        comps,
        statics,
        allowance,
        split_charge,
    } = prepare(set).map_err(Halt::Refused)?;
    let indexes: Vec<Index> = comps.iter().map(Index::new).collect();
    let runs: Vec<Run<'_>> = comps
        .iter()
        .zip(&indexes)
        .zip(statics)
        .map(|((comp, index), stat)| Run::new(set, comp, index, stat, stop))
        .collect();
    let (labelled, _, walk) = drive(
        set,
        &comps,
        runs,
        (allowance, split_charge),
        limit,
        force_search,
        false,
    )?;
    Ok((labelled, walk))
}

/// An upper bound, in bytes, of the arrays the engine builds for the top
/// components and their first calls over `set`: the incidences and their
/// labels, the partitions, the line index of the twin test and the encodings
/// of the lines with every leaf free, proportional to the blank occurrences
/// (each one counted, so a record that repeats a node is charged for every
/// repetition), the line tokens, the vertices (7.12) and the structs each top
/// component holds. It is the admission of
/// [`compare_view_with_memory`](super::compare_view_with_memory); the
/// certificates and nested instances held while a search runs are not metered
/// beyond it. `None` when the bound passes the word.
pub(crate) fn working_bytes(set: &LineSet) -> Option<usize> {
    let word = size_of::<usize>();
    let mut occurrences = 0usize;
    let mut tokens = 0usize;
    let mut encoded = 0usize;
    for (l, line) in set.lines.iter().enumerate() {
        tokens = tokens.checked_add(line.len())?;
        occurrences = occurrences.checked_add(set.occurrences(l).count())?;
        encoded = encoded.checked_add(usize::try_from(set.free_len(l)).ok()?)?;
    }
    let vertices = set.nodes.len().checked_add(set.lines.len())?;
    // Per occurrence: an incidence at each end (three words each), the label
    // number and the copies a call and a child keep (sixteen words in all);
    // per token: the line as a key of the twin test (four words); per vertex:
    // the two partitions, the maps and the queue (thirty-two words).
    // Per top component: the structs held for it while the engine runs.
    let per_component = size_of::<Comp>()
        .checked_add(size_of::<Index>())?
        .checked_add(size_of::<Run<'static>>())?
        .checked_add(size_of::<Static>())?
        .checked_add(size_of::<Labelling>())?
        .checked_add(size_of::<Component>())?;
    let structs = component_count(set).checked_mul(per_component)?;
    let words = occurrences
        .checked_mul(16)?
        .checked_add(tokens.checked_mul(4)?)?
        .checked_add(vertices.checked_mul(32)?)?;
    words
        .checked_mul(word)?
        .checked_add(encoded.checked_mul(4)?)?
        .checked_add(structs)
}
