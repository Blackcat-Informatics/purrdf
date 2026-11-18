// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-local content order, independently of dictionary and row ordinals.
//! Canonical Merkle nodes over the sorted digests let two immutable heads skip
//! unchanged digest ranges. A sparse checkpoint keeps the node of every branch
//! holding at least [`CHECKPOINT_ROWS`] members; smaller nodes are recomputed.

use std::ops::Range;

use purrdf_hash::multiset::Multiset256;
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_lex::walk::WorkList;

use crate::{StatisticsStream, source::SourceDifferenceWork};

mod merkle;
pub(crate) use merkle::{DirectoryNode, branch as merkle_branch, node_of};

pub(crate) const CHECKPOINT_ROWS: usize = 32;
pub(crate) const RECORD_BYTES: usize = 37;
/// A persisted checkpoint: the little-endian `u64` boundary, then the node.
pub(crate) const NODE_CHECKPOINT_BYTES: usize = 40;

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
    /// The canonical node of `range`, which holds exactly the records whose
    /// digests share one bit prefix; `None` when it is empty. Without
    /// checkpoints the node is recomputed from the records.
    fn node(
        &self,
        range: Range<usize>,
        work: &mut SourceDifferenceWork,
    ) -> Result<Option<DirectoryNode>, Self::Error> {
        recompute(self, range, work)
    }
}

/// The node of `range`, hashed from its records.
pub(crate) fn recompute<D: DirectoryRead + ?Sized>(
    directory: &D,
    range: Range<usize>,
    work: &mut SourceDifferenceWork,
) -> Result<Option<DirectoryNode>, D::Error> {
    let mut builder = merkle::NodeBuilder::new(0);
    for index in range {
        builder.push(directory.record(index)?.digest(), &mut |_| {});
        work.members += 1;
    }
    Ok(builder.finish(&mut |_| {}))
}

/// The node of a digest-prefix `range` from sparse checkpoints. A range of at
/// least [`CHECKPOINT_ROWS`] members with two distinct digests is one recorded
/// branch, found by the ordinal of its first member with the split bit set.
pub(crate) fn checkpointed<D: DirectoryRead>(
    directory: &D,
    range: Range<usize>,
    work: &mut SourceDifferenceWork,
    lookup: impl FnOnce(u64) -> Result<DirectoryNode, D::Error>,
) -> Result<Option<DirectoryNode>, D::Error> {
    if range.len() < CHECKPOINT_ROWS {
        return recompute(directory, range, work);
    }
    let first = directory.record(range.start)?.digest();
    let last = directory.record(range.end - 1)?.digest();
    let Some(bit) = first_differing_bit(&first, &last) else {
        return Ok(Some(merkle::leaf(first, range.len() as u64)));
    };
    let boundary = partition(directory, range, usize::from(bit / 8), 0x80 >> (bit % 8))?;
    lookup(boundary as u64).map(Some)
}

/// The node of one recorded branch, keyed by its boundary ordinal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NodeCheckpoint {
    pub(crate) boundary: u64,
    pub(crate) node: DirectoryNode,
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
    // Every branch of at least CHECKPOINT_ROWS members, by ascending boundary.
    checkpoints: Box<[NodeCheckpoint]>,
    root: Option<DirectoryNode>,
    content: Multiset256,
}

impl SourceDirectory {
    /// The additive sum of every record, for content fingerprints and for
    /// agreement between two values this process built.
    pub(crate) const fn content_records(&self) -> Multiset256 {
        self.content
    }

    /// The canonical node of every record; `None` when there is none.
    pub(crate) const fn root(&self) -> Option<DirectoryNode> {
        self.root
    }

    pub(crate) fn retained_bytes(&self) -> Option<usize> {
        size_of_val(self.records.as_ref()).checked_add(size_of_val(self.checkpoints.as_ref()))
    }

    pub(crate) fn freeze<S: Admission + ?Sized>(
        mut records: Vec<SourceRecord>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let mut checkpoints = Vec::new();
        let prepared = (|| {
            memory.admission_mut().checkpoint(0)?;
            // Resident and controlled sealing use the same digest ordering and
            // equal-digest permutation. Source-local addresses do not define
            // content order, but both paths must emit the same directory bytes.
            purrdf_lex::walk::try_sort_unstable_by(&mut records, |left, right| {
                memory.admission_mut().checkpoint(1)?;
                Ok::<_, StorageError>(left.digest.cmp(&right.digest))
            })?;
            memory.admission_mut().checkpoint(0)?;
            let mut content = Multiset256::default();
            let mut builder = merkle::NodeBuilder::new(0);
            let mut refused = None;
            let mut record_branch = |branch: merkle::Branch, memory: &mut Memory<'_, S>| {
                if branch.population >= CHECKPOINT_ROWS as u64 && refused.is_none() {
                    let checkpoint = NodeCheckpoint {
                        boundary: branch.boundary,
                        node: branch.node,
                    };
                    if let Err(cause) = memory.push_copy_with_progress(&mut checkpoints, checkpoint)
                    {
                        refused = Some(cause);
                    }
                }
            };
            for record in &records {
                memory.admission_mut().checkpoint(1)?;
                content
                    .insert(record.digest)
                    .map_err(|_| StorageError::SizeOverflow)?;
                builder.push(record.digest, &mut |branch| record_branch(branch, memory));
            }
            let root = builder.finish(&mut |branch| record_branch(branch, memory));
            if let Some(cause) = refused {
                return Err(cause);
            }
            // Branches complete in post-order; readers search by boundary.
            purrdf_lex::walk::try_sort_unstable_by(&mut checkpoints, |left, right| {
                memory.admission_mut().checkpoint(1)?;
                Ok::<_, StorageError>(left.boundary.cmp(&right.boundary))
            })?;
            memory.admission_mut().checkpoint(0)?;
            Ok::<_, StorageError>((root, content))
        })();
        let (root, content) = match prepared {
            Ok(values) => values,
            Err(cause) => {
                // Both local owners die before their individual refunds. A refused
                // refund cannot replace the first failure or prevent the other refund.
                let _ = memory.release_vec(checkpoints);
                let _ = memory.release_vec(records);
                return Err(cause);
            }
        };
        Self::from_arrays(records, checkpoints, (root, content), memory)
    }

    /// Publish both originally admitted buffers, retaining neither on refusal.
    fn from_arrays<S: Admission + ?Sized>(
        records: Vec<SourceRecord>,
        checkpoints: Vec<NodeCheckpoint>,
        (root, content): (Option<DirectoryNode>, Multiset256),
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let records = match memory.boxed_slice(records) {
            Ok(records) => records,
            Err(cause) => {
                let _ = memory.release_vec(checkpoints);
                return Err(cause);
            }
        };
        let checkpoints = match memory.boxed_slice(checkpoints) {
            Ok(checkpoints) => checkpoints,
            Err(cause) => {
                let _ = memory.release_vec(records.into_vec());
                return Err(cause);
            }
        };
        Ok(Self {
            records,
            checkpoints,
            root,
            content,
        })
    }

    pub(crate) fn clone_admitted<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let records = memory.collect(self.records.iter().copied())?;
        let checkpoints = match memory.collect(self.checkpoints.iter().copied()) {
            Ok(checkpoints) => checkpoints,
            Err(cause) => {
                let _ = memory.release_vec(records);
                return Err(cause);
            }
        };
        Self::from_arrays(records, checkpoints, (self.root, self.content), memory)
    }

    /// Destroy both buffers and attempt their exact original refunds.
    pub(super) fn release_admitted<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let records = memory.release_vec(self.records.into_vec());
        let checkpoints = memory.release_vec(self.checkpoints.into_vec());
        records.and(checkpoints)
    }

    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn node_checkpoints(&self) -> &[NodeCheckpoint] {
        &self.checkpoints
    }

    /// The additive prefix sums a version 7 image stores after every complete
    /// block of [`CHECKPOINT_ROWS`] records, recomputed from the records.
    pub(crate) fn additive_checkpoints(&self) -> impl ExactSizeIterator<Item = [u8; 32]> + '_ {
        let mut state = Multiset256::default();
        self.records
            .as_chunks::<CHECKPOINT_ROWS>()
            .0
            .iter()
            .map(move |block| {
                for record in block {
                    state
                        .insert(record.digest)
                        .expect("a directory population fits u128");
                }
                *state.sum()
            })
    }

    /// Persist the sparse checkpoints from `start`, as [`NODE_CHECKPOINT_BYTES`]
    /// records: the boundary as a little-endian `u64`, then the node.
    pub(crate) fn encode_node_checkpoints(&self, start: usize, output: &mut [u8]) {
        assert!(output.len().is_multiple_of(NODE_CHECKPOINT_BYTES));
        for (index, bytes) in output
            .as_chunks_mut::<NODE_CHECKPOINT_BYTES>()
            .0
            .iter_mut()
            .enumerate()
        {
            let checkpoint = self.checkpoints[start + index];
            crate::bytes::put_u64_le(bytes, 0, checkpoint.boundary)
                .expect("fixed checkpoint width");
            bytes[8..].copy_from_slice(&checkpoint.node);
        }
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
    fn node(
        &self,
        range: Range<usize>,
        work: &mut SourceDifferenceWork,
    ) -> Result<Option<DirectoryNode>, Self::Error> {
        if range.start == 0 && range.end == self.records.len() {
            return Ok(self.root);
        }
        checkpointed(self, range, work, |boundary| {
            let index = self
                .checkpoints
                .binary_search_by_key(&boundary, |checkpoint| checkpoint.boundary)
                // INVARIANT: freeze records every branch of at least
                // CHECKPOINT_ROWS members, and a digest-prefix range of that
                // size with two distinct digests is exactly one such branch.
                .expect("every large branch has a checkpoint");
            Ok(self.checkpoints[index].node)
        })
    }
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

struct DirectoryScratch {
    pending: WorkList<usize, 0>,
    ranges: Vec<Range<usize>>,
    working: Vec<Range<usize>>,
}

struct DirectoryWorkspace<'m, 'a, S: Admission + ?Sized> {
    scratch: Option<DirectoryScratch>,
    memory: &'m mut Memory<'a, S>,
}

impl<S: Admission + ?Sized> DirectoryWorkspace<'_, '_, S> {
    fn release(&mut self) -> Result<(), StorageError> {
        let Some(DirectoryScratch {
            pending,
            ranges,
            working,
        }) = self.scratch.take()
        else {
            return Ok(());
        };
        // Attempt every refund even if the original account refuses one shrink.
        let pending = pending.release_admitted(self.memory);
        let ranges = self.memory.release_vec(ranges);
        let working = self.memory.release_vec(working);
        pending.and(ranges).and(working)
    }
}

impl<S: Admission + ?Sized> Drop for DirectoryWorkspace<'_, '_, S> {
    fn drop(&mut self) {
        // Only these scratch buffers belong to us; retained visitor output keeps
        // its own admission. Preserve an unwinding visitor's original panic.
        let _ = self.release();
    }
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
    let mut workspace = DirectoryWorkspace {
        scratch: Some(DirectoryScratch {
            pending: WorkList::new(),
            ranges: Vec::new(),
            working: Vec::new(),
        }),
        memory,
    };
    let DirectoryWorkspace { scratch, memory } = &mut workspace;
    // Scratch is installed above and only release takes it after this callback
    // finishes or while the workspace unwinds.
    let DirectoryScratch {
        pending,
        ranges,
        working,
    } = scratch.as_mut().expect("owned directory scratch");
    let memory = &mut **memory;
    let result = (|| {
        let width = previous
            .len()
            .checked_add(next.len())
            .ok_or(StorageError::SizeOverflow)?;
        if width == 0 {
            return Ok(SourceDifferenceWork::default());
        }
        for source in previous {
            memory.push(ranges, 0..source.len()?)?;
        }
        for source in next {
            memory.push(ranges, 0..source.len()?)?;
        }
        for range in ranges.iter() {
            memory.push(working, range.clone())?;
        }
        pending.try_push_admitted(0_usize, memory)?;
        let mut work = SourceDifferenceWork::default();
        while let Some(start) = pending.pop() {
            debug_assert_eq!(start + width, ranges.len());
            working.clone_from_slice(&ranges[start..]);
            ranges.truncate(start);
            let (old, new) = working.split_at(previous.len());
            work.ranges += 1;
            if same_carriers::<_, _, E>((previous, old), (next, new), &mut work)? {
                continue;
            }
            let old_population = old.iter().map(Range::len).sum::<usize>();
            let new_population = new.iter().map(Range::len).sum::<usize>();
            let bounds = if old_population == 0 || new_population == 0 {
                None
            } else {
                let a = group_bounds(previous, old)?;
                let b = group_bounds(next, new)?;
                match (a, b) {
                    (Some(a), Some(b)) => {
                        let bit = first_differing_bit(&a.first.min(b.first), &a.last.max(b.last));
                        if bit.is_none() && old_population == new_population {
                            // Every member on both sides is one digest, so
                            // equal populations are equal multisets.
                            continue;
                        }
                        bit
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
                memory.push(ranges, middle..range.end)?;
                range.end = middle;
            }
            for (source, range) in next.iter().zip(&mut working[previous.len()..]) {
                let middle = partition(source, range.clone(), byte, mask)?;
                memory.push(ranges, middle..range.end)?;
                range.end = middle;
            }
            pending.try_push_admitted(right, memory)?;
            let left = ranges.len();
            for range in working.iter() {
                memory.push(ranges, range.clone())?;
            }
            pending.try_push_admitted(left, memory)?;
        }
        Ok(work)
    })();
    let cleanup = workspace.release();
    match result {
        Err(error) => Err(error),
        Ok(work) => cleanup.map(|()| work).map_err(E::from),
    }
}

/// Whether both sides hold the same members carrier by carrier: their
/// nonempty ranges pair up in order with equal canonical nodes. That proves
/// the unions equal. Members spread differently over the carriers are not
/// recognised here; the caller splits the range and compares the parts.
fn same_carriers<A: DirectoryRead, B: DirectoryRead, E: From<A::Error> + From<B::Error>>(
    (previous, old): (&[A], &[Range<usize>]),
    (next, new): (&[B], &[Range<usize>]),
    work: &mut SourceDifferenceWork,
) -> Result<bool, E> {
    let mut old = previous
        .iter()
        .zip(old)
        .filter(|(_, range)| !range.is_empty());
    let mut new = next.iter().zip(new).filter(|(_, range)| !range.is_empty());
    loop {
        match (old.next(), new.next()) {
            (None, None) => return Ok(true),
            (Some((a, before)), Some((b, after))) => {
                if before.len() != after.len() {
                    return Ok(false);
                }
                let before = a.node(before.clone(), work)?;
                if before != b.node(after.clone(), work)? {
                    return Ok(false);
                }
            }
            _ => return Ok(false),
        }
    }
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
mod tests;
