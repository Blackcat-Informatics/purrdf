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

use purrdf_validate::ShapesProductRefusal;

/// Validate `data_nt` against `shapes_ttl` and render the report to SARIF 2.1.0.
///
/// Returns a plain `String` error (NOT a `JsError`) so it is unit-testable on the
/// native build — constructing a `JsError` calls a wasm-only import that panics
/// off wasm. The `#[wasm_bindgen]` wrapper maps the `String` to a `JsError`.
///
/// `conformance_disallows` is the request's conformance-disallow set as severity
/// IRIs; `None` is SHACL's default set, and an empty list or a non-IRI is an error.
pub(crate) fn validate_to_sarif_impl(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    conformance_disallows: Option<&[String]>,
) -> Result<String, String> {
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
    )
}

/// `shaclValidateToSarif(shapesTtl, dataNt, shapesBase?, conformanceDisallows?)` → a SARIF
/// 2.1.0 JSON string.
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
#[wasm_bindgen(js_name = shaclValidateToSarif)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn shacl_validate_to_sarif(
    shapes_ttl: &str,
    data_nt: &str,
    shapes_base: Option<String>,
    conformance_disallows: Option<Vec<String>>,
) -> Result<String, JsError> {
    validate_to_sarif_impl(
        shapes_ttl,
        shapes_base.as_deref(),
        data_nt,
        conformance_disallows.as_deref(),
    )
    .map_err(|e| JsError::new(&e))
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
) -> Result<(String, purrdf_validate::ChangeScope), String> {
    purrdf_validate::validate_changes_to_sarif_string(
        shapes_ttl,
        shapes_base,
        data_nt,
        added_nt,
        removed_nt,
        &purrdf_validate::SarifOptions::default(),
    )
}

/// `shaclValidateChangesToSarif(shapesTtl, dataNt, addedNt?, removedNt?, shapesBase?)`
/// → a `ShaclChangeValidation` carrying a SARIF 2.1.0 JSON string and its scope.
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
/// Throws (rejects) if the shapes graph or any of the three N-Triples documents
/// fails to parse. Call `.free()` on the returned object when done.
#[wasm_bindgen(js_name = shaclValidateChangesToSarif)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn shacl_validate_changes_to_sarif(
    shapes_ttl: &str,
    data_nt: &str,
    added_nt: Option<String>,
    removed_nt: Option<String>,
    shapes_base: Option<String>,
) -> Result<ShaclChangeValidation, JsError> {
    let (sarif, scope) = validate_changes_to_sarif_impl(
        shapes_ttl,
        shapes_base.as_deref(),
        data_nt,
        added_nt.as_deref(),
        removed_nt.as_deref(),
    )
    .map_err(|e| JsError::new(&e))?;
    Ok(ShaclChangeValidation { sarif, scope })
}

/// Entail `data_nt` under `shapes_ttl` and render the MATERIALIZED dataset (base
/// graph plus every SHACL-AF `sh:rule` inference) to canonical N-Triples.
///
/// The entailment twin of [`validate_to_sarif_impl`]: returns a plain `String`
/// error (NOT a `JsError`) so it is unit-testable on the native build; the
/// `#[wasm_bindgen]` wrapper maps the `String` to a `JsError`.
pub(crate) fn entail_to_ntriples_impl(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
) -> Result<String, String> {
    purrdf_validate::entail_to_ntriples_string(shapes_ttl, shapes_base, data_nt)
}

/// `shaclEntail(shapesTtl, dataNt, shapesBase?)` → the materialized dataset as an
/// N-Triples string (the base graph plus every inferred triple).
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
#[wasm_bindgen(js_name = shaclEntail)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn shacl_entail(
    shapes_ttl: &str,
    data_nt: &str,
    shapes_base: Option<String>,
) -> Result<String, JsError> {
    entail_to_ntriples_impl(shapes_ttl, shapes_base.as_deref(), data_nt)
        .map_err(|e| JsError::new(&e))
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
            message: refusal.message().to_owned(),
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
) -> Result<Vec<u8>, ShapesProductRefusal> {
    purrdf_validate::pack_shapes_product(shapes_ttl, shapes_base)
}

/// `shaclPackProduct(shapesTtl, shapesBase?)` → the prepared product as a `Uint8Array`.
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
/// Rejects with a [`ShaclProductRefusal`]; call `.free()` on it when done.
#[wasm_bindgen(js_name = shaclPackProduct)]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
pub fn shacl_pack_product(
    shapes_ttl: &str,
    shapes_base: Option<String>,
) -> Result<Vec<u8>, ShaclProductRefusal> {
    pack_product_impl(shapes_ttl, shapes_base.as_deref()).map_err(ShaclProductRefusal::from)
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
        let sarif = validate_to_sarif_impl(SHAPES, None, DATA, None).expect("sarif produced");
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
            let sarif =
                validate_to_sarif_impl(&shapes, None, DATA, disallows).expect("sarif produced");
            let log: serde_json::Value = serde_json::from_str(&sarif).expect("json");
            log["runs"][0]["properties"]["shaclConforms"].clone()
        };
        assert_eq!(conforms(None), serde_json::json!(false));
        let violation = ["http://www.w3.org/ns/shacl#Violation".to_owned()];
        assert_eq!(conforms(Some(&violation)), serde_json::json!(true));
        assert!(validate_to_sarif_impl(&shapes, None, DATA, Some(&[])).is_err());
        assert!(
            validate_to_sarif_impl(&shapes, None, DATA, Some(&["Violation".to_owned()])).is_err()
        );
    }

    #[test]
    fn malformed_shapes_is_an_error() {
        assert!(validate_to_sarif_impl("@@@ not turtle", None, DATA, None).is_err());
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
        let (sarif, scope) =
            validate_changes_to_sarif_impl(SHAPES, None, CHANGE_BASE, Some(BAD_AGE), None)
                .expect("the change validates");
        assert_eq!(scope.focus_nodes(), Some(1));
        assert!(scope.is_bounded());
        assert_eq!(
            sarif,
            validate_to_sarif_impl(SHAPES, None, &format!("{CHANGE_BASE}{BAD_AGE}"), None)
                .expect("full validation"),
        );

        // The retract half is a real half: taking the row back out restores
        // conformance through the same one call.
        let merged = format!("{CHANGE_BASE}{BAD_AGE}");
        let (sarif, scope) =
            validate_changes_to_sarif_impl(SHAPES, None, &merged, None, Some(BAD_AGE))
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
        let (sarif, scope) =
            validate_changes_to_sarif_impl(SPARQL_SHAPES, None, CHANGE_BASE, Some(BAD_AGE), None)
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
        let (sarif, scope) =
            validate_changes_to_sarif_impl(SHAPES, None, CHANGE_BASE, Some(BAD_AGE), None)
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
            )
            .is_err()
        );
        // The neighbouring VALID case still succeeds — a refusal is a claim too.
        validate_changes_to_sarif_impl(SHAPES, None, CHANGE_BASE, Some(BAD_AGE), None)
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
        let nt =
            entail_to_ntriples_impl(RULE_SHAPES, None, RULE_DATA).expect("entailment produced");
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
        assert!(entail_to_ntriples_impl("@@@ not turtle", None, RULE_DATA).is_err());
    }

    #[test]
    fn a_product_round_trips_and_reaches_the_same_verdict() {
        let product = pack_product_impl(SHAPES, None).expect("product packed");
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
            validate_to_sarif_impl(SHAPES, None, DATA, None).expect("validated directly");
        assert_eq!(via_product, via_document);

        // Rebuilding a CURRENT product reaches the byte-identical verdict too: the
        // forward-compatibility door must not be a second, divergent answer.
        let via_rebuild =
            product_validate_rebuild_impl(&product, DATA).expect("rebuilt via product");
        assert_eq!(via_rebuild, via_product);
    }

    #[test]
    fn a_refused_product_keeps_its_dimension_across_the_boundary() {
        let mut wrong_magic = pack_product_impl(SHAPES, None).expect("product packed");
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
        let product = pack_product_impl(SHAPES, None).expect("product packed");
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
        let held = pack_product_impl(SHAPES, None).expect("product packed");
        let wanted = rendered_selector(&pack_product_impl(OTHER_SHAPES, None).expect("packed"));
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
        let product = pack_product_impl(SHAPES, None).expect("product packed");
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
        let held = pack_product_impl(SHAPES, None).expect("product packed");
        let wanted = rendered_selector(&pack_product_impl(OTHER_SHAPES, None).expect("packed"));
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
        let product = pack_product_impl(SHAPES, None).expect("product packed");
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
}
