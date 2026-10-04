// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL Core §4.1.1, `sh:datatype`: "A literal value node that is ill-formed for
//! its datatype does not conform, even when its datatype IRI matches." Every XSD
//! datatype whose lexical space this workspace models is held to it — the
//! temporal family, the Gregorian fragments, the durations, the binary types and
//! `xsd:dateTimeStamp` — and every refusal is paired with neighbouring valid
//! lexical forms of the same datatype that must still conform, so the check is
//! proven in both directions. Datatypes whose lexical space is not modelled
//! (custom IRIs, `rdf:HTML`, `xsd:anyURI`, ...) keep conforming on an exact IRI
//! match.

use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "@prefix ex: <http://example.org/ns#> .\n\
    @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
    @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n";

const DATATYPE_COMPONENT: &str = "http://www.w3.org/ns/shacl#DatatypeConstraintComponent";

/// The number of `sh:DatatypeConstraintComponent` results for one value `object`
/// checked against `sh:datatype datatype`.
fn datatype_violations(datatype: &str, object: &str) -> usize {
    let shapes = format!(
        "{PREFIXES}ex:S a sh:NodeShape ; sh:targetNode ex:x ;\n\
         sh:property [ sh:path ex:p ; sh:datatype {datatype} ] .\n"
    );
    let shapes = parse_shapes(&shapes, None).expect("shapes parse");
    let data = parse_turtle_to_dataset(&format!("{PREFIXES}ex:x ex:p {object} .\n"), None)
        .expect("data parses");
    let report = validate_dataset_with_shapes_graph(&data, &shapes, None).expect("validates");
    let violations = report
        .results
        .iter()
        .filter(|result| result.source_constraint_component.as_str() == DATATYPE_COMPONENT)
        .count();
    assert_eq!(
        violations,
        report.results.len(),
        "only the datatype constraint can fire for {object} against {datatype}"
    );
    violations
}

/// `"lexical"^^datatype`, escaped for a Turtle short string.
fn typed(lexical: &str, datatype: &str) -> String {
    let escaped = lexical
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"^^{datatype}")
}

/// Every `invalid` lexical form of `datatype` violates, and every `valid` one
/// conforms.
fn check(datatype: &str, invalid: &[&str], valid: &[&str]) {
    let mut wrong = Vec::new();
    for lexical in invalid {
        if datatype_violations(datatype, &typed(lexical, datatype)) != 1 {
            wrong.push(format!(
                "{lexical:?}^^{datatype} conformed but is ill-formed"
            ));
        }
    }
    for lexical in valid {
        if datatype_violations(datatype, &typed(lexical, datatype)) != 0 {
            wrong.push(format!(
                "{lexical:?}^^{datatype} violated but is well-formed"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn ill_formed_date_times_violate_and_well_formed_ones_conform() {
    check(
        "xsd:dateTime",
        &[
            "notadate",
            "x\u{e9}12345",
            "2001-01-01",
            "2001-13-01T00:00:00Z",
            "2001-02-30T00:00:00Z",
            "2001-01-01T25:00:00",
            "2001-01-01T12:60:00",
            "2001-01-01 00:00:00",
            "2001-01-01T00:00:00+0\u{e9}:00",
            "2001-01-01T00:00:00+15:00",
            "",
        ],
        &[
            "2001-01-01T00:00:00Z",
            "2001-01-01T00:00:00",
            "2001-01-01T00:00:00.5-05:00",
            "2001-01-01T24:00:00",
            "2000-02-29T12:00:00+14:00",
            "-0044-03-15T12:00:00",
            " 2001-01-01T00:00:00Z ",
            // Past `i64` years and past the decimal's fractional scale, but still
            // in the lexical space: a range limit is not ill-formedness.
            "123456789012345678901-01-01T00:00:00Z",
            "2001-01-01T12:00:00.1234567890123456789012345Z",
        ],
    );
}

#[test]
fn ill_formed_dates_violate_and_well_formed_ones_conform() {
    check(
        "xsd:date",
        &[
            "notadate",
            "x\u{e9}12345",
            "2001-02-30",
            "2001-1-01",
            "01-01-01",
            "2001-01-01T00:00:00",
            "2001-01-01+0\u{e9}:00",
            "2001-01-01Z\u{e9}",
        ],
        &[
            "2001-01-01",
            "2001-01-01+05:00",
            "2001-01-01Z",
            "2000-02-29",
            "-0044-03-15",
            "12345-01-01",
            " 2001-01-01\n",
        ],
    );
}

#[test]
fn ill_formed_times_violate_and_well_formed_ones_conform() {
    check(
        "xsd:time",
        &["noon", "12:00", "25:00:00", "12:00:61", "12:00:00+\u{e9}"],
        &["12:00:00", "12:00:00.125Z", "23:59:59-08:00", "24:00:00"],
    );
}

#[test]
fn a_date_time_stamp_requires_a_timezone() {
    check(
        "xsd:dateTimeStamp",
        &[
            "2001-01-01T00:00:00",
            "notadate",
            "2001-01-01",
            "2001-01-01T00:00:00+\u{e9}",
            // Past the representable range, the timezone is still required.
            "123456789012345678901-01-01T00:00:00",
            "2001-01-01T00:00:00.1234567890123456789012345",
        ],
        &[
            "2001-01-01T00:00:00Z",
            "2001-01-01T00:00:00.5+05:30",
            " 2001-01-01T00:00:00-01:00 ",
            "123456789012345678901-01-01T00:00:00Z",
            "2001-01-01T00:00:00.1234567890123456789012345+05:00",
        ],
    );
}

#[test]
fn ill_formed_gregorian_fragments_violate_and_well_formed_ones_conform() {
    check(
        "xsd:gYear",
        &["01", "abcd", "2001-01", "x\u{e9}12"],
        &["2001", "-0001", "12345", "2001Z"],
    );
    check(
        "xsd:gYearMonth",
        &["2001", "2001-13", "2001-1", "notadate"],
        &["2001-12", "2001-01+01:00"],
    );
    check(
        "xsd:gMonth",
        &["13", "--13", "--1", "-12"],
        &["--12", "--01Z"],
    );
    check(
        "xsd:gMonthDay",
        &["--02-30", "--13-01", "02-01", "--2-01"],
        &["--02-29", "--12-31-05:00"],
    );
    check(
        "xsd:gDay",
        &["32", "---32", "--01", "---1"],
        &["---31", "---01Z"],
    );
}

#[test]
fn ill_formed_durations_violate_and_well_formed_ones_conform() {
    check(
        "xsd:duration",
        &[
            "P",
            "PT",
            "1D",
            "P1H",
            "P-1D",
            "notaduration",
            "P1YT",
            "P1D1Y",
            "PT1.5M",
        ],
        &[
            "P1D",
            "-P1Y2M3DT4H5M6.7S",
            "PT0S",
            "P1Y",
            "PT1M",
            "PT1.S",
            "P99999999999999999999Y",
        ],
    );
    check(
        "xsd:dayTimeDuration",
        &["P1Y", "P1M", "P"],
        &["P1DT2H", "PT1M", "-P3D"],
    );
    check(
        "xsd:yearMonthDuration",
        &["P1D", "PT1H", "P"],
        &["P1Y2M", "-P13M"],
    );
}

#[test]
fn ill_formed_binary_values_violate_and_well_formed_ones_conform() {
    check(
        "xsd:hexBinary",
        &["ABC", "zz", "0x0F"],
        &["0FB7", "0fb7", ""],
    );
    check(
        "xsd:base64Binary",
        &["abc", "!!!!", "AQJ=", "AR=="],
        &["AQID", "AQ==", "AQI=", "AQ I D", ""],
    );
}

#[test]
fn the_already_checked_datatypes_keep_their_verdicts() {
    check(
        "xsd:integer",
        &["3.5", "abc"],
        &["42", "-0", "99999999999999999999999"],
    );
    check("xsd:decimal", &["1e3", "abc"], &["1.5", "-.5", "3"]);
    check("xsd:double", &["+INF", "abc"], &["1e3", "INF", "NaN"]);
    check("xsd:float", &["+INF", "abc"], &["1e3", "-INF"]);
    check("xsd:boolean", &["yes", "TRUE"], &["true", "0", " false "]);
    check("xsd:byte", &["128", "abc"], &["127", "-128"]);
    check("xsd:string", &[], &["anything at all", ""]);
}

#[test]
fn a_language_tagged_string_matches_rdf_lang_string_only() {
    assert_eq!(datatype_violations("rdf:langString", "\"chat\"@fr"), 0);
    assert_eq!(datatype_violations("xsd:string", "\"chat\"@fr"), 1);
    assert_eq!(datatype_violations("rdf:langString", "\"chat\""), 1);
}

#[test]
fn datatypes_without_a_modelled_lexical_space_keep_conforming() {
    for (datatype, lexical) in [
        ("ex:custom", "anything at all"),
        ("ex:custom", ""),
        ("rdf:HTML", "<b>bold</b>"),
        ("rdf:XMLLiteral", "<b>bold</b>"),
        ("xsd:anyURI", "http://example.org/a"),
        ("xsd:token", "a token"),
        ("xsd:language", "en-GB"),
    ] {
        assert_eq!(
            datatype_violations(datatype, &typed(lexical, datatype)),
            0,
            "{lexical:?}^^{datatype} keeps conforming"
        );
    }
    // The IRI must still match exactly: a well-formed date is not an `ex:custom`.
    assert_eq!(
        datatype_violations("ex:custom", "\"2001-01-01\"^^xsd:date"),
        1
    );
}
