<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Repaired-source host qualification

The identity admission repair changes FILTER and OPTIONAL execution. Earlier CLI and Wasm receipts establish their captured source behavior; they do not qualify this repair. Fresh affected-host checks are required.

Production diff captured in raw/identity-host-production.diff has Git blob c87119193f1e8ea4d349c3b6f7572991aa23b099, against committed prerequisite f35709b0e. It covers evaluator and host production paths, excluding count tooling still under development. The source writer confirmed production source remains stable during these checks.

- Wasm release evaluator library check: PASS, raw/identity-evaluator-wasm-library.log.
- Wasm numeric runtime: corrected-source retry PASS, both cases, frozen digest da3900a93723c4ac over 2,727 bytes, raw/identity-numeric-governor-wasm-runtime-retry.log. The initial compile FAILED because the new shared workload fixture called ToString on a hash without Display; raw/identity-numeric-governor-wasm-runtime.log retains that failure.
- CLI governor integration tests: PASS, all 25 cases, raw/identity-cli-governor-tests.log.
- Normal release CLI build: PASS, raw/identity-cli-release-build.log and raw/identity-cli-release-artifacts.jsonl. Preserved executable /home/paudley/Active/purrdf-recovery-artifacts/20261006-2253/governor-experimental-source/compiled/cli-identity-release has Git blob 4b7dd5184f24dd371b1f37e933b436c9a4394b81, raw/identity-cli-release-identity.txt.

The existing 200,000-subject OPTIONAL witness passes on that release executable with RAYON_NUM_THREADS=32 and explicit --results-format tsv. Plain, fuel 1,000,000,000 and inclusive 600,000-cell outputs are byte-identical. The existing Rust verifier checks all 200,000 exact subject/integer pairs, both cells bound, with no duplicates (raw/identity-optional-200000-answers-verified.log). The 599,999-cell neighbor exits 3 with consumed 600,000 and a certain 199,999-row prefix; its entire TSV equals the header plus the first 199,999 verified answer rows. Explain remains profile v13, fuel 1,400,005, rows 200,000, cells 600,000 and scratch 76. Receipts are raw/identity-optional-200000-*. These are functional calls without elapsed-time performance claims.

Initial witness commands omitted --results-format tsv. They succeeded as queries and produced JSON, preserved as raw/identity-optional-200000-*-default-format.json. The TSV verifier exited 101 on their JSON header, and the attempted TSV prefix comparison failed. Corrected explicit-format invocations and every full-answer/prefix comparison above pass. The initial invocation does not count as a TSV check.

Instruction/allocation counters, required final gates, normal commit hooks, PR publication and protected integration are separate acceptance checks. This record does not mark the combined governor unit complete.
