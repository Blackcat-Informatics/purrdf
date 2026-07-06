// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-xsd` — the native XSD **value space** for the RDF 1.2 query stack.
//!
//! This is a pure-Rust, **zero-runtime-dependency**, wasm-clean leaf crate. It is
//! the drop-in replacement for the oxigraph-family `oxsdatatypes`, and the
//! foundation layer of the native SPARQL engine: the SPARQL evaluator evaluates
//! `FILTER`/`ORDER BY` over *typed values*, which this crate supplies. It is
//! deliberately decoupled from `purrdf-core` (no dependency in
//! either direction yet); the IR keeps literals **lexical-verbatim** (Constitution
//! C0.1) and this crate is the value layer that complements it.
//!
//! # Two distinct identities (load-bearing — do not conflate)
//!
//! A typed literal has TWO different notions of "equal", and mixing them silently
//! corrupts behavior:
//!
//! * **Term identity** — RDF `sameTerm`, which is the IR's `(lexical, datatype,
//!   language)` tuple, NOT this crate's value type. A consumer caches parsed values
//!   in a `HashMap<TermId, XsdValue>` keyed by that `TermId`, so `XsdValue` itself
//!   needs no `Eq`/`Hash`. `"1"^^xsd:integer` and `"01"^^xsd:integer` are **distinct**
//!   term identities (different lexical forms) even though they share one value.
//! * **Value-space identity** — SPARQL `=` / `<` over the *value* (the free fns
//!   `value_eq` / `value_cmp`). Here `"1"^^xsd:integer` and `"1.0"^^xsd:decimal` are
//!   **equal** (numeric promotion).
//!
//! `value_cmp` returns `Option<Ordering>`: `None` means the values are genuinely
//! **incomparable** (NaN, indeterminate-timezone dateTime, the two-component partial
//! order of `xsd:duration`, or non-comparable cross-types) — a spec-mandated outcome,
//! never a degraded fallback. `XsdValue` therefore implements neither `Eq`/`Hash`
//! (term identity is the IR's job, not this crate's) nor `PartialOrd`/`Ord` (that
//! would re-introduce the conflation for `BTreeMap`); ordering is the free fn.
//!
//! # XSD version: 1.1
//!
//! purrdf-xsd targets the **XSD 1.1** value spaces (W3C REC 2012-04-05).
//! Two load-bearing consequences for the year lexical:
//!
//! * Year `0000` is **permitted** (XSD 1.1; it denotes 1 BCE). XSD 1.0 forbade it.
//! * The year field must have **at least 4 digits**. A year field wider than 4 digits
//!   must **not** have a leading zero — e.g. `00044-03-15` and `012345-01-01` are
//!   invalid; `12345-06-15` and `-12345-06-15` are valid. Exactly 4 digits with a
//!   leading zero (`0044`, `0000`) are valid.
//!
//! # XSD 1.0 compatibility
//!
//! The crate stays honestly XSD 1.1 by default: [`parse`]/[`numeric::parse_float`]/
//! [`numeric::parse_double`] accept the XSD 1.1-only `+INF` spelling of positive
//! infinity for `xsd:float`/`xsd:double`. Spec-pinned consumers that reference XSD
//! 1.0 (e.g. ShEx/SHACL/SPARQL conformance suites written against the 1.0 lexical
//! space) call [`parse_xsd10`]/[`parse_float_xsd10`]/[`parse_double_xsd10`] instead,
//! which reject `+INF` (XSD 1.0 spells positive infinity only `INF`). This is an
//! opt-in function, not a Cargo feature — the kernel's default behavior never
//! changes underneath a caller that did not ask for it.
//!
//! # Datatype coverage
//!
//! purrdf-xsd models — and value-compares — a **superset** of `oxsdatatypes`:
//!
//! * numeric: `integer` (i128), the twelve derived-integer facets (`long`/`int`/
//!   `short`/`byte`, the `unsigned*` family, and `nonNegative`/`positive`/
//!   `nonPositive`/`negativeInteger`, each range-checked), `decimal` (i128 mantissa +
//!   scale ≤ 18), `float`, `double`;
//! * `boolean`, `string`;
//! * temporal: `dateTime`/`date`/`time`, `duration` + `dayTimeDuration`/
//!   `yearMonthDuration`, and the gregorian family `gYear`/`gMonth`/`gDay`/
//!   `gYearMonth`/`gMonthDay` (tz-indeterminate partial order);
//! * binary: `hexBinary`/`base64Binary` (hand-rolled codecs — still zero-dep).
//!
//! The derived-integer facets and the binary types are **not** modelled by
//! `oxsdatatypes`; the gregorian family matches it. Integer and decimal are
//! `i128`-bounded (decimal scale ≤ 18); lexicals beyond that domain hard-fail on
//! range rather than promoting to arbitrary precision.
//!
//! # Hard-fail
//!
//! Malformed lexical input is a hard error ([`XsdError`]), never a silent default.
//! Out-of-range integer/decimal lexicals fail rather than saturate (this crate is
//! `i128`-bounded — already exceeding `oxsdatatypes`' `i64`).
//!
//! # Examples
//!
//! Parse lexical forms into the value space and compare across the numeric tower:
//!
//! ```rust
//! use std::cmp::Ordering;
//!
//! use purrdf_xsd::{XsdDatatype, parse, value_cmp, value_eq};
//!
//! // Parse maps a lexical form to the value it denotes.
//! let int = parse("42", XsdDatatype::Integer)?;
//! assert_eq!(int.canonical_lexical(), "42");
//!
//! // SPARQL numeric promotion: `"42"^^xsd:integer = "42.0"^^xsd:decimal`.
//! let dec = parse("42.0", XsdDatatype::Decimal)?;
//! assert!(value_eq(&int, &dec));
//!
//! // Ordering works across numeric types too.
//! let dbl = parse("2.5e1", XsdDatatype::Double)?;
//! assert_eq!(value_cmp(&dbl, &int), Some(Ordering::Less));
//!
//! // Different value-space families are INCOMPARABLE (`None`), not "not equal".
//! let s = parse("42", XsdDatatype::String)?;
//! assert_eq!(value_cmp(&int, &s), None);
//!
//! // Malformed or out-of-range lexicals hard-fail — never a silent default.
//! assert!(parse("4.2", XsdDatatype::Integer).is_err());
//! assert!(parse("300", XsdDatatype::Byte).is_err());
//! # Ok::<(), purrdf_xsd::XsdError>(())
//! ```
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]

pub mod binary;
pub mod datatype;
pub mod numeric;
pub mod ops;
pub mod simple;
pub mod temporal;
pub mod value;

pub use binary::{canonical_base64, canonical_hex, parse_base64, parse_binary, parse_hex};
pub use datatype::{XSD_NS, XsdDatatype};
pub use numeric::{
    Decimal, numeric_abs, numeric_add, numeric_ceil, numeric_div, numeric_floor, numeric_mul,
    numeric_round, numeric_sub, numeric_unary_minus, numeric_unary_plus, parse_double_xsd10,
    parse_float_xsd10,
};
pub use ops::{effective_boolean_value, value_cmp, value_eq};
pub use simple::{normalize_whitespace_collapse, normalize_whitespace_replace};
pub use temporal::{datetime_epoch, datetime_from_unix_seconds};
pub use value::{XsdError, XsdValue, parse, parse_by_iri, parse_xsd10};
