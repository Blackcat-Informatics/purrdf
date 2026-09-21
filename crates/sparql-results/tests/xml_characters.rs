// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Production SRX writer/reader preservation and refusal boundaries.

use purrdf_core::{RdfDatasetBuilder, TermValue};
use purrdf_sparql_results::{ResultProvenance, SparqlResult, from_xml, to_xml};

fn result(value: &str) -> SparqlResult {
    SparqlResult::Solutions {
        variables: vec!["a\t\n\rb".to_owned()],
        rows: vec![vec![Some(TermValue::Literal {
            lexical_form: value.to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
            language: None,
            direction: None,
        })]],
        aux: RdfDatasetBuilder::new().freeze().unwrap(),
    }
}

#[test]
fn text_and_attribute_whitespace_round_trip() {
    let original = result("\t\n\r\r\n<&>\u{85}\u{A0}🐈\u{10FFFF}");
    let bytes = to_xml(&original, &ResultProvenance::default(), None)
        .unwrap()
        .bytes;
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains("name=\"a&#x9;&#xA;&#xD;b\""));
    assert!(!text.contains('\r'));
    let parsed = from_xml(&bytes).unwrap();
    let SparqlResult::Solutions {
        variables, rows, ..
    } = original
    else {
        unreachable!()
    };
    assert_eq!(parsed.variables, variables);
    assert_eq!(parsed.rows, rows);
}

#[test]
fn invalid_scalars_are_refused_by_name() {
    for scalar in (0..0x20).chain([0xFFFE, 0xFFFF]) {
        if matches!(scalar, 9 | 10 | 13) {
            continue;
        }
        let value = format!("&{}", char::from_u32(scalar).unwrap());
        let error = to_xml(&result(&value), &ResultProvenance::default(), None).unwrap_err();
        assert!(
            error.to_string().contains(&format!("U+{scalar:04X}")),
            "{error}"
        );
    }
}
