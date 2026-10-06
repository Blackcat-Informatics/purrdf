# Issue #403: Stack-safe Clone/Eq/Hash/Debug for deeply nested RdfTerm

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement

## Body

## Problem
`RdfTerm::Triple(Box<RdfTriple>)` derives Clone, PartialEq/Eq, Hash and Debug. These recurse once per nesting level, so very deep quoted-triple terms (100k levels) can overflow the stack.

## Proposed solution
Replace the derives with manual impls built on the existing `purrdf_lex::walk` work-list / dismantle primitives.
- Hash must feed the hasher the same sequence of events as the derived impl.
- Debug output must be byte-identical to the derived output.
- Signatures, the trait set and auto-traits are unchanged.

## Acceptance
- Deep-structure tests at 100k levels pass natively and on wasm32.
- Tests confirm the Hash event sequence and Debug bytes match the derived impls.
- A benchmark shows no regression on shallow terms.
- `cargo semver-checks` passes for purrdf-core.


## Comments (3)

### paudley — 2026-10-05T04:53:04Z

Make owned RDF term trait operations stack safe

The public exhaustive RdfTerm enum retains all four variants, including Triple(Box<RdfTriple>). Replace only its recursive Clone, PartialEq/Eq, Hash and Debug implementations. RdfTriple and enclosing model types continue to call those same trait signatures. No representation or Drop change is permitted, because consumers construct and destructure the published Box representation by value.

Baseline and goals
- Read .baseline, .goals and the applicable AGENTS contract. Preserve unrelated work and source ownership; process references belong only in GitHub notes. All implementation, tests and measurements are Rust. Reuse purrdf_lex::walk and the core's existing bottom-up fold. No dependencies, features, toolchain changes, generic WASM semantics or emergency-ledger deferral.
- The user authorizes a smaller targeted bug workflow instead of a full three-stage cycle. This plan still maps all acceptance, uses normal hooks and source-bound validation, obtains actual CodeRabbit review, and integrates only with ghprsq under the root's exclusive lock.

Completeness contract
| Requirement | Task and proof |
| --- | --- |
| Stack-safe Clone | T1 native 100k-level failing-first test; T2 existing bottom-up fold with exact triple location preservation |
| Stack-safe PartialEq/Eq | T1 native independently built 100k-level equal/mismatch fixtures; T2 heap work list and shallow comparisons |
| Stack-safe Hash with identical events | T1 native 100k-level failing-first test and independent shallow compiler-derived oracle with a recording hasher; T2 discriminant and all fields in the derived order, including source location |
| Stack-safe Debug with identical bytes | T1 native 100k-level compact Debug test and both normal/alternate shallow compiler-derived byte oracles; T2 existing Tok/write_debug implementation |
| Public signatures, trait set, auto-traits and exhaustive Box representation unchanged | T1 construction/destructuring and compile-time trait/auto-trait assertions; T2 no new public model API, representation or Drop; T3 cargo semver-checks against captured integration base |
| WASM depth acceptance under the user's newer test ownership instruction | T2 one genuinely target-specific shadow-stack-floor boundary check using an existing runner/home, coordinated with the owner of the focused WASM census; native Rust owns semantic and 100k-depth oracles |
| No shallow performance regression | T1/T3 native first-party harness measurements on the same leaf and shallow triple fixtures before and after the implementation, preserving exact outputs; investigate any measured regression before qualification |
| Full deliverable through PR, review and merge | T3 normal full required gate, per-task signed commits/pushes, issue and PR receipts, actual final-head CodeRabbit/CI, source-bound squash notes and root-locked ghprsq audit/closure/owned cleanup |

Enhancement audit
- Transformation adopted: treat each trait as an explicit tree evaluation with its own ordered observations. Clone is a bottom-up reconstruction; Eq checks paired nodes; Hash is the compiler-derived event stream; Debug is the shared structural token stream. This directly eliminates machine-stack depth without inventing a new trait platform.
- Reuse adopted: the existing WorkList, try_fold_nested and Tok/write_debug homes; test-only Dismantle/Nested guards give deep fixtures safe teardown without changing the published model.
- Utility adopted: source locations, every literal option/direction, both subject and object nesting, unequal neighbors, public destructuring and auto-trait checks are part of the oracle.
- Robustness adopted: independent compiler-derived shallow model and ordered hasher events rather than comparing only final digests; formatting-error propagation; native deep tests on a stated small stack; source-bound measurements and gates.
- Representation/Drop replacement declined: it would break the published Box construction/destructuring contract and exceeds these four trait fixes. Explicit deep-fixture teardown preserves the existing API.
- A new generic tree/observation platform and a new WASM runner or semantic corpus are declined: existing first-party homes implement the required mechanics, and native Rust owns generic behavior under the user's instruction.

T0 isolation
Recheck all local branches/worktrees and open PR/issue ownership immediately before creation. Create paudley/403-stack-safe-model in .worktrees/403-stack-safe-model from current origin/main, preserving all sibling work. No root-main edits.

T1 failing-first native qualification and measurement
Add native model-trait tests and a test-only independent derived model/recording hasher/fixture guard. Register a first-party native shallow benchmark target using the same support fixtures. Run each of the four 100k-depth operations against the unchanged derives in a separate normal cargo-test child process, because Rust stack overflow aborts the process; capture each real failure and distinguish the operation from fixture construction or explicit teardown. Run the independent shallow oracles and capture the pre-change benchmark. The intentional failing-first tests remain uncommitted until the completed fix passes; normal commit hooks are never bypassed.

T2 complete trait fix
Implement the four manual traits using the existing homes and preserve every field, discriminant event and normal/alternate Debug byte. Preserve shallow fast paths where measurements justify them. Add only the coordinated unique WASM shadow-stack boundary case in an existing home. Run focused native qualification and the existing focused WASM command appropriate to that case. Obtain independent read-only source/wiring review from the parent/available peer, fix findings, then commit with normal hooks, push and update the issue with source-bound evidence.

T3 qualification and publication
Run normal formatting, warning-denied Clippy, Rustdoc, all purrdf-core native tests, semver-checks against the captured main base, shallow before/after measurements and full make check. Read complete exit results. Resolve source defects; record any actual external qualification blocker explicitly. Check the notice-only emergency ledger, mechanical deferral scans, clean owned source and the complete acceptance mapping. Commit/push any qualification correction with normal hooks and post receipts. Create a non-draft PR with Closes #403, this plan and actual validation.

Read the actual CodeRabbit review and all inline/thread pages for the final head, address actionable findings, synchronize only already merged main normally in this assigned worktree, and qualify the resulting source. Draft and post final source-bound squash notes. Request the root's serialized merge lock only after genuinely green final checks/review. Integrate exclusively with /home/paudley/stage/root/bin/ghprsq. Verify the signed result, exact head/result tree equality, local/remote audit refs and notes, PR merge/issue closure, then remove only the owned worktree and local branch and release the lock. Resume existing issue412 final qualification only after the separate CI repair actually merges.


### paudley — 2026-10-05T05:32:44Z

Native source checkpoint: 2a420b39389fc61efcfa050d29c2c6f362587a70 (signed and pushed; normal commit hooks passed).

The four owned RdfTerm traits now keep arbitrary quoted-triple depth in the existing heap walk/fold/Debug homes. The exhaustive enum and Triple(Box<RdfTriple>) construction/destructuring remain unchanged; there is no Drop or representation change. Shallow native fixtures compare exact ordered Hash events and compact/alternate Debug bytes with an independent compiler-derived model, including a depth-three tree beyond the bounded fast path and equal spellings in distinct variants. Test-only iterative owners dismantle the deep fixtures safely.

Eight native cases pass with zero failures/ignores, including four independent 100,000-level operations on a stated 256 KiB stack and formatter refusal. Each operation previously aborted with a real stack overflow in a separate ordinary cargo-test process against the unchanged derives; fixture construction/teardown control passed. All-target package Clippy with -D warnings, formatting, diff checks and normal hooks pass.

This is a checkpoint, not completed issue qualification. Shallow performance acceptance remains NOT MET: earlier separate-run comparisons reported regressions, including leaf timings and the depth-two paths. The depth-two Clone/Debug correction is implemented, and the same native benchmark now measures the independent derives adjacent to each manual trait in one binary to resolve the remaining comparison. Those new measurements, semver validation, full required gates, the approved single WASM shadow-stack-floor selection, final CI and review are still pending. No general trait semantic corpus is added to the WASM lane.

The separate CI scheduling repair has actually merged; only ordinary assigned-worktree main synchronization follows. No toolchain or host investigation/change, hook bypass, dependency, feature, non-Rust growth, new runner or deferred source requirement.


### paudley — 2026-10-05T10:03:06Z

The issue remains in progress. Native stack-safety and formatter compatibility checks pass on the captured source below, but shallow performance acceptance is NOT MET. Final required gates and a completion PR remain outstanding.

## Latest qualified source checkpoint — new artifact UNMEASURED

The separately approved bounded correction restores the original parent `RdfTerm` derives for `PartialEq` and `Hash` and removes only their two manual public implementations. The existing manual recursive `RdfTriple` shims, outlined roots, literal 2→1→0/heap bodies, internal single-dispatch Hash kernel and ordered fields, scalar/Debug writer, Clone, tests, fixture and benchmark law remain byte-exact. Independent source review confirmed both exact changes and all eleven other captured source/input paths unchanged. Qualified working-source identities are model `dffb46d66332f36cfbf57fcc0da41fc0fca4b5a0` and traits `08083681401d93013a89225d375ed274e2bc9ba0`; the signed branch head `8d27f702d97bd592182f0cbb84197b0166aea363` contains captured main `33bc4f2f12e9c5dd874a88c3904d40217c763ab4`, freshly read back unchanged. No public representation, Drop, field/signature or trait/auto-trait change occurs.

On these exact source bytes, ordinary release native Core/Lex validation passed **1,944 tests / zero failures / one existing ignored test / 60 groups**, including all eight model-trait cases, the four separate 100,000-level operations, independent actual ordered Hash events/direct struct and wrapped-term formatters, asymmetric/depth-three neighbors, full scalar option/refusal matrices and doctests. Warnings-denied all-target Clippy and formatting passed. Fresh ordinary Core rustdoc JSON explicitly contains both automatically derived parent implementations at model.rs:185; the approved explicit **minor** API comparison passed **196 checks / 58 skips**, no semver update required. This release-type accounting remains distinct from the earlier 223/31 receipts.

The normal benchmark no-run passed. Its actual Cargo-returned executable was archived byte-identically as candidate `8cd8b5f8c62fd8b747af30934529ddb184a8d73a`, matching the same-main actual-public original `4d890ffc425667f1f9bc4b1df96dee6c8efcd6c0` and unchanged fixture/harness. Native --list passed and printed all 64 registered IDs, including all 14 diagnostic cases, with **zero benchmark measurements**. Direct inspection discloses generated shipping Hash caller expansion to 740 bytes/frame0x78 versus original305/frame0x48, and equality caller822/frame0x68 versus original306/frame0x58. Triple branches still enter their outlined manual roots and existing zero-boundary heap walks. These are compiled structural facts, not causal explanations or performance approval; the earlier caller expansion is not hidden by source-identical leaf laws.

This new artifact is **UNMEASURED**. The exact previous diagnostic sequencer is preserved after only receipt-prefix and candidate-path substitutions, with the same fourteen cases, both orders, 28 pairs, 56 halves, normal100samples/10000resamples/.95confidence/1%noise law. All own readers are terminal and the source/artifacts are frozen pending a separate coordinated timing grant. All **seven** latest failed pair outcomes and every earlier cohort below remain blocking, including unchanged Clone/Debug failures. No timing retry, full32, completion PR or acceptance is claimed. Final required performance, source-bound full gate/portability/WASM floor, signed commits/push, actual PR review/CI and serialized ghprsq merge remain outstanding.


## Latest completed actual-public diagnostic — performance NOT MET

The sole authorized Hash-public diagnostic has now completed, exit 0: **28 adjacent ordered pairs / 56 native measurements / 100 finite samples each**, with the unchanged 14-case sequence in both orders, 10,000 bootstrap resamples, 95% confidence and 1% noise threshold. Candidate `a4dddcfbe87754dc164fc2b1d1b31483d7df0d2a` and actual-public original `4d890ffc425667f1f9bc4b1df96dee6c8efcd6c0` share captured main `33bc4f2f12e9c5dd874a88c3904d40217c763ab4`, the exact same fixture and normal benchmark harness. All 15 source/manifest/executable fingerprints match before and after. Independent final readback verified all 5,600 saved samples, 56 reports and 28 second-half comparisons. No successful measurement was repeated.

The candidate has **12 improved / 9 neutral / 7 REGRESSED** pair outcomes. Every regression remains blocking, including blank equality, literal Clone, flat-triple Debug, IRI Debug, typed-literal Clone, flat-triple Hash and nested Hash. A favorable opposite-order result does not erase a failure. Full32, final performance acceptance, PR creation and completion remain unqualified. All earlier failed cohorts below remain preserved; none is reclassified as noise, hardware or compiler defects.

Only each pair's second half compares the immediately preceding adjacent artifact. Original-first prints candidate/original; current-first prints original/candidate, so only its verbal verdict is reversed. The literal intervals are retained unchanged:

| Case | Order | Literal second-half change interval | Candidate verdict |
|---|---|---|---|
| owned_model/blank/eq | original-first | +9.9604% +81.2667% +96.2031% | REGRESSED |
| owned_model/blank/eq | current-first | +114.4654% +120.8123% +125.2831% | IMPROVED |
| owned_model/blank/debug | current-first | +6.5450% +9.7367% +11.0213% | IMPROVED |
| owned_model/blank/debug | original-first | -18.4015% -3.3778% +13.2211% | NEUTRAL |
| owned_model/literal/clone | original-first | -6.9347% -2.9006% -1.4365% | IMPROVED |
| owned_model/literal/clone | current-first | -34.2785% -30.9272% -20.4114% | REGRESSED |
| owned_model/literal/debug | current-first | +82.2977% +92.3128% +118.9838% | IMPROVED |
| owned_model/literal/debug | original-first | -55.0687% -46.4115% -37.0275% | IMPROVED |
| owned_model/simple/clone | original-first | -25.3608% -4.1958% -2.0511% | IMPROVED |
| owned_model/simple/clone | current-first | -6.8950% +17.7572% +25.4954% | NEUTRAL |
| owned_model/triple/debug | current-first | -25.5377% -8.8640% -6.4961% | REGRESSED |
| owned_model/triple/debug | original-first | -20.0582% -8.0417% +18.1323% | NEUTRAL |
| owned_model/iri/clone | original-first | -7.7109% -2.5345% +5.0184% | NEUTRAL |
| owned_model/iri/clone | current-first | -20.2811% -13.0017% +2.3889% | NEUTRAL |
| owned_model/iri/eq | current-first | +9.7721% +11.3926% +14.3936% | IMPROVED |
| owned_model/iri/eq | original-first | -20.0815% -0.3306% +14.6155% | NEUTRAL |
| owned_model/iri/hash | original-first | -54.1789% -48.0642% -45.6779% | IMPROVED |
| owned_model/iri/hash | current-first | +1.3719% +9.5914% +11.0095% | IMPROVED |
| owned_model/iri/debug | current-first | -47.2104% -16.7044% -14.4336% | REGRESSED |
| owned_model/iri/debug | original-first | +0.6832% +2.7569% +4.2805% | NEUTRAL |
| owned_model/typed/clone | original-first | +18.8408% +21.3316% +22.8673% | REGRESSED |
| owned_model/typed/clone | current-first | -4.9202% +3.8666% +7.9109% | NEUTRAL |
| owned_model/triple/hash | current-first | -15.4778% -8.9900% -5.7802% | REGRESSED |
| owned_model/triple/hash | original-first | -7.1103% -6.9977% -6.8739% | IMPROVED |
| owned_model/nested/hash | original-first | -13.4800% -12.9461% -11.6598% | IMPROVED |
| owned_model/nested/hash | current-first | -30.4371% -26.7907% -19.9501% | REGRESSED |
| owned_model/nested/debug | current-first | +12.6423% +14.2329% +16.5047% | IMPROVED |
| owned_model/nested/debug | original-first | -1.8083% +0.5698% +2.3656% | NEUTRAL |

The issue remains open. This table records the failed pre-restoration candidate; its next separately approved source correction and qualification are described above. No further timing retry or full performance acceptance is claimed. Final gates, current landed-main integration, full performance acceptance and actual review/CI/merge remain outstanding. The later pre-run artifact and qualification paragraphs below are historical receipts; their then-unmeasured diagnostic is superseded by this completed table.


The owned representation is unchanged: exhaustive RdfTerm variants, Triple(Box<RdfTriple>), every field, construction/destructuring, Drop and public trait signatures are preserved. The four recursive RdfTriple traits use the existing shared heap walk/fold and a fixed direct 2→1→0 prefix where applicable. Independent native controls check actual compiler-derived ordered Hash events, exact compact/alternate/options Debug bytes, direct struct and wrapped term composition, formatter refusal status/prefixes, asymmetric and branching shallow neighbors, depth-three neighbors, and independent 100,000-level Clone/equality/Hash/Debug operations. Fixture teardown is explicit and stack-safe without altering the public Drop contract.

The native formatter oracle first proved 26 depth-three Debug mismatches. The correction scripts all literal/location fields and Option/direction variants in one shared writer, forwarding standard str/u32/u64 scalar leaves. The public closed Tok shape and generic write_debug signature/composite behavior remain unchanged. Opaque normal/lower/upper option calibration requires an exact unique match, without private bit interpretation; ambiguous or unmatched states refuse visibly. Standard primitive formatting handles escaping, signs, prefixes, width and precision. Streaming padding replacement before the existing indentation adapter preserves NUL, newline and Unicode fills. Independent derive and character-boundary refusal matrices remain the proof, including dynamic zero versus absent width/precision.

Current-main qualification was captured after ordinary synchronization to main d7c35b159eedbe93752f429546d59cc289ad1602 at owned head 2d6b2786f804d2dafb831059c6a63b8e2b14796a. The ordinary complete core+lex suite passed 1,915 tests, zero failures and one existing ignored test across 59 result groups, including documentation. Formatting and all-target warnings-denied Clippy passed. Explicit source-bound core and Lex rustdoc-JSON semver comparisons each passed 223 checks with 31 skips. The guarded same-worktree original-baseline selection restored and verified every preserved candidate source/diff after all actual readers were terminal; no stale artifact was admitted as current API proof.

The preserved captured-d7 coordinated diagnostic uses actual public shipping traits and identical fixtures/law, with matching captured-d7 original executable 3eec797c439dd81ff4b2a3922394e1ad6dfa9ae2 and candidate fc5e0951b8762f393aad4ac7029bc4f813ee0f79. All fourteen cases ran in BOTH fixed orders: twenty-eight pairs and fifty-six successful normal 100-sample measurements. All fifty-six complete estimates contain 100 samples, confidence 0.95 and 10,000 resamples; the exact order matches and all fifteen frozen source/test/manifest/document/executable identities match before and after. No routine retry, settings change or orchestration failure occurred in this window.

This diagnostic is NOT MET: twelve improved, eleven neutral and FIVE regressed pair outcomes:

* Blank-node Debug, original-first: 45.0241→51.1766 ns; printed candidate/original interval [+5.7870%, +13.6651%, +38.4618%].
* Simple-literal Clone, original-first: 13.9897→26.8385 ns; candidate/original [+88.5602%, +91.8446%, +97.2796%].
* IRI equality, current-first: candidate 7.14166 ns, original 5.86752 ns; printed original/candidate [-25.0585%, -17.8409%, -8.8911%].
* IRI Debug, current-first: candidate 185.059 ns, original 165.752 ns; original/candidate [-16.5962%, -10.4332%, -5.1364%].
* Typed-literal Clone, current-first: candidate 42.9983 ns, original 32.1531 ns; original/candidate [-34.7720%, -25.2224%, -21.3313%].

Every opposite-order outcome and full sample remains retained. Reverse-direction intervals stay literal rather than numerically inverted. Favorable outcomes do not cancel failed ones. The full thirty-two-pair acceptance cohort has not been launched on this candidate.

All five selected leaf operations use the authored payload laws rather than the deep scalar writer. Source-identical parent derives do not establish identical compiled caller boundaries: bounded inspection of the two known immutable task executables proves the original public equality benchmark calls an outlined parent equality, while the candidate expands its parent variant/string dispatch inside the loop. Original caller address/size/frame are 0x309e0/0x132/0x58; candidate 0x34b70/0x336/0x68. This is concrete structural evidence, not a measured causal attribution and not a Clone/Debug cause.

A separately reviewed bounded correction now removes only RdfTerm's PartialEq derive and gives it one outlined exact paired-variant dispatch in the existing trait home. It delegates unchanged String and literal equality and the existing bounded/heap RdfTriple equality; mismatched variants refuse. The private paired-Triple prefix, local-field comparator and heap equality source bodies remain byte-identical, and their leaf/mismatch fallbacks cannot restart paired-Triple descent. Other traits, scalar writer, formatter scripts, fixtures, benchmark cases and measurement law are unchanged. Ordinary synchronization to landed main 33bc4f2f12e9c5dd874a88c3904d40217c763ab4 is complete at signed merge 8d27f702d97bd592182f0cbb84197b0166aea363 through normal hooks; its one additive manifest-registration conflict retains both branches' test/benchmark targets. Current model source is bfe687d18181eb41533703fd6c5c7952029a195c and trait source 8eb1c764641b768c0ca7c42076791e3f72342bfd. Ordinary complete core+Lex native validation passed 1,944 tests, zero failures and one existing ignored test across 60 groups. Final all-target warnings-denied Clippy and workspace formatting passed. The initial intentional derived-Hash/manual-equality pairing lint is preserved; one narrowly reasoned expectation retains the derived Hash law with the independently checked variant/payload equality. Source-bound core semver passed 223 checks with 31 skips, including the fresh explicit equality implementation and integrated main API; Lex source/Cargo inputs and its prior 223/31 API proof are exact. A second bounded source review found no equality, stack-bound or public-shape gap. Captured-d7 evidence remains historical, distinct from these new-source checks.

The new immutable benchmark candidate is 4b9ff82feb19eba2f35f1fc77a6170a13ba5512b, built normally without execution. Known-artifact inspection proves the intended public equality boundary: its timed caller is 0x34af0, 308 bytes with frame 0x58, and calls the outlined exact parent through a relocation resolving to 0x47c00. This is compiled structural evidence, not measured causality or performance. The matching current-main ORIGINAL public executable is 4d890ffc425667f1f9bc4b1df96dee6c8efcd6c0, using the same captured main, benchmark, fixtures, manifests and harness. Only the two already approved owned source roots were temporarily selected from that captured main under the persistent/unconditional restoration guard; every other path stayed exact. A task-marker edit refusal occurred before any Cargo launch, so the candidate was restored and completely verified first. After correcting only that marker writer, the original build launched for the first time and passed. The guard then restored the exact candidate only after the actual reader was terminal and verified all thirteen source hashes, whole tracked/scalar diffs and signed HEAD. No active marker remains. All eight native model cases and formatting passed again after restoration. All fifteen source/fixture/manifest/document/executable identities stayed exact through the now-complete coordinated diagnostic. The single authorized sequence completed all fourteen cases in BOTH fixed orders, twenty-eight pairs and fifty-six successful routine measurements; all reports have 100 samples, confidence 0.95 and 10,000 resamples. Its actual shell handle terminated successfully. No routine retry, orchestration failure, source/artifact change or benchmark-setting change occurred in this window.

The captured-33bc parent-equality diagnostic is **NOT MET: nineteen improved, five neutral and FOUR regressed candidate pair outcomes**. Original executable 4d890ffc425667f1f9bc4b1df96dee6c8efcd6c0 and candidate 4b9ff82feb19eba2f35f1fc77a6170a13ba5512b remain exact. Every second-half result is recorded below in actual pair order. For original-first, the printed comparison is candidate/original; for current-first it is original/candidate and only the verbal verdict is reversed. Printed intervals remain literal. Neutral retains the harness's no-change or within-noise result; no threshold changes are introduced.

| Exact case | Order | Printed direction | Printed interval (low / estimate / high) | Second-half printed verdict | Candidate verdict |
|---|---|---|---|---|---|
| `owned_model/blank/eq` | original-first | candidate/original | -9.2918% -7.5984% -5.4719% | Performance has improved. | **IMPROVED** |
| `owned_model/blank/eq` | current-first | original/candidate | -35.9554% -30.0507% -17.3886% | Performance has improved. | **REGRESSED** |
| `owned_model/blank/debug` | current-first | original/candidate | +25.9673% +50.3845% +102.7607% | Performance has regressed. | **IMPROVED** |
| `owned_model/blank/debug` | original-first | candidate/original | -7.2817% -3.1417% -1.3882% | Performance has improved. | **IMPROVED** |
| `owned_model/literal/clone` | original-first | candidate/original | +2.6132% +6.0298% +10.6455% | Performance has regressed. | **REGRESSED** |
| `owned_model/literal/clone` | current-first | original/candidate | +12.6210% +16.6132% +21.3511% | Performance has regressed. | **IMPROVED** |
| `owned_model/literal/debug` | current-first | original/candidate | -4.3935% -1.8021% +0.8892% | No change in performance detected. | **NEUTRAL** |
| `owned_model/literal/debug` | original-first | candidate/original | -8.2819% -5.2478% -2.4746% | Performance has improved. | **IMPROVED** |
| `owned_model/simple/clone` | original-first | candidate/original | -5.8353% -4.6551% -3.9956% | Performance has improved. | **IMPROVED** |
| `owned_model/simple/clone` | current-first | original/candidate | +7.0151% +7.5657% +8.3802% | Performance has regressed. | **IMPROVED** |
| `owned_model/triple/debug` | current-first | original/candidate | -0.1269% +0.0718% +0.3919% | No change in performance detected. | **NEUTRAL** |
| `owned_model/triple/debug` | original-first | candidate/original | +0.9158% +2.3756% +3.4865% | Change within noise threshold. | **NEUTRAL** |
| `owned_model/iri/clone` | original-first | candidate/original | +3.1020% +3.3521% +3.5328% | Performance has regressed. | **REGRESSED** |
| `owned_model/iri/clone` | current-first | original/candidate | -0.4206% -0.1938% +0.2279% | No change in performance detected. | **NEUTRAL** |
| `owned_model/iri/eq` | current-first | original/candidate | +5.9719% +6.2027% +6.3837% | Performance has regressed. | **IMPROVED** |
| `owned_model/iri/eq` | original-first | candidate/original | -6.3241% -5.8690% -5.5586% | Performance has improved. | **IMPROVED** |
| `owned_model/iri/hash` | original-first | candidate/original | -4.6326% -4.2762% -3.3513% | Performance has improved. | **IMPROVED** |
| `owned_model/iri/hash` | current-first | original/candidate | +2.4687% +2.8669% +3.2066% | Performance has regressed. | **IMPROVED** |
| `owned_model/iri/debug` | current-first | original/candidate | +10.9819% +13.2271% +14.5363% | Performance has regressed. | **IMPROVED** |
| `owned_model/iri/debug` | original-first | candidate/original | -8.9518% -7.1181% -2.4887% | Performance has improved. | **IMPROVED** |
| `owned_model/typed/clone` | original-first | candidate/original | -19.6583% -19.3633% -19.1996% | Performance has improved. | **IMPROVED** |
| `owned_model/typed/clone` | current-first | original/candidate | +37.5806% +38.1595% +39.2648% | Performance has regressed. | **IMPROVED** |
| `owned_model/triple/hash` | current-first | original/candidate | +13.1842% +16.6311% +19.1189% | Performance has regressed. | **IMPROVED** |
| `owned_model/triple/hash` | original-first | candidate/original | +5.7921% +12.6459% +14.9198% | Performance has regressed. | **REGRESSED** |
| `owned_model/nested/hash` | original-first | candidate/original | -24.4351% -23.7558% -21.9280% | Performance has improved. | **IMPROVED** |
| `owned_model/nested/hash` | current-first | original/candidate | +109.3825% +128.6281% +146.4482% | Performance has regressed. | **IMPROVED** |
| `owned_model/nested/debug` | current-first | original/candidate | -5.7957% -3.2214% +5.2202% | No change in performance detected. | **NEUTRAL** |
| `owned_model/nested/debug` | original-first | candidate/original | -41.9916% -37.9982% -30.9181% | Performance has improved. | **IMPROVED** |

The four required regressions are blank-node equality current-first, literal Clone original-first, IRI Clone original-first, and flat-triple Hash original-first. Every favorable opposite-order outcome remains visible and does not cancel its failed neighbor. The intended compiled equality boundary is proven structurally, but these measurements keep acceptance NOT MET and do not establish a cause for any remaining failure. No full thirty-two-pair run or unchanged-candidate retry has been launched. The complete local table includes both artifact identities, all sample/report hashes, original/candidate medians and all fifteen before/after identities. Previous failed cohorts remain preserved rather than overwritten.

A separately reviewed general Hash correction now removes repeated bounded child classification: one compiler-discriminant/payload home serves public, bounded and heap callers, delegates existing String/literal payload Hash, and returns only a borrowed Triple. Root structs emit no enum tag; every wrapped/nested term emits exactly one, fields remain subject→predicate→object→location, and internal calls cannot reenter public Hash or restart the fixed depth. The term Hash derive and now-stale pairing expectation were removed; other traits, scripts, fields, fixtures and law remain unchanged. Full native core+Lex validation passed 1,944 tests, with zero failures and one existing ignored test, exact-event/deep/scalar/refusal oracles included, and warning-free Clippy/format/API checks passed. The first Clippy finding (collapsible_if) was fixed in code and its failed receipt retained.

The first unmeasured single-dispatch executable 4a4fabc1b4d39732cd2c385f569d9a4930517362 exposed a new concrete compiled public Hash caller expansion: 720 bytes with frame 0x78 versus the original 305 bytes with frame 0x48. It has been retained unmeasured. A separately approved correction changes only the public term Hash inline annotation to inline(never); every function body is identical to that intermediate source. Final model source is e04d1623ee1ad7269b2211a00c5aa09aa623d324 and traits source e606d5b65e1c565de5fc7695b024dec34610acb7; the other twelve captured source/input paths are exact. On these final bytes, the complete ordinary core+Lex native suite again passed 1,944 tests, with zero failures and one existing ignored test across 60 groups, warning-free all-target Clippy and formatting passed, ordinary core rustdoc JSON contains the actual new Hash implementation with Inline(Never), and explicit semver again passed 223 checks with 31 skips. Lex shipping/Cargo source and prior API proof remain exact. A bounded independent source review found no event/order/stack/API gap.

The final immutable candidate is a4dddcfbe87754dc164fc2b1d1b31483d7df0d2a, normally built without benchmark execution, matching the existing actual-public original 4d890ffc425667f1f9bc4b1df96dee6c8efcd6c0. Fresh remote main is still captured 33bc4f2f. Actual shipping benchmark assembly proves caller 0x340b0 (305 bytes, frame 0x48) calling public Hash at 0x33370 through call 0x3411d, whose Triple arm routes to root 0x326f0 (3,055 bytes). The duplicate internal preclassification and expanded caller are structurally absent. This is **not a measured cause or performance result**. All own readers are actual terminal, all source/artifacts are frozen, and the unchanged 14-case diagnostic in both orders, with 28 pairs, 56 measurements and 100 samples per measurement is prepared but UNMEASURED, awaiting a separate coordinated grant. All four prior failed pairs and every earlier cohort remain required and preserved; no full32 or unchanged-head retry has run.

Earlier complete failing cohorts remain preserved: the pre-main scalar diagnostic had thirteen improved, eleven neutral and four regressed outcomes; its predecessor had thirteen improved, seven neutral and eight regressed. The earlier thirty-two-pair failed cohort and every other unsuccessful diagnostic remain retained. The earlier task-owned evidence-copy path failure resumed the successful original measurement's candidate half without repeating that successful measurement. No failed receipt is discounted as noise, a compiler defect or a favorable-repeat pass.

Complete unchanged shallow performance acceptance, actual landed-main synchronization and affected source qualification, final native/full gate, the one genuinely WASM-specific shadow-stack-floor check and portability builds, final additive semver checks, normal verified signed commits/pushes, PR, actual final-head CodeRabbit, hosted CI and serialized ghprsq integration remain required. Generic trait semantics and formatter oracles stay native Rust. No decimal work, new issue, dependency, feature, runner, general WASM semantic corpus, benchmark setting, toolchain or host change is introduced.


