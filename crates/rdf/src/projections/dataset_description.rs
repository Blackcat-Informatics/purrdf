// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared deterministic RDF dataset-description serialization.

use std::sync::Arc;

use purrdf_core::{DatasetView, LossLedger, RdfDataset, SparqlResult, TermRef};
use purrdf_sparql_algebra::{
    AggregateFunction, Expression, Function, GraphPattern, OrderExpression, ParserOptions,
    PurrdfFn, Query, SparqlParser, TermPattern, TriplePattern,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
use serde::{Deserialize, Deserializer, Serialize};

use crate::native_codecs::{NativeRdfFormat, serialize_dataset_to_format};

use super::{ProjectionError, ProjectionLimits, ProjectionPackage};

/// Mandatory bounds and query text for a whole-dataset SPARQL CONSTRUCT view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConstructViewConfig {
    query: String,
    base_iri: Option<String>,
    limits: ProjectionLimits,
    max_query_bytes: usize,
    max_input_records: usize,
    max_output_records: usize,
}

impl ConstructViewConfig {
    /// Construct and parse a bounded CONSTRUCT-view policy, under
    /// [`ParserOptions::default`] (the property-function seam off — every
    /// predicate position parses as an ordinary term reference).
    ///
    /// # Errors
    ///
    /// Rejects zero/non-portable limits, an oversized or malformed query, a query
    /// form other than CONSTRUCT, or a non-absolute caller base IRI.
    #[allow(
        clippy::too_many_arguments,
        reason = "all independent resource and query policies are mandatory"
    )]
    pub fn new(
        query: impl Into<String>,
        base_iri: Option<String>,
        limits: ProjectionLimits,
        max_query_bytes: usize,
        max_input_records: usize,
        max_output_records: usize,
    ) -> Result<Self, ProjectionError> {
        Self::new_with_parser_options(
            query,
            base_iri,
            limits,
            max_query_bytes,
            max_input_records,
            max_output_records,
            &ParserOptions::default(),
        )
    }

    /// [`Self::new`] with explicit [`ParserOptions`] (e.g. a caller's
    /// property-function seam) applied to the reproducibility-validation parse.
    ///
    /// A predicate position `parser_options` recognizes as a property-function
    /// call parses as [`GraphPattern::PropertyFunction`] rather than an ordinary
    /// triple pattern, so the crate's reproducibility check can apply its
    /// documented conservative refusal to it — see the `PropertyFunction` arm of
    /// the pattern walk below. Under the default (empty) options that arm never
    /// fires, because the parser has nothing to recognize the predicate by.
    ///
    /// # Errors
    ///
    /// Same as [`Self::new`].
    #[allow(
        clippy::too_many_arguments,
        reason = "all independent resource and query policies are mandatory"
    )]
    pub fn new_with_parser_options(
        query: impl Into<String>,
        base_iri: Option<String>,
        limits: ProjectionLimits,
        max_query_bytes: usize,
        max_input_records: usize,
        max_output_records: usize,
        parser_options: &ParserOptions,
    ) -> Result<Self, ProjectionError> {
        let query = query.into();
        validate_portable_bound(max_query_bytes, "CONSTRUCT max_query_bytes")?;
        validate_portable_bound(max_input_records, "CONSTRUCT max_input_records")?;
        validate_portable_bound(max_output_records, "CONSTRUCT max_output_records")?;
        if query.is_empty() {
            return Err(ProjectionError::configuration(
                "CONSTRUCT query must not be empty",
            ));
        }
        if query.len() > max_query_bytes {
            return Err(ProjectionError::limit(format!(
                "CONSTRUCT query is {} bytes; limit is {max_query_bytes}",
                query.len()
            )));
        }
        if let Some(base_iri) = &base_iri {
            super::validate_absolute_iri(base_iri, "CONSTRUCT base IRI")?;
        }
        parse_construct(&query, base_iri.as_deref(), parser_options)?;
        Ok(Self {
            query,
            base_iri,
            limits,
            max_query_bytes,
            max_input_records,
            max_output_records,
        })
    }

    /// Caller-supplied SPARQL CONSTRUCT text.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Explicit caller base IRI, or `None` when the query must be self-contained.
    pub fn base_iri(&self) -> Option<&str> {
        self.base_iri.as_deref()
    }

    /// Shared projection package bounds.
    pub const fn limits(&self) -> ProjectionLimits {
        self.limits
    }

    /// Maximum accepted query bytes.
    pub const fn max_query_bytes(&self) -> usize {
        self.max_query_bytes
    }

    /// Maximum combined source records visible to evaluation.
    pub const fn max_input_records(&self) -> usize {
        self.max_input_records
    }

    /// Maximum records materialized by CONSTRUCT.
    pub const fn max_output_records(&self) -> usize {
        self.max_output_records
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RequiredNullableString {
    Value(String),
    Null(()),
}

impl RequiredNullableString {
    fn into_option(self) -> Option<String> {
        match self {
            Self::Value(value) => Some(value),
            Self::Null(()) => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConstructViewConfig {
    query: String,
    base_iri: RequiredNullableString,
    limits: ProjectionLimits,
    max_query_bytes: usize,
    max_input_records: usize,
    max_output_records: usize,
}

impl<'de> Deserialize<'de> for ConstructViewConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawConstructViewConfig::deserialize(deserializer)?;
        Self::new(
            raw.query,
            raw.base_iri.into_option(),
            raw.limits,
            raw.max_query_bytes,
            raw.max_input_records,
            raw.max_output_records,
        )
        .map_err(serde::de::Error::custom)
    }
}

/// Materialized result of one bounded whole-dataset CONSTRUCT view.
#[derive(Debug, Clone)]
pub struct ConstructViewProjection {
    /// Frozen CONSTRUCT result graph.
    pub dataset: Arc<RdfDataset>,
    /// Combined source record count charged to the input limit.
    pub input_records: usize,
    /// Combined result record count charged to the output limit.
    pub output_records: usize,
}

/// Evaluate a caller-supplied SPARQL CONSTRUCT over any static RDF 1.2 dataset view.
///
/// # Errors
///
/// Returns a typed resource, syntax, or integrity error when source/result bounds are
/// exceeded, preparation/evaluation fails, or the engine violates the graph-result
/// and default-graph CONSTRUCT contract.
pub fn project_construct_view<D: DatasetView + Sync>(
    view: &D,
    config: &ConstructViewConfig,
) -> Result<ConstructViewProjection, ProjectionError> {
    let input_records = view_record_count(view, "CONSTRUCT input")?;
    if input_records > config.max_input_records() {
        return Err(ProjectionError::limit(format!(
            "CONSTRUCT input has {input_records} records; limit is {}",
            config.max_input_records()
        )));
    }

    let engine = NativeSparqlEngine::default();
    let prepared = engine
        .prepare_query(config.query(), config.base_iri())
        .map_err(|error| ProjectionError::syntax(format!("prepare CONSTRUCT query: {error}")))?;
    if !matches!(prepared.query, Query::Construct { .. }) {
        return Err(ProjectionError::configuration(
            "dataset-description query must be SPARQL CONSTRUCT",
        ));
    }
    let result = engine
        .query_prepared_view(view, &prepared, &[], QueryOptions::EMPTY)
        .map_err(|error| {
            ProjectionError::integrity(format!("evaluate CONSTRUCT query: {error}"))
        })?;
    let SparqlResult::Graph(dataset) = result else {
        return Err(ProjectionError::integrity(
            "SPARQL CONSTRUCT evaluation returned a non-graph result",
        ));
    };
    if dataset.named_graphs().next().is_some() {
        return Err(ProjectionError::integrity(
            "SPARQL CONSTRUCT result unexpectedly contains named graphs",
        ));
    }
    ensure_blank_free(dataset.as_ref(), config.limits())?;
    let output_records = view_record_count(dataset.as_ref(), "CONSTRUCT output")?;
    if output_records > config.max_output_records() {
        return Err(ProjectionError::limit(format!(
            "CONSTRUCT output has {output_records} records; limit is {}",
            config.max_output_records()
        )));
    }
    Ok(ConstructViewProjection {
        dataset,
        input_records,
        output_records,
    })
}

fn parse_construct(
    query: &str,
    base_iri: Option<&str>,
    parser_options: &ParserOptions,
) -> Result<(), ProjectionError> {
    let parser = if let Some(base_iri) = base_iri {
        SparqlParser::new().with_base_iri(base_iri)
    } else {
        SparqlParser::new()
    };
    let parsed = parser
        .parse_query_with(query, parser_options)
        .map_err(|error| ProjectionError::syntax(format!("parse CONSTRUCT query: {error}")))?;
    validate_reproducible_construct(&parsed)
}

fn validate_reproducible_construct(query: &Query) -> Result<(), ProjectionError> {
    let Query::Construct {
        template, pattern, ..
    } = query
    else {
        return Err(ProjectionError::configuration(
            "dataset-description query must be SPARQL CONSTRUCT",
        ));
    };
    if template.iter().any(triple_pattern_contains_blank) {
        return Err(ProjectionError::configuration(
            "dataset-description CONSTRUCT templates must not mint blank nodes",
        ));
    }
    if let Some(cause) = pattern_reaches_non_reproducible_builtin(pattern) {
        return Err(ProjectionError::configuration(format!(
            "dataset-description CONSTRUCT must not {}",
            cause.reason()
        )));
    }
    Ok(())
}

/// Which non-reproducible SPARQL construct a CONSTRUCT WHERE pattern/expression walk
/// found, reachable from [`pattern_reaches_non_reproducible_builtin`] or
/// [`expression_reaches_non_reproducible_builtin`].
///
/// This replaces a plain `bool` the two walk functions used to return: every refusal
/// was funnelled through ONE message naming only `NOW`, `RAND`, `UUID`, `STRUUID`,
/// `BNODE`, and the blank-minting list functions — accurate for those five/two causes,
/// but three later-added causes (a custom aggregate call, a custom scalar-function
/// call, a `SERVICE` clause) reused the same boolean and inherited that message even
/// though none of them appears in it, so a caller was told the wrong reason for the
/// refusal. Each variant here names exactly one concrete construct, and
/// [`validate_reproducible_construct`] reports it directly instead of restating the
/// whole historical list. Mirrors the same fix already applied to the planner's two
/// admission seams (`crate::property_fn_plan`'s `PlanSeam`, in `purrdf-sparql-eval`):
/// a typed cause the raiser attaches, read at the one call site that turns it into a
/// message, instead of a boolean every call site has to reinterpret from scratch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NonReproducibleCause {
    /// `NOW()` — the result depends on wall-clock time.
    Now,
    /// `RAND()` — the result depends on the evaluator's RNG state.
    Rand,
    /// `UUID()` — mints a fresh, non-reproducible IRI.
    Uuid,
    /// `STRUUID()` — mints a fresh, non-reproducible literal.
    StrUuid,
    /// `BNODE()` — mints a fresh blank node identity.
    BNode,
    /// `purrdf:listSlice`/`purrdf:listConcat` — both mint a fresh `rdf:List` (a chain
    /// of fresh blank nodes) rather than returning an existing term.
    ListFunction(PurrdfFn),
    /// `AGG(<iri>, …)` — a host-supplied reduction whose reproducibility is a
    /// property of the registry the *evaluation* carries, unknowable here at
    /// shape-validation time — see `pattern_reaches_non_reproducible_builtin`'s
    /// `Group` arm for the full reasoning.
    CustomAggregate,
    /// `<iri>(args…)` outside every configured extension-function namespace — a
    /// host-supplied scalar function, the identical unknowable-reproducibility
    /// situation as [`Self::CustomAggregate`].
    CustomFunction,
    /// A property-function call recognized under the caller's configured
    /// `ParserOptions` — a host-supplied relation, the identical
    /// unknowable-reproducibility situation as [`Self::CustomAggregate`].
    PropertyFunction,
    /// A `SERVICE` (or `SERVICE SILENT`) clause — forwards to a live external
    /// endpoint whose data can change between calls.
    Service,
}

impl NonReproducibleCause {
    /// The clause embedded after `"dataset-description CONSTRUCT must not "` —
    /// names the exact construct this cause represents, never a restatement of every
    /// possible cause.
    fn reason(self) -> String {
        match self {
            Self::Now => "call NOW".to_owned(),
            Self::Rand => "call RAND".to_owned(),
            Self::Uuid => "call UUID".to_owned(),
            Self::StrUuid => "call STRUUID".to_owned(),
            Self::BNode => "call BNODE".to_owned(),
            Self::ListFunction(kind) => {
                format!("call the blank-minting list function {}", kind.local_name())
            }
            Self::CustomAggregate => "call a custom aggregate function".to_owned(),
            Self::CustomFunction => "call a custom scalar function".to_owned(),
            Self::PropertyFunction => "use a property-function call".to_owned(),
            Self::Service => "use a SERVICE clause".to_owned(),
        }
    }
}

fn triple_pattern_contains_blank(pattern: &TriplePattern) -> bool {
    term_pattern_contains_blank(&pattern.subject) || term_pattern_contains_blank(&pattern.object)
}

fn term_pattern_contains_blank(term: &TermPattern) -> bool {
    match term {
        TermPattern::BlankNode(_) => true,
        TermPattern::Triple(triple) => triple_pattern_contains_blank(triple),
        TermPattern::NamedNode(_) | TermPattern::Literal(_) | TermPattern::Variable(_) => false,
    }
}

fn pattern_reaches_non_reproducible_builtin(
    pattern: &GraphPattern,
) -> Option<NonReproducibleCause> {
    match pattern {
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => None,
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => pattern_reaches_non_reproducible_builtin(left)
            .or_else(|| pattern_reaches_non_reproducible_builtin(right)),
        GraphPattern::Graph { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Project { inner, .. } => pattern_reaches_non_reproducible_builtin(inner),
        GraphPattern::Filter { expr, inner } => expression_reaches_non_reproducible_builtin(expr)
            .or_else(|| pattern_reaches_non_reproducible_builtin(inner)),
        GraphPattern::Extend {
            inner, expression, ..
        } => expression_reaches_non_reproducible_builtin(expression)
            .or_else(|| pattern_reaches_non_reproducible_builtin(inner)),
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => pattern_reaches_non_reproducible_builtin(left)
            .or_else(|| pattern_reaches_non_reproducible_builtin(right))
            .or_else(|| {
                expression
                    .as_ref()
                    .and_then(expression_reaches_non_reproducible_builtin)
            }),
        GraphPattern::OrderBy { inner, expression } => {
            pattern_reaches_non_reproducible_builtin(inner).or_else(|| {
                expression.iter().find_map(|order| match order {
                    OrderExpression::Asc(expression) | OrderExpression::Desc(expression) => {
                        expression_reaches_non_reproducible_builtin(expression)
                    }
                })
            })
        }
        GraphPattern::Group {
            inner, aggregates, ..
        } => pattern_reaches_non_reproducible_builtin(inner).or_else(|| {
            aggregates.iter().find_map(|(_, aggregate)| {
                // A `Custom` aggregate invokes a HOST-supplied reduction, and whether
                // that reduction is reproducible is a property of the registry the
                // *evaluation* carries — the exact unknowable-at-shape-validation-time
                // situation the `PropertyFunction` arm below documents. Unlike a
                // property-function call, `AGG(<iri>, …)` needs no `ParserOptions` seam
                // to parse (`parser.rs`'s `parse_agg_call` recognizes it unconditionally),
                // so this check cannot lean on a caller-configured gate the way the
                // `PropertyFunction` arm's doc explains its own dead-under-default case —
                // a `Custom` aggregate is reachable under `ParserOptions::default`, always.
                if matches!(aggregate.function, AggregateFunction::Custom(_)) {
                    Some(NonReproducibleCause::CustomAggregate)
                } else {
                    aggregate
                        .args()
                        .iter()
                        .find_map(expression_reaches_non_reproducible_builtin)
                }
            })
        }),
        // A property-function call invokes a HOST-supplied relation, and whether that
        // relation is reproducible is a property of the registry the *evaluation*
        // carries — which is not available here, at shape-validation time, and cannot
        // be inferred from the IRI. A dataset description must be byte-reproducible, so
        // the unknowable case is refused conservatively.
        //
        // Reaching this arm at all requires the caller to have recognized the
        // predicate as a call in the first place: `parse_construct` parses under
        // caller-supplied `ParserOptions` ([`ConstructViewConfig::new_with_parser_options`]),
        // and only a predicate matching that configuration's `property_fn_namespaces` /
        // `property_fn_iris` produces `GraphPattern::PropertyFunction` rather than an
        // ordinary triple pattern. Under [`ParserOptions::default`] (what
        // [`ConstructViewConfig::new`] uses) this arm never fires.
        GraphPattern::PropertyFunction(_) => Some(NonReproducibleCause::PropertyFunction),
        // `SERVICE` forwards `inner` to a remote endpoint resolved through a
        // `RemoteQuerySource` the *evaluation* carries (`purrdf-sparql-eval`'s
        // `EvalCtx::remote`) — not visible here, at shape-validation time, and not
        // inferrable from the endpoint IRI (even a fixed, non-variable endpoint names a
        // live external service whose data can change between calls). The identical
        // "unknowable registry" reasoning the `PropertyFunction` arm states applies
        // one-for-one, so `SERVICE` gets the same unconditional refusal regardless of
        // `inner`'s own content or of `silent` — a swallowed remote failure is still a
        // remote dependency, just one that fails quietly instead of loudly.
        GraphPattern::Service { .. } => Some(NonReproducibleCause::Service),
    }
}

fn expression_reaches_non_reproducible_builtin(
    expression: &Expression,
) -> Option<NonReproducibleCause> {
    match expression {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => None,
        Expression::Or(left, right)
        | Expression::And(left, right)
        | Expression::Equal(left, right)
        | Expression::SameTerm(left, right)
        | Expression::Greater(left, right)
        | Expression::GreaterOrEqual(left, right)
        | Expression::Less(left, right)
        | Expression::LessOrEqual(left, right)
        | Expression::Add(left, right)
        | Expression::Subtract(left, right)
        | Expression::Multiply(left, right)
        | Expression::Divide(left, right) => expression_reaches_non_reproducible_builtin(left)
            .or_else(|| expression_reaches_non_reproducible_builtin(right)),
        Expression::UnaryPlus(inner) | Expression::UnaryMinus(inner) | Expression::Not(inner) => {
            expression_reaches_non_reproducible_builtin(inner)
        }
        Expression::In(head, list) => {
            expression_reaches_non_reproducible_builtin(head).or_else(|| {
                list.iter()
                    .find_map(expression_reaches_non_reproducible_builtin)
            })
        }
        Expression::If(condition, then_value, else_value) => {
            expression_reaches_non_reproducible_builtin(condition)
                .or_else(|| expression_reaches_non_reproducible_builtin(then_value))
                .or_else(|| expression_reaches_non_reproducible_builtin(else_value))
        }
        Expression::Coalesce(list) => list
            .iter()
            .find_map(expression_reaches_non_reproducible_builtin),
        Expression::FunctionCall(function, arguments) => match function {
            Function::Now => Some(NonReproducibleCause::Now),
            Function::Rand => Some(NonReproducibleCause::Rand),
            Function::Uuid => Some(NonReproducibleCause::Uuid),
            Function::StrUuid => Some(NonReproducibleCause::StrUuid),
            Function::BNode => Some(NonReproducibleCause::BNode),
            Function::Purrdf(call)
                if matches!(call.fn_kind, PurrdfFn::ListSlice | PurrdfFn::ListConcat) =>
            {
                Some(NonReproducibleCause::ListFunction(call.fn_kind))
            }
            // A `Custom` scalar call invokes a HOST-supplied function, and whether
            // that function is reproducible is a property of the registry the
            // *evaluation* carries — the same unknowable-at-shape-validation-time
            // situation `pattern_reaches_non_reproducible_builtin`'s `PropertyFunction`
            // arm documents. `<iri>(args…)` needs no `ParserOptions` seam either: any
            // call-position IRI outside every *configured* extension-function
            // namespace falls through to `Function::Custom` unconditionally (see
            // `parser.rs`'s `parse_iri_or_function`), so this is reachable under
            // `ParserOptions::default`, always.
            Function::Custom(_) => Some(NonReproducibleCause::CustomFunction),
            _ => arguments
                .iter()
                .find_map(expression_reaches_non_reproducible_builtin),
        },
        Expression::Exists(pattern) => pattern_reaches_non_reproducible_builtin(pattern),
    }
}

fn validate_portable_bound(value: usize, field: &str) -> Result<(), ProjectionError> {
    if value == 0 {
        return Err(ProjectionError::configuration(format!(
            "{field} must be greater than zero"
        )));
    }
    if u32::try_from(value).is_err() {
        return Err(ProjectionError::configuration(format!(
            "{field} exceeds the portable u32 ceiling"
        )));
    }
    Ok(())
}

fn view_record_count<D: DatasetView>(view: &D, label: &str) -> Result<usize, ProjectionError> {
    view.quads()
        .count()
        .checked_add(view.named_graphs().count())
        .and_then(|count| count.checked_add(view.reifier_quads().count()))
        .and_then(|count| count.checked_add(view.annotation_quads().count()))
        .ok_or_else(|| ProjectionError::limit(format!("{label} record count overflow")))
}

fn ensure_blank_free<D: DatasetView>(
    view: &D,
    limits: ProjectionLimits,
) -> Result<(), ProjectionError> {
    for quad in view.quads() {
        for id in [quad.s, quad.p, quad.o] {
            reject_blank_term(view, id, limits)?;
        }
        if let Some(graph) = quad.g {
            reject_blank_term(view, graph, limits)?;
        }
    }
    for row in view.reifier_quads() {
        for id in [row.s, row.p, row.o] {
            reject_blank_term(view, id, limits)?;
        }
        if let Some(graph) = row.g {
            reject_blank_term(view, graph, limits)?;
        }
    }
    for row in view.annotation_quads() {
        for id in [row.s, row.p, row.o] {
            reject_blank_term(view, id, limits)?;
        }
        if let Some(graph) = row.g {
            reject_blank_term(view, graph, limits)?;
        }
    }
    for graph in view.named_graphs() {
        reject_blank_term(view, graph, limits)?;
    }
    Ok(())
}

fn reject_blank_term<D: DatasetView>(
    view: &D,
    id: D::Id,
    limits: ProjectionLimits,
) -> Result<(), ProjectionError> {
    if term_contains_blank(view, id, limits, 0)? {
        return Err(ProjectionError::integrity(
            "RDF dataset descriptions must not contain blank nodes",
        ));
    }
    Ok(())
}

fn term_contains_blank<D: DatasetView>(
    view: &D,
    id: D::Id,
    limits: ProjectionLimits,
    depth: usize,
) -> Result<bool, ProjectionError> {
    match view.resolve(id) {
        TermRef::Blank { .. } => Ok(true),
        TermRef::Triple { s, p, o } => {
            if depth > limits.max_term_depth() {
                return Err(ProjectionError::limit(format!(
                    "RDF triple term exceeds the configured depth limit of {}",
                    limits.max_term_depth()
                )));
            }
            for component in [s, p, o] {
                if term_contains_blank(view, component, limits, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        TermRef::Iri(_) | TermRef::Literal { .. } => Ok(false),
    }
}

/// One frozen RDF description graph and its deterministic packaged serialization.
#[derive(Debug, Clone)]
pub struct RdfDescriptionProjection {
    /// Frozen caller-vocabulary RDF 1.2 description graph.
    pub dataset: Arc<RdfDataset>,
    /// Canonical package member path derived from the selected syntax registry row.
    pub artifact_path: String,
    /// One-member deterministic projection package.
    pub package: ProjectionPackage,
    /// Always-computed semantic projection loss ledger.
    pub loss_ledger: LossLedger,
}

/// Package an already-materialized RDF 1.2 description graph in any registered syntax.
///
/// # Errors
///
/// Returns a typed configuration, integrity, codec, package, or resource-limit error
/// when the artifact stem is unsafe, the dataset contains named graphs, the selected
/// syntax would lower RDF 1.2 content, serialization fails, or package bounds are
/// exceeded.
pub fn serialize_rdf_description(
    dataset: Arc<RdfDataset>,
    format: NativeRdfFormat,
    artifact_stem: &str,
    limits: ProjectionLimits,
) -> Result<RdfDescriptionProjection, ProjectionError> {
    serialize_description(dataset, LossLedger::new(), format, artifact_stem, limits)
}

/// Serialize a default-graph RDF description without lowering any RDF 1.2 content.
///
/// The description engines deliberately emit one default graph, so every registered
/// syntax carries the same graph. Syntaxes unable to carry a produced RDF 1.2
/// statement row or directional literal fail instead of silently lowering it.
pub(crate) fn serialize_description(
    dataset: Arc<RdfDataset>,
    loss_ledger: LossLedger,
    format: NativeRdfFormat,
    artifact_stem: &str,
    limits: ProjectionLimits,
) -> Result<RdfDescriptionProjection, ProjectionError> {
    validate_artifact_stem(artifact_stem)?;
    ensure_blank_free(dataset.as_ref(), limits)?;
    if dataset.named_graphs().next().is_some() {
        return Err(ProjectionError::integrity(
            "an RDF dataset description must contain only the default graph",
        ));
    }
    let serialized =
        serialize_dataset_to_format(dataset.as_ref(), format, None).map_err(|error| {
            ProjectionError::integrity(format!(
                "native {} serialization of RDF dataset description failed: {error}",
                format.id()
            ))
        })?;
    if serialized.statement_rows_dropped != 0 || serialized.directional_literals_dropped != 0 {
        return Err(ProjectionError::integrity(format!(
            "native {} serialization would drop {} RDF 1.2 statement rows and {} directional literals",
            format.id(),
            serialized.statement_rows_dropped,
            serialized.directional_literals_dropped
        )));
    }
    let artifact_path = format!("{artifact_stem}.{}", format.file_extension());
    let package =
        ProjectionPackage::from_artifacts(limits, [(artifact_path.clone(), serialized.bytes)])?;
    Ok(RdfDescriptionProjection {
        dataset,
        artifact_path,
        package,
        loss_ledger,
    })
}

fn validate_artifact_stem(value: &str) -> Result<(), ProjectionError> {
    let mut chars = value.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_alphabetic())
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(ProjectionError::configuration(
            "RDF description artifact stem must start with an ASCII letter and contain only ASCII alphanumerics, `-`, or `_`",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use purrdf_core::{
        BlankScope, PackBuilder, PackView, RdfDatasetBuilder, RdfLiteral, RdfTextDirection,
        datasets_isomorphic,
    };

    use super::*;

    fn limits() -> ProjectionLimits {
        ProjectionLimits::new(1, 1_000_000, 1_000_000, 1_002_000, 16).expect("limits")
    }

    fn default_graph() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/subject");
        let predicate = builder.intern_iri("https://example.org/predicate");
        let object = builder.intern_iri("https://example.org/object");
        builder.push_quad(subject, predicate, object, None);
        builder.freeze().expect("dataset")
    }

    fn construct_source(reverse: bool) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let graph = builder.intern_iri("https://example.org/source-graph");
        let predicate = builder.intern_iri("https://example.org/source-predicate");
        let rows = if reverse { [2, 1] } else { [1, 2] };
        for row in rows {
            let subject = builder.intern_iri(&format!("https://example.org/subject-{row}"));
            let object = builder.intern_iri(&format!("https://example.org/object-{row}"));
            builder.push_quad(subject, predicate, object, Some(graph));
        }
        builder.freeze().expect("CONSTRUCT source")
    }

    fn construct_config(
        max_input_records: usize,
        max_output_records: usize,
    ) -> ConstructViewConfig {
        ConstructViewConfig::new(
            "CONSTRUCT { ?s <https://example.org/copied> ?o } WHERE { GRAPH <https://example.org/source-graph> { ?s <https://example.org/source-predicate> ?o } }",
            None,
            limits(),
            1_000,
            max_input_records,
            max_output_records,
        )
        .expect("CONSTRUCT config")
    }

    #[test]
    fn every_registered_syntax_gets_one_stable_artifact() {
        for format in NativeRdfFormat::all() {
            let first = serialize_description(
                default_graph(),
                LossLedger::new(),
                format,
                "description",
                limits(),
            )
            .expect("serialize description");
            let second = serialize_description(
                default_graph(),
                LossLedger::new(),
                format,
                "description",
                limits(),
            )
            .expect("serialize description again");
            assert_eq!(
                first.artifact_path,
                format!("description.{}", format.file_extension())
            );
            assert_eq!(first.package.len(), 1);
            assert_eq!(
                first.package.to_ustar().expect("first archive"),
                second.package.to_ustar().expect("second archive")
            );
        }
    }

    #[test]
    fn named_graphs_and_unsafe_stems_fail_closed() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/subject");
        let predicate = builder.intern_iri("https://example.org/predicate");
        let object = builder.intern_iri("https://example.org/object");
        let graph = builder.intern_iri("https://example.org/graph");
        builder.push_quad(subject, predicate, object, Some(graph));
        let dataset = builder.freeze().expect("dataset");
        assert!(
            serialize_description(
                dataset,
                LossLedger::new(),
                NativeRdfFormat::TriG,
                "description",
                limits(),
            )
            .is_err()
        );
        assert!(
            serialize_description(
                default_graph(),
                LossLedger::new(),
                NativeRdfFormat::Turtle,
                "../unsafe",
                limits(),
            )
            .is_err()
        );
    }

    #[test]
    fn serializers_reject_rdf12_content_the_selected_syntax_would_drop() {
        let mut directional = RdfDatasetBuilder::new();
        let subject = directional.intern_iri("https://example.org/subject");
        let predicate = directional.intern_iri("https://example.org/label");
        let object = directional.intern_literal(RdfLiteral {
            lexical_form: "مرحبا".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        directional.push_quad(subject, predicate, object, None);
        let directional = directional.freeze().expect("directional dataset");
        assert!(
            serialize_rdf_description(
                Arc::clone(&directional),
                NativeRdfFormat::TriX,
                "directional",
                limits(),
            )
            .is_err()
        );
        assert!(
            serialize_rdf_description(
                directional,
                NativeRdfFormat::Turtle,
                "directional",
                limits(),
            )
            .is_ok()
        );

        let mut statement = RdfDatasetBuilder::new();
        let subject = statement.intern_iri("https://example.org/subject");
        let predicate = statement.intern_iri("https://example.org/predicate");
        let object = statement.intern_iri("https://example.org/object");
        let reifier = statement.intern_iri("https://example.org/assertion");
        let triple = statement.intern_triple(subject, predicate, object);
        statement.push_quad(subject, predicate, object, None);
        statement.push_reifier(reifier, triple);
        let statement = statement.freeze().expect("statement-layer dataset");
        assert!(
            serialize_rdf_description(
                Arc::clone(&statement),
                NativeRdfFormat::RdfXml,
                "statement",
                limits(),
            )
            .is_err()
        );
        assert!(
            serialize_rdf_description(statement, NativeRdfFormat::Turtle, "statement", limits(),)
                .is_ok()
        );
    }

    #[test]
    fn allocation_free_blank_scan_preserves_the_term_depth_bound() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/subject");
        let predicate = builder.intern_iri("https://example.org/predicate");
        let object = builder.intern_iri("https://example.org/object");
        let inner = builder.intern_triple(subject, predicate, object);
        let middle = builder.intern_triple(subject, predicate, inner);
        let outer = builder.intern_triple(subject, predicate, middle);
        builder.push_quad(subject, predicate, outer, None);
        let dataset = builder.freeze().expect("deep triple-term dataset");
        let shallow_limits =
            ProjectionLimits::new(1, 1_000_000, 1_000_000, 1_002_000, 1).expect("limits");
        let error = serialize_rdf_description(
            dataset,
            NativeRdfFormat::Turtle,
            "description",
            shallow_limits,
        )
        .expect_err("term-depth limit");
        assert_eq!(
            error.kind(),
            super::super::ProjectionErrorKind::ResourceLimit
        );
    }

    #[test]
    fn construct_reads_named_graphs_and_is_backend_and_order_independent() {
        let source = construct_source(false);
        let reordered = construct_source(true);
        let config = construct_config(3, 2);
        let resident = project_construct_view(source.as_ref(), &config).expect("resident");
        let reversed = project_construct_view(reordered.as_ref(), &config).expect("reordered");
        assert_eq!(resident.input_records, 3);
        assert_eq!(resident.output_records, 2);
        assert!(datasets_isomorphic(&resident.dataset, &reversed.dataset));

        let bytes = PackBuilder::build_bytes(&source).expect("pack");
        let view = PackView::from_bytes(&bytes).expect("pack view");
        let packed = project_construct_view(&view, &config).expect("packed");
        assert!(datasets_isomorphic(&resident.dataset, &packed.dataset));
        assert_eq!(resident.input_records, packed.input_records);
        assert_eq!(resident.output_records, packed.output_records);
    }

    #[test]
    fn construct_honours_from_dataset_clauses() {
        let config = ConstructViewConfig::new(
            "CONSTRUCT { ?s <https://example.org/copied> ?o } FROM <https://example.org/source-graph> WHERE { ?s <https://example.org/source-predicate> ?o }",
            None,
            limits(),
            1_000,
            3,
            2,
        )
        .expect("CONSTRUCT config with FROM");
        let projected = project_construct_view(construct_source(false).as_ref(), &config)
            .expect("CONSTRUCT FROM projection");
        assert_eq!(projected.input_records, 3);
        assert_eq!(projected.output_records, 2);
    }

    #[test]
    fn construct_config_is_strict_and_requires_an_explicit_nullable_base() {
        assert!(ConstructViewConfig::new("ASK {}", None, limits(), 100, 1, 1).is_err());
        assert!(ConstructViewConfig::new("CONSTRUCT WHERE", None, limits(), 100, 1, 1).is_err());
        assert!(
            ConstructViewConfig::new(
                "CONSTRUCT { <item> <https://example.org/p> <https://example.org/o> } WHERE {}",
                None,
                limits(),
                200,
                1,
                1,
            )
            .is_err()
        );
        let with_base = ConstructViewConfig::new(
            "CONSTRUCT { <item> <https://example.org/p> <https://example.org/o> } WHERE {}",
            Some("https://example.org/base/".to_owned()),
            limits(),
            200,
            1,
            1,
        )
        .expect("relative IRI with base");
        let json = serde_json::to_value(&with_base).expect("serialize config");
        assert_eq!(json["base_iri"], "https://example.org/base/");
        assert!(serde_json::from_value::<ConstructViewConfig>(json).is_ok());

        let missing_base = serde_json::json!({
            "query": "CONSTRUCT {} WHERE {}",
            "limits": limits(),
            "max_query_bytes": 100,
            "max_input_records": 1,
            "max_output_records": 1
        });
        assert!(serde_json::from_value::<ConstructViewConfig>(missing_base).is_err());
    }

    #[test]
    fn construct_enforces_input_output_and_query_boundaries() {
        let source = construct_source(false);
        let input_error = project_construct_view(source.as_ref(), &construct_config(2, 2))
            .expect_err("input limit");
        assert_eq!(
            input_error.kind(),
            super::super::ProjectionErrorKind::ResourceLimit
        );
        let output_error = project_construct_view(source.as_ref(), &construct_config(3, 1))
            .expect_err("output limit");
        assert_eq!(
            output_error.kind(),
            super::super::ProjectionErrorKind::ResourceLimit
        );
        assert!(
            ConstructViewConfig::new("CONSTRUCT {} WHERE {}", None, limits(), 10, 1, 1,).is_err()
        );
    }

    #[test]
    fn construct_rejects_non_reproducible_and_blank_minting_queries() {
        // Each builtin's own name must appear in the message the production parse
        // path produces — not a restatement of the whole non-reproducible-builtin
        // list regardless of which one actually fired (F2: that used to be the ONE
        // message every refusal here returned).
        for (query, own_name) in [
            (
                "CONSTRUCT { <https://example.org/s> <https://example.org/p> ?value } WHERE { BIND(NOW() AS ?value) }",
                "NOW",
            ),
            (
                "CONSTRUCT { <https://example.org/s> <https://example.org/p> ?value } WHERE { BIND(RAND() AS ?value) }",
                "RAND",
            ),
            (
                "CONSTRUCT { <https://example.org/s> <https://example.org/p> ?value } WHERE { BIND(UUID() AS ?value) }",
                "UUID",
            ),
            (
                "CONSTRUCT { <https://example.org/s> <https://example.org/p> ?value } WHERE { BIND(STRUUID() AS ?value) }",
                "STRUUID",
            ),
            (
                "CONSTRUCT { <https://example.org/s> <https://example.org/p> ?value } WHERE { BIND(BNODE() AS ?value) }",
                "BNODE",
            ),
        ] {
            let error = ConstructViewConfig::new(query, None, limits(), 1_000, 10, 10)
                .expect_err(&format!("query must be rejected: {query}"));
            assert!(
                error.message().contains(own_name),
                "message must name its own construct {own_name:?}, got {:?}",
                error.message()
            );
        }

        // Blank-minting templates are rejected under a separate message entirely
        // (they never reach the non-reproducible-builtin walk at all).
        let blank_error = ConstructViewConfig::new(
            "CONSTRUCT { _:generated <https://example.org/p> <https://example.org/o> } WHERE {}",
            None,
            limits(),
            1_000,
            10,
            10,
        )
        .expect_err("blank-minting template must be rejected");
        assert!(blank_error.message().contains("blank"));
    }

    #[test]
    fn list_functions_are_refused_and_name_their_own_construct() {
        // `purrdf:listSlice`/`purrdf:listConcat` mint fresh blank nodes (a fresh
        // `rdf:List`), so both are refused like the other blank-minting builtins —
        // each naming its OWN local name, not the other's.
        for (call, own_name) in [
            ("purrdf:listSlice(?list, 0, 1)", "listSlice"),
            ("purrdf:listConcat(?list, ?list)", "listConcat"),
        ] {
            let query = format!(
                "PREFIX purrdf: <https://purrdf.example/fn/> \
                 CONSTRUCT {{ <https://example.org/s> <https://example.org/p> ?value }} WHERE {{ \
                 ?s <https://example.org/source-predicate> ?list . BIND({call} AS ?value) }}"
            );
            let options = ParserOptions {
                extension_fn_namespaces: vec!["https://purrdf.example/fn/".to_owned()],
                ..ParserOptions::default()
            };
            let error = ConstructViewConfig::new_with_parser_options(
                &query,
                None,
                limits(),
                1_000,
                10,
                10,
                &options,
            )
            .expect_err(&format!("{call} must be rejected"));
            assert_eq!(
                error.kind(),
                super::super::ProjectionErrorKind::Configuration
            );
            assert!(
                error.message().contains(own_name),
                "message must name its own construct {own_name:?}, got {:?}",
                error.message()
            );
        }
    }

    #[test]
    fn property_function_predicate_is_refused_only_when_parser_options_recognize_it() {
        // GAP-9 coverage: `parse_construct` used to parse with `ParserOptions::default`
        // unconditionally, so `GraphPattern::PropertyFunction` could never be produced
        // and the `pattern_reaches_non_reproducible_builtin` arm documenting the
        // conservative refusal was dead code. This pins both halves.
        let call_query = "CONSTRUCT { ?s <https://example.org/copied> ?o } \
             WHERE { ?s <https://example.org/pf/related> ?o }";

        // Default options (`ConstructViewConfig::new`): the seam is off, so the
        // predicate is an ordinary triple pattern and the query is accepted.
        assert!(
            ConstructViewConfig::new(call_query, None, limits(), 1_000, 10, 10).is_ok(),
            "under the default seam the predicate must parse as ordinary data"
        );

        // Configured options recognizing the SAME predicate IRI as a
        // property-function relation: the call is refused as unknowably
        // reproducible, per the documented conservative semantics.
        let options = ParserOptions {
            property_fn_iris: vec!["https://example.org/pf/related".to_owned()],
            ..ParserOptions::default()
        };
        let error = ConstructViewConfig::new_with_parser_options(
            call_query,
            None,
            limits(),
            1_000,
            10,
            10,
            &options,
        )
        .expect_err("a recognized property-function call must be refused");
        assert_eq!(
            error.kind(),
            super::super::ProjectionErrorKind::Configuration
        );
        assert!(
            error.message().contains("property-function"),
            "message must name the property-function construct, got {:?}",
            error.message()
        );

        // An ordinary predicate that the configured options do NOT recognize is
        // still accepted even with the seam configured for a different IRI.
        let ordinary_query = "CONSTRUCT { ?s <https://example.org/copied> ?o } \
             WHERE { ?s <https://example.org/pf/unrelated> ?o }";
        assert!(
            ConstructViewConfig::new_with_parser_options(
                ordinary_query,
                None,
                limits(),
                1_000,
                10,
                10,
                &options,
            )
            .is_ok()
        );
    }

    #[test]
    fn custom_aggregate_call_is_refused_but_builtin_aggregates_stay_reproducible() {
        // A `Custom` aggregate (the `AGG(<iri>, …)` surface) invokes host-supplied
        // reduction code reachable through the aggregate registry the *evaluation*
        // carries — unlike a property-function call, this needs no `ParserOptions`
        // seam at all (`parser.rs`'s `parse_agg_call` recognizes `AGG` unconditionally),
        // so it must be refused even under `ParserOptions::default`.
        let custom_query = "CONSTRUCT { ?s <https://example.org/copied> ?agg } WHERE { \
             SELECT ?s (AGG(<https://example.org/agg/custom>, ?o) AS ?agg) WHERE { \
             ?s <https://example.org/source-predicate> ?o } GROUP BY ?s }";
        let error = ConstructViewConfig::new(custom_query, None, limits(), 1_000, 10, 10)
            .expect_err("a custom aggregate call must be refused");
        assert_eq!(
            error.kind(),
            super::super::ProjectionErrorKind::Configuration
        );
        assert!(
            error.message().contains("custom aggregate"),
            "message must name the custom-aggregate construct, not the historical \
             NOW/RAND/UUID/STRUUID/BNODE/list-function text; got {:?}",
            error.message()
        );

        // Built-in aggregates carry no host dependency and must remain accepted —
        // the fix above must not over-correct into refusing every aggregate.
        for builtin in [
            "COUNT(?o)",
            "SUM(?o)",
            "AVG(?o)",
            "MIN(?o)",
            "MAX(?o)",
            "SAMPLE(?o)",
        ] {
            let builtin_query = format!(
                "CONSTRUCT {{ ?s <https://example.org/copied> ?agg }} WHERE {{ \
                 SELECT ?s ({builtin} AS ?agg) WHERE {{ \
                 ?s <https://example.org/source-predicate> ?o }} GROUP BY ?s }}"
            );
            assert!(
                ConstructViewConfig::new(&builtin_query, None, limits(), 1_000, 10, 10).is_ok(),
                "built-in aggregate must stay reproducible: {builtin}"
            );
        }
    }

    #[test]
    fn custom_scalar_function_call_is_refused_as_non_reproducible() {
        // A `Function::Custom` call (any call-position IRI outside every configured
        // extension-function namespace) invokes host-supplied code reachable through
        // the function registry the *evaluation* carries, exactly like a `Custom`
        // aggregate or a property function — and, like the aggregate surface, needs
        // no `ParserOptions` seam to parse (`parser.rs`'s `parse_iri_or_function`
        // falls through to `Function::Custom` unconditionally).
        let query = "CONSTRUCT { ?s <https://example.org/copied> ?value } WHERE { \
             ?s <https://example.org/source-predicate> ?o . \
             BIND(<https://example.org/fn/custom>(?o) AS ?value) }";
        let error = ConstructViewConfig::new(query, None, limits(), 1_000, 10, 10)
            .expect_err("a custom scalar-function call must be refused");
        assert_eq!(
            error.kind(),
            super::super::ProjectionErrorKind::Configuration
        );
        assert!(
            error.message().contains("custom scalar function"),
            "message must name the custom-scalar-function construct, not the historical \
             NOW/RAND/UUID/STRUUID/BNODE/list-function text; got {:?}",
            error.message()
        );

        // An ordinary built-in scalar function call stays accepted.
        let builtin_query = "CONSTRUCT { ?s <https://example.org/copied> ?value } WHERE { \
             ?s <https://example.org/source-predicate> ?o . \
             BIND(STR(?o) AS ?value) }";
        assert!(ConstructViewConfig::new(builtin_query, None, limits(), 1_000, 10, 10).is_ok());
    }

    #[test]
    fn service_clause_is_refused_as_non_reproducible() {
        // `SERVICE` forwards its inner pattern to a remote endpoint resolved through
        // a `RemoteQuerySource` the *evaluation* carries — not visible at this
        // shape-validation point, and not inferrable from even a fixed endpoint IRI
        // (a live external service's data can change between calls). It needs no
        // `ParserOptions` seam either: `SERVICE` is ordinary SPARQL 1.1 syntax.
        let query = "CONSTRUCT { ?s <https://example.org/copied> ?o } WHERE { \
             SERVICE <https://example.org/endpoint> { \
             ?s <https://example.org/source-predicate> ?o } }";
        let error = ConstructViewConfig::new(query, None, limits(), 1_000, 10, 10)
            .expect_err("a SERVICE clause must be refused");
        assert_eq!(
            error.kind(),
            super::super::ProjectionErrorKind::Configuration
        );
        assert!(
            error.message().contains("SERVICE"),
            "message must name the SERVICE construct, not the historical \
             NOW/RAND/UUID/STRUUID/BNODE/list-function text; got {:?}",
            error.message()
        );

        // `SERVICE SILENT` is refused identically — a swallowed remote failure is
        // still a remote dependency, just one that fails quietly instead of loudly.
        let silent_query = "CONSTRUCT { ?s <https://example.org/copied> ?o } WHERE { \
             SERVICE SILENT <https://example.org/endpoint> { \
             ?s <https://example.org/source-predicate> ?o } }";
        assert!(ConstructViewConfig::new(silent_query, None, limits(), 1_000, 10, 10).is_err());
    }

    #[test]
    fn construct_rejects_blank_nodes_copied_from_source_data() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_blank("source", BlankScope::DEFAULT);
        let predicate = builder.intern_iri("https://example.org/source-predicate");
        let object = builder.intern_iri("https://example.org/object");
        builder.push_quad(subject, predicate, object, None);
        let source = builder.freeze().expect("blank source");
        let config = ConstructViewConfig::new(
            "CONSTRUCT { ?s <https://example.org/copied> ?o } WHERE { ?s <https://example.org/source-predicate> ?o }",
            None,
            limits(),
            1_000,
            1,
            1,
        )
        .expect("config");
        assert!(project_construct_view(source.as_ref(), &config).is_err());
    }
}
