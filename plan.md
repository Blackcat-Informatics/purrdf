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
