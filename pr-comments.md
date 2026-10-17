# PR #583 — comments and review threads

2 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/583?cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: review in progress by coderabbit.ai -->

> [!NOTE]
> Currently processing new changes in this PR. This may take a few minutes, please wait...
> 
> <details>
> <summary><strong>⚙️ Run configuration</strong></summary>
> <dl>
> <dd>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `75cd2971-6460-4e97-9900-d38f1cbe8853`
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
> Reviewing files that changed from the base of the PR and between bcd5bd5711734a6fc7087ca08a04b05120990bc2 and 6fbf766a216188e353619cfc9bf4eadac2c3bec7.
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📒 Files selected for processing: 15</strong></summary>
> <dl>
> <dd>
> 
> * `Makefile`
> * `crates/datalog/src/stop.rs`
> * `crates/entail/Cargo.toml`
> * `crates/entail/src/lib.rs`
> * `crates/entail/src/owl_dl/mod.rs`
> * `crates/entail/src/owl_dl/proof.rs`
> * `crates/entail/src/owl_dl/query.rs`
> * `crates/entail/src/reasoner/mod.rs`
> * `crates/entail/tests/recorded_stop.rs`
> * `crates/rdf-core/src/governor.rs`
> * `crates/rdf-core/src/ir/canon.rs`
> * `crates/rdf-core/src/ir/mod.rs`
> * `crates/rdf-core/src/lib.rs`
> * `crates/rdf/src/lib.rs`
> * `docs/WASM_TESTING.md`
> 
> </dd>
> </dl>
> </details>
> 
> ```ascii
> +-----------------------------------------------------+
> | Bugs were harmed in the making of this code review. |
> +-----------------------------------------------------+
>   \
>    \     /)/)
>         ( '')
>         /  |
>       o(___)
> ```

<!-- end of auto-generated comment: review in progress by coderabbit.ai -->

<!-- finishing_touch_checkbox_start -->

<details>
<summary><strong>✨ Finishing Touches</strong></summary>
<dl>
<dd>

<details open>
<summary><strong>📝 Generate docstrings</strong></summary>
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

# Cancellable recorded DL preparation

Issue 582 supports the required native class boundary in GMEOW 1803. Preserve the unrelated REWORK_LIST.md and text-index worktree/PR. Repository goals, baseline, ring-fence, one-home rule, RDF 1.2 canonical profile, fixed poison budget and existing build profiles apply. No resource probes/caps, local GMEOW corpus producer, clock, new dependency or hook bypass is permitted.

## Task 1: One stop interface and controlled canonicalization

Move the existing latching StopSignal trait and optional-signal poll to core governance; Datalog re-exports that exact implementation, with no second trait or adapter. Add a controlled DatasetView canonicalization entry point and typed CanonError::Stopped. Share the existing canonical algorithm. Poll before source work, while collecting/admitting components and nested terms, at first-/n-degree and permutation work, and while serializing output. Do not publish partial canonical bytes. Preserve complete bytes/labels, hash selection, reserved-vocabulary/read diagnostics and fixed poison-budget charging. Keep uncontrolled wrappers on the same code path.

## Task 2: Recorded DL preparation with the same signal

Add a recording constructor taking an explicit schema preparation budget and caller signal. Compute canonical input identity cooperatively, map typed cancellation to EntailError::Stopped, and retain the same signal in native ontology preparation/search. Reuse one identity implementation from existing recording constructors. Preserve successful proof identity and replay, source diagnostics and unknown/stopped decisions. This does not claim shared-store/delta or rooted proof support is complete.

## Task 3: Verify, publish, integrate

Check immediate and mid-operation cancellation, symmetric blank search, RDF 1.2 statement-layer handling, SHA-256/SHA-384 complete parity, existing canonical refusals and exact poison budget. Exercise public recorded DL construction, native proof replay, preparation and search cancellation. Run affected core/Datalog/entail tests, strict affected Clippy and real WASM checks with existing profiles, then normal hooks. Publish a focused PR, collect CodeRabbit feedback, use ghprsq under existing authorization, archive evidence and clean up only owned work. No independent review is fabricated when unavailable. GMEOW pin/adapter adoption follows the actual native merge.

