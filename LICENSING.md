# Licensing

PurRDF (`purrdf`) is **triple-licensed**. Blackcat Informatics® Inc. is the sole
copyright holder (© 2026) and makes the work available under the open-source terms
below **and** reserves the right to grant separate commercial/proprietary licenses.

## Open-source terms

All first-party material in this repository — the Rust workspace, the Python,
WebAssembly, and C bindings, first-party test fixtures and harnesses,
documentation, and build tooling — is offered under your choice of either:

| License | Text |
|---|---|
| **MIT** | [`LICENSE-MIT`](./LICENSE-MIT) |
| **Apache License 2.0** | [`LICENSE-APACHE`](./LICENSE-APACHE) |

You may use the software under the terms of **MIT _or_ Apache-2.0, at your option**.
This is expressed in every first-party source file and package manifest with the SPDX
identifier:

```text
SPDX-License-Identifier: MIT OR Apache-2.0
```

Some documentation files instead carry `SPDX-License-Identifier: CC-BY-4.0`; those are
still first-party Blackcat Informatics® material, licensed under
[Creative Commons Attribution 4.0](https://creativecommons.org/licenses/by/4.0/).

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work shall be dual-licensed as above (MIT OR Apache-2.0), without any
additional terms or conditions.

## Third-party material (carve-out)

**The terms above do not apply to the vendored third-party conformance corpora.**
Those files are *not* Blackcat Informatics® copyright, are vendored verbatim, are
**not** relicensed, and are not redistributable under MIT or Apache-2.0.

| Vendored tree | Upstream | License |
|---|---|---|
| `crates/sparql-conformance/suite/w3c-sparql11/` | W3C `rdf-tests` SPARQL 1.1 suite, plus the referenced W3C RIF Working Group rule documents | `LicenseRef-W3C-Test-Suite` (W3C Test Suite / Software and Document License) |
| `crates/sparql-conformance/suite/w3c-sparql12/` | W3C SPARQL 1.2 / RDF 1.2 suite | `LicenseRef-W3C-Test-Suite` |
| `crates/sparql-conformance/entailment-suite/w3c-owl2/` | W3C OWL 2 test suite | `LicenseRef-W3C-Test-Suite` |
| `crates/sparql-conformance/entailment-suite/w3c-owl2-rl/` | W3C OWL 2 test suite (entailment cases) | `LicenseRef-W3C-Test-Suite` |
| `crates/rdf/tests/corpus/w3c/` | W3C `rdf-tests` syntax corpus | W3C test-suite dual licensing (see its `LICENSE`) |
| `crates/rdf/tests/fixtures/jsonld-w3c-rec/` | W3C JSON-LD 1.1 test suite | W3C test-suite dual licensing (see its `LICENSE.md`) |
| `crates/rdf/tests/fixtures/rdfc/` | W3C `rdf-canon` (RDFC-1.0) vectors | W3C Software and Document License |
| `crates/rdf/tests/fixtures/csvw-w3c/` | W3C CSVW manifests | W3C Document License |
| `crates/rdf/tests/fixtures/obographs-0.3.2/` | official OBO Graphs JSON Schema closure | BSD-3-Clause |
| `vectors/shacl/` (`core/`, `sparql/`) | W3C SHACL `data-shapes-test-suite` | W3C Software and Document License |
| `vectors/shacl/af/` | pySHACL DASH tests | Apache-2.0 |
| `vectors/shexTest/` | shexTest v2.1.0 | MIT (per upstream `package.json`) |

The first-party selectors, harnesses, and reconstructed expected-result files that sit
*inside* those trees remain MIT OR Apache-2.0. Each tree's own `LICENSE`,
`LICENSE.md`, `LICENSES/`, `PROVENANCE.md`, or `README.md` is authoritative for its
upstream terms, source URL, and pinned revision; the table above is a summary, not a
substitute.

Two gates keep this honest, and each covers a stated subset rather than the whole
repository:

- `scripts/check-licenses.py` treats any directory under `crates/` or `bindings/` that
  holds a `LICENSES/` subdirectory as a vendored root, and fails the build if a file
  beneath one lacks a `.license` SPDX sidecar, an inline SPDX header, or a `REUSE.toml`
  annotation. Today that covers the four W3C SPARQL/OWL 2 suites and the OBO Graphs
  schema closure.
- `scripts/check-corpus-frozen.py` SHA-256-verifies `vectors/shacl`,
  `vectors/shexTest`, `crates/shapes/corpus`,
  `crates/sparql-conformance/entailment-suite/w3c-owl2` and
  `crates/sparql-conformance/entailment-suite/w3c-owl2-rl` against committed
  freeze manifests, so vendored bytes cannot be edited in place.

## Proprietary / commercial licensing

The open licenses above are offered **in addition to — not in place of** — Blackcat
Informatics®' right, as copyright holder, to license the software under separate
commercial or proprietary terms. Granting the open licenses does not revoke or limit
this reservation.

To obtain a proprietary license, contact **licensing@blackcatinformatics.ca**.

## Trademarks

"Blackcat Informatics®" is a registered trademark of Blackcat Informatics® Inc. Neither
open license grants any right to use this name, its logos, or marks — see **Apache
License 2.0 §6**. Nominative references (e.g. "built on PurRDF") are permitted; uses
implying endorsement or origin are not.

## Contributions

Contributions to purrdf are accepted under **Apache-2.0 OR MIT** and, under the project
CLA, under terms that permit separate proprietary/commercial licensing. For the
dual-licensing reservation above to extend to contributed material, contributors agree to
license their contributions to Blackcat Informatics® Inc. under terms that permit
relicensing, including under proprietary terms. A Contributor License Agreement may be
required before substantial contributions are merged. See
[`CONTRIBUTING.md`](./CONTRIBUTING.md) for details.

## Copyright notice

> Copyright © 2026 Blackcat Informatics® Inc. All rights reserved, except as expressly
> granted under the licenses above.
