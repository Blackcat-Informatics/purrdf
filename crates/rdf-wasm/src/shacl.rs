// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL validation → SARIF 2.1.0, and the prepared-shapes-product codec, for the
//! wasm/JS surface.
//!
//! A thin shim over the wasm-clean SHACL engine and its SARIF reporting
//! boundary: validate a data graph (N-Triples) against a shapes graph (Turtle)
//! and return a SARIF 2.1.0 JSON string that editors and CI dashboards consume.
//!
//! # Validating a CHANGE rather than a graph
//!
//! `shaclValidateChangesToSarif` is the incremental twin: hand it both halves of
//! a delta — the rows joining the data graph and the rows leaving it — and the
//! engine expands the change into the focus nodes it can move and re-validates
//! exactly those. A browser host that edits a graph as the user types asks *what
//! did my last change break?* and pays for the change rather than for the graph.
//!
//! It returns a [`ShaclChangeValidation`] rather than a bare string, because the
//! log alone cannot say which question it answered: a bounded run reports about
//! the affected focus nodes, while a shapes graph whose constraints read through
//! SPARQL query text has no bounded footprint and falls back to validating the
//! whole mutated graph. The fallback is not optional — a short expansion and a
//! clean bill of health are indistinguishable in a report.
//!
//! # Prepared products, and why a refusal is a CLASS here rather than a message
//!
//! `shaclPackProduct` compiles a shapes graph once into a digest-chained product a
//! host can cache in IndexedDB, ship over the wire, or hold across a page load;
//! `shaclProductValidateToSarif` restores it instead of re-parsing. Those bytes are
//! UNTRUSTED when they come back — nothing in them is evidence of their own
//! provenance — so restoring one is an admission, and the codec refuses on a closed
//! set of named dimensions rather than one opaque error.
//!
//! `shaclProductValidateToSarifRebuild` is the forward-compatibility path:
//! `shaclProductValidateToSarif` refuses a product whose stage id this guest does
//! not know with `dimension === "stage-id"`, and rebuilding re-derives the
//! preparation from the shapes dataset the product carries instead of admitting
//! its memo — no RDF text is parsed and no file is read.
//!
//! Carrying that across the JS boundary as a `JsError` would delete it. The label
//! would survive only as a prefix of the message string, and the codec's own
//! documentation says matching on message text is not supported, so every JS
//! consumer would end up doing the thing the typed boundary exists to prevent. So
//! the four product functions reject with a [`ShaclProductRefusal`] — a class with a
//! `dimension` getter carrying the pinned kebab-case label, and a `message` getter
//! carrying the prose without it. `catch (e) { if (e.dimension === "stage-id") … }`
//! is the branch, and it is a field read rather than a substring search.
//!
//! Like every other wasm-bindgen class in this package, a caught
//! [`ShaclProductRefusal`] owns wasm memory and is released with `e.free()`.

use wasm_bindgen::prelude::*;

use purrdf_validate::{ShapesError, ShapesProductRefusal};

// ---------------------------------------------------------------------------
// The shapes graph's owl:imports
// ---------------------------------------------------------------------------

/// A shapes graph's `owl:imports` closure is not in hand, or the import table cannot be
/// used — the one refusal every shapes-graph entry point raises, on every PurRDF host
/// alike.
///
/// Every function that takes a Turtle shapes graph takes the caller's import table as two
/// trailing parallel arrays, `importIris` and `importDocuments` — entry `i` declares that
/// the ontology IRI `importIris[i]` names the Turtle document `importDocuments[i]`, parsed
/// with that IRI as its base; the same convention `entailCertainAnswers` uses. Omitted,
/// the table is empty, and the rule still applies: an `owl:imports` is resolved by a table
/// entry, by `shapesBase` (or the document's own `@base`) naming the imported document,
/// by the closure declaring the ontology (`<X> a owl:Ontology`, or an ontology whose
/// `owl:versionIRI` is `<X>`), or by the closure describing `<X>` with `sh:declare` —
/// SHACL's prefix-declaration idiom. Anything else rejects with this class rather than
/// validating a smaller shapes graph than the one named. PurRDF fetches nothing.
///
/// `kind` is the matchable half — `unresolved-import`, `unreached-import` (a table entry
/// no import names) or `invalid-import` (a key that is not an absolute IRI, a key named
/// twice, or a document that is not Turtle) — and `iris` the IRIs it names. `message` is
/// prose; do not match on it. Like every other wasm-bindgen class in this package, a
/// caught instance owns wasm memory and is released with `e.free()`.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct ShaclImportError {
    /// The engine's kebab-case kind label.
    kind: String,
    /// The IRIs the refusal names.
    iris: Vec<String>,
    /// The engine's own rendering.
    message: String,
}

#[wasm_bindgen]
impl ShaclImportError {
    /// `unresolved-import`, `unreached-import` or `invalid-import`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn kind(&self) -> String {
        self.kind.clone()
    }

    /// The IRIs the refusal names.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn iris(&self) -> Vec<String> {
        self.iris.clone()
    }

    /// The engine's explanation, naming the remedy.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// The engine's own rendering, which leads with the kind label.
    #[wasm_bindgen(js_name = toString)]
    #[must_use]
    pub fn to_js_string(&self) -> String {
        self.message.clone()
    }
}

impl From<&purrdf_validate::ShapesImportError> for ShaclImportError {
    fn from(error: &purrdf_validate::ShapesImportError) -> Self {
        Self {
            kind: error.kind().to_owned(),
            iris: error.iris().into_iter().map(ToOwned::to_owned).collect(),
            message: error.to_string(),
        }
    }
}

/// Reject with the shapes-graph error's JS form: a [`ShaclImportError`] for the import
/// refusal, a plain `Error` for anything else.
fn shapes_rejection(error: ShapesError) -> JsValue {
    match error {
        ShapesError::Imports(error) => ShaclImportError::from(&error).into(),
        ShapesError::Invalid(message) => JsError::new(&message).into(),
        ShapesError::ShaclJs(refusal) => JsError::new(refusal.message()).into(),
    }
}

/// The caller's two import arrays as the boundary's import table, with the length
/// agreement [`crate::entail::import_pairs`] enforces for every host table.
fn shapes_import_pairs<'a>(
    iris: &'a [String],
    documents: &'a [String],
) -> Result<Vec<(&'a str, &'a str)>, ShapesError> {
    crate::entail::import_pairs(iris, documents).map_err(ShapesError::Invalid)
}

/// Validate `data_nt` against `shapes_ttl` and render the report to SARIF 2.1.0.
///
/// Returns the plain Rust [`ShapesError`] (NOT a `JsValue`) so it is unit-testable on
/// the native build — constructing a JS error calls a wasm-only import that panics
/// off wasm. The `#[wasm_bindgen]` wrapper maps it through [`shapes_rejection`].
///
/// `conformance_disallows` is the request's conformance-disallow set as severity
/// IRIs; `None` is SHACL's default set, and an empty list or a non-IRI is an error.
pub(crate) fn validate_to_sarif_impl(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    conformance_disallows: Option<&[String]>,
    import_iris: &[String],
    import_documents: &[String],
) -> Result<String, ShapesError> {
    let imports = shapes_import_pairs(import_iris, import_documents)?;
    let validation = match conformance_disallows {
        None => purrdf_validate::ValidationOptions::default(),
        Some(iris) => purrdf_validate::ValidationOptions::default()
            .with_conformance_disallows(purrdf_validate::ConformanceDisallows::from_iris(iris)?),
    };
    purrdf_validate::validate_to_sarif_string(
        shapes_ttl,
        shapes_base,
        data_nt,
        &purrdf_validate::SarifOptions {
            validation,
            ..purrdf_validate::SarifOptions::default()
        },
        &imports,
    )
}

/// `shaclValidateToSarif(shapesTtl, dataNt, shapesBase?, conformanceDisallows?,
/// importIris?, importDocuments?)` → a SARIF 2.1.0 JSON string.
///
/// `shapesTtl` is a Turtle shapes graph; `dataNt` is an N-Triples data graph.
/// Throws (rejects) if either graph fails to parse.
///
/// `conformanceDisallows` is the conformance-disallow set: severity IRIs whose results make
/// the data non-conforming. Omitted, it is SHACL's default set (`sh:Violation`,
/// `sh:Warning`, `sh:Info`); an empty array or a non-IRI throws. The log's run carries
/// `properties.shaclConforms` and `properties.shaclConformanceDisallows` (the set the
/// report was judged against), because the results alone cannot say whether the data
/// conforms: an `sh:Debug` / `sh:Trace` result — SARIF `kind` `informational`, `level`
/// `none` — appears in the log of a conforming report.
///
/// `shapesBase` is the base IRI the SHAPES document's relative IRI references resolve
/// against. A browser or Node host has no retrieval IRI of its own — it was handed a
/// string — so PurRDF will not invent one; omit it and a relative reference is a hard
/// `iri-relative-no-base` naming the remedy. Passing the document's own URL is what makes
/// `<PersonShape>` in a fetched shapes graph mean what its author wrote. `dataNt` needs
/// no such parameter: N-Triples admits no relative IRI by grammar.
///
/// `importIris` / `importDocuments` are the shapes graph's `owl:imports` table (see
/// [`ShaclImportError`]); an incomplete closure rejects with that class.
#[wasm_bindgen(js_name = shaclValidateToSarif)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn shacl_validate_to_sarif(
    shapes_ttl: &str,
    data_nt: &str,
    shapes_base: Option<String>,
    conformance_disallows: Option<Vec<String>>,
    import_iris: Option<Vec<String>>,
    import_documents: Option<Vec<String>>,
) -> Result<String, JsValue> {
    validate_to_sarif_impl(
        shapes_ttl,
        shapes_base.as_deref(),
        data_nt,
        conformance_disallows.as_deref(),
        import_iris.as_deref().unwrap_or_default(),
        import_documents.as_deref().unwrap_or_default(),
    )
    .map_err(shapes_rejection)
}

/// The outcome of `shaclValidateChangesToSarif`: the SARIF log, and the SCOPE that
/// log describes.
///
/// Two facts rather than one, because a report alone cannot say which question it
/// answered. `bounded === true` means the log covers the focus nodes the change
/// could move — for those nodes it is identical, results and ordering alike, to a
/// full validation of the mutated graph — and is silent about a pre-existing
/// violation the change cannot reach, so an empty log means *this change
/// introduced no violation*. `bounded === false` means the shapes graph reads
/// through SPARQL query text, no bounded footprint exists for it, the call fell
/// back to a FULL validation of the mutated graph, and an empty log means *the
/// graph conforms*. The weaker reading is the dangerous one, so it is stated
/// rather than left to be assumed.
///
/// Like every other wasm-bindgen class in this package, this owns wasm memory and
/// is released with `.free()`.
#[wasm_bindgen]
#[derive(Debug)]
pub struct ShaclChangeValidation {
    /// The SARIF 2.1.0 log, rendered once here rather than on each read.
    sarif: String,
    /// Which question `sarif` answered, carried as the engine's own two-armed
    /// answer rather than re-spelled as a pair of nullable fields — a pair admits
    /// a fourth state the engine cannot produce.
    scope: purrdf_validate::ChangeScope,
}

#[wasm_bindgen]
impl ShaclChangeValidation {
    /// The SARIF 2.1.0 JSON log. See `bounded` for what it describes.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn sarif(&self) -> String {
        self.sarif.clone()
    }

    /// Whether the change's footprint could be bounded.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn bounded(&self) -> bool {
        self.scope.is_bounded()
    }

    /// How many focus nodes the change was expanded into, or `undefined` when the
    /// footprint could not be bounded and the whole graph was validated.
    ///
    /// `undefined` rather than the graph's node count on the fallback path: "every
    /// focus node in the graph" and a number are different statements, and
    /// collapsing them would make a fallback indistinguishable from a large
    /// bounded expansion.
    #[wasm_bindgen(getter = focusNodes)]
    #[must_use]
    pub fn focus_nodes(&self) -> Option<usize> {
        self.scope.focus_nodes()
    }

    /// Which construct made this shapes graph's change footprint unbounded, or
    /// `undefined` when it was bounded.
    ///
    /// Actionable rather than decorative: it names what to change to get
    /// incremental validation back.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn reason(&self) -> Option<String> {
        self.scope.reason().map(ToOwned::to_owned)
    }
}

/// Validate a CHANGE to `data_nt` against `shapes_ttl`, returning the SARIF log
/// beside the scope it describes. Native-testable core.
///
/// Returns a plain `String` error (NOT a `JsError`) for the reason
/// [`validate_to_sarif_impl`] does, and the engine's own `ChangeScope` rather than
/// the guest class, so the loop is exercisable off wasm.
pub(crate) fn validate_changes_to_sarif_impl(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    added_nt: Option<&str>,
    removed_nt: Option<&str>,
    import_iris: &[String],
    import_documents: &[String],
) -> Result<(String, purrdf_validate::ChangeScope), ShapesError> {
    let imports = shapes_import_pairs(import_iris, import_documents)?;
    purrdf_validate::validate_changes_to_sarif_string(
        shapes_ttl,
        shapes_base,
        data_nt,
        added_nt,
        removed_nt,
        &purrdf_validate::SarifOptions::default(),
        &imports,
    )
}

/// `shaclValidateChangesToSarif(shapesTtl, dataNt, addedNt?, removedNt?, shapesBase?,
/// importIris?, importDocuments?)` → a `ShaclChangeValidation` carrying a SARIF 2.1.0
/// JSON string and its scope.
///
/// The incremental twin of [`shacl_validate_to_sarif`]: instead of re-validating
/// the whole graph after an edit, hand it both halves of the delta and the engine
/// expands the change into the focus nodes it can move and re-validates exactly
/// those. A host that edits a graph and asks *what did my last change break?* pays
/// for the change rather than for the graph.
///
/// `addedNt` is the rows joining `dataNt` and `removedNt` the rows leaving it,
/// each an N-Triples string or omitted. Both halves, because a verdict moves when
/// a row leaves the graph as readily as when one joins, and one parameter would be
/// half a delta. Additions apply before removals, so a change naming the same row
/// on both halves settles on *removed*; a removal naming a row `dataNt` does not
/// carry retracts nothing rather than throwing, because a change set describes
/// what moved and does not assert what the base contained.
///
/// `shapesBase` carries the same meaning it does on [`shacl_validate_to_sarif`] —
/// the shapes document's own base IRI, supplied by the host because a wasm guest
/// has no retrieval IRI to derive one from.
///
/// **Read `bounded` before the log.** It decides what the log MEANS; see
/// [`ShaclChangeValidation`]. The unbounded fallback is not optional — a short
/// expansion and a clean bill of health are indistinguishable in a report.
///
/// `importIris` / `importDocuments` are the shapes graph's `owl:imports` table (see
/// [`ShaclImportError`]).
///
/// Throws (rejects) if the shapes graph or any of the three N-Triples documents
/// fails to parse, and with a [`ShaclImportError`] when the shapes graph's
/// `owl:imports` closure is not in hand. Call `.free()` on the returned object when
/// done.
#[wasm_bindgen(js_name = shaclValidateChangesToSarif)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn shacl_validate_changes_to_sarif(
    shapes_ttl: &str,
    data_nt: &str,
    added_nt: Option<String>,
    removed_nt: Option<String>,
    shapes_base: Option<String>,
    import_iris: Option<Vec<String>>,
    import_documents: Option<Vec<String>>,
) -> Result<ShaclChangeValidation, JsValue> {
    let (sarif, scope) = validate_changes_to_sarif_impl(
        shapes_ttl,
        shapes_base.as_deref(),
        data_nt,
        added_nt.as_deref(),
        removed_nt.as_deref(),
        import_iris.as_deref().unwrap_or_default(),
        import_documents.as_deref().unwrap_or_default(),
    )
    .map_err(shapes_rejection)?;
    Ok(ShaclChangeValidation { sarif, scope })
}

/// Entail `data_nt` under `shapes_ttl` and render the MATERIALIZED dataset (base
/// graph plus every SHACL-AF `sh:rule` inference) to canonical N-Triples.
///
/// The entailment twin of [`validate_to_sarif_impl`]: returns the plain Rust
/// [`ShapesError`] so it is unit-testable on the native build; the
/// `#[wasm_bindgen]` wrapper maps it through [`shapes_rejection`].
pub(crate) fn entail_to_ntriples_impl(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    import_iris: &[String],
    import_documents: &[String],
) -> Result<String, ShapesError> {
    let imports = shapes_import_pairs(import_iris, import_documents)?;
    purrdf_validate::entail_to_ntriples_string(shapes_ttl, shapes_base, data_nt, &imports)
}

/// `shaclEntail(shapesTtl, dataNt, shapesBase?, importIris?, importDocuments?)` → the
/// materialized dataset as an N-Triples string (the base graph plus every inferred
/// triple).
///
/// `shapesTtl` is a Turtle shapes graph; `dataNt` is an N-Triples data graph.
/// Throws (rejects) if either graph fails to parse or if rule application fails.
///
/// `shapesBase` carries the same meaning it does on
/// [`shacl_validate_to_sarif`] — the shapes document's own base IRI, supplied by the
/// host because a wasm guest has no retrieval IRI to derive one from.
///
/// Nothing is dropped on the way out: the underlying writer is the graph-carrying
/// canonical N-Quads serializer, and the output is N-Triples because BOTH inputs
/// are single-graph syntaxes, not because a graph slot was discarded.
///
/// `importIris` / `importDocuments` are the shapes graph's `owl:imports` table (see
/// [`ShaclImportError`]): an imported document's rules run.
#[wasm_bindgen(js_name = shaclEntail)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn shacl_entail(
    shapes_ttl: &str,
    data_nt: &str,
    shapes_base: Option<String>,
    import_iris: Option<Vec<String>>,
    import_documents: Option<Vec<String>>,
) -> Result<String, JsValue> {
    entail_to_ntriples_impl(
        shapes_ttl,
        shapes_base.as_deref(),
        data_nt,
        import_iris.as_deref().unwrap_or_default(),
        import_documents.as_deref().unwrap_or_default(),
    )
    .map_err(shapes_rejection)
}

// ---------------------------------------------------------------------------
// Shapes-graph tools: rules, node expressions, lint
// ---------------------------------------------------------------------------

/// The outcome of `shaclApplyRules`: the inference graph, and its proof when one was
/// asked for.
///
/// Like every other wasm-bindgen class in this package, this owns wasm memory and is
/// released with `.free()`.
#[wasm_bindgen]
#[derive(Debug)]
pub struct ShaclRulesInference {
    /// The inference graph as N-Triples.
    inferred: String,
    /// The proof text, when `explain` was set.
    proof: Option<String>,
}

#[wasm_bindgen]
impl ShaclRulesInference {
    /// The INFERENCE GRAPH — the inferred triples only, never the data graph — as
    /// N-Triples 1.2, one triple per line, in canonical order.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn inferred(&self) -> String {
        self.inferred.clone()
    }

    /// The proof of every inferred triple, or `undefined` when `explain` was not set:
    /// `derived S P O .`, then `  rule R` and one `  premise S P O .` per fact the rule's
    /// body matched, or `  data-block` for a SPARQL 1.2 RL data-block triple.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn proof(&self) -> Option<String> {
        self.proof.clone()
    }
}

/// Run a rule set over `data_nt`. Native-testable core of [`shacl_apply_rules`]; the
/// plain Rust [`ShapesError`] for the reason [`validate_to_sarif_impl`] gives.
pub(crate) fn apply_rules_impl(
    request: &purrdf_validate::RulesRequest<'_>,
) -> Result<purrdf_validate::RulesOutcome, ShapesError> {
    purrdf_validate::apply_rules_to_ntriples(request)
}

/// `shaclApplyRules(dataNt, shapesTtl?, srl?, shapesBase?, srlBase?, explain?,
/// maxTermGeneratingRounds?, importIris?, importDocuments?)` → a `ShaclRulesInference`.
///
/// Runs exactly one rule source over the N-Triples data graph: the SHACL 1.2 rules of the
/// Turtle shapes graph `shapesTtl` (its default rule set), or the SPARQL 1.2 RL rule set
/// `srl`. Naming neither or both throws. `shapesBase` / `srlBase` are the documents' base
/// IRIs — a guest has no retrieval IRI to derive one from.
///
/// `maxTermGeneratingRounds` (a `bigint`) bounds the evaluation rounds that infer a term
/// the graph did not hold; one more throws naming the limit. Omitted, the limit is a
/// divergence criterion derived from the input — at most max(256, 4 × N) such rounds for
/// N distinct input terms — past which the rule set is refused as divergent, naming its
/// rules. A rule set bounded by a constant past that horizon states its bound here.
///
/// `importIris` / `importDocuments` are the SHACL shapes graph's `owl:imports` table (see
/// [`ShaclImportError`]); an imported document's rules run. A SPARQL 1.2 RL rule set reads
/// no table, so passing one beside `srl` throws.
///
/// Throws on a document that does not parse, an ill-formed or unstratifiable rule set, a
/// rule failing during execution, and a passed round limit; rejects with a
/// [`ShaclImportError`] when the shapes graph's `owl:imports` closure is not in hand. Call
/// `.free()` on the result.
#[wasm_bindgen(js_name = shaclApplyRules)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
#[allow(
    clippy::too_many_arguments,
    reason = "each parameter is a distinct, independently-named input at the wasm boundary"
)]
pub fn shacl_apply_rules(
    data_nt: &str,
    shapes_ttl: Option<String>,
    srl: Option<String>,
    shapes_base: Option<String>,
    srl_base: Option<String>,
    explain: Option<bool>,
    max_term_generating_rounds: Option<u64>,
    import_iris: Option<Vec<String>>,
    import_documents: Option<Vec<String>>,
) -> Result<ShaclRulesInference, JsValue> {
    let imports = shapes_import_pairs(
        import_iris.as_deref().unwrap_or_default(),
        import_documents.as_deref().unwrap_or_default(),
    )
    .map_err(shapes_rejection)?;
    let outcome = apply_rules_impl(&purrdf_validate::RulesRequest {
        data_nt,
        shapes_ttl: shapes_ttl.as_deref(),
        shapes_base: shapes_base.as_deref(),
        shapes_imports: &imports,
        srl: srl.as_deref(),
        srl_base: srl_base.as_deref(),
        explain: explain.unwrap_or(false),
        max_term_generating_rounds,
    })
    .map_err(shapes_rejection)?;
    Ok(ShaclRulesInference {
        inferred: outcome.inferred_ntriples,
        proof: outcome.proof,
    })
}

/// The expression selector's four optional inputs, as `shaclEvalNodeExpr` receives them:
/// `expr`, `exprAt`, `exprVia` and `exprTurtle`.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ExprInputs<'a> {
    pub(crate) expr: Option<&'a str>,
    pub(crate) at: Option<&'a str>,
    pub(crate) via: &'a [String],
    pub(crate) turtle: Option<&'a str>,
}

#[cfg(test)]
impl<'a> ExprInputs<'a> {
    /// `expr` alone — the node form.
    const fn node(expr: &'a str) -> Self {
        Self {
            expr: Some(expr),
            at: None,
            via: &[],
            turtle: None,
        }
    }
}

/// Evaluate one node expression. Native-testable core of [`shacl_eval_node_expr`]:
/// `scope` holds `NAME=TERM` bindings, and `expr` is mapped to the one selector it names
/// by [`purrdf_validate::ExprSelector::from_parts`].
#[allow(
    clippy::too_many_arguments,
    reason = "each parameter is a distinct, independently-named input at the wasm boundary"
)]
pub(crate) fn eval_node_expr_impl(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    expr: ExprInputs<'_>,
    focus: &str,
    scope: &[String],
    import_iris: &[String],
    import_documents: &[String],
) -> Result<Vec<String>, ShapesError> {
    let via: Vec<&str> = expr.via.iter().map(String::as_str).collect();
    let expr = purrdf_validate::ExprSelector::from_parts(expr.expr, expr.at, &via, expr.turtle)?;
    let imports = shapes_import_pairs(import_iris, import_documents)?;
    let bindings = scope
        .iter()
        .map(|binding| purrdf_validate::parse_scope_binding(binding))
        .collect::<Result<Vec<_>, _>>()?;
    purrdf_validate::eval_node_expr_to_terms(&purrdf_validate::NodeExprRequest {
        shapes_ttl,
        shapes_base,
        data_nt,
        expr,
        focus,
        scope: &bindings,
        imports: &imports,
    })
}

/// `shaclEvalNodeExpr(shapesTtl, dataNt, expr, focus, scope?, shapesBase?, importIris?,
/// importDocuments?, exprAt?, exprVia?, exprTurtle?)` → the output nodes, as an array of
/// N-Triples 1.2 terms in the order the expression's sequence semantics define.
///
/// Evaluates ONE node expression of the Turtle shapes graph — SHACL 1.2 Node Expressions'
/// `evalExpr(expr, focusGraph, focusNode, scope)` — against a focus node of the
/// N-Triples data graph. The expression is named exactly one way: `expr` is an absolute
/// IRI or `"_:label"` for a blank node the shapes document labels so; or `expr` is
/// `undefined` and `exprAt` names a node and `exprVia` the predicate IRIs a walk from it
/// follows, each step reaching exactly one value (how an anonymous `[ … ]` expression is
/// named); or `expr` is `undefined` and `exprTurtle` is the expression as a Turtle
/// document, read under the shapes document's prefixes and base, whose one root blank node
/// is the expression. None or several selectors, a walk step reaching no value or several,
/// and an inline document without exactly one root throw. `focus` is an absolute IRI or
/// any N-Triples term; `scope` is
/// an array of `"NAME=TERM"` bindings read by `shnex:var "NAME"`, the term spelled as
/// `focus` is. Throws on a label the shapes document never wrote, a binding named
/// `focusNode` or bound twice (neither could ever be read), and any parse or evaluation
/// failure. `importIris` / `importDocuments` are the shapes graph's `owl:imports` table
/// (see [`ShaclImportError`]); an imported document's functions and shapes are in scope.
#[wasm_bindgen(js_name = shaclEvalNodeExpr)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
#[allow(
    clippy::too_many_arguments,
    reason = "each parameter is a distinct, independently-named input at the wasm boundary"
)]
pub fn shacl_eval_node_expr(
    shapes_ttl: &str,
    data_nt: &str,
    expr: Option<String>,
    focus: &str,
    scope: Option<Vec<String>>,
    shapes_base: Option<String>,
    import_iris: Option<Vec<String>>,
    import_documents: Option<Vec<String>>,
    expr_at: Option<String>,
    expr_via: Option<Vec<String>>,
    expr_turtle: Option<String>,
) -> Result<Vec<String>, JsValue> {
    eval_node_expr_impl(
        shapes_ttl,
        shapes_base.as_deref(),
        data_nt,
        ExprInputs {
            expr: expr.as_deref(),
            at: expr_at.as_deref(),
            via: expr_via.as_deref().unwrap_or_default(),
            turtle: expr_turtle.as_deref(),
        },
        focus,
        scope.as_deref().unwrap_or_default(),
        import_iris.as_deref().unwrap_or_default(),
        import_documents.as_deref().unwrap_or_default(),
    )
    .map_err(shapes_rejection)
}

/// The outcome of `shaclLintShapes`: the cold-certify report of a shapes graph.
///
/// Like every other wasm-bindgen class in this package, this owns wasm memory and is
/// released with `.free()`.
#[wasm_bindgen]
#[derive(Debug)]
pub struct ShaclLintReport {
    /// The engine's report.
    report: purrdf_validate::LintReport,
}

#[wasm_bindgen]
impl ShaclLintReport {
    /// Whether the report carries no finding: the loader accepted the graph and every
    /// `shacl-shacl.ttl` result is superseded (flagged there, well-formed SHACL 1.2 Core).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn clean(&self) -> bool {
        self.report.is_clean()
    }

    /// The finding count: one for a load refusal, plus every `shacl-shacl.ttl` result no
    /// supersession covers.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn findings(&self) -> usize {
        self.report.findings()
    }

    /// The loader's refusal, or `undefined` when it accepted the graph.
    #[wasm_bindgen(getter = loadError)]
    #[must_use]
    pub fn load_error(&self) -> Option<String> {
        self.report.load_error().map(ToOwned::to_owned)
    }

    /// The whole report as the deterministic text every PurRDF host prints: the `load`,
    /// `shacl-shacl` (`result …` lines, `superseded NAME` where SHACL 1.2 Core makes the
    /// flagged graph well-formed), `functions` (`call BINDING <IRI> in OWNER`) and
    /// `validators` (`alternative <COMPONENT> <ATTACHMENT> VALIDATOR LANGUAGE
    /// superseded-by-native`, one per validator declared for a built-in component)
    /// sections, then `findings N` and `clean true|false`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn report(&self) -> String {
        self.report.render()
    }
}

/// Certify a shapes graph. Native-testable core of [`shacl_lint_shapes`].
pub(crate) fn lint_shapes_impl(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    import_iris: &[String],
    import_documents: &[String],
) -> Result<purrdf_validate::LintReport, ShapesError> {
    let imports = shapes_import_pairs(import_iris, import_documents)?;
    purrdf_validate::lint_shapes_ttl(shapes_ttl, shapes_base, &imports)
}

/// `shaclLintShapes(shapesTtl, shapesBase?, importIris?, importDocuments?)` → a
/// `ShaclLintReport`.
///
/// Certifies a Turtle shapes graph COLD — its whole `owl:imports` closure, resolved
/// against `importIris` / `importDocuments` (see [`ShaclImportError`]): the loader's
/// verdict, every result of validating it against the W3C `shacl-shacl.ttl`, and which
/// implementation every node-expression function call binds to (`native`, `custom`,
/// `sparql-registered`, `host-extension`). Rejects with a [`ShaclImportError`] when the
/// closure is not in hand — never a report about the importing document alone. Otherwise
/// throws only when the document is not Turtle; a malformed shapes graph is a report with
/// findings, not an exception. Call `.free()` on the result.
#[wasm_bindgen(js_name = shaclLintShapes)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn shacl_lint_shapes(
    shapes_ttl: &str,
    shapes_base: Option<String>,
    import_iris: Option<Vec<String>>,
    import_documents: Option<Vec<String>>,
) -> Result<ShaclLintReport, JsValue> {
    lint_shapes_impl(
        shapes_ttl,
        shapes_base.as_deref(),
        import_iris.as_deref().unwrap_or_default(),
        import_documents.as_deref().unwrap_or_default(),
    )
    .map(|report| ShaclLintReport { report })
    .map_err(shapes_rejection)
}

// ---------------------------------------------------------------------------
// Prepared shapes products
// ---------------------------------------------------------------------------

/// A refusal from the prepared-shapes-product admission boundary, thrown by the four
/// `shaclProduct*` functions.
///
/// `dimension` is the stable, matchable half: one of the codec's pinned kebab-case
/// labels (`magic`, `format-version`, `stage-id`, `profile`, `truncated`, `trailer`,
/// `section-digest`, `container-digest`, `dataset-identity`, `shapes-graph`,
/// `prefixes`, `base`, `vocabulary`, `function-registry`, `aggregate-registry`,
/// `property-function-registry`, `class-catalog`, `unsupported-capability`,
/// `depth-limit`, `malformed`), or `undefined` when the failure happened before any
/// product existed — a shapes or data DOCUMENT that did not parse was never admitted,
/// and naming a dimension for it would claim a product was inspected when none was.
///
/// `message` is prose for a human: it names the action that resolves the refusal,
/// not only the condition that caused it. Do not match on it.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct ShaclProductRefusal {
    /// The pinned kebab-case dimension label, absent for a pre-admission failure.
    dimension: Option<String>,
    /// The prescriptive explanation, without the dimension label.
    message: String,
}

#[wasm_bindgen]
impl ShaclProductRefusal {
    /// The admission dimension that refused, or `undefined`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn dimension(&self) -> Option<String> {
        self.dimension.clone()
    }

    /// The prescriptive explanation, without the dimension label.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// `<dimension>: <message>`, or the message alone when there is no dimension —
    /// the codec's own rendering, so a host that only logs still sees the label.
    #[wasm_bindgen(js_name = toString)]
    #[must_use]
    pub fn to_js_string(&self) -> String {
        match &self.dimension {
            Some(dimension) => format!("{dimension}: {}", self.message),
            None => self.message.clone(),
        }
    }
}

impl From<ShapesProductRefusal> for ShaclProductRefusal {
    /// Carry the boundary's refusal across unchanged: the label where there is one,
    /// the prose either way. Nothing is re-worded and nothing is dropped.
    fn from(refusal: ShapesProductRefusal) -> Self {
        Self {
            dimension: refusal.dimension_label().map(ToOwned::to_owned),
            message: refusal.message().into_owned(),
        }
    }
}

/// Compile a Turtle shapes graph into a prepared product. Native-testable core.
///
/// Returns the plain Rust refusal so this is exercisable off wasm; the
/// `#[wasm_bindgen]` wrapper converts it to [`ShaclProductRefusal`].
pub(crate) fn pack_product_impl(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    import_iris: &[String],
    import_documents: &[String],
) -> Result<Vec<u8>, ShapesProductRefusal> {
    let imports =
        shapes_import_pairs(import_iris, import_documents).map_err(ShapesProductRefusal::Shapes)?;
    purrdf_validate::pack_shapes_product(shapes_ttl, shapes_base, &imports)
}

/// `shaclPackProduct(shapesTtl, shapesBase?, importIris?, importDocuments?)` → the
/// prepared product as a `Uint8Array`.
///
/// Compile once, restore many times: the product carries the compiled SHACL model AND
/// the shapes dataset it came from, under a per-section SHA-256 and a whole-container
/// digest, plus the binding of every input it was compiled against.
///
/// Byte-deterministic — no clock, no randomness and no hash-iteration order reach the
/// writer — so two calls over the same shapes graph and base produce identical bytes
/// and a content-addressed cache key over them is stable.
///
/// `shapesBase` carries the same meaning it does on
/// [`shacl_validate_to_sarif`] — the shapes document's own base IRI, supplied by the
/// host because a wasm guest has no retrieval IRI to derive one from. It is RECORDED
/// in the product, so a restore resolves the same relative references without the
/// document.
///
/// `importIris` / `importDocuments` are the shapes graph's `owl:imports` table (see
/// [`ShaclImportError`]); the product carries the merged closure, so a restore needs no
/// documents.
///
/// Rejects with a [`ShaclImportError`] — the same refusal `shaclValidateToSarif` raises —
/// when the shapes graph's `owl:imports` closure is not in hand, and with a
/// [`ShaclProductRefusal`] otherwise; call `.free()` on either when done.
#[wasm_bindgen(js_name = shaclPackProduct)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn shacl_pack_product(
    shapes_ttl: &str,
    shapes_base: Option<String>,
    import_iris: Option<Vec<String>>,
    import_documents: Option<Vec<String>>,
) -> Result<Vec<u8>, JsValue> {
    pack_product_impl(
        shapes_ttl,
        shapes_base.as_deref(),
        import_iris.as_deref().unwrap_or_default(),
        import_documents.as_deref().unwrap_or_default(),
    )
    .map_err(|refusal| match refusal.import_error() {
        Some(error) => ShaclImportError::from(error).into(),
        None => ShaclProductRefusal::from(refusal).into(),
    })
}

/// Read a prepared product's self-description. Native-testable core.
pub(crate) fn product_explain_impl(product: &[u8]) -> Result<String, ShapesProductRefusal> {
    purrdf_validate::explain_shapes_product(product).map_err(ShapesProductRefusal::from)
}

/// `shaclProductExplain(product)` → what the product says it was compiled from, as
/// deterministic `key value` lines, WITHOUT admitting it.
///
/// The container format version, the preparation stage id and whether this build knows
/// it, the identity digest and every labelled identity component, then the recorded
/// parse inputs (base, `sh:shapesGraph` IRI, prefix map). This is what makes a named
/// refusal actionable: a restore rejected with `dimension === "prefixes"` is answered
/// by reading which prefix map the product actually carries, not by guessing.
///
/// Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done.
#[wasm_bindgen(js_name = shaclProductExplain)]
pub fn shacl_product_explain(product: &[u8]) -> Result<String, ShaclProductRefusal> {
    product_explain_impl(product).map_err(ShaclProductRefusal::from)
}

/// Corroborate a prepared product's carried dataset against its claimed identity.
/// Native-testable core.
pub(crate) fn product_certify_impl(product: &[u8]) -> Result<(), ShapesProductRefusal> {
    purrdf_validate::certify_shapes_product(product).map_err(ShapesProductRefusal::from)
}

/// `shaclProductCertify(product)` → resolves when the product's shapes dataset
/// canonicalizes to the digest its own identity binding claims.
///
/// The codec's COLD path, and deliberately unreachable from a restore:
/// canonicalization is a graph-isomorphism computation over the shapes graph's blank
/// nodes and can cost more than the shapes parse a product exists to eliminate. Call
/// it from a build step or a test, not before every validation —
/// `shaclProductValidateToSarif` already verifies every section digest and the whole
/// container.
///
/// Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done.
#[wasm_bindgen(js_name = shaclProductCertify)]
pub fn shacl_product_certify(product: &[u8]) -> Result<(), ShaclProductRefusal> {
    product_certify_impl(product).map_err(ShaclProductRefusal::from)
}

/// Admit a prepared product and validate a data graph with it. Native-testable core.
pub(crate) fn product_validate_impl(
    product: &[u8],
    data_nt: &str,
) -> Result<String, ShapesProductRefusal> {
    purrdf_validate::validate_with_shapes_product(
        product,
        data_nt,
        &purrdf_validate::SarifOptions::default(),
    )
}

/// `shaclProductValidateToSarif(product, dataNt)` → a SARIF 2.1.0 JSON string.
///
/// The point of a product: restore the preparation instead of re-parsing the shapes
/// graph, then validate. The verdict is the identical one
/// [`shacl_validate_to_sarif`] reaches over the shapes document the product was packed
/// from — the same engine entry point runs, over the same restored `Shapes`.
///
/// Admission runs first and in full: the container's framing, every section digest,
/// the whole-container digest, then the product's stage id, profile and complete input
/// binding, all before a single focus node is resolved. A product prepared under a
/// different prefix map, base, vocabulary or registry is REFUSED rather than validated
/// into a report about a shapes graph nobody asked for.
///
/// Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done. A malformed
/// `dataNt` rejects with `dimension === undefined`: the data graph is not a product and
/// no admission dimension names it.
#[wasm_bindgen(js_name = shaclProductValidateToSarif)]
pub fn shacl_product_validate_to_sarif(
    product: &[u8],
    data_nt: &str,
) -> Result<String, ShaclProductRefusal> {
    product_validate_impl(product, data_nt).map_err(ShaclProductRefusal::from)
}

/// Rebuild a prepared product and validate a data graph with it. Native-testable
/// core.
pub(crate) fn product_validate_rebuild_impl(
    product: &[u8],
    data_nt: &str,
) -> Result<String, ShapesProductRefusal> {
    purrdf_validate::validate_with_rebuilt_shapes_product(
        product,
        data_nt,
        &purrdf_validate::SarifOptions::default(),
    )
}

/// `shaclProductValidateToSarifRebuild(product, dataNt)` → a SARIF 2.1.0 JSON
/// string, restoring the preparation by RE-DERIVING it from the shapes dataset the
/// product carries rather than admitting its memo.
///
/// The forward-compatibility path: [`shacl_product_validate_to_sarif`] refuses a
/// product whose stage id this guest does not know with `dimension ===
/// "stage-id"`, and this is the remedy it names. No RDF text is parsed and no
/// file is read — the dataset travels inside the product under the envelope's
/// own digests, and this re-derives the shapes graph from it.
///
/// Also correct, and does the identical work, over a CURRENT product whose stage
/// id this guest already knows: rebuilding re-derives from the SAME carried
/// dataset [`shacl_product_validate_to_sarif`] restores a memo of, so the two
/// reach the byte-identical report. This function is a second DOOR onto one
/// product, never a second, divergent answer.
///
/// Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done. A
/// malformed `dataNt` rejects with `dimension === undefined`, for the same
/// reason [`shacl_product_validate_to_sarif`] does.
#[wasm_bindgen(js_name = shaclProductValidateToSarifRebuild)]
pub fn shacl_product_validate_to_sarif_rebuild(
    product: &[u8],
    data_nt: &str,
) -> Result<String, ShaclProductRefusal> {
    product_validate_rebuild_impl(product, data_nt).map_err(ShaclProductRefusal::from)
}

/// Rebuild a prepared product bound to an expected identity and validate a data
/// graph with it. Native-testable core.
///
/// The selector arrives as TEXT for the same reason it does on
/// [`product_validate_expecting_impl`]: it is the only shape a JavaScript host
/// can hold it in, and decoding it here rather than at the boundary keeps this
/// exercisable off wasm exactly as its siblings are.
pub(crate) fn product_validate_rebuild_expecting_impl(
    product: &[u8],
    data_nt: &str,
    expect_identity: &str,
) -> Result<String, ShaclProductRefusal> {
    // A selector that is not 64 hexadecimal digits refused BEFORE the product is
    // opened, so the refusal carries no dimension: nothing was inspected, and
    // naming a dimension would claim the product was at fault for the caller's
    // argument.
    let expected = purrdf_validate::parse_identity_digest(expect_identity).map_err(|message| {
        ShaclProductRefusal {
            dimension: None,
            message,
        }
    })?;
    purrdf_validate::validate_with_rebuilt_shapes_product_expecting(
        product,
        data_nt,
        &expected,
        &purrdf_validate::SarifOptions::default(),
    )
    .map_err(ShaclProductRefusal::from)
}

/// `shaclProductValidateToSarifRebuildExpecting(product, dataNt, expectIdentity)` →
/// a SARIF 2.1.0 JSON string, restoring the preparation by RE-DERIVING it from
/// the shapes dataset the product carries rather than admitting its memo, but
/// only from the product whose input binding is `expectIdentity`.
///
/// The bound twin of [`shacl_product_validate_to_sarif_rebuild`], for the same
/// reason [`shacl_product_validate_to_sarif_expecting`] exists beside
/// [`shacl_product_validate_to_sarif`]: the forward-compatibility rescue is not
/// a reason to stop asking *is this the product the host meant?* — a product
/// fetched over the network or read out of a cache under a stage id this guest
/// does not recognize is still just bytes that could be the wrong ones. The
/// 32-byte comparison runs FIRST, ahead of the re-derivation, exactly as it does
/// on [`shacl_product_validate_to_sarif_expecting`].
///
/// `expectIdentity` carries the same meaning it does on
/// [`shacl_product_validate_to_sarif_expecting`] — the 64 hexadecimal digits
/// `shaclProductExplain` prints on its `identity-digest` line.
///
/// Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done. A
/// product carrying a different binding rejects with `dimension ===
/// "shapes-graph"`; an `expectIdentity` that is not 64 hexadecimal digits
/// rejects with `dimension === undefined`, because no product was ever
/// inspected.
#[wasm_bindgen(js_name = shaclProductValidateToSarifRebuildExpecting)]
pub fn shacl_product_validate_to_sarif_rebuild_expecting(
    product: &[u8],
    data_nt: &str,
    expect_identity: &str,
) -> Result<String, ShaclProductRefusal> {
    product_validate_rebuild_expecting_impl(product, data_nt, expect_identity)
}

/// Admit a prepared product bound to an expected identity and validate a data graph
/// with it. Native-testable core.
///
/// The selector arrives as TEXT because that is the only shape a JavaScript host can
/// hold it in, and it is decoded here rather than at the boundary so this is
/// exercisable off wasm exactly as its siblings are.
///
/// This core returns the GUEST-facing refusal where its siblings return the plain Rust
/// one, because it is the only product entry point with a failure the Rust type cannot
/// spell honestly: a selector that is not a digest is neither an admission refusal nor
/// a shapes document that did not parse. Converting at the wasm wrapper would mean
/// inventing one of those two claims here and unpicking it there.
pub(crate) fn product_validate_expecting_impl(
    product: &[u8],
    data_nt: &str,
    expect_identity: &str,
) -> Result<String, ShaclProductRefusal> {
    // A selector that is not 64 hexadecimal digits refused BEFORE the product is
    // opened, so the refusal carries no dimension: nothing was inspected, and naming
    // a dimension would claim the product was at fault for the caller's argument.
    let expected = purrdf_validate::parse_identity_digest(expect_identity).map_err(|message| {
        ShaclProductRefusal {
            dimension: None,
            message,
        }
    })?;
    purrdf_validate::validate_with_shapes_product_expecting(
        product,
        data_nt,
        &expected,
        &purrdf_validate::SarifOptions::default(),
    )
    .map_err(ShaclProductRefusal::from)
}

/// `shaclProductValidateToSarifExpecting(product, dataNt, expectIdentity)` → a SARIF
/// 2.1.0 JSON string, but only from the product whose input binding is
/// `expectIdentity`.
///
/// Everything [`shacl_product_validate_to_sarif`] checks is a question about the
/// executing guest — its build, its registries, its class analysis. None of them asks
/// whether these are the bytes the host meant, because nothing in a product states
/// which product was wanted. A host that fetches a product over the network, reads one
/// out of a cache, or builds its path from a configuration string has no other way to
/// say so, and admitting the wrong one produces a decided, well-formed SARIF log about
/// a shapes graph nobody asked about.
///
/// `expectIdentity` is the 64 hexadecimal digits `shaclProductExplain` prints on its
/// `identity-digest` line — one spelling, readable off the artifact, so the selector
/// can be pinned in a manifest beside the product it names.
///
/// Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done. A product
/// carrying a different binding rejects with `dimension === "shapes-graph"`; an
/// `expectIdentity` that is not 64 hexadecimal digits rejects with
/// `dimension === undefined`, because no product was ever inspected.
#[wasm_bindgen(js_name = shaclProductValidateToSarifExpecting)]
pub fn shacl_product_validate_to_sarif_expecting(
    product: &[u8],
    data_nt: &str,
    expect_identity: &str,
) -> Result<String, ShaclProductRefusal> {
    product_validate_expecting_impl(product, data_nt, expect_identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
        ex:PersonShape a sh:NodeShape ;\n\
          sh:targetClass ex:Person ;\n\
          sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\n";

    const DATA: &str = "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n\
        <http://example.org/alice> <http://example.org/age> \"nope\" .\n";

    #[test]
    fn validate_emits_sarif_2_1_0() {
        let sarif =
            validate_to_sarif_impl(SHAPES, None, DATA, None, &[], &[]).expect("sarif produced");
        assert!(sarif.contains("\"version\": \"2.1.0\""));
        assert!(sarif.contains("\"level\": \"error\""));
    }

    /// The conformance-disallow set reaches the validation: a Warning-only
    /// graph does not conform under the default set, conforms under
    /// {sh:Violation}, and an empty set or a non-IRI is an error.
    #[test]
    fn wasm_validate_conformance_disallows() {
        let shapes = SHAPES.replace(
            "sh:path ex:age ;",
            "sh:path ex:age ; sh:severity sh:Warning ;",
        );
        let conforms = |disallows: Option<&[String]>| -> serde_json::Value {
            let sarif = validate_to_sarif_impl(&shapes, None, DATA, disallows, &[], &[])
                .expect("sarif produced");
            let log: serde_json::Value = serde_json::from_str(&sarif).expect("json");
            log["runs"][0]["properties"]["shaclConforms"].clone()
        };
        assert_eq!(conforms(None), serde_json::json!(false));
        let violation = ["http://www.w3.org/ns/shacl#Violation".to_owned()];
        assert_eq!(conforms(Some(&violation)), serde_json::json!(true));
        assert!(validate_to_sarif_impl(&shapes, None, DATA, Some(&[]), &[], &[]).is_err());
        assert!(
            validate_to_sarif_impl(
                &shapes,
                None,
                DATA,
                Some(&["Violation".to_owned()]),
                &[],
                &[]
            )
            .is_err()
        );
    }

    #[test]
    fn malformed_shapes_is_an_error() {
        assert!(validate_to_sarif_impl("@@@ not turtle", None, DATA, None, &[], &[]).is_err());
    }

    /// A conforming base, so every violation a change test sees is the change's.
    const CHANGE_BASE: &str = "<http://example.org/alice> \
        <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";

    /// The row that breaks it.
    const BAD_AGE: &str = "<http://example.org/alice> <http://example.org/age> \"nope\" .\n";

    /// The bounded arm: the change is validated, the scope says what the log is
    /// about, and the log is the one a full validation of the merged graph
    /// produces — a cheaper route to ONE answer, never a second answer.
    #[test]
    fn a_change_reaches_the_full_validations_own_log() {
        let (sarif, scope) = validate_changes_to_sarif_impl(
            SHAPES,
            None,
            CHANGE_BASE,
            Some(BAD_AGE),
            None,
            &[],
            &[],
        )
        .expect("the change validates");
        assert_eq!(scope.focus_nodes(), Some(1));
        assert!(scope.is_bounded());
        assert_eq!(
            sarif,
            validate_to_sarif_impl(
                SHAPES,
                None,
                &format!("{CHANGE_BASE}{BAD_AGE}"),
                None,
                &[],
                &[]
            )
            .expect("full validation"),
        );

        // The retract half is a real half: taking the row back out restores
        // conformance through the same one call.
        let merged = format!("{CHANGE_BASE}{BAD_AGE}");
        let (sarif, scope) =
            validate_changes_to_sarif_impl(SHAPES, None, &merged, None, Some(BAD_AGE), &[], &[])
                .expect("the retraction validates");
        assert!(scope.is_bounded());
        assert!(!sarif.contains("\"level\": \"error\""), "{sarif}");
    }

    /// The fallback arm, and the guest class that carries it: a shapes graph
    /// reading through query text validates EVERYTHING and says why.
    #[test]
    fn an_unbounded_footprint_is_reported_as_such_across_the_boundary() {
        const SPARQL_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
            @prefix ex: <http://example.org/> .\n\
            ex:PersonShape a sh:NodeShape ;\n\
              sh:targetClass ex:Person ;\n\
              sh:sparql [ a sh:SPARQLConstraint ;\n\
                sh:message \"every person needs a name\" ;\n\
                sh:select \"\"\"SELECT $this WHERE { FILTER NOT EXISTS \
                  { $this <http://example.org/name> ?n } }\"\"\" ] .\n";
        let (sarif, scope) = validate_changes_to_sarif_impl(
            SPARQL_SHAPES,
            None,
            CHANGE_BASE,
            Some(BAD_AGE),
            None,
            &[],
            &[],
        )
        .expect("the change validates");
        let validation = ShaclChangeValidation { sarif, scope };

        assert!(!validation.bounded());
        assert_eq!(
            validation.focus_nodes(),
            None,
            "a fallback covers no COUNT: every focus node is not a number",
        );
        assert!(validation.reason().is_some_and(|reason| !reason.is_empty()));
        assert!(
            validation.sarif().contains("alice"),
            "the fallback validated the whole graph",
        );

        // The neighbouring BOUNDED case still reports a count and no reason.
        let (sarif, scope) = validate_changes_to_sarif_impl(
            SHAPES,
            None,
            CHANGE_BASE,
            Some(BAD_AGE),
            None,
            &[],
            &[],
        )
        .expect("the change validates");
        let bounded = ShaclChangeValidation { sarif, scope };
        assert!(bounded.bounded());
        assert_eq!(bounded.focus_nodes(), Some(1));
        assert_eq!(bounded.reason(), None);
    }

    #[test]
    fn a_malformed_change_document_is_an_error() {
        assert!(
            validate_changes_to_sarif_impl(
                SHAPES,
                None,
                CHANGE_BASE,
                Some("@@@ not n-triples"),
                None,
                &[],
                &[],
            )
            .is_err()
        );
        // The neighbouring VALID case still succeeds — a refusal is a claim too.
        validate_changes_to_sarif_impl(SHAPES, None, CHANGE_BASE, Some(BAD_AGE), None, &[], &[])
            .expect("a well-formed change document still validates");
    }

    // A shapes graph with a `sh:TripleRule` that types every `ex:Person` as an
    // `ex:adult` — the entailment analogue of the SARIF validation fixtures.
    const RULE_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        ex:PersonRule a sh:NodeShape ;\n\
          sh:targetClass ex:Person ;\n\
          sh:rule [ a sh:TripleRule ;\n\
            sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .\n";

    const RULE_DATA: &str = "<http://example.org/alice> \
        <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";

    #[test]
    fn entail_materializes_inferred_triple() {
        let nt = entail_to_ntriples_impl(RULE_SHAPES, None, RULE_DATA, &[], &[])
            .expect("entailment produced");
        assert!(nt.contains(
            "<http://example.org/alice> <http://example.org/adult> <http://example.org/yes> ."
        ));
        // The base fact survives into the materialized dataset.
        assert!(nt.contains(
            "<http://example.org/alice> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> ."
        ));
    }

    #[test]
    fn entail_malformed_shapes_is_an_error() {
        assert!(entail_to_ntriples_impl("@@@ not turtle", None, RULE_DATA, &[], &[]).is_err());
    }

    #[test]
    fn a_product_round_trips_and_reaches_the_same_verdict() {
        let product = pack_product_impl(SHAPES, None, &[], &[]).expect("product packed");
        product_certify_impl(&product).expect("certified");
        assert!(
            product_explain_impl(&product)
                .expect("explained")
                .contains("stage-known true\n")
        );

        // Restoring the product and parsing the shapes graph are two routes to ONE
        // verdict, which is the property a cache is only allowed to have.
        let via_product = product_validate_impl(&product, DATA).expect("validated via product");
        let via_document =
            validate_to_sarif_impl(SHAPES, None, DATA, None, &[], &[]).expect("validated directly");
        assert_eq!(via_product, via_document);

        // Rebuilding a CURRENT product reaches the byte-identical verdict too: the
        // forward-compatibility door must not be a second, divergent answer.
        let via_rebuild =
            product_validate_rebuild_impl(&product, DATA).expect("rebuilt via product");
        assert_eq!(via_rebuild, via_product);
    }

    #[test]
    fn a_refused_product_keeps_its_dimension_across_the_boundary() {
        let mut wrong_magic = pack_product_impl(SHAPES, None, &[], &[]).expect("product packed");
        wrong_magic[0] = b'X';

        let refusal = ShaclProductRefusal::from(
            product_validate_impl(&wrong_magic, DATA).expect_err("a foreign magic is refused"),
        );
        assert_eq!(refusal.dimension().as_deref(), Some("magic"));
        assert!(refusal.to_js_string().starts_with("magic: "));
        // The prose is carried WITHOUT the label doubled into it.
        assert!(!refusal.message().starts_with("magic: "));

        // A DATA graph that does not parse never reached the admission boundary, so
        // it truthfully names no dimension rather than borrowing one.
        let product = pack_product_impl(SHAPES, None, &[], &[]).expect("product packed");
        let data_refusal = ShaclProductRefusal::from(
            product_validate_impl(&product, "@@@ not n-triples").expect_err("refused"),
        );
        assert_eq!(data_refusal.dimension(), None);

        // The neighbouring VALID case still succeeds — a refusal is a claim too.
        product_validate_impl(&product, DATA).expect("the unmodified product still validates");
    }

    /// A second shapes graph over different classes, so the two products genuinely
    /// carry two input bindings.
    const OTHER_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        ex:WidgetShape a sh:NodeShape ;\n\
          sh:targetClass ex:Widget ;\n\
          sh:property [ sh:path ex:maker ; sh:minCount 1 ] .\n";

    /// The `identity-digest` a product renders — read the way a JavaScript host reads
    /// it, out of `shaclProductExplain`'s own text.
    fn rendered_selector(product: &[u8]) -> String {
        product_explain_impl(product)
            .expect("explained")
            .lines()
            .find_map(|line| line.strip_prefix("identity-digest ").map(ToOwned::to_owned))
            .expect("the rendering carries an identity digest")
    }

    #[test]
    fn a_product_that_is_not_the_expected_one_is_refused_across_the_boundary() {
        let held = pack_product_impl(SHAPES, None, &[], &[]).expect("product packed");
        let wanted =
            rendered_selector(&pack_product_impl(OTHER_SHAPES, None, &[], &[]).expect("packed"));
        assert_ne!(wanted, rendered_selector(&held));

        let refusal = product_validate_expecting_impl(&held, DATA, &wanted)
            .expect_err("the product held is not the product required");
        assert_eq!(refusal.dimension().as_deref(), Some("shapes-graph"));

        // A selector that is not a digest refuses with NO dimension: nothing was
        // opened, so naming one would blame the product for the host's argument.
        let mistyped = product_validate_expecting_impl(&held, DATA, "not-a-digest")
            .expect_err("a non-digest selector is refused");
        assert_eq!(mistyped.dimension(), None);

        // The gap this closes: unbound, the very same bytes validate.
        product_validate_impl(&held, DATA).expect("an unbound validation cannot ask which product");
    }

    #[test]
    fn a_product_required_to_be_itself_validates_identically() {
        let product = pack_product_impl(SHAPES, None, &[], &[]).expect("product packed");
        let own = rendered_selector(&product);

        let bound = product_validate_expecting_impl(&product, DATA, &own)
            .expect("a product required to be itself validates");
        let unbound = product_validate_impl(&product, DATA).expect("validated");
        assert_eq!(
            bound, unbound,
            "stating which product you meant changes the door, not the answer",
        );

        // The rendering is the accepted spelling, and case on the way in is not
        // significant — a selector that passed through a manifest or a CI variable
        // must not be turned away for a shape the mechanism does not care about.
        product_validate_expecting_impl(&product, DATA, &own.to_uppercase())
            .expect("an upper-case selector names the same product");
    }

    /// The rebuild path answers the same "is this the product the host meant?"
    /// question `product_validate_expecting_impl` does: a product whose binding
    /// is not the one required is refused on `shapes-graph` even though its
    /// stage id is one this guest knows and the unbound rebuild would otherwise
    /// happily re-derive it.
    #[test]
    fn a_rebuilt_product_that_is_not_the_expected_one_is_refused_across_the_boundary() {
        let held = pack_product_impl(SHAPES, None, &[], &[]).expect("product packed");
        let wanted =
            rendered_selector(&pack_product_impl(OTHER_SHAPES, None, &[], &[]).expect("packed"));
        assert_ne!(wanted, rendered_selector(&held));

        let refusal = product_validate_rebuild_expecting_impl(&held, DATA, &wanted)
            .expect_err("the product held is not the product required");
        assert_eq!(refusal.dimension().as_deref(), Some("shapes-graph"));

        // A selector that is not a digest refuses with NO dimension: nothing was
        // opened, so naming one would blame the product for the host's argument.
        let mistyped = product_validate_rebuild_expecting_impl(&held, DATA, "not-a-digest")
            .expect_err("a non-digest selector is refused");
        assert_eq!(mistyped.dimension(), None);

        // The gap this closes: the unbound rebuild restores the very same bytes,
        // because nothing in them states which product was meant.
        product_validate_rebuild_impl(&held, DATA)
            .expect("an unbound rebuild cannot ask which product was wanted");
    }

    /// The neighbouring VALID case: a product required to be ITSELF still
    /// rebuilds across the boundary, and reaches the byte-identical report the
    /// unbound rebuild and the bound admission-based validation both reach.
    #[test]
    fn a_rebuilt_product_required_to_be_itself_validates_identically() {
        let product = pack_product_impl(SHAPES, None, &[], &[]).expect("product packed");
        let own = rendered_selector(&product);

        let bound_rebuild = product_validate_rebuild_expecting_impl(&product, DATA, &own)
            .expect("a product required to be itself rebuilds");
        let unbound_rebuild =
            product_validate_rebuild_impl(&product, DATA).expect("the unbound rebuild validates");
        assert_eq!(
            bound_rebuild, unbound_rebuild,
            "stating which product you meant changes the door, not the answer",
        );

        let bound_admit = product_validate_expecting_impl(&product, DATA, &own)
            .expect("a product required to be itself admits");
        assert_eq!(
            bound_rebuild, bound_admit,
            "choosing to re-derive rather than restore the memo must not change the answer",
        );

        // The rendering is the accepted spelling, and case on the way in is not
        // significant — a selector that passed through a manifest or a CI
        // variable must not be turned away for a shape the mechanism does not
        // care about.
        product_validate_rebuild_expecting_impl(&product, DATA, &own.to_uppercase())
            .expect("an upper-case selector names the same product");
    }

    /// The shapes-graph tools' fixture: the W3C SHACL 1.2 declaration of
    /// `sh:SPARQLExprExpression` verbatim, a `sh:sparqlExpr` node naming `ex:yes` through
    /// `sh:prefixes` (`ex:Tag`), a labelled `shnex:var` node, a rule tagging every
    /// `ex:Item` through the same expression, and a counter rule stepping `ex:n` to 5 —
    /// exactly four term-generating rounds.
    const TOOLS_SHAPES: &str = r#"
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex: <http://example.org/ns#> .

sh:SPARQLExprExpression a sh:NamedParameterExpressionFunction ;
  rdfs:label "SPARQL expr expression"@en ;
  rdfs:comment "The class of node expressions based on SPARQL expressions (sh:sparqlExpr)."@en ;
  rdfs:isDefinedBy sh: ;
  rdfs:subClassOf sh:NamedParameterExpression,
  sh:SPARQLExecutable ;
  sh:parameter sh:SPARQLExprExpression-prefixes,
  sh:SPARQLExprExpression-sparqlExpr .

sh:SPARQLExprExpression-prefixes a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:description "The prefixes that shall be applied before parsing the SPARQL query that gets derived from the sh:sparqlExpr expression. The object should define those prefixes using sh:declare."@en ;
  sh:name "prefixes"@en ;
  sh:nodeKind sh:BlankNodeOrIRI ;
  sh:path sh:prefixes .

sh:SPARQLExprExpression-sparqlExpr a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:datatype xsd:string ;
  sh:description "The SPARQL expression that is executed during evaluation of this node expression."@en ;
  sh:keyParameter true ;
  sh:name "SPARQL expr"@en ;
  sh:path sh:sparqlExpr .

ex:Prefixes sh:declare [ sh:prefix "ex" ; sh:namespace "http://example.org/ns#"^^xsd:anyURI ] .
ex:Tag sh:sparqlExpr "ex:yes" ; sh:prefixes ex:Prefixes .
_:suffix shnex:var "suffix" .

ex:Tagger a sh:NodeShape ;
  sh:targetClass ex:Item ;
  sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:tagged ;
            sh:object [ sh:sparqlExpr "ex:yes" ; sh:prefixes ex:Prefixes ] ] .

ex:Counter a sh:NodeShape ;
  sh:targetSubjectsOf ex:n ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """PREFIX ex: <http://example.org/ns#>
CONSTRUCT { $this ex:n ?m } WHERE { $this ex:n ?k . FILTER(?k < 5) BIND(?k + 1 AS ?m) }""" ] .
"#;

    /// One `ex:Item` whose counter starts at 1.
    const TOOLS_DATA: &str = "<http://example.org/ns#a> \
        <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Item> .\n\
        <http://example.org/ns#a> <http://example.org/ns#n> \
        \"1\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n";

    /// The inference graph the fixture's rules produce, in canonical order.
    fn tools_inference() -> String {
        let mut out = String::new();
        for n in 2..=5 {
            out.push_str("<http://example.org/ns#a> <http://example.org/ns#n> \"");
            out.push_str(&n.to_string());
            out.push_str("\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n");
        }
        out.push_str(
            "<http://example.org/ns#a> <http://example.org/ns#tagged> \
             <http://example.org/ns#yes> .\n",
        );
        out
    }

    /// The rules entry point: the inference graph alone, the proof on request, the
    /// round limit refusing at 3 and completing at 4, and SPARQL 1.2 RL text.
    #[test]
    fn wasm_apply_rules() {
        let request = purrdf_validate::RulesRequest {
            data_nt: TOOLS_DATA,
            shapes_ttl: Some(TOOLS_SHAPES),
            ..purrdf_validate::RulesRequest::default()
        };
        let plain = apply_rules_impl(&request).expect("rules run");
        assert_eq!(plain.inferred_ntriples, tools_inference());
        assert_eq!(plain.proof, None);
        let explained = apply_rules_impl(&purrdf_validate::RulesRequest {
            explain: true,
            ..request
        })
        .expect("rules run");
        let proof = explained.proof.expect("a proof was asked for");
        assert_eq!(proof.matches("derived ").count(), 5, "{proof}");
        let refused = apply_rules_impl(&purrdf_validate::RulesRequest {
            max_term_generating_rounds: Some(3),
            ..request
        })
        .expect_err("three rounds are too few")
        .to_string();
        assert!(refused.contains("past the limit of 3"), "{refused}");
        let enough = apply_rules_impl(&purrdf_validate::RulesRequest {
            max_term_generating_rounds: Some(4),
            ..request
        })
        .expect("four rounds suffice");
        assert_eq!(enough.inferred_ntriples, tools_inference());
        let srl = apply_rules_impl(&purrdf_validate::RulesRequest {
            data_nt: TOOLS_DATA,
            srl: Some(
                "PREFIX ex: <http://example.org/ns#>\n\
                 RULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\nDATA { ex:d ex:q 2 }\n",
            ),
            explain: true,
            ..purrdf_validate::RulesRequest::default()
        })
        .expect("SPARQL 1.2 RL runs");
        assert_eq!(
            srl.inferred_ntriples,
            "<http://example.org/ns#a> <http://example.org/ns#q> \
             \"1\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n\
             <http://example.org/ns#d> <http://example.org/ns#q> \
             \"2\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n"
        );
        assert!(srl.proof.expect("proof").contains("  data-block\n"));
        assert!(
            apply_rules_impl(&purrdf_validate::RulesRequest {
                data_nt: TOOLS_DATA,
                ..purrdf_validate::RulesRequest::default()
            })
            .is_err(),
            "no rule source"
        );
    }

    /// The node-expression entry point: a `sh:sparqlExpr` node natively with its
    /// prefixes, a labelled blank node reading a `NAME=TERM` scope binding, and the
    /// refusals beside them.
    #[test]
    fn wasm_eval_node_expr() {
        assert_eq!(
            eval_node_expr_impl(
                TOOLS_SHAPES,
                None,
                TOOLS_DATA,
                ExprInputs::node("http://example.org/ns#Tag"),
                "http://example.org/ns#a",
                &[],
                &[],
                &[]
            ),
            Ok(vec!["<http://example.org/ns#yes>".to_owned()])
        );
        assert_eq!(
            eval_node_expr_impl(
                TOOLS_SHAPES,
                None,
                TOOLS_DATA,
                ExprInputs::node("_:suffix"),
                "http://example.org/ns#a",
                &["suffix=\"!\"@en".to_owned()],
                &[],
                &[]
            ),
            Ok(vec!["\"!\"@en".to_owned()])
        );
        let unknown = eval_node_expr_impl(
            TOOLS_SHAPES,
            None,
            TOOLS_DATA,
            ExprInputs::node("_:nosuch"),
            "http://example.org/ns#a",
            &[],
            &[],
            &[],
        )
        .expect_err("an unknown label")
        .to_string();
        assert!(
            unknown.contains("mentions no blank node _:nosuch"),
            "{unknown}"
        );
        let no_equals = eval_node_expr_impl(
            TOOLS_SHAPES,
            None,
            TOOLS_DATA,
            ExprInputs::node("_:suffix"),
            "http://example.org/ns#a",
            &["suffix".to_owned()],
            &[],
            &[],
        )
        .expect_err("a binding with no `=`")
        .to_string();
        assert!(no_equals.contains("not NAME=TERM"), "{no_equals}");
    }

    /// An anonymous expression named by a walk and inline as Turtle, each refusal beside
    /// a valid neighbour: a step reaching two values beside one reaching one, two roots
    /// beside one, and two selectors beside one.
    #[test]
    fn wasm_eval_node_expr_selectors() {
        const SH: &str = "http://www.w3.org/ns/shacl#";
        let eval = |expr: ExprInputs<'_>| {
            eval_node_expr_impl(
                TOOLS_SHAPES,
                None,
                TOOLS_DATA,
                expr,
                "http://example.org/ns#a",
                &[],
                &[],
                &[],
            )
            .map_err(|error| error.to_string())
        };
        let yes = Ok(vec!["<http://example.org/ns#yes>".to_owned()]);
        let tagger_walk = [format!("{SH}rule"), format!("{SH}object")];
        assert_eq!(
            eval(ExprInputs {
                at: Some("http://example.org/ns#Tagger"),
                via: &tagger_walk,
                ..ExprInputs::default()
            }),
            yes
        );
        let one = ["http://www.w3.org/2000/01/rdf-schema#isDefinedBy".to_owned()];
        let parameter = format!("{SH}SPARQLExprExpression");
        assert_eq!(
            eval(ExprInputs {
                at: Some(&parameter),
                via: &one,
                ..ExprInputs::default()
            }),
            Ok(vec![format!("<{SH}>")])
        );
        let two = [format!("{SH}parameter")];
        let refused = eval(ExprInputs {
            at: Some(&parameter),
            via: &two,
            ..ExprInputs::default()
        })
        .expect_err("two values");
        assert!(refused.contains("reaches 2 values"), "{refused}");

        assert_eq!(
            eval(ExprInputs {
                turtle: Some("[ sh:sparqlExpr \"ex:yes\" ; sh:prefixes ex:Prefixes ] ."),
                ..ExprInputs::default()
            }),
            yes
        );
        let roots = eval(ExprInputs {
            turtle: Some("[ shnex:var \"a\" ] . [ shnex:var \"b\" ] ."),
            ..ExprInputs::default()
        })
        .expect_err("two roots");
        assert!(roots.contains("has 2 root blank nodes"), "{roots}");
        let both = eval(ExprInputs {
            expr: Some("http://example.org/ns#Tag"),
            turtle: Some("[ shnex:var \"a\" ] ."),
            ..ExprInputs::default()
        })
        .expect_err("two selectors");
        assert!(both.contains("2 of the expression node"), "{both}");
    }

    /// The lint entry point: the fixture certifies clean with `sh:sparqlExpr`'s function
    /// bound natively; a malformed neighbour carries findings.
    #[test]
    fn wasm_lint_shapes() {
        let clean = ShaclLintReport {
            report: lint_shapes_impl(TOOLS_SHAPES, None, &[], &[]).expect("lint runs"),
        };
        assert!(clean.clean());
        assert_eq!(clean.findings(), 0);
        assert_eq!(clean.load_error(), None);
        assert!(
            clean.report().contains(
                "call native <http://www.w3.org/ns/shacl#SPARQLExprExpression> in sh:rule on \
                 <http://example.org/ns#Tagger>\n"
            ),
            "{}",
            clean.report()
        );
        let malformed = ShaclLintReport {
            report: lint_shapes_impl(
                &format!(
                    "{TOOLS_SHAPES}ex:Bad a sh:NodeShape ; \
                     sh:property [ sh:path ex:p ; sh:minCount \"one\" ] .\n"
                ),
                None,
                &[],
                &[],
            )
            .expect("lint runs"),
        };
        assert!(!malformed.clean());
        assert!(malformed.findings() >= 2, "{}", malformed.report());
        assert!(malformed.load_error().is_some());
        assert!(malformed.report().ends_with("clean false\n"));
        assert!(lint_shapes_impl("@@@ not turtle", None, &[], &[]).is_err());
    }
}
