// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_query` (typed result + row cursor) and `purrdf_query_json` (the
//! SPARQL 1.1/1.2 Query Results JSON convenience path).

use std::os::raw::c_char;

use purrdf_rs::{SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::NativeSparqlEngine;

use crate::buffer::PurrdfBuffer;
use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::rowcursor::PurrdfRowCursor;
use crate::status::PurrdfStatus;
use crate::{cstr_to_str, opt_cstr_to_str};

/// The discriminant written to `purrdf_query`'s `out_kind`.
const KIND_SOLUTIONS: i32 = 0;
const KIND_GRAPH: i32 = 1;
const KIND_BOOLEAN: i32 = 2;

/// The native SPARQL engine for the C ABI. `NOW()`/`RAND()`/`UUID()`/`STRUUID()`
/// are live by construction — `EvalCtx::new` samples the real host wall clock and
/// OS entropy itself, so no host-side clock/entropy wiring is needed here.
fn engine() -> NativeSparqlEngine {
    NativeSparqlEngine::new()
}

/// Run a SPARQL query over a frozen dataset, materializing the result.
unsafe fn run_query(
    dataset: *const PurrdfDataset,
    query: *const c_char,
    base_iri: *const c_char,
) -> Result<SparqlResult, PurrdfError> {
    unsafe {
        let query = cstr_to_str(query)?;
        let base_iri = opt_cstr_to_str(base_iri)?;
        // Evaluate over the frozen `Arc<RdfDataset>` directly via the native engine —
        // no oxigraph `Store` round-trip. `NativeSparqlEngine::query` is the single
        // `SparqlEngine` impl; its `Dataset` IS the `Arc<RdfDataset>` the
        // handle already owns.
        engine()
            .query(
                PurrdfDataset::arc(dataset),
                SparqlRequest {
                    query,
                    base_iri,
                    substitutions: &[],
                },
            )
            .map_err(|diagnostic| {
                PurrdfError::from_diagnostic(PurrdfStatus::QueryError, &diagnostic)
            })
    }
}

/// Execute a SPARQL query. The result shape is reported in `*out_kind`:
/// `0` = SELECT → `*out_rows` is a `PurrdfRowCursor` (free with
/// `purrdf_rowcursor_free`); `1` = CONSTRUCT/DESCRIBE → `*out_graph` is a
/// `PurrdfDataset` (free with `purrdf_dataset_free`); `2` = ASK → `*out_boolean`
/// is `0`/`1`. Exactly one output is set per kind. `base_iri` may be null.
///
/// # Safety
/// `dataset` must be a live handle; `query` must be a NUL-terminated C string;
/// the out-params must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_query(
    dataset: *const PurrdfDataset,
    query: *const c_char,
    base_iri: *const c_char,
    out_kind: *mut i32,
    out_rows: *mut *mut PurrdfRowCursor,
    out_graph: *mut *mut PurrdfDataset,
    out_boolean: *mut u8,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if dataset.is_null() || query.is_null() || out_kind.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_query",
                ));
            }
            match run_query(dataset, query, base_iri)? {
                SparqlResult::Solutions {
                    variables, rows, ..
                } => {
                    if out_rows.is_null() {
                        return Err(PurrdfError::new(
                            PurrdfStatus::NullPointer,
                            "out_rows is null for a SELECT result",
                        ));
                    }
                    *out_kind = KIND_SOLUTIONS;
                    *out_rows = PurrdfRowCursor::new(variables, rows).into_raw();
                }
                SparqlResult::Graph(graph) => {
                    if out_graph.is_null() {
                        return Err(PurrdfError::new(
                            PurrdfStatus::NullPointer,
                            "out_graph is null for a CONSTRUCT/DESCRIBE result",
                        ));
                    }
                    *out_kind = KIND_GRAPH;
                    *out_graph = PurrdfDataset::into_raw(graph);
                }
                SparqlResult::Boolean(value) => {
                    *out_kind = KIND_BOOLEAN;
                    if !out_boolean.is_null() {
                        *out_boolean = u8::from(value);
                    }
                }
            }
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Execute a SPARQL query and serialize the result to the SPARQL 1.1 Query
/// Results JSON format (SELECT and ASK) into `*out_buffer` (UTF-8). A
/// CONSTRUCT/DESCRIBE graph is rendered as N-Triples inside a documented
/// `{"graph": "..."}` envelope. The simple/robust path — no row cursor needed.
///
/// # Safety
/// `dataset` must be a live handle; `query` must be a NUL-terminated C string;
/// the out-params must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_query_json(
    dataset: *const PurrdfDataset,
    query: *const c_char,
    base_iri: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if dataset.is_null() || query.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_query_json",
                ));
            }
            let result = run_query(dataset, query, base_iri)?;
            // Delegate to the canonical SPARQL-Results serializer (purrdf S9). An
            // empty `ResultProvenance` yields byte-identical pure W3C SRJ for
            // SELECT/ASK; the CONSTRUCT-graph path is rendered by the crate's
            // wasm-clean rdf-core N-Triples writer.
            let outcome = purrdf_sparql_results::to_json(
                &result,
                &purrdf_sparql_results::ResultProvenance::default(),
            )
            .map_err(|e| {
                PurrdfError::new(
                    PurrdfStatus::QueryError,
                    format!("SPARQL results JSON serialization failed: {e}"),
                )
            })?;
            *out_buffer = PurrdfBuffer::into_raw(outcome.bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

#[cfg(test)]
mod tests {
    use purrdf_core::{RdfDatasetBuilder, TermValue};

    use super::*;

    /// `NOW()` must report the real wall clock through the C ABI's `engine()`.
    /// `year(NOW())` on any date after this crate existed is `>= 2025`; a
    /// frozen-epoch regression would yield `1970`.
    #[test]
    fn now_reports_the_real_wall_clock_year() {
        let dataset = RdfDatasetBuilder::new()
            .freeze()
            .expect("empty dataset freezes");
        let result = engine()
            .query(
                &dataset,
                SparqlRequest {
                    query: "SELECT (year(NOW()) AS ?y) WHERE {}",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("query evaluates");
        let SparqlResult::Solutions { rows, .. } = result else {
            panic!("expected a SELECT solutions result");
        };
        assert_eq!(rows.len(), 1, "empty WHERE yields exactly one solution");
        let cell = rows[0][0].as_ref().expect("?y is bound");
        let TermValue::Literal { lexical_form, .. } = cell else {
            panic!("?y must be a literal, got {cell:?}");
        };
        let year: i64 = lexical_form
            .parse()
            .unwrap_or_else(|e| panic!("?y `{lexical_form}` must parse as an integer: {e}"));
        assert!(year >= 2025, "year(NOW()) = {year}, expected >= 2025");
    }
}
