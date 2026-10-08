# G2: productive partial-delta factor evaluation

Status: VERIFIED SOURCE DEFECT; initial coherent source fix/two regressions written,
qualification NOT RUN. Source review and targeted coverage remain in progress.
No build, test, benchmark, forge operation or runtime qualification has run for
this remediation. Root holds admission until the active260 build lane releases.
Review concern: PR500 inline4215385143, factors.rs full_matches/evaluate.

## Observed computation

Every factor currently builds Full matches using Scan::Full. project classifies
the already materialized tuples by delta membership. The whole-rule delta check
only prevents a round with no potentially new operator; it cannot prevent the
growing recursive factor's complete old tuples from being joined again. This is
a real source defect, not merely a concern about one counter fixture.

For reach(x,z) :- reach(x,y), edge(y,z), independentGroundFact, with n chain
vertices and seeded one-edge reach/edge rows, distance grows one round at a time.
Old reach tuples grow quadratically overall; rescanning them in each of O(n)
rounds is cubic work. The ground factor never gains delta after the initial round.
The ordinary binary path already anchors the recursive relation in Delta and
uses OldOnly suffixes. Limiting factorization to all-delta rounds would remove
this regression but reintroduce irrelevant cross-factor products for partial
one-factor heads, violating accepted additivity. That broad fallback is rejected.

## Coherent design

1. Keep the existing all-delta path unchanged: compute Full once, borrow matching
   Full/New frontiers and empty Old. Preserve measured initial3N independent work
   and original9N/positive18N connected guard controls.
2. Generalize the factor matching home to a requested Full, OldOnly or New mode.
   Uniform modes call the existing indexed/hybrid kernels with uniform scans;
   New uses the existing per-positive-position Delta/OldOnly decomposition and
   delta_can_match admission. Each factor's tuple is New exactly when at least
   one of its sources is delta; decompositions are disjoint by the same anchor
   law, irrespective of physical join order. Restore authored source coordinates
   and apply the existing factor-owned negative filters to each selected tuple.
3. For a partial delta, compute New only for factors with a delta-addressable
   operator. Empty/ineligible New cannot be an anchor. If no actual productive
   factor anchor remains, return without building any Full/Old relation.
4. From actual productive anchors derive required modes. A factor needs Old if
   a later anchor exists, Full if an earlier anchor exists, and New only if it
   itself is productive. Build each required mode once against the frozen round.
   Full can supply borrowed New/Old frontiers by classification; standalone
   New-only or Old-only matching avoids its unrelated complete relation. Keep
   selected tuples owned once and frontier winners borrowed, not Cartesian.
5. Emit each conjunctive head using the existing per-head projections and global
   dual height/sum/lex threshold frontiers. Required full/old context is essential
   to exact witnesses when another factor supplies the new row; do not replace
   it with first/local-minimum existential witnesses. Stream only the necessary
   projected product, retaining admission before owned expansion and total refusal.

The existing leapfrog state currently stores only delta_position and derives
scan_for internally. Uniform OldOnly needs an explicit shared scan-selection
value (uniform Scan versus decomposition position), consumed by both source
capture and variable cursors. This is a small generalization of the existing
matching state, not a second cyclic matcher or public API. Ordinary callers keep
their existing decomposition semantics; all-full factors keep uniform Full.

No persistent cache, new support lifetime, new dependency/feature, memoized guard
or alternate evaluator is introduced. Negative coupling, scheduled full-model
guards, canonical assumptions and source slots remain unchanged.

## Required source regressions and later qualification

- Long chain plus independent ground atom: complete planned/ForcedBinary facts
  and authored proofs match at representative n32/64/128/256; n256 completes
  under unchanged default limits. Assert a deterministic quadratic work envelope
  and measured growth, not a wall-time threshold. Include zero/one-below refusal.
- One-factor projected head with large independent existential relation and a
  small partial delta: use actual evaluate_rule with frozen old/new ranges;
  counters/state remain additive rather than reverting to the binary product.
  Check both earlier/later factor anchor positions and all-old exclusion.
- Both factors productive: compare complete head/projected witnesses against
  exhaustive binary across delta ranges/insertion orders, ensuring Old-before/
  New-anchor/Full-after decomposition remains exact and each mode is reused.
- Delta addresses a partition but yields no complete factor match: no unrelated
  Full/Old rebuild or head firing. Include empty factor/negative blocking controls.
- Hybrid cyclic factor uniform OldOnly and New: exact facts/proofs against
  ForcedBinary, with authored coordinates and existing certificate intact.
- Retain height masking, u32MAX/u64 saturation, all four head positions,
  conjunctive/constant heads, negative coupling, opaque guard caller/thread/stage
  barriers, successful/refused1/4/32worker counts and actual allocation controls.

Later admitted checks: focused Datalog package/all-target clippy; affected shapes/
entailment/public host paths and exact-source original CLI/default-limit controls,
including a real recursive CLI case and relevant WASM runtime. Only affected
goldens/pins may move after owning-generator audits. New partial matching changes
join observations and possible refusal boundaries: characterize exact deltas;
facts/proofs must remain unchanged. All earlier acceptance remains binding, and
the earlier full gate is not relabeled as a full pass on changed source. Root
chooses proportionate final requalification after the focused fix settles.

## Initial source implementation

Only seminaive.rs and private seminaive/factors.rs change. JoinScan generalizes
the existing leapfrog scan selector; ordinary callers retain DeltaPosition, while
factor matching supports Uniform Full/OldOnly and per-factor DeltaPosition.
full_matches now delegates to that same factor matcher. Partial evaluation first
collects disjoint New tuples, derives actual productive factor anchors, and
builds only preceding Old context/following Full context. Full replaces its
temporary New relation instead of retaining duplicate source frames. Existing
all-delta matching and all project/frontier/product code remain unchanged.

Four Rust regressions are written: full recursive chain with independent
ground atom at32/64/128/256 (complete default-limit planned/binary facts/proofs and
quadratic counter bound), and one-factor partial heads at10/100/1000 with both
authored factor orders (additive work and exhaustive witness parity for the small
cases, avoiding a million oracle source frames for the large growth control).
The cyclic-factor regression compares complete candidate witnesses against
ForcedBinary over four insertion orders and every contiguous delta range, using
varied source heights. It also supplies an addressable but unproductive new edge
beside a thousand-row independent relation and requires fewer than twenty join
observations with no firing. A native worker regression evaluates thirty-two
recursive factor rules with productive partial rounds, requires identical exact
facts/proofs/budgets at one/four/thirty-two workers, and checks typed zero and
one-below refusals plus identical reports. Existing exhaustive delta/frontier,
masking/saturation/negative/guard tests remain in place. Direct
rustfmt parsed/formatted the source successfully; that is not Cargo compilation,
test execution, clippy, runtime qualification or completion.

## Source-settled focused qualification matrix (NOT RUN)

1. Datalog all targets/tests and strict clippy: the four new regressions above,
   all existing exhaustive candidate/frontier, negative coupling, guard barriers,
   saturation, scheduler and allocation controls. Inspect any changed counter
   assertion against actual computation; no blanket golden updates.
2. Actual release CLI: disconnected recursive transitive chain with ground
   precondition at representative growing sizes including n256, using unchanged
   default governors, full closure/proof identity and deterministic work growth;
   retain forced-binary parity on bounded native controls. Re-run accepted
   nonrecursive additive/product and cyclic controls that exercise shared scans.
3. Affected shapes/entailment/shared validate/C/Python/public WASM consumers:
   original owned generator only where changed work counters/contract identity
   require it, byte-identical facts/proofs independently audited, frozen external
   corpora unchanged. Preserve initial/current logs and identity receipts.
4. Final source hygiene/metadata as affected, independent review and root-selected
   full acceptance/hosted requalification after focused results settle.

No Cargo command, runtime, hook, generator or benchmark was invoked for this
source set while the 260 lane is active. Source changes remain exactly two Rust
paths; this report is Stage evidence. Root admission is required before step 1.

Source coverage follow-up: the existing global-height-masking fixture now runs
all twenty-one contiguous delta ranges for each of its four source orders and
four height-boundary fixtures, including MAX-1/MAX saturation. Complete candidate
entries are compared to ForcedBinary; the specific b1 winning witness assertion
is restricted to DeltaAll. The existing four negative-coupling/local-scope
fixtures now compare complete planned/binary round candidates across every
contiguous delta range and four source orders, before their retained full-run
facts/proofs comparisons. These cover blocked New and old negative context in
the changed mode paths. Direct rustfmt and diff whitespace checks passed;
qualification remains NOT RUN.

## Runtime fixture source delivery

The existing target-neutral rules_runtime harness registers a fifth case: n16
recursive reach plus adjacent edge and independent ground condition. It checks
every expected inferred pair, every three-premise authored proof (including
graph/rule identity and unique final edge), complete fact cardinality, exact
credits and typed zero/one-below JoinSteps refusals. This reaches productive
partial factor rounds through the public evaluator on native and WASM.

The original Rust rules_campaign collector now accepts
`--recursive PRE_FIX_BINARY FIXED_BINARY NEW_DIRECTORY`. The original three-arg
twenty-case route retains its child success requirement and measurement logic.
Both routes share child argv/status/stdout/stderr/load/time/wait4 RSS capture,
binary/source/tool identity receipts and the exact facts/complete proof-block
validator. Recursive mode writes n32/n256 inputs once, uses identical files and
unchanged CLI defaults for both binaries, observes any earlier unsuccessful
exit without accepting partial artifacts, and requires successful exact fixed
output. Every distance-at-least-two chain pair and its uniquely determined
recursive prefix/final edge/ground premise is validated. Before/after BLAKE3
identities detect binary mutation. Self-test includes accepted recursive proof
blocks and missing/duplicated-premise negative controls.

Tracked source scope is now exactly four Rust paths: seminaive.rs,
seminaive/factors.rs, tests/rules_runtime.rs, CLI examples/rules_campaign.rs.
Direct rustfmt and git diff --check pass. Compilation, clippy, self-test,
native/WASM runtime and paired actual CLI campaign remain NOT RUN while 260
owns the build lane. The preparation commands require collector all-target
clippy/self-test and the registered five-case WASM runtime on final source.

Collector source-binding follow-up: each recursive input and ruleset is hashed
once after generation; a single local receipt closure records attributable path
and BLAKE3 identities before and after each paired child and refuses any changed
bytes before accepting its facts/proofs. Recursive invalid-argument diagnostics
now identify that mode's actual argument contract. Rustfmt/diff checks pass;
Cargo/runtime checks remain unrun.

Fixture law correction: new recursive CLI inputs/rules/expected proof IRIs use
https://example.org/k#. The single IRI rendering home is namespace-parameterized;
the original twenty-case route retains its historical namespace and identical
fixture/proof bytes. No production matcher behavior changes in this correction.
Rustfmt/diff checks pass; runtime checks remain unrun.
