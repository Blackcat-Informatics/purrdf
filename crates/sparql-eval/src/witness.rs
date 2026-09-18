// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! of the schedule as much as of the index. Unioning the declarations and summing the
//! counts makes the record a function of *what was attested* and nothing else, which is
//! what lets [`RelationWitness::merge`] fold a forked child's ledger back into its
//! parent without a lock and without caring which worker finished first — see that
//! method's own docs for the full argument.

use std::collections::{BTreeMap, BTreeSet, btree_map};

use crate::property_fn::{IndexGeneration, ServiceLevel, push_canonical_field};

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

    /// Append this ledger's canonical, injective encoding to `out`.
    fn push_canonical(&self, out: &mut String) {
        push_canonical_field(out, &self.invocations.to_string());
        push_canonical_field(out, &self.generations.len().to_string());
        for generation in &self.generations {
            match generation {
                IndexGeneration::Undeclared => push_canonical_field(out, GENERATION_UNDECLARED),
                IndexGeneration::Declared(value) => {
                    push_canonical_field(out, GENERATION_DECLARED);
                    push_canonical_field(out, value);
                }
            }
        }
        push_canonical_field(out, &self.incompleteness.len().to_string());
        for reason in &self.incompleteness {
            push_canonical_field(out, reason);
        }
    }
}

/// The canonical tag of an [`IndexGeneration::Undeclared`] entry. A framed tag with no
/// following value.
const GENERATION_UNDECLARED: &str = "u";

/// The canonical tag of an [`IndexGeneration::Declared`] entry. A framed tag followed by
/// the framed declared spelling, so the two cases can never be read as one another.
const GENERATION_DECLARED: &str = "d";

/// The version tag every [`RelationWitness::canonical_bytes`] encoding opens with, so a
/// stored record and a freshly computed one cannot be compared across a change to the
/// encoding without the difference showing up in the first field.
const WITNESS_ENCODING: &str = "purrdf-relation-witness/1";

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

    /// This witness's canonical, injective byte encoding — identical on every target, in
    /// every build, for equal witnesses, and different for unequal ones.
    ///
    /// # The framing discipline
    ///
    /// Every component is written as a **length-framed field** (`<decimal length>:<bytes>`),
    /// the same discipline
    /// `property_fn`'s `push_canonical_field` already gives
    /// the registry's ranked declarations, and the same one `purrdf-retrieval`'s
    /// `FusionProfile::canonical_bytes` and plan encoder apply with their own `Writer`.
    /// Framing is what makes the encoding injective rather than merely deterministic:
    /// without it, a relation IRI ending in a digit and a count beginning with one could
    /// concatenate into the same bytes as a different pair, and two genuinely different
    /// witnesses would hash alike. Every collection is preceded by its own element count
    /// for the same reason.
    ///
    /// # Why it is deterministic
    ///
    /// The map and both sets are ordered ([`BTreeMap`]/[`BTreeSet`]), so the traversal
    /// order is the values' own order and not a hasher's. Nothing here reads a clock, a
    /// thread id, an address, or a locale, and the only integers written are rendered in
    /// decimal, so the bytes are identical on a 32-bit wasm target and a 64-bit host.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = String::new();
        push_canonical_field(&mut out, WITNESS_ENCODING);
        push_canonical_field(&mut out, &self.entries.len().to_string());
        for (iri, attestations) in &self.entries {
            push_canonical_field(&mut out, iri);
            attestations.push_canonical(&mut out);
        }
        out.into_bytes()
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
        IndexGeneration::Declared(value.to_owned())
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
        assert_eq!(
            left_assoc.canonical_bytes(),
            right_assoc_full.canonical_bytes()
        );
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

    /// Length framing is what makes the encoding injective: two witnesses that differ
    /// only in where a boundary falls must not encode alike.
    #[test]
    fn canonical_bytes_separates_values_a_concatenation_would_confuse() {
        let mut one = RelationWitness::default();
        one.record("ab", declared("c"), ServiceLevel::Undeclared);
        let mut other = RelationWitness::default();
        other.record("a", declared("bc"), ServiceLevel::Undeclared);
        assert_ne!(one.canonical_bytes(), other.canonical_bytes());
    }

    /// A declared generation and an undeclared one are different facts, and the
    /// encoding keeps them apart even when the declared spelling is the tag itself.
    #[test]
    fn canonical_bytes_separates_undeclared_from_a_lookalike_declaration() {
        let mut undeclared = RelationWitness::default();
        undeclared.record("r", IndexGeneration::Undeclared, ServiceLevel::Undeclared);
        let mut lookalike = RelationWitness::default();
        lookalike.record(
            "r",
            declared(GENERATION_UNDECLARED),
            ServiceLevel::Undeclared,
        );
        assert_ne!(undeclared.canonical_bytes(), lookalike.canonical_bytes());
    }

    #[test]
    fn canonical_bytes_is_stable_across_repeated_encodings() {
        let mut witness = RelationWitness::default();
        witness.record(
            "https://example.org/rel/a",
            declared("gen-7"),
            incomplete("x"),
        );
        witness.record(
            "https://example.org/rel/b",
            IndexGeneration::Undeclared,
            ServiceLevel::Undeclared,
        );
        assert_eq!(witness.canonical_bytes(), witness.canonical_bytes());
        assert_eq!(
            String::from_utf8(witness.canonical_bytes()).expect("the encoding is UTF-8"),
            String::from_utf8(witness.clone().canonical_bytes()).expect("the encoding is UTF-8")
        );
    }
}
