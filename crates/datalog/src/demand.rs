// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete answers to one positive triple or quad goal without materializing unrelated rules.
//!
//! A magic-set transformation carries bound subject/predicate/object/graph coordinates through
//! authored body order. Subsuming adornments share one demand relation; the
//! original atoms and final goal retain constants and repeated-variable equality.
//! The transformed clauses run through the ordinary compiler and semi-naive engine.
//!
//! Preparation uses resident allocation, just like [`crate::seminaive::compile`].
//! Evaluation uses the caller's limits and stop signal. Work and budget reports
//! count the transformed program, including its internal demand facts, and all
//! original input facts.
//! The retained partial closure is private: it is complete only for the requested goal.

mod maintained;
pub(crate) mod projection;
mod transform;
pub use maintained::{DemandDelta, DemandDeltaResult, MaintainedDemand};

use std::collections::{BTreeMap, BTreeSet};

use crate::binding_pattern::BindingPattern;
use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadForm, ReadSource};
use crate::cursor::LendingIterator;
use crate::dictionary::TermDictionary;
use crate::guard::NoGuards;
use crate::seminaive::{
    BudgetReport, EvalError, EvalOptions, Evaluation, Executable, WorkReport, evaluate_guarded,
};
use crate::stop::StopSignal;
use crate::store::{Arity, Bound, RelationStore, StoreShape};

/// A fragment refusal before any transformed execution or source mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// Only a single atomic head without existential variables is admitted.
    Head(HeadForm),
    /// Guards, including model reads, need a different demand contract.
    Guard,
    /// The requested goal must be positive.
    Negation,
    /// Immutable base reads belong to the scheduled evaluator.
    BaseRead,
    /// A constant predicate must be an IRI; variable predicates are admitted.
    Predicate,
    /// A head variable has no positive relational binder.
    UnboundVariable(String),
}

impl purrdf_hash::error::StructuredError for Unsupported {
    purrdf_hash::error_visit! {
        Head(form) => "datalog-demand-unsupported-head" { "form" => Record(form) },
        Guard => "datalog-demand-unsupported-guard" {},
        Negation => "datalog-demand-unsupported-negation" {},
        BaseRead => "datalog-demand-unsupported-base-read" {},
        Predicate => "datalog-demand-unsupported-predicate" {},
        UnboundVariable(variable) => "datalog-demand-unbound-variable" { "variable" => Text(variable) },
    }
}

/// Original compilation, dictionary or evaluation failure, or an unsupported fragment.
#[derive(Debug)]
pub enum DemandError<E> {
    /// The whole program is checked, including rules unreachable from the goal.
    Unsupported {
        /// Authored rule ordinal; `None` denotes the query goal.
        rule: Option<usize>,
        /// The first unsupported construct.
        reason: Unsupported,
    },
    /// The ordinary compiler refused the authored or transformed clauses.
    Compile(EvalError),
    /// The original source dictionary refused a preparation lookup.
    Source(E),
    /// The ordinary governed evaluator refused completion.
    Evaluation(EvalError<E>),
    /// Every representable namespace nonce collided with authored/source terms.
    NamespaceExhausted,
    /// A supplied source now uses a name reserved by this prepared artifact.
    /// Reprepare against that source to choose a different private namespace.
    PreparedNamespaceCollision {
        /// Exact conflicting IRI spelling, including angle brackets.
        term: String,
    },
}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError
    for DemandError<E>
{
    purrdf_hash::error_visit! {
        Unsupported { rule, reason } => "datalog-demand-unsupported" {
            "rule" => Value(rule.map_or(purrdf_hash::error::ErrorValue::Absent, |ordinal| purrdf_hash::error::ErrorValue::Unsigned(ordinal as u128))),
            "reason" => Record(reason),
        },
        Compile(cause) => "datalog-demand-compile" { "cause" => Cause(cause) },
        Source(cause) => "datalog-demand-source" { "cause" => Cause(cause) },
        Evaluation(cause) => "datalog-demand-evaluation" { "cause" => Cause(cause) },
        NamespaceExhausted => "datalog-demand-namespace-exhausted" {},
        PreparedNamespaceCollision { term } => "datalog-demand-prepared-namespace-collision" {
            "term" => Text(term),
        },
    }
}

impl<E: std::fmt::Display> std::fmt::Display for DemandError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported { rule, reason } => write!(
                f,
                "unsupported demand query fragment at {rule:?}: {reason:?}"
            ),
            Self::Compile(error) => error.fmt(f),
            Self::Source(error) => error.fmt(f),
            Self::Evaluation(error) => error.fmt(f),
            Self::NamespaceExhausted => f.write_str("demand predicate namespace exhausted"),
            Self::PreparedNamespaceCollision { term } => write!(
                f,
                "prepared demand namespace conflicts with source term {term}; reprepare against this source"
            ),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for DemandError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Compile(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::Evaluation(error) => Some(error),
            Self::Unsupported { .. }
            | Self::NamespaceExhausted
            | Self::PreparedNamespaceCollision { .. } => None,
        }
    }
}

/// The source-bound owner of complete answers to one goal.
///
/// Original term IDs and dictionary ownership are preserved. Internal facts are
/// intentionally inaccessible as a model of the authored program.
#[derive(Debug)]
pub struct DemandAnswer<D: TermDictionary, const N: usize = 3>
where
    Arity<N>: StoreShape<N>,
{
    evaluation: Evaluation<D, N>,
    goal: ClauseAtom<N>,
    preparation: Preparation,
}

/// Original input and private projection counts for this execution.
///
/// Projection runs under the evaluator's governors. Private mapping and shadow
/// rows count as transformed input, while new terms count against the original
/// input dictionary's generated-term allowance. They are not rule-derived facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preparation {
    /// Original caller-owned rows before private projection.
    pub original_facts: usize,
    /// Original dictionary's distinct terms before private projection.
    pub original_terms: u64,
    /// Additional private input rows in mapping and isolated shadow relations.
    pub shadow_facts: usize,
    /// Additional dictionary terms minted during private projection.
    pub shadow_terms: u64,
}

/// A reusable compiled demand transformation for one authored program and goal.
///
/// Owns no source dictionary or result. Each execution consumes its original
/// store, preserving that dictionary's IDs, and uses the same native evaluator.
/// A new source may have different rows and IDs, but must not use this artifact's
/// generated names. Repreparing against an incompatible source chooses a fresh
/// namespace without reserving any vocabulary against the caller.
#[derive(Debug)]
pub struct PreparedDemand<const N: usize = 3>
where
    Arity<N>: StoreShape<N>,
{
    executable: Executable<N>,
    goal: ClauseAtom<N>,
    names: Vec<String>,
}

impl<const N: usize> PreparedDemand<N>
where
    Arity<N>: StoreShape<N>,
{
    /// Check a source without consuming its original dictionary or rows.
    ///
    /// On a namespace conflict, pass this same source to fresh preparation
    /// before moving it into execution. This check grants no snapshot token:
    /// [`Self::evaluate`] checks the dictionary again at its execution boundary.
    /// # Errors
    /// Returns an exact namespace conflict or the original source failure.
    pub fn check_compatibility<D: TermDictionary>(
        &self,
        dictionary: &D,
    ) -> Result<(), DemandError<D::Error>> {
        dictionary.check_read().map_err(DemandError::Source)?;
        for name in &self.names {
            let term = format!("<{name}>");
            if dictionary
                .lookup_term(&term)
                .map_err(DemandError::Source)?
                .is_some()
            {
                dictionary.check_read().map_err(DemandError::Source)?;
                return Err(DemandError::PreparedNamespaceCollision { term });
            }
        }
        dictionary.check_read().map_err(DemandError::Source)
    }

    /// Execute the prepared transformation over a compatible original source.
    ///
    /// Namespace compatibility is checked before private projection or evaluation.
    /// Work, cancellation, budgets and answer ownership retain the one-shot
    /// API's contract; preparation counts describe this execution's source.
    /// # Errors
    /// Returns an actionable namespace conflict, original source failure or
    /// original evaluator refusal. No partial answer is returned on exhaustion.
    pub fn evaluate<D: TermDictionary>(
        &self,
        edb: RelationStore<D, N>,
        options: &EvalOptions,
        stop: Option<&dyn StopSignal>,
    ) -> Result<DemandAnswer<D, N>, DemandError<D::Error>> {
        self.check_compatibility(edb.interner())?;
        let original_facts = edb.row_count();
        let original_terms = edb.interner().term_count();
        let evaluation = evaluate_guarded(&self.executable, edb, &NoGuards, options, stop)
            .map_err(DemandError::Evaluation)?;
        let preparation = Preparation {
            original_facts,
            original_terms,
            shadow_facts: evaluation
                .projection
                .as_ref()
                .map_or(0, |projection| projection.seeded_rows()),
            shadow_terms: evaluation
                .projection
                .as_ref()
                .map_or(0, |projection| projection.seeded_terms()),
        };
        Ok(DemandAnswer {
            evaluation,
            goal: self.goal.clone(),
            preparation,
        })
    }
}

/// Compile a reusable demand transformation without modifying the source.
///
/// Checks the same complete triple/quad fragment as [`evaluate_demanded`],
/// including variable predicates and stratified negation. The dictionary is used
/// only to choose collision-free generated names; it is neither retained nor
/// copied. Later executions check their own dictionary before mutation.
/// # Errors
/// Returns the original fragment, compiler or dictionary failure.
pub fn prepare_demanded<D: TermDictionary, const N: usize>(
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    dictionary: &D,
) -> Result<PreparedDemand<N>, DemandError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    validate(rules, goal)?;
    transform::prepare(rules, goal, dictionary)
}

impl<D: TermDictionary, const N: usize> DemandAnswer<D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Original input and governed private projection counts.
    pub const fn preparation(&self) -> Preparation {
        self.preparation
    }

    /// The original dictionary, extended by transformed preparation and execution.
    pub fn dictionary(&self) -> &D {
        self.evaluation.facts().interner()
    }

    /// Work performed by the transformed program, including demand propagation.
    pub fn work(&self) -> WorkReport {
        self.evaluation.work()
    }

    /// Original evaluation budget report; internal and input facts are counted.
    pub fn budget(&self) -> BudgetReport {
        self.evaluation.budget()
    }

    /// Complete matching rows in deterministic native partition and arrangement order.
    ///
    /// The returned vector uses resident allocation. IDs borrow their meaning
    /// from this owner's dictionary. Constants and repeated variables are checked
    /// against the original goal even when a broader demand was shared.
    /// # Errors
    /// Preserves dictionary lookup and read failures, including an empty answer.
    pub fn answers(&self) -> Result<Vec<[D::Id; N]>, D::Error> {
        answers(
            self.evaluation.facts(),
            &self.goal,
            self.evaluation.projection.as_deref(),
        )
    }
}

struct GoalMatch<'a, I, const N: usize> {
    terms: [&'a ClauseTerm; N],
    constants: [Option<I>; N],
}

impl<'a, I: crate::dictionary::TermIdentity, const N: usize> GoalMatch<'a, I, N> {
    fn new<D: TermDictionary<Id = I>>(
        dictionary: &D,
        goal: &'a ClauseAtom<N>,
    ) -> Result<Option<Self>, D::Error> {
        dictionary.check_read()?;
        let terms = goal.terms();
        let mut constants = [None; N];
        for (at, term) in terms.iter().enumerate() {
            if let Some(surface) = term.surface() {
                let Some(id) = dictionary.lookup_term(&surface)? else {
                    dictionary.check_read()?;
                    return Ok(None);
                };
                constants[at] = Some(id);
            }
        }
        dictionary.check_read()?;
        Ok(Some(Self { terms, constants }))
    }

    fn matches(&self, row: [I; N]) -> bool {
        self.terms.iter().enumerate().all(|(at, term)| {
            self.constants[at].is_none_or(|id| row[at] == id)
                && (!term.is_var()
                    || (0..at).all(|before| self.terms[before] != *term || row[before] == row[at]))
        })
    }
}

fn answers<D: TermDictionary, const N: usize>(
    facts: &RelationStore<D, N>,
    goal: &ClauseAtom<N>,
    projection: Option<&projection::Runtime<D::Id, N>>,
) -> Result<Vec<[D::Id; N]>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let dictionary = facts.interner();
    let Some(matcher) = GoalMatch::new(dictionary, goal)? else {
        return Ok(Vec::new());
    };
    let mut answers = Vec::new();
    for partition in facts.partitions_for(matcher.constants) {
        let bound = match (matcher.constants[0], matcher.constants[2]) {
            (Some(subject), Some(object)) => Bound::Both(subject, object),
            (Some(subject), None) => Bound::Subject(subject),
            (None, Some(object)) => Bound::Object(object),
            (None, None) => Bound::Any,
        };
        let mut rows = partition.select(bound);
        while let Some((subject, object, _)) = rows.next() {
            let row = partition.row(subject, object);
            if matcher.matches(row)
                && (!goal.predicate().is_var()
                    || projection.is_some_and(|projection| projection.original(row[1])))
            {
                answers.push(row);
            }
        }
    }
    dictionary.check_read()?;
    Ok(answers)
}

/// Answer one positive triple or quad goal with demand propagation.
///
/// The complete authored program and goal are validated before dictionary mutation
/// or execution. Atomic heads must be range-restricted; guards, base/model reads
/// and every other head form are refused. Stratified negation, including guard-free
/// conjunctions, discovers demand through an isolated positive shadow of the input.
/// Predicates may be fixed IRIs or variables. Private mapping relations preserve
/// original predicate identities and keep generated facts out of variable matches.
/// [`DemandAnswer::preparation`] reports additional private rows and terms. The
/// ordinary evaluator governs their construction, execution and cancellation.
/// # Errors
/// Returns an unsupported fragment, namespace/dictionary failure, or the original
/// compiler/evaluator refusal. A budget or cancellation refusal yields no answer owner.
pub fn evaluate_demanded<D: TermDictionary, const N: usize>(
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    edb: RelationStore<D, N>,
    options: &EvalOptions,
    stop: Option<&dyn StopSignal>,
) -> Result<DemandAnswer<D, N>, DemandError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    prepare_demanded(rules, goal, edb.interner())?.evaluate(edb, options, stop)
}

fn validate<E, const N: usize>(
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
) -> Result<(), DemandError<E>> {
    for (index, rule) in rules.iter().enumerate() {
        let refuse = |reason| DemandError::Unsupported {
            rule: Some(index),
            reason,
        };
        if rule.head_form() != HeadForm::Atomic {
            return Err(refuse(Unsupported::Head(rule.head_form())));
        }
        if !rule.guards().is_empty() {
            return Err(refuse(Unsupported::Guard));
        }
        if rule
            .negations()
            .iter()
            .any(|group| !group.guards().is_empty())
        {
            return Err(refuse(Unsupported::Guard));
        }
        for atom in rule.body().iter().chain(rule.head_atoms()).chain(
            rule.negations()
                .iter()
                .flat_map(crate::guard::Negation::atoms),
        ) {
            validate_atom(atom).map_err(refuse)?;
        }
        let bound = rule.bound_variables();
        // The head-form check above admits only one atomic Datalog head.
        for term in rule.datalog_head().expect("validated atomic head").terms() {
            if let Some(variable) = term.variable()
                && !bound.contains(variable)
            {
                return Err(refuse(Unsupported::UnboundVariable(variable.to_owned())));
            }
        }
    }
    if goal.is_negated() {
        return Err(DemandError::Unsupported {
            rule: None,
            reason: Unsupported::Negation,
        });
    }
    validate_atom(goal).map_err(|reason| DemandError::Unsupported { rule: None, reason })
}

fn validate_atom<const N: usize>(atom: &ClauseAtom<N>) -> Result<(), Unsupported> {
    if atom.read_source() != ReadSource::Evaluation {
        return Err(Unsupported::BaseRead);
    }
    if atom.predicate_iri().is_none() && !atom.predicate().is_var() {
        return Err(Unsupported::Predicate);
    }
    Ok(())
}
