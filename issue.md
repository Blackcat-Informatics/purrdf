# Issue #682: sparql-eval: evidence entry points over a fallible `DatasetView`

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

Per-row evidence (#515) is reachable only through the infallible entry points
`query_interned_evidence_view`, `query_evidence_status_view`, `query_evidence_answer_view`,
`execute_witnessed_evidence`, and retrieval's `search_scoped`. Each is bounded on
`D: DatasetView<ReadError = Infallible>`.

A host whose reads can fail is shut out. Katamari evaluates over `SegmentedStackQuery`
(`ReadError = SegmentedError`): sealed layers on disk plus a resident head. Its fallible doors
take no `EvidenceSource`: `query_fallible_view`, `query_prepared_fallible_view`,
`execute_fallible` and the governed variants. Materialising an infallible dataset per query would
copy the graph on every query, which is exactly the cost the segmented layers exist to avoid.

## Ask

Fallible counterparts that thread the source the way `execute_fallible` threads the rest:

- `query_evidence_answer_fallible_view(dataset: &D, request, options, source, answer, visit)`
  with `D: FallibleDatasetView + Sync`. A read error is returned as the read error and is never
  turned into evidence: a transport or read failure is not a scope declaration. This matches the
  rule `service_scope` already keeps.
- The same for `query_interned_evidence_view` and `query_evidence_status_view`, and for
  `search_scoped` where retrieval's producers read through a fallible view.

The plumbing underneath already takes `Option<&dyn EvidenceSource>`
(`execute_ungoverned_admitted`), so this asks for public doors, not new evaluation.

## Done when

The evidence suite (`tests/evidence_ingress.rs`) passes over a fallible view, and a read error
raised mid-query surfaces as the error rather than as an undetermined row.


## Comments (0)

