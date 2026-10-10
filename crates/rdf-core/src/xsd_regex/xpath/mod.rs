// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dated XPath pattern laws and their finite admission budgets.
//!
//! The dated law is explicit: [`Profile`] has no default. It is separate from
//! the compatibility compiler [`super::compile`] and its public regular-engine
//! representation. A resource refusal is a distinct [`Error::Resource`], never
//! a syntax verdict or a negative match.

use std::fmt;

mod cache;
mod compatibility;
mod compatibility_tables;
mod compile;
mod dated_blocks;
mod dated_names;
mod r#match;
mod pike;
mod replace;
mod sets;
mod unicode_tables;

pub use cache::PatternCache;
pub use compatibility::{
    CompatibilityPattern, OwnedCompatibilityPattern, compile_compatibility_with_storage,
};
pub use compile::{CompiledPattern, OwnedCompiledPattern, compile, compile_with_storage};
pub use r#match::Captures;
pub use r#match::OwnedCaptures;
pub use replace::OwnedReplacement;

/// A trusted ownership transformation at a native publication boundary.
///
/// Implementations must finish every fallible allocation and grant resize while
/// `value` remains covered by `storage`. Removing the original payload and its
/// grant is the final, infallible construction of an ownership-carrying output.
/// An independent copied output must acquire its own admission before allocation.
///
/// On an error, this method leaves any surviving original payload admitted. The
/// enclosing carrier destroys the remaining value before its original account.
pub trait Publication<T, S> {
    /// The published ownership-carrying result.
    type Output;
    /// The original typed allocation, admission or transformation failure.
    type Error;

    /// Transform the enclosed payload without a public raw extraction operation.
    ///
    /// # Errors
    /// Returns the publisher's original typed failure. All surviving native
    /// payload storage remains covered until the enclosing carrier is destroyed.
    fn publish(self, value: &mut Option<T>, storage: &mut S) -> Result<Self::Output, Self::Error>;
}

/// One immutable native value and the original physical account admitting it.
///
/// Payload destruction precedes account destruction. Immutable borrowed views
/// and the trusted ownership-carrying publication boundary preserve admission;
/// only borrowed views of the payload and its original typed account are public.
pub struct OwnedPatternValue<T, S> {
    value: T,
    storage: S,
}

impl<T, S> OwnedPatternValue<T, S> {
    pub(super) const fn new(value: T, storage: S) -> Self {
        Self { value, storage }
    }

    /// Borrow the payload while its physical admission remains alive.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Inspect the original caller's typed account without detaching it.
    #[must_use]
    pub const fn storage(&self) -> &S {
        &self.storage
    }

    /// Transfer a native value through a trusted ownership-carrying publisher.
    ///
    /// There is no raw `(value, account)` extraction. On refusal, the remaining
    /// payload dies before its original account; on success, the publisher's
    /// result owns the transferred payload and its grant together.
    ///
    /// # Errors
    /// Returns the publisher's original typed failure without erasing it.
    pub fn publish_with<P: Publication<T, S>>(self, publisher: P) -> Result<P::Output, P::Error> {
        let Self { value, storage } = self;
        // Same field order as the original carrier; wrapping is allocation-free.
        let mut pending = OwnedPatternValue::new(Some(value), storage);
        publisher.publish(&mut pending.value, &mut pending.storage)
    }
}

impl<T: fmt::Debug, S> fmt::Debug for OwnedPatternValue<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedPatternValue")
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

/// A failed native pattern operation with its original typed admission owner.
pub type OwnedPatternError<S> = OwnedPatternValue<Error, S>;

impl<S> fmt::Display for OwnedPatternError<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<S> std::error::Error for OwnedPatternError<S> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.value)
    }
}

/// The Recommendation that defines a pattern's grammar and matching law.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Profile {
    /// XPath F&O 2.0 Second Edition, 14 December 2010.
    ///
    /// <https://www.w3.org/TR/2010/REC-xpath-functions-20101214/#regex-syntax>
    Xpath20,
    /// XPath F&O 3.1, 21 March 2017.
    ///
    /// <https://www.w3.org/TR/2017/REC-xpath-functions-31-20170321/#regex-syntax>
    Xpath31,
}

impl Profile {
    /// The stable name of the dated grammar, including its edition date.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Xpath20 => "xpath-2.0-2010-12-14",
            Self::Xpath31 => "xpath-3.1-2017-03-21",
        }
    }

    /// Every dated law, oldest first.
    pub const ALL: [Self; 2] = [Self::Xpath20, Self::Xpath31];

    /// The dated law whose stable [`Self::name`] is exactly `name`.
    ///
    /// Binding and command-line surfaces select a law by this name. Matching is
    /// exact: no case folding, abbreviation or undated alias names a law, so an
    /// unknown name is refused rather than mapped to a guess.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|profile| profile.name() == name)
    }
}

/// The internal law carried by the one native program representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Law {
    Dated(Profile),
    Compatibility,
}

impl Law {
    pub(super) const fn is_compatibility(self) -> bool {
        matches!(self, Self::Compatibility)
    }
}

/// A separately admitted compiler, storage or execution resource.
///
/// A slot is one stored node operand, continuation, capture or repeat-counter
/// cell; variable-length storage accounts for each cell, not just its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Resource {
    /// UTF-8 bytes of the source, checked before materializing parser storage.
    PatternBytes,
    /// Compiler operations, including scanned characters and set operations.
    CompileSteps,
    /// Nodes in the flat compiled program.
    ProgramNodes,
    /// Cells retained during construction of the program.
    CompileSlots,
    /// Matcher operations, including search advances, compared code points
    /// and program transitions.
    MatchSteps,
    /// Simultaneously pending alternative states: the backtracking machine's
    /// pending alternatives, the thread machine's distinct control states at
    /// one input position, or the set machine's entries at one input position.
    MatchStates,
    /// Cells in all live states, captures and continuations, including the
    /// thread machine's thread lists, control-state table, work list and
    /// step cache, and the set machine's count sets, work list, state cache,
    /// kept reverse states and covered range.
    MatchSlots,
    /// UTF-8 bytes of replacement output.
    OutputBytes,
}

impl Resource {
    const COUNT: usize = 8;

    const fn index(self) -> usize {
        match self {
            Self::PatternBytes => 0,
            Self::CompileSteps => 1,
            Self::ProgramNodes => 2,
            Self::CompileSlots => 3,
            Self::MatchSteps => 4,
            Self::MatchStates => 5,
            Self::MatchSlots => 6,
            Self::OutputBytes => 7,
        }
    }

    /// A stable diagnostic code identifying the refused resource.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::PatternBytes => "xpath-pattern-bytes",
            Self::CompileSteps => "xpath-compile-steps",
            Self::ProgramNodes => "xpath-program-nodes",
            Self::CompileSlots => "xpath-compile-slots",
            Self::MatchSteps => "xpath-match-steps",
            Self::MatchStates => "xpath-match-states",
            Self::MatchSlots => "xpath-match-slots",
            Self::OutputBytes => "xpath-output-bytes",
        }
    }
}

/// Finite bounds on source admission, compilation, matching and replacement.
///
/// Fields are private so additional resources can be introduced without
/// changing downstream struct literals. Zero is a valid bound: it withholds
/// that resource rather than meaning unlimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    bounds: [u64; Resource::COUNT],
}

impl Limits {
    /// The production bounds, with every resource finite.
    ///
    /// Compilation is linear in the source, so the program and construction
    /// bounds admit every source the 64 KiB source bound admits: the largest
    /// measured requirement is about 3 nodes and 28 construction cells per
    /// source byte, for a literal under the x flag, including the parent table
    /// the matchers walk the program with. An oversized pattern is refused by
    /// its source bytes, before any parsing.
    ///
    /// Matching first runs the backtracking machine. A program without a
    /// backreference whose attempt spends more than 16 steps per search start
    /// it has passed, 2 per input byte it has examined (one greedy
    /// single-character run's own cost) and 64 per program node, or exceeds a
    /// live-storage bound, continues on the linear-time machines; a program
    /// with a backreference stays on the backtracking machine. Measured with
    /// the `native_xpath_large` bench group
    /// (`crates/rdf-core/benches/xsd_regex.rs`, release build, one pinned
    /// core), per input byte at these bounds:
    ///
    /// * the backtracking machine spends 1.4 steps on a literal search
    ///   through 1 MiB of filler and 2.0 on a greedy single-character run;
    /// * the set machine spends 1.00 to 1.02 steps deciding every counted
    ///   repetition the thread machine alone refused (`(ab){1,1000}c`,
    ///   `(ab){2,50}c`, `(ab|cd){1,20}e`, `((a|b){3}){5,9}c`,
    ///   `((a|b){2}){2,5}c`, `(a|b){1,30}c`, `(a|b){3,9}c`,
    ///   `(\w+\s){3,5}zzz` over 128 KiB to 4 MiB) and searches with several
    ///   unbounded runs in these measured fixtures, where each position's
    ///   state was seen before; a position whose state is new
    ///   costs its closure, about 95 steps for `(a|b){100000}c`;
    /// * a search for the match itself adds one reverse step per byte from the
    ///   end of the input, and one more for every stretch the walk reaches;
    ///   the walk spends about one step per byte of a match whose steps
    ///   repeat, so `^([a-z]+ ?)+$`, `^(\w+\s)*\w+$`, `^(a|b)*$`,
    ///   `^(ab)*$` and `^(a|aa)*$` matched whole cost 3.1 to 3.2 steps per
    ///   byte in all, over 1 MiB and over 64 MiB, and `(a|aa){1,1000}b`
    ///   closing a run of `a` 4.9;
    /// * an abandoned attempt has spent at most about 2: `node.*graph.*zzz`
    ///   without a match costs 3.0 steps per byte in all, over 1 MiB and over
    ///   64 MiB.
    ///
    /// Reverse states that never repeat cost a closure at every position, and
    /// the stretches the walk reaches are recomputed: `(a|aa){1,100000}b`
    /// over 1 MiB, whose reverse states hold the least number of further
    /// iterations a position needs, costs about 98 steps per byte, and an
    /// empty-preferring body below a minimum beyond `u64`,
    /// `(|a){18446744073709551616}b`, about 250 per byte of its match. The
    /// measured patterns keep a few states and a few hundred cells independent
    /// of the input length, beside a reverse state every 16 KiB. These figures
    /// do not establish admission for every finite repetition: large or nested
    /// counts can reach the step or storage bounds on much shorter input. A
    /// count of a counted repetition nested in another that both can repeat
    /// empty iterations is kept one entry per value. Its storage depends on
    /// the combination of counts; `((a?){100000}){100000}` is one measured
    /// storage refusal, not a minimum count threshold.
    ///
    /// A step costs 3.5 to 11 ns: 6.8 ns in the set machine at a cached
    /// position (one hash probe) and 3.5 ns at a new one, 3.7 to 10.5 ns in the
    /// reverse scan and walk of a search for a match, and 6.8 to 10.9 ns in the
    /// backtracking machine on the measured host. The measured low-cost
    /// searches admitted more than 64 MiB of input in under three seconds;
    /// this is neither a universal input threshold nor a timing guarantee.
    /// The step bound also refuses the exponential exploration that only a
    /// backreference can still reach, `^(a|aa)*c\1$` over forty `a`, after
    /// 2.7 s of work.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bounds: [
                64 * 1024,
                4_000_000,
                256 * 1024,
                2 * 1024 * 1024,
                250_000_000,
                64 * 1024,
                1024 * 1024,
                64 * 1024 * 1024,
            ],
        }
    }

    /// Remove application presets while keeping checked native counter ranges.
    ///
    /// The caller's physical admission still bounds every actual allocation.
    /// Counter/layout overflow remains a typed operational error. This does not
    /// silently install the dated production limits in an unselected evaluator.
    #[must_use]
    pub const fn without_presets() -> Self {
        Self {
            bounds: [u64::MAX; Resource::COUNT],
        }
    }

    /// These bounds with one named resource replaced.
    #[must_use]
    pub const fn with(mut self, resource: Resource, limit: u64) -> Self {
        self.bounds[resource.index()] = limit;
        self
    }

    /// The admitted maximum for `resource`.
    #[must_use]
    pub const fn limit(self, resource: Resource) -> u64 {
        self.bounds[resource.index()]
    }

    /// Admit an exact resource requirement before its allocation or work.
    ///
    /// `u128` retains an overflowing sum of two `u64` costs in the refusal;
    /// saturation cannot turn an impossible requirement into an admitted one.
    /// Live-storage callers pass the whole prospective live count here.
    ///
    /// # Errors
    ///
    /// A typed [`Refusal`] if the requirement exceeds this request's bound.
    pub fn admit(self, resource: Resource, required: u128) -> Result<(), Refusal> {
        let limit = self.limit(resource);
        if required > u128::from(limit) {
            Err(Refusal {
                resource,
                required,
                limit,
            })
        } else {
            Ok(())
        }
    }

    /// Admit source UTF-8 bytes before a parser or a compiled-program cache.
    ///
    /// # Errors
    ///
    /// [`Resource::PatternBytes`] is refused when the source is too large,
    /// independent of whether that source has valid pattern syntax.
    pub fn admit_pattern(self, pattern: &str) -> Result<(), Refusal> {
        self.admit(Resource::PatternBytes, pattern.len() as u128)
    }
}

purrdf_hash::default_from_new!(Limits);

/// An execution's accumulated work or retained compiler storage.
///
/// The counter is intentionally not clonable: copying a continuation must not
/// copy its remaining instruction budget. Live matcher storage uses
/// [`Limits::admit`] on the prospective whole live count instead of accumulating
/// allocations that have already been released.
#[derive(Debug)]
pub struct Budget<'storage> {
    memory: Option<
        purrdf_lex::allocation::Memory<'storage, dyn purrdf_lex::allocation::Admission + 'storage>,
    >,
    limits: Limits,
    used: [u64; Resource::COUNT],
    peak_compile_slots: u64,
}

impl<'storage> Budget<'storage> {
    /// A fresh execution under explicit finite bounds.
    #[must_use]
    pub const fn new(limits: Limits) -> Self {
        Self {
            memory: None,
            limits,
            used: [0; Resource::COUNT],
            peak_compile_slots: 0,
        }
    }

    /// The same instruction/slot law with actual physical storage admission.
    pub(super) fn with_storage(
        limits: Limits,
        storage: &'storage mut dyn purrdf_lex::allocation::Admission,
    ) -> Self {
        Self {
            memory: Some(purrdf_lex::allocation::Memory::new(storage)),
            ..Self::new(limits)
        }
    }

    pub(super) fn reserve<T>(&mut self, values: &mut Vec<T>, required: usize) -> Result<(), Error> {
        self.reserve_resource(values, required, Resource::CompileSlots, required as u64)
    }

    pub(super) fn reserve_match<T>(
        &mut self,
        values: &mut Vec<T>,
        required: usize,
        units: u64,
    ) -> Result<(), Error> {
        self.reserve_resource(values, required, Resource::MatchSlots, units)
    }

    fn reserve_resource<T>(
        &mut self,
        values: &mut Vec<T>,
        required: usize,
        resource: Resource,
        units: u64,
    ) -> Result<(), Error> {
        if let Some(memory) = &mut self.memory {
            return memory.reserve(values, required).map_err(Error::Storage);
        }
        if required > values.capacity() {
            values
                .try_reserve_exact(required - values.len())
                .map_err(|_| Error::Allocation { resource, units })?;
        }
        Ok(())
    }

    pub(super) fn push_work<T, const N: usize>(
        &mut self,
        work: &mut purrdf_lex::walk::WorkList<T, N>,
        value: T,
        resource: Resource,
        units: u64,
    ) -> Result<(), Error> {
        if let Some(memory) = &mut self.memory {
            return work
                .try_push_admitted(value, memory)
                .map_err(Error::Storage);
        }
        work.try_push(value)
            .map_err(|_| Error::Allocation { resource, units })
    }

    pub(super) fn release_work<T, const N: usize>(
        &mut self,
        work: purrdf_lex::walk::WorkList<T, N>,
    ) -> Result<(), Error> {
        if let Some(memory) = &mut self.memory {
            return work.release_admitted(memory).map_err(Error::Storage);
        }
        drop(work);
        Ok(())
    }

    /// Only after the abandoned machine and its pending states have died.
    pub(super) fn release_abandoned_machine(&mut self) -> Result<(), Error> {
        if let Some(memory) = &mut self.memory {
            memory
                .release_bytes(memory.admitted_bytes())
                .map_err(Error::Storage)?;
        }
        Ok(())
    }

    /// The one rendered diagnostic destination, with every physical growth admitted.
    pub(super) fn format(&mut self, value: &(impl fmt::Display + ?Sized)) -> Result<String, Error> {
        match &mut self.memory {
            Some(memory) => memory.format(value).map_err(Error::Storage),
            None => Ok(value.to_string()),
        }
    }

    /// The scanner's original error spelling, admitted before its owned copy.
    pub(super) fn string(&mut self, input: &str) -> Result<String, Error> {
        purrdf_lex::allocation::string_with_reserve(input, |text, required| {
            self.reserve_string(text, required)
        })
    }

    pub(super) fn reserve_string(
        &mut self,
        value: &mut String,
        required: usize,
    ) -> Result<(), Error> {
        if let Some(memory) = &mut self.memory {
            return memory
                .reserve_string(value, required)
                .map_err(Error::Storage);
        }
        if required > value.capacity() {
            value
                .try_reserve_exact(required - value.len())
                .map_err(|_| Error::Allocation {
                    resource: Resource::CompileSlots,
                    units: required as u64,
                })?;
        }
        Ok(())
    }

    pub(super) fn release_vec<T>(&mut self, values: Vec<T>) -> Result<(), Error> {
        if let Some(memory) = &mut self.memory {
            return memory.release_vec(values).map_err(Error::Storage);
        }
        drop(values);
        Ok(())
    }

    pub(super) fn release_string(&mut self, value: String) -> Result<(), Error> {
        if let Some(memory) = &mut self.memory {
            return memory.release_string(value).map_err(Error::Storage);
        }
        drop(value);
        Ok(())
    }

    pub(super) fn release_physical(&mut self, bytes: usize) -> Result<(), Error> {
        if let Some(memory) = &mut self.memory {
            memory.release_bytes(bytes).map_err(Error::Storage)?;
        }
        Ok(())
    }

    /// The unchanged bounds of this execution.
    #[must_use]
    pub const fn limits(&self) -> Limits {
        self.limits
    }

    /// The amount of a resource already charged by this execution.
    #[must_use]
    pub const fn used(&self, resource: Resource) -> u64 {
        self.used[resource.index()]
    }

    /// Charge before doing the work; a refusal leaves the counter unchanged.
    ///
    /// # Errors
    ///
    /// A typed [`Refusal`] when the exact accumulated requirement exceeds the
    /// bound. Even `u64::MAX + 1` remains a refusal with its exact requirement.
    #[inline]
    pub fn charge(&mut self, resource: Resource, amount: u64) -> Result<(), Refusal> {
        let index = resource.index();
        // The common admitted case, without the wide arithmetic of a refusal.
        if resource != Resource::CompileSlots
            && let Some(sum) = self.used[index].checked_add(amount)
            && sum <= self.limits.bounds[index]
        {
            self.used[index] = sum;
            return Ok(());
        }
        self.charge_wide(resource, u128::from(amount))
    }

    /// Internal allocation arithmetic is retained before a host-sized cast.
    pub(super) fn charge_wide(&mut self, resource: Resource, amount: u128) -> Result<(), Refusal> {
        let required = u128::from(self.used(resource)) + amount;
        self.limits.admit(resource, required)?;
        // Admission proves this sum fits the u64 limit as well as its counter.
        self.used[resource.index()] = u64::try_from(required)
            .expect("admission proves the exact requirement fits its u64 bound");
        if resource == Resource::CompileSlots {
            self.peak_compile_slots = self.peak_compile_slots.max(self.used(resource));
        }
        Ok(())
    }

    /// Release only compiler storage whose owner has actually been dropped.
    pub(super) fn release_compile_slots(&mut self, amount: u64) {
        self.used[Resource::CompileSlots.index()] -= amount;
    }
}

/// A precise operational refusal, disjoint from an invalid pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Refusal {
    /// The exhausted resource.
    pub resource: Resource,
    /// The exact requirement, including requirements beyond `u64::MAX`.
    pub required: u128,
    /// The current request's finite bound.
    pub limit: u64,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: requires {}, limit {}",
            self.resource.code(),
            self.required,
            self.limit
        )
    }
}

impl std::error::Error for Refusal {}

/// A syntax verdict or an operational refusal from the native XPath surface.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A flag is not defined by the selected dated law (FORX0001).
    Flags {
        /// UTF-8 byte offset within the flags string.
        offset: usize,
        /// The rejected flag character.
        flag: char,
        /// The law against which the flags were admitted.
        profile: Profile,
    },
    /// A flag outside the existing unselected compatibility alphabet.
    CompatibilityFlags {
        /// UTF-8 byte offset within the flag text.
        offset: usize,
        /// The rejected character.
        flag: char,
    },
    /// The existing compatibility surface's authored source-byte contract.
    CompatibilitySource {
        /// Actual source UTF-8 bytes.
        bytes: usize,
        /// The compatibility source limit.
        limit: usize,
    },
    /// Invalid grammar under the explicitly selected dated profile.
    Syntax {
        /// UTF-8 byte offset of the offending construct.
        offset: usize,
        /// The rejected construct and the violated rule.
        message: String,
    },
    /// A compiler, matcher or replacement resource was withheld.
    Resource(Refusal),
    /// The host refused an admitted storage allocation.
    Allocation {
        /// The kind of storage the host could not provide.
        resource: Resource,
        /// The requested storage units.
        units: u64,
    },
    /// Concrete native buffer layout, allocator or caller-admission failure.
    Storage(purrdf_lex::allocation::StorageError),
    /// The replacement pattern matches the empty string (FORX0003).
    EmptyMatch,
    /// The replacement text violates the shared XPath grammar (FORX0004).
    Replacement(super::ReplacementError),
}

purrdf_lex::variant_from!(Error { Resource(Refusal), Replacement(super::ReplacementError) });

impl Error {
    /// Whether this is an operational failure rather than a pattern,
    /// flag or replacement-language error.
    #[must_use]
    pub const fn is_operational(&self) -> bool {
        matches!(
            self,
            Self::Resource(_) | Self::Allocation { .. } | Self::Storage(_)
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flags {
                offset,
                flag,
                profile,
            } => write!(
                f,
                "invalid XPath flag {flag:?} at byte {offset} for {} (err:FORX0001)",
                profile.name()
            ),
            Self::CompatibilityFlags { offset, flag } => write!(
                f,
                "invalid compatibility pattern flag {flag:?} at byte {offset}"
            ),
            Self::CompatibilitySource { bytes, limit } => write!(
                f,
                "compatibility pattern has {bytes} source bytes, limit {limit}"
            ),
            Self::Syntax { offset, message } => {
                write!(f, "invalid XPath pattern at byte {offset}: {message}")
            }
            Self::Resource(refusal) => refusal.fmt(f),
            Self::Storage(error) => error.fmt(f),
            Self::Allocation { resource, units } => write!(
                f,
                "{}: host refused allocation of {units} units",
                resource.code()
            ),
            Self::EmptyMatch => {
                f.write_str("replacement pattern matches the empty string (err:FORX0003)")
            }
            Self::Replacement(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resource(refusal) => Some(refusal),
            Self::Replacement(error) => Some(error),
            Self::Flags { .. }
            | Self::CompatibilityFlags { .. }
            | Self::CompatibilitySource { .. }
            | Self::Syntax { .. }
            | Self::Allocation { .. }
            | Self::Storage(_)
            | Self::EmptyMatch => None,
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{Budget, Error, Limits, Profile, Resource};

    #[test]
    fn source_admission_counts_utf8_bytes_and_refuses_before_syntax() {
        let limits = Limits::new().with(Resource::PatternBytes, 2);
        assert!(limits.admit_pattern("é").is_ok());
        assert!(limits.admit_pattern("[(").is_ok());
        for pattern in ["éa", "[(("] {
            let refusal = limits.admit_pattern(pattern).unwrap_err();
            assert_eq!(refusal.resource, Resource::PatternBytes);
            assert_eq!(refusal.required, 3);
            assert_eq!(refusal.limit, 2);
            assert!(matches!(Error::from(refusal), Error::Resource(_)));
        }
    }

    #[test]
    fn accumulated_work_refuses_exactly_without_spending_or_wrapping() {
        let mut budget = Budget::new(Limits::new().with(Resource::MatchSteps, u64::MAX));
        budget.charge(Resource::MatchSteps, u64::MAX).unwrap();
        let refusal = budget.charge(Resource::MatchSteps, 1).unwrap_err();
        assert_eq!(refusal.required, u128::from(u64::MAX) + 1);
        assert_eq!(refusal.limit, u64::MAX);
        assert_eq!(budget.used(Resource::MatchSteps), u64::MAX);
        assert!(budget.charge(Resource::MatchSteps, 0).is_ok());
    }

    #[test]
    fn current_admission_is_not_the_previous_executions_admission() {
        let high = Limits::new().with(Resource::ProgramNodes, 4);
        let low = high.with(Resource::ProgramNodes, 3);
        assert!(high.admit(Resource::ProgramNodes, 4).is_ok());
        assert!(low.admit(Resource::ProgramNodes, 4).is_err());
        assert!(low.admit(Resource::ProgramNodes, 3).is_ok());
        let mut first = Budget::new(high.with(Resource::MatchSteps, 2));
        first.charge(Resource::MatchSteps, 2).unwrap();
        assert!(first.charge(Resource::MatchSteps, 1).is_err());
        let mut second = Budget::new(first.limits());
        assert!(second.charge(Resource::MatchSteps, 1).is_ok());
        assert_eq!(first.used(Resource::MatchSteps), 2);
    }

    #[test]
    fn zero_withholds_work_and_all_resources_have_independent_bounds() {
        let resources = [
            Resource::PatternBytes,
            Resource::CompileSteps,
            Resource::ProgramNodes,
            Resource::CompileSlots,
            Resource::MatchSteps,
            Resource::MatchStates,
            Resource::MatchSlots,
            Resource::OutputBytes,
        ];
        for resource in resources {
            let limits = Limits::new().with(resource, 0);
            assert!(limits.admit(resource, 0).is_ok());
            assert!(limits.admit(resource, 1).is_err());
            for other in resources {
                if other != resource {
                    assert!(limits.admit(other, 1).is_ok());
                }
            }
        }
    }

    #[test]
    fn dated_laws_have_distinct_stable_names() {
        assert_eq!(Profile::Xpath20.name(), "xpath-2.0-2010-12-14");
        assert_eq!(Profile::Xpath31.name(), "xpath-3.1-2017-03-21");
        assert_ne!(Profile::Xpath20, Profile::Xpath31);
        for profile in Profile::ALL {
            assert_eq!(Profile::from_name(profile.name()), Some(profile));
        }
        for unknown in [
            "",
            "xpath-3.1",
            "XPATH-3.1-2017-03-21",
            "xpath-3.1-2017-03-21 ",
            "xpath-4.0",
        ] {
            assert_eq!(Profile::from_name(unknown), None, "{unknown:?}");
        }
    }
}

#[cfg(test)]
mod publication_fixtures {
    use super::{OwnedPatternValue, Publication};
    use std::cell::Cell;

    struct Value<'a>(&'a Cell<u8>);
    struct Grant<'a>(&'a Cell<u8>);

    impl Drop for Value<'_> {
        fn drop(&mut self) {
            assert_eq!(self.0.replace(1), 0, "value must die before grant");
        }
    }
    impl Drop for Grant<'_> {
        fn drop(&mut self) {
            assert_eq!(self.0.replace(2), 1, "grant covers value destruction");
        }
    }

    struct Refuse;
    impl<'a> Publication<Value<'a>, Option<Grant<'a>>> for Refuse {
        type Output = ();
        type Error = &'static str;
        fn publish(
            self,
            _: &mut Option<Value<'a>>,
            _: &mut Option<Grant<'a>>,
        ) -> Result<(), &'static str> {
            Err("original admission refusal")
        }
    }

    struct Published<'a> {
        _value: Value<'a>,
        _grant: Grant<'a>,
    }
    struct Transfer;
    impl<'a> Publication<Value<'a>, Option<Grant<'a>>> for Transfer {
        type Output = Published<'a>;
        type Error = &'static str;
        fn publish(
            self,
            value: &mut Option<Value<'a>>,
            storage: &mut Option<Grant<'a>>,
        ) -> Result<Self::Output, Self::Error> {
            // All fallible work is complete; only the final owner is constructed.
            Ok(Published {
                _value: value.take().unwrap(),
                _grant: storage.take().unwrap(),
            })
        }
    }

    #[test]
    fn publication_refusal_keeps_original_payload_before_grant_drop_order() {
        let order = Cell::new(0);
        let value = OwnedPatternValue::new(Value(&order), Some(Grant(&order)));
        assert_eq!(
            value.publish_with(Refuse),
            Err("original admission refusal")
        );
        assert_eq!(order.get(), 2);
    }

    #[test]
    fn successful_publication_has_no_gap_between_native_and_result_owners() {
        let order = Cell::new(0);
        let value = OwnedPatternValue::new(Value(&order), Some(Grant(&order)));
        let published = value.publish_with(Transfer).unwrap();
        assert_eq!(order.get(), 0);
        drop(published);
        assert_eq!(order.get(), 2);
    }
}
