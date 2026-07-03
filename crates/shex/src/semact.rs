// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Semantic-action dispatch (ShEx 2.1 spec §5.5.2 / ShExJ `SemAct`).
//!
//! A schema may attach semantic actions — `%<iri>{ code %}` / `%<iri>%` — to
//! the schema (`startActs`), a shape, a triple-expression group, or a triple
//! constraint. During validation each action is dispatched to an **extension**
//! registered for its IRI; the extension returns `true` (the action succeeds)
//! or `false` (the enclosing element fails to match).
//!
//! * **Registry, not evaluation.** [`SemActRegistry`] maps an extension IRI to
//!   a boolean [`SemActExtension`] closure. Arbitrary code evaluation is out of
//!   scope; an extension decides success from the action's code and context.
//! * **Inert by default.** An action whose IRI has no registered extension is
//!   a no-op that succeeds, so a schema carrying actions this engine does not
//!   understand still validates by its structural semantics.
//! * **The Test extension.** [`SemActRegistry::with_test`] ships the
//!   `http://shex.io/extensions/Test/` extension used by the shexTest suite:
//!   `fail(...)` code fails, everything else (`print(...)`, no code) succeeds.

use std::collections::HashMap;

use purrdf_core::TermValue;

use crate::ast::SemAct;

/// The shexTest `Test` semantic-action extension IRI.
pub const TEST_EXTENSION: &str = "http://shex.io/extensions/Test/";

/// The context in which a semantic action fires.
///
/// Field presence is exact per firing position, not best-effort:
///
/// * **Start actions** (schema `startActs` / query-level actions): all three
///   fields are `None` — they fire once for the whole shape map, before any
///   focus node is chosen.
/// * **Shape and `EachOf`/`OneOf` group actions**: `focus` is the node that
///   matched the shape; `predicate` and `value` are `None` (no single arc is
///   implicated).
/// * **Triple-constraint actions**: fired once per triple the constraint
///   matched. `focus` is the node that matched the shape, `predicate` is the
///   constraint's predicate IRI, and `value` is that triple's value node
///   (the object for a forward arc, the subject for `^` inverse) — all three
///   are always `Some`. A constraint matching zero triples does not fire.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SemActContext {
    /// The focus node being validated, when concrete.
    pub focus: Option<TermValue>,
    /// The predicate IRI of the matched arc (triple-constraint position).
    pub predicate: Option<String>,
    /// The matched arc's value node (object for forward arcs, subject for
    /// inverse), for a triple-constraint firing.
    pub value: Option<TermValue>,
}

/// An extension: decides whether one [`SemAct`] succeeds in a [`SemActContext`].
pub type SemActExtension<'a> = dyn Fn(&SemAct, &SemActContext) -> bool + 'a;

/// A mapping from extension IRI to its [`SemActExtension`].
///
/// Unregistered IRIs dispatch to a success no-op (see the module doc).
#[derive(Default)]
pub struct SemActRegistry<'a> {
    extensions: HashMap<String, Box<SemActExtension<'a>>>,
}

impl<'a> SemActRegistry<'a> {
    /// An empty registry (every action is inert / succeeds).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A registry carrying the built-in `http://shex.io/extensions/Test/`
    /// extension.
    #[must_use]
    pub fn with_test() -> Self {
        let mut registry = Self::new();
        registry.register(TEST_EXTENSION, Box::new(test_extension));
        registry
    }

    /// Register `ext` for extension IRI `iri`, replacing any prior binding.
    pub fn register(&mut self, iri: impl Into<String>, ext: Box<SemActExtension<'a>>) -> &mut Self {
        self.extensions.insert(iri.into(), ext);
        self
    }

    /// Dispatch a single action. An unregistered IRI is an inert success.
    #[must_use]
    pub fn dispatch(&self, act: &SemAct, ctx: &SemActContext) -> bool {
        self.extensions
            .get(&act.name)
            .is_none_or(|ext| ext(act, ctx))
    }

    /// Dispatch every action, short-circuiting on the first failure.
    #[must_use]
    pub fn dispatch_all(&self, acts: &[SemAct], ctx: &SemActContext) -> bool {
        acts.iter().all(|act| self.dispatch(act, ctx))
    }

    /// `true` when no extension is registered (all actions inert).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }
}

impl core::fmt::Debug for SemActRegistry<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut keys: Vec<&str> = self.extensions.keys().map(String::as_str).collect();
        keys.sort_unstable();
        f.debug_struct("SemActRegistry")
            .field("extensions", &keys)
            .finish()
    }
}

/// The `Test` extension: `fail(...)` fails; no code and everything else
/// (notably `print(...)`) succeeds.
fn test_extension(act: &SemAct, _ctx: &SemActContext) -> bool {
    match &act.code {
        None => true,
        Some(code) => !code.trim_start().starts_with("fail"),
    }
}
