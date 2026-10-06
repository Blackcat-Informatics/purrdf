// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ordered native matching, with one request's work fuel and live storage.
//!
//! A program without backreferences runs on the linear-time thread machine in
//! [`super::pike`]: its cost is bounded by the input length times the
//! program's distinct control states, whatever the nesting of repetitions.
//! A program with a backreference runs on the backtracking machine below,
//! whose continuations depend on captured text and so cannot be merged.
//!
//! Both machines visit alternatives in the same priority order, retain the
//! same captures, and apply the same progress rule: a nullable repetition
//! stops after an empty iteration once the minimum is satisfied; required
//! empty iterations still spend fuel. Backtracking alternatives retain
//! complete captures and continuations, and storage admission accounts for
//! those cells across all pending states, rather than admitting only the
//! small state header. No operational refusal is converted to a failed branch.

use std::ops::Range;

use purrdf_lex::walk::WorkList;

use super::compile::{CompiledPattern, Count, Lead, Node, Set, case_variants};
use super::pike::Pike;
use super::{Budget, Error, Limits, Profile, Refusal, Resource, unicode_tables};

/// UTF-8 byte spans captured by one ordered successful match.
#[derive(Debug, PartialEq, Eq)]
pub struct Captures {
    pub(super) spans: Vec<Option<Range<usize>>>,
}

impl Captures {
    /// The capture's UTF-8 byte span; zero is the entire match.
    ///
    /// A group that did not participate and an out-of-range group return None.
    #[must_use]
    pub fn get(&self, number: usize) -> Option<Range<usize>> {
        self.spans.get(number).cloned().flatten()
    }

    /// Captures including the entire-pattern capture zero.
    #[must_use]
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Whether no capture slots exist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

impl CompiledPattern {
    /// Whether the input contains a match under this program's dated law.
    ///
    /// # Errors
    ///
    /// Current artifact admission, matcher work and live-state/storage failures
    /// are typed operational errors, never negative matches.
    pub fn is_match(&self, input: &str, limits: Limits) -> Result<bool, Error> {
        Ok(self.find(input, limits)?.is_some())
    }

    /// The first ordered match and its capturing-group spans.
    ///
    /// Search visits UTF-8 boundaries in order. Alternatives visit source order;
    /// greedy/reluctant repetition chooses the corresponding continuation order.
    ///
    /// # Errors
    ///
    /// Admission or execution may refuse its finite resource bound. No partial
    /// match is returned after an operational failure.
    pub fn find(&self, input: &str, limits: Limits) -> Result<Option<Captures>, Error> {
        self.admit(limits)?;
        Vm::new(self, input, limits).find_from(0)
    }
}

/// Work a backtracking attempt may spend per search start it has passed, when
/// the thread machine can take over the program.
const ATTEMPT_START_STEPS: u64 = 16;

/// Work such an attempt may spend per input byte it has examined.
const ATTEMPT_BYTE_STEPS: u64 = 8;

/// Work such an attempt may spend per program node, wherever it is.
const ATTEMPT_NODE_STEPS: u64 = 16;

/// The progress of a backtracking attempt the thread machine can take over.
///
/// An attempt that spends more than its allowance for the starts it has
/// passed and the input it has examined is not running in linear time, and is
/// abandoned before its cost grows further.
#[derive(Debug, Clone, Copy)]
struct Attempt {
    /// The first start of the search.
    origin: usize,
    /// The steps the execution had spent before this search.
    before: u64,
    /// The furthest input position any state of the attempt has reached.
    furthest: usize,
    /// The allowance for the program, independent of the input.
    program: u64,
    /// The steps this attempt may spend before its allowance is recomputed.
    ///
    /// The allowance only grows with the start and the furthest position, so
    /// spending within the last computed value needs no new arithmetic.
    allowed: u64,
}

impl Attempt {
    /// Whether `spent`, at search start `start`, exceeds the allowance.
    #[inline]
    fn exceeded(&mut self, start: usize, position: usize, spent: u64) -> Option<Refusal> {
        self.furthest = self.furthest.max(position);
        if spent - self.before <= self.allowed {
            return None;
        }
        self.recompute(start, spent)
    }

    #[cold]
    fn recompute(&mut self, start: usize, spent: u64) -> Option<Refusal> {
        let allowance = u128::from(ATTEMPT_START_STEPS) * (start - self.origin + 1) as u128
            + u128::from(ATTEMPT_BYTE_STEPS) * (self.furthest - self.origin) as u128
            + u128::from(self.program);
        let spent = u128::from(spent - self.before);
        self.allowed = u64::try_from(allowance).unwrap_or(u64::MAX);
        (spent > allowance).then(|| Refusal {
            resource: Resource::MatchSteps,
            required: spent,
            limit: u64::try_from(allowance).unwrap_or(u64::MAX),
        })
    }
}

/// The matcher a program runs on.
///
/// A program with a backreference runs on the backtracking machine alone. Any
/// other program first runs the backtracking machine, whose single-character
/// runs and skipped search starts make ordinary searches cost about one step
/// per character, under a work allowance linear in the search starts it has
/// passed, the input it has examined and the program. An attempt that exceeds
/// that allowance or any live-storage bound is abandoned, its pending states
/// are released, and the linear-time thread machine runs the same search, and
/// every later one, on the remaining fuel. The attempt's work stays charged.
/// Both machines report the same match, so the attempt changes only the cost.
pub(super) struct Vm<'a> {
    machine: Machine<'a>,
}

// One machine lives on the stack for one execution; boxing the backtracking
// machine's inline work lists would add an allocation to every match.
#[allow(clippy::large_enum_variant)]
enum Machine<'a> {
    Backtrack {
        machine: Backtrack<'a>,
        /// Whether the thread machine may take over an abandoned attempt.
        fallback: bool,
    },
    Pike(Pike<'a>),
    /// Only while the machines are exchanged.
    Exchanging,
}

impl<'a> Vm<'a> {
    pub(super) fn new(program: &'a CompiledPattern, input: &'a str, limits: Limits) -> Self {
        Self {
            machine: Machine::Backtrack {
                machine: Backtrack::new(program, input, limits),
                fallback: !program.links.backreferences,
            },
        }
    }

    /// The backtracking machine alone, which every program can run on.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(super) fn backtracking(
        program: &'a CompiledPattern,
        input: &'a str,
        limits: Limits,
    ) -> Self {
        Self {
            machine: Machine::Backtrack {
                machine: Backtrack::new(program, input, limits),
                fallback: false,
            },
        }
    }

    /// The thread machine alone, for a backreference-free program.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(super) fn threads(program: &'a CompiledPattern, input: &'a str, limits: Limits) -> Self {
        assert!(!program.links.backreferences);
        Self {
            machine: Machine::Pike(Pike::new(Ctx::new(program, input, limits))),
        }
    }

    /// Whether the thread machine has taken over this execution.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(super) const fn is_threaded(&self) -> bool {
        matches!(self.machine, Machine::Pike(_))
    }

    /// This execution's work and storage accounting.
    pub(super) fn budget(&mut self) -> &mut Budget {
        match &mut self.machine {
            Machine::Backtrack { machine, .. } => &mut machine.ctx.budget,
            Machine::Pike(machine) => &mut machine.ctx.budget,
            Machine::Exchanging => unreachable!("the machines are exchanged within one call"),
        }
    }

    /// The first ordered match starting at or after the UTF-8 offset `start`.
    pub(super) fn find_from(&mut self, start: usize) -> Result<Option<Captures>, Error> {
        match &mut self.machine {
            Machine::Backtrack {
                machine,
                fallback: false,
            } => machine.find_from(start),
            Machine::Backtrack {
                machine,
                fallback: true,
            } => {
                let program = machine.ctx.program;
                machine.attempt = Some(Attempt {
                    origin: start,
                    before: machine.ctx.budget.used(Resource::MatchSteps),
                    furthest: start,
                    program: ATTEMPT_NODE_STEPS
                        .saturating_mul((program.nodes.len() + program.sets.len()) as u64),
                    allowed: 0,
                });
                let attempt = machine.find_from(start);
                machine.attempt = None;
                match attempt {
                    Err(Error::Resource(_)) => {}
                    decided => return decided,
                }
                let Machine::Backtrack { machine, .. } =
                    std::mem::replace(&mut self.machine, Machine::Exchanging)
                else {
                    unreachable!("the backtracking machine was matched above");
                };
                self.machine = Machine::Pike(Pike::new(machine.abandon()));
                self.find_from(start)
            }
            Machine::Pike(machine) => machine.find_from(start),
            Machine::Exchanging => unreachable!("the machines are exchanged within one call"),
        }
    }
}

#[derive(Clone, Copy)]
enum SetAction {
    Visit(usize),
    Not,
    Or,
    Difference,
}

/// What both matchers share: the program, the input, the request's fuel, the
/// live-storage count, and the character-set, anchor and search-start tests.
pub(super) struct Ctx<'a> {
    pub(super) program: &'a CompiledPattern,
    pub(super) input: &'a str,
    pub(super) budget: Budget,
    pub(super) live_slots: u128,
}

impl<'a> Ctx<'a> {
    pub(super) fn new(program: &'a CompiledPattern, input: &'a str, limits: Limits) -> Self {
        Self {
            program,
            input,
            budget: Budget::new(limits),
            live_slots: 0,
        }
    }

    fn push<T, const N: usize>(
        &mut self,
        stack: &mut WorkList<T, N>,
        value: T,
        slots: u64,
    ) -> Result<(), Error> {
        let required = self.live_slots + u128::from(slots);
        self.budget.limits().admit(Resource::MatchSlots, required)?;
        stack.try_push(value).map_err(|_| Error::Allocation {
            resource: Resource::MatchSlots,
            units: stack.len() as u64 + 1,
        })?;
        self.live_slots = required;
        Ok(())
    }

    /// Whether `position` satisfies `^` under the program's modes.
    pub(super) fn at_start(&self, position: usize) -> bool {
        position == 0
            || (self.program.modes.multiline
                && position < self.input.len()
                && self.input[..position].ends_with('\n'))
    }

    /// Whether `position` satisfies `$` under the program's modes.
    pub(super) fn at_end(&self, position: usize) -> bool {
        if self.program.modes.multiline {
            self.input[position..].starts_with('\n')
                || (position == self.input.len() && !self.input.ends_with('\n'))
        } else {
            position == self.input.len()
        }
    }

    /// Whether `ch` belongs to the search's leading set, in one comparison
    /// against the sorted ranges when they exist.
    #[inline]
    fn leads_with(&mut self, first: usize, ch: char) -> Result<bool, Error> {
        let scalars = &self.program.lead.scalars;
        if scalars.is_empty() {
            self.set_matches(first, ch)
        } else {
            self.budget.charge(Resource::MatchSteps, 1)?;
            let index = scalars.partition_point(|&(_, hi)| hi < ch);
            Ok(scalars.get(index).is_some_and(|&(lo, _)| lo <= ch))
        }
    }

    /// Whether a match can begin at `start`, spending the comparison that
    /// decides it.
    pub(super) fn may_start(&mut self, start: usize) -> Result<bool, Error> {
        match self.program.lead.first {
            Lead::Any => Ok(true),
            Lead::Set(first) => match self.input[start..].chars().next() {
                Some(ch) => self.leads_with(first, ch),
                None => Ok(false),
            },
            Lead::Start => {
                self.budget.charge(Resource::MatchSteps, 1)?;
                Ok(self.at_start(start))
            }
        }
    }

    /// The first start position at or after `start` that can begin a match.
    ///
    /// Each skipped position spends the comparison that rejects it, so a
    /// literal search costs about one step per scanned character.
    pub(super) fn candidate(&mut self, mut start: usize) -> Result<Option<usize>, Error> {
        match self.program.lead.first {
            Lead::Any => Ok(Some(start)),
            Lead::Set(first) => {
                while let Some(ch) = self.input[start..].chars().next() {
                    if self.leads_with(first, ch)? {
                        return Ok(Some(start));
                    }
                    start += ch.len_utf8();
                }
                Ok(None)
            }
            Lead::Start => {
                if start == 0 {
                    return Ok(Some(0));
                }
                if !self.program.modes.multiline {
                    return Ok(None);
                }
                // The multiline start: after a newline that is not final.
                loop {
                    self.budget.charge(Resource::MatchSteps, 1)?;
                    if start >= self.input.len() {
                        return Ok(None);
                    }
                    if self.input[..start].ends_with('\n') {
                        return Ok(Some(start));
                    }
                    start += self.input[start..]
                        .chars()
                        .next()
                        .expect("start precedes the end")
                        .len_utf8();
                }
            }
        }
    }

    pub(super) fn set_matches(&mut self, root: usize, ch: char) -> Result<bool, Error> {
        if let Some(ranges) = self.program.flat.get(root) {
            // One comparison against the flattened union: one held cell and
            // one step, as for an atomic set.
            self.budget
                .limits()
                .admit(Resource::MatchSlots, self.live_slots + 1)?;
            self.budget.charge(Resource::MatchSteps, 1)?;
            let index = ranges.partition_point(|&(_, hi)| hi < ch);
            return Ok(ranges.get(index).is_some_and(|&(lo, _)| lo <= ch));
        }
        let set = self.program.sets[root];
        if !matches!(
            set,
            Set::Complement(_) | Set::Union(..) | Set::Difference(..)
        ) {
            // The work list below would hold one cell at a time and spend one
            // step before the atomic test: the same admission and spend.
            self.budget
                .limits()
                .admit(Resource::MatchSlots, self.live_slots + 1)?;
            self.budget.charge(Resource::MatchSteps, 1)?;
            return self.atomic_set(set, ch);
        }
        let mut work = WorkList::<SetAction, 16>::new();
        let mut values = WorkList::<bool, 16>::new();
        self.push(&mut work, SetAction::Visit(root), 1)?;
        while let Some(action) = work.pop() {
            self.live_slots -= 1;
            self.budget.charge(Resource::MatchSteps, 1)?;
            match action {
                SetAction::Visit(set) => match self.program.sets[set] {
                    Set::Complement(child) => {
                        self.push(&mut work, SetAction::Not, 1)?;
                        self.push(&mut work, SetAction::Visit(child), 1)?;
                    }
                    Set::Union(left, right) | Set::Difference(left, right) => {
                        let join = if matches!(self.program.sets[set], Set::Union(..)) {
                            SetAction::Or
                        } else {
                            SetAction::Difference
                        };
                        self.push(&mut work, join, 1)?;
                        self.push(&mut work, SetAction::Visit(right), 1)?;
                        self.push(&mut work, SetAction::Visit(left), 1)?;
                    }
                    set => {
                        let value = self.atomic_set(set, ch)?;
                        self.push(&mut values, value, 1)?;
                    }
                },
                SetAction::Not => {
                    let value = values.pop().expect("a complement operand completed");
                    self.live_slots -= 1;
                    self.push(&mut values, !value, 1)?;
                }
                SetAction::Or | SetAction::Difference => {
                    let right = values.pop().expect("right set operand completed");
                    let left = values.pop().expect("left set operand completed");
                    self.live_slots -= 2;
                    let value = if matches!(action, SetAction::Or) {
                        left || right
                    } else {
                        left && !right
                    };
                    self.push(&mut values, value, 1)?;
                }
            }
        }
        let value = values.pop().expect("one complete character-set result");
        self.live_slots -= 1;
        Ok(value)
    }

    fn atomic_set(&mut self, set: Set, ch: char) -> Result<bool, Error> {
        Ok(match set {
            Set::Range { lo, hi, folded } => {
                if (lo..=hi).contains(&ch) {
                    true
                } else if folded {
                    let variants = case_variants(ch);
                    self.budget
                        .charge_wide(Resource::MatchSteps, variants.len() as u128)?;
                    variants
                        .iter()
                        .any(|&variant| (lo as u32..=hi as u32).contains(&variant))
                } else {
                    false
                }
            }
            Set::Table(ranges) => purrdf_iri::terminals::in_ranges(ch as u32, ranges),
            Set::Space => purrdf_iri::terminals::is_ws_char(ch),
            Set::Word => {
                self.budget.charge(Resource::MatchSteps, 3)?;
                !["P", "Z", "C"].iter().any(|name| {
                    let index = unicode_tables::CATEGORIES
                        .binary_search_by_key(name, |&(name, _)| name)
                        .expect("generated table contains every major general category");
                    purrdf_iri::terminals::in_ranges(ch as u32, unicode_tables::CATEGORIES[index].1)
                })
            }
            Set::Dot => {
                self.program.modes.dot_all
                    || (ch != '\n' && (self.program.profile == Profile::Xpath20 || ch != '\r'))
            }
            Set::Complement(_) | Set::Union(..) | Set::Difference(..) => {
                unreachable!("compound character sets are evaluated on the explicit work list");
            }
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Node(usize),
    CaptureEnd {
        number: usize,
        start: usize,
    },
    Repeat {
        node: usize,
        count: u64,
        stalled: bool,
    },
    RepeatEnd {
        node: usize,
        count: u64,
        start: usize,
    },
}

/// The remaining stops of a greedy single-character run.
#[derive(Debug, Clone, Copy)]
struct Run {
    /// The shortest admissible stop.
    floor: usize,
    /// The set the character after a viable stop must belong to.
    follow: Option<usize>,
}

struct State {
    start: usize,
    position: usize,
    /// A pending greedy single-character run: its position is the next stop
    /// to consider, and resuming it first leaves the next shorter stop pending.
    run: Option<Run>,
    actions: WorkList<Action, 8>,
    captures: Vec<Option<Range<usize>>>,
}

impl State {
    fn slots(&self) -> u128 {
        2 + (self.captures.len() as u128) * 2 + (self.actions.len() as u128) * 3
    }
}

/// The backtracking machine for programs whose continuations read captures.
pub(super) struct Backtrack<'a> {
    ctx: Ctx<'a>,
    pending: WorkList<State, 4>,
    /// The allowance of an attempt the thread machine can take over.
    attempt: Option<Attempt>,
}

impl<'a> Backtrack<'a> {
    fn new(program: &'a CompiledPattern, input: &'a str, limits: Limits) -> Self {
        Self {
            ctx: Ctx::new(program, input, limits),
            pending: WorkList::new(),
            attempt: None,
        }
    }

    /// Release every pending state and hand over the request's accounting.
    fn abandon(self) -> Ctx<'a> {
        let Self {
            mut ctx, pending, ..
        } = self;
        drop(pending);
        // A refused attempt may hold its current state and its set-evaluation
        // cells too; all of them are dropped with the machine.
        ctx.live_slots = 0;
        ctx
    }

    fn action(&mut self, state: &mut State, action: Action) -> Result<(), Error> {
        self.ctx.push(&mut state.actions, action, 3)
    }

    fn initial(&mut self, start: usize) -> Result<State, Error> {
        let count = self.ctx.program.captures + 1;
        let slots = 2 + (count as u128) * 2;
        self.ctx
            .budget
            .limits()
            .admit(Resource::MatchSlots, self.ctx.live_slots + slots)?;
        self.ctx
            .budget
            .charge_wide(Resource::MatchSteps, count as u128)?;
        let mut captures = Vec::new();
        captures
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation {
                resource: Resource::MatchSlots,
                units: count as u64,
            })?;
        captures.resize_with(count, || None);
        self.ctx.live_slots += slots;
        let mut state = State {
            start,
            position: start,
            run: None,
            actions: WorkList::new(),
            captures,
        };
        self.action(&mut state, Action::Node(self.ctx.program.root))?;
        Ok(state)
    }

    fn fork(&mut self, state: &State) -> Result<State, Error> {
        self.ctx
            .budget
            .limits()
            .admit(Resource::MatchStates, self.pending.len() as u128 + 1)?;
        let slots = state.slots();
        self.ctx
            .budget
            .limits()
            .admit(Resource::MatchSlots, self.ctx.live_slots + slots)?;
        self.ctx.budget.charge_wide(
            Resource::MatchSteps,
            state.captures.len() as u128 + state.actions.len() as u128,
        )?;
        let mut captures = Vec::new();
        captures
            .try_reserve_exact(state.captures.len())
            .map_err(|_| Error::Allocation {
                resource: Resource::MatchSlots,
                units: state.captures.len() as u64,
            })?;
        captures.extend(state.captures.iter().cloned());
        let mut actions = WorkList::new();
        for &action in state.actions.iter() {
            actions.try_push(action).map_err(|_| Error::Allocation {
                resource: Resource::MatchSlots,
                units: state.actions.len() as u64,
            })?;
        }
        self.ctx.live_slots += slots;
        Ok(State {
            start: state.start,
            position: state.position,
            run: None,
            actions,
            captures,
        })
    }

    fn enqueue(&mut self, state: State) -> Result<(), Error> {
        self.ctx
            .budget
            .limits()
            .admit(Resource::MatchStates, self.pending.len() as u128 + 1)?;
        self.pending.try_push(state).map_err(|_| Error::Allocation {
            resource: Resource::MatchStates,
            units: self.pending.len() as u64 + 1,
        })
    }

    /// The first start after a failed `start` that a leading unbounded run
    /// does not already cover, or the next character without such a run.
    fn after_failure(&mut self, start: usize) -> Result<Option<usize>, Error> {
        let input = self.ctx.input;
        let mut next = start;
        if let Some(run) = self.ctx.program.lead.run {
            while let Some(ch) = input[next..].chars().next() {
                if !self.ctx.set_matches(run, ch)? {
                    break;
                }
                next += ch.len_utf8();
            }
        }
        if next == input.len() {
            return Ok(None);
        }
        self.ctx.budget.charge(Resource::MatchSteps, 1)?;
        next += input[next..]
            .chars()
            .next()
            .expect("next precedes the end")
            .len_utf8();
        Ok(Some(next))
    }

    /// Resume a pending state, first leaving its next shorter greedy stop.
    ///
    /// A run with no viable stop left is discarded, and None is returned.
    fn resume(&mut self, mut state: State) -> Result<Option<State>, Error> {
        if let Some(run) = state.run.take() {
            let Some(stop) = self.stop(state.position, run)? else {
                self.ctx.live_slots -= state.slots();
                return Ok(None);
            };
            self.stop_at(&mut state, stop, run)?;
        }
        Ok(Some(state))
    }

    /// The longest stop at or below `position` whose next character can
    /// continue, spending one comparison per stop it passes over.
    fn stop(&mut self, mut position: usize, run: Run) -> Result<Option<usize>, Error> {
        let Some(follow) = run.follow else {
            return Ok(Some(position));
        };
        let input = self.ctx.input;
        loop {
            if let Some(ch) = input[position..].chars().next()
                && self.ctx.set_matches(follow, ch)?
            {
                return Ok(Some(position));
            }
            if position == run.floor {
                return Ok(None);
            }
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            position = self.previous(position);
        }
    }

    /// Continue at `stop`, leaving every shorter stop as one pending state.
    fn stop_at(&mut self, state: &mut State, stop: usize, run: Run) -> Result<(), Error> {
        state.position = stop;
        if stop > run.floor {
            let mut shorter = self.fork(state)?;
            shorter.position = self.previous(stop);
            shorter.run = Some(run);
            self.enqueue(shorter)?;
        }
        Ok(())
    }

    fn previous(&self, position: usize) -> usize {
        position
            - self.ctx.input[..position]
                .chars()
                .next_back()
                .expect("a run above its floor consumed a character")
                .len_utf8()
    }

    fn find_from(&mut self, start: usize) -> Result<Option<Captures>, Error> {
        let Some(mut start) = self.ctx.candidate(start)? else {
            return Ok(None);
        };
        loop {
            let mut state = self.initial(start)?;
            loop {
                self.ctx.budget.charge(Resource::MatchSteps, 1)?;
                if let Some(attempt) = &mut self.attempt
                    && let Some(refusal) = attempt.exceeded(
                        start,
                        state.position,
                        self.ctx.budget.used(Resource::MatchSteps),
                    )
                {
                    return Err(refusal.into());
                }
                if let Some(action) = state.actions.pop() {
                    self.ctx.live_slots -= 3;
                    if self.execute(&mut state, action)? {
                        continue;
                    }
                } else {
                    state.captures[0] = Some(state.start..state.position);
                    // A later replacement search cannot inherit continuations
                    // from this successful match. Returned captures are expanded
                    // and dropped before that caller starts its next search.
                    self.ctx.live_slots -= state.slots();
                    while let Some(stale) = self.pending.pop() {
                        self.ctx.live_slots -= stale.slots();
                    }
                    debug_assert_eq!(self.ctx.live_slots, 0);
                    return Ok(Some(Captures {
                        spans: state.captures,
                    }));
                }
                self.ctx.live_slots -= state.slots();
                drop(state);
                let mut resumed = None;
                while let Some(next) = self.pending.pop() {
                    resumed = self.resume(next)?;
                    if resumed.is_some() {
                        break;
                    }
                }
                if let Some(next) = resumed {
                    state = next;
                } else {
                    break;
                }
            }
            let Some(next) = self.after_failure(start)? else {
                return Ok(None);
            };
            let Some(next) = self.ctx.candidate(next)? else {
                return Ok(None);
            };
            start = next;
        }
    }

    fn execute(&mut self, state: &mut State, action: Action) -> Result<bool, Error> {
        match action {
            Action::Node(node) => self.node(state, node),
            Action::CaptureEnd { number, start } => {
                state.captures[number] = Some(start..state.position);
                Ok(true)
            }
            Action::Repeat {
                node,
                count,
                stalled,
            } => self.repeat(state, node, count, stalled),
            Action::RepeatEnd { node, count, start } => {
                // This spend precedes every count increment, even an empty one.
                self.ctx.budget.charge(Resource::MatchSteps, 1)?;
                let count = count
                    .checked_add(1)
                    .expect("finite fuel refuses before a repetition count can overflow");
                self.action(
                    state,
                    Action::Repeat {
                        node,
                        count,
                        stalled: state.position == start,
                    },
                )?;
                Ok(true)
            }
        }
    }

    fn node(&mut self, state: &mut State, node: usize) -> Result<bool, Error> {
        match self.ctx.program.nodes[node] {
            Node::Empty => Ok(true),
            Node::Start => Ok(self.ctx.at_start(state.position)),
            Node::End => Ok(self.ctx.at_end(state.position)),
            Node::Character(set) => {
                let Some(ch) = self.ctx.input[state.position..].chars().next() else {
                    return Ok(false);
                };
                if !self.ctx.set_matches(set, ch)? {
                    return Ok(false);
                }
                state.position += ch.len_utf8();
                Ok(true)
            }
            Node::Backreference(number) => self.backreference(state, number),
            Node::Sequence(left, right) => {
                self.action(state, Action::Node(right))?;
                self.action(state, Action::Node(left))?;
                Ok(true)
            }
            Node::Choice(left, right) => {
                let mut alternative = self.fork(state)?;
                self.action(&mut alternative, Action::Node(right))?;
                self.enqueue(alternative)?;
                self.action(state, Action::Node(left))?;
                Ok(true)
            }
            Node::Capture { number, body } => {
                self.action(
                    state,
                    Action::CaptureEnd {
                        number,
                        start: state.position,
                    },
                )?;
                self.action(state, Action::Node(body))?;
                Ok(true)
            }
            Node::Repeat { .. } => {
                self.action(
                    state,
                    Action::Repeat {
                        node,
                        count: 0,
                        stalled: false,
                    },
                )?;
                Ok(true)
            }
        }
    }

    fn repeat(
        &mut self,
        state: &mut State,
        node: usize,
        count: u64,
        stalled: bool,
    ) -> Result<bool, Error> {
        let Node::Repeat {
            body,
            min,
            max,
            greedy,
            ..
        } = self.ctx.program.nodes[node]
        else {
            unreachable!("a repetition continuation names its immutable repeat node");
        };
        if greedy
            && count == 0
            && let Node::Character(set) = self.ctx.program.nodes[body]
        {
            let Node::Repeat { follow, .. } = self.ctx.program.nodes[node] else {
                unreachable!("the repetition was matched above");
            };
            return self.greedy_run(state, set, min, max, follow);
        }
        let (can_stop, can_repeat) = decide(min, max, count, stalled);
        if !can_repeat {
            return Ok(can_stop);
        }
        if can_stop {
            let mut alternative = self.fork(state)?;
            if greedy {
                self.enqueue(alternative)?;
            } else {
                self.iteration(&mut alternative, node, body, count)?;
                self.enqueue(alternative)?;
                return Ok(true);
            }
        }
        self.iteration(state, node, body, count)?;
        Ok(true)
    }

    /// A greedy repetition of one character set, without a pending state per
    /// iteration.
    ///
    /// Every iteration consumes exactly one character and has no capture, so
    /// the alternatives differ only in where the run stops. The run is scanned
    /// once; a single pending state then stands for every shorter admissible
    /// stop, longest first, which is the order the general repetition visits.
    fn greedy_run(
        &mut self,
        state: &mut State,
        set: usize,
        min: Count,
        max: Option<Count>,
        follow: Option<usize>,
    ) -> Result<bool, Error> {
        let min = match min {
            Count::Finite(min) => Some(min),
            Count::AboveU64 => None,
        };
        let max = match max {
            Some(Count::Finite(max)) => max,
            None | Some(Count::AboveU64) => u64::MAX,
        };
        let input = self.ctx.input;
        let mut floor = (min == Some(0)).then_some(state.position);
        let mut count = 0_u64;
        while count < max {
            // The same spend that precedes every general repetition count.
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            let Some(ch) = input[state.position..].chars().next() else {
                break;
            };
            if !self.ctx.set_matches(set, ch)? {
                break;
            }
            state.position += ch.len_utf8();
            count += 1;
            if Some(count) == min {
                floor = Some(state.position);
            }
        }
        let Some(floor) = floor else {
            return Ok(false);
        };
        let run = Run { floor, follow };
        let Some(stop) = self.stop(state.position, run)? else {
            return Ok(false);
        };
        self.stop_at(state, stop, run)?;
        Ok(true)
    }

    fn iteration(
        &mut self,
        state: &mut State,
        node: usize,
        body: usize,
        count: u64,
    ) -> Result<(), Error> {
        self.action(
            state,
            Action::RepeatEnd {
                node,
                count,
                start: state.position,
            },
        )?;
        self.action(state, Action::Node(body))
    }

    fn backreference(&mut self, state: &mut State, number: usize) -> Result<bool, Error> {
        let Some(span) = state.captures[number].clone() else {
            return Ok(true);
        };
        let input = self.ctx.input;
        for expected in input[span].chars() {
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            let Some(actual) = input[state.position..].chars().next() else {
                return Ok(false);
            };
            if actual != expected {
                if !self.ctx.program.modes.insensitive {
                    return Ok(false);
                }
                let variants = case_variants(expected);
                self.ctx
                    .budget
                    .charge_wide(Resource::MatchSteps, variants.len() as u128)?;
                if !variants.contains(&(actual as u32)) {
                    return Ok(false);
                }
            }
            state.position += actual.len_utf8();
        }
        Ok(true)
    }
}

/// Whether a repetition that has completed `count` iterations may stop, and
/// whether it may iterate again.
///
/// An iteration that consumed nothing (`stalled`) once the minimum is met
/// stops the repetition: a further empty iteration could only repeat it.
pub(super) fn decide(min: Count, max: Option<Count>, count: u64, stalled: bool) -> (bool, bool) {
    let can_stop = matches!(min, Count::Finite(min) if count >= min);
    let can_repeat =
        !(matches!(max, Some(Count::Finite(max)) if count >= max) || stalled && can_stop);
    (can_stop, can_repeat)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::xsd_regex::xpath::{Refusal, compile};

    fn pattern(profile: Profile, source: &str, flags: &str) -> CompiledPattern {
        compile(profile, source, flags, Limits::new()).unwrap()
    }

    fn matched(source: &str, flags: &str, input: &str) -> Captures {
        pattern(Profile::Xpath31, source, flags)
            .find(input, Limits::new())
            .unwrap()
            .unwrap()
    }

    /// Cartesian words, including empty, specified without the pattern parser
    /// or VM. Every input below has at most five scalar values.
    fn words(alphabet: &[char], maximum: usize) -> Vec<String> {
        let mut all = vec![String::new()];
        let mut level = vec![String::new()];
        for _ in 0..maximum {
            let mut next = Vec::new();
            for prefix in level {
                for &ch in alphabet {
                    let mut word = prefix.clone();
                    word.push(ch);
                    next.push(word);
                }
            }
            all.extend(next.iter().cloned());
            level = next;
        }
        all
    }

    #[test]
    fn independent_finite_languages_cover_matching_and_first_search_position() {
        // Expected words are specified directly, including equality constraints
        // from backreferences. The bounded star language is complete for this
        // input universe; no input contains more than five scalar values.
        let fixtures = [
            ("a", vec!["a".to_owned()]),
            ("(a|b)", vec!["a".to_owned(), "b".to_owned()]),
            ("(ab|a)", vec!["ab".to_owned(), "a".to_owned()]),
            ("(a|b){0,3}", words(&['a', 'b'], 3)),
            ("a{1,3}", ["a", "aa", "aaa"].map(str::to_owned).to_vec()),
            ("(ab){1,2}", ["ab", "abab"].map(str::to_owned).to_vec()),
            ("(|a)b?", ["", "b", "a", "ab"].map(str::to_owned).to_vec()),
            ("(a|b)*", words(&['a', 'b'], 5)),
            ("[a-b-[b]]{1,2}", ["a", "aa"].map(str::to_owned).to_vec()),
            (r"(a|b)\1", ["aa", "bb"].map(str::to_owned).to_vec()),
            (
                r"((a|b)\2){2}",
                ["aaaa", "aabb", "bbaa", "bbbb"].map(str::to_owned).to_vec(),
            ),
            (r"(a?)\1", ["", "aa"].map(str::to_owned).to_vec()),
            (r"((a|ab))\2", ["aa", "abab"].map(str::to_owned).to_vec()),
            (r"(é|𐀀)\1", ["éé", "𐀀𐀀"].map(str::to_owned).to_vec()),
        ];
        let inputs = words(&['a', 'b', 'é', '𐀀'], 5);
        assert_eq!(inputs.len(), 1365);
        let mut comparisons = 0;
        for profile in [Profile::Xpath20, Profile::Xpath31] {
            for (source, language) in &fixtures {
                // No added group: capture numbering belongs to the fixture.
                let whole = pattern(profile, &format!("^{source}$"), "");
                let search = pattern(profile, source, "");
                for input in &inputs {
                    assert_eq!(
                        whole.is_match(input, Limits::new()).unwrap(),
                        language.contains(input),
                        "whole {profile:?} {source:?} {input:?}"
                    );
                    let first = language.iter().filter_map(|word| input.find(word)).min();
                    let actual = search.find(input, Limits::new()).unwrap();
                    assert_eq!(
                        actual
                            .as_ref()
                            .map(|captures| captures.get(0).unwrap().start),
                        first,
                        "search {profile:?} {source:?} {input:?}"
                    );
                    if let Some(captures) = actual {
                        let span = captures.get(0).unwrap();
                        let text = &input[span];
                        assert!(language.iter().any(|word| word == text));
                    }
                    comparisons += 1;
                }
            }
        }
        assert_eq!(comparisons, 38_220);
    }

    /// Every whole match and capture of the thread machine, from every start,
    /// against the backtracking machine on the same backreference-free program.
    #[test]
    fn thread_machine_reports_the_backtracking_machines_matches_and_captures() {
        let sources = [
            "a",
            "(a|b)",
            "(ab|a)",
            "(a|b){0,3}",
            "a{1,3}",
            "(ab){1,2}",
            "(|a)b?",
            "(a|b)*",
            "(a?)*",
            "(a*)*",
            "(a*)+",
            "(a|)*",
            "(|a)*",
            "(a?){2}",
            "(a?){0,2}",
            "(a?){2,}",
            "(a??)*",
            "(a*?)*",
            "((a)|b)+",
            "((a)|(b))*",
            "(a|ab)(b|bab)?",
            "^(a|aa)*$",
            "(a|aa)*b",
            "^(a*)*b$",
            "((a+)+)+b",
            "(a|b)*?b",
            "a*?",
            "(a+?)(a*)",
            "(?:a|b)+?",
            "((a?)(b?))*",
            "(a(b)?)+",
            "(a|b|)+",
            "(()|a)+",
            "(a*|b)*",
            "(a{0,2}){2}",
            "((a|b){2})*",
            "^(?:a|b)*$",
            "$",
            "^",
            "^$",
            "(^a|b$)+",
            "(a$|b)*",
            "(^|a)+",
            "(\\n|^)a",
            "a.b",
            ".*",
            "(.)*?a",
            "[ab]{2,}",
            "(é|b)+",
            "(a|é){1,3}?",
            "((a|é)?){3}",
            "(a{2})*",
            "(a{0}b)*",
            "x{0}",
            "((a*)(b*))*",
            "(a|(b))+?a",
            "((a)*|b)*",
            "(?:(a)|b|)*",
            "((?:a|)*)*",
            "(a?)+?b",
            "((a?)*?)*",
            "^(a?){1,2}$",
            "(a|b){1,2}?b",
            "(b*a*)*$",
            "(?:^a|$|b)+",
            "((ab)|a(b?))*",
            "((a|b){2,3}){1,2}",
            "(a{1,2}?){2,}",
            "((a?){2}b?){0,2}",
            "(?:(a)|(b)){2,}?",
            "(a{2,})*",
            "(a+|b+){3,}",
        ];
        let inputs = words(&['a', 'b', 'é', '\n'], 4);
        let mut comparisons = 0;
        for flags in ["", "m", "s", "i"] {
            for source in sources {
                let program = pattern(Profile::Xpath31, source, flags);
                assert!(!program.links.backreferences, "{source}");
                for input in &inputs {
                    for (start, _) in input.char_indices().chain([(input.len(), ' ')]) {
                        let thread = Vm::threads(&program, input, Limits::new())
                            .find_from(start)
                            .unwrap();
                        let backtrack = Vm::backtracking(&program, input, Limits::new())
                            .find_from(start)
                            .unwrap();
                        assert_eq!(thread, backtrack, "{source:?} {flags:?} {input:?} {start}");
                        comparisons += 1;
                    }
                }
            }
        }
        assert_eq!(comparisons, 4 * sources.len() * 1593);
    }

    #[test]
    fn single_character_greedy_runs_match_the_general_repetition() {
        // A capturing body takes the general per-iteration path; the bare set
        // takes the single pending run. Their whole-match spans must agree.
        let pairs = [
            ("a*ab", "(a)*ab"),
            ("^a{2,3}a", "^(a){2,3}a"),
            (".+b", "(.)+b"),
            ("[ab]{0,2}b$", "([ab]){0,2}b$"),
            ("é*𐀀", "(é)*𐀀"),
            ("a{3,}", "(a){3,}"),
            ("b+a*b", "(b)+(a)*b"),
            ("^[^b]*$", "^([^b])*$"),
            ("a{0}b", "(a){0}b"),
            (".*b", "(.)*b"),
            ("[ab]*é", "([ab])*é"),
            ("a+b+a", "(a)+(b)+a"),
            ("a*?ab", "(a)*?ab"),
            ("é{1,2}é", "(é){1,2}é"),
        ];
        let inputs = words(&['a', 'b', 'é', '𐀀'], 5);
        for profile in [Profile::Xpath20, Profile::Xpath31] {
            for (run, general) in pairs {
                let run = pattern(profile, run, "");
                let general = pattern(profile, general, "");
                for input in &inputs {
                    assert_eq!(
                        run.find(input, Limits::new())
                            .unwrap()
                            .and_then(|captures| captures.get(0)),
                        general
                            .find(input, Limits::new())
                            .unwrap()
                            .and_then(|captures| captures.get(0)),
                        "{profile:?} {:?} {input:?}",
                        run.source()
                    );
                }
            }
        }
    }

    #[test]
    fn skipped_search_starts_never_change_the_first_match() {
        // A leading empty group hides every lead fact, so the same pattern
        // searches every start position; the spans must not differ.
        let sources = [
            ".*b", "a*ab", ".+ba", "[ab]*b$", "^ab", "^a|^b", "a|b", "ab|ba", "(a|b)a", "^b", "^$",
            "[^b]*b", "é*𐀀", "(a)\\1", "a+?b", ".*?b", "b{2,}", "[ab]+$", "(ab|a)*b", "\\n.*a",
        ];
        let inputs = words(&['a', 'b', 'é', '\n'], 5);
        for flags in ["", "m", "s", "i"] {
            for source in sources {
                let fast = pattern(Profile::Xpath31, source, flags);
                let slow = pattern(Profile::Xpath31, &format!("(?:){source}"), flags);
                assert_eq!(slow.lead.first, Lead::Any);
                assert!(slow.lead.run.is_none());
                for input in &inputs {
                    assert_eq!(
                        fast.find(input, Limits::new()).unwrap(),
                        slow.find(input, Limits::new()).unwrap(),
                        "{source:?} {flags:?} {input:?}"
                    );
                }
            }
        }
        assert!(matches!(
            pattern(Profile::Xpath31, "^a|^b", "").lead.first,
            Lead::Start
        ));
        assert!(matches!(
            pattern(Profile::Xpath31, "ab|ba", "").lead.first,
            Lead::Set(_)
        ));
        assert!(pattern(Profile::Xpath31, ".*b", "").lead.run.is_some());
        assert!(pattern(Profile::Xpath31, "(.*)b", "").lead.run.is_none());
        assert!(pattern(Profile::Xpath31, ".{0,9}b", "").lead.run.is_none());
    }

    #[test]
    fn flattened_class_unions_match_like_their_alternatives() {
        // A class union is flattened into sorted ranges; the same members
        // written as an alternative of atoms keep per-atom evaluation.
        for (class, alternative, flags) in [
            ("^[a-z0-9]$", "^(?:[a-z]|[0-9])$", ""),
            ("^[a-cK_]$", "^(?:[a-c]|K|_)$", "i"),
            ("^[\u{3b8}x-z-]$", "^(?:\u{3b8}|[x-z]|-)$", "i"),
            ("^[\n\t ]$", "^(?:\n|\t| )$", ""),
        ] {
            let flat = pattern(Profile::Xpath31, class, flags);
            assert!(
                (0..flat.sets.len()).any(|set| flat.flat.get(set).is_some()),
                "{class}"
            );
            let atoms = pattern(Profile::Xpath31, alternative, flags);
            for point in (0..0x2200).chain(0x1_0000..0x1_0010) {
                let Some(ch) = char::from_u32(point) else {
                    continue;
                };
                let text = ch.to_string();
                assert_eq!(
                    flat.is_match(&text, Limits::new()).unwrap(),
                    atoms.is_match(&text, Limits::new()).unwrap(),
                    "{class} {flags} U+{point:04X}"
                );
            }
        }
    }

    #[test]
    fn ordinary_large_inputs_are_admitted_at_the_production_defaults() {
        let limits = Limits::new();
        let letters = "abcdefghijklmnopqrstuvwxyz".repeat(5000);
        assert_eq!(letters.len(), 130_000);
        let mixed = "aé𐀀 ".repeat(30_000);
        for (source, input) in [
            (".*", letters.as_str()),
            (".*", mixed.as_str()),
            ("^.*$", letters.as_str()),
            ("[a-z]+", letters.as_str()),
            ("^[a-z]+$", letters.as_str()),
            ("[a-z]*z", letters.as_str()),
            (r"\S+", letters.as_str()),
        ] {
            for profile in [Profile::Xpath20, Profile::Xpath31] {
                let span = pattern(profile, source, "")
                    .find(input, limits)
                    .unwrap_or_else(|error| panic!("{profile:?} {source}: {error}"))
                    .and_then(|captures| captures.get(0));
                assert_eq!(span, Some(0..input.len()), "{profile:?} {source}");
            }
        }
        // A multi-megabyte literal search, absent and present at the end.
        let mut haystack = "lorem ipsum dolor sit amet ".repeat(320_000);
        assert!(haystack.len() > 8 * 1024 * 1024);
        let needle = pattern(Profile::Xpath31, "needle", "");
        assert!(!needle.is_match(&haystack, limits).unwrap());
        haystack.push_str("needle");
        assert_eq!(
            needle.find(&haystack, limits).unwrap().unwrap().get(0),
            Some(haystack.len() - 6..haystack.len())
        );
        // Searches whose first character is an alternative's, an anchor, or a
        // leading unbounded run stay linear over the same multi-megabyte text.
        let lines = "lorem ipsum\ndolor sit amet\n".repeat(300_000);
        assert!(lines.len() > 7 * 1024 * 1024);
        for (source, flags, found) in [
            ("needle|haystack", "", None),
            ("^needle", "", None),
            ("^needle", "m", None),
            (".*needle.*", "", None),
            (".*needle", "s", None),
            ("^dolor", "m", Some(12..17)),
            ("[a-z]+ sit", "", Some(12..21)),
        ] {
            for profile in [Profile::Xpath20, Profile::Xpath31] {
                let span = pattern(profile, source, flags)
                    .find(&lines, limits)
                    .unwrap_or_else(|error| panic!("{profile:?} {source} {flags}: {error}"))
                    .and_then(|captures| captures.get(0));
                assert_eq!(span, found, "{profile:?} {source} {flags}");
            }
        }
        // A 30,000-character literal pattern compiles and finds itself.
        let literal: String = ('a'..='z').cycle().take(30_000).collect();
        for profile in [Profile::Xpath20, Profile::Xpath31] {
            for flags in ["", "q", "i"] {
                if profile == Profile::Xpath20 && flags == "q" {
                    continue;
                }
                let program = compile(profile, &literal, flags, limits)
                    .unwrap_or_else(|error| panic!("{profile:?} {flags:?}: {error}"));
                let input = format!("--{literal}--");
                assert_eq!(
                    program.find(&input, limits).unwrap().unwrap().get(0),
                    Some(2..2 + literal.len()),
                    "{profile:?} {flags:?}"
                );
            }
        }
        // The largest admitted source is an ordinary literal too.
        let largest: String = ('a'..='z')
            .cycle()
            .take(usize::try_from(limits.limit(Resource::PatternBytes)).unwrap())
            .collect();
        for flags in ["", "i", "x", "q"] {
            for profile in [Profile::Xpath20, Profile::Xpath31] {
                if profile == Profile::Xpath20 && flags == "q" {
                    continue;
                }
                let program = compile(profile, &largest, flags, limits)
                    .unwrap_or_else(|error| panic!("{profile:?} {flags:?}: {error}"));
                assert!(program.is_match(&largest, limits).unwrap());
            }
        }
        // One byte more is refused by its size alone, before any parsing.
        let oversized = format!("{largest}a");
        assert!(matches!(
            compile(Profile::Xpath31, &oversized, "", limits),
            Err(Error::Resource(Refusal {
                resource: Resource::PatternBytes,
                ..
            }))
        ));
    }

    /// Deterministic prose of at least `bytes` bytes: lowercase words joined
    /// by single spaces, with no leading or trailing space.
    fn prose(bytes: usize) -> String {
        const WORDS: [&str; 10] = [
            "gamma", "graph", "beta", "rdf", "pattern", "shape", "delta", "alpha", "node", "sparql",
        ];
        let mut text = String::with_capacity(bytes + 16);
        let mut index = 0_usize;
        while text.len() < bytes {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(WORDS[(index * 7 + index / 3) % WORDS.len()]);
            index += 1;
        }
        text
    }

    /// Every capture span of a first match, or no match.
    type Spans = Option<Vec<Option<Range<usize>>>>;

    /// Every capture span of the compatibility engine's first match.
    fn compatibility(source: &str, input: &str) -> Spans {
        let compiled = crate::xsd_regex::compile(source, "").unwrap();
        let regex = compiled.as_regex();
        regex.captures(input).map(|found| {
            (0..regex.captures_len())
                .map(|group| found.get(group).map(|span| span.range()))
                .collect()
        })
    }

    /// The native match at the production defaults under every law that
    /// accepts `source`, and whether the thread machine took over.
    fn native(source: &str, input: &str) -> (Spans, bool) {
        let mut answer = None;
        for profile in [Profile::Xpath20, Profile::Xpath31] {
            let Ok(program) = compile(profile, source, "", Limits::new()) else {
                assert!(source.contains("(?:"), "{source}");
                continue;
            };
            let mut vm = Vm::new(&program, input, Limits::new());
            let found = vm
                .find_from(0)
                .unwrap_or_else(|error| panic!("{profile:?} {source}: {error}"))
                .map(|captures| captures.spans);
            if let Some((earlier, _)) = &answer {
                assert_eq!(earlier, &found, "{source}: the laws agree");
            }
            answer = Some((found, vm.is_threaded()));
        }
        answer.expect("a law accepts every fixture")
    }

    #[test]
    fn adversary_shapes_answer_like_the_compatibility_engine_at_the_production_defaults() {
        // Each shape was refused by the backtracking machine alone at a size
        // well below these: multiple unbounded runs at 8 and 55 KB of
        // non-matching prose, group repetition at 44 to 131 KB, and a word
        // repetition at 159 KB.
        let text = prose(1 << 20);
        let ab = "ab".repeat(1 << 19);
        let end = |text: &str| text.len();
        let last_word = text.rfind(' ').unwrap() + 1;
        let second_last = text[..last_word - 1].rfind(' ').unwrap() + 1;
        let first_node = text.find("node").unwrap();
        let first_alpha = text.find("alpha").unwrap();
        let zzz = format!("{text} zzz");
        let cases: [(&str, String, Spans); 14] = [
            ("node.*graph.*zzz", text.clone(), None),
            (
                "node.*graph.*zzz",
                zzz.clone(),
                Some(vec![Some(first_node..end(&zzz))]),
            ),
            ("alpha.*zzz", text.clone(), None),
            (
                "alpha.*zzz",
                zzz.clone(),
                Some(vec![Some(first_alpha..end(&zzz))]),
            ),
            (
                "^([a-z]+ ?)+$",
                text.clone(),
                Some(vec![Some(0..end(&text)), Some(last_word..end(&text))]),
            ),
            ("^([a-z]+ ?)+$", format!("{text}!"), None),
            (
                r"^(\w+\s)*\w+$",
                text.clone(),
                Some(vec![Some(0..end(&text)), Some(second_last..last_word)]),
            ),
            (r"^(\w+\s)*\w+$", format!("{text} "), None),
            (
                "^(a|b)*$",
                ab.clone(),
                Some(vec![Some(0..end(&ab)), Some(end(&ab) - 1..end(&ab))]),
            ),
            ("^(a|b)*$", format!("{ab}c"), None),
            (
                "^(ab)*$",
                ab.clone(),
                Some(vec![Some(0..end(&ab)), Some(end(&ab) - 2..end(&ab))]),
            ),
            ("^(ab)*$", format!("{ab}a"), None),
            ("^(?:ab)*$", ab.clone(), Some(vec![Some(0..end(&ab))])),
            ("^(?:ab)*$", format!("{ab}a"), None),
        ];
        for (source, input, expected) in &cases {
            let (found, threaded) = native(source, input);
            assert_eq!(&found, expected, "{source} over {} bytes", input.len());
            assert_eq!(
                found,
                compatibility(source, input),
                "{source}: compatibility"
            );
            // A match after unbounded runs is found by the backtracking
            // attempt; every other case was refused by that machine alone.
            if expected.is_none() || !source.contains(".*") {
                assert!(threaded, "{source}: the thread machine took over");
            }
        }
        // Group repetition and nested nullable repetition at the adversary's
        // whole-pattern shape, where only the whole match is compared: the
        // compatibility engine reports a different empty last iteration.
        for (input, expected) in [
            (format!("{}b", "a".repeat(40)), Some(0..41)),
            ("a".repeat(40), Some(0..40)),
            (format!("{}c", "a".repeat(40)), None),
            (format!("{}b", "a".repeat(100_000)), Some(0..100_001)),
        ] {
            let source = "^(a|aa)*$|^(a*)*b$";
            let (found, _) = native(source, &input);
            let whole = found.map(|spans| spans[0].clone().unwrap());
            assert_eq!(whole, expected, "{source} over {}", input.len());
            assert_eq!(
                whole,
                compatibility(source, &input).map(|spans| spans[0].clone().unwrap()),
                "{source} over {}",
                input.len()
            );
        }
        let (found, threaded) = native("^(a|aa)*$|^(a*)*b$", &format!("{}b", "a".repeat(40)));
        // The empty second iteration of (a*)* is its last, by the progress rule.
        assert_eq!(found, Some(vec![Some(0..41), None, Some(40..40)]));
        assert!(threaded);
    }

    #[test]
    fn adversary_shapes_scale_linearly_well_beyond_the_adversarys_sizes() {
        // Four to eight mebibytes at the production defaults, answered by the
        // thread machine in time linear in the input.
        let text = prose(4 << 20);
        let ab = "ab".repeat(4 << 20);
        let a = "a".repeat(4 << 20);
        for (source, input, matched) in [
            ("node.*graph.*zzz", text.as_str(), false),
            ("alpha.*zzz", text.as_str(), false),
            ("^([a-z]+ ?)+$", text.as_str(), true),
            (r"^(\w+\s)*\w+$", text.as_str(), true),
            ("^(a|b)*$", ab.as_str(), true),
            ("^(ab)*$", ab.as_str(), true),
            ("^(?:ab)*$", ab.as_str(), true),
            ("^(a|aa)*$", a.as_str(), true),
            ("(a|aa)*b", a.as_str(), false),
        ] {
            let (found, threaded) = native(source, input);
            assert_eq!(found.is_some(), matched, "{source} over {}", input.len());
            assert!(threaded, "{source}");
        }
    }

    #[test]
    fn formerly_exponential_shapes_answer_and_backreference_blowups_still_refuse() {
        let limits = Limits::new();
        let forty = "a".repeat(40);
        for source in ["(a|a)*b", "(a|aa)*b", "(a*)*b", "((a+)+)+b"] {
            // Refused by the backtracking machine alone, answered by the
            // thread machine as the compatibility engine answers.
            let program = pattern(Profile::Xpath31, source, "");
            assert!(
                matches!(
                    Vm::backtracking(&program, &forty, limits).find_from(0),
                    Err(Error::Resource(_))
                ),
                "{source}"
            );
            for (input, expected) in [
                (forty.clone(), false),
                (format!("{forty}b"), true),
                ("a".repeat(100_000), false),
                (format!("{}b", "a".repeat(100_000)), true),
            ] {
                assert_eq!(
                    program.is_match(&input, limits).unwrap(),
                    expected,
                    "{source} over {}",
                    input.len()
                );
                assert_eq!(
                    crate::xsd_regex::compile(source, "")
                        .unwrap()
                        .is_match(&input),
                    expected
                );
            }
        }
        // A backreference reads captured text, so its program keeps the
        // backtracking machine, and exponential exploration is still refused.
        for (source, refused, neighbour) in [
            (r"^(a|a)*\1b", forty.clone(), format!("{forty}b")),
            (r"^(a|aa)*c\1$", forty.clone(), format!("{forty}ca")),
            (r"(a*)*\1b", forty.clone(), format!("{forty}b")),
        ] {
            let program = pattern(Profile::Xpath31, source, "");
            assert!(program.links.backreferences);
            assert!(
                matches!(
                    program.is_match(&refused, limits),
                    Err(Error::Resource(Refusal {
                        resource: Resource::MatchSteps
                            | Resource::MatchStates
                            | Resource::MatchSlots,
                        ..
                    }))
                ),
                "{source}"
            );
            assert!(program.is_match(&neighbour, limits).unwrap(), "{source}");
        }
    }

    /// The least bound of `resource` under which `run` succeeds, by bisection.
    fn requirement(
        run: &dyn Fn(Limits) -> Result<Option<Captures>, Error>,
        resource: Resource,
    ) -> u64 {
        let at = |limit| Limits::new().with(resource, limit);
        let (mut refused, mut admitted) = (0, Limits::new().limit(resource));
        assert!(run(at(admitted)).is_ok(), "{resource:?}");
        assert!(run(at(0)).is_err(), "{resource:?}");
        while admitted - refused > 1 {
            let middle = refused + (admitted - refused) / 2;
            if run(at(middle)).is_ok() {
                admitted = middle;
            } else {
                refused = middle;
            }
        }
        admitted
    }

    #[test]
    fn thread_machine_resources_admit_the_exact_requirement_and_refuse_one_less() {
        let words = prose(3000);
        let pairs = "ab".repeat(1500);
        let forty = "a".repeat(40);
        for (source, input) in [
            ("^([a-z]+ ?)+$", words.as_str()),
            ("node.*graph.*zzz", words.as_str()),
            ("^(a|b)*$", pairs.as_str()),
            ("(a|aa)*b", forty.as_str()),
            // Stored counts select the hashed control-state table.
            ("^(a?){3,5}(b{2,4}){2}$", "aabbbbbb"),
        ] {
            let program = pattern(Profile::Xpath31, source, "");
            let expected = Vm::threads(&program, input, Limits::new())
                .find_from(0)
                .unwrap();
            let threads = |limits| Vm::threads(&program, input, limits).find_from(0);
            let public = |limits| program.find(input, limits);
            for resource in [
                Resource::MatchSteps,
                Resource::MatchStates,
                Resource::MatchSlots,
            ] {
                for run in [&threads as &dyn Fn(_) -> _, &public] {
                    let required = requirement(run, resource);
                    let admitted = run(Limits::new().with(resource, required)).unwrap();
                    assert_eq!(admitted, expected, "{source} {resource:?}");
                    let refusal = run(Limits::new().with(resource, required - 1)).unwrap_err();
                    assert!(
                        matches!(
                            refusal,
                            Error::Resource(Refusal { resource: refused, limit, .. })
                                if refused == resource && limit == required - 1
                        ),
                        "{source} {resource:?}: {refusal}"
                    );
                }
            }
        }
    }

    #[test]
    fn huge_counted_repetitions_spend_finite_work_beside_admitted_neighbours() {
        let limits = Limits::new();
        // A nullable body repeated more than u64 times: every required empty
        // iteration is a distinct control state and spends fuel.
        let nullable = pattern(Profile::Xpath31, "(a?){18446744073709551616}", "");
        assert!(matches!(
            nullable.is_match("", limits),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchSteps | Resource::MatchStates,
                ..
            }))
        ));
        let thousand = pattern(Profile::Xpath31, "(a?){1000}", "");
        let captures = thousand.find("", limits).unwrap().unwrap();
        assert_eq!((captures.get(0), captures.get(1)), (Some(0..0), Some(0..0)));
        // A large counted group repeated from every start keeps one thread per
        // distinct count: unanchored, the counts of all live starts are kept
        // and the per-position state bound refuses. Anchored, one start keeps
        // one count, and the same repetition is admitted.
        let input = "ab".repeat(1 << 16);
        let unanchored = pattern(Profile::Xpath31, "(ab){1,100000}c", "");
        assert!(matches!(
            unanchored.is_match(&input, limits),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchStates | Resource::MatchSteps,
                ..
            }))
        ));
        let anchored = pattern(Profile::Xpath31, "^(ab){1,100000}$", "");
        let captures = anchored.find(&input, limits).unwrap().unwrap();
        assert_eq!(captures.get(0), Some(0..input.len()));
        assert_eq!(captures.get(1), Some(input.len() - 2..input.len()));
        assert!(
            !pattern(Profile::Xpath31, "^(ab){1,65535}$", "")
                .is_match(&input, limits)
                .unwrap()
        );
        assert!(
            pattern(Profile::Xpath31, "^(ab){65536}$", "")
                .is_match(&input, limits)
                .unwrap()
        );
        // A counted single-character run keeps its bound exactly.
        let letters = "a".repeat(100_001);
        for (source, expected) in [
            ("^a{100000}$", false),
            ("^a{100001}$", true),
            ("^a{1,100000}$", false),
            ("^(?:a|b){100001}$", true),
            ("^(?:a|b){100002,}$", false),
        ] {
            assert_eq!(
                pattern(Profile::Xpath31, source, "")
                    .is_match(&letters, limits)
                    .unwrap(),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn dated_dot_anchor_and_quoted_modes_have_exact_neighbors() {
        for profile in [Profile::Xpath20, Profile::Xpath31] {
            let dot = pattern(profile, ".", "");
            assert!(!dot.is_match("\n", Limits::new()).unwrap());
            assert_eq!(
                dot.is_match("\r", Limits::new()).unwrap(),
                profile == Profile::Xpath20
            );
            assert!(
                pattern(profile, ".", "s")
                    .is_match("\n", Limits::new())
                    .unwrap()
            );
            assert!(
                pattern(profile, "^a$", "m")
                    .is_match("a\n", Limits::new())
                    .unwrap()
            );
            assert!(
                !pattern(profile, "^a$", "")
                    .is_match("a\n", Limits::new())
                    .unwrap()
            );
            let start = pattern(profile, "^", "m");
            assert!(
                Vm::new(&start, "a\n", Limits::new())
                    .find_from(2)
                    .unwrap()
                    .is_none()
            );
            let end = pattern(profile, "$", "m");
            assert!(
                Vm::new(&end, "a\n", Limits::new())
                    .find_from(2)
                    .unwrap()
                    .is_none()
            );
        }
        assert_eq!(matched("^b$", "m", "a\nb\n").get(0), Some(2..3));
        let source = r" a.*[\";
        let quoted = pattern(Profile::Xpath31, source, "qsimx");
        assert_eq!(
            quoted.find(source, Limits::new()).unwrap().unwrap().get(0),
            Some(0..source.len())
        );
        assert!(!quoted.is_match("aZ", Limits::new()).unwrap());
    }

    #[test]
    fn full_case_variants_are_direct_and_do_not_fold_properties() {
        for (source, input, expected) in [
            ("θ", "ϑ", true),
            ("θ", "ϴ", true),
            ("ϑ", "ϴ", false),
            ("ϴ", "ϑ", false),
            ("ß", "ẞ", true),
            ("ß", "SS", false),
            ("K", "\u{212a}", true),
            ("[A-Z]", "a", true),
            ("[^A]", "a", false),
            ("[A-Z-[K]]", "\u{212a}", false),
            (r"\p{Lu}", "a", false),
            (r"\p{Lu}", "A", true),
            (r"\P{Lu}", "a", true),
        ] {
            assert_eq!(
                pattern(Profile::Xpath31, source, "i")
                    .is_match(input, Limits::new())
                    .unwrap(),
                expected,
                "{source:?} {input:?}"
            );
        }
        assert!(
            pattern(Profile::Xpath31, r"([A-Z])\1", "i")
                .is_match("Aa", Limits::new())
                .unwrap()
        );
        assert!(
            !pattern(Profile::Xpath31, r"(ϑ)\1", "i")
                .is_match("ϑϴ", Limits::new())
                .unwrap()
        );
    }

    #[test]
    fn ordered_alternatives_repetition_and_utf8_captures_are_exact() {
        let first = matched("(a|aa)(a?)", "", "aa");
        assert_eq!(first.get(0), Some(0..2));
        assert_eq!(first.get(1), Some(0..1));
        assert_eq!(first.get(2), Some(1..2));
        let greedy = matched("^(a+)(a*)$", "", "aaa");
        assert_eq!(greedy.get(1), Some(0..3));
        assert_eq!(greedy.get(2), Some(3..3));
        let reluctant = matched("^(a+?)(a*)$", "", "aaa");
        assert_eq!(reluctant.get(1), Some(0..1));
        assert_eq!(reluctant.get(2), Some(1..3));
        assert_eq!(matched("^(a|b)+$", "", "ab").get(1), Some(1..2));
        let utf8 = matched(r"(é)\1", "", "éé");
        assert_eq!(utf8.get(0), Some(0..4));
        assert_eq!(utf8.get(1), Some(0..2));
        assert_eq!(utf8.get(2), None);
        assert_eq!(utf8.len(), 2);
        assert!(!utf8.is_empty());
        assert_eq!(matched(r"^(a)?b\1$", "", "b").get(1), None);
        assert!(
            pattern(Profile::Xpath31, r"^(a)\12$", "")
                .is_match("aa2", Limits::new())
                .unwrap()
        );
        assert!(
            pattern(
                Profile::Xpath31,
                r"^(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)\12$",
                ""
            )
            .is_match("abcdefghijkll", Limits::new())
            .unwrap()
        );
        assert!(
            pattern(Profile::Xpath31, r"(a)(?:b)\1", "")
                .is_match("aba", Limits::new())
                .unwrap()
        );
    }

    #[test]
    fn nullable_and_arbitrary_size_repetitions_spend_finite_work() {
        assert_eq!(matched("^(a?){3}$", "", "").get(1), Some(0..0));
        assert_eq!(matched("(a?)*", "", "").get(1), Some(0..0));
        let nonnullable = pattern(Profile::Xpath31, "a{18446744073709551616}", "");
        assert!(!nonnullable.is_match("", Limits::new()).unwrap());
        let nullable = pattern(Profile::Xpath31, "(a?){18446744073709551616}", "");
        assert!(matches!(
            nullable.is_match("", Limits::new().with(Resource::MatchSteps, 200)),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchSteps,
                ..
            }))
        ));
    }

    #[test]
    fn work_state_and_all_live_storage_refusals_never_become_false() {
        let literal = pattern(Profile::Xpath31, "a", "");
        for input in ["a", "z"] {
            assert!(matches!(
                literal.is_match(input, Limits::new().with(Resource::MatchSteps, 0)),
                Err(Error::Resource(Refusal {
                    resource: Resource::MatchSteps,
                    ..
                }))
            ));
        }
        assert!(
            literal
                .is_match("a", Limits::new().with(Resource::MatchStates, 0))
                .unwrap()
        );
        let choice = pattern(Profile::Xpath31, "z|a", "");
        assert!(matches!(
            choice.is_match("a", Limits::new().with(Resource::MatchStates, 0)),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchStates,
                ..
            }))
        ));
        assert!(
            choice
                .is_match("a", Limits::new().with(Resource::MatchStates, 1))
                .unwrap()
        );
        assert!(
            literal
                .is_match("a", Limits::new().with(Resource::MatchSlots, 7))
                .unwrap()
        );
        assert!(matches!(
            literal.is_match("a", Limits::new().with(Resource::MatchSlots, 6)),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchSlots,
                ..
            }))
        ));
        assert!(
            choice
                .is_match("a", Limits::new().with(Resource::MatchSlots, 14))
                .unwrap()
        );
        assert!(matches!(
            choice.is_match("a", Limits::new().with(Resource::MatchSlots, 13)),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchSlots,
                ..
            }))
        ));
    }

    #[test]
    fn search_has_one_budget_and_each_execution_has_fresh_fuel() {
        let program = pattern(Profile::Xpath31, "a", "");
        let input = "bbbbbbba";
        let mut vm = Vm::new(&program, input, Limits::new());
        assert_eq!(vm.find_from(0).unwrap().unwrap().get(0), Some(7..8));
        let steps = vm.budget().used(Resource::MatchSteps);
        let exact = Limits::new().with(Resource::MatchSteps, steps);
        assert!(program.is_match(input, exact).unwrap());
        assert!(program.is_match(input, exact).unwrap());
        assert!(matches!(
            program.is_match(input, exact.with(Resource::MatchSteps, steps - 1)),
            Err(Error::Resource(Refusal {
                resource: Resource::MatchSteps,
                ..
            }))
        ));
        assert!(matches!(
            program.is_match(input, exact.with(Resource::ProgramNodes, 0)),
            Err(Error::Resource(Refusal {
                resource: Resource::ProgramNodes,
                ..
            }))
        ));
    }

    #[test]
    fn deep_capturing_groups_match_and_drop_on_a_bounded_native_stack() {
        purrdf_stack::on_stack(256 * 1024, || {
            let source = format!("{}é{}", "(".repeat(6000), ")".repeat(6000));
            let program = pattern(Profile::Xpath31, &source, "");
            let captures = program.find("é", Limits::new()).unwrap().unwrap();
            assert_eq!(captures.len(), 6001);
            assert_eq!(captures.get(6000), Some(0..2));
            drop(captures);
            drop(program);
        })
        .unwrap();
    }
}
