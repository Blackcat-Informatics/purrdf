// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One native allocation account for scanners, parser frames and syntax products.

use crate::allocation::{Admission, Memory, OrderedTable, StorageError};

pub(super) struct Allocations {
    limit: Option<usize>,
    live: usize,
}

pub(super) struct Ceiling(usize);
impl Admission for Ceiling {
    fn resize(&mut self, live: usize) -> Result<(), StorageError> {
        if live > self.0 {
            Err(StorageError::AdmissionFailed)
        } else {
            Ok(())
        }
    }
}

impl Allocations {
    pub(super) const fn legacy() -> Self {
        Self {
            limit: None,
            live: 0,
        }
    }
    pub(super) const fn bounded(limit: usize) -> Self {
        Self {
            limit: Some(limit),
            live: 0,
        }
    }
    pub(super) const fn limit(&self) -> Option<usize> {
        self.limit
    }
    pub(super) const fn live(&self) -> usize {
        self.live
    }
    pub(super) fn format(
        &mut self,
        value: &impl core::fmt::Display,
    ) -> Result<String, StorageError> {
        if self.limit.is_none() {
            return Ok(value.to_string());
        }
        self.memory(|memory| memory.format(value))
    }
    pub(super) fn release_bytes(&mut self, bytes: usize) -> Result<(), StorageError> {
        if self.limit.is_none() {
            return Ok(());
        }
        self.memory(|memory| memory.release_bytes(bytes))
    }
    pub(super) fn memory<T>(
        &mut self,
        body: impl FnOnce(&mut Memory<'_, Ceiling>) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let mut ceiling = Ceiling(self.limit.unwrap_or(usize::MAX));
        let mut memory = Memory::resume(&mut ceiling, self.live);
        let result = body(&mut memory);
        self.live = memory.admitted_bytes();
        result
    }
    pub(super) fn push<T>(&mut self, values: &mut Vec<T>, value: T) -> Result<(), StorageError> {
        if self.limit.is_none() {
            values.push(value);
            return Ok(());
        }
        self.memory(|memory| memory.push(values, value))
    }
    pub(super) fn string(&mut self, text: &str) -> Result<String, StorageError> {
        if self.limit.is_none() {
            return Ok(text.to_owned());
        }
        self.memory(|memory| {
            let mut value = String::new();
            memory.push_str(&mut value, text)?;
            Ok(value)
        })
    }
    pub(super) fn push_str(&mut self, text: &mut String, suffix: &str) -> Result<(), StorageError> {
        if self.limit.is_none() {
            text.push_str(suffix);
            return Ok(());
        }
        self.memory(|memory| memory.push_str(text, suffix))
    }
    pub(super) fn push_char(&mut self, text: &mut String, ch: char) -> Result<(), StorageError> {
        if self.limit.is_none() {
            text.push(ch);
            return Ok(());
        }
        self.memory(|memory| memory.push_char(text, ch))
    }
    pub(super) fn repeat(
        &mut self,
        text: &mut String,
        ch: char,
        count: usize,
    ) -> Result<(), StorageError> {
        if self.limit.is_some() {
            let bytes = count
                .checked_mul(ch.len_utf8())
                .and_then(|n| text.len().checked_add(n))
                .ok_or(StorageError::SizeOverflow)?;
            self.memory(|memory| memory.reserve_string(text, bytes))?;
        }
        text.extend(core::iter::repeat_n(ch, count));
        Ok(())
    }
    pub(super) fn reserve_index<K, V>(
        &mut self,
        table: &mut OrderedTable<K, V>,
    ) -> Result<(), StorageError> {
        self.memory(|memory| table.reserve_insert(memory))
    }
    pub(super) fn discard_string(&mut self, value: String) -> Result<(), StorageError> {
        if self.limit.is_none() {
            drop(value);
            return Ok(());
        }
        self.memory(|memory| memory.release_string(value))
    }
    pub(super) fn discard_vec<T>(&mut self, value: Vec<T>) -> Result<(), StorageError> {
        if self.limit.is_none() {
            drop(value);
            return Ok(());
        }
        self.memory(|memory| memory.release_vec(value))
    }
}
