// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The named class hierarchy and individual types from one native classification pass.
//!
//! One session owns both answers, their proof and their certificate. A stopped or
//! incomplete type pass therefore also withholds exact direct class edges, even if
//! classification finished. Established positive answers remain available in both views.
//! Work and step caps remain per native decision, not whole-service budgets.

use purrdf_core::TermValue;

use super::certificate::{Certified, DirectnessRefusal};
use super::classify::{Subsumptions, subsumes};
use super::proof::{ClaimSubject, Question, refutation_claim};
use super::{ClassHierarchy, Realization, Reasoner};
use crate::EntailError;

/// Named class subsumptions and individual types sharing one classification matrix.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct ClassAnalysis {
    hierarchy: ClassHierarchy,
    realization: Realization,
}

purrdf_hash::debug_non_exhaustive!(ClassAnalysis {
    hierarchy,
    realization
});

impl ClassAnalysis {
    /// Established class relations and empty classes; exact direct edges require certification.
    #[must_use]
    pub const fn hierarchy(&self) -> &ClassHierarchy {
        &self.hierarchy
    }

    /// Established individual types; exact most-specific types require certification.
    #[must_use]
    pub const fn realization(&self) -> &Realization {
        &self.realization
    }
}

impl Certified<ClassAnalysis> {
    /// Exact direct class edges, only when BOTH classification and realization completed.
    /// A later type-pass stop withholds this view as well as direct types.
    ///
    /// # Errors
    /// Returns the combined certificate's original completeness refusal.
    pub fn direct_subsumptions(&self) -> Result<&[(TermValue, TermValue)], DirectnessRefusal<'_>> {
        Ok(&self.direct_answer()?.hierarchy.direct)
    }

    /// Exact most-specific types, only when BOTH passes completed.
    ///
    /// # Errors
    /// Returns the combined certificate's original completeness refusal.
    pub fn direct_types(&self) -> Result<&[(TermValue, TermValue)], DirectnessRefusal<'_>> {
        Ok(&self.direct_answer()?.realization.direct)
    }
}

impl Reasoner {
    /// The combined question over this reasoner's source-owned class and individual lists.
    #[must_use]
    pub fn class_analysis_question(&self) -> Question {
        Question::ClassAnalysis {
            individuals: self.named_individuals(),
            classes: self.signature(),
        }
    }

    /// Classify named classes and realize named individuals with one session and matrix.
    /// The combined certificate gates both exact direct views. Positive answers survive
    /// incomplete decisions; existing standalone services retain their own certificates.
    ///
    /// # Errors
    /// Refuses inconsistent input and propagates native storage/preparation failures.
    pub fn classify_and_realize(&self) -> Result<Certified<ClassAnalysis>, EntailError> {
        let (mut session, usable) = self.open()?;
        let poll = || !purrdf_core::governor::is_stopped(self.kb.stop.as_deref());
        let mut answer = ClassAnalysis::default();
        let mut claims = Vec::new();
        if usable {
            let matrix = Subsumptions::decide_until(&mut session, &self.classes, poll)?;
            answer.hierarchy = ClassHierarchy::derive_until(&self.kb, &self.classes, &matrix, poll);
            let (realization, type_claims) = Realization::decide_until(
                &mut session,
                &self.individuals,
                &self.classes,
                &matrix,
                poll,
            );
            answer.realization = realization;
            if session.records() {
                claims = matrix.claims_with_native_decisions(&self.kb, &self.classes, true);
                claims.extend(type_claims);
            }
        }
        if !poll() {
            // Even with no individuals or residual decisions, a stop in saturation or
            // reduction belongs to the native session. This actual stopped reflexive
            // decision has an Unknown binding and adds no positive answer.
            let verdict = subsumes(&mut session, self.kb.top, self.kb.top);
            if session.records() {
                let top = TermValue::iri(purrdf_iri::vocab::owl::THING);
                claims.push(refutation_claim(
                    ClaimSubject::Subsumption {
                        sub: top.clone(),
                        sup: top,
                    },
                    verdict,
                    &[session.last_run()],
                ));
            }
        }
        session.check_storage()?;
        Ok(self.seal(session, answer, || (self.class_analysis_question(), claims)))
    }
}
