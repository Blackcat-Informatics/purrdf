# Current acceptance

Implementation is committed/pushed as 0461e3d46 in PR #693. Production callers
in both compilation modes consume the shared definition map only after JSON
Schema emission. OpenAPI relocation uses the existing schema keyword lists and
heap WorkList, never unfolds reference edges, and moves the catalog directly
past the borrowing JSON builder. JSON Schema generation remains unchanged. The
compilation policy identity changes to invalidate caches of the broken artifact.

| Requirement | Current evidence |
| --- | --- |
| Resolvable OpenAPI references | Native test code checks the complete physical document via the shared JSON Pointer resolver; openapi-native.log passes exactly one actual public regression; schema-native.log passes all 85 JSON-schema module tests (exit 0). |
| Recursive and escaped references; literal data unchanged | schema-native.log passes the new recursive/escaped-key/data-preservation regression. |
| Annotation, class/Node, Literal, vocabulary and catalog definitions | Existing actual compile cases now check closure/equivalent schema constraints; all 85 focused JSON-schema module tests pass, including ontology Literal/class and vocabulary enum callers. |
| Proportional work/allocation | Source walks each emitted schema node once with shared WorkList and constant keyword sets; catalog moved without cloning per reference. No measured performance claim. |
| Goldens / JSON Schema byte preservation | Supported PURRDF_WRITE_GOLDEN test wrote the fixture and returned its expected embedded-old-golden mismatch (golden-generation.log, exit 101). Recompiled normal ontology-native.log passes all 29 cases, exit 0, including golden equality and historical semantic scope. Inspected full fixture delta: exactly 36 OpenAPI $ref prefixes change; JSON Schema and coverage bytes are unchanged. |
| Caller lint / hooks / CI / independent review | First strict all-target Clippy rejected five redundant clones in the new test fixture (clippy.log, exit 101). Removed those clones; final strict Shapes all-target Clippy passes in clippy-final.log, exit 0. No production change; ref-fixture-final.log passes exactly the changed recursive/escaped-reference/data-preservation native test, exit 0. All broader results remain applicable: the only subsequent change removes redundant test-only clones. Normal commit hooks pass (commit.log, exit 0), producing 0461e3d46. Normal push passed (push.log, exit 0); PR #693 is open. Hosted run 40051857586 completed successfully; final stagectl checks report state=pass. Independent source review covers 0461e3d46 with no actionable findings (raw/issue-comments.json, coverage kind=reviewed). The final merge brief passes all current-head checks. Fresh paginated reviews/inline captures and full thread capture have zero items/threads and no remaining page. Paginated reviews/inline comments and complete review-thread capture currently have zero submissions/threads; independent technical review is complete in the published walkthrough. |

Commands use /home/paudley/stage/root/bin/cargo normally, eight build jobs and
four native test/Rayon threads. First cargo fmt --check reported formatting
changes; normal cargo fmt completed. The initial build and full schema-module handles are terminal. Golden generation and all 29 ontology controls are terminal. Strict Shapes all-target Clippy is terminal and passing. The final changed fixture regression is terminal and passing. Normal commit hooks are terminal and passing. Push is terminal and passing. No owned local Cargo process is live. Preserve
sibling processes. Hosted acceptance and independent review are complete. The clean integration candidate against base e3eb29225 and head 0461e3d46 is e713634aa2f50f08c82e545d32f002589eaa9804; base has not moved since branch setup, so current-head qualification covers the candidate. Final merge/archive/cleanup have not happened yet. Focused
source formatting and git diff --check pass. No WASM semantic execution is planned:
these are general schema correctness obligations owned by native Rust.

Source references: crates/shapes/src/json_schema.rs, schema_catalog.rs, and
ontology_schema_surface.rs. Specification consulted: OpenAPI 3.1.0 sections 4.6,
4.8.23 and 4.8.24 (https://spec.openapis.org/oas/v3.1.0.html).
Graphify exact-symbol observation was stopped after it did not return promptly;
no graph evidence, reindex, or infrastructure change is claimed.
