// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_query` (typed result + row cursor) and `purrdf_query_json` (the
//! SPARQL 1.1/1.2 Query Results JSON convenience path).

use std::os::raw::c_char;

use purrdf_rs::{
    ClosureRelations, GovernedEntailment, QueryEntailmentPlan, SparqlEngine, SparqlRequest,
    SparqlResult, query_with_entailment_governed,
};
use purrdf_sparql_eval::{
    AggregateRegistry, BudgetExhausted, GovernedOutcome, GovernedUpdateOutcome, NativeSparqlEngine,
    PartialAnswers, QueryOptions,
};
use sha2::{Digest as _, Sha256};

use crate::buffer::PurrdfBuffer;
use crate::error::PurrdfError;
use crate::governor::{
    PurrdfGovernorEvidence, PurrdfGovernorTrip, PurrdfQueryGovernors, decode_governors,
    encode_evidence, encode_trip, validate_update_governors,
};
use crate::handles::PurrdfDataset;
use crate::rowcursor::PurrdfRowCursor;
use crate::status::PurrdfStatus;
use crate::term::PurrdfStr;
use crate::{cstr_to_str, opt_cstr_to_str};

/// The result-form discriminant written to every query entry point's `out_kind`.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurrdfResultKind {
    /// A SELECT solution sequence returned through `PurrdfRowCursor`.
    Solutions = 0,
    /// A CONSTRUCT or DESCRIBE graph returned through `PurrdfDataset`.
    Graph = 1,
    /// An ASK boolean returned through `uint8_t`.
    Boolean = 2,
}

const KIND_SOLUTIONS: i32 = PurrdfResultKind::Solutions as i32;
const KIND_GRAPH: i32 = PurrdfResultKind::Graph as i32;
const KIND_BOOLEAN: i32 = PurrdfResultKind::Boolean as i32;
const KIND_NONE: i32 = -1;

/// The outcome discriminant written by [`purrdf_query_governed`].
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurrdfQueryOutcomeKind {
    /// The query completed and the returned result is exhaustive.
    Complete = 0,
    /// A governor stopped the query; consult the partial certificate and evidence.
    BudgetExhausted = 1,
}

/// The outcome discriminant written by [`purrdf_update_governed`].
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurrdfUpdateOutcomeKind {
    /// The entire request applied.
    Applied = 0,
    /// A governor stopped the request and no mutation applied.
    BudgetExhausted = 1,
}

/// The two-phase outcome discriminant written by [`purrdf_query_entailment_governed`].
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurrdfEntailmentQueryOutcomeKind {
    /// Closure and query both completed.
    Complete = 0,
    /// Closure completed, but a query governor tripped; partial certificate is available.
    QueryBudgetExhausted = 1,
    /// A cancellation/deadline stopped closure; no query ran and no report exists.
    ClosureStopped = 2,
}

/// Evidence for both phases of a governed entailment-aware query.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PurrdfGovernedEntailmentEvidence {
    /// `1` when closure completed and phase two ran; `0` on closure stop.
    pub query_ran: u8,
    /// Reserved for ABI-compatible extension; must be zero.
    pub reserved: [u8; 7],
    /// Phase-two evidence, or an all-zero carrier when `query_ran == 0`.
    pub query: PurrdfGovernorEvidence,
    /// Closure-phase stop, or `kind == NONE` when closure completed.
    pub closure_trip: PurrdfGovernorTrip,
}

/// What a governed query's partial result certifies.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurrdfPartialKind {
    /// A complete query has no partial certificate.
    None = 0,
    /// Every returned item is certainly an answer; more may exist.
    Certain = 1,
    /// Every true answer is in the returned result; some returned items may be extra.
    AtMost = 2,
    /// No sound bound survived; no result item crosses the ABI.
    Unknown = 3,
}

/// The certificate paired with a governed query result.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PurrdfPartialCertificate {
    /// [`PurrdfPartialKind`] discriminant.
    pub kind: i32,
    /// `1` when the returned items are the true output's ordered prefix.
    pub positional_prefix: u8,
    /// Operator label when `kind == UNKNOWN`; process-lifetime borrowed UTF-8 otherwise empty.
    pub barrier: PurrdfStr,
}

impl PurrdfPartialCertificate {
    fn none() -> Self {
        Self {
            kind: PurrdfPartialKind::None as i32,
            positional_prefix: 0,
            barrier: PurrdfStr::empty(),
        }
    }
}

/// The native SPARQL engine for the C ABI. `NOW()`/`RAND()`/`UUID()`/`STRUUID()`
/// are live by construction — `EvalCtx::new` samples the real host wall clock and
/// OS entropy itself, so no host-side clock/entropy wiring is needed here.
fn engine() -> NativeSparqlEngine {
    NativeSparqlEngine::new()
}

/// Decode `aggregate_namespace` (nullable, `opt_cstr_to_str`'s convention for every
/// other optional string parameter on this ABI) and build the statistical-aggregate
/// registry it requests, or `None` for a null pointer.
///
/// This is the ENTIRE C ABI surface for purrdf's first-party statistical aggregate set
/// (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
/// `FIRST`, `LAST`, `TOPK` — see `purrdf_sparql_eval::stat_agg`):
/// `AggregateRegistry::register_statistical_aggregates` takes only an IRI namespace
/// string, so it crosses the ABI as one nullable `const char*`, with no callback and no
/// per-aggregate marshaling. The GENERAL custom-aggregate seam
/// (`purrdf_sparql_eval::agg_fn::AggregateRegistry::register`, an arbitrary
/// `init`/`step`/`combine`/`finish` closure) is Rust-host-only and genuinely cannot cross
/// a C boundary as a string at all — this crate exposes no surface for it, with or
/// without this parameter.
///
/// # Safety
/// Same contract as [`opt_cstr_to_str`].
unsafe fn decode_aggregate_namespace(
    aggregate_namespace: *const c_char,
) -> Result<Option<AggregateRegistry>, PurrdfError> {
    unsafe {
        let Some(namespace) = opt_cstr_to_str(aggregate_namespace)? else {
            return Ok(None);
        };
        let mut registry = AggregateRegistry::new();
        registry.register_statistical_aggregates(namespace);
        Ok(Some(registry))
    }
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

/// Clear whichever optional result outputs the caller supplied.
unsafe fn clear_result_outputs(
    out_kind: *mut i32,
    out_rows: *mut *mut PurrdfRowCursor,
    out_graph: *mut *mut PurrdfDataset,
    out_boolean: *mut u8,
) {
    unsafe {
        *out_kind = KIND_NONE;
        if !out_rows.is_null() {
            *out_rows = std::ptr::null_mut();
        }
        if !out_graph.is_null() {
            *out_graph = std::ptr::null_mut();
        }
        if !out_boolean.is_null() {
            *out_boolean = 0;
        }
    }
}

/// Store one ordinary complete-or-certified-partial result through the existing C result
/// carriers.
unsafe fn store_result(
    result: SparqlResult,
    out_kind: *mut i32,
    out_rows: *mut *mut PurrdfRowCursor,
    out_graph: *mut *mut PurrdfDataset,
    out_boolean: *mut u8,
) -> Result<(), PurrdfError> {
    unsafe {
        match result {
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
        Ok(())
    }
}

/// Store either arm of a governed query through the common C result/certificate carriers.
unsafe fn store_governed_query_outcome(
    outcome: GovernedOutcome,
    out_kind: *mut i32,
    out_rows: *mut *mut PurrdfRowCursor,
    out_graph: *mut *mut PurrdfDataset,
    out_boolean: *mut u8,
    out_partial: *mut PurrdfPartialCertificate,
) -> Result<(PurrdfQueryOutcomeKind, PurrdfGovernorEvidence), PurrdfError> {
    unsafe {
        match outcome {
            GovernedOutcome::Complete {
                result, evidence, ..
            } => {
                store_result(result, out_kind, out_rows, out_graph, out_boolean)?;
                Ok((PurrdfQueryOutcomeKind::Complete, encode_evidence(&evidence)))
            }
            GovernedOutcome::BudgetExhausted(BudgetExhausted {
                evidence, partial, ..
            }) => {
                match partial {
                    PartialAnswers::Certain(partial) => {
                        *out_partial = PurrdfPartialCertificate {
                            kind: PurrdfPartialKind::Certain as i32,
                            positional_prefix: u8::from(partial.is_positional_prefix()),
                            barrier: PurrdfStr::empty(),
                        };
                        store_result(
                            partial.into_result(),
                            out_kind,
                            out_rows,
                            out_graph,
                            out_boolean,
                        )?;
                    }
                    PartialAnswers::AtMost(partial) => {
                        *out_partial = PurrdfPartialCertificate {
                            kind: PurrdfPartialKind::AtMost as i32,
                            positional_prefix: u8::from(partial.is_positional_prefix()),
                            barrier: PurrdfStr::empty(),
                        };
                        store_result(
                            partial.into_result(),
                            out_kind,
                            out_rows,
                            out_graph,
                            out_boolean,
                        )?;
                    }
                    PartialAnswers::Unknown(barrier) => {
                        *out_partial = PurrdfPartialCertificate {
                            kind: PurrdfPartialKind::Unknown as i32,
                            positional_prefix: 0,
                            barrier: PurrdfStr::from_str(barrier.operator()),
                        };
                    }
                }
                Ok((
                    PurrdfQueryOutcomeKind::BudgetExhausted,
                    encode_evidence(&evidence),
                ))
            }
        }
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

/// Decode `provenance_prefix`/`provenance_iri` (both nullable) into an optional
/// [`purrdf_sparql_results::ProvenanceNamespace`] anchoring the additive `purrdf`
/// provenance extension on a JSON emission. Both null → `None` (pure W3C output,
/// unchanged from before these parameters existed). Exactly one null is a usage error:
/// a namespace needs both halves, and silently treating a lone prefix or IRI as "no
/// namespace" would be the exact silent-drop this ABI refuses everywhere else.
///
/// # Safety
/// Same contract as [`opt_cstr_to_str`].
unsafe fn decode_provenance_namespace(
    provenance_prefix: *const c_char,
    provenance_iri: *const c_char,
) -> Result<Option<purrdf_sparql_results::ProvenanceNamespace>, PurrdfError> {
    unsafe {
        let prefix = opt_cstr_to_str(provenance_prefix)?;
        let iri = opt_cstr_to_str(provenance_iri)?;
        match (prefix, iri) {
            (None, None) => Ok(None),
            (Some(prefix), Some(iri)) => {
                let namespace = purrdf_sparql_results::ProvenanceNamespace::new(prefix, iri)
                    .map_err(|e| PurrdfError::new(PurrdfStatus::ParseError, e.to_string()))?;
                Ok(Some(namespace))
            }
            _ => Err(PurrdfError::new(
                PurrdfStatus::NullPointer,
                "provenance_prefix and provenance_iri must be both null or both non-null",
            )),
        }
    }
}

/// Build the [`purrdf_sparql_results::ResultProvenance`] a JSON emission carries: empty
/// when no namespace was supplied (pure-W3C output), or populated with a content hash of
/// the query text plus this engine's label when one was. Mirrors
/// `crate::query::build_query_provenance` in the CLI. `solutions` stays empty:
/// per-solution source provenance is the evaluator/S11 derivation graph's progressive
/// fill (see `purrdf_sparql_results::ResultProvenance`'s module docs), not something
/// this ABI entry point can populate on its own.
fn build_query_provenance(
    namespace: Option<&purrdf_sparql_results::ProvenanceNamespace>,
    query: &str,
) -> purrdf_sparql_results::ResultProvenance {
    if namespace.is_none() {
        return purrdf_sparql_results::ResultProvenance::default();
    }
    let digest = Sha256::digest(query.as_bytes());
    purrdf_sparql_results::ResultProvenance {
        query_hash: Some(format!("sha256:{digest:x}")),
        engine: Some("purrdf-sparql-eval".to_owned()),
        solutions: Vec::new(),
    }
}

/// Execute a SPARQL query and serialize the result to the SPARQL 1.1 Query
/// Results JSON format (SELECT and ASK) into `*out_buffer` (UTF-8). A
/// CONSTRUCT/DESCRIBE graph is rendered as N-Quads inside a documented
/// `{"graph": "..."}` envelope. The simple/robust path — no row cursor needed.
///
/// The envelope carries EVERYTHING the result holds: the base quads, the RDF 1.2
/// statement layer (reifier declarations and annotations), and every row's named
/// graph. A `CONSTRUCT { GRAPH <g> { … } }` or a `DESCRIBE` over a TriG dataset is
/// therefore readable through this entry point without loss, and needs no loss
/// out-param, because nothing is dropped. A default-graph-only result is
/// byte-identical to the N-Triples this member used to hold: an N-Quads line with
/// no graph term IS the N-Triples line.
///
/// This is deliberately the WIDENING answer rather than the refusal
/// `purrdf_core::named_graph` supplies on the QUERY lane — the CLI's `query` and
/// `describe`, the wasm query surface, and Python's query results. Those three
/// refuse because their caller asked a QUESTION whose own text names the graph
/// (`CONSTRUCT { GRAPH <g> { … } }`) and then named a single-graph RDF syntax
/// (`turtle`, `RdfFormat.TURTLE`) to receive the answer in: the two halves of one
/// request contradict, and answering in a syntax that cannot hold the answer would
/// report a partial answer as a complete one. Here the caller names no RDF syntax at
/// all: `{"graph": …}` is PurRDF's own envelope member, not a W3C SPARQL-Results
/// member and not a selectable format. With no request to contradict, MAXIMAL
/// UTILITY makes carrying the graph strictly better than refusing — and refusing
/// would leave this ABI with no JSON path at all for a quad-template CONSTRUCT.
///
/// `purrdf_serialize` does NOT refuse, and is not a counter-example to any of that:
/// it is the TRANSCODE lane, where the caller hands over a dataset and a target
/// syntax and nothing in the request contradicts anything else in it. It FLATTENS —
/// every graph-scoped row is DROPPED, never folded into the default graph — and
/// charges the whole of that loss to `out_named_graph_rows_dropped`, the same
/// number the CLI's `convert` records as a `named-graph-rows-dropped` ledger entry,
/// wasm's `serializeWithLoss` returns as `namedGraphRowsDropped`, and Python's
/// `dump_with_loss` returns as `named_graph_rows_dropped`. No host refuses on that
/// lane. Two consequences a C caller must plan for: the count out-param is
/// independently nullable, so passing null DECLINES a report this call computes
/// either way; and a dataset whose rows are ALL graph-scoped flattens to a
/// well-formed EMPTY document with status `Ok`, which is the correct rendering of an
/// empty default graph and not an error. Use `purrdf_query` + `purrdf_serialize`
/// when a specific RDF media type is required, and READ
/// `out_named_graph_rows_dropped` when you do: it is the only place that lane says
/// what the media type could not hold.
///
/// `provenance_prefix`/`provenance_iri` (both nullable, both-or-neither) anchor the
/// additive `purrdf` provenance extension on a SELECT/ASK emission under that
/// `PREFIX`/`IRI`. Null leaves the output pure W3C SRJ, exactly as before these
/// parameters existed; a CONSTRUCT/DESCRIBE result never carries the extension
/// (it is not a SPARQL-results document). Read the extension back with
/// `purrdf_sparql_results::provenance_from_json` under the SAME namespace.
///
/// # Safety
/// `dataset` must be a live handle; `query` must be a NUL-terminated C string;
/// the out-params must be writable. `provenance_prefix`/`provenance_iri`, if non-null,
/// must be NUL-terminated UTF-8 C strings live for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_query_json(
    dataset: *const PurrdfDataset,
    query: *const c_char,
    base_iri: *const c_char,
    provenance_prefix: *const c_char,
    provenance_iri: *const c_char,
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
            let query_text = cstr_to_str(query)?;
            let namespace = decode_provenance_namespace(provenance_prefix, provenance_iri)?;
            let result = run_query(dataset, query, base_iri)?;
            // Delegate to the canonical SPARQL-Results serializer (purrdf S9). An
            // empty `ResultProvenance` (no namespace supplied) yields byte-identical
            // pure W3C SRJ for SELECT/ASK; the CONSTRUCT-graph path is rendered by the
            // crate's wasm-clean rdf-core N-QUADS writer — graph slots and the RDF 1.2
            // statement layer included — and never carries the extension (`to_json`
            // only appends it for `Solutions`/`Boolean`; a `Graph` result serializes
            // as `{"graph": "..."}` regardless).
            let provenance = build_query_provenance(namespace.as_ref(), query_text);
            let outcome = purrdf_sparql_results::to_json(&result, &provenance, namespace.as_ref())
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

/// Execute a SPARQL query under caller-supplied governors.
///
/// `*out_outcome` is a [`PurrdfQueryOutcomeKind`]. A complete outcome writes the ordinary
/// typed result and `partial.kind == NONE`. An exhausted outcome is still status `OK`:
/// `*out_evidence` names the trip, and `*out_partial` says whether the typed result is a
/// certain lower bound, an at-most upper bound, or withheld (`UNKNOWN`, `out_kind == -1`).
/// Result kinds retain `purrdf_query`'s `0` solutions / `1` graph / `2` boolean values.
///
/// `aggregate_namespace` (nullable) registers purrdf's first-party statistical aggregate
/// set under that IRI namespace, so the query text can call
/// `AGG(<{NAMESPACE}NAME>, args…)` for `MEDIAN`/`PERCENTILE`/`STDDEV`/`STDDEV_POP`/
/// `VARIANCE`/`VAR_POP`/`MODE`/`FIRST`/`LAST`/`TOPK`. Null leaves every one of the ten
/// names an ordinary unregistered custom-aggregate IRI, exactly as before this parameter
/// existed.
///
/// # Safety
/// `dataset`, `query`, and `governors` must remain live for the call. `out_outcome`,
/// `out_kind`, `out_evidence`, and `out_partial` must be writable. The shape-specific
/// result pointer must be writable when that shape is returned. Any enabled cancellation
/// handle must remain live until the call returns. `aggregate_namespace`, if non-null,
/// must be a NUL-terminated UTF-8 C string live for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_query_governed(
    dataset: *const PurrdfDataset,
    query: *const c_char,
    base_iri: *const c_char,
    aggregate_namespace: *const c_char,
    governors: *const PurrdfQueryGovernors,
    out_outcome: *mut i32,
    out_kind: *mut i32,
    out_rows: *mut *mut PurrdfRowCursor,
    out_graph: *mut *mut PurrdfDataset,
    out_boolean: *mut u8,
    out_evidence: *mut PurrdfGovernorEvidence,
    out_partial: *mut PurrdfPartialCertificate,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if dataset.is_null()
                || query.is_null()
                || governors.is_null()
                || out_outcome.is_null()
                || out_kind.is_null()
                || out_evidence.is_null()
                || out_partial.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null required pointer argument to purrdf_query_governed",
                ));
            }
            clear_result_outputs(out_kind, out_rows, out_graph, out_boolean);
            *out_partial = PurrdfPartialCertificate::none();

            let query = cstr_to_str(query)?;
            let base_iri = opt_cstr_to_str(base_iri)?;
            let governors = decode_governors(governors)?;
            let aggregates = decode_aggregate_namespace(aggregate_namespace)?;
            let outcome = engine()
                .query_governed(
                    PurrdfDataset::arc(dataset),
                    SparqlRequest {
                        query,
                        base_iri,
                        substitutions: &[],
                    },
                    QueryOptions {
                        aggregates: aggregates.as_ref().unwrap_or(&AggregateRegistry::EMPTY),
                        ..QueryOptions::EMPTY
                    },
                    &governors,
                )
                .map_err(|diagnostic| {
                    PurrdfError::from_diagnostic(PurrdfStatus::QueryError, &diagnostic)
                })?;

            let (kind, evidence) = store_governed_query_outcome(
                outcome,
                out_kind,
                out_rows,
                out_graph,
                out_boolean,
                out_partial,
            )?;
            *out_outcome = kind as i32;
            *out_evidence = evidence;
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Execute SPARQL over an explicitly named entailment closure under governors.
///
/// `regime` uses the shared spellings (`simple`, `rdf`, `rdfs`, `owl-rl`,
/// `owl-direct`, `rif`, `d`). `program` must be empty except for `rif`, where it is the
/// required RIF-in-XML document. On `COMPLETE` or `QUERY_BUDGET_EXHAUSTED`,
/// `*out_report` owns a byte-stable reasoning report (free with `purrdf_buffer_free`) and
/// the ordinary result/partial carriers describe phase two. On `CLOSURE_STOPPED`, no
/// query ran: result kind is `-1`, report is null, and `closure_trip` names the stop.
///
/// `aggregate_namespace` (nullable) behaves exactly as on [`purrdf_query_governed`]:
/// it registers purrdf's first-party statistical aggregate set under that IRI namespace
/// for the closure query's PARSE and its evaluation, so `AGG(<{NAMESPACE}NAME>, args…)`
/// reaches the entailment-aware lane exactly as it reaches the ordinary one. Null leaves
/// every one of the ten names an ordinary unregistered custom-aggregate IRI.
///
/// # Safety
/// All input strings and handles must remain live for the synchronous call. Required
/// out-pointers must be writable; any enabled cancellation handle must remain live until
/// return. Shape-specific result pointers are required when that shape is returned.
/// `aggregate_namespace`, if non-null, must be a NUL-terminated UTF-8 C string live for
/// the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_query_entailment_governed(
    dataset: *const PurrdfDataset,
    query: *const c_char,
    base_iri: *const c_char,
    regime: *const c_char,
    program: *const c_char,
    aggregate_namespace: *const c_char,
    governors: *const PurrdfQueryGovernors,
    out_outcome: *mut i32,
    out_kind: *mut i32,
    out_rows: *mut *mut PurrdfRowCursor,
    out_graph: *mut *mut PurrdfDataset,
    out_boolean: *mut u8,
    out_evidence: *mut PurrdfGovernedEntailmentEvidence,
    out_partial: *mut PurrdfPartialCertificate,
    out_report: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if dataset.is_null()
                || query.is_null()
                || regime.is_null()
                || program.is_null()
                || governors.is_null()
                || out_outcome.is_null()
                || out_kind.is_null()
                || out_evidence.is_null()
                || out_partial.is_null()
                || out_report.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null required pointer argument to purrdf_query_entailment_governed",
                ));
            }
            clear_result_outputs(out_kind, out_rows, out_graph, out_boolean);
            *out_partial = PurrdfPartialCertificate::none();
            *out_report = std::ptr::null_mut();

            let query = cstr_to_str(query)?;
            let base_iri = opt_cstr_to_str(base_iri)?;
            let regime = cstr_to_str(regime)?;
            let program = cstr_to_str(program)?;
            let plan = QueryEntailmentPlan::parse(regime, program)
                .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
            let governors = decode_governors(governors)?;
            let aggregates = decode_aggregate_namespace(aggregate_namespace)?;
            let outcome = query_with_entailment_governed(
                &engine(),
                PurrdfDataset::arc(dataset),
                SparqlRequest {
                    query,
                    base_iri,
                    substitutions: &[],
                },
                plan.entailment(),
                QueryOptions {
                    aggregates: aggregates.as_ref().unwrap_or(&AggregateRegistry::EMPTY),
                    ..QueryOptions::EMPTY
                },
                // This surface registers no relation at all, so there is none to re-derive
                // over the closure — `NONE` is the accurate claim here, not a default.
                &ClosureRelations::NONE,
                &governors,
            )
            .map_err(|error| match error {
                purrdf_rs::ReasoningError::Query(diagnostic) => {
                    PurrdfError::from_diagnostic(PurrdfStatus::QueryError, &diagnostic)
                }
                other => PurrdfError::new(PurrdfStatus::QueryError, other.to_string()),
            })?;

            match outcome {
                GovernedEntailment::Answered { outcome, report } => {
                    let (kind, query_evidence) = store_governed_query_outcome(
                        outcome,
                        out_kind,
                        out_rows,
                        out_graph,
                        out_boolean,
                        out_partial,
                    )?;
                    *out_outcome = match kind {
                        PurrdfQueryOutcomeKind::Complete => {
                            PurrdfEntailmentQueryOutcomeKind::Complete as i32
                        }
                        PurrdfQueryOutcomeKind::BudgetExhausted => {
                            PurrdfEntailmentQueryOutcomeKind::QueryBudgetExhausted as i32
                        }
                    };
                    *out_evidence = PurrdfGovernedEntailmentEvidence {
                        query_ran: 1,
                        reserved: [0; 7],
                        query: query_evidence,
                        closure_trip: PurrdfGovernorTrip::NONE,
                    };
                    *out_report = PurrdfBuffer::into_raw(
                        purrdf_validate::render_reasoning_report(&report).into_bytes(),
                    );
                }
                GovernedEntailment::ClosureStopped { tripped } => {
                    *out_outcome = PurrdfEntailmentQueryOutcomeKind::ClosureStopped as i32;
                    *out_evidence = PurrdfGovernedEntailmentEvidence {
                        query_ran: 0,
                        reserved: [0; 7],
                        query: PurrdfGovernorEvidence::EMPTY,
                        closure_trip: encode_trip(Some(tripped)),
                    };
                }
                _ => {
                    return Err(PurrdfError::new(
                        PurrdfStatus::QueryError,
                        "unsupported governed entailment outcome",
                    ));
                }
            }
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Apply one SPARQL UPDATE request under caller-supplied governors.
///
/// `*out_outcome` is a [`PurrdfUpdateOutcomeKind`]. On `APPLIED`, the dataset handle now
/// owns the new frozen snapshot. On `BUDGET_EXHAUSTED`, the handle retains the exact same
/// `Arc` and no mutation applied. Both outcomes return status `OK` plus evidence. An
/// enabled `MAX_ANSWERS` flag is invalid because UPDATE has no answer sequence.
///
/// `aggregate_namespace` (nullable) behaves exactly as on [`purrdf_query_governed`],
/// reachable from a `DELETE`/`INSERT … WHERE` clause through a nested
/// `SELECT … GROUP BY` — the only place SPARQL UPDATE's grammar admits an aggregate.
///
/// # Safety
/// `dataset` must be a live, exclusively borrowed handle; `request` a NUL-terminated C
/// string; `governors` live for the call; and both output pointers writable. Any enabled
/// cancellation handle must remain live until the call returns. `aggregate_namespace`, if
/// non-null, must be a NUL-terminated UTF-8 C string live for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_update_governed(
    dataset: *mut PurrdfDataset,
    request: *const c_char,
    base_iri: *const c_char,
    aggregate_namespace: *const c_char,
    governors: *const PurrdfQueryGovernors,
    out_outcome: *mut i32,
    out_evidence: *mut PurrdfGovernorEvidence,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if dataset.is_null()
                || request.is_null()
                || governors.is_null()
                || out_outcome.is_null()
                || out_evidence.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null required pointer argument to purrdf_update_governed",
                ));
            }
            validate_update_governors(governors)?;
            let request = cstr_to_str(request)?;
            let base_iri = opt_cstr_to_str(base_iri)?;
            let governors = decode_governors(governors)?;
            let aggregates = decode_aggregate_namespace(aggregate_namespace)?;
            let outcome = engine()
                .update_governed(
                    &mut (*dataset).0,
                    SparqlRequest {
                        query: request,
                        base_iri,
                        substitutions: &[],
                    },
                    QueryOptions {
                        aggregates: aggregates.as_ref().unwrap_or(&AggregateRegistry::EMPTY),
                        ..QueryOptions::EMPTY
                    },
                    &governors,
                )
                .map_err(|diagnostic| {
                    PurrdfError::from_diagnostic(PurrdfStatus::QueryError, &diagnostic)
                })?;
            match outcome {
                GovernedUpdateOutcome::Applied { evidence } => {
                    *out_outcome = PurrdfUpdateOutcomeKind::Applied as i32;
                    *out_evidence = encode_evidence(&evidence);
                }
                GovernedUpdateOutcome::BudgetExhausted { evidence, .. } => {
                    *out_outcome = PurrdfUpdateOutcomeKind::BudgetExhausted as i32;
                    *out_evidence = encode_evidence(&evidence);
                }
            }
            Ok(PurrdfStatus::Ok)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::sync::Arc;

    use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};

    use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};
    use crate::governor::{
        PurrdfGovernorFlag, PurrdfGovernorTripKind, PurrdfResourceDimension, PurrdfStopCause,
        purrdf_cancellation_cancel, purrdf_cancellation_free, purrdf_cancellation_new,
        purrdf_query_governors_init,
    };
    use crate::handles::purrdf_dataset_free;
    use crate::rowcursor::{purrdf_rowcursor_free, purrdf_rowcursor_next, purrdf_rowcursor_term};
    use crate::term::PurrdfTermView;

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

    fn initialized_governors() -> PurrdfQueryGovernors {
        let mut out = MaybeUninit::uninit();
        assert_eq!(unsafe { purrdf_query_governors_init(out.as_mut_ptr()) }, 0);
        unsafe { out.assume_init() }
    }

    fn query_dataset() -> *mut PurrdfDataset {
        let mut builder = RdfDatasetBuilder::new();
        let predicate = builder.intern_iri("http://example.org/p");
        for index in 0..3 {
            let subject = builder.intern_iri(&format!("http://example.org/s{index}"));
            let object = builder.intern_iri(&format!("http://example.org/o{index}"));
            builder.push_quad(subject, predicate, object, None);
        }
        PurrdfDataset::into_raw(builder.freeze().expect("freeze"))
    }

    fn entailment_dataset() -> *mut PurrdfDataset {
        let mut builder = RdfDatasetBuilder::new();
        let rdf_type = builder.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let subclass = builder.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
        let cat = builder.intern_iri("http://example.org/Cat");
        let animal = builder.intern_iri("http://example.org/Animal");
        let tom = builder.intern_iri("http://example.org/tom");
        builder.push_quad(cat, subclass, animal, None);
        builder.push_quad(tom, rdf_type, cat, None);
        PurrdfDataset::into_raw(builder.freeze().expect("freeze"))
    }

    #[test]
    fn governed_query_carries_partial_certificate_and_evidence() {
        let dataset = query_dataset();
        let query = CString::new("SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o }")
            .expect("query C string");
        let mut governors = initialized_governors();
        governors.enabled = PurrdfGovernorFlag::MaxAnswers as u32;
        governors.max_answers = 1;

        let mut outcome = -1;
        let mut kind = KIND_NONE;
        let mut rows = std::ptr::null_mut();
        let mut graph = std::ptr::null_mut();
        let mut boolean = 0;
        let mut evidence = MaybeUninit::uninit();
        let mut partial = MaybeUninit::uninit();
        let mut error = std::ptr::null_mut();
        let status = unsafe {
            purrdf_query_governed(
                dataset,
                query.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                &raw const governors,
                &raw mut outcome,
                &raw mut kind,
                &raw mut rows,
                &raw mut graph,
                &raw mut boolean,
                evidence.as_mut_ptr(),
                partial.as_mut_ptr(),
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        assert_eq!(outcome, PurrdfQueryOutcomeKind::BudgetExhausted as i32);
        assert_eq!(kind, KIND_SOLUTIONS);
        assert!(graph.is_null());

        let evidence = unsafe { evidence.assume_init() };
        assert_eq!(evidence.trip.kind, PurrdfGovernorTripKind::Budget as i32);
        assert_eq!(
            evidence.trip.dimension,
            PurrdfResourceDimension::AnswerRows as i32
        );
        assert_eq!(evidence.trip.limit, 1);
        assert_eq!(evidence.trip.consumed, 2);

        let partial = unsafe { partial.assume_init() };
        assert_eq!(partial.kind, PurrdfPartialKind::Certain as i32);
        assert_eq!(partial.positional_prefix, 1);
        let mut count = 0;
        loop {
            match unsafe { purrdf_rowcursor_next(rows) } {
                status if status == PurrdfStatus::Ok as i32 => count += 1,
                status if status == PurrdfStatus::CursorExhausted as i32 => break,
                status => panic!("unexpected row-cursor status {status}"),
            }
        }
        assert_eq!(count, 1);

        unsafe {
            purrdf_rowcursor_free(rows);
            purrdf_dataset_free(dataset);
        }
    }

    #[test]
    fn governed_entailment_query_carries_answer_and_report() {
        let dataset = entailment_dataset();
        let query = CString::new(
            "SELECT ?x WHERE { ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.org/Animal> }",
        )
        .expect("query");
        let regime = CString::new("rdfs").expect("regime");
        let program = CString::new("").expect("program");
        let governors = initialized_governors();
        let mut outcome = -1;
        let mut kind = KIND_NONE;
        let mut rows = std::ptr::null_mut();
        let mut evidence = MaybeUninit::uninit();
        let mut partial = MaybeUninit::uninit();
        let mut report = std::ptr::null_mut();
        let mut error = std::ptr::null_mut();

        let status = unsafe {
            purrdf_query_entailment_governed(
                dataset,
                query.as_ptr(),
                std::ptr::null(),
                regime.as_ptr(),
                program.as_ptr(),
                std::ptr::null(),
                &raw const governors,
                &raw mut outcome,
                &raw mut kind,
                &raw mut rows,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                evidence.as_mut_ptr(),
                partial.as_mut_ptr(),
                &raw mut report,
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        assert_eq!(outcome, PurrdfEntailmentQueryOutcomeKind::Complete as i32);
        assert_eq!(kind, KIND_SOLUTIONS);
        let evidence = unsafe { evidence.assume_init() };
        assert_eq!(evidence.query_ran, 1);
        assert_eq!(
            evidence.closure_trip.kind,
            PurrdfGovernorTripKind::None as i32
        );
        assert!(!report.is_null());
        let mut ptr = std::ptr::null();
        let mut len = 0;
        assert_eq!(
            unsafe { purrdf_buffer_data(report, &raw mut ptr, &raw mut len) },
            PurrdfStatus::Ok as i32
        );
        let report_text = std::str::from_utf8(unsafe { std::slice::from_raw_parts(ptr, len) })
            .expect("UTF-8 report");
        assert!(report_text.starts_with("purrdf-reasoning-report 4\n"));
        assert!(report_text.contains("\nregime rdfs\n"));

        unsafe {
            purrdf_buffer_free(report);
            purrdf_rowcursor_free(rows);
            purrdf_dataset_free(dataset);
        }
    }

    #[test]
    fn governed_entailment_query_exposes_a_closure_stop_without_report() {
        let dataset = entailment_dataset();
        let query = CString::new("ASK { ?s ?p ?o }").expect("query");
        let regime = CString::new("rdfs").expect("regime");
        let program = CString::new("").expect("program");
        let mut governors = initialized_governors();
        governors.enabled = PurrdfGovernorFlag::DeadlineMillis as u32;
        governors.deadline_millis = 0;
        let mut outcome = -1;
        let mut kind = KIND_NONE;
        let mut evidence = MaybeUninit::uninit();
        let mut partial = MaybeUninit::uninit();
        let mut report = std::ptr::null_mut();
        let mut error = std::ptr::null_mut();

        let status = unsafe {
            purrdf_query_entailment_governed(
                dataset,
                query.as_ptr(),
                std::ptr::null(),
                regime.as_ptr(),
                program.as_ptr(),
                std::ptr::null(),
                &raw const governors,
                &raw mut outcome,
                &raw mut kind,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                evidence.as_mut_ptr(),
                partial.as_mut_ptr(),
                &raw mut report,
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        assert_eq!(
            outcome,
            PurrdfEntailmentQueryOutcomeKind::ClosureStopped as i32
        );
        assert_eq!(kind, KIND_NONE);
        assert!(report.is_null());
        let evidence = unsafe { evidence.assume_init() };
        assert_eq!(evidence.query_ran, 0);
        assert_eq!(
            evidence.closure_trip.kind,
            PurrdfGovernorTripKind::Stopped as i32
        );
        assert_eq!(
            evidence.closure_trip.stop_cause,
            PurrdfStopCause::Deadline as i32
        );

        unsafe { purrdf_dataset_free(dataset) };
    }

    /// Three cats, each `rdfs:subClassOf`-entailed to be an `Animal`, carrying distinct
    /// `ex:weight` literals (1, 2, 3) — `MEDIAN` over the entailed `?s a Animal` binding
    /// folds to `2`.
    fn entailed_median_dataset() -> *mut PurrdfDataset {
        let mut builder = RdfDatasetBuilder::new();
        let rdf_type = builder.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let subclass = builder.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
        let weight = builder.intern_iri("http://example.org/weight");
        let cat = builder.intern_iri("http://example.org/Cat");
        let animal = builder.intern_iri("http://example.org/Animal");
        builder.push_quad(cat, subclass, animal, None);
        for (name, value) in [("tom", 1_i64), ("felix", 2), ("garfield", 3)] {
            let subject = builder.intern_iri(&format!("http://example.org/{name}"));
            builder.push_quad(subject, rdf_type, cat, None);
            let literal = builder.intern_literal(RdfLiteral::typed(
                value.to_string(),
                "http://www.w3.org/2001/XMLSchema#integer",
            ));
            builder.push_quad(subject, weight, literal, None);
        }
        PurrdfDataset::into_raw(builder.freeze().expect("freeze"))
    }

    /// End-to-end: `purrdf_query_entailment_governed`'s `aggregate_namespace` parameter
    /// reaches `MEDIAN` over bindings the RDFS closure itself produced — the reachability
    /// gap F10 closes. Before this parameter existed, the entailment-aware C ABI lane
    /// passed empty query options unconditionally, so no statistical aggregate could ever
    /// be registered on it, unlike `purrdf_query_governed`.
    #[test]
    fn aggregate_namespace_computes_median_through_entailment_governed_query() {
        const NS: &str = "https://example.org/agg#";
        let dataset = entailed_median_dataset();
        let query = CString::new(
            "PREFIX ex: <http://example.org/> \
             SELECT (AGG(<https://example.org/agg#MEDIAN>, ?w) AS ?m) \
             WHERE { ?s a ex:Animal . ?s ex:weight ?w }",
        )
        .expect("query");
        let regime = CString::new("rdfs").expect("regime");
        let program = CString::new("").expect("program");
        let namespace = CString::new(NS).expect("namespace C string");
        let governors = initialized_governors();
        let mut outcome = -1;
        let mut kind = KIND_NONE;
        let mut rows = std::ptr::null_mut();
        let mut evidence = MaybeUninit::uninit();
        let mut partial = MaybeUninit::uninit();
        let mut report = std::ptr::null_mut();
        let mut error = std::ptr::null_mut();

        let status = unsafe {
            purrdf_query_entailment_governed(
                dataset,
                query.as_ptr(),
                std::ptr::null(),
                regime.as_ptr(),
                program.as_ptr(),
                namespace.as_ptr(),
                &raw const governors,
                &raw mut outcome,
                &raw mut kind,
                &raw mut rows,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                evidence.as_mut_ptr(),
                partial.as_mut_ptr(),
                &raw mut report,
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32, "status");
        assert!(error.is_null(), "no error");
        assert_eq!(outcome, PurrdfEntailmentQueryOutcomeKind::Complete as i32);
        assert_eq!(kind, KIND_SOLUTIONS);
        assert!(!report.is_null());

        assert_eq!(
            unsafe { purrdf_rowcursor_next(rows) },
            PurrdfStatus::Ok as i32
        );
        let mut view = MaybeUninit::<PurrdfTermView>::uninit();
        let mut bound = 0u8;
        assert_eq!(
            unsafe { purrdf_rowcursor_term(rows, 0, view.as_mut_ptr(), &raw mut bound) },
            PurrdfStatus::Ok as i32
        );
        assert_eq!(bound, 1, "?m is bound");
        let view = unsafe { view.assume_init() };
        let lexical = unsafe { view.lexical.as_str() }.expect("UTF-8 lexical form");
        assert_eq!(lexical, "2", "MEDIAN of the entailed {{1, 2, 3}} is 2");

        unsafe {
            purrdf_buffer_free(report);
            purrdf_rowcursor_free(rows);
            purrdf_dataset_free(dataset);
        }
    }

    #[test]
    fn governed_update_trip_preserves_the_exact_dataset_arc() {
        let dataset = query_dataset();
        let before = unsafe { Arc::clone(&(*dataset).0) };
        let request = CString::new(
            "INSERT DATA { <http://example.org/new> <http://example.org/p> \
             <http://example.org/value> }",
        )
        .expect("update C string");
        let mut governors = initialized_governors();
        governors.enabled = PurrdfGovernorFlag::Fuel as u32;
        governors.fuel = 0;
        let mut outcome = -1;
        let mut evidence = MaybeUninit::uninit();
        let mut error = std::ptr::null_mut();

        let status = unsafe {
            purrdf_update_governed(
                dataset,
                request.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                &raw const governors,
                &raw mut outcome,
                evidence.as_mut_ptr(),
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        assert_eq!(outcome, PurrdfUpdateOutcomeKind::BudgetExhausted as i32);
        assert!(unsafe { Arc::ptr_eq(&before, &(*dataset).0) });
        assert_eq!(unsafe { (*dataset).0.quad_count() }, 3);
        let evidence = unsafe { evidence.assume_init() };
        assert_eq!(
            evidence.trip.dimension,
            PurrdfResourceDimension::Fuel as i32
        );

        unsafe { purrdf_dataset_free(dataset) };
    }

    #[test]
    fn governed_query_observes_a_prefired_c_cancellation() {
        let dataset = query_dataset();
        let query = CString::new("SELECT * WHERE { ?s ?p ?o }").expect("query C string");
        let mut cancellation = std::ptr::null_mut();
        assert_eq!(unsafe { purrdf_cancellation_new(&raw mut cancellation) }, 0);
        assert_eq!(unsafe { purrdf_cancellation_cancel(cancellation) }, 0);
        let mut governors = initialized_governors();
        governors.enabled = PurrdfGovernorFlag::Cancellation as u32;
        governors.cancellation = cancellation;

        let mut outcome = -1;
        let mut kind = KIND_NONE;
        let mut rows = std::ptr::null_mut();
        let mut evidence = MaybeUninit::uninit();
        let mut partial = MaybeUninit::uninit();
        let mut error = std::ptr::null_mut();
        let status = unsafe {
            purrdf_query_governed(
                dataset,
                query.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                &raw const governors,
                &raw mut outcome,
                &raw mut kind,
                &raw mut rows,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                evidence.as_mut_ptr(),
                partial.as_mut_ptr(),
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32);
        assert!(error.is_null());
        assert_eq!(outcome, PurrdfQueryOutcomeKind::BudgetExhausted as i32);
        let evidence = unsafe { evidence.assume_init() };
        assert_eq!(evidence.trip.kind, PurrdfGovernorTripKind::Stopped as i32);
        assert_eq!(evidence.trip.stop_cause, PurrdfStopCause::Cancelled as i32);

        unsafe {
            purrdf_rowcursor_free(rows);
            purrdf_cancellation_free(cancellation);
            purrdf_dataset_free(dataset);
        }
    }

    /// Three `xsd:integer` values (1, 2, 3) on distinct subjects under one predicate —
    /// the fixture `MEDIAN` folds to the value `2`.
    fn median_dataset() -> *mut PurrdfDataset {
        let mut builder = RdfDatasetBuilder::new();
        let predicate = builder.intern_iri("http://example.org/value");
        for value in [1_i64, 2, 3] {
            let subject = builder.intern_iri(&format!("http://example.org/s{value}"));
            let object = builder.intern_literal(RdfLiteral::typed(
                value.to_string(),
                "http://www.w3.org/2001/XMLSchema#integer",
            ));
            builder.push_quad(subject, predicate, object, None);
        }
        PurrdfDataset::into_raw(builder.freeze().expect("freeze"))
    }

    /// End-to-end: `purrdf_query_governed`'s `aggregate_namespace` parameter actually
    /// registers and COMPUTES `MEDIAN` through the C ABI — not merely that the
    /// parameter parses. This is the reachability gap this parameter closes: before it
    /// existed, `AggregateRegistry::register_statistical_aggregates` was reachable only
    /// by embedding the Rust engine directly.
    #[test]
    fn aggregate_namespace_computes_median_through_the_c_abi() {
        const NS: &str = "https://example.org/agg#";
        let dataset = median_dataset();
        let query = CString::new(
            "PREFIX ex: <http://example.org/> \
             SELECT (AGG(<https://example.org/agg#MEDIAN>, ?v) AS ?m) \
             WHERE { ?s ex:value ?v }",
        )
        .expect("query C string");
        let namespace = CString::new(NS).expect("namespace C string");
        let governors = initialized_governors();

        let mut outcome = -1;
        let mut kind = KIND_NONE;
        let mut rows = std::ptr::null_mut();
        let mut evidence = MaybeUninit::uninit();
        let mut partial = MaybeUninit::uninit();
        let mut error = std::ptr::null_mut();
        let status = unsafe {
            purrdf_query_governed(
                dataset,
                query.as_ptr(),
                std::ptr::null(),
                namespace.as_ptr(),
                &raw const governors,
                &raw mut outcome,
                &raw mut kind,
                &raw mut rows,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                evidence.as_mut_ptr(),
                partial.as_mut_ptr(),
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32, "status");
        assert!(error.is_null(), "no error");
        assert_eq!(outcome, PurrdfQueryOutcomeKind::Complete as i32);
        assert_eq!(kind, KIND_SOLUTIONS);

        assert_eq!(
            unsafe { purrdf_rowcursor_next(rows) },
            PurrdfStatus::Ok as i32
        );
        let mut view = MaybeUninit::<PurrdfTermView>::uninit();
        let mut bound = 0u8;
        assert_eq!(
            unsafe { purrdf_rowcursor_term(rows, 0, view.as_mut_ptr(), &raw mut bound) },
            PurrdfStatus::Ok as i32
        );
        assert_eq!(bound, 1, "?m is bound");
        let view = unsafe { view.assume_init() };
        let lexical = unsafe { view.lexical.as_str() }.expect("UTF-8 lexical form");
        assert_eq!(lexical, "2", "MEDIAN of {{1, 2, 3}} is 2");

        unsafe {
            purrdf_rowcursor_free(rows);
            purrdf_dataset_free(dataset);
        }
    }

    /// Regression: the namespace stays caller-supplied with no fabricated default —
    /// omitting `aggregate_namespace` (a null pointer) leaves the ten statistical names
    /// unregistered, and the SAME typed error an ordinary unregistered custom-aggregate
    /// IRI already produces surfaces here, unchanged.
    #[test]
    fn omitted_aggregate_namespace_leaves_the_statistical_set_unregistered() {
        let dataset = median_dataset();
        let query = CString::new(
            "PREFIX ex: <http://example.org/> \
             SELECT (AGG(<https://example.org/agg#MEDIAN>, ?v) AS ?m) \
             WHERE { ?s ex:value ?v }",
        )
        .expect("query C string");
        let governors = initialized_governors();

        let mut outcome = -1;
        let mut kind = KIND_NONE;
        let mut rows = std::ptr::null_mut();
        let mut evidence = MaybeUninit::uninit();
        let mut partial = MaybeUninit::uninit();
        let mut error = std::ptr::null_mut();
        let status = unsafe {
            purrdf_query_governed(
                dataset,
                query.as_ptr(),
                std::ptr::null(),
                std::ptr::null(), // no aggregate_namespace
                &raw const governors,
                &raw mut outcome,
                &raw mut kind,
                &raw mut rows,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                evidence.as_mut_ptr(),
                partial.as_mut_ptr(),
                &raw mut error,
            )
        };
        assert_ne!(
            status,
            PurrdfStatus::Ok as i32,
            "unregistered aggregate refuses"
        );
        assert!(!error.is_null());
        unsafe {
            let message = crate::error::purrdf_error_message(error);
            let text = std::ffi::CStr::from_ptr(message).to_string_lossy();
            assert!(
                text.contains("aggregate") || text.contains("registered"),
                "error should name the unregistered aggregate, got: {text}"
            );
            crate::error::purrdf_error_free(error);
            purrdf_dataset_free(dataset);
        }
    }

    /// End-to-end: `purrdf_update_governed`'s `aggregate_namespace` reaches `MEDIAN`
    /// from a `DELETE`/`INSERT … WHERE` clause through a nested `SELECT … GROUP BY` —
    /// the only place SPARQL UPDATE's grammar admits an aggregate.
    #[test]
    fn aggregate_namespace_computes_median_through_a_governed_update() {
        const NS: &str = "https://example.org/agg#";
        let dataset = median_dataset();
        let update = CString::new(
            "PREFIX ex: <http://example.org/> \
             INSERT { ex:summary ex:median ?m } \
             WHERE { SELECT (AGG(<https://example.org/agg#MEDIAN>, ?v) AS ?m) \
                     WHERE { ?s ex:value ?v } }",
        )
        .expect("update C string");
        let namespace = CString::new(NS).expect("namespace C string");
        let governors = initialized_governors();

        let mut outcome = -1;
        let mut evidence = MaybeUninit::uninit();
        let mut error = std::ptr::null_mut();
        let status = unsafe {
            purrdf_update_governed(
                dataset,
                update.as_ptr(),
                std::ptr::null(),
                namespace.as_ptr(),
                &raw const governors,
                &raw mut outcome,
                evidence.as_mut_ptr(),
                &raw mut error,
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32, "status");
        assert!(error.is_null(), "no error");
        assert_eq!(outcome, PurrdfUpdateOutcomeKind::Applied as i32);

        let check = engine()
            .query(
                unsafe { &(*dataset).0 },
                SparqlRequest {
                    query: "PREFIX ex: <http://example.org/> \
                            SELECT ?m WHERE { ex:summary ex:median ?m }",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("check query evaluates");
        let SparqlResult::Solutions { rows, .. } = check else {
            panic!("expected SELECT solutions");
        };
        assert_eq!(rows.len(), 1);
        let cell = rows[0][0].as_ref().expect("?m is bound");
        let TermValue::Literal { lexical_form, .. } = cell else {
            panic!("?m must be a literal, got {cell:?}");
        };
        assert_eq!(lexical_form, "2");

        unsafe { purrdf_dataset_free(dataset) };
    }
}
