// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL 1.2 Inference Rules.
//!
//! This module holds the SHACL rule MODEL the shapes parser produces — [`Rule`],
//! [`RuleBody`], the [`RuleGraph`] of global rules and rule sets — the host's
//! [`RuleOptions`], and the SHACL half of rule execution: the producers that evaluate
//! a triple rule's node expressions or a SPARQL rule's CONSTRUCT query for one
//! execution. It holds no fixpoint. [`infer`] lowers the shapes graph's rules to the
//! rule-set IR ([`crate::srl::ir`]), and the one rules engine ([`crate::srl`]) evaluates
//! that IR on `purrdf-datalog`'s ordered schedule.
//!
//! # The execution the specification defines
//!
//! SHACL 1.2 Inference Rules, "General Execution Instructions for SHACL Rules":
//!
//! > For all layers in the rule set (in ascending order): Compute the expected derived
//! > triples for all rules in the layer; Execute one iteration over all run-once rules
//! > in the layer; do Execute one iteration over all iterating rules in the layer while
//! > the iteration has produced newly inferred triples; Delete the derived triples
//! > (except those that were also inferred by rules) and their reifiers. Delete the
//! > temporary triples and their reifiers.
//!
//! with "an iteration is a single execution of each individual rule in the order as
//! specified by" `sh:order`: "within the same layer, rules with larger order values will
//! be executed after those with smaller values. The inferred triples of one rule (or
//! group of same-order rules) become immediately visible to the subsequent rule (or group
//! of same-order rules) in the order. Rules with the same order are executed concurrently
//! and must not see each other's inferences before they have all completed."
//!
//! Each layer is a layer of the schedule, each `(run-once?, sh:order)` value a group of
//! concurrently executed rules — see [`crate::srl`] for how the schedule is built and
//! `purrdf_datalog::schedule` for how it runs.
//!
//! # Blank nodes are fresh per execution
//!
//! A SPARQL rule's CONSTRUCT template blank nodes and its `BNODE()` calls mint blank
//! nodes under a label prefix unique to ONE execution of the rule for one focus node, so
//! two executions never share a minted blank — "some rules may produce fresh blank nodes
//! with each execution and therefore cause infinite iterations", which is exactly what
//! `sh:runOnce` exists to prevent. A rule that mints on every pass without `sh:runOnce`
//! is stopped by the term-generating round limit the caller configures
//! ([`RuleOptions::with_max_term_generating_rounds`]), as the specification allows:
//! "Rule engines MAY also report a failure after a pre-configured maximum iteration
//! count has been exceeded".
//!
//! # Ill-formed triples are skipped
//!
//! SHACL 1.2 Inference Rules, "Execution of triple rules": "Skip ill-formed triples,
//! for example when a blank node is used as predicate." A triple rule producing a
//! literal subject or a non-IRI predicate produces no triple for that combination, and
//! a SPARQL rule's CONSTRUCT already omits them by SPARQL's own CONSTRUCT semantics.

use std::sync::Arc;

use ::purrdf::{FastSet, RdfDataset, RdfDatasetBuilder, RdfQuad, RdfTerm};
use purrdf_sparql_algebra::{Query, SparqlParser};

use crate::constraints::conforms_with_plan;
use crate::data::{GraphFilter, ShaclData, quads_for_pattern_ids};
use crate::engine::resolve_focus_nodes;
use crate::expression::{NodeExpr, RecursionGuard, eval_planned_node_expr};
use crate::shapes::{Path, PropertyShape, Shape, Shapes};
use crate::srl::{self, Inference};
use crate::term::{NamedNode, Term, term_id_to_native};

/// The fixed non-default [`::purrdf::BlankScope`] the shapes document's blanks
/// are standardized apart into when exposed as `$shapesGraph` (see
/// [`build_round_base`]): disjoint from [`::purrdf::BlankScope::DEFAULT`], which
/// the base data and derived facts share. Named so a second scoped push cannot
/// silently reuse the bare literal `1` and conflate two documents' blanks — every
/// additional shapes-graph-scope push in this module must use this constant.
const SHAPES_BLANK_SCOPE: ::purrdf::BlankScope = ::purrdf::BlankScope(1);

// ── Model ───────────────────────────────────────────────────────────────────────

/// What a SHACL rule executes.
#[derive(Debug, Clone)]
#[allow(
    clippy::large_enum_variant,
    reason = "the model mirrors the SHACL rules vocabulary: a TripleRule head is three \
              inline node expressions, a SPARQLRule head one query string; boxing either \
              would obscure the 1:1 mapping with sh:subject/predicate/object vs sh:construct"
)]
pub enum RuleBody {
    /// A `sh:TripleRule`. SHACL 1.2 Inference Rules: "Let S, P and O be the sets of
    /// nodes produced by evaluating the node expressions that are the values of
    /// sh:subject, sh:predicate and sh:object respectively at the triple rule. Where
    /// sh:subject, sh:predicate, or sh:object are absent, use the list consisting of the
    /// current focus node (which is empty for global rules). For each combination of
    /// members s of S, p of P and o of O, infer a triple with subject s, predicate p and
    /// object o."
    Triple {
        /// The `sh:subject` node expression; `None` when absent (the focus node).
        subject: Option<NodeExpr>,
        /// The `sh:predicate` node expression; `None` when absent (the focus node).
        predicate: Option<NodeExpr>,
        /// The `sh:object` node expression; `None` when absent (the focus node).
        object: Option<NodeExpr>,
    },
    /// A `sh:SPARQLRule`, or an instance of a `sh:SPARQLRuleTemplate`. SHACL 1.2
    /// Inference Rules: "If the rule is a shape rule: For each focus node, execute the
    /// query Q pre-binding the variable this to the focus node, and infer the constructed
    /// triples. If the rule is a global rule: Execute the query Q without any
    /// pre-binding, and infer the constructed triples." For a template instance: "Using
    /// a pre-binding map for each declared sh:parameter of T at R similar to SPARQL-based
    /// constraint components, evaluate Q like a corresponding SPARQL Rule but using the
    /// extra pre-bound variables."
    Sparql {
        /// The CONSTRUCT query text, with its `PREFIX` header.
        construct: String,
        /// The template parameters' pre-bindings, `(variable name, value)` in parameter
        /// path order; empty for a `sh:SPARQLRule`.
        parameters: Vec<(String, Term)>,
    },
}

/// A rule's `sh:layer` or `sh:order` value: a numeric literal, lower runs first.
///
/// Not `Ord` (it wraps an `f64`); the scheduler orders values with
/// [`OrderKey::value`] via `f64::total_cmp`.
///
/// The stored value is CANONICAL: `-0.0` is normalized to `0.0` on construction. Both
/// properties are decimal-valued, and equal values mean "same layer" or "same group",
/// which PARTITIONS execution and so changes the inferences — two spellings of the
/// same NUMBER must never land apart. `-0.0` is the only IEEE-754 value with two
/// encodings a decimal lexical form can produce, and `f64::total_cmp` distinguishes
/// them. The non-finite encodings (`NaN`, `INF`), which have no position at all, are
/// refused by the parser before reaching here.
#[derive(Debug, Clone, Copy)]
pub struct OrderKey {
    value: f64,
}

impl OrderKey {
    /// Wrap a numeric value, normalizing `-0.0` to `0.0` so the key identifies the
    /// NUMBER rather than its IEEE-754 encoding.
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self {
            // `+ 0.0` maps -0.0 to +0.0 and is the identity on every other finite value.
            value: value + 0.0,
        }
    }

    /// The canonical numeric value (lower runs first).
    #[must_use]
    pub fn value(self) -> f64 {
        self.value
    }
}

/// One SHACL rule.
#[derive(Debug, Clone)]
pub struct Rule {
    /// The rule node (IRI or blank node): the rule's identity, the tie-break inside a
    /// group, and the name every diagnostic and explanation gives it.
    pub id: Term,
    /// What the rule executes.
    pub body: RuleBody,
    /// The `sh:condition` shapes. SHACL 1.2 Inference Rules: "A shape rule may have
    /// values for the property sh:condition to specify shapes that the target nodes must
    /// conform to before they become focus nodes for the rule." A condition that is a
    /// SHACL instance of both `sh:NodeShape` and `rdfs:Class` brings its non-deactivated
    /// such superclasses with it ("the focus nodes must also conform to the constraints
    /// of the non-deactivated SHACL superclasses of C that are also SHACL instances of
    /// both sh:NodeShape and rdfs:Class"), resolved by the parser into this list.
    ///
    /// These are PARSED SHAPES: a condition that names nothing the shapes graph describes
    /// as a shape is a shapes-LOAD error, so a rule never fires over an unevaluated
    /// condition.
    pub conditions: Vec<Shape>,
    /// The `sh:layer` value, if declared (default layer 0).
    pub layer: Option<OrderKey>,
    /// The `sh:order` value, if declared (default order 0).
    pub order: Option<OrderKey>,
    /// Whether `sh:runOnce true` is set: "A run-once rule is a rule for which at most one
    /// iteration is performed (per shape, if it is a shape rule), i.e. it is executed at
    /// most once per focus node."
    pub run_once: bool,
    /// Whether `sh:deactivated true` is set — "Deactivated rules are ignored by the rules
    /// engine."
    pub deactivated: bool,
    /// The rule's `sh:expectedPredicate` values, sorted and deduplicated (SHACL 1.2
    /// Inference Rules, "Expected Derived Triples").
    pub expected_predicates: Vec<NamedNode>,
    /// The rule's `sh:ruleProcessor` values: IRIs or `xsd:string` literals, in canonical
    /// term order. See [`RuleOptions::with_rule_processor`].
    pub processors: Vec<Term>,
}

impl Rule {
    /// The effective layer (declared `sh:layer`, or 0).
    #[must_use]
    pub fn layer_value(&self) -> OrderKey {
        self.layer.unwrap_or_else(|| OrderKey::new(0.0))
    }

    /// The effective order (declared `sh:order`, or 0).
    #[must_use]
    pub fn order_value(&self) -> OrderKey {
        self.order.unwrap_or_else(|| OrderKey::new(0.0))
    }
}

/// A `sh:RuleSet`. SHACL 1.2 Inference Rules, "Rule Sets": "A rule set is identified by
/// an IRI. The property sh:hasRule can be used to declare that a rule set has a given
/// rule as a member. […] Rule sets can use the property sh:includesRuleSet to
/// (transitively) include other rule sets."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSetDeclaration {
    /// The rule set's IRI.
    pub id: NamedNode,
    /// Its `sh:hasRule` members, in canonical term order.
    pub rules: Vec<Term>,
    /// Its `sh:includesRuleSet` values, in IRI order.
    pub includes: Vec<NamedNode>,
    /// Its `sh:ruleProcessor` values, in canonical term order. "When used in a rule set,
    /// the values of sh:ruleProcessor from an included rule set also apply to the
    /// including rule set."
    pub processors: Vec<Term>,
}

/// The rules of a shapes graph that are not attached to a shape, and its rule sets.
#[derive(Debug, Clone, Default)]
pub struct RuleGraph {
    /// The GLOBAL rules: "A global rule is a rule that is not linked to a shape by a
    /// sh:rule predicate." In canonical rule-node order.
    pub global_rules: Vec<Rule>,
    /// Every `sh:RuleSet` of the shapes graph, in IRI order.
    pub rule_sets: Vec<RuleSetDeclaration>,
    /// Whether the shapes graph declares `sh:entailment sh:RulesEntailment`, so
    /// validation runs the rules first. SHACL 1.2 Inference Rules: "Validation engines
    /// that do support the SHACL rules entailment regime execute the rules following the
    /// rules execution instructions prior to performing the actual validation."
    pub entailment: bool,
}

/// What a host declares a `sh:ruleProcessor` value to mean.
///
/// SHACL 1.2 Inference Rules, "Custom Rule Processors": "The property sh:ruleProcessor
/// can be used at rule sets or rules to instruct a rules engine that non-standard
/// processing is required for the given rules. The values of sh:ruleProcessor are IRIs,
/// or literals with datatype xsd:string. […] A rules engine that encounters rules or rule
/// sets with a value for sh:ruleProcessor that they are unable to handle MUST report a
/// failure." The specification names no processor, and PurRDF mints none: a value is
/// handled exactly when the host has registered it, and every unregistered value is a
/// failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleProcessor {
    /// The specification's own "General Execution Instructions for SHACL Rules": the
    /// host declares the value names processing this engine performs by default, with
    /// no additional run-once rules, orders or layers computed.
    Standard,
}

/// A host's rule-execution request.
#[derive(Debug, Clone)]
pub struct RuleOptions {
    /// The `sh:ruleProcessor` values the host handles, and how.
    processors: Vec<(Term, RuleProcessor)>,
    /// The rule set to execute; `None` for the default rule set.
    rule_set: Option<NamedNode>,
    /// Whether to track each inferred triple's rule with `sh:sourceRule`.
    source_rules: bool,
    /// The limit on term-generating rounds ([`Self::with_max_term_generating_rounds`]).
    max_term_generating_rounds: u64,
}

impl Default for RuleOptions {
    fn default() -> Self {
        Self {
            processors: Vec::new(),
            rule_set: None,
            source_rules: false,
            max_term_generating_rounds:
                purrdf_datalog::seminaive::DEFAULT_MAX_TERM_GENERATING_ROUNDS,
        }
    }
}

impl RuleOptions {
    /// Declare that the host handles the `sh:ruleProcessor` value `value` as `processor`.
    #[must_use]
    pub fn with_rule_processor(mut self, value: Term, processor: RuleProcessor) -> Self {
        self.processors.retain(|(known, _)| *known != value);
        self.processors.push((value, processor));
        self
    }

    /// Execute the rule set `iri` instead of the default rule set. SHACL 1.2 Inference
    /// Rules: "A SHACL rules engine is a computer procedure that takes as input a data
    /// graph called the base graph, a shapes graph, and an optional rule set (defaulting
    /// to the default rule set of the shapes graph)" — "The default rule set of a graph
    /// is the set of all rules in the graph."
    #[must_use]
    pub fn with_rule_set(mut self, iri: NamedNode) -> Self {
        self.rule_set = Some(iri);
        self
    }

    /// Track the rule that produced each inferred triple. SHACL 1.2 Inference Rules,
    /// "Tracking the Rule that has produced a Triple": "The property sh:sourceRule can be
    /// used in a reifier of a triple in the inferences graph to link the triple with the
    /// rule. […] If a rule engine adds these triples, the triples MUST NOT be visible to
    /// executing rules" — they are added after the last layer.
    #[must_use]
    pub fn with_source_rules(mut self, track: bool) -> Self {
        self.source_rules = track;
        self
    }

    /// Permit at most `rounds` TERM-GENERATING rounds: rounds that infer a term the
    /// evaluation graph did not hold — a computed literal, a longer IRI, a fresh blank
    /// node. One more is refused with an error naming the limit.
    ///
    /// SHACL 1.2 Inference Rules: "Rule engines MAY also report a failure after a
    /// pre-configured maximum iteration count has been exceeded". Whether a rule set
    /// that keeps computing new terms terminates is undecidable, so no fixed count is
    /// right for every rule set: a counter stepping to 1000 needs 1000 such rounds and
    /// terminates. The limit is therefore the host's, defaulting to
    /// `purrdf_datalog::seminaive::DEFAULT_MAX_TERM_GENERATING_ROUNDS`. A round that
    /// infers only terms the graph already holds is never counted. Every SHACL rule
    /// round re-executes the rule over the whole evaluation graph, so a host running
    /// untrusted rule sets bounds the time a divergent one takes by LOWERING this limit.
    #[must_use]
    pub fn with_max_term_generating_rounds(mut self, rounds: u64) -> Self {
        self.max_term_generating_rounds = rounds;
        self
    }

    /// The term-generating round limit in force.
    #[must_use]
    pub fn max_term_generating_rounds(&self) -> u64 {
        self.max_term_generating_rounds
    }

    /// How the host handles `value`, if it does.
    #[must_use]
    pub fn rule_processor(&self, value: &Term) -> Option<RuleProcessor> {
        self.processors
            .iter()
            .find(|(known, _)| known == value)
            .map(|(_, processor)| *processor)
    }

    /// The rule set to execute, if not the default.
    #[must_use]
    pub fn rule_set(&self) -> Option<&NamedNode> {
        self.rule_set.as_ref()
    }

    /// Whether inferred triples are tracked with `sh:sourceRule`.
    #[must_use]
    pub fn source_rules(&self) -> bool {
        self.source_rules
    }
}

// ── Entry points ────────────────────────────────────────────────────────────────

/// Execute the shapes graph's rules over `data` and return the dataset of the base
/// graph plus every inferred triple. The default rule set, with no rule processor
/// registered.
///
/// The rules read and write the FLATTENED default graph (the same projection the
/// validator operates over, exposed by [`ShaclData::core`]). The result is
/// deterministic: byte-stable across runs, under isomorphic input relabeling, and under
/// permutation of the shapes graph's insertion order.
///
/// # Errors
///
/// See [`infer`].
pub fn apply_rules(data: &ShaclData, shapes: &Shapes) -> Result<Arc<RdfDataset>, String> {
    infer(data, shapes, &RuleOptions::default()).map(|inference| Arc::clone(inference.dataset()))
}

/// Execute the shapes graph's rules over `data` under `options`.
///
/// # Errors
///
/// A failure the specification makes one: a `sh:ruleProcessor` value the host has not
/// registered, on a rule or on any rule set; a requested rule set the shapes graph does
/// not declare; a rule or expression that fails during execution; and a rule set that
/// passes one of the engine's fixed ceilings — a rule minting a new term every pass
/// included.
pub fn infer(
    data: &ShaclData,
    shapes: &Shapes,
    options: &RuleOptions,
) -> Result<Inference, String> {
    // Declared SHACL-AF functions, and any caller-injected custom aggregates, are in
    // scope for node expressions and CONSTRUCT bodies for the whole run; the guards
    // restore the previous tables on drop.
    let _function_scope =
        crate::sparql::enter_function_scope(crate::sparql::bind_in_current_env(&shapes.functions)?);
    let _aggregate_scope = crate::sparql::enter_aggregate_scope(Arc::clone(&shapes.aggregates));
    let rule_set = shacl_rule_set(shapes, options)?;
    srl::evaluate(&rule_set, data, shapes, options)
}

/// Execute the shapes graph's rules over a frozen [`RdfDataset`]: build the SHACL
/// projection, run [`apply_rules`], and return the entailed dataset (mirrors
/// [`validate_dataset`](crate::engine::validate_dataset)).
///
/// # Errors
///
/// Returns `Err(String)` when the SHACL projection cannot be frozen or when rule
/// application fails (see [`infer`]).
pub fn entail_dataset(data: &RdfDataset, shapes: &Shapes) -> Result<Arc<RdfDataset>, String> {
    let projected = crate::engine::project_dataset(data)?;
    // Core lookups and the SHACL-SPARQL / CONSTRUCT paths run over the same
    // flattened projection.
    let holder = ShaclData::new(Arc::clone(&projected), projected, None);
    apply_rules(&holder, shapes)
}

/// Lower the shapes graph's rules — the default rule set, or the one `options` names —
/// to the rule-set IR.
///
/// Every `sh:ruleProcessor` value the rules engine encounters is checked first: those of
/// every rule set the shapes graph declares, since "a rules engine that encounters rules
/// or rule sets with a value for sh:ruleProcessor that they are unable to handle MUST
/// report a failure" and a declared rule set is encountered whether or not it is
/// executed, and those of every rule executed.
///
/// A rule linked from several shapes is ONE rule executed for the target nodes of each
/// linked, non-deactivated shape.
///
/// # Errors
///
/// An unregistered rule processor, or a requested rule set that is not declared.
pub fn shacl_rule_set<'a>(
    shapes: &'a Shapes,
    options: &RuleOptions,
) -> Result<srl::ir::RuleSet<'a>, String> {
    for set in &shapes.rules.rule_sets {
        for value in &set.processors {
            if options.rule_processor(value).is_none() {
                return Err(format!(
                    "rule set {} declares sh:ruleProcessor {value}, which this rules engine \
                     has not been given a processor for; SHACL 1.2 Inference Rules: \"A rules \
                     engine that encounters rules or rule sets with a value for \
                     sh:ruleProcessor that they are unable to handle MUST report a failure\"",
                    Term::NamedNode(set.id.clone())
                ));
            }
        }
    }
    let members = match options.rule_set() {
        None => None,
        Some(iri) => Some(rule_set_members(&shapes.rules.rule_sets, iri)?),
    };
    let selected = |rule: &Rule| members.as_ref().is_none_or(|set| set.contains(&rule.id));

    // Shape rules: one IR rule per rule node, linked to every non-deactivated shape.
    let mut linked: Vec<(&'a Rule, Vec<&'a Shape>)> = Vec::new();
    for shape in &shapes.node_shapes {
        for rule in &shape.rules {
            if !selected(rule) {
                continue;
            }
            match linked.iter_mut().find(|(known, _)| known.id == rule.id) {
                Some((_, shapes)) => {
                    if !shape.deactivated {
                        shapes.push(shape);
                    }
                }
                None => linked.push((
                    rule,
                    if shape.deactivated {
                        Vec::new()
                    } else {
                        vec![shape]
                    },
                )),
            }
        }
    }
    let mut rules: Vec<srl::ir::IrRule<'a>> = Vec::new();
    for (rule, shapes) in linked {
        check_rule_processors(rule, options)?;
        // A rule linked only from deactivated shapes executes for no focus node; it is
        // still checked, and it is still a shape rule rather than a global one.
        if !rule.deactivated && !shapes.is_empty() {
            rules.push(ir_rule(rule, shapes));
        }
    }
    for rule in &shapes.rules.global_rules {
        if selected(rule) {
            check_rule_processors(rule, options)?;
            if !rule.deactivated {
                rules.push(ir_rule(rule, Vec::new()));
            }
        }
    }
    Ok(srl::ir::RuleSet {
        rules,
        data: Vec::new(),
        scheduling: srl::ir::Scheduling::Declared,
    })
}

/// Refuse a rule whose `sh:ruleProcessor` the host has not registered.
fn check_rule_processors(rule: &Rule, options: &RuleOptions) -> Result<(), String> {
    for value in &rule.processors {
        if options.rule_processor(value).is_none() {
            return Err(format!(
                "rule {} declares sh:ruleProcessor {value}, which this rules engine has not \
                 been given a processor for; SHACL 1.2 Inference Rules: \"A rules engine that \
                 encounters rules or rule sets with a value for sh:ruleProcessor that they are \
                 unable to handle MUST report a failure\"",
                rule.id
            ));
        }
    }
    Ok(())
}

/// The IR rule of a SHACL rule linked from `shapes` (none for a global rule).
fn ir_rule<'a>(rule: &'a Rule, shapes: Vec<&'a Shape>) -> srl::ir::IrRule<'a> {
    srl::ir::IrRule {
        id: rule.id.clone(),
        body: srl::ir::IrRuleBody::Shacl(srl::ir::ShaclProducer { rule, shapes }),
        schedule: srl::ir::DeclaredSchedule {
            layer: rule.layer_value(),
            order: rule.order_value(),
            run_once: rule.run_once,
        },
        expected_predicates: rule.expected_predicates.clone(),
    }
}

/// The rules of rule set `iri`, its included rule sets' transitively.
///
/// # Errors
///
/// When `iri`, or a rule set it includes, is not a declared rule set.
fn rule_set_members(sets: &[RuleSetDeclaration], iri: &NamedNode) -> Result<Vec<Term>, String> {
    let mut members: Vec<Term> = Vec::new();
    let mut pending = vec![iri.clone()];
    let mut seen: Vec<NamedNode> = Vec::new();
    while let Some(next) = pending.pop() {
        if seen.contains(&next) {
            continue;
        }
        let Some(set) = sets.iter().find(|set| set.id == next) else {
            return Err(format!(
                "rule set {} is not declared in the shapes graph (no sh:RuleSet of that IRI)",
                Term::NamedNode(next)
            ));
        };
        members.extend(set.rules.iter().cloned());
        pending.extend(set.includes.iter().cloned());
        seen.push(next);
    }
    Ok(members)
}

// ── Producers: one execution of one SHACL rule ──────────────────────────────────

/// Execute one SHACL rule over the evaluation graph `data` and return the triples it
/// infers, well-formed ones only.
///
/// `mint` numbers this execution: every blank node the execution mints carries it, so no
/// two executions ever mint the same blank node.
///
/// # Errors
///
/// When a condition, a node expression or the CONSTRUCT query fails to evaluate.
pub(crate) fn execute_rule(
    data: &ShaclData,
    rule: &Rule,
    shapes: &[&Shape],
    shapes_graph_iri: Option<&str>,
    mint: &mut dyn FnMut() -> u64,
) -> Result<Vec<[Term; 3]>, String> {
    let mut out: Vec<[Term; 3]> = Vec::new();
    if shapes.is_empty() {
        match &rule.body {
            RuleBody::Triple {
                subject,
                predicate,
                object,
            } => global_triple_rule(
                data,
                subject.as_ref(),
                predicate.as_ref(),
                object.as_ref(),
                mint(),
                &mut out,
            )?,
            RuleBody::Sparql {
                construct,
                parameters,
            } => sparql_rule_execution(
                data,
                &SparqlExecution {
                    construct,
                    parameters,
                    focus_nodes: None,
                    shape: None,
                    shapes_graph_iri,
                },
                mint,
                &mut out,
            )?,
        }
        return Ok(out);
    }
    for shape in shapes {
        let plan = RulePlan::of(data, shape, &rule.conditions);
        let mut focus_nodes = Vec::new();
        for focus in plan.focus_nodes(data)? {
            if conditions_hold(data, &focus, &plan)? {
                focus_nodes.push(focus);
            }
        }
        match &rule.body {
            RuleBody::Triple {
                subject,
                predicate,
                object,
            } => triple_rule_execution(data, [subject, predicate, object], &focus_nodes, &mut out)?,
            RuleBody::Sparql {
                construct,
                parameters,
            } => sparql_rule_execution(
                data,
                &SparqlExecution {
                    construct,
                    parameters,
                    focus_nodes: Some(&focus_nodes),
                    shape: Some(&shape.id),
                    shapes_graph_iri,
                },
                mint,
                &mut out,
            )?,
        }
    }
    Ok(out)
}

/// Whether `(s, p, o)` is a well-formed RDF triple: an IRI or blank subject, an IRI
/// predicate. A triple term subject is admitted as RDF 1.2 admits it nowhere, so it is
/// refused as ill-formed too.
fn well_formed(subject: &Term, predicate: &Term) -> bool {
    matches!(subject, Term::NamedNode(_) | Term::BlankNode(_))
        && matches!(predicate, Term::NamedNode(_))
}

/// A shape rule's `sh:TripleRule` execution over its focus nodes.
fn triple_rule_execution(
    data: &ShaclData,
    expressions: [&Option<NodeExpr>; 3],
    focus_nodes: &[Term],
    out: &mut Vec<[Term; 3]>,
) -> Result<(), String> {
    // The head's node expressions are rule constants: lowered and bound ONCE per
    // execution rather than once per focus node.
    let plans: Vec<Option<ExprPlan<'_>>> = expressions
        .iter()
        .map(|expr| expr.as_ref().map(|expr| ExprPlan::of(data, expr)))
        .collect();
    for focus in focus_nodes {
        let mut guard = RecursionGuard::new();
        let mut sets: Vec<Vec<Term>> = Vec::with_capacity(3);
        for plan in &plans {
            sets.push(match plan {
                // "Where sh:subject, sh:predicate, or sh:object are absent, use the list
                // consisting of the current focus node".
                None => vec![focus.clone()],
                Some(plan) => plan.eval(data, focus, &mut guard)?,
            });
        }
        cartesian(&sets[0], &sets[1], &sets[2], out);
    }
    Ok(())
}

/// A GLOBAL `sh:TripleRule` execution: "(which is empty for global rules)".
///
/// An absent `sh:subject`, `sh:predicate` or `sh:object` is the empty list, so the rule
/// infers nothing unless all three are present. The three expressions are evaluated with
/// NO focus node: they are evaluated at a blank node minted for this execution alone,
/// which no graph mentions — it has no values, no types and conforms to nothing a graph
/// states about it — and every output node that IS that blank node is dropped, because
/// it stands for the focus node a global rule does not have. So `sh:this` yields nothing
/// and a path from the focus yields nothing, while a constant or a node-independent
/// expression yields exactly its value.
fn global_triple_rule(
    data: &ShaclData,
    subject: Option<&NodeExpr>,
    predicate: Option<&NodeExpr>,
    object: Option<&NodeExpr>,
    execution: u64,
    out: &mut Vec<[Term; 3]>,
) -> Result<(), String> {
    let (Some(subject), Some(predicate), Some(object)) = (subject, predicate, object) else {
        return Ok(());
    };
    let absent = Term::blank(format!("g-x{execution}_focus"));
    let mut guard = RecursionGuard::new();
    let mut sets: Vec<Vec<Term>> = Vec::with_capacity(3);
    for expr in [subject, predicate, object] {
        let mut values = ExprPlan::of(data, expr).eval(data, &absent, &mut guard)?;
        values.retain(|value| *value != absent);
        sets.push(values);
    }
    cartesian(&sets[0], &sets[1], &sets[2], out);
    Ok(())
}

/// Every well-formed `(s, p, o)` of `S × P × O`.
fn cartesian(subjects: &[Term], predicates: &[Term], objects: &[Term], out: &mut Vec<[Term; 3]>) {
    for s in subjects {
        for p in predicates {
            // "Skip ill-formed triples, for example when a blank node is used as
            // predicate."
            if !well_formed(s, p) {
                continue;
            }
            for o in objects {
                out.push([s.clone(), p.clone(), o.clone()]);
            }
        }
    }
}

/// One SPARQL rule's execution parameters.
struct SparqlExecution<'q> {
    /// The CONSTRUCT query text.
    construct: &'q str,
    /// The template parameters' pre-bindings.
    parameters: &'q [(String, Term)],
    /// The focus nodes of a shape rule; `None` for a global rule.
    focus_nodes: Option<&'q [Term]>,
    /// The linking shape of a shape rule.
    shape: Option<&'q Term>,
    /// The shapes graph IRI, pre-bound as `$shapesGraph` for a shape rule.
    shapes_graph_iri: Option<&'q str>,
}

/// A `sh:SPARQLRule` (or template instance) execution: the CONSTRUCT query once per
/// focus node with `$this` pre-bound, or once without it for a global rule.
fn sparql_rule_execution(
    data: &ShaclData,
    run: &SparqlExecution<'_>,
    mint: &mut dyn FnMut() -> u64,
    out: &mut Vec<[Term; 3]>,
) -> Result<(), String> {
    match run.focus_nodes {
        Some(focus_nodes) => {
            // SHACL-SPARQL pre-binds `$this`, `$shapesGraph` and `$currentShape`, and a
            // template instance its parameters. Everything but `$this` is a constant of
            // the RULE, so the query is PREPARED once for the whole focus set and only
            // `$this` is rewritten per focus node.
            const THIS_SLOT: usize = 0;
            let context =
                crate::sparql::this_and_shape_context_names(run.shapes_graph_iri, run.shape);
            let mut names: Vec<&str> = context.to_vec();
            names.extend(run.parameters.iter().map(|(name, _)| name.as_str()));
            crate::sparql::with_cached_execution(
                run.construct,
                &names,
                purrdf_sparql_eval::ShaclPrebinding::Applied,
                |execution| {
                    let first = crate::sparql::bind_shape_context(
                        execution,
                        THIS_SLOT + 1,
                        run.shapes_graph_iri,
                        run.shape,
                    )?;
                    for (slot, (_, value)) in (first..).zip(run.parameters) {
                        execution.bind(slot, value.to_term_value())?;
                    }
                    for focus in focus_nodes {
                        let tag = mint_tag(Some(focus), mint());
                        execution.bind(THIS_SLOT, focus.to_term_value())?;
                        let graph = crate::sparql::run_bound_construct_with_shacl_prebinding_view(
                            data.sparql_view(),
                            execution,
                            Some(tag.as_str()),
                        )?;
                        read_constructed(&graph, out);
                    }
                    Ok(())
                },
            )
        }
        None => {
            // "Execute the query Q without any pre-binding" — a template instance's
            // parameters are its only pre-bindings.
            let names: Vec<&str> = run
                .parameters
                .iter()
                .map(|(name, _)| name.as_str())
                .collect();
            crate::sparql::with_cached_execution(
                run.construct,
                &names,
                purrdf_sparql_eval::ShaclPrebinding::Applied,
                |execution| {
                    for (slot, (_, value)) in run.parameters.iter().enumerate() {
                        execution.bind(slot, value.to_term_value())?;
                    }
                    let tag = mint_tag(None, mint());
                    let graph = crate::sparql::run_bound_construct_with_shacl_prebinding_view(
                        data.sparql_view(),
                        execution,
                        Some(tag.as_str()),
                    )?;
                    read_constructed(&graph, out);
                    Ok(())
                },
            )
        }
    }
}

/// Read a CONSTRUCT graph's triples — its RDF 1.2 statement layer included, since a
/// `?r rdf:reifies <<( … )>>` head row is a reifier declaration and a row about `?r` its
/// annotation, both held beside the quads — keeping the well-formed ones.
fn read_constructed(graph: &RdfDataset, out: &mut Vec<[Term; 3]>) {
    for quad in quads_for_pattern_ids(graph, None, None, None, GraphFilter::AnyGraph)
        .chain(graph.reifier_quads())
        .chain(graph.annotation_quads())
    {
        let s = term_id_to_native(graph, quad.s);
        let p = term_id_to_native(graph, quad.p);
        if !well_formed(&s, &p) {
            continue;
        }
        out.push([s, p, term_id_to_native(graph, quad.o)]);
    }
}

// ── Expected derived triples ────────────────────────────────────────────────────

/// Whether property shape `ps` computes derived value nodes for one of
/// `expected`: it is not deactivated, its path is one of those predicates, and it
/// declares `sh:values` or `sh:defaultValue`.
fn derives_expected(ps: &PropertyShape, expected: &FastSet<&str>) -> bool {
    !ps.deactivated
        && (ps.values.is_some() || ps.default_value.is_some())
        && matches!(&ps.path, Path::Predicate(p) if expected.contains(p.as_str()))
}

/// The EXPECTED DERIVED TRIPLES for the predicates `expected`, read over `data`.
///
/// SHACL 1.2 Inference Rules, "Expected Derived Triples": "For a given predicate p, the
/// derived value nodes are all value nodes that can be computed using sh:defaultValue and
/// sh:values as defined by SHACL 1.2 Core in any (non-deactivated) property shape that
/// uses p as sh:path in the shapes graph. For these derived value nodes v the derived
/// triples are the triples where v is the object, p is the predicate and the subjects are
/// the target nodes of the property shapes."
///
/// A property shape's target nodes are the focus nodes its declaring shape's targets
/// select — in the specification's own example the property shape carries no target and
/// its node shape `sh:targetClass ex:Rectangle` supplies the subjects — so each
/// non-deactivated top-level shape's targets are resolved and its property shapes' value
/// nodes computed there, by the one "Value Nodes of Property Shapes" rule validation
/// applies. The path's own value nodes are already triples of the graph, so only the
/// computed ones are new. A literal target node cannot be the subject of a triple, so it
/// has no derived triple.
///
/// # Errors
///
/// Returns an error when target resolution or an expression evaluation fails.
pub(crate) fn expected_derived_triples(
    data: &ShaclData,
    shapes: &Shapes,
    expected: &FastSet<&str>,
) -> Result<Vec<[Term; 3]>, String> {
    let mut out: Vec<[Term; 3]> = Vec::new();
    for shape in &shapes.node_shapes {
        if shape.deactivated
            || !shape
                .property_shapes
                .iter()
                .any(|ps| derives_expected(ps, expected))
        {
            continue;
        }
        let lowered = crate::plan::lower_shapes(std::iter::once(shape));
        let binding = lowered.bind(data.core_view(), lowered.classes());
        let focus_nodes =
            resolve_focus_nodes(data, &shape.id, &shape.targets, &binding, lowered.classes())?;
        let plan = lowered.plan(shape, 0, &binding, lowered.classes(), lowered.no_targets())?;
        for (ps, property_plan) in plan.properties()? {
            if !derives_expected(ps, expected) {
                continue;
            }
            let Path::Predicate(predicate) = &ps.path else {
                continue;
            };
            for focus in &focus_nodes {
                let focus = focus.to_term(data.core_view());
                if !focus.is_subject() {
                    continue;
                }
                for value in
                    crate::constraints::property_value_terms(data, &focus, ps, property_plan)?
                {
                    out.push([focus.clone(), Term::NamedNode(predicate.clone()), value]);
                }
            }
        }
    }
    Ok(out)
}

// ── Helpers ─────────────────────────────────────────────────────────────────────

/// The one lowering a rule execution needs: the rule's shape AND every `sh:condition`
/// shape it will check, lowered and bound once.
///
/// The conditions follow the rule's shape in the lowering, so condition `i` is plan
/// position `i + 1`.
struct RulePlan<'a> {
    lowered: crate::plan::LoweredShapes,
    binding: crate::plan::DatasetBinding,
    shape: &'a Shape,
    conditions: &'a [Shape],
}

impl<'a> RulePlan<'a> {
    /// Lower the rule's shape TOGETHER WITH its conditions and bind the lot to `data`,
    /// once per execution. One lowering rather than several is what fixes the plan
    /// positions the rest of this type relies on.
    fn of(data: &ShaclData, shape: &'a Shape, conditions: &'a [Shape]) -> Self {
        let lowered = crate::plan::lower_shapes(std::iter::once(shape).chain(conditions));
        let binding = lowered.bind(data.core_view(), lowered.classes());
        Self {
            lowered,
            binding,
            shape,
            conditions,
        }
    }

    /// Resolve the focus nodes of the rule's shape against the current dataset.
    fn focus_nodes(&self, data: &ShaclData) -> Result<Vec<Term>, String> {
        resolve_focus_nodes(
            data,
            &self.shape.id,
            &self.shape.targets,
            &self.binding,
            self.lowered.classes(),
        )
        .map(|nodes| {
            // The rules engine drives the owned-term SHACL-AF surfaces, so this is one of
            // the boundaries that really does need every focus node materialized.
            nodes
                .into_iter()
                .map(|node| node.to_term(data.core_view()))
                .collect()
        })
    }

    /// The plan of the `position`-th `sh:condition` shape.
    fn condition(&self, position: usize) -> Result<crate::plan::ShapePlan<'_>, String> {
        self.lowered.plan(
            &self.conditions[position],
            position + 1,
            &self.binding,
            self.lowered.classes(),
            self.lowered.no_targets(),
        )
    }
}

/// One node expression of a rule head, lowered and bound once per execution.
struct ExprPlan<'a> {
    expr: &'a NodeExpr,
    lowering: crate::plan::StandaloneLowering,
    binding: crate::plan::DatasetBinding,
}

impl<'a> ExprPlan<'a> {
    /// Lower `expr` standalone and resolve every identity it names against `data`.
    fn of(data: &ShaclData, expr: &'a NodeExpr) -> Self {
        let lowering = crate::plan::lower_standalone_expression(expr);
        let binding = lowering.bind(data.core_view());
        Self {
            expr,
            lowering,
            binding,
        }
    }

    /// Evaluate the expression at `focus`. `guard` is supplied rather than created here
    /// because the three head expressions of one execution share a single budget.
    fn eval(
        &self,
        data: &ShaclData,
        focus: &Term,
        guard: &mut RecursionGuard,
    ) -> Result<Vec<Term>, String> {
        eval_planned_node_expr(
            data,
            focus,
            self.expr,
            self.lowering.expr(),
            self.lowering.plan(&self.binding),
            guard,
        )
    }
}

/// Whether `focus` conforms to every `sh:condition` shape.
///
/// An error from the conformance check itself propagates rather than being read as "the
/// condition did not hold" — a rule must never fire, or decline to fire, on a verdict
/// that was not computed.
fn conditions_hold(data: &ShaclData, focus: &Term, plan: &RulePlan<'_>) -> Result<bool, String> {
    for position in 0..plan.conditions.len() {
        if !conforms_with_plan(data, focus, plan.condition(position)?)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The blank-node mint prefix of one rule execution: the focus node's identity encoded
/// into the blank-node-label alphabet ([`focus_tag`]) followed by the execution number,
/// or `g` and the execution number for a global rule.
///
/// Every label the execution mints is spelled `{prefix}{label}` at mint time, so two
/// executions — the same focus node in two passes, or two focus nodes in one — never
/// mint the same blank node. The execution number follows `-x`, which the focus
/// encoding never produces (its `-` always opens a two-hex-digit escape), so the prefix
/// stays injective; it ends in `_`, a legal `BLANK_NODE_LABEL` and `rdf:nodeID` prefix
/// outside `BlankScope`'s reserved `purrdfesc` marker namespace.
fn mint_tag(focus: Option<&Term>, execution: u64) -> String {
    match focus {
        Some(focus) => {
            let mut tag = focus_tag(focus);
            tag.pop();
            format!("{tag}-x{execution}_")
        }
        None => format!("g-x{execution}_"),
    }
}

/// The focus node's identity encoded into the blank-node-label alphabet, followed by
/// the `_` separator.
///
/// Every ASCII-alphanumeric byte of the focus rendering passes through; every other
/// byte becomes `-` plus two lowercase hex digits. The encoding is injective: `-` itself
/// is escaped (`-2d`), so each `-` in the output opens a fixed-width escape and decoding
/// is unambiguous. `_` never appears ahead of the separator (escaped as `-5f`), and the
/// output matches `f[A-Za-z0-9-]*_`.
fn focus_tag(focus: &Term) -> String {
    let rendered = focus.to_string();
    let mut tag = String::with_capacity(rendered.len() * 3 + 2);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    tag.push('f');
    for byte in rendered.bytes() {
        if byte.is_ascii_alphanumeric() {
            tag.push(char::from(byte));
        } else {
            tag.push('-');
            tag.push(char::from(HEX[usize::from(byte >> 4)]));
            tag.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    tag.push('_');
    tag
}

/// Seed `builder` with the base projection's statements, keeping every blank's
/// (scope-qualified) label at the DEFAULT blank scope.
///
/// Deliberately NOT [`RdfDatasetBuilder::push_dataset`]: that standardizes the source
/// apart into a fresh blank scope, which is right when merging unrelated documents but
/// wrong here — the engine re-materializes the SAME document every time the evaluation
/// graph changes, and an inferred triple that references a base blank carries that
/// blank's label ([`push_fact`] pushes owned terms at the DEFAULT scope). Re-scoping
/// the base would sever exactly that co-reference.
pub(crate) fn push_projection(builder: &mut RdfDatasetBuilder, base: &RdfDataset) {
    for quad in base.owned_quads() {
        builder.push_owned_quad(&quad);
    }
    for reifier in base.owned_reifiers() {
        builder.push_owned_reifier(&reifier);
    }
    for annotation in base.owned_annotations() {
        builder.push_owned_annotation(&annotation);
    }
}

/// The base default-graph triples as owned terms — the RDF 1.2 statement layer
/// included, as `r rdf:reifies <<( s p o )>>` and annotation rows — in the dataset's
/// order.
pub(crate) fn base_triples(base: &RdfDataset) -> Vec<[Term; 3]> {
    let statement_layer = base
        .reifier_quads()
        .chain(base.annotation_quads())
        .filter(|quad| quad.g.is_none());
    quads_for_pattern_ids(base, None, None, None, GraphFilter::DefaultGraph)
        .chain(statement_layer)
        .map(|quad| {
            [
                term_id_to_native(base, quad.s),
                term_id_to_native(base, quad.p),
                term_id_to_native(base, quad.o),
            ]
        })
        .collect()
}

/// Every subject of a `r rdf:reifies <<( s p o )>>` fact: the reifiers of the graph,
/// whose other triples are their annotations.
pub(crate) fn reifier_subjects<'t>(facts: impl Iterator<Item = &'t [Term; 3]>) -> FastSet<Term> {
    facts
        .filter(|[_, p, o]| {
            matches!(p, Term::NamedNode(p) if p.as_str() == crate::model::rdf::REIFIES)
                && matches!(o, Term::Triple(_))
        })
        .map(|[r, _, _]| r.clone())
        .collect()
}

/// Push one owned triple into `builder`, into the layer RDF 1.2 puts it in:
/// `r rdf:reifies <<( s p o )>>` is a reifier declaration, a triple whose subject is one
/// of `reifiers` is that reifier's annotation, and every other triple is a quad — the
/// classification a parsed graph gets, so a rule's reification reads back through the
/// statement layer exactly as an asserted one does.
///
/// # Errors
///
/// An internal-invariant breach: a non-IRI predicate, which the producers never emit.
pub(crate) fn push_fact(
    builder: &mut RdfDatasetBuilder,
    triple: &[Term; 3],
    reifiers: &FastSet<Term>,
) -> Result<(), String> {
    let [s, p, o] = triple;
    let Term::NamedNode(predicate) = p else {
        return Err(format!(
            "internal error: inferred triple has non-IRI predicate {p}"
        ));
    };
    if predicate.as_str() == crate::model::rdf::REIFIES
        && let Term::Triple(statement) = o
    {
        builder.push_owned_reifier(&::purrdf::RdfReifier::new(
            s.to_rdf_term(),
            ::purrdf::RdfTriple::new(
                statement.subject.to_rdf_term(),
                statement.predicate.as_str(),
                statement.object.to_rdf_term(),
            ),
        ));
    } else if reifiers.contains(s) {
        builder.push_owned_annotation(&::purrdf::RdfAnnotation::new(
            s.to_rdf_term(),
            predicate.as_str(),
            o.to_rdf_term(),
        ));
    } else {
        builder.push_owned_quad(&RdfQuad::new(
            s.to_rdf_term(),
            predicate.as_str(),
            o.to_rdf_term(),
        ));
    }
    Ok(())
}

/// Build the evaluation dataset's SPARQL half: the projected data (default graph) plus
/// the shapes graph exposed as a named graph under `graph_iri`, when known — so a SPARQL
/// rule's CONSTRUCT sees the shapes graph under `$shapesGraph`. SHACL 1.2 Inference
/// Rules: "At no time are inferred triples visible to the shapes graph".
pub(crate) fn build_round_base(
    base: &Arc<RdfDataset>,
    shapes: &Shapes,
    graph_iri: Option<&str>,
) -> Result<Arc<RdfDataset>, String> {
    let Some(graph_iri) = graph_iri else {
        return Ok(Arc::clone(base));
    };

    let mut builder = RdfDatasetBuilder::new();
    push_projection(&mut builder, base.as_ref());

    // The shapes document's blanks are standardized apart from the data's (a fixed
    // non-default scope — deterministic, and disjoint from the DEFAULT scope the base and
    // the inferred triples share): two documents' same-label blanks must never co-refer.
    let graph_term = RdfTerm::iri(graph_iri);
    for mut quad in shapes.shapes_dataset.owned_quads() {
        quad.graph_name = Some(graph_term.clone());
        builder.push_owned_quad_scoped(&quad, SHAPES_BLANK_SCOPE);
    }

    builder.freeze().map_err(|e| e.to_string())
}

/// Validate a SPARQL rule's CONSTRUCT at load: it parses, is a CONSTRUCT, names no
/// target graph, and — for a shape rule, and for any pre-bound template parameter —
/// meets the pre-binding restrictions.
///
/// SHACL 1.2 Inference Rules: "For SPARQL rules that are shape rules (i.e., linked to a
/// shape with sh:rule), a SHACL rules engine also counts as a SHACL-SPARQL processor
/// […] and the syntax limitations required by pre-binding do apply to shape rules. Since
/// global SPARQL rules do not use pre-binding, the syntax limitations required by
/// pre-binding do not apply to them."
///
/// A template that names a graph is refused: a SHACL rule derives triples into the data
/// graph — the one graph targets, paths, conditions and every other rule read — so a
/// head naming another graph would produce output no shape could observe.
///
/// # Errors
///
/// A message naming the rule and the defect.
pub(crate) fn check_construct(
    rule_node: &Term,
    construct: &str,
    prebound: &[&str],
) -> Result<(), String> {
    let query = SparqlParser::new().parse_query(construct).map_err(|e| {
        format!("SPARQL rule {rule_node} has an unparsable sh:construct query: {e}")
    })?;
    let Query::Construct { template, .. } = &query else {
        return Err(format!("SPARQL rule {rule_node} must be a CONSTRUCT query"));
    };
    if let Some(graph) = template.iter().find_map(|quad| quad.graph.as_ref()) {
        let graph = match graph {
            purrdf_sparql_algebra::NamedNodePattern::NamedNode(n) => format!("<{}>", n.as_str()),
            purrdf_sparql_algebra::NamedNodePattern::Variable(v) => format!("?{}", v.as_str()),
        };
        return Err(format!(
            "SPARQL rule {rule_node} uses CONSTRUCT GRAPH {graph}; a SHACL rule head produces \
             triples inferred into the data graph and cannot target a named graph"
        ));
    }
    if !prebound.is_empty() {
        crate::prebinding::check_construct(&query, prebound)
            .map_err(|e| format!("SPARQL rule {rule_node}: {e}"))?;
    }
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;
    use crate::shapes::Shapes;

    const PREFIXES: &str = r"
        @prefix sh:   <http://www.w3.org/ns/shacl#> .
        @prefix ex:   <http://example.org/ns#> .
        @prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
    ";

    fn parse_shapes(body: &str) -> Shapes {
        crate::engine::parse_shapes(&format!("{PREFIXES}\n{body}"), None)
            .expect("shapes must parse")
    }

    fn parse_shapes_err(body: &str) -> String {
        crate::engine::parse_shapes(&format!("{PREFIXES}\n{body}"), None)
            .expect_err("shapes must fail to parse")
    }

    fn entail(data_ttl: &str, shapes_body: &str) -> Arc<RdfDataset> {
        let data =
            crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}\n{data_ttl}"), None)
                .expect("data must parse");
        let shapes = parse_shapes(shapes_body);
        entail_dataset(data.as_ref(), &shapes).expect("entailment must succeed")
    }

    /// Entail under explicit `options`.
    fn entail_with(
        data_ttl: &str,
        shapes_body: &str,
        options: &RuleOptions,
    ) -> Result<Arc<RdfDataset>, String> {
        let data =
            crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}\n{data_ttl}"), None)
                .expect("data must parse");
        let projected = crate::engine::project_dataset(data.as_ref()).expect("projects");
        let holder = ShaclData::new(Arc::clone(&projected), projected, None);
        let shapes = parse_shapes(shapes_body);
        infer(&holder, &shapes, options).map(|inference| Arc::clone(inference.dataset()))
    }

    /// Every default-graph triple of `ds` as `(s, p, o)` N-Triples strings.
    fn triples(ds: &RdfDataset) -> Vec<(String, String, String)> {
        quads_for_pattern_ids(ds, None, None, None, GraphFilter::DefaultGraph)
            .map(|q| {
                (
                    term_id_to_native(ds, q.s).to_string(),
                    term_id_to_native(ds, q.p).to_string(),
                    term_id_to_native(ds, q.o).to_string(),
                )
            })
            .collect()
    }

    fn ex(local: &str) -> String {
        format!("<http://example.org/ns#{local}>")
    }

    /// Whether the dataset asserts `(s, p, o)` in IRI shorthand.
    fn has_iri(ds: &RdfDataset, s: &str, p: &str, o: &str) -> bool {
        triples(ds).contains(&(ex(s), ex(p), ex(o)))
    }

    fn canon(ds: &RdfDataset) -> String {
        ::purrdf::canonicalize(ds).nquads
    }

    // ── Parsing ────────────────────────────────────────────────────────────────

    #[test]
    fn parses_triple_rule() {
        let shapes = parse_shapes(
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ;
                        sh:subject sh:this ;
                        sh:predicate ex:adult ;
                        sh:object true ] .",
        );
        let shape = shapes
            .node_shapes
            .iter()
            .find(|s| !s.rules.is_empty())
            .expect("a shape with a rule");
        assert_eq!(shape.rules.len(), 1);
        assert!(matches!(shape.rules[0].body, RuleBody::Triple { .. }));
        assert!(!shape.rules[0].deactivated);
        assert!(shape.rules[0].order.is_none());
        assert!(shape.rules[0].conditions.is_empty());
    }

    #[test]
    fn parses_sparql_rule() {
        let shapes = parse_shapes(
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:SPARQLRule ;
                        sh:construct "CONSTRUCT { $this ex:adult true } WHERE { $this a ex:Person }" ] ."#,
        );
        let shape = shapes
            .node_shapes
            .iter()
            .find(|s| !s.rules.is_empty())
            .expect("a shape with a rule");
        assert!(matches!(shape.rules[0].body, RuleBody::Sparql { .. }));
    }

    #[test]
    fn parses_order_deactivated_and_conditions() {
        let shapes = parse_shapes(
            r"
            ex:HasName a sh:NodeShape ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ;
                        sh:order 3 ;
                        sh:deactivated true ;
                        sh:condition ex:HasName ;
                        sh:subject sh:this ; sh:predicate ex:p ; sh:object ex:o ] .",
        );
        let rule = shapes
            .node_shapes
            .iter()
            .flat_map(|s| &s.rules)
            .next()
            .expect("a rule");
        assert!((rule.order.expect("order").value() - 3.0).abs() < f64::EPSILON);
        assert!(rule.deactivated);
        assert_eq!(rule.conditions.len(), 1);
        // The condition is a RESOLVED shape, not the node term it was written as:
        // it carries the property shape `ex:HasName` declares, so the rule holds
        // everything it needs to evaluate the condition without a later lookup.
        assert_eq!(rule.conditions[0].id.to_string(), ex("HasName"));
        assert_eq!(
            rule.conditions[0].property_shapes.len(),
            1,
            "the referenced shape's sh:property was parsed into the condition"
        );
    }

    #[test]
    fn malformed_rule_unknown_kind_errors() {
        let err = parse_shapes_err(
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ ex:foo ex:bar ] .",
        );
        assert!(err.contains("not a recognised SHACL rule"), "got: {err}");
    }

    /// SHACL 1.2 Inference Rules: "Where sh:subject, sh:predicate, or sh:object are
    /// absent, use the list consisting of the current focus node". A second value of one
    /// is refused: "Each triple rule must have at most one value of the property".
    #[test]
    fn an_absent_triple_rule_position_is_the_focus_node() {
        let out = entail(
            "ex:alice a ex:Person .",
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:predicate ex:self ] .",
        );
        assert!(has_iri(&out, "alice", "self", "alice"));
        let err = parse_shapes_err(
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:predicate ex:p , ex:q ] .",
        );
        assert!(err.contains("more than one value"), "got: {err}");
    }

    #[test]
    fn ambiguous_rule_both_kinds_errors() {
        let err = parse_shapes_err(
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule , sh:SPARQLRule ;
                        sh:subject sh:this ; sh:predicate ex:p ; sh:object ex:o ;
                        sh:construct "CONSTRUCT { $this ex:x ex:y } WHERE { $this a ex:Person }" ] ."#,
        );
        assert!(err.contains("ambiguous"), "got: {err}");
    }

    /// "Each SHACL rule has at least one rdf:type which is an IRI", and "If a rules
    /// engine is not able to execute a given rule because it does not support any of
    /// the rule types of the rule, then it reports a failure": an UNTYPED rule node is
    /// refused even when its properties look like a triple rule's. The typed neighbour
    /// loads ([`parses_triple_rule`]).
    #[test]
    fn an_untyped_rule_is_refused() {
        let err = parse_shapes_err(
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ sh:subject sh:this ; sh:predicate ex:p ; sh:object ex:o ] .",
        );
        assert!(err.contains("not a recognised SHACL rule"), "got: {err}");
    }

    #[test]
    fn non_numeric_order_errors() {
        let err = parse_shapes_err(
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:order "soon" ;
                        sh:subject sh:this ; sh:predicate ex:p ; sh:object ex:o ] ."#,
        );
        assert!(err.contains("shacl#order"), "got: {err}");
    }

    #[test]
    fn sparql_rule_non_construct_query_errors() {
        let err = parse_shapes_err(
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:SPARQLRule ; sh:construct "SELECT $this WHERE { $this a ex:Person }" ] ."#,
        );
        assert!(err.contains("must be a CONSTRUCT"), "got: {err}");
    }

    /// The quad-producing `CONSTRUCT` parses as a `CONSTRUCT`, but a SHACL rule
    /// head produces TRIPLES inferred into the data graph — `sparql_rule_producer`
    /// returns `[Term; 3]` and has nowhere to put a graph name. Accepting one
    /// would silently discard the named graph, so the rule loader refuses EVERY
    /// spelling that names a graph and names the graph it refused: the
    /// whole-template shorthand, a `GRAPH` block inside the template, a graph
    /// block that scopes only PART of the template, and a graph VARIABLE.
    #[test]
    fn sparql_rule_construct_graph_errors() {
        for (construct, named) in [
            (
                "CONSTRUCT GRAPH <http://example.org/g> { $this ex:x ex:y } \
                 WHERE { $this a ex:Person }",
                "<http://example.org/g>",
            ),
            (
                "CONSTRUCT { GRAPH <http://example.org/g> { $this ex:x ex:y } } \
                 WHERE { $this a ex:Person }",
                "<http://example.org/g>",
            ),
            (
                "CONSTRUCT { $this ex:x ex:y . GRAPH <http://example.org/g> { $this ex:x ex:z } } \
                 WHERE { $this a ex:Person }",
                "<http://example.org/g>",
            ),
            (
                "CONSTRUCT { GRAPH ?g { $this ex:x ex:y } } \
                 WHERE { $this a ex:Person . GRAPH ?g { $this a ex:Person } }",
                "?g",
            ),
        ] {
            let err = parse_shapes_err(&format!(
                r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:SPARQLRule ; sh:construct "{construct}" ] ."#
            ));
            assert!(err.contains("CONSTRUCT GRAPH"), "got: {err}");
            assert!(
                err.contains(named),
                "the diagnostic must name `{named}`, got: {err}"
            );
        }
    }

    /// The counterpart that keeps the refusal falsifiable: a `sh:SPARQLRule`
    /// whose template names NO graph is still accepted.
    #[test]
    fn sparql_rule_without_a_target_graph_loads() {
        let shapes = parse_shapes(
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:SPARQLRule ; sh:construct
                "CONSTRUCT { $this ex:x ex:y } WHERE { $this a ex:Person }" ] ."#,
        );
        assert_eq!(shapes.node_shapes.len(), 1);
    }

    #[test]
    fn sparql_rule_illegal_prebinding_errors() {
        // MINUS is forbidden under pre-binding; must be rejected at load.
        let err = parse_shapes_err(
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:SPARQLRule ; sh:construct
                "CONSTRUCT { $this ex:x ex:y } WHERE { $this a ex:Person MINUS { $this a ex:Robot } }" ] ."#,
        );
        assert!(err.contains("MINUS"), "got: {err}");
    }

    // ── TripleRule execution ─────────────────────────────────────────────────────

    #[test]
    fn single_triple_rule_derives_head() {
        let out = entail(
            "ex:alice a ex:Person .",
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ;
                        sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .",
        );
        assert!(has_iri(&out, "alice", "adult", "yes"));
        // The base triple survives.
        assert!(triples(&out).contains(&(
            ex("alice"),
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>".to_owned(),
            ex("Person"),
        )));
    }

    #[test]
    fn triple_rule_cartesian_product() {
        // subject=path ex:child (two values), object=this → two derived triples.
        let out = entail(
            "ex:p a ex:Parent ; ex:child ex:a, ex:b .",
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Parent ;
              sh:rule [ a sh:TripleRule ;
                        sh:subject [ sh:path ex:child ] ;
                        sh:predicate ex:childOf ;
                        sh:object sh:this ] .",
        );
        assert!(has_iri(&out, "a", "childOf", "p"));
        assert!(has_iri(&out, "b", "childOf", "p"));
    }

    /// SHACL 1.2 Inference Rules, "Execution of triple rules": "Skip ill-formed
    /// triples, for example when a blank node is used as predicate." A literal subject
    /// or a literal predicate infers nothing, while the well-formed combinations of the
    /// same rule are still inferred.
    #[test]
    fn triple_rule_ill_formed_combinations_are_skipped() {
        for (label, head) in [
            (
                "literal subject",
                r#"sh:subject ( "notasubject" ex:bob ) ; sh:predicate ex:p ; sh:object ex:o"#,
            ),
            (
                "literal predicate",
                r#"sh:subject ex:bob ; sh:predicate ( "notapred" ex:p ) ; sh:object ex:o"#,
            ),
        ] {
            let out = entail(
                "ex:alice a ex:Person .",
                &format!(
                    "ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                       sh:rule [ a sh:TripleRule ; {head} ] ."
                ),
            );
            let derived: Vec<(String, String, String)> = triples(&out)
                .into_iter()
                .filter(|(_, _, o)| *o == ex("o"))
                .collect();
            assert_eq!(
                derived,
                vec![(ex("bob"), ex("p"), ex("o"))],
                "{label}: only the well-formed combination is inferred"
            );
        }
    }

    // ── Node-expression audit (every normative kind × rule head position) ────────
    //
    // A *confirmed audit* that EVERY normative SHACL node-expression kind works in
    // the `sh:subject` / `sh:predicate` / `sh:object` positions of a
    // `sh:TripleRule`. All kinds route through the shared `eval_node_expr`; the
    // audit proves each one end-to-end in a rule head:
    //
    //   NodeExpr variant  object   subject   predicate  legality (subj/pred)
    //   Constant(IRI)       ✓        ✓          ✓
    //   Constant(literal)   ✓                              hard-fail (existing)
    //   This                ✓        ✓          ✓
    //   Path                ✓        ✓          ✓
    //   Filter              ✓        ✓          ✓
    //   Union               ✓        ✓          ✓
    //   Intersection        ✓        ✓          ✓
    //   If                  ✓        ✓          ✓
    //   Count               ✓                              hard-fail
    //   Count(distinct)     ✓
    //   Distinct            ✓        ✓          ✓
    //   Min                 ✓        ✓ (IRI)    ✓ (IRI)
    //   Max                 ✓        ✓ (IRI)    ✓ (IRI)
    //   Sum                 ✓                              hard-fail
    //   Limit               ✓        ✓          ✓
    //   Offset              ✓        ✓          ✓
    //   OrderBy             ✓        ✓          ✓
    //   Exists              ✓                              hard-fail
    //   Call::Builtin       ✓                              hard-fail (literal cast)
    //   Call::UserDefined   ✓        ✓          ✓ (IRI-returning fn)

    /// Focus `ex:a` (class `ex:Root`) with IRI-valued edges (`ex:p → {ex:b, ex:c}`),
    /// numeric edges (`ex:n → {1, 2}`), and `ex:b` marked so a filter shape can
    /// select it.
    const AUDIT_DATA: &str = "ex:a a ex:Root ; ex:p ex:b, ex:c ; ex:n 1, 2 . ex:b a ex:Keep .";

    /// A `sh:SPARQLFunction` returning a fixed IRI (`ex:derived`) regardless of its
    /// single argument — the IRI-yielding surface used to exercise a user-defined
    /// function call (`FnCall::UserDefined`) in subject/predicate position, where
    /// the head term must be an IRI.
    const MK_IRI_FN: &str = r#"
        ex:mkIri a sh:SPARQLFunction ;
          sh:parameter [ sh:path ex:arg ] ;
          sh:select "SELECT (IRI(\"http://example.org/ns#derived\") AS ?out) WHERE {}" .
    "#;

    fn int(n: &str) -> String {
        format!("\"{n}\"^^<http://www.w3.org/2001/XMLSchema#integer>")
    }

    fn boolean(b: bool) -> String {
        format!("\"{b}\"^^<http://www.w3.org/2001/XMLSchema#boolean>")
    }

    /// Entail [`AUDIT_DATA`] under a single `sh:TripleRule` whose head is
    /// (`subject`, `predicate`, `object`), returning every derived default-graph
    /// triple. `extra` injects auxiliary shapes-graph declarations (e.g. a
    /// `sh:SPARQLFunction`).
    fn entail_rule(
        subject: &str,
        predicate: &str,
        object: &str,
        extra: &str,
    ) -> Vec<(String, String, String)> {
        let out = entail(
            AUDIT_DATA,
            &format!(
                "{extra}
                 ex:S a sh:NodeShape ; sh:targetClass ex:Root ;
                   sh:rule [ a sh:TripleRule ;
                             sh:subject {subject} ; sh:predicate {predicate} ;
                             sh:object {object} ] ."
            ),
        );
        triples(&out)
    }

    /// The IRI-yielding kinds (usable in subject/predicate position): `(label,
    /// node-expression Turtle, expected head IRI N-Triples, extra shapes)`.
    fn iri_yielding_cases() -> Vec<(&'static str, &'static str, String, &'static str)> {
        vec![
            ("This", "sh:this", ex("a"), ""),
            ("Constant(IRI)", "ex:z", ex("z"), ""),
            ("Path", "[ sh:path ex:p ]", ex("b"), ""),
            (
                "Filter",
                "[ sh:filterShape [ sh:in ( ex:b ) ] ; sh:nodes [ sh:path ex:p ] ]",
                ex("b"),
                "",
            ),
            (
                "Union",
                "[ sh:union ( [ sh:path ex:p ] ex:z ) ]",
                ex("z"),
                "",
            ),
            (
                "Intersection",
                "[ sh:intersection ( [ sh:path ex:p ] [ sh:path ex:p ] ) ]",
                ex("b"),
                "",
            ),
            (
                "If",
                "[ sh:if [ sh:exists [ sh:path ex:p ] ] ; sh:then ex:yes ; sh:else ex:no ]",
                ex("yes"),
                "",
            ),
            ("Distinct", "[ sh:distinct [ sh:path ex:p ] ]", ex("b"), ""),
            ("Min(IRI)", "[ sh:min [ sh:path ex:p ] ]", ex("b"), ""),
            ("Max(IRI)", "[ sh:max [ sh:path ex:p ] ]", ex("c"), ""),
            ("Limit", "[ sh:path ex:p ; sh:limit 1 ]", ex("b"), ""),
            ("Offset", "[ sh:path ex:p ; sh:offset 1 ]", ex("c"), ""),
            (
                "OrderBy",
                "[ sh:path ex:p ; sh:orderby sh:this ]",
                ex("b"),
                "",
            ),
            (
                "Call(user-defined)",
                "[ ex:mkIri ( sh:this ) ]",
                ex("derived"),
                MK_IRI_FN,
            ),
        ]
    }

    /// Every normative node-expression kind works positively in **object**
    /// position of a `sh:TripleRule` head — including the function-call kinds
    /// (`FnCall::Builtin` and `FnCall::UserDefined`) and the filter-shape kind.
    #[test]
    fn audit_object_position_every_node_expr_kind() {
        // (label, object node-expression, expected object N-Triples, extra shapes).
        let cases: Vec<(&str, &str, String, &str)> = vec![
            ("Constant(IRI)", "ex:z", ex("z"), ""),
            ("Constant(literal)", "\"lit\"", "\"lit\"".to_owned(), ""),
            ("This", "sh:this", ex("a"), ""),
            ("Path", "[ sh:path ex:p ]", ex("b"), ""),
            (
                "Filter",
                "[ sh:filterShape [ sh:in ( ex:b ) ] ; sh:nodes [ sh:path ex:p ] ]",
                ex("b"),
                "",
            ),
            (
                "Union",
                "[ sh:union ( [ sh:path ex:p ] ex:z ) ]",
                ex("z"),
                "",
            ),
            (
                "Intersection",
                "[ sh:intersection ( [ sh:path ex:p ] [ sh:path ex:p ] ) ]",
                ex("b"),
                "",
            ),
            (
                "If",
                "[ sh:if [ sh:exists [ sh:path ex:p ] ] ; sh:then ex:yes ; sh:else ex:no ]",
                ex("yes"),
                "",
            ),
            ("Count", "[ sh:count [ sh:path ex:n ] ]", int("2"), ""),
            (
                "Count(distinct)",
                "[ sh:count [ sh:distinct [ sh:path ex:n ] ] ]",
                int("2"),
                "",
            ),
            ("Distinct", "[ sh:distinct [ sh:path ex:p ] ]", ex("b"), ""),
            ("Min", "[ sh:min [ sh:path ex:n ] ]", int("1"), ""),
            ("Max", "[ sh:max [ sh:path ex:n ] ]", int("2"), ""),
            ("Sum", "[ sh:sum [ sh:path ex:n ] ]", int("3"), ""),
            ("Limit", "[ sh:path ex:p ; sh:limit 1 ]", ex("b"), ""),
            ("Offset", "[ sh:path ex:p ; sh:offset 1 ]", ex("c"), ""),
            (
                "OrderBy",
                "[ sh:path ex:p ; sh:orderby sh:this ]",
                ex("b"),
                "",
            ),
            (
                "Exists",
                "[ sh:exists [ sh:path ex:p ] ]",
                boolean(true),
                "",
            ),
            (
                // `xsd:string(<iri>)` casts to a simple literal (rendered without an
                // explicit `^^xsd:string` datatype in N-Triples).
                "Call(builtin)",
                "[ xsd:string ( [ sh:path ex:p ] ) ]",
                "\"http://example.org/ns#b\"".to_owned(),
                "",
            ),
            (
                "Call(user-defined)",
                "[ ex:mkIri ( sh:this ) ]",
                ex("derived"),
                MK_IRI_FN,
            ),
        ];
        for (label, obj, expected, extra) in cases {
            let objects: Vec<String> = entail_rule("sh:this", "ex:out", obj, extra)
                .into_iter()
                .filter(|(s, p, _)| *s == ex("a") && *p == ex("out"))
                .map(|(_, _, o)| o)
                .collect();
            assert!(
                objects.contains(&expected),
                "object kind {label} ({obj}) must derive {expected}; got {objects:?}"
            );
        }
    }

    /// Every IRI-yielding node-expression kind works positively in **subject**
    /// position of a `sh:TripleRule` head (the head subject must be an IRI/blank).
    #[test]
    fn audit_subject_position_iri_yielding_kinds() {
        for (label, expr, expected, extra) in iri_yielding_cases() {
            let subjects: Vec<String> = entail_rule(expr, "ex:out", "ex:marker", extra)
                .into_iter()
                .filter(|(_, p, o)| *p == ex("out") && *o == ex("marker"))
                .map(|(s, _, _)| s)
                .collect();
            assert!(
                subjects.contains(&expected),
                "subject kind {label} ({expr}) must derive subject {expected}; got {subjects:?}"
            );
        }
    }

    /// Every IRI-yielding node-expression kind works positively in **predicate**
    /// position of a `sh:TripleRule` head (the head predicate must be an IRI).
    #[test]
    fn audit_predicate_position_iri_yielding_kinds() {
        for (label, expr, expected, extra) in iri_yielding_cases() {
            let predicates: Vec<String> = entail_rule("sh:this", expr, "ex:marker", extra)
                .into_iter()
                .filter(|(s, _, o)| *s == ex("a") && *o == ex("marker"))
                .map(|(_, p, _)| p)
                .collect();
            assert!(
                predicates.contains(&expected),
                "predicate kind {label} ({expr}) must derive predicate {expected}; got {predicates:?}"
            );
        }
    }

    /// A node-expression kind whose result is a literal is well-formed only in
    /// **object** position: in subject or predicate position every combination is an
    /// ill-formed triple, which SHACL 1.2 Inference Rules skips ("Skip ill-formed
    /// triples"). Completes the audit for the literal-yielding kinds (`Count`, `Sum`,
    /// `Exists`, and a literal-casting `Call::Builtin`).
    #[test]
    fn audit_literal_only_kinds_are_skipped_in_subject_and_predicate() {
        // (label, node-expression yielding a literal, extra shapes).
        let literal_kinds: Vec<(&str, &str, &str)> = vec![
            ("Count", "[ sh:count [ sh:path ex:n ] ]", ""),
            ("Sum", "[ sh:sum [ sh:path ex:n ] ]", ""),
            ("Exists", "[ sh:exists [ sh:path ex:p ] ]", ""),
            ("Call(builtin literal)", "[ xsd:string ( sh:this ) ]", ""),
        ];
        for (label, expr, extra) in &literal_kinds {
            for derived in [
                entail_rule(expr, "ex:out", "ex:marker", extra),
                entail_rule("sh:this", expr, "ex:marker", extra),
            ] {
                assert!(
                    !derived.iter().any(|(_, _, o)| *o == ex("marker")),
                    "literal kind {label} in subject or predicate position infers nothing \
                     (\"Skip ill-formed triples\"): {derived:?}"
                );
            }
        }
    }

    // ── SPARQLRule execution ─────────────────────────────────────────────────────

    /// A rule's CONSTRUCT head may declare a reifier (`_:r rdf:reifies <<( … )>>`)
    /// and annotate it; both live in the CONSTRUCT graph's RDF 1.2 statement layer,
    /// not its quads, and both reach the entailed graph's statement layer — the
    /// reifier declared for the reified triple, the annotation on the reifier —
    /// rather than being dropped as the quads-only read did. A second rule then
    /// READS the reification back (`?r rdf:reifies ?t`), so a derived reifier is
    /// visible to rules exactly as an asserted one is. The control rule reads the
    /// same pattern over a graph with no reification and derives nothing.
    #[test]
    fn a_rule_derived_reification_reaches_the_statement_layer() {
        let shapes = r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:T ;
              sh:rule [ a sh:SPARQLRule ; sh:order 0 ;
                sh:runOnce true ; sh:construct """CONSTRUCT {
                    _:r rdf:reifies <<( $this ex:p ?o )>> . _:r ex:source "rule" }
                  WHERE { $this ex:p ?o }""" ] ;
              sh:rule [ a sh:SPARQLRule ; sh:order 1 ;
                sh:construct """CONSTRUCT { $this ex:reified true }
                  WHERE { ?r rdf:reifies <<( $this ex:p ?o )>> }""" ] .
        "#;
        let ds = entail("ex:a a ex:T ; ex:p ex:b .", shapes);
        assert_eq!(ds.reifiers().count(), 1, "{:?}", triples(&ds));
        let (reifier, _) = ds.reifiers().next().expect("one reifier");
        assert_eq!(ds.annotations_of(reifier).count(), 1);
        assert!(
            triples(&ds)
                .iter()
                .any(|(s, p, _)| *s == ex("a") && *p == ex("reified")),
            "{:?}",
            triples(&ds)
        );

        let control = entail(
            "ex:a a ex:T ; ex:p ex:b .",
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:T ;
              sh:rule [ a sh:SPARQLRule ;
                sh:construct """CONSTRUCT { $this ex:reified true }
                  WHERE { ?r rdf:reifies <<( $this ex:p ?o )>> }""" ] .
            "#,
        );
        assert_eq!(control.reifiers().count(), 0);
        assert!(
            !triples(&control)
                .iter()
                .any(|(_, p, _)| *p == ex("reified")),
            "{:?}",
            triples(&control)
        );
    }

    #[test]
    fn single_sparql_rule_derives_head() {
        crate::class_membership::reset_thread_index_builds();
        let out = entail(
            "ex:Leaf rdfs:subClassOf ex:Person . ex:alice a ex:Leaf .",
            r#"
            ex:S a sh:NodeShape ; sh:targetNode ex:alice ;
              sh:rule [ a sh:SPARQLRule ; sh:construct
                "CONSTRUCT { $this ex:adult ex:yes } WHERE { $this a ex:Person }" ] ."#,
        );
        assert!(has_iri(&out, "alice", "adult", "yes"));
        assert_eq!(
            crate::class_membership::thread_index_builds(),
            2,
            "the deriving round and terminating round each build one shared index"
        );
    }

    #[test]
    fn sparql_rule_prebinds_shapes_graph() {
        // A sh:SPARQLRule CONSTRUCT that derives its head ONLY when `$shapesGraph`
        // is pre-bound to the RIGHT graph IRI: it reads a marker triple attached to
        // `$currentShape` (living exclusively in the shapes graph) and then requires
        // the enclosing graph name to EQUAL `$shapesGraph`. If `$shapesGraph` is left
        // unbound (the bug), the `?g = $shapesGraph` filter compares against an
        // unbound value, yields no solution, and the head is never derived — so a
        // stray single named graph cannot mask the missing binding.
        let shapes_ttl = format!(
            "{PREFIXES}\n{}",
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              ex:marker ex:secret ;
              sh:rule [ a sh:SPARQLRule ; sh:construct
                "CONSTRUCT { $this ex:marked ?m } WHERE { GRAPH ?g { $currentShape ex:marker ?m } FILTER(?g = $shapesGraph) }" ] .
            "#
        );
        let shapes_dataset = crate::text_ingest::parse_turtle_to_dataset(&shapes_ttl, None)
            .expect("shapes must parse");
        let prefixes = crate::text_ingest::parse_turtle_document(&shapes_ttl, None)
            .expect("fixture parses")
            .prefixes;
        let shapes = crate::shapes::from_dataset_with_config_and_graph(
            &shapes_dataset,
            &prefixes,
            None,
            Some("http://example.org/shapes-graph".to_owned()),
        )
        .expect("shapes must parse");

        let data = crate::text_ingest::parse_turtle_to_dataset(
            &format!("{PREFIXES}\n ex:alice a ex:Person ."),
            None,
        )
        .expect("data must parse");
        let projected = crate::engine::project_dataset(data.as_ref()).expect("project");
        let holder = ShaclData::new(Arc::clone(&projected), Arc::clone(&projected), None);
        let out = apply_rules(&holder, &shapes).expect("entailment must succeed");

        // The head is derived — proving `$shapesGraph` (and `$currentShape`) were
        // pre-bound so the CONSTRUCT could reach `ex:S ex:marker ex:secret`.
        assert!(
            has_iri(&out, "alice", "marked", "secret"),
            "sh:SPARQLRule must derive ex:alice ex:marked ex:secret via $shapesGraph; got {:?}",
            triples(&out)
        );
        // The shapes graph must NOT leak into the entailed default graph.
        assert!(
            !has_iri(&out, "S", "marker", "secret"),
            "the shapes graph must stay a named graph, never leaking into the data graph"
        );
    }

    // ── Driver: fixpoint / conditions / order / deactivation ─────────────────────

    #[test]
    fn two_round_fixpoint_chain() {
        // Rule A: Person → a ex:Adult. Rule B: Adult → ex:status ex:verified. B can
        // only fire once A has produced the ex:Adult typing, i.e. a later round.
        let out = entail(
            "ex:alice a ex:Person .",
            r"
            ex:PersonRule a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate rdf:type ; sh:object ex:Adult ] .
            ex:AdultRule a sh:NodeShape ; sh:targetClass ex:Adult ;
              sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:status ; sh:object ex:verified ] .",
        );
        assert!(triples(&out).contains(&(
            ex("alice"),
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>".to_owned(),
            ex("Adult"),
        )));
        assert!(has_iri(&out, "alice", "status", "verified"));
    }

    #[test]
    fn condition_gates_rule_firing() {
        let shapes = r"
            ex:HasName a sh:NodeShape ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:condition ex:HasName ;
                        sh:subject sh:this ; sh:predicate ex:greeted ; sh:object ex:yes ] .";
        let out = entail(
            "ex:alice a ex:Person ; ex:name \"Alice\" .\n ex:bob a ex:Person .",
            shapes,
        );
        assert!(has_iri(&out, "alice", "greeted", "yes"), "alice conforms");
        assert!(
            !has_iri(&out, "bob", "greeted", "yes"),
            "bob lacks ex:name, condition fails"
        );
    }

    /// An ANONYMOUS inline condition shape gates the rule.
    ///
    /// `sh:condition [ sh:property [ … ] ]` is legal SHACL and is what the
    /// specification's own examples use, but a blank node is in no index of
    /// top-level shapes: resolving conditions by IRI string at firing time refused
    /// it outright. Resolving them at shapes-load makes it ordinary.
    #[test]
    fn an_inline_anonymous_condition_shape_gates_rule_firing() {
        let shapes = r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ;
                        sh:condition [ sh:property [ sh:path ex:name ; sh:minCount 1 ] ] ;
                        sh:subject sh:this ; sh:predicate ex:greeted ; sh:object ex:yes ] .";
        let out = entail(
            "ex:alice a ex:Person ; ex:name \"Alice\" .\n ex:bob a ex:Person .",
            shapes,
        );
        assert!(
            has_iri(&out, "alice", "greeted", "yes"),
            "alice has ex:name, so the inline condition holds"
        );
        assert!(
            !has_iri(&out, "bob", "greeted", "yes"),
            "bob lacks ex:name, so the inline condition fails and the rule must not fire"
        );
    }

    /// A top-level `sh:PropertyShape` is a legal condition, and it constrains the
    /// focus node's values along its own path.
    #[test]
    fn a_property_shape_condition_gates_rule_firing() {
        let shapes = r"
            ex:NameRequired a sh:PropertyShape ; sh:path ex:name ; sh:minCount 1 .
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:condition ex:NameRequired ;
                        sh:subject sh:this ; sh:predicate ex:greeted ; sh:object ex:yes ] .";
        let out = entail(
            "ex:alice a ex:Person ; ex:name \"Alice\" .\n ex:bob a ex:Person .",
            shapes,
        );
        assert!(
            has_iri(&out, "alice", "greeted", "yes"),
            "alice satisfies the property-shape condition"
        );
        assert!(
            !has_iri(&out, "bob", "greeted", "yes"),
            "bob does not, so the rule must not fire"
        );
    }

    /// A `sh:condition` naming something that is not a shape is a shapes-LOAD
    /// error even when the owning shape TARGETS NOTHING.
    ///
    /// This is the swallowed error the load-time resolution exists to kill.
    /// Resolving conditions when a rule fires meant the check ran only after the
    /// shape produced a focus node — so an untargeted shape never reached it, the
    /// graph loaded green, and the rule was free to entail as though a condition
    /// nobody could evaluate had held.
    #[test]
    fn an_unresolvable_condition_on_an_untargeted_shape_is_a_load_error() {
        let err = parse_shapes_err(
            r"
            ex:S a sh:NodeShape ;
              sh:rule [ a sh:TripleRule ; sh:condition ex:NotAShape ;
                        sh:subject sh:this ; sh:predicate ex:p ; sh:object ex:o ] .",
        );
        assert!(
            err.contains("does not resolve to a shape"),
            "the refusal must name the unresolvable condition: {err}"
        );
        assert!(err.contains("NotAShape"), "got: {err}");
    }

    /// The same refusal for a TARGETED shape, so the load-time check is not
    /// specific to the untargeted case.
    #[test]
    fn an_unresolvable_condition_on_a_targeted_shape_is_a_load_error() {
        let err = parse_shapes_err(
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:condition ex:NotAShape ;
                        sh:subject sh:this ; sh:predicate ex:p ; sh:object ex:o ] .",
        );
        assert!(err.contains("does not resolve to a shape"), "got: {err}");
    }

    /// A rule conditioned on ITS OWN shape resolves to that shape's real
    /// constraints, not to an empty stand-in.
    ///
    /// Load-time resolution has to reach a shape that is, at that moment, already
    /// being parsed. If the ordinary cycle guard answered with the empty
    /// stand-in shape, the condition would conform to everything and silently
    /// always hold — so this asserts the negative case actually fails.
    #[test]
    fn a_rule_conditioned_on_its_own_shape_sees_that_shape_s_constraints() {
        let shapes = r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:property [ sh:path ex:name ; sh:minCount 1 ] ;
              sh:rule [ a sh:TripleRule ; sh:condition ex:S ;
                        sh:subject sh:this ; sh:predicate ex:greeted ; sh:object ex:yes ] .";
        let out = entail(
            "ex:alice a ex:Person ; ex:name \"Alice\" .\n ex:bob a ex:Person .",
            shapes,
        );
        assert!(
            has_iri(&out, "alice", "greeted", "yes"),
            "alice conforms to ex:S, so the self-condition holds"
        );
        assert!(
            !has_iri(&out, "bob", "greeted", "yes"),
            "bob violates ex:S's own sh:minCount, so the self-condition must NOT hold"
        );
    }

    #[test]
    fn deactivated_rule_is_skipped() {
        let out = entail(
            "ex:alice a ex:Person .",
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:deactivated true ;
                        sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .",
        );
        assert!(!has_iri(&out, "alice", "adult", "yes"));
    }

    #[test]
    fn deactivated_shape_rules_are_skipped() {
        let out = entail(
            "ex:alice a ex:Person .",
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ; sh:deactivated true ;
              sh:rule [ a sh:TripleRule ;
                        sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .",
        );
        assert!(!has_iri(&out, "alice", "adult", "yes"));
    }

    #[test]
    fn sh_order_is_applied_and_result_is_order_independent() {
        // Two rules with explicit orders. A monotonic fixpoint is order-independent,
        // so swapping the orders must yield byte-identical entailment (proving the
        // order key is honored without corrupting the result).
        let data = "ex:alice a ex:Person .";
        let forward = entail(
            data,
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:order 1 ; sh:subject sh:this ; sh:predicate ex:a ; sh:object ex:x ] ;
              sh:rule [ a sh:TripleRule ; sh:order 2 ; sh:subject sh:this ; sh:predicate ex:b ; sh:object ex:y ] .",
        );
        let swapped = entail(
            data,
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:order 2 ; sh:subject sh:this ; sh:predicate ex:a ; sh:object ex:x ] ;
              sh:rule [ a sh:TripleRule ; sh:order 1 ; sh:subject sh:this ; sh:predicate ex:b ; sh:object ex:y ] .",
        );
        assert!(has_iri(&forward, "alice", "a", "x"));
        assert!(has_iri(&forward, "alice", "b", "y"));
        assert_eq!(canon(&forward), canon(&swapped));
    }

    // ── Ordered execution (SHACL 1.2 Inference Rules) ────────────────────────────

    /// The order-0 rule tags the focus node; the order-1 rule is gated on a shape
    /// demanding ZERO tags. "The inferred triples of one rule (or group of same-order
    /// rules) become immediately visible to the subsequent rule (or group of
    /// same-order rules) in the order", so the order-1 rule evaluates its
    /// `sh:condition` over the tag, the condition FAILS and the order-1 head is ABSENT.
    ///
    /// A single global fixpoint over one flat rule list evaluates both rules against
    /// the same graph, sees an untagged focus node, and derives `ex:untagged true` —
    /// so the exact-set assertion below is what separates the two models.
    const LAYERED_GATING_DATA: &str = "ex:alice a ex:Person .";

    /// The gating shapes graph, parameterized on the two rules' `sh:order` values.
    fn layered_gating_shapes(tag_order: u32, gated_order: u32) -> String {
        format!(
            r"
            ex:Untagged a sh:NodeShape ; sh:property [ sh:path ex:tag ; sh:maxCount 0 ] .
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:order {tag_order} ;
                        sh:subject sh:this ; sh:predicate ex:tag ; sh:object ex:seen ] ;
              sh:rule [ a sh:TripleRule ; sh:order {gated_order} ; sh:condition ex:Untagged ;
                        sh:subject sh:this ; sh:predicate ex:untagged ; sh:object true ] ."
        )
    }

    /// The default-graph triples of `ds` as a sorted, deduplicated set.
    fn triple_set(ds: &RdfDataset) -> std::collections::BTreeSet<(String, String, String)> {
        triples(ds).into_iter().collect()
    }

    /// The same gating pair as [`layered_gating_shapes`], with NAMED rule nodes:
    /// when the two rules share an order they run CONCURRENTLY — "Rules with the same
    /// order are executed concurrently and must not see each other's inferences
    /// before they have all completed" — so the gated rule sees an untagged node and
    /// its head IS derived.
    fn same_group_probe_shapes(tag_order: &str, gated_order: &str) -> String {
        format!(
            r"
            ex:Untagged a sh:NodeShape ; sh:property [ sh:path ex:tag ; sh:maxCount 0 ] .
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ; sh:rule ex:rTag, ex:rGated .
            ex:rTag a sh:TripleRule ; sh:order {tag_order} ;
                    sh:subject sh:this ; sh:predicate ex:tag ; sh:object ex:seen .
            ex:rGated a sh:TripleRule ; sh:order {gated_order} ; sh:condition ex:Untagged ;
                    sh:subject sh:this ; sh:predicate ex:untagged ; sh:object true ."
        )
    }

    /// `sh:order -0.0` and `sh:order 0` are the SAME NUMBER, so the two rules
    /// share a group and compute the same inferences.
    ///
    /// `f64::total_cmp` — which the scheduler needs for its deterministic total
    /// order — separates `-0.0` from `0.0`. Since a group PARTITIONS execution,
    /// treating the two spellings as different orders would silently run one rule
    /// before the other and change the derived set. The probe makes that
    /// observable: sharing a group lets `ex:rGated` fire (both rules read the
    /// untagged node), whereas a spurious `-0.0 < 0.0` split runs `ex:rTag` first and
    /// the gated head disappears.
    #[test]
    fn negative_zero_order_shares_the_group_of_zero() {
        let plain = entail(LAYERED_GATING_DATA, &same_group_probe_shapes("0", "0"));
        let negative_zero = entail(LAYERED_GATING_DATA, &same_group_probe_shapes("-0.0", "0"));

        let gated_head = (
            ex("alice"),
            ex("untagged"),
            "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_owned(),
        );
        assert!(
            triple_set(&plain).contains(&gated_head),
            "baseline: rules sharing a group let the gated rule fire: {:?}",
            triple_set(&plain)
        );
        assert!(
            triple_set(&negative_zero).contains(&gated_head),
            "sh:order -0.0 must share the group of sh:order 0: {:?}",
            triple_set(&negative_zero)
        );
        assert_eq!(
            canon(&plain),
            canon(&negative_zero),
            "sh:order -0.0 and sh:order 0 must produce the identical closure"
        );
    }

    /// `sh:order -0.0` parses to the canonical `+0.0`, so nothing downstream can
    /// observe the sign of the zero.
    #[test]
    fn negative_zero_order_is_normalized_at_parse_time() {
        let shapes = parse_shapes(
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:order -0.0 ;
                        sh:subject sh:this ; sh:predicate ex:p ; sh:object ex:o ] .",
        );
        let rule = shapes
            .node_shapes
            .iter()
            .flat_map(|s| &s.rules)
            .next()
            .expect("a rule");
        let value = rule.order.expect("order").value();
        assert!(
            value.is_sign_positive(),
            "sh:order -0.0 must normalize to +0.0, got {value:?}"
        );
        assert_eq!(value.total_cmp(&0.0_f64), Ordering::Equal);
    }

    /// A non-finite `sh:order` has no position in the order (`NaN` is not
    /// even equal to itself), so it must be REFUSED rather than scheduled at an
    /// unresolvable position. `"NaN"` and `"inf"` both parse as `f64`, which is
    /// exactly why the check is explicit.
    #[test]
    fn non_finite_order_errors() {
        for spelling in ["NaN", "inf", "-inf"] {
            let err = parse_shapes_err(&format!(
                r#"
                ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                  sh:rule [ a sh:TripleRule ; sh:order "{spelling}"^^xsd:decimal ;
                            sh:subject sh:this ; sh:predicate ex:p ; sh:object ex:o ] ."#
            ));
            assert!(
                err.contains("xsd:decimal"),
                "sh:order \"{spelling}\" must be refused, got: {err}"
            );
        }
    }

    #[test]
    fn layered_execution_differs_from_flat_fixpoint() {
        let out = entail(LAYERED_GATING_DATA, &layered_gating_shapes(0, 1));
        let expected: std::collections::BTreeSet<(String, String, String)> = [
            (
                ex("alice"),
                "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>".to_owned(),
                ex("Person"),
            ),
            (ex("alice"), ex("tag"), ex("seen")),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            triple_set(&out),
            expected,
            "the order-1 head must be ABSENT: the order-0 tag was visible before \
             the order-1 rule checked ex:Untagged"
        );
    }

    /// Swapping the two `sh:order` values changes the INFERRED SET: with the gated
    /// rule at the LOWER order it fires (nothing has tagged the node yet) and the
    /// tag rule then also fires, so both heads are derived.
    ///
    /// This is the assertion that proves `sh:order` is no longer semantically
    /// inert — under a flat global fixpoint both spellings produce the same set.
    #[test]
    fn swapping_orders_changes_the_inferred_set() {
        let forward = entail(LAYERED_GATING_DATA, &layered_gating_shapes(0, 1));
        let swapped = entail(LAYERED_GATING_DATA, &layered_gating_shapes(1, 0));

        assert_ne!(
            canon(&forward),
            canon(&swapped),
            "swapping sh:order over a condition-gated rule must change the closure"
        );
        // And concretely: the gated head appears only when its rule runs FIRST.
        assert!(!has_iri(&forward, "alice", "untagged", "true"));
        assert!(
            triple_set(&swapped).contains(&(
                ex("alice"),
                ex("untagged"),
                "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_owned(),
            )),
            "with the gated rule at order 0 it fires: {:?}",
            triple_set(&swapped)
        );
        assert!(has_iri(&swapped, "alice", "tag", "seen"));
    }

    /// A head-minting `sh:SPARQLRule` with `sh:runOnce true` is evaluated exactly once
    /// at the start of its layer, so each focus node gets EXACTLY ONE minted resource
    /// — not one per pass, and not a ceiling error.
    #[test]
    fn once_rule_runs_exactly_once_and_terminates() {
        let out = entail(
            "ex:alice a ex:Person . ex:bob a ex:Person .",
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:SPARQLRule ; sh:runOnce true ; sh:construct
                "CONSTRUCT { $this ex:addr _:a . _:a ex:city ex:Metropolis } WHERE { $this a ex:Person }" ] ."#,
        );

        // One ex:addr edge per focus node, each to a DISTINCT minted subject.
        let addrs: Vec<(String, String)> = triples(&out)
            .into_iter()
            .filter(|(_, p, _)| *p == ex("addr"))
            .map(|(s, _, o)| (s, o))
            .collect();
        assert_eq!(addrs.len(), 2, "exactly one mint per focus node: {addrs:?}");
        let minted: FastSet<&String> = addrs.iter().map(|(_, o)| o).collect();
        assert_eq!(minted.len(), 2, "the two foci mint distinct blanks");
        let sources: FastSet<&String> = addrs.iter().map(|(s, _)| s).collect();
        assert_eq!(sources.len(), 2, "one edge from each focus node");

        // Each minted subject carries its whole head, exactly once.
        let cities: Vec<String> = triples(&out)
            .into_iter()
            .filter(|(_, p, o)| *p == ex("city") && *o == ex("Metropolis"))
            .map(|(s, _, _)| s)
            .collect();
        assert_eq!(cities.len(), 2, "one ex:city per minted blank: {cities:?}");
    }

    /// Entailment is byte-identical when the SAME shapes graph is spelled with its
    /// rule triples emitted in a DIFFERENT insertion order.
    ///
    /// The scheduler orders layers by `sh:layer`, groups by `sh:order` and rules within
    /// a group by rule-node identity, so nothing downstream may depend on the order the
    /// shapes document happened to assert its triples in. The proof is a byte
    /// comparison of the serialized N-Triples, not an isomorphism check.
    #[test]
    fn layered_entailment_is_byte_identical_under_insertion_permutation() {
        let data = "ex:alice a ex:Employee . ex:bob a ex:Employee .";

        // The same nine rule assertions, emitted in two different orders. Named
        // rule nodes keep the two spellings term-for-term identical, so any byte
        // difference is a scheduling artefact rather than a blank-label artefact.
        let forward = r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Employee ; sh:rule ex:r1, ex:r2, ex:r3 .
            ex:T a sh:NodeShape ; sh:targetClass ex:Person ; sh:rule ex:r4 .
            ex:r1 a sh:TripleRule ; sh:order 0 ; sh:subject sh:this ; sh:predicate rdf:type ; sh:object ex:Person .
            ex:r2 a sh:TripleRule ; sh:order 1 ; sh:subject sh:this ; sh:predicate ex:b ; sh:object ex:y .
            ex:r3 a sh:TripleRule ; sh:order 1 ; sh:subject sh:this ; sh:predicate ex:a ; sh:object ex:x .
            ex:r4 a sh:SPARQLRule ; sh:layer 1 ; sh:runOnce true ; sh:construct
                "CONSTRUCT { $this ex:addr _:a . _:a ex:city ex:Metropolis } WHERE { $this a ex:Person }" .
        "#;
        let permuted = r#"
            ex:r4 a sh:SPARQLRule ; sh:layer 1 ; sh:runOnce true ; sh:construct
                "CONSTRUCT { $this ex:addr _:a . _:a ex:city ex:Metropolis } WHERE { $this a ex:Person }" .
            ex:r3 a sh:TripleRule ; sh:order 1 ; sh:subject sh:this ; sh:predicate ex:a ; sh:object ex:x .
            ex:T a sh:NodeShape ; sh:targetClass ex:Person ; sh:rule ex:r4 .
            ex:r1 a sh:TripleRule ; sh:order 0 ; sh:subject sh:this ; sh:predicate rdf:type ; sh:object ex:Person .
            ex:S a sh:NodeShape ; sh:targetClass ex:Employee ; sh:rule ex:r3 .
            ex:r2 a sh:TripleRule ; sh:order 1 ; sh:subject sh:this ; sh:predicate ex:b ; sh:object ex:y .
            ex:S sh:rule ex:r2 .
            ex:S sh:rule ex:r1 .
        "#;

        let serialize = |shapes_body: &str| {
            let entailed = entail(data, shapes_body);
            ::purrdf::serialize_dataset(
                entailed.as_ref(),
                "application/n-triples",
                ::purrdf::SerializeGraph::Dataset,
            )
            .expect("N-Triples serialization must succeed")
        };

        let a = serialize(forward);
        let b = serialize(permuted);
        assert_eq!(
            std::str::from_utf8(&a).expect("N-Triples is UTF-8"),
            std::str::from_utf8(&b).expect("N-Triples is UTF-8"),
            "permuting the shapes graph's insertion order must not move a single \
             output byte"
        );
        // The permuted spelling really did exercise the whole layered pipeline.
        let entailed = entail(data, forward);
        assert!(has_iri(&entailed, "alice", "a", "x"));
        assert!(has_iri(&entailed, "alice", "b", "y"));
        assert_eq!(
            triples(&entailed)
                .into_iter()
                .filter(|(_, p, _)| *p == ex("addr"))
                .count(),
            2,
            "the layer-1 run-once rule saw layer 0's ex:Person typing"
        );
    }

    /// A rule is run-once exactly when it says so. SHACL 1.2 Inference Rules: "A rule
    /// that has true as its value for sh:runOnce is a run-once rule. Custom rule
    /// processors may compute additional run-once rules when sh:runOnce is absent" —
    /// and the standard processor computes none, so a CONSTRUCT template minting a
    /// blank node without `sh:runOnce` is an iterating rule.
    #[test]
    fn run_once_is_read_from_sh_run_once_only() {
        let run_once = |body: &str| {
            parse_shapes(body)
                .node_shapes
                .iter()
                .flat_map(|s| &s.rules)
                .map(|r| r.run_once)
                .next()
                .expect("a rule")
        };
        assert!(!run_once(
            r#"ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                 sh:rule [ a sh:SPARQLRule ; sh:construct
                   "CONSTRUCT { $this ex:addr _:a } WHERE { $this a ex:Person }" ] ."#
        ));
        assert!(run_once(
            r#"ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                 sh:rule [ a sh:SPARQLRule ; sh:runOnce true ; sh:construct
                   "CONSTRUCT { $this ex:addr _:a } WHERE { $this a ex:Person }" ] ."#
        ));
        assert!(!run_once(
            r"ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                sh:rule [ a sh:TripleRule ; sh:runOnce false ; sh:predicate ex:p ;
                          sh:object ex:o ] ."
        ));
        // "The values of sh:runOnce at rules are literals with datatype xsd:boolean."
        let err = parse_shapes_err(
            r#"ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                 sh:rule [ a sh:TripleRule ; sh:runOnce "yes" ; sh:predicate ex:p ;
                           sh:object ex:o ] ."#,
        );
        assert!(err.contains("sh:runOnce"), "{err}");
    }

    /// A `sh:layer` is a layer, `sh:order` an order inside it; both take
    /// `xsd:integer` or `xsd:decimal`, and a rule may carry at most one of each.
    #[test]
    fn layer_and_order_are_read_as_declared() {
        let rule = |body: &str| {
            parse_shapes(body)
                .node_shapes
                .into_iter()
                .flat_map(|s| s.rules)
                .next()
                .expect("a rule")
        };
        let declared = rule(
            r"ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                sh:rule [ a sh:TripleRule ; sh:layer 2 ; sh:order 1.5 ;
                          sh:predicate ex:p ; sh:object ex:o ] .",
        );
        assert!((declared.layer_value().value() - 2.0).abs() < f64::EPSILON);
        assert!((declared.order_value().value() - 1.5).abs() < f64::EPSILON);
        let defaulted = rule(
            r"ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                sh:rule [ a sh:TripleRule ; sh:predicate ex:p ; sh:object ex:o ] .",
        );
        assert!(defaulted.layer.is_none() && defaulted.order.is_none());
        assert!(defaulted.layer_value().value().abs() < f64::EPSILON);
        for bad in [
            r#"sh:layer "1""#,
            r#"sh:layer "1"^^xsd:double"#,
            "sh:layer 1 , 2",
            "sh:order 1 , 2",
        ] {
            let err = parse_shapes_err(&format!(
                "ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                   sh:rule [ a sh:TripleRule ; {bad} ; sh:predicate ex:p ; sh:object ex:o ] ."
            ));
            assert!(
                err.contains("layer") || err.contains("order"),
                "{bad}: {err}"
            );
        }
    }

    /// A bare blank node in a `sh:TripleRule` head position is a
    /// `shnex:EmptyExpression` (SHACL 1.2 Node Expressions §4.1.1), whose output
    /// nodes are the empty list — so the head produces NOTHING and no triple is
    /// inferred from it.
    #[test]
    fn a_bare_blank_in_a_triple_rule_head_produces_nothing() {
        for position in [
            "sh:subject _:x ; sh:predicate ex:p ; sh:object ex:o",
            "sh:subject sh:this ; sh:predicate ex:p ; sh:object _:x",
        ] {
            let body = format!(
                "ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
                   sh:rule [ a sh:TripleRule ; {position} ] ."
            );
            let shapes = parse_shapes(&body);
            let rule = shapes
                .node_shapes
                .iter()
                .flat_map(|s| &s.rules)
                .next()
                .expect("the shape declares one rule");
            let RuleBody::Triple {
                subject, object, ..
            } = &rule.body
            else {
                panic!("expected a sh:TripleRule head for {position}");
            };
            assert!(
                matches!(subject, Some(NodeExpr::Empty)) || matches!(object, Some(NodeExpr::Empty)),
                "a bare blank head position is an empty expression for {position}"
            );
            let out = entail("ex:alice a ex:Person .", &body);
            assert!(
                !triples(&out).iter().any(|(_, p, _)| *p == ex("p")),
                "an empty expression infers nothing for {position}"
            );
        }
    }

    // ── Termination ───────────────────────────────────────────────────────────────

    #[test]
    fn bounded_self_feeding_rule_converges() {
        // "ex:knows is symmetric": derive the reverse edge. Bounded (value-preserving
        // over {alice, bob}), so it reaches a fixpoint.
        let out = entail(
            "ex:alice ex:knows ex:bob .",
            r"
            ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:knows ;
              sh:rule [ a sh:TripleRule ;
                        sh:subject [ sh:path ex:knows ] ;
                        sh:predicate ex:knows ;
                        sh:object sh:this ] .",
        );
        assert!(has_iri(&out, "bob", "knows", "alice"));
        assert!(has_iri(&out, "alice", "knows", "bob"));
    }

    #[test]
    fn diverging_fresh_term_rule_errors() {
        // An iterating rule that mints a strictly LONGER IRI each pass: every pass's
        // fresh Counter becomes the next pass's focus, so fresh-term minting never
        // stops and the caller's term-generating round limit refuses the run, naming
        // the limit and how to raise it.
        let options = RuleOptions::default().with_max_term_generating_rounds(64);
        let err = entail_with(
            "ex:c0 a ex:Counter .",
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Counter ;
              sh:rule [ a sh:SPARQLRule ; sh:construct
                "CONSTRUCT { $this ex:next ?n . ?n a ex:Counter } WHERE { $this a ex:Counter . BIND(IRI(CONCAT(STR($this), \"0\")) AS ?n) }" ] ."#,
            &options,
        )
        .map(|_| ())
        .expect_err("a rule minting a longer IRI every pass diverges");
        assert!(
            err.contains("past the limit of 64 such rounds"),
            "got: {err}"
        );
        assert!(err.contains("65 rounds"), "got: {err}");
        assert!(
            err.contains("RuleOptions::with_max_term_generating_rounds"),
            "got: {err}"
        );
    }

    /// The SAME shape of self-feeding rule, but with the fresh term MINTED AS A
    /// BLANK NODE in the CONSTRUCT template and `sh:runOnce true`, terminates: a
    /// run-once rule is evaluated exactly once at the start of its layer, so the
    /// minted `ex:Counter` never becomes a focus node for a second firing. The
    /// assertion below pins the EXACT derived set rather than merely accepting `Ok`.
    #[test]
    fn head_minting_self_feeding_rule_runs_once_and_terminates() {
        let out = entail(
            "ex:c0 a ex:Counter .",
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Counter ;
              sh:rule [ a sh:SPARQLRule ; sh:runOnce true ; sh:construct
                "CONSTRUCT { $this ex:next _:n . _:n a ex:Counter } WHERE { $this a ex:Counter }" ] ."#,
        );
        // Exactly one minted Counter: `ex:c0 ex:next _:n` plus `_:n a ex:Counter`.
        let nexts: Vec<String> = triples(&out)
            .into_iter()
            .filter(|(s, p, _)| *s == ex("c0") && *p == ex("next"))
            .map(|(_, _, o)| o)
            .collect();
        assert_eq!(nexts.len(), 1, "one firing → one minted Counter: {nexts:?}");
        assert!(
            nexts[0].starts_with("_:"),
            "the minted term is a blank node"
        );
        let counters: Vec<String> = triples(&out)
            .into_iter()
            .filter(|(_, p, o)| {
                *p == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>" && *o == ex("Counter")
            })
            .map(|(s, _, _)| s)
            .collect();
        assert_eq!(
            counters.len(),
            2,
            "the base ex:c0 and exactly one minted Counter: {counters:?}"
        );
    }

    /// A run-once rule's minted blank nodes are ordinary terms of the graph the next
    /// layer reads, so a LATER layer that merely propagates them is not mistaken for
    /// a rule generating terms.
    ///
    /// Layer 1 here is strictly value-preserving over the minted terms: it only
    /// closes `ex:reaches` transitively over the minted chain, deriving no term that
    /// does not already exist. It needs one pass per chain link, and none of those
    /// passes commits a term the store did not already hold, so none counts against
    /// the term-generating round limit.
    ///
    /// The limit itself stays live: `diverging_fresh_term_rule_errors` pins that a
    /// rule minting a NEW term on every pass still reaches it.
    #[test]
    fn minted_terms_are_ordinary_terms_for_later_layers() {
        use std::fmt::Write as _;

        // One run-once firing mints a chain of M blank nodes, linked by ex:b and
        // seeded on ex:reaches for every direct link.
        const M: usize = 40;
        let mut template = String::from("$this ex:head _:n0 .");
        for i in 0..M - 1 {
            let j = i + 1;
            write!(template, " _:n{i} ex:b _:n{j} . _:n{i} ex:reaches _:n{j} .")
                .expect("write to String");
        }
        let shapes = format!(
            r#"
            ex:Mint a sh:NodeShape ; sh:targetClass ex:Seed ;
              sh:rule [ a sh:SPARQLRule ; sh:layer 0 ; sh:runOnce true ; sh:construct
                "CONSTRUCT {{ {template} }} WHERE {{ $this a ex:Seed }}" ] .
            ex:Close a sh:NodeShape ; sh:targetSubjectsOf ex:b ;
              sh:rule [ a sh:SPARQLRule ; sh:layer 1 ; sh:construct
                "CONSTRUCT {{ $this ex:reaches ?z }} WHERE {{ $this ex:b ?y . ?y ex:reaches ?z }}" ] ."#
        );

        // The closure needs one pass per chain link — about M of them — yet the run
        // completes under a limit far below M: only the one minting round counts.
        // Were the propagating passes counted, this limit would refuse the run.
        let options = RuleOptions::default().with_max_term_generating_rounds(4);
        let out = entail_with("ex:s0 a ex:Seed .", &shapes, &options)
            .expect("no propagating pass counts as term-generating");

        // And the closure is EXACT: every i<j pair over the minted chain, no more.
        let reaches: Vec<(String, String)> = triples(&out)
            .into_iter()
            .filter(|(_, p, _)| *p == ex("reaches"))
            .map(|(s, _, o)| (s, o))
            .collect();
        let distinct: std::collections::BTreeSet<&(String, String)> = reaches.iter().collect();
        assert_eq!(
            distinct.len(),
            M * (M - 1) / 2,
            "transitive closure over the minted chain must be exactly the i<j pairs"
        );
        // Every closure endpoint is a MINTED blank, not a base term.
        assert!(
            distinct
                .iter()
                .all(|(s, o)| s.starts_with("_:") && o.starts_with("_:")),
            "the closure must run entirely over run-once-minted blank nodes"
        );
    }

    // ── Determinism ───────────────────────────────────────────────────────────────

    #[test]
    fn entailment_is_byte_identical_across_runs() {
        let data = "ex:alice a ex:Person . ex:bob a ex:Person .";
        let shapes = r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .";
        assert_eq!(canon(&entail(data, shapes)), canon(&entail(data, shapes)));
    }

    #[test]
    fn blank_minting_rule_is_byte_stable() {
        let data = "ex:alice a ex:Person . ex:bob a ex:Person .";
        let shapes = r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:SPARQLRule ; sh:runOnce true ; sh:construct
                "CONSTRUCT { $this ex:addr _:a . _:a ex:city ex:Metropolis } WHERE { $this a ex:Person }" ] ."#;
        let first = entail(data, shapes);
        let second = entail(data, shapes);
        assert_eq!(canon(&first), canon(&second));
        // Distinct focus nodes get distinct minted blanks (two ex:addr edges to two
        // different blank subjects of ex:city).
        let addr_objects: Vec<String> = triples(&first)
            .into_iter()
            .filter(|(_, p, _)| *p == ex("addr"))
            .map(|(_, _, o)| o)
            .collect();
        assert_eq!(addr_objects.len(), 2, "two persons → two blanks");
        assert_ne!(
            addr_objects[0], addr_objects[1],
            "blanks are per-focus distinct"
        );
    }

    #[test]
    fn blank_focus_blank_minting_is_distinct_and_stable() {
        // Two DISTINCT blank nodes are the objects of ex:hasContact; a shape
        // targeting those blanks (sh:targetObjectsOf) runs a sh:SPARQLRule whose
        // CONSTRUCT mints a fresh blank per focus and links it. The mint-time
        // prefix tags the minted blank with `focus_tag(focus)`, which encodes the
        // focus's `_:...` rendering — a corner the IRI-focus minting test never
        // exercises.
        // We prove: (i) it works at all with a blank focus; (ii) the two blank foci
        // do NOT conflate (distinct minted blanks); (iii) re-derivation in a later
        // fixpoint round produces the identical label so the fixpoint converges and
        // output is byte-stable across independent runs.
        let data = "\
            ex:alice ex:hasContact [ a ex:Contact ] .\n\
            ex:bob   ex:hasContact [ a ex:Contact ] .";
        let shapes = r#"
            ex:S a sh:NodeShape ; sh:targetObjectsOf ex:hasContact ;
              sh:rule [ a sh:SPARQLRule ; sh:runOnce true ; sh:construct
                "CONSTRUCT { $this ex:addr _:a . _:a ex:city ex:Metropolis } WHERE { $this a ex:Contact }" ] ."#;

        // (iii) Byte-stable across two INDEPENDENT entailment runs. A blank focus that
        // re-minted a different label each round would diverge (never converge) or
        // differ run-to-run; identical canonical N-Quads proves stable re-derivation.
        let first = entail(data, shapes);
        let second = entail(data, shapes);
        assert_eq!(
            canon(&first),
            canon(&second),
            "blank-focus blank-minting must be byte-identical across runs"
        );

        // (ii) Two distinct blank foci → two ex:addr edges to two DISTINCT minted
        // blanks. If the per-focus tag failed to disambiguate (both foci minting the
        // same `_:c1`), the two edges would point at the SAME blank and these labels
        // would be equal — so `assert_ne!` genuinely catches conflation.
        let addr_objects: Vec<String> = triples(&first)
            .into_iter()
            .filter(|(_, p, _)| *p == ex("addr"))
            .map(|(_, _, o)| o)
            .collect();
        assert_eq!(
            addr_objects.len(),
            2,
            "two blank foci → two minted blanks; got {addr_objects:?}"
        );
        assert_ne!(
            addr_objects[0], addr_objects[1],
            "distinct blank foci must mint distinct blanks (no conflation)"
        );

        // (i) Each minted blank carries its ex:city structure — the CONSTRUCT head is
        // fully materialized per focus, not just the linking edge.
        let city_count = triples(&first)
            .into_iter()
            .filter(|(_, p, o)| *p == ex("city") && *o == ex("Metropolis"))
            .count();
        assert_eq!(
            city_count, 2,
            "each per-focus minted blank must carry its ex:city ex:Metropolis edge"
        );
    }

    #[test]
    fn value_preserving_chain_converges_over_many_passes() {
        // A transitive-closure chain over a path n0→n1→…→n(N-1) (via ex:next), seeding
        // ex:reaches on every DIRECT edge. The rule is strictly VALUE-PRESERVING: every
        // produced term already exists, so no pass is a term-generating round. The
        // extend-by-one-hop rule advances the reachability frontier by a single ex:next
        // step each pass, so the closure legitimately needs ~N passes — a genuine
        // multi-pass chain that must converge without reaching any ceiling.
        use std::fmt::Write as _;
        const N: usize = 8; // n0..n7 → 7 edges → ~6 rounds; closure = C(8,2) = 28 pairs.
        let mut data = String::new();
        for i in 0..N - 1 {
            let j = i + 1;
            writeln!(data, "ex:n{i} ex:next ex:n{j} .").expect("write to String");
            writeln!(data, "ex:n{i} ex:reaches ex:n{j} .").expect("write to String");
        }
        let shapes = r#"
            ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:next ;
              sh:rule [ a sh:SPARQLRule ; sh:construct
                "CONSTRUCT { $this ex:reaches ?z } WHERE { $this ex:next ?y . ?y ex:reaches ?z }" ] ."#;

        // `entail` unwraps the `Ok`, so a completed call is itself the assertion that
        // NO ceiling error was raised for this multi-pass value-preserving chain.
        let out = entail(&data, shapes);

        // The closure must be EXACT: ex:reaches holds for every ordered pair i<j and
        // for nothing else.
        let reaches: std::collections::BTreeSet<(String, String)> = triples(&out)
            .into_iter()
            .filter(|(_, p, _)| *p == ex("reaches"))
            .map(|(s, _, o)| (s, o))
            .collect();
        let mut expected: std::collections::BTreeSet<(String, String)> =
            std::collections::BTreeSet::new();
        for i in 0..N {
            for j in i + 1..N {
                expected.insert((ex(&format!("n{i}")), ex(&format!("n{j}"))));
            }
        }
        assert_eq!(
            reaches, expected,
            "transitive closure of ex:reaches must be exactly the i<j pairs"
        );
    }

    #[test]
    fn entailment_is_stable_under_isomorphic_input_relabeling() {
        let shapes = r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .";
        let a = entail("_:x a ex:Person .", shapes);
        let b = entail("_:y a ex:Person .", shapes);
        assert_eq!(
            canon(&a),
            canon(&b),
            "isomorphic inputs (blank relabeled) must entail identically"
        );
    }

    // ── Focus tags and mint-time blank labels ────────────────────────────────────

    /// The per-focus tag is injective across every focus kind — foci whose
    /// renderings differ only in an escaped byte (`/` vs `#`, `/` vs a literal
    /// `-2f-`) must get distinct tags — and every minted `{tag}c{n}` label
    /// (`tag` already carries its trailing `_` separator) is a serializable
    /// `BLANK_NODE_LABEL`.
    #[test]
    fn focus_tag_is_injective_across_focus_kinds() {
        use crate::term::{Literal, NamedNode, Triple};
        let foci = vec![
            Term::NamedNode(NamedNode::new_unchecked("http://example.org/x/y")),
            Term::NamedNode(NamedNode::new_unchecked("http://example.org/x#y")),
            Term::NamedNode(NamedNode::new_unchecked("urn:x/y")),
            Term::NamedNode(NamedNode::new_unchecked("urn:x-2f-y")),
            Term::blank("b1"),
            Term::blank("b_1"),
            Term::Literal(Literal::new_directional_language_tagged_literal_unchecked(
                "x",
                "en",
                ::purrdf::RdfTextDirection::Ltr,
            )),
            Term::Literal(Literal::new_typed_literal(
                "x",
                NamedNode::new_unchecked("http://example.org/dt"),
            )),
            Term::Triple(Box::new(Triple::new(
                Term::NamedNode(NamedNode::new_unchecked("http://example.org/s")),
                NamedNode::new_unchecked("http://example.org/p"),
                Term::blank("o1"),
            ))),
        ];
        let tags: Vec<String> = foci.iter().map(focus_tag).collect();
        for (i, (focus, left)) in foci.iter().zip(&tags).enumerate() {
            for (other, right) in foci.iter().zip(&tags).skip(i + 1) {
                assert_ne!(left, right, "foci {focus} and {other} must not share a tag");
            }
            assert!(
                ::purrdf::blank_label::is_valid_blank_node_label(&format!("{left}c1")),
                "the minted label for focus {focus} must be serializable: {left}c1"
            );
        }
    }

    /// Byte sweep over the escape classes: whatever bytes the focus rendering
    /// carries (alphanumeric, `-`, `_`, a raw C0 control, a space, multi-byte
    /// UTF-8), the tag stays inside `f[A-Za-z0-9-]*_` (the trailing `_` is the
    /// constant mint-time separator) and distinct inputs give distinct tags.
    /// Blank-node foci are used because their labels render raw.
    #[test]
    fn focus_tag_byte_sweep_stays_in_alphabet() {
        let inputs = ["a", "Z", "9", "-", "_", "\u{1f}", " ", "é", "日"];
        let tags: Vec<String> = inputs
            .iter()
            .map(|s| focus_tag(&Term::blank((*s).to_owned())))
            .collect();
        for (input, tag) in inputs.iter().zip(&tags) {
            assert!(tag.starts_with('f'), "{input:?} → {tag}");
            assert!(tag.ends_with('_'), "{input:?} → {tag}");
            let body = &tag[1..tag.len() - 1];
            assert!(
                body.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "the tag must match f[A-Za-z0-9-]*_: {input:?} → {tag}"
            );
        }
        for (i, left) in tags.iter().enumerate() {
            for right in &tags[i + 1..] {
                assert_ne!(left, right, "distinct inputs must give distinct tags");
            }
        }
        // Pin the exact output for a couple of representative inputs: an
        // alphanumeric byte passes through untouched and an escaped byte
        // becomes `-` + two lowercase hex digits, all inside `_:<label>`
        // (blank-node `Display` renders as `_:a`), terminated by `_`.
        assert_eq!(tags[0], "f-5f-3aa_", "blank(\"a\") → _:a → f-5f-3aa_");
        assert_eq!(tags[3], "f-5f-3a-2d_", "blank(\"-\") → _:- → f-5f-3a-2d_");
    }

    /// A minted `{tag}c{n}` label (`tag` already carries its trailing `_`
    /// separator) starts with `f` and is dot-free by construction (`.` is
    /// escaped as `-2e`), so it lands outside the reserved `purrdfesc` marker
    /// namespace and the scope decode must return it verbatim — even when the
    /// focus rendering itself contains a scope-suffix-shaped substring.
    #[test]
    fn minted_labels_are_never_scope_split() {
        let foci = [
            Term::NamedNode(NamedNode::new_unchecked("http://example.org/x.s5")),
            Term::blank("b.s2"),
            Term::blank("b1"),
        ];
        for focus in &foci {
            let label = format!("{}c1", focus_tag(focus));
            assert!(!label.contains('.'), "minted labels are dot-free: {label}");
            assert_eq!(
                ::purrdf::BlankScope::unqualify_label(&label),
                (
                    std::borrow::Cow::Borrowed(label.as_str()),
                    ::purrdf::BlankScope::DEFAULT
                ),
                "{label}"
            );
        }
        // The §16.2 freshness suffix `r{k}` is alphanumeric, so a reminted label
        // is equally immune.
        assert_eq!(
            ::purrdf::BlankScope::unqualify_label("fb1_c1r0"),
            (
                std::borrow::Cow::Borrowed("fb1_c1r0"),
                ::purrdf::BlankScope::DEFAULT
            )
        );
    }

    /// A `sh:SPARQLRule` carrying a DATA blank through a CONSTRUCT variable must
    /// preserve co-reference: the derived triple references the SAME blank node
    /// as the base triple (mint-time prefixing touches only minted blanks), the
    /// fixpoint closes without error, and entailment is stable under repetition.
    #[test]
    fn sparql_rule_preserves_data_blank_co_reference() {
        let data = "ex:alice ex:has _:contact . _:contact a ex:Contact .";
        let shapes = r#"
            ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:has ;
              sh:rule [ a sh:SPARQLRule ; sh:construct
                "CONSTRUCT { $this ex:copied ?b } WHERE { $this ex:has ?b }" ] ."#;

        // The fixpoint is reached without error (`entail` unwraps) and repeated
        // entailment is byte-stable.
        let first = entail(data, shapes);
        let second = entail(data, shapes);
        assert_eq!(canon(&first), canon(&second));

        // RDFC canonical labels: the blank in the base ex:has triple and in the
        // derived ex:copied triple must canonicalize to the SAME label.
        let nq = canon(&first);
        let object_of = |predicate: &str| -> String {
            nq.lines()
                .find(|line| line.contains(predicate))
                .and_then(|line| line.split_whitespace().nth(2))
                .unwrap_or_else(|| panic!("no triple with {predicate} in {nq}"))
                .to_owned()
        };
        let has_object = object_of("ns#has>");
        let copied_object = object_of("ns#copied>");
        assert!(
            has_object.starts_with("_:"),
            "the base object is a blank: {has_object}"
        );
        assert_eq!(
            has_object, copied_object,
            "the derived triple must reference the SAME blank node as the base triple"
        );

        // Entailing the entailed dataset derives nothing new: the derived triple
        // already co-refers with the base blank, so the closure is stable.
        let shapes_parsed = parse_shapes(shapes);
        let again = entail_dataset(first.as_ref(), &shapes_parsed).expect("re-entailment");
        assert_eq!(canon(&first), canon(&again));
    }

    /// The same co-reference guarantee for a data blank whose label is DOTTED.
    ///
    /// A dotted label is the one that survives the projection only if the owned
    /// round trip is exactly invertible: the projection, every fixpoint round,
    /// and the `$this` pre-binding each re-materialize the focus term, and a
    /// re-encoding that rewrote the label would silently point the derived
    /// triple at a node that does not exist.
    #[test]
    fn sparql_rule_preserves_a_dotted_data_blank_co_reference() {
        // `_:a..b` is a legal `BLANK_NODE_LABEL` and denotes the label `a..b`
        // itself — every surface must carry it byte for byte.
        let data = "ex:alice ex:has _:a..b . _:a..b a ex:Contact .";
        let shapes = r#"
            ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:has ;
              sh:rule [ a sh:SPARQLRule ; sh:construct
                "CONSTRUCT { $this ex:copied ?b } WHERE { $this ex:has ?b }" ] ."#;

        let entailed = entail(data, shapes);
        let nq = canon(&entailed);
        let object_of = |predicate: &str| -> String {
            nq.lines()
                .find(|line| line.contains(predicate))
                .and_then(|line| line.split_whitespace().nth(2))
                .unwrap_or_else(|| panic!("no triple with {predicate} in {nq}"))
                .to_owned()
        };
        assert_eq!(
            object_of("ns#has>"),
            object_of("ns#copied>"),
            "the derived triple must reference the SAME dotted blank node: {nq}"
        );

        // The label itself never grew: the entailed dataset still holds `a..b`.
        assert!(
            blank_labels(&entailed).contains(&"a..b".to_owned()),
            "the dotted label must survive un-requalified: {:?}",
            blank_labels(&entailed)
        );
    }

    /// The derivation matrix: a rule targeting blank foci must fire for EVERY
    /// focus label, including the ones whose spelling collides with the
    /// blank-scope encoding.
    ///
    /// `a..b`, `x.s1` and `a..b..c` are the labels a non-invertible owned round
    /// trip mangles; a mangled focus term denotes nothing in the projection, so
    /// the rule derives NOTHING for it and `entail_dataset` still returns `Ok` —
    /// a silent wrong answer, which is why every label is checked rather than a
    /// representative one.
    #[test]
    fn every_blank_focus_label_derives_regardless_of_dots_and_scopes() {
        // Each token is a legal `BLANK_NODE_LABEL` denoting ITSELF: the dotted
        // spellings and the scope-suffix-shaped `x.s1` are ordinary labels that
        // every surface must carry through unchanged.
        let data = "\
            _:p a ex:Person .\n\
            _:ab a ex:Person .\n\
            _:a..b a ex:Person .\n\
            _:x.s1 a ex:Person .\n\
            _:a..b..c a ex:Person .";
        let shapes = r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:SPARQLRule ; sh:construct
                "CONSTRUCT { $this ex:seen true } WHERE { $this a ex:Person }" ] ."#;

        let entailed = entail(data, shapes);
        let seen: FastSet<String> = triples(&entailed)
            .into_iter()
            .filter(|(_, p, _)| *p == ex("seen"))
            .map(|(s, _, _)| s)
            .collect();
        let people: FastSet<String> = triples(&entailed)
            .into_iter()
            .filter(|(_, p, o)| {
                *p == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>" && *o == ex("Person")
            })
            .map(|(s, _, _)| s)
            .collect();
        assert_eq!(people.len(), 5, "five distinct foci: {people:?}");
        assert_eq!(
            seen, people,
            "every focus must derive ex:seen — a focus missing here derived NOTHING"
        );
    }

    // ── Hostile-focus serialization round-trips ──────────────────────────────────
    //
    // End-to-end net over the full pipeline: a `sh:SPARQLRule` whose CONSTRUCT
    // template mints anonymous property-shape blanks (`ex:property [ ex:path … ;
    // ex:minCount 1 ]`), run against focus nodes whose renderings carry bytes
    // OUTSIDE the blank-node-label alphabet (`#`, `:`, `/`). The minted per-focus
    // labels must stay serializable, survive Turtle and N-Triples egress
    // byte-clean, and re-parse into an isomorphic dataset.

    /// Two focus IRIs whose renderings are hostile to naive label minting: a
    /// fragment IRI and a colon-riddled URN.
    const HOSTILE_FOCI_DATA: &str = "\
        <http://example.org/p#frag> a ex:Thing .\n\
        <urn:maplib:69e3353f-2246-4469-bb83-a068cdaa9f1c> a ex:Thing .";

    /// A `sh:SPARQLRule` whose CONSTRUCT template carries an anonymous
    /// property-shape-style blank (bracketed, so the engine mints its label).
    const PROPERTY_MINTING_SHAPES: &str = r#"
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
          sh:rule [ a sh:SPARQLRule ; sh:runOnce true ; sh:construct
            "CONSTRUCT { $this ex:property [ ex:path ex:name ; ex:minCount 1 ] } WHERE { $this a ex:Thing }" ] ."#;

    /// The distinct blank-node labels of `ds`, as serialized (scope-qualified).
    fn blank_labels(ds: &RdfDataset) -> Vec<String> {
        let mut labels: Vec<String> =
            quads_for_pattern_ids(ds, None, None, None, GraphFilter::AnyGraph)
                .flat_map(|q| [q.s, q.p, q.o])
                .filter_map(|id| {
                    term_id_to_native(ds, id)
                        .blank_label()
                        .map(ToOwned::to_owned)
                })
                .collect();
        labels.sort();
        labels.dedup();
        labels
    }

    /// Every `_:` token of a serialized document (from `_:` up to whitespace)
    /// must be free of `<`, `>`, and raw C0 control bytes — the raw-focus-bytes
    /// leak signature.
    fn assert_blank_tokens_clean(bytes: &[u8], media: &str) {
        let text = std::str::from_utf8(bytes).expect("serialized RDF text is UTF-8");
        let mut rest = text;
        while let Some(pos) = rest.find("_:") {
            let token = rest[pos..]
                .split_whitespace()
                .next()
                .expect("a found token has a non-whitespace head");
            assert!(
                !token.contains('<') && !token.contains('>') && !token.contains('\u{1f}'),
                "{media}: blank token carries raw focus bytes: {token:?}"
            );
            rest = &rest[pos + 2..];
        }
    }

    /// Serialize `entailed` to Turtle and N-Triples, assert every blank token is
    /// byte-clean, re-parse each document, and assert the re-parsed dataset is
    /// isomorphic to the entailed one (canonical N-Quads equality).
    fn assert_serialization_roundtrip(entailed: &RdfDataset) {
        for media in ["text/turtle", "application/n-triples"] {
            let bytes =
                ::purrdf::serialize_dataset(entailed, media, ::purrdf::SerializeGraph::Dataset)
                    .unwrap_or_else(|e| panic!("{media} serialization must succeed: {e}"));
            assert_blank_tokens_clean(&bytes, media);
            let reparsed = ::purrdf::parse_dataset(&bytes, media, None)
                .unwrap_or_else(|e| panic!("{media} re-parse must succeed: {e}"));
            assert_eq!(
                canon(entailed),
                canon(reparsed.as_ref()),
                "{media} round-trip must be isomorphic to the entailed dataset"
            );
        }
    }

    /// Hostile IRI foci (fragment IRI + colon-riddled URN): every minted blank
    /// label is a legal `BLANK_NODE_LABEL`, both text egresses succeed byte-clean,
    /// and both round-trip isomorphically.
    #[test]
    fn hostile_iri_foci_entail_and_roundtrip_through_text() {
        let entailed = entail(HOSTILE_FOCI_DATA, PROPERTY_MINTING_SHAPES);
        let labels = blank_labels(&entailed);
        assert!(!labels.is_empty(), "the rule must mint template blanks");
        for label in &labels {
            assert!(
                ::purrdf::blank_label::is_valid_blank_node_label(label),
                "minted label must be a legal BLANK_NODE_LABEL: {label:?}"
            );
        }
        assert_serialization_roundtrip(&entailed);
    }

    /// Blank-node foci (via `sh:targetObjectsOf`): the minted labels encode a
    /// `_:` rendering and must satisfy the same egress contract end-to-end.
    #[test]
    fn blank_foci_entail_and_roundtrip_through_text() {
        let data = "\
            ex:alice ex:hasContact [ a ex:Contact ] .\n\
            ex:bob   ex:hasContact [ a ex:Contact ] .";
        let shapes = r#"
            ex:S a sh:NodeShape ; sh:targetObjectsOf ex:hasContact ;
              sh:rule [ a sh:SPARQLRule ; sh:runOnce true ; sh:construct
                "CONSTRUCT { $this ex:property [ ex:path ex:name ; ex:minCount 1 ] } WHERE { $this a ex:Contact }" ] ."#;
        let entailed = entail(data, shapes);
        let labels = blank_labels(&entailed);
        // Two data blanks (the contacts) plus two minted template blanks.
        assert!(
            labels.len() >= 4,
            "expected data + minted blanks: {labels:?}"
        );
        for label in &labels {
            assert!(
                ::purrdf::blank_label::is_valid_blank_node_label(label),
                "label must be a legal BLANK_NODE_LABEL: {label:?}"
            );
        }
        assert_serialization_roundtrip(&entailed);
    }

    /// Two foci each mint a template blank; after Turtle egress and re-parse the
    /// two blanks are still DISTINCT nodes (no conflation on the wire).
    #[test]
    fn multi_focus_minted_blanks_stay_distinct_after_serialization() {
        let entailed = entail(HOSTILE_FOCI_DATA, PROPERTY_MINTING_SHAPES);
        assert_eq!(
            blank_labels(&entailed).len(),
            2,
            "two foci mint one template blank each"
        );
        let bytes = ::purrdf::serialize_dataset(
            entailed.as_ref(),
            "text/turtle",
            ::purrdf::SerializeGraph::Dataset,
        )
        .expect("Turtle serialization must succeed");
        let reparsed =
            ::purrdf::parse_dataset(&bytes, "text/turtle", None).expect("re-parse must succeed");
        assert_eq!(
            blank_labels(reparsed.as_ref()).len(),
            2,
            "the two per-focus blanks must stay distinct through the wire"
        );
        // Each focus's ex:property edge points at its OWN blank.
        let property_objects: FastSet<String> = triples(reparsed.as_ref())
            .into_iter()
            .filter(|(_, p, _)| *p == ex("property"))
            .map(|(_, _, o)| o)
            .collect();
        assert_eq!(
            property_objects.len(),
            2,
            "distinct foci must keep distinct property-shape blanks"
        );
    }

    /// Two independent entailment runs serialize to byte-identical Turtle.
    #[test]
    fn entailed_serialization_is_byte_identical_across_runs() {
        let serialize = || {
            let entailed = entail(HOSTILE_FOCI_DATA, PROPERTY_MINTING_SHAPES);
            ::purrdf::serialize_dataset(
                entailed.as_ref(),
                "text/turtle",
                ::purrdf::SerializeGraph::Dataset,
            )
            .expect("Turtle serialization must succeed")
        };
        assert_eq!(
            serialize(),
            serialize(),
            "independent entailment runs must serialize byte-identically"
        );
    }
}
