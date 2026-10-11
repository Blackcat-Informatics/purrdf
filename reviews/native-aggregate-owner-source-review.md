# Native aggregate owner correction: bounded source review

Root reviewed the correction to the outstanding aggregate finding in
`native-lazy-owner-repair-review.md`. No build or runtime check was run by this
review; the evaluator owner is running the focused collection separately.

`modifier::aggregate_evidence` retains the explicit-source route and passes the
same context's lazy owner callback to `RowEvidence::native_aggregate` otherwise.
The latter preserves virtual/empty-aggregate fast returns and an existing row
owner. Only the previously ownerless, nonvirtual fallback invokes that callback;
it no longer constructs an independent dictionary. Its aggregate computation,
workspace and closed-absence predicate are unchanged.

`EvalCtx::native_evidence_for_child` also clones an already admitted control
when no provider registry remains. It does not manufacture a control for a
plain context with none. Explicit-source children retain their existing shared
`EvidenceInput` control. Cross-query combination checks are not weakened.

The new `native_groups_from_ownerless_search_share_the_original_query` case
uses actual grouping and aggregate homes with zero, one and two members. It
requires retained lower bounds and unclosed search, combines independently
produced group records, then uses a worker after the original context has been
dropped. This directly covers the reported fallback and retained ownership.

Bounded source verdict: the aggregate finding is corrected coherently. Runtime
acceptance, existing cross-owner/admission/release controls, full conformance,
fuel-trace disposition, allocation and performance gates remain separate and
are not established by this review.
