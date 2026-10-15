# #492 portable registration and documentation: bounded source review

Status: REVISE documentation; portable execution UNVERIFIED.

Scope: current Makefile `wasm-test` selection, `docs/WASM_TESTING.md`, the new graph-free sections in the backend contract and Datalog/Text/Shapes READMEs, and the interned-dataset book source/Chinese PO. This is a source and claim review, not a WASM run, complete corpus adjudication, or whole-issue verdict.

## Registration and claim checks

- The four new Makefile invocations select five `triple_carrier`, four Datalog `rules_runtime`, two evaluator `host_identity`, and two Shapes `srl_language` functions. The two added `purrdf-text` `physical_shapes` selections are present in the original testkit harness. All 15 named functions exist in the selected source targets. The old invocations and runner remain in place. The new aggregate claims 120→135 distinct cases, 19→23 targets, 125→140 executions, and 23→27 invocations are arithmetically consistent with the diff. None of those added WASM selections has yet been observed executing.
- The documented 12/8/12 row widths, checked `TripleDataset::read` session, named-role refusal, explicit `infer_triples` result, and `TextIndexConfig<GraphFree>::for_triples` have corresponding production declarations. The new book paragraph is consistent with those APIs. The Chinese PO contains translations for the six added/changed messages; the retained `book-po-update.log` records terminal 0. I did not rerun `msgfmt` or book rendering.
- The Datalog and Shapes README statements match the selected source interfaces and previously reviewed static-arity/public-SRL source. They remain behavior claims awaiting the current native/portable and corpus qualification.

## Required correction

`crates/text/README.md` says to select `Config::for_triples`. There is no exported `Config` alias: `crates/text/src/lib.rs` exports `TextIndexConfig`, and `crates/text/src/index.rs` defines `TextIndexConfig<GraphFree>::for_triples`. Change the README call to that precise public name (the crate-root doctest already uses it). This is a user-facing API instruction error, not a production source defect.

After that correction, this review has no further source/claim finding within its bounded scope. Actual scalar/SIMD execution, official corpus results, final digest pins, and full #492 delivery remain open.

## Corrected source readback (2026-10-13)

`crates/text/README.md:31` now names `TextIndexConfig::<GraphFree>::for_triples`, matching the public implementation and crate-root example. The single documentation finding above is resolved. This readback adds no portable execution result; that qualification and the remaining #492 acceptance stay open.
