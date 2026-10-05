// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-xsd` — the native XSD **value space** for the RDF 1.2 query stack.
//!
//! This is a pure-Rust, **zero-runtime-dependency**, wasm-clean leaf crate. It is
//! the foundation layer of the native SPARQL engine: the SPARQL evaluator evaluates
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
//! # `<` is not the sort order
//!
//! [`value_cmp`] means the SPARQL `<` / `=` operators, and §17.3 defines those over
//! the numeric PROMOTION lattice — which sends any comparison involving an
//! `xsd:float`/`xsd:double` through IEEE, losing precision, while comparing an
//! integer against a decimal exactly. Mixing an exact sub-relation with a lossy one
//! is not transitive, and a Rust sort handed an intransitive comparator may panic.
//! [`value_total_cmp`] is therefore the relation a SORT uses: identical to
//! [`value_cmp`] everywhere except the numeric tower, where [`numeric_total_cmp`]
//! compares the exact rationals instead. The divergence, and the three-literal cycle
//! that forces it, are spelled out on [`numeric_total_cmp`].
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
//! purrdf-xsd models — and value-compares — these datatypes:
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
//! Integer and decimal arithmetic is bounded; the bounds are the conformance
//! contract below.
//!
//! # Numeric limits (the conformance contract)
//!
//! XSD 1.1 Part 2 lets a processor limit the `xsd:integer` and `xsd:decimal` value
//! spaces, provided it supports at least sixteen decimal digits (§5.4, *partial
//! implementation of infinite datatypes*) and documents its limits. These are this
//! crate's:
//!
//! * **`xsd:integer`** and every integer-family datatype compute in `i128`:
//!   `−2^127 ..= 2^127 − 1`, thirty-eight full decimal digits.
//! * **`xsd:decimal`** computes as an `i128` mantissa with at most **18** fractional
//!   digits: magnitudes below `2^127`, and thirty-eight significant digits whenever no
//!   more than eighteen of them follow the point.
//!
//! Both exceed XSD's sixteen-digit minimum. Inside them every operation is exact,
//! except the roundings XPath and XQuery Functions and Operators 3.1 explicitly
//! permits:
//!
//! * an `xs:decimal` result with more digits than the representation retains is
//!   **truncated toward zero at the finest scale ≤ 18 whose mantissa fits** — the
//!   implementation-defined precision of F&O §4.2 for `+`, `−`, `×` and `÷`
//!   ([`numeric_add`], [`numeric_sub`], [`numeric_mul`], [`numeric_div`]) and for a
//!   mean ([`decimal_mean`]);
//! * a cast or promotion to `xsd:float`/`xsd:double` rounds to nearest, ties to even,
//!   once (F&O §19.1.2.2, XSD `floatingPointRound`), from an exact value of any size
//!   ([`Decimal::to_f64`], [`DecimalDigits::to_f64`]);
//! * a cast from `xsd:float`/`xsd:double` to `xsd:decimal` takes the closest
//!   representable decimal (F&O §19.1.2.3, [`Decimal::from_f64_closest`]).
//!
//! Every other limit is a **typed error**, never a silently wrapped, rounded or
//! saturated value — [`XsdError::code`] names its F&O code:
//!
//! * an arithmetic result whose integer part exceeds the bounds is
//!   `err:FOAR0002` ([`ErrorCode::Foar0002`]);
//! * an `xsd:integer` lexical form past `i128` is `err:FOCA0003`, an `xsd:decimal`
//!   one past the mantissa `err:FOCA0001`, and one with more than eighteen
//!   significant fractional digits `err:FOCA0006` ([`parse`]); trailing fractional
//!   zeros are spelling, not precision, and never refuse a value the representation
//!   holds.
//!
//! Every error's message ends with its code, and [`XsdError::presentation`] gives it
//! typed.
//!
//! **A literal past the bounds is still a value.** Its lexical form round-trips
//! untouched (the IR keeps literals lexical-verbatim), ordering, value equality and
//! the conversions to `xsd:float`/`xsd:double` are exact for an
//! `xsd:integer`/`xsd:decimal` literal of any length and scale ([`DecimalDigits`],
//! [`LiteralValue`], [`literal_cmp`], [`literal_equal`], [`literal_total_cmp`]), and
//! arithmetic over it is computed exactly and answers whenever the bounded value space
//! holds the result ([`literal_add`], [`literal_sub`], [`literal_mul`],
//! [`literal_div`] and the unary functions), `err:FOAR0002` otherwise.
//!
//! [`bigint::BigInt`] is not a value space either: it exists so a caller
//! ACCUMULATING many bounded values (SPARQL `SUM`/`AVG` over a group) can keep the
//! exact running total even where the total would overflow the bounds — see its
//! module docs for why a running total can need one when no individual value ever
//! does.
//!
//! # Datatype-range satisfiability
//!
//! [`range`] answers a question the value spaces alone do not: **is this datatype range
//! empty?** Given an OWL 2 data range over these datatypes — constraining facets,
//! enumerations, complement, intersection, union — [`satisfiability`] reports `Empty` or
//! `Inhabited` as a *proof*, [`contains`] tests one value against the range, and
//! [`cardinality`] bounds how many distinct values it holds. Each answer is
//! three-valued: what the module cannot prove is `Undecided`, never guessed, because a
//! consuming reasoner reads `Empty` as an inconsistent ontology.
//!
//! The residue that answers `Undecided`: an opaque range (an unmodelled datatype, an
//! `xsd:pattern` or `rdf:langRange` facet, an n-ary range); a bound facet over a
//! temporal space that neither contradicts another bound nor exhibits an endpoint, and
//! the complement of any temporal bound restriction (the XSD order there is partial);
//! `xsd:dayTimeDuration`/`xsd:yearMonthDuration` as whole datatypes; an enumerated
//! `xsd:float`/`xsd:double` zero; and a facet inapplicable to its base's value space.
//! [`is_exactly_decided`] reports exactly which ranges are free of that residue.
//!
//! [`same_value`] is the third identity this crate names: XSD/OWL 2 value-space
//! identity, which — unlike [`value_eq`] — holds `"NaN"^^xsd:double` identical to
//! itself and keeps the `xsd:float`, `xsd:double` and `xsd:decimal` value spaces
//! disjoint.
//!
//! # Binary floating-point arithmetic
//!
//! [`ieee`] is the workspace's one implementation of binary64 and binary32 `+`, `−`,
//! `×`, `÷` and `√`, each correctly rounded on every target -- the x87 included, where
//! a precision guard and a subnormal scaling make the unit round once. The
//! `xsd:double`/`xsd:float` operators here compute with it, and so does every other
//! crate whose floating-point results reach a query answer, serialized bytes or an
//! identity. Everywhere but the x87 each operation is the bare operator.
//!
//! # Hard-fail
//!
//! Malformed lexical input is a hard error ([`XsdError`]), never a silent default.
//! Out-of-range integer/decimal lexicals fail rather than saturate; see *Numeric
//! limits* above for the typed error each limit raises.
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
// `deny`, not `forbid`: the one exception is `ieee::x87`, the x87 control word and the
// correctly rounded x87 sequences, which are inline assembly and exist only on a 32-bit
// `x86` build without SSE2. Every other module is unsafe-free.
#![deny(unsafe_code)]

pub mod bigint;
pub mod binary;
pub mod datatype;
pub mod decimal_digits;
mod decimal_float;
pub mod ieee;
pub mod json_number;
pub mod numeric;
pub mod ops;
pub mod range;
pub mod rational;
pub mod rfc3339;
pub mod simple;
pub mod temporal;
pub mod value;
pub mod wide;

pub use bigint::BigInt;
pub use binary::{canonical_base64, canonical_hex, parse_base64, parse_binary, parse_hex};
pub use datatype::{XSD_NS, XsdDatatype};
pub use decimal_digits::{
    DecimalDigits, LiteralValue, literal_abs, literal_add, literal_ceil, literal_cmp, literal_div,
    literal_equal, literal_floor, literal_mul, literal_round, literal_sub, literal_total_cmp,
    literal_unary_minus, literal_unary_plus,
};
#[allow(deprecated)]
pub use numeric::bigint_avg_decimal_lexical;
pub use numeric::{
    Decimal, bigint_avg_decimal, decimal_mean, numeric_abs, numeric_add, numeric_ceil, numeric_cmp,
    numeric_div, numeric_floor, numeric_mul, numeric_round, numeric_sub, numeric_total_cmp,
    numeric_unary_minus, numeric_unary_plus, parse_double_xsd10, parse_float_xsd10,
};
pub use ops::{
    effective_boolean_value, value_add, value_cmp, value_div, value_eq, value_equal, value_mul,
    value_sub, value_total_cmp, value_unary_minus,
};
pub use range::{
    Cardinality, DataRange, Facet, Known, Satisfiability, cardinality, contains,
    is_exactly_decided, same_value, satisfiability,
};
pub use simple::{normalize_whitespace_collapse, normalize_whitespace_replace};
pub use temporal::{
    civil_from_days, datetime_epoch, datetime_from_unix_seconds, days_from_civil, days_in_month,
    duration_equal, is_leap,
};
pub use value::{ErrorCode, XsdError, XsdValue, parse, parse_by_iri, parse_xsd10};
