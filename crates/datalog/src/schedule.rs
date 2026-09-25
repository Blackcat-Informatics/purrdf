// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ORDERED schedule: the same rule evaluator, driven layer by layer and group by
//! group in an order the caller's rule language defines.
//!
//! [`crate::seminaive`] computes the least model of a stratified program: the strata are
//! derived from the program, and inside a stratum rule order is unobservable. Two rule
//! languages PurRDF evaluates define their answer by an EXECUTION ORDER instead, and
//! this module is the one place that order is run — over the one rule IR
//! ([`DlClause`]), the one join, the one guard seam ([`crate::guard`]) and the one commit
//! the stratified fixpoint uses, so there is still exactly one rule evaluator:
//!
//! * SHACL 1.2 Inference Rules, "General Execution Instructions for SHACL Rules":
//!   "For all layers in the rule set (in ascending order): Compute the expected derived
//!   triples for all rules in the layer; Execute one iteration over all run-once rules
//!   in the layer; do Execute one iteration over all iterating rules in the layer while
//!   the iteration has produced newly inferred triples; Delete the derived triples
//!   (except those that were also inferred by rules) and their reifiers. Delete the
//!   temporary triples and their reifiers." — with, inside a layer, "rules with larger
//!   order values will be executed after those with smaller values. […] Rules with the
//!   same order are executed concurrently and must not see each other's inferences
//!   before they have all completed."
//! * SPARQL 1.2 RL, "Evaluation of a Rule Set": "A stratum is evaluated by first
//!   evaluating each of the run-once rules of that stratum, and then evaluating general
//!   rules of the stratum repeatedly until no new triples are produced" — with the
//!   strata computed by [`stratify_rules`] from clause atoms, or by
//!   [`stratify_dependency_graph`] from a dependency graph the rule language built.
//!
//! # The shape
//!
//! A [`Schedule`] is a sequence of [`Layer`]s. A layer holds RUN-ONCE groups, each
//! evaluated exactly once, and ITERATING groups, evaluated in order, pass after pass,
//! until a whole pass commits nothing. A GROUP is a set of rules evaluated CONCURRENTLY:
//! every rule of a group reads the model as it stood when the group started, and their
//! derivations are merged and committed together — which is the stratified fixpoint's
//! round, and exactly SHACL's same-order rule. A language whose rules run one after
//! another (SPARQL 1.2 RL) schedules one rule per group.
//!
//! # Semantics: inflationary, not least-model
//!
//! Negation — a negated atom, a negated conjunction, or a model-reading guard's
//! `NOT EXISTS` — is decided against the model AS IT STANDS when the rule runs. Facts are
//! never retracted inside a layer, so the model only grows, and a solution a negation
//! blocked once stays blocked: that is what makes the per-rule semi-naive delta exact
//! here too. A rule whose guard reads the model is re-evaluated against the whole model
//! each time it runs, because its answer may depend on any fact at all.
//!
//! # Assumptions and retractions
//!
//! A layer may carry ASSUMED facts ([`LayerHooks::assume`]): visible to every rule of the
//! layer, and withdrawn at its end unless some rule of the layer derived them too — SHACL's
//! expected derived triples. The hook then names further facts to retract with them
//! ([`LayerHooks::retract`]; a derived triple's reifiers), and one more hook runs after the
//! last layer ([`LayerHooks::finish`]; SHACL's temporary triples). Retraction rebuilds the
//! store from the surviving rows in their original row order, so the relative order every
//! observable is derived from is unchanged.
//!
//! # Termination
//!
//! Every ceiling of [`crate::seminaive`] applies, each round of each group counting as a
//! round — the caller's term-generating round limit
//! ([`EvalOptions`](crate::seminaive::EvalOptions)) included, which is what stops a rule
//! that computes a new term every pass (the SHACL rules specification: "Rule engines MAY
//! also report a failure after a pre-configured maximum iteration count has been
//! exceeded").

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use crate::cache::{self, ContractHash};
use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadForm};
use crate::guard::{GuardEvaluator, Negation};
use crate::id::RowId;
use crate::plan::RulePlan;
use crate::seminaive::{
    Delta, EvalError, EvalOptions, Evaluation, FixpointState, JoinStrategy, RoundExecution,
    RoundSnapshot, RuleEntry, RuleRuntime, check_budget, check_range_restricted, evaluate_round,
};
use crate::stop::{StopSignal, is_stopped};
use crate::store::{Fact, RelationStore};

/// One layer of an ordered schedule: its run-once groups and its iterating groups.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Layer {
    /// Groups evaluated exactly once, in order, at the start of the layer.
    once: Vec<Vec<usize>>,
    /// Groups evaluated in order, pass after pass, until a pass commits nothing.
    iterating: Vec<Vec<usize>>,
}

impl Layer {
    /// A layer from its run-once and iterating groups, each group a list of rule
    /// indices in authored program order.
    pub fn new(once: Vec<Vec<usize>>, iterating: Vec<Vec<usize>>) -> Self {
        Self { once, iterating }
    }

    /// The run-once groups, in execution order.
    pub fn once(&self) -> &[Vec<usize>] {
        &self.once
    }

    /// The iterating groups, in execution order.
    pub fn iterating(&self) -> &[Vec<usize>] {
        &self.iterating
    }
}

/// An ordered schedule: layers in execution order. See the [module docs](self).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Schedule {
    /// The layers, in execution order.
    layers: Vec<Layer>,
}

impl Schedule {
    /// A schedule from its layers, in execution order.
    pub fn new(layers: Vec<Layer>) -> Self {
        Self { layers }
    }

    /// The layers, in execution order.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Every scheduled rule index, layer by layer, run-once groups before iterating ones.
    fn rule_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.layers.iter().flat_map(|layer| {
            layer
                .once
                .iter()
                .chain(&layer.iterating)
                .flat_map(|group| group.iter().copied())
        })
    }
}

/// A rule program compiled for the ordered schedule.
#[derive(Debug, Clone)]
pub struct ScheduledProgram {
    /// The program, in authored order.
    rules: Arc<[DlClause]>,
    /// One store-independent join plan per rule.
    plans: Vec<RulePlan>,
    /// One lowered runtime per rule.
    runtimes: Vec<RuleRuntime>,
    /// The schedule the program runs under.
    schedule: Schedule,
}

impl ScheduledProgram {
    /// The program's rules, in authored order.
    pub fn rules(&self) -> &[DlClause] {
        &self.rules
    }

    /// The schedule the program runs under.
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// The identity of the calculus this program is evaluated under `options`: the
    /// clause program, the schedule, every ceiling and the crate's calculus version
    /// ([`cache::scheduled_contract_hash`]).
    pub fn contract_hash(&self, options: &EvalOptions) -> ContractHash {
        cache::scheduled_contract_hash(&self.rules, &self.schedule, options)
    }
}

/// Compile `rules` for the ordered `schedule`, or refuse them.
///
/// Admits the ATOMIC and the CONJUNCTIVE head forms: a conjunctive head asserts every
/// conjunct from one solution, which is how a rule whose head writes several triples
/// sharing one fresh term (a blank node a guard minted) keeps them co-referring. The
/// other three forms have no forward-chaining meaning and are refused by name. Every head
/// variable must be range-restricted, and the schedule must mention every rule exactly
/// once. Stratification is NOT required: see the [module docs](self).
///
/// # Errors
///
/// [`EvalError::NonDatalogHead`], [`EvalError::UnboundHeadVariable`] or
/// [`EvalError::MalformedSchedule`].
pub fn compile_scheduled(
    rules: Vec<DlClause>,
    schedule: Schedule,
) -> Result<ScheduledProgram, EvalError> {
    for (index, rule) in rules.iter().enumerate() {
        let form = rule.head_form();
        if !matches!(form, HeadForm::Atomic | HeadForm::Conjunctive) {
            return Err(EvalError::NonDatalogHead { rule: index, form });
        }
    }
    check_range_restricted(&rules)?;
    let mut seen: BTreeSet<usize> = BTreeSet::new();
    for index in schedule.rule_indices() {
        if index >= rules.len() {
            return Err(EvalError::MalformedSchedule {
                detail: format!(
                    "the schedule names rule {index}, but the program has {} rules",
                    rules.len()
                ),
            });
        }
        if !seen.insert(index) {
            return Err(EvalError::MalformedSchedule {
                detail: format!("rule {index} is scheduled more than once"),
            });
        }
    }
    if let Some(missing) = (0..rules.len()).find(|index| !seen.contains(index)) {
        return Err(EvalError::MalformedSchedule {
            detail: format!("rule {missing} is not scheduled"),
        });
    }
    let plans: Vec<RulePlan> = rules.iter().map(RulePlan::for_rule).collect();
    let runtimes = rules
        .iter()
        .zip(&plans)
        .map(|(rule, plan)| RuleRuntime::new(rule, plan))
        .collect();
    Ok(ScheduledProgram {
        rules: Arc::from(rules),
        plans,
        runtimes,
        schedule,
    })
}

/// The caller's layer-boundary actions. Every method has a do-nothing default.
pub trait LayerHooks {
    /// The facts to ASSUME for the duration of layer `layer`, read over the model it
    /// starts from. See the [module docs](self).
    ///
    /// # Errors
    ///
    /// A message, which aborts the evaluation as [`EvalError::LayerHook`].
    fn assume(&mut self, layer: usize, model: &RelationStore) -> Result<Vec<Fact>, String> {
        let _ = (layer, model);
        Ok(Vec::new())
    }

    /// Further facts to retract at the end of layer `layer`, alongside the `withdrawn`
    /// assumptions no rule of the layer derived. Read over the model before anything is
    /// retracted.
    ///
    /// # Errors
    ///
    /// A message, which aborts the evaluation as [`EvalError::LayerHook`].
    fn retract(
        &mut self,
        layer: usize,
        model: &RelationStore,
        withdrawn: &[Fact],
    ) -> Result<Vec<Fact>, String> {
        let _ = (layer, model, withdrawn);
        Ok(Vec::new())
    }

    /// Facts to retract after the last layer.
    ///
    /// # Errors
    ///
    /// A message, which aborts the evaluation as [`EvalError::LayerHook`].
    fn finish(&mut self, model: &RelationStore) -> Result<Vec<Fact>, String> {
        let _ = model;
        Ok(Vec::new())
    }
}

/// The hooks of a schedule with no layer-boundary actions.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoHooks;

impl LayerHooks for NoHooks {}

/// Evaluate `program` over the seeded store `edb` under its schedule.
///
/// The returned [`Evaluation`]'s derivations are those of the facts in its final model:
/// a fact retracted at a layer boundary takes its derivation with it, while a surviving
/// fact's derivation keeps naming the sources it was derived from, retracted or not. An
/// assumed fact a rule derived too carries that rule's derivation.
///
/// # Errors
///
/// Every refusal [`crate::seminaive::evaluate_guarded`] makes, plus
/// [`EvalError::LayerHook`].
pub fn evaluate_scheduled(
    program: &ScheduledProgram,
    edb: RelationStore,
    guards: &dyn GuardEvaluator,
    hooks: &mut dyn LayerHooks,
    options: &EvalOptions,
    stop: Option<&dyn StopSignal>,
) -> Result<Evaluation, EvalError> {
    let mut state = FixpointState::seeded(edb, *options);
    check_budget(&state)?;
    for (layer_index, layer) in program.schedule.layers.iter().enumerate() {
        run_layer(program, layer_index, layer, &mut state, guards, hooks, stop)?;
    }
    let finished = hooks
        .finish(&state.rel)
        .map_err(|message| EvalError::LayerHook {
            layer: None,
            message,
        })?;
    retract(&mut state, &finished);
    Ok(state.finish())
}

/// Run one layer into `state`.
fn run_layer(
    program: &ScheduledProgram,
    layer_index: usize,
    layer: &Layer,
    state: &mut FixpointState,
    guards: &dyn GuardEvaluator,
    hooks: &mut dyn LayerHooks,
    stop: Option<&dyn StopSignal>,
) -> Result<(), EvalError> {
    let hook_error = |message: String| EvalError::LayerHook {
        layer: Some(layer_index),
        message,
    };
    // The layer's assumptions become rows like any other, flagged so a rule that derives
    // one again is recorded as having done so.
    let assumptions = hooks.assume(layer_index, &state.rel).map_err(hook_error)?;
    let mut assumed = vec![false; state.rel.row_count()];
    for fact in &assumptions {
        if let Some((_, _, row)) =
            state
                .rel
                .insert(&fact.subject, &fact.predicate, &fact.object, &fact.graph)
        {
            state.depth.push(0);
            assumed.push(true);
            debug_assert_eq!(row.index() + 1, assumed.len(), "assumed tracks store rows");
        }
    }
    check_budget(state)?;
    let mut confirmed: BTreeMap<RowId, crate::seminaive::Derivation> = BTreeMap::new();

    // Run-once groups: each rule evaluated exactly once, against the whole model.
    for group in &layer.once {
        run_group(
            program,
            group,
            state,
            GroupRun {
                guards,
                stop,
                assumed: &mut assumed,
                confirmed: &mut confirmed,
                seen: None,
            },
        )?;
    }

    // Iterating groups: pass after pass until a whole pass commits nothing. `seen[r]` is
    // the row count rule `r` last ran against, so its next run's delta is exactly what
    // was committed since — the per-rule semi-naive delta.
    let mut seen: Vec<Option<usize>> = vec![None; program.rules.len()];
    loop {
        let mut progressed = false;
        for group in &layer.iterating {
            progressed |= run_group(
                program,
                group,
                state,
                GroupRun {
                    guards,
                    stop,
                    assumed: &mut assumed,
                    confirmed: &mut confirmed,
                    seen: Some(&mut seen),
                },
            )?;
        }
        if !progressed {
            break;
        }
    }

    // Withdraw every assumption no rule derived, and whatever the hook retracts with them;
    // record the derivations of the ones a rule did derive.
    let rows = state.rel.facts_in_row_order();
    let withdrawn: Vec<Fact> = rows
        .iter()
        .filter(|(row, _)| {
            assumed.get(row.index()).copied().unwrap_or(false) && !confirmed.contains_key(row)
        })
        .map(|(_, fact)| fact.clone())
        .collect();
    state.derivations.extend(confirmed.into_values());
    let mut retracted = hooks
        .retract(layer_index, &state.rel, &withdrawn)
        .map_err(hook_error)?;
    retracted.extend(withdrawn);
    retract(state, &retracted);
    Ok(())
}

/// The per-layer state one group evaluation reads and updates.
struct GroupRun<'a, 'g> {
    /// The caller's guard evaluator.
    guards: &'g dyn GuardEvaluator,
    /// The caller's stop signal.
    stop: Option<&'g dyn StopSignal>,
    /// Per-row assumed flags.
    assumed: &'a mut Vec<bool>,
    /// The first derivation of each assumed row a rule derived again.
    confirmed: &'a mut BTreeMap<RowId, crate::seminaive::Derivation>,
    /// For an iterating group, the row count each rule last ran against; `None` for a
    /// run-once group, whose rules run once against everything.
    seen: Option<&'a mut Vec<Option<usize>>>,
}

/// Evaluate one group as one round and commit it. Returns whether it committed anything.
fn run_group(
    program: &ScheduledProgram,
    group: &[usize],
    state: &mut FixpointState,
    run: GroupRun<'_, '_>,
) -> Result<bool, EvalError> {
    if is_stopped(run.stop) {
        return Err(EvalError::Stopped {
            report: state.report(),
        });
    }
    let hi = state.rel.row_count();
    let mut entries: Vec<RuleEntry<'_>> = Vec::with_capacity(group.len());
    let mut seen = run.seen;
    for &index in group {
        let (rule, plan) = (&program.rules[index], &program.plans[index]);
        let delta = match seen.as_deref_mut() {
            None => Delta::all(hi),
            Some(seen) => {
                let last = seen[index].replace(hi);
                match last {
                    // A rule whose guard reads the model may change its answer when any
                    // fact changes: it always runs against everything.
                    _ if rule.reads_model() => Delta::all(hi),
                    None => Delta::all(hi),
                    // Nothing the rule's positive join could read has changed, and a rule
                    // with no positive atom has no delta at all: its answer can only have
                    // shrunk as the negations it reads became satisfied.
                    Some(last) if last == hi || plan.positive().is_empty() => continue,
                    Some(last) => Delta { lo: last, hi },
                }
            }
        };
        entries.push(RuleEntry {
            index,
            rule,
            plan,
            runtime: &program.runtimes[index],
            delta,
        });
    }
    if entries.is_empty() {
        return Ok(false);
    }
    let round = evaluate_round(
        &entries,
        RoundSnapshot {
            rel: &state.rel,
            depth: &state.depth,
            assumed: run.assumed,
        },
        RoundExecution::Parallel,
        JoinStrategy::Planned,
        state.allowance(),
        run.guards,
    )?;
    for (row, derivation) in round.confirmed() {
        run.confirmed
            .entry(*row)
            .or_insert_with(|| derivation.clone());
    }
    let committed = !round.is_empty();
    state.absorb(round)?;
    run.assumed.resize(state.rel.row_count(), false);
    Ok(committed)
}

/// Retract `facts` from `state`, rebuilding the store from the surviving rows in their
/// original row order and dropping the derivations of the retracted facts.
fn retract(state: &mut FixpointState, facts: &[Fact]) {
    if facts.is_empty() {
        return;
    }
    let doomed: BTreeSet<&Fact> = facts.iter().collect();
    let mut rebuilt = RelationStore::new();
    let mut depth = Vec::with_capacity(state.depth.len());
    for (row, fact) in state.rel.facts_in_row_order() {
        if doomed.contains(&fact) {
            continue;
        }
        rebuilt.insert(&fact.subject, &fact.predicate, &fact.object, &fact.graph);
        depth.push(state.depth[row.index()]);
    }
    state.rel = rebuilt;
    state.depth = depth;
    state
        .derivations
        .retain(|derivation| !doomed.contains(derivation.fact()));
}

// ── Rule-level stratification ───────────────────────────────────────────────────

/// Whether `pattern` (a body atom) could match a fact `template` (a head atom)
/// generates: SPARQL 1.2 RL's "the triple template can generate a triple that matches
/// the triple pattern".
///
/// Decided by unifying the two atoms position by position, each in its own variable
/// namespace: two constants must be the same surface, a variable takes whatever the
/// other side holds, and a variable used twice in one atom must take one value. A
/// position a guard fills (a triple term built or taken apart by caller code) is a
/// variable here, so it may match anything — the conservative direction, which can only
/// add a dependency, never lose one.
fn may_generate(pattern: &ClauseAtom, template: &ClauseAtom) -> bool {
    let mut unifier = Unifier::default();
    pattern
        .terms()
        .into_iter()
        .zip(template.terms())
        .all(|(left, right)| {
            let a = unifier.node(false, left);
            let b = unifier.node(true, right);
            unifier.union(a, b)
        })
}

/// A union-find over two atoms' variables and constants, each class carrying at most one
/// constant surface.
#[derive(Default)]
struct Unifier<'a> {
    /// `(parent, constant surface)` per node.
    classes: Vec<(usize, Option<String>)>,
    /// The node of each `(template side?, variable name)`.
    names: BTreeMap<(bool, &'a str), usize>,
}

impl<'a> Unifier<'a> {
    /// The node of `term` on one side: a variable's shared node, or a fresh constant one.
    fn node(&mut self, side: bool, term: &'a ClauseTerm) -> usize {
        let index = self.classes.len();
        match term.variable() {
            Some(name) => {
                let classes = &mut self.classes;
                *self.names.entry((side, name)).or_insert_with(|| {
                    classes.push((index, None));
                    index
                })
            }
            None => {
                self.classes.push((index, term.surface()));
                index
            }
        }
    }

    /// The class root of `node`, compressing the path.
    fn find(&mut self, mut node: usize) -> usize {
        while self.classes[node].0 != node {
            let parent = self.classes[node].0;
            self.classes[node].0 = self.classes[parent].0;
            node = parent;
        }
        node
    }

    /// Merge the classes of `a` and `b`; `false` when they hold different constants.
    fn union(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return true;
        }
        let merged = match (self.classes[ra].1.take(), self.classes[rb].1.take()) {
            (Some(x), Some(y)) if x != y => return false,
            (Some(x), _) | (_, Some(x)) => Some(x),
            (None, None) => None,
        };
        self.classes[rb].0 = ra;
        self.classes[ra].1 = merged;
        true
    }
}

/// One rule's body reads, each with whether the dependency it induces is CLOSED.
fn body_reads(rule: &DlClause) -> Vec<(&ClauseAtom, bool)> {
    rule.body()
        .iter()
        .map(|atom| (atom, atom.is_negated()))
        .chain(
            rule.negations()
                .iter()
                .flat_map(Negation::atoms)
                .map(|atom| (atom, true)),
        )
        .collect()
}

/// Stratify a rule set by RULE dependencies, SPARQL 1.2 RL's way, into an ordered
/// [`Schedule`]; `run_once[i]` says whether rule `i` is a run-once rule.
///
/// SPARQL 1.2 RL, "Rule Dependency": "Rule R1 depends on R2 if any triple pattern in the
/// body of R1, whether as a triple pattern element or inside a negation element, depends
/// on a triple template in the head of R2", and the dependency is CLOSED when "A triple
/// pattern occurring inside a negation element of R1 matches a triple template in the rule
/// head of R2" or "Rule R1 depends on rule R2 and R1 is a run-once rule". "The stratification
/// condition requires that there is no recursive dependency involving a closed dependency
/// in the dependency graph for a rule set." A rule whose guard reads the model
/// ([`crate::guard::GuardReads::Model`]) reads every relation with unknown polarity, so it
/// depends — closed — on every rule.
///
/// The strata follow the specification's algorithm: an open edge puts the depending rule
/// at or above the stratum of the rule it depends on, a closed edge strictly above. Each
/// stratum becomes one [`Layer`] whose run-once rules are each a group of their own, in
/// authored order, followed by its general rules, each a group of its own, in authored
/// order — "for each rule R in ST.once … for each rule R in ST.general", every rule seeing
/// the inferences of the rules before it.
///
/// # Errors
///
/// [`EvalError::NonStratifiableRules`] naming the first closed edge, in authored order,
/// that lies in a cycle, and one shortest cycle through it.
///
/// # Panics
///
/// Panics if `run_once` does not have one entry per rule — a construction bug.
pub fn stratify_rules(rules: &[DlClause], run_once: &[bool]) -> Result<Schedule, EvalError> {
    assert_eq!(
        rules.len(),
        run_once.len(),
        "stratify_rules needs one run-once flag per rule"
    );
    let mut graph = DependencyGraph::new(rules.len());
    for (r1, rule) in rules.iter().enumerate() {
        let reads = body_reads(rule);
        for (r2, other) in rules.iter().enumerate() {
            if rule.reads_model() && other.head_atoms().next().is_some() {
                graph.depend(r1, r2, true);
            }
            for (pattern, negated) in &reads {
                if other.head_atoms().any(|head| may_generate(pattern, head)) {
                    graph.depend(r1, r2, *negated || run_once[r1]);
                }
            }
        }
    }
    stratify_dependency_graph(&graph, run_once)
}

/// A rule-set DEPENDENCY GRAPH: SPARQL 1.2 RL, "A dependency graph of a rule set is a
/// directed graph where each vertex is a rule in the rule set, and an edge exists from
/// rule R1 to rule R2 if R1 depends on R2. The edge is labeled either open or closed".
///
/// [`stratify_rules`] builds one from clause atoms; a caller whose rule language can
/// decide "the triple template can generate a triple that matches the triple pattern"
/// more precisely than the clause IR can state it — a triple term taken apart or built by
/// caller code is an opaque guard variable to the clause IR, but a structured term to
/// the caller — builds its own and stratifies it with [`stratify_dependency_graph`], so
/// there is still exactly one stratifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraph {
    /// The number of rules (vertices).
    rules: usize,
    /// `(R1, R2) -> closed?`, merged "closed overrides open".
    edges: BTreeMap<(usize, usize), bool>,
}

impl DependencyGraph {
    /// A graph over `rules` vertices and no edges.
    #[must_use]
    pub fn new(rules: usize) -> Self {
        Self {
            rules,
            edges: BTreeMap::new(),
        }
    }

    /// Record that rule `rule` depends on rule `depends_on`, the dependency CLOSED when
    /// `closed`. A repeated edge keeps the specification's `mergeLabel`: "Closed
    /// dependency overrides open dependency."
    ///
    /// # Panics
    ///
    /// Panics if either index is not a vertex — a construction bug.
    pub fn depend(&mut self, rule: usize, depends_on: usize, closed: bool) {
        assert!(
            rule < self.rules && depends_on < self.rules,
            "dependency edge {rule} -> {depends_on} outside a {}-rule graph",
            self.rules
        );
        let label = self.edges.entry((rule, depends_on)).or_insert(false);
        *label |= closed;
    }

    /// The number of rules.
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules
    }

    /// Every edge as `(rule, depends_on, closed)`, in `(rule, depends_on)` order.
    pub fn edges(&self) -> impl Iterator<Item = (usize, usize, bool)> + '_ {
        self.edges
            .iter()
            .map(|(&(r1, r2), &closed)| (r1, r2, closed))
    }
}

/// Stratify a [`DependencyGraph`] SPARQL 1.2 RL's way into an ordered [`Schedule`];
/// `run_once[i]` says whether rule `i` is a run-once rule. See [`stratify_rules`] for the
/// condition checked and the strata produced.
///
/// # Errors
///
/// [`EvalError::NonStratifiableRules`] naming the first closed edge, in `(rule,
/// depends_on)` order, that lies in a cycle, and one shortest cycle through it.
///
/// # Panics
///
/// Panics if `run_once` does not have one entry per rule — a construction bug.
pub fn stratify_dependency_graph(
    graph: &DependencyGraph,
    run_once: &[bool],
) -> Result<Schedule, EvalError> {
    assert_eq!(
        graph.rules,
        run_once.len(),
        "stratify_dependency_graph needs one run-once flag per rule"
    );
    let edges = &graph.edges;
    let rule_count = graph.rules;

    // The stratification condition, decided before any stratum is assigned: a closed
    // edge R1 -> R2 violates it exactly when R2 reaches R1.
    let mut depends: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for &(r1, r2) in edges.keys() {
        depends.entry(r1).or_default().insert(r2);
    }
    for (&(r1, r2), &closed) in edges {
        if !closed {
            continue;
        }
        if let Some(path) = shortest_rule_path(&depends, r2, r1) {
            let mut cycle = vec![r1];
            cycle.extend(path);
            cycle.pop();
            return Err(EvalError::NonStratifiableRules {
                rule: r1,
                depends_on: r2,
                cycle,
            });
        }
    }

    // The specification's relaxation; it terminates because the condition holds.
    let mut stratum = vec![0usize; rule_count];
    let mut changed = true;
    while changed {
        changed = false;
        for (&(p, q), &closed) in edges {
            let need = if closed { stratum[q] + 1 } else { stratum[q] };
            if stratum[p] < need {
                stratum[p] = need;
                changed = true;
            }
        }
    }
    let top = stratum.iter().copied().max().unwrap_or(0);
    let layers = (0..=top)
        .filter(|level| stratum.contains(level))
        .map(|level| {
            let members = (0..rule_count).filter(|&index| stratum[index] == level);
            let once = members
                .clone()
                .filter(|&index| run_once[index])
                .map(|index| vec![index])
                .collect();
            let iterating = members
                .filter(|&index| !run_once[index])
                .map(|index| vec![index])
                .collect();
            Layer::new(once, iterating)
        })
        .collect();
    Ok(Schedule::new(layers))
}

/// The shortest `from -> … -> to` path through `depends`, inclusive of both ends, over
/// lexically ordered adjacency — a pure function of the rule set.
fn shortest_rule_path(
    depends: &BTreeMap<usize, BTreeSet<usize>>,
    from: usize,
    to: usize,
) -> Option<Vec<usize>> {
    let mut parent: BTreeMap<usize, usize> = BTreeMap::new();
    let mut queue: VecDeque<usize> = VecDeque::from([from]);
    let mut seen: BTreeSet<usize> = BTreeSet::from([from]);
    while let Some(node) = queue.pop_front() {
        if node == to {
            let mut path = vec![node];
            let mut cursor = node;
            while cursor != from {
                cursor = parent[&cursor];
                path.push(cursor);
            }
            path.reverse();
            return Some(path);
        }
        for &next in depends.get(&node).into_iter().flatten() {
            if seen.insert(next) {
                parent.insert(next, node);
                queue.push_back(next);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::clause::HeadDisjunct;
    use crate::guard::{Guard, GuardCall, GuardReads};
    use crate::seminaive::{
        BudgetResource, DEFAULT_MAX_TERM_GENERATING_ROUNDS, EvalOptions, compile, evaluate_guarded,
    };

    const EX: &str = "https://example.org/";

    fn iri(local: &str) -> ClauseTerm {
        ClauseTerm::iri(format!("{EX}{local}"))
    }

    fn v(name: &str) -> ClauseTerm {
        ClauseTerm::var(name)
    }

    fn atom(s: ClauseTerm, p: &str, o: ClauseTerm) -> ClauseAtom {
        ClauseAtom::positive(s, format!("{EX}{p}"), o)
    }

    fn surface(local: &str) -> String {
        format!("<{EX}{local}>")
    }

    fn store(facts: &[(&str, &str, &str)]) -> RelationStore {
        let mut store = RelationStore::new();
        for (s, p, o) in facts {
            store.insert(&surface(s), &surface(p), o, RelationStore::DEFAULT_GRAPH);
        }
        store
    }

    fn has(model: &RelationStore, s: &str, p: &str, o: &str) -> bool {
        model.contains(&surface(s), &surface(p), o, RelationStore::DEFAULT_GRAPH)
    }

    /// A test guard evaluator: `gt:N` keeps an integer literal input above `N`,
    /// `succ` answers the next integer, `cat` appends `x` to a string literal, `fresh`
    /// mints a counter-numbered blank, `count:p` counts the `p` facts of the model.
    #[derive(Default)]
    struct Guards {
        minted: std::cell::Cell<u32>,
    }

    fn int_of(surface: &str) -> Option<i64> {
        surface.trim_matches('"').parse().ok()
    }

    impl GuardEvaluator for Guards {
        fn evaluate(&self, call: &GuardCall<'_>) -> Result<Vec<Vec<String>>, String> {
            let name = call.guard.name();
            if let Some(bound) = name.strip_prefix("gt:") {
                let bound: i64 = bound.parse().map_err(|_| "bad bound".to_owned())?;
                let value = int_of(call.inputs[0]).ok_or("not an integer")?;
                return Ok(if value > bound {
                    vec![Vec::new()]
                } else {
                    Vec::new()
                });
            }
            if let Some(limit) = name.strip_prefix("succ-below:") {
                let limit: i64 = limit.parse().map_err(|_| "bad limit".to_owned())?;
                let value = int_of(call.inputs[0]).ok_or("not an integer")?;
                return Ok(if value + 1 < limit {
                    vec![vec![format!("\"{}\"", value + 1)]]
                } else {
                    Vec::new()
                });
            }
            match name {
                "succ" => {
                    let value = int_of(call.inputs[0]).ok_or("not an integer")?;
                    Ok(vec![vec![format!("\"{}\"", value + 1)]])
                }
                "cat" => {
                    let inner = call.inputs[0].trim_matches('"');
                    Ok(vec![vec![format!("\"{inner}x\"")]])
                }
                "fresh" => {
                    self.minted.set(self.minted.get() + 1);
                    Ok(vec![vec![format!("_:m{}", self.minted.get())]])
                }
                "fail" => Err("the guard could not decide".to_owned()),
                "count-p" => {
                    let count = call
                        .model
                        .len_for(&surface("p"), RelationStore::DEFAULT_GRAPH);
                    Ok(vec![vec![surface("a"), format!("\"{count}\"")]])
                }
                other => Err(format!("unknown guard {other}")),
            }
        }
    }

    fn scheduled(rules: Vec<DlClause>, schedule: Schedule, edb: RelationStore) -> Evaluation {
        let program = compile_scheduled(rules, schedule).expect("compiles");
        evaluate_scheduled(
            &program,
            edb,
            &Guards::default(),
            &mut NoHooks,
            &EvalOptions::default(),
            None,
        )
        .expect("evaluates")
    }

    /// A FILTER guard drops a solution and an assignment guard binds a fresh term — the
    /// SPARQL 1.2 RL `FILTER` and `SET` elements, evaluated semi-naively.
    #[test]
    fn a_filter_and_an_assignment_guard_run_in_the_stratified_fixpoint() {
        let rule = DlClause::datalog(
            atom(v("?x"), "next", v("?n")),
            vec![atom(v("?x"), "value", v("?v"))],
        )
        .with_guards(vec![
            Guard::filter("gt:1", vec!["?v".to_owned()]),
            Guard::assign("succ", vec!["?v".to_owned()], "?n"),
        ]);
        let exe = compile(vec![rule]).expect("pure guards stratify");
        let edb = store(&[("a", "value", "\"1\""), ("b", "value", "\"5\"")]);
        let model = evaluate_guarded(&exe, edb, &Guards::default(), &EvalOptions::default(), None)
            .expect("evaluates");
        assert!(has(model.facts(), "b", "next", "\"6\""));
        assert!(
            !has(model.facts(), "a", "next", "\"2\""),
            "the filter dropped a"
        );
        assert_eq!(model.derivations().len(), 1);
    }

    /// A guard that could not decide aborts the run by name; it is never read as "no".
    #[test]
    fn a_failing_guard_is_a_typed_error() {
        let rule = DlClause::datalog(
            atom(v("?x"), "q", v("?v")),
            vec![atom(v("?x"), "p", v("?v"))],
        )
        .with_guards(vec![Guard::filter("fail", vec!["?v".to_owned()])]);
        let exe = compile(vec![rule]).expect("compiles");
        let error = evaluate_guarded(
            &exe,
            store(&[("a", "p", "\"1\"")]),
            &Guards::default(),
            &EvalOptions::default(),
            None,
        )
        .expect_err("the guard fails");
        assert!(matches!(error, EvalError::Guard { rule: 0, .. }), "{error}");
        assert!(error.to_string().contains("could not decide"), "{error}");
    }

    /// A negated CONJUNCTION blocks a solution only when some extension matches every
    /// atom and passes every guard of the group.
    #[test]
    fn a_negated_conjunction_is_decided_as_a_whole() {
        let rule = DlClause::datalog(
            atom(v("?x"), "safe", iri("yes")),
            vec![atom(v("?x"), "type", iri("C"))],
        )
        .with_negations(vec![Negation::new(
            vec![
                atom(v("?x"), "exposed", v("?v")),
                atom(v("?v"), "severity", v("?s")),
            ],
            vec![Guard::filter("gt:8", vec!["?s".to_owned()])],
        )]);
        let exe = compile(vec![rule]).expect("stratifies");
        let edb = store(&[
            ("a", "type", &surface("C")),
            ("b", "type", &surface("C")),
            ("c", "type", &surface("C")),
            ("a", "exposed", &surface("v1")),
            ("b", "exposed", &surface("v2")),
            ("v1", "severity", "\"9\""),
            ("v2", "severity", "\"3\""),
        ]);
        let model = evaluate_guarded(&exe, edb, &Guards::default(), &EvalOptions::default(), None)
            .expect("evaluates");
        let yes = surface("yes");
        assert!(
            !has(model.facts(), "a", "safe", &yes),
            "a is critically exposed"
        );
        assert!(
            has(model.facts(), "b", "safe", &yes),
            "b's exposure is not severe"
        );
        assert!(
            has(model.facts(), "c", "safe", &yes),
            "c is exposed to nothing"
        );
    }

    /// The stratified fixpoint refuses a model-reading guard by name; the ordered
    /// schedule runs the very same rule.
    #[test]
    fn a_model_reading_guard_is_refused_by_the_stratifier_and_run_by_the_schedule() {
        let rule = || {
            DlClause::datalog(atom(v("?s"), "count", v("?n")), Vec::new()).with_guards(vec![
                Guard::new(
                    "count-p",
                    Vec::new(),
                    vec!["?s".to_owned(), "?n".to_owned()],
                    GuardReads::Model,
                ),
            ])
        };
        assert_eq!(
            compile(vec![rule()]).expect_err("refused"),
            EvalError::ModelReadingGuard { rule: 0 }
        );
        let model = scheduled(
            vec![rule()],
            Schedule::new(vec![Layer::new(vec![vec![0]], Vec::new())]),
            store(&[("a", "p", "\"1\""), ("b", "p", "\"2\"")]),
        );
        assert!(has(model.facts(), "a", "count", "\"2\""));
    }

    /// The term-generating round limit is the caller's. A counter stepping `?n + 1` to
    /// 1000 terminates after 1000 term-generating rounds and completes under the default
    /// limit; the SAME program under a limit of 500 is refused with the typed error naming
    /// 500 and how to raise it; a rule appending to a string every round with no bound is
    /// refused under the default, in bounded time.
    #[test]
    fn the_term_generating_round_limit_is_the_callers() {
        let counter = || {
            let rule = DlClause::datalog(
                atom(v("?x"), "value", v("?m")),
                vec![atom(v("?x"), "value", v("?n"))],
            )
            .with_guards(vec![Guard::assign(
                "succ-below:1001",
                vec!["?n".to_owned()],
                "?m",
            )]);
            compile(vec![rule]).expect("compiles")
        };
        let seed = || store(&[("a", "value", "\"0\"")]);
        let model = evaluate_guarded(
            &counter(),
            seed(),
            &Guards::default(),
            &EvalOptions::default(),
            None,
        )
        .expect("the 1000-step counter terminates under the default limit");
        assert!(has(model.facts(), "a", "value", "\"1000\""));
        assert_eq!(model.budget().term_generating_rounds(), 1000);
        assert_eq!(
            model.budget().term_generating_round_limit(),
            DEFAULT_MAX_TERM_GENERATING_ROUNDS
        );

        let limited = EvalOptions::default().with_max_term_generating_rounds(500);
        let error = evaluate_guarded(&counter(), seed(), &Guards::default(), &limited, None)
            .expect_err("the same counter is refused under a limit of 500");
        let rendered = error.to_string();
        let EvalError::BudgetExhausted { resource, report } = error else {
            panic!("expected a budget refusal, got {error}");
        };
        assert_eq!(resource, BudgetResource::TermGeneratingRounds);
        assert_eq!(report.term_generating_round_limit(), 500);
        assert_eq!(report.term_generating_rounds(), 501);
        assert!(rendered.contains("500 permitted"), "{rendered}");
        assert!(
            rendered.contains("with_max_term_generating_rounds"),
            "{rendered}"
        );

        let growing = DlClause::datalog(
            atom(v("?x"), "name", v("?m")),
            vec![atom(v("?x"), "name", v("?n"))],
        )
        .with_guards(vec![Guard::assign("cat", vec!["?n".to_owned()], "?m")]);
        let exe = compile(vec![growing]).expect("compiles");
        let error = evaluate_guarded(
            &exe,
            store(&[("a", "name", "\"s\"")]),
            &Guards::default(),
            &EvalOptions::default(),
            None,
        )
        .expect_err("an unbounded string-growing rule diverges");
        assert!(
            matches!(error, EvalError::BudgetExhausted { .. }),
            "a divergence is a typed ceiling refusal: {error}"
        );
    }

    /// A guard-free program never counts a term-generating round, and hashes exactly as it
    /// did; a guarded one hashes differently.
    #[test]
    fn guard_free_programs_keep_their_digests() {
        let plain = DlClause::datalog(
            atom(v("?x"), "q", v("?y")),
            vec![atom(v("?x"), "p", v("?y"))],
        );
        let guarded = plain
            .clone()
            .with_guards(vec![Guard::filter("gt:1", vec!["?y".to_owned()])]);
        assert_ne!(
            cache::canonical_rule_hash(std::slice::from_ref(&plain)),
            cache::canonical_rule_hash(std::slice::from_ref(&guarded))
        );
        assert_ne!(
            cache::contract_hash(std::slice::from_ref(&plain)),
            cache::contract_hash(std::slice::from_ref(&guarded))
        );
        let options = EvalOptions::default().with_max_term_generating_rounds(7);
        assert_eq!(
            cache::contract_hash_with(std::slice::from_ref(&plain), &options),
            cache::contract_hash(std::slice::from_ref(&plain)),
            "a guard-free program's contract ignores the round limit"
        );
        assert_ne!(
            cache::contract_hash_with(std::slice::from_ref(&guarded), &options),
            cache::contract_hash(std::slice::from_ref(&guarded)),
            "a guarded program's contract names the round limit in force"
        );
        let exe = compile(vec![plain]).expect("compiles");
        let model = crate::seminaive::evaluate(&exe, store(&[("a", "p", "\"1\"")])).expect("runs");
        assert_eq!(model.budget().term_generating_rounds(), 0);
    }

    /// Two negating rules in ONE group run concurrently and both fire; in two groups the
    /// first one's output blocks the second — SHACL's same-order rule, observed.
    #[test]
    fn a_group_runs_concurrently_and_groups_run_in_order() {
        let rules = || {
            let node = atom(v("?x"), "type", iri("Node"));
            vec![
                DlClause::datalog(
                    atom(v("?x"), "p", iri("t")),
                    vec![
                        node.clone(),
                        ClauseAtom::negated(v("?x"), format!("{EX}q"), iri("t")),
                    ],
                ),
                DlClause::datalog(
                    atom(v("?x"), "q", iri("t")),
                    vec![
                        node,
                        ClauseAtom::negated(v("?x"), format!("{EX}p"), iri("t")),
                    ],
                ),
            ]
        };
        let edb = || store(&[("n", "type", &surface("Node"))]);
        let t = surface("t");
        let together = scheduled(
            rules(),
            Schedule::new(vec![Layer::new(Vec::new(), vec![vec![0, 1]])]),
            edb(),
        );
        assert!(has(together.facts(), "n", "p", &t) && has(together.facts(), "n", "q", &t));
        let ordered = scheduled(
            rules(),
            Schedule::new(vec![Layer::new(Vec::new(), vec![vec![0], vec![1]])]),
            edb(),
        );
        assert!(has(ordered.facts(), "n", "p", &t) && !has(ordered.facts(), "n", "q", &t));
    }

    /// A run-once rule minting a fresh term runs exactly once. Left iterating, a rule
    /// whose guard is a function of its bindings fires once per body solution — the
    /// semi-naive delta never re-presents a solution — while the same rule reading the
    /// model is re-run against everything each pass, mints a new term every time, and is
    /// stopped by the ceiling with a typed error rather than hanging.
    #[test]
    fn a_run_once_group_runs_once() {
        let rule = |reads: GuardReads| {
            DlClause::new(
                vec![HeadDisjunct::new(vec![
                    atom(v("?x"), "head", v("?b")),
                    atom(v("?b"), "value", iri("zero")),
                ])],
                Vec::new(),
                vec![atom(v("?x"), "type", iri("Container"))],
            )
            .with_guards(vec![Guard::new(
                "fresh",
                Vec::new(),
                vec!["?b".to_owned()],
                reads,
            )])
        };
        let edb = || store(&[("c", "type", &surface("Container"))]);
        for layer in [
            Layer::new(vec![vec![0]], Vec::new()),
            Layer::new(Vec::new(), vec![vec![0]]),
        ] {
            let model = scheduled(
                vec![rule(GuardReads::Bindings)],
                Schedule::new(vec![layer]),
                edb(),
            );
            assert_eq!(model.derivations().len(), 2, "one solution, two conjuncts");
            assert!(
                model
                    .facts()
                    .contains(&surface("c"), &surface("head"), "_:m1", "")
            );
            assert!(
                model
                    .facts()
                    .contains("_:m1", &surface("value"), &surface("zero"), "")
            );
        }

        let once = scheduled(
            vec![rule(GuardReads::Model)],
            Schedule::new(vec![Layer::new(vec![vec![0]], Vec::new())]),
            edb(),
        );
        assert_eq!(once.derivations().len(), 2);
        let program = compile_scheduled(
            vec![rule(GuardReads::Model)],
            Schedule::new(vec![Layer::new(Vec::new(), vec![vec![0]])]),
        )
        .expect("compiles");
        let error = evaluate_scheduled(
            &program,
            edb(),
            &Guards::default(),
            &mut NoHooks,
            &EvalOptions::default(),
            None,
        )
        .expect_err("never converges");
        assert!(
            matches!(error, EvalError::BudgetExhausted { .. }),
            "{error}"
        );
        let error = evaluate_scheduled(
            &program,
            edb(),
            &Guards::default(),
            &mut NoHooks,
            &EvalOptions::default().with_max_term_generating_rounds(100),
            None,
        )
        .expect_err("never converges");
        let EvalError::BudgetExhausted { resource, report } = error else {
            panic!("expected a budget refusal, got {error}");
        };
        assert_eq!(resource, BudgetResource::TermGeneratingRounds);
        assert_eq!(report.term_generating_round_limit(), 100);
    }

    /// Hooks: an assumption is visible to the layer and withdrawn at its end unless a rule
    /// derived it too; the retraction hook and the finish hook remove what they name.
    #[test]
    fn assumptions_are_withdrawn_unless_derived() {
        struct Hooks;
        impl LayerHooks for Hooks {
            fn assume(&mut self, _: usize, _: &RelationStore) -> Result<Vec<Fact>, String> {
                Ok(vec![fact("a", "area", "\"1\""), fact("b", "area", "\"2\"")])
            }
            fn retract(
                &mut self,
                _: usize,
                _: &RelationStore,
                withdrawn: &[Fact],
            ) -> Result<Vec<Fact>, String> {
                assert_eq!(withdrawn, [fact("a", "area", "\"1\"")]);
                Ok(vec![fact("a", "note", "\"withdrawn\"")])
            }
            fn finish(&mut self, _: &RelationStore) -> Result<Vec<Fact>, String> {
                Ok(vec![fact("b", "temp", "\"t\"")])
            }
        }
        fn fact(s: &str, p: &str, o: &str) -> Fact {
            Fact {
                subject: surface(s),
                predicate: surface(p),
                object: o.to_owned(),
                graph: String::new(),
            }
        }
        let rules = vec![
            // Reads the assumption: every area-carrying node is small.
            DlClause::datalog(
                atom(v("?x"), "small", iri("yes")),
                vec![atom(v("?x"), "area", v("?a"))],
            ),
            // Derives b's assumed area itself, so it survives the layer.
            DlClause::datalog(
                atom(v("?x"), "area", v("?a")),
                vec![atom(v("?x"), "size", v("?a"))],
            ),
        ];
        let program = compile_scheduled(
            rules,
            Schedule::new(vec![Layer::new(Vec::new(), vec![vec![0, 1]])]),
        )
        .expect("compiles");
        let edb = store(&[
            ("b", "size", "\"2\""),
            ("a", "note", "\"withdrawn\""),
            ("b", "temp", "\"t\""),
        ]);
        let model = evaluate_scheduled(
            &program,
            edb,
            &Guards::default(),
            &mut Hooks,
            &EvalOptions::default(),
            None,
        )
        .expect("evaluates");
        let yes = surface("yes");
        assert!(
            has(model.facts(), "a", "small", &yes),
            "the assumption was visible"
        );
        assert!(!has(model.facts(), "a", "area", "\"1\""), "withdrawn");
        assert!(
            has(model.facts(), "b", "area", "\"2\""),
            "derived too, so kept"
        );
        assert!(
            !has(model.facts(), "a", "note", "\"withdrawn\""),
            "retract hook"
        );
        assert!(!has(model.facts(), "b", "temp", "\"t\""), "finish hook");
        let kept = model
            .derivations()
            .iter()
            .find(|d| d.fact().subject == surface("b") && d.fact().predicate == surface("area"))
            .expect("the confirming derivation is recorded");
        assert_eq!(kept.rule(), 1);
    }

    /// A malformed schedule is a typed refusal, and every rule must be scheduled once.
    #[test]
    fn a_schedule_must_partition_the_rules() {
        let rule = || {
            DlClause::datalog(
                atom(v("?x"), "q", v("?y")),
                vec![atom(v("?x"), "p", v("?y"))],
            )
        };
        for schedule in [
            Schedule::new(Vec::new()),
            Schedule::new(vec![Layer::new(vec![vec![0]], vec![vec![0]])]),
            Schedule::new(vec![Layer::new(vec![vec![0, 1]], Vec::new())]),
        ] {
            assert!(matches!(
                compile_scheduled(vec![rule()], schedule),
                Err(EvalError::MalformedSchedule { .. })
            ));
        }
        let program = compile_scheduled(
            vec![rule()],
            Schedule::new(vec![Layer::new(Vec::new(), vec![vec![0]])]),
        )
        .expect("the neighbour partition compiles");
        let other = compile_scheduled(
            vec![rule()],
            Schedule::new(vec![Layer::new(vec![vec![0]], Vec::new())]),
        )
        .expect("compiles");
        assert_ne!(
            program.contract_hash(&EvalOptions::default()),
            other.contract_hash(&EvalOptions::default()),
            "the schedule is hashed"
        );
    }

    /// SPARQL 1.2 RL stratification: a negation over a lower stratum evaluates, with the
    /// negated rule placed strictly below; a negation inside a cycle is refused naming it.
    #[test]
    fn rule_stratification_evaluates_or_names_the_cycle() {
        // R0: ?x exposedTo ?v <- ?x hasVuln ?v ; R1: ?x safe yes <- ?x type C, NOT ?x exposedTo ?v
        let exposed = DlClause::datalog(
            atom(v("?x"), "exposedTo", v("?v")),
            vec![atom(v("?x"), "hasVuln", v("?v"))],
        );
        let safe = DlClause::datalog(
            atom(v("?x"), "safe", iri("yes")),
            vec![
                atom(v("?x"), "type", iri("C")),
                ClauseAtom::negated(v("?x"), format!("{EX}exposedTo"), v("?v")),
            ],
        );
        let rules = vec![safe.clone(), exposed.clone()];
        let schedule = stratify_rules(&rules, &[false, false]).expect("stratifiable");
        assert_eq!(schedule.layers().len(), 2);
        assert_eq!(schedule.layers()[0].iterating(), [vec![1]]);
        assert_eq!(schedule.layers()[1].iterating(), [vec![0]]);
        let model = scheduled(
            rules,
            schedule,
            store(&[
                ("a", "type", &surface("C")),
                ("b", "type", &surface("C")),
                ("a", "hasVuln", &surface("v")),
            ]),
        );
        let yes = surface("yes");
        assert!(!has(model.facts(), "a", "safe", &yes));
        assert!(has(model.facts(), "b", "safe", &yes));

        // A negation that feeds itself through a positive edge is not stratifiable.
        let back = DlClause::datalog(
            atom(v("?x"), "exposedTo", iri("self")),
            vec![atom(v("?x"), "safe", iri("yes"))],
        );
        let error = stratify_rules(&[safe, exposed, back], &[false, false, false])
            .expect_err("negative cycle");
        assert_eq!(
            error,
            EvalError::NonStratifiableRules {
                rule: 0,
                depends_on: 2,
                cycle: vec![0, 2],
            }
        );
        assert!(
            error.to_string().contains("rule 0 -> rule 2 -> rule 0"),
            "{error}"
        );
    }

    /// Dependency matching is by unification: a pattern whose constant differs from the
    /// template's never depends on it, so a negation over a DIFFERENT class in the same
    /// predicate is stratifiable — where the predicate-level stratifier would refuse it.
    #[test]
    fn dependency_matching_is_finer_than_the_predicate() {
        let a = DlClause::datalog(
            atom(v("?x"), "type", iri("A")),
            vec![
                atom(v("?x"), "type", iri("B")),
                ClauseAtom::negated(v("?x"), format!("{EX}type"), iri("C")),
            ],
        );
        let c = DlClause::datalog(
            atom(v("?x"), "type", iri("C")),
            vec![atom(v("?x"), "type", iri("D"))],
        );
        let rules = vec![a, c];
        assert!(compile(rules.clone()).is_err(), "predicate-level: refused");
        let schedule = stratify_rules(&rules, &[false, false]).expect("rule-level: stratifiable");
        let model = scheduled(
            rules,
            schedule,
            store(&[
                ("x", "type", &surface("B")),
                ("y", "type", &surface("B")),
                ("y", "type", &surface("D")),
            ]),
        );
        assert!(has(model.facts(), "x", "type", &surface("A")));
        assert!(!has(model.facts(), "y", "type", &surface("A")));
        assert!(!may_generate(
            &atom(v("?x"), "p", v("?x")),
            &atom(iri("a"), "p", iri("b"))
        ));
        assert!(may_generate(
            &atom(v("?x"), "p", v("?x")),
            &atom(v("?y"), "p", iri("b"))
        ));
    }

    /// A run-once rule's dependency is closed, so it lands strictly above what it reads.
    #[test]
    fn a_run_once_rule_is_stratified_above_what_it_reads() {
        let general = DlClause::datalog(
            atom(v("?x"), "q", v("?y")),
            vec![atom(v("?x"), "p", v("?y"))],
        );
        let once = DlClause::datalog(
            atom(v("?x"), "r", v("?y")),
            vec![atom(v("?x"), "q", v("?y"))],
        );
        let schedule = stratify_rules(&[once, general], &[true, false]).expect("stratifiable");
        assert_eq!(
            schedule.layers(),
            [
                Layer::new(Vec::new(), vec![vec![1]]),
                Layer::new(vec![vec![0]], Vec::new())
            ]
        );
        let unique: BTreeSet<usize> = schedule.rule_indices().collect();
        assert_eq!(unique.len(), 2);
    }

    /// A caller-built dependency graph is stratified by the same core: an open cycle is
    /// one stratum, a closed edge lands its rule strictly above, a closed edge inside a
    /// cycle is refused naming it, and a repeated edge keeps "closed overrides open".
    #[test]
    fn a_caller_built_dependency_graph_is_stratified_by_the_same_core() {
        // 0 <-> 1 open, 2 -> 1 closed: two strata.
        let mut graph = DependencyGraph::new(3);
        graph.depend(0, 1, false);
        graph.depend(1, 0, false);
        graph.depend(2, 1, false);
        graph.depend(2, 1, true);
        assert_eq!(
            graph.edges().collect::<Vec<_>>(),
            [(0, 1, false), (1, 0, false), (2, 1, true)]
        );
        let schedule = stratify_dependency_graph(&graph, &[false, false, true]).expect("ok");
        assert_eq!(
            schedule.layers(),
            [
                Layer::new(Vec::new(), vec![vec![0], vec![1]]),
                Layer::new(vec![vec![2]], Vec::new())
            ]
        );
        // The same closed edge inside a cycle is refused by name.
        graph.depend(1, 2, false);
        assert_eq!(
            stratify_dependency_graph(&graph, &[false, false, true]),
            Err(EvalError::NonStratifiableRules {
                rule: 2,
                depends_on: 1,
                cycle: vec![2, 1],
            })
        );
    }

    /// The consumers that read only clause text refuse a guarded clause by name.
    #[test]
    fn text_only_consumers_refuse_guarded_clauses() {
        let guarded = DlClause::datalog(
            atom(v("?x"), "q", v("?y")),
            vec![atom(v("?x"), "p", v("?y"))],
        )
        .with_guards(vec![Guard::filter("gt:1", vec!["?y".to_owned()])]);
        assert!(matches!(
            crate::chase::chase(std::slice::from_ref(&guarded), RelationStore::new()),
            Err(crate::chase::ChaseError::GuardedClause { clause: 0 })
        ));
        let refusal = crate::resolve_fol::solve_datalog_goal(
            std::slice::from_ref(&guarded),
            &atom(v("?x"), "q", v("?y")),
            &crate::resolve_fol::FolBudget { max_steps: 10 },
        )
        .expect_err("refused");
        assert!(refusal.is_guarded());
        let mut arena = crate::proof::ProofArena::new();
        let premise = arena.axiom(Fact {
            subject: surface("a"),
            predicate: surface("p"),
            object: "\"2\"".to_owned(),
            graph: String::new(),
        });
        let step = arena.by_rule(
            Fact {
                subject: surface("a"),
                predicate: surface("q"),
                object: "\"2\"".to_owned(),
                graph: String::new(),
            },
            0,
            &[premise],
        );
        let edb = store(&[("a", "p", "\"2\"")]);
        let rules = [guarded];
        let ctx = crate::proof::ProofContext::new(&rules, &edb, &edb);
        assert_eq!(
            arena.check(step, &ctx),
            Err(crate::proof::ProofError::GuardedRule { rule: 0 })
        );
    }
}
