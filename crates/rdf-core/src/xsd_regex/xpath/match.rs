// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ordered native matching, with one request's work fuel and live storage.
//!
//! A program without backreferences first runs the backtracking machine below
//! under an allowance linear in the input it examines. Past that allowance,
//! the capture-free set machine in [`super::sets`] decides whether a match
//! exists, and marks where matches start, keeping each counted repetition's
//! live counts as one set; the thread machine in [`super::pike`] then walks
//! the leftmost start as one thread, guided by the set machine's reverse
//! states, for the priority match and its captures. A program with a
//! backreference runs on the backtracking machine alone, whose continuations
//! depend on captured text and so cannot be merged.
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

use super::compile::{
    CompiledPattern, Count, Follow, Lead, Node, Set, case_variants, case_variants_for,
};
use super::pike::{Pike, copy_steps};
use super::{Budget, Error, Law, Limits, Profile, Refusal, Resource, unicode_tables};

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

/// Captures retaining the original caller's physical account.
pub type OwnedCaptures<S> = super::OwnedPatternValue<Option<Captures>, S>;

fn owned_execution<S: purrdf_lex::allocation::Admission, T>(
    program: &CompiledPattern,
    input: &str,
    limits: Limits,
    mut storage: S,
    leg: impl FnOnce(&mut Vm<'_>) -> Result<(T, usize), Error>,
) -> Result<super::OwnedPatternValue<T, S>, super::OwnedPatternError<S>> {
    let verdict = (|| {
        program.admit(limits)?;
        let mut vm = Vm::with_storage(program, input, limits, &mut storage);
        leg(&mut vm)
    })();
    // Every machine buffer is destroyed before its account is shrunk to the
    // output that actually survives this operation.
    match verdict {
        Ok((value, bytes)) => match storage.resize(bytes) {
            Ok(()) => Ok(super::OwnedPatternValue::new(value, storage)),
            Err(error) => {
                drop(value);
                Err(super::OwnedPatternValue::new(
                    Error::Storage(error),
                    storage,
                ))
            }
        },
        Err(error) => Err(super::OwnedPatternValue::new(error, storage)),
    }
}

impl CompiledPattern {
    /// Match through the unchanged native machines with physical admission.
    ///
    /// # Errors
    /// Retains the original typed account on language, work, layout, allocator
    /// or physical-capacity refusal. No partial match is published.
    pub fn is_match_with_storage<S: purrdf_lex::allocation::Admission>(
        &self,
        input: &str,
        limits: Limits,
        storage: S,
    ) -> Result<super::OwnedPatternValue<bool, S>, super::OwnedPatternError<S>> {
        owned_execution(self, input, limits, storage, |vm| {
            Ok((vm.is_match_from(0)?, 0))
        })
    }

    /// Find ordered captures while retaining their exact buffer admission.
    ///
    /// # Errors
    /// Preserves the same native verdict and original physical account.
    pub fn find_with_storage<S: purrdf_lex::allocation::Admission>(
        &self,
        input: &str,
        limits: Limits,
        storage: S,
    ) -> Result<OwnedCaptures<S>, super::OwnedPatternError<S>> {
        owned_execution(self, input, limits, storage, |vm| {
            let found = vm.find_from(0)?;
            let bytes = found.as_ref().map_or(Ok(0), |captures| {
                std::alloc::Layout::array::<Option<Range<usize>>>(captures.spans.capacity())
                    .map(|layout| layout.size())
                    .map_err(|_| Error::Storage(purrdf_lex::allocation::StorageError::SizeOverflow))
            })?;
            Ok((found, bytes))
        })
    }

    /// Whether the input contains a match under this program's dated law.
    ///
    /// # Errors
    ///
    /// Current artifact admission, matcher work and live-state/storage failures
    /// are typed operational errors, never negative matches.
    // Out of line: a host evaluator that dispatches to the matcher must not
    // absorb the matcher's frames into its own, which every nesting level pays.
    #[inline(never)]
    pub fn is_match(&self, input: &str, limits: Limits) -> Result<bool, Error> {
        self.admit(limits)?;
        Vm::new(self, input, limits).is_match_from(0)
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
    // Out of line: a host evaluator that dispatches to the matcher must not
    // absorb the matcher's frames into its own, which every nesting level pays.
    #[inline(never)]
    pub fn find(&self, input: &str, limits: Limits) -> Result<Option<Captures>, Error> {
        self.admit(limits)?;
        Vm::new(self, input, limits).find_from(0)
    }
}

/// Operands [`Backtrack::may_begin`] follows to the first character of an
/// alternative before it leaves the alternative pending anyway.
const BEGIN_DEPTH: usize = 8;

/// Counts a repetition may distinguish before the backtracking machine keeps
/// an attempt that follows it in linear time ([`Ctx::large`]): the set
/// machine's states repeat within a few times this many positions.
pub(super) const LARGE_COUNT: u64 = 1024;

/// Iterations of a greedy run between two checks of an attempt's allowance.
const RUN_CHECK: u64 = 64;

/// Work a backtracking attempt may spend per search start it has passed, when
/// the thread machine can take over the program.
const ATTEMPT_START_STEPS: u64 = 16;

/// Work such an attempt may spend per input byte it has examined: one
/// greedy single-character run's own cost. A longer attempt costs more than
/// the linear-time machines' reverse scan and walk.
const ATTEMPT_BYTE_STEPS: u64 = 2;

/// Work such an attempt may spend per program node, wherever it is: on a
/// short input the attempt is cheaper than setting up the linear-time
/// machines.
const ATTEMPT_NODE_STEPS: u64 = 64;

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
    /// Work credited as the attempt's own linear progress: what it spent
    /// beyond [`ATTEMPT_BYTE_STEPS`] per byte the first time it examined each
    /// byte, either in a greedy run or with no alternative pending.
    credited: u64,
    /// The position below which every byte has been consumed once.
    scanned: usize,
    /// The position below which every byte has ended a greedy run once.
    ended: usize,
    /// The steps spent when the current stretch without a pending
    /// alternative last reached [`Self::scanned`], or None outside such a
    /// stretch.
    quiet: Option<u64>,
    /// Whether the program has a large count that can decide a choice on
    /// this input: only then can the set machine cost far more per byte
    /// than one path of this attempt, so only then are quiet stretches
    /// credited.
    large: bool,
}

impl Attempt {
    fn grant(&mut self, excess: u64) {
        self.credited += excess;
        self.allowed = self.allowed.saturating_add(excess);
    }

    /// Credit what a greedy run spent comparing the character at `position`
    /// beyond a plain comparison, the first time any run consumes it. The
    /// comparison that ends a run at a byte nothing has consumed yet is
    /// credited whole, once per byte: the byte's own allowance is left for
    /// the path that consumes it next.
    #[inline]
    fn credit(&mut self, position: usize, spent: u64, matched: bool) {
        if position < self.scanned {
            return;
        }
        if matched {
            self.scanned = position + 1;
            self.grant(spent.saturating_sub(ATTEMPT_BYTE_STEPS));
        } else if position >= self.ended {
            self.ended = position + 1;
            self.grant(spent);
        }
    }

    /// Account a greedy run that began with `entry` steps spent, examined
    /// input up to `scanned` before it, and ended with `spent`: the run
    /// credits its own work, so it is left out of the current quiet stretch.
    /// When the run moved past every byte examined before it, the stretch's
    /// work before the run was the path's own progress to it, and is
    /// credited.
    fn exclude(&mut self, entry: u64, scanned: usize, spent: u64) {
        let Some(mark) = self.quiet else {
            return;
        };
        if self.scanned > scanned {
            self.grant(entry - mark);
            self.quiet = Some(spent);
        } else {
            self.quiet = Some(mark + (spent - entry));
        }
    }

    /// Whether `spent`, at search start `start`, exceeds the allowance.
    ///
    /// While no alternative is pending (`quiet` is `Some(true)`), the attempt
    /// follows one path that never returns to input it has examined: the
    /// first time such a stretch moves past every byte examined so far, what
    /// it spent beyond [`ATTEMPT_BYTE_STEPS`] per new byte is credited. A path
    /// that later re-examines input, from a resumed alternative or a later
    /// start, earns no credit, so the credit is at most linear in the input.
    /// A greedy run's scan back over its stops (`quiet` is None) leaves the
    /// stretch as it is.
    #[inline]
    fn exceeded(
        &mut self,
        start: usize,
        position: usize,
        spent: u64,
        quiet: Option<bool>,
    ) -> Option<Refusal> {
        self.furthest = self.furthest.max(position);
        let quiet = quiet.map(|quiet| quiet && self.large);
        match (quiet, self.quiet) {
            (Some(false), _) => self.quiet = None,
            (Some(true), None) => self.quiet = Some(spent),
            (Some(true), Some(mark)) if position > self.scanned => {
                let advanced = (position - self.scanned) as u64;
                self.grant(
                    (spent - mark).saturating_sub(ATTEMPT_BYTE_STEPS.saturating_mul(advanced)),
                );
                self.scanned = position;
                self.quiet = Some(spent);
            }
            (Some(true), Some(_)) if position < self.scanned => self.quiet = Some(spent),
            _ => {}
        }
        if spent - self.before <= self.allowed {
            return None;
        }
        self.recompute(start, spent)
    }

    #[cold]
    fn recompute(&mut self, start: usize, spent: u64) -> Option<Refusal> {
        let allowance = u128::from(ATTEMPT_START_STEPS) * (start - self.origin + 1) as u128
            + u128::from(ATTEMPT_BYTE_STEPS) * (self.furthest - self.origin) as u128
            + u128::from(self.program)
            + u128::from(self.credited);
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
/// are released, and the linear-time machines run the same search, and every
/// later one, on the remaining fuel: the set machine answers whether a match
/// exists, or marks every match start in one reverse scan, and the thread
/// machine walks the first marked start for the match and its captures. The
/// attempt's work stays charged. Every machine reports the same match, so the
/// attempt changes only the cost.
pub(super) struct Vm<'a> {
    machine: Machine<'a>,
}

// One machine lives on the stack for one execution; boxing the backtracking
// machine's inline work lists would add an allocation to every match.
#[allow(clippy::large_enum_variant)]
enum Machine<'a> {
    Backtrack {
        machine: Backtrack<'a>,
        /// Whether the linear-time machines may take over an abandoned
        /// attempt.
        fallback: bool,
    },
    /// The thread machine alone, from every start.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    Pike(Pike<'a>),
    /// The set machine finds where a match starts; the thread machine walks
    /// that one start for the priority match and its captures.
    Linear(Pike<'a>),
    /// Only while the machines are exchanged.
    Exchanging,
}

impl<'a> Vm<'a> {
    pub(super) fn new(program: &'a CompiledPattern, input: &'a str, limits: Limits) -> Self {
        Self::with_context(Ctx::new(program, input, limits))
    }

    pub(super) fn with_storage(
        program: &'a CompiledPattern,
        input: &'a str,
        limits: Limits,
        storage: &'a mut dyn purrdf_lex::allocation::Admission,
    ) -> Self {
        Self::with_context(Ctx::with_budget(
            program,
            input,
            Budget::with_storage(limits, storage),
        ))
    }

    pub(super) fn with_budget(
        program: &'a CompiledPattern,
        input: &'a str,
        budget: Budget<'a>,
    ) -> Self {
        Self::with_context(Ctx::with_budget(program, input, budget))
    }

    /// Destroy all execution buffers before transferring the shared work fuel.
    pub(super) fn into_budget(self) -> Result<Budget<'a>, Error> {
        match self.machine {
            Machine::Backtrack { machine, .. } => Ok(machine.abandon()?.budget),
            #[cfg(all(test, not(target_arch = "wasm32")))]
            Machine::Pike(machine) => machine.into_budget(),
            Machine::Linear(machine) => machine.into_budget(),
            Machine::Exchanging => unreachable!("the machines are exchanged within one call"),
        }
    }

    fn with_context(ctx: Ctx<'a>) -> Self {
        if ctx.program.law.is_compatibility() {
            // Compatibility has no backreferences. Boolean matching needs no
            // capture histories; capture matching first certifies the leftmost
            // viable start, then keeps the compatibility thread-priority law.
            return Self {
                machine: Machine::Linear(Pike::new(ctx)),
            };
        }
        let fallback = !ctx.program.links.backreferences;
        Self {
            machine: Machine::Backtrack {
                machine: Backtrack::with_context(ctx),
                fallback,
            },
        }
    }

    pub(super) fn release_captures(&mut self, captures: Captures) -> Result<(), Error> {
        self.budget().release_vec(captures.spans)
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
                machine: Backtrack::with_context(Ctx::new(program, input, limits)),
                fallback: false,
            },
        }
    }

    /// The original general repetition path, without run/follow shortcuts.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    fn general_backtracking(program: &'a CompiledPattern, input: &'a str, limits: Limits) -> Self {
        let mut vm = Self::backtracking(program, input, limits);
        let Machine::Backtrack { machine, .. } = &mut vm.machine else {
            unreachable!("the backtracking constructor selects that machine");
        };
        machine.general = true;
        vm
    }

    /// The thread machine alone, for a backreference-free program.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(super) fn threads(program: &'a CompiledPattern, input: &'a str, limits: Limits) -> Self {
        assert!(!program.links.backreferences);
        Self {
            machine: Machine::Pike(Pike::new(Ctx::new(program, input, limits))),
        }
    }

    /// The set and thread machines without a backtracking attempt, for a
    /// backreference-free program.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(super) fn linear(program: &'a CompiledPattern, input: &'a str, limits: Limits) -> Self {
        assert!(!program.links.backreferences);
        Self {
            machine: Machine::Linear(Pike::new(Ctx::new(program, input, limits))),
        }
    }

    /// [`Self::linear`] with a reverse state kept every few bytes and covered
    /// ranges split after a few cells, so every search crosses kept states
    /// and splits.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(super) fn tight(program: &'a CompiledPattern, input: &'a str, limits: Limits) -> Self {
        assert!(!program.links.backreferences);
        let mut pike = Pike::new(Ctx::new(program, input, limits));
        pike.sets = super::sets::Sets::tight(3, 40);
        Self {
            machine: Machine::Linear(pike),
        }
    }

    /// Whether the linear-time machines have taken over this execution.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(super) const fn is_threaded(&self) -> bool {
        matches!(self.machine, Machine::Pike(_) | Machine::Linear(_))
    }

    /// This execution's work and storage accounting.
    pub(super) fn budget(&mut self) -> &mut Budget<'a> {
        match &mut self.machine {
            Machine::Backtrack { machine, .. } => &mut machine.ctx.budget,
            #[cfg(all(test, not(target_arch = "wasm32")))]
            Machine::Pike(machine) => &mut machine.ctx.budget,
            Machine::Linear(pike) => &mut pike.ctx.budget,
            Machine::Exchanging => unreachable!("the machines are exchanged within one call"),
        }
    }

    /// Whether a match starts at or after the UTF-8 offset `start`.
    pub(super) fn is_match_from(&mut self, start: usize) -> Result<bool, Error> {
        match &mut self.machine {
            Machine::Backtrack {
                machine,
                fallback: false,
            } => Ok(machine.find_from(start)?.is_some()),
            Machine::Backtrack {
                machine,
                fallback: true,
            } => match Self::attempt(machine, start) {
                Err(Error::Resource(_)) => {
                    self.hand_over()?;
                    self.is_match_from(start)
                }
                decided => Ok(decided?.is_some()),
            },
            #[cfg(all(test, not(target_arch = "wasm32")))]
            Machine::Pike(machine) => Ok(machine.find_from(start)?.is_some()),
            Machine::Linear(pike) => Self::linear_is_match(pike, start),
            Machine::Exchanging => unreachable!("the machines are exchanged within one call"),
        }
    }

    /// A backtracking search from `start` under the allowance of an attempt
    /// the linear-time machines can take over.
    fn attempt(machine: &mut Backtrack<'a>, start: usize) -> Result<Option<Captures>, Error> {
        let program = machine.ctx.program;
        let large = machine.large_counts()?;
        machine.attempt = Some(Attempt {
            origin: start,
            before: machine.ctx.budget.used(Resource::MatchSteps),
            furthest: start,
            program: ATTEMPT_NODE_STEPS
                .saturating_mul((program.nodes.len() + program.sets.len()) as u64),
            allowed: 0,
            credited: 0,
            scanned: start,
            ended: start,
            quiet: None,
            large,
        });
        let attempt = machine.find_from(start);
        machine.attempt = None;
        attempt
    }

    /// Abandon the backtracking machine for the linear-time machines, which
    /// inherit its accounting. Kept out of line, as are the linear-time
    /// machines' searches, so that an execution the backtracking machine
    /// decides runs only its own code.
    #[cold]
    #[inline(never)]
    fn hand_over(&mut self) -> Result<(), Error> {
        let Machine::Backtrack { machine, .. } =
            std::mem::replace(&mut self.machine, Machine::Exchanging)
        else {
            unreachable!("only the backtracking machine hands over");
        };
        self.machine = Machine::Linear(Pike::new(machine.abandon()?));
        Ok(())
    }

    /// Whether the linear-time machines find a match from `start`.
    #[cold]
    #[inline(never)]
    fn linear_is_match(pike: &mut Pike<'a>, start: usize) -> Result<bool, Error> {
        pike.sets.is_match(&mut pike.ctx, start)
    }

    /// The linear-time machines' first match from `start`.
    #[cold]
    #[inline(never)]
    fn linear_find(pike: &mut Pike<'a>, start: usize) -> Result<Option<Captures>, Error> {
        match pike.sets.first_start(&mut pike.ctx, start)? {
            Some(first) => pike.walk(first),
            None => Ok(None),
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
            } => match Self::attempt(machine, start) {
                Err(Error::Resource(_)) => {
                    self.hand_over()?;
                    self.find_from(start)
                }
                decided => decided,
            },
            #[cfg(all(test, not(target_arch = "wasm32")))]
            Machine::Pike(machine) => machine.find_from(start),
            Machine::Linear(pike) => Self::linear_find(pike, start),
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
    pub(super) budget: Budget<'a>,
    pub(super) live_slots: u128,
}

impl<'a> Ctx<'a> {
    pub(super) fn new(program: &'a CompiledPattern, input: &'a str, limits: Limits) -> Self {
        Self::with_budget(program, input, Budget::new(limits))
    }

    fn with_budget(program: &'a CompiledPattern, input: &'a str, budget: Budget<'a>) -> Self {
        Self {
            program,
            input,
            budget,
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
        let units = stack.len() as u64 + 1;
        self.budget
            .push_work(stack, value, Resource::MatchSlots, units)?;
        self.live_slots = required;
        Ok(())
    }

    /// The bounds of the repetition `repeat` that can decide a choice on this
    /// input.
    ///
    /// Under a dated law, no completed count of one entry into a repetition
    /// exceeds the input's bytes plus its minimum plus one: every iteration
    /// consumes a character, or is empty below the minimum, or is the one
    /// empty iteration that stops it once the minimum is met. A finite maximum
    /// above that bound
    /// never refuses another iteration, so the repetition makes exactly the
    /// choices it makes without a maximum, and the linear-time machines keep
    /// its counts as they keep those of a repetition without one: the counts
    /// from one below the minimum on make the same choices. The bound depends
    /// only on the input's length, so every state either machine computes or
    /// caches during one execution holds under it. Compatibility keeps a
    /// nullable body's finite maximum: empty iterations can distinguish its
    /// ordered capture histories even after the minimum is satisfied.
    pub(super) fn bounds(&self, repeat: usize) -> (Count, Option<Count>) {
        let Node::Repeat { body, min, max, .. } = self.program.nodes[repeat] else {
            unreachable!("only a repetition has bounds");
        };
        match (min, max) {
            (Count::Finite(low), Some(Count::Finite(high)))
                if (!self.program.law.is_compatibility()
                    || !self.program.links.nodes[body].nullable)
                    && u128::from(high) > self.input.len() as u128 + u128::from(low) + 1 =>
            {
                (min, None)
            }
            _ => (min, max),
        }
    }

    /// Whether the repetition `repeat` stores its count on this input and can
    /// distinguish more than [`LARGE_COUNT`] counts: the set machine keeps
    /// such counts exactly, so its states need not repeat for a long stretch
    /// of input, and every position may cost a full closure.
    pub(super) fn large(&self, repeat: usize) -> bool {
        let (min, max) = self.bounds(repeat);
        super::pike::counted(min, max)
            && match (min, max) {
                (_, Some(Count::Finite(max))) => max > LARGE_COUNT,
                (Count::Finite(min), _) => min > LARGE_COUNT,
                (Count::AboveU64, _) => true,
            }
    }

    /// The window of [`Self::bounds`]: the maximum less the minimum, or
    /// `u64::MAX` without a maximum that can bind.
    pub(super) fn window(&self, repeat: usize) -> u64 {
        match self.bounds(repeat) {
            (_, None) => u64::MAX,
            _ => self.program.window(repeat),
        }
    }

    /// Whether `position` satisfies `^` under the program's modes.
    pub(super) fn at_start(&self, position: usize) -> bool {
        position == 0
            || (self.program.modes.multiline
                && (position < self.input.len() || self.program.law.is_compatibility())
                && self.input[..position].ends_with('\n'))
    }

    /// Whether `position` satisfies `$` under the program's modes.
    pub(super) fn at_end(&self, position: usize) -> bool {
        if self.program.modes.multiline {
            self.input[position..].starts_with('\n')
                || (position == self.input.len()
                    && (self.program.law.is_compatibility() || !self.input.ends_with('\n')))
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
                    if start > self.input.len() {
                        return Ok(None);
                    }
                    if self.at_start(start) {
                        return Ok(Some(start));
                    }
                    if start == self.input.len() {
                        return Ok(None);
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
        self.budget.release_work(work)?;
        self.budget.release_work(values)?;
        Ok(value)
    }

    fn atomic_set(&mut self, set: Set, ch: char) -> Result<bool, Error> {
        Ok(match set {
            Set::Range { lo, hi, folded } => {
                if (lo..=hi).contains(&ch) {
                    true
                } else if folded {
                    let variants = case_variants_for(self.program.law, ch);
                    self.budget
                        .charge_wide(Resource::MatchSteps, variants.len() as u128)?;
                    variants
                        .iter()
                        .any(|&variant| (lo as u32..=hi as u32).contains(&variant))
                } else {
                    false
                }
            }
            Set::Table { ranges, folded } => {
                if purrdf_iri::terminals::in_ranges(ch as u32, ranges) {
                    true
                } else if folded {
                    let variants = case_variants_for(self.program.law, ch);
                    self.budget
                        .charge_wide(Resource::MatchSteps, variants.len() as u128)?;
                    variants
                        .iter()
                        .any(|&point| purrdf_iri::terminals::in_ranges(point, ranges))
                } else {
                    false
                }
            }
            Set::Block { lo, hi, folded } => {
                if (lo..=hi).contains(&(ch as u32)) {
                    true
                } else if folded {
                    let variants = case_variants_for(self.program.law, ch);
                    self.budget
                        .charge_wide(Resource::MatchSteps, variants.len() as u128)?;
                    variants.iter().any(|point| (lo..=hi).contains(point))
                } else {
                    false
                }
            }
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
                    || (ch != '\n'
                        && (self.program.law == Law::Dated(Profile::Xpath20) || ch != '\r'))
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

/// What a kept [`Scan`] says about a run from a position inside it.
#[derive(Debug, Clone, Copy)]
struct Known {
    /// Where the run reaches without scanning, after `count` characters.
    position: usize,
    count: u64,
    /// Where it reaches its nonzero minimum, if within the scan.
    floor: Option<usize>,
    /// Whether the character at `position`, or the end of the input, ends
    /// the run.
    exhausted: bool,
}

/// What one greedy run reached, kept in its body's [`Scan`].
#[derive(Debug, Clone, Copy)]
struct Reach {
    /// Where the run began, and whether it read the kept scan.
    origin: usize,
    reused: bool,
    /// Where it stopped scanning, after `count` characters.
    end: usize,
    count: u64,
    /// Whether the character at `end`, or the end of the input, ended it.
    exhausted: bool,
    /// Where it reached its minimum, if it did.
    floor: Option<usize>,
}

/// The body of a greedy single-character run.
#[derive(Debug, Clone, Copy)]
struct Body {
    /// The [`super::pike::Link::single`] node each iteration matches.
    single: usize,
    /// The outermost capture of the chain each iteration sets, if any.
    capture: Option<usize>,
    /// The captures in that chain.
    chain: u64,
    /// Whether the repetition or one enclosing it has a large count that can
    /// decide a choice on this input ([`Backtrack::counted`]).
    counted: bool,
}

/// The remaining stops of a greedy single-character run.
#[derive(Debug, Clone, Copy)]
struct Run {
    /// The shortest admissible stop.
    floor: usize,
    /// What a viable stop must satisfy at once.
    follow: Option<Follow>,
    /// Where the run began: a stop there completes no iteration.
    origin: usize,
    /// The outermost capture of the chain each iteration sets, if any.
    capture: Option<usize>,
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
    fn release(self, budget: &mut Budget<'_>) -> Result<(), Error> {
        let Self {
            actions, captures, ..
        } = self;
        budget.release_vec(captures)?;
        budget.release_work(actions)
    }

    fn into_captures(self, budget: &mut Budget<'_>) -> Result<Captures, Error> {
        let Self {
            actions, captures, ..
        } = self;
        budget.release_work(actions)?;
        Ok(Captures { spans: captures })
    }

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
    /// The extent of the last greedy run scanned from a new position.
    scan: Option<Scan>,
    /// Whether the program has a large count that can decide a choice on
    /// this input, once known.
    large: Option<bool>,
    /// The independently exercised original general repetition law.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    general: bool,
}

/// A character position inside a [`Scan`]: its byte offset and the
/// characters from the scan's origin before it.
#[derive(Debug, Clone, Copy)]
struct Cursor {
    at: usize,
    chars: u64,
}

/// The extent of a greedy run, kept so that a run of the same body that
/// begins inside it, as one does at each later search start, is not scanned
/// again: every character between its origin and its end matches the body.
///
/// Its cursors only move forward, by one character at a time, so the runs of
/// successive search starts cost one step each, not their length.
#[derive(Debug, Clone, Copy)]
struct Scan {
    /// The [`super::pike::Link::single`] node of the run's body.
    single: usize,
    /// How far every character from where the scan began matched.
    end: usize,
    /// The characters from where the scan began to `end`.
    chars: u64,
    /// Whether the character at `end`, or the end of the input, ended it.
    exhausted: bool,
    /// The start of the run that last used the scan, which only moves
    /// forward.
    start: Cursor,
    /// Where that run reached its minimum and its maximum, when it did.
    low: Option<Cursor>,
    high: Option<Cursor>,
}

impl<'a> Backtrack<'a> {
    fn with_context(ctx: Ctx<'a>) -> Self {
        Self {
            ctx,
            pending: WorkList::new(),
            attempt: None,
            scan: None,
            large: None,
            #[cfg(all(test, not(target_arch = "wasm32")))]
            general: false,
        }
    }

    fn optimize_runs(&self) -> bool {
        #[cfg(all(test, not(target_arch = "wasm32")))]
        let general = self.general;
        #[cfg(not(all(test, not(target_arch = "wasm32"))))]
        let general = false;
        !general
    }

    /// Release every pending state and hand over the request's accounting.
    fn abandon(self) -> Result<Ctx<'a>, Error> {
        let Self {
            mut ctx, pending, ..
        } = self;
        drop(pending);
        // A refused attempt may hold its current state and its set-evaluation
        // cells too; all of them are dropped with the machine.
        ctx.live_slots = 0;
        ctx.budget.release_abandoned_machine()?;
        Ok(ctx)
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
        self.ctx
            .budget
            .reserve_match(&mut captures, count, count as u64)?;
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
        self.ctx.budget.reserve_match(
            &mut captures,
            state.captures.len(),
            state.captures.len() as u64,
        )?;
        captures.extend(state.captures.iter().cloned());
        let mut actions = WorkList::new();
        for &action in state.actions.iter() {
            self.ctx.budget.push_work(
                &mut actions,
                action,
                Resource::MatchSlots,
                state.actions.len() as u64,
            )?;
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
        let units = self.pending.len() as u64 + 1;
        self.ctx
            .budget
            .push_work(&mut self.pending, state, Resource::MatchStates, units)
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
            let Some(stop) = self.stop(state.start, state.position, run)? else {
                self.ctx.live_slots -= state.slots();
                state.release(&mut self.ctx.budget)?;
                return Ok(None);
            };
            self.stop_at(&mut state, stop, run)?;
        }
        Ok(Some(state))
    }

    /// The longest stop at or below `position` whose next character can
    /// continue, spending one comparison per stop it passes over.
    ///
    /// An attempt the linear-time machines can take over is checked against
    /// its allowance at every stop it passes, so a run scanned back over the
    /// whole input is abandoned as soon as it is not linear.
    fn stop(
        &mut self,
        start: usize,
        mut position: usize,
        run: Run,
    ) -> Result<Option<usize>, Error> {
        let Some(follow) = run.follow else {
            return Ok(Some(position));
        };
        if matches!(follow, Follow::End) && !self.ctx.program.modes.multiline {
            // Only the end of the input satisfies `$`.
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            let end = self.ctx.input.len();
            return Ok((run.floor..=position).contains(&end).then_some(end));
        }
        loop {
            if self.follows(follow, position)? {
                return Ok(Some(position));
            }
            if position == run.floor {
                return Ok(None);
            }
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            if let Some(attempt) = &mut self.attempt
                && let Some(refusal) = attempt.exceeded(
                    start,
                    position,
                    self.ctx.budget.used(Resource::MatchSteps),
                    None,
                )
            {
                return Err(refusal.into());
            }
            position = self.previous(position);
        }
    }

    /// Whether the atom `follow` holds at `position`, spending the
    /// comparison that decides it.
    fn follows(&mut self, follow: Follow, position: usize) -> Result<bool, Error> {
        match follow {
            Follow::Set(set) => match self.ctx.input[position..].chars().next() {
                Some(ch) => self.ctx.set_matches(set, ch),
                None => Ok(false),
            },
            Follow::End => {
                self.ctx.budget.charge(Resource::MatchSteps, 1)?;
                Ok(self.ctx.at_end(position))
            }
        }
    }

    /// Continue at `stop`, leaving every shorter stop as one pending state.
    ///
    /// When the run has a following set, the next shorter viable stop is
    /// found now, by the scan resuming the pending state would make first,
    /// and no state is left pending when there is none. The pending state
    /// keeps the captures from before the run; this one takes its last
    /// iteration's.
    fn stop_at(&mut self, state: &mut State, stop: usize, run: Run) -> Result<(), Error> {
        state.position = stop;
        if stop > run.floor {
            let below = self.previous(stop);
            let shorter = if run.follow.is_some() {
                self.stop(state.start, below, run)?
            } else {
                Some(below)
            };
            if let Some(shorter) = shorter {
                let mut pending = self.fork(state)?;
                pending.position = shorter;
                pending.run = Some(run);
                self.enqueue(pending)?;
            }
        }
        if stop > run.origin {
            let last = self.previous(stop)..stop;
            let mut node = run.capture;
            while let Some(capture) = node {
                let Node::Capture { number, body } = self.ctx.program.nodes[capture] else {
                    break;
                };
                // The step the general repetition spends ending the capture.
                self.ctx.budget.charge(Resource::MatchSteps, 1)?;
                state.captures[number] = Some(last.clone());
                node = Some(body);
            }
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
                        Some(self.pending.is_empty()),
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
                        stale.release(&mut self.ctx.budget)?;
                    }
                    debug_assert_eq!(self.ctx.live_slots, 0);
                    return state.into_captures(&mut self.ctx.budget).map(Some);
                }
                self.ctx.live_slots -= state.slots();
                state.release(&mut self.ctx.budget)?;
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
                if !self.optimize_runs() || self.may_begin(right, state.position)? {
                    let mut alternative = self.fork(state)?;
                    self.action(&mut alternative, Action::Node(right))?;
                    self.enqueue(alternative)?;
                }
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
        if self.optimize_runs() && greedy && count == 0 && self.ctx.program.links.nodes[body].run {
            let Node::Repeat { follow, .. } = self.ctx.program.nodes[node] else {
                unreachable!("the repetition was matched above");
            };
            let capture =
                matches!(self.ctx.program.nodes[body], Node::Capture { .. }).then_some(body);
            let (mut single, mut chain) = (body, 0);
            while let Node::Capture { body, .. } = self.ctx.program.nodes[single] {
                // The step each capture of the general repetition spends.
                self.ctx.budget.charge(Resource::MatchSteps, 1)?;
                (single, chain) = (body, chain + 1);
            }
            let run = Body {
                single,
                capture,
                chain,
                counted: self.counted(node)?,
            };
            return self.greedy_run(state, run, min, max, follow);
        }
        let (can_stop, can_repeat) = decide(min, max, count, stalled);
        if !can_repeat {
            return Ok(can_stop);
        }
        if can_stop {
            if self.optimize_runs()
                && greedy
                && let Node::Repeat {
                    follow: Some(follow),
                    ..
                } = self.ctx.program.nodes[node]
                && can_repeat
                && !self.follows(follow, state.position)?
            {
                // The stop fails at once when resumed, before it sets any
                // capture: only the next iteration is explored.
                self.iteration(state, node, body, count)?;
                return Ok(true);
            }
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

    /// A greedy repetition whose every iteration consumes exactly one
    /// character, without a pending state per iteration.
    ///
    /// The body is a character set, or a choice between such bodies, whose
    /// alternatives that match a character all continue identically, under a
    /// chain of captures that each iteration sets to its character. The
    /// alternatives therefore differ only in where the run stops, and the
    /// captures of a stop are those of its last iteration. The run is scanned
    /// once; a single pending state then stands for every shorter admissible
    /// stop, longest first, which is the order the general repetition visits.
    /// A run of the same body that begins inside the last scan, as one does at
    /// each later search start, reads that scan instead ([`Scan`]).
    ///
    /// When a bound of the repetition, or of one enclosing it, can decide a
    /// choice on this input, the set machine keeps those counts exactly, so
    /// its states need not repeat and it may cost far more per byte than this
    /// scan. The first scan of each input byte by such a run is linear work of
    /// the run itself, so what it spends beyond one plain character
    /// comparison is credited to an attempt the linear-time machines can take
    /// over. Any other run is left to that allowance: the set machine decides
    /// it from repeated states.
    fn greedy_run(
        &mut self,
        state: &mut State,
        body: Body,
        min: Count,
        max: Option<Count>,
        follow: Option<Follow>,
    ) -> Result<bool, Error> {
        let Body {
            single,
            capture,
            chain,
            counted,
        } = body;
        let min = match min {
            Count::Finite(min) => Some(min),
            Count::AboveU64 => None,
        };
        let max = match max {
            Some(Count::Finite(max)) => max,
            None | Some(Count::AboveU64) => u64::MAX,
        };
        let input = self.ctx.input;
        let origin = state.position;
        let entry = self.ctx.budget.used(Resource::MatchSteps);
        let scanned = self.attempt.map_or(0, |attempt| attempt.scanned);
        let mut floor = (min == Some(0)).then_some(state.position);
        let mut count = 0_u64;
        // Whether the character at the position, or the end of the input,
        // ends the run.
        let mut exhausted = false;
        // Only a run whose counts the set machine keeps exactly is worth
        // keeping: any other is decided by that machine from repeated states.
        let known = if counted {
            self.known(single, origin, min, max)?
        } else {
            None
        };
        if let Some(known) = known {
            state.position = known.position;
            count = known.count;
            floor = floor.or(known.floor);
            exhausted = known.exhausted;
        }
        while count < max && !exhausted {
            let before = self.ctx.budget.used(Resource::MatchSteps);
            // The same spend that precedes every general repetition count.
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            let Some(ch) = input[state.position..].chars().next() else {
                exhausted = true;
                break;
            };
            let matched = self.single_matches(single, ch)?;
            if let Some(attempt) = &mut self.attempt {
                let used = self.ctx.budget.used(Resource::MatchSteps);
                if counted {
                    attempt.credit(state.position, used - before, matched);
                }
                // A run is checked against the allowance as it scans, so a
                // run that costs more than its credit is abandoned early.
                if count.is_multiple_of(RUN_CHECK)
                    && let Some(refusal) = attempt.exceeded(state.start, state.position, used, None)
                {
                    return Err(refusal.into());
                }
            }
            if !matched {
                exhausted = true;
                break;
            }
            state.position += ch.len_utf8();
            count += 1;
            if Some(count) == min {
                floor = Some(state.position);
            }
        }
        if counted {
            self.remember(
                single,
                Reach {
                    origin,
                    reused: known.is_some(),
                    end: state.position,
                    count,
                    exhausted,
                    floor,
                },
                min,
                max,
            );
        }
        let looped = self.ctx.budget.used(Resource::MatchSteps);
        let decided = self.run_stop(state, floor, follow, origin, capture);
        if let Some(attempt) = &mut self.attempt {
            let spent = self.ctx.budget.used(Resource::MatchSteps);
            attempt.exclude(entry, scanned, spent);
            // The scan for the first stop and the next shorter one, and the
            // captures, for a run that moved past every byte examined before
            // it: at most one comparison back over each new byte.
            if attempt.scanned > scanned {
                let new = (attempt.scanned - scanned) as u64;
                attempt.grant((spent - looped).min(chain + 2 + 2 * ATTEMPT_BYTE_STEPS * new));
            }
        }
        decided
    }

    /// What the kept scan of the body `single` says about a run of it from
    /// `origin`, moving the scan's cursors there. None when the scan holds no
    /// character from `origin` on, or no cursor where the run needs one.
    fn known(
        &mut self,
        single: usize,
        origin: usize,
        min: Option<u64>,
        max: u64,
    ) -> Result<Option<Known>, Error> {
        let Some(mut scan) = self.scan else {
            return Ok(None);
        };
        // A run from the scan's end gains nothing from it.
        if scan.single != single || origin < scan.start.at || origin >= scan.end {
            return Ok(None);
        }
        self.ctx.budget.charge(Resource::MatchSteps, 1)?;
        let input = self.ctx.input;
        let budget = &mut self.ctx.budget;
        let mut step = |cursor: &mut Cursor| -> Result<(), Error> {
            budget.charge(Resource::MatchSteps, 1)?;
            cursor.at += input[cursor.at..]
                .chars()
                .next()
                .expect("a cursor inside a scan precedes its end")
                .len_utf8();
            cursor.chars += 1;
            Ok(())
        };
        while scan.start.at < origin {
            step(&mut scan.start)?;
        }
        let before = scan.start.chars;
        let available = scan.chars - before;
        // The cursor `target` characters after the run's start, moved forward
        // from where an earlier run left it.
        let mut reach = |cursor: Option<Cursor>, target: u64| -> Result<Option<Cursor>, Error> {
            let Some(mut cursor) = cursor else {
                return Ok(None);
            };
            if cursor.chars > before + target {
                return Ok(None);
            }
            while cursor.chars < before + target {
                step(&mut cursor)?;
            }
            Ok(Some(cursor))
        };
        let floor = match min {
            Some(min) if min > 0 && min <= available => {
                scan.low = reach(scan.low, min)?;
                match scan.low {
                    Some(low) => Some(low.at),
                    None => {
                        self.scan = Some(scan);
                        return Ok(None);
                    }
                }
            }
            _ => None,
        };
        let known = if available > max {
            scan.high = reach(scan.high, max)?;
            let Some(high) = scan.high else {
                self.scan = Some(scan);
                return Ok(None);
            };
            Known {
                position: high.at,
                count: max,
                floor,
                exhausted: false,
            }
        } else {
            Known {
                position: scan.end,
                count: available,
                floor,
                exhausted: scan.exhausted,
            }
        };
        self.scan = Some(scan);
        Ok(Some(known))
    }

    /// Keep what a greedy run of `single` reached, for later runs of it.
    fn remember(&mut self, single: usize, reach: Reach, min: Option<u64>, max: u64) {
        let Reach {
            origin,
            reused,
            end,
            count,
            exhausted,
            floor,
        } = reach;
        if !reused {
            self.scan = Some(Scan {
                single,
                end: origin,
                chars: 0,
                exhausted: false,
                start: Cursor {
                    at: origin,
                    chars: 0,
                },
                low: None,
                high: None,
            });
        }
        let scan = self.scan.as_mut().expect("a scan is kept");
        let before = scan.start.chars;
        if end >= scan.end {
            scan.end = end;
            scan.chars = before + count;
            scan.exhausted = exhausted;
        }
        if let (Some(min), Some(floor)) = (min, floor)
            && min > 0
            && count >= min
        {
            scan.low = Some(Cursor {
                at: floor,
                chars: before + min,
            });
        }
        if count == max {
            scan.high = Some(Cursor {
                at: end,
                chars: before + max,
            });
        }
    }

    /// Continue a greedy run scanned from `origin` to the state's position at
    /// its longest viable stop, if it has one.
    fn run_stop(
        &mut self,
        state: &mut State,
        floor: Option<usize>,
        follow: Option<Follow>,
        origin: usize,
        capture: Option<usize>,
    ) -> Result<bool, Error> {
        let Some(floor) = floor else {
            return Ok(false);
        };
        let run = Run {
            floor,
            follow,
            origin,
            capture,
        };
        let Some(stop) = self.stop(state.start, state.position, run)? else {
            return Ok(false);
        };
        self.stop_at(state, stop, run)?;
        Ok(true)
    }

    /// Whether `node` can begin to match at `position`: false only when the
    /// first character it must consume, along a short path of operands that
    /// cannot match the empty string, is rejected there. An alternative that
    /// cannot begin fails at once when resumed, before it sets any capture, so
    /// it need not be left pending.
    fn may_begin(&mut self, mut node: usize, position: usize) -> Result<bool, Error> {
        let program = self.ctx.program;
        for _ in 0..BEGIN_DEPTH {
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            match program.nodes[node] {
                Node::Character(set) => {
                    return match self.ctx.input[position..].chars().next() {
                        Some(ch) => self.ctx.set_matches(set, ch),
                        None => Ok(false),
                    };
                }
                Node::Sequence(left, _) if !program.links.nodes[left].nullable => node = left,
                Node::Capture { body, .. } => node = body,
                Node::Repeat { body, min, .. }
                    if min != Count::Finite(0) && !program.links.nodes[body].nullable =>
                {
                    node = body;
                }
                _ => return Ok(true),
            }
        }
        Ok(true)
    }

    /// Whether the repetition `repeat`, or a counted repetition enclosing
    /// it, has a large count that can decide a choice on this input
    /// ([`Ctx::large`]).
    fn counted(&mut self, repeat: usize) -> Result<bool, Error> {
        let links = &self.ctx.program.links.nodes;
        let mut node = repeat;
        while node != super::pike::ROOT {
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            if self.ctx.large(node) {
                return Ok(true);
            }
            node = links[node].outer;
        }
        Ok(false)
    }

    /// Whether some repetition of the program has a large count that can
    /// decide a choice on this input, found once per execution.
    fn large_counts(&mut self) -> Result<bool, Error> {
        if let Some(large) = self.large {
            return Ok(large);
        }
        let program = self.ctx.program;
        let large = program.links.large && {
            self.ctx
                .budget
                .charge_wide(Resource::MatchSteps, copy_steps(program.nodes.len()))?;
            (0..program.nodes.len()).any(|node| {
                matches!(program.nodes[node], Node::Repeat { .. }) && self.ctx.large(node)
            })
        };
        self.large = Some(large);
        Ok(large)
    }

    /// Whether the [`super::pike::Link::single`] node `single` matches `ch`,
    /// testing its character sets in source order on an explicit work list.
    fn single_matches(&mut self, single: usize, ch: char) -> Result<bool, Error> {
        let nodes = &self.ctx.program.nodes;
        if let Node::Character(set) = nodes[single] {
            return self.ctx.set_matches(set, ch);
        }
        let mut work = WorkList::<usize, 16>::new();
        self.ctx.push(&mut work, single, 1)?;
        let mut matched = false;
        while let Some(node) = work.pop() {
            self.ctx.live_slots -= 1;
            self.ctx.budget.charge(Resource::MatchSteps, 1)?;
            match nodes[node] {
                Node::Character(set) => {
                    if self.ctx.set_matches(set, ch)? {
                        matched = true;
                        break;
                    }
                }
                Node::Choice(left, right) => {
                    self.ctx.push(&mut work, right, 1)?;
                    self.ctx.push(&mut work, left, 1)?;
                }
                _ => unreachable!("a single-character node is a set or a choice of them"),
            }
        }
        self.ctx.live_slots -= work.len() as u128;
        self.ctx.budget.release_work(work)?;
        Ok(matched)
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
    use purrdf_testkit::text::binary_text as random_ab;

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
            // One run of single characters, whose last iteration's capture
            // is read back; with no iteration it is unset and matches empty.
            (
                r"(a|b){1,3}\1",
                words(&['a', 'b'], 3)
                    .into_iter()
                    .filter_map(|word| Some(format!("{word}{}", word.chars().last()?)))
                    .collect(),
            ),
            (
                r"(a|b){0,2}\1",
                words(&['a', 'b'], 2)
                    .into_iter()
                    .map(|word| match word.chars().last() {
                        Some(last) => format!("{word}{last}"),
                        None => word,
                    })
                    .collect(),
            ),
            (
                r"((a|b)){2}\1\2",
                words(&['a', 'b'], 2)
                    .into_iter()
                    .filter(|word| word.len() == 2)
                    .map(|word| {
                        let last = &word[1..];
                        format!("{word}{last}{last}")
                    })
                    .collect(),
            ),
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
                    if let Some(captures) = &actual {
                        let span = captures.get(0).unwrap();
                        let text = &input[span];
                        assert!(language.iter().any(|word| word == text));
                    }
                    // The linear-time machines decide the same languages and
                    // report the same first match.
                    if !search.links.backreferences {
                        assert_eq!(
                            Vm::linear(&whole, input, Limits::new())
                                .is_match_from(0)
                                .unwrap(),
                            language.contains(input),
                            "linear whole {profile:?} {source:?} {input:?}"
                        );
                        assert_eq!(
                            Vm::linear(&search, input, Limits::new())
                                .find_from(0)
                                .unwrap(),
                            actual,
                            "linear search {profile:?} {source:?} {input:?}"
                        );
                    }
                    comparisons += 1;
                }
            }
        }
        assert_eq!(comparisons, 46_410);
    }

    /// Backreference-free shapes every linear-time machine is proven on.
    const DIFFERENTIAL: &[&str] = &[
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

    /// Counted repetitions: nested counts, alternation, reluctance, captures
    /// and empty iterations inside counts, and anchors around them.
    const COUNTED: &[&str] = &[
        "(ab){1,3}c",
        "(ab){2,3}",
        "(a|b){1,3}b",
        "(a|b){2}",
        "(a|b){3}?",
        "((a|b){2}){1,2}",
        "((a|b){2}){2,3}b",
        "((a|b){2}){1,2}?a",
        "(a|ab){1,3}",
        "(ab|a){2,4}?",
        "(a|aa){2,3}b",
        "(a{1,2}){2}",
        "(a{2}|b){1,3}",
        "((a)|b){2,3}",
        "((a)|(b)){1,2}?",
        "(a(b)?){2,3}",
        "((a?)b){1,3}",
        "(a?){3}",
        "(a?){3}b",
        "(a?){3}?a",
        "(a*){2,3}",
        "(a*?){2,3}b",
        "(a|){3}",
        "(|a){3}",
        "(|a){2,3}b",
        "(a??){3}",
        "(a??){2,3}b",
        "(a?){2,}",
        "(a?b?){3,}",
        "(a?b?){3,}?b",
        "((a?)(b?)){3}",
        "(^a|b){2}",
        "(a$|b){1,2}",
        "(^|a){3}",
        "(^|a){2,3}b",
        "(a|$){2,3}",
        "(a|^){3}$",
        "((a?){2}){2}",
        "((a?){2,3}b){1,2}",
        "((a?){2}|b){2}",
        "(a{0,2}?){2}",
        "((ab){1,2}|a){2}",
        "(a|b){0,2}?a",
        "(?:a|b){2,3}?b",
        "((?:a|b){2}){1,2}?",
        "(a{3}){2}",
        "(a{3}|b){1,2}",
        "(é|a){2,3}",
        "(é|a){1,4}é",
        "(.){2}",
        "(.{2}){1,2}",
        "(\\n|a){2}",
        ".{1,3}b",
        "[ab]{2,3}",
        "(a{1,3}){1,3}",
        "(a{1,2}?){2,3}b",
        "(a{2}){0,2}b",
        "((a{1,2}){2}){1,2}",
        "((a|b){1,2}b){2}",
        "(a{2,}|b){2}",
        "(b*a{2}){1,2}",
        "((a|é){2})+",
        "((a|b){2})*?b",
        "(a|b){2}(a|b){1,2}",
        "^(a|b){1,3}$",
        "^(a|b){2,3}?$",
        "(a{1,2}|b{1,2}){2}",
        "((^|a){2}){2,3}",
        "((a|$){2}){1,3}",
        "((a?){3}|b){2,4}",
        "((a?){2,3}){2}?b",
        "((^a?){2}){2}",
        "((|a){2}){2,3}b",
        "((a??){2}b?){2}",
        "((a{0,2}){2}){1,2}a",
        "(((a|b){2}){2}){1,2}",
        "((a|b){1,2}(a|b){2}){1,2}?",
    ];

    /// Every whole match and capture of the thread machine, from every start,
    /// against the backtracking machine on the same backreference-free program.
    #[test]
    fn thread_machine_reports_the_backtracking_machines_matches_and_captures() {
        let inputs = words(&['a', 'b', 'é', '\n'], 4);
        let mut comparisons = 0;
        for flags in ["", "m", "s", "i"] {
            for source in DIFFERENTIAL.iter().chain(COUNTED) {
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
        assert_eq!(comparisons, 4 * (DIFFERENTIAL.len() + COUNTED.len()) * 1593);
    }

    /// Every match start, whole match and capture of the set machine and the
    /// one-start thread machine, from every start and over successive
    /// searches of one execution, against the backtracking machine.
    #[test]
    fn linear_machines_report_the_backtracking_machines_matches_and_captures() {
        let inputs = words(&['a', 'b', 'é', '\n'], 4);
        let mut comparisons = 0;
        for flags in ["", "m", "s", "i"] {
            for source in DIFFERENTIAL.iter().chain(COUNTED) {
                let program = pattern(Profile::Xpath31, source, flags);
                for input in &inputs {
                    let mut successive = Vm::linear(&program, input, Limits::new());
                    let mut previous = 0;
                    for (start, _) in input.char_indices().chain([(input.len(), ' ')]) {
                        let backtrack = Vm::backtracking(&program, input, Limits::new())
                            .find_from(start)
                            .unwrap();
                        let linear = Vm::linear(&program, input, Limits::new())
                            .find_from(start)
                            .unwrap();
                        assert_eq!(linear, backtrack, "{source:?} {flags:?} {input:?} {start}");
                        let tight = Vm::tight(&program, input, Limits::new())
                            .find_from(start)
                            .unwrap();
                        assert_eq!(
                            tight, backtrack,
                            "tight {source:?} {flags:?} {input:?} {start}"
                        );
                        let matched = Vm::linear(&program, input, Limits::new())
                            .is_match_from(start)
                            .unwrap();
                        assert_eq!(
                            matched,
                            backtrack.is_some(),
                            "{source:?} {flags:?} {input:?} {start}"
                        );
                        // A later search of the same execution reads the
                        // starts an earlier one marked.
                        if start >= previous {
                            let found = successive.find_from(start).unwrap();
                            assert_eq!(found, backtrack, "{source:?} {flags:?} {input:?} {start}");
                            if let Some(span) = found.and_then(|captures| captures.get(0)) {
                                previous = span.end;
                            }
                        }
                        comparisons += 1;
                    }
                }
            }
        }
        assert_eq!(comparisons, 4 * (DIFFERENTIAL.len() + COUNTED.len()) * 1593);
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

    /// Longer seeded inputs than the exhaustive universe, over every shape and
    /// flag, against the backtracking machine where it answers within the
    /// production bounds and the thread machine from every start otherwise.
    #[test]
    fn linear_machines_agree_with_both_machines_on_longer_seeded_inputs() {
        let alphabet = ['a', 'b', 'a', 'b', 'é', '\n', 'c'];
        let mut rng = purrdf_testkit::rng::SplitMix64::new(0x0047_2600_BE4C_0006);
        // An exponential backtracking search gives way to the thread machine soon.
        let backtrack = Limits::new().with(Resource::MatchSteps, 1 << 21);
        let mut comparisons = 0;
        for flags in ["", "m", "s", "i"] {
            for source in DIFFERENTIAL.iter().chain(COUNTED) {
                let program = pattern(Profile::Xpath31, source, flags);
                for _ in 0..6 {
                    let length = 8 + rng.below_usize(56);
                    let input: String = (0..length)
                        .map(|_| alphabet[rng.below_usize(alphabet.len())])
                        .collect();
                    let expected = Vm::backtracking(&program, &input, backtrack)
                        .find_from(0)
                        .or_else(|_| Vm::threads(&program, &input, Limits::new()).find_from(0))
                        .unwrap();
                    // Kept states every few bytes and small covered ranges
                    // split, across successive searches.
                    let linear = Vm::tight(&program, &input, Limits::new());
                    assert_eq!(
                        Vm::linear(&program, &input, Limits::new())
                            .is_match_from(0)
                            .unwrap(),
                        expected.is_some(),
                        "{source:?} {flags:?} {input:?}"
                    );
                    let mut successive = linear;
                    let mut position = 0;
                    let mut found = successive.find_from(position).unwrap();
                    assert_eq!(found, expected, "{source:?} {flags:?} {input:?}");
                    // Every later match of one execution, as a replacement visits them.
                    while let Some(span) = found.as_ref().and_then(|captures| captures.get(0)) {
                        position = if span.is_empty() {
                            match input[span.end..].chars().next() {
                                Some(ch) => span.end + ch.len_utf8(),
                                None => break,
                            }
                        } else {
                            span.end
                        };
                        found = successive.find_from(position).unwrap();
                        let oracle = Vm::backtracking(&program, &input, backtrack)
                            .find_from(position)
                            .or_else(|_| {
                                Vm::threads(&program, &input, Limits::new()).find_from(position)
                            })
                            .unwrap();
                        assert_eq!(found, oracle, "{source:?} {flags:?} {input:?} {position}");
                    }
                    comparisons += 1;
                }
            }
        }
        assert_eq!(comparisons, 4 * 6 * (DIFFERENTIAL.len() + COUNTED.len()));
    }

    /// Repetitions whose minimum far exceeds the input, written with `N` and
    /// `W` for the minimum and the maximum. Each body can match the empty
    /// string everywhere, so the minimum is met by empty iterations wherever
    /// the consuming ones end, and the first match is the same for every
    /// minimum beyond the input's length: the backtracking machine answers
    /// the small counts, and only the linear-time machines answer the huge
    /// ones.
    const HUGE: &[&str] = &[
        "(|a){N}",
        "(|a){N}b",
        "(a??){N}",
        "(a??){N}b",
        "(a*?){N}b",
        "(|a|b){N}c",
        "((|a)b?){N}",
        "(|a){N,}b",
        "(|a){N,W}b",
        "(|a){N,W}",
        "(a??){2,N}b",
        "x(|a){N}y",
        "(|a){N}(|b){N}",
        "((|a){N}){2}",
        "((|a){2}){N}b",
        "(a?|b){N}",
        "(a?|b){N}c",
        "(b|a??){N}c",
        "(a?){N}",
        "(a?){N}b",
        "(a*|b){N,W}",
        "^(|a){N}$",
        "(|a){N}$",
        "(|(a)|(b)){N}",
        "((a)|){N}b",
    ];

    #[test]
    fn minimums_beyond_the_input_answer_like_lesser_minimums() {
        let inputs = words(&['a', 'b', 'c', 'x', 'y'], 4);
        // Every small minimum beyond these inputs gives the same answer, for
        // the same window above it.
        let small = |shape: &str, minimum: u64, window: u64| {
            shape
                .replace('N', &minimum.to_string())
                .replace('W', &(minimum + window).to_string())
        };
        let mut comparisons = 0;
        for flags in ["", "m", "i"] {
            for shape in HUGE {
                for (huge, top, window) in [
                    ("18446744073709551616", "18446744073709551617", 1),
                    (
                        "99999999999999999999999999999",
                        "100000000000000000000000000002",
                        3,
                    ),
                    ("18446744073709551615", "18446744073709551615", 0),
                    ("1000000000", "1000000001", 1),
                ] {
                    let oracles = [7, 9].map(|minimum| {
                        pattern(Profile::Xpath31, &small(shape, minimum, window), flags)
                    });
                    let source = shape.replace('N', huge).replace('W', top);
                    let program = pattern(Profile::Xpath31, &source, flags);
                    for input in &inputs {
                        for (start, _) in input.char_indices().chain([(input.len(), ' ')]) {
                            let expected = Vm::backtracking(&oracles[0], input, Limits::new())
                                .find_from(start)
                                .unwrap();
                            let other = Vm::backtracking(&oracles[1], input, Limits::new())
                                .find_from(start)
                                .unwrap();
                            assert_eq!(
                                expected, other,
                                "{shape} invariance {flags:?} {input:?} {start}"
                            );
                            assert_eq!(
                                Vm::linear(&program, input, Limits::new())
                                    .find_from(start)
                                    .unwrap(),
                                expected,
                                "{source} {flags:?} {input:?} {start}"
                            );
                            assert_eq!(
                                Vm::tight(&program, input, Limits::new())
                                    .find_from(start)
                                    .unwrap(),
                                expected,
                                "tight {source} {flags:?} {input:?} {start}"
                            );
                            assert_eq!(
                                Vm::linear(&program, input, Limits::new())
                                    .is_match_from(start)
                                    .unwrap(),
                                expected.is_some(),
                                "{source} {flags:?} {input:?} {start}"
                            );
                            comparisons += 1;
                        }
                        // The public entries, whatever machine answers.
                        assert_eq!(
                            program.find(input, Limits::new()).unwrap(),
                            Vm::backtracking(&oracles[0], input, Limits::new())
                                .find_from(0)
                                .unwrap(),
                            "{source} {flags:?} {input:?}"
                        );
                    }
                }
            }
        }
        assert_eq!(comparisons, 3 * HUGE.len() * 4 * 3711);
    }

    /// A count set that changes at every position until it saturates makes a
    /// new state at every position, so the bounded state cache is cleared and
    /// refilled many times; the answers stay exact.
    #[test]
    fn cleared_state_caches_keep_exact_answers() {
        let limits = Limits::new();
        let input = random_ab(400_000, 11);
        let program = pattern(Profile::Xpath31, "(a|b){100000}c", "");
        for (text, start) in [
            (format!("{input}c"), Some(300_000)),
            (format!("{}c", &input[..99_999]), None),
            (format!("{}c", &input[..100_000]), Some(0)),
        ] {
            assert_eq!(
                program.is_match(&text, limits).unwrap(),
                start.is_some(),
                "{}",
                text.len()
            );
            let found = program.find(&text, limits).unwrap();
            assert_eq!(
                found.as_ref().and_then(|captures| captures.get(0)),
                start.map(|start| start..text.len()),
                "{}",
                text.len()
            );
            if let Some(captures) = found {
                assert_eq!(captures.get(1), Some(text.len() - 2..text.len() - 1));
            }
        }
    }

    /// Counted repetitions the thread machine refused at the production
    /// defaults, at the sizes it refused them and twice beyond: one thread per
    /// distinct count from every live start exhausted the work bound, while
    /// the set machine keeps each repetition's counts as one set. Each is
    /// compared with the compatibility engine, without a match and with the
    /// suffix that completes one, all capture spans included.
    #[test]
    fn counted_repetitions_answer_like_the_compatibility_engine_at_the_production_defaults() {
        let limits = Limits::new();
        for scale in [1, 2] {
            let pairs = |bytes: usize| "ab".repeat(bytes * scale / 2);
            let cases: [(&str, String, &str); 11] = [
                ("(ab){1,1000}c", pairs(128 << 10), "c"),
                ("(ab){2,50}c", pairs(1 << 20), "c"),
                ("(ab){1,100}c", pairs(1 << 20), "c"),
                ("(ab|cd){1,20}e", "abcd".repeat((1 << 18) * scale), "e"),
                ("((a|b){3}){5,9}c", random_ab((400 << 10) * scale, 3), "c"),
                ("((a|b){2}){2,5}c", random_ab((800 << 10) * scale, 4), "c"),
                ("(a|b){1,30}c", random_ab((4 << 20) * scale, 5), "c"),
                ("(a|b){3,9}c", random_ab((4 << 20) * scale, 6), "c"),
                (r"(\w+\s){3,5}zzz", prose((4 << 20) * scale), " zzz"),
                ("node.*graph.*zzz", prose((8 << 20) * scale), " zzz"),
                // An ambiguous body: one start keeps many counts until the end.
                ("(a|aa){1,1000}b", "a".repeat((1 << 20) * scale), "b"),
            ];
            for (source, input, suffix) in &cases {
                let program = pattern(Profile::Xpath31, source, "");
                let completed = format!("{input}{suffix}");
                for (text, matched) in [(input, false), (&completed, true)] {
                    assert_eq!(
                        program.is_match(text, limits).unwrap(),
                        matched,
                        "{source} over {}",
                        text.len()
                    );
                    let (found, threaded) = native(source, text);
                    assert_eq!(found.is_some(), matched, "{source} over {}", text.len());
                    assert_eq!(
                        found,
                        compatibility(source, text),
                        "{source} over {}: compatibility",
                        text.len()
                    );
                    if !matched {
                        assert!(threaded, "{source}: the linear-time machines took over");
                    }
                }
            }
        }
    }

    /// The production matcher's exact work for one search.
    fn match_steps(program: &CompiledPattern, input: &str, find: bool) -> u64 {
        let mut vm = Vm::new(program, input, Limits::new());
        if find {
            vm.find_from(0).unwrap();
        } else {
            vm.is_match_from(0).unwrap();
        }
        vm.budget().used(Resource::MatchSteps)
    }

    /// Counted repetitions of one character class or one group, with counts
    /// beyond the input or equal to it, at the sizes the step bound refused
    /// them: no state repeated, and every position cost a full closure. A
    /// maximum no count on the input can reach no longer decides anything, a
    /// run of one character per iteration is scanned once, and a search that
    /// follows one path pays for that path alone, so each answers with its
    /// exact spans, at a cost per byte that does not depend on the count.
    #[test]
    fn large_counts_of_one_class_or_one_group_answer_at_the_production_defaults() {
        let limits = Limits::new();
        let mut text = prose(4 << 20);
        text.truncate(4 << 20);
        let ab = random_ab(4 << 20, 7);
        let (n, last_word) = (
            text.len(),
            text.trim_end_matches(' ').rfind(' ').unwrap() + 1,
        );
        let whole = |input: &str| Some(0..input.len());
        // Each source, its input, the match and group 1, and a bound on the
        // steps per byte of a search for the match and of a decision.
        let cases = [
            (r"^[\s\S]{0,10000000}$", &text, whole(&text), None, 4),
            (r"^[^\n]{0,10000000}$", &text, whole(&text), None, 4),
            (r"^[^<>]{1,5000000}$", &text, whole(&text), None, 4),
            (r"^[\p{L} ]{1,5000000}$", &text, whole(&text), None, 4),
            (
                r"^(\w+\s?){1,5000000}$",
                &text,
                whole(&text),
                Some(last_word..n),
                4,
            ),
            (
                "^(a|b){4194304}$",
                &ab,
                whole(&ab),
                Some(ab.len() - 1..ab.len()),
                6,
            ),
            (
                "(a|b){4000000}",
                &ab,
                Some(0..4_000_000),
                Some(3_999_999..4_000_000),
                6,
            ),
            ("(?:a|b){4000000}", &ab, Some(0..4_000_000), None, 6),
            (
                "^((a|b)){1,10000000}$",
                &ab,
                whole(&ab),
                Some(ab.len() - 1..ab.len()),
                6,
            ),
        ];
        for (source, input, matched, group, bound) in cases {
            for profile in [Profile::Xpath20, Profile::Xpath31] {
                if profile == Profile::Xpath20 && source.contains("(?:") {
                    continue;
                }
                let program = pattern(profile, source, "");
                let found = program.find(input, limits).unwrap();
                assert_eq!(
                    found.as_ref().and_then(|captures| captures.get(0)),
                    matched,
                    "{profile:?} {source}"
                );
                if program.capture_count() > 0 {
                    assert_eq!(
                        found.as_ref().and_then(|captures| captures.get(1)),
                        group,
                        "{profile:?} {source}"
                    );
                }
                assert!(
                    program.is_match(input, limits).unwrap(),
                    "{profile:?} {source}"
                );
                for find in [false, true] {
                    let steps = match_steps(&program, input, find);
                    assert!(
                        steps <= bound * u64::try_from(input.len()).unwrap(),
                        "{profile:?} {source}: {steps} steps for {} bytes, bound {bound} per byte",
                        input.len()
                    );
                }
            }
        }
        // The cost does not depend on the count: a maximum ten times larger
        // spends exactly the same steps.
        for (source, larger) in [
            (r"^[\s\S]{0,10000000}$", r"^[\s\S]{0,100000000}$"),
            (r"^[^\n]{0,10000000}$", r"^[^\n]{0,100000000}$"),
            (r"^[^<>]{1,5000000}$", r"^[^<>]{1,50000000}$"),
            (r"^[\p{L} ]{1,5000000}$", r"^[\p{L} ]{1,50000000}$"),
            (r"^(\w+\s?){1,5000000}$", r"^(\w+\s?){1,50000000}$"),
        ] {
            for profile in Profile::ALL {
                for find in [false, true] {
                    assert_eq!(
                        match_steps(&pattern(profile, source, ""), &text, find),
                        match_steps(&pattern(profile, larger, ""), &text, find),
                        "{profile:?} {source}"
                    );
                }
            }
        }
        // Valid neighbours that do not match answer false: past the class, a
        // byte beyond an exact count, a count beyond the input, and past a
        // maximum the input exceeds.
        let wider = prose(12 << 20);
        let refusals: [(&str, String); 7] = [
            (r"^[^<>]{1,5000000}$", format!("{text}<")),
            (r"^(\w+\s?){1,5000000}$", format!("{text}!")),
            ("^(a|b){4194304}$", format!("{ab}a")),
            ("(a|b){4000000}", ab[..3_999_999].to_owned()),
            (
                "(a|b){4000000}",
                format!("{}c{}", &ab[..3_999_999], &ab[..194_305]),
            ),
            (r"^[\s\S]{0,10000000}$", wider.clone()),
            ("^.{1,5000000}$", wider[..8 << 20].to_owned()),
        ];
        for (source, input) in &refusals {
            let program = pattern(Profile::Xpath31, source, "");
            assert!(!program.is_match(input, limits).unwrap(), "{source}");
            assert!(program.find(input, limits).unwrap().is_none(), "{source}");
        }
    }

    /// Replacement of a large counted group's only match, at the size the
    /// step bound refused it, with the last iteration's capture.
    #[test]
    fn large_counted_group_replacement_answers_at_the_production_defaults() {
        let limits = Limits::new();
        let mut text = prose(2 << 20);
        text.truncate(2 << 20);
        let last_word = &text[text.trim_end_matches(' ').rfind(' ').unwrap() + 1..];
        let program = pattern(Profile::Xpath31, r"^(\w+\s?){1,5000000}$", "");
        assert_eq!(
            program.replace_all(&text, "[$1]", limits).unwrap(),
            format!("[{last_word}]")
        );
        let ab = random_ab(4 << 20, 8);
        let exact = pattern(Profile::Xpath31, "(a|b){4000000}", "");
        let replaced = exact.replace_all(&ab, "<$1>", limits).unwrap();
        assert_eq!(
            replaced,
            format!("<{}>{}", &ab[3_999_999..4_000_000], &ab[4_000_000..])
        );
    }

    /// Counted repetitions of one character per iteration, of groups, and of
    /// classes, with counts below, at and beyond seeded inputs, under every
    /// flag and both laws: the production matcher, the backtracking machine
    /// and the thread machine from every start report the same matches and
    /// captures, search after search, and the compatibility engine the same
    /// whole matches.
    #[test]
    fn large_and_exact_counts_agree_across_machines_flags_and_laws() {
        const SHAPES: &[&str] = &[
            r"^[\s\S]{0,M}$",
            r"[^\n]{1,M}",
            r"^[^<>]{1,M}$",
            r"[\p{L} ]{2,M}x",
            r"^(\w+\s?){1,M}$",
            r"(\w+\s?){1,M}",
            "^(a|b){N}$",
            "(a|b){N}",
            "(a|b){N}c",
            "((a|b)){1,M}",
            "(?:a|b){N}",
            "^(ab){N}$",
            "(ab|cd){N}",
            "(a|bc){1,M}",
            "((a|b){2}){N}",
            "^([a-z]+ ){1,M}[a-z]+$",
            r"(\w+ ){N}",
            "[ab]{N,M}c",
            "(a|b){N,M}?b",
            "((a)|b){1,M}",
            "(a|é){N}",
            "([aA]|b){N}$",
            "^(a|b){0,N}$",
            "(.){N}",
            "^.{1,N}$",
            "(a|b){0,N}b",
            "x(a|b){N}",
            "(a|b|c){N,M}c",
            "(?:(a)|b){N}",
            "((a|b)|c){1,M}$",
            "(a|\n){N}",
            "(a|b){N}$|c",
            "(ab){1,N}$",
            "^(a|b){N}(a|b){N}$",
            "((a|b){N}c)+",
        ];
        let alphabet = ['a', 'b', 'a', 'b', 'c', ' ', '\n', 'é', 'x', 'A', '<', 'd'];
        let mut rng = purrdf_testkit::rng::SplitMix64::new(0x0047_2600_BE4C_0009);
        let backtrack = Limits::new().with(Resource::MatchSteps, 1 << 22);
        let mut comparisons = 0;
        let mut executions = 0;
        for flags in ["", "i", "m", "s", "x"] {
            for template in SHAPES {
                for _ in 0..12 {
                    // The original general machine completes every generated
                    // ambiguous case; a resource refusal is not a comparison.
                    let length = rng.below_usize(12);
                    let input: String = (0..length)
                        .map(|_| alphabet[rng.below_usize(alphabet.len())])
                        .collect();
                    let chars = input.chars().count() as u64;
                    let count = [1, chars / 2, chars, chars + 1][rng.below_usize(4)].max(1);
                    let most = [chars + 3, 5000][rng.below_usize(2)].max(count);
                    let source = template
                        .replace('N', &count.to_string())
                        .replace('M', &most.to_string());
                    for profile in [Profile::Xpath20, Profile::Xpath31] {
                        let program = match compile(profile, &source, flags, Limits::new()) {
                            Ok(program) => program,
                            Err(error) => {
                                assert!(
                                    source.contains("(?:")
                                        && profile == Profile::Xpath20
                                        && matches!(error, Error::Syntax { .. }),
                                    "{profile:?} {source:?} {flags:?}: {error}"
                                );
                                continue;
                            }
                        };
                        let reference = |position: usize| {
                            Vm::threads(&program, &input, Limits::new()).find_from(position)
                        };
                        let mut production = Vm::new(&program, &input, Limits::new());
                        executions += 1;
                        let mut position = 0;
                        loop {
                            let expected = reference(position).unwrap();
                            let found = production.find_from(position).unwrap();
                            assert_eq!(
                                found, expected,
                                "{profile:?} {source:?} {flags:?} {input:?} {position}"
                            );
                            let alone = Vm::general_backtracking(&program, &input, backtrack)
                                .find_from(position)
                                .unwrap_or_else(|error| panic!(
                                    "general {profile:?} {source:?} {flags:?} {input:?}: {error}"
                                ));
                            assert_eq!(
                                alone, expected,
                                "general {profile:?} {source:?} {flags:?} {input:?}"
                            );
                            if position == 0 {
                                assert_eq!(
                                    Vm::new(&program, &input, Limits::new())
                                        .is_match_from(0)
                                        .unwrap(),
                                    expected.is_some(),
                                    "{profile:?} {source:?} {flags:?} {input:?}"
                                );
                                assert_eq!(
                                    Vm::linear(&program, &input, Limits::new())
                                        .is_match_from(0)
                                        .unwrap(),
                                    expected.is_some(),
                                    "{profile:?} {source:?} {flags:?} {input:?}"
                                );
                                if flags.is_empty()
                                    && profile == Profile::Xpath31
                                    && let Ok(compiled) = crate::xsd_regex::compile(&source, "")
                                {
                                    assert_eq!(
                                        compiled.as_regex().find(&input).map(|span| span.range()),
                                        expected.as_ref().and_then(|captures| captures.get(0)),
                                        "compatibility {source:?} {input:?}"
                                    );
                                }
                            }
                            comparisons += 1;
                            let Some(span) = expected.and_then(|captures| captures.get(0)) else {
                                break;
                            };
                            position = if span.is_empty() {
                                match input[span.end..].chars().next() {
                                    Some(ch) => span.end + ch.len_utf8(),
                                    None => break,
                                }
                            } else {
                                span.end
                            };
                        }
                    }
                }
            }
        }
        assert_eq!(executions, 5 * 12 * (SHAPES.len() * 2 - 2));
        assert!(comparisons >= executions);
    }

    /// Counts above the size the set machine keeps relative to an offset
    /// ([`LARGE_COUNT`]), over inputs long enough to cross the bounds: the
    /// production matcher, the set and thread machines guided by the reverse
    /// scan, and the thread machine from every start agree, search after
    /// search, under every flag and both laws.
    #[test]
    fn counts_kept_relative_to_offsets_agree_with_the_thread_machine() {
        const SHAPES: &[&str] = &[
            r"^(\w+\s?){N,M}$",
            r"^(\w+\s?){1,N}$",
            r"^(\w+ ){N}",
            "^(ab|b){N,M}$",
            "(ab|b){N}c",
            "^(a|bc){1,N}$",
            "^((a|b){2}){N,M}",
            "(a|b){N}",
            "^[ab]{N}$",
            "^(?:ab|a){N,M}$",
            "x(ab|a){1,N}y",
            "^(ab){N}|b",
        ];
        let mut rng = purrdf_testkit::rng::SplitMix64::new(0x0047_2600_BE4C_000A);
        let mut comparisons = 0;
        for flags in ["", "i", "m", "s"] {
            for template in SHAPES {
                for _ in 0..6 {
                    let length = 1030 + rng.below_usize(400);
                    let input: String = (0..length)
                        .map(|index| {
                            let roll = rng.below_usize(16);
                            match roll {
                                0 => ' ',
                                1 if index % 97 == 5 => 'x',
                                2 if index % 89 == 7 => 'y',
                                3 if index % 83 == 3 => 'c',
                                4 if index % 79 == 1 => '\n',
                                _ if roll.is_multiple_of(2) => 'a',
                                _ => 'b',
                            }
                        })
                        .collect();
                    let chars = input.chars().count() as u64;
                    let count = [1025, chars / 2, chars - 3, chars, chars + 2][rng.below_usize(5)]
                        .max(1025);
                    let most = [count + 3, count + 400, 1 << 20][rng.below_usize(3)];
                    let source = template
                        .replace('N', &count.to_string())
                        .replace('M', &most.to_string());
                    for profile in [Profile::Xpath20, Profile::Xpath31] {
                        let program = match compile(profile, &source, flags, Limits::new()) {
                            Ok(program) => program,
                            Err(error) => {
                                assert!(
                                    source.contains("(?:")
                                        && profile == Profile::Xpath20
                                        && matches!(error, Error::Syntax { .. }),
                                    "{profile:?} {source:?} {flags:?}: {error}"
                                );
                                continue;
                            }
                        };
                        let mut production = Vm::new(&program, &input, Limits::new());
                        let mut linear = Vm::tight(&program, &input, Limits::new());
                        let mut position = 0;
                        loop {
                            let expected = Vm::threads(&program, &input, Limits::new())
                                .find_from(position)
                                .unwrap();
                            assert_eq!(
                                production.find_from(position).unwrap(),
                                expected,
                                "{profile:?} {source:?} {flags:?} {position}"
                            );
                            assert_eq!(
                                linear.find_from(position).unwrap(),
                                expected,
                                "linear {profile:?} {source:?} {flags:?} {position}"
                            );
                            if position == 0 {
                                assert_eq!(
                                    Vm::linear(&program, &input, Limits::new())
                                        .is_match_from(0)
                                        .unwrap(),
                                    expected.is_some(),
                                    "{profile:?} {source:?} {flags:?}"
                                );
                            }
                            comparisons += 1;
                            let Some(span) = expected.and_then(|captures| captures.get(0)) else {
                                break;
                            };
                            position = if span.is_empty() {
                                match input[span.end..].chars().next() {
                                    Some(ch) => span.end + ch.len_utf8(),
                                    None => break,
                                }
                            } else {
                                span.end
                            };
                        }
                    }
                }
            }
        }
        assert_eq!(comparisons, 30_884);
    }

    #[test]
    fn adversary_shapes_scale_linearly_well_beyond_the_adversarys_sizes() {
        // Sixty-four mebibytes at the production defaults: an abandoned
        // backtracking attempt spends at most two steps per byte, the reverse
        // scan one, and a match's walk about one more over its covered states,
        // so every shape answers within the step bound.
        let text = prose(64 << 20);
        let ab = "ab".repeat(32 << 20);
        let a = "a".repeat(64 << 20);
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
            assert_eq!(
                found.map(|spans| spans[0].clone()),
                matched.then_some(Some(0..input.len())),
                "{source} over {}",
                input.len()
            );
            assert!(threaded, "{source}");
            assert_eq!(
                pattern(Profile::Xpath31, source, "")
                    .is_match(input, Limits::new())
                    .unwrap(),
                matched,
                "{source}"
            );
        }
    }

    #[test]
    fn formerly_exponential_shapes_answer_and_backreference_blowups_still_refuse() {
        let limits = Limits::new();
        let forty = "a".repeat(40);
        for source in ["(a|a)*b", "(a|aa)*b", "(a*)*b", "((a+)+)+b"] {
            // Refused by the backtracking machine alone, answered by the
            // thread machine as the compatibility engine answers; a choice
            // between single characters is one run, whose stops the
            // backtracking machine visits once each.
            let program = pattern(Profile::Xpath31, source, "");
            let alone = Vm::backtracking(&program, &forty, limits).find_from(0);
            if source == "(a|a)*b" {
                assert!(alone.unwrap().is_none(), "{source}");
            } else {
                assert!(matches!(alone, Err(Error::Resource(_))), "{source}");
            }
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
        // A run of single characters is not exponential, backreference or not:
        // its last iteration's capture is read back at each stop.
        let run = pattern(Profile::Xpath31, r"^(a|a)*\1b", "");
        assert!(!run.is_match(&forty, limits).unwrap());
        let captures = run.find(&format!("{forty}b"), limits).unwrap().unwrap();
        assert_eq!(
            (captures.get(0), captures.get(1)),
            (Some(0..41), Some(38..39))
        );
        for (source, refused, neighbour) in [
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
        // A nullable body repeated more than u64 times: one empty iteration
        // below the minimum stands for every further one, so the minimum is
        // met in finite work, with the empty last iteration's captures.
        let nullable = pattern(Profile::Xpath31, "(a?){18446744073709551616}", "");
        let thousand = pattern(Profile::Xpath31, "(a?){1000}", "");
        for (input, whole, last) in [("", 0..0, 0..0), ("a", 0..1, 1..1), ("aab", 0..2, 2..2)] {
            for program in [&nullable, &thousand] {
                assert!(program.is_match(input, limits).unwrap(), "{input:?}");
                let captures = program.find(input, limits).unwrap().unwrap();
                assert_eq!(
                    (captures.get(0), captures.get(1)),
                    (Some(whole.clone()), Some(last.clone())),
                    "{} {input:?}",
                    program.source()
                );
            }
        }
        // Not nullable, the same count can never be met.
        let required = pattern(Profile::Xpath31, "(a){18446744073709551616}", "");
        assert!(!required.is_match(&"a".repeat(1000), limits).unwrap());
        assert!(required.find(&"a".repeat(1000), limits).unwrap().is_none());
        // A large counted group repeated from every start keeps one set of
        // counts, not one thread per distinct count: the search over every
        // start answers, without and with the character that completes it.
        let input = "ab".repeat(1 << 16);
        let unanchored = pattern(Profile::Xpath31, "(ab){1,100000}c", "");
        assert!(!unanchored.is_match(&input, limits).unwrap());
        assert!(unanchored.find(&input, limits).unwrap().is_none());
        let completed = format!("{input}c");
        assert!(unanchored.is_match(&completed, limits).unwrap());
        let captures = unanchored.find(&completed, limits).unwrap().unwrap();
        assert_eq!(captures.get(0), Some(0..completed.len()));
        assert_eq!(captures.get(1), Some(input.len() - 2..input.len()));
        // Below the pairs the input holds, the leftmost match starts late.
        let bounded = pattern(Profile::Xpath31, "(ab){1,1000}c", "");
        let captures = bounded.find(&completed, limits).unwrap().unwrap();
        assert_eq!(captures.get(0), Some(input.len() - 2000..completed.len()));
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
    fn count_ladders_keep_integer_work_linear_at_production_limits() {
        for bytes in [1 << 20, 2 << 20, 4 << 20] {
            let input = random_ab(bytes, 0x467 + bytes as u64);
            for source in [format!("^(a|b){{{bytes}}}$"), format!("(a|b){{{bytes}}}")] {
                for profile in Profile::ALL {
                    let program = pattern(profile, &source, "");
                    for find in [false, true] {
                        let steps = match_steps(&program, &input, find);
                        assert!(steps <= 6 * bytes as u64, "{source}: {steps}/{bytes}");
                    }
                    let captured = program.find(&input, Limits::new()).unwrap().unwrap();
                    assert_eq!(captured.get(0), Some(0..bytes));
                    assert_eq!(captured.get(1), Some(bytes - 1..bytes));
                }
            }
        }
    }

    #[test]
    fn nested_nullable_refusal_has_an_actual_admitted_count_neighbor() {
        for profile in Profile::ALL {
            let admitted = pattern(profile, "((a?){3000}){3000}b", "");
            assert!(admitted.is_match("aaab", Limits::new()).unwrap());
            assert_eq!(
                admitted
                    .find("aaab", Limits::new())
                    .unwrap()
                    .unwrap()
                    .get(0),
                Some(0..4)
            );
            let refused = pattern(profile, "((a?){5000}){5000}b", "");
            for find in [false, true] {
                let mut vm = Vm::new(&refused, "aaab", Limits::new());
                let failure = if find {
                    vm.find_from(0).map(|found| found.is_some())
                } else {
                    vm.is_match_from(0)
                }
                .unwrap_err();
                assert!(
                    matches!(
                        failure,
                        Error::Resource(Refusal {
                            resource: Resource::MatchSlots | Resource::MatchSteps,
                            ..
                        })
                    ),
                    "{profile:?}: {failure:?}"
                );
            }
        }
    }

    #[test]
    fn routing_and_offset_boundaries_agree_with_original_general_repetition() {
        for count in [1020, 1023, 1024, 1025, 1028, 4096] {
            for pair in [false, true] {
                let atom = if pair { "ab" } else { "a|b" };
                let source = format!("^({atom}){{{count}}}$");
                for length in [count - 1, count, count + 1] {
                    let input = if pair {
                        "ab".repeat(length)
                    } else {
                        random_ab(length, 0x467 + length as u64)
                    };
                    for profile in Profile::ALL {
                        for flags in ["", "i", "m", "s"] {
                            let program = pattern(profile, &source, flags);
                            let expected =
                                Vm::general_backtracking(&program, &input, Limits::new())
                                    .find_from(0)
                                    .unwrap();
                            assert_eq!(expected.is_some(), length == count);
                            assert_eq!(
                                program.find(&input, Limits::new()).unwrap(),
                                expected,
                                "production {profile:?} {source} {flags:?} {length}"
                            );
                            assert_eq!(
                                Vm::tight(&program, &input, Limits::new())
                                    .find_from(0)
                                    .unwrap(),
                                expected,
                                "checkpoints {profile:?} {source} {flags:?} {length}"
                            );
                        }
                    }
                }
            }
        }
        // The input reaches and crosses an actual maximum in forward and
        // reverse states. The independent original path keeps absolute counts.
        for count in [1023, 1024, 1025, 1028] {
            let source = format!("(ab){{{},{}}}c", count - 4, count + 4);
            for pairs in [count - 5, count - 4, count, count + 4, count + 5] {
                let input = format!("{}c", "ab".repeat(pairs));
                let program = pattern(Profile::Xpath31, &source, "");
                let expected = Vm::general_backtracking(&program, &input, Limits::new())
                    .find_from(0)
                    .unwrap();
                assert_eq!(program.find(&input, Limits::new()).unwrap(), expected);
                assert_eq!(
                    Vm::tight(&program, &input, Limits::new())
                        .find_from(0)
                        .unwrap(),
                    expected,
                    "{source} {pairs}"
                );
            }
        }
    }

    #[test]
    fn nullable_finite_bounds_keep_original_empty_captures_and_replacement() {
        for law in [
            Law::Dated(Profile::Xpath20),
            Law::Dated(Profile::Xpath31),
            Law::Compatibility,
        ] {
            for count in [0, 1, 2, 3, 5, 8] {
                for body in ["(a?)", "(a*)", "((a)?)"] {
                    for suffix in ["", "?"] {
                        let source = format!("{body}{{0,{count}}}{suffix}");
                        let owned = compile::compile_owned_law(
                            law,
                            &source,
                            "",
                            Limits::new(),
                            purrdf_lex::allocation::Resident,
                            |pattern| pattern,
                        )
                        .unwrap();
                        let program = owned.value();
                        for input in ["", "a", "aa", "aaa", "b", "ab", "aab", "éa"] {
                            for start in input
                                .char_indices()
                                .map(|(start, _)| start)
                                .chain(std::iter::once(input.len()))
                            {
                                let expected =
                                    Vm::general_backtracking(program, input, Limits::new())
                                        .find_from(start)
                                        .unwrap();
                                for (machine, mut vm) in [
                                    ("production", Vm::new(program, input, Limits::new())),
                                    ("threads", Vm::threads(program, input, Limits::new())),
                                    ("checkpoints", Vm::tight(program, input, Limits::new())),
                                ] {
                                    assert_eq!(
                                        vm.find_from(start).unwrap(),
                                        expected,
                                        "{law:?} {source:?} {input:?} {start} {machine}"
                                    );
                                }
                            }
                        }
                        // The required tail makes every replaced match
                        // nonempty while its captured last iteration can be
                        // empty. The original machine supplies the history.
                        let source = format!("{source}b");
                        let owned = compile::compile_owned_law(
                            law,
                            &source,
                            "",
                            Limits::new(),
                            purrdf_lex::allocation::Resident,
                            |pattern| pattern,
                        )
                        .unwrap();
                        let program = owned.value();
                        for input in ["", "b", "ab", "aab", "aaab", "baab", "éab"] {
                            assert_eq!(
                                program.replace_all(input, "[$1]", Limits::new()).unwrap(),
                                general_capture_replacement(program, input),
                                "{law:?} {source:?} {input:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    fn general_capture_replacement(program: &CompiledPattern, input: &str) -> String {
        let mut vm = Vm::general_backtracking(program, input, Limits::new());
        let mut expected = String::new();
        let mut from = 0;
        while let Some(captures) = vm.find_from(from).unwrap() {
            let whole = captures.get(0).unwrap();
            assert!(!whole.is_empty());
            expected.push_str(&input[from..whole.start]);
            expected.push('[');
            if let Some(last) = captures.get(1) {
                expected.push_str(&input[last]);
            }
            expected.push(']');
            from = whole.end;
        }
        expected.push_str(&input[from..]);
        expected
    }

    #[test]
    fn counted_replacement_matches_original_general_capture_history() {
        let mut rng = purrdf_testkit::rng::SplitMix64::new(0x467_CAFE);
        for profile in Profile::ALL {
            for _ in 0..96 {
                let count = 1 + rng.below_usize(12);
                let input: String = (0..48)
                    .map(|_| ['a', 'b', 'c', 'é'][rng.below_usize(4)])
                    .collect();
                let source = format!("(a|b|é){{1,{count}}}");
                let program = pattern(profile, &source, "");
                assert_eq!(
                    program.replace_all(&input, "[$1]", Limits::new()).unwrap(),
                    general_capture_replacement(&program, &input),
                    "{profile:?} {source} {input:?}"
                );
            }
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
        // The work an empty iteration's count jump needs is independent of
        // the count: a required minimum beyond u64 needs exactly the steps a
        // minimum of a thousand needs, and one step less is still refused.
        for input in ["", "a", "aab"] {
            let mut required = Vec::new();
            for source in [
                "(a?){18446744073709551616}",
                "(a?){18446744073709551615}",
                "(a?){1000}",
            ] {
                let program = pattern(Profile::Xpath31, source, "");
                let expected = Vm::backtracking(
                    &pattern(Profile::Xpath31, "(a?){4}", ""),
                    input,
                    Limits::new(),
                )
                .find_from(0)
                .unwrap();
                let run = |limits| Vm::linear(&program, input, limits).find_from(0);
                let steps = requirement(&run, Resource::MatchSteps);
                assert_eq!(
                    run(Limits::new().with(Resource::MatchSteps, steps)).unwrap(),
                    expected,
                    "{source} {input:?}"
                );
                assert!(
                    matches!(
                        run(Limits::new().with(Resource::MatchSteps, steps - 1)),
                        Err(Error::Resource(Refusal {
                            resource: Resource::MatchSteps,
                            ..
                        }))
                    ),
                    "{source} {input:?}"
                );
                required.push(steps);
            }
            assert!(
                required.iter().all(|&steps| steps == required[0]),
                "{input:?}: {required:?}"
            );
        }
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
