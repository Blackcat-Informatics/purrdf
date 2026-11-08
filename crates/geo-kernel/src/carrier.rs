// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shared RDF geometry-literal boundary, selected by datatype.

use crate::{GeoError, GeoTerm, GeoVocab, GeometryLiteral, geojson, wkt};
use purrdf_core::TermValue;
use purrdf_lex::allocation::StorageError;
use std::cell::RefCell;

mod admission;
mod output;
mod storage;
mod term;
pub(crate) mod writer;
pub use admission::{CarrierStorage, ParseAdmission as InputAdmission, ParseReceipt};
pub(crate) use admission::{
    JsonProgress, ParseAdmission, parse_in_policy, whitespace_only, with_storage,
};
pub(crate) use output::{MaterializationStorage, MaterializedLiteral};
#[cfg(test)]
pub(crate) use storage::owned_coordinates_storage;
pub(crate) use storage::{GeometryStorageVisitor, reserve_metadata, visit_geometry_storage};
pub use term::{
    CarrierFormat, error_from_display_metered, literal_from_display_metered, term_in_context,
    term_in_context_metered, term_source_in_policy, term_source_metered, term_source_with_storage,
};

/// The original native array growth used by both geometry carrier readers.
/// Resident adapters describe their existing caller-owned buffer before using
/// the same growth body; native buffers are already in their original account.
pub(crate) fn push<T>(
    storage: Option<&CarrierStorage<'_>>,
    values: &mut Vec<T>,
    value: T,
) -> Result<(), StorageError> {
    with_storage(storage, |memory| {
        if storage.is_none() {
            memory.add_bytes(
                core::alloc::Layout::array::<T>(values.capacity())
                    .map_err(|_| StorageError::SizeOverflow)?
                    .size(),
            )?;
        }
        memory.push(values, value)
    })
}

pub(crate) fn reserve<T>(
    storage: Option<&CarrierStorage<'_>>,
    values: &mut Vec<T>,
    capacity: usize,
) -> Result<(), StorageError> {
    with_storage(storage, |memory| {
        if storage.is_none() {
            memory.add_bytes(
                core::alloc::Layout::array::<T>(values.capacity())
                    .map_err(|_| StorageError::SizeOverflow)?
                    .size(),
            )?;
        }
        memory.reserve(values, capacity)
    })
}

/// Destroy the original array before returning its exact layout to the account.
pub(crate) fn release<T>(
    storage: Option<&CarrierStorage<'_>>,
    values: Vec<T>,
) -> Result<(), StorageError> {
    with_storage(storage, |memory| {
        if storage.is_none() {
            memory.add_bytes(
                core::alloc::Layout::array::<T>(values.capacity())
                    .map_err(|_| StorageError::SizeOverflow)?
                    .size(),
            )?;
        }
        memory.release_vec(values)
    })
}

pub(crate) fn boxed<T>(
    storage: Option<&CarrierStorage<'_>>,
    value: T,
) -> Result<Box<T>, StorageError> {
    with_storage(storage, |memory| {
        memory.add_bytes(core::alloc::Layout::new::<T>().size())?;
        purrdf_lex::allocation::try_boxed(value).map_err(|_| StorageError::AllocationFailed)
    })
}

/// Render one stable typed text diagnostic under its original producer account.
/// Fields must render without private allocations; numerical groups are prepared
/// separately. The caller retains the account through error destruction.
#[must_use]
pub fn text_error(
    storage: Option<&CarrierStorage<'_>>,
    message: core::fmt::Arguments<'_>,
    make: fn(String) -> GeoError,
) -> GeoError {
    match with_storage(storage, |memory| memory.format(&message)) {
        Ok(message) => make(message),
        Err(error) => GeoError::Storage(error),
    }
}

pub(crate) fn release_string(
    storage: Option<&CarrierStorage<'_>>,
    value: String,
) -> Result<(), StorageError> {
    with_storage(storage, |memory| {
        if storage.is_none() {
            memory.add_bytes(value.capacity())?;
        }
        memory.release_string(value)
    })
}

pub(crate) fn resident_shared_text(
    text: &str,
) -> Result<purrdf_lex::allocation::SharedText, GeoError> {
    shared_text_with_storage(text, None)
}

pub(crate) fn shared_text_with_storage(
    text: &str,
    storage: Option<&CarrierStorage<'_>>,
) -> Result<purrdf_lex::allocation::SharedText, GeoError> {
    with_storage(storage, |memory| {
        let text = memory.string(text)?;
        shared_owned_text(text, memory)
    })
    .map_err(GeoError::Storage)
}

/// Move a caller-owned configuration buffer through the same immutable
/// publication home. Native producers retain their own original account.
pub(crate) fn resident_owned_text(
    text: String,
) -> Result<purrdf_lex::allocation::SharedText, GeoError> {
    use purrdf_lex::allocation::{Memory, Resident};
    let mut resident = Resident;
    let mut memory = Memory::resume(&mut resident, text.capacity());
    shared_owned_text(text, &mut memory).map_err(GeoError::Storage)
}

fn shared_owned_text<S: purrdf_lex::allocation::Admission + ?Sized>(
    text: String,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<purrdf_lex::allocation::SharedText, StorageError> {
    use purrdf_lex::allocation::{Resident, SharedText};
    memory.add_bytes(SharedText::allocation_layout::<Resident>().size())?;
    // The explicit resident carrier transfers to its Rust caller. Bounded
    // evaluator producers instead publish through the original observer above.
    SharedText::try_from_admitted(text, Resident).map_err(|_| StorageError::AllocationFailed)
}
#[cfg(test)]
mod tests;

/// Complete original carrier and its admitted ingestion receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedGeometry {
    literal: GeometryLiteral,
    receipt: ParseReceipt,
}
impl ParsedGeometry {
    /// Original exact source coordinates and declared carrier reference.
    #[must_use]
    pub const fn literal(&self) -> &GeometryLiteral {
        &self.literal
    }
    /// Work and retained source allowance already charged during parsing.
    #[must_use]
    pub const fn receipt(&self) -> ParseReceipt {
        self.receipt
    }
    /// Move the exact source without copying coordinate allocations.
    #[must_use]
    pub fn into_literal(self) -> GeometryLiteral {
        self.literal
    }
}

/// Parse with explicit complete source/resource admission.
///
/// # Errors
/// Preserves carrier argument errors and fatal work, memory and output refusals.
pub fn geometry_arg_in_policy(
    vocab: &GeoVocab,
    value: &TermValue,
    policy: crate::ExecutionPolicy,
) -> Result<ParsedGeometry, GeoError> {
    parse_admitted(vocab, value, policy, None, None)
}

/// Parse with bounded input and exact-construction cancellation/work polls.
///
/// # Errors
/// A refused chunk stops before further source geometry is constructed.
pub fn geometry_arg_metered(
    vocab: &GeoVocab,
    value: &TermValue,
    policy: crate::ExecutionPolicy,
    observer: &mut dyn crate::MetricWorkObserver,
) -> Result<ParsedGeometry, GeoError> {
    parse_admitted(vocab, value, policy, Some(observer), None)
}

/// Parse through the original grammar under a separately retained physical
/// account. Work/cancellation polls and native integer factories use `observer`;
/// all parser containers, lexical text and diagnostics use `storage`.
///
/// The caller keeps the account alive until destroying or publishing the
/// returned literal with its original grant. This method never adopts a
/// resident allocation after it has been made.
///
/// # Errors
/// Preserves the first physical, work, source or malformed-carrier failure.
pub fn geometry_arg_with_storage(
    vocab: &GeoVocab,
    value: &TermValue,
    policy: crate::ExecutionPolicy,
    observer: &mut dyn crate::MetricWorkObserver,
    storage: &CarrierStorage<'_>,
) -> Result<ParsedGeometry, GeoError> {
    parse_admitted(vocab, value, policy, Some(observer), Some(storage))
}

fn parse_admitted(
    vocab: &GeoVocab,
    value: &TermValue,
    policy: crate::ExecutionPolicy,
    observer: Option<&mut dyn crate::MetricWorkObserver>,
    storage: Option<&CarrierStorage<'_>>,
) -> Result<ParsedGeometry, GeoError> {
    let bytes = match value {
        TermValue::Literal { lexical_form, .. } => lexical_form.len(),
        _ => 0,
    };
    let mut source = ParseAdmission::new(bytes, policy, observer)?;
    if let Some(storage) = storage {
        source = source.with_storage(storage);
    }
    term::admit_term_source(value, bytes as u64, &mut source)?;
    let admission = RefCell::new(source);
    let result = geometry_arg_inner(vocab, value, Some(&admission));
    if let Some(error) = admission.borrow_mut().take_error() {
        return Err(error);
    }
    Ok(ParsedGeometry {
        literal: result?,
        receipt: admission.borrow().receipt(),
    })
}

/// Read a geometry-literal argument.
///
/// The datatype decides the codec, and it decides it against the **caller's**
/// vocabulary: `geo:wktLiteral` is parsed as WKT against the vocabulary's default
/// coordinate reference system (a literal may override it with an explicit
/// `<IRI>` prefix), and `geo:geoJSONLiteral` as GeoJSON against the one system
/// RFC 7946 admits.
///
/// A plain `xsd:string` is **not** accepted, and that is not pedantry: a geometry
/// literal's datatype is what makes its lexical form a geometry rather than text
/// that happens to look like one, and a store that has lost the datatype has lost
/// the fact. The refusal names the datatype that did arrive so the gap is
/// diagnosable.
///
/// # Errors
///
/// [`GeoError::Unsupported`] for `geo:gmlLiteral`, `geo:kmlLiteral` and
/// `geo:dggsLiteral` — spec-defined datatypes whose codecs this crate does not
/// have — naming the datatype. [`GeoError::CarrierInput`] retains any other
/// datatype or non-literal term. [`GeoError::WktSyntax`] retains WKT grammar
/// failures and [`GeoError::JsonSyntax`] retains JSON grammar failures.
/// Structural and GeoJSON shape failures retain their literal-error channel.
/// WKT and GeoJSON use their parsers' default geometry resource limits; use
/// [`geometry_arg_in_policy`] for an explicit policy.
pub fn geometry_arg(vocab: &GeoVocab, value: &TermValue) -> Result<GeometryLiteral, GeoError> {
    geometry_arg_inner(vocab, value, None)
}

fn geometry_arg_inner(
    vocab: &GeoVocab,
    value: &TermValue,
    admission: Option<&RefCell<ParseAdmission<'_, '_, '_>>>,
) -> Result<GeometryLiteral, GeoError> {
    let storage = admission.and_then(|admission| admission.borrow().storage());
    let format = geometry_format(vocab, value, storage)?;
    let TermValue::Literal { lexical_form, .. } = value else {
        unreachable!("the original carrier-format boundary admits only literals");
    };
    match format {
        CarrierFormat::Wkt if admission.is_none() => {
            wkt::parse(lexical_form, vocab.default_wkt_crs())
        }
        CarrierFormat::Wkt => wkt::parse_admitted(lexical_form, vocab.default_wkt_crs(), admission),
        CarrierFormat::GeoJson if admission.is_none() => {
            geojson::parse(lexical_form, vocab.geojson_crs())
        }
        CarrierFormat::GeoJson => {
            geojson::parse_admitted(lexical_form, vocab.geojson_crs(), admission)
        }
    }
}

/// Classify the original geometry argument without parsing or copying its text.
/// Invalid terms use the same admitted diagnostic body as the native parser.
///
/// # Errors
/// Preserves unsupported-codec, wrong-type and physical diagnostic refusal.
pub fn geometry_argument_format(
    vocab: &GeoVocab,
    value: &TermValue,
    storage: &CarrierStorage<'_>,
) -> Result<CarrierFormat, GeoError> {
    geometry_format(vocab, value, Some(storage))
}

/// Retain the original admitted input when its parsed owner is missing.
/// Original datatype/type refusals keep priority over this producer invariant.
#[must_use]
pub fn missing_parsed_source_error(
    vocab: &GeoVocab,
    value: &TermValue,
    storage: &CarrierStorage<'_>,
) -> GeoError {
    match geometry_format(vocab, value, Some(storage)) {
        Err(original) => original,
        Ok(_) => crate::CarrierInputError::capture(
            value,
            crate::CarrierInputReason::MissingParsedSource,
            Some(storage),
        ),
    }
}

fn geometry_format(
    vocab: &GeoVocab,
    value: &TermValue,
    storage: Option<&CarrierStorage<'_>>,
) -> Result<CarrierFormat, GeoError> {
    let TermValue::Literal { datatype, .. } = value else {
        return Err(crate::CarrierInputError::capture(
            value,
            crate::CarrierInputReason::ExpectedLiteral,
            storage,
        ));
    };
    // `term_of` is a prefix strip plus a table scan over fixed strings, so this
    // dispatch allocates nothing on the per-row path.
    match vocab.term_of(datatype) {
        Some(GeoTerm::WktLiteral) => Ok(CarrierFormat::Wkt),
        Some(GeoTerm::GeoJsonLiteral) => Ok(CarrierFormat::GeoJson),
        Some(term @ (GeoTerm::GmlLiteral | GeoTerm::KmlLiteral | GeoTerm::DggsLiteral)) => {
            let datatype = with_storage(storage, |memory| memory.string(datatype))
                .map_err(GeoError::Storage)?;
            Err(GeoError::Unsupported(crate::UnsupportedRefusal::Datatype {
                datatype,
                term,
            }))
        }
        _ => Err(crate::CarrierInputError::capture(
            value,
            crate::CarrierInputReason::UnknownDatatype,
            storage,
        )),
    }
}

/// Name what a term is, for a refusal message.
#[must_use]
pub fn term_description(value: &TermValue) -> String {
    TermDescription(value).to_string()
}

/// Borrow the original term's diagnostic fields without constructing text or
/// recursively walking a triple. The physical sink owns the final rendering.
#[must_use]
pub fn term_description_display(value: &TermValue) -> impl core::fmt::Display + '_ {
    TermDescription(value)
}

struct TermDescription<'a>(&'a TermValue);

impl core::fmt::Display for TermDescription<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            TermValue::Iri(iri) => write!(formatter, "the IRI <{iri}>"),
            TermValue::Blank { label, .. } => write!(formatter, "the blank node _:{label}"),
            TermValue::Triple { .. } => formatter.write_str("a triple term"),
            TermValue::Literal {
                datatype,
                language: Some(tag),
                ..
            } => write!(formatter, "a literal tagged @{tag} (datatype <{datatype}>)"),
            TermValue::Literal { datatype, .. } => {
                write!(formatter, "a literal of datatype <{datatype}>")
            }
        }
    }
}
