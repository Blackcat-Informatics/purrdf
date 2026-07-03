# Vendored ShEx conformance suite (shexTest)

Frozen copy of the official ShEx test suite, vendored for the
`purrdf-shex` conformance harness. **Do not hand-edit** — treat exactly
like the GTS vectors: byte-frozen third-party conformance data. The freeze is
enforced: `make check` runs `scripts/check-corpus-frozen.py`, which
SHA-256-verifies every file here against
`scripts/conformance-frozen/vectors-shexTest.sha256`, so a silent content edit
fails the build. A deliberate re-vendor regenerates that manifest with
`python3 scripts/check-corpus-frozen.py --update`.

- Upstream: <https://github.com/shexSpec/shexTest>
- Tag: `v2.1.0` (commit `8772d2d32c94bfba21a30c09915dfc7662e1539f`)
- License: MIT (per upstream `package.json`)
- Vendored subset: `schemas/` (ShExC↔ShExJ representation tests),
  `validation/` (validation/failure tests + data), `negativeSyntax/`,
  `negativeStructure/`, `context.jsonld`.

The `v2.1.0` tag is pinned deliberately: upstream `main` has drifted to
2.2-alpha and contains ShEx 2.next `EXTENDS` tests that are out of scope
for a ShEx 2.1 implementation.

Harness: `crates/shex/tests/` reads the Turtle manifests in each
directory; expected-failure entries live in the harness xfail ledger,
never here.
