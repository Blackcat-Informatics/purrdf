// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The plan value: pure, inspectable, editable and serializable data.
//!
//! A plan records everything the planner decided — which request terms it
//! bound, which producers it selected or rejected and why, the depth of every
//! stratum, the statistics it was planned against, and both registry
//! identities. It is a value rather than a reporting obligation, so a caller
//! can inspect or correct it term by term.
//!
//! What it does **not** record is how those strata will be weighted against each
//! other. That is a [`FusionProfile`](crate::FusionProfile), chosen at `fuse`
//! time and deliberately not a planning input (§8 of the design record), so a
//! plan carrying a second weight map could only ever be a number no fusion
//! reads — and a plan identity sensitive to it.
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

use std::collections::{BTreeMap, HashMap};

use purrdf_sparql_eval::RegistryId;
use purrdf_text::Fixed;
use serde::{Deserialize, Serialize, Serializer};

use crate::canonical::{Reader, Writer};
use crate::error::{PlanError, StatisticsDimension};
use crate::fuse::TopK;
use crate::id::{PLAN_VERSION, PlanId};
use crate::iri::{Iri, Term};
use crate::request::{Metric, ReadBound, RequestTerm};

// Canonical discriminators. One tag space per enum, never reused.
//
// A tag space grows forward-compatibly and does **not** move
// [`PLAN_VERSION`](crate::PLAN_VERSION) with it. The version exists to stop
// bytes being reinterpreted under a layout they were not written for, and
// appending a discriminator reinterprets nothing: every previously-written tag
// keeps its number and its encoding, so an old plan's canonical bytes — and
// therefore its [`PlanId`] — are unchanged, and a build that meets a tag it does
// not know refuses it by name with [`PlanError::InvalidTag`] rather than reading
// it as something else. Bumping the version instead would refuse every plan
// already issued, in order to protect against a misreading that cannot occur.
const TERM_LEXICAL: u8 = 0;
const TERM_VECTOR: u8 = 1;
const TERM_SPATIAL: u8 = 2;
const TERM_ENTITY_SEED: u8 = 3;
const TERM_TEMPORAL: u8 = 4;
const TERM_NUMERIC_RANGE: u8 = 5;

const METRIC_COSINE: u8 = 0;
const METRIC_DOT: u8 = 1;
const METRIC_EUCLIDEAN: u8 = 2;

const DECISION_SELECTED: u8 = 0;
const DECISION_REJECTED: u8 = 1;

const REJECT_NOT_RANKED: u8 = 0;
const REJECT_NO_ACCEPTED_TERM: u8 = 1;
const REJECT_DEPTH_EXCEEDED: u8 = 2;
const REJECT_UNSATISFIED_CONSTRAINT: u8 = 3;

const BOUND_COMPLETE: u8 = 0;
const BOUND_BOUNDED: u8 = 1;

const UNSERVED_NO_PRODUCER_ACCEPTS: u8 = 0;
const UNSERVED_EVERY_ACCEPTING_PRODUCER_REJECTED: u8 = 1;
const UNSERVED_UNBOUND: u8 = 2;
const UNSERVED_ACCEPTED_WITHOUT_PLACEMENT: u8 = 3;

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
    ///
    /// A producer that declares no access mode at all lands here: it admits no
    /// invocation, so placement cannot render one. A producer that declares an
    /// access mode promising **zero rows** does not — that is a measurement of its
    /// data, not a refusal of its own invocation, so it is selected like any other
    /// and planned at the floored depth of one, where it reads and reports its own
    /// emptiness. "Declared nothing" and "declared zero" are different facts and
    /// only the first is a rejection.
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
    /// Some producer's declaration accepts this term's shape, and the
    /// alternative that matched it declares **no placement** — so the producer
    /// would be invoked with none of the term written into its arguments.
    ///
    /// This is the distinction between a producer that *matches* a term and one
    /// that *receives* it. A declaration is entitled to accept a shape it wants
    /// matched but not written: such a producer ranks within its stratum without
    /// reading this term, and it is selected and emitted like any other. What it
    /// does not do is serve the term, so the term is reported here rather than
    /// recorded as bound — an empty
    /// [`Plan::unserved_terms`](Plan::unserved_terms) means every term reached a
    /// producer *with its content*, and a term whose content reached nothing
    /// cannot be one of them.
    ///
    /// Which producer matched it is in
    /// [`Plan::producer_decisions`](Plan::producer_decisions) and
    /// [`Plan::producer_bindings`](Plan::producer_bindings), the same place
    /// [`Self::EveryAcceptingProducerRejected`] sends a reader for the same
    /// reason: this value is per term, and naming a producer in it would make
    /// one term's evidence depend on which of several producers was named.
    AcceptedWithoutPlacement,
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

/// Everything one stratum's planned depth was derived from.
///
/// # Why a plan records this
///
/// A depth is a claim about how deep a read may go, and a claim a reader cannot
/// check is a claim a reader must take on trust. These fields are the *arguments*
/// to [`depth_from`](crate::depth_from), the one function that derives a depth —
/// so recording them makes [`Plan::certify`] able to recompute the number beside
/// them and refuse a plan whose depth does not follow from its own inputs.
///
/// That framing is also what keeps the record honest as the derivation grows:
/// `depth_from` takes nothing but a `DepthInputs`, so **an input that is not
/// recorded here cannot be an input at all**. A leg that goes unrecorded is a
/// compile error rather than a plan that silently under-explains itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepthInputs {
    /// The registry's declared row bound for this stratum, read at the mode the
    /// producer will actually be invoked under.
    ///
    /// [`u64::MAX`] is the genuinely unbounded declaration — more rows than any
    /// read can reach — and is the one value that skips the selectivity step
    /// below, because a fraction of an unmeasured total is not a measurement.
    pub declared: u64,
    /// The cardinality the statistics provider reported, or `None` when it
    /// reported none.
    ///
    /// Absent is not zero. Zero is a measurement — "the provider counted no
    /// rows" — and it narrows the bound before any ratio applies; absence is the
    /// provider declining to answer, and it leaves the declaration standing.
    pub cardinality: Option<u64>,
    /// The aggregate selectivity **that was applied**, in parts per million, or
    /// `None` when none was.
    ///
    /// This is the number the depth was actually derived from, never one
    /// recomputed beside it. The distinction is load-bearing: an unbounded
    /// stratum never reaches the selectivity step at all, so a provider may well
    /// report a selectivity for it that bounded nothing — and recording that
    /// value here would describe a derivation that did not happen.
    pub selectivity_ppm: Option<u64>,
    /// The ascending request-term indices whose reported selectivity contributed
    /// to [`Self::selectivity_ppm`]. Empty when none did.
    ///
    /// The aggregate is a **sum** over the terms a provider answered for, so the
    /// sum alone does not say which terms those were: a provider that moved a
    /// selectivity from one term to another at an unchanged total would leave an
    /// identical record, and the move — which is a different statement about the
    /// data — would be undetectable. Recording the domain makes the aggregate's
    /// derivation as checkable as the depth's.
    pub selectivity_terms: Vec<u32>,
    /// The request's licensed row prefix, or `None` when the registry's own
    /// bound stood.
    ///
    /// Recorded separately from [`Plan::read_bound`] because they answer
    /// different questions. The read bound is what the caller *asked for*; this
    /// is whether the declarations let that number bound this stratum. A plan
    /// carrying only the first records a depth whose derivation cannot be
    /// reconstructed whenever the answer was "no".
    pub licensed_prefix: Option<u64>,
}

/// Which recorded input bound a stratum's depth.
///
/// Returned by [`Plan::explain_depth`]. The variants are the legs of
/// [`depth_from`](crate::depth_from) in the order that function applies them, so
/// the answer names the constraint that actually bound the number rather than
/// the first one that could have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DepthCause {
    /// The registry's declared row bound was the narrowest input.
    Declaration,
    /// The provider's reported cardinality was narrower than the declaration.
    Cardinality,
    /// The applied selectivity scaled the bound below what the declaration and
    /// cardinality allowed.
    Selectivity,
    /// The request's licensed row prefix was the narrowest input.
    LicensedPrefix,
    /// Every other input reached zero and the floor lifted the read to a single
    /// probing row, so that emptiness is reported by the producer rather than
    /// claimed by the plan.
    Floor,
    /// The declaration promised more rows than a read can reach and no licensed
    /// prefix narrowed it, so the depth is the deepest a read can be taken to.
    Unbounded,
    /// The derived bound was finite but deeper than a plan can record, so it sits
    /// at the read ceiling.
    ReadCeiling,
}

/// One entry of a statistics snapshot: what a provider said about one subject
/// planning consulted.
///
/// A subject is here for one of two reasons, and the row reads the same either
/// way. A **stratum** is here because a depth was derived for it; its row is a
/// projection of the [`DepthInputs`] recorded in
/// [`Plan::stratum_derivations`](Plan::stratum_derivations), so the two records
/// state the same statistics and [`Plan::certify`] refuses a plan in which they
/// disagree. A **request predicate** is here because planning asked about it in
/// order to report: nothing derives a number from it — only a stratum's own
/// selectivity bounds a stratum's depth — so its row is context alone.
///
/// A subject that is both carries the stratum's row, because that is the
/// consultation that bound a depth.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatisticsEntry {
    /// A caller-supplied subject label (a predicate IRI, a producer IRI, …).
    pub subject: String,
    /// The cardinality the statistics provider reported for it, or `None` when
    /// it reported none.
    ///
    /// Absent rather than zero, for the reason
    /// [`DepthInputs::cardinality`] is: "the provider measured nothing" and "the
    /// provider measured zero" are different facts, and collapsing them would
    /// turn silence into a claim that no row matches.
    pub cardinality: Option<u64>,
    /// An optional selectivity in parts per million.
    pub selectivity_ppm: Option<u64>,
    /// The ascending request-term indices whose reported selectivity contributed
    /// to [`Self::selectivity_ppm`], for the reason
    /// [`DepthInputs::selectivity_terms`] records them.
    pub selectivity_terms: Vec<u32>,
}

/// The entries of a [`StatisticsSnapshot`]: ascending by subject, each subject
/// named once.
///
/// # The order is the type's law, not the encoder's
///
/// A plan's identity is a digest of its canonical bytes, and [`Plan`] documents
/// that two plans are equal iff their canonical bytes are equal iff their ids
/// are. A bare `Vec` cannot hold that up: comparing two snapshots compares their
/// vectors position by position, so a snapshot built descending is *unequal* to
/// its ascending twin — while an encoder that sorted before writing would give
/// the two identical bytes and therefore one identity. That is the biconditional
/// broken in the middle, and it is broken for exactly as long as the ordering
/// law lives in the encoder rather than in the value.
///
/// So the law lives here, where [`Plan::stratum_derivations`] already keeps its
/// own: order is established on construction, every read is of an ordered
/// sequence, and the encoder writes what it is given. Two constructions from the
/// same entries in any order are the same value, byte for byte and field for
/// field.
///
/// # Order is established; a duplicate is refused
///
/// The two are treated differently because they carry different amounts of
/// information. The order a caller happened to build its entries in says nothing
/// about the data — the snapshot is a set of rows keyed by subject, so
/// establishing the ascending order loses nothing a reader could have wanted.
///
/// A repeated subject is not like that. Two rows for one subject are two answers
/// to one question, and nothing in the value says which the plan was planned
/// against: keeping the first, the last, or the wider of them would be inventing
/// a rule the data does not carry. It is refused by name with
/// [`PlanError::DuplicateStatisticsSubject`], on every path in — including
/// serde's, so a hand-written JSON document is held to the same law as a caller
/// with a `Vec`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(try_from = "Vec<StatisticsEntry>")]
pub struct StatisticsEntries(Vec<StatisticsEntry>);

impl StatisticsEntries {
    /// The entries of `entries`, ordered ascending by subject.
    ///
    /// # Errors
    ///
    /// [`PlanError::DuplicateStatisticsSubject`] when two entries name one
    /// subject.
    pub fn new(mut entries: Vec<StatisticsEntry>) -> Result<Self, PlanError> {
        entries.sort_unstable_by(|left, right| left.subject.cmp(&right.subject));
        if let Some(pair) = entries
            .windows(2)
            .find(|pair| pair[0].subject == pair[1].subject)
        {
            return Err(PlanError::DuplicateStatisticsSubject {
                subject: pair[0].subject.clone(),
            });
        }
        Ok(Self(entries))
    }

    /// The row this snapshot records for `subject`, or `None` when it records
    /// none.
    ///
    /// A binary search rather than a scan, which the ascending order makes
    /// exact. [`Plan::certify`] asks this once per stratum, so a scan would make
    /// certifying a plan quadratic in a plan's own size — over a value that
    /// arrives from wherever a plan arrives from.
    #[must_use]
    pub fn get_subject(&self, subject: &str) -> Option<&StatisticsEntry> {
        self.0
            .binary_search_by(|entry| entry.subject.as_str().cmp(subject))
            .ok()
            .map(|index| &self.0[index])
    }
}

impl core::ops::Deref for StatisticsEntries {
    type Target = [StatisticsEntry];

    /// Read-only slice access: iteration, indexing, `len`, `to_vec`.
    ///
    /// Read-only is the point. A `&mut` view would let a caller move one
    /// subject's text and leave the sequence unordered or doubled, which is the
    /// state this type exists to make unrepresentable; an edit goes through
    /// [`Self::new`], which re-establishes the law over the whole sequence.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> IntoIterator for &'a StatisticsEntries {
    type Item = &'a StatisticsEntry;
    type IntoIter = core::slice::Iter<'a, StatisticsEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl TryFrom<Vec<StatisticsEntry>> for StatisticsEntries {
    type Error = PlanError;

    /// The conversion serde's `try_from` runs, so a deserialized snapshot is
    /// held to [`StatisticsEntries::new`]'s law rather than admitted around it.
    fn try_from(entries: Vec<StatisticsEntry>) -> Result<Self, Self::Error> {
        Self::new(entries)
    }
}

impl Serialize for StatisticsEntries {
    /// Serialized as the bare sequence of its entries, so the serde document is
    /// a JSON **array** of rows — the shape the Python surface reads as a list
    /// and the planner goldens pin.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

/// The statistics a plan was planned against, for every subject planning
/// consulted.
///
/// Statistics are an explicit input to planning, not something the planner
/// reaches into a store for; the emitted plan records what it assumed so a
/// replay against moved statistics is a detectable condition rather than a
/// silent replan. "What it assumed" is defined rather than implied: an entry
/// exists for every subject planning consulted, and each statistic is absent
/// when the provider reported none — so a subject the provider was silent about
/// is still named, and a subject nothing consulted is absent rather than
/// recorded as empty.
///
/// The strata are here too, and so are their statistics in
/// [`Plan::stratum_derivations`], because the two answer different questions. A
/// derivation binds a stratum's statistics to the one depth they produced, which
/// is what makes that depth recomputable. This names, in one place, every subject
/// an answer to this plan depended on a provider for, whether or not a number
/// came of it — which is what makes "the statistics moved" a question a caller
/// can ask of the snapshot alone. A stratum's entry here is a projection of its
/// derivation rather than a second consultation, and [`Plan::certify`] refuses a
/// plan whose two records of one stratum disagree.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatisticsSnapshot {
    /// A caller-supplied label for the statistics provider.
    pub source: String,
    /// The provider-declared revision of the snapshot.
    pub revision: String,
    /// The snapshot's entries, ascending by subject.
    ///
    /// A [`StatisticsEntries`] rather than a `Vec`, so that order is a property
    /// of the value and not of the encoder that writes it: the bytes — and
    /// therefore the plan's identity — are a pure function of the entries, and
    /// so is equality, which is what makes [`Plan`]'s biconditional true rather
    /// than nearly true. A snapshot naming one subject twice is refused at
    /// construction rather than ordered into an arbitrary winner, because two
    /// rows for one subject are two answers to one question.
    pub entries: StatisticsEntries,
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
    /// How much of the answer the request asked for, and therefore the bound
    /// [`Self::stratum_depths`] was derived under.
    ///
    /// It is content rather than provenance, and therefore part of [`Self::id`],
    /// because a plan for five rows and a plan for five hundred are different
    /// plans: under a request whose strata declare disjoint candidate blocks
    /// they record different depths, emit different `LIMIT`s and read different
    /// numbers of rows. Two plans that agree on everything else and disagree
    /// here must not share an identity, or a caller comparing identities would
    /// be told two different reads were the same one.
    ///
    /// It is also what lets a bound mismatch be caught rather than guessed at.
    /// The depths below are honest for *this* bound; fusing the compiled bundle
    /// at another is refused by name
    /// ([`FusionError::ReadBoundMismatch`](crate::FusionError::ReadBoundMismatch)),
    /// because a depth derived for five rows cannot serve five hundred and the
    /// recorded depth would be describing a read nobody asked for.
    pub read_bound: ReadBound,
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
    /// # What an empty list means
    ///
    /// That every request term reached a producer **with its content**: some
    /// binding carries the term, and the alternative that matched it renders at
    /// least one of its facets into an argument position, so the emitted query
    /// text contains it. A producer that matches a term and declares no
    /// placement for it is not serving that term — it is being called with the
    /// term absent from its arguments — and the term appears here as
    /// [`UnservedReason::AcceptedWithoutPlacement`] rather than in that
    /// producer's [`ProducerBinding::request_terms`].
    ///
    /// Read the evidence through [`Self::unserved_evidence`] rather than from
    /// this field: an edited plan's bindings and this list can disagree, and the
    /// accessor reports what the plan in hand actually supports.
    pub unserved_terms: Vec<UnservedTerm>,
    /// Per-stratum maximum depth, keyed by stratum label.
    pub stratum_depths: HashMap<Iri, u32>,
    /// What each of those depths was derived from, keyed by the same labels.
    ///
    /// One entry per stratum in [`Self::stratum_depths`], carrying every input
    /// [`depth_from`](crate::depth_from) consumed to produce it. This is what
    /// makes a recorded depth a checkable claim rather than an asserted one:
    /// [`Self::certify`] recomputes each depth from the inputs beside it and
    /// refuses a plan the two disagree about.
    ///
    /// A [`BTreeMap`](std::collections::BTreeMap) rather than a
    /// [`HashMap`](std::collections::HashMap) so the ordering law lives in the
    /// type instead of in a sort the encoder has to remember — the depths above
    /// predate that reasoning and are sorted at the encoder instead.
    pub stratum_derivations: BTreeMap<Iri, DepthInputs>,
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
            && self.read_bound == other.read_bound
            && self.producer_bindings == other.producer_bindings
            && self.producer_decisions == other.producer_decisions
            && self.unserved_terms == other.unserved_terms
            && self.stratum_depths == other.stratum_depths
            && self.stratum_derivations == other.stratum_derivations
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
        write_derivations(&mut writer, &self.stratum_derivations);
        write_statistics(&mut writer, &self.statistics_snapshot);
        writer.u64(self.registry_instance_id.as_u64());
        writer.string(&self.registry_content_fingerprint);
        write_unserved_terms(&mut writer, &self.unserved_terms);
        write_read_bound(&mut writer, self.read_bound);
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
        let stratum_derivations = read_derivations(&mut reader)?;
        let statistics_snapshot = read_statistics(&mut reader)?;
        let registry_instance_id = RegistryId::from_raw(reader.u64()?);
        let registry_content_fingerprint = reader.string("registry content fingerprint")?;
        let unserved_terms = read_unserved_terms(&mut reader)?;
        let read_bound = read_read_bound(&mut reader)?;
        reader.finish()?;
        // Reconstructed from bytes, so the instance id just read is a counter
        // value this process cannot have minted; the plan is held to its
        // content fingerprint instead. Recorded here rather than inferred at
        // admission, which cannot tell a foreign counter from a stale one.
        Ok(Self {
            origin: PlanOrigin::Deserialized,
            version,
            request_terms,
            read_bound,
            producer_bindings,
            producer_decisions,
            unserved_terms,
            stratum_depths,
            stratum_derivations,
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

    /// Recompute every recorded depth from its own recorded inputs, and refuse a
    /// plan the two disagree about.
    ///
    /// A plan is untrusted input: it can be edited, and it can be forged. Its
    /// depths are the numbers that decide how deep each stratum is actually read,
    /// so a depth nothing checks is a number a caller must take on the plan's
    /// word. Because [`Self::stratum_derivations`] records every input
    /// [`depth_from`](crate::depth_from) consumes, that word is checkable: this
    /// runs the planner's own arithmetic over the plan's own evidence and
    /// compares.
    ///
    /// # This is the cold path
    ///
    /// Admission does **not** call this, deliberately. Admitting a plan is on the
    /// hot path of every read, and it answers a different question — is this plan
    /// still valid against the registry and statistics in force *now*. Certifying
    /// asks whether the plan is internally coherent *at all*, which is a property
    /// of the value alone and does not change between admissions. Splitting them
    /// keeps the per-read cost where it was and leaves the check available to
    /// anyone receiving a plan from somewhere they do not control.
    ///
    /// # The two records of one stratum must agree
    ///
    /// A stratum's statistics are recorded twice — in [`Self::stratum_derivations`],
    /// bound to the depth they produced, and in [`Self::statistics_snapshot`],
    /// which names every subject planning consulted. The planner writes the
    /// second as a projection of the first, so they agree by construction in any
    /// plan it emitted; a plan in which they differ was edited or forged, and its
    /// two readings license different depths with nothing saying which is the
    /// measurement. Both directions are checked here, so neither record can
    /// quietly become the other's contradiction.
    ///
    /// # Which stratum a refusal names is a function of the plan
    ///
    /// Every loop below walks a sorted key list or a
    /// [`BTreeMap`](std::collections::BTreeMap), never [`Self::stratum_depths`]
    /// in its own iteration order. A `HashMap` walk would make the *first*
    /// disagreement a plan carries several of depend on hash order, so two
    /// processes refusing one forged plan could name two different strata and a
    /// caller comparing the messages would think the plans differed.
    ///
    /// # Errors
    ///
    /// [`PlanError::DepthNotDerivable`] when a recorded depth is not the depth
    /// its inputs derive; [`PlanError::DepthWithoutDerivation`] and
    /// [`PlanError::DerivationWithoutDepth`] when the depth and derivation maps
    /// do not name the same strata;
    /// [`PlanError::DerivationWithoutStatisticsEntry`] when the snapshot does not
    /// name a stratum a depth was derived for; and
    /// [`PlanError::StatisticsEntryContradictsDerivation`] when it names one and
    /// says something else about it. Each is a refusal rather than a repair: a
    /// plan whose depth and evidence disagree has no reading under which one of
    /// them is the truth.
    pub fn certify(&self) -> Result<(), PlanError> {
        let mut recorded: Vec<(&Iri, u32)> = self
            .stratum_depths
            .iter()
            .map(|(stratum, depth)| (stratum, *depth))
            .collect();
        recorded.sort_unstable_by(|left, right| left.0.cmp(right.0));
        for (stratum, depth) in recorded {
            let Some(inputs) = self.stratum_derivations.get(stratum) else {
                return Err(PlanError::DepthWithoutDerivation {
                    stratum: stratum.as_str().to_owned(),
                });
            };
            let derived = crate::depth_from(inputs);
            if derived != depth {
                return Err(PlanError::DepthNotDerivable {
                    stratum: stratum.as_str().to_owned(),
                    recorded: depth,
                    derived,
                });
            }
        }
        for (stratum, inputs) in &self.stratum_derivations {
            if !self.stratum_depths.contains_key(stratum) {
                return Err(PlanError::DerivationWithoutDepth {
                    stratum: stratum.as_str().to_owned(),
                });
            }
            reconcile(stratum, inputs, &self.statistics_snapshot)?;
        }
        Ok(())
    }

    /// Which recorded input bound `stratum`'s depth, or `None` when the plan
    /// records no derivation for it.
    ///
    /// A depth of one is the motivating case. It arrives by four different roads
    /// — a declaration of zero or one row, a measured cardinality, a selectivity
    /// that scaled the bound down to nothing, or the floor that stops any of them
    /// reaching zero — and a caller looking at the number alone cannot tell which,
    /// even though the four have completely different remedies. Recording the
    /// inputs makes the question a total function of the plan.
    #[must_use]
    pub fn explain_depth(&self, stratum: &Iri) -> Option<DepthCause> {
        self.stratum_derivations
            .get(stratum)
            .map(crate::depth_cause)
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

/// The canonical discriminator byte a vector term's metric is written as.
///
/// The byte lands in a plan's identity, so the mapping is fixed forever: a tag
/// once issued keeps its meaning in every build that reads it, and a new metric
/// takes a new byte rather than renumbering the existing ones.
fn metric_tag(metric: Metric) -> u8 {
    match metric {
        Metric::Cosine => METRIC_COSINE,
        Metric::Dot => METRIC_DOT,
        Metric::Euclidean => METRIC_EUCLIDEAN,
    }
}

/// The metric a canonical discriminator byte names.
///
/// An unrecognised byte is a typed refusal, never a nearest-known metric. A
/// plan written by a build that knows a metric this one does not is asking a
/// question this build cannot answer, and answering it under a substituted
/// metric would return a confidently wrong result for a plan whose identity the
/// caller still recognises.
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

/// The canonical discriminator byte a producer's rejection reason is written as.
fn reason_tag(reason: RejectionReason) -> u8 {
    match reason {
        RejectionReason::NotRanked => REJECT_NOT_RANKED,
        RejectionReason::NoAcceptedTerm => REJECT_NO_ACCEPTED_TERM,
        RejectionReason::DepthExceeded => REJECT_DEPTH_EXCEEDED,
        RejectionReason::UnsatisfiedConstraint => REJECT_UNSATISFIED_CONSTRAINT,
    }
}

/// The rejection reason a canonical discriminator byte names.
///
/// Refused rather than defaulted, for the reason [`metric_from_tag`] is: the
/// reason a producer was dropped is the evidence a caller reads to find out why
/// its answer is narrower than it expected, and a substituted reason is worse
/// than no plan at all.
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

/// The canonical discriminator byte an unserved term's reason is written as.
fn unserved_tag(reason: UnservedReason) -> u8 {
    match reason {
        UnservedReason::NoProducerAccepts => UNSERVED_NO_PRODUCER_ACCEPTS,
        UnservedReason::EveryAcceptingProducerRejected => {
            UNSERVED_EVERY_ACCEPTING_PRODUCER_REJECTED
        }
        UnservedReason::AcceptedWithoutPlacement => UNSERVED_ACCEPTED_WITHOUT_PLACEMENT,
        UnservedReason::Unbound => UNSERVED_UNBOUND,
    }
}

/// The unserved-term reason a canonical discriminator byte names.
///
/// Refused rather than defaulted. The four reasons are distinguishable facts
/// about the registry — nothing accepted the term, something accepted it and
/// declared nowhere to put it, everything that accepted it was then rejected, or
/// the plan was edited — and collapsing one into another is exactly the loss
/// [`Plan::unserved_terms`] exists to prevent.
fn unserved_from_tag(tag: u8) -> Result<UnservedReason, PlanError> {
    match tag {
        UNSERVED_NO_PRODUCER_ACCEPTS => Ok(UnservedReason::NoProducerAccepts),
        UNSERVED_EVERY_ACCEPTING_PRODUCER_REJECTED => {
            Ok(UnservedReason::EveryAcceptingProducerRejected)
        }
        UNSERVED_ACCEPTED_WITHOUT_PLACEMENT => Ok(UnservedReason::AcceptedWithoutPlacement),
        UNSERVED_UNBOUND => Ok(UnservedReason::Unbound),
        tag => Err(PlanError::InvalidTag {
            what: "unserved term reason",
            tag,
        }),
    }
}

/// Write the per-term unserved evidence, length-framed and in list order.
///
/// The list order is the plan's own and is not re-sorted here: the planner
/// emits it ascending by request-term index, and the encoding reproduces the
/// field rather than imposing an order the value does not have.
fn write_unserved_terms(writer: &mut Writer, terms: &[UnservedTerm]) {
    writer.u64(terms.len() as u64);
    for term in terms {
        writer.u32(term.request_term);
        writer.u8(unserved_tag(term.reason));
    }
}

/// Read the per-term unserved evidence.
///
/// The pre-allocation is capped rather than taken from the framed count: the
/// count is untrusted input, and reserving what a forged length asks for would
/// let a short, malformed plan demand an arbitrary allocation before a single
/// element is read. Every decoder in this module caps the same way.
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

/// Write the request's read bound: its arm's discriminator byte, then the row
/// count the bounded arm carries.
///
/// The count goes out as a `u64` rather than as the `usize` [`TopK`] holds, so
/// the bytes — and therefore the identity digested from them — are the same on a
/// 32-bit and a 64-bit target. A count wider than a `u64` is not expressible in a `usize`
/// on any target this workspace builds for, so the conversion cannot lose one.
fn write_read_bound(writer: &mut Writer, bound: ReadBound) {
    match bound {
        ReadBound::Complete => writer.u8(BOUND_COMPLETE),
        ReadBound::Bounded(top_k) => {
            writer.u8(BOUND_BOUNDED);
            writer.u64(top_k.get() as u64);
        }
    }
}

/// Read the read bound written by [`write_read_bound`].
///
/// An unknown arm byte is a refusal for the reason every other tag in this module
/// is refused: the two arms license different depths, so reading one as the other
/// would admit a plan whose recorded depths were derived under a bound this build
/// then ignored.
///
/// A count a `usize` cannot hold is refused rather than clamped. Clamping it
/// would answer a narrower question than the plan asked while still reporting the
/// plan's own identity, and a bound is the one field where "smaller" is not a
/// safe direction — it is the number the recorded depths were derived from.
fn read_read_bound(reader: &mut Reader<'_>) -> Result<ReadBound, PlanError> {
    match reader.u8()? {
        BOUND_COMPLETE => Ok(ReadBound::Complete),
        BOUND_BOUNDED => {
            let rows = reader.u64()?;
            let rows = usize::try_from(rows).map_err(|_| PlanError::InvalidTag {
                what: "read bound row count wider than this target's usize",
                tag: BOUND_BOUNDED,
            })?;
            Ok(ReadBound::Bounded(TopK::new(rows)))
        }
        tag => Err(PlanError::InvalidTag {
            what: "read bound",
            tag,
        }),
    }
}

/// Write an IRI as its length-framed canonical text.
fn write_iri(writer: &mut Writer, iri: &Iri) {
    writer.string(iri.as_str());
}

/// Read an IRI, re-parsing it with the kernel's own parser.
///
/// The text is validated on the way in rather than trusted because it came out
/// of a plan: a plan is untrusted input, and an [`Iri`] that skipped validation
/// would carry span offsets that do not describe its own bytes. `what` names
/// the field, so a refusal says which IRI was malformed.
fn read_iri(reader: &mut Reader<'_>, what: &'static str) -> Result<Iri, PlanError> {
    let text = reader.string(what)?;
    Iri::parse(&text)
}

/// Write an optional exact value as a presence byte and, when present, its raw
/// scaled integer.
///
/// The raw `i128` is written rather than a rendered decimal: it is the value's
/// exact representation, so the encoding neither rounds nor depends on a
/// formatter.
fn write_option_fixed(writer: &mut Writer, value: Option<Fixed>) {
    match value {
        None => writer.u8(ABSENT),
        Some(value) => {
            writer.u8(PRESENT);
            writer.i128(value.into_raw());
        }
    }
}

/// Read an optional exact value written by [`write_option_fixed`].
///
/// A presence byte that is neither absent nor present is a refusal rather than
/// a treated-as-absent field, because silently reading a constrained endpoint
/// as unconstrained would widen the query the plan describes.
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

/// Write one request term: its arm's discriminator byte, then that arm's fields
/// in declaration order.
///
/// Every arm writes a fixed field sequence, so the encoding is a pure function
/// of the term. A vector term's components go out as exact bit patterns
/// ([`Writer::f32_bits`]), which is the same identity `RequestTerm`'s hand-written
/// `PartialEq` compares by — so two terms are equal exactly when their encodings
/// are, including the `0.0`/`-0.0` and `NaN` cases a float comparison would get
/// wrong in both directions.
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
        RequestTerm::Temporal {
            predicate,
            lower,
            upper,
        } => {
            writer.u8(TERM_TEMPORAL);
            write_iri(writer, predicate);
            writer.option_string(lower.as_deref());
            writer.option_string(upper.as_deref());
        }
        RequestTerm::NumericRange {
            predicate,
            lower,
            upper,
        } => {
            writer.u8(TERM_NUMERIC_RANGE);
            write_iri(writer, predicate);
            write_option_fixed(writer, *lower);
            write_option_fixed(writer, *upper);
        }
        RequestTerm::EntitySeed { entity } => {
            writer.u8(TERM_ENTITY_SEED);
            writer.string(entity.as_str());
        }
    }
}

/// Read one request term written by [`write_request_term`].
///
/// An unknown arm byte is a refusal: the request lattice is closed and grows by
/// addition, so a term this build cannot name is one a newer build wrote, and
/// planning it as some other arm would answer a different question.
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
        TERM_TEMPORAL => {
            let predicate = read_iri(reader, "temporal predicate")?;
            let lower = reader.option_string("temporal lower endpoint")?;
            let upper = reader.option_string("temporal upper endpoint")?;
            Ok(RequestTerm::Temporal {
                predicate,
                lower,
                upper,
            })
        }
        TERM_NUMERIC_RANGE => {
            let predicate = read_iri(reader, "numeric range predicate")?;
            let lower = read_option_fixed(reader)?;
            let upper = read_option_fixed(reader)?;
            Ok(RequestTerm::NumericRange {
                predicate,
                lower,
                upper,
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

/// Write the request's terms, length-framed, in the caller's own order.
///
/// The order is identity-bearing — producer bindings index into it — so it is
/// never sorted.
fn write_request_terms(writer: &mut Writer, terms: &[RequestTerm]) {
    writer.u64(terms.len() as u64);
    for term in terms {
        write_request_term(writer, term);
    }
}

/// Read the request's terms, preserving the encoded order.
fn read_request_terms(reader: &mut Reader<'_>) -> Result<Vec<RequestTerm>, PlanError> {
    let count = reader.count()?;
    let mut terms = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        terms.push(read_request_term(reader)?);
    }
    Ok(terms)
}

/// Write the producer bindings: per binding, the producer IRI, its stratum, and
/// the request-term indices it receives.
///
/// The producer is written as a plain framed string rather than through
/// [`write_iri`] because a plan carries the registry's key byte-exactly — what
/// the host registered under, not a re-canonicalized spelling of it.
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

/// Read the producer bindings written by [`write_bindings`].
///
/// The indices are read as-is. Whether they address the plan's own request is
/// not this decoder's question — it is a semantic claim, checked once at the
/// admission waist against the registry the plan will actually run on.
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

/// Write every considered producer's decision, selected or rejected with its
/// reason.
///
/// Rejections are encoded beside selections rather than dropped: they are the
/// plan's account of why the answer is the shape it is, and an encoding that
/// kept only the selections would make two different plans hash alike.
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

/// Read the producer decisions written by [`write_decisions`].
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

/// Write the per-stratum depths, sorted by stratum IRI.
///
/// This sort is where determinism is won. The field is a `HashMap`, whose
/// iteration order is not a function of its contents, so encoding it in
/// iteration order would give one plan many identities. Sorting by the
/// stratum's canonical text makes the bytes — and therefore the plan id — a
/// pure function of the entries, on every target and in every process.
fn write_depths(writer: &mut Writer, depths: &HashMap<Iri, u32>) {
    let mut entries: Vec<(&Iri, u32)> = depths.iter().map(|(key, value)| (key, *value)).collect();
    entries.sort_by(|left, right| left.0.as_str().cmp(right.0.as_str()));
    writer.u64(entries.len() as u64);
    for (key, value) in entries {
        write_iri(writer, key);
        writer.u32(value);
    }
}

/// Read the per-stratum depths. The encoded order is sorted; the map that comes
/// back does not preserve it and does not need to, because the next encode sorts
/// again.
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

/// Refuse a stratum whose snapshot row says anything other than its derivation.
///
/// The planner writes the row as a projection of `inputs`, so the two are equal
/// in any plan it emitted and this reads as a no-op there. It earns its keep on
/// the plans [`Plan::certify`] exists for: edited ones, forged ones, and ones
/// written by a build whose projection differed.
///
/// The dimensions are tested in the order [`depth_from`](crate::depth_from)
/// consumes them, so a plan disagreeing about several is named by the one
/// nearest the arithmetic — and the order is fixed rather than incidental, which
/// is what makes the reported dimension a function of the plan.
///
/// A missing row is a different fact from a wrong one and is refused separately:
/// one is a snapshot that forgot a subject, the other a snapshot that
/// contradicts one.
fn reconcile(
    stratum: &Iri,
    inputs: &DepthInputs,
    snapshot: &StatisticsSnapshot,
) -> Result<(), PlanError> {
    let Some(entry) = snapshot.entries.get_subject(stratum.as_str()) else {
        return Err(PlanError::DerivationWithoutStatisticsEntry {
            stratum: stratum.as_str().to_owned(),
        });
    };
    let disagreement = if entry.cardinality != inputs.cardinality {
        Some((
            StatisticsDimension::Cardinality,
            measurement(entry.cardinality),
            measurement(inputs.cardinality),
        ))
    } else if entry.selectivity_ppm != inputs.selectivity_ppm {
        Some((
            StatisticsDimension::SelectivityPpm,
            measurement(entry.selectivity_ppm),
            measurement(inputs.selectivity_ppm),
        ))
    } else if entry.selectivity_terms != inputs.selectivity_terms {
        Some((
            StatisticsDimension::SelectivityTerms,
            format!("{:?}", entry.selectivity_terms),
            format!("{:?}", inputs.selectivity_terms),
        ))
    } else {
        None
    };
    match disagreement {
        Some((dimension, snapshot, derivation)) => {
            Err(PlanError::StatisticsEntryContradictsDerivation {
                stratum: stratum.as_str().to_owned(),
                dimension,
                snapshot,
                derivation,
            })
        }
        None => Ok(()),
    }
}

/// One reported statistic, rendered for a refusal message.
///
/// An absent measurement renders as the word rather than as a number, because
/// "the provider measured nothing" and "the provider measured zero" are the two
/// facts this whole record exists to keep apart — and a message that printed
/// `0` for both would collapse them at the one moment a reader is trying to tell
/// which of them moved.
fn measurement(value: Option<u64>) -> String {
    value.map_or_else(|| "absent".to_owned(), |value| value.to_string())
}

/// Write the per-stratum derivations, sorted by stratum IRI.
///
/// The map is a [`BTreeMap`], so the sort is already the type's; the count and
/// the field order are what the encoding adds. Each record is the argument list
/// [`depth_from`](crate::depth_from) consumed, written in the order that
/// function reads it, so the bytes and the derivation tell the same story in the
/// same sequence.
fn write_derivations(writer: &mut Writer, derivations: &BTreeMap<Iri, DepthInputs>) {
    writer.u64(derivations.len() as u64);
    for (stratum, inputs) in derivations {
        write_iri(writer, stratum);
        writer.u64(inputs.declared);
        writer.option_u64(inputs.cardinality);
        writer.option_u64(inputs.selectivity_ppm);
        writer.u32_slice(&inputs.selectivity_terms);
        writer.option_u64(inputs.licensed_prefix);
    }
}

/// Read the per-stratum derivations written by [`write_derivations`].
///
/// A repeated stratum is refused rather than collapsed by the map: two
/// derivations for one stratum are two explanations of one depth, and silently
/// keeping the last would let a forged plan carry an explanation the encoder
/// never wrote.
fn read_derivations(reader: &mut Reader<'_>) -> Result<BTreeMap<Iri, DepthInputs>, PlanError> {
    let count = reader.count()?;
    let mut derivations = BTreeMap::new();
    for _ in 0..count {
        let stratum = read_iri(reader, "stratum derivation key")?;
        let inputs = DepthInputs {
            declared: reader.u64()?,
            cardinality: reader.option_u64("derivation cardinality presence")?,
            selectivity_ppm: reader.option_u64("derivation selectivity presence")?,
            selectivity_terms: reader.u32_slice()?,
            licensed_prefix: reader.option_u64("derivation licensed prefix presence")?,
        };
        if derivations.insert(stratum.clone(), inputs).is_some() {
            return Err(PlanError::DuplicateStatisticsSubject {
                subject: stratum.as_str().to_owned(),
            });
        }
    }
    Ok(derivations)
}

/// Write the statistics snapshot: the provider's label and revision, then each
/// entry's subject, optional cardinality, optional selectivity and that
/// selectivity's term domain.
///
/// **The entries are not sorted here.** They arrive ascending because
/// [`StatisticsEntries`] establishes that on construction, which is where the
/// law belongs: a sort at the encoder makes the *bytes* a pure function of the
/// entries while leaving the *value* order-sensitive, so a descending snapshot
/// and its ascending twin would share an identity and compare unequal. The
/// encoder writes the sequence it is given, exactly as it does for
/// [`write_derivations`]'s [`BTreeMap`].
fn write_statistics(writer: &mut Writer, snapshot: &StatisticsSnapshot) {
    writer.string(&snapshot.source);
    writer.string(&snapshot.revision);
    writer.u64(snapshot.entries.len() as u64);
    for entry in &snapshot.entries {
        writer.string(&entry.subject);
        writer.option_u64(entry.cardinality);
        writer.option_u64(entry.selectivity_ppm);
        writer.u32_slice(&entry.selectivity_terms);
    }
}

/// Read the statistics snapshot written by [`write_statistics`].
///
/// An absent value stays absent: "the provider measured nothing" and "the
/// provider measured zero" are different facts, and a decoder that read the
/// first as the second would turn silence into a claim that no row matches.
/// That now holds for the cardinality as well as the selectivity — the two are
/// one rule, written once.
///
/// A repeated subject is refused by name, by the same construction law a caller
/// with a `Vec` is held to: two rows for one subject are two answers to one
/// question, and the decoder has no more basis for picking between them than the
/// constructor does.
fn read_statistics(reader: &mut Reader<'_>) -> Result<StatisticsSnapshot, PlanError> {
    let source = reader.string("statistics source")?;
    let revision = reader.string("statistics revision")?;
    let count = reader.count()?;
    let mut entries: Vec<StatisticsEntry> = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        entries.push(StatisticsEntry {
            subject: reader.string("statistics subject")?,
            cardinality: reader.option_u64("statistics cardinality presence")?,
            selectivity_ppm: reader.option_u64("statistics selectivity presence")?,
            selectivity_terms: reader.u32_slice()?,
        });
    }
    Ok(StatisticsSnapshot {
        source,
        revision,
        entries: StatisticsEntries::new(entries)?,
    })
}
