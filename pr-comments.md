# Pull request feedback

Head: `f1bad25c953bbfc911f7472f8567a58479d6ef27`.

Coverage:

```json
{
  "comments": "complete",
  "reviews": "complete",
  "inline_comments": "complete",
  "threads": "complete"
}
```

## comment `github:comment:IC_kwDOTKq-Ms8AAAABfOm7Qw`

[Original source](https://github.com/Blackcat-Informatics/purrdf/pull/711#issuecomment-6390659907)

Author: coderabbitai. State: `comment`.

Technical review projection for final head
f1bad25c953bbfc911f7472f8567a58479d6ef27.

The final Makefile increment reviewed 13744dde2 through f1bad25c9. The exact-charge
increment covered fcad5d31d through 13744dde2; substantive implementation review
covered base 0b67da9bf through a60e405b6. All three changed files are covered. No actionable current-head defect or merge-blocking risk is established.
Merge risk is minimal; architecture risk is low, with no retained concern.

The review describes private frame/group/candidate owners and the shared work
list replacing native recursive child searches. Parents resume with child
hashes and issuers. Permutation ordering, budget charging, lexicographic
pruning/ties, issuer adoption and final hashing remain in the production path.
The native 256-node chain runs on a 128 KiB stack and checks issuer order and
remaining budget. Constructor authorities use source:a0a182593706ad6a;
other proof fields remain unchanged.

The formal exact-charge test finding is addressed by the strengthened existing
case. It consumes 256 frame entries and 510 related-group permutations, exactly
766 units. Focused native execution, full source/helper qualification and normal
signed hooks/push pass. No new case or production runtime branch was added by
that correction.

The initial architecture assessment observed all-or-nothing result publication,
existing search charging and cancellation controls. It retained uncertainty
about peak heap consumption and allocator failure: WorkList::push ultimately
uses infallible Vec::push, and work units do not meter heap bytes. The baseline
already allocated issuer copies and search containers; no introduced regression
was established. Caller-owned byte admission and explicit allocation refusal
remain original active parent requirements, not a completion claim here.
Deployment-specific host/tenant exposure was not supplied.

Title, description, linked-issue coding requirements and scope checks pass.
The direct target is 710; configurable budgets, dataset-state controls and heap
UDF execution remain outside this unit under 489. The profile and dependencies
are unchanged. The implementation has no recursive fallback or semantic WASM
copy. Docstring coverage is reported as 100 percent over six functions, with one
unsupported item skipped.

No formal approval or independently rerun test qualification is inferred.
Any review wording implying hosted success is governed by actual final CI
evidence, not adopted while checks are pending. Decorative UI, navigation,
billing, automation controls/prompts and poems are omitted from this projection.

The final Makefile review covers all four SmallVec Miri selectors. Each selects
purrdf-core library cases matching small::tests::. Default, Tree Borrows, strict
provenance and i686 models remain. The selection correction preserves the
unsafe-storage obligations and avoids running the native semantic stack fixture
under Miri. No actionable comment was generated for this increment. Current
architecture assessment covers the three changed files and retains no concern.


## comment `github:comment:IC_kwDOTKq-Ms8AAAABfO0tsQ`

[Original source](https://github.com/Blackcat-Informatics/purrdf/pull/711#issuecomment-6390885809)

Author: paudley. State: `comment`.

Addressed the exact-charge finding in the existing deep-chain case. Its
256 frame entries and 510 related-group permutations now assert exactly
766 work units. The focused case passes on the same 128 KiB native stack;
no test case or production runtime branch was added.

The source census retains the same 173 constructor files and every original
sink. Aggregate bindings are refreshed and the complete helper gate passes.
The prior behavioral, frozen-conformance and strict production qualification
remains applicable because the review fix changes the test assertion only.

The review's memory-containment limitation remains explicit under the active
parent requirements; this bounded change removes native recursion and makes
no byte-admission guarantee.

Review: https://github.com/Blackcat-Informatics/purrdf/pull/711#pullrequestreview-5640555936


## comment `github:comment:IC_kwDOTKq-Ms8AAAABfPOmpQ`

[Original source](https://github.com/Blackcat-Informatics/purrdf/pull/711#issuecomment-6391309989)

Author: paudley. State: `comment`.

The cancelled Miri job exceeded its 30-minute deadline, as confirmed by the
check annotation. Its broad `small` filter selected the new canonicalization
stack case, spending over 15 minutes there before the intended storage cases.

The gate now targets `--lib small::tests::` under each existing model. All 18
SmallVec cases remain in every model, and the BLAKE3 probe is unchanged. The
complete corrected `make miri` gate passes locally: four sets of 18 storage
cases plus the streaming probe. No deadline or assertion was relaxed. The
semantic canonicalization case remains in normal native coverage.


## review `github:review:PRR_kwDOTKq-Ms8AAAABUDQNoA`

[Original source](https://github.com/Blackcat-Informatics/purrdf/pull/711#pullrequestreview-5640555936)

Author: coderabbitai. State: `COMMENTED`.

Technical formal-review projection.
Source review: https://github.com/Blackcat-Informatics/purrdf/pull/711#pullrequestreview-5640555936
Reviewed head: fcad5d31d7efa9f3bc54847df32c8de82d7ec9ea.
Review state: COMMENTED; this is not an approval.

One finding, classified trivial functional correctness, concerns
crates/rdf-core/src/ir/canon.rs lines 5598 through 5638:
the new deep-chain test's loose budget comparison cannot detect a missing or
extra frame-entry or permutation charge. The expected fixture total is
3 * COUNT - 2: one entry per blank, one permutation at each endpoint and two
at each internal node. The suggested replacement is:

```rust
assert_eq!(
    state.budget,
    RDFC_CALL_LIMIT - (3 * COUNT as u64 - 2),
);
```

Addressed in 13744dde282cdd5fc7cd2e91ca410b5cebfca23b by strengthening the existing
assertion. The focused native case proves exactly 766 units on the same 128 KiB
stack. Source/helper qualification and normal signed hooks/push pass, and the
disposition and public response are recorded. No other substantive finding is
present in this formal review.

Its increment was a60e405b6 through fcad5d31d; canon.rs was selected but also
reported similar to prior changes. This coverage limit is retained.
Decorative UI, billing, automation instructions and repeated generated prompts
are omitted; the technical finding and code suggestion are retained.

