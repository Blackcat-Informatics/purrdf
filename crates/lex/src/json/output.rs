// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Checked storage and traversal bounds before JSON records are constructed.

use super::Value;

/// A conservative complete compact-output layout, independent of field values.
/// It includes owned record strings, geometrically grown member/element vectors,
/// simultaneous compact output and writer frames. Callers derive text bounds
/// before formatting numerical values and refuse `None` before allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputLayout {
    heap: u64,
    compact: u64,
    work: u64,
    depth: u64,
}

impl OutputLayout {
    /// A scalar requiring no owned string or nested allocation.
    #[must_use]
    pub const fn scalar() -> Self {
        Self {
            heap: 0,
            compact: 5,
            work: 1,
            depth: 0,
        }
    }

    /// A record with scalar values. `text_bytes` bounds the complete lengths of
    /// member names, number lexemes and string payloads together. A UTF-8 byte
    /// requires at most six compact spelling bytes (`\u00xx`).
    #[must_use]
    pub fn record(members: u64, text_bytes: u64) -> Option<Self> {
        let slots = members.checked_mul(2)?.max(4);
        let heap = slots
            .checked_mul(size_of::<(String, Value)>() as u64)?
            .checked_add(text_bytes.checked_mul(2)?)?
            .checked_add(members.checked_mul(16)?)?;
        let compact = text_bytes
            .checked_mul(6)?
            .checked_add(members.checked_mul(12)?)?
            .checked_add(2)?;
        Some(Self {
            heap,
            compact,
            work: compact.checked_add(members.checked_mul(4)?)?,
            depth: 1,
        })
    }

    /// A repeated element layout in an owned array, admitted before collection.
    #[must_use]
    pub fn array(count: u64, element: Self) -> Option<Self> {
        let slots = count.checked_mul(2)?.max(4);
        let heap = slots
            .checked_mul(size_of::<Value>() as u64)?
            .checked_add(count.checked_mul(element.heap)?)?;
        let compact = count
            .checked_mul(element.compact.checked_add(1)?)?
            .checked_add(2)?;
        Some(Self {
            heap,
            compact,
            work: count.checked_mul(element.work.checked_add(2)?)?,
            depth: element.depth.checked_add(1)?,
        })
    }

    /// Add an already bounded child allocation to a containing record. Scalar
    /// member slots may be counted twice; this preserves a conservative bound.
    #[must_use]
    pub fn with_child(self, child: Self) -> Option<Self> {
        Some(Self {
            heap: self.heap.checked_add(child.heap)?,
            compact: self.compact.checked_add(child.compact)?,
            work: self.work.checked_add(child.work)?,
            depth: self.depth.max(child.depth.checked_add(1)?),
        })
    }

    /// Include a separately derived exact-formatting work/scratch bound.
    /// Formatting temporaries may coexist with the completed record and writer.
    #[must_use]
    pub fn with_formatting(self, work: u64, workspace: u64) -> Option<Self> {
        Some(Self {
            work: self.work.checked_add(work)?,
            heap: self.heap.checked_add(workspace)?,
            ..self
        })
    }

    /// Bound record construction, escaping and complete compact traversal work.
    #[must_use]
    pub const fn work_items(self) -> u64 {
        self.work
    }

    /// Bound simultaneous owned records, compact string and writer-frame storage.
    #[must_use]
    pub fn workspace_bytes(self) -> Option<u64> {
        // Each writer frame is at most four machine words; Vec growth uses
        // twice the live depth with its minimum capacity of four.
        let frames = self
            .depth
            .checked_mul(2)?
            .max(4)
            .checked_mul((4 * size_of::<usize>()) as u64)?;
        self.heap
            .checked_add(self.compact.checked_mul(2)?)?
            .checked_add(frames)
    }
}

#[cfg(test)]
mod tests {
    use super::OutputLayout;
    use crate::json::{Object, Value, write_compact};

    #[test]
    fn repeated_escaped_records_have_checked_complete_output_bounds() {
        let item = OutputLayout::record(2, 12).unwrap();
        let layout = OutputLayout::array(257, item).unwrap();
        let value = Value::Array(
            (0..257)
                .map(|_| Object::new().with("x", "\0\n\"").with("y", "中文").into())
                .collect(),
        );
        assert!(layout.workspace_bytes().unwrap() > write_compact(&value).len() as u64);
        assert!(layout.work_items() > 257);
        assert_eq!(OutputLayout::array(u64::MAX, item), None);
        assert_eq!(OutputLayout::record(1, u64::MAX), None);
    }
}
