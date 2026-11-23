// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;

use super::fields::{Fields, push};
use super::terms::read_enum;
use super::{
    ContentError, FrameCheck, FrameEnvelope, FrameField, FrameType, LinkState, MalformedReason,
    PayloadState, WireError,
};
use crate::FastSet;

/// Ordering claimed by a row frame with graph runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowOrder {
    /// The rows claim canonical order.
    Canonical,
    /// The frame makes no ordering claim.
    Unordered,
}

/// A decoded row of term IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Row {
    /// Subject, predicate and object IDs, with an optional named graph ID.
    Quad([u64; 3], Option<u64>),
    /// Reifier and triple term IDs, with an optional named graph ID.
    ReifierBinding([u64; 2], Option<u64>),
    /// Reifier, predicate and object IDs, multiplicity and optional graph ID.
    Annotation([u64; 3], NonZeroU64, Option<u64>),
    /// The ID of an explicitly declared empty named graph.
    EmptyGraph(u64),
}

/// Rows obtained from an admitted column payload.
#[derive(Debug)]
#[non_exhaustive]
pub struct DecodedRows {
    /// The claimed order when the frame carries one.
    pub order: Option<RowOrder>,
    /// A canonical declaration conflicts with the rows or their graph runs.
    pub declared_order_violated: bool,
    /// Rows in wire order, with term IDs awaiting resolution.
    pub rows: Vec<Row>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct OrderKey(Option<u64>, [u64; 3]);

impl Row {
    pub(super) const fn order_key(self) -> OrderKey {
        match self {
            Self::Quad([s, p, o], graph) => OrderKey(graph, [p, o, s]),
            Self::ReifierBinding([reifier, triple], graph) => OrderKey(graph, [reifier, triple, 0]),
            Self::Annotation(terms, _, graph) => OrderKey(graph, terms),
            Self::EmptyGraph(graph) => OrderKey(Some(graph), [0; 3]),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Quad,
    ReifierBinding,
    Annotation,
    EmptyGraph,
}

impl Kind {
    const fn deltas(self) -> usize {
        match self {
            Self::Quad | Self::ReifierBinding => 2,
            Self::Annotation => 4,
            Self::EmptyGraph => 1,
        }
    }
}

/// The admitted content record of a row frame.
#[derive(Clone, Copy, Debug)]
pub struct RowHeader {
    kind: Kind,
    registry: NonZeroU64,
    /// Declared order when the frame carries one.
    pub order: Option<RowOrder>,
    /// Declared row count.
    pub count: NonZeroU64,
}

impl RowHeader {
    pub(super) fn inspect(
        envelope: &FrameEnvelope<'_>,
        frame: &FrameCheck<'_>,
        registry: NonZeroU64,
    ) -> Option<Result<Self, ContentError>> {
        if frame.malformed.is_some()
            || frame.link.state == LinkState::Broken
            || !matches!(frame.payload.state, PayloadState::Verified(_))
        {
            return None;
        }
        let kind = match frame.frame_type.and_then(FrameType::from_code)? {
            FrameType::Quads => Kind::Quad,
            FrameType::ReifierBindings => Kind::ReifierBinding,
            FrameType::Annotations => Kind::Annotation,
            FrameType::EmptyGraphs => Kind::EmptyGraph,
            _ => return None,
        };
        Some(Self::read(
            kind,
            &mut envelope.read_field(FrameField::Content),
            registry,
        ))
    }

    fn read(
        kind: Kind,
        fields: &mut Fields<'_>,
        registry: NonZeroU64,
    ) -> Result<Self, ContentError> {
        let order = if kind == Kind::EmptyGraph {
            fields.record(1)?;
            None
        } else {
            fields.record(2)?;
            Some(if read_enum(fields, 2, registry)? == 1 {
                RowOrder::Canonical
            } else {
                RowOrder::Unordered
            })
        };
        let count = fields.positive(MalformedReason::OutOfRange)?;
        Ok(Self {
            kind,
            registry,
            order,
            count,
        })
    }

    /// Decode the graph runs and columns of the verified payload.
    ///
    /// # Errors
    /// Returns the first payload grammar or count violation or a host
    /// reservation failure.
    pub fn decode(self, payload: &[u8]) -> Result<DecodedRows, ContentError> {
        let mut fields = Fields::new(payload);
        let graph_column = self.kind != Kind::EmptyGraph;
        let predicate_runs = self.kind == Kind::Quad;
        let delta_columns = self.kind.deltas();
        fields.record(
            (delta_columns + usize::from(graph_column) + usize::from(predicate_runs)) as u64,
        )?;
        let graphs = graph_column
            .then(|| Runs::read(&mut fields, |reader| graph_name(reader, self.registry)))
            .transpose()?;
        let predicates = predicate_runs
            .then(|| Runs::read(&mut fields, |reader| Ok(reader.uint()?)))
            .transpose()?;
        let mut columns = Vec::new();
        columns.try_reserve_exact(delta_columns)?;
        for index in 0..delta_columns {
            columns.push(Column::read(
                &mut fields,
                self.kind == Kind::Annotation && index == 3,
            )?);
        }
        fields.finish()?;
        let count = self.count.get();
        if let Some(graphs) = &graphs {
            graphs.check(count)?;
        }
        for column in &columns {
            check_count(column.values.len() as u128, count, column.offset)?;
        }
        if let Some(predicates) = &predicates {
            predicates.check(count)?;
        }
        let rows = self.materialize(graphs, predicates, &columns)?;
        if self.kind == Kind::Annotation {
            let mut keys = FastSet::default();
            keys.try_reserve(rows.len())?;
            if rows.iter().any(|row| !keys.insert(row.order_key())) {
                return Err(WireError::new(MalformedReason::UnorderedOrRepeated, 0).into());
            }
        }
        let declared_order_violated = self.order == Some(RowOrder::Canonical)
            && rows
                .windows(2)
                .any(|pair| pair[0].order_key() >= pair[1].order_key());
        Ok(DecodedRows {
            order: self.order,
            declared_order_violated,
            rows,
        })
    }

    fn materialize(
        self,
        graphs: Option<Runs<Option<u64>>>,
        predicates: Option<Runs<u64>>,
        columns: &[Column],
    ) -> Result<Vec<Row>, ContentError> {
        let mut graphs = graphs.map(Runs::values);
        let mut predicates = predicates.map(Runs::values);
        let count = columns[0].values.len();
        let mut rows = Vec::new();
        rows.try_reserve_exact(count)?;
        for (index, primary) in columns[0].values.iter().copied().enumerate() {
            let value = |column: usize| columns[column].values[index];
            let graph = graphs
                .as_mut()
                .and_then(|graphs| graphs.next().expect("admitted graph runs"));
            let row = match self.kind {
                Kind::Quad => {
                    let predicate = predicates
                        .as_mut()
                        .and_then(Iterator::next)
                        .expect("admitted predicate runs");
                    Row::Quad([value(1), predicate, primary], graph)
                }
                Kind::ReifierBinding => Row::ReifierBinding([primary, value(1)], graph),
                Kind::Annotation => Row::Annotation(
                    [primary, value(1), value(2)],
                    NonZeroU64::new(value(3)).expect("admitted annotation multiplicity"),
                    graph,
                ),
                Kind::EmptyGraph => Row::EmptyGraph(primary),
            };
            rows.push(row);
        }
        Ok(rows)
    }
}

struct Column {
    offset: usize,
    values: Vec<u64>,
}

impl Column {
    fn read(fields: &mut Fields<'_>, positive: bool) -> Result<Self, ContentError> {
        let offset = fields.offset();
        let count = fields.array()?;
        let mut value = 0_u64;
        let mut values = Vec::new();
        for _ in 0..count {
            let value_offset = fields.offset();
            let next = i128::from(value) + fields.int()?;
            let error = WireError::new(MalformedReason::OutOfRange, value_offset);
            value = u64::try_from(next).map_err(|_| error)?;
            if positive && value == 0 {
                return Err(error.into());
            }
            push(&mut values, value)?;
        }
        Ok(Self { offset, values })
    }
}

struct Runs<T> {
    offset: usize,
    entries: Vec<(T, NonZeroU64)>,
    total: u128,
}

impl<T: Copy + PartialEq> Runs<T> {
    fn read(
        fields: &mut Fields<'_>,
        mut value: impl FnMut(&mut Fields<'_>) -> Result<T, ContentError>,
    ) -> Result<Self, ContentError> {
        let offset = fields.offset();
        let count = fields.array()?;
        let mut runs = Vec::new();
        let mut total = 0;
        for _ in 0..count {
            fields.record(2)?;
            let start = fields.offset();
            let current = value(fields)?;
            if runs
                .last()
                .is_some_and(|(previous, _)| *previous == current)
            {
                return Err(WireError::new(MalformedReason::NotDeterministic, start).into());
            }
            let count = fields.positive(MalformedReason::OutOfRange)?;
            total += u128::from(count.get());
            push(&mut runs, (current, count))?;
        }
        Ok(Self {
            offset,
            entries: runs,
            total,
        })
    }

    fn check(&self, count: u64) -> Result<(), ContentError> {
        check_count(self.total, count, self.offset)
    }

    fn values(self) -> impl Iterator<Item = T> {
        self.entries.into_iter().flat_map(|(value, count)| {
            // The total run count agrees with an addressable delta column.
            core::iter::repeat_n(
                value,
                usize::try_from(count.get()).expect("count fits its column"),
            )
        })
    }
}

fn check_count(actual: u128, expected: u64, offset: usize) -> Result<(), ContentError> {
    if actual != u128::from(expected) {
        return Err(WireError::new(MalformedReason::InconsistentCounts, offset).into());
    }
    Ok(())
}

pub(super) fn graph_name(
    fields: &mut Fields<'_>,
    registry: NonZeroU64,
) -> Result<Option<u64>, ContentError> {
    let offset = fields.offset();
    let length = fields.array()?;
    if length == 0 {
        return Err(WireError::new(MalformedReason::WrongLength, offset).into());
    }
    match (read_enum(fields, 2, registry)?, length) {
        (1, 1) => Ok(None),
        (2, 2) => Ok(Some(fields.uint()?)),
        _ => Err(WireError::new(MalformedReason::WrongLength, offset).into()),
    }
}
