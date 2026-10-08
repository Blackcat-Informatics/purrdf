<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Compatibility compiler feasibility review

Design assessment only, not completion acceptance. Inspected base `18489a91cb452b1773c60b469d4c30dc33a9a4a4`, supplied issue-analysis and finalized `raw/rdflib-7.6-oracle-probe.json`, shared evaluator primitives, and read-only installed oracle source named by those receipts. No source/env/build/forge mutations or execution. Only this Stage report was authored.

**Verdict: the R/C/I separation is necessary and plausible, but lowering solely to unannotated native Lateral/LeftJoin/Minus is insufficient.** A first-party compiler plus small typed operator/context metadata in the shared evaluator can make the design coherent. No second parser/evaluator is needed. Concrete gaps follow.

## 1. Context and output really are different

The executed receipt's bind_then_read case returns `this=b` with a's o1/o2: Extend changes R while subsequent BGP reads C, not R. group_override and star_override expose b; nested_override restores a; optional_override exposes a. A single name-to-current-variable map fails these witnesses. Preserve distinct SSA aliases for R and C plus immutable I, and retain logical variable/domain identity separately from hidden transport columns.

The proposed thaw rule is supported by the installed source: QueryContext.clone uses `bindings or self.bindings`, then the constructor overlays initBindings. Thus a **logically empty mapping** reuses old C; a nonempty mapping replaces C, then I wins. Native schema width is not this truth test: a row containing only None or hidden aliases is still logically empty. Add an explicit logical-domain view/predicate; do not count carried I/C aliases as output mapping entries. Values of unbound columns are absent mapping entries, not a present key with a null RDF term.

`forget` is also runtime-sensitive: it retains an R entry if the name is in the source scope, in I, or absent from the pre-evaluation C. Static alias membership alone cannot decide the last case when OPTIONAL or VALUES left a cell unbound. The source scope set must precede SSA rewriting; recomputing `_vars` from private aliases changes the law. Expression reads then use that filtered R and I fallback, not arbitrary C fallback.

## 2. Native Lateral is a useful engine seam, not the required merge law

`binop.rs::eval_correlated` (around 272) converts every bound schema column into substitution input. `eval_substituted` handles tracked copies, deferred EXISTS/LATERAL windows and the normal evaluator. Native eval_lateral then tests ordinary compatibility before merging (around 574–605). rdflib lazy join instead evaluates the right under thaw(left) and returns right merged with **left precedence**, even when the right assigned a different logical value.

Disjoint SSA output aliases can avoid native compatibility rejection, followed by explicit coalesced left-precedence normalization. However the correlation row must contain exactly the compiler's C inputs, not every R/I/old alias in the physical schema. A typed correlation-input mask/mapping is the minimally coherent repair. Preserve right/output aliases separately, and project internal aliases before comparing logical mappings. Merely changing ordinary Lateral's compatibility globally would violate its existing SEP theorem.

Ordinary Join is independently evaluated hash compatibility, with positive-plan/seeded optimizations and possible SERVICE-driven operand order (`binop.rs:69–177`). The compatibility compiler must preserve the oracle's lazy/nonlazy classification; do not let ordinary commutative rewrites choose execution shape. For nonlazy Join the oracle deduplicates RHS mappings. Insert DISTINCT on **logical R only**, before normal compatibility; hidden context/history columns must not prevent deduplication. Empty/unbound mapping domains remain significant.

## 3. OPTIONAL needs a shared per-driver operator extension

Native eval_left_join (around 1308) evaluates a shared RHS or a positive seeded plan, then performs compatible pairing/filtering. It has no rdflib remembered-scope retry. Replacing it by ordinary LeftJoin or by `LeftJoin(left, Lateral(...))` loses driver occurrence identity and can multiply duplicates.

Add a typed compatibility application policy to the existing per-driver join machinery: evaluate RHS under thaw(a), evaluate its condition using the specified forget/lookup view, emit left-precedence merged matches; if no match, retry under thaw(a restricted to the remembered source-left scope) to decide whether a is emitted alone. The retry is an existence test, not extra emitted rows. Preserve the current incomplete-child rule: a truncated RHS cannot prove no match and cannot license padding. Reuse the correlated evaluation/window guard, VM, scratch, governors and actual row driver; this is an operator policy, not a second evaluator over the whole tree.

## 4. MINUS must see logical domains and independently evaluated RHS

Native eval_minus (around 1945–2035) computes shared columns from entire VarSchema and removes a row only for compatibility plus a bound shared-domain intersection. Carrying shared I/C aliases makes disjoint logical mappings appear overlapping; giving every logical variable distinct per-arm SSA aliases removes real overlap. Both errors are possible.

Normalize arm R aliases to common logical comparison slots and project transport-only columns before MINUS, or supply explicit typed logical-column pairs/domain masks to its existing kernel. Restore left context columns afterwards without rejoining a bag in a way that squares duplicates. Its RHS is independent, not thaw(left), and oracle RHS uses set semantics; hidden history must not affect that set. Preserve native MINUS default behavior and its truncation/error rules.

## 5. Native EXISTS substitution has a different scope contract

Native EXISTS preparation can normalize/probe/memoize a pattern. Its correlated fallback uses the same substitution machinery as Lateral (`expr.rs:1559` onward). `substitute_pattern_impl` narrows the substitution row at each Project to projected variables (`expr.rs:3309`); nested deferral environments carry substitutions by native analyzed variable sets. That narrowing is an explicit native theorem, not automatically rdflib context propagation. A compiler-generated Project used only to hide aliases could accidentally stop needed C inputs.

A compatibility EXISTS site must specify its correlation C map and logical scope explicitly. Ensure synthesized projections retain required hidden transport until evaluation is complete, or teach the substitution home about the typed compatibility mapping while preserving ordinary Project behavior. Prevent native ENF/positive-probe eligibility from silently assuming ordinary substitution law for compatibility assignment/context nodes. Memo keys and deferred environments must include every effective C/I dependency, or use the existing nonmemoized definition route for sites that cannot prove native eligibility. Never cache an answer keyed only by public R when different context yields different results. Preserve operational errors and the expression truncation barrier.

## 6. Extend, grouping and presentation

Native eval_extend sets the target cell to the VM result, including None on ordinary expression failure (`expr.rs:212–304`). A compatibility assignment can lower to a fresh temporary and then `COALESCE(temp, old-R-or-I)` to retain the old value on error. Do not catch EvalError: dataset/invariant/governor errors remain hard errors. Fresh identities must cover duplicate targets and input spellings, including repeated compilation/per-run substitution.

Grouping destroys nonkey/nonaggregate columns; compiled R and C lifetimes must be specified at Group, Aggregate and Project, not restored indiscriminately from input rows. The oracle AggregateJoin yields FrozenBindings with its context even when grouped results are empty; inspect and differential-test explicit-key empty input and implicit aggregate input. DISTINCT, RHS set construction, MINUS comparison and final SELECT must operate on logical mapping identity, not hidden aliases. SELECT * is derived from original visible scope; CONSTRUCT/ASK/DESCRIBE use the appropriate final R/I view. Final renaming is algebra/output metadata, not another Python column fold.

## Minimal coherent implementation boundary

Keep one parser with a named internal compatibility admission mode limited to the actual shim route; it must admit rdflib's assignment collisions before strict checks discard them, while retaining unrelated syntax checks. Keep one immutable compiled plan and the shared eval dispatcher/VM/storage carrier. The Rust compiler emits ordinary algebra wherever its law matches and typed compatibility metadata at context-sensitive application/EXISTS/domain boundaries. Put per-driver context/match mechanics in the existing binop/correlated homes and mapping views in the shared row/schema home. Do not build a recursive compatibility evaluator beside eval_evaluated.

Metadata must survive plan admission, copying/substitution, source ordinals, dependency analysis, schema derivation, caches and safe parallel execution. Identity/cache keys distinguish the compatibility law and logical scopes. Operator source attribution and budget/cell admissions cannot disappear into untracked synthetic wrappers. Reuse existing guards and hard-error plumbing rather than broad fallback. Establish ordinary native/prepared/SHACL regression controls before wiring the production shim.

The nine supplied oracle witnesses constrain the design but do not qualify all operators/combinations. Before implementation acceptance, require witnesses for nonlazy RHS duplicate elimination, empty-map thaw, runtime forget with unbound cells, OPTIONAL retry where first context matches differently, nested EXISTS crossing a projected/unprojected subquery, explicit-key empty grouping, alias-collision inputs and correlated duplicate driver multiplicity. No tests were run by this reviewer and no completion PASS is asserted.
