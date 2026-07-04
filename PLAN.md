# Implementation Plan for Issue #30

## Issue Summary

Issue #30 is an epic tracking the completion of SHACL Advanced Features (SHACL-AF) inside PurRDF's validation-only scope. Its four child issues cover:

- **#12** — SHACL-SPARQL custom constraint components (`sh:ConstraintComponent`, `sh:validator` / `sh:nodeValidator` / `sh:propertyValidator`).
- **#13** — SHACL-SPARQL pre-binding substitution semantics (`$this`, `$shapesGraph`, `$currentShape`).
- **#28** — Expression constraints (`sh:expression`) and the node-expression evaluator.
- **#29** — SPARQL-based functions (`sh:SPARQLFunction`).

Codebase exploration shows that the implementation for these four areas is already present, and the existing W3C SHACL harness passes **120/120** with **zero** ledgered xfails. However, the W3C `data-shapes` repository does **not** publish SHACL-AF test manifests — only `core/` and `sparql/` tests exist at the pinned commit and on every current branch. The `vectors/shacl/af/` seam is therefore empty.

This plan closes #30 by sourcing the closest available authoritative AF tests from **pySHACL's DASH test corpus**, converting them to the W3C manifest format, and gating them. Any validation-only AF gaps discovered by the new tests (notably `sh:SPARQLTargetType` and SHACL-SPARQL annotation variables) are implemented so that the AF seam is green. SHACL Rules remain explicitly out of scope.

## Baseline (overridable defaults, from `.baseline`)

- Be adversarial in gap analysis — assume code does not meet the spec until proven by a passing conformance test.
- No deferrals or "left for future" work.
- No GitHub issue/PR numbers in repo documentation or ontology files.
- Fix bugs when seen; blame is irrelevant.
- Strive for best-of-all-choices, greenfield-first implementations.

## Standing Constraints (inviolable, from `.goals`)

- **GREENFIELD-FIRST / SOTA / no compromises.**
- **RUST-FIRST, PYTHON-SURFACE.**
- **SUBSUME, EXTEND, ENHANCE.**
- **LOW/NO OPTIONALITY, HARD FAILS.**
- **MAXIMAL UTILITY, PERFORMANCE, PORTABILITY** (wasm-able core).
- Plus the repo's hard constraints: zero Cargo features, no oxigraph in `purrdf-core`, byte-deterministic output, fixed-key hashing, SPDX headers.

## Task 0: Worktree & Branch Setup (MANDATORY FIRST STEP)

**This task MUST be completed before any implementation work.**

1. Resolve the top-level repo (robust even if invoked from inside a worktree — the first `git worktree list` entry is always the main working tree):
   ```bash
   REPO_ROOT=$(git worktree list --porcelain | awk '/^worktree /{print ; exit}')
   BRANCH="paudley/30-shacl-af-completion"
   WORKTREE="$REPO_ROOT/.worktrees/30-shacl-af-completion"
   ```
2. Keep `.worktrees/` out of git locally (no tracked-file change):
   ```bash
   EXCL="$REPO_ROOT/.git/info/exclude"
   grep -qxF '/.worktrees/' "$EXCL" 2>/dev/null || echo '/.worktrees/' >> "$EXCL"
   ```
3. Create the worktree on a fresh feature branch off up-to-date main, then enter it:
   ```bash
   git -C "$REPO_ROOT" fetch origin
   git -C "$REPO_ROOT" worktree add "$WORKTREE" -b "$BRANCH" origin/main
   cd "$WORKTREE"
   git branch --show-current   # MUST show the feature branch, NOT main
   ```
4. Copy this plan into the worktree root so it is available after context is cleared and from inside the worktree:
   ```bash
   cp /home/paudley/.kimi-code/sessions/wd_purrdf_2f9e8775380a/session_a9919f41-3e9d-495c-b673-a257372e28e7/agents/main/plans/simon-baz-kid-flash-hawkgirl.md "$WORKTREE/PLAN.md"
   ```
5. Re-establish the baseline in the fresh worktree — worktrees do NOT inherit built artifacts. Build as the project requires and confirm green before implementing:
   ```bash
   make check
   make test
   ```
6. Post this plan to the issue:
   ```bash
   gh issue comment 30 --body-file "$WORKTREE/PLAN.md"
   ```

**⚠️ DO NOT proceed to Task 1 until the worktree exists, you are on the feature branch, and the baseline is green.**

## Task 1: Vendor DASH SHACL-AF validation tests into `vectors/shacl/af/`

Source the validation-scope AF tests from the pySHACL DASH corpus (`test/resources/dash_tests/`) and convert them to the W3C `sht:Validate` manifest format already used by `crates/shapes/tests/w3c_conformance.rs`.

Tests to vendor (validation-only; rules excluded):

- `dash_tests/expression/booleans-001.test.ttl` — `sh:expression` with `sh:this`, path expressions, and `sh:SPARQLFunction` calls.
- `dash_tests/function/callSPARQLFunction.test.ttl` — `sh:sparql` constraint calling a user-declared `sh:SPARQLFunction`.
- `dash_tests/function/simpleSPARQLFunction.test.ttl` — pure function test; convert to a validation test that uses the function inside an `sh:expression` constraint so the W3C harness can run it.
- `dash_tests/target/sparqlTarget-001.test.ttl` — `sh:SPARQLTarget`.
- `dash_tests/target/sparqlTargetType-001.test.ttl` — `sh:SPARQLTargetType` (parameterized custom target type).

Steps:

1. Create `vectors/shacl/af/expression/`, `vectors/shacl/af/function/`, `vectors/shacl/af/target/`.
2. For each DASH test, produce:
   - A single Turtle file containing the W3C manifest entry, the shapes graph, and the data graph (mirroring the combined-file style of the existing W3C suite), **or** separate `shapes.ttl` / `data.ttl` / `expected-report.ttl` files referenced by a manifest entry.
   - Strip the DASH-specific `dash:GraphValidationTestCase` / `dash:FunctionTestCase` metadata and replace it with `mf:Manifest` + `sht:Validate` + `mf:result`.
   - Preserve the original DASH test IRI as the entry IRI and add a `rdfs:comment` noting the pySHACL/DASH provenance.
   - Keep `example.org` namespaces where the original uses them; do not introduce `blackcatinformatics.ca` URIs.
3. Create `vectors/shacl/af/manifest.ttl` that `mf:include`s the three sub-manifests.
4. Update `vectors/shacl/af/README.md` (or create it) documenting:
   - Source: pySHACL DASH tests, commit hash, upstream URL.
   - License: pySHACL is Apache-2.0; DASH content from TopQuadrant — preserve any existing copyright/versionInfo comments verbatim.
   - Vendored subset and excluded content (rules out of scope).
5. Update the frozen-corpus manifest:
   ```bash
   python3 scripts/check-corpus-frozen.py --update
   ```
6. Run `cargo test -p purrdf-shapes --test w3c_conformance -- --nocapture` to confirm the new tests are discovered. They will fail at this stage because `sh:SPARQLTargetType` is not yet implemented; capture the failures and list them in `XFAIL` with precise reasons.

### After Task 1 Implementation:
- Commit: Use Agent tool with subagent_type: "coder" to commit.
- Push: Use Agent tool with subagent_type: "coder" to push.
- Post update to issue #30.

## Task 2: Extend the W3C conformance harness for the AF manifest tree

1. Update `crates/shapes/tests/w3c_conformance.rs`:
   - Bump `TOTAL_TESTS` to `120 + N` where `N` is the number of vendored AF `sht:Validate` entries.
   - Populate `XFAIL` with the tests that fail because the feature is not yet implemented (e.g., `af/target/sparqlTargetType-001`). Each entry must carry a concrete reason.
   - Ensure `XFAIL` assertions still enforce XPASS discipline.
2. Verify the harness discovers the AF sub-manifests and prints them in the scoreboard under sections `af/expression`, `af/function`, `af/target`.
3. Confirm the test fails RED as expected (new failing tests are ledgered, not silent).

### After Task 2 Implementation:
- Commit: Use Agent tool with subagent_type: "coder" to commit.
- Push: Use Agent tool with subagent_type: "coder" to push.
- Post update to issue #30.

## Task 3: Implement missing validation-only SHACL-AF features discovered by the new tests

Implement the AF gaps required to make the vendored validation tests pass. Likely gaps (to be confirmed by test failures):

1. **`sh:SPARQLTargetType`** — parameterized custom target types:
   - Parse `sh:SPARQLTargetType` declarations in `crates/shapes/src/shapes.rs` (parameters via `sh:parameter`, query body via `sh:select`, label template optional).
   - Extend the `Target` enum and shape-target resolution in `crates/shapes/src/engine.rs` so that a `sh:target [ a <CustomTargetType> ; <param> <value> ]` instantiates the query with the parameter values substituted/pre-bound and returns the resulting focus nodes.
   - Reuse the existing pre-binding infrastructure from `crates/shapes/src/sparql.rs`.
2. **SHACL-SPARQL annotation variables (`sh:annotationProperty`)** — if the vendored tests or spec examples exercise them:
   - Parse `sh:annotationProperty` declarations on `sh:SPARQLConstraint` / validators.
   - Project the annotated variables from solution rows and attach them to `ValidationResult`s (the harness does not compare them, but the engine should emit them for completeness).
3. **Any other validation-only AF gaps** surfaced by running the new tests (e.g., `sh:SPARQLFunction` edge cases, node-expression forms).

Rules (`sh:rule`, `sh:TripleRule`, `sh:SPARQLRule`) remain out of scope per issue #30.

After each fix, run:
```bash
cargo test -p purrdf-shapes --test w3c_conformance -- --nocapture
```
Remove fixed tests from `XFAIL`. The final state must have all AF validation tests passing or ledgered with a precise, justified reason.

### After Task 3 Implementation:
- Commit: Use Agent tool with subagent_type: "coder" to commit.
- Push: Use Agent tool with subagent_type: "coder" to push.
- Post update to issue #30.

## Task 4: Update conformance scoreboard and frozen-corpus metadata

1. Lower the SHACL W3C budget in `scripts/conformance-baseline.json` to match the new reality (the existing 6 `sparql/` xfails have already been fixed, so the budget should drop from `6` to `0`).
2. Regenerate the conformance matrix and scoreboard:
   ```bash
   python3 scripts/conformance-matrix.py --write-doc
   ```
3. Re-run the frozen-corpus checksum update if any vector changed:
   ```bash
   python3 scripts/check-corpus-frozen.py --update
   ```
4. Run the full local gate to ensure nothing regressed:
   ```bash
   make check
   make conformance
   ```

### After Task 4 Implementation:
- Commit: Use Agent tool with subagent_type: "coder" to commit.
- Push: Use Agent tool with subagent_type: "coder" to push.
- Post update to issue #30.

## Task N: Create Pull Request (MANDATORY FINAL STEP)

**This task MUST be the final step after all implementation is complete.**

1. Verify all changes are committed and pushed:
   ```bash
   git status
   git log --oneline -5
   ```
2. Create PR:
   ```bash
   gh pr create --title "feat(shapes): Complete SHACL-AF validation coverage" --body "Closes #30

   ## Summary
   Closes the SHACL-AF validation-scope epic by vendoring authoritative DASH AF tests into `vectors/shacl/af/` and implementing the remaining validation-only gaps (notably `sh:SPARQLTargetType`).

   ## Changes
   - Vendored DASH SHACL-AF validation tests (expression, function, target) into `vectors/shacl/af/`.
   - Converted DASH `.test.ttl` files to W3C `sht:Validate` manifest entries.
   - Extended `crates/shapes/tests/w3c_conformance.rs` to discover and gate the AF manifest tree.
   - Implemented `sh:SPARQLTargetType` support in shape parsing and target resolution.
   - Fixed any additional validation-only AF gaps surfaced by the new tests.
   - Updated `scripts/conformance-baseline.json` and `docs/CONFORMANCE.md` to reflect the now-zero SHACL W3C xfail budget.
   - Regenerated frozen-corpus checksums.
   "
   ```
3. Post plan to PR:
   ```bash
   PR_NUM=$(gh pr list --head $(git branch --show-current) --json number -q '.[0].number')
   gh pr comment $PR_NUM --body-file ./PLAN.md
   ```
4. Post the answers to these two questions to the PR **and** the issue:
   - What are you least confident about in this work and why?
   - What do I not know but should?
5. Report: "Stage 1 complete. PR #$PR_NUM created for issue #30. Ready for /stage2."

## Execution Rules (READ BEFORE STARTING)

1. **Task 0 (Worktree Setup) is MANDATORY** — Never skip, never work on `main`; all work happens inside the worktree `.worktrees/30-shacl-af-completion`.
2. **Commit after EACH task** — Use Agent tool with subagent_type: "coder" to commit, never batch.
3. **Push after EACH commit** — Use Agent tool with subagent_type: "coder" to push.
4. **Task N (PR Creation) is MANDATORY** — Never skip the final PR step.
5. **Use sub-agents** for all commits/pushes (never run git directly in main context).
6. **Stay inside the issue scope** — validation-only AF features. SHACL Rules, OWL/RDFS entailment, and triple materialization are explicitly out of scope.
7. **Preserve licenses and attribution** — vendored DASH tests keep their original copyright/versionInfo comments and the README documents provenance.
