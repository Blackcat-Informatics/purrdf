<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Resumed geographic delivery material review

VERDICT: FINDINGS-OPEN for completion. The two concrete source findings below
were repaired during review and pass their bounded source recheck. Required
execution remains open. No new mathematical correctness defect was established
in the bounded source inspected; this is not approval to merge.

## Scope and authority

Reviewed the complete combined contract in fresh issue-408-resumed.md and
issue-409-resumed.md, plan.md, issue-analysis.md, current validation.md, worktree
AGENTS.md, root .baseline/.goals, clean emergency ledger, and Stage completion
and evidence-reuse rules. Worktree is the supplied
409-geo-deterministic-hierarchical-spatial, branch
paudley/409-geo-deterministic-hierarchical-spatial, comparison origin/main with
the current uncommitted recovery delta included.

Source inspection covered scratch retry/working-width changes and the existing
review disposition, formatter/error/host changes, default metric and EPSG
consumers, standalone distance/within and cell range exports, qualification
driver, architecture jobs, assembly registration and current profile claims.
It is not a new line-by-line proof of every mathematical module. Reused bounded
reviews in root-source-review-notes.md and recovery-formatter-review.md;
neither is a complete acceptance audit.

Only this report was written. No source edits, builds, tests, benchmarks,
private-input reads, process changes or forge mutations were performed. Root
is running the full public/private collections; partial progress is not a pass.

## Concrete findings and repair adjudication

### G1 — HIGH: architecture replay consumer missing — source FIXED, execution OPEN

Before repair, .github/workflows/ci.yaml:928-978 ran kNN/HNSW/hash/DEFLATE/CSV
on Arm64 without the geographic or certified numerical transcripts; lines
1034-1043 likewise omitted them on i686; lines 1059-1068 ran XSD library tests
on i586 but omitted geographic corpora. XSD library tests alone cannot prove
geographic inverse/direct, transformed geometry or cell IDs on x87.

The distinction matters because
crates/geo-kernel/examples/qualify_determinism.rs:18-25 defines only the current
native host, portable WASM and SIMD WASM. Makefile:682 delegates
make geo-determinism to that driver. On this x86-64 host it cannot establish
the additionally required Arm64/i686/i586 executions. Seven architecture
assembly compilations also cannot prove executed byte parity.

Root added three actual CI consumers during review. Rechecked the diff:
each existing architecture job now executes certified_numerical,
cells_determinism, geo_determinism and facade determinism. The corresponding
numeric path requirements are portable+neon, portable+sse2 and portable.
The certified_numerical directed-endpoint replay actually calls
assert_required_available for the numeric family, iterates available paths and
compares endpoint bits. The environment values therefore have a real consumer.
No new test-only substitute or skipped architecture was introduced.

Bounded source verdict: PASS. Root reports actionlint passed; this reviewer did
not execute it. Actual successful target execution remains required. Commands
now registered include:

```sh
cargo test --locked -p purrdf-xsd --test certified_numerical
cargo test --locked -p purrdf-geo-kernel --test cells_determinism --test geo_determinism
cargo test --locked -p purrdf-geo --test determinism
```

The i686/i586 jobs pass their actual --target values; Arm executes natively on
its existing runner. Require successful frozen digest/count receipts and the
ordinary shipping warning/profile policy. Green source syntax is not a runtime
pass. Existing i586 XSD library/environment checks remain in place.

Root subsequently found and repaired the missing hosted consumer for the
three-lane runner itself. Bounded recheck PASS: the separate geo-determinism
job uses the existing pinned Rust/WASM toolchain composite and Node setup,
then invokes make geo-determinism. It leaves the existing wasm-exec time budget
alone. The composite supplies the required target and matched wasm-bindgen
and Binaryen binaries. The actual runner executes all four registered corpora
on all three lanes, enforces the registered nonignored/unfiltered test counts,
validates frozen digests, and compares records across lanes. The revised
docs/WASM_TESTING.md correctly describes these twelve executions separately
from make wasm-test counts, and removes the obsolete native-only geometry/
index coverage claim. Root reports actionlint passed. Actual hosted/local
runtime remains pending; neither source registration nor this recheck is a
twelve-execution pass.

### G2 — LOW: unconditional tier-order claim — FIXED

Before repair docs/GEO-PROFILE.md:133-138 promised binary64, double-word and
fixed point in that order. Current geodesic.rs:443-456 sets direct_fixed for
qualified cut-locus neighbors when the precision policy admits the selected
fixed proof, bypassing the first two tiers. Exact symmetric and meridian
exclusions remain meaningful.

Root corrected the paragraph during review. It now describes the admitted
cut-locus bypass, specialized symmetric/meridian cases and ordinary ordered
tiers while preserving correct-rounding and certificate identity. Source
recheck PASS. Reuse the already passing
cut_locus_distance_finishes_with_default_work_in_cold_and_prepared_paths
regression; this prose correction does not justify a new full suite.

### G3 — completion acceptance remains OPEN

This is missing evidence, not a finding that current mathematical answers are
wrong. The current validation index correctly retains these obligations:

- Both complete current-source 500,000-row public modes and their final
  mismatch/refusal receipts. The all-mode inverse proof counter is not the
  plain-distance fixed-fallback fraction. Record the latter as measured
  acceptance when that run completes.
- All 31 original private bundles under the unchanged input population and
  acceptance. No partial private receipt was inspected or accepted here.
- Actual native/portable-WASM/SIMD-WASM transcripts from make geo-determinism,
  plus G1's additional target executions. Installed optimized WASM package
  acceptance proves a different surface and does not replace these replays.
- Actual make simd-asm results on all seven configurations. The new endpoint
  product row is still pending in docs/design/purrdf-simd.md:826. Regenerate
  its cells and all associated source-derived counts from actual results.
- Required metadata, header, translated-book, conformance and full local gate,
  then final-head hosted checks and integration assessment. A successful
  nested all-release-crate WASM build in make check can be reused; an aborted
  prefix cannot qualify its suffix. ASM is explicitly separate from check.

Do not rerun unchanged installed-package gates merely because this review ran.
Refresh source-invalidated checks and the missing obligations above.

## Requirement and evidence adjudication

| Contract | Inspected demonstration or source | Current judgment |
|---|---|---|
| Original point inverse/direct, range refusal, antipodes, rounded distance | Current kernel receipt ends 712 passed; 414-case XSD result is indexed after scratch repair; standalone Rust distance and actual solver inspected. | Focused native demonstrated; full current public corpus unverified. |
| Complete metrics/buffers/continuous geometry | Kernel receipt includes geometry and buffer coverage. recovery-complete-geometry.log completes five actual API measurements, including 10 m buffer under declared 20,000,000 work. | Native examples demonstrated; full cross-target/private acceptance unverified. The buffer receipt is not a default-work success. |
| Chinese coordinate-model fixture bounds | recovery-scratch-chinese-input.log reports 664 rows for each model, maximum upper errors 0.732856766/0.011660802/0.181866093 m below 1/0.02/0.2 m. | Public provider fixture demonstrated, without global/survey accuracy inference. |
| Cells/hierarchy/seams/mixed covers/emitted caps/ranges | Real range_min/range_max APIs; current kernel tests; six successful disk measurements before the historical covers failure. | Native focused evidence demonstrated; portable replay unverified. |
| Index/refinement/physical scales | Kernel index/scale controls; current index measurements record 83 accepted keys at both levels and 287/101 candidates. | Native focused evidence; not independently reexecuted here. |
| Default CRS84 and explicit EPSG axes | standard_tests.rs:136 exercises actual new/default/query/prepared engine on POINT(0 0)/POINT(1 0), expected 1.11319490793E5; current 95-case evaluator receipt covers it and EPSG controls. | Demonstrated on native production API. |
| Real CLI/C/Python/WASM hosts | CLI five-case pass; installed C log ends C smoke OK; installed Python log identifies installed path and five passes; repaired WASM guard and packed tarball smoke pass, plus identity suffix. | Reusable installed/entry-point evidence for unchanged paths; not a new whole WASM gate pass. |
| Physical admission, original errors, clone/drop and scratch retries | Existing bounded reviews, current 712 kernel and 95 evaluator/47 validate receipts; Destinations encoder preserves counts; generic retry drops math before growth and retries only the shape request. | Bounded source/targeted runtime demonstrated; not an independent complete allocation proof. |
| Before-birth coefficient shape and failed-attempt work | Source review and correction receipts; retry drains attempt state; cover correction retains shared width while exact results remain live. | Prior bounded findings resolved; complete numerical corpus still open. |
| Scalar/batch costs and allocations | Actual geodesic/transforms/cells/geometry measurements indexed. Allocation log has zero inverse-computation allocations at 112/224/448 bits, separately 65 proof allocations at 224/448 and zero retained bytes after destruction. | Measured windows demonstrated; complete-corpus fallback fraction due. |
| Portable/ISA runtime and actual emitted instructions | Driver and new architecture consumers wired; seven-config endpoint-product cells still pending. | Execution unverified; G1 source repair passes. |
| Documentation/generators/header/book/conformance/full gate | Tier paragraph corrected; mandatory gates remain pending in index. | G2 fixed; final qualification unverified. |
| H3 architecture without runtime claim | Accepted design retained; inspected facade adds no unsupported H3 runtime variant. | Design retained; no execution claim. |
| Publication/merge/closures/cleanup | No qualifying merge or closure supplied or observed. | NOT MET. |

## Evidence reuse

The WASM repair calls existing assertNotPoisoned before wasm-bindgen's class
check in setGeoSession and forwards to the original native method. The three
previously failing cases, healthy invalid-session control and packed smoke are
appropriate affected checks. Keep the earlier 446 passes/three failures
historically intact and compose current coverage explicitly as validation.md
already does. The reviewed native error encoder and C/XSD mappings introduce
no new demonstrated source blocker.

The failed covers command had already completed six disk and three polygon
measurements. Those results remain usable for unchanged source and recorded
conditions; the corrected geometry receipt establishes the formerly missing
completed buffer lifecycle. Never relabel the failed combined command as a
pass. The later 56–60% polygon timing change is recorded without a proven
cause; this contract asks for measured cost, not an invented speedup threshold.

No replacement numerical campaign, new branch or re-review of unchanged cleared
surfaces is warranted by these findings. Finish the authorized qualification,
reconcile actual claims and adjudicate the completed evidence. This report
remains a bounded material review, not the final completion judgment.

## Follow-up: numerical replay harness registration

Root reports that the actual three-lane runner completed the first three
corpora on all three paths, then rejected native certified_numerical because
Cargo ran zero tests where nine were registered. This report does not turn that
partial execution into a completed twelve-execution pass.

The source cause and correction are confirmed. The test file uses
purrdf_testkit::harness_main!, whose macro expands to an ordinary main function,
not libtest #[test] registrations. Without harness=false, Cargo's default
test harness does not execute those nine functions. Root added the missing
[[test]] name="certified_numerical", harness=false entry at
crates/xsd/Cargo.toml:57-59. This selects the existing test file by its normal
path and makes the registered custom runner the executable entry. Bounded
source correction PASS; its actual nine-case runtime remains pending.

Inspected all changed Rust integration-test paths against origin/main and the
current untracked test paths. No second omission was found:

| Target | Actual harness contract |
|---|---|
| purrdf-geo-kernel/cells_determinism | Explicit [[test]], harness=false at Cargo.toml:40-42. |
| purrdf-geo-kernel/geo_determinism | Explicit [[test]], harness=false at Cargo.toml:44-46. |
| purrdf-geo/determinism | Explicit [[test]], harness=false at Cargo.toml:47-49. |
| purrdf-xsd/certified_numerical | Corrected explicit registration at Cargo.toml:57-59; nine custom-runner cases. |
| purrdf-capi/abi | Ordinary #[test] functions; default libtest harness is correct. |
| purrdf-cli/geo_cli (untracked) | Five ordinary #[test] functions; default libtest harness is correct. |

The unchanged XSD exact_wasm_determinism custom-runner target is also explicitly
registered with harness=false immediately above the correction.

Rechecked numeric backend names against FloatProductBackend::name in
crates/xsd/src/math/floating/packed.rs: names are portable, sse2, avx2, avx512,
neon and simd128. CI's numeric:portable,numeric:neon;
numeric:portable,numeric:sse2; and numeric:portable exactly match the numeric
family enforced by directed_endpoint_product_replay. That replay checks
required availability and then executes all available implementations against
portable endpoint bits. Registration now makes that enforcement live in the
actual binary; earlier zero-test execution enforced none of it.

No builds, private reads or source changes were performed by this follow-up.
Required focused execution is the corrected certified_numerical target on
native, portable WASM and SIMD WASM, followed by the already registered required
architectures. Preserve and reuse the first nine qualifying replay executions
where their source remains unchanged; the manifest repair changes the XSD test
entry, not the previously executed geography/cell/facade production bodies.

## Follow-up: pooled rational lifetime assertion

Root reports corrected native certified_numerical execution reached all nine
cases, with eight passes and one failure: line 231 of
bounded_rational_ownership_and_rounding_replay expects all 32 destinations free
after copy_in(...).into_shared(), whereas 31 are free.

Source adjudication: this expectation contradicts the current original-owner
contract; no production leak or unintended detachment was established. The
copied numerator is 449 bits and holds one pooled destination; its denominator
is inline one. BigInt::into_shared at bigint.rs:1507 retains Mag::Pooled as-is.
Its comment explicitly excludes an unpriced second buffer. Rat::into_shared
delegates to the underlying Rational operands, whose into_shared calls that
same BigInt function. It cannot lawfully make this live pooled destination free.

Pooled in bigint/scratch.rs owns both its immutable SharedBuffer and the
original SharedOwned arena. Its derived Clone shares both controls. Its Drop
serializes uniqueness and decrement: only the final destination clone clears
and returns the buffer. The arena mutex is released before the original shared
owner drops. FreeBuffers contains shared limb buffers, not Pooled values, so
this ownership does not form an arena/Pooled cycle. Final arena destruction
therefore retains the existing payload-before-admission drop boundary.

Required regression correction is behavioral, not merely changing 32 to 31:

1. Retain all original rounding/normalization assertions. Assert exactly one
   held destination after constructing the shared rational (31 free of 32).
2. Measure immutable rational clone and repeated into_shared with the existing
   allocator probe; neither should allocate or consume another destination.
3. With a scratch observer still alive, drop one numeric clone and assert 31
   remain free; drop the final destination owner and assert all 32 are free.
4. In the distinct escaped-owner case, drop all worker handles first and prove
   the rational still equals its original exact value. Do not retain an extra
   scratch observer when claiming that numeric ownership alone preserves it.
5. Prove original admission bytes persist through worker-handle drop and
   nonfinal numeric drops, then become zero on the last numeric owner. Existing
   original_arena_owner_survives_handle_drop_and_last_numeric_clone at
   scratch.rs:815 already has the Admission observer for this contract; reuse
   or extend that control through into_shared and the rational wrapper, rather
   than creating an unrelated bookkeeping mechanism.

The explicitly admitted detached_with_memory/publish_detached path remains the
proper operation for releasing a worker arena while retaining independent
published limbs. Changing into_shared to detach implicitly would violate the
admitted ownership contract and introduce unpriced allocation. No such
production change is justified by this failed assertion.

This is the pre-correction source judgment only. No test/source changes or
executions were performed by the reviewer; repaired native and portable
lifecycle checks remain to be demonstrated.

## Follow-up: facade assembly attribution and completed portable replay

Bounded source verdict PASS for the geo.exact-limbs row attribution repair.
The actual facade lib.rs contains only pub use purrdf_geo_kernel::* and the
evaluator adapter reexports; it introduces no computational body. Kernel lib.rs
reexports Int from exact.rs, which reexports purrdf_xsd::integer::Int, the
BigInt alias. The facade's real relate benchmark imports Rat/topology through
purrdf_geo and exercises exact rational parsing, arithmetic and relations.
Those operations use the same XSD integer home.

The row now names purrdf-geo as the public member whose surface is represented,
while its fn/path and prose explicitly identify the reexport chain and actual
BigInt::gcd home in XSD. The assembly manifest still has exactly the existing
geo.exact-limbs site measuring that XSD symbol across all seven configurations,
with its existing scalar floor, zero-FMA and no-relaxed requirements. It does
not pretend there is an emitted facade gcd body. This is consistent with the
existing facade.multimodal-read row's explicit public-facade/actual-home split.

Kernel still has geo.relate and geo.json-write-string function rows, and both
facade and kernel benchmark files remain mapped to the same actual site IDs.
No measurement cell, gate rule, threshold, manifest duplicate or exemption was
introduced by this correction. The documentation coverage gate's demand for a
workspace-member row is met by an actual reachable implementation, rather than
by a fabricated facade symbol or a blanket exception. Actual all-configuration
measurement and regenerated cells remain required; this is source attribution
review only.

Read recovery-resumed-portable-registered.log: every one of the four targets
ran on native, portable WASM and SIMD WASM, with the final all-twelve success
line and terminal_status=0. The certified numerical target actually runs nine
tests on each path; the previously failing ownership case now passes. Frozen
elementary and endpoint digests remain 8268aff8be7630be and f1508ddac398273b.
This supersedes the earlier pending native/WASM replay judgments in this report;
it does not qualify the separate Arm/i686/i586 execution or assembly matrix.

Reviewed the actual ownership-test correction too: it retains the rounding
checks, asserts one occupied destination after sharing and cloning, keeps it
occupied after the first clone drops, restores full capacity only after the
last clone, and measures zero allocation/requested bytes through that lifecycle.
A separate copy survives dropping the only scratch handle with exact value
equality. The existing original-admission final-clone observer test remains
unchanged and passed in recovery-scratch-complete-kernel-xsd.log:788, whose XSD
suite was the recorded 414-case pass. That unchanged observer evidence and the
new portable rational-wrapper execution compose the required lifetime coverage;
there is no need to duplicate the observer merely because this test was enabled.

No builds, private reads or source edits were performed by this reviewer.
