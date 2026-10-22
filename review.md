# Focused implementation review

The current source, rather than donor ancestry or the stale graph, establishes
three erased native producers. Writer verify_all now moves ContentStoreError;
reader insert_checked moves it alongside its original zero-based row; freeze
moves the entire RdfDiagnostic. No alternate producer or formatting path remains
at those sites. Existing generic source-session transport is unchanged.

ContentDigest delegates to Digest32's existing byte record. ContentStoreError
visits its actual stored/actual fields exhaustively. New columnar variants visit
original causes and borrow those same objects through standard source downcasts.
The reconstruction Display extracts storage only for compatibility text, never
for transport. Existing Clone/Eq traits remain; resident_clone! already implements
the diagnostic trait. No box or clone is added to any new transport branch.

Tests execute genuine core insert_checked and verify_all failures, including a
corrupt private-owner fixture because safe public insertion cannot create one.
Public reads execute corrupt-blob and invalid-absolute-IRI failures after a real
uncompressed write. Physical values are changed in place without weakening
Parquet boundaries. Blob row1 and both complete digest bytes remain; native
reconstruction agrees with the actual builder refusal after inputs are disposed.
Storage-bearing diagnostic controls retain detail, full-width location and a
native cause whose Display panics, whose clone count stays zero and whose owner
is released only with the returned error. All four transport and recursive
inspection quantities are zero. Full-width synthetic row control complements
the actual row1 producer; it is not represented as a huge executed input.

Existing native suite and portable goldens/inspection pass. Strict Clippy found
one missing visitor semicolon, corrected without changing behavior. Consumer and
remaining strict qualification are underway. This is implementer review, not
independent review; PR feedback and full hosted qualification remain required.

Complete helper hygiene then found introduced conservative capacity closure pin
invalidation. capacity-proof-review.md verifies unchanged167file membership,
all40sink authority identities and one matching sink per row, with only
content_store.rs changed in the core closure. Reporting additions add no capacity
constructor or owner path. Refreshing only40constructor pins is justified; no
exemption or authority changes. Complete helper rerun42341 exits0. Final native
and packaged-WASM six-case replay89900 exits0, including added human/source
assertions. Consumer libraries, strict Clippy/rustdoc, prose/route and fmt pass.
Normal signed commit79f7c7975 completes0. Independent PR review and hosted gates
remain required; these are not claimed by the local implementer review.

Parent558 retains every broader malformed/UTF-8, shipping-inventory and host
requirement. All four donors remain intact. No feature/dependency/wire/algorithm,
compiler/profile/cache change or unrelated repair. Three native wrappers are
one coherent public cause-preservation unit, not completion of the full parent.
