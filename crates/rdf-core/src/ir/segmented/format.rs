// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Checked local codecs for the segmented representation. No host I/O lives here.

use crate::bytes::{read_u32_le, read_u64_le};
use crate::{BlankScope, GlobalTermId, QuadIds, RdfTextDirection, TermRef};
use purrdf_hash::blake3::Hasher;

use super::{
    SEGMENTED_BLOCK_DOMAIN, SEGMENTED_NODE_DOMAIN, SEGMENTED_SNAPSHOT_DOMAIN, SegmentedError,
    SegmentedSnapshot,
};

pub(super) const MAGIC: &[u8; 8] = b"PURRSEG7";
pub(super) const HEADER_BYTES: usize = 456;
pub(super) const RECORD_HEADER: usize = 13;
const COMPRESSED_RECORDS: u32 = 1 << 31;
pub(super) const SUMMARY_BYTES: usize = 104;
pub(super) const VALUE_RANGES: u8 = 14;
pub(super) const VALUE_RANGE_BYTES: usize = 24;
pub(super) const MAX_RECORDS_PER_BLOCK: u32 = 256;
pub(super) const DICTIONARY: u8 = 1;
pub(super) const REVERSE: u8 = 2;
pub(super) const QUADS: u8 = 3;
pub(super) const GRAPHS: u8 = 4;
pub(super) const SUMMARIES: u8 = 5;
pub(super) const SOURCE_DIRECTORY: u8 = 10;
pub(super) const SOURCE_CHECKPOINTS: u8 = 11;
pub(super) const SPANNED_DICTIONARY: u8 = 12;
pub(super) const OVERFLOW: u8 = 13;

pub(super) const fn statistics_kind(section: super::super::statistics::StatisticsSection) -> u8 {
    6 + section.index() as u8
}

/// Binary search over a certified logical directory without materializing it.
pub(super) fn partition_point<E>(
    count: u64,
    mut before: impl FnMut(u64) -> Result<bool, E>,
) -> Result<u64, E> {
    let (mut low, mut high) = (0, count);
    while low < high {
        let middle = low + (high - low) / 2;
        if before(middle)? {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    Ok(low)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Stream {
    pub first: u64,
    pub blocks: u64,
    pub rows: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Header {
    pub block_bytes: u32,
    pub records_per_block: u32,
    pub terms: u64,
    pub first_term_index: u64,
    pub max_owned_term_bytes: u64,
    pub dictionary: Stream,
    pub reverse: Stream,
    pub streams: [Stream; 3],
    pub graphs: Stream,
    pub summaries: Stream,
    pub value_ranges: Stream,
    pub statistics: [Stream; 4],
    pub effective_directory: Stream,
    pub source_checkpoints: Stream,
    pub overflow: Stream,
    pub flags: u8,
    pub source_union: purrdf_hash::multiset::Multiset256,
}

impl Header {
    pub(super) fn dictionary_decode_bound(self, kind: u8) -> u64 {
        if kind == SPANNED_DICTIONARY {
            self.max_owned_term_bytes
        } else {
            u64::from(self.block_bytes) * u64::from(self.records_per_block)
        }
    }

    pub(super) fn encode(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_BYTES);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&7_u32.to_le_bytes());
        out.extend_from_slice(&self.block_bytes.to_le_bytes());
        out.extend_from_slice(&self.records_per_block.to_le_bytes());
        out.push(self.flags);
        out.extend_from_slice(&[0; 3]);
        out.extend_from_slice(&self.terms.to_le_bytes());
        out.extend_from_slice(&self.first_term_index.to_le_bytes());
        out.extend_from_slice(&self.max_owned_term_bytes.to_le_bytes());
        for stream in [
            self.dictionary,
            self.reverse,
            self.streams[0],
            self.streams[1],
            self.streams[2],
            self.graphs,
            self.summaries,
            self.value_ranges,
            self.statistics[0],
            self.statistics[1],
            self.statistics[2],
            self.statistics[3],
            self.effective_directory,
            self.source_checkpoints,
            self.overflow,
        ] {
            for value in [stream.first, stream.blocks, stream.rows] {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        out.extend_from_slice(&self.source_union.population().to_le_bytes());
        out.extend_from_slice(self.source_union.sum());
        out
    }

    pub(super) fn decode(bytes: &[u8]) -> Result<Self, SegmentedError> {
        if bytes.get(..8) != Some(MAGIC.as_slice()) || read_u32_le(bytes, 8) != Some(7) {
            return Err(SegmentedError::Corrupt(
                "unsupported segmented format/profile; regenerate the image from RDF input",
            ));
        }
        let mut input = Reader::at(bytes, 12);
        let block_bytes = input.u32()?;
        let records_per_block = input.u32()?;
        let flags = input.byte()?;
        if input.take(3)? != [0; 3]
            || flags > 7
            || !(512..=1_048_576).contains(&block_bytes)
            || records_per_block == 0
            || records_per_block > MAX_RECORDS_PER_BLOCK
        {
            return Err(SegmentedError::Corrupt("invalid segmented header"));
        }
        let terms = input.u64()?;
        let first_term_index = input.u64()?;
        let max_owned_term_bytes = input.u64()?;
        first_term_index
            .checked_add(terms)
            .ok_or(SegmentedError::AddressExhausted)?;
        let mut streams = [Stream::default(); 15];
        for stream in &mut streams {
            *stream = Stream {
                first: input.u64()?,
                blocks: input.u64()?,
                rows: input.u64()?,
            };
        }
        let source_population = u128::from_le_bytes(
            input
                .take(16)?
                .try_into()
                .map_err(|_| SegmentedError::Corrupt("truncated source population"))?,
        );
        let source_sum = input
            .take(32)?
            .try_into()
            .map_err(|_| SegmentedError::Corrupt("truncated source sum"))?;
        let header = Self {
            block_bytes,
            records_per_block,
            flags,
            terms,
            first_term_index,
            max_owned_term_bytes,
            dictionary: streams[0],
            reverse: streams[1],
            streams: [streams[2], streams[3], streams[4]],
            graphs: streams[5],
            summaries: streams[6],
            value_ranges: streams[7],
            statistics: [streams[8], streams[9], streams[10], streams[11]],
            effective_directory: streams[12],
            source_checkpoints: streams[13],
            overflow: streams[14],
            source_union: purrdf_hash::multiset::Multiset256::from_parts(
                source_sum,
                source_population,
            ),
        };
        let physical_rows = header.streams.iter().try_fold(0_u64, |rows, stream| {
            rows.checked_add(stream.rows)
                .ok_or(SegmentedError::AddressExhausted)
        })?;
        if source_population > u128::from(physical_rows) {
            return Err(SegmentedError::Corrupt(
                "source population exceeds physical rows",
            ));
        }
        if header.dictionary.rows != terms || header.reverse.rows != terms {
            return Err(SegmentedError::Corrupt(
                "dictionary/index cardinality mismatch",
            ));
        }
        let expected = terms.div_ceil(u64::from(records_per_block));
        let reverse_expected = terms.div_ceil((u64::from(block_bytes) - 9) / 8);
        if header.dictionary.blocks < expected
            || header.dictionary.blocks > terms
            || header.reverse.blocks != reverse_expected
        {
            return Err(SegmentedError::Corrupt("dictionary block count mismatch"));
        }
        for section in super::super::statistics::StatisticsSection::ALL {
            let stream = header.statistics[section.index()];
            let per_block = (u64::from(block_bytes) - 9) / section.width() as u64;
            if stream.blocks != stream.rows.div_ceil(per_block) {
                return Err(SegmentedError::Corrupt("statistics block count mismatch"));
            }
        }
        if header.value_ranges.blocks
            != header
                .value_ranges
                .rows
                .div_ceil((u64::from(block_bytes) - 9) / VALUE_RANGE_BYTES as u64)
        {
            return Err(SegmentedError::Corrupt("value range block count mismatch"));
        }
        let per_block =
            (u64::from(block_bytes) - 9) / super::super::source_directory::RECORD_BYTES as u64;
        if u128::from(header.effective_directory.rows) != source_population
            || header.effective_directory.blocks
                != header.effective_directory.rows.div_ceil(per_block)
        {
            return Err(SegmentedError::Corrupt(
                "effective source directory packing mismatch",
            ));
        }
        let checkpoints = header.effective_directory.rows
            / super::super::source_directory::CHECKPOINT_ROWS as u64;
        if header.source_checkpoints.rows != checkpoints
            || header.source_checkpoints.blocks
                != checkpoints.div_ceil((u64::from(block_bytes) - 9) / 32)
        {
            return Err(SegmentedError::Corrupt(
                "source checkpoint packing mismatch",
            ));
        }
        if header.overflow.rows > terms || header.overflow.blocks < header.overflow.rows {
            return Err(SegmentedError::Corrupt("invalid overflow population"));
        }
        Ok(header)
    }

    pub(super) fn validate_addresses(self, count: u64) -> Result<(), SegmentedError> {
        let mut next = 1_u64;
        for stream in [
            self.dictionary,
            self.reverse,
            self.streams[0],
            self.streams[1],
            self.streams[2],
            self.graphs,
            self.summaries,
            self.value_ranges,
            self.statistics[0],
            self.statistics[1],
            self.statistics[2],
            self.statistics[3],
            self.effective_directory,
            self.source_checkpoints,
            self.overflow,
        ] {
            if stream.first != next || (stream.blocks == 0) != (stream.rows == 0) {
                return Err(SegmentedError::Corrupt("non-contiguous block directory"));
            }
            next = next
                .checked_add(stream.blocks)
                .ok_or(SegmentedError::AddressExhausted)?;
        }
        if next != count {
            return Err(SegmentedError::Corrupt("unreferenced or missing blocks"));
        }
        Ok(())
    }
}

pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Reader<'a> {
    pub(super) fn at(bytes: &'a [u8], position: usize) -> Self {
        Self { bytes, position }
    }
    pub(super) fn take(&mut self, len: usize) -> Result<&'a [u8], SegmentedError> {
        let end = self
            .position
            .checked_add(len)
            .ok_or(SegmentedError::AddressExhausted)?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or(SegmentedError::Corrupt("truncated block record"))?;
        self.position = end;
        Ok(bytes)
    }
    pub(super) fn byte(&mut self) -> Result<u8, SegmentedError> {
        Ok(self.take(1)?[0])
    }
    pub(super) fn u32(&mut self) -> Result<u32, SegmentedError> {
        let value = read_u32_le(self.bytes, self.position)
            .ok_or(SegmentedError::Corrupt("truncated u32"))?;
        self.position += 4;
        Ok(value)
    }
    pub(super) fn u64(&mut self) -> Result<u64, SegmentedError> {
        let value = read_u64_le(self.bytes, self.position)
            .ok_or(SegmentedError::Corrupt("truncated u64"))?;
        self.position += 8;
        Ok(value)
    }
    pub(super) fn text(&mut self) -> Result<&'a str, SegmentedError> {
        let length = usize::try_from(self.u32()?).map_err(|_| SegmentedError::AddressExhausted)?;
        core::str::from_utf8(self.take(length)?)
            .map_err(|_| SegmentedError::Corrupt("non-UTF-8 dictionary string"))
    }
    pub(super) fn id(&mut self) -> Result<GlobalTermId, SegmentedError> {
        let raw = self.u64()?;
        raw.checked_sub(1)
            .and_then(GlobalTermId::checked_from_index)
            .ok_or(SegmentedError::Corrupt("zero dictionary reference"))
    }
    pub(super) fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }
}

pub(super) fn push_text(out: &mut Vec<u8>, value: &str) -> Result<(), SegmentedError> {
    out.extend_from_slice(
        &u32::try_from(value.len())
            .map_err(|_| SegmentedError::AddressExhausted)?
            .to_le_bytes(),
    );
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

pub(super) fn push_id(out: &mut Vec<u8>, id: GlobalTermId) {
    out.extend_from_slice(&(id.index() + 1).to_le_bytes());
}

pub(super) fn encode_term(
    term: TermRef<'_, GlobalTermId>,
    out: &mut Vec<u8>,
) -> Result<(), SegmentedError> {
    match term {
        TermRef::Iri(iri) => {
            out.push(0);
            push_text(out, iri)?;
        }
        TermRef::Blank { label, scope } => {
            out.push(1);
            out.extend_from_slice(&scope.ordinal().to_le_bytes());
            push_text(out, label)?;
        }
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            out.push(2);
            push_id(out, datatype);
            push_text(out, lexical)?;
            out.push(u8::from(language.is_some()));
            if let Some(language) = language {
                push_text(out, language)?;
            }
            out.push(match direction {
                None => 0,
                Some(RdfTextDirection::Ltr) => 1,
                Some(RdfTextDirection::Rtl) => 2,
            });
        }
        TermRef::Triple { s, p, o } => {
            out.push(3);
            for id in [s, p, o] {
                push_id(out, id);
            }
        }
    }
    Ok(())
}

pub(super) fn decode_term(bytes: &[u8]) -> Result<TermRef<'_, GlobalTermId>, SegmentedError> {
    let mut input = Reader::at(bytes, 0);
    let term = match input.byte()? {
        0 => TermRef::Iri(input.text()?),
        1 => TermRef::Blank {
            scope: BlankScope(input.u32()?),
            label: input.text()?,
        },
        2 => {
            let datatype = input.id()?;
            let lexical = input.text()?;
            let language = match input.byte()? {
                0 => None,
                1 => Some(input.text()?),
                _ => return Err(SegmentedError::Corrupt("invalid language presence")),
            };
            let direction = match input.byte()? {
                0 => None,
                1 => Some(RdfTextDirection::Ltr),
                2 => Some(RdfTextDirection::Rtl),
                _ => return Err(SegmentedError::Corrupt("invalid text direction")),
            };
            TermRef::Literal {
                datatype,
                lexical,
                language,
                direction,
            }
        }
        3 => TermRef::Triple {
            s: input.id()?,
            p: input.id()?,
            o: input.id()?,
        },
        _ => return Err(SegmentedError::Corrupt("unknown dictionary term")),
    };
    if input.remaining() != 0 {
        return Err(SegmentedError::Corrupt("trailing dictionary term bytes"));
    }
    Ok(term)
}

pub(super) fn record_block(
    records: &[(GlobalTermId, Vec<u8>)],
    block_bytes: usize,
) -> Result<(Vec<u8>, usize), SegmentedError> {
    let raw = raw_record_block(records, block_bytes)?;
    let decoded_limit = block_bytes
        .checked_mul(records.len())
        .ok_or(SegmentedError::AddressExhausted)?;
    let mut count = records.len();
    while count > raw.1 {
        let decoded = records[..count]
            .iter()
            .try_fold(0_usize, |size, (_, key)| {
                size.checked_add(key.len())
                    .ok_or(SegmentedError::AddressExhausted)
            })?;
        if decoded > decoded_limit {
            count = raw.1 + (count - raw.1) / 2;
            continue;
        }
        if records[..count].iter().any(|(_, key)| {
            key.len()
                .checked_add(RECORD_HEADER + 16)
                .is_none_or(|size| size > block_bytes)
        }) {
            return Err(SegmentedError::ConstructionLimit);
        }
        let mut previous = &[][..];
        let mut framed = RECORD_HEADER;
        for (_, key) in &records[..count] {
            let prefix = purrdf_deflate::common_prefix_len(previous, key);
            framed = framed
                .checked_add(16)
                .and_then(|bytes| bytes.checked_add(key.len() - prefix))
                .ok_or(SegmentedError::AddressExhausted)?;
            previous = key;
        }
        let (candidate, used) = raw_record_block(&records[..count], framed)?;
        let payload = usize::try_from(
            read_u32_le(&candidate, 9)
                .ok_or(SegmentedError::Corrupt("record encoder omitted its header"))?,
        )
        .map_err(|_| SegmentedError::AddressExhausted)?;
        let compressed = purrdf_deflate::deflate::compress(
            &candidate[RECORD_HEADER..RECORD_HEADER + payload],
            purrdf_deflate::Level::DEFAULT,
        );
        if used > raw.1
            && compressed
                .len()
                .checked_add(RECORD_HEADER)
                .is_some_and(|size| size <= block_bytes)
        {
            let mut block = candidate[..RECORD_HEADER].to_vec();
            crate::bytes::put_u32_le(
                &mut block,
                9,
                u32::try_from(compressed.len()).map_err(|_| SegmentedError::AddressExhausted)?
                    | COMPRESSED_RECORDS,
            )
            .expect("record header is present");
            block.extend_from_slice(&compressed);
            block.resize(block_bytes, 0);
            return Ok((block, used));
        }
        // Actual compressed size need not be monotone. Probe deterministic
        // shorter prefixes; every selected block has independently fitted.
        count = raw.1 + (count - raw.1) / 2;
    }
    Ok(raw)
}

fn raw_record_block(
    records: &[(GlobalTermId, Vec<u8>)],
    block_bytes: usize,
) -> Result<(Vec<u8>, usize), SegmentedError> {
    let first = records
        .first()
        .ok_or(SegmentedError::Corrupt("empty record block input"))?
        .1
        .as_slice();
    // Routing reads the first decoded record; no second copy of its key is stored.
    let first_bytes = first
        .len()
        .checked_add(RECORD_HEADER + 16)
        .ok_or(SegmentedError::AddressExhausted)?;
    if first_bytes > block_bytes {
        return Err(SegmentedError::ConstructionLimit);
    }
    let mut out = Vec::with_capacity(block_bytes);
    out.push(DICTIONARY);
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    let mut previous = &[][..];
    let mut count = 0_usize;
    let mut decoded = 0_u32;
    for (id, key) in records {
        let prefix = purrdf_deflate::common_prefix_len(previous, key);
        let next = out
            .len()
            .checked_add(16)
            .and_then(|bytes| bytes.checked_add(key.len() - prefix))
            .ok_or(SegmentedError::AddressExhausted)?;
        if next > block_bytes {
            break;
        }
        decoded = decoded
            .checked_add(u32::try_from(key.len()).map_err(|_| SegmentedError::AddressExhausted)?)
            .ok_or(SegmentedError::AddressExhausted)?;
        out.extend_from_slice(
            &u32::try_from(prefix)
                .map_err(|_| SegmentedError::AddressExhausted)?
                .to_le_bytes(),
        );
        out.extend_from_slice(
            &u32::try_from(key.len() - prefix)
                .map_err(|_| SegmentedError::AddressExhausted)?
                .to_le_bytes(),
        );
        push_id(&mut out, *id);
        out.extend_from_slice(&key[prefix..]);
        previous = key;
        count += 1;
    }
    crate::bytes::put_u32_le(
        &mut out,
        1,
        u32::try_from(count).map_err(|_| SegmentedError::AddressExhausted)?,
    )
    .expect("record header is present");
    crate::bytes::put_u32_le(&mut out, 5, decoded).expect("record header is present");
    let payload =
        u32::try_from(out.len() - RECORD_HEADER).map_err(|_| SegmentedError::AddressExhausted)?;
    crate::bytes::put_u32_le(&mut out, 9, payload).expect("record header is present");
    out.resize(block_bytes, 0);
    Ok((out, count))
}

#[derive(Clone, Copy, Debug)]
pub(super) struct OverflowSpan {
    pub first: u64,
    pub bytes: u32,
}

impl OverflowSpan {
    pub(super) fn blocks(self, block_bytes: usize) -> Result<u64, SegmentedError> {
        let capacity = block_bytes
            .checked_sub(5)
            .filter(|&n| n > 0)
            .ok_or(SegmentedError::Corrupt("invalid overflow page size"))?;
        Ok(u64::from(self.bytes).div_ceil(capacity as u64))
    }

    fn validate(self, stream: Stream, block_bytes: usize) -> Result<(), SegmentedError> {
        if self.bytes == 0
            || self
                .first
                .checked_add(self.blocks(block_bytes)?)
                .is_none_or(|end| end > stream.blocks)
        {
            return Err(SegmentedError::Corrupt("overflow span escapes its section"));
        }
        Ok(())
    }
}

pub(super) fn spanned_record(
    id: GlobalTermId,
    key: &[u8],
    overflow: &mut Vec<Vec<u8>>,
    block_bytes: usize,
) -> Result<Vec<u8>, SegmentedError> {
    let span = OverflowSpan {
        first: overflow.len() as u64,
        bytes: u32::try_from(key.len()).map_err(|_| SegmentedError::AddressExhausted)?,
    };
    for chunk in key.chunks(block_bytes - 5) {
        let mut page = Vec::with_capacity(block_bytes);
        page.push(OVERFLOW);
        page.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        page.extend_from_slice(chunk);
        page.resize(block_bytes, 0);
        overflow.push(page);
    }
    let mut descriptor = Vec::with_capacity(block_bytes);
    descriptor.push(SPANNED_DICTIONARY);
    descriptor.extend_from_slice(&1_u32.to_le_bytes());
    descriptor.extend_from_slice(&span.bytes.to_le_bytes());
    descriptor.extend_from_slice(&16_u32.to_le_bytes());
    push_id(&mut descriptor, id);
    descriptor.extend_from_slice(&span.first.to_le_bytes());
    descriptor.resize(block_bytes, 0);
    Ok(descriptor)
}

pub(super) fn overflow_descriptor(
    bytes: &[u8],
    stream: Stream,
) -> Result<(GlobalTermId, OverflowSpan), SegmentedError> {
    let (count, decoded, payload) = record_header(bytes, 1, u64::from(u32::MAX))?;
    if bytes[0] != SPANNED_DICTIONARY
        || count != 1
        || payload != 16
        || u64::from(decoded) + (RECORD_HEADER + 16) as u64 <= bytes.len() as u64
    {
        return Err(SegmentedError::Corrupt("noncanonical overflow descriptor"));
    }
    let mut input = Reader::at(bytes, RECORD_HEADER);
    let id = input.id()?;
    let span = OverflowSpan {
        first: input.u64()?,
        bytes: decoded,
    };
    if input.take(input.remaining())?.iter().any(|&byte| byte != 0) {
        return Err(SegmentedError::Corrupt(
            "nonzero overflow descriptor padding",
        ));
    }
    span.validate(stream, bytes.len())?;
    Ok((id, span))
}

/// Visit authenticated page payloads. A stopped visit is a selective read, never
/// a certification of unvisited bytes; certification always visits the full span.
pub(super) fn visit_overflow(
    span: OverflowSpan,
    stream: Stream,
    block_bytes: usize,
    mut read: impl FnMut(u64, &mut [u8]) -> Result<(), SegmentedError>,
    mut visit: impl FnMut(&[u8]) -> core::ops::ControlFlow<()>,
) -> Result<core::ops::ControlFlow<()>, SegmentedError> {
    span.validate(stream, block_bytes)?;
    let mut resident = purrdf_lex::allocation::Resident;
    let mut memory = purrdf_lex::allocation::Memory::new(&mut resident);
    let mut page = Vec::new();
    memory.reserve(&mut page, block_bytes)?;
    page.resize(block_bytes, 0);
    let mut remaining = span.bytes as usize;
    for offset in 0..span.blocks(block_bytes)? {
        let slot = stream
            .first
            .checked_add(span.first)
            .and_then(|n| n.checked_add(offset))
            .ok_or(SegmentedError::AddressExhausted)?;
        read(slot, &mut page)?;
        let expected = remaining.min(block_bytes - 5);
        if page[0] != OVERFLOW
            || read_u32_le(&page, 1) != Some(expected as u32)
            || page[5 + expected..].iter().any(|&byte| byte != 0)
        {
            return Err(SegmentedError::Corrupt("noncanonical overflow page"));
        }
        if visit(&page[5..5 + expected]).is_break() {
            memory.release_vec(page)?;
            return Ok(core::ops::ControlFlow::Break(()));
        }
        remaining -= expected;
    }
    memory.release_vec(page)?;
    Ok(core::ops::ControlFlow::Continue(()))
}

fn record_header(
    bytes: &[u8],
    max_records: u32,
    max_decoded: u64,
) -> Result<(u32, u32, u32), SegmentedError> {
    let mut input = Reader::at(bytes, 0);
    if !matches!(input.byte()?, DICTIONARY | SPANNED_DICTIONARY) {
        return Err(SegmentedError::Corrupt("wrong record block kind"));
    }
    let count = input.u32()?;
    let decoded = input.u32()?;
    let payload = input.u32()?;
    if count == 0 || count > max_records || u64::from(decoded) > max_decoded {
        return Err(SegmentedError::Corrupt("record decode bound exceeded"));
    }
    Ok((count, decoded, payload))
}

pub(super) fn decode_memory_bytes(
    bytes: &[u8],
    max_records: u32,
    max_decoded: u64,
) -> Result<u64, SegmentedError> {
    let (count, decoded, _) = record_header(bytes, max_records, max_decoded)?;
    let entry_bytes = u64::try_from(size_of::<(GlobalTermId, core::ops::Range<usize>)>())
        .map_err(|_| SegmentedError::AddressExhausted)?;
    u64::from(count)
        .checked_mul(entry_bytes)
        .and_then(|entries| entries.checked_add(u64::from(decoded)))
        .ok_or(SegmentedError::AddressExhausted)
}

pub(super) fn decode_scratch_bytes(
    bytes: &[u8],
    max_records: u32,
    max_decoded: u64,
) -> Result<u64, SegmentedError> {
    let (count, decoded, payload) = record_header(bytes, max_records, max_decoded)?;
    if payload & COMPRESSED_RECORDS == 0 {
        return Ok(0);
    }
    let workspace = u64::try_from(purrdf_deflate::Inflater::max_live_heap_bytes())
        .map_err(|_| SegmentedError::AddressExhausted)?;
    framed_bytes(count, decoded)
        .checked_add(workspace)
        .ok_or(SegmentedError::AddressExhausted)
}

fn framed_bytes(count: u32, decoded: u32) -> u64 {
    u64::from(decoded) + u64::from(count) * 16
}

#[derive(Debug)]
pub(super) struct DecodedRecords {
    pub arena: Vec<u8>,
    pub entries: Vec<(GlobalTermId, core::ops::Range<usize>)>,
}

impl DecodedRecords {
    /// Capacities are bounded by the authenticated header and reserved by the
    /// session before this function allocates either collection.
    pub(super) fn decode(
        bytes: &[u8],
        max_records: u32,
        max_decoded: u64,
        overflow: Stream,
        mut read: impl FnMut(u64, &mut [u8]) -> Result<(), SegmentedError>,
    ) -> Result<Self, SegmentedError> {
        let (count, decoded, payload) = record_header(bytes, max_records, max_decoded)?;
        let span = if bytes[0] == SPANNED_DICTIONARY {
            Some(overflow_descriptor(bytes, overflow)?)
        } else {
            None
        };
        let end = RECORD_HEADER
            .checked_add(
                usize::try_from(payload & !COMPRESSED_RECORDS)
                    .map_err(|_| SegmentedError::AddressExhausted)?,
            )
            .ok_or(SegmentedError::AddressExhausted)?;
        let stored = bytes
            .get(RECORD_HEADER..end)
            .ok_or(SegmentedError::Corrupt("truncated dictionary payload"))?;
        if bytes[end..].iter().any(|&byte| byte != 0) {
            return Err(SegmentedError::Corrupt(
                "non-canonical record block length/padding",
            ));
        }
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = purrdf_lex::allocation::Memory::new(&mut resident);
        let mut inflated = Vec::new();
        let framed = if payload & COMPRESSED_RECORDS != 0 {
            let bound = framed_bytes(count, decoded);
            memory.reserve(
                &mut inflated,
                usize::try_from(bound).map_err(|_| SegmentedError::AddressExhausted)?,
            )?;
            purrdf_deflate::inflate::decompress_into(stored, &mut inflated, bound)
                .map_err(SegmentedError::Compression)?;
            inflated.as_slice()
        } else {
            stored
        };
        let mut input = Reader::at(framed, 0);
        let mut arena = Vec::new();
        memory.reserve(
            &mut arena,
            usize::try_from(decoded).map_err(|_| SegmentedError::AddressExhausted)?,
        )?;
        let mut entries: Vec<(GlobalTermId, core::ops::Range<usize>)> = Vec::new();
        memory.reserve(
            &mut entries,
            usize::try_from(count).map_err(|_| SegmentedError::AddressExhausted)?,
        )?;
        if let Some((id, span)) = span {
            let completed = visit_overflow(span, overflow, bytes.len(), &mut read, |chunk| {
                arena.extend_from_slice(chunk);
                core::ops::ControlFlow::Continue(())
            })?;
            if completed.is_break() {
                return Err(SegmentedError::Corrupt(
                    "incomplete overflow materialization",
                ));
            }
            if arena.len() != decoded as usize {
                return Err(SegmentedError::Corrupt("false expanded dictionary size"));
            }
            entries.push((id, 0..arena.len()));
            return Ok(Self { arena, entries });
        }
        for _ in 0..count {
            let prefix =
                usize::try_from(input.u32()?).map_err(|_| SegmentedError::AddressExhausted)?;
            let suffix =
                usize::try_from(input.u32()?).map_err(|_| SegmentedError::AddressExhausted)?;
            let id = input.id()?;
            let start = arena.len();
            let next = start
                .checked_add(prefix)
                .and_then(|v| v.checked_add(suffix))
                .ok_or(SegmentedError::AddressExhausted)?;
            if next > usize::try_from(decoded).map_err(|_| SegmentedError::AddressExhausted)? {
                return Err(SegmentedError::Corrupt("decoded record size is false"));
            }
            if let Some((_, previous)) = entries.last() {
                if prefix > previous.len() {
                    return Err(SegmentedError::Corrupt(
                        "front-code prefix exceeds previous record",
                    ));
                }
                arena.extend_from_within(previous.start..previous.start + prefix);
            } else if prefix != 0 {
                return Err(SegmentedError::Corrupt("first record has a prefix"));
            }
            arena.extend_from_slice(input.take(suffix)?);
            entries.push((id, start..next));
        }
        if arena.len() != usize::try_from(decoded).map_err(|_| SegmentedError::AddressExhausted)?
            || input.remaining() != 0
        {
            return Err(SegmentedError::Corrupt(
                "non-canonical record block length/padding",
            ));
        }
        Ok(Self { arena, entries })
    }
    pub(super) fn key(&self, index: usize) -> &[u8] {
        &self.arena[self.entries[index].1.clone()]
    }
}

pub(super) fn encode_quad(quad: QuadIds<GlobalTermId>, out: &mut Vec<u8>) {
    for id in [quad.s, quad.p, quad.o] {
        push_id(out, id);
    }
    out.extend_from_slice(&quad.g.map_or(0, |id| id.index() + 1).to_le_bytes());
}
pub(super) fn decode_quad(bytes: &[u8]) -> Result<QuadIds<GlobalTermId>, SegmentedError> {
    let mut input = Reader::at(bytes, 0);
    let (s, p, o) = (input.id()?, input.id()?, input.id()?);
    let graph: u64 = input.u64()?;
    let g = if graph == 0 {
        None
    } else {
        Some(GlobalTermId::from_index(graph - 1).expect("stored term index is representable"))
    };
    Ok(QuadIds { s, p, o, g })
}

pub(super) fn leaf_digest(index: u64, bytes: &[u8]) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(SEGMENTED_BLOCK_DOMAIN.as_bytes());
    hash.update(&index.to_le_bytes());
    hash.update(bytes);
    *hash.finalize().as_bytes()
}
pub(super) fn node_digest(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(SEGMENTED_NODE_DOMAIN.as_bytes());
    hash.update(left);
    hash.update(right);
    *hash.finalize().as_bytes()
}
pub(super) fn snapshot_digest(root: &[u8; 32], block_bytes: u32, count: u64) -> SegmentedSnapshot {
    let mut hash = Hasher::new();
    hash.update(SEGMENTED_SNAPSHOT_DOMAIN.as_bytes());
    hash.update(root);
    hash.update(&block_bytes.to_le_bytes());
    hash.update(&count.to_le_bytes());
    SegmentedSnapshot(*hash.finalize().as_bytes())
}

#[cfg(test)]
#[path = "../../../tests/support/segmented_text.rs"]
mod text_fixture;

#[cfg(test)]
mod tests {
    use super::*;

    fn no_external_pages(_: u64, _: &mut [u8]) -> Result<(), SegmentedError> {
        Err(SegmentedError::Corrupt(
            "inline fixture requested an external page",
        ))
    }

    #[test]
    fn compressed_dictionary_refusals_retain_the_native_codec_cause() {
        let mut bytes = vec![0; 512];
        bytes[0] = DICTIONARY;
        crate::bytes::put_u32_le(&mut bytes, 1, 1).unwrap();
        crate::bytes::put_u32_le(&mut bytes, 5, 1).unwrap();
        crate::bytes::put_u32_le(&mut bytes, 9, COMPRESSED_RECORDS | 1).unwrap();
        bytes[RECORD_HEADER] = 7; // BFINAL=1, reserved BTYPE=11.
        let error = DecodedRecords::decode(&bytes, 1, 512, Stream::default(), no_external_pages)
            .unwrap_err();
        assert_eq!(
            error,
            SegmentedError::Compression(purrdf_deflate::Error::InvalidBlockType)
        );
        assert_eq!(
            std::error::Error::source(&error)
                .unwrap()
                .downcast_ref::<purrdf_deflate::Error>(),
            Some(&purrdf_deflate::Error::InvalidBlockType)
        );
        assert_eq!(
            decode_scratch_bytes(&bytes, 1, 512).unwrap(),
            17 + u64::try_from(purrdf_deflate::Inflater::max_live_heap_bytes()).unwrap()
        );
        crate::bytes::put_u32_le(&mut bytes, 5, 513).unwrap();
        assert_eq!(
            decode_scratch_bytes(&bytes, 1, 512),
            Err(SegmentedError::Corrupt("record decode bound exceeded"))
        );
    }

    #[test]
    fn long_nonprefix_text_records_share_one_compressed_dictionary_block() {
        let text = text_fixture::ARCHIVE_TEXT.repeat(12).into_bytes();
        let records: Vec<_> = (0..64)
            .map(|index| {
                let mut key = vec![u8::try_from(index).unwrap()];
                key.extend_from_slice(&text);
                (GlobalTermId::from_index(100 + index).unwrap(), key)
            })
            .collect();
        let (bytes, used) = record_block(&records, 4096).expect("compressible text fits");
        assert_eq!(
            used,
            records.len(),
            "prefix coding cannot share these leading bytes"
        );
        assert_eq!(bytes.len(), 4096);
        let decoded =
            DecodedRecords::decode(&bytes, 64, 4096 * 64, Stream::default(), no_external_pages)
                .unwrap();
        for (index, (id, key)) in records.iter().enumerate() {
            assert_eq!(decoded.entries[index].0, *id);
            assert_eq!(decoded.key(index), key);
        }
    }

    #[test]
    fn record_fit_stores_the_first_key_once_and_counts_actual_front_coding() {
        let mut second = vec![b'a'; 200];
        second[199] = b'b';
        let records = [
            (GlobalTermId::from_index(12).unwrap(), vec![b'a'; 200]),
            (GlobalTermId::from_index(13).unwrap(), second),
            (GlobalTermId::from_index(14).unwrap(), vec![b'z'; 200]),
        ];
        // 13 header + 16 framing + 200 first key = 229; no duplicate fence.
        // The next key shares 199 bytes, so adds exactly 17 bytes.
        for (width, expected) in [(229, 1), (245, 1), (246, 2), (461, 2), (462, 3), (512, 3)] {
            let (bytes, used) = raw_record_block(&records, width).unwrap();
            assert_eq!(bytes.len(), width);
            assert_eq!(used, expected);
            assert_eq!(
                bytes
                    .windows(200)
                    .filter(|key| *key == records[0].1)
                    .count(),
                1
            );
            let decoded =
                DecodedRecords::decode(&bytes, 3, 600, Stream::default(), no_external_pages)
                    .unwrap();
            assert_eq!(decoded.entries.len(), expected);
            for (index, (id, key)) in records[..used].iter().enumerate() {
                assert_eq!(decoded.entries[index].0, *id);
                assert_eq!(decoded.key(index), key);
            }
        }
        assert!(matches!(
            raw_record_block(&records, 228),
            Err(SegmentedError::ConstructionLimit)
        ));
        let (bytes, used) = raw_record_block(&records[..1], 512).unwrap();
        assert_eq!(used, 1);
        assert_eq!(
            DecodedRecords::decode(&bytes, 1, 200, Stream::default(), no_external_pages)
                .unwrap()
                .key(0),
            records[0].1
        );
    }
}
