# Issue #495: Layered stack over segmented snapshots: open and snapshot without a whole-dictionary rebuild, per-segment dictionaries that purge with their segment

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Compose **many** segmented snapshots (plus a resident head and removal sets) into one queryable view whose open and snapshot cost does not grow with the total dictionary, with each segment keeping its own dictionary so that rewriting a segment is the only thing needed to make a term disappear. In short: the layered-stack semantics of #457 / G11, over the sparse-open `segmented` format rather than over in-memory pages with one rebuilt dictionary.

## Current behaviour (`main` at `6273b6173`)

Much of what Katamari needs already exists, in separate pieces:

- **Segmented format** (`crates/rdf-core/src/ir/segmented/`): certified, provider-backed, persisted blocks; "opening and reading a certified snapshot keep no whole-dictionary, block-directory or translation array" (`segmented/mod.rs:4-9`); "No term, reverse-index, graph, or quad block is decoded at open" (`segmented/session.rs:366-367`). `SegmentedSession` implements `DatasetView` and `FallibleDatasetView` with `GlobalTermId` (`session.rs:1169-1170, 1358`). It can be built from a pack (`segmented/build.rs:421`, `from_pack_v1`).
- **Layered stack** (`ir/paged/stack.rs`, contract G11): bases, chronological sealed deltas, a mutable head, per-layer removals, one snapshot. But `paged/` has no reference to `segmented`, and a stack snapshot "rebuilds one private physical dictionary and remaps translations by RDF value", with work and retention `O(all retained dictionary values + translation entries + …)`; "Repeated small publications can therefore accumulate quadratic metadata work" (`docs/design/purrdf-backend-contract.md`, G11).
- **Packs** decode their whole dictionary at open: `PackView` "Owns the decoded `PackDict` (an arena the dictionary's PFC sections are decompressed into once, at open time)" (`ir/pack/container.rs:637-639`, `PackDict::open` at `container.rs:727`, `pack/dict.rs:940-942`).
- **Term deletion** is by whole rebuild: `GlobalDictionary` is append-only (`ir/global.rs:306-323`); `PagedDataset::compact` (`paged/mod.rs:761-764`) reclaims dead ids by renumbering everything.
- Durable tiers are the consumer's (contract G5), which Katamari agrees with: it owns its files and publication. This request is for the read-side mechanism over many segments, not for persistence.

## Why it matters

Katamari's RDF data plane (ADR-0038 38.15) is immutable sorted segments plus a small delta tier, bulk-built, with purge rewriting exactly the segments that held a deleted object. One mail vault is about 300 million claims and 2-3 billion naive triples; segments number in the thousands, and the vault is opened often and queried constantly. Three properties follow, each of which the current pieces miss in one place:

1. **Open and snapshot cost independent of total size.** A snapshot that rebuilds one dictionary over every retained value is `O(billions)` per publication; with frequent small commits that is the quadratic metadata work G11 warns about. Segmented sessions already avoid this per segment.
2. **Delete means delete.** Katamari must guarantee a purged object leaves no trace, including its strings. A dictionary shared across segments keeps a purged string alive until a global compaction renumbers everything (Oxigraph's string dictionary is never collected at all; Katamari's review lists it as a failure to avoid). Per-segment dictionaries that die with their segment make purge local: rewrite the affected segments, drop the old ones.
3. **Statistics and summaries live with the segment** and are rebuilt with it (mergeable across segments; see #484).

## Proposed direction (a sketch)

- A `SegmentedStack` (or `PagedStack` over a `SegmentedSession`-backed provider) holding: an ordered list of segmented snapshots, optional sealed deltas, a resident head, and per-layer removals, with G11's semantics (newer layers win, removals suppress older occurrences).
- **No global dictionary rebuild per snapshot.** Joins across segments compare by value, or through a per-query translation built lazily only for the terms a query touches; `GlobalTermId` stays operation-local.
- Snapshot cost `O(number of segments + head)`; open cost `O(number of segments)` (headers only).
- Compaction / purge rewrite: replace a set of segments with their rewritten successors (minus removed values) as one publication; nothing outside the replaced segments changes.
- Packs: either add a sparse dictionary open (front-coded blocks decoded on demand), or document `segmented` as the route for large read-only data and keep packs for whole-load use.

## Acceptance criteria

- Opening a stack of 1,000 segmented snapshots decodes no term block (G8 evidence: zero dictionary bytes read at open).
- `snapshot()` work is independent of the number of distinct terms (benchmark: constant per segment as total terms grow 10x).
- Read equivalence: any query over the stack equals the same query over an eager seal of the effective set (the #457 identity).
- After rewriting the segments that contained a term and dropping the originals, a byte scan of the remaining segments finds no trace of the term's lexical form.

## Related

#457 (layered stack over paged), #90, #331, #484, #490.


## Comments (2)

### paudley — 2026-10-09T06:49:59Z

## A path for the host to supply term identity

Katamari (ADR-0045 45.23, Proposed; `docs/design/QUERY-AT-SCALE.md` decision 3) will keep its own term identity:
- 64-bit, vault-local, type-tagged ids;
- an **order-preserving** encoding for times, numbers and confidences inside the id, so time and range filters become range scans;
- triple terms with their own ids;
- a dictionary that need not be resident and that supports deletion (this issue's per-segment dictionaries).

The proposal above compares across segments by value, or through "a per-query translation built lazily only for the terms a query touches". For Katamari's planner seam we would like a route that avoids translation on the join path altogether: either purrdf adopts this identity, or it provides a path for a host to **supply** term identity and dictionary access (an id type and a dictionary trait the evaluator and the Datalog store are generic over), so that rows produced by Katamari's segment scans enter purrdf's operators without remapping.

That is the "they adopt or make a path available" choice. Either answer works for us, provided no join translates identifiers at the seam. Related: #490's 64-bit Datalog id space, which this would subsume where a host supplies the ids.


### paudley — 2026-10-10T23:15:20Z

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


