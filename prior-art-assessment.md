<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent prior-art assessment

Target: issue 454; branch `paudley/454-rdflib-shim-algebra-level-reassignment`; selected base `18489a91cb452b1773c60b469d4c30dc33a9a4a4`. Source inspection was read-only. This is the sole authored Stage artifact. No forge reads/mutations, fetch, compilation, Python execution, tests, index/history changes, lifecycle actions or model-backed work were performed. Historical commit messages and assertions are evidence of past implementations and expectations, not fresh test results.

## Governing evidence and coverage

Read root `AGENTS.md`, `.baseline`, `.goals`, the supplied issue/brief/prior-art artifacts, selected-base shipped source and tests, `docs/book/src/sparql/querying.md`, Python fixtures/version lock and vendored provenance. A repository filename search found no ADR documents; the brief also names none. Concrete relevant contracts are the repository guide, parser scope checks, native prebinding, SHACL assignment detector and documented behavior.

The issue has zero comments. Broad related conformance/SPARQL search hits do not establish previous reassignment implementation. The `none: 9` defect-class trailer tally is taxonomy, not nine reassignment failures; no numeric recurrence claim is warranted.

At selected base there is no `UnmodelledReassignment` in the inspected shim, Python native binding, SPARQL algebra or evaluator paths. `_rebinding.py` and its differential suite were deleted. `Graph.query` directly passes query text and converted `initBindings` to native `Store.query` (`bindings/python/python/src/purrdf/compat/rdflib/graph.py:1104–1175`). The remaining textual `_inline_bound_variables` is an UPDATE mechanism, not the reassignment query path. Exception removal is already satisfied as source structure; it proves no answer parity.

## Concrete prior attempts

Each of the following seven OIDs passed `git merge-base --is-ancestor <OID> 18489a91...` with exit 0. They are already ancestors of the selected base.

| Revision | Recorded implementation/failure | Reuse boundary |
| --- | --- | --- |
| `e307062e4be275f4755eb14cc589da176808644c` | Added shim `_rebinding.py`: fresh assignment variables, expression reads through COALESCE, final result-column folding. Then-new differentials were claimed to match rdflib 7.6; native refusals remained. | Query inventory and shim/native separation are useful. Later counterexamples disproved the general architecture. |
| `859b174c5e512a11973c8fb64dd225b69e556fc8` | Commit/tests record wrong answers under OPTIONAL, MINUS, EXISTS, GROUP BY, SELECT *, nested groups, and a broken bracketed VALUES column list. Added UnmodelledReassignment refusals and no-assignment neighbors; fixed token-only prefix guard. | Concrete failure families to restore as answering differentials, not refusal tests. |
| `ca1be82262b2a99c19cb69c4241cfa101144f136` | More failures: assignment in sub-SELECT WHERE with/without outer filter; CONSTRUCT template reading assigned value; duplicate assignment targets. Module additionally refuses DESCRIBE. Tests record rdflib answering while shim refuses. | Cover subquery projection, repeated targets and graph-producing forms. A final row-column fold cannot repair these. |
| `27c5ef60204aaa1d72f1b005d355d690d4c952ed` | Deleted rewrite, refusal class, differential file and prose; restored direct native substitutions. Commit explicitly says full parity was separate work; historical Python count was 3917 passes. | Deletion is already present. Old counts do not establish current parity or preservation of formerly working reassignment answers. |
| `e5347587147bbc3fad5952abd7e64ecace0388e0` | Removed blanket native assignment refusal while preserving parser scope errors; localized assigned unprojected subquery names; changed native/prepared/SHACL expectations. | Purposeful native contract. Do not replace globally with rdflib override semantics. |
| `ec9d364d34c21051b81a09e04f4a79c1dfcb5c29` | Fixed assignment behavior depending on unrelated siblings: fresh targets filtered by `!BOUND(?fresh) || sameTerm(?fresh, ?bound)`, with re-extension. Commit records four rows alone versus zero beside a sibling before repair. | Reuse traversal/fresh-name infrastructure where valid; its native join filter is not full rdflib reassignment parity. |
| `05562b61c099a7338fb6006e19603098db910df8` | Fixed restoring names for MINUS, localization inside EXISTS, bound column above lone subquery, and seeding both MINUS sides. Counters prevent collisions between passes; EXISTS respects SEP-0007. | Preserve controls and repeated prepare/run behavior, and check compatibility transform together with these mechanisms. |

Historical differential: `859b174c5:bindings/python/tests/test_compat_prebound_reassignment.py`, extended at `ca1be8226`. It compares names, row multiplicity and explicit ordering, but normalizes cells with str, losing RDF term-kind distinctions. Reuse query concepts with a term-aware comparator and bag/order checks; discard obsolete native-refusal assertions.

## Current homes and semantic boundaries

Production path: shim Graph.query → `bindings/python/src/py_store/quad_store.rs::query` → NativeSparqlEngine/SparqlRequest. The binding collects substitutions/options before releasing the GIL. `py_store/query.rs` owns engine configuration/result adapters rather than the store method.

Native prebinding lives in `crates/sparql-eval/src/substitute.rs`: apply_shacl_prebinding/apply_shacl_probes/walk_shacl_probes, localize_unprojected_assignments, join_assignments_with_prebinding, expression walks, seed/MINUS helpers. Preparation in engine.rs applies the names-aware assignment transform, and per-run doors use the shared transform. `crates/sparql-algebra/src/substitute.rs` provides core algebra mutation/injection and Query::assigned_prebound; its detector remains needed for SHACL loader refusals. Reuse existing tree/walk and conversion homes rather than duplicating them.

Native/compat mismatch is decisive:

- Native join_assignments_with_prebinding rejects conflicting assigned values by sameTerm. `prebound_grouping.rs:337–411` pins these answers and an in-scope BIND parser refusal.
- Historical compat tests include overriding caller values and repeated/in-scope assignments rdflib accepts. ca1be8226 documents projection and graph-construction failures.
- parser/machine.rs checks in-scope BIND, UNFOLD, GROUP BY and SELECT-expression targets; parser.rs contains explicit BIND/EXISTS/LATERAL scope regressions. Removing those checks globally changes standard native/conformance behavior.
- SHACL rejects assignments to potentially prebound names with lane-specific VALUES/MINUS rules. Compat must preserve those admission rules.

Existing native parameter substitution alone cannot fulfill the issue. The implementation needs a deliberate Rust compatibility boundary and oracle-proven mapping/merge rules. An internal typed route is preferable to semantic Cargo features or changed native defaults; exact placement/representation remains a decision to prove. Keep Python thin. Do not add a textual second parser, Python evaluator, upstream implementation copy, runtime rdflib dependency or silent fallback.

The book claim that the shim refuses every reassignment (`querying.md:191–193`) is too broad for the later inspected native assignment-admission code. Reconcile it with the actual final public behavior instead of treating prose as implementation truth.

## Sibling work must remain untouched

Read-only sibling `/home/paudley/Active/purrdf/.worktrees/prebound-values-nested` has HEAD `82085354e` and seven pre-existing modified tracked files: CHANGELOG; evaluator engine/execution/substitute; prebound_grouping.rs; querying book source; zh-Hans catalogue. No changes were made there.

Three committed sibling changes are NOT ancestors of selected base (each ancestor check exited 1):

- `fa5d4398babbf1f25766712dd97558c7981b1d12`: nested VALUES columns moved to fresh targets and filtered locally before OPTIONAL/MINUS/EXISTS/subquery composition; native sibling controls.
- `e3cc734732b8f58d20119490b6f2ab5e72dbadf7`: sub-SELECT seeding before DISTINCT/GROUP to remove dependence on outer siblings; changed property-function grouped-UNION expectation/manual oracle.
- `073c57e14bd457eef3a7fe415367e302ac723b1f`: documents native VALUES rules at every depth.

The dirty patch further removes unprojected-assignment localization and asserts globally constrained assignments inside those subqueries; its rustdoc explicitly distinguishes some results from rdflib. This is independently active native-contract work, not qualified compatibility evidence. Do not stage/discard/overwrite/cherry-pick it opportunistically. Coordinate if overlapping integration is necessary; any reuse requires source-bound review and exact-case differentials.

## Acceptance, licensing and tests

The existing real oracle is rdflib 7.6.0 (`bindings/python/uv.lock:1316–1318`). conftest.py separates compat and real oracle imports and excludes vendored/shadow tests from the parent process. test_rdflib_version.py binds the reported version to the lock. Preserve identities; a shadow subprocess is not the real oracle.

`make pytest` builds the native extension using maturin and runs locked uv/pytest; it is separate from make check. Relevant regressions: test_compat_parity, native/Python prepared/prebound suites, SHACL prebinding, vendor/shadow gate, W3C parser/evaluator suites. No xfail additions or lowered counts can prove acceptance. Update counts/projections only from actual final runs.

Existing Python test paths cannot grow under the merge-base non-Rust ratchet. A new narrow Python differential file can be justified by the real Python oracle and must carry a concrete Why-not-Rust explanation in its first 40 lines. Keep rewriting, generators and core tests in Rust. No runtime dependencies or semantic Cargo features are authorized; affected published crates remain wasm-buildable.

Local vendored provenance names BSD-3-Clause rdflib 7.6.0 sdist SHA-256 `6c831288d5e4a5a7ece85d0ccde9877d512a3d0f02d7c06455d00d6d0ea379df`. Existing test vendoring is not authorization to copy upstream parser/evaluator bodies. Use package behavior as oracle and build a first-party transform with SPDX/provenance discipline.

Novel decisions/evidence required:

1. Determine rdflib mapping precedence at Extend, Join, LeftJoin, Minus, EXISTS, projection and aggregation from actual observations, including errors/unbound cells and repeated targets; historical messages are hypotheses, not the complete law.
2. Preserve compatibility assignment identities/visibility through algebra: SELECT *, private/projected subqueries, grouping/aggregates, SELECT expressions, graph-producing forms and collisions. Final column folding alone is insufficient.
3. Wire the actual shim to a compatibility algebra route while preserving Store/prepared/SHACL contracts and unrelated syntax failures.
4. Cover requested constructs/combinations plus historical subquery/filter, duplicate-BIND, CONSTRUCT, UNION, bracketed VALUES, ordering, token-prefix, no-assignment and fresh-variable controls. Compare term identity, visible variables, bags and explicit order.
5. Qualify final public differentials, native/conformance and portability gates against final source. No tests ran in this assessment; answer parity remains NOT MET until implementation and evidence exist.

The prior text rewrite failed concretely and was removed. A substantial native algebra transform already exists, with deliberately different semantics. This task requires a scoped first-party Rust compatibility transform, not symbol deletion; preserve active native sibling work.

Artifact-writing note: an initial shell heredoc attempt was rejected before execution by the shell policy parser. This document was then written through the supported patch tool. That rejected attempt changed no source or artifact.
