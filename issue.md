# Issue #1107: v4 API: retain native GTS composition and projection failures

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement, bindings

## Body

Release gate: #1104. Release preparation: #666.

## Problem

`SnapshotBuilder::add_dataset` and related operations return string errors. The existing `GtsIngestError` alternative also formats source failures and offending terms, while emission and relational projection discard native writer/blob causes.

Original requirements: #558. This is a bounded pre-v4 delivery; the original issues remain open for their complete contract.

## Source evidence

- [Dataset ingestion](https://github.com/Blackcat-Informatics/purrdf/blob/05d1e3550752d8f185710da0b00459651955d9f1/crates/rdf/src/gts_compose.rs#L408)
- [GtsIngestError](https://github.com/Blackcat-Informatics/purrdf/blob/05d1e3550752d8f185710da0b00459651955d9f1/crates/rdf/src/gts_compose.rs#L1195)
- [GTS emission](https://github.com/Blackcat-Informatics/purrdf/blob/05d1e3550752d8f185710da0b00459651955d9f1/crates/rdf/src/gts_compose.rs#L1427)
- [Relational projection](https://github.com/Blackcat-Informatics/purrdf/blob/05d1e3550752d8f185710da0b00459651955d9f1/crates/rdf/src/gts_view.rs#L720)

## Required change

- Finalize typed return contracts for dataset/scoped/view ingestion, `emit_gts`, and method/free-function relational projection.
- Retain original source-read, readiness, writer and blob-decoding causes and native offending terms. Settle generic source-error bounds and retained ownership before v4.
- Preserve the last fully accepted checkpoint, rollback, poisoned-builder refusal and original ingestion receipts. A poisoned builder must never publish its accepted subset as success.
- Route existing Python and native callers through the same production implementation; remove native formatting routes superseded by the typed transport.
- Preserve successful GTS bytes, scoped blank identity and relational results.

## Acceptance

- [ ] Actual source-read, readiness, ingestion, writer and blob-decoding failures expose their native fields through the existing visitor.
- [ ] Failed ingestion preserves rollback and poisoning, including later emission refusal.
- [ ] Existing successful composition/projection cases retain their output and identity behavior.
- [ ] Affected callers compile, focused native/host checks pass, and signature/ownership migration is documented.

## Remaining parent work

Remaining #558 inspection and producer coverage stays open. This child does not expand into a new GTS wire format or streaming redesign.

Source inspection at `05d1e3550752d8f185710da0b00459651955d9f1` establishes the current API shape, not runtime qualification. No builds or tests were run for this issue's intake.

Use focused native behavioral checks and affected production-consumer compilation with the required profile/warnings. WASM execution is limited to documented dispatch, host-interface, shadow-stack or frozen-identity obligations. Required hooks and release gates remain mandatory. Do not add compiler-guarantee tests, duplicate semantic suites, or a whole-workspace run merely to finish this child.



## Comments (0)

