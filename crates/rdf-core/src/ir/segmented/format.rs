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

pub(super) const MAGIC: &[u8; 8] = b"PURRSEG2";
pub(super) const HEADER_BYTES: usize = 312;
pub(super) const RECORD_HEADER: usize = 17;
pub(super) const SUMMARY_BYTES: usize = 88;
pub(super) const MAX_RECORDS_PER_BLOCK: u32 = 256;
pub(super) const DICTIONARY: u8 = 1;
pub(super) const REVERSE: u8 = 2;
pub(super) const QUADS: u8 = 3;
pub(super) const GRAPHS: u8 = 4;
pub(super) const SUMMARIES: u8 = 5;

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
    pub statistics: [Stream; 4],
    pub flags: u8,
}

impl Header {
    pub(super) fn encode(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_BYTES);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&2_u32.to_le_bytes());
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
            self.statistics[0],
            self.statistics[1],
            self.statistics[2],
            self.statistics[3],
        ] {
            for value in [stream.first, stream.blocks, stream.rows] {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        out
    }

    pub(super) fn decode(bytes: &[u8]) -> Result<Self, SegmentedError> {
        if bytes.get(..8) != Some(MAGIC.as_slice()) || read_u32_le(bytes, 8) != Some(2) {
            return Err(SegmentedError::Corrupt(
                "unsupported segmented format/profile",
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
        let mut streams = [Stream::default(); 11];
        for stream in &mut streams {
            *stream = Stream {
                first: input.u64()?,
                blocks: input.u64()?,
                rows: input.u64()?,
            };
        }
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
            statistics: [streams[7], streams[8], streams[9], streams[10]],
        };
        if header.dictionary.rows != terms || header.reverse.rows != terms {
            return Err(SegmentedError::Corrupt(
                "dictionary/index cardinality mismatch",
            ));
        }
        let expected = terms / u64::from(records_per_block)
            + u64::from(terms % u64::from(records_per_block) != 0);
        if [header.dictionary, header.reverse]
            .iter()
            .any(|stream| stream.blocks < expected || stream.blocks > terms)
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
            self.statistics[0],
            self.statistics[1],
            self.statistics[2],
            self.statistics[3],
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
    kind: u8,
    records: &[(GlobalTermId, Vec<u8>)],
    block_bytes: usize,
) -> Result<(Vec<u8>, usize), SegmentedError> {
    let fence = records
        .first()
        .ok_or(SegmentedError::Corrupt("empty record block input"))?
        .1
        .as_slice();
    // The first key is also the fence. Refuse it before copying either instance;
    // all following records are admitted at this same encoder before appending.
    let first_bytes = fence
        .len()
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(RECORD_HEADER + 16))
        .ok_or(SegmentedError::AddressExhausted)?;
    if first_bytes > block_bytes {
        return Err(SegmentedError::ConstructionLimit);
    }
    let mut out = Vec::with_capacity(block_bytes);
    out.push(kind);
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(
        &u32::try_from(fence.len())
            .map_err(|_| SegmentedError::AddressExhausted)?
            .to_le_bytes(),
    );
    out.extend_from_slice(fence);
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
        kind: u8,
        max_records: u32,
        max_decoded: u64,
    ) -> Result<Self, SegmentedError> {
        let mut input = Reader::at(bytes, 0);
        if input.byte()? != kind {
            return Err(SegmentedError::Corrupt("wrong record block kind"));
        }
        let count = input.u32()?;
        let decoded = input.u32()?;
        let payload = input.u32()?;
        let fence_len = input.u32()?;
        if count == 0 || count > max_records || u64::from(decoded) > max_decoded {
            return Err(SegmentedError::Corrupt("record decode bound exceeded"));
        }
        let fence = input
            .take(usize::try_from(fence_len).map_err(|_| SegmentedError::AddressExhausted)?)?;
        let mut arena = Vec::with_capacity(
            usize::try_from(decoded).map_err(|_| SegmentedError::AddressExhausted)?,
        );
        let mut entries: Vec<(GlobalTermId, core::ops::Range<usize>)> = Vec::with_capacity(
            usize::try_from(count).map_err(|_| SegmentedError::AddressExhausted)?,
        );
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
        let used = bytes.len() - input.remaining();
        if arena.len() != usize::try_from(decoded).map_err(|_| SegmentedError::AddressExhausted)?
            || used
                != RECORD_HEADER
                    + usize::try_from(payload).map_err(|_| SegmentedError::AddressExhausted)?
            || bytes[used..].iter().any(|&byte| byte != 0)
        {
            return Err(SegmentedError::Corrupt(
                "non-canonical record block length/padding",
            ));
        }
        if arena.get(entries[0].1.clone()) != Some(fence) {
            return Err(SegmentedError::Corrupt(
                "index fence does not name its first record",
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
mod tests {
    use super::*;

    #[test]
    fn record_fit_includes_fence_framing_and_actual_front_coding() {
        let mut second = vec![b'a'; 200];
        second[199] = b'b';
        let records = [
            (GlobalTermId::from_index(12).unwrap(), vec![b'a'; 200]),
            (GlobalTermId::from_index(13).unwrap(), second),
            (GlobalTermId::from_index(14).unwrap(), vec![b'z'; 200]),
        ];
        // 17 header + 200 fence + 16 framing + 200 first key = 433.
        // The next key shares 199 bytes, so adds exactly 17 bytes.
        for (width, expected) in [(433, 1), (449, 1), (450, 2), (512, 2)] {
            let (bytes, used) = record_block(DICTIONARY, &records, width).unwrap();
            assert_eq!(bytes.len(), width);
            assert_eq!(used, expected);
            let decoded = DecodedRecords::decode(&bytes, DICTIONARY, 3, 600).unwrap();
            assert_eq!(decoded.entries.len(), expected);
            for (index, (id, key)) in records[..used].iter().enumerate() {
                assert_eq!(decoded.entries[index].0, *id);
                assert_eq!(decoded.key(index), key);
            }
        }
        assert!(matches!(
            record_block(DICTIONARY, &records, 432),
            Err(SegmentedError::ConstructionLimit)
        ));
        let (bytes, used) = record_block(REVERSE, &records[..1], 512).unwrap();
        assert_eq!(used, 1);
        assert_eq!(
            DecodedRecords::decode(&bytes, REVERSE, 1, 200)
                .unwrap()
                .key(0),
            records[0].1
        );
    }
}
