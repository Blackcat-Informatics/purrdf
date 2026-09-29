<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-iri` — Native IRI/URI Value Space

[![crates.io](https://img.shields.io/crates/v/purrdf-iri.svg)](https://crates.io/crates/purrdf-iri)
[![docs.rs](https://docs.rs/purrdf-iri/badge.svg)](https://docs.rs/purrdf-iri)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-iri` is the IRI/URI foundation of the PurRDF toolkit: a pure-Rust,
wasm-clean crate with no third-party dependency — its runtime dependencies are
the lexical layer [`purrdf-lex`](https://crates.io/crates/purrdf-lex) and the
zero-dependency [`purrdf-hash`](https://crates.io/crates/purrdf-hash) —
implementing RFC 3987/3986
parsing, validation, reference resolution, syntax normalization, and
CURIE/prefix handling.

- **Parse + validate** — RFC 3987 IRIs (`parse`) and the strict-ASCII RFC 3986
  URI subset (`parse_uri`), with component spans (scheme/authority/path/query/
  fragment) exposed without re-encoding.
- **Reference resolution** — RFC 3986 §5 strict resolution (`Iri::resolve`).
- **Syntax normalization** — RFC 3986 §6.2.2 (`Iri::normalize`): case,
  percent-encoding, and dot-segment normalization; idempotent.
- **CURIE/prefix** — `expand_curie` / `resolve` / `contract` over a
  `PrefixMap`.
- **IDNA2008** — `idna`: RFC 5891 host-name validation over the RFC 5892
  derived property, contextual rules and the RFC 5893 Bidi rule, RFC 3492
  Punycode, and `Iri::to_uri` (RFC 3987 §3.1). Tables are generated from the
  vendored Unicode 17.0.0 database.
- **Grammar terminals, byte-class scanners and the JSON string escape law** —
  `terminals`, `scan` and `json_escape`, re-exported from `purrdf-lex`, where
  every grammar in the workspace shares them.
- **Hard-fail** — malformed input is a typed `IriError`, never a degraded
  fallback or silent default.

## Usage

```sh
cargo add purrdf-iri
```

```rust
use purrdf_iri::{parse, PrefixMap, expand_curie};

// Parse, resolve a relative reference, normalize.
let base = parse("https://example.org/a/b/c").expect("valid IRI");
let joined = base.resolve("../d?x=1").expect("resolvable");
assert_eq!(joined.as_str(), "https://example.org/a/d?x=1");

// CURIEs over a caller-supplied prefix map (PurRDF mints no vocabulary IRIs).
let mut prefixes = PrefixMap::new();
prefixes.insert("ex", "https://example.org/");
assert_eq!(
    expand_curie("ex:cat", &prefixes).as_deref(),
    Some("https://example.org/cat"),
);
```

The `Option`-returning surfaces are CURIE expansion, where `None` is a
*semantic* "not a CURIE / undeclared prefix" signal, and the IDNA conversions,
where `None` is the protocol's own "not a valid name".

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C (the GTS container itself reaches Python and C, not the wasm package). Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
as `purrdf::iri`; depend on `purrdf-iri` directly when you just want a small
IRI library with no third-party dependency.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.98, stable toolchain only).

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)

The IDNA tables compiled into this crate are derived from the Unicode Character
Database, so the crate's licence expression is
`(MIT OR Apache-2.0 OR MulanPSL-2.0) AND Unicode-3.0`: the Unicode-3.0 terms
([`LICENSES/Unicode-3.0.txt`](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSES/Unicode-3.0.txt))
apply alongside whichever of the three you choose. See
[`LICENSING.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSING.md).
