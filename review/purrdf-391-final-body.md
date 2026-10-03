<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

## Summary

For `?s (ex:p|ex:q)/ex:r ?o`, a generated junction must connect both UNION arms to the final edge while source-pattern blanks retain BGP ownership. Complete the three-design investigation and repair the carrier/runtime defects demonstrated by independent controls.

Closes #387.

Retain compatible identities, put typed observer/ownership invariants in `purrdf_sparql_algebra::scope`, and use predecessor provenance where required. One dev-only implementation supplies the experimental tests, report and benchmarks.

## Acceptance evidence

| Issue criterion | Completed evidence |
| --- | --- |
| Invariants and minimized hazard taxonomy | [Binding/scope report](docs/design/purrdf-sparql-scope.md) distinguishes user variables, pattern existentials, generated witnesses, template allocations and concrete dataset blanks; includes legal/illegal path, capture, ownership, projection, multiplicity and allocation controls. |
| Three representation/checking designs and costs | Three candidates compare correctness, compatibility, preparation/reuse and native allocation windows. Public `Variable` remains 16 bytes native / 8 bytes wasm; certificate layouts are measured separately. |
| Mutation detection, false positives and independent bags | All three accept 14 legal controls; they detect respectively 1, 10 and 19 of 19 supported hazards. Seeded structural/category properties shrink failures; native/prepared/carrier controls use independently enumerated expected bags and identity partitions. |
| Recommended bounded design and authoritative home | Production `scope` supplies deterministic typed identity/role/site diagnostics and repair guidance; the report specifies ownership, correspondence, liveness, projection and runtime obligations. |
| Boundary validation and deep-query cost | Parser, raw/compiler admission, transformation, carrier/reparse and result-egress responsibilities are explicit. Thirty prototype benchmarks use 100 samples each, including 100,000-deep algebra; memory accounting and allocator measurements are reported separately. |
| Portable cases and conformance separation | Twenty-one portable cases and their manifest/rationales are coordinated with #384: 17 SPARQL 1.1 evaluation cases, three syntax refusals and one separately labelled RDF 1.2 quoted-term case. SHACL profile acceptance/refusal and internal property-function/budget controls remain separate; official/upstream and GTS corpora are unchanged. |

The prototype checks declared positive flat-term binding incidences, owner partitions, branch multiplicity and correspondence. Its 13 unavailable inputs and six unavailable transformations receive typed refusals, excluded from detection counts. Arbitrary filter/OPTIONAL/MINUS equivalence and runtime freshness require execution evidence; the closed proof boundary is implemented completely.

## Production behavior and compatibility

- Checked carriers preserve independent raw blank owners, shared identities across required braces, caller-name hygiene, duplicate bags and visible columns. Opaque labels receive injective legal aliases through the shared lexical validator. Concrete dataset blanks, including nested ground triple terms, retain native injection support and receive actionable checked-text refusals where SPARQL has no ground-blank spelling. Zero-visible carriers use one declared unit column which the SERVICE adapter removes without dropping rows.
- BNODE, template and list-cell allocation share a fallible vacancy check and governed concrete-input reservations. Embedded composite identities enter the authoritative dictionary index. Stateful user-function children preserve caller reservations; SERVICE remaps each response's equivalence classes while separating responses from local/prior identities. Source-read errors remain distinct from unbound terms and typed budget exhaustion.
- CONSTRUCT append and UPDATE share deterministic destination namespace selection. Allocation-budget trips withhold incomplete graph/list/response output and abort staged UPDATE publication. CONSTRUCT builds once; its collision control deliberately changes emitted `c1r0` to `c2`, preserving the existing dataset `c1`. No-collision counter spelling is preserved.

Governor profile **10** records the corrected per-arena scratch ownership and immediate reservation checkpoints; unrelated aggregate buffers, custom state and child arenas can no longer hide computed growth. The existing twelve-input SUM costs 879 retained-input bytes plus its separate 74-byte result, hence 953. Consumers must remeasure scratch ceilings through `QueryGovernors::METERED` and repin profile/corpus identity. The fuel-price table is unchanged.

- Profile digest: `d1a2df1c68427c65add1e1d9279bb0d252290f921be8086384b4178531921ea8`.
- First-party governor corpus digest: `ac0b35b6444e5640dca77fc72e083733c646c6c5d567fe67d30d07ae5ff908bc`.

First-party cost/boundary evidence and evaluator traces use the documented generators; the issue addendum records the profile transition. Official conformance inputs and semantic requirements remain authoritative.

## Validation

Completed evidence before the profile-10 identity/receipt transition:

- Comparative candidate suite: 13 tests natively and 13 on wasm32; 30 prototype measurements with 100 samples each. Nine of ten prototype capture-source hashes still match; separate metadata records the documentation-only scope.rs correction. Original capture hashes and samples remain unchanged.
- Runtime allocator source in `d6f94f62a`: 1,124 core library tests plus two blank-publication controls; 1,353 evaluator library tests, 25 governed-query, 22 governed-UPDATE, 17 prepared-execution, 16 scope-interaction and 13 fallible-query controls; strict all-target core/evaluator clippy. These targeted results do not establish final full-gate status.
- Three existing 30,000-row CONSTRUCT benchmarks measured in each of two source captures, ten samples per case. Seventeen of 19 runtime capture-source hashes still match; separate metadata identifies documentation-only eval.rs/remote.rs corrections. Executable line streams and original capture hashes/samples are preserved. The blank-free control also moved; the report makes no isolated causal speedup, cross-hardware or wasm timing claim. Profile changes update identity, regression coverage and receipts; public contract corrections change Rust documentation comments only. Timings remain observations of their recorded source captures.

Final qualification on `28744b104af37faeb43a25bf865c25fa7094bc4d` passed: complete `make check`, explicit `make wasm`, full unsharded `make conformance CONFORMANCE_ARGS=`, and `bash scripts/check-generated.sh`. The conformance matrix reports 15,286 passes, 25 existing ledgered exceptions and zero failures. All 114 changed source-file hashes and qualification-log hashes match the archived source manifest. The original `92842627b` qualification is preserved separately.

The benchmark is covered by the actual shipping `scope::validate_pattern` assembly site. Stable generation and the separate final-source assembly report pass all 103 sites over seven configurations (721 cells). All 102 existing site settings/counts are unchanged. Hosted reports independently match the document under Rust 1.101 nightly (`0abfedbc7`, LLVM 23.1.1); local assembly uses Rust 1.100 nightly (`4b6d04e70`, LLVM 23.1.1).

Current-head hosted qualification is green: 49 successful checks, one expected Pages skip, zero pending/failing checks, and successful [CI](https://github.com/Blackcat-Informatics/purrdf/actions/runs/37119631972), [Docs](https://github.com/Blackcat-Informatics/purrdf/actions/runs/37119631979), and [security](https://github.com/Blackcat-Informatics/purrdf/actions/runs/37119630127) workflows. CodeRabbit completed the final incremental review with no actionable comments and zero unresolved threads. The escaped table pipe, accurate public allocation/carrier contracts and measured assembly coverage are fixed in `aa529ee`, `7f137d3` and `28744b1`. Its 69.74% aggregate docstring advisory remains documented: all 14 new callable public APIs, six types and the scope module have meaningful documentation; concrete stale contracts were corrected, 59 focused controls and warning-free Rust documentation passed, and no coverage threshold was lowered.

## Checklist

- [x] Final `cargo fmt --all` and complete `make check` qualification recorded.
- [x] No new Cargo features or external dependency versions; one first-party RDF test dependency reads portable expected results through the existing codec.
- [x] Generated artifacts are untouched; first-party governor expectations and traces use their existing generators, and official/upstream/GTS corpus bytes are preserved.
- [x] Final release-crate wasm32 build recorded with `make wasm`.
- [x] Design evidence and governor profile document user-visible behavior and scratch-budget migration.
- [x] Measurements include full samples, uncertainty, source hashes and workload/compiler/hardware limits.


<!-- This is an auto-generated comment: release notes by coderabbit.ai -->
## Summary by CodeRabbit

* **New Features**
  * Added blank-node label validation and fallible RDF list construction.
  * Improved blank-node identity handling across nested RDF terms, remote results, queries, and updates.
  * Added SPARQL scope validation and safer query serialization, with clear refusals for unsupported observations and concrete blank-node bindings.
* **Bug Fixes**
  * Prevented blank-node identifier collisions during construction and updates.
  * Improved scratch-memory accounting. When limits are exceeded, incomplete output is withheld and staged updates are not published; certified partial query results are preserved.
* **Documentation**
  * Expanded SPARQL scope and resource-governor guidance, with broader portable query coverage.
<!-- end of auto-generated comment: release notes by coderabbit.ai -->