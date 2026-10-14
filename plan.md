# Complete remaining evaluator acceptance

One owner, no subagents. Deliver the full current 545 contract from main at commit `cb38600b6`.
Reuse original reports only for unaffected computations; no timing acceptance.

## Implementation

Use the existing ResidentPending abort guard for NodeBoundary in
eval_evaluated_with. Keep it armed through callback and finish; disarm only after
finish returns, preserving source-error precedence and normal cleanup. Keep one
NodeBoundary abort home and one guard Drop body. Reuse the existing RootJoin
fixture to enable real evidence tracking in the callback-panic regression.

## Qualification

1. Actual RowConsumer panic and returned-error cases must reach installed cap,
   cursor and evidence scopes, restore all state, and permit healthy same-context
   evaluation. Keep existing positive-scope and hoisted fixture controls passing.
2. Run affected warning-denied Clippy and unchanged helper census. Correct direct
   findings before final gates; no exemptions or new duplicate implementations.
3. Execute revised stack_refusal natively, actual scalar WASM and actual simd128:
   native 192KiB refusal, WASM 192KiB correct answer, below-margin refusal and healthy
   recovery in governed and ordinary routes. Use provided Cargo directories and
   the existing runner/target-scoped SIMD flags.
4. Assess original 19 physical/110 scalar/15 SIMD and all seven current hosted
   assembly configurations against the changed guard. Rerun affected/uncovered
   coverage; reconcile source-bound generated/manual claims through original
   producers, without rebinding identity or hand editing numeric cells.
5. Settle implementation, perform required final qualification once, and obtain
   actual hosted library/Python/optimized package results through existing recipes.
   Original 181/281/180 depth floors and Worker limit remain intact.
6. Resolve actual feedback using existing and hosted reviews, describe self-review
   accurately, commit/push with normal hooks, merge via ghprsq, verify closure and
   ordinary mirror, then remove this delivery after evidence archival.

## Evidence and scope

Archived original 531 evidence is refs/ghprsq/pr/546/stage. The old full completion
audit remains FINDINGS-OPEN for this contract. Scalar/runtime/host results qualify
their recorded source and configuration. Cancelled 36/78 timing campaigns stay
excluded; fixed-work instruction/layout/allocation observations retain uncertainty.
