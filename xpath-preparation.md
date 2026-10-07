<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Retained XPath and counted-repetition preparation

This is a recovery preparation artifact, not implementation or qualification evidence. Source, sibling worktrees and refs were inspected read-only. No builds, tests, commits, Java, Gitee or model actions were performed. The only writes in this preparation are this report and the companion recovery coverage report in the governor Stage directory. Issue and PR identifiers below are process mappings requested for recovery; they are not proposed shipping documentation.

## Source identities and extraction boundary

| Asset | Verified identity | Treatment |
| --- | --- | --- |
| Current integration baseline | `dfc0c21adabe557e5d11f027aaf2576bddbe1dd6` | Preserve its governor, exact numeric, division-policy and host fixes. |
| Retained XPath branch, PR 448 / issue 406 | `9e9e67c80d832175bdafb8e938d7abae00486f8b`, worktree `.worktrees/406-native-xpath-regex` | Clean worktree at inspection. Last commit merges the integration baseline above. Continue this retained implementation; do not create another matcher or profile layer. |
| Counted-repetition WIP, issue 467 | `efd2829a65a1139818ed4f610d4ff414c48b0e2b`, branch `paudley/406-large-count-followup` | One unfinished commit, no registered worktree. Candidate material only. |
| WIP parent | `36a13ac79bd5883a8f67fb3791c46cfb0afca483` | All four compiler/matcher files are byte-identical between this parent and retained XPath head. The own-commit delta is the correct extraction source. |

The WIP commit changes 11 files, adding 2,153 lines and deleting 124. Comparing its entire tip against retained XPath instead reports 205 changed files and about 61,672 deletions: the WIP predates later integration changes. Never restore the WIP tree wholesale or use that tip-to-tip diff as a recovery patch. Inspect/extract `efd2829a6^..efd2829a6` and retain current branch contents elsewhere.

Of the six host/consumer test files affected by the WIP, only two changed after its parent: Python added 23 lines and CLI added 29 lines for ambiguous counted REPLACE comparisons. Preserve those additions when appending new fixtures. The four core files, the bench file, C test, Wasm native export test and SHACL/ShEx tests have no intervening retained-head delta against the WIP parent. This observation establishes a textual extraction boundary, not correctness of the WIP.

The historical handoff is `WHY_I_FAILED.md` sections beginning at lines 652 and 705. Its 288,000 differential comparisons and 25,000 REPLACE comparisons belong to an older source state. Its last merge had 29 conflict paths; historical CI/Python results do not qualify the completed combined unit.

## Minimal coherent patch sequence

1. **Append the independent reproductions and bench cases.** Extract the new large-class/group tests and benchmark fixtures from the WIP without changing production behavior. Keep the existing Python/CLI ambiguous-REPLACE additions. The fixtures assert exact spans/captures, explicit false neighbors and replacement bytes; they must fail visibly if the old refusal remains.
2. **Finish one counted-repetition implementation in the existing four-file home.** The dependency-closed file boundary is `compile.rs`, `match.rs`, `pike.rs`, `sets.rs` under `crates/rdf-core/src/xsd_regex/xpath/`. The WIP touches shared node metadata and count interpretation in every execution machine, so taking only its enlarged backtracking loop would leave inconsistent semantics. Use the intent map below to review the four-file delta rather than treating the unfinished commit as verified code.
3. **Preserve typed-resource witnesses under the new fast path.** The WIP changes three existing fixture patterns from single-character alternatives to `a|ab`: one SHACL and two ShEx fixtures. Those changes are relevant because `a|b` now takes a cheaper run path; retain a genuine exact refusal and an admitted neighbor for each named resource. Do not change expected errors merely to get a pass.
4. **Qualify actual production hosts and the retained merge.** Run the targeted checks below on the completed combined source, including a real Wasm package/runtime witness. Confirm dated regex selection and division policy both survive on the shared host paths.
5. **Correct cost/refusal documentation from fresh measurements.** The WIP contains no documentation repairs. Rewrite the precise claims below after the completed source has fresh step and timing receipts. Regenerate affected projections/catalogue normally; do not hand-edit generated output or vendor vectors.

The WIP was intentionally committed as unfinished. A coherent extraction may still use its own-commit patch as candidate material because the core files have the same base; it cannot be landed merely because it applies. Do not cherry-pick an entire historical branch, replay unrelated parent commits, or reset the retained head.

### Production intent and dependencies

| Existing file/home | Candidate change | Review obligation |
| --- | --- | --- |
| `compile.rs`: `Node::Repeat`, parser sequence follow annotation | Replace `Option<usize>` with `Follow::{Set, End}`; recognize an immediately following `$`; include new `Links::normal` storage in `heap_bytes`. | Preserve both dated anchor laws, multiline anchors, character-set admission and exact resource accounting. A following end anchor is a necessary pruning condition, not permission to change match priority. |
| `pike.rs`: `Link`, `Links`, `analyze`, count transitions | Detect capture-free single-character alternatives and captured chains of those runs; record large counted repetitions eligible for relative offsets; use execution-effective count bounds. | A run may collapse alternatives only when they consume one character and have identical continuation/capture effects. Preserve nested captures, unset captures, backreferences, reluctance and empty-iteration rules. Charge retained vectors and construction work. |
| `match.rs`: `Ctx::bounds`, `large`, `window` | A finite maximum greater than `input.len() + minimum + 1` cannot bind during this execution, so treat it as unbounded without changing the compiled pattern. | The input-length proof counts possible consuming iterations plus pre-minimum and terminal empty iterations. Test UTF-8 and nullable bodies, counts around `u64`, and boundaries where a maximum becomes binding; do not approximate a reachable finite count. |
| `match.rs`: `Attempt::{credit, exclude, exceeded}`, `Backtrack::{greedy_run, known, remember, run_stop, may_begin, single_matches}` | Credit first-time linear progress; extend class/choice/capture runs; reuse scanned run extent instead of rescanning overlapping search starts. | All real work remains charged to the caller's budget. Re-examining input must earn no new credit. Preserve first match, greedy/reluctant priority, last-iteration captures, capture chains, real failing neighbors and exponential-backreference refusals. New cached extents must not cross UTF-8 boundaries or reuse an incompatible class/minimum. |
| `sets.rs`: `normal`, `far_counts`, `absolute`, state/cache offset handling | Intern non-nullable finite-count states relative to an offset away from count bounds; admit absolute states at sensitive transitions. | WIP uses `LARGE_COUNT = 1024` and `MARGIN = 4`. Test each side of both thresholds, reverse traversal, cache clearing, tight checkpoint/cover limits, nested counters, minima/maxima and exact resource boundaries. Sharing relative shape is valid only while all future choices agree. |

The input-effective bounds, Pike count storage, forward/reverse set state interpretation and cache keys must agree within one execution. The WIP has cross-file dependencies through `Follow`, `Links::single/run/large/normal`, `Ctx::bounds/large/window` and `LARGE_COUNT`; it is not a set of interchangeable alternative matchers. Its relative-offset optimization expands coverage beyond the simplest class run; profile it and retain only justified coherent changes, without silently omitting required group/search acceptance.

## Acceptance map

The live counted-repetition requirement was read once during this preparation. All rows below remain **NOT RUN on the completed candidate**.

| Required behavior | Candidate tests/assets | Required final proof |
| --- | --- | --- |
| `^[\\s\\S]{0,10000000}$`, `^[^\\n]{0,10000000}$`, `^[^<>]{1,5000000}$`, `^[\\p{L} ]{1,5000000}$` answer on 4 MiB under both dated laws | Core `large_counts_of_one_class_or_one_group_answer_at_the_production_defaults`; Python, CLI, C and native-Wasm appended query cases | Exact booleans through REGEX at production bounds; matching input plus class-violating/over-maximum neighbors. No silent unbound result, raised default or partial result. |
| `^(\\w+\\s?){1,5000000}$` on 4 MiB and REPLACE on 2 MiB | Same core case; `large_counted_group_replacement_answers_at_the_production_defaults`; each host's appended REPLACE case | Whole match, last-word capture and `[$1]` replacement bytes agree. A trailing `!` returns false. |
| `^(a|b){4194304}$` and `(a|b){4000000}` on 4 MiB | Core exact/first-search cases; four host case tables; bench exact/search/broken lanes | Exact whole span or first span `0..4_000_000`, last-iteration capture, over-length and interrupted/too-short false neighbors. |
| Per-byte cost independent of the repetition count | Core `steps_per_byte` asserts proposed <=4 steps/byte for class/word forms and <=6 for exact-choice forms; compares maxima with tenfold larger maxima | Verify these candidate assertions rather than adopting them as facts. Hold input fixed and vary count; hold pattern fixed and scale input. Report REGEX, find/capture and REPLACE separately using fresh actual counters and testkit estimates. Include existing 8 MiB `[a-z ]`/dot neighbors. |
| Randomized agreement with the backtracking engine | WIP `large_and_exact_counts_agree_across_machines_flags_and_laws`, independent finite-language additions, existing differential suites | Compare exact capture vectors and successive search spans, including UTF-8, all applicable flags and both laws. Compatibility regex is a control only where semantics agree. Preserve original seeded suites. |
| Relative-offset states are exact | WIP `counts_kept_relative_to_offsets_agree_with_the_thread_machine`; claimed 30,884 comparisons, not executed here | Add/verify 1023/1024/1025 counter boundaries, transitions near minima/maxima, clearing/rebuilding caches, reverse state restoration and tight checkpoints. Compare production, straight thread and tight set/thread paths. |
| Typed refusal and exact admission boundary remain | Existing resource-boundary suites, updated SHACL/ShEx fast-path fixtures | Admitted exact requirement versus one-less for work/state/all live storage; resource identity, fresh execution fuel and reused-program admission. Never turn exhaustion into `false` or an expression error. |
| Actual CLI/Python/C/Wasm callers share the implementation | WIP host query witnesses; existing retained host selector and error-channel tests | Current-source artifact identity plus actual host execution. `crates/rdf-wasm/src/xpath_regex/exports.rs` explicitly tests the native layer beneath JS exceptions; its appended Rust test alone is not real Wasm/JS qualification. Exercise sync and async package entrypoints with both dated laws. |
| Retained XPath/main merge preserved both features | Shared Python `.pyi`/`py_store`/ShEx, CLI query/update/ShEx, C header/query/version/smoke, Wasm operation/query/async, SPARQL engine paths listed in handoff | Inspect both merge parents and rerun actual selector/division controls. `operation.rs::selected_options` currently threads both division and optional XPath selection; preserve that path during later exact-only migration. |

The new Python file already carries a concrete `Why not Rust` explanation because it exercises the real Python surface. If PR 448 lands before adding this material, adding 86 lines to that now-existing test path would engage the non-Rust ratchet. Completing the combined retained unit before landing avoids inventing an exemption or bypassing the gate. A later standalone patch must respect the ratchet through appropriate Rust-hosted surface coverage.

### Focused check inventory for implementation owner

Use the repository's optimized assertion-enabled test profile and normal lint/hooks. Candidate commands are recorded for the next implementation stage; none was run during this preparation:

- Core named large-count, replacement, randomized, offset and typed-resource tests via `cargo test -p purrdf-core --lib <test-name>`.
- `cargo test -p purrdf-cli --test xpath_regex_cli`; `cargo test -p purrdf-capi --test xpath_regex`; retained SHACL/ShEx `native_xpath` targets.
- `cargo test -p purrdf-wasm --lib xpath_regex` for native wrapper coverage, followed by the existing Wasm package/runtime harness with the large-pattern witness. Verify exact artifacts; compilation alone is insufficient.
- Build the current Python extension by its existing workflow, then its real `test_xpath_regex_profiles.py` surface suite. An already installed older wheel does not qualify the source.
- Existing `native_xpath_large` benchmark group through `purrdf_testkit::bench`, separating setup, `find`, `is_match`, and REPLACE, with actual MatchSteps and retained-storage observations.
- Required targeted shipping-profile lint/build/hygiene and all normal commit hooks. The owner has excluded broad discovery builds; do not run Java or substitute a manual test for hook verification.

## Exact stale documentation and thresholds

| Current retained path/location | Defect | Required correction |
| --- | --- | --- |
| `crates/rdf-core/src/xsd_regex/xpath/mod.rs:160-190`, `Limits::new` documentation | Cached-state counted forms are described as 1.00-1.02 steps/byte and ordinary input storage as independent of length; the passage omits finite large-count forms whose states do not repeat. It suggests nested nullable counts fail only when each count is near the storage bound. | Describe the implemented classes separately: cached repeat states, binding finite counts, greedy runs, nullable/nested counters and backreferences. Use final source-bound observations for REGEX/find/REPLACE; do not imply a universal cheap constant. |
| `docs/book/src/project/diagnostic-codes.md:222-245` | Broad 1-3.2 steps/byte and 74-83 MiB admission statements; claim that costly find forms still have ~1-step yes/no decisions; nullable nesting threshold described as counts near the storage bound. | Correct all linked claims together. Historical issue evidence says the newly required forms cost 56-93 steps/byte for REGEX and 140-215 for find/REPLACE, and fail at 1-4 MiB. These are historical observations, not completed-source results. |
| Same chapter, `xpath-match-steps` row at line 253 | Lists only backreferences, ~74-83 MiB input and the previously described match searches. | Make the meaning/remedy reflect actual final cost classes and typed operational failure. Avoid a universal maximum admitted input size. |
| Same chapter, `xpath-match-slots` row at line 255; module doc around line 188 | Suggests empty-capable nested counts must each approach the 1,048,576-cell slot bound before refusal. | The recorded counterexample is `((a?){5000}){5000}b` on `aaab` refusing while the `{3000}` counterpart answers. Rerun both with exact resource/required/limit before documenting the new boundary. Do not raise bounds or relabel a refusal as false. |
| `docs/BENCHMARKS.md`, the book's related links and `docs/book/po/zh-Hans.po` | WIP adds 95 bench lines but no updated cost or threshold text. | Add final measured conditions and lanes; regenerate/update translation through its normal procedure. Historical timing prose is not qualification of the WIP. |

The proposed resource defaults remain PatternBytes 65,536; CompileSteps 4,000,000; ProgramNodes 262,144; CompileSlots 2,097,152; MatchSteps 250,000,000; MatchStates 65,536; MatchSlots 1,048,576; OutputBytes 67,108,864. Finishing this unit should satisfy the ordinary-shape acceptance through an algorithmic repair rather than quietly expanding these ceilings.

## Handoff status

Extraction boundary and source/acceptance dependencies are established. Algorithm correctness, randomized counts, shipping-profile checks, actual four-host execution, final performance, documentation repair and hosted integration are all unfinished. Continue on the retained XPath source after the active governor foundation is integrated; keep SHACL-only and portable-corpus recovery outside this matcher patch except the already existing consumer controls needed to prove it.
