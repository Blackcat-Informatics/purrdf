<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Authorized recovery coverage and residual ownership

This map follows the approved recovery order recorded in `plan.md`. It records which existing source assets to retain, which requirements share an implementation, and which remain distinct. It does not authorize extra work, close an issue or qualify an implementation. The active governor unit remains governed by its own plan, issue analysis and validation records. All later units are owned by the root recovery lane; the preparation agent makes no source changes.

The source baseline inspected here is main `dfc0c21adabe557e5d11f027aaf2576bddbe1dd6`. Retained XPath is `9e9e67c80d832175bdafb8e938d7abae00486f8b`; retained SHACL is `4faeb36ce487eacbe48db2fc3e8bbcbd0c5a1df6`; the abandoned portable corpus head is `ab8870616cf7bb0d79daf3e012b97f5efeafbc61`. No new tests, builds or source mutations were performed for this map. Evidence that a test is present is not a claim that it passed today.

Cached `raw/recovery-issue-N.json` records were read for 470, 451, 453, 452, 471, 476, 468, 474, 364 and 475. These are body-only captures with title/state/URL and no comments. Older follow-on discussion and source findings were inspected during the preceding read-only audit; the parent supplied the previously audited advanced arithmetic and spatial requirements. Refresh each complete discussion through stagectl when its unit becomes active, not by repeating a broad forge inventory now.

## Queue refresh: 2026-10-07 10:18 UTC

Stagectl's open-issue search returns the same 30 issue numbers recorded below, with no additions or closures. The open PR list still contains only 448, 462 and 463. Stagectl PR evidence confirms 462 remains open; forge metadata and local git objects establish the head distinction below. This refresh checks queue membership, PR metadata and donor refs, not every renewed issue/review discussion or current-source qualification.

| Open PR | Current remote head | Current merge metadata |
| --- | --- | --- |
| 448, native XPath | `9e9e67c80d832175bdafb8e938d7abae00486f8b` | MERGEABLE / CLEAN |
| 462, dated SHACL | `a0f9a070b0c772435a336a3c239e605ca327ff28` | MERGEABLE / CLEAN |
| 463, geodesy/grid | `a7bf02d6e139017302b961e75f12b8a99feef18a` | CONFLICTING / DIRTY |

Remote main is still `dfc0c21adabe557e5d11f027aaf2576bddbe1dd6`. Retained XPath/count, SHACL, both geodesy refs, exact-only numeric, remembered-default, Turtle, Python-row and portable-corpus donor refs are unchanged from this preparation. The RDFLib donor has advanced from `56e636497c049debe2de5445389abf3e018d4783` to `cee1c41b2faa370b7daa60cc7536e784a4756d0e` ("Integrate division governance with contextual queries"), a merge of that prior donor with current main. Its current logical delta must be considered when the contextual unit activates; no fresh qualification was performed here.

The retained local SHACL fix `4faeb36ce487eacbe48db2fc3e8bbcbd0c5a1df6` is a direct child of remote PR head `a0f9a070b`. Its own delta changes CHANGELOG and the shapes admission/SPARQL files (77 additions, three deletions) to pre-bind no shape context under the dated draft. This retained fix is absent from the current remote PR head. Remote merge metadata therefore does not qualify or integrate the retained fix.

## Current count-based qualification

The current approved measurement contract in `plan.md` qualifies recovery performance with retired user-mode instructions and allocation counts. Measure the production call through its returned materialized result and receipt; prepare fixtures/requests, initialize workers, warm up and validate full typed answers outside that region. Instruction coverage includes the caller and participating workers, verified by worker-coverage and empty-region controls. Separate allocation passes use the shared counting allocator/window. Keep compiler/profile, fixed work, fixtures, worker counts and result-lifetime boundaries matched across baseline, control and candidate.

Report ten raw samples with the existing median/MAD and seeded 95-percent bootstrap intervals, retaining all samples and the existing one-percent comparison threshold. The numeric criterion is metered/plain median retired instructions <=1.20 at both four and 32 workers with all 5,000 typed SUM values checked. Row-loop stop and fuel-plus-stop lanes must reduce instructions against unchanged main at 32 workers and meet the normal-chunk control; plain controls retain their comparison acceptance, with one/four-worker neighbors reported. Allocation calls, requested/retained bytes and peak bytes have their own comparisons and bounded-work acceptance. Historical elapsed-time receipts remain historical records. These count criteria govern current recovery qualification; older preparation prose does not restore elapsed-time acceptance.

## Preservation and qualification rules

- Separate staged, unstaged, untracked and merge-index state. The root's `recovery-snapshot-index.md` now locates recovery backups at `/home/paudley/Active/purrdf-recovery-artifacts/20261006-2253/worktree-snapshots`, outside selected merge evidence. A repository bundle preserves existing refs. These backups are not acceptance receipts.
- Extract a commit's own logical delta or selected file hunks onto the current foundation. Do not restore an old tree wholesale. Evaluate both sides of shared-file conflicts and retain both required behaviors.
- Existing binary/checkpoint, allocation, timing, browser and external-engine observations remain bound to their captured source/artifact identity. Fresh acceptance must bind the final unit source to the executable, parameters, corpus and receipts used.
- Historical Jena results are data only. Never execute its bundled JDK, regenerate through Java, or describe old Java observations as new Rust-only independent comparison evidence.
- Keep agent reviews labelled as agent reviews. The corpus's existing `reviews/index.json` and `reviews/service.md` use independent-review wording that must be corrected without falsifying the historical capture.
- No rebase, force push, hook bypass, semantic Cargo feature, invented second helper home, or model/service lifecycle action. Every final integration uses ghprsq. Frozen GTS/vendor vectors remain frozen.

## Active foundation

The governed row-loop regression and metered-overhead requirements share the existing `parallel.rs`, `row_checkpoint.rs`, `governor/mod.rs`, scratch and testkit measurement homes. Issues 478 and 469 belong to this one unit. Retain deterministic source-order trip selection, consumption, cancellation and the ceiling-plus-in-flight-work bound while recovering useful scheduling granularity. The OPTIONAL CLI path must be verified against its actual meter configuration. Acceptance is the plan's instruction/allocation comparisons, numeric grouped median instruction overhead <=1.20 at four/32 workers, unchanged vectors and real host witnesses. A scheduling fix alone does not satisfy the overhead ceiling.

The current count qualification extracts the native controller from the existing hash workload into the shared testkit counter home and adds explicit-unit records using the existing statistics and atomic store. The governor unit owns that shared infrastructure and its two production count drivers. Later XPath, numeric and residual selected-measurement work must reuse it rather than introduce another counter controller, statistics implementation or record codec. This extraction is under qualification; it does not complete the residual comparison/replay requirements.

## Remaining authorized order

The rows below are semantic extraction boundaries, not permission to reproduce all historical PR contents. Some issues span several rows and remain open until every required row is complete.

| Unit in recorded order | Retain / extract | Dependencies and complete acceptance |
| --- | --- | --- |
| Native XPath and large counted forms | Retained PR 448 source; own-commit `36a13ac7..efd2829a6` candidate material. See `xpath-preparation.md`. | Current governor first. Finish count-independent production-default class/group forms, exact captures/REPLACE, randomized comparisons, typed bounds, actual CLI/Python/C/Wasm execution and corrected cost/nullable-count documentation. Keep division-policy wiring. Both 406 and 467 remain incomplete until this combined unit is qualified. |
| Shared conformance kit and vocabulary | Prefer later retained SHACL branch's kit. Prior blob comparison found portable branch's staged production kit identical to retained SHACL; its grading test differs. | Extract one kit rather than both versions; use existing lexical/hash/testkit/vocabulary homes. Qualify exact multiset/literal/blank-node/report grading, typed fail channels and fixture identity. Move only shared foundation here, leaving SHACL policy and residual SPARQL/community cases to their own units. |
| One integer and exact-only v4 numeric/default/seconds migration | Existing `purrdf_xsd::bigint::BigInt` public home, useful binary-limb implementation intent from geodesy, and exact-only WIP `5854ef59e`. | Do not hide a second integer or retain a bridging wrapper as the solution. Preserve signed arithmetic, division/remainder, decimal/rational conversion, inline storage and portable behavior through the one engine. Remove bounded Decimal/i128 public numeric alternatives. Default division is exact for terminating reduced denominators, otherwise 18 fractional digits half-to-even; explicit `exact`, `N`, `N:ROUNDING` remain. Temporal seconds are exact. Cover all hosts, governor profile bump/regenerated first-party vectors, Breaking docs/book/zh-Hans. Shared performance requirements from 443 belong here, but its advanced algorithms remain separately owned below. |
| SPARQL solution modifiers | Existing parser/evaluator/serializer paths; current refusal points, rather than an alternative evaluator. | Implement legal ASK LIMIT and grouping/HAVING, DESCRIBE LIMIT and CONSTRUCT GROUP BY. Exact bag/order/slice/aggregation semantics and round-trip behavior through real hosts. This consumes item 6 of 364, not all of 364. |
| Contextual RDFLib algebra and Python row iteration | Preserve/review current 454 donor `cee1c41b2` and its contextual algebra intent; recover 473's two-file WIP from Python query and `.pyi`. | Respect a sibling writer: a historical snapshot is not a stable current source. Preserve the donor's integrated division governance. Native PurRDF evaluates the contextual algebra; avoid foreign CUSTOM_EVALS/callback execution architectures rejected in the portable branch. Separately prove row iteration has the intended ordered values/keys contract without KeyError and retains indexing behavior. The row fix alone cannot close 454, and the algebra fix alone cannot close 473. |
| SHACL-only dated policy and complete reports | Rebuild from retained 462 intent after removing duplicate XPath/kit foundation and superseded main fixes. Retain later corpus fixtures when they differ from portable staged copies. | Explicit REC20170720 and WD20260918 policy, actual loader/validator/rules/target/function callers, report completeness, diagnostic/source identity, host policy parity and strict controls. Include 364's missing prepared/Python/incremental-Wasm options, full-validation ASK/SELECT disagreement and consistent spec citations. Correct CLI diagnostic/error blank labels. The 64 SHACL community cases belong here once kit is shared. |
| Certified arithmetic for geometry | Useful exact/certified numerical intent in stopped geodesy source, grounded in the unified integer/rational home. | Separate pure interval/certification kernels from computational geometry and query/host bindings. Preserve precision laws, exact enclosure, fallible resource admission and independent numerical witnesses. No floating fallback, copied external kernels/coefficient arrays or stale performance receipts. This is part of 408; it does not satisfy geometry or binding acceptance by itself. |
| Computational geodesy and grid/cover substrate | Preserved geodesy and CubeHilbertQ62V1 source after its numerical dependency is repaired. | 408's direct/inverse/bounds geometry plus 409's levels 0-30, physical-scale mixed conservative disk/box covers, emitted-cell count and no false negatives/truncation. Qualify poles/dateline/antipodal/boundary cases and portable execution using actual exact/certified kernels. Separate this substrate from evaluator/host wiring; do not close either umbrella from a kernel test alone. |
| Geometry evaluator and host wiring | Existing extension/property-function seams and stopped binding intent. | Depends on certified arithmetic and geometry/grid units. Drive real SPARQL/GeoSPARQL and every authorized host through the single substrate, with vocabulary/configuration, typed errors, governors, exact deterministic results and current-source end-to-end measurements. Close 408/409 only after their entire captured acceptance surfaces are met. |
| Typed transfer and remembered-graph defaults | Current graph-lifetime implementation and `1b2c0cb67` committed/default WIP intent, preserving staged Python work. Use existing source views/table seams. | 401 still requires preserving asserted/quoted/other table membership and fresh blank-node document scope on LOAD/merge; main LOAD's untyped projected `QuadValues` loop is not a complete transfer solution. 471 changes mutable/UPDATE default to remembered empty graphs on Rust/CLI/Python/C/Wasm, with explicit caller opt-out, CREATE/CLEAR/DROP and governed controls. Graph-lifetime opt-in already landed; default migration and typed transfer remain distinct witnesses. |
| Datatype domains and OWL verdict repair | Existing XSD range home for 451; RDF-aware datatype algebra for 453; current OWL conformance and reasoner homes for 476. | dateTimeStamp is timezone-required dateTime and must be exactly decided without a data-range boundary, including parser/value-domain/range witnesses. langString/dirLangString membership, complement and cardinality are RDF-aware; do not fabricate XSD variants for them. Repair bottomObject/bottomData assertion clashes, Thing equivalent Nothing's nonempty-domain inconsistency, and the cyclic class fixture's verdict; all four published verdicts must agree and the ledger must be empty, with no new RL negative unsoundness. Regular role chains remain separately owned below. |
| Full schema accuracy and input-derived caps | Current checked node-or-literal developer schema behavior under the Full floor, plus existing importer-style input-derived admission patterns. | Retain the owner's OWL DL/RL/Full support floor. 468 requires either exact well-balanced XML judgment or truthful `representation_approximation` in every cell that can hold XMLLiteral, with well/ill-formed witnesses. 474 replaces fixed 65,536-class, 1,048,576 coverage-cell and 65,536 emitter-definition caps with input-derived bounds; retain the shared depth ceiling and prove beyond-old-cap acceptance plus depth-refused neighbors and scaling. These independent criteria must both hold. |
| Deep Turtle/GTS and async Wasm residuals | Current flat quoted terms/stack-safe main traits; permanent deep-chain admission guard; useful stopped Turtle renderer hunk. | 472 retains the permanent guard and cancels the heap-based evaluator proposal. Make the real renderer/drop paths safe and pack depth correct while preserving byte output. GTS key/depth work and nested SELECT-star growing projection vectors remain scaling concerns. 365 additionally owns poison code, stronger sync/async admission/diagnostics, stack HTTP mapping and the slow yielding witness. Its deadline and deep SERVICE egress portions are already implemented; do not re-add old patches. |
| Residual portable corpus, selected measurement and Rust comparison/replay | From 384, retain the 36 non-SHACL community cases after shared kit/64 SHACL cases are assigned above; pure fixture relocation commit `4165c10d380a998e7fed5fe1bdcce283580fd9be` is a candidate after byte checks. | Current main already has composite cursors, remembered graph opt-in/egress repairs, complete-state digest, Rust guards, stack-safe model traits and several SPARQL repairs; exclude duplicates. Freshly qualify semantic interactions, SERVICE and scope cases; exact graded external observations, actual cross-host parity, current-source compiler/artifact identity and selected-graph instruction/allocation phases. Historical 36-configuration grid and 200 phase records are retained data, not final performance. No physical-owner/governor-format/PACK/product/embedding/pipeline rewrite, moved shapes harness or Python compiler-capture helper. |
| Rust LUBM generator and lane | Existing unpublished `purrdf-bench` tooling home and comparison-lane contracts. | 475 requires byte determinism for university count/seed while matching UBA statistics/schema, existing university/index/seed/ontology/document-base knobs, serializer/artifact identity and comparison/replay receipt laws. Remove Java/JRE dependency from scripts/tests/CI/docs. Use licensed specification and frozen historical data as provenance; do not execute the Java generator to qualify Rust. This is a prerequisite for any resumed LUBM lane, not a claim about LargeRDFBench. |

## Unique authorized requirements that foundation overlap does not close

### Advanced arithmetic performance

Issue 443 is not closed by unifying integers, adding inline storage, improving scratch reuse or implementing ordinary Karatsuba/radix/rational paths. Its advanced Toom/NTT, subquadratic division and SIMD requirements, portable vectors, assembly verification and actual ORDER/SUM/AVG/FILTER workloads remain visible authorized work. The one integer and exact-only migration provide the shared foundation. Activate the distinct algorithm/performance requirements against the full original capture after that foundation, and carry each acceptance explicitly to qualification. A simpler foundation implementation cannot silently replace the advanced contract.

### Datalog capacity

Issue 364's capacity requirements remain authorized and cannot disappear into the SPARQL governor work. Main already has `StepGovernor` checks while enumerating candidates (`crates/datalog/src/seminaive.rs:651`), with a per-rule allowance of remaining budget plus one and exact ceiling tests. The old comment claiming unrestricted pre-check materialization is stale. The native join default remains `1 << 20`; remeasure and choose its raised native default after proving bounded enumeration. The term arena remains fixed at `1 << 24`, on every host. Add the fifth caller-settable, target-aware limit across semi-naive evaluation, chase, prepared/cache identity and CLI/Python/C/Wasm. Raising stored-fact/join allowances alone does not admit the user's long-literal datasets.

### Regular role chains

Issue 452 requires exact OWL 2 DL decision for regular role-inclusion chains such as `p o q <= r`, with typed refusal for irregular/non-DL input. It remains distinct from the four verdict fixes, literal datatype domains, schema projection and RL materialization optimizations. Own a complete regularity/admission and decided semantics unit after the relevant OWL foundation; do not leave the property-chain boundary and close the issue on another reasoner improvement.

## Follow-on portions already present and portions still owed

### Evaluation/SHACL follow-on 364

| Body or discussion requirement | Current evidence and disposition |
| --- | --- |
| Check candidate enumeration before growth | Implemented in `StepGovernor` and join loops, but source comment is stale; default remeasurement remains. The round allowance is `rules x allowance`, not a proof of a global one-candidate allocation ceiling. |
| Fifth arena limit | Not implemented at baseline. Constant remains in semi-naive/chase checks and cache digest. Own in the capacity unit above. |
| Python prepared/constructor/incremental options and Wasm incremental controls | Not implemented at baseline. Retained SHACL adds XPath selection but these signatures still omit the requested disallow controls. Own in SHACL parity. |
| Full target ASK/SELECT check | Candidate-only `AskTarget::decide` explicitly says it is not paid during whole validation. Own in SHACL behavior. |
| Dated prebinding consistency/citation | Strong overlap with retained SHACL dated profiles; qualify both REC/WD production routes and legacy behavior rather than assuming the PR title resolves every caller. |
| Legal non-SELECT modifiers | Current parser explicitly refuses several forms. Own in the solution-modifier unit. |
| CLI internal blank labels | CLI prints original `MandatoryDiagnostic`; report's `with_report_blank_labels` produces a separate relabelled copy. Own in SHACL output parity, including rule errors. |
| Later false-DL-proof discussion | Current `crates/validate/src/regime.rs` has a signed judgment for false/unknown and `scalar_proofs_bind_all_three_verdicts` at line 8561. Keep the repair; verify through focused controls before marking that discussion item satisfied. It does not resolve the rest of 364. |

### Async/parser/serializer follow-on 365

| Requirement | Current evidence and disposition |
| --- | --- |
| Hung handler deadline in otherwise idle Node | Implemented: referenced runtime timer with disposal; `async-term-effects.test.mjs` drives child SERVICE/LOAD hung/success/failure/cancel and sibling cases. Keep; qualify current package. |
| Poison error stable code | Still plain `new Error` in `purrdf_jspi.mjs::poisonError`; not satisfied. |
| Identical sync/async admission/budget/position | Current nesting test permits endpoints differing by one and different last-frame construct wording. It is useful control but weaker than the requested contract. Finish on actual production dispatch while preserving before-change performance controls. |
| Admitted nesting maps consistently | `FailureCode::HostStackExhausted` maps to 400, `EvaluationStackExhausted` to 500; Wasm protocol test asserts 500. Complete consistent admission/mapping and real Cloudflare problem output. |
| Excessively slow yielding witness | Deep-chain test still sets `yieldEveryPolls: 0`; reduce witness/quantum while retaining the same suspension, cancellation, floor and result proof. |
| Nested SELECT-star projection scaling | Parser constructs a visible-variable vector at each star projection. Growing nested scope has retained quadratic storage/work potential; do not cite the separate linear BIND-scope repair as proof this is solved. |
| Repeated GTS identity/depth expansion | `deterministic_term_remap` computes depth and expanded identity bytes separately for each term, using linear path membership. Own the memoization/structural-order repair with frozen byte controls. |
| Later 20,000-deep SERVICE egress discussion | Main's `QuotedTerm` is flat/shared, with 100,000-deep small-stack native controls and an actual 20,000-deep package child witness. Keep newer source; original tree-shaped patch is superseded. |

## Complete open-issue overlap inventory

This preserves every item in the prior 30-issue inventory. A mapping is not a closure decision. Where the preparation has only prior audit/body evidence, complete current discussion acceptance must be captured at activation.

| Issue | Scope owner / overlap | Current closure position |
| --- | --- | --- |
| 384 | Shared kit/vocabulary; SHACL corpus; residual portable corpus/measurement/comparison; Rust LUBM provenance | Partial and split. Exclude superseded main repairs and rejected architecture; every retained requirement needs a visible unit. |
| 401 | Typed transfer and graph scope | Unique transfer invariant remains; graph-lifetime repair alone does not satisfy it. |
| 402 | SHACL-only policy/report/parity | Retained 462 asset; not yet complete integration/qualification. |
| 406 | Native XPath | Retain 448; qualify with 467 before final combined completion. |
| 408 | Certified arithmetic; geodesy; evaluator/host wiring | Spans all three units; integer clash must be resolved through the one home. |
| 409 | Conservative mixed grid covers and real host surface | Shares 408 substrate; complete physical-scale/level/count/no-false-negative criteria remain. |
| 443 | Numeric foundation plus distinct advanced algorithms/performance | Partially overlapping, explicitly authorized unique remainder; cannot close on foundation alone. |
| 451 | Exact dateTimeStamp parser/value/range | Numeric-temporal/domain foundations; complete exact decided range required. |
| 452 | Regular OWL role chains | Unique authorized reasoner unit; typed irregular refusal and exact regular decisions. |
| 453 | RDF language/directional literal datatype algebra | RDF-aware domains; no fake XSD datatype route. |
| 454 | Contextual RDFLib algebra | Current donor cee1c41b2 integrates main/division governance; actual full capture and qualification at activation. No assumption that 473 solves it. |
| 467 | Large counted native XPath forms | WIP candidate plus missing documentation/runtime qualification. |
| 468 | XMLLiteral developer-schema truthfulness | Full schema accuracy; exact validation or every affected cell marked representation approximation. |
| 469 | Metered/plain median instructions <=1.20 at 4 and 32 workers | Active governor unit; full typed SUM and identity/trip controls, instruction ceiling and separate allocation comparisons required. |
| 470 | Exact-only v4 APIs, exact-or-18-half-even default and seconds | Whole-host/profile/Breaking migration, dependent on one integer. |
| 471 | Remembered graph default | Existing opt-in is not completion; whole-host explicit-choice/default migration. |
| 472 | Turtle renderer/pack depth | Retain permanent guard, recover stack-safe writer; cancelled heap evaluator stays cancelled. |
| 473 | Python row iteration | Separate language-surface witness within contextual/Python unit. |
| 474 | Input-derived schema and emitter caps | Above-old-cap controls plus permanent depth refusal and scaling. |
| 475 | Deterministic native LUBM tooling | Rust implementation and removal of Java/JRE lane dependency. |
| 476 | Four published OWL verdict divergences | All four leave the ledger; no RL negative regression. |
| 477 | One binary integer | One public/production engine in existing BigInt home; hiding/wrapping duplicate is insufficient. |
| 478 | Governed row-loop regression | Active governor unit; scheduling, determinism, peak spend, OPTIONAL claim and actual A/B. |
| 364 | Capacity, SHACL parity/targets/policy, modifiers and CLI output | Several units plus unique capacity ownership. Present proof-polarity repair does not justify whole closure. |
| 365 | Async refusal/admission, parser/GTS scaling | Some implemented portions, several residuals; do not close from deadline or deep-term repair alone. |
| 308 | Native CI compilation/C smoke measurement | Necessary overlap with current-source build evidence; core duplicate dev/test build already repaired, cold/warm acceptance unproven here. Do not silently add a separate CI redesign to recovery. |
| 367 | Independent SHA-2 project | Outside recovery. Preserve issue; shared arithmetic/SIMD effort does not satisfy a digest implementation. |
| 280 | Independent LargeRDFBench benchmark project | Outside recovery except reusable acquisition/grading/replay infrastructure; distinct workload acceptance remains open. |
| 260 | Independent translation/glossary project | Outside recovery. Required translation of each shipping change still applies; those edits do not close the independent project. |
| 261 | Independent translation/glossary project | Same boundary; retain its unique requirements without expanding this recovery. |

## Bounded external and measurement findings

### Native CI/C smoke overlap

`crates/rdf-capi/tests/c_smoke.rs` already selects the compiling harness's profile, invokes a fresh Cargo build, obtains the actual cdylib from Cargo JSON and compiles/links/runs the real C smoke source/header. Commit `65e110cb9` also separates C smoke from six native test shards. This addresses the old duplicate dev/test compilation mechanism. The independent issue's original cold/warm CI qualification, current-source artifact reuse and full assertion/lint/hygiene coverage remain unproven during this audit. Retain the correct fingerprint-driven build pattern in recovery receipts; an executable merely existing on disk proves nothing.

### LargeRDFBench availability and reuse boundary

Upstream [LargeRDFBench PR 4](https://github.com/dice-group/LargeRDFBench/pull/4) remains open, but its README update advertises an Interoperable Edition through the [IDLab public share](https://cloud.ilabt.imec.be/index.php/s/xzE93HaqAbr2kSD). The share returns HTTP 200, exposes read-only permissions and advertises 12,930,250,998 bytes. Thus the old claim that no data exists until PR merge is no longer sufficient. No dataset was downloaded; bounded public-DAV acquisition/metadata probes returned 401, so acquisition and complete artifact/checksum/provenance/space placement remain unverified.

The [modernization pipeline](https://github.com/shape-federated-queries/large-rdf-bench-modernization) invokes `hdt-java` for two large datasets. Consuming cleaned RDF artifacts can avoid Java; executing that default pipeline cannot. The [upstream result-analysis comparator](https://github.com/shape-federated-queries/large-rdf-bench-result-analysis) ignores literal whitespace, so it must not become PurRDF's normative exact grader. Reuse the shared exact multiset grader, acquisition identities and replay receipts if this independent project is later activated. The federated multi-source query workload and its corrected reference results remain distinct from LUBM's generated entailment workload.

### Independent comparison families and historical Java

Previously inspected first-party documentation identifies [Rudof's native SHACL lane](https://github.com/rudof-project/rudof/blob/master/shacl/README.md) and [Oxigraph's SPARQL lane](https://github.com/oxigraph/oxigraph/wiki/SPARQL) as non-Java external candidates. They can share a SPARQL backend and therefore cannot count as two independent query engines. A genuinely different family such as pySHACL/RDFLib may be useful if the owner permits a Python external comparator; Rust implementation of the comparison runner does not itself imply every external engine must be Rust. Pin exact executable/version/build capability and preserve grammar/semantic differences in explicit triage. Do not call two wrappers around one engine independent or treat Jena history as a freshly executed comparison.

## Completion accounting

For each activated unit, record the original captured body and substantive discussion, explicit acceptance cells, retained/excluded source inventory, final current-source receipts, actual host results, documented semantic changes, normal hooks, hosted review/CI and ghprsq integration. Mark a requirement satisfied only by its own evidence. Keep advanced arithmetic, Datalog capacity and regular role-chain requirements visibly owned until complete; do not silently defer them or remove their issue because a neighboring foundation lands. Release packaging, registry publication and downstream live acceptance remain separate from source recovery.
