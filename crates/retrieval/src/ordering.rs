// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-authored ordering data and bounds for certified fusion.
//!
//! Keys compare as unsigned bytes, lexicographically, with a shorter prefix
//! first. Callers normalize composite fields into that order (for example tier
//! ascending followed by activation descending). The semantic IRI identifies
//! that encoding; no comparator or vocabulary is invented by the engine.

use core::cmp::Ordering;
use std::collections::BTreeMap;

use crate::canonical::{CanonicalSink, Writer};
use crate::{Iri, Score, Term};

/// Immutable key bytes retaining their original native allocation authority.
pub type OrderBytes = purrdf_sparql_eval::SharedWorkspace<purrdf_sparql_eval::AdmittedVec<u8>>;

pub(crate) fn retain_bytes(
    bytes: &[u8],
    workspace: &purrdf_sparql_eval::WorkspaceCapability,
) -> Result<OrderBytes, purrdf_sparql_eval::EvalError> {
    let bytes =
        purrdf_sparql_eval::AdmittedVec::from_iter_admitted(bytes.iter().copied(), workspace)?;
    purrdf_sparql_eval::SharedWorkspace::new_admitted(bytes, workspace)
}

/// The caller's declared interpretation of normalized ordering bytes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrderingPolicy {
    /// Caller-owned identity of the key layout and comparison semantics.
    pub semantics: Iri,
    /// Whether a leading key is compared before the score.
    pub leading: bool,
}

/// A candidate's immutable keys, shared by all strata that name it.
#[derive(Clone, Debug)]
pub struct CandidateKeys {
    /// Compared before score when the policy declares leading keys.
    pub leading: OrderBytes,
    /// Compared immediately after score, before the canonical term.
    pub tie: OrderBytes,
}

impl CandidateKeys {
    /// Retain explicit caller bytes through the original admitted storage.
    /// # Errors
    /// Returns the original allocation, layout or admission refusal.
    pub fn new(
        leading: &[u8],
        tie: &[u8],
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Self, purrdf_sparql_eval::EvalError> {
        Ok(Self {
            leading: retain_bytes(leading, workspace)?,
            tie: retain_bytes(tie, workspace)?,
        })
    }

    /// Retain these bytes under the receiving execution's allocation authority.
    /// # Errors
    /// Returns the original allocation, layout or admission refusal.
    pub fn clone_admitted(
        &self,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Self, purrdf_sparql_eval::EvalError> {
        Self::new(&self.leading, &self.tie, workspace)
    }
}
impl PartialEq for CandidateKeys {
    fn eq(&self, other: &Self) -> bool {
        self.leading.as_slice() == other.leading.as_slice()
            && self.tie.as_slice() == other.tie.as_slice()
    }
}
impl Eq for CandidateKeys {}

/// A lower bound on the best normalized leading key a source may still name.
///
/// `Inclusive(k)` promises every covered key is at least `k`. `Empty` promises
/// there is no covered candidate. Neither may be inferred from a lossy read's
/// exhaustion when bounding its full domain.
#[derive(Clone, Debug)]
pub enum LeadingBound {
    /// The source has supplied no useful bound.
    Unbounded,
    /// Every covered candidate has a leading key at least this value.
    Inclusive(OrderBytes),
    /// The covered population is empty.
    Empty,
}

impl LeadingBound {
    pub(crate) fn cmp_best(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Unbounded, Self::Unbounded) | (Self::Empty, Self::Empty) => Ordering::Equal,
            (Self::Unbounded, _) | (_, Self::Empty) => Ordering::Less,
            (Self::Empty, _) | (_, Self::Unbounded) => Ordering::Greater,
            (Self::Inclusive(left), Self::Inclusive(right)) => {
                left.as_slice().cmp(right.as_slice())
            }
        }
    }

    pub(crate) fn union(self, other: Self) -> Self {
        if self.cmp_best(&other).is_le() {
            self
        } else {
            other
        }
    }

    pub(crate) fn permits(&self, key: &[u8]) -> bool {
        match self {
            Self::Unbounded => true,
            Self::Inclusive(minimum) => key >= minimum.as_slice(),
            Self::Empty => false,
        }
    }

    pub(crate) fn write<B: CanonicalSink>(&self, writer: &mut Writer<B>) {
        match self {
            Self::Unbounded => writer.u8(0),
            Self::Inclusive(key) => {
                writer.u8(1);
                writer.bytes(key);
            }
            Self::Empty => writer.u8(2),
        }
    }

    pub(crate) fn clone_admitted(
        &self,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Self, purrdf_sparql_eval::EvalError> {
        Ok(match self {
            Self::Unbounded => Self::Unbounded,
            Self::Empty => Self::Empty,
            Self::Inclusive(key) => Self::Inclusive(retain_bytes(key, workspace)?),
        })
    }
}
impl PartialEq for LeadingBound {
    fn eq(&self, other: &Self) -> bool {
        self.cmp_best(other).is_eq()
    }
}
impl Eq for LeadingBound {}

/// A pinned key-data snapshot and a bound over the source's entire domain.
///
/// The full-domain bound includes candidates omitted by an incomplete index,
/// refused read or lossy search. It is distinct from the moving suffix bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderingEvidence {
    /// Must equal the semantic identity in the fusion law.
    pub semantics: Iri,
    /// Caller-supplied content identity of the actual key-data snapshot.
    pub snapshot: [u8; 32],
    /// Bound over the whole source domain, including missing candidates.
    pub domain: LeadingBound,
}
impl OrderingEvidence {
    pub(crate) fn clone_admitted(
        &self,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Self, purrdf_sparql_eval::EvalError> {
        Ok(Self {
            semantics: self.semantics.clone_admitted(workspace)?,
            snapshot: self.snapshot,
            domain: self.domain.clone_admitted(workspace)?,
        })
    }
}

/// One source's pinned ordering evidence and current unread-suffix bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderingStatus {
    /// Identity and full-domain bound pinned before the first pull.
    pub evidence: OrderingEvidence,
    /// Bound after including the engine's buffered head.
    pub unread: LeadingBound,
}

/// An optimistic full-order bound on a population of candidates.
#[derive(Clone, Debug)]
pub struct OrderBound {
    /// Best leading key the population may contain.
    pub leading: LeadingBound,
    /// Highest score it may contain; absence means no finite score bound.
    pub score: Option<Score>,
    /// Exact tail keys for a known rival; absent for an unseen population.
    pub tail: Option<(OrderBytes, Term)>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LeadingRef<'a> {
    Unbounded,
    Inclusive(&'a [u8]),
    Empty,
}

/// Borrowed optimistic order, used by settlement without acquiring storage.
#[derive(Clone, Copy)]
pub(crate) struct OrderBoundView<'a> {
    leading: LeadingRef<'a>,
    score: Option<Score>,
    tail: Option<(&'a [u8], &'a Term)>,
}

impl OrderBound {
    /// Whether the exact candidate precedes every member this bound permits.
    /// # Errors
    /// Refuses score units different from the bound's units.
    pub fn is_after(
        &self,
        keys: &CandidateKeys,
        score: Score,
        term: &Term,
    ) -> Result<bool, crate::FusionError> {
        self.view().is_after(keys, score, term)
    }

    pub(crate) fn view(&self) -> OrderBoundView<'_> {
        OrderBoundView {
            leading: match &self.leading {
                LeadingBound::Unbounded => LeadingRef::Unbounded,
                LeadingBound::Inclusive(key) => LeadingRef::Inclusive(key),
                LeadingBound::Empty => LeadingRef::Empty,
            },
            score: self.score,
            tail: self.tail.as_ref().map(|(tie, term)| (tie.as_slice(), term)),
        }
    }

    pub(crate) fn cmp_best(&self, other: &Self) -> Ordering {
        self.view().cmp_best(other.view())
    }

    pub(crate) fn union(self, other: Self) -> Self {
        if self.cmp_best(&other).is_le() {
            self
        } else {
            other
        }
    }
}

impl<'a> OrderBoundView<'a> {
    pub(crate) fn known(keys: &'a CandidateKeys, score: Option<Score>, term: &'a Term) -> Self {
        Self {
            leading: LeadingRef::Inclusive(&keys.leading),
            score,
            tail: Some((&keys.tie, term)),
        }
    }

    pub(crate) fn is_after(
        self,
        keys: &CandidateKeys,
        score: Score,
        term: &Term,
    ) -> Result<bool, crate::FusionError> {
        if let Some(ceiling) = self.score {
            score.require_precision(ceiling.precision())?;
        }
        Ok(self.is_after_same_units(keys, score, term))
    }

    fn is_after_same_units(&self, keys: &CandidateKeys, score: Score, term: &Term) -> bool {
        match self.leading {
            LeadingRef::Empty => return true,
            LeadingRef::Unbounded => return false,
            LeadingRef::Inclusive(minimum) => match keys.leading.as_slice().cmp(minimum) {
                Ordering::Less => return true,
                Ordering::Greater => return false,
                Ordering::Equal => {}
            },
        }
        let Some(ceiling) = self.score else {
            return false;
        };
        match score.partial_cmp(&ceiling) {
            Some(Ordering::Greater) => true,
            Some(Ordering::Equal) => self.tail.is_some_and(|(tie, other)| {
                keys.tie
                    .as_slice()
                    .cmp(tie)
                    .then_with(|| term.cmp(other))
                    .is_lt()
            }),
            _ => false,
        }
    }

    fn cmp_best(self, other: Self) -> Ordering {
        self.leading
            .cmp(&other.leading)
            .then_with(|| match (self.score, other.score) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Less,
                (Some(_), None) => Ordering::Greater,
                (Some(left), Some(right)) => right.into_raw().cmp(&left.into_raw()),
            })
            .then_with(|| match (self.tail, other.tail) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Less,
                (Some(_), None) => Ordering::Greater,
                (Some((left_key, left_term)), Some((right_key, right_term))) => left_key
                    .cmp(right_key)
                    .then_with(|| left_term.cmp(right_term)),
            })
    }

    pub(crate) fn union(self, other: Self) -> Self {
        if self.cmp_best(other).is_le() {
            self
        } else {
            other
        }
    }
}
impl PartialEq for OrderBound {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.cmp_best(other).is_eq()
    }
}
impl Eq for OrderBound {}

/// Replayable evidence of the full-order unseen gate at row certification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderWitness {
    /// The law's caller-owned key semantics.
    pub semantics: Iri,
    /// This candidate's fixed keys.
    pub keys: CandidateKeys,
    /// Optimistic bound over every candidate not yet observed.
    pub unseen: OrderBound,
}

/// Typed ordering-protocol refusals. No missing key is replaced by a fallback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrderingError {
    /// A keyed law received a candidate without a key.
    MissingKey {
        /// Canonical candidate lexical.
        item: String,
    },
    /// A candidate's keys changed in a later observation.
    ConflictingKey {
        /// Canonical candidate lexical.
        item: String,
    },
    /// A score-only policy received a nonempty leading key.
    UnexpectedLeadingKey {
        /// Canonical candidate lexical.
        item: String,
    },
    /// A keyed source omitted its snapshot evidence.
    MissingEvidence {
        /// Source stratum IRI.
        stratum: String,
    },
    /// A source's semantic identity differs from the law.
    PolicyMismatch {
        /// Source stratum IRI or ingress name.
        stratum: String,
    },
    /// A returned row contradicts a prior bound.
    BoundViolated {
        /// Source stratum IRI.
        stratum: String,
    },
    /// A source widened a suffix bound after it had been used.
    BoundLoosened {
        /// Source stratum IRI.
        stratum: String,
    },
    /// A source changed the pinned key-data snapshot.
    EvidenceChanged {
        /// Source stratum IRI.
        stratum: String,
    },
    /// The available source bounds cannot certify the remaining answer.
    Uncertified {
        /// Missing proof and its required remedy.
        reason: String,
    },
}

impl core::fmt::Display for OrderingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingKey { item } => write!(f, "caller ordering has no keys for {item}"),
            Self::ConflictingKey { item } => write!(f, "caller ordering keys changed for {item}"),
            Self::UnexpectedLeadingKey { item } => write!(
                f,
                "score-first caller ordering received a leading key for {item}"
            ),
            Self::MissingEvidence { stratum } => write!(
                f,
                "caller ordering source <{stratum}> supplied no key snapshot evidence"
            ),
            Self::PolicyMismatch { stratum } => write!(
                f,
                "caller ordering source <{stratum}> declares different key semantics from the fusion law"
            ),
            Self::BoundViolated { stratum } => write!(
                f,
                "caller ordering source <{stratum}> returned a key before its declared bound"
            ),
            Self::BoundLoosened { stratum } => write!(
                f,
                "caller ordering source <{stratum}> widened its unread key bound"
            ),
            Self::EvidenceChanged { stratum } => write!(
                f,
                "caller ordering source <{stratum}> changed its pinned key snapshot"
            ),
            Self::Uncertified { reason } => write!(
                f,
                "caller ordering could not certify the remaining answer: {reason}"
            ),
        }
    }
}

impl std::error::Error for OrderingError {}

impl purrdf_hash::error::StructuredError for OrderingError {
    purrdf_hash::error_visit! {
        MissingKey { item } => "ordering-missing-key" { "item" => Text(item) },
        ConflictingKey { item } => "ordering-conflicting-key" { "item" => Text(item) },
        UnexpectedLeadingKey { item } => "ordering-unexpected-leading-key" { "item" => Text(item) },
        MissingEvidence { stratum } => "ordering-missing-evidence" { "stratum" => Iri(stratum) },
        PolicyMismatch { stratum } => "ordering-policy-mismatch" { "stratum" => Iri(stratum) },
        BoundViolated { stratum } => "ordering-bound-violated" { "stratum" => Iri(stratum) },
        BoundLoosened { stratum } => "ordering-bound-loosened" { "stratum" => Iri(stratum) },
        EvidenceChanged { stratum } => "ordering-evidence-changed" { "stratum" => Iri(stratum) },
        Uncertified { reason } => "ordering-uncertified" { "reason" => Text(reason) },
    }
}

/// An optional concrete key provider for native adapters and language bindings.
///
/// Stream producers may instead implement the typed evidence and bound methods
/// directly against their own index. This catalog is never required by fusion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderCatalog {
    policy: OrderingPolicy,
    entries: BTreeMap<Term, CandidateKeys>,
    evidence: OrderingEvidence,
    by_leading: Vec<Term>,
}

/// Domain of a concrete caller key catalog's content identity.
pub const ORDER_CATALOG_DOMAIN: purrdf_hash::Domain =
    purrdf_hash::Domain::new(b"purrdf:order-catalog:v1");

impl OrderCatalog {
    /// Bind every supplied key to its semantic policy and content identity.
    ///
    /// # Errors
    /// Rejects a leading key under a score-first policy.
    pub fn new(
        policy: OrderingPolicy,
        entries: BTreeMap<Term, CandidateKeys>,
    ) -> Result<Self, crate::FusionError> {
        let mut writer = Writer::new();
        writer.string(policy.semantics.as_str());
        writer.u8(u8::from(policy.leading));
        writer.u64(entries.len() as u64);
        let mut minimum = LeadingBound::Empty;
        for (term, keys) in &entries {
            if !policy.leading && !keys.leading.is_empty() {
                return Err(OrderingError::UnexpectedLeadingKey {
                    item: term.as_str().to_owned(),
                }
                .into());
            }
            writer.string(term.as_str());
            writer.bytes(&keys.leading);
            writer.bytes(&keys.tie);
            minimum = minimum.union(LeadingBound::Inclusive(keys.leading.clone()));
        }
        let mut hasher = purrdf_hash::blake3::RecordHasher::new();
        hasher.update(ORDER_CATALOG_DOMAIN.as_bytes());
        hasher.update(&[0]);
        hasher.update(&writer.into_bytes());
        let mut ordered: Vec<_> = entries.iter().collect();
        ordered.sort_unstable_by(|(left_term, left_keys), (right_term, right_keys)| {
            left_keys
                .leading
                .as_slice()
                .cmp(right_keys.leading.as_slice())
                .then_with(|| left_term.cmp(right_term))
        });
        let by_leading = ordered.into_iter().map(|(term, _)| term.clone()).collect();
        let evidence = OrderingEvidence {
            semantics: policy
                .semantics
                .clone_admitted(&purrdf_sparql_eval::WorkspaceCapability::resident())?,
            snapshot: *hasher.finalize().as_bytes(),
            domain: minimum,
        };
        Ok(Self {
            policy,
            entries,
            evidence,
            by_leading,
        })
    }

    /// The law the catalog's normalized bytes implement.
    #[must_use]
    pub const fn policy(&self) -> &OrderingPolicy {
        &self.policy
    }

    /// The explicit key for a candidate; absence is a refusal.
    pub fn keys(&self, term: &Term) -> Result<&CandidateKeys, OrderingError> {
        self.entries
            .get(term)
            .ok_or_else(|| OrderingError::MissingKey {
                item: term.as_str().to_owned(),
            })
    }

    /// Pinned evidence over the whole catalog, including absent source rows.
    #[must_use]
    pub fn evidence(&self) -> OrderingEvidence {
        self.evidence.clone()
    }

    pub(crate) fn advance_cursor(
        &self,
        yielded: &purrdf_sparql_eval::AdmittedOrderedSet<Term>,
        cursor: &mut usize,
    ) {
        while self
            .by_leading
            .get(*cursor)
            .is_some_and(|term| yielded.contains(term))
        {
            *cursor += 1;
        }
    }

    pub(crate) fn unread_bound(&self, cursor: usize) -> LeadingBound {
        self.by_leading
            .get(cursor)
            .map_or(LeadingBound::Empty, |term| {
                // Every index member is collected from the immutable entry map.
                LeadingBound::Inclusive(self.entries[term].leading.clone())
            })
    }
}
