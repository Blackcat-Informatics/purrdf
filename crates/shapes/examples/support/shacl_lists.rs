// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The SHACL 1.2 list-component fixture every emitter oracle runs.
//!
//! One shapes graph using `sh:minListLength`, `sh:maxListLength`,
//! `sh:uniqueMembers` and `sh:memberShape`, compiled to JSON Schema; and data
//! variants, each validated by SHACL and projected to its JSON-LD node, so an
//! oracle's probes are real projected instances whose source verdict is SHACL
//! validation's (checked here to be the compiled schema's too).

use crate::holder::Fixture;

const PREFIXES: &str = r"
    @prefix sh:  <http://www.w3.org/ns/shacl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    @prefix ex:  <https://example.org/> .
";

/// Each list component on its own property; members of `ex:bounded` and
/// `ex:unique` are strings, `ex:members` holds its members to non-negative
/// integers.
const SHAPES: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:property [ sh:path ex:bounded ; sh:maxCount 1 ; sh:minListLength 2 ; sh:maxListLength 3 ] ;
        sh:property [ sh:path ex:unique ; sh:maxCount 1 ; sh:uniqueMembers true ] ;
        sh:property [ sh:path ex:members ; sh:maxCount 1 ;
                      sh:memberShape [ sh:datatype xsd:integer ; sh:minInclusive 0 ] ] .
";

/// The conforming data every variant replaces one property of.
const BASE: [(&str, &str); 3] = [
    ("ex:bounded", r#"( "a" "b" )"#),
    ("ex:unique", r#"( "a" "b" )"#),
    ("ex:members", "( 0 5 )"),
];

/// `(label, property, replacement value)`; the first row keeps the base.
pub(crate) const VARIANTS: [(&str, &str, &str); 8] = [
    ("conforming", "ex:bounded", r#"( "a" "b" )"#),
    ("bounded-at-maximum", "ex:bounded", r#"( "a" "b" "c" )"#),
    ("bounded-too-short", "ex:bounded", r#"( "a" )"#),
    ("bounded-too-long", "ex:bounded", r#"( "a" "b" "c" "d" )"#),
    ("unique-repeated", "ex:unique", r#"( "a" "a" )"#),
    ("member-negative", "ex:members", "( 0 -1 )"),
    ("member-not-integer", "ex:members", r#"( 0 "x" )"#),
    ("bounded-not-a-list", "ex:bounded", r#""a""#),
];

/// The fixture: its shapes graph, base values and variants.
pub(crate) const FIXTURE: Fixture = Fixture {
    prefixes: PREFIXES,
    shapes: SHAPES,
    base: &BASE,
    variants: &VARIANTS,
};
