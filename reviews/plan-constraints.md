# Plan constraints review, pass 2: submit-w3c-shacl-implementation-reports

Reviewed plan: `/home/paudley/Active/purrdf/.worktrees/570-submit-w3c-shacl-implementation-reports/.stage/submit-w3c-shacl-implementation-reports/plan.md` (revision 2)

Lens: does the plan break a rule that this repository, the W3C workspace, or the owner has written down?

## Governing documents read

| Document | Status |
|---|---|
| `AGENTS.md` (worktree): crate map, §2 hard constraints, §6 releases | read |
| `CLAUDE.md` (worktree) | read |
| `.goals` | gitignored; read from `/home/paudley/Active/purrdf/.goals` |
| `.baseline` | gitignored; read from `/home/paudley/Active/purrdf/.baseline` |
| `CONTRIBUTING.md` | read |
| `helpers-ledger.toml` (`w3c-vocab` :733-763, `test-temporary-paths` :537-551; no EARL or report job) | read |
| `layers.toml` header (:4-39) | read |
| `LICENSING.md` vendored-tree table (:79-119) | read |
| `Makefile` (`.PHONY` :54-55, `help` :77-78, `check` :172, `release-tags` :217-225) | read |
| `scripts/vendor-shacl12.py`, `scripts/check-corpus-frozen.py`, `scripts/conformance-frozen/roots.toml` | read |
| `scripts/release-crates.sh` (:1-50), `scripts/check-publish-order.py`, `scripts/check-versions.py` (:197-221) | read |
| Root `Cargo.toml` `[workspace.dependencies]` (`purrdf-conformance-kit` :86) | read |
| `crates/conformance-kit/Cargo.toml`, `crates/iri/src/vocab.rs` (`mf`, `sht`, `earl`) | read |
| `vectors/shacl/README.md` | read |
| `CONSTITUTION.md`, `PRECOMMIT_GUIDANCE.md`, `docs/adr/`, `docs/decisions/` | absent |
| `/home/paudley/Active/w3c/AGENTS.md`, `docs/contributing.md`, `data-shapes/CONTRIBUTING.md` | read |
| `$STAGE/issue.md`, `$STAGE/issue-analysis.md` | read |
| User global rule: no agent attribution or trailers in commits | applied |

## User decisions in force

- **U1** (quoted at `plan.md:56-69`): tagging and the W3C PR are outside #570, and Patrick authorized editing #570's text to match. This overrides a `.baseline` default (`.baseline:3`, "No DEFFERALS") and the issue's acceptance line "Both reports are generated from a tagged release and submitted upstream" (`issue.md:60-61`). I report it as a decision, not a silent clearance. The plan is compliant with it: Task 10 no longer tags, and the two publication steps are recorded as open and never claimed (`plan.md:65-66`, `:266`).
- **U2** (quoted at `plan.md:70-73`): a mechanical 2017 inventory check. The plan follows it (R32/R33 2017 row, Task 9). How the inventory file reaches the repository is the subject of N1 below.

---

## Pass-1 disposition

| Pass-1 finding | Status | Evidence |
|---|---|---|
| **B1** G1 reported-failed cases had no harness home | **DISCHARGED** | `plan.md:93-98`: a count-pinned `SPEC_CONTRADICTED` table, separate from `XFAIL`. Its entries are graded unamended, refused on XPASS, and emitted `earl:failed` with an `earl:info` rationale. The repository gets no WG issue number or URL. `XFAIL` stays empty (R7/R9/R10 row, `:255`). This keeps the exact-count and XPASS discipline of `AGENTS.md:321-322` and the "no process text" rule of `.baseline:5`. My pass-1 rewrite said not to add the table if it stayed empty. The plan adds it empty (`:415`). That breaks no written rule, so I accept it. The interpretive question is assessed separately under P1. |
| **B2** generating from a tag before the PR merges | **DISCHARGED by user decision U1** | Tagging is now outside #570 (`plan.md:63-64`, `:44-45`). `make w3c-earl` builds from `git rev-parse HEAD` on a clean tree (G7), and "A later tag build runs the same target" (`:139`). R22/R23/R34/R35 are reported open up to the two external steps (`:266`, `:469`). Nothing now requires tagging from a non-`main` commit, so `docs/RELEASE.md`'s `main`-only rule (enforced at `Makefile:220`) is not engaged. |
| **A1** undeclared dev-dependency, stale release docs | **STILL-OPEN (narrowed)** | G15 (`plan.md:195-199`) now states the dev-only edge and keeps EARL code in `tests/`. I verified the root entry is versionless (`Cargo.toml:86`: `purrdf-conformance-kit = { path = "crates/conformance-kit" }`), so cargo strips it at publish. The edge passes `check-publish-order.py`, because the kit is not in the release set (`check-publish-order.py:134`, `graph & listed`). `layers.toml:7-8` exempts dev edges. Still open: G15 says "*If* `release-crates.sh` carries a dev-dependency stripping list". It does carry that text (`release-crates.sh:20-25`), and so does `AGENTS.md:471-475` §6, which G15 does not mention. See A1' below. |
| **A2** DOAP/FOAF kept out of kit `src/` | **DISCHARGED** | G10 (`plan.md:151-163`): the kit's `src/` names only EARL and RDF terms; kit tests use `example.org`; DOAP, FOAF, the homepage and the assertor IRI are spelled only in the two harness test files. FOAF and DOAP are not modules of `purrdf_iri::vocab` (`vocab.rs` modules end at `earl`, `:1634`), so literals in test code do not trip `w3c-vocab`. `AGENTS.md:331-333` forbids a `blackcatinformatics.ca` namespace only in library code, so `https://github.com/Blackcat-Informatics/purrdf` in a test file is compliant. |
| **A3** serialization form and golden | **DISCHARGED** | G13 (`plan.md:180-188`): sorted N-Triples, double generation byte-identical, `assert_golden!` on `to_earl`, and the `earl.nt` byte change stated in the PR (`:470`). This satisfies `AGENTS.md:304-306`. |
| **A4** refusal neighbours in Tasks 3, 5, 6, 7 | **DISCHARGED** | The binding rule (`plan.md:27-28`) applies to every task. The rest is covered by G1 (3), Task 2 (`:346-348`), Task 3 (`:355-356`), Task 4 (`:368-369`), Task 5 (`:378`) and Task 6 (`:385-386`). Task 7 adds no refusal. |
| **A5** conventional commits for every task | **DISCHARGED** | Tasks 2-10 and the PR title each name a `type(scope): summary` message (`plan.md:339`, `:352`, `:365`, `:373`, `:382`, `:390`, `:403`, `:428`, `:439`, `:463`). None carries an issue number or attribution. |
| **A6** interim `docs/CONFORMANCE.md` process text | **DISCHARGED** | G16 (`plan.md:203-204`): "current counts only, with no process text". |
| **A7** W3C workspace layout | **DISCHARGED, one residual (A7')** | Task 9 uses a worktree under `worktrees/data-shapes/` at `4cd46986`, with the base passed explicitly, and never touches the `data-shapes/` base (`plan.md:48-49`, `:430-433`). Drafts go in `contributions/data-shapes/purrdf-earl/drafts/` (`:50`, `:448-449`). The W3C PR and its contributor identification (`data-shapes/CONTRIBUTING.md:8-9`) are outside scope by U1. The residual is claim labeling (A7'). |
| **A8** testkit temp paths for disk checks | **DISCHARGED, one new site (N4)** | G13 makes double generation in-memory (`plan.md:183`), and writes happen only under `PURRDF_W3C_EARL_DIR` (`:187-188`). The new write-error test in Task 8 (`:421`) is a new filesystem site; see N4. |

---

## BLOCKING findings

### N1. Task 9 vendors `alltests.ttl` "via the vendor path", but no compliant vendor path exists for that file. Every literal reading of the plan breaks a hard constraint.

**Governing text**
- `AGENTS.md:221-223`: "**Tooling and tests are Rust.** Checks, generators, gates and tests are written in Rust … Python and JavaScript remain only where a language surface itself must be exercised."
- `AGENTS.md:243-246`: "**Ratchet.** A path that exists in BASE under `scripts/` … may not hold more non-Rust lines in TARGET than in BASE". The section heading, `AGENTS.md:213`, reads "violating these fails CI or review".
- `CLAUDE.md:14`: "**Never**: … hand-edit `generated/` or `vectors/`".
- `scripts/check-corpus-frozen.py:53-55`: "adding a new root is a deliberate edit to that registry followed by `--update` — a corpus is NEVER guarded until it appears here."
- `LICENSING.md:99-104`: "Each tree's own `LICENSE`, `LICENSE.md`, `LICENSES/`, `PROVENANCE.md`, or `README.md` is authoritative for its upstream terms, source URL, and pinned revision".
- The plan's own binding line, `plan.md:31-32`: "`scripts/vendor-shacl12.py` changes only its pin line."

**Offending plan lines**
- `plan.md:434-435` (Task 9): "**2017:** vendor `alltests.ttl` via the vendor path, pin its hash, and run `w3c_2017_earl_matches_wg_inventory`."
- `plan.md:271` (R32/R33 2017): "a vendored copy of that file, pinned by hash".
- `plan.md:253` (R3): "`scripts/conformance-frozen/vectors-shacl.sha256` (unchanged)".

**Why this blocks**
- **The upstream file is outside every vendored prefix.** It lives at `data-shapes-test-suite/reports/alltests.ttl` (confirmed in `/home/paudley/Active/w3c/data-shapes`).
- **The only data-shapes vendor tool can't reach it without growing.** `scripts/vendor-shacl12.py` extracts only `shacl12-vocabularies/` and `shacl12-test-suite/tests/` (`:65-66`). Adding a prefix adds Python lines to a BASE path under `scripts/`, which fails the ratchet in CI. It also contradicts the plan's own `:31-32`.
- **The 2017 suite has no vendor script at all** (`vectors/shacl/README.md:11-17` records provenance only).
- **The remaining routes also break rules.**
  - A new Python fetcher breaks `AGENTS.md:221-223`; vendoring is tooling, not a language surface.
  - Copying the file into `vectors/shacl/` by hand breaks `CLAUDE.md:14`. It would also change `vectors-shacl.sha256`, which makes R3's "(unchanged)" false.
- **The plan says it may not be in the repository at all.** Task 9's preamble ("No source commit unless the 2017 test lives in the repository", `:427-428`) leaves even that open.

**REWRITE TO** (Task 9, 2017 bullet, and the R32/R33 2017 row). Choose one option and state it:

> **Option A, in-repository.**
> - The 2017 WG inventory is acquired by Rust code, not Python. For example, add a small `vendor` subcommand or example in an unpublished host crate. It fetches `data-shapes-test-suite/reports/alltests.ttl` at the pinned `gh-pages` commit and writes it verbatim.
> - The file goes into a new root outside `vectors/shacl/`, for example `vectors/shacl-reports/`, alongside a first-party `PROVENANCE.md` that names the upstream path, the commit and the W3C Software and Document License, plus a verbatim `LICENSE.md`.
> - The root is registered in `scripts/conformance-frozen/roots.toml` with its own `.sha256` receipt, produced by `check-corpus-frozen.py --update`.
> - A row is added to the `LICENSING.md` vendored-tree table.
> - `vectors-shacl.sha256` stays unchanged (R3 holds).
> - `scripts/vendor-shacl12.py` is not touched beyond its pin.
> - `w3c_2017_earl_matches_wg_inventory` is a Rust test in `crates/shapes/tests/`.
>
> **Option B, W3C workspace.**
> - The 2017 inventory check runs in `/home/paudley/Active/w3c/` against the Task 9 worktree's own `data-shapes-test-suite/reports/alltests.ttl` at `4cd46986`.
> - The tool lives in `contributions/data-shapes/purrdf-earl/`.
> - The evidence goes to `validation.md`.
> - Nothing is vendored into PurRDF, and the R32/R33 (2017) row names that check instead of a repository test.

---

## ADVISORY findings

### P1. `SPEC_CONTRADICTED` against the issue's "don't … ledger the test away": the plan flags its reading honestly, but nothing makes Patrick actually rule on it.

**Governing text**
- `issue.md:32-33`: "Report it as failed until the WG settles it; don't amend the expectation or ledger the test away."
- `issue.md:50-55`: the acceptance criteria require both "`make check` and `make conformance` pass" and that a contradicted entry "is reported as failed with a WG issue".
- `.baseline:5`: no issue numbers and no process text in the repository.

**Assessment**
- **The issue gives no way to meet both demands without some list in the harness.** It wants green gates and also wants failing entries reported failed. With the harness's exact-count assertions in place (`AGENTS.md:321-322`), the only way to have both is a pinned list.
- **The plan's list is unlike `XFAIL`.**
  - It never relabels a failure: the scoreboard label is its own and EARL emits `failed` (`plan.md:96-98`).
  - Entries are graded against the unamended expectation.
  - A pass is an error.
  - Each entry needs a quoted normative sentence and a drafted WG issue before it qualifies (G1 (1)-(4)).
- **"Hide or pass" is a defensible reading.** The plan states it in so many words: "That reading is Patrick's to override" (`plan.md:99-102`). It does not clear the question silently. On that test the flag is honest.
- **The gap is that no step asks for the ruling.**
  - The plan never asks Patrick to confirm the reading.
  - Task 11 step 1's authorized issue edit covers "U1 and U2" only (`plan.md:456`). Rewording R10's sentence to fit the plan's reading would exceed that authorization.
  - If the table stays empty, the question is moot. If an entry is added, the plan relies on an interpretation the owner has not confirmed.

**REWRITE TO** (G1 "Patrick's visibility" and Task 11 step 1):
> Before any case is added to `SPEC_CONTRADICTED`, Patrick is asked to confirm or reject the reading of R10 in so many words. His answer is recorded as a quoted decision, U3. If he rejects it, the affected case is not tabled; the harness and gates outcome is put back to him as a scope question.
>
> If the table is still empty at Task 11, the question is reported as moot.
>
> Task 11 step 1 edits #570's text only as U1 and U2 authorize. It does not reword R10 unless U3 authorizes it.
>
> Each table rationale is a clause citation only: no "until", "pending", issue number or URL. It must pass `make check-issue-refs` (`Makefile:177-178`).

### A1'. Make G15's documentation update unconditional, and include `AGENTS.md` §6.

**Governing text**
- `AGENTS.md:471-475`: "`purrdf-alloc-probe` and `purrdf-testkit` are dev-dependencies of published crates, so their root `[workspace.dependencies]` entries are path-only with **no `version`**".
- `scripts/release-crates.sh:20-25` says the same, and `:15-18` lists the members kept out of the release set, but that list already omits `purrdf-conformance-kit`.
- `.baseline:7`: "when you see it, you own it".

**Offending plan line**
- `plan.md:198-199`: "If `release-crates.sh` carries a dev-dependency stripping list, it is updated."

**REWRITE TO:**
> Add `purrdf-conformance-kit` to:
> - the dev-dependency sentence of `AGENTS.md` §6 (`:471-475`);
> - `scripts/release-crates.sh:20-25`;
> - the not-released list at `release-crates.sh:15-18`.
>
> In each place, say that its root entry must stay versionless. The `AGENTS.md:78` crate-map row names `purrdf-shapes`'s harnesses as a caller.

### N2. The six claimed conformance-class IRIs are not in `purrdf_iri::vocab::sht`, so pinning them in harness code will trip `w3c-vocab`.

**Governing text**
- `AGENTS.md:334-337`: "W3C Recommendation terms … name them through `purrdf_iri::vocab` … never as string literals — the helper census (`w3c-vocab`) refuses a literal that spells a term or namespace".
- `helpers-ledger.toml:733-746`: `w3c-vocab`, `enforced = true`, `rule:vocabulary-literal`.
- `AGENTS.md:80`: the census covers "every test, bench and example target".

**Offending plan lines**
- `plan.md:113-115` (G5): the claimed set is pinned as "Core, SPARQLValidation, NodeExpr, InferenceRules, SPARQLNodeExpr and SPARQLCustomFunctions".
- `plan.md:162-163` (G10): only `TestSubject`, `Software`, `Assertor` and `info` are added, all to `vocab::earl`.

The vendored manifests use `mf:requires sht:Core`, `sht:NodeExpr` and so on (for example `vectors/shacl12/tests/core/validation-reports/shared.ttl:26`). `vocab::sht` (`vocab.rs:1594-1631`) holds none of these class terms. `mf:requires` and `mf:status` are already present (`vocab.rs:1534-1537`).

**REWRITE TO** (G5 and the Task 8 kit bullet):
> Add the `sht:` conformance-class terms that Task 1 confirms from discovery to `purrdf_iri::vocab::sht`, each with a doc comment. The harness pins the claimed set through those constants.

### N3. Make the `make w3c-earl` target follow the Makefile's own conventions.

**Governing text**
- `Makefile:54-55`: every target is listed in `.PHONY`.
- `Makefile:77-78`: `help` lists only targets that carry a `## ` description.
- `Makefile:172-173`: cargo is invoked `--locked`.
- `Makefile:219`: the dirty-tree precedent is `git status --porcelain`.

**Offending plan lines**
- `plan.md:441-445` (Task 10 Makefile bullets).

**REWRITE TO:**
> Add `w3c-earl` to `.PHONY` with a `## ` description. It refuses a dirty tree with the same `git status --porcelain` test as `release-tags`. It invokes `cargo test --locked -p purrdf-shapes --test w3c12_conformance --test w3c_conformance`.
>
> It sets `PURRDF_W3C_EARL_DIR` to a directory under the cargo target directory, which is ignored. A second run then does not dirty the tree, and no EARL file is ever committed to PurRDF.
>
> The recipe holds no `python3` invocation.

### N4. The new write-error test must use testkit temporary paths.

**Governing text**
- `helpers-ledger.toml:537-546`: `test-temporary-paths`, home `purrdf_testkit::TempDir`, entry points `temp_dir`/`temp_file`, `enforced = true`.

**Offending plan line**
- `plan.md:421`: "a write-error test".

**REWRITE TO:**
> The write-error test provokes the failure inside `purrdf_testkit::temp_dir!`/`temp_file!`, for example by setting `PURRDF_W3C_EARL_DIR` to a path whose parent is a regular file. It writes nowhere else.

### A7'. Label the evidence type in WG issue drafts.

**Governing text**
- `/home/paudley/Active/w3c/AGENTS.md:37-39`: "Keep claims tied to actual evidence … distinguish syntax, graph equivalence, implementation observations, specification derivation, and Working Group acceptance."

**Offending plan line**
- `plan.md:448-450`: Task 10 WG-issue drafts.

**REWRITE TO:**
> Each draft labels:
> - the quoted normative sentence as a specification derivation;
> - PurRDF's output as an implementation observation;
> - the approved expectation as WG-approved test data, not as WG acceptance of PurRDF's reading.

### N5. `vectors/shacl/README.md` states stale counts that Task 1 should correct.

**Governing text**
- `.baseline:7`: "On bugs - when you see it, you own it".
- `.baseline:5`: documentation carries no process text.

**The stale text**
- `vectors/shacl/README.md:31-32`: "the seam contributes **6** `sht:Validate` entries of the harness's 126 total".
- The plan measures 9 `af/` entries and 129 total (`plan.md:245`).

The README is a first-party sidecar outside the freeze (`check-corpus-frozen.py:98`), so editing it needs no `--update`.

**REWRITE TO:** add to Task 1's baseline record:
> Correct the `af/` and total counts in `vectors/shacl/README.md` to the measured values, with no process text.

---

## Checked and compliant (new material)

- **The `Report` type in `purrdf-conformance-kit`**:
  - One home: `to_earl` is the only EARL writer (callers are `community-conformance.rs`, `grading.rs` and `outcome.rs`), and `helpers-ledger.toml` has no competing report or scoreboard job. G11 extends it in place and migrates `community-conformance`.
  - The kit is `publish = false` (`crates/conformance-kit/Cargo.toml:13`), and its normal dependencies do not include `purrdf-shapes`, so the new dev-edge closes no normal-edge cycle.
- **The dev-dependency** `purrdf-shapes` → kit is fine under `layers.toml:7-8`, `check-versions.py:221` and `check-publish-order.py:134`, and it satisfies `AGENTS.md:340-342` (`workspace = true`, versionless root). The wasm build is unaffected, because dev-dependencies are not built for the library.
- **DOAP and FOAF in harness test code only**: compliant (see A2). The Rust port of `testKey` spells the WG's own `urn:x-shacl-test:` key scheme in test code, which mints no PurRDF vocabulary (`CLAUDE.md:20-22`).
- **EARL vocabulary additions** (`TestSubject`, `Software`, `Assertor`, `info`) extend `vocab::earl` (`vocab.rs:1634-1665`). That is the `w3c-vocab` home.
- **Build-level hard fails** (`.goals:6`): a malformed or unpaired `PURRDF_W3C_EARL_COMMIT`/`PURRDF_W3C_EARL_DIR` is a hard failure (G7).
- **Intermediate red commits (G16)**: the hooks run rustfmt, the ratchet and the fast hygiene gates, not the conformance harness (`AGENTS.md:251-253`). No hook bypass is needed, and none is planned. `CONTRIBUTING.md:31-32` and `:64-66` are met at the PR, through Task 11 step 3.
- **Licensing (G14)**: no relicensing. The existing `LICENSING.md:90` and `:92` rows cover `vectors/shacl/` and `vectors/shacl12/`. A new root needs its own row (N1 Option A).
- **W3C workspace approval gates**: every WG-facing issue, comment and push is gated on Patrick's approval of the concrete text (`plan.md:46-50`, `:333-335`, `:448-450`), which matches `/home/paudley/Active/w3c/AGENTS.md:10-16`.
- **No issue numbers and no attribution in repository text or commits**: `Closes #570` and the reference to the community-corpus issue appear only in GitHub text (`plan.md:210`, `:466`). The `.stage/` directory is excluded from git. The assertor is Patrick Audley, and no agent is credited.

VERDICT: BLOCKED
