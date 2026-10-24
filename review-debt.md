# PR 691 review debt

VERDICT: PASS — required final-head CI passes, independent review/reconciliation is complete, and every thread is resolved.

The current source is ea133b288. This disposition index records the primary
agent's evidence assessment; independent review comes from the captured PR
reviews, not this document. The final stable head has been explicitly submitted
for review because automatic reviews paused after d0ef5e401.

| Feedback | Disposition and evidence |
|---|---|
| 4364860635, annotated/local-reference object choices | Fixed and pushed in 0b115e5bc using the existing guarded element-kind resolver and annotation predicate. All 38 LinkML library tests and strict Shapes all-target Clippy pass (review-choice-tests.log, review-choice-strict.log). The bot accepted the source repair and resolved the thread; review-choice-ack.txt and review-threads-0b.json record that disposition. |
| Current-head assembly documentation | Five native configurations measure v4/f0/r0 for the corrected production symbol; both WASM configurations pass with v0/f0/r0. Documentation cells/prose corrected from actual CI logs; no manifest/gate weakening. All seven final-head configurations and the aggregate now pass in run 40041027523. |
| Current-head pytest / LinkML oracle | Fixed and pushed in ea133b288. Named TypeDefinition dropped scalar/object and mixed-enum alternatives, and the official 1.11 generator closes a named choice wrapper over its empty local attributes. The repair borrows original slot expressions, preserves conjunctions, guards cycles/stack margin and bounds expression count by source bytes. The unchanged original oracle, 41 native LinkML and 139 projection controls, strict lint, helper checks and normal hook pass; see slot-carrier-validation.md. Final-head hosted pytest also passes. |
| Final-head independent review | Completed through ea133b288. bot-status-ea133b.json/md explicitly records that last-reviewed commit and no actionable comments, with coverage metadata kind=reviewed. Both changed Rust files were processed; the SIMD doc row was skipped as similar to previously reviewed material and is independently checked by the seven-platform assembly jobs. No formal new review submission was created for the empty review, so its published walkthrough is the evidence. |
| Merge-risk summary: property cardinality | Addressed by source/evidence, not a scope waiver. The byte-unchanged raw-data guard runs before shape/declaration collection in prepare_with_expressions. All 12 native controls pass, including singleton rejection and both repeated property paths; final-head SHACL conformance passes. Posted in final-review-risk-disposition.md/log. |
| Merge-risk summary: OpenAPI references | Verified pre-existing packaging defect, now issue 692. The complete openapi_doc function and frozen main golden are byte-unchanged from d26bc6e64. The baseline artifact has #/$defs/Annotation and Literal refs but no root $defs or any $id. Exact source comparison and baseline JSON are retained in openapi-baseline-*; this is separate work under the user's frozen scope and the instruction to report unrelated pre-existing failures. No new OpenAPI regression is accepted. |
| Security architecture proposal: eager semantic closure | Legitimately declined for this delivery. On-demand lowering and constant cold-root cost are part of the published behavior; eager validation of every retained definition would change that contract. Reached unknown references hard-refuse in preparation/binding before validation/report serialization, with a native UnknownReference/zero-allocation clone control. The 65 product-refusal controls pass. No eager whole-catalog closure claim is made. Reply posted. |
| Organization docstring coverage warning, 74.92% vs 80% | Legitimately declined broad 622-function/72-file documentation campaign under the explicit user scope freeze. This informational UI warning is not a failing required repository CI gate; final workspace/doc checks pass, no compiler/documentation gate or coverage setting was weakened. Exact quote and disposition posted. |
| 4364279822, rules target model fallback | Fixed in 96b0ba3d3. The same resolved original model reaches lowering and focus selection. The bare-root targetWhere/derived-value regression passes in the 989-test native library run. Reply posted; thread resolved. |
| 4364279849, parser body/property inspections | Fixed in 96b0ba3d3, with integration fixture callers completed in 2443eec1b. Assertions inspect original definitions and property identities. All 65 parser-module tests pass; the current native integration repairs preserve values, paths, sources and failure assertions. Reply posted; thread resolved. |
| 4364279859, restored private component/target contexts | Fixed in 96b0ba3d3. Contexts are populated before private definitions are parsed. The complete-domain control includes a custom component and parameterized target and passes natively. Reply posted; thread resolved. |
| 5634506638 review-body performance nit, repeated planned lookup | Fixed in 96b0ba3d3. Node/property evaluation binds the planned constraint once and reuses it. Existing validation/allocation controls pass without raising ceilings. |
| CI regression inventory | Issue 690 records the failures and their repairs. Native original failure groups are cleared through full runs plus focused rechecks; current hosted results are still required. GraphQL wrong-kind acceptance was reproduced by the official oracle and fixed in d0ef5e401; LinkML shared object-choice classification is fixed in 2443eec1b and its 139 projection/29 ontology controls pass. |

Initial independent review covered the published diff and later increments
through 2443eec1b; the resulting fourth finding is fixed in 0b115e5bc and the bot
accepted and resolved it. The final review request is final-slot-review-request.md
and its posted log. bot-status-ea133b.json/md records the completed review from
2443eec1b through ea133b288 with no actionable comments. The general risk,
hardening and coverage statements were read in full and reconciled in
final-review-risk-disposition.md/log; issue 692 owns the unrelated baseline bug.
Required current-head checks now pass: CI run 40041027523 is completed/success on
ea133b288; hosted-checks-ea133b-final.txt reports state: pass. No gate was waived.
review-threads-ea133b-final.json has zero unresolved threads and no uncollected
thread/comment page. Final mandatory checks will be refreshed before ghprsq.

## Independent reconciliation of general risks

The reviewer independently inspected the current source and compared the base,
then explicitly withdrew the general risk summary as a basis for additional
changes in this PR. It confirmed no PR-specific cardinality counterexample,
byte-identical baseline OpenAPI packaging owned by 692, safe missing-reference
refusal, and the decision against a broad coverage campaign. It reports no
additional actionable finding and requires only final current-head CI before
merge. Full response: independent-risk-reconciliation.json/md, public comment
https://github.com/Blackcat-Informatics/purrdf/pull/691#issuecomment-6377318138.
This response supersedes the older generic risk label in the sticky walkthrough.
The reviewer explicitly did not rerun tests; actual execution evidence is indexed
separately in validation.md and the current hosted check capture.

Before changing this verdict, refresh all paginated review submissions, inline
comments and unresolved threads, then reconcile any new findings and current CI.
The queue delivery scope is publication-scope.md; the complete solver in issue
569 remains outside this user-frozen PR. No independent completion of that
larger issue is claimed.
