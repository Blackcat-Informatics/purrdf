# Independent Task 5 review

VERDICT: PASS

**PASS.** The settled production repair, complete actual CLI campaign and focused
consumer qualification satisfy Task 5. This does not certify Task 6 or close
the delivery. Reviewed applicable repository laws, the sole plan, implementation,
campaign admission/results, all four new Rust targets and registrations, the
production/entailment changes, and actual settled logs/receipts. The reviewer did
not implement these changes. No builds, tests, benchmarks, timing, model actions,
Git/forge mutations or source changes occurred during review.

## Production repair and semantics

`crates/datalog/src/seminaive.rs:2185–2198` selects only the final positive anchor
when delta is exactly `Delta::all(snapshot.row_count())`. Every earlier anchor
requires an OldOnly suffix, which cannot have a row in that snapshot. This test
does not scan an index, produce a candidate or invoke a guard. Both indexed binary
(`:2239`) and hybrid (`:2279`) kernels use the same selector; positive-count zero
still produces an empty range. Partial/prefix/suffix deltas retain every anchor,
so this optimization cannot discard later matches against older facts. Existing
authored-source restoration remains in each kernel. Guard execution is after the
positive matches, so eliminated impossible variants do not remove reached guard
callbacks. Observable work changes are deliberately inside the calculus-v2
contract, not presented as a hash-preserving physical rewrite.

The new selector regression covers whole versus three partial ranges and zero
atoms. `tests/rules_runtime.rs` executes connected authored premises, independent
canonical existential witnesses, the original guarded pair and a later-round
fixture needing both old/new anchors. The pair's initial connected expansions
are 9N; its positive two-Spool neighbor is 16N plus 2N admitted head emissions,
18N total. The connected three-atom case is 3N. The Cartesian boundary now counts
n+n²; the exact-credit success and one-credit-short typed refusal remain tested.
No evaluator default or shipping dependency/feature changed.

## Actual entry point and measurement integrity

`crates/cli/examples/rules_campaign.rs:75–157` constructs the issue namespace and
term-length fixtures and independent complete expected fact/proof sets. Validation
at `:160–193` checks every fact, duplicate count, complete proof block, canonical
five authored pair premises, authored rule, exact inferred counter and channels.
The original pair requires genuinely empty fact and proof files. The two-Spool
neighbor proves the positive path and chooses the canonical b/c assignment rather
than accepting either arbitrary guard assignment. Self-tests plant omitted
premises, duplicated proofs, junk prologues/trailers, extra facts/stdout and ten
malformed/missing/duplicate/invalid allocation receipts; the immutable v3
self-test log reports PASS.

The collector calls the ordinary `rules --srl ... --explain=... INPUT OUTPUT`
production executable with unchanged defaults (`:196–250`). Linux wait4 owns the
exact child, retries EINTR and records child ru_maxrss only after successful wait;
failed children do not qualify. Elapsed time includes spawn through wait. Status,
argv, stdout/stderr, proof/output and before/after load are retained per child.
The public-dispatch observer in `rules_alloc.rs` separately runs the same
`purrdf_cli::run()` inside CountingAllocator/WholeProcessWindow and writes its
receipt only after returning. Signed retention, allocation traffic and peak
working bytes are distinct from OS RSS and production elapsed time.

Both CLI examples disable automatic test execution. The native allocation test
uses the actual public evaluator with input/compilation outside its window and
returned evaluation state inside; the separate harness-free portable runtime
target runs the same assertions natively and in Node/wasm. These instruments do
not change production CLI dispatch or substitute a private evaluation kernel.

## Captured campaign and source binding

The actual `/opt/purrdf-rules-479-campaign-20261007-v3/measurements.tsv` contains
20 cases: three original rules at 1k/10k/100k, each production and allocation
observer, plus the positive 1k pair through both. All twenty retained status files
report exit zero and all twenty validation files report complete acceptance.
The tenfold input steps satisfy the stated twentyfold elapsed/RSS and requested/
peak allocation envelopes. At 100k, production single/pair/notype elapsed times
are 5.095/4.587/4.312 seconds and OS peaks 1,325,292/1,037,184/1,293,256 KiB.
The pair remains empty; single and notype each emit N complete derivations.

The immutable factor allocation capture reproduces exact 3N credits and requested
bytes 994,911/9,762,171/101,923,859 at N=100/1k/10k, with retained and peak spans
separately recorded. The actual existing seminaive benchmark run reports nine
measured and zero failed; all nine estimates.json artifacts are present and
retain samples/intervals/outliers. No absent historical timing baseline is used
to claim a statistical speedup.

Independently rechecked all four executable SHA256 identities and four captured
new Rust source identities against their v3 receipts; each matches. Each current
new source also byte-matches its immutable capture. The current tracked source
patch, immutable-v3-source.patch and campaign source-diff.patch all have SHA256
`4efee569c646f4febb3c1675e475c2dd2be30caf360b2c6fcb9a4ea7ea69cb2a`.
Campaign parent is 3ec2dbba892bd9f9b9105a942d36faf26cfedd9c; Cargo-selected artifact
paths, compiler identity, manifests/lock and binary BLAKE3 identities are retained.
The four executable copies have read-only mode 555. Captured data belongs to
the repaired source, not the preserved v1/v2/pre-repair observations.

## Consumers, goldens and remaining acceptance

Settled logs establish 334 Datalog library tests, one actual factor allocation
test and four native portable cases; 59 shapes cases and 14 actual CLI shapes
cases; 683 entail library cases plus 23 corpus, 11 frozen OWL, 3 pack-parity,
47 reasoner and 2 ordering cases. Three intentionally ignored maintainer/upstream
generators are not counted as executed coverage. Corrected target-selection logs
supersede the retained failed attempts. Datalog and entail all-target clippy and
format checks pass. Four affected wasm library crates build, and the settled
Node run executes all four exact-selected portable cases with zero filtered or
ignored cases. This is actual shared-kernel wasm runtime coverage, not the whole
public JavaScript package gate.

The original first-party entail generator owns the 142 regenerated goldens.
Inspected T5-golden-audit.rs and its settled output: comparison against parent
HEAD permits only contract hashes, join-step counts and the explicitly bounded
generator-owned transition comment. Facts, complete witnesses, rule tallies,
stored-fact and arena-byte fields remain unchanged. Versioned native/wasm pins
move with the shared calculus contract; authored clause programs remain unchanged.
The published transition accounts for premise-free admission and narrow
connectivity/empty-suffix work changes without calling those counts latency.
No external frozen GTS vector was edited.

Host activity was captured and preserved rather than claimed absent; the campaign
is one ordered empirical growth observation under its stated conditions. OS peaks
include process startup and are not evaluator heap windows. The reviewer inspected
actual source-bound captures without rerunning measurements or consumer builds.

Task 6 still owns the settled full local gate, final completion audit, normal
hooks/commit/push, PR, hosted qualification and ghprsq integration. Broader #364
physical-budget, eager producer and continuation obligations remain in their
accepted portfolio deliveries. No blocking Task 5 correction was found.
