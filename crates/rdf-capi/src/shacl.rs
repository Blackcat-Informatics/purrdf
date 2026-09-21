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
    ChangeScope, SarifOptions, ShapesProductRefusal, entail_to_ntriples_string,
    validate_changes_to_sarif_string, validate_to_sarif_string,
};

use crate::buffer::PurrdfBuffer;
use crate::error::PurrdfError;
use crate::status::PurrdfStatus;
use crate::{cstr_to_str, opt_cstr_to_str};

/// Validate `data_nt` (N-Triples) against `shapes_ttl` (Turtle) and render the
/// report to SARIF 2.1.0 bytes. Native-testable, pointer-free core.
///
/// The validate→SARIF sequence lives in [`validate_to_sarif_string`]; this only
/// adds the C-ABI byte framing.
fn validate_to_sarif_bytes(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
) -> Result<Vec<u8>, String> {
    Ok(
        validate_to_sarif_string(shapes_ttl, shapes_base, data_nt, &SarifOptions::default())?
            .into_bytes(),
    )
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
/// # Safety
/// `shapes_ttl` and `data_nt` must be non-null, NUL-terminated C strings;
/// `shapes_base_iri` must be null or a NUL-terminated C string;
/// `out_buffer` must be a writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_validate_to_sarif(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    data_nt: *const c_char,
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
            let bytes = validate_to_sarif_bytes(shapes, base, data)
                .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
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
) -> Result<(Vec<u8>, ChangeScope), String> {
    let (sarif, scope) = validate_changes_to_sarif_string(
        shapes_ttl,
        shapes_base,
        data_nt,
        added_nt,
        removed_nt,
        &SarifOptions::default(),
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
/// strings; `out_buffer`, `out_scope`, `out_focus_nodes` and `out_reason` must be
/// writable pointers; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_validate_changes_to_sarif(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    data_nt: *const c_char,
    added_nt: *const c_char,
    removed_nt: *const c_char,
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
            let (bytes, scope) =
                validate_changes_to_sarif_bytes(shapes, base, data, added, removed)
                    .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
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
) -> Result<Vec<u8>, String> {
    Ok(entail_to_ntriples_string(shapes_ttl, shapes_base, data_nt)?.into_bytes())
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
/// # Safety
/// `shapes_ttl` and `data_nt` must be non-null, NUL-terminated C strings;
/// `shapes_base_iri` must be null or a NUL-terminated C string;
/// `out_buffer` must be a writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shacl_entail_to_ntriples(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
    data_nt: *const c_char,
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
            let bytes = entail_to_ntriples_bytes(shapes, base, data)
                .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
            *out_buffer = PurrdfBuffer::into_raw(bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

// ---------------------------------------------------------------------------
// Prepared shapes products
// ---------------------------------------------------------------------------

/// Map a boundary refusal onto the C error that preserves its dimension.
fn product_error(refusal: &ShapesProductRefusal) -> PurrdfError {
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
) -> Result<Vec<u8>, ShapesProductRefusal> {
    purrdf_validate::pack_shapes_product(shapes_ttl, shapes_base)
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
/// # Safety
/// `shapes_ttl` must be a non-null, NUL-terminated C string; `shapes_base_iri` must be
/// null or a NUL-terminated C string; `out_buffer` must be a writable pointer;
/// `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_shapes_product_encode(
    shapes_ttl: *const c_char,
    shapes_base_iri: *const c_char,
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
            let bytes =
                encode_product_bytes(shapes, base).map_err(|refusal| product_error(&refusal))?;
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
        let bytes = validate_to_sarif_bytes(SHAPES, None, DATA).expect("sarif produced");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("\"version\": \"2.1.0\""));
        assert!(text.contains("\"level\": \"error\""));
    }

    #[test]
    fn malformed_shapes_is_an_error() {
        assert!(validate_to_sarif_bytes("@@@ not turtle", None, DATA).is_err());
    }

    /// A conforming base, so every violation a change test sees is the change's.
    const CHANGE_BASE: &str = "<http://example.org/alice> \
        <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";

    /// The row that breaks it, and the row that un-breaks it again.
    const BAD_AGE: &str = "<http://example.org/alice> <http://example.org/age> \"nope\" .\n";

    #[test]
    fn a_change_is_validated_against_the_graph_it_joins() {
        let (bytes, scope) =
            validate_changes_to_sarif_bytes(SHAPES, None, CHANGE_BASE, Some(BAD_AGE), None)
                .expect("the change validates");
        assert_eq!(scope, ChangeScope::Bounded { focus_nodes: 1 });
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("DatatypeConstraintComponent"), "{text}");

        // The retract half is a real half: taking the bad row back out of the
        // merged graph restores conformance, through the same one call.
        let merged = format!("{CHANGE_BASE}{BAD_AGE}");
        let (bytes, scope) =
            validate_changes_to_sarif_bytes(SHAPES, None, &merged, None, Some(BAD_AGE))
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
        let bytes =
            entail_to_ntriples_bytes(RULE_SHAPES, None, RULE_DATA).expect("entailment produced");
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
        assert!(entail_to_ntriples_bytes("@@@ not turtle", None, RULE_DATA).is_err());
    }

    #[test]
    fn a_product_round_trips_to_the_same_verdict() {
        let product = encode_product_bytes(SHAPES, None).expect("product encoded");
        certify_product_bytes(&product).expect("certified");
        let described = String::from_utf8(open_product_bytes(&product).expect("opened"))
            .expect("the description is UTF-8");
        assert!(described.contains("stage-known true\n"));

        // Admitting the product and parsing the shapes graph are two routes to ONE
        // verdict, which is the property a cache is only allowed to have.
        let via_product = admit_product_bytes(&product, DATA).expect("validated via product");
        let via_document = validate_to_sarif_bytes(SHAPES, None, DATA).expect("validated directly");
        assert_eq!(via_product, via_document);

        // Rebuilding a CURRENT product reaches the byte-identical verdict too: the
        // forward-compatibility door must not be a second, divergent answer.
        let via_rebuild = rebuild_product_bytes(&product, DATA).expect("rebuilt via product");
        assert_eq!(via_rebuild, via_product);
    }

    #[test]
    fn a_refused_product_carries_its_dimension_through_the_error_handle() {
        let mut wrong_magic = encode_product_bytes(SHAPES, None).expect("product encoded");
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
        let product = encode_product_bytes(SHAPES, None).expect("product encoded");
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
        let held = encode_product_bytes(SHAPES, None).expect("product encoded");
        let wanted = rendered_selector(&encode_product_bytes(OTHER_SHAPES, None).expect("encoded"));
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
        let product = encode_product_bytes(SHAPES, None).expect("product encoded");
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

        let product = encode_product_bytes(SHAPES, None).expect("product encoded");
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
        let held = encode_product_bytes(SHAPES, None).expect("product encoded");
        let wanted = rendered_selector(&encode_product_bytes(OTHER_SHAPES, None).expect("encoded"));
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
        let product = encode_product_bytes(SHAPES, None).expect("product encoded");
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

        let product = encode_product_bytes(SHAPES, None).expect("product encoded");
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
}
