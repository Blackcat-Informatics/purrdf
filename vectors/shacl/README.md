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

## Vendored AF seam (`af/`)

`af/` is a **vendored third-party** seam, not part of the W3C mirror above and
not first-party: its SHACL Advanced Features tests come from the pySHACL/DASH
suite (Apache-2.0 — see `LICENSING.md`'s carve-out table), because the W3C
suite at the pinned commit ships no AF tests of its own. The harness discovers
`af/manifest.ttl` directly, so the vendored W3C root `manifest.ttl` stays
pristine; the seam contributes **6** `sht:Validate` entries of the harness's
126 total (120 from `core/` + `sparql/`). First-party AF coverage (e.g.
`sh:expression`) lives separately in the shapes corpus
(`crates/shapes/corpus`).
