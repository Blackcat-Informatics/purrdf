# Native source accounting

## Production constraint

The actual Katamari caller supplies SegmentedStackQuery plus separate added and
removed datasets. Its query statistics are bounds, not a content identity;
stats_fingerprint cannot validate requested changes. Both operands of a source
check must not come from the requested delta. No cross-repository implementation
or timing requirement is authorized.

## Implemented source path

- The zero-dependency hash root owns Multiset256: little-endian sum modulo
  2^256 plus checked u128 population, preserving duplicate occurrences.
- Borrowed native and owned terms use the same canonical shallow encoder.
  Core canonical tags differ from Ord/Hash ranks and legacy text term tags;
  the original public vectors remain unchanged.
- Native predicate/scope metadata carries both literal-source projections and
  physical stream accounting. Profile 4 uses 240-byte predicate records. Its
  384-byte header additionally stores normalized statement-union/reifier state:
  ordinary and annotation value-quads count once in that union, while virtual
  reifier declarations remain a separate family. Physical stream sums cannot
  simply be added to obtain the chronological union.
- Construction and full certification compute/check metadata from original RDF
  rows. Rebuilding an image hash tree cannot authorize forged union state.
- Frozen, triple, paged and single-segment readers expose exact independent
  content metadata and global statement counts, with Arc forwarding. None
  means unavailable, distinct from an empty source. Original read failures
  take precedence; there is no update scan fallback.
- Native resident digest-prefix directories find changed records across moved
  local IDs. Chronological observations reuse the actual visibility engine
  and one original read operation. Their source versions retain the current
  immutable snapshot and grant, never a historical resident-head chain.
- Equal-content head sealing and full compaction reuse certified union roots.
  Applying an unchanged resident head twice is idempotent, so retaining that
  head across full compaction needs no physical candidate enumeration.
- Certified sealed images now persist the sorted physical digest directory with
  inclusive prefix sums. The resident and on-disk readers use one range-difference
  algorithm. Replaced layers inspect only changed physical records, then the
  existing chronological owner resolves actual visibility. Three native and eight
  observer cases pass; six physical candidates stay constant at 128 / 1,024 /
  4,096 subjects. Folding a small overlapping tail inspects nine candidates at
  each size. Ten certification checks pass, including forged directory bytes
  after rebuilding image hashes. These do not prove many-large-layer compaction
  or large-head folding. The later request/byte growth assertions pass in the
  original operation: replacement requests are 10,580 / 22,217 / 34,125 and I/O
  846,400 / 1,531,264 / 2,184,000 bytes at the three sizes. These are logarithmic
  directory/proof/dictionary paths, not a low-constant or total-resource claim.
- TextIndex retains the original source version. with_delta independently
  observes the next selected state, checks supplied additions by subject
  probes, reverses the requested literal delta against the old digest, checks
  nonliteral coverage counts, and advances the source version even when text
  is unchanged. Final read checks precede every successful publication.

## Retained token storage and input identity

The latest persistent-table integration supersedes the storage state in this
section: documents, lexical dictionary/postings, partitions, field lengths,
subject/occurrence orders, per-document Han evidence and all surface tables now
share the same tree.
Tokens retain shared spellings; the former slot arena is removed. Only changed
documents and their terms edit these owners and partition facts. Native query
projections use the original workspace's fallible term comparisons.

All 19 focused index cases pass after that complete integration and owned-query
lookup changes. Fixed lexical insertion at 128 / 4,096 / 16,384 retained documents
builds 43 / 73 / 85 table nodes and copies 196 leaf handles at each size, with
unchanged document/posting addresses and fresh-table parity. Actual native
query checks pass eleven determinism cases (600 chronological transitions), ten
point/stream membership and seventeen property-function cases. These scoped
results are not a full new suite, allocation-byte proof, WASM or delivery claim.
Earlier whole-text, strict/helper and workspace results below remain historical.

Token buffers retain shared spellings and use the persistent AVL positional
table with at most 32 handles per leaf. Requested occurrence iterators project
those spellings into canonical public dictionary ordinals. Changed documents
alone create new buffers. Retired spellings release with their last owner;
there is no slot arena or historical address map. Surface evidence uses
source-local occurrence chunks, with keyed gram/code postings. Its focused
generation/replacement/refusal and fresh parity checks pass; fixed insertion
copies 581 leaf values at each of three retained populations (see validation.md
for the original growth run's lost shell handle). Actual allocation/peak proof,
current full native callers and portable qualification remain pending.

The exact source multiset identity, configuration, analyzer and ranking laws,
and Unicode versions determine every derived table. New graph-aware v6 and
graph-free v3 index domains bind those inputs without walking the corpus tables.
Original spelling and multiplicity remain bound; equal statistics alone do not
give equal identities. Full fresh-table comparison remains an independent oracle.

Current native text coverage is 365 cases across 33 groups: 363 passed in the
complete no-fail-fast execution, with only the two expected index goldens failing;
both complete affected targets passed after migrating those two vectors. This
includes the 600-transition/four-scope chronological oracle. The later redundant
closure fix passes all eleven determinism cases on recheck. No source/analyzer/term-sequence
or score expectation changed.

The 4,096-edit token-table model checks ordering, balance and prior snapshots.
At 128 / 4,096 / 131,072 documents a fixed insert/remove builds 8 / 18 / 28 nodes
and copies 63 handles at each size. Clone builds no nodes or copied handles.
Another 128-update control checks two-slot reuse, retained-buffer pointers and
fresh-table equality. These count work, not allocation bytes. The identity
encoder receives exactly 293 bytes at both 1 and 512 documents and distinguishes
same-statistics reordered and normalized-equivalent original source texts.

Strict text/all-target Clippy, whole-workspace/all-target native caller compilation
and all 149 hash-domain checks pass. Current helper hygiene passes all 87 jobs,
21 variants and 120 distinct groups over 2,481 files without changing capacity
proofs. The earlier
core qualification below remains valid because these edits affect text only;
its earlier text, caller and hygiene statements are historical scope.

## Source-foundation qualification before retained-token changes

The complete core run passes 1,392 library cases. Later focused qualification
passes three directory and seven observer controls, including actual event/fresh
root parity, bounded changed-head work, sealing/overlap/compaction, consumer stop
and retry, removals/re-additions and predecessor-owner release. The inventory
control caught a missing 88-byte directory entry in retained_bytes; record and
checkpoint buffers now enter the original checked inventory.

The full native text run passes 362 cases across 33 result groups. The strengthened
eleven-case determinism run separately executes 600 transitions over all/default/
named/absent graph scopes, comparing actual document, partition, dictionary,
token, posting and frequency tables as well as fingerprints, coverage and answers
against fresh builds. Omitted selected-source changes, unavailable evidence,
direct/latched failures, forbidden full iterators and invalid corpus shape are
covered. This is native execution; actual WASM is pending.

Whole-workspace/all-target native caller compilation passes, including retrieval,
umbrella, CLI and language binding crates. This is compilation only.
Strict affected all-target Clippy and final core-only requalification pass. Final
helper hygiene passes all 87 enforced jobs, 21 reasoned variants and 120 distinct
groups over 2,480 files. Forty existing capacity proofs preserve their sink
hashes/classes/reasons; reviewed authorities, one moved location, and constructor
closure source:d20d0e9ff21dc16f are current. The 149-domain registry/check holds;
the final baseline edit changes no domains or hasher calls. Validation.md names
commands, logs, coverage reuse and failed runs. No commit, PR or delivery claim.

## Required remaining work

1. Preserve the completed persistent index/surface path and its independent
   fresh-table oracle. Recheck current public query/evidence callers after the
   API migration; do not rebuild the completed representation again.
2. Qualify bounded immutable snapshot/clone costs for every table.
   Preserve full fresh-table comparison independently of input-identity equality;
   never hide retained materialization in query publication. Validate allocation
   bytes and live/peak bounds beyond the current token work counters.
3. Replaced sealed layers now use persisted physical directories. General
   compaction of multiple large layers and resident heads still enumerates
   unpaired sources. Maintain an effective value directory to skip unchanged
   ranges across those transitions.
   Preserve current owners; no stale local addresses or retained head history.
4. Measure original producer metadata preparation, update, reads, visited entries,
   allocations and retained/peak bytes together. Do not move full work outside the
   measured update boundary or treat equal-root controls as general acceptance.
5. Complete native and actual scalar/SIMD WASM acceptance, production caller
   qualification, normal hooks, independent review and hosted gates; deliver
   through ghprsq and verify issue closure/archive before supported cleanup.

The complete issue remains open. Neither source-scan removal nor this local
qualification completes the retained-index and general source-transition contract.
