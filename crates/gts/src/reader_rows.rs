// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_lex::cbor::{DecodeError, Value, ValueRef};

use crate::model::{AnnotationRow, ReifierRow, TermKind, Triple3};
use crate::reader::as_i128;
use crate::term_table::TermsRef;

mod error;
pub(crate) use error::RowError;

/// Translate snapshot-local references without building another CBOR tree.
/// Ordinary frames use zero; a snapshot captures its base before adding terms.
#[derive(Clone, Copy)]
pub(crate) struct TermIdBase(pub(crate) usize);

impl TermIdBase {
    pub(crate) fn integer(self, value: &Value) -> Result<Option<i128>, RowError> {
        self.integer_value(as_i128(value))
    }
    fn integer_value(self, raw: Option<i128>) -> Result<Option<i128>, RowError> {
        let Some(local) = raw.and_then(|id| usize::try_from(id).ok()) else {
            // Keep malformed and negative operands for the existing semantic
            // checks and their diagnostics, rather than coercing them to IDs.
            return Ok(raw);
        };
        local
            .checked_add(self.0)
            .map(|id| Some(id as i128))
            .ok_or(RowError::TermIdBaseOverflow {
                value: local,
                base: self.0,
            })
    }

    pub(crate) fn row<const N: usize>(
        self,
        items: [Option<i128>; N],
    ) -> Result<[Option<i128>; N], RowError> {
        let mut ids = [None; N];
        for (id, value) in ids.iter_mut().zip(items) {
            *id = self.integer_value(value)?;
        }
        Ok(ids)
    }
}

pub(crate) enum RowDecode<T> {
    Skip,
    Row(T),
    Damaged(Box<RowError>),
    Position(Box<RowError>),
}

pub(crate) fn decode_reifier_row(
    row: Option<NumericRow<5>>,
    terms: TermsRef<'_>,
    base: TermIdBase,
) -> RowDecode<ReifierRow> {
    let Some(row) = row else {
        return RowDecode::Skip;
    };
    if !(4..=5).contains(&row.width) {
        return RowDecode::Damaged(Box::new(RowError::Width {
            context: "reifies",
            actual: row.width,
            minimum: 4,
            maximum: 5,
        }));
    }

    let raw = match base.row::<5>(row.values) {
        Ok(raw) => raw,
        Err(error) => return RowDecode::Damaged(Box::new(error)),
    };
    let [rid, s, p, o, graph_slot] = raw.map(|value| value.and_then(|id| usize::try_from(id).ok()));
    let has_graph = row.width == 5;
    let n = terms.len();
    let ok = matches!((rid, s, p, o), (Some(rid), Some(s), Some(p), Some(o))
        if rid < n && s < n && p < n && o < n && graph_slot.is_none_or(|g| g < n));
    if !ok || (has_graph && graph_slot.is_none()) {
        return RowDecode::Damaged(Box::new(RowError::Ids {
            context: "reifies",
            values: raw[..row.width].to_vec(),
            terms: n,
        }));
    }

    let (Some(rid), Some(s), Some(p), Some(o)) = (rid, s, p, o) else {
        return RowDecode::Skip;
    };
    let triple = (s, p, o);
    if let Err(detail) = check_statement_positions(terms, triple, graph_slot, "reifier row") {
        return RowDecode::Position(detail);
    }
    RowDecode::Row((rid, triple, graph_slot))
}

pub(crate) fn decode_annotation_row(
    row: Option<NumericRow<4>>,
    terms: TermsRef<'_>,
    base: TermIdBase,
) -> RowDecode<AnnotationRow> {
    let Some(row) = row else {
        return RowDecode::Skip;
    };
    if !(3..=4).contains(&row.width) {
        return RowDecode::Damaged(Box::new(RowError::Width {
            context: "annot",
            actual: row.width,
            minimum: 3,
            maximum: 4,
        }));
    }

    let raw = match base.row::<4>(row.values) {
        Ok(raw) => raw,
        Err(error) => return RowDecode::Damaged(Box::new(error)),
    };
    let [reifier, predicate, value, graph_slot] =
        raw.map(|value| value.and_then(|id| usize::try_from(id).ok()));
    let has_graph = row.width == 4;
    let n = terms.len();
    let ok = matches!((reifier, predicate, value), (Some(r), Some(p), Some(v))
        if r < n && p < n && v < n && graph_slot.is_none_or(|g| g < n));
    if !ok || (has_graph && graph_slot.is_none()) {
        return RowDecode::Damaged(Box::new(RowError::Ids {
            context: "annot",
            values: raw[..row.width].to_vec(),
            terms: n,
        }));
    }

    let (Some(reifier), Some(predicate), Some(value)) = (reifier, predicate, value) else {
        return RowDecode::Skip;
    };
    if terms[predicate].kind != TermKind::Iri {
        return RowDecode::Position(Box::new(RowError::AnnotationPredicate {
            id: predicate,
            actual: terms[predicate].kind,
        }));
    }
    if let Some(graph_name) = graph_slot
        && matches!(terms[graph_name].kind, TermKind::Literal | TermKind::Triple)
    {
        return RowDecode::Position(Box::new(RowError::AnnotationGraph {
            id: graph_name,
            actual: terms[graph_name].kind,
        }));
    }
    RowDecode::Row((reifier, predicate, value, graph_slot))
}

pub(crate) fn check_quad_positions(
    terms: TermsRef<'_>,
    s: usize,
    p: usize,
    o: usize,
    graph_slot: Option<usize>,
) -> Result<(), Box<RowError>> {
    check_statement_positions(terms, (s, p, o), graph_slot, "quad")
}

fn check_statement_positions(
    terms: TermsRef<'_>,
    (s, p, o): Triple3,
    graph_slot: Option<usize>,
    context: &'static str,
) -> Result<(), Box<RowError>> {
    let n = terms.len();
    if !(s < n && p < n && o < n && graph_slot.is_none_or(|gv| gv < n)) {
        return Err(Box::new(RowError::Bounds {
            context,
            s,
            p,
            o,
            graph: graph_slot,
            terms: n,
        }));
    }

    // A well-formed quad: predicate is an IRI, subject is not a literal, and any
    // graph name is neither a literal nor a quoted triple.
    let ok = terms[p].kind == TermKind::Iri
        && terms[s].kind != TermKind::Literal
        && graph_slot.is_none_or(|graph_name| {
            !matches!(terms[graph_name].kind, TermKind::Literal | TermKind::Triple)
        });
    if ok {
        Ok(())
    } else {
        Err(Box::new(RowError::Positions {
            context,
            s,
            p,
            o,
            graph: graph_slot,
            subject_kind: terms[s].kind,
            predicate_kind: terms[p].kind,
            graph_kind: graph_slot.map(|id| terms[id].kind),
        }))
    }
}

fn fmt_opt(graph_slot: Option<usize>) -> String {
    match graph_slot {
        Some(value) => value.to_string(),
        None => "None".to_string(),
    }
}

pub(crate) struct NumericRow<const N: usize> {
    pub(crate) width: usize,
    pub(crate) values: [Option<i128>; N],
}
pub(crate) fn numeric_row<const N: usize>(
    value: ValueRef<'_>,
) -> Result<Option<NumericRow<N>>, DecodeError> {
    let Some(items) = value.array()? else {
        return Ok(None);
    };
    let width = items.len();
    let mut values = [None; N];
    for (slot, item) in values.iter_mut().zip(items) {
        *slot = item?.integer();
    }
    Ok(Some(NumericRow { width, values }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_row_decode_layout_is_unchanged() {
        #[expect(
            dead_code,
            reason = "the former representation is a frozen layout witness only"
        )]
        enum Previous<T> {
            Skip,
            Row(T),
            Damaged(&'static str),
            Position(String),
        }
        assert_eq!(
            size_of::<RowDecode<ReifierRow>>(),
            size_of::<Previous<ReifierRow>>()
        );
        assert_eq!(
            size_of::<RowDecode<AnnotationRow>>(),
            size_of::<Previous<AnnotationRow>>()
        );
    }
}
