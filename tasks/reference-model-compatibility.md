# Shared identity candidate: public API failure

The current uncommitted candidate adds Shape.catalog and Constraint.Reference.
It propagates definitions through native lowering, evaluation and metadata walks,
but it is not a qualified delivery. The existing product_api_compat integration
test rejects two bare Shape constructions and its exhaustive Constraint match.
That test deliberately protects public constructors and exhaustive matches and
has not been changed. The candidate also still has explicit unsupported-product
refusals, so library compilation does not establish product acceptance.

The all-test compile failure is retained in reference-model-test-compile.log.
The earlier unnecessary Arc qualification and two internal exhaustive matches
were corrected; those corrections do not resolve the actual public API conflict.
The latest compilation reaches and correctly reports that conflict.

A question is pending about preserving this public model contract versus
accepting and documenting a breaking model change. Do not adapt the compatibility
test merely to silence it. No reference-model code has been committed or pushed.
ShEx parser stack admission is an independent safety unit and can be qualified
and committed without including this candidate in the index.

The root REWORK_LIST.md records the dirty checkout and this limitation. No new
PR, merge, issue closure, full gate or independent approval is claimed.
