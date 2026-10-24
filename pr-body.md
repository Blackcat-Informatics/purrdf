The authenticated source directory stores an 88-byte record for every normalized
source row, including a repeated cumulative sum and implied population. Store
the original digest and physical address in 37 bytes and retain one cumulative
sum per 32 rows. Prefix reads reconstruct at most 31 tail digests through the
original admitted reader, with exact work accounting.

Certification independently rebuilds and compares all records and checkpoints
against RDF rows, including reifiers and annotations. Forged checkpoints remain
refused after recomputing the hash tree. Data profile 5 refuses earlier images
with an explicit regeneration message; no compatibility decoder is retained.

Native qualification passes 53 selected cases: the resident prefix control,
13 certification cases, 21 segmented cases, three source-directory cases and
15 chronological source-observation cases. The deterministic 1025-row fixture
uses 82 directory/checkpoint pages versus 205 under the previous packing at
512 bytes per page. This measures directory pages, not whole-image memory or
the external vault workload.

Strict compilation of the core library and three integration consumers passes
with warnings denied. The full helper hygiene target passes after reviewing and
rebinding the changed builder authority, aggregate constructor closure and two
shifted sink positions. Native ownership, work growth, refusal and cancellation
coverage pass; no general-semantic WASM execution is added.

Integration with current main passes strict consumer compilation, the exact
ported RDF 1.2 round-trip/isomorphism case and the full helper gate. Both source
sides are retained and the combined constructor pin is derived from the actual
merged source. Normal signed commit/push hooks pass. Independent review and
hosted qualification are pending.

Closes #708. Parent #705 remains open for oversized out-of-line terms, and #564
retains streaming construction/certification and common-order scan requirements.
