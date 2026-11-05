// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The bytes-to-Basic-value adapter shared by every native YAML-LD parse path.

use super::{CompiledJsonLdContext, RdfDataset, RdfDiagnostic};
use purrdf_lex::{
    json::Value,
    yaml::{
        self, ErrorKind, SyntaxKeyPolicy, SyntaxProjectionLimits, SyntaxRootPolicy,
        SyntaxSelection, SyntaxStreamLimits,
    },
};
use std::sync::Arc;

/// Native YAML-LD Basic selection and explicit admission controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YamlLdOptions {
    /// Construct every document into one aggregate JSON-LD value instead of the first.
    pub extract_all_scripts: bool,
    /// Whole-stream lexical admission, including unselected documents.
    pub syntax: SyntaxStreamLimits,
    /// Combined retained arena and selected semantic construction limits.
    pub projection: SyntaxProjectionLimits,
    /// Additional live bytes for escaped JSON output and writer scratch during
    /// string conversion. The already admitted Basic value is outside this
    /// account. Direct RDF parsing does not serialize or spend this allowance.
    pub max_serialization_bytes: usize,
}
impl YamlLdOptions {
    /// First-document construction; standard native limits and 1,048,576 documents.
    pub const DEFAULT: Self = Self {
        extract_all_scripts: false,
        syntax: SyntaxStreamLimits {
            syntax: yaml::SyntaxLimits::DEFAULT,
            max_documents: 1 << 20,
        },
        projection: SyntaxProjectionLimits::DEFAULT,
        max_serialization_bytes: 128 << 20,
    };
}
impl Default for YamlLdOptions {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Convert YAML-LD Basic bytes to JSON-LD JSON with explicit native controls.
/// # Errors
/// Returns positioned specification refusals or native resource refusals without partial output.
pub fn yamlld_to_jsonld_with_options(
    bytes: &[u8],
    options: &YamlLdOptions,
) -> Result<String, RdfDiagnostic> {
    use purrdf_lex::allocation::{Ceiling, Memory};
    let value = basic_value(bytes, options)?;
    let mut ceiling = Ceiling(options.max_serialization_bytes);
    let mut memory = Memory::new(&mut ceiling);
    purrdf_lex::json::write_with_memory(&value, purrdf_lex::json::Format::COMPACT, &mut memory)
        .map_err(|cause| {
            let mut diagnostic = RdfDiagnostic::from(cause);
            if cause == purrdf_lex::allocation::StorageError::AdmissionFailed {
                "native-yaml-output-limit".clone_into(&mut diagnostic.code);
                diagnostic.message = format!(
                    "JSON output and writer scratch exceed {} bytes",
                    options.max_serialization_bytes
                );
            }
            diagnostic
        })
}

/// Parse YAML-LD Basic directly through the existing JSON-LD value engine.
/// # Errors
/// Returns the original YAML profile, admission or JSON-LD diagnostic.
pub fn parse_yamlld_with_options(
    bytes: &[u8],
    base: Option<&str>,
    options: &YamlLdOptions,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let mut scope = crate::native_codecs::parse::base_scope_for(base)?;
    parse_into_scope(bytes, &mut scope, options)
}

/// Parse YAML-LD Basic with a caller-owned compiled active context.
///
/// The context supplies initial terms, the document base and immutable offline
/// context-IRI/import lookup. No network or filesystem lookup is attempted.
/// Options retain the same native admission bounds and first/all selection.
/// # Errors
/// Returns the original YAML profile, admission or JSON-LD diagnostic.
pub fn parse_yamlld_with_context(
    bytes: &[u8],
    context: &CompiledJsonLdContext,
    options: &YamlLdOptions,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    super::expand_to_dataset(basic_value(bytes, options)?, context)
}

pub(in crate::native_codecs) fn parse_into_scope(
    bytes: &[u8],
    base: &mut purrdf_iri::BaseScope,
    options: &YamlLdOptions,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    super::value_into_scope(basic_value(bytes, options)?, base)
}

fn basic_value(bytes: &[u8], options: &YamlLdOptions) -> Result<Value, RdfDiagnostic> {
    let limit = options.syntax.syntax.max_input_bytes;
    if bytes.len() > limit {
        let mut diagnostic =
            RdfDiagnostic::from(purrdf_lex::allocation::StorageError::AdmissionFailed);
        "native-yaml-input-limit".clone_into(&mut diagnostic.code);
        diagnostic.message = format!("YAML-LD input exceeds {limit} bytes");
        diagnostic.location = Some(Box::new(location(1, 1)));
        return Err(diagnostic);
    }
    let source = std::str::from_utf8(bytes).map_err(|error| {
        let valid = std::str::from_utf8(&bytes[..error.valid_up_to()])
            .expect("the UTF-8 error's valid prefix");
        let position = yaml::Position::at(valid, valid.len()).expect("the valid prefix EOF");
        RdfDiagnostic::error(
            "invalid-encoding",
            format!("YAML-LD input is not UTF-8: {error}"),
        )
        .with_location(location(position.line, position.column))
    })?;
    let stream = yaml::read_syntax_stream_with(source, options.syntax, SyntaxKeyPolicy::Deferred)
        .map_err(yaml_error)?;
    stream
        .into_core(
            if options.extract_all_scripts {
                SyntaxSelection::All
            } else {
                SyntaxSelection::First
            },
            SyntaxRootPolicy::CollectionsOrNull,
            options.projection,
        )
        .map_err(yaml_error)
}

fn location(line: usize, column: usize) -> purrdf_core::RdfLocation {
    purrdf_core::RdfLocation::default()
        .with_line(u64::try_from(line).unwrap_or(u64::MAX))
        .with_column(u32::try_from(column).unwrap_or(u32::MAX))
}

fn yaml_error(error: yaml::Error) -> RdfDiagnostic {
    let code = match error.kind() {
        ErrorKind::NonStringKey => "mapping-key-error",
        ErrorKind::NumericWork { .. } => "native-yaml-numeric-work-limit",
        ErrorKind::NumericOutput { .. } => "native-yaml-numeric-output-limit",
        ErrorKind::Allocation { .. } => "native-yaml-allocation-limit",
        ErrorKind::Storage(_) => "native-yaml-storage",
        ErrorKind::Nodes { .. } => "native-yaml-node-limit",
        ErrorKind::Depth { .. } => "native-yaml-depth-limit",
        ErrorKind::Input { .. } => "native-yaml-input-limit",
        ErrorKind::Documents { .. } => "native-yaml-document-limit",
        _ => "loading-document-failed",
    };
    let result = || {
        use purrdf_lex::allocation::{Memory, Resident};
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut diagnostic = RdfDiagnostic::try_error_with_memory(code, &error, &mut memory)?;
        diagnostic.location = Some(memory.boxed(location(error.line(), error.column()))?);
        let cause = purrdf_lex::allocation::retain_native_cause(error)?;
        diagnostic.with_native_cause(cause, &mut memory)
    };
    result().unwrap_or_else(RdfDiagnostic::from)
}
