# Complete Columnar allocation delivery

Behavior: remove successful-validation temporary allocation and move checked
decoded blob payloads into their final store without copying.
Dependencies: existing Columnar/ContentStore and static metadata/schema homes.
Exclusions: unfinished typed grammar migration and later page-writer work.
Acceptance: the original five-table/two-codec allocation/ownership/frozen-output
contract below, strict affected compilation, normal hooks, hosted review/checks
and safe ghprsq archival/cleanup. Collect the full gate already in progress.
Risk: validation precedence, checksum ownership, deterministic output and actual
payload retention. Scope and tests are frozen; no optional review expansion.

Scope is the independently useful three-file production change already committed
as bb07611a9. The donor remains untouched. No typed grammar migration or later
page-writer changes enter this delivery. Parents 688 and 558 retain their full
remaining contracts. API compatibility is not a constraint for v4.

## Task 1: Transfer and qualify the production path

Borrow static expected metadata and encodings, move parsed keys, move checked
blob payloads into their final ContentStore, and construct the five output files
directly in their array. Source review confirms existing parsing, ordering and
checksum checks precede publication. Run all native Columnar library and both
integration targets, strict affected Clippy, repository-mandated checks and
release WASM portability. No general semantic WASM execution.

Acceptance: all five table schemas and both codecs validate with zero allocation
traffic; 1 KiB/1 MiB blob pointers and bytes survive without size-dependent
allocator traffic; original checksum/source/refusal ownership controls pass;
all ten frozen file identities remain unchanged. Keep actual logs and exit
status. Old donor checks are historical evidence only for their tested source.

## Task 2: Review, land and clean

Record normal signed commit and push hooks, open a focused PR closing only 702,
collect complete current-head checks/review threads and obtain actual independent
hosted review. Address substantive findings. Inspect the base delta, archive the
exact selected Stage, land only with ghprsq and verify actual closure/archive.
Use supported cleanup after examining ignored/untracked contents. Resume 489
next while preserving all unfinished donors and deprioritized SHACL work.

One owned Cargo invocation at a time; eight build jobs, four Rust/Rayon threads.
Four-hour active-work budget begins 2026-10-24 16:45 UTC; hosted waits excluded.
No upstream/external implementation or agent delegation.
