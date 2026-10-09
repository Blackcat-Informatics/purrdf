# Task4 configuration admission correction — 2026-10-08

Status: source correction and affected local qualification PASS; actual receipts
are in T4-config-qualification.md. The immutable hosted attempt remains failed for measurement acceptance.
Root owns commit/push/rerun and the heavy lane; no build was performed here.

## Actual hosted observation versus local diagnosis

All twelve hosted attempt1 cold receipts passed private empty-artifact admission,
then failed `effective-build-configuration` with exit101, before any native
command. The source query was `cargo -Z unstable-options config get build
--format json-value`. Its historical helper withheld raw stderr, so the exact
hosted Cargo reason is unavailable; it cannot honestly be called confirmed
absent-table diagnosis. All12 exact failed arm artifacts and their SHA256 manifest
are retained, along with the complete job logs and current artifact URLs.

Actual metadata-only LOCAL probe at
`/opt/purrdf-308-config-probe.XuUSRyYv` uses the same active raw SDK Cargo,
not the Stage wrapper. Cargo1.100.0-nightly commit7941be6fb; fresh owned working
directory/HOME/CARGO_HOME and `env -i` eliminate real credential/config leakage.
The retained outputs, statuses and `cargo-version.txt` establish:

| Exact query/control | Actual exit/output |
| --- | --- |
| `-Z unstable-options config get build --format json-value` |101; `error: config value build is not set` |
| Same individual query for profile/target |101; corresponding exact not-set diagnostic |
| `-Z unstable-options config get --format json-value` |0; `{}` plus harmless clean CARGO_HOME note |
| No-key query with explicit CLI build.jobs4/profile.dev.opt-level3/target rustflags |0; complete JSON object contains those exact three tables |
| No-key query with unsupported format |1; explicit invalid-format diagnostic, admitted format names include json-value |

Each command invokes `/home/paudley/stage/packages/rustup/active-toolchain/bin/cargo`
with clean PATH containing that bin and /usr/bin:/bin. No package manifest,
metadata compilation or native build is involved. The successful configured
control uses `--config 'build.jobs=4'`, `--config 'profile.dev.opt-level=3'` and
`--config 'target.x86_64-unknown-linux-gnu.rustflags=["-Ctarget-cpu=x86-64"]'`.
Separate stdout/stderr/exit files preserve every actual result. The host version
differs from admitted hosted Cargo1.101: this is a local source-semantics witness,
not a recovered hosted diagnostic or matched performance result.

The initial local probes above historically overrode child HOME. Root corrected
that convention: subsequent actual no-HOME probe uses only `env -i`, task-owned
CARGO_HOME and owned cwd, returns exit0 `{}`. Its no-home stdout/stderr/exit files
are retained. Future fixtures/commands do not override HOME; source fixture now
uses only CARGO_HOME/cwd isolation.

Cargo's [configuration documentation](https://doc.rust-lang.org/cargo/reference/config.html)
and [unstable config command documentation](https://doc.rust-lang.org/cargo/reference/unstable.html#cargo-config)
describe configuration and the optional-key query. Actual local observations,
not an assumption about undocumented exit-code meaning, drive this correction.

## Single-home production correction

Only `tests/support/profile.rs` was edited by this actor. Root independently
removed duplicate `invalid` bodies in profile/hosted and exposes the existing
phases-home helper; these source changes remain local and unpushed while the
hosted run completes.

The single `query_configuration` home uses Cargo's successful no-key JSON query.
`configuration_projection` parses its output privately, immediately retains ONLY
build/profile/target code-generation tables, and represents each explicitly as
`state:present,value:...` or `state:absent`. Both effective-build admission and
resolved identity use this same path. Successful `{}` is therefore a bound legal
absence state; a process error is never guessed to mean missing configuration.
Present tables require actual JSON objects; malformed/duplicate/wrong-shape data
hard-fails. The comparison identity distinguishes absent and present configuration.

Complete Cargo response and stderr are never persisted or echoed: unrelated
configuration may contain credentials. Parse diagnostics deliberately omit raw
data. Unknown child failures preserve an actionable command/format, actual exit
and required supported-nightly/valid-config remedy, while withholding raw output.
There is no catch-all empty table fallback, private alternative query or new
dependency. Requested effective jobs/threads/target/build overrides and their
separate captured environment remain explicit; absence of a file-config table
does not mean absence of effective environment configuration.

Three written fixtures cover absent/configured table states, exclusion of a
synthetic registry token and actual comparison refusal for changed state;
malformed/non-object/duplicate/non-object table/invalid UTF8 refusal without
diagnostic leakage; and an actual SDK Cargo subprocess fixture with clean private
CARGO_HOME/working directory (no HOME override), successful absent/configured queries, synthetic registry-token
exclusion and malformed private TOML refusal without leaking its marker. The
fixture resolves SDK Cargo through rustc's actual sysroot instead of inheriting
the managed wrapper; subprocesses query configuration only and never compile.
Fixtures are source coverage until actually compiled/executed.
Direct SDK rustfmt and `git diff --check` actually passed, exit0.

## Required qualification and attempt disposition

After root lane admission: actual native_profile tests, strict all-target CAPI
clippy, live shared-helper census, controller rebuild, actual raw-Cargo
empty/configured config admission fixture and relevant workflow/shard/profile
checks. Preserve exact source/compiler receipts and failures. Existing22-test
receipts are historical on this changed query source; three new tests are written,
not claimed passing. No normal hook/commit verification is bypassed.

Current hosted comparison job113188002388 actually FAILED exit1 after successful
complete attempt restoration, refusing `failed/incomplete phase cannot qualify
a comparison`. Its complete log and artifact11533373092 are retained in
`T4-hosted-comparison-113188002388.log` and
`T4-hosted-comparison-37738739777-attempt1.zip`. Archive includes the cold native
policy and refusal, no successful numerical comparison. The whole workflow
subsequently completed overall FAILURE. The mandatory workspace job also failed
on the three duplicate invalid-helper bodies; its log and final monitor retain
that failure. The shared phases::invalid correction and current live census pass
address that defect without relabeling the historical job as successful. Other
non-profiling mandatory jobs succeeded and optional SIMD projection was skipped.
Root must qualify/commit the owned fixes and authorize a fresh complete matched
attempt afterward; source-only repair cannot turn this failed attempt into a PASS.
