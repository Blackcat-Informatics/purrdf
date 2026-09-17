// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The plan value: pure, inspectable, editable and serializable data.
//!
//! A plan records everything the planner decided — which request terms it
//! bound, which producers it selected or rejected and why, the depth and weight
//! of every stratum, the statistics it was planned against, and both registry
//! identities. It is a value rather than a reporting obligation, so a caller
//! can inspect or correct it term by term.
//!
//! The same property makes a plan untrusted input: a value that can be edited
//! can be edited wrongly, and one that can be deserialized can be forged. The
//! pipeline therefore admits plans at a later `compile` stage; this module
//! defines the value and its canonical, versioned, domain-separated identity.
//!
//! # A plan records where it came from
//!
//! One of the two registry identities a plan carries — the live instance id —
//! is a process-lifetime counter, so it is meaningful only inside the process
//! that minted it. A plan that crossed a process boundary carries a number that
//! names no live registry, and a number that names no live registry is
//! indistinguishable, by inspection, from one that names a *different* live
//! registry. Those two cases must be admitted on different terms, so the plan
//! records which it is in [`PlanOrigin`] rather than leaving admission to guess.

use std::collections::HashMap;

use purrdf_sparql_eval::RegistryId;
use purrdf_text::Fixed;
use serde::{Deserialize, Serialize};

use crate::canonical::{Reader, Writer};
use crate::error::PlanError;
use crate::id::{PLAN_VERSION, PlanId};
use crate::iri::{Iri, Term, Weight};
use crate::request::{Metric, RequestTerm};

// Canonical discriminators. One tag space per enum, never reused.
const TERM_LEXICAL: u8 = 0;
const TERM_VECTOR: u8 = 1;
const TERM_SPATIAL: u8 = 2;
const TERM_ENTITY_SEED: u8 = 3;

const METRIC_COSINE: u8 = 0;
const METRIC_DOT: u8 = 1;
const METRIC_EUCLIDEAN: u8 = 2;

const DECISION_SELECTED: u8 = 0;
const DECISION_REJECTED: u8 = 1;

const REJECT_NOT_RANKED: u8 = 0;
const REJECT_NO_ACCEPTED_TERM: u8 = 1;
const REJECT_DEPTH_EXCEEDED: u8 = 2;
const REJECT_UNSATISFIED_CONSTRAINT: u8 = 3;

const UNSERVED_NO_PRODUCER_ACCEPTS: u8 = 0;
const UNSERVED_EVERY_ACCEPTING_PRODUCER_REJECTED: u8 = 1;
const UNSERVED_UNBOUND: u8 = 2;

const PRESENT: u8 = 1;
const ABSENT: u8 = 0;

/// Serde bridge for [`RegistryId`], which carries its counter privately and has
/// no serde impl of its own. The value is encoded as the raw `u64` counter; a
/// decoded identity names no live registry, which is why admission falls back to
/// the durable content fingerprint for a deserialized plan.
mod registry_serde {
    use purrdf_sparql_eval::RegistryId;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Encode the registry identity as its raw counter.
    ///
    /// The by-reference parameter is fixed by serde's `with` contract; the
    /// by-value form the lint prefers cannot be expressed here.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(crate) fn serialize<S: Serializer>(
        value: &RegistryId,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(value.as_u64())
    }

    /// Decode a registry identity from its raw counter.
    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<RegistryId, D::Error> {
        Ok(RegistryId::from_raw(u64::deserialize(deserializer)?))
    }
}

/// Where a plan value came from, and therefore which of the two registry
/// identities admission can hold it to.
///
/// [`Plan::registry_instance_id`] is a
/// [`RegistryId`](purrdf_sparql_eval::RegistryId): a monotonic counter that is
/// unique among the registries built during **one** process's lifetime, and
/// meaningless outside it. So the same stored number means two different things:
///
/// * in the process that planned it, it names a registry admission can compare
///   against, and a mismatch is a real refusal — two registries that declare
///   byte-identical contents can still register one IRI to two implementations
///   that answer differently, and the instance id is the only value that sees
///   that difference;
/// * in any other process, it names nothing at all, and comparing it against a
///   freshly minted counter refuses every plan that was ever serialized.
///
/// Nothing in the number itself distinguishes those two cases, so the plan
/// records which it is. A [`Deserialized`](Self::Deserialized) plan is admitted
/// against [`Plan::registry_content_fingerprint`] — the durable,
/// instance-independent digest of the registry's declared contents — which is
/// the strongest claim a plan that crossed a process boundary can make.
///
/// # It is data, like every other field
///
/// Origin is public and editable, exactly like the rest of the plan, and
/// admission treats it as untrusted input like the rest of the plan. Editing it
/// never removes a check: it only chooses which of the two registry identities
/// the plan is held to, and both are checked against the live environment.
/// Marking a decoded plan [`SameProcess`](Self::SameProcess) makes admission
/// *stricter* (the instance must now match); marking a fresh plan
/// [`Deserialized`](Self::Deserialized) leaves it held to the content
/// fingerprint, which is the same bar every serialized plan clears.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlanOrigin {
    /// The plan was produced in this process — by [`plan`](crate::plan), or by
    /// a caller building one by hand against a live registry — so
    /// [`Plan::registry_instance_id`] names a registry this process minted and
    /// admission holds the plan to that exact instance.
    SameProcess,
    /// The plan was reconstructed from bytes, by
    /// [`Plan::from_canonical_bytes`] or by serde, so
    /// [`Plan::registry_instance_id`] is a counter value from some other
    /// process and admission holds the plan to
    /// [`Plan::registry_content_fingerprint`] instead.
    ///
    /// This is the default because it is what every decode path produces: a
    /// value carried by `#[serde(skip)]` is filled in by [`Default`], and a
    /// decoded plan is precisely the case that must not be held to a counter it
    /// cannot have minted.
    #[default]
    Deserialized,
}

/// A producer's binding of one selection of a request term.
///
/// The binding names the registered producer IRI, the stratum it emits under,
/// and which request terms (by index into [`Plan::request_terms`]) it was given.
/// It carries no function pointer, no trait object and no `TermId`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProducerBinding {
    /// The registered producer IRI, byte-exact.
    pub producer: String,
    /// The caller-supplied stratum label the producer emits under.
    pub stratum: Iri,
    /// Indices into [`Plan::request_terms`] this producer was bound to.
    pub request_terms: Vec<u32>,
}

/// Why a producer was not selected for a plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RejectionReason {
    /// The producer declares no ranked capability.
    NotRanked,
    /// No request term matched any shape the producer accepts.
    NoAcceptedTerm,
    /// The producer was omitted because a depth bound excluded it.
    DepthExceeded,
    /// A registry-declared constraint the producer requires was not satisfied.
    UnsatisfiedConstraint,
}

/// The planner's decision for one producer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProducerDecision {
    /// The producer was selected and bound.
    Selected {
        /// The registered producer IRI.
        producer: String,
        /// The stratum it was selected under.
        stratum: Iri,
    },
    /// The producer was rejected, with a reason.
    Rejected {
        /// The registered producer IRI.
        producer: String,
        /// Why it was not selected.
        reason: RejectionReason,
    },
}

/// Why one request term reached no producer at all.
///
/// [`RejectionReason`] answers the question per **producer**: this producer was
/// not selected, and here is the dimension that refused it. That is a different
/// question from the one a caller asks about its own request, which is per
/// **term**: I asked for this, did anything answer it? A request whose every
/// producer is accounted for can still carry a term nothing was ever going to
/// serve, and a term that reached nothing must be visible as exactly that rather
/// than as a quiet omission — the request lattice deliberately carries modalities
/// ahead of the producers that answer them, which is honest only when an
/// unanswered one says so.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnservedReason {
    /// No registered producer declares a shape that accepts this term, so
    /// nothing was ever a candidate for it. This is the armed-but-unserved
    /// modality: the request lattice can express the term and this registry has
    /// nobody who takes it.
    NoProducerAccepts,
    /// Some producer's declaration accepts this term's shape, but every producer
    /// that accepted it was rejected before selection — because it could not be
    /// invoked, or because it declares no ranked capability at all. The
    /// producer's own dimension is in
    /// [`Plan::producer_decisions`](Plan::producer_decisions).
    EveryAcceptingProducerRejected,
    /// The plan routes this term to no producer and records no reason of its
    /// own for that.
    ///
    /// The planner always records one of the reasons above, so this is reached
    /// only by a plan that was hand-built or edited — legitimately, since
    /// narrowing a producer the registry does not declare mandatory is allowed.
    /// The term is still named rather than dropped: the evidence a caller reads
    /// says what the plan it actually ran supports, never what an earlier
    /// version of that plan said.
    Unbound,
}

/// One request term that reached no producer, and why.
///
/// `request_term` indexes [`Plan::request_terms`], which is the request in the
/// caller's own order, so a caller reads the evidence straight back onto the
/// term it wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnservedTerm {
    /// The index into [`Plan::request_terms`] of the term nothing served.
    pub request_term: u32,
    /// Why nothing served it.
    pub reason: UnservedReason,
}

/// One entry of a statistics snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatisticsEntry {
    /// A caller-supplied subject label (a predicate IRI, a producer IRI, …).
    pub subject: String,
    /// The cardinality the statistics provider reported for it.
    pub cardinality: u64,
    /// An optional selectivity in parts per million.
    pub selectivity_ppm: Option<u64>,
}

/// The statistics a plan was planned against.
///
/// Statistics are an explicit input to planning, not something the planner
/// reaches into a store for; the emitted plan records what it assumed so a
/// replay against moved statistics is a detectable condition rather than a
/// silent replan.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatisticsSnapshot {
    /// A caller-supplied label for the statistics provider.
    pub source: String,
    /// The provider-declared revision of the snapshot.
    pub revision: String,
    /// The snapshot's entries, in caller order.
    pub entries: Vec<StatisticsEntry>,
}

/// A retrieval plan: pure data, versioned and content-addressed.
///
/// Every field is public and owned, so a caller can inspect and edit the plan
/// and hand it back. [`Plan::id`] is the plan's canonical identity; two plans
/// are equal iff their content fields are equal iff their canonical bytes and
/// ids are equal.
///
/// [`Plan::origin`] is the one field outside that biconditional, because it is
/// provenance rather than content: a plan and its own decoded round trip are the
/// same plan and must keep one identity, yet they are reached differently and
/// are therefore admitted against different registry identities. It is absent
/// from [`Plan::canonical_bytes`] and from equality for that single reason.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    /// The plan layout version. Callers constructing a plan by hand must set
    /// this to [`PLAN_VERSION`](crate::PLAN_VERSION); a decoded plan whose header
    /// disagrees in the other direction is refused by
    /// [`Plan::from_canonical_bytes`].
    pub version: u16,
    /// The request terms this plan was built for, in caller order.
    pub request_terms: Vec<RequestTerm>,
    /// The producers selected, with the request terms each was bound to.
    pub producer_bindings: Vec<ProducerBinding>,
    /// Every producer considered, selected or rejected, with reasons.
    pub producer_decisions: Vec<ProducerDecision>,
    /// Every request term that reached no producer, ascending by index.
    ///
    /// The planner records one entry per term no binding carries, with the
    /// reason that term went unserved. It is not derivable from
    /// [`Self::producer_bindings`] alone: "nothing accepts this shape" and
    /// "something accepts it but every acceptor was rejected" are different
    /// facts about a registry, and only the planner saw both.
    ///
    /// Read the evidence through [`Self::unserved_evidence`] rather than from
    /// this field: an edited plan's bindings and this list can disagree, and the
    /// accessor reports what the plan in hand actually supports.
    pub unserved_terms: Vec<UnservedTerm>,
    /// Per-stratum maximum depth, keyed by stratum label.
    pub stratum_depths: HashMap<Iri, u32>,
    /// Per-stratum fusion weight, keyed by stratum label.
    pub stratum_weights: HashMap<Iri, Weight>,
    /// The statistics snapshot the plan was planned against.
    pub statistics_snapshot: StatisticsSnapshot,
    /// The live registry instance the plan was planned against. This is a
    /// process-lifetime value, so it is compared only when
    /// [`Plan::origin`] says the plan never left the process that recorded it;
    /// a [`PlanOrigin::Deserialized`] plan is admitted against
    /// [`Plan::registry_content_fingerprint`] instead.
    #[serde(with = "registry_serde")]
    pub registry_instance_id: RegistryId,
    /// The durable, instance-independent fingerprint of the registry's declared
    /// contents the plan was planned against.
    pub registry_content_fingerprint: String,
    /// Where this plan value came from, which decides which registry identity
    /// admission holds it to.
    ///
    /// Skipped by serde and absent from [`Plan::canonical_bytes`]: it describes
    /// how this value was *reached*, not what it says, so encoding it would give
    /// a plan and its own round trip two identities. Every decode path
    /// therefore yields [`PlanOrigin::Deserialized`], which is
    /// [`PlanOrigin`]'s [`Default`].
    #[serde(skip)]
    pub origin: PlanOrigin,
}

impl PartialEq for Plan {
    /// Equality over the plan's content, which excludes [`Plan::origin`].
    ///
    /// The type documents that two plans are equal iff their canonical bytes
    /// are equal iff their ids are equal, and origin is deliberately not in the
    /// canonical bytes; comparing it here would break that biconditional and
    /// make a plan unequal to its own decoded round trip.
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.request_terms == other.request_terms
            && self.producer_bindings == other.producer_bindings
            && self.producer_decisions == other.producer_decisions
            && self.unserved_terms == other.unserved_terms
            && self.stratum_depths == other.stratum_depths
            && self.stratum_weights == other.stratum_weights
            && self.statistics_snapshot == other.statistics_snapshot
            && self.registry_instance_id == other.registry_instance_id
            && self.registry_content_fingerprint == other.registry_content_fingerprint
    }
}

impl Eq for Plan {}

impl Plan {
    /// The version this build writes and understands.
    pub const VERSION: u16 = PLAN_VERSION;

    /// The plan's canonical, length-framed bytes.
    ///
    /// The encoding is a pure function of the plan's fields. Map entries are
    /// sorted by key, so it does not depend on `HashMap` iteration order, and
    /// all integers are little-endian, so it is identical on every target.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut writer = Writer::new();
        writer.u16(self.version);
        write_request_terms(&mut writer, &self.request_terms);
        write_bindings(&mut writer, &self.producer_bindings);
        write_decisions(&mut writer, &self.producer_decisions);
        write_depths(&mut writer, &self.stratum_depths);
        write_weights(&mut writer, &self.stratum_weights);
        write_statistics(&mut writer, &self.statistics_snapshot);
        writer.u64(self.registry_instance_id.as_u64());
        writer.string(&self.registry_content_fingerprint);
        write_unserved_terms(&mut writer, &self.unserved_terms);
        writer.into_bytes()
    }

    /// Decode a plan from its canonical bytes.
    ///
    /// # Errors
    ///
    /// [`PlanError`] when the encoding is truncated, carries an unknown tag or
    /// invalid UTF-8, contains an invalid IRI, has trailing bytes, or — loudly,
    /// as its own variant — begins with a version this build does not write.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, PlanError> {
        let mut reader = Reader::new(bytes);
        let version = reader.u16()?;
        if version != PLAN_VERSION {
            return Err(PlanError::VersionMismatch {
                found: version,
                expected: PLAN_VERSION,
            });
        }
        let request_terms = read_request_terms(&mut reader)?;
        let producer_bindings = read_bindings(&mut reader)?;
        let producer_decisions = read_decisions(&mut reader)?;
        let stratum_depths = read_depths(&mut reader)?;
        let stratum_weights = read_weights(&mut reader)?;
        let statistics_snapshot = read_statistics(&mut reader)?;
        let registry_instance_id = RegistryId::from_raw(reader.u64()?);
        let registry_content_fingerprint = reader.string("registry content fingerprint")?;
        let unserved_terms = read_unserved_terms(&mut reader)?;
        reader.finish()?;
        // Reconstructed from bytes, so the instance id just read is a counter
        // value this process cannot have minted; the plan is held to its
        // content fingerprint instead. Recorded here rather than inferred at
        // admission, which cannot tell a foreign counter from a stale one.
        Ok(Self {
            origin: PlanOrigin::Deserialized,
            version,
            request_terms,
            producer_bindings,
            producer_decisions,
            unserved_terms,
            stratum_depths,
            stratum_weights,
            statistics_snapshot,
            registry_instance_id,
            registry_content_fingerprint,
        })
    }

    /// The plan's content identity.
    #[must_use]
    pub fn id(&self) -> PlanId {
        PlanId::from_canonical(&self.canonical_bytes())
    }

    /// The per-term evidence **this** plan supports: every request term no
    /// binding of it carries, ascending by index, each with the reason the plan
    /// records for it — or [`UnservedReason::Unbound`] when it records none.
    ///
    /// This is the value to report, and [`Self::unserved_terms`] is only one of
    /// its two inputs. A plan is editable, so its recorded list and its bindings
    /// can disagree in both directions, and each disagreement has an honest
    /// reading:
    ///
    /// * a term the recorded list names that a binding does serve is **not**
    ///   reported, because the plan in hand does serve it — reporting it would
    ///   raise an alarm the plan itself falsifies;
    /// * a term no binding serves that the recorded list omits **is** reported,
    ///   as [`UnservedReason::Unbound`], because narrowing a producer is a
    ///   legitimate edit and the term it stranded is exactly the quiet omission
    ///   this evidence exists to prevent.
    ///
    /// So the answer's evidence is always true of the plan that actually ran,
    /// and no forged or stale entry can make it claim otherwise.
    #[must_use]
    pub fn unserved_evidence(&self) -> Vec<UnservedTerm> {
        let mut served = vec![false; self.request_terms.len()];
        for binding in &self.producer_bindings {
            for index in &binding.request_terms {
                if let Some(slot) = served.get_mut(*index as usize) {
                    *slot = true;
                }
            }
        }
        served
            .iter()
            .enumerate()
            .filter(|(_, served)| !**served)
            .map(|(index, _)| {
                let request_term = u32::try_from(index).unwrap_or(u32::MAX);
                UnservedTerm {
                    request_term,
                    reason: self
                        .unserved_terms
                        .iter()
                        .find(|entry| entry.request_term == request_term)
                        .map_or(UnservedReason::Unbound, |entry| entry.reason),
                }
            })
            .collect()
    }
}

fn metric_tag(metric: Metric) -> u8 {
    match metric {
        Metric::Cosine => METRIC_COSINE,
        Metric::Dot => METRIC_DOT,
        Metric::Euclidean => METRIC_EUCLIDEAN,
    }
}

fn metric_from_tag(tag: u8) -> Result<Metric, PlanError> {
    match tag {
        METRIC_COSINE => Ok(Metric::Cosine),
        METRIC_DOT => Ok(Metric::Dot),
        METRIC_EUCLIDEAN => Ok(Metric::Euclidean),
        tag => Err(PlanError::InvalidTag {
            what: "metric",
            tag,
        }),
    }
}

fn reason_tag(reason: RejectionReason) -> u8 {
    match reason {
        RejectionReason::NotRanked => REJECT_NOT_RANKED,
        RejectionReason::NoAcceptedTerm => REJECT_NO_ACCEPTED_TERM,
        RejectionReason::DepthExceeded => REJECT_DEPTH_EXCEEDED,
        RejectionReason::UnsatisfiedConstraint => REJECT_UNSATISFIED_CONSTRAINT,
    }
}

fn reason_from_tag(tag: u8) -> Result<RejectionReason, PlanError> {
    match tag {
        REJECT_NOT_RANKED => Ok(RejectionReason::NotRanked),
        REJECT_NO_ACCEPTED_TERM => Ok(RejectionReason::NoAcceptedTerm),
        REJECT_DEPTH_EXCEEDED => Ok(RejectionReason::DepthExceeded),
        REJECT_UNSATISFIED_CONSTRAINT => Ok(RejectionReason::UnsatisfiedConstraint),
        tag => Err(PlanError::InvalidTag {
            what: "rejection reason",
            tag,
        }),
    }
}

fn unserved_tag(reason: UnservedReason) -> u8 {
    match reason {
        UnservedReason::NoProducerAccepts => UNSERVED_NO_PRODUCER_ACCEPTS,
        UnservedReason::EveryAcceptingProducerRejected => {
            UNSERVED_EVERY_ACCEPTING_PRODUCER_REJECTED
        }
        UnservedReason::Unbound => UNSERVED_UNBOUND,
    }
}

fn unserved_from_tag(tag: u8) -> Result<UnservedReason, PlanError> {
    match tag {
        UNSERVED_NO_PRODUCER_ACCEPTS => Ok(UnservedReason::NoProducerAccepts),
        UNSERVED_EVERY_ACCEPTING_PRODUCER_REJECTED => {
            Ok(UnservedReason::EveryAcceptingProducerRejected)
        }
        UNSERVED_UNBOUND => Ok(UnservedReason::Unbound),
        tag => Err(PlanError::InvalidTag {
            what: "unserved term reason",
            tag,
        }),
    }
}

fn write_unserved_terms(writer: &mut Writer, terms: &[UnservedTerm]) {
    writer.u64(terms.len() as u64);
    for term in terms {
        writer.u32(term.request_term);
        writer.u8(unserved_tag(term.reason));
    }
}

fn read_unserved_terms(reader: &mut Reader<'_>) -> Result<Vec<UnservedTerm>, PlanError> {
    let count = reader.count()?;
    let mut terms = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let request_term = reader.u32()?;
        let reason = unserved_from_tag(reader.u8()?)?;
        terms.push(UnservedTerm {
            request_term,
            reason,
        });
    }
    Ok(terms)
}

fn write_iri(writer: &mut Writer, iri: &Iri) {
    writer.string(iri.as_str());
}

fn read_iri(reader: &mut Reader<'_>, what: &'static str) -> Result<Iri, PlanError> {
    let text = reader.string(what)?;
    Iri::parse(&text)
}

fn write_option_fixed(writer: &mut Writer, value: Option<Fixed>) {
    match value {
        None => writer.u8(ABSENT),
        Some(value) => {
            writer.u8(PRESENT);
            writer.i128(value.into_raw());
        }
    }
}

fn read_option_fixed(reader: &mut Reader<'_>) -> Result<Option<Fixed>, PlanError> {
    match reader.u8()? {
        ABSENT => Ok(None),
        PRESENT => Ok(Some(Fixed::from_raw(reader.i128()?))),
        tag => Err(PlanError::InvalidTag {
            what: "optional fixed value",
            tag,
        }),
    }
}

fn write_request_term(writer: &mut Writer, term: &RequestTerm) {
    match term {
        RequestTerm::Lexical {
            text,
            language,
            predicate,
        } => {
            writer.u8(TERM_LEXICAL);
            writer.string(text);
            writer.option_string(language.as_deref());
            writer.option_string(predicate.as_ref().map(Iri::as_str));
        }
        RequestTerm::Vector {
            embedding,
            metric,
            index_hint,
        } => {
            writer.u8(TERM_VECTOR);
            writer.u64(embedding.len() as u64);
            for value in embedding {
                writer.f32_bits(*value);
            }
            writer.u8(metric_tag(*metric));
            writer.option_string(index_hint.as_deref());
        }
        RequestTerm::Spatial {
            geometry,
            predicate,
            max_distance,
        } => {
            writer.u8(TERM_SPATIAL);
            writer.string(geometry);
            write_iri(writer, predicate);
            write_option_fixed(writer, *max_distance);
        }
        RequestTerm::EntitySeed { entity } => {
            writer.u8(TERM_ENTITY_SEED);
            writer.string(entity.as_str());
        }
    }
}

fn read_request_term(reader: &mut Reader<'_>) -> Result<RequestTerm, PlanError> {
    match reader.u8()? {
        TERM_LEXICAL => {
            let text = reader.string("lexical text")?;
            let language = reader.option_string("lexical language")?;
            let predicate = match reader.u8()? {
                ABSENT => None,
                PRESENT => Some(read_iri(reader, "lexical predicate")?),
                tag => {
                    return Err(PlanError::InvalidTag {
                        what: "lexical predicate presence",
                        tag,
                    });
                }
            };
            Ok(RequestTerm::Lexical {
                text,
                language,
                predicate,
            })
        }
        TERM_VECTOR => {
            let count = reader.count()?;
            let mut embedding = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                embedding.push(reader.f32_bits()?);
            }
            let metric = metric_from_tag(reader.u8()?)?;
            let index_hint = reader.option_string("vector index hint")?;
            Ok(RequestTerm::Vector {
                embedding,
                metric,
                index_hint,
            })
        }
        TERM_SPATIAL => {
            let geometry = reader.string("spatial geometry")?;
            let predicate = read_iri(reader, "spatial predicate")?;
            let max_distance = read_option_fixed(reader)?;
            Ok(RequestTerm::Spatial {
                geometry,
                predicate,
                max_distance,
            })
        }
        TERM_ENTITY_SEED => Ok(RequestTerm::EntitySeed {
            entity: Term::new(reader.string("entity seed")?),
        }),
        tag => Err(PlanError::InvalidTag {
            what: "request term",
            tag,
        }),
    }
}

fn write_request_terms(writer: &mut Writer, terms: &[RequestTerm]) {
    writer.u64(terms.len() as u64);
    for term in terms {
        write_request_term(writer, term);
    }
}

fn read_request_terms(reader: &mut Reader<'_>) -> Result<Vec<RequestTerm>, PlanError> {
    let count = reader.count()?;
    let mut terms = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        terms.push(read_request_term(reader)?);
    }
    Ok(terms)
}

fn write_bindings(writer: &mut Writer, bindings: &[ProducerBinding]) {
    writer.u64(bindings.len() as u64);
    for binding in bindings {
        writer.string(&binding.producer);
        write_iri(writer, &binding.stratum);
        writer.u64(binding.request_terms.len() as u64);
        for index in &binding.request_terms {
            writer.u32(*index);
        }
    }
}

fn read_bindings(reader: &mut Reader<'_>) -> Result<Vec<ProducerBinding>, PlanError> {
    let count = reader.count()?;
    let mut bindings = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let producer = reader.string("producer IRI")?;
        let stratum = read_iri(reader, "binding stratum")?;
        let term_count = reader.count()?;
        let mut request_terms = Vec::with_capacity(term_count.min(1024));
        for _ in 0..term_count {
            request_terms.push(reader.u32()?);
        }
        bindings.push(ProducerBinding {
            producer,
            stratum,
            request_terms,
        });
    }
    Ok(bindings)
}

fn write_decisions(writer: &mut Writer, decisions: &[ProducerDecision]) {
    writer.u64(decisions.len() as u64);
    for decision in decisions {
        match decision {
            ProducerDecision::Selected { producer, stratum } => {
                writer.u8(DECISION_SELECTED);
                writer.string(producer);
                write_iri(writer, stratum);
            }
            ProducerDecision::Rejected { producer, reason } => {
                writer.u8(DECISION_REJECTED);
                writer.string(producer);
                writer.u8(reason_tag(*reason));
            }
        }
    }
}

fn read_decisions(reader: &mut Reader<'_>) -> Result<Vec<ProducerDecision>, PlanError> {
    let count = reader.count()?;
    let mut decisions = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        match reader.u8()? {
            DECISION_SELECTED => {
                let producer = reader.string("selected producer IRI")?;
                let stratum = read_iri(reader, "selected stratum")?;
                decisions.push(ProducerDecision::Selected { producer, stratum });
            }
            DECISION_REJECTED => {
                let producer = reader.string("rejected producer IRI")?;
                let reason = reason_from_tag(reader.u8()?)?;
                decisions.push(ProducerDecision::Rejected { producer, reason });
            }
            tag => {
                return Err(PlanError::InvalidTag {
                    what: "producer decision",
                    tag,
                });
            }
        }
    }
    Ok(decisions)
}

fn write_depths(writer: &mut Writer, depths: &HashMap<Iri, u32>) {
    let mut entries: Vec<(&Iri, u32)> = depths.iter().map(|(key, value)| (key, *value)).collect();
    entries.sort_by(|left, right| left.0.as_str().cmp(right.0.as_str()));
    writer.u64(entries.len() as u64);
    for (key, value) in entries {
        write_iri(writer, key);
        writer.u32(value);
    }
}

fn read_depths(reader: &mut Reader<'_>) -> Result<HashMap<Iri, u32>, PlanError> {
    let count = reader.count()?;
    let mut depths = HashMap::with_capacity(count.min(1024));
    for _ in 0..count {
        let key = read_iri(reader, "stratum depth key")?;
        let value = reader.u32()?;
        depths.insert(key, value);
    }
    Ok(depths)
}

fn write_weights(writer: &mut Writer, weights: &HashMap<Iri, Weight>) {
    let mut entries: Vec<(&Iri, Weight)> =
        weights.iter().map(|(key, value)| (key, *value)).collect();
    entries.sort_by(|left, right| left.0.as_str().cmp(right.0.as_str()));
    writer.u64(entries.len() as u64);
    for (key, value) in entries {
        write_iri(writer, key);
        writer.i128(value.into_raw());
    }
}

fn read_weights(reader: &mut Reader<'_>) -> Result<HashMap<Iri, Weight>, PlanError> {
    let count = reader.count()?;
    let mut weights = HashMap::with_capacity(count.min(1024));
    for _ in 0..count {
        let key = read_iri(reader, "stratum weight key")?;
        let value = Weight::from_raw(reader.i128()?);
        weights.insert(key, value);
    }
    Ok(weights)
}

fn write_statistics(writer: &mut Writer, snapshot: &StatisticsSnapshot) {
    writer.string(&snapshot.source);
    writer.string(&snapshot.revision);
    writer.u64(snapshot.entries.len() as u64);
    for entry in &snapshot.entries {
        writer.string(&entry.subject);
        writer.u64(entry.cardinality);
        match entry.selectivity_ppm {
            None => writer.u8(ABSENT),
            Some(value) => {
                writer.u8(PRESENT);
                writer.u64(value);
            }
        }
    }
}

fn read_statistics(reader: &mut Reader<'_>) -> Result<StatisticsSnapshot, PlanError> {
    let source = reader.string("statistics source")?;
    let revision = reader.string("statistics revision")?;
    let count = reader.count()?;
    let mut entries = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let subject = reader.string("statistics subject")?;
        let cardinality = reader.u64()?;
        let selectivity_ppm = match reader.u8()? {
            ABSENT => None,
            PRESENT => Some(reader.u64()?),
            tag => {
                return Err(PlanError::InvalidTag {
                    what: "statistics selectivity presence",
                    tag,
                });
            }
        };
        entries.push(StatisticsEntry {
            subject,
            cardinality,
            selectivity_ppm,
        });
    }
    Ok(StatisticsSnapshot {
        source,
        revision,
        entries,
    })
}
