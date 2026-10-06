// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shared RDF geometry-literal boundary, selected by datatype.

use crate::{GeoError, GeoTerm, GeoVocab, GeometryLiteral, geojson, wkt};
use purrdf_core::TermValue;
use std::{cell::RefCell, rc::Rc};

mod admission;
mod output;
mod storage;
mod term;
pub(crate) mod writer;
pub(crate) use admission::{JsonProgress, ParseAdmission, whitespace_only};
pub use admission::{ParseAdmission as InputAdmission, ParseReceipt};
pub(crate) use output::MaterializationStorage;
pub(crate) use storage::{
    GeometryStorageVisitor, owned_coordinates_storage, reserve_metadata, reserve_metadata_for_push,
    visit_geometry_storage,
};
pub use term::{
    CarrierFormat, term_in_context, term_in_context_metered, term_source_in_policy,
    term_source_metered,
};
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
    parse_admitted(vocab, value, policy, None)
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
    parse_admitted(vocab, value, policy, Some(observer))
}

fn parse_admitted(
    vocab: &GeoVocab,
    value: &TermValue,
    policy: crate::ExecutionPolicy,
    observer: Option<&mut dyn crate::MetricWorkObserver>,
) -> Result<ParsedGeometry, GeoError> {
    let bytes = match value {
        TermValue::Literal { lexical_form, .. } => lexical_form.len(),
        _ => 0,
    };
    let mut source = ParseAdmission::new(bytes, policy, observer)?;
    term::admit_term_source(value, bytes as u64, &mut source)?;
    let admission = Rc::new(RefCell::new(source));
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
/// have — naming the datatype. [`GeoError::Literal`] for any other datatype, for
/// a non-literal term, and for a well-typed literal whose lexical form is
/// malformed.
pub fn geometry_arg(vocab: &GeoVocab, value: &TermValue) -> Result<GeometryLiteral, GeoError> {
    geometry_arg_inner(vocab, value, None)
}

fn geometry_arg_inner(
    vocab: &GeoVocab,
    value: &TermValue,
    admission: Option<&Rc<RefCell<ParseAdmission<'_>>>>,
) -> Result<GeometryLiteral, GeoError> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        ..
    } = value
    else {
        return Err(GeoError::literal(format!(
            "a geof: function takes a geometry literal and {} arrived; a geometry's datatype is \
             what makes its lexical form a geometry rather than text, so purrdf-geo refuses \
             rather than guessing a codec",
            term_description(value)
        )));
    };
    // `term_of` is a prefix strip plus a table scan over fixed strings, so this
    // dispatch allocates nothing on the per-row path.
    match vocab.term_of(datatype) {
        Some(GeoTerm::WktLiteral) => {
            wkt::parse_admitted(lexical_form, vocab.default_wkt_crs(), admission)
        }
        Some(GeoTerm::GeoJsonLiteral) => {
            geojson::parse_admitted(lexical_form, vocab.geojson_crs(), admission)
        }
        Some(term @ (GeoTerm::GmlLiteral | GeoTerm::KmlLiteral | GeoTerm::DggsLiteral)) => {
            Err(GeoError::unsupported(format!(
                "<{datatype}> is the geo:{} datatype, and purrdf-geo implements the WKT and \
                 GeoJSON codecs only; it refuses rather than reading the lexical form as a \
                 serialization it is not",
                term.local_name()
            )))
        }
        _ => Err(GeoError::literal(format!(
            "<{datatype}> is not a geometry datatype; a geof: function takes a geo:wktLiteral or \
             a geo:geoJSONLiteral, and an xsd:string carrying the same characters is text rather \
             than a geometry"
        ))),
    }
}

/// Name what a term is, for a refusal message.
pub fn term_description(value: &TermValue) -> String {
    match value {
        TermValue::Iri(iri) => format!("the IRI <{iri}>"),
        TermValue::Blank { label, .. } => format!("the blank node _:{label}"),
        TermValue::Triple { .. } => "a triple term".to_owned(),
        TermValue::Literal {
            datatype,
            language: Some(tag),
            ..
        } => format!("a literal tagged @{tag} (datatype <{datatype}>)"),
        TermValue::Literal { datatype, .. } => format!("a literal of datatype <{datatype}>"),
    }
}
