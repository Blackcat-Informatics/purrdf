// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Guard literals: the body literals whose meaning is CALLER CODE.
//!
//! An atom is decided by the relation store; a guard is decided by a
//! [`GuardEvaluator`] the caller supplies at evaluation time. That is the seam a rule
//! language with an expression sublanguage needs and a pure Datalog does not have:
//! SPARQL 1.2 RL's `FILTER` and `SET` elements, SHACL's node expressions and a SHACL
//! SPARQL rule's `CONSTRUCT` query all compute values the clause IR cannot spell,
//! and this crate has no business re-implementing SPARQL's function library to spell
//! them. The evaluator stays the one fixpoint; the caller supplies what an expression
//! means.
//!
//! # The shape of a guard
//!
//! A [`Guard`] names its INPUT variables (read from the solution, and required to be
//! bound when it runs) and its OUTPUT variables (bound by it, and required to be
//! fresh). The evaluator answers with zero or more OUTPUT ROWS, one lexical surface
//! per output variable:
//!
//! * a **filter** has no outputs, and answers one empty row to keep the solution or
//!   no row to drop it;
//! * an **assignment** has one output, and answers one row binding it — or no row,
//!   which drops the solution (SPARQL 1.2 RL: "If evaluating the expression in an
//!   assignment causes an error, then the current solution mapping is rejected by
//!   the assignment");
//! * a **producer** may answer many rows, each extending the solution once — the
//!   shape a SHACL rule whose whole body is opaque caller code has.
//!
//! A surface the store has never interned is a FRESH term. It is carried through the
//! rest of the rule as a computed binding and interned only if a winning head commits
//! it; a negated atom that mentions it can never be satisfied, because no stored fact
//! carries a term the store has never seen.
//!
//! # What a guard may read
//!
//! [`GuardReads::Bindings`] declares a guard a pure function of its inputs — a
//! filter or an assignment. [`GuardReads::Model`] declares that it reads the
//! evaluation model itself (a SPARQL `EXISTS`, a CONSTRUCT query, a SHACL target
//! selection), which has two consequences the evaluator enforces rather than trusts:
//!
//! * a rule carrying one cannot be evaluated semi-naively — its answer can change
//!   when ANY fact changes, not only when a positive body atom's relation does — so
//!   it is re-evaluated against the whole model every time it runs;
//! * it cannot be stratified, because what it reads, and with which polarity, is not
//!   something the clause states. The stratified fixpoint
//!   ([`compile`](crate::seminaive::compile)) refuses it by name; the ordered
//!   schedule ([`crate::schedule`]) is where it runs, under the schedule the caller's
//!   rule language defines.
//!
//! # Determinism
//!
//! A guard is called on the evaluating thread, in program order, once per solution
//! reaching it — never from a rayon worker — so a caller's thread-scoped evaluation
//! context (a function table, a prefix map) reaches it, and a caller that is itself
//! deterministic keeps the evaluation byte-deterministic. A stratum or group holding
//! a guarded rule is therefore evaluated sequentially; a guard-free one keeps the
//! rule-parallel round.

use std::fmt;

use crate::clause::ClauseAtom;
use crate::store::RelationStore;

/// What a [`Guard`] reads besides its declared inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GuardReads {
    /// Nothing: the guard is a pure function of its input bindings.
    Bindings,
    /// The evaluation model. See the [module docs](self) for what that obliges.
    Model,
}

/// One guard literal: caller code over named input variables, binding named outputs.
///
/// The `name` is the guard's CONTENT IDENTITY — typically the expression text it
/// evaluates — and it is part of the clause digest, so two programs whose guards
/// compute different things never share a cached plan or a contract hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Guard {
    /// The guard's content identity, as the caller names it.
    name: String,
    /// The variables read, in the order the evaluator receives them.
    inputs: Vec<String>,
    /// The variables bound, in the order an output row lists them.
    outputs: Vec<String>,
    /// What the guard reads besides its inputs.
    reads: GuardReads,
}

impl Guard {
    /// A guard with every field explicit.
    ///
    /// # Panics
    ///
    /// Panics if an output variable is listed twice, or is also an input: an output
    /// is a FRESH binding, so reading and writing one name in the same guard is a
    /// construction bug rather than a data state.
    pub fn new(
        name: impl Into<String>,
        inputs: Vec<String>,
        outputs: Vec<String>,
        reads: GuardReads,
    ) -> Self {
        for (index, output) in outputs.iter().enumerate() {
            assert!(
                !outputs[..index].contains(output),
                "guard output {output} is bound twice by one guard"
            );
            assert!(
                !inputs.contains(output),
                "guard output {output} is also one of the guard's inputs"
            );
        }
        Self {
            name: name.into(),
            inputs,
            outputs,
            reads,
        }
    }

    /// A pure filter over `inputs`: it keeps or drops a solution and binds nothing.
    pub fn filter(name: impl Into<String>, inputs: Vec<String>) -> Self {
        Self::new(name, inputs, Vec::new(), GuardReads::Bindings)
    }

    /// A pure assignment of one fresh `output` from `inputs`.
    pub fn assign(name: impl Into<String>, inputs: Vec<String>, output: impl Into<String>) -> Self {
        Self::new(name, inputs, vec![output.into()], GuardReads::Bindings)
    }

    /// The guard's content identity.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The input variables, in the order the evaluator receives their values.
    pub fn inputs(&self) -> &[String] {
        &self.inputs
    }

    /// The output variables, in the order an output row lists their values.
    pub fn outputs(&self) -> &[String] {
        &self.outputs
    }

    /// What the guard reads besides its inputs.
    pub fn reads(&self) -> GuardReads {
        self.reads
    }

    /// Whether the guard reads the evaluation model.
    pub fn reads_model(&self) -> bool {
        self.reads == GuardReads::Model
    }
}

/// A negated CONJUNCTION: `not ∃ȳ. (A₁ ∧ … ∧ Aₙ ∧ G₁ ∧ … ∧ Gₘ)`.
///
/// A negated [`ClauseAtom`] denies one fact; this denies a pattern — SPARQL 1.2 RL's
/// negation element, "a sequence of triple pattern elements and filter elements".
/// The variables the enclosing rule binds are fixed by the solution; every other
/// variable of the group is existentially quantified inside it, so the group holds
/// exactly when SOME extension matches every atom and passes every guard. The rule
/// fires when it does not.
///
/// The atoms are matched first, in authored order, then the guards in authored order
/// — a guard may read a variable an atom of the group binds, and may bind variables a
/// later guard reads. Every atom here is positive: the group is negated as a whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Negation {
    /// The group's atoms, in authored order; never negated.
    atoms: Vec<ClauseAtom>,
    /// The group's guards, in authored order.
    guards: Vec<Guard>,
}

impl Negation {
    /// A negated conjunction of `atoms` and `guards`.
    ///
    /// # Panics
    ///
    /// Panics if the group is empty (the empty conjunction is `true`, so its negation
    /// would block every solution), if an atom is itself negated, or if a guard reads
    /// the evaluation model — the group's truth must be a function of the facts it
    /// matches, or the negation would be decided by code no stratification can see.
    pub fn new(atoms: Vec<ClauseAtom>, guards: Vec<Guard>) -> Self {
        assert!(
            !atoms.is_empty() || !guards.is_empty(),
            "a negated conjunction may not be empty"
        );
        assert!(
            atoms.iter().all(|atom| !atom.is_negated()),
            "a negated conjunction's atoms are positive; the group is negated as a whole"
        );
        assert!(
            guards.iter().all(|guard| !guard.reads_model()),
            "a negated conjunction's guards are pure functions of their bindings"
        );
        Self { atoms, guards }
    }

    /// The group's atoms, in authored order.
    pub fn atoms(&self) -> &[ClauseAtom] {
        &self.atoms
    }

    /// The group's guards, in authored order.
    pub fn guards(&self) -> &[Guard] {
        &self.guards
    }
}

/// Where in a rule the guard being evaluated sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GuardSite {
    /// The rule's `index`-th body guard.
    Body(usize),
    /// Guard `guard` of the rule's negated conjunction `negation`.
    Negation {
        /// The negated conjunction's index in the rule.
        negation: usize,
        /// The guard's index inside it.
        guard: usize,
    },
}

/// One guard evaluation request.
#[derive(Debug, Clone, Copy)]
pub struct GuardCall<'a> {
    /// The rule's index in authored program order.
    pub rule: usize,
    /// Which of the rule's guards this is.
    pub site: GuardSite,
    /// The guard itself.
    pub guard: &'a Guard,
    /// The lexical surfaces bound to [`Guard::inputs`], in the same order.
    pub inputs: &'a [&'a str],
    /// The evaluation model at the point the rule runs.
    pub model: &'a RelationStore,
}

/// The caller's implementation of its guards.
///
/// `evaluate` answers the output rows of one call — see the [module docs](self) for
/// what an empty answer, an empty row and several rows mean — or `Err` with a message,
/// which aborts the evaluation as [`EvalError::Guard`](crate::seminaive::EvalError::Guard):
/// a guard that could not decide is never read as a guard that said no.
pub trait GuardEvaluator {
    /// Evaluate one guard call.
    ///
    /// # Errors
    ///
    /// A message naming why the guard could not be evaluated.
    fn evaluate(&self, call: &GuardCall<'_>) -> Result<Vec<Vec<String>>, String>;
}

/// The evaluator for a program that has no guards: any call is an error, because a
/// guarded program evaluated without its guards' meaning has no answer.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoGuards;

impl GuardEvaluator for NoGuards {
    fn evaluate(&self, call: &GuardCall<'_>) -> Result<Vec<Vec<String>>, String> {
        Err(format!(
            "rule {} carries guard {:?}, but the evaluation was given no guard evaluator",
            call.rule,
            call.guard.name()
        ))
    }
}

impl fmt::Display for GuardSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Body(index) => write!(f, "body guard {index}"),
            Self::Negation { negation, guard } => {
                write!(f, "guard {guard} of negated conjunction {negation}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clause::ClauseTerm;

    #[test]
    fn a_filter_and_an_assignment_have_the_documented_shapes() {
        let filter = Guard::filter("?x > 1", vec!["?x".to_owned()]);
        assert_eq!(filter.outputs(), [] as [String; 0]);
        assert_eq!(filter.reads(), GuardReads::Bindings);
        let assign = Guard::assign("?x + 1", vec!["?x".to_owned()], "?y");
        assert_eq!(assign.outputs(), ["?y"]);
        assert!(!assign.reads_model());
        let producer = Guard::new("q", Vec::new(), vec!["?s".to_owned()], GuardReads::Model);
        assert!(producer.reads_model());
    }

    #[test]
    #[should_panic(expected = "also one of the guard's inputs")]
    fn an_output_may_not_also_be_an_input() {
        let _ = Guard::assign("f", vec!["?x".to_owned()], "?x");
    }

    #[test]
    #[should_panic(expected = "bound twice")]
    fn an_output_may_not_be_bound_twice() {
        let _ = Guard::new(
            "f",
            Vec::new(),
            vec!["?x".to_owned(), "?x".to_owned()],
            GuardReads::Bindings,
        );
    }

    #[test]
    #[should_panic(expected = "may not be empty")]
    fn an_empty_negated_conjunction_is_refused() {
        let _ = Negation::new(Vec::new(), Vec::new());
    }

    #[test]
    #[should_panic(expected = "pure functions")]
    fn a_negated_conjunction_may_not_read_the_model() {
        let _ = Negation::new(
            vec![ClauseAtom::positive(
                ClauseTerm::var("?x"),
                "https://example.org/p",
                ClauseTerm::var("?y"),
            )],
            vec![Guard::new("q", Vec::new(), Vec::new(), GuardReads::Model)],
        );
    }

    #[test]
    fn the_no_guards_evaluator_refuses_every_call() {
        let guard = Guard::filter("f", Vec::new());
        let store = RelationStore::new();
        let call = GuardCall {
            rule: 3,
            site: GuardSite::Body(0),
            guard: &guard,
            inputs: &[],
            model: &store,
        };
        let refusal = NoGuards.evaluate(&call).expect_err("no evaluator");
        assert!(refusal.contains("rule 3"), "{refusal}");
        assert_eq!(GuardSite::Body(2).to_string(), "body guard 2");
    }
}
