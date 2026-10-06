# Stage 2 specialist: structure, security and performance

VERDICT: BLOCKED

Open findings: existing **HF1 MEDIUM**, independently confirmed; new **PF1 HIGH**,
unnecessary repeated full-page metadata scans in the stack's logical hot path.
No additional concrete security or structural source defect is asserted.

## Exact identity, scope and independence

Issue 457, PR 466; branch `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
Worktree: `/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
Reviewed HEAD: `b2edf7450cf654d20ae97bd856d67102d0916b7b`.
Reviewed tree: `13496228db0e5ec79074acad9020cac5aab7556e`.
Verified base supplied/current: `b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac`.
Plan SHA-256: `ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29`.
Identities and source cleanliness were checked again after inspection.

Read G11 and governing C/G laws, captured source patch inventory and changed
production bodies, relevant current native homes and test assertions, validation,
prior independent plan/task reviews, historical-fit and Stage 1 handoff. Both
T1/T2 source manifests match current files. Applied Stage 2 specialist scope;
this reviewer is neither gap analyzer nor completion auditor. Source/evidence
judgment is independent; existing runtime logs are attributable prior execution.

No Cargo/build/runtime command, source mutation, forge action, merge or cleanup
was performed. Only this report was written. No timing/throughput conclusion or
measurement is fabricated; PF1 follows directly from the actual nested loops.

## Named risk scopes

| Scope | Inspected boundaries | Judgment |
|---|---|---|
| Untrusted page identity and metadata | StackProvider source routing, complete descriptor checks, PageMaterialization generation translation, PagedQueryView term/value/layout/charge/capability/summary certification | Existing guards are reused coherently; no new bypass found in these paths. |
| Operational failures, caches and limits | One physical PagedQueryView, page accessor, before/after admission and engine checkpoints, direct metadata gating, typed source errors and refused-request receipt | Whole-operation ceilings/cache/sticky latch are preserved; local public-consumer fault evidence is applicable at this identity. |
| Physical/logical trust separation | Private overlapping physical composition assembled from checked sealed source parts; all public logical egress applies chronology/classification | G3 is not weakened for public ordinary generations; private physical overlap is intentional and not exposed as a logical disjoint dataset. |
| Resident address/index integrity | Checked dictionary reservations, page ordinals/ranges/origins, layer vectors, iterative term/source walks, canonical page construction | Numeric indexing is internally derived from owned vectors; no new unsafe or external dependency is present. See HF1 for semantic incompleteness in checked interning. |
| Structure and shared homes | classify_statement native/mutable consumers, checked interning, admission/certification, importer, PackBuilder, root/IR/RDF/umbrella exports | Main composition fits one-home constraints; checked literal registration is the one confirmed exception. |
| Snapshot/membership metadata cost | Dictionary/remap rebuilding, sealed_snapshot reuse, head-only normalization/seal, operation-local cache allocation | Linear retained-metadata cost and cumulative quadratic publication work are disclosed; no throughput improvement is claimed. |
| Deep stack and query metadata work | visible_at, source-vector checks, reifier association probes, range cursors and graph postings | Iterative depth avoids recursive stack growth, but PF1 introduces avoidable row-times-page work even for plain RDF. |
| Canonical output/cold fold | Guarded typed drain, record ordering, page bound, PackBuilder charges, canonical dictionary remap, empty graphs and RDF 1.2 records | Public fold and real per-page carrier evidence exist; HF1 must be repaired before shared dictionary semantics qualify. |

## Existing HF1 — MEDIUM: checked reinterning omits embedded CDT blank registration

This is the SAME finding as historical-fit.md HF1, not a new duplicate defect.
Independent source inspection confirms it at `ir/global.rs:519-545`.

`intern_literal` at lines 569-590 registers concrete `(label,scope)` blank
identities extracted through the single `cdt_embedded_blanks` home before
interning the literal. The prior reinterning path called that home. The new
`try_reintern_validated` literal arm calls `try_intern_lookup(Literal)` directly,
so a cold dictionary retains the lexical bytes but does not register the
embedded identities. This affects direct and nested CDT List/Map terms. The
existing global regression calls reinterning only AFTER ordinary intern populated
the same dictionary, so its successful hit is not a cold-path witness.

Security implication: this is a semantic identity/registration discrepancy, not
an established memory-safety exploit. "Already validated" allows avoiding
redundant IRI parsing; it does not authorize dropping RDF identity registration.
Stack dictionary construction and removal-value reinterning consume this shared
home, as do native page translation and existing PagedDataset compaction. Source
dictionary loops may copy embedded identities separately and mask the defect;
this report does not assert a demonstrated wrong public query from that masking
case. It still violates the pre-existing native operation invariant and must
be repaired inside this task's touched shared home.

Required disposition matches HF1: one shared composite-literal registration
operation usable by both ordinary and checked literal interning, retaining typed
address/allocation refusal. Use the existing CDT parser/extractor, preserve
scoped identities and exact opaque lexical bytes, and add cold separate-dictionary
ordinary-versus-checked tests for direct/nested/recursive CDT values, quoted
non-blanks and idempotence. Reuse affected global/paging evidence only after
new checks cover the changed production source. Do not resolve it by another
parser, a documentation caveat or an assertion over a prepopulated dictionary.

## PF1 — HIGH: stream range probes ignore existing graph postings

Locations:

- `ir/paged/query.rs:388-400`: `stream_pattern_range` iterates
  `self.dataset.pages[range]` then tests `stream_admitted` on every slot.
- `ir/paged/query.rs:454-465`: `stream_estimate` similarly visits every page.
- `ir/paged/stack.rs:735-761`: each ordinary-row `external_declaration` calls a
  full physical reifier stream probe, with a bound named/default graph.
- `ir/paged/stack.rs:786-803`: `logical_rows` invokes that association probe for
  EVERY newest visible ordinary row; physical annotations perform whole/prefix
  declaration probes as well.
- Existing solution home: `ir/paged/admission.rs:245-259`,
  `candidate_pages_for_stream`, backed by exact per-stream GraphPageIndex postings.

Concrete source consequence: take P sealed ordinary-only pages with R visible
ordinary rows and no reifiers. Every one of those R rows invokes
external_declaration, whose graph-scoped Reifier probe performs P
stream_admitted calls before correctly finding no reifier. This creates
Theta(R*P) unnecessary metadata work on a plain-RDF full read, even for one
source, where the graph index's reifier postings are empty. Page/materialization
receipts correctly remain unchanged, so the current counted-provider tests do
not expose this compute amplification. If R grows with P, the overhead is
quadratic. A graph-selective logical read/estimate also scans irrelevant slots
before the exact summary refuses them. This is stronger evidence than a guessed
timing regression and directly conflicts with the established G10 graph-index
narrowing and repository maximal-performance goal.

### Bounded repair requirement

Reuse the EXISTING stream candidate/graph-posting home for ordinary, reifier and
annotation physical probes AND estimates. Extend that home only as needed to
restrict candidates to a chronological page range. For Named/Default, restrict
the sorted posting slice to `[range.start,range.end)` with two binary boundaries
(e.g. partition_point), then iterate only that slice. For Any, directly iterate
the supplied dense physical range. Preserve ascending physical/in-page order and
the surrounding newest-first layer order; retain stream_admitted as the exact
axis gate and the original page accessor as the sole cache/budget/certifier.

Do not filter an entire global posting list afresh for each layer: that simply
changes the unnecessary multiplier from all slots to depth*all-postings.
Chronological range restriction belongs before iteration. Do not add a second
graph index, another admission law or a second classifier/cache. No memoization
framework, new public budget, depth ceiling or throughput benchmark target is
required to fix this specific defect.

### Focused operation-count evidence

Add bounded deterministic evidence at the candidate/admission boundary, not a
brittle elapsed-time assertion:

1. Graph/default Reifier association over a large ordinary-only physical corpus
   has ZERO page-slot admission visits for that empty stream posting, even while
   logical ordinary rows are drained.
2. For each stream and a bound graph, a requested chronological range visits
   exactly its in-range postings, never unrelated-graph pages or out-of-range
   layer postings. Increase unrelated pages/layers while holding those postings
   fixed; visits remain fixed apart from logarithmic boundary searches.
3. Any scans visit exactly their range once. Empty ranges and deletion-only
   sources visit none. Compare values/typed streams and cardinality bounds to
   the existing independent effective oracle.
4. Preserve real guarded evaluator answers, deterministic qualified first-request
   order, page/byte totals, cache rereads, zero/equality limits and operational
   failures. Narrowed metadata visits must not silently change which matching
   physical streams are needed after reclassification.

A helper-level count plus a concrete plain-RDF logical-path witness is sufficient
for this repair. Temporary diagnostic probes must be removed; any retained test
accounting must measure the real candidate home rather than a replica. No timing
speedup claim is justified until a matching benchmark actually runs, and none
is needed here.

## Other security and structure dispositions

StackProvider compares each source's original generation and page count directly,
including zero-page sources, at the shared descriptor checkpoint. It validates
returned source generation before substituting the private routing generation.
Original provider faults remain typed and gain routed/source address context.
One operation-local accessor performs limit checks BEFORE provider materialization,
then validates sealed values/layout/charge/capabilities and summary digest in
all profiles before consuming the page. There is no stack-only uncertified read.
Existing G10 warm-restart certification responsibility for pages skipped by all
queries remains unchanged and explicit; a skipped page cannot be newly certified
without an actual read. Do not claim the stack eliminates that trust boundary.

The private physical PagedDataset knowingly contains cross-generation overlapping
facts, but is owned as a private field of snapshot/query view. Public quads,
side-table methods, point reads, graph enumeration and compaction pass through
logical chronology/classification and guarded status. No unchecked physical-view
public export was found. Original per-generation G3 refusal stays intact.

The source scan finds no added unsafe block, runtime dependency, semantic feature,
random default HashMap/HashSet, forbidden namespace, stub or deferral marker.
The helper census and affected clippy/native tests provide attributable supporting
evidence, not proof of HF1 or PF1 correctness. Layer/page casts index vectors that
were built with those ordinals; they do not reinterpret a caller's TermId as a
cross-dataset numerical range. Term/source traversal is iterative. No novel
machine-stack-depth risk or access-control boundary is introduced by those loops.

The shared classifier is a small concrete decision reused by DeltaDatasetView
and the stack, while each source retains its own indexed row retrieval. The
canonical partitioner orchestrates typed records through existing native builder
and PackBuilder homes; it is not a competing encoder. Root/IR/RDF/umbrella
exports and the runnable evaluator consumer are connected. No library addition
is left solely reachable through a private test bypass.

The 1174-line stack module combines writer/snapshot/provider/logical-read/fold
roles, but those roles have concrete type/API boundaries. File length alone is
not a defect or reason to split into an arbitrary abstraction framework. Keep
repairs inside the existing natural interning/admission homes.

## Evidence limits and final disposition

This review independently inspected source and attributable logs; it performed
no runtime experiment. The current local success receipts do not clear the cold
CDT discrepancy or the candidate-iteration cost, because they do not test those
dimensions. Hosted checks and final independent completion/merge/archive state
remain parent-owned and unverified here.

Required remediation: **HF1** and **PF1**. Subsequent specialist recheck should
cover their actual changed interning/admission/source paths, bounded operation
counts, matching query receipts and qualifying affected regressions. Unchanged
security/structure judgments can be reused with exact identity applicability.

VERDICT: BLOCKED
