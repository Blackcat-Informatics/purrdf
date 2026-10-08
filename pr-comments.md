# PR #504 — comments and review threads

3 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/504?cs_source=review_comment"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=2" alt="Review in Change Stack →" width="220" height="32"></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: review in progress by coderabbit.ai -->

> [!NOTE]
> Currently processing new changes in this PR. This may take a few minutes, please wait...
> 
> <details>
> <summary>⚙️ Run configuration</summary>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `a1f7fa06-abe2-4be3-83bf-38c7ce8f8cdc`
> 
> </details>
> 
> <details>
> <summary>📥 Commits</summary>
> 
> Reviewing files that changed from the base of the PR and between df2cec88b0f8e05e05ef3d01d14714474be6bbb1 and 08e7cd585dfecfb335d258c187c14c2ce75e388d.
> 
> </details>
> 
> <details>
> <summary>📒 Files selected for processing (58)</summary>
> 
> * `CHANGELOG.md`
> * `bindings/python/python/src/purrdf/__init__.pyi`
> * `bindings/python/python/src/purrdf/compat/rdflib/graph.py`
> * `bindings/python/python/src/purrdf/compat/rdflib/query.py`
> * `bindings/python/src/py_store/quad_store.rs`
> * `bindings/python/src/py_store/query.rs`
> * `bindings/python/tests/README.md`
> * `bindings/python/tests/test_contextual_mappings.py`
> * `bindings/python/tests/test_solution_row_protocol.py`
> * `crates/purrdf/src/reasoning.rs`
> * `crates/rdf-core/src/small.rs`
> * `crates/rdf/src/projections/dataset_description.rs`
> * `crates/shapes/src/prebinding.rs`
> * `crates/shapes/src/profile.rs`
> * `crates/shapes/src/shapes/parser/admission.rs`
> * `crates/shapes/src/srl/reads.rs`
> * `crates/slice/src/ownership.rs`
> * `crates/sparql-algebra/src/algebra.rs`
> * `crates/sparql-algebra/src/owned.rs`
> * `crates/sparql-algebra/src/parser.rs`
> * `crates/sparql-algebra/src/parser/machine.rs`
> * `crates/sparql-algebra/src/parser/triples.rs`
> * `crates/sparql-algebra/src/retained_size.rs`
> * `crates/sparql-algebra/src/serialize.rs`
> * `crates/sparql-algebra/src/traits.rs`
> * `crates/sparql-algebra/src/validate.rs`
> * `crates/sparql-algebra/src/walk.rs`
> * `crates/sparql-algebra/tests/iterative_traits.rs`
> * `crates/sparql-algebra/tests/serializer_roundtrip_sweep.rs`
> * `crates/sparql-eval/src/basic_profile.rs`
> * `crates/sparql-eval/src/bgp.rs`
> * `crates/sparql-eval/src/binop.rs`
> * `crates/sparql-eval/src/blank_scope.rs`
> * `crates/sparql-eval/src/cdt_agg.rs`
> * `crates/sparql-eval/src/construct.rs`
> * `crates/sparql-eval/src/engine.rs`
> * `crates/sparql-eval/src/eval.rs`
> * `crates/sparql-eval/src/expr.rs`
> * `crates/sparql-eval/src/governor/soundness.rs`
> * `crates/sparql-eval/src/lib.rs`
> * `crates/sparql-eval/src/modifier.rs`
> * `crates/sparql-eval/src/prebind_memo.rs`
> * `crates/sparql-eval/src/property_fn_eval.rs`
> * `crates/sparql-eval/src/property_fn_plan.rs`
> * `crates/sparql-eval/src/rdflib.rs`
> * `crates/sparql-eval/src/remote.rs`
> * `crates/sparql-eval/src/row_checkpoint.rs`
> * `crates/sparql-eval/src/service_endpoints.rs`
> * `crates/sparql-eval/src/substitute.rs`
> * `crates/sparql-eval/tests/fixtures/contextual-mappings.json`
> * `crates/sparql-eval/tests/native_xpath.rs`
> * `crates/sparql-eval/tests/rdflib_contextual.rs`
> * `crates/xsd/src/exact/cost.rs`
> * `docs/CONFORMANCE.md`
> * `docs/book/po/zh-Hans.po`
> * `docs/book/src/interop/rdflib.md`
> * `docs/book/src/sparql/querying.md`
> * `docs/design/purrdf-simd.md`
> 
> </details>
> 
> ```ascii
>  ______________________________________
> < My code review levels are over 9000! >
>  --------------------------------------
>   \
>    \   \
>         \ /\
>         ( )
>       .( o ).
> ```

<!-- end of auto-generated comment: review in progress by coderabbit.ai -->

<!-- finishing_touch_checkbox_start -->

<details>
<summary>✨ Finishing Touches</summary>

<details open>
<summary>📝 Generate docstrings</summary>

- [ ] <!-- {"checkboxId":"3e1879ae-f29b-4d0d-8e06-d12b7ba33d98"} --> Commit to this branch
- [ ] <!-- {"checkboxId":"7962f53c-55bc-4827-bfbf-6a18da830691"} --> Create a new PR

</details>
<details open>
<summary>🧪 Generate unit tests (beta)</summary>

- [ ] <!-- {"checkboxId": "6ba7b810-9dad-11d1-80b4-00c04fd430c8", "radioGroupId": "utg-output-choice-group-unknown_comment_id"} --> Commit to this branch
- [ ] <!-- {"checkboxId": "f47ac10b-58cc-4372-a567-0e02b2c3d479", "radioGroupId": "utg-output-choice-group-unknown_comment_id"} --> Create a new PR

</details>

</details>

<!-- finishing_touch_checkbox_end -->

<!-- autopilot:start -->
- [ ] <!-- {"checkboxId":"2708ad07-9f24-4260-9c11-7dc76a49f2e3"} --> <strong title="Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts">Autopilot</strong> · Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts
<!-- autopilot:end -->
<!-- tips_start -->

---




<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Algebra compilation for rdflib reassignment

## Issue, identity and governing contract

Issue 454 requires the compatibility Graph.query surface to answer every variable-reassignment form as genuine rdflib 7.6 does, including OPTIONAL, MINUS, EXISTS, GROUP BY, SELECT star, nested groups and combinations. It requires removal or proven unreachability of UnmodelledReassignment and preservation of answers that already work. This is not restricted to targets present in initBindings: repeated or already-in-scope assignments without initial bindings are included.

Worktree: `/home/paudley/Active/purrdf/.worktrees/454-rdflib-shim-algebra-level-reassignment`.
Branch: `paudley/454-rdflib-shim-algebra-level-reassignment`.
Initial base/head: `18489a91cb452b1773c60b469d4c30dc33a9a4a4`; tree `5d150fa4076cc75e293b517f79dd080b6ea807ed`.
The live captured issue has zero comments. Source is unchanged; `.stage/` holds process evidence separately from shipping files. Plan publication, task commits and final PR/merge are authorized by the user's descending issue-queue instruction. This plan does not claim implementation acceptance.

Additional user steering: confirm no copied implementation and no degradation of primary Rust performance. Original first-party implementation is mandatory; upstream source inspection and oracle execution supply behavior rather than copied implementation bodies/tables. The user explicitly forbids time-based performance measurement on this host and requires logical proof. Primary native Rust cost requires structural qualification before merge, not an assertion that a disabled compatibility mode has no cost.

Read governing AGENTS, root `.baseline` and `.goals`, relevant parser/evaluator contracts and empty `.deficiencies`. No governing ADR/CONSTITUTION was found or cited by the issue. Rust-first core, maximal utility/performance/portability, one home per job, exact RDF 1.2 terms, existing dependency layers, no new dependency or Cargo feature, byte determinism, exact terminals, required signed/unbypassed hooks and protected main apply. Never put issue/process references or tool attribution in shipping code/docs. Preserve the dirty `prebound-values-nested` sibling and all other worktrees. No lifecycle, model or GPU actions are needed or authorized here.

## Current evidence and prior art

The issue's text-rewrite/refusal premise is stale. Ancestor `27c5ef602` removed the old Python rewrite, its named exception and its differential tests without implementing parity. Current Graph.query calls native Store.query; native preparation intentionally constrains assignments by prebinding and native parsing rejects some rdflib-accepted target collisions. Symbol absence is not the requested answer contract.

`issue-analysis.md`, `prior-art-assessment.md`, `reviews/compiler-design-analysis.md` and the specialist feasibility report are the design inputs. Seven relevant ancestor attempts were inspected. Two old rewrite/hardening iterations and their follow-on divergence report demonstrate that global text freshening/COALESCE is insufficient. The subsequent removal is not a successful algebra compiler. Current native fixes preserve deliberate ordinary SPARQL/SHACL semantics; a dirty sibling has different unqualified work and will not be imported. Recurrence across all tracker history is UNKNOWN; the seven inspected ancestor commits and the recovered concrete witnesses are counted, not an invented exhaustive recurrence total.

`raw/rdflib-7.6-oracle-probe.json` contains nine executed genuine-7.6.0 cases and source hashes, including BIND b followed by this-triple returning b with a's objects, OPTIONAL/nested group restoring a, MINUS retaining all three rows, EXISTS yielding none, grouping b/count3, later BIND winning and expression failure keeping a. This is oracle-only evidence. The isolated locked environment is being built for base characterization; its native module must be attributable to this source before recording base answers. No historical pass total or installed root native module is treated as current source qualification.

`raw/expanded-oracle-characterization.json` records 58 successful genuine-oracle answers: the 24 historical inputs, 13 formerly refused inputs, 4 ordered inputs and 17 additional cases. The additional cases include no-init BIND/SELECT/GROUP collisions, duplicate OPTIONAL drivers, nonlazy RHS deduplication, empty explicit/implicit groups and empty projected mappings. These are input/answer receipts, not shim parity. Public iteration suppresses an empty projected actual mapping before constructing ResultRow with its I fallback; internal aggregate/projection mappings must therefore remain distinct from the public iterated row bag. The empty-explicit-group and empty-subselect-init-fallback witnesses have zero iterated rows, while the bound-check witness retains a nonempty mapping and returns true/a.

## Design: compile mapping and context as separate identities

Use the existing tokenizer/parser machine and shared native evaluator. Add an explicitly named compatibility admission/compilation entry, with ordinary parser methods and Store.query retaining strict laws. Compatibility admission permits the oracle's BIND/SELECT/GROUP alias collisions and EXISTS collision forms; it retains the same grammar, terminals, validation, resource accounting and iterative nesting machinery. Preserve structural group/operator boundaries before native normalization so source grouping and the oracle's lazy-Join facts remain available. A small typed parse/compilation representation or stable per-node facts is allowed; a second parser or evaluator is not.

Each compiler case receives graph context C and fixed initial mapping I and returns an actual output mapping R with a native bag. These are distinct, immutable versions with capture-free private variable identities. An R carrier being bound means a key belongs to the actual output domain. Expression lookup resolves the operation-visible R then I; graph positions resolve C. User BOUND uses expression lookup, while internal domain tests use R alone. Operation-specific forget retains keys absent from the incoming C, keys in its `_except` set, and initial-binding names. Preserve syntactic variable summaries independently of native free-variable sets.

Define M(A,B) as right-preferred actual-key merge. Thaw is M(R if R has any actual key else C, I), because the oracle substitutes its prior context for an empty frozen mapping, then reinstates initial bindings. Do not reduce this to one COALESCE over every variable occurrence.

Private-name selection scans all parsed names, including nested EXISTS/subqueries/templates/graph slots and caller parameters. At every edge, input/context carriers and assignment outputs are disjoint. Correlation can substitute declared input carriers without capturing an output target. Project keeps the advertised actual logical mapping and explicit transport; only final output aliases become public names. Private columns must not affect MINUS domains, DISTINCT identity, grouping keys, SELECT star or templates.

| Operator | Structural lowering and required invariant |
|---|---|
| BGP/path/GRAPH/VALUES | Match graph slots under C, return its actual keys and newly matched keys. Contiguous triples share match identities. VALUES keeps context conflict/UNDEF behavior. Graph/context transport is explicit. |
| Extend/BIND/repeated assignment | Evaluate expression over forgotten R with I fallback into a fresh value. Fresh target is the new value on success and prior actual target on expression error. Operational/governor errors remain failures. Same rule applies without I. |
| Lazy Join | Lateral drives RHS under thaw(left); left/right output identities remain disjoint; synthesize left-preferred M(right,left), without imposing ordinary compatibility on overridden logical names. |
| Nonlazy Join | Independently evaluate both sides under C, project canonical actual mapping aliases, DISTINCT the RHS mapping, perform natural compatibility/merge. Hidden transport cannot change equality or deduplication. |
| UNION | Align each arm to common logical output carriers and concatenate bags, preserving absent cells and multiplicities. |
| MINUS | Compare only canonical actual R domains and values. Project/mask all transport aliases before native subtraction; restore context without multiplying rows. A correlated domain-aware predicate is allowed when transport cannot be safely separated. |
| OPTIONAL | Evaluate left once. Per left row, run RHS under thaw(left), retaining a success marker; ordinary unit LeftJoin supplies an unmatched padding row. Matched output restores left priority. Admit unmatched only when the scope-remembered retry has no satisfying RHS. Initial condition uses forgotten R; retry condition uses its row directly. A successful volatile RHS must not be evaluated a second time solely to detect absence. |
| EXISTS/NOT EXISTS | Thaw the expression's frozen binding state and test compiled body nonemptiness through the native correlated seam. Body assignment outputs remain distinct from captured input carriers. |
| Sub-SELECT/projection | Compile under C, restrict actual R to projected names; retain I fallback only for expression reads, not fabricated mapping-domain keys. Preserve projected and unprojected assignment scope. |
| GROUP BY/aggregates | Compute keys from row overrides, rename aggregate targets, preserve the oracle's aggregate/SAMPLE aliases and post-group Extend chain. Context transport is not a grouping key. Empty explicit and implicit grouping need exact default/empty-row witnesses; padding alone must not invent aggregate values or duplicate groups. |
| FILTER/HAVING/ORDER BY | Rewrite reads with the operation's visibility/forget rule and recursively compile EXISTS. Preserve ordered sequences and error contracts. |
| SELECT/star/ASK/CONSTRUCT/DESCRIBE | Derive public/star columns before SSA. Preserve projected actual-domain state through presentation: public SELECT iteration suppresses an empty mapping before ResultRow lookup; I fallback may supply a visible cell only for a retained mapping. Internal empty mappings remain available to enclosing algebra. Template/describe reads use the correct final mapping and fallback where observed. Existing native result/graph homes emit RDF terms directly. |

Compute Join laziness from the preserved oracle structural summary: a Join records conjunction of child summaries and itself returns false upward; Slice/Distinct return false; other nodes propagate children, including embedded expression bodies where relevant. Preserve the oracle's simplification order and left-associative joins. Ordinary parser normalization and SEP scope rules remain unchanged outside the explicitly named compatibility entry.

Compilation prepares owned native algebra through existing `NativeSparqlEngine::prepare_algebra` and executes the prepared plan through its shared dataset/configuration/governor/admission paths. Do not apply ordinary logical substitutions or native prebinding rewrite after I has been structurally encoded. The Python native boundary shares the existing snapshot, registry/env, diagnostics and result materialization plumbing; Graph.query uses that entry, and plugin/inherited Dataset/ConjunctiveGraph/shadow callers reach it.

The specialist source review identifies concrete native limitations, so typed shared operator metadata is part of the core task rather than assuming unannotated native nodes suffice. Correlation carries an explicit C input mask; output aliases/merge policy remain separate. OPTIONAL uses the existing per-driver binop/correlated home with a typed left-precedence/remembered-retry application policy, storing its RHS once. Mapping comparison/DISTINCT sees normalized logical R slots or explicit logical-domain masks. EXISTS retains effective C/I dependencies across synthetic projections, deferred substitutions and memo keys. Forget and empty-map thaw use bound logical cells at runtime, not schema width or static carrier membership. Ordinary operator policies remain unchanged.

The existing-node lowerings are candidates where the laws match, not proof of implementation. Typed support must have complete visitor/admission/governor/wasm coverage and explicit compiled provenance. It must not become a separate compatibility evaluator, global semantic switch or Python fallback. Nested OPTIONAL retry representation is proportional: no exponential subtree cloning hidden behind a refusal. Count lowered representation and execution work; retain one RHS with the shared application policy. Compatibility EXISTS uses the existing definition route when its native positive-probe/memo eligibility cannot be proved, with complete governor/error attribution rather than a guessed cached result. Implementation owns these gaps before its task can pass.

## Completeness contract

| Requirement | Task | Falsifiable acceptance on real input/caller |
|---|---|---|
| Rust algebra rewrite, one production path | 1,2 | Rust compiled-plan execution and actual compat Graph.query return matching answers; no Python query-text mutation/runtime oracle call/unused alternative. |
| Every assignment, including no initBindings | 1,2 | Repeated and already-in-scope BIND/SELECT/GROUP target collisions without I answer like genuine oracle; standard native parser rejects its existing forbidden forms. |
| OPTIONAL | 1,2 | Before/inside/after assignment, RHS triples/filter/errors, matched/unmatched, empty remembered scope, nested retry and volatile successful RHS witnesses. |
| MINUS | 1,2 | Left/right/both assignment, empty/nonempty RHS and shared/disjoint logical domains, transport-only overlap, unbound cells and duplicates. |
| EXISTS | 1,2 | EXISTS/NOT EXISTS, correlated outer values, body/expression assignment, nested EXISTS, sub-SELECT and OPTIONAL/MINUS combinations. |
| GROUP BY | 1,2 | Group key/alias/aggregate collisions, implicit/explicit empty grouping, HAVING/ORDER BY, SAMPLE/count defaults, duplicates and key identity. |
| SELECT star and nested groups | 1,2 | Complete public columns with associated typed cells, no hidden aliases; projected/unprojected sub-SELECT, outer FILTER, nested plain group, repeated assignment and grouped star subqueries. |
| Combinations | 1,2 | Concrete OPTIONAL containing MINUS/EXISTS; nested grouping/star/project; assignments at several levels; deterministic bounded generated compositions alongside the explicit witnesses. |
| No refusal | 1,2,3 | Static symbol absence plus every oracle-answering corpus input succeeds with matching result, without new xfails, renamed exceptions or deleted witnesses. |
| Preserve working cases | 1,2 | Attributed base observations, all 24 historical successful inputs, 4 ordered inputs, 13 formerly refused inputs as successful answers, prefix-name collision control, native/fresh-variable neighbors and existing compatibility suite. |
| Result and term identity | 2 | Bags for unordered SELECT, sequences for ordered results, column-name alignment for star, ASK boolean, graph-semantic CONSTRUCT/DESCRIBE; term kind/value/datatype/language/direction/quoted triple and scoped blanks preserved where supported by both carriers. |
| Empty projected mapping versus visible fallback | 1,2 | Actual compat Graph.query iteration matches the three named expanded-oracle witnesses, preserving empty internal mappings for nested operators while emitting zero public rows for the empty projected mappings and true/a for the retained bound-check row. |
| Native/SHACL/other consumers | 1,2 | Existing native prepared/prebinding/SHACL/conformance tests retain strict laws. Configured extensions/results and plugin/dataset/shadow forwarding execute the actual new route. |
| Portability/resource correctness | 1,2,3 | Affected Rust wasm build, explicit-stack deep query and proportional size witnesses; expression failure does not swallow operational/governor errors; required hooks and relevant hosted matrix pass. |
| Original implementation and primary Rust performance | 1,3 | Review final diff/provenance for original first-party bodies/tables. Prove native parser, preparation, reused execution and correlation/operator paths preserve allocation, representation and algorithmic work from the unchanged base. Pin native control-flow/operation counts, GraphPattern layout, retained-plan bytes and allocation invariants; compile-time admission specialization removes compatibility branches/state from ordinary parsing. Inspect relevant compiled IR/assembly where source alone cannot establish unchanged work. No timing benchmarks on this host. Added native cost or an unproved claim blocks merge until the design/proof is repaired. |

Corpus comparisons retain None separately and preserve duplicate multiplicities. They do not stringify cells into indistinguishable values or compare row sets. Generated compositions are supplemental, never a replacement for the recovered explicit cases. Existing unrelated oracle divergences/ledgers are identified from actual evidence, with no new reassignment ledger entries or softened expected answers. A changed public answer is a finding to resolve, not silently called compatible.

## Enhancements considered

Chosen: typed binding/context state and operator laws provide one reusable compiler within the native algebra homes and remove the prior boundary-level string heuristic. Proportional representation, real RDF terms and complete scope facts improve both portability and diagnosis; they are required to make the requested capability coherent.

Declined: a second Python/upstream evaluator, copied translator bodies, text rewriting, a runtime rdflib dependency, global native scope relaxation, and importing the dirty sibling. Each would undermine one-path/native contracts or unqualified work preservation. Broad language/mapping frameworks, new dependencies, a new public query language, unrelated update changes and performance benchmark infrastructure are outside this repair; focused compiler growth/work evidence covers its actual cost.

## Validation policy

Use the Stage-managed floating nightly and ordinary wrappers, `CARGO_BUILD_JOBS=2`, locked dependencies and existing warning/profile settings. No manual toolchain install/select. Focused development checks cover modified packages and real callers; mandatory signed commit and push hooks run fully. Final broad hosted Rust/workspace/wasm/Python/conformance qualification is required to cover the shared parser/evaluator and compatibility closure. Do not claim a local `make check` was run when CI supplies qualification; do not duplicate whole-suite gates per task.

The affected Python surface also needs a final full binding suite, vendor/shadow lane and installed empty-graph gate, using the repository's actual make/CI runner and collected counts. Run locally where needed to diagnose/matrix acceptance and reuse matching hosted evidence where policy permits. No exact historical test count is assumed. Add Python tests in a new narrow file with concrete Why-not-Rust justification; do not grow ratcheted existing Python tests. Rust owns compiler/unit/property/growth checks. Update maintained generated metadata/counts only through their generators where the final changes require it.

Maintain `validation.md` with source/plan/toolchain/module/oracle identities, commands, exit status, tested commit/tree and missing coverage. Historical failures remain historical receipts. Independent task reviews must assess executed laws and actual wiring, not just source shape or file presence.

For the user's performance requirement, do not run timing benchmarks on this host. Use the task-owned clean detached base checkout at the initial OID for source/cost comparison and existing first-party allocator/structural-check homes. Preserve exact source and compiler/profile/configuration identities for layout, allocation and compiled-code checks. Establish an explicit native-path cost argument over parse, prepare/admission, plan storage/copy/drop, ordinary execution and correlated substitution: each changed seam names its base operations, candidate operations, allocation/space law and reason no extra compatibility work reaches native callers. Const-generic admission specializes one shared parser source so compatibility checks/state disappear from the ordinary monomorphization. Boxed/shared policy data must preserve ordinary algebra layout and allocate only for compiled compatibility nodes. Account for shared enum dispatch/visitor additions with actual layout and compiled IR/assembly evidence where needed. Tests of operation/allocation invariants supplement the proof; elapsed timing and a branch being usually false cannot establish it. This qualification belongs to Tasks 1 and 3 and blocks merge if performance preservation remains unproved. No numeric regression allowance is silently introduced.

## Task 1: Native compatibility admission and complete algebra compiler

Implement the explicit parser admission and preserved typed group/lazy/variable facts, complete C/R/I compiler for every operator, collision-safe identities, production engine compilation entry and any demonstrated minimal shared primitive. Initial scope: `crates/sparql-algebra/src/parser.rs` and machine, algebra support as warranted; `crates/sparql-eval` compiler/engine/shared operator seams and Rust tests. Update all impacted visitor/schema/admission/cache/governor paths when an IR change requires them. No incomplete node or refused operator counts as done.

Execute Rust compiled-plan witnesses for the nine primary probes, recovered historical shapes, no-init collisions, context/mapping fallback, projection/EXISTS correlation, OPTIONAL retry, MINUS domains, grouping defaults, result forms and proportional deep-nesting behavior. Protect current strict parser, `prebound_grouping`, SHACL `prebinding_lanes`, relevant W3C conformance and other binding consumers. Run focused release-profile warning-clean affected Rust checks plus affected wasm compilation. Establish exact production-entry behavior, not parse-only acceptance. Obtain independent task review of compiler laws and evidence. Fix all findings, commit this coherent core unit with hooks/signing, push normally and verify remote head, then update the issue. Task completion requires all its operator laws; surface integration remains explicit Task 2 work.

## Task 2: Wire the compatibility consumers and prove complete oracle parity

Factor/reuse native quad-store query plumbing for the explicit compatibility entry; wire Graph.query and verify processor, Dataset/ConjunctiveGraph and shadow routes. Keep ordinary Store/prepared/governed/SHACL contracts. Add a justified new Python differential file covering every contract row and all recovered inputs, plus current successful neighbors. Baseline characterization records genuine oracle and installed base-module identities/answers before relying on preservation claims. Canonicalize terms/columns/bags/order/graphs using existing test homes where appropriate.

Execute the entire new file with genuine locked rdflib 7.6.0, existing query/processor/prepared tests and relevant native/SHACL controls. Exercise configuration/term fidelity and hidden-variable hygiene through actual binding results. Run full affected Python/vendor/shadow and installed empty-graph qualification, preserving exact counts and XPASS discipline. Update relevant maintained rdflib/query documentation, translations/conformance counts and changelog to describe actual behavior, with no process references in shipping text. Use existing documentation/generated workflow. Obtain independent source-and-consumer review, resolve findings, commit with hooks/signing, push/verify and update the issue.

## Task 3: Final qualification, PR creation and honest Stage handoff

Resolve every missing/invalidated acceptance row, perform final relevant hosted qualification on attributable source/candidate, independent completion assessment and deferral scan. Verify plan/task review identities, source cleanliness apart from selected Stage, empty emergency ledger and preserved siblings. Open a scoped PR closing 454, publish this plan and substantive changes/real evidence to issue and PR, and answer least-confidence/user-should-know questions honestly. Verify PR/head/remote identities. Stage 1 ends with the PR; carry authorized Stage 2 review remediation and Stage 3 base synchronization, actual candidate evidence, notes/archive and mandatory ghprsq merge through verified cleanup, preserving all audit refs. No release/publication/runtime action beyond repository integration is implied.

## 3. paudley — 

The combined contextual-query and Python row interface is published in PR504 on08e7. Full attributable CI37830765907 passed40jobs with only the declared optional SIMD-generation skip; the owning seven-configuration generation was separately qualified before its exact generated patch was committed. The three missing test-policy fields and the generated/restated documentation mismatches were corrected through their owning checks and normal signed hooks.

Least confidence: the finite compatibility corpus cannot establish every possible future RDFLib query composition. The accepted241 native oracle fixtures, current shared-path controls, actual28 production-wheel boundary cases and full hosted matrix cover the agreed behavior. Future concrete counterexamples still require an owning native regression.

What should be known: the analytical native/host cost proof preserves work under its recorded profiles; it is not a wall-clock or shipping fat-LTO performance measurement. Duplicate positional row cells are preserved, while compatibility name lookup retains its established final-label behavior. PR-created checks/review and final selectedStage capture/archive/ghprsq gates remain pending; the pre-PR full-green run does not waive them.

