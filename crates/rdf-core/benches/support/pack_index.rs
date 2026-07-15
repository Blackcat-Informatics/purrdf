// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// This module is included by both the benchmark and its correctness test; each
// target intentionally exercises a different subset of the experiment helpers.
#![allow(dead_code)]

use std::cmp::Ordering;

use purrdf_core::ir::pack::bits::{
    BitVec, DeltaListRef, IntVector, RankSelect, bits_for, write_delta_list,
};

#[derive(Debug, Clone)]
pub(crate) struct Adjacency {
    sp: Vec<u64>,
    bp: RankSelect,
    so: Vec<u64>,
    bo: RankSelect,
    predicate_values: Vec<u64>,
    object_values: Vec<u64>,
}

impl Adjacency {
    pub(crate) fn from_triples(triples: &[(u64, u64, u64)]) -> Self {
        let mut triples = triples.to_vec();
        triples.sort_unstable();
        triples.dedup();

        let mut subject_values: Vec<_> = triples.iter().map(|&(s, _, _)| s).collect();
        subject_values.sort_unstable();
        subject_values.dedup();
        let mut predicate_values: Vec<_> = triples.iter().map(|&(_, p, _)| p).collect();
        predicate_values.sort_unstable();
        predicate_values.dedup();
        let mut object_values: Vec<_> = triples.iter().map(|&(_, _, o)| o).collect();
        object_values.sort_unstable();
        object_values.dedup();

        let mut local: Vec<_> = triples
            .into_iter()
            .map(|(s, p, o)| {
                (
                    subject_values.binary_search(&s).expect("subject present") as u64,
                    predicate_values
                        .binary_search(&p)
                        .expect("predicate present") as u64,
                    object_values.binary_search(&o).expect("object present") as u64,
                )
            })
            .collect();
        local.sort_unstable();

        let mut sp = Vec::new();
        let mut bp = BitVec::new();
        let mut so = Vec::with_capacity(local.len());
        let mut bo = BitVec::new();
        let mut i = 0usize;
        while i < local.len() {
            let subject = local[i].0;
            let mut j = i;
            while j < local.len() && local[j].0 == subject {
                let predicate = local[j].1;
                sp.push(predicate);
                let mut k = j;
                while k < local.len() && local[k].0 == subject && local[k].1 == predicate {
                    so.push(local[k].2);
                    let last = k + 1 == local.len()
                        || local[k + 1].0 != subject
                        || local[k + 1].1 != predicate;
                    bo.push(last);
                    k += 1;
                }
                let last = k == local.len() || local[k].0 != subject;
                bp.push(last);
                j = k;
            }
            i = j;
        }

        Self {
            sp,
            bp: bp.freeze(),
            so,
            bo: bo.freeze(),
            predicate_values,
            object_values,
        }
    }

    pub(crate) fn predicate_local(&self, value: u64) -> Option<u64> {
        self.predicate_values
            .binary_search(&value)
            .ok()
            .map(|i| i as u64)
    }

    pub(crate) fn object_local(&self, value: u64) -> Option<u64> {
        self.object_values
            .binary_search(&value)
            .ok()
            .map(|i| i as u64)
    }

    pub(crate) fn sp_len(&self) -> usize {
        self.sp.len()
    }

    pub(crate) fn so_len(&self) -> usize {
        self.so.len()
    }

    fn pair_slice(&self, sp_position: u64) -> (usize, usize) {
        let position = sp_position as usize;
        let start = if position == 0 {
            0
        } else {
            self.bo
                .select1(position - 1)
                .expect("one boundary per Sp entry")
                + 1
        };
        let end = self
            .bo
            .select1(position)
            .expect("one boundary per Sp entry")
            + 1;
        (start, end)
    }

    fn owner_of_object_position(&self, so_position: usize) -> u64 {
        self.bo.rank1(so_position) as u64
    }

    pub(crate) fn shared_serialized_len(&self) -> usize {
        let mut sp = IntVector::with_width(bits_for(
            self.predicate_values.len().saturating_sub(1) as u64
        ));
        for &value in &self.sp {
            sp.push(value);
        }
        let mut so =
            IntVector::with_width(bits_for(self.object_values.len().saturating_sub(1) as u64));
        for &value in &self.so {
            so.push(value);
        }
        sp.serialized_len()
            + self.bp.serialized_len()
            + so.serialized_len()
            + self.bo.serialized_len()
    }
}

#[derive(Debug, Clone)]
struct PostingIndex {
    offsets: IntVector,
    counts: IntVector,
    data: Vec<u8>,
}

impl PostingIndex {
    fn build(alphabet_len: usize, entries: impl IntoIterator<Item = (u64, u64)>) -> Self {
        let mut positions = vec![Vec::new(); alphabet_len];
        for (value, position) in entries {
            positions[value as usize].push(position);
        }

        let mut data = Vec::new();
        let mut raw_offsets = Vec::with_capacity(alphabet_len);
        let mut raw_counts = Vec::with_capacity(alphabet_len);
        for list in &positions {
            raw_offsets.push(data.len() as u64);
            raw_counts.push(list.len() as u64);
            write_delta_list(&mut data, list);
        }
        let mut offsets =
            IntVector::with_width(bits_for(raw_offsets.iter().copied().max().unwrap_or(0)));
        let mut counts =
            IntVector::with_width(bits_for(raw_counts.iter().copied().max().unwrap_or(0)));
        for offset in raw_offsets {
            offsets.push(offset);
        }
        for count in raw_counts {
            counts.push(count);
        }
        Self {
            offsets,
            counts,
            data,
        }
    }

    fn positions(&self, value: u64) -> Vec<u64> {
        let count = self.counts.get(value as usize) as usize;
        let offset = self.offsets.get(value as usize) as usize;
        DeltaListRef::new(&self.data[offset..], count)
            .map(|item| item.expect("bench-built delta list is valid"))
            .collect()
    }

    fn serialized_len(&self) -> usize {
        self.offsets.serialized_len()
            + self.counts.serialized_len()
            + size_of::<u64>()
            + self.data.len()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FoqIndexes {
    predicates: PostingIndex,
    predicate_totals: IntVector,
    objects: PostingIndex,
}

impl FoqIndexes {
    pub(crate) fn build(adjacency: &Adjacency) -> Self {
        let predicates = PostingIndex::build(
            adjacency.predicate_values.len(),
            adjacency
                .sp
                .iter()
                .copied()
                .enumerate()
                .map(|(position, value)| (value, position as u64)),
        );
        let objects = PostingIndex::build(
            adjacency.object_values.len(),
            adjacency
                .so
                .iter()
                .copied()
                .enumerate()
                .map(|(position, value)| (value, adjacency.owner_of_object_position(position))),
        );

        let mut totals = vec![0u64; adjacency.predicate_values.len()];
        for (position, &predicate) in adjacency.sp.iter().enumerate() {
            let (start, end) = adjacency.pair_slice(position as u64);
            totals[predicate as usize] += (end - start) as u64;
        }
        let mut predicate_totals =
            IntVector::with_width(bits_for(totals.iter().copied().max().unwrap_or(0)));
        for total in totals {
            predicate_totals.push(total);
        }

        Self {
            predicates,
            predicate_totals,
            objects,
        }
    }

    pub(crate) fn predicate_count(&self, adjacency: &Adjacency, predicate: u64) -> usize {
        self.predicates
            .positions(predicate)
            .into_iter()
            .map(|position| {
                let (start, end) = adjacency.pair_slice(position);
                end - start
            })
            .sum()
    }

    pub(crate) fn object_count(&self, object: u64) -> usize {
        self.objects.positions(object).len()
    }

    pub(crate) fn predicate_object_count(&self, predicate: u64, object: u64) -> usize {
        intersect_count(
            &self.predicates.positions(predicate),
            &self.objects.positions(object),
        )
    }

    pub(crate) fn index_serialized_len(&self) -> usize {
        self.predicates.serialized_len()
            + self.predicate_totals.serialized_len()
            + self.objects.serialized_len()
    }

    pub(crate) fn serialized_len(&self, adjacency: &Adjacency) -> usize {
        adjacency.shared_serialized_len() + self.index_serialized_len()
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.index_serialized_len());
        bytes.extend_from_slice(&self.predicates.offsets.to_bytes());
        bytes.extend_from_slice(&self.predicates.counts.to_bytes());
        bytes.extend_from_slice(&self.predicate_totals.to_bytes());
        bytes.extend_from_slice(&(self.predicates.data.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&self.predicates.data);
        bytes.extend_from_slice(&self.objects.offsets.to_bytes());
        bytes.extend_from_slice(&self.objects.counts.to_bytes());
        bytes.extend_from_slice(&(self.objects.data.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&self.objects.data);
        bytes
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WaveletMatrix {
    len: usize,
    width: u32,
    max_value: u64,
    levels: Vec<RankSelect>,
    zeros: Vec<usize>,
}

impl WaveletMatrix {
    pub(crate) fn build(values: &[u64]) -> Self {
        let max_value = values.iter().copied().max().unwrap_or(0);
        let width = if values.is_empty() {
            0
        } else {
            bits_for(max_value)
        };
        let mut current = values.to_vec();
        let mut levels = Vec::with_capacity(width as usize);
        let mut zeros = Vec::with_capacity(width as usize);

        for shift in (0..width).rev() {
            let mut bits = BitVec::new();
            let mut zero_values = Vec::with_capacity(current.len());
            let mut one_values = Vec::with_capacity(current.len());
            for &value in &current {
                let one = ((value >> shift) & 1) != 0;
                bits.push(one);
                if one {
                    one_values.push(value);
                } else {
                    zero_values.push(value);
                }
            }
            zeros.push(zero_values.len());
            zero_values.extend(one_values);
            current = zero_values;
            levels.push(bits.freeze());
        }

        Self {
            len: values.len(),
            width,
            max_value,
            levels,
            zeros,
        }
    }

    pub(crate) fn access(&self, position: usize) -> Option<u64> {
        if position >= self.len {
            return None;
        }
        let mut position = position;
        let mut value = 0u64;
        for (level, bitmap) in self.levels.iter().enumerate() {
            let one = bitmap.rank1(position + 1) != bitmap.rank1(position);
            let shift = self.width as usize - level - 1;
            if one {
                value |= 1u64 << shift;
                position = self.zeros[level] + bitmap.rank1(position);
            } else {
                position = bitmap.rank0(position);
            }
        }
        Some(value)
    }

    pub(crate) fn rank(&self, value: u64, end: usize) -> usize {
        if value > self.max_value || end > self.len {
            return 0;
        }
        let mut start = 0usize;
        let mut end = end;
        for (level, bitmap) in self.levels.iter().enumerate() {
            let shift = self.width as usize - level - 1;
            if ((value >> shift) & 1) == 0 {
                start = bitmap.rank0(start);
                end = bitmap.rank0(end);
            } else {
                start = self.zeros[level] + bitmap.rank1(start);
                end = self.zeros[level] + bitmap.rank1(end);
            }
        }
        end - start
    }

    pub(crate) fn select(&self, value: u64, occurrence: usize) -> Option<usize> {
        if value > self.max_value {
            return None;
        }
        let mut start = 0usize;
        let mut end = self.len;
        for (level, bitmap) in self.levels.iter().enumerate() {
            let shift = self.width as usize - level - 1;
            if ((value >> shift) & 1) == 0 {
                start = bitmap.rank0(start);
                end = bitmap.rank0(end);
            } else {
                start = self.zeros[level] + bitmap.rank1(start);
                end = self.zeros[level] + bitmap.rank1(end);
            }
        }
        if occurrence >= end - start {
            return None;
        }
        let mut position = start + occurrence;
        for level in (0..self.levels.len()).rev() {
            let shift = self.width as usize - level - 1;
            position = if ((value >> shift) & 1) == 0 {
                self.levels[level].select0(position)?
            } else {
                self.levels[level].select1(position - self.zeros[level])?
            };
        }
        Some(position)
    }

    fn positions(&self, value: u64) -> Vec<u64> {
        (0..self.rank(value, self.len))
            .map(|occurrence| {
                self.select(value, occurrence)
                    .expect("occurrence is below rank") as u64
            })
            .collect()
    }

    pub(crate) fn serialized_len(&self) -> usize {
        size_of::<u8>()
            + size_of::<u64>()
            + size_of::<u32>()
            + self
                .levels
                .iter()
                .map(|level| 2 * size_of::<u64>() + level.serialized_len())
                .sum::<usize>()
    }

    fn append_bytes(&self, bytes: &mut Vec<u8>) {
        bytes.push(1);
        bytes.extend_from_slice(&(self.len as u64).to_le_bytes());
        bytes.extend_from_slice(&self.width.to_le_bytes());
        for (&zero_count, level) in self.zeros.iter().zip(&self.levels) {
            let level_bytes = level.to_bytes();
            bytes.extend_from_slice(&(zero_count as u64).to_le_bytes());
            bytes.extend_from_slice(&(level_bytes.len() as u64).to_le_bytes());
            bytes.extend_from_slice(&level_bytes);
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WaveletIndexes {
    predicates: WaveletMatrix,
    objects: WaveletMatrix,
}

impl WaveletIndexes {
    pub(crate) fn build(adjacency: &Adjacency) -> Self {
        Self {
            predicates: WaveletMatrix::build(&adjacency.sp),
            objects: WaveletMatrix::build(&adjacency.so),
        }
    }

    pub(crate) fn predicate_count(&self, adjacency: &Adjacency, predicate: u64) -> usize {
        self.predicates
            .positions(predicate)
            .into_iter()
            .map(|position| {
                let (start, end) = adjacency.pair_slice(position);
                end - start
            })
            .sum()
    }

    pub(crate) fn object_count(&self, object: u64) -> usize {
        self.objects.positions(object).len()
    }

    pub(crate) fn predicate_object_count(
        &self,
        adjacency: &Adjacency,
        predicate: u64,
        object: u64,
    ) -> usize {
        let predicate_positions = self.predicates.positions(predicate);
        let object_positions: Vec<_> = self
            .objects
            .positions(object)
            .into_iter()
            .map(|position| adjacency.owner_of_object_position(position as usize))
            .collect();
        intersect_count(&predicate_positions, &object_positions)
    }

    pub(crate) fn index_serialized_len(&self) -> usize {
        self.predicates.serialized_len() + self.objects.serialized_len()
    }

    pub(crate) fn serialized_len(&self, adjacency: &Adjacency) -> usize {
        adjacency.shared_serialized_len() + self.index_serialized_len()
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.index_serialized_len());
        self.predicates.append_bytes(&mut bytes);
        self.objects.append_bytes(&mut bytes);
        bytes
    }
}

fn intersect_count(left: &[u64], right: &[u64]) -> usize {
    let (mut i, mut j, mut count) = (0usize, 0usize, 0usize);
    while i < left.len() && j < right.len() {
        match left[i].cmp(&right[j]) {
            Ordering::Less => i += 1,
            Ordering::Greater => j += 1,
            Ordering::Equal => {
                count += 1;
                i += 1;
                j += 1;
            }
        }
    }
    count
}

pub(crate) fn reference_triples(rows: usize) -> Vec<(u64, u64, u64)> {
    let mut triples = Vec::with_capacity(rows * 4);
    for subject in 0..rows as u64 {
        triples.push((subject, 0, 0));
        triples.push((subject, 0, 1 + subject % 257));
        triples.push((subject, 1 + subject % 31, 258 + subject % 4096));
        triples.push((subject, 32 + subject % 1024, 4354 + subject));
    }
    triples
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReferenceSpace {
    pub(crate) triples: u64,
    pub(crate) shared: u64,
    pub(crate) foq_index: u64,
    pub(crate) wavelet_index: u64,
}

impl ReferenceSpace {
    pub(crate) fn foq_total(self) -> u64 {
        self.shared + self.foq_index
    }

    pub(crate) fn wavelet_total(self) -> u64 {
        self.shared + self.wavelet_index
    }
}

pub(crate) fn reference_space(rows: u64) -> ReferenceSpace {
    assert!(rows > 0, "the reference workload requires at least one row");

    let predicate_alphabet = 1 + rows.min(31) + rows.min(1_024);
    let object_alphabet = 1 + rows.min(257) + rows.min(4_096) + rows;
    let sp_len = rows.checked_mul(3).expect("Sp length fits u64");
    let so_len = rows.checked_mul(4).expect("So length fits u64");

    let shared = int_vector_space(sp_len, predicate_alphabet - 1)
        + rank_select_space(sp_len)
        + int_vector_space(so_len, object_alphabet - 1)
        + rank_select_space(so_len);

    let predicate_data = cycle_delta_space(rows, 1, 0)
        + cycle_delta_space(rows, 31, 1)
        + cycle_delta_space(rows, 1_024, 2);
    let last_predicate_list = cycle_list_space(rows, 1_024, rows.min(1_024) - 1, 2);
    let predicate_offsets_max = predicate_data - last_predicate_list;
    let predicate_index = int_vector_space(predicate_alphabet, predicate_offsets_max)
        + int_vector_space(predicate_alphabet, rows)
        + int_vector_space(predicate_alphabet, rows * 2)
        + size_of::<u64>() as u64
        + predicate_data;

    let object_data = cycle_delta_space(rows, 1, 0)
        + cycle_delta_space(rows, 257, 0)
        + cycle_delta_space(rows, 4_096, 1)
        + unique_delta_space(rows, 2);
    let last_object_list = varint_space(3 * (rows - 1) + 2);
    let object_offsets_max = object_data - last_object_list;
    let object_index = int_vector_space(object_alphabet, object_offsets_max)
        + int_vector_space(object_alphabet, rows)
        + size_of::<u64>() as u64
        + object_data;

    let wavelet_index =
        wavelet_space(sp_len, predicate_alphabet - 1) + wavelet_space(so_len, object_alphabet - 1);

    ReferenceSpace {
        triples: so_len,
        shared,
        foq_index: predicate_index + object_index,
        wavelet_index,
    }
}

fn int_vector_space(len: u64, max_value: u64) -> u64 {
    let width = u64::from(bits_for(max_value));
    12 + len
        .checked_mul(width)
        .expect("IntVector bit length fits u64")
        .div_ceil(64)
        * 8
}

fn rank_select_space(len: u64) -> u64 {
    let words = len.div_ceil(64);
    32 + words.div_ceil(8) * 8 + words * 10
}

fn wavelet_space(len: u64, max_value: u64) -> u64 {
    let width = u64::from(bits_for(max_value));
    13 + width * (2 * size_of::<u64>() as u64 + rank_select_space(len))
}

fn cycle_delta_space(rows: u64, cycle: u64, position_offset: u64) -> u64 {
    let lists = rows.min(cycle);
    unique_delta_space(lists, position_offset) + (rows - lists) * varint_space(3 * cycle)
}

fn cycle_list_space(rows: u64, cycle: u64, residue: u64, position_offset: u64) -> u64 {
    let count = 1 + (rows - 1 - residue) / cycle;
    varint_space(3 * residue + position_offset) + (count - 1) * varint_space(3 * cycle)
}

fn unique_delta_space(rows: u64, position_offset: u64) -> u64 {
    (1..=10)
        .map(|bytes| {
            let lower = if bytes == 1 {
                0
            } else {
                1u64 << (7 * (bytes - 1))
            };
            let upper = if bytes == 10 {
                u64::MAX
            } else {
                (1u64 << (7 * bytes)) - 1
            };
            arithmetic_positions_in_range(rows, position_offset, lower, upper) * bytes
        })
        .sum()
}

fn arithmetic_positions_in_range(rows: u64, offset: u64, lower: u64, upper: u64) -> u64 {
    let first = lower.saturating_sub(offset).div_ceil(3);
    let last = upper.saturating_sub(offset) / 3;
    if first >= rows || first > last {
        0
    } else {
        (rows - 1).min(last) - first + 1
    }
}

fn varint_space(value: u64) -> u64 {
    u64::from((64 - value.leading_zeros()).max(1)).div_ceil(7)
}
