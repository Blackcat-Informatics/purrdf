<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# `purrdf-shapes` — Rust SHACL Core Validator

[![crates.io](https://img.shields.io/crates/v/purrdf-shapes.svg)](https://crates.io/crates/purrdf-shapes)
[![docs.rs](https://docs.rs/purrdf-shapes/badge.svg)](https://docs.rs/purrdf-shapes)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

> **An LLM output is a claim, not a truth.**

`purrdf-shapes` is the native SHACL validator of the PurRDF toolkit — the
complete SHACL Core feature set plus SHACL-SPARQL constraints and targets,
running entirely on PurRDF's own interned IR and native SPARQL engine (no
oxigraph, no PyO3). It validates an RDF 1.2 data graph against a SHACL shapes
graph with no inference (parity with pySHACL `inference="none"`).

In four-box terms, the data graph is usually the ABox, the shapes graph is a
TBox/RBox validation surface, and RDF 1.2 reifier metadata is the CBox. The
crate preserves existing report keys while adding optional box-role metadata for
callers that want richer diagnostics.

The crate implements a scoped SHACL 1.2 Working Draft feature:
`sh:reifierShape` and `sh:reificationRequired` for direct IRI property paths.
The relevant SHACL 1.2 Core draft is dated 2026-06-02. This is not a claim of
full SHACL 1.2 conformance.

The Python SHACL surface is exposed from `bindings/python` as part of the
`purrdf_native` extension. The engine core (`engine.rs`, `shapes.rs`,
`constraints.rs`, `path.rs`, `report.rs`, `model.rs`) is deliberately
**PyO3-free** — it links as a plain `rlib` into any Rust consumer without
any Python dependency.

This crate is gated by a SHACL conformance corpus.

---

## Build

> **Toolchain:** stable Rust (the repo ships a `rust-toolchain.toml` at the
> root pinning `stable`; the MSRV floor is `rust-version` in the workspace
> `Cargo.toml`). `cargo` and `rustup` pick this up automatically.

```bash
cargo build -p purrdf-shapes
```

## Test

```bash
cargo test -p purrdf-shapes
```

---

## Python extension

```bash
cd ../../bindings/python
maturin develop
```

```python
from purrdf_native import shacl

report = shacl.validate(shapes_ttl="...", data_nt="...")
print(report["conforms"])  # True / False
print(report["results"])   # list of violation dicts
```

Each result dict keeps the stable keys `focus`, `path`, `value`, `severity`,
`component`, `source_shape`, and `message`. When the shapes or path terms carry
`purrdf:graphBoxRole`, result dicts may also include `source_box_roles`,
`path_box_roles`, and `result_box_roles`.

---

## Project and community

`purrdf-shapes` is developed by [Blackcat Informatics® Inc.](https://blackcatinformatics.ca)
as one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
RDF 1.2 toolkit. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
as `purrdf::shapes`.

Related crates:

- [`purrdf-sparql-eval`](https://crates.io/crates/purrdf-sparql-eval) — the native
  SPARQL engine that powers SHACL-SPARQL constraints and targets
- [`purrdf-validate`](https://crates.io/crates/purrdf-validate) — renders
  validation reports as byte-deterministic SARIF 2.1.0
- [`purrdf-gts`](https://crates.io/crates/purrdf-gts) — the GTS graph-transport
  container engine

---

## License and copyright

Copyright © 2026 Blackcat Informatics® Inc.

This crate is licensed under **MIT OR Apache-2.0** — see
[`LICENSE-MIT`](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
and
[`LICENSE-APACHE`](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
in the repository root. Separate proprietary/commercial terms are available;
contact `licensing@blackcatinformatics.ca`.
