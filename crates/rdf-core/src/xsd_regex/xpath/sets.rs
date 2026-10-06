// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Capture-free matching for programs without backreferences, over sets of
//! repetition counts.
//!
//! Whether a match exists, and where matches start, depend on no priority
//! and no capture: only on which control states are reachable. The thread
//! machine keeps one thread per distinct control state, so a counted
//! repetition entered from many search starts keeps one thread per distinct
//! count. This machine keeps one entry per program point, progress level and
//! value of every enclosing count but one, and holds that one repetition's
//! counts as a single ordered set of disjoint intervals: the repetition whose
//! body can match the empty string, or else the one with the most
//! distinguishable counts. Every count of the set moves through the program
//! together: entering the repetition is the singleton zero, completing an
//! iteration shifts the whole set, and its bounds split it into the counts
//! that may stop and the counts that may iterate again. An empty iteration
//! below the minimum fills the run of counts up to it at once, so even a
//! minimum above `u64` is finite work. The progress levels are the thread
//! machine's, so every transition is one the backtracking machine can take.
//!
//! Between positions, the counts of a parked entry are reduced to those whose
//! futures differ. A count `c` of a repetition `{m,M}`, in an iteration that
//! has consumed a character, may complete between `max(m - c - 1, 0)` and
//! `M - c - 1` further iterations. Two counts at most `M - m + 1` apart
//! allow every number of further iterations the counts between them allow,
//! so the gap is filled; every count from `m - 1` on allows a superset of
//! what each larger one allows, so only the least is kept; and without a
//! finite maximum only the largest count matters. A set therefore holds at
//! most about `m / (M - m + 2) + 2` intervals, whatever the number of live
//! counts: one for `{0,M}`, `{1,M}` and `{m,}`, and at most two for every
//! window at least as wide as its minimum.
//!
//! The work for one input position is the program points each entry reaches
//! times the intervals of its set, never a count, with two exceptions that
//! need nested counted repetitions. Entering a counted repetition that takes
//! the set from an enclosing one enumerates the enclosing counts into keys,
//! at most that repetition's distinguishable counts, which are no more than
//! the nested one's; and an empty iteration of a key-held count whose body
//! matches the empty string only beside an anchor fills the counts up to its
//! minimum one entry each. The parked entries after a position are its
//! state, and states are interned in a bounded cache with their transitions,
//! so a position whose state, character and anchors were seen before costs
//! one step. The same machine runs backwards over the reversed program to
//! mark every position at which a match starts.

use std::cmp::Ordering;
use std::ops::Range;

use super::compile::{Count, Node};
use super::r#match::{Ctx, decide};
use super::pike::{CONSUMED, Pc, ROOT, copy_steps, reserve};
use super::{Error, Resource};

/// The stored count of a repetition without a finite maximum whose minimum
/// exceeds `u64`, once an empty iteration has met that minimum.
const TOP: u64 = u64::MAX;

/// Cells admitted per work item and per entry of a position's table.
const ITEM_CELLS: u128 = 4;
const ENTRY_CELLS: u128 = 5;

/// Cells admitted per interval and per hash-table slot.
const INTERVAL_CELLS: u128 = 2;
const SLOT_CELLS: u128 = 1;

/// Cells the state cache may retain before it is cleared.
const CACHE_CELLS: usize = 1 << 17;

/// A run of counts in an arena.
#[derive(Debug, Clone, Copy)]
struct Span {
    start: usize,
    len: usize,
}

impl Span {
    const fn range(self) -> Range<usize> {
        self.start..self.start + self.len
    }
}

/// A delta of control states still to be explored at the current position.
#[derive(Debug, Clone, Copy)]
struct Item {
    pc: Pc,
    level: u32,
    /// The offset of the entry's key in the key arena.
    key: usize,
    set: Span,
}

/// The accumulated counts of one control state at the current position.
#[derive(Debug, Clone, Copy)]
struct Entry {
    /// The offset of the program point, level and key in the key arena.
    probe: usize,
    set: Span,
}

/// The repetition bounds that decide a stored count.
#[derive(Debug, Clone, Copy)]
struct Bounds {
    min: Count,
    max: Option<Count>,
}

impl Bounds {
    const fn finite_max(self) -> Option<u64> {
        match self.max {
            Some(Count::Finite(max)) => Some(max),
            _ => None,
        }
    }

    /// The largest stored count: a finite maximum, or below the minimum,
    /// where every larger count makes the same choices.
    const fn cap(self) -> u64 {
        match (self.finite_max(), self.min) {
            (Some(max), _) => max,
            (None, Count::Finite(min)) => min.saturating_sub(1),
            (None, Count::AboveU64) => TOP,
        }
    }

    /// One more completed iteration than the stored `count`.
    const fn increment(self, count: u64) -> u64 {
        if count == TOP && self.finite_max().is_none() {
            TOP
        } else {
            count + 1
        }
    }

    /// The least completed count that may stop, if a stored count reaches it.
    const fn stop_floor(self) -> u64 {
        match self.min {
            Count::Finite(min) => min,
            Count::AboveU64 => TOP,
        }
    }

    /// The largest count that may begin another iteration.
    const fn repeat_ceiling(self) -> u64 {
        match self.finite_max() {
            Some(max) => max.saturating_sub(1),
            None => TOP,
        }
    }

    /// The counts an empty iteration below the minimum reaches without
    /// stopping: every count from one more than `count` to below the minimum.
    fn fill(self, count: u64) -> Option<(u64, u64)> {
        let low = self.increment(count);
        let high = match self.min {
            Count::Finite(0) => return None,
            Count::Finite(min) => min - 1,
            Count::AboveU64 => TOP,
        };
        (low <= high).then_some((low, high))
    }
}

/// The direction of a scan over the input and the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Forward,
    Reverse,
}

/// Interned states and their transitions, cleared when full.
#[derive(Debug, Default)]
struct Cache {
    /// Every state's encoding, back to back.
    contents: Vec<u64>,
    /// Each state's span in `contents`.
    spans: Vec<(usize, usize)>,
    /// Open addressing over state ids, `u32::MAX` empty.
    index: Vec<u32>,
    /// Open addressing over transitions: the key, then the next state.
    moves: Vec<(u64, u32)>,
    moves_held: usize,
}

const NO_MOVE: u64 = u64::MAX;

impl Cache {
    fn cells(&self) -> usize {
        self.contents.capacity()
            + self.spans.capacity() * 2
            + self.index.capacity()
            + self.moves.capacity() * 2
    }

    fn state(&self, id: u32) -> &[u64] {
        let (start, len) = self.spans[id as usize];
        &self.contents[start..start + len]
    }

    fn lookup(&self, key: u64) -> Option<u32> {
        if self.moves.is_empty() {
            return None;
        }
        let mask = self.moves.len() - 1;
        let mut slot = purrdf_hash::fixed::hash_one(&key) as usize & mask;
        loop {
            let (stored, next) = self.moves[slot];
            if stored == NO_MOVE {
                return None;
            }
            if stored == key {
                return Some(next);
            }
            slot = (slot + 1) & mask;
        }
    }
}

/// The capture-free machine and the match starts it has marked.
#[derive(Debug)]
pub(super) struct Sets {
    direction: Direction,
    /// Program points, levels and keys of entries and items.
    keys: Vec<u64>,
    /// Intervals of counts.
    intervals: Vec<(u64, u64)>,
    work: Vec<Item>,
    entries: Vec<Entry>,
    /// Open addressing over the current position's entries: generation, entry.
    table: Vec<(u32, u32)>,
    generation: u32,
    /// The entries parked at character nodes at the current position.
    parked: Vec<u32>,
    accepted: bool,
    /// Whether an accepting entry ends the closure at once.
    eager: bool,
    /// The current state's encoding, when it is not only in the cache.
    state: Vec<u64>,
    fresh: bool,
    next: Vec<u64>,
    order: Vec<u32>,
    cache: Cache,
    /// How many times the cache has been cleared during this scan.
    epoch: u64,
    /// Positions at which a match starts, marked by a reverse scan down to
    /// `floor`.
    starts: Vec<u64>,
    floor: Option<usize>,
}

impl Sets {
    pub(super) const fn new() -> Self {
        Self {
            direction: Direction::Forward,
            keys: Vec::new(),
            intervals: Vec::new(),
            work: Vec::new(),
            entries: Vec::new(),
            table: Vec::new(),
            generation: 0,
            parked: Vec::new(),
            accepted: false,
            eager: false,
            state: Vec::new(),
            fresh: false,
            next: Vec::new(),
            order: Vec::new(),
            cache: Cache {
                contents: Vec::new(),
                spans: Vec::new(),
                index: Vec::new(),
                moves: Vec::new(),
                moves_held: 0,
            },
            epoch: 0,
            starts: Vec::new(),
            floor: None,
        }
    }

    /// Whether a match starts at or after `start`.
    pub(super) fn is_match(&mut self, ctx: &mut Ctx<'_>, start: usize) -> Result<bool, Error> {
        let found = self.forward(ctx, start);
        self.release(ctx);
        found
    }

    /// The first position at or after `start` at which a match starts.
    ///
    /// The first call marks every match start from `start` to the end of the
    /// input in one reverse scan; later calls of the same execution read the
    /// marks.
    pub(super) fn first_start(
        &mut self,
        ctx: &mut Ctx<'_>,
        start: usize,
    ) -> Result<Option<usize>, Error> {
        if self.floor.is_none_or(|floor| start < floor) {
            let marked = self.reverse(ctx, start);
            self.release(ctx);
            marked?;
            self.floor = Some(start);
        }
        let input = ctx.input.len();
        let mut word = start / 64;
        let mut bits = self.starts[word] & (u64::MAX << (start % 64));
        loop {
            ctx.budget.charge(Resource::MatchSteps, 1)?;
            if bits != 0 {
                let position = word * 64 + bits.trailing_zeros() as usize;
                return Ok((position <= input).then_some(position));
            }
            word += 1;
            let Some(&next) = self.starts.get(word) else {
                return Ok(None);
            };
            bits = next;
        }
    }

    /// Release every scan buffer and the cache; the match starts remain.
    fn release(&mut self, ctx: &mut Ctx<'_>) {
        let cells = self.keys.capacity() as u128
            + self.intervals.capacity() as u128 * INTERVAL_CELLS
            + self.work.capacity() as u128 * ITEM_CELLS
            + self.entries.capacity() as u128 * ENTRY_CELLS
            + self.table.capacity() as u128 * SLOT_CELLS
            + self.parked.capacity() as u128
            + self.state.capacity() as u128
            + self.next.capacity() as u128
            + self.order.capacity() as u128
            + self.cache.cells() as u128;
        ctx.live_slots -= cells;
        let starts = std::mem::take(&mut self.starts);
        let floor = self.floor;
        *self = Self::new();
        self.starts = starts;
        self.floor = floor;
    }

    /// Scan forwards from `start` until a match ends or no start remains.
    fn forward(&mut self, ctx: &mut Ctx<'_>, start: usize) -> Result<bool, Error> {
        self.direction = Direction::Forward;
        self.eager = true;
        let input = ctx.input;
        let Some(mut position) = ctx.candidate(start)? else {
            return Ok(false);
        };
        self.begin(ctx)?;
        self.inject(ctx)?;
        self.closure(ctx, position)?;
        self.settle(ctx)?;
        let mut current = self.intern(ctx)?;
        loop {
            if self.accepting(current) {
                return Ok(true);
            }
            let Some(ch) = input[position..].chars().next() else {
                return Ok(false);
            };
            let after = position + ch.len_utf8();
            if self.parked_none(current) {
                // Nothing is alive: only a later start can match.
                let Some(next) = ctx.candidate(after)? else {
                    return Ok(false);
                };
                position = next;
                self.begin(ctx)?;
                self.inject(ctx)?;
                self.closure(ctx, position)?;
                self.settle(ctx)?;
                current = self.intern(ctx)?;
                continue;
            }
            current = self.transition(ctx, current, ch, after)?;
            position = after;
        }
    }

    /// Scan backwards from the end of the input to `floor`, marking every
    /// position at which a match starts.
    fn reverse(&mut self, ctx: &mut Ctx<'_>, floor: usize) -> Result<(), Error> {
        self.direction = Direction::Reverse;
        self.eager = false;
        let input = ctx.input;
        let words = input.len() / 64 + 1;
        ctx.budget
            .charge_wide(Resource::MatchSteps, copy_steps(words))?;
        let old = self.starts.capacity() as u128;
        ctx.live_slots -= old;
        self.starts = Vec::new();
        reserve(ctx, &mut self.starts, words, 1)?;
        self.starts.resize(words, 0);
        let mut position = input.len();
        self.begin(ctx)?;
        self.inject(ctx)?;
        self.closure(ctx, position)?;
        self.settle(ctx)?;
        let mut current = self.intern(ctx)?;
        loop {
            if self.accepting(current) {
                self.starts[position / 64] |= 1 << (position % 64);
            }
            if position <= floor {
                return Ok(());
            }
            let ch = input[..position]
                .chars()
                .next_back()
                .expect("a position above the floor follows a character");
            let before = position - ch.len_utf8();
            current = self.transition(ctx, current, ch, before)?;
            position = before;
        }
    }

    /// The state after `current` consumes `ch` and a match may begin again at
    /// `position`, from the cache when it holds the move.
    fn transition(
        &mut self,
        ctx: &mut Ctx<'_>,
        current: u32,
        ch: char,
        position: usize,
    ) -> Result<u32, Error> {
        let anchors = if ctx.program.links.anchors {
            u64::from(ctx.at_start(position)) | u64::from(ctx.at_end(position)) << 1
        } else {
            0
        };
        let key = u64::from(current) << 24 | u64::from(ch) << 2 | anchors;
        if let Some(next) = self.cache.lookup(key) {
            ctx.budget.charge(Resource::MatchSteps, 1)?;
            self.fresh = false;
            return Ok(next);
        }
        if !self.fresh {
            // The current state is only in the cache: copy its encoding out.
            let (start, len) = self.cache.spans[current as usize];
            ctx.budget
                .charge_wide(Resource::MatchSteps, copy_steps(len))?;
            reserve(ctx, &mut self.state, len, 1)?;
            self.state.clear();
            self.state
                .extend_from_slice(&self.cache.contents[start..start + len]);
        }
        self.begin(ctx)?;
        self.seed(ctx, ch)?;
        self.inject(ctx)?;
        self.closure(ctx, position)?;
        self.settle(ctx)?;
        let epoch = self.epoch;
        let next = self.intern(ctx)?;
        if epoch == self.epoch {
            self.remember(ctx, key, next)?;
        }
        Ok(next)
    }

    fn accepting(&self, current: u32) -> bool {
        self.cache.state(current)[0] != 0
    }

    fn parked_none(&self, current: u32) -> bool {
        self.cache.state(current)[1] == 0
    }

    /// Forget the previous position's entries and work.
    fn begin(&mut self, ctx: &mut Ctx<'_>) -> Result<(), Error> {
        self.keys.clear();
        self.intervals.clear();
        self.work.clear();
        self.entries.clear();
        self.parked.clear();
        self.accepted = false;
        if self.table.is_empty() {
            reserve(ctx, &mut self.table, 16, SLOT_CELLS)?;
            self.table.resize(16, (0, 0));
        }
        if self.generation == u32::MAX {
            ctx.budget
                .charge_wide(Resource::MatchSteps, copy_steps(self.table.len()))?;
            self.table.fill((0, 0));
            self.generation = 1;
        } else {
            self.generation += 1;
        }
        Ok(())
    }

    /// Append the singleton count zero, which also stands for "present" at a
    /// point no counted repetition encloses.
    fn zero(&mut self, ctx: &mut Ctx<'_>) -> Result<Span, Error> {
        self.interval(ctx, 0, 0)?;
        Ok(Span {
            start: self.intervals.len() - 1,
            len: 1,
        })
    }

    fn interval(&mut self, ctx: &mut Ctx<'_>, low: u64, high: u64) -> Result<(), Error> {
        let needed = self.intervals.len() + 1;
        reserve(ctx, &mut self.intervals, needed, INTERVAL_CELLS)?;
        self.intervals.push((low, high));
        Ok(())
    }

    fn push(&mut self, ctx: &mut Ctx<'_>, item: Item) -> Result<(), Error> {
        let needed = self.work.len() + 1;
        reserve(ctx, &mut self.work, needed, ITEM_CELLS)?;
        self.work.push(item);
        Ok(())
    }

    /// Begin a match at the current position.
    fn inject(&mut self, ctx: &mut Ctx<'_>) -> Result<(), Error> {
        let set = self.zero(ctx)?;
        let key = self.keys.len();
        self.push(
            ctx,
            Item {
                pc: Pc::Enter(ctx.program.root),
                level: CONSUMED,
                key,
                set,
            },
        )
    }

    /// Advance every parked entry of the current state over `ch`.
    fn seed(&mut self, ctx: &mut Ctx<'_>, ch: char) -> Result<(), Error> {
        let program = ctx.program;
        let mut at = 2;
        for _ in 0..self.state[1] {
            let node = self.state[at] as usize;
            let key_len = key_len(program.links.nodes[node].counters);
            let key = at + 1;
            let count = self.state[key + key_len] as usize;
            let intervals = key + key_len + 1;
            at = intervals + count * 2;
            let Node::Character(set) = program.nodes[node] else {
                unreachable!("entries park only at character nodes");
            };
            if !ctx.set_matches(set, ch)? {
                continue;
            }
            ctx.budget
                .charge_wide(Resource::MatchSteps, copy_steps(key_len + count * 2))?;
            let start = self.keys.len();
            reserve(ctx, &mut self.keys, start + key_len, 1)?;
            self.keys.extend_from_slice(&self.state[key..key + key_len]);
            let first = self.intervals.len();
            reserve(ctx, &mut self.intervals, first + count, INTERVAL_CELLS)?;
            for pair in self.state[intervals..at].as_chunks::<2>().0 {
                self.intervals.push(<(u64, u64)>::from(*pair));
            }
            self.push(
                ctx,
                Item {
                    pc: Pc::Exit(node),
                    level: CONSUMED,
                    key: start,
                    set: Span {
                        start: first,
                        len: count,
                    },
                },
            )?;
        }
        Ok(())
    }

    /// Explore every control state reachable at `position` without
    /// consuming input.
    fn closure(&mut self, ctx: &mut Ctx<'_>, position: usize) -> Result<(), Error> {
        let at_start = ctx.at_start(position);
        let at_end = ctx.at_end(position);
        while let Some(item) = self.work.pop() {
            ctx.budget
                .charge_wide(Resource::MatchSteps, 1 + item.set.len as u128)?;
            self.process(ctx, item, at_start, at_end)?;
            if self.accepted && self.eager {
                self.work.clear();
                return Ok(());
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn process(
        &mut self,
        ctx: &mut Ctx<'_>,
        item: Item,
        at_start: bool,
        at_end: bool,
    ) -> Result<(), Error> {
        let program = ctx.program;
        let links = &program.links.nodes;
        let reverse = self.direction == Direction::Reverse;
        let next = |pc| Item { pc, ..item };
        match item.pc {
            Pc::Enter(node) => match program.nodes[node] {
                Node::Empty => self.push(ctx, next(Pc::Exit(node))),
                Node::Start => {
                    if at_start {
                        self.push(ctx, next(Pc::Exit(node)))?;
                    }
                    Ok(())
                }
                Node::End => {
                    if at_end {
                        self.push(ctx, next(Pc::Exit(node)))?;
                    }
                    Ok(())
                }
                Node::Character(_) => self.merge(ctx, item.pc, 0, item.key, item.set),
                Node::Backreference(_) => {
                    unreachable!("a program with a backreference runs on the backtracking machine")
                }
                Node::Sequence(left, right) => {
                    self.push(ctx, next(Pc::Enter(if reverse { right } else { left })))
                }
                Node::Choice(left, right) => {
                    self.push(ctx, next(Pc::Enter(right)))?;
                    self.push(ctx, next(Pc::Enter(left)))
                }
                Node::Capture { body, .. } => self.push(ctx, next(Pc::Enter(body))),
                Node::Repeat { body, min, max, .. } => {
                    let (can_stop, can_repeat) = decide(min, max, 0, false);
                    if can_stop {
                        self.merge(ctx, Pc::Exit(node), item.level, item.key, item.set)?;
                    }
                    if can_repeat {
                        if links[body].counters > links[node].counters {
                            self.enter_count(ctx, node, item)?;
                        } else {
                            self.merge(ctx, Pc::Iterate(node), item.level, item.key, item.set)?;
                        }
                    }
                    Ok(())
                }
            },
            Pc::Iterate(node) => {
                let Node::Repeat { body, .. } = program.nodes[node] else {
                    unreachable!("only a repetition iterates");
                };
                self.push(ctx, next(Pc::Enter(body)))
            }
            Pc::Exit(node) => {
                let parent = links[node].parent;
                if parent == ROOT {
                    self.accepted = true;
                    return Ok(());
                }
                match program.nodes[parent] {
                    Node::Sequence(left, right) => {
                        let (first, second) = if reverse {
                            (right, left)
                        } else {
                            (left, right)
                        };
                        if node == first {
                            self.push(ctx, next(Pc::Enter(second)))
                        } else {
                            self.push(ctx, next(Pc::Exit(parent)))
                        }
                    }
                    Node::Choice(..) => {
                        self.merge(ctx, Pc::Exit(parent), item.level, item.key, item.set)
                    }
                    Node::Capture { .. } => self.push(ctx, next(Pc::Exit(parent))),
                    Node::Repeat { min, max, .. } => {
                        let link = links[parent];
                        let stalled = links[node].stall > link.stall && item.level <= link.stall;
                        if links[node].counters > link.counters {
                            return self.complete(ctx, parent, item, stalled);
                        }
                        let (can_stop, can_repeat) = decide(min, max, 1, stalled);
                        if can_stop {
                            self.merge(ctx, Pc::Exit(parent), item.level, item.key, item.set)?;
                        }
                        if can_repeat {
                            self.merge(ctx, Pc::Iterate(parent), item.level, item.key, item.set)?;
                        }
                        Ok(())
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
        }
    }

    /// Enter the counted repetition `repeat` with the count zero.
    fn enter_count(&mut self, ctx: &mut Ctx<'_>, repeat: usize, item: Item) -> Result<(), Error> {
        let program = ctx.program;
        let links = &program.links.nodes;
        let Node::Repeat { body, .. } = program.nodes[repeat] else {
            unreachable!("only a repetition is counted");
        };
        let depth = links[repeat].counters as usize;
        let pc = Pc::Iterate(repeat);
        if depth == 0 {
            let key = self.keys.len();
            let set = self.zero(ctx)?;
            return self.merge(ctx, pc, item.level, key, set);
        }
        let outer = links[links[repeat].set].counters as usize;
        if links[body].set != repeat {
            // The enclosing set stays the set; this count joins the key.
            let key = self.rekey(ctx, item.key, depth - 1, Rekey::Append(0))?;
            return self.merge(ctx, pc, item.level, key, item.set);
        }
        // This count becomes the set: each enclosing count joins the key.
        for index in item.set.range() {
            let (low, high) = self.intervals[index];
            ctx.budget
                .charge_wide(Resource::MatchSteps, u128::from(high - low) + 1)?;
            for value in low..=high {
                let key = self.rekey(ctx, item.key, depth - 1, Rekey::Insert(outer, value))?;
                let set = self.zero(ctx)?;
                self.merge(ctx, pc, item.level, key, set)?;
            }
        }
        Ok(())
    }

    /// Complete an iteration of the counted repetition `repeat`, whose body
    /// has just been left.
    fn complete(
        &mut self,
        ctx: &mut Ctx<'_>,
        repeat: usize,
        item: Item,
        stalled: bool,
    ) -> Result<(), Error> {
        let program = ctx.program;
        let links = &program.links.nodes;
        let Node::Repeat { body, min, max, .. } = program.nodes[repeat] else {
            unreachable!("only a repetition is counted");
        };
        let bounds = Bounds { min, max };
        let depth = links[repeat].counters as usize;
        if links[body].set == repeat {
            let set = item.set;
            if stalled {
                // The iteration was empty: further empty iterations reach
                // every count up to the minimum, where the repetition stops.
                self.leave(ctx, repeat, item.level, item.key, item.set)?;
                let least = self.intervals[set.start].0;
                if let Some((low, high)) = bounds.fill(least) {
                    self.interval(ctx, low, high)?;
                    let filled = Span {
                        start: self.intervals.len() - 1,
                        len: 1,
                    };
                    self.merge(ctx, Pc::Iterate(repeat), item.level, item.key, filled)?;
                }
                return Ok(());
            }
            let shifted = self.shift(ctx, set, bounds)?;
            let floor = bounds.stop_floor();
            if self.intervals[shifted.range()]
                .last()
                .is_some_and(|&(_, high)| high >= floor)
            {
                self.leave(ctx, repeat, item.level, item.key, item.set)?;
            }
            let repeated = self.clip(ctx, shifted, bounds.repeat_ceiling(), bounds.cap())?;
            if repeated.len > 0 {
                self.merge(ctx, Pc::Iterate(repeat), item.level, item.key, repeated)?;
            }
            return Ok(());
        }
        // This count is the last value of the key.
        let slot = item.key + depth - 1;
        let count = bounds.increment(self.keys[slot]);
        if stalled {
            self.leave(ctx, repeat, item.level, item.key, item.set)?;
            if links[body].empty {
                // A lower count, already explored, allows everything a higher
                // count reached by empty iterations allows.
                return Ok(());
            }
            if let Some((low, high)) = bounds.fill(self.keys[slot]) {
                ctx.budget
                    .charge_wide(Resource::MatchSteps, u128::from(high - low) + 1)?;
                for value in low..=high {
                    let key = self.rekey(ctx, item.key, depth, Rekey::Replace(depth - 1, value))?;
                    self.merge(ctx, Pc::Iterate(repeat), item.level, key, item.set)?;
                }
            }
            return Ok(());
        }
        if count >= bounds.stop_floor() {
            self.leave(ctx, repeat, item.level, item.key, item.set)?;
        }
        if count <= bounds.repeat_ceiling() {
            let key = self.rekey(
                ctx,
                item.key,
                depth,
                Rekey::Replace(depth - 1, count.min(bounds.cap())),
            )?;
            self.merge(ctx, Pc::Iterate(repeat), item.level, key, item.set)?;
        }
        Ok(())
    }

    /// Leave the counted repetition `repeat` from inside its body, dropping
    /// its count from an entry whose key starts at `key` and whose set is
    /// `set`.
    fn leave(
        &mut self,
        ctx: &mut Ctx<'_>,
        repeat: usize,
        level: u32,
        key: usize,
        set: Span,
    ) -> Result<(), Error> {
        let program = ctx.program;
        let links = &program.links.nodes;
        let Node::Repeat { body, .. } = program.nodes[repeat] else {
            unreachable!("only a repetition is counted");
        };
        let depth = links[repeat].counters as usize;
        let pc = Pc::Exit(repeat);
        if links[body].set != repeat {
            // The count was the key's last value; the shorter key is a prefix
            // of the same cells, and the set is unchanged.
            return self.merge(ctx, pc, level, key, set);
        }
        if depth == 0 {
            let empty = self.keys.len();
            let unit = self.zero(ctx)?;
            return self.merge(ctx, pc, level, empty, unit);
        }
        // The enclosing set's count leaves the key and becomes the set again.
        let outer = links[links[repeat].set].counters as usize;
        let value = self.keys[key + outer];
        let shorter = self.rekey(ctx, key, depth, Rekey::Remove(outer))?;
        self.interval(ctx, value, value)?;
        let single = Span {
            start: self.intervals.len() - 1,
            len: 1,
        };
        self.merge(ctx, pc, level, shorter, single)
    }

    /// Write a changed copy of the `len` key values at `from`, returning its
    /// offset.
    fn rekey(
        &mut self,
        ctx: &mut Ctx<'_>,
        from: usize,
        len: usize,
        change: Rekey,
    ) -> Result<usize, Error> {
        let start = self.keys.len();
        reserve(ctx, &mut self.keys, start + len + 1, 1)?;
        ctx.budget
            .charge_wide(Resource::MatchSteps, copy_steps(len + 1))?;
        match change {
            Rekey::Append(value) => {
                self.keys.extend_from_within(from..from + len);
                self.keys.push(value);
            }
            Rekey::Insert(index, value) => {
                self.keys.extend_from_within(from..from + index);
                self.keys.push(value);
                self.keys.extend_from_within(from + index..from + len);
            }
            Rekey::Replace(index, value) => {
                self.keys.extend_from_within(from..from + len);
                self.keys[start + index] = value;
            }
            Rekey::Remove(index) => {
                self.keys.extend_from_within(from..from + index);
                self.keys.extend_from_within(from + index + 1..from + len);
            }
        }
        Ok(start)
    }

    /// Add `set` to the control state `pc` at `level` with the key at `key`,
    /// exploring only the counts it did not already hold.
    fn merge(
        &mut self,
        ctx: &mut Ctx<'_>,
        pc: Pc,
        level: u32,
        key: usize,
        set: Span,
    ) -> Result<(), Error> {
        let program = ctx.program;
        let links = &program.links.nodes;
        let (level, depth) = match pc {
            Pc::Iterate(repeat) => {
                let Node::Repeat { body, .. } = program.nodes[repeat] else {
                    unreachable!("only a repetition iterates");
                };
                let (depth, inner) = (links[repeat].stall, links[body].stall);
                // This nullable repetition's iteration begins here.
                let level = if inner > depth && level > depth {
                    depth
                } else {
                    level
                };
                (level.min(inner), links[body].counters)
            }
            Pc::Exit(node) => (level.min(links[node].stall), links[node].counters),
            // Consuming a character ends every empty iteration.
            Pc::Enter(node) => (0, links[node].counters),
        };
        let len = key_len(depth);
        let probe = self.keys.len();
        reserve(ctx, &mut self.keys, probe + 2 + len, 1)?;
        self.keys.push(pc.code());
        self.keys.push(u64::from(level));
        self.keys.extend_from_within(key..key + len);
        ctx.budget
            .charge_wide(Resource::MatchSteps, 1 + copy_steps(len + 2))?;
        let hash = purrdf_hash::fixed::hash_one(&self.keys[probe..]);
        let mask = self.table.len() - 1;
        let mut slot = hash as usize & mask;
        loop {
            let (stamp, index) = self.table[slot];
            if stamp != self.generation {
                break;
            }
            let entry = self.entries[index as usize];
            if self.keys[entry.probe..entry.probe + 2 + len] == self.keys[probe..] {
                self.keys.truncate(probe);
                let delta = self.difference(ctx, set, entry.set)?;
                if delta.len == 0 {
                    return Ok(());
                }
                let union = self.union(ctx, entry.set, delta)?;
                self.entries[index as usize].set = union;
                if matches!(pc, Pc::Enter(_)) {
                    return Ok(());
                }
                return self.push(
                    ctx,
                    Item {
                        pc,
                        level,
                        key: entry.probe + 2,
                        set: delta,
                    },
                );
            }
            ctx.budget.charge(Resource::MatchSteps, 1)?;
            slot = (slot + 1) & mask;
        }
        let index = self.entries.len();
        ctx.budget
            .limits()
            .admit(Resource::MatchStates, index as u128 + 1)?;
        reserve(ctx, &mut self.entries, index + 1, ENTRY_CELLS)?;
        self.entries.push(Entry { probe, set });
        self.table[slot] = (self.generation, index as u32);
        if self.entries.len() * 2 > self.table.len() {
            self.rehash(ctx)?;
        }
        if matches!(pc, Pc::Enter(_)) {
            let needed = self.parked.len() + 1;
            reserve(ctx, &mut self.parked, needed, 1)?;
            self.parked.push(index as u32);
            return Ok(());
        }
        self.push(
            ctx,
            Item {
                pc,
                level,
                key: probe + 2,
                set,
            },
        )
    }

    /// Double the position's table, admitting both tables while entries move.
    fn rehash(&mut self, ctx: &mut Ctx<'_>) -> Result<(), Error> {
        let old = self.table.len();
        let size = old * 2;
        ctx.budget.limits().admit(
            Resource::MatchSlots,
            ctx.live_slots + size as u128 * SLOT_CELLS,
        )?;
        ctx.budget.charge_wide(
            Resource::MatchSteps,
            (size + self.entries.len() * 2) as u128,
        )?;
        let mut table = Vec::new();
        table
            .try_reserve_exact(size)
            .map_err(|_| Error::Allocation {
                resource: Resource::MatchSlots,
                units: size as u64,
            })?;
        table.resize(size, (0, 0));
        let mask = size - 1;
        for (index, entry) in self.entries.iter().enumerate() {
            let node = (self.keys[entry.probe] / 3) as usize;
            let depth = match self.keys[entry.probe] % 3 {
                2 => {
                    let Node::Repeat { body, .. } = ctx.program.nodes[node] else {
                        unreachable!("only a repetition iterates");
                    };
                    ctx.program.links.nodes[body].counters
                }
                _ => ctx.program.links.nodes[node].counters,
            };
            let probe = &self.keys[entry.probe..entry.probe + 2 + key_len(depth)];
            let mut slot = purrdf_hash::fixed::hash_one(probe) as usize & mask;
            while table[slot].0 == self.generation {
                slot = (slot + 1) & mask;
            }
            table[slot] = (self.generation, index as u32);
        }
        ctx.live_slots =
            ctx.live_slots - old as u128 * SLOT_CELLS + table.capacity() as u128 * SLOT_CELLS;
        self.table = table;
        Ok(())
    }

    /// Append `a` without the counts of `b`.
    fn difference(&mut self, ctx: &mut Ctx<'_>, a: Span, b: Span) -> Result<Span, Error> {
        let start = self.intervals.len();
        reserve(
            ctx,
            &mut self.intervals,
            start + a.len + b.len,
            INTERVAL_CELLS,
        )?;
        ctx.budget
            .charge_wide(Resource::MatchSteps, (a.len + b.len) as u128)?;
        let mut other = b.start;
        let end = b.start + b.len;
        for index in a.range() {
            let (mut low, high) = self.intervals[index];
            while other < end && self.intervals[other].1 < low {
                other += 1;
            }
            let mut cursor = other;
            let mut open = true;
            while cursor < end && self.intervals[cursor].0 <= high {
                let (cut_low, cut_high) = self.intervals[cursor];
                if cut_low > low {
                    self.intervals.push((low, cut_low - 1));
                }
                if cut_high >= high {
                    open = false;
                    break;
                }
                low = cut_high + 1;
                cursor += 1;
            }
            if open {
                self.intervals.push((low, high));
            }
        }
        Ok(Span {
            start,
            len: self.intervals.len() - start,
        })
    }

    /// Append the union of `a` and `b`.
    fn union(&mut self, ctx: &mut Ctx<'_>, a: Span, b: Span) -> Result<Span, Error> {
        let start = self.intervals.len();
        reserve(
            ctx,
            &mut self.intervals,
            start + a.len + b.len,
            INTERVAL_CELLS,
        )?;
        ctx.budget
            .charge_wide(Resource::MatchSteps, (a.len + b.len) as u128)?;
        let (mut left, mut right) = (a.start, b.start);
        let (left_end, right_end) = (a.start + a.len, b.start + b.len);
        while left < left_end || right < right_end {
            let take_left = right == right_end
                || (left < left_end && self.intervals[left].0 <= self.intervals[right].0);
            let next = if take_left {
                left += 1;
                self.intervals[left - 1]
            } else {
                right += 1;
                self.intervals[right - 1]
            };
            self.extend(start, next);
        }
        Ok(Span {
            start,
            len: self.intervals.len() - start,
        })
    }

    /// Append `interval` to the run beginning at `start`, coalescing it with
    /// the last interval when they overlap or touch. Capacity is reserved.
    fn extend(&mut self, start: usize, (low, high): (u64, u64)) {
        if self.intervals.len() > start
            && let Some(last) = self.intervals.last_mut()
            && low <= last.1.saturating_add(1)
        {
            last.1 = last.1.max(high);
            return;
        }
        self.intervals.push((low, high));
    }

    /// Append `set` with one more completed iteration for every count.
    fn shift(&mut self, ctx: &mut Ctx<'_>, set: Span, bounds: Bounds) -> Result<Span, Error> {
        let start = self.intervals.len();
        reserve(ctx, &mut self.intervals, start + set.len, INTERVAL_CELLS)?;
        for index in set.range() {
            let (low, high) = self.intervals[index];
            let next = (bounds.increment(low), bounds.increment(high));
            self.extend(start, next);
        }
        Ok(Span {
            start,
            len: self.intervals.len() - start,
        })
    }

    /// Append the counts of `set` at most `ceiling`, each stored at most
    /// `cap`.
    fn clip(
        &mut self,
        ctx: &mut Ctx<'_>,
        set: Span,
        ceiling: u64,
        cap: u64,
    ) -> Result<Span, Error> {
        let start = self.intervals.len();
        reserve(ctx, &mut self.intervals, start + set.len, INTERVAL_CELLS)?;
        for index in set.range() {
            let (low, high) = self.intervals[index];
            if low > ceiling {
                break;
            }
            let high = high.min(ceiling);
            self.extend(start, (low.min(cap), high.min(cap)));
        }
        Ok(Span {
            start,
            len: self.intervals.len() - start,
        })
    }

    /// Encode the parked entries as the state for the next position: each
    /// set reduced to the counts whose futures differ, in a canonical order.
    fn settle(&mut self, ctx: &mut Ctx<'_>) -> Result<(), Error> {
        let program = ctx.program;
        let links = &program.links.nodes;
        let count = self.parked.len();
        reserve(ctx, &mut self.order, count, 1)?;
        self.order.clear();
        self.order.extend_from_slice(&self.parked);
        let sort = count as u128 * (u128::from(count.bit_width()) + 1);
        ctx.budget.charge_wide(Resource::MatchSteps, sort)?;
        let (keys, entries) = (&self.keys, &self.entries);
        let span = |index: u32| {
            let probe = entries[index as usize].probe;
            let node = (keys[probe] / 3) as usize;
            &keys[probe + 2..probe + 2 + key_len(links[node].counters)]
        };
        self.order.sort_unstable_by(|&a, &b| {
            let node = |index: u32| keys[entries[index as usize].probe];
            node(a)
                .cmp(&node(b))
                .then_with(|| span(a).cmp(span(b)))
                .then(Ordering::Equal)
        });
        self.next.clear();
        reserve(ctx, &mut self.next, 2, 1)?;
        self.next.push(u64::from(self.accepted));
        self.next.push(count as u64);
        for position in 0..count {
            let entry = self.entries[self.order[position] as usize];
            let node = (self.keys[entry.probe] / 3) as usize;
            let len = key_len(links[node].counters);
            let at = self.next.len();
            reserve(ctx, &mut self.next, at + 2 + len + entry.set.len * 2, 1)?;
            ctx.budget.charge_wide(
                Resource::MatchSteps,
                copy_steps(2 + len + entry.set.len * 2),
            )?;
            self.next.push(node as u64);
            self.next
                .extend_from_slice(&self.keys[entry.probe + 2..entry.probe + 2 + len]);
            let held = self.next.len();
            self.next.push(0);
            let repeat = links[node].set;
            if repeat == ROOT {
                self.next.extend_from_slice(&[0, 0]);
                self.next[held] = 1;
                continue;
            }
            let Node::Repeat { min, max, .. } = program.nodes[repeat] else {
                unreachable!("only a repetition is counted");
            };
            let written = self.reduce(entry.set, Bounds { min, max });
            self.next[held] = written as u64;
        }
        Ok(())
    }

    /// Append the counts of `set` whose futures differ to the encoding, and
    /// return how many intervals were written. Capacity is reserved.
    fn reduce(&mut self, set: Span, bounds: Bounds) -> usize {
        let intervals = &self.intervals[set.range()];
        let Some(max) = bounds.finite_max() else {
            // Without a finite maximum the largest count allows the most.
            let largest = intervals.last().expect("a parked set is not empty").1;
            let stored = largest.min(bounds.cap());
            self.next.extend_from_slice(&[stored, stored]);
            return 1;
        };
        let Count::Finite(min) = bounds.min else {
            unreachable!("a minimum is at most its finite maximum");
        };
        // Two counts this close allow every count between them.
        let gap = max - min + 1;
        // Every count from here on allows what each larger count allows.
        let least = min.saturating_sub(1);
        let mut written = 0;
        let mut open: Option<(u64, u64)> = None;
        for &(low, high) in intervals {
            let (low, high) = match open {
                Some((open_low, open_high)) if low - open_high <= gap => (open_low, high),
                Some(done) => {
                    self.next.extend_from_slice(&<[u64; 2]>::from(done));
                    written += 1;
                    (low, high)
                }
                None => (low, high),
            };
            if high >= least {
                let kept = (low, low.max(least));
                self.next.extend_from_slice(&<[u64; 2]>::from(kept));
                return written + 1;
            }
            open = Some((low, high));
        }
        if let Some(done) = open {
            self.next.extend_from_slice(&<[u64; 2]>::from(done));
            written += 1;
        }
        written
    }

    /// Make the encoded next state current and intern it.
    fn intern(&mut self, ctx: &mut Ctx<'_>) -> Result<u32, Error> {
        std::mem::swap(&mut self.state, &mut self.next);
        self.fresh = true;
        let len = self.state.len();
        ctx.budget
            .charge_wide(Resource::MatchSteps, 1 + copy_steps(len))?;
        let hash = purrdf_hash::fixed::hash_one(&self.state[..]);
        if !self.cache.index.is_empty() {
            let mask = self.cache.index.len() - 1;
            let mut slot = hash as usize & mask;
            loop {
                let id = self.cache.index[slot];
                if id == u32::MAX {
                    break;
                }
                if self.cache.state(id) == self.state.as_slice() {
                    return Ok(id);
                }
                ctx.budget.charge(Resource::MatchSteps, 1)?;
                slot = (slot + 1) & mask;
            }
        }
        let used = self.cache.contents.len()
            + self.cache.spans.len() * 2
            + self.cache.index.len()
            + self.cache.moves.len() * 2;
        if used + len + 2 > CACHE_CELLS {
            self.clear_cache(ctx);
        }
        let id = self.cache.spans.len();
        let start = self.cache.contents.len();
        reserve(ctx, &mut self.cache.contents, start + len, 1)?;
        reserve(ctx, &mut self.cache.spans, id + 1, 2)?;
        self.cache.contents.extend_from_slice(&self.state);
        self.cache.spans.push((start, len));
        if (id + 1) * 2 > self.cache.index.len() {
            self.grow_index(ctx)?;
        } else {
            let mask = self.cache.index.len() - 1;
            let mut slot = hash as usize & mask;
            while self.cache.index[slot] != u32::MAX {
                slot = (slot + 1) & mask;
            }
            self.cache.index[slot] = id as u32;
        }
        Ok(id as u32)
    }

    /// Empty the cache.
    fn clear_cache(&mut self, ctx: &mut Ctx<'_>) {
        ctx.live_slots -= self.cache.cells() as u128;
        self.cache = Cache::default();
        self.epoch += 1;
    }

    /// Rebuild the state index at twice the size, with every interned state.
    fn grow_index(&mut self, ctx: &mut Ctx<'_>) -> Result<(), Error> {
        let old = self.cache.index.capacity();
        let size = (self.cache.index.len() * 2).max(16);
        ctx.budget
            .limits()
            .admit(Resource::MatchSlots, ctx.live_slots + size as u128)?;
        ctx.budget.charge_wide(
            Resource::MatchSteps,
            (size + self.cache.contents.len()) as u128,
        )?;
        let mut index = Vec::new();
        index
            .try_reserve_exact(size)
            .map_err(|_| Error::Allocation {
                resource: Resource::MatchSlots,
                units: size as u64,
            })?;
        index.resize(size, u32::MAX);
        let mask = size - 1;
        for id in 0..self.cache.spans.len() {
            let mut slot =
                purrdf_hash::fixed::hash_one(self.cache.state(id as u32)) as usize & mask;
            while index[slot] != u32::MAX {
                slot = (slot + 1) & mask;
            }
            index[slot] = id as u32;
        }
        ctx.live_slots = ctx.live_slots - old as u128 + index.capacity() as u128;
        self.cache.index = index;
        Ok(())
    }

    /// Record the move `key` to the interned state `next`.
    fn remember(&mut self, ctx: &mut Ctx<'_>, key: u64, next: u32) -> Result<(), Error> {
        if (self.cache.moves_held + 1) * 2 > self.cache.moves.len() {
            let old = self.cache.moves.capacity();
            let size = (self.cache.moves.len() * 2).max(16);
            ctx.budget
                .limits()
                .admit(Resource::MatchSlots, ctx.live_slots + size as u128 * 2)?;
            ctx.budget.charge_wide(
                Resource::MatchSteps,
                (size + self.cache.moves.len()) as u128,
            )?;
            let mut moves = Vec::new();
            moves
                .try_reserve_exact(size)
                .map_err(|_| Error::Allocation {
                    resource: Resource::MatchSlots,
                    units: size as u64 * 2,
                })?;
            moves.resize(size, (NO_MOVE, 0));
            let mask = size - 1;
            for &(stored, target) in &self.cache.moves {
                if stored == NO_MOVE {
                    continue;
                }
                let mut slot = purrdf_hash::fixed::hash_one(&stored) as usize & mask;
                while moves[slot].0 != NO_MOVE {
                    slot = (slot + 1) & mask;
                }
                moves[slot] = (stored, target);
            }
            ctx.live_slots = ctx.live_slots - old as u128 * 2 + moves.capacity() as u128 * 2;
            self.cache.moves = moves;
        }
        let mask = self.cache.moves.len() - 1;
        let mut slot = purrdf_hash::fixed::hash_one(&key) as usize & mask;
        while self.cache.moves[slot].0 != NO_MOVE {
            slot = (slot + 1) & mask;
        }
        self.cache.moves[slot] = (key, next);
        self.cache.moves_held += 1;
        Ok(())
    }
}

/// A change to a copied key.
#[derive(Debug, Clone, Copy)]
enum Rekey {
    Append(u64),
    Insert(usize, u64),
    Replace(usize, u64),
    Remove(usize),
}

/// Key values at a point with `counters` enclosing counted repetitions: every
/// count but the set's.
const fn key_len(counters: u32) -> usize {
    counters.saturating_sub(1) as usize
}
