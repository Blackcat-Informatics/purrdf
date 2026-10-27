// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete answers to one positive triple goal without materializing unrelated rules.
//!
//! A magic-set transformation carries bound subject/object coordinates through
//! authored body order. Subsuming adornments share one demand relation; the
//! original atoms and final goal retain constants and repeated-variable equality.
//! The transformed clauses run through the ordinary compiler and semi-naive engine.
//!
//! Preparation uses resident allocation, just like [`compile`]. Evaluation uses
//! the caller's limits and stop signal. Work and budget reports count the transformed
//! program, including its internal demand facts, and all original input facts.
//! The retained partial closure is private: it is complete only for the requested goal.

use std::collections::{BTreeMap, BTreeSet};

use crate::binding_pattern::BindingPattern;
use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadForm, ReadSource};
use crate::cursor::LendingIterator;
use crate::dictionary::TermDictionary;
use crate::guard::NoGuards;
use crate::seminaive::{
    BudgetReport, EvalError, EvalOptions, Evaluation, WorkReport, compile, evaluate_guarded,
};
use crate::stop::StopSignal;
use crate::store::{Bound, RelationStore};
use purrdf_hash::{Domain, frame::frame_le_into};

/// A fragment refusal before any transformed execution or source mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// Only a single atomic head without existential variables is admitted.
    Head(HeadForm),
    /// Guards, including model reads, need a different demand contract.
    Guard,
    /// Negated atoms or conjunctions need a different demand contract.
    Negation,
    /// Immutable base reads belong to the scheduled evaluator.
    BaseRead,
    /// Every predicate must be a fixed IRI.
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
    /// The ordinary compiler refused the transformed clauses.
    Compile(EvalError),
    /// The original source dictionary refused a preparation lookup.
    Source(E),
    /// The ordinary governed evaluator refused completion.
    Evaluation(EvalError<E>),
    /// Every representable namespace nonce collided with authored/source terms.
    NamespaceExhausted,
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
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for DemandError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Compile(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::Evaluation(error) => Some(error),
            Self::Unsupported { .. } | Self::NamespaceExhausted => None,
        }
    }
}

/// The source-bound owner of complete answers to one goal.
///
/// Original term IDs and dictionary ownership are preserved. Internal facts are
/// intentionally inaccessible as a model of the authored program.
#[derive(Debug)]
pub struct DemandAnswer<D: TermDictionary> {
    evaluation: Evaluation<D, 3>,
    goal: ClauseAtom<3>,
}

impl<D: TermDictionary> DemandAnswer<D> {
    /// The original dictionary, extended only by terms of the transformed execution.
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

    /// Complete matching triples in deterministic native arrangement order.
    ///
    /// The returned vector uses resident allocation. IDs borrow their meaning
    /// from this owner's dictionary. Constants and repeated variables are checked
    /// against the original goal even when a broader demand was shared.
    /// # Errors
    /// Preserves dictionary lookup and read failures, including an empty answer.
    pub fn answers(&self) -> Result<Vec<[D::Id; 3]>, D::Error> {
        let dictionary = self.dictionary();
        dictionary.check_read()?;
        let terms = self.goal.terms();
        let mut constants = [None; 3];
        for (at, term) in terms.iter().enumerate() {
            if let Some(surface) = term.surface() {
                let Some(id) = dictionary.lookup_term(&surface)? else {
                    dictionary.check_read()?;
                    return Ok(Vec::new());
                };
                constants[at] = Some(id);
            }
        }
        let repeated = self.goal.subject().is_var() && self.goal.subject() == self.goal.object();
        let mut answers = Vec::new();
        if let Some(partition) = self.evaluation.facts().partition_for(&constants) {
            let bound = match (constants[0], constants[2]) {
                (Some(subject), Some(object)) => Bound::Both(subject, object),
                (Some(subject), None) => Bound::Subject(subject),
                (None, Some(object)) => Bound::Object(object),
                (None, None) => Bound::Any,
            };
            let mut rows = partition.select(bound);
            while let Some((subject, object, _)) = rows.next() {
                if !repeated || subject == object {
                    answers.push(partition.row(subject, object));
                }
            }
        }
        dictionary.check_read()?;
        Ok(answers)
    }
}

/// Answer one pure positive, fixed-predicate, graph-free goal with demand propagation.
///
/// The complete authored program and goal are validated before dictionary mutation
/// or execution. Atomic heads must be range-restricted; guards, base/model reads,
/// negation and every other head form are refused. All preparation is resident;
/// the ordinary evaluator alone enforces execution options and cancellation.
/// # Errors
/// Returns an unsupported fragment, namespace/dictionary failure, or the original
/// compiler/evaluator refusal. A budget or cancellation refusal yields no answer owner.
pub fn evaluate_demanded<D: TermDictionary>(
    rules: &[DlClause<3>],
    goal: &ClauseAtom<3>,
    edb: RelationStore<D, 3>,
    options: &EvalOptions,
    stop: Option<&dyn StopSignal>,
) -> Result<DemandAnswer<D>, DemandError<D::Error>> {
    validate(rules, goal)?;
    let descriptors = descriptors(rules, goal);
    let names = names(&descriptors, rules, goal, edb.interner())?;
    let transformed = transform(rules, goal, &descriptors, &names);
    let executable = compile(transformed).map_err(DemandError::Compile)?;
    let evaluation = evaluate_guarded(&executable, edb, &NoGuards, options, stop)
        .map_err(DemandError::Evaluation)?;
    Ok(DemandAnswer {
        evaluation,
        goal: goal.clone(),
    })
}

fn validate<E>(rules: &[DlClause<3>], goal: &ClauseAtom<3>) -> Result<(), DemandError<E>> {
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
        if !rule.negations().is_empty() {
            return Err(refuse(Unsupported::Negation));
        }
        for atom in rule.body().iter().chain(rule.head_atoms()) {
            validate_atom(atom).map_err(refuse)?;
        }
        let bound = rule.bound_variables();
        for term in rule.datalog_head().expect("validated atomic head").terms() {
            if let Some(variable) = term.variable()
                && !bound.contains(variable)
            {
                return Err(refuse(Unsupported::UnboundVariable(variable.to_owned())));
            }
        }
    }
    validate_atom(goal).map_err(|reason| DemandError::Unsupported { rule: None, reason })
}

fn validate_atom(atom: &ClauseAtom<3>) -> Result<(), Unsupported> {
    if atom.is_negated() {
        return Err(Unsupported::Negation);
    }
    if atom.read_source() != ReadSource::Evaluation {
        return Err(Unsupported::BaseRead);
    }
    if atom.predicate_iri().is_none() {
        return Err(Unsupported::Predicate);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Descriptor {
    predicate: String,
    pattern: BindingPattern,
}

fn arguments(atom: &ClauseAtom<3>) -> [&ClauseTerm; 2] {
    [atom.subject(), atom.object()]
}
fn pattern(atom: &ClauseAtom<3>, bound: &BTreeSet<String>) -> BindingPattern {
    BindingPattern::from_bools(
        arguments(atom).map(|term| term.variable().is_none_or(|name| bound.contains(name))),
    )
}
fn bind(atom: &ClauseAtom<3>, bound: &mut BTreeSet<String>) {
    bound.extend(
        arguments(atom)
            .into_iter()
            .filter_map(ClauseTerm::variable)
            .map(str::to_owned),
    );
}
fn head_bindings(head: &ClauseAtom<3>, pattern: BindingPattern) -> BTreeSet<String> {
    arguments(head)
        .into_iter()
        .enumerate()
        .filter(|(at, _)| pattern.is_bound(*at))
        .filter_map(|(_, term)| term.variable().map(str::to_owned))
        .collect()
}

/// Finite adornment closure followed by its minimal (most general) antichains.
/// Keep discovered patterns monotonically: a broader pattern may expose new demands.
fn descriptors(rules: &[DlClause<3>], goal: &ClauseAtom<3>) -> Vec<Descriptor> {
    let definitions: BTreeSet<_> = rules
        .iter()
        .map(|rule| rule.datalog_head().unwrap().predicate_iri().unwrap())
        .collect();
    let mut found = BTreeSet::from([Descriptor {
        predicate: goal.predicate_iri().unwrap().to_owned(),
        pattern: pattern(goal, &BTreeSet::new()),
    }]);
    loop {
        let mut next = found.clone();
        for demand in &found {
            for rule in rules.iter().filter(|rule| {
                rule.datalog_head().unwrap().predicate_iri() == Some(demand.predicate.as_str())
            }) {
                let mut bound = head_bindings(rule.datalog_head().unwrap(), demand.pattern);
                for atom in rule.body() {
                    if definitions.contains(atom.predicate_iri().unwrap()) {
                        next.insert(Descriptor {
                            predicate: atom.predicate_iri().unwrap().to_owned(),
                            pattern: pattern(atom, &bound),
                        });
                    }
                    bind(atom, &mut bound);
                }
            }
        }
        if next == found {
            break;
        }
        found = next;
    }
    found
        .iter()
        .filter(|entry| {
            !found.iter().any(|other| {
                other.predicate == entry.predicate
                    && other.pattern != entry.pattern
                    && other.pattern.subsumes(entry.pattern)
            })
        })
        .cloned()
        .collect()
}

const NAMESPACE: Domain = Domain::new(b"purrdf-datalog/demand-predicates/v1");

/// One digest, one common collision nonce and unique sorted ordinals avoid
/// both existing dictionary terms and collisions between internal descriptors.
fn names<D: TermDictionary>(
    descriptors: &[Descriptor],
    rules: &[DlClause<3>],
    goal: &ClauseAtom<3>,
    dictionary: &D,
) -> Result<Vec<String>, DemandError<D::Error>> {
    let authored: BTreeSet<_> = rules
        .iter()
        .flat_map(|rule| rule.body().iter().chain(rule.head_atoms()))
        .chain(std::iter::once(goal))
        .flat_map(ClauseAtom::terms)
        .filter_map(ClauseTerm::surface)
        .collect();
    let mut digest = purrdf_hash::blake3::Hasher::new();
    digest.update(NAMESPACE.as_bytes());
    frame_le_into(&mut digest, &crate::cache::canonical_rule_hash(rules));
    let goal_clause = DlClause::datalog(goal.clone(), Vec::new());
    frame_le_into(
        &mut digest,
        &crate::cache::canonical_rule_hash(&[goal_clause]),
    );
    for descriptor in descriptors {
        frame_le_into(&mut digest, descriptor.predicate.as_bytes());
        frame_le_into(&mut digest, descriptor.pattern.code().as_bytes());
    }
    let digest = purrdf_hash::hex::encode(digest.finalize().as_bytes());
    dictionary.check_read().map_err(DemandError::Source)?;
    let mut nonce = 0_u64;
    loop {
        let candidates: Vec<_> = (0..=descriptors.len())
            .map(|ordinal| format!("urn:purrdf:demand:{digest}:{nonce}:{ordinal}"))
            .collect();
        let mut collision = false;
        for name in &candidates {
            let surface = format!("<{name}>");
            // Check the entire original dictionary, even terms absent from rows.
            let existing = dictionary
                .lookup_term(&surface)
                .map_err(DemandError::Source)?;
            collision |= existing.is_some() || authored.contains(&surface);
        }
        dictionary.check_read().map_err(DemandError::Source)?;
        if !collision {
            return Ok(candidates);
        }
        nonce = nonce
            .checked_add(1)
            .ok_or(DemandError::NamespaceExhausted)?;
    }
}

fn demand_atom(
    atom: &ClauseAtom<3>,
    descriptor: &Descriptor,
    name: &str,
    unit: &str,
) -> ClauseAtom<3> {
    let [subject, object] = arguments(atom).map(Clone::clone);
    ClauseAtom::triple(
        if descriptor.pattern.is_bound(0) {
            subject
        } else {
            ClauseTerm::iri(unit)
        },
        ClauseTerm::iri(name),
        if descriptor.pattern.is_bound(1) {
            object
        } else {
            ClauseTerm::iri(unit)
        },
    )
}

fn select<'a>(
    descriptors: &'a [Descriptor],
    predicate: &str,
    pattern: BindingPattern,
) -> Option<(usize, &'a Descriptor)> {
    descriptors.iter().enumerate().find(|(_, descriptor)| {
        descriptor.predicate == predicate && descriptor.pattern.subsumes(pattern)
    })
}

fn transform(
    rules: &[DlClause<3>],
    goal: &ClauseAtom<3>,
    descriptors: &[Descriptor],
    names: &[String],
) -> Vec<DlClause<3>> {
    let unit = names.last().expect("the unit has its own ordinal");
    let (seed, descriptor) = select(
        descriptors,
        goal.predicate_iri().unwrap(),
        pattern(goal, &BTreeSet::new()),
    )
    .expect("the goal demand is covered");
    let mut transformed = vec![DlClause::datalog(
        demand_atom(goal, descriptor, &names[seed], unit),
        Vec::new(),
    )];
    let definitions: BTreeMap<_, Vec<_>> = rules.iter().fold(BTreeMap::new(), |mut map, rule| {
        map.entry(rule.datalog_head().unwrap().predicate_iri().unwrap())
            .or_default()
            .push(rule);
        map
    });
    for (ordinal, descriptor) in descriptors.iter().enumerate() {
        let Some(rules) = definitions.get(descriptor.predicate.as_str()) else {
            continue;
        };
        for rule in rules {
            let head = rule.datalog_head().unwrap();
            let magic = demand_atom(head, descriptor, &names[ordinal], unit);
            let mut bound = head_bindings(head, descriptor.pattern);
            let mut prefix = vec![magic];
            for atom in rule.body() {
                if let Some((at, needed)) = select(
                    descriptors,
                    atom.predicate_iri().unwrap(),
                    pattern(atom, &bound),
                ) {
                    transformed.push(DlClause::datalog(
                        demand_atom(atom, needed, &names[at], unit),
                        prefix.clone(),
                    ));
                }
                prefix.push(atom.clone());
                bind(atom, &mut bound);
            }
            transformed.push(DlClause::datalog(head.clone(), prefix));
        }
    }
    transformed
}
