fix: align WASM stack boundaries and GitHub release assets

The integrated pre-tag gate found an async query refusing one level earlier than its synchronous twin. Return resolver and evaluator-context construction frames before recursive execution so setup scratch no longer remains underneath the evaluator. The existing directional assertion, job region size, stack guard margins, typed refusal and no-poison checks are unchanged. The two evaluator-context barriers are scoped to wasm32, preserving native compiler inlining policy. Current-nightly/Node 26 focused execution now admits 202 and refuses 203 on both lanes.

Replace the full 454 KB GitHub release body with an exact reviewed 2,245-byte summary that links the unchanged complete changelog, migration guide, registry packages, tagged source downloads and hosted C SDK. A shared checker validates version, visible content and a 64 KiB ceiling before tags and before crate publication. SHA256SUMS covers the actual C archive and its two recipient evidence files.

Reviewed head: dd6ceee579f8369f1b597c3b38ae0ca5ae7f1bf3 against main 5660cc18b221ef23def198c067cec0b495e72bdd. Refs #375; publication remains tracked there.

Files and requirements

- crates/rdf-wasm/src/async_query.rs: separate resolver construction from the operation's live recursive execution frame.
- crates/sparql-eval/src/engine.rs: retain one shared context-construction seam with its construction frame returned before ordinary or governed recursion.
- crates/rdf-wasm/js/tests/async-concurrency.test.mjs: report discovered endpoints; all prior assertions and bisection remain unchanged.
- scripts/check-release-notes.py and docs/releases/3.0.0.md: the validated summary and exact three-file checksum home.
- Makefile and .github/workflows/ci.yaml: current-summary validation and meaningful valid/refused regression checks.
- .github/workflows/release-cargo.yaml: prepare summary before publication, retain checksums, create the GitHub Release with exact summary bytes and upload four reviewed assets.
- docs/RELEASE.md, docs/book/src/project/releases.md and docs/book/po/zh-Hans.po: maintain the English and Chinese procedure, translating exactly four updated active units.

Validation

- Normal staged-snapshot verification hooks passed on the signed commit after correcting Python package labels in the new summary. The complete changelog is byte-identical to the base.
- Current-nightly optimized WASM focused boundary and explicit refusal checks pass on Node 26.10: both lanes admit 202 and return typed evaluation-stack refusals at 203. The 1,048,576-byte job region and 64 KiB margin are unchanged. A fresh scope-adjusted build is byte-identical across all five shipping module/glue/type files and its focused test passes again. Independent retained-source/module/log review passes.
- 74 native async tests, 94 engine tests, package all-target clippy with warnings denied, formatting and diff checks pass.
- Summary/checksum valid-and-refused regressions, 70 gate-parity checks, actionlint, version/toolchain/doc/license checks pass. Full i18n rendering passes six gates, 33 pages and 24 SPARQL fences; all 2,865 active messages are translated with zero fuzzy, untranslated or source-drift entries. Independent review confirms all four changed Chinese units.
- Hosted exact-head checks pass at dd6ceee579f8369f1b597c3b38ae0ca5ae7f1bf3: CI run 36978861928 has all 42 jobs successful; Docs run 36978861975 builds successfully (deployment is skipped on pull requests); CodeQL run 36978856740 has all five language jobs successful. The completed watcher exits 0. Independent final source and focused-runtime receipt reviews pass.

Standing constraints and scans

.goals is respected: one Rust engine, explicit failures, deterministic output and portable paths remain intact. No Cargo feature, dependency, release version, serialization or safety budget changes. No benchmark speed or physical-hardware memory claim is added. The deficiency ledger has one canonical marker and no entries. The full untruncated diff contains no added-line deferral matches. The complete PR body/comment scan finds one false positive: the automated passing-check table heading “Out of Scope Changes check” in comment 5947139410 is a check label, not deferred implementation. No genuine deferrals remain. Current origin/main merged as already up to date with no conflicts.

Review disposition

The maintainer explicitly directed release without changing the empty-changelog-section check. The current section contains 454 KB of history, and the existing pre-tag Makefile gate independently rejects an empty section. The checker and reviewed source remain unchanged.
