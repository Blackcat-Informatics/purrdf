// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ShEx 2.1 shape-map validator (spec §5.2–§5.5).
//!
//! [`validate`] checks a **fixed shape map** — `(node, shape)` association
//! pairs — against a frozen [`purrdf_core::RdfDataset`], evaluating in
//! interned [`TermId`] space:
//!
//! * **shape expressions** (§5.3): `AND`/`OR`/`NOT`, references, node
//!   constraints, shapes, and `EXTERNAL` via an optional resolver hook;
//! * **node constraints** (§5.4): node kind, datatype with lexical-validity
//!   checking for the SPARQL operand datatypes, string/numeric facets, and
//!   value sets with stems/ranges/exclusions;
//! * **triple expressions** (§5.5): neighbourhood matching with `EXTRA` and
//!   `CLOSED`, `EachOf` partitions, `OneOf` choices, group cardinalities and
//!   inclusions (see the matcher's module doc for the two-layer design);
//! * **recursion** (§5.3): typing-based — a `(node, shape)` pair
//!   re-encountered while being proven is coinductively assumed to hold, and
//!   settled pairs are memoized per validation call. Negation through
//!   recursion is safe because [`crate::structure`] enforces stratification.
//!
//! Iteration order is deterministic everywhere (arcs sort by [`TermId`],
//! slots by document order), so failure reasons are reproducible.

mod matcher;
mod node;
mod pattern;

use std::collections::{BTreeMap, HashMap, HashSet};

use purrdf_core::{DatasetView, GraphMatch, RdfDataset, TermId, TermRef, TermValue};

use crate::ast::{Schema, SemAct, Shape, ShapeExpr, TripleExpr};
use crate::semact::{SemActContext, SemActRegistry};
use matcher::{ArcOptions, CNode, Card, Compiled};
use node::{FactKind, NodeFacts, RDF_LANG_STRING};

// ── public API ──────────────────────────────────────────────────────────────

/// Which shape a shape-map entry associates its node with.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ShapeSelector {
    /// The schema's `start` shape expression.
    Start,
    /// A labeled shape expression (IRI, or `_:`-prefixed blank label).
    Label(String),
}

/// The verdict for one shape-map entry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConformanceStatus {
    /// The node satisfies the shape expression.
    Conformant,
    /// It does not (see [`ResultEntry::reason`]).
    Nonconformant,
}

/// One `(node, shape)` verdict in a [`ResultShapeMap`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ResultEntry {
    /// The focus node, by value.
    pub node: TermValue,
    /// The associated shape.
    pub shape: ShapeSelector,
    /// Conformant or not.
    pub status: ConformanceStatus,
    /// A human-useful reason (the deepest failure), for nonconformant
    /// entries.
    pub reason: Option<String>,
}

/// The result of validating a fixed shape map: one entry per input
/// association, in input order.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ResultShapeMap {
    /// The per-association verdicts.
    pub entries: Vec<ResultEntry>,
}

impl ResultShapeMap {
    /// `true` iff every entry conformed.
    #[must_use]
    pub fn all_conformant(&self) -> bool {
        self.entries
            .iter()
            .all(|e| e.status == ConformanceStatus::Conformant)
    }

    /// Serialize as a result shape map: a JSON array of
    /// `{"node","shape","status","reason"?}` objects, in entry order with a
    /// fixed field order. Nodes and shapes use the same term syntax
    /// [`crate::shapemap::parse_shape_map`] accepts (`<iri>` / `_:label` /
    /// Turtle literal, and `START` / `<label>`); `status` is `conformant` or
    /// `nonconformant`; `reason` is present only for nonconformant entries.
    #[must_use]
    pub fn to_result_json(&self) -> String {
        let mut out = String::from("[");
        for (index, entry) in self.entries.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("{\"node\":");
            push_json_string(&mut out, &node_term_string(&entry.node));
            out.push_str(",\"shape\":");
            push_json_string(&mut out, &shape_term_string(&entry.shape));
            out.push_str(",\"status\":");
            push_json_string(&mut out, status_str(entry.status));
            if let Some(reason) = &entry.reason {
                out.push_str(",\"reason\":");
                push_json_string(&mut out, reason);
            }
            out.push('}');
        }
        out.push(']');
        out
    }
}

/// `xsd:string`, the implicit datatype omitted from a literal's term syntax.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Append `s` as a JSON string literal (serde_json handles escaping).
fn push_json_string(out: &mut String, s: &str) {
    out.push_str(&serde_json::to_string(s).expect("a &str always serializes"));
}

/// The result-map spelling of a conformance status.
fn status_str(status: ConformanceStatus) -> &'static str {
    match status {
        ConformanceStatus::Conformant => "conformant",
        ConformanceStatus::Nonconformant => "nonconformant",
    }
}

/// A term in the shape-map term syntax (`<iri>` / `_:label` / Turtle literal).
fn node_term_string(value: &TermValue) -> String {
    match value {
        TermValue::Iri(iri) => format!("<{iri}>"),
        TermValue::Blank { label, .. } => format!("_:{label}"),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        } => {
            let mut lit = format!("\"{}\"", turtle_escape(lexical_form));
            if let Some(language) = language {
                lit.push('@');
                lit.push_str(language);
            } else if datatype != XSD_STRING {
                lit.push_str("^^<");
                lit.push_str(datatype);
                lit.push('>');
            }
            lit
        }
        TermValue::Triple { s, p, o } => format!(
            "<< {} {} {} >>",
            node_term_string(s),
            node_term_string(p),
            node_term_string(o)
        ),
    }
}

/// A shape label in the shape-map syntax (`START` / `<label>` / `_:label`).
fn shape_term_string(shape: &ShapeSelector) -> String {
    match shape {
        ShapeSelector::Start => "START".to_owned(),
        ShapeSelector::Label(label) if label.starts_with("_:") => label.clone(),
        ShapeSelector::Label(label) => format!("<{label}>"),
    }
}

/// Escape a literal's lexical form for a double-quoted Turtle string.
fn turtle_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

/// A hook resolving an `EXTERNAL` shape declaration's label to its
/// externally-defined expression.
pub type ExternalResolver<'a> = dyn Fn(&str) -> Option<ShapeExpr> + 'a;

/// Optional validator knobs.
#[derive(Default)]
pub struct ValidationOptions<'a> {
    /// Resolves `EXTERNAL` shape declarations by label. Without it, an
    /// `EXTERNAL` shape fails every node (its semantics are unavailable).
    pub external_resolver: Option<&'a ExternalResolver<'a>>,
    /// Extensions dispatched for semantic actions. The default registry is
    /// empty, so every semantic action is an inert success.
    pub sem_acts: SemActRegistry<'a>,
    /// Query-level semantic actions supplied out-of-band (the shexTest
    /// `sht:semActs` / the no-code `%iri%` form), fired as start actions.
    pub extern_start_acts: &'a [SemAct],
}

impl core::fmt::Debug for ValidationOptions<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ValidationOptions")
            .field("external_resolver", &self.external_resolver.map(|_| "<fn>"))
            .field("sem_acts", &self.sem_acts)
            .field("extern_start_acts", &self.extern_start_acts.len())
            .finish()
    }
}

/// Validate a fixed shape map against a frozen dataset.
///
/// Each `(node, shape)` association is checked independently (memoized
/// within the call); the result preserves association order. A focus node
/// absent from the dataset is validated against an empty neighbourhood.
#[must_use]
pub fn validate(
    schema: &Schema,
    data: &RdfDataset,
    map: &[(TermValue, ShapeSelector)],
) -> ResultShapeMap {
    validate_with(schema, data, map, &ValidationOptions::default())
}

/// [`validate`] with explicit [`ValidationOptions`].
#[must_use]
pub fn validate_with(
    schema: &Schema,
    data: &RdfDataset,
    map: &[(TermValue, ShapeSelector)],
    options: &ValidationOptions<'_>,
) -> ResultShapeMap {
    // Resolve whole-declaration EXTERNALs up front so the resolved
    // expressions outlive the engine borrowing them.
    let externals: Vec<(String, ShapeExpr)> = match options.external_resolver {
        Some(resolver) => schema
            .shapes
            .iter()
            .filter(|decl| matches!(decl.expr, ShapeExpr::External))
            .filter_map(|decl| resolver(&decl.id).map(|expr| (decl.id.clone(), expr)))
            .collect(),
        None => Vec::new(),
    };
    // Start actions (schema `startActs` and any query-level actions) fire
    // once before the map is checked; a failure fails the whole validation.
    let start_ctx = SemActContext::default();
    if !options
        .sem_acts
        .dispatch_all(&schema.start_acts, &start_ctx)
        || !options
            .sem_acts
            .dispatch_all(options.extern_start_acts, &start_ctx)
    {
        return ResultShapeMap {
            entries: map
                .iter()
                .map(|(value, selector)| ResultEntry {
                    node: value.clone(),
                    shape: selector.clone(),
                    status: ConformanceStatus::Nonconformant,
                    reason: Some("start semantic action failed".to_owned()),
                })
                .collect(),
        };
    }

    let mut engine = Engine::new(schema, data, &externals, &options.sem_acts);
    let mut entries = Vec::with_capacity(map.len());
    for (value, selector) in map {
        let outcome = engine.check_association(value, selector);
        entries.push(ResultEntry {
            node: value.clone(),
            shape: selector.clone(),
            status: match outcome {
                Ok(()) => ConformanceStatus::Conformant,
                Err(_) => ConformanceStatus::Nonconformant,
            },
            reason: outcome.err(),
        });
    }
    ResultShapeMap { entries }
}

// ── the engine ──────────────────────────────────────────────────────────────

/// A focus or value node: interned in the dataset, or a detached value (a
/// shape-map focus naming a term the data never mentions).
#[derive(Clone, Copy, Debug)]
enum Focus<'a> {
    Id(TermId),
    Detached(&'a TermValue),
}

/// A `(node, shape-label)` pair key for the memo/assumption tables.
type Pair = (TermId, u32);

struct Engine<'a> {
    data: &'a RdfDataset,
    sem_acts: &'a SemActRegistry<'a>,
    start: Option<&'a ShapeExpr>,
    shape_map: HashMap<&'a str, &'a ShapeExpr>,
    te_map: HashMap<&'a str, &'a TripleExpr>,
    label_ids: HashMap<&'a str, u32>,
    /// Settled `(node, shape)` verdicts for this validation call.
    memo: HashMap<Pair, Result<(), String>>,
    /// Pairs currently being proven (the coinductive assumption set).
    in_progress: HashSet<Pair>,
    /// In-progress pairs whose assumption the current proof relied on.
    used_assumptions: HashSet<Pair>,
    /// Labels being proven for a detached focus (cycle guard).
    detached_in_progress: HashSet<u32>,
}

impl<'a> Engine<'a> {
    fn new(
        schema: &'a Schema,
        data: &'a RdfDataset,
        externals: &'a [(String, ShapeExpr)],
        sem_acts: &'a SemActRegistry<'a>,
    ) -> Self {
        let mut shape_map: HashMap<&'a str, &'a ShapeExpr> = schema
            .shapes
            .iter()
            .map(|decl| (decl.id.as_str(), &decl.expr))
            .collect();
        for (label, expr) in externals {
            shape_map.insert(label.as_str(), expr);
        }
        let mut te_labels = Vec::new();
        for decl in &schema.shapes {
            crate::structure::collect_triple_labels_shape_expr(&decl.expr, &mut te_labels);
        }
        if let Some(start) = &schema.start {
            crate::structure::collect_triple_labels_shape_expr(start, &mut te_labels);
        }
        Self {
            data,
            sem_acts,
            start: schema.start.as_deref(),
            shape_map,
            te_map: te_labels.into_iter().collect(),
            label_ids: HashMap::new(),
            memo: HashMap::new(),
            in_progress: HashSet::new(),
            used_assumptions: HashSet::new(),
            detached_in_progress: HashSet::new(),
        }
    }

    fn check_association(
        &mut self,
        value: &TermValue,
        selector: &ShapeSelector,
    ) -> Result<(), String> {
        let focus = match self.data.term_id_by_value(value) {
            Some(id) => Focus::Id(id),
            None => Focus::Detached(value),
        };
        match selector {
            ShapeSelector::Start => {
                let Some(start) = self.start else {
                    return Err("schema declares no start shape".to_owned());
                };
                self.satisfies(focus, start)
            }
            ShapeSelector::Label(label) => self.satisfies_label(focus, label),
        }
    }

    fn label_id(&mut self, label: &'a str) -> u32 {
        let next = self.label_ids.len() as u32;
        *self.label_ids.entry(label).or_insert(next)
    }

    // ── shape expressions (§5.3) ────────────────────────────────────────────

    fn satisfies(&mut self, focus: Focus<'_>, expr: &'a ShapeExpr) -> Result<(), String> {
        match expr {
            ShapeExpr::And(parts) => parts
                .iter()
                .try_for_each(|part| self.satisfies(focus, part)),
            ShapeExpr::Or(parts) => {
                let mut reasons = Vec::new();
                for part in parts {
                    match self.satisfies(focus, part) {
                        Ok(()) => return Ok(()),
                        Err(reason) => reasons.push(reason),
                    }
                }
                Err(format!("no OR branch matched: {}", reasons.join(" / ")))
            }
            ShapeExpr::Not(inner) => match self.satisfies(focus, inner) {
                Ok(()) => Err("NOT: negated expression matched".to_owned()),
                Err(_) => Ok(()),
            },
            ShapeExpr::Node(nc) => {
                let facts = match focus {
                    Focus::Id(id) => facts_of_id(self.data, id),
                    Focus::Detached(value) => facts_of_value(value),
                };
                node::check_node_constraint(nc, &facts)
            }
            ShapeExpr::Shape(shape) => self.match_shape(focus, shape),
            ShapeExpr::External => Err("EXTERNAL shape has no resolved definition".to_owned()),
            ShapeExpr::Ref(label) => self.satisfies_label(focus, label),
        }
    }

    /// Resolve and check a labeled shape, with coinductive-assumption
    /// recursion handling and per-call memoization.
    fn satisfies_label(&mut self, focus: Focus<'_>, label: &str) -> Result<(), String> {
        let Some((&interned_label, &expr)) = self.shape_map.get_key_value(label) else {
            return Err(format!("reference to undeclared shape {label}"));
        };
        let label_id = self.label_id(interned_label);
        let Focus::Id(id) = focus else {
            // A detached node has no arcs, so a labelled cycle can only be
            // reference-only; guard it and evaluate directly.
            if !self.detached_in_progress.insert(label_id) {
                return Ok(());
            }
            let result = self.satisfies(focus, expr);
            self.detached_in_progress.remove(&label_id);
            return result;
        };
        let key: Pair = (id, label_id);
        if let Some(settled) = self.memo.get(&key) {
            return settled.clone();
        }
        if self.in_progress.contains(&key) {
            // Coinductive assumption: a pair re-encountered while being
            // proven is assumed to hold (spec §5.3 typing semantics).
            self.used_assumptions.insert(key);
            return Ok(());
        }
        self.in_progress.insert(key);
        let saved_used = std::mem::take(&mut self.used_assumptions);
        let result = self.satisfies(focus, expr);
        self.in_progress.remove(&key);
        let mut used = std::mem::replace(&mut self.used_assumptions, saved_used);
        used.remove(&key);
        // Only a proof that leaned on no OTHER open assumption is settled;
        // one that did may be invalidated when the outer pair refutes.
        if used.is_empty() {
            self.memo.insert(key, result.clone());
        }
        self.used_assumptions.extend(used);
        result
    }

    // ── shape / triple-expression matching (§5.2, §5.5) ─────────────────────

    fn match_shape(&mut self, focus: Focus<'_>, shape: &'a Shape) -> Result<(), String> {
        let compiled = match &shape.expression {
            Some(expr) => matcher::compile(expr, &self.te_map)?,
            None => Compiled {
                root: CNode::Each(
                    Vec::new(),
                    Card {
                        min: 1,
                        max: Some(1),
                    },
                    Vec::new(),
                ),
                slots: Vec::new(),
            },
        };
        // Per-(predicate, direction) slot indexes, in deterministic order.
        let mut forward: BTreeMap<&'a str, Vec<usize>> = BTreeMap::new();
        let mut inverse: BTreeMap<&'a str, Vec<usize>> = BTreeMap::new();
        for (index, slot) in compiled.slots.iter().enumerate() {
            let bucket = if slot.inverse {
                &mut inverse
            } else {
                &mut forward
            };
            bucket.entry(slot.predicate).or_default().push(index);
        }

        // Neighbourhood: arcs out, plus arcs in for inverse-mentioned
        // predicates; sorted by TermId for determinism.
        let data = self.data;
        let mut arcs_out: Vec<(TermId, TermId)> = match focus {
            Focus::Id(id) => data
                .quads_for_pattern(Some(id), None, None, GraphMatch::Any)
                .map(|q| (q.p, q.o))
                .collect(),
            Focus::Detached(_) => Vec::new(),
        };
        arcs_out.sort_unstable();
        arcs_out.dedup();

        // (inverse?, predicate string, value node) for every matchable arc.
        let mut arcs: Vec<(bool, &'a str, TermId)> = Vec::new();
        for &(p, o) in &arcs_out {
            let pred = iri_str(data, p);
            if let Some((&interned, _)) = forward.get_key_value(pred) {
                arcs.push((false, interned, o));
            } else if shape.closed == Some(true) {
                return Err(format!("CLOSED shape does not mention predicate <{pred}>"));
            }
        }
        if let Focus::Id(id) = focus {
            for &pred in inverse.keys() {
                let Some(pid) = data.term_id_by_value(&TermValue::iri(pred)) else {
                    continue;
                };
                let mut subjects: Vec<TermId> = data
                    .quads_for_pattern(None, Some(pid), Some(id), GraphMatch::Any)
                    .map(|q| q.s)
                    .collect();
                subjects.sort_unstable();
                subjects.dedup();
                arcs.extend(subjects.into_iter().map(|s| (true, pred, s)));
            }
        }

        // Candidate slots per arc (value expressions checked recursively).
        let mut options: Vec<ArcOptions> = Vec::with_capacity(arcs.len());
        let mut value_failures: Vec<String> = Vec::new();
        for &(inv, pred, value) in &arcs {
            let slots = if inv { &inverse[pred] } else { &forward[pred] };
            let mut candidates = Vec::new();
            for &slot in slots {
                match compiled.slots[slot].value_expr {
                    None => candidates.push(slot),
                    Some(ve) => match self.satisfies(Focus::Id(value), ve) {
                        Ok(()) => candidates.push(slot),
                        Err(reason) => value_failures.push(format!(
                            "value of {}<{pred}> fails: {reason}",
                            if inv { "^" } else { "" }
                        )),
                    },
                }
            }
            // EXTRA diversion (spec §5.2): an unmatched arc is permitted
            // only when its predicate is EXTRA **and** it matches no triple
            // constraint — an arc that satisfies some constraint's value
            // expression must be matched (and counts against cardinality).
            let extra_allowed = candidates.is_empty() && shape.extra.iter().any(|e| e == pred);
            if candidates.is_empty() && !extra_allowed {
                return Err(value_failures.pop().unwrap_or_else(|| {
                    format!("triple with predicate <{pred}> cannot be matched")
                }));
            }
            options.push(ArcOptions {
                candidates,
                extra_allowed,
            });
        }

        // Fast path: every (predicate, direction) lives in exactly one slot,
        // so the assignment is forced up to EXTRA diversion and per-slot
        // counts collapse to intervals.
        let unique = forward
            .values()
            .chain(inverse.values())
            .all(|v| v.len() == 1);
        let matched = if unique {
            let mut counts = vec![(0u64, 0u64); compiled.slots.len()];
            for option in &options {
                // An arc with a candidate MUST be matched (EXTRA never
                // diverts a matching arc); a candidate-less arc was already
                // vetted as EXTRA-divertible above and consumes nothing.
                if let Some(&slot) = option.candidates.first() {
                    counts[slot].0 += 1;
                    counts[slot].1 += 1;
                }
            }
            matcher::counts_match(&compiled, &counts).then_some(counts)
        } else {
            matcher::assignment_search(&compiled, &options)?
        };
        let Some(counts) = matched else {
            return Err(cardinality_reason(&compiled, &arcs, &value_failures));
        };
        // The neighbourhood matched; fire semantic actions (§5.5.2).
        self.fire_sem_acts(focus, shape, &compiled, &counts)
    }

    /// Dispatch the semantic actions that a successful shape match triggers:
    /// each matched triple constraint's actions, the expression's group
    /// actions, then the shape's own actions. A failing action fails the
    /// match. A no-op when no extension is registered.
    fn fire_sem_acts(
        &self,
        focus: Focus<'_>,
        shape: &Shape,
        compiled: &Compiled<'_>,
        counts: &[(u64, u64)],
    ) -> Result<(), String> {
        if self.sem_acts.is_empty() {
            return Ok(());
        }
        let focus_value = self.focus_value(focus);
        for (index, slot) in compiled.slots.iter().enumerate() {
            if counts[index].0 == 0 || slot.sem_acts.is_empty() {
                continue;
            }
            let ctx = SemActContext {
                focus: Some(focus_value.clone()),
                predicate: Some(slot.predicate.to_owned()),
                value: None,
            };
            if !self.sem_acts.dispatch_all(slot.sem_acts, &ctx) {
                return Err(format!(
                    "semantic action failed on {}<{}>",
                    if slot.inverse { "^" } else { "" },
                    slot.predicate
                ));
            }
        }
        let group_ctx = SemActContext {
            focus: Some(focus_value.clone()),
            predicate: None,
            value: None,
        };
        for act in matcher::participating_group_acts(compiled, counts) {
            if !self.sem_acts.dispatch(act, &group_ctx) {
                return Err("group semantic action failed".to_owned());
            }
        }
        let shape_ctx = SemActContext {
            focus: Some(focus_value),
            predicate: None,
            value: None,
        };
        if !self.sem_acts.dispatch_all(&shape.sem_acts, &shape_ctx) {
            return Err("shape semantic action failed".to_owned());
        }
        Ok(())
    }

    /// The focus node as an owned [`TermValue`].
    fn focus_value(&self, focus: Focus<'_>) -> TermValue {
        match focus {
            Focus::Id(id) => self.data.term_value(id),
            Focus::Detached(value) => value.clone(),
        }
    }
}

/// A best-effort failure message when the triple expression cannot consume
/// the neighbourhood: per-slot counts against declared cardinalities.
fn cardinality_reason(
    compiled: &Compiled<'_>,
    arcs: &[(bool, &str, TermId)],
    value_failures: &[String],
) -> String {
    let mut parts = Vec::new();
    for slot in &compiled.slots {
        let count = arcs
            .iter()
            .filter(|(inv, pred, _)| *inv == slot.inverse && *pred == slot.predicate)
            .count();
        let max = slot
            .card
            .max
            .map_or_else(|| "*".to_owned(), |m| m.to_string());
        parts.push(format!(
            "{}<{}> has {count} triple(s) for cardinality {{{},{max}}}",
            if slot.inverse { "^" } else { "" },
            slot.predicate,
            slot.card.min,
        ));
    }
    let mut reason = format!("triple expression not matched: {}", parts.join("; "));
    if let Some(failure) = value_failures.last() {
        reason.push_str("; ");
        reason.push_str(failure);
    }
    reason
}

// ── node facts extraction ───────────────────────────────────────────────────

/// The IRI string behind a term id (predicates and datatypes are always
/// IRIs in a frozen dataset).
fn iri_str(data: &RdfDataset, id: TermId) -> &str {
    match data.resolve(id) {
        TermRef::Iri(iri) => iri,
        _ => "",
    }
}

fn facts_of_id(data: &RdfDataset, id: TermId) -> NodeFacts<'_> {
    match data.resolve(id) {
        TermRef::Iri(iri) => NodeFacts {
            kind: FactKind::Iri,
            lexical: iri,
            datatype: None,
            language: None,
        },
        TermRef::Blank { label, .. } => NodeFacts {
            kind: FactKind::Blank,
            lexical: label,
            datatype: None,
            language: None,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            ..
        } => NodeFacts {
            kind: FactKind::Literal,
            lexical,
            datatype: Some(iri_str(data, datatype)),
            language,
        },
        TermRef::Triple { .. } => NodeFacts {
            kind: FactKind::Triple,
            lexical: "",
            datatype: None,
            language: None,
        },
    }
}

fn facts_of_value(value: &TermValue) -> NodeFacts<'_> {
    match value {
        TermValue::Iri(iri) => NodeFacts {
            kind: FactKind::Iri,
            lexical: iri,
            datatype: None,
            language: None,
        },
        TermValue::Blank { label, .. } => NodeFacts {
            kind: FactKind::Blank,
            lexical: label,
            datatype: None,
            language: None,
        },
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        } => NodeFacts {
            kind: FactKind::Literal,
            lexical: lexical_form,
            datatype: Some(if language.is_some() {
                RDF_LANG_STRING
            } else {
                datatype.as_str()
            }),
            language: language.as_deref(),
        },
        TermValue::Triple { .. } => NodeFacts {
            kind: FactKind::Triple,
            lexical: "",
            datatype: None,
            language: None,
        },
    }
}
