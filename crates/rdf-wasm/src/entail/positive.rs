// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained positive-world handles over the shared reasoning boundary.

use std::sync::Arc;

use purrdf_validate::regime::{
    PositiveReasonerSession, ReasoningError, RequestError, positive_limits_from_json,
};
use wasm_bindgen::prelude::*;

use super::{ReasoningAnswer, reasoning_error, run_reasoning};
use crate::query::CancellationToken;

/// A native positive assertion lifetime, consumed only by mutating failures.
#[wasm_bindgen]
#[derive(Debug)]
pub struct PositiveReasoner {
    session: Option<PositiveReasonerSession>,
    /// A shared bit, not a borrowed JavaScript wrapper.
    cancel: Option<CancellationToken>,
    canonicalization_json: String,
}

impl PositiveReasoner {
    pub(super) fn open(
        document: &str,
        limits: &str,
        step_cap: u32,
        work_cap: u32,
        policy: &str,
        cancel: Option<CancellationToken>,
    ) -> Result<Self, JsValue> {
        let signal = cancel
            .as_ref()
            .map(|cancel| Arc::new(cancel.share()) as Arc<dyn purrdf_core::StopSignal>);
        let (session, canonicalization_json) = run_reasoning(policy, cancel.as_ref(), |control| {
            PositiveReasonerSession::open_with_proofs(
                document,
                positive_limits_from_json(limits)?,
                step_cap,
                work_cap,
                signal,
                control,
            )
        })?;
        Ok(Self {
            session: Some(session),
            cancel,
            canonicalization_json,
        })
    }

    fn session(&self) -> Result<&PositiveReasonerSession, JsValue> {
        self.session.as_ref().ok_or_else(consumed)
    }

    fn take(&mut self) -> Result<PositiveReasonerSession, JsValue> {
        self.session.take().ok_or_else(consumed)
    }
}

fn consumed() -> JsValue {
    reasoning_error(ReasoningError::Request(
        RequestError::ConsumedPositiveSession,
    ))
}

#[wasm_bindgen]
impl PositiveReasoner {
    /// Ordered canonical receipts from opening the retained world.
    #[wasm_bindgen(getter, js_name = canonicalizationJson)]
    #[must_use]
    pub fn canonicalization_json(&self) -> String {
        self.canonicalization_json.clone()
    }

    /// Advance through the shared source parser. Policy or operation failure consumes
    /// this handle; its native cause and receipts remain independently retained.
    pub fn advance(
        &mut self,
        document: &str,
        canonical_search: &str,
    ) -> Result<PositiveUpdateWork, JsValue> {
        // Take before run_reasoning: policy parsing itself may fail before its closure.
        let session = self.take()?;
        let ((session, work), canonicalization_json) =
            run_reasoning(canonical_search, self.cancel.as_ref(), |control| {
                session.advance(document, control)
            })?;
        self.session = Some(session);
        Ok(PositiveUpdateWork {
            work,
            canonicalization_json,
        })
    }

    /// Borrow the current native world for its recorded consistency answer.
    pub fn consistency(&self, canonical_search: &str) -> Result<ReasoningAnswer, JsValue> {
        let session = self.session()?;
        let (answer, receipts) = run_reasoning(canonical_search, self.cancel.as_ref(), |_| {
            session.consistency()
        })?;
        let mut answer = ReasoningAnswer::from(answer);
        answer.canonicalization_json = receipts;
        Ok(answer)
    }

    /// Prepare and answer one class question; failure consumes the mutating owner.
    #[wasm_bindgen(js_name = classSatisfiability)]
    pub fn class_satisfiability(
        &mut self,
        class: &str,
        canonical_search: &str,
    ) -> Result<ReasoningAnswer, JsValue> {
        let session = self.take()?;
        let ((session, answer), receipts) =
            run_reasoning(canonical_search, self.cancel.as_ref(), |_| {
                session.class_satisfiability(class)
            })?;
        self.session = Some(session);
        let mut answer = ReasoningAnswer::from(answer);
        answer.canonicalization_json = receipts;
        Ok(answer)
    }

    /// Prepare and answer one axiom question; failure consumes the mutating owner.
    pub fn entails(
        &mut self,
        axiom: &str,
        canonical_search: &str,
    ) -> Result<ReasoningAnswer, JsValue> {
        let session = self.take()?;
        let ((session, answer), receipts) =
            run_reasoning(canonical_search, self.cancel.as_ref(), |_| {
                session.entails(axiom)
            })?;
        self.session = Some(session);
        let mut answer = ReasoningAnswer::from(answer);
        answer.canonicalization_json = receipts;
        Ok(answer)
    }

    /// Prepare and answer one instance question; failure consumes the mutating owner.
    pub fn instances(
        &mut self,
        class: &str,
        canonical_search: &str,
    ) -> Result<ReasoningAnswer, JsValue> {
        let session = self.take()?;
        let ((session, answer), receipts) =
            run_reasoning(canonical_search, self.cancel.as_ref(), |_| {
                session.instances(class)
            })?;
        self.session = Some(session);
        let mut answer = ReasoningAnswer::from(answer);
        answer.canonicalization_json = receipts;
        Ok(answer)
    }

    /// Check an explicit positive envelope with an independent context over this
    /// consumer's complete source. A failed read-only check does not consume it.
    #[wasm_bindgen(js_name = checkPositiveProof)]
    pub fn check_positive_proof(
        &self,
        service: &str,
        argument: &str,
        answer: &str,
        certificate: &str,
        proof: &str,
        canonical_search: &str,
    ) -> Result<crate::canonical::CanonicalResult, JsValue> {
        let session = self.session()?;
        let (report, receipts) =
            run_reasoning(canonical_search, self.cancel.as_ref(), |control| {
                session.check_positive_proof(service, argument, answer, certificate, proof, control)
            })?;
        Ok(crate::canonical::CanonicalResult::new(
            JsValue::from_str(&report),
            receipts,
        ))
    }

    /// The last successful native source-work counters, transported at full width.
    pub fn work(&self) -> Result<PositiveUpdateWork, JsValue> {
        Ok(PositiveUpdateWork {
            work: self.session()?.work(),
            canonicalization_json: "[]".to_owned(),
        })
    }
}

/// Exact native source-work counters and ordered canonical receipts.
#[wasm_bindgen]
#[derive(Debug)]
pub struct PositiveUpdateWork {
    work: purrdf_validate::regime::PositiveUpdateWork,
    canonicalization_json: String,
}

#[wasm_bindgen]
impl PositiveUpdateWork {
    /// Borrowed source identity lookups.
    #[wasm_bindgen(getter, js_name = borrowedTermLookups)]
    #[must_use]
    pub fn borrowed_term_lookups(&self) -> u64 {
        self.work.borrowed_term_lookups
    }
    /// Exact probes of retained rows and declarations.
    #[wasm_bindgen(getter, js_name = retainedRowProbes)]
    #[must_use]
    pub fn retained_row_probes(&self) -> u64 {
        self.work.retained_row_probes
    }
    /// Original source rows inspected.
    #[wasm_bindgen(getter, js_name = scannedRows)]
    #[must_use]
    pub fn scanned_rows(&self) -> u64 {
        self.work.scanned_rows
    }
    /// Actual owned source term reads.
    #[wasm_bindgen(getter, js_name = copiedTerms)]
    #[must_use]
    pub fn copied_terms(&self) -> u64 {
        self.work.copied_terms
    }
    /// Newly retained source statements.
    #[wasm_bindgen(getter, js_name = newRows)]
    #[must_use]
    pub fn new_rows(&self) -> u64 {
        self.work.new_rows
    }
    /// Newly retained graph declarations.
    #[wasm_bindgen(getter, js_name = newGraphs)]
    #[must_use]
    pub fn new_graphs(&self) -> u64 {
        self.work.new_graphs
    }
    /// New rows passed to the original default-graph decoder.
    #[wasm_bindgen(getter, js_name = interpretedDefaultRows)]
    #[must_use]
    pub fn interpreted_default_rows(&self) -> u64 {
        self.work.interpreted_default_rows
    }
    /// Whether full proof/source identities were recomputed.
    #[wasm_bindgen(getter, js_name = identityRecomputed)]
    #[must_use]
    pub fn identity_recomputed(&self) -> bool {
        self.work.identity_recomputed
    }
    /// Ordered receipts of canonicalization performed by the source operation.
    #[wasm_bindgen(getter, js_name = canonicalizationJson)]
    #[must_use]
    pub fn canonicalization_json(&self) -> String {
        self.canonicalization_json.clone()
    }
}
