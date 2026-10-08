<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent preliminary core-law and provenance review

Verdict: **NOT qualified for Task 1 completion**. This assigned preliminary source review is complete. It found concrete mapping-law omissions beyond the passing development matrix. It is neither a fourth planning audit nor a final-source test PASS. Source is changing under the sole implementation writer; findings describe the inspected snapshot and need repaired-source execution and independent final binding.

## Identity and method

Worktree `/home/paudley/Active/purrdf/.worktrees/454-rdflib-shim-algebra-level-reassignment`, branch `paudley/454-rdflib-shim-algebra-level-reassignment`, inspected HEAD/base `18489a91cb452b1773c60b469d4c30dc33a9a4a4`. Read root and worktree governing instructions, root `.baseline`/`.goals`, the empty emergency ledger, current plan and validation index, issue analysis and compiler-design/feasibility reports. No deeper AGENTS files were found under the affected crates. The authoritative plan hash was `8627bc25be581a26269cf8ad5cbe9f826d2917f6168d8cb5e35c71e132ca01f6`.

Read the complete new compiler, Rust contextual test, the tracked delta across parser/algebra/evaluator homes, and relevant unchanged native aggregate and oracle code. Oracle source was inspected read-only in the existing genuine environment. No candidate build/test, oracle execution, dependency/environment mutation, shipping-source edit, commit, publication or timing measurement was performed by this reviewer. Only this report was written. No temporary checkout, process or diagnostic debris was created.

At the identity capture during review, SHA-256 of `git diff --binary` was `e5a8d16be7e540ede7002abfe4e0481c3ed5968c42e64d3cfa3e250b3f1e164c`. Untracked-file hashes were:

| File | SHA-256 |
|---|---|
| `crates/sparql-eval/src/rdflib.rs` | `b3f2991a958118fda0a66b37a660f9b179df0c5254e3f669ef35726d54537032` |
| `crates/sparql-eval/tests/rdflib_contextual.rs` | `f158c7d45127d01c2a3037f0c1f19e6883ed211fc709b54a3e986d6a2419f90f` |
| `crates/sparql-eval/tests/fixtures/contextual-mappings.json` | `73d1c38fbf9cdf4bb7df6976cf6dea004cd2dded4a351bb93b37ecd90f1ec2ae` |

These are read-window receipts, not a frozen atomic final source tree. The fixture already incorporated the additional eight grouped HAVING/order/slice inputs by this capture, while the test's asserted total was still 166 during its earlier read. Final cardinality and source identity must agree after integration.

## Required findings

### 1. COUNT(DISTINCT *) observes obsolete assignment carriers

The Group compiler passes `compiled.pattern` directly into native Group after adding key carriers. Extend retains its old carriers and temporary result as physical schema columns. Native `modifier.rs` COUNT(DISTINCT *) compares the full solution row, excluding only shared-blank hidden columns. SSA assignment history therefore changes distinct logical mapping identity.

Witness: `SELECT (COUNT(DISTINCT *) AS ?n) WHERE { VALUES ?x { 1 2 } BIND(0 AS ?x) }`. The two returned logical mappings are both `{x:0}`, but their old x slots differ. The genuine receipt `raw/count-distinct-logical-domain-oracle.json` proves count 1 with no I and with an unrelated I binding. SHA-256 `88ae4154837674ecb3af3d241d8e3219c2a210b60cd98feb2c79756530758b44`. Source predicts count 2 on this snapshot; candidate execution was not performed by this reviewer. Restore logical-domain identity at the aggregate input boundary, without changing ordinary native COUNT semantics.

### 2. Bound GRAPH fabricates an actual graph-name mapping entry

The variable Graph compiler unconditionally extends a target from the returned graph name or selected graph identifier. Genuine evalGraph differs by branch: an initially unbound graph name is compatibility-joined onto returned mappings, whereas a bound graph simply yields body mappings. A sub-SELECT can remove the actual graph-name key even though I still provides expression/result lookup.

Witness on the named g1/g2 fixture: `SELECT ?g WHERE { GRAPH ?g { SELECT ?missing WHERE {} } }`. With I[g]=g1, the genuine public bag is empty; with no I it contains g1 and g2. `raw/graph-projection-relational-facts-oracle.json` proves both. SHA-256 `f10c6b02aff128fb2913742d2d57840d0b6cf96be5c5cef90ec3ce1b63e32a7f`. The snapshot synthesizes g even in the bound branch, defeating the empty-actual-map suppression rule. Preserve the branch-specific actual domain, rather than merely the visible value.

### 3. Relational-expression facts retain names the oracle excludes

The Facts fold unions operand-variable names for every scalar expression. Genuine `_addVars` sets RelationalExpression's `_vars` to empty. This matters to FILTER's `forget(C, except=_vars)`: an expression-only incoming context variable must be forgotten unless I or another syntactic source scope retains it.

Witness: `SELECT ?x ?z WHERE { VALUES ?x { 1 } { BIND(0 AS ?z) FILTER(?x = 1) } }`, no I or unrelated I[this]=a. The same genuine receipt above proves zero rows for both. In the inspected compiler, x from the comparison enters Filter facts and preserves the contextual x, incorrectly satisfying the filter. Include all relevant relational forms and nested Boolean compositions when repairing facts; do not replace syntactic summaries with native free-variable summaries.

### 4. OPTIONAL retry evaluates the whole RHS instead of stopping at the first satisfying row

`evaluate_application_row` calls `run(retry_inputs)` to completion, then `application_condition` filters every retry row, then asks whether any remain. Genuine evalLeftJoin uses `any(condition for b in retry)` and stops when a witness is found. The stored-once representation and successful-first-attempt early return are correct useful properties, but do not establish retry invocation/error behavior.

Witness: `SELECT ?x WHERE { VALUES ?x { 1 } { OPTIONAL { VALUES ?y { 1 2 3 } BIND(<http://example.org/counter>(1) AS ?tick) FILTER(BOUND(?x)) } } }`, with the successful one-argument volatile counter and no I. The first attempt's condition forgets incoming x; an empty remembered scope thaws old C; retry's direct condition accepts its first row. `raw/retry-exists-clause-facts-oracle.json` proves exactly four callback invocations and zero public rows; SHA-256 `661702815105a9d7f078b7dcb290d0c28994b98a0a1de88e0b66d1f18fe15c2f`. Source predicts three first calls plus three retry calls. Continuing after a successful retry can also encounter a later operational error that the genuine path does not reach. Repair through the shared execution home and prove both invocation and error behavior. This reviewer read the genuine receipt but did not execute the candidate.

### 5. EXISTS root filters need the nonisolated visibility law

Genuine `translateExists` marks the translated graph's root Filter `no_isolated_scope=true`; evalFilter then uses its row directly instead of forget. The compiler has no such fact and compiles every Filter with forgotten visibility. The overinclusive relational facts in finding 3 can accidentally conceal this omission. Correcting them without this rule will turn a valid outer-variable EXISTS filter into a false result.

Witnesses: `SELECT ?x WHERE { VALUES ?x { 1 } FILTER EXISTS { FILTER(?x = 1) } }` and its NOT EXISTS twin, with the normal nested-group filter control from finding 3. The same genuine receipt proves one x=1 row for EXISTS, zero for NOT EXISTS and zero for the normal plain-group filter control. Preserve only the executable root Filter's specific law, rather than disabling isolation for every filter at arbitrary EXISTS depth.

### 6. Group-only HAVING sampling is omitted; initial prediction retracted

The initial prediction for `SELECT (<http://example.org/b> AS ?this) (COUNT(*) AS ?n) WHERE { VALUES ?this { <http://example.org/a> } } HAVING(?this = <http://example.org/a>)` was contradicted by genuine execution. The receipt above proves one n=1/this=b row. Inspection of the actual `traverse` computation explains it: `complete=True` returns True after a complete aggregate-free HAVING traversal, so `translateAggregates` does sample HAVING even when that clause contains no aggregate. The initial clause-local HAVING claim is retracted; this query is a control, not a defect finding.

A narrower confirmed missing case remains: the compiler's `had_aggregates` guard skips HAVING sampling for GROUP BY with no original aggregate. Corrected witness: `SELECT (<http://example.org/b> AS ?this) WHERE { VALUES ?this { <http://example.org/a> } } GROUP BY ?this HAVING(?this = <http://example.org/a>)`. `raw/group-only-having-oracle.json` proves this=b both without I and with I[this]=a; SHA-256 `14b33e40ccd7af69698ea3723646800782738bdf18196e7922804f8a40f4f912`. The snapshot lacks the necessary sampled post-group this. ORDER BY's `complete=False` differs from HAVING and also requires exact clause-specific coverage rather than assuming the two conditions share a law.

Additional volatile ordering qualification was communicated: first application eagerly computes the entire RHS before evaluating any optional condition, while the genuine source interleaves each yielded RHS row and its condition. A one-driver RHS with VALUES y={1,2,3}, BIND(counter(1) AS tick), and FILTER(counter(1)>tick) can distinguish tick values 1/3/5 from 1/2/3 even with equal invocation counts. This is a source-supported additional witness, not an executed oracle/candidate result in this report. The existing successful-volatile tests use one RHS row per duplicate driver and do not prove this interleaving law.

## Other visitor and qualification gaps

`prebind_memo::for_each_child_slot` classifies Apply with binary pattern-only nodes and omits its OptionalApplication condition. Its reference walker repeats that omission, so equivalence tests using this reference cannot discover it. `substitute::own_expressions` and node-expression readers likewise omit the condition, and the ordinary SHACL substitution explicitly stops at Apply. The contextual entry deliberately executes without ordinary substitutions, so this is not proof of an already executed compatibility wrong answer. Nevertheless a new public GraphPattern primitive requires a demonstrated typed admission prohibition or complete supported transformation contract for callers constructing Apply via prepare_algebra. Document and execute that contract; passing a mirrored reference alone is insufficient.

The new test comparator retains bags and ordered sequences and compares complete named columns, unlike the old stringified set comparator. Duplicate-column lookup uses the first matching name for every occurrence; current duplicate same-name outputs intentionally share the same value, so this is consistent with their law, but does not independently prove distinct-slot publication or governor charging. The separate governed duplicate-columns control addresses part of this. RDF 1.2 initial-term controls include directional literals, quoted triples and blank terms; the oracle matrix cell function itself handles only IRI/literal cells and ignores direction. Final cross-language term fidelity still belongs to actual surface qualification.

Incomplete-child retry/padding handling is conservative in source: only Complete first/retry results can license an unmatched row, and shared per-driver execution discards an incomplete driver block. The optional condition checks fuel and expression barriers and propagates operational errors. Final resource/governor tests must still bind repaired source, especially any new retry short-circuit support.

The explicit context map, fresh immutable assignment outputs, complete query/binding variable census and prefix exclusion provide a coherent capture-free structure. Leaf/context merges, per-bound-cell empty-map thaw, initial overlay, canonical logical-only projections for MINUS/nonlazy RHS DISTINCT, aligned UNION bags, public empty-map filtering and compatibility-only duplicate-column metadata are all present. These observations are structural support, not proof that every law composes: the findings above show why 166 passing cases are insufficient.

## Provenance and native-path assessment

No imported implementation body, upstream coefficient table, added runtime dependency, alternate tokenizer or separate evaluator was found in the reviewed delta. The compiler is first-party Rust building native algebra; Apply uses the shared binop/substitution/VM/governor homes, and the compatibility entry reuses prepare_algebra and prepared execution. The relocated mutable-expression walker is an existing first-party body moved to the common algebra home with its former body removed, not a new upstream copy. Genuine upstream code supplies semantic laws and oracle answers. A source review cannot establish an absolute global plagiarism claim, but this delta supports the original-implementation provenance judgment; final source must still be reviewed after repairs.

Ordinary parser admission uses compile-time false specialization, ordinary PreparedQuery layout is unchanged by the wrapper, and compatibility state is held only by the explicit route. No direct unconditional new ordinary native collection pass was identified in this law review. Shared enum dispatch, wrappers, clone callbacks and correlated specialization codegen require the separate native-cost proof. Allocation/layout equality and source plausibility do not establish unchanged primary Rust execution work. No timing benchmark was used or authorized.

Final Task 1 acceptance remains missing: repaired executed witnesses for every finding; authoritative final-source matrix count; strict native/prepared/SHACL/W3C controls, release warning-clean checks, wasm and resource coverage; complete compiled logical native-cost evidence; and final independent source/evidence review. Actual Python production wiring, full oracle/vendor/shadow qualification and integration/merge are later required tasks. No requirement is waived and no external blocker is established.
