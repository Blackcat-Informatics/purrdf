# Current technical review

Current source review: complete through a010ace7b. The independent sticky review
at issuecomment-6380583352 was updated at 08:16:27 UTC and explicitly records
sourceCommitId/coveredCommitId a010ace7b3e826ddd1c8d1b70d2ebfcd8eb08964 with kind
"reviewed". Its current merge risk is Low and it identifies no correctness or
architecture defect. The completed commit status is success. The separate
review submission remains attached to the initial e3c2b93b2 head; it is not
misrepresented as a new submission. Full technical text and coverage metadata
are in raw/current-comments.json and raw/current-reviews.json.

The initial repeated-work finding was remediated before landing: matching owned
bindings share their immutable data, class pairs are retained and their term
dictionary is built lazily once, and each actual graph maps its own handles.
Current review notes the remaining bounded subclass setup cost. Actual fresh
graph handle/membership work remains part of this operation; there is no
unfulfilled acceptance item or promised after-merge performance repair. Original
default allocation bounds and the growing-payload controls pass. This is a
scope/cost assessment, not a claim that the implementation has zero setup work.

The documentation metric is a nonblocking warning in the successful review:
73.17% across touched functions, against the bot's 80% threshold. That heuristic
also counts internal/trait methods. Introduced public methods are documented;
strict rustdoc passed for all three changed crates. No repository instruction
sets this percentage as an acceptance gate, and no hook, configured gate or
scope requirement has been waived. The linked-issue check is inconclusive while
full hosted results are pending, which agrees with the actual readiness state.

CI run 40076759527 passed with 42 successful jobs and four configured skips.
The live watcher exited with status 0 and was collected. All 53 current PR checks
are terminal with success or configured skips. Native paginated capture confirms
no inline comments and no review threads. The refreshed clean merge preview
against unchanged bf85c9174 is a459bbf4e119572f4e68ee478823802eb64cd317, the
qualified source tree with no additional base delta. No required review item or
acceptance criterion remains open for this child. Parent fixed-point/corpus/host
contracts remain open. Normal ghprsq publication and archive verification remain.

## Prior implementation assessment and remediation history

The implementation review traced the actual registered custom/built-in producers,
all production registry-binding callers, shared request carriers, planned and
standalone expressions, targets, native rule heads/conditions, fresh rule views,
profile fallback and complete report/product admission. Original declaration
owners remain in the function closures; each invocation reads call.focus_graph.
Canonical default requests and empty registries remain allocation-free. Large
severity/graph payloads are shared; registry clones share immutable bindings.
Nested registries hold a weak request link and the root registry retains the
request, avoiding a cycle without depending on the factory Arc's allocation.

The former weak registry link would fail after Arc::try_unwrap; replaced it before
publication. The native retained-registry test now moves out of both factory
Arcs, proves zero-allocation cloning, drops original inputs and registries, then
executes nested calls under alternating requests and graphs. No missing context
is silently accepted after its required owner was released.

The model census correctly refused the request-field type change. The codec
still obtains options from the current request, while its derived stage and
current fixture were regenerated through the supported native generator. All
219 final affected cases and final strict workspace checks pass. Prior 995
library and focused allocation/memo/host/XPath results cover unchanged execution
paths; the later product batch closes the changed stage/fixture paths.

Independent review of the initial head identified repeated shapes-graph work.
The suggested after-merge optimization was not accepted as a deferral. Current
remediation shares matching owned bindings within the existing scope and retains
immutable subclass configuration once per callback/rule operation. Weak original
owner anchors prevent pointer reuse without retaining raw source payloads. Scope
reuse requires the same environment, request owner, declarations, source and graph
name. Weak nested registries cannot escape as new independent request owners.

Static subclass dictionaries include all supplement IRIs, never a first graph's
missing subset. Fresh actual snapshots independently map their own handles and
build their own membership analysis. Stored failures propagate as typed errors;
no missing context or resource failure is converted to default success. Borrowed
core lookups preserve first-source canonical identities and graph selections.
New native controls verify these ownership, isolation and allocation properties.
The existing allocation golden remains byte-identical; exact allocation counts
decreased. Catalog expansion counts/bytes are now pinned at each existing depth,
so a smaller fixed cost cannot accidentally invalidate a ratio envelope.

This remediation assessment is the implementer's review. Independent final-head
review and all required hosted checks remain pending; no merge readiness is
claimed. Parent fixed-point/corpus/host scope remains fully open.

The workspace gate caught stale core capacity-source proofs after the borrowed
lookup edit. Reviewed the actual owner/constructor delta and compared native
helper indexes against clean main. Constructor membership is unchanged at 173;
all 40 existing full sink addresses/body fingerprints remain exact. Refreshed
only their constructor identities and the two changed file authorities. Existing
proof reasons, classes, members and sinks remain unchanged. The full native
helper check and normal signed commit hooks pass. This proof-only metadata
commit is ce5b5f8a2 and does not invalidate the runtime/strict source evidence.
Final-head CI/review still must pass; the previous run was automatically
superseded after the new push and is not claimed green.
