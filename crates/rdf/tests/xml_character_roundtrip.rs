// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! XML character preservation through production writers and readers.

use purrdf_rdf::{NativeRdfFormat, dataset_from_bytes, serialize_dataset_to_format};
use std::fmt::Write as _;

fn literal_document(lexical: &str) -> String {
    let mut escaped = String::new();
    for character in lexical.chars() {
        write!(escaped, "\\U{:08X}", character as u32).unwrap();
    }
    format!("<http://example.org/s> <http://example.org/p> \"{escaped}\" .\n")
}

#[test]
fn xml_writers_preserve_valid_scalars_and_line_endings() {
    let values = [
        "a\rb",
        "a\r\nb",
        "\t\n\r",
        "&<>'\"",
        "\u{20}\u{7F}\u{85}\u{A0}",
        "\u{D7FF}\u{E000}\u{FFFD}",
        "\u{10000}\u{1FFFE}\u{10FFFF}",
    ];
    for value in values {
        let dataset = dataset_from_bytes(
            literal_document(value).as_bytes(),
            NativeRdfFormat::NTriples,
        )
        .unwrap();
        let expected = serialize_dataset_to_format(&*dataset, NativeRdfFormat::NTriples, None)
            .unwrap()
            .bytes;
        for format in [NativeRdfFormat::RdfXml, NativeRdfFormat::TriX] {
            let bytes = serialize_dataset_to_format(&*dataset, format, None)
                .unwrap()
                .bytes;
            let xml = std::str::from_utf8(&bytes).unwrap();
            roxmltree::Document::parse(xml).unwrap();
            assert!(!xml.contains('\r'), "raw CR in {format:?}");
            let reread = dataset_from_bytes(&bytes, format).unwrap();
            let actual = serialize_dataset_to_format(&*reread, NativeRdfFormat::NTriples, None)
                .unwrap()
                .bytes;
            assert_eq!(actual, expected, "{format:?}: {value:?}");
        }
    }
}

#[test]
fn xml_writers_refuse_every_unrepresentable_scalar() {
    for scalar in (0..0x20).chain([0xFFFE, 0xFFFF]) {
        if matches!(scalar, 9 | 10 | 13) {
            continue;
        }
        let value = format!("before{}after", char::from_u32(scalar).unwrap());
        let dataset = dataset_from_bytes(
            literal_document(&value).as_bytes(),
            NativeRdfFormat::NTriples,
        )
        .unwrap();
        for format in [NativeRdfFormat::RdfXml, NativeRdfFormat::TriX] {
            let error = serialize_dataset_to_format(&*dataset, format, None).unwrap_err();
            assert!(
                error.to_string().contains(&format!("U+{scalar:04X}")),
                "{error}"
            );
        }
    }
}

#[test]
fn xml_literal_canonicalization_preserves_attribute_whitespace() {
    let source = br#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:ex="http://example.org/">
      <rdf:Description rdf:about="http://example.org/s"><ex:p rdf:parseType="Literal"><ex:v a="&#x9;&#xA;&#xD;">a&#xD;b</ex:v></ex:p></rdf:Description>
    </rdf:RDF>"#;
    let dataset = dataset_from_bytes(source, NativeRdfFormat::RdfXml).unwrap();
    let nt = serialize_dataset_to_format(&*dataset, NativeRdfFormat::NTriples, None)
        .unwrap()
        .bytes;
    let text = std::str::from_utf8(&nt).unwrap();
    assert!(text.contains("&#x9;&#xA;&#xD;"), "{text}");
    assert!(text.contains("a&#xD;b"), "{text}");
    let xml = serialize_dataset_to_format(&*dataset, NativeRdfFormat::RdfXml, None)
        .unwrap()
        .bytes;
    let reread = dataset_from_bytes(&xml, NativeRdfFormat::RdfXml).unwrap();
    assert_eq!(
        serialize_dataset_to_format(&*reread, NativeRdfFormat::NTriples, None)
            .unwrap()
            .bytes,
        nt
    );
}

#[test]
fn projection_attributes_survive_xml_attribute_normalization() {
    let value = "\t\n\r&<>'\"\u{85}\u{A0}🐈";
    let attribute = purrdf_rdf::escape_xml_attribute(value).unwrap();
    let text = purrdf_rdf::escape_xml_text(value).unwrap();
    let source = format!("<root value=\"{attribute}\">{text}</root>");
    let parsed = roxmltree::Document::parse(&source).unwrap();
    assert_eq!(parsed.root_element().attribute("value"), Some(value));
    assert_eq!(parsed.root_element().text(), Some(value));
}
