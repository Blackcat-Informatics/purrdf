# Schema surface: remove fixed caps that refuse valid large input

Source: https://github.com/Blackcat-Informatics/purrdf/issues/474
State: OPEN; updated: 2026-10-09T03:11:42Z; full comments: 0.

These fixed caps refuse valid large ontologies:
- the class compiler cap (65,536 classes);
- class × property coverage cells (1,048,576);
- the TypeScript, GraphQL and Pydantic emitter definition caps (65,536).

Derive each bound from the input instead, as the JSON Schema importer now does. Keep the shared depth ceiling. Tests: inputs past each old cap compile and emit, alongside a neighbour still refused by the depth ceiling. Add bench lanes for scaling. Refs #416.

## Comments

None.

