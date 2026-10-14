# Compact live continuations for the two recursive stack floors

**Status: historical design; its two-home implementation was measured, rejected and
reverted.** See `T3-531-compact-continuation-disposition.md`. The clean source is
`330c55f8c`. The latest unchanged optimized package still answers only 234
right-nested LATERAL levels (floor 281) and 133 nested FILTER NOT EXISTS levels
(floor 180). The synchronous SELECT floor also remains failed. The exact
pre-link release diagnostic shows recurring frame reservations, not verified
Rust names or a byte-saving patch; see `T3-531-live-release-stack-attribution.md`.

The active recursion retains state that completion after the child needs. In
`binop::eval_application`, the left bag and `Lift`, left and right schemas, and
the admitted `per_row` buffer survive while each right operand is evaluated.
In `expr::eval_filter_sequence`, the input row iterator, linked VM, output
builder, schema, `Lift` and row checkpoint survive across `Linked::filter` and
its nested EXISTS. The wasm32 fork branch is disabled and may not be cited as a
live stack owner. An ungoverned FILTER checkpoint is behaviorally `Free`, but
its current type embeds a `WorkerLedger` even then. Moving final merge or a
dead fork arm cannot release these required owners.

Use the **existing** `workspace::WorkspaceBox<T>::new_admitted` for an exclusive
continuation, rather than another box/grant wrapper. Its `value: Box<T>` is
declared before its `LexicalFrame` owner, so payload destruction precedes grant
release. Its one allocation is charged before the fallible allocator and has a
typed refusal. An out-of-line constructor at each existing operator home must
build the large state and return only this compact owner before the recursive
child call. Calling `new_admitted(large_local, ...)` directly from the recursive
caller would leave the construction temporary in that caller's fixed wasm frame
and is not an adequate correction.

For FILTER, the continuation carries the existing linked VM, checkpoint,
schema, input iterator and output builder through the original single
sequential row loop. Its out-of-line constructor retains the original
checkpoint creation, program lookup, then link/admission order before boxing;
`checkpoint.pass` remains before `linked.filter`, and
kept original rows receive the same optional evidence before `push_row`. The
barrier and `lift.finish` remain after the loop. The forked native arm remains
at its original single home with the same worker, harvest, ordered commit and
resume operations. A zero-sized Free checkpoint specialization may remove the
unused ledger from the ordinary loop, but it is not a standalone floor fix and
must not duplicate the loop.

For LATERAL, after left evaluation and the existing property-function and
deferred-site admission, a continuation carries the left bag, `Lift`, both
schemas and `per_row` buffer. The original loop evaluates each right child with
the same `M::DECLARED` policy, deferred-site ordering and `eval_correlated`
fallback; it commits only complete per-row results, retains partial-child
truncation through the original lift, and preserves the first error. The
original schema union and `merge_application_row` remain the only output merge
body. A post-child-only helper cannot reduce the state live across recursion.

The box is one new physical allocation per operator invocation, including
ordinary shallow executions. That cost is explicit, and the `WorkspaceBox`
charge must include only the new concrete control, while existing payload
buffers retain their original admissions. Do not claim a performance win from
the depth floor alone: use fixed-work instructions and allocation counts on
the same original inputs, with shared-host interference stated. Do not weaken
the stack budget, floor assertions or typed refusal. If a query-level reusable
pool becomes necessary, its lifetimes need separate proof: FILTER's `Linked`
borrows expression sites from per-row substituted trees, shorter than
`EvalCtx`'s query lifetime. Storing those borrows in a query-lifetime Vec by
index would require lifetime erasure or redesign and is outside this bounded
candidate.

Review must settle the exact extraction boundary and construction/drop order
before source edits. Focused native checks then cover LATERAL rows/evidence,
EXISTS definition/deferred/forced strategy, refusal and first error, followed
by strict compilation and one fresh standard-dir optimized package with the
three unchanged floors and stack-byte controls. Source review or native checks
alone cannot qualify the candidate. No exclusive timing is requested.
