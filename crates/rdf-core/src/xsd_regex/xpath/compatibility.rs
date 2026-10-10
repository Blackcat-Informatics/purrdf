// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The existing unselected evaluator law over the one admitted native program.
//!
//! Its Unicode 16 simple folding, shared modern XML name terminals, exact
//! Unicode 16 block names, final-newline anchors and empty replacement matches
//! remain separate from explicitly selected dated XPath laws. No regular-engine
//! compiler or matcher participates in this owned production path.

use purrdf_lex::allocation::Admission;

use super::{
    CompiledPattern, Law, Limits, OwnedCaptures, OwnedPatternError, OwnedPatternValue,
    OwnedReplacement,
};

/// The immutable unselected law, without an exposed dated-program view.
#[derive(Debug)]
pub struct CompatibilityPattern {
    pattern: CompiledPattern,
}

impl CompatibilityPattern {
    /// The exact original source and flags belonging to this retained program.
    #[must_use]
    pub fn matches_source(&self, pattern: &str, flags: &str) -> bool {
        self.pattern.law == Law::Compatibility
            && self.pattern.source() == pattern
            && self.pattern.flags() == flags
    }

    /// Original source text, before x-flag normalization.
    #[must_use]
    pub fn source(&self) -> &str {
        self.pattern.source()
    }

    /// Original flags, including legal repeated flags.
    #[must_use]
    pub fn flags(&self) -> &str {
        self.pattern.flags()
    }

    /// Capturing groups, excluding the complete match at capture zero.
    #[must_use]
    pub const fn capture_count(&self) -> usize {
        self.pattern.capture_count()
    }

    /// Native retained program capacity; a cache fee, never a physical proof.
    #[must_use]
    pub fn storage_bytes(&self) -> usize {
        self.pattern.storage_bytes()
    }

    /// Recheck the caller's current explicit logical resource bounds.
    ///
    /// # Errors
    /// Returns the original typed native resource refusal.
    pub fn admit(&self, limits: Limits) -> Result<(), super::Error> {
        self.pattern.admit(limits)
    }

    /// Match with the native control-state machine and original physical owner.
    ///
    /// # Errors
    /// Preserves allocator, layout, explicit resource and capacity refusals.
    pub fn is_match_with_storage<S: Admission>(
        &self,
        input: &str,
        limits: Limits,
        storage: S,
    ) -> Result<OwnedPatternValue<bool, S>, OwnedPatternError<S>> {
        self.pattern.is_match_with_storage(input, limits, storage)
    }

    /// Find the original ordered captures with an ownership-carrying result.
    ///
    /// # Errors
    /// Preserves the original physical owner and every operational refusal.
    pub fn find_with_storage<S: Admission>(
        &self,
        input: &str,
        limits: Limits,
        storage: S,
    ) -> Result<OwnedCaptures<S>, OwnedPatternError<S>> {
        self.pattern.find_with_storage(input, limits, storage)
    }

    /// Replace ordered nonoverlapping matches, including legal empty matches.
    ///
    /// The shared XPath replacement cursor still parses the complete template
    /// before searching. Empty matches advance at UTF-8 boundaries; an empty
    /// match immediately following a nonempty match is suppressed, as before.
    ///
    /// # Errors
    /// Preserves both original matcher and output physical owners. Malformed
    /// replacement text remains lexical; no partial replacement is published.
    pub fn replace_all_with_storage<'h, M: Admission, O: Admission>(
        &self,
        input: &'h str,
        replacement: &str,
        limits: Limits,
        matcher: M,
        output: O,
    ) -> Result<OwnedReplacement<'h, M, O>, OwnedPatternError<(M, O)>> {
        self.pattern
            .replace_all_with_storage(input, replacement, limits, matcher, output)
    }
}

/// An unselected native program retaining its original concrete account.
pub type OwnedCompatibilityPattern<S> = OwnedPatternValue<CompatibilityPattern, S>;

/// Compile the existing unselected law through the same native parser/arenas.
///
/// `limits` are explicit caller bounds. Use [`Limits::without_presets`] when
/// no dated application preset was selected; actual physical admission still
/// precedes every compiler allocation. The public legacy `super::super::compile`
/// remains unchanged for consumers of its `as_regex` representation.
///
/// # Errors
/// Returns the original account alongside grammar, work, layout, allocator and
/// admission failures. Failed requests are never immutable cached programs.
pub fn compile_compatibility_with_storage<S: Admission>(
    pattern: &str,
    flags: &str,
    limits: Limits,
    storage: S,
) -> Result<OwnedCompatibilityPattern<S>, OwnedPatternError<S>> {
    super::compile::compile_owned_law(
        Law::Compatibility,
        pattern,
        flags,
        limits,
        storage,
        |pattern| CompatibilityPattern { pattern },
    )
}
