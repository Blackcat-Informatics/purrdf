Segmented construction reserved its declared ceilings before input and refused a
whole dictionary group when adjacent long terms exceeded the block size. Grow
resident storage with input and close front-coded blocks at their actual encoded
byte limit, with the record count as an upper bound.

Header admission, complete certification and on-demand term lookup consume
variable populations together. Preserve stable full-width IDs, header-only open,
the existing cache/admission ownership and original encoding for full-count blocks.
A single encoded record that cannot fit, including its fence and framing, still
returns `ConstructionLimit`. Older readers that require fixed populations need
the updated reader for newly variable-population images.

Validation:

- All 18 segmented and 15 stack integration cases pass on the combined source, including actual unequal
  dictionary/reverse block counts, long terms, high IDs, sparse reads, RDF 1.2
  side tables, streamed output, refusal, ownership and source faults.
- All 17 segmented cases execute in scalar and SIMD-enabled WASM configurations.
- All 21 affected units pass; strict Clippy and fresh helper hygiene pass.
- The original 2,000/200,000-claim file-backed evaluator control passes all eight
  lanes with unchanged LIMIT, bound-key, COUNT and 256 MiB acceptance. Routing
  bounds learned from authenticated reads survive payload eviction under the
  original memory owner. The 200,000-claim 4096/128 COUNT returns 400,000 rows at
  169,204,364 peak bytes and 996,135 provider requests on the combined profile-2 source.
- A tight-budget routing-growth regression reproduces the review finding and
  passes natively and on scalar/SIMD WASM. Only optional routing-hint Residency
  omits the hint without evicting payloads; payload, exact request, source and allocator failures retain
  their original typed propagation.
- Normal commit/push hooks and the current hosted gate/review are recorded
  separately; no owner assessment is presented as independent approval.

Landed profile 2 statistics streams and semantic certification are preserved.
Profile 1 requires regeneration from the RDF input, as documented on main.

The original evaluator/layout failure is repaired. Header-only open remains lazy;
the learned routing metadata owns no lexical bytes or payload pins. Historical
measurements remain labelled at their original scope. No timing acceptance is used.

Closes #560.

Final hosted gate: 49 successful checks, four expected skips on run 38876291016. Both material review threads are resolved. All 21 units and the extended pressure case with both cache sizes pass; scalar and SIMD pressure executions pass on the final source.
