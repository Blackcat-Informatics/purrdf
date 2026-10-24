OpenAPI output stored definitions under `components/schemas` but referenced the absent root `$defs`. Relocate those references while transferring the shared catalog, preserving escaped pointer tokens, recursive edges, literal instance data and the JSON Schema artifact. Update the compilation policy identity so cached output is regenerated.

The supported golden changes exactly 36 OpenAPI reference prefixes; JSON Schema and coverage bytes are unchanged. Native regressions check complete document reference closure and equivalent constraints across ontology and vocabulary surfaces.

Validation: all 85 native schema-module tests and all 29 ontology-surface tests pass. Strict Shapes all-target Clippy with warnings denied and the final recursive/escaped-reference regression pass. Normal commit hooks pass.

Closes #692.
