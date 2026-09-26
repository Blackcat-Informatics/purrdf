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
    ChangeScope, ConformanceDisallows, LintReport, NodeExprRequest, RulesOutcome, RulesRequest,
    SarifOptions, ShapesError, ShapesProductRefusal, ValidationOptions, apply_rules_to_ntriples,
    entail_to_ntriples_string, eval_node_expr_to_terms, lint_shapes_ttl, parse_scope_binding,
    validate_changes_to_sarif_string, validate_to_sarif_string,
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
/// IRIs; empty is SHACL's default set.
fn validate_to_sarif_bytes(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    conformance_disallows: &[&str],
    imports: &[(&str, &str)],
) -> Result<Vec<u8>, ShapesError> {
    let validation = if conformance_disallows.is_empty() {
        ValidationOptions::default()
    } else {
        ValidationOptions::default()
            .with_conformance_disallows(ConformanceDisallows::from_iris(conformance_disallows)?)
    };
    let options = SarifOptions {
        validation,
        ..SarifOptions::default()
    };
    Ok(validate_to_sarif_string(shapes_ttl, shapes_base, data_nt, &options, imports)?.into_bytes())
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
/// `properties.shaclConforms` and `properties.shaclConformanceDisallows`, because the
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
/// arrays may then be NULL) is the empty table. An `owl:imports` is resolved by a table
/// entry, by `shapes_base_iri` (or the document's own `@base`) naming the imported
/// document, or by the closure declaring the ontology (`<X> a owl:Ontology`, or an
/// ontology whose `owl:versionIRI` is `<X>`); anything else — or a table entry nothing
/// imports — returns `PURRDF_STATUS_SHAPES_IMPORT_ERROR` rather than a report about a
/// smaller shapes graph than the one named. Read its kind and IRIs with
/// `purrdf_shapes_import_error_kind` / `_iri_count` / `_iri`.
///
/// # Safety
/// `shapes_ttl` and `data_nt` must be non-null, NUL-terminated C strings;
/// `shapes_base_iri` must be null or a NUL-terminated C string; when
/// `conformance_disallows_count` is non-zero, `conformance_disallows` must address
/// that many NUL-terminated C strings; when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings; `out_buffer` must be a writable
/// pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_validate_to_sarif(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    data_nt: *const c_char,
    conformance_disallows: *const *const c_char,
    conformance_disallows_count: usize,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
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
            let bytes = validate_to_sarif_bytes(shapes, base, data, &disallows, &imports)
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
fn validate_changes_to_sarif_bytes(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    added_nt: Option<&str>,
    removed_nt: Option<&str>,
    imports: &[(&str, &str)],
) -> Result<(Vec<u8>, ChangeScope), ShapesError> {
    let (sarif, scope) = validate_changes_to_sarif_string(
        shapes_ttl,
        shapes_base,
        data_nt,
        added_nt,
        removed_nt,
        &SarifOptions::default(),
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
/// # Safety
/// `shapes_ttl` and `data_nt` must be non-null, NUL-terminated C strings;
/// `shapes_base_iri`, `added_nt` and `removed_nt` must be null or NUL-terminated C
/// strings; when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings; `out_buffer`, `out_scope`, `out_focus_nodes` and
/// `out_reason` must be writable pointers; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_validate_changes_to_sarif(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    data_nt: *const c_char,
    added_nt: *const c_char,
    removed_nt: *const c_char,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
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
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_shacl_validate_changes_to_sarif",
            )?;
            let (bytes, scope) =
                validate_changes_to_sarif_bytes(shapes, base, data, added, removed, &imports)
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

/// Entail `data_nt` (N-Triples) under `shapes_ttl` (Turtle) and serialize the
/// materialized dataset (base graph plus every SHACL-AF rule inference) to
/// canonical N-Triples bytes. Native-testable, pointer-free core.
///
/// The parse→entail→serialize sequence lives in [`entail_to_ntriples_string`];
/// this only adds the C-ABI byte framing.
fn entail_to_ntriples_bytes(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    imports: &[(&str, &str)],
) -> Result<Vec<u8>, ShapesError> {
    Ok(entail_to_ntriples_string(shapes_ttl, shapes_base, data_nt, imports)?.into_bytes())
}

/// Entail a data graph (N-Triples) under a shapes graph (Turtle) and write the
/// materialized dataset (base graph plus every inferred triple) as canonical
/// N-Triples bytes to `*out_buffer` (free with `purrdf_buffer_free`).
///
/// `shapes_base_iri` carries the same meaning it does on
/// `purrdf_shacl_validate_to_sarif`: the shapes document's own base IRI, nullable,
/// and read rather than accepted-and-dropped.
///
/// Nothing is dropped on the way out: the underlying writer is the graph-carrying
/// canonical N-Quads serializer, and the output is N-Triples because BOTH inputs
/// are single-graph syntaxes, not because a graph slot was discarded.
///
/// `import_iris` / `import_documents` / `import_count` are the shapes graph's
/// `owl:imports` table (see `purrdf_shacl_validate_to_sarif`). An imported document's rules
/// run.
///
/// # Safety
/// `shapes_ttl` and `data_nt` must be non-null, NUL-terminated C strings;
/// `shapes_base_iri` must be null or a NUL-terminated C string; when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings;
/// `out_buffer` must be a writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_entail_to_ntriples(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    data_nt: *const c_char,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    out_buffer: *mut *mut PurrdfBuffer,
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
            let data = cstr_to_str(data_nt)?;
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_shacl_entail_to_ntriples",
            )?;
            let bytes = entail_to_ntriples_bytes(shapes, base, data, &imports)
                .map_err(PurrdfError::shapes)?;
            *out_buffer = PurrdfBuffer::into_raw(bytes);
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
/// `max_term_generating_rounds` may be NULL for the engine default (65,536); otherwise it
/// points at the limit on evaluation rounds that infer a term the graph did not hold, and
/// one more round fails the call naming the limit. A host running UNTRUSTED rule sets
/// should lower it: an exponential rule set reaches the engine's fixed arena and join
/// ceilings only slowly under the default, and the limit is what bounds the time it can
/// take.
///
/// `out_proof` asks for the proof: NULL skips it; non-NULL receives a buffer with the
/// proof of every inferred triple (`derived S P O .`, then `  rule R` and one
/// `  premise S P O .` per matched fact, or `  data-block` for a SPARQL 1.2 RL data-block
/// triple), freed with `purrdf_buffer_free`.
///
/// `import_iris` / `import_documents` / `import_count` are the shapes graph's
/// `owl:imports` table (see `purrdf_shacl_validate_to_sarif`). An imported document's rules
/// run. A SPARQL 1.2 RL rule set reads no table, so a non-empty one beside `srl` is a
/// `ParseError`.
///
/// # Safety
/// `data_nt` must be a non-null NUL-terminated C string; `shapes_ttl`, `shapes_base_iri`,
/// `srl` and `srl_base_iri` must each be null or a NUL-terminated C string;
/// `max_term_generating_rounds` must be null or readable; when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings; `out_inferred`
/// must be writable; `out_proof` and `out_error` must each be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_apply_rules(
    data_nt: *const c_char,
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    srl: *const c_char,
    srl_base_iri: *const c_char,
    max_term_generating_rounds: *const u64,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    out_inferred: *mut *mut PurrdfBuffer,
    out_proof: *mut *mut PurrdfBuffer,
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
                shapes_imports: &imports,
                srl: opt_cstr_to_str(srl)?,
                srl_base: opt_cstr_to_str(srl_base_iri)?,
                explain: !out_proof.is_null(),
                // SAFETY: the caller's contract — null or readable.
                max_term_generating_rounds: max_term_generating_rounds.as_ref().copied(),
            };
            let outcome = apply_rules_outcome(&request).map_err(PurrdfError::shapes)?;
            if let Some(proof) = outcome.proof {
                *out_proof = PurrdfBuffer::into_raw(proof.into_bytes());
            }
            *out_inferred = PurrdfBuffer::into_raw(outcome.inferred_ntriples.into_bytes());
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Evaluate one node expression. Native-testable, pointer-free core of
/// [`purrdf_shacl_eval_node_expr`]: `scope` holds `NAME=TERM` bindings, and the output
/// is one N-Triples 1.2 term per line.
fn eval_node_expr_bytes(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    expr: &str,
    focus: &str,
    scope: &[&str],
    imports: &[(&str, &str)],
) -> Result<Vec<u8>, ShapesError> {
    let bindings = scope
        .iter()
        .map(|binding| parse_scope_binding(binding))
        .collect::<Result<Vec<_>, _>>()?;
    let terms = eval_node_expr_to_terms(&NodeExprRequest {
        shapes_ttl,
        shapes_base,
        data_nt,
        expr,
        focus,
        scope: &bindings,
        imports,
    })?;
    let mut out = String::new();
    for term in terms {
        out.push_str(&term);
        out.push('\n');
    }
    Ok(out.into_bytes())
}

/// Evaluate ONE node expression of a shapes graph (Turtle) against a focus node of a data
/// graph (N-Triples) — SHACL 1.2 Node Expressions' `evalExpr(expr, focusGraph, focusNode,
/// scope)` — and write its output nodes to `*out_terms` (free with `purrdf_buffer_free`):
/// one N-Triples 1.2 term per line, in the order the expression's sequence semantics
/// define. N-Triples escapes every line break inside a term, so each line is one term; an
/// expression with no output writes an empty buffer.
///
/// `expr` is an absolute IRI or `_:label` for a blank node the shapes document labels so;
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
/// # Safety
/// `shapes_ttl`, `data_nt`, `expr` and `focus` must be non-null NUL-terminated C strings;
/// `shapes_base_iri` must be null or a NUL-terminated C string; when `scope_count` is
/// non-zero, `scope` must address that many NUL-terminated C strings; when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings;
/// `out_terms` must be writable; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_eval_node_expr(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    data_nt: *const c_char,
    expr: *const c_char,
    focus: *const c_char,
    scope: *const *const c_char,
    scope_count: usize,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    out_terms: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if shapes_ttl.is_null()
                || data_nt.is_null()
                || expr.is_null()
                || focus.is_null()
                || out_terms.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_shacl_eval_node_expr",
                ));
            }
            let bindings = cstr_array(scope, scope_count, "purrdf_shacl_eval_node_expr")?;
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_shacl_eval_node_expr",
            )?;
            let bytes = eval_node_expr_bytes(
                cstr_to_str(shapes_ttl)?,
                opt_cstr_to_str(shapes_base_iri)?,
                cstr_to_str(data_nt)?,
                cstr_to_str(expr)?,
                cstr_to_str(focus)?,
                &bindings,
                &imports,
            )
            .map_err(PurrdfError::shapes)?;
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
    imports: &[(&str, &str)],
) -> Result<LintReport, ShapesError> {
    lint_shapes_ttl(shapes_ttl, shapes_base, imports)
}

/// Certify a shapes graph (Turtle) COLD — the loader's verdict, every result of validating
/// it against the W3C `shacl-shacl.ttl`, and which implementation every node-expression
/// function call binds to — and write the report's deterministic text to `*out_report`
/// (free with `purrdf_buffer_free`): the `load`, `shacl-shacl` (`result …` lines,
/// `superseded NAME` where SHACL 1.2 Core makes the flagged graph well-formed) and
/// `functions` (`call BINDING <IRI> in OWNER`) sections, then `findings N` and
/// `clean true|false`.
///
/// `*out_clean` receives 1 when the report carries no finding — the loader accepted the
/// graph and every `shacl-shacl.ttl` result is superseded — and 0 otherwise;
/// `*out_findings` receives the finding count. A malformed shapes graph is a report with
/// findings and status `Ok`; only a document that is not Turtle is a `ParseError`.
///
/// `import_iris` / `import_documents` / `import_count` are the shapes graph's
/// `owl:imports` table (see `purrdf_shacl_validate_to_sarif`). The report certifies the
/// whole closure; one that is not in hand returns `PURRDF_STATUS_SHAPES_IMPORT_ERROR` and
/// no report — never a report about the importing document alone, which would call a
/// shapes graph clean that validation refuses.
///
/// # Safety
/// `shapes_ttl` must be a non-null NUL-terminated C string; `shapes_base_iri` must be
/// null or a NUL-terminated C string; when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings; `out_report`, `out_clean` and
/// `out_findings` must be writable; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_lint_shapes(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
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
    imports: &[(&str, &str)],
) -> Result<Vec<u8>, ShapesProductRefusal> {
    purrdf_validate::pack_shapes_product(shapes_ttl, shapes_base, imports)
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
/// # Safety
/// `shapes_ttl` must be a non-null, NUL-terminated C string; `shapes_base_iri` must be
/// null or a NUL-terminated C string; when `import_count` is non-zero, `import_iris` and `import_documents` must each
/// address that many NUL-terminated C strings; `out_buffer` must be a writable
/// pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_product_encode(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
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
            let bytes = encode_product_bytes(shapes, base, &imports)
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
/// `property-function-registry`, `class-catalog`, `unsupported-capability`,
/// `depth-limit`, `malformed`.
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
/// table supplies documents no import names) or `invalid-import` (a key that is not an
/// absolute IRI, a key named twice, or a document that is not Turtle).
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
        let bytes = validate_to_sarif_bytes(SHAPES, None, DATA, &[], &[]).expect("sarif produced");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("\"version\": \"2.1.0\""));
        assert!(text.contains("\"level\": \"error\""));
    }

    #[test]
    fn malformed_shapes_is_an_error() {
        assert!(validate_to_sarif_bytes("@@@ not turtle", None, DATA, &[], &[]).is_err());
    }

    /// The conformance-disallow set crosses the boundary as a C string array and
    /// reaches the validation: a Warning-graded violation does not conform under the
    /// default set (count 0, NULL array) and conforms under `sh:Violation` alone; a
    /// non-IRI level is a `ParseError`, and a NULL array with a non-zero count a
    /// `NullPointer`, each refused before anything is validated.
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
                data.as_ptr(),
                std::ptr::null(),
                1,
                std::ptr::null(),
                std::ptr::null(),
                0,
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

    #[test]
    fn a_change_is_validated_against_the_graph_it_joins() {
        let (bytes, scope) =
            validate_changes_to_sarif_bytes(SHAPES, None, CHANGE_BASE, Some(BAD_AGE), None, &[])
                .expect("the change validates");
        assert_eq!(scope, ChangeScope::Bounded { focus_nodes: 1 });
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("DatatypeConstraintComponent"), "{text}");

        // The retract half is a real half: taking the bad row back out of the
        // merged graph restores conformance, through the same one call.
        let merged = format!("{CHANGE_BASE}{BAD_AGE}");
        let (bytes, scope) =
            validate_changes_to_sarif_bytes(SHAPES, None, &merged, None, Some(BAD_AGE), &[])
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
                data.as_ptr(),
                added.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
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
                data.as_ptr(),
                added.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
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
                data.as_ptr(),
                malformed.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
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
        let bytes = entail_to_ntriples_bytes(RULE_SHAPES, None, RULE_DATA, &[])
            .expect("entailment produced");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains(
            "<http://example.org/alice> <http://example.org/adult> <http://example.org/yes> ."
        ));
        assert!(text.contains(
            "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> ."
        ));
    }

    #[test]
    fn entail_malformed_shapes_is_an_error() {
        assert!(entail_to_ntriples_bytes("@@@ not turtle", None, RULE_DATA, &[]).is_err());
    }

    #[test]
    fn a_product_round_trips_to_the_same_verdict() {
        let product = encode_product_bytes(SHAPES, None, &[]).expect("product encoded");
        certify_product_bytes(&product).expect("certified");
        let described = String::from_utf8(open_product_bytes(&product).expect("opened"))
            .expect("the description is UTF-8");
        assert!(described.contains("stage-known true\n"));

        // Admitting the product and parsing the shapes graph are two routes to ONE
        // verdict, which is the property a cache is only allowed to have.
        let via_product = admit_product_bytes(&product, DATA).expect("validated via product");
        let via_document =
            validate_to_sarif_bytes(SHAPES, None, DATA, &[], &[]).expect("validated directly");
        assert_eq!(via_product, via_document);

        // Rebuilding a CURRENT product reaches the byte-identical verdict too: the
        // forward-compatibility door must not be a second, divergent answer.
        let via_rebuild = rebuild_product_bytes(&product, DATA).expect("rebuilt via product");
        assert_eq!(via_rebuild, via_product);
    }

    #[test]
    fn a_refused_product_carries_its_dimension_through_the_error_handle() {
        let mut wrong_magic = encode_product_bytes(SHAPES, None, &[]).expect("product encoded");
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
        let product = encode_product_bytes(SHAPES, None, &[]).expect("product encoded");
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
        let held = encode_product_bytes(SHAPES, None, &[]).expect("product encoded");
        let wanted =
            rendered_selector(&encode_product_bytes(OTHER_SHAPES, None, &[]).expect("encoded"));
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
        let product = encode_product_bytes(SHAPES, None, &[]).expect("product encoded");
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

        let product = encode_product_bytes(SHAPES, None, &[]).expect("product encoded");
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
        let held = encode_product_bytes(SHAPES, None, &[]).expect("product encoded");
        let wanted =
            rendered_selector(&encode_product_bytes(OTHER_SHAPES, None, &[]).expect("encoded"));
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
        let product = encode_product_bytes(SHAPES, None, &[]).expect("product encoded");
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

        let product = encode_product_bytes(SHAPES, None, &[]).expect("product encoded");
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
                    srl,
                    std::ptr::null(),
                    limit_ptr,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut inferred,
                    if explain {
                        &raw mut proof
                    } else {
                        std::ptr::null_mut()
                    },
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
                focus.as_ptr(),
                std::ptr::null(),
                1,
                std::ptr::null(),
                std::ptr::null(),
                0,
                &raw mut terms,
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
        let (status, clean, findings, report) = lint(&format!(
            "{TOOLS_SHAPES}ex:Bad a sh:NodeShape ; \
             sh:property [ sh:path ex:p ; sh:minCount \"one\" ] .\n"
        ));
        assert_eq!((status, clean), (PurrdfStatus::Ok as i32, 0));
        assert!(findings >= 2, "{report}");
        assert!(report.starts_with("load refused\n"), "{report}");
        let (status, ..) = lint("@@@ not turtle");
        assert_eq!(status, PurrdfStatus::ParseError as i32);
    }
}
