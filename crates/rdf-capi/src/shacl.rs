// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_shacl_validate_to_sarif`: validate a data graph against a shapes graph
//! and return a SARIF 2.1.0 report.
//!
//! The C-ABI counterpart of the Python/WASM `to_sarif` surface. It drives the
//! SHACL engine and its SARIF reporting boundary, writing the report bytes into
//! the shared [`PurrdfBuffer`].

use std::os::raw::c_char;

use purrdf_validate::{SarifOptions, entail_to_ntriples_string, validate_to_sarif_string};

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
}
