// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SPARQL Results **JSON** (SRJ) reader — the inverse of [`crate::json`].
//!
//! Parses a W3C SPARQL 1.1 Query Results JSON document
//! (<https://www.w3.org/TR/sparql11-results-json/>) into a [`ParsedSolutions`]
//! (for `SELECT`) or a boolean (for `ASK`). This is what SPARQL `SERVICE`
//! federation uses to ingest a remote endpoint's response, and what
//! the W3C conformance harness uses to read expected `.srj` results.
//!
//! # Wasm discipline
//!
//! The JSON grammar is the workspace's one reader, [`purrdf_lex::json`]. The
//! `SELECT` readers drive its pull [`Reader`] and build each binding row
//! straight from its events: no JSON tree is built, a string without an escape
//! is borrowed from the document until it becomes a term, and a member the
//! reader does not want is checked without being built. The `ASK` and
//! provenance readers, whose documents are a handful of members, read the tree.
//! No `std::io`, so the crate stays wasm-clean.
//!
//! # Refusals
//!
//! [`from_json`] refuses exactly what a reader that first built the whole
//! tree and then judged its shape would refuse, with the same message: a
//! document that is not JSON is refused as JSON at its first defect, wherever
//! a shape refusal sits before it, and among shape refusals the first in the
//! order the tree would meet them wins — the `boolean` member, then `head`,
//! its `vars`, `results`, its `bindings`, then the rows in order, each row's
//! cells in `head.vars` order, and each binding's members in the order the
//! decoder consults them. The streaming decoder therefore records a shape
//! refusal, checks the rest of what it is reading, and reports the refusal
//! only once nothing that outranks it can follow.
//!
//! # Nesting depth
//!
//! Every walk over input nesting — reading a JSON value, skipping one, and
//! decoding an RDF 1.2 triple-term binding — runs over an explicit heap stack
//! rather than the machine stack, and the reader sets no container-depth cap.
//! A document nested to any depth is therefore parsed (or refused for a syntax
//! or shape error) with a machine-stack footprint independent of that depth,
//! which is what keeps a remote `SERVICE` endpoint's deeply nested answer from
//! overflowing the stack.

use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use std::borrow::Cow;
use std::ops::Range;

use purrdf_core::TermBox;
use purrdf_core::{BlankScope, RdfTextDirection, TermValue};
use purrdf_lex::json::{self, Event, Kind, Limits, Reader, Value};

use crate::error::{Error, NativePrimary, ReadReason, ReadRefusal, ReadShape, ReadToken};
use crate::model::{ProvenanceNamespace, ResultProvenance, SolutionProvenance};

use purrdf_core::datatype::XSD_STRING;
use purrdf_core::vocab::language_datatype_iri;

/// Dense decoded row and bounded row-prefix result aliases keep the streaming reader's
/// signatures readable without changing the public model.
type BindingRow = Vec<Option<TermValue>>;
type BoundedRows = (Vec<BindingRow>, bool);
type BoundedRowsResult = Result<BoundedRows, Error>;

/// What a streaming step decoded, or the shape refusal it recorded: the inner
/// result of a [`Step`].
type Shaped<T> = Result<T, Error>;

/// A streaming step: the outer error is a JSON refusal, which outranks every
/// shape refusal and ends the read at once; the inner one is a shape refusal
/// the step recorded after reading its whole value.
type Step<T> = Result<Shaped<T>, Error>;

/// Original JSON parser containers retained after a cleanup refusal.
///
/// This owner reuses the original admitted allocations. Releasing it under the
/// same account retries cleanup and returns every remaining owner on failure.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct JsonRelease {
    strings: Option<Vec<String>>,
    text: Option<String>,
    row: Option<BindingRow>,
    rows: Option<Vec<BindingRow>>,
    frame: Option<Open>,
    scratch: Option<RowScratch>,
    term: Option<TermValue>,
    primary: Option<NativePrimary>,
    superseded: Option<NativePrimary>,
}

#[derive(Debug, PartialEq, Eq)]
enum ReleaseRetained {
    Strings(Vec<String>),
    Row(BindingRow),
    Rows(Vec<BindingRow>),
    Parts(Box<TripleParts>),
    Binding(Box<BindingFields>),
    Scratch(RowScratch),
}

impl JsonRelease {
    fn new(retained: ReleaseRetained) -> Self {
        let mut owner = Self::default();
        match retained {
            ReleaseRetained::Strings(value) => {
                if value.capacity() != 0 {
                    owner.strings = Some(value);
                }
            }
            ReleaseRetained::Row(value) => {
                if value.capacity() != 0 {
                    owner.row = Some(value);
                }
            }
            ReleaseRetained::Rows(value) => {
                if value.capacity() != 0 {
                    owner.rows = Some(value);
                }
            }
            ReleaseRetained::Parts(value) => owner.frame = Some(Open::Triple(value)),
            ReleaseRetained::Binding(value) => owner.frame = Some(Open::Binding(value)),
            ReleaseRetained::Scratch(value) => owner.scratch = Some(value),
        }
        owner
    }

    // One current row, its preceding rows, the single head, and the shared
    // scratch belong to distinct scopes. Nested frames stay in their original
    // parent slots; a physical refusal aborts decoding before another cell starts.
    fn merge(&mut self, mut other: Self) {
        macro_rules! merge_slot {
            ($slot:ident) => {
                if other.$slot.is_some() {
                    assert!(
                        self.$slot.is_none(),
                        "one original JSON cleanup owner per scope"
                    );
                    self.$slot = other.$slot.take();
                }
            };
        }
        // A producer's take() leaves a zero-capacity placeholder of the same
        // vector type. It has no elements or original allocation to retain.
        macro_rules! merge_vector {
            ($slot:ident) => {
                if self.$slot.is_some() && other.$slot.is_some() {
                    if self
                        .$slot
                        .as_ref()
                        .is_some_and(|value| value.capacity() == 0)
                    {
                        self.$slot = None;
                    } else if other
                        .$slot
                        .as_ref()
                        .is_some_and(|value| value.capacity() == 0)
                    {
                        other.$slot = None;
                    }
                }
                merge_slot!($slot);
            };
        }
        merge_vector!(strings);
        merge_slot!(text);
        merge_vector!(row);
        merge_vector!(rows);
        merge_slot!(frame);
        merge_slot!(scratch);
        merge_slot!(term);
        if let Some(discarded) = other.primary.take() {
            if self.primary.is_some() {
                // The caller already selected its cause by projected column.
                // A failed disposal returns the discarded cause as its own
                // primary; keep that payload without changing the selection.
                self.retain_superseded(discarded);
            } else {
                self.primary = Some(discarded);
            }
        }
        if let Some(discarded) = other.superseded.take() {
            self.retain_superseded(discarded);
        }
    }

    fn retain_superseded(&mut self, discarded: NativePrimary) {
        if matches!(
            &discarded,
            NativePrimary::Format(ReadRefusal {
                reason: ReadReason::Token { .. } | ReadReason::Predicate(_),
                ..
            }) | NativePrimary::LanguageTag { .. }
                | NativePrimary::TermRelease { term: Some(_), .. }
        ) {
            // One selected refusal can coexist with one discarded/replaced
            // refusal whose disposal failed. That physical failure ends the
            // row before another cell or later syntax can add another owner.
            assert!(self.superseded.is_none(), "one stopped semantic disposal");
            self.superseded = Some(discarded);
        }
    }

    fn absorb_cleanup(&mut self, error: Error) -> StorageError {
        match error {
            Error::Storage(cause) => cause,
            Error::TermRelease { term, cause } => {
                assert!(self.term.is_none(), "one failed completed JSON term");
                self.term = term;
                cause
            }
            Error::JsonRelease { owner, cause } => {
                self.merge(owner);
                cause
            }
            _ => unreachable!("native cleanup returns a physical refusal"),
        }
    }

    fn with_prior(mut self, error: Error) -> (Self, Option<StorageError>) {
        match error {
            Error::Storage(cause) => {
                if self.primary.is_none() {
                    self.primary = Some(
                        NativePrimary::from_error(Error::Storage(cause))
                            .expect("native storage refusal"),
                    );
                }
                (self, Some(cause))
            }
            Error::JsonRelease { owner, cause } => {
                self.merge(owner);
                (self, Some(cause))
            }
            Error::TermRelease { term, cause } => {
                assert!(self.term.is_none(), "one failed completed JSON term");
                self.term = term;
                (self, Some(cause))
            }
            error => {
                let primary =
                    NativePrimary::from_error(error).expect("a non-container native error");
                if let Some(previous) = self.primary.take() {
                    self.retain_superseded(previous);
                }
                self.primary = Some(primary);
                (self, None)
            }
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        if self.frame.is_some() {
            "frame"
        } else if self.row.is_some() {
            "row"
        } else if self.rows.is_some() {
            "rows"
        } else if self.strings.is_some() {
            "strings"
        } else if self.scratch.is_some() {
            "scratch"
        } else {
            "completed"
        }
    }

    fn original_error(&self) -> Option<&Error> {
        let open = self
            .frame
            .as_ref()
            .or_else(|| self.scratch.as_ref()?.open.last())?;
        let finished = match open {
            Open::Binding(fields) => &fields.finished,
            Open::Triple(parts) => &parts.finished,
        };
        finished.as_ref().and_then(|result| result.as_ref().err())
    }

    pub(crate) fn primary<'a>(
        &'a self,
        cause: &'a StorageError,
    ) -> &'a (dyn purrdf_hash::error::NativeError + 'static) {
        if let Some(primary) = &self.primary {
            primary
        } else if let Some(error) = self.original_error() {
            error
        } else {
            cause
        }
    }

    fn cleanup_containers<S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let result = (|| {
            if let Some(value) = self.strings.take() {
                release_strings(value, memory)?;
            }
            if let Some(value) = self.text.take() {
                memory.release_string(value).map_err(Error::Storage)?;
            }
            if let Some(value) = self.rows.take() {
                release_rows(value, memory)?;
            }
            if let Some(value) = self.row.take() {
                release_row(value, memory)?;
            }
            if let Some(value) = self.term.take() {
                release_term(value, memory)?;
            }
            if let Some(value) = self.frame.take() {
                match value {
                    Open::Binding(fields) => release_binding(fields, memory)?,
                    Open::Triple(parts) => release_parts(parts, memory)?,
                }
            }
            if let Some(value) = self.scratch.take() {
                release_scratch(value, memory)?;
            }
            if let Some(value) = self.superseded.take()
                && let Err((retained, cause)) = value.release_with_memory(memory)
            {
                self.superseded = retained;
                return Err(Error::Storage(cause));
            }
            Ok(())
        })();
        result.map_err(|error| self.absorb_cleanup(error))
    }

    /// Retry destruction under the containers' original admission account.
    ///
    /// # Errors
    /// Returns the first cleanup refusal with every remaining original owner.
    pub fn release_with_memory<S: Admission + ?Sized>(
        mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), Error> {
        let result = self.cleanup_containers(memory).and_then(|()| {
            if let Some(value) = self.primary.take()
                && let Err((retained, cause)) = value.release_with_memory(memory)
            {
                self.primary = retained;
                return Err(cause);
            }
            Ok(())
        });
        result.map_err(|cause| Error::JsonRelease { owner: self, cause })
    }
}

/// Stage separate row/refusal/output owners before cleanup can fail. A physical
/// refusal only moves the remaining owners; it never starts another census.
fn finish_cleanup<T, S: Admission + ?Sized>(
    mut pending: JsonRelease,
    result: Step<T>,
    retain_output: impl FnOnce(T, &mut JsonRelease),
    memory: &mut Memory<'_, S>,
) -> Step<T> {
    let (error, outer) = match result {
        Ok(Ok(value)) => {
            assert!(
                pending.primary.is_none() && pending.superseded.is_none(),
                "successful decoding has no pending refusal"
            );
            return match pending.cleanup_containers(memory) {
                Ok(()) => Ok(Ok(value)),
                Err(cause) => {
                    retain_output(value, &mut pending);
                    Err(Error::JsonRelease {
                        owner: pending,
                        cause,
                    })
                }
            };
        }
        Ok(Err(error)) => (error, false),
        Err(error) => (error, true),
    };
    let (mut pending, original_physical) = pending.with_prior(error);
    if let Some(cause) = original_physical {
        return Err(Error::JsonRelease {
            owner: pending,
            cause,
        });
    }
    match pending.cleanup_containers(memory) {
        Err(cause) => Err(Error::JsonRelease {
            owner: pending,
            cause,
        }),
        Ok(()) => {
            let error = pending
                .primary
                .take()
                .expect("the original native refusal is retained")
                .into_error();
            if outer { Err(error) } else { Ok(Err(error)) }
        }
    }
}

fn retain_plain_result<T>(
    mut pending: JsonRelease,
    result: Result<T, Error>,
    retain_output: impl FnOnce(T, &mut JsonRelease),
    cause: StorageError,
) -> Result<T, Error> {
    let cause = match result {
        Ok(value) => {
            retain_output(value, &mut pending);
            cause
        }
        Err(error) => {
            let (retained, original_physical) = pending.with_prior(error);
            pending = retained;
            original_physical.unwrap_or(cause)
        }
    };
    Err(Error::JsonRelease {
        owner: pending,
        cause,
    })
}

fn finish_plain<T, S: Admission + ?Sized>(
    pending: JsonRelease,
    result: Result<T, Error>,
    retain_output: impl FnOnce(T, &mut JsonRelease),
    memory: &mut Memory<'_, S>,
) -> Result<T, Error> {
    finish_cleanup(pending, result.map(Ok), retain_output, memory).and_then(|result| result)
}

/// A decoded `SELECT` result set: ordered variable names plus dense rows. A
/// `None` cell is an unbound (absent) binding for that variable in that row.
///
/// This is the structural inverse of [`purrdf_core::SparqlResult::Solutions`]
/// and the shape the SERVICE evaluator interns into a solution sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSolutions {
    /// The result variables, in `head.vars` order.
    pub variables: Vec<String>,
    /// One row per binding; `rows[i][j]` is the value of `variables[j]`.
    pub rows: Vec<Vec<Option<TermValue>>>,
}

/// A bounded decode result. `truncated` means the document contained at least one binding
/// beyond the supplied intermediate-cell ceiling; `solutions.rows` is the ordered prefix
/// that fit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedParsedSolutions {
    /// The decoded, cell-bounded solution prefix.
    pub solutions: ParsedSolutions,
    /// Whether a further binding was present and deliberately not materialized.
    pub truncated: bool,
}

/// Parse a SPARQL Results JSON `SELECT` document into [`ParsedSolutions`].
///
/// # Examples
///
/// ```
/// use purrdf_core::TermValue;
/// use purrdf_sparql_results::from_json;
///
/// let doc = br#"{
///   "head": { "vars": ["s"] },
///   "results": { "bindings": [
///     { "s": { "type": "uri", "value": "http://example.org/alice" } }
///   ] }
/// }"#;
///
/// let parsed = from_json(doc).expect("well-formed SRJ");
/// assert_eq!(parsed.variables, ["s"]);
/// assert_eq!(
///     parsed.rows,
///     [vec![Some(TermValue::Iri("http://example.org/alice".to_string()))]],
/// );
/// ```
///
/// # Errors
///
/// Returns [`Error::Json`] on malformed JSON, or [`Error::Format`] on a non-object document, an `ASK`
/// (`boolean`) document (use [`from_json_boolean`]), or a binding object whose
/// `type`/`value` shape is invalid.
pub fn from_json(bytes: &[u8]) -> Result<ParsedSolutions, Error> {
    let mut storage = Resident;
    from_json_with_memory(bytes, None, &mut Memory::new(&mut storage))
        .map(|parsed| parsed.solutions)
        .map_err(|error| match error {
            ReadError::Lexical(error) => error,
            ReadError::Storage(cause) => Error::Storage(cause),
        })
}

/// The original result shape refusal or physical producer failure.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    /// JSON syntax or SPARQL result shape, preserving its original native cause.
    Lexical(Error),
    /// Original admission, layout or allocator refusal.
    Storage(StorageError),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lexical(error) => error.fmt(f),
            Self::Storage(error) => error.fmt(f),
        }
    }
}
impl purrdf_hash::error::StructuredError for ReadError {
    purrdf_hash::error_visit! {
        Lexical(cause) => "sparql-results-read-lexical" { "cause" => Cause(cause) },
        Storage(cause) => "sparql-results-read-storage" { "cause" => Cause(cause) },
    }
}

purrdf_lex::error_source!(ReadError { Lexical, Storage });

/// Decode the original SELECT producer under caller-owned physical admission.
/// The caller keeps this original grant with the returned payload or refusal.
/// `max_cells` selects the existing bounded-prefix law; `None` selects the
/// full decoder's syntax-before-shape ordering.
/// # Errors
/// Returns the original syntax/shape refusal or typed storage failure.
pub fn from_json_with_memory<S: Admission + ?Sized>(
    bytes: &[u8],
    max_cells: Option<u64>,
    memory: &mut Memory<'_, S>,
) -> Result<BoundedParsedSolutions, ReadError> {
    let initial_bytes = memory.admitted_bytes();
    let result = (|| {
        if let Some(max_cells) = max_cells {
            return bounded_document(bytes, max_cells, memory);
        }
        let mut reader =
            Reader::from_slice(bytes, LIMITS).map_err(|error| syntax_with_memory(error, memory))?;
        let result = select_document(&mut reader, memory).and_then(|result| result);
        let result = match reader.release_with_memory(memory) {
            Ok(()) => result,
            Err(cause) => retain_plain_result(
                JsonRelease::default(),
                result,
                |solutions, pending| {
                    pending.strings = Some(solutions.variables);
                    pending.rows = Some(solutions.rows);
                },
                cause,
            ),
        };
        result.map(|solutions| BoundedParsedSolutions {
            solutions,
            truncated: false,
        })
    })();
    // On grammar/storage failure no parser-owned output survives this scope.
    // All local strings, frames and rows have already been destroyed. JSON's
    // native grammar error and StorageError are inline, so refund the remaining
    // transient charge while preserving the caller's pre-existing ownership.
    if matches!(&result, Err(Error::Json(_) | Error::Storage(_))) {
        let transient = memory
            .admitted_bytes()
            .checked_sub(initial_bytes)
            .ok_or(ReadError::Storage(StorageError::SizeOverflow))?;
        let refund = memory.release_bytes(transient).map_err(ReadError::Storage);
        if let Err(Error::Storage(error)) = &result {
            // Match Memory::release_vec_after: a refusal to shrink keeps the
            // original grant alive and never replaces the first physical fault.
            return Err(ReadError::Storage(*error));
        }
        refund?;
    }
    result.map_err(|error| match error {
        Error::Storage(error) => ReadError::Storage(error),
        error => ReadError::Lexical(error),
    })
}

/// Parse a SPARQL Results JSON `SELECT` document without materializing more than
/// `max_cells` cells (`rows * head.vars.len()`).
///
/// The document is scanned twice. The first pass reads only `head.vars`; the second decodes
/// bindings in source order until the inclusive ceiling is full, then structurally skips
/// the suffix without constructing its JSON tree or owned RDF terms. Two passes over bytes
/// are cheaper than building an attacker-sized response tree, and retain JSON's
/// order-independence (`results` may precede `head`).
///
/// A zero-column result has zero cells regardless of row count and is therefore decoded in
/// full. Callers that need to bound unit rows must use a row/answer governor.
///
/// # Errors
///
/// Returns [`Error::Format`] under the same structural conditions as [`from_json`]. JSON in
/// the deliberately skipped over-limit suffix is still syntax-validated, but its SPARQL
/// binding shape is not decoded because the cell governor has already refused that work.
pub fn from_json_bounded(bytes: &[u8], max_cells: u64) -> Result<BoundedParsedSolutions, Error> {
    bounded_document(bytes, max_cells, &mut Memory::new(&mut Resident))
}

fn bounded_document<S: Admission + ?Sized>(
    bytes: &[u8],
    max_cells: u64,
    memory: &mut Memory<'_, S>,
) -> Result<BoundedParsedSolutions, Error> {
    let variables = scan_variables(bytes, memory)?;
    let row_limit = if variables.is_empty() {
        None
    } else {
        let width = u64::try_from(variables.len()).unwrap_or(u64::MAX);
        Some(usize::try_from(max_cells / width).unwrap_or(usize::MAX))
    };
    let (rows, truncated) = match scan_bindings(bytes, &variables, row_limit, memory) {
        Ok(rows) => rows,
        Err(error) => {
            return finish_cleanup(
                JsonRelease::new(ReleaseRetained::Strings(variables)),
                Err(error),
                |_: BoundedParsedSolutions, _| {},
                memory,
            )
            .and_then(|result| result);
        }
    };
    Ok(BoundedParsedSolutions {
        solutions: ParsedSolutions { variables, rows },
        truncated,
    })
}

/// First bounded-decoder pass: locate and decode `head.vars`, skipping every other value
/// without building a JSON tree.
fn scan_variables<S: Admission + ?Sized>(
    bytes: &[u8],
    memory: &mut Memory<'_, S>,
) -> Result<Vec<String>, Error> {
    let mut reader =
        Reader::from_slice(bytes, LIMITS).map_err(|error| syntax_with_memory(error, memory))?;
    let mut variables = None;
    let result = (|| {
        open_object(&mut reader, ReadShape::TopObject, memory)?;
        while let Some(key) = next_key(&mut reader, memory)? {
            if key == "head" && variables.is_none() {
                variables = Some(head_variables(&mut reader, memory)?);
            } else {
                reader
                    .skip_value_with_memory(memory)
                    .map_err(|error| syntax_with_memory(error, memory))?;
            }
        }
        reader
            .finish_with_memory(memory)
            .map_err(|error| syntax_with_memory(error, memory))?;
        variables
            .take()
            .ok_or_else(|| fmt_with_memory(ReadShape::Head, memory))
    })();
    let mut pending = JsonRelease::default();
    pending.strings = variables;
    if let Err(cause) = reader.release_with_memory(memory) {
        return retain_plain_result(
            pending,
            result,
            |strings, pending| pending.strings = Some(strings),
            cause,
        );
    }
    finish_plain(
        pending,
        result,
        |strings, pending| pending.strings = Some(strings),
        memory,
    )
}

/// Second bounded-decoder pass: locate `results.bindings`, materializing only the prefix
/// admitted by `row_limit`.
fn scan_bindings<S: Admission + ?Sized>(
    bytes: &[u8],
    variables: &[String],
    row_limit: Option<usize>,
    memory: &mut Memory<'_, S>,
) -> BoundedRowsResult {
    let mut reader =
        Reader::from_slice(bytes, LIMITS).map_err(|error| syntax_with_memory(error, memory))?;
    let mut found = None;
    let result = (|| {
        open_object(&mut reader, ReadShape::TopObject, memory)?;
        let mut saw_boolean = false;
        while let Some(key) = next_key(&mut reader, memory)? {
            match key {
                "boolean" => {
                    saw_boolean = true;
                    reader
                        .skip_value_with_memory(memory)
                        .map_err(|error| syntax_with_memory(error, memory))?;
                }
                "results" if found.is_none() => {
                    found = Some(bounded_results(&mut reader, variables, row_limit, memory)?);
                }
                _ => {
                    reader
                        .skip_value_with_memory(memory)
                        .map_err(|error| syntax_with_memory(error, memory))?;
                }
            }
        }
        reader
            .finish_with_memory(memory)
            .map_err(|error| syntax_with_memory(error, memory))?;
        if saw_boolean {
            return Err(fmt_with_memory(ReadShape::Select, memory));
        }
        found
            .take()
            .ok_or_else(|| fmt_with_memory(ReadShape::Results, memory))
    })();
    let mut pending = JsonRelease::default();
    if let Some((rows, _)) = found {
        pending.rows = Some(rows);
    }
    let result = match reader.release_with_memory(memory) {
        Ok(()) => result,
        Err(cause) => {
            return retain_plain_result(
                pending,
                result,
                |(rows, _), pending| {
                    pending.rows = Some(rows);
                },
                cause,
            );
        }
    };
    finish_cleanup(
        pending,
        result.map(Ok),
        |(rows, _), pending| {
            assert!(
                pending.rows.is_none(),
                "successful bounded rows moved out before cleanup"
            );
            pending.rows = Some(rows);
        },
        memory,
    )
    .and_then(|result| result)
}

/// Parse a SPARQL Results JSON `ASK` document into its boolean.
///
/// # Examples
///
/// ```
/// use purrdf_sparql_results::from_json_boolean;
///
/// let verdict = from_json_boolean(br#"{ "head": {}, "boolean": true }"#)
///     .expect("well-formed ASK document");
/// assert!(verdict);
/// ```
///
/// # Errors
///
/// Returns [`Error::Json`] on malformed JSON, or [`Error::Format`] on a document without a boolean
/// `boolean` field.
pub fn from_json_boolean(bytes: &[u8]) -> Result<bool, Error> {
    let doc = read_document(bytes)?;
    let obj = doc.as_object().ok_or_else(|| {
        ReadRefusal::shape(crate::SparqlResultsFormat::Json, ReadShape::TopObject)
    })?;
    match obj.get("boolean") {
        Some(Value::Bool(b)) => Ok(*b),
        _ => Err(ReadRefusal::shape(
            crate::SparqlResultsFormat::Json,
            ReadShape::Boolean,
        )),
    }
}

/// Decode the additive `purrdf` provenance extension a
/// [`ProvenanceNamespace`]-keyed SRJ document carries, or
/// [`ResultProvenance::default`] when no such top-level member is present.
///
/// This is the inverse of [`crate::json::to_json`]'s additive extension (see
/// [`crate::model::ProvenanceNamespace`]'s module docs): the writer emits the
/// member keyed under the caller-supplied namespace's `prefix`, but the KEY
/// SPELLING is not what identifies the member as this caller's own — a bare
/// string like `"prov"` has no uniqueness guarantee (the actual W3C PROV
/// namespace, `http://www.w3.org/ns/prov#`, is commonly bound to exactly that
/// prefix). Identity is the `"namespace"` field the writer records INSIDE the
/// member (the JSON twin of the XML writer's `xmlns:{prefix}="{iri}"`
/// declaration): every top-level object-valued member is scanned, and the
/// first one whose own `"namespace"` field equals `namespace.iri()` is
/// decoded — regardless of which key it happens to be spelled under. This
/// mirrors [`crate::xml_read::provenance_from_xml`]'s namespace-URI match
/// exactly: a document that writes this caller's IRI under a DIFFERENT
/// top-level key still decodes correctly, and a document that reuses this
/// caller's PREFIX spelling for an unrelated namespace is correctly not read
/// as this caller's extension. A document with no member recording
/// `namespace.iri()` (never written, or written under a different namespace)
/// decodes to the empty provenance, exactly like a document nothing ever
/// populated.
///
/// The `queryForm` field is read to VALIDATE the member's shape but is not
/// itself carried in [`ResultProvenance`] (it is derived from the result kind
/// on write, not caller data).
///
/// # Errors
///
/// Returns [`Error::Json`] on malformed JSON, or [`Error::Format`] on a non-object document or a
/// member recording `namespace.iri()` whose shape does not match the
/// writer's (`queryForm`/`queryHash`/`engine`/`solutions[].sources[]`, all
/// strings).
pub fn provenance_from_json(
    bytes: &[u8],
    namespace: &ProvenanceNamespace,
) -> Result<ResultProvenance, Error> {
    let doc = read_document(bytes)?;
    let obj = doc.as_object().ok_or_else(|| {
        ReadRefusal::shape(crate::SparqlResultsFormat::Json, ReadShape::TopObject)
    })?;
    let iri = namespace.iri();
    let Some(member_obj) = obj.iter().find_map(|(_, value)| {
        let candidate = value.as_object()?;
        let recorded = candidate.get("namespace").and_then(Value::as_str)?;
        (recorded == iri).then_some(candidate)
    }) else {
        return Ok(ResultProvenance::default());
    };
    let query_hash = member_obj
        .get("queryHash")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let engine = member_obj
        .get("engine")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let solutions = match member_obj.get("solutions") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                let item_obj = item.as_object().ok_or_else(|| {
                    ReadRefusal::shape(
                        crate::SparqlResultsFormat::Json,
                        ReadShape::ProvenanceSolution,
                    )
                })?;
                let sources = match item_obj.get("sources") {
                    Some(Value::Array(values)) => values
                        .iter()
                        .map(|v| {
                            v.as_str().map(str::to_owned).ok_or_else(|| {
                                ReadRefusal::shape(
                                    crate::SparqlResultsFormat::Json,
                                    ReadShape::ProvenanceSource,
                                )
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    _ => Vec::new(),
                };
                Ok(SolutionProvenance { sources })
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => Vec::new(),
    };
    Ok(ResultProvenance {
        query_hash,
        engine,
        solutions,
    })
}

/// A `SELECT` document, read to its end: its solutions, or the shape refusal
/// that outranks every other one it holds.
///
/// `head` may follow `results`. When it does, the `bindings` array is checked
/// where it stands and its span kept, and its rows are decoded once `head.vars`
/// is known.
fn select_document<S: Admission + ?Sized>(
    reader: &mut Reader<'_>,
    memory: &mut Memory<'_, S>,
) -> Step<ParsedSolutions> {
    if reader.peek_kind() != Some(Kind::Object) {
        reader
            .check_value_with_memory(memory)
            .map_err(|error| syntax_with_memory(error, memory))?;
        reader
            .finish_with_memory(memory)
            .map_err(|error| syntax_with_memory(error, memory))?;
        return Ok(Err(fmt_with_memory(ReadShape::TopObject, memory)));
    }
    reader
        .next_event_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?;
    let mut scratch = RowScratch::default();
    let mut boolean = false;
    let mut head: Option<Shaped<Vec<String>>> = None;
    let mut results: Option<Shaped<Bindings>> = None;
    let result = (|| {
        while let Some(key) = next_key(reader, memory)? {
            match key {
                "boolean" => {
                    boolean = true;
                    reader
                        .check_value_with_memory(memory)
                        .map_err(|error| syntax_with_memory(error, memory))?;
                }
                "head" if head.is_none() => head = Some(read_head(reader, memory)?),
                "results" if results.is_none() => {
                    // Rows are worth decoding only for a document that can still
                    // be a `SELECT` whose variables are known.
                    let variables = match &head {
                        Some(Ok(variables)) if !boolean => Some(variables.as_slice()),
                        _ => None,
                    };
                    results = Some(read_results(reader, variables, &mut scratch, memory)?);
                }
                _ => {
                    reader
                        .check_value_with_memory(memory)
                        .map_err(|error| syntax_with_memory(error, memory))?;
                }
            }
        }
        reader
            .finish_with_memory(memory)
            .map_err(|error| syntax_with_memory(error, memory))?;
        if boolean {
            return Ok(Err(fmt_with_memory(ReadShape::Select, memory)));
        }
        let variables = match head.as_ref() {
            Some(Ok(variables)) => variables,
            Some(Err(_)) => {
                let Some(Err(refusal)) = head.take() else {
                    unreachable!("the refused head exists");
                };
                return Ok(Err(refusal));
            }
            None => return Ok(Err(fmt_with_memory(ReadShape::Head, memory))),
        };
        let rows = match results.take() {
            None => return Ok(Err(fmt_with_memory(ReadShape::Results, memory))),
            Some(Err(refusal)) => return Ok(Err(refusal)),
            Some(Ok(Bindings::Decoded(rows))) => rows,
            Some(Ok(Bindings::Unread(span))) => {
                let mut rows = Reader::new(&reader.text()[span], LIMITS);
                let result = binding_rows(&mut rows, variables, &mut scratch, memory);
                let result = match rows.release_with_memory(memory) {
                    Ok(()) => result,
                    Err(cause) => retain_plain_result(
                        JsonRelease::default(),
                        result.and_then(|result| result),
                        |rows, pending| {
                            pending.rows = Some(rows);
                        },
                        cause,
                    )
                    .map(Ok),
                };
                result?
            }
        };
        let rows = match rows {
            Ok(rows) => rows,
            Err(error) => return Ok(Err(error)),
        };
        let variables = head
            .take()
            .expect("the projected head exists")
            .expect("the projected head was checked above");
        Ok(Ok(ParsedSolutions { variables, rows }))
    })();
    let mut pending = JsonRelease::new(ReleaseRetained::Scratch(scratch));
    if let Some(head) = head {
        match head {
            Ok(variables) => pending.strings = Some(variables),
            Err(error) => pending = pending.with_prior(error).0,
        }
    }
    if let Some(results) = results {
        match results {
            Ok(Bindings::Unread(_)) => {}
            Ok(Bindings::Decoded(Ok(rows))) => pending.rows = Some(rows),
            Ok(Bindings::Decoded(Err(error))) | Err(error) => pending = pending.with_prior(error).0,
        }
    }
    finish_cleanup(
        pending,
        result,
        |solutions, pending| {
            assert!(
                pending.strings.is_none() && pending.rows.is_none(),
                "published solutions moved their original head and rows"
            );
            pending.strings = Some(solutions.variables);
            pending.rows = Some(solutions.rows);
        },
        memory,
    )
}

/// The `results.bindings` array as [`select_document`] found it.
enum Bindings {
    /// Decoded where it stood: its rows, or its first row's refusal.
    Decoded(Shaped<Vec<BindingRow>>),
    /// Checked where it stood, before `head.vars` was known: its span.
    Unread(Range<usize>),
}

/// A `head` value: its `vars`, or the refusal it earns.
fn read_head<S: Admission + ?Sized>(
    reader: &mut Reader<'_>,
    memory: &mut Memory<'_, S>,
) -> Step<Vec<String>> {
    if reader.peek_kind() != Some(Kind::Object) {
        reader
            .check_value_with_memory(memory)
            .map_err(|error| syntax_with_memory(error, memory))?;
        return Ok(Err(fmt_with_memory(ReadShape::Head, memory)));
    }
    reader
        .next_event_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?;
    let mut variables: Option<Shaped<Vec<String>>> = None;
    let result = (|| {
        while let Some(key) = next_key(reader, memory)? {
            if key == "vars" && variables.is_none() && reader.peek_kind() == Some(Kind::Array) {
                variables = Some(string_items(reader, ReadShape::Variable, memory)?);
            } else {
                if key == "vars" && variables.is_none() {
                    variables = Some(Ok(Vec::new()));
                }
                reader
                    .check_value_with_memory(memory)
                    .map_err(|error| syntax_with_memory(error, memory))?;
            }
        }
        Ok(variables.take().unwrap_or_else(|| Ok(Vec::new())))
    })();
    let mut pending = JsonRelease::default();
    if let Some(variables) = variables {
        match variables {
            Ok(strings) => pending.strings = Some(strings),
            Err(error) => pending = pending.with_prior(error).0,
        }
    }
    finish_cleanup(
        pending,
        result,
        |strings, pending| pending.strings = Some(strings),
        memory,
    )
}

/// A string-only array: its strings, or `item_error` at its first other item.
fn string_items<S: Admission + ?Sized>(
    reader: &mut Reader<'_>,
    item_error: ReadShape,
    memory: &mut Memory<'_, S>,
) -> Step<Vec<String>> {
    reader
        .next_event_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?;
    let mut strings = Vec::new();
    let mut text = None;
    let mut refusal = None;
    let result = (|| {
        while reader
            .next_item()
            .map_err(|error| syntax_with_memory(error, memory))?
        {
            if refusal.is_none() && reader.peek_kind() == Some(Kind::String) {
                let Event::String(item) = reader
                    .next_event_with_memory(memory)
                    .map_err(|error| syntax_with_memory(error, memory))?
                else {
                    unreachable!("a string begins at the cursor");
                };
                text = Some(owned_text(
                    item.decode_with_memory(memory)
                        .map_err(|error| syntax_with_memory(error, memory))?,
                    memory,
                )?);
                memory
                    .reserve_for_push(&mut strings)
                    .map_err(Error::Storage)?;
                strings.push(text.take().expect("the decoded original string exists"));
            } else {
                if refusal.is_none() {
                    let original = fmt_with_memory(item_error, memory);
                    let prefix = std::mem::take(&mut strings);
                    refusal = Some(
                        finish_cleanup(
                            JsonRelease::new(ReleaseRetained::Strings(prefix)),
                            Ok::<_, Error>(Err(original)),
                            |_: Vec<String>, _| {},
                            memory,
                        )?
                        .expect_err("the item was refused"),
                    );
                }
                reader
                    .check_value_with_memory(memory)
                    .map_err(|error| syntax_with_memory(error, memory))?;
            }
        }
        Ok(match refusal.take() {
            Some(error) => Err(error),
            None => Ok(std::mem::take(&mut strings)),
        })
    })();
    let mut pending = JsonRelease::new(ReleaseRetained::Strings(strings));
    pending.text = text;
    if let Some(error) = refusal {
        pending = pending.with_prior(error).0;
    }
    finish_cleanup(
        pending,
        result,
        |strings, pending| pending.strings = Some(strings),
        memory,
    )
}

/// A `results` value: its `bindings` array, decoded when `variables` are
/// known and checked otherwise, or the refusal it earns.
fn read_results<'a, S: Admission + ?Sized>(
    reader: &mut Reader<'a>,
    variables: Option<&[String]>,
    scratch: &mut RowScratch,
    memory: &mut Memory<'_, S>,
) -> Step<Bindings> {
    if reader.peek_kind() != Some(Kind::Object) {
        reader
            .check_value_with_memory(memory)
            .map_err(|error| syntax_with_memory(error, memory))?;
        return Ok(Err(fmt_with_memory(ReadShape::Results, memory)));
    }
    reader
        .next_event_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?;
    let mut bindings: Option<Shaped<Bindings>> = None;
    let result = (|| {
        while let Some(key) = next_key(reader, memory)? {
            if key != "bindings" || bindings.is_some() {
                reader
                    .check_value_with_memory(memory)
                    .map_err(|error| syntax_with_memory(error, memory))?;
                continue;
            }
            bindings = Some(if reader.peek_kind() != Some(Kind::Array) {
                reader
                    .check_value_with_memory(memory)
                    .map_err(|error| syntax_with_memory(error, memory))?;
                Err(fmt_with_memory(ReadShape::Bindings, memory))
            } else if let Some(variables) = variables {
                Ok(Bindings::Decoded(binding_rows(
                    reader, variables, scratch, memory,
                )?))
            } else {
                Ok(Bindings::Unread(
                    reader
                        .check_value_with_memory(memory)
                        .map_err(|error| syntax_with_memory(error, memory))?,
                ))
            });
        }
        Ok(bindings
            .take()
            .unwrap_or_else(|| Err(fmt_with_memory(ReadShape::Bindings, memory))))
    })();
    let mut pending = JsonRelease::default();
    if let Some(bindings) = bindings {
        match bindings {
            Ok(Bindings::Unread(_)) => {}
            Ok(Bindings::Decoded(Ok(rows))) => pending.rows = Some(rows),
            Ok(Bindings::Decoded(Err(error))) | Err(error) => pending = pending.with_prior(error).0,
        }
    }
    finish_cleanup(
        pending,
        result,
        |bindings, pending| {
            if let Bindings::Decoded(Ok(rows)) = bindings {
                pending.rows = Some(rows);
            }
        },
        memory,
    )
}

/// A `bindings` array, the cursor on its `[`: every row, or the first row's
/// refusal once the rest of the array is checked.
fn binding_rows<'a, S: Admission + ?Sized>(
    reader: &mut Reader<'a>,
    variables: &[String],
    scratch: &mut RowScratch,
    memory: &mut Memory<'_, S>,
) -> Step<Vec<BindingRow>> {
    reader
        .next_event_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?;
    let mut rows = Vec::new();
    let mut current = None;
    let mut refusal = None;
    let result = (|| {
        while reader
            .next_item()
            .map_err(|error| syntax_with_memory(error, memory))?
        {
            match binding_row(reader, variables, scratch, memory)? {
                Ok(row) => {
                    current = Some(row);
                    memory.reserve_for_push(&mut rows).map_err(Error::Storage)?;
                    rows.push(current.take().expect("the decoded original row exists"));
                }
                Err(error) => {
                    refusal = Some(error);
                    while reader
                        .next_item()
                        .map_err(|error| syntax_with_memory(error, memory))?
                    {
                        reader
                            .check_value_with_memory(memory)
                            .map_err(|error| syntax_with_memory(error, memory))?;
                    }
                    return Ok(Err(refusal.take().expect("the row was refused")));
                }
            }
        }
        Ok(Ok(std::mem::take(&mut rows)))
    })();
    let mut pending = JsonRelease::new(ReleaseRetained::Rows(rows));
    pending.row = current;
    if let Some(error) = refusal {
        pending = pending.with_prior(error).0;
    }
    finish_cleanup(
        pending,
        result,
        |rows, pending| {
            assert!(
                pending.rows.is_none(),
                "successful rows moved out before cleanup"
            );
            pending.rows = Some(rows);
        },
        memory,
    )
}

/// Buffers one row decode reuses from the last: the open binding objects and
/// which variables the row has bound.
#[derive(Debug, Default, PartialEq, Eq)]
struct RowScratch {
    open: Vec<Open>,
    bound: Vec<bool>,
}

/// One `results.bindings` entry, the cursor on it: a dense row over
/// `variables`, or the refusal of the cell of the earliest variable that has
/// one.
///
/// A cell is the FIRST member named after its variable; a later member of the
/// same name, like every member no variable names, is checked and not decoded.
fn binding_row<'a, S: Admission + ?Sized>(
    reader: &mut Reader<'a>,
    variables: &[String],
    scratch: &mut RowScratch,
    memory: &mut Memory<'_, S>,
) -> Step<BindingRow> {
    if reader.peek_kind() != Some(Kind::Object) {
        reader
            .check_value_with_memory(memory)
            .map_err(|error| syntax_with_memory(error, memory))?;
        return Ok(Err(fmt_with_memory(ReadShape::BindingEntry, memory)));
    }
    reader
        .next_event_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?;
    let mut row = Vec::new();
    let mut refusal: Option<(usize, Error)> = None;
    let result = (|| {
        memory
            .reserve(&mut row, variables.len())
            .map_err(Error::Storage)?;
        row.resize_with(variables.len(), || None);
        scratch.bound.clear();
        memory
            .reserve(&mut scratch.bound, variables.len())
            .map_err(Error::Storage)?;
        scratch.bound.resize(variables.len(), false);
        while let Some(raw) = reader
            .next_key_with_memory(memory)
            .map_err(|error| syntax_with_memory(error, memory))?
        {
            let key = raw
                .decode_with_memory(memory)
                .map_err(|error| syntax_with_memory(error, memory))?;
            let index = variables
                .iter()
                .position(|variable| variable == key.as_ref());
            release_text(key, memory)?;
            let Some(index) = index else {
                reader
                    .check_value_with_memory(memory)
                    .map_err(|error| syntax_with_memory(error, memory))?;
                continue;
            };
            if scratch.bound[index] {
                reader
                    .check_value_with_memory(memory)
                    .map_err(|error| syntax_with_memory(error, memory))?;
                continue;
            }
            let start = reader.offset();
            let cell = binding(reader, &mut scratch.open, memory)?;
            let end = reader.offset();
            let copied = cell.is_ok();
            match cell {
                Err(error)
                    if refusal
                        .as_ref()
                        .is_none_or(|(earliest, _)| index < *earliest) =>
                {
                    if let Some((_, previous)) = refusal.replace((index, error)) {
                        release_error(previous, memory)?;
                    }
                }
                Ok(term) => row[index] = Some(term),
                Err(error) => release_error(error, memory)?,
            }
            for column in index..variables.len() {
                if variables[column] == variables[index] {
                    scratch.bound[column] = true;
                    if column != index && copied {
                        let mut duplicate = Reader::new(&reader.text()[start..end], LIMITS);
                        let decoded = binding(&mut duplicate, &mut scratch.open, memory)
                            .and_then(|result| result);
                        let refund = duplicate.release_with_memory(memory);
                        match (decoded, refund) {
                            (Ok(term), refund) => {
                                row[column] = Some(term);
                                refund.map_err(Error::Storage)?;
                            }
                            (Err(error), Ok(())) => return Err(error),
                            (Err(error), Err(cause)) => {
                                return retain_plain_result(
                                    JsonRelease::default(),
                                    Err(error),
                                    |_: BindingRow, _| {},
                                    cause,
                                )
                                .map(Ok);
                            }
                        }
                    }
                }
            }
        }
        Ok(match refusal.take() {
            Some((_, error)) => Err(error),
            None => Ok(std::mem::take(&mut row)),
        })
    })();
    let mut pending = JsonRelease::new(ReleaseRetained::Row(row));
    if let Some((_, error)) = refusal {
        pending = pending.with_prior(error).0;
    }
    finish_cleanup(
        pending,
        result,
        |row, pending| {
            assert!(
                pending.row.is_none(),
                "successful row moved out before cleanup"
            );
            pending.row = Some(row);
        },
        memory,
    )
}

/// A binding object or a triple term's `value` object, open and being read.
#[derive(Debug, PartialEq, Eq)]
enum Open {
    Binding(Box<BindingFields>),
    Triple(Box<TripleParts>),
}

/// The members of a binding object that decide its term: the FIRST member of
/// each name, `None` when it has none.
#[derive(Debug, Default, PartialEq, Eq)]
struct BindingFields {
    kind: Option<Member>,
    value: Option<BindingValue>,
    language: Option<Member>,
    its_dir: Option<Member>,
    dir: Option<Member>,
    datatype: Option<Member>,
    finished: Option<Shaped<TermValue>>,
}

/// Borrowed lexical fields retain only their original reader-relative body span.
/// Cleanup never projects a span; normal finish uses the same binding reader text.
#[derive(Debug, PartialEq, Eq)]
enum Lexical {
    Borrowed(Range<usize>),
    Owned(String),
}

impl Lexical {
    fn text<'a>(&'a self, source: &'a str) -> &'a str {
        match self {
            Self::Borrowed(span) => &source[span.clone()],
            Self::Owned(text) => text,
        }
    }

    fn own<S: Admission + ?Sized>(
        &mut self,
        source: &str,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), Error> {
        if let Self::Borrowed(span) = self {
            let text = memory
                .string(&source[span.clone()])
                .map_err(Error::Storage)?;
            *self = Self::Owned(text);
        }
        Ok(())
    }

    fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        match self {
            Self::Borrowed(_) => Ok(()),
            Self::Owned(text) => memory.release_string(text),
        }
    }
}

/// A binding object's member whose value is a string when it is well formed.
#[derive(Debug, PartialEq, Eq)]
enum Member {
    /// The original decoded lexical owner or source span.
    String(Lexical),
    /// A value of another kind.
    Other,
}

impl Member {
    /// The string, when the member holds one.
    fn text<'a>(member: Option<&'a Self>, source: &'a str) -> Option<&'a str> {
        match member {
            Some(Self::String(text)) => Some(text.text(source)),
            Some(Self::Other) | None => None,
        }
    }
}

/// A binding object's `value` member.
#[derive(Debug, PartialEq, Eq)]
enum BindingValue {
    /// A string: an IRI, a label or a lexical form.
    String(Lexical),
    /// An object, read as a triple term's components whatever the binding's
    /// `type` turns out to be; only a `triple` binding consults them.
    Triple(Box<TripleParts>),
    /// Original completed result retained across a cleanup refusal.
    Finished(Shaped<TermValue>),
    /// Anything else.
    Other,
}

/// A triple term's components: the FIRST `subject`, `predicate` and `object`
/// member, each decoded or refused, and the one being read.
#[derive(Debug, Default, PartialEq, Eq)]
struct TripleParts {
    subject: Option<Shaped<TermValue>>,
    predicate: Option<Shaped<TermValue>>,
    object: Option<Shaped<TermValue>>,
    reading: Option<Component>,
    finished: Option<Shaped<TermValue>>,
    boxed: [Option<TermBox>; 3],
}

/// A triple term's component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Component {
    Subject,
    Predicate,
    Object,
}

impl TripleParts {
    fn slot(&mut self, component: Component) -> &mut Option<Shaped<TermValue>> {
        match component {
            Component::Subject => &mut self.subject,
            Component::Predicate => &mut self.predicate,
            Component::Object => &mut self.object,
        }
    }

    /// The triple term, or the refusal a decoder descending into `subject`,
    /// `predicate` and `object` in that order meets first; the predicate is
    /// held to be an IRI once its object has been decoded.
    fn finish<S: Admission + ?Sized>(&mut self, memory: &mut Memory<'_, S>) -> Shaped<TermValue> {
        // Leave successful components in their original slots until all three
        // are present. The caller releases those slots after a shape refusal.
        for (component, shape) in [
            (Component::Subject, ReadShape::Subject),
            (Component::Predicate, ReadShape::Predicate),
            (Component::Object, ReadShape::Object),
        ] {
            let slot = self.slot(component);
            match slot.as_ref() {
                None => return Err(fmt_with_memory(shape, memory)),
                Some(Err(_)) => return slot.take().expect("the refused component exists"),
                Some(Ok(_)) => {}
            }
        }
        if !matches!(self.predicate.as_ref(), Some(Ok(TermValue::Iri(_)))) {
            let p = self.predicate.take().expect("the predicate exists")?;
            return Err(Error::Format(ReadRefusal {
                format: crate::SparqlResultsFormat::Json,
                reason: ReadReason::Predicate(p),
            }));
        }
        for component in [Component::Subject, Component::Predicate, Component::Object] {
            let term = self
                .slot(component)
                .take()
                .expect("the checked component exists")?;
            match memory.boxed_retaining(term) {
                Ok(value) => self.boxed[component as usize] = Some(value.into()),
                Err((term, cause)) => {
                    *self.slot(component) = Some(Ok(term));
                    return Err(Error::Storage(cause));
                }
            }
        }
        Ok(TermValue::Triple {
            s: self.boxed[0]
                .take()
                .expect("the subject box was constructed"),
            p: self.boxed[1]
                .take()
                .expect("the predicate box was constructed"),
            o: self.boxed[2]
                .take()
                .expect("the object box was constructed"),
        })
    }
}

impl BindingFields {
    /// Keep every fallible intermediate in its original admitted field. Only
    /// after all native strings exist do infallible moves assemble the term.
    fn finish<S: Admission + ?Sized>(
        &mut self,
        source: &str,
        memory: &mut Memory<'_, S>,
    ) -> Shaped<TermValue> {
        let Some(kind) = Member::text(self.kind.as_ref(), source) else {
            return Err(fmt_with_memory(ReadShape::BindingType, memory));
        };
        match kind {
            "uri" | "bnode" => {
                let blank = kind == "bnode";
                own_binding_text(&mut self.value, source, memory)?;
                let value = take_binding_text(&mut self.value);
                Ok(if blank {
                    TermValue::Blank {
                        label: value,
                        scope: BlankScope::DEFAULT,
                    }
                } else {
                    TermValue::Iri(value)
                })
            }
            "literal" | "typed-literal" => {
                own_binding_text(&mut self.value, source, memory)?;
                let language = Member::text(self.language.as_ref(), source);
                if let Some(cause) = language.and_then(crate::error::language_tag_refusal) {
                    own_member(&mut self.language, source, memory)?;
                    return Err(Error::LanguageTag {
                        format: "SPARQL-JSON",
                        language: take_member(&mut self.language)
                            .expect("the rejected language exists"),
                        cause,
                    });
                }
                let has_language = language.is_some();
                let selected = if self.its_dir.is_some() {
                    &mut self.its_dir
                } else {
                    &mut self.dir
                };
                let direction = match Member::text(selected.as_ref(), source) {
                    Some(token) => match RdfTextDirection::from_str_token(token) {
                        Some(direction) => Some(direction),
                        None => {
                            return Err(ReadRefusal::token(
                                crate::SparqlResultsFormat::Json,
                                ReadToken::Direction,
                                token,
                                memory,
                            ));
                        }
                    },
                    None => None,
                };
                // Preserve the native successful order: release the selected
                // direction token before datatype construction, then own language.
                if let Some(Member::String(text)) = selected.take() {
                    text.release(memory).map_err(Error::Storage)?;
                }
                if Member::text(self.datatype.as_ref(), source).is_some() {
                    own_member(&mut self.datatype, source, memory)?;
                } else {
                    let datatype =
                        resolve_datatype(None, has_language, direction.is_some(), memory)?;
                    self.datatype = Some(Member::String(Lexical::Owned(datatype)));
                }
                own_member(&mut self.language, source, memory)?;
                Ok(TermValue::Literal {
                    lexical_form: take_binding_text(&mut self.value),
                    datatype: take_member(&mut self.datatype).expect("the native datatype exists"),
                    language: take_member(&mut self.language),
                    direction,
                })
            }
            "triple" => match self.value.take() {
                Some(BindingValue::Triple(mut parts)) => {
                    let term = parts.finish(memory);
                    release_finished_parts(parts, term, memory)
                }
                value => {
                    self.value = value;
                    Err(fmt_with_memory(ReadShape::TripleValue, memory))
                }
            },
            other => Err(ReadRefusal::token(
                crate::SparqlResultsFormat::Json,
                ReadToken::BindingType,
                other,
                memory,
            )),
        }
    }
}

fn own_member<S: Admission + ?Sized>(
    member: &mut Option<Member>,
    source: &str,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    if let Some(Member::String(text)) = member {
        text.own(source, memory)?;
    }
    Ok(())
}

fn take_member(member: &mut Option<Member>) -> Option<String> {
    match member.take() {
        Some(Member::String(Lexical::Owned(text))) => Some(text),
        Some(Member::String(Lexical::Borrowed(_))) => {
            unreachable!("the native field was owned before assembly")
        }
        _ => None,
    }
}

fn own_binding_text<S: Admission + ?Sized>(
    value: &mut Option<BindingValue>,
    source: &str,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    match value {
        Some(BindingValue::String(text)) => text.own(source, memory),
        _ => Err(fmt_with_memory(ReadShape::BindingValue, memory)),
    }
}

fn take_binding_text(value: &mut Option<BindingValue>) -> String {
    match value.take() {
        Some(BindingValue::String(Lexical::Owned(text))) => text,
        _ => unreachable!("the native lexical field was owned before assembly"),
    }
}

/// A member's value, the cursor on it: the string it holds, or
/// [`Member::Other`] once a value of another kind has been checked.
fn string_member<'a, S: Admission + ?Sized>(
    reader: &mut Reader<'a>,
    memory: &mut Memory<'_, S>,
) -> Result<Member, Error> {
    if reader.peek_kind() == Some(Kind::String) {
        let Event::String(text) = reader
            .next_event_with_memory(memory)
            .map_err(|error| syntax_with_memory(error, memory))?
        else {
            unreachable!("a string begins at the cursor");
        };
        let span = text.span();
        return text
            .decode_with_memory(memory)
            .map(|text| {
                Member::String(match text {
                    Cow::Borrowed(_) => Lexical::Borrowed(span),
                    Cow::Owned(text) => Lexical::Owned(text),
                })
            })
            .map_err(|error| syntax_with_memory(error, memory));
    }
    reader
        .check_value_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?;
    Ok(Member::Other)
}

/// One SPARQL-JSON binding object, the cursor on it: its [`TermValue`], or its
/// refusal once the whole value has been read.
///
/// An RDF 1.2 triple term nests further binding objects under its `subject`,
/// `predicate` and `object` members to any depth. They are read over `open`, a
/// heap stack of the binding and component objects being read, so the machine
/// stack used does not grow with the nesting depth. A binding's members may
/// come in any order, so each object is read whole before it is judged: a
/// `value` object is read as a triple term's components before the `type` that
/// says whether it is one may have been seen.
fn binding<'a, S: Admission + ?Sized>(
    reader: &mut Reader<'a>,
    open: &mut Vec<Open>,
    memory: &mut Memory<'_, S>,
) -> Step<TermValue> {
    open.clear();
    if reader.peek_kind() != Some(Kind::Object) {
        reader
            .check_value_with_memory(memory)
            .map_err(|error| syntax_with_memory(error, memory))?;
        return Ok(Err(fmt_with_memory(ReadShape::BindingObject, memory)));
    }
    reader
        .next_event_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?;
    memory.reserve_for_push(open).map_err(Error::Storage)?;
    let fields = memory
        .boxed(BindingFields::default())
        .map_err(Error::Storage)?;
    open.push(Open::Binding(fields));
    loop {
        let Some(key) = next_key(reader, memory)? else {
            // The innermost object is complete: hand it to the one enclosing
            // it, or it is the binding.
            match open.pop().expect("an object is open") {
                Open::Binding(mut fields) => {
                    let term = fields.finish(reader.text(), memory);
                    let term = release_finished_fields(fields, term, memory)?;
                    let term = match term {
                        Err(
                            error @ (Error::Storage(_)
                            | Error::TermRelease { .. }
                            | Error::JsonRelease { .. }),
                        ) => return Err(error),
                        term => term,
                    };
                    match open.last_mut() {
                        None => return Ok(term),
                        Some(Open::Triple(parts)) => {
                            let component = parts.reading.take().expect("a component was open");
                            *parts.slot(component) = Some(term);
                        }
                        Some(Open::Binding(_)) => {
                            unreachable!("a binding object opens only in a triple term's component")
                        }
                    }
                }
                Open::Triple(parts) => match open.last_mut() {
                    Some(Open::Binding(fields)) => fields.value = Some(BindingValue::Triple(parts)),
                    _ => unreachable!("a component object opens only as a binding's value"),
                },
            }
            continue;
        };
        match open.last_mut().expect("an object is open") {
            Open::Binding(fields) => {
                let slot = match key {
                    "type" => &mut fields.kind,
                    "xml:lang" => &mut fields.language,
                    "its:dir" => &mut fields.its_dir,
                    "dir" => &mut fields.dir,
                    "datatype" => &mut fields.datatype,
                    "value" if fields.value.is_none() => {
                        fields.value = Some(match reader.peek_kind() {
                            Some(Kind::Object) => {
                                reader
                                    .next_event_with_memory(memory)
                                    .map_err(|error| syntax_with_memory(error, memory))?;
                                memory.reserve_for_push(open).map_err(Error::Storage)?;
                                let parts = memory
                                    .boxed(TripleParts::default())
                                    .map_err(Error::Storage)?;
                                open.push(Open::Triple(parts));
                                continue;
                            }
                            Some(Kind::String) => match string_member(reader, memory)? {
                                Member::String(text) => BindingValue::String(text),
                                Member::Other => unreachable!("a string begins at the cursor"),
                            },
                            _ => {
                                reader
                                    .check_value_with_memory(memory)
                                    .map_err(|error| syntax_with_memory(error, memory))?;
                                BindingValue::Other
                            }
                        });
                        continue;
                    }
                    _ => {
                        reader
                            .check_value_with_memory(memory)
                            .map_err(|error| syntax_with_memory(error, memory))?;
                        continue;
                    }
                };
                if slot.is_some() {
                    reader
                        .check_value_with_memory(memory)
                        .map_err(|error| syntax_with_memory(error, memory))?;
                } else {
                    *slot = Some(string_member(reader, memory)?);
                }
            }
            Open::Triple(parts) => {
                let component = match key {
                    "subject" => Component::Subject,
                    "predicate" => Component::Predicate,
                    "object" => Component::Object,
                    _ => {
                        reader
                            .check_value_with_memory(memory)
                            .map_err(|error| syntax_with_memory(error, memory))?;
                        continue;
                    }
                };
                if parts.slot(component).is_some() {
                    reader
                        .check_value_with_memory(memory)
                        .map_err(|error| syntax_with_memory(error, memory))?;
                } else if reader.peek_kind() == Some(Kind::Object) {
                    reader
                        .next_event_with_memory(memory)
                        .map_err(|error| syntax_with_memory(error, memory))?;
                    parts.reading = Some(component);
                    memory.reserve_for_push(open).map_err(Error::Storage)?;
                    let fields = memory
                        .boxed(BindingFields::default())
                        .map_err(Error::Storage)?;
                    open.push(Open::Binding(fields));
                } else {
                    reader
                        .check_value_with_memory(memory)
                        .map_err(|error| syntax_with_memory(error, memory))?;
                    *parts.slot(component) =
                        Some(Err(fmt_with_memory(ReadShape::BindingObject, memory)));
                }
            }
        }
    }
}

/// Resolve a literal's datatype: an explicit `datatype` wins; otherwise a
/// language-tagged literal is `rdf:langString` (or `rdf:dirLangString` with a
/// base direction), and a plain literal is `xsd:string`.
fn resolve_datatype<S: Admission + ?Sized>(
    datatype: Option<&str>,
    has_lang: bool,
    has_dir: bool,
    memory: &mut Memory<'_, S>,
) -> Result<String, Error> {
    memory
        .string(match datatype {
            Some(dt) => dt,
            None if has_lang => language_datatype_iri(has_dir),
            None => XSD_STRING,
        })
        .map_err(Error::Storage)
}

fn owned_text<S: Admission + ?Sized>(
    text: Cow<'_, str>,
    memory: &mut Memory<'_, S>,
) -> Result<String, Error> {
    match text {
        Cow::Borrowed(text) => memory.string(text).map_err(Error::Storage),
        Cow::Owned(text) => Ok(text),
    }
}

fn release_text<S: Admission + ?Sized>(
    text: Cow<'_, str>,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    match text {
        Cow::Borrowed(_) => Ok(()),
        Cow::Owned(text) => memory.release_string(text).map_err(Error::Storage),
    }
}

fn release_error<S: Admission + ?Sized>(
    error: Error,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    match NativePrimary::from_error(error) {
        Ok(native) => {
            native
                .release_with_memory(memory)
                .map_err(|(retained, cause)| match retained {
                    Some(native) => {
                        let (owner, original) =
                            JsonRelease::default().with_prior(native.into_error());
                        Error::JsonRelease {
                            owner,
                            cause: original.unwrap_or(cause),
                        }
                    }
                    None => Error::Storage(cause),
                })
        }
        Err((owner, original)) => owner
            .release_with_memory(memory)
            .map_err(|error| original_release_cause(error, original)),
    }
}

fn original_release_cause(error: Error, original: StorageError) -> Error {
    match error {
        Error::TermRelease { term, .. } => Error::TermRelease {
            term,
            cause: original,
        },
        Error::JsonRelease { owner, .. } => Error::JsonRelease {
            owner,
            cause: original,
        },
        Error::Storage(_) => Error::Storage(original),
        error => error,
    }
}

/// Pop only after reserving each original slot for restoration on failure.
/// A popped value leaves spare capacity, so restoring it cannot allocate.
fn release_items<T, S: Admission + ?Sized>(
    mut items: Vec<T>,
    memory: &mut Memory<'_, S>,
    mut release: impl FnMut(T, &mut Memory<'_, S>) -> Result<(), (Option<T>, StorageError)>,
) -> Result<(), (Vec<T>, StorageError)> {
    while let Some(item) = items.pop() {
        if let Err((retained, cause)) = release(item, memory) {
            if let Some(item) = retained {
                // The successful pop left this exact admitted slot vacant.
                items.push(item);
            }
            return Err((items, cause));
        }
    }
    memory
        .release_vec(items)
        .map_err(|cause| (Vec::new(), cause))
}

fn retained_release(retained: ReleaseRetained, cause: StorageError) -> Error {
    Error::JsonRelease {
        owner: JsonRelease::new(retained),
        cause,
    }
}

fn release_strings<S: Admission + ?Sized>(
    strings: Vec<String>,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    release_items(strings, memory, |text, memory| {
        memory.release_string(text).map_err(|cause| (None, cause))
    })
    .map_err(|(strings, cause)| {
        if strings.capacity() == 0 {
            Error::Storage(cause)
        } else {
            retained_release(ReleaseRetained::Strings(strings), cause)
        }
    })
}

fn release_row_owned<S: Admission + ?Sized>(
    row: BindingRow,
    memory: &mut Memory<'_, S>,
) -> Result<(), (BindingRow, StorageError)> {
    release_items(row, memory, |term, memory| match term {
        Some(term) => term
            .release_with_memory(memory)
            .map_err(|(term, cause)| (term.map(Some), cause)),
        None => Ok(()),
    })
}

fn release_row<S: Admission + ?Sized>(
    row: BindingRow,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    release_row_owned(row, memory).map_err(|(row, cause)| {
        if row.capacity() == 0 {
            Error::Storage(cause)
        } else {
            retained_release(ReleaseRetained::Row(row), cause)
        }
    })
}

fn release_rows<S: Admission + ?Sized>(
    rows: Vec<BindingRow>,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    release_items(rows, memory, |row, memory| {
        release_row_owned(row, memory).map_err(|(row, cause)| (Some(row), cause))
    })
    .map_err(|(rows, cause)| {
        if rows.capacity() == 0 {
            Error::Storage(cause)
        } else {
            retained_release(ReleaseRetained::Rows(rows), cause)
        }
    })
}

fn release_term<S: Admission + ?Sized>(
    term: TermValue,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    term.release_with_memory(memory)
        .map_err(|(term, cause)| Error::TermRelease { term, cause })
}

/// Recover the physical cause from these sealed native cleanup paths.
fn cleanup_cause(error: &Error) -> StorageError {
    match error {
        Error::Storage(cause)
        | Error::TermRelease { cause, .. }
        | Error::JsonRelease { cause, .. } => *cause,
        _ => unreachable!("native cleanup returns a physical refusal"),
    }
}

fn release_shaped<S: Admission + ?Sized>(
    slot: &mut Option<Shaped<TermValue>>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    match slot.take() {
        None => Ok(()),
        Some(Ok(term)) => match term.release_with_memory(memory) {
            Ok(()) => Ok(()),
            Err((term, cause)) => {
                *slot = term.map(Ok);
                Err(cause)
            }
        },
        Some(Err(error)) => match release_error(error, memory) {
            Ok(()) => Ok(()),
            Err(error) => {
                let cause = cleanup_cause(&error);
                *slot = Some(Err(error));
                Err(cause)
            }
        },
    }
}

fn release_parts_contents<S: Admission + ?Sized>(
    parts: &mut TripleParts,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    for component in [Component::Subject, Component::Predicate, Component::Object] {
        if let Some(value) = parts.boxed[component as usize].take() {
            // The original edge allocation dies before its exact box grant is
            // refunded; the unboxed payload stays in its original component slot.
            *parts.slot(component) = Some(Ok(value.into_inner()));
            memory.release_bytes(size_of::<TermValue>())?;
        }
        release_shaped(parts.slot(component), memory)?;
    }
    release_shaped(&mut parts.finished, memory)
}

fn release_parts<S: Admission + ?Sized>(
    mut parts: Box<TripleParts>,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    if let Err(cause) = release_parts_contents(&mut parts, memory) {
        return Err(retained_release(ReleaseRetained::Parts(parts), cause));
    }
    drop(parts);
    memory
        .release_bytes(size_of::<TripleParts>())
        .map_err(Error::Storage)
}

/// A completed result remains owned across the original container's cleanup.
fn release_finished_parts<S: Admission + ?Sized>(
    mut parts: Box<TripleParts>,
    result: Shaped<TermValue>,
    memory: &mut Memory<'_, S>,
) -> Shaped<TermValue> {
    let result = match result {
        Err(
            error @ (Error::Storage(_) | Error::TermRelease { .. } | Error::JsonRelease { .. }),
        ) => {
            let cause = cleanup_cause(&error);
            parts.finished = Some(Err(error));
            return Err(retained_release(ReleaseRetained::Parts(parts), cause));
        }
        result => result,
    };
    if let Err(cause) = release_parts_contents(&mut parts, memory) {
        parts.finished = Some(result);
        return Err(retained_release(ReleaseRetained::Parts(parts), cause));
    }
    drop(parts);
    retain_finished_result(result, memory.release_bytes(size_of::<TripleParts>()))
}

fn retain_finished_result(
    result: Shaped<TermValue>,
    refund: Result<(), StorageError>,
) -> Shaped<TermValue> {
    match refund {
        Ok(()) => result,
        Err(cause) => {
            let mut owner = JsonRelease::default();
            let cause = match result {
                Ok(term) => {
                    owner.term = Some(term);
                    cause
                }
                Err(error) => {
                    let (retained, original_physical) = owner.with_prior(error);
                    owner = retained;
                    original_physical.unwrap_or(cause)
                }
            };
            Err(Error::JsonRelease { owner, cause })
        }
    }
}

fn release_binding_value<S: Admission + ?Sized>(
    value: BindingValue,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    match value {
        BindingValue::String(text) => text.release(memory).map_err(Error::Storage),
        BindingValue::Triple(parts) => release_parts(parts, memory),
        BindingValue::Finished(Ok(term)) => release_term(term, memory),
        BindingValue::Finished(Err(error)) => release_error(error, memory),
        BindingValue::Other => Ok(()),
    }
}

fn release_fields<S: Admission + ?Sized>(
    fields: &mut BindingFields,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    for slot in [
        &mut fields.kind,
        &mut fields.language,
        &mut fields.its_dir,
        &mut fields.dir,
        &mut fields.datatype,
    ] {
        if let Some(Member::String(text)) = slot.take() {
            text.release(memory)?;
        }
    }
    if let Some(value) = fields.value.take()
        && let Err(error) = release_binding_value(value, memory)
    {
        let cause = cleanup_cause(&error);
        fields.value = Some(BindingValue::Finished(Err(error)));
        return Err(cause);
    }
    release_shaped(&mut fields.finished, memory)
}

fn release_binding<S: Admission + ?Sized>(
    mut fields: Box<BindingFields>,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    if let Err(cause) = release_fields(&mut fields, memory) {
        return Err(retained_release(ReleaseRetained::Binding(fields), cause));
    }
    drop(fields);
    memory
        .release_bytes(size_of::<BindingFields>())
        .map_err(Error::Storage)
}

fn release_finished_fields<S: Admission + ?Sized>(
    mut fields: Box<BindingFields>,
    result: Shaped<TermValue>,
    memory: &mut Memory<'_, S>,
) -> Result<Shaped<TermValue>, Error> {
    let result = match result {
        Err(
            error @ (Error::Storage(_) | Error::TermRelease { .. } | Error::JsonRelease { .. }),
        ) => {
            let cause = cleanup_cause(&error);
            if fields.finished.is_none() {
                fields.finished = Some(Err(error));
            } else {
                // Only the kind-string refund can leave a completed result here;
                // its returned primitive cause owns no additional payload.
                assert!(
                    matches!(error, Error::Storage(_)),
                    "kind refund returns an inline cause"
                );
            }
            return Err(retained_release(ReleaseRetained::Binding(fields), cause));
        }
        result => result,
    };
    if let Err(cause) = release_fields(&mut fields, memory) {
        fields.finished = Some(result);
        return Err(retained_release(ReleaseRetained::Binding(fields), cause));
    }
    drop(fields);
    let refund = memory.release_bytes(size_of::<BindingFields>());
    match refund {
        Ok(()) => Ok(result),
        Err(cause) => Err(retain_finished_result(result, Err(cause))
            .expect_err("a refused refund retains its original result")),
    }
}

fn release_scratch<S: Admission + ?Sized>(
    mut scratch: RowScratch,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    while let Some(open) = scratch.open.pop() {
        let result = match open {
            Open::Binding(mut fields) => match release_fields(&mut fields, memory) {
                Err(cause) => Err((Some(Open::Binding(fields)), cause)),
                Ok(()) => {
                    drop(fields);
                    memory
                        .release_bytes(size_of::<BindingFields>())
                        .map_err(|cause| (None, cause))
                }
            },
            Open::Triple(mut parts) => match release_parts_contents(&mut parts, memory) {
                Err(cause) => Err((Some(Open::Triple(parts)), cause)),
                Ok(()) => {
                    drop(parts);
                    memory
                        .release_bytes(size_of::<TripleParts>())
                        .map_err(|cause| (None, cause))
                }
            },
        };
        if let Err((retained, cause)) = result {
            if let Some(open) = retained {
                // Pop reserved this original slot; restoring it cannot allocate.
                scratch.open.push(open);
            }
            return Err(retained_release(ReleaseRetained::Scratch(scratch), cause));
        }
    }
    let open = std::mem::take(&mut scratch.open);
    if let Err(cause) = memory.release_vec(open) {
        return Err(retained_release(ReleaseRetained::Scratch(scratch), cause));
    }
    memory.release_vec(scratch.bound).map_err(Error::Storage)
}

/// Report a native shape; it has no heap payload to admit or render.
fn fmt_with_memory<S: Admission + ?Sized>(shape: ReadShape, _memory: &mut Memory<'_, S>) -> Error {
    ReadRefusal::shape(crate::SparqlResultsFormat::Json, shape)
}

fn syntax_with_memory<S: Admission + ?Sized>(
    error: json::Error,
    _memory: &mut Memory<'_, S>,
) -> Error {
    match error.kind() {
        json::ErrorKind::Storage(storage)
        | json::ErrorKind::Escape(purrdf_lex::json_escape::JsonEscapeErrorKind::Storage(storage)) => {
            Error::Storage(storage)
        }
        _ => Error::Json(error),
    }
}

fn syntax(error: json::Error) -> Error {
    syntax_with_memory(error, &mut Memory::new(&mut Resident))
}

/// The reader's bounds: no container-depth cap. Every walk over the document —
/// the reader's own, the tree's drop, [`binding`] — runs over a heap stack, so
/// nesting costs memory proportional to the input and never machine stack, and
/// a triple term may nest as deep as the endpoint wrote it.
const LIMITS: Limits = Limits::with_depth(usize::MAX);

/// The whole document as a JSON tree.
fn read_document(bytes: &[u8]) -> Result<Value, Error> {
    json::read_slice(bytes, LIMITS).map_err(syntax)
}

/// Read the next value, requiring it to open an object; `message` otherwise.
fn open_object<S: Admission + ?Sized>(
    reader: &mut Reader<'_>,
    message: ReadShape,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    match reader
        .next_event_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?
    {
        Event::BeginObject { .. } => Ok(()),
        _ => Err(fmt_with_memory(message, memory)),
    }
}

/// Read the next value, requiring it to open an array; `message` otherwise.
fn open_array<S: Admission + ?Sized>(
    reader: &mut Reader<'_>,
    message: ReadShape,
    memory: &mut Memory<'_, S>,
) -> Result<(), Error> {
    match reader
        .next_event_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?
    {
        Event::BeginArray { .. } => Ok(()),
        _ => Err(fmt_with_memory(message, memory)),
    }
}

/// The next member name of the open object, decoded, or `None` at its `}`.
fn next_key<S: Admission + ?Sized>(
    reader: &mut Reader<'_>,
    memory: &mut Memory<'_, S>,
) -> Result<Option<&'static str>, Error> {
    let Some(raw) = reader
        .next_key_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?
    else {
        return Ok(None);
    };
    let key = raw
        .decode_with_memory(memory)
        .map_err(|error| syntax_with_memory(error, memory))?;
    let identity = match key.as_ref() {
        "boolean" => "boolean",
        "head" => "head",
        "vars" => "vars",
        "results" => "results",
        "bindings" => "bindings",
        "type" => "type",
        "xml:lang" => "xml:lang",
        "its:dir" => "its:dir",
        "dir" => "dir",
        "datatype" => "datatype",
        "value" => "value",
        "subject" => "subject",
        "predicate" => "predicate",
        "object" => "object",
        _ => "",
    };
    release_text(key, memory)?;
    Ok(Some(identity))
}

/// Decode the first `vars` array in a `head` object, skipping other fields.
fn head_variables<S: Admission + ?Sized>(
    reader: &mut Reader<'_>,
    memory: &mut Memory<'_, S>,
) -> Result<Vec<String>, Error> {
    open_object(reader, ReadShape::Head, memory)?;
    let mut variables = None;
    let result = (|| {
        while let Some(key) = next_key(reader, memory)? {
            if key == "vars" && variables.is_none() && reader.peek_kind() == Some(Kind::Array) {
                variables = Some(string_array(reader, ReadShape::Variable, memory)?);
            } else {
                reader
                    .skip_value_with_memory(memory)
                    .map_err(|error| syntax_with_memory(error, memory))?;
            }
        }
        Ok(variables.take().unwrap_or_default())
    })();
    let mut pending = JsonRelease::default();
    pending.strings = variables;
    finish_plain(
        pending,
        result,
        |strings, pending| pending.strings = Some(strings),
        memory,
    )
}

/// Read a string-only JSON array.
fn string_array<S: Admission + ?Sized>(
    reader: &mut Reader<'_>,
    item_error: ReadShape,
    memory: &mut Memory<'_, S>,
) -> Result<Vec<String>, Error> {
    open_array(reader, ReadShape::Array, memory)?;
    let mut items = Vec::new();
    let mut text = None;
    let result = (|| {
        while reader
            .next_item()
            .map_err(|error| syntax_with_memory(error, memory))?
        {
            match reader
                .next_event_with_memory(memory)
                .map_err(|error| syntax_with_memory(error, memory))?
            {
                Event::String(item) => {
                    let decoded = item
                        .decode_with_memory(memory)
                        .map_err(|error| syntax_with_memory(error, memory))?;
                    text = Some(owned_text(decoded, memory)?);
                    memory
                        .reserve_for_push(&mut items)
                        .map_err(Error::Storage)?;
                    items.push(text.take().expect("the decoded original string exists"));
                }
                _ => return Err(fmt_with_memory(item_error, memory)),
            }
        }
        Ok(std::mem::take(&mut items))
    })();
    let mut pending = JsonRelease::new(ReleaseRetained::Strings(items));
    pending.text = text;
    finish_plain(
        pending,
        result,
        |strings, pending| pending.strings = Some(strings),
        memory,
    )
}

/// Read the first `bindings` array in a `results` object through a bounded sink.
fn bounded_results<S: Admission + ?Sized>(
    reader: &mut Reader<'_>,
    variables: &[String],
    row_limit: Option<usize>,
    memory: &mut Memory<'_, S>,
) -> BoundedRowsResult {
    open_object(reader, ReadShape::Results, memory)?;
    let mut found = None;
    let result = (|| {
        while let Some(key) = next_key(reader, memory)? {
            if key == "bindings" && found.is_none() {
                found = Some(bounded_binding_array(reader, variables, row_limit, memory)?);
            } else {
                reader
                    .skip_value_with_memory(memory)
                    .map_err(|error| syntax_with_memory(error, memory))?;
            }
        }
        found
            .take()
            .ok_or_else(|| fmt_with_memory(ReadShape::Bindings, memory))
    })();
    let mut pending = JsonRelease::default();
    if let Some((rows, _)) = found {
        pending.rows = Some(rows);
    }
    finish_plain(
        pending,
        result,
        |(rows, _), pending| pending.rows = Some(rows),
        memory,
    )
}

/// Decode the prefix of a bindings array that fits `row_limit`, then syntax-scan the
/// suffix without constructing its tree.
fn bounded_binding_array<S: Admission + ?Sized>(
    reader: &mut Reader<'_>,
    variables: &[String],
    row_limit: Option<usize>,
    memory: &mut Memory<'_, S>,
) -> BoundedRowsResult {
    open_array(reader, ReadShape::Bindings, memory)?;
    let mut rows = Vec::new();
    let mut current = None;
    let mut truncated = false;
    let mut scratch = RowScratch::default();
    let result = (|| {
        while reader
            .next_item()
            .map_err(|error| syntax_with_memory(error, memory))?
        {
            if row_limit.is_none_or(|limit| rows.len() < limit) {
                current = Some(binding_row(reader, variables, &mut scratch, memory)??);
                memory.reserve_for_push(&mut rows).map_err(Error::Storage)?;
                rows.push(current.take().expect("the decoded original row exists"));
            } else {
                truncated = true;
                reader
                    .skip_value_with_memory(memory)
                    .map_err(|error| syntax_with_memory(error, memory))?;
            }
        }
        Ok((std::mem::take(&mut rows), truncated))
    })();
    let mut pending = JsonRelease::new(ReleaseRetained::Rows(rows));
    pending.row = current;
    pending.scratch = Some(scratch);
    finish_plain(
        pending,
        result,
        |(rows, _), pending| pending.rows = Some(rows),
        memory,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::to_json;
    use purrdf_core::SparqlResult;
    use purrdf_core::TermBox;
    use purrdf_core::vocab::rdf::{
        DIR_LANG_STRING as RDF_DIR_LANGSTRING, LANG_STRING as RDF_LANGSTRING,
    };

    /// A `SELECT` document binding `?x` to a literal whose `value` member is
    /// the JSON string `string` (quotes included, escapes as written).
    fn literal_document(string: &[u8]) -> Vec<u8> {
        let mut document =
            br#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":{"type":"literal","value":"#
                .to_vec();
        document.extend_from_slice(string);
        document.extend_from_slice(b"}}]}}");
        document
    }

    /// The lexical form `from_json` reads for [`literal_document`].
    fn literal_value(string: &[u8]) -> Result<String, Error> {
        let parsed = from_json(&literal_document(string))?;
        let Some(TermValue::Literal { lexical_form, .. }) = parsed.rows[0][0].clone() else {
            panic!("expected a literal");
        };
        Ok(lexical_form)
    }

    /// A raw C0 control inside a JSON string is refused (RFC 8259 §7), both in a
    /// value that is decoded and in one that is only skipped.
    #[test]
    fn a_raw_control_character_in_a_string_is_refused() {
        for raw in [0x01_u8, b'\t', b'\n', 0x1F] {
            let mut string = b"\"a".to_vec();
            string.push(raw);
            string.extend_from_slice(b"b\"");
            let error = literal_value(&string).expect_err("a raw control is refused");
            assert!(
                matches!(&error, Error::Json(cause)
                    if cause.kind() == json::ErrorKind::RawControl),
                "{raw:#04X}: {error:?}"
            );
            assert!(
                from_json_bounded(&literal_document(&string), u64::MAX).is_err(),
                "{raw:#04X}"
            );
            // And in a member the reader only skips.
            let mut skipped = br#"{"head":{"vars":[],"note":"a"#.to_vec();
            skipped.push(raw);
            skipped.extend_from_slice(br#"b"},"boolean":true}"#);
            assert!(from_json_boolean(&skipped).is_err(), "{raw:#04X}");
            assert!(from_json_bounded(&skipped, u64::MAX).is_err(), "{raw:#04X}");
        }
    }

    /// The valid neighbours: the same scalars written as escapes parse, and DEL
    /// (not a C0 control) is lawful raw.
    #[test]
    fn an_escaped_control_character_still_parses() {
        assert_eq!(
            literal_value(br#""a\u0001b""#).expect("escaped U+0001"),
            "a\u{1}b"
        );
        assert_eq!(
            literal_value(br#""a\tb\nc\u001f""#).expect("escaped controls"),
            "a\tb\nc\u{1f}"
        );
        assert_eq!(literal_value(b"\"a\x7fb\"").expect("raw DEL"), "a\u{7f}b");
        let skipped = br#"{"head":{"vars":[],"note":"a\u0001b"},"boolean":true}"#;
        assert!(from_json_boolean(skipped).expect("an escaped control in a skipped member"));
    }

    /// Strings decode across escapes, raw multibyte UTF-8 and surrogate pairs,
    /// in the tree reader and the bounded reader alike.
    #[test]
    fn strings_decode_escapes_and_multibyte_text() {
        let cases: [(&[u8], &str); 8] = [
            (b"\"\"", ""),
            (b"\"plain ascii\"", "plain ascii"),
            (b"\"a\\\"b\"", "a\"b"),
            (
                b"\"tab\\tnl\\ncr\\r\\\\ \\/ \\b\\f\"",
                "tab\tnl\ncr\r\\ / \u{8}\u{c}",
            ),
            (b"\"\\u00e9\\ud83d\\udc31\"", "\u{e9}\u{1f431}"),
            (
                "\"caf\u{e9} \u{4e2d}\u{6587} \u{1f431}\"".as_bytes(),
                "caf\u{e9} \u{4e2d}\u{6587} \u{1f431}",
            ),
            (
                "\"ascii \u{e9} \\\" more \\u0041 \u{1f431}\\n tail\"".as_bytes(),
                "ascii \u{e9} \" more A \u{1f431}\n tail",
            ),
            (b"\"raw\\u0001control\"", "raw\u{1}control"),
        ];
        for (string, expected) in cases {
            assert_eq!(
                literal_value(string).expect("parses"),
                expected,
                "{string:?}"
            );
            let bounded = from_json_bounded(&literal_document(string), u64::MAX).expect("parses");
            assert_eq!(
                bounded.solutions,
                from_json(&literal_document(string)).expect("parses"),
                "{string:?}"
            );
        }
    }

    /// A malformed string is refused: unterminated, a bad escape, a lone
    /// surrogate, invalid UTF-8.
    #[test]
    fn malformed_strings_are_refused() {
        for string in [
            &b"\"no terminator"[..],
            b"\"ascii then bad escape \\q\"",
            b"\"ascii then \\",
            b"\"lone high \\ud83d\"",
            b"\"ascii then \xff\"",
            b"\"ascii then truncated \xe4\xb8\"",
        ] {
            assert!(literal_value(string).is_err(), "{string:?}");
            assert!(
                from_json_bounded(&literal_document(string), u64::MAX).is_err(),
                "{string:?}"
            );
        }
    }

    /// RFC 8259 §6: a number is `[-] int [frac] [exp]`. A run of number
    /// characters outside that grammar is refused wherever it stands, in a
    /// skipped member included.
    #[test]
    fn a_number_outside_the_grammar_is_refused() {
        let document = br#"{"head":{"vars":[]},"extra":1-2+e,"boolean":true}"#;
        assert!(from_json_boolean(document).is_err());
        let select = br#"{"head":{"vars":[]},"extra":1-2+e,"results":{"bindings":[]}}"#;
        assert!(from_json(select).is_err());
        assert!(from_json_bounded(select, u64::MAX).is_err());
    }

    /// The valid neighbour: a signed fraction with a signed exponent is a number.
    #[test]
    fn a_number_inside_the_grammar_is_accepted() {
        let document = br#"{"head":{"vars":[]},"extra":-1.2e+3,"boolean":true}"#;
        assert!(from_json_boolean(document).expect("a lawful number"));
        let select = br#"{"head":{"vars":[]},"extra":-1.2e+3,"results":{"bindings":[]}}"#;
        let empty = ParsedSolutions {
            variables: Vec::new(),
            rows: Vec::new(),
        };
        assert_eq!(from_json(select), Ok(empty.clone()));
        assert_eq!(
            from_json_bounded(select, u64::MAX).map(|bounded| bounded.solutions),
            Ok(empty)
        );
    }

    /// Provenance round-trip: what [`crate::json::to_json`] writes under a namespace,
    /// [`provenance_from_json`] reads back — the writer no longer emits
    /// something nothing can decode.
    #[test]
    fn provenance_round_trips_through_json() {
        let result = SparqlResult::Boolean(true);
        let provenance = ResultProvenance {
            query_hash: Some("deadbeef".to_owned()),
            engine: Some("purrdf-sparql-eval".to_owned()),
            solutions: vec![
                SolutionProvenance {
                    sources: vec!["http://example.org/g1".to_owned()],
                },
                SolutionProvenance { sources: vec![] },
            ],
        };
        let namespace = ProvenanceNamespace::new("prov", "http://example.org/ns/prov#")
            .expect("valid namespace");
        let outcome = to_json(&result, &provenance, Some(&namespace)).expect("serializes");

        let decoded =
            provenance_from_json(&outcome.bytes, &namespace).expect("provenance decodes back");
        assert_eq!(decoded, provenance);
    }

    /// A document with no member under the caller's namespace prefix — because
    /// the writer never populated one, or a different namespace was used —
    /// decodes to the empty provenance rather than erroring.
    #[test]
    fn absent_provenance_member_decodes_to_default() {
        let namespace = ProvenanceNamespace::new("prov", "http://example.org/ns/prov#")
            .expect("valid namespace");
        let doc = br#"{"head":{},"boolean":true}"#;
        let decoded = provenance_from_json(doc, &namespace).expect("decodes");
        assert!(decoded.is_empty());
    }

    /// Namespace-by-IRI pin (false NEGATIVE, mirroring
    /// [`crate::xml_read::tests::provenance_reads_correctly_under_an_alternate_prefix_for_the_same_namespace`]):
    /// a document that records the caller's OWN namespace IRI under a top-level
    /// key spelled with a DIFFERENT prefix than this crate's own writer happens
    /// to use (`p` instead of `prov`) must still decode — JSON provenance
    /// identity is IRI-based (the `"namespace"` field), not top-level-key-based.
    #[test]
    fn provenance_reads_correctly_under_an_alternate_prefix_for_the_same_namespace() {
        let namespace = ProvenanceNamespace::new("prov", "https://example.org/ns/prov#")
            .expect("valid namespace");
        let doc = br#"{"head":{},"boolean":true,"p":{
            "namespace":"https://example.org/ns/prov#",
            "queryForm":"ask",
            "queryHash":"deadbeef",
            "engine":"purrdf-sparql-eval"
        }}"#;
        let decoded = provenance_from_json(doc, &namespace).expect("decodes");
        assert_eq!(
            decoded,
            ResultProvenance {
                query_hash: Some("deadbeef".to_owned()),
                engine: Some("purrdf-sparql-eval".to_owned()),
                solutions: Vec::new(),
            }
        );
    }

    /// Namespace-by-IRI pin (false POSITIVE, mirroring
    /// [`crate::xml_read::tests::foreign_namespace_under_the_writers_own_prefix_is_not_read_as_provenance`]):
    /// a document that reuses this crate's writer's OWN prefix spelling
    /// (`prov`) as its top-level key, but records an UNRELATED namespace IRI —
    /// here the actual W3C PROV namespace (`http://www.w3.org/ns/prov#`), the
    /// single most common real-world `prov:` binding — must NOT be read as the
    /// caller's provenance extension just because the top-level key spelling
    /// matches; only the recorded `"namespace"` IRI identifies it. This is the
    /// direction the old, tautological `provenance_under_a_different_namespace_is_not_found`
    /// test (which varied prefix AND iri together) could never actually observe.
    #[test]
    fn foreign_namespace_under_the_writers_own_prefix_is_not_read_as_provenance() {
        let namespace = ProvenanceNamespace::new("prov", "https://example.org/ns/prov#")
            .expect("valid namespace");
        let doc = br#"{"head":{},"boolean":true,"prov":{
            "namespace":"http://www.w3.org/ns/prov#",
            "queryForm":"ask",
            "queryHash":"deadbeef"
        }}"#;
        let decoded = provenance_from_json(doc, &namespace).expect("decodes without error");
        assert!(
            decoded.is_empty(),
            "a `prov`-keyed member recording the W3C PROV namespace must not be mistaken \
             for this caller's own `prov` namespace extension"
        );
    }

    #[test]
    fn reads_select_with_mixed_terms() {
        let srj = r#"{
          "head": { "vars": [ "s", "name", "label", "age" ] },
          "results": { "bindings": [
            {
              "s": { "type": "uri", "value": "http://example.org/s" },
              "name": { "type": "literal", "value": "Ada" },
              "label": { "type": "literal", "value": "bonjour", "xml:lang": "fr" },
              "age": { "type": "literal", "value": "42",
                       "datatype": "http://www.w3.org/2001/XMLSchema#integer" }
            },
            {
              "s": { "type": "bnode", "value": "b0" }
            }
          ] }
        }"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        assert_eq!(parsed.variables, vec!["s", "name", "label", "age"]);
        assert_eq!(parsed.rows.len(), 2);
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Iri("http://example.org/s".to_owned()))
        );
        assert_eq!(
            parsed.rows[0][1],
            Some(TermValue::Literal {
                lexical_form: "Ada".to_owned(),
                datatype: XSD_STRING.to_owned(),
                language: None,
                direction: None,
            })
        );
        assert_eq!(
            parsed.rows[0][2],
            Some(TermValue::Literal {
                lexical_form: "bonjour".to_owned(),
                datatype: RDF_LANGSTRING.to_owned(),
                language: Some("fr".to_owned()),
                direction: None,
            })
        );
        assert_eq!(
            parsed.rows[0][3],
            Some(TermValue::Literal {
                lexical_form: "42".to_owned(),
                datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                language: None,
                direction: None,
            })
        );
        // Second row: only `s` bound (a bnode), the rest unbound.
        assert_eq!(
            parsed.rows[1][0],
            Some(TermValue::Blank {
                label: "b0".to_owned(),
                scope: BlankScope::DEFAULT,
            })
        );
        assert_eq!(parsed.rows[1][1], None);
        assert_eq!(parsed.rows[1][3], None);
    }

    #[test]
    fn bounded_reader_stops_before_the_limit_plus_one_binding() {
        // `results` deliberately precedes `head`: the two-pass reader may not rely on the
        // conventional field order when deriving the two-column row ceiling.
        let srj = br#"{
          "results":{"bindings":[
            {"x":{"type":"uri","value":"http://example.org/0"}},
            {"x":{"type":"uri","value":"http://example.org/1"}},
            {"x":{"type":"uri","value":"http://example.org/2"}}
          ]},
          "head":{"vars":["x","y"]}
        }"#;

        let bounded = from_json_bounded(srj, 4).expect("two two-cell rows fit");
        assert_eq!(bounded.solutions.variables, ["x", "y"]);
        assert_eq!(bounded.solutions.rows.len(), 2);
        assert!(bounded.truncated, "the third binding is the overflow proof");

        let exact = from_json_bounded(srj, 6).expect("the exact boundary is inclusive");
        assert_eq!(exact.solutions.rows.len(), 3);
        assert!(!exact.truncated);
        assert_eq!(exact.solutions, from_json(srj).expect("ordinary decode"));
    }

    #[test]
    fn bounded_reader_does_not_invent_a_row_bound_for_zero_columns() {
        let srj = br#"{"head":{"vars":[]},"results":{"bindings":[{},{},{}]}}"#;
        let bounded = from_json_bounded(srj, 0).expect("unit rows consume zero cells");
        assert_eq!(bounded.solutions.rows.len(), 3);
        assert!(!bounded.truncated);
    }

    #[test]
    fn reads_directional_literal() {
        // `its:dir` is the SPARQL 1.2 Query Results spec spelling (see
        // `crate::json`'s module docs for the fixture evidence).
        let srj = r#"{"head":{"vars":["x"]},"results":{"bindings":[
          {"x":{"type":"literal","value":"שלום","xml:lang":"he","its:dir":"rtl"}}]}}"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Literal {
                lexical_form: "שלום".to_owned(),
                datatype: RDF_DIR_LANGSTRING.to_owned(),
                language: Some("he".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            })
        );
    }

    /// The bare `dir` spelling is tolerated for interop with producers that
    /// predate the SPARQL 1.2 `its:dir` spelling.
    #[test]
    fn tolerates_legacy_bare_dir_spelling() {
        let srj = r#"{"head":{"vars":["x"]},"results":{"bindings":[
          {"x":{"type":"literal","value":"hello","xml:lang":"en","dir":"ltr"}}]}}"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Literal {
                lexical_form: "hello".to_owned(),
                datatype: RDF_DIR_LANGSTRING.to_owned(),
                language: Some("en".to_owned()),
                direction: Some(RdfTextDirection::Ltr),
            })
        );
    }

    /// When both spellings are present, `its:dir` — the spec spelling — takes
    /// priority over the legacy bare `dir`.
    #[test]
    fn its_dir_takes_priority_over_bare_dir() {
        let srj = r#"{"head":{"vars":["x"]},"results":{"bindings":[
          {"x":{"type":"literal","value":"hello","xml:lang":"en","its:dir":"ltr","dir":"rtl"}}]}}"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Literal {
                lexical_form: "hello".to_owned(),
                datatype: RDF_DIR_LANGSTRING.to_owned(),
                language: Some("en".to_owned()),
                direction: Some(RdfTextDirection::Ltr),
            })
        );
    }

    #[test]
    fn reads_triple_term() {
        let srj = r#"{"head":{"vars":["t"]},"results":{"bindings":[
          {"t":{"type":"triple","value":{
            "subject":{"type":"uri","value":"http://ex/s"},
            "predicate":{"type":"uri","value":"http://ex/p"},
            "object":{"type":"uri","value":"http://ex/o"}}}}]}}"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Triple {
                s: TermBox::new(TermValue::Iri("http://ex/s".to_owned())),
                p: TermBox::new(TermValue::Iri("http://ex/p".to_owned())),
                o: TermBox::new(TermValue::Iri("http://ex/o".to_owned())),
            })
        );
    }

    #[test]
    fn reads_ask_boolean() {
        assert!(from_json_boolean(br#"{"head":{},"boolean":true}"#).expect("ask"));
        assert!(!from_json_boolean(br#"{"head":{},"boolean":false}"#).expect("ask"));
    }

    #[test]
    fn select_reader_rejects_ask_document() {
        let err = from_json(br#"{"head":{},"boolean":true}"#).unwrap_err();
        assert!(matches!(err, Error::Format(_)));
    }

    #[test]
    fn handles_escapes_and_unicode() {
        let srj = r#"{"head":{"vars":["x"]},"results":{"bindings":[
          {"x":{"type":"literal","value":"a\"b\\c\nA😀"}}]}}"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        let TermValue::Literal { lexical_form, .. } = parsed.rows[0][0].clone().unwrap() else {
            panic!("expected literal");
        };
        assert_eq!(lexical_form, "a\"b\\c\nA😀");
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(from_json(br#"{"head":{"vars":[]},"results":{"bindings":[]}} oops"#).is_err());
    }

    /// Guard against the O(N²) regression: a long multibyte string must parse
    /// correctly and the decoded value must round-trip.
    #[test]
    fn long_multibyte_string_parses_correctly() {
        // Build a large string of multibyte chars: mix of 2-byte (é, U+00E9)
        // and 3-byte (你, U+4F60) code points so all width branches are hit.
        let repeated_2byte = "é".repeat(1_500); // 3 000 bytes
        let repeated_3byte = "你".repeat(1_000); // 3 000 bytes
        let long_value = format!("{repeated_2byte}{repeated_3byte}");

        let srj = format!(
            r#"{{"head":{{"vars":["x"]}},"results":{{"bindings":[{{"x":{{"type":"literal","value":"{long_value}"}}}}]}}}}"#
        );
        let parsed = from_json(srj.as_bytes()).expect("parse long multibyte string");
        let TermValue::Literal { lexical_form, .. } = parsed.rows[0][0].clone().unwrap() else {
            panic!("expected literal");
        };
        assert_eq!(lexical_form, long_value, "decoded value must round-trip");
    }

    /// A 4-byte UTF-8 sequence (emoji, U+1F600) must decode correctly through
    /// the lead-byte-width path.
    #[test]
    fn four_byte_utf8_sequence_parses() {
        // U+1F600 GRINNING FACE encodes as 4 UTF-8 bytes.
        let val = "😀".repeat(500);
        let srj = format!(
            r#"{{"head":{{"vars":["x"]}},"results":{{"bindings":[{{"x":{{"type":"literal","value":"{val}"}}}}]}}}}"#
        );
        let parsed = from_json(srj.as_bytes()).expect("parse 4-byte sequences");
        let TermValue::Literal { lexical_form, .. } = parsed.rows[0][0].clone().unwrap() else {
            panic!("expected literal");
        };
        assert_eq!(lexical_form, val);
    }

    /// Malformed UTF-8 bytes inside a JSON string must yield a parse Error, not
    /// a panic.  We inject a raw invalid continuation byte (0x80) that is not
    /// preceded by a valid lead byte.
    #[test]
    fn malformed_utf8_yields_error_not_panic() {
        // Construct bytes: valid JSON prefix, then a bare 0x80 continuation byte
        // (invalid as a lead byte), then closing JSON.
        let prefix =
            br#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":{"type":"literal","value":""#;
        let suffix = br#""}}]}}"#;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(prefix);
        // Insert the invalid lead byte just before the closing quote.
        bytes.push(0x80); // bare continuation — not a valid UTF-8 lead
        bytes.extend_from_slice(suffix);
        let result = from_json(&bytes);
        assert!(result.is_err(), "expected Err for invalid UTF-8, got Ok");
        assert!(
            matches!(result.unwrap_err(), Error::Json(_)),
            "error must retain the native JSON refusal"
        );
    }

    /// Tags the `LANGTAG` grammar refuses, and the neighbouring ones it must
    /// still take. Shared with the XML reader's twin and with the
    /// `STRLANG`/`STRLANGDIR` gate in `purrdf-sparql-eval` — one accept set,
    /// whichever door a tag arrives through.
    const REFUSED_TAGS: &[&str] = &["en us", "1", "9-9", "123-456", "en-", "-", "!!!"];
    const ACCEPTED_TAGS: &[&str] = &[
        "en",
        "en-US",
        "zh-Hans-CN",
        "de-CH-x-phonebk",
        "i-enochian",
        "x-purrdf-afrikaans",
        "x-gmeow-english",
        "en-fr-jura",
        "fr-be-fbcl",
    ];

    fn srj_with_lang(tag: &str) -> Vec<u8> {
        format!(
            "{{\"head\":{{\"vars\":[\"l\"]}},\"results\":{{\"bindings\":\
             [{{\"l\":{{\"type\":\"literal\",\"value\":\"x\",\"xml:lang\":\"{tag}\"}}}}]}}}}"
        )
        .into_bytes()
    }

    /// A results DOCUMENT is parsed input. A federated `SERVICE` response is
    /// how a hostile or sloppy endpoint's tag gets in, and before this gate it
    /// was copied verbatim into a `TermValue` and written straight back out.
    #[test]
    fn a_refused_language_tag_in_a_document_is_refused_with_its_diagnostic_code() {
        for tag in REFUSED_TAGS {
            let error = from_json(&srj_with_lang(tag))
                .expect_err("a tag the grammar refuses must not decode");
            let Error::LanguageTag {
                language, cause, ..
            } = &error
            else {
                panic!("expected native language refusal for {tag:?}, got {error:?}");
            };
            let fields = purrdf_testkit::error_capture::Capture::read(&error);
            assert!(
                matches!(fields.field("language"), purrdf_hash::error::ErrorValue::Text(value) if value == *tag)
            );
            assert_eq!(fields.child("cause").code, cause.diagnostic_code());
            assert_eq!(language, tag);
            let presentation = error.to_string();
            let message = presentation.strip_prefix("result format error: ").unwrap();
            assert!(
                message.starts_with("SPARQL-JSON: invalid language tag "),
                "the refusal must name the format and the tag: {message}"
            );
            assert!(
                message.contains(&format!("`{tag}`")),
                "the refusal must quote the offending tag verbatim, since this \
                 crate's error carries no position: {message}"
            );
            assert!(
                message.contains("(langtag-"),
                "the refusal must surface the grammar's own diagnostic code, not \
                 collapse to a generic sentence: {message}"
            );
        }
    }

    /// The over-refusal half. `i-enochian`, the two `x-` private-use tags and
    /// the two terminal-only shapes are exactly what a profile chosen one notch
    /// too strict would start silently rejecting on ingress.
    #[test]
    fn every_well_formed_language_tag_still_reads_back() {
        for tag in ACCEPTED_TAGS {
            let parsed = from_json(&srj_with_lang(tag))
                .unwrap_or_else(|e| panic!("{tag:?} must still parse: {e}"));
            assert_eq!(
                parsed.rows[0][0],
                Some(TermValue::Literal {
                    lexical_form: "x".to_owned(),
                    datatype: RDF_LANGSTRING.to_owned(),
                    language: Some((*tag).to_owned()),
                    direction: None,
                }),
                "the gate must not alter the tag it lets through ({tag:?})"
            );
        }
    }

    /// The nesting depth every deep-input test uses.
    const DEEP: usize = 1_000_000;

    /// The machine stack every deep-input test runs on, 128 KiB: a walk that
    /// recursed once per nesting level would overflow it after a few hundred levels.
    const SMALL_STACK: usize = 128 * 1024;

    /// `depth` nested JSON arrays around nothing: `[[…[]…]]`.
    fn nested_arrays(depth: usize) -> String {
        let mut text = "[".repeat(depth);
        text.push_str(&"]".repeat(depth));
        text
    }

    fn select_with_binding(cell: &str) -> Vec<u8> {
        format!(r#"{{"head":{{"vars":["x"]}},"results":{{"bindings":[{{"x":{cell}}}]}}}}"#)
            .into_bytes()
    }

    fn format_error(message: &str) -> String {
        format!("result format error: SPARQL-JSON: {message}")
    }

    #[test]
    fn a_deeply_nested_value_in_a_binding_position_is_refused_without_overflow() {
        let (full, bounded, truncated) = purrdf_stack::on_stack(SMALL_STACK, || {
            let doc = select_with_binding(&nested_arrays(DEEP));
            // The same nesting with its innermost array left unclosed.
            let unclosed = select_with_binding(&"[".repeat(DEEP));
            (
                from_json(&doc),
                from_json_bounded(&doc, u64::MAX).map(|bounded| bounded.solutions),
                from_json(&unclosed),
            )
        })
        .expect("spawn a small-stack thread");
        let not_an_object = format_error("binding is not an object");
        assert_eq!(
            full.map_err(|error| error.to_string()),
            Err(not_an_object.clone())
        );
        assert_eq!(
            bounded.map_err(|error| error.to_string()),
            Err(not_an_object)
        );
        assert_eq!(
            truncated.map_err(|error| error.to_string()),
            Err(format_error(&format!(
                "JSON byte {}: expected a JSON value",
                r#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":"#.len() + DEEP
            )))
        );
    }

    #[test]
    fn a_deeply_nested_ignored_member_parses_without_overflow() {
        let (full, bounded, skipped) = purrdf_stack::on_stack(SMALL_STACK, || {
            let deep = nested_arrays(DEEP);
            let doc = format!(
                r#"{{"head":{{"vars":["x"],"link":{deep}}},"results":{{"bindings":[{{"x":{{"type":"uri","value":"http://example.org/a"}},"y":{deep}}}]}},"extra":{deep}}}"#
            )
            .into_bytes();
            // A second binding past a one-cell ceiling is skipped, not decoded.
            let over_limit = format!(
                r#"{{"head":{{"vars":["x"]}},"results":{{"bindings":[{{"x":{{"type":"uri","value":"http://example.org/a"}}}},{deep}]}}}}"#
            )
            .into_bytes();
            (
                from_json(&doc),
                from_json_bounded(&doc, u64::MAX),
                from_json_bounded(&over_limit, 1),
            )
        })
        .expect("spawn a small-stack thread");
        let expected = ParsedSolutions {
            variables: vec!["x".to_owned()],
            rows: vec![vec![Some(TermValue::Iri(
                "http://example.org/a".to_owned(),
            ))]],
        };
        assert_eq!(full.as_ref(), Ok(&expected));
        assert_eq!(
            bounded,
            Ok(BoundedParsedSolutions {
                solutions: expected.clone(),
                truncated: false,
            })
        );
        assert_eq!(
            skipped,
            Ok(BoundedParsedSolutions {
                solutions: expected,
                truncated: true,
            })
        );
    }

    /// A triple term nesting `depth` triple terms in object position, around an IRI.
    fn nested_triple_binding(depth: usize) -> String {
        let open = r#"{"type":"triple","value":{"subject":{"type":"bnode","value":"b"},"predicate":{"type":"uri","value":"http://example.org/p"},"object":"#;
        let mut text = open.repeat(depth);
        text.push_str(r#"{"type":"uri","value":"http://example.org/o"}"#);
        text.push_str(&"}}".repeat(depth));
        text
    }

    /// The triple-term nesting depth of `term`, counted along object positions.
    fn object_chain_depth(mut term: &TermValue) -> usize {
        let mut depth = 0;
        while let TermValue::Triple { s, p, o } = term {
            assert!(matches!(&**s, TermValue::Blank { label, .. } if label == "b"));
            assert_eq!(&**p, &TermValue::Iri("http://example.org/p".to_owned()));
            depth += 1;
            term = o;
        }
        assert_eq!(term, &TermValue::Iri("http://example.org/o".to_owned()));
        depth
    }

    #[test]
    fn a_deeply_nested_triple_term_binding_decodes_without_overflow() {
        let (full, bounded) = purrdf_stack::on_stack(SMALL_STACK, || {
            let doc = select_with_binding(&nested_triple_binding(DEEP));
            let full = from_json(&doc).map(|parsed| {
                let [row] = parsed.rows.as_slice() else {
                    panic!("one row");
                };
                object_chain_depth(row[0].as_ref().expect("bound"))
            });
            let bounded = from_json_bounded(&doc, u64::MAX).map(|bounded| {
                object_chain_depth(bounded.solutions.rows[0][0].as_ref().expect("bound"))
            });
            (full, bounded)
        })
        .expect("spawn a small-stack thread");
        assert_eq!(full, Ok(DEEP));
        assert_eq!(bounded, Ok(DEEP));
    }

    #[test]
    fn a_shallow_nested_triple_term_decodes_to_the_same_term() {
        let parsed = from_json(&select_with_binding(&nested_triple_binding(2))).expect("parse");
        let triple = |o: TermValue| TermValue::Triple {
            s: TermBox::new(TermValue::Blank {
                label: "b".to_owned(),
                scope: BlankScope::DEFAULT,
            }),
            p: TermBox::new(TermValue::Iri("http://example.org/p".to_owned())),
            o: TermBox::new(o),
        };
        let expected = triple(triple(TermValue::Iri("http://example.org/o".to_owned())));
        assert_eq!(parsed.rows, vec![vec![Some(expected)]]);
    }

    #[test]
    fn a_nested_triple_term_reports_its_first_error_in_component_order() {
        // The predicate is not an IRI, and the object is malformed: the object is
        // decoded before the predicate is checked, so the object's error is reported.
        let both = r#"{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"literal","value":"p"},"object":{"type":"nonsense"}}}"#;
        assert_eq!(
            from_json(&select_with_binding(both)).map_err(|error| error.to_string()),
            Err(format_error("unknown binding type `nonsense`"))
        );
        let predicate_only = r#"{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"literal","value":"p"},"object":{"type":"uri","value":"http://example.org/o"}}}"#;
        assert_eq!(
            from_json(&select_with_binding(predicate_only)).map_err(|error| error.to_string()),
            Err(format_error("triple-term predicate is not an IRI"))
        );
        let no_object = r#"{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"uri","value":"http://example.org/p"}}}"#;
        assert_eq!(
            from_json(&select_with_binding(no_object)).map_err(|error| error.to_string()),
            Err(format_error("triple has no object"))
        );
    }

    #[test]
    fn malformed_nesting_reports_the_same_errors() {
        for (doc, message) in [
            (
                r#"{"a":[1,2}"#,
                "JSON byte 9: expected `,` or `]` in an array",
            ),
            (
                r#"{"a":{"b":1]}"#,
                "JSON byte 11: expected `,` or `}` in an object",
            ),
            (
                r#"{"a":{"b" 1}}"#,
                "JSON byte 10: expected `:` after a member name",
            ),
            (
                r#"{"a":[{ 7 }]}"#,
                "JSON byte 8: expected a `\"`-quoted member name",
            ),
            (r#"{"a":[1,]}"#, "JSON byte 8: expected a JSON value"),
            (
                r#"{"a":[[[]]]} x"#,
                "JSON byte 13: trailing data after the top-level value",
            ),
        ] {
            let full = from_json(doc.as_bytes()).unwrap_err();
            let bounded = from_json_bounded(doc.as_bytes(), u64::MAX).unwrap_err();
            assert_eq!(full, bounded, "syntax priority on {doc}");
            assert_eq!(full.to_string(), format_error(message));
            let Error::Json(cause) = full else {
                panic!("syntax refusal must retain its native fields");
            };
            let fields = purrdf_testkit::error_capture::Capture::read(&cause);
            assert!(
                matches!(fields.field("offset"), purrdf_hash::error::ErrorValue::ByteCount(value) if value == cause.offset() as u128)
            );
        }
    }
    #[test]
    fn native_select_keeps_exact_original_payload_grant_and_refuses_before_birth() {
        #[derive(Default)]
        struct Grant {
            live: usize,
            peak: usize,
            limit: usize,
            growths: usize,
            refuse_at: Option<usize>,
            refused: bool,
            later_growths: usize,
        }
        impl Admission for Grant {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if bytes > self.live {
                    self.growths += 1;
                    if self.refused {
                        self.later_growths += 1;
                    }
                    if self.refused || self.refuse_at == Some(self.growths) || bytes > self.limit {
                        self.refused = true;
                        return Err(StorageError::AdmissionFailed);
                    }
                }
                self.live = bytes;
                self.peak = self.peak.max(bytes);
                Ok(())
            }
        }
        // Results before head, escaped structural/variable keys, repeated
        // columns and nested RDF 1.2 terms exercise original live producers.
        let bytes = br#"{"results":{"bindings":[{"\u0078":{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"uri","value":"http://example.org/p"},"object":{"type":"literal","value":"a\nb","xml:lang":"en","its:dir":"ltr","dir":"r\u0074l"}}}}]},"head":{"vars":["\u0078","x"]}}"#;
        let mut refused = Grant::default();
        assert!(matches!(
            from_json_with_memory(bytes, None, &mut Memory::new(&mut refused)),
            Err(ReadError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(refused.live, 0);
        assert_eq!(refused.peak, 0);

        for ceiling in [None, Some(2)] {
            let mut grant = Grant {
                limit: 1 << 20,
                ..Grant::default()
            };
            let mut memory = Memory::new(&mut grant);
            let parsed = from_json_with_memory(bytes, ceiling, &mut memory).expect("native SELECT");
            let decoder_growths = memory.admission_mut().growths;
            assert!(!parsed.truncated);
            assert_eq!(parsed.solutions, from_json(bytes).expect("resident parity"));
            assert_eq!(parsed.solutions.rows[0][0], parsed.solutions.rows[0][1]);
            let mut expected = parsed.solutions.variables.capacity() * size_of::<String>()
                + parsed
                    .solutions
                    .variables
                    .iter()
                    .map(String::capacity)
                    .sum::<usize>()
                + parsed.solutions.rows.capacity() * size_of::<BindingRow>();
            for row in &parsed.solutions.rows {
                expected += row.capacity() * size_of::<Option<TermValue>>();
                for term in row.iter().flatten() {
                    let _ = term.visit_terms(|term| {
                        expected += match term {
                            TermValue::Iri(text) | TermValue::Blank { label: text, .. } => {
                                text.capacity()
                            }
                            TermValue::Literal {
                                lexical_form,
                                datatype,
                                language,
                                ..
                            } => {
                                lexical_form.capacity()
                                    + datatype.capacity()
                                    + language.as_ref().map_or(0, String::capacity)
                            }
                            TermValue::Triple { .. } => 3 * size_of::<TermValue>(),
                        };
                        std::ops::ControlFlow::<()>::Continue(())
                    });
                }
            }
            assert_eq!(
                memory.admitted_bytes(),
                expected,
                "only surviving output owns the original grant"
            );
            let mut solutions = parsed.solutions;
            while let Some(mut row) = solutions.rows.pop() {
                while let Some(term) = row.pop() {
                    if let Some(term) = term {
                        release_term(term, &mut memory).unwrap();
                    }
                }
                memory.release_vec(row).unwrap();
            }
            memory.release_vec(solutions.rows).unwrap();
            while let Some(variable) = solutions.variables.pop() {
                memory.release_string(variable).unwrap();
            }
            memory.release_vec(solutions.variables).unwrap();
            assert_eq!(memory.admitted_bytes(), 0);
            assert_eq!(grant.live, 0);
            assert!(grant.peak >= expected);

            // Every native growth cut exercises the actual staged Strings,
            // rows, duplicate readers, literal fields and partial child boxes.
            for cut in 1..=decoder_growths {
                let mut grant = Grant {
                    limit: 1 << 20,
                    ..Grant::default()
                };
                let mut memory = Memory::new(&mut grant);
                let parent = memory.string("healthy-original-parent").unwrap();
                let baseline = memory.admitted_bytes();
                memory.admission_mut().refuse_at = Some(cut + 1);
                let error = from_json_with_memory(bytes, ceiling, &mut memory)
                    .expect_err("the selected original growth refuses");
                assert_eq!(
                    memory.admission_mut().later_growths,
                    0,
                    "no allocation or census follows the first physical refusal"
                );
                assert_eq!(parent, "healthy-original-parent");
                memory.admission_mut().refused = false;
                memory.admission_mut().refuse_at = None;
                match error {
                    ReadError::Lexical(error) => release_error(error, &mut memory).unwrap(),
                    ReadError::Storage(cause) => assert_eq!(cause, StorageError::AdmissionFailed),
                }
                assert_eq!(
                    memory.admitted_bytes(),
                    baseline,
                    "all failed decoder owners release under their original account"
                );
                memory.release_string(parent).unwrap();
                assert_eq!(memory.admitted_bytes(), 0);
            }
        }
    }

    #[test]
    fn native_select_retains_syntax_priority_and_bounded_suffix_law() {
        let malformed = br#"{"head":{"vars":[false]},"results":{"bindings":[]},"extra":"\udc00"}"#;
        let resident = from_json(malformed).unwrap_err();
        let native =
            from_json_with_memory(malformed, None, &mut Memory::new(&mut Resident)).unwrap_err();
        assert_eq!(native, ReadError::Lexical(resident));
        let bytes = br#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":{"type":"uri","value":"http://example.org/x"}},false]}}"#;
        let native =
            from_json_with_memory(bytes, Some(1), &mut Memory::new(&mut Resident)).unwrap();
        assert_eq!(native, from_json_bounded(bytes, 1).unwrap());
        assert!(native.truncated);
        assert_eq!(native.solutions.rows.len(), 1);
    }

    #[test]
    fn superseded_native_refusals_release_original_tokens_and_terms() {
        // The parser must finish syntax validation after a semantic refusal.
        // A later syntax error supersedes it and releases every native payload.
        for cell in [
            r#"{"type":"unrecognized-original","value":"x"}"#,
            r#"{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"literal","value":"original-predicate"},"object":{"type":"uri","value":"http://example.org/o"}}}"#,
            r#"{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"uri","value":"http://example.org/p"}}}"#,
        ] {
            let input = format!(
                r#"{{"head":{{"vars":["x"]}},"results":{{"bindings":[{{"x":{cell}}}]}},"tail":]}}"#
            );
            let mut resident = Resident;
            let mut memory = Memory::new(&mut resident);
            let error = from_json_with_memory(input.as_bytes(), None, &mut memory).unwrap_err();
            assert!(matches!(error, ReadError::Lexical(Error::Json(_))));
            assert_eq!(memory.admitted_bytes(), 0);
        }
    }

    #[test]
    fn semantic_refusals_keep_only_their_original_payload_charge() {
        for (cell, retained) in [
            (
                r#"{"type":"unrecognized-original","value":"x"}"#,
                "unrecognized-original".len(),
            ),
            (
                r#"{"type":"literal","value":"original lexical form","xml:lang":"en us"}"#,
                "en us".len(),
            ),
            (
                r#"{"type":"literal","value":"original lexical form","xml:lang":"en","its:dir":"backwards"}"#,
                "backwards".len(),
            ),
            (
                r#"{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"bnode","value":"p"},"object":{"type":"uri","value":"http://example.org/o"}}}"#,
                1,
            ),
            (
                r#"{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"uri","value":"http://example.org/p"}}}"#,
                0,
            ),
            (r#"{"value":"original value without a type"}"#, 0),
        ] {
            for ceiling in [None, Some(u64::MAX)] {
                let mut resident = Resident;
                let mut memory = Memory::new(&mut resident);
                let original = memory.string("caller-owned-before-parsing").unwrap();
                let original_bytes = memory.admitted_bytes();
                // A successful prefix row must be discarded when the next
                // row fails, under both original parser policies.
                let input = format!(
                    r#"{{"head":{{"vars":["x"]}},"results":{{"bindings":[{{"x":{{"type":"uri","value":"http://example.org/first"}}}},{{"x":{cell}}}]}}}}"#
                );
                let error =
                    from_json_with_memory(input.as_bytes(), ceiling, &mut memory).unwrap_err();
                drop(input);
                assert_eq!(memory.admitted_bytes(), original_bytes + retained);
                let ReadError::Lexical(error) = error else {
                    panic!("the original semantic refusal must survive");
                };
                drop(purrdf_testkit::error_capture::Capture::read(&error));
                release_error(error, &mut memory).unwrap();
                assert_eq!(memory.admitted_bytes(), original_bytes);
                memory.release_string(original).unwrap();
                assert_eq!(memory.admitted_bytes(), 0);
            }
        }

        // Cleanup refusal retains the failed term and every untouched row,
        // component and original container allocation. A scalar neighbor can
        // still be released without asking for traversal growth.
        struct Grant<'a> {
            live: usize,
            refuse_growth: &'a std::cell::Cell<bool>,
            refused: &'a std::cell::Cell<usize>,
        }
        impl Admission for Grant<'_> {
            fn resize(&mut self, next: usize) -> Result<(), StorageError> {
                if next > self.live && self.refuse_growth.get() {
                    self.refused.set(self.refused.get() + 1);
                    return Err(StorageError::AdmissionFailed);
                }
                self.live = next;
                Ok(())
            }
        }
        let refuse_growth = std::cell::Cell::new(false);
        let refused = std::cell::Cell::new(0);
        let mut grant = Grant {
            live: 0,
            refuse_growth: &refuse_growth,
            refused: &refused,
        };
        let mut memory = Memory::new(&mut grant);
        let source = TermValue::Triple {
            s: TermValue::iri("subject").into(),
            p: TermValue::iri("predicate").into(),
            o: TermValue::iri("object").into(),
        };
        for container in ["rows", "parts", "scratch"] {
            let term = source
                .try_clone_with_memory(&mut memory, |_| Ok::<_, std::convert::Infallible>(()))
                .unwrap();
            let neighbor = TermValue::Iri(memory.string("healthy-neighbor").unwrap());
            let residual = if container == "rows" {
                let mut first = Vec::new();
                let first_term = TermValue::Iri(memory.string("first-row").unwrap());
                memory.push(&mut first, Some(first_term)).unwrap();
                let mut last = Vec::new();
                let remaining_term = TermValue::Iri(memory.string("remaining-column").unwrap());
                memory.push(&mut last, Some(remaining_term)).unwrap();
                memory.push(&mut last, Some(term)).unwrap();
                let mut rows = Vec::new();
                memory.push(&mut rows, first).unwrap();
                memory.push(&mut rows, last).unwrap();
                ReleaseRetained::Rows(rows)
            } else {
                let predicate = TermValue::Iri(memory.string("remaining-predicate").unwrap());
                let object = TermValue::Iri(memory.string("remaining-object").unwrap());
                let original_error = ReadRefusal::token(
                    crate::SparqlResultsFormat::Json,
                    ReadToken::BindingType,
                    "original-component-kind",
                    &mut memory,
                );
                let parts = memory
                    .boxed(TripleParts {
                        subject: Some(Ok(term)),
                        predicate: Some(Ok(predicate)),
                        object: Some(Ok(object)),
                        reading: None,
                        finished: Some(Err(original_error)),
                        boxed: [None, None, None],
                    })
                    .unwrap();
                if container == "parts" {
                    ReleaseRetained::Parts(parts)
                } else {
                    let text = memory.string("original-owned-metadata").unwrap();
                    let fields = memory
                        .boxed(BindingFields {
                            // Cleanup must not project a span or require the original source.
                            kind: Some(Member::String(Lexical::Borrowed(
                                usize::MAX - 1..usize::MAX,
                            ))),
                            datatype: Some(Member::String(Lexical::Owned(text))),
                            ..BindingFields::default()
                        })
                        .unwrap();
                    let mut open = Vec::new();
                    memory.push(&mut open, Open::Binding(fields)).unwrap();
                    memory.push(&mut open, Open::Triple(parts)).unwrap();
                    let mut bound = Vec::new();
                    memory.push(&mut bound, true).unwrap();
                    ReleaseRetained::Scratch(RowScratch { open, bound })
                }
            };
            let pending = JsonRelease::new(residual);
            let semantic = if container == "rows" {
                Some(ReadRefusal::token(
                    crate::SparqlResultsFormat::Json,
                    ReadToken::BindingType,
                    "original-row-kind",
                    &mut memory,
                ))
            } else {
                None
            };
            let expected_primary = semantic.as_ref().map_or_else(
                || format_error("unknown binding type `original-component-kind`"),
                ToString::to_string,
            );
            let baseline = memory.admitted_bytes();
            refuse_growth.set(true);
            let before_refusal = refused.get();
            let error = if let Some(semantic) = semantic {
                finish_cleanup(pending, Ok(Err(semantic)), |(): (), _| {}, &mut memory).unwrap_err()
            } else {
                pending.release_with_memory(&mut memory).unwrap_err()
            };
            assert_eq!(
                error.to_string(),
                expected_primary,
                "the original semantic refusal remains primary"
            );
            assert_eq!(
                memory.admitted_bytes(),
                baseline,
                "no retained payload or backing credit was lost"
            );
            assert_eq!(
                refused.get() - before_refusal,
                1,
                "cleanup starts no later census after the first refusal"
            );
            assert_eq!(
                purrdf_testkit::error_capture::Capture::read(&error)
                    .child("cause")
                    .code,
                "sparql-results-format"
            );
            let Error::JsonRelease { owner, cause } = error else {
                panic!("the original residual owner survives");
            };
            assert_eq!(cause, StorageError::AdmissionFailed);
            match (&owner.rows, &owner.frame) {
                (Some(rows), _) => {
                    assert_eq!(rows.len(), 2);
                    assert_eq!(rows[0][0], Some(TermValue::iri("first-row")));
                    assert_eq!(rows[1].len(), 2);
                    assert_eq!(rows[1][0], Some(TermValue::iri("remaining-column")));
                    assert_eq!(rows[1][1], Some(source.clone()));
                    assert!(
                        rows.capacity() >= 2 && rows[0].capacity() >= 1 && rows[1].capacity() >= 2
                    );
                }
                (_, Some(Open::Triple(parts))) => {
                    assert_eq!(parts.subject.as_ref().unwrap().as_ref().unwrap(), &source);
                    assert_eq!(
                        parts.predicate.as_ref().unwrap().as_ref().unwrap(),
                        &TermValue::iri("remaining-predicate")
                    );
                    assert_eq!(
                        parts.object.as_ref().unwrap().as_ref().unwrap(),
                        &TermValue::iri("remaining-object")
                    );
                }
                _ if owner.scratch.is_some() => {
                    let scratch = owner.scratch.as_ref().unwrap();
                    assert_eq!(scratch.open.len(), 2);
                    assert_eq!(scratch.bound, [true]);
                    let Open::Binding(fields) = &scratch.open[0] else {
                        panic!("the original parent frame survives");
                    };
                    assert!(matches!(
                        fields.kind,
                        Some(Member::String(Lexical::Borrowed(_)))
                    ));
                    assert!(matches!(
                        fields.datatype,
                        Some(Member::String(Lexical::Owned(_)))
                    ));
                    let Open::Triple(parts) = &scratch.open[1] else {
                        panic!("the failed original component frame survives");
                    };
                    assert_eq!(parts.subject.as_ref().unwrap().as_ref().unwrap(), &source);
                    assert!(parts.finished.as_ref().unwrap().is_err());
                }
                _ => panic!("the selected original container survives"),
            }
            release_term(neighbor, &mut memory).unwrap();
            refuse_growth.set(false);
            release_error(Error::JsonRelease { owner, cause }, &mut memory).unwrap();
            assert_eq!(memory.admitted_bytes(), 0);
        }

        // Projected order selects a in both source orders. Disposing b's
        // non-IRI nested-triple predicate must not replace a or lose b's owner.
        struct PredicateCensusRefusal {
            live: usize,
            armed: bool,
            refused: bool,
            later_growths: usize,
        }
        impl Admission for PredicateCensusRefusal {
            fn resize(&mut self, next: usize) -> Result<(), StorageError> {
                if next > self.live {
                    if self.refused {
                        self.later_growths += 1;
                        return Err(StorageError::AdmissionFailed);
                    }
                    // This is the original term census's three-answer buffer.
                    if self.armed && next - self.live == 3 * size_of::<usize>() {
                        self.refused = true;
                        return Err(StorageError::AdmissionFailed);
                    }
                }
                self.live = next;
                Ok(())
            }
        }
        let a = r#""a":{"type":"literal","value":"selected","xml:lang":"en us"}"#;
        let b = r#""b":{"type":"triple","value":{"subject":{"type":"uri","value":"urn:outer-s"},"predicate":{"type":"triple","value":{"subject":{"type":"uri","value":"urn:inner-s"},"predicate":{"type":"uri","value":"urn:inner-p"},"object":{"type":"uri","value":"urn:inner-o"}}},"object":{"type":"uri","value":"urn:outer-o"}}}"#;
        for (first, second) in [(a, b), (b, a)] {
            let input = format!(
                r#"{{"head":{{"vars":["a","b"]}},"results":{{"bindings":[{{{first},{second}}}]}}}}"#
            );
            let mut grant = PredicateCensusRefusal {
                live: 0,
                armed: false,
                refused: false,
                later_growths: 0,
            };
            let mut memory = Memory::new(&mut grant);
            let parent = memory.string("healthy-original-parent").unwrap();
            let neighbor = TermValue::Iri(memory.string("healthy-neighbor").unwrap());
            memory.admission_mut().armed = true;
            let error = from_json_with_memory(input.as_bytes(), None, &mut memory).unwrap_err();
            drop(input);
            assert!(
                memory.admission_mut().refused,
                "the nested predicate disposal reached its original census"
            );
            assert_eq!(memory.admission_mut().later_growths, 0);
            assert!(error.to_string().contains("invalid language tag `en us`"));
            let ReadError::Lexical(Error::JsonRelease { owner, cause }) = error else {
                panic!("both semantic payloads survive the physical refusal");
            };
            assert_eq!(cause, StorageError::AdmissionFailed);
            assert!(
                matches!(&owner.primary, Some(NativePrimary::LanguageTag { language, .. }) if language == "en us")
            );
            assert!(matches!(
                &owner.superseded,
                Some(NativePrimary::Format(ReadRefusal {
                    reason: ReadReason::Predicate(TermValue::Triple { .. }),
                    ..
                }))
            ));
            assert_eq!(parent, "healthy-original-parent");
            neighbor.release_with_memory(&mut memory).unwrap();
            assert_eq!(
                memory.admission_mut().later_growths,
                0,
                "a healthy scalar neighbor requires no new census buffer"
            );
            memory.admission_mut().armed = false;
            memory.admission_mut().refused = false;
            release_error(Error::JsonRelease { owner, cause }, &mut memory).unwrap();
            assert_eq!(
                memory.admitted_bytes(),
                parent.capacity(),
                "selected and discarded causes plus every original parser container were refunded"
            );
            memory.release_string(parent).unwrap();
            assert_eq!(memory.admitted_bytes(), 0);
        }
    }
}
