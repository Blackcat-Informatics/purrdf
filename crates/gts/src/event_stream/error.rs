// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native refusals at the GTS-to-event protocol boundary.

use crate::model::Diagnostic;
use purrdf_events::EventTermId;
use purrdf_hash::error::ErrorValue;

#[derive(Debug)]
pub(super) enum EventRefusal {
    Datatype {
        id: EventTermId,
    },
    Direction {
        token: String,
    },
    DirectionLanguage {
        direction: String,
        language: Option<String>,
    },
    GraphScoped {
        operation: &'static str,
        segment: u64,
        reifier: EventTermId,
        subject: Option<EventTermId>,
        predicate: EventTermId,
        object: EventTermId,
        graph: EventTermId,
    },
    Dangling {
        segment: u64,
        term: usize,
        role: String,
    },
    Nesting {
        segment: u64,
        term: usize,
    },
    UnboundTriple {
        segment: u64,
        term: usize,
    },
    MissingReifier {
        segment: u64,
        reifier: usize,
    },
    Reader(Diagnostic),
}

impl core::fmt::Display for EventRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Datatype { .. } => f.write_str("GTS literal datatype must resolve to an IRI"),
            Self::Direction { token } => {
                write!(f, "unrecognized GTS literal base direction {token:?}")
            }
            Self::DirectionLanguage { .. } => {
                f.write_str("an RDF 1.2 literal base direction requires a non-empty language tag")
            }
            Self::GraphScoped { operation, .. } => write!(
                f,
                "graph-scoped {operation} cannot be represented on the RdfEventSink protocol (no graph slot)"
            ),
            Self::Dangling {
                segment,
                term,
                role,
            } => write!(
                f,
                "GTS dangling term reference: segment {segment} {role} names undeclared term id {term}"
            ),
            Self::Nesting { segment, term } => write!(
                f,
                "GTS quoted-triple nesting limit exceeded resolving segment {segment} term id {term}"
            ),
            Self::UnboundTriple { segment, term } => write!(
                f,
                "GTS triple term has no reifier binding: segment {segment} term id {term}"
            ),
            Self::MissingReifier { segment, reifier } => write!(
                f,
                "GTS triple term references missing reifier {reifier} in segment {segment}"
            ),
            Self::Reader(cause) => write!(
                f,
                "GTS dangling term reference ({}): {}",
                cause.code, cause.detail
            ),
        }
    }
}

purrdf_lex::error_source!(EventRefusal { Reader });

impl purrdf_hash::error::StructuredError for EventRefusal {
    purrdf_hash::error_visit! {
        Datatype { id } => "gts-event-datatype" { "id" => Unsigned(u128::from(id.0)) },
        Direction { token } => "gts-event-direction" { "token" => Text(token) },
        DirectionLanguage { direction, language } => "gts-event-direction-language" { "direction" => Text(direction), "language" => Value(language.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)) },
        GraphScoped { operation, segment, reifier, subject, predicate, object, graph } => "gts-event-graph-scoped" {
            "operation" => Code(operation), "segment" => Unsigned(u128::from(*segment)),
            "reifier" => Unsigned(u128::from(reifier.0)),
            "subject" => Value(subject.map_or(ErrorValue::Absent, |id| ErrorValue::Unsigned(u128::from(id.0)))),
            "predicate" => Unsigned(u128::from(predicate.0)), "object" => Unsigned(u128::from(object.0)),
            "graph" => Unsigned(u128::from(graph.0))
        },
        Dangling { segment, term, role } => "gts-event-dangling" { "segment" => Unsigned(u128::from(*segment)), "term" => Unsigned(*term as u128), "role" => Text(role) },
        Nesting { segment, term } => "gts-event-nesting" { "segment" => Unsigned(u128::from(*segment)), "term" => Unsigned(*term as u128), "limit" => Unsigned(purrdf_events::MAX_TERM_NESTING_DEPTH as u128) },
        UnboundTriple { segment, term } => "gts-event-unbound-triple" { "segment" => Unsigned(u128::from(*segment)), "term" => Unsigned(*term as u128) },
        MissingReifier { segment, reifier } => "gts-event-missing-reifier" { "segment" => Unsigned(u128::from(*segment)), "reifier" => Unsigned(*reifier as u128) },
        Reader(cause) => "gts-event-reader" { "cause" => Cause(cause) },
    }
}
