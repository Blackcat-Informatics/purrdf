# Requirement candidates — issue #492

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 17 | modal | Katamari's data model is one RDF 1.2 graph per vault, by decision: "No named graph, graph name, quad or dataset-kind shall appear in authority, projection, page layout, index key, export or wire format" (Katamari ADR-0038 38.1). Claims are reifiers with annotations; standpoint and lifecycle are annotations, not graphs. |
| B2 | body | 33 | list-under-requirement-heading | - A triple-only dataset holding N triples uses 12N bytes of row storage (plus permutations), not 16N. |
| B3 | body | 34 | list-under-requirement-heading | - SPARQL conformance (the default-graph portion) passes unchanged on the triple-only carrier; `GRAPH ?g { … }` is refused with a typed diagnostic. |
| B4 | body | 35 | list-under-requirement-heading | - SPARQL 1.2 RL rules over a triple-only input produce the same model as over the quad carrier with all-default graphs (test), with arity-3 atoms. |
| B5 | body | 36 | list-under-requirement-heading | - RDFC over a triple-only dataset equals the quad form's canonical output for the same triples (or the distinct identity is documented and tested). |
| C1.1 | comment 1 (paudley, 2026-10-10T23:53:57Z) | 95 | modal | changes integrate; native/host IDs must not be remapped at integer joins. |
| C1.2 | comment 1 (paudley, 2026-10-10T23:53:57Z) | 129 | modal | \| Native/portable/hygiene/docs/landing \| Complete affected native and actual WASM executions, original controls/scale bench, strict workspace consumers, generated docs/book, one complete final gate after corrections settle; independent full-contract audit, normal hooks, required hosted checks, ghprsq and owned cleanup \| 2,3 \| |
| C1.3 | comment 1 (paudley, 2026-10-10T23:53:57Z) | 145 | modal | required equivalent for settled source. No baseline whole-workspace discovery. |
| C1.4 | comment 1 (paudley, 2026-10-10T23:53:57Z) | 147 | modal | qualification. Commit/push the complete issue normally after required review/gates. |
| C1.5 | comment 1 (paudley, 2026-10-10T23:53:57Z) | 162 | modal | existing published crates/hosts must still build and preserve original behavior, |
| C1.6 | comment 1 (paudley, 2026-10-10T23:53:57Z) | 163 | modal | and actual portable execution of the new core/rule/text contract is required. |
| C1.7 | comment 1 (paudley, 2026-10-10T23:53:57Z) | 165 | modal | is deferred to another issue. Planning recheck must adjudicate the concrete |
| C2.1 | comment 2 (paudley, 2026-10-13T19:45:59Z) | 98 | modal | changes integrate; native/host IDs must not be remapped at integer joins. |
| C2.2 | comment 2 (paudley, 2026-10-13T19:45:59Z) | 133 | modal | \| Native/portable/hygiene/docs/landing \| Complete affected native and actual WASM executions, original controls/scale bench, strict workspace consumers, generated docs/book, one complete final gate after corrections settle; independent full-contract audit, normal hooks, required hosted checks, ghprsq and owned cleanup \| 2,3 \| |
| C2.3 | comment 2 (paudley, 2026-10-13T19:45:59Z) | 149 | modal | required equivalent for settled source. No baseline whole-workspace discovery. |
| C2.4 | comment 2 (paudley, 2026-10-13T19:45:59Z) | 151 | modal | qualification. Commit/push the complete issue normally after required review/gates. |
| C2.5 | comment 2 (paudley, 2026-10-13T19:45:59Z) | 166 | modal | existing published crates/hosts must still build and preserve original behavior, |
| C2.6 | comment 2 (paudley, 2026-10-13T19:45:59Z) | 167 | modal | and actual portable execution of the new core/rule/text contract is required. |
| C2.7 | comment 2 (paudley, 2026-10-13T19:45:59Z) | 169 | modal | is deferred to another issue. Planning recheck must adjudicate the concrete |
| C2.8 | comment 2 (paudley, 2026-10-13T19:45:59Z) | 211 | modal | publication; any shared builder control must not retain quad rows or expose a |
| C2.9 | comment 2 (paudley, 2026-10-13T19:45:59Z) | 214 | modal | an unsupported named graph must not become a successful empty result through |
| C2.10 | comment 2 (paudley, 2026-10-13T19:45:59Z) | 219 | modal | Independent planning recheck must accept the concrete TripleRead and closed |
| C4.1 | comment 4 (paudley, 2026-10-14T18:21:53Z) | 10 | modal | parity and final hosted checks/review remain required. |

## Comments that matched no rule

- comment 3 (paudley, 2026-10-14T14:42:40Z)
- comment 5 (paudley, 2026-10-15T03:12:22Z)
