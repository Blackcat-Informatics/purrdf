# Independent Task 2 review

**PASS.** The settled production caller migration and obsolete-path retirement
satisfy Task 2. Reviewed main .baseline/.goals, applicable AGENTS instructions,
the sole plan and prior corrected Task 1 review, all current Task 2 changes,
the native input-selection path, exact source receipt and actual focused logs.
The reviewer did not implement this source. No builds, tests, scans, timing,
Git/forge mutations or source edits occurred during review; only this report
was written. This approval does not certify Task 3 or the completed issue.

## Production callers and single implementation

- Makefile's actual `check` and `check-i18n` recipes both invoke
  `cargo run -q --locked -p helper-census -- --glossary-gate`. The workspace CI
  job invokes that same mode after installing the floating nightly toolchain.
  The docs job reaches it through `make check-i18n`; nightly and its Rust cache
  are installed first. Its path filters now include helper-census source changes.
- `.githooks/pre-commit:95–104` preserves the existing parallel glossary gate
  slot and substitutes the native mode with explicit `--root "$snapshot"`.
  The hook first exports the real/private staged index, Git directory and
  snapshot work tree after checkout-index. Cargo builds admitted working-tree
  helper code, as the existing native policy slot already does, while its gate
  reads staged catalogue/table/Markdown bytes. Remaining hook gates and refusal
  aggregation are preserved. No verification bypass was introduced.
- The old 717-line Python glossary implementation is deleted. Authoritative
  glossary/PO/tool documentation names the native gate and bounded compatible
  patterns. A search of live scripts/crates/docs/Make/hooks/workflows finds no
  remaining old-file reference. `check-i18n-render.py` and its actual po_catalog
  imports remain; po_catalog's modification is documentation only. No matcher,
  dependency, feature or translation body was changed in this task.

## Input selection, parity and real staged-hook controls

`crates/helper-census/tests/glossary_callers.rs` executes the actual compiled
helper with an external root, PO and glossary. It clears inherited Git context,
uses actual Git-tracked content-selected Markdown, renames that file without
language markers, checks multiline fenced exclusions, then plants visible
global poison and a dropped English RDF token. Missing external glossary input
must hard-fail. Diagnostics name the actual selected external/renamed paths.
The production selector in `glossary/mod.rs:375–419` retains NUL-safe sorted
git-ls-files paths, content-based translation selection and glossary exclusion.
Normal production execution runs every table self-test before scanning inputs.

The parity fixture runs the existing production parity program against copied
actual Make/workflow inputs. It first checks a passing neighbour, then removes
both local native occurrences while retaining hosted execution, and separately
removes the direct CI and transitive docs occurrence while retaining make check.
Both directions must refuse the native identity. The existing parity extractor
adds that one mode to its existing recognition; it does not create another parity
implementation or grow the legacy Python file.

The explicit `glossary_hook_probe` example stages settled tracked changes plus
exactly the two named new Rust files in a private index. It inserts a poisoned
PO blob into that index while keeping working PO clean, executes the real normal
hook and requires glossary-specific refusal. It then restores the clean private
index, poisons only working PO and requires real-hook success. Exact before/after
real-index and catalogue bytes are asserted after restoration; the RAII guard
restores working catalogue bytes on ordinary failure. The example is test=false
and is only an explicitly serialized qualification instrument, so a general
concurrent harness cannot initiate its working-file poison.

Actual T2-hook-probe-settled.log and both retained settled hook logs show refusal
of the staged RDF omission at snapshot PO line 24269, success for the inverse
working-only poison, and exact real-index/catalogue byte preservation. These
prove staged snapshot admission rather than merely testing a mock hook function.

## Evidence binding and qualification limits

Independently verified every current T2-source-receipt.sha256 entry: eleven
surviving changed/new files and the actual sweep match, including restored PO
bytes. The deleted Python path has no source digest and is explicitly recorded
as removed. Read-only inspection of T2-metadata.index confirms precisely the
settled tracked changes, deleted old tool and two new Rust files, with no Stage
artifact or unrelated untracked file included.

Actual settled evidence records two CLI/parity integration tests and ten native
glossary tests PASS; unrelated filtered tests are not counted as coverage.
Strict all-target helper clippy passes. The production scan reports 69 rows,
43 rejections, 24 K tokens, 4,181 translated units, three Markdown documents and
1,716 default production controls. Parity records 71 identities; layers records
208 normal edges across 42 members and six unsafe-code ring fences. Floating
toolchain agreement, six-shard coverage, native non-Rust ratchet, formatting and
diff checks pass. No legacy non-Rust growth is hidden by the old-file deletion.

The first actual metadata attempt failed because the deliberately unstaged real
index still listed the deleted Python tool. The failure remains recorded, and
the missing-file invariant was not weakened. Actual make metadata subsequently
ran with the isolated settled index and passed its generators, 154 documented
claims and 35 license profiles; no generated source-controlled diff remains.
The reviewer inspected those actual logs and source bindings without rerunning
any build/scan/hook or modifying either index.

Root's normal Task 2 commit/push and progress update remain the immediate handoff.
Task 3 still owns the settled full suite, actual complete i18n/render execution
and its prerequisites, completion audit, hosted qualification, PR and ghprsq
integration. A native scan and static caller inspection are not a
rendered-book acceptance claim. No blocking Task 2 correction was found.
