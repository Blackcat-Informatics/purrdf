# Fallible evidence delivery

Issue: #682. Base: main after the complete signed-maintenance delivery.
This is the selected high-priority queue delivery. The full issue includes
retrieval; evaluator wrappers alone do not complete it. No dataset materialization
or second evaluator is permitted. Existing #558 work remains preserved separately.

## Implementation order

1. Share the existing admitted query/evidence body between resident and fallible
   publication boundaries in crates/sparql-eval/src/engine.rs. Preserve the same
   declaration capture, evidence dictionary, answer-tier stopping, WHERE receipt,
   admission owner and source read sequencing. Add public fallible counterparts
   for query_interned_evidence_view, query_evidence_status_view and
   query_evidence_answer_view, plus prepared witnessed execution. Use the existing
   FallibleScopedResult and operational receipts; original read errors outrank
   query/evidence failures. Reset retained prepared state after a failed run.
2. Wire scoped retrieval through that original fallible execution path. Current
   search_bound, execute_bound and DatasetExclusion require Infallible. Preserve
   their planning, compilation, producer, constraint, stream and fusion behavior
   while making operational failure an original typed result. Do not put a read
   failure in an unknown evidence row or per-stratum soft status. Check completion
   after actual stream/visitor use before returning publishable ranked output.
3. Execute evidence_ingress against a real fallible view. Preserve the current
   SELECT, ASK, CONSTRUCT and DESCRIBE expectations, answer tiers, explicit source
   declarations, coalescing and actual WHERE receipt. Inject failures at preparation,
   mid-query, term/ledger resolution and final read completion; assert the original
   operational cause/evidence, discarded visitor value and healthy subsequent run.
   Exercise scoped retrieval with a fallible segmented source, including exclusion
   and stream reads. Include unknown semantic evidence as the distinct healthy
   neighbor. No infallible materialization adapter can satisfy these controls.
4. Run focused native and supported scalar/SIMD WASM execution, affected strict and
   allocation/ownership checks, and required normal hooks. Add API documentation
   without process references. Open one complete issue-closing PR, obtain substantive
   independent review, finish hosted checks, then land through ghprsq. Verify the
   actual integration tree, issue closure and archive before supported cleanup.

## Acceptance map

| Requirement | Demonstration |
|---|---|
| Public answer/status/interned doors accept FallibleDatasetView + Sync | Actual calls in the existing public evidence harness; same semantic results as the resident control without constructing a replacement dataset. |
| Prepared witnessed execution keeps the source | Original prepared execution and EvidenceSource run once; unchanged witness and row evidence; healthy retry after an injected read failure. |
| Read errors remain operational | Mid-query and final-check refusal return the original typed source error and exact receipt; no successful visitor value or unknown evidence row escapes. |
| Scoped retrieval supports the same source | Real fallible producer, hard constraint, exclusion and fusion path; original read refusal prevents successful ranked publication. |
| Existing evidence semantics remain unchanged | SELECT and non-SELECT forms, answer tiers, supported early stop, declaration failures and semantic unknown neighbors retain their current expectations. |
| Portable, bounded and reviewed delivery | Actual registered native/WASM cases, strict affected checks, original owner accounting, normal hooks, complete feedback and hosted CI. |

Source observations: execute_ungoverned_admitted already accepts DatasetView and an
optional EvidenceSource; execute_fallible already owns preflight/reporting/execution
admission and finish_fallible_read. query_interned_input retains the richer answer
context needed by all query forms. Retrieval's source path is broader than one
public wrapper, so its current Infallible bounds are explicit work in this delivery.

Missing acceptance remains visible until executed. No implementation, test or merge
is claimed by this plan.
