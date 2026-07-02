# Vendored W3C SHACL test suite (data-shapes)

Frozen copy of the W3C SHACL test suite, vendored for the
`purrdf-shapes` conformance harness. **Do not hand-edit** — byte-frozen
third-party conformance data.

- Upstream: <https://github.com/w3c/data-shapes>
  (`data-shapes-test-suite/tests/`)
- Commit: `08adb3776709a014bc3062ede793c36275b22446`
- License: W3C Software and Document License
  (<http://www.w3.org/Consortium/Legal/copyright-software>)
- Vendored subset: `core/` (SHACL Core tests), `sparql/`
  (SHACL-SPARQL tests), `manifest.ttl`.

Harness: `crates/shapes/tests/w3c_conformance.rs` reads the Turtle
manifests; expected-failure entries live in the harness xfail ledger,
never here. The first-party frozen corpus in `crates/shapes/corpus/`
remains separate and authoritative for purrdf-specific behavior.
