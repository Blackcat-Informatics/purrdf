// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dated XPath pattern laws and their finite admission budgets.
//!
//! The dated law is explicit: [`Profile`] has no default. It is separate from
//! the compatibility compiler [`super::compile`] and its public regular-engine
//! representation. A resource refusal is a distinct [`Error::Resource`], never
//! a syntax verdict or a negative match.

use std::fmt;

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
    /// Matcher operations, including search advances and compared code points.
    MatchSteps,
    /// Simultaneously pending alternative states.
    MatchStates,
    /// Cells in all live states, captures and continuations.
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
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bounds: [
                64 * 1024,
                4_000_000,
                64 * 1024,
                256 * 1024,
                10_000_000,
                64 * 1024,
                1024 * 1024,
                64 * 1024 * 1024,
            ],
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
pub struct Budget {
    limits: Limits,
    used: [u64; Resource::COUNT],
}

impl Budget {
    /// A fresh execution under explicit finite bounds.
    #[must_use]
    pub const fn new(limits: Limits) -> Self {
        Self {
            limits,
            used: [0; Resource::COUNT],
        }
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
    pub fn charge(&mut self, resource: Resource, amount: u64) -> Result<(), Refusal> {
        let required = u128::from(self.used(resource)) + u128::from(amount);
        self.limits.admit(resource, required)?;
        // Admission proves this sum fits the u64 limit as well as its counter.
        self.used[resource.index()] += amount;
        Ok(())
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
}

purrdf_lex::variant_from!(Error { Resource(Refusal) });

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax { offset, message } => {
                write!(f, "invalid XPath pattern at byte {offset}: {message}")
            }
            Self::Resource(refusal) => refusal.fmt(f),
            Self::Allocation { resource, units } => write!(
                f,
                "{}: host refused allocation of {units} units",
                resource.code()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resource(refusal) => Some(refusal),
            Self::Syntax { .. } | Self::Allocation { .. } => None,
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
    }
}
