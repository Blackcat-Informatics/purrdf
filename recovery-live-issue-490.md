# Issue #490: Typed capacity errors instead of u32 overflow panics, and 64-bit room for Datalog ids and global dictionary text

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Several core tables are `u32`-indexed and **panic** when they fill: term tables, the term string arena, quad tables, delta term tables, merge and blank-node scope counters, and the Datalog id space. At Katamari's scale these are reachable data states, not programming errors. We ask for (1) typed capacity errors instead of panics on every such path, and (2) a 64-bit path where data can legitimately grow past `u32`: the Datalog id space, and the `GlobalDictionary` string arena (whose ids are already `u64` but whose string offsets are `u32`).

## Current behaviour (`main` at `6273b6173`)

Panicking sites (non-test):
- `crates/rdf-core/src/ir/term.rs:109-112` `TermId::from_index`: `expect("term table cannot exceed u32::MAX entries")`.
- `term.rs:252-256` `push_arena_str`: `expect("term arena exceeds u32::MAX bytes")` (`StrRange { offset: u32, len: u32 }`, `term.rs:224-226`).
- `ir/builder.rs:98` (interner), `builder.rs:259` (term table), `builder.rs:478-481` and `1078-1081` (`assert!` on quad capacity), `builder.rs:1062, 1069` (merge scope counter).
- `ir/dataset.rs:1097`: `expect("dataset quad count exceeds u32::MAX")` in lazy permutation build.
- `ir/mutable.rs:174`: `expect("delta term table exceeds u32::MAX")` on the delta insert path.
- `imports.rs:891, 1168` (blank-node scope counter), `provenance.rs:287, 344, 405` (unit / artifact / origin-set tables).
- `crates/datalog/src/id.rs:68-74` `Id::from_index`: panics past `u32::MAX - 1`, documented as "a programming error … never a reachable data state". Every Datalog `TermId`, `RowId` and `PartitionId` is this 32-bit type (`id.rs:50, 195, 203-207`).

Already typed, and the model to follow:
- `ir/import.rs:172-174` returns `DatasetImportError::Capacity` (term count only).
- `ir/global.rs:371-398` `try_intern_lookup` returns `Result`; `paged/stack.rs:82` `PagedStackError::Capacity`; segmented `SegmentedError::AddressExhausted`.

64-bit already exists: `GlobalTermId(NonZeroU64)` (`ir/global.rs:65`), `PackTermId = u64` (`ir/pack/dict.rs:102`), `EventTermId(u64)`. But `GlobalDictionary` stores strings through the same `u32` `StrRange`, and `global.rs:388` refuses past `u32::MAX` total bytes, so a global dictionary holds at most 4 GiB of term text.

## Why it matters

One Katamari mail vault is about 15 million messages, 300 million claims and 2-3 billion triples in naive RDF 1.2 form; its memory vaults are meant to grow for decades. In that range:
- a single dataset's quad count approaches `u32::MAX` (4.29 billion), and a rule evaluation over a vault's reifiers approaches the Datalog id space;
- the distinct literal text of a mail corpus (subjects, addresses, names, extracted text) passes 4 GiB long before the term count does.

A panic in a library linked into a store process takes down the store with no evidence of what was reached. Katamari's rule is that the only acceptable limits are physical ones, and a type width that cannot be made wider counts only when it really cannot: here it can.

## Proposed direction (a sketch)

1. **Typed errors everywhere first** (cheap, no layout change): every listed site returns a `Capacity` error naming the table and the count, propagated to the public entry points (builders, `MutableDataset`, imports, Datalog `compile`/`evaluate`). The Datalog id doc should stop calling overflow unreachable.
2. **`GlobalDictionary` with 64-bit string offsets** (or a chunked arena of `u32`-offset blocks addressed by a 64-bit position), so the global path's only limit is memory.
3. **A 64-bit Datalog id space**, either a type parameter on `Id<C>` (keeping `u32` where a caller wants the compact form) or a build that uses `GlobalTermId`-width ids, so rules can run over data that does not fit 32 bits.
4. The in-memory `RdfDataset` can stay `u32` per dataset (contract C0.8 makes ids dataset-local, and the paged/segmented layers are the 64-bit route), provided its limit is a typed error.

## Acceptance criteria

- A test per site drives the table past capacity (with a reduced-width test build or a mocked counter) and asserts a typed error, no panic.
- `GlobalDictionary` interns more than 4 GiB of term text in a test gated to 64-bit hosts with enough memory (or a unit test on the offset arithmetic with a synthetic arena).
- A Datalog evaluation with more than `u32::MAX` distinct ids is representable (unit test on id minting at the boundary; an end-to-end test where practical).
- No `expect`/`assert!` on table capacity remains in non-test code of `rdf-core` and `datalog`, enforced by a census check.

## Related

#90 (introduced the 64-bit global dictionary path), #457, #364, #477.


## Comments (1)

### paudley — 2026-10-10T23:15:18Z

# Complete capacity and sparse-stack delivery

Primary490 plus full495, including its host-identity comment. This plan does not
close492,491,484 or494; their complete requirements remain in REWORK_LIST and the
captured six-issue intake. Independent grouping judgment recommends this boundary
because Datalog host term IDs and internal ID widths share a real migration,
whereas graph-free canonical/text identity and indexed range/statistics contracts
have separate acceptance. Plan/design acceptance is PENDING. No source is written.

## Governing constraints and prior art

Read AGENTS, .baseline and .goals. Keep main and unrelated donors protected;
preserve every hook, native memory admission, determinism, first-fault barrier and
WASM path. Add no semantic features or runtime dependency. Owned GMEOW source is
available for reuse under explicit user authority. Process artifacts stay in the
selected Stage, separately archived at integration.

Use existing homes: lexical Memory/StorageError for physical admission, core
TermId/GlobalTermId and DatasetView for identity, Datalog Id/store/compiler for
rule execution, paged G8/G9/G10/G11 for read/chronology laws, and certified
segmented sessions for sparse storage. G11 currently admits whole-dictionary
publication cost; the new sparse route changes that mechanism without discarding
its duplicate/removal, stream classification, generation and sticky-failure laws.
Packed dictionaries remain whole-load; document segmented as the large sparse
route rather than adding a second sparse pack codec.

Stagectl prior-art.md records bounded live search and linked issue coverage.
prior-art-assessment.md is independent intake/grouping judgment, not design PASS.
Precise grouped recurrence is UNKNOWN. Existing checked/admitted paths are useful
but public convenience calls still panic, global text remains u32-offset, Datalog
IDs remain u32 and paged publications reintern all source dictionaries. New unused
APIs alone cannot satisfy either issue.

## Design

1. Logical capacity refusal carries a typed space/table, attempted count and
   representable limit. Keep it distinct from physical allocation/admission and
   invalid RDF failures. The shared native storage owner carries capacity through
   core adapters; Datalog preserves the same detail in compile/evaluate errors.
   Replace actual public convenience wrappers and migrate callers so no production
   mint/growth path converts capacity to expect/assert/panic. Document breaking
   Result signatures; do not leave a parallel unchecked convenience API.
2. Keep local RDF TermId's compact NonZeroU32 representation. Its checked mint
   and range constructors fail before mutation. Widen global string ranges to
   u64 while retaining one checked arena range/append implementation for local
   and global widths. Global IDs are logical u64; convert to host buffer indexes
   only after checked bounds. Datalog native entity handles become NonZeroU64,
   with checked minting for every brand and separate physical usize addressing.
3. Datalog facts and compiled constants use an associated term carrier/dictionary
   interface. The native carrier uses the widened native term handle; the host
   carrier returns caller IDs unchanged. Variable/proof/row/partition addresses
   remain distinct internal spaces. Consumers supply dictionary lookup and any
   derived-term creation they own. Reuse DatasetView's existing generic evaluator
   route; no remapping wrapper is inserted into joins. Raw ID order never becomes
   a claim of numeric/time value order: expose certified ordering separately.
4. Freeze immutable removal and graph-declaration carriers once when each layer
   seals, under their native storage owners. An owned SegmentedStack publication
   shallow-pins those carriers and chronological certified source descriptors;
   it copies only the current mutable head and its pending removals/declarations.
   Opening reads only headers. Publication neither traverses nor deep-copies
   historical tombstones, lexical payloads or declarations, so its work/retention
   is O(segment descriptors + current head), including current head removals.
   Dictionaries stay with their segments. Native query sessions build a bounded, lazy value-to-ID
   translation only for touched terms; host sessions use the host's stable IDs and
   dictionary directly. Both feed the same evaluator/Datalog operator path.
5. Original source cursors supply ordered physical rows. Effective-set merging
   applies newer removals/additions, duplicate suppression and native RDF1.2
   stream classification using the existing classification law. One operation
   owns all source budgets, original request addresses and the first typed fault.
   Metadata/head egress cannot bypass that latch. Every checkpoint verifies exact
   pinned descriptors, including skipped and empty sources; final readiness is
   required before publishing any query result.
6. Segment replacement is an immutable publication. Rewriting affected segments
   removes their dead text and dependent dictionary entries, leaving unrelated
   segment bytes unchanged. Retained old snapshots continue to read their own
   originals until released. New snapshots do not retain rewritten-away providers,
   dictionaries, caches or lexical metadata. Consumer persistence and compaction
   policy remain caller-owned; no hidden segment/depth/count ceiling is added.

## Completeness contract

| Requirement | Task and real acceptance |
| --- | --- |
| Every listed490 ingress | T1: terms/arena/quads/permutations/delta terms/merge/import scopes/provenance unit, artifact, origin set/Datalog spaces each have a boundary-counter test through their actual mint/growth home and public error propagation; no panic, healthy neighbor succeeds, failed mutation preserves prior state. |
| No capacity panic | T1/T3: Rust census checks non-test core and Datalog mint/growth implementations and fails on capacity unwrap/expect/assert; invariant checks are not used to disguise data exhaustion. |
| Global text beyond4GiB | T1: synthetic start/count at and beyond u32::MAX through the real checked wide-range constructor, local width refuses, global width succeeds, terminal overflow fails without mutation; real ordinary strings round-trip. |
| Datalog beyondu32 | T1: native term/row/partition and other branded minting crosses u32::MAX on64-bit hosts; checked host-address conversion on32-bit/WASM refuses only genuinely unrepresentable resident buffers; compile/store/evaluate consume the widened types. |
| Host identity addendum | T2: instrumented type-tagged host IDs, triple-term IDs, deleting/nonresident dictionary and certified numeric/time order enter actual SPARQL and Datalog scans; counters show no join remaps or translation. Equal semantics with the native carrier and typed source faults are tested. |
| Sparse1000-segment open | T2/T3: actual certified1000-segment providers count every block request; stack open reads no term/reverse/quad block and reports zero decoded dictionary bytes. |
| Term-independent publication | T2/T3: fixed descriptors/current head with tenfold term counts and separately tenfold historical tombstone counts/lexical bytes measure complete snapshot work/retention and per-segment cost; no historical payload traversal/copy, dictionary rebuild/decode/translation allocation occurs at publication. Current mutable head work is included, not subtracted from the bound. |
| Eager effective-set identity | T2: generated and authored actual queries over duplicates, removals, insert/remove/insert, multi-source reifiers, orphan annotations, empty named graphs and deep terms match an eager seal; original G11 typing/order laws remain. |
| Binding and faults | T2: retained snapshots, generation drift in skipped/empty sources, provider/corruption/admission failures and constants/head/metadata egress preserve the exact first typed cause and refuse publication; whole-operation budgets never reset per segment. |
| Local purge | T2/T3: rewrite only touched segments, drop originals, scan remaining persisted bytes for a unique deleted lexical sentinel and its dependency dictionary entries; unaffected bytes and retained-old-snapshot reads are independently checked. |
| Portability and real callers | T3: affected native core/Datalog/evaluator and other changed public callers; portable operational provider/host-ID controls and all release WASM builds; actual public import/builder/compile/evaluate/read entry points, not helper-only tests. |
| Complete merge | T4: both full issue contracts independently accepted, mandatory local gate, normal hooks, current hosted checks/review debt, actual merge candidate, ghprsq, verified issue closure and safe owned cleanup. Partial source gives zero completion credit. |

## Task 1: Complete typed capacity and native widths

Implement all actual mint/growth/public propagation sites and caller migrations
as one coherent source batch, including global text and every Datalog ID brand.
Use boundary injection into the same checks production calls; never allocate
billions of rows merely to exercise arithmetic. Remove obsolete unchecked paths.
Complete the structural census and width/rollback/ordinary neighbor controls.
Run affected strict compilation and the complete boundary/production family
after source settles. Independent task review covers each listed ingress, actual
callers, admitted buffers and default layout/throughput. Commit/push normally only
after acceptance; publish the task record, with both whole issues still open.

## Task 2: Complete host identity and sparse segmented stacks

Implement the associated Datalog term/dictionary path and actual host consumers;
freeze sealed removal/declaration carriers once and shallow-pin them while
composing certified segmented sources through owned sparse publications and
whole-operation guarded sessions. Include removals, native stream classification,
replacement/purge and retained snapshots. Write all semantic, source-fault,
zero-remap and sparse-open controls before one consolidated native/portable
qualification batch. Reuse native physical owners; no unpriced per-query table.
Independent review must follow real scans into both evaluator and rule engine.
Fix required findings, run affected checks and commit/push the coherent unit.

## Task 3: Complete scale, performance and final qualification

Run1000-segment open and fixed-segment/head tenfold-term and historical-removal
publication measurements,
effective-set differential/property cases, local byte purge and all required
portable/host/public capacity consumers. Capture actual native ownership and
default-throughput controls. Finalize documented API/error/backend contracts and
regenerate maintained projections through their generators. Run the required full
make-check once after source settles; retain passing evidence and use affected
corrections for later changes under Stage validation policy. Perform independent
full-contract judgment; every missing/failed criterion blocks publication.

## Task 4: Publish and fully merge both contracts

Use Stage tools for issue updates and one complete PR closing490 and495. Publish
the accepted plan, current evidence and reader uncertainty. Finish proportional
review remediation, current hosted gates, integration-tree/base assessment and
independent completion/review-debt judgment. Merge only with ghprsq and selected
Stage evidence. Verify both issues closed and durable archive, then safely clean
only the owned worktree/branch. Update REWORK_LIST with two completions only after
the authoritative merged/closed state exists.

## Declined alternatives and costs

Do not couple all six issues merely to share crates/gates; their separate scale
and identity contracts would delay ready enablers. Do not offer a foundations-only
PR or close495 for interfaces. Do not widen local RDF rows unnecessarily, use Cargo
features for width/semantics, rebuild a global dictionary per snapshot, remap host
IDs in joins, add a dependency, silently use raw ID order for ranges, or weaken
source-fault/native-memory barriers. Native lazy translations have query-local
work; the host direct path deliberately avoids that work. Widened Datalog handles
cost physical bytes, which must be admitted and measured, never hidden by an
unchanged budget formula. All retained contracts remain work to complete now.

## Independent review disposition

R1 addressed in design4, the term-independent publication acceptance row and
T2/T3: historical sealed removals/declarations are immutable shared carriers,
publication pins them without traversal, and measured total snapshot work includes
controls growing historical tombstone entries and lexical bytes tenfold at fixed
descriptors/current head. No residual historical-copy cost is subtracted or
reported as satisfying the required bound. Independent targeted recheck is due;
the earlier NEEDS-REVISION report remains historical until that recheck completes.


