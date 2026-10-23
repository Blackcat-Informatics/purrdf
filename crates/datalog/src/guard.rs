// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Guard literals: native term filters and caller-defined computation.
//!
//! An atom is decided by the relation store; a guard is decided by a
//! [`GuardEvaluator`] the caller supplies at evaluation time, except for explicit native
//! operations such as [`Guard::not_equal`]. That is the seam a rule
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
//! fresh). The evaluator answers with zero or more OUTPUT ROWS, one original
//! identity or owned lexical value per output variable:
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
use crate::dictionary::{DictionaryTerm, DictionaryValue, TermDictionary};
use crate::id::TermId;
use crate::store::{Arity, RelationStore, StoreShape, TermInterner};

/// What a [`Guard`] reads besides its declared inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GuardReads {
    /// Nothing: the guard is a pure function of its input bindings.
    Bindings,
    /// The evaluation model. See the [module docs](self) for what that obliges.
    Model,
}

/// The implementation of a guard, independent of its caller-supplied name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GuardOperation {
    /// Evaluate through the caller's [`GuardEvaluator`].
    Callback,
    /// Keep a row when its two bound term identities differ exactly.
    NotEqual,
}

impl purrdf_hash::error::StructuredError for GuardOperation {
    purrdf_hash::error_visit! {
        Callback => "datalog-guard-callback" {},
        NotEqual => "datalog-guard-not-equal" {},
    }
}

impl GuardOperation {
    /// Decide a native filter; callback guards have no built-in interpretation.
    pub(crate) fn filter<T: PartialEq>(self, inputs: &[T]) -> Option<bool> {
        match self {
            Self::Callback => None,
            Self::NotEqual => {
                assert_eq!(inputs.len(), 2, "native inequality has two declared inputs");
                Some(inputs[0] != inputs[1])
            }
        }
    }
}

/// One guard literal: a native filter or caller code over named inputs and outputs.
///
/// The `name` is the guard's CONTENT IDENTITY — typically the expression text it
/// evaluates — and it is part of the clause digest, so two programs whose guards
/// compute different things never share a cached plan or a contract hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Guard {
    /// Native operation or explicit caller callback; names never select behavior.
    operation: GuardOperation,
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
            operation: GuardOperation::Callback,
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

    /// A native `!=` filter over two bound variables, with no callback required.
    /// Equality is exact term identity, without numeric coercion or normalization.
    pub fn not_equal(left: impl Into<String>, right: impl Into<String>) -> Self {
        Self {
            operation: GuardOperation::NotEqual,
            ..Self::filter("!=", vec![left.into(), right.into()])
        }
    }

    /// Whether this guard is a native operation or a caller callback.
    pub fn operation(&self) -> GuardOperation {
        self.operation
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
pub struct Negation<const N: usize = 4> {
    /// The group's atoms, in authored order; never negated.
    atoms: Vec<ClauseAtom<N>>,
    /// The group's guards, in authored order.
    guards: Vec<Guard>,
}

impl<const N: usize> Negation<N> {
    /// A negated conjunction of `atoms` and `guards`.
    ///
    /// # Panics
    ///
    /// Panics if the group is empty (the empty conjunction is `true`, so its negation
    /// would block every solution), if an atom is itself negated, or if a guard reads
    /// the evaluation model — the group's truth must be a function of the facts it
    /// matches, or the negation would be decided by code no stratification can see.
    pub fn new(atoms: Vec<ClauseAtom<N>>, guards: Vec<Guard>) -> Self {
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
    pub fn atoms(&self) -> &[ClauseAtom<N>] {
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

impl purrdf_hash::error::StructuredError for GuardSite {
    purrdf_hash::error_visit! {
        Body(index) => "guard-site-body" { "index" => Unsigned(*index as u128) },
        Negation { negation, guard } => "guard-site-negation" { "negation" => Unsigned(*negation as u128), "guard" => Unsigned(*guard as u128) },
    }
}

/// One guard evaluation request.
#[derive(Debug)]
pub struct GuardCall<'a, D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The rule's index in authored program order.
    pub rule: usize,
    /// Which of the rule's guards this is.
    pub site: GuardSite,
    /// The guard itself.
    pub guard: &'a Guard,
    /// Original identities or fresh spellings bound to [`Guard::inputs`], in order.
    pub inputs: &'a [DictionaryTerm<'a, D::Id>],
    /// The evaluation model at the point the rule runs.
    pub model: &'a RelationStore<D, N>,
}

impl<D: TermDictionary, const N: usize> Copy for GuardCall<'_, D, N> where Arity<N>: StoreShape<N> {}

impl<D: TermDictionary, const N: usize> Clone for GuardCall<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn clone(&self) -> Self {
        *self
    }
}

/// Output rows preserve original identities and genuinely computed spellings.
pub type GuardRows<I = TermId> = Vec<Vec<DictionaryValue<I>>>;

impl<D: TermDictionary, const N: usize> GuardCall<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Evaluate a guard whose meaning requires lexical values, retaining every
    /// original dictionary pin through the callback. Native guards use `inputs`
    /// and return original identities directly.
    ///
    /// # Errors
    ///
    /// Preserves original source failures and the callback's typed refusal.
    pub fn evaluate_lexical(
        &self,
        evaluate: impl FnOnce(&[&str]) -> Result<GuardRows<D::Id>, CallbackError<D::Error>>,
    ) -> Result<GuardRows<D::Id>, CallbackError<D::Error>> {
        let dictionary = self.model.interner();
        dictionary.check_read().map_err(CallbackError::Source)?;
        let pinned = self
            .inputs
            .iter()
            .map(|input| input.lexical(dictionary))
            .collect::<Result<Vec<_>, D::Error>>()
            .map_err(CallbackError::Source)?;
        dictionary.check_read().map_err(CallbackError::Source)?;
        let inputs = pinned.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        evaluate(&inputs)
    }
}

/// The caller's implementation of its guards.
///
/// `evaluate` answers the output rows of one call — see the [module docs](self) for
/// what an empty answer, an empty row and several rows mean — or `Err` with a message,
/// which aborts the evaluation as [`EvalError::Guard`](crate::seminaive::EvalError::Guard):
/// a guard that could not decide is never read as a guard that said no.
pub trait GuardEvaluator<D: TermDictionary = TermInterner, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// Evaluate one guard call.
    ///
    /// # Errors
    ///
    /// A typed source or storage refusal, or a message naming why evaluation failed.
    fn evaluate(
        &self,
        call: &GuardCall<'_, D, N>,
    ) -> Result<GuardRows<D::Id>, CallbackError<D::Error>>;
}

/// A refusal from caller-supplied guard or layer-boundary evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackError<E = core::convert::Infallible> {
    /// The original failure of the dictionary read by a model-dependent guard.
    Source(E),
    /// Logical capacity or physical allocation refused the operation.
    Storage(purrdf_lex::allocation::StorageError),
    /// The guard's own evaluation diagnostic.
    Evaluation(String),
    /// Original native callback configuration or output-shape refusal.
    Native(CallbackDiagnostic),
}

/// The diagnostic contextualized by a guard or layer boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackDiagnostic {
    /// An actual caller-authored callback message.
    Message(String),
    /// A rule requires an evaluator that was not supplied.
    NoEvaluator {
        /// Original rule index.
        rule: usize,
        /// Original guard name.
        guard: String,
    },
    /// A callback returned a row incompatible with its output declaration.
    RowWidth {
        /// Actual returned width.
        actual: usize,
        /// Declared output width.
        expected: usize,
    },
}

impl purrdf_hash::error::StructuredError for CallbackDiagnostic {
    purrdf_hash::error_visit! {
        Message(message) => "datalog-callback-message" { "message" => Message(message) },
        NoEvaluator { rule, guard } => "datalog-callback-no-evaluator" { "rule" => Unsigned(*rule as u128), "guard" => Text(guard) },
        RowWidth { actual, expected } => "datalog-callback-row-width" { "actual" => Unsigned(*actual as u128), "expected" => Unsigned(*expected as u128) },
    }
}

impl fmt::Display for CallbackDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) => f.write_str(message),
            Self::NoEvaluator { rule, guard } => write!(
                f,
                "rule {rule} carries guard {guard:?}, but the evaluation was given no guard evaluator"
            ),
            Self::RowWidth { actual, expected } => write!(
                f,
                "the evaluator answered a row of {actual} values for {expected} outputs"
            ),
        }
    }
}

impl std::error::Error for CallbackDiagnostic {}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError
    for CallbackError<E>
{
    purrdf_hash::error_visit! {
        Source(cause) => "datalog-callback-source" { "cause" => Cause(cause) },
        Storage(cause) => "datalog-callback-storage" { "cause" => Cause(cause) },
        Evaluation(message) => "datalog-callback-evaluation" { "message" => Message(message) },
        Native(cause) => "datalog-callback-native" { "cause" => Cause(cause) },
    }
}

impl<E> CallbackError<E> {
    /// Preserve source and storage identity; contextualize only the textual diagnostic.
    pub(crate) fn into_eval_error(
        self,
        describe: impl FnOnce(CallbackDiagnostic) -> crate::seminaive::EvalError<E>,
    ) -> crate::seminaive::EvalError<E> {
        match self {
            Self::Source(error) => crate::seminaive::EvalError::Source(error),
            Self::Storage(error) => crate::seminaive::EvalError::Storage(error),
            Self::Evaluation(message) => describe(CallbackDiagnostic::Message(message)),
            Self::Native(cause) => describe(cause),
        }
    }
}

impl<E: fmt::Display> fmt::Display for CallbackError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => error.fmt(f),
            Self::Storage(error) => error.fmt(f),
            Self::Evaluation(message) => f.write_str(message),
            Self::Native(cause) => cause.fmt(f),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CallbackError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::Evaluation(_) => None,
            Self::Native(cause) => Some(cause),
        }
    }
}

purrdf_lex::variant_from!(impl<E> CallbackError<E> {
    Storage(purrdf_lex::allocation::StorageError),
    Evaluation(String),
});
purrdf_lex::variant_from!(impl<E> CallbackError<E> { Evaluation(&str) as ToString::to_string });

/// The evaluator for a program that has no guards: any call is an error, because a
/// guarded program evaluated without its guards' meaning has no answer.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoGuards;

impl<D: TermDictionary, const N: usize> GuardEvaluator<D, N> for NoGuards
where
    Arity<N>: StoreShape<N>,
{
    fn evaluate(
        &self,
        call: &GuardCall<'_, D, N>,
    ) -> Result<GuardRows<D::Id>, CallbackError<D::Error>> {
        Err(CallbackError::Native(CallbackDiagnostic::NoEvaluator {
            rule: call.rule,
            guard: call.guard.name().to_owned(),
        }))
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
        let _: Negation = Negation::new(Vec::new(), Vec::new());
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
        assert!(refusal.to_string().contains("rule 3"), "{refusal}");
        assert_eq!(GuardSite::Body(2).to_string(), "body guard 2");
    }
}
