// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Grounded agent memory over an append-only GTS `ai-package`.
//!
//! This module mirrors the Python `gts.examples.agent_memory` workflow without
//! adding graph database, SPARQL, embedding, or RDF toolkit dependencies. Claims
//! are reified RDF 1.2 statements. Revisions append suppressions and optional
//! `purrdf:wasDerivedFrom` audit links. Tool calls are ordinary provenance quads
//! in the same package.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use ciborium::value::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::model::{Graph, Term, TermKind};
use crate::reader::{read, read_file_segments};
use crate::wire::map_get;
use crate::writer::{
    FrameOptions, Writer, WriterError, WriterOptions, annot_payload, digest_string, quads_payload,
    reifies_payload, suppress_payload, terms_payload,
};

const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const CONFIDENCE: &str = "https://example.org/memory/confidence";
const ACCORDING_TO: &str = "https://example.org/memory/accordingTo";
const SOURCE_LOCATION: &str = "https://example.org/memory/sourceLocation";
const WAS_DERIVED_FROM: &str = "https://example.org/memory/wasDerivedFrom";
const DCT_CREATED: &str = "http://purl.org/dc/terms/created";
const TOOL_CALL: &str = "https://example.org/memory/ToolCall";
const SOFTWARE_AGENT: &str = "https://example.org/memory/SoftwareAgent";
const USED_TOOL: &str = "https://example.org/memory/usedTool";
const TOOL_ARGUMENTS: &str = "https://example.org/memory/toolArguments";
const TOOL_RESULT: &str = "https://example.org/memory/toolResult";
const CALLED_BY_INVOCATION: &str = "https://example.org/memory/calledByInvocation";
const WAS_GENERATED_BY: &str = "https://example.org/memory/wasGeneratedBy";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
const PROFILE: &str = "ai-package";
const INLINE_PAYLOAD_BUDGET: usize = 4096;
/// Declared zstd level for the default authoring profile — the measured knee
/// (see `purrdf_rdf::gts_compose::DIST_ZSTD_LEVEL`).
const DEFAULT_ZSTD_LEVEL: i32 = 12;

/// One recalled claim, the user-facing view of a reified statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Claim {
    /// Reifier IRI identifying the claim.
    pub id: String,
    /// Claim text (the reified statement's `rdf:value` literal).
    pub text: String,
    /// Confidence annotation in `[0, 1]`, when recorded.
    pub confidence: Option<f64>,
    /// IRI of the party the claim is attributed to (`accordingTo`).
    pub according_to: Option<String>,
    /// Source-location annotation, when recorded.
    pub source: Option<String>,
    /// Creation timestamp (`dct:created`) as an `xsd:dateTime` lexical value.
    pub created: Option<String>,
    /// Whether a later revision suppressed this claim.
    pub suppressed: bool,
}

/// One recorded tool call, represented as provenance in the same graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCallRecord {
    /// IRI identifying the tool-call activity.
    pub id: String,
    /// IRI of the software agent that was invoked (`usedTool`).
    pub tool: String,
    /// Serialized tool arguments, when recorded.
    pub arguments: Option<String>,
    /// Serialized tool result, when recorded.
    pub result: Option<String>,
    /// IRI of the enclosing invocation, when recorded.
    pub invocation: Option<String>,
    /// Creation timestamp (`dct:created`) as an `xsd:dateTime` lexical value.
    pub created: Option<String>,
    /// IRIs of entities this call generated (`wasGeneratedBy` targets).
    pub generated: Vec<String>,
}

/// Options for [`Memory::store`].
#[derive(Clone, Copy, Debug, Default)]
pub struct StoreOptions<'a> {
    /// Source-location annotation to attach to the claim.
    pub source: Option<&'a str>,
    /// Confidence annotation; must be finite and in `[0, 1]`.
    pub confidence: Option<f64>,
    /// IRI of the party the claim is attributed to.
    pub according_to: Option<&'a str>,
}

/// Options for [`Memory::revise`].
#[derive(Clone, Copy, Debug, Default)]
pub struct RevisionOptions<'a> {
    /// Human-readable reason recorded with the suppression.
    pub reason: Option<&'a str>,
    /// Claim id of the replacement, recorded as a `wasDerivedFrom` audit link.
    pub superseded_by: Option<&'a str>,
}

/// Options for [`Memory::record_tool_call`].
#[derive(Clone, Copy, Debug, Default)]
pub struct ToolCallOptions<'a> {
    /// Serialized tool arguments to record.
    pub arguments: Option<&'a str>,
    /// Serialized tool result to record.
    pub result: Option<&'a str>,
    /// IRI of the enclosing invocation; must be non-empty when supplied.
    pub invocation: Option<&'a str>,
    /// IRIs of entities generated by this call; each must be non-empty.
    pub generated: &'a [&'a str],
}

/// Options for [`Memory::recall`].
#[derive(Clone, Copy, Debug)]
pub struct RecallOptions<'a> {
    /// Keyword query matched against claim text; empty matches everything.
    pub query: &'a str,
    /// Drop claims whose confidence is below this threshold.
    pub min_confidence: Option<f64>,
    /// Maximum number of claims to return.
    pub limit: usize,
    /// Include claims hidden by suppression frames.
    pub include_suppressed: bool,
}

impl Default for RecallOptions<'_> {
    fn default() -> Self {
        Self {
            query: "",
            min_confidence: None,
            limit: 10,
            include_suppressed: false,
        }
    }
}

/// Errors raised by the dependency-light example API.
#[derive(Debug)]
pub enum MemoryError {
    /// Reading or writing the package file failed.
    Io(std::io::Error),
    /// The claim text was empty.
    EmptyClaim,
    /// The confidence was not finite or fell outside `[0, 1]`.
    InvalidConfidence,
    /// The tool agent IRI was empty.
    EmptyTool,
    /// An invocation IRI was supplied but empty.
    EmptyInvocation,
    /// A generated-entity IRI was empty.
    EmptyGeneratedEntity,
    /// The GTS writer refused the authoring configuration or an appended frame.
    Author(WriterError),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::EmptyClaim => f.write_str("a claim needs text"),
            Self::InvalidConfidence => {
                f.write_str("confidence must be finite and in the range [0, 1]")
            }
            Self::EmptyTool => f.write_str("a tool call needs a tool agent IRI"),
            Self::EmptyInvocation => {
                f.write_str("invocation must be a non-empty IRI when supplied")
            }
            Self::EmptyGeneratedEntity => f.write_str("generated entity IRIs must be non-empty"),
            Self::Author(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for MemoryError {}

impl From<std::io::Error> for MemoryError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Convenient result alias for the example API.
pub type Result<T> = std::result::Result<T, MemoryError>;

/// How a [`Memory`] package authors its frames.
///
/// The default is transform-chained and dictionary-capable. The previous
/// default — a bare `Writer::new(PROFILE)` with no transform at all — stored
/// every claim's payload uncompressed, which is a degraded default rather than
/// a chosen one: an agent memory is append-heavy, highly repetitive text, the
/// single best case for a primed zstd stream.
#[derive(Clone, Debug)]
pub struct MemoryOptions {
    /// GTS profile recorded in the segment header.
    pub profile: String,
    /// Transform chain every authored frame payload rides.
    ///
    /// `zstd-rsyncable` is the default: an agent memory file is appended to
    /// constantly and typically lives under rsync/git, so bounded-neighbourhood
    /// compressed output is worth more than the last few percent of ratio.
    pub transform: Vec<String>,
    /// Declared zstd level (§8.5 catalog `level?`).
    pub zstd_level: Option<i32>,
    /// In-band dictionaries pinned in the store's ONE header (§5 `"dct"`).
    ///
    /// This is why the append mode below matters: a per-claim header would
    /// repeat every one of these dictionaries — kilobytes — for a few hundred
    /// bytes of claim.
    pub dicts: Vec<(String, Vec<u8>)>,
    /// Which pinned dictionary primes authored frames.
    pub dict: Option<String>,
}

impl Default for MemoryOptions {
    fn default() -> Self {
        Self {
            profile: PROFILE.to_string(),
            transform: vec!["zstd-rsyncable".to_string()],
            zstd_level: Some(DEFAULT_ZSTD_LEVEL),
            dicts: Vec::new(),
            dict: None,
        }
    }
}

/// An append-only grounded-memory package.
///
/// A store file carries ONE segment header; each `store`/`revise`/
/// `record_tool_call` appends frames onto that segment's `prev` chain
/// ([`Writer::appending`]). Before, every call minted its own self-contained
/// pack, so an N-claim file was N headers — and with an in-band dictionary each
/// of those headers re-pinned the whole dictionary.
#[derive(Clone, Debug)]
pub struct Memory {
    path: PathBuf,
    options: MemoryOptions,
}

impl Memory {
    /// Open an existing memory package, or create it on first write, with the
    /// default (transform-chained, dictionary-capable) authoring options.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self::with_options(path, MemoryOptions::default())
    }

    /// Open a memory package with explicit authoring options.
    pub fn with_options(path: impl Into<PathBuf>, options: MemoryOptions) -> Self {
        Self {
            path: path.into(),
            options,
        }
    }

    /// The package path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The authoring options in force.
    pub fn options(&self) -> &MemoryOptions {
        &self.options
    }

    /// A writer that CONTINUES the store's single segment, or mints its header
    /// when the file does not exist yet.
    ///
    /// The returned writer's bytes are always appended to the file, so the
    /// header is written exactly once per store.
    fn writer(&self) -> Result<Writer> {
        if self.path.exists() {
            let existing = fs::read(&self.path)?;
            if !existing.is_empty() {
                return Writer::appending(&existing).map_err(MemoryError::Author);
            }
        }
        Writer::with_options(
            &self.options.profile,
            WriterOptions {
                dicts: self.options.dicts.clone(),
                zstd_level: self.options.zstd_level,
                ..WriterOptions::default()
            },
        )
        .map_err(MemoryError::Author)
    }

    /// The term table of the store's LAST segment, in segment-local id order.
    ///
    /// This is the LAST segment's own fold — never the cross-segment union,
    /// whose by-value merge would under-count the ids actually in use.
    fn last_segment_terms(&self) -> Result<Vec<Term>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&self.path)?;
        if bytes.is_empty() {
            return Ok(Vec::new());
        }
        Ok(read_file_segments(&bytes)
            .segments
            .last()
            .map_or_else(Vec::new, |segment| segment.terms.clone()))
    }

    /// How many term rows the store's LAST segment already carries.
    ///
    /// Term ids are SEGMENT-scoped and cumulative across the segment's `terms`
    /// frames (§7.5). When every claim was its own self-contained pack this was
    /// always zero; appending into one segment means each new batch of terms is
    /// numbered from here, and every quad/reifies/annot row it authors must be
    /// shifted by the same base — otherwise the second claim's rows would
    /// silently point at the FIRST claim's terms.
    fn term_base(&self) -> Result<usize> {
        Ok(self.last_segment_terms()?.len())
    }

    /// Append one frame under the store's transform/dictionary profile.
    fn append_frame(&self, writer: &mut Writer, frame_type: &str, payload: Value) -> Result<()> {
        writer
            .add_frame_with_options(
                frame_type,
                FrameOptions {
                    payload: Some(payload),
                    transform: self.options.transform.clone(),
                    dict: self.options.dict.clone(),
                    ..FrameOptions::default()
                },
            )
            .map(|_| ())
            .map_err(MemoryError::Author)
    }

    /// Append one claim as a reified RDF 1.2 statement.
    pub fn store(&self, text: &str, options: StoreOptions<'_>) -> Result<Claim> {
        if text.trim().is_empty() {
            return Err(MemoryError::EmptyClaim);
        }
        if let Some(confidence) = options.confidence
            && (!confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
        {
            return Err(MemoryError::InvalidConfidence);
        }

        let created = now_rfc3339();
        let file_len = self.file_len()?;
        let confidence_text = options.confidence.map(|value| value.to_string());
        let assertion = self.digest_id(
            "assertion",
            file_len,
            [
                text,
                created.as_str(),
                options.source.unwrap_or(""),
                confidence_text.as_deref().unwrap_or(""),
                options.according_to.unwrap_or(""),
            ],
        );
        let subject = format!("urn:purrdf:claim:{}", digest_string(text.as_bytes()));

        let mut writer = self.writer()?;
        let mut terms = vec![
            iri(&subject),
            iri(RDF_VALUE),
            lit(text),
            iri(&assertion),
            iri(XSD_DATETIME),
        ];
        let datetime_dt = 4;
        let mut annotations = Vec::new();

        push_annotation(
            &mut terms,
            &mut annotations,
            3,
            DCT_CREATED,
            literal_with_datatype(&created, datetime_dt),
        );
        if let Some(confidence) = confidence_text.as_deref() {
            let decimal_dt = terms.len();
            terms.push(iri(XSD_DECIMAL));
            push_annotation(
                &mut terms,
                &mut annotations,
                3,
                CONFIDENCE,
                literal_with_datatype(confidence, decimal_dt),
            );
        }
        if let Some(according_to) = options.according_to {
            push_annotation(
                &mut terms,
                &mut annotations,
                3,
                ACCORDING_TO,
                lit(according_to),
            );
        }
        if let Some(source) = options.source {
            push_annotation(
                &mut terms,
                &mut annotations,
                3,
                SOURCE_LOCATION,
                lit(source),
            );
        }

        // Term ids continue the segment's existing table (see `term_base`).
        let base = self.term_base()?;
        let terms = shift_terms(&terms, base);
        let quads = shift_quads(&[(0, 1, 2, None)], base);
        let reifies = [(3 + base, (base, 1 + base, 2 + base), None)];
        let annotations = shift_annotations(&annotations, base);

        self.append_frame(&mut writer, "terms", terms_payload(&terms))?;
        self.append_frame(&mut writer, "quads", quads_payload(&quads))?;
        self.append_frame(&mut writer, "reifies", reifies_payload(&reifies))?;
        self.append_frame(&mut writer, "annot", annot_payload(&annotations))?;
        self.append(&writer.into_bytes())?;

        Ok(Claim {
            id: assertion,
            text: text.to_string(),
            confidence: options.confidence,
            according_to: options.according_to.map(str::to_string),
            source: options.source.map(str::to_string),
            created: Some(created),
            suppressed: false,
        })
    }

    /// Append a value-wise suppression for a claim, optionally linking a successor.
    ///
    /// A suppression NAMES an existing row, so every IRI here is resolved
    /// against the live segment's term table rather than re-stated — see
    /// [`SegmentIris`] for why re-stating it would silently miss the claim.
    pub fn revise(&self, claim_id: &str, options: RevisionOptions<'_>) -> Result<()> {
        let mut writer = self.writer()?;
        let mut iris = SegmentIris::new(&self.last_segment_terms()?);
        let target = iris.resolve(claim_id);
        let mut annotations = Vec::new();
        if let Some(successor) = options.superseded_by {
            let successor_id = iris.resolve(successor);
            let derived_from = iris.resolve(WAS_DERIVED_FROM);
            annotations.push((successor_id, derived_from, target, None));
        }
        if !iris.appended().is_empty() {
            self.append_frame(&mut writer, "terms", terms_payload(iris.appended()))?;
        }
        if !annotations.is_empty() {
            self.append_frame(&mut writer, "annot", annot_payload(&annotations))?;
        }
        let suppression = suppress_payload(
            vec![Value::Map(vec![
                ("kind".into(), "term".into()),
                ("id".into(), Value::from(target as u64)),
            ])],
            options.reason,
            None,
        );
        self.append_frame(&mut writer, "suppress", suppression)?;
        self.append(&writer.into_bytes())
    }

    /// Append one tool-call provenance record.
    pub fn record_tool_call(
        &self,
        tool: &str,
        options: ToolCallOptions<'_>,
    ) -> Result<ToolCallRecord> {
        let tool = tool.trim();
        if tool.is_empty() {
            return Err(MemoryError::EmptyTool);
        }
        let invocation = options.invocation.map(str::trim);
        if invocation.is_some_and(str::is_empty) {
            return Err(MemoryError::EmptyInvocation);
        }
        let generated: Vec<&str> = options.generated.iter().map(|value| value.trim()).collect();
        if generated.iter().any(|value| value.is_empty()) {
            return Err(MemoryError::EmptyGeneratedEntity);
        }

        let created = now_rfc3339();
        let arguments = inline_or_digest(options.arguments);
        let result = inline_or_digest(options.result);
        let call = self.digest_id(
            "toolcall",
            self.file_len()?,
            [
                tool,
                created.as_str(),
                arguments.as_deref().unwrap_or(""),
                result.as_deref().unwrap_or(""),
                invocation.unwrap_or(""),
            ],
        );

        let mut terms = Vec::new();
        let mut quads = Vec::new();
        let t_call = push_term(&mut terms, iri(&call));
        let t_type = push_term(&mut terms, iri(RDF_TYPE));
        quads.push((t_call, t_type, push_term(&mut terms, iri(TOOL_CALL)), None));
        let t_tool = push_term(&mut terms, iri(tool));
        quads.push((t_call, push_term(&mut terms, iri(USED_TOOL)), t_tool, None));
        quads.push((
            t_tool,
            t_type,
            push_term(&mut terms, iri(SOFTWARE_AGENT)),
            None,
        ));
        let t_datetime = push_term(&mut terms, iri(XSD_DATETIME));
        quads.push((
            t_call,
            push_term(&mut terms, iri(DCT_CREATED)),
            push_term(&mut terms, literal_with_datatype(&created, t_datetime)),
            None,
        ));
        if let Some(arguments) = arguments.as_deref() {
            quads.push((
                t_call,
                push_term(&mut terms, iri(TOOL_ARGUMENTS)),
                push_term(&mut terms, lit(arguments)),
                None,
            ));
        }
        if let Some(result) = result.as_deref() {
            quads.push((
                t_call,
                push_term(&mut terms, iri(TOOL_RESULT)),
                push_term(&mut terms, lit(result)),
                None,
            ));
        }
        if let Some(invocation) = invocation {
            quads.push((
                t_call,
                push_term(&mut terms, iri(CALLED_BY_INVOCATION)),
                push_term(&mut terms, iri(invocation)),
                None,
            ));
        }
        if !generated.is_empty() {
            let t_generated_by = push_term(&mut terms, iri(WAS_GENERATED_BY));
            for entity in &generated {
                quads.push((
                    push_term(&mut terms, iri(entity)),
                    t_generated_by,
                    t_call,
                    None,
                ));
            }
        }

        let mut writer = self.writer()?;
        let base = self.term_base()?;
        let terms = shift_terms(&terms, base);
        let quads = shift_quads(&quads, base);
        self.append_frame(&mut writer, "terms", terms_payload(&terms))?;
        self.append_frame(&mut writer, "quads", quads_payload(&quads))?;
        self.append(&writer.into_bytes())?;

        Ok(ToolCallRecord {
            id: call,
            tool: tool.to_string(),
            arguments,
            result,
            invocation: invocation.map(str::to_string),
            created: Some(created),
            generated: generated.into_iter().map(str::to_string).collect(),
        })
    }

    /// Return claims matching `options.query`, best match first.
    pub fn recall(&self, options: RecallOptions<'_>) -> Result<Vec<Claim>> {
        let mut claims: Vec<Claim> = self
            .claims()?
            .into_iter()
            .filter(|claim| options.include_suppressed || !claim.suppressed)
            .filter(|claim| match options.min_confidence {
                None => true,
                Some(min) => claim.confidence.is_some_and(|got| got >= min),
            })
            .collect();
        let tokens: HashSet<String> = options
            .query
            .to_lowercase()
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if tokens.is_empty() {
            claims.reverse();
        } else {
            let mut scored: Vec<(usize, usize, Claim)> = claims
                .into_iter()
                .enumerate()
                .map(|(index, claim)| {
                    let claim_tokens: HashSet<String> = claim
                        .text
                        .to_lowercase()
                        .split_whitespace()
                        .map(str::to_string)
                        .collect();
                    let score = tokens.intersection(&claim_tokens).count();
                    (score, index, claim)
                })
                .filter(|(score, _, _)| *score > 0)
                .collect();
            scored.sort_by_key(|(score, index, _)| {
                (std::cmp::Reverse(*score), std::cmp::Reverse(*index))
            });
            claims = scored.into_iter().map(|(_, _, claim)| claim).collect();
        }
        claims.truncate(options.limit);
        Ok(claims)
    }

    /// Every claim in storage order.
    pub fn claims(&self) -> Result<Vec<Claim>> {
        let Some(graph) = self.graph()? else {
            return Ok(Vec::new());
        };
        let suppressed = suppressed_terms(&graph);
        let annotations = annotations_by_reifier(&graph);
        let mut out = Vec::new();
        for &(rid, (s, p, o), _) in &graph.reifiers {
            if term_value(&graph, p) != RDF_VALUE {
                continue;
            }
            let ann = annotations.get(&rid);
            let confidence = ann
                .and_then(|values| values.get(CONFIDENCE))
                .and_then(|value| value.parse::<f64>().ok());
            out.push(Claim {
                id: term_value(&graph, rid).to_string(),
                text: term_value(&graph, o).to_string(),
                confidence,
                according_to: ann.and_then(|values| values.get(ACCORDING_TO)).cloned(),
                source: ann.and_then(|values| values.get(SOURCE_LOCATION)).cloned(),
                created: ann.and_then(|values| values.get(DCT_CREATED)).cloned(),
                suppressed: suppressed.contains(&rid) || suppressed.contains(&s),
            });
        }
        Ok(out)
    }

    /// Every recorded tool call in storage order.
    pub fn tool_calls(&self) -> Result<Vec<ToolCallRecord>> {
        let Some(graph) = self.graph()? else {
            return Ok(Vec::new());
        };
        let mut call_ids = Vec::new();
        let mut props: HashMap<usize, HashMap<String, String>> = HashMap::new();
        let mut backlinks: HashMap<usize, Vec<String>> = HashMap::new();
        for &(s, p, o, _) in &graph.quads {
            let pred = term_value(&graph, p);
            if pred == RDF_TYPE && term_value(&graph, o) == TOOL_CALL {
                if !call_ids.contains(&s) {
                    call_ids.push(s);
                }
            } else if pred == WAS_GENERATED_BY {
                backlinks
                    .entry(o)
                    .or_default()
                    .push(term_value(&graph, s).to_string());
            } else if matches!(
                pred,
                USED_TOOL | TOOL_ARGUMENTS | TOOL_RESULT | CALLED_BY_INVOCATION | DCT_CREATED
            ) {
                props
                    .entry(s)
                    .or_default()
                    .insert(pred.to_string(), term_value(&graph, o).to_string());
            }
        }

        let mut out = Vec::new();
        for cid in call_ids {
            let values = props.get(&cid);
            out.push(ToolCallRecord {
                id: term_value(&graph, cid).to_string(),
                tool: values
                    .and_then(|value| value.get(USED_TOOL))
                    .cloned()
                    .unwrap_or_default(),
                arguments: values.and_then(|value| value.get(TOOL_ARGUMENTS)).cloned(),
                result: values.and_then(|value| value.get(TOOL_RESULT)).cloned(),
                invocation: values
                    .and_then(|value| value.get(CALLED_BY_INVOCATION))
                    .cloned(),
                created: values.and_then(|value| value.get(DCT_CREATED)).cloned(),
                generated: backlinks.remove(&cid).unwrap_or_default(),
            });
        }
        Ok(out)
    }

    /// Reader diagnostics for the package. Empty means transport-clean.
    pub fn verify(&self) -> Result<Vec<String>> {
        let Some(graph) = self.graph()? else {
            return Ok(Vec::new());
        };
        Ok(graph
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.detail))
            .collect())
    }

    fn graph(&self) -> Result<Option<Graph>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&self.path)?;
        Ok(Some(read(&bytes, true, None)))
    }

    fn file_len(&self) -> Result<u64> {
        match fs::metadata(&self.path) {
            Ok(meta) => Ok(meta.len()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(err) => Err(MemoryError::Io(err)),
        }
    }

    fn append(&self, segment: &[u8]) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(segment)?;
        Ok(())
    }

    fn digest_id<'a>(
        &self,
        kind: &str,
        file_len: u64,
        parts: impl IntoIterator<Item = &'a str>,
    ) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(kind.as_bytes());
        hasher.update(&file_len.to_le_bytes());
        for part in parts {
            hasher.update(&[0]);
            hasher.update(part.as_bytes());
        }
        format!("urn:purrdf:{kind}:blake3:{}", hasher.finalize().to_hex())
    }
}

/// Resolves IRIs against a live segment's EXISTING term table, minting rows
/// only for the genuinely new ones.
///
/// Term ids are POSITIONAL within a segment (§7.5): the reader interns by value
/// only when it unions distinct segments. So a frame appended into a segment
/// that re-states an IRI the segment already carries gets a SECOND, different
/// id for it. That is harmless for a self-contained batch like `store`'s, whose
/// rows only ever reference their own terms — but it is fatal for anything that
/// must NAME a row somebody else already keyed on: a `suppress` target or an
/// `annot` reifier pointing at the duplicate addresses a term no reifier is
/// bound to, so the directive silently applies to nothing. Pack-per-claim hid
/// this, because each claim was its own segment and the cross-segment union
/// merged the two rows by value before anything consulted them.
struct SegmentIris {
    by_value: HashMap<String, usize>,
    appended: Vec<Term>,
    base: usize,
}

impl SegmentIris {
    /// Index the segment's IRI rows; the FIRST id wins, which is the one the
    /// segment's reifier and quad rows were authored against.
    fn new(terms: &[Term]) -> Self {
        let mut by_value: HashMap<String, usize> = HashMap::new();
        for (id, term) in terms.iter().enumerate() {
            if term.kind == TermKind::Iri
                && let Some(value) = term.value.as_deref()
            {
                by_value.entry(value.to_string()).or_insert(id);
            }
        }
        Self {
            by_value,
            appended: Vec::new(),
            base: terms.len(),
        }
    }

    /// The segment-scoped id of `value`, appending a row when it is new.
    ///
    /// A value that is new to THIS segment but present in an earlier one still
    /// resolves correctly: it is appended here, and the reader's cross-segment
    /// union merges it with the earlier row by value.
    fn resolve(&mut self, value: &str) -> usize {
        if let Some(&id) = self.by_value.get(value) {
            return id;
        }
        let id = self.base + self.appended.len();
        self.appended.push(iri(value));
        self.by_value.insert(value.to_string(), id);
        id
    }

    /// The rows to author, already numbered from the segment's base.
    fn appended(&self) -> &[Term] {
        &self.appended
    }
}

fn iri(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn lit(value: &str) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn literal_with_datatype(value: &str, datatype: usize) -> Term {
    Term {
        datatype: Some(datatype),
        ..lit(value)
    }
}

/// Shift a locally-numbered term batch into the segment's term-id space.
///
/// A term's `dt`/`rf` fields are themselves term ids, and §7.5 requires a `dt`
/// to name an ALREADY-INTRODUCED term — an unshifted datatype reference would
/// either dangle or, worse, resolve to some earlier claim's term.
fn shift_terms(terms: &[Term], base: usize) -> Vec<Term> {
    terms
        .iter()
        .map(|term| Term {
            kind: term.kind,
            value: term.value.clone(),
            datatype: term.datatype.map(|id| id + base),
            lang: term.lang.clone(),
            direction: term.direction.clone(),
            reifier: term.reifier.map(|id| id + base),
        })
        .collect()
}

/// Shift locally-numbered quad rows into the segment's term-id space.
fn shift_quads(
    quads: &[(usize, usize, usize, Option<usize>)],
    base: usize,
) -> Vec<(usize, usize, usize, Option<usize>)> {
    quads
        .iter()
        .map(|&(s, p, o, g)| (s + base, p + base, o + base, g.map(|g| g + base)))
        .collect()
}

/// Shift locally-numbered annotation rows into the segment's term-id space.
fn shift_annotations(
    rows: &[(usize, usize, usize, Option<usize>)],
    base: usize,
) -> Vec<(usize, usize, usize, Option<usize>)> {
    shift_quads(rows, base)
}

fn push_term(terms: &mut Vec<Term>, term: Term) -> usize {
    terms.push(term);
    terms.len() - 1
}

fn push_annotation(
    terms: &mut Vec<Term>,
    annotations: &mut Vec<(usize, usize, usize, Option<usize>)>,
    reifier: usize,
    predicate: &str,
    value: Term,
) {
    let pred = push_term(terms, iri(predicate));
    let val = push_term(terms, value);
    annotations.push((reifier, pred, val, None));
}

fn inline_or_digest(payload: Option<&str>) -> Option<String> {
    let payload = payload?;
    if payload.len() <= INLINE_PAYLOAD_BUDGET {
        Some(payload.to_string())
    } else {
        Some(digest_string(payload.as_bytes()))
    }
}

fn annotations_by_reifier(graph: &Graph) -> HashMap<usize, HashMap<String, String>> {
    let mut out: HashMap<usize, HashMap<String, String>> = HashMap::new();
    for &(rid, p, v, _) in &graph.annotations {
        let pred = term_value(graph, p);
        let value = term_value(graph, v);
        if !pred.is_empty() && !value.is_empty() {
            out.entry(rid)
                .or_default()
                .insert(pred.to_string(), value.to_string());
        }
    }
    out
}

fn suppressed_terms(graph: &Graph) -> HashSet<usize> {
    let mut out = HashSet::new();
    for suppression in &graph.suppressions {
        for target in &suppression.targets {
            let Value::Map(entries) = target else {
                continue;
            };
            let Some(Value::Text(kind)) = map_get(entries, "kind") else {
                continue;
            };
            if !matches!(kind.as_str(), "term" | "reifier") {
                continue;
            }
            if let Some(id) = map_get(entries, "id").and_then(value_as_idx) {
                out.insert(id);
            }
        }
    }
    out
}

fn value_as_idx(value: &Value) -> Option<usize> {
    let Value::Integer(raw) = value else {
        return None;
    };
    usize::try_from(i128::from(*raw)).ok()
}

fn term_value(graph: &Graph, term_id: usize) -> &str {
    graph
        .terms
        .get(term_id)
        .and_then(|term| term.value.as_deref())
        .unwrap_or("")
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
