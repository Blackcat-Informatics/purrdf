// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use ciborium::value::Value;

use crate::model::{AnnotationRow, Graph, ReifierRow, TermKind, Triple3};
use crate::reader::as_idx;

pub(crate) enum RowDecode<T> {
    Skip,
    Row(T),
    Damaged(&'static str),
    Position(String),
}

pub(crate) fn decode_reifier_row(row: &Value, graph: &Graph) -> RowDecode<ReifierRow> {
    let Value::Array(items) = row else {
        return RowDecode::Skip;
    };
    if !(4..=5).contains(&items.len()) {
        return RowDecode::Damaged("reifies row must have 4 or 5 term ids");
    }

    let (rid, s, p, o) = (
        as_idx(&items[0]),
        as_idx(&items[1]),
        as_idx(&items[2]),
        as_idx(&items[3]),
    );
    let has_graph = items.len() == 5;
    let graph_slot = if has_graph { as_idx(&items[4]) } else { None };
    let n = graph.terms.len();
    let ok = matches!((rid, s, p, o), (Some(rid), Some(s), Some(p), Some(o))
        if rid < n && s < n && p < n && o < n && graph_slot.is_none_or(|g| g < n));
    if !ok || (has_graph && graph_slot.is_none()) {
        return RowDecode::Damaged("reifies row has bad/out-of-range ids");
    }

    let (Some(rid), Some(s), Some(p), Some(o)) = (rid, s, p, o) else {
        return RowDecode::Skip;
    };
    let triple = (s, p, o);
    if let Err(detail) = check_statement_positions(graph, triple, graph_slot, "reifier row") {
        return RowDecode::Position(detail);
    }
    RowDecode::Row((rid, triple, graph_slot))
}

pub(crate) fn decode_annotation_row(row: &Value, graph: &Graph) -> RowDecode<AnnotationRow> {
    let Value::Array(items) = row else {
        return RowDecode::Skip;
    };
    if !(3..=4).contains(&items.len()) {
        return RowDecode::Damaged("annot row must have 3 or 4 term ids");
    }

    let (reifier, predicate, value) = (as_idx(&items[0]), as_idx(&items[1]), as_idx(&items[2]));
    let has_graph = items.len() == 4;
    let graph_slot = if has_graph { as_idx(&items[3]) } else { None };
    let n = graph.terms.len();
    let ok = matches!((reifier, predicate, value), (Some(r), Some(p), Some(v))
        if r < n && p < n && v < n && graph_slot.is_none_or(|g| g < n));
    if !ok || (has_graph && graph_slot.is_none()) {
        return RowDecode::Damaged("annot row has bad/out-of-range ids");
    }

    let (Some(reifier), Some(predicate), Some(value)) = (reifier, predicate, value) else {
        return RowDecode::Skip;
    };
    if graph.terms[predicate].kind != TermKind::Iri {
        return RowDecode::Position(format!("annot predicate {predicate} not an IRI"));
    }
    if let Some(graph_name) = graph_slot {
        if matches!(
            graph.terms[graph_name].kind,
            TermKind::Literal | TermKind::Triple
        ) {
            return RowDecode::Position(format!(
                "annot graph name {graph_name} is not an IRI or blank node"
            ));
        }
    }
    RowDecode::Row((reifier, predicate, value, graph_slot))
}

pub(crate) fn check_quad_positions(
    graph: &Graph,
    s: usize,
    p: usize,
    o: usize,
    graph_slot: Option<usize>,
) -> Result<(), String> {
    check_statement_positions(graph, (s, p, o), graph_slot, "quad")
}

fn check_statement_positions(
    graph: &Graph,
    (s, p, o): Triple3,
    graph_slot: Option<usize>,
    context: &str,
) -> Result<(), String> {
    let n = graph.terms.len();
    if !(s < n && p < n && o < n && graph_slot.is_none_or(|gv| gv < n)) {
        return Err(format!(
            "{context} ({s},{p},{o},{}) has out-of-range term ids",
            fmt_opt(graph_slot)
        ));
    }

    let mut ok = graph.terms[p].kind == TermKind::Iri;
    if graph.terms[s].kind == TermKind::Literal {
        ok = false;
    }
    if let Some(graph_name) = graph_slot {
        if matches!(
            graph.terms[graph_name].kind,
            TermKind::Literal | TermKind::Triple
        ) {
            ok = false;
        }
    }
    if ok {
        Ok(())
    } else {
        Err(format!(
            "{context} ({s},{p},{o},{}) violates positions",
            fmt_opt(graph_slot)
        ))
    }
}

fn fmt_opt(graph_slot: Option<usize>) -> String {
    match graph_slot {
        Some(value) => value.to_string(),
        None => "None".to_string(),
    }
}
