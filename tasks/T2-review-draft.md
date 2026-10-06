# Task 2 independent source review — preliminary

VERDICT: BLOCKED (PRELIMINARY; consumer coverage and stable qualification pending).

Issue: 457. Worktree:
`/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
Branch: `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
Current committed HEAD: `331c44e93aa6bcf8434dd027c4e0aae2b5b939b9`.
It incorporates signed/hooked base synchronization with
`b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac`.
Plan SHA-256:
`ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29`.

## Review scope, identity and limits

Read Task 2's plan, current validation index and T2-consumer-contract.md. Reuse
the independently qualified Task 1 source judgment: its production hashes were
independently rechecked against raw/T1-source-sha256.txt and still match after
base synchronization. Do not infer evaluator or wasm acceptance from that core
review. The base-evaluator patch records five changed evaluator files; Task 2
uses the current QueryOptions::EMPTY public route rather than an obsolete API.

Read all new evaluator tests, executable example and its shared fixture, changed
RDF exports and umbrella smoke. Existing support::solutions is consumed from its
home. No source edits, Cargo/build/runtime checks, forge retrieval, commit/push
or subagent dispatch occurred. Only this report was written. This is independent
source judgment while implementation/checks are changing, not a runtime audit.
No final Task 2 PASS or whole-issue completion is claimed.

Initial inspected source SHA-256 captures:

| File | SHA-256 |
| --- | --- |
| crates/rdf/src/lib.rs | 1b0f9cad6ac0fb83724c5272db77dd7d556d235fd7f19ca49d92e9587fc6eefc |
| crates/purrdf/src/lib.rs | 7035d22faa1e23ab170f1a4b5d429395736e35cdae7667383fc31b07d025e703 |
| crates/sparql-eval/tests/paged_stack_query.rs | f1cec82005e9d4f593ee2b9e1a210bb81397d9184a7a8e55efd35bb42c38a631 |
| crates/sparql-eval/examples/paged_stack.rs | 765580136161854f9586d3314c3ccb7455eb2f2b80530cfc4aa4eb53d3d80ff0 |
| crates/sparql-eval/examples/support/paged_stack.rs | e5df7a0504d4f2c517fc308dca5402f0fac0fd8724e7ab73f8f30488726837f7 |

## Prompt findings and acceptance observations sent to parent

### R1 — no delta/head-only named graph is observed through the evaluator

Status: OPEN; required consumer-coverage gap, not a presumed implementation bug.

All GRAPH matrix data currently originates in initial bases: empty/implicit in
the ordinary fixture, g/elsewhere in the typed fixture. Every new ordinary delta
or head value uses the default graph. The example and root smoke also only add
default-graph head rows. Thus the real evaluator demonstrations do not establish
the T2-consumer-contract's required delta-only graph identity.

Repair/evidence: introduce a graph/value solely in a sealed delta or resident
head and compare constant and variable GRAPH queries, including graph membership,
before and after head sealing through both public guarded entry points. Build
the eager input independently and include ordered compacted page-byte parity.
An empty newly declared graph and a row-bearing new graph can share this fixture.
This is a concrete absence in the inspected test inputs, not an allegation that
the production stack cannot represent a new graph.

### R2 — generated property-path submatrix has no path witness

Status: OPEN test-utility observation.

generated_mutations_and_retained_snapshots selects QUERIES[8], which starts at
ex:alice, but generated data has anchor/s*/o* subjects and no alice term. The
path result is therefore empty in every generated snapshot, even though the
test's anchor ensures some unrelated data remains nonempty. The ordinary
chronological fixture does exercise a real alice path, so this does not prove
property paths are entirely missing.

Prefer a seeded real anchor path or a generated path keyed on terms the sequence
changes, and assert a nonempty witness before checking mutation effects. Do not
describe this always-empty generated submatrix as evidence that generated path
closure changes correctly.

### R3 — newly promoted physical ordinary rows have core-only coverage

Status: bounded consumer-coverage improvement for adjudication; no code defect
presumed, and the existing real evaluator demotion case is meaningful.

The typed consumer fixture begins with native reifier/annotation tables and
removes the last declaration, then compares demoted ordinary results. It does
not currently add an external reifier above a physical ordinary row through the
guarded evaluator. Task 1 proved that direct-view path, but Task 2 is the caller
demonstration. A small extension can add a head/delta reifier for an ordinary row,
construct the expected annotation table explicitly, query both public routes and
compare actual fold bytes. This can reuse the new-graph coverage fixture.

## Positive source judgment

- matrix invokes NativeSparqlEngine.query_fallible_view and
  query_prepared_fallible_view directly over fresh PagedStackQueryView instances.
  Expected results query a separately constructed native dataset; the oracle
  does not drain the stack. The same prepared syntax enters both views.
- The ordinary oracle independently maintains BTreeSet value mutations and
  reconstructs eager rows with RdfDatasetBuilder. The typed oracle explicitly
  constructs reifier/annotation tables and separately constructs the post-removal
  ordinary table. It preserves scoped blanks, nested triples and directional
  literal values rather than flattening the RDF 1.2 surface.
- SELECT comparisons use the full variable list and Vec<Vec<Option<TermValue>>>
  equality, preserving duplicate solution rows and unbound cells. Matrix queries
  use defined ORDER BY clauses where multiple rows matter, and an ordered
  LIMIT/OFFSET case is present. UNION, DISTINCT, OPTIONAL, MINUS/NOT EXISTS,
  aggregation/subquery, ASK, property paths and named/default graph queries have
  concrete inputs. CONSTRUCT compares deterministic PackBuilder bytes.
- Independently numbered base dictionaries are checked to disagree at numeric
  ordinal zero; repeated physical rows have a nonempty witness count of four
  effective edges. Retained pre-edit/removed snapshots are queried after later
  head mutations and seals. Generated operation history checks mutation booleans
  against independent values and revisits retained snapshots.
- Compaction compares actual ordered per-page PackBuilder carriers against
  independently effective eager input under multiple row bounds and histories;
  completely removed input is also observed. It does not substitute RDF digest
  equality or translation-only renumbering for page identity.
- Operational error helper requires the Operational variant and verifies there
  is no diagnostic substitution or partial-answer payload. Provider, cancel,
  deadline, explicit corrupt-carrier and changed materialization faults retain
  typed causes, fault after admitted head data refuses publication, and a later
  prepared constants query cannot escape the latch. Fold refusal is checked.
- The public result boundary tests global zero/equality/one-below page/byte
  limits, exact qualified newest-first origins, deterministic head charge, cache
  rereads and provider call counts. Constants and head-only queries cover both
  generation and page-count drift in skipped and zero-page sources, with zero
  requested/consumed pages and no lower materialization.
- The example actually constructs independent sealed generations, retains a
  reader, removes/replaces values, seals and continues mutating, invokes both
  public evaluator routes, asserts Bob versus Robert and identical cached
  receipts, and folds to independently computed eager carrier bytes. It prints
  asserted concrete row/layer/page/byte/origin observations. Shared fixture code
  only builds inputs and carriers; it does not replace the live stack mechanism.
- RDF selective exports add the nine public core stack/canonical symbols.
  Umbrella smoke constructs/mutates a stack through the supported root facade,
  evaluates a guarded ASK with Head evidence and performs canonical fold. No
  runtime dependency or feature was added.

## Pending evidence and final boundary

The first implementation/check pass is active; no attributable stable
command/exit/source handoff has yet been supplied for this task. Required
evaluator regressions/example, supported facade test, affected all-target clippy,
helper qualification on the synchronized tree and core/evaluator/umbrella wasm
builds remain UNVERIFIED in this preliminary review. Pending commands are not
reported as failures. Final adjudication must read actual logs, bind identities
and recheck substantive repairs, while reusing unchanged qualified Task 1 core
judgment rather than repeating it.

## Repair recheck during the active qualification pass

Re-read test source at SHA-256
`a9931489d1176f60fe2f58d63d7cea22388e748f3ef47bfc08f7124304f8a0fa`.
The other four initial file captures above were unchanged at this observation.
This source is still pre-final-handoff and is not bound to final runtime exits.

- R1 is repaired in source: a row-bearing delta graph and explicit deltaEmpty
  declaration are introduced in the resident head, observed by constant/variable
  GRAPH and membership queries before and after seal, compared to independent
  eager rows/declarations, then the row-bearing graph is withdrawn after last-row
  removal/seal. Retained head snapshot and ordered fold bytes are checked too.
- R2 is repaired in source: generated path queries now start at the actual seeded
  anchor self-cycle, with the exact nonempty one-row closure asserted at every
  generated step. This proves retained nonempty cycle closure amid other edits;
  the ordinary chronological fixture provides changing path-closure coverage.
  No claim that this fixed anchor cycle itself changes during generation is made.
- R3 is repaired in source: an external head reifier promotes the existing
  physical ordinary r/note row in elsewhere. The oracle explicitly builds that
  annotation and new reifier while retaining the demoted g note as ordinary.
  Ordinary/annotation/reifier counts, both public matrix routes and eager actual
  page bytes are checked for both head and sealed publication, then the older
  demoted snapshot is revisited.
- The new native ordinary/virtual-reifier overlap consumer regression compares
  SELECT bags and COUNT against an independent native input, retains both native
  streams, and compares actual canonical page bytes. This honors the existing
  native contract rather than inventing cross-stream suppression.
- Descriptor drift now targets chronological source ordinal 1 beside an unchanged
  older source. It asserts that ordinal, two retained source descriptors, typed
  generation/page-count causes and zero requests/charges, including a zero-page
  second source, through both guarded entry points.

Read raw/T2-example.log: the executable completed and printed old Bob (2 layers,
2 pages, 202 bytes), current Robert (3 layers, 4 pages, 1088 bytes), qualified
origins, and successful ordered carrier equality for one compacted page. Also
read raw/T2-new-second.log: its historical six-case pass predates the current
added overlap regression, so it does not qualify the final seven-case target.
Final execution exits/current identities, affected existing tests, facade checks,
clippy/helper hygiene and wasm remain pending final handoff.

No additional source defect is established after these bounded repairs. The
historical preliminary findings remain above; no final PASS or independent
runtime execution is inferred from this repair recheck.

## Stable native handoff recheck; remaining gates pending

Independently verified all five entries in raw/T2-source-sha256.txt. The final
evaluator-test SHA-256 is
`e362206826c0ac977be6ec2269caefb5bff27a09f7452e088c3303ba2095ebce`;
the other four file hashes remain unchanged from the initial table. The final
test changes are exact typed-empty-array equality in place of emptiness checks
and moving the blank value after its last use, as described in the implementation
handoff. They preserve the assertions and input identities already reviewed.

Read tasks/T2-implementation.md exact commands/exits, raw/T2-toolchain.txt,
raw/T2-evaluator-qualified.log (32 pass: new7/fallible17/paged7/delta1),
raw/T2-new-final.log (final seven new tests pass), raw/T2-clippy-qualified.log
(four-package all-target warnings-deny check completed), raw/T2-facade.log
(the intended root smoke test passes, 43 unrelated tests filtered) and the
previously read executable output. The report records each exit as zero.
Final new-target execution includes the three repaired coverage cases and native
overlap/source-vector additions. Reusing the other 25 consumer tests, facade and
example is justified because their code and production inputs did not change;
only the new target's mechanical clippy edits followed their executions.

Compiler capture: rustc 1.100.0-nightly,
`4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1. Native profiles
retain repository opt-level=3, assertions and overflow checks, test debug=0.
No Cargo checks were independently rerun by this reviewer.

At this observation raw/T2-wasm-qualified.log still records compilation in
progress; helper hygiene is pending. Therefore no final Task 2 verdict artifact
was written. Final source patch/candidate identity, successful wasm and helper
results and the complete final handoff must arrive before final PASS. No
additional source defect was established in this bounded recheck.
