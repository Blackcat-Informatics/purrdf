// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native SSSOM (Simple Standard for Sharing Ontology Mappings) codec.
//!
//! This is the PyO3-free Rust replacement for the `sssom` PyPI package's
//! parse + validate behaviour (#848). It carries the PurRDF mapping artifacts
//! (`generated/mappings/*.sssom.tsv`) in and out of an owned IR and adds a
//! native RDF serializer the Python toolkit never had (the SUBSUME/ENHANCE
//! deliverable).
//!
//! ## The SSSOM TSV format (as PurRDF writes it)
//!
//! A PurRDF SSSOM file is a YAML-ish `#`-prefixed metadata header followed by a
//! tab-separated mapping table:
//!
//! ```text
//! # mapping_set_id: https://…/mappings/accessibility
//! # mapping_set_version: 0.1.0
//! # license: https://creativecommons.org/licenses/by/4.0/
//! # comment: "Accessibility alignments …"
//! # curie_map:
//! #   ex: https://example.org/vocab/
//! #   skos:  http://www.w3.org/2004/02/skos/core#
//! subject_id  predicate_id  object_id  mapping_justification  confidence  comment
//! ex:Foo  skos:closeMatch  sosa:Observation  semapv:ManualMappingCuration  0.7  …
//! # # REFUSED … (trailer provenance — ignored, not a mapping)
//! ```
//!
//! (The column header and data rows are tab-separated in the real file; spaces
//! stand in here only because doc comments forbid raw tabs.)
//!
//! Scalar header lines are `# key: value`; the nested `# curie_map:` block is a
//! set of indented `#   prefix: uri` lines. PurRDF *owns* this format, so the
//! header is parsed bespoke — no YAML library. (The Python `_sssom_for_validation`
//! shim only existed to satisfy sssom-py's YAML reader; the native codec needs no
//! such crutch.) Trailing `# #…` provenance comments after the TSV header row are
//! deliberately ignored — they are documentation, not mappings.
//!
//! ## Parity intent
//!
//! [`validate`] reproduces the *reachable* default checks of sssom-py 0.4.x
//! (`PrefixMapCompleteness` + `JsonSchema`; see
//! `tests/fixtures/lint-golden/sssom_validation.json`) and then deliberately
//! *extends* them with a `RequiredSlot` check. sssom-py silently drops rows with a
//! missing required slot during parsing, so they never reach its validators and
//! produce no diagnostic; that is a parity *gap in sssom-py's favour of silence*,
//! not a behaviour worth preserving (CONSTITUTION / no-compromises). The native
//! validator is a strict SUPERSET: every committed corpus file is clean under both
//! the parity checks and the enhancement, so flagging the missing-slot negatives
//! is an improvement with no corpus regression.

use std::collections::{BTreeMap, BTreeSet};

use crate::{RdfDiagnostic, RdfLiteral, RdfLocation, RdfQuad, RdfSeverity, RdfTerm};

// --------------------------------------------------------------------------- //
// Vocabulary
// --------------------------------------------------------------------------- //

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const OWL_AXIOM: &str = "http://www.w3.org/2002/07/owl#Axiom";
const OWL_ANNOTATED_SOURCE: &str = "http://www.w3.org/2002/07/owl#annotatedSource";
const OWL_ANNOTATED_PROPERTY: &str = "http://www.w3.org/2002/07/owl#annotatedProperty";
const OWL_ANNOTATED_TARGET: &str = "http://www.w3.org/2002/07/owl#annotatedTarget";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";

/// The `https://w3id.org/sssom/` metadata namespace.
const SSSOM_NS: &str = "https://w3id.org/sssom/";

/// The default check set sssom-py runs (`validation_types=None`), captured from
/// `sssom.validators.DEFAULT_VALIDATION_TYPES` into the frozen golden. The native
/// validator implements the two *reachable-through-parse* checks of this set
/// (`PrefixMapCompleteness`, `JsonSchema`); `StrictCurieFormat` is unreachable
/// because PurRDF never emits a pipe-bearing entity slot. Exposed so a consumer can
/// report the parity surface it covers.
pub const SSSOM_DEFAULT_VALIDATION_TYPES: &[&str] =
    &["JsonSchema", "PrefixMapCompleteness", "StrictCurieFormat"];

/// The canonical SSSOM column order PurRDF writes.
const SSSOM_ORDER: &[&str] = &[
    "subject_id",
    "subject_label",
    "predicate_id",
    "object_id",
    "object_label",
    "mapping_justification",
    "confidence",
    "comment",
];

/// The columns PurRDF always emits, even when blank for every row.
const SSSOM_ALWAYS: &[&str] = &[
    "subject_id",
    "predicate_id",
    "object_id",
    "mapping_justification",
    "confidence",
    "comment",
];

// --------------------------------------------------------------------------- //
// IR types
// --------------------------------------------------------------------------- //

/// The SSSOM metadata header.
///
/// The named scalars cover everything PurRDF writes; any other `# key: value`
/// scalar is preserved verbatim in [`extra`](SssomMeta::extra) so nothing is lost
/// (maximal information flow). The `comment` value is kept *as authored* — PurRDF
/// JSON-quotes it (`"…—…"`), which this codec round-trips byte-for-byte
/// rather than re-interpreting.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SssomMeta {
    pub mapping_set_id: Option<String>,
    pub mapping_set_version: Option<String>,
    pub license: Option<String>,
    pub mapping_tool: Option<String>,
    pub mapping_tool_version: Option<String>,
    pub mapping_date: Option<String>,
    pub comment: Option<String>,
    /// `prefix → namespace-IRI` for every declared CURIE prefix.
    pub curie_map: BTreeMap<String, String>,
    /// Any header scalar outside the named set, preserved losslessly.
    pub extra: BTreeMap<String, String>,
}

/// A single SSSOM mapping (one TSV data row).
///
/// The named columns cover PurRDF's [`SSSOM_ORDER`]; any other column is preserved
/// verbatim in [`extras`](SssomMapping::extras) so a non-PurRDF SSSOM file still
/// round-trips losslessly.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SssomMapping {
    pub subject_id: String,
    pub subject_label: Option<String>,
    pub predicate_id: String,
    pub object_id: String,
    pub object_label: Option<String>,
    pub mapping_justification: String,
    pub confidence: Option<f64>,
    pub comment: Option<String>,
    /// Any column outside the named set, in declaration order.
    pub extras: BTreeMap<String, String>,
}

/// A parsed SSSOM mapping set: header metadata + its mappings.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SssomMappingSet {
    pub meta: SssomMeta,
    pub mappings: Vec<SssomMapping>,
}

/// A single validation diagnostic, mirroring the sssom-py golden record shape
/// `{severity, type, message, instance, check}`. `code` carries the golden's
/// `type` string; `check` carries the originating check name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SssomDiagnostic {
    pub severity: RdfSeverity,
    pub code: String,
    pub message: String,
    pub instance: Option<String>,
    pub check: String,
}

impl SssomDiagnostic {
    fn error(
        code: impl Into<String>,
        check: impl Into<String>,
        message: impl Into<String>,
        instance: Option<String>,
    ) -> Self {
        Self {
            severity: RdfSeverity::Error,
            code: code.into(),
            message: message.into(),
            instance,
            check: check.into(),
        }
    }
}

// --------------------------------------------------------------------------- //
// Parsing
// --------------------------------------------------------------------------- //

/// Parse a PurRDF SSSOM TSV document into an owned [`SssomMappingSet`].
///
/// The `#`-prefixed header (scalars + the nested `curie_map:` block) is parsed
/// bespoke; the TSV body is parsed with the `csv` crate (tab delimiter, flexible
/// column counts, by-name access). Unknown header scalars and unknown columns are
/// preserved verbatim. Trailing `# #…` provenance comments after the TSV header
/// row are ignored.
///
/// Returns `Err(RdfDiagnostic)` only on structurally unparsable input (no TSV
/// column-header row, a malformed `# key` line, or a body the TSV reader rejects):
/// semantic defects (unknown prefix, bad confidence, missing slot) are surfaced by
/// [`validate`], not here, so a caller can ingest-then-validate the way sssom-py's
/// `parse_tsv` → `validate` split does.
pub fn parse_tsv(text: &str) -> Result<SssomMappingSet, RdfDiagnostic> {
    let lines: Vec<&str> = text.lines().collect();

    // Locate the TSV column-header row: the first line that does not start with
    // '#'. Everything before it is the metadata header.
    let header_idx = lines.iter().position(|line| !line.starts_with('#'));
    let Some(header_idx) = header_idx else {
        return Err(RdfDiagnostic::error(
            "sssom-tsv-parse",
            "no TSV column-header row found (every line is a `#` comment)",
        ));
    };

    let meta = parse_header(&lines[..header_idx])?;

    // The body is the column-header row plus the data rows. `#` provenance
    // comments (PurRDF emits its trailer block right after the column header when
    // there are zero rows, or interleaved) and blank lines are not mappings; skip
    // them while recording each kept line's original 1-based source line number,
    // so a diagnostic reports the true line even when a comment is interleaved
    // between data rows (passing a flat offset shifted every later row's line).
    let mut body = String::new();
    let mut line_numbers: Vec<u32> = Vec::new();
    for (offset, line) in lines[header_idx..].iter().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        line_numbers.push((header_idx + offset + 1) as u32);
        body.push_str(line);
        body.push('\n');
    }

    let mappings = parse_body(&body, &line_numbers)?;
    Ok(SssomMappingSet { meta, mappings })
}

/// Parse the `#`-prefixed metadata header lines into an [`SssomMeta`].
fn parse_header(lines: &[&str]) -> Result<SssomMeta, RdfDiagnostic> {
    let mut meta = SssomMeta::default();
    let mut in_curie_map = false;

    for (offset, raw) in lines.iter().enumerate() {
        let line_no = (offset + 1) as u32;
        // Strip the leading '#'. A bare '#' line is an empty comment; skip it.
        let body = raw.strip_prefix('#').unwrap_or(raw);
        if body.trim().is_empty() {
            continue;
        }
        // A `# #…` line is a trailer/provenance comment (the second '#' makes it
        // YAML-invisible). PurRDF emits these
        // after the curie_map block but still inside the leading `#` region; they
        // are never header scalars or curie entries, so skip them outright (they
        // do NOT close an open curie_map block — more curie entries can follow in
        // principle, though PurRDF always writes the trailer last).
        if body.trim_start().starts_with('#') {
            continue;
        }

        // `#   prefix: uri` (two+ leading spaces) inside a `curie_map:` block.
        // The block is "open" from the `# curie_map:` line until a non-indented
        // scalar (or end of header). PurRDF indents curie entries with three
        // spaces; accept any indent of one or more.
        let is_indented = body.starts_with(' ') && body.trim_start() != body;
        if in_curie_map && is_indented {
            let entry = body.trim();
            let (prefix, uri) = split_key_value(entry).ok_or_else(|| {
                RdfDiagnostic::error(
                    "sssom-tsv-parse",
                    format!("malformed curie_map entry: {entry:?}"),
                )
                .with_location(RdfLocation::default().with_line(line_no))
            })?;
            meta.curie_map.insert(prefix.to_owned(), uri.to_owned());
            continue;
        }

        // A non-indented line ends any open curie_map block.
        in_curie_map = false;

        let scalar = body.trim_start();
        let (key, value) = split_key_value(scalar).ok_or_else(|| {
            RdfDiagnostic::error(
                "sssom-tsv-parse",
                format!("malformed header line: {scalar:?}"),
            )
            .with_location(RdfLocation::default().with_line(line_no))
        })?;

        match key {
            "curie_map" => {
                // The nested-block opener: `# curie_map:` with an empty value.
                // A non-empty inline value (e.g. the broken `[unclosed` fixture)
                // is a malformed flow sequence PurRDF never emits — reject it the
                // way sssom-py's YAML reader does (it raised a FATAL parse error).
                if !value.is_empty() {
                    return Err(RdfDiagnostic::error(
                        "sssom-tsv-parse",
                        format!("inline curie_map value is not supported: {value:?}"),
                    )
                    .with_location(RdfLocation::default().with_line(line_no)));
                }
                in_curie_map = true;
            }
            "mapping_set_id" => meta.mapping_set_id = Some(value.to_owned()),
            "mapping_set_version" => meta.mapping_set_version = Some(value.to_owned()),
            "license" => meta.license = Some(value.to_owned()),
            "mapping_tool" => meta.mapping_tool = Some(value.to_owned()),
            "mapping_tool_version" => meta.mapping_tool_version = Some(value.to_owned()),
            "mapping_date" => meta.mapping_date = Some(value.to_owned()),
            "comment" => meta.comment = Some(value.to_owned()),
            other => {
                meta.extra.insert(other.to_owned(), value.to_owned());
            }
        }
    }
    Ok(meta)
}

/// Split a `key: value` scalar at the first `": "` (or a trailing `":"`).
///
/// Returns `None` if there is no colon at all. The value is the remainder
/// verbatim (no YAML unquoting — PurRDF owns the format and the comment value is
/// kept as authored for byte-stable round-trips).
fn split_key_value(scalar: &str) -> Option<(&str, &str)> {
    let idx = scalar.find(':')?;
    let key = scalar[..idx].trim();
    if key.is_empty() {
        return None;
    }
    // Skip the colon and a single following space if present.
    let rest = &scalar[idx + 1..];
    let value = rest.strip_prefix(' ').unwrap_or(rest);
    Some((key, value.trim_end()))
}

/// Parse the TSV body (column-header row + data rows) into mappings.
///
/// `line_numbers` holds the original 1-based source line of each body line in
/// order: `line_numbers[0]` is the column-header row, and data row `i` is at
/// `line_numbers[i + 1]`. This keeps diagnostics anchored to the true source
/// position regardless of how many `#` comment lines were filtered out upstream.
fn parse_body(body: &str, line_numbers: &[u32]) -> Result<Vec<SssomMapping>, RdfDiagnostic> {
    if body.trim().is_empty() {
        return Err(RdfDiagnostic::error(
            "sssom-tsv-parse",
            "missing TSV column-header row",
        ));
    }

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(true)
        .has_headers(true)
        .from_reader(body.as_bytes());

    // `csv` validates the header lazily; fetch it up front so a malformed header
    // is reported with the right line (the first body line).
    let header_line = line_numbers.first().copied().unwrap_or(1);
    let columns: Vec<String> = reader
        .headers()
        .map_err(|e| {
            RdfDiagnostic::error("sssom-tsv-parse", format!("unreadable TSV header: {e}"))
                .with_location(RdfLocation::default().with_line(header_line))
        })?
        .iter()
        .map(str::to_owned)
        .collect();

    let mut mappings = Vec::new();
    for (row_index, record) in reader.records().enumerate() {
        // The data row's true 1-based source line: line_numbers[0] is the column
        // header, so data row `row_index` is at line_numbers[row_index + 1].
        let line_no = line_numbers
            .get(row_index + 1)
            .copied()
            .unwrap_or(header_line);
        let record = record.map_err(|e| {
            RdfDiagnostic::error("sssom-tsv-parse", format!("malformed TSV row: {e}"))
                .with_location(RdfLocation::default().with_line(line_no))
        })?;
        mappings.push(parse_row(&columns, &record, line_no)?);
    }
    Ok(mappings)
}

/// Map one TSV record onto a [`SssomMapping`] by column name.
fn parse_row(
    columns: &[String],
    record: &csv::StringRecord,
    line_no: u32,
) -> Result<SssomMapping, RdfDiagnostic> {
    let mut mapping = SssomMapping::default();
    for (col, value) in columns.iter().zip(record.iter()) {
        let value = value.to_owned();
        match col.as_str() {
            "subject_id" => mapping.subject_id = value,
            "subject_label" => mapping.subject_label = non_empty(value),
            "predicate_id" => mapping.predicate_id = value,
            "object_id" => mapping.object_id = value,
            "object_label" => mapping.object_label = non_empty(value),
            "mapping_justification" => mapping.mapping_justification = value,
            "confidence" => {
                if !value.is_empty() {
                    let parsed = value.parse::<f64>().map_err(|_| {
                        RdfDiagnostic::error(
                            "sssom-tsv-parse",
                            format!("non-numeric confidence: {value:?}"),
                        )
                        .with_location(RdfLocation::default().with_line(line_no))
                    })?;
                    mapping.confidence = Some(parsed);
                }
            }
            "comment" => mapping.comment = non_empty(value),
            other => {
                mapping.extras.insert(other.to_owned(), value);
            }
        }
    }
    Ok(mapping)
}

/// `None` for an empty string, `Some(value)` otherwise.
fn non_empty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

// --------------------------------------------------------------------------- //
// Validation
// --------------------------------------------------------------------------- //

/// The required entity slots of a mapping (SSSOM core).
const REQUIRED_SLOTS: &[&str] = &[
    "subject_id",
    "predicate_id",
    "object_id",
    "mapping_justification",
];

/// Validate a mapping set, returning every ERROR-severity diagnostic.
///
/// Reproduces the *reachable* default sssom-py checks and extends them:
///
/// * **`PrefixMapCompleteness`** — every CURIE prefix used in `subject_id`,
///   `predicate_id`, `object_id`, or `mapping_justification` must be declared in
///   the `curie_map`. An undeclared prefix is an ERROR (`Missing prefix: <pfx>`).
/// * **`JsonSchema`** — `confidence`, when present, must lie in `[0.0, 1.0]`;
///   out-of-range is an ERROR (matching sssom-py's JSON-schema min/max).
/// * **`RequiredSlot`** *(enhancement)* — every required entity slot
///   (`subject_id`, `predicate_id`, `object_id`, `mapping_justification`) must be
///   present and non-empty. sssom-py *silently drops* such rows during parsing, so
///   they never reach its validators; the native codec flags them instead. This is
///   a deliberate strict-superset improvement, NOT a parity regression — every
///   committed corpus file populates all four slots, so the clean corpus is
///   unaffected.
///
/// The mapping-justification slot is checked for prefix completeness but its CURIE
/// shape is otherwise left to `RequiredSlot`/downstream: a non-CURIE justification
/// (no `:`) carries no prefix, so it neither trips nor satisfies the prefix check —
/// matching the corpus, where the justification is always a `semapv:` CURIE.
pub fn validate(set: &SssomMappingSet) -> Vec<SssomDiagnostic> {
    let mut diagnostics = Vec::new();

    for mapping in &set.mappings {
        validate_required_slots(mapping, &mut diagnostics);
        validate_prefixes(mapping, &set.meta, &mut diagnostics);
        validate_confidence(mapping, &mut diagnostics);
    }

    diagnostics
}

/// ENHANCEMENT check: every required slot is present and non-empty.
fn validate_required_slots(mapping: &SssomMapping, out: &mut Vec<SssomDiagnostic>) {
    let slots = [
        ("subject_id", mapping.subject_id.as_str()),
        ("predicate_id", mapping.predicate_id.as_str()),
        ("object_id", mapping.object_id.as_str()),
        (
            "mapping_justification",
            mapping.mapping_justification.as_str(),
        ),
    ];
    for (name, value) in slots {
        debug_assert!(REQUIRED_SLOTS.contains(&name));
        if value.trim().is_empty() {
            out.push(SssomDiagnostic::error(
                "required slot",
                "RequiredSlot",
                format!("Missing required slot: {name}"),
                mapping_instance(mapping),
            ));
        }
    }
}

/// Parity check: every CURIE prefix used is declared in the curie_map.
fn validate_prefixes(mapping: &SssomMapping, meta: &SssomMeta, out: &mut Vec<SssomDiagnostic>) {
    let entities = [
        mapping.subject_id.as_str(),
        mapping.predicate_id.as_str(),
        mapping.object_id.as_str(),
        mapping.mapping_justification.as_str(),
    ];
    for entity in entities {
        let Some(prefix) = curie_prefix(entity) else {
            continue;
        };
        if !meta.curie_map.contains_key(prefix) {
            out.push(SssomDiagnostic::error(
                "prefix validation",
                "PrefixMapCompleteness",
                format!("Missing prefix: {prefix}"),
                mapping_instance(mapping),
            ));
        }
    }
}

/// Parity check: `confidence`, when present, lies in `[0.0, 1.0]`.
fn validate_confidence(mapping: &SssomMapping, out: &mut Vec<SssomDiagnostic>) {
    if let Some(confidence) = mapping.confidence {
        if confidence < 0.0 {
            out.push(SssomDiagnostic::error(
                "jsonschema validation",
                "JsonSchema",
                format!("{confidence} is less than the minimum of 0.0"),
                mapping_instance(mapping),
            ));
        } else if confidence > 1.0 {
            out.push(SssomDiagnostic::error(
                "jsonschema validation",
                "JsonSchema",
                format!("{confidence} is greater than the maximum of 1.0"),
                mapping_instance(mapping),
            ));
        }
    }
}

/// The CURIE prefix of an entity reference, or `None` if it is not a CURIE.
///
/// A CURIE is `prefix:reference` with a non-empty prefix that is *not* a scheme of
/// an absolute IRI (`http://`, `https://`, …). PurRDF writes bare absolute URIs for
/// unregistered namespaces, which must not be mistaken for a `http`/`https` CURIE
/// prefix.
fn curie_prefix(entity: &str) -> Option<&str> {
    let idx = entity.find(':')?;
    let prefix = &entity[..idx];
    if prefix.is_empty() {
        return None;
    }
    // An absolute IRI: `prefix` is a scheme and the reference starts with `//`.
    if entity[idx + 1..].starts_with("//") {
        return None;
    }
    Some(prefix)
}

/// The instance handle a diagnostic points at: the offending row's `subject_id`
/// (or its first non-empty entity if the subject is itself missing), or `None`.
fn mapping_instance(mapping: &SssomMapping) -> Option<String> {
    for candidate in [
        mapping.subject_id.as_str(),
        mapping.object_id.as_str(),
        mapping.predicate_id.as_str(),
    ] {
        if !candidate.is_empty() {
            return Some(candidate.to_owned());
        }
    }
    None
}

// --------------------------------------------------------------------------- //
// TSV serialization
// --------------------------------------------------------------------------- //

/// Serialize a mapping set back to canonical PurRDF SSSOM TSV.
///
/// Emits the metadata header, then the `_SSSOM_ORDER` columns (always-on columns
/// plus any optional column some row populates), then rows sorted by
/// `(subject_id, predicate_id, object_id)` — matching the PurRDF SSSOM writer
/// closely enough that `parse ∘ serialize_tsv` is stable (round-trip-stable). IDs
/// are kept as authored (CURIEs are not expanded).
pub fn serialize_tsv(set: &SssomMappingSet) -> String {
    let mut lines: Vec<String> = Vec::new();
    let meta = &set.meta;

    push_scalar(&mut lines, "mapping_set_id", meta.mapping_set_id.as_ref());
    push_scalar(
        &mut lines,
        "mapping_set_version",
        meta.mapping_set_version.as_ref(),
    );
    push_scalar(&mut lines, "license", meta.license.as_ref());
    push_scalar(&mut lines, "mapping_tool", meta.mapping_tool.as_ref());
    push_scalar(
        &mut lines,
        "mapping_tool_version",
        meta.mapping_tool_version.as_ref(),
    );
    push_scalar(&mut lines, "mapping_date", meta.mapping_date.as_ref());
    push_scalar(&mut lines, "comment", meta.comment.as_ref());
    for (key, value) in &meta.extra {
        lines.push(format!("# {key}: {value}"));
    }
    if !meta.curie_map.is_empty() {
        lines.push("# curie_map:".to_owned());
        for (prefix, uri) in &meta.curie_map {
            lines.push(format!("#   {prefix}: {uri}"));
        }
    }

    // Choose the emitted columns: always-on columns, plus any optional column
    // that at least one row populates (mirrors `_SSSOM_ALWAYS`/`_SSSOM_ORDER`).
    let mut columns: Vec<&str> = SSSOM_ORDER
        .iter()
        .copied()
        .filter(|col| {
            SSSOM_ALWAYS.contains(col) || set.mappings.iter().any(|m| !cell(m, col).is_empty())
        })
        .collect();
    // Append any preserved extra (non-SSSOM-core) columns captured on parse so a
    // `parse → serialize_tsv` round-trip is lossless for non-PurRDF inputs. The
    // union of `extras` keys across rows is sorted (BTreeSet) for deterministic
    // output; these keys are disjoint from SSSOM_ORDER by construction (parse_row
    // routes only unknown columns into `extras`).
    let extra_columns: BTreeSet<&str> = set
        .mappings
        .iter()
        .flat_map(|m| m.extras.keys().map(String::as_str))
        .collect();
    columns.extend(extra_columns);
    lines.push(columns.join("\t"));

    let mut sorted: Vec<&SssomMapping> = set.mappings.iter().collect();
    sorted.sort_by(|a, b| {
        (&a.subject_id, &a.predicate_id, &a.object_id).cmp(&(
            &b.subject_id,
            &b.predicate_id,
            &b.object_id,
        ))
    });
    for mapping in sorted {
        let row: Vec<String> = columns.iter().map(|col| cell(mapping, col)).collect();
        lines.push(row.join("\t"));
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Push a `# key: value` header line when the scalar is present.
fn push_scalar(lines: &mut Vec<String>, key: &str, value: Option<&String>) {
    if let Some(value) = value {
        lines.push(format!("# {key}: {value}"));
    }
}

/// The serialized TSV cell for a named column, including any preserved extra.
fn cell(mapping: &SssomMapping, column: &str) -> String {
    match column {
        "subject_id" => mapping.subject_id.clone(),
        "subject_label" => mapping.subject_label.clone().unwrap_or_default(),
        "predicate_id" => mapping.predicate_id.clone(),
        "object_id" => mapping.object_id.clone(),
        "object_label" => mapping.object_label.clone().unwrap_or_default(),
        "mapping_justification" => mapping.mapping_justification.clone(),
        "confidence" => mapping
            .confidence
            .map(format_confidence)
            .unwrap_or_default(),
        "comment" => mapping.comment.clone().unwrap_or_default(),
        other => mapping.extras.get(other).cloned().unwrap_or_default(),
    }
}

/// Format a confidence as the shortest round-trip-stable decimal (`0.7`, not
/// `0.70`), matching the PurRDF writer.
fn format_confidence(value: f64) -> String {
    // Rust's default float formatting already drops trailing zeros and prints the
    // shortest decimal that round-trips, so `0.7_f64` → "0.7" and `1.0` → "1".
    // Preserve a decimal point for whole confidences so the column stays numeric.
    let text = value.to_string();
    if text.contains('.') {
        text
    } else {
        format!("{text}.0")
    }
}

// --------------------------------------------------------------------------- //
// RDF serialization (SUBSUME / ENHANCE — a capability sssom-py lacked here)
// --------------------------------------------------------------------------- //

/// Serialize a mapping set to the SSSOM RDF mapping-set form (owned quads).
///
/// Each mapping is emitted as a reified OWL axiom node (`a owl:Axiom`) with
/// `owl:annotatedSource`/`owl:annotatedProperty`/`owl:annotatedTarget` carrying
/// the (CURIE-resolved) subject/predicate/object, plus the SSSOM metadata
/// predicates `sssom:mapping_justification`, `sssom:confidence`, and
/// `sssom:comment` under the `https://w3id.org/sssom/` namespace. Confidence is an
/// `xsd:double` literal; everything else is an IRI or plain literal.
///
/// CURIEs are resolved through `meta.curie_map`; an already-absolute IRI is used
/// as-is. The reifier node is a blank node, one per mapping. This RDF form is a
/// NEW capability (the Python toolkit had no equivalent here) and is not under any
/// byte-parity gate; it is well-formed and tested but matches no committed file.
pub fn to_rdf(set: &SssomMappingSet) -> Vec<RdfQuad> {
    let mut quads = Vec::new();
    for (index, mapping) in set.mappings.iter().enumerate() {
        let node = RdfTerm::blank_node(format!("mapping{index}"));

        quads.push(RdfQuad::new(
            node.clone(),
            RDF_TYPE,
            RdfTerm::iri(OWL_AXIOM),
        ));
        quads.push(RdfQuad::new(
            node.clone(),
            OWL_ANNOTATED_SOURCE,
            resolve_iri(&mapping.subject_id, &set.meta),
        ));
        quads.push(RdfQuad::new(
            node.clone(),
            OWL_ANNOTATED_PROPERTY,
            resolve_iri(&mapping.predicate_id, &set.meta),
        ));
        quads.push(RdfQuad::new(
            node.clone(),
            OWL_ANNOTATED_TARGET,
            resolve_iri(&mapping.object_id, &set.meta),
        ));

        if !mapping.mapping_justification.is_empty() {
            quads.push(RdfQuad::new(
                node.clone(),
                format!("{SSSOM_NS}mapping_justification"),
                resolve_iri(&mapping.mapping_justification, &set.meta),
            ));
        }
        if let Some(confidence) = mapping.confidence {
            quads.push(RdfQuad::new(
                node.clone(),
                format!("{SSSOM_NS}confidence"),
                RdfTerm::literal(RdfLiteral::typed(confidence.to_string(), XSD_DOUBLE)),
            ));
        }
        if let Some(comment) = &mapping.comment {
            quads.push(RdfQuad::new(
                node.clone(),
                format!("{SSSOM_NS}comment"),
                RdfTerm::literal(RdfLiteral::simple(comment.clone())),
            ));
        }
        if let Some(label) = &mapping.subject_label {
            quads.push(RdfQuad::new(
                resolve_iri(&mapping.subject_id, &set.meta),
                format!("{SSSOM_NS}subject_label"),
                RdfTerm::literal(RdfLiteral::simple(label.clone())),
            ));
        }
        if let Some(label) = &mapping.object_label {
            quads.push(RdfQuad::new(
                resolve_iri(&mapping.object_id, &set.meta),
                format!("{SSSOM_NS}object_label"),
                RdfTerm::literal(RdfLiteral::simple(label.clone())),
            ));
        }
    }
    quads
}

/// Resolve an entity reference to a full-IRI [`RdfTerm`].
///
/// A CURIE whose prefix is in the curie_map expands to `<namespace><reference>`;
/// anything else (an already-absolute IRI, or a CURIE with an undeclared prefix)
/// is used verbatim. Prefix completeness is [`validate`]'s job, not the
/// serializer's — `to_rdf` emits the best-effort IRI regardless so a partially
/// valid set still produces inspectable RDF.
fn resolve_iri(entity: &str, meta: &SssomMeta) -> RdfTerm {
    if let Some(prefix) = curie_prefix(entity) {
        if let Some(namespace) = meta.curie_map.get(prefix) {
            let reference = &entity[prefix.len() + 1..];
            return RdfTerm::iri(format!("{namespace}{reference}"));
        }
    }
    RdfTerm::iri(entity.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal, clean in-memory SSSOM document.
    const CLEAN: &str = "\
# mapping_set_id: https://example.org/mappings/demo
# mapping_set_version: 0.1.0
# license: https://creativecommons.org/licenses/by/4.0/
# comment: \"demo set\"
# curie_map:
#   ex: https://example.org/vocab/
#   skos: http://www.w3.org/2004/02/skos/core#
#   semapv: https://w3id.org/semapv/vocab/
#   sosa: http://www.w3.org/ns/sosa/
subject_id\tpredicate_id\tobject_id\tobject_label\tmapping_justification\tconfidence\tcomment
ex:Foo\tskos:closeMatch\tsosa:Observation\tObservation\tsemapv:ManualMappingCuration\t0.7\ta note
";

    fn negative(rows: &str) -> String {
        format!(
            "\
# mapping_set_id: https://example.org/mappings/demo
# curie_map:
#   ex: https://example.org/vocab/
#   skos: http://www.w3.org/2004/02/skos/core#
#   semapv: https://w3id.org/semapv/vocab/
#   sosa: http://www.w3.org/ns/sosa/
subject_id\tpredicate_id\tobject_id\tmapping_justification\tconfidence\tcomment
{rows}"
        )
    }

    #[test]
    fn parse_minimal_doc_fields() {
        let set = parse_tsv(CLEAN).expect("parse");
        assert_eq!(
            set.meta.mapping_set_id.as_deref(),
            Some("https://example.org/mappings/demo")
        );
        assert_eq!(set.meta.mapping_set_version.as_deref(), Some("0.1.0"));
        // The comment value is kept as authored (JSON-quoted), not unquoted.
        assert_eq!(set.meta.comment.as_deref(), Some("\"demo set\""));
        assert_eq!(
            set.meta.curie_map.get("ex").map(String::as_str),
            Some("https://example.org/vocab/")
        );
        assert_eq!(set.mappings.len(), 1);
        let m = &set.mappings[0];
        assert_eq!(m.subject_id, "ex:Foo");
        assert_eq!(m.predicate_id, "skos:closeMatch");
        assert_eq!(m.object_id, "sosa:Observation");
        assert_eq!(m.object_label.as_deref(), Some("Observation"));
        assert_eq!(m.mapping_justification, "semapv:ManualMappingCuration");
        assert_eq!(m.confidence, Some(0.7));
        assert_eq!(m.comment.as_deref(), Some("a note"));
    }

    #[test]
    fn parse_preserves_unknown_header_and_columns() {
        let doc = "\
# mapping_set_id: https://example.org/x
# creator_id: orcid:0000
# curie_map:
#   ex: https://example.org/vocab/
#   skos: http://www.w3.org/2004/02/skos/core#
#   semapv: https://w3id.org/semapv/vocab/
subject_id\tpredicate_id\tobject_id\tmapping_justification\tmapping_cardinality
ex:A\tskos:exactMatch\tex:B\tsemapv:ManualMappingCuration\t1:1
";
        let set = parse_tsv(doc).expect("parse");
        assert_eq!(
            set.meta.extra.get("creator_id").map(String::as_str),
            Some("orcid:0000")
        );
        assert_eq!(
            set.mappings[0]
                .extras
                .get("mapping_cardinality")
                .map(String::as_str),
            Some("1:1")
        );
    }

    #[test]
    fn serialize_preserves_unknown_columns_roundtrip() {
        // A non-PurRDF SSSOM file carrying an extra column must survive
        // parse → serialize_tsv (lossless round-trip), not be silently dropped
        // because the column set is built only from SSSOM_ORDER (H-1 / #855).
        let doc = "\
# mapping_set_id: https://example.org/x
# curie_map:
#   ex: https://example.org/vocab/
#   skos: http://www.w3.org/2004/02/skos/core#
#   semapv: https://w3id.org/semapv/vocab/
subject_id\tpredicate_id\tobject_id\tmapping_justification\tmapping_cardinality
ex:A\tskos:exactMatch\tex:B\tsemapv:ManualMappingCuration\t1:1
";
        let set = parse_tsv(doc).expect("parse");
        let serialized = serialize_tsv(&set);
        assert!(
            serialized.contains("mapping_cardinality"),
            "extra column dropped on serialize:\n{serialized}"
        );
        let reparsed = parse_tsv(&serialized).expect("reparse");
        assert_eq!(
            reparsed.mappings[0]
                .extras
                .get("mapping_cardinality")
                .map(String::as_str),
            Some("1:1"),
            "extra column value lost on round-trip"
        );
    }

    #[test]
    fn diagnostic_line_survives_interleaved_comment() {
        // A `#` provenance comment between data rows must not shift the reported
        // line of a later row's diagnostic. Before M-1 (gemini #1+#2 on #855) a
        // flat offset ignored the filtered comment and reported the wrong line.
        let doc = "\
# mapping_set_id: https://example.org/x
# curie_map:
#   ex: https://example.org/vocab/
#   skos: http://www.w3.org/2004/02/skos/core#
#   semapv: https://w3id.org/semapv/vocab/
subject_id\tpredicate_id\tobject_id\tmapping_justification\tconfidence
ex:A\tskos:exactMatch\tex:B\tsemapv:ManualMappingCuration\t0.7
# interleaved provenance comment
ex:C\tskos:exactMatch\tex:D\tsemapv:ManualMappingCuration\tNOTNUM
";
        let err = parse_tsv(doc).unwrap_err();
        assert_eq!(err.code, "sssom-tsv-parse");
        assert!(err.message.contains("confidence"), "{}", err.message);
        // The bad row is the 9th source line; the interleaved comment (line 8)
        // must not shift the reported line back to 8.
        assert_eq!(
            err.location.as_ref().and_then(|loc| loc.line),
            Some(9),
            "reported line should be the bad row's true source line",
        );
    }

    #[test]
    fn parse_ignores_trailer_comments() {
        let doc = format!("{CLEAN}# # REFUSED ex:Bar — out of scope\n# # provenance note\n");
        let set = parse_tsv(&doc).expect("parse");
        assert_eq!(set.mappings.len(), 1, "trailer comments are not mappings");
    }

    #[test]
    fn parse_rejects_headerless_doc() {
        let err = parse_tsv("# only: comments\n# more: comments\n").unwrap_err();
        assert_eq!(err.code, "sssom-tsv-parse");
    }

    #[test]
    fn parse_rejects_inline_curie_map_flow_sequence() {
        // The `unparseable-bad-yaml-curie-map` negative: an inline `[unclosed`
        // value sssom-py reported as a FATAL parse error.
        let doc = "\
# mapping_set_id: https://example.org/broken
# curie_map: [unclosed
subject_id\tpredicate_id\tobject_id
ex:A\tskos:exactMatch\tex:B
";
        let err = parse_tsv(doc).unwrap_err();
        assert_eq!(err.code, "sssom-tsv-parse");
    }

    #[test]
    fn roundtrip_serialize_is_stable() {
        let set = parse_tsv(CLEAN).expect("parse");
        let serialized = serialize_tsv(&set);
        let reparsed = parse_tsv(&serialized).expect("reparse");
        assert_eq!(set, reparsed, "parse ∘ serialize must be stable");
        // And serializing the reparse is byte-identical (fixed point).
        assert_eq!(serialized, serialize_tsv(&reparsed));
    }

    #[test]
    fn validate_clean_doc_has_no_errors() {
        let set = parse_tsv(CLEAN).expect("parse");
        assert!(validate(&set).is_empty());
    }

    #[test]
    fn validate_flags_unknown_prefix() {
        let set = parse_tsv(&negative(
            "nope:Foo\tskos:closeMatch\tsosa:Observation\tsemapv:ManualMappingCuration\t0.7\tx\n",
        ))
        .expect("parse");
        let diags = validate(&set);
        let prefix: Vec<_> = diags
            .iter()
            .filter(|d| d.check == "PrefixMapCompleteness")
            .collect();
        assert_eq!(prefix.len(), 1);
        assert_eq!(prefix[0].code, "prefix validation");
        assert_eq!(prefix[0].message, "Missing prefix: nope");
        assert_eq!(prefix[0].severity, RdfSeverity::Error);
    }

    #[test]
    fn validate_flags_confidence_too_high() {
        let set = parse_tsv(&negative(
            "ex:A\tskos:closeMatch\tsosa:Observation\tsemapv:ManualMappingCuration\t1.5\tx\n",
        ))
        .expect("parse");
        let diags = validate(&set);
        let js: Vec<_> = diags.iter().filter(|d| d.check == "JsonSchema").collect();
        assert_eq!(js.len(), 1);
        assert_eq!(js[0].code, "jsonschema validation");
        assert!(
            js[0].message.contains("greater than the maximum of 1.0"),
            "{}",
            js[0].message
        );
    }

    #[test]
    fn validate_flags_confidence_negative() {
        let set = parse_tsv(&negative(
            "ex:A\tskos:closeMatch\tsosa:Observation\tsemapv:ManualMappingCuration\t-0.5\tx\n",
        ))
        .expect("parse");
        let diags = validate(&set);
        let js: Vec<_> = diags.iter().filter(|d| d.check == "JsonSchema").collect();
        assert_eq!(js.len(), 1);
        assert!(
            js[0].message.contains("less than the minimum of 0.0"),
            "{}",
            js[0].message
        );
    }

    #[test]
    fn validate_flags_missing_required_slot() {
        // ENHANCEMENT: sssom-py silently drops this row; we flag it.
        let set = parse_tsv(&negative(
            "\tskos:closeMatch\tsosa:Observation\tsemapv:ManualMappingCuration\t0.7\tx\n",
        ))
        .expect("parse");
        let diags = validate(&set);
        let req: Vec<_> = diags.iter().filter(|d| d.check == "RequiredSlot").collect();
        assert_eq!(req.len(), 1);
        assert_eq!(req[0].code, "required slot");
        assert!(req[0].message.contains("subject_id"), "{}", req[0].message);
    }

    #[test]
    fn to_rdf_emits_owl_reification_for_a_mapping() {
        let set = parse_tsv(CLEAN).expect("parse");
        let quads = to_rdf(&set);
        assert!(!quads.is_empty());

        let has = |pred: &str, obj: &str| {
            quads.iter().any(|q| {
                q.predicate == pred && matches!(&q.object, RdfTerm::Iri(iri) if iri == obj)
            })
        };
        // CURIEs are resolved to full IRIs via the curie_map.
        assert!(has(RDF_TYPE, OWL_AXIOM));
        assert!(has(OWL_ANNOTATED_SOURCE, "https://example.org/vocab/Foo"));
        assert!(has(
            OWL_ANNOTATED_PROPERTY,
            "http://www.w3.org/2004/02/skos/core#closeMatch"
        ));
        assert!(has(
            OWL_ANNOTATED_TARGET,
            "http://www.w3.org/ns/sosa/Observation"
        ));
        // Confidence is an xsd:double literal.
        assert!(quads.iter().any(|q| {
            q.predicate == format!("{SSSOM_NS}confidence")
                && matches!(
                    &q.object,
                    RdfTerm::Literal(l) if l.datatype.as_deref() == Some(XSD_DOUBLE)
                )
        }));
    }

    #[test]
    fn corpus_accessibility_parses_and_validates_clean() {
        // The committed corpus file is the parity anchor: parse must succeed and
        // validate must yield zero ERRORs (full 66-file parity is covered by the
        // Python parity test, Task 7). Path is resolved relative to this crate's
        // manifest dir up to the repo root.
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../generated/mappings/gmeow-accessibility.sssom.tsv"
        );
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read corpus file {path}: {e}"));
        let set = parse_tsv(&text).expect("corpus parse");
        assert!(
            set.mappings.len() >= 10,
            "corpus rows: {}",
            set.mappings.len()
        );
        let diags = validate(&set);
        assert!(
            diags.iter().all(|d| d.severity != RdfSeverity::Error),
            "corpus must validate clean, got: {diags:?}"
        );
    }
}
