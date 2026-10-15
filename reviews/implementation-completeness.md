# Complete ordinary-row repair review

Owner assessment, not independent approval. The complete issue's production
repair is implemented and qualified through real persisted APIs; hosted review
and the final full gate remain required before merge.

The original split-image reproduction fails with (0 ordinary, 1 reifier,
2 annotations), while its combined/no-declaration controls already pass. The
corrected test passes all six layer permutations, the combined image and the
no-declaration controls in both graph scopes, checking keyed object values and
the original ready/error state. It also passes unchanged as an exact scalar
WASM execution through the existing runner.

The actual classification home distinguishes inferred mutable input from
explicitly ordinary sealed input. Visibility was already enforced at all three
callers, so the former always-true visibility parameter and optional return
were replaced with the needed inference policy. Both stack adapters preserve
authored ordinary rows and no longer perform their promotion-only declaration
lookups. The obsolete paged helper is removed. No competing algorithm is added.

All 37 directly affected stack controls pass (22 paged, 15 segmented). Five
mutable classification controls pass, preserving actual inferred-input behavior.
The original public-query suite passes seven cases unchanged; its remaining
typed restoration case encoded the obsolete promotion contract. The independent
native oracle now imports all four authored ordinary rows and one reifier,
rather than replacing a row with an annotation. That case passes its actual
engine, retained-snapshot, graph and compacted-carrier comparisons after the
correction. Assertions were changed to the new explicit input contract, not
weakened to omit typing or canonical carrier comparisons.

Strict core library and both affected core integration targets pass with warnings
denied. The native helper census passes after rebinding the same forty capacity
proofs to the current constructor closure and three unchanged range authorities.
The last public-fixture-only adaptation does not change those shipping identities;
final hosted census still covers its test-body normalization. No new exemption,
runtime dependency, toolchain/cache change, RDF row representation or timing
acceptance is introduced.

Native producer/refusal/lifetime coverage is retained, including header-only
thousand-source opening and shallow historical snapshot ownership. Foreign
Katamari test paths are unavailable here and are not claimed as run. Source is
based on main 0d075201f; integration with the pending statistics delivery must
be assessed when the next actual base is selected.

Final disposition at cef34c8aa: all 49 hosted checks pass with four expected
skips; independent hosted CodeRabbit source review covers all eight files and
finds no actionable issue. Fresh inline/formal review collections and GraphQL
threads contain no unresolved feedback. The actual base has not advanced; the
clean merge-tree candidate is the same assessed source tree. Earlier scoped
native/public-query/WASM executions therefore still cover it. The whole issue's
ordinary/keyed/reifier/annotation, graph, layer-order and readiness requirements
are demonstrated, without foreign consumer execution claims or deferred work.

VERDICT: PASS — the complete contract is qualified for this integration. This
owner judgment remains separate from the actual independent hosted source review.
