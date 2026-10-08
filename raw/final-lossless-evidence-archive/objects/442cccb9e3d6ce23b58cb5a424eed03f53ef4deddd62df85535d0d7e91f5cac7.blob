<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Targeted ordinary/contextual admission recheck

Preliminary source judgment: the new typed preparation boundary resolves the production ordinary-transform escape identified in the earlier review. No demonstrated route was found from caller-built Apply into the ordinary engine/SHACL prebinding or retained prebinding-memo transformations. Executed rejection controls and final source binding remain unqualified; this is not Task 1 PASS or a complete compiler re-review.

## Inspected identity and authority

Issue worktree `/home/paudley/Active/purrdf/.worktrees/454-rdflib-shim-algebra-level-reassignment`, branch `paudley/454-rdflib-shim-algebra-level-reassignment`, HEAD/base `18489a91cb452b1773c60b469d4c30dc33a9a4a4`. Applicable governance and plan remain those read in `core-laws-preliminary.md`. No planning audit was repeated. Source is changing under the sole writer; hashes identify the read window, not a frozen final tree.

Read-window SHA-256 of `git diff --binary`: `23f88312b92a4ff520e25c559b553b77d07bed8629b167a0d462ad214f6c26b0`.

| Inspected file | SHA-256 |
|---|---|
| `crates/sparql-algebra/src/validate.rs` | `cf8bc2b847c360d351837b7489feac997d0715a84ab1f65c565cc58bf979fbf6` |
| `crates/sparql-algebra/src/walk.rs` | `0be7f62613ef6c246f02cc4a3b1da571ab4f45fc64d64dabfb0c329ce157c657` |
| `crates/sparql-eval/src/engine.rs` | `f2322feae4b27406fb886715ac31a8c7a6fd13714c4c31e02902ee2d3255a580` |
| `crates/sparql-eval/src/substitute.rs` | `e1d9321aede3f6f9135919edf09f116639649ccc832623f86f65de1290b1c8b9` |
| `crates/sparql-eval/src/prebind_memo.rs` | `9001c254f5073a09b08c94f2c411df9a261851a1bc0b602839bda3b178a307fe` |

Read actual constructors, public query/prepared/SHACL/fallible/execution/function-body callers, validator and expression-child traversal, public raw evaluation doors, and the hidden fixture rewrite hook. No candidate builds/tests, shipping edits, forge changes, toolchain changes or timing measurements were performed. Only this evidence report was created; no temporary processes/checkouts or debris need removal.

## Actual production boundary

`NativeSparqlEngine::prepare_algebra` and `PreparedQuery::rewritten` call private `PreparedQuery::from_algebra::<false>`. That calls `admit_algebra_with::<false>`, then `admit_structure_with::<false>`, then `Query::validate_ordinary`. The existing const-generic structural work-list walk checks every reached pattern with `check_pattern::<false>`. Apply immediately returns the actionable structural error `contextual application requires typed contextual preparation` before ordinary feasibility planning or later substitution.

The walk enumerates expression operands and EXISTS bodies through shared NodeRef children. Thus an Apply hidden inside FILTER/BIND/order/aggregate expression EXISTS is still reached and rejected. A top-level non-Apply root does not bypass this policy. The guard also precedes registry planning, so callers cannot conceal Apply behind a configured extension and then enter the ordinary path.

The text-based native cache preparation paths use the ordinary parser, which cannot produce Apply; cached preparation and prepare_execution use only those parsed/admitted trees. PreparedQuery's query, fingerprints and constructors are private, and it has no public writable field or unchecked conversion. The prepared/governed/fallible/SHACL entries accept that admitted immutable type, not an arbitrary Query. Function bodies are prepared from text through the same native admission homes. SHACL transformation helpers remain crate-private and receive ordinarily prepared algebra on production routes.

The explicit compatibility preparation alone calls `from_algebra::<true>`. It constructs `PreparedRdflibQuery` with a private enclosed Arc<PreparedQuery>. Its public query accessor returns only &Query, which a caller may clone but cannot convert to an ordinary PreparedQuery without passing the rejecting constructors above. Its execution functions internally supply an empty substitution list. No Deref, public Arc accessor, From conversion or exposed setter was found that would permit ordinary substitutions or SHACL mode to be applied to the contextual prepared handle.

The internal shared prepared evaluator still accepts its private underlying PreparedQuery, which is necessary for the typed route to share engine execution. That internal use does not create a public preparation bypass.

## Earlier visitor observation is superseded by representation changes

OptionalApplication no longer owns an Expression condition. It now holds retry input metadata and a forget marker; the actual condition is an ordinary Filter in the stored RHS/row continuation. NodeRef's Apply child enumeration therefore correctly visits left and right patterns without a third policy expression. The prebind-memo and ordinary substitute walkers no longer omit a policy expression that exists in this representation.

Ordinary substitutions still deliberately stop at Apply and do not rewrite its correlation input policy. On the observed production APIs, the rejecting admission boundary makes that behavior unreachable. The typed contextual route uses the separate declared-correlation substitution contract, so this is no longer an unresolved requirement to teach ordinary SHACL rewriting the contextual law.

## Exact limits and remaining controls

The public raw `eval(pattern, ctx)` and `evaluate_query(query, ctx)` doors can still receive caller-built Apply. They evaluate algebra directly and do not accept/apply ordinary request or SHACL substitutions, so this does not demonstrate the identified silent ordinary-transform failure. Do not claim that every public symbol rejects Apply; the enforced boundary is ordinary prepared/request transformation admission. Raw evaluation retains its existing lower-level structural/hidden-variable/depth contract.

`fixture::shacl_prebinding_rewrite` is also technically public through a doc-hidden fixture module. Its source explicitly says it is shared test/bench support, not API, and it takes an arbitrary Query without structural admission. It can therefore be handed Apply and invoke the ordinary test rewrite that stops at it. This is a test-support caveat, not an observed production SHACL escape. Keep production-boundary claims scoped accordingly; fixtures do not establish an unchecked engine admission constructor.

No new targeted executed tests were read for this boundary during this review. Required focused controls are: a structurally valid caller-built Apply rejected by both prepare_algebra and PreparedQuery::rewritten; the same Apply nested in EXISTS rejected through both doors; ordinary counterpart algebra admitted and retaining existing prebinding/SHACL results; and a plan produced by prepare_rdflib_query continuing to execute through its typed wrapper. Rejection assertions must inspect the contextual-admission diagnostic rather than pass because a malformed policy, registry or unrelated syntax check failed first. Existing deep-walk and native controls can support these checks but are not a substitute for executing the boundary witnesses.

No necessary shipping-source fix was established by this targeted recheck. The source-level production escape finding is resolved in the inspected representation; final executed controls, source identity and logical native-cost qualification remain required. Broader compiler laws and earlier findings are outside this narrow recheck and remain subject to their own repaired-source evidence.
