// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::{num::NonZeroU64, ops::Range};
use purrdf_lex::cbor::DecodeErrorKind;

use super::Digest;
use crate::v2::{
    Item, PayloadBudget, ReadFailure, SegmentCheckError,
    fields::{copy_bytes, push},
};

/// The reason a span cannot contribute to the repaired copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParityRepairFailure {
    /// A stripe exceeds the erasure limit.
    TooManyErasures,
    /// A shard, overlapping run or predecessor failed verification.
    VerifyFailed,
}

pub(super) struct OriginalItem {
    pub(super) bytes: Range<usize>,
    pub(super) ordinary: Option<Digest>,
}

pub(super) struct Candidate {
    pub(super) offset: usize,
    pub(super) writes: Vec<Write>,
    pub(super) count: usize,
    pub(super) failure: Option<ParityRepairFailure>,
    touched: Vec<usize>,
}

impl Candidate {
    pub(super) fn new(offset: usize) -> Self {
        Self {
            offset,
            writes: Vec::new(),
            count: 0,
            failure: None,
            touched: Vec::new(),
        }
    }
}

pub(super) struct Write {
    pub(super) offset: usize,
    pub(super) bytes: Vec<u8>,
    pub(super) data: bool,
}

pub(super) struct SegmentInput<'a> {
    pub(super) original: &'a [u8],
    pub(super) items: &'a [OriginalItem],
    pub(super) opener: Digest,
    pub(super) registry: NonZeroU64,
}

#[derive(Default)]
struct Run {
    dependents: Vec<usize>,
}

enum RunResult {
    Failed,
    PreservedHead,
    Advanced(Digest),
}

impl SegmentInput<'_> {
    pub(super) fn verify(
        &self,
        candidates: &mut [Candidate],
        budget: &mut PayloadBudget,
    ) -> Result<(), ReadFailure> {
        let mut touched = Vec::new();
        for (span, candidate) in candidates.iter().enumerate() {
            for write in &candidate.writes {
                if write.data {
                    push(
                        &mut touched,
                        (write.offset, write.offset + write.bytes.len(), span),
                    )?;
                }
            }
        }
        if touched.is_empty() {
            return Ok(());
        }
        touched.sort_unstable();
        // Every segment starts with its retained opener.
        let start = self.items[0].bytes.start;
        let end = self.items[self.items.len() - 1].bytes.end;
        let mut working = copy_bytes(&self.original[start..end])?;
        for candidate in candidates.iter() {
            for write in &candidate.writes {
                let offset = write.offset - start;
                working[offset..offset + write.bytes.len()].copy_from_slice(&write.bytes);
            }
        }
        let mut runs: Vec<Run> = Vec::new();
        let mut next = 0;
        let mut input_item = 0;
        let mut head = self.opener;
        let mut head_run: Option<usize> = None;
        let mut failed_since_head = false;
        while let Some(&(patch, _, _)) = touched.get(next) {
            while self.items[input_item].bytes.end <= patch {
                if let Some(ordinary) = self.items[input_item].ordinary {
                    head = ordinary;
                    head_run = None;
                    failed_since_head = false;
                }
                input_item += 1;
            }
            let index = runs.len();
            let begin = self.items[input_item].bytes.start;
            let (checked_end, checked) = self.walk(&working, budget, begin, input_item, head)?;
            while self.items[input_item].bytes.end < checked_end {
                input_item += 1;
            }
            let limit = self.items[input_item].bytes.end;
            input_item += 1;
            let mut run = Run::default();
            for &(patch_start, patch_end, span) in &touched[next..] {
                if patch_start >= limit {
                    break;
                }
                let candidate = &mut candidates[span];
                if candidate.touched.last() != Some(&index) {
                    push(&mut candidate.touched, index)?;
                    push(&mut run.dependents, span)?;
                    if let Some(predecessor) = head_run {
                        push(&mut runs[predecessor].dependents, span)?;
                    }
                    if failed_since_head || matches!(checked, RunResult::Failed) {
                        candidate.failure = Some(ParityRepairFailure::VerifyFailed);
                    }
                }
                if patch_end > limit {
                    break;
                }
                next += 1;
            }
            match checked {
                RunResult::Advanced(ordinary) => {
                    head = ordinary;
                    head_run = Some(index);
                    failed_since_head = false;
                }
                RunResult::Failed => failed_since_head = true,
                RunResult::PreservedHead => {}
            }
            push(&mut runs, run)?;
        }

        let mut rejected = Vec::new();
        for (index, candidate) in candidates.iter().enumerate() {
            if candidate.failure.is_some() {
                push(&mut rejected, index)?;
            }
        }
        while let Some(span) = rejected.pop() {
            for run in core::mem::take(&mut candidates[span].touched) {
                for dependent in core::mem::take(&mut runs[run].dependents) {
                    let failure = &mut candidates[dependent].failure;
                    if failure.is_none() {
                        *failure = Some(ParityRepairFailure::VerifyFailed);
                        push(&mut rejected, dependent)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn walk(
        &self,
        working: &[u8],
        budget: &mut PayloadBudget,
        begin: usize,
        mut input_item: usize,
        mut head: Digest,
    ) -> Result<(usize, RunResult), ReadFailure> {
        let start = self.items[0].bytes.start;
        let mut at = begin;
        let mut ordinary = None;
        if input_item == 0 {
            let opener = &self.items[0].bytes;
            if working[..opener.len()] != self.original[opener.clone()] {
                return Ok((opener.end, RunResult::Failed));
            }
            return Ok((opener.end, RunResult::PreservedHead));
        }
        loop {
            let (item, examined) = Item::first_examined(&working[at - start..]);
            let item = match item {
                Ok(Some(item)) => item,
                Ok(None) => return Ok((at, RunResult::Failed)),
                Err(error) => match error.kind() {
                    DecodeErrorKind::Malformed(_) | DecodeErrorKind::Truncated => {
                        return Ok((at + examined, RunResult::Failed));
                    }
                    _ => return Err(ReadFailure::Framing(error)),
                },
            };
            at += item.bytes().len();
            let Some(frame) = item.frame().map_err(ReadFailure::Framing)? else {
                return Ok((at, RunResult::Failed));
            };
            let Some(link) = frame
                .check_repair(&head, self.registry, budget)
                .map_err(|error| ReadFailure::Segment(SegmentCheckError::Payload(error)))?
            else {
                return Ok((at, RunResult::Failed));
            };
            if let Some(identifier) = link.effective_identifier {
                head = identifier;
                ordinary = Some(identifier);
            }
            while self.items[input_item].bytes.end < at {
                input_item += 1;
            }
            if self.items[input_item].bytes.end == at {
                return Ok((
                    at,
                    ordinary.map_or(RunResult::PreservedHead, RunResult::Advanced),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Candidate, OriginalItem, ParityRepairFailure, SegmentInput, Write};
    use crate::v2::{Header, Item, PayloadBudget, ReadCapacity};

    #[test]
    fn rejection_propagates_to_a_run_using_the_rejected_repair_as_predecessor() {
        let intact = include_bytes!("../../../tests/fixtures/v2/P5/parity-base.bin");
        let first = Item::first(intact).unwrap().unwrap();
        let header = Header::decode(first).unwrap();
        let opener = header.identifier().unwrap();
        let registry = header.registry_version();
        let mut damaged = intact.to_vec();
        let mut at = first.bytes().len();
        let mut head = opener;
        let mut items = vec![OriginalItem {
            bytes: 0..at,
            ordinary: None,
        }];
        while let Some(item) = Item::first(&intact[at..]).unwrap() {
            let end = at + item.bytes().len();
            if items.len() <= 2 {
                damaged[end - 2] ^= 1;
            }
            let original = Item::first(&damaged[at..end]).unwrap().unwrap();
            let frame = original.frame().unwrap().unwrap();
            let ordinary = frame.check_link(&head, None).effective_identifier;
            if let Some(identifier) = ordinary {
                head = identifier;
            }
            items.push(OriginalItem {
                bytes: at..end,
                ordinary,
            });
            at = end;
        }

        for failure in [None, Some(ParityRepairFailure::VerifyFailed)] {
            let mut candidates = [Candidate::new(0), Candidate::new(1)];
            for (candidate, item) in candidates.iter_mut().zip(&items[1..3]) {
                let offset = item.bytes.end - 2;
                candidate.writes.push(Write {
                    offset,
                    bytes: vec![intact[offset]],
                    data: true,
                });
            }
            candidates[0].failure = failure;
            SegmentInput {
                original: &damaged,
                items: &items,
                opener,
                registry,
            }
            .verify(
                &mut candidates,
                &mut PayloadBudget::new(ReadCapacity::default().payload),
            )
            .unwrap();
            assert_eq!(
                candidates.each_ref().map(|candidate| candidate.failure),
                [failure; 2]
            );
        }
    }

    #[test]
    fn rewalk_accepts_a_linked_parity_envelope_with_a_signature_list() {
        let bytes = include_bytes!("../../../tests/fixtures/v2/P1/parity-signature.bin");
        let first = Item::first(bytes).unwrap().unwrap();
        let header = Header::decode(first).unwrap();
        let mut head = header.identifier().unwrap();
        let registry = header.registry_version();
        let mut at = first.bytes().len();
        while at < 353 {
            let item = Item::first(&bytes[at..]).unwrap().unwrap();
            let frame = item.frame().unwrap().unwrap();
            head = frame.check_link(&head, None).effective_identifier.unwrap();
            at += item.bytes().len();
        }
        assert_eq!(at, 353);
        let item = Item::first(&bytes[at..]).unwrap().unwrap();
        let frame = item.frame().unwrap().unwrap();
        let mut budget = PayloadBudget::new(ReadCapacity::default().payload);
        assert!(
            frame
                .check_repair(&head, registry, &mut budget)
                .unwrap()
                .is_some()
        );
    }
}
