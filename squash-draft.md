Preserve authored ordinary rows across sealed stack layers

An ordinary row whose subject is a reifier in another layer disappeared from
ordinary queries and appeared as an annotation. Both sealed stack adapters now
preserve the row's authored kind, independent of partitioning and layer order.
Mutable input still infers annotations through the original shared classifier.
Remove the obsolete external-declaration promotion lookup and document the
sealed-row contract. Keep annotation demotion, removal chronology and faults.

The public persisted regression first reproduces the original failure, then
passes all six three-layer permutations, combined-image and no-declaration
controls, default and named graphs, stream counts and keyed object values. All
37 affected native stack cases, five mutable-inference cases and eight actual
SPARQL consumer cases pass within their recorded scopes. The same persisted
regression passes through the existing scalar WASM runtime. Strict affected
compilation and helper census pass. Normal commit and push hooks passed.

Independent hosted CodeRabbit review covers all eight changed files through
cef34c8aa and found no actionable findings. Its generic docstring warning counts
touched tests/private helpers; the production home and G11 contract describe
the behavior, with no undocumented new public API. The owner completion review
separately maps the whole issue to actual execution and does not claim to be
independent approval. Foreign Katamari test paths were not run here.

Full hosted qualification passes: 49 successful checks and four expected skips,
with no failures or pending checks. Run 38809158043 includes every workspace
test shard, strict compilation/hygiene, portable builds/executions, all seven
assembly configurations and installed host surfaces. Existing gate parity and
the complete workflow establish the equivalent final gate; no duplicate local
whole-workspace run is added. The final refreshed review collections contain
no inline findings or unresolved threads.

Integration assessment: origin/main remains the original base. The clean
merge-tree candidate is 52bb86ee8f37e3001befc29d2a68c1e5aefab2da, with no
additional base delta or untested combined changes. Refresh if main advances.
The pending statistics branch is not part of this integration; it must assess
the corrected stream counts and constructor proof bindings when synchronizing.

The implementation keeps one classification home, introduces no dependency or
row-layout change, and remains portable. No timing acceptance, tooling override,
subagent or deferred part of the issue is introduced.

Authoritative plan: .stage/segmented-stack-reads-an-ordinary-quad/plan.md.
Selected evidence: .stage/segmented-stack-reads-an-ordinary-quad.
Current index and owner completion review are linked there; actual independent
hosted review is retained in tasks/hosted-comments.json.

Closes #556.
Defect-Class: correctness
