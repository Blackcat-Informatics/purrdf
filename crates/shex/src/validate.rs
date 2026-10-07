// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! * **the RDF 1.2 statement layer** (a PurRDF extension beyond the ShEx 2.1
//!   arc model): a focus node's neighbourhood includes the reifier and
//!   statement-annotation side-tables as well as the quad table. Both
//!   side-tables put the *reifier* in subject position, so a focus node that
//!   IS a reifier gains an `rdf:reifies` arc to the triple term it reifies
//!   plus one arc per statement annotation, while an ordinary subject's
//!   neighbourhood is unchanged. Inverse arcs see the layer too
//!   (`^rdf:reifies` from a triple term, `^<annotationPredicate>` from an
//!   annotation object). Consequently a `CLOSED` shape whose focus is a
//!   reifier must mention `rdf:reifies` and every annotation predicate;
//! * **recursion** (§5.3): typing-based — a `(node, shape)` pair
//!   re-encountered while being proven is coinductively assumed to hold, and
//!   settled pairs are memoized per validation call. Negation through
//!   recursion is safe because [`crate::structure`] enforces stratification.
//!
//! Iteration order is deterministic everywhere (arcs sort by [`TermId`],
//! slots by document order), so failure reasons are reproducible.

mod error;
mod matcher;
mod node;
mod pattern;

use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_core::{
    DatasetView, FastMap, FastSet, GraphMatch, RdfDataset, RdfTextDirection, TermId, TermRef,
    TermValue,
};
use purrdf_lex::json_escape::{JsonEscapes, push_string};
use purrdf_lex::term_syntax;

use crate::ast::{Schema, SemAct, Shape, ShapeExpr, TripleExpr};
use crate::semact::{SemActContext, SemActRegistry};
use crate::statement;
use error::CheckError;
use matcher::{ArcOptions, Assignment, CNode, Card, Compiled};
use node::{FactKind, NodeFacts, RDF_LANG_STRING};
use purrdf_core::xsd_regex::xpath;

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
            push_string(
                &mut out,
                &node_term_string(&entry.node),
                JsonEscapes::ShortForms,
            );
            out.push_str(",\"shape\":");
            push_string(
                &mut out,
                &shape_term_string(&entry.shape),
                JsonEscapes::ShortForms,
            );
            out.push_str(",\"status\":");
            push_string(&mut out, status_str(entry.status), JsonEscapes::ShortForms);
            if let Some(reason) = &entry.reason {
                out.push_str(",\"reason\":");
                push_string(&mut out, reason, JsonEscapes::ShortForms);
            }
            out.push('}');
        }
        out.push(']');
        out
    }
}

/// The result-map spelling of a conformance status.
fn status_str(status: ConformanceStatus) -> &'static str {
    match status {
        ConformanceStatus::Conformant => "conformant",
        ConformanceStatus::Nonconformant => "nonconformant",
    }
}

/// A term in the shape-map term syntax: the RDF 1.2 canonical term form of
/// [`purrdf_lex::term_syntax`] (`<iri>`, `_:label`, a literal with its language
/// tag and base direction), each triple term spelled `<<( s p o )>>` over
/// [`TermValue::try_write_nested`]'s work list. [`crate::shapemap::parse_shape_map`]
/// reads every spelling this writes back to the same term.
pub(crate) fn node_term_string(value: &TermValue) -> String {
    let mut out = String::new();
    let open = format!("{} ", term_syntax::TRIPLE_TERM_OPEN);
    let close = format!(" {}", term_syntax::TRIPLE_TERM_CLOSE);
    let written = value.try_write_nested(
        &mut out,
        &open,
        " ",
        &close,
        |out, leaf| {
            write_leaf_term(leaf, out);
            Ok::<(), std::convert::Infallible>(())
        },
        |out, text| {
            out.push_str(text);
            Ok(())
        },
    );
    match written {
        Ok(()) => out,
    }
}

/// [`node_term_string`] for a term that is not a triple term.
fn write_leaf_term(value: &TermValue, out: &mut String) {
    match value {
        TermValue::Iri(iri) => term_syntax::write_iri(iri, out),
        TermValue::Blank { label, .. } => term_syntax::write_blank(label, out),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => term_syntax::write_literal(
            lexical_form,
            datatype,
            language.as_deref(),
            direction.map(RdfTextDirection::as_str),
            out,
        ),
        TermValue::Triple { .. } => unreachable!("a triple term is written from its parts"),
    }
}

/// A shape label in the shape-map syntax (`START` / `<label>` / `_:label`).
pub(crate) fn shape_term_string(shape: &ShapeSelector) -> String {
    match shape {
        ShapeSelector::Start => "START".to_owned(),
        ShapeSelector::Label(label) if label.starts_with("_:") => label.clone(),
        ShapeSelector::Label(label) => {
            let mut out = String::new();
            term_syntax::write_iri(label, &mut out);
            out
        }
    }
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
///
/// # Examples
///
/// ```
/// use purrdf_core::TermValue;
/// use purrdf_rdf::parse_dataset;
/// use purrdf_shex::{ConformanceStatus, ShapeSelector, parse_shexc, validate};
///
/// let schema = parse_shexc(
///     "<http://example.org/UserShape> { <http://example.org/name> LITERAL }",
///     None,
/// )
/// .expect("a well-formed schema parses");
///
/// let data = parse_dataset(
///     b"<http://example.org/alice> <http://example.org/name> \"Alice\" .",
///     "text/turtle",
///     None,
/// )
/// .expect("a well-formed graph parses");
///
/// let map = vec![(
///     TermValue::Iri("http://example.org/alice".to_string()),
///     ShapeSelector::Label("http://example.org/UserShape".to_string()),
/// )];
/// let result = validate(&schema, &data, &map);
/// assert_eq!(result.entries.len(), 1);
/// assert_eq!(result.entries[0].status, ConformanceStatus::Conformant);
/// ```
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
    validate_using(
        schema,
        data,
        map,
        options,
        None,
        &mut pattern::PatternCache::default(),
    )
    .expect("compatibility validation cannot produce a native XPath refusal")
}

/// The exact numeric facet bounds of an [`crate::ExactSchema`], by node-constraint
/// address, in the order `MININCLUSIVE`, `MINEXCLUSIVE`, `MAXINCLUSIVE`,
/// `MAXEXCLUSIVE`.
pub(crate) type ExactBoundsMap = FastMap<usize, [Option<purrdf_xsd::XsdValue>; 4]>;

/// [`validate_with`], comparing numeric facets against `bounds` where a node
/// constraint has an entry, and against its `i64`/`f64` AST values otherwise.
pub(crate) fn validate_with_bounds(
    schema: &Schema,
    data: &RdfDataset,
    map: &[(TermValue, ShapeSelector)],
    options: &ValidationOptions<'_>,
    bounds: Option<&ExactBoundsMap>,
) -> ResultShapeMap {
    validate_using(
        schema,
        data,
        map,
        options,
        bounds,
        &mut pattern::PatternCache::default(),
    )
    .expect("compatibility validation cannot produce a native XPath refusal")
}

/// A reusable schema whose selected native patterns retain successful programs.
///
/// The schema is borrowed immutably. Each validation takes an explicit dated law
/// and current finite limits; only admitted programs survive between calls, never
/// conformance findings, syntax verdicts, execution fuel or resource refusals.
#[derive(Debug)]
pub struct XPathValidator<'a> {
    schema: &'a Schema,
    patterns: pattern::PatternCache,
}

impl<'a> XPathValidator<'a> {
    /// Prepare native pattern reuse for an immutable schema.
    #[must_use]
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            patterns: pattern::PatternCache::default(),
        }
    }

    /// Validate a fixed map under the explicitly selected native XPath law.
    ///
    /// Syntax and flag errors remain facet findings. Operational refusal aborts
    /// the whole map, including previously computed associations. A later call
    /// can use another law or larger bounds without inheriting that refusal.
    ///
    /// # Errors
    /// Returns the actual typed native XPath resource or allocation cause.
    pub fn validate(
        &mut self,
        data: &RdfDataset,
        map: &[(TermValue, ShapeSelector)],
        options: &ValidationOptions<'_>,
        profile: xpath::Profile,
        limits: xpath::Limits,
    ) -> Result<ResultShapeMap, xpath::Error> {
        self.patterns.select(profile, limits);
        validate_using(self.schema, data, map, options, None, &mut self.patterns)
    }
}

/// [`validate_with`] using an explicit native XPath law and finite limits.
///
/// This uses the same traversal as [`XPathValidator::validate`]. For repeated
/// validations, keep an [`XPathValidator`] to reuse admitted programs.
///
/// # Errors
/// A typed operational refusal; pattern-language errors remain findings.
pub fn validate_with_xpath(
    schema: &Schema,
    data: &RdfDataset,
    map: &[(TermValue, ShapeSelector)],
    options: &ValidationOptions<'_>,
    profile: xpath::Profile,
    limits: xpath::Limits,
) -> Result<ResultShapeMap, xpath::Error> {
    XPathValidator::new(schema).validate(data, map, options, profile, limits)
}

/// [`validate_with_xpath`], comparing numeric facets against `bounds` where a node
/// constraint has an entry, as [`validate_with_bounds`] does.
pub(crate) fn validate_with_xpath_bounds(
    schema: &Schema,
    data: &RdfDataset,
    map: &[(TermValue, ShapeSelector)],
    options: &ValidationOptions<'_>,
    profile: xpath::Profile,
    limits: xpath::Limits,
    bounds: Option<&ExactBoundsMap>,
) -> Result<ResultShapeMap, xpath::Error> {
    let mut patterns = pattern::PatternCache::default();
    patterns.select(profile, limits);
    validate_using(schema, data, map, options, bounds, &mut patterns)
}

fn validate_using(
    schema: &Schema,
    data: &RdfDataset,
    map: &[(TermValue, ShapeSelector)],
    options: &ValidationOptions<'_>,
    bounds: Option<&ExactBoundsMap>,
    patterns: &mut pattern::PatternCache,
) -> Result<ResultShapeMap, xpath::Error> {
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
    // They run once for the whole shape map rather than per association, so
    // no single focus node is in scope here — `focus`/`predicate`/`value`
    // all stay `None`.
    let start_ctx = SemActContext::default();
    if !options
        .sem_acts
        .dispatch_all(&schema.start_acts, &start_ctx)
        || !options
            .sem_acts
            .dispatch_all(options.extern_start_acts, &start_ctx)
    {
        return Ok(ResultShapeMap {
            entries: map
                .iter()
                .map(|(value, selector)| ResultEntry {
                    node: value.clone(),
                    shape: selector.clone(),
                    status: ConformanceStatus::Nonconformant,
                    reason: Some("start semantic action failed".to_owned()),
                })
                .collect(),
        });
    }

    let mut engine = Engine::new(
        schema,
        data,
        &externals,
        &options.sem_acts,
        std::mem::take(patterns),
        bounds,
    );
    let result = map
        .iter()
        .map(|(value, selector)| {
            let reason = match engine.check_association(value, selector) {
                Ok(()) => None,
                Err(CheckError::Violation(reason)) => Some(reason),
                Err(CheckError::Operational(error)) => return Err(error),
            };
            Ok(ResultEntry {
                node: value.clone(),
                shape: selector.clone(),
                status: if reason.is_none() {
                    ConformanceStatus::Conformant
                } else {
                    ConformanceStatus::Nonconformant
                },
                reason,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|entries| ResultShapeMap { entries });
    *patterns = engine.patterns;
    result
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
    shape_map: FastMap<&'a str, &'a ShapeExpr>,
    prepared_shapes: FastMap<*const Shape, Result<Arc<PreparedShape<'a>>, String>>,
    label_ids: FastMap<&'a str, u32>,
    /// Settled `(node, shape)` verdicts for this validation call.
    memo: FastMap<Pair, Result<(), String>>,
    /// Pairs currently being proven (the coinductive assumption set).
    in_progress: FastSet<Pair>,
    /// In-progress pairs whose assumption the current proof relied on.
    used_assumptions: FastSet<Pair>,
    /// Labels being proven for a detached focus (cycle guard).
    detached_in_progress: FastSet<u32>,
    /// Compiled `PATTERN` facets for this validation call. The facet is
    /// checked per value node, so without the memo a `PATTERN` over a large
    /// neighbourhood recompiles the same regex once per value.
    patterns: pattern::PatternCache,
    /// The exact numeric facet bounds of an [`crate::ExactSchema`], by
    /// node-constraint address; `None` compares against the AST's `i64`/`f64`.
    bounds: Option<&'a ExactBoundsMap>,
}

struct PreparedShape<'a> {
    compiled: Compiled<'a>,
    forward: BTreeMap<&'a str, Vec<usize>>,
    inverse: BTreeMap<&'a str, Vec<usize>>,
    unique: bool,
}

fn prepare_shape<'a>(
    shape: &'a Shape,
    te_map: &FastMap<&'a str, &'a TripleExpr>,
) -> Result<PreparedShape<'a>, String> {
    let compiled = match &shape.expression {
        Some(expr) => matcher::compile(expr, te_map)?,
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
    let mut forward = BTreeMap::new();
    let mut inverse = BTreeMap::new();
    for (index, slot) in compiled.slots.iter().enumerate() {
        let bucket = if slot.inverse {
            &mut inverse
        } else {
            &mut forward
        };
        bucket
            .entry(slot.predicate)
            .or_insert_with(Vec::new)
            .push(index);
    }
    let unique = forward
        .values()
        .chain(inverse.values())
        .all(|slots| slots.len() == 1);
    Ok(PreparedShape {
        compiled,
        forward,
        inverse,
        unique,
    })
}

fn collect_shapes<'a>(expr: &'a ShapeExpr, shapes: &mut Vec<&'a Shape>) {
    match expr {
        ShapeExpr::And(parts) | ShapeExpr::Or(parts) => {
            for part in parts {
                collect_shapes(part, shapes);
            }
        }
        ShapeExpr::Not(inner) => collect_shapes(inner, shapes),
        ShapeExpr::Shape(shape) => {
            shapes.push(shape);
            if let Some(expr) = &shape.expression {
                collect_triple_expr_shapes(expr, shapes);
            }
        }
        ShapeExpr::Node(_) | ShapeExpr::External | ShapeExpr::Ref(_) => {}
    }
}

fn collect_triple_expr_shapes<'a>(expr: &'a TripleExpr, shapes: &mut Vec<&'a Shape>) {
    match expr {
        TripleExpr::EachOf(group) | TripleExpr::OneOf(group) => {
            for child in &group.expressions {
                collect_triple_expr_shapes(child, shapes);
            }
        }
        TripleExpr::TripleConstraint(constraint) => {
            if let Some(value_expr) = &constraint.value_expr {
                collect_shapes(value_expr, shapes);
            }
        }
        TripleExpr::Ref(_) => {}
    }
}

impl<'a> Engine<'a> {
    fn new(
        schema: &'a Schema,
        data: &'a RdfDataset,
        externals: &'a [(String, ShapeExpr)],
        sem_acts: &'a SemActRegistry<'a>,
        patterns: pattern::PatternCache,
        bounds: Option<&'a ExactBoundsMap>,
    ) -> Self {
        let mut shape_map: FastMap<&'a str, &'a ShapeExpr> = schema
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
        let te_map: FastMap<&'a str, &'a TripleExpr> = te_labels.into_iter().collect();
        let mut shapes = Vec::new();
        for expr in shape_map.values() {
            collect_shapes(expr, &mut shapes);
        }
        for expr in te_map.values() {
            collect_triple_expr_shapes(expr, &mut shapes);
        }
        if let Some(start) = schema.start.as_deref() {
            collect_shapes(start, &mut shapes);
        }
        let mut prepared_shapes = FastMap::default();
        for shape in shapes {
            prepared_shapes
                .entry(std::ptr::from_ref(shape))
                .or_insert_with(|| prepare_shape(shape, &te_map).map(Arc::new));
        }
        Self {
            data,
            sem_acts,
            start: schema.start.as_deref(),
            shape_map,
            prepared_shapes,
            label_ids: FastMap::default(),
            memo: FastMap::default(),
            in_progress: FastSet::default(),
            used_assumptions: FastSet::default(),
            detached_in_progress: FastSet::default(),
            patterns,
            bounds,
        }
    }

    fn check_association(
        &mut self,
        value: &TermValue,
        selector: &ShapeSelector,
    ) -> Result<(), CheckError> {
        let focus = match self.data.term_id_by_value(value) {
            Some(id) => Focus::Id(id),
            None => Focus::Detached(value),
        };
        match selector {
            ShapeSelector::Start => {
                let Some(start) = self.start else {
                    return Err("schema declares no start shape".to_owned().into());
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

    fn satisfies(&mut self, focus: Focus<'_>, expr: &'a ShapeExpr) -> Result<(), CheckError> {
        match expr {
            ShapeExpr::And(parts) => parts
                .iter()
                .try_for_each(|part| self.satisfies(focus, part)),
            ShapeExpr::Or(parts) => {
                let mut reasons = Vec::new();
                for part in parts {
                    match self.satisfies(focus, part) {
                        Ok(()) => return Ok(()),
                        Err(CheckError::Violation(reason)) => reasons.push(reason),
                        Err(error @ CheckError::Operational(_)) => return Err(error),
                    }
                }
                Err(format!("no OR branch matched: {}", reasons.join(" / ")).into())
            }
            ShapeExpr::Not(inner) => match self.satisfies(focus, inner) {
                Ok(()) => Err("NOT: negated expression matched".to_owned().into()),
                Err(CheckError::Violation(_)) => Ok(()),
                Err(error @ CheckError::Operational(_)) => Err(error),
            },
            ShapeExpr::Node(nc) => {
                let facts = match focus {
                    Focus::Id(id) => facts_of_id(self.data, id),
                    Focus::Detached(value) => facts_of_value(value),
                };
                let exact = self
                    .bounds
                    .and_then(|bounds| bounds.get(&(std::ptr::from_ref(nc) as usize)));
                node::check_node_constraint(nc, &facts, &mut self.patterns, exact)
            }
            ShapeExpr::Shape(shape) => self.match_shape(focus, shape),
            ShapeExpr::External => Err("EXTERNAL shape has no resolved definition"
                .to_owned()
                .into()),
            ShapeExpr::Ref(label) => self.satisfies_label(focus, label),
        }
    }

    /// Resolve and check a labeled shape, with coinductive-assumption
    /// recursion handling and per-call memoization.
    fn satisfies_label(&mut self, focus: Focus<'_>, label: &str) -> Result<(), CheckError> {
        let Some((&interned_label, &expr)) = self.shape_map.get_key_value(label) else {
            return Err(format!("reference to undeclared shape {label}").into());
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
            return settled.clone().map_err(CheckError::Violation);
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
            match &result {
                Ok(()) => {
                    self.memo.insert(key, Ok(()));
                }
                Err(CheckError::Violation(reason)) => {
                    self.memo.insert(key, Err(reason.clone()));
                }
                Err(CheckError::Operational(_)) => {}
            }
        }
        self.used_assumptions.extend(used);
        result
    }

    // ── shape / triple-expression matching (§5.2, §5.5) ─────────────────────

    fn match_shape(&mut self, focus: Focus<'_>, shape: &'a Shape) -> Result<(), CheckError> {
        let prepared = self
            .prepared_shapes
            .get(&std::ptr::from_ref(shape))
            .expect("every structural shape is prepared at engine construction")
            .clone()?;
        let compiled = &prepared.compiled;
        let forward = &prepared.forward;
        let inverse = &prepared.inverse;

        // Neighbourhood: arcs out, plus arcs in for inverse-mentioned
        // predicates; sorted by TermId for determinism.
        let data = self.data;
        let mut arcs_out: Vec<(TermId, TermId)> = match focus {
            Focus::Id(id) => {
                let mut out: Vec<(TermId, TermId)> = data
                    .quads_for_pattern(Some(id), None, None, GraphMatch::Any)
                    .map(|q| (q.p, q.o))
                    .collect();
                // RDF 1.2 statement layer (side-tables, not in `quads`): a
                // focus node that IS a reifier also has an `rdf:reifies` arc to
                // the triple term it reifies plus one arc per statement
                // annotation. Both layers key on the reifier as subject, so an
                // ordinary subject's neighbourhood is untouched.
                statement::visit_quads(data, Some(id), None, None, |_, p, o| out.push((p, o)));
                out
            }
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
                return Err(format!("CLOSED shape does not mention predicate <{pred}>").into());
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
                // Statement layer, inverse direction: `^rdf:reifies` from a
                // triple term reaches its reifier(s), and `^<annPred>` from an
                // annotation object reaches the annotated reifier.
                statement::visit_quads(data, None, Some(pid), Some(id), |s, _, _| {
                    subjects.push(s);
                });
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
                        Err(CheckError::Violation(reason)) => value_failures.push(format!(
                            "value of {}<{pred}> fails: {reason}",
                            if inv { "^" } else { "" }
                        )),
                        Err(error @ CheckError::Operational(_)) => return Err(error),
                    },
                }
            }
            // EXTRA diversion (spec §5.2): an unmatched arc is permitted
            // only when its predicate is EXTRA **and** it matches no triple
            // constraint — an arc that satisfies some constraint's value
            // expression must be matched (and counts against cardinality).
            let extra_allowed = candidates.is_empty() && shape.extra.iter().any(|e| e == pred);
            if candidates.is_empty() && !extra_allowed {
                return Err(value_failures
                    .pop()
                    .unwrap_or_else(|| format!("triple with predicate <{pred}> cannot be matched"))
                    .into());
            }
            options.push(ArcOptions {
                candidates,
                extra_allowed,
            });
        }

        // Fast path: every (predicate, direction) lives in exactly one slot,
        // so the assignment is forced up to EXTRA diversion and per-slot
        // counts collapse to intervals.
        let unique = prepared.unique;
        // `assignment[i]` is the slot arc `i` was routed to (`None` when
        // diverted to `EXTRA`), kept alongside `counts` so semantic actions
        // can be fired per matched arc rather than merely per slot.
        let (counts, assignment): Assignment = if unique {
            let mut counts = vec![(0u64, 0u64); compiled.slots.len()];
            let mut assignment = vec![None; options.len()];
            for (index, option) in options.iter().enumerate() {
                // An arc with a candidate MUST be matched (EXTRA never
                // diverts a matching arc); a candidate-less arc was already
                // vetted as EXTRA-divertible above and consumes nothing.
                if let Some(&slot) = option.candidates.first() {
                    counts[slot].0 += 1;
                    counts[slot].1 += 1;
                    assignment[index] = Some(slot);
                }
            }
            if !matcher::counts_match(compiled, &counts) {
                return Err(cardinality_reason(compiled, &arcs, &value_failures).into());
            }
            (counts, assignment)
        } else {
            match matcher::assignment_search(compiled, &options)? {
                Some(found) => found,
                None => return Err(cardinality_reason(compiled, &arcs, &value_failures).into()),
            }
        };
        // Per-slot matched value nodes, in arc order, for firing triple-
        // constraint semantic actions once per matched arc (§5.5.2).
        let mut matched_values: Vec<Vec<TermId>> = vec![Vec::new(); compiled.slots.len()];
        for (index, slot) in assignment.iter().enumerate() {
            if let Some(slot) = slot {
                matched_values[*slot].push(arcs[index].2);
            }
        }
        // The neighbourhood matched; fire semantic actions (§5.5.2).
        self.fire_sem_acts(focus, shape, compiled, &counts, &matched_values)
            .map_err(CheckError::from)
    }

    /// Dispatch the semantic actions that a successful shape match triggers:
    /// each matched triple constraint's actions (once per matched arc), the
    /// expression's group actions, then the shape's own actions. A failing
    /// action fails the match. A no-op when no extension is registered.
    fn fire_sem_acts(
        &self,
        focus: Focus<'_>,
        shape: &Shape,
        compiled: &Compiled<'_>,
        counts: &[(u64, u64)],
        matched_values: &[Vec<TermId>],
    ) -> Result<(), String> {
        if self.sem_acts.is_empty() {
            return Ok(());
        }
        let focus_value = self.focus_value(focus);
        for (index, slot) in compiled.slots.iter().enumerate() {
            if counts[index].0 == 0 || slot.sem_acts.is_empty() {
                continue;
            }
            // Fire once per matched arc so `ctx.value` names the actual
            // object (or, for `^`, subject) node of that triple.
            for &value_id in &matched_values[index] {
                let ctx = SemActContext {
                    focus: Some(focus_value.clone()),
                    predicate: Some(slot.predicate.to_owned()),
                    value: Some(self.data.term_value(value_id)),
                };
                if !self.sem_acts.dispatch_all(slot.sem_acts, &ctx) {
                    return Err(format!(
                        "semantic action failed on {}<{}>",
                        if slot.inverse { "^" } else { "" },
                        slot.predicate
                    ));
                }
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
