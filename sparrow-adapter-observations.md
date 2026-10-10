# Read-only downstream adapter observations

Inspected the actual native adapter in `/home/paudley/Active/sparrow/crates/sparrow-core/src/storage/rdf_view.rs` and `rdf_view/budget.rs` during implementation assistance. No sibling files, refs, processes or forge state were changed; no sibling build/test was run.

- `StoredRdfView<'_>` borrows a RocksDB snapshot but its budget is `Arc<Budget>`. The concrete `Reservation` owns that Arc and its byte charge; it does not borrow the view. Its existing scoped RPIT return hides this ownership. An explicit owned-reservation implementation can therefore return the actual reservation without extending a borrowed lifetime.
- `ReadError` is `Arc<Error>`; `RdfReadEvidence` is a fixed-size numeric record. The bounded view exposes `max_owned_term_bytes()` and `storage_live_budget()`. It does not override `cardinality_estimate()`. Ordinary quads, reifier rows and annotation rows use separate tables; a generic default admission bound must not assume the segmented inclusive estimate.
- `Reservation::resize` checks the deadline/sticky source state before shrinking too. Failed shrink must retain conservative charge; final reservation Drop subtracts its actual retained bytes directly. Query ownership must not corrupt child/base accounting or leave the final guard allocated after its last covered owner drops.
- Native use sites currently appear in Sparrow storage tests, including graph-troubleshooting admission cases. This inspection verifies adapter shape and migration feasibility, not downstream qualification. PurRDF delivery and later sibling pin/API adoption remain distinct.

These facts were sent to the sole implementation writer. They supplement, rather than replace, the accepted issue and plan. No existing capability is suppressed or replaced by an unbounded copy.
