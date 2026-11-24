// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::io::{self, Write};

use purrdf_hash::blake3::Hasher;
use purrdf_lex::cbor::head;

use super::{
    DATA_COLUMNS, DIGEST_ENTRY_BYTES, Digest, Geometry, MAX_SHARD_BYTES, PARITY_ROWS,
    SHARD_DIGEST_BYTES, SPAN_BYTES, ZERO_PADDING, parity_table_digest, shard_hasher,
};

const DATA_SHARDS: usize = SPAN_BYTES / MAX_SHARD_BYTES;
const FULL_GEOMETRY: Geometry = Geometry {
    span_bytes: SPAN_BYTES,
    shard_bytes: MAX_SHARD_BYTES,
    data_shards: DATA_SHARDS,
    parity_shards: DATA_SHARDS / DATA_COLUMNS * PARITY_ROWS,
};

/// One span, with parity accumulated directly in the encoded payload.
#[derive(Debug)]
struct Span {
    payload: Vec<u8>,
    geometry: Geometry,
    shard_count: usize,
    data_start: usize,
    parity_start: usize,
    table_end: usize,
    parity_body: usize,
}

impl Span {
    fn allocate(geometry: Geometry) -> io::Result<Self> {
        let mut payload = Vec::new();
        let payload_bytes = geometry.payload_length();
        payload
            .try_reserve_exact(payload_bytes)
            .map_err(io::Error::other)?;
        head::write_head(&mut payload, head::ARRAY, 3)?;
        let data_start = reserve_digests(&mut payload, geometry.data_shards)?;
        let parity_start = reserve_digests(&mut payload, geometry.parity_shards)?;
        let table_end = payload.len();
        let parity_bytes = geometry.parity_shards * geometry.shard_bytes;
        head::write_head(&mut payload, head::BYTES, parity_bytes as u64)?;
        let parity_body = payload.len();
        payload.resize(payload_bytes, 0);
        Ok(Self {
            payload,
            geometry,
            shard_count: 0,
            data_start,
            parity_start,
            table_end,
            parity_body,
        })
    }

    fn from_bytes(bytes: &[u8], initial: &Hasher) -> io::Result<Self> {
        let geometry = Geometry::for_span(0, bytes.len() as u64)
            .expect("a buffered span has between one byte and four MiB");
        let size = geometry.shard_bytes;
        let mut span = Self::allocate(geometry)?;
        for shard in bytes.chunks(size) {
            span.add_shard(shard, initial);
        }
        Ok(span)
    }

    fn add_shard(&mut self, bytes: &[u8], initial: &Hasher) {
        let shard = self.shard_count;
        let size = self.geometry.shard_bytes;
        write_digest(
            &mut self.payload,
            self.data_start,
            shard,
            bytes,
            size,
            initial,
        );
        let depth = self.geometry.parity_shards / PARITY_ROWS;
        let stripe_start = self.parity_body + shard % depth * PARITY_ROWS * size;
        let stripe = &mut self.payload[stripe_start..stripe_start + PARITY_ROWS * size];
        let column = shard / depth;
        for (row, parity) in stripe.chunks_exact_mut(size).enumerate() {
            super::field::add_data_shard(parity, bytes, row, column);
        }
        self.shard_count += 1;
    }

    fn finish(
        mut self,
        opener: &Digest,
        first: u64,
        initial: &Hasher,
    ) -> io::Result<EncodedParity> {
        let size = self.geometry.shard_bytes;
        let (head, parity) = self.payload.split_at_mut(self.parity_body);
        for (shard, bytes) in parity.chunks_exact(size).enumerate() {
            write_digest(head, self.parity_start, shard, bytes, size, initial);
        }
        let end = first
            .checked_add(self.geometry.span_bytes as u64)
            .ok_or_else(|| io::Error::other("GTS parity span exceeds the wire offset range"))?;
        Ok(EncodedParity {
            first,
            end,
            table: parity_table_digest(opener, &self.payload[1..self.table_end]),
            payload: self.payload,
        })
    }
}

fn reserve_digests(payload: &mut Vec<u8>, count: usize) -> io::Result<usize> {
    head::write_head(payload, head::ARRAY, count as u64)?;
    let start = payload.len();
    for _ in 0..count {
        head::write_bytes(payload, &[0; SHARD_DIGEST_BYTES])?;
    }
    Ok(start)
}

fn write_digest(
    payload: &mut [u8],
    start: usize,
    shard: usize,
    bytes: &[u8],
    size: usize,
    initial: &Hasher,
) {
    let mut hasher = initial.clone();
    hasher.update(bytes);
    if bytes.len() < size {
        hasher.update(&ZERO_PADDING[..size - bytes.len()]);
    }
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
    current: Option<Span>,
    finished: Vec<EncodedParity>,
    first: u64,
    closing_tail: Option<Vec<u8>>,
}

impl Recorder {
    pub(in crate::v2) fn new() -> io::Result<Self> {
        Self::with_tail(None)
    }

    pub(in crate::v2) fn new_closing() -> io::Result<Self> {
        Self::with_tail(Some(Vec::new()))
    }

    fn with_tail(closing_tail: Option<Vec<u8>>) -> io::Result<Self> {
        let mut shard = Vec::new();
        if closing_tail.is_none() {
            shard
                .try_reserve_exact(MAX_SHARD_BYTES)
                .map_err(io::Error::other)?;
        }
        Ok(Self {
            opener: [0; super::DIGEST_LENGTH],
            initial: shard_hasher(),
            shard,
            current: None,
            finished: Vec::new(),
            first: 0,
            closing_tail,
        })
    }

    pub(in crate::v2) const fn set_opener(&mut self, opener: Digest) {
        self.opener = opener;
    }

    pub(in crate::v2) fn drain(&mut self) -> impl Iterator<Item = EncodedParity> + '_ {
        self.finished.drain(..)
    }

    pub(in crate::v2) fn record(&mut self, mut bytes: &[u8]) -> io::Result<()> {
        if let Some(tail) = &mut self.closing_tail {
            while !bytes.is_empty() {
                let length = bytes.len().min(SPAN_BYTES - tail.len());
                let (prefix, rest) = bytes.split_at(length);
                let required = tail.len() + length;
                if required > tail.capacity() {
                    let capacity = required.max(tail.capacity() * 2).min(SPAN_BYTES);
                    tail.try_reserve_exact(capacity - tail.len())
                        .map_err(io::Error::other)?;
                }
                tail.extend_from_slice(prefix);
                bytes = rest;
                if tail.len() == SPAN_BYTES {
                    let span = Span::from_bytes(tail, &self.initial)?;
                    queue_span(
                        span,
                        &self.opener,
                        &self.initial,
                        &mut self.first,
                        &mut self.finished,
                    )?;
                    tail.clear();
                }
            }
            return Ok(());
        }

        while !bytes.is_empty() {
            let length = bytes.len().min(MAX_SHARD_BYTES - self.shard.len());
            let (prefix, rest) = bytes.split_at(length);
            self.shard.extend_from_slice(prefix);
            bytes = rest;
            if self.shard.len() != MAX_SHARD_BYTES {
                break;
            }

            if self.current.is_none() {
                self.current = Some(Span::allocate(FULL_GEOMETRY)?);
            }
            let span = self.current.as_mut().expect("a span was just allocated");
            span.add_shard(&self.shard, &self.initial);
            self.shard.clear();
            if span.shard_count == DATA_SHARDS {
                let span = self
                    .current
                    .take()
                    .expect("only a complete span can finish");
                queue_span(
                    span,
                    &self.opener,
                    &self.initial,
                    &mut self.first,
                    &mut self.finished,
                )?;
            }
        }
        Ok(())
    }

    pub(in crate::v2) fn finish_closing(&mut self) -> io::Result<Option<EncodedParity>> {
        let tail = self
            .closing_tail
            .take()
            .expect("a closing recorder has an open span buffer");
        if tail.is_empty() {
            return Ok(None);
        }
        let span = Span::from_bytes(&tail, &self.initial)?;
        span.finish(&self.opener, self.first, &self.initial)
            .map(Some)
    }
}

fn queue_span(
    span: Span,
    opener: &Digest,
    initial: &Hasher,
    first: &mut u64,
    finished: &mut Vec<EncodedParity>,
) -> io::Result<()> {
    finished.try_reserve(1).map_err(io::Error::other)?;
    let encoded = span.finish(opener, *first, initial)?;
    *first = encoded.end;
    finished.push(encoded);
    Ok(())
}

pub(in crate::v2) struct RecordingWriter<'a, W> {
    pub(in crate::v2) sink: &'a mut W,
    pub(in crate::v2) recorder: Option<&'a mut Recorder>,
    pub(in crate::v2) length: Option<&'a mut u64>,
}

impl<W: Write> Write for RecordingWriter<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.sink.write(bytes)?;
        if let Some(length) = &mut self.length {
            **length = length
                .checked_add(written as u64)
                .ok_or_else(|| io::Error::other("GTS item exceeds the wire length"))?;
        }
        if let Some(recorder) = &mut self.recorder {
            recorder.record(&bytes[..written])?;
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.sink.flush()
    }
}
