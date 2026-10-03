## Issue summary

Investigate blank-node and hidden-binding scope hazards with executable evidence, compare three representation/checking designs, and recommend a bounded design supported by mutation, semantic and cost measurements. Deliver the complete investigation on this branch. Preserve the existing alternative-path correction and frozen official corpora.

## Baseline defaults

Adversarial gap analysis; fix observed defects; no incomplete requirements; keep issue/PR references and process history in GitHub rather than repository documentation. `.baseline` is loaded from the repository root.

## Standing constraints

`.goals`: greenfield-first design, Rust core, explicit hard failures, RDF 1.2, maximal utility/performance/portability. `AGENTS.md`: one authoritative implementation per shared job, deterministic outputs, stack-safe algebra walks, wasm support, inherited lints, source licensing, unchanged frozen corpora and uncompromised commit hooks. `CONSTITUTION.md`: None.

## Completeness contract

| Requirement / acceptance criterion | Task |
| --- | --- |
| Distinguish user variables, BGP existentials, path witnesses, template allocations and concrete dataset blanks | 1, 3 |
| Binding/scope invariants and minimized legal/illegal hazard taxonomy, including alternative/sequence crossing UNION | 1, 2, 3 |
| Compare existing identities plus validation, explicit binder/scope identities, transformation certificates | 1, 3 |
| Compare correctness, compatibility, memory/layout, preparation cost, reuse and wasm implications | 1, 3, 4 |
| Intentional mutations: capture, escape, lost connection, alpha-renaming, template freshness, early witness projection; false-positive controls and static limits | 1, 2, 3 |
| Contracts for split/merge, distribution, substitution, projection, serialization and repeated normalization | 1, 2, 3 |
| Collision-free SERVICE/carrier naming, roundtrip identity, duplicate bags, visible columns and generated-name lookalikes | 2, 3 |
| Interaction matrix: UNION, paths/inverses, OPTIONAL, MINUS, EXISTS, GRAPH, subqueries, BIND/VALUES, property functions, prepared reuse, UPDATE/CONSTRUCT and quoted terms | 2, 3 |
| Seeded property generation and shrinking across scope interactions, alongside intentional mutations | 1, 2 |
| Recommended bounded design, authoritative invariant home and deterministic typed actionable diagnostics | 1, 3 |
| Define checks at parser, raw/compiler algebra, transform, carrier/reparse and egress boundaries | 1, 2, 3 |
| Measure representative and deeply nested checks, layout and preparation/reuse costs with reproducible inputs | 3, 4 |
| Portable normative cases coordinated with the community-suite issue; internal invariant/budget evidence separate; frozen official corpora preserved | 2, 3, 4 |
| Independent plan/compliance/enhancement review, isolated worktree, task commits/pushes/updates, full gates, PR, CodeRabbit remediation and structured merge | 0–6 |

No blocked requirement is currently identified. Each experimental detector will explicitly state what its inputs establish; detection rates will not be presented as proof of general query equivalence.

## Enhancements considered and declined

Adopted transformation (M): study provenance-bearing binding-incidence contracts rather than textual blank-name equality. A single resulting tree cannot prove a missing intended connection; the comparison therefore separates static role/scope admission from before/after identity obligations and runtime bag/freshness controls.

Adopted leverage (S): one typed borrowed checker in `purrdf_sparql_algebra::scope` backs the existing hidden-observer validation paths and uses the exhaustive iterative algebra walk. Typed diagnostics carry the binding, observer role and deterministic location rather than requiring consumers to match prose. Reuse existing preparation, carrier, result and template controls.

Adopted utility/robustness (S/M): one shared dev-only candidate evidence implementation at `crates/sparql-algebra/tests/support/scope_candidates.rs`; a machine-readable candidate/mutation matrix distinguishing detected, accepted legal controls and claims not established from inputs; legal independent-blank controls; duplicate UNION-arm and alpha-renaming controls; profile-labelled portable fixtures; deep-query measurement; explicit evidence limits and deterministic reruns. Freeze contract preconditions before mutation and include repeated template execution and deep raw-algebra controls.

Declined wholesale AST identity replacement (L): the issue asks for a measured investigation, and the current Arc-backed unspellable identities already express the immediate path/UNION connection. A representation-wide replacement before evaluating provenance and transformation obligations would add a breaking change without establishing the missing proof. The branch delivers the three-candidate comparison and a complete, bounded recommended design instead.

Declined a public experimental certificate graph (M): publishing speculative graph types would freeze a contract before its mutation evidence is reviewed. Candidate graph types remain executable research support; the concrete typed observer checker is reusable production API.

## Task 0: worktree and branch setup

Verify repository identity, live issue/body/comments, baseline/goals, deficiency ledger and `ghprsq`. Fetch origin. Create `.worktrees/387-blank-scope-investigation` on `paudley/387-blank-scope-investigation` from `origin/main`; preserve the unrelated local-main commit and sibling worktrees. Verify worktree instructions and baseline prerequisites. Post the independently reviewed plan to the issue.

## Task 1: executable candidate and mutation evidence

Implement the small typed borrowed checker in `crates/sparql-algebra/src/scope.rs` and route existing observer rules through that authoritative home, preserving public validation entry points and valid algebra semantics. Implement the single dev-only Rust evidence home `crates/sparql-algebra/tests/support/scope_candidates.rs`, consumed by the `scope_candidates` integration suite, `scope_checks` benchmark and `scope_investigation` report example. Use typed semantic categories, scoped identities and actionable diagnostics. Exercise the existing validator, explicit binder/scope evidence and before/after binding-incidence contracts against the same intentional mutations and legal controls. Use seeded property generation and shrinking from `purrdf_testkit` for scope-interaction variations. Include provenance requirements, multiset/branch sensitivity, consistent alpha-renaming and repeated transformations; demonstrate claims requiring additional input honestly with semantic experiments. Contracts compare stable source-site binding partitions under explicit correspondence, visible schema and branch multiplicity, with legal reassociation/distribution/bijective-renaming controls; raw tree hashes or occurrence-count equality are insufficient. Freeze contract inputs before mutations and emit the candidate/mutation matrix. Run `cargo test -p purrdf-sparql-algebra --test scope_invariants --test scope_candidates`, commit with hooks, push, and post results plus commit to the issue.

## Task 2: independent semantic and portable controls

Build minimized data/query/expected-result cases with specification rationales and a category/profile matrix. Validate native/prepared paths, result bags and columns, query/template/dataset blank distinctions, carrier/reparse aliasing and RDF 1.2 quoted terms, including freshness across repeated template executions. Cover each listed interaction, including legal controls and required rejection where profiles forbid constructs. Reuse existing shared harnesses and add no edits to frozen corpora. Run integration review and `cargo test -p purrdf-sparql-eval --test scope_interactions`, commit with hooks, push, and post results plus commit. Coordinate the portable cases on the community-suite issue.

## Task 3: measured investigation and recommended design

Measure candidate preparation/checking/reuse, memory/layout and representative/deep query costs using the first-party benchmark harness and reproducible inputs. Document the taxonomy, candidate comparison, observed mutation matrix and limits, boundary validation design, normative rationale and complete bounded implementation plan. Keep process references out of repository docs. Record hardware/compiler and measurement parameters; label implementation invariants separately from normative conformance. Validate report evidence and commands, commit with hooks, push, and post results plus commit.

## Task 4: validation gates and final integration review

Run the complete new evidence/semantic suites (`cargo test -p purrdf-sparql-algebra --test scope_invariants --test scope_candidates` and `cargo test -p purrdf-sparql-eval --test scope_interactions`), `cargo run -p purrdf-sparql-algebra --example scope_investigation`, `cargo bench -p purrdf-sparql-algebra --bench scope_checks`, `make check`, `make wasm`, full `make conformance` without shard selection, and `bash scripts/check-generated.sh`. Check all acceptance claims against authoritative outputs, recheck `.deficiencies`, scan the complete branch for forbidden incomplete-work language and inspect the clean worktree. Repair every in-scope failure, with separately validated commits/pushes/issue updates as needed. Independent final review confirms all six acceptance criteria and all matrix rows have evidence.

## Task 5: pull request creation and review

After all tasks are committed, pushed and verified, open a non-draft PR with a plain title and `Closes #387`. Include final behavior, investigation findings, full evidence and validation results; post the implementation plan to the PR. Inspect all CodeRabbit review bodies, inline threads and CI. Address each applicable finding with validated commits/pushes and explicit thread replies; record evidence for false positives and verify review completion on the latest head.

## Task 6: stage-3 finalization and merge

Resolve/verify worktree, branch and PR, ensure final clean state, fetch and merge the latest PR base into the issue branch, resolve any conflicts with maximum valid behavior, validate and push. Run deficiency and full diff/PR deferral gates. Write `${WORKTREE}.squash_notes` with requirements, files/purposes, validations, goals, scan evidence and conflict details; post to issue and PR. Run only `/home/paudley/stage/root/bin/ghprsq`. Verify merged PR, closed issue, signed result/audit refs and branch deletion. Remove only the clean issue worktree/local branch and preserve the pre-existing local-main commit when synchronizing the top-level checkout. Refresh the open-issue list and continue newest to oldest.
