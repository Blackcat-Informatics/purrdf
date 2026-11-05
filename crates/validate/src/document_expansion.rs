// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Closed structural document expansion shared by every host, without RDF lowering.

use crate::{json_config, rdf_input};
use purrdf_core::RdfDiagnostic;
use purrdf_lex::allocation::{Ceiling, Memory};
use purrdf_lex::json::record::{DecodeError, FromJsonNative, Record, RecordAllocation, Within};
use purrdf_lex::json::{self, Object, Value};
use purrdf_lex::walk::WorkList;
use purrdf_rdf::native_codecs::jsonld::{
    CompiledJsonLdContext, JsonLdContextLimits, JsonLdContextRegistry, JsonLdExpansionLimits,
    YamlLdOptions, expand_jsonld_with_context, expand_yamlld_with_context,
    jsonld_document_max_bytes,
};
use purrdf_rdf::native_codecs::{NativeRdfFormat, classify};
use std::cell::RefCell;

/// Independent serialized UTF-8 bytes and live writer/output allocation bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputLimits {
    /// Logical serialized-byte ceiling, independent of physical address width.
    pub max_bytes: u64,
    /// Live requested output and writer-scratch layouts.
    pub max_allocation_bytes: u64,
}
impl OutputLimits {
    /// 4 GiB of JSON bytes and 32 GiB of writer/output construction.
    pub const DEFAULT: Self = Self {
        max_bytes: 4 * 1024 * 1024 * 1024,
        max_allocation_bytes: 32 * 1024 * 1024 * 1024,
    };
}
impl Default for OutputLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Decoded and context-compiled request, reusable without decoding its JSON again.
/// Initial context presence remains distinct from an explicit null expression.
#[derive(Debug)]
pub struct Request {
    format: NativeRdfFormat,
    context: CompiledJsonLdContext,
    context_supplied: bool,
    yaml_ld: YamlLdOptions,
    expansion: JsonLdExpansionLimits,
    output: OutputLimits,
}
impl Request {
    /// The source format resolved through the native format-classification home.
    pub const fn format(&self) -> NativeRdfFormat {
        self.format
    }
    /// The original effective bare context, or absence; null stays explicit.
    pub fn initial_context(&self) -> Option<&Value> {
        self.context_supplied
            .then(|| self.context.canonical_context())
    }
    /// Existing source-decoder byte allowance, for bounded host acquisition.
    /// Physical slice/bridge layout admission remains separate.
    pub fn max_input_bytes(&self) -> usize {
        if self.format == NativeRdfFormat::YamlLd {
            self.yaml_ld.syntax.syntax.max_input_bytes
        } else {
            usize::try_from(jsonld_document_max_bytes()).unwrap_or(usize::MAX)
        }
    }
}
struct Decoded {
    format: String,
    base: Option<String>,
    context: Option<Value>,
    registry: Vec<rdf_input::Registration>,
    yaml_ld: Option<YamlLdOptions>,
    expansion: JsonLdExpansionLimits,
    output: OutputLimits,
}

fn bare_context(
    value: &Value,
    allocation: &dyn RecordAllocation,
    memory: &RefCell<Memory<'_, Ceiling>>,
) -> Result<(), DecodeError> {
    let mut pending: WorkList<_, 8> = WorkList::new();
    pending
        .try_push_admitted(value, &mut memory.borrow_mut())
        .map_err(DecodeError::from_storage)?;
    while let Some(value) = pending.pop() {
        match value {
            Value::Array(values) => {
                for value in values {
                    pending
                        .try_push_admitted(value, &mut memory.borrow_mut())
                        .map_err(DecodeError::from_storage)?;
                }
            }
            Value::Object(object) if object.contains_key("@context") => {
                return Err(DecodeError::custom_native(
                    "context requires a bare context expression, not a document containing @context",
                    Some(allocation),
                ));
            }
            // A term definition's own scoped @context is not a wrapper. Only
            // context-expression positions (including array entries) are checked.
            _ => {}
        }
    }
    pending
        .release_admitted(&mut memory.borrow_mut())
        .map_err(DecodeError::from_storage)
}

fn decode(
    value: &Value,
    allocation: &dyn RecordAllocation,
    memory: &RefCell<Memory<'_, Ceiling>>,
) -> Result<Decoded, DecodeError> {
    let mut fields = Record::new_native(value, "document expansion request", Some(allocation))?;
    let version: u32 = fields.required_native("version")?;
    if version != 1 {
        return Err(DecodeError::custom_native(
            "expansion request version must be 1",
            Some(allocation),
        ));
    }
    let format = fields.required_native("format")?;
    let base = fields
        .raw("base")?
        .map(|value| {
            String::from_json_native(value, Some(allocation))
                .map_err(|error| error.within_native("base", Some(allocation)))
        })
        .transpose()?;
    let context = fields
        .raw("context")?
        .map(|value| {
            bare_context(value, allocation, memory)
                .map_err(|error| error.within_native("context", Some(allocation)))?;
            value
                .clone_with_memory(&mut memory.borrow_mut())
                .map_err(DecodeError::from_storage)
        })
        .transpose()?;
    let registry = fields
        .raw("registry")?
        .map(|value| {
            json::record::items_with_memory(value, memory, Some(allocation), |value| {
                rdf_input::registration(value, allocation)
            })
            .map_err(|error| error.within_native("registry", Some(allocation)))
        })
        .transpose()?
        .unwrap_or_default();
    let yaml_ld = fields
        .raw("yaml_ld")?
        .map(|value| {
            let mut fields = Record::new_native(value, "YAML-LD input object", Some(allocation))?;
            let options = rdf_input::basic_fields(&mut fields, allocation)?;
            fields.deny_unknown()?;
            Ok::<_, DecodeError>(options)
        })
        .transpose()
        .map_err(|error| error.within_native("yaml_ld", Some(allocation)))?;
    let expansion = fields
        .raw("expansion")?
        .map(|value| {
            let mut expansion = JsonLdExpansionLimits::DEFAULT;
            let mut fields =
                Record::new_native(value, "expansion limits object", Some(allocation))?;
            expansion.max_allocation_bytes = json_config::field(
                &mut fields,
                "max_allocation_bytes",
                expansion.max_allocation_bytes,
                allocation,
            )?;
            fields.deny_unknown()?;
            Ok::<_, DecodeError>(expansion)
        })
        .transpose()
        .map_err(|error| error.within_native("expansion", Some(allocation)))?
        .unwrap_or(JsonLdExpansionLimits::DEFAULT);
    let output = fields
        .raw("output")?
        .map(|value| {
            let mut output = OutputLimits::DEFAULT;
            let mut fields = Record::new_native(
                value,
                "expanded JSON output limits object",
                Some(allocation),
            )?;
            output.max_bytes =
                json_config::field(&mut fields, "max_bytes", output.max_bytes, allocation)?;
            output.max_allocation_bytes = json_config::field(
                &mut fields,
                "max_allocation_bytes",
                output.max_allocation_bytes,
                allocation,
            )?;
            fields.deny_unknown()?;
            Ok::<_, DecodeError>(output)
        })
        .transpose()
        .map_err(|error| error.within_native("output", Some(allocation)))?
        .unwrap_or(OutputLimits::DEFAULT);
    fields.deny_unknown()?;
    Ok(Decoded {
        format,
        base,
        context,
        registry,
        yaml_ld,
        expansion,
        output,
    })
}

/// Decode the closed version-one request and compile its effective input context.
/// Request storage uses the existing bounded record account; registered documents
/// and context compilation retain their separate existing native authorities.
/// # Errors
/// Refuses unknown/duplicate/mistyped fields, wrappers, non-JSON-LD/YAML-LD formats,
/// invalid bases/contexts and original lexical/record/storage diagnostics.
pub fn request(bytes: &[u8]) -> Result<Request, RdfDiagnostic> {
    let limits = JsonLdContextLimits::DEFAULT;
    let decoded = json_config::decode(
        bytes,
        "document-expansion-request",
        rdf_input::MAX_OPTIONS_BYTES,
        rdf_input::MAX_OPTIONS_ALLOCATION_BYTES,
        json::Limits {
            max_depth: limits.max_nesting(),
            max_values: limits.max_expansion_work() as u64,
            max_string_bytes: rdf_input::MAX_OPTIONS_BYTES,
            unique_members: true,
        },
        "expansion request exceeds the native options byte limit",
        decode,
    )?;
    let format = classify(&decoded.format)?;
    if !matches!(format, NativeRdfFormat::JsonLd | NativeRdfFormat::YamlLd) {
        return Err(RdfDiagnostic::error(
            "document-expansion-format",
            "document expansion requires JSON-LD or YAML-LD input",
        ));
    }
    if format == NativeRdfFormat::JsonLd && decoded.yaml_ld.is_some() {
        return Err(RdfDiagnostic::error(
            "document-expansion-options",
            "yaml_ld controls require YAML-LD source input",
        ));
    }
    let registry = JsonLdContextRegistry::new_with_formats(decoded.registry)?;
    let context_supplied = decoded.context.is_some();
    let empty = Value::Object(Object::new());
    let context = CompiledJsonLdContext::compile_with_registry(
        decoded.context.as_ref().unwrap_or(&empty),
        decoded.base.as_deref(),
        &registry,
    )?;
    Ok(Request {
        format,
        context,
        context_supplied,
        yaml_ld: decoded.yaml_ld.unwrap_or_default(),
        expansion: decoded.expansion,
        output: decoded.output,
    })
}

/// Expand source bytes with a decoded request, returning compact expanded JSON text.
/// No dataset is constructed. Input decoding, expansion, context/compiler storage
/// and output/writer construction each retain their own separate authority.
/// # Errors
/// Preserves native source/expansion diagnostics and original storage causes;
/// refuses serialized bytes before the append that would exceed the ceiling.
pub fn expand_decoded(source: &[u8], request: &Request) -> Result<String, RdfDiagnostic> {
    let value = match request.format {
        NativeRdfFormat::JsonLd => {
            expand_jsonld_with_context(source, &request.context, &request.expansion)?
        }
        NativeRdfFormat::YamlLd => expand_yamlld_with_context(
            source,
            &request.context,
            &request.yaml_ld,
            &request.expansion,
        )?,
        _ => unreachable!("only request() constructs the private admitted format"),
    };
    let mut ceiling =
        Ceiling(usize::try_from(request.output.max_allocation_bytes).unwrap_or(usize::MAX));
    json::write_bounded_with_memory(
        &value,
        json::Format::COMPACT,
        request.output.max_bytes,
        &mut Memory::new(&mut ceiling),
    )
    .map_err(|error| {
        let storage = match error {
            json::WriteError::Storage(cause) => Some(cause),
            json::WriteError::Bytes { .. } => None,
        };
        json_config::failure("document-expansion-output", error, storage)
    })
}

/// Decode a request before parsing source, then use the shared structural body.
/// # Errors
/// Returns the original request, native expansion or bounded output diagnostic.
pub fn expand(source: &[u8], request_json: &[u8]) -> Result<String, RdfDiagnostic> {
    expand_decoded(source, &request(request_json)?)
}
