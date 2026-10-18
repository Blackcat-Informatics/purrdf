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

pub(crate) struct Phase {
    contract: [u8; 32],
    pub(crate) equalities: usize,
    pub(crate) types: usize,
    pub(crate) roles: usize,
    pub(crate) concepts: usize,
    pub(crate) tbox: usize,
    pub(crate) metas: Vec<(u32, usize)>,
}

impl std::fmt::Debug for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Phase")
            .field("contract", &self.contract)
            .field("equalities", &self.equalities)
            .field("types", &self.types)
            .field("roles", &self.roles)
            .field("concepts", &self.concepts)
            .field("tbox", &self.tbox)
            .field("metas", &self.metas)
            .finish()
    }
}

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
    pub(crate) phase: Phase,
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
    pub(crate) fn for_phase(ctx: &DlProofContext<'_>) -> Result<Self, EntailError> {
        let kb = &ctx.kb;
        assert_eq!(
            kb.meta.len(),
            kb.meta_sources.len(),
            "every original meta has its source inclusion"
        );
        let mut storage = Resident;
        let mut memory = Memory::new(&mut storage);
        let metas = memory.collect(kb.meta.iter().copied().zip(kb.meta_sources.iter().copied()))?;
        let live = memory.admitted_bytes();
        Ok(Self {
            input: ctx.input(),
            phase: Phase {
                contract: ctx.contract(),
                equalities: kb.same_as.len(),
                types: kb.abox_types.len(),
                roles: kb.abox_roles.len(),
                concepts: kb.table.len(),
                tbox: kb.tbox.len(),
                metas,
            },
            members: OrderedTable::default(),
            storage,
            live,
        })
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
        if ctx.input() != self.input
            || ctx.contract() != self.phase.contract
            || kb.same_as.len() != self.phase.equalities
            || kb.abox_types.len() != self.phase.types
            || kb.abox_roles.len() != self.phase.roles
            || kb.table.len() != self.phase.concepts
            || kb.tbox.len() != self.phase.tbox
            || kb
                .keys
                .get(key)
                .is_none_or(|(expected, _)| *expected != class)
        {
            return Err(EntailError::Build(
                "key membership checking requires its original pre-key source and class question"
                    .to_owned(),
            ));
        }
        let member = kb.with_negative_instance(individual, class, |assumptions| {
            let (decision, recorder) =
                hyper::decide_recording(kb, assumptions, super::graph::Budget::for_kb(kb));
            if hyper::decision_result(decision)? {
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

    #[cfg(test)]
    pub(crate) fn member(&self, key: usize, individual: u32) -> Option<&Membership> {
        self.members.get(&(key, individual))
    }
}
