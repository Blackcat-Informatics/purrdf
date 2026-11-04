// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Closed input configuration shared by native and foreign RDF callers.

use crate::json_config::{field, portable};
use purrdf_core::RdfDiagnostic;
use purrdf_lex::json::Value;
use purrdf_lex::json::record::{DecodeError, Record, RecordAllocation, Within};
use purrdf_rdf::native_codecs::jsonld::YamlLdOptions;
use purrdf_rdf::{ParseOptions, ParseOutcome, parse_dataset_with};

/// Maximum UTF-8 bytes in an input-options document.
pub const MAX_OPTIONS_BYTES: usize = 16 << 10;
/// Separate native allocation account for JSON decoding and record metadata.
pub const MAX_OPTIONS_ALLOCATION_BYTES: usize = 128 << 10;

fn syntax(
    value: &Value,
    options: &mut YamlLdOptions,
    allocation: &dyn RecordAllocation,
) -> Result<(), DecodeError> {
    let mut fields = Record::new_native(value, "YAML-LD syntax object", Some(allocation))?;
    let limits = &mut options.syntax.syntax;
    limits.max_input_bytes = portable(
        &mut fields,
        "max_input_bytes",
        limits.max_input_bytes,
        allocation,
    )?;
    limits.max_allocation_bytes = portable(
        &mut fields,
        "max_allocation_bytes",
        limits.max_allocation_bytes,
        allocation,
    )?;
    limits.max_occurrences = field(
        &mut fields,
        "max_occurrences",
        limits.max_occurrences,
        allocation,
    )?;
    limits.max_depth = portable(&mut fields, "max_depth", limits.max_depth, allocation)?;
    limits.aliases = field(&mut fields, "aliases", limits.aliases, allocation)?;
    options.syntax.max_documents = portable(
        &mut fields,
        "max_documents",
        options.syntax.max_documents,
        allocation,
    )?;
    fields.deny_unknown()
}

fn projection(
    value: &Value,
    options: &mut YamlLdOptions,
    allocation: &dyn RecordAllocation,
) -> Result<(), DecodeError> {
    let mut fields = Record::new_native(value, "YAML-LD projection object", Some(allocation))?;
    let limits = &mut options.projection;
    limits.max_allocation_bytes = portable(
        &mut fields,
        "max_allocation_bytes",
        limits.max_allocation_bytes,
        allocation,
    )?;
    limits.max_nodes = field(&mut fields, "max_nodes", limits.max_nodes, allocation)?;
    limits.max_depth = portable(&mut fields, "max_depth", limits.max_depth, allocation)?;
    limits.max_numeric_work = field(
        &mut fields,
        "max_numeric_work",
        limits.max_numeric_work,
        allocation,
    )?;
    limits.max_numeric_bytes = portable(
        &mut fields,
        "max_numeric_bytes",
        limits.max_numeric_bytes,
        allocation,
    )?;
    fields.deny_unknown()
}

fn decode(value: &Value, allocation: &dyn RecordAllocation) -> Result<YamlLdOptions, DecodeError> {
    let mut envelope = Record::new_native(value, "RDF input options object", Some(allocation))?;
    let version: u32 = envelope.required_native("version")?;
    if version != 1 {
        return Err(DecodeError::custom_native(
            "input-options version must be 1",
            Some(allocation),
        ));
    }
    let options = envelope.required_with("yaml_ld", |value| {
        let mut fields = Record::new_native(value, "YAML-LD input object", Some(allocation))?;
        let mut options = YamlLdOptions::DEFAULT;
        options.extract_all_scripts = field(
            &mut fields,
            "extract_all_scripts",
            options.extract_all_scripts,
            allocation,
        )?;
        if let Some(value) = fields.raw("syntax")? {
            syntax(value, &mut options, allocation)
                .map_err(|error| error.within_native("syntax", Some(allocation)))?;
        }
        if let Some(value) = fields.raw("projection")? {
            projection(value, &mut options, allocation)
                .map_err(|error| error.within_native("projection", Some(allocation)))?;
        }
        fields.deny_unknown()?;
        Ok::<_, DecodeError>(options)
    })?;
    envelope.deny_unknown()?;
    Ok(options)
}

/// Decode explicit version-one YAML-LD Basic input options.
/// `usize` controls accept integer lexemes in `0..=u32::MAX`; `u64` controls
/// preserve their complete range. Omitted controls inherit native defaults.
/// # Errors
/// Refuses oversized, malformed, duplicate, unknown, null or mistyped records.
pub fn options(bytes: &[u8]) -> Result<YamlLdOptions, RdfDiagnostic> {
    crate::json_config::decode(
        bytes,
        "rdf-input-options",
        MAX_OPTIONS_BYTES,
        MAX_OPTIONS_ALLOCATION_BYTES,
        "input-options document exceeds 16384 bytes",
        decode,
    )
}

/// Parse original RDF bytes with an optional closed input-options document.
/// # Errors
/// Configuration is decoded before document parsing. Explicit YAML controls
/// refuse all other formats; native parse diagnostics pass through unchanged.
pub fn parse(
    bytes: &[u8],
    media_type: &str,
    base: Option<&str>,
    input_options: Option<&[u8]>,
) -> Result<ParseOutcome, RdfDiagnostic> {
    let yaml_ld = input_options.map(options).transpose()?;
    parse_dataset_with(
        bytes,
        media_type,
        base,
        &ParseOptions {
            track_source_spans: false,
            yaml_ld,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    const YAML: &[u8] = b"'@id': first\nhttps://example.org/p: one\n---\n'@id': second\nhttps://example.org/p: two\n";
    const ALL: &[u8] = br#"{"version":1,"yaml_ld":{"extract_all_scripts":true}}"#;

    #[test]
    fn closed_options_preserve_defaults_and_exact_portable_limits() {
        assert_eq!(
            options(br#"{"version":1,"yaml_ld":{}}"#).unwrap(),
            YamlLdOptions::DEFAULT
        );
        let limits = options(br#"{"version":1,"yaml_ld":{"syntax":{"max_input_bytes":4294967295,"max_occurrences":18446744073709551615},"projection":{"max_nodes":18446744073709551615,"max_numeric_work":18446744073709551615}}}"#).unwrap();
        assert_eq!(limits.syntax.syntax.max_input_bytes, u32::MAX as usize);
        assert_eq!(limits.syntax.syntax.max_occurrences, u64::MAX);
        assert_eq!(limits.projection.max_nodes, u64::MAX);
        assert_eq!(limits.projection.max_numeric_work, u64::MAX);
        for refused in [
            r#"{}"#,
            r#"{"version":2,"yaml_ld":{}}"#,
            r#"{"version":1,"version":1,"yaml_ld":{}}"#,
            r#"{"version":1,"yaml_ld":null}"#,
            r#"{"version":1,"yaml_ld":{"syntax":null}}"#,
            r#"{"version":1,"yaml_ld":{"extract_all_scripts":1}}"#,
            r#"{"version":1,"yaml_ld":{"syntax":{"aliases":null}}}"#,
            r#"{"version":1,"yaml_ld":{"syntax":{"max_depth":4294967296}}}"#,
            r#"{"version":1,"yaml_ld":{"projection":{"max_nodes":18446744073709551616}}}"#,
            r#"{"version":1,"yaml_ld":{"projection":{"max_nodes":1.0}}}"#,
            r#"{"version":1,"yaml_ld":{"projection":{"max_nodes":1e2}}}"#,
            r#"{"version":1,"yaml_ld":{"syntax":{"max_depth":-1}}}"#,
            r#"{"version":1,"yaml_ld":{"max_serialization_bytes":0}}"#,
            r#"{"version":1,"yaml_ld":{"profile":"extended"}}"#,
            r#"{"version":1,"yaml_ld":{"projection":{"unknown":0}}}"#,
        ] {
            let error = options(refused.as_bytes()).unwrap_err();
            assert!(error.native_cause().is_some(), "{refused}");
        }
        let shape = options(br#"{"version":1,"yaml_ld":{"syntax":{"max_depth":-1}}}"#).unwrap_err();
        let original = crate::native_error::to_json(shape.native_cause().unwrap()).unwrap();
        assert_eq!(original["code"].as_str(), Some("json-record-decode"));
        assert_eq!(
            original["fields"]["pointer"]["value"].as_str(),
            Some("/yaml_ld/syntax/max_depth")
        );
        let syntax = options(b"{").unwrap_err();
        let original = crate::native_error::to_json(syntax.native_cause().unwrap()).unwrap();
        assert_eq!(original["code"].as_str(), Some("json-document"));
        assert!(options(&vec![b' '; MAX_OPTIONS_BYTES + 1]).is_err());
        assert!(options(br#"{"version":1,"yaml_ld":{"syntax":{"x":[[[[]]]]}}}"#).is_err());
    }

    #[test]
    fn configured_byte_boundary_preserves_selection_base_and_native_diagnostics() {
        let first = parse(
            YAML,
            "application/ld+yaml",
            Some("https://example.org/"),
            None,
        )
        .unwrap();
        let all = parse(
            YAML,
            "application/ld+yaml",
            Some("https://example.org/"),
            Some(ALL),
        )
        .unwrap();
        assert_eq!(first.dataset.quad_count(), 1);
        assert_eq!(all.dataset.quad_count(), 2);
        assert_eq!(first.document_base, all.document_base);
        let native = parse_dataset_with(
            YAML,
            "application/ld+yaml",
            Some("https://example.org/"),
            &ParseOptions {
                track_source_spans: true,
                yaml_ld: Some(options(ALL).unwrap()),
            },
        )
        .unwrap();
        assert!(native.spans.unwrap().is_empty());
        assert!(native.document_prefixes.is_empty());
        let resource = parse(
            YAML,
            "application/ld+yaml",
            None,
            Some(br#"{"version":1,"yaml_ld":{"syntax":{"max_input_bytes":0}}}"#),
        )
        .unwrap_err();
        assert_eq!(resource.code, "native-yaml-input-limit");
        assert!(resource.storage_error().is_some());
        assert!(resource.location.is_some());
        let semantic = parse(b"42: value\n", "application/ld+yaml", None, Some(ALL)).unwrap_err();
        assert_eq!(semantic.code, "mapping-key-error");
        assert!(semantic.location.is_some());
        for format in [
            "text/turtle",
            "application/ld+json",
            "application/gts",
            "pack",
        ] {
            assert!(parse(b"", format, None, Some(ALL)).is_err(), "{format}");
        }
        assert!(parse(b"", "text/turtle", None, None).is_ok());
    }
}
