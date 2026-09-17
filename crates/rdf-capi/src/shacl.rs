// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_shacl_validate_to_sarif`: validate a data graph against a shapes graph
//! and return a SARIF 2.1.0 report — plus the prepared-shapes-product codec.
//!
//! The C-ABI counterpart of the Python/WASM `to_sarif` surface. It drives the
//! SHACL engine and its SARIF reporting boundary, writing the report bytes into
//! the shared [`PurrdfBuffer`].
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
    SarifOptions, ShapesProductRefusal, entail_to_ntriples_string, validate_to_sarif_string,
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
}
