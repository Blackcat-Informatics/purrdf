// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original row-width, term-ID and RDF-position failures.

use crate::model::TermKind;
use core::fmt;
use purrdf_hash::error::ErrorValue;

#[derive(Debug)]
pub(crate) enum RowError {
    Width {
        context: &'static str,
        actual: usize,
        minimum: usize,
        maximum: usize,
    },
    Ids {
        context: &'static str,
        values: Vec<Option<i128>>,
        terms: usize,
    },
    QuadIds {
        values: Vec<Option<i128>>,
        width: usize,
    },
    Bounds {
        context: &'static str,
        s: usize,
        p: usize,
        o: usize,
        graph: Option<usize>,
        terms: usize,
    },
    Positions {
        context: &'static str,
        s: usize,
        p: usize,
        o: usize,
        graph: Option<usize>,
        subject_kind: TermKind,
        predicate_kind: TermKind,
        graph_kind: Option<TermKind>,
    },
    AnnotationPredicate {
        id: usize,
        actual: TermKind,
    },
    AnnotationGraph {
        id: usize,
        actual: TermKind,
    },
}

impl fmt::Display for RowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Width {
                context,
                minimum,
                maximum,
                ..
            } => write!(f, "{context} row must have {minimum} or {maximum} term ids"),
            Self::Ids { context, .. } => write!(f, "{context} row has bad/out-of-range ids"),
            Self::QuadIds { .. } => f.write_str("quad has non-integer term ids"),
            Self::Bounds {
                context,
                s,
                p,
                o,
                graph,
                ..
            } => write!(
                f,
                "{context} ({s},{p},{o},{}) has out-of-range term ids",
                super::fmt_opt(*graph)
            ),
            Self::Positions {
                context,
                s,
                p,
                o,
                graph,
                ..
            } => write!(
                f,
                "{context} ({s},{p},{o},{}) violates positions",
                super::fmt_opt(*graph)
            ),
            Self::AnnotationPredicate { id, .. } => write!(f, "annot predicate {id} not an IRI"),
            Self::AnnotationGraph { id, .. } => {
                write!(f, "annot graph name {id} is not an IRI or blank node")
            }
        }
    }
}

impl std::error::Error for RowError {}

impl purrdf_hash::error::StructuredError for RowError {
    purrdf_hash::error_visit! {
        Width { context, actual, minimum, maximum } => "gts-row-width" {
            "context" => Code(context), "actual" => Unsigned(*actual as u128), "minimum" => Unsigned(*minimum as u128), "maximum" => Unsigned(*maximum as u128)
        },
        Ids { context, values, terms } => "gts-row-ids" {
            "context" => Code(context), "values" => List(values.iter().map(|value| value.map_or(ErrorValue::Absent, ErrorValue::Signed))), "terms" => Unsigned(*terms as u128)
        },
        QuadIds { values, width } => "gts-quad-noninteger-ids" {
            "values" => List(values.iter().map(|value| value.map_or(ErrorValue::Absent, ErrorValue::Signed))), "width" => Unsigned(*width as u128)
        },
        Bounds { context, s, p, o, graph, terms } => "gts-row-bounds" {
            "context" => Code(context), "subject" => Unsigned(*s as u128), "predicate" => Unsigned(*p as u128), "object" => Unsigned(*o as u128),
            "graph" => Value(graph.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(value as u128))), "terms" => Unsigned(*terms as u128)
        },
        Positions { context, s, p, o, graph, subject_kind, predicate_kind, graph_kind } => "gts-row-positions" {
            "context" => Code(context), "subject" => Unsigned(*s as u128), "predicate" => Unsigned(*p as u128), "object" => Unsigned(*o as u128),
            "graph" => Value(graph.map_or(ErrorValue::Absent, |value| ErrorValue::Unsigned(value as u128))),
            "subject_kind" => Record(subject_kind), "predicate_kind" => Record(predicate_kind),
            "graph_kind" => Value(graph_kind.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value)))
        },
        AnnotationPredicate { id, actual } => "gts-annotation-predicate-kind" { "id" => Unsigned(*id as u128), "actual" => Record(actual) },
        AnnotationGraph { id, actual } => "gts-annotation-graph-kind" { "id" => Unsigned(*id as u128), "actual" => Record(actual) },
    }
}
