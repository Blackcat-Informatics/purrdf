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
//!   understand still validates by its structural semantics. This is an
//!   intentional reading of the spec — an unrecognized semantic-action IRI does
//!   not fail validation — and is therefore distinct from a swallowed error: no
//!   outcome is discarded, the action simply carries no constraint here.
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
///
/// # What may precede the function name
///
/// The extension publishes its own grammar as one regular expression, and that
/// expression is the whole law for the leading run:
///
/// ```text
/// /^ *(fail|print) *\( *(?:("(?:[^\\"]|\\\\|\\")*")|([spo])) *\) *$/
/// ```
///
/// `^ *` is **zero or more U+0020 SPACE characters** — not `\s`, not a Unicode
/// property, and not a tab. So the leading run is stripped with
/// [`str::trim_start_matches`] over the single character the extension names,
/// never with [`str::trim_start`], which answers the 26-member Unicode
/// `White_Space` property.
///
/// The difference here is an **over-refusal**, which is the direction that
/// hides. `"\u{A0}fail(\"x\")"` does not match the expression above, so it is
/// not the extension's `fail` and the action must SUCCEED — the module doc's
/// "everything else succeeds". Under [`str::trim_start`] the U+00A0 was stripped
/// away, the code read as `fail`, and a schema that validates under the
/// published extension was reported as failing. Nothing looked broken: a
/// refusal reads as correct strictness right up until someone writes the schema
/// that should validate and does not.
///
/// This still reads the code by its **prefix** rather than by matching the
/// whole expression: `fail` is recognized where the extension would also
/// require the parenthesized argument. That looseness is unchanged here and is
/// deliberate — it is what the shexTest suite exercises — but it is a reading
/// of the function name, not of white space, and it is stated so a later reader
/// does not mistake this function for a transcription of the full grammar.
fn test_extension(act: &SemAct, _ctx: &SemActContext) -> bool {
    match &act.code {
        None => true,
        Some(code) => !code
            .trim_start_matches(TEST_LEADING_RUN)
            .starts_with("fail"),
    }
}

/// The `^ *` of the `Test` extension's published expression: U+0020 SPACE, and
/// no other character, may precede the function name.
const TEST_LEADING_RUN: char = ' ';

#[cfg(test)]
mod test_extension_leading_run {
    use super::{SemActContext, SemActRegistry, TEST_EXTENSION, TEST_LEADING_RUN, test_extension};
    use crate::ast::SemAct;
    use pretty_assertions::assert_eq;

    fn act(code: &str) -> SemAct {
        SemAct {
            name: TEST_EXTENSION.to_owned(),
            code: Some(code.to_owned()),
        }
    }

    #[test]
    fn only_a_space_may_precede_the_function_name() {
        let ctx = SemActContext::default();
        // THE REFUSAL, unchanged: the extension's own `^ *(fail|print)` still
        // matches behind any run of spaces, and behind none at all.
        for code in ["fail(\"x\")", " fail(\"x\")", "      fail(s)"] {
            assert!(!test_extension(&act(code), &ctx), "{code:?} must fail");
        }
        // THE VALID NEIGHBOUR: every other scalar is outside `^ *`, so the code
        // is not the extension's `fail` and the action succeeds. Each of these
        // satisfies `char::is_whitespace` and `str::trim_start` ate all of them.
        for lead in [
            '\u{9}', '\u{A}', '\u{B}', '\u{C}', '\u{D}', '\u{A0}', '\u{2028}', '\u{3000}',
        ] {
            assert!(lead.is_whitespace(), "{lead:?}");
            let code = format!("{lead}fail(\"x\")");
            assert!(
                test_extension(&act(&code), &ctx),
                "{code:?} is not the extension's fail"
            );
        }
        // And the rest of the extension's surface is untouched.
        assert!(test_extension(&act("print(\"x\")"), &ctx));
        assert!(test_extension(&act(" print(o)"), &ctx));
        assert!(test_extension(
            &SemAct {
                name: TEST_EXTENSION.to_owned(),
                code: None,
            },
            &ctx
        ));
    }

    #[test]
    fn the_registry_dispatches_the_same_reading() {
        let registry = SemActRegistry::with_test();
        let ctx = SemActContext::default();
        assert_eq!(registry.dispatch(&act("  fail(s)"), &ctx), false);
        assert_eq!(registry.dispatch(&act("\u{A0}fail(s)"), &ctx), true);
    }

    /// Keeps the doc's citation from drifting away from the constant it names.
    #[test]
    fn the_leading_run_is_one_character() {
        assert_eq!(TEST_LEADING_RUN, ' ');
    }
}
