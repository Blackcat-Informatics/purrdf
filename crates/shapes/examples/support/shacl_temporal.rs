// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The temporal range-bound fixture every emitter oracle runs.
//!
//! One shapes graph bounding an `xsd:date`, an `xsd:dateTime` and an
//! `xsd:time` value, compiled to JSON Schema; and data variants, each
//! validated by SHACL and projected to its JSON-LD node, so an oracle's probes
//! are real projected instances whose source verdict is SHACL validation's.
//! The variants cross the XSD timeline's edges: a timezone against a local
//! bound and the reverse (comparable only beyond ±14:00), `24:00:00`, a leap
//! day, and values of another datatype.

use crate::holder::Fixture;

const PREFIXES: &str = r"
    @prefix sh:  <http://www.w3.org/ns/shacl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    @prefix ex:  <https://example.org/> .
";

/// A local lower and a zoned upper date bound, a zoned exclusive instant
/// bound, and a local upper time bound.
const SHAPES: &str = r#"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:property [ sh:path ex:day ; sh:maxCount 1 ;
                      sh:minInclusive "2020-03-01"^^xsd:date ;
                      sh:maxExclusive "2021-01-01Z"^^xsd:date ] ;
        sh:property [ sh:path ex:at ; sh:maxCount 1 ;
                      sh:minExclusive "2020-03-01T12:00:00Z"^^xsd:dateTime ] ;
        sh:property [ sh:path ex:clock ; sh:maxCount 1 ;
                      sh:maxInclusive "12:00:00"^^xsd:time ] .
"#;

/// The conforming data every variant replaces one property of.
const BASE: [(&str, &str); 3] = [
    ("ex:day", r#""2020-06-01"^^xsd:date"#),
    ("ex:at", r#""2020-03-01T12:00:01Z"^^xsd:dateTime"#),
    ("ex:clock", r#""11:00:00"^^xsd:time"#),
];

/// `(label, property, replacement value)`; the first row keeps the base.
pub(crate) const VARIANTS: [(&str, &str, &str); 12] = [
    ("conforming", "ex:day", r#""2020-06-01"^^xsd:date"#),
    (
        "day-zoned-inside",
        "ex:day",
        r#""2020-12-31+14:00"^^xsd:date"#,
    ),
    ("day-too-early", "ex:day", r#""2020-02-29"^^xsd:date"#),
    (
        "day-zone-incomparable",
        "ex:day",
        r#""2020-03-01Z"^^xsd:date"#,
    ),
    ("day-not-a-leap-day", "ex:day", r#""2021-02-29"^^xsd:date"#),
    (
        "at-equal",
        "ex:at",
        r#""2020-03-01T12:00:00Z"^^xsd:dateTime"#,
    ),
    (
        "at-offset-later",
        "ex:at",
        r#""2020-03-01T17:30:01+05:30"^^xsd:dateTime"#,
    ),
    (
        "at-local-in-window",
        "ex:at",
        r#""2020-03-02T02:00:00"^^xsd:dateTime"#,
    ),
    (
        "at-end-of-day",
        "ex:at",
        r#""2020-03-01T24:00:00Z"^^xsd:dateTime"#,
    ),
    ("clock-late", "ex:clock", r#""12:00:00.5"^^xsd:time"#),
    (
        "clock-zoned-early",
        "ex:clock",
        r#""01:59:59+14:00"^^xsd:time"#,
    ),
    ("clock-a-string", "ex:clock", r#""noon""#),
];

/// The fixture: its shapes graph, base values and variants.
pub(crate) const FIXTURE: Fixture = Fixture {
    prefixes: PREFIXES,
    shapes: SHAPES,
    base: &BASE,
    variants: &VARIANTS,
};
