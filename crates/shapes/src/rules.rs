// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL Advanced Features **rules** (`sh:rule`).
//!
//! A rule attaches to a shape and derives new triples for the shape's target
//! focus nodes. Two rule kinds are supported:
//!
//! - [`RuleBody::Triple`] (`sh:TripleRule`): the head is a single triple whose
//!   subject / predicate / object are SHACL-AF node expressions, evaluated with
//!   the focus node as `sh:this`. The cartesian product of the three result sets
//!   yields the inferred triples.
//! - [`RuleBody::Sparql`] (`sh:SPARQLRule`): the head is a SPARQL `sh:construct`
//!   query run with `$this` pre-bound to the focus node; its CONSTRUCT graph is
//!   the inferred triples.
//!
//! A rule fires for a focus node only if the node conforms to EVERY
//! `sh:condition` shape, is not `sh:deactivated`, and its owning shape is active.
//! Rules run lowest-`sh:order` first (missing order defaults to `0`), tie-broken
//! by rule-node identity.
//!
//! [`apply_rules`] drives an **iterative fixpoint**: each round runs every rule
//! over the current dataset, adds the genuinely-new inferred triples, and rebuilds
//! the dataset for the next round; it stops when a round adds nothing. The result
//! is a NEW frozen `Arc<RdfDataset>` holding the base graph plus every inferred
//! triple, emitted in a deterministic order.
//!
//! # Termination
//!
//! Value-preserving rules over the finite term universe (base ∪ rules graph)
//! reach a fixpoint with no artificial cap: each round strictly grows a set that
//! is bounded by `terms³`. The only divergence mode is a rule minting a fresh
//! term each round (e.g. a CONSTRUCT `BNODE()` or a value-growing expression). The
//! driver tracks the base∪rules term universe; if fresh-term-introducing rounds
//! exceed a deterministic, input-derived bound, [`apply_rules`] returns `Err`
//! naming the offending rule and term rather than looping forever.

use std::sync::Arc;

use ::purrdf::{FastMap, FastSet, IdSet, RdfDataset, RdfDatasetBuilder, RdfQuad};

use crate::constraints::conforms;
use crate::data::{quads_for_pattern_ids, GraphFilter, ShaclData};
use crate::engine::resolve_focus_nodes;
use crate::expression::{eval_node_expr, NodeExpr, RecursionGuard};
use crate::shapes::{Shape, Shapes};
use crate::term::{term_id_to_native, NamedNode, Term, Triple};

// ── Model ───────────────────────────────────────────────────────────────────────

/// The head of a SHACL-AF rule.
#[derive(Debug, Clone)]
#[allow(
    clippy::large_enum_variant,
    reason = "the model mirrors the SHACL-AF vocabulary: a TripleRule head is three \
              inline node expressions, a SPARQLRule head one query string; boxing either \
              would obscure the 1:1 mapping with sh:subject/predicate/object vs sh:construct"
)]
pub enum RuleBody {
    /// A `sh:TripleRule`: three node expressions producing one head triple each
    /// over their cartesian product.
    Triple {
        /// The subject node expression (must yield IRIs or blank nodes).
        subject: NodeExpr,
        /// The predicate node expression (must yield IRIs).
        predicate: NodeExpr,
        /// The object node expression (may yield any term).
        object: NodeExpr,
    },
    /// A `sh:SPARQLRule`: a SPARQL CONSTRUCT query (with any injected `PREFIX`
    /// header) run with `$this` pre-bound to the focus node.
    Sparql {
        /// The CONSTRUCT query text.
        construct: String,
    },
}

/// A rule's `sh:order` sort key: a numeric literal, lower runs first.
///
/// Not `Ord` (it wraps an `f64`); the rules engine orders rules with
/// [`OrderKey::value`] via `f64::total_cmp`, tie-broken by rule-node identity.
#[derive(Debug, Clone, Copy)]
pub struct OrderKey {
    value: f64,
}

impl OrderKey {
    /// Wrap a numeric `sh:order` value.
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self { value }
    }

    /// The numeric order value (lower runs first).
    #[must_use]
    pub fn value(self) -> f64 {
        self.value
    }
}

/// A single SHACL-AF rule attached to a shape.
#[derive(Debug, Clone)]
pub struct Rule {
    /// The rule node identity (IRI or blank node) — the deterministic tie-break
    /// for `sh:order` and the label named in fixpoint / legality errors.
    pub id: Term,
    /// The rule head.
    pub body: RuleBody,
    /// `sh:condition` shapes (as their node terms): a rule fires for a focus node
    /// only if the node conforms to every one, resolved against the shapes graph's
    /// top-level shapes at firing time.
    pub conditions: Vec<Term>,
    /// The `sh:order` sort key, if declared (missing = default order `0`).
    pub order: Option<OrderKey>,
    /// Whether `sh:deactivated true` is set — a deactivated rule never fires.
    pub deactivated: bool,
}

impl Rule {
    /// The effective numeric order (declared `sh:order`, or `0` when absent).
    fn order_value(&self) -> f64 {
        self.order.map_or(0.0, OrderKey::value)
    }
}

// ── Driver ──────────────────────────────────────────────────────────────────────

/// A rule producer: maps the current frozen dataset to the owned head triples the
/// rule derives this round (the `Fn(&RdfDataset)` seam, taking the round's `Arc`
/// so it can hand the SPARQL/expression engines an owned dataset handle).
type Producer<'a> = Box<dyn Fn(&Arc<RdfDataset>) -> Result<Vec<[Term; 3]>, String> + 'a>;

/// A prepared rule bound to its owning shape: a producer closure plus its sort
/// key and identity (for ordering and error messages).
struct PreparedRule<'a> {
    order: f64,
    tiebreak: String,
    rule_id: String,
    producer: Producer<'a>,
}

/// Apply every active SHACL-AF rule to `data` under `shapes`, materializing a NEW
/// frozen dataset of the base graph plus all inferred triples.
///
/// The rules read and write the FLATTENED default graph (the same projection the
/// validator operates over, exposed by [`ShaclData::core`]). Rule firing is an
/// iterative fixpoint over the ordered rule list; the returned dataset is
/// deterministic (byte-stable across runs and under isomorphic input relabeling).
///
/// # Errors
///
/// Returns `Err(String)` when a rule is malformed at firing time (an illegal
/// subject/predicate in a produced triple, an unresolvable `sh:condition`, a
/// failing node-expression / CONSTRUCT evaluation) or when the rule set does not
/// reach a fixpoint (a fresh-term-minting rule exceeding the divergence bound).
pub fn apply_rules(data: &ShaclData, shapes: &Shapes) -> Result<Arc<RdfDataset>, String> {
    // Declared SHACL-AF functions are in scope for node expressions and CONSTRUCT
    // bodies for the whole run; the guard restores the previous table on drop.
    let _function_scope = crate::sparql::enter_function_scope(Arc::clone(&shapes.functions));

    let base = data.core_arc();

    // A top-level-shape index for `sh:condition` resolution (id string → shape).
    let mut shape_index: FastMap<String, &Shape> = FastMap::default();
    for shape in &shapes.node_shapes {
        shape_index.insert(shape.id.to_string(), shape);
    }

    // The base∪rules term universe: any triple whose every term is drawn from here
    // is value-preserving and cannot diverge. Used to detect fresh-term minting.
    let base_universe = build_term_universe(base.as_ref(), shapes.shapes_dataset.as_ref());

    // The base default-graph triples, so a rule re-deriving a base fact is not
    // counted as new inference.
    let original = base_triples(base.as_ref());

    // Build one producer per (active shape, active rule).
    let mut prepared: Vec<PreparedRule<'_>> = Vec::new();
    for shape in &shapes.node_shapes {
        if shape.deactivated {
            continue;
        }
        for rule in &shape.rules {
            if rule.deactivated {
                continue;
            }
            prepared.push(prepare_rule(shape, rule, &shape_index));
        }
    }

    // Order: lowest sh:order first, tie-broken by rule-node identity.
    prepared.sort_by(|a, b| {
        a.order
            .total_cmp(&b.order)
            .then_with(|| a.tiebreak.cmp(&b.tiebreak))
    });

    // No rules → the base graph unchanged (still a fresh frozen projection).
    let mut facts = original.clone();
    let mut current = Arc::clone(&base);

    // Deterministic, input-derived divergence bound: the number of distinct base∪
    // rules terms plus the rule count. Value-preserving rounds never touch it (they
    // introduce no fresh term); only fresh-term-minting rounds are capped.
    let bound = base_universe.len() + prepared.len() + 1;
    let mut fresh_rounds = 0usize;

    loop {
        let mut round_new: Vec<[Term; 3]> = Vec::new();
        let mut fresh_offender: Option<(String, String)> = None;

        for prep in &prepared {
            for triple in (prep.producer)(&current)? {
                if facts.contains(&triple) {
                    continue;
                }
                if fresh_offender.is_none() {
                    if let Some(term) = fresh_term(&triple, &base_universe) {
                        fresh_offender = Some((prep.rule_id.clone(), term));
                    }
                }
                facts.insert(triple.clone());
                round_new.push(triple);
            }
        }

        if round_new.is_empty() {
            break;
        }

        if let Some((rule_id, term)) = fresh_offender {
            fresh_rounds += 1;
            if fresh_rounds > bound {
                return Err(format!(
                    "SHACL rules did not reach a fixpoint: rule {rule_id} keeps deriving \
                     fresh terms not present in the base or rules graph (e.g. {term}) after \
                     {fresh_rounds} rounds (bound {bound})"
                ));
            }
        }

        current = rebuild_dataset(&base, &facts, &original)?;
    }

    // Materialize base ⊎ inferred, emitting inferred triples in a stable sorted
    // order (freeze canonicalizes quad order, but sorting keeps the builder input
    // deterministic — mirrors the entail engine's discipline).
    let mut inferred: Vec<[Term; 3]> = facts
        .iter()
        .filter(|triple| !original.contains(*triple))
        .cloned()
        .collect();
    inferred.sort_by_cached_key(triple_sort_key);

    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(base.as_ref());
    for triple in &inferred {
        push_fact(&mut builder, triple)?;
    }
    builder.freeze().map_err(|e| e.to_string())
}

/// Entail a frozen [`RdfDataset`] under `shapes`: build the SHACL projection, run
/// [`apply_rules`], and return the entailed dataset (mirrors
/// [`validate_dataset`](crate::engine::validate_dataset)).
///
/// # Errors
///
/// Returns `Err(String)` when the SHACL projection cannot be frozen or when rule
/// application fails (see [`apply_rules`]).
pub fn entail_dataset(data: &RdfDataset, shapes: &Shapes) -> Result<Arc<RdfDataset>, String> {
    let projected = crate::engine::project_dataset(data)?;
    // Core lookups and the SHACL-SPARQL / CONSTRUCT paths run over the same
    // flattened projection.
    let holder = ShaclData::new(Arc::clone(&projected), projected, None);
    apply_rules(&holder, shapes)
}

// ── Producer construction ───────────────────────────────────────────────────────

/// Build the producer closure for one rule bound to its shape.
fn prepare_rule<'a>(
    shape: &'a Shape,
    rule: &'a Rule,
    shape_index: &'a FastMap<String, &'a Shape>,
) -> PreparedRule<'a> {
    let rule_id = rule.id.to_string();
    let conditions = rule.conditions.as_slice();

    let producer: Producer<'a> = match &rule.body {
        RuleBody::Triple {
            subject,
            predicate,
            object,
        } => {
            let rule_id = rule_id.clone();
            Box::new(move |ds: &Arc<RdfDataset>| {
                triple_rule_producer(
                    ds,
                    shape,
                    subject,
                    predicate,
                    object,
                    conditions,
                    shape_index,
                    &rule_id,
                )
            })
        }
        RuleBody::Sparql { construct } => {
            let rule_id = rule_id.clone();
            Box::new(move |ds: &Arc<RdfDataset>| {
                sparql_rule_producer(ds, shape, construct, conditions, shape_index, &rule_id)
            })
        }
    };

    PreparedRule {
        order: rule.order_value(),
        tiebreak: rule_id.clone(),
        rule_id,
        producer,
    }
}

/// The `sh:TripleRule` producer: evaluate subject/predicate/object node
/// expressions per focus node and emit the cartesian product as head triples.
#[allow(clippy::too_many_arguments)]
fn triple_rule_producer(
    ds: &Arc<RdfDataset>,
    shape: &Shape,
    subject: &NodeExpr,
    predicate: &NodeExpr,
    object: &NodeExpr,
    conditions: &[Term],
    shape_index: &FastMap<String, &Shape>,
    rule_id: &str,
) -> Result<Vec<[Term; 3]>, String> {
    let data = ShaclData::new(Arc::clone(ds), Arc::clone(ds), None);
    let focus_nodes = focus_nodes(&data, shape)?;
    let mut out: Vec<[Term; 3]> = Vec::new();
    for focus in &focus_nodes {
        if !conditions_hold(&data, focus, conditions, shape_index)? {
            continue;
        }
        let mut guard = RecursionGuard::new();
        let subjects = eval_node_expr(&data, focus, subject, &mut guard)?;
        let predicates = eval_node_expr(&data, focus, predicate, &mut guard)?;
        let objects = eval_node_expr(&data, focus, object, &mut guard)?;
        for s in &subjects {
            if !s.is_subject() {
                return Err(format!(
                    "sh:TripleRule {rule_id} produced an illegal subject {s} \
                     (a triple subject must be an IRI or blank node)"
                ));
            }
            for p in &predicates {
                let Term::NamedNode(_) = p else {
                    return Err(format!(
                        "sh:TripleRule {rule_id} produced an illegal predicate {p} \
                         (a triple predicate must be an IRI)"
                    ));
                };
                for o in &objects {
                    out.push([s.clone(), p.clone(), o.clone()]);
                }
            }
        }
    }
    Ok(out)
}

/// The `sh:SPARQLRule` producer: run the CONSTRUCT query with `$this` pre-bound to
/// each focus node and collect the derived triples.
fn sparql_rule_producer(
    ds: &Arc<RdfDataset>,
    shape: &Shape,
    construct: &str,
    conditions: &[Term],
    shape_index: &FastMap<String, &Shape>,
    rule_id: &str,
) -> Result<Vec<[Term; 3]>, String> {
    let data = ShaclData::new(Arc::clone(ds), Arc::clone(ds), None);
    let focus_nodes = focus_nodes(&data, shape)?;
    let mut out: Vec<[Term; 3]> = Vec::new();
    for focus in &focus_nodes {
        if !conditions_hold(&data, focus, conditions, shape_index)? {
            continue;
        }
        let subs = [("this".to_owned(), focus.to_term_value())];
        let graph = crate::sparql::run_construct_with_shacl_prebinding(
            ds,
            construct,
            &subs,
            None,
            Some(&shape.id),
        )?;
        // A CONSTRUCT template blank is minted `_:c{n}` from a per-evaluation
        // counter that resets each call, so two focus nodes would both mint `_:c1`
        // and conflate. Relabel every minted blank with the focus's identity so
        // distinct focus nodes get distinct blanks — deterministically, so a
        // re-derivation in a later round produces the identical label and the
        // fixpoint converges.
        let tag = focus.to_string();
        for quad in quads_for_pattern_ids(graph.as_ref(), None, None, None, GraphFilter::AnyGraph) {
            let s = relabel_blanks(term_id_to_native(graph.as_ref(), quad.s), &tag);
            let p = term_id_to_native(graph.as_ref(), quad.p);
            let o = relabel_blanks(term_id_to_native(graph.as_ref(), quad.o), &tag);
            if !s.is_subject() {
                return Err(format!(
                    "sh:SPARQLRule {rule_id} CONSTRUCT produced an illegal subject {s}"
                ));
            }
            let Term::NamedNode(_) = &p else {
                return Err(format!(
                    "sh:SPARQLRule {rule_id} CONSTRUCT produced an illegal predicate {p}"
                ));
            };
            out.push([s, p, o]);
        }
    }
    Ok(out)
}

// ── Helpers ─────────────────────────────────────────────────────────────────────

/// Resolve the focus nodes of `shape` against the current dataset.
fn focus_nodes(data: &ShaclData, shape: &Shape) -> Result<Vec<Term>, String> {
    let mut memo: FastMap<NamedNode, IdSet> = FastMap::default();
    resolve_focus_nodes(data, &shape.targets, &mut memo)
}

/// Whether `focus` conforms to every `sh:condition` shape (resolved against the
/// top-level shape index). An unresolvable condition is a hard error.
fn conditions_hold(
    data: &ShaclData,
    focus: &Term,
    conditions: &[Term],
    shape_index: &FastMap<String, &Shape>,
) -> Result<bool, String> {
    for condition in conditions {
        let shape = shape_index.get(&condition.to_string()).ok_or_else(|| {
            format!("sh:condition {condition} does not reference a known top-level shape")
        })?;
        if !conforms(data, focus, shape)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Recursively rename every blank-node label in `term` by prefixing it with `tag`
/// (a deterministic per-focus identity), preserving co-reference.
fn relabel_blanks(term: Term, tag: &str) -> Term {
    match term {
        Term::BlankNode(label) => Term::BlankNode(format!("{tag}\u{1f}{label}")),
        Term::Triple(inner) => Term::Triple(Box::new(Triple::new(
            relabel_blanks(inner.subject, tag),
            inner.predicate,
            relabel_blanks(inner.object, tag),
        ))),
        other => other,
    }
}

/// The base default-graph triples as an owned-term fact set.
fn base_triples(base: &RdfDataset) -> FastSet<[Term; 3]> {
    let mut set: FastSet<[Term; 3]> = FastSet::default();
    for quad in quads_for_pattern_ids(base, None, None, None, GraphFilter::DefaultGraph) {
        set.insert([
            term_id_to_native(base, quad.s),
            term_id_to_native(base, quad.p),
            term_id_to_native(base, quad.o),
        ]);
    }
    set
}

/// The set of every term string appearing in `base` (all graphs) or in the shapes
/// graph `shapes_ds` — the value-preserving universe used for divergence detection.
fn build_term_universe(base: &RdfDataset, shapes_ds: &RdfDataset) -> FastSet<String> {
    let mut universe: FastSet<String> = FastSet::default();
    for ds in [base, shapes_ds] {
        for quad in quads_for_pattern_ids(ds, None, None, None, GraphFilter::AnyGraph) {
            for id in [quad.s, quad.p, quad.o] {
                universe.insert(term_id_to_native(ds, id).to_string());
            }
        }
    }
    universe
}

/// The first term of `triple` whose string is absent from the value-preserving
/// `universe` — a freshly minted term, if any.
fn fresh_term(triple: &[Term; 3], universe: &FastSet<String>) -> Option<String> {
    triple.iter().find_map(|term| {
        let key = term.to_string();
        (!universe.contains(&key)).then_some(key)
    })
}

/// The deterministic total-order sort key for an inferred triple.
fn triple_sort_key(triple: &[Term; 3]) -> (String, String, String) {
    (
        triple[0].to_string(),
        triple[1].to_string(),
        triple[2].to_string(),
    )
}

/// Push one owned head triple into `builder`. Its predicate is an IRI (enforced by
/// the producers), so a non-IRI predicate here is an internal invariant breach.
fn push_fact(builder: &mut RdfDatasetBuilder, triple: &[Term; 3]) -> Result<(), String> {
    let [s, p, o] = triple;
    let Term::NamedNode(predicate) = p else {
        return Err(format!(
            "internal error: inferred triple has non-IRI predicate {p}"
        ));
    };
    builder.push_owned_quad(&RdfQuad::new(
        s.to_rdf_term(),
        predicate.as_str(),
        o.to_rdf_term(),
    ));
    Ok(())
}

/// Rebuild the round dataset: the base projection plus every inferred triple so
/// far (seed `push_dataset(base)`, push derived quads, freeze).
fn rebuild_dataset(
    base: &Arc<RdfDataset>,
    facts: &FastSet<[Term; 3]>,
    original: &FastSet<[Term; 3]>,
) -> Result<Arc<RdfDataset>, String> {
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(base.as_ref());
    for triple in facts {
        if !original.contains(triple) {
            push_fact(&mut builder, triple)?;
        }
    }
    builder.freeze().map_err(|e| e.to_string())
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
        crate::engine::parse_shapes(&format!("{PREFIXES}\n{body}")).expect("shapes must parse")
    }

    fn parse_shapes_err(body: &str) -> String {
        crate::engine::parse_shapes(&format!("{PREFIXES}\n{body}"))
            .expect_err("shapes must fail to parse")
    }

    fn entail(data_ttl: &str, shapes_body: &str) -> Arc<RdfDataset> {
        let data = crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}\n{data_ttl}"))
            .expect("data must parse");
        let shapes = parse_shapes(shapes_body);
        entail_dataset(data.as_ref(), &shapes).expect("entailment must succeed")
    }

    fn entail_err(data_ttl: &str, shapes_body: &str) -> String {
        let data = crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}\n{data_ttl}"))
            .expect("data must parse");
        let shapes = parse_shapes(shapes_body);
        entail_dataset(data.as_ref(), &shapes).expect_err("entailment must fail")
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
        assert_eq!(rule.conditions[0].to_string(), ex("HasName"));
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

    #[test]
    fn malformed_triple_rule_missing_object_errors() {
        let err = parse_shapes_err(
            r"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:p ] .",
        );
        assert!(err.contains("missing sh:object"), "got: {err}");
    }

    #[test]
    fn ambiguous_rule_both_kinds_errors() {
        let err = parse_shapes_err(
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ sh:subject sh:this ; sh:predicate ex:p ; sh:object ex:o ;
                        sh:construct "CONSTRUCT { $this ex:x ex:y } WHERE { $this a ex:Person }" ] ."#,
        );
        assert!(err.contains("ambiguous"), "got: {err}");
    }

    #[test]
    fn non_numeric_order_errors() {
        let err = parse_shapes_err(
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ; sh:order "soon" ;
                        sh:subject sh:this ; sh:predicate ex:p ; sh:object ex:o ] ."#,
        );
        assert!(err.contains("sh:order"), "got: {err}");
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

    #[test]
    fn triple_rule_literal_subject_errors() {
        let err = entail_err(
            "ex:alice a ex:Person .",
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ;
                        sh:subject "notasubject" ; sh:predicate ex:p ; sh:object ex:o ] ."#,
        );
        assert!(err.contains("illegal subject"), "got: {err}");
    }

    #[test]
    fn triple_rule_literal_predicate_errors() {
        let err = entail_err(
            "ex:alice a ex:Person .",
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:TripleRule ;
                        sh:subject sh:this ; sh:predicate "notapred" ; sh:object ex:o ] ."#,
        );
        assert!(err.contains("illegal predicate"), "got: {err}");
    }

    /// Every W3C-normative node-expression kind works in rule (object) position —
    /// they all route through the shared `eval_node_expr`.
    #[test]
    fn node_expr_kinds_work_in_rule_position() {
        let data = "ex:a a ex:Root ; ex:p ex:b, ex:c ; ex:n 1, 2 .";
        // (object node-expression Turtle, expected object N-Triples string).
        let cases: Vec<(&str, String)> = vec![
            ("sh:this", ex("a")),
            ("ex:z", ex("z")),
            ("[ sh:path ex:p ]", ex("b")), // one of {b,c}
            (
                "[ sh:count [ sh:path ex:n ] ]",
                "\"2\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned(),
            ),
            ("[ sh:union ( [ sh:path ex:p ] ex:z ) ]", ex("z")),
            (
                "[ sh:intersection ( [ sh:path ex:p ] [ sh:path ex:p ] ) ]",
                ex("b"),
            ),
            (
                "[ sh:exists [ sh:path ex:p ] ]",
                "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>".to_owned(),
            ),
            ("[ sh:distinct [ sh:path ex:p ] ]", ex("b")),
            (
                "[ sh:max [ sh:path ex:n ] ]",
                "\"2\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned(),
            ),
            (
                "[ sh:if [ sh:exists [ sh:path ex:p ] ] ; sh:then ex:yes ; sh:else ex:no ]",
                ex("yes"),
            ),
        ];
        for (obj, expected) in cases {
            let out = entail(
                data,
                &format!(
                    "ex:S a sh:NodeShape ; sh:targetClass ex:Root ;
                       sh:rule [ a sh:TripleRule ;
                                 sh:subject sh:this ; sh:predicate ex:out ; sh:object {obj} ] ."
                ),
            );
            let objects: Vec<String> = triples(&out)
                .into_iter()
                .filter(|(s, p, _)| *s == ex("a") && *p == ex("out"))
                .map(|(_, _, o)| o)
                .collect();
            assert!(
                objects.contains(&expected),
                "object expr {obj} must derive {expected}; got {objects:?}"
            );
        }
    }

    // ── SPARQLRule execution ─────────────────────────────────────────────────────

    #[test]
    fn single_sparql_rule_derives_head() {
        let out = entail(
            "ex:alice a ex:Person .",
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
              sh:rule [ a sh:SPARQLRule ; sh:construct
                "CONSTRUCT { $this ex:adult ex:yes } WHERE { $this a ex:Person }" ] ."#,
        );
        assert!(has_iri(&out, "alice", "adult", "yes"));
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
        // Each round mints a fresh blank Counter that becomes a new focus node →
        // unbounded fresh-term minting → the divergence bound trips.
        let err = entail_err(
            "ex:c0 a ex:Counter .",
            r#"
            ex:S a sh:NodeShape ; sh:targetClass ex:Counter ;
              sh:rule [ a sh:SPARQLRule ; sh:construct
                "CONSTRUCT { $this ex:next _:n . _:n a ex:Counter } WHERE { $this a ex:Counter }" ] ."#,
        );
        assert!(err.contains("did not reach a fixpoint"), "got: {err}");
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
              sh:rule [ a sh:SPARQLRule ; sh:construct
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
}
