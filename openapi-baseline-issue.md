OpenAPI output contains local JSON Schema references such as `#/$defs/Annotation` while storing the definitions only under `components/schemas`. Those document-local targets do not exist in the emitted artifact.

Verified on the integration base d26bc6e640ae116754aebb0cadcf40cb6a19933c, before PR #691:

- `crates/shapes/src/json_schema.rs::openapi_doc` clones the shared definitions into `components/schemas` without relocating their references. The function is byte-identical between that base and ea133b288.
- The tracked `crates/shapes/tests/fixtures/iri-only-surface.golden.txt`, section `ontology-complete openapi`, contains `components.schemas.Agent.properties.@annotation.$ref = "#/$defs/Annotation"` and further `#/$defs/Literal` references.
- That emitted document has no root `$defs` and no `$id` anywhere. `components.schemas.Annotation` exists; the referenced root `$defs.Annotation` does not. The frozen golden is unchanged in PR #691.

This is an existing packaging defect, discovered while reconciling the PR's general OpenAPI risk summary. It is tracked separately because the user explicitly froze #691 to the already-published allocation/shared-catalog work and its regressions. It is a bug, not an accepted output contract.

The [OpenAPI 3.1.0 reference rules](https://spec.openapis.org/oas/v3.1.0.html#relative-references-in-urls) use the relevant base URI and JSON Schema `$id` rules; consumers must be able to resolve the artifact's local references.

Acceptance:

- Emit resolvable local references under `#/components/schemas/...` or another explicitly valid OpenAPI structure, including Annotation, Literal, Node/class branches, vocabulary enums and shared definitions.
- Add a native reference-closure regression that walks the complete OpenAPI document, checks every local target, and covers escaped definition keys and recursive references without unfolding their graphs.
- Keep the JSON Schema artifact's `#/$defs/...` references unchanged; preserve equivalent schema constraints across both surfaces.
- Keep work and allocation proportional to emitted structure, avoiding a catalog copy per reference.
- Regenerate affected golden artifacts through their supported path and state the emitted-byte change. Run focused schema/OpenAPI and ontology surface checks plus required gates.
