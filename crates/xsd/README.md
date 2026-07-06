<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-xsd` — Zero-Dependency XSD 1.1 Value Space

[![crates.io](https://img.shields.io/crates/v/purrdf-xsd.svg)](https://crates.io/crates/purrdf-xsd)
[![docs.rs](https://docs.rs/purrdf-xsd/badge.svg)](https://docs.rs/purrdf-xsd)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-xsd` is the typed-value foundation of the PurRDF toolkit: a pure-Rust,
**zero-runtime-dependency**, wasm-clean crate implementing the **XSD 1.1**
value spaces — lexical parsing, value equality and ordering, canonical lexical
forms, and SPARQL numeric promotion. It is the layer the SPARQL evaluator uses
to compute `FILTER` / `ORDER BY` over typed values, while the RDF IR keeps
literals lexical-verbatim beside it.

Coverage:

- **Numeric** — `integer` (i128), the twelve derived-integer facets
  (range-checked), `decimal`, `float`, `double`, with SPARQL numeric promotion.
- **Temporal** — `dateTime`/`date`/`time`, `duration` plus
  `dayTimeDuration`/`yearMonthDuration`, and the gregorian family, honoring the
  spec's timezone-indeterminate partial order.
- **Binary** — `hexBinary`/`base64Binary` with hand-rolled (still zero-dep)
  codecs.
- **`boolean`, `string`**, effective boolean value, and whitespace facets.

One deliberate design point: the crate keeps **term identity** (RDF `sameTerm`,
the IR's job) and **value-space identity** (SPARQL `=`/`<`) strictly apart.
`XsdValue` implements neither `Eq`/`Hash` nor `Ord`; comparison is the free
functions `value_eq` / `value_cmp`, and `value_cmp` returning `None` means the
values are genuinely incomparable per spec (NaN, indeterminate timezones,
partial-order durations) — never a degraded fallback.

## Usage

```sh
cargo add purrdf-xsd
```

```rust
use purrdf_xsd::{parse, value_cmp, value_eq, XsdDatatype};

let a = parse("1", XsdDatatype::Integer).expect("valid integer");
let b = parse("1.0", XsdDatatype::Decimal).expect("valid decimal");

// SPARQL value equality with numeric promotion: "1"^^xsd:integer = "1.0"^^xsd:decimal.
assert!(value_eq(&a, &b));

let c = parse("2.5", XsdDatatype::Decimal).expect("valid decimal");
assert_eq!(value_cmp(&a, &c), Some(core::cmp::Ordering::Less));
```

Spec-pinned consumers that need the XSD 1.0 lexical space (e.g. conformance
suites written against 1.0) call the explicit `parse_xsd10` family — as an
opt-in function, not a feature flag, so default behavior never changes
underneath a caller.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
as `purrdf::xsd`; depend on `purrdf-xsd` directly when you just want a small,
dependency-free XSD value library.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
