// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original failures of a host's explicit SHACL law selection.

use super::{UnsupportedProfile, XPathProfileConflict};

/// A requested profile's original unsupported identifier or regex-law conflict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileRequestError(Box<Failure>);

#[derive(Clone, Debug, PartialEq, Eq)]
enum Failure {
    Unsupported(UnsupportedProfile),
    Conflict(XPathProfileConflict),
}

impl From<UnsupportedProfile> for ProfileRequestError {
    fn from(cause: UnsupportedProfile) -> Self {
        Self(Box::new(Failure::Unsupported(cause)))
    }
}
impl From<XPathProfileConflict> for ProfileRequestError {
    fn from(cause: XPathProfileConflict) -> Self {
        Self(Box::new(Failure::Conflict(cause)))
    }
}
impl std::fmt::Display for ProfileRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.as_ref() {
            Failure::Unsupported(cause) => write!(f, "unsupported-profile: {cause}"),
            Failure::Conflict(cause) => cause.fmt(f),
        }
    }
}
impl std::error::Error for ProfileRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self.0.as_ref() {
            Failure::Unsupported(cause) => cause,
            Failure::Conflict(cause) => cause,
        })
    }
}
impl purrdf_hash::error::StructuredError for ProfileRequestError {
    fn visit<'e>(&'e self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'e>) {
        use purrdf_hash::error::ErrorValue;
        match self.0.as_ref() {
            Failure::Unsupported(cause) => {
                visitor.variant("shacl-profile-request-unsupported");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Failure::Conflict(cause) => {
                visitor.variant("shacl-profile-request-conflict");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
        }
    }
}
