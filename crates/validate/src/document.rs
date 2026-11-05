// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Closed structural document operations shared by every host, without RDF lowering.

use crate::{json_config, rdf_input};
use purrdf_core::RdfDiagnostic;
use purrdf_lex::allocation::{Ceiling, Memory};
use purrdf_lex::json::record::{DecodeError, FromJsonNative, Record, RecordAllocation, Within};
use purrdf_lex::json::{self, Object, Value};
use purrdf_lex::walk::WorkList;
use purrdf_rdf::native_codecs::jsonld::{
    CompiledJsonLdContext, JsonLdCompactionLimits, JsonLdCompactionPolicy, JsonLdContextLimits,
    JsonLdContextRegistry, JsonLdExpansionLimits, JsonLdFlattenLimits, YamlLdOptions,
    compact_expanded_with_context, expand_jsonld_document_with_context,
    expand_yamlld_document_with_context, flatten_expanded, jsonld_document_max_bytes,
};
use purrdf_rdf::native_codecs::{NativeRdfFormat, classify};
use std::cell::RefCell;

/// The three concrete structural source/request operations exposed by the hosts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Expand without node-map construction.
    Expand,
    /// Expand, then compact under an explicit output context.
    Compact,
    /// Expand and flatten, then compact only under a non-null output context.
    Flatten,
}
impl Operation {
    /// Operation noun used in host-facing diagnostics.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Expand => "expansion",
            Self::Compact => "compaction",
            Self::Flatten => "flattening",
        }
    }
    const fn request_code(self) -> &'static str {
        match self {
            Self::Expand => "document-expansion-request",
            Self::Compact => "document-compaction-request",
            Self::Flatten => "document-flattening-request",
        }
    }
    /// Bounded serialized-output refusal code, for host status classification.
    pub const fn output_code(self) -> &'static str {
        match self {
            Self::Expand => "document-expansion-output",
            Self::Compact => "document-compaction-output",
            Self::Flatten => "document-flattening-output",
        }
    }
    /// Portable byte-buffer length refusal code used by the WASM bridge.
    pub const fn byte_length_code(self) -> &'static str {
        match self {
            Self::Expand => "document-expansion-byte-length",
            Self::Compact => "document-compaction-byte-length",
            Self::Flatten => "document-flattening-byte-length",
        }
    }
    /// Source-capture limit refusal code used by the WASM bridge.
    pub const fn input_limit_code(self) -> &'static str {
        match self {
            Self::Expand => "document-expansion-input-limit",
            Self::Compact => "document-compaction-input-limit",
            Self::Flatten => "document-flattening-input-limit",
        }
    }
}

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
    operation: Operation,
    format: NativeRdfFormat,
    context: CompiledJsonLdContext,
    context_supplied: bool,
    yaml_ld: YamlLdOptions,
    expansion: JsonLdExpansionLimits,
    output: OutputLimits,
    original_base: Option<String>,
    action: Action,
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
    action: Action,
}

#[derive(Debug)]
enum OutputBase {
    Input,
    None,
    Iri(String),
}

#[derive(Debug)]
enum RetrievalBase {
    Original,
    None,
    Iri(String),
}

#[derive(Debug)]
struct Compaction {
    context: Option<Value>,
    retrieval_base: RetrievalBase,
    base: OutputBase,
    policy: JsonLdCompactionPolicy,
    limits: JsonLdCompactionLimits,
}

#[derive(Debug)]
enum Action {
    Expand,
    Compact(Compaction),
    Flatten(Compaction, JsonLdFlattenLimits),
}

fn bare_context(
    value: &Value,
    allocation: &dyn RecordAllocation,
    memory: &RefCell<Memory<'_, Ceiling>>,
    check_kinds: bool,
) -> Result<(), DecodeError> {
    let mut pending: WorkList<_, 8> = WorkList::new();
    pending
        .try_push_admitted(value, &mut memory.borrow_mut())
        .map_err(DecodeError::from_storage)?;
    while let Some(value) = pending.pop() {
        match value {
            Value::Bool(_) | Value::Number(_) if check_kinds => {
                return Err(DecodeError::custom_native(
                    "output context expressions require null, a string, an object or an array",
                    Some(allocation),
                ));
            }
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

fn compaction(
    fields: &mut Record<'_>,
    allocation: &dyn RecordAllocation,
    memory: &RefCell<Memory<'_, Ceiling>>,
    required: bool,
) -> Result<Compaction, DecodeError> {
    let context = fields.raw("output_context")?;
    if required && context.is_none() {
        return Err(DecodeError::missing_field_native(
            "output_context",
            Some(allocation),
        ));
    }
    let context = context
        .map(|value| {
            bare_context(value, allocation, memory, true)?;
            value
                .clone_with_memory(&mut memory.borrow_mut())
                .map_err(DecodeError::from_storage)
        })
        .transpose()
        .map_err(|error| error.within_native("output_context", Some(allocation)))?;
    let retrieval_base = fields
        .raw("output_context_base")?
        .map(|value| {
            if value.is_null() {
                Ok(RetrievalBase::None)
            } else {
                String::from_json_native(value, Some(allocation)).map(RetrievalBase::Iri)
            }
        })
        .transpose()
        .map_err(|error| error.within_native("output_context_base", Some(allocation)))?
        .unwrap_or(RetrievalBase::Original);
    let base = fields
        .raw("output_base")?
        .map(|value| {
            let mut fields = Record::new_native(value, "output base object", Some(allocation))?;
            let mode: String = fields.required_native("mode")?;
            let base = match mode.as_str() {
                "input" => OutputBase::Input,
                "none" => OutputBase::None,
                "iri" => OutputBase::Iri(fields.required_native("iri")?),
                _ => {
                    return Err(DecodeError::unknown_variant_native(
                        &mode,
                        &["input", "none", "iri"],
                        Some(allocation),
                    )
                    .within_native("mode", Some(allocation)));
                }
            };
            fields.deny_unknown()?;
            Ok::<_, DecodeError>(base)
        })
        .transpose()
        .map_err(|error| error.within_native("output_base", Some(allocation)))?
        .unwrap_or(OutputBase::Input);
    let (policy, limits) = fields
        .raw("compaction")?
        .map(|value| {
            let mut fields =
                Record::new_native(value, "compaction controls object", Some(allocation))?;
            let policy = JsonLdCompactionPolicy {
                compact_arrays: json_config::field(
                    &mut fields,
                    "compact_arrays",
                    JsonLdCompactionPolicy::DEFAULT.compact_arrays,
                    allocation,
                )?,
                compact_to_relative: json_config::field(
                    &mut fields,
                    "compact_to_relative",
                    JsonLdCompactionPolicy::DEFAULT.compact_to_relative,
                    allocation,
                )?,
            };
            let limits = JsonLdCompactionLimits {
                max_allocation_bytes: json_config::field(
                    &mut fields,
                    "max_allocation_bytes",
                    JsonLdCompactionLimits::DEFAULT.max_allocation_bytes,
                    allocation,
                )?,
            };
            fields.deny_unknown()?;
            Ok::<_, DecodeError>((policy, limits))
        })
        .transpose()
        .map_err(|error| error.within_native("compaction", Some(allocation)))?
        .unwrap_or((
            JsonLdCompactionPolicy::DEFAULT,
            JsonLdCompactionLimits::DEFAULT,
        ));
    Ok(Compaction {
        context,
        retrieval_base,
        base,
        policy,
        limits,
    })
}

fn validate_output_bases(action: &Action, operation: Operation) -> Result<(), RdfDiagnostic> {
    let compaction = match action {
        Action::Expand => return Ok(()),
        Action::Compact(compaction) | Action::Flatten(compaction, _) => compaction,
    };
    let explicit_base = match &compaction.base {
        OutputBase::Iri(iri) => Some(iri.as_str()),
        OutputBase::Input | OutputBase::None => None,
    };
    let retrieval_base = match &compaction.retrieval_base {
        RetrievalBase::Iri(iri) => Some(iri.as_str()),
        RetrievalBase::Original | RetrievalBase::None => None,
    };
    for iri in [retrieval_base, explicit_base].into_iter().flatten() {
        purrdf_iri::BaseIri::parse(iri)
            .map_err(|cause| json_config::failure(operation.request_code(), cause, None))?;
    }
    Ok(())
}

fn compact(
    value: &Value,
    effective_base: Option<&str>,
    request: &Request,
    compaction: &Compaction,
) -> Result<Value, RdfDiagnostic> {
    let base = match &compaction.base {
        OutputBase::Input => effective_base,
        OutputBase::None => None,
        OutputBase::Iri(iri) => Some(iri.as_str()),
    };
    let retrieval = match &compaction.retrieval_base {
        RetrievalBase::Original => request.original_base.as_deref(),
        RetrievalBase::None => None,
        RetrievalBase::Iri(iri) => Some(iri.as_str()),
    };
    let output = CompiledJsonLdContext::compile_with_bases(
        compaction
            .context
            .as_ref()
            .expect("compact requires a context; flatten compacts only a non-null expression"),
        retrieval,
        base,
        request.context.registry(),
    )?;
    compact_expanded_with_context(value, &output, &compaction.policy, &compaction.limits)
}

fn decode(
    value: &Value,
    allocation: &dyn RecordAllocation,
    memory: &RefCell<Memory<'_, Ceiling>>,
    operation: Operation,
) -> Result<Decoded, DecodeError> {
    let description = match operation {
        Operation::Expand => "document expansion request",
        Operation::Compact => "document compaction request",
        Operation::Flatten => "document flattening request",
    };
    let mut fields = Record::new_native(value, description, Some(allocation))?;
    let version: u32 = fields.required_native("version")?;
    if version != 1 {
        return Err(DecodeError::custom_native(
            format_args!("{} request version must be 1", operation.name()),
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
            bare_context(value, allocation, memory, false)
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
    let action = match operation {
        Operation::Expand => Action::Expand,
        Operation::Compact => Action::Compact(compaction(&mut fields, allocation, memory, true)?),
        Operation::Flatten => {
            let compaction = compaction(&mut fields, allocation, memory, false)?;
            let flattening = fields
                .raw("flattening")?
                .map(|value| {
                    let mut fields =
                        Record::new_native(value, "flattening limits object", Some(allocation))?;
                    let limits = JsonLdFlattenLimits {
                        max_allocation_bytes: json_config::field(
                            &mut fields,
                            "max_allocation_bytes",
                            JsonLdFlattenLimits::DEFAULT.max_allocation_bytes,
                            allocation,
                        )?,
                    };
                    fields.deny_unknown()?;
                    Ok::<_, DecodeError>(limits)
                })
                .transpose()
                .map_err(|error| error.within_native("flattening", Some(allocation)))?
                .unwrap_or(JsonLdFlattenLimits::DEFAULT);
            Action::Flatten(compaction, flattening)
        }
    };
    fields.deny_unknown()?;
    Ok(Decoded {
        format,
        base,
        context,
        registry,
        yaml_ld,
        expansion,
        output,
        action,
    })
}

/// Decode the closed version-one request and compile its effective input context.
/// Request storage uses the existing bounded record account; registered documents
/// and context compilation retain their separate existing native authorities.
/// # Errors
/// Refuses unknown/duplicate/mistyped fields, wrappers, non-JSON-LD/YAML-LD formats,
/// invalid bases/contexts and original lexical/record/storage diagnostics.
pub fn request(bytes: &[u8], operation: Operation) -> Result<Request, RdfDiagnostic> {
    let limits = JsonLdContextLimits::DEFAULT;
    let decoded = json_config::decode(
        bytes,
        operation.request_code(),
        rdf_input::MAX_OPTIONS_BYTES,
        rdf_input::MAX_OPTIONS_ALLOCATION_BYTES,
        json::Limits {
            max_depth: limits.max_nesting(),
            max_values: limits.max_expansion_work() as u64,
            max_string_bytes: rdf_input::MAX_OPTIONS_BYTES,
            unique_members: true,
        },
        match operation {
            Operation::Expand => "expansion request exceeds the native options byte limit",
            Operation::Compact => "compaction request exceeds the native options byte limit",
            Operation::Flatten => "flattening request exceeds the native options byte limit",
        },
        |value, allocation, memory| decode(value, allocation, memory, operation),
    )?;
    let format = classify(&decoded.format)?;
    if !matches!(format, NativeRdfFormat::JsonLd | NativeRdfFormat::YamlLd) {
        return Err(RdfDiagnostic::error(
            match operation {
                Operation::Expand => "document-expansion-format",
                Operation::Compact => "document-compaction-format",
                Operation::Flatten => "document-flattening-format",
            },
            format!(
                "document {} requires JSON-LD or YAML-LD input",
                operation.name()
            ),
        ));
    }
    if format == NativeRdfFormat::JsonLd && decoded.yaml_ld.is_some() {
        return Err(RdfDiagnostic::error(
            match operation {
                Operation::Expand => "document-expansion-options",
                Operation::Compact => "document-compaction-options",
                Operation::Flatten => "document-flattening-options",
            },
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
    validate_output_bases(&decoded.action, operation)?;
    Ok(Request {
        operation,
        format,
        context,
        context_supplied,
        yaml_ld: decoded.yaml_ld.unwrap_or_default(),
        expansion: decoded.expansion,
        output: decoded.output,
        original_base: decoded.base,
        action: decoded.action,
    })
}

/// Execute the decoded structural operation and return compact JSON text.
/// No dataset is constructed. Input decoding, each construction phase,
/// context/compiler storage and output/writer construction keep separate authority.
/// # Errors
/// Preserves native source/phase diagnostics and original storage causes;
/// refuses serialized bytes before the append that would exceed the ceiling.
pub fn execute_decoded(source: &[u8], request: &Request) -> Result<String, RdfDiagnostic> {
    let expanded = match request.format {
        NativeRdfFormat::JsonLd => {
            expand_jsonld_document_with_context(source, &request.context, &request.expansion)?
        }
        NativeRdfFormat::YamlLd => expand_yamlld_document_with_context(
            source,
            &request.context,
            &request.yaml_ld,
            &request.expansion,
        )?,
        _ => unreachable!("only request() constructs the private admitted format"),
    };
    let value = match &request.action {
        Action::Expand => expanded.value,
        Action::Compact(compaction) => compact(
            &expanded.value,
            expanded.document_base.as_deref(),
            request,
            compaction,
        )?,
        Action::Flatten(compaction, limits) => {
            let flattened = flatten_expanded(&expanded.value, limits)?;
            if compaction
                .context
                .as_ref()
                .is_some_and(|value| !value.is_null())
            {
                compact(
                    &flattened,
                    expanded.document_base.as_deref(),
                    request,
                    compaction,
                )?
            } else {
                flattened
            }
        }
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
        json_config::failure(request.operation.output_code(), error, storage)
    })
}

/// Decode the operation's closed request before parsing and transforming source.
/// # Errors
/// Returns original request, native construction and bounded-output diagnostics.
pub fn execute(
    source: &[u8],
    request_json: &[u8],
    operation: Operation,
) -> Result<String, RdfDiagnostic> {
    execute_decoded(source, &request(request_json, operation)?)
}
