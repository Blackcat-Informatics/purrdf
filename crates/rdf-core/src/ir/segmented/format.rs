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

/// The written format version: source checkpoints hold canonical Merkle nodes.
pub(super) const VERSION: u32 = 8;
/// The readable earlier version, whose source checkpoints are additive sums.
pub(super) const ADDITIVE_VERSION: u32 = 7;

const fn magic(version: u32) -> &'static [u8; 8] {
    if version == ADDITIVE_VERSION {
        b"PURRSEG7"
    } else {
        b"PURRSEG8"
    }
}
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
    pub version: u32,
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
        out.extend_from_slice(magic(self.version));
        out.extend_from_slice(&self.version.to_le_bytes());
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
        let version = match read_u32_le(bytes, 8) {
            Some(version @ (ADDITIVE_VERSION | VERSION))
                if bytes.get(..8) == Some(magic(version).as_slice()) =>
            {
                version
            }
            _ => {
                return Err(SegmentedError::Corrupt(
                    "unsupported segmented format/profile; regenerate the image from RDF input",
                ));
            }
        };
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
            version,
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
        let checkpoints = header.source_checkpoints.rows;
        let (count, width) = if version == ADDITIVE_VERSION {
            (
                checkpoints
                    == header.effective_directory.rows
                        / super::super::source_directory::CHECKPOINT_ROWS as u64,
                32,
            )
        } else {
            // Each checkpoint is a distinct branch of at least two members.
            (
                checkpoints < header.effective_directory.rows.max(1),
                super::super::source_directory::NODE_CHECKPOINT_BYTES as u64,
            )
        };
        if !count
            || header.source_checkpoints.blocks
                != checkpoints.div_ceil((u64::from(block_bytes) - 9) / width)
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

fn text_length(value: &str) -> Result<[u8; 4], SegmentedError> {
    Ok(u32::try_from(value.len())
        .map_err(|_| SegmentedError::AddressExhausted)?
        .to_le_bytes())
}

pub(super) fn id_bytes(id: GlobalTermId) -> [u8; 8] {
    (id.index() + 1).to_le_bytes()
}

pub(super) fn encoded_term_size<Id>(term: &TermRef<'_, Id>) -> Result<usize, SegmentedError> {
    match term {
        TermRef::Iri(iri) => iri.len().checked_add(5),
        TermRef::Blank { label, .. } => label.len().checked_add(9),
        TermRef::Literal {
            lexical, language, ..
        } => lexical
            .len()
            .checked_add(language.map_or(0, str::len))
            .and_then(|v| v.checked_add(if language.is_some() { 19 } else { 15 })),
        TermRef::Triple { .. } => Some(25),
    }
    .ok_or(SegmentedError::AddressExhausted)
}

/// Wire fragments borrow text from its original owner, including across pages.
pub(super) struct TermEncoding<'a> {
    prefix: [u8; 25],
    prefix_len: usize,
    text: &'a [u8],
    suffix: [u8; 5],
    suffix_len: usize,
    language: &'a [u8],
    direction: [u8; 1],
}

impl<'a> TermEncoding<'a> {
    pub(super) fn new(term: TermRef<'a, GlobalTermId>) -> Result<Self, SegmentedError> {
        let mut encoding = Self {
            prefix: [0; 25],
            prefix_len: 0,
            text: &[],
            suffix: [0; 5],
            suffix_len: 0,
            language: &[],
            direction: [0],
        };
        match term {
            TermRef::Iri(iri) => {
                encoding.prefix[1..5].copy_from_slice(&text_length(iri)?);
                encoding.prefix_len = 5;
                encoding.text = iri.as_bytes();
            }
            TermRef::Blank { label, scope } => {
                encoding.prefix[0] = 1;
                crate::bytes::put_u32_le(&mut encoding.prefix, 1, scope.ordinal())
                    .expect("fixed blank scope field");
                encoding.prefix[5..9].copy_from_slice(&text_length(label)?);
                encoding.prefix_len = 9;
                encoding.text = label.as_bytes();
            }
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                encoding.prefix[0] = 2;
                encoding.prefix[1..9].copy_from_slice(&id_bytes(datatype));
                encoding.prefix[9..13].copy_from_slice(&text_length(lexical)?);
                encoding.prefix_len = 13;
                encoding.text = lexical.as_bytes();
                encoding.suffix_len = 1;
                if let Some(language) = language {
                    encoding.suffix[0] = 1;
                    encoding.suffix[1..5].copy_from_slice(&text_length(language)?);
                    encoding.suffix_len = 5;
                    encoding.language = language.as_bytes();
                }
                encoding.direction[0] = match direction {
                    None => 0,
                    Some(RdfTextDirection::Ltr) => 1,
                    Some(RdfTextDirection::Rtl) => 2,
                };
            }
            TermRef::Triple { s, p, o } => {
                encoding.prefix[0] = 3;
                for (dst, id) in encoding.prefix[1..]
                    .as_chunks_mut::<8>()
                    .0
                    .iter_mut()
                    .zip([s, p, o])
                {
                    *dst = id_bytes(id);
                }
                encoding.prefix_len = 25;
            }
        }
        Ok(encoding)
    }

    pub(super) fn parts(&self) -> [&[u8]; 5] {
        [
            &self.prefix[..self.prefix_len],
            self.text,
            &self.suffix[..self.suffix_len],
            self.language,
            &self.direction[..usize::from(self.suffix_len != 0)],
        ]
    }
}

pub(super) fn encode_term(
    term: TermRef<'_, GlobalTermId>,
    out: &mut Vec<u8>,
) -> Result<(), SegmentedError> {
    for part in TermEncoding::new(term)?.parts() {
        out.extend_from_slice(part);
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

/// A wire key borrows lexical text without gathering it into a flat array.
pub(super) enum WireKey<'a> {
    Term(TermEncoding<'a>),
    #[cfg(test)]
    Flat(&'a [u8]),
}

impl WireKey<'_> {
    pub(super) fn parts(&self) -> [&[u8]; 5] {
        match self {
            Self::Term(term) => term.parts(),
            #[cfg(test)]
            Self::Flat(bytes) => [bytes, &[], &[], &[], &[]],
        }
    }

    fn len(&self) -> usize {
        self.parts().into_iter().map(<[u8]>::len).sum()
    }

    fn suffix(&self, mut skip: usize) -> impl Iterator<Item = &[u8]> {
        self.parts().into_iter().filter_map(move |part| {
            let offset = skip.min(part.len());
            skip -= offset;
            (offset < part.len()).then(|| &part[offset..])
        })
    }
}

/// Return the equal prefix length and order of the concatenated, not padded, parts.
pub(super) fn compare_key_parts<'a, 'b>(
    left: impl Iterator<Item = &'a [u8]>,
    right: impl Iterator<Item = &'b [u8]>,
) -> (usize, core::cmp::Ordering) {
    let mut left = left.filter(|part| !part.is_empty());
    let mut right = right.filter(|part| !part.is_empty());
    let mut a = &[][..];
    let mut b = &[][..];
    let mut equal = 0;
    loop {
        if a.is_empty() {
            a = left.next().unwrap_or_default();
        }
        if b.is_empty() {
            b = right.next().unwrap_or_default();
        }
        if a.is_empty() || b.is_empty() {
            return (equal, a.len().cmp(&b.len()));
        }
        let width = a.len().min(b.len());
        let prefix = purrdf_deflate::common_prefix_len(&a[..width], &b[..width]);
        equal += prefix;
        if prefix != width {
            return (equal, a[prefix].cmp(&b[prefix]));
        }
        a = &a[width..];
        b = &b[width..];
    }
}

type WireRecord<'a> = Result<(GlobalTermId, WireKey<'a>), SegmentedError>;

/// The selected prefix and its packing, independent of any output buffer.
pub(super) struct RecordLayout {
    pub used: usize,
    pub compressed: bool,
}

pub(super) fn plan_record_block<'a>(
    records: &(impl ExactSizeIterator<Item = WireRecord<'a>> + Clone),
    block_bytes: usize,
) -> Result<RecordLayout, SegmentedError> {
    let raw = raw_records(records.clone(), block_bytes, |_, _, _, _| {})?;
    let decoded_limit = block_bytes
        .checked_mul(records.len())
        .ok_or(SegmentedError::AddressExhausted)?;
    let mut count = records.len();
    while count > raw.count {
        let decoded = records
            .clone()
            .take(count)
            .try_fold(0_usize, |size, record| {
                let (_, key) = record?;
                let len = key.len();
                if len
                    .checked_add(RECORD_HEADER + 16)
                    .is_none_or(|width| width > block_bytes)
                {
                    return Err(SegmentedError::ConstructionLimit);
                }
                size.checked_add(len)
                    .ok_or(SegmentedError::AddressExhausted)
            })?;
        if decoded > decoded_limit {
            count = raw.count + (count - raw.count) / 2;
            continue;
        }
        let compressed = compress_records(records.clone().take(count), |_, _| Ok(()))?;
        if compressed.count > raw.count
            && compressed
                .payload
                .checked_add(RECORD_HEADER)
                .is_some_and(|size| size <= block_bytes)
        {
            return Ok(RecordLayout {
                used: compressed.count,
                compressed: true,
            });
        }
        // Actual compressed size need not be monotone. Probe deterministic
        // shorter prefixes; every selected block has independently fitted.
        count = raw.count + (count - raw.count) / 2;
    }
    Ok(RecordLayout {
        used: raw.count,
        compressed: false,
    })
}

pub(super) fn replay_record_block<'a>(
    records: impl ExactSizeIterator<Item = WireRecord<'a>> + Clone,
    compressed: bool,
    destination: &mut [u8],
) -> Result<(), SegmentedError> {
    let expected = records.len();
    let capacity = destination.len();
    let (header, payload) =
        destination
            .split_at_mut_checked(RECORD_HEADER)
            .ok_or(SegmentedError::Corrupt(
                "dictionary record header does not fit",
            ))?;
    let layout = if compressed {
        compress_records(records, |position, bytes| {
            let target = payload
                .get_mut(position..)
                .and_then(|tail| tail.get_mut(..bytes.len()))
                .ok_or(SegmentedError::Corrupt(
                    "selected compressed record exceeds its page",
                ))?;
            target.copy_from_slice(bytes);
            Ok(())
        })?
    } else {
        raw_records(records, capacity, |position, fields, key, prefix| {
            write_record_payload(payload, position, fields, key, prefix);
        })?
    };
    if layout.count != expected {
        return Err(SegmentedError::Corrupt(
            "selected dictionary prefix changed during emission",
        ));
    }
    write_record_header(header, layout, compressed)
}

/// The frame prefix describes one predecessor and is shared by raw and compressed pages.
fn record_fields(
    prefix: usize,
    suffix: usize,
    id: GlobalTermId,
) -> Result<[u8; 16], SegmentedError> {
    let mut fields = [0; 16];
    let prefix = u32::try_from(prefix).map_err(|_| SegmentedError::AddressExhausted)?;
    let suffix = u32::try_from(suffix).map_err(|_| SegmentedError::AddressExhausted)?;
    crate::bytes::put_u32_le(&mut fields, 0, prefix).expect("fixed prefix field");
    crate::bytes::put_u32_le(&mut fields, 4, suffix).expect("fixed suffix field");
    fields[8..].copy_from_slice(&id_bytes(id));
    Ok(fields)
}

#[derive(Clone, Copy, Default)]
struct RecordContents {
    count: usize,
    decoded: u32,
    payload: usize,
}

impl RecordContents {
    fn record(&mut self, width: usize) -> Result<(), SegmentedError> {
        self.decoded = self
            .decoded
            .checked_add(u32::try_from(width).map_err(|_| SegmentedError::AddressExhausted)?)
            .ok_or(SegmentedError::AddressExhausted)?;
        self.count += 1;
        Ok(())
    }
}

fn write_record_header(
    header: &mut [u8],
    layout: RecordContents,
    compressed: bool,
) -> Result<(), SegmentedError> {
    header[0] = DICTIONARY;
    crate::bytes::put_u32_le(
        header,
        1,
        u32::try_from(layout.count).map_err(|_| SegmentedError::AddressExhausted)?,
    )
    .expect("record header is present");
    crate::bytes::put_u32_le(header, 5, layout.decoded).expect("record header is present");
    let payload = u32::try_from(layout.payload).map_err(|_| SegmentedError::AddressExhausted)?;
    let flags = if compressed { COMPRESSED_RECORDS } else { 0 };
    crate::bytes::put_u32_le(header, 9, payload | flags).expect("record header is present");
    Ok(())
}

/// Run one DEFLATE stream during sizing or writing without owning a full framed
/// payload. Ordinary DEFLATE writes are chunk invariant. Never flush between records.
fn compress_records<'a>(
    records: impl Iterator<Item = WireRecord<'a>>,
    mut write: impl FnMut(usize, &[u8]) -> Result<(), SegmentedError>,
) -> Result<RecordContents, SegmentedError> {
    let mut deflater = purrdf_deflate::deflate::Deflater::new(purrdf_deflate::Level::DEFAULT);
    let mut staged = Vec::new();
    let mut written = 0_usize;
    let mut transfer = |bytes: &mut Vec<u8>| -> Result<(), SegmentedError> {
        let end = written
            .checked_add(bytes.len())
            .ok_or(SegmentedError::AddressExhausted)?;
        write(written, bytes)?;
        written = end;
        bytes.clear();
        Ok(())
    };
    let mut layout = RecordContents::default();
    let mut framed = 0_u32;
    let mut previous = None;
    for record in records {
        let (id, key) = record?;
        let prefix = key_prefix(previous.as_ref(), &key);
        let width = key.len();
        let suffix = width - prefix;
        let fields = record_fields(prefix, suffix, id)?;
        framed = framed
            .checked_add(16)
            .and_then(|n| n.checked_add(u32::try_from(suffix).ok()?))
            .ok_or(SegmentedError::AddressExhausted)?;
        layout.record(width)?;
        deflater.write(&fields, &mut staged);
        transfer(&mut staged)?;
        for part in key.suffix(prefix) {
            for piece in part.chunks(8192) {
                deflater.write(piece, &mut staged);
                transfer(&mut staged)?;
            }
        }
        previous = Some(key);
    }
    deflater.finish(&mut staged);
    transfer(&mut staged)?;
    layout.payload = written;
    Ok(layout)
}

fn raw_records<'a>(
    records: impl Iterator<Item = WireRecord<'a>> + Clone,
    block_bytes: usize,
    mut write: impl FnMut(usize, &[u8; 16], &WireKey<'a>, usize),
) -> Result<RecordContents, SegmentedError> {
    let (_, first) = records
        .clone()
        .next()
        .ok_or(SegmentedError::Corrupt("empty record block input"))??;
    // Routing reads the first decoded record and stores no second key.
    let first_bytes = first
        .len()
        .checked_add(RECORD_HEADER + 16)
        .ok_or(SegmentedError::AddressExhausted)?;
    if first_bytes > block_bytes {
        return Err(SegmentedError::ConstructionLimit);
    }
    let capacity = block_bytes - RECORD_HEADER;
    let mut layout = RecordContents::default();
    let mut previous = None;
    for record in records {
        let (id, key) = record?;
        let prefix = key_prefix(previous.as_ref(), &key);
        let width = key.len();
        let suffix = width - prefix;
        let next = layout
            .payload
            .checked_add(16)
            .and_then(|bytes| bytes.checked_add(suffix))
            .ok_or(SegmentedError::AddressExhausted)?;
        if next > capacity {
            break;
        }
        let fields = record_fields(prefix, suffix, id)?;
        layout.record(width)?;
        write(layout.payload, &fields, &key, prefix);
        layout.payload = next;
        previous = Some(key);
    }
    Ok(layout)
}

fn key_prefix(previous: Option<&WireKey<'_>>, key: &WireKey<'_>) -> usize {
    previous.map_or(0, |prior| {
        compare_key_parts(prior.parts().into_iter(), key.parts().into_iter()).0
    })
}

fn write_record_payload(
    payload: &mut [u8],
    position: usize,
    fields: &[u8; 16],
    key: &WireKey<'_>,
    prefix: usize,
) {
    let mut tail = &mut payload[position..];
    let (header, rest) = tail.split_at_mut(fields.len());
    header.copy_from_slice(fields);
    tail = rest;
    for part in key.suffix(prefix) {
        let (destination, rest) = tail.split_at_mut(part.len());
        destination.copy_from_slice(part);
        tail = rest;
    }
}

#[cfg(test)]
fn record_block<'a>(
    records: &(impl ExactSizeIterator<Item = (GlobalTermId, &'a [u8])> + Clone),
    block_bytes: usize,
) -> Result<(Vec<u8>, usize), SegmentedError> {
    let records = records
        .clone()
        .map(|(id, key)| Ok((id, WireKey::Flat(key))));
    let layout = plan_record_block(&records, block_bytes)?;
    let mut block = vec![0; block_bytes];
    replay_record_block(
        records.clone().take(layout.used),
        layout.compressed,
        &mut block,
    )?;
    Ok((block, layout.used))
}

#[cfg(test)]
fn raw_record_block<'a>(
    records: impl Iterator<Item = (GlobalTermId, &'a [u8])> + Clone,
    block_bytes: usize,
) -> Result<(Vec<u8>, usize), SegmentedError> {
    let mut block = vec![0; block_bytes];
    let records = records.map(|(id, key)| Ok((id, WireKey::Flat(key))));
    let layout = raw_records(records, block_bytes, |position, fields, key, prefix| {
        write_record_payload(&mut block[RECORD_HEADER..], position, fields, key, prefix);
    })?;
    let count = layout.count;
    write_record_header(&mut block[..RECORD_HEADER], layout, false)?;
    Ok((block, count))
}

#[derive(Clone, Copy, Debug)]
pub(super) struct OverflowSpan {
    pub first: u64,
    pub bytes: u32,
}

impl OverflowSpan {
    pub(super) fn blocks(self, block_bytes: usize) -> Result<u64, SegmentedError> {
        let capacity = block_bytes
            .checked_sub(OVERFLOW_HEADER_BYTES)
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

pub(super) fn write_spanned_record(id: GlobalTermId, span: OverflowSpan, block: &mut [u8]) {
    block[0] = SPANNED_DICTIONARY;
    crate::bytes::put_u32_le(block, 1, 1).expect("overflow record count");
    crate::bytes::put_u32_le(block, 5, span.bytes).expect("overflow decoded length");
    crate::bytes::put_u32_le(block, 9, 16).expect("overflow payload length");
    block[RECORD_HEADER..RECORD_HEADER + 8].copy_from_slice(&id_bytes(id));
    crate::bytes::put_u64_le(block, RECORD_HEADER + 8, span.first)
        .expect("fixed first overflow page field");
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

pub(super) const OVERFLOW_HEADER_BYTES: usize = 5;

pub(super) fn overflow_payload(page: &[u8], width: usize) -> Result<&[u8], SegmentedError> {
    let end = OVERFLOW_HEADER_BYTES
        .checked_add(width)
        .ok_or(SegmentedError::AddressExhausted)?;
    let Some((record, padding)) = page.split_at_checked(end) else {
        return Err(SegmentedError::Corrupt("noncanonical overflow page"));
    };
    if record[0] != OVERFLOW
        || read_u32_le(record, 1)
            != Some(u32::try_from(width).map_err(|_| SegmentedError::AddressExhausted)?)
        || padding.iter().any(|&byte| byte != 0)
    {
        return Err(SegmentedError::Corrupt("noncanonical overflow page"));
    }
    Ok(&record[OVERFLOW_HEADER_BYTES..])
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
        let expected = remaining.min(block_bytes - OVERFLOW_HEADER_BYTES);
        if visit(overflow_payload(&page, expected)?).is_break() {
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
    let (_, _, payload) = record_header(bytes, max_records, max_decoded)?;
    if payload & COMPRESSED_RECORDS == 0 {
        return Ok(0);
    }
    u64::try_from(purrdf_deflate::Inflater::max_live_heap_bytes())
        .map_err(|_| SegmentedError::AddressExhausted)
}

fn framed_bytes(count: u32, decoded: u32) -> u64 {
    u64::from(decoded) + u64::from(count) * 16
}

struct RecordInput<'a, 'i> {
    stored: &'a [u8],
    inflater: Option<&'i mut purrdf_deflate::Inflater>,
}

impl RecordInput<'_, '_> {
    fn fill(&mut self, out: &mut [u8]) -> Result<(), SegmentedError> {
        if out.is_empty() {
            return Ok(());
        }
        if let Some(inflater) = &mut self.inflater {
            let progress = inflater
                .feed(self.stored, out)
                .map_err(SegmentedError::Compression)?;
            self.stored = &self.stored[progress.consumed..];
            if progress.status == purrdf_deflate::inflate::Status::NeedsInput {
                return Err(SegmentedError::Compression(
                    purrdf_deflate::Error::Truncated,
                ));
            }
            if progress.written == out.len() {
                return Ok(());
            }
        } else if let Some((head, tail)) = self.stored.split_at_checked(out.len()) {
            out.copy_from_slice(head);
            self.stored = tail;
            return Ok(());
        }
        Err(SegmentedError::Corrupt("truncated block record"))
    }

    fn finish(mut self, decoded: usize, observed: usize) -> Result<(), SegmentedError> {
        let extra = if let Some(inflater) = &mut self.inflater {
            let progress = inflater
                .feed(self.stored, &mut [0])
                .map_err(SegmentedError::Compression)?;
            if progress.status == purrdf_deflate::inflate::Status::NeedsInput {
                return Err(SegmentedError::Compression(
                    purrdf_deflate::Error::Truncated,
                ));
            }
            if progress.status == purrdf_deflate::inflate::Status::StreamEnd
                && (progress.consumed != self.stored.len() || !inflater.trailing_bytes().is_empty())
            {
                return Err(SegmentedError::Compression(
                    purrdf_deflate::Error::TrailingData,
                ));
            }
            progress.written != 0
        } else {
            !self.stored.is_empty()
        };
        if extra || decoded != observed {
            Err(SegmentedError::Corrupt(
                "non-canonical record block length/padding",
            ))
        } else {
            Ok(())
        }
    }
}

pub(super) struct InlineFrames<'a, 'i> {
    input: RecordInput<'a, 'i>,
    remaining: u32,
    decoded: usize,
    observed: usize,
    previous: Option<usize>,
}

impl<'a, 'i> InlineFrames<'a, 'i> {
    pub(super) fn open(
        bytes: &'a [u8],
        max_records: u32,
        max_decoded: u64,
        inflater: &'i mut Option<purrdf_deflate::Inflater>,
    ) -> Result<Self, SegmentedError> {
        let (count, decoded, payload) = record_header(bytes, max_records, max_decoded)?;
        if bytes[0] != DICTIONARY {
            return Err(SegmentedError::Corrupt(
                "wrong inline dictionary block kind",
            ));
        }
        Self::new(bytes, count, decoded, payload, inflater)
    }

    fn new(
        bytes: &'a [u8],
        count: u32,
        decoded: u32,
        payload: u32,
        inflater: &'i mut Option<purrdf_deflate::Inflater>,
    ) -> Result<Self, SegmentedError> {
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
        let inflater = (payload & COMPRESSED_RECORDS != 0).then(|| {
            let state = inflater.get_or_insert_with(purrdf_deflate::Inflater::new);
            state.reset();
            state.set_limit(framed_bytes(count, decoded));
            state
        });
        Ok(Self {
            input: RecordInput { stored, inflater },
            remaining: count,
            decoded: usize::try_from(decoded).map_err(|_| SegmentedError::AddressExhausted)?,
            observed: 0,
            previous: None,
        })
    }

    fn next(&mut self) -> Result<Option<(GlobalTermId, usize, usize)>, SegmentedError> {
        if self.remaining == 0 {
            return Ok(None);
        }
        let mut frame = [0; 16];
        self.input.fill(&mut frame)?;
        let mut frame = Reader::at(&frame, 0);
        let prefix = usize::try_from(frame.u32()?).map_err(|_| SegmentedError::AddressExhausted)?;
        let suffix = usize::try_from(frame.u32()?).map_err(|_| SegmentedError::AddressExhausted)?;
        let id = frame.id()?;
        let width = prefix
            .checked_add(suffix)
            .ok_or(SegmentedError::AddressExhausted)?;
        self.observed = self
            .observed
            .checked_add(width)
            .ok_or(SegmentedError::AddressExhausted)?;
        if self.observed > self.decoded {
            return Err(SegmentedError::Corrupt("decoded record size is false"));
        }
        match self.previous {
            Some(previous) if prefix > previous => {
                return Err(SegmentedError::Corrupt(
                    "front-code prefix exceeds previous record",
                ));
            }
            None if prefix != 0 => {
                return Err(SegmentedError::Corrupt("first record has a prefix"));
            }
            _ => {}
        }
        self.previous = Some(width);
        self.remaining -= 1;
        Ok(Some((id, prefix, width)))
    }

    pub(super) fn read_next(
        &mut self,
        key: &mut Vec<u8>,
        memory: &mut purrdf_lex::allocation::Memory<'_, purrdf_lex::allocation::Resident>,
    ) -> Result<Option<GlobalTermId>, SegmentedError> {
        let Some((id, prefix, width)) = self.next()? else {
            return Ok(None);
        };
        key.truncate(prefix);
        if width > key.capacity() {
            let capacity = purrdf_lex::allocation::growth_capacity::<u8>(key.capacity(), width)?;
            memory.reserve(key, capacity)?;
        }
        key.resize(width, 0);
        self.input.fill(&mut key[prefix..])?;
        Ok(Some(id))
    }

    pub(super) fn finish(self) -> Result<(), SegmentedError> {
        if self.remaining != 0 {
            return Err(SegmentedError::Corrupt(
                "incomplete inline dictionary traversal",
            ));
        }
        self.input.finish(self.decoded, self.observed)
    }
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
        let span = (bytes[0] == SPANNED_DICTIONARY)
            .then(|| overflow_descriptor(bytes, overflow))
            .transpose()?;
        let mut inflater = None;
        let frames = if span.is_none() {
            Some(InlineFrames::new(
                bytes,
                count,
                decoded,
                payload,
                &mut inflater,
            )?)
        } else {
            None
        };
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = purrdf_lex::allocation::Memory::new(&mut resident);
        let decoded = usize::try_from(decoded).map_err(|_| SegmentedError::AddressExhausted)?;
        let mut arena = Vec::new();
        memory.reserve(&mut arena, decoded)?;
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
            if arena.len() != decoded {
                return Err(SegmentedError::Corrupt("false expanded dictionary size"));
            }
            entries.push((id, 0..arena.len()));
            return Ok(Self { arena, entries });
        }
        if let Some(mut frames) = frames {
            while let Some((id, prefix, width)) = frames.next()? {
                let start = arena.len();
                if let Some((_, previous)) = entries.last() {
                    arena.extend_from_within(previous.start..previous.start + prefix);
                }
                let suffix_start = arena.len();
                // Frames checked the running decoded width against the admitted total.
                let end = start + width;
                arena.resize(end, 0);
                frames.input.fill(&mut arena[suffix_start..])?;
                entries.push((id, start..end));
            }
            frames.finish()?;
        }
        Ok(Self { arena, entries })
    }
    pub(super) fn key(&self, index: usize) -> &[u8] {
        &self.arena[self.entries[index].1.clone()]
    }
}

pub(super) fn encode_quad(quad: QuadIds<GlobalTermId>, out: &mut [u8; 32]) {
    out[..8].copy_from_slice(&id_bytes(quad.s));
    out[8..16].copy_from_slice(&id_bytes(quad.p));
    out[16..24].copy_from_slice(&id_bytes(quad.o));
    out[24..].copy_from_slice(&quad.g.map_or([0; 8], id_bytes));
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
            u64::try_from(purrdf_deflate::Inflater::max_live_heap_bytes()).unwrap()
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
        let (bytes, used) =
            record_block(&records.iter().map(|(id, key)| (*id, key.as_slice())), 4096)
                .expect("compressible text fits");
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
        let mut bytes = bytes;
        let old_payload = Reader::at(&bytes, 9).u32().unwrap();
        assert_ne!(old_payload & COMPRESSED_RECORDS, 0);
        let mut decode_with_payload = |payload| {
            crate::bytes::put_u32_le(&mut bytes, 9, payload).unwrap();
            DecodedRecords::decode(&bytes, 64, 4096 * 64, Stream::default(), no_external_pages)
                .unwrap_err()
        };
        assert_eq!(
            decode_with_payload(old_payload + 1),
            SegmentedError::Compression(purrdf_deflate::Error::TrailingData)
        );
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
            let (bytes, used) =
                raw_record_block(records.iter().map(|(id, key)| (*id, key.as_slice())), width)
                    .unwrap();
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
            raw_record_block(records.iter().map(|(id, key)| (*id, key.as_slice())), 228),
            Err(SegmentedError::ConstructionLimit)
        ));
        let (bytes, used) = raw_record_block(
            records[..1].iter().map(|(id, key)| (*id, key.as_slice())),
            512,
        )
        .unwrap();
        assert_eq!(used, 1);
        assert_eq!(
            DecodedRecords::decode(&bytes, 1, 200, Stream::default(), no_external_pages)
                .unwrap()
                .key(0),
            records[0].1
        );
    }
}
