# Current acceptance

PR #548 changes one implementation/test file, `eval.rs`, from main at commit `cb38600b6`. The existing `ResidentPending` guard now owns `NodeBoundary` through callback execution and completion. Its adapter calls the original abort implementation. There is one guard destructor and no new heap allocation. The existing `RootJoin` fixture is shared by both tests.

| Criterion | Evidence | State |
|---|---|---|
| Callback unwind and returned-error cleanup | The actual answer callback observes installed cap, cursor and evidence scopes. Both panic and returned error restore them; a query on the same context succeeds afterward. The focused native regression and final full gate pass. | PASS |
| Existing provider and nested scopes | The copied-root provider control and temporary positive-scope success, error and unwind controls pass, including in the final gate. | PASS |
| Physical stack boundaries | The original fixture passes natively, on scalar WASM and on SIMD WASM. It retains native 192 KiB refusal, WASM 192 KiB correct answers, separate below-margin refusal and healthy recovery in both governed modes. | PASS |
| Strict compilation and helper census | Affected all-target Clippy passes with warnings denied. The unchanged helper census passes with 81 enforced jobs, 118 distinct groups and no open copies. The final full gate also passes. | PASS |
| Ordinary rows and admitted owners | The final gate executes the 40-byte ordinary-row assertion, refusal before copied EXISTS/LATERAL scope entry, pending-buffer refusal and complete grant release after the last output. It also executes shared owner and row lifetime controls. | PASS for those assertions |
| Physical shapes | The final gate passes all 19 physical-shape cases, including bounded owners, cache refusal, evidence and empty barriers. | PASS |
| Portable computation and installed package | Hosted run 38523547585 passes the original portable execution recipe, Python/conformance jobs and complete optimized package job. Changed guard coverage also has focused native, scalar and SIMD execution. Historical portable results remain attributed to their original inputs. | PASS for executed coverage |
| Assembly and maintained projection | The same hosted run passes all seven configurations and the original complete projection producer. Downloaded aggregate and qualification logs cover 112 sites on seven configurations. The generated document matches the maintained document. | PASS |
| Required local gate | Normal `make check` completes with exit status 0, including its nested release-WASM build. Receipt: `tasks/final-full-check.log`. Normal commit and push hooks also passed. | PASS |
| Current PR feedback and integration | PR #548 has no failed or pending check. The current independent one-file source review has no actionable finding; the documentation warning was declined publicly. A fresh merge-tree computation against fetched main is clean; main has no additional delta beyond the tested base. The owner's complete-contract assessment maps all remaining execution requirements. | Ready for protected integration |

The guard changes cleanup ownership at an existing call boundary. It does not change row representation, text-shape computation, numeric kernels, result multiplicity or first-error precedence. The final local gate and hosted portable/package execution cover affected consumers. Historical assembly identities are never rebound.

No fresh independent audit is claimed. Existing source/requirement reviews remain archived under `refs/ghprsq/pr/546/stage`; current PR review must adjudicate the affected correction and acceptance. The fixed-work DISTINCT estimate remains uncertain and establishes no instruction win. No timing acceptance or exclusive campaign is used.
