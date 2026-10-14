<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Segmented effective-set view review

Root read the initial compiling `query/view.rs` source in the existing capacity
checkout, alongside original `mutable::classify_statement`, paged-stack
chronology and DatasetView forward/reverse/admission contracts.
`recovery-segmented-effective-view-check.log` is terminal 0. This is production
compilation only, not behavioral qualification.

The reviewed stream visits newer sources first and annotations before ordinary
rows; later removal carriers suppress earlier occurrences, and additions win
over their own layer's removals. Row dedup uses admitted HashTable state under
the shared operation. Declaration checks preserve graph scope and exclude the
same source for physical ordinary-row promotion, matching the original paged
classification home. Source guards retain original backing and translate only
datatype/triple children. No full lexical dictionary is reconstructed here.

Material forward/reverse coherence finding: `term_id_by_value` searches sealed,
head and declaration dictionaries but excludes removal dictionaries and already
minted representatives. A value held only by removal metadata can receive an
operation ID through `intern_source_term` or visibility probes and resolve
forward, while reverse lookup says None. DatasetView requires forward/reverse
coherence, including exact scoped blanks inside quoted/CDT terms. Owner has the
correction and a removal-only identity roundtrip control. Resolve via actual
touched identities or corresponding source dictionaries, without whole-history
reconstruction. No correction or runtime pass is claimed yet.

Subsequent source correction: reverse lookup now first uses `find_touched`,
hashing the requested value under the original scratch account, then comparing
exact source-qualified representative values while their copied owners remain
live. Root read that implementation and caller. This resolves the reviewed
source-level gap for already minted removal-only identities without retaining
their whole dictionary. Root read the final fixture receipt: the removal-only
nested/scoped roundtrip and chronological removal/reinsertion/empty-graph controls
pass. Current strict core and original segmented13 pass; stack controls9/10 in
`recovery-segmented-effective-view-fixture-runtime.log`, terminal101. The failing
annotation traversal is not cleared by those passing neighbors.

The focused cause receipt `recovery-segmented-classification-cause-runtime.log`
is terminal101 and exposes PinnedBlocks. Owner traced outer/nested row iterators
retaining both cache slots between yields. Root read the ensuing QuadIter source:
only the certified page address and row ordinal survive each yielded Copy row;
next() reacquires through the original block/cache home. Actual borrowed term
guards keep their original pin behavior. This addresses the identified source
overlap without a second decoder or raised limit. Runtime for that correction
has now passed: root read `recovery-segmented-row-cursor-typed-runtime.log`,
terminal0, strict core Clippy, original segmented13/13 and stack10/10 under the
unchanged cache2/pins4 fixture. Annotation chronology/classification and the
removal-only roundtrip pass. This clears the identified iterator-overlap failure;
the preceding failed receipts remain failures. Full engine parity is still open.

Root also read the new generic SegmentedEvidence and FallibleDatasetView route.
operation_status first checks all descriptors, then records original shallow
publication identity and the shared ledger's immutable prefix, digest and account
counters. The evidence retains the original base charge. Snapshot equality compares
ordered original layer owners and resident dataset/removal/declaration owners;
it does not claim portable content-hash equality. Cloning evidence retains Arcs
rather than copying historical terms or the ledger. These reviewed source paths
are coherent; immutable-checkpoint/old-head lifetime runtime coverage and the
actual full evaluator route remain required.

Root read the next source-only certificate delta: immutable resident head and
removal/declaration adoption computes the original expanded_footprint law once
over topologically ordered native terms. The retained maximum includes untouched
metadata, not just values queried so far. Query max_owned_term_bytes combines
certified sealed sessions and those retained resident maxima; snapshot creation
does not traverse any resident dictionary. Native validated nesting and the
original expanded-footprint law bound triple depth16; the view advertises that
same bound. Head construction is now fallible rather than disguising a failed
certificate. No new material source finding here; current compilation, evidence
lifetime tests and actual governed evaluator parity remain pending.

Later current checkpoint: root read the certificate/evidence unit and retained
`recovery-segmented-publication-evidence-runtime.log`, terminal0, strict core,
segmented13/13 and stack11/11. The new unit covers an untouched large removal
literal and declaration IRI before any ID is minted, allocation-free checkpoint
clone, immutable prior request prefix/digest after reads, and original sealed,
head, removal and declaration ownership through final evidence release. This
qualifies those focused certificate/evidence properties; actual full evaluator
and Datalog/purge/portable acceptance remain open.

## Actual evaluator COUNT refusal

The corrected evaluator target compiles, but
`recovery-segmented-chronological-engine-typed-runtime.log` is terminal101:
the provider-failure/evidence test passes and chronological COUNT refuses with
EvidenceExhausted at65533 requests, 7380864 I/O bytes and7631 evictions for the
roughly202-row fixture. The live8MiB ceiling is not the exhausted dimension. No
budget increase or complete parity PASS is authorized or claimed.

Root inspected the actual computation: declarations(row.s,...) probes each
source's reifier stream, but rows_from translates the pattern before checking
whether that physical stream has any rows. The all-zero reifier fixture therefore
copies/resolves each subject and probes reverse dictionaries on behalf of an
empty stream. Empty annotations and removal streams have the same needless
pattern-conversion path. Owner has the bounded original-home correction to
consult certified zero sealed table counts or original resident stream emptiness
before conversion, preserving sticky readiness and final all-descriptor checks.
This is a concrete unnecessary computation; it is not yet proven to explain the
entire refusal. Retain unchanged COUNT, inspect request deltas and diagnose any
remaining failure. The earlier scoped core/view tests do not qualify this engine
cost. Failure presentation should use typed cause and counters rather than dumping
entire provider byte arrays from Debug.

Owner source correction is now present: physical_rows consults authenticated
sealed stream counts or original native stream emptiness before rows_from's
pattern translation. Root read the zero-count helper and source filters; the
private Stream discriminants match original physical tables0/1/2. Shared ready
and final all-descriptor checking remain in place. No new material source defect
found in this correction; unchanged COUNT/runtime and request counters remain
pending, so the EvidenceExhausted finding is not yet qualified as resolved.

Final affected slice now passes: root read
`recovery-segmented-purge-engine-final-runtime.log`, terminal0, strict core,
stack13/13 and engine2/2 (48 raw/prepared executions). Original budgets/cache
limits unchanged. COUNT uses36841/36814 requests depending on reinsertion,
4667904/4665600 I/O bytes and4391/4388 evictions; peaks remain below8MiB.
The specific EvidenceExhausted finding is resolved by the production correction.
No complete scale/elapsed-time win is inferred.

Root read the encoded purge control and its passing receipt. Original literal
and datatype bytes are positively observed, a real successor contains retained
values only, rejected/drifted successor publication preserves old identity, the
untouched provider/receipt/bytes are unchanged, old snapshots still read their
removed value, and final evidence release drops original provider/layer owners.
Raw surviving encoded provider scans then contain neither deleted lexical form
nor datatype. This is actual consumer-owned persisted format acceptance, not
membership-only deletion. Full S1 capacity/host/rule/portable/generated/full-gate
and merge acceptance remains open.

FallibleDatasetView evidence acceptance, escaped evaluator workspace acceptance,
residency certificates and full eager-seal/SPARQL/Datalog parity remain incomplete. Final
readiness must preserve first failure and inspect every descriptor, including
skipped sources. The already passing query identity and ownership controls do
not prove this new view's complete behavior. Persisted purge remains unfinished.
