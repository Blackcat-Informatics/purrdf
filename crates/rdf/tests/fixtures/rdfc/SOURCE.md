# W3C RDF Dataset Canonicalization (RDFC-1.0) test suite — vendored

These fixtures are the **canonicalization (`rdfc10`) test vectors** from the W3C
`rdf-canon` test suite, vendored verbatim for the native RDFC-1.0 conformance gate
(`crates/rdf/tests/rdfc_w3c.rs`).

- **Upstream:** <https://github.com/w3c/rdf-canon> — `tests/rdfc10/`
- **Spec:** *RDF Dataset Canonicalization* (RDFC-1.0), W3C Recommendation —
  <https://www.w3.org/TR/rdf-canon/>
- **License:** dual W3C Test Suite License + W3C 3-clause BSD License (see the
  upstream `LICENSE.md` / the header of `tests/manifest.ttl`). Redistribution for
  conformance testing is permitted under both.

## Layout

- `testNNN-in.nq` — the input N-Quads dataset.
- `testNNN-rdfc10.nq` — the expected canonical N-Quads (RDFC-1.0). Inputs WITHOUT a
  matching expected file are **negative** (poison / complexity-limit) tests that the
  canonicalizer must abort rather than complete (e.g. `test074`, a 10-node blank-node
  clique).
- `testNNN-rdfc10map.json` — the expected issued-identifier map (informative here).

## Hash algorithm

Every vector uses SHA-256 except `test075`, which the manifest tags
`rdfc:hashAlgorithm "SHA384"` ("blank node - diamond (uses SHA-384)"). The harness
selects SHA-384 for it via `CanonHash::Sha384` (see `SHA384_TESTS` in the harness).

## Updating

Re-vendor by copying `tests/rdfc10/*.nq` and `*-rdfc10map.json` from a fresh checkout
of `w3c/rdf-canon`. The harness discovers vectors by filename, so new vectors are
picked up automatically.
