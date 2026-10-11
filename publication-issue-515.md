# Issue #515: sparql-eval/datalog: per-row evidence -- information state per standpoint, completeness, frame -- combined by the bilattice

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Per-row **evidence** through `sparql-eval` and `datalog`. Each solution row carries a small id into a per-query interned evidence table. Each entry holds three independent parts:

1. **information state per standpoint**: GMEOW's four values, `supported`, `opposed`, `both`, `neither` (Belnap's FOUR), from the matched claims' polarity;
2. **completeness**: `complete`, `undetermined` or `not-evaluated`, with ledger reasons;
3. **frame**: the non-actual contexts the row rests on (counterfactual, assumed, imagined, licensed falsehood), or none.

Every operator combines its inputs' evidence into its output's. Id 0 is the common case (`supported`, `complete`, actual), so the fast path pays nothing. Combination is a lookup memoised over pairs of ids, and the table is canonically ordered, so output stays byte-identical.

Today the closest thing is `FallibleDatasetView::operation_status`, whose `Evidence` is per **view**. This asks for evidence per **row**.

## Why

Katamari ADR-0045 45.20 (Proposed; Patrick approved this design 2026-10-09). Design note: `katamari/docs/design/PURRDF-SEAM.md` §3 at `e65fe50`. Four reasons:
- A join of an incomplete input with a complete one must not come out looking complete.
- `both` (a contradiction within a standpoint) must not be read as "unknown".
- Negation over missing data must not succeed: deception by omission.
- No conclusion from a counterfactual or imagined premise may reach the actual frame.

## Combination rules (per standpoint, pointwise)

| Operator | Information state | Completeness | Frame |
|---|---|---|---|
| Join | truth-order meet (t∧t=t, t∧f=f, t∧both=both, t∧neither=neither, both∧neither=f, f∧x=f) | the worse; ledgers united | the meet of frames |
| Duplicate solutions merged | information-order join (t⊕f=both, t⊕neither=t, x⊕x=x) | the better | actual if either is actual |
| `UNION` | each branch's row keeps its own | own | own |
| `FILTER` that decides | unchanged | unchanged | unchanged |
| `FILTER` the data cannot decide | unchanged | `undetermined`; the row moves to a separate undecided partition, counted and enumerable on demand | unchanged |
| `OPTIONAL` with no match | left side's | `undetermined` for the optional part unless the right side was complete | left side's |
| `EXISTS P` | information-order join over matches; `neither` if none | P's | union over matches |
| `NOT EXISTS P` / `MINUS` | negation of `EXISTS` (¬t=f, ¬f=t, ¬both=both, ¬neither=neither) **only if** P is complete **and** a closure is declared (`logic:ClosureEntry`); otherwise `undetermined` | as stated | P's |
| Property path | a found path is a witness, and **complete** | "no path" complete only under the `NOT EXISTS` conditions | along the witness |
| Aggregates | per standpoint | `COUNT`, and `SUM` over non-negatives, are lower bounds; `MAX` a lower bound, `MIN` an upper bound; `AVG` undetermined; each labelled | union |

The principle: **incompleteness never weakens a found witness, only conclusions from absence and totals.**

**Standpoints** follow GMEOW's `gmeow:sharpens` poset (Standpoint Logic): a claim according to S2 counts for S1 whenever S1 sharpens S2. `gmeow:unspecifiedStandpoint` sits outside the order and is reported as unscoped evidence. **Frames** join at their meet: the intersection of standpoints, or the conjunction of scenarios. An actual premise joined with a scenario premise holds in the scenario.

## Acceptance criteria

- Property tests: evidence combines per the table for random algebras.
- A join with a locked or incomplete input is labelled incomplete.
- `NOT EXISTS` over incomplete input is `undetermined`, and true only with completeness and a declared closure.
- A found property path is complete even when other data is missing.
- Output is byte-identical across runs and targets.
- With all-default evidence, throughput is within noise of today's.
- No compiled-in limit.


## Comments (2)

### paudley — 2026-10-09T21:07:06Z

## Addendum: answer tiers, so a yes or no stays cheap

"Was there a blue thing here before?" wants `yes`, `no`, `both` or `unknown`, not thousands of annotated rows. So the evaluator should take the **tier** a query asks for and compute only that:
- **verdict**: one value, or rows plus one answer-level status;
- **summary**: plus per-reason counts;
- **rows**: per-row evidence;
- **ledger**: the RDF 1.2 ledger (#516).

The witness rule gives early termination at the verdict tier:
- `yes` stops at the first supported witness;
- `both` stops at a supported and an opposed witness;
- `no` needs a complete search under a declared closure, and the evaluator stops as soon as `no` is unreachable (an incomplete input), answering `unknown`.

Evidence ids are still tracked at every tier, since `no` *is* a completeness judgement, but nothing is rendered below the tier asked for. Design: Katamari `docs/design/PURRDF-SEAM.md` §3.7.


### paudley — 2026-10-10T04:09:32Z

# Row evidence, queryable ledgers and scoped candidate generation

Complete delivery:515,516 and511, including both accepted addenda. Sole shipping
writer:root in the isolated515 worktree returned by stagectl. Existing Sparrow,
ranking and geo writers continue unchanged. Ready protected merges take priority
over opening additional source units. No issue/PR/worktree boundary limits this
group; shared evidence and undecided-row ownership is the reason to combine it.

## Governing inputs and prior art

Read the complete issue bodies/comments saved as issue.md, issue-516.md and
issue-511.md; the current root .baseline/.goals and applicable AGENTS.md govern.
The ignored root context files are not present in the fresh worktree; their
absolute root copies were read. Katamari's actual local PURRDF-SEAM.md supplies
the approved design. Its referenced ADRs are upstream context, not invented local
PurRDF files. Existing core loss.rs owns conversion kinds/locations; operational
FallibleDatasetView status owns hard runtime failures. Neither presently supplies
per-row semantic evidence. Existing retrieval compile emits producer depth and
outer LIMIT, so filtering its returned list cannot implement scope-before-depth.

Owned GMEOW Belnap/standpoint code may be reused. Its information-order meet is
not the truth-order meet required for joins: verify the actual truth tables.
Do not import its serde dependency, ontology namespace or donor size limits.
Canonical digest/framing and numeric jobs remain in their existing homes.
Fresh recurrence counts beyond the captured prior-art evidence are UNKNOWN;
neither the number of matching titles nor absent local ADRs proves recurrence.

## Complete production design

One shared core evidence home supplies FOUR, completeness, causes/concerns,
standpoint order, non-actual frames and a query-local interner with pair memo.
Default id0 is supported/complete/actual. A uniform-default row column stores no
per-row vector; only non-default rows materialize the admitted native column.
Interning order must not leak into published identities: canonical content order
and remapping, sorted frame/reason components and existing framed hashes govern
serialization. There is no chosen dictionary, row, reason or poset size limit.
Cycle/invalid-reference and physical/resource failures are typed hard errors.

Standpoint claims are evaluated per selected standpoint with transitive sharpens
inheritance; unspecified remains outside the order and is explicitly unscoped.
Reuse the existing caller-supplied StandpointPredicates declaration. Its heldIn
function currently asks whether a claim is held in the queried broader standpoint
by a direct materialized sharper vantage edge; that query is distinct from this
contract's selected-sharper standpoint inheriting a broader claim. Do not reverse
either relation silently. Put shared poset representation/walks in one home and
retain explicit caller semantics, with directional fixtures for both consumers.
The existing property-function Completeness is a producer attestation with a
lossy-producer cause; translate that actual declaration into row completeness
instead of treating it as default complete or duplicating a producer contract.
Join is truth meet; duplicate coalescing is information join. Completeness and
frame are independent components, never inferred from FOUR. Conjunctive frames
combine scenarios/standpoint intersections without promoting them to actual;
duplicate alternatives are actual when either actual witness exists.

Wire the actual SPARQL solution carriers and Datalog relations/derivations, with
the accepted native allocation/ownership homes. Every operator's real bulk and
delivered routes carry evidence: BGP, paths, join/LATERAL/correlation, UNION,
FILTER/BIND, OPTIONAL, EXISTS/MINUS/NOT EXISTS, DISTINCT/REDUCED, ordering/slice,
group/aggregates, property functions and SERVICE. There is one evaluator/planner,
not a second evidence engine. Existing standard query results preserve their
shape; an evidence request exposes the richer result through the same execution.
Caller-selected semantic declarations supply standpoints, closures and gap
causes. Caller vocabularies are required for RDF rendering; no ontology default.
The common standard path carries uniform id0 and pays no new per-row allocation.

Semantic gaps are input facts, not an exception-catching policy. Faulting scans,
missing required dependencies, cancellation and admission failures retain their
existing typed-error precedence and never become undetermined rows. Decidable
FILTER leaves evidence unchanged; an expression genuinely undecidable from the
declared input retains its original row in the separate undecided partition.
No optional-right match preserves left information and marks its optional part
undetermined unless that right search was complete. A found property-path
witness is complete even if unsearched data is missing. Absence requires complete
search and an explicit closure. Negation swaps supported/opposed and preserves
both/neither; neither alone is not a closed negative witness.

Aggregates carry the original inputs' standpoints/frames and label partial COUNT,
nonnegative SUM and MAX as lower bounds, MIN as upper bound, AVG as undetermined;
negative SUM is not mislabeled monotone. DISTINCT merges identical solutions with
information join, better completeness and alternative-frame union; UNION retains
each branch's own evidence until an actual coalescing operator.

The request specifies verdict, summary, rows or ledger. All track evidence ids;
only the requested tier is rendered. A witness question explicitly asks for
positive support, contradiction, or closed absence: first supported witness
settles the positive question; both polarities settle contradiction; incomplete
search makes closed absence unknown immediately. Do not promise a definitive
both answer while stopping after the first positive witness. Review this query
specification against the accepted early-termination addendum before coding.

RDF rendering is lazy and uses the original interner. Each solution evidence node
is a genuine reifier of its solution triple term. Each reason is a genuine
reifier of a concern triple term and has a separate cause annotation. Canonical
cause/constraint/vault/watermark content determines reason identity, not row id:
2279 rows lacking the same field can share one reason. Row-specific bindings stay
on row evidence. Fact of incompleteness is independent of reason links; cause is
independent of the reified concern so ordinary CONSTRUCT can withhold concerns
or reasons while retaining incompleteness/cause. The ledger is one default graph,
never a named-graph side channel. Existing conversion loss codes map to causes
without replacing conversion contracts or the existing JSON ledger.

Accepting reasons performs a stable ordered merge of retained established and
undecided rows using their original keys, scores and ranks. It never re-queries
the provider or retrospectively calls an incomplete answer complete. Keep all
original reasons, annotate the explicit acceptance and bind the acceptance set
into merged answer identity. A row carrying an accepted reason is admitted even
if other reasons remain, with those causes still visible. Repeated acceptance
must not duplicate rows or mutate original answer identity.

Field scopes and time/place/entity constraints are request values, canonically
serialized and admitted with the plan. Predicate-to-field resolution happens at
query time; postings remain field-free. Compile/apply membership inside EVERY
stratum's actual producer candidate generation, before producer depth AND outer
LIMIT. Scope is not a ranking weight. Producers implement the declared scoped
seam or refuse admission explicitly; a post-depth adapter is forbidden. Missing
time, missing place hierarchy and coarser temporal precision retain original
ordered candidate keys as undecided, with per-constraint counts/reasons. Fusion
and accepted-reason merge keep original producer rank/score/order without a
provider rescan. All default producer/query behavior keeps its existing law.

## Completeness and executable acceptance

All rows below belong to Task1 implementation and Task2 qualification/publication.
Use real production entry points and unchanged controls, not kernel-only claims.

| Requirement | Actual acceptance |
|---|---|
| FOUR and independent components | Exhaustive truth/information tables plus seeded random algebra properties; pointwise selected standpoints and unrelated completeness/frame changes |
| Standpoints and frames | Actual sharpens chain/diamond, cycle refusal, unspecified claim, actual/scenario and scenario/scenario queries; no imagined conclusion promoted to actual |
| Whole SPARQL/Datalog propagation | Real bulk/delivered, direct/prepared/governed routes over all operators and Datalog recursive joins/duplicate derivations; identical answer rows plus expected evidence |
| Incomplete joins and operational faults | Locked/declared-gap input joins complete input and stays incomplete; original faulting provider/cancellation/admission controls still hard-fail with precedence |
| FILTER/OPTIONAL/negation | Enumerable undecided FILTER rows, complete/incomplete optional absence, incomplete NOT EXISTS/MINUS unknown, complete declared closure negative and no-closure unknown |
| Paths and partial aggregates | Found path through incomplete source is complete; absence obeys closure; COUNT/SUM/MAX/MIN/AVG labels checked with positive and negative inputs |
| Tiers and early stops | Counting real producer proves first-support and two-polarity stops, incomplete closed-no stop, no rendering below tier, summary counts and complete row/ledger tiers |
| RDF ledger and dedup | Actual undecidable query yields reifier ledger; same NativeSparqlEngine queries it with GROUP BY and2279 rows share one reason node |
| Safe projection | Actual CONSTRUCT removes concern/reason nodes while separate incompleteness and retained cause survive; genuine RDF1.2 triple terms checked |
| Accepted merge | Ranked and fused established/undecided streams merge in original order with unchanged scores/ranks; acceptance annotation/identity change; provider counter proves no re-query |
| Pre-depth field scope | Valid candidates beyond unscoped depth appear in scoped top-depth; multiple strata, depth0/1/full neighbors, predicate remapping without rebuilt postings or weight changes |
| Hard constraints | Actual missing time/place hierarchy/coarse precision are counted and enumerable per constraint, not false; unsupported scoped producer admission is a typed error |
| Determinism and defaults | Frozen native/actual WASM serialization, permutation/parallel interner construction; original conformance corpus and measured default throughput within original harness noise |
| Physical ownership | Before-birth allocator refusals, held-row/clone/last-owner and cancellation conservation using the accepted shared homes; no compiled semantic limits or runtime dependency |

## Alternatives and boundaries

Reject full evidence objects per row, a parallel evaluator, scope-after-depth,
unknown-as-false, exception swallowing, named-graph ledgers, implicit ontology
namespaces and accepted-reason re-query. They respectively duplicate storage,
semantics, lose candidates, misstate knowledge or violate the explicit contract.
Do not combine the independent unlimited ranking leaf or physical-shape campaign
merely because they share crates. Conversely none of this delivery's operator,
producer, frame, host or ledger requirements is moved to those campaigns.

## Task 1: Complete evidence and scope production migration

Implement the single shared evidence/concern/frame home, complete SPARQL/Datalog
carriers/operators, tiers, lazy reifier ledger, ordered accepted merge and every
scoped ranked producer/caller as one coherent source unit. Write meaningful
native/portable/negative/default-performance cases alongside the production
paths. Resolve the combined independent intake/prior-art/plan review first.
No standalone kernel PR or partial operator publication counts as this task.

## Task 2: Qualify, review, publish and integrate the complete three contracts

Collect the complete affected native/portable family, repair shared findings,
then run the original required full gate on settled source. One independent
complete contract judgment covers actual callers, old semantics, defaults and
every acceptance row. Commit with normal signing/hooks, push and publish the
plan/results and one grouped PR linking all three closures. Satisfy current
hosted acceptance and distinct final feedback debt, assess the actual combined
tree, integrate only through ghprsq, verify all three issue closures and archive,
clean only owned worktree/branch, and update REWORK_LIST. Target one settled full
gate; failed required gates must be fixed, never waived by a numeric budget.

Current acceptance: complete plan written; independent plan judgment, source
implementation, all runtime qualification, publication and integration NOT MET.
No authorized scope cuts or deferred criteria.


