# String admission validation

Both public datatype parse doors now use one borrowed XML-character validator. AnyURI delegates to that reader before applying its existing collapse formatter. The former independent URI validation branch and unchecked String admission branches are removed. Valid String lexical bytes remain unchanged.

Completed checks on the four-file patch:
- XSD crate suite: 413 unit tests, all integration suites and doctests passed (xsd-tests.log).
- XSD all-target clippy with warnings denied passed (clippy.log).
- Actual wasm32 portable text-primitive suite: all three tests passed (wasm-text-primitives.log), including both text primitives refusing invalid XML characters and String preserving valid whitespace.
- Invalid admitted String inputs return Forg0001 before any admitted allocation; the existing accounting matrix passed.

The complete entailment library suite passed: 718 passed, zero failed, one ignored (entail-tests.log). Full workspace/hosted CI is not claimed passed. Existing user instruction authorizes pushing and merging with subsequent hosted CI findings handled separately. No local GMEOW corpus factory or machine inspection was run.

Production-constructor searches found the two repaired parse branches; other consumer occurrences inspected were matches or test fixtures. The supplied-buffer UTF-8 copier remains because it is a distinct low-level operation, not datatype admission.
