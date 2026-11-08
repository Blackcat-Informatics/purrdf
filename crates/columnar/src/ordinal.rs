// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Requested original table-row evidence from a complete strict read.

use purrdf_core::{QuadIds, TermId};

use crate::{ColumnarError, ColumnarRead, Table};

/// One original zero-based row in a Columnar RDF table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColumnarOrdinal {
    /// Canonical dictionary row, including row zero.
    Term(u64),
    /// Base statement row in `quads.parquet`.
    Quad(u64),
    /// Binding row in `reifiers.parquet`.
    Reifier(u64),
    /// Metadata row in `annotations.parquet`.
    Annotation(u64),
}

impl ColumnarOrdinal {
    /// The original physical table.
    #[must_use]
    pub const fn table(self) -> Table {
        match self {
            Self::Term(_) => Table::Terms,
            Self::Quad(_) => Table::Quads,
            Self::Reifier(_) => Table::Reifiers,
            Self::Annotation(_) => Table::Annotations,
        }
    }

    /// The original row index within that table.
    #[must_use]
    pub const fn index(self) -> u64 {
        match self {
            Self::Term(index)
            | Self::Quad(index)
            | Self::Reifier(index)
            | Self::Annotation(index) => index,
        }
    }
}

/// Original row fields translated by the strict reader's dictionary-to-kernel map.
/// The IDs belong only to the dataset held by the accompanying read evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnarOrdinalValue {
    /// One original dictionary value.
    Term(TermId),
    /// Original subject, predicate, object and graph fields.
    Quad(QuadIds<TermId>),
    /// Original resource, reified triple and graph fields.
    Reifier {
        /// The reifier resource.
        reifier: TermId,
        /// The triple term named by the original row's three statement fields.
        triple: TermId,
        /// The binding's graph; absence denotes the default graph.
        graph: Option<TermId>,
    },
    /// Original reifier, predicate, value and graph fields.
    Annotation(QuadIds<TermId>),
}

/// Requested provenance bound to the actual dataset of a complete strict read.
/// No public constructor accepts caller-assembled rows or a replacement dataset.
#[derive(Debug, Clone)]
pub struct ColumnarOrdinalRead {
    read: ColumnarRead,
    values: Vec<(ColumnarOrdinal, ColumnarOrdinalValue)>,
}

impl ColumnarOrdinalRead {
    /// The complete original read, borrowed without allowing dataset replacement.
    #[must_use]
    pub const fn read(&self) -> &ColumnarRead {
        &self.read
    }

    /// Evidence for a requested row; unrequested rows are not retained.
    #[must_use]
    pub fn get(&self, ordinal: ColumnarOrdinal) -> Option<ColumnarOrdinalValue> {
        self.values
            .binary_search_by_key(&ordinal, |(key, _)| *key)
            .ok()
            .map(|index| self.values[index].1)
    }

    pub(crate) fn into_read(self) -> ColumnarRead {
        self.read
    }
}

pub(crate) struct OrdinalCapture {
    values: Vec<(ColumnarOrdinal, Option<ColumnarOrdinalValue>)>,
    rows: [u64; 4],
}

impl OrdinalCapture {
    pub(crate) fn new(requested: &[ColumnarOrdinal]) -> Self {
        let mut requested = requested.to_vec();
        requested.sort_unstable();
        requested.dedup();
        Self {
            values: requested.into_iter().map(|key| (key, None)).collect(),
            rows: [0; 4],
        }
    }

    pub(crate) fn row_counts(&mut self, counts: [usize; 4]) {
        self.rows = counts.map(|count| count as u64);
    }

    pub(crate) fn capture(&mut self, ordinal: ColumnarOrdinal, value: ColumnarOrdinalValue) {
        if let Ok(index) = self.values.binary_search_by_key(&ordinal, |(key, _)| *key) {
            self.values[index].1 = Some(value);
        }
    }

    pub(crate) fn finish(self, read: ColumnarRead) -> Result<ColumnarOrdinalRead, ColumnarError> {
        let mut values = Vec::with_capacity(self.values.len());
        for (ordinal, value) in self.values {
            let Some(value) = value else {
                let index = match ordinal {
                    ColumnarOrdinal::Term(_) => 0,
                    ColumnarOrdinal::Quad(_) => 1,
                    ColumnarOrdinal::Reifier(_) => 2,
                    ColumnarOrdinal::Annotation(_) => 3,
                };
                return Err(ColumnarError::OrdinalOutOfRange {
                    ordinal,
                    rows: self.rows[index],
                });
            };
            values.push((ordinal, value));
        }
        Ok(ColumnarOrdinalRead { read, values })
    }
}
