<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Licensing

Blackcat Informatics® Inc. is the sole copyright holder of PurRDF (© 2026).
Two distinct things are true about its licensing, and this document keeps
them distinct:

1. All first-party code is offered under **three open-source licenses, at
   your option**: MIT, Apache-2.0, or MulanPSL-2.0.
2. Separately, Blackcat Informatics® reserves the right, as copyright
   holder, to grant commercial/proprietary licenses outside those terms.

## Open-source terms

All first-party material in this repository — the Rust workspace, the Python,
WebAssembly, and C bindings, first-party test fixtures and harnesses,
documentation, and build tooling — is offered under your choice of any one of:

| License | Text |
|---|---|
| **MIT** | [`LICENSE-MIT`](./LICENSE-MIT) |
| **Apache License 2.0** | [`LICENSE-APACHE`](./LICENSE-APACHE) |
| **Mulan Permissive Software License, Version 2 (MulanPSL-2.0)** | [`LICENSE-MULAN`](./LICENSE-MULAN) |

This is expressed in every first-party source file and package manifest with
the SPDX identifier:

```text
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
```

Some documentation files instead carry `SPDX-License-Identifier: CC-BY-4.0`;
those are still first-party Blackcat Informatics® material, licensed under
[Creative Commons Attribution 4.0](https://creativecommons.org/licenses/by/4.0/).

### About MulanPSL-2.0, precisely

MulanPSL-2.0 is an [OSI-approved](https://opensource.org/license/MulanPSL-2.0)
permissive license published bilingually in Chinese and English. Its terms an
adopter should know exactly:

- **The Chinese text governs** (its §6): in any divergence between the
  Chinese and English versions, the Chinese version prevails. The committed
  [`LICENSE-MULAN`](./LICENSE-MULAN) carries both languages.
- It grants a **patent license** per contributor that **terminates** if you
  initiate patent litigation over the software (§2).
- It grants **no trademark rights** (§3), and requires recipients to receive
  a copy of the license and to retain copyright, patent, trademark, and
  disclaimer statements (§4).
- Unlike Apache-2.0, it does **not** require stating changes to modified
  files.
- Choosing MulanPSL-2.0 does not alter the third-party carve-outs below:
  vendored material keeps its own upstream terms under every disjunct.

The committed text is byte-pinned (SHA-256
`eb7a1d713eb919b146787629e22e4c975cb701f529a65d4d7e0fcd417558bf1c`, from the
SPDX license-list corpus, matching the canonical text at
`license.coscl.org.cn/MulanPSL2`) and re-verified by `scripts/check-licenses.py`.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work shall be licensed as above
(MIT OR Apache-2.0 OR MulanPSL-2.0), without any additional terms or
conditions.

## Third-party material (carve-out)

**The terms above do not apply to the vendored third-party conformance
corpora.** Those files are *not* Blackcat Informatics® copyright, are
vendored verbatim, are **not** relicensed, and are not redistributable under
MIT, Apache-2.0, or MulanPSL-2.0.

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

The first-party selectors, harnesses, and reconstructed expected-result files
that sit *inside* those trees carry the repository's own
`MIT OR Apache-2.0 OR MulanPSL-2.0` grant. Each tree's own `LICENSE`,
`LICENSE.md`, `LICENSES/`, `PROVENANCE.md`, or `README.md` is authoritative
for its upstream terms, source URL, and pinned revision; the table above is a
summary, not a substitute.

Two gates keep this honest, and each covers a stated subset rather than the
whole repository:

- `scripts/check-licenses.py` treats any directory under `crates/` or
  `bindings/` that holds a `LICENSES/` subdirectory as a vendored root, and
  fails the build if a file beneath one lacks a `.license` SPDX sidecar, an
  inline SPDX header, or a `REUSE.toml` annotation. It also re-verifies the
  committed MulanPSL-2.0 text against its pinned digest. Today that covers
  the four W3C SPARQL/OWL 2 suites and the OBO Graphs schema closure.
- `scripts/check-corpus-frozen.py` SHA-256-verifies `vectors/shacl`,
  `vectors/shexTest`, `crates/shapes/corpus`,
  `crates/sparql-conformance/entailment-suite/w3c-owl2` and
  `crates/sparql-conformance/entailment-suite/w3c-owl2-rl` against committed
  freeze manifests, so vendored bytes cannot be edited in place.

## Proprietary / commercial licensing

The open licenses above are offered **in addition to — not in place of** —
Blackcat Informatics®' right, as copyright holder, to license the software
under separate commercial or proprietary terms. Granting the open licenses
does not revoke or limit this reservation.

To obtain a proprietary license, contact **licensing@blackcatinformatics.ca**.

## Trademarks

"Blackcat Informatics®" is a registered trademark of Blackcat Informatics®
Inc. None of the open licenses grants any right to use this name, its logos,
or marks — see **Apache License 2.0 §6** and **MulanPSL-2.0 §3**. Nominative
references (e.g. "built on PurRDF") are permitted; uses implying endorsement
or origin are not.

## Contributions

Contributions to purrdf are accepted under **MIT OR Apache-2.0 OR
MulanPSL-2.0** and, under the project CLA, under terms that permit separate
proprietary/commercial licensing. For the commercial reservation above to
extend to contributed material, contributors agree to license their
contributions to Blackcat Informatics® Inc. under terms that permit
relicensing, including under proprietary terms. A Contributor License
Agreement may be required before substantial contributions are merged. See
[`CONTRIBUTING.md`](./CONTRIBUTING.md) for details.

## Copyright notice

> Copyright © 2026 Blackcat Informatics® Inc. All rights reserved, except as
> expressly granted under the licenses above.
