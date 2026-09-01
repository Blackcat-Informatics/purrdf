// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One closed enum over both value spaces this crate can reach.

use alloc::string::{String, ToString};

use purrdf_xsd::XsdValue;

use crate::datatype::CdtDatatype;
use crate::parse::parse_cdt;
use crate::value::CdtValue;

/// What a literal's `(lexical form, datatype IRI)` pair denotes.
///
/// # Why a closed enum and not a trait
///
/// A `trait Datatype` with a registry of implementations is a **runtime datatype
/// registry**, which this project forbids: it would make a literal's meaning depend
/// on which implementations happened to be registered in the process, so the same
/// dataset could answer two different queries in two different hosts. The set of
/// value spaces PurRDF models is fixed at compile time — the XSD 1.1 value spaces
/// and the two SEP-0009 composites — so a closed enum states that fact in the type
/// system, exactly as [`purrdf_xsd::XsdDatatype`] and [`CdtDatatype`] already do for
/// their own datatype sets.
///
/// # The tri-state is preserved, not collapsed
///
/// [`purrdf_xsd::parse_by_iri`] separates three outcomes, and so does this: a
/// literal that parsed ([`Self::Xsd`] / [`Self::Cdt`]), a literal whose datatype is
/// outside every value space PurRDF models ([`Self::Opaque`]), and a literal whose
/// datatype IS modelled but whose lexical form is wrong for it
/// ([`Self::IllTyped`]). The last two look alike to a caller that only asks "did it
/// parse?", but they are opposites: an opaque literal is perfectly well-formed RDF
/// that this crate simply has nothing to say about, whereas an ill-typed literal is
/// a defect a validator must report.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum LiteralValue {
    /// The datatype is an XSD value space and the lexical form parsed.
    Xsd(XsdValue),
    /// The datatype is a SEP-0009 composite and the lexical form parsed.
    Cdt(CdtValue),
    /// The datatype is one PurRDF models, but the lexical form is not in its
    /// lexical space. The typed diagnostic is available from
    /// [`purrdf_xsd::parse_by_iri`] / [`crate::parse_cdt_by_iri`], which return the
    /// exact error; this variant records the offending pair for a caller that only
    /// needs to know the literal is ill-typed.
    IllTyped {
        /// The datatype IRI the lexical form failed against.
        datatype: String,
        /// The offending lexical form.
        lexical: String,
    },
    /// The datatype is outside every value space PurRDF models. The literal is a
    /// plain term; this is **not** a failure.
    Opaque,
}

/// Resolve a literal into the value it denotes.
///
/// Subsumes [`purrdf_xsd::parse_by_iri`]: the composite datatypes are tried first
/// (they are disjoint from the XSD namespace, so the order is immaterial to the
/// result), and every other IRI falls through to the XSD value space with its
/// tri-state intact. `purrdf-xsd` itself is untouched by this — it stays a pure,
/// zero-dependency leaf that knows nothing about composites.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{LiteralValue, parse_literal};
///
/// // The XSD value space.
/// let integer = parse_literal("42", "http://www.w3.org/2001/XMLSchema#integer");
/// assert!(matches!(integer, LiteralValue::Xsd(_)));
///
/// // The composite value space.
/// let list = parse_literal("[1,2]", "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List");
/// assert!(matches!(list, LiteralValue::Cdt(_)));
///
/// // Not this value space — an ordinary opaque term, NOT an error.
/// let opaque = parse_literal("anything", "http://example.org/custom");
/// assert!(matches!(opaque, LiteralValue::Opaque));
///
/// // This value space, but malformed — a defect, and distinguishable from opaque.
/// let bad = parse_literal("[1,", "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List");
/// assert!(matches!(bad, LiteralValue::IllTyped { .. }));
/// let bad_xsd = parse_literal("maybe", "http://www.w3.org/2001/XMLSchema#boolean");
/// assert!(matches!(bad_xsd, LiteralValue::IllTyped { .. }));
/// ```
#[must_use]
pub fn parse_literal(lexical: &str, datatype: &str) -> LiteralValue {
    let ill_typed = || LiteralValue::IllTyped {
        datatype: datatype.to_string(),
        lexical: lexical.to_string(),
    };
    if let Some(composite) = CdtDatatype::from_iri(datatype) {
        return match parse_cdt(lexical, composite) {
            Ok(value) => LiteralValue::Cdt(value),
            Err(_) => ill_typed(),
        };
    }
    match purrdf_xsd::parse_by_iri(lexical, datatype) {
        Ok(Some(value)) => LiteralValue::Xsd(value),
        Ok(None) => LiteralValue::Opaque,
        Err(_) => ill_typed(),
    }
}
