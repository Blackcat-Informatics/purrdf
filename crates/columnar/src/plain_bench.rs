// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bench-only access to the PLAIN `INT64` value codec, so the criterion bench
//! can measure the value copy on its own rather than inside a whole five-table
//! write. Not public API: hidden from the docs, unstable, and no shipping path
//! calls it.

use crate::ColumnarError;
use crate::column::{Int64Column, Presence};
use crate::parquet::{decode_int64_plain, encode_int64_plain};

/// A nullable `INT64` column in the codec's own layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainInt64(Int64Column);

/// A column's definition levels, decoded and counted, as a page decode holds
/// them when it reaches the value body.
#[derive(Debug, Clone)]
pub struct PlainPresence {
    presence: Presence,
    present: usize,
}

impl PlainInt64 {
    /// The column holding `rows`, a `None` row null.
    pub fn from_rows(rows: impl IntoIterator<Item = Option<i64>>) -> Self {
        Self(rows.into_iter().collect())
    }

    /// The number of rows.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the column has no rows.
    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    /// This column's definition levels, for [`PlainInt64::decode`].
    pub fn presence(&self) -> PlainPresence {
        let presence = self.0.presence().clone();
        let present = presence.count();
        PlainPresence { presence, present }
    }

    /// The PLAIN value body: the present values, little-endian, in row order.
    pub fn encode(&self) -> Vec<u8> {
        encode_int64_plain(self.0.values())
    }

    /// The column a PLAIN value body holds under `presence`.
    pub fn decode(body: &[u8], presence: PlainPresence) -> Result<Self, ColumnarError> {
        decode_int64_plain(body, presence.presence, presence.present).map(Self)
    }
}
