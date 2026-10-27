// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete answers to one positive triple or quad goal without materializing unrelated rules.
//!
//! A magic-set transformation carries bound subject/object/graph coordinates through
//! authored body order. Subsuming adornments share one demand relation; the
//! original atoms and final goal retain constants and repeated-variable equality.
//! The transformed clauses run through the ordinary compiler and semi-naive engine.
//!
//! Preparation uses resident allocation, just like [`compile`]. Evaluation uses
//! the caller's limits and stop signal. Work and budget reports count the transformed
//! program, including its internal demand facts, and all original input facts.
//! The retained partial closure is private: it is complete only for the requested goal.

mod stratified;

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
use crate::store::{Arity, Bound, RelationStore, StoreShape};
use purrdf_hash::{Domain, frame::frame_le_into};

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
    /// The ordinary compiler refused the authored or transformed clauses.
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
pub struct DemandAnswer<D: TermDictionary, const N: usize = 3>
where
    Arity<N>: StoreShape<N>,
{
    evaluation: Evaluation<D, N>,
    goal: ClauseAtom<N>,
    preparation: Preparation,
}

/// Resident source preparation before the governed evaluator starts.
///
/// Shadow facts and terms are part of the evaluator's transformed input baseline,
/// including its generated-term allowance. They are not reported as derived facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preparation {
    /// Original caller-owned rows before shadow seeding.
    pub original_facts: usize,
    /// Original dictionary's distinct terms before shadow seeding.
    pub original_terms: u64,
    /// Additional input rows in isolated shadow relations.
    pub shadow_facts: usize,
    /// Additional dictionary terms minted while seeding those rows.
    pub shadow_terms: u64,
}

impl<D: TermDictionary, const N: usize> DemandAnswer<D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Original input and resident shadow-seeding counts.
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
        let dictionary = self.dictionary();
        dictionary.check_read()?;
        let terms = self.goal.terms();
        let mut constants = [None; N];
        for (at, term) in terms.iter().enumerate() {
            if let Some(surface) = term.surface() {
                let Some(id) = dictionary.lookup_term(&surface)? else {
                    dictionary.check_read()?;
                    return Ok(Vec::new());
                };
                constants[at] = Some(id);
            }
        }
        let mut answers = Vec::new();
        for partition in self.evaluation.facts().partitions_for(constants) {
            let bound = match (constants[0], constants[2]) {
                (Some(subject), Some(object)) => Bound::Both(subject, object),
                (Some(subject), None) => Bound::Subject(subject),
                (None, Some(object)) => Bound::Object(object),
                (None, None) => Bound::Any,
            };
            let mut rows = partition.select(bound);
            while let Some((subject, object, _)) = rows.next() {
                let row = partition.row(subject, object);
                if terms.iter().enumerate().all(|(at, term)| {
                    !term.is_var()
                        || (0..at).all(|before| terms[before] != *term || row[before] == row[at])
                }) {
                    answers.push(row);
                }
            }
        }
        dictionary.check_read()?;
        Ok(answers)
    }
}

/// Answer one positive, fixed-predicate triple or quad goal with demand propagation.
///
/// The complete authored program and goal are validated before dictionary mutation
/// or execution. Atomic heads must be range-restricted; guards, base/model reads
/// and every other head form are refused. Stratified negation, including guard-free
/// conjunctions, discovers demand through an isolated positive shadow of the input.
/// Shadow input is resident preparation and belongs to the transformed input baseline;
/// [`DemandAnswer::preparation`] reports its additional rows and terms. The ordinary
/// evaluator alone enforces execution options and cancellation.
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
    validate(rules, goal)?;
    if rules
        .iter()
        .any(|rule| !rule.negations().is_empty() || rule.body().iter().any(ClauseAtom::is_negated))
    {
        return stratified::evaluate(rules, goal, edb, options, stop);
    }
    let preparation = Preparation {
        original_facts: edb.row_count(),
        original_terms: edb.interner().term_count(),
        shadow_facts: 0,
        shadow_terms: 0,
    };
    let descriptors = descriptors(rules, goal);
    let names = names(&descriptors, rules, goal, edb.interner(), 0)?;
    let transformed = transform(rules, goal, &descriptors, &names);
    let executable = compile(transformed).map_err(DemandError::Compile)?;
    let evaluation = evaluate_guarded(&executable, edb, &NoGuards, options, stop)
        .map_err(DemandError::Evaluation)?;
    Ok(DemandAnswer {
        evaluation,
        goal: goal.clone(),
        preparation,
    })
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

fn arguments<const N: usize>(atom: &ClauseAtom<N>) -> impl Iterator<Item = &ClauseTerm> {
    atom.terms()
        .into_iter()
        .enumerate()
        .filter_map(|(at, term)| (at != 1).then_some(term))
}
fn pattern<const N: usize>(atom: &ClauseAtom<N>, bound: &BTreeSet<String>) -> BindingPattern {
    BindingPattern::from_bools(
        arguments(atom).map(|term| term.variable().is_none_or(|name| bound.contains(name))),
    )
}
fn bind<const N: usize>(atom: &ClauseAtom<N>, bound: &mut BTreeSet<String>) {
    bound.extend(
        arguments(atom)
            .filter_map(ClauseTerm::variable)
            .map(str::to_owned),
    );
}
fn head_bindings<const N: usize>(
    head: &ClauseAtom<N>,
    pattern: BindingPattern,
) -> BTreeSet<String> {
    arguments(head)
        .enumerate()
        .filter(|(at, _)| pattern.is_bound(*at))
        .filter_map(|(_, term)| term.variable().map(str::to_owned))
        .collect()
}

/// Finite adornment closure followed by its minimal (most general) antichains.
/// Keep discovered patterns monotonically: a broader pattern may expose new demands.
fn descriptors<const N: usize>(rules: &[DlClause<N>], goal: &ClauseAtom<N>) -> Vec<Descriptor> {
    let definitions: BTreeSet<_> = rules
        .iter()
        // validate admitted one atomic head and an IRI predicate for every rule.
        .map(|rule| rule.datalog_head().unwrap().predicate_iri().unwrap())
        .collect();
    let mut found = BTreeSet::from([Descriptor {
        // validate_atom checked the goal's predicate before descriptors is called.
        predicate: goal.predicate_iri().unwrap().to_owned(),
        pattern: pattern(goal, &BTreeSet::new()),
    }]);
    loop {
        let mut next = found.clone();
        for demand in &found {
            for rule in rules.iter().filter(|rule| {
                // validate admitted only atomic heads; these immutable rules are unchanged.
                rule.datalog_head().unwrap().predicate_iri() == Some(demand.predicate.as_str())
            }) {
                // The same validated rule still has its single atomic head.
                let mut bound = head_bindings(rule.datalog_head().unwrap(), demand.pattern);
                for atom in rule.body() {
                    // validate_atom checked every authored body predicate as an IRI.
                    if definitions.contains(atom.predicate_iri().unwrap()) {
                        next.insert(Descriptor {
                            // This is the same validated body atom used by the lookup above.
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
fn names<D: TermDictionary, const N: usize>(
    descriptors: &[Descriptor],
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    dictionary: &D,
    additional: usize,
) -> Result<Vec<String>, DemandError<D::Error>> {
    let authored: BTreeSet<_> = rules
        .iter()
        .flat_map(|rule| {
            rule.body().iter().chain(rule.head_atoms()).chain(
                rule.negations()
                    .iter()
                    .flat_map(crate::guard::Negation::atoms),
            )
        })
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
        let candidates: Vec<_> = (0..=descriptors.len() + additional)
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

fn demand_atom<const N: usize>(
    atom: &ClauseAtom<N>,
    descriptor: &Descriptor,
    name: &str,
    unit: &str,
) -> ClauseAtom<N> {
    // Dense payload adornments [subject, object, graph] retain their native
    // positions [0, 2, 3]. In particular, DefaultGraph never leaves its graph slot.
    let mut positions = atom.terms().map(Clone::clone);
    positions[1] = ClauseTerm::iri(name);
    for (payload, at) in (0..N).filter(|at| *at != 1).enumerate() {
        if !descriptor.pattern.is_bound(payload) {
            positions[at] = ClauseTerm::iri(unit);
        }
    }
    ClauseAtom::from_positions(positions)
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

fn transform<const N: usize>(
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    descriptors: &[Descriptor],
    names: &[String],
) -> Vec<DlClause<N>> {
    // names emits one ordinal per descriptor plus a unit ordinal, even for an empty set.
    let unit = names.last().expect("the unit has its own ordinal");
    // descriptors starts with the goal pattern; antichain reduction retains either
    // that pattern or a subsuming pattern for the same predicate, so select succeeds.
    let (seed, descriptor) = select(
        descriptors,
        // validate_atom admitted this unchanged goal only with an IRI predicate.
        goal.predicate_iri().unwrap(),
        pattern(goal, &BTreeSet::new()),
    )
    .expect("the goal demand is covered");
    let mut transformed = vec![DlClause::datalog(
        demand_atom(goal, descriptor, &names[seed], unit),
        Vec::new(),
    )];
    let definitions: BTreeMap<_, Vec<_>> = rules.iter().fold(BTreeMap::new(), |mut map, rule| {
        // validate admitted one atomic head with an IRI predicate for every rule.
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
            // The definition map contains only the immutable, validated atomic rules.
            let head = rule.datalog_head().unwrap();
            let magic = demand_atom(head, descriptor, &names[ordinal], unit);
            let mut bound = head_bindings(head, descriptor.pattern);
            let mut prefix = vec![magic];
            for atom in rule.body() {
                if let Some((at, needed)) = select(
                    descriptors,
                    // validate_atom admitted every body atom only with an IRI predicate.
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
