// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Corpus-independent posting envelopes retained by the persistent table.

use super::{CorpusProfile, Posting, Shared, Table, TextError, share_index};
use crate::query_workspace::{admitted, data_error};
use crate::ranking::FieldEnvelope;
use purrdf_sparql_eval::{AdmittedVec, WorkspaceCapability};

type NativeRankedPosting<'a> = (u32, &'a [(u32, u64)], bool);

/// Owned native ranked cursor. Canonical ranks are resolved against the pinned
/// snapshot; retained summaries themselves never persist shifting ranks.
#[derive(Debug)]
pub struct IndexRankedPostings {
    term: Option<usize>,
    at: usize,
    end: usize,
    examined_through: usize,
}

impl<P: CorpusProfile> super::TextIndex<P> {
    pub(crate) fn open_ranked_native(
        &self,
        partition: &super::PartitionKey<P>,
        term: &str,
        workspace: &WorkspaceCapability,
    ) -> Result<IndexRankedPostings, TextError> {
        let term = self
            .terms
            .binary_search_by(|entry| entry.term.as_str().cmp(term))
            .ok();
        let Some(ordinal) = term else {
            return Ok(IndexRankedPostings {
                term,
                at: 0,
                end: 0,
                examined_through: 0,
            });
        };
        let postings = &self.terms[ordinal].postings;
        let compare = |posting: &Shared<Posting<P>>| -> Result<core::cmp::Ordering, purrdf_sparql_eval::EvalError> {
            Ok(super::compare_graphs_owned(P::graph(&posting.document.graph), P::graph(&partition.graph), workspace)?.then_with(|| posting.document.language.cmp(&partition.language)))
        };
        let at = admitted(
            postings
                .try_partition_point(|posting| compare(posting).map(core::cmp::Ordering::is_lt)),
        )?;
        let end = admitted(
            postings.try_partition_point(|posting| compare(posting).map(|order| !order.is_gt())),
        )?;
        Ok(IndexRankedPostings {
            term,
            at,
            end,
            examined_through: at,
        })
    }
    pub(crate) fn ranked_window_native(
        &self,
        cursor: &IndexRankedPostings,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<crate::RankedWindow>, TextError> {
        if cursor.at == cursor.end {
            return Ok(None);
        }
        let postings = &self.terms[cursor.term.expect("nonempty cursor has a term")].postings;
        let (_, end) = postings.leaf_bounds(cursor.at);
        let end = end.min(cursor.end);
        Ok(Some(crate::RankedWindow {
            document: admitted(self.document_rank_owned(&postings[cursor.at].document, workspace))?,
            block_end: admitted(self.document_rank_owned(&postings[end - 1].document, workspace))?,
            block: postings.range_summary(cursor.at, end, workspace)?.fields,
            suffix: postings
                .range_summary(cursor.at, cursor.end, workspace)?
                .fields,
        }))
    }
    pub(crate) fn read_ranked_native(
        &self,
        cursor: &mut IndexRankedPostings,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<NativeRankedPosting<'_>>, TextError> {
        if cursor.at == cursor.end {
            return Ok(None);
        }
        let postings = &self.terms[cursor.term.expect("nonempty cursor has a term")].postings;
        let posting = &postings[cursor.at];
        let document = admitted(self.document_rank_owned(&posting.document, workspace))?;
        let new_block = cursor.at >= cursor.examined_through;
        if new_block {
            cursor.examined_through = postings.leaf_bounds(cursor.at).1;
        }
        cursor.at += 1;
        Ok(Some((document, &posting.predicate_frequencies, new_block)))
    }
    pub(crate) fn seek_ranked_native(
        &self,
        cursor: &mut IndexRankedPostings,
        target: Option<u32>,
        workspace: &WorkspaceCapability,
    ) -> Result<u64, TextError> {
        let Some(term) = cursor.term else {
            return Ok(0);
        };
        let postings = &self.terms[term].postings;
        let mut low = cursor.at;
        let mut high = cursor.end;
        if let Some(target) = target {
            while low < high {
                let middle = low + (high - low) / 2;
                let document =
                    admitted(self.document_rank_owned(&postings[middle].document, workspace))?;
                if document < target {
                    low = middle + 1;
                } else {
                    high = middle;
                }
            }
        } else {
            low = high;
        }
        let skipped = postings.blocks_in_range(cursor.at.max(cursor.examined_through), low);
        cursor.at = low;
        Ok(skipped)
    }
}

#[derive(Clone, Debug)]
pub(super) struct PostingPolicy {
    routing: Shared<Vec<usize>>,
    fields: usize,
}
impl PostingPolicy {
    pub(super) fn new(routing: Vec<usize>, fields: usize) -> Result<Self, TextError> {
        Ok(Self {
            routing: share_index(routing)?,
            fields,
        })
    }
    pub(super) fn matches(&self, routing: &[usize], fields: usize) -> bool {
        self.routing.as_slice() == routing && self.fields == fields
    }
}
#[derive(Debug)]
pub(super) struct PostingSummary {
    fields: AdmittedVec<FieldEnvelope>,
    blocks: u64,
}

pub(super) type PostingTable<P> = Table<Shared<Posting<P>>, PostingPolicy>;

impl<P: CorpusProfile> super::token_streams::Summary<Shared<Posting<P>>> for PostingPolicy {
    type Value = PostingSummary;
    fn blocks(&self, value: &Self::Value) -> u64 {
        value.blocks
    }
    fn leaf(
        &self,
        items: &[Shared<Posting<P>>],
        workspace: &WorkspaceCapability,
    ) -> Result<Self::Value, TextError> {
        let mut output = admitted(AdmittedVec::with_capacity(self.fields, workspace))?;
        for _ in 0..self.fields {
            admitted(output.push(FieldEnvelope::default()))?;
        }
        for posting in items {
            let fields = crate::source::assemble_fields_owned(
                0,
                posting.document.predicate_lengths(),
                &posting.predicate_frequencies,
                self.fields,
                workspace,
                |predicate| {
                    self.routing
                        .get(predicate as usize)
                        .copied()
                        .ok_or_else(|| {
                            data_error("posting names an absent predicate", Some(workspace))
                        })
                },
            )?;
            for (envelope, input) in output.iter_mut().zip(fields.iter()) {
                envelope.include(*input);
            }
        }
        Ok(PostingSummary {
            fields: output,
            blocks: u64::from(!items.is_empty()),
        })
    }
    fn branch(
        &self,
        left: &Self::Value,
        right: &Self::Value,
        workspace: &WorkspaceCapability,
    ) -> Result<Self::Value, TextError> {
        let mut output = admitted(AdmittedVec::with_capacity(self.fields, workspace))?;
        for (a, b) in left.fields.iter().zip(right.fields.iter()) {
            admitted(output.push(a.union(*b)))?;
        }
        Ok(PostingSummary {
            fields: output,
            blocks: left
                .blocks
                .checked_add(right.blocks)
                .ok_or_else(crate::query_workspace::overflow)?,
        })
    }
}
