# Final qualification and publication preparation

Status: FAILED

Source is frozen. Required review finding is OPEN and PR publication is BLOCKED. Parent reports its independent public producer/consumer probe has proven signature loss: invalid or non-UTC xsd:dateTime lexemes with otherwise positive Compaction facts are classified packaging=true in eager/evented reads, produce no detached pairs and an empty content projection, and compact_and_certify drops the original authored COSE while reporting all six checks true. GTS-SPEC section 13.3 requires rewrite timestamps to be xsd:dateTime in UTC. The classifier must validate that required shape before treating a signature as packaging. This is a required actionable repair, not an accepted limitation or an external blocker. The parent retains remediation ownership and explicitly instructed this owner not to mutate source until assigned. This owner did not independently execute that new probe; its actual written review/probe evidence belongs to the independent reviewer.

The successful full gate below remains valid for its captured inputs, but does not prove this uncovered boundary. Earlier preparation status is superseded by the concrete review finding. This owner did not commit, edit the index, push, post, create a PR or merge, and did not write T6-review.md. Task6 is unfinished.

## Source identity and governing context

Root `/home/paudley/Active/purrdf`; worktree `/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`; branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`. Current signed/pushed HEAD remains `b2560386263ff53578b3a06e40f5739f577829ac`, tree `3fc84e6a3ab25917151b94d836389797acfd006c`; base `ce3c07192aba1e36666062c00f958670a827cfb5`. Final source is that HEAD plus the exact four-file uncommitted delta, not a newly committed head.

Read applicable AGENTS, root .baseline/.goals, Stage1/stagectl and their quality/validation/delegation/no-deferrals references; complete issue/intake/prior-art, approved plan, task implementation/review records and execution/validation. Inventory found no additional nested ADR/constitution. Governing copies/hashes remain in Stage. Protected main, unrelated/sibling work and processes, failure history and immutable governed vectors are preserved. No model, GPU/NPU, service, lifecycle or private-memory actions occurred.

Approved plan SHA remains `dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`. Its historical footer was not edited. Prepared publication text explicitly distinguishes that original snapshot from task checkpoints/current qualification.

Complete 52-path final base-to-working-source patch: `raw/T6-final-branch.diff`, SHA `1d9dacca903f3649ee8de5e823e4a51e4bf359dd7fe5b61af744aaec00ae6741`; complete manifest `raw/T6-final-branch-files.sha256`, SHA `1bf6eded811940dba80f8b34156e2102b4a16ebb32a3b2b8ede0f60edf5fa989`.

Task6 modifies only `crates/gts/README.md`, `crates/gts/tests/compaction_signatures.rs`, `crates/rdf/tests/gts_certify.rs`, `scripts/check-issue-refs.py`. HEAD delta `raw/T6-source.diff` SHA `0924b7432ac948a493a1b49d497293f01bc58caa7c83771f2d920ee3a4a79881`; four-file manifest `raw/T6-files.sha256` SHA `22479cc868c18355e61038c95d870bf6b94c326f80051190d9f2af41ab8b25cb`.

## Required full gate

Initial `CARGO_BUILD_JOBS=4 make check` terminated normally with exit 2 at the issue-reference gate. Five register rows were stale after already-touched source process labels had been removed. Exact failure is retained in `raw/T6-make-check.log`. Only those five rows were retired; detection rules/self-tests were unchanged. README incorrectly attributed authentication to parsing; it now accurately says parsing preserves original protected bytes for authentication. These fixes preceded the qualifying gate.

The second required `CARGO_BUILD_JOBS=4 make check` terminated normally with exit 0. Log `raw/T6-make-check-fixed.log` SHA `7f7aea50e039f0d0643fd1ddb0170089290c8cf8f691385463e4e3c4d8562c99`. Actual requirements ran: workspace/consumer formatting and all-target clippy/check, all policy/hygiene/generator gates, native workspace tests and docs, preserve-order consumer, core/ring-fence hygiene and actual release wasm compilation. No SKIP, hook escape, environment bypass, weakened profile, killed gate or substituted ad-hoc test occurred.

Gate source patch `raw/T6-gate-source.diff` SHA `16ef9abee2a7d701c4b4d900cdb79c0fffd24f6bd29d592e368ba89e97760b70`; 52-path manifest `raw/T6-gate-files.sha256` SHA `77082e9b925a52e168b3523d872174f96c39de49840fb11127a1ed6e04651eda`. Actual Makefile/hooks/CI/compiler inputs match `raw/parent-final-gate-inputs.sha256`; actual recipes are retained in `raw/T6-make-check-recipe.txt` and `raw/T6-wasm-recipe.txt`.

`raw/T6-native-summary.json` records 21,045 native cases passed across 594 result groups, 0 failed, 35 default ignored; 460 doc tests passed across 47 groups, 0 failed, 2 ignored. Preserve-order consumer passed 1 test and its zero-case docs. Actual native C ABI smoke freshly built/linked the current cdylib/header and executed its C program. Counts are actual result cases, not nested fixture records.

All 37 ignored lines/reasons are preserved verbatim in `raw/T6-native-summary.json`: existing fixture/ledger rewrite tools, installed-package acceptance, exhaustive scalar scanners and existing 100k-depth stack lanes; two alloc-probe doc examples. No ignore was added or counted as an executed pass. Immutable corpus regeneration was not invoked. None is a new composite acceptance omission.

`make wasm` actually completed optimized release compilation in 5m52s for all 31 publishable libraries plus bench. `raw/T6-wasm-artifacts.sha256` identifies 32 rlibs plus purrdf_wasm.wasm, manifest SHA `05b2b97d766b3f8d1e8aa2b3d50df6d28a2a20eccd3dea6ac01120bf884f0eef`. Compilation is distinct from the qualified earlier runtime executions below. Actual key native executable identities are in `raw/T6-native-artifacts.sha256`, with executed paths in the full log.

rustc 1.100.0-nightly commit `4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1; Cargo 1.100.0-nightly (7941be6fb 2026-09-11); native target x86_64-unknown-linux-gnu; wasm target wasm32-unknown-unknown. Native dev/test opt-level 3 with assertions/overflow checks enabled, test debug 0; release wasm opt-level 3, fat LTO, one codegen unit and normal strip policy. Consumer uses its actual separate manifest profile. Only jobs were capped to four. Complete environment is `raw/T6-environment.log`. Gate artifacts are slot0 `/opt/.cargo/slots/caa92a003e12f897/0/target`, native cached executables resolve through slot0/build. Earlier concurrent metadata observed slot1; that diagnostic is not gate artifact attribution.

## Final delta and current-input checks

After normal full-gate termination, two already-touched test process labels were removed and their two exact obsolete register rows retired (31 original → 26 gate → 24 final). Their substantive graph-budget/mandatory-signing rationale and every assertion remain. The compaction comment now correctly describes sealed PackagingSigner with its default tuple type.

Exact three-file post-gate patch `raw/T6-post-gate.diff` SHA `f348161d50fb839e7ee488f1a1b24e3d5a28611cd1d5fb1f169cda3a7d860093`. `raw/T6-post-gate-closure.json` compares all 52 captured inputs: only these three differ; both Rust files have identical non-comment lines; checker delta deletes two exemptions only. No executable body, assertion, fixture, expected byte, crypto/parser/caller, detection rule or generator input changed after the qualifying gate.

Final-input commands all exited 0 (`raw/T6-statuses.json`, individual `raw/T6-final-*.log`):

- `python3 scripts/check-issue-refs.py --self-test` (73 controls), actual `python3 scripts/check-issue-refs.py` (24 existing rows).
- `python3 scripts/check-brand-casing.py`; `python3 scripts/check-doc-claims.py`.
- `cargo fmt --all -- --check`; `git diff --check`.
- `CARGO_BUILD_JOBS=4 cargo run -q --locked -p helper-census -- --non-rust-ratchet --merge-base-with origin/main --target worktree` (one decreased legacy non-Rust path, no new unexplained program).

The old full gate is not represented as executing on the final prose/register identity. Functional/native/wasm-build evidence is reused only for unchanged executable closure; changed prose/checker was checked at final identity. Parent normal hooks must still run against its actual staged result.

## Qualified runtime/primary-answer reuse

Node v26.10.0 and wasm-bindgen 0.2.125 remain available; actual repository runtime entry is scripts/wasm-test-runner.sh. Reuse is bounded:

- `raw/T2-wasm-runtime-qualified.log`: six public primitive groups, all 70 official NIST records. SHAKE/ML-DSA implementation, public cases and fixtures are unchanged since their signed qualification; subsequent provenance prose does not alter answers.
- `raw/T3-R1-wasm-qualified.log`: eleven public COSE groups and complete pinned IETF answer. Subsequent typed-resolver dispatch uses the same parser/combiner and is qualified through actual Writer/file execution below; original parser/composite bytes remain unchanged.
- `raw/T4-R1-wasm-complete.log`: thirteen public Writer/file groups. Writer/signing/composite home/verification core unchanged from signed Task4.
- `raw/T5-correction-terminal-wasm.log`: nine current public compaction/certification groups after provenance/root/packaging repairs; actual eager/evented, repack/rotation/refusal combinations. Exact IRI extraction moved to its native home with identical behavior (`raw/T6-policy-reuse.diff`); `raw/T5-policy-wasm-complete.log` additionally qualifies that policy change. Initial failed development probes remain preserved and are not passes.

These cover 39 distinct primitive/COSE/Writer/compaction group roles; they are not 39 new Task6 executions or the whole CI wasm matrix. Current full native gate additionally exercised every changed native production path. Primary SHAKE/NIST/IETF source identities, complete record joins and independent audits remain in final Tasks1–5 evidence and unchanged final manifest. Governed vectors were neither fabricated nor regenerated.

## Current external authority and limits

Final preparation refresh 2026-10-06T13:52:50Z: `raw/T6-external-refresh.md`. Official IANA CSV SHA `4dc4c64f84e6020a05403862219b66b0bfd9cc853245d87b82ed00f6d77600f3`, official XHTML agrees: -58 lies in unassigned -256 to -54; -49 is standalone ML-DSA-65. JOSE draft-04/LAMPS draft-19 construction remains exactly pinned; no silent allocation substitution.

Governed Blackcat-Informatics/gmeow-gts main remains `0d1c8299c9411ea4ead853e31721d42ea66f081e`; actual commit JSON SHA `e4d78ae07e93bff91c3eca21b4f60a647a64ddf60c53389ddf36ddbd4b6a3dbe`; complete recursive tree JSON SHA `f644842d31e6234af02e8fda1945b7afbfed24be00608c3fd957ca9db53d54c8`, truncated:false. COSE directory only sign1-basic.json/sign1-empty-id.json; full tree no composite/ML-DSA path. Identity matches prior full absence audit. Conditional newly published shared composite fixture is absent at this captured authority state; primary known answers do not establish shared-engine GTS composite interoperability. This is preparation refresh, not a fabricated later merge-boundary check.

Actual CI requirements were inspected. Local qualification does not claim hosted CI, MSRV/Miri/full architecture/oracle/installed-language lanes, PR publication or Stage2/3 acceptance ran. Functional tests do not prove whole-operation constant time, external entropy quality, independent external key provenance or formal FIPS certification; API/docs and confidence text state those boundaries accurately.

## Scans and publication files

Final complete branch diff, immutable plan, all six current commit messages and prepared PR/confidence bodies were scanned. Branch has one proven HashML-DSA documentation false positive (distinct excluded primitive; requested composite is implemented); prose has zero hits. Laundering scan has 38 adjudicated witnesses: fixed seed slicing after exact length checking, typed infallible Ed paths, sealed Writer invariants, existing graph optimization and hard test failures. No added allow/ignore/semantic feature/optional dependency/SKIP/xfail/unconditional passing assertion or weakened golden exists. All dispositions in `raw/T6-scan-dispositions.md`; actual outputs/statuses in `raw/T6-deferral-final-branch.log`, `raw/T6-deferral-final-prose.log`, `raw/T6-laundering-final.log`, `raw/T6-scan-statuses.json`.

Captured candidate PR body `tasks/T6-pr-body.md` includes Closes #458, actual behavior/checks and draft limits. Required candidate issue/PR confidence answers `tasks/T6-confidence.md`. These are not publication-ready while the required classifier repair is OPEN: source/check identities and behavior claims must be refreshed after actual remediation and qualification. Neither has tool branding/agent credit; both identify the attached approved plan footer as historical. Parent must resolve and qualify the defect, independently review the resulting source/qualification, use normal signed commit/push, publish actual PR plus complete plan and confidence answers, verify receipts, then obtain final Task6 review/completion.
