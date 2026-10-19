// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native dataset failures crossing the event protocol.

#[derive(Debug)]
pub(super) enum IngestError {
    Freeze(crate::RdfDiagnostic),
    Datatype { literal: u64, datatype: usize },
    Reifier { reifier: usize, triple: usize },
}

impl core::fmt::Display for IngestError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Freeze(cause) => write!(f, "freeze failed: {cause}"),
            Self::Datatype { .. } => f.write_str("literal datatype did not resolve to an IRI"),
            Self::Reifier { .. } => f.write_str("reifier did not bind a triple term"),
        }
    }
}

purrdf_lex::error_source!(IngestError { Freeze });

impl purrdf_hash::error::StructuredError for IngestError {
    purrdf_hash::error_visit! {
        Freeze(cause) => "event-ingest-freeze" { "cause" => Cause(cause) },
        Datatype { literal, datatype } => "event-ingest-datatype" { "literal" => Unsigned(u128::from(*literal)), "datatype" => Unsigned(*datatype as u128) },
        Reifier { reifier, triple } => "event-ingest-reifier" { "reifier" => Unsigned(*reifier as u128), "triple" => Unsigned(*triple as u128) },
    }
}
