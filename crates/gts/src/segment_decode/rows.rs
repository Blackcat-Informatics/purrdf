// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Replay of accepted numeric row events through the one CBOR-head codec.

use crate::dictionary::DictionaryError;
use purrdf_lex::cbor::{
    ReplayStorage,
    head::{self, ARRAY, UNSIGNED},
};
use std::io::{self, Read, Seek, SeekFrom, Write};

const ROW_BYTES: usize = 64;

#[derive(Default)]
pub(super) struct Rows<const N: usize> {
    bytes: Vec<u8>,
    storage: Option<Box<dyn ReplayStorage>>,
    start: u64,
    end: u64,
    count: usize,
    poisoned: bool,
}

impl<const N: usize> Rows<N> {
    pub(super) fn external(mut storage: Box<dyn ReplayStorage>) -> io::Result<Self> {
        let start = storage.stream_position()?;
        Ok(Self {
            storage: Some(storage),
            start,
            end: start,
            ..Self::default()
        })
    }

    pub(super) fn len(&self) -> usize {
        self.count
    }

    pub(super) fn push(
        &mut self,
        segment: u64,
        ids: [usize; N],
        graph: Option<usize>,
    ) -> io::Result<()> {
        if self.poisoned {
            return Err(corrupt("a failed row replay cannot accept more rows"));
        }
        let next = self
            .count
            .checked_add(1)
            .ok_or_else(|| corrupt("row count overflow"))?;
        let mut bytes = [0; ROW_BYTES];
        let mut output = &mut bytes[..];
        head::write_head(
            &mut output,
            ARRAY,
            (N + 1 + usize::from(graph.is_some())) as u64,
        )?;
        head::write_head(&mut output, UNSIGNED, segment)?;
        for id in ids.into_iter().chain(graph) {
            head::write_head(&mut output, UNSIGNED, id as u64)?;
        }
        let used = ROW_BYTES - output.len();
        let encoded = &bytes[..used];
        if let Some(storage) = self.storage.as_mut() {
            self.poisoned = true;
            let end = self
                .end
                .checked_add((used + 33) as u64)
                .ok_or_else(|| corrupt("row storage offset overflow"))?;
            storage.seek(SeekFrom::Start(self.end))?;
            storage.write_all(&[used as u8])?;
            storage.write_all(encoded)?;
            storage.write_all(purrdf_hash::blake3::hash(encoded).as_bytes())?;
            self.end = end;
            self.poisoned = false;
        } else {
            self.bytes.extend_from_slice(encoded);
        }
        self.count = next;
        Ok(())
    }

    pub(super) fn finish(&mut self) -> io::Result<()> {
        match self.storage.as_mut() {
            Some(storage) => storage.finish(),
            None => Ok(()),
        }
    }
}

impl<const N: usize> IntoIterator for Rows<N> {
    type Item = io::Result<(u64, [usize; N], Option<usize>)>;
    type IntoIter = Iter<N>;

    fn into_iter(self) -> Self::IntoIter {
        let remaining = self.count;
        Iter {
            rows: self,
            offset: 0,
            remaining,
            started: false,
            failed: false,
        }
    }
}

pub(super) struct Iter<const N: usize> {
    rows: Rows<N>,
    offset: usize,
    remaining: usize,
    started: bool,
    failed: bool,
}

impl<const N: usize> Iter<N> {
    pub(super) fn into_empty(mut self, succeeded: bool) -> Rows<N> {
        self.rows.poisoned |= !succeeded;
        self.rows.bytes.clear();
        self.rows.count = 0;
        self.rows.start = self.rows.end;
        self.rows
    }

    fn read_row(&mut self) -> io::Result<(u64, [usize; N], Option<usize>)> {
        let mut buffer = [0; ROW_BYTES];
        let owned_input = if let Some(storage) = self.rows.storage.as_mut() {
            if !self.started {
                storage.flush()?;
                storage.seek(SeekFrom::Start(self.rows.start))?;
                self.started = true;
            }
            let mut length = [0];
            storage.read_exact(&mut length)?;
            let length = usize::from(length[0]);
            let bytes = buffer
                .get_mut(..length)
                .ok_or_else(|| corrupt("invalid row record width"))?;
            storage.read_exact(bytes)?;
            let mut digest = [0; 32];
            storage.read_exact(&mut digest)?;
            if purrdf_hash::blake3::hash(bytes).as_bytes() != &digest {
                return Err(corrupt("accepted row bytes changed in replay storage"));
            }
            Some(length)
        } else {
            None
        };
        let mut input = match owned_input {
            Some(length) => &buffer[..length],
            None => &self.rows.bytes[self.offset..],
        };
        let before = input.len();
        let width = head::read_argument(&mut input, ARRAY)
            .ok_or_else(|| corrupt("missing accepted row array head"))?;
        if width != (N + 1) as u64 && width != (N + 2) as u64 {
            return Err(corrupt("accepted row has the wrong identifier width"));
        }
        let segment = head::read_argument(&mut input, UNSIGNED)
            .ok_or_else(|| corrupt("missing accepted row segment"))?;
        let mut id = || {
            let value = head::read_argument(&mut input, UNSIGNED)
                .ok_or_else(|| corrupt("missing accepted row identifier"))?;
            usize::try_from(value).map_err(|_| corrupt("accepted identifier exceeds the platform"))
        };
        let mut ids = [0; N];
        for value in &mut ids {
            *value = id()?;
        }
        let graph = if width == (N + 2) as u64 {
            Some(id()?)
        } else {
            None
        };
        if owned_input.is_some() && !input.is_empty() {
            return Err(corrupt("trailing bytes in accepted row record"));
        }
        self.offset += before - input.len();
        Ok((segment, ids, graph))
    }
}

impl<const N: usize> Iterator for Iter<N> {
    type Item = io::Result<(u64, [usize; N], Option<usize>)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rows.poisoned && !self.failed {
            self.failed = true;
            return Some(Err(corrupt("a failed row replay cannot be resumed")));
        }
        if self.failed || self.remaining == 0 {
            return None;
        }
        let row = self.read_row();
        self.failed = row.is_err();
        if !self.failed {
            self.remaining -= 1;
        }
        Some(row)
    }
}

impl<const N: usize> std::iter::FusedIterator for Iter<N> {}

fn corrupt(reason: &'static str) -> io::Error {
    purrdf_hash::error::io_error(io::ErrorKind::InvalidData, DictionaryError::Corrupt(reason))
}
