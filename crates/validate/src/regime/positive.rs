// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit positive-world ownership and proof transport at the shared string boundary.

use core::fmt::Write as _;
use std::str::FromStr;
use std::sync::Arc;

use purrdf_core::{CanonicalizationControl, RdfDataset, StopSignal};
pub use purrdf_entail::reasoner::{PositiveUpdateWork, PositiveWorldLimits};
use purrdf_entail::reasoner::{PositiveWorld, SchemaPreparationBudget};
use purrdf_entail::{Question, Service, ServiceProof};
use purrdf_lex::json::record::{DecodeError, Record};
use purrdf_lex::json::{self, Value};

use super::{ReasoningAnswer, ReasoningError, RequestError};

/// Original integer refusal with the exact host spelling and checked target type.
#[derive(Debug)]
struct PositiveIntegerError {
    offered: String,
    target: &'static str,
    target_bytes: usize,
    cause: core::num::ParseIntError,
}

impl core::fmt::Display for PositiveIntegerError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            out,
            "positive limit {:?} does not fit {}: {}",
            self.offered, self.target, self.cause
        )
    }
}

impl std::error::Error for PositiveIntegerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

impl purrdf_hash::error::StructuredError for PositiveIntegerError {
    purrdf_hash::error_visit! {
        @record self { offered, target, target_bytes, cause } => "positive-limit-integer" {
            "offered" => Text(&self.offered), "target" => Code(self.target),
            "target-bytes" => Unsigned(self.target_bytes as u128), "cause" => Cause(&self.cause),
        }
    }
}

fn decimal<T: FromStr<Err = core::num::ParseIntError>>(value: &Value) -> Result<T, DecodeError> {
    let text = value
        .as_str()
        .ok_or_else(|| DecodeError::invalid_type(value, "canonical decimal integer string"))?;
    if text.is_empty()
        || (text.len() > 1 && text.starts_with('0'))
        || !text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(DecodeError::invalid_value(
            value,
            "canonical decimal integer string",
        ));
    }
    text.parse().map_err(|cause| {
        DecodeError::native(PositiveIntegerError {
            offered: text.to_owned(),
            target: core::any::type_name::<T>(),
            target_bytes: size_of::<T>(),
            cause,
        })
    })
}

/// Read all six explicit positive ceilings without a lossy host-number conversion.
/// Members are canonical decimal strings; unknown, repeated or missing fields refuse.
///
/// # Errors
/// Preserves the original JSON, field, integer or target-capacity failure.
pub fn positive_limits_from_json(text: &str) -> Result<PositiveWorldLimits, ReasoningError> {
    let read = || {
        let value = json::read_slice(text.as_bytes(), json::Limits::DEFAULT)?;
        let mut record = Record::new(&value, "positive reasoning limits")?;
        let limits = PositiveWorldLimits {
            history_steps: record.required_with("historySteps", decimal)?,
            source_terms: record.required_with("sourceTerms", decimal)?,
            source_rows: record.required_with("sourceRows", decimal)?,
            question_bytes: record.required_with("questionBytes", decimal)?,
            metadata_bytes: record.required_with("metadataBytes", decimal)?,
            metadata_work: record.required_with("metadataWork", decimal)?,
        };
        record.deny_unknown()?;
        Ok::<_, DecodeError>(limits)
    };
    read().map_err(|cause| ReasoningError::PositiveLimits {
        offered: text.to_owned(),
        cause: Box::new(cause),
    })
}

/// Explicitly decode a positive proof envelope under the consumer's own ceilings.
/// Legacy [`super::decode_dl_proof`] continues refusing this envelope.
///
/// # Errors
/// Original outer-document or positive native proof refusal.
pub fn decode_positive_dl_proof(
    document: &str,
    limits: PositiveWorldLimits,
) -> Result<ServiceProof, ReasoningError> {
    super::decode_proof_document(document, |bytes| {
        ServiceProof::decode_positive(bytes, limits)
    })
}

/// A proof-recording positive world over parsed caller snapshots.
///
/// Mutators consume this owner, including input and preparation failures. Read-only
/// consistency and independent proof checking borrow it. No source, parser or
/// completion graph is copied to satisfy this boundary.
pub struct PositiveReasonerSession {
    world: PositiveWorld<Arc<RdfDataset>>,
    limits: PositiveWorldLimits,
    stop: Option<Arc<dyn StopSignal>>,
}

impl core::fmt::Debug for PositiveReasonerSession {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.debug_struct("PositiveReasonerSession")
            .field("world", &self.world)
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl PositiveReasonerSession {
    /// Parse and record a complete source under explicit retention and native controls.
    /// Zero decision caps preserve the native baseline, as on [`super::ReasonerSession`].
    ///
    /// # Errors
    /// Original source, preparation, canonicalization, cancellation or retention refusal.
    pub fn open_with_proofs(
        document: &str,
        limits: PositiveWorldLimits,
        step_cap: u32,
        work_cap: u32,
        stop: Option<Arc<dyn StopSignal>>,
        control: &mut CanonicalizationControl<'_>,
    ) -> Result<Self, ReasoningError> {
        let source = super::parse_premise(document)?;
        let mut world = match &stop {
            Some(stop) => PositiveWorld::with_proofs_and_stop(
                source,
                limits,
                SchemaPreparationBudget::default(),
                Arc::clone(stop),
                control,
            ),
            None => PositiveWorld::with_proofs(source, limits, control),
        }?;
        if step_cap != 0 {
            world = world.with_step_cap(u64::from(step_cap));
        }
        if work_cap != 0 {
            world = world.with_work_cap(u64::from(work_cap));
        }
        Ok(Self {
            world,
            limits,
            stop,
        })
    }

    /// Consume this owner and advance to the supplied complete append-only snapshot.
    ///
    /// # Errors
    /// Original parsing or native update refusal; no replacement session is returned.
    pub fn advance(
        mut self,
        document: &str,
        control: &mut CanonicalizationControl<'_>,
    ) -> Result<(Self, PositiveUpdateWork), ReasoningError> {
        self.world = self
            .world
            .advance(super::parse_premise(document)?, control)?;
        let work = self.world.work();
        Ok((self, work))
    }

    /// Actual work reported by the latest successful native source operation.
    #[must_use]
    pub fn work(&self) -> PositiveUpdateWork {
        self.world.work()
    }

    /// Borrow this world for its native consistency answer and positive proof.
    /// A stopped search retains the native unknown answer and budget-exhausted certificate.
    ///
    /// # Errors
    /// Original proof-retention refusal; this read does not consume it.
    pub fn consistency(&self) -> Result<ReasoningAnswer<String>, ReasoningError> {
        let (verdict, certificate, proof) = self
            .world
            .consistency()
            .map_err(|cause| {
                ReasoningError::entail(super::EntailContext::Service("consistency".into()), cause)
            })?
            .into_certified_parts();
        Ok(ReasoningAnswer {
            answer: format!("consistency {}\n", super::verdict_name(verdict)),
            certificate: super::render_dl_certificate("consistency", &certificate),
            proof: proof.as_ref().map(super::render_dl_proof),
        })
    }

    /// Consume this owner while preparing a class-satisfiability question.
    ///
    /// # Errors
    /// Original term, preparation, decision or proof-retention refusal.
    pub fn class_satisfiability(
        mut self,
        class: &str,
    ) -> Result<(Self, ReasoningAnswer<String>), ReasoningError> {
        let class = super::parse_one_term(class)?;
        let (world, answer) = self.world.class_satisfiability(&class).map_err(|cause| {
            ReasoningError::entail(
                super::EntailContext::Service("class-satisfiability".into()),
                cause,
            )
        })?;
        self.world = world;
        let (verdict, certificate, proof) = answer.into_certified_parts();
        Ok((
            self,
            ReasoningAnswer {
                answer: format!(
                    "class-satisfiability {}\nterm {}\n",
                    super::verdict_name(verdict),
                    super::emit(&class)
                ),
                certificate: super::render_dl_certificate("class-satisfiability", &certificate),
                proof: proof.as_ref().map(super::render_dl_proof),
            },
        ))
    }

    /// Consume this owner while preparing and answering an axiom question.
    ///
    /// # Errors
    /// Original axiom, preparation, decision or proof-retention refusal.
    pub fn entails(
        mut self,
        axiom: &str,
    ) -> Result<(Self, ReasoningAnswer<String>), ReasoningError> {
        let axiom = super::parse_axiom(axiom)?;
        let (world, answer) = self.world.entails(&axiom).map_err(|cause| {
            ReasoningError::entail(super::EntailContext::Service("entails".into()), cause)
        })?;
        self.world = world;
        let (verdict, certificate, proof) = answer.into_certified_parts();
        let mut answer = format!("entails {}\n", super::verdict_name(verdict));
        super::write_axiom(&axiom, &mut answer);
        Ok((
            self,
            ReasoningAnswer {
                answer,
                certificate: super::render_dl_certificate("entails", &certificate),
                proof: proof.as_ref().map(super::render_dl_proof),
            },
        ))
    }

    /// Consume this owner while preparing and answering an instance question.
    ///
    /// # Errors
    /// Original term, preparation, decision or proof-retention refusal.
    pub fn instances(
        mut self,
        class: &str,
    ) -> Result<(Self, ReasoningAnswer<String>), ReasoningError> {
        let class = super::parse_one_term(class)?;
        let (world, answer) = self.world.instances(&class).map_err(|cause| {
            ReasoningError::entail(super::EntailContext::Service("instances".into()), cause)
        })?;
        self.world = world;
        let (individuals, certificate, proof) = answer.into_certified_parts();
        let mut answer = String::new();
        for individual in &individuals {
            let _ = writeln!(answer, "instance {}", super::emit(individual));
        }
        Ok((
            self,
            ReasoningAnswer {
                answer,
                certificate: super::render_dl_certificate("instances", &certificate),
                proof: proof.as_ref().map(super::render_dl_proof),
            },
        ))
    }

    /// Independently check a positive envelope against this consumer's current source.
    ///
    /// # Errors
    /// Original source, proof, question, answer, certificate or coverage refusal.
    pub fn check_positive_proof(
        &self,
        service: &str,
        argument: &str,
        answer: &str,
        certificate: &str,
        proof: &str,
        control: &mut CanonicalizationControl<'_>,
    ) -> Result<String, ReasoningError> {
        let asked = super::parse_proof_service(service)?;
        let question = match asked {
            Service::Consistency => {
                super::refuse_argument(service, argument)?;
                Question::Consistency
            }
            Service::ClassSatisfiability => Question::ClassSatisfiability {
                class: super::parse_one_term(argument)?,
            },
            Service::AxiomEntailment => Question::AxiomEntailment {
                axiom: Box::new(super::parse_axiom(argument)?),
            },
            Service::InstanceRetrieval => Question::InstanceRetrieval {
                class: super::parse_one_term(argument)?,
            },
            service => {
                return Err(RequestError::UnsupportedService {
                    service,
                    operation: "positive proof checking",
                }
                .into());
            }
        };
        let term = decode_positive_dl_proof(proof, self.limits)?;
        let context = term.prepare_positive_context(
            self.world.source(),
            &question,
            self.limits,
            SchemaPreparationBudget::default(),
            self.stop.clone(),
            control,
        )?;
        super::check_prepared_proof(
            &term,
            asked,
            argument,
            answer,
            certificate,
            &question,
            &context,
            control,
        )
    }
}
