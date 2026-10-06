# Stage 2 gap analysis

VERDICT: BLOCKED — HF1 and PF1 require coherent source remediation. The requested
mechanism and real guarded consumers are implemented, but passing existing tests
do not clear these shared-ingress and read-path findings. No issue requirement
has been waived. Pending hosted checks and later workflow gates are unverified.

## Bound review identity and evidence

Issue 457; PR 466; branch `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
Worktree `/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
Independently read HEAD `b2edf7450cf654d20ae97bd856d67102d0916b7b` and source tree
`13496228db0e5ec79074acad9020cac5aab7556e`. Captured verified main base is
`b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac`; the recorded merge candidate equals
that source tree. Source status remains clean, with only existing `.stage/`
untracked. Plan identity is
`ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29`.

Read the complete issue and captured issue discussion, actual captured PR body,
review/inline/thread/check surfaces, plan, branch-under-review, historical-fit,
S2-issue-analysis, prior-art assessment, current validation index, both final
task reviews and implementation handoffs, initial separate completion audit,
changed paging/global/mutable implementation, shipped public example and
facade wiring, relevant core/evaluator assertions and attributable raw logs.
Applicable authority is worktree AGENTS.md, root `.baseline`/`.goals`, and the
C/G backend contract. No separate governing ADR is cited by this diff.
The emergency ledger was independently read and has no entries.

This reviewer executed read-only identity/source/scan commands and wrote only
this process report. No source edits, Cargo/runtime execution, forge retrieval
or publication, history changes, configuration or lifecycle action occurred.
Existing execution is adjudicated by attribution; it was not repeated here.

## Open findings and coherent remediation

### HF1 — MEDIUM: checked cold literal reinterning loses embedded blank registration

Source: `crates/rdf-core/src/ir/global.rs:519-545`, specifically the literal arm.
The retained native `intern_literal` home at lines 566-583 extracts concrete
embedded CDT blanks with `cdt_embedded_blanks`, preserves `(label, scope)`, and
registers them before inserting the literal. Ordinary `intern` calls that home;
the previous validated reinterning did too. The new `try_reintern_validated`
calls `try_intern_lookup(Literal)` directly, so its cold literal path omits this
operation. Its infallible validated wrapper now inherits the omission.

For a fresh dictionary and validated `typed_literal("[_:bnode1]", CDT_LIST)`,
ordinary ingress registers the embedded blank while checked validated ingress
does not. Nested List/Map values and literal children of recursive triple terms
have the same omitted operation. The current global regression calls validated
reinterning only after ordinary `intern` populated the same dictionary; a warm
hit cannot establish cold parity. Full source-dictionary traversal can separately
visit embedded blanks and mask this defect in current page composition. This
report therefore does not invent an observed wrong stack query for that input;
the semantic regression in the changed shared home is source-established.

Blast radius: checked page translation, existing paged dictionary compaction,
new snapshot dictionary/removal composition, and validated reinterning callers.
Validated input permits skipping repeated IRI parsing; it does not permit
dropping native RDF identity registration. Historical fit HF1 and completion
audit F1 are the same defect and should become one remediation item.

Fix: preserve checked allocation/address refusal while placing literal blank
extraction/registration in one shared native checked literal-ingress home used
by ordinary and validated reinterning. Reuse the existing CDT extraction home;
do not add another parser or lossy fallback. Keep literal bytes and ordering
unchanged. Add cold dictionary parity for default/scoped embedded blanks, nested
CDT List/Map/triple values, opaque quoted text, exact lexical bytes, store-once
idempotence, and nested triple-term containment.

Acceptance: a new regression must fail against this reviewed source and pass
after the fix. Run `CARGO_BUILD_JOBS=2 cargo test --locked -p purrdf-core --lib
ir::global`, affected paged/stack targets and the real public consumer. Independently
review checked capacity propagation and exact native registration behavior.
Refresh affected lint/one-home/portability evidence as needed for the final delta.
Commit boundary: one signed, hooked shared-ingress fix plus its focused tests;
push/OID verification and disposition published to both issue and PR before
calling HF1 fixed.

### PF1 — HIGH: ordinary row classification rescans every physical page per row

Source: `crates/rdf-core/src/ir/paged/stack.rs:735-761` and `:786-796`;
`crates/rdf-core/src/ir/paged/query.rs:388-400` and `:454-465`. The latter
estimate method likewise bypasses the stream postings and must share the repair.
Every surviving physical ordinary row invokes `external_declaration`, which
calls `stream_pattern(Reifier)` for its subject and graph. That delegates to
`stream_pattern_range`, whose iterator visits `self.dataset.pages[range]` in
full and only then applies summary admission. For plain RDF with R ordinary
rows, P physical pages and no reifiers, every such probe is negative and visits
all P page slots: O(R*P) metadata work in a full logical read. With one/few rows
per page this becomes quadratic in dataset size. The I/O cache does not cache
these negative summary probes. Exact page/byte receipts and provider counters
can remain linear and conceal the extra work.

This is distinct from the disclosed linear snapshot dictionary rebuild and
possible cumulative quadratic publication cost. It occurs during each query
and explicit fold over an already-created snapshot, on ordinary input without
the side-table feature being present. No timings were executed by this reviewer;
the bound follows directly from the iterator/call nesting. Source-established
quadratic work on this carrier's normal read path violates the applicable
maximal-performance/one-home guidance and the prior-art direction to reuse
metadata-derived per-stream graph postings. Parent and specialist independently
identified the same risk; the completed specialist report independently binds
PF1 in `reviews/S2-specialist-structure-security-performance.md`.

Existing `admission::candidate_pages_for_stream` at admission.rs:245-260 and
native `reifier_quads_in_graph` at query.rs:1040-1072 already narrow default/named
graph candidates to sealed stream postings. The new raw stream seam bypasses
that narrowing. Its per-page subject indexes remain useful after admission,
but do not eliminate the full page-slot scan preceding admission.

Fix: route raw physical stream candidates and estimates through the existing metadata-derived
stream/graph candidate home, slicing sorted named/default graph postings to the
requested layer/prefix range using binary boundaries before iteration. For Any,
iterate the requested physical range directly, once. Preserve ascending physical
order and the one accessor/cache/budget/latch. Filtering a whole global posting
list separately for each layer would introduce a different depth-times-pages
scan and does not satisfy this repair.
In particular, a graph with no reifier postings must not search every page for
each ordinary row. Use a sound additional presence fast path or operation-local
memoization only where warranted by remaining repeated probes; do not eagerly
materialize an all-stack reifier table or fork admission/accounting. Preserve
chronological prefix, source-external declaration and graph-scoped semantics.

Acceptance: add a meaningful deterministic candidate/posting regression proving
an empty or sparse side stream does not examine all irrelevant page slots on
repeated classification probes, not just zero provider reads. Include a concrete
plain-RDF logical-path witness plus real candidate/admission boundary counts:
empty Reifier postings visit zero slots; fixed in-range default/named graph
postings retain fixed visits as unrelated pages/layers grow; Any visits precisely
the requested range once; empty ranges visit none. Retain dataset shape,
page/row counts, source/compiler identity and observations. These bounded Rust
operation measurements address the algorithmic risk; no new timing benchmark
or brittle elapsed-time CI threshold is required. Existing
core/evaluator classification, pruning, receipt ordering, exact budgets, drift,
faults and canonical bytes must still pass; rerun the runnable public example.
Any speed claim requires the actual measurements. Commit boundary: one signed,
hooked paging candidate fix with focused correctness/scaling witnesses; normal
push/readback and issue/PR disposition. A prose cost disclaimer does not fix PF1.

## Requirement, integration and feedback accounting

| Area | Independent gap judgment |
| --- | --- |
| Base/delta/head/removal composition | Public new/append/mutation/snapshot/seal APIs cover the requested shape. Source chronology and existing independent operation/eager fixtures support removal/reinsertion and snapshot isolation. No hidden eager whole-stack query adapter was found. |
| G7/G8/G9/G10 | One physical accessor/cache/limits/latch and exact qualified receipts are wired. Full original descriptor vector, including skipped/zero-page sources, is checked. Existing tests cover inclusive/zero/refused budgets and sticky faults. PF1 concerns computation beyond those I/O receipts. |
| Term identity/RDF 1.2 | Source dictionaries map by values; summaries retain local IDs. Scoped/nested/directional/reifier/annotation/graph witnesses are meaningful. HF1 prevents clearing complete shared native identity behavior. |
| Read equivalence | Real `NativeSparqlEngine::query_fallible_view` and prepared entry points consume the stack, with independently constructed eager ordinary and typed inputs. Exact solution bags/cells, ASK, CONSTRUCT carriers, paths and graphs execute. Representative tests are not exhaustive proof of every query. |
| Canonical fold | Guarded logical drain, canonical typed/value partition and actual PackBuilder carriers implement the fold; independent effective inputs and ordered carrier/dictionary comparisons cover multiple histories/bounds and empty output. Failed reads publish no artifact. Shared partitioning is intentional, not a drained-stack oracle. |
| Shipped callers | Runnable evaluator example and root-only umbrella smoke use actual public APIs; selective RDF exports were repaired. No new binding methods are required by this core mechanism issue. Executed example observes retained Bob/current Robert, exact receipts, prepared reread reuse and eager byte parity. |
| Source structure/repository law | No runtime dependency, feature, generated edit, fabricated vocabulary, helper exemption or competing encoder/accounting body. Shared classifier preserves old delta conditions. HF1/PF1 are concrete exceptions to otherwise coherent reuse. |
| Ownership/depth | Storage/log/when-to-compact are explicitly excluded by the issue; consumer atomic publication and depth policy are intentional boundaries. Plain fold preserves current typed rows/graph membership; extra populated explicit/implicit writer lifetime is documented consumer policy. No missing current membership criterion is excused by that distinction. |
| Feedback | Captured native reviews/inline arrays and complete threads are empty; no captured human scope reduction. Refresh is required before exit; empty old surfaces do not prove absence of later debt. |
| Qualification | Logs establish 82 core integrations, 42 mutable, 23 global cases, 32 evaluator integrations plus final 7-case rerun, root smoke, example, affected all-target clippy, release wasm library compilation and helper/profile hygiene. These do not clear cold HF1 or metadata PF1. Wasm compilation is not runtime execution. Pending CI is not PASS. |

The initial separate completion auditor independently adjudicates real-entry-point
demonstrations in `reviews/S2-completion-initial.md` and leaves F1 open. This gap
report does not replace that audit or manufacture final readiness from code/tests.
After remediation, the parent must qualify the changed inputs, refresh complete
review/check surfaces and obtain the fresh final completion audit. Stage 3 then
owns actual candidate binding, ghprsq integration/archive verification and cleanup.

## No-deferrals and laundering-language disposition

Independently reran the prescribed added-line scan over the captured exact
`raw/S2-source.patch`: rg exit 1, no hits. Read existing scan evidence and ran
the prose/laundering scan across the plan, PR body input, task commit messages,
issue-update inputs and Stage 1 handoff. The only hit is handoff line 13,
"No issue requirement was deferred or cut": a negated accounting statement,
not a promise to postpone work. Actual captured PR/issue text and logical
commit messages contain no hidden stub/TODO, later fix or ownership laundering.
The normal base-sync merge message describes actual incorporated commits.

No unauthorized scope cut, fake fallback or unimplemented production path was
identified. HF1 and PF1 remain required fixes now; filing another issue or
adding an emergency ledger entry cannot discharge them. The notice/marker-only
ledger remains unchanged. Keep Stage 1 plan intact and publish a separate
remediation-plan covering these coherent fixes, checks and publication points.

VERDICT: BLOCKED
