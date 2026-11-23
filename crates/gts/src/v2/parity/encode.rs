// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::io::{self, Write};

use purrdf_hash::blake3::Hasher;
use purrdf_lex::cbor::head;

use super::{
    DATA_COLUMNS, DIGEST_ENTRY_BYTES, Digest, MAX_SHARD_BYTES, PARITY_ROWS, SHARD_DIGEST_BYTES,
    SPAN_BYTES, parity_table_digest, shard_hasher,
};

const DATA_SHARDS: usize = SPAN_BYTES / MAX_SHARD_BYTES;
const DEPTH: usize = DATA_SHARDS / DATA_COLUMNS;
const PARITY_SHARDS: usize = DEPTH * PARITY_ROWS;
const PARITY_BYTES: usize = PARITY_SHARDS * MAX_SHARD_BYTES;
const DATA_START: usize = 1 + head::head_len(DATA_SHARDS as u64);
const PARITY_START: usize =
    DATA_START + DATA_SHARDS * DIGEST_ENTRY_BYTES + head::head_len(PARITY_SHARDS as u64);
const TABLE_END: usize = PARITY_START + PARITY_SHARDS * DIGEST_ENTRY_BYTES;
const PARITY_BODY: usize = TABLE_END + head::head_len(PARITY_BYTES as u64);
const PAYLOAD_BYTES: usize = PARITY_BODY + PARITY_BYTES;

/// One full span, with parity accumulated directly in the encoded payload.
#[derive(Debug)]
struct FullSpan {
    payload: Vec<u8>,
    data_shards: usize,
}

impl FullSpan {
    fn allocate() -> io::Result<Self> {
        let mut payload = Vec::new();
        payload
            .try_reserve_exact(PAYLOAD_BYTES)
            .map_err(io::Error::other)?;
        head::write_head(&mut payload, head::ARRAY, 3)?;
        for count in [DATA_SHARDS, PARITY_SHARDS] {
            head::write_head(&mut payload, head::ARRAY, count as u64)?;
            for _ in 0..count {
                head::write_bytes(&mut payload, &[0; SHARD_DIGEST_BYTES])?;
            }
        }
        head::write_head(&mut payload, head::BYTES, PARITY_BYTES as u64)?;
        payload.resize(PAYLOAD_BYTES, 0);
        Ok(Self {
            payload,
            data_shards: 0,
        })
    }

    fn add_shard(&mut self, bytes: &[u8], initial: &Hasher) {
        let shard = self.data_shards;
        write_digest(&mut self.payload, DATA_START, shard, bytes, initial);
        let stripe_start = shard % DEPTH * PARITY_ROWS * MAX_SHARD_BYTES;
        let column = shard / DEPTH;
        for (row, parity) in self.payload[PARITY_BODY + stripe_start..]
            .as_chunks_mut::<MAX_SHARD_BYTES>()
            .0
            .iter_mut()
            .take(PARITY_ROWS)
            .enumerate()
        {
            super::field::add_data_shard(parity, bytes, row, column);
        }
        self.data_shards += 1;
    }

    fn finish(mut self) -> Vec<u8> {
        let initial = shard_hasher();
        let (head, parity) = self.payload.split_at_mut(PARITY_BODY);
        for (shard, bytes) in parity.as_chunks::<MAX_SHARD_BYTES>().0.iter().enumerate() {
            write_digest(head, PARITY_START, shard, bytes, &initial);
        }
        self.payload
    }
}

fn write_digest(payload: &mut [u8], start: usize, shard: usize, bytes: &[u8], initial: &Hasher) {
    let mut hasher = initial.clone();
    hasher.update(bytes);
    let offset = start + shard * DIGEST_ENTRY_BYTES + 1;
    payload[offset..offset + SHARD_DIGEST_BYTES]
        .copy_from_slice(&hasher.finalize().as_bytes()[..SHARD_DIGEST_BYTES]);
}

#[derive(Debug)]
pub(in crate::v2) struct EncodedParity {
    pub(in crate::v2) first: u64,
    pub(in crate::v2) end: u64,
    pub(in crate::v2) table: Digest,
    pub(in crate::v2) payload: Vec<u8>,
}

#[derive(Debug)]
pub(in crate::v2) struct Recorder {
    opener: Digest,
    initial: Hasher,
    shard: Vec<u8>,
    current: Option<FullSpan>,
    finished: Vec<EncodedParity>,
    first: u64,
}

impl Recorder {
    pub(in crate::v2) fn new() -> io::Result<Self> {
        let mut shard = Vec::new();
        shard
            .try_reserve_exact(MAX_SHARD_BYTES)
            .map_err(io::Error::other)?;
        Ok(Self {
            opener: [0; super::DIGEST_LENGTH],
            initial: shard_hasher(),
            shard,
            current: None,
            finished: Vec::new(),
            first: 0,
        })
    }

    pub(in crate::v2) const fn set_opener(&mut self, opener: Digest) {
        self.opener = opener;
    }

    pub(in crate::v2) fn drain(&mut self) -> impl Iterator<Item = EncodedParity> + '_ {
        self.finished.drain(..)
    }

    pub(in crate::v2) fn record(&mut self, mut bytes: &[u8]) -> io::Result<()> {
        while !bytes.is_empty() {
            let length = bytes.len().min(MAX_SHARD_BYTES - self.shard.len());
            let (prefix, rest) = bytes.split_at(length);
            self.shard.extend_from_slice(prefix);
            bytes = rest;
            if self.shard.len() != MAX_SHARD_BYTES {
                break;
            }

            if self.current.is_none() {
                self.current = Some(FullSpan::allocate()?);
            }
            let span = self.current.as_mut().expect("a span was just allocated");
            span.add_shard(&self.shard, &self.initial);
            self.shard.clear();
            if span.data_shards == DATA_SHARDS {
                self.finish_span()?;
            }
        }
        Ok(())
    }

    fn finish_span(&mut self) -> io::Result<()> {
        let end = self
            .first
            .checked_add(SPAN_BYTES as u64)
            .ok_or_else(|| io::Error::other("GTS parity span exceeds the wire offset range"))?;
        self.finished.try_reserve(1).map_err(io::Error::other)?;
        let span = self
            .current
            .take()
            .expect("only a complete span can finish");
        let payload = span.finish();
        self.finished.push(EncodedParity {
            first: self.first,
            end,
            table: parity_table_digest(&self.opener, &payload[1..TABLE_END]),
            payload,
        });
        self.first = end;
        Ok(())
    }
}

pub(in crate::v2) struct RecordingWriter<'a, W> {
    pub(in crate::v2) sink: &'a mut W,
    pub(in crate::v2) recorder: Option<&'a mut Recorder>,
}

impl<W: Write> Write for RecordingWriter<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.sink.write(bytes)?;
        if let Some(recorder) = &mut self.recorder {
            recorder.record(&bytes[..written])?;
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.sink.flush()
    }
}
