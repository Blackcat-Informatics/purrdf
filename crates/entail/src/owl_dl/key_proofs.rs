// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original membership decisions and checked journals from the pre-key world.

use super::{
    hyper,
    proof::{
        DlProof, DlProofContext, DlProofError, ProofAnswer, RefutationReplay, capture::Capture,
    },
};
use crate::EntailError;
use purrdf_lex::allocation::{Memory, OrderedTable, Resident};
#[cfg(test)]
mod tests;

#[derive(Default)]
pub(crate) enum Records {
    #[default]
    Disabled,
    Pending([u8; 32]),
    Ready(Evidence),
}

impl std::fmt::Debug for Records {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => f.write_str("Disabled"),
            Self::Pending(_) => f.write_str("Pending"),
            Self::Ready(evidence) => f.debug_tuple("Ready").field(evidence).finish(),
        }
    }
}

impl Records {
    pub(crate) fn input(&self) -> Option<[u8; 32]> {
        match self {
            Self::Disabled => None,
            Self::Pending(input) => Some(*input),
            Self::Ready(evidence) => Some(evidence.input),
        }
    }
}

/// Exact native contract and prepared-row frontiers of a key-membership world.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct KeyMembershipPhase {
    /// Original DL contract identity.
    pub contract: [u8; 32],
    /// Equality rows in that prepared phase.
    pub equalities: usize,
    /// Type assertion rows in that prepared phase.
    pub types: usize,
    /// Property assertion rows in that prepared phase.
    pub roles: usize,
    /// Interned native concepts in that prepared phase.
    pub concepts: usize,
    /// Native terminology inclusions in that prepared phase.
    pub tbox: usize,
}

impl std::fmt::Debug for KeyMembershipPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyMembershipPhase")
            .field("contract", &self.contract)
            .field("equalities", &self.equalities)
            .field("types", &self.types)
            .field("roles", &self.roles)
            .field("concepts", &self.concepts)
            .field("tbox", &self.tbox)
            .finish()
    }
}

impl KeyMembershipPhase {
    fn of(ctx: &DlProofContext<'_>) -> Self {
        let kb = &ctx.kb;
        Self {
            contract: ctx.contract(),
            equalities: kb.same_as.len(),
            types: kb.abox_types.len(),
            roles: kb.abox_roles.len(),
            concepts: kb.table.len(),
            tbox: kb.tbox.len(),
        }
    }
}

impl purrdf_hash::error::StructuredError for KeyMembershipPhase {
    purrdf_hash::error_visit! {
        @record self { contract, equalities, types, roles, concepts, tbox } => "owl-key-membership-phase" {
            "contract" => Bytes(&self.contract),
            "equalities" => Unsigned(self.equalities as u128),
            "types" => Unsigned(self.types as u128), "roles" => Unsigned(self.roles as u128),
            "concepts" => Unsigned(self.concepts as u128), "tbox" => Unsigned(self.tbox as u128),
        }
    }
}

/// A key-membership question presented to another source or preparation phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMembershipError {
    /// Original pre-key source identity.
    pub expected_input: [u8; 32],
    /// Source identity supplied by the current checking context.
    pub actual_input: [u8; 32],
    /// Original pre-key contract and native frontiers.
    pub expected: KeyMembershipPhase,
    /// Actual contract and native frontiers.
    pub actual: KeyMembershipPhase,
    /// Original requested key ordinal.
    pub key: usize,
    /// Requested native concept identifier in the actual contract.
    pub requested_concept: u32,
    /// Current key's concept identifier, or absence of that key ordinal.
    pub key_concept: Option<u32>,
}

impl purrdf_hash::error::StructuredError for KeyMembershipError {
    purrdf_hash::error_visit! {
        @record self { expected_input, actual_input, expected, actual, key, requested_concept, key_concept } => "owl-key-membership-mismatch" {
            "expected_input" => Bytes(&self.expected_input), "actual_input" => Bytes(&self.actual_input),
            "expected" => Record(&self.expected), "actual" => Record(&self.actual),
            "key" => Unsigned(self.key as u128), "requested_concept" => Unsigned(u128::from(self.requested_concept)),
            "key_concept" => Value(self.key_concept.map_or(purrdf_hash::error::ErrorValue::Absent, |concept| purrdf_hash::error::ErrorValue::Unsigned(u128::from(concept)))),
        }
    }
}

impl core::fmt::Display for KeyMembershipError {
    fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        output.write_str(
            "key membership checking requires its original pre-key source and class question",
        )
    }
}

impl std::error::Error for KeyMembershipError {}

/// A replay result is explicit even when older data/merge leaves are not closed.
pub(crate) enum Verification {
    Closed(RefutationReplay, Capture),
    Unclosed(RefutationReplay, Capture),
    Refused(DlProofError),
}

impl std::fmt::Debug for Verification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed(report, capture) => f
                .debug_tuple("Closed")
                .field(report)
                .field(capture)
                .finish(),
            Self::Unclosed(report, capture) => f
                .debug_tuple("Unclosed")
                .field(report)
                .field(capture)
                .finish(),
            Self::Refused(error) => f.debug_tuple("Refused").field(error).finish(),
        }
    }
}

pub(crate) struct Membership {
    pub(crate) individual: u32,
    pub(crate) class: u32,
    pub(crate) proof: DlProof,
    pub(crate) verification: Verification,
}

impl std::fmt::Debug for Membership {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Membership")
            .field("individual", &self.individual)
            .field("class", &self.class)
            .field("proof", &self.proof)
            .field("verification", &self.verification)
            .finish()
    }
}

pub(crate) struct Evidence {
    input: [u8; 32],
    pub(crate) phase: KeyMembershipPhase,
    members: OrderedTable<(usize, u32), Membership>,
    storage: Resident,
    live: usize,
}

impl std::fmt::Debug for Evidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Evidence")
            .field("phase", &self.phase)
            .field("members", &self.members.len())
            .field("storage", &self.storage)
            .field("live", &self.live)
            .finish_non_exhaustive()
    }
}

impl Evidence {
    pub(crate) fn for_phase(ctx: &DlProofContext<'_>) -> Self {
        Self {
            input: ctx.input(),
            phase: KeyMembershipPhase::of(ctx),
            members: OrderedTable::default(),
            storage: Resident,
            live: 0,
        }
    }

    /// Observe the original negative membership query, then check all its branches.
    pub(crate) fn entails(
        &mut self,
        key: usize,
        individual: u32,
        class: u32,
        ctx: &DlProofContext<'_>,
    ) -> Result<bool, EntailError> {
        let kb = &ctx.kb;
        let actual = KeyMembershipPhase::of(ctx);
        let key_concept = kb.keys.get(key).map(|(concept, _)| *concept);
        if ctx.input() != self.input || actual != self.phase || key_concept != Some(class) {
            return Err(KeyMembershipError {
                expected_input: self.input,
                actual_input: ctx.input(),
                expected: self.phase,
                actual,
                key,
                requested_concept: class,
                key_concept,
            }
            .into());
        }
        let member = kb.with_negative_instance(individual, class, |assumptions| {
            let budget = super::graph::Budget::for_kb(kb);
            let (decision, recorder) = hyper::decide_recording(kb, assumptions, budget);
            if hyper::decision_result(kb, budget, decision)? {
                return Ok(None);
            }
            let proof =
                recorder.into_proof(kb, self.input, ctx.contract(), ProofAnswer::Inconsistent);
            let verification = match proof.replay_refutation_recording(ctx, assumptions) {
                Ok((report, capture)) if report.is_closed() => {
                    Verification::Closed(report, capture)
                }
                Ok((report, capture)) => Verification::Unclosed(report, capture),
                Err(DlProofError::Stopped) => return Err(EntailError::Stopped),
                Err(DlProofError::Storage(original)) => return Err(EntailError::Storage(original)),
                Err(error) => Verification::Refused(error),
            };
            Ok(Some(Membership {
                individual,
                class,
                proof,
                verification,
            }))
        })?;
        let Some(member) = member else {
            return Ok(false);
        };
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = self.members.insert((key, individual), member, &mut memory);
        self.live = memory.admitted_bytes();
        result?;
        Ok(true)
    }

    pub(crate) fn member(&self, key: usize, individual: u32) -> Option<&Membership> {
        self.members.get(&(key, individual))
    }
}
