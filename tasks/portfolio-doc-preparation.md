# Owning documentation qualification — preparation only

Status: SOURCE-ONLY READY; no PO/generated source edit, Cargo, uv, glossary, render, cost or timing execution. Current shipping tree d632307fb725304ed864967d5234cdab35bc5c99 includes currentmain2eb, contextual queries and the qualified row protocol.402 currently owns the local lane. Root must separately admit generation and validation.

## Source mutation before final evidence binding

Both conflict preimages remain intact: `raw/portfolio-integration-main-zh-Hans.po` and `raw/portfolio-integration-contextual-zh-Hans.po`. Current PO came from main-first msgcat with branch-only translations preserved; it is not yet the current English-source projection. Before generation, capture their hashes plus the merged current PO bytes in a new owned log directory. Never overwrite either preimage or remove branch translations merely because they came from the old side.

Owning command `make book-po-update` runs `book-pot` (actual mdbook xgettext renderer against current English) and `msgmerge --quiet --update --backup=none docs/book/po/zh-Hans.po docs/book/po/messages.pot`. POT is ignored output; PO is mutable tracked source. Capture exact command/full output/terminal, fresh POT and post-merge PO. Review actual msgid/msgstr/fuzzy/obsolete changes against both preimages and current English, preserving valid translations and investigating dropped rendering/mismatched meaning. Translation lag/fuzzy entries legitimately render English; never count them as translated. Do not hand-edit a generated POT/reference counter or silently discard older msgstr content.

Any needed deterministic SVG projection is owned by `make book-samples`; run its generation before final tree binding if generated verification identifies drift. Other generated drift must be repaired through the specific owning generator (or normal metadata workflow when justified), with each actual source delta reviewed. Do not run blanket metadata writes or broad conformance/Python/vendor tests to manufacture counts.

Root stages ONLY actual reviewed tracked changes, then captures a new EXPECT_TREE and complete source/projection delta. Validation cannot start against d632 and silently bless changed PO/assets afterward. The docs driver deliberately excludes PO update for this reason.

## Required real gates and outputs after generation settles

Use same active SDK/private target/build/tmp with jobs8, fresh logs and explicitly admitted lane. No new dependency/tool install/pin bypass. Current authority pins mdbook0.5.3 in docs.yaml and i18n helpers0.4.0 in Makefile; existing render gate verifies them itself.

|Owning command|Actual required evidence|
|---|---|
|`cargo run --locked --jobs 8 -p helper-census -- --glossary-gate --self-test`|Terminal0 and actual complete native glossary self-test/coverage; do not import a prior260 count as this source's result.|
|`make check-i18n`|Actual native production glossary scans current MD/PO, then existing render self-test. All seven real poisoned-catalogue refusals (including both fence forms and unreachable stale msgid) and all six final zh rendered arms must execute/pass, with current catalogue translated/fuzzy/untranslated/stale statistics and actual page/fence counts. Missing tools/zero coverage is failure.|
|`bash scripts/check-generated.sh`|Actual generator-output byte comparisons and doc-claim checks0, including all Unicode/entailment/codec projections and deterministic visualization. No handwritten counter replacement or weakened check.|
|Existing English prose gates: `check-brand-casing.py`, `check-issue-refs.py`, `check-spec-attribution.py`, `check-doc-claims.py`|Actual current English/source claim checks0; native terms/first-party attribution and consumer scope remain honest. Record their owning self-tests where the caller normally requires them.|
|`make book` followed by `make book-zh`|Actual normal English HTML and zh-Hans HTML outputs and terminal0. book invokes the owning SVG generator; final source hash readback must prove its rewritten tracked projections remain identical to the bound source. Capture generated public chapter/sidebar output identities before cleanup; a prior Markdown gate alone is not these actual HTML builds.|
|`python3 scripts/check-python-stub-parity.py` only if interface/stub source changes during doc repair|No silent API invention: current328-member/52-class gate already passed at d632, so unchanged interface evidence may be reused. Changed declarations require actual owning requalification.|

The existing Stage docs phase currently covers native self-test, real check-i18n and generated checks. Root can add the bounded source prose/normal HTML commands above to that admitted phase or capture them in a sequential owning wrapper; preparation does not imply they ran. Final tracked bytes/index/refs and rendered artifact inventories must be captured/read back, and actual session terminal is required for PASS.

## Claims and counts that must stay truthful

Current book `interop/rdflib.md` and CHANGELOG describe contextual Graph/Dataset/processor mapping laws while ordinary Store/prepare and SHACL keep native prebinding. Current row Rust docs/stub describe len, projected iteration/None/signed IndexError and FIRST native name lookup; compat tuple LAST labels/asdict remain their existing home. No native asdict factory, universal query performance or broader compatibility completion is invented.

CONFORMANCE's generated4164 broad-Python row explicitly says last complete broad-suite snapshot. The new28 focused boundary PASS does not refresh that row or the old vendor total; no broad rerun is authorized. Generated entailment/benchmark inventory/restatement counts must follow their actual current owners. Do not replace the generated matrix with a hand-counted test sum.

Frozen incoming W3C license bytes (including inherited trailing spaces) remain unchanged; no withdrawal/submission/corpus mutation is part of this route. All external-repository submissions are excluded and untouched.

## Precise reuse after projection-only changes

If only PO/messages/references, prose or deterministic assets change, reuse source-bound native2119+4+2 passes, owning row Rust gates and current exact-wheel28 boundary evidence for byte-identical shipping Rust/Python/stubs/tests. Record their prior source manifest plus an explicit allowed projection delta, rather than claim the old full-tree identity is unchanged. Native-cost inputs must exclude documentation projections only by explicit source comparison; any executable/generator-dependent semantic source change requires its specific owning requalification and new artifacts. No blind full native/Python/render repeat solely for a commit/tree identity change.

No final documentation, native-cost, whole454/473 completion, normal-hook/publication or hosted PASS is currently asserted by this preparation.
