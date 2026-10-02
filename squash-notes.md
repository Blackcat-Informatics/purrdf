docs: align release guidance and Chinese catalog with shipped behavior

Correct the book's C ABI declaration from 0.7 to the shipped 0.8 and replace historical interleaved crate bootstrapping with the current upfront empty 0.0.0 records, publisher configuration and publishing locks. Name both Python distributions, preserve handwritten migration/release notes and list the six mandatory pre-tag gates in their actual order.

Reviewed head: 4cafe98e2d53f681e6f6a671d81e32f46ce5220c against main 91c22d7f44f418f05e88f6c9025f3289af43a9ee. Related release publication remains in #375; this PR does not close it.

Files and requirements

- docs/book/src/project/releases.md: align the public release procedure with docs/RELEASE.md, Makefile and the committed C header, including registry setup, note preservation and the Python/C artifact gates.
- docs/book/po/zh-Hans.po: regenerate the catalog using make book-po-update and translate 28 changed or newly exposed units (15 release, 13 diagnostics). Source-reference and gettext wrapping changes are generated; superseded messages remain explicitly obsolete. Active instructions preserve code tokens, URLs, exact API meanings and the prohibition on partial answers after a fatal source-read failure.

Validation

- The signed commits passed normal staged-snapshot verification hooks. All 154 documented-claim checks, version coherence, 68 gate-parity checks, brand/issue-reference checks and git diff --check pass.
- Actual make check-i18n passes all six rendering gates across 33 pages and all 24 SPARQL fences. All 2,865 active messages are translated; zero active fuzzy/untranslated units and zero source/catalog drift. Independent bilingual review confirms all 28 changed translations against their English source and authoritative procedure.
- The first hosted Docs run correctly rejected stale translated msgids. After the catalog correction, the exact reviewed head passes all 42 jobs in CI run 36968998654, the complete Docs build in run 36968998653 and all five CodeQL language jobs in run 36968996095. Pages deployment is skipped under the normal pull-request condition.

Standing constraints and final scan

.goals is respected: the two-file documentation change alters no runtime implementation, release version, serialized bytes, binding behavior, Cargo features or dependency architecture. It describes the existing single Rust engine, explicit source failures and metered operation accurately. The deficiency ledger has one canonical marker and no entries.

The complete untruncated diff has zero added-line deferral matches. The sole public-comment match is CodeRabbit's automatic table label "Out of Scope Changes check", which reports a skipped bot check because no closing issue link exists; it makes no implementation descope assertion. Every related correction is implemented and validated. The current base is merged without conflicts.
