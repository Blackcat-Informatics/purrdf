// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original normalization and monomial-construction refusals.

use crate::context::retained::NativeDetached;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

/// Original polynomial inputs at the first rejected construction requirement.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PolynomialInputError {
    /// The original normalization scale on one axis is not positive.
    Normalization {
        /// Original declared total degree.
        degree: u16,
        /// Original normalization origin.
        origin: [crate::Rat; 2],
        /// Original normalization scale.
        scale: [crate::Rat; 2],
        /// First failed zero-based coordinate axis.
        axis: usize,
        /// Original number of supplied terms.
        term_count: usize,
    },
    /// The first sorted term exceeds the declared total degree.
    Degree {
        /// Original declared total degree.
        degree: u16,
        /// Zero-based position in the original constructor's sorted table.
        index: usize,
        /// Original term count.
        term_count: usize,
        /// Original failing powers and exact coefficient pair.
        term: crate::operation::PolynomialTerm,
    },
    /// The first adjacent pair in the sorted table has identical powers.
    Duplicate {
        /// Original declared total degree.
        degree: u16,
        /// Zero-based position of the first term in the pair.
        index: usize,
        /// Original term count.
        term_count: usize,
        /// Original first term, including its coefficients.
        first: crate::operation::PolynomialTerm,
        /// Original second term, including its coefficients.
        second: crate::operation::PolynomialTerm,
    },
}
impl purrdf_hash::error::StructuredError for PolynomialInputError {
    purrdf_hash::error_visit! {
        Normalization { degree, origin, scale, axis, term_count } => "polynomial-normalization" {
            "degree" => Unsigned(u128::from(*degree)),
            "origin" => List(origin.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))),
            "scale" => List(scale.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))),
            "axis" => Unsigned(*axis as u128), "term_count" => Unsigned(*term_count as u128)
        },
        Degree { degree, index, term_count, term } => "polynomial-degree" {
            "degree" => Unsigned(u128::from(*degree)), "index" => Unsigned(*index as u128),
            "term_count" => Unsigned(*term_count as u128), "term" => Record(term)
        },
        Duplicate { degree, index, term_count, first, second } => "polynomial-duplicate" {
            "degree" => Unsigned(u128::from(*degree)), "index" => Unsigned(*index as u128),
            "term_count" => Unsigned(*term_count as u128), "first" => Record(first), "second" => Record(second)
        },
    }
}
impl PolynomialInputError {
    pub(crate) fn normalization(
        degree: u16,
        origin: [crate::Rat; 2],
        scale: [crate::Rat; 2],
        axis: usize,
        term_count: usize,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let [origin, scale] =
            match crate::context::retained::produce_persistent_numeric([origin, scale], progress) {
                Ok(value) => value.into_value(),
                Err(error) => return error,
            };
        progress.retained_error(|memory| {
            memory
                .boxed(Self::Normalization {
                    degree,
                    origin,
                    scale,
                    axis,
                    term_count,
                })
                .map(super::GeoError::PolynomialInput)
        })
    }

    /// Retain only the actually rejected terms inside the original sort factory.
    pub(crate) fn monomial_with_memory(
        degree: u16,
        index: usize,
        term_count: usize,
        first: &crate::operation::PolynomialTerm,
        second: Option<&crate::operation::PolynomialTerm>,
        memory: &mut Memory<'_, dyn Admission + '_>,
    ) -> Result<super::GeoError, StorageError> {
        let first = first.prepare(memory)?;
        let second = second.map(|value| value.prepare(memory)).transpose()?;
        let cause = match second {
            None => Self::Degree {
                degree,
                index,
                term_count,
                term: first,
            },
            Some(second) => Self::Duplicate {
                degree,
                index,
                term_count,
                first,
                second,
            },
        };
        memory.boxed(cause).map(super::GeoError::PolynomialInput)
    }

    pub(crate) fn native_owned_bytes(&self) -> Result<usize, StorageError> {
        let fields = match self {
            Self::Normalization {
                degree: _,
                origin,
                scale,
                axis: _,
                term_count: _,
            } => [
                Some(&origin[0]),
                Some(&origin[1]),
                Some(&scale[0]),
                Some(&scale[1]),
            ],
            Self::Degree {
                degree: _,
                index: _,
                term_count: _,
                term,
            } => [
                Some(&term.x_coefficient),
                Some(&term.y_coefficient),
                None,
                None,
            ],
            Self::Duplicate {
                degree: _,
                index: _,
                term_count: _,
                first,
                second,
            } => [
                Some(&first.x_coefficient),
                Some(&first.y_coefficient),
                Some(&second.x_coefficient),
                Some(&second.y_coefficient),
            ],
        };
        fields
            .into_iter()
            .flatten()
            .try_fold(size_of::<Self>(), |bytes, value| {
                value
                    .integer_operands()
                    .into_iter()
                    .try_fold(bytes, |bytes, integer| {
                        // Published immutable magnitudes retain their own original
                        // grants. Only raw detached buffers belong to this error box.
                        let owned = if integer.detached_required_bytes()? == 0 {
                            0
                        } else {
                            integer.allocated_bytes()
                        };
                        bytes.checked_add(owned).ok_or(StorageError::SizeOverflow)
                    })
            })
    }
}
impl core::fmt::Display for PolynomialInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Normalization { .. } => "polynomial normalization scales must be positive",
            Self::Degree { .. } => "polynomial term exceeds declared degree",
            Self::Duplicate { .. } => "duplicate polynomial monomial",
        })
    }
}
impl std::error::Error for PolynomialInputError {}
