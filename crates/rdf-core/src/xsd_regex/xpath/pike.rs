// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Linear-time ordered matching for programs without backreferences.
//!
//! Without a backreference, the future of a partial match depends only on its
//! control state: the program point, the repetition counts that still decide
//! a bound, and which repetitions have consumed nothing in their current
//! iteration. Captures are carried along but never read. Two partial matches
//! with the same control state at the same input position therefore have the
//! same futures, and the one reached first in priority order is the only one
//! that can produce the reported match. This machine keeps one thread per
//! distinct control state at each position, in exactly the priority order the
//! backtracking machine explores, so it reports the same leftmost,
//! priority-ordered match and the same captures in time bounded by the input
//! length times the number of distinct control states.
//!
//! The program is walked through the flat node arena with a parent table built
//! at compilation: entering a node and leaving it are the program points, so
//! no second instruction encoding exists. Alternatives and repetition choices
//! are an explicit work list with undo records, never the machine stack.
//!
//! A repetition keeps no state per iteration. Its count is stored only when a
//! bound depends on it, saturated at the minimum when there is no finite
//! maximum, so `*`, `+` and `?` add at most two counter values. The progress
//! rule (an empty iteration stops the repetition once the minimum is met) is
//! decided by one level per thread: the outermost nullable repetition whose
//! current iteration began at the current position.

use super::compile::{Count, Node};
use super::r#match::{Captures, Ctx, decide};
use super::{Budget, Error, Resource};

/// The parent of the root node.
const ROOT: usize = usize::MAX;

/// A capture boundary that has not been set.
const UNSET: u64 = u64::MAX;

/// A thread that has consumed a character since every iteration it is in began.
const CONSUMED: u32 = u32::MAX;

/// A dense visited table is used when it has at most this many cells.
const DENSE_CELLS: usize = 1 << 16;

/// State cells moved by one step of work: copying a thread's counters and
/// capture boundaries is a block move, far cheaper per cell than a transition.
const COPY_CELLS: usize = 8;

/// The steps charged for copying or clearing `cells` state cells.
const fn copy_steps(cells: usize) -> u128 {
    cells.div_ceil(COPY_CELLS) as u128
}

/// The static position of one node in the program tree.
#[derive(Debug, Clone, Copy)]
pub(super) struct Link {
    /// The node whose operand this node is, or [`ROOT`].
    pub(super) parent: usize,
    /// Repetitions with a nullable body that enclose this node. A repetition
    /// with this many such ancestors tracks its progress at this level.
    pub(super) stall: u32,
    /// Repetitions whose bounds depend on their count that enclose this node.
    /// A counted repetition stores its count in the cell of this index.
    pub(super) counters: u32,
}

/// Static facts about a compiled program that its matchers read.
#[derive(Debug)]
pub(super) struct Links {
    /// One link per node of the program arena.
    pub(super) nodes: Vec<Link>,
    /// Whether any node reads a capture, which selects the backtracking machine.
    pub(super) backreferences: bool,
    /// The deepest nesting of nullable-body repetitions.
    pub(super) stall_depth: u32,
    /// The deepest nesting of counted repetitions: counter cells per thread.
    pub(super) counter_depth: u32,
}

/// Whether a repetition's later choices can depend on how many iterations it
/// has completed, so that its count must be stored while it iterates.
///
/// A count is stored only between completing an iteration and completing the
/// next, so only the values below the maximum (or, without a finite maximum,
/// below the minimum, which [`stored`] saturates) are distinguished. `?`, `*`,
/// `+` and `{1}` therefore store nothing.
const fn counted(min: Count, max: Option<Count>) -> bool {
    match (min, max) {
        (_, Some(Count::Finite(max))) => max >= 2,
        (Count::Finite(min), None | Some(Count::AboveU64)) => min >= 2,
        (Count::AboveU64, None | Some(Count::AboveU64)) => true,
    }
}

/// The count a repetition stores before another iteration: without a finite
/// maximum, every count from one below the minimum on completes that next
/// iteration at or above the minimum and so makes the same choices.
const fn stored(min: Count, max: Option<Count>, count: u64) -> u64 {
    match (min, max) {
        (Count::Finite(min), None | Some(Count::AboveU64)) if min > 0 && count >= min => min - 1,
        _ => count,
    }
}

/// The operands of a compound node, in source order.
fn operands(node: &Node) -> [Option<usize>; 2] {
    match *node {
        Node::Sequence(left, right) | Node::Choice(left, right) => [Some(left), Some(right)],
        Node::Capture { body, .. } | Node::Repeat { body, .. } => [Some(body), None],
        Node::Empty | Node::Character(_) | Node::Start | Node::End | Node::Backreference(_) => {
            [None, None]
        }
    }
}

fn reserve_compile<T>(vec: &mut Vec<T>, count: usize) -> Result<(), Error> {
    vec.try_reserve_exact(count).map_err(|_| Error::Allocation {
        resource: Resource::CompileSlots,
        units: count as u64,
    })
}

/// Build the parent table and the nesting facts of a parsed program.
///
/// Three linear passes over a breadth-first order of the tree, which lists
/// every parent before its operands. The table is retained with the program;
/// the order and the nullability flags are released.
pub(super) fn analyze(nodes: &[Node], root: usize, budget: &mut Budget) -> Result<Links, Error> {
    let count = nodes.len();
    budget.charge_wide(Resource::CompileSlots, count as u128 * 3)?;
    budget.charge_wide(Resource::CompileSlots, count as u128 * 2)?;
    budget.charge_wide(Resource::CompileSteps, count as u128 * 3)?;
    let mut links = Vec::new();
    reserve_compile(&mut links, count)?;
    links.resize(
        count,
        Link {
            parent: ROOT,
            stall: 0,
            counters: 0,
        },
    );
    let mut order = Vec::new();
    reserve_compile(&mut order, count)?;
    let mut nullable = Vec::new();
    reserve_compile(&mut nullable, count)?;
    nullable.resize(count, false);
    order.push(root);
    let mut index = 0;
    let mut backreferences = false;
    while let Some(&node) = order.get(index) {
        index += 1;
        backreferences |= matches!(nodes[node], Node::Backreference(_));
        for operand in operands(&nodes[node]).into_iter().flatten() {
            debug_assert!(order.len() < count, "the program arena is a tree");
            links[operand].parent = node;
            order.push(operand);
        }
    }
    for &node in order.iter().rev() {
        nullable[node] = match nodes[node] {
            Node::Character(_) => false,
            Node::Empty | Node::Start | Node::End | Node::Backreference(_) => true,
            Node::Sequence(left, right) => nullable[left] && nullable[right],
            Node::Choice(left, right) => nullable[left] || nullable[right],
            Node::Capture { body, .. } => nullable[body],
            Node::Repeat { body, min, .. } => min == Count::Finite(0) || nullable[body],
        };
    }
    let (mut stall_depth, mut counter_depth) = (0, 0);
    for &node in &order {
        let link = links[node];
        stall_depth = stall_depth.max(link.stall);
        counter_depth = counter_depth.max(link.counters);
        let (stall, counters) = match nodes[node] {
            Node::Repeat { body, min, max, .. } => (
                link.stall + u32::from(nullable[body]),
                link.counters + u32::from(counted(min, max)),
            ),
            _ => (link.stall, link.counters),
        };
        for operand in operands(&nodes[node]).into_iter().flatten() {
            links[operand].stall = stall;
            links[operand].counters = counters;
        }
    }
    drop((order, nullable));
    budget.release_compile_slots(count as u64 * 2);
    Ok(Links {
        nodes: links,
        backreferences,
        stall_depth,
        counter_depth,
    })
}

/// A program point: entering a node, leaving it, or beginning another
/// iteration of a repetition.
#[derive(Debug, Clone, Copy)]
enum Pc {
    Enter(usize),
    Exit(usize),
    Iterate(usize),
}

impl Pc {
    const fn code(self) -> u64 {
        match self {
            Self::Enter(node) => node as u64 * 3,
            Self::Exit(node) => node as u64 * 3 + 1,
            Self::Iterate(node) => node as u64 * 3 + 2,
        }
    }
}

/// One entry of the explicit exploration work list.
#[derive(Debug, Clone, Copy)]
enum Frame {
    /// A lower-priority alternative, explored after everything above it.
    Explore(Pc),
    /// Undo a counter or capture cell write when its alternative is exhausted.
    Restore { cell: usize, old: u64 },
    /// Undo a progress-level change.
    Level(u32),
}

/// Cells admitted per exploration frame.
const FRAME_CELLS: u128 = 3;

/// Cells admitted per hash table slot: its generation and key index.
const SLOT_CELLS: u128 = 2;

/// Grow `vec` to hold `needed` elements, admitting the whole prospective live
/// storage before the allocation. Capacity is retained, so it stays admitted.
fn reserve<T>(
    ctx: &mut Ctx<'_>,
    vec: &mut Vec<T>,
    needed: usize,
    cells: u128,
) -> Result<(), Error> {
    if needed <= vec.capacity() {
        return Ok(());
    }
    let old = vec.capacity() as u128 * cells;
    let capacity = needed.max(vec.capacity().saturating_mul(2)).max(4);
    let required = ctx.live_slots - old + capacity as u128 * cells;
    ctx.budget.limits().admit(Resource::MatchSlots, required)?;
    let units = u64::try_from(capacity as u128 * cells)
        .expect("admitted capacity fits its finite u64 slot bound");
    vec.try_reserve_exact(capacity - vec.len())
        .map_err(|_| Error::Allocation {
            resource: Resource::MatchSlots,
            units,
        })?;
    ctx.live_slots = ctx.live_slots - old + vec.capacity() as u128 * cells;
    Ok(())
}

/// The control states already reached at the current input position.
///
/// Small programs without counters index a dense stamp table by program point
/// and level; others hash the whole control state. Moving to the next position
/// advances a generation instead of clearing either table.
struct Visited {
    dense: bool,
    width: usize,
    stamps: Vec<u32>,
    table: Vec<(u32, usize)>,
    keys: Vec<u64>,
    key_len: usize,
    generation: u32,
    entries: usize,
}

/// The thread machine for one input and one request's limits.
pub(super) struct Pike<'a> {
    pub(super) ctx: Ctx<'a>,
    /// Counter cells at the front of a thread's state.
    counters: usize,
    /// Cells of one parked thread: its node, counters and capture boundaries.
    stride: usize,
    /// The state of the thread being explored: counters, then for each group
    /// its start and end boundaries.
    scratch: Vec<u64>,
    level: u32,
    current: Vec<u64>,
    next: Vec<u64>,
    stack: Vec<Frame>,
    visited: Visited,
    /// Capture boundaries of the best match found so far.
    found: Vec<u64>,
    matched: bool,
    ready: bool,
}

impl<'a> Pike<'a> {
    pub(super) fn new(ctx: Ctx<'a>) -> Self {
        let program = ctx.program;
        let counters = program.links.counter_depth as usize;
        let captures = (program.captures + 1) * 2;
        let width = program.links.stall_depth as usize + 1;
        let points = program.nodes.len().saturating_mul(3);
        let dense = counters == 0 && points.saturating_mul(width) <= DENSE_CELLS;
        Self {
            ctx,
            counters,
            stride: 1 + counters + captures,
            scratch: Vec::new(),
            level: CONSUMED,
            current: Vec::new(),
            next: Vec::new(),
            stack: Vec::new(),
            visited: Visited {
                dense,
                width,
                stamps: Vec::new(),
                table: Vec::new(),
                keys: Vec::new(),
                key_len: 2 + counters,
                generation: 0,
                entries: 0,
            },
            found: Vec::new(),
            matched: false,
            ready: false,
        }
    }

    /// Admit and allocate the storage every search needs.
    fn prepare(&mut self) -> Result<(), Error> {
        if self.ready {
            return Ok(());
        }
        let cells = self.stride - 1;
        reserve(&mut self.ctx, &mut self.scratch, cells, 1)?;
        self.scratch.resize(cells, 0);
        let captures = cells - self.counters;
        reserve(&mut self.ctx, &mut self.found, captures, 1)?;
        if self.visited.dense {
            let size = self.ctx.program.nodes.len() * 3 * self.visited.width;
            self.ctx
                .budget
                .charge_wide(Resource::MatchSteps, copy_steps(size))?;
            reserve(&mut self.ctx, &mut self.visited.stamps, size, 1)?;
            self.visited.stamps.resize(size, 0);
        } else {
            reserve(&mut self.ctx, &mut self.visited.table, 16, SLOT_CELLS)?;
            self.visited.table.resize(16, (0, 0));
        }
        self.ready = true;
        Ok(())
    }

    /// Forget the control states of the previous position.
    fn advance(&mut self) -> Result<(), Error> {
        let visited = &mut self.visited;
        visited.entries = 0;
        visited.keys.clear();
        if visited.generation == u32::MAX {
            let cells = visited.stamps.len() + visited.table.len();
            self.ctx
                .budget
                .charge_wide(Resource::MatchSteps, copy_steps(cells))?;
            visited.stamps.fill(0);
            visited.table.fill((0, 0));
            visited.generation = 1;
        } else {
            visited.generation += 1;
        }
        Ok(())
    }

    /// Record the control state at `point`, returning whether it is new.
    ///
    /// Each new state is admitted against the per-position state bound first.
    fn visit(&mut self, point: Pc, level: u32) -> Result<bool, Error> {
        let generation = self.visited.generation;
        if self.visited.dense {
            // One indexed probe, within the transition that reached `point`.
            let index = point.code() as usize * self.visited.width + level as usize;
            if self.visited.stamps[index] == generation {
                return Ok(false);
            }
            self.admit_state()?;
            self.visited.stamps[index] = generation;
            return Ok(true);
        }
        let key_len = self.visited.key_len;
        let start = self.visited.keys.len();
        reserve(&mut self.ctx, &mut self.visited.keys, start + key_len, 1)?;
        let keys = &mut self.visited.keys;
        keys.push(point.code());
        keys.push(u64::from(level));
        keys.extend_from_slice(&self.scratch[..self.counters]);
        self.ctx
            .budget
            .charge_wide(Resource::MatchSteps, 1 + copy_steps(key_len))?;
        let hash = purrdf_hash::fixed::hash_one(&self.visited.keys[start..]);
        let mask = self.visited.table.len() - 1;
        let mut slot = hash as usize & mask;
        loop {
            let (stamp, entry) = self.visited.table[slot];
            if stamp != generation {
                break;
            }
            let keys = &self.visited.keys;
            if keys[entry * key_len..(entry + 1) * key_len] == keys[start..] {
                self.visited.keys.truncate(start);
                return Ok(false);
            }
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            slot = (slot + 1) & mask;
        }
        self.admit_state()?;
        self.visited.table[slot] = (generation, self.visited.entries - 1);
        if self.visited.entries * 2 > self.visited.table.len() {
            self.rehash()?;
        }
        Ok(true)
    }

    fn admit_state(&mut self) -> Result<(), Error> {
        let entries = self.visited.entries + 1;
        self.ctx
            .budget
            .limits()
            .admit(Resource::MatchStates, entries as u128)?;
        self.visited.entries = entries;
        Ok(())
    }

    /// Double the hash table, admitting both tables while entries move.
    fn rehash(&mut self) -> Result<(), Error> {
        let old = self.visited.table.len();
        let size = old * 2;
        let required = self.ctx.live_slots + size as u128 * SLOT_CELLS;
        self.ctx
            .budget
            .limits()
            .admit(Resource::MatchSlots, required)?;
        let key_len = self.visited.key_len;
        self.ctx.budget.charge_wide(
            Resource::MatchSteps,
            size as u128 + (self.visited.entries * key_len) as u128,
        )?;
        let mut table = Vec::new();
        table
            .try_reserve_exact(size)
            .map_err(|_| Error::Allocation {
                resource: Resource::MatchSlots,
                units: size as u64 * 2,
            })?;
        table.resize(size, (0, 0));
        let generation = self.visited.generation;
        let mask = size - 1;
        for entry in 0..self.visited.entries {
            let key = &self.visited.keys[entry * key_len..(entry + 1) * key_len];
            let mut slot = purrdf_hash::fixed::hash_one(key) as usize & mask;
            while table[slot].0 == generation {
                slot = (slot + 1) & mask;
            }
            table[slot] = (generation, entry);
        }
        self.visited.table = table;
        self.ctx.live_slots = self.ctx.live_slots - old as u128 * SLOT_CELLS
            + self.visited.table.capacity() as u128 * SLOT_CELLS;
        Ok(())
    }

    fn frame(&mut self, frame: Frame) -> Result<(), Error> {
        let needed = self.stack.len() + 1;
        reserve(&mut self.ctx, &mut self.stack, needed, FRAME_CELLS)?;
        self.stack.push(frame);
        Ok(())
    }

    /// Write a state cell, leaving the record that undoes the write.
    fn set(&mut self, cell: usize, value: u64) -> Result<(), Error> {
        let old = self.scratch[cell];
        if old != value {
            self.frame(Frame::Restore { cell, old })?;
            self.scratch[cell] = value;
        }
        Ok(())
    }

    /// Park the explored thread at a character node, after every thread of
    /// higher priority.
    fn park(&mut self, node: usize, list: &mut Vec<u64>) -> Result<(), Error> {
        let needed = list.len() + self.stride;
        reserve(&mut self.ctx, list, needed, 1)?;
        self.ctx
            .budget
            .charge_wide(Resource::MatchSteps, copy_steps(self.stride))?;
        list.push(node as u64);
        list.extend_from_slice(&self.scratch);
        Ok(())
    }

    /// Record the explored thread as the best match so far.
    fn accept(&mut self, position: usize) -> Result<(), Error> {
        let captures = &self.scratch[self.counters..];
        self.ctx
            .budget
            .charge_wide(Resource::MatchSteps, copy_steps(captures.len()))?;
        self.found.clear();
        self.found.extend_from_slice(captures);
        self.found[1] = position as u64;
        self.matched = true;
        Ok(())
    }

    /// Explore every thread reachable from `pc` without consuming input,
    /// highest priority first, until one completes a match.
    fn closure(&mut self, pc: Pc, position: usize, list: &mut Vec<u64>) -> Result<bool, Error> {
        self.frame(Frame::Explore(pc))?;
        while let Some(frame) = self.stack.pop() {
            match frame {
                Frame::Explore(pc) => {
                    if self.explore(pc, position, list)? {
                        // Every remaining alternative has lower priority.
                        self.stack.clear();
                        return Ok(true);
                    }
                }
                Frame::Restore { cell, old } => self.scratch[cell] = old,
                Frame::Level(old) => self.level = old,
            }
        }
        Ok(false)
    }

    /// Follow one thread from `pc` until it parks, fails or matches, leaving
    /// its lower-priority alternatives on the work list.
    fn explore(&mut self, mut pc: Pc, position: usize, list: &mut Vec<u64>) -> Result<bool, Error> {
        let program = self.ctx.program;
        let links = &program.links.nodes;
        loop {
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            pc = match pc {
                Pc::Enter(node) => match program.nodes[node] {
                    Node::Empty => Pc::Exit(node),
                    Node::Start => {
                        if !self.ctx.at_start(position) {
                            return Ok(false);
                        }
                        Pc::Exit(node)
                    }
                    Node::End => {
                        if !self.ctx.at_end(position) {
                            return Ok(false);
                        }
                        Pc::Exit(node)
                    }
                    Node::Character(_) => {
                        // Consuming a character ends every empty iteration, so
                        // the progress level does not distinguish parked threads.
                        if self.visit(pc, 0)? {
                            self.park(node, list)?;
                        }
                        return Ok(false);
                    }
                    Node::Backreference(_) => {
                        unreachable!(
                            "a program with a backreference runs on the backtracking machine"
                        )
                    }
                    Node::Sequence(left, _) => Pc::Enter(left),
                    Node::Choice(left, right) => {
                        self.frame(Frame::Explore(Pc::Enter(right)))?;
                        Pc::Enter(left)
                    }
                    Node::Capture { number, body } => {
                        self.set(self.counters + 2 * number, position as u64)?;
                        Pc::Enter(body)
                    }
                    Node::Repeat { .. } => match self.check(node, 0, false)? {
                        Some(next) => next,
                        None => return Ok(false),
                    },
                },
                Pc::Iterate(node) => {
                    let Node::Repeat { body, .. } = program.nodes[node] else {
                        unreachable!("only a repetition iterates");
                    };
                    let depth = links[node].stall;
                    let inner = links[body].stall;
                    if inner > depth && self.level > depth {
                        // This nullable repetition's iteration begins here.
                        self.frame(Frame::Level(self.level))?;
                        self.level = depth;
                    }
                    if !self.visit(pc, self.level.min(inner))? {
                        return Ok(false);
                    }
                    Pc::Enter(body)
                }
                Pc::Exit(node) => {
                    match program.nodes[node] {
                        Node::Choice(..) => {
                            if !self.visit(pc, self.level.min(links[node].stall))? {
                                return Ok(false);
                            }
                        }
                        Node::Repeat { body, .. } => {
                            let link = links[node];
                            if links[body].counters > link.counters {
                                // Leaving the repetition clears its count.
                                self.set(link.counters as usize, 0)?;
                            }
                            if !self.visit(pc, self.level.min(link.stall))? {
                                return Ok(false);
                            }
                        }
                        _ => {}
                    }
                    let parent = links[node].parent;
                    if parent == ROOT {
                        self.accept(position)?;
                        return Ok(true);
                    }
                    match program.nodes[parent] {
                        Node::Sequence(left, right) => {
                            if node == left {
                                Pc::Enter(right)
                            } else {
                                Pc::Exit(parent)
                            }
                        }
                        Node::Choice(..) => Pc::Exit(parent),
                        Node::Capture { number, .. } => {
                            self.set(self.counters + 2 * number + 1, position as u64)?;
                            Pc::Exit(parent)
                        }
                        Node::Repeat { .. } => {
                            // This transition's spend precedes every count
                            // increment, even an empty one.
                            let link = links[parent];
                            let count = self.count(parent).checked_add(1).expect(
                                "finite fuel refuses before a repetition count can overflow",
                            );
                            let stalled =
                                links[node].stall > link.stall && self.level <= link.stall;
                            match self.check(parent, count, stalled)? {
                                Some(next) => next,
                                None => return Ok(false),
                            }
                        }
                        Node::Empty
                        | Node::Character(_)
                        | Node::Start
                        | Node::End
                        | Node::Backreference(_) => {
                            unreachable!("only a compound node has operands")
                        }
                    }
                }
            };
        }
    }

    /// The stored count of a repetition the explored thread is inside.
    fn count(&self, node: usize) -> u64 {
        let links = &self.ctx.program.links.nodes;
        let Node::Repeat { body, .. } = self.ctx.program.nodes[node] else {
            unreachable!("only a repetition has a count");
        };
        if links[body].counters > links[node].counters {
            self.scratch[links[node].counters as usize]
        } else {
            0
        }
    }

    /// Choose between another iteration and leaving a repetition that has
    /// completed `count` iterations, in its greedy or reluctant order.
    fn check(&mut self, node: usize, count: u64, stalled: bool) -> Result<Option<Pc>, Error> {
        let program = self.ctx.program;
        let Node::Repeat {
            body,
            min,
            max,
            greedy,
            ..
        } = program.nodes[node]
        else {
            unreachable!("only a repetition chooses its iterations");
        };
        let link = program.links.nodes[node];
        let (can_stop, can_repeat) = decide(min, max, count, stalled);
        if !can_repeat {
            return Ok(can_stop.then_some(Pc::Exit(node)));
        }
        if program.links.nodes[body].counters > link.counters {
            // Leaving instead clears the count again.
            self.set(link.counters as usize, stored(min, max, count))?;
        }
        if !can_stop {
            return Ok(Some(Pc::Iterate(node)));
        }
        Ok(Some(if greedy {
            self.frame(Frame::Explore(Pc::Exit(node)))?;
            Pc::Iterate(node)
        } else {
            self.frame(Frame::Explore(Pc::Iterate(node)))?;
            Pc::Exit(node)
        }))
    }

    /// Start a thread at `position`, after every thread already alive there.
    fn inject(&mut self, position: usize, list: &mut Vec<u64>) -> Result<(), Error> {
        let captures = self.stride - 1 - self.counters;
        self.ctx
            .budget
            .charge_wide(Resource::MatchSteps, copy_steps(captures))?;
        self.scratch[..self.counters].fill(0);
        self.scratch[self.counters..].fill(UNSET);
        self.scratch[self.counters] = position as u64;
        self.level = CONSUMED;
        self.closure(Pc::Enter(self.ctx.program.root), position, list)?;
        Ok(())
    }

    /// Advance every parked thread over `ch`, in priority order, into the
    /// list for the position `next`.
    fn step(
        &mut self,
        ch: char,
        next: usize,
        from: &[u64],
        into: &mut Vec<u64>,
    ) -> Result<(), Error> {
        let program = self.ctx.program;
        for record in from.chunks_exact(self.stride) {
            let node = record[0] as usize;
            let Node::Character(set) = program.nodes[node] else {
                unreachable!("threads park only at character nodes");
            };
            if !self.ctx.set_matches(set, ch)? {
                continue;
            }
            self.ctx
                .budget
                .charge_wide(Resource::MatchSteps, copy_steps(self.stride))?;
            self.scratch.copy_from_slice(&record[1..]);
            self.level = CONSUMED;
            if self.closure(Pc::Exit(node), next, into)? {
                // Every later thread has lower priority than this match.
                break;
            }
        }
        Ok(())
    }

    pub(super) fn find_from(&mut self, start: usize) -> Result<Option<Captures>, Error> {
        self.prepare()?;
        self.stack.clear();
        self.matched = false;
        let mut current = std::mem::take(&mut self.current);
        let mut next = std::mem::take(&mut self.next);
        current.clear();
        let outcome = self.search(start, &mut current, &mut next);
        // The lists keep their admitted capacity for a later search.
        current.clear();
        next.clear();
        self.current = current;
        self.next = next;
        outcome?;
        if !self.matched {
            return Ok(None);
        }
        self.captures()
    }

    /// Run every admissible start from `position` on, as one pass over the
    /// input, until the best match is final or no thread is left.
    fn search(
        &mut self,
        mut position: usize,
        current: &mut Vec<u64>,
        next: &mut Vec<u64>,
    ) -> Result<(), Error> {
        self.advance()?;
        let input = self.ctx.input;
        let run = self.ctx.program.lead.run;
        // Whether `position` lies in the leading run of an earlier start: a
        // match from there is a match from that start with a longer run, so
        // the earlier start's match always has priority.
        let mut covered = false;
        loop {
            let mut tracked = covered;
            if !self.matched {
                if current.is_empty() {
                    // Nothing is alive: skip every start that cannot match.
                    let Some(skip) = self.ctx.candidate(position)? else {
                        return Ok(());
                    };
                    if skip != position {
                        position = skip;
                        tracked = false;
                        self.advance()?;
                    }
                    if !tracked {
                        self.inject(position, current)?;
                        tracked = true;
                    }
                } else if !covered && self.ctx.may_start(position)? {
                    self.inject(position, current)?;
                    tracked = true;
                }
            }
            let Some(ch) = input[position..].chars().next() else {
                return Ok(());
            };
            if current.is_empty() && self.matched {
                return Ok(());
            }
            covered = match run {
                Some(run) if tracked && !self.matched => self.ctx.set_matches(run, ch)?,
                _ => false,
            };
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            self.advance()?;
            let after = position + ch.len_utf8();
            next.clear();
            self.step(ch, after, current, next)?;
            std::mem::swap(current, next);
            position = after;
        }
    }

    fn captures(&self) -> Result<Option<Captures>, Error> {
        let groups = self.found.len() / 2;
        self.ctx.budget.limits().admit(
            Resource::MatchSlots,
            self.ctx.live_slots + groups as u128 * 2,
        )?;
        let mut spans = Vec::new();
        spans
            .try_reserve_exact(groups)
            .map_err(|_| Error::Allocation {
                resource: Resource::MatchSlots,
                units: groups as u64 * 2,
            })?;
        spans.extend(self.found.as_chunks::<2>().0.iter().map(|&[start, end]| {
            (end != UNSET).then(|| {
                debug_assert_ne!(start, UNSET, "a closed group was opened");
                start as usize..end as usize
            })
        }));
        Ok(Some(Captures { spans }))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::super::{Limits, Profile, compile};
    use super::{ROOT, operands};

    #[test]
    fn only_counts_that_decide_a_later_choice_are_stored() {
        for (source, depth) in [
            ("a?", 0),
            ("a*", 0),
            ("a+", 0),
            ("a{1}", 0),
            ("a{0,1}", 0),
            ("a{1,}", 0),
            ("a{2}", 1),
            ("a{2,}", 1),
            ("a{0,2}", 1),
            ("(a{2})*", 1),
            ("(a{2}){3}", 2),
            ("(a{2})|(b{3}){4}", 2),
            ("a{18446744073709551616,}", 1),
        ] {
            let program = compile(Profile::Xpath31, source, "", Limits::new()).unwrap();
            assert_eq!(program.links.counter_depth, depth, "{source}");
        }
    }

    #[test]
    fn progress_levels_count_nested_nullable_repetitions() {
        for (source, depth) in [
            ("a*", 0),
            ("(a+)*", 0),
            ("(a*)*", 1),
            ("(a|)*", 1),
            ("(^)*", 1),
            ("((a*)*)*", 2),
            ("((a?)+)+", 2),
            ("(a*)*(b*)*", 1),
        ] {
            let program = compile(Profile::Xpath31, source, "", Limits::new()).unwrap();
            assert_eq!(program.links.stall_depth, depth, "{source}");
        }
    }

    #[test]
    fn every_operand_names_its_parent_and_only_the_root_has_none() {
        for source in ["", "a", "(a|b)*c", "^(?:x(y)?|z{2,3}?)+$", r"((a)\2)|b"] {
            let program = compile(Profile::Xpath31, source, "", Limits::new()).unwrap();
            let links = &program.links.nodes;
            assert_eq!(links.len(), program.nodes.len());
            for (node, kind) in program.nodes.iter().enumerate() {
                for operand in operands(kind).into_iter().flatten() {
                    assert_eq!(links[operand].parent, node, "{source}");
                }
            }
            let roots: Vec<usize> = (0..links.len())
                .filter(|&node| links[node].parent == ROOT)
                .collect();
            assert_eq!(roots, [program.root], "{source}");
            assert_eq!(program.links.backreferences, source.contains('\\'));
        }
    }
}
