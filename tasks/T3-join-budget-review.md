# Independent LUBM explicit join-budget delta review

VERDICT: PASS

Source and focused admission review PASS for the four current patch paths: Makefile, scripts/lubm-lane.sh, docs/BENCHMARKS.md and crates/bench/tests/make_bench_lanes.rs. No builds, tests, source/Git changes or forge actions were performed by this reviewer. Task3/full campaign completion is NOT established by this bounded verdict.

The preserved real cold retry was PARTIAL, three of fourteen, because full RDFS and OWL-RL closure exhausted the ordinary1,048,576 join-step default on the109,647-fact input. Root's actual same-corpus/current-CLI64GiB-scope diagnoses with explicit100,000,000 join steps both terminated0, and T3-full-rdfs-diagnosis.log/T3-full-owlrl-diagnosis.log contain actual one-row SPARQL JSON with the University0_0.nt URI. These prove the full corpus can close under that finite budget, not that the fourteen-query campaign now passes. Original partial/failure evidence must remain historical.

## Source findings

* Makefile:552/560 adds LUBM_MAX_JOIN_STEPS with default100000000 and routes its exact bytes through existing lane-env. That home freezes Make command-line values via value and exports them, rather than reparsing an interpolated shell command. The new knob is not executed as code.
* lubm-lane.sh:35/98 applies existing lane_require_positive before acquisition. Its one shared unsigned parser requires digits, normalizes leading zeros textually, bounds signed-shell-safe values at9223372036854775807 and rejectszero. There is no arithmetic wrap, ad hoc second numeric parser or new fallback.
* lubm-lane.sh:436–440 is the single run_query owner. Every actual entailment probe (call at523) and measurement (call at609) receives the same normalized MAX_JOIN_STEPS in its argv array. The no-entailment path remains unchanged; the production CLI flag requires entailment, so adding it to plain query rows would be incorrect.
* All dataset generation/acceptance, full/file/slice bytes and hashes, raw/normalized query sets, independent Q1/Q14 graph/result oracle, exact results parsing, stdout/stderr separation, executable identity and profiling/timing window remain unchanged. This patch increases admitted work without dropping input or changing a conclusion. No engine/API/global default changes occur.
* Summary records the actual selected budget. Ladder diagnostics retain the actual rung, counts, engine diagnostic and partial/unexecuted status. The obsolete universal claim that no CLI flag raises a fixed ceiling is removed. A smaller rung still cannot count as full-scale COMPLETE;100m is not promised to fit every corpus.

The flag is a real production option at crates/cli/src/cli.rs:374–379, an optional u64 join-step ceiling. Exhaustion is still a hard refusal with observed/permitted counts; larger admitted budget does not mean unmetered/infinite work or a suppressed diagnostic. No source/governor violation or workload softening was observed.

## Actual focused evidence

Root session75448 actual terminal0; T3-join-budget-admission.log records one passed test, zero failed,16filtered. The new Rust test at make_bench_lanes.rs:722 exercises actual Make/script admission: five invalid values (zero, negative, explicit plus, junk, overflow) must fail by knob name before1/7artifacts; four valid values (one,001,default100m,signed64maximum) must reach the deliberately unusable owned arena and fail by LUBM_OUT instead. The regular-file parent makes that arena unusable for every uid, ensuring valid controls do not accidentally fetch/build/run a corpus. The test establishes actual early admission and normalized guard neighbors, not full campaign or all budget propagation runtime behavior; latter source routing is one shared run_query.

Actual strict changed-package all-target Clippy/fmt, normal hooks/source publication and the resumed complete campaign remain pending at review time. Do not repeat unchanged fixtures/gates without a new failure/change. Task3 retains its binding full fourteen OK/full rows,14of14/COMPLETE, default warm/cold attribution, two-university/custom inputs, real fault/tamper controls and exact artifacts. The three-row partial retry and successful closure diagnoses do not discharge those criteria. No new source defect is identified in this bounded correction.
