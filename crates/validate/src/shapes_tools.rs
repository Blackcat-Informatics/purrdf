// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The string-in / string-out boundary of the shapes-graph TOOLS the Python, WASM and
//! C-ABI hosts expose beside validation: running rules ([`apply_rules_to_ntriples`]),
//! checking a SPARQL 1.2 RL rule set without running it ([`check_rules`]), evaluating one
//! node expression ([`eval_node_expr_to_terms`]) and certifying a shapes graph
//! ([`lint_shapes_ttl`]).
//!
//! Each is one function here so the three bindings share one implementation, exactly as
//! [`crate::validate_to_sarif_string`] and [`crate::entail_to_ntriples_string`] are shared.
//! Shapes graphs cross the boundary as Turtle, data graphs as N-Triples (which admits no
//! relative IRI, so it needs no base), and SPARQL 1.2 RL rule sets as their own text.
//!
//! # The shapes graph's `owl:imports`
//!
//! A shapes graph is its `owl:imports` closure, on these three tools exactly as on
//! validation: each takes the host's [`ShapesImportList`] and resolves the closure
//! through the one engine helper every shapes-graph entry point shares
//! ([`purrdf_shapes::imports::resolve_shapes_imports`]). An imported document's rules
//! run, its node expressions and functions are in scope, and its shapes are certified; a
//! closure that is not in hand is refused with [`ShapesError::Imports`] — never a rules
//! run over fewer rules, and never a `clean` lint of a shapes graph validation refuses.
//!
//! # A SPARQL 1.2 RL rule set's `IMPORTS`
//!
//! The rules tool's one import table serves whichever rule source it runs. For a SPARQL
//! 1.2 RL rule set it is a table of `(import IRI, rule-set text)` pairs, resolved through
//! [`srl::RuleSetDocument::resolve_import_table`] — the route the command line takes too:
//! followed transitively, each IRI read once, an import no entry supplies refused by name,
//! and an entry the closure never names refused as unused.
//!
//! # The rule-evaluation limits
//!
//! [`RulesRequest::max_term_generating_rounds`] and [`RulesRequest::max_generated_terms`]
//! are the host's knobs over [`purrdf_shapes::RuleOptions::with_max_term_generating_rounds`]
//! and [`purrdf_shapes::RuleOptions::with_max_generated_terms`] (and their
//! [`purrdf_shapes::srl::InferOptions`] twins): at most that many evaluation rounds may
//! infer a term the evaluation graph did not hold, and at most that many terms may be
//! inferred beyond the input's. `None` keeps the engine default — 16,384 rounds, and
//! `max(65,536, 4 × N)` terms for `N` distinct input terms. A run past either is a
//! failure naming the limit, the numbers, the rules that inferred a new term last, and
//! the knob that raises it in the calling host's own terms ([`RulesRequest::host`]).
//!
//! [`RulesRequest::max_stored_facts`] and [`RulesRequest::max_join_steps`] are the knobs
//! over [`purrdf_shapes::RuleOptions::with_max_stored_facts`] and
//! [`purrdf_shapes::RuleOptions::with_max_join_steps`]: the facts the evaluation store may
//! hold — the data graph, a rule set's data and every inferred triple — and the candidate
//! solutions the rule bodies may enumerate. `None` keeps the target's default — 4,194,304
//! facts and 1,048,576 join steps natively, 131,072 and 1,048,576 on `wasm32`. A run
//! past either fails naming the limit, the numbers and the knob that raises it.

use purrdf_shapes::data::ShaclData;
use purrdf_shapes::free_expression::{self, FreeExpression};
use purrdf_shapes::lint::{self, LintReport};
use purrdf_shapes::srl::{self, InferOptions};
use purrdf_shapes::text_ingest::{parse_ntriples_to_dataset, parse_turtle_document};
use purrdf_shapes::{Inference, LimitKnobs, RuleOptions, ShapesError, ShapesImports, engine};

use crate::ShapesImportList;
use crate::expr_selector::ExprSelector;

/// One rules run across the host boundary: the data graph and exactly ONE rule source —
/// a SHACL shapes graph's rules, or a SPARQL 1.2 RL rule set.
#[derive(Debug, Clone, Copy, Default)]
pub struct RulesRequest<'a> {
    /// The data (base) graph, as N-Triples.
    pub data_nt: &'a str,
    /// A SHACL shapes graph, as Turtle, whose rules — its default rule set — are run.
    pub shapes_ttl: Option<&'a str>,
    /// The base IRI the shapes document's relative references resolve against.
    pub shapes_base: Option<&'a str>,
    /// The shapes-graph IRI the SHACL rules see the shapes graph under: a `sh:SPARQLRule`'s
    /// `$shapesGraph` is pre-bound to it and `GRAPH $shapesGraph { … }` reads the shapes
    /// graph, exactly as [`crate::validate_to_sarif_string_with_shapes_graph`] exposes it
    /// to validation. A relative one resolves against [`Self::shapes_base`]
    /// ([`engine::resolve_shapes_graph_iri`]); `None` leaves `$shapesGraph` an ordinary
    /// variable. A SPARQL 1.2 RL rule set has no shapes graph, so naming one beside
    /// [`Self::srl`] is refused. Python's `shapes_graph=`, WebAssembly's `shapesGraph` and
    /// C's `shapes_graph_iri` all reach here.
    pub shapes_graph: Option<&'a str>,
    /// The rule source's import table, `(IRI, document text)` pairs: the shapes graph's
    /// `owl:imports` table (Turtle documents), or the SPARQL 1.2 RL rule set's `IMPORTS`
    /// table (rule-set texts). Empty is the ordinary case, and still refuses a rule source
    /// that imports a document it does not hold.
    pub imports: &'a ShapesImportList<'a>,
    /// A SPARQL 1.2 RL rule set, as text.
    pub srl: Option<&'a str>,
    /// The base IRI the rule set's relative references resolve against.
    pub srl_base: Option<&'a str>,
    /// Whether to render the proof of every inferred triple
    /// ([`Inference::proof_text`]).
    pub explain: bool,
    /// The term-generating round limit, or `None` for the engine default. See the
    /// [module docs](self): hosts running untrusted rule sets should lower it.
    pub max_term_generating_rounds: Option<u64>,
    /// The generated-term budget, or `None` for the engine default. See the
    /// [module docs](self).
    pub max_generated_terms: Option<u64>,
    /// The stored-fact limit, or `None` for the target's default. See the
    /// [module docs](self).
    pub max_stored_facts: Option<u64>,
    /// The join-step limit, or `None` for the target's default. See the
    /// [module docs](self).
    pub max_join_steps: Option<u64>,
    /// The host calling, whose names for the limits' knobs a refusal gives.
    pub host: RulesHost,
}

/// The host a [`RulesRequest`] comes from: it decides how a refusal for a passed
/// rule-evaluation limit names the knob that raises it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RulesHost {
    /// A Rust caller of this boundary: [`RulesRequest`]'s fields.
    #[default]
    Rust,
    /// The Python `purrdf.shapes.apply_rules` keyword arguments.
    Python,
    /// The WebAssembly `shaclApplyRules` arguments.
    Wasm,
    /// The C ABI `purrdf_shacl_apply_rules` parameters.
    CAbi,
}

impl RulesHost {
    /// The host's names for the term-generating round limit, the generated-term budget,
    /// the stored-fact limit and the join-step limit.
    #[must_use]
    pub fn limit_knobs(self) -> LimitKnobs {
        match self {
            Self::Rust => LimitKnobs::new(
                "RulesRequest::max_term_generating_rounds",
                "RulesRequest::max_generated_terms",
                "RulesRequest::max_stored_facts",
                "RulesRequest::max_join_steps",
            ),
            Self::Python => LimitKnobs::new(
                "apply_rules(max_term_generating_rounds=...)",
                "apply_rules(max_generated_terms=...)",
                "apply_rules(max_stored_facts=...)",
                "apply_rules(max_join_steps=...)",
            ),
            Self::Wasm => LimitKnobs::new(
                "shaclApplyRules's maxTermGeneratingRounds",
                "shaclApplyRules's maxGeneratedTerms",
                "shaclApplyRules's maxStoredFacts",
                "shaclApplyRules's maxJoinSteps",
            ),
            Self::CAbi => LimitKnobs::new(
                "purrdf_shacl_apply_rules's max_term_generating_rounds",
                "purrdf_shacl_apply_rules's max_generated_terms",
                "purrdf_shacl_apply_rules's max_stored_facts",
                "purrdf_shacl_apply_rules's max_join_steps",
            ),
        }
    }
}

/// What a rules run produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulesOutcome {
    /// The inference graph — the inferred triples only, never the base graph — as
    /// N-Triples 1.2 in canonical order ([`Inference::inferred_ntriples`]).
    pub inferred_ntriples: String,
    /// The proof text ([`Inference::proof_text`]) when the request asked for it.
    pub proof: Option<String>,
}

/// Run a rule set over a data graph and return the inference graph.
///
/// The SHACL route is [`purrdf_shapes::infer`] over the shapes graph's default rule set,
/// with no `sh:ruleProcessor` registered — a rule or rule set naming one is a failure,
/// as SHACL 1.2 Inference Rules requires of a processor the engine cannot handle. The
/// SPARQL 1.2 RL route is [`check_rules`] at [`srl::CheckLevel::Stratified`] over
/// [`RulesRequest::imports`] — the check-only entry point, so a rules run refuses exactly
/// what a check refuses — then [`srl::infer`].
///
/// # Errors
///
/// [`ShapesError::Imports`] when the shapes graph's `owl:imports` closure is not in hand
/// or its import table cannot be used. Otherwise [`ShapesError::Invalid`]: neither or
/// both rule sources named; a shapes-graph IRI that names no graph, or one named beside a
/// SPARQL 1.2 RL rule set; a document that does not parse; a rule set that is
/// ill-formed, unstratifiable or fails during execution; a rule-evaluation limit
/// passed; a SPARQL 1.2 RL `IMPORTS` the import table does not supply, or a table
/// entry its import closure never names.
pub fn apply_rules_to_ntriples(request: &RulesRequest<'_>) -> Result<RulesOutcome, ShapesError> {
    let data = parse_ntriples_to_dataset(request.data_nt).map_err(|errors| errors.join("\n"))?;
    let inference: Inference = match (request.shapes_ttl, request.srl) {
        (Some(shapes_ttl), None) => {
            let shapes = engine::parse_shapes_with_graph(
                shapes_ttl,
                request.shapes_base,
                None,
                request.shapes_graph,
                &ShapesImports::from_turtle(request.imports)?,
            )?;
            let projected = engine::project_dataset(data.as_ref())?;
            let holder = ShaclData::new(std::sync::Arc::clone(&projected), projected, None);
            let mut options = RuleOptions::default().with_limit_knobs(request.host.limit_knobs());
            if let Some(rounds) = request.max_term_generating_rounds {
                options = options.with_max_term_generating_rounds(rounds);
            }
            if let Some(terms) = request.max_generated_terms {
                options = options.with_max_generated_terms(terms);
            }
            if let Some(facts) = request.max_stored_facts {
                options = options.with_max_stored_facts(facts);
            }
            if let Some(steps) = request.max_join_steps {
                options = options.with_max_join_steps(steps);
            }
            purrdf_shapes::infer(&holder, &shapes, &options)?
        }
        (None, Some(_)) if request.shapes_graph.is_some() => {
            return Err(ShapesError::Invalid(
                "a shapes-graph IRI names the graph a SHACL shapes graph's rules see the shapes \
                 graph under, and a SPARQL 1.2 RL rule set has no shapes graph; drop it, or run \
                 a SHACL shapes graph"
                    .to_owned(),
            ));
        }
        (None, Some(text)) => {
            let document = check_rules(
                text,
                request.srl_base,
                request.imports,
                srl::CheckLevel::Stratified,
            )?
            .into_document();
            let mut options = InferOptions::default().with_limit_knobs(request.host.limit_knobs());
            if let Some(rounds) = request.max_term_generating_rounds {
                options = options.with_max_term_generating_rounds(rounds);
            }
            if let Some(terms) = request.max_generated_terms {
                options = options.with_max_generated_terms(terms);
            }
            if let Some(facts) = request.max_stored_facts {
                options = options.with_max_stored_facts(facts);
            }
            if let Some(steps) = request.max_join_steps {
                options = options.with_max_join_steps(steps);
            }
            srl::infer(&document, data.as_ref(), &options).map_err(|e| e.to_string())?
        }
        (None, None) => {
            return Err(ShapesError::Invalid(
                "no rule source: name a SHACL shapes graph or a SPARQL 1.2 RL rule set".to_owned(),
            ));
        }
        (Some(_), Some(_)) => {
            return Err(ShapesError::Invalid(
                "two rule sources: a SHACL shapes graph and a SPARQL 1.2 RL rule set are two \
                 rule sets, and neither specification defines running them as one; name one"
                    .to_owned(),
            ));
        }
    };
    Ok(RulesOutcome {
        inferred_ntriples: inference.inferred_ntriples(),
        proof: request.explain.then(|| inference.proof_text()),
    })
}

/// Check a SPARQL 1.2 RL rule set without evaluating it: [`srl::check`] up to `level`
/// — the §7 grammar, the `IMPORTS` closure resolved from `imports` (the rules tool's
/// import table, `(import IRI, rule-set text)` pairs), §4.2 well-formedness and §4.4
/// stratification — with no base graph read and no rule run. The Python `check_rules`,
/// WebAssembly `shaclCheckRules` and C `purrdf_shacl_check_rules` entry points, and every
/// host's SPARQL 1.2 RL rules run ([`apply_rules_to_ntriples`]), pass through here.
///
/// # Errors
///
/// [`ShapesError::Invalid`] carrying the [`srl::SrlError`] text, which names the stage that
/// refused: a syntax error, an import the table does not supply or an entry it never
/// reaches, an ill-formed rule, a rule set that cannot be stratified.
pub fn check_rules(
    srl_text: &str,
    srl_base: Option<&str>,
    imports: &ShapesImportList<'_>,
    level: srl::CheckLevel,
) -> Result<srl::CheckedRuleSet, ShapesError> {
    srl::check(srl_text, srl_base, imports, level)
        .map_err(|error| ShapesError::Invalid(error.to_string()))
}

/// The [`srl::CheckLevel`] a host spelled by name — `syntax`, `well-formed` or
/// `stratified` — or [`srl::CheckLevel::Stratified`] when it named none.
///
/// # Errors
///
/// [`ShapesError::Invalid`] naming the three levels, for any other name: a level the host
/// mistyped is refused rather than read as the default.
pub fn parse_check_level(name: Option<&str>) -> Result<srl::CheckLevel, ShapesError> {
    let Some(name) = name else {
        return Ok(srl::CheckLevel::default());
    };
    srl::CheckLevel::from_name(name).ok_or_else(|| {
        ShapesError::Invalid(format!(
            "`{name}` is not a SPARQL 1.2 RL check level; name one of {}",
            srl::CheckLevel::ALL
                .iter()
                .map(|level| format!("`{level}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })
}

/// One node-expression evaluation across the host boundary.
#[derive(Debug, Clone, Copy)]
pub struct NodeExprRequest<'a> {
    /// The shapes graph carrying the expression, as Turtle.
    pub shapes_ttl: &'a str,
    /// The base IRI the shapes document's relative references resolve against.
    pub shapes_base: Option<&'a str>,
    /// The focus graph, as N-Triples.
    pub data_nt: &'a str,
    /// Which node expression to evaluate: the node itself, the node a walk of
    /// predicates reaches from a named node, or an inline Turtle expression (see
    /// [`ExprSelector`]).
    pub expr: ExprSelector<'a>,
    /// The focus node, as an absolute IRI or an N-Triples term.
    pub focus: &'a str,
    /// The scope's variable bindings, `(name, term)`, the term spelled as `focus` is.
    pub scope: &'a [(&'a str, &'a str)],
    /// The shapes graph's `owl:imports` table: an imported document's functions and
    /// shapes are in scope for the expression.
    pub imports: &'a ShapesImportList<'a>,
}

/// Evaluate one node expression and return its output nodes as N-Triples 1.2 terms, in
/// the order the expression's sequence semantics define. See
/// [`free_expression::evaluate`], and [`ExprSelector`] for how the expression is named.
///
/// # Errors
///
/// [`ShapesError::Imports`] when the shapes graph's `owl:imports` closure is not in hand
/// or its import table cannot be used; [`ShapesError::Invalid`] for a document that does
/// not parse, a term that is not one, a selector that names no single expression (the
/// [`crate::ExprSelectorError`] text: a walk step reaching no value or several, an inline
/// expression without exactly one root), and anything else [`free_expression::evaluate`]
/// refuses.
pub fn eval_node_expr_to_terms(request: &NodeExprRequest<'_>) -> Result<Vec<String>, ShapesError> {
    let imports = ShapesImports::from_turtle(request.imports)?;
    let shapes = parse_turtle_document(request.shapes_ttl, request.shapes_base)
        .map_err(|errors| errors.join("\n"))?;
    let imports = read_under(imports, request.shapes_base, shapes.base.as_deref());
    let data = parse_ntriples_to_dataset(request.data_nt).map_err(|errors| errors.join("\n"))?;
    let selector = request.expr.parse()?;
    let focus = free_expression::parse_term(request.focus).map_err(|e| format!("focus: {e}"))?;
    let scope = request
        .scope
        .iter()
        .map(|(name, term)| {
            free_expression::parse_term(term)
                .map(|term| ((*name).to_owned(), term))
                .map_err(|e| format!("scope {name}: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let selected = selector.select(
        &shapes.dataset,
        &shapes.prefixes,
        shapes.base.as_deref().or(request.shapes_base),
    )?;
    let outputs = free_expression::evaluate(&FreeExpression {
        shapes: &selected.shapes,
        prefixes: &shapes.prefixes,
        root: &selected.root,
        data: data.as_ref(),
        focus: &focus,
        scope: &scope,
        imports: &imports,
    })?;
    Ok(outputs.iter().map(ToString::to_string).collect())
}

/// Split one `NAME=TERM` scope binding at its first `=` — the spelling the CLI's
/// `--scope` and the WASM and C-ABI scope arrays share.
///
/// # Errors
///
/// A binding with no `=`.
pub fn parse_scope_binding(binding: &str) -> Result<(&str, &str), String> {
    binding
        .split_once('=')
        .ok_or_else(|| format!("scope binding `{binding}` is not NAME=TERM: it has no `=`"))
}

/// Certify a Turtle shapes graph — its whole `owl:imports` closure, resolved against
/// `imports`: the loader's verdict, its `shacl-shacl.ttl` results and its function-call
/// bindings. See [`lint::lint`].
///
/// # Errors
///
/// [`ShapesError::Imports`] when the shapes graph's `owl:imports` closure is not in hand
/// or `imports` cannot be used — a lint of the importing document alone would certify a
/// shapes graph nobody asked about; [`ShapesError::Invalid`] when the shapes document
/// does not parse as Turtle. A shapes graph that parses but is malformed is not an
/// error: its refusal is the report's `load` section.
pub fn lint_shapes_ttl(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    imports: &ShapesImportList<'_>,
) -> Result<LintReport, ShapesError> {
    lint_shapes_ttl_with_shapes_graph(shapes_ttl, shapes_base, None, imports)
}

/// [`lint_shapes_ttl`] with the shapes-graph IRI the loader is configured with, as
/// `purrdf shapes lint --shapes-graph` configures it: a relative one resolves against
/// `shapes_base` ([`engine::resolve_shapes_graph_iri`]). `None` is [`lint_shapes_ttl`].
///
/// # Errors
///
/// Everything [`lint_shapes_ttl`] refuses, and [`ShapesError::Invalid`] for a
/// `shapes_graph` that names no graph.
pub fn lint_shapes_ttl_with_shapes_graph(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    shapes_graph: Option<&str>,
    imports: &ShapesImportList<'_>,
) -> Result<LintReport, ShapesError> {
    let shapes_graph = shapes_graph
        .map(|raw| engine::resolve_shapes_graph_iri(raw, shapes_base))
        .transpose()?;
    let table = ShapesImports::from_turtle(imports)?;
    let document =
        parse_turtle_document(shapes_ttl, shapes_base).map_err(|errors| errors.join("\n"))?;
    let table = read_under(table, shapes_base, document.base.as_deref());
    lint::lint(
        &document.dataset,
        &document.prefixes,
        None,
        shapes_graph,
        &table,
    )
}

/// `imports`, with the IRIs a shapes document was read under declared loaded: the base the
/// host parsed it under, and the base its own `@base` established. An `owl:imports` of
/// either names the document in hand — the same declaration
/// [`engine::parse_shapes_with_config`] makes for validation.
fn read_under(
    mut imports: ShapesImports,
    shapes_base: Option<&str>,
    document_base: Option<&str>,
) -> ShapesImports {
    for iri in shapes_base.into_iter().chain(document_base) {
        imports.declare_loaded(iri);
    }
    imports
}
