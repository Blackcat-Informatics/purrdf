# Settled qualification preparation

Status: READY FOR ADMISSION. Read-only inspection only: no builds, tests,
rendering, installs, hooks, Git/forge mutations or Task 3 qualification executed.
Task 2 is committed at 4db9f79477e81377181584e27c5ab28b5766d91a; only Stage
evidence is untracked. Root owns prior rules qualification/integration and any
necessary base synchronization before binding final source identity here.

## Actual installed prerequisites

- PATH mdbook: /home/paudley/.local/bin/mdbook, actual version 0.5.3. This
  matches docs.yaml's MDBOOK_VERSION. Cargo install inventory separately reports
  an older mdbook 0.4.52; that shadowed installation is not selected by PATH.
- Helpers: cargo install --list reports mdbook-i18n-helpers 0.4.0, matching
  Makefile. Actual mdbook-gettext and mdbook-xgettext are both present under
  /home/paudley/.cargo/bin. The package intentionally ships no executable named
  mdbook-i18n-helpers; a failed lookup of that nominal package name is not a
  missing prerequisite. The production version gate checks install inventory.
- Rust: 1.100.0-nightly, commit 4b6d04e706108ccfeafe2547fbe857dfe8972bad;
  Cargo 1.100.0-nightly, 7941be6fb, both through Stage managed launchers.
- Python 3.13.5; Node 26.10.0; wasm-opt 130 matches BINARYEN_VERSION 130.
- wasm32-unknown-unknown is actually installed, so the full gate's local wasm
  target will build rather than invoke its missing-target skip branch.
- wasm-bindgen 0.2.125 is present. The selected translation gate does not require
  wasm-bindgen, but it is available for existing broader qualification surfaces.
- This worktree's target resolves to
  /opt/.cargo/target/260-zh-hans-glossary-gate-harden-the-b9ada488. Render scratch
  uses target/gate-scratch via scripts/build-scratch.sh; no /tmp build relocation
  is needed. /opt had 307 GiB free at this read-only observation; refresh before
  execution instead of treating it as a reservation.

No prerequisite installation or dependency repair is presently indicated.
Runtime support by these installed tools is UNKNOWN until the real gate runs.
Do not change pins, substitute tools, skip rendering, or claim a static version
check proves output acceptance.

SHA256 of selected binaries observed:

- mdbook: 94a83793d0b56b37d551d804766dba238ef7478cda877719b339da6e26ad0610
- mdbook-gettext: 5a66cb701edbe83ba98604e7eca65893c7408ef161e635829b96af7abd0ce62c
- mdbook-xgettext: 2cb5942d28118fac0f25f55111fc7549399fc1480e8c8ee40ac6bbf546428fd8

## Exact authorized execution after root admission

Capture final HEAD, tracked/untracked source identity, current catalogue/table
digests, compiler and selected tool identities after any required base sync.
Use the one owned heavy-build lane and CARGO_BUILD_JOBS=8; direct Cargo commands
also specify --jobs 8. Do not run Make with parallel recipes or race the gates.

1. From the worktree, run the one settled full local gate:

   CARGO_BUILD_JOBS=8 make check > .stage/zh-hans-glossary-gate-harden-the/tasks/T3-full-check.log 2>&1

   Require successful fmt, strict all-target clippy, build/tests, every existing
   hygiene/generated/recipient gate, preserve-order downstream consumer, core
   hygiene and actual wasm release build. The full gate already executes the
   native glossary production scan; preserve its measured current counts.

2. Run the actual second production Make caller and complete rendering gate:

   CARGO_BUILD_JOBS=8 make check-i18n > .stage/zh-hans-glossary-gate-harden-the/tasks/T3-i18n-render.log 2>&1

   This first invokes the native glossary and its complete default controls,
   then python3 scripts/check-i18n-render.py --self-test. Do not replace it with
   a simple mdbook build or a direct pattern test.

The rendering program enforces both selected tool pins, extracts a fresh English
POT through mdbook-xgettext, and renders through the real gettext preprocessor.
Seven poisoned catalogue cases must go red: four prose gates, two distinct
SPARQL fence poison shapes and a stale untranslated-msgid reachability poison.
Every ordinary poison must actually reach rendered output before its refusal
counts. It then renders the actual zh-Hans catalogue to HTML and Markdown and
requires all six arms: branding, issue-reference policy, specification
attribution, entailment/documented claims, SPARQL syntax, and translated-message
reachability. It reports measured catalogue/template drift. Scratch cleanup is
the existing program's responsibility; retain complete command output as proof.

No separate catalogue refresh, translation expansion, book publication, release,
Java workload or additional benchmark campaign is required for this scope.
Do not rewrite the tracked PO merely because drift is reported: determine
whether actual acceptance fails and fix any real scoped defect with evidence.

## Delivery after gates

Record actual current row/control/scan counts, source/tool identities and any
failed attempts without presenting them as passes. Independently audit every
criterion against implementation and runtime evidence. Root owns final Stage
PR, hosted checks/review, Stage 2 corrections, ghprsq integration and preservation
of selected evidence before deleting only the merged task's worktree/branch.
All Task 3 runtime qualification and integrated delivery remain NOT RUN here.
