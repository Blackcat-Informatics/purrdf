// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf_shacl_validate_to_sarif`: validate a data graph against a shapes graph
//! and return a SARIF 2.1.0 report — plus the prepared-shapes-product codec.
//!
//! The C-ABI counterpart of the Python/WASM `to_sarif` surface. It drives the
//! SHACL engine and its SARIF reporting boundary, writing the report bytes into
//! the shared [`PurrdfBuffer`].
//!
//! # Validating a CHANGE rather than a graph
//!
//! `purrdf_shacl_validate_changes_to_sarif` is the incremental twin: hand it both
//! halves of a delta — the rows joining the data graph and the rows leaving it —
//! and the engine expands the change into the focus nodes it can move and
//! re-validates exactly those. A host that edits a graph and asks *what did my
//! last change break?* pays for the change rather than for the graph.
//!
//! It returns the scope beside the report, and that is not decoration. A bounded
//! run reports about the affected focus nodes, so an empty log means "this change
//! introduced no violation"; a shapes graph whose constraints read through SPARQL
//! query text has no bounded footprint, so the run falls back to validating the
//! whole mutated graph and an empty log means "the graph conforms". The fallback
//! is not optional — a short expansion and a clean bill of health are
//! indistinguishable in a report — and a caller that cannot tell the two readings
//! apart has been handed the weaker one believing it is the stronger.
//!
//! # The shapes-graph tools
//!
//! `purrdf_shacl_apply_rules` runs a SHACL shapes graph's rules, or a SPARQL 1.2 RL
//! rule set, and returns the inference graph (and, on request, its proof);
//! `purrdf_shacl_eval_node_expr` evaluates one node expression of a shapes graph;
//! `purrdf_shacl_lint_shapes` certifies a shapes graph cold. Each is the C framing of
//! one `purrdf_validate` function the Python and WASM bindings call too.
//!
//! # The shapes graph's `owl:imports`
//!
//! Every entry point that takes a Turtle shapes graph takes the caller's `owl:imports`
//! table as three trailing inputs, `import_iris` / `import_documents` / `import_count` —
//! the parallel-array convention `purrdf_entail_certain_answers` uses — each document
//! Turtle parsed with its ontology IRI as its base. `import_count == 0` (the arrays may
//! then be NULL) is the ordinary empty table, and the rule still applies: an import
//! nothing in hand resolves — or a table entry nothing imports — fails the call with
//! [`PurrdfStatus::ShapesImportError`], the same refusal the Rust, command-line, Python
//! and WebAssembly hosts raise, whose kind and IRIs are read with
//! [`purrdf_shapes_import_error_kind`], [`purrdf_shapes_import_error_iri_count`] and
//! [`purrdf_shapes_import_error_iri`]. PurRDF fetches nothing.
//!
//! # Prepared products, and the one thing this ABI cannot carry
//!
//! `purrdf_shapes_product_encode` compiles a shapes graph once into a
//! digest-chained container; `purrdf_shapes_product_admit` restores it instead of
//! re-parsing. Those bytes are UNTRUSTED when they come back, so restoring one is an
//! admission: the framing, every section digest, the whole-container digest, then the
//! product's stage id, profile and complete input binding are checked before any of
//! it reaches a validator.
//!
//! `purrdf_shapes_product_rebuild` is the forward-compatibility path: a product
//! whose stage id this build does not know refuses `purrdf_shapes_product_admit`
//! with the dimension `stage-id`, and rebuilding re-derives the preparation from
//! the shapes dataset the product carries instead — no RDF text is parsed and no
//! file is read.
//!
//! **A refusal is not flattened to a string here.** The codec refuses on a closed set
//! of named dimensions, and collapsing them would put "these bytes are corrupt", "this
//! product is from another build" and "your configuration differs from the one it was
//! prepared against" into one bucket, when they are three different actions. So a
//! refusal returns [`PurrdfStatus::ShapesProductError`] and its [`PurrdfError`]
//! carries the pinned kebab-case label, read with
//! [`purrdf_shapes_product_error_dimension`].
//!
//! The limitation, stated here rather than hidden: these entry points restore a
//! product against the EMPTY host bindings. A C host cannot pass a Rust
//! `UserFunctionRegistry`, an `AggregateRegistry` or a `PropertyFunctionRegistry`
//! across this boundary — those are Rust closure tables and this ABI has no
//! representation for them — so this surface serves exactly the products
//! `ShapesProfile::CORE` is defined around, whose every capability is declared by the
//! shapes graph itself. A product prepared against host-injected registries is not
//! silently validated under empty ones: its identity binds them, so admission REFUSES
//! it on `function-registry`, `aggregate-registry` or `property-function-registry`,
//! and the caller learns which. Rust is the surface for those products.

use std::os::raw::c_char;

use purrdf_validate::{
    ChangeScope, ConformanceDisallows, EntailOutcome, EntailRequest, ExprSelector, LintReport,
    NodeExprRequest, RuleLimits, RulesOutcome, RulesRequest, SarifOptions, ShapesError,
    ShapesProductRefusal, ValidationOptions, apply_rules_to_ntriples, check_rules,
    entail_to_ntriples, eval_node_expr, lint_shapes_ttl_with_shapes_graph, parse_scope_binding,
    validate_changes_to_sarif_string_with_shapes_graph, validate_to_sarif_string_with_shapes_graph,
};

use crate::buffer::PurrdfBuffer;
use crate::entail::import_pairs;
use crate::error::PurrdfError;
use crate::status::PurrdfStatus;
use crate::{cstr_to_str, opt_cstr_to_str};

/// Validate `data_nt` (N-Triples) against `shapes_ttl` (Turtle) and render the
/// report to SARIF 2.1.0 bytes. Native-testable, pointer-free core.
///
/// The validate→SARIF sequence lives in [`validate_to_sarif_string`]; this only
/// adds the C-ABI byte framing.
///
/// `conformance_disallows` is the request's conformance-disallow set as severity
/// IRIs; empty is SHACL's default set. `subclass_of_in_shapes_graph` is SHACL 1.2 Core
/// §6.3's `subClassOfInShapesGraph`.
fn validate_to_sarif_bytes(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    shapes_graph: Option<&str>,
    data_nt: &str,
    conformance_disallows: &[&str],
    imports: &[(&str, &str)],
    subclass_of_in_shapes_graph: bool,
) -> Result<Vec<u8>, ShapesError> {
    let options = sarif_options(conformance_disallows, subclass_of_in_shapes_graph)?;
    Ok(validate_to_sarif_string_with_shapes_graph(
        shapes_ttl,
        shapes_base,
        shapes_graph,
        data_nt,
        &options,
        imports,
    )?
    .into_bytes())
}

/// The SARIF options both validation entry points — the whole-graph and the change path —
/// read their two validation parameters into, so the two cannot come to mean different
/// things by the same arguments. `conformance_disallows` is the conformance-disallow set as
/// severity IRIs, empty being SHACL's default set; `subclass_of_in_shapes_graph` is SHACL
/// 1.2 Core §6.3's `subClassOfInShapesGraph`.
fn sarif_options(
    conformance_disallows: &[&str],
    subclass_of_in_shapes_graph: bool,
) -> Result<SarifOptions, ShapesError> {
    let base_options =
        ValidationOptions::default().with_subclass_of_in_shapes_graph(subclass_of_in_shapes_graph);
    let validation = if conformance_disallows.is_empty() {
        base_options
    } else {
        base_options
            .with_conformance_disallows(ConformanceDisallows::from_iris(conformance_disallows)?)
    };
    Ok(SarifOptions {
        validation,
        ..SarifOptions::default()
    })
}

/// The `count` C strings at `array`, borrowed.
///
/// `count == 0` is accepted with a NULL array — there is nothing to dereference.
/// A NULL array with a non-zero count, or a NULL element, is refused before any
/// dereference.
///
/// # Safety
/// When `count` is non-zero, `array` must address at least `count` readable
/// `*const c_char`, each null (refused here) or a NUL-terminated C string that
/// outlives the returned borrows.
unsafe fn cstr_array<'a>(
    array: *const *const c_char,
    count: usize,
    entry: &str,
) -> Result<Vec<&'a str>, PurrdfError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if array.is_null() {
        return Err(PurrdfError::new(
            PurrdfStatus::NullPointer,
            format!("null array with a non-zero count ({count}) passed to {entry}"),
        ));
    }
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        // SAFETY: the caller's contract above — the array is non-null (checked) and
        // holds at least `count` readable elements, so `index < count` is in bounds.
        // Each element is handed to `cstr_to_str`, which refuses a null pointer rather
        // than dereferencing it.
        out.push(unsafe { cstr_to_str(*array.add(index))? });
    }
    Ok(out)
}

/// Validate a data graph (N-Triples) against a shapes graph (Turtle) and write
/// the SARIF 2.1.0 report bytes to `*out_buffer` (free with `purrdf_buffer_free`).
///
/// `shapes_base_iri` is the base IRI the SHAPES document's relative IRI references
/// resolve against, and may be NULL. It is a real parameter and is read: a C host was
/// handed a string and has no retrieval IRI, so PurRDF will not invent one, and NULL
/// leaves a relative reference a hard `iri-relative-no-base` rather than a silent
/// mis-parse. `data_nt` needs no counterpart — N-Triples admits no relative IRI by
/// grammar, so a base there could only be ignored.
///
/// `conformance_disallows` / `conformance_disallows_count` name the
/// conformance-disallow set: the severity IRIs whose results make the data
/// non-conforming — the report's verdict and every nested `sh:node` / `sh:not` /
/// `sh:and` / `sh:or` / `sh:xone` check alike. `count == 0` (the array may then be
/// NULL) is SHACL's default set, `sh:Violation`, `sh:Warning` and `sh:Info`; a
/// value that is not an absolute IRI is a `ParseError`. The SARIF run carries
/// `properties.shaclConforms`, `properties.shaclConformanceDisallows` and
/// `properties.shaclShapesGraphWellFormed` (the report's `sh:shapesGraphWellFormed`), because the
/// results alone cannot say whether the data conforms: an `sh:Debug` / `sh:Trace`
/// result (SARIF `kind` `informational`, `level` `none`) appears in the log of a
/// conforming report. A result's `message.text` is its untagged `sh:resultMessage`
/// when it has one, else the first in canonical order; whenever that text alone
/// would lose something (several messages, a language tag, a direction, an
/// `rdf:HTML` message) the result's `properties.shaclMessages` lists every message
/// as `{"text", "language"?, "direction"?, "datatype"?}`.
///
/// `import_iris` / `import_documents` / `import_count` are the shapes graph's
/// `owl:imports` table: entry `i` declares that `import_iris[i]` names the Turtle document
/// `import_documents[i]`, parsed with that IRI as its base. `import_count == 0` (the
/// arrays may then be NULL) is the empty table. An `owl:imports` is an import only on the
/// shapes graph's own IRI (`shapes_base_iri`, or the document's own `@base`), on an
/// `owl:Ontology` header, on a `sh:ShapesGraph` (`sh:RulesGraph` and subclasses included),
/// or on a node naming one of those as its `owl:versionIRI`; on any other node — one that is
/// only a `sh:DataGraph` among them — it is data. An import is resolved by a table entry, by
/// `shapes_base_iri` (or the document's own `@base`) naming the imported document, or by the
/// closure declaring it (`<X> a owl:Ontology`, `<X> a sh:ShapesGraph`, or an ontology whose
/// `owl:versionIRI` is `<X>`); anything else — or a table entry
/// nothing imports — returns `PURRDF_STATUS_SHAPES_IMPORT_ERROR` rather than a report about a
/// smaller shapes graph than the one named. Read its kind and IRIs with
/// `purrdf_shapes_import_error_kind` / `_iri_count` / `_iri`.
///
/// `shapes_graph_iri` is the IRI SHACL-SPARQL sees the shapes graph under, and may be
/// NULL — `purrdf validate --shapes-graph`. `$shapesGraph` is pre-bound to it and `GRAPH
/// $shapesGraph { … }` reads the shapes graph: SHACL 1.0's pre-binding, which SHACL 1.2
/// removed. NULL names no graph, and `$shapesGraph` is then an ordinary variable. A
/// relative IRI resolves against `shapes_base_iri`; one with no base is a `ParseError`
/// (`iri-relative-no-base`).
///
/// `subclass_of_in_shapes_graph` is SHACL 1.2 Core §6.3's `subClassOfInShapesGraph`:
/// `true` reads the shapes graph's `rdfs:subClassOf` triples, in addition to the data
/// graph's, wherever SHACL type decides class membership (`sh:targetClass`, implicit class
/// targets, `sh:class`, `sh:rootClass`, `shnex:instancesOf`); `false`, the specification's
/// default, reads the data graph alone. Only class membership changes: `rdf:type` triples
/// are always read from the data graph.
///
/// # Safety
/// `shapes_ttl` and `data_nt` must be non-null, NUL-terminated C strings;
/// `shapes_base_iri` and `shapes_graph_iri` must each be null or a NUL-terminated C
/// string; when `conformance_disallows_count` is non-zero, `conformance_disallows` must address
/// that many NUL-terminated C strings; when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings; `out_buffer` must be a writable
/// pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_validate_to_sarif(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    shapes_graph_iri: *const c_char,
    data_nt: *const c_char,
    conformance_disallows: *const *const c_char,
    conformance_disallows_count: usize,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    subclass_of_in_shapes_graph: bool,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if shapes_ttl.is_null() || data_nt.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shacl_validate_to_sarif",
                ));
            }
            let shapes = cstr_to_str(shapes_ttl)?;
            let base = opt_cstr_to_str(shapes_base_iri)?;
            let data = cstr_to_str(data_nt)?;
            let disallows = cstr_array(
                conformance_disallows,
                conformance_disallows_count,
                "purrdf_shacl_validate_to_sarif",
            )?;
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_shacl_validate_to_sarif",
            )?;
            let graph = opt_cstr_to_str(shapes_graph_iri)?;
            let bytes = validate_to_sarif_bytes(
                shapes,
                base,
                graph,
                data,
                &disallows,
                &imports,
                subclass_of_in_shapes_graph,
            )
            .map_err(PurrdfError::shapes)?;
            *out_buffer = PurrdfBuffer::into_raw(bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Which question a change-path report answered, written to
/// `purrdf_shacl_validate_changes_to_sarif`'s `out_scope`.
///
/// Append-only, like every other discriminant this ABI exports: never renumber a
/// variant. It is carried as an `int32_t` out-parameter rather than as this enum
/// type so a C caller writing an out-of-range value cannot produce an invalid
/// discriminant, exactly as the governed query/update outcomes are carried.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurrdfShaclChangeScopeKind {
    /// The change's footprint was bounded: the report covers the affected focus
    /// nodes, and `conforms` means THIS CHANGE introduced no violation.
    Bounded = 0,
    /// No bounded footprint exists for this shapes graph: the report covers the
    /// whole mutated graph, and `conforms` means THE GRAPH conforms.
    Everything = 1,
}

/// Validate a CHANGE to `data_nt` against `shapes_ttl` and render the report to
/// SARIF 2.1.0 bytes, beside the scope it describes. Native-testable,
/// pointer-free core.
///
/// The expand-then-validate sequence lives in
/// [`validate_changes_to_sarif_string`]; this only adds the C-ABI byte framing.
#[allow(
    clippy::too_many_arguments,
    reason = "the change path's three documents, the shapes document's two IRIs, the two \
              validation parameters and the import table are each an independent input"
)]
fn validate_changes_to_sarif_bytes(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    shapes_graph: Option<&str>,
    data_nt: &str,
    added_nt: Option<&str>,
    removed_nt: Option<&str>,
    conformance_disallows: &[&str],
    imports: &[(&str, &str)],
    subclass_of_in_shapes_graph: bool,
) -> Result<(Vec<u8>, ChangeScope), ShapesError> {
    let (sarif, scope) = validate_changes_to_sarif_string_with_shapes_graph(
        shapes_ttl,
        shapes_base,
        shapes_graph,
        data_nt,
        added_nt,
        removed_nt,
        &sarif_options(conformance_disallows, subclass_of_in_shapes_graph)?,
        imports,
    )?;
    Ok((sarif.into_bytes(), scope))
}

/// Validate a CHANGE to a data graph (N-Triples) against a shapes graph (Turtle),
/// writing the SARIF 2.1.0 report bytes to `*out_buffer` and the scope that report
/// describes to `*out_scope`, `*out_focus_nodes` and `*out_reason`.
///
/// The incremental twin of `purrdf_shacl_validate_to_sarif`. `added_nt` and
/// `removed_nt` are the two halves of the delta — rows joining and rows leaving
/// `data_nt` — and each may be NULL for "nothing on this half". Both halves are
/// real: a verdict moves when a row leaves the graph as readily as when one joins,
/// and one parameter would be half a delta. Additions apply before removals, so a
/// change set naming the same row on both halves settles on *removed*. A removal
/// naming a row `data_nt` does not carry retracts nothing rather than failing: a
/// change set describes what moved, it does not assert what the base contained.
///
/// `shapes_base_iri` carries the same meaning it does on
/// `purrdf_shacl_validate_to_sarif` — the shapes document's own base IRI, nullable.
/// The three N-Triples documents need no counterpart; N-Triples admits no relative
/// IRI by grammar.
///
/// `import_iris` / `import_documents` / `import_count` are the shapes graph's
/// `owl:imports` table (see `purrdf_shacl_validate_to_sarif`).
///
/// `conformance_disallows` / `conformance_disallows_count` and
/// `subclass_of_in_shapes_graph` carry exactly the meaning they do on
/// `purrdf_shacl_validate_to_sarif`: the severity IRIs whose results make the data
/// non-conforming (`count == 0`, the array then possibly NULL, is SHACL's default set;
/// a value that is not an absolute IRI is a `ParseError`), and SHACL 1.2 Core §6.3's
/// `subClassOfInShapesGraph` (`false` is the specification's default). The report of a
/// change is judged exactly as the report of the whole graph would be.
///
/// # Read the scope before the report
///
/// `*out_scope` is a `PurrdfShaclChangeScopeKind` and it decides what the SARIF log
/// MEANS. On `PURRDF_SHACL_CHANGE_SCOPE_KIND_BOUNDED` the log covers the focus
/// nodes the change could move — for those nodes it is identical, results and
/// ordering alike, to a full validation of the mutated graph — and is silent about
/// a pre-existing violation the change cannot reach, so an empty log means *this
/// change introduced no violation*. On
/// `PURRDF_SHACL_CHANGE_SCOPE_KIND_EVERYTHING` the shapes graph reads through
/// SPARQL query text, no bounded footprint exists for it, the call fell back to a
/// FULL validation of the mutated graph, and an empty log means *the graph
/// conforms*. The fallback is not optional, and a caller that cannot tell the two
/// apart has been handed the more dangerous of the two readings.
///
/// `*out_focus_nodes` is how many focus nodes a bounded expansion named. It is
/// written `0` on the `EVERYTHING` arm, where it is NOT a node count: "every focus
/// node in the graph" is not a number, so branch on the kind, never on this.
///
/// `*out_reason` is a `PurrdfBuffer` of UTF-8 prose naming the construct that made
/// the footprint unbounded — actionable rather than decorative, because it names
/// what to change to get incremental validation back. It is NULL — never an empty
/// buffer — on the `BOUNDED` arm. Free a non-NULL one with `purrdf_buffer_free`,
/// exactly as `*out_buffer` is freed.
///
/// `shapes_graph_iri` carries the meaning it does on `purrdf_shacl_validate_to_sarif`.
///
/// # Safety
/// `shapes_ttl` and `data_nt` must be non-null, NUL-terminated C strings;
/// `shapes_base_iri`, `shapes_graph_iri`, `added_nt` and `removed_nt` must be null or
/// NUL-terminated C strings; when `conformance_disallows_count` is non-zero,
/// `conformance_disallows` must address that many NUL-terminated C strings; when
/// `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings; `out_buffer`, `out_scope`, `out_focus_nodes` and
/// `out_reason` must be writable pointers; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_validate_changes_to_sarif(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    shapes_graph_iri: *const c_char,
    data_nt: *const c_char,
    added_nt: *const c_char,
    removed_nt: *const c_char,
    conformance_disallows: *const *const c_char,
    conformance_disallows_count: usize,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    subclass_of_in_shapes_graph: bool,
    out_buffer: *mut *mut PurrdfBuffer,
    out_scope: *mut i32,
    out_focus_nodes: *mut usize,
    out_reason: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if shapes_ttl.is_null()
                || data_nt.is_null()
                || out_buffer.is_null()
                || out_scope.is_null()
                || out_focus_nodes.is_null()
                || out_reason.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shacl_validate_changes_to_sarif",
                ));
            }
            let shapes = cstr_to_str(shapes_ttl)?;
            let base = opt_cstr_to_str(shapes_base_iri)?;
            let data = cstr_to_str(data_nt)?;
            let added = opt_cstr_to_str(added_nt)?;
            let removed = opt_cstr_to_str(removed_nt)?;
            let disallows = cstr_array(
                conformance_disallows,
                conformance_disallows_count,
                "purrdf_shacl_validate_changes_to_sarif",
            )?;
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_shacl_validate_changes_to_sarif",
            )?;
            let (bytes, scope) = validate_changes_to_sarif_bytes(
                shapes,
                base,
                opt_cstr_to_str(shapes_graph_iri)?,
                data,
                added,
                removed,
                &disallows,
                &imports,
                subclass_of_in_shapes_graph,
            )
            .map_err(PurrdfError::shapes)?;
            // Written before the buffer so a caller reading the outputs in
            // declaration order never sees a report without the scope it is about.
            match scope {
                ChangeScope::Bounded { focus_nodes } => {
                    *out_scope = PurrdfShaclChangeScopeKind::Bounded as i32;
                    *out_focus_nodes = focus_nodes;
                    *out_reason = std::ptr::null_mut();
                }
                ChangeScope::Everything { reason } => {
                    *out_scope = PurrdfShaclChangeScopeKind::Everything as i32;
                    *out_focus_nodes = 0;
                    *out_reason = PurrdfBuffer::into_raw(reason.as_bytes().to_vec());
                }
            }
            *out_buffer = PurrdfBuffer::into_raw(bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Entail `data_nt` (N-Triples) under `shapes_ttl` (Turtle): the materialized dataset
/// (base graph plus every SHACL-AF rule inference) as canonical N-Triples, and the shapes
/// graph's mandatory diagnostics. Native-testable, pointer-free core.
///
/// The parse→entail→serialize sequence lives in [`entail_to_ntriples`]; the exported
/// symbol only adds the C-ABI buffer framing.
fn entail_outcome(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    shapes_graph: Option<&str>,
    data_nt: &str,
    imports: &[(&str, &str)],
    limits: RuleLimits,
) -> Result<EntailOutcome, ShapesError> {
    entail_to_ntriples(&EntailRequest {
        shapes_ttl,
        shapes_base,
        shapes_graph,
        data_nt,
        imports,
        max_term_generating_rounds: limits.max_term_generating_rounds,
        max_generated_terms: limits.max_generated_terms,
        max_stored_facts: limits.max_stored_facts,
        max_join_steps: limits.max_join_steps,
        host: purrdf_validate::RulesHost::CAbi,
    })
}

/// The shapes graph's mandatory diagnostics as the text every host renders: one
/// `diagnostic RULE SHAPE` line per shape with an empty `sh:in` or `sh:xone` list, in the
/// engine's order — the lines `purrdf_shacl_lint_shapes`' report carries.
fn diagnostic_text(diagnostics: &[purrdf_validate::MandatoryDiagnostic]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for diagnostic in diagnostics {
        let _ = writeln!(out, "diagnostic {diagnostic}");
    }
    out
}

/// Entail a data graph (N-Triples) under a shapes graph (Turtle) and write the
/// materialized dataset (base graph plus every inferred triple) as canonical
/// N-Triples bytes to `*out_buffer` (free with `purrdf_buffer_free`).
///
/// `shapes_base_iri` carries the same meaning it does on
/// `purrdf_shacl_validate_to_sarif`: the shapes document's own base IRI, nullable,
/// and read rather than accepted-and-dropped. `shapes_graph_iri` is the nullable
/// shapes-graph IRI the SHACL rules see the shapes graph under, as on
/// `purrdf_shacl_apply_rules`: a `sh:SPARQLRule`'s `$shapesGraph` is pre-bound to it; NULL
/// leaves `$shapesGraph` an ordinary variable.
///
/// Nothing is dropped on the way out: the underlying writer is the graph-carrying
/// canonical N-Quads serializer, and the output is N-Triples because BOTH inputs
/// are single-graph syntaxes, not because a graph slot was discarded.
///
/// `import_iris` / `import_documents` / `import_count` are the shapes graph's
/// `owl:imports` table (see `purrdf_shacl_validate_to_sarif`). An imported document's rules
/// run.
///
/// `max_term_generating_rounds`, `max_generated_terms`, `max_stored_facts` and
/// `max_join_steps` are the four rule-evaluation limits, exactly as
/// `purrdf_shacl_apply_rules` takes them and with the same defaults: each may be NULL for
/// the engine or target default — 16384 term-generating rounds, max(65536, 4 × N)
/// generated terms for N distinct input terms, 4194304 stored facts and 1048576 join steps
/// natively — or point at an exact limit. A run past one fails the call naming the limit,
/// the numbers and the parameter that raises it
/// (`purrdf_shacl_entail_to_ntriples's max_stored_facts`, …).
///
/// `out_diagnostics` asks for the shapes graph's mandatory diagnostics: NULL skips them;
/// non-NULL receives a buffer (free with `purrdf_buffer_free`) of one `diagnostic RULE
/// SHAPE` line per shape with an empty `sh:in` or `sh:xone` list — empty when there is none
/// — which every run reports beside its outcome.
/// Each line is `diagnostic `, the RULE (`in-minListLength` or `xone-minListLength`), one
/// space, then the SHAPE as an N-Triples term, then `\n` — the rule first and the shape
/// second, the field order every host's structured value (`{rule, shape}`) carries.
///
/// # Safety
/// `shapes_ttl` and `data_nt` must be non-null, NUL-terminated C strings;
/// `shapes_base_iri` and `shapes_graph_iri` must each be null or a NUL-terminated C string;
/// when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings; `max_term_generating_rounds`,
/// `max_generated_terms`, `max_stored_facts` and `max_join_steps` must each be null or
/// readable; `out_buffer` must be a writable pointer; `out_diagnostics` and `out_error`
/// must each be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_entail_to_ntriples(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    shapes_graph_iri: *const c_char,
    data_nt: *const c_char,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    max_term_generating_rounds: *const u64,
    max_generated_terms: *const u64,
    max_stored_facts: *const u64,
    max_join_steps: *const u64,
    out_buffer: *mut *mut PurrdfBuffer,
    out_diagnostics: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if shapes_ttl.is_null() || data_nt.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shacl_entail_to_ntriples",
                ));
            }
            let shapes = cstr_to_str(shapes_ttl)?;
            let base = opt_cstr_to_str(shapes_base_iri)?;
            let shapes_graph = opt_cstr_to_str(shapes_graph_iri)?;
            let data = cstr_to_str(data_nt)?;
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_shacl_entail_to_ntriples",
            )?;
            let limits = RuleLimits {
                // SAFETY: the caller's contract — null or readable.
                max_term_generating_rounds: max_term_generating_rounds.as_ref().copied(),
                // SAFETY: the caller's contract — null or readable.
                max_generated_terms: max_generated_terms.as_ref().copied(),
                // SAFETY: the caller's contract — null or readable.
                max_stored_facts: max_stored_facts.as_ref().copied(),
                // SAFETY: the caller's contract — null or readable.
                max_join_steps: max_join_steps.as_ref().copied(),
            };
            let outcome = entail_outcome(shapes, base, shapes_graph, data, &imports, limits)
                .map_err(PurrdfError::shapes)?;
            if !out_diagnostics.is_null() {
                *out_diagnostics =
                    PurrdfBuffer::into_raw(diagnostic_text(&outcome.diagnostics).into_bytes());
            }
            *out_buffer = PurrdfBuffer::into_raw(outcome.ntriples.into_bytes());
            Ok(PurrdfStatus::Ok)
        })
    }
}

// ---------------------------------------------------------------------------
// Shapes-graph tools: rules, node expressions, lint
// ---------------------------------------------------------------------------

/// Run a rule set over a data graph. Native-testable, pointer-free core of
/// [`purrdf_shacl_apply_rules`]: the work is [`apply_rules_to_ntriples`].
fn apply_rules_outcome(request: &RulesRequest<'_>) -> Result<RulesOutcome, ShapesError> {
    apply_rules_to_ntriples(request)
}

/// Run a rule set over a data graph (N-Triples) and write the INFERENCE GRAPH — the
/// inferred triples only, never the data graph — as N-Triples 1.2 bytes, one triple per
/// line in canonical order, to `*out_inferred` (free with `purrdf_buffer_free`).
///
/// The rule source is exactly one of `shapes_ttl` — a SHACL shapes graph (Turtle), whose
/// default rule set runs — and `srl`, a SPARQL 1.2 RL rule set; both NULL, or both
/// non-NULL, is a `ParseError`. `shapes_base_iri` / `srl_base_iri` are the documents' base
/// IRIs and may be NULL (a C host has no retrieval IRI, so PurRDF invents none).
///
/// `shapes_graph_iri` is the nullable shapes-graph IRI the SHACL rules see the shapes graph
/// under, as `purrdf rules --shapes-graph` names it: a `sh:SPARQLRule`'s `$shapesGraph` is
/// pre-bound to it and `GRAPH $shapesGraph { … }` reads the shapes graph. A relative one
/// resolves against `shapes_base_iri`; NULL leaves `$shapesGraph` an ordinary variable.
/// Non-NULL beside `srl` is a `ParseError`: a SPARQL 1.2 RL rule set has no shapes graph.
///
/// `max_term_generating_rounds` bounds the evaluation rounds that infer a term the graph
/// did not hold, and `max_generated_terms` the terms inferred beyond the input's. Each may
/// be NULL for the engine default — 16384 rounds, and max(65536, 4 × N) terms for N
/// distinct input terms — or point at an exact limit. A run past either fails the call
/// naming the limit, the numbers, the rules that inferred a new term last, and the
/// parameter that raises it.
///
/// `max_stored_facts` bounds the facts the evaluation store may hold — the data graph, a
/// rule set's data and every inferred triple — and `max_join_steps` the candidate
/// solutions the rule bodies may enumerate. Each may be NULL for the target's default —
/// 4194304 facts and 1048576 join steps natively — or point at an exact limit. A run
/// past either fails the call naming the limit, the numbers and the parameter that raises
/// it.
///
/// `out_proof` asks for the proof: NULL skips it; non-NULL receives a buffer with the
/// proof of every inferred triple (`derived S P O .`, then `  rule R` and one
/// `  premise S P O .` per matched fact, or `  data-block` for a SPARQL 1.2 RL data-block
/// triple), freed with `purrdf_buffer_free`.
///
/// `import_iris` / `import_documents` / `import_count` are the rule source's import table:
/// the shapes graph's `owl:imports` table (Turtle documents, see
/// `purrdf_shacl_validate_to_sarif`) beside `shapes_ttl`, the rule set's `IMPORTS` table
/// (SPARQL 1.2 RL texts) beside `srl`, followed transitively. An imported document's rules
/// run. An import no entry supplies, and an entry the import closure never names, are a
/// `ParseError`.
///
/// # Safety
/// `data_nt` must be a non-null NUL-terminated C string; `shapes_ttl`, `shapes_base_iri`,
/// `shapes_graph_iri`, `srl` and `srl_base_iri` must each be null or a NUL-terminated C
/// string;
/// `max_term_generating_rounds`, `max_generated_terms`, `max_stored_facts` and
/// `max_join_steps` must each be null or readable; when `import_count` is non-zero,
/// `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings; `out_inferred`
/// must be writable; `out_proof`, `out_diagnostics` and `out_error` must each be null or
/// writable.
///
/// `out_diagnostics` asks for the shapes graph's mandatory diagnostics: NULL skips them;
/// non-NULL receives a buffer (free with `purrdf_buffer_free`) of one `diagnostic RULE
/// SHAPE` line per shape with an empty `sh:in` or `sh:xone` list — empty when there is
/// none, and always for an `srl` rule set, which has no shapes graph.
/// Each line is `diagnostic `, the RULE (`in-minListLength` or `xone-minListLength`), one
/// space, then the SHAPE as an N-Triples term, then `\n` — the rule first and the shape
/// second, the field order every host's structured value (`{rule, shape}`) carries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_apply_rules(
    data_nt: *const c_char,
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    shapes_graph_iri: *const c_char,
    srl: *const c_char,
    srl_base_iri: *const c_char,
    max_term_generating_rounds: *const u64,
    max_generated_terms: *const u64,
    max_stored_facts: *const u64,
    max_join_steps: *const u64,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    out_inferred: *mut *mut PurrdfBuffer,
    out_proof: *mut *mut PurrdfBuffer,
    out_diagnostics: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if data_nt.is_null() || out_inferred.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shacl_apply_rules",
                ));
            }
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_shacl_apply_rules",
            )?;
            let request = RulesRequest {
                data_nt: cstr_to_str(data_nt)?,
                shapes_ttl: opt_cstr_to_str(shapes_ttl)?,
                shapes_base: opt_cstr_to_str(shapes_base_iri)?,
                shapes_graph: opt_cstr_to_str(shapes_graph_iri)?,
                imports: &imports,
                srl: opt_cstr_to_str(srl)?,
                srl_base: opt_cstr_to_str(srl_base_iri)?,
                explain: !out_proof.is_null(),
                // SAFETY: the caller's contract — null or readable.
                max_term_generating_rounds: max_term_generating_rounds.as_ref().copied(),
                // SAFETY: the caller's contract — null or readable.
                max_generated_terms: max_generated_terms.as_ref().copied(),
                // SAFETY: the caller's contract — null or readable.
                max_stored_facts: max_stored_facts.as_ref().copied(),
                // SAFETY: the caller's contract — null or readable.
                max_join_steps: max_join_steps.as_ref().copied(),
                host: purrdf_validate::RulesHost::CAbi,
            };
            let outcome = apply_rules_outcome(&request).map_err(PurrdfError::shapes)?;
            if let Some(proof) = outcome.proof {
                *out_proof = PurrdfBuffer::into_raw(proof.into_bytes());
            }
            if !out_diagnostics.is_null() {
                *out_diagnostics =
                    PurrdfBuffer::into_raw(diagnostic_text(&outcome.diagnostics).into_bytes());
            }
            *out_inferred = PurrdfBuffer::into_raw(outcome.inferred_ntriples.into_bytes());
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// How far `purrdf_shacl_check_rules` checks a SPARQL 1.2 RL rule set, each level
/// including the ones before it.
///
/// Append-only, like every other discriminant this ABI exports: never renumber a
/// variant. It is carried as an `int32_t` parameter rather than as this enum type so a C
/// caller passing an out-of-range value is refused rather than producing an invalid
/// discriminant.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurrdfSrlCheckLevel {
    /// The SPARQL 1.2 RL grammar, for the rule set and every document its imports read.
    Syntax = 0,
    /// `Syntax`, and every rule — imported ones included — is well formed.
    WellFormed = 1,
    /// `WellFormed`, and the combined rule set can be stratified: every static check a
    /// rules run applies before it evaluates.
    Stratified = 2,
}

impl PurrdfSrlCheckLevel {
    /// The engine's level for a caller's `int32_t`, or `None` for a value no variant has.
    const fn from_raw(raw: i32) -> Option<purrdf_validate::CheckLevel> {
        match raw {
            0 => Some(purrdf_validate::CheckLevel::Syntax),
            1 => Some(purrdf_validate::CheckLevel::WellFormed),
            2 => Some(purrdf_validate::CheckLevel::Stratified),
            _ => None,
        }
    }
}

/// Check a SPARQL 1.2 RL rule set. Native-testable, pointer-free core of
/// [`purrdf_shacl_check_rules`]: the work is [`check_rules`], and the answer is the
/// checked rule set's one-line summary.
fn check_rules_summary(
    srl: &str,
    srl_base: Option<&str>,
    level: i32,
    imports: &[(&str, &str)],
) -> Result<String, ShapesError> {
    let level = PurrdfSrlCheckLevel::from_raw(level).ok_or_else(|| {
        ShapesError::Invalid(format!(
            "{level} is not a PurrdfSrlCheckLevel: pass PURRDF_SRL_CHECK_LEVEL_SYNTAX (0), \
             PURRDF_SRL_CHECK_LEVEL_WELL_FORMED (1) or PURRDF_SRL_CHECK_LEVEL_STRATIFIED (2)"
        ))
    })?;
    check_rules(srl, srl_base, imports, level).map(|checked| checked.summary())
}

/// Check a SPARQL 1.2 RL rule set WITHOUT evaluating it — the grammar, the `IMPORTS`
/// closure resolved from the import table, well-formedness and stratification, every static
/// check `purrdf_shacl_apply_rules` applies before it runs — with no data graph read and no
/// rule run, and write the one-line summary every PurRDF host reports to `*out_summary`
/// (free with `purrdf_buffer_free`).
///
/// `level` is a `PurrdfSrlCheckLevel`: `PURRDF_SRL_CHECK_LEVEL_SYNTAX` (the grammar, for the
/// rule set and every document its imports read), `PURRDF_SRL_CHECK_LEVEL_WELL_FORMED`
/// (every rule, imported ones included, is well formed) or
/// `PURRDF_SRL_CHECK_LEVEL_STRATIFIED` (the combined rule set can be stratified). Any other
/// value is a `ParseError`. `srl_base_iri` is the rule set's base IRI and may be NULL.
///
/// `import_iris` / `import_documents` / `import_count` are the rule set's `IMPORTS` table
/// (SPARQL 1.2 RL texts), exactly as `purrdf_shacl_apply_rules` takes it beside `srl`. A
/// rule set a check refuses — a syntax error, an import no entry supplies or an entry its
/// closure never names, an ill-formed rule, a rule set that cannot be stratified — is a
/// `ParseError` whose message names the stage; `*out_summary` is then left untouched.
///
/// # Safety
/// `srl` must be a non-null NUL-terminated C string; `srl_base_iri` must be null or a
/// NUL-terminated C string; when `import_count` is non-zero, `import_iris` and
/// `import_documents` must each address that many NUL-terminated C strings; `out_summary`
/// must be writable; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_check_rules(
    srl: *const c_char,
    srl_base_iri: *const c_char,
    level: i32,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    out_summary: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if srl.is_null() || out_summary.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shacl_check_rules",
                ));
            }
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_shacl_check_rules",
            )?;
            let summary = check_rules_summary(
                cstr_to_str(srl)?,
                opt_cstr_to_str(srl_base_iri)?,
                level,
                &imports,
            )
            .map_err(PurrdfError::shapes)?;
            *out_summary = PurrdfBuffer::into_raw(summary.into_bytes());
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Evaluate one node expression. Native-testable, pointer-free core of
/// [`purrdf_shacl_eval_node_expr`]: `scope` holds `NAME=TERM` bindings, and the output
/// is one N-Triples 1.2 term per line, beside the shapes graph's mandatory diagnostics.
fn eval_node_expr_bytes(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    expr: ExprSelector<'_>,
    focus: &str,
    scope: &[&str],
    imports: &[(&str, &str)],
) -> Result<(Vec<u8>, Vec<purrdf_validate::MandatoryDiagnostic>), ShapesError> {
    let bindings = scope
        .iter()
        .map(|binding| parse_scope_binding(binding))
        .collect::<Result<Vec<_>, _>>()?;
    let outcome = eval_node_expr(&NodeExprRequest {
        shapes_ttl,
        shapes_base,
        data_nt,
        expr,
        focus,
        scope: &bindings,
        imports,
    })?;
    let mut out = String::new();
    for term in &outcome.outputs {
        out.push_str(term);
        out.push('\n');
    }
    Ok((out.into_bytes(), outcome.diagnostics))
}

/// Evaluate ONE node expression of a shapes graph (Turtle) against a focus node of a data
/// graph (N-Triples) — SHACL 1.2 Node Expressions' `evalExpr(expr, focusGraph, focusNode,
/// scope)` — and write its output nodes to `*out_terms` (free with `purrdf_buffer_free`):
/// one N-Triples 1.2 term per line, in the order the expression's sequence semantics
/// define. N-Triples escapes every line break inside a term, so each line is one term; an
/// expression with no output writes an empty buffer.
///
/// The expression is named exactly one way: exactly one of `expr`, `expr_at` and
/// `expr_turtle` is non-NULL. `expr` is an absolute IRI or `_:label` for a blank node the
/// shapes document labels so. `expr_at` names a node and `expr_via` / `expr_via_count` the
/// predicate IRIs a walk from it follows, each step reaching exactly one value — how an
/// anonymous `[ … ]` expression is named (`expr_via_count == 0` with `expr_at` is
/// refused; `expr_via` may be NULL only when the count is 0). `expr_turtle` is the
/// expression as a Turtle document, read under the shapes document's prefixes and base and
/// merged into the shapes graph, whose one root blank node is the expression. None or
/// several selectors, walk predicates with no `expr_at`, a walk step reaching no value or
/// several, and an inline document without exactly one root are a `ParseError`.
///
/// `focus` is an absolute IRI or any N-Triples term. `scope` / `scope_count` are
/// `NAME=TERM` bindings read by `shnex:var "NAME"`, the term spelled as `focus` is;
/// `scope_count == 0` binds nothing (`scope` may then be NULL). A label the shapes
/// document never wrote, a binding named `focusNode` or bound twice (neither could ever
/// be read), and any parse or evaluation failure are a `ParseError`.
///
/// `import_iris` / `import_documents` / `import_count` are the shapes graph's
/// `owl:imports` table (see `purrdf_shacl_validate_to_sarif`). An imported document's functions
/// and shapes are in scope.
///
/// `out_diagnostics` asks for the shapes graph's mandatory diagnostics, exactly as on
/// `purrdf_shacl_apply_rules`: NULL skips them; non-NULL receives a buffer (free with
/// `purrdf_buffer_free`) of one `diagnostic RULE SHAPE` line per shape of the shapes graph's
/// `owl:imports` closure with an empty `sh:in` or `sh:xone` list — empty when there is none
/// — which every run reports. They change no output.
/// Each line is `diagnostic `, the RULE (`in-minListLength` or `xone-minListLength`), one
/// space, then the SHAPE as an N-Triples term, then `\n` — the rule first and the shape
/// second, the field order every host's structured value (`{rule, shape}`) carries.
///
/// # Safety
/// `shapes_ttl`, `data_nt` and `focus` must be non-null NUL-terminated C strings;
/// `shapes_base_iri`, `expr`, `expr_at` and `expr_turtle` must each be null or a
/// NUL-terminated C string; when `expr_via_count` is non-zero, `expr_via` must address
/// that many NUL-terminated C strings; when `scope_count` is
/// non-zero, `scope` must address that many NUL-terminated C strings; when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings;
/// `out_terms` must be writable; `out_diagnostics` and `out_error` must each be null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_eval_node_expr(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    data_nt: *const c_char,
    expr: *const c_char,
    expr_at: *const c_char,
    expr_via: *const *const c_char,
    expr_via_count: usize,
    expr_turtle: *const c_char,
    focus: *const c_char,
    scope: *const *const c_char,
    scope_count: usize,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    out_terms: *mut *mut PurrdfBuffer,
    out_diagnostics: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if shapes_ttl.is_null() || data_nt.is_null() || focus.is_null() || out_terms.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shacl_eval_node_expr",
                ));
            }
            let bindings = cstr_array(scope, scope_count, "purrdf_shacl_eval_node_expr")?;
            let via = cstr_array(expr_via, expr_via_count, "purrdf_shacl_eval_node_expr")?;
            let selector = ExprSelector::from_parts(
                opt_cstr_to_str(expr)?,
                opt_cstr_to_str(expr_at)?,
                &via,
                opt_cstr_to_str(expr_turtle)?,
            )
            .map_err(|error| PurrdfError::shapes(error.into()))?;
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_shacl_eval_node_expr",
            )?;
            let (bytes, diagnostics) = eval_node_expr_bytes(
                cstr_to_str(shapes_ttl)?,
                opt_cstr_to_str(shapes_base_iri)?,
                cstr_to_str(data_nt)?,
                selector,
                cstr_to_str(focus)?,
                &bindings,
                &imports,
            )
            .map_err(PurrdfError::shapes)?;
            if !out_diagnostics.is_null() {
                *out_diagnostics =
                    PurrdfBuffer::into_raw(diagnostic_text(&diagnostics).into_bytes());
            }
            *out_terms = PurrdfBuffer::into_raw(bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Certify a shapes graph. Native-testable, pointer-free core of
/// [`purrdf_shacl_lint_shapes`].
fn lint_shapes_report(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    shapes_graph: Option<&str>,
    imports: &[(&str, &str)],
) -> Result<LintReport, ShapesError> {
    lint_shapes_ttl_with_shapes_graph(shapes_ttl, shapes_base, shapes_graph, imports)
}

/// Certify a shapes graph (Turtle) COLD — the loader's verdict, every result of validating
/// it against the W3C `shacl-shacl.ttl`, and which implementation every node-expression
/// function call binds to — and write the report's deterministic text to `*out_report`
/// (free with `purrdf_buffer_free`): the `load`, `shacl-shacl` (`result …` lines,
/// `superseded NAME` where SHACL 1.2 Core makes the flagged graph well-formed),
/// `functions` (`call BINDING <IRI> in OWNER`), `validators` (`alternative
/// <COMPONENT> <ATTACHMENT> VALIDATOR LANGUAGE superseded-by-native`, one per validator
/// declared for a built-in component), `unexecuted` (`violation DECLARATION`, one
/// per query that violates a pre-binding restriction and that nothing executes),
/// `diagnostics` (`diagnostic RULE SHAPE`, one per shape whose `sh:in` or `sh:xone` list is
/// empty, `RULE` being `in-minListLength` or `xone-minListLength`; each a finding, and the
/// `shacl-shacl` warning on the same list is marked `diagnosed RULE` and not counted) and
/// `unanchored-imports` (`unanchored SUBJECT OBJECT document -|<IRI>`, one per
/// `owl:imports` triple of the closure whose subject is no anchor of its document — not
/// the IRI it was read or imported under, not an ontology header, not a shapes graph — so
/// it is data and imported nothing; never a finding) sections, then `findings N` and
/// `clean true|false`.
///
/// `*out_clean` receives 1 when the report carries no finding — the loader accepted the
/// graph, every `shacl-shacl.ttl` result is superseded, no unexecuted query violates a
/// pre-binding restriction and no mandatory diagnostic applies — and 0 otherwise;
/// `*out_findings` receives the finding count. A malformed shapes graph is a report with
/// findings and status `Ok`; only a document that is not Turtle is a `ParseError`.
///
/// `import_iris` / `import_documents` / `import_count` are the shapes graph's
/// `owl:imports` table (see `purrdf_shacl_validate_to_sarif`). The report certifies the
/// whole closure; one that is not in hand returns `PURRDF_STATUS_SHAPES_IMPORT_ERROR` and
/// no report — never a report about the importing document alone, which would call a
/// shapes graph clean that validation refuses.
///
/// `shapes_graph_iri` is the shapes-graph IRI the loader is configured with, nullable —
/// `purrdf shapes lint --shapes-graph` (see `purrdf_shacl_validate_to_sarif`).
///
/// # Safety
/// `shapes_ttl` must be a non-null NUL-terminated C string; `shapes_base_iri` and
/// `shapes_graph_iri` must each be null or a NUL-terminated C string; when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings; `out_report`, `out_clean` and
/// `out_findings` must be writable; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_lint_shapes(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    shapes_graph_iri: *const c_char,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    out_report: *mut *mut PurrdfBuffer,
    out_clean: *mut i32,
    out_findings: *mut usize,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if shapes_ttl.is_null()
                || out_report.is_null()
                || out_clean.is_null()
                || out_findings.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shacl_lint_shapes",
                ));
            }
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_shacl_lint_shapes",
            )?;
            let report = lint_shapes_report(
                cstr_to_str(shapes_ttl)?,
                opt_cstr_to_str(shapes_base_iri)?,
                opt_cstr_to_str(shapes_graph_iri)?,
                &imports,
            )
            .map_err(PurrdfError::shapes)?;
            *out_clean = i32::from(report.is_clean());
            *out_findings = report.findings();
            *out_report = PurrdfBuffer::into_raw(report.render().into_bytes());
            Ok(PurrdfStatus::Ok)
        })
    }
}

// ---------------------------------------------------------------------------
// Prepared shapes products
// ---------------------------------------------------------------------------

/// Map a boundary refusal onto the C error that preserves its dimension — or, for a
/// shapes graph whose `owl:imports` closure is not in hand, onto the SAME
/// `PURRDF_STATUS_SHAPES_IMPORT_ERROR` every other shapes-graph entry point returns.
fn product_error(refusal: &ShapesProductRefusal) -> PurrdfError {
    if let Some(error) = refusal.import_error() {
        return PurrdfError::shapes_import(error);
    }
    PurrdfError::product(refusal.dimension_label(), refusal.to_string())
}

/// Borrow `len` bytes at `product`, refusing a null pointer by name.
///
/// A zero-length product is a real input a caller can hand over — an empty file — and
/// it is refused by the CODEC (it cannot carry the magic), not here, so the empty
/// slice is constructed rather than short-circuited. `std::slice::from_raw_parts`
/// requires a non-null, aligned pointer even at length zero, which the null check
/// above establishes for `u8`.
///
/// # Safety
/// `product` must be null or valid for reads of `len` bytes.
unsafe fn product_bytes<'a>(
    product: *const u8,
    len: usize,
    entry_point: &str,
) -> Result<&'a [u8], PurrdfError> {
    if product.is_null() {
        return Err(PurrdfError::new(
            PurrdfStatus::NullPointer,
            format!("null product pointer argument to {entry_point}"),
        ));
    }
    Ok(unsafe { std::slice::from_raw_parts(product, len) })
}

/// Compile `shapes_ttl` (Turtle) into prepared-product bytes. Native-testable,
/// pointer-free core.
fn encode_product_bytes(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    shapes_graph: Option<&str>,
    imports: &[(&str, &str)],
) -> Result<Vec<u8>, ShapesProductRefusal> {
    purrdf_validate::pack_shapes_product_with_shapes_graph(
        shapes_ttl,
        shapes_base,
        shapes_graph,
        imports,
    )
}

/// Compile a Turtle shapes graph into a PREPARED PRODUCT and write its bytes to
/// `*out_buffer` (free with `purrdf_buffer_free`).
///
/// The parse-and-analyze work `purrdf_shacl_validate_to_sarif` performs on every call,
/// done once and written to a container a host can cache on disk or ship between
/// processes. Byte-deterministic: no clock, no randomness and no hash-iteration order
/// reach the writer, so two calls over the same shapes graph and base produce identical
/// bytes and a content-addressed cache key over them is stable.
///
/// `shapes_base_iri` carries the same meaning it does on
/// `purrdf_shacl_validate_to_sarif` — the shapes document's own base IRI, nullable —
/// and is RECORDED in the product, so a restore resolves the same relative references
/// without the document.
///
/// Returns `PURRDF_STATUS_SHAPES_PRODUCT_ERROR` when the shapes graph declares something the
/// product format cannot carry; the error's dimension is then readable with
/// `purrdf_shapes_product_error_dimension`, and is NULL when the shapes document simply
/// did not parse (no product existed to name a dimension of).
///
/// `import_iris` / `import_documents` / `import_count` are the shapes graph's
/// `owl:imports` table (see `purrdf_shacl_validate_to_sarif`). The product carries the merged
/// closure, so a restore needs no documents; one that is not in hand returns
/// `PURRDF_STATUS_SHAPES_IMPORT_ERROR`, exactly as validation does.
///
/// `shapes_graph_iri` (see `purrdf_shacl_validate_to_sarif`) is nullable, RECORDED in the
/// product and bound by its identity — `purrdf shacl pack --shapes-graph` — so a restore
/// exposes the shapes graph under it.
///
/// # Safety
/// `shapes_ttl` must be a non-null, NUL-terminated C string; `shapes_base_iri` and
/// `shapes_graph_iri` must each be null or a NUL-terminated C string; when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings; `out_buffer` must be a writable
/// pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_product_encode(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    shapes_graph_iri: *const c_char,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if shapes_ttl.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shapes_product_encode",
                ));
            }
            let shapes = cstr_to_str(shapes_ttl)?;
            let base = opt_cstr_to_str(shapes_base_iri)?;
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_shapes_product_encode",
            )?;
            let graph = opt_cstr_to_str(shapes_graph_iri)?;
            let bytes = encode_product_bytes(shapes, base, graph, &imports)
                .map_err(|refusal| product_error(&refusal))?;
            *out_buffer = PurrdfBuffer::into_raw(bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Open a prepared product and render its self-description. Native-testable,
/// pointer-free core.
fn open_product_bytes(product: &[u8]) -> Result<Vec<u8>, ShapesProductRefusal> {
    purrdf_validate::explain_shapes_product(product)
        .map(String::into_bytes)
        .map_err(ShapesProductRefusal::from)
}

/// OPEN a prepared product — verify its envelope and decode what it says it was
/// compiled from — and write that description to `*out_buffer` (free with
/// `purrdf_buffer_free`). Nothing is admitted.
///
/// The description is deterministic UTF-8 `key value` lines: the container format
/// version, the preparation stage id and whether this build knows it, the identity
/// digest and every labelled identity component, then the recorded base,
/// `sh:shapesGraph` IRI and prefix map. It is the identical text the CLI's
/// `purrdf shacl explain` prints and the WebAssembly host receives.
///
/// This is what makes a named refusal actionable: an admit refused on `prefixes` is
/// answered by reading which prefix map the product actually carries, rather than
/// guessing or re-encoding blindly.
///
/// # Safety
/// `product` must be valid for reads of `product_len` bytes; `out_buffer` must be a
/// writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_product_open(
    product: *const u8,
    product_len: usize,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shapes_product_open",
                ));
            }
            let bytes = product_bytes(product, product_len, "purrdf_shapes_product_open")?;
            let described = open_product_bytes(bytes).map_err(|refusal| product_error(&refusal))?;
            *out_buffer = PurrdfBuffer::into_raw(described);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Admit a prepared product and validate a data graph with it. Native-testable,
/// pointer-free core.
fn admit_product_bytes(product: &[u8], data_nt: &str) -> Result<Vec<u8>, ShapesProductRefusal> {
    purrdf_validate::validate_with_shapes_product(product, data_nt, &SarifOptions::default())
        .map(String::into_bytes)
}

/// ADMIT a prepared product, validate `data_nt` (N-Triples) with it, and write the
/// SARIF 2.1.0 report bytes to `*out_buffer` (free with `purrdf_buffer_free`).
///
/// The point of a product: restore the preparation rather than re-parse the shapes
/// graph. The verdict is the identical one `purrdf_shacl_validate_to_sarif` reaches
/// over the shapes document the product was encoded from — the same engine entry point
/// runs, over the same restored shapes.
///
/// Admission runs first and in full, and the product is restored against the EMPTY
/// host bindings; see this module's documentation for why a C host cannot supply
/// others, and why a product that needs them is refused rather than mis-executed.
///
/// A malformed `data_nt` also returns `PURRDF_STATUS_SHAPES_PRODUCT_ERROR`, with a NULL
/// dimension: the data graph is not a product, so no admission dimension names it, and
/// borrowing one would claim the product was at fault.
///
/// # Safety
/// `product` must be valid for reads of `product_len` bytes; `data_nt` must be a
/// non-null, NUL-terminated C string; `out_buffer` must be a writable pointer;
/// `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_product_admit(
    product: *const u8,
    product_len: usize,
    data_nt: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if data_nt.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shapes_product_admit",
                ));
            }
            let bytes = product_bytes(product, product_len, "purrdf_shapes_product_admit")?;
            let data = cstr_to_str(data_nt)?;
            let sarif =
                admit_product_bytes(bytes, data).map_err(|refusal| product_error(&refusal))?;
            *out_buffer = PurrdfBuffer::into_raw(sarif);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Admit a prepared product bound to an expected identity, and validate a data graph
/// with it. Native-testable, pointer-free core.
///
fn admit_product_bytes_expecting(
    product: &[u8],
    data_nt: &str,
    expect_identity: &[u8; 32],
) -> Result<Vec<u8>, ShapesProductRefusal> {
    purrdf_validate::validate_with_shapes_product_expecting(
        product,
        data_nt,
        expect_identity,
        &SarifOptions::default(),
    )
    .map(String::into_bytes)
}

/// ADMIT a prepared product ONLY IF its input binding is `expect_identity`, validate
/// `data_nt` (N-Triples) with it, and write the SARIF 2.1.0 report bytes to
/// `*out_buffer` (free with `purrdf_buffer_free`).
///
/// Everything `purrdf_shapes_product_admit` checks is a question about the executing
/// process — its build, its registries, its class analysis. None of them asks whether
/// these are the bytes the caller meant, because nothing in a product states which
/// product was wanted. A host that mmaps a cache entry, reads a product a deployment
/// placed on disk, or builds its path from a configuration string has no other way to
/// say so, and admitting the wrong one produces a decided, well-formed SARIF log about
/// a shapes graph nobody asked about.
///
/// `expect_identity` is the 64 hexadecimal digits `purrdf_shapes_product_open` renders
/// on its `identity-digest` line, passed back unchanged — one spelling, readable off
/// the artifact, so the selector can be pinned beside the product it names.
///
/// A product carrying a different binding returns `PURRDF_STATUS_SHAPES_PRODUCT_ERROR`
/// with the dimension `shapes-graph`. An `expect_identity` that is not 64 hexadecimal
/// digits returns `PURRDF_STATUS_INVALID_ARGUMENT` instead, and not as a product
/// refusal: no product was ever opened, so there is nothing for an admission dimension
/// to name, and reporting the caller's own argument as a product failure would send
/// them to inspect an artifact that is not at fault.
///
/// # Safety
/// `product` must be valid for reads of `product_len` bytes; `data_nt` and
/// `expect_identity` must be non-null, NUL-terminated C strings; `out_buffer` must be a
/// writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_product_admit_expecting(
    product: *const u8,
    product_len: usize,
    data_nt: *const c_char,
    expect_identity: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if data_nt.is_null() || expect_identity.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shapes_product_admit_expecting",
                ));
            }
            let bytes = product_bytes(
                product,
                product_len,
                "purrdf_shapes_product_admit_expecting",
            )?;
            let data = cstr_to_str(data_nt)?;
            let expected = purrdf_validate::parse_identity_digest(cstr_to_str(expect_identity)?)
                .map_err(|message| PurrdfError::new(PurrdfStatus::InvalidArgument, message))?;
            let sarif = admit_product_bytes_expecting(bytes, data, &expected)
                .map_err(|refusal| product_error(&refusal))?;
            *out_buffer = PurrdfBuffer::into_raw(sarif);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Rebuild a prepared product and validate a data graph with it. Native-testable,
/// pointer-free core.
fn rebuild_product_bytes(product: &[u8], data_nt: &str) -> Result<Vec<u8>, ShapesProductRefusal> {
    purrdf_validate::validate_with_rebuilt_shapes_product(
        product,
        data_nt,
        &SarifOptions::default(),
    )
    .map(String::into_bytes)
}

/// REBUILD a prepared product — re-deriving its preparation from the shapes
/// dataset it carries, ignoring its memo — validate `data_nt` (N-Triples) with it,
/// and write the SARIF 2.1.0 report bytes to `*out_buffer` (free with
/// `purrdf_buffer_free`).
///
/// The forward-compatibility path: a product whose stage id this build does not
/// know refuses `purrdf_shapes_product_admit` with the dimension `stage-id`, and
/// this is the remedy it names. No RDF text is parsed and no file other than the
/// product itself is read — the shapes dataset travels inside the product under
/// the envelope's own digests, and this re-derives the shapes graph from it.
///
/// Also correct, and does the identical work, over a CURRENT product whose stage
/// id this build already knows: rebuilding re-derives from the SAME carried
/// dataset `purrdf_shapes_product_admit` restores a memo of, so the two reach the
/// byte-identical report. This entry point is a second DOOR onto one product,
/// never a second, divergent answer.
///
/// Admission runs against the EMPTY host bindings, for the same reason
/// `purrdf_shapes_product_admit` does; see this module's documentation.
///
/// A malformed `data_nt` also returns `PURRDF_STATUS_SHAPES_PRODUCT_ERROR`, with a NULL
/// dimension: the data graph is not a product, so no admission dimension names it.
///
/// # Safety
/// `product` must be valid for reads of `product_len` bytes; `data_nt` must be a
/// non-null, NUL-terminated C string; `out_buffer` must be a writable pointer;
/// `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_product_rebuild(
    product: *const u8,
    product_len: usize,
    data_nt: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if data_nt.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shapes_product_rebuild",
                ));
            }
            let bytes = product_bytes(product, product_len, "purrdf_shapes_product_rebuild")?;
            let data = cstr_to_str(data_nt)?;
            let sarif =
                rebuild_product_bytes(bytes, data).map_err(|refusal| product_error(&refusal))?;
            *out_buffer = PurrdfBuffer::into_raw(sarif);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Rebuild a prepared product bound to an expected identity, and validate a data
/// graph with it. Native-testable, pointer-free core.
fn rebuild_product_bytes_expecting(
    product: &[u8],
    data_nt: &str,
    expect_identity: &[u8; 32],
) -> Result<Vec<u8>, ShapesProductRefusal> {
    purrdf_validate::validate_with_rebuilt_shapes_product_expecting(
        product,
        data_nt,
        expect_identity,
        &SarifOptions::default(),
    )
    .map(String::into_bytes)
}

/// REBUILD a prepared product ONLY IF its input binding is `expect_identity` —
/// re-deriving its preparation from the shapes dataset it carries, ignoring its
/// memo — validate `data_nt` (N-Triples) with it, and write the SARIF 2.1.0
/// report bytes to `*out_buffer` (free with `purrdf_buffer_free`).
///
/// The bound twin of `purrdf_shapes_product_rebuild`, for the same reason
/// `purrdf_shapes_product_admit_expecting` exists beside
/// `purrdf_shapes_product_admit`: the forward-compatibility rescue is not a
/// reason to stop asking *is this the product I asked for?* — a cache entry from
/// another build, or a product a deployment placed on disk under a stage id this
/// build does not recognize, is still just a file that could be the wrong one.
/// The 32-byte comparison runs FIRST, ahead of the re-derivation, exactly as it
/// does on `purrdf_shapes_product_admit_expecting`, so a product that is not the
/// one required is named as such rather than re-derived and validated against.
///
/// `expect_identity` carries the same meaning it does on
/// `purrdf_shapes_product_admit_expecting` — the 64 hexadecimal digits
/// `purrdf_shapes_product_open` renders on its `identity-digest` line, passed
/// back unchanged.
///
/// Admission runs against the EMPTY host bindings, for the same reason
/// `purrdf_shapes_product_admit` does; see this module's documentation.
///
/// A product carrying a different binding returns `PURRDF_STATUS_SHAPES_PRODUCT_ERROR`
/// with the dimension `shapes-graph`. An `expect_identity` that is not 64
/// hexadecimal digits returns `PURRDF_STATUS_INVALID_ARGUMENT` instead, and not
/// as a product refusal: no product was ever opened, so there is nothing for an
/// admission dimension to name.
///
/// # Safety
/// `product` must be valid for reads of `product_len` bytes; `data_nt` and
/// `expect_identity` must be non-null, NUL-terminated C strings; `out_buffer`
/// must be a writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_product_rebuild_expecting(
    product: *const u8,
    product_len: usize,
    data_nt: *const c_char,
    expect_identity: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if data_nt.is_null() || expect_identity.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shapes_product_rebuild_expecting",
                ));
            }
            let bytes = product_bytes(
                product,
                product_len,
                "purrdf_shapes_product_rebuild_expecting",
            )?;
            let data = cstr_to_str(data_nt)?;
            let expected = purrdf_validate::parse_identity_digest(cstr_to_str(expect_identity)?)
                .map_err(|message| PurrdfError::new(PurrdfStatus::InvalidArgument, message))?;
            let sarif = rebuild_product_bytes_expecting(bytes, data, &expected)
                .map_err(|refusal| product_error(&refusal))?;
            *out_buffer = PurrdfBuffer::into_raw(sarif);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Corroborate a prepared product's carried dataset against its claimed identity.
/// Native-testable, pointer-free core.
fn certify_product_bytes(product: &[u8]) -> Result<(), ShapesProductRefusal> {
    purrdf_validate::certify_shapes_product(product).map_err(ShapesProductRefusal::from)
}

/// CERTIFY a prepared product: independently re-derive its shapes dataset's canonical
/// identity and compare it against the one the product's own binding claims.
///
/// The codec's COLD path, and deliberately unreachable from a restore —
/// canonicalization is a graph-isomorphism computation over the shapes graph's blank
/// nodes and can cost more than the shapes parse a product exists to eliminate. Call it
/// from a build step or a test, never before every validation:
/// `purrdf_shapes_product_admit` already verifies every section digest and the whole
/// container.
///
/// There is no out-buffer: the answer is the status. `PURRDF_STATUS_OK` means the product's
/// dataset canonicalizes to the digest it claims.
///
/// # Safety
/// `product` must be valid for reads of `product_len` bytes; `out_error` must be null
/// or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_product_certify(
    product: *const u8,
    product_len: usize,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            let bytes = product_bytes(product, product_len, "purrdf_shapes_product_certify")?;
            certify_product_bytes(bytes).map_err(|refusal| product_error(&refusal))?;
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// The prepared-shapes-product admission DIMENSION `err` names, or NULL.
///
/// A borrowed, NUL-terminated string valid until `purrdf_error_free(err)`; the C side
/// must not free it. It is one of the codec's pinned kebab-case labels — `magic`,
/// `format-version`, `stage-id`, `profile`, `truncated`, `trailer`, `section-digest`,
/// `container-digest`, `dataset-identity`, `shapes-graph`, `prefixes`, `base`,
/// `vocabulary`, `function-registry`, `aggregate-registry`,
/// `property-function-registry`, `class-catalog`, `parse-configuration`,
/// `included-graphs`, `unsupported-capability`, `depth-limit`, `malformed`.
///
/// NULL — never an empty string — when `err` is null, when it is not a product refusal
/// at all, or when the failure happened before any product existed (a shapes or data
/// document that did not parse was never admitted). An empty string would be a label a
/// caller could compare against and believe.
///
/// This is the stable, matchable half of a refusal. Branch on it rather than on
/// `purrdf_error_message`, whose prose names the fix and may be reworded.
///
/// # Safety
/// `err` must be null or a pointer returned by a libpurrdf entry point and not yet
/// freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_product_error_dimension(
    err: *const PurrdfError,
) -> *const c_char {
    unsafe {
        ffi_guard!(std::ptr::null(), {
            if err.is_null() {
                return std::ptr::null();
            }
            (*err)
                .dimension
                .as_ref()
                .map_or(std::ptr::null(), |label| label.as_ptr())
        })
    }
}

/// The KIND of the shapes-graph `owl:imports` refusal `err` is, or NULL.
///
/// A borrowed, NUL-terminated string valid until `purrdf_error_free(err)`; the C side
/// must not free it. One of `unresolved-import` (the closure imports ontologies nothing
/// in hand resolves — pass their documents in the import table), `unreached-import` (the
/// table supplies documents neither an import nor a data-graph link names),
/// `incompatible-import-versions` (the closure holds two versions of one series, or a graph
/// another declares `owl:incompatibleWith`, SHACL 1.2 Core sections 1.3 and 6.1),
/// `invalid-import` (a key that is not an absolute IRI, a key named twice, or a document
/// that is not Turtle), `unresolved-shapes-graph-link` (the data graph links a graph with
/// `sh:shapesGraph`, SHACL 1.2 Core section 6.4, that nothing in hand resolves — pass it in
/// the import table), `unheld-shapes-graph-link` (a prepared product does not hold a graph the data
/// graph links) or `invalid-shapes-graph-link` (a data-graph `sh:shapesGraph` value that
/// is not an IRI).
///
/// NULL — never an empty string — when `err` is null or is not a
/// `PURRDF_STATUS_SHAPES_IMPORT_ERROR`. Branch on it rather than on
/// `purrdf_error_message`, whose prose names the fix and may be reworded.
///
/// # Safety
/// `err` must be null or a pointer returned by a libpurrdf entry point and not yet
/// freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_import_error_kind(err: *const PurrdfError) -> *const c_char {
    unsafe {
        ffi_guard!(std::ptr::null(), {
            if err.is_null() {
                return std::ptr::null();
            }
            (*err)
                .import
                .as_ref()
                .map_or(std::ptr::null(), |import| import.kind.as_ptr())
        })
    }
}

/// How many IRIs the shapes-graph `owl:imports` refusal `err` names: 0 when `err` is
/// null or is not a `PURRDF_STATUS_SHAPES_IMPORT_ERROR`.
///
/// # Safety
/// Same contract as [`purrdf_shapes_import_error_kind`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_import_error_iri_count(err: *const PurrdfError) -> usize {
    unsafe {
        ffi_guard!(0, {
            if err.is_null() {
                return 0;
            }
            (*err).import.as_ref().map_or(0, |import| import.iris.len())
        })
    }
}

/// The `index`-th IRI the shapes-graph `owl:imports` refusal `err` names, in the
/// engine's order, or NULL when `err` is null, is not a
/// `PURRDF_STATUS_SHAPES_IMPORT_ERROR`, or `index` is out of range.
///
/// A borrowed, NUL-terminated string valid until `purrdf_error_free(err)`.
///
/// # Safety
/// Same contract as [`purrdf_shapes_import_error_kind`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_import_error_iri(
    err: *const PurrdfError,
    index: usize,
) -> *const c_char {
    unsafe {
        ffi_guard!(std::ptr::null(), {
            if err.is_null() {
                return std::ptr::null();
            }
            (*err)
                .import
                .as_ref()
                .and_then(|import| import.iris.get(index))
                .map_or(std::ptr::null(), |iri| iri.as_ptr())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{purrdf_error_code, purrdf_error_free};

    const SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
        ex:PersonShape a sh:NodeShape ;\n\
          sh:targetClass ex:Person ;\n\
          sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\n";

    const DATA: &str = "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n\
        <http://example.org/alice> <http://example.org/age> \"nope\" .\n";

    #[test]
    fn validate_emits_sarif_bytes() {
        let bytes = validate_to_sarif_bytes(SHAPES, None, None, DATA, &[], &[], false)
            .expect("sarif produced");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("\"version\": \"2.1.0\""));
        assert!(text.contains("\"level\": \"error\""));
    }

    #[test]
    fn malformed_shapes_is_an_error() {
        assert!(
            validate_to_sarif_bytes("@@@ not turtle", None, None, DATA, &[], &[], false).is_err()
        );
    }

    /// Across the C boundary, a shapes graph carrying the W3C
    /// SHACL 1.2 vocabulary's `sh:SPARQLExprExpression` declaration verbatim (the tools
    /// fixture, [`TOOLS_SHAPES`]) loads, and a property shape's `sh:values [
    /// sh:sparqlExpr "ex:yes" ; sh:prefixes ex:Prefixes ]` is evaluated natively: the
    /// Warning result's value is the computed `<http://example.org/ns#yes>`, which only
    /// the prefix-expanded expression yields. It does not conform under the default
    /// conformance-disallow set and conforms under `sh:Violation` alone.
    #[test]
    fn capi_validate_evaluates_sparql_expr_beside_its_vocabulary_declaration() {
        let shapes = format!(
            "{TOOLS_SHAPES}
ex:StatusShape a sh:NodeShape ;
  sh:targetClass ex:Item ;
  sh:property [
    sh:path ex:status ;
    sh:values [ sh:sparqlExpr \"ex:yes\" ; sh:prefixes ex:Prefixes ] ;
    sh:in ( ex:no ) ;
    sh:severity sh:Warning
  ] .
"
        );
        let run = |levels: &[&str]| -> serde_json::Value {
            let sarif =
                validate_to_sarif_bytes(&shapes, None, None, TOOLS_DATA, levels, &[], false)
                    .expect("the declaration-bearing shapes graph loads and validates");
            serde_json::from_slice(&sarif).expect("json")
        };
        let default = run(&[]);
        assert_eq!(default["runs"][0]["properties"]["shaclConforms"], false);
        let results = default["runs"][0]["results"].as_array().expect("results");
        assert_eq!(results.len(), 1, "{default}");
        let text = results[0]["message"]["text"].as_str().expect("message");
        assert!(
            text.starts_with("Value <http://example.org/ns#yes> "),
            "the computed value: {text}"
        );
        let relaxed = run(&["http://www.w3.org/ns/shacl#Violation"]);
        assert_eq!(relaxed["runs"][0]["properties"]["shaclConforms"], true);
    }

    /// The conformance-disallow set crosses the boundary as a C string array and
    /// reaches the validation: a Warning-graded violation does not conform under the
    /// default set (count 0, NULL array) and conforms under `sh:Violation` alone; a
    /// non-IRI level is a `ParseError`, and a NULL array with a non-zero count a
    /// `NullPointer`, each refused before anything is validated.
    #[test]
    fn capi_validate_subclass_of_in_shapes_graph() {
        use std::ffi::CString;

        use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};

        // A class target reached only through the shapes graph's rdfs:subClassOf fires
        // with the parameter and not without it; the control, a direct instance of the
        // target class, fires both ways.
        let shapes = CString::new(
            "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix ex: <http://example.org/> .\n\
             ex:Student rdfs:subClassOf ex:Person .\n\
             ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;\n\
               sh:property [ sh:path ex:name ; sh:minCount 1 ] .\n",
        )
        .expect("no NUL");
        let data = CString::new(
            "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.org/Student> .\n\
             <http://example.org/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.org/Person> .\n",
        )
        .expect("no NUL");
        let run = |on: bool| -> String {
            let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString or a writable local; the disallow
            // set and the import table are empty (NULL arrays, zero counts).
            let status = unsafe {
                purrdf_shacl_validate_to_sarif(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    data.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    on,
                    &raw mut buffer,
                    &raw mut error,
                )
            };
            assert_eq!(status, PurrdfStatus::Ok as i32);
            // SAFETY: a successful call wrote a live buffer.
            unsafe {
                let mut ptr: *const u8 = std::ptr::null();
                let mut len = 0usize;
                assert_eq!(
                    purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len),
                    PurrdfStatus::Ok as i32
                );
                let text = std::str::from_utf8(std::slice::from_raw_parts(ptr, len))
                    .expect("utf8")
                    .to_owned();
                purrdf_buffer_free(buffer);
                text
            }
        };
        let off = run(false);
        assert!(off.contains("http://example.org/bob"), "{off}");
        assert!(!off.contains("http://example.org/alice"), "{off}");
        // Every report states sh:shapesGraphWellFormed (SHACL 1.2 Core §6.7.1.4).
        assert!(
            off.contains("\"shaclShapesGraphWellFormed\": true"),
            "{off}"
        );
        let on = run(true);
        assert!(on.contains("http://example.org/bob"), "{on}");
        assert!(on.contains("http://example.org/alice"), "{on}");
    }

    #[test]
    fn capi_validate_conformance_disallows() {
        use std::ffi::CString;

        use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};

        let shapes = CString::new(SHAPES.replace(
            "sh:path ex:age ;",
            "sh:path ex:age ; sh:severity sh:Warning ;",
        ))
        .expect("no NUL");
        let data = CString::new(DATA).expect("no NUL");
        let conforms = |levels: &[&str]| -> (i32, Option<serde_json::Value>) {
            let owned: Vec<CString> = levels
                .iter()
                .map(|level| CString::new(*level).expect("no NUL"))
                .collect();
            let pointers: Vec<*const c_char> = owned.iter().map(|level| level.as_ptr()).collect();
            let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString or a writable local; the array
            // holds exactly `pointers.len()` elements.
            let status = unsafe {
                purrdf_shacl_validate_to_sarif(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    data.as_ptr(),
                    if pointers.is_empty() {
                        std::ptr::null()
                    } else {
                        pointers.as_ptr()
                    },
                    pointers.len(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    false,
                    &raw mut buffer,
                    &raw mut error,
                )
            };
            if status != PurrdfStatus::Ok as i32 {
                // SAFETY: the error handle was written by the call above.
                unsafe { purrdf_error_free(error) };
                return (status, None);
            }
            // SAFETY: a successful call wrote a live buffer.
            let text = unsafe {
                let mut ptr: *const u8 = std::ptr::null();
                let mut len = 0usize;
                assert_eq!(
                    purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len),
                    PurrdfStatus::Ok as i32
                );
                let text = std::str::from_utf8(std::slice::from_raw_parts(ptr, len))
                    .expect("utf8")
                    .to_owned();
                purrdf_buffer_free(buffer);
                text
            };
            let log: serde_json::Value = serde_json::from_str(&text).expect("json");
            (
                status,
                Some(log["runs"][0]["properties"]["shaclConforms"].clone()),
            )
        };
        assert_eq!(
            conforms(&[]),
            (PurrdfStatus::Ok as i32, Some(serde_json::json!(false)))
        );
        assert_eq!(
            conforms(&["http://www.w3.org/ns/shacl#Violation"]),
            (PurrdfStatus::Ok as i32, Some(serde_json::json!(true)))
        );
        assert_eq!(conforms(&["Violation"]).0, PurrdfStatus::ParseError as i32);

        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: a NULL array with a non-zero count is refused before any read.
        let status = unsafe {
            purrdf_shacl_validate_to_sarif(
                shapes.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                data.as_ptr(),
                std::ptr::null(),
                1,
                std::ptr::null(),
                std::ptr::null(),
                0,
                false,
                &raw mut buffer,
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::NullPointer as i32);
        // SAFETY: the error handle was written by the call above.
        unsafe { purrdf_error_free(error) };
    }

    /// A conforming base, so every violation a change test sees is the change's.
    const CHANGE_BASE: &str = "<http://example.org/alice> \
        <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";

    /// The row that breaks it, and the row that un-breaks it again.
    const BAD_AGE: &str = "<http://example.org/alice> <http://example.org/age> \"nope\" .\n";

    /// Run the change path through the POINTER surface with a disallow set and the
    /// subclass parameter, returning the SARIF log as JSON, or the error's status.
    fn changes_through_pointers(
        shapes: &str,
        data: &str,
        added: &str,
        disallows: &[&str],
        subclass_of_in_shapes_graph: bool,
    ) -> Result<serde_json::Value, i32> {
        use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};
        let shapes = std::ffi::CString::new(shapes).expect("no interior NUL");
        let data = std::ffi::CString::new(data).expect("no interior NUL");
        let added = std::ffi::CString::new(added).expect("no interior NUL");
        let levels: Vec<std::ffi::CString> = disallows
            .iter()
            .map(|level| std::ffi::CString::new(*level).expect("no interior NUL"))
            .collect();
        let level_ptrs: Vec<*const c_char> = levels.iter().map(|level| level.as_ptr()).collect();
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut reason: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut scope: i32 = -1;
        let mut focus_nodes: usize = usize::MAX;
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: every C string and the level array outlive the call; the out-pointers are
        // writable locals, and a written buffer or error is freed below.
        unsafe {
            let status = purrdf_shacl_validate_changes_to_sarif(
                shapes.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                data.as_ptr(),
                added.as_ptr(),
                std::ptr::null(),
                if level_ptrs.is_empty() {
                    std::ptr::null()
                } else {
                    level_ptrs.as_ptr()
                },
                level_ptrs.len(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                subclass_of_in_shapes_graph,
                &raw mut buffer,
                &raw mut scope,
                &raw mut focus_nodes,
                &raw mut reason,
                &raw mut error,
            );
            if status != PurrdfStatus::Ok as i32 {
                assert!(buffer.is_null() && !error.is_null());
                purrdf_error_free(error);
                return Err(status);
            }
            let mut ptr: *const u8 = std::ptr::null();
            let mut len = 0usize;
            assert_eq!(
                purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len),
                PurrdfStatus::Ok as i32
            );
            let log = serde_json::from_slice(std::slice::from_raw_parts(ptr, len)).expect("json");
            purrdf_buffer_free(buffer);
            if !reason.is_null() {
                purrdf_buffer_free(reason);
            }
            Ok(log)
        }
    }

    /// The change path judges a change exactly as the whole-graph path judges the graph:
    /// the conformance-disallow set and `subClassOfInShapesGraph` both reach it.
    ///
    /// A Warning-graded violation the change introduces makes the report non-conforming
    /// under the default set and conforming under `sh:Violation` alone; a non-IRI level is
    /// refused. A class target the changed node reaches only through the shapes graph's
    /// `rdfs:subClassOf` fires with the parameter and not without it, while the control — a
    /// direct instance of the target class — fires both ways.
    #[test]
    fn a_change_takes_the_disallow_set_and_the_subclass_parameter() {
        const WARNING_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
            @prefix ex: <http://example.org/> .\n\
            @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
            ex:PersonShape a sh:NodeShape ;\n\
              sh:targetClass ex:Person ;\n\
              sh:property [ sh:path ex:age ; sh:datatype xsd:integer ; \
                sh:severity sh:Warning ] .\n";
        let conforms =
            |log: &serde_json::Value| log["runs"][0]["properties"]["shaclConforms"].clone();
        let default = changes_through_pointers(WARNING_SHAPES, CHANGE_BASE, BAD_AGE, &[], false)
            .expect("the default set");
        assert_eq!(conforms(&default), false, "{default}");
        let relaxed = changes_through_pointers(
            WARNING_SHAPES,
            CHANGE_BASE,
            BAD_AGE,
            &["http://www.w3.org/ns/shacl#Violation"],
            false,
        )
        .expect("sh:Violation alone");
        assert_eq!(conforms(&relaxed), true, "{relaxed}");
        assert_eq!(
            changes_through_pointers(WARNING_SHAPES, CHANGE_BASE, BAD_AGE, &["not an iri"], false)
                .expect_err("a level that is not an IRI"),
            PurrdfStatus::ParseError as i32
        );

        const SUBCLASS_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
            @prefix ex: <http://example.org/> .\n\
            ex:Student rdfs:subClassOf ex:Person .\n\
            ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;\n\
              sh:property [ sh:path ex:name ; sh:minCount 1 ] .\n";
        const ADDED: &str = "<http://example.org/alice> \
            <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Student> .\n\
            <http://example.org/bob> \
            <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";
        let focus = |log: &serde_json::Value| log.to_string();
        let off = changes_through_pointers(SUBCLASS_SHAPES, "", ADDED, &[], false).expect("off");
        assert!(focus(&off).contains("http://example.org/bob"), "{off}");
        assert!(!focus(&off).contains("http://example.org/alice"), "{off}");
        let on = changes_through_pointers(SUBCLASS_SHAPES, "", ADDED, &[], true).expect("on");
        assert!(focus(&on).contains("http://example.org/bob"), "{on}");
        assert!(focus(&on).contains("http://example.org/alice"), "{on}");
    }

    #[test]
    fn a_change_is_validated_against_the_graph_it_joins() {
        let (bytes, scope) = validate_changes_to_sarif_bytes(
            SHAPES,
            None,
            None,
            CHANGE_BASE,
            Some(BAD_AGE),
            None,
            &[],
            &[],
            false,
        )
        .expect("the change validates");
        assert_eq!(scope, ChangeScope::Bounded { focus_nodes: 1 });
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("DatatypeConstraintComponent"), "{text}");

        // The retract half is a real half: taking the bad row back out of the
        // merged graph restores conformance, through the same one call.
        let merged = format!("{CHANGE_BASE}{BAD_AGE}");
        let (bytes, scope) = validate_changes_to_sarif_bytes(
            SHAPES,
            None,
            None,
            &merged,
            None,
            Some(BAD_AGE),
            &[],
            &[],
            false,
        )
        .expect("the retraction validates");
        assert_eq!(scope, ChangeScope::Bounded { focus_nodes: 1 });
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(!text.contains("\"level\": \"error\""), "{text}");
    }

    /// The exported entry point, driven through pointers exactly as a C host
    /// drives it — including the scope outputs, which are what stop a caller from
    /// reading "this change introduced no violation" as "the graph conforms".
    #[test]
    fn the_exported_change_entry_point_reports_its_scope_through_pointers() {
        use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};

        /// The bytes behind a buffer, as UTF-8.
        unsafe fn text_of(buffer: *mut PurrdfBuffer) -> String {
            let mut ptr: *const u8 = std::ptr::null();
            let mut len: usize = 0;
            unsafe {
                assert_eq!(
                    purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len),
                    PurrdfStatus::Ok as i32
                );
                std::str::from_utf8(std::slice::from_raw_parts(ptr, len))
                    .expect("utf8")
                    .to_owned()
            }
        }

        let shapes = std::ffi::CString::new(SHAPES).expect("no interior NUL");
        let data = std::ffi::CString::new(CHANGE_BASE).expect("no interior NUL");
        let added = std::ffi::CString::new(BAD_AGE).expect("no interior NUL");

        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut reason: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut scope: i32 = -1;
        let mut focus_nodes: usize = usize::MAX;
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        unsafe {
            let status = purrdf_shacl_validate_changes_to_sarif(
                shapes.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                data.as_ptr(),
                added.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                false,
                &raw mut buffer,
                &raw mut scope,
                &raw mut focus_nodes,
                &raw mut reason,
                &raw mut error,
            );
            assert_eq!(status, PurrdfStatus::Ok as i32);
            assert!(error.is_null());
            assert_eq!(scope, PurrdfShaclChangeScopeKind::Bounded as i32);
            assert_eq!(focus_nodes, 1);
            assert!(reason.is_null(), "a bounded scope names no reason");
            assert!(text_of(buffer).contains("DatatypeConstraintComponent"));
            purrdf_buffer_free(buffer);
        }

        // The fallback arm, through the same pointers: a shapes graph reading
        // through query text reports EVERYTHING, with the reason a host needs to
        // get incremental validation back.
        const SPARQL_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
            @prefix ex: <http://example.org/> .\n\
            ex:PersonShape a sh:NodeShape ;\n\
              sh:targetClass ex:Person ;\n\
              sh:sparql [ a sh:SPARQLConstraint ;\n\
                sh:message \"every person needs a name\" ;\n\
                sh:select \"\"\"SELECT $this WHERE { FILTER NOT EXISTS \
                  { $this <http://example.org/name> ?n } }\"\"\" ] .\n";
        let sparql_shapes = std::ffi::CString::new(SPARQL_SHAPES).expect("no interior NUL");
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut reason: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut scope: i32 = -1;
        let mut focus_nodes: usize = usize::MAX;
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        unsafe {
            let status = purrdf_shacl_validate_changes_to_sarif(
                sparql_shapes.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                data.as_ptr(),
                added.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                false,
                &raw mut buffer,
                &raw mut scope,
                &raw mut focus_nodes,
                &raw mut reason,
                &raw mut error,
            );
            assert_eq!(status, PurrdfStatus::Ok as i32);
            assert_eq!(scope, PurrdfShaclChangeScopeKind::Everything as i32);
            assert_eq!(focus_nodes, 0, "a fallback covers no COUNT");
            assert!(!reason.is_null(), "a fallback names what made it one");
            assert_ne!(text_of(reason), "");
            // The fallback validated the WHOLE graph, so the untouched node is in
            // the log too — which is what makes it a full validation.
            assert!(text_of(buffer).contains("alice"));
            purrdf_buffer_free(reason);
            purrdf_buffer_free(buffer);
        }

        // A malformed change document is refused, and writes no buffer. The
        // neighbouring well-formed call above succeeded, so this is strictness
        // rather than a route that cannot run.
        let malformed = std::ffi::CString::new("@@@ not n-triples").expect("no interior NUL");
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut reason: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut scope: i32 = -1;
        let mut focus_nodes: usize = usize::MAX;
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        unsafe {
            let status = purrdf_shacl_validate_changes_to_sarif(
                shapes.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                data.as_ptr(),
                malformed.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                false,
                &raw mut buffer,
                &raw mut scope,
                &raw mut focus_nodes,
                &raw mut reason,
                &raw mut error,
            );
            assert_eq!(status, PurrdfStatus::ParseError as i32);
            assert!(
                buffer.is_null() && reason.is_null(),
                "a refused call writes no buffer"
            );
            assert!(!error.is_null());
            purrdf_error_free(error);
        }
    }

    // A shapes graph with a `sh:TripleRule` typing every `ex:Person` an `ex:adult`.
    const RULE_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        ex:PersonRule a sh:NodeShape ;\n\
          sh:targetClass ex:Person ;\n\
          sh:rule [ a sh:TripleRule ;\n\
            sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .\n";

    const RULE_DATA: &str = "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";

    #[test]
    fn entail_emits_materialized_ntriples() {
        let bytes = entail_outcome(
            RULE_SHAPES,
            None,
            None,
            RULE_DATA,
            &[],
            RuleLimits::default(),
        )
        .expect("entailment produced");
        let text = bytes.ntriples;
        assert_eq!(bytes.diagnostics, []);
        assert!(text.contains(
            "<http://example.org/alice> <http://example.org/adult> <http://example.org/yes> ."
        ));
        assert!(text.contains(
            "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> ."
        ));
    }

    #[test]
    fn entail_malformed_shapes_is_an_error() {
        assert!(
            entail_outcome(
                "@@@ not turtle",
                None,
                None,
                RULE_DATA,
                &[],
                RuleLimits::default()
            )
            .is_err()
        );
    }

    #[test]
    fn a_product_round_trips_to_the_same_verdict() {
        let product = encode_product_bytes(SHAPES, None, None, &[]).expect("product encoded");
        certify_product_bytes(&product).expect("certified");
        let described = String::from_utf8(open_product_bytes(&product).expect("opened"))
            .expect("the description is UTF-8");
        assert!(described.contains("stage-known true\n"));

        // Admitting the product and parsing the shapes graph are two routes to ONE
        // verdict, which is the property a cache is only allowed to have.
        let via_product = admit_product_bytes(&product, DATA).expect("validated via product");
        let via_document = validate_to_sarif_bytes(SHAPES, None, None, DATA, &[], &[], false)
            .expect("validated directly");
        assert_eq!(via_product, via_document);

        // Rebuilding a CURRENT product reaches the byte-identical verdict too: the
        // forward-compatibility door must not be a second, divergent answer.
        let via_rebuild = rebuild_product_bytes(&product, DATA).expect("rebuilt via product");
        assert_eq!(via_rebuild, via_product);
    }

    #[test]
    fn a_refused_product_carries_its_dimension_through_the_error_handle() {
        let mut wrong_magic =
            encode_product_bytes(SHAPES, None, None, &[]).expect("product encoded");
        wrong_magic[0] = b'X';

        let refusal = admit_product_bytes(&wrong_magic, DATA).expect_err("a foreign magic refuses");
        let error = Box::into_raw(Box::new(product_error(&refusal)));
        unsafe {
            assert_eq!(
                purrdf_error_code(error),
                PurrdfStatus::ShapesProductError as i32
            );
            let dimension = purrdf_shapes_product_error_dimension(error);
            assert!(!dimension.is_null(), "a product refusal names a dimension");
            assert_eq!(
                std::ffi::CStr::from_ptr(dimension).to_str().expect("utf8"),
                "magic"
            );
            purrdf_error_free(error);
        }

        // An error from another boundary names NO dimension — not an empty string.
        let other = Box::into_raw(Box::new(PurrdfError::new(PurrdfStatus::ParseError, "boom")));
        unsafe {
            assert!(purrdf_shapes_product_error_dimension(other).is_null());
            purrdf_error_free(other);
            assert!(purrdf_shapes_product_error_dimension(std::ptr::null()).is_null());
        }

        // The neighbouring VALID case still succeeds — a refusal is a claim too.
        let product = encode_product_bytes(SHAPES, None, None, &[]).expect("product encoded");
        admit_product_bytes(&product, DATA).expect("the unmodified product still validates");
    }

    /// A second shapes graph over different classes, so the two products genuinely
    /// carry two input bindings.
    const OTHER_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        ex:WidgetShape a sh:NodeShape ;\n\
          sh:targetClass ex:Widget ;\n\
          sh:property [ sh:path ex:maker ; sh:minCount 1 ] .\n";

    /// The `identity-digest` a product renders — read the way a C host reads it, out of
    /// `purrdf_shapes_product_open`'s own description.
    fn rendered_selector(product: &[u8]) -> String {
        String::from_utf8(open_product_bytes(product).expect("opened"))
            .expect("the description is UTF-8")
            .lines()
            .find_map(|line| line.strip_prefix("identity-digest ").map(ToOwned::to_owned))
            .expect("the description carries an identity digest")
    }

    #[test]
    fn a_product_that_is_not_the_expected_one_is_refused() {
        let held = encode_product_bytes(SHAPES, None, None, &[]).expect("product encoded");
        let wanted = rendered_selector(
            &encode_product_bytes(OTHER_SHAPES, None, None, &[]).expect("encoded"),
        );
        assert_ne!(wanted, rendered_selector(&held));

        let expected = purrdf_validate::parse_identity_digest(&wanted).expect("selector parses");
        let refusal = admit_product_bytes_expecting(&held, DATA, &expected)
            .expect_err("the product held is not the product required");
        let error = Box::into_raw(Box::new(product_error(&refusal)));
        unsafe {
            let dimension = purrdf_shapes_product_error_dimension(error);
            assert!(!dimension.is_null());
            assert_eq!(
                std::ffi::CStr::from_ptr(dimension).to_str().expect("utf8"),
                "shapes-graph",
            );
            purrdf_error_free(error);
        }

        // The gap this closes: unbound, the very same bytes validate.
        admit_product_bytes(&held, DATA).expect("an unbound admit cannot ask which product");
    }

    #[test]
    fn a_product_required_to_be_itself_validates_identically() {
        let product = encode_product_bytes(SHAPES, None, None, &[]).expect("product encoded");
        let own = purrdf_validate::parse_identity_digest(&rendered_selector(&product))
            .expect("the rendering is accepted back");

        let bound = admit_product_bytes_expecting(&product, DATA, &own)
            .expect("a product required to be itself validates");
        let unbound = admit_product_bytes(&product, DATA).expect("validated");
        assert_eq!(
            bound, unbound,
            "stating which product you meant changes the door, not the answer",
        );
    }

    /// The exported entry point, driven through pointers exactly as a C host drives it:
    /// the satisfied expectation yields a SARIF buffer and `PURRDF_STATUS_OK`, and a
    /// selector that is not a digest yields `PURRDF_STATUS_INVALID_ARGUMENT` with NO
    /// dimension — no product was opened, so nothing may be blamed on one.
    #[test]
    fn the_exported_entry_point_binds_a_restore_through_pointers() {
        use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};

        let product = encode_product_bytes(SHAPES, None, None, &[]).expect("product encoded");
        let own = std::ffi::CString::new(rendered_selector(&product)).expect("no interior NUL");
        let data = std::ffi::CString::new(DATA).expect("no interior NUL");

        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        unsafe {
            let status = purrdf_shapes_product_admit_expecting(
                product.as_ptr(),
                product.len(),
                data.as_ptr(),
                own.as_ptr(),
                &raw mut buffer,
                &raw mut error,
            );
            assert_eq!(status, PurrdfStatus::Ok as i32);
            assert!(error.is_null());

            let mut ptr: *const u8 = std::ptr::null();
            let mut len: usize = 0;
            assert_eq!(
                purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len),
                PurrdfStatus::Ok as i32
            );
            let sarif = std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).expect("utf8");
            assert!(sarif.contains("\"version\": \"2.1.0\""));
            purrdf_buffer_free(buffer);
        }

        let mistyped = std::ffi::CString::new("not-a-digest").expect("no interior NUL");
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        unsafe {
            let status = purrdf_shapes_product_admit_expecting(
                product.as_ptr(),
                product.len(),
                data.as_ptr(),
                mistyped.as_ptr(),
                &raw mut buffer,
                &raw mut error,
            );
            assert_eq!(status, PurrdfStatus::InvalidArgument as i32);
            assert!(buffer.is_null(), "a refused call writes no buffer");
            assert!(!error.is_null());
            assert!(
                purrdf_shapes_product_error_dimension(error).is_null(),
                "no product was opened, so no admission dimension names this",
            );
            purrdf_error_free(error);
        }
    }

    /// The rebuild path answers the same "is this the product I asked for?"
    /// question `admit_expecting` does: a product whose binding is not the one
    /// required is refused on `shapes-graph` even though its stage id is one this
    /// build knows and the unbound `rebuild` would otherwise happily re-derive it.
    #[test]
    fn a_rebuilt_product_that_is_not_the_expected_one_is_refused() {
        let held = encode_product_bytes(SHAPES, None, None, &[]).expect("product encoded");
        let wanted = rendered_selector(
            &encode_product_bytes(OTHER_SHAPES, None, None, &[]).expect("encoded"),
        );
        assert_ne!(wanted, rendered_selector(&held));

        let expected = purrdf_validate::parse_identity_digest(&wanted).expect("selector parses");
        let refusal = rebuild_product_bytes_expecting(&held, DATA, &expected)
            .expect_err("the product held is not the product required");
        let error = Box::into_raw(Box::new(product_error(&refusal)));
        unsafe {
            let dimension = purrdf_shapes_product_error_dimension(error);
            assert!(!dimension.is_null());
            assert_eq!(
                std::ffi::CStr::from_ptr(dimension).to_str().expect("utf8"),
                "shapes-graph",
            );
            purrdf_error_free(error);
        }

        // The gap this closes: the unbound rebuild restores the very same bytes,
        // because nothing in them states which product was meant.
        rebuild_product_bytes(&held, DATA).expect("an unbound rebuild cannot ask which product");
    }

    /// The neighbouring VALID case: a product required to be ITSELF still
    /// rebuilds, and reaches the byte-identical report the unbound rebuild and
    /// the bound `admit_expecting` both reach.
    #[test]
    fn a_rebuilt_product_required_to_be_itself_validates_identically() {
        let product = encode_product_bytes(SHAPES, None, None, &[]).expect("product encoded");
        let own = purrdf_validate::parse_identity_digest(&rendered_selector(&product))
            .expect("the rendering is accepted back");

        let bound_rebuild = rebuild_product_bytes_expecting(&product, DATA, &own)
            .expect("a product required to be itself rebuilds");
        let unbound_rebuild =
            rebuild_product_bytes(&product, DATA).expect("the unbound rebuild validates");
        assert_eq!(
            bound_rebuild, unbound_rebuild,
            "stating which product you meant changes the door, not the answer",
        );

        let bound_admit = admit_product_bytes_expecting(&product, DATA, &own)
            .expect("a product required to be itself admits");
        assert_eq!(
            bound_rebuild, bound_admit,
            "choosing to re-derive rather than restore the memo must not change the answer",
        );
    }

    /// The exported entry point, driven through pointers exactly as a C host
    /// drives it: the satisfied expectation yields a SARIF buffer and
    /// `PURRDF_STATUS_OK`, and a selector that is not a digest yields
    /// `PURRDF_STATUS_INVALID_ARGUMENT` with NO dimension — no product was
    /// opened, so nothing may be blamed on one.
    #[test]
    fn the_exported_rebuild_entry_point_binds_a_restore_through_pointers() {
        use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};

        let product = encode_product_bytes(SHAPES, None, None, &[]).expect("product encoded");
        let own = std::ffi::CString::new(rendered_selector(&product)).expect("no interior NUL");
        let data = std::ffi::CString::new(DATA).expect("no interior NUL");

        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        unsafe {
            let status = purrdf_shapes_product_rebuild_expecting(
                product.as_ptr(),
                product.len(),
                data.as_ptr(),
                own.as_ptr(),
                &raw mut buffer,
                &raw mut error,
            );
            assert_eq!(status, PurrdfStatus::Ok as i32);
            assert!(error.is_null());

            let mut ptr: *const u8 = std::ptr::null();
            let mut len: usize = 0;
            assert_eq!(
                purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len),
                PurrdfStatus::Ok as i32
            );
            let sarif = std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).expect("utf8");
            assert!(sarif.contains("\"version\": \"2.1.0\""));
            purrdf_buffer_free(buffer);
        }

        let mistyped = std::ffi::CString::new("not-a-digest").expect("no interior NUL");
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        unsafe {
            let status = purrdf_shapes_product_rebuild_expecting(
                product.as_ptr(),
                product.len(),
                data.as_ptr(),
                mistyped.as_ptr(),
                &raw mut buffer,
                &raw mut error,
            );
            assert_eq!(status, PurrdfStatus::InvalidArgument as i32);
            assert!(buffer.is_null(), "a refused call writes no buffer");
            assert!(!error.is_null());
            assert!(
                purrdf_shapes_product_error_dimension(error).is_null(),
                "no product was opened, so no admission dimension names this",
            );
            purrdf_error_free(error);
        }
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

    /// Read a buffer's bytes as UTF-8 and free it.
    ///
    /// # Safety
    /// `buffer` must be a live buffer written by a successful call.
    unsafe fn take_text(buffer: *mut PurrdfBuffer) -> String {
        use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};
        unsafe {
            let mut ptr: *const u8 = std::ptr::null();
            let mut len = 0usize;
            assert_eq!(
                purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len),
                PurrdfStatus::Ok as i32
            );
            let text = std::str::from_utf8(std::slice::from_raw_parts(ptr, len))
                .expect("utf8")
                .to_owned();
            purrdf_buffer_free(buffer);
            text
        }
    }

    /// The message of a failed call's error, freeing it.
    ///
    /// # Safety
    /// `error` must be a live error written by a failed call.
    unsafe fn take_error(error: *mut PurrdfError) -> String {
        unsafe {
            let message = std::ffi::CStr::from_ptr(crate::error::purrdf_error_message(error))
                .to_string_lossy()
                .into_owned();
            purrdf_error_free(error);
            message
        }
    }

    /// The approved W3C test `core/node/in-002`'s shapes (an empty `sh:in` list) through
    /// the three C entry points: every run reports the `in-minListLength` diagnostic — the
    /// SARIF log as a note-level tool-execution notification, the rules and entailment
    /// runs in `out_diagnostics` — while the verdict and the results are the
    /// specification's. The neighbour whose list has a member reports none.
    #[test]
    fn capi_mandatory_diagnostics() {
        let shapes = |members: &str| {
            std::ffi::CString::new(format!(
                "@prefix ex: <http://example.com/ns#> .\n\
                 @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
                 @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
                 @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
                 ex:TestShape rdf:type rdfs:Class , sh:NodeShape ; sh:in {members} ;\n\
                   sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:seen ; \
                   sh:object ex:yes ] .\n"
            ))
            .expect("no NUL")
        };
        let data = std::ffi::CString::new(
            "<http://example.com/ns#Instance> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.com/ns#TestShape> .\n",
        )
        .expect("no NUL");
        let expected = "diagnostic in-minListLength <http://example.com/ns#TestShape>\n";
        for (members, diagnostics) in [
            ("()", expected),
            ("( <http://example.com/ns#Instance> )", ""),
        ] {
            let shapes = shapes(members);
            let mut sarif: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut entailed: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut entail_diagnostics: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut inferred: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut rules_diagnostics: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString, NULL, or a writable local.
            let (log, closure, from_entail, from_rules) = unsafe {
                let status = purrdf_shacl_validate_to_sarif(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    data.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    false,
                    &raw mut sarif,
                    &raw mut error,
                );
                assert_eq!(status, PurrdfStatus::Ok as i32, "{}", take_error(error));
                let status = purrdf_shacl_entail_to_ntriples(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    data.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    &raw mut entailed,
                    &raw mut entail_diagnostics,
                    &raw mut error,
                );
                assert_eq!(status, PurrdfStatus::Ok as i32, "{}", take_error(error));
                let status = purrdf_shacl_apply_rules(
                    data.as_ptr(),
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut inferred,
                    std::ptr::null_mut(),
                    &raw mut rules_diagnostics,
                    &raw mut error,
                );
                assert_eq!(status, PurrdfStatus::Ok as i32, "{}", take_error(error));
                let _ = take_text(inferred);
                (
                    take_text(sarif),
                    take_text(entailed),
                    take_text(entail_diagnostics),
                    take_text(rules_diagnostics),
                )
            };
            assert_eq!(from_entail, diagnostics, "{members}");
            assert_eq!(from_rules, diagnostics, "{members}");
            assert!(
                closure.contains("<http://example.com/ns#seen>"),
                "{closure}"
            );
            let log: serde_json::Value = serde_json::from_str(&log).expect("SARIF JSON");
            let run = &log["runs"][0];
            let notifications = &run["invocations"][0]["toolExecutionNotifications"];
            if diagnostics.is_empty() {
                assert_eq!(run["properties"]["shaclConforms"], true, "{log:#}");
                assert!(run.get("invocations").is_none(), "{log:#}");
            } else {
                assert_eq!(run["properties"]["shaclConforms"], false, "{log:#}");
                assert_eq!(run["results"].as_array().map(Vec::len), Some(1), "{log:#}");
                assert_eq!(notifications[0]["level"], "note", "{log:#}");
                assert_eq!(notifications[0]["descriptor"]["id"], "in-minListLength");
                assert_eq!(
                    notifications[0]["locations"][0]["logicalLocations"][0]["name"],
                    "<http://example.com/ns#TestShape>"
                );
            }
        }
    }

    /// `purrdf_shacl_entail_to_ntriples`' four rule-evaluation limits reach the engine
    /// through the exported symbol, and each refusal names THIS host's parameter; the same
    /// call with every limit raised materializes the whole closure.
    #[test]
    fn capi_entail_limits() {
        const COUNTER: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
            @prefix ex: <http://example.org/ns#> .\n\
            ex:Counter a sh:NodeShape ; sh:targetSubjectsOf ex:n ;\n\
              sh:rule [ a sh:SPARQLRule ; sh:construct \"\"\"PREFIX ex: <http://example.org/ns#>\n\
            CONSTRUCT { $this ex:n ?m } WHERE { $this ex:n ?k . FILTER(?k < 5) BIND(?k + 1 AS ?m) }\"\"\" ] .\n";
        let shapes = std::ffi::CString::new(COUNTER).expect("no NUL");
        let data = std::ffi::CString::new(
            "<http://example.org/ns#a> <http://example.org/ns#n> \
             \"1\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
        )
        .expect("no NUL");
        let entail = |limits: [Option<u64>; 4]| {
            let pointer =
                |limit: &Option<u64>| limit.as_ref().map_or(std::ptr::null(), std::ptr::from_ref);
            let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString, a live local, NULL, or writable.
            unsafe {
                let status = purrdf_shacl_entail_to_ntriples(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    data.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    pointer(&limits[0]),
                    pointer(&limits[1]),
                    pointer(&limits[2]),
                    pointer(&limits[3]),
                    &raw mut buffer,
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                if status == PurrdfStatus::Ok as i32 {
                    Ok(take_text(buffer))
                } else {
                    Err(take_error(error))
                }
            }
        };
        for (pick, knob) in [
            "purrdf_shacl_entail_to_ntriples's max_term_generating_rounds",
            "purrdf_shacl_entail_to_ntriples's max_generated_terms",
            "purrdf_shacl_entail_to_ntriples's max_stored_facts",
            "purrdf_shacl_entail_to_ntriples's max_join_steps",
        ]
        .into_iter()
        .enumerate()
        {
            let mut limits = [None; 4];
            limits[pick] = Some(1);
            let refused = entail(limits).expect_err("a limit of one refuses the counter");
            assert!(refused.ends_with(knob), "{refused}");
        }
        let closed = entail([Some(64), Some(64), Some(64), Some(4_096)])
            .expect("raised limits admit the counter");
        assert!(
            closed.contains(
                "<http://example.org/ns#a> <http://example.org/ns#n> \
                 \"5\"^^<http://www.w3.org/2001/XMLSchema#integer> ."
            ),
            "{closed}"
        );
    }

    /// `shapes_graph_iri` on the two rules entry points: named, a SPARQL rule's
    /// `$shapesGraph` is the IRI and `GRAPH $shapesGraph` reads the shapes graph; NULL, it
    /// is an ordinary variable. Beside `srl` it is a `ParseError`, and the rule set alone
    /// runs.
    #[test]
    fn capi_rules_shapes_graph() {
        use std::ffi::CString;

        let shapes = CString::new(
            r#"@prefix ex: <http://example.org/ns#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
ex:S a sh:NodeShape ; sh:targetClass ex:Person ; ex:marker ex:secret ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """
    CONSTRUCT { $this <http://example.org/ns#shapesGraph> ?g }
    WHERE { BIND (COALESCE($shapesGraph, <http://example.org/ns#none>) AS ?g) }""" ] ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """
    CONSTRUCT { $this <http://example.org/ns#marked> ?m }
    WHERE { GRAPH $shapesGraph { $currentShape <http://example.org/ns#marker> ?m } }""" ] .
"#,
        )
        .expect("no NUL");
        let data = CString::new(
            "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.org/ns#Person> .\n",
        )
        .expect("no NUL");
        let srl = CString::new(
            "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:tagged true } WHERE { ?x a ex:Person }\n",
        )
        .expect("no NUL");
        let graph = CString::new("http://example.org/shapes-graph").expect("no NUL");
        let named = "<http://example.org/ns#alice> <http://example.org/ns#shapesGraph> \
            <http://example.org/shapes-graph> .";
        let unnamed = "<http://example.org/ns#alice> <http://example.org/ns#shapesGraph> \
            <http://example.org/ns#none> .";
        let marked = "<http://example.org/ns#alice> <http://example.org/ns#marked> \
            <http://example.org/ns#secret> .";
        let rules = |shapes_ttl: *const c_char, srl: *const c_char, graph: *const c_char| {
            let mut inferred: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString, NULL, or a writable local.
            unsafe {
                let status = purrdf_shacl_apply_rules(
                    data.as_ptr(),
                    shapes_ttl,
                    std::ptr::null(),
                    graph,
                    srl,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut inferred,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                if status == PurrdfStatus::Ok as i32 {
                    Ok(take_text(inferred))
                } else {
                    Err((status, take_error(error)))
                }
            }
        };
        let entail = |graph: *const c_char| {
            let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString, NULL, or a writable local.
            unsafe {
                let status = purrdf_shacl_entail_to_ntriples(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    graph,
                    data.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    &raw mut buffer,
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                assert_eq!(status, PurrdfStatus::Ok as i32, "{}", take_error(error));
                take_text(buffer)
            }
        };
        let with = rules(shapes.as_ptr(), std::ptr::null(), graph.as_ptr()).expect("runs");
        assert!(with.contains(named) && with.contains(marked), "{with}");
        let without = rules(shapes.as_ptr(), std::ptr::null(), std::ptr::null()).expect("runs");
        assert!(
            without.contains(unnamed) && !without.contains(marked),
            "{without}"
        );
        let entailed = entail(graph.as_ptr());
        assert!(
            entailed.contains(named) && entailed.contains(marked),
            "{entailed}"
        );
        let plain = entail(std::ptr::null());
        assert!(
            plain.contains(unnamed) && !plain.contains(marked),
            "{plain}"
        );
        let (status, refused) = rules(std::ptr::null(), srl.as_ptr(), graph.as_ptr())
            .expect_err("no shapes graph beside srl");
        assert_eq!(status, PurrdfStatus::ParseError as i32, "{refused}");
        assert!(refused.contains("has no shapes graph"), "{refused}");
        let tagged = rules(std::ptr::null(), srl.as_ptr(), std::ptr::null()).expect("runs");
        assert!(
            tagged.contains("<http://example.org/ns#tagged>"),
            "{tagged}"
        );
    }

    /// `purrdf_shacl_apply_rules` across the boundary: the inference graph alone, the
    /// proof exactly when `out_proof` is non-NULL, the round limit refusing at 3 and
    /// completing at 4 (and at the NULL default), and SPARQL 1.2 RL text.
    #[test]
    fn capi_apply_rules() {
        use std::ffi::CString;

        let data = CString::new(TOOLS_DATA).expect("no NUL");
        let shapes = CString::new(TOOLS_SHAPES).expect("no NUL");
        let run = |shapes: *const c_char,
                   srl: *const c_char,
                   limit: Option<u64>,
                   explain: bool|
         -> Result<(String, Option<String>), String> {
            let mut inferred: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut proof: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            let limit_ptr = limit.as_ref().map_or(std::ptr::null(), std::ptr::from_ref);
            // SAFETY: every pointer is a live CString, a readable local, NULL, or a
            // writable local.
            unsafe {
                let status = purrdf_shacl_apply_rules(
                    data.as_ptr(),
                    shapes,
                    std::ptr::null(),
                    std::ptr::null(),
                    srl,
                    std::ptr::null(),
                    limit_ptr,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut inferred,
                    if explain {
                        &raw mut proof
                    } else {
                        std::ptr::null_mut()
                    },
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                if status != PurrdfStatus::Ok as i32 {
                    return Err(take_error(error));
                }
                let proof = (!proof.is_null()).then(|| take_text(proof));
                Ok((take_text(inferred), proof))
            }
        };
        let (graph, proof) = run(shapes.as_ptr(), std::ptr::null(), None, false).expect("runs");
        assert_eq!(graph, tools_inference());
        assert_eq!(proof, None);
        let (_, proof) = run(shapes.as_ptr(), std::ptr::null(), None, true).expect("runs");
        assert_eq!(proof.expect("asked for").matches("derived ").count(), 5);
        let refused = run(shapes.as_ptr(), std::ptr::null(), Some(3), false)
            .expect_err("three rounds are too few");
        assert!(refused.contains("past the limit of 3"), "{refused}");
        assert!(
            refused.ends_with(
                "raise the limit with purrdf_shacl_apply_rules's max_term_generating_rounds"
            ),
            "{refused}"
        );
        let (graph, _) = run(shapes.as_ptr(), std::ptr::null(), Some(4), false).expect("runs");
        assert_eq!(graph, tools_inference());

        let srl = CString::new(
            "PREFIX ex: <http://example.org/ns#>\n\
             RULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\nDATA { ex:d ex:q 2 }\n",
        )
        .expect("no NUL");
        let (graph, proof) = run(std::ptr::null(), srl.as_ptr(), None, true).expect("runs");
        assert_eq!(
            graph,
            "<http://example.org/ns#a> <http://example.org/ns#q> \
             \"1\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n\
             <http://example.org/ns#d> <http://example.org/ns#q> \
             \"2\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n"
        );
        assert!(proof.expect("asked for").contains("  data-block\n"));
        let both = run(shapes.as_ptr(), srl.as_ptr(), None, false).expect_err("two sources");
        assert!(both.contains("two rule sources"), "{both}");
    }

    /// The generated-term budget crosses the C boundary as its own nullable parameter:
    /// the tools rule set adds six terms, so 5 is refused naming this ABI's parameter
    /// and 6 admits it.
    #[test]
    fn capi_apply_rules_takes_a_generated_term_budget() {
        use std::ffi::CString;

        let data = CString::new(TOOLS_DATA).expect("no NUL");
        let shapes = CString::new(TOOLS_SHAPES).expect("no NUL");
        let run = |budget: u64| -> Result<String, String> {
            let mut inferred: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString, NULL, a readable local, or a
            // writable local.
            unsafe {
                let status = purrdf_shacl_apply_rules(
                    data.as_ptr(),
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    &raw const budget,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut inferred,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                if status != PurrdfStatus::Ok as i32 {
                    return Err(take_error(error));
                }
                Ok(take_text(inferred))
            }
        };
        let refused = run(5).expect_err("five terms are too few");
        assert!(
            refused.contains("past the budget of 5 (the caller's budget)"),
            "{refused}"
        );
        assert!(
            refused
                .ends_with("raise the budget with purrdf_shacl_apply_rules's max_generated_terms"),
            "{refused}"
        );
        assert_eq!(run(6).expect("six terms suffice"), tools_inference());
    }

    /// The stored-fact and join-step limits cross the C boundary as their own nullable
    /// parameters: a refusal names this ABI's parameter and the numbers, a limit of exactly
    /// the store the run needs admits it with the default inference graph, and one fewer
    /// refuses it.
    #[test]
    fn capi_apply_rules_takes_the_capacity_limits() {
        use std::ffi::CString;

        let data = CString::new(TOOLS_DATA).expect("no NUL");
        let shapes = CString::new(TOOLS_SHAPES).expect("no NUL");
        let run = |facts: Option<u64>, steps: Option<u64>| -> Result<String, String> {
            let mut inferred: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            let facts_ptr = facts.as_ref().map_or(std::ptr::null(), std::ptr::from_ref);
            let steps_ptr = steps.as_ref().map_or(std::ptr::null(), std::ptr::from_ref);
            // SAFETY: every pointer is a live CString, NULL, a readable local, or a
            // writable local.
            unsafe {
                let status = purrdf_shacl_apply_rules(
                    data.as_ptr(),
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    facts_ptr,
                    steps_ptr,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut inferred,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                if status != PurrdfStatus::Ok as i32 {
                    return Err(take_error(error));
                }
                Ok(take_text(inferred))
            }
        };
        let mut limit = 1_u64;
        let admitted = loop {
            match run(Some(limit), None) {
                Ok(graph) => break graph,
                Err(refused) => {
                    assert!(
                        refused.contains(&format!("{limit} permitted (the caller's limit)"))
                            && refused.ends_with(
                                "raise it with purrdf_shacl_apply_rules's max_stored_facts"
                            ),
                        "{refused}"
                    );
                    let observed: u64 = refused
                        .split("the rules exceeded the stored-fact limit: ")
                        .nth(1)
                        .and_then(|tail| tail.split(' ').next())
                        .and_then(|count| count.parse().ok())
                        .expect("an observed count");
                    assert!(observed > limit, "{refused}");
                    limit = observed;
                }
            }
        };
        assert_eq!(admitted, tools_inference());
        assert!(run(Some(limit - 1), None).is_err(), "one fact short");
        let steps = run(None, Some(1)).expect_err("one join step");
        assert!(
            steps.ends_with("raise it with purrdf_shacl_apply_rules's max_join_steps"),
            "{steps}"
        );
    }

    /// A SPARQL 1.2 RL rule set's `IMPORTS` resolve from the import table across the C
    /// boundary: the imported rule's inference appears, an unsupplied import and an unused
    /// entry are refused with the shared boundary's text.
    #[test]
    fn capi_apply_rules_resolves_srl_imports() {
        use std::ffi::CString;

        let data = CString::new(TOOLS_DATA).expect("no NUL");
        let importing = CString::new(
            "PREFIX ex: <http://example.org/ns#>\nIMPORTS <http://example.org/more>\n\
             RULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\n",
        )
        .expect("no NUL");
        let lone = CString::new(
            "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\n",
        )
        .expect("no NUL");
        let iri = CString::new("http://example.org/more").expect("no NUL");
        let imported = CString::new(
            "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:counted true } WHERE { ?x ex:q ?y }\n",
        )
        .expect("no NUL");
        let iris = [iri.as_ptr()];
        let documents = [imported.as_ptr()];
        let run = |srl: &CString, count: usize| -> Result<String, String> {
            let mut inferred: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString, NULL, a live array of `count`
            // CString pointers, or a writable local.
            unsafe {
                let status = purrdf_shacl_apply_rules(
                    data.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    srl.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    iris.as_ptr(),
                    documents.as_ptr(),
                    count,
                    &raw mut inferred,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                if status != PurrdfStatus::Ok as i32 {
                    return Err(take_error(error));
                }
                Ok(take_text(inferred))
            }
        };
        let graph = run(&importing, 1).expect("resolves");
        assert!(
            graph.contains(
                "<http://example.org/ns#a> <http://example.org/ns#counted> \
                 \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> .\n"
            ),
            "{graph}"
        );
        assert_eq!(
            run(&importing, 0).expect_err("unsupplied"),
            "SPARQL 1.2 RL import <http://example.org/more> failed: no import-table entry \
             supplies the rule set it names, and PurRDF fetches nothing it was not handed; \
             supply that rule set's text under this IRI"
        );
        assert_eq!(
            run(&lone, 1).expect_err("unused"),
            "the SPARQL 1.2 RL rule set's import closure never reaches \
             <http://example.org/more>, so the import table's rule set would be read and never \
             used; remove it"
        );
    }

    /// W3C SHACL 1.0 `sparql/pre-binding/shapesGraph-001` across the C boundary, through
    /// `purrdf_shacl_validate_to_sarif`, `purrdf_shacl_validate_changes_to_sarif`,
    /// `purrdf_shapes_product_encode` (then `purrdf_shapes_product_admit`) and
    /// `purrdf_shacl_lint_shapes`. With `shapes_graph_iri` named: the approved ONE
    /// result. NULL: SHACL 1.2's ordinary, unbound `$shapesGraph`, so the data graph
    /// conforms. A relative IRI with no base is a `ParseError` naming its code.
    #[test]
    fn capi_shapes_graph_001_honours_the_shapes_graph_iri() {
        use std::ffi::CString;

        let path = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/shacl/sparql/pre-binding/shapesGraph-001.ttl"
        ))
        .canonicalize()
        .expect("the vendored W3C test file");
        let base_text = format!("file://{}", path.display());
        let turtle = std::fs::read_to_string(&path).expect("reads");
        let dataset = purrdf_rs::parse_dataset(turtle.as_bytes(), "text/turtle", Some(&base_text))
            .expect("parses");
        let data_text = String::from_utf8(
            purrdf_rs::serialize_dataset(
                &*dataset,
                "application/n-quads",
                purrdf_rs::SerializeGraph::Dataset,
            )
            .expect("serializes"),
        )
        .expect("utf-8");
        let shapes = CString::new(turtle).expect("no NUL");
        let base = CString::new(base_text).expect("no NUL");
        let data = CString::new(data_text).expect("no NUL");
        let empty = CString::new("").expect("no NUL");
        let results = |sarif: &str| -> usize {
            let log: serde_json::Value = serde_json::from_str(sarif).expect("json");
            log["runs"][0]["results"]
                .as_array()
                .expect("a completed run always carries results")
                .len()
        };

        for (graph, expected) in [(Some(&base), 1usize), (None, 0)] {
            let graph_ptr = graph.map_or(std::ptr::null(), |iri| iri.as_ptr());
            let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString, NULL, or a writable local.
            let sarif = unsafe {
                let status = purrdf_shacl_validate_to_sarif(
                    shapes.as_ptr(),
                    base.as_ptr(),
                    graph_ptr,
                    data.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    false,
                    &raw mut buffer,
                    &raw mut error,
                );
                assert_eq!(status, PurrdfStatus::Ok as i32);
                take_text(buffer)
            };
            assert_eq!(results(&sarif), expected, "validate, {graph:?}");

            let mut scope = 0i32;
            let mut focus_nodes = 0usize;
            let mut reason: *mut PurrdfBuffer = std::ptr::null_mut();
            // SAFETY: as above.
            let sarif = unsafe {
                let status = purrdf_shacl_validate_changes_to_sarif(
                    shapes.as_ptr(),
                    base.as_ptr(),
                    graph_ptr,
                    empty.as_ptr(),
                    data.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    false,
                    &raw mut buffer,
                    &raw mut scope,
                    &raw mut focus_nodes,
                    &raw mut reason,
                    &raw mut error,
                );
                assert_eq!(status, PurrdfStatus::Ok as i32);
                if !reason.is_null() {
                    crate::buffer::purrdf_buffer_free(reason);
                }
                take_text(buffer)
            };
            assert_eq!(results(&sarif), expected, "change path, {graph:?}");

            // SAFETY: as above.
            let product = unsafe {
                let status = purrdf_shapes_product_encode(
                    shapes.as_ptr(),
                    base.as_ptr(),
                    graph_ptr,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut buffer,
                    &raw mut error,
                );
                assert_eq!(status, PurrdfStatus::Ok as i32);
                take_bytes(buffer)
            };
            let sarif = admit_product_bytes(&product, data.to_str().expect("utf-8"))
                .expect("the product validates");
            assert_eq!(
                results(std::str::from_utf8(&sarif).expect("utf-8")),
                expected,
                "product, {graph:?}"
            );

            let mut clean = 0i32;
            let mut findings = 0usize;
            // SAFETY: as above.
            unsafe {
                let status = purrdf_shacl_lint_shapes(
                    shapes.as_ptr(),
                    base.as_ptr(),
                    graph_ptr,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut buffer,
                    &raw mut clean,
                    &raw mut findings,
                    &raw mut error,
                );
                assert_eq!(status, PurrdfStatus::Ok as i32);
                drop(take_text(buffer));
            }
        }

        // A relative IRI with no base names no graph; the same IRI against the base
        // names the test file and fires.
        let relative = CString::new("shapesGraph-001.ttl").expect("no NUL");
        let plain = CString::new(
            "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <http://example.org/> .\n\
             ex:S a sh:NodeShape ; sh:targetNode ex:n ;\n\
             sh:sparql [ sh:select \"SELECT $this WHERE { FILTER bound($shapesGraph) }\" ] .\n",
        )
        .expect("no NUL");
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: as above.
        let message = unsafe {
            let status = purrdf_shacl_validate_to_sarif(
                plain.as_ptr(),
                std::ptr::null(),
                relative.as_ptr(),
                empty.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                false,
                &raw mut buffer,
                &raw mut error,
            );
            assert_eq!(status, PurrdfStatus::ParseError as i32);
            assert!(buffer.is_null());
            take_error(error)
        };
        assert!(
            message.starts_with("shapes graph `shapesGraph-001.ttl`: iri-relative-no-base"),
            "{message}"
        );
        let resolved = validate_to_sarif_bytes(
            &turtle_of(&shapes),
            Some(base.to_str().expect("utf-8")),
            Some("shapesGraph-001.ttl"),
            data.to_str().expect("utf-8"),
            &[],
            &[],
            false,
        )
        .expect("resolves against the base");
        assert_eq!(results(std::str::from_utf8(&resolved).expect("utf-8")), 1);
    }

    fn turtle_of(shapes: &std::ffi::CString) -> String {
        shapes.to_str().expect("utf-8").to_owned()
    }

    /// The bytes of a live buffer, freeing it.
    unsafe fn take_bytes(buffer: *mut PurrdfBuffer) -> Vec<u8> {
        use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};
        unsafe {
            let mut ptr: *const u8 = std::ptr::null();
            let mut len = 0usize;
            assert_eq!(
                purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len),
                PurrdfStatus::Ok as i32
            );
            let bytes = std::slice::from_raw_parts(ptr, len).to_vec();
            purrdf_buffer_free(buffer);
            bytes
        }
    }

    /// `purrdf_shacl_check_rules` across the boundary: each level answers its own
    /// question with the one-line summary, a check refuses by stage, the `IMPORTS` table
    /// resolves, and a level no `PurrdfSrlCheckLevel` names is refused.
    #[test]
    fn capi_check_rules() {
        use std::ffi::CString;

        let run = |srl: &str, level: i32, count: usize| -> Result<String, String> {
            let srl = CString::new(srl).expect("no NUL");
            let iri = CString::new("http://example.org/more").expect("no NUL");
            let imported = CString::new(
                "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:counted true } WHERE { ?x ex:q ?y }\n",
            )
            .expect("no NUL");
            let iris = [iri.as_ptr()];
            let documents = [imported.as_ptr()];
            let mut summary: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString, NULL, a live array of `count`
            // CString pointers, or a writable local.
            unsafe {
                let status = purrdf_shacl_check_rules(
                    srl.as_ptr(),
                    std::ptr::null(),
                    level,
                    iris.as_ptr(),
                    documents.as_ptr(),
                    count,
                    &raw mut summary,
                    &raw mut error,
                );
                if status != PurrdfStatus::Ok as i32 {
                    assert!(summary.is_null(), "a refusal writes no summary");
                    return Err(take_error(error));
                }
                Ok(take_text(summary))
            }
        };
        // The discriminants are ABI: append-only, never renumbered.
        assert_eq!(PurrdfSrlCheckLevel::Syntax as i32, 0);
        assert_eq!(PurrdfSrlCheckLevel::WellFormed as i32, 1);
        assert_eq!(PurrdfSrlCheckLevel::Stratified as i32, 2);
        let stratified = PurrdfSrlCheckLevel::Stratified as i32;
        let importing = "PREFIX ex: <http://example.org/ns#>\nIMPORTS <http://example.org/more>\n\
             RULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\n";
        assert_eq!(
            run(importing, stratified, 1).expect("resolved"),
            "SPARQL 1.2 RL rule set is well formed and stratified (level stratified): 2 rules, \
             0 data triples, 1 imported rule set, 1 stratum, no VERSION"
        );
        assert!(
            run(importing, stratified, 0)
                .expect_err("unsupplied")
                .starts_with("SPARQL 1.2 RL import <http://example.org/more> failed")
        );

        let cyclic = "PREFIX ex: <http://example.org/ns#>\n\
            RULE { ?x ex:p ex:z } WHERE { ?x ex:q ex:o NOT { ?x ex:p ex:z } }\n";
        let acyclic = "PREFIX ex: <http://example.org/ns#>\n\
            RULE { ?x ex:p ex:z } WHERE { ?x ex:q ex:o NOT { ?x ex:r ex:z } }\n";
        for (level, name) in [
            (PurrdfSrlCheckLevel::Syntax, "syntax"),
            (PurrdfSrlCheckLevel::WellFormed, "well-formed"),
        ] {
            let summary = run(cyclic, level as i32, 0).expect(name);
            assert!(summary.contains(&format!("(level {name})")), "{summary}");
        }
        assert!(
            run(cyclic, stratified, 0)
                .expect_err("unstratifiable")
                .contains("is not stratifiable")
        );
        assert!(run(acyclic, stratified, 0).is_ok());
        assert!(
            run("RULE {", PurrdfSrlCheckLevel::Syntax as i32, 0)
                .expect_err("not SRL")
                .starts_with("SPARQL 1.2 RL syntax error")
        );
        assert!(
            run(acyclic, 3, 0)
                .expect_err("no such level")
                .contains("is not a PurrdfSrlCheckLevel")
        );
    }

    /// `purrdf_shacl_eval_node_expr` across the boundary: a `sh:sparqlExpr` node
    /// natively with its prefixes, a labelled blank node reading a `NAME=TERM` binding,
    /// an unknown label refused, and a NULL scope array with a non-zero count refused
    /// as a `NullPointer`.
    #[test]
    fn capi_eval_node_expr() {
        use std::ffi::CString;

        let shapes = CString::new(TOOLS_SHAPES).expect("no NUL");
        let data = CString::new(TOOLS_DATA).expect("no NUL");
        let focus = CString::new("http://example.org/ns#a").expect("no NUL");
        let run = |expr: &str, scope: &[&str]| -> (i32, String) {
            let expr = CString::new(expr).expect("no NUL");
            let owned: Vec<CString> = scope
                .iter()
                .map(|binding| CString::new(*binding).expect("no NUL"))
                .collect();
            let pointers: Vec<*const c_char> = owned.iter().map(|b| b.as_ptr()).collect();
            let mut terms: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString or a writable local; the array
            // holds exactly `pointers.len()` elements.
            unsafe {
                let status = purrdf_shacl_eval_node_expr(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    data.as_ptr(),
                    expr.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    focus.as_ptr(),
                    if pointers.is_empty() {
                        std::ptr::null()
                    } else {
                        pointers.as_ptr()
                    },
                    pointers.len(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut terms,
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                if status == PurrdfStatus::Ok as i32 {
                    (status, take_text(terms))
                } else {
                    (status, take_error(error))
                }
            }
        };
        assert_eq!(
            run("http://example.org/ns#Tag", &[]),
            (
                PurrdfStatus::Ok as i32,
                "<http://example.org/ns#yes>\n".to_owned()
            )
        );
        assert_eq!(
            run("_:suffix", &["suffix=\"!\"@en"]),
            (PurrdfStatus::Ok as i32, "\"!\"@en\n".to_owned())
        );
        let (status, message) = run("_:nosuch", &[]);
        assert_eq!(status, PurrdfStatus::ParseError as i32);
        assert!(
            message.contains("mentions no blank node _:nosuch"),
            "{message}"
        );

        let expr = CString::new("_:suffix").expect("no NUL");
        let mut terms: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: a NULL array with a non-zero count is refused before it is read.
        let status = unsafe {
            purrdf_shacl_eval_node_expr(
                shapes.as_ptr(),
                std::ptr::null(),
                data.as_ptr(),
                expr.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                focus.as_ptr(),
                std::ptr::null(),
                1,
                std::ptr::null(),
                std::ptr::null(),
                0,
                &raw mut terms,
                std::ptr::null_mut(),
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::NullPointer as i32);
        // SAFETY: the failed call wrote the error.
        unsafe { purrdf_error_free(error) };
    }

    /// A node-expression evaluation writes the shapes graph's mandatory diagnostic to
    /// `out_diagnostics` — one `diagnostic RULE SHAPE` line per shape with an empty `sh:in`
    /// list, the rule first and the shape second, as every host orders the two — and an
    /// empty buffer for the neighbour whose list has a member. The terms are the
    /// expression's either way.
    #[test]
    fn capi_eval_node_expr_reports_the_mandatory_diagnostic() {
        use std::ffi::CString;

        use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};

        unsafe fn text(buffer: *mut PurrdfBuffer) -> String {
            let mut ptr: *const u8 = std::ptr::null();
            let mut len = 0usize;
            // SAFETY: the caller's contract — a live buffer, freed here after the copy.
            unsafe {
                assert_eq!(
                    purrdf_buffer_data(buffer, &raw mut ptr, &raw mut len),
                    PurrdfStatus::Ok as i32
                );
                let owned = std::str::from_utf8(std::slice::from_raw_parts(ptr, len))
                    .expect("utf8")
                    .to_owned();
                purrdf_buffer_free(buffer);
                owned
            }
        }
        let run = |members: &str| -> (String, String) {
            let shapes = CString::new(format!(
                "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
                 @prefix ex: <http://example.org/ns#> .\n\
                 ex:Listed a sh:NodeShape ; sh:in ( {members} ) .\n"
            ))
            .expect("no NUL");
            let data =
                CString::new("<http://example.org/ns#a> <http://example.org/ns#n> \"1\" .\n")
                    .expect("no NUL");
            let expr = CString::new("http://example.org/ns#Constant").expect("no NUL");
            let focus = CString::new("http://example.org/ns#a").expect("no NUL");
            let mut terms: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut diagnostics: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every C string outlives the call; the arrays are empty (NULL, 0); the
            // out-pointers are writable locals, and both buffers are read and freed below.
            unsafe {
                let status = purrdf_shacl_eval_node_expr(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    data.as_ptr(),
                    expr.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    focus.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut terms,
                    &raw mut diagnostics,
                    &raw mut error,
                );
                assert_eq!(status, PurrdfStatus::Ok as i32);
                (text(terms), text(diagnostics))
            }
        };
        let (terms, diagnostics) = run("");
        assert_eq!(terms, "<http://example.org/ns#Constant>\n");
        assert_eq!(
            diagnostics,
            "diagnostic in-minListLength <http://example.org/ns#Listed>\n"
        );
        let (terms, diagnostics) = run("ex:one");
        assert_eq!(terms, "<http://example.org/ns#Constant>\n");
        assert_eq!(diagnostics, "");
    }

    /// The expression selectors across the boundary: an anonymous expression named by a
    /// walk and inline as Turtle, each refusal beside a valid neighbour — a step reaching
    /// two values beside one reaching one, two roots beside one, two selectors beside
    /// one — and a NULL walk array with a non-zero count refused as a `NullPointer`.
    #[test]
    fn capi_eval_node_expr_selectors() {
        use std::ffi::CString;

        use purrdf_core::vocab::sh::NS as SH;
        let shapes = CString::new(TOOLS_SHAPES).expect("no NUL");
        let data = CString::new(TOOLS_DATA).expect("no NUL");
        let focus = CString::new("http://example.org/ns#a").expect("no NUL");
        let c = |text: Option<&str>| text.map(|t| CString::new(t).expect("no NUL"));
        let ptr = |text: &Option<CString>| text.as_ref().map_or(std::ptr::null(), |t| t.as_ptr());
        let run = |expr: Option<&str>,
                   at: Option<&str>,
                   via: &[&str],
                   turtle: Option<&str>|
         -> (i32, String) {
            let (expr, at, turtle) = (c(expr), c(at), c(turtle));
            let owned: Vec<CString> = via
                .iter()
                .map(|p| CString::new(*p).expect("no NUL"))
                .collect();
            let pointers: Vec<*const c_char> = owned.iter().map(|p| p.as_ptr()).collect();
            let mut terms: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString, NULL, or a writable local; the walk
            // array holds exactly `pointers.len()` elements.
            unsafe {
                let status = purrdf_shacl_eval_node_expr(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    data.as_ptr(),
                    ptr(&expr),
                    ptr(&at),
                    if pointers.is_empty() {
                        std::ptr::null()
                    } else {
                        pointers.as_ptr()
                    },
                    pointers.len(),
                    ptr(&turtle),
                    focus.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut terms,
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                if status == PurrdfStatus::Ok as i32 {
                    (status, take_text(terms))
                } else {
                    (status, take_error(error))
                }
            }
        };
        let yes = (
            PurrdfStatus::Ok as i32,
            "<http://example.org/ns#yes>\n".to_owned(),
        );
        let (rule, object) = (format!("{SH}rule"), format!("{SH}object"));
        assert_eq!(
            run(
                None,
                Some("http://example.org/ns#Tagger"),
                &[&rule, &object],
                None
            ),
            yes
        );
        let function = format!("{SH}SPARQLExprExpression");
        assert_eq!(
            run(
                None,
                Some(&function),
                &["http://www.w3.org/2000/01/rdf-schema#isDefinedBy"],
                None
            ),
            (PurrdfStatus::Ok as i32, format!("<{SH}>\n"))
        );
        let parameter = format!("{SH}parameter");
        let (status, message) = run(None, Some(&function), &[&parameter], None);
        assert_eq!(status, PurrdfStatus::ParseError as i32);
        assert!(message.contains("reaches 2 values"), "{message}");

        assert_eq!(
            run(
                None,
                None,
                &[],
                Some("[ sh:sparqlExpr \"ex:yes\" ; sh:prefixes ex:Prefixes ] .")
            ),
            yes
        );
        let (status, message) = run(
            None,
            None,
            &[],
            Some("[ shnex:var \"a\" ] . [ shnex:var \"b\" ] ."),
        );
        assert_eq!(status, PurrdfStatus::ParseError as i32);
        assert!(message.contains("has 2 root blank nodes"), "{message}");
        let (status, message) = run(
            Some("http://example.org/ns#Tag"),
            None,
            &[],
            Some("[ shnex:var \"a\" ] ."),
        );
        assert_eq!(status, PurrdfStatus::ParseError as i32);
        assert!(message.contains("2 of the expression node"), "{message}");

        let at = CString::new("http://example.org/ns#Tagger").expect("no NUL");
        let mut terms: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: a NULL walk array with a non-zero count is refused before it is read.
        let status = unsafe {
            purrdf_shacl_eval_node_expr(
                shapes.as_ptr(),
                std::ptr::null(),
                data.as_ptr(),
                std::ptr::null(),
                at.as_ptr(),
                std::ptr::null(),
                2,
                std::ptr::null(),
                focus.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                &raw mut terms,
                std::ptr::null_mut(),
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::NullPointer as i32);
        // SAFETY: the failed call wrote the error.
        unsafe { purrdf_error_free(error) };
    }

    /// `purrdf_shacl_lint_shapes` across the boundary: the fixture certifies clean with
    /// `sh:sparqlExpr`'s function bound natively; a malformed neighbour is a report with
    /// findings, not an error; a document that is not Turtle is a `ParseError`.
    #[test]
    fn capi_lint_shapes() {
        use std::ffi::CString;

        let lint = |text: &str| -> (i32, i32, usize, String) {
            let shapes = CString::new(text).expect("no NUL");
            let mut report: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut clean = -1i32;
            let mut findings = usize::MAX;
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString or a writable local.
            unsafe {
                let status = purrdf_shacl_lint_shapes(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut report,
                    &raw mut clean,
                    &raw mut findings,
                    &raw mut error,
                );
                if status == PurrdfStatus::Ok as i32 {
                    (status, clean, findings, take_text(report))
                } else {
                    (status, clean, findings, take_error(error))
                }
            }
        };
        let (status, clean, findings, report) = lint(TOOLS_SHAPES);
        assert_eq!((status, clean, findings), (PurrdfStatus::Ok as i32, 1, 0));
        assert!(
            report.contains(
                "call native <http://www.w3.org/ns/shacl#SPARQLExprExpression> in sh:rule on \
                 <http://example.org/ns#Tagger>\n"
            ),
            "{report}"
        );
        assert!(
            report.ends_with("unanchored-imports 0\nfindings 0\nclean true\n"),
            "{report}"
        );
        // An owl:imports on a node that is no anchor is data: listed, never a finding.
        let (status, clean, findings, report) = lint(&format!(
            "{TOOLS_SHAPES}ex:Other <http://www.w3.org/2002/07/owl#imports> ex:Target .\n"
        ));
        assert_eq!((status, clean, findings), (PurrdfStatus::Ok as i32, 1, 0));
        assert!(
            report.contains(
                "unanchored-imports 1\nunanchored <http://example.org/ns#Other> \
                 <http://example.org/ns#Target> document -\n"
            ),
            "{report}"
        );
        let (status, clean, findings, report) = lint(&format!(
            "{TOOLS_SHAPES}ex:Bad a sh:NodeShape ; \
             sh:property [ sh:path ex:p ; sh:minCount \"one\" ] .\n"
        ));
        assert_eq!((status, clean), (PurrdfStatus::Ok as i32, 0));
        assert!(findings >= 2, "{report}");
        assert!(report.starts_with("load refused\n"), "{report}");
        let (status, ..) = lint("@@@ not turtle");
        assert_eq!(status, PurrdfStatus::ParseError as i32);
        // An empty sh:in list is a mandatory diagnostic named by rule id: one finding, the
        // load accepted. Its neighbour, the clean fixture, carries `diagnostics 0`.
        let (status, clean, findings, report) = lint(&format!(
            "{TOOLS_SHAPES}ex:Empty a sh:NodeShape ; sh:in () .\n"
        ));
        assert_eq!((status, clean, findings), (PurrdfStatus::Ok as i32, 0, 1));
        assert!(report.starts_with("load accepted\n"), "{report}");
        assert!(
            report.contains(
                "diagnostics 1\ndiagnostic in-minListLength <http://example.org/ns#Empty>\n"
            ),
            "{report}"
        );
        let (.., report) = lint(TOOLS_SHAPES);
        assert!(report.contains("diagnostics 0\n"), "{report}");
    }

    // ── One shapes graph, one owl:imports verdict, on every entry point ─────────

    /// The importing shapes graph: an ontology header and its import, no shape of its own.
    const IMPORTER: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
        <http://example.org/shapes> a owl:Ontology ;\n\
          owl:imports <http://example.org/lib> .\n";

    /// The imported document: a shape needing `ex:name`, a rule tagging every `ex:Person`
    /// `ex:checked ex:yes`, and a node expression `ex:Who` reading the scope variable `who`.
    const IMPORTED: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .\n\
        @prefix ex: <http://example.org/> .\n\
        ex:NameShape a sh:NodeShape ;\n\
          sh:targetClass ex:Person ;\n\
          sh:property [ sh:path ex:name ; sh:minCount 1 ] ;\n\
          sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:checked ; \
                    sh:object ex:yes ] .\n\
        ex:Who shnex:var \"who\" .\n";

    /// One `ex:Person` with no `ex:name`.
    const PERSON: &str = "<http://example.org/alice> \
        <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";

    /// The typed half of a failed call — status, kind, IRIs — freeing the error.
    ///
    /// # Safety
    /// `error` must be a live error written by a failed call.
    unsafe fn take_import_error(
        status: i32,
        error: *mut PurrdfError,
    ) -> (i32, String, Vec<String>) {
        unsafe {
            let text = |pointer: *const c_char| {
                (!pointer.is_null()).then(|| {
                    std::ffi::CStr::from_ptr(pointer)
                        .to_string_lossy()
                        .into_owned()
                })
            };
            let kind = text(purrdf_shapes_import_error_kind(error)).unwrap_or_default();
            let iris = (0..purrdf_shapes_import_error_iri_count(error))
                .filter_map(|index| text(purrdf_shapes_import_error_iri(error, index)))
                .collect();
            assert!(
                purrdf_shapes_import_error_iri(error, usize::MAX).is_null(),
                "an out-of-range index reads NULL"
            );
            purrdf_error_free(error);
            (status, kind, iris)
        }
    }

    /// Every shapes-graph entry point of the C ABI refuses the importing shapes graph with
    /// `PURRDF_STATUS_SHAPES_IMPORT_ERROR`, kind `unresolved-import` and the one IRI, when the
    /// import table is empty — and, with the imported document in the table, applies it:
    /// the imported shape reports, the rule infers, the expression reads its scope, the lint
    /// is clean, and the product carries the shape.
    #[test]
    fn every_shapes_entry_point_gives_the_same_owl_imports_verdict() {
        use std::ffi::CString;

        let shapes = CString::new(IMPORTER).expect("no NUL");
        let data = CString::new(PERSON).expect("no NUL");
        let lib_iri = CString::new("http://example.org/lib").expect("no NUL");
        let lib_document = CString::new(IMPORTED).expect("no NUL");
        let iris = [lib_iri.as_ptr()];
        let documents = [lib_document.as_ptr()];
        // (import_iris, import_documents, import_count) for the empty and the full table.
        let tables: [(*const *const c_char, *const *const c_char, usize); 2] = [
            (std::ptr::null(), std::ptr::null(), 0),
            (iris.as_ptr(), documents.as_ptr(), 1),
        ];
        let refusal = (
            PurrdfStatus::ShapesImportError as i32,
            "unresolved-import".to_owned(),
            vec!["http://example.org/lib".to_owned()],
        );
        let who = CString::new("http://example.org/Who").expect("no NUL");
        let alice = CString::new("http://example.org/alice").expect("no NUL");
        let scope_binding = CString::new("who=http://example.org/bob").expect("no NUL");
        let scope = [scope_binding.as_ptr()];

        for (index, (import_iris, import_documents, import_count)) in tables.into_iter().enumerate()
        {
            let supplied = index == 1;
            // SAFETY: every pointer is a live CString, an array of live CStrings of the
            // stated length (or NULL with a zero count), or a writable local.
            unsafe {
                let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
                let mut error: *mut PurrdfError = std::ptr::null_mut();
                let status = purrdf_shacl_validate_to_sarif(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    data.as_ptr(),
                    std::ptr::null(),
                    0,
                    import_iris,
                    import_documents,
                    import_count,
                    false,
                    &raw mut buffer,
                    &raw mut error,
                );
                if supplied {
                    assert_eq!(status, PurrdfStatus::Ok as i32);
                    assert!(take_text(buffer).contains("MinCountConstraintComponent"));
                } else {
                    assert_eq!(take_import_error(status, error), refusal, "validate");
                }

                let mut scope_kind = -1i32;
                let mut focus_nodes = 0usize;
                let mut reason: *mut PurrdfBuffer = std::ptr::null_mut();
                let status = purrdf_shacl_validate_changes_to_sarif(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    c"".as_ptr(),
                    data.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    import_iris,
                    import_documents,
                    import_count,
                    false,
                    &raw mut buffer,
                    &raw mut scope_kind,
                    &raw mut focus_nodes,
                    &raw mut reason,
                    &raw mut error,
                );
                if supplied {
                    assert_eq!(status, PurrdfStatus::Ok as i32);
                    assert!(take_text(buffer).contains("MinCountConstraintComponent"));
                    if !reason.is_null() {
                        crate::buffer::purrdf_buffer_free(reason);
                    }
                } else {
                    assert_eq!(take_import_error(status, error), refusal, "change path");
                }

                let status = purrdf_shacl_entail_to_ntriples(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    data.as_ptr(),
                    import_iris,
                    import_documents,
                    import_count,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    &raw mut buffer,
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                if supplied {
                    assert_eq!(status, PurrdfStatus::Ok as i32);
                    assert!(take_text(buffer).contains("<http://example.org/checked>"));
                } else {
                    assert_eq!(take_import_error(status, error), refusal, "entail");
                }

                let status = purrdf_shacl_apply_rules(
                    data.as_ptr(),
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    import_iris,
                    import_documents,
                    import_count,
                    &raw mut buffer,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                if supplied {
                    assert_eq!(status, PurrdfStatus::Ok as i32);
                    assert_eq!(
                        take_text(buffer),
                        "<http://example.org/alice> <http://example.org/checked> \
                         <http://example.org/yes> .\n"
                    );
                } else {
                    assert_eq!(take_import_error(status, error), refusal, "rules");
                }

                let status = purrdf_shacl_eval_node_expr(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    data.as_ptr(),
                    who.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    alice.as_ptr(),
                    scope.as_ptr(),
                    scope.len(),
                    import_iris,
                    import_documents,
                    import_count,
                    &raw mut buffer,
                    std::ptr::null_mut(),
                    &raw mut error,
                );
                if supplied {
                    assert_eq!(status, PurrdfStatus::Ok as i32);
                    assert_eq!(take_text(buffer), "<http://example.org/bob>\n");
                } else {
                    assert_eq!(take_import_error(status, error), refusal, "node-expr");
                }

                let mut clean = -1i32;
                let mut findings = usize::MAX;
                let status = purrdf_shacl_lint_shapes(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    import_iris,
                    import_documents,
                    import_count,
                    &raw mut buffer,
                    &raw mut clean,
                    &raw mut findings,
                    &raw mut error,
                );
                if supplied {
                    assert_eq!((status, clean, findings), (PurrdfStatus::Ok as i32, 1, 0));
                    take_text(buffer);
                } else {
                    assert_eq!(take_import_error(status, error), refusal, "lint");
                }

                let status = purrdf_shapes_product_encode(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    import_iris,
                    import_documents,
                    import_count,
                    &raw mut buffer,
                    &raw mut error,
                );
                if supplied {
                    assert_eq!(status, PurrdfStatus::Ok as i32);
                    let mut product_ptr: *const u8 = std::ptr::null();
                    let mut product_len = 0usize;
                    crate::buffer::purrdf_buffer_data(
                        buffer,
                        &raw mut product_ptr,
                        &raw mut product_len,
                    );
                    let product = std::slice::from_raw_parts(product_ptr, product_len).to_vec();
                    crate::buffer::purrdf_buffer_free(buffer);
                    let sarif = purrdf_validate::validate_with_shapes_product(
                        &product,
                        PERSON,
                        &SarifOptions::default(),
                    )
                    .expect("restores");
                    assert!(sarif.contains("MinCountConstraintComponent"), "{sarif}");
                } else {
                    assert_eq!(take_import_error(status, error), refusal, "product encode");
                }
            }
        }
    }

    /// SHACL's prefix idiom (the W3C `sparql/node/prefixes-001` shape on `example.org`): the
    /// `owl:imports` sits on `ex:TestPrefixes`, which is neither the shapes graph's IRI nor an
    /// ontology header, so it is a prefix edge and not an import: the call succeeds with an
    /// EMPTY import table, and the prefixes `imp:` (declared only on the target) and `test:`
    /// (only on the importing node) reach the query, which reports `ex:Invalid` with
    /// `test:Value`. The neighbour types `ex:TestPrefixes` `owl:Ontology`, making its
    /// `owl:imports` an import of a document nothing in hand declares, and is refused with the
    /// typed import error naming it.
    #[test]
    fn the_shacl_prefix_idiom_resolves_with_no_table_and_a_header_import_does_not() {
        use std::ffi::CString;

        let idiom = |importer_type: &str| {
            CString::new(format!(
                "@prefix ex: <http://example.org/ns#> .\n\
                 @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                 @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
                 @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
                 @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
                 <http://example.org/ns#> sh:declare [ sh:prefix \"imp\" ; \
                   sh:namespace \"http://example.org/ns#\"^^xsd:anyURI ] .\n\
                 ex:TestPrefixes {importer_type} owl:imports <http://example.org/ns#> ;\n\
                   sh:declare [ sh:prefix \"test\" ; \
                                sh:namespace \"http://example.org/test#\"^^xsd:anyURI ] .\n\
                 ex:TestSPARQL sh:prefixes ex:TestPrefixes ;\n\
                   sh:select \"SELECT $this ?value WHERE {{ $this imp:property ?value . \
                                FILTER (?value = test:Value) }}\" .\n\
                 ex:TestShape a sh:NodeShape ; sh:sparql ex:TestSPARQL ;\n\
                   sh:targetNode ex:Invalid , ex:Valid .\n"
            ))
            .expect("no NUL")
        };
        let data = CString::new(
            "<http://example.org/ns#Invalid> <http://example.org/ns#property> \
             <http://example.org/test#Value> .\n\
             <http://example.org/ns#Valid> <http://example.org/ns#property> \
             <http://example.org/test#Other> .\n",
        )
        .expect("no NUL");
        let validate = |shapes: &CString| {
            let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString or a writable local; the import table
            // is empty (NULL arrays, zero count).
            let status = unsafe {
                purrdf_shacl_validate_to_sarif(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    data.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    false,
                    &raw mut buffer,
                    &raw mut error,
                )
            };
            (status, buffer, error)
        };

        let (status, buffer, _) = validate(&idiom(""));
        assert_eq!(status, PurrdfStatus::Ok as i32);
        // SAFETY: a successful call wrote a live buffer.
        let sarif = unsafe { take_text(buffer) };
        assert!(
            sarif.contains("SPARQLConstraintComponent")
                && sarif.contains("http://example.org/ns#Invalid")
                && sarif.contains("http://example.org/test#Value")
                && !sarif.contains("http://example.org/test#Other"),
            "both declared prefixes reached the query: {sarif}"
        );

        let (status, _, error) = validate(&idiom("a owl:Ontology ;"));
        // SAFETY: a failed call wrote a live error.
        let refused = unsafe { take_import_error(status, error) };
        assert_eq!(
            refused,
            (
                PurrdfStatus::ShapesImportError as i32,
                "unresolved-import".to_owned(),
                vec!["http://example.org/ns#".to_owned()],
            )
        );
    }

    /// SHACL 1.2 Core section 6.4 across the C boundary: a data graph's `sh:shapesGraph`
    /// link is resolved through the same `import_iris` / `import_documents` table, refused
    /// by kind and IRI when the table lacks it, and — supplied — its shape reports a result
    /// the same call without the link does not.
    #[test]
    fn a_data_graph_link_resolves_through_the_import_table() {
        use std::ffi::CString;

        const SHAPES1: &str = "http://example.org/graph-shapes1";
        let shapes = CString::new(
            "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
             @prefix ex: <http://example.org/> .\n\
             ex:LocalNode a sh:NodeShape ; sh:targetNode ex:Focus ;\n\
               sh:property ex:LocalShape .\n\
             ex:LocalShape sh:path ex:q ; sh:maxCount 0 .\n",
        )
        .expect("no NUL");
        let focus = "<http://example.org/Focus> <http://example.org/q> \"x\" .\n\
             <http://example.org/myDataGraph> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://www.w3.org/ns/shacl#DataGraph> .\n";
        let linked = CString::new(format!(
            "{focus}<http://example.org/myDataGraph> \
             <http://www.w3.org/ns/shacl#shapesGraph> <{SHAPES1}> .\n"
        ))
        .expect("no NUL");
        let unlinked = CString::new(focus).expect("no NUL");
        let link_iri = CString::new(SHAPES1).expect("no NUL");
        let link_document = CString::new(
            "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
             @prefix ex: <http://example.org/> .\n\
             ex:LinkedNode a sh:NodeShape ; sh:targetNode ex:Focus ;\n\
               sh:property ex:LinkedShape .\n\
             ex:LinkedShape sh:path ex:p ; sh:minCount 1 .\n",
        )
        .expect("no NUL");
        let iris = [link_iri.as_ptr()];
        let documents = [link_document.as_ptr()];

        let validate = |data: &CString,
                        import_iris: *const *const c_char,
                        import_documents: *const *const c_char,
                        import_count: usize|
         -> (i32, *mut PurrdfBuffer, *mut PurrdfError) {
            let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut error: *mut PurrdfError = std::ptr::null_mut();
            // SAFETY: every pointer is a live CString, an array of live CStrings of the
            // stated length (or NULL with a zero count), or a writable local.
            let status = unsafe {
                purrdf_shacl_validate_to_sarif(
                    shapes.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    data.as_ptr(),
                    std::ptr::null(),
                    0,
                    import_iris,
                    import_documents,
                    import_count,
                    false,
                    &raw mut buffer,
                    &raw mut error,
                )
            };
            (status, buffer, error)
        };

        let (status, _, error) = validate(&linked, std::ptr::null(), std::ptr::null(), 0);
        // SAFETY: `error` is the live error the call above wrote.
        let refusal = unsafe { take_import_error(status, error) };
        assert_eq!(
            refusal,
            (
                PurrdfStatus::ShapesImportError as i32,
                "unresolved-shapes-graph-link".to_owned(),
                vec![SHAPES1.to_owned()],
            )
        );

        let (status, buffer, _) = validate(&linked, iris.as_ptr(), documents.as_ptr(), 1);
        assert_eq!(status, PurrdfStatus::Ok as i32);
        // SAFETY: `buffer` is the live buffer the successful call wrote.
        let sarif = unsafe { take_text(buffer) };
        assert!(
            sarif.contains("LinkedShape") && sarif.contains("LocalShape"),
            "{sarif}"
        );

        let (status, buffer, _) = validate(&unlinked, std::ptr::null(), std::ptr::null(), 0);
        assert_eq!(status, PurrdfStatus::Ok as i32);
        // SAFETY: as above.
        let sarif = unsafe { take_text(buffer) };
        assert!(
            !sarif.contains("LinkedShape") && sarif.contains("LocalShape"),
            "{sarif}"
        );
    }

    /// The accessors answer NULL / 0 for an error that is not an import refusal.
    #[test]
    fn the_import_accessors_are_silent_on_other_errors() {
        let error = PurrdfError::new(PurrdfStatus::ParseError, "not an import refusal");
        let pointer = std::ptr::from_ref(&error);
        // SAFETY: `pointer` borrows a live local error.
        unsafe {
            assert!(purrdf_shapes_import_error_kind(pointer).is_null());
            assert_eq!(purrdf_shapes_import_error_iri_count(pointer), 0);
            assert!(purrdf_shapes_import_error_iri(pointer, 0).is_null());
            assert!(purrdf_shapes_import_error_kind(std::ptr::null()).is_null());
        }
    }
}
