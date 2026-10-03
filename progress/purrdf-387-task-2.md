Task 2 is implemented and pushed as `9837c4214e2899602f4303ab9686d5e404982449`.

The portable inventory contains 21 independently specified cases (18 evaluation controls and three syntax refusals), with a W3C-compatible SPARQL 1.1 manifest and a separately labelled RDF 1.2 quoted-term case. Native, retained-prepared, raw-prepared, repeated execution and checked carrier/reparse paths preserve exact bags and visible columns. SHACL restrictions are tested independently through the shapes loader.

The experiments exposed production defects that are repaired on this branch:

- Checked carriers now share evaluation's authoritative positive-spine ownership law. Independent raw blank owners receive distinct legal labels; sharing across required braces uses an unprojected fresh alias. Legal parsed spelling is preserved, and opaque raw labels use the shared exact lexical validator.
- Concrete dataset blank bindings remain supported by native injection, including quoted terms, and receive an actionable refusal at checked text boundaries that cannot represent them.
- UPDATE and CONSTRUCT append share the existing deterministic destination namespace law. UPDATE inventories retained identities without freezing the dataset, including suppressed terms and blanks inside quoted/composite values; repeated requests and LOAD cannot collide with template allocations.

Independent production/integration review accepted the final source. Validation passed: nine dedicated carrier tests; eight typed-invariant tests; nine alternative-path tests; 24 existing serializer sweep/nesting tests; ten evaluator interaction tests; the 21-case portable inventory; two SHACL profile tests; two retained-identity visitor tests; 32 existing blank-label controls; 35 lexical terminal controls; strict focused clippy. The earlier complete evaluator library run passed all 1,343 tests. Commit hooks ran normally.

The candidate comparison and measured report are separate logical units. Full repository, wasm and conformance gates remain required before PR creation.
