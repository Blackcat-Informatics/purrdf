# Fault-driver focused qualification — 2026-10-08

Status: PASS for the assigned contained driver build/control qualification;
heavy lane FREE. This is not Task3 production Make-campaign acceptance. No Make
LUBM fault run, fourteen-query campaign, full gate, commit/push or forge action
was performed. Root authored the driver and corrected its temporary-directory
constructor; this qualifier made no source edit.

Read applicable main baseline/goals, worktree instructions, sole plan, validation,
Task3 preparation and independent fault-driver review. Used normal managed Cargo
route, CARGO_BUILD_JOBS=8 plus explicit --jobs8 for every compiling invocation.
Actual raw compiler/Cargo identities are retained in T3-fault-logs/rustc.txt and
cargo.txt. No HOME override, synthetic cache tag or private target override.

## Actual commands and terminals

All logs/JSON/terminal summary are under tasks/T3-fault-logs.

| Command | Actual result |
| --- | --- |
| cargo build --locked --release --jobs8 -p purrdf-bench --example lubm_fault_cli --message-format=json-render-diagnostics | exit0; actual unique Cargo artifact selected, build.jsonl/build.log |
| cargo test --locked --jobs8 -p purrdf-bench --example lubm_fault_cli, initial | exit101; compile-time CARGO_TARGET_TMPDIR absent for example target, tests.log preserved |
| same test after root correction | exit0, one actual Unix identity test passed, tests-corrected.log |
| cargo clippy --locked --jobs8 -p purrdf-bench --all-targets -- -D warnings, initial and corrected | both exit0, warning-free; both logs retained |
| same release build after correction | exit0, actual selected artifact in build-corrected.jsonl/build-corrected.log |
| actual delegated/fault/configuration child batch | exit0, 22 real child executions; details below |
| cargo fmt --all -- --check; git diff --check | actual exit0 each, fmt.log/whitespace.log |

The failed example test was reported before any edit. Root replaced the
integration-test temp_dir macro with the existing public
purrdf_testkit::TempDir::for_unit_test home. The corrected test executes actual
direct/symlink/hardlink identity equality and separate-file inequality. No
environment workaround or duplicate temporary-path implementation was introduced.

## Actual CLI delegation and refusal controls

Owned artifacts are retained in
/opt/purrdf-lubm-fault-controls-20261008.2p06nl4h. commands.json records every
actual child argv/exit and environment changes; per-child stdout/stderr remain
separate. The driver executes the actual Task2 release CLI, not a mock engine.

Direct real and delegated driver executions compare exact exit/stdout/stderr
for --version, actual Turtle-to-NQuads conversion, actual RDFS regime ASK probe,
unselected ASK query and a selected-query neighbor differing by one significant
trailing space. Every pair succeeds and matches exactly. The fixture contains
an actual RDF statement; conversion writes real NQuads. No fault response
qualifies a positive engine result.

The selected query file has two trailing LF bytes. Passing its Bash-equivalent
LF-stripped text actually triggers all three modes: malformed exits0 with the
deliberately malformed bytes; wrong-uri exits0 with parsed syntactically valid
JSON and the deliberately wrong URI; exit mode exits83 with empty stdout.
Each has its attributable qualification diagnostic. A significant-space change
delegates to the real engine, proving selection is exact rather than approximate.

Missing real-CLI/query/mode variables, invalid mode, empty query, nonexecutable
CLI and direct/symlink/hardlink self-delegation each actually exit1. Aliases
produce the separate-regular-executable diagnostic and terminate within the
bounded child timeout. No recursive process was left running. Mode responses
remain qualification faults, never successful production benchmark acceptance.

## Source/executable binding and remaining acceptance

Settled source SHA256:
a56550a1e8c47cb8eff2fd6f9642cba6317c99a159139e8f52793934b5e8d0a3.
Actual Cargo release executable is
/opt/.cargo/slots/589dcf4f131a442c/0/target/release/examples/lubm_fault_cli,
SHA25641158387d1baa91146caa50014f878b22fd3f14d74a62902faabd7bcc94aa416.
Frozen task-owned copy at the control directory's lubm_fault_cli has identical
bytes. The corrected source changes only the cfg(test) constructor; rebuilt
release artifact identity is actually equal to the artifact used for all runtime
controls. No unchanged control batch was repeated merely for that test-only fix.
settled-identity.json retains actual Cargo target/profile/executable and all
source/CLI/copy identities. Real CLI SHA256 is
efa0e30cbe8627fce67c1ea63a1c81c045a2bf89e7e5dea32c7e7b04bc1e5691.

The source review is applicable to the corrected shared-temp home plus actual
qualification. This report does not replace independent judgment of production
Make outcomes. The accepted four full fourteen-query default/nondefault/cache
campaigns and six actual Q1/Q14 malformed/wrong-uri/exit Make controls remain
NOT RUN and require root's separate campaign admission. Those controls must
prove BAD-RESULTS/CANNOT-EXECUTE through the real row/oracle/exit-trap paths,
not substitute these direct driver probes for complete benchmark acceptance.
Prior Task2 qualification remains historical, with no production algorithm or
shared WatDiv/scale law changed by this driver.
