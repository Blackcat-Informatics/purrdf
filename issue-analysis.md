# Complete identity and storage group intake

Scope: full issues 490, 492, 495, 491, 484 and 494, including 495's host identity
addendum. Six live issue captures contain all bodies and comments: issue.md and
issue-{492,495,491,484,494}.md. All remain OPEN. Only 495 has a comment (one),
and it requires a host identity/dictionary path without join-time translation.
No implementation, plan review, acceptance or completion is claimed here.

## Requirements and proving surfaces

| Contract | Required implementation and evidence |
| --- | --- |
| 490 capacity | Typed table/count errors for local terms, string arena, quads, lazy permutations, delta terms, merge/import blank scopes, provenance units/artifacts/origin sets and every Datalog ID space. Public builders, mutable datasets, imports, compile and evaluate propagate them; boundary injections exercise each actual mint/growth path without panic. A structural census refuses shipping capacity asserts/expects in core and Datalog. |
| 490 global text | Global dictionary offsets and lengths represent more than four GiB. Synthetic arithmetic crosses the boundary through the real arena-range constructor, with checked host-address conversion and no giant allocation requirement. |
| 490 Datalog | Term/row/partition IDs represent the u32 boundary at 64-bit width; real compile/store/evaluate paths consume that representation, with boundary minting, healthy neighbors and typed terminal exhaustion. Local RDF TermId remains compact and bounded with typed refusal. |
| 492 triple carrier | Twelve-byte SPO rows, three physical permutations and graph-free reifier/annotation rows. Triple-only DatasetView admission refuses named graphs with typed diagnostics. Default-graph SPARQL corpus is unchanged; rules use actual arity-three atoms and match all-default quad RL models. Canonicalization/digest and graph-free text corpus identities have documented, executed identity laws. |
| 495 segmented stack | Header-only open of 1,000 certified segments; no decoded term bytes. Owned immutable publications compose chronological segments, sealed deltas, resident head and removals without rebuilding all dictionaries. Snapshot cost depends on segments/head, not total distinct terms; measured tenfold term growth demonstrates this. |
| 495 semantics/purge | Real queries equal an eager seal of the effective set, including duplicates, RDF1.2 side tables, removals and empty graphs. Replacing just affected segments and releasing original snapshots leaves no lexical trace in remaining persisted segment bytes. Unchanged retained snapshots preserve their original read identity. Packs remain documented whole-load carriers; segmented is the sparse-open route. |
| 495 host seam | Caller-supplied 64-bit, vault-local, type-tagged and ordered IDs plus deletable nonresident dictionary access enter evaluator and Datalog scans unchanged. Triple terms have IDs. Instrumented host consumers show zero remappings/translations in joins and correctly preserve identity provenance and typed read failures. |
| 491 indexes | Range-addressed triple-to-reifier, triple-component-to-reifier and annotation predicate/object-to-reifier directions on frozen data; DatasetView production callers use them when the reifier is unbound. Ten-million reifiers with six annotations demonstrate matching-proportional examined candidates. RdfDataset, packs, paged and segmented either supply actual indexed paths or explicitly expose honest scan paths in EXPLAIN. Existing RDF1.2 results remain unchanged. |
| 484 statistics | Snapshot-bound deterministic per-predicate counts/distinct subjects/objects, per-class counts, reifier predicate and annotation counts, object kind/datatype and low-cardinality predicate/object frequencies. Exact/bound/sketch status is explicit. Counts and sorted distinct identities merge across segments; property tests compare with one combined segment. |
| 484 consumers | Production SPARQL and Datalog planners read the shared seam; consumer-supplied statistics change real plans and EXPLAIN identifies exactness. A million-claim reifier/annotation estimate is within twofold actual cardinality. Paged plans respond to changed reifier counts; rare-class queries plan class-first on resident and paged data. |
| 494 coordinates | Public host NOW and entropy seed flow through all execution contexts; explicit inputs enter evaluation identity/provenance. Equal coordinates produce equal NOW/RAND/UUID results, changed NOW changes results, absent coordinates preserve live behavior. Pre-epoch time does not silently map to 1970. Rule consumers share the same coordinate owner. |
| 494 history | snapshot_at(k) pins bases plus exactly the first k sealed generations and their removals, excluding later layers and resident head. G1..G5 prefix-three queries equal an eager seal of G1..G3; invalid positions produce typed refusal and original snapshot binding remains intact. |
| 494 ranges | Ordered annotation filters reach actual range scans; numeric/time order follows value semantics, never raw unrelated ID order. Paged/segmented summaries reject nonoverlapping pages before admission, including outlier/multi-range cases. Ten-million-claim valid-time queries read only overlapping pages and equal literal post-filter answers. |

## Source observations

Current base is e88a8e0e2. Resident interning has admitted fallible paths, but its
public convenience paths still unwrap storage failure. GlobalDictionary still
uses StrRange u32 ranges and checks total bytes as u32; checked global minting
already exists. Datalog Id remains NonZeroU32 with a panicking mint constructor.
PagedStack snapshots still reintern every retained source dictionary. Segmented
sessions already provide sparse reads and fallible storage diagnostics. Reverse
reifier lookup remains a full scan. Clock inputs remain host-only; public query
options lack explicit coordinates. These observations require migration of the
production callers, not additional unused APIs.

Existing 515/516/511/518/512 source and geo/regex qualifications remain owned by
their existing writers. This group does not import their mutable worktrees or
rerun their gates. Its integration assessment must retain their final landed
owners if those groups merge first.

## Prior art and uncertainty

Stagectl's 490 brief covers ten related issues within two hops, fourteen related
forge results and the last two hundred commits' defect trailers. This is bounded
coverage, not proof of absence. Relevant landed homes are DatasetView/GlobalTermId,
the admitted lexical Memory/StorageError owner, paged G11 and sparse segmented
sessions, the evaluator's generic ViewTermId path and the Datalog store. Owned
GMEOW physical IDs can be inspected/reused; no sparse-stack implementation was
found in the initial targeted source search. Recurrence for this precise grouped
failure class is UNKNOWN; broad trailer counts are not an issue recurrence count.

Required independent analysis, design review and complete accepted plan are still
due before implementation. The full six-contract scope is retained, with no
foundation-only publication or completion credit.
