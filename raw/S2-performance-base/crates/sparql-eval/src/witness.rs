// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What the relations invoked by one query attested about themselves.
//!
//! A relation is host code over host state, and two facts about that state can change a
//! query's answer while every input the evaluator can see stays identical: **which
//! version of the backing index answered**, and **whether that index was whole**. Both
//! are declared by the cursor ([`PfCursor::generation`](crate::property_fn::PfCursor::generation),
//! [`PfCursor::service_level`](crate::property_fn::PfCursor::service_level)) and by
//! nothing else — the dataset snapshot, the query text, and the registry fingerprint are
//! all unchanged by an index rebuild. This module is the per-query ledger those
//! declarations accumulate into on their way to the caller.
//!
//! # Why this is a ledger and not a log
//!
//! Every field here is a SET or a COUNT, never a sequence. A query can invoke one
//! relation thousands of times, across several forked workers, in an order the evaluator
//! is free to choose; an ordered log of what each invocation said would be a description
//! of the schedule as much as of the index. Unioning the declarations makes them a
//! function of *what was attested* and nothing else, which is what lets
//! [`RelationWitness::merge`] fold a forked child's ledger back into its parent without a
//! lock and without caring which worker finished first — see that method's own docs for
//! the full argument. The one field that is not schedule-independent is
//! [`RelationAttestations::invocations`], and its own docs say so at the point a reader
//! reads it.
//!
//! # This ledger has no canonical byte encoding, deliberately
//!
//! It is a value with a total order on every key, so equal ledgers compare equal and
//! [`RelationWitness::iter`] yields the same sequence on every target — that is all the
//! determinism a caller rendering a receipt needs, and it is checked by comparing ledgers
//! rather than digests.
//!
//! Minting bytes here as well would put a SECOND canonical encoding of "what the indexes
//! attested" in the tree beside the one that ships. `purrdf-retrieval` already has that
//! encoder: it collapses this ledger per stratum and digests the result into its
//! `EvidenceId`, which is the identity an answer is compared by. Those bytes cannot be
//! derived from an encoding of this type and must not be — they are keyed by STRATUM
//! rather than by relation IRI, they carry exactly one generation and one service level
//! rather than the sets here, and above all they deliberately exclude `invocations`,
//! because an answer whose evidence identity moved with the evaluator's chunk count would
//! stop being comparable with the answer beside it. Two encodings of one fact is two
//! chances to disagree; this crate keeps the ledger and the consumer keeps the encoding.

use std::collections::{BTreeMap, BTreeSet, btree_map};

use crate::property_fn::{IndexGeneration, ServiceLevel};

/// Everything one relation attested across every invocation it served in one query.
///
/// The three fields answer three different questions a caller actually asks — how much
/// did this query lean on this relation, which index versions answered, and did any of
/// them admit to being short — and none of the three can be derived from the others.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationAttestations {
    /// How many invocations of this relation entered host code during the query.
    ///
    /// Counted at the same place the
    /// [`ChargePoint::PropertyFunctionInvocation`](crate::governor::ChargePoint::PropertyFunctionInvocation)
    /// charge is paid, so the receipt and the meter describe the same executions: an
    /// invocation refused by the arity check or by the access-pattern check entered no
    /// relation and is counted by neither.
    ///
    /// Summed with saturation on merge. A query that overflowed a `u64` of invocations
    /// would have had to run for longer than any deadline this engine can express, and
    /// wrapping to a small number is the one failure mode a count like this must not
    /// have: it would report a heavily-leaned-on relation as barely touched.
    ///
    /// # This count is a fact about the SCHEDULE, not about the index
    ///
    /// Unlike the two declaration sets beside it, this number is not a function of what
    /// was attested, and it is **not comparable across runs** — not even across two runs
    /// of the same query over the same data on the same machine.
    ///
    /// The lane where it moves is an expression-embedded `EXISTS` under the parallel row
    /// loop. Each chunk of driving rows is evaluated on a forked child whose
    /// `exists_inner_cache` is a snapshot taken at fork time, so the inner pattern — and
    /// therefore any relation inside it — is re-entered once per chunk, and the chunk
    /// count is derived from the runtime's thread count. A thousand driving rows can
    /// charge this relation once or once per worker for exactly the same index and exactly
    /// the same answer.
    ///
    /// It is the same dependence `crate::parallel`'s `expression_re_enters_evaluation`
    /// documents for the fuel meter, which is worth reading for the measured numbers — but
    /// the remedy recorded there does not reach this field. That rule forces the loop
    /// sequential only while a governor is **engaged**, because what it protects is an
    /// exact meter; a witnessed run under a budget that declines every ceiling (the
    /// `UNBOUNDED` governors `purrdf-retrieval` runs every unit under, for one) has no
    /// engaged governor and forks exactly as before. So this count can differ between two
    /// runs of one query over one dataset that differ only in the budget they were given.
    ///
    /// What does NOT move with it is every other field of this ledger: the generation a
    /// re-entered invocation pins is the same generation, and the reason it gives for a
    /// missing shard is the same reason, so [`Self::generations`] and
    /// [`Self::incompleteness`] union back to identical sets however the rows were
    /// chunked. That is why `purrdf-retrieval` keys its per-stratum conformance rule and
    /// its evidence identity on the sets alone and reads this count only to ignore it: a
    /// rule keyed here would fail on an input size rather than on a defect.
    ///
    /// So the two readings this value supports are "did this relation run at all"
    /// (`0` versus non-zero, which is exact and stable) and "roughly how hard did this
    /// execution lean on it" (a magnitude, useful for a log line or a cost attribution,
    /// never for an equality comparison between two receipts).
    pub invocations: u64,
    /// Every distinct generation any of those invocations declared, ordered.
    ///
    /// A set rather than a single value because a long-running query CAN legitimately
    /// straddle a rebuild: one invocation pins generation 7, a later one pins 8, and a
    /// caller reading two entries here learns the one thing that would otherwise be
    /// invisible — that this answer was assembled from two different indexes. A
    /// relation that declared nothing contributes
    /// [`IndexGeneration::Undeclared`], which is a member of the set like any
    /// other, because "some invocations said nothing" is itself worth being able to see.
    pub generations: BTreeSet<IndexGeneration>,
    /// The reasons given by invocations that declared their index NOT whole, ordered,
    /// de-duplicated, verbatim.
    ///
    /// EMPTY is the overwhelmingly common case, and it means exactly "nobody declared an
    /// incompleteness" — never "the index was certified whole", which is a claim this
    /// seam deliberately cannot carry (see [`ServiceLevel`]). Holding the reason strings
    /// rather than a bare flag is what makes the record actionable: "shard 3 is
    /// rebuilding" tells an operator what to do, a `true` does not.
    pub incompleteness: BTreeSet<String>,
}

impl RelationAttestations {
    /// Fold one invocation's attestation in: count it, record what it declared.
    fn record(&mut self, generation: IndexGeneration, service: ServiceLevel) {
        self.invocations = self.invocations.saturating_add(1);
        self.generations.insert(generation);
        if let ServiceLevel::Incomplete { reason } = service {
            self.incompleteness.insert(reason);
        }
    }

    /// Fold another ledger for the SAME relation into this one: saturating sum of
    /// counts, set union of declarations. Commutative and associative, per
    /// [`RelationWitness::merge`].
    fn merge(&mut self, other: Self) {
        self.invocations = self.invocations.saturating_add(other.invocations);
        self.generations.extend(other.generations);
        self.incompleteness.extend(other.incompleteness);
    }
}

/// What every relation this query invoked attested, keyed by the relation's registered
/// IRI.
///
/// # Absence, not omission
///
/// An empty witness means no relation attested anything — usually because the query
/// invoked none at all. It never means "this build does not report it": the field is
/// always present on the receipt that carries it
/// ([`RelationIdentity`](crate::RelationIdentity)), which is the same "present but
/// empty" convention every other absence on that receipt uses. A relation that was
/// invoked but overrode neither cursor method still gets an entry here, with an
/// invocation count and a single [`IndexGeneration::Undeclared`] generation — "it ran
/// and said nothing" and "it never ran" are different facts and stay different.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationWitness {
    /// Per-relation ledgers, keyed by registered IRI. A [`BTreeMap`], so iteration is in
    /// IRI order on every target and in every build — a hash map would make the order of
    /// [`Self::iter`], and therefore of anything a caller derived from it, a function of
    /// the hasher rather than of what was attested.
    entries: BTreeMap<String, RelationAttestations>,
}

impl RelationWitness {
    /// The witness of a query that attested nothing, constructible in a `const` context
    /// so a receipt carrying it can keep its own `EMPTY` constant.
    pub const EMPTY: Self = Self {
        entries: BTreeMap::new(),
    };

    /// Whether no relation attested anything — see the type's "Absence, not omission"
    /// note for what this does and does not mean.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The number of distinct relations that attested.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// What the relation registered under `iri` attested, or `None` if it never entered
    /// host code during this query.
    ///
    /// The IRI is matched byte-exactly, exactly as the registry resolves it: a relation
    /// is identified by the IRI it was registered under and by nothing else.
    #[must_use]
    pub fn get(&self, iri: &str) -> Option<&RelationAttestations> {
        self.entries.get(iri)
    }

    /// Every relation's ledger, in IRI order.
    ///
    /// The order is part of the contract, not an implementation detail: a caller that
    /// renders this into a receipt, a log line, or a hash must get the same sequence on
    /// every machine and every run, or the artifact it produces stops being comparable
    /// with the one produced beside it.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &RelationAttestations)> {
        self.entries
            .iter()
            .map(|(iri, attestations)| (iri.as_str(), attestations))
    }

    /// Record one completed invocation of the relation registered under `iri`.
    ///
    /// Called once per invocation that entered host code, at the moment the invocation
    /// ends — which is when both halves are known, the generation having been read at
    /// `open` and the service level at the close (see
    /// [`PfCursor`](crate::property_fn::PfCursor)'s two methods for why those are the
    /// right instants).
    pub fn record(&mut self, iri: &str, generation: IndexGeneration, service: ServiceLevel) {
        match self.entries.get_mut(iri) {
            Some(existing) => existing.record(generation, service),
            None => {
                let mut fresh = RelationAttestations::default();
                fresh.record(generation, service);
                self.entries.insert(iri.to_owned(), fresh);
            }
        }
    }

    /// Fold `other` into this witness: per relation, the invocation counts add
    /// (saturating) and the declaration sets union.
    ///
    /// # Why this operation is commutative and associative, and why that is the point
    ///
    /// `a.merge(b)` and `b.merge(a)` produce the same witness, and so does any bracketing
    /// of a three-way fold: set union and integer addition are both commutative and
    /// associative, and saturation preserves both.
    ///
    /// That is not an incidental algebraic nicety — it is the entire reason this crate
    /// carries the record in an OWNED per-context field instead of behind a shared
    /// `Mutex`. The evaluator's fork-join invariant is that a forked worker gets its
    /// own mutable evaluation state precisely so workers never contend on a lock; a mutex
    /// added here would be taken once per driving row, inside the hottest loop the
    /// evaluator has, to protect data that needs no ordering at all. Each fork instead
    /// starts EMPTY, accumulates privately, and is merged back after the join. Because
    /// the fold is commutative and associative, the result is the same whatever order the
    /// workers finished in and however the rows were chunked — the record is a function
    /// of what was attested, never of the schedule that attested it.
    pub fn merge(&mut self, other: Self) {
        for (iri, attestations) in other.entries {
            match self.entries.entry(iri) {
                btree_map::Entry::Occupied(mut occupied) => occupied.get_mut().merge(attestations),
                btree_map::Entry::Vacant(vacant) => {
                    vacant.insert(attestations);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn incomplete(reason: &str) -> ServiceLevel {
        ServiceLevel::Incomplete {
            reason: reason.to_owned(),
        }
    }

    fn declared(value: &str) -> IndexGeneration {
        IndexGeneration::declared(value)
    }

    #[test]
    fn an_empty_witness_reports_itself_empty() {
        let witness = RelationWitness::default();
        assert!(witness.is_empty());
        assert_eq!(witness.len(), 0);
        assert_eq!(witness.get("https://example.org/rel/a"), None);
        assert_eq!(witness, RelationWitness::EMPTY);
    }

    #[test]
    fn recording_counts_invocations_and_unions_declarations() {
        let mut witness = RelationWitness::default();
        let iri = "https://example.org/rel/a";
        witness.record(iri, declared("gen-7"), ServiceLevel::Undeclared);
        witness.record(iri, declared("gen-7"), incomplete("shard 3 rebuilding"));
        witness.record(iri, declared("gen-8"), ServiceLevel::Undeclared);

        let entry = witness.get(iri).expect("the relation attested");
        assert_eq!(entry.invocations, 3);
        assert_eq!(
            entry.generations,
            BTreeSet::from([declared("gen-7"), declared("gen-8")])
        );
        assert_eq!(
            entry.incompleteness,
            BTreeSet::from(["shard 3 rebuilding".to_owned()])
        );
    }

    /// The property the no-lock design rests on: the fold does not depend on the order
    /// the workers finished in, nor on how the work was bracketed.
    #[test]
    fn merge_is_commutative_and_associative() {
        let a_iri = "https://example.org/rel/a";
        let b_iri = "https://example.org/rel/b";
        let leaf = |iri: &str, generation: &str, reason: Option<&str>| {
            let mut witness = RelationWitness::default();
            witness.record(
                iri,
                declared(generation),
                reason.map_or(ServiceLevel::Undeclared, incomplete),
            );
            witness
        };
        let a = leaf(a_iri, "gen-7", None);
        let b = leaf(a_iri, "gen-8", Some("shard 3"));
        let c = leaf(b_iri, "gen-1", None);

        let mut left = a.clone();
        left.merge(b.clone());
        let mut commuted = b.clone();
        commuted.merge(a.clone());
        assert_eq!(left, commuted);

        let mut left_assoc = a.clone();
        left_assoc.merge(b.clone());
        left_assoc.merge(c.clone());
        let mut right_assoc = b;
        right_assoc.merge(c);
        let mut right_assoc_full = a;
        right_assoc_full.merge(right_assoc);
        assert_eq!(left_assoc, right_assoc_full);
    }

    /// Two ledgers that differ only in where a boundary between an IRI and a declared
    /// spelling falls are DIFFERENT ledgers, and equality says so.
    ///
    /// This is the property a canonical encoding of this type would have had to preserve
    /// by framing its fields. It holds here without any encoding, because the value is a
    /// map keyed by the whole IRI rather than a concatenation of its parts — which is why
    /// no second encoder is needed to check it.
    #[test]
    fn a_boundary_that_slides_is_a_different_ledger() {
        let mut one = RelationWitness::default();
        one.record("ab", declared("c"), ServiceLevel::Undeclared);
        let mut other = RelationWitness::default();
        other.record("a", declared("bc"), ServiceLevel::Undeclared);
        assert_ne!(one, other);
    }

    /// Silence and a declaration are different facts, whatever the declared spelling is.
    #[test]
    fn an_undeclared_generation_never_equals_a_declared_one() {
        let mut undeclared = RelationWitness::default();
        undeclared.record("r", IndexGeneration::Undeclared, ServiceLevel::Undeclared);
        for spelling in ["", "u", "undeclared"] {
            let mut lookalike = RelationWitness::default();
            lookalike.record("r", declared(spelling), ServiceLevel::Undeclared);
            assert_ne!(
                undeclared, lookalike,
                "a relation that said nothing must not compare equal to one that said {spelling:?}"
            );
        }
    }

    #[test]
    fn iteration_is_in_iri_order_whatever_the_recording_order() {
        let mut forwards = RelationWitness::default();
        for iri in ["https://example.org/rel/c", "https://example.org/rel/a"] {
            forwards.record(iri, IndexGeneration::Undeclared, ServiceLevel::Undeclared);
        }
        let seen: Vec<&str> = forwards.iter().map(|(iri, _)| iri).collect();
        assert_eq!(
            seen,
            vec!["https://example.org/rel/a", "https://example.org/rel/c"]
        );
    }

    /// Two relations that rendered the SAME generation into two separate allocations
    /// attest the same member of the set — the shared-pointer payload compares by
    /// spelling, never by address, or a set would hold one entry per producer instead of
    /// one per generation.
    #[test]
    fn two_separately_allocated_spellings_are_one_generation() {
        let left: std::sync::Arc<str> = std::sync::Arc::from("gen-7".to_owned());
        let right: std::sync::Arc<str> = std::sync::Arc::from(String::from("gen-7"));
        assert!(
            !std::sync::Arc::ptr_eq(&left, &right),
            "the fixture really does hold two allocations, or the claim below is vacuous"
        );

        let mut witness = RelationWitness::default();
        let iri = "https://example.org/rel/a";
        witness.record(
            iri,
            IndexGeneration::Declared(left),
            ServiceLevel::Undeclared,
        );
        witness.record(
            iri,
            IndexGeneration::Declared(right),
            ServiceLevel::Undeclared,
        );
        let entry = witness.get(iri).expect("the relation attested");
        assert_eq!(entry.invocations, 2);
        assert_eq!(entry.generations, BTreeSet::from([declared("gen-7")]));
    }
}
