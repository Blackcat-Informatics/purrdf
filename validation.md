# Canonicalization acceptance

VERDICT: PASS
Scope: frozen refusal propagation through actual public identity/comparison
paths at f0790be778cb81201162e3649815f423974305fa. Successful canonical bytes,
labels, hash choice and refusal order are preserved. Parent budgets/defaults,
UDF execution and Datalog policy remain open.

| Evidence | Result and tested scope |
|---|---|
| core-canonicalization-native.txt | Exit 0; 102 core canonicalization, comparison and pipeline bundle unit cases. |
| core-view-identities-native.txt | Exit 0; 12 CDT blank-identity, 25 dataset-state and 60 shared-view cases. One unrelated pre-existing large-compaction case retains its explicit memory-ceiling requirement. |
| rdf-canonicalization-certification-native.txt | Exit 0; 17 GTS certification, 12 RDF 1.2 profile and six W3C RDFC cases, including public digest/verifier poison refusal and healthy controls. |
| identity-consumers-native.txt | Exit 0; 23 reasoning/proof, ten flat-output and 24 SPARQL comparison cases. Original cause survives input disposal and REDUCED search; frozen proof digests pass. |
| justification-boundary-native.txt | Exit 0; all three shared justification refusal/healthy/certificate cases. |
| production-callers-clippy.txt | Exit 0 with warnings denied; core, RDF, reasoning, shared validation, SPARQL conformance, CLI, C, Python and WASM production libraries/binaries. |
| Selected fixture Clippy logs | Exit 0 with warnings denied for all 37 changed integration targets and three benchmarks. Named per-domain logs retain results. Benchmarks compile only; no timing claim. |
| library-fixture-consumers-build.txt | Exit 0; Shapes, SPARQL evaluator and Columnar library test binaries compile. Build-only evidence. |
| example-clippy.txt | Exit 0 with warnings denied for projection_archive. |
| shared-view-correction.txt | Exit 0; exact existing graph projection/isolation/refusal case after removing a redundant comparison. |
| wasm-package-build.txt | Exit 0; optimized actual package, binding generation, optimization, link guarantees and SIMD verification. |
| js-canonicalization-host-error.txt | Exit 0; exactly one documented JavaScript Error construction/prototype case, with native IRI text and no runtime trap. General semantics stay native. |
| i18n-correction.txt | Exit 0; glossary and all six translated-book rendering gates. All 3375 translated messages render; nine pre-existing absent source messages are outside this migration. |
| capacity-helpers-gate.txt | Exit 0; full helpers-hygiene, capacity/source census, self-tests and hash-domain checks after reviewed pin refresh. |
| commit.txt, correction-commit.txt, capacity-commit.txt and corresponding push logs | Every final normal signed commit/push exits 0. Initial failed layer declaration attempt is retained separately and corrected. |
| preflight-current.json, status.json, pr-checks.txt | Complete final-head hosted qualification: 50 SUCCESS, six SKIPPED, no blockers, unavailable sources or review debt. |

Native executions total 294 distinct passing cases, plus the explicit existing
ignored control. The corrected graph case repeats selected coverage and does not
inflate that count. Strict/build-only checks do not stand in for execution.
Raw .txt outputs are retained without encoding or changing any gate.

Independent review covers the substantive API change and corrections. Valid
documentation/redundancy findings are addressed. Native compilation and execution
contradicted the tuple-pattern assertion; the reviewer explicitly withdrew it.
The claimed CSVW mode change is absent and also withdrawn. Complete feedback,
revisions and dispositions remain, with UI projection documented separately.

Skipped jobs are Deploy to GitHub Pages, simd-asm-projection,
native-profile-admission, native-profile, native-profile-comparison and native.
These are recorded forge outcomes; no extra waiver is introduced. Required
native, conformance, host, release portability, documentation and hygiene pass.

Integration: origin/main is 2341eb0ad. Clean merge-tree prediction:
edef4d74c762286a033711b7949ef87fd549a885. The additional assembly lease has
its own passing qualification and changes no Rust caller/dependency. No base
synchronization or manual workspace replay is justified.
