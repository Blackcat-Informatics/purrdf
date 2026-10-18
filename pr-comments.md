# PR #598 — comments and review threads

8 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/598?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

<!-- review_stack_entry_end -->
<!-- recent_review_start -->

No actionable comments were generated in the recent review. 🎉

<details>
<summary><strong>ℹ️ Recent review info</strong></summary>
<dl>
<dd>

<details>
<summary><strong>⚙️ Run configuration</strong></summary>
<dl>
<dd>

- **Configuration used**: defaults
- **Review profile**: CHILL
- **Plan**: Team
- **Run ID**: `f5318de4-bada-4937-82bb-3993dc33b369`

</dd>
</dl>
</details>

<details>
<summary><strong>📥 Commits</strong></summary>
<dl>
<dd>

Reviewing files that changed from the base of the PR and between 5a279b272bc9c2e8f1569fc114301fff0d2694d1 and bec31ed26dada71d914d84d113bfceac9e9029a6.

</dd>
</dl>
</details>

<details>
<summary><strong>📒 Files selected for processing: 6</strong></summary>
<dl>
<dd>

* `crates/entail/src/owl_dl/proof.rs`
* `crates/entail/src/owl_dl/proof/schema_tests.rs`
* `crates/rdf-core/src/lib.rs`
* `crates/rdf-core/src/ownership.rs`
* `crates/retrieval/src/fusion_stream.rs`
* `helpers-ledger.toml`

</dd>
</dl>
</details>

**Included review availability:** This review used your included allowance. 0 included reviews remain after this review. Your included PR review attempts over the past 7 days set your current allowance at 1 review per hour.

</dd>
</dl>
</details>





<!-- recent_review_end -->
<!-- walkthrough_start -->

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>

The change adds a shared owned-or-borrowed storage type in `purrdf-core`. Proof contexts and retrieval profiles now use it instead of local wrappers. Helper ledger entries are added or updated.

**Changes: Shared storage and callers**

|Layer / File(s)|Summary|
|:---|:---|
|**Add shared ownership representation** <br> `crates/rdf-core/src/ownership.rs`, `crates/rdf-core/src/lib.rs`, `helpers-ledger.toml`|`OwnedOrBorrowed` stores a value or an immutable reference and implements `Deref`. Tests cover moving and dropping owned and borrowed non-`Clone` payloads. The helper ledger adds the new helper job and updates constructor-source hashes.|
|**Use shared storage in proof contexts** <br> `crates/entail/src/owl_dl/proof.rs`, `crates/entail/src/owl_dl/proof/schema_tests.rs`|`DlProofContext` and its prepared-context construction use `OwnedOrBorrowed`; the stop-signal test matches the owned variant.|
|**Use shared storage in retrieval profiles** <br> `crates/retrieval/src/fusion_stream.rs`|`FusionStream` stores its profile as `OwnedOrBorrowed`; its owned and borrowed constructors pass the corresponding variants.|

<!-- change_assessment_start -->
**Priority:** ➖ Normal









**Estimated code review effort:** 3 (Moderate) | ~20 minutes

<!-- change_assessment_commit:"bec31ed26dada71d914d84d113bfceac9e9029a6" -->
**Change:** Refactor · **Severity of issue fixed:** Medium
<!-- change_assessment_end -->

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<details>
<summary><strong>Merge Risk:</strong> <strong>🟡 Moderate</strong> · up to <code>bec31</code></summary>
<dl>
<dd>

<!-- final_review_risk_start -->
<!-- final_review_risk_label
**Merge Risk:** **🟡 Moderate** · up to `bec31`
-->
<!-- final_review_risk_coverage:{"sourceCommitId":"bec31ed26dada71d914d84d113bfceac9e9029a6","coveredCommitId":"bec31ed26dada71d914d84d113bfceac9e9029a6","kind":"reviewed"} -->

The native census passed, but the required full helper-hygiene run remains incomplete because a locked dependency was unavailable. Complete that check before merging.
<!-- final_review_risk_end -->

</dd>
</dl>
</details>
<!-- pre_merge_checks_walkthrough_start -->

<details>
<summary><strong>Pre-merge checks: <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> 4 · <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> 1</strong></summary>
<dl>
<dd>

<strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v2.svg" width="14" height="14" align="middle" alt="Failed" title="Failed"></picture></sup> Failed checks: 1 warning</strong>

|     Check name     |                                              Status                                              | Explanation                                                                                                                                                                                               | Resolution                                                                         |
| :---------------- | :----------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------- |
| Docstring Coverage | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 53.85% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 13 functions across 5 files. (1 skipped: … | Write docstrings for the functions missing them to satisfy the coverage threshold. |

<details>
<summary><strong><sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v2.svg" width="14" height="14" align="middle" alt="Passed" title="Passed"></picture></sup> Passed checks: 4 passed</strong></summary>
<dl>
<dd>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :------------------------ | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|     Linked Issues check    | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The PR satisfies the coding requirements in `#597`. `crates/rdf-core/src/ownership.rs` defines one generic `OwnedOrBorrowed<'a, T>` carrier with inline `Owned(T)` and immutable `Borrowed(&'a T)` variant… |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The changes stay within `#597` scope. The new core carrier, caller migrations, duplicate removal, ownership tests, and helper-ledger updates directly support shared helper hygiene and ownership requirem… |
|      Description Check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the main change: introducing shared inline immutable owned-or-borrowed storage for multiple production callers.                                                 |

<details>
<summary><strong>Full details: Linked Issues check</strong></summary>
<dl>
<dd>

**Explanation**

The PR satisfies the coding requirements in `#597`. `crates/rdf-core/src/ownership.rs` defines one generic `OwnedOrBorrowed&lt;'a, T&gt;` carrier with inline `Owned(T)` and immutable `Borrowed(&amp;'a T)` variants. Its `Deref` implementation requires neither `Clone` nor `ToOwned`. Tests cover a non-`Clone` payload, pointer identity, move/drop behavior, and borrowed-value lifetime. `crates/entail/src/owl_dl/proof.rs` and `crates/retrieval/src/fusion_stream.rs` use the shared carrier and remove their private duplicate storage types. The reported validation covers both production callers, affected strict all-target Clippy, the ownership controls, the compile-fail lifetime test, retrieval tests, normal hooks, and the complete helper-hygiene target. The PR reports no census or proof-policy change and keeps `#596` separate.

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Out of Scope Changes check</strong></summary>
<dl>
<dd>

**Explanation**

The changes stay within `#597` scope. The new core carrier, caller migrations, duplicate removal, ownership tests, and helper-ledger updates directly support shared helper hygiene and ownership requirements. The ledger constructor-source hash updates preserve affected proof records while retaining their sinks, authorities, classes, and reasons. No unrelated semantic feature, shipping dependency, boxed or cloned prepared knowledge base, census exemption, or `#596` causal-support implementation is identified.

</dd>
</dl>
</details>

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Docstring Coverage</strong></summary>
<dl>
<dd>

**Explanation**

Docstring coverage is 53.85% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 13 functions across 5 files. (1 skipped: 1 unsupported.)

**Resolution**

Write docstrings for the functions missing them to satisfy the coverage threshold.

</dd>
</dl>
</details>

</dd>
</dl>
</details>

<!-- pre_merge_checks_walkthrough_end -->

- [ ] <!-- {"checkboxId":"585bb3f6-faf5-4dbf-96d2-74e382adf19a"} --> Fix all pre-merge checks with AI
<!-- finishing_touch_checkbox_start -->

<details>
<summary><strong>✨ Finishing Touches: 💡 1</strong></summary>
<dl>
<dd>

<!-- finishing_touch_suggestion:docstrings -->
<details open>
<summary><strong>📝 Generate docstrings: 💡</strong></summary>
<dl>
<dd>

- [ ] <!-- {"checkboxId":"3e1879ae-f29b-4d0d-8e06-d12b7ba33d98"} --> Commit to this branch
- [ ] <!-- {"checkboxId":"7962f53c-55bc-4827-bfbf-6a18da830691"} --> Create a new PR

</dd>
</dl>
</details>
<details open>
<summary><strong>🧪 Generate unit tests (beta)</strong></summary>
<dl>
<dd>

- [ ] <!-- {"checkboxId": "6ba7b810-9dad-11d1-80b4-00c04fd430c8", "radioGroupId": "utg-output-choice-group-unknown_comment_id"} --> Commit to this branch
- [ ] <!-- {"checkboxId": "f47ac10b-58cc-4372-a567-0e02b2c3d479", "radioGroupId": "utg-output-choice-group-unknown_comment_id"} --> Create a new PR

</dd>
</dl>
</details>

</dd>
</dl>
</details>

<!-- finishing_touch_checkbox_end -->

<!-- autopilot:start -->
- [ ] <!-- {"checkboxId":"2708ad07-9f24-4260-9c11-7dc76a49f2e3"} --> <strong title="Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts">Autofix</strong> · Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts
<!-- autopilot:end -->
<!-- tips_start -->






<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

The complete `make helpers-hygiene` target now passes. Strict affected all-target Clippy, six ownership controls, four independently prepared proof-context tests, all 114 retrieval tests, and the actual borrowed-context compile-fail test passed. Normal commit and push hooks passed.

The constructor source audit found exactly one added module and no changed prior constructor sources or named capacity authorities. The new module contains inline owned/reference variants, immutable dereference and non-Clone ownership tests; it creates no term IDs or capacity indexes. Only the constructor-set references in the existing proof rows were renewed. Their sinks, authority lists and invariant reasons remain unchanged.

Review must cover both actual caller migrations, non-Clone and zero-copy storage, lifetime/drop semantics, removal of both duplicates, and preservation of the capacity audit. The full diff and issue contract are ready for independent review. Hosted CI and CodeRabbit are still pending; local qualification does not claim either passed.

## 3. paudley — 

@coderabbitai Please check the complete acceptance contract for #597 against this diff before merge: one shared implementation used by both actual production callers; both obsolete enums and accessors removed; inline owning storage, exact immutable borrowing, no Clone or ToOwned requirement, no borrow allocation, and preserved move/drop/lifetime behavior; no new dependency, feature, census exemption or weakened capacity audit.

Local evidence available for your assessment: strict all-target Clippy for core, entailment and retrieval passed; six core ownership controls passed; four independently prepared proof-context tests passed; all 114 retrieval library tests passed; the actual borrowed proof-context compile-fail test passed; normal commit/push hooks and the complete helpers-hygiene target passed. I found that the public borrowed fusion entry point needs its existing boundary suite as additional execution evidence, and that suite is running now.

The constructor audit compares the native census before and after this change: exactly the new ownership module joins the conservative constructor set, all prior constructor sources and all named capacity authorities are unchanged, and the 40 existing proof rows change only their constructor-set references. Please challenge that renewal as well as the caller migrations. Report any missing requirement or unsound completion claim; the separate causal graph in #596 remains unfinished and is not part of this repair.

## 4. paudley — 

The additional public-entry check passed: `cargo test --locked -p purrdf-retrieval --test boundary` ran all 21 tests successfully. This covers the actual borrowed `fuse` path as well as the exported evaluator-to-fusion bridge. Together with the full library suite, both production storage choices now have execution evidence. No source change or weakened assertion was required.

## 5. paudley — 

Share inline immutable owned-or-borrowed storage

Closes #597.

Prepared native proof contexts and retrieval profiles now use one OwnedOrBorrowed implementation in their existing core dependency. Both duplicate private enums and accessors are removed. Owning storage remains inline; immutable borrowing retains the original value and lifetime without Clone, ToOwned, allocation or a mutable accessor.

The helper ledger enforces the shared home. Manual constructor-source review found exactly one added module and unchanged existing constructors and capacity authorities. Only the constructor-set references in the 40 existing proof rows were renewed; their sinks, classes, authorities, reasons and the audit policies are unchanged.

Validation: full helpers-hygiene, strict affected all-target Clippy, two new carrier controls within the six-test ownership selection, four independently prepared proof-context tests, all 114 retrieval library tests, all 21 public boundary tests, and the actual borrowed-context lifetime compile-fail test passed. The public boundary suite covers the borrowed fuse entry point and exported evaluator bridge. Normal commit and push hooks passed.

CodeRabbit completed its substantive review with no actionable code comments and explicitly passed the linked-issue and scope checks. Targeted function documentation addresses its coverage warning. Its incomplete helper execution is resolved by the actual complete local gate; no missing dependency is concealed. The final source delta is documentation plus the reviewed source-reference renewal, so the runtime evidence and substantive review remain applicable. Local analysis is self-review; no independent local audit is invented.

The predicted integration is clean and the base has not advanced, so the affected qualification covers the actual candidate. Hosted CI is pending on the documentation update; no full-workspace or packaged WASM pass is claimed. The user authorized integration after passing applicable checks while later CI findings receive visible issue ownership.

Authoritative plan: .stage/share-owned-or-borrowed-storage-to/plan.md. Selected evidence directory: .stage/share-owned-or-borrowed-storage-to. The unfinished causal dependency graph remains owned by #596.

Defect-Class: duplicate-implementation

## 6. coderabbitai — 

<!-- This is an auto-generated reply by CodeRabbit -->
> [!TIP]
> For best results, initiate chat on the files or code changes.

<details>
<summary>🧩 Analysis chain</summary>

🏁 Script executed:

```shell
#!/bin/bash
set -euo pipefail
gh issue view 597 --repo Blackcat-Informatics/purrdf --json title,body,state,url
gh pr view 598 --repo Blackcat-Informatics/purrdf --json baseRefOid,headRefOid,state,mergeStateStatus,statusCheckRollup,files
git status --short
git rev-parse HEAD
gh pr diff 598 --repo Blackcat-Informatics/purrdf
ast-grep outline crates/rdf-core/src/ownership.rs --items all
ast-grep outline crates/entail/src/owl_dl/proof.rs --items all
ast-grep outline crates/retrieval/src/fusion_stream.rs --items all
fd -H '^(plan\.md|.*ownership.*|.*helpers.*|.*boundary.*)$' . --exclude .git
```

Length of output: 76909

---

🏁 Script executed:

```shell
#!/bin/bash
set -euo pipefail
sed -n '2680,2845p' crates/entail/src/owl_dl/proof.rs
sed -n '2410,2555p' crates/retrieval/src/fusion_stream.rs
rg -n -C3 '\b(ProofKnowledgeBase|ProfileInput|borrowing_prepared_kb|of_prepared_kb)\b|FusionStream::borrowed|prepared.*context|compile_fail' crates/entail crates/retrieval
ast-grep outline scripts/check-shared-helpers.py --items all --match 'constructor|capacity|fingerprint|source'
rg -n -C4 'constructor|source_fingerprint|logical-capacity-panic' scripts/check-shared-helpers.py
ast-grep outline crates/retrieval/tests/boundary.rs --items all --type function
fd -H '^plan\.md$' .stage 2>/dev/null || true
python - <<'PY'
import subprocess,tomllib
base='5a279b272bc9c2e8f1569fc114301fff0d2694d1'
old=tomllib.loads(subprocess.check_output(['git','show',base+':helpers-ledger.toml'],text=True))
new=tomllib.load(open('helpers-ledger.toml','rb'))
oj={j['id']:j for j in old['job']}; nj={j['id']:j for j in new['job']}
print('ADDED JOBS',set(nj)-set(oj),'REMOVED JOBS',set(oj)-set(nj))
for k,a in oj.items():
 b=nj[k]
 if a==b: continue
 print('CHANGED JOB',k,'changed top fields',[x for x in set(a)|set(b) if a.get(x)!=b.get(x)])
 pa=a.get('capacity_proof',[]);pb=b.get('capacity_proof',[])
 print('PROOF COUNTS',len(pa),len(pb))
 for i,(x,y) in enumerate(zip(pa,pb)):
  changed=[f for f in set(x)|set(y) if x.get(f)!=y.get(f)]
  if changed: print('PROOF',i,'CHANGED',changed,'CONSTRUCTORS',x.get('constructors'),y.get('constructors'))
 authorities={v.split('=source:')[0] for p in pb for v in p.get('authorities',[])}
 changed_auth=[]
 for p in sorted(authorities):
  before=subprocess.check_output(['git','show',base+':'+p])
  after=open(p,'rb').read()
  if before!=after: changed_auth.append(p)
 print('AUTHORITY FILES',len(authorities),'CHANGED AUTHORITY FILES',changed_auth)
PY
```

Length of output: 33379

---

🏁 Script executed:

```shell
#!/bin/bash
set -euo pipefail
rg -n -C3 'constructor.set|constructor_set|constructors_fingerprint|constructor_sources|logical-capacity-panic' --glob '*.rs' --glob '*.py' .
ast-grep outline crates/entail/src/reasoner/borrowed_tests.rs --items all
sed -n '1,240p' crates/entail/src/reasoner/borrowed_tests.rs
sed -n '540,585p' crates/entail/src/reasoner/mod.rs
sed -n '1000,1060p' crates/retrieval/tests/boundary.rs
sed -n '1790,1880p' crates/retrieval/tests/boundary.rs
sed -n '1,120p' scripts/check-shared-helpers.py
rg -n -C2 'ownership|nonclone|non_clone' crates/rdf-core --glob '*.rs'
git ls-files .stage/share-owned-or-borrowed-storage-to
gh api repos/Blackcat-Informatics/purrdf/pulls/598/reviews --jq '.[] | {user:.user.login,state,body}'
gh api --paginate repos/Blackcat-Informatics/purrdf/pulls/598/comments --jq '.[] | {user:.user.login,path,line,body}'
```

Length of output: 51605

---

🏁 Script executed:

```shell
#!/bin/bash
set -euo pipefail
ast-grep outline crates/helper-census/src/capacity.rs --items all --match 'constructor|source'
rg -n -C18 'fn constructor_identity|fn source_identity|capacity_constructors' crates/helper-census/src/capacity.rs
rg -n -C12 'capacity_constructors|fn source_identity|fn source_fingerprint' crates/helper-census/src
rg -n -C3 'fn .*fingerprint|fn .*hash' crates/helper-census/src
gh pr view 598 --repo Blackcat-Informatics/purrdf --json headRefOid,state,statusCheckRollup --jq '{headRefOid,state,checks:[.statusCheckRollup[]|select(.status=="IN_PROGRESS" or .conclusion=="FAILURE" or .conclusion=="CANCELLED")|{name,status,conclusion}]}'
```

Length of output: 24304

---

</details>

`@paudley` I checked `#597` against the complete diff at `bec31ed26dada71d914d84d113bfceac9e9029a6`. **I found no source-level blocker.** Full execution qualification remains partly dependent on your reported results.

### Acceptance contract

| Requirement | Assessment |
|:---|:---|
| One shared implementation | Both `DlProofContext` and `FusionStream` use `purrdf_core::ownership::OwnedOrBorrowed`. |
| Actual production callers | `Reasoner::proof_context` and `Reasoner::borrowed_proof_context` reach the shared carrier. `FusionStream::new` owns its profile. Public `fuse` reaches `FusionStream::borrowed`. |
| Remove both duplicates | The diff removes `ProofKnowledgeBase`, `ProfileInput`, and both local `Deref` implementations. |
| Inline ownership and exact borrowing | The variants contain `T` and `&'a T` directly. The carrier performs no allocation, boxing, cloning, or reconstruction. |
| No cloning requirement | Neither the enum nor its `Deref` implementation requires `Clone` or `ToOwned`. |
| Immutable access and lifetime | `Deref` returns `&T`. There is no `DerefMut`. Both caller lifetimes remain tied to their borrowed source. |
| Move and drop behavior | Ordinary Rust enum ownership preserves these semantics. Tests check a non-`Clone` payload, pointer identity, one owned drop, and no payload drop when the borrowed carrier drops. |
| Preserve caller behavior | Clause derivation, contract binding, proof checks, and fusion algorithms are unchanged by the diff. |
| No dependency, semantic feature, or audit exemption | No manifest, lockfile, census implementation, or audit-policy file changes appear in the diff. The new ledger job is enforced. |

The prepared-context tests also check independent owned/borrowed replay, exact source identity, question preparation, source refusal, and stop propagation. The lifetime compile-fail example targets the actual `Reasoner` API.

The public boundary tests exercise both hand-built streams through `fuse` and the exported evaluator-to-fusion bridge. Your later report that all 21 boundary tests passed addresses the execution gap identified in the original comment.

### Capacity proof renewal

I independently confirmed:

- All **40** existing proof rows remain present.
- Each row changes **only** its `constructors` reference.
- Sinks, classes, authorities, and invariant reasons remain unchanged.
- All **23 named authority files** are byte-identical to the base.
- The added ownership module contains no term-ID or capacity-index construction.

The native audit hashes a conservative constructor-source set. Thus, a newly visible module can require renewal without adding an actual capacity constructor. **I found no weakening of the capacity audit.**

I did not independently reproduce the exact native constructor-set delta or the new aggregate hash. Please provide the before/after native census output so that part can be verified.

### Remaining qualification limits

- I did not run repository code. Clippy, helper hygiene, tests, and hooks remain author-reported execution evidence.
- `ownership.rs` contains **two test functions**. Please provide the commands or output that account for the reported **six ownership controls**.
- Hosted workspace and multiple test checks were still running in the retrieved snapshot. The standalone `wasm` check had passed, but `wasm-package` and `wasm-exec` remained pending.
- Merge through `ghprsq` remains a procedural requirement of `#597`.

Your qualified completion statement does not claim a full CI pass. That distinction is sound. This review does not establish completion of `#596`.

Thanks for the audit clarification.

---

<details>
<summary>✏️ Learnings added</summary>

```
Learnt from: paudley
URL: https://github.com/Blackcat-Informatics/purrdf/pull/598

Timestamp: 2026-10-18T00:06:56.000Z
Learning: In purrdf, the helper-census capacity constructor identity covers a conservative source set, including the frozen lexical owner's visible module domain. A newly visible module can require renewal of existing capacity proof constructor references without adding a term-ID constructor. Review such renewals against the source-set delta, existing constructor sources, named authority sources, and unchanged proof sinks, classes, authority lists, and invariant reasons.
```

> Note: Learnings are effective only in the context of similar code segments. To apply general coding guidelines, please configure [review instructions](https://docs.coderabbit.ai/guides/review-instructions). You can manage existing learnings in the [app](https://app.coderabbit.ai/learnings).

</details>

_You are interacting with an AI system._

## 7. paudley — 

The review checklist concerns are addressed.

Function documentation now covers the shared dereference, non-Clone ownership/drop controls, the proof stop fixture, and the retrieval storage constructors and debug renderers. Strict affected all-target Clippy passed after that documentation change, and normal commit/push hooks passed.

The complete `make helpers-hygiene` target also passed again on this final version. It executed the native census self-tests, shared-helper wrapper self-tests and full census, then the domain-registry self-tests and full registry check. The final output confirms 83 enforced ledger jobs, 118 distinct rows and all 134 registered domains. The complete run finished successfully with the required locked dependencies available.

The constructor-set references were renewed only after reviewing the documentation delta. All prior constructor sources and named capacity authorities, and all proof sinks, classes and invariant reasons, remain unchanged. Runtime and lifetime tests are reused because no executable code or assertion changed after their successful runs. Broad hosted CI is still pending; no complete matrix result is claimed.

## 8. paudley — 

The six-test count comes from `cargo test --locked -p purrdf-core --lib ownership`. There are exactly two new carrier tests; the broad name filter also selects four existing ownership tests. All six passed:

```text
ownership::tests::borrowed_nonclone_payload_is_the_original_and_outlives_the_carrier
ownership::tests::owned_nonclone_payload_moves_and_drops_once
small::tests::every_ownership_path_drops_each_element_once
diagnostic::tests::shared_attachments_preserve_original_reporting_and_admitted_ownership
xsd_regex::xpath::cache::tests::shared_ownership_reuses_current_admission_without_holding_the_cache
small::tests::ownership_matches_vec
```

The native before/after constructor audit ran `cargo run -q --locked -p helper-census -- --root <checkout> --index` against the unchanged base checkout and this issue worktree. Projecting the native results to source-set comparison fields gives:

```text
baseline constructor sources: 164
final constructor sources: 165
added: crates/rdf-core/src/ownership.rs
removed: none
changed existing constructor sources: none
```

All 164 original constructor-source identities match; every named proof authority is unchanged. The complete ownership module was reviewed: its Debug derives conservatively place it in the set, while the carrier and tests create no term IDs or capacity indexes. The later doc attributes change its recorded source identity, so its constructor reference was renewed again after the documentation-only review. Every other proof field remains unchanged.

Raw before/after native index projections, the source-set comparison, and the full local logs are selected for the structured merge evidence. The complete final `make helpers-hygiene` target passed, including both census and domain registry stages and their self-tests. This clarifies the exact local execution evidence; it does not claim independent rerunning or a full hosted matrix pass. The PR and final notes now distinguish the two new tests from all six filter matches.

