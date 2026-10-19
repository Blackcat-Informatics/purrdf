// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native unsupported geographic operations and original carrier operands.

use super::GeoFailureCause;

/// Why a geographic operation has no implementation or exact representation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnsupportedRefusal {
    /// Genuine caller-supplied message data.
    Message(String),
    /// A recognized geometry datatype has no codec.
    Datatype {
        /// Original datatype identifier.
        datatype: String,
        /// Original vocabulary term recognized by the caller's vocabulary.
        term: crate::GeoTerm,
    },
    /// A scalar function has its original native dispatch refusal.
    Scalar(GeoFailureCause),
    /// An exact decimal carrier cannot represent this original rational.
    NonTerminatingDecimal(crate::context::Retained<crate::Rat>),
}

impl purrdf_hash::error::StructuredError for UnsupportedRefusal {
    purrdf_hash::error_visit! {
        Message(message) => "geo-unsupported-message" { "message" => Message(message) },
        Datatype { datatype, term } => "geo-unsupported-datatype" {
            "datatype" => Iri(datatype), "term" => Code(term.local_name())
        },
        Scalar(cause) => "geo-unsupported-scalar" { "cause" => Cause(cause) },
        NonTerminatingDecimal(value) => "geo-nonterminating-decimal" { "value" => Record(value) },
    }
}

impl UnsupportedRefusal {
    pub(crate) fn native_text_bytes(&self) -> usize {
        match self {
            Self::Message(message) => message.capacity(),
            Self::Datatype { datatype, term: _ } => datatype.capacity(),
            Self::Scalar(cause) => cause.native_text_bytes(),
            Self::NonTerminatingDecimal(_) => 0,
        }
    }
}

impl core::fmt::Display for UnsupportedRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Message(message) => f.write_str(message),
            Self::Datatype { datatype, term } => write!(
                f,
                "<{datatype}> is the geo:{} datatype, and purrdf-geo implements the WKT and GeoJSON codecs only; it refuses rather than reading the lexical form as a serialization it is not",
                term.local_name()
            ),
            Self::Scalar(cause) => core::fmt::Display::fmt(cause, f),
            Self::NonTerminatingDecimal(_) => {
                f.write_str("an exact decimal carrier cannot represent a nonterminating rational")
            }
        }
    }
}
purrdf_lex::error_source!(UnsupportedRefusal => { Self::Scalar(cause) => cause });
