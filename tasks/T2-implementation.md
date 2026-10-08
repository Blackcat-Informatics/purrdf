# Production glossary callers: Task 2

Status: PASS for assigned implementation and focused qualification. Source is
settled and ready for independent review. Root owns review, normal commit/push
and issue progress. No real-index mutation, Git commit, forge action or full
suite was performed. The build lane is FREE.

Both Make callers and workspace CI invoke the single native helper mode.
The existing pre-commit parallel gate slot now invokes admitted working-tree
helper code with the exported staged snapshot root; every other slot remains.
The docs workflow includes helper sources in its path triggers because it
reaches the glossary gate through check-i18n. Gate parity recognizes the native
mode without increasing Python lines. The obsolete glossary Python body is
removed; render tooling and its shared PO reader remain. Authoritative tool
references now name the native gate and bounded Python-compatible patterns.

New Rust integration fixtures execute the actual CLI over external root, PO and
glossary inputs, tracked content-selected Markdown renamed without language
markers, multiline fences, visible poison, dropped RDF token and missing input.
They execute the existing production parity program against copied actual
Make/workflow files, including both local and hosted glossary omissions and a
passing neighbour. No second parity implementation was introduced.

An explicit Rust qualification example runs the real normal hook twice with a
private index. It stages tracked working-tree changes plus the two named new
Rust fixtures only; Stage evidence and unrelated untracked files are excluded.
It inserts a poisoned PO Git blob into that private index with clean working
text, then restores the private index and poisons only working text. An RAII
guard restores exact catalogue bytes; the real index is never written. Actual
refusal/pass and exact real-index/catalogue byte readbacks passed. The example
is explicitly test=false so a concurrent general harness cannot run its poison.

## Actual focused results

All Cargo builds used CARGO_BUILD_JOBS=8 and direct Cargo --jobs 8 under root
lane admission; nested Make/hook Cargo children inherited the eight-job cap.

- Caller integration target: PASS, two actual CLI and parity tests.
- Native glossary filtered target: PASS, ten tests; unrelated tests filtered.
- Strict helper all-target clippy: PASS after final readback assertions.
- Native production scan: PASS, 69 rows, 43 rejections, 24 K tokens, 4,181 units,
  three Markdown documents and 1,716 default production controls.
- Real hook: poisoned private index / clean working PO REFUSED specifically by
  glossary-gate; clean private index / poisoned working PO PASSED. Exact real
  index and catalogue byte assertions passed. Initial and settled pairs retained.
- Gate parity: PASS, all 71 local hygiene identities also in PR workflows.
- Layers: PASS, 208 edges, 42 members and six unsafe-code ring fences.
- Floating-toolchain gate and six-shard coverage of 42 members: PASS.
- Native non-Rust ratchet: PASS, two changed paths; no legacy growth.
- Helper fmt and working diff whitespace checks: PASS.
- Actual make metadata: PASS, all generators executed, 154 prose claims
  qualified and 35 recipient license profiles regenerated. No generated diff.

The docs job already installs floating nightly and its Rust cache before make
check-i18n, so it can compile the native gate; its new helper source path trigger
preserves execution when that source changes.

## Failed attempt retained

Initial metadata reached doc-claims after generators through lexicons, then
failed because the deliberately unstaged real index still listed the deleted
Python file. The gate was not weakened to skip missing files. Actual metadata
was rerun with retained T2-metadata.index representing the settled tracked
changes plus exactly the two named new Rust files. It passed. The real index
was never written. Failed and successful logs are both retained.

## Evidence and remaining delivery

Actual task logs: T2-callers-tests.log, T2-native-tests.log, T2-clippy.log,
T2-clippy-settled.log, T2-sweep.log, T2-hook-probe.log,
T2-hook-probe-settled.log and both T2-hook-logs directories; T2-parity.log,
T2-layers.log, T2-target.log, T2-shards.log, T2-ratchet.log, T2-fmt.log,
T2-whitespace.log, T2-metadata.log and T2-metadata-settled.log.
T2-source-receipt.sha256 binds eleven surviving changed/new source files and
the generated sweep, including the catalogue; the deleted file has no digest.

Modified pre-commit, both workflows, Makefile, helper manifest, glossary prose,
PO header, gate-parity mode recognition and PO-reader documentation. Added
original Rust caller tests and qualification example. Removed obsolete Python.
No dependency, feature, matcher, translation body or other gate was changed.

Independent review and root normal commit/push/progress remain. Task 3 owns the
single full suite, actual i18n/render qualification, hosted CI, PR and ghprsq
integration; none of those is claimed here.
