// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A JSON number becomes the binary64 nearest its decimal value in every reader that
//! turns one into RDF, on every target, the x87 included.
//!
//! The witnesses are numbers a reader that is not correctly rounded gets wrong, found by
//! search against [`soft::significand_power_decimal`] -- the conversion `serde_json` makes
//! without its `float_roundtrip` feature, reproduced in integers -- and against the x87's
//! double rounding of an exact fast path ([`soft::x87_fast_path_decimals`]). Each reader
//! must give every witness the value `str::parse::<f64>` gives it (core's reader is
//! correctly rounded and sets the x87's precision itself), and so the literal that value
//! implies, and never the neighbour the old reading landed on.

use std::collections::BTreeMap;

use purrdf_rdf::{
    CsvwAction, CsvwConfig, CsvwContext, CsvwInput, CsvwMode, CsvwVocabulary, DatasetSink,
    OkfBundle, OkfConfig, ProjectionLimits, SerializeGraph, datasets_isomorphic, lift_okf_bundle,
    parse_dataset, read_csvw, serialize_dataset, write_okf_bundle,
};
use purrdf_xsd::ieee::Binary64Scope;
use purrdf_xsd::ieee::reference as soft;
use purrdf_xsd::numeric::canonical_double;

const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";

/// Every decimal witness: long mantissas and shortest forms the significand-times-power
/// reader misreads, fast-path numbers the x87 double rounds, the published misreadings,
/// and midpoints between neighbouring binary64 values (with a digit more and a digit
/// less), which only a reader of every digit decides.
fn witnesses() -> Vec<String> {
    let mut witnesses = soft::misread_decimals(40);
    witnesses.extend(
        soft::misread_spellings(40, |value| Some(format!("{value:e}")))
            .into_iter()
            .map(|(_, lexical)| lexical),
    );
    witnesses.extend(soft::x87_fast_path_decimals(40));
    witnesses.extend(
        [
            "122.416294033786585",
            "51.708947112827516",
            "8.64759627780072e32",
        ]
        .map(str::to_owned),
    );
    for value in [1.0, 0.1, 1e-300, 123_456.789, f64::MIN_POSITIVE, 5e-324] {
        let midpoint = soft::successor_midpoint_decimal(value);
        let below = format!("{}4999", &midpoint[..midpoint.len() - 1]);
        witnesses.push(format!("{midpoint}00000000000000000000000001"));
        witnesses.push(below);
        witnesses.push(midpoint);
    }
    let negated: Vec<String> = witnesses
        .iter()
        .step_by(7)
        .map(|w| format!("-{w}"))
        .collect();
    witnesses.extend(negated);
    witnesses
}

/// The binary64 nearest `lexical`'s decimal value.
fn correct(lexical: &str) -> f64 {
    lexical.parse().expect("a decimal number")
}

fn nquads(dataset: &purrdf_rdf::RdfDataset) -> String {
    String::from_utf8(
        serialize_dataset(dataset, "application/n-quads", SerializeGraph::Dataset)
            .expect("N-Quads"),
    )
    .expect("UTF-8")
}

/// The reader itself, as the workspace resolves it: `serde_json` with `float_roundtrip`,
/// read under the binary64 scope every result-affecting read in this workspace enters.
/// The sweep covers the witnesses and a wide draw of midpoints, the hardest inputs a
/// decimal reader has.
#[test]
fn serde_json_reads_every_witness_and_midpoint_correctly_rounded() {
    let mut lexicals = witnesses();
    let mut bits = 0x6a73_6f6e_u64;
    for index in 0..1_500_u32 {
        bits = bits
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let raw = if index % 10 == 0 {
            (bits >> 12).max(1)
        } else {
            (bits >> 1) & 0x7fef_ffff_ffff_ffff
        };
        let midpoint = soft::successor_midpoint_decimal(f64::from_bits(raw.max(1)));
        if midpoint.contains('.') {
            lexicals.push(format!("{}4999", &midpoint[..midpoint.len() - 1]));
            lexicals.push(format!("{midpoint}0000000000000001"));
        }
        lexicals.push(midpoint);
    }
    let mut misread_by_the_old_reader = 0;
    for lexical in &lexicals {
        let read: f64 = {
            let _binary64 = Binary64Scope::enter();
            serde_json::from_str(lexical).expect("a JSON number")
        };
        assert_eq!(read.to_bits(), correct(lexical).to_bits(), "{lexical}");
        if soft::significand_power_decimal(lexical).map(f64::to_bits) != Some(read.to_bits()) {
            misread_by_the_old_reader += 1;
        }
    }
    assert!(
        misread_by_the_old_reader > 1_000,
        "the sweep must hold inputs the default reader gets wrong: {misread_by_the_old_reader}"
    );
}

/// JSON-LD: a bare number, a value object, a number coerced to `xsd:double` by the
/// context, and an `@json` value all carry the correctly rounded value.
#[test]
fn json_ld_numbers_expand_to_the_correctly_rounded_double() {
    let witnesses = witnesses();
    let list = witnesses.join(",");
    let objects = witnesses
        .iter()
        .map(|witness| format!("{{\"@value\":{witness}}}"))
        .collect::<Vec<_>>()
        .join(",");
    let document = format!(
        r#"{{
  "@context": {{
    "coerced": {{"@id": "http://example.org/coerced", "@type": "{XSD_DOUBLE}"}},
    "json": {{"@id": "http://example.org/json", "@type": "@json"}}
  }},
  "@id": "http://example.org/s",
  "http://example.org/bare": [{list}],
  "http://example.org/object": [{objects}],
  "coerced": [{list}],
  "json": [{list}]
}}"#
    );
    let dataset = parse_dataset(document.as_bytes(), "application/ld+json", None)
        .expect("parse the JSON-LD document");
    let quads = nquads(&dataset);

    let mut expected = std::collections::BTreeSet::new();
    for witness in &witnesses {
        let value = correct(witness);
        let lexical = canonical_double(value);
        for property in ["bare", "object", "coerced"] {
            let line = format!(
                "<http://example.org/s> <http://example.org/{property}> \"{lexical}\"^^<{XSD_DOUBLE}> .",
            );
            assert!(quads.contains(&line), "{witness}: missing {line}");
            expected.insert(line);
        }
    }
    // An `@json` term's value is the whole array: one rdf:JSON literal, serialized from
    // the values read.
    let values: Vec<f64> = witnesses.iter().map(|witness| correct(witness)).collect();
    let json = serde_json::to_string(&values).expect("finite numbers");
    let line = format!(
        "<http://example.org/s> <http://example.org/json> \"{json}\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#JSON> .",
    );
    assert!(quads.contains(&line), "missing {line}");
    expected.insert(line);
    // Nothing else: no neighbour of any witness was written beside it.
    assert_eq!(quads.lines().count(), expected.len(), "{quads}");
}

fn csvw_config() -> CsvwConfig {
    CsvwConfig::new(
        "http://example.org/",
        CsvwContext::new("http://www.w3.org/ns/csvw", BTreeMap::new()).expect("context"),
        "http://example.org/package",
        CsvwVocabulary::new(
            "http://www.w3.org/ns/csvw#",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
            "http://www.w3.org/2000/01/rdf-schema#",
            "http://www.w3.org/2001/XMLSchema#",
        )
        .expect("vocabulary"),
        CsvwMode::Standard,
        ProjectionLimits::new(64, 1_000_000, 4_000_000, 5_000_000, 16).expect("limits"),
        10_000,
    )
    .expect("config")
}

/// CSVW: a number in a common property, bare or as a value object, becomes a literal
/// whose lexical form is the correctly rounded value's.
#[test]
fn csvw_metadata_numbers_become_the_correctly_rounded_literal() {
    let witnesses = witnesses();
    let bare = witnesses.join(",");
    let objects = witnesses
        .iter()
        .map(|witness| format!("{{\"@value\":{witness}}}"))
        .collect::<Vec<_>>()
        .join(",");
    let metadata_iri = "http://example.org/metadata.json";
    let table_iri = "http://example.org/numbers.csv";
    let metadata = format!(
        r#"{{
            "@context":"http://www.w3.org/ns/csvw",
            "url":"{table_iri}",
            "http://example.org/bare":[{bare}],
            "http://example.org/object":[{objects}],
            "tableSchema":{{"columns":[{{"name":"n","titles":"n"}}]}}
        }}"#
    );
    let config = csvw_config();
    let input = CsvwInput::new(
        CsvwAction::Metadata {
            metadata_iri: metadata_iri.to_owned(),
        },
        BTreeMap::from([
            (metadata_iri.to_owned(), metadata.into_bytes()),
            (table_iri.to_owned(), b"n\n1\n".to_vec()),
        ]),
        config.limits(),
    )
    .expect("input");
    let outcome = read_csvw(&input, &config).expect("read CSVW");
    let quads = nquads(&outcome.dataset);
    for witness in &witnesses {
        let value = correct(witness);
        let lexical = serde_json::Number::from_f64(value)
            .expect("finite")
            .to_string();
        let old = soft::significand_power_decimal(witness)
            .and_then(serde_json::Number::from_f64)
            .map(|number| number.to_string());
        for property in ["bare", "object"] {
            let object = format!("<http://example.org/{property}> \"{lexical}\"^^<{XSD_DOUBLE}> .");
            assert!(quads.contains(&object), "{witness}: missing {object}");
            if let Some(old) = old.as_ref().filter(|old| **old != lexical) {
                let neighbour =
                    format!("<http://example.org/{property}> \"{old}\"^^<{XSD_DOUBLE}> .");
                let spelled_by_another = witnesses.iter().any(|other| {
                    serde_json::Number::from_f64(correct(other))
                        .map(|number| number.to_string())
                        .as_deref()
                        == Some(old.as_str())
                });
                assert!(
                    spelled_by_another || !quads.contains(&neighbour),
                    "{witness}: the neighbour {old} was written"
                );
            }
        }
    }
}

fn okf_config() -> OkfConfig {
    OkfConfig::new(
        "https://example.org/okf#",
        "https://example.org/doc/",
        ["type", "producer"],
    )
    .expect("valid caller profile")
}

/// OKF: a YAML float inside structured frontmatter becomes an rdf:JSON literal through
/// `serde_json`, and writing it back parses that literal again and requires it to be
/// canonical. The read witnesses are floats whose decimal lexical form the old reader
/// misread; the write witnesses are floats whose serialized form it misread, which made
/// the canonical check refuse a literal the reader itself produced. Each list also holds
/// values whose form the x87 double rounds in the exact fast path.
#[test]
fn okf_structured_numbers_read_and_write_back_correctly_rounded() {
    // OKF reads a YAML float as an `xsd:decimal` of at most 18 fraction digits: the
    // witnesses are the fractional values it accepts.
    let decimal = |value: f64| {
        let plain = value.to_string();
        plain
            .split_once('.')
            .is_some_and(|(_, fraction)| fraction.len() <= 18)
            .then_some(plain)
    };
    let serialized = |value: f64| {
        decimal(value)?;
        serde_json::to_string(&value).ok()
    };
    let values = |found: Vec<(f64, String)>| found.into_iter().map(|(value, _)| value);
    let read: Vec<f64> = values(soft::misread_spellings(40, decimal))
        .chain(values(soft::x87_misread_spellings(20, decimal)))
        .collect();
    let write: Vec<f64> = values(soft::misread_spellings(40, serialized))
        .chain(values(soft::x87_misread_spellings(20, serialized)))
        .collect();

    let yaml = |values: &[f64]| {
        values
            .iter()
            .map(f64::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let document = format!(
        "---\ntype: Table\nproducer:\n  read: [{}]\n  write: [{}]\n---\nNumbers.\n",
        yaml(&read),
        yaml(&write)
    );
    let config = okf_config();
    let bundle =
        OkfBundle::from_documents([("numbers.md", document.as_str())]).expect("valid OKF bundle");
    let mut sink = DatasetSink::new();
    let outcome = lift_okf_bundle(&bundle, &config, &mut sink).expect("lift bundle");
    assert!(outcome.losses.is_empty());
    let dataset = sink.into_dataset().expect("finished sink");

    let expected = serde_json::to_string(&serde_json::json!({ "read": read, "write": write }))
        .expect("finite numbers");
    let quads = nquads(&dataset);
    let literal = serde_json::to_string(&expected).expect("a string");
    assert!(
        quads.contains(&format!("{literal}^^<{}>", config.json_datatype())),
        "missing {literal} in {quads}"
    );

    let written = write_okf_bundle(&dataset, &config).expect("write the bundle back");
    let mut sink = DatasetSink::new();
    lift_okf_bundle(&written.bundle, &config, &mut sink).expect("lift the written bundle");
    let reread = sink.into_dataset().expect("finished sink");
    assert!(datasets_isomorphic(&dataset, &reread));
}

/// CSVW: a datatype's `minimum` and `maximum` read from a referenced schema resource are
/// the values their decimals spell. Each column bounds a double to exactly the value of
/// its one cell, which is written with the same decimal, so a bound read as either
/// neighbour refuses the cell.
#[test]
fn csvw_schema_bounds_admit_the_value_they_spell() {
    let witnesses = witnesses();
    let columns = witnesses
        .iter()
        .enumerate()
        .map(|(index, witness)| {
            format!(
                r#"{{"name":"c{index}","titles":"c{index}","datatype":{{"base":"double","minimum":{witness},"maximum":{witness}}}}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let metadata_iri = "http://example.org/bounds-metadata.json";
    let schema_iri = "http://example.org/bounds-schema.json";
    let table_iri = "http://example.org/bounds.csv";
    let metadata = format!(
        r#"{{"@context":"http://www.w3.org/ns/csvw","url":"{table_iri}","tableSchema":"{schema_iri}"}}"#
    );
    let schema = format!(r#"{{"columns":[{columns}]}}"#);
    let header = (0..witnesses.len())
        .map(|index| format!("c{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let table = format!("{header}\n{}\n", witnesses.join(","));
    let config = csvw_config();
    let input = CsvwInput::new(
        CsvwAction::Metadata {
            metadata_iri: metadata_iri.to_owned(),
        },
        BTreeMap::from([
            (metadata_iri.to_owned(), metadata.into_bytes()),
            (schema_iri.to_owned(), schema.into_bytes()),
            (table_iri.to_owned(), table.into_bytes()),
        ]),
        config.limits(),
    )
    .expect("input");
    let outcome = read_csvw(&input, &config).expect("read CSVW");
    assert!(
        outcome.warnings.is_empty(),
        "a bound refused its own value: {:#?}",
        outcome.warnings
    );
    let cells = &outcome.group.tables[0].rows[0].cells;
    assert_eq!(cells.len(), witnesses.len());
    for (cell, witness) in cells.iter().zip(&witnesses) {
        assert!(
            cell.values.len() == 1 && cell.values[0].datatype == XSD_DOUBLE,
            "{witness}: {cell:?}"
        );
    }
}
