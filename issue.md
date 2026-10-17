# Issue #572: Model the XSD anyURI value space in the native datatype engine

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

The native XSD engine leaves xsd:anyURI uninterpreted, so GMEOW maintains whitespace, equality and length branches (Blackcat-Informatics/gmeow-ontology#1803). Implement native XsdDatatype/XsdValue parsing, identity, storage accounting and data ranges. Use fixed XML whitespace collapse and XML Char validation; accept empty, relative and fragment references without URL normalization. Keep the primitive disjoint from string. Verify enumerations, Boolean range combinations, length facets, sound cardinality, governed parsing and WASM portability. Then integrate downstream and delete the superseded branches with no fallback. Normative source: https://www.w3.org/TR/xmlschema11-2/#anyURI

## Comments (2)

### paudley — 2026-10-17T08:31:59Z

# Native anyURI ownership

This implements PurRDF issue 572 as one mechanism cut of GMEOW issue 1803; it does not complete the entire reasoning migration. Native main lacks a modeled anyURI primitive. GMEOW has ad-hoc branches in reason/value.rs. Existing whitespace collapse and XML Char terminals are the canonical reusable implementations. No additional runtime dependency is needed.

## Task 1: Implement the native primitive

Extend datatype resolution, parsed value, ordinary and admitted parsing, canonical formatting, retained storage, equality and unsupported-operation classification. anyURI is unordered and disjoint from string. XML whitespace collapse is its only normalization; do not parse or normalize a URL. Reject non-XML characters before allocation. Reuse CollapsedWhitespace and the shared XML terminal.

Extend the range partition with a distinct anyURI stratum. Length is collapsed Unicode scalar count. Reuse the conservative character cardinality algorithm with a sound non-whitespace alphabet lower bound; do not claim exact finite cardinalities where the existing engine only proves a lower bound. Exercise enumeration, complements, intersections, empty values and length bounds.

Acceptance: focused native XSD tests, admitted-reader refusal/accounting controls, affected consumer compilation/tests, affected WASM build/runtime coverage. No GMEOW corpus production or hardware probing. Normal hooks must run. Independent review is unavailable locally; use actual CodeRabbit review after publication, never fabricate review verdicts.

## Task 2: Remove downstream duplication

Publish and integrate the native replacement through ghprsq. Pin GMEOW to the qualified native commit; replace its anyURI special cases with ordinary native value paths and delete the old equality/whitespace/length branches. Run focused GMEOW datatype tests without corpus production. Preserve unrelated worktrees and the frozen authenticated input store.

## Task 3: Publish, review and merge

Record exact checks and uncovered acceptance in validation.md, publish focused PRs and inspect review feedback. Use only ghprsq for final integration. Existing user authority permits merging while hosted CI is pending, but mandatory hooks and actual correctness findings remain binding. Keep broader ownership issues open until their full contracts are met.


### paudley — 2026-10-17T08:55:06Z

feat(xsd): model native anyURI values and range semantics

Advances #572 and Blackcat-Informatics/gmeow-ontology#1803.

The native datatype engine now owns anyURI parsing, XML character validation, fixed whitespace collapse, primitive value identity, retained storage and range semantics. This lets the downstream reasoner delete its ad-hoc normalization, membership, equality and capacity paths. Relative references, empty values, case and percent spelling are preserved; strings remain a separate primitive. Unsupported ordering and arithmetic do not acquire invented semantics.

The XSD value and admitted-reader implementations share the existing XML terminal and CollapsedWhitespace formatter. Range enumeration, complements, intersections and length facets use a distinct anyURI stratum. Positive-length cardinality remains an honest lower bound. XsdValue::facet_length is the single character/octet measure used by the native range engine and downstream adapter. The entailment literal prepass uses the same primitive. No new runtime dependency, feature switch, fallback or compatibility alias was added.

Validation: the complete purrdf-xsd crate suite passed. Three new anyURI cases passed natively and under actual WASM/Node execution. The admitted-parser allocation/refusal test and production entailment-prepass regression passed. XSD and entailment all-target clippy passed with warnings denied. After extracting the shared facet method, the affected 3 anyURI plus 36 data-range tests, XSD all-target clippy and all 3 exact WASM cases passed again. All three commits passed normal configured hooks and were pushed.

CodeRabbit reviewed ac4094981 with no actionable comments and minimal reported merge risk; its documentation coverage was 82.76% over 29 touched functions across 10 files, with 3 unsupported functions skipped. The final 128dda6bf change only moves the already-tested length measure into a public native method and delegates the old private calculation to it; that delta passed the affected checks above. CodeRabbit explicitly rate-limited the additional review. No local independent reviewer API is available; this limitation is recorded rather than represented as a final-head independent review.

Hosted workspace CI is pending on the final head, with no failed checks observed in the full current check inventory. The user explicitly authorized pushing and merging with subsequent hosted CI findings tracked, so pending hosted CI is not represented as a pass. No local GMEOW corpus producer or machine probes ran.

Integration: origin/main remains 410a927d9e97; merge-tree predicts the clean candidate bc13dfb30b5a5162c78b146d0ada5ffdca9c23c1. There are no base-side changes or conflicts to resolve. Evidence covers this native implementation; GMEOW pin/deletion tests are ongoing in its existing isolated issue worktree. Issue 572 stays open until that downstream acceptance lands; issue 1803 is a broader ownership contract and remains open.

Authoritative plan: /home/paudley/Active/purrdf/.worktrees/572-model-the-xsd-anyuri-value-space-in-the/.stage/model-the-xsd-anyuri-value-space-in-the/plan.md
Selected evidence: /home/paudley/Active/purrdf/.worktrees/572-model-the-xsd-anyuri-value-space-in-the/.stage/model-the-xsd-anyuri-value-space-in-the

Defect-Class: none


