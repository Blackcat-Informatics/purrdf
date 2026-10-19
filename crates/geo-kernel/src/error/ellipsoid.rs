// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original ellipsoid inputs at their admitted numerical refusal.

/// Which exact ellipsoid input failed validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EllipsoidRequirement {
    /// The semimajor axis must exceed zero.
    PositiveSemimajor,
    /// Inverse flattening must exceed one.
    InverseFlatteningAboveOne,
}

impl EllipsoidRequirement {
    /// Original human diagnostic for this mathematical requirement.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::PositiveSemimajor => "semimajor axis must be positive",
            Self::InverseFlatteningAboveOne => "inverse flattening must exceed one",
        }
    }
}

impl purrdf_hash::error::StructuredError for EllipsoidRequirement {
    purrdf_hash::error_visit! {
        PositiveSemimajor => "ellipsoid-positive-semimajor" {},
        InverseFlatteningAboveOne => "ellipsoid-inverse-flattening-above-one" {},
    }
}

/// The original exact parameters refused by ellipsoid construction.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EllipsoidInputError {
    /// Original semimajor axis in metres.
    pub semimajor: crate::Rat,
    /// Original inverse flattening.
    pub inverse_flattening: crate::Rat,
    /// First mathematical requirement that failed.
    pub requirement: EllipsoidRequirement,
}

crate::context::retained::native_detached_fields!(EllipsoidInputError {
    native: [semimajor, inverse_flattening],
    copy: [requirement]
});

impl purrdf_hash::error::StructuredError for EllipsoidInputError {
    purrdf_hash::error_visit! {
        @record self { semimajor, inverse_flattening, requirement } => "ellipsoid-input" {
            "semimajor" => Record(&self.semimajor), "inverse_flattening" => Record(&self.inverse_flattening),
            "requirement" => Record(&self.requirement)
        }
    }
}

impl EllipsoidInputError {
    pub(crate) fn capture(
        semimajor: &crate::Rat,
        inverse_flattening: &crate::Rat,
        requirement: EllipsoidRequirement,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        use crate::context::NativeDetached as _;

        match crate::context::Retained::produce(
            progress,
            |memory| {
                memory.try_scope(|memory| {
                    Ok(Self {
                        semimajor: semimajor
                            .detached_with_memory(memory)
                            .map_err(super::GeoError::Storage)?,
                        inverse_flattening: inverse_flattening
                            .detached_with_memory(memory)
                            .map_err(super::GeoError::Storage)?,
                        requirement,
                    })
                })
            },
            |value, original| value.publish(original).map_err(super::GeoError::Storage),
        ) {
            Ok(cause) => super::GeoError::InvalidEllipsoid(cause),
            Err(error) => error,
        }
    }
}

impl core::fmt::Display for EllipsoidInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.requirement.message())
    }
}
impl std::error::Error for EllipsoidInputError {}
