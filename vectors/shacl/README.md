# Vendored W3C SHACL test suite (data-shapes)

Frozen copy of the W3C SHACL test suite, vendored for the
`purrdf-shapes` conformance harness. **Do not hand-edit** — byte-frozen
third-party conformance data. The freeze is enforced: `make check` runs
`scripts/check-corpus-frozen.py`, which SHA-256-verifies every file here
against `scripts/conformance-frozen/vectors-shacl.sha256`, so a silent content
edit fails the build. A deliberate re-vendor regenerates that manifest with
`python3 scripts/check-corpus-frozen.py --update`.

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

## First-party AF seam (`af/`)

`af/manifest.ttl` is a **first-party** conformance seam, NOT part of the
vendored upstream mirror — upstream at the pinned commit ships no SHACL
Advanced Features (AF) tests. The placeholder manifest declares itself a
manifest with no entries, so the harness discovers **0 tests** from it today
and the total stays 120. When upstream publishes AF manifests they vendor in
here and the harness picks them up automatically (the harness discovers
`af/manifest.ttl` directly, so the vendored root `manifest.ttl` stays
pristine). First-party AF coverage (e.g. `sh:expression`) already lives in the
shapes corpus (`crates/shapes/corpus`).
