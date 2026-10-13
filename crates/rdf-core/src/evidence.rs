// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent information, search completeness and aggregate-bound laws.
//!
//! A positive and a negative witness are independent facts. Search completeness
//! neither removes a witness nor turns a contradiction into missing knowledge.
//! These operations are shared by query evaluation and rule derivation.

mod column;
pub use column::{EvidenceColumn, EvidenceColumnError};
mod answer;
pub use answer::{
    AnswerAccumulator, AnswerError, AnswerRequest, AnswerStatus, EvidencePartition, ReasonCount,
};
mod standpoint;
pub use standpoint::{StandpointError, StandpointOrder};
mod frame;
pub use frame::{Frame, FrameComponent};
mod reason;
pub use reason::{
    ReasonDescription, ReasonError, ReasonId, ReasonTable, ReasonTableError, RetainedReason,
};
mod conversion;
pub use conversion::{LossCauseError, LossCauses};
mod ledger;
pub use ledger::{
    AnnotationStatements, AnswerLedger, BorrowedLedgerTerms, FrameStatements, FrameVocabulary,
    LedgerBuildError, LedgerDataset, LedgerError, LedgerNodeKey, LedgerNodes, LedgerReadError,
    LedgerRow, LedgerSource, LedgerStatement, LedgerTermId, LedgerVocabulary, ReasonStatements,
    ReasonVocabulary, RowStatements, RowVocabulary,
};
mod information;
pub use information::{InformationMap, StandpointKey};
mod record;
pub use record::{EvidenceRecord, EvidenceRecordError};
mod dictionary;
pub use dictionary::{EvidenceDictionary, EvidenceDictionaryError};
mod workspace;
pub use workspace::{EvidenceWorkspace, EvidenceWorkspaceError};
mod accepted;
pub use accepted::{AcceptedMerge, AcceptedMergeError, AcceptedRow, OrderedEvidenceRow};

/// The operator law used to combine two original rows' evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EvidenceCombination {
    /// Joined premises: truth meet, worse completeness and frame conjunction.
    Conjunction,
    /// Coalesced identical bindings: information join and alternative witnesses.
    Alternatives,
}

/// Belnap's FOUR: independent support and opposition within one standpoint.
///
/// Derived ordering is canonical encoding order, neither lattice's order.
/// Use the named lattice operations to combine information.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Information {
    /// Neither polarity has a witness.
    Neither = 0,
    /// A positive witness exists.
    #[default]
    Supported = 1,
    /// A negative witness exists.
    Opposed = 2,
    /// Both polarities have witnesses; this is a contradiction, not ignorance.
    Both = 3,
}

impl Information {
    /// Whether a positive witness exists.
    #[must_use]
    pub const fn supports(self) -> bool {
        self as u8 & 1 != 0
    }

    /// Whether a negative witness exists.
    #[must_use]
    pub const fn opposes(self) -> bool {
        self as u8 & 2 != 0
    }

    /// Conjunction in truth order: intersect support and unite opposition.
    #[must_use]
    pub const fn truth_meet(self, other: Self) -> Self {
        Self::from_polarities(
            self.supports() && other.supports(),
            self.opposes() || other.opposes(),
        )
    }

    /// Alternative witnesses in information order: unite both polarities.
    #[must_use]
    pub const fn information_join(self, other: Self) -> Self {
        Self::from_polarities(
            self.supports() || other.supports(),
            self.opposes() || other.opposes(),
        )
    }

    /// Swap polarities, preserving contradiction and absence of information.
    #[must_use]
    pub const fn negate(self) -> Self {
        Self::from_polarities(self.opposes(), self.supports())
    }

    /// Construct from two independent witness facts.
    #[must_use]
    pub const fn from_polarities(supported: bool, opposed: bool) -> Self {
        match (supported, opposed) {
            (false, false) => Self::Neither,
            (true, false) => Self::Supported,
            (false, true) => Self::Opposed,
            (true, true) => Self::Both,
        }
    }
}

/// Whether the input search has established its whole declared scope.
///
/// Ordering is worsening completeness, independently of information state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SearchCompleteness {
    /// The whole declared scope was evaluated.
    #[default]
    Complete,
    /// Evaluated input leaves a semantic gap, with separately retained reasons.
    Undetermined,
    /// The required scope has not been evaluated.
    NotEvaluated,
}

impl SearchCompleteness {
    /// A conjunction is no more complete than either input.
    #[must_use]
    pub const fn worse(self, other: Self) -> Self {
        if self as u8 >= other as u8 {
            self
        } else {
            other
        }
    }

    /// Duplicate alternative witnesses retain the better-established evidence.
    #[must_use]
    pub const fn better(self, other: Self) -> Self {
        if self as u8 <= other as u8 {
            self
        } else {
            other
        }
    }

    /// Absence is decidable only after complete search under a declared closure.
    #[must_use]
    pub const fn admits_absence(self, closure_declared: bool) -> bool {
        matches!(self, Self::Complete) && closure_declared
    }
}

/// The relationship of an aggregate over retained rows to its complete value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AggregateBound {
    /// The aggregate's declared input scope is complete.
    #[default]
    Exact,
    /// Missing rows can only increase the complete value.
    Lower,
    /// Missing rows can only decrease the complete value.
    Upper,
    /// Neither directional bound is justified.
    Undetermined,
}

/// Aggregate monotonicity, established from the actual expression and inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AggregateLaw {
    /// COUNT and MAX; SUM only when its domain is established nonnegative.
    Increasing,
    /// MIN.
    Decreasing,
    /// AVG and SUM whose domain does not establish nonnegative terms.
    NonMonotone,
}

/// Stable identity of a projected aggregate column, independent of row ordinal
/// or query-local dictionary insertion order.
#[must_use]
pub fn aggregate_column_identity(name: &str) -> purrdf_hash::hex::Digest32 {
    let mut hash = purrdf_hash::blake3::RecordHasher::new();
    purrdf_hash::frame::frame_le_into(
        &mut hash,
        purrdf_hash::Domain::new(b"purrdf-core/evidence-aggregate-column/v1").as_bytes(),
    );
    purrdf_hash::frame::frame_le_into(&mut hash, name.as_bytes());
    purrdf_hash::hex::Digest32::new(*hash.finalize().as_bytes())
}

impl AggregateLaw {
    /// Label a value without representing a partial total as exact.
    #[must_use]
    pub const fn bound(self, completeness: SearchCompleteness) -> AggregateBound {
        if matches!(completeness, SearchCompleteness::Complete) {
            return AggregateBound::Exact;
        }
        match self {
            Self::Increasing => AggregateBound::Lower,
            Self::Decreasing => AggregateBound::Upper,
            Self::NonMonotone => AggregateBound::Undetermined,
        }
    }
}

/// A small query-local reference into its original evidence dictionary.
///
/// Zero denotes supported, complete and actual. Nonzero references are minted
/// by that dictionary; they are not global identities or serialization order.
/// The physical usize representation imposes no independent semantic ceiling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceId(usize);

impl EvidenceId {
    /// The allocation-free common-case reference.
    pub const DEFAULT: Self = Self(0);

    /// Whether this is the common supported/complete/actual evidence.
    #[must_use]
    pub const fn is_default(self) -> bool {
        self.0 == 0
    }
}

/// The evidence projection requested by a query, in increasing detail.
///
/// Ids are tracked at every tier; a lower tier does not render higher-tier data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnswerTier {
    /// One witness verdict, or ordinary rows with one answer-level status.
    #[default]
    Verdict,
    /// Also report counts grouped by the original semantic reasons.
    Summary,
    /// Also expose each established and undecided row's original evidence.
    Rows,
    /// Also expose the lazily rendered queryable RDF 1.2 ledger.
    Ledger,
}

/// The question whose witnesses can settle a verdict before full enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WitnessQuestion {
    /// A positive witness settles support; it does not exclude later opposition.
    Support,
    /// Both independent polarities are required to establish contradiction.
    Contradiction,
    /// A negative answer requires exhausted complete search and declared closure.
    ClosedAbsence,
    /// Determine the full FOUR value; first support alone cannot settle it.
    Classify,
}

/// A settled answer to the requested witness question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessVerdict {
    /// The requested fact has been established.
    Yes,
    /// A counterexample, or complete closed search, excludes the requested fact.
    No,
    /// Exhaustion left a semantic gap or no applicable closure declaration.
    Unknown,
}

impl WitnessQuestion {
    /// Settle only the requested fact, retaining independence of both polarities.
    ///
    /// `None` means evaluation must continue. Operational failures are returned
    /// by the execution caller and must never be supplied as search incompleteness.
    #[must_use]
    pub const fn settle(
        self,
        information: Information,
        completeness: SearchCompleteness,
        exhausted: bool,
        closure_declared: bool,
    ) -> Option<WitnessVerdict> {
        if matches!(self, Self::ClosedAbsence) && information.supports() {
            return Some(WitnessVerdict::No);
        }
        let established = match self {
            Self::Support => information.supports(),
            Self::Contradiction => matches!(information, Information::Both),
            Self::ClosedAbsence => false,
            Self::Classify => matches!(information, Information::Both),
        };
        if established {
            return Some(WitnessVerdict::Yes);
        }
        if matches!(self, Self::ClosedAbsence) && !completeness.admits_absence(closure_declared) {
            return Some(WitnessVerdict::Unknown);
        }
        if !exhausted {
            return None;
        }
        if !completeness.admits_absence(closure_declared) {
            return Some(WitnessVerdict::Unknown);
        }
        match self {
            Self::ClosedAbsence if !information.supports() => Some(WitnessVerdict::Yes),
            Self::Classify => Some(WitnessVerdict::Yes),
            Self::Support | Self::Contradiction | Self::ClosedAbsence => Some(WitnessVerdict::No),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AggregateBound, AggregateLaw, Information, SearchCompleteness, WitnessQuestion,
        WitnessVerdict,
    };

    #[test]
    fn truth_conjunction_and_alternative_information_have_distinct_tables() {
        use Information::{Both, Neither, Opposed, Supported};
        let values = [Neither, Supported, Opposed, Both];
        let truth = [
            [Neither, Neither, Opposed, Opposed],
            [Neither, Supported, Opposed, Both],
            [Opposed, Opposed, Opposed, Opposed],
            [Opposed, Both, Opposed, Both],
        ];
        let alternative = [
            [Neither, Supported, Opposed, Both],
            [Supported, Supported, Both, Both],
            [Opposed, Both, Opposed, Both],
            [Both, Both, Both, Both],
        ];
        for (left, a) in values.iter().enumerate() {
            for (right, b) in values.iter().enumerate() {
                assert_eq!(a.truth_meet(*b), truth[left][right]);
                assert_eq!(a.information_join(*b), alternative[left][right]);
            }
            assert_eq!(a.negate().negate(), *a);
        }
        assert_eq!(Both.negate(), Both);
        assert_eq!(Neither.negate(), Neither);
    }

    #[test]
    fn support_cannot_prematurely_settle_contradiction_or_classification() {
        use Information::{Both, Neither, Supported};
        use SearchCompleteness::{Complete, Undetermined};
        use WitnessVerdict::{No, Unknown, Yes};
        assert_eq!(
            WitnessQuestion::Support.settle(Supported, Undetermined, false, false),
            Some(Yes)
        );
        assert_eq!(
            WitnessQuestion::Contradiction.settle(Supported, Complete, false, true),
            None
        );
        assert_eq!(
            WitnessQuestion::Classify.settle(Supported, Complete, false, true),
            None
        );
        assert_eq!(
            WitnessQuestion::Contradiction.settle(Both, Undetermined, false, false),
            Some(Yes)
        );
        assert_eq!(
            WitnessQuestion::ClosedAbsence.settle(Neither, Complete, true, false),
            Some(Unknown)
        );
        assert_eq!(
            WitnessQuestion::ClosedAbsence.settle(Neither, Undetermined, true, true),
            Some(Unknown)
        );
        assert_eq!(
            WitnessQuestion::ClosedAbsence.settle(Neither, Undetermined, false, true),
            Some(Unknown)
        );
        assert_eq!(
            WitnessQuestion::ClosedAbsence.settle(Neither, Complete, false, false),
            Some(Unknown)
        );
        assert_eq!(
            WitnessQuestion::ClosedAbsence.settle(Neither, Complete, true, true),
            Some(Yes)
        );
        assert_eq!(
            WitnessQuestion::ClosedAbsence.settle(Supported, Complete, true, true),
            Some(No)
        );
        assert_eq!(
            WitnessQuestion::ClosedAbsence.settle(Supported, Undetermined, false, false),
            Some(No)
        );
    }

    #[test]
    fn missing_inputs_do_not_turn_partial_totals_into_exact_values() {
        for completeness in [
            SearchCompleteness::Undetermined,
            SearchCompleteness::NotEvaluated,
        ] {
            assert_eq!(
                AggregateLaw::Increasing.bound(completeness),
                AggregateBound::Lower
            );
            assert_eq!(
                AggregateLaw::Decreasing.bound(completeness),
                AggregateBound::Upper
            );
            assert_eq!(
                AggregateLaw::NonMonotone.bound(completeness),
                AggregateBound::Undetermined
            );
        }
        assert_eq!(
            AggregateLaw::NonMonotone.bound(SearchCompleteness::Complete),
            AggregateBound::Exact
        );
    }
}
