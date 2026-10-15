@coderabbitai review

Please review the ownership correction in commits `b12019ebe` and `5425211ec`.
Hosted native and WASM owner-release controls exposed retained demand-grown
request/cache-slot arrays in balances that previously assumed upfront capacity.
Evidence now captures their original reservation separately; strict owner-release
equalities, pin retention, live/peak ceilings and allocator comparisons remain.
The existing internal control independently checks both concrete Vec capacities.

Tight-session fixtures also now use the actual opening peak, including old/new
buffer overlap. Their typed query refusal, refusal before another read, exact
release and healthy-neighbor assertions remain. Check that these adaptations
cannot mask query-owned memory or silently relax the operational contract.

Focused execution passes: 22 SPARQL cases, 13 native and 13 portable segmented
controls, seven HNSW cases, both portable text-owner cases, native text focus and
cache ownership, and the internal escaped-request/capacity control. The required
final local gate and new-head hosted checks remain pending. The full 200,000-claim
file-backed LIMIT/COUNT contract remains required; no work is moved to follow-up.
