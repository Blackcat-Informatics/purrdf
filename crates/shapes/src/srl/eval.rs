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
//! A SHACL rule is executed by the SHACL machinery over a frozen view of the evaluation
//! graph as it stands — "During execution of a rule, the evaluation graph is used as the
//! active query graph" — built anew only when the model has changed since the last one,
//! so every rule of one concurrently executed group reads one view.
//!
//! Within a layer the evaluation graph only grows, so a new view is the last one plus
//! the triples committed since: a delta over the last COMPACTED view, frozen on its own
//! and read together with it. The view is compacted — rebuilt whole — when a retraction
//! rebuilt the store, when a new triple is a reifier declaration or mentions a blank node
//! (a delta cannot move an earlier triple into a reifier's annotations, and a compacted
//! view is where a blank node keeps its label), and by the rent-or-buy rule: once the
//! deltas frozen since the last compaction have cost, in term bytes, as much as the
//! store holds. Every view reads the same triples a rebuilt one would; only what an
//! iteration pays for it changes — the size of what the iteration added, not of the
//! graph.
//!
//! # Re-executing a shape rule
//!
//! A shape rule — one linked to a shape, executed once per focus node — is re-executed
//! each iteration only for the focus nodes whose answer can have changed since it was
//! last executed for them: the new ones, and those a triple the iteration before added
//! is ABOUT, as the rule's read set ([`super::reads`]) says. A focus node whose inputs
//! gained nothing would yield exactly the triples it yielded before, all of them already
//! in the graph, because the answer is a function of the focus node, the rule's constants
//! and the graph. A gained triple the rule reads for every focus node — one a join step
//! away from `$this`, a variable-predicate or zero-length pattern — re-executes every
//! focus node, and a rule whose answer is not repeatable (fresh blank nodes, `RAND()`,
//! `NOW()`, an order-dependent selection, a function the analysis does not read) is
//! executed for every focus node every iteration, exactly as the specification reads.
//!
//! When the rule has no condition and its shape's targets only grow with the graph
//! (`sh:targetClass`, `sh:targetSubjectsOf`, `sh:targetObjectsOf`, `sh:targetNode`, an
//! implicit class), the new focus nodes themselves are found from the gained triples
//! rather than by resolving the targets again. Every focus node draws its execution
//! number (the label prefix of the blank nodes it mints) whether it is re-executed or
//! not, so the labels of every rule's blank nodes are the same either way.
//!
//! The re-execution is proven against the specification's reading — every focus node,
//! every iteration, over a view rebuilt whole — by comparing the two on the rules corpus
//! and on randomised rule sets: the same inferred triples, blank-node labels included,
//! the same proof, or the same refusal. What changes is the cost. A shape rule minting
//! one focus node per iteration costs one execution per iteration rather than one per
//! focus node so far, so a divergent one is refused by a term limit (or, when its
//! terms grow longer every iteration, by the term-arena ceiling) within seconds, rather
//! than running into the join-step limit — or, with that raised, for minutes.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::Arc;

use ::purrdf::{FastMap, FastSet, MutableDataset, RdfDataset, RdfDatasetBuilder};
use purrdf_datalog::guard::{GuardCall, GuardEvaluator, GuardSite};
use purrdf_datalog::schedule::{self, Layer, LayerHooks, Schedule};
use purrdf_datalog::seminaive::{BudgetResource, EvalError};
use purrdf_datalog::store::{Fact, RelationStore};

use super::ir::{IrRuleBody, RuleSet, Scheduling};
use super::lower::{self, BASE_GRAPH, GuardImpl, LoweredPosition, LoweredTriple};
use super::reads::{ReadPattern, RuleReads};
use crate::data::ShaclData;
use crate::data_view::ShaclDatasetView;
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
    /// evaluated as one producer over the whole evaluation graph has none: its
    /// explanation is the rule alone. A global SPARQL rule read as rule elements
    /// ([`super::sparql_rule`]) has its triple patterns' matches.
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

    /// The inference graph alone — the inferred triples, WITHOUT the base graph
    /// [`Self::dataset`] carries — as a dataset. A `r rdf:reifies <<( s p o )>>`
    /// triple is a reifier declaration and a triple whose subject is such a reifier
    /// its annotation, exactly as a parsed graph classifies them.
    ///
    /// # Errors
    ///
    /// An internal-invariant breach: an inferred triple with a non-IRI predicate,
    /// which the evaluation refuses before it returns.
    pub fn inferred_dataset(&self) -> Result<Arc<RdfDataset>, String> {
        let reifiers = rules::reifier_subjects(self.inferred.iter());
        let mut builder = RdfDatasetBuilder::new();
        for triple in &self.inferred {
            rules::push_fact(&mut builder, triple, &reifiers)?;
        }
        builder.freeze().map_err(|e| e.to_string())
    }

    /// The inferred triples as N-Triples 1.2, one `S P O .` line each, in the canonical
    /// order of [`Self::inferred`]. Blank nodes keep the labels the evaluation gave
    /// them, so the lines match [`Self::proof_text`] term for term.
    #[must_use]
    pub fn inferred_ntriples(&self) -> String {
        let mut out = String::new();
        for [s, p, o] in &self.inferred {
            let _ = writeln!(out, "{s} {p} {o} .");
        }
        out
    }

    /// The proof of every inferred triple, as deterministic line-oriented text.
    ///
    /// One block per inferred triple, in the canonical order of [`Self::inferred`]:
    ///
    /// ```text
    /// derived S P O .
    ///   rule R
    ///   premise S P O .
    /// ```
    ///
    /// `derived` names the conclusion in N-Triples 1.2 term syntax. `rule` names the rule
    /// that derived it ([`Explanation::rule`]), followed by one `premise` line per fact
    /// its body matched, in authored body order ([`Explanation::premises`]); a SHACL rule
    /// executed as one producer over the whole evaluation graph lists none. A
    /// triple no rule derived — a SPARQL 1.2 RL data-block triple — carries the single
    /// line `  data-block` instead. Every line ends in `\n`; nothing depends on hash
    /// order, so the text is a pure function of the evaluation.
    #[must_use]
    pub fn proof_text(&self) -> String {
        let mut out = String::new();
        for triple in &self.inferred {
            let [s, p, o] = triple;
            let _ = writeln!(out, "derived {s} {p} {o} .");
            match self.explanations.get(triple) {
                Some(explanation) => {
                    let _ = writeln!(out, "  rule {}", explanation.rule);
                    for [s, p, o] in &explanation.premises {
                        let _ = writeln!(out, "  premise {s} {p} {o} .");
                    }
                }
                None => out.push_str("  data-block\n"),
            }
        }
        out
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
/// that fails, a rule-evaluation limit passed (naming the host's knob that raises it), or
/// the fixed term-arena ceiling passed — each named by the rule it concerns.
pub fn evaluate(
    set: &RuleSet<'_>,
    data: &ShaclData,
    shapes: &Shapes,
    options: &rules::RuleOptions,
) -> Result<Inference, rules::RulesError> {
    evaluate_with(set, data, shapes, options, Reexecution::Incremental)
}

/// How an iterating shape rule is re-executed ([module docs](self), "Re-executing a
/// shape rule").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reexecution {
    /// For the focus nodes that are new, or for every focus node once the graph gained a
    /// triple the rule reads — the production evaluation.
    Incremental,
    /// For every focus node, every iteration, over a graph rebuilt from the whole store
    /// whenever it grew — the SHACL execution read literally, which the incremental one
    /// is proven equal to.
    Full,
}

/// [`evaluate`], re-executing shape rules as `reexecution` says.
pub(crate) fn evaluate_with(
    set: &RuleSet<'_>,
    data: &ShaclData,
    shapes: &Shapes,
    options: &rules::RuleOptions,
    reexecution: Reexecution,
) -> Result<Inference, rules::RulesError> {
    let source_rules = options.source_rules();
    let governors = options.eval_options();
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
        Scheduling::Stratified => super::depend::stratify(set).map_err(|e| describe(&e, set))?,
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
    let base_triples = rules::base_triples(&base);
    // Every label this evaluation mints — a head blank node, a blank node an assignment
    // computes, a data block's blank node standardized apart from the base graph ("rdf
    // merge") — starts with a prefix no base-graph or data-block label starts with.
    let mint_prefix = unused_label_prefix(
        base_triples
            .iter()
            .chain(&set.data)
            .flat_map(|triple| triple.iter()),
    );
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
        mint_prefix,
        expectations,
        reexecution,
        records: RefCell::new(FastMap::default()),
    };
    // Every constant a clause mentions enters the store through its head or a probe, so
    // each is registered with the codec first.
    for rule in &set.rules {
        if let IrRuleBody::Elements(element_rule) = &rule.body {
            register_constants(&engine.codec, element_rule);
        }
    }
    let mut edb = RelationStore::new();
    for triple in base_triples {
        let [s, p, o] = triple.each_ref().map(|t| engine.codec.surface(t));
        edb.insert(&s, &p, &o, RelationStore::DEFAULT_GRAPH);
        if lowered.uses_base {
            edb.insert(&s, &p, &o, BASE_GRAPH);
        }
        engine.originals.insert(triple);
    }
    for triple in &set.data {
        let triple = triple
            .each_ref()
            .map(|term| data_block_term(term, &engine.mint_prefix));
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
    .map_err(|e| refusal(&e, set, options.limit_knobs()))?;

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

    // SPARQL 1.2 RL: "An inference graph is an RDF Graph". A head instantiated with a
    // literal subject or a non-IRI predicate, or a data-block triple spelled that way
    // (the data-block grammar admits "symmetric" literal subjects), is not an RDF triple,
    // and is refused by name rather than dropped from the graph.
    for triple in &inferred {
        if let Err(why) = rdf_triple_check(triple) {
            let source = explanations.get(triple).map_or_else(
                || "a data block".to_owned(),
                |explanation| format!("rule {}", explanation.rule),
            );
            let [s, p, o] = triple;
            return Err(format!(
                "{source} put `{s} {p} {o}` in the inference graph, which is not an RDF \
                 triple: {why}"
            )
            .into());
        }
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

/// An evaluation refusal: a passed rule-evaluation limit naming its rules (for a term
/// limit) and the host's knob, or the described error.
fn refusal(error: &EvalError, set: &RuleSet<'_>, knobs: &rules::LimitKnobs) -> rules::RulesError {
    match error {
        EvalError::BudgetExhausted {
            resource: resource @ (BudgetResource::StoredFacts | BudgetResource::JoinSteps),
            report,
        } => rules::RulesError::limit_exceeded(
            if *resource == BudgetResource::StoredFacts {
                rules::RuleLimit::StoredFacts
            } else {
                rules::RuleLimit::JoinSteps
            },
            Vec::new(),
            *report,
            knobs,
        ),
        EvalError::TermLimitExceeded {
            resource,
            rules: indices,
            report,
        } => rules::RulesError::limit_exceeded(
            match resource {
                BudgetResource::GeneratedTerms => rules::RuleLimit::GeneratedTerms,
                _ => rules::RuleLimit::TermGeneratingRounds,
            },
            indices
                .iter()
                .map(|&index| {
                    let name = set.rules.get(index).map_or_else(
                        || format!("rule {index}"),
                        |rule| format!("rule {}", rule.id),
                    );
                    (index, name)
                })
                .collect(),
            *report,
            knobs,
        ),
        other => describe(other, set).into(),
    }
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

/// The evaluation graph as the SHACL rules read it: the last compacted view with the
/// triples committed since as a delta over it ([`Engine::view`]).
struct LiveView {
    /// The store generation the view was built at.
    generation: u64,
    /// The store row count the view reflects.
    rows: usize,
    /// The surface bytes of the triples in the delta.
    delta_bytes: usize,
    /// The surface bytes every extension since the last compaction re-froze.
    spent: usize,
    /// The Core graph: the compacted view plus the delta.
    core: MutableDataset,
    /// The SPARQL dataset (the Core graph plus the shapes graph), when it is a separate
    /// dataset; `None` when it is the Core graph.
    sparql: Option<MutableDataset>,
    /// The view as SHACL data.
    data: Rc<ShaclData>,
}

impl LiveView {
    /// A view with no delta over the compacted `core` and `sparql` datasets.
    fn compacted(
        generation: u64,
        rows: usize,
        core: Arc<RdfDataset>,
        sparql: Arc<RdfDataset>,
        engine: &Engine<'_, '_>,
    ) -> Self {
        let shared = Arc::ptr_eq(&core, &sparql);
        let data = Rc::new(ShaclData::new(
            Arc::clone(&core),
            Arc::clone(&sparql),
            engine.shapes_graph_iri.clone(),
        ));
        Self {
            generation,
            rows,
            delta_bytes: 0,
            spent: 0,
            core: MutableDataset::new(core),
            sparql: (!shared).then(|| MutableDataset::new(sparql)),
            data,
        }
    }
}

/// The store term bytes below which [`Engine::view`] never compacts on account of the
/// delta's cost: a small store is cheap to rebuild, and a small delta cheap to freeze.
const COMPACTION_FLOOR_BYTES: usize = 1 << 16;

/// The surface bytes of `term`, the unit a view extension and a compaction are costed in.
fn term_bytes(term: &Term) -> usize {
    match term {
        Term::NamedNode(node) => node.as_str().len(),
        Term::BlankNode(label) => label.len(),
        Term::Literal(literal) => literal.value().len() + literal.datatype_str().len(),
        Term::Triple(inner) => {
            term_bytes(&inner.subject) + inner.predicate.as_str().len() + term_bytes(&inner.object)
        }
    }
}

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
    /// The evaluation-graph view last built.
    view: RefCell<Option<LiveView>>,
    /// Bumped whenever the store is rebuilt by a retraction.
    generation: Cell<u64>,
    /// Executions numbered so far, for blank-node minting.
    mints: Cell<u64>,
    /// The label prefix of every blank node an element rule mints (see
    /// [`unused_label_prefix`]).
    mint_prefix: String,
    /// The expected predicates of each layer.
    expectations: Vec<Vec<String>>,
    /// How an iterating shape rule is re-executed.
    reexecution: Reexecution,
    /// Per shape-rule producer (by rule index), what its re-execution is decided by.
    /// `None` for a volatile rule, which keeps no record.
    records: RefCell<FastMap<usize, Option<ProducerRecord>>>,
}

/// What one shape rule's next execution is decided by: its read set, the store it last
/// ran against, and the focus nodes it ran for since.
struct ProducerRecord {
    /// The rule's read set ([`super::reads`]), plus every `r rdf:reifies t`: a new
    /// reifier turns the triples about its subject into that reifier's annotations, a
    /// different layer of the evaluation graph.
    reads: Vec<ReadPattern>,
    /// The `(generation, row count)` of the store the rule last ran against; `None`
    /// before its first execution.
    last: Option<(u64, usize)>,
    /// The focus nodes executed since.
    focus: rules::FocusRecord,
}

/// The read set of a non-volatile shape rule, `rdf:reifies` included (see
/// [`ProducerRecord::reads`]), or `None` for a volatile one.
fn read_patterns(rule: &rules::Rule) -> Option<Vec<ReadPattern>> {
    let RuleReads::Patterns(mut patterns) = super::reads::rule_reads(rule) else {
        return None;
    };
    patterns.push(ReadPattern::unanchored(Some(rdf::REIFIES)));
    Some(patterns)
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

    /// The evaluation graph as SHACL data — see the [module docs](self), "The graph a
    /// SHACL rule reads".
    ///
    /// The view is extended rather than rebuilt when the store has only grown since the
    /// last one, by triples that change no triple's layer: a delta over the last
    /// compacted view, holding every triple committed since. The view is compacted — rebuilt
    /// from the whole store — when a retraction rebuilt the store, when a new triple is
    /// an `r rdf:reifies t` declaration or mentions a blank node (whose identity the
    /// compacted view keeps by label), and once re-freezing the delta has cost as much as
    /// a compaction would.
    fn view(&self, model: &RelationStore) -> Result<Rc<ShaclData>, String> {
        let generation = self.generation.get();
        let rows = model.row_count();
        let mut live = self.view.borrow_mut();
        if let Some(view) = live.as_mut()
            && view.generation == generation
        {
            // The full re-execution also rebuilds the graph it reads from the whole store
            // every time the store grew: the evaluation as it ran before either was
            // incremental, which the incremental one is proven against.
            if view.rows == rows {
                return Ok(Rc::clone(&view.data));
            }
            if self.reexecution == Reexecution::Incremental
                && let Some(data) = self.extend(view, model)?
            {
                return Ok(data);
            }
        }
        let view = self.compact(model, generation)?;
        let data = Rc::clone(&view.data);
        *live = Some(view);
        Ok(data)
    }

    /// The evaluation graph rebuilt from the whole store.
    fn compact(&self, model: &RelationStore, generation: u64) -> Result<LiveView, String> {
        let facts = self.model_facts(model)?;
        let reifiers = rules::reifier_subjects(facts.iter());
        let mut builder = RdfDatasetBuilder::new();
        rules::push_projection(&mut builder, self.base.as_ref());
        for triple in facts.iter().filter(|t| !self.originals.contains(*t)) {
            rules::push_fact(&mut builder, triple, &reifiers)?;
        }
        let core = builder.freeze().map_err(|e| e.to_string())?;
        let sparql = rules::build_round_base(&core, self.shapes, self.shapes_graph_iri.as_deref())?;
        Ok(LiveView::compacted(
            generation,
            model.row_count(),
            core,
            sparql,
            self,
        ))
    }

    /// `view` extended by the store rows committed since it was built, or `None` when
    /// they call for a compaction (see [`Self::view`]).
    fn extend(
        &self,
        view: &mut LiveView,
        model: &RelationStore,
    ) -> Result<Option<Rc<ShaclData>>, String> {
        let rows = model.row_count();
        let reifies = Term::NamedNode(NamedNode::from(rdf::REIFIES));
        let mut triples = self.new_triples(model, view.rows)?;
        if triples
            .iter()
            .any(|triple| triple[1] == reifies || triple.iter().any(rules::term_mentions_blank))
        {
            return Ok(None);
        }
        triples.retain(|triple| !self.originals.contains(triple));
        // Each extension re-freezes the whole delta; a compaction re-freezes the whole
        // store. Once the extensions since the last compaction have cost as much as one
        // more would, compact instead: the rent-or-buy rule, which keeps the total within
        // twice the cheaper of the two strategies' costs whatever rate the graph grows at.
        let delta_bytes =
            view.delta_bytes + triples.iter().flatten().map(term_bytes).sum::<usize>();
        let compact = view.spent + delta_bytes > model.term_bytes().max(COMPACTION_FLOOR_BYTES);
        for [subject, predicate, object] in &triples {
            let quad = ::purrdf::QuadValues::triple(
                subject.to_term_value(),
                predicate.to_term_value(),
                object.to_term_value(),
            );
            for dataset in std::iter::once(&mut view.core).chain(view.sparql.as_mut()) {
                ::purrdf::DatasetMut::insert(dataset, quad.clone()).map_err(|e| e.to_string())?;
            }
        }
        view.rows = rows;
        if compact {
            // The delta holds no triple whose layer a compaction would change, so
            // freezing the base and the delta together is the compaction.
            let core = view.core.freeze().map_err(|e| e.to_string())?;
            let sparql = match &view.sparql {
                Some(sparql) => sparql.freeze().map_err(|e| e.to_string())?,
                None => Arc::clone(&core),
            };
            *view = LiveView::compacted(view.generation, rows, core, sparql, self);
            return Ok(Some(Rc::clone(&view.data)));
        }
        view.delta_bytes = delta_bytes;
        view.spent += delta_bytes;
        let snapshot = |dataset: &MutableDataset| -> Result<Arc<ShaclDatasetView>, String> {
            let delta = dataset.snapshot_view().map_err(|e| e.to_string())?;
            ShaclDatasetView::delta(Arc::new(delta), false, ::purrdf::ir::ViewLimits::default())
                .map(Arc::new)
        };
        let core = snapshot(&view.core)?;
        let sparql = match &view.sparql {
            Some(sparql) => snapshot(sparql)?,
            None => Arc::clone(&core),
        };
        view.data = Rc::new(ShaclData::from_views(
            core,
            sparql,
            self.shapes_graph_iri.clone(),
        ));
        Ok(Some(Rc::clone(&view.data)))
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
                let mut records = self.records.borrow_mut();
                let selection =
                    self.focus_selection(&mut records, call.rule, producer, call.model)?;
                let triples = rules::execute_rule(
                    &view,
                    producer.rule,
                    &producer.shapes,
                    self.shapes_graph_iri.as_deref(),
                    &mut || self.mint(),
                    selection,
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
                // A blank node the expression computes (`BNODE()`) is fresh per
                // evaluation: each call mints under its own prefix.
                let prefix = format!("{}a{}_", self.mint_prefix, self.mint());
                let value = crate::sparql::eval_scalar_query_view_minting(
                    self.empty.sparql_view(),
                    query,
                    &args,
                    &prefix,
                )?;
                // "If evaluating the expression in an assignment causes an error, then the
                // current solution mapping is rejected by the assignment."
                Ok(value.map_or_else(Vec::new, |term| vec![vec![self.codec.surface(&term)]]))
            }
            GuardImpl::FreshBlank => {
                let blank = Term::blank(format!("{}h{}", self.mint_prefix, self.mint()));
                Ok(vec![vec![self.codec.surface(&blank)]])
            }
            GuardImpl::SameTerm => Ok(if call.inputs[0] == call.inputs[1] {
                vec![Vec::new()]
            } else {
                Vec::new()
            }),
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
                let term = build_triple(template, &known)?;
                Ok(vec![vec![self.codec.surface(&term)]])
            }
        }
    }
}

impl Engine<'_, '_> {
    /// Which focus nodes shape rule `rule` executes against `model` — see the [module
    /// docs](self), "Re-executing a shape rule".
    ///
    /// # Errors
    ///
    /// An internal-invariant breach: a store surface the codec never saw.
    fn focus_selection<'m>(
        &self,
        records: &'m mut FastMap<usize, Option<ProducerRecord>>,
        rule: usize,
        producer: &super::ir::ShaclProducer<'_>,
        model: &RelationStore,
    ) -> Result<rules::FocusSelection<'m>, String> {
        if self.reexecution == Reexecution::Full || producer.shapes.is_empty() {
            return Ok(rules::FocusSelection::Every);
        }
        let Some(record) = records
            .entry(rule)
            .or_insert_with(|| {
                read_patterns(producer.rule).map(|reads| ProducerRecord {
                    reads,
                    last: None,
                    focus: rules::FocusRecord::default(),
                })
            })
            .as_mut()
        else {
            return Ok(rules::FocusSelection::Every);
        };
        let now = (self.generation.get(), model.row_count());
        match record.last.replace(now) {
            // A retraction rebuilt the store: nothing about the last run carries over.
            Some((generation, rows)) if generation == now.0 => {
                Ok(rules::FocusSelection::Incremental {
                    gained: self.new_triples(model, rows)?,
                    reads: &record.reads,
                    record: &mut record.focus,
                })
            }
            _ => Ok(rules::FocusSelection::Changed(&mut record.focus)),
        }
    }

    /// The default-graph triples of `model`'s rows from `since` on, in row order.
    ///
    /// # Errors
    ///
    /// An internal-invariant breach: a store surface the codec never saw.
    fn new_triples(&self, model: &RelationStore, since: usize) -> Result<Vec<[Term; 3]>, String> {
        let Some(graph) = model.term_id(RelationStore::DEFAULT_GRAPH) else {
            return Ok(Vec::new());
        };
        model
            .rows_from(since)
            .filter(|(_, [.., g])| *g == graph)
            .map(|(_, [s, p, o, _])| {
                let [s, p, o] = [s, p, o].map(|id| self.codec.term(model.interner().resolve(id)));
                Ok([s?, p?, o?])
            })
            .collect()
    }

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

/// Build the triple term `template` denotes under `known`.
///
/// # Errors
///
/// The instantiation is not an RDF 1.2 triple term — a subject that is not an IRI or a
/// blank node, or a predicate that is not an IRI: RDF 1.2 Concepts §3.1 "A triple term
/// is an RDF term with the same components as an RDF triple".
fn build_triple(template: &LoweredTriple, known: &BTreeMap<&str, Term>) -> Result<Term, String> {
    let value = |position: &LoweredPosition| -> Result<Term, String> {
        match position {
            LoweredPosition::Constant(term) => Ok(term.clone()),
            LoweredPosition::Variable(name) => known.get(name.as_str()).cloned().ok_or_else(|| {
                format!("internal error: triple-term template variable {name} is unbound")
            }),
            LoweredPosition::Triple(inner) => build_triple(inner, known),
        }
    };
    let subject = value(&template.0[0])?;
    let predicate = value(&template.0[1])?;
    let object = value(&template.0[2])?;
    let triple = [subject, predicate, object];
    rdf_triple_check(&triple).map_err(|why| {
        let [s, p, o] = &triple;
        format!("the head builds the triple term <<( {s} {p} {o} )>>, which is not one: {why}")
    })?;
    let [subject, predicate, object] = triple;
    let Term::NamedNode(predicate) = predicate else {
        unreachable!("rdf_triple_check admits only an IRI predicate")
    };
    Ok(Term::Triple(Box::new(Triple {
        subject,
        predicate,
        object,
    })))
}

/// Whether `triple` is an RDF 1.2 triple: RDF 1.2 Concepts §3.1, "the subject, which is
/// an IRI or a blank node", "the predicate, which is an IRI", "the object, which is an
/// IRI, a literal, a blank node or a triple term" — and a triple term object is itself
/// such a triple.
fn rdf_triple_check(triple: &[Term; 3]) -> Result<(), String> {
    let [subject, predicate, object] = triple;
    if !matches!(subject, Term::NamedNode(_) | Term::BlankNode(_)) {
        return Err(format!(
            "its subject {subject} is not an IRI or a blank node (RDF 1.2 Concepts §3.1: \
             \"the subject, which is an IRI or a blank node\")"
        ));
    }
    if !matches!(predicate, Term::NamedNode(_)) {
        return Err(format!(
            "its predicate {predicate} is not an IRI (RDF 1.2 Concepts §3.1: \"the \
             predicate, which is an IRI\")"
        ));
    }
    if let Term::Triple(inner) = object {
        rdf_triple_check(&[
            inner.subject.clone(),
            Term::NamedNode(inner.predicate.clone()),
            inner.object.clone(),
        ])?;
    }
    Ok(())
}

/// A blank-node label prefix no label among `terms` (nested triple terms included)
/// starts with: `srl-` lengthened with `x` until none does.
fn unused_label_prefix<'t>(terms: impl Iterator<Item = &'t Term>) -> String {
    fn labels<'t>(term: &'t Term, out: &mut Vec<&'t str>) {
        match term {
            Term::BlankNode(label) => out.push(label),
            Term::Triple(inner) => {
                labels(&inner.subject, out);
                labels(&inner.object, out);
            }
            Term::NamedNode(_) | Term::Literal(_) => {}
        }
    }
    let mut all = Vec::new();
    for term in terms {
        labels(term, &mut all);
    }
    let mut prefix = String::from("srl-");
    while all.iter().any(|label| label.starts_with(prefix.as_str())) {
        prefix.insert(0, 'x');
    }
    prefix
}

/// A data-block term with its blank nodes standardized apart from the base graph's: the
/// rule set's data blocks and the base graph are separate RDF graphs, and their union is
/// an RDF merge, so a data-block `_:b` is never the base graph's `_:b`.
fn data_block_term(term: &Term, prefix: &str) -> Term {
    match term {
        Term::BlankNode(label) => Term::blank(format!("{prefix}d-{label}")),
        Term::Triple(inner) => Term::Triple(Box::new(Triple {
            subject: data_block_term(&inner.subject, prefix),
            predicate: inner.predicate.clone(),
            object: data_block_term(&inner.object, prefix),
        })),
        Term::NamedNode(_) | Term::Literal(_) => term.clone(),
    }
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
