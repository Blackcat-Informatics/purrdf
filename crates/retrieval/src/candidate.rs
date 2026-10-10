// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit candidate-prefix planning beside the fused-answer plan.

use crate::admission::{AdmissionEnvironment, AdmissionError, ProbedDepth, Unprobeable};
use crate::canonical::{Reader, Writer};
use crate::compile::{
    CompiledRead, StratumUnit, UnitAttribution, emit_units, tagged_units_as_assembled,
};
use crate::error::PlanError;
use crate::execute::{CandidateExecutionResult, ReadStratum};
use crate::fusion_stream::ProducerStatus;
use crate::id::PlanId;
use crate::iri::Iri;
use crate::plan::{PlanOrigin, PlanParts, ProducerBinding, ProducerDecision, UnservedTerm};
use crate::request::RequestTerm;
use crate::statistics::Statistics;
use purrdf_core::{FastHasher, FastMap};
use purrdf_sparql_eval::{PropertyFunctionRegistry, RegistryId};
use std::collections::BTreeMap;

/// A refusal of caller-supplied per-stratum work depths or candidate planning.
#[derive(Debug)]
pub enum CandidatePlanError {
    /// The original matching, placement or canonical reader refused.
    Plan(PlanError),
    /// A requested prefix leaves no expressible probe row.
    DepthWithoutProbe {
        /// The affected caller-supplied stratum.
        stratum: Box<Iri>,
        /// The caller's actual requested depth.
        requested: u32,
        /// The largest native readable depth.
        ceiling: u32,
    },
    /// Two entries tried to choose the work depth of one stratum.
    DuplicateDepth {
        /// The repeated stratum.
        stratum: Box<Iri>,
    },
    /// A selected producer has no caller work depth.
    MissingDepth {
        /// The selected stratum.
        stratum: Box<Iri>,
    },
    /// A depth names no selected producer.
    UnexpectedDepth {
        /// The unused caller-supplied stratum.
        stratum: Box<Iri>,
    },
    /// A decoded candidate plan contradicts its own production records.
    InvalidEncoding {
        /// The precise contradiction.
        reason: String,
    },
}

impl std::fmt::Display for CandidatePlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plan(error) => error.fmt(f),
            Self::DepthWithoutProbe {
                stratum,
                requested,
                ceiling,
            } => write!(
                f,
                "candidate stratum {stratum} requests depth {requested}, above readable depth {ceiling}"
            ),
            Self::DuplicateDepth { stratum } => {
                write!(f, "candidate stratum {stratum} has two work depths")
            }
            Self::MissingDepth { stratum } => write!(
                f,
                "selected candidate stratum {stratum} has no caller work depth"
            ),
            Self::UnexpectedDepth { stratum } => {
                write!(f, "candidate work depth names unselected stratum {stratum}")
            }
            Self::InvalidEncoding { reason } => write!(f, "invalid candidate plan: {reason}"),
        }
    }
}

impl std::error::Error for CandidatePlanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Plan(error) => Some(error),
            _ => None,
        }
    }
}
purrdf_lex::variant_from!(CandidatePlanError { Plan(PlanError) });

/// Independently chosen candidate work depths, keyed by the caller's strata.
///
/// Zero is an explicit empty prefix. Every positive value is the number of
/// original producer ranks requested, independent of a fused k or estimated
/// cardinality/selectivity. The native read additionally has its one probe
/// slot, and reports that read in rows_materialised. Unknown strata and
/// missing selected strata are refused when the request is planned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateDepths(pub(crate) FastMap<Iri, u32>);

impl CandidateDepths {
    /// Validate explicit work depths without clamping or deriving any value.
    ///
    /// # Errors
    /// Refuses duplicate strata and a depth whose probe row is inexpressible.
    pub fn new(depths: impl IntoIterator<Item = (Iri, u32)>) -> Result<Self, CandidatePlanError> {
        let mut values = FastMap::with_hasher(FastHasher::default());
        for (stratum, depth) in depths {
            if ProbedDepth::checked_work(depth) == Err(Unprobeable::PastCeiling) {
                return Err(CandidatePlanError::DepthWithoutProbe {
                    stratum: Box::new(stratum),
                    requested: depth,
                    ceiling: crate::admission::MAX_READ_DEPTH,
                });
            }
            if values.insert(stratum.clone(), depth).is_some() {
                return Err(CandidatePlanError::DuplicateDepth {
                    stratum: Box::new(stratum),
                });
            }
        }
        Ok(Self(values))
    }

    /// The caller's work depth for this stratum.
    #[must_use]
    pub fn get(&self, stratum: &Iri) -> Option<u32> {
        self.0.get(stratum).copied()
    }

    /// The number of explicitly named strata.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no stratum was named.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Canonical ascending strata, independent of the fixed hash table's layout.
    #[must_use]
    pub fn sorted(&self) -> Vec<(&Iri, u32)> {
        let mut values: Vec<_> = self
            .0
            .iter()
            .map(|(stratum, depth)| (stratum, *depth))
            .collect();
        values.sort_unstable_by(|left, right| left.0.cmp(right.0));
        values
    }
}

/// A candidate plan whose depths were supplied, rather than statistically derived.
///
/// It uses the original matching and placement kernel, but is a separate type
/// and canonical encoding from Plan. No DepthInputs::licensed_prefix or fusion
/// profile is fabricated. The declared row bounds remain separate evidence;
/// they never silently lower the requested work depths.
#[derive(Clone, Debug)]
pub struct CandidatePlan {
    pub(crate) parts: PlanParts,
}

impl CandidatePlan {
    /// The version of the separate candidate-plan encoding.
    pub const VERSION: u16 = 1;

    /// Caller-supplied depths, in the actual plan.
    #[must_use]
    pub fn depths(&self) -> CandidateDepths {
        CandidateDepths(self.parts.stratum_depths.clone())
    }

    /// The original request terms in caller order.
    #[must_use]
    pub fn request_terms(&self) -> &[RequestTerm] {
        &self.parts.request_terms
    }

    /// The selected producers and the terms they actually receive.
    #[must_use]
    pub fn producer_bindings(&self) -> &[ProducerBinding] {
        &self.parts.producer_bindings
    }

    /// All matching/placement decisions, including rejected producers.
    #[must_use]
    pub fn producer_decisions(&self) -> &[ProducerDecision] {
        &self.parts.producer_decisions
    }

    /// The actual invocation-mode row declarations; no requested depth is substituted.
    #[must_use]
    pub fn declared_rows(&self) -> &BTreeMap<Iri, u64> {
        &self.parts.declared_rows
    }

    /// Per-term loss derived through the original evidence body.
    #[must_use]
    pub fn unserved_evidence(&self) -> Vec<UnservedTerm> {
        crate::plan::unserved_evidence(
            self.parts.request_terms.len(),
            &self.parts.producer_bindings,
            &self.parts.unserved_terms,
        )
    }

    /// The separate, canonical candidate question identity.
    #[must_use]
    pub fn id(&self) -> PlanId {
        PlanId::from_canonical(&self.canonical_bytes())
    }

    /// Versioned canonical bytes, containing every caller depth and declaration.
    ///
    /// Integer widths, map order and length framing are target-independent.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut writer = Writer::new();
        writer.u16(Self::VERSION);
        writer.string("candidate-union");
        crate::plan::write_request_terms(&mut writer, &self.parts.request_terms);
        crate::plan::write_bindings(&mut writer, &self.parts.producer_bindings);
        crate::plan::write_decisions(&mut writer, &self.parts.producer_decisions);
        crate::plan::write_depths(&mut writer, &self.parts.stratum_depths);
        writer.u64(self.parts.declared_rows.len() as u64);
        for (stratum, rows) in &self.parts.declared_rows {
            writer.string(stratum.as_str());
            writer.u64(*rows);
        }
        crate::plan::write_statistics(&mut writer, &self.parts.statistics_snapshot);
        writer.u64(self.parts.registry_instance_id.as_u64());
        writer.string(&self.parts.registry_content_fingerprint);
        crate::plan::write_unserved_terms(&mut writer, &self.parts.unserved_terms);
        writer.into_bytes()
    }

    /// Decode and certify the separate candidate-plan format.
    ///
    /// # Errors
    /// Refuses malformed framing, unknown version/kind, unordered/duplicate
    /// declaration keys, missing producer depths, and dishonest depth evidence.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, CandidatePlanError> {
        let mut reader = Reader::new(bytes);
        let version = reader.u16()?;
        if version != Self::VERSION {
            return Err(PlanError::VersionMismatch {
                found: version,
                expected: Self::VERSION,
            }
            .into());
        }
        if reader.string("candidate plan kind")? != "candidate-union" {
            return Err(CandidatePlanError::InvalidEncoding {
                reason: "the encoding names another read kind".to_owned(),
            });
        }
        let request_terms = crate::plan::read_request_terms(&mut reader)?;
        let producer_bindings = crate::plan::read_bindings(&mut reader)?;
        let producer_decisions = crate::plan::read_decisions(&mut reader)?;
        let depths = CandidateDepths::new(crate::plan::read_depths(&mut reader)?)?;
        let count = reader.u64()?;
        let mut declared_rows = BTreeMap::new();
        let mut previous: Option<Iri> = None;
        for _ in 0..count {
            let stratum = crate::plan::read_iri(&mut reader, "candidate row-bound stratum")?;
            if previous.as_ref().is_some_and(|key| key >= &stratum) {
                return Err(CandidatePlanError::InvalidEncoding {
                    reason: "declared row-bound strata are not strictly ascending".to_owned(),
                });
            }
            previous = Some(stratum.clone());
            declared_rows.insert(stratum, reader.u64()?);
        }
        let statistics_snapshot = crate::plan::read_statistics(&mut reader)?;
        let registry_instance_id = RegistryId::from_raw(reader.u64()?);
        let registry_content_fingerprint = reader.string("registry content fingerprint")?;
        let unserved_terms = crate::plan::read_unserved_terms(&mut reader)?;
        reader.finish()?;
        let plan = Self {
            parts: PlanParts {
                request_terms,
                producer_bindings,
                producer_decisions,
                unserved_terms,
                stratum_depths: depths.0,
                stratum_derivations: BTreeMap::new(),
                declared_rows,
                statistics_snapshot,
                registry_instance_id,
                registry_content_fingerprint,
                origin: PlanOrigin::Deserialized,
            },
        };
        plan.certify()?;
        Ok(plan)
    }

    /// Check caller-depth coverage, separately from the fusion derivation law.
    ///
    /// # Errors
    /// Refuses missing/extra depths or declarations, duplicate selected strata,
    /// and a forged statistics derivation for this independent-depth read.
    pub fn certify(&self) -> Result<(), CandidatePlanError> {
        CandidateDepths::new(
            self.parts
                .stratum_depths
                .iter()
                .map(|(key, depth)| (key.clone(), *depth)),
        )?;
        let mut selected = std::collections::BTreeSet::new();
        for binding in &self.parts.producer_bindings {
            if !selected.insert(&binding.stratum) {
                return Err(CandidatePlanError::InvalidEncoding {
                    reason: format!("stratum {} has two producer bindings", binding.stratum),
                });
            }
            if !self.parts.stratum_depths.contains_key(&binding.stratum) {
                return Err(CandidatePlanError::MissingDepth {
                    stratum: Box::new(binding.stratum.clone()),
                });
            }
            if !self.parts.declared_rows.contains_key(&binding.stratum) {
                return Err(CandidatePlanError::InvalidEncoding {
                    reason: format!(
                        "stratum {} has no invocation row declaration",
                        binding.stratum
                    ),
                });
            }
        }
        for stratum in self.parts.stratum_depths.keys() {
            if !selected.contains(stratum) {
                return Err(CandidatePlanError::UnexpectedDepth {
                    stratum: Box::new(stratum.clone()),
                });
            }
        }
        if self.parts.declared_rows.len() != selected.len() {
            return Err(CandidatePlanError::InvalidEncoding {
                reason: "row declarations name unselected strata".to_owned(),
            });
        }
        if !self.parts.stratum_derivations.is_empty()
            || !self.parts.statistics_snapshot.entries.is_empty()
        {
            return Err(CandidatePlanError::InvalidEncoding {
                reason: "independent work depths record no statistical depth derivation".to_owned(),
            });
        }
        Ok(())
    }
}

impl PartialEq for CandidatePlan {
    fn eq(&self, other: &Self) -> bool {
        self.canonical_bytes() == other.canonical_bytes()
    }
}
impl Eq for CandidatePlan {}

/// Plan candidate prefixes through the original matching and placement body.
///
/// # Errors
/// Preserves the original planner's malformed-term/registry/placement errors,
/// and refuses caller depths that do not name exactly the selected strata.
pub fn plan_candidates(
    terms: &[RequestTerm],
    depths: &CandidateDepths,
    registry: &PropertyFunctionRegistry,
    statistics: &impl Statistics,
) -> Result<CandidatePlan, CandidatePlanError> {
    let parts = crate::planner::plan_read(
        terms,
        crate::planner::DepthPolicy::Candidates(depths),
        registry,
        statistics,
    )?;
    let plan = CandidatePlan { parts };
    plan.certify()?;
    Ok(plan)
}

/// Compiled candidate reads, with their original depth and producer attribution.
///
/// This type has no TopK, fusion weight or fusion profile. execute and
/// execute_within run it through the same native executor as CompiledRetrieval.
#[derive(Debug)]
pub struct CompiledCandidates {
    /// The emitted per-stratum query units, ascending by stratum.
    units: Vec<StratumUnit>,
    /// The independent candidate-plan identity.
    pub plan_id: PlanId,
    /// The registry instance that admitted the actual units.
    pub registry_id: RegistryId,
    /// Its durable declared-content fingerprint.
    pub registry_fingerprint: String,
    pub(crate) depths: CandidateDepths,
    pub(crate) attribution: Vec<UnitAttribution>,
    pub(crate) unserved_terms: Vec<UnservedTerm>,
    pub(crate) producer_bindings: Vec<ProducerBinding>,
    pub(crate) producer_decisions: Vec<ProducerDecision>,
}

impl CompiledCandidates {
    /// The original compiled units, borrowed without a query/depth rewrite seam.
    #[must_use]
    pub fn units(&self) -> &[StratumUnit] {
        &self.units
    }
}

/// Compile explicit candidate depths at the same native admission waist.
///
/// # Errors
/// Preserves original semantic admission/placement errors and refuses forged
/// candidate depth/declaration records before rendering any query.
pub fn compile_candidates(
    plan: &CandidatePlan,
    env: &AdmissionEnvironment<'_>,
) -> Result<CompiledCandidates, AdmissionError> {
    plan.certify()
        .map_err(|error| AdmissionError::MalformedPlan {
            reason: error.to_string(),
        })?;
    let admitted = crate::admission::admit_candidates(plan, env)?;
    let units = emit_units(&admitted)?;
    Ok(CompiledCandidates {
        attribution: units.iter().map(UnitAttribution::of).collect(),
        units,
        plan_id: plan.id(),
        registry_id: admitted.instance_id,
        registry_fingerprint: admitted.fingerprint,
        depths: plan.depths(),
        unserved_terms: plan.unserved_evidence(),
        producer_bindings: plan.parts.producer_bindings.clone(),
        producer_decisions: plan.parts.producer_decisions.clone(),
    })
}

impl crate::compile::sealed::Sealed for CompiledCandidates {}
impl CompiledRead for CompiledCandidates {
    type Output<'d> = CandidateExecutionResult<'d>;
    fn units(&self) -> &[StratumUnit] {
        &self.units
    }
    fn registry_id(&self) -> RegistryId {
        self.registry_id
    }
    fn plan_id(&self) -> PlanId {
        self.plan_id
    }
    fn tagged_as_assembled(&self) -> Result<(), crate::ExecutionError> {
        tagged_units_as_assembled(self.plan_id, &self.units, &self.attribution)?;
        for unit in &self.units {
            if self.depths.get(&unit.stratum) != Some(unit.depth()) {
                return Err(crate::ExecutionError::UnitsNotAsAssembled {
                    plan: self.plan_id,
                    reason: format!(
                        "candidate stratum {} was requested at {:?} ranks but its unit reads {}",
                        unit.stratum,
                        self.depths.get(&unit.stratum),
                        unit.depth()
                    ),
                });
            }
        }
        Ok(())
    }
    fn finish<'d>(
        &self,
        streams: Vec<ReadStratum<'d>>,
        statuses: FastMap<Iri, ProducerStatus>,
    ) -> Self::Output<'d> {
        CandidateExecutionResult {
            streams,
            statuses,
            plan_id: self.plan_id,
            depths: self.depths.clone(),
            contracts: self
                .units
                .iter()
                .map(|unit| (unit.stratum.clone(), unit.contract.clone()))
                .collect(),
            unserved_terms: self.unserved_terms.clone(),
            producer_bindings: self.producer_bindings.clone(),
            producer_decisions: self.producer_decisions.clone(),
        }
    }
}
