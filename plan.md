# Layered paged snapshots

Issue: https://github.com/Blackcat-Informatics/purrdf/issues/457
Branch: paudley/457-paged-tier-an-lsm-style-stack-of-sealed
Initial base: origin/main, c1d5bcd259089769e3679c6e1926b5ce24fbc1c2.

## Contract and governing decisions

Deliver a continuously mutable paged stack whose reads pin sealed base generations,
sealed delta generations, a frozen copy of the in-memory head, and ordered removal
sets. Preserve C0.8/C4 by resolving all cross-generation identities by RDF value.
Preserve G7-G10 through the existing PagedQueryView admission, certification, cache,
sticky failure and exact resource accounting. No durable storage or commit log is
requested; storage and atomic publication remain consumer-owned under G5.

Read AGENTS.md and docs/design/purrdf-backend-contract.md. The standing .baseline
and .goals are untracked files in /home/paudley/Active/purrdf and apply to this
worktree: Rust core, no semantic Cargo features, no silent failure, RDF 1.2 identity,
one home per job, portable deterministic code and maximal utility/performance.
No constitution or separate governing ADR was found; the backend design contract
is the applicable repository design law. Existing sibling work remains untouched.

Each sealed layer retains its own sealed dictionary/local page summaries. A private
physical composition remaps metadata by value into one snapshot dictionary, routes
flat page identities to (layer, original page), and never materializes older pages
while appending or opening a snapshot. It must not run the ordinary G3 seal over
legal repeated facts across generations. G3 remains enforced inside each generation.
Snapshot provider checks compare every pinned source generation and page count
directly; no hash can turn a changed vector into an unchanged snapshot. Evidence
reports ordered layer identities, stack depth, and page origins alongside the existing
global page/byte receipt. Head residency is separately identified in the receipt.

Removals belong to the layer that made them and suppress lower layers. An addition
in that layer or a newer layer wins, so remove/reinsert sequences preserve set
semantics. Physical summaries stay exact for the physical pages; they bound the
effective result conservatively after filtering, rather than claiming exact logical
counts. Probes must include physical annotation candidates when reifier removals
can promote them into the primary stream, and primary candidates when additions
can make annotations. Factor common RDF 1.2 row-classification logic where needed;
do not copy DeltaDatasetView's implementation or collapse the stack eagerly.

Mutable operations use value quads and typed fallible results for membership and
mutation that consult lower providers. DatasetMut's infallible bool contract must
not turn a provider error into absence. A snapshot owns its frozen head/removals,
so later mutation and sealing cannot change an active query. Stack depth is the
consumer's compaction policy, reported in evidence rather than an arbitrary limit.

Compaction drains a guarded effective view, canonically sorts surviving values and
typed RDF rows, partitions them under an explicit positive page-row bound, and
rebuilds pages using the same canonical sealing path an eager effective-set input
uses. Both primary and side tables, nested triple terms, scoped blanks, language
direction and declared empty graphs survive. Failure publishes no partial base.
Compare actual PackBuilder carrier bytes page by page and canonical global term
numbering; materialized Turtle parity alone cannot prove the page-byte identity.

## Completeness matrix

| Requirement | Task | Falsifiable acceptance |
| --- | --- | --- |
| One or more base generations and sealed delta stack | 1 | Public stack construction/append with independently sealed dictionaries; counted provider shows zero older-page reads during append/snapshot |
| Mutable head, batch sealing, independent new terms | 1 | Public insert/remove/contains, snapshot, seal-head; cross-layer join and exact term lookup including head-only terms |
| Ordered removals and reinsertion | 1 | Remove lower fact, seal deletion-only batch, reinsert in a newer batch/head; compare each snapshot to independent eager effective-set oracle |
| Whole-stack pin and G9 drift refusal | 1 | Retained snapshot unchanged after writer mutation; drift any source including untouched and zero-page source; constants-only evaluator refuses drift |
| G7/G8 aggregate page and byte evidence | 1 | Public query_fallible_view with inclusive/zero/over-limit budgets across layers, cache rereads, failure/cancel/deadline; assert exact page origins/totals and no complete partial answer |
| G10 sound pruning after removals/reclassification | 1 | Subject/graph-bound public queries and primary/annotation promotions/demotions; counted excluded pages stay unread, admitted corrupt pages fail typed |
| Read equivalence for queries | 2 | NativeSparqlEngine public fallible entry point against eager oracle: joins, filters, OPTIONAL, aggregates, ASK/CONSTRUCT, named graphs, RDF 1.2 reifiers/annotations |
| Canonical compaction and page-byte equivalence | 1,2 | Public compact/eager canonical seal; different ingestion/removal histories produce identical per-page PackBuilder bytes and canonical dictionary; fault refuses whole fold |
| Portable production utility | 2 | Public runnable Rust example consumes stack through evaluator and compaction; affected crate native checks and wasm build, no added dependencies/features |
| Review, publication, merge and cleanup | 3 | Full review/check retrieval, independent completion audit, ghprsq signed result/audit refs/stage archive, issue closure and scoped cleanup |

## Task 1: Implement layered snapshots, mutable batching and canonical fold

Implement the complete public stack mechanism in rdf-core. Reuse existing paging,
interner, mutable head, builder, pack and operational homes. Include focused core
regressions for layer ordering, snapshot isolation, RDF 1.2 stream partition,
zero materialization setup, exact aggregate budgets/evidence, drift/faults and
canonical page bytes. Document API costs and semantic laws in the backend contract
without issue/process references. Run cargo test -p purrdf-core for the new test
target and affected paging/delta tests; cargo clippy -p purrdf-core --all-targets
-- -D warnings. Obtain independent task review, repair findings, commit with signing
and mandatory hooks, push and verify remote OID, then update the issue.

## Task 2: Demonstrate evaluator equivalence and portability

Add real consumer coverage in sparql-eval and a runnable Rust example using the
public stack/query/fold path. Construct the eager oracle independently by applying
value-set operations and use the shipped evaluator/PackBuilder for observations.
Cover all matrix rows with representative valid RDF 1.2 inputs and adversarial
faults; add deterministic generated operation sequences rather than mirroring
private functions. Run affected core/evaluator tests, the example, affected clippy
and wasm builds using repository profiles/toolchain. Broaden checks only for a
named uncovered consumer/invariant. Keep command output and exact tested identity
in validation.md. Obtain independent task review; commit/push/update issue.

## Task 3: Publish the PR and complete review, integration and cleanup

Check complete acceptance evidence, hygiene and source cleanliness; create the
issue-linked PR through stagectl and publish this plan. Stage 2 independently
analyzes gaps and audits actual public entry-point demonstrations. Remediate
findings in coherent signed, hooked commits, push, and publish dispositions to
both issue and PR. Hosted CI and required feedback must be satisfied before merge.
Stage 3 assesses the actual base/head merge tree and semantic interactions,
adjudicates complete review debt, binds the qualifying completion audit, archives
selected evidence, and finalizes squash notes. Merge only with ghprsq and verify
signed result, parents/tree, notes, all audit refs including selected stage data,
PR/issue state and remote branch deletion. Perform stagectl scoped cleanup after
archive verification and fast-forward the protected main checkout.

## Considered alternatives

No new durable provider or storage policy: the issue explicitly reserves those to
the consumer. No fixed depth ceiling: exact depth evidence enables caller policy.
No whole-stack MutableDataset base: it would materialize lower generations for
every batch. No independent per-layer budgets: they would admit more than the
operation ceiling. No live-summary rewriting for tombstones: physical summaries
remain certified and logical filtering is explicitly conservative. No arbitrary
ingest-order page partition: it cannot satisfy byte-identical compaction.
