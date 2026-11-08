// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Closed input configuration shared by native and foreign RDF callers.

use crate::json_config::{field, portable};
use purrdf_core::RdfDiagnostic;
use purrdf_lex::allocation::{Ceiling, Memory};
use purrdf_lex::json::record::{DecodeError, Record, RecordAllocation, Within};
use purrdf_lex::json::{Object, Value};
use purrdf_rdf::native_codecs::jsonld::{
    CompiledJsonLdContext, JsonLdContextFormat, JsonLdContextLimits, JsonLdContextRegistry,
    YamlLdOptions,
};
use purrdf_rdf::{ParseOptions, ParseOutcome, parse_dataset_with};
use std::cell::RefCell;

/// Maximum UTF-8 bytes in an input-options document.
pub const MAX_OPTIONS_BYTES: usize = JsonLdContextLimits::DEFAULT.max_options_bytes();
/// Separate native account for JSON decoding, record metadata and retained configuration.
pub const MAX_OPTIONS_ALLOCATION_BYTES: usize = 128 << 20;

/// Owned version-one YAML-LD input controls and immutable offline context documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdfInputOptions {
    /// Native Basic selection and admission controls.
    pub yaml_ld: YamlLdOptions,
    /// Initial context; absence differs from an explicit JSON null reset.
    pub context: Option<Value>,
    /// Caller-supplied original context document bytes and explicit formats.
    pub registry: JsonLdContextRegistry,
}

pub(crate) type Registration = (String, Vec<u8>, JsonLdContextFormat);

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

pub(crate) fn basic_fields(
    fields: &mut Record<'_>,
    allocation: &dyn RecordAllocation,
) -> Result<YamlLdOptions, DecodeError> {
    let mut options = YamlLdOptions::DEFAULT;
    options.extract_all_scripts = field(
        fields,
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
    Ok(options)
}

pub(crate) fn registration(
    value: &Value,
    allocation: &dyn RecordAllocation,
) -> Result<Registration, DecodeError> {
    let mut fields = Record::new_native(value, "offline context record", Some(allocation))?;
    let iri: String = fields.required_native("iri")?;
    let format_text: String = fields.required_native("format")?;
    let document: String = fields.required_native("document")?;
    let format = match format_text.as_str() {
        "json" => JsonLdContextFormat::Json,
        "yaml" => JsonLdContextFormat::Yaml(fields.required_with("yaml_ld", |value| {
            let mut controls = Record::new_native(value, "YAML-LD input object", Some(allocation))?;
            let options = basic_fields(&mut controls, allocation)?;
            controls.deny_unknown()?;
            Ok::<_, DecodeError>(options)
        })?),
        _ => {
            return Err(DecodeError::custom_native(
                "context format must be json or yaml",
                Some(allocation),
            )
            .within_native("format", Some(allocation)));
        }
    };
    allocation
        .release_text(format_text)
        .map_err(DecodeError::from_storage)?;
    fields.deny_unknown()?;
    Ok((iri, document.into_bytes(), format))
}

fn decode(
    value: &Value,
    allocation: &dyn RecordAllocation,
    memory: &RefCell<Memory<'_, Ceiling>>,
) -> Result<(YamlLdOptions, Option<Value>, Vec<Registration>), DecodeError> {
    let mut envelope = Record::new_native(value, "RDF input options object", Some(allocation))?;
    let version: u32 = envelope.required_native("version")?;
    if version != 1 {
        return Err(DecodeError::custom_native(
            "input-options version must be 1",
            Some(allocation),
        ));
    }
    let decoded = envelope.required_with("yaml_ld", |value| {
        let mut fields = Record::new_native(value, "YAML-LD input object", Some(allocation))?;
        let options = basic_fields(&mut fields, allocation)?;
        let context = fields
            .raw("context")?
            .map(|value| value.clone_with_memory(&mut memory.borrow_mut()))
            .transpose()
            .map_err(DecodeError::from_storage)?;
        let registry = fields
            .raw("registry")?
            .map(|value| {
                purrdf_lex::json::record::items_with_memory(
                    value,
                    memory,
                    Some(allocation),
                    |value| registration(value, allocation),
                )
                .map_err(|error| error.within_native("registry", Some(allocation)))
            })
            .transpose()?
            .unwrap_or_default();
        fields.deny_unknown()?;
        Ok::<_, DecodeError>((options, context, registry))
    })?;
    envelope.deny_unknown()?;
    Ok(decoded)
}

/// Decode owned version-one YAML-LD Basic and offline context input options.
/// `usize` controls accept integer lexemes in `0..=u32::MAX`; `u64` controls
/// preserve their complete range. Omitted controls inherit native defaults.
/// # Errors
/// Refuses oversized, malformed, duplicate, unknown, null or mistyped records.
pub fn options(bytes: &[u8]) -> Result<RdfInputOptions, RdfDiagnostic> {
    let limits = JsonLdContextLimits::DEFAULT;
    let (yaml_ld, context, registry) = crate::json_config::decode(
        bytes,
        "rdf-input-options",
        MAX_OPTIONS_BYTES,
        MAX_OPTIONS_ALLOCATION_BYTES,
        purrdf_lex::json::Limits {
            max_depth: limits.max_nesting(),
            max_values: limits.max_expansion_work() as u64,
            max_string_bytes: MAX_OPTIONS_BYTES,
            unique_members: true,
        },
        "input-options document exceeds the native context options byte limit",
        decode,
    )?;
    Ok(RdfInputOptions {
        yaml_ld,
        context,
        registry: JsonLdContextRegistry::new_with_formats(registry)?,
    })
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
    let configuration = input_options.map(options).transpose()?;
    let empty = Value::Object(Object::new());
    let compiled = configuration
        .as_ref()
        .filter(|options| options.context.is_some() || !options.registry.is_empty())
        .map(|options| {
            CompiledJsonLdContext::compile_with_registry(
                options.context.as_ref().unwrap_or(&empty),
                base,
                &options.registry,
            )
        })
        .transpose()?;
    parse_dataset_with(
        bytes,
        media_type,
        base,
        &ParseOptions {
            track_source_spans: false,
            yaml_ld: configuration.as_ref().map(|options| options.yaml_ld),
            yaml_ld_context: compiled.as_ref(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    const YAML: &[u8] = b"'@id': first\nhttps://example.org/p: one\n---\n'@id': second\nhttps://example.org/p: two\n";
    const ALL: &[u8] = br#"{"version":1,"yaml_ld":{"extract_all_scripts":true}}"#;

    #[test]
    fn configured_yamlld_input_context_preserves_rdf_base_and_admission() {
        let base = "https://example.org/docs/source.yamlld";
        let configured = r#"{"version":1,"yaml_ld":{
          "context":{"label":"https://example.org/label"},
          "registry":[
            {"iri":"https://example.org/docs/contexts/main.yamlld","format":"yaml","document":"'@context':\n  '@import': shared.jsonld\n  child:\n    '@id': https://example.org/child\n    '@context': scoped.yamlld\n","yaml_ld":{}},
            {"iri":"https://example.org/docs/contexts/shared.jsonld","format":"json","document":"{\"@context\":{\"p\":{\"@id\":\"https://example.org/p\",\"@type\":\"@id\"}}}"},
            {"iri":"https://example.org/docs/contexts/scoped.yamlld","format":"yaml","document":"'@context': {q: 'https://example.org/q'}\n","yaml_ld":{}}
          ]}}"#;
        let source = b"'@context': contexts/main.yamlld\n'@id': subject\np: object\nlabel: one\nchild: {'@id': child, q: inside}\n";
        let actual = parse(
            source,
            "application/ld+yaml",
            Some(base),
            Some(configured.as_bytes()),
        )
        .unwrap();
        let expected = purrdf_rdf::parse_dataset(
            concat!(
                "<https://example.org/docs/subject> <https://example.org/p> <https://example.org/docs/object> .\n",
                "<https://example.org/docs/subject> <https://example.org/label> \"one\" .\n",
                "<https://example.org/docs/subject> <https://example.org/child> <https://example.org/docs/child> .\n",
                "<https://example.org/docs/child> <https://example.org/q> \"inside\" .\n",
            ).as_bytes(),
            "application/n-quads",
            None,
        ).unwrap();
        assert!(
            purrdf_rdf::datasets_isomorphic(
                &actual.dataset,
                &expected,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                ),
            )
            .unwrap()
        );
        assert_eq!(actual.document_base_iri(), Some(base));
        assert_eq!(
            actual.document_base.as_ref().unwrap().origin(),
            purrdf_iri::BaseOrigin::Caller
        );

        let registry_only =
            configured.replace("\"context\":{\"label\":\"https://example.org/label\"},", "");
        let registry_source = b"'@context': contexts/main.yamlld\n'@id': subject\np: object\n";
        let registry_dataset = parse(
            registry_source,
            "application/ld+yaml",
            Some(base),
            Some(registry_only.as_bytes()),
        )
        .unwrap();
        assert_eq!(registry_dataset.dataset.quad_count(), 1);
        assert!(options(registry_only.as_bytes()).unwrap().context.is_none());
        let reset = br#"{"version":1,"yaml_ld":{"context":null}}"#;
        assert_eq!(options(reset).unwrap().context, Some(Value::Null));
        assert_eq!(
            parse(
                b"'@id': subject\nhttps://example.org/p: one\n",
                "application/ld+yaml",
                Some(base),
                Some(reset)
            )
            .unwrap()
            .document_base_iri(),
            Some(base)
        );

        let admission = configured.replace(
            "\"yaml_ld\":{}",
            "\"yaml_ld\":{\"syntax\":{\"max_input_bytes\":0}}",
        );
        let refused = parse(
            source,
            "application/ld+yaml",
            Some(base),
            Some(admission.as_bytes()),
        )
        .unwrap_err();
        assert_eq!(refused.code, "native-yaml-input-limit");
        assert!(refused.storage_error().is_some());
        assert!(refused.location.is_some());

        let moved = br#"{"version":1,"yaml_ld":{"context":{"@base":"https://moved.example/","p":"https://example.org/p"}}}"#;
        let moved_source = b"'@id': subject\np: one\n";
        let moved_outcome =
            parse(moved_source, "application/ld+yaml", Some(base), Some(moved)).unwrap();
        assert_eq!(
            purrdf_rdf::serialize_dataset(
                &moved_outcome.dataset,
                "application/n-quads",
                purrdf_rdf::SerializeGraph::Dataset
            )
            .unwrap(),
            b"<https://moved.example/subject> <https://example.org/p> \"one\" .\n"
        );
        assert_eq!(
            moved_outcome.document_base_iri(),
            Some("https://moved.example/")
        );
        assert_eq!(
            moved_outcome.document_base.as_ref().unwrap().origin(),
            purrdf_iri::BaseOrigin::Enclosing
        );
        let cleared =
            br#"{"version":1,"yaml_ld":{"context":{"@base":null,"p":"https://example.org/p"}}}"#;
        assert!(
            parse(
                b"'@id': https://example.org/subject\np: one\n",
                "application/ld+yaml",
                Some(base),
                Some(cleared)
            )
            .unwrap()
            .document_base
            .is_none()
        );

        let configuration = options(moved).unwrap();
        let context = CompiledJsonLdContext::compile_with_registry(
            configuration.context.as_ref().unwrap(),
            Some(base),
            &configuration.registry,
        )
        .unwrap();
        let native_options = ParseOptions {
            yaml_ld_context: Some(&context),
            ..ParseOptions::default()
        };
        let native = parse_dataset_with(
            moved_source,
            "application/ld+yaml",
            Some(base),
            &native_options,
        )
        .unwrap();
        assert_eq!(native.document_base, moved_outcome.document_base);
        assert_eq!(
            parse_dataset_with(
                moved_source,
                "application/ld+yaml",
                Some("https://wrong.example/"),
                &native_options
            )
            .unwrap_err()
            .code,
            "yamlld-input-context-base"
        );
        assert_eq!(
            parse_dataset_with(b"{}", "application/ld+json", Some(base), &native_options)
                .unwrap_err()
                .code,
            "yamlld-input-options-format"
        );

        for registry in [
            "null",
            r#"[{"iri":"https://example.org/c","format":"yaml","document":"'@context': {}"}]"#,
            r#"[{"iri":"https://example.org/c","format":"json","document":"{}","yaml_ld":{}}]"#,
            r#"[{"iri":"https://example.org/c","format":"other","document":"{}"}]"#,
            r#"[{"iri":"https://example.org/c","format":"json","document":"{}","extra":0}]"#,
            r#"[{"iri":"https://example.org/c","format":"json"}]"#,
            r#"[{"iri":"https://example.org/c","format":"json","format":"yaml","document":"{}"}]"#,
            r#"[{"iri":"https://example.org/c","format":"json","document":"{}"},{"iri":"https://example.org/c","format":"json","document":"{}"}]"#,
        ] {
            let refused = format!("{{\"version\":1,\"yaml_ld\":{{\"registry\":{registry}}}}}");
            assert!(options(refused.as_bytes()).is_err(), "{registry}");
        }
    }

    #[test]
    fn closed_options_preserve_defaults_and_exact_portable_limits() {
        assert_eq!(
            options(br#"{"version":1,"yaml_ld":{}}"#).unwrap().yaml_ld,
            YamlLdOptions::DEFAULT
        );
        let limits = options(br#"{"version":1,"yaml_ld":{"syntax":{"max_input_bytes":4294967295,"max_occurrences":18446744073709551615},"projection":{"max_nodes":18446744073709551615,"max_numeric_work":18446744073709551615}}}"#).unwrap().yaml_ld;
        assert_eq!(limits.syntax.syntax.max_input_bytes, u32::MAX as usize);
        assert_eq!(limits.syntax.syntax.max_occurrences, u64::MAX);
        assert_eq!(limits.projection.max_nodes, u64::MAX);
        assert_eq!(limits.projection.max_numeric_work, u64::MAX);
        for refused in [
            "{}",
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
                yaml_ld: Some(options(ALL).unwrap().yaml_ld),
                yaml_ld_context: None,
            },
        )
        .unwrap();
        assert!(native.spans.unwrap().is_empty());
        assert_eq!(native.document_prefixes, Vec::<(String, String)>::new());
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
