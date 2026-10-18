// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original asserted key-value rows, indexed once for the native preparation pass.

use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Common {
    pub(crate) property: u32,
    pub(crate) value: u32,
    pub(crate) left: usize,
    pub(crate) right: usize,
}

impl std::fmt::Debug for Common {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Common")
            .field("property", &self.property)
            .field("value", &self.value)
            .field("left", &self.left)
            .field("right", &self.right)
            .finish()
    }
}

/// Borrow actual assertions; sorting stores only their ordinals, never copies RDF rows.
pub(crate) struct Index<'a> {
    rows: &'a [(u32, u32, u32)],
    order: Vec<usize>,
}

impl std::fmt::Debug for Index<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Index")
            .field("rows", &self.rows.len())
            .field("order", &self.order)
            .finish()
    }
}

impl<'a> Index<'a> {
    pub(crate) fn new<S: Admission + ?Sized>(
        rows: &'a [(u32, u32, u32)],
        properties: impl IntoIterator<Item = u32>,
        stop: Option<&dyn purrdf_datalog::StopSignal>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, crate::EntailError> {
        memory.try_scope(|memory| {
            poll(stop)?;
            let mut properties = memory.collect(properties)?;
            properties.sort_unstable();
            properties.dedup();
            let mut order = Vec::new();
            for (index, &(_, property, _)) in rows.iter().enumerate() {
                poll(stop)?;
                if properties.binary_search(&property).is_ok() {
                    memory.push(&mut order, index)?;
                }
            }
            order.sort_unstable_by_key(|&index| (rows[index], index));
            memory.release_vec(properties)?;
            poll(stop)?;
            Ok(Self { rows, order })
        })
    }

    fn values(&self, subject: u32, property: u32) -> &[usize] {
        let key = (subject, property);
        let start = self.order.partition_point(|&index| {
            let (s, p, _) = self.rows[index];
            (s, p) < key
        });
        let end = self.order.partition_point(|&index| {
            let (s, p, _) = self.rows[index];
            (s, p) <= key
        });
        &self.order[start..end]
    }

    pub(crate) fn common(
        &self,
        left: u32,
        right: u32,
        property: u32,
        stop: Option<&dyn purrdf_datalog::StopSignal>,
    ) -> Result<Option<Common>, crate::EntailError> {
        let mut left = self.values(left, property).iter().copied();
        let mut right = self.values(right, property).iter().copied();
        let mut a = left.next();
        let mut b = right.next();
        while let (Some(l), Some(r)) = (a, b) {
            poll(stop)?;
            match self.rows[l].2.cmp(&self.rows[r].2) {
                std::cmp::Ordering::Less => a = left.next(),
                std::cmp::Ordering::Greater => b = right.next(),
                std::cmp::Ordering::Equal => {
                    return Ok(Some(Common {
                        property,
                        value: self.rows[l].2,
                        left: l,
                        right: r,
                    }));
                }
            }
        }
        poll(stop)?;
        Ok(None)
    }

    pub(crate) fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.release_vec(self.order)
    }
}

#[derive(Default)]
pub(crate) struct Reads {
    pub(crate) base: usize,
    pub(crate) spans: Vec<std::ops::Range<usize>>,
    pub(crate) values: Vec<Common>,
    storage: Resident,
    live: usize,
}

impl std::fmt::Debug for Reads {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reads")
            .field("base", &self.base)
            .field("spans", &self.spans)
            .field("values", &self.values)
            .field("storage", &self.storage)
            .field("live", &self.live)
            .finish()
    }
}

impl Reads {
    pub(crate) fn push(&mut self, value: Common) -> Result<(), StorageError> {
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = memory.push(&mut self.values, value);
        self.live = memory.admitted_bytes();
        result
    }

    pub(crate) fn finish(&mut self, start: usize) -> Result<(), StorageError> {
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = memory.push(&mut self.spans, start..self.values.len());
        self.live = memory.admitted_bytes();
        result
    }

    #[cfg(test)]
    pub(crate) fn get(&self, equality: usize) -> Option<&[Common]> {
        self.values
            .get(self.spans.get(equality.checked_sub(self.base)?)?.clone())
    }
}

fn poll(stop: Option<&dyn purrdf_datalog::StopSignal>) -> Result<(), crate::EntailError> {
    if stop.is_some_and(purrdf_datalog::StopSignal::stopped) {
        Err(crate::EntailError::Stopped)
    } else {
        Ok(())
    }
}
