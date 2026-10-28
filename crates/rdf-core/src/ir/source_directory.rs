// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-local content order, independently of dictionary and row ordinals.
//! Sparse prefix sums let two immutable heads skip unchanged digest ranges.
//! The zero prefix is implicit; only complete nonempty blocks retain a checkpoint.

use std::ops::Range;

use purrdf_hash::multiset::Multiset256;
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_lex::walk::WorkList;

use crate::{StatisticsStream, source::SourceDifferenceWork};

pub(crate) const CHECKPOINT_ROWS: usize = 32;
pub(crate) const RECORD_BYTES: usize = 37;

pub(crate) fn first_differing_bit(a: &[u8; 32], b: &[u8; 32]) -> Option<u16> {
    (0_u16..)
        .zip(a.iter().zip(b))
        .find_map(|(index, (&a, &b))| {
            (a != b).then(|| {
                index * 8
                    + u16::try_from((a ^ b).leading_zeros())
                        .expect("nonzero byte has at most seven leading zero bits")
            })
        })
}

/// The same content-range comparison serves resident and authenticated readers.
/// Addresses remain local to each reader; only value digests are compared.
pub(crate) trait DirectoryRead {
    type Error: From<StorageError>;
    type Record: DirectoryEntry;
    fn len(&self) -> Result<usize, Self::Error>;
    fn record(&self, index: usize) -> Result<Self::Record, Self::Error>;
    fn find(&self, key: [u8; 32]) -> Result<Option<Self::Record>, Self::Error> {
        let (mut low, mut high) = (0, self.len()?);
        while low < high {
            let middle = low + (high - low) / 2;
            let record = self.record(middle)?;
            match record.digest().cmp(&key) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => return Ok(Some(record)),
            }
        }
        Ok(None)
    }
    fn prefix(
        &self,
        end: usize,
        work: &mut SourceDifferenceWork,
    ) -> Result<Multiset256, Self::Error>;
}

pub(crate) trait DirectoryEntry: Copy {
    fn digest(self) -> [u8; 32];
}

impl DirectoryEntry for [u8; 32] {
    fn digest(self) -> [u8; 32] {
        self
    }
}

pub(crate) enum DirectoryChange<A, B> {
    Previous(A),
    Next(B),
}

impl<R> DirectoryChange<R, R> {
    pub(crate) fn parts(self) -> (bool, R) {
        match self {
            Self::Previous(record) => (false, record),
            Self::Next(record) => (true, record),
        }
    }
}

/// The row address is meaningful only in the original frozen carrier. Only its
/// value-derived digest participates in range equality and ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceRecord {
    pub(crate) digest: [u8; 32],
    pub(crate) ordinal: u32,
    pub(crate) stream: StatisticsStream,
}

impl DirectoryEntry for SourceRecord {
    fn digest(self) -> [u8; 32] {
        self.digest
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SourceDirectory {
    records: Box<[SourceRecord]>,
    checkpoints: Box<[Multiset256]>,
}

impl SourceDirectory {
    pub(crate) fn content_records(&self) -> Multiset256 {
        self.prefix(self.records.len(), &mut SourceDifferenceWork::default())
    }

    pub(crate) fn retained_bytes(&self) -> Option<usize> {
        size_of_val(self.records.as_ref()).checked_add(size_of_val(self.checkpoints.as_ref()))
    }

    pub(crate) fn freeze<S: Admission + ?Sized>(
        mut records: Vec<SourceRecord>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        records.sort_unstable_by_key(|record| record.digest);
        let mut checkpoints = Vec::new();
        let checkpoint_count = records.len() / CHECKPOINT_ROWS;
        if checkpoint_count != 0 {
            memory.reserve(&mut checkpoints, checkpoint_count)?;
            let mut state = Multiset256::default();
            for (index, record) in records.iter().enumerate() {
                state
                    .insert(record.digest)
                    .map_err(|_| StorageError::SizeOverflow)?;
                if (index + 1).is_multiple_of(CHECKPOINT_ROWS) {
                    memory.push(&mut checkpoints, state)?;
                }
            }
        }
        Ok(Self {
            records: memory.boxed_slice(records)?,
            checkpoints: memory.boxed_slice(checkpoints)?,
        })
    }

    pub(crate) fn clone_admitted<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let records = memory.collect(self.records.iter().copied())?;
        let checkpoints = memory.collect(self.checkpoints.iter().copied())?;
        Ok(Self {
            records: memory.boxed_slice(records)?,
            checkpoints: memory.boxed_slice(checkpoints)?,
        })
    }

    fn prefix(&self, end: usize, work: &mut SourceDifferenceWork) -> Multiset256 {
        if end == 0 {
            return Multiset256::default();
        }
        let block = end / CHECKPOINT_ROWS;
        let mut sum = if block == 0 {
            Multiset256::default()
        } else {
            self.checkpoints[block - 1]
        };
        for record in &self.records[block * CHECKPOINT_ROWS..end] {
            sum.insert(record.digest)
                .expect("a directory population fits u128");
            work.members += 1;
        }
        sum
    }

    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn checkpoint_sums(&self) -> impl ExactSizeIterator<Item = &[u8; 32]> {
        self.checkpoints.iter().map(Multiset256::sum)
    }

    /// Persist sorted digests and physical addresses without implied populations. Full
    /// certification compares these bytes with metadata rebuilt from RDF rows.
    pub(crate) fn encode_records(&self, start: usize, output: &mut [u8]) {
        assert!(output.len().is_multiple_of(RECORD_BYTES));
        for (index, bytes) in output
            .as_chunks_mut::<RECORD_BYTES>()
            .0
            .iter_mut()
            .enumerate()
        {
            let record = self.records[start + index];
            bytes[..32].copy_from_slice(&record.digest);
            bytes[32..]
                .first_chunk_mut::<4>()
                .expect("fixed source ordinal")
                .copy_from_slice(&record.ordinal.to_le_bytes());
            bytes[36] = match record.stream {
                StatisticsStream::Asserted => 0,
                StatisticsStream::Reifiers => 1,
                StatisticsStream::Annotations => 2,
            };
        }
    }

    /// Visit actual physical changes, without accepting a consumer delta as
    /// evidence. Equal value-digest ranges skip even when insertion shifted all
    /// subsequent ordinals. The heap work list is admitted by the caller.
    pub(crate) fn visit_changes<S: Admission + ?Sized, E: From<StorageError>>(
        &self,
        next: &Self,
        memory: &mut Memory<'_, S>,
        mut visit: impl FnMut(bool, SourceRecord, &mut Memory<'_, S>) -> Result<(), E>,
    ) -> Result<SourceDifferenceWork, E> {
        visit_changes(self, next, memory, |change, memory| {
            let (added, record) = change.parts();
            visit(added, record, memory)
        })
    }
}

impl DirectoryRead for SourceDirectory {
    type Error = StorageError;
    type Record = SourceRecord;
    fn len(&self) -> Result<usize, Self::Error> {
        Ok(self.records.len())
    }
    fn record(&self, index: usize) -> Result<SourceRecord, Self::Error> {
        Ok(self.records[index])
    }
    fn prefix(
        &self,
        end: usize,
        work: &mut SourceDifferenceWork,
    ) -> Result<Multiset256, Self::Error> {
        Ok(self.prefix(end, work))
    }
}

fn summary<D: DirectoryRead, E: From<D::Error> + From<StorageError>>(
    directory: &D,
    range: &Range<usize>,
    work: &mut SourceDifferenceWork,
) -> Result<Multiset256, E> {
    let mut sum = directory.prefix(range.end, work)?;
    sum.subtract(directory.prefix(range.start, work)?)
        .map_err(|_| StorageError::SizeOverflow)?;
    Ok(sum)
}

pub(crate) fn visit_changes<
    A: DirectoryRead,
    B: DirectoryRead,
    S: Admission + ?Sized,
    E: From<StorageError> + From<A::Error> + From<B::Error>,
>(
    previous: &A,
    next: &B,
    memory: &mut Memory<'_, S>,
    mut visit: impl FnMut(DirectoryChange<A::Record, B::Record>, &mut Memory<'_, S>) -> Result<(), E>,
) -> Result<SourceDifferenceWork, E> {
    visit_directory_sets(
        std::slice::from_ref(previous),
        std::slice::from_ref(next),
        memory,
        |_, change, memory| visit(change, memory),
    )
}

/// Compare canonical multisets across any number of sorted carrier directories.
/// Frames hold one range per carrier; no merged row or key array is built.
/// The callback's index belongs to the selected side's directory list.
pub(crate) fn visit_directory_sets<
    A: DirectoryRead,
    B: DirectoryRead,
    S: Admission + ?Sized,
    E: From<StorageError> + From<A::Error> + From<B::Error>,
>(
    previous: &[A],
    next: &[B],
    memory: &mut Memory<'_, S>,
    mut visit: impl FnMut(
        usize,
        DirectoryChange<A::Record, B::Record>,
        &mut Memory<'_, S>,
    ) -> Result<(), E>,
) -> Result<SourceDifferenceWork, E> {
    // Release only our scratch. A failed visitor may retain an independently
    // admitted output vector; refunding a blanket scope would uncharge it.
    let mut pending = WorkList::<_, 0>::new();
    let mut ranges = Vec::new();
    let mut working = Vec::new();
    let result = (|| {
        let width = previous
            .len()
            .checked_add(next.len())
            .ok_or(StorageError::SizeOverflow)?;
        if width == 0 {
            return Ok(SourceDifferenceWork::default());
        }
        for source in previous {
            memory.push(&mut ranges, 0..source.len()?)?;
        }
        for source in next {
            memory.push(&mut ranges, 0..source.len()?)?;
        }
        for range in &ranges {
            memory.push(&mut working, range.clone())?;
        }
        pending.try_push_admitted(0_usize, memory)?;
        let mut work = SourceDifferenceWork::default();
        while let Some(start) = pending.pop() {
            debug_assert_eq!(start + width, ranges.len());
            working.clone_from_slice(&ranges[start..]);
            ranges.truncate(start);
            let (old, new) = working.split_at(previous.len());
            work.ranges += 1;
            let old_sum = group_summary::<_, E>(previous, old, &mut work)?;
            let new_sum = group_summary::<_, E>(next, new, &mut work)?;
            if old_sum == new_sum {
                continue;
            }
            let bounds = if old_sum.population() == 0 || new_sum.population() == 0 {
                None
            } else {
                let a = group_bounds(previous, old)?;
                let b = group_bounds(next, new)?;
                match (a, b) {
                    (Some(a), Some(b)) => {
                        first_differing_bit(&a.first.min(b.first), &a.last.max(b.last))
                    }
                    _ => None,
                }
            };
            let Some(bit) = bounds else {
                // Empty sides and full digest collisions preserve every
                // original physical occurrence, including cross-carrier copies.
                for (source_index, (source, range)) in previous.iter().zip(old).enumerate() {
                    for index in range.clone() {
                        visit(
                            source_index,
                            DirectoryChange::Previous(source.record(index)?),
                            memory,
                        )?;
                        work.changed += 1;
                    }
                }
                for (source_index, (source, range)) in next.iter().zip(new).enumerate() {
                    for index in range.clone() {
                        visit(
                            source_index,
                            DirectoryChange::Next(source.record(index)?),
                            memory,
                        )?;
                        work.changed += 1;
                    }
                }
                continue;
            };
            let byte = usize::from(bit / 8);
            let mask = 0x80_u8 >> (bit % 8);
            let right = ranges.len();
            for (source, range) in previous.iter().zip(&mut working[..previous.len()]) {
                let middle = partition(source, range.clone(), byte, mask)?;
                memory.push(&mut ranges, middle..range.end)?;
                range.end = middle;
            }
            for (source, range) in next.iter().zip(&mut working[previous.len()..]) {
                let middle = partition(source, range.clone(), byte, mask)?;
                memory.push(&mut ranges, middle..range.end)?;
                range.end = middle;
            }
            pending.try_push_admitted(right, memory)?;
            let left = ranges.len();
            for range in &working {
                memory.push(&mut ranges, range.clone())?;
            }
            pending.try_push_admitted(left, memory)?;
        }
        Ok(work)
    })();
    let released = pending.release_admitted(memory);
    let ranges_released = memory.release_vec(ranges);
    let working_released = memory.release_vec(working);
    match result {
        Err(error) => Err(error),
        Ok(work) => {
            released?;
            ranges_released?;
            working_released?;
            Ok(work)
        }
    }
}

fn group_summary<D: DirectoryRead, E: From<D::Error> + From<StorageError>>(
    sources: &[D],
    ranges: &[Range<usize>],
    work: &mut SourceDifferenceWork,
) -> Result<Multiset256, E> {
    let mut sum = Multiset256::default();
    for (source, range) in sources
        .iter()
        .zip(ranges)
        .filter(|(_, range)| !range.is_empty())
    {
        sum.combine(summary::<_, E>(source, range, work)?)
            .map_err(|_| StorageError::SizeOverflow)?;
    }
    Ok(sum)
}

struct DigestBounds {
    first: [u8; 32],
    last: [u8; 32],
}

fn group_bounds<D: DirectoryRead>(
    sources: &[D],
    ranges: &[Range<usize>],
) -> Result<Option<DigestBounds>, D::Error> {
    let mut bounds = None;
    for (source, range) in sources
        .iter()
        .zip(ranges)
        .filter(|(_, range)| !range.is_empty())
    {
        let first = source.record(range.start)?.digest();
        let last = source.record(range.end - 1)?.digest();
        bounds = Some(
            bounds.map_or(DigestBounds { first, last }, |bounds: DigestBounds| {
                DigestBounds {
                    first: bounds.first.min(first),
                    last: bounds.last.max(last),
                }
            }),
        );
    }
    Ok(bounds)
}

fn partition<D: DirectoryRead>(
    directory: &D,
    range: Range<usize>,
    byte: usize,
    mask: u8,
) -> Result<usize, D::Error> {
    let (mut low, mut high) = (range.start, range.end);
    while low < high {
        let middle = low + (high - low) / 2;
        if directory.record(middle)?.digest()[byte] & mask == 0 {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    Ok(low)
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::allocation::Resident;

    #[test]
    fn exact_checkpoints_preserve_every_prefix_across_block_boundaries() {
        for count in [0_usize, 1, 31, 32, 33, 63, 64, 65] {
            let mut resident = Resident;
            let mut memory = Memory::new(&mut resident);
            let records = memory
                .collect((0..count).map(|index| {
                    let mut digest = [0; 32];
                    digest[..8].copy_from_slice(&(index as u64).to_be_bytes());
                    SourceRecord {
                        digest,
                        ordinal: u32::try_from(index).unwrap(),
                        stream: StatisticsStream::Asserted,
                    }
                }))
                .unwrap();
            let directory = SourceDirectory::freeze(records, &mut memory).unwrap();
            assert_eq!(directory.checkpoints.len(), count / CHECKPOINT_ROWS);
            let mut expected = Multiset256::default();
            for end in 0..=count {
                if end != 0 {
                    expected.insert(directory.records[end - 1].digest).unwrap();
                }
                let mut work = SourceDifferenceWork::default();
                assert_eq!(directory.prefix(end, &mut work), expected);
                assert_eq!(work.members, (end % CHECKPOINT_ROWS) as u64);
            }
        }
    }
}
