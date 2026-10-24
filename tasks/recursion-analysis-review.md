# Static recursion analysis: owner assessment

This is an implementation assessment, not independent review or whole-issue
acceptance. No child agents were used.

The shared `Parser::prepare_with_expressions` invokes the analysis after shape
syntax admission and before any recursive shape expansion. This covers native
loads, product/source reconstruction and expression-bearing loads. The original
typed error is retained through `load_error` / `load_complete_error`, including
storage refusal from the existing fallible Tarjan implementation.

All shapes come from the complete specification census, including parameter
declarations; the narrower explicit-target candidate census would miss those.
An absent referenced shape hard-fails rather than silently dropping its edge.
Shape-expecting parameters come from the authoritative specification table,
deduplicated for the shared qualified-value parameter. `targetWhere` is the
additional normative refers edge; it is labelled as a target dependency rather
than invented as a conformance constraint. Logical lists use the existing strict
RDF list walker. Nodes and edges have canonical order; no hash iteration order
decides which component or diagnostic is returned.

Qualified value-shape polarity follows the actual count: lower-bound positive,
upper-bound negative, both nonmonotone. Excluded sibling conformance has the
opposite polarity; both bounds remain nonmonotone. Sibling identities are
collected at their existing production home, now shared with analysis. The pack
entry retains `ShapesError::Recursion`, with no false product-admission dimension
or conversion into an ordinary validation result.

Checks: native library build, 15 relevant SHACL tests, strict shapes and consumer
library/all-test compilation, and one real prepared-product pack regression pass. The
first consumer compilation exposed the exhaustive error-match omission; its
failure log is retained and the production match was fixed.

Open contract: this implements static refusal without selected semantics. It
does not implement an LFP/GFP option, cyclic identity model, fixed-point solver,
pair-work resource budget, complete stack-safe lowering, ShEx coverage, sheval
corpus, host options or portable/final qualification. No PR, merge or issue
closure is justified by this slice. The separate W3C report work is untouched.
