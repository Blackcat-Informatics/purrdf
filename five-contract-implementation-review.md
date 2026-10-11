# Independent five-contract implementation and completion review

Status: FAILED

Verdict: FINDINGS-OPEN

Scope: the complete combined delivery for #515, #516, #511, #518 and #512,
including original SPARQL/Datalog operators, producer scope, physical shapes,
native ownership, hosts, portable execution and default performance. Comparison
base: `a4bb1232d`. This is a Stage-1 independent task review and Stage-2
implementation-rubric review before PR creation; it does not authorize a subset
PR or turn unfinished requirements into followups.

The source inspection finds three substantive contract failures. Required current
runtime, actual WASM, performance and final gates also remain unverified. Earlier
strict or focused runtime successes do not certify those missing requirements.

## Review authority and evidence

Applied the complete `stagectl/references/completion-adversary.md` implementation
rubric, Stage-1 independent task-review rules, and shared quality, validation and
no-deferrals instructions. Read repository AGENTS, `.baseline`, `.goals`, root
and worktree `.deficiencies`. Both emergency ledgers have no emergency entries.
Read all five issue inputs, issue analysis, combined plan, shape-contract plan,
plan review, ordered-storage review, fusion-storage source review, validation
record, native-repair-2 rationale, and shipped physical-shape design.

Inspected the complete changed implementation by contract family, with original
callers and concrete tests: shared core information/frame/record/dictionary and
accepted merge; evaluator ingress, VM transitions, expression/negation/path and
modifier integration; original Datalog evidence propagation; shape registry,
dictionary/window/bucket/span execution; original text occurrence producer;
retrieval scope, ranked execution, search and fusion. This is a source/wiring
review, not a claim that reading code proves all execution behavior. No duplicate
test suites, process changes, shipping-source edits, git mutations or publication
actions were performed.

Attributable evidence reused: strict native-repair passes; prior native production
ledger/acceptance controls; the production GROUP BY/CONSTRUCT ledger receipt;
original real-producer shared native/portable cases; prior default baseline.
Native full run 54404 exited 101 with 13 failed targets on earlier source. Portable
54548 failed an ordinal expectation before the repair and did not execute its
remaining cases. Current native 31994, probe/prepared 68624 and release 63725 were
pending when inspected. Baseline measurements cover 36 benchmarks, but no current
default comparison or required physical performance measurement is certified.

## Concrete findings requiring one implementation batch

### F1 — Phrase producer declares an invalid physical row bound

`crates/text/src/dictionary_relation.rs:35` initializes `rows` with
`index.token_occurrence_count()` and `:107` returns it as `rows_per_invocation`.
The original literal matcher in that same module joins consecutive occurrences
across physical documents with the same logical subject/language. Its exact bag
therefore contains cross-document derivations. Two aliased documents, each
containing the same three-token phrase, have six physical token occurrences but
eight matching derivations (`2 × 2 × 2`). Six is not an upper bound of eight.

This is not an optimization-only concern: the original property-function trait
requires the declared per-invocation bound to respect reality for governed
admission. A correct literal result can be refused as exceeding its declared
physical bound, and an allocation admission based on the declaration is false.
Preserving the original bag rules out collapsing aliases to hide the defect.

Required repair: derive a conservative finite bound from actual contiguous
subject/language position runs and branch multiplicity, with checked/saturating
representation only where genuine derivation cardinality warrants it. Keep the
no-alias path linear. Demonstrate a governed original-consumer alias case whose
derivation count exceeds total physical token count, on native and actual WASM;
preserve original duplicates, order and exact resource-failure semantics.

### F2 — Fused undecided acceptance has no complete production consumer

Original issue #516, `issue-516.md:23–24`, requires:

> `merge(answer, accepted reasons)` folds the undecided rows carrying those reasons into the established stream, in order. Each admitted row keeps its reason plus an acceptance annotation, and the acceptance is part of the merged answer's identity: a visible, explicit override.

> Undecided rows keep their ranks and scores, so a merge into a ranked or fused answer is an ordered merge, not a re-query.

The combined plan's acceptance row, `plan.md:140`, is equally explicit:

> Ranked and fused established/undecided streams merge in original order with unchanged scores/ranks; acceptance annotation/identity change; provider counter proves no re-query.

Actual path: `CallRead::pull` in `crates/retrieval/src/execute.rs` retains scoped
raw cells and `RowEvidence`, then excludes undecided rows from `ReadStep::Row`.
`search_scoped` (`crates/retrieval/src/search.rs:548`) returns an ordinary
`SearchResult` and separate per-stratum `ScopedRows`. Its original search path
calls `fuse_admitted` at `:790` over established ranked streams only.
`FusedRow` (`crates/retrieval/src/fusion_stream.rs:442`) has original fused score
and contributions but no shared per-row evidence/undecided partition or accepted
merge consumer. The existing fusion trailer's generation evidence identity is
not the shared query reason/acceptance identity. Core `AcceptedMerge` is wired
to ordinary SPARQL partition cursors, not this fused answer.

Consequently an undecided candidate has a raw per-stratum receipt, but no retained
original fused score, contributions and total order that an acceptance consumer
can borrow. `scoped_real_producers_keep_unknown_receipts_outside_the_original_fusion`
explicitly tests leaving unknown candidates outside fusion; it does not
demonstrate their required fused accepted merge.

Smallest existing consumer to complete is the public `search_scoped` path through
the original scoped execution, `RankedStreamAdapter`, `fuse_admitted` and
`SearchResult`. Retain the required undecided fused partition during that original
composition under the existing fusion/contribution law and native query owners,
then expose its ordered accepted merge through the shared acceptance law. Do not
invent another scoring implementation or re-query/re-fuse on acceptance. Prove
an established and undecided interleaving with original fused scores,
contributions/order, unchanged provider counts, visible acceptance annotation and
identity change, including a candidate shared across strata and native refusals.

### F3 — Scoped settlement erases native operational failure identity

`OnDemandRead::settle` (`crates/retrieval/src/execute.rs:862`) returns
`Result<PfAttestation, String>`. `CallRead::settle` (`:3333`) drains remaining
scoped rows and converts `next_admitted_row`, retained-row push, cursor settle
and retained finish errors with `error.to_string()`. `RankedStreamImpl::settle`
(`:1514`) wraps these as `ProtocolError::AttestationMoved`.

Thus a native governor/allocation/segmented refusal occurring after a consumed
prefix while settlement drains an unknown suffix loses its typed original
cause. This differs from the pull path's native `ReadFault` handling.
`FusionStream` (`fusion_stream.rs:2683`, `:2833`) latches terminal native failures
only through `FusionError::NativeFailure`; the stringified attestation error
misses that path. Rendering a string also allocates while handling refusal.

Required repair: carry a typed read/native fault through settlement, distinguish
genuine attestation changes from operational refusal, and route original native
failure through the existing terminal latch. Demonstrate refusal occurring only
after an established prefix during suffix settlement: the original typed cause
must persist across repeated settle/next/trailer requests; no successful trailer,
callback reentry or lost native grants. Do not replace typed refusal with a
different protocol error or stringify it before dispatch.

## Requirement-by-requirement judgment

`SOURCE-COVERED / UNVERIFIED` means the actual production wiring was inspected,
but required current execution or broader acceptance evidence remains missing.
It is not a completion pass.

| Combined requirement | Actual demonstration or source judgment | Verdict |
|---|---|---|
| Shared FOUR, pointwise concern evidence and frame home | Core information laws, canonical factorized records, frame meet/alternative absorption and shared dictionary are actual homes; evaluator/Datalog call them | SOURCE-COVERED / UNVERIFIED current full execution |
| Caller vocabulary, triple concerns, frame selectors and typed construction | Actual validating constructors and source bridge; concern triple refusal remains enforced | SOURCE-COVERED / UNVERIFIED portable |
| All SPARQL operators/query forms and Datalog rounds | Original evaluator/VM, expressions, joins/negation, paths/modifiers, four WHERE-result forms and original semi-naive emission/negative capture inspected; no alternate evidence engine found | SOURCE-COVERED / UNVERIFIED complete current family matrix |
| Semantic uncertainty versus operational failure | Original unknown gaps and source/closure evidence propagate; scoped settlement violates native refusal boundary | FAILED F3 |
| FILTER/BIND/OPTIONAL/MINUS/EXISTS and negative partitions | Original sparse branch transitions, expression gaps and negative scopes inspected; focused historical native controls are attributable but not complete current qualification | SOURCE-COVERED / UNVERIFIED |
| Paths, grouping, aggregates, projection, deduplication and slicing | Original path/modifier integration and evidence propagation inspected; GROUP BY/CONSTRUCT production ledger suffix passed | SOURCE-COVERED / UNVERIFIED full native/portable |
| Tiers and early stops | Original count/query-form status and scoped early-stop path inspected; required complete forms/tier/stop matrix not certified | SOURCE-COVERED / UNVERIFIED |
| Lazy queryable RDF reasons ledger | Native actual-query ledger bridge, original reasons and accepted annotations; production ledger receipt passed | DEMONSTRATED focused native; UNVERIFIED full portable |
| Privacy-safe evidence projection | Canonical alias projection/factorization inspected; no default ontology or reason-dropping workaround found | SOURCE-COVERED / UNVERIFIED current complete matrix |
| Ordered accepted merge, including ranked/fused answers | Ordinary borrowed SPARQL merge preserves row identity and focused native scores; required fused consumer absent | FAILED F2 |
| Actual pre-depth text and kNN candidate scope | Borrowed CandidateScope evaluates inside original candidate generation before requested depth; shared real-producer cases exercise two actual strata | SOURCE-COVERED / UNVERIFIED current native/portable |
| Hard constraints, resource/refusal conservation | Original row/native admission integrated, but false phrase bound and settlement type erasure violate contract | FAILED F1, F3 |
| No default behavior/allocation regression (original allocation 50) | Original no-evidence path and zero-column approach inspected; original default baseline exists; current exact allocation/performance comparison missing | UNVERIFIED |
| Logical and physical owners, bounded results/fusion | Actual owner-backed shapes and ranked/fusion admission inspected; prior fusion source review is source-only; settlement refusal remains broken | FAILED F3; remaining execution UNVERIFIED |

| Shape-plan acceptance | Actual production path / remaining acceptance | Verdict |
|---|---|---|
| S1 registered native host offers/declines | Public registry, typed offer request, original host fixtures and decline/panic controls inspected | SOURCE-COVERED / UNVERIFIED current runtime |
| S2 all applicable query forms | Shape selection uses original central evaluation entry and original modifiers; SELECT/ASK/CONSTRUCT/DESCRIBE controls present | SOURCE-COVERED / UNVERIFIED |
| S3 shaped versus literal exactness | Shared public physical-shape cases retain original literal oracle; seeded aliases/order/duplicate controls inspected | SOURCE-COVERED / UNVERIFIED native+WASM |
| S4 evidence and zero-budget behavior | Node charge precedes shape offer; window restriction declines evidence-input shortcut so expression-gap callbacks remain; original evidence source carried | SOURCE-COVERED / UNVERIFIED |
| D1 complete dictionary and original analyzer/positions | Actual shared occurrence dictionary and original analyzer tokens, expanded relation adjacency and original literal phrase matcher inspected | SOURCE-COVERED / UNVERIFIED |
| D2 compile reuse and parameter boundary | Original owned compiled cache and `prepare_execution` parameter slots; repeat/prepared fixture uses actual production door | SOURCE-COVERED / UNVERIFIED current runtime |
| D3 real 534/4097 dictionaries and one-pass scan | Public consumer fixtures and actual scan/compile counters inspected; no invented cap found | SOURCE-COVERED / UNVERIFIED measurement |
| D4 aliases, graph/predicate/language and exact bags | Literal preserves cross-document derivations; declared upper bound contradicts them | FAILED F1 |
| D5 span selector | Public caller-selected longest-first/containment semantics, independent oracle controls and original borrowed selections inspected | SOURCE-COVERED / UNVERIFIED |
| D6 no policy cap, truthful finite native admission | No authored dictionary limit substituted, but token count is not phrase derivation bound | FAILED F1 |
| W1 temporal windows, ordering and original arithmetic | Original BGP probe order, admitted indexes/masks and sweep; strict repaired compilation passed, full current runtime pending | SOURCE-COVERED / UNVERIFIED |
| B1 full calendar buckets and original aggregates | Existing XSD calendar parser, bucket recognition and original grouping/aggregate body; member and first-seen order restored | SOURCE-COVERED / UNVERIFIED native+WASM |
| P1 real consumers, all owners and hard failures | Public real-producer paths inspected; scope refusal on settle loses native type/latch | FAILED F3 |
| P2 frozen native and actual WASM parity | Shared original real-producer cases exist; previous portable run failed before later cases; repaired portable execution not certified | UNVERIFIED |
| P3 measured performance and mandatory gates | Baseline only; current comparison, physical measurements, complete local gate, metadata, hooks, hosted checks and protected merge not certified | UNVERIFIED |

## Adjudication of changed fixture expectations

1. **Borrowed CandidateScope eager admission at open: accepted source rationale.**
   The original trait explicitly requires candidate evaluation inside original
   generation before requested depth, forbids the borrowed evaluator escaping
   into a cursor, and makes failure operational. The older zero-publication
   promise is not zero candidate computation. Requiring AllocationFailed before
   either schedule publishes an execution, while proving actual callback entry,
   is stronger failure behavior, not permission to omit scope. Current execution
   still needs certification.
2. **Prepared binding uses `prepare_execution`: accepted source rationale.**
   Original engine documentation makes this the admitted parameter-binding door
   with actual native parameter slots. `declared_prebound` only adds context names
   and supplies no parameter value. Updating the fixture to call the actual door
   preserves the original bound-producer requirement.
3. **Workspace residency refusal leaves segmented snapshot Ready: accepted
   source rationale.** The original segmented-session implementation explicitly
   says a refused workspace resize leaves the readable snapshot valid, whereas
   block-load omissions are sticky failures. The repaired fixture still requires
   the exact Operational Residency refusal, no execution, preserved cache
   pointer, actual compilation entry and a healthy retry. Ready is not evidence
   of query success and does not excuse a missing refusal.
4. **Determinism source inventory 25 to 28: accepted scope change.** Three actual
   new shipping modules are included; this does not waive determinism or float
   restrictions.
5. **Concern fixture now uses a triple: accepted contract correction.** The
   original typed reason-description contract requires a triple; replacing an
   IRI concern exercises that contract rather than weakening it.

## Completion decision

Fix F1–F3 in the existing coherent production path, then bind the required
native/actual-WASM, original allocation/default performance, physical measurements
and final gates to the repaired source. Reuse valid attributable successes;
repeat only affected controls and mandatory missing acceptance. No issue in this
combined scope can be called complete from this review, and no foundation-only
PR, deficiency entry or future followup satisfies the missing contracts.

## Coherent repair source review

F1 now derives a conservative content bound from subject/language contiguous
position runs, retaining physical graph-alias branch multiplicity. Each start
is charged the suffix product of positive branch counts. Any phrase prefix
selects a subset, so the bound dominates the original alias derivations;
positions advance across every original document projection. Runtime alias
bags/native governor behavior remain subject to the required repaired-source
qualification.

F2 has now been implemented through the complete original scoped search,
execute, original RankedStreamAdapter and original fusion consumer. See
`fused-acceptance-implementation.md` for exact source homes, cost and original
consumer controls. No scoring or acceptance-only compatibility subset replaces
the original law. Parent independently reviews this delegated implementation.

F3 retains typed suffix settlement failures and the original terminal latch.
The coherent batch additionally fixes typed-native erasure in scoped original
workspace/receipt publication and actual exclusion output publication, including
the original exclusion method's terminal latch. New shared original-producer
pre-owner and suffix controls qualify these behaviors when run. The precise
allocator failure instant after successful exclusion evaluation is not injected;
this is stated explicitly rather than represented by an unrelated synthetic
allocation test.

Root's ordinary/tracked dedup repair was independently inspected: one exact
binding kernel, original first survivor/order, tracked-only witness combination,
ordinary row-only keys, and concrete key allocation bound. Runtime DISTINCT,
evidence and original allocation controls remain required; paired original
performance is not yet established.

**Combined completion remains UNVERIFIED / NOT COMPLETE:** mandatory final
native families, actual-WASM parity, default allocation/performance and full
gates must qualify the final repaired source. The earlier failed receipts and
source-only repairs do not close any of the original five issue contracts.
