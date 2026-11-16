// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The optimised engine: the result of [`super::reference`], computed on dense
//! arrays with an explicit stack of calls and the shortcuts of section 10.
//!
//! A cell is a range of an array of its side, and its colour is the index at
//! which the range starts, so splitting a cell rewrites only the range of the
//! cell. An incidence carries its label as a slice of occurrence numbers, kept
//! as the input numbered them (8.2, item 4). Every call has its own compact
//! instance, rebuilt for each nested component, so the work of a call is
//! proportional to its own size. Calls are frames of one heap stack: nothing
//! recurses (6.6).
//!
//! The shortcuts, each scoped to one call and each checked against the call's
//! own state (10.1):
//!
//! * the strict stop of 10.4, on `s` and then on `E`;
//! * route (1): two children of one call with equal certificates give a map
//!   that is used only after it is checked to carry the lines of the call onto
//!   themselves; it joins classes of the target cell, and a child in the class
//!   of an explored child is not explored;
//! * route (2): the exchange of two tied nested calls of an explored child or
//!   of a rule 2 call, checked at the nearest branching call;
//! * route (4): the twins of the first explored child of a call;
//! * route (5): two complete leaves of the top call with equal line sets,
//!   found at the top call in place of the certificate key of route (1);
//! * lifting (route (3)): a map used at a branching call is used, unchecked,
//!   at every branching call below it on the stack.
//!
//! In descending order the same classes skip children; the attaining child of
//! smallest source identifier, which the result names, is explored last when
//! its class was skipped.

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BinaryHeap};

use super::certificate::{Certificate, Entry, entry_of};
use super::lines::{Leaf, LineSet, Tok};
use super::reference::{
    CallOutcome, Child, Classes, Comp, ComponentRun, Labelling, LabellingParts, Meter, Prepared,
    Static, Stop, Tree, TwinTest, drive, failure_diagnostics, finish_labelling, fragments_to_queue,
    positions, prepare,
};
use super::{ComponentRecord, FailureDiagnostic, Halt, Labelled, Refusal, Units};
use crate::StopSignal;
use crate::governor::is_stopped;

/// The optimised engine's own limit on the limited units of a search.
pub(super) const ENGINE_LIMIT: u64 = 1 << 34;

/// The shortcuts the engine uses, and the order in which it visits children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one independent switch per shortcut of 10.1 and 10.4, which the tests turn on one at a time"
)]
pub(crate) struct Options {
    /// The strict stop of 10.4.
    pub(crate) stops: bool,
    /// The routes of 10.1 in a search: the classes of the target cell. Without
    /// it no child is skipped.
    pub(crate) orbits: bool,
    /// Visit the children of a branching call in descending order of source
    /// identifier. The result is the same; only the stops fire elsewhere.
    pub(crate) descending: bool,
    /// Route (1) between children of equal certificate, and route (5) at the
    /// top call. Used only with `orbits`.
    pub(crate) maps: bool,
    /// Route (4): the twins of the first explored child. Used only with
    /// `orbits`.
    pub(crate) twins: bool,
    /// Route (2): the exchange of tied nested calls. Used only with `orbits`.
    pub(crate) exchanges: bool,
    /// Lifting: a map used at a call is used at the calls below it. Used only
    /// with `orbits`.
    pub(crate) lifting: bool,
    /// Route (5): leaves of the top call keyed by their line sets, in place of
    /// the certificate key of route (1). Used only with `orbits` and `maps`.
    pub(crate) leaves: bool,
    /// A shortcut of 10.5 that is wrong, for the tests that show it changes
    /// the result.
    #[cfg(test)]
    pub(crate) wrong: Wrong,
}

/// The shortcuts of 10.5 that fail, each reproduced so that a test can show
/// that it changes the result on the input the definition prints.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wrong {
    /// No wrong shortcut.
    Sound,
    /// (a) A child is stopped when `s` is equal and `E` is greater or equal.
    StopOnTie,
    /// (b) Every map found in the top component is used at every call.
    MapsEverywhere,
    /// (d) The child with the larger `s` is chosen before certificates are
    /// compared.
    InvariantFirst,
    /// (f) A child fixes the whole class of twins of its node.
    FixTwinClass,
    /// (c) The attaining child of smallest source identifier, when its class
    /// was skipped, takes the state of the explored child carried through the
    /// transposition of the two nodes.
    CarryChoice,
    /// (e) The input is refined once as a whole before the components start,
    /// and each component starts from the cells that gives it.
    SharedPartition,
}

impl Options {
    /// Every shortcut, ascending order.
    pub(crate) const PRODUCTION: Self = Self {
        stops: true,
        orbits: true,
        descending: false,
        maps: true,
        twins: true,
        exchanges: true,
        lifting: true,
        leaves: true,
        #[cfg(test)]
        wrong: Wrong::Sound,
    };
}

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
    let ids: Vec<usize> = touched.iter().map(|t| t.0).collect();
    let key = |t: &(usize, usize, usize)| run[t.1..t.2].iter().map(|r| &occ_nums[r.2..r.2 + r.3]);
    touched.sort_by(|a, b| key(a).cmp(key(b)));
    let size = side.size[cy];
    let untouched = size - ids.len();
    let mut groups = 1;
    for pair in touched.windows(2) {
        if key(&pair[0]).cmp(key(&pair[1])) != Ordering::Equal {
            groups += 1;
        }
    }
    if untouched == 0 && groups == 1 {
        return;
    }
    let mut order: Vec<usize> = Vec::with_capacity(size);
    order.extend(
        side.elems[cy..cy + size]
            .iter()
            .copied()
            .filter(|m| ids.binary_search(m).is_err()),
    );
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
        fragments.push((cy + order.len(), end - at));
        order.extend(touched[at..end].iter().map(|t| t.0));
        at = end;
    }
    for (i, &v) in order.iter().enumerate() {
        side.elems[cy + i] = v;
        side.pos[v] = cy + i;
    }
    for &(colour, count) in &fragments {
        side.size[colour] = count;
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
    failed: bool,
}

/// A rule 3 call in progress.
struct Branch {
    adj: Adj,
    pair: [Side; 2],
    node_g: Vec<usize>,
    line_g: Vec<usize>,
    p: u64,
    n_k: u64,
    /// The target cell, local nodes in ascending source order.
    target: Vec<usize>,
    /// Indices into `target` in visiting order.
    explore: Vec<usize>,
    next: usize,
    best: Option<Child>,
    /// Final positions below the first completed child of each key, by local
    /// node: the certificate of the child, or at the top call (route (5)) the
    /// sorted entries of its leaf.
    witnesses: BTreeMap<Vec<Entry>, Vec<usize>>,
    /// The certificates of the witnesses of the non-top key.
    cert_witnesses: BTreeMap<Certificate, Vec<usize>>,
    /// The most witnesses kept: a bound on memory, not on the result.
    witness_cap: usize,
    /// Target index by global node, ascending by node.
    by_global: Vec<(usize, usize)>,
    /// Whether the child of each target member was visited.
    visited: Vec<bool>,
    /// The child explored although its class was explored.
    forced: Option<usize>,
    /// The target index of the child that is the best so far.
    #[cfg(test)]
    best_ti: usize,
    /// Target indices of the explored children that attain the best so far.
    attain: Vec<usize>,
    /// Union-find over target indices: the classes of checked maps and twins.
    classes: Classes,
    class_explored: Vec<bool>,
    /// Local node to target index.
    target_index: Vec<usize>,
    stopped_s: u64,
    stopped_e: u64,
    search_free: bool,
    /// Whether the routes apply at this call.
    orbits: bool,
    /// The children whose certificates or stops are known.
    explored_count: usize,
}

enum Frame<'a> {
    Branch(Box<Branch>),
    Nest(Box<Nest<'a>>),
}

/// What a frame delivers to the frame below it.
enum Event {
    Tree(Tree),
    Child(Child, usize),
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
    options: Options,
    meter: Meter,
    twin: TwinTest,
    /// First failures: `(p, size of the target cell, local node)`.
    failures: Vec<(u64, u64, usize)>,
    stop: Option<&'a dyn StopSignal>,
    /// The path position of each fixed node of the top component.
    fixed_pos: Vec<Option<u64>>,
    /// The map under test, identity between checks.
    map: Vec<usize>,
    /// Final positions scratch, by local node of the top component.
    lambda: Vec<u64>,
    scratch: Scratch,
    /// Maps used at a branching call, as moved `(node, image)` pairs by node
    /// of the top component, still to be lifted to the calls below it.
    found: Vec<Vec<(usize, usize)>>,
    /// Exchanges of tied nested calls (route (2)) found by the last nest.
    exchanges: Vec<Vec<(usize, usize)>>,
    /// Branching calls on the stack.
    branch_depth: usize,
    /// Local node of the call under test, by node of the top component.
    local: Vec<usize>,
    /// Every map found in the top component (`Wrong::MapsEverywhere` only).
    #[cfg(test)]
    pool: Vec<Vec<(usize, usize)>>,
    /// Whether the pool holds the maps of a whole first pass.
    #[cfg(test)]
    pooled: bool,
}

impl<'a> Engine<'a> {
    /// An engine over `comp` with nothing run yet; `start` is the start charge
    /// of each line of the component.
    fn new(
        set: &'a LineSet,
        comp: &'a Comp,
        index: &'a Index,
        options: Options,
        stop: Option<&'a dyn StopSignal>,
        limit: u64,
        start: Vec<u64>,
    ) -> Self {
        Self {
            set,
            comp,
            index,
            options,
            meter: Meter::new(limit),
            twin: TwinTest::new(set, comp, start),
            failures: Vec::new(),
            stop,
            fixed_pos: vec![None; comp.nodes.len()],
            map: (0..comp.nodes.len()).collect(),
            lambda: vec![0; comp.nodes.len()],
            scratch: Scratch::default(),
            found: Vec::new(),
            exchanges: Vec::new(),
            branch_depth: 0,
            local: vec![usize::MAX; comp.nodes.len()],
            #[cfg(test)]
            pool: Vec::new(),
            #[cfg(test)]
            pooled: false,
        }
    }
}

impl<'a> Engine<'a> {
    fn source_of(&self, v: usize) -> &'a [u8] {
        self.set.source(self.comp.nodes[v])
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
        let search_free = self.meter.search_free;
        let explore: Vec<usize> = if search_free {
            let u = inst.node_g[target[0]];
            for &w in &target[1..] {
                let w = inst.node_g[w];
                let (ru, rw) = (self.twin.find(u), self.twin.find(w));
                if ru == rw {
                    continue;
                }
                if self.twin.test(self.set, self.comp, &mut self.meter, u, w) {
                    self.twin.join(ru, rw);
                } else {
                    self.failures.push((p, k as u64, u));
                    return Err(Stop::Failed);
                }
            }
            vec![0]
        } else if self.options.descending {
            (0..k).rev().collect()
        } else {
            (0..k).collect()
        };
        let mut target_index = vec![usize::MAX; inst.node_g.len()];
        for (i, &v) in target.iter().enumerate() {
            target_index[v] = i;
        }
        let mut by_global: Vec<(usize, usize)> = target
            .iter()
            .enumerate()
            .map(|(i, &v)| (inst.node_g[v], i))
            .collect();
        by_global.sort_unstable();
        let Inst {
            node_g,
            line_g,
            adj,
            pair,
        } = inst;
        self.branch_depth += 1;
        let witness_cap = ((1usize << 22) / node_g.len().max(1)).max(8);
        let mut branch = Box::new(Branch {
            adj,
            pair,
            node_g,
            line_g,
            p,
            n_k,
            target,
            explore,
            next: 0,
            best: None,
            witnesses: BTreeMap::new(),
            cert_witnesses: BTreeMap::new(),
            witness_cap,
            by_global,
            visited: vec![false; k],
            forced: None,
            #[cfg(test)]
            best_ti: 0,
            attain: Vec::new(),
            classes: Classes::new(k),
            class_explored: vec![false; k],
            target_index,
            stopped_s: 0,
            stopped_e: 0,
            search_free,
            orbits: self.options.orbits && !search_free,
            explored_count: 0,
        });
        self.seed_pool(&mut branch);
        Ok(Frame::Branch(branch))
    }

    /// The maps a wrong shortcut keeps for every call (10.5(b)).
    #[cfg(test)]
    fn seed_pool(&self, branch: &mut Branch) {
        for pairs in &self.pool {
            join_pairs(branch, pairs, true);
        }
    }

    #[cfg(not(test))]
    #[expect(clippy::unused_self, reason = "the sound engine keeps no pool")]
    const fn seed_pool(&self, _branch: &mut Branch) {}

    /// FIX the singleton node cells of the call's pair, form the entries,
    /// split the remainder into its free components and build their instances.
    /// With a `bound`, the stops of 10.4 apply once `s`, and then `E`, are known.
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
                units.entries += 1 + (entry.encoding.len() as u64).div_ceil(8);
                entries.push(entry);
            }
        }
        entries.sort();
        let removed_incidences: u64 = fixed_local
            .iter()
            .map(|&v| call.adj[0][v].len() as u64)
            .sum();
        units.fix += s + entries.len() as u64 + removed_incidences;
        #[cfg(test)]
        let on_tie = self.options.wrong == Wrong::StopOnTie;
        #[cfg(not(test))]
        let on_tie = false;
        let cut = bound.and_then(|best| {
            if !self.options.stops {
                return None;
            }
            match s.cmp(&best.s) {
                Ordering::Greater => Some(Cut::OnS),
                Ordering::Equal
                    if entries > best.entries || (on_tie && entries == best.entries) =>
                {
                    Some(Cut::OnE)
                }
                _ => None,
            }
        });
        if let Some(cut) = cut {
            self.undo(&fixed);
            self.meter.charge(units)?;
            return Ok(Nested::Cut(cut));
        }
        let free_nodes = call.pair[0]
            .starts()
            .filter(|&c| call.pair[0].size[c] >= 2)
            .count();
        let components = self.split(call, &mut units);
        self.meter.charge(units)?;
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
            failed: false,
        })))
    }

    fn undo(&mut self, fixed: &[(usize, u64)]) {
        for &(v, _) in fixed {
            self.fixed_pos[v] = None;
        }
    }

    /// The free components of the call after its fixing, as instances, the
    /// smallest source last; the split is charged to `units`.
    fn split(&self, call: &CallRef<'_>, units: &mut Units) -> Vec<(&'a [u8], Inst)> {
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
            return Vec::new();
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
        if count >= 2 {
            units.fix += sizes.iter().sum::<u64>() - sizes.iter().copied().max().unwrap_or(0);
        }
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
        out
    }
}

/// The wrong shortcut of 10.5(c): the state of the best child, with the
/// target nodes of the two children exchanged, stands for the child `minimum`.
#[cfg(test)]
fn carry_choice(b: &mut Branch, minimum: usize) -> bool {
    let (u, w) = (b.node_g[b.target[b.best_ti]], b.node_g[b.target[minimum]]);
    let Some(best) = b.best.as_mut() else {
        return false;
    };
    let swap = |x: &mut usize| {
        if *x == u {
            *x = w;
        } else if *x == w {
            *x = u;
        }
    };
    for (x, _) in &mut best.fixed {
        swap(x);
    }
    let mut work: Vec<&mut Tree> = best.nested.iter_mut().collect();
    while let Some(tree) = work.pop() {
        for (x, _) in &mut tree.fixed {
            swap(x);
        }
        tree.target.iter_mut().for_each(swap);
        work.extend(tree.nested.iter_mut());
    }
    b.best_ti = minimum;
    true
}

/// The cells of the node and line partitions of `comp` after refining the
/// whole of `set` once (10.5(e)), restricted to `comp`, in local indices.
#[cfg(test)]
pub(crate) fn shared_cells(set: &LineSet, comp: &Comp) -> [Vec<Vec<usize>>; 2] {
    let all = super::reference::top_components(set);
    let mut merged = Comp {
        nodes: Vec::new(),
        lines: Vec::new(),
        occ: Vec::new(),
        node_lines: Vec::new(),
        line_nodes: Vec::new(),
    };
    let (mut node_at, mut line_at) = (0, 0);
    for c in &all {
        if c.nodes[0] == comp.nodes[0] {
            (node_at, line_at) = (merged.nodes.len(), merged.lines.len());
        }
        let (nn, ln) = (merged.nodes.len(), merged.lines.len());
        merged.nodes.extend(&c.nodes);
        merged.lines.extend(&c.lines);
        merged
            .occ
            .extend(c.occ.iter().map(|o| o.iter().map(|&x| x + nn).collect()));
        merged.node_lines.extend(
            c.node_lines
                .iter()
                .map(|v| v.iter().map(|&l| l + ln).collect()),
        );
        merged.line_nodes.extend(
            c.line_nodes
                .iter()
                .map(|v| v.iter().map(|&x| x + nn).collect()),
        );
    }
    let index = Index::new(&merged);
    let mut engine = Engine::new(
        set,
        &merged,
        &index,
        Options::PRODUCTION,
        None,
        ENGINE_LIMIT,
        vec![0; merged.lines.len()],
    );
    let (inst, _) = engine.fresh();
    let restrict = |side: &Side, at: usize, len: usize| -> Vec<Vec<usize>> {
        side.starts()
            .map(|c| {
                side.elems[c..c + side.size[c]]
                    .iter()
                    .filter(|&&x| x >= at && x < at + len)
                    .map(|&x| x - at)
                    .collect::<Vec<usize>>()
            })
            .filter(|cell| !cell.is_empty())
            .collect()
    };
    [
        restrict(&inst.pair[0], node_at, comp.nodes.len()),
        restrict(&inst.pair[1], line_at, comp.lines.len()),
    ]
}

/// The node cells of each top component after its fresh refinement, as source
/// identifiers: alone, or after refining the whole input (10.5(e)).
#[cfg(test)]
pub(crate) fn fresh_node_cells(set: &LineSet, shared: bool) -> Vec<Vec<Vec<Vec<u8>>>> {
    let comps = super::reference::top_components(set);
    comps
        .iter()
        .map(|comp| {
            let cells = if shared {
                let [nodes, _] = shared_cells(set, comp);
                nodes
            } else {
                let index = Index::new(comp);
                let mut engine = Engine::new(
                    set,
                    comp,
                    &index,
                    Options::PRODUCTION,
                    None,
                    ENGINE_LIMIT,
                    vec![0; comp.lines.len()],
                );
                let (inst, _) = engine.fresh();
                inst.pair[0]
                    .starts()
                    .map(|c| inst.pair[0].elems[c..c + inst.pair[0].size[c]].to_vec())
                    .collect()
            };
            cells
                .into_iter()
                .map(|cell| {
                    let mut named: Vec<Vec<u8>> = cell
                        .into_iter()
                        .map(|v| set.source(comp.nodes[v]).to_vec())
                        .collect();
                    named.sort();
                    named
                })
                .collect()
        })
        .collect()
}

/// Join the classes of two target indices of a call.
fn join_classes(b: &mut Branch, i: usize, j: usize) {
    let (ri, rj) = (b.classes.find(i), b.classes.find(j));
    if ri != rj {
        b.classes.join(ri, rj);
        b.class_explored[ri] = b.class_explored[ri] || b.class_explored[rj];
    }
}

/// Use a map found below the call, given as moved `(node, image)` pairs, at a
/// call whose automorphism it is by lifting (10.1): no check, because it fixes
/// the call's constants and every line holds free nodes of one nested
/// component. A map with a pair that leaves the target cell is not used, unless
/// `lenient`, which joins the pairs inside it and ignores the rest: the wrong
/// shortcut of 10.5(b), used by the tests only.
fn join_pairs(b: &mut Branch, pairs: &[(usize, usize)], lenient: bool) {
    if !b.orbits {
        return;
    }
    let index = |g: usize| {
        b.by_global
            .binary_search_by_key(&g, |&(node, _)| node)
            .ok()
            .map(|at| b.by_global[at].1)
    };
    let mut joins = Vec::new();
    for &(g, h) in pairs {
        match (index(g), index(h)) {
            (Some(i), Some(j)) => joins.push((i, j)),
            (None, None) => {}
            _ if lenient => {}
            _ => return,
        }
    }
    for (i, j) in joins {
        join_classes(b, i, j);
    }
}

/// The map that sends each node of the call to the node of the other leaf with
/// the same final position (6.5); `None` when the positions are not a
/// permutation.
fn map_between(first: &[usize], new: &[usize]) -> Option<Vec<usize>> {
    let n = first.len();
    let mut inverse = vec![usize::MAX; n];
    for (x, &position) in first.iter().enumerate() {
        if position >= n {
            return None;
        }
        inverse[position] = x;
    }
    new.iter()
        .map(|&position| inverse.get(position).copied().filter(|&x| x != usize::MAX))
        .collect()
}

/// The nodes of a result path below `tree` with their final positions, the
/// offset of `tree` taken as `0`, ascending.
fn tree_nodes(tree: &Tree) -> Vec<(u64, usize)> {
    let mut out = Vec::new();
    let mut work = vec![(tree, 0u64)];
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

/// Route (2): for each pair of consecutive tied calls (equal certificates, in
/// the order of 6.4), the exchange of the two, as moved `(node, image)` pairs.
fn tied_exchanges(nested: &[Tree]) -> Vec<Vec<(usize, usize)>> {
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

impl<'a> Engine<'a> {
    /// Run the call on `top` with argument `p` to its tree, on the heap stack.
    fn run(&mut self, top: Inst, p: u64) -> Result<Tree, Stop> {
        self.branch_depth = 0;
        self.found.clear();
        self.exchanges.clear();
        let mut stack: Vec<Frame<'a>> = vec![self.begin(top, p)?];
        let mut event: Option<Event> = None;
        loop {
            if let Some(delivered) = event.take() {
                let Some(frame) = stack.last_mut() else {
                    return match delivered {
                        Event::Tree(tree) => Ok(tree),
                        _ => Err(Stop::Failed),
                    };
                };
                let mut unwind = false;
                match (frame, delivered) {
                    (Frame::Nest(nest), Event::Tree(tree)) => {
                        nest.sourced.push((nest.current, tree));
                    }
                    (Frame::Nest(nest), Event::Failed) => nest.failed = true,
                    (Frame::Branch(branch), Event::Child(child, target_index)) => {
                        self.child_done(branch, child, target_index);
                    }
                    (Frame::Branch(_), Event::Failed) => unwind = true,
                    // INVARIANT: a nest receives trees and failures only, a
                    // branch children and failures only.
                    _ => return Err(Stop::Failed),
                }
                if unwind {
                    stack.pop();
                    self.branch_depth -= 1;
                    event = Some(Event::Failed);
                    continue;
                }
                if !self.found.is_empty() {
                    self.lift(&mut stack);
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
            if !self.found.is_empty() {
                self.lift(&mut stack);
            }
            match act {
                Act::Push(frame) => stack.push(frame),
                Act::Again => {}
                Act::Finish => match stack.pop() {
                    Some(Frame::Nest(nest)) => {
                        event = Some(self.finish_nest(*nest));
                        self.use_exchanges(&mut stack);
                    }
                    Some(Frame::Branch(branch)) => {
                        self.branch_depth -= 1;
                        event = Some(Event::Tree(Self::finish_branch(*branch)?));
                    }
                    None => return Err(Stop::Failed),
                },
            }
        }
    }

    /// The next child of a branching call: its frame, or the end of the call.
    fn step_branch(&mut self, b: &mut Branch) -> Result<Act<'a>, Stop> {
        loop {
            while b.next < b.explore.len() {
                let ti = b.explore[b.next];
                b.next += 1;
                if b.orbits && b.forced != Some(ti) {
                    let root = b.classes.find(ti);
                    if b.class_explored[root] {
                        continue;
                    }
                }
                let v = b.target[ti];
                let mut pair = b.pair.clone();
                let mut queue = Queue::new([b.node_g.len(), b.line_g.len()]);
                individualise(&mut pair[0], v, &mut queue);
                #[cfg(test)]
                if self.options.wrong == Wrong::FixTwinClass {
                    let gv = b.node_g[v];
                    for &w in &b.target {
                        let gw = b.node_g[w];
                        if w != v
                            && pair[0].cell_size_of(w) >= 2
                            && self.twin.is_pair(self.set, self.comp, gv, gw)
                        {
                            individualise(&mut pair[0], w, &mut queue);
                        }
                    }
                }
                let refine_units = refine(
                    &b.adj,
                    &self.index.occ_nums,
                    &mut pair,
                    &mut queue,
                    &mut self.scratch,
                );
                self.meter.charge(Units {
                    refine: refine_units,
                    ..Units::default()
                })?;
                let call = CallRef {
                    adj: &b.adj,
                    pair: &pair,
                    node_g: &b.node_g,
                    line_g: &b.line_g,
                    p: b.p,
                };
                let bound = b.best.as_ref().map(|child| &child.cert);
                let kind = NestKind::Child {
                    target_index: ti,
                    refine: refine_units,
                };
                match self.nest(&call, kind, bound)? {
                    Nested::Frame(nest) => return Ok(Act::Push(Frame::Nest(nest))),
                    Nested::Cut(cut) => self.cut(b, ti, cut),
                }
            }
            // The result names the attaining child of smallest source
            // identifier: explore it if its class was skipped.
            let missing = if b.orbits {
                Self::unvisited_minimum(b)
            } else {
                None
            };
            let Some(minimum) = missing else {
                return Ok(Act::Finish);
            };
            #[cfg(test)]
            if self.options.wrong == Wrong::CarryChoice && carry_choice(b, minimum) {
                b.visited[minimum] = true;
                continue;
            }
            b.forced = Some(minimum);
            b.explore.push(minimum);
        }
    }

    /// The smallest target index whose child attains, when its child was not
    /// visited.
    fn unvisited_minimum(b: &mut Branch) -> Option<usize> {
        let mut roots: Vec<usize> = b.attain.iter().map(|&ti| b.classes.find(ti)).collect();
        roots.sort_unstable();
        roots.dedup();
        let minimum =
            (0..b.target.len()).find(|&ti| roots.binary_search(&b.classes.find(ti)).is_ok())?;
        (!b.visited[minimum]).then_some(minimum)
    }

    /// A child the stop of 10.4 ended: classified as not attaining.
    fn cut(&mut self, b: &mut Branch, target_index: usize, cut: Cut) {
        match cut {
            Cut::OnS => b.stopped_s += 1,
            Cut::OnE => b.stopped_e += 1,
        }
        self.explored_child(b, target_index);
    }

    /// A child completed: the best so far, the attaining list, and the checked
    /// map between it and the best when their certificates are equal.
    fn child_done(&mut self, b: &mut Branch, child: Child, target_index: usize) {
        if b.orbits && self.options.maps {
            self.record_child(b, &child);
        }
        #[cfg(test)]
        let invariant_first = self.options.wrong == Wrong::InvariantFirst;
        #[cfg(not(test))]
        let invariant_first = false;
        let order = b.best.as_ref().map_or(Ordering::Less, |best| {
            if invariant_first {
                best.cert
                    .s
                    .cmp(&child.cert.s)
                    .then_with(|| child.cert.cmp(&best.cert))
            } else {
                child.cert.cmp(&best.cert)
            }
        });
        match order {
            Ordering::Less => {
                b.attain.clear();
                b.attain.push(target_index);
                b.best = Some(child);
                #[cfg(test)]
                {
                    b.best_ti = target_index;
                }
            }
            Ordering::Equal => {
                b.attain.push(target_index);
                if self.options.descending {
                    // Ties go to the later child, the smaller source.
                    b.best = Some(child);
                    #[cfg(test)]
                    {
                        b.best_ti = target_index;
                    }
                }
            }
            Ordering::Greater => {}
        }
        self.explored_child(b, target_index);
    }

    /// Mark the class of a child explored; after the first, test the twins of
    /// that child in the rest of the target cell (route (4)).
    fn explored_child(&mut self, b: &mut Branch, target_index: usize) {
        let root = b.classes.find(target_index);
        b.class_explored[root] = true;
        b.visited[target_index] = true;
        b.explored_count += 1;
        if !b.orbits || !self.options.twins || b.explored_count != 1 {
            return;
        }
        let u = b.node_g[b.target[target_index]];
        for tj in 0..b.target.len() {
            let (ru, rj) = (b.classes.find(target_index), b.classes.find(tj));
            if ru == rj {
                continue;
            }
            if self.twin.test(
                self.set,
                self.comp,
                &mut self.meter,
                u,
                b.node_g[b.target[tj]],
            ) {
                b.classes.join(ru, rj);
                b.class_explored[ru] = b.class_explored[ru] || b.class_explored[rj];
                if self.options.lifting && self.branch_depth >= 2 {
                    let w = b.node_g[b.target[tj]];
                    self.found.push(vec![(u, w), (w, u)]);
                }
            }
        }
    }

    /// The final positions below `child`, by local node of the call (6.5).
    fn lambda_of(&mut self, child: &Child, b: &Branch) -> Vec<usize> {
        for &(v, q) in &child.fixed {
            self.lambda[v] = q - b.p;
        }
        let s = child.fixed.len() as u64;
        let mut t = 0;
        for nested in &child.nested {
            positions(nested, s + t, &mut self.lambda);
            t += nested.n_k;
        }
        b.node_g
            .iter()
            .map(|&g| usize::try_from(self.lambda[g]).unwrap_or(usize::MAX))
            .collect()
    }

    /// Routes (1) and (5): the map between a completed child and the first
    /// completed child with the same key, used only when it is checked to
    /// carry the lines of the call onto themselves (10.1). At the top call the
    /// key is the set of lines of the child's leaf, else its certificate.
    fn record_child(&mut self, b: &mut Branch, child: &Child) {
        let lambda = self.lambda_of(child, b);
        let hit = if self.options.leaves && b.p == 0 {
            let key = self.leaf_key(b, &lambda);
            let hit = b
                .witnesses
                .get(&key)
                .map(|first| map_between(first, &lambda));
            if hit.is_none() && b.witnesses.len() < b.witness_cap {
                b.witnesses.insert(key, lambda);
            }
            hit
        } else {
            let hit = b
                .cert_witnesses
                .get(&child.cert)
                .map(|first| map_between(first, &lambda));
            if hit.is_none() && b.cert_witnesses.len() < b.witness_cap {
                b.cert_witnesses.insert(child.cert.clone(), lambda);
            }
            hit
        };
        if let Some(Some(image)) = hit {
            self.try_map(b, &image);
        }
    }

    /// The lines of the top component with every blank leaf replaced by its
    /// final position, ascending (10.1, route (5)). The top call has nothing
    /// fixed above it, so its local nodes are the nodes of the component.
    fn leaf_key(&self, b: &Branch, lambda: &[usize]) -> Vec<Entry> {
        let mut entries: Vec<Entry> = b
            .line_g
            .iter()
            .map(|&lg| {
                let positions: Vec<u64> = self.comp.occ[lg]
                    .iter()
                    .map(|&x| lambda[x] as u64)
                    .collect();
                entry_of(self.set, self.comp.lines[lg], &positions)
            })
            .collect();
        entries.sort();
        entries
    }

    /// Use `image` (local node to local node of the call) at the call when it
    /// is checked: it joins the classes of the target cell and is offered to
    /// the calls below. Whether it was used.
    fn try_map(&mut self, b: &mut Branch, image: &[usize]) -> bool {
        // A map of the call preserves its cells; a map that does not is not used.
        if b.target
            .iter()
            .any(|&x| b.target_index[image[x]] == usize::MAX)
        {
            return false;
        }
        if !self.is_automorphism(b, image) {
            return false;
        }
        let joins: Vec<(usize, usize)> = b
            .target
            .iter()
            .enumerate()
            .map(|(ti, &x)| (ti, b.target_index[image[x]]))
            .collect();
        for (ti, tj) in joins {
            join_classes(b, ti, tj);
        }
        #[cfg(test)]
        if self.options.wrong == Wrong::MapsEverywhere {
            let pairs: Vec<(usize, usize)> = image
                .iter()
                .enumerate()
                .filter(|&(x, &y)| x != y)
                .map(|(x, &y)| (b.node_g[x], b.node_g[y]))
                .collect();
            self.pool.push(pairs);
        }
        if self.options.lifting && self.branch_depth >= 2 {
            let pairs: Vec<(usize, usize)> = image
                .iter()
                .enumerate()
                .filter(|&(x, &y)| x != y)
                .map(|(x, &y)| (b.node_g[x], b.node_g[y]))
                .collect();
            if !pairs.is_empty() {
                self.found.push(pairs);
            }
        }
        true
    }

    /// The map that moves each node of `pairs` to its image and fixes every
    /// other node, as local nodes of the call; `None` when a node is not in it.
    fn image_of(&mut self, b: &Branch, pairs: &[(usize, usize)]) -> Option<Vec<usize>> {
        for (x, &g) in b.node_g.iter().enumerate() {
            self.local[g] = x;
        }
        let mut image: Vec<usize> = (0..b.node_g.len()).collect();
        let mut complete = true;
        for &(g, h) in pairs {
            let (x, y) = (self.local[g], self.local[h]);
            if x == usize::MAX || y == usize::MAX {
                complete = false;
                break;
            }
            image[x] = y;
        }
        for &g in &b.node_g {
            self.local[g] = usize::MAX;
        }
        complete.then_some(image)
    }

    /// Offer the maps found at the call on top of `stack` to the calls below
    /// it, without a check (lifting: 10.1).
    fn lift(&mut self, stack: &mut [Frame<'a>]) {
        let found = std::mem::take(&mut self.found);
        let Some((_, below)) = stack.split_last_mut() else {
            return;
        };
        Self::lift_to(below, &found);
    }

    fn lift_to(below: &mut [Frame<'a>], found: &[Vec<(usize, usize)>]) {
        for frame in below {
            if let Frame::Branch(a) = frame {
                for pairs in found {
                    join_pairs(a, pairs, false);
                }
            }
        }
    }

    /// Route (2): the exchanges of tied nested calls the last nest found, each
    /// checked at the nearest branching call below it and lifted from there.
    fn use_exchanges(&mut self, stack: &mut [Frame<'a>]) {
        let maps = std::mem::take(&mut self.exchanges);
        if maps.is_empty() {
            return;
        }
        let Some(at) = stack.iter().rposition(|f| matches!(f, Frame::Branch(_))) else {
            return;
        };
        let (below, rest) = stack.split_at_mut(at);
        let Some(Frame::Branch(b)) = rest.first_mut() else {
            return;
        };
        if !b.orbits {
            return;
        }
        for pairs in maps {
            let Some(image) = self.image_of(b, &pairs) else {
                continue;
            };
            if self.try_map(b, &image) {
                let found = std::mem::take(&mut self.found);
                Self::lift_to(below, &found);
            }
        }
    }

    /// Whether `image` (local node to local node of the call) carries the set
    /// of the call's lines onto itself with every constant held fixed (10.1).
    fn is_automorphism(&mut self, b: &Branch, image: &[usize]) -> bool {
        for (x, &y) in image.iter().enumerate() {
            self.map[b.node_g[x]] = b.node_g[y];
        }
        let ok = b.line_g.iter().all(|&lg| {
            let mut occurrences = self.comp.occ[lg].iter();
            let mut tokens = Vec::with_capacity(self.set.lines[self.comp.lines[lg]].len());
            for token in &*self.set.lines[self.comp.lines[lg]] {
                tokens.push(match *token {
                    Tok::Blank(_) => {
                        let Some(&node) = occurrences.next() else {
                            return false;
                        };
                        Tok::Blank(self.comp.nodes[self.map[node]])
                    }
                    other => other,
                });
            }
            self.twin.holds(&tokens)
        });
        for &g in &b.node_g {
            self.map[g] = g;
        }
        ok
    }

    /// The nest is complete: undo its fixing and deliver what it made.
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
        if self.options.orbits
            && self.options.exchanges
            && self.branch_depth > 0
            && nested.len() >= 2
        {
            self.exchanges = tied_exchanges(&nested);
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
            NestKind::Call { n_k } => Event::Tree(Tree::of_child(child, nest.p, n_k)),
            NestKind::Child {
                target_index,
                refine,
            } => {
                child.units.refine += refine;
                Event::Child(child, target_index)
            }
        }
    }

    /// The branching call is complete: its tree, the chosen child being the
    /// best, and the attaining count the classes of the attaining children.
    fn finish_branch(mut b: Branch) -> Result<Tree, Stop> {
        let Some(child) = b.best.take() else {
            return Err(Stop::Failed);
        };
        let k = b.target.len();
        let attaining = if b.search_free {
            k as u64
        } else {
            let mut roots: Vec<usize> = b.attain.iter().map(|&ti| b.classes.find(ti)).collect();
            roots.sort_unstable();
            roots.dedup();
            (0..k)
                .filter(|&ti| roots.binary_search(&b.classes.find(ti)).is_ok())
                .count() as u64
        };
        let mut tree = Tree::of_child(child, b.p, b.n_k);
        tree.child_rule = tree.rule;
        tree.rule = 3;
        tree.target = b.target.iter().map(|&v| b.node_g[v]).collect();
        tree.attaining = attaining;
        tree.stopped_s = b.stopped_s;
        tree.stopped_e = b.stopped_e;
        Ok(tree)
    }
}

/// One top component between its search-free run and its search.
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
        limit: u64,
        options: Options,
        stop: Option<&'a dyn StopSignal>,
    ) -> Self {
        let Static {
            mut record,
            start,
            w,
        } = stat;
        let mut engine = Engine::new(set, comp, index, options, stop, limit, start);
        let (fresh, fresh_units) = engine.fresh();
        #[cfg(test)]
        let fresh = if options.wrong == Wrong::SharedPartition {
            let [nodes, lines] = shared_cells(set, comp);
            Inst {
                pair: [
                    Side::from_cells(nodes, comp.nodes.len()),
                    Side::from_cells(lines, comp.lines.len()),
                ],
                ..fresh
            }
        } else {
            fresh
        };
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
        engine.meter.explored.start = record.units.start;
        engine.meter.explored.refine = fresh_units;
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
    fn base_units(&self) -> u64 {
        self.record.units.start + self.fresh_units
    }

    fn top_call(&mut self, search_free: bool, spent: &mut u64) -> CallOutcome {
        self.engine.meter.search_free = search_free;
        self.engine.meter.limited = *spent;
        // A wrong shortcut that uses every map found anywhere in the component
        // sees them all: a first pass finds them, the second uses them.
        #[cfg(test)]
        if self.engine.options.wrong == Wrong::MapsEverywhere && !self.engine.pooled {
            self.engine.pooled = true;
            let _ = self.engine.run(self.fresh.clone(), 0);
        }
        let result = self.engine.run(self.fresh.clone(), 0);
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
            twin_units: self.engine.meter.twin_units,
            w: self.w,
            steps: Vec::new(),
            comparison_units: 0,
            explored: self.engine.meter.explored,
        };
        finish_labelling(self.record, tree, parts).map(Some)
    }
}

/// Label an admitted set with the optimised engine. `force_search` skips the
/// search-free run and searches every component.
///
/// # Errors
/// `width` when the allowance overflows; `limit` when a search exceeds
/// `max(limit, A)` limited units; [`Halt::Stopped`] when `stop` is set, polled
/// before the first call and at every call.
pub(crate) fn label_set(
    set: &LineSet,
    limit: u64,
    force_search: bool,
    options: Options,
    stop: Option<&dyn StopSignal>,
) -> Result<Labelled, Halt> {
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
    let indexes: Vec<Index> = comps.iter().map(Index::new).collect();
    let runs: Vec<Run<'_>> = comps
        .iter()
        .zip(&indexes)
        .zip(statics)
        .map(|((comp, index), stat)| Run::new(set, comp, index, stat, limit, options, stop))
        .collect();
    let (labelled, _) = drive(
        set,
        &comps,
        runs,
        allowance,
        split_charge,
        force_search,
        false,
    )?;
    Ok(labelled)
}

/// An upper bound, in bytes, of the arrays the engine builds for the top
/// components and their first calls over `set`: the incidences and their
/// labels, the partitions, the line index of the twin test and the encodings
/// of the lines with every leaf free, proportional to the incidences `M`, the
/// constant leaf occurrences `Z` and the vertices (7.12). It is the admission
/// of [`compare_view_with_memory`](super::compare_view_with_memory); the
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
    let words = occurrences
        .checked_mul(16)?
        .checked_add(tokens.checked_mul(4)?)?
        .checked_add(vertices.checked_mul(32)?)?;
    words
        .checked_mul(word)?
        .checked_add(encoded.checked_mul(4)?)
}
