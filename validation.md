# Current evidence

Published as PR #559 at cef34c8aa after normal commit and push hooks. The
production repair and focused qualification are complete. Final hosted run
38809158043 passes all 49 checks, with four expected skips and no failures or
pending checks. Material hosted source review has no actionable findings. The
refreshed clean candidate remains the assessed tree, with no additional base
delta. All issue behavior is demonstrated; merge/closure/archive/mirror/cleanup
are the remaining delivery operations, not completed claims.

Independent hosted CodeRabbit review covers the complete eight-file diff through
cef34c8aa and found no actionable findings. Raw comments and the empty formal/
inline review collections are retained under tasks/hosted-*.json. See
review-debt.md for the generic docstring-warning disposition and the remaining
CI condition. Origin/main still equals the original base; the clean integration
candidate is 52bb86ee8f37e3001befc29d2a68c1e5aefab2da. There is no additional
base delta requiring combined-behavior execution for this candidate.

The actual pre-fix public persisted reproduction fails at the reported tuple:
0 ordinary, 1 reifier, 2 annotations instead of 2 ordinary, 1 reifier, 0 annotations.
The combined-image and no-declaration controls passed before the first split
case failed. Evidence: reproduction-runtime.log, exit 101. The first fixture
attempt failed compilation because it named a nonexistent term constructor;
reproduction.log is not runtime evidence.

The existing classifier now distinguishes inferred mutable input from authored
sealed ordinary typing. All three callers establish visibility before calling
it; the unused visibility branch and optional return are removed. Both stacks
retain explicit ordinary rows and skip their promotion-only declaration probes.
The paged external-declaration helper is removed rather than kept as an obsolete
alternative. Explicit annotation demotion and mutable inference remain in the
same existing algorithm. Technical documentation states the corrected contract.

stack-controls-keyed.log passes all 37 directly affected cases: 22 paged and 15
segmented, including the new persisted regression, generated chronology, original
source faults, header-only thousand-source open and shallow snapshot ownership.
The new regression executes every three-layer permutation plus combined/no-
declaration controls in default/named graphs and checks actual keyed objects.

The first revised fault control used an unbound physical scan, which necessarily
visits the unrelated page under the existing admission path. It did not isolate
classification. The corrected indexed subject/predicate probe proves an ordinary
row needs no declaration read, while an associated annotation still refuses on
its required declaration fault. The original failed stack-controls.log is
retained, not relabeled as a pass.

Strict affected compilation, preserved mutable-inference controls, public engine
queries and native helper verification have the scoped passes recorded below. The existing forty capacity
proof contracts retain their reasons, classes, sink identities and coordinates;
only the constructor closure and two unchanged mutable range authorities plus
one paged range authority are rebound to their current source. No new proof or
exemption is added. Applicable portable and final hosted acceptance remain open.

Current qualification completes the focused contract: strict.log exits 0;
mutable-classification-controls.log passes all five selected controls; the seven
unchanged public-query-controls cases pass, and the typed-case oracle adaptation
passes in public-query-typed-control.log. The oracle preserves all four authored
ordinary rows plus the new reifier, and retains exact engine/carrier comparisons.
scalar-wasm-regression.log executes the complete new persisted regression in
Node/WASM and passes. helpers.log exits 0 for the production correction; the
later public-fixture-only edit leaves all shipping proof identities unchanged.
No elapsed-time acceptance or foreign consumer execution is claimed. See
reviews/implementation-completeness.md for the remaining hosted/integration gates.
