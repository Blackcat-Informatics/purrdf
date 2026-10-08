# Task3 independent completion preparation — 2026-10-08

Status: PRELIMINARY; completion is UNVERIFIED pending settled full and rendering
executions. This is not a final PASS, PR authorization or integration claim.

## Scope and method

Reviewed committed source `b1833aeed387299546f883baaacb87f2206e169a`, the sole
plan, validation index, captured complete issue260 (body and zero comments),
Task1/Task2 implementation and independent reviews, Task3 correction and its
independent review, and stagectl's `references/completion-adversary.md` prompt.
Repository laws and the empty emergency ledger govern this review. The source
receipt records only untracked `.stage/`; no shipping changes were made here.
This preparation used source and existing actual execution records, with no
builds, tests, hook invocation or forge mutation. GMEOW owns the heavy lane.

Evidence reuse is deliberate: unchanged parser, pattern, table, PO, token and
caller behavior retain their qualifying Task1/Task2 executions. The subsequent
two-file correction changes Markdown title/fence whitespace and removes a
duplicate test fixture write helper. Its changed behavior has fresh focused
qualification; it does not invalidate every unchanged prior demonstration.
The committed-source receipt records the current catalogue and glossary hashes,
which differ from Task1's earlier receipt after Task2 documentation changes.
Task2's settled sweep, not a historical catalogue identity, is the reusable
production inventory.

## Explicit issue requirements

| Issue requirement | Actual implementation and demonstration | Preliminary assessment |
| --- | --- | --- |
| R1: anchored rejected substrings in larger Chinese phrases; preserve narrow provenance/determinism exclusions | Native matcher and table-derived rejection/neighbour controls; deterministic actual catalogue inventory; 41 applicable real-paragraph poisons refused in the settled native sweep. No collision exemption was needed. | Demonstrated by actual execution; current full scan must confirm the current result. |
| R2: a URL-only English msgid must not anchor a row | Shared visible-text projection masks inline/image destinations and titles and reference definitions. Production controls exercise URL-only, mixed visible prose, reference labels and multiline destinations. The initial multiline leak was found independently, corrected and re-reviewed. | Demonstrated; no source regression found. |
| R3: decide/document narrow half-width `研究对象 (Research Object)` exception | Authoritative glossary documentation states the English parenthetical exception. Actual row-derived controls accept half/full-width parentheticals and refuse bare/translated wrong forms; no general CJK punctuation relaxation. | Demonstrated. |
| R4: whole-token survival controls for every K row | Self-tests derive controls from all current rows/tokens, including multiword/hyphenated names; dropped/kept, embedded prefix/suffix, case, punctuation and CJK controls on both sides. Actual gate reports 24 K tokens and 1,716 production controls. | Demonstrated; not an RDF-only fixture. |

## Accepted contract matrix

Numbers correspond exactly to the nine executable criteria in `plan.md`.

| Criterion | Current production path | Qualifying actual evidence and limits |
| --- | --- | --- |
| 1. Host Rust gate, explicit inputs, single bounded matcher and hard errors | `helper-census::glossary::{mod,pattern}` uses the public first-party `purrdf_jsonschema::ecma` matcher; host dependency/layers edge is explicit. Root/PO/glossary/self-test argument admission, rule/location errors and resource refusal are native. No claimed exact match span or private fallback. | Task1 native tests and production controls, strict all-target clippy and layer hygiene; Task2 external-input caller tests. Current full hygiene remains required. |
| 2. Supported Python-compatible pattern contract | `pattern.rs` explicitly adapts Unicode word/boundary/space, LF dot/dollar and case behavior, documents Unicode/interpreter differences and refuses unsupported constructs. Ordinary anchor ASCII-left and case-sensitive K-both-side boundaries remain explicit. | Task1 matcher fixtures and current-expression controls, including Unicode/ASCII neighbours and malformed/resource controls. This source is unchanged by the Task3 correction. |
| 3. Table/PO compatibility, mandatory specimens and self-test-before-scan | Native table/PO readers retain seven-column parsing, escapes, continuation/context/header/plural handling and inactive units. `run` performs consistency and all production self-tests before normal scanning; malformed/missing inputs fail. | Task1 native fixtures and 1,716 controls; Task2 external-input/missing-input production caller tests. Existing native self-test invocation is not treated as independent rendering acceptance. |
| 4. One visible projection, whole-document fences and real locations | `surface.rs` is shared by anchors/K survival/rejection prose, masks destinations without concatenating tokens, retains visible labels/autolinks/code identifiers for K, and excludes exact code/fences from rejection prose. Paragraph-bounded balancing fixes multiline URLs. Task3 uses the existing clause-appropriate terminal predicate for title separators and closing fences, without a broad Unicode exception. | Task1 corrected multiline fixtures/re-review; Task2 caller tests exercise renamed tracked Markdown and multiline fences. Fresh Task3 11 glossary tests include FF title/false-closing-fence positives and neighbours; two external caller tests pass after correction. |
| 5. Every current K token, both-language boundary/case controls | Actual table rows drive isolated production checks rather than a private RDF-only substitute. Other active K obligations are satisfied while isolating each tested token. | Actual native production controls for all 24 tokens; dropped/kept/prefix/suffix/case and punctuation/CJK cases. No new K row was introduced by Task3. |
| 6. Current anchored sweep/context/poison and narrow exclusions | Native scan/sweep retains deterministic inventory and real anchored paragraph poisons; broad Chinese word-boundary immunity is absent. Existing provenance, determinism and global exclusions are tested. | Settled Task2 `T2-sweep.log` / `T2-glossary-sweep.tsv`: 69 rows, 43 rejection rows, 24 tokens; 4,181 units (2,956 active PO and three Markdown documents), zero actual wrong/raw rejected occurrences and 41 real-paragraph poison refusals. Knowledge graph and Research Object have zero active anchored PO pairs, honestly zero real-paragraph poisons, with synthetic controls retained. Current full execution must confirm current scan counts and zero-hit result. |
| 7. Narrow Research Object typography | Specific documented English-parenthetical allowance, with accepted half/full-width forms and rejected bare/translated forms. No generic punctuation allowlist expansion. | Actual table-derived production controls and Task1/Task2 source review. |
| 8. NUL-safe content-selected Markdown and inactive PO units | Native `git ls-files -z`, byte-sorted tracked paths, glossary exclusion, specified CJK ranges and 15% non-whitespace threshold. Whole Markdown documents receive GLOBAL refusals only. PO fuzzy/obsolete/untranslated units remain inactive. | Native reader fixtures plus actual Git-backed caller test selecting renamed translated Markdown without a language filename. Missing inputs and visible global poison refuse actionably. |
| 9. Actual Make/CI/staged-hook wiring; obsolete path retired; projections | Make `check` and `check-i18n`, workspace CI and staged hook invoke the same Rust mode. Hook compiles working-tree helper code and passes the staged snapshot root. Python glossary implementation is removed; independent PO/render tooling remains. Existing gate-parity recognizes the native slot. | Task2 actual parity tests refuse missing native slots in either direction; actual normal-hook probe refuses poisoned index/clean working tree and passes clean index/poisoned working tree, preserving the real index/catalogue. Native tests, strict clippy, parity (71 identities), layers, ratchet and metadata regeneration passed. Task3 corrected live helper census passes 81 jobs/23 variants/91 distinct/1,879 files. Current full gate and actual `check-i18n` still must finish. |

Evidence homes: `T1-implementation.md`, `T1-review.md`, `T2-implementation.md`,
`T2-review.md`, their named settled logs, `T3-correction.md` and
`T3-correction-review.md`. Fresh correction execution records are
`T3-correction-focused-settled.log`, `T3-correction-callers.log`,
`T3-correction-clippy.log` and `T3-correction-helpers.log`. Root confirmed actual
terminal exits zero for sessions44571 and86961: 11 native glossary tests, parity,
two external caller tests, strict all-target helper clippy and live census.
Normal hooks then passed for committed/pushed `b1833aeed`.

## What current qualification must prove

1. Session56552 must reach an actual terminal success for the complete settled
   `make check` at the committed source, with source/compiler/configuration
   receipts and its actual gate outputs. `T3-full-check-settled.log` was still
   compiling workspace crates when read here; a running log is not a PASS.
   Inspect actual native glossary counts, helper census, warnings, all normal
   mandatory targets/hygiene and final exit rather than assuming success from
   the already-qualified focused subset.
2. The actual Make `check-i18n` recipe must terminate successfully. Its native
   gate comes first; `check-i18n-render.py --self-test` then verifies pinned
   mdBook/i18n-helper identities, renders poison catalogues and checks each arm
   goes red, **then renders and gates the actual zh-Hans book**. It does not exit
   after synthetic self-tests. Required evidence is actual prerequisite
   admission, all poison arms, nonempty rendered pages, translation/output and
   SPARQL/prose gate results, and terminal exit. Tool SHA256 receipt alone is
   not rendered-output acceptance. Preserve meaningful catalogue diagnostic
   counts rather than converting report-only stale-msgid diagnostics into an
   invented new requirement.
3. Final independent audit must reconcile these actual terminals with the
   criterion matrix and current source. Any real failure requires an owned
   correction and relevant reruns; missing evidence stays UNVERIFIED. No fixed
   review count or repeat of unchanged focused tests is required.

Historical dispositions remain visible: original full gate exit2 exposed real
owned helper hygiene defects; focused exit101 was missing genuine Cargo cache
markers, corrected using Cargo-created roots; coordination abort143 of the
precommit full run is neither success nor a code failure. None is erased by a
later pass. Initial Task1 multiline leakage and Task2 initial metadata/index
failure likewise retain their corrective records.

## Preliminary findings and delivery boundary

No new shipping-source defect was found in this preparation. The two concrete
open evidence items are the settled full terminal and actual complete i18n
render terminal. They block final completion approval until evidenced, but are
currently assigned/running or pending qualification, not silently deferred
implementation. Root owns subsequent PR, hosted CI/review and ghprsq integration;
local qualification, hosted acceptance and merged cleanup remain distinct.
