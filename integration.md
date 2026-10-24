# Integration assessment

Actual remote main is 6c9c66fcf, PR head de2095b46. The PR baseRefOid remains
the original e3eb29225; baseRef.target is the current integration tip. The native
merge-tree command exits 0 with tree baa4fca159ce3acfab5559c5cbffa142c0626d2c,
matching potential merge 69077f350981a0c44425b5429d3e08253cc4d2ee.

The base delta is the independently qualified OpenAPI local-reference closure in
json_schema.rs and its output fixtures. This branch changes only the run-scoped
conformance key, original reference-plan definition lookup and catalog regression.
Neither side changes dependencies, shared public signatures, model schemas,
generated products, build configuration or the other's data path. OpenAPI schema
export does not execute the conformance memo; the definition lookup retains its
existing API. Source inspection establishes separate behavior despite the shared
Shapes crate. No synchronization commit is needed solely for being behind.

The completed workspace job's original log is retained as
ci-logs/workspace-current.txt. Its checkout explicitly fetches and checks out
69077f350981a0c44425b5429d3e08253cc4d2ee, merging de2095b46 into 6c9c66fcf.
Thus hosted pull-request run 40056143633 exercises the assessed combined tree,
not merely two separate green inputs. Full run 40056143633 completes successfully
with 46 jobs and no failures. The final PR check rollup has no unsuccessful entry.
The final fresh fetch still produces the same clean candidate against main.
