# Prior-art inventory: issue #570: Submit W3C SHACL implementation reports (EARL) against the current test suites

Evidence: prior-art.md, references.md, references.json, brief.json; plus local reads of
`.stage/submit-w3c-shacl-implementation-reports/issue.md` (issue body, for step 3 wording) and
the worktree files listed below. No network or forge calls were made. Coverage, copied:
- issue #570 + 0 comment(s) via github
- link crawl: 4 issue(s) within 2 hop(s) of `#N` references
- forge search on ['Submit', 'SHACL', 'implementation']: 9 other item(s)
- GhpRsq-Defect-Class trailers: 5 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #570 (second parents walked), 0 other branch(es) named for it
- PRs citing #570: 0
- requirement candidates: 6 line(s); 0 comment(s) matched no rule
- referenced artifacts: 7 path(s) (3 on disk), 6 identifier(s), 0 workspace package(s), 3 URL(s)

## Referenced artifacts
Repository root: `/home/paudley/Active/purrdf/.worktrees/570-submit-w3c-shacl-implementation-reports`.
The first seven rows come from references.md; rows marked "local check" were read from the worktree for this inventory and are not in references.md.

| Reference | Resolved | Exists | Lines | Licence header | Symbol ranges |
|-----------|----------|--------|-------|----------------|---------------|
| `shacl12-test-suite/reports/` | repo (not elsewhere) | NO | — | — | — |
| `data-shapes-test-suite/reports/` | repo (not elsewhere) | NO | — | — | — |
| `vectors/shacl12/` | repo | yes (dir: `tests/`, `vocabularies/`, `LICENSE.md`, `PROVENANCE.md`) | — | — | — |
| `shacl12-test-suite/tests/` | repo (not elsewhere) | NO | — | — | — |
| `scripts/vendor-shacl12.py` | repo | yes | 267 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 | — |
| `vectors/shacl/` | repo | yes (dir: `af/`, `core/`, `sparql/`, `README.md`, `manifest.ttl`) | — | — | — |
| `data-shapes-test-suite/tests/` | repo (not elsewhere) | NO | — | — | — |
| `vectors/shacl12/PROVENANCE.md` (local check) | repo | yes | — | not checked | — |
| `crates/conformance-kit/src/outcome.rs` (local check) | repo | yes | 190 | not checked | `to_earl`: 133-190 (`pub fn to_earl(` at 133; grep `^}` gives 190) |
| `crates/sparql-conformance/src/bin/community-conformance.rs` (local check) | repo | yes | 68 | not checked | `to_earl` call at 37; writes `earl.nt` at 40 |
| `crates/shapes/tests/w3c12_conformance.rs` (local check) | repo | yes | 1760 | not checked | none: grep for `to_earl` and `earl` (case-insensitive word match) found no hits |
| `crates/shapes/tests/w3c_conformance.rs` (local check) | repo | yes | 198 | not checked | none: same grep, no hits |
| `docs/CONFORMANCE.md` lines ~120-130 (local check) | repo | yes | — | not checked | scoreboard table; SHACL 1.2 row cites `vectors/shacl12/tests/` (538/544 pass as approved); no EARL mention in the file |

Repository licence (for comparison): `Cargo.toml:61` `license = "MIT OR Apache-2.0 OR MulanPSL-2.0"`; `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-MULAN` exist at the root. `scripts/vendor-shacl12.py` header matches this licence.

### History (from references.md)
- `vectors/shacl12/`: `ae645efe7 test(shapes): vendor the W3C SHACL 1.2 vocabularies and test suite`
- `scripts/vendor-shacl12.py`: `3525eeb84 feat(shapes)!: result annotations, sh:minus, and the remaining SHACL terms`; `11f54933b feat(cli,python,wasm,capi): rules, node expressions and shapes lint on every host`; `ae645efe7 test(shapes): vendor the W3C SHACL 1.2 vocabularies and test suite`
- `vectors/shacl/`: ten commits, `ab5b27ccb` (newest) through `1dd87380c` (oldest), as listed in references.md

## Existing EARL-producing code and callers
- Definition: `purrdf_conformance_kit::outcome::to_earl` at `crates/conformance-kit/src/outcome.rs:133-190`.
- Caller 1 (production bin): `crates/sparql-conformance/src/bin/community-conformance.rs:37` calls `to_earl(&records, ASSERTED_BY, SUBJECT)` and writes `earl.nt` (line 40) next to `records.json`. The file header (line 9) describes `earl.nt` as the same verdicts as EARL assertions. `corpora/community/README.md:33` documents `earl.nt` as a public output.
- Caller 2 (tests): `crates/conformance-kit/tests/grading.rs` lines 581, 590, 598, 606.
- The Rust W3C harnesses named by the issue (`w3c12_conformance.rs`, `w3c_conformance.rs`) contain no `to_earl` or `earl` text. The issue (step 3) asks them to emit through `to_earl`. Whether that is new work is a planning question (see NEEDS-JUDGMENT 2).
- Count discrepancy: references.md lists `purrdf_conformance_kit::outcome::to_earl`: 1 file, which is the fully qualified path only. A bare `to_earl` search in the worktree hits three files (outcome.rs, grading.rs, community-conformance.rs).

## Identifiers (copied from references.md)
- `w3c12_conformance`: 15 file(s)
- `w3c_conformance`: 9 file(s)
- `purrdf_conformance_kit::outcome::to_earl`: 1 file(s)
- `passed`: 348; `failed`: 617; `inapplicable`: 13 (file counts, not EARL-specific)

## Prior attempts at this issue
- Commits: none found (brief: 0 commit(s) citing #570, second parents walked).
- Branches: none (brief: 0 other branch(es) named for it). The worktree's own branch `paudley/570-submit-w3c-shacl-implementation-reports` exists locally and is the current branch, so it is not an earlier attempt.
- PRs: none found (brief: PRs citing #570: 0). Forge search was not re-run for this inventory.

## Linked issues
| # | Relation | State | Title |
|---|----------|-------|-------|
| 384 | mentions | OPEN | Build a portable adversarial SHACL suite and contribute missing W3C tests |
| 382 | 2-hop | CLOSED | SHACL pre-binding leaves `$this` unbound in a triple pattern inside `OPTIONAL` |
| 383 | 2-hop | CLOSED | Read one named graph of a native dataset as the default graph |
| 385 | 2-hop | CLOSED | An alternative path of plain predicates is not translated, and a UNION in its place joins by position |

Related items from the forge search (no relation stated): #490 (CLOSED), #387 (CLOSED), #323 (CLOSED), #375 (CLOSED), #130 (CLOSED), #339 (CLOSED), #330 (CLOSED), all as listed in prior-art.md.

## Governing ADRs
- NOT FOUND. The issue cites no ADRs (brief `adrs: []`).

## Recurrence
Counted from `GhpRsq-Defect-Class` trailers (repository-wide, last 200 commits; not specific to #570):
- `none`: 20
- `performance`: 3
- `bug`: 2
- `correctness`: 1
- `semantic-identity-loss`: 1

## NEEDS-JUDGMENT
1. Four referenced paths do not exist in this repo and are not marked elsewhere-only: `shacl12-test-suite/reports/`, `shacl12-test-suite/tests/`, `data-shapes-test-suite/reports/`, `data-shapes-test-suite/tests/` (references.md rows; no sibling checkout at `../../` either). The first two are named as report output locations in the issue; the last two are upstream suite locations the issue's step 1 asks to compare against `vectors/shacl/`. Question: should the planner treat the reports paths as files to create here, and the `tests/` paths as an external checkout the plan must obtain (and from where)?
2. Issue step 3 (issue.md) says `w3c12_conformance` and `w3c_conformance` emit through `purrdf_conformance_kit::outcome::to_earl`, but neither file contains `to_earl` or `earl` text (local grep). Existing callers are `community-conformance.rs:37` and `grading.rs:581-606`. Question: is wiring the two W3C harnesses to `to_earl` new code to be planned, and should the planner reuse the `community-conformance.rs` pattern?
3. Issue step 1 asks to check whether `vectors/shacl/` matches `data-shapes-test-suite/tests/`. That sibling path does not exist here (row 7 above), so the comparison is UNKNOWN from this worktree. Question: how should the planner source the upstream tree for that comparison?
4. Issue step 1 also asks to re-vendor `vectors/shacl12/` to the current `gh-pages` commit with `scripts/vendor-shacl12.py`. The script is present (267 lines, header MIT OR Apache-2.0 OR MulanPSL-2.0, same as the repository). The vendored suite carries its own `vectors/shacl12/LICENSE.md` and `LICENSES/W3C-test-suite-license.html`. Question: does the planner want the licence status of the vendored W3C suite recorded alongside the published EARL output? This inventory does not assess it.
5. Issue #384 (OPEN, relation "mentions") covers adversarial suite work and missing W3C tests. Question: does the planner need #384's scope settled before step 2 (fixing or WG-referring failing approved entries), or is it independent?
