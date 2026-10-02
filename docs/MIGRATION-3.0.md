<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Migrating to PurRDF 3.0

PurRDF 3.0 changes the public boundaries below. Update consumers together with
all PurRDF packages in their language distribution.

## Exact JavaScript dataset identity

`Dataset.id` and `Dataset.generation` now return `bigint`. Update TypeScript
annotations and compare generations with bigint literals:

```javascript
const dataset = new Dataset();
const snapshot = dataset.snapshot();
if (dataset.generation === 0n) {
  console.log(dataset.id.toString());
}
const saved = JSON.stringify({ datasetId: dataset.id.toString() });
```

Use decimal strings when storing these values in JSON. Converting to `number`
can lose precision above `2^53`. Identity allocation and mutation generations
refuse exhaustion; a refused generation increment leaves the dataset untouched.
Snapshots retain independent identities and start at generation `0n`.

`AsyncEffect.exchangeId` also returns `bigint` for `SERVICE` and shared-exchange
wait effects. Pass that exact value to `AsyncJob.deliverExchangeBindings`,
`deliverExchangeFailure`, `faultExchange` and `exchangeIsOpen`. These methods
reject numeric, zero, negative and out-of-range identities without settling any
exchange. Keep exchange maps keyed by `bigint`; converting IDs to `number` can
alias adjacent exchanges above `2^53`. The final unsigned 64-bit ID is issued
once. Subsequent allocation fails with
`native-sparql-exchange-id-exhausted`, including under `SERVICE SILENT`.

## Logical source and GTS positions

The following public fields and corresponding constructor/accessor/callback
arguments use `u64`:

| Surface | Logical values |
|---|---|
| `purrdf_events::EventTermId` | Drive-global term identifier |
| `purrdf_events::SourceSpan` | Document byte offset and line number |
| `purrdf_iri::Position`, `BaseOrigin::Directive`, `purrdf_core::csv::CsvPosition` | Source line number |
| `RdfLocation` | Source line and GTS term/quad/reifier/frame/segment anchors |
| SARIF `Region` | Source line, byte offset and byte length |
| GTS `ByteRange`, streaming result and frame provenance | File byte ranges, torn offsets and segment/frame ordinals |
| GTS `StreamingSink`, `ResolvedSink` | Stream-global segment index |
| GTS streamability, replication inventory and RDF lookaside segment records | Frame coverage/tail counts, global ordinals, file lengths and ranges |
| RDF lookaside blob records | Declared decoded payload length |

Convert existing bounded integers with `u64::from(value)` or a checked
conversion appropriate to the input type. Only convert a logical address to
`usize` when accessing an actual bounded buffer, and check that buffer's range.
`FrameContext::verify` returns false for a range outside its supplied buffer.

Resident term/scope IDs, segment-local GTS term IDs, in-buffer offsets and
local columns retain their bounded representations. `LineIndex::try_locate`
returns `PositionError::ColumnLimit` for an oversized local column. `locate`
is the convenience for sources known to fit that bound. Native text readers
refuse source position exhaustion rather than saturating or wrapping. GTS event
identifier exhaustion is `EventError::IdSpaceExhausted`; exhausted blank-node
scope assignment returns `gts-scope-limit`.
CSV source row exhaustion is `CsvErrorKind::SourceRowExhausted`; an oversized
error column returns `CsvErrorKind::ColumnLimit` without wrapping or panicking.

GTS container bytes and existing frozen vectors keep their established wire
encoding. The width migration changes host representations of their positions.

## Structured diagnostics

Construct `RdfDiagnostic` with `new` or `error`; the type is now
`#[non_exhaustive]`. Retain `..` when matching it. Its readable severity, code,
message, detail and location fields remain available.

A diagnostic can carry an opaque `DiagnosticPresentation` with a stable
message identity, named `DiagnosticParameter` values and structured secondary
detail. One code can identify several conditions, so use the presentation's
`message_id()` when selecting a message template. Payload arguments come from
typed producer data; consumers read `presentation()` without extracting fields
from English prose.

```rust
use purrdf::{
    DiagnosticParameter, DiagnosticPresentation, DiagnosticValue, RdfDiagnostic,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let presentation = DiagnosticPresentation::new(
        "example.lookup.missing",
        "term {term} is absent",
        vec![DiagnosticParameter::new("term", DiagnosticValue::Unsigned(42))],
    )?;
    let diagnostic = RdfDiagnostic::error("example-lookup", "")
        .with_presentation(presentation);
    assert_eq!(diagnostic.message, "term 42 is absent");
    Ok(())
}
```

The presentation constructor validates the exact template/argument contract,
including duplicate, missing and unused names. `with_presentation` renders the
compatibility English fields from validated data. `presentation()` is optional,
so existing textual producers remain readable.

`RdfDiagnostic::to_json()` emits `purrdf-diagnostic-v1`; logical 64-bit anchors
and typed integer arguments use decimal strings to remain exact in JSON hosts.
Boolean arguments remain JSON booleans. SARIF carries the typed presentation in
`properties.diagnosticPresentation` alongside its standard message/location.
`properties.diagnosticRecord` retains the full record, including exact logical
anchors when standard SARIF numeric fields exceed a host's binary64 precision.

C callers can read the borrowed JSON record with
`purrdf_error_presentation_json(error)`. The pointer remains valid until
`purrdf_error_free(error)` and is null when that handle has no RDF diagnostic.
Existing status/message accessors retain their roles.

## Bounded operational query execution

A `DatasetView` can declare a shared live memory ceiling through
`storage_live_budget()` and a certified recursively expanded owned-term bound
through `max_owned_term_bytes()`. Engine execution reserves its working memory
before converting query lookup keys or creating an evaluation context. The
reservation covers overlapping intermediate rows and complete owned result
staging; scoped interned callbacks retain it through consumption. A returned
owned result belongs to the caller after the engine returns.

Governed ingress admits its governor owner and fixed error-report staging before
allocation. Operational source-error text is rendered into a UTF-8-safe buffer
of at most 2,048 bytes with an explicit truncation marker; the fallible receipt
retains the complete typed root cause. Tight-capacity and long-host-label
regressions also check actual allocations against the shared ledger.

The certified execution shape is a plain variable-projected `SELECT` over one
basic graph pattern, using the store's default dataset and empty extension
configuration. Its bound uses certified matching-page cardinalities, including
ordinary, reifier and annotation streams. Subject-bound patterns count their
certified pruned range with the same bounded iterator before row admission. Unrelated dictionary terms do not
become a whole-dictionary scratch reservation. Other shapes, nested triple
patterns, prebound substitutions and configured extensions return
`native-sparql-workspace-unpriced`; arithmetic overflow in the bound returns
`native-sparql-workspace-bound-overflow`. A shared capacity refusal preserves
the session's typed operational error and publishes no partial result.

Ordinary, prepared, scoped, governed, explanation and typed CONSTRUCT convenience
methods and prepared `bind_id` calls returning diagnostics now require `DatasetView<ReadError = Infallible>`.
A generic adapter calling them must carry that bound. SPARQL-backed retrieval
execution/search and the archive/DCAT CONSTRUCT projection bridge carry the same
infallible bound; their existing outputs do not certify a bounded operational
execution. Operational storage uses
`FallibleDatasetView`, whose checkpoint error and `DatasetView::ReadError` are
now the same type. Its prepared/unprepared query and shared-governor entry points
retain the reporting reservation through materialization and the final checkpoint.
A refused reservation remains a typed operational cause even before a sticky
failure is recorded.

For retained scoped execution, use `execute_fallible` over the same prepared
execution and evaluator. It returns `(visitor_value, final_evidence)` only after
the visit and final read checkpoint succeed; publish the visitor value after that
`Ok`. An iterator or term failure, including one during the visit, discards the
return value. Typed resident `Infallible` monomorphs retain native parallel
capability; operational readers execute sequentially.

Use the engine's fallible entry points for operational storage. Raw `eval` and
`evaluate_query` with a bounded view return a typed unpriced refusal because
those callers do not retain the engine's result-drain reservation. Resident
views keep their ordinary APIs and parallel execution.

TriG parsing and export now preserve declaration-only empty named graphs as
well as all three RDF streams. Compare declarations separately when validating
a canonical RDF roundtrip: canonical statements alone cannot distinguish an
explicit empty graph from its absence.

The unpublished `wasm_storage_qualification` example in `purrdf-envelope-probe`
runs the existing frozen resident browser profile and a supplementary scale
with at least 100,000 actual RDF rows, reusing the same workload implementation.
It uses these public APIs to scan a certified million-row source, run an exact
two-pattern join and drain its complete TriG output. Prepare it with
`scripts/prepare-wasm-storage-qualification.sh SOURCE RECEIPT TRUSTED_PIN NEW_DIRECTORY`,
using the receipt pin retained independently from successful full certification.
An optional final byte count selects a lower power-of-two cap. The script
verifies the requested maximum (256 MiB by default) in the emitted module's
memory declaration;
the browser records its observed linear-memory peak and the separate 64 MiB
reader ledger. Source bytes remain inside the module cap. This browser workload
does not qualify a physical ARM board or supply comparative timing evidence.


## Python canonical five-table exports

`gts_to_sqlite`, `gts_to_duckdb` and `gts_to_parquet` now use the native v1
projection shared with Rust. Read the canonical rows and their schema through
`gts_columnar_rows_from_bytes`; obtain native Parquet bytes through
`gts_columnar_parquet_from_bytes`. `RdfDataset.columnar_rows()` and
`RdfDataset.to_parquet_files()` expose the same projection for resident RDF.

The five tables are `terms`, `quads`, `reifiers`, `annotations` and `blobs`.
They preserve blank-node scope, directional literals, nested quoted terms,
graph slots and declaration-only empty named graphs; `terms` includes `scope`
and `named_graph`. Export term IDs follow canonical RDF value order.
`gts_relational_rows_from_bytes` remains the folded inspection API with its
append-order IDs, so those IDs must not be used to join canonical exports.
Inline blob payloads are verified before export; refused payloads fail the
export. Parquet is emitted by the first-party writer, and SQLite/DuckDB replace
the five projection tables in one transaction.

## Rust dataset read guards and logical addresses

`DatasetView` now declares `ReadError` and `TermGuard<'a>`. Its forward lookup
returns `Result<TermGuard<'_>, ReadError>`; reverse lookup returns
`Result<Option<Id>, ReadError>`. Import `TermGuard` to inspect a guard's borrowed
`term()` value, keeping the guard alive for every borrowed string you use:

```rust
use purrdf_core::{DatasetView, TermGuard, TermRef};

fn iri_length<D: DatasetView>(view: &D, id: D::Id) -> Result<Option<usize>, D::ReadError> {
    let guard = view.resolve(id)?;
    Ok(match guard.term() {
        TermRef::Iri(iri) => Some(iri.len()),
        _ => None,
    })
}
```

`with_term` scopes one inspection. `resolve_batch` and `lookup_batch` visit a
batch without constructing an output vector. Generic `quad_refs` yields
`Result<ResolvedQuad<TermGuard<'_>>, ReadError>`; `as_ref()` borrows the resolved
row while its guards remain alive. Drop pins at the end of each lexical read,
before unrelated block reads. Resident `RdfDataset` keeps its inherent borrowed
conveniences; on `Arc<RdfDataset>`, use `as_ref()` to select those inherent APIs.
Its generic guard is the same borrowed `TermRef`, with `Infallible` read errors.

Row streams keep their iterator shape. Operational readers latch a typed failure
instead of publishing a successful partial stream. Wrap a complete drain in
`checked_read`, which checks the reader before and after the drain and discards
its result if either checkpoint fails. Use the fallible `try_*` canonicalization,
import and graph-role APIs for operational storage. Materializing a view requires
separate admission for the owned output; a cache limit alone does not price it.
Implementors forward `read_error`, shared workspace reservations and certified
bounds when adapting an operational view. `FallibleDatasetView::Error` must be
the same type as `DatasetView::ReadError`, including workspace-admission refusals.
SHACL's `ShaclRead` seam explicitly
requires a resident `Infallible` carrier and a borrowed `resolve_term` accessor.

`GlobalTermId::index`, `PageId` and provider `page_count` use `u64`. `DatasetView`
term counts, row and text size hints, and cardinality estimates use `u64`;
`PagePart`/`PageSlot` logical quad counts also use `u64`. `ViewTermId::encode`
returns `u128` so composite and global identities do not truncate. Use
`GlobalTermId::checked_from_index` at untrusted ingress. Check conversion to
`usize` only when addressing an admitted local allocation. Resident `TermId`
remains four bytes and a resident quad remains sixteen bytes.

The provider-backed `SegmentedSession` preserves insertion IDs through sealing
and reopening. Persisted handles include the exact snapshot identity; reject a
handle attached to another snapshot even when its ordinal is in range. Reopening
requires an authenticated complete-certification receipt, which attests truthful
indexes and RDF closure. Hashing arbitrary unvalidated input is insufficient.
The existing eager pack reader remains available. Pack v1 did not encode
declaration-only empty graphs; restate them from an explicit trusted sidecar
when migrating. See
[storage contracts](../crates/rdf-core/STORAGE.md) for admission, certification,
range-provider and migration details.
