// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original RDF inputs refused by the geometry carrier boundary.

use purrdf_core::TermValue;
use purrdf_lex::allocation::StorageError;

/// Which original carrier-input requirement failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CarrierInputReason {
    /// A geometry argument must be a literal.
    ExpectedLiteral,
    /// Its datatype must select a supported geometry carrier.
    UnknownDatatype,
    /// An admitted geometry argument has no retained parsed source.
    MissingParsedSource,
    /// A radius argument must be an untagged numeric literal.
    RadiusExpectedLiteral,
    /// A radius literal must name a numeric XSD datatype.
    RadiusDatatype,
}

impl purrdf_hash::error::StructuredError for CarrierInputReason {
    purrdf_hash::error_visit! {
        ExpectedLiteral => "carrier-input-expected-literal" {},
        UnknownDatatype => "carrier-input-unknown-datatype" {},
        MissingParsedSource => "carrier-input-missing-parsed-source" {},
        RadiusExpectedLiteral => "carrier-input-radius-literal" {},
        RadiusDatatype => "carrier-input-radius-datatype" {},
    }
}

/// An original term copied under the carrier's original physical account.
/// The caller retains that account until this payload dies or is published.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CarrierInputError {
    reason: CarrierInputReason,
    value: TermValue,
    // Exact surviving layouts born in the original native factory. Traversal
    // scratch is already refunded before this value is published.
    owned_bytes: usize,
}

impl purrdf_hash::error::StructuredError for CarrierInputError {
    purrdf_hash::error_visit! {
        @record self { reason, value, owned_bytes } => "carrier-input" {
            "reason" => Record(&self.reason), "value" => Term(&self.value),
            "owned_bytes" => ByteCount(self.owned_bytes as u128)
        }
    }
}

impl CarrierInputError {
    /// Original input, including blank scope and every nested term component.
    #[must_use]
    pub const fn value(&self) -> &TermValue {
        &self.value
    }

    /// Native requirement that failed at this input boundary.
    #[must_use]
    pub const fn reason(&self) -> CarrierInputReason {
        self.reason
    }

    /// A missing parsed owner is a producer configuration invariant.
    #[must_use]
    pub const fn is_configuration(&self) -> bool {
        matches!(self.reason, CarrierInputReason::MissingParsedSource)
    }

    pub(crate) const fn owned_bytes(&self) -> usize {
        self.owned_bytes
    }

    pub(crate) fn capture(
        value: &TermValue,
        reason: CarrierInputReason,
        storage: Option<&crate::carrier::CarrierStorage<'_>>,
    ) -> super::GeoError {
        crate::carrier::with_storage(storage, |memory| {
            Self::capture_with_memory(value, reason, memory)
        })
        .unwrap_or_else(super::GeoError::Storage)
    }

    pub(crate) fn capture_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        value: &TermValue,
        reason: CarrierInputReason,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<super::GeoError, StorageError> {
        memory.try_scope(|memory| {
            let baseline = memory.admitted_bytes();
            let value = value
                .try_clone_with_memory(memory, |_| Ok::<_, StorageError>(()))
                .map_err(|error| match error {
                    purrdf_core::OwnedTermFoldError::Visitor(error)
                    | purrdf_core::OwnedTermFoldError::Storage(error) => error,
                })?;
            let mut cause = memory.boxed(Self {
                reason,
                value,
                owned_bytes: 0,
            })?;
            cause.owned_bytes = memory
                .admitted_bytes()
                .checked_sub(baseline)
                .ok_or(StorageError::SizeOverflow)?;
            Ok(super::GeoError::CarrierInput(cause))
        })
    }
}

impl core::fmt::Display for CarrierInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match (self.reason, &self.value) {
            (CarrierInputReason::RadiusExpectedLiteral, _) => {
                f.write_str("a buffer radius must be a numeric literal")
            }
            (CarrierInputReason::RadiusDatatype, _) => {
                f.write_str("a buffer radius must have a numeric XSD datatype")
            }
            (CarrierInputReason::MissingParsedSource, _) => {
                f.write_str("native geometry argument has no original parsed source owner")
            }
            (CarrierInputReason::UnknownDatatype, TermValue::Literal { datatype, .. }) => write!(
                f,
                "<{datatype}> is not a geometry datatype; a geof: function takes a geo:wktLiteral or a geo:geoJSONLiteral, and an xsd:string carrying the same characters is text rather than a geometry"
            ),
            (CarrierInputReason::ExpectedLiteral | CarrierInputReason::UnknownDatatype, value) => {
                write!(
                    f,
                    "a geof: function takes a geometry literal and {} arrived; a geometry's datatype is what makes its lexical form a geometry rather than text, so purrdf-geo refuses rather than guessing a codec",
                    crate::carrier::term_description_display(value)
                )
            }
        }
    }
}
impl std::error::Error for CarrierInputError {}
