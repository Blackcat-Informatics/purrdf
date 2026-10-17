# PR #592 — comments and review threads

3 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/592?cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

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
> - **Run ID**: `d17575eb-2af7-4abd-b759-986650b182ef`
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
> Reviewing files that changed from the base of the PR and between 9f70e55e92abb747df771372c9b270e202b39b9a and aa2a75789a8ce561d4a393c134f29f23cef56ffe.
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📒 Files selected for processing: 7</strong></summary>
> <dl>
> <dd>
> 
> * `Makefile`
> * `crates/sparql-eval/src/eval.rs`
> * `crates/sparql-eval/src/expr.rs`
> * `crates/sparql-eval/src/lib.rs`
> * `crates/sparql-eval/src/workspace.rs`
> * `crates/sparql-eval/tests/numeric_wasm_determinism.rs`
> * `docs/WASM_TESTING.md`
> 
> </dd>
> </dl>
> </details>
> 
> ```ascii
> +---------------------------------------------------------------------------+
> | I love a good shortcut. This is a great shortcut to production incidents. |
> +---------------------------------------------------------------------------+
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

# Borrowed native SPARQL equality

Prerequisite for the observed GMEOW1843 single-value metadata failure. Full logic goal stays active. Actual downstream tiny production-gate test reports7 passes and one failure: distinct IRI values yield no comparison violation. Existing ORDER BY total ordering is not FILTER equality. Reuse the native owned-value equality implementation, which already handles recursive triple components and SEP-0009; do not copy semantics into GMEOW, render query text, construct a dataset per production comparison, or weaken errors.

## Task 1: Expose the owning implementation

Publish equal_values over two borrowed TermValues, explicit LanguageStringEquality and caller-supplied WorkspaceCapability. Expose the existing language-string enum instead of inventing another profile type. Core and Disjoint remain deliberate choices. Delegate to rdf_equal_in_admitted; preserve boolean/expression-error/operational-refusal states. Re-export from the evaluator facade; umbrella already re-exports it. No new dependency or stored relation, no GMEOW namespace in native code. Review actual private helper and admission paths first.

## Task 2: Prove the public native and WASM doors

Add one portable case in the existing numeric_wasm_determinism harness. Compare public equality with the actual query equality operator over tiny native datasets containing the original values: IRIs, scopes, numeric lexical variants/NaN, typed/language/directional literals, triple terms and composites, including unknown/ill-typed expression errors. Include explicit disjoint profile and admitted storage refusal where the supported admission fixture permits. Existing equality-path unit controls should also reach the public wrapper. Register the additional portable case and update exact inventory counts. Focused native/public-runtime and WASM tests, strict affected Clippy, helper hygiene and normal hooks. No GMEOW corpus, fixture factory, optimized producer or full gate; no host/resource probes/caps/banned commands.

## Task 3: Publish and integrate

Self-review only: no independent dispatcher available and none claimed. Gather full issue comments/prior art and PR reviews/threads/CodeRabbit, resolve actual findings, commit/push normally and merge only via ghprsq. Existing user authority permits hosted CI pending. Archive exact Stage before owned cleanup. Then update the downstream compiler/runtime to actually execute equality/distinctness and make the failing required-single test pass; public API alone does not complete GMEOW1843.

## 3. paudley — 

Expose borrowed native SPARQL value equality

Refs #590

## What changed

Expose the existing admitted SPARQL equality operation as equal_values for consumers holding original native RDF values. The old private name is removed and its internal expression callers use that same implementation. Export the existing LanguageStringEquality enum so callers explicitly select Core or Disjoint; retain their original WorkspaceCapability. Boolean answers, expression errors and operational refusals remain distinct. No copied equality engine, query rendering, per-comparison store, dependency, profile or governor change.

The actual public native and baseline WASM case checks14 hand-expected value pairs against the query equality operator: IRIs, scoped blanks, numeric lexical variants/NaN, typed/language/directional literals, triples, composites and expression errors. Both pass, with0.08s WASM runtime. A separate native bounded-account control preserves the original provider refusal identity7 and releases every charge. Strict final affected all-target Clippy, complete helper hygiene, normal commit and push hooks passed. One additional case is registered in the existing portable harness; inventory is145 named cases/150 executions across25 targets/29 invocations.

Self-review only; independent dispatch was unavailable and no independent audit is claimed. Fresh submitted reviews, inline comments and review threads are empty, with no omitted page. The bot is processing, not approval; its optional generated-tests/docstrings controls are informational. Hosted checks remain pending with no PR592 failure at inspection. Existing user instruction permits merging now and owning subsequent findings. Native589 packaged WASM/Python conformance passed; its separate imported-closure proof replay failure is explicitly owned by591 and is not repaired here. No whole-hosted-CI success is claimed. Keep590 open while hosted qualification is pending.

Authoritative plan: .stage/sparql-expose-borrowed-native-value/plan.md. Selected Stage: .stage/sparql-expose-borrowed-native-value. It retains complete issue/prior-art/review evidence, actual native/WASM/admission/Clippy/helper/hook logs and failed589 CI evidence. No GMEOW corpus, fixture factory, optimized producer, full gate, host/resource probes/caps or banned commands were used. Unrelated root REWORK_LIST.md and the separate GMEOW1843 worktree remain preserved. The downstream equality-filter integration and the full logic backlog remain active; this public API alone does not complete1843.

## Commits squashed (1)

- `aa2a75789` Expose borrowed native SPARQL value equality

## Files

```
Makefile                                           |   1 +
 crates/sparql-eval/src/eval.rs                     |   2 +-
 crates/sparql-eval/src/expr.rs                     |  20 +++-
 crates/sparql-eval/src/lib.rs                      |   4 +-
 crates/sparql-eval/src/workspace.rs                |  27 +++++
 .../sparql-eval/tests/numeric_wasm_determinism.rs  | 129 +++++++++++++++++++++
 docs/WASM_TESTING.md                               |   5 +-
 7 files changed, 180 insertions(+), 8 deletions(-)
```

## Conflicts

None. The fetched base has no additional delta; clean integration candidate699cc9b6098b86bb31165b8b5bf47774939a0cb1. Qualification covers the source head on that base; no new combined behavior is introduced by integration.

---

Defect-Class: missing-intrinsic-runtime

