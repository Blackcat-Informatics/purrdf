# PR #581 — comments and review threads

2 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->

> [!WARNING]
> ## Review limit reached
> 
> Your organization has reached its usage spending cap. Adjust your spending cap in the [billing tab](https://app.coderabbit.ai/settings/billing?tab=usage&orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> **Next included review available in 10 minutes.**
> 
> [Check out review usage here](https://app.coderabbit.ai/dashboard/review-capacity?orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> <details>
> <summary><strong>View limit details</strong></summary>
> <dl>
> <dd>
> 
> **Limit details:** You’ve used all 2 included reviews currently available. Your 89 included PR review attempts over the past 7 days set your current allowance at 2 reviews per hour.
> 
> [Learn how review limits work](https://docs.coderabbit.ai/management/rate-limits).
> 
> **Review configuration:**
> 
> <details>
> <summary><strong>⚙️ Run configuration</strong></summary>
> <dl>
> <dd>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `e41a0139-489c-4528-b021-5d1ae6c08c36`
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📥 Commits</strong></summary>
> <dl>
> <dd>
> 
> Reviewing files that changed from the base of the PR and between 0994332d93bc89747f95e5a942715faf36129648 and bfd8e4af9d437110719c9a5c9fc155fe81c3f92c.
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📒 Files selected for processing: 3</strong></summary>
> <dl>
> <dd>
> 
> * `Makefile`
> * `crates/entail/src/owl_dl/parser.rs`
> * `crates/entail/tests/regular_role_chains.rs`
> 
> </dd>
> </dl>
> </details>
> 
> </dd>
> </dl>
> </details>

<!-- end of auto-generated comment: rate limited by coderabbit.ai -->

<!-- autopilot:start -->
- [ ] <!-- {"checkboxId":"2708ad07-9f24-4260-9c11-7dc76a49f2e3"} --> <strong title="Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts">Autofix</strong> · Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts
<!-- autopilot:end -->
<!-- tips_start -->






<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

fix(entail): retain source witnesses for class-expression role refusals

Closes #580. Advances the producer repair within #558 without claiming its complete structured-error inventory or the downstream class-model ownership move.

Class-expression role parsing converted its typed failure directly, bypassing the source-witness home used by other DL producers. Once the private interner died, consumers received a local term ID without the original source spelling. CeExtractor::role_of now invokes the existing with_source helper with its actual interner before conversion. This is one production-line change: typed classification, source arguments and original error chain survive failure without a new error type, grant, formatter, dependency, premise filter or fallback. The canonical helper and accepted-input path are unchanged.

A public regression covers annotation-property IRIs and literal restriction roles in both ordinary and proof-recording construction, with no role-chain path to mask the missing class-extractor producer. It drops the source dataset before inspecting typed InvalidProperty classification, named DiagnosticValue text, source spelling and the original typed error chain. The old code fails at missing presentation, exit 101; the repaired code passes all four combinations. The portable case is registered in the existing harness and exact WASM Make inventory.

Validation passes: all 17 public role cases, all 56 public reasoner service cases, 718 entailment library cases with one ignored, strict all-target entailment clippy with warnings denied, and three actual WASM controls covering the new matrix, RBox source retention and cancellation. Every normal hook and push passes. The source tree and deficiency ledger are clean. There is no local GMEOW corpus producer, machine/resource/environment probe, profile weakening or hook bypass.

CodeRabbit explicitly refuses review because the organization usage cap is reached, with the next included review reported in 10 minutes. No independent review or full hosted CI pass is claimed. The user explicitly instructed ghprsq now with subsequent CI findings handled; local acceptance is complete and external checks remain pending. No implementation requirement is moved to follow-up. The original malformed production input is still a refusal, not a consistency answer; its actual source can now be diagnosed on a downstream rerun.

Integration is clean against unchanged fetched base 0994332d93bc89747f95e5a942715faf36129648, predicted tree 2312e735662612c023b38a2c09565b003ea7b210. No conflict or intervening base interaction invalidates the evidence. The authoritative plan and selected evidence are .stage/retain-source-terms-for-class. Concurrent issue-558 work and all sibling worktrees remain untouched; unrelated REWORK_LIST.md is preserved.

Commit bfd8e4af9: fix(entail): retain source witnesses for class-expression role refusals.

Defect-Class: none

