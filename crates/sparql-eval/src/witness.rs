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

use std::collections::BTreeSet;

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
    pub generations: WitnessSet<IndexGeneration>,
    /// The reasons given by invocations that declared their index NOT whole, ordered,
    /// de-duplicated, verbatim.
    ///
    /// EMPTY is the overwhelmingly common case, and it means exactly "nobody declared an
    /// incompleteness" — never "the index was certified whole", which is a claim this
    /// seam deliberately cannot carry (see [`ServiceLevel`]). Holding the reason strings
    /// rather than a bare flag is what makes the record actionable: "shard 3 is
    /// rebuilding" tells an operator what to do, a `true` does not.
    pub incompleteness: WitnessSet<purrdf_lex::allocation::SharedText>,
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
#[derive(Clone, Debug, Default)]
pub struct RelationWitness {
    /// Per-relation ledgers in a sorted native array, sharing original buffer
    /// ownership when a receipt is cloned.
    entries: Option<crate::workspace::SharedWorkspace<WitnessEntries>>,
}

impl RelationWitness {
    /// The witness of a query that attested nothing, constructible in a `const` context
    /// so a receipt carrying it can keep its own `EMPTY` constant.
    pub const EMPTY: Self = Self { entries: None };

    /// Whether no relation attested anything — see the type's "Absence, not omission"
    /// note for what this does and does not mean.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The number of distinct relations that attested.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.as_ref().map_or(0, |entries| entries.len())
    }

    /// What the relation registered under `iri` attested, or `None` if it never entered
    /// host code during this query.
    ///
    /// The IRI is matched byte-exactly, exactly as the registry resolves it: a relation
    /// is identified by the IRI it was registered under and by nothing else.
    #[must_use]
    pub fn get(&self, iri: &str) -> Option<&RelationAttestations> {
        let entries = self.entries.as_ref()?;
        entries
            .binary_search_by(|(name, _)| name.as_str().cmp(iri))
            .ok()
            .map(|index| &entries[index].1)
    }

    /// Every relation's ledger, in IRI order.
    ///
    /// The order is part of the contract, not an implementation detail: a caller that
    /// renders this into a receipt, a log line, or a hash must get the same sequence on
    /// every machine and every run, or the artifact it produces stops being comparable
    /// with the one produced beside it.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &RelationAttestations)> {
        self.entries
            .as_deref()
            .map_or(&[][..], |entries| &entries[..])
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
        self.try_record(
            iri,
            &generation,
            &service,
            &crate::workspace::WorkspaceCapability::resident(),
        )
        .expect("resident relation witness allocation failed");
        drop((generation, service));
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
        self.try_merge(&other, &crate::workspace::WorkspaceCapability::resident())
            .expect("resident relation witness merge allocation failed");
        drop(other);
    }
}

/// An ordered declaration set whose native clones share original payload and
/// buffer owners. Mutations copy shared bookkeeping fallibly; unique arrays grow
/// at their original admitted producer.
#[derive(Debug)]
pub struct WitnessSet<T> {
    values: Option<
        crate::workspace::SharedWorkspace<
            crate::workspace::AdmittedVec<purrdf_lex::allocation::SharedOwned<T>>,
        >,
    >,
}

impl<T> Default for WitnessSet<T> {
    fn default() -> Self {
        Self { values: None }
    }
}
impl<T> Clone for WitnessSet<T> {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
        }
    }
}
impl<T> WitnessSet<T> {
    /// The number of distinct declarations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.as_ref().map_or(0, |values| values.len())
    }
    /// Whether no declaration was recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Borrow declarations in their specified total order.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        self.values
            .as_deref()
            .map_or(&[][..], |values| &values[..])
            .iter()
            .map(|value| &**value)
    }
    /// Borrow the first declaration without copying its original owner.
    #[must_use]
    pub fn first(&self) -> Option<&T> {
        self.iter().next()
    }
}
impl<T: PartialEq> PartialEq for WitnessSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}
impl<T: Eq> Eq for WitnessSet<T> {}
impl<T: Ord> PartialEq<BTreeSet<T>> for WitnessSet<T> {
    fn eq(&self, other: &BTreeSet<T>) -> bool {
        self.iter().eq(other.iter())
    }
}
impl PartialEq<BTreeSet<String>> for WitnessSet<purrdf_lex::allocation::SharedText> {
    fn eq(&self, other: &BTreeSet<String>) -> bool {
        self.iter()
            .map(purrdf_lex::allocation::SharedText::as_str)
            .eq(other.iter().map(String::as_str))
    }
}
impl<T: Ord> WitnessSet<T> {
    /// Whether this exact declaration was recorded.
    #[must_use]
    pub fn contains<Q: Ord + ?Sized>(&self, value: &Q) -> bool
    where
        T: core::borrow::Borrow<Q>,
    {
        self.values.as_ref().is_some_and(|values| {
            values
                .binary_search_by(|candidate| {
                    core::borrow::Borrow::<Q>::borrow(&**candidate).cmp(value)
                })
                .is_ok()
        })
    }
}

fn mutable_witness_array<'a, T: Clone>(
    storage: &'a mut Option<crate::workspace::SharedWorkspace<crate::workspace::AdmittedVec<T>>>,
    workspace: &crate::workspace::WorkspaceCapability,
) -> Result<&'a mut crate::workspace::AdmittedVec<T>, crate::EvalError> {
    let unique = storage
        .as_mut()
        .is_some_and(|values| values.unique_mut().is_some());
    if !unique {
        let count = storage.as_ref().map_or(0, |values| values.len());
        let mut replacement = crate::workspace::AdmittedVec::with_capacity(count, workspace)?;
        if let Some(original) = storage.as_ref() {
            for value in original.iter() {
                replacement.push(value.clone())?;
            }
        }
        *storage = Some(crate::workspace::SharedWorkspace::new_admitted(
            replacement,
            workspace,
        )?);
    }
    Ok(storage
        .as_mut()
        .expect("the original array was installed")
        .unique_mut()
        .expect("the original array is unique"))
}

fn retained_witness_value<T: Send + Sync + 'static, U: ?Sized>(
    value: &U,
    workspace: &crate::workspace::WorkspaceCapability,
    copy: impl FnOnce(
        &U,
        &mut purrdf_lex::allocation::Memory<'_, crate::workspace::LexicalFrame>,
    ) -> Result<T, purrdf_lex::allocation::StorageError>,
) -> Result<purrdf_lex::allocation::SharedOwned<T>, crate::EvalError> {
    use crate::workspace::LexicalFrame;
    use purrdf_lex::allocation::{Memory, SharedOwned};
    let mut frame = LexicalFrame::new(workspace);
    let copied = {
        let mut memory = Memory::new(&mut frame);
        memory
            .add_bytes(SharedOwned::<T>::allocation_layout::<LexicalFrame>().size())
            .map_err(|error| {
                memory
                    .admission_mut()
                    .storage_error(error, "relation witness owner")
            })?;
        copy(value, &mut memory).map_err(|error| {
            memory
                .admission_mut()
                .storage_error(error, "relation witness value")
        })?
    };
    SharedOwned::try_from_admitted(copied, frame).map_err(|_| crate::EvalError::AllocationFailed {
        construct: "relation witness owner",
    })
}

impl<T: Ord + Send + Sync + 'static> WitnessSet<T> {
    fn insert_owned(
        &mut self,
        value: purrdf_lex::allocation::SharedOwned<T>,
        workspace: &crate::workspace::WorkspaceCapability,
    ) -> Result<bool, crate::EvalError> {
        let Err(index) = self.values.as_ref().map_or(Err(0), |values| {
            values.binary_search_by(|candidate| (**candidate).cmp(&*value))
        }) else {
            return Ok(false);
        };
        let values = mutable_witness_array(&mut self.values, workspace)?;
        values.push(value)?;
        values.as_mut_slice()[index..].rotate_right(1);
        Ok(true)
    }
    fn insert_admitted(
        &mut self,
        value: &T,
        workspace: &crate::workspace::WorkspaceCapability,
        copy: impl FnOnce(
            &T,
            &mut purrdf_lex::allocation::Memory<'_, crate::workspace::LexicalFrame>,
        ) -> Result<T, purrdf_lex::allocation::StorageError>,
    ) -> Result<bool, crate::EvalError> {
        if self.contains(value) {
            return Ok(false);
        }
        self.insert_owned(retained_witness_value(value, workspace, copy)?, workspace)
    }
    fn merge_admitted(
        &mut self,
        other: &Self,
        workspace: &crate::workspace::WorkspaceCapability,
    ) -> Result<(), crate::EvalError> {
        if let Some(values) = &other.values {
            for value in values.iter() {
                self.insert_owned(value.clone(), workspace)?;
            }
        }
        Ok(())
    }
}

impl<T: Ord + Clone + Send + Sync + 'static> WitnessSet<T> {
    /// Insert a caller-owned declaration through the resident producer.
    pub fn insert(&mut self, value: T) -> bool {
        let inserted = self
            .insert_admitted(
                &value,
                &crate::workspace::WorkspaceCapability::resident(),
                |value, _| Ok(value.clone()),
            )
            .expect("resident witness set allocation failed");
        drop(value);
        inserted
    }
}
impl<T: Ord + Clone + Send + Sync + 'static> FromIterator<T> for WitnessSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(values: I) -> Self {
        let mut result = Self::default();
        for value in values {
            result.insert(value);
        }
        result
    }
}
impl<'a, T> IntoIterator for &'a WitnessSet<T> {
    type Item = &'a T;
    type IntoIter = WitnessSetIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        WitnessSetIter(
            self.values
                .as_deref()
                .map_or(&[][..], |values| &values[..])
                .iter(),
        )
    }
}
/// A borrowed declaration iterator keeps its original set owner borrowed.
#[derive(Debug)]
pub struct WitnessSetIter<'a, T>(core::slice::Iter<'a, purrdf_lex::allocation::SharedOwned<T>>);
impl<'a, T> Iterator for WitnessSetIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|value| &**value)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<T> ExactSizeIterator for WitnessSetIter<'_, T> {}

type WitnessEntries = crate::workspace::AdmittedVec<(
    purrdf_lex::allocation::SharedOwned<String>,
    RelationAttestations,
)>;

impl PartialEq for RelationWitness {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}
impl Eq for RelationWitness {}

impl RelationAttestations {
    fn record_admitted(
        &mut self,
        generation: &IndexGeneration,
        service: &ServiceLevel,
        workspace: &crate::workspace::WorkspaceCapability,
    ) -> Result<(), crate::EvalError> {
        self.generations
            .insert_admitted(generation, workspace, |value, _| Ok(value.clone()))?;
        if let ServiceLevel::Incomplete { reason } = service {
            self.incompleteness
                .insert_admitted(reason, workspace, |reason, _| Ok(reason.clone()))?;
        }
        self.invocations = self.invocations.saturating_add(1);
        Ok(())
    }
    fn merge_admitted(
        &mut self,
        other: &Self,
        workspace: &crate::workspace::WorkspaceCapability,
    ) -> Result<(), crate::EvalError> {
        self.generations
            .merge_admitted(&other.generations, workspace)?;
        self.incompleteness
            .merge_admitted(&other.incompleteness, workspace)?;
        self.invocations = self.invocations.saturating_add(other.invocations);
        Ok(())
    }
}

impl RelationWitness {
    /// Record at the original relation receipt producer. IRI/reason strings and
    /// declaration arrays are admitted before allocation; an existing generation
    /// shares its caller-supplied immutable spelling without copying it.
    ///
    /// # Errors
    /// Returns physical workspace or allocator refusal while existing owners stay live.
    pub fn try_record(
        &mut self,
        iri: &str,
        generation: &IndexGeneration,
        service: &ServiceLevel,
        workspace: &crate::workspace::WorkspaceCapability,
    ) -> Result<(), crate::EvalError> {
        let index = self.entries.as_ref().map_or(Err(0), |entries| {
            entries.binary_search_by(|(name, _)| name.as_str().cmp(iri))
        });
        match index {
            Ok(index) => {
                let entries = mutable_witness_array(&mut self.entries, workspace)?;
                entries.as_mut_slice()[index]
                    .1
                    .record_admitted(generation, service, workspace)?;
            }
            Err(index) => {
                let name =
                    retained_witness_value(&iri, workspace, |text, memory| memory.string(text))?;
                let mut attestations = RelationAttestations::default();
                attestations.record_admitted(generation, service, workspace)?;
                let entries = mutable_witness_array(&mut self.entries, workspace)?;
                entries.push((name, attestations))?;
                entries.as_mut_slice()[index..].rotate_right(1);
            }
        }
        Ok(())
    }

    /// Merge original child receipts by sharing their declaration payloads and
    /// admitting only genuinely new destination arrays/control storage.
    ///
    /// # Errors
    /// Returns physical workspace or allocator refusal.
    pub fn try_merge(
        &mut self,
        other: &Self,
        workspace: &crate::workspace::WorkspaceCapability,
    ) -> Result<(), crate::EvalError> {
        if let Some(source) = &other.entries {
            for (name, attestations) in source.iter() {
                let index = self.entries.as_ref().map_or(Err(0), |entries| {
                    entries.binary_search_by(|(candidate, _)| candidate.as_str().cmp(name.as_str()))
                });
                let entries = mutable_witness_array(&mut self.entries, workspace)?;
                match index {
                    Ok(index) => entries.as_mut_slice()[index]
                        .1
                        .merge_admitted(attestations, workspace)?,
                    Err(index) => {
                        entries.push((name.clone(), attestations.clone()))?;
                        entries.as_mut_slice()[index..].rotate_right(1);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn incomplete(reason: &str) -> ServiceLevel {
        ServiceLevel::incomplete(reason)
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
            BTreeSet::from([purrdf_lex::allocation::SharedText::from_static(
                "shard 3 rebuilding"
            )])
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
