// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The string-in / string-out boundary of the three shapes-graph TOOLS the Python, WASM
//! and C-ABI hosts expose beside validation: running rules ([`apply_rules_to_ntriples`]),
//! evaluating one node expression ([`eval_node_expr_to_terms`]) and certifying a shapes
//! graph ([`lint_shapes_ttl`]).
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
//! # The term-generating round limit
//!
//! [`RulesRequest::max_term_generating_rounds`] is the host's knob over
//! [`purrdf_shapes::RuleOptions::with_max_term_generating_rounds`] and
//! [`purrdf_shapes::srl::InferOptions::with_max_term_generating_rounds`]: at most that many
//! evaluation rounds may infer a term the evaluation graph did not hold, and one more is a
//! failure naming the limit. `None` keeps the engine default
//! (`purrdf_datalog::seminaive::DEFAULT_MAX_TERM_GENERATING_ROUNDS`, 65,536), which a
//! trusted rule set that genuinely counts far needs. **A host running UNTRUSTED rule sets
//! should lower it**: a rule set whose term generation diverges — an exponential one in
//! particular — reaches the engine's fixed arena and join ceilings only slowly under the
//! default, and the limit is what bounds the time such a rule set can take.

use purrdf_shapes::data::ShaclData;
use purrdf_shapes::free_expression::{self, FreeExpression};
use purrdf_shapes::lint::{self, LintReport};
use purrdf_shapes::srl::{self, InferOptions};
use purrdf_shapes::text_ingest::{parse_ntriples_to_dataset, parse_turtle_document};
use purrdf_shapes::{Inference, RuleOptions, ShapesError, ShapesImports, engine};

use crate::ShapesImportList;

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
    /// The shapes graph's `owl:imports` table. Empty is the ordinary case, and still
    /// refuses a shapes graph that imports a document it does not hold.
    pub shapes_imports: &'a ShapesImportList<'a>,
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
/// SPARQL 1.2 RL route is [`srl::parse_and_check`] then [`srl::infer`]; an `IMPORTS` is a
/// failure, because this boundary reads nothing but the text it was handed.
///
/// # Errors
///
/// [`ShapesError::Imports`] when the shapes graph's `owl:imports` closure is not in hand
/// or its import table cannot be used. Otherwise [`ShapesError::Invalid`]: neither or
/// both rule sources named; a document that does not parse; a rule set that is
/// ill-formed, unstratifiable or fails during execution; the term-generating round
/// limit passed; or an import table handed to a SPARQL 1.2 RL rule set, which reads
/// none.
pub fn apply_rules_to_ntriples(request: &RulesRequest<'_>) -> Result<RulesOutcome, ShapesError> {
    let data = parse_ntriples_to_dataset(request.data_nt).map_err(|errors| errors.join("\n"))?;
    let inference: Inference = match (request.shapes_ttl, request.srl) {
        (Some(shapes_ttl), None) => {
            let shapes = engine::parse_shapes_with_config(
                shapes_ttl,
                request.shapes_base,
                None,
                &ShapesImports::from_turtle(request.shapes_imports)?,
            )?;
            let projected = engine::project_dataset(data.as_ref())?;
            let holder = ShaclData::new(std::sync::Arc::clone(&projected), projected, None);
            let mut options = RuleOptions::default();
            if let Some(rounds) = request.max_term_generating_rounds {
                options = options.with_max_term_generating_rounds(rounds);
            }
            purrdf_shapes::infer(&holder, &shapes, &options)?
        }
        (None, Some(text)) => {
            if !request.shapes_imports.is_empty() {
                return Err(ShapesError::Invalid(
                    "a shapes-graph import table was given with a SPARQL 1.2 RL rule set, which \
                     has no owl:imports to resolve it against; the table would be read and \
                     never used"
                        .to_owned(),
                ));
            }
            let document =
                srl::parse_and_check(text, request.srl_base).map_err(|e| e.to_string())?;
            if let Some(import) = document.imports().first() {
                return Err(ShapesError::Invalid(format!(
                    "the rule set imports {import}, and this boundary reads only the rule-set \
                     text it was handed; merge the imported rules into the text"
                )));
            }
            let mut options = InferOptions::default();
            if let Some(rounds) = request.max_term_generating_rounds {
                options = options.with_max_term_generating_rounds(rounds);
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

/// One node-expression evaluation across the host boundary.
#[derive(Debug, Clone, Copy)]
pub struct NodeExprRequest<'a> {
    /// The shapes graph carrying the expression, as Turtle.
    pub shapes_ttl: &'a str,
    /// The base IRI the shapes document's relative references resolve against.
    pub shapes_base: Option<&'a str>,
    /// The focus graph, as N-Triples.
    pub data_nt: &'a str,
    /// The expression node: an absolute IRI, or `_:label` for a blank node the shapes
    /// document labels (see [`free_expression::parse_term`]).
    pub expr: &'a str,
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
/// [`free_expression::evaluate`].
///
/// # Errors
///
/// [`ShapesError::Imports`] when the shapes graph's `owl:imports` closure is not in hand
/// or its import table cannot be used; [`ShapesError::Invalid`] for a document that does
/// not parse, a term that is not one, and anything else [`free_expression::evaluate`]
/// refuses.
pub fn eval_node_expr_to_terms(request: &NodeExprRequest<'_>) -> Result<Vec<String>, ShapesError> {
    let imports = ShapesImports::from_turtle(request.imports)?;
    let shapes = parse_turtle_document(request.shapes_ttl, request.shapes_base)
        .map_err(|errors| errors.join("\n"))?;
    let imports = read_under(imports, request.shapes_base, shapes.base.as_deref());
    let data = parse_ntriples_to_dataset(request.data_nt).map_err(|errors| errors.join("\n"))?;
    let root = free_expression::parse_term(request.expr).map_err(|e| format!("expr: {e}"))?;
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
    let outputs = free_expression::evaluate(&FreeExpression {
        shapes: &shapes.dataset,
        prefixes: &shapes.prefixes,
        root: &root,
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
    let table = ShapesImports::from_turtle(imports)?;
    let document =
        parse_turtle_document(shapes_ttl, shapes_base).map_err(|errors| errors.join("\n"))?;
    let table = read_under(table, shapes_base, document.base.as_deref());
    lint::lint(&document.dataset, &document.prefixes, None, None, &table)
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
