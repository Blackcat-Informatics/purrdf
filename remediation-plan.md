# Retained reader bookkeeping in owner-conservation controls

Hosted wasm-exec and native integration shards 1 and 4 exposed release checks
that assumed reader arrays had their final capacity at opening. That assumption
was true for the old maximum-sized arrays and is false after demand-grown
request/cache-slot admission. The reader deliberately retains these buffers
across query results. This does not authorize ignoring a query or pin leak.

The coherent correction captures the original State.storage reservation in
SegmentedEvidence::bookkeeping_bytes, alongside live/peak and evictable blocks.
Only State's two retained Vec buffers use that reservation. An existing internal
escaped-owner test independently compares it with both concrete capacities and
element layouts. Existing external owner balances subtract that category at
each checkpoint; exact equalities, live/peak ceilings, retained-result and
pinned-block checks remain. No production admission or allocation policy changes.

Affected consumers are core segmented-provider controls, SPARQL segmented query
ownership and repeated explain, HNSW bounded search, and text physical-shape,
cache and focus routes. Check their actual native cases and portable registered
targets before final qualification. The authored 200,000-claim file-backed
contract remains required, including full COUNT and all original budget floors.

The former final local gate was stopped after this demonstrated hosted defect;
it is not qualification. Publish this correction normally, refresh material
review, complete the required final local gate and new-head hosted checks, then
merge through ghprsq. No issue work is deferred. CodeQL's separate aggregate
histogram false positive is documented in tasks/codeql-disposition.md.
