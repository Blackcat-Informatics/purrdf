// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Question-bound causal capture in the original native service verification loop.

use super::{DlCertificate, DlProofContext, DlProofError, Question, ServiceProof, ServiceReplay};
use crate::owl_dl::proof::source::causal::{CausalRefutation, CausalSupportKind, support_error};
use crate::owl_dl::{
    Kb,
    proof::{RefutationReplay, capture::Capture},
};
use purrdf_lex::allocation::{Memory, Resident};

#[derive(Debug)]
struct Checked {
    run: usize,
    replay: RefutationReplay,
    capture: Capture,
}

#[derive(Debug, Default)]
pub(super) struct Captures {
    runs: Vec<Checked>,
    storage: Resident,
    live: usize,
}

pub(super) fn unavailable(
    run: usize,
    clash: Option<usize>,
    kind: CausalSupportKind,
    detail: String,
) -> DlProofError {
    DlProofError::CausalSupport {
        run: Some(run),
        clash,
        kind,
        detail,
    }
}

/// Attach the checked service locus without reducing the native refusal payload.
pub(super) fn run_error(run: usize, error: DlProofError) -> DlProofError {
    match error {
        DlProofError::CausalSupport {
            clash,
            kind,
            detail,
            ..
        } => unavailable(run, clash, kind, detail),
        original => original,
    }
}

impl Captures {
    pub(super) fn push(
        &mut self,
        run: usize,
        replay: RefutationReplay,
        capture: Capture,
    ) -> Result<(), DlProofError> {
        if !replay.is_closed()
            || capture.clashes.len() != replay.clashes()
            || capture.clashes.is_empty()
        {
            return Err(unavailable(
                run,
                None,
                CausalSupportKind::Unclosed,
                format!("{replay:?}; captured clashes={}", capture.clashes.len()),
            ));
        }
        for clash in &capture.clashes {
            if let Err(error) = &clash.support {
                return Err(support_error(run, clash.index, error));
            }
        }
        let mut memory = Memory::resume(&mut self.storage, self.live);
        let result = memory.push(
            &mut self.runs,
            Checked {
                run,
                replay,
                capture,
            },
        );
        self.live = memory.admitted_bytes();
        result.map_err(DlProofError::Storage)
    }
}

/// Verified service and complete causal graphs for its closed refutation runs.
///
/// The caller's source, question, receipts, calculus and every selected run are
/// checked before this value can exist. Model runs have their ordinary verification
/// report and no contradiction graph. Missing or undecided runs refuse explicitly.
pub struct CausalServiceReplay<'a> {
    report: ServiceReplay,
    proof: &'a ServiceProof,
    kb: &'a Kb,
    captures: Captures,
}

impl std::fmt::Debug for CausalServiceReplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalServiceReplay")
            .field("report", &self.report)
            .field("question", &self.proof.question)
            .field("captures", &self.captures)
            .finish()
    }
}

impl ServiceProof {
    /// Verify and select causal support in the same independent native replay.
    ///
    /// Prepare the caller's question before borrowing `ctx`, as for `verify_prepared`.
    /// No solver search, source reconstruction or second verification pass is performed.
    ///
    /// # Errors
    /// Original native source/question/calculus, receipt, stopping and admission errors;
    /// `DlProofError::CausalSupport` for missing, unsupported or unclosed causal evidence.
    pub fn verify_prepared_causal<'a>(
        &'a self,
        question: &Question,
        certificate: Option<&DlCertificate>,
        ctx: &'a DlProofContext<'_>,
    ) -> Result<CausalServiceReplay<'a>, DlProofError> {
        let mut captures = Captures::default();
        let report = self.verify_prepared_inner(question, certificate, ctx, Some(&mut captures))?;
        Ok(CausalServiceReplay {
            report,
            proof: self,
            kb: &ctx.kb,
            captures,
        })
    }
}

impl CausalServiceReplay<'_> {
    /// The unchanged native verification counts and explicit trust boundary.
    #[must_use]
    pub const fn report(&self) -> &ServiceReplay {
        &self.report
    }

    /// Exact question checked against the caller's independently prepared source.
    #[must_use]
    pub fn question(&self) -> &Question {
        &self.proof.question
    }

    /// Closed refutation runs, in original service order; models are not contradiction graphs.
    pub fn refutations(&self) -> impl ExactSizeIterator<Item = (usize, CausalRefutation<'_>)> + '_ {
        self.captures.runs.iter().map(|checked| {
            let proof = self.proof.runs[checked.run]
                .proof
                .as_ref()
                .expect("causal verification requires the native proof");
            (
                checked.run,
                CausalRefutation::checked(proof, self.kb, &checked.replay, &checked.capture),
            )
        })
    }
}
