// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ordered acceptance of retained undecided rows, without re-querying them.

use purrdf_hash::{Domain, blake3::RecordHasher, frame::frame_le_into, hex::Digest32};
use purrdf_lex::allocation::Admission;

use super::{EvidenceId, EvidenceRecord, EvidenceWorkspace, EvidenceWorkspaceError, ReasonId};

const ACCEPTANCE_DOMAIN: Domain = Domain::new(b"purrdf-core/accepted-row/v1");

/// One retained answer row at its original total-order position.
///
/// The payload owns its original cells, ranks, scores and native admission.
/// Positions come from the original answer order, shared by its established and
/// undecided partitions; they are not dictionary ids or new ranks.
#[derive(Debug)]
pub struct OrderedEvidenceRow<T> {
    /// Position in the original answer's total order.
    pub position: u64,
    /// The original retained payload, moved without recomputation or cloning.
    pub row: T,
    /// The id owned by this answer's still-live evidence workspace.
    pub evidence: EvidenceId,
}

/// A malformed acceptance declaration or retained answer invariant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcceptedMergeError {
    /// Accepted reason identities were not sorted and unique.
    NonCanonicalReasons,
    /// An accepted reason is not part of the original answer's evidence.
    UnknownReason(ReasonId),
    /// A partition descended or repeated an original position.
    InvalidOrder,
    /// Both partitions claimed the same original answer position.
    DuplicatePosition,
    /// The original evidence owner refused or contains an invalid id.
    Evidence(EvidenceWorkspaceError),
}

impl core::fmt::Display for AcceptedMergeError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonCanonicalReasons => {
                out.write_str("accepted reasons must be sorted and unique")
            }
            Self::UnknownReason(reason) => write!(
                out,
                "accepted reason {} is absent from the original answer",
                reason.digest()
            ),
            Self::InvalidOrder => {
                out.write_str("retained answer partition violates its original total order")
            }
            Self::DuplicatePosition => {
                out.write_str("established and undecided partitions share an original position")
            }
            Self::Evidence(error) => error.fmt(out),
        }
    }
}

impl std::error::Error for AcceptedMergeError {}

/// An original row and its separate visible acceptance annotation.
///
/// Its information, incompleteness and reasons are unchanged by an override.
/// Borrowing the original record keeps the native table owner alive throughout
/// projection; acceptance is never silently rewritten as complete evidence.
#[derive(Debug)]
pub struct AcceptedRow<'a, T> {
    /// The original total-order position.
    pub position: u64,
    /// The original retained payload, including its unchanged rank and score.
    pub row: T,
    /// The original immutable evidence, including reasons after acceptance.
    pub evidence: &'a EvidenceRecord,
    accepted: &'a [ReasonId],
}

impl<T> AcceptedRow<'_, T> {
    /// Original reasons explicitly accepted for this row, in canonical order.
    pub fn accepted_reasons(&self) -> impl Iterator<Item = ReasonId> + '_ {
        self.evidence
            .reasons()
            .iter()
            .copied()
            .filter(|reason| self.accepted.binary_search(reason).is_ok())
    }

    /// Whether this original undecided row carries an explicit override.
    #[must_use]
    pub fn is_accepted(&self) -> bool {
        self.accepted_reasons().next().is_some()
    }

    /// Bind the visible override to the caller's original canonical row identity.
    ///
    /// `original` identifies the retained bindings, ranks, scores and evidence.
    /// An established row keeps that identity. An accepted row's new identity
    /// includes its unchanged evidence and every accepted original reason.
    #[must_use]
    pub fn identity(&self, original: Digest32) -> Digest32 {
        let count = self.accepted_reasons().count();
        if count == 0 {
            return original;
        }
        let mut hash = RecordHasher::new();
        frame_le_into(&mut hash, ACCEPTANCE_DOMAIN.as_bytes());
        frame_le_into(&mut hash, original.as_bytes());
        frame_le_into(&mut hash, self.evidence.identity().as_bytes());
        frame_le_into(&mut hash, &(count as u64).to_le_bytes());
        for reason in self.accepted_reasons() {
            frame_le_into(&mut hash, reason.digest().as_bytes());
        }
        Digest32::new(*hash.finalize().as_bytes())
    }
}

/// A lazy ordered merge of already retained answer partitions.
///
/// The two cursors keep at most one original payload each and allocate nothing.
/// No producer is invoked, no score/rank is recomputed, and no reason payload is
/// cloned. The workspace and canonical acceptance declaration remain borrowed
/// until the last returned row has been projected.
#[derive(Debug)]
pub struct AcceptedMerge<'a, T, A, U, S>
where
    A: Iterator<Item = OrderedEvidenceRow<T>>,
    U: Iterator<Item = OrderedEvidenceRow<T>>,
    S: Admission,
{
    established: core::iter::Peekable<A>,
    undecided: core::iter::Peekable<U>,
    workspace: &'a EvidenceWorkspace<S>,
    accepted: &'a [ReasonId],
    previous: Option<u64>,
    failed: bool,
}

impl<'a, T, A, U, S> AcceptedMerge<'a, T, A, U, S>
where
    A: Iterator<Item = OrderedEvidenceRow<T>>,
    U: Iterator<Item = OrderedEvidenceRow<T>>,
    S: Admission,
{
    /// Validate acceptance against the original answer before reading its rows.
    ///
    /// # Errors
    /// Refuses unsorted/repeated/unknown reason identities or an aborted owner.
    pub fn new(
        established: A,
        undecided: U,
        workspace: &'a EvidenceWorkspace<S>,
        accepted: &'a [ReasonId],
    ) -> Result<Self, AcceptedMergeError> {
        if accepted.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(AcceptedMergeError::NonCanonicalReasons);
        }
        let index = workspace
            .canonical_index()
            .map_err(AcceptedMergeError::Evidence)?;
        for reason in accepted {
            let mut found = false;
            for &(_, id) in index {
                let evidence = workspace.get(id).map_err(AcceptedMergeError::Evidence)?;
                if evidence.reasons().binary_search(reason).is_ok() {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(AcceptedMergeError::UnknownReason(*reason));
            }
        }
        Ok(Self {
            established: established.peekable(),
            undecided: undecided.peekable(),
            workspace,
            accepted,
            previous: None,
            failed: false,
        })
    }

    fn pull(&mut self) -> Result<Option<AcceptedRow<'a, T>>, AcceptedMergeError> {
        loop {
            let is_undecided = match (self.established.peek(), self.undecided.peek()) {
                (None, None) => return Ok(None),
                (Some(_), None) => false,
                (None, Some(_)) => true,
                (Some(left), Some(right)) => {
                    if left.position == right.position {
                        return Err(AcceptedMergeError::DuplicatePosition);
                    }
                    right.position < left.position
                }
            };
            let Some(original) = (if is_undecided {
                self.undecided.next()
            } else {
                self.established.next()
            }) else {
                return Err(AcceptedMergeError::InvalidOrder);
            };
            if self
                .previous
                .is_some_and(|position| position >= original.position)
            {
                return Err(AcceptedMergeError::InvalidOrder);
            }
            self.previous = Some(original.position);
            let evidence = self
                .workspace
                .get(original.evidence)
                .map_err(AcceptedMergeError::Evidence)?;
            if is_undecided
                && !evidence
                    .reasons()
                    .iter()
                    .any(|reason| self.accepted.binary_search(reason).is_ok())
            {
                continue;
            }
            return Ok(Some(AcceptedRow {
                position: original.position,
                row: original.row,
                evidence,
                accepted: if is_undecided { self.accepted } else { &[] },
            }));
        }
    }
}

impl<'a, T, A, U, S> Iterator for AcceptedMerge<'a, T, A, U, S>
where
    A: Iterator<Item = OrderedEvidenceRow<T>>,
    U: Iterator<Item = OrderedEvidenceRow<T>>,
    S: Admission,
{
    type Item = Result<AcceptedRow<'a, T>, AcceptedMergeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        match self.pull() {
            Ok(row) => row.map(Ok),
            Err(error) => {
                self.failed = true;
                Some(Err(error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{Frame, InformationMap, ReasonDescription, SearchCompleteness};
    use crate::{TermBox, TermValue};
    use purrdf_lex::allocation::{Memory, Resident};

    fn reason(cause: &str) -> ReasonId {
        let cause = TermValue::iri(cause);
        let concern = TermValue::Triple {
            s: TermBox::new(TermValue::iri("https://example.org/record")),
            p: TermBox::new(TermValue::iri("https://example.org/time")),
            o: TermBox::new(TermValue::simple_literal("missing")),
        };
        ReasonDescription {
            cause: &cause,
            concern: &concern,
            constraint: None,
            vault: None,
            watermark: None,
        }
        .identity(&mut Memory::new(&mut Resident))
        .unwrap()
    }

    #[derive(Debug)]
    struct Retained {
        rank: u64,
        score: i128,
    }

    fn row(position: u64, evidence: EvidenceId) -> OrderedEvidenceRow<Retained> {
        OrderedEvidenceRow {
            position,
            row: Retained {
                rank: position,
                score: 1000 - i128::from(position),
            },
            evidence,
        }
    }

    #[test]
    fn accepted_rows_keep_original_order_payload_evidence_and_visible_override() {
        let first = reason("https://example.org/missing-time");
        let second = reason("https://example.org/missing-place");
        let mut accepted = [first, second];
        accepted.sort_unstable();
        let mut workspace = EvidenceWorkspace::with_admission(Resident);
        let incomplete = workspace
            .intern(|memory| {
                EvidenceRecord::from_parts(
                    &InformationMap::default(),
                    SearchCompleteness::Undetermined,
                    &Frame::actual(),
                    &accepted,
                    &[],
                    memory,
                )
            })
            .unwrap();
        let established = [row(1, EvidenceId::DEFAULT), row(5, EvidenceId::DEFAULT)].into_iter();
        let undecided = [row(2, incomplete), row(3, incomplete)].into_iter();
        let merged =
            AcceptedMerge::new(established, undecided, &workspace, &accepted[..1]).unwrap();
        let mut positions = Vec::new();
        for item in merged {
            let item = item.unwrap();
            positions.push(item.position);
            assert_eq!(item.row.rank, item.position);
            assert_eq!(item.row.score, 1000 - i128::from(item.position));
            let original = Digest32::new([7; 32]);
            if item.position == 2 || item.position == 3 {
                assert!(item.is_accepted());
                assert_eq!(
                    item.evidence.completeness(),
                    SearchCompleteness::Undetermined
                );
                assert_eq!(item.evidence.reasons(), &accepted);
                assert_eq!(item.accepted_reasons().collect::<Vec<_>>(), accepted[..1]);
                assert_ne!(item.identity(original), original);
            } else {
                assert!(!item.is_accepted());
                assert_eq!(item.identity(original), original);
            }
        }
        assert_eq!(positions, [1, 2, 3, 5]);
        let one = AcceptedMerge::new(
            core::iter::empty(),
            [row(2, incomplete)].into_iter(),
            &workspace,
            &accepted[..1],
        )
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .identity(Digest32::new([7; 32]));
        let both = AcceptedMerge::new(
            core::iter::empty(),
            [row(2, incomplete)].into_iter(),
            &workspace,
            &accepted,
        )
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .identity(Digest32::new([7; 32]));
        assert_ne!(one, both);
    }

    #[test]
    fn unaccepted_rows_remain_excluded_and_order_errors_stay_typed() {
        let workspace = EvidenceWorkspace::with_admission(Resident);
        let mut excluded = AcceptedMerge::new(
            core::iter::empty(),
            [row(2, EvidenceId::DEFAULT)].into_iter(),
            &workspace,
            &[],
        )
        .unwrap();
        assert!(excluded.next().is_none());
        let mut duplicate = AcceptedMerge::new(
            [row(1, EvidenceId::DEFAULT)].into_iter(),
            [row(1, EvidenceId::DEFAULT)].into_iter(),
            &workspace,
            &[],
        )
        .unwrap();
        assert!(matches!(
            duplicate.next(),
            Some(Err(AcceptedMergeError::DuplicatePosition))
        ));
        assert!(duplicate.next().is_none());
        let mut descending = AcceptedMerge::new(
            [row(2, EvidenceId::DEFAULT), row(1, EvidenceId::DEFAULT)].into_iter(),
            core::iter::empty(),
            &workspace,
            &[],
        )
        .unwrap();
        assert!(descending.next().unwrap().is_ok());
        assert!(matches!(
            descending.next(),
            Some(Err(AcceptedMergeError::InvalidOrder))
        ));
        let unknown = [reason("https://example.org/unknown")];
        assert!(matches!(
            AcceptedMerge::new(
                [row(1, EvidenceId::DEFAULT)].into_iter(),
                core::iter::empty(),
                &workspace,
                &unknown
            ),
            Err(AcceptedMergeError::UnknownReason(_))
        ));
    }
}
