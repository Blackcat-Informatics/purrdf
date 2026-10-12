# Geo combined delivery validation

## Current shipping authority

The user has superseded the earlier requirement to withhold geo until every
original acceptance item is complete: "geo get's pushed and merged after this
round regardless of findings, all findings go into a new issue."

Finish the pending verification round, freeze implementation, record every
unresolved finding and uncompleted acceptance item in a new visible follow-up
issue, then publish and integrate through `ghprsq`. Keep the actual hook and
required merge checks intact. Do not portray deferred findings as passed checks
or the original full technical contract as qualified. Preserve the ongoing
private process and its completed work; incomplete population at publication is
an explicit follow-up item, not grounds for another discretionary repair round.

## Technical qualification

### Current main integration

After final-round completion, the geo source was committed normally at
`b5ef3879d`; its commit hook passed the staged non-Rust ratchet and other
configured checks. The actual `origin/main` at evaluator PR #532 required a
source merge. The merge resolution preserves both evaluator evidence-bearing
EXISTS/candidate-scope/dictionary behavior and geo's original native cursor
ownership and metered opening. The governor profile is now v16: v15 remains
the evaluator's original EXISTS law, and v16 appends native-function-work.
The published v16 schedule digest is
`a62e6e30a5ad78c6c25253fd6d0757c7bee48900a6a9001e06178d87d5c66e53`.

Affected combined-tree `cargo check --locked -p purrdf-sparql-eval -p
purrdf-text -p purrdf-hnsw --lib` passes. The first library compile exposed
one missed geo-profile argument and an unused import; both were corrected.
One evaluator library test fixture and one integration fixture still used the
old boxed admitted cursor type; they were migrated to the same original
WorkspaceCursor owner. The profile digest test passes; evaluator
`evidence_ingress` 20/20, `governor_correctness` 9/9, and
`ranked_declaration` 35/35 pass. Text `phrase_composition` 5/5 and HNSW
`ranked_declaration` 17/17 pass. The frozen governor corpus passes 15 active
cases with one intentional regeneration ignore. `make metadata` passes on the
combined branch, including 154 generated claims and 37 license profiles.
These are affected integration checks, not a passing rerun of the original
full gate. The final merge commit, publication and hosted outcomes remain due.

The final-round shared witness adapter refresh passes all 23 atlas relation
tests and 73 topology tests, each terminal 0 in
`recovery-witness-final-atlas.log` and `recovery-witness-final-topology.log`.
The original default-budget query pass remains applicable. Governor projection
regeneration also passes. Its diff appends only zero native-function-work to
21 charge fixtures, changes only their 21 freeze-manifest hashes, and repins
the corpus digest. No answers, spend, metered totals or fuel traces change.
`recovery-governor-projection-check.log` records the 29-corpus freeze gate,
exact reproducible-digest test and all 15 active governor corpus tests passing,
terminal 0. Independent bounded projection review passes.
The final-round determinism launcher passes all twelve native, portable WASM
and SIMD WASM executions with identical frozen records; `make metadata` passes.
The remaining launcher is active. Its seven-configuration emitted-assembly
generator and checker pass with 112 measured sites, terminal 0 in
`recovery-helper-assembly-check.log`. The mandatory `make check` passed formatting
then failed Clippy on eight arguments in `scan_for_areas` at
`crates/geo-kernel/src/topology/relate.rs:569`, terminal 2 in
`recovery-shared-mean-make-check.log`. Build, tests, hygiene and nested all-release
WASM were not reached. Separate `make conformance` failed terminal 2 because
`docs/CONFORMANCE.md` records 4,218 Python binding cases but the measured
scoreboard has 4,223 (`recovery-shared-mean-conformance.log`). The final-round
master terminated 1, accurately aggregating both failed stages and the host
launcher failure; no whole-round PASS is claimed.

The remaining launcher now records a partial host result:
`recovery-helper-installed-c.log` terminates 0 after a fresh installed C
distribution, header/library identity check and linked C smoke. Python release
wheels and their own installed package self-check pass in
`recovery-helper-python-artifacts.log`; the separate selected host launcher
`recovery-helper-installed-python.log` terminates 1 at its internal
`purrdf.purrdf_native` import, before eight affected tests. The installed
package intentionally presents its native `rdf` module at the public `purrdf`
name, so this is a qualification-launcher failure, not a demonstrated wheel
failure. `recovery-helper-wasm-build.log` terminates 0 with 391218 SIMD opcodes
in the linked package; `recovery-helper-installed-wasm.log` terminates 0 after
four geo tests, three poison tests, typecheck and packed tarball smoke.
The combined host launcher terminates 1 because the Python selection is
unreached. The final assembly and full gates remain running/pending; this
partial host result is not labeled a whole round PASS.

### Retained pre-final-round qualification record

The paragraphs below preserve the earlier failure and repair chronology. Their
pending statements describe that historical checkpoint; the current result is
recorded above and will be completed when the running final round terminates.

At that checkpoint, the original technical completion verdict was **NOT READY**.
Both complete 500,000-row public modes
pass; the private run remains active and incomplete. The repaired
helper census, 42 core distance tests, six HNSW environment tests, five-package
strict checks and layer gate pass. The latest full gate passed the repaired
helper census and regenerated hash-domain checks, then stopped at two prose
findings. The comment and named grid spelling are corrected, with the translation
catalog regenerated; the affected prose and translated-book checks pass.
The full-gate retry in `recovery-resumed-make-check-prose-fixed.log` passed the
repaired prose boundary and finished native/doc-test collection, failing three
targets (fixture paths, planar query work, charge projections; detailed below).
It records terminal 2. The fixture repair has passed all 36 shared native-profile
tests in R2. The current exact-mean consolidation passes the 49-pair direct
kernel budget control, 73 topology tests, the ownership/refusal control and four
native determinism tests. All 714 kernel tests and all nine original query
controls passed, resolving the prior planar query-work failure. The final
same-law atlas relation mean adapter has since been routed to the shared witness
mean; its affected atlas relation controls are pending, while the planar query
computation and the private/public corpus paths are unchanged by that adapter.
Charge projection regeneration and the full-gate retry remain pending.
Conformance remains pending. Final source-bound
assembly, affected installed-host refresh, actual
architecture CI, final review, publication and integration remain due. No merge
or issue closure is claimed.

## Resumed execution

`recovery-resumed-make-check-domains-fixed.log` ends with terminal 2 at the
prose gate: the geometry dispatch comment used "branch" and the book used a
bare hazard-shaped grid label. The comment now says "arm" and the book uses
the same `h3` identifier as its existing architecture document. No scanner
exemption or semantic claim changed. `recovery-prose-catalog.log` records
successful catalog regeneration; the matching Chinese entry retains its
meaning and uses the same identifier. `recovery-prose-refresh.log` passes the
prose scan (including its self-tests) and actual translated-book build, terminal
0. The mandatory gate remains pending; the stopped gate did not reach
conformance or its nested WASM build.

Qualification cache placement was corrected before the next full-gate run.
The evaluator's runtime failures exposed that precreated native cache roots
lacked `CACHEDIR.TAG`, and sibling qualification paths canonicalized beneath
main's personal `.worktrees` exclusion. The existing output-admission rule
correctly refuses such scratch output. Cargo's matching implementation
([commit 7941be6fb, paths.rs](https://github.com/rust-lang/cargo/blob/7941be6fb416b4cd9666aef7b858dfea25587a8c/crates/cargo-util/src/paths.rs#L692))
returns without tagging an already existing directory.
`recovery-cache-containment.log` records moving the older external geo target
into the real worktree `target`, and relocating the current native caches to
`target/geo-artifacts` and `target/geo-build`. Old lexical cache paths remain
symlinks. Accurate cache markers were explicitly restored, not represented as
preexisting or Cargo-created. Git readback identifies the tracked `**/target`
rule for both scratch roots. The running private executable retained its inode
and bytes; no process was restarted. This environment repair changes no source,
test or admission law and is not a full-gate pass.
The next invocations explicitly override `CARGO_TARGET_DIR` and
`CARGO_BUILD_BUILD_DIR` after the existing contained runner with the canonical
worktree paths above. The old aliases preserve earlier artifacts but cannot
qualify testkit's exact equality between current-executable-derived scratch
paths and compile-time `CARGO_TARGET_TMPDIR`; invalidated absolute-path compiler
metadata must rebuild normally. The running private launch environment remains
unchanged.

Current scratch-corrected distance acceptance is complete:
`recovery-resumed-geodtest-distance.log` records all 500,000 rows, zero refusals
and mismatches, 161,376 fixed fallbacks (32.2752%), work 9,551,323,977 and peak
workspace 866,560 bytes. The launcher checked a zero exit before advancing to
all-mode; that active child's arguments and scope confirm the transition.
Systemd expanded the filename's `${mode}` before shell execution, so root
renamed each open log to its mode-specific path without changing or restarting
either executable. The separate terminal-status append path was overwritten
at all-mode start; distance success is evidenced by the complete final receipt
and the launcher's checked transition, not an invented terminal line.
Elapsed 1,316.350404512 seconds is not controlled throughput.
All-mode now completes in `recovery-resumed-geodtest-all.log`: 500,000 rows,
zero refusals/mismatches, all 500,000 inverse, inverse-witness and direct checks
complete with zero witness/direct mismatches. Work is 56,219,473,328 and peak
workspace 866,768 bytes. The proof-path counter is 500,000, not the scalar
distance fallback fraction. Original nominal azimuth/area disagreements remain
reported in the receipt; every conditioned metadata excess count is zero.
Elapsed 6,681.836066600 seconds is not controlled throughput. The parent exits 0;
its separate final append was renamed after exit to
`recovery-resumed-geodtest-all-status.log` and contains `terminal_status=0`.
The private collection remains incomplete.

The first portable gate completed all three geo corpora on native, portable
WASM and SIMD WASM with identical frozen records. Its final numerical target
ran zero of nine expected cases and the gate correctly failed (terminal 2).
`certified_numerical` used the shared harness but lacked its manifest's
`harness = false` declaration. That declaration is now present. The next run
executed eight cases successfully and rejected the stale expectation that
`into_shared` detaches pooled limbs. Independent review confirms the original
admitted arena must remain owned by the live pooled value. The corrected replay
requires zero-allocation sharing/cloning, destination retention until the last
clone, slot return and exact values after worker-handle destruction. Both failed
receipts remain unchanged. `recovery-resumed-portable-registered.log` now passes
all twelve actual target executions (terminal 0), including all nine numerical
cases on each path and identical frozen records. Metadata regeneration passes
in `recovery-resumed-metadata.log`, terminal 0: generated-source checks and all
154 documented claims agree, and 37 license profiles are regenerated. The
combined portable/metadata launcher also terminates 0.

The seven-configuration assembly gate initially refused missing facade coverage
before compilation (`recovery-resumed-assembly-coverage-failed.log`, terminal 2).
The existing `geo.exact-limbs` row now truthfully attributes the public facade's
`Int` re-export to its actual, unchanged XSD gcd measurement; kernel rows and both
bench mappings remain covered. Independent review passes. No new computation,
gate exception or manual measured-cell edit was introduced. The renewed full
assembly generator completed all seven configuration builds but failed its
measurements, terminal 2 in `recovery-resumed-assembly-write.log`: both WASM
configurations lack the specified `results.json-escape` and
`slice.prefix-json-string` call-site symbols. Five native configurations
completed analysis. No complete assembly pass or generated cells are claimed;
the normal check suffix was not reached. Read-only emitted-code diagnosis is
recorded in `reviews/recovery-wasm-assembly-findings.md`. Both computations are
present: WASM retains the results writer's same closure in `try_escape_body`
and inlines the slice wrapper into `emit_jsonld_context`, which still calls
the same `push_string` law at all three sites. The manifest now keeps the five
native selectors and measures these exact WASM paths separately. All scanner
floors/mnemonics, zero-FMA and no-relaxed requirements remain unchanged; measured
document cells were not edited. Independent source recheck passes. The final
normal-cache assembly run remains pending while the evaluator owns the build slot.

The repaired final generator and checker now both pass: 112 sites on all seven
configurations, terminal 0 in `recovery-resumed-assembly-callsite-write.log`
and `recovery-resumed-assembly-callsite-check.log`. The actual source-derived
cells are regenerated; `recovery-resumed-assembly-report.json` is the final
checker report. The normal cache was reused without a forced rebuild or changed
acceptance. The combined metadata/ASM launcher terminates 0.

After the ASM process exited, root removed the two duplicate kernel benchmark
files, their manifest targets and bench-map rows, as independently reviewed in
`reviews/recovery-benchmark-duplication.md`. The original facade benchmarks
continue to call the identical kernel bodies; the four distinct kernel targets
remain. This removes copied benchmark implementations without changing shipping
computation, corpus inputs, thresholds or assembly sites. Final helper/strict
checks and regenerated source-bound assembly reports remain required.

The first post-removal metadata invocation stopped in the existing tracked-file
reader because the two deleted benchmark paths were still in the index:
`recovery-resumed-benchmark-metadata.log`, terminal 2. Root staged exactly those
two intended deletions, preserving all other staging state, then resumed into
the separate `recovery-resumed-benchmark-metadata-staged.log`. No gate bypass or
reader fallback was introduced; the failed receipt is retained.
The resumed metadata run passes, terminal 0, including all 154 claims and 37
generated license profiles. Mandatory `make check`, followed only on success
by `make conformance`, is now active in the contained lane. Its nested
`make wasm` remains part of the required successful suffix; no partial full
gate result is claimed.

The first full gate stopped at its first `cargo fmt --all --check`: one
indentation line in `cells/cover.rs`, terminal 2 in
`recovery-resumed-make-check.log`. Root applied exactly rustfmt's indentation
change and restarted the mandatory gate into
`recovery-resumed-make-check-format-fixed.log`. This required retry follows an
actual failure, not workflow advancement; the failed full run is not
qualification. No computation changed, so corpus, allocation and emitted-code
observations retain their behavioral applicability. The raw source identity
changed, so refresh the final source-bound ASM report after source checks settle.

The format-fixed full gate passed strict workspace compilation and the preceding
hygiene checks, then failed at the shared-helper census with 42 findings,
terminal 2. Conformance and the nested WASM build were not reached. The original
failure remains in `recovery-resumed-make-check-format-fixed.log`; the extracted
findings are in `recovery-helper-findings.txt`.

The reviewed repair routes core distance environment/control through the
existing XSD implementation, removes the two duplicate core modules, shares
Python/WASM response conversion, uses the existing constructor macro, names the
existing OGC vocabulary constants, restores the already sanctioned frozen
big-endian fingerprint framing at its moved home, and documents exact typed or
ownership adapters. `reviews/recovery-helper-triage.md` explains each disposition.
`recovery-helper-repaired.log` now passes: 81 enforced jobs, 22 reasoned variants,
115 exact distinct groups, 2,202 files, terminal 0. This is helper acceptance,
not full-gate acceptance. Affected distance/HNSW runtime checks and strict
production-package compilation are in progress in `recovery-helper-focused.log`.
The first focused compile refused an unnecessary qualification in the new test;
the corrected retry passes all 42 core distance tests and all six HNSW native
environment/worker tests. That retry then correctly refused the Python
constructor macro because Python had no direct lexical dependency. The existing
workspace `purrdf-lex` edge is now declared in Python's manifest and layers,
with only that first-party edge added to the offline-generated lockfile.
`recovery-helper-focused-python-edge-fixed.log` passes strict all-target checks
for core, HNSW, geo-kernel, Python and WASM and the complete 230-edge layer gate,
terminal 0. Both earlier focused logs remain failed invocations. No new external
package, feature or implementation was introduced.
`recovery-resumed-helper-metadata.log` subsequently passes the required
regeneration with all 37 license profiles, terminal 0. The next full gate,
`recovery-resumed-make-check-helpers-fixed.log`, passes strict workspace checks
and the repaired helper census, then fails at the generated hash-domain table,
terminal 2. The existing `check-hash-domains.py --write-readme` generator adds
the missing geographic registrations and updates the moved index-source home;
its subsequent ordinary checker passes all 121 unique, prefix-free domains and
three declared exceptions. No hash bytes or computation changed. Conformance
and nested `make wasm` remain unreached until a whole mandatory gate succeeds.

The consolidated native environment now also refuses enabled hardware traps
before probing; the core production entry has a regression for this behavior.
Current source-bound ASM, installed host acceptance and mandatory gates must
be refreshed for these changes. Long numerical corpus runs use the unchanged
XSD environment and unchanged arithmetic; they remain active on their recorded
artifacts and are not restarted or relabeled as a new source build.

On resumption, process and user-scope inspection found neither retained corpus
process alive. The prior distance log stops at 450,000 rows without a terminal
receipt; the prior private log remains partial. Neither driver implements a
checkpoint/resume contract, so those receipts cannot be extended into a complete
run. Their logs are preserved. Replacement whole-population runs use the existing
scratch-corrected release binaries, original inputs and unchanged acceptance:
`recovery-resumed-geodtest-distance.log` followed by the all-mode log, and the
mode-600 `private-result-resumed-complete.log` in the qualification directory.
Both correctness processes may overlap one bounded build, as authorized. No
timing comparison is inferred from these overlapping runs. Earlier paragraphs
describing the old processes as live are historical, superseded by this record.

Live issue refresh confirms both contracts remain open; stagectl finds no PR for
the current delivery branch. No recovery closure or merge has occurred.

The resumed independent material review is
`reviews/recovery-resumed-material-review.md`. Its two source findings are fixed:
the profile now describes the cut-locus precision bypass, and CI executes the
geographic/cell/certified-numerical replay targets on Arm64, i686 and i586 plus
the complete native/portable-WASM/SIMD-WASM comparison in its own job. The WASM
testing guide separates those twelve executions from existing selected tests.
Independent source rereview passes; actual architecture execution remains due.
`recovery-resumed-ci-lint.log` records actionlint success, terminal 0.
`recovery-resumed-book-zh.log` records the actual translated-book build success,
terminal 0. No book input changed in these CI/profile/testing-guide repairs.

The standalone resumed header check failed because the default Stage Cargo
wrapper ignored cargo-c's private artifact directory; its log is
`recovery-resumed-header.log`. This was an invocation error, not header drift.
The existing successful installed-C-bundle gate already compares the generated,
installed and committed header bytes (`build-capi-bundle.py`, after private
output verification) before compiling the actual installed smoke caller.
Those C/header inputs are unchanged, so its passing header evidence is reused;
another C rebuild is unnecessary. The failed standalone check is not relabeled.

Root owns the complete #408/#409 implementation in the #409 worktree. Signed
checkpoint c5637dbd6de92a87ed9dd6df31232391710a3c06 contains main and original
Gitee ancestry integration. Current uncommitted changes restore CLI/numeric cases,
repair IEEE formatting under flush controls, require successful cover benchmarks,
correct numeric-home and CLI documentation, exercise geographic handles from the
compiled C smoke caller and regenerate license projections.
The current distance delta selects the existing cut-locus fixed precision before
spending work on binary64/double-word attempts, excluding exact symmetric and
meridian cases. Focused default-work regressions pass; complete acceptance remains
pending. The subsequent integer-scratch correction replaces maximum-policy
allocation with attempted precision and actual coefficient-order admission.
It covers exact preparation, covering, solver loops and generic numerical callers,
preserves inline coefficient storage, and charges failed attempts before retry.
Bounded source review passes. Focused current-source regressions pass: 12 kernel
and 19 XSD cases in recovery-scratch-actual-shape.log, terminal 0, strict warnings.
They cover actual adaptive order with live outer arrays, cumulative work,
inline endpoints, original owner lifetimes and refusal before allocation.
The complete kernel/XSD collection terminated 101: XSD 414 PASS; kernel 703 PASS
and nine covering/physical-scale failures. All nine report live destinations
preventing replacement after exact cover preparation admitted only its first
scalar's width. The shared covering working-width correction passes all nine
failed cases plus an allocation-refusal control, including bounded/unbounded
value, work and peak parity for physical scales. The complete kernel refresh
passes all 712 tests in recovery-scratch-kernel-cover-fixed.log, terminal 0.
XSD's 414-case pass remains current. The strict private driver rebuild passes
in recovery-private-actual-shape-build.log, terminal 0; its complete private
runtime used the original acquisition and contained mode-600 output, with its
partial/terminated disposition below.
No repaired private completion is claimed; other qualification remains pending.
No recovery issue is closed.

The private run completed one bundle and reported one bundle failure at
continuous original-edge materialization, class `domain`. Root deliberately
terminated the obsolete caller after implementing the independently demonstrated
zone-partition correction; terminal 241 reports child signal 15. This was an
incomplete collection, not full private acceptance. The specific domain subtype
is not established. Source inspection independently found that the driver's
midpoint-selected single zone cannot contain an original edge crossing a zone
wall. The driver now partitions the complete original parameter interval at
exact zone/hemisphere walls and materializes every piece in its declared frame,
with separate original-edge and piece counts and unchanged native domain/error
certificates. Four public boundary/refusal/redaction controls and a static
domain-subtype whitelist are added. Bounded source review passes. The first
control compilation failed on a temporary target-identity borrow; its corrected
assertion retains the owned identity. All four controls then passed under strict
release compilation in recovery-private-zone-controls-fixed.log, terminal 0.
The strict release driver build passes in recovery-private-zone-driver-build.log,
terminal 0; the delta was absent from the terminated binary.
The corrected driver must process all 31 bundles;
the partial run does not replace or reduce that requirement.
The final guard-lifetime source refresh passes all four public driver controls
and the strict release executable build in recovery-private-final-guard-build.log,
terminal 0. The complete corrected private runtime is now running, with the
original acquisition and policy limits, through the contained 16 GiB scope.
Its private-result-zone-complete.log receipt is mode 600; no completion is claimed.
The corrected run has completed one bundle. Its owned systemd scope was frozen
while the installed C bundle qualified and was verified running after automatic
thaw on successful command exit. The runner uses wall time only for its receipt, not a deadline; actual
elapsed time therefore includes scheduling pauses and is not a controlled
performance comparison. Completed native work is retained in the same process.

## Recorded results and applicability

| Check | Result and limits |
|---|---|
| Current complete kernel | recovery-scratch-kernel-cover-fixed.log: 712 PASS, terminal 0, including the shared cover working-width correction and bounded/unbounded scale value/work/peak parity. |
| Current scratch correction | recovery-scratch-actual-shape.log: 12 kernel + 19 XSD PASS, terminal 0. Bounded independent source review PASS in root-source-review-notes.md. Complete acceptance remains pending. |
| Complete scratch correction collection | recovery-scratch-complete-kernel-xsd.log: terminal 101, kernel 703 PASS/9 FAIL, XSD 414 PASS. Every kernel failure is NumericalScratch::Retained in shared cover/scale arithmetic. recovery-cover-shared-working-width.log then failed compilation on an unnecessary qualification. Corrected source passes all 10 selected cover/scale cases in recovery-cover-shared-working-width-fixed.log, terminal 0, and all 712 kernel tests in recovery-scratch-kernel-cover-fixed.log, terminal 0. XSD source is unchanged after its 414-case pass. |
| Complete kernel before scratch correction | recovery-integrated-kernel.log: 707 PASS, terminal 0, on integrated main/Gitee source with formatter and cut-locus precision-selection delta. The subsequent scratch admission/retry correction requires refresh. |
| Native geo consumers | recovery-scratch-consumer-refresh.log: scratch correction passes 95 evaluator + 47 validate cases; runtime_status=0. The combined command exits1 because strict lint separately fails. Subsequent production change explicitly drops the preflight mutex after its last read and before constructing the same refusal; numerical computations and consumer paths are unchanged. recovery-scratch-guard-release.log passes all 7 scratch ownership controls after that change; its strict phase fails on the CLI empty-output assertion. |
| Six-package all-target strict lint | recovery-scratch-strict-cli.log: PASS, terminal 0, xsd/kernel/geo/evaluator/validate/CLI all-target strict lint. Earlier failures in recovery-scratch-consumer-refresh.log and recovery-scratch-guard-release.log were repaired by dropping the mutex guard after its last use and expressing the CLI empty-output assertion through the existing stdout helper. No suppression. |
| XSD unit suite before scratch correction | recovery-formatter-and-wide-fixed.log: 413 PASS, terminal 0, including binades/extremes, DAZ/FTZ original subnormal and signed-zero formatting, dyadic division and full-width rounding oracle cases. New destination-shape preflight and regression require refresh. |
| Numerical delta review | recovery-formatter-review.md: bounded independent PASS, not complete geo qualification. |
| License projections | recovery-license-projections.log: 37 profiles regenerated; recovery-license-self-test.log: generator controls PASS. |
| Five restored CLI cases | recovery-scratch-strict-cli.log: current source 5 PASS, terminal 0. The corrected donor case pins the bounded host's exact 512-bit operational cause on stderr, exit 1 and empty stdout. The original failed expectation remains recorded in recovery-restored-cli.log. |
| Complete geographic performance coverage | Current release-derived fat-LTO benchmark build passes (recovery-complete-performance-build.log, terminal 0). The cells/index target completes all seven measurements, terminal 0: batches 1/4/16/256/4096 allocate zero; both physical index levels return the same 83 exact matches, with 287 and 101 candidates respectively. The covers target completed all six disk distributions and three polygon metrics, then terminated 101 when the 10 m point buffer exhausted default work before producing output. The repaired fixture declares and reports 20,000,000 work without changing production defaults. recovery-complete-geometry-clippy.log and recovery-complete-geometry-build.log both pass, and recovery-complete-geometry.log completes all five geometry measurements, terminal 0: polygon length/perimeter/area, warmed point-to-edge distance and complete buffer materialization. The 10 m buffer completes at 251.0 ms median, 1,792,091 work and 67,064 retained output bytes; allocation and timed calls release each output receipt after destruction. The geodesic target passes all 32 measurements, terminal 0, including every 1/4/16/256/4096 distance/inverse/direct batch and available portable/SSE2/AVX2/AVX512 prepared paths. The transforms target completes all 63 measurements with zero failures, terminal 0; all 45 warmed batch probes allocate zero. Both inverse projections complete through 4096: Gauss-Krüger median 46.48 s and UTM median 68.71 s. GEO-PROFILE.md reports actual scalar and large-batch costs. The combined performance shell exits 1 because of the earlier covers failure; no complete performance verdict yet. New benchmark source review passes after correcting explicit buffer output-receipt release. Strict bench compilation initially failed on inferred function-pointer lifetimes; explicit reference types pass in recovery-complete-performance-clippy-fixed.log. These final benchmark additions pass strict compilation and actual runtime. The repeated polygon timings are 56–60% slower than the earlier artifact despite unchanged metric source; both measurements remain recorded and GEO-PROFILE.md reports the later costs without assigning an unproven cause. recovery-current-geodesic-allocations.log and recovery-current-geodesic-allocations-adversarial.log both pass, terminal 0, after a strict release-derived example build. All25 normal windows and all3 inverse precision windows allocate zero. Published112-bit proofs also allocate zero; published224/448-bit proofs each perform65 allocations while publishing the proof, requesting3096/3984 bytes with zero net retained bytes after destruction. The current full-corpus fallback metric remains due. Formatting and scoped diff checks pass. |
| Complete-cover functional smoke | recovery-complete-cover-smoke.log: all 6 cases PASS, terminal 0; complete 1/30/400 km covers at levels 16/17. Not timing qualification. |
| Whole diff whitespace | PASS after correcting notices at the generator. |
| Complete Chinese fixture | recovery-scratch-chinese-input.log: current scratch-corrected release executable passes all 664 pinned rows for each of GCJ, BD09LL and BaiduMercatorAnalytic, all original gates PASS, terminal 0. Input SHA-256 matches public-inputs-verification.md. recovery-scratch-chinese.log built the executable successfully but its invocation omitted stdin and failed parsing empty input before any row; that failure does not qualify the fixture. recovery-complete-chinese.log remains the earlier-artifact result. |
| Private runtime | RUNNING, NOT QUALIFIED. recovery-private-final-guard-build.log passes all four public controls and strict release build, terminal 0. The current whole-population process writes mode-600 private-result-resumed-complete.log in the contained qualification directory; its latest aggregate receipt completes four of 31 bundles. The earlier private-result-zone-complete.log is partial and its process was absent on resumption, as recorded above. The original complete collection failed all 31 bundles with integer_scratch; its subsequent run completed one bundle and failed one at continuous edge materialization, class domain, before deliberate termination (child signal 15, parent terminal 241). Earlier failures and partial receipts are not relabeled. |
| Earlier-artifact complete GeodTest | recovery-complete-geodtest.log: all 500,000 rows PASS in inverse/direct/witness mode on portable backend, terminal 0. No distance, inverse-witness or direct mismatch/refusal; all conditioned metadata excess counts are zero. Original nominal metadata disagreements and conjugate attribution remain reported. Current scratch-corrected source still requires both complete modes. |
| Earlier-artifact plain-distance GeodTest | recovery-complete-geodtest-distance-fixed.log: 500,000/500,000 complete, zero refusals/mismatches, terminal 0, portable backend and unchanged default 262,144 work cap. 161,376 fixed fallbacks (32.2752%), total work 9,551,323,977, peak workspace 866,560 bytes, elapsed 1,240.951130339 seconds. Artifact/source/compiler identity: recovery-distance-fixed-source-identity.txt. Predecessor recovery-complete-geodtest-distance.log remains terminal 1 with one refusal at row 472,840 and total work 10,354,360,942; its failed result is not relabeled. Timing is a workload receipt, not a controlled benchmark comparison. |
| Cut-locus precision-selection repair | recovery-cut-locus-distance.log: 11 geodesic tests PASS, terminal 0. Failing corpus row plus neighboring cases complete under unchanged default work in prepared/proof, cold, plain and reused calls; exact antipodal/equatorial/meridian cases preserve answers and work across precision limits. Includes forced-backend and batch parity. Not full corpus or performance acceptance. |
| C smoke geographic caller | recovery-installed-c-bundle.log: actual installed-bundle build, provenance/license audit, C linkage and complete smoke execution PASS, terminal 0. Release build took 4m17s. The executed geo checks pass session success, semantic response/error, operational no-response refusal, index lifetime after session free and selected versus undeclared query axes. Distribution is target/dist/purrdf-capi-3.0.1-x86_64-unknown-linux-gnu.tar.gz. |
| Installed Python distribution and geographic callers | recovery-installed-python-artifacts.log: source distribution, wheel built from that source distribution, license/package/publisher checks and actual installed purrdf + purrdf-rdflib metadata/import/graph checks PASS, terminal 0. Candidate is target/python-release-candidate/attempt.NvaaLP. recovery-installed-python-geo.log: all five existing geographic tests PASS against that installed wheel under isolated Python, terminal 0; covers session success and semantic refusal, operational exception, retained index, query/governed/entailment/prepared profile forwarding and both UPDATE WHERE doors. Test tooling uses versions from bindings/python/uv.lock. The private process was frozen during qualification and automatically resumed afterward. |
| Optimized WASM package and geographic poison precedence | recovery-optimized-wasm-package.log: strict release build, wasm-bindgen, wasm-opt, wasm-link, actual SIMD-opcode check and TypeScript pass; complete runtime collection has 446 PASS/3 FAIL, gate terminal 2. All three failures are the new setGeoSession class check running before the native poison gate. The package entry now calls the existing assertNotPoisoned first; other geo methods already enter the native gate before decoding arguments. recovery-wasm-geo-poison-fixed.log: all three failed poisoning cases, all four geographic cases (including healthy invalid-session refusal), TypeScript and actual packed-tarball smoke PASS, terminal 0, using the unchanged compiled WASM. The earlier whole gate remains failed. Under Stage validation reuse policy, the unchanged compiled module and 446 unaffected passing cases remain applicable; the only shipped repair wraps setGeoSession with the existing poison guard, and all three failed cases plus healthy/invalid-session controls were refreshed. Makefile wasm-pkg-test has no remaining suffix after packed-tarball smoke and the identity fixture. This is composed current package acceptance, not a newly successful whole-gate invocation. Its unreached Dataset identity-ABI fixture subsequently passes in recovery-wasm-dataset-identity.log, terminal 0: fresh fixture build, actual Node bigint Dataset/exchange identity, wrapping/exhaustion refusal and TypeScript contract. The long asynchronous correctness test retained its process across scheduling pauses; its elapsed duration is not a performance measurement. |

The original regression and first integer-zero fix both failed: generated code
used a floating comparison under DAZ. The final implementation uses the existing
scientific formatter, with XPath normalizing its exact zero spelling. Original
failed logs remain failures.

Bounded independent review accepts the final cut-locus selection: exact
opposite-latitude and longitude-180 branches retain their earlier cheap proofs;
lower precision limits retain their earlier tier order and clamped fallback.
For that precision-selection delta alone, the complete predecessor inverse/direct numerical evidence remains applicable:
those solver bodies are unchanged and the reordered initial-precision helper
returns the same values. Its successful endpoint residuals were at most three
micrometres, so they do not exercise the changed cut-locus bypass (near-pole
longitude seams have same-sign latitudes). Evidence reuse therefore retains the
earlier all-mode numerical results; its artifact identity, work and timing remain
explicitly earlier-artifact data. Current 707 kernel tests include the connected
residual/backend controls; final required gates remain due.
The later scratch ownership and retry correction changes the resource paths in
these callers. That earlier reuse judgment does not qualify the current source;
the affected numerical and resource acceptance must be refreshed. The complete
Chinese fixture has now passed on that corrected source; both complete GeodTest
modes remain due.

## Blocking acceptance and next actions

- Current scratch-corrected public acceptance is complete in both 500,000-row
  modes, with zero refusals/mismatches. The distance mode records 32.2752% fixed
  fallbacks; the all-mode proof counter is not that scalar fallback metric.
  Both receipts and their exact applicability are recorded above. Do not rerun
  either population solely because a later workflow stage begins.
- Four public driver controls and the corrected strict release build pass.
  Complete repaired-source private qualification: all 31 bundles, 5,580 features, 6,424 parts and 1,691,348
  vertices. The user permits read-only original archive/manifest access as the
  single external-input exception. Archive digest matches configuration. No data
  is copied or printed; outputs/scratch stay inside the repository. The live
  process's latest aggregate receipt has four completed bundles; that is
  incomplete acceptance.
- Prepared/batch/backend, allocation and 1/30/400 km measurements are recorded
  above with their successful component receipts and original failed combined
  invocation. Retain those applicability limits and reported costs; do not
  invent a successful whole invocation or an unsupported speedup threshold.
- Native/portable/SIMD determinism passes all twelve actual executions. The
  corrected seven-configuration ASM generator and checker both pass for all
  112 sites, with source-generated documentation and report. Actual
  Arm64/i686/i586 runtime remains required in
  the registered hosted jobs. The two redundant kernel benchmark targets
  identified in `reviews/recovery-benchmark-duplication.md` are removed,
  retaining the facade targets and four distinct kernel targets. A final
  source-bound seven-configuration refresh remains due after the shared-helper
  changes, through `recovery-helper-assembly.sh`. The earlier successful reports
  retain their recorded identities; deleting duplicate benchmark targets does
  not change library computation or corpus results.
- Installed C/Python and optimized WASM package acceptance above remains
  applicable only to unchanged production paths. The shared-helper changes
  require fresh affected artifacts and execution via
  `recovery-helper-host-refresh.sh`, scoped by the independently reviewed
  `reviews/recovery-helper-host-applicability.md`. Its C/header, Python
  sdist-to-wheel and installed eight-case, and optimized WASM seven-case/type/
  packed-package refreshes have not yet run.
  The `make wasm` all-member build is still required; package acceptance does
  not cover release crates outside the package dependency closure. The actual
  `make check` recipe ends by invoking `make wasm`, so its successful nested
  execution with `CI=1` will satisfy that requirement without a duplicate run.
  An earlier failure that prevents reaching that suffix does not qualify it. New
  benchmark source remains subject to strict compilation and final native gates.
- Metadata and translated-book generation pass. Earlier C-bundle evidence
  establishes header parity on its recorded artifact; the affected refresh
  above rebuilds and verifies the current installed artifact. Complete
  conformance and mandatory final gates,
  final review, signed hooked publication, hosted checks, ghprsq integration,
  main/issue closure and normal mirror readback.

The contained runner is .worktrees/purrdf-geo-qualification/run: managed compiler,
eight build jobs, four test threads, 16 GiB/no-swap scope and repository-contained
Cargo/build/target/temp storage. One heavy lane serves both recovery owners.
Python/Node caches and UV interpreter installs are also configured under that
qualification directory. Private C package/header builds must unset
CARGO_BUILD_BUILD_DIR so their explicit contained target directory owns the
artifacts those existing tools capture.

#467/PR530 was previously merged with verified acceptance repairs; its obsolete
products were deleted and earn no recovery closure. The reconciled #408 donor is
deleted. This current index replaces obsolete chronology without archiving it.

# Native collection findings and fixture correction

The completed `recovery-resumed-make-check-prose-fixed.log` has three failed native
targets: `native_profile` (31 passed, five fixture permission failures) and
`query_rewrite_e2e` (one passed, eight cumulative geometry-work refusals), plus
`governor_corpus` (13 passed, two failed, one regeneration test ignored). The
governor failures are stale `.charges` projections missing the appended
`native-function-work` row, whose measured amount is zero; metered consumption,
answers and fuel sweeps passed. Regeneration through the existing measured
corpus entry point and review of its exact diff remain due. All remaining native
and documentation tests finished; the gate records `terminal_status=2` and its
launcher session 69161 exited 1. The nested WASM and separate conformance suffix
were not reached. No full-gate pass is claimed. The build slot was handed to R2
for its prepared repair qualification before another geo build.

After native compilation completed, root corrected only the native-profile
fixture and workflow source: owned repository temporary paths replace the
hardcoded test parent, canonical path and sibling protection remain enforced,
and the configuration-absence subprocess reads from the filesystem root without
writing there. The obsolete CI fixture provisioning is removed; observed setup
start ordering is independently enforced. Source review passes in
`reviews/recovery-native-profile-fixture-review.md`; rebuilt runtime acceptance
is still due. The collected native-profile binary and its failure receipt precede
these edits. The gate launcher itself has not changed.

R2 executed the shared fixture repair and disproved the initial source review:
32 native-profile tests passed, four failed because `completed_case` called
`completed_pair`, which still routed already-completed receipt fields through
the `/opt` execution-location check. The review was withdrawn and corrected.
The current repair keeps shared field validation in one body, retains `/opt`
admission before actual profiling IO, and checks completed receipts against
their caller-admitted exact case/lane/warmth/previous and unchanged proof.
Hosted reclamation still derives its fixed `/opt` campaign and checks canonical
case/cache boundaries before deletion. The existing `/tmp` execution refusal
remains tested, and its generic receipt fields are explicitly valid. Owned
fixture paths are not assumed to be outside `/opt`, so hosted target placement
remains valid. `CONTRIBUTING.md` now describes the actual fixture requirements.
The exact follow-up is `recovery-native-profile-receipt-validation.patch`;
current bounded source review passes, but rebuilt runtime acceptance is due.

The planar query diagnosis is in
`reviews/recovery-planar-query-work-diagnosis.md`. No work limit or expected
answer was changed. Pair-level work measurement and a production correction
remain due. The private original-input process continues unchanged, last
observed at four completed bundles without an aggregate failure.

After the full gate exited, root applied only the temporary
`recovery-planar-pair-probe.patch` to record each matrix's consumed work and the
first refusal's requested charge/backtrace on the unchanged public query. It is
not a production correction and must be reversed before final qualification or
commit. No diagnostic build was launched while R2 owns the lane.


## Measured planar refusal and bounded correction

The temporary pair probe finished with the original failure and was reversed
before the production edit. Its receipt is `recovery-planar-pair-probe.log`:
point/point matrices consumed 323 work; point/square matrices 17,960–18,674;
square/square 54,377. Pair 21 started at 222,776 and refused an additional 16
at cumulative 262,143. The refusing computation was exact comparison in
`fragments_inner`. The original 262,144 query budget and all expected answers
remain unchanged. These are diagnostic work counts, not timing qualification.

The correction confines deduplication to exact undirected XY witness supports,
retaining every original endpoint seed and the original geometries for parity.
For a singleton point, its actual location fixes its interior row/column;
area witnesses run first and traversal may stop only after the remaining cells
actually reach conservative dimension ceilings. Degenerate shapes retain the
full witness fallback. Independent source review found no material correctness
or ownership defect; savings remained unproven at review time.

The first focused compilation rejected elided independent endpoint lifetimes in
the mutable support array. That receipt remains failed in
`recovery-topology-settlement-kernel.log`. The helper now names their shared
borrow lifetime. The follow-up kernel/query collection is running under the
original resource cap; no runtime result is claimed yet.

The shared native-profile follow-up has now passed all 36 cases in the R2
worktree, receipt `recovery-profile-trace-focused.log`, for the exact shared
fixture and receipt-validation patches. Root still owes its mandatory complete
gate; the earlier failing full-gate invocation has not been promoted to a pass.

The lifetime-corrected source passed all 713 kernel tests, but the original nine
query controls still had eight work-budget failures. Both terminal receipts are
retained as `recovery-topology-settlement-lifetime-{kernel,query}.log`; the kernel
pass is not a query acceptance claim.

The next diagnostics used only a temporary print in the existing native matrix
parity test, then phase probes in the same generic matrix body. Receipts
`recovery-topology-remaining-work.log` and `recovery-topology-phase-work.log`
show the point-inside-square computation consuming 1,318 through collection,
4,122 through noding, 7,845 through area scanning, 10,213 through event
classification and 12,667 through its first sufficient fragment. The identical
square pair consumed 24,899. These are actual work quantities, not throughput.
All diagnostic prints and the temporary trait probe were removed.

The next bounded correction visits fragments before events for singleton
operands, leaving events as the complete fallback. It also removes outside
scan-line bands: linear edges cannot extend beyond their endpoints' ordinate
range, so these bands can establish only the already-proven E/E area. Distinct
original event ordinates still delimit every interior band. The source retains
all original noding seeds, geometry parity and exact admission.

The existing owned-allocation test now covers both singleton operand orders,
release after success, storage refusal before allocation and a refusal at the
last positive work charge after real witness work. The new complete-matrix
regressions also execute both admitted operand orders. Qualification is running
in `recovery-topology-witness-order-{kernel,query}.log` with unchanged original
query inputs, expected answers and 262,144 work limit.

The first witness-order compilation refused the now-unused `NodingWork::append`
method. Its only caller had built the removed sentinel bands. The obsolete
trait method and both implementations were removed; shared noding arithmetic
was unchanged. `recovery-topology-witness-order-clean-kernel.log` passes all 713
cases, including early-completion ownership and late refusal. The associated
query receipt still has eight unchanged-budget failures, terminal 101. This
correction is therefore not qualified.

Root returned the bounded build lane to R2 at that terminal checkpoint. The next
source correction uses the singleton dimension law directly: one exact location
fixes its interior row/column, removal of one point cannot lower a positive
fragment/face dimension, and only the other operand requires noding. The
positive-dimensional classifier explicitly assigns the singleton's exterior;
zero-dimensional event witnesses still locate both operands. This prevents a
sample equal to the singleton from being misclassified as a positive-dimensional
interior intersection. All of the other operand's endpoint and isolated-point
seeds remain. Current source review and runtime acceptance are pending.

A direct kernel regression now measures all 49 ordered pairs of the original
query fixture's seven spatial entries in one unchanged default work context,
asserting explicit complete nine-cell matrices. It must pass before another
dependent query rebuild; it supplements rather than replaces the original nine
query controls. The existing line-midpoint and polygon-area sample coincidence
cases and degenerate/transposed admitted controls cover this changed proof.

The direct 49-pair control stopped late in the final bare-point row at pair 3/2,
consumed 262,134, under the unchanged 262,144 default. Receipt
`recovery-singleton-direct-budget.log` records terminal 101; downstream controls
in that launcher were not run. Its source identity is retained separately.
The build lane returned to R2 without rebuilding dependent query consumers.

The next source correction removes remaining redundant exact midpoint work:
fragment ordinates now use the same local `mean` as scan-line ordinates. Admitted
equality permits an admitted original copy; a zero operand omits the unnecessary
addition; unequal nonzero operands retain the original add-and-half body. The
obsolete midpoint trait methods were removed. Every surviving comparison, copy
and multiply still uses the original exact admission/ownership home. Shared
segment and numerical implementations are unchanged, preserving public/private
corpus applicability. Independent source review accepts this algebra and its
admission law; sufficient savings remain unverified.

Before executing that correction, one-home review identified the same exact
mean in the segment midpoint and two arrangement helpers. The arithmetic now
lives once in `topology/segment.rs::mean_inner`; segment coordinates, planar
matrix witnesses, planar arrangement averages and geographic arc cut witnesses
call that home through their existing admission adapters. The earlier statement
that the shared segment body is unchanged is superseded. Polygon metrics and
atlas relation coverage must be refreshed. Read-only callgraph review confirms
that the public geodesic corpus and the ongoing private multiline driver do not
reach any of these means; their recorded artifact applicability is retained.
The direct 49-pair control and subsequent topology, ownership and determinism
controls are executing in `recovery-singleton-mean.sh`, with identities captured
for all seven changed source files.

`recovery-singleton-mean-{budget,topology,ownership,determinism}.log` all record
terminal 0 (1, 73, 1 and 4 controls respectively). The following consumer
checkpoint, `recovery-shared-mean-{kernel,query}.log`, records 714/714 and 9/9
passing tests, terminal 0. The cumulative default remains 262144; no fixture,
budget or acceptance predicate was weakened. The bounded lane returned to R2.

Final source-only consolidation routes `atlas/relate.rs::average` through the
same scalar witness mean. It is a typed admission adapter with the same exact
law, called only by geographic relation scan faces. Its focused refresh is
pending. The scope is the touched topology/atlas witness computation, not a
workspace-wide replacement of operation subdivision, interval/MathContext or
retained-owner arithmetic contracts. Independent callgraph review again finds
no path from the public corpus or sanitized private driver to this adapter.

The final adapter form removes both standalone `average` wrapper bodies:
topology arrangement and atlas relation import `mean_admitted as average` from
the segment home. That home constructs the existing ExactAdmission once;
RelateWork and atlas cut midpoint call it as well. This is the same arithmetic
and ownership path, with one context/progress adapter instead of repeated thin
forwarders. The next focused receipt covers both atlas relations and topology.
