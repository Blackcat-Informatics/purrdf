// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit loss-code meanings and projection-pair membership.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::OnceLock;

type Descriptions = BTreeMap<Cow<'static, str>, Cow<'static, str>>;
type Targets = BTreeMap<Cow<'static, str>, Descriptions>;
type Registry = BTreeMap<Cow<'static, str>, Targets>;

/// One registered loss meaning for a particular projection pair.
///
/// The description defines the code; a runtime occurrence's `LossEntry::note`
/// may instead explain the particular input that caused that loss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossDefinition {
    /// Source representation.
    pub from: Cow<'static, str>,
    /// Target representation.
    pub to: Cow<'static, str>,
    /// Stable machine code.
    pub code: Cow<'static, str>,
    /// Registered meaning of this code for this pair.
    pub description: Cow<'static, str>,
}

impl purrdf_hash::error::StructuredError for LossDefinition {
    purrdf_hash::error_visit! {
        @record self { from, to, code, description } => "loss-definition" {
            "from" => Code(self.from.as_ref()), "to" => Code(self.to.as_ref()),
            "code" => Code(self.code.as_ref()), "description" => Text(self.description.as_ref()),
        }
    }
}

/// A registration tried to give an existing code a conflicting meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossProfileError {
    /// Original rejected registration, including its owned strings.
    pub definition: LossDefinition,
}

impl purrdf_hash::error::StructuredError for LossProfileError {
    purrdf_hash::error_visit! {
        @record self { definition } => "loss-profile-conflict" {
            "definition" => Record(&self.definition),
        }
    }
}

impl std::fmt::Display for LossProfileError {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            output,
            "loss code {:?} has a conflicting registered description for ({:?}, {:?})",
            self.definition.code, self.definition.from, self.definition.to
        )
    }
}

impl std::error::Error for LossProfileError {}

/// Caller-owned loss meanings and projection pairs, selected explicitly.
///
/// Builtin profiles borrow the immutable builtin registry. Extending one owns
/// its registry; no custom registration changes another caller or process state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossProfile {
    registry: Cow<'static, Registry>,
}

impl LossProfile {
    /// Select the complete builtin registry without copying its rows.
    pub fn builtin() -> Self {
        Self {
            registry: Cow::Borrowed(builtin_registry()),
        }
    }

    /// Add projection pairs and codes, returning a new complete profile.
    ///
    /// An exact repeated definition is idempotent. An existing pair/code may
    /// not change description. Reusing a code on another pair requires one of
    /// that code's existing descriptions, preserving builtin pair-specific
    /// meanings. Failure returns the original rejected definition and exposes
    /// no partially extended profile.
    pub fn extend(
        mut self,
        definitions: impl IntoIterator<Item = LossDefinition>,
    ) -> Result<Self, LossProfileError> {
        for definition in definitions {
            let exact = self.description(&definition.from, &definition.to, &definition.code);
            if exact == Some(definition.description.as_ref()) {
                continue;
            }
            let mut known = false;
            let mut same_meaning = false;
            for targets in self.registry.values() {
                for codes in targets.values() {
                    if let Some(description) = codes.get(definition.code.as_ref()) {
                        known = true;
                        same_meaning |= description == &definition.description;
                    }
                }
            }
            if exact.is_some() || (known && !same_meaning) {
                return Err(LossProfileError { definition });
            }
            self.registry
                .to_mut()
                .entry(definition.from)
                .or_default()
                .entry(definition.to)
                .or_default()
                .insert(definition.code, definition.description);
        }
        Ok(self)
    }

    /// Registered meaning for exactly this projection pair and code.
    pub fn description(&self, from: &str, to: &str, code: &str) -> Option<&str> {
        self.registry
            .get(from)?
            .get(to)?
            .get(code)
            .map(AsRef::as_ref)
    }

    /// Enumerate registered pairs in canonical source/target order.
    pub fn registered_pairs(&self) -> impl Iterator<Item = (&str, &str)> {
        pairs(&self.registry)
    }

    /// Enumerate this pair's codes in canonical order; unknown pairs are empty.
    pub fn codes_for(&self, from: &str, to: &str) -> impl Iterator<Item = &str> {
        codes(&self.registry, from, to)
    }
}

fn builtin_registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = Registry::new();
        for entry in super::registry_entries() {
            let previous = registry
                .entry(entry.from)
                .or_default()
                .entry(entry.to)
                .or_default()
                .insert(entry.code, entry.note);
            assert!(
                previous.is_none(),
                "builtin pair/code definitions must be unique"
            );
        }
        registry
    })
}

fn pairs(registry: &Registry) -> impl Iterator<Item = (&str, &str)> {
    registry
        .iter()
        .flat_map(|(from, targets)| targets.keys().map(move |to| (from.as_ref(), to.as_ref())))
}

fn codes<'a>(registry: &'a Registry, from: &str, to: &str) -> impl Iterator<Item = &'a str> {
    registry
        .get(from)
        .and_then(|targets| targets.get(to))
        .into_iter()
        .flat_map(BTreeMap::keys)
        .map(AsRef::as_ref)
}

pub(super) fn builtin_pairs() -> impl Iterator<Item = (&'static str, &'static str)> {
    pairs(builtin_registry())
}

pub(super) fn builtin_codes(from: &str, to: &str) -> impl Iterator<Item = &'static str> {
    codes(builtin_registry(), from, to)
}

/// Original borrowed code/pair operands of an unsound ledger entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LossSoundnessError<'a> {
    /// The entry belongs to a different conversion than the one being checked.
    PairMismatch {
        /// Original recorded code.
        code: &'a str,
        /// Original recorded source.
        actual_from: &'a str,
        /// Original recorded target.
        actual_to: &'a str,
        /// Requested source.
        expected_from: &'a str,
        /// Requested target.
        expected_to: &'a str,
    },
    /// This code is not registered for its conversion pair.
    UnregisteredCode {
        /// Original recorded code.
        code: &'a str,
        /// Original recorded source.
        from: &'a str,
        /// Original recorded target.
        to: &'a str,
    },
}

impl purrdf_hash::error::StructuredError for LossSoundnessError<'_> {
    purrdf_hash::error_visit! {
        PairMismatch { code, actual_from, actual_to, expected_from, expected_to } => "loss-pair-mismatch" {
            "code" => Code(code), "actual_from" => Code(actual_from), "actual_to" => Code(actual_to),
            "expected_from" => Code(expected_from), "expected_to" => Code(expected_to),
        },
        UnregisteredCode { code, from, to } => "loss-unregistered-code" {
            "code" => Code(code), "from" => Code(from), "to" => Code(to),
        },
    }
}

impl std::fmt::Display for LossSoundnessError<'_> {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PairMismatch {
                code,
                actual_from,
                actual_to,
                expected_from,
                expected_to,
            } => write!(
                output,
                "loss ledger code {code:?} belongs to ({actual_from:?}, {actual_to:?}), expected ({expected_from:?}, {expected_to:?})"
            ),
            Self::UnregisteredCode { code, from, to } => write!(
                output,
                "loss ledger unsound for ({from:?}, {to:?}): code {code:?} is not in the declared profile"
            ),
        }
    }
}

impl std::error::Error for LossSoundnessError<'_> {}
