// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use core::num::NonZeroU64;
use std::collections::TryReserveError;

use purrdf_lex::cbor::StructuralError;

use super::index::{IndexContent, TypeRuns};
use super::parity::ParityHistory;
use super::rows::OrderKey;
use super::{
    CommitCheck, ContentError, DIGEST_LENGTH, DecodedRows, DecodedTerms, FormedBudget, FrameCheck,
    FrameEnvelope, FrameField, FrameType, IndexPrefixCheck, ParityCheck, PayloadBudget,
    PayloadResourceError, PayloadState, Range, RangeFull, RowHeader, TermReservation,
    UnknownFrameType,
};
use super::{RowOrder, terms};

type Digest = [u8; DIGEST_LENGTH];

/// Independent checks for a frame and any commit claim it carries.
#[derive(Debug)]
#[non_exhaustive]
pub struct SegmentFrame<'a> {
    /// Envelope, link and payload checks.
    pub frame: FrameCheck<'a>,
    /// Result of checking a commit against the observed prefix.
    pub commit: Option<CommitCheck>,
    /// Index grammar and comparisons with the observed ordinary prefix.
    pub index: Option<Result<IndexPrefixCheck, ContentError>>,
    /// Dictionary reservation when this item is a terms frame.
    pub terms: Option<TermReservation<'a>>,
    /// The content record check of a row frame that reaches interpretation.
    pub rows: Option<Result<RowHeader, ContentError>>,
    /// Parity frame and table admission before stream placement and shard checks.
    pub parity: Option<ParityCheck<'a>>,
    row_order: Option<(usize, Option<OrderKey>)>,
}

impl SegmentFrame<'_> {
    fn decode_terms(
        &self,
        budget: &mut FormedBudget,
    ) -> Option<Result<DecodedTerms<'_>, ContentError>> {
        let header = self.terms.as_ref()?.content.as_ref()?.as_ref().ok()?;
        let PayloadState::Verified(payload) = &self.frame.payload.state else {
            return None;
        };
        Some(header.decode(payload, budget))
    }
}

/// A resource or range failure while checking a segment.
#[derive(Debug)]
#[non_exhaustive]
pub enum SegmentCheckError {
    /// The output buffer could not be admitted on this host.
    Payload(PayloadResourceError),
    /// The segment exceeds the range leaf count.
    Range(RangeFull),
    /// A structural read failed while locating an independently reserved field.
    Framing(StructuralError),
    /// The allocator refused to record a parity span for duplicate checking.
    ParityHistory(TryReserveError),
    /// The allocator refused to record a run of ordinary frame types.
    IndexHistory(TryReserveError),
}

impl fmt::Display for SegmentCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Payload(error) => error.fmt(f),
            Self::Range(error) => error.fmt(f),
            Self::Framing(error) => error.fmt(f),
            Self::ParityHistory(error) => {
                write!(f, "could not record the GTS parity span ({error})")
            }
            Self::IndexHistory(error) => {
                write!(f, "could not record the GTS frame type run ({error})")
            }
        }
    }
}

impl std::error::Error for SegmentCheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Payload(error) => Some(error),
            Self::Range(error) => Some(error),
            Self::Framing(error) => Some(error),
            Self::ParityHistory(error) | Self::IndexHistory(error) => Some(error),
        }
    }
}

impl purrdf_hash::error::StructuredError for SegmentCheckError {
    purrdf_hash::error_visit! {
        Payload(cause) => "gts-v2-segment-payload-resource" { "cause" => Cause(cause) },
        Range(cause) => "gts-v2-segment-range-full" { "cause" => Cause(cause) },
        Framing(cause) => "gts-v2-segment-framing" { "cause" => Cause(cause) },
        ParityHistory(cause) => "gts-v2-segment-parity-history" { "cause" => Cause(cause) },
        IndexHistory(cause) => "gts-v2-segment-index-history" { "cause" => Cause(cause) },
    }
}

/// Verify the linked frames and range of a generation 2 segment.
#[derive(Debug)]
pub struct SegmentVerifier {
    registry: NonZeroU64,
    head: Digest,
    resume_anchor: Option<Digest>,
    range: Range,
    parity: ParityHistory,
    incomplete_prefix: bool,
    types: TypeRuns,
    next_term: u128,
    content_stopped: bool,
    last_rows: [Option<OrderKey>; 3],
}

impl SegmentVerifier {
    /// Begin at the computed identifier of a readable opener.
    #[must_use]
    pub fn new(opener: Digest, registry: NonZeroU64) -> Self {
        Self {
            registry,
            head: opener,
            resume_anchor: None,
            range: Range::new(&opener),
            parity: ParityHistory::new(opener),
            incomplete_prefix: false,
            types: TypeRuns::default(),
            next_term: 0,
            content_stopped: false,
            last_rows: [None; 3],
        }
    }

    /// The effective identifier of the last ordinary frame, or the opener.
    #[must_use]
    pub const fn head(&self) -> &Digest {
        &self.head
    }

    pub(super) const fn incomplete_prefix(&self) -> bool {
        self.incomplete_prefix
    }

    pub(super) const fn ordinary_count(&self) -> u64 {
        self.range.count().get() - 1
    }

    pub(super) fn predecessor(&self) -> &Digest {
        self.resume_anchor.as_ref().unwrap_or(&self.head)
    }

    pub(super) const fn parity_history(&self) -> &ParityHistory {
        &self.parity
    }

    pub(super) fn resume_parity(
        &mut self,
        anchor: Digest,
        table: &super::ParityTable<'_>,
    ) -> Result<bool, TryReserveError> {
        self.parity.record_resumption(table)?;
        let detached = self.head != anchor;
        self.incomplete_prefix |= detached;
        self.resume_anchor = Some(anchor);
        Ok(detached)
    }

    /// Whether an unknown critical type or enumeration has stopped content.
    #[must_use]
    pub const fn content_stopped(&self) -> bool {
        self.content_stopped
    }

    pub(super) fn stop_content(&mut self) {
        self.content_stopped = true;
    }

    /// Interpret the current checked terms frame before advancing to the next item.
    ///
    /// An unknown enumeration stops interpretation of later content in this
    /// segment. Their well-formed runs still reserve dictionary IDs.
    pub fn decode_terms<'a>(
        &mut self,
        frame: &'a SegmentFrame<'_>,
        budget: &mut FormedBudget,
    ) -> Option<Result<DecodedTerms<'a>, ContentError>> {
        if self.content_stopped {
            return None;
        }
        let result = frame.decode_terms(budget)?;
        self.content_stopped |= matches!(result, Err(ContentError::UnknownEnumeration { .. }));
        Some(result)
    }

    /// Interpret the current checked row frame before advancing to the next item.
    pub fn decode_rows(
        &mut self,
        frame: &SegmentFrame<'_>,
    ) -> Option<Result<DecodedRows, ContentError>> {
        if self.content_stopped {
            return None;
        }
        let header = frame.rows.as_ref()?.as_ref().ok()?;
        let PayloadState::Verified(payload) = &frame.frame.payload.state else {
            return None;
        };
        let result = header.decode(payload).map(|mut decoded| {
            if let Some((slot, previous)) = frame.row_order
                && decoded.order == Some(RowOrder::Canonical)
            {
                if let (Some(previous), Some(first)) = (previous, decoded.rows.first()) {
                    decoded.declared_order_violated |= previous >= first.order_key();
                }
                self.last_rows[slot] = decoded.rows.last().map(|row| row.order_key());
            }
            decoded
        });
        self.content_stopped |= matches!(result, Err(ContentError::UnknownEnumeration { .. }));
        Some(result)
    }

    pub(super) fn omit_row_order(&mut self, frame: &SegmentFrame<'_>) {
        if let Some((slot, _)) = frame.row_order {
            self.last_rows[slot] = None;
        }
    }

    /// Verify a framed item and advance the ordinary prefix.
    ///
    /// Supply a witness only when the immediately following item is a frame
    /// with final framing. Share the payload budget across segments.
    ///
    /// # Errors
    /// Returns a resource, framing, or range failure.
    pub fn check<'a>(
        &mut self,
        frame: &FrameEnvelope<'a>,
        witness: Option<&FrameEnvelope<'a>>,
        budget: &mut PayloadBudget,
    ) -> Result<SegmentFrame<'a>, SegmentCheckError> {
        let mut next_budget = budget.clone();
        let checked = frame
            .check(self.predecessor(), witness, self.registry, &mut next_budget)
            .map_err(SegmentCheckError::Payload)?;
        let accepted = self.accept_checked(frame, checked, budget, None)?;
        *budget = next_budget;
        Ok(accepted)
    }

    pub(super) fn accept_checked<'a>(
        &mut self,
        frame: &FrameEnvelope<'a>,
        checked: FrameCheck<'a>,
        budget: &PayloadBudget,
        index_content: Option<&IndexContent>,
    ) -> Result<SegmentFrame<'a>, SegmentCheckError> {
        let commit = checked.commit_claim.map(|claim| {
            let mut commit = self.range.check_commit(claim, checked.link.state);
            if self.incomplete_prefix {
                commit.root_matches = false;
                commit.valid = false;
            }
            commit
        });
        let frame_type = checked.frame_type.and_then(FrameType::from_code);
        let index = if frame_type == Some(FrameType::Index) {
            let content = index_content.filter(|_| !self.incomplete_prefix);
            self.types
                .inspect(
                    frame,
                    &checked,
                    self.registry,
                    &self.head,
                    &self.range,
                    content,
                )
                .map(|result| {
                    result.map(|mut index| {
                        if self.incomplete_prefix {
                            index.head_matches = false;
                            index.root_matches = false;
                            index.type_runs_match = false;
                        }
                        index
                    })
                })
        } else {
            None
        };
        let terms = if frame_type == Some(FrameType::Terms) {
            Some(
                terms::reserve(
                    frame,
                    &checked,
                    self.next_term,
                    self.registry,
                    budget.admits_content(frame.field(FrameField::Content).len() as u64),
                    self.content_stopped,
                )
                .map_err(SegmentCheckError::Framing)?,
            )
        } else {
            None
        };
        let rows = if self.content_stopped {
            None
        } else {
            RowHeader::inspect(frame, &checked, self.registry)
        };
        let parity = if frame_type == Some(FrameType::Parity) {
            let predecessor = *self.predecessor();
            Some(
                self.parity
                    .inspect(
                        frame,
                        &checked,
                        &predecessor,
                        self.registry,
                        self.content_stopped,
                    )
                    .map_err(SegmentCheckError::ParityHistory)?,
            )
        } else {
            None
        };
        if let Some(identifier) = checked.link.effective_identifier {
            self.types
                .reserve_next(checked.frame_type)
                .map_err(SegmentCheckError::IndexHistory)?;
            self.range
                .append(&identifier)
                .map_err(SegmentCheckError::Range)?;
            self.head = identifier;
            self.resume_anchor = None;
            self.types.record(checked.frame_type);
        }
        if let Some(terms) = &terms {
            self.next_term = terms.end;
            self.content_stopped |= matches!(
                terms.content,
                Some(Err(ContentError::UnknownEnumeration { .. }))
            );
        }
        self.content_stopped |= matches!(checked.unknown_type, Some(UnknownFrameType::Critical(_)));
        self.content_stopped |= matches!(rows, Some(Err(ContentError::UnknownEnumeration { .. })));
        let row_order = match frame_type {
            Some(FrameType::Quads) => Some(0),
            Some(FrameType::ReifierBindings) => Some(1),
            Some(FrameType::Annotations) => Some(2),
            _ => None,
        }
        .map(|slot| (slot, self.last_rows[slot].take()));
        Ok(SegmentFrame {
            frame: checked,
            commit,
            index,
            terms,
            rows,
            parity: parity.map(|observation| self.parity.record(observation)),
            row_order,
        })
    }
}
