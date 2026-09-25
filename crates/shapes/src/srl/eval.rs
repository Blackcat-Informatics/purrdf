// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Evaluating a rule set: lower, schedule, run on `purrdf-datalog`, read the result back.
//!
//! # Terms in the store
//!
//! The datalog store keys every term by a lexical surface. Every term that enters it —
//! a base triple's, a data block's, a guard's output — passes through one [`Codec`],
//! which renders it with [`Term`]'s own N-Triples-shaped `Display` and remembers the
//! term under that surface, so every surface the store can hand back decodes to exactly
//! the term that produced it. Base triples live in the store's default graph, which is
//! the EVALUATION graph ("The evaluation graph is the union of the base graph and the
//! inference graph"); a base-graph mirror is added only when a rule matches the base
//! graph alone ([`super::lower::BASE_GRAPH`]).
//!
//! # The graph a SHACL rule reads
//!
//! A SHACL rule is executed by the SHACL machinery over a frozen dataset of the
//! evaluation graph as it stands — "During execution of a rule, the evaluation graph is
//! used as the active query graph" — rebuilt only when the model has changed since the
//! last build, so every rule of one concurrently executed group reads one dataset.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use ::purrdf::{FastMap, FastSet, RdfDataset, RdfDatasetBuilder};
use purrdf_datalog::guard::{GuardCall, GuardEvaluator, GuardSite};
use purrdf_datalog::schedule::{self, Layer, LayerHooks, Schedule};
use purrdf_datalog::seminaive::{BudgetResource, EvalError, EvalOptions};
use purrdf_datalog::store::{Fact, RelationStore};

use super::ir::{IrRule, IrRuleBody, RuleSet, Scheduling};
use super::lower::{self, BASE_GRAPH, GuardImpl, LoweredPosition, LoweredTriple};
use crate::data::ShaclData;
use crate::model::{rdf, sh};
use crate::rules;
use crate::shapes::Shapes;
use crate::term::{NamedNode, Term, Triple};

/// The explanation of one inferred triple: the rule that derived it and the facts its
/// body matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    /// The rule that derived the triple.
    rule: Term,
    /// The facts the rule's triple patterns matched, in authored body order. A SHACL rule
    /// is evaluated as one producer over the whole evaluation graph, so it has none: its
    /// explanation is the rule alone.
    premises: Vec<[Term; 3]>,
}

impl Explanation {
    /// The rule that derived the triple.
    #[must_use]
    pub fn rule(&self) -> &Term {
        &self.rule
    }

    /// The facts the rule's body matched, in authored body order.
    #[must_use]
    pub fn premises(&self) -> &[[Term; 3]] {
        &self.premises
    }
}

/// A completed rule-set evaluation.
#[derive(Debug, Clone)]
pub struct Inference {
    /// The base graph plus every inferred triple.
    dataset: Arc<RdfDataset>,
    /// The inferred triples, in canonical order.
    inferred: Vec<[Term; 3]>,
    /// The derivation of every inferred triple a rule derived.
    explanations: FastMap<[Term; 3], Explanation>,
    /// The identity of the evaluated program and schedule.
    contract: String,
}

impl Inference {
    /// The base graph plus every inferred triple.
    #[must_use]
    pub fn dataset(&self) -> &Arc<RdfDataset> {
        &self.dataset
    }

    /// The inferred triples — "The new triples that are produced by a rules engine are
    /// called the inferred triples" — in canonical order.
    #[must_use]
    pub fn inferred(&self) -> &[[Term; 3]] {
        &self.inferred
    }

    /// Why `triple` was inferred: the rule that derived it and the facts its body
    /// matched — the datalog derivation of the triple, with every surface decoded — or
    /// `None` when `triple` is not an inferred triple a rule derived (a base triple, or a
    /// data-block triple).
    #[must_use]
    pub fn explain_conclusion(&self, triple: &[Term; 3]) -> Option<&Explanation> {
        self.explanations.get(triple)
    }

    /// The content identity of the evaluated clause program and its schedule
    /// (`purrdf_datalog::cache::scheduled_contract_hash`), as lowercase hex.
    #[must_use]
    pub fn contract_hash(&self) -> &str {
        &self.contract
    }
}

/// Evaluate `set` over `data`. `shapes` supplies the shapes graph a SHACL rule reads and
/// the expected derived triples; `options` supplies the term-generating round limit and
/// whether to add `sh:sourceRule` reifiers (its processor and rule-set selections concern
/// only how a SHACL shapes graph is lowered to `set`, and are not read here).
///
/// # Errors
///
/// An ill-formed element rule, a rule set that is not stratifiable, a guard or producer
/// that fails, or a fixed ceiling passed — each named by the rule it concerns.
pub fn evaluate(
    set: &RuleSet<'_>,
    data: &ShaclData,
    shapes: &Shapes,
    options: &rules::RuleOptions,
) -> Result<Inference, String> {
    let source_rules = options.source_rules();
    let governors = EvalOptions::default()
        .with_max_term_generating_rounds(options.max_term_generating_rounds());
    for rule in &set.rules {
        if let IrRuleBody::Elements(element_rule) = &rule.body {
            element_rule
                .check_well_formed()
                .map_err(|e| format!("rule {} is not well formed: {e}", rule.id))?;
        }
    }
    let empty = empty_data()?;
    let now = if lower::uses_now(set) {
        Some(capture_now(&empty)?)
    } else {
        None
    };
    let lowered = lower::lower(set, now.as_ref());
    let schedule = match set.scheduling {
        Scheduling::Declared => declared_schedule(set),
        Scheduling::Stratified => {
            let run_once: Vec<bool> = set.rules.iter().map(IrRule::is_run_once).collect();
            schedule::stratify_rules(&lowered.clauses, &run_once).map_err(|e| describe(&e, set))?
        }
    };
    let expectations: Vec<Vec<String>> = schedule
        .layers()
        .iter()
        .map(|layer| {
            let mut predicates: Vec<String> = layer
                .once()
                .iter()
                .chain(layer.iterating())
                .flatten()
                .flat_map(|&index| set.rules[index].expected_predicates.iter())
                .map(|p| p.as_str().to_owned())
                .collect();
            predicates.sort();
            predicates.dedup();
            predicates
        })
        .collect();
    let program =
        schedule::compile_scheduled(lowered.clauses, schedule).map_err(|e| describe(&e, set))?;

    let base = data.core_arc();
    let shapes_graph_iri = data
        .shapes_graph_iri()
        .map(ToOwned::to_owned)
        .or_else(|| shapes.shapes_graph.clone())
        .filter(|_| shapes.shapes_dataset.quad_count() > 0);
    let mut engine = Engine {
        codec: Codec::default(),
        guards: lowered.guards,
        base: Arc::clone(&base),
        originals: FastSet::default(),
        shapes,
        shapes_graph_iri,
        empty,
        view: RefCell::new(None),
        generation: Cell::new(0),
        mints: Cell::new(0),
        expectations,
    };
    // Every constant a clause mentions enters the store through its head or a probe, so
    // each is registered with the codec first.
    for rule in &set.rules {
        if let IrRuleBody::Elements(element_rule) = &rule.body {
            register_constants(&engine.codec, element_rule);
        }
    }
    let mut edb = RelationStore::new();
    for triple in rules::base_triples(&base) {
        let [s, p, o] = triple.each_ref().map(|t| engine.codec.surface(t));
        edb.insert(&s, &p, &o, RelationStore::DEFAULT_GRAPH);
        if lowered.uses_base {
            edb.insert(&s, &p, &o, BASE_GRAPH);
        }
        engine.originals.insert(triple);
    }
    for triple in &set.data {
        let [s, p, o] = triple.each_ref().map(|t| engine.codec.surface(t));
        edb.insert(&s, &p, &o, RelationStore::DEFAULT_GRAPH);
    }

    let evaluation = schedule::evaluate_scheduled(
        &program,
        edb,
        &engine,
        &mut Hooks { engine: &engine },
        &governors,
        None,
    )
    .map_err(|e| describe(&e, set))?;

    // The inference graph: every default-graph fact that is not a base triple.
    let facts = engine.model_facts(evaluation.facts())?;
    let mut inferred: Vec<[Term; 3]> = facts
        .iter()
        .filter(|triple| !engine.originals.contains(*triple))
        .cloned()
        .collect();
    inferred.sort_by_cached_key(|triple| triple.each_ref().map(ToString::to_string));

    let mut explanations: FastMap<[Term; 3], Explanation> = FastMap::default();
    for derivation in evaluation.derivations() {
        if derivation.fact().graph != RelationStore::DEFAULT_GRAPH {
            continue;
        }
        let triple = engine.decode_fact(derivation.fact())?;
        let premises = derivation
            .sources()
            .iter()
            .map(|fact| engine.decode_fact(fact))
            .collect::<Result<Vec<_>, _>>()?;
        explanations.entry(triple).or_insert_with(|| Explanation {
            rule: set.rules[derivation.rule()].id.clone(),
            premises,
        });
    }

    let mut all: Vec<&[Term; 3]> = facts.iter().collect();
    let mut sources: Vec<[Term; 3]> = Vec::new();
    if source_rules {
        for (index, triple) in inferred.iter().enumerate() {
            let Some(explanation) = explanations.get(triple) else {
                continue;
            };
            let reifier = Term::blank(format!("s-x{index}_rule"));
            let [s, p, o] = triple.clone();
            let Term::NamedNode(predicate) = p else {
                continue;
            };
            sources.push([
                reifier.clone(),
                Term::NamedNode(NamedNode::from(rdf::REIFIES)),
                Term::Triple(Box::new(Triple {
                    subject: s,
                    predicate,
                    object: o,
                })),
            ]);
            sources.push([
                reifier,
                Term::NamedNode(NamedNode::from(sh::SOURCE_RULE)),
                explanation.rule.clone(),
            ]);
        }
        all.extend(sources.iter());
    }
    let reifiers = rules::reifier_subjects(all.iter().copied());
    let mut builder = RdfDatasetBuilder::new();
    rules::push_projection(&mut builder, base.as_ref());
    for triple in inferred.iter().chain(&sources) {
        rules::push_fact(&mut builder, triple, &reifiers)?;
    }
    let dataset = builder.freeze().map_err(|e| e.to_string())?;
    Ok(Inference {
        dataset,
        inferred,
        explanations,
        contract: program.contract_hash(&governors).to_hex(),
    })
}

/// The schedule SHACL 1.2 Inference Rules declares: one layer per distinct `sh:layer`
/// value, ascending; in each, one run-once group per distinct `sh:order` value of its
/// run-once rules, ascending, then one iterating group per distinct `sh:order` value of
/// the rest. A group's rules are listed in rule-node order.
fn declared_schedule(set: &RuleSet<'_>) -> Schedule {
    let mut layers: Vec<(f64, Vec<usize>)> = Vec::new();
    for (index, rule) in set.rules.iter().enumerate() {
        let layer = rule.schedule.layer.value();
        match layers
            .iter_mut()
            .find(|(value, _)| value.total_cmp(&layer).is_eq())
        {
            Some((_, members)) => members.push(index),
            None => layers.push((layer, vec![index])),
        }
    }
    layers.sort_by(|(a, _), (b, _)| a.total_cmp(b));
    let groups = |members: &[usize], run_once: bool| -> Vec<Vec<usize>> {
        let mut groups: Vec<(f64, Vec<usize>)> = Vec::new();
        for &index in members {
            let rule = &set.rules[index];
            if rule.schedule.run_once != run_once {
                continue;
            }
            let order = rule.schedule.order.value();
            match groups
                .iter_mut()
                .find(|(value, _)| value.total_cmp(&order).is_eq())
            {
                Some((_, group)) => group.push(index),
                None => groups.push((order, vec![index])),
            }
        }
        groups.sort_by(|(a, _), (b, _)| a.total_cmp(b));
        groups
            .into_iter()
            .map(|(_, mut group)| {
                group.sort_by_cached_key(|&index| set.rules[index].id.to_string());
                group
            })
            .collect()
    };
    Schedule::new(
        layers
            .iter()
            .map(|(_, members)| Layer::new(groups(members, true), groups(members, false)))
            .collect(),
    )
}

/// Register every constant term of `rule` — in its head, its patterns and its negation
/// elements, triple terms included — with `codec`.
fn register_constants(codec: &Codec, rule: &super::ir::ElementRule) {
    fn position(codec: &Codec, term: &super::ir::PatternTerm) {
        match term {
            super::ir::PatternTerm::Term(term) => {
                codec.surface(term);
            }
            super::ir::PatternTerm::Triple(inner) => pattern(codec, inner),
            super::ir::PatternTerm::Variable(_) | super::ir::PatternTerm::BlankNode(_) => {}
        }
    }
    fn pattern(codec: &Codec, triple: &super::ir::TriplePattern) {
        for term in triple.positions() {
            position(codec, term);
        }
    }
    fn elements(codec: &Codec, sequence: &[super::ir::Element]) {
        for element in sequence {
            match element {
                super::ir::Element::Pattern(triple) => pattern(codec, triple),
                super::ir::Element::Negation {
                    elements: inner, ..
                } => elements(codec, inner),
                super::ir::Element::Filter(_) | super::ir::Element::Assign { .. } => {}
            }
        }
    }
    for template in &rule.head {
        pattern(codec, template);
    }
    elements(codec, &rule.body);
}

/// An evaluation error, with rule indices named by the rules they index.
fn describe(error: &EvalError, set: &RuleSet<'_>) -> String {
    let name = |index: usize| {
        set.rules.get(index).map_or_else(
            || format!("rule {index}"),
            |rule| format!("rule {}", rule.id),
        )
    };
    match error {
        EvalError::NonStratifiableRules {
            rule,
            depends_on,
            cycle,
        } => {
            let path: Vec<String> = cycle.iter().map(|&index| name(index)).collect();
            format!(
                "the rule set is not stratifiable: {} depends on {} through a negation or a \
                 run-once rule, inside the cycle {} -> {} (SPARQL 1.2 RL: \"there is no \
                 recursive dependency involving a closed dependency\")",
                name(*rule),
                name(*depends_on),
                path.join(" -> "),
                name(*rule)
            )
        }
        EvalError::Guard {
            rule,
            guard,
            message,
            ..
        } => format!(
            "{} failed during execution ({guard}): {message}",
            name(*rule)
        ),
        EvalError::BudgetExhausted {
            resource: BudgetResource::TermGeneratingRounds,
            report,
        } => format!(
            "SHACL rules did not complete: {} rounds inferred a term the evaluation graph \
             did not hold, past the limit of {} such rounds (SHACL 1.2 Inference Rules: \
             \"Rule engines MAY also report a failure after a pre-configured maximum \
             iteration count has been exceeded\"); if the rule set terminates, raise the \
             limit with RuleOptions::with_max_term_generating_rounds",
            report.term_generating_rounds(),
            report.term_generating_round_limit()
        ),
        other => format!("SHACL rules did not complete: {other}"),
    }
}

/// An empty evaluation dataset, the one a pure expression is evaluated over.
fn empty_data() -> Result<ShaclData, String> {
    let empty = RdfDatasetBuilder::new()
        .freeze()
        .map_err(|e| e.to_string())?;
    Ok(ShaclData::new(Arc::clone(&empty), empty, None))
}

/// The one point in time `NOW()` denotes throughout this evaluation.
fn capture_now(empty: &ShaclData) -> Result<purrdf_sparql_algebra::Literal, String> {
    let now = crate::sparql::eval_scalar_query_view(
        empty.sparql_view(),
        "SELECT (NOW() AS ?result) WHERE {}",
        &[],
    )?;
    let Some(Term::Literal(now)) = now else {
        return Err("NOW() did not evaluate to a literal".to_owned());
    };
    let datatype = purrdf_sparql_algebra::NamedNode::new(now.datatype_str())
        .map_err(|e| format!("NOW() returned an invalid datatype IRI: {e}"))?;
    Ok(purrdf_sparql_algebra::Literal::new_typed(
        now.value(),
        datatype,
    ))
}

/// Surface ↔ term, for every term that enters the store. See the [module docs](self).
#[derive(Debug, Default)]
struct Codec {
    /// Every term, by its surface.
    terms: RefCell<FastMap<String, Term>>,
}

impl Codec {
    /// The surface of `term`, remembering the term under it.
    fn surface(&self, term: &Term) -> String {
        let surface = term.to_string();
        self.terms
            .borrow_mut()
            .entry(surface.clone())
            .or_insert_with(|| term.clone());
        surface
    }

    /// The term a store surface denotes.
    fn term(&self, surface: &str) -> Result<Term, String> {
        self.terms.borrow().get(surface).cloned().ok_or_else(|| {
            format!("internal error: the rules engine's store holds an unknown term {surface}")
        })
    }
}

/// An evaluation-graph view with the `(generation, row count)` it was built at.
type CachedView = ((u64, usize), Rc<ShaclData>);

/// The evaluation state the guard evaluator and the layer hooks share.
struct Engine<'r, 'a> {
    /// Surface ↔ term.
    codec: Codec,
    /// How every guard is evaluated.
    guards: BTreeMap<(usize, GuardSite), GuardImpl<'r, 'a>>,
    /// The base graph.
    base: Arc<RdfDataset>,
    /// The base graph's triples.
    originals: FastSet<[Term; 3]>,
    /// The shapes.
    shapes: &'a Shapes,
    /// The shapes graph IRI a SPARQL rule sees as `$shapesGraph`.
    shapes_graph_iri: Option<String>,
    /// The empty dataset a pure expression is evaluated over.
    empty: ShaclData,
    /// The last evaluation-graph view built, by `(generation, row count)`.
    view: RefCell<Option<CachedView>>,
    /// Bumped whenever the store is rebuilt by a retraction.
    generation: Cell<u64>,
    /// Executions numbered so far, for blank-node minting.
    mints: Cell<u64>,
    /// The expected predicates of each layer.
    expectations: Vec<Vec<String>>,
}

impl Engine<'_, '_> {
    /// The next execution number.
    fn mint(&self) -> u64 {
        let next = self.mints.get() + 1;
        self.mints.set(next);
        next
    }

    /// A store fact as a triple.
    fn decode_fact(&self, fact: &Fact) -> Result<[Term; 3], String> {
        Ok([
            self.codec.term(&fact.subject)?,
            self.codec.term(&fact.predicate)?,
            self.codec.term(&fact.object)?,
        ])
    }

    /// The evaluation graph's triples, in lexical surface order.
    fn model_facts(&self, model: &RelationStore) -> Result<Vec<[Term; 3]>, String> {
        model
            .facts_sorted()
            .iter()
            .filter(|fact| fact.graph == RelationStore::DEFAULT_GRAPH)
            .map(|fact| self.decode_fact(fact))
            .collect()
    }

    /// A triple as a default-graph store fact.
    fn fact(&self, triple: &[Term; 3]) -> Fact {
        let [subject, predicate, object] = triple.each_ref().map(|t| self.codec.surface(t));
        Fact {
            subject,
            predicate,
            object,
            graph: RelationStore::DEFAULT_GRAPH.to_owned(),
        }
    }

    /// The evaluation graph as SHACL data, rebuilt only when the model changed.
    fn view(&self, model: &RelationStore) -> Result<Rc<ShaclData>, String> {
        let key = (self.generation.get(), model.row_count());
        if let Some((cached, view)) = self.view.borrow().as_ref()
            && *cached == key
        {
            return Ok(Rc::clone(view));
        }
        let facts = self.model_facts(model)?;
        let reifiers = rules::reifier_subjects(facts.iter());
        let mut builder = RdfDatasetBuilder::new();
        rules::push_projection(&mut builder, self.base.as_ref());
        for triple in facts.iter().filter(|t| !self.originals.contains(*t)) {
            rules::push_fact(&mut builder, triple, &reifiers)?;
        }
        let core = builder.freeze().map_err(|e| e.to_string())?;
        let sparql = rules::build_round_base(&core, self.shapes, self.shapes_graph_iri.as_deref())?;
        let view = Rc::new(ShaclData::new(core, sparql, self.shapes_graph_iri.clone()));
        *self.view.borrow_mut() = Some((key, Rc::clone(&view)));
        Ok(view)
    }

    /// The non-base triples to delete with `doomed`'s reifiers: every inferred
    /// `r rdf:reifies <<t>>` of a doomed `t`, and every inferred triple about a reifier
    /// left reifying nothing.
    fn reifiers_of(&self, facts: &[[Term; 3]], doomed: &FastSet<[Term; 3]>) -> Vec<[Term; 3]> {
        let reifies = Term::NamedNode(NamedNode::from(rdf::REIFIES));
        let reifies_doomed = |fact: &[Term; 3]| -> bool {
            let [_, predicate, object] = fact;
            if *predicate != reifies || self.originals.contains(fact) {
                return false;
            }
            let Term::Triple(reified) = object else {
                return false;
            };
            doomed.contains(&[
                reified.subject.clone(),
                Term::NamedNode(reified.predicate.clone()),
                reified.object.clone(),
            ])
        };
        let mut out: Vec<[Term; 3]> = facts
            .iter()
            .filter(|f| reifies_doomed(f))
            .cloned()
            .collect();
        let mut orphaned: FastSet<Term> = out.iter().map(|[r, _, _]| r.clone()).collect();
        // A reifier that still declares another reification is still a reifier.
        for fact in facts {
            if fact[1] == reifies && !reifies_doomed(fact) {
                orphaned.remove(&fact[0]);
            }
        }
        out.extend(
            facts
                .iter()
                .filter(|fact| orphaned.contains(&fact[0]) && !self.originals.contains(*fact))
                .filter(|fact| !reifies_doomed(fact))
                .cloned(),
        );
        out
    }
}

impl GuardEvaluator for Engine<'_, '_> {
    fn evaluate(&self, call: &GuardCall<'_>) -> Result<Vec<Vec<String>>, String> {
        let implementation = self
            .guards
            .get(&(call.rule, call.site))
            .ok_or_else(|| format!("internal error: no implementation for {}", call.site))?;
        match implementation {
            GuardImpl::Producer(producer) => {
                let view = self.view(call.model)?;
                let triples = rules::execute_rule(
                    &view,
                    producer.rule,
                    &producer.shapes,
                    self.shapes_graph_iri.as_deref(),
                    &mut || self.mint(),
                )?;
                Ok(triples
                    .iter()
                    .map(|triple| triple.iter().map(|t| self.codec.surface(t)).collect())
                    .collect())
            }
            GuardImpl::Filter { query, variables } => {
                let args = self.arguments(variables, call.inputs)?;
                let value =
                    crate::sparql::eval_scalar_query_view(self.empty.sparql_view(), query, &args)?;
                Ok(match value {
                    Some(Term::Literal(l)) if l.value() == "true" => vec![Vec::new()],
                    _ => Vec::new(),
                })
            }
            GuardImpl::Assign { query, variables } => {
                let args = self.arguments(variables, call.inputs)?;
                let value =
                    crate::sparql::eval_scalar_query_view(self.empty.sparql_view(), query, &args)?;
                // "If evaluating the expression in an assignment causes an error, then the
                // current solution mapping is rejected by the assignment."
                Ok(value.map_or_else(Vec::new, |term| vec![vec![self.codec.surface(&term)]]))
            }
            GuardImpl::FreshBlank => {
                let blank = Term::blank(format!("r-x{}_bnode", self.mint()));
                Ok(vec![vec![self.codec.surface(&blank)]])
            }
            GuardImpl::MatchTriple {
                pattern,
                inputs,
                outputs,
            } => {
                let term = self.codec.term(call.inputs[0])?;
                let mut known: BTreeMap<&str, Term> = BTreeMap::new();
                for (name, surface) in inputs.iter().zip(&call.inputs[1..]) {
                    known.insert(name.as_str(), self.codec.term(surface)?);
                }
                if !match_triple(pattern, &term, &mut known) {
                    return Ok(Vec::new());
                }
                Ok(vec![
                    outputs
                        .iter()
                        .map(|name| self.codec.surface(&known[name.as_str()]))
                        .collect(),
                ])
            }
            GuardImpl::BuildTriple { template, inputs } => {
                let mut known: BTreeMap<&str, Term> = BTreeMap::new();
                for (name, surface) in inputs.iter().zip(call.inputs) {
                    known.insert(name.as_str(), self.codec.term(surface)?);
                }
                Ok(build_triple(template, &known)
                    .map_or_else(Vec::new, |term| vec![vec![self.codec.surface(&term)]]))
            }
        }
    }
}

impl Engine<'_, '_> {
    /// The SPARQL pre-bindings of a scalar query: canonical names with decoded values.
    fn arguments(
        &self,
        variables: &[String],
        inputs: &[&str],
    ) -> Result<Vec<(String, Term)>, String> {
        variables
            .iter()
            .zip(inputs)
            .map(|(name, surface)| Ok((name.clone(), self.codec.term(surface)?)))
            .collect()
    }
}

/// Match `term` against the triple term pattern, extending `known`.
fn match_triple<'n>(
    pattern: &'n LoweredTriple,
    term: &Term,
    known: &mut BTreeMap<&'n str, Term>,
) -> bool {
    let Term::Triple(triple) = term else {
        return false;
    };
    let parts = [
        triple.subject.clone(),
        Term::NamedNode(triple.predicate.clone()),
        triple.object.clone(),
    ];
    pattern
        .0
        .iter()
        .zip(parts)
        .all(|(position, value)| match position {
            LoweredPosition::Constant(constant) => *constant == value,
            LoweredPosition::Variable(name) => match known.get(name.as_str()) {
                Some(bound) => *bound == value,
                None => {
                    known.insert(name.as_str(), value);
                    true
                }
            },
            LoweredPosition::Triple(inner) => match_triple(inner, &value, known),
        })
}

/// Build the triple term `template` denotes under `known`; `None` when it would be
/// ill-formed (a non-IRI predicate, a literal subject).
fn build_triple(template: &LoweredTriple, known: &BTreeMap<&str, Term>) -> Option<Term> {
    let value = |position: &LoweredPosition| -> Option<Term> {
        match position {
            LoweredPosition::Constant(term) => Some(term.clone()),
            LoweredPosition::Variable(name) => known.get(name.as_str()).cloned(),
            LoweredPosition::Triple(inner) => build_triple(inner, known),
        }
    };
    let subject = value(&template.0[0])?;
    let Term::NamedNode(predicate) = value(&template.0[1])? else {
        return None;
    };
    if matches!(subject, Term::Literal(_)) {
        return None;
    }
    Some(Term::Triple(Box::new(Triple {
        subject,
        predicate,
        object: value(&template.0[2])?,
    })))
}

/// The SHACL layer-boundary actions.
struct Hooks<'e, 'r, 'a> {
    /// The shared evaluation state.
    engine: &'e Engine<'r, 'a>,
}

impl LayerHooks for Hooks<'_, '_, '_> {
    /// "Compute the expected derived triples for all rules in the layer".
    fn assume(&mut self, layer: usize, model: &RelationStore) -> Result<Vec<Fact>, String> {
        let expected = &self.engine.expectations[layer];
        if expected.is_empty() {
            return Ok(Vec::new());
        }
        let view = self.engine.view(model)?;
        let predicates: FastSet<&str> = expected.iter().map(String::as_str).collect();
        let triples = rules::expected_derived_triples(&view, self.engine.shapes, &predicates)?;
        Ok(triples
            .iter()
            .map(|triple| self.engine.fact(triple))
            .collect())
    }

    /// "Delete the derived triples (except those that were also inferred by rules) and
    /// their reifiers". A reifier of a deleted triple is the subject `r` of an INFERRED
    /// `r rdf:reifies <<( s p o )>>` whose triple term is that triple; the declaration
    /// goes, and a reifier left reifying nothing is no reifier, so every inferred triple
    /// about it goes too. Nothing the base graph asserts is touched.
    fn retract(
        &mut self,
        _layer: usize,
        model: &RelationStore,
        withdrawn: &[Fact],
    ) -> Result<Vec<Fact>, String> {
        self.engine.generation.set(self.engine.generation.get() + 1);
        if withdrawn.is_empty() {
            return Ok(Vec::new());
        }
        let doomed: FastSet<[Term; 3]> = withdrawn
            .iter()
            .map(|fact| self.engine.decode_fact(fact))
            .collect::<Result<_, _>>()?;
        let facts = self.engine.model_facts(model)?;
        Ok(self
            .engine
            .reifiers_of(&facts, &doomed)
            .iter()
            .map(|triple| self.engine.fact(triple))
            .collect())
    }

    /// "Delete the temporary triples and their reifiers": "A temporary triple is an
    /// inferred triple for which the inference graph contains a reifier with the value
    /// sh:tempTriple true."
    fn finish(&mut self, model: &RelationStore) -> Result<Vec<Fact>, String> {
        self.engine.generation.set(self.engine.generation.get() + 1);
        let facts = self.engine.model_facts(model)?;
        let temp = Term::NamedNode(NamedNode::from(sh::TEMP_TRIPLE));
        let reifies = Term::NamedNode(NamedNode::from(rdf::REIFIES));
        let marked: FastSet<Term> = facts
            .iter()
            .filter(|fact| {
                fact[1] == temp
                    && !self.engine.originals.contains(*fact)
                    && crate::shapes::parser_boolean(&fact[2]) == Some(true)
            })
            .map(|[reifier, _, _]| reifier.clone())
            .collect();
        if marked.is_empty() {
            return Ok(Vec::new());
        }
        let mut doomed: Vec<[Term; 3]> = Vec::new();
        for fact in &facts {
            if self.engine.originals.contains(fact) || !marked.contains(&fact[0]) {
                continue;
            }
            if fact[1] == reifies
                && let Term::Triple(reified) = &fact[2]
            {
                let triple = [
                    reified.subject.clone(),
                    Term::NamedNode(reified.predicate.clone()),
                    reified.object.clone(),
                ];
                if !self.engine.originals.contains(&triple) {
                    doomed.push(triple);
                }
            }
            doomed.push(fact.clone());
        }
        Ok(doomed
            .iter()
            .map(|triple| self.engine.fact(triple))
            .collect())
    }
}
