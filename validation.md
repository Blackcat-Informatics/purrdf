# Compact source directory acceptance

VERDICT: PASS
Scope: the frozen compact directory, original source addresses, sparse prefix
summaries, independent certification and native admitted readers.
Evidence: resident-prefix.txt exits 0 for the existing resident checkpoint case.
native-certification.txt exits 0 for all 13 selected certification cases,
including independently rehashed record/checkpoint corruption, deterministic
image bytes and every prefix of a 1025-row fixture across physical pages and
checkpoint boundaries. Exact tail work matches the number of visited digests.
The fixture uses 82 compact directory/checkpoint pages against 205 pages under
the previous 88-byte directory packing at the same 512-byte page size. This is
a directory-page comparison; it does not reproduce the external vaults or claim
a measured whole-image peak reduction.
native-consumers.txt exits 0: 21 segmented reader/image cases, three resident
source-directory cases and 15 chronological source-observation cases pass,
none ignored. These include reifiers/annotations, overlap, removal/reinsertion,
compaction, cancellation/read refusal, exact work growth and owner release.
Findings: the first strict compile exposed the missing temporary header field;
the first certification run exposed a missing checkpoint stream in the block
count. Both are corrected, and the actual named certification cases then pass.
The initial native filter selected only the resident prefix case; no segmented
runtime acceptance is inferred from that first run.
core-clippy.txt exits 0 with -D warnings for the core library and the three
integration consumers. helpers-gate.txt exits 0 for the full helper hygiene
target, including its source/capacity census, self-tests and hash-domain checks.
Constructor-source membership is unchanged from the integration base; original
sink fingerprints remain intact. Four builder authorities and the aggregate
closure are rebound to reviewed source; two sink positions move by one line.
No proof class, reason or enforcement is removed. The changed codec and reader
do not alter term minting, the checked ID interval or original row ordinals.
Integration from main at 2cc11ce45 retains both the directory work and the
fallible API delivery. Only the aggregate constructor pin conflicted; dataset
and ingest authorities from main and builder authorities from this branch are
combined. The fresh source-derived closure is source:85d94cb0bf107df0; the
integrated full helper hygiene target passes in integrated-helpers-gate.txt.
integrated-clippy.txt exits 0 for the core library and three integration consumers
with warnings denied. integrated-consumer-native.txt exits 0 for the exact
ported RDF 1.2 round-trip/isomorphism case. This repeats existing coverage and
does not inflate the count. The other source-directory implementations/cases
are unchanged, so their passing qualification is reused. The actual difference
against current main remains eight files.
Initial and integration normal commit/push hooks pass. The integrated head is
b45cef24c7f1198cfff0a28f5a02b40cfc378df3. Exact-head hosted qualification passes:
56 successful checks and five skipped checks, with no pending or failed check.
Independent review reports no actionable code or architecture concern. The
CodeQL synthetic page-count fixture finding is dismissed as used in tests;
the exact production exclusion and fixture provenance are recorded in the
resolved review reply and dispositions. Fresh status reports no review debt
or unresolved thread. No whole-workspace replay or general-semantic WASM
execution was added for completion.
