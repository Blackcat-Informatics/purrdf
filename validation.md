# XML 1.1 text-domain qualification

The actual hosted failure on PR 576 is core/property/singleLine-001: the SHACL suite produced two unexpected DatatypeConstraintComponent violations on InvalidInstanceF (form feed) and InvalidInstanceV (vertical tab). Source log: https://github.com/Blackcat-Informatics/purrdf/actions/runs/39030094987/job/117388654055. This is an observed regression, not an inferred infrastructure issue.

XSD 1.1 §1.3 and §3.3.1 allow selecting either XML 1.0 or XML 1.1 characters. Native text datatypes now explicitly select XML 1.1 through one lexical terminal. The existing XML 1.0 parser terminal is unchanged. The range alphabet is derived at compile time from the selected terminal ranges and independently censused as 1,112,061 admitted scalars in the finite-exhaustion regression. The shared String/AnyURI reader still refuses NUL/U+FFFE/U+FFFF before allocation and admits XML 1.1 controls without losing bytes.

Completed checks:
- cargo test -p purrdf-lex terminals::tests: 27 terminal tests passed, including the full scalar census against declared ranges (lex-tests.log).
- cargo test -p purrdf-xsd: complete unit, integration, allocation-governance and doctest suites passed (xsd-tests.log).
- cargo clippy -p purrdf-lex -p purrdf-xsd --all-targets -- -D warnings: passed (clippy.log).
- Actual wasm32 text contract: all three tests passed, including valid controls and invalid scalar refusal for both primitives (wasm-tests.log).

The complete cargo test -p purrdf-shapes --test w3c12_conformance passed all 16 tests, including both full inventory checks and the previously failing singleLine case (shacl12-tests.log). No frozen conformance expectations, ledgers or goldens were weakened. Full workspace hosted CI remains separate. No local GMEOW producer, machine inspection or resource workaround was invoked.
