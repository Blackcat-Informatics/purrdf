// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fallible `RdfDataset` builder: value-interning plus the quad / reifier /
//! annotation / source-location tables, and the validate-then-freeze path (C1).
//!
//! This module owns term storage and value-interning. The C0 literal-identity
//! policy (datatype expansion, language lowercasing, direction-in-key, verbatim
//! lexical spelling — see `docs/design/819-rdf-ir-dataflow.md` *Appendix C0.1*) is
//! applied here, at intern time, so that the frozen dataset carries fully resolved
//! identity.
//!
//! Pushing structure (`push_quad` / `push_reifier` / `push_annotation` /
//! `attach_location`) accumulates raw rows; [`RdfDatasetBuilder::freeze`] then runs
//! structural validation ([`super::validate`]) and, on success, materializes an
//! immutable, deterministically-ordered, deduplicated [`RdfDataset`]. Per the
//! no-optionality doctrine, malformed structure is a HARD failure (`Err`), never a
//! silent default.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use hashbrown::HashTable;
use purrdf_iri::IriError;
use purrdf_lex::allocation::{Admission, CapacitySpace, Memory, Resident, StorageError};

use crate::blank_label::LabelAlphabet;
use crate::{
    Blake3ContentId, ContentIdScheme, RdfAnnotation, RdfLiteral, RdfQuad, RdfReifier, RdfTerm,
    RdfTextDirection,
};

use super::dataset::{FastHasher, QuadHandle, QuadIds, QuadRow, RdfDataset, TermRef};
use super::term::{
    BlankScope, InternedLiteral, InternedTerm, StrRange, TermId, arena_str,
    push_arena_str_with_memory,
};
use crate::RdfLocation;
// The store-once tables hash with the fixed-key `hash_of`, so they are
// deterministic across runs. The frozen output is sorted by id, not
// hash-iteration order, so any hash would do; the fixed key avoids SipHash on the
// hot interning path.
use crate::hash::hash_of;

mod triple;
pub use triple::TripleDatasetBuilder;

fn hash_lookup_value(lookup: &TermLookup<'_>) -> u64 {
    match lookup {
        TermLookup::Iri(iri) => crate::hash::hash_iri_for_interner(iri),
        TermLookup::Blank { label, scope } => crate::hash::hash_blank_for_interner(label, scope.0),
        TermLookup::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => crate::hash::hash_literal_for_interner(
            lexical,
            datatype.index() as u64,
            *language,
            *direction,
        ),
        TermLookup::Triple { s, p, o } => crate::hash::hash_triple_for_interner(
            s.index() as u64,
            p.index() as u64,
            o.index() as u64,
        ),
    }
}

fn hash_stored_value(arena: &[u8], term: &InternedTerm) -> u64 {
    match term {
        InternedTerm::Iri(r) => crate::hash::hash_iri_for_interner(arena_str(arena, *r)),
        InternedTerm::Blank { label, scope } => {
            crate::hash::hash_blank_for_interner(arena_str(arena, *label), scope.0)
        }
        InternedTerm::Literal(lit) => crate::hash::hash_literal_for_interner(
            arena_str(arena, lit.lexical_form),
            lit.datatype.index() as u64,
            lit.language.map(|r| arena_str(arena, r)),
            lit.direction,
        ),
        InternedTerm::Triple { s, p, o } => crate::hash::hash_triple_for_interner(
            s.index() as u64,
            p.index() as u64,
            o.index() as u64,
        ),
    }
}

/// Production is zero-sized; reduced limits exercise the same public ingress
/// without allocating multi-billion-entry dictionaries or row tables.
#[derive(Clone, Copy, Debug, Default)]
pub(super) enum LocalCapacity {
    #[default]
    Native,
    #[cfg(test)]
    Reduced { terms: u32, text: u32, rows: u32 },
}

enum ImportBinding {
    Preserve,
    Fresh(Option<BlankScope>),
}

impl LocalCapacity {
    fn limit(self, space: CapacitySpace) -> u128 {
        match (self, space) {
            (Self::Native, _) => u128::from(u32::MAX),
            #[cfg(test)]
            (Self::Reduced { terms, text, rows }, space) => u128::from(match space {
                CapacitySpace::LocalTerms | CapacitySpace::DeltaTerms => terms,
                CapacitySpace::LocalTermBytes => text,
                CapacitySpace::Quads
                | CapacitySpace::Triples
                | CapacitySpace::Reifiers
                | CapacitySpace::Annotations
                | CapacitySpace::Permutation => rows,
                _ => unreachable!("local builder capacity space"),
            }),
        }
    }

    pub(super) fn required(
        self,
        space: CapacitySpace,
        len: usize,
        additional: usize,
    ) -> Result<usize, StorageError> {
        let requested = len as u128 + additional as u128;
        purrdf_lex::allocation::CapacityError::check(space, requested, self.limit(space))?;
        usize::try_from(requested).map_err(|_| StorageError::SizeOverflow)
    }

    fn hint(self, space: CapacitySpace, len: usize, additional: usize) -> usize {
        // A whole-source count is not a count of new destination values. An
        // unrepresentable upper-bound hint must not refuse duplicate-only replay.
        self.required(space, len, additional).unwrap_or(len)
    }
}

/// **Store-once** insert-or-find (P3c): `vec` is the sole owner of the values;
/// `table` holds only their `u32` indices, with hash/eq that look INTO `vec`. Returns
/// the index of the (existing or newly pushed) value. No value is ever stored twice.
///
/// `vec` and `table` are distinct `&mut` params — callers pass disjoint struct fields,
/// so the find-then-push-then-insert sequence has no overlapping borrow.
fn store_once<T: Hash + Eq>(
    vec: &mut Vec<T>,
    table: &mut HashTable<u32>,
    value: T,
    capacity: LocalCapacity,
    space: CapacitySpace,
) -> Result<u32, StorageError> {
    let mut resident = Resident;
    let bytes = std::alloc::Layout::array::<T>(vec.capacity())
        .map_err(|_| StorageError::SizeOverflow)?
        .size()
        .checked_add(
            crate::hash::hash_table_retained_bound(table).ok_or(StorageError::SizeOverflow)?,
        )
        .ok_or(StorageError::SizeOverflow)?;
    let mut memory = Memory::resume(&mut resident, bytes);
    store_once_with_memory(vec, table, value, capacity, space, &mut memory)
}
fn store_once_with_memory<T: Hash + Eq, S: Admission + ?Sized>(
    vec: &mut Vec<T>,
    table: &mut HashTable<u32>,
    value: T,
    capacity: LocalCapacity,
    space: CapacitySpace,
    memory: &mut Memory<'_, S>,
) -> Result<u32, StorageError> {
    store_once_with_dependency(vec, table, value, capacity, space, memory, |_| Ok(()))
}

fn remove_index_suffix<T: Hash>(values: &[T], table: &mut HashTable<u32>, from: usize) {
    for (index, value) in values.iter().enumerate().skip(from) {
        if let Ok(entry) = table.find_entry(hash_of(value), |&stored| stored as usize == index) {
            entry.remove();
        }
    }
}

// Admit the row before constructing an implicit dependency, and publish the row
// only after that dependency succeeds. A duplicate needs neither operation.
fn store_once_with_dependency<T: Hash + Eq, S: Admission + ?Sized>(
    vec: &mut Vec<T>,
    table: &mut HashTable<u32>,
    value: T,
    capacity: LocalCapacity,
    space: CapacitySpace,
    memory: &mut Memory<'_, S>,
    dependency: impl FnOnce(&mut Memory<'_, S>) -> Result<(), StorageError>,
) -> Result<u32, StorageError> {
    let hash = hash_of(&value);
    if let Some(&i) = table.find(hash, |&i| vec[i as usize] == value) {
        return Ok(i);
    }
    let required = capacity.required(space, vec.len(), 1)?;
    let i = u32::try_from(vec.len()).map_err(|_| StorageError::SizeOverflow)?;
    let capacity = purrdf_lex::allocation::growth_capacity::<T>(vec.capacity(), required)?;
    memory.reserve(vec, capacity)?;
    crate::hash::reserve_table_with_memory(
        table,
        vec.len().checked_add(1).ok_or(StorageError::SizeOverflow)?,
        |&i| hash_of(&vec[i as usize]),
        memory,
    )?;
    dependency(memory)?;
    vec.push(value);
    table.insert_unique(hash, i, |&i| hash_of(&vec[i as usize]));
    Ok(i)
}

/// Reserve room for `additional` more values in a [`store_once`] pair: the table
/// and its index grow together, the index rehashing through the values it points
/// into, so a bulk load that states its size rehashes neither mid-way.
fn reserve_store_once<T: Hash>(
    vec: &mut Vec<T>,
    table: &mut HashTable<u32>,
    additional: usize,
    capacity: LocalCapacity,
    space: CapacitySpace,
) -> Result<(), StorageError> {
    let required = capacity.hint(space, vec.len(), additional);
    let bytes = std::alloc::Layout::array::<T>(vec.capacity())
        .map_err(|_| StorageError::SizeOverflow)?
        .size()
        .checked_add(
            crate::hash::hash_table_retained_bound(table).ok_or(StorageError::SizeOverflow)?,
        )
        .ok_or(StorageError::SizeOverflow)?;
    let mut resident = Resident;
    let mut memory = Memory::resume(&mut resident, bytes);
    memory.reserve(vec, required)?;
    crate::hash::reserve_table_with_memory(
        table,
        required,
        |&i| hash_of(&vec[i as usize]),
        &mut memory,
    )
}

/// A borrowed term lookup key (P3b): carries the string components by reference
/// so the interner can dedup BY VALUE *before* anything is pushed to the arena.
/// Mirrors [`InternedTerm`] but with `&str` where the stored form holds a `StrRange`.
#[derive(Clone, Copy)]
enum TermLookup<'a> {
    Iri(&'a str),
    Blank {
        label: &'a str,
        scope: BlankScope,
    },
    Literal {
        lexical: &'a str,
        datatype: TermId,
        language: Option<&'a str>,
        direction: Option<RdfTextDirection>,
    },
    Triple {
        s: TermId,
        p: TermId,
        o: TermId,
    },
}

/// One pending step of an owned-term interning walk
/// ([`RdfDatasetBuilder::intern_owned_term_scoped`] and
/// [`RdfDatasetBuilder::intern_owned_term_bound`]): a term to intern, a triple
/// term's predicate IRI to intern, or a triple term to assemble from the three ids
/// its components left.
enum OwnedStep<'t> {
    Term(&'t RdfTerm),
    Predicate(&'t str),
    Assemble,
}

/// Whether a stored term equals a lookup, resolving the stored ranges through `arena`.
fn term_eq(arena: &[u8], term: &InternedTerm, lookup: &TermLookup<'_>) -> bool {
    match (term, lookup) {
        (InternedTerm::Iri(r), TermLookup::Iri(s)) => arena_str(arena, *r) == *s,
        (
            InternedTerm::Blank { label, scope },
            TermLookup::Blank {
                label: ls,
                scope: ss,
            },
        ) => arena_str(arena, *label) == *ls && scope == ss,
        (
            InternedTerm::Literal(lit),
            TermLookup::Literal {
                lexical,
                datatype,
                language,
                direction,
            },
        ) => {
            arena_str(arena, lit.lexical_form) == *lexical
                && lit.datatype == *datatype
                && lit.direction == *direction
                && lit.language.map(|r| arena_str(arena, r)) == *language
        }
        (
            InternedTerm::Triple { s, p, o },
            TermLookup::Triple {
                s: ls,
                p: lp,
                o: lo,
            },
        ) => s == ls && p == lp && o == lo,
        _ => false,
    }
}

/// Term storage + value-interning dedup + the C0 identity policy, in one cohesive
/// unit (SRP). Private: the builder is the only public surface.
#[derive(Debug)]
struct Interner {
    capacity: LocalCapacity,
    /// The byte arena owning every interned string ONCE (P3b); terms hold ranges.
    arena: Vec<u8>,
    /// Dense table of interned terms (range-backed); the sole structural owner.
    terms: Vec<InternedTerm>,
    /// Store-once value→id index: `u32` indices into `terms`, with hash/eq resolving
    /// ranges through the arena and comparing BY VALUE (so equal strings dedup to
    /// one id even though the table itself stores neither strings nor ranges).
    index: HashTable<u32>,
    /// The caller-supplied content-id recognition spelling (e.g. `"blake3:"`).
    /// `None` (the default) means recognition is INACTIVE: no fabricated default,
    /// per the no-vocabulary-minting policy. Fixed at construction time (see
    /// [`RdfDatasetBuilder::with_content_addressing`]) — there is no setter.
    content_scheme: Option<ContentIdScheme>,
    /// Side table from a recognized content-id term to its decoded
    /// [`Blake3ContentId`]. Empty while recognition is inactive; populated at
    /// intern time in the miss branch of [`Interner::intern`] (IRI arm only).
    content_ids: HashMap<TermId, Blake3ContentId, FastHasher>,
    /// Original physical charge; deletion changes map insertion headroom only.
    content_ids_bytes: usize,
    /// The FIRST IRI that failed the IR-boundary absoluteness invariant, with the
    /// typed reason. Recorded in the miss branch of [`Interner::intern`] (IRI arm
    /// only) and turned into a hard `Err` by [`super::validate::validate`], which
    /// [`RdfDatasetBuilder::freeze`] runs before any `RdfDataset` is materialized.
    ///
    /// Only the first is kept: `validate` reports the first violation for every
    /// other structural invariant too, and once one is recorded the dataset can
    /// never be frozen, so later ones cannot change the outcome.
    relative_iri: Option<(String, IriError)>,
    /// First refused literal, as `(diagnostic code, message)`, held for the
    /// existing validation boundary. The code is the grammar's own `langtag-*`
    /// when a language tag is malformed, and `model::LITERAL_SHAPE_CODE` when
    /// the datatype, language and direction merely disagree.
    invalid_literal: Option<(&'static str, &'static str)>,
}

impl Interner {
    fn ensure_reifies<S: Admission + ?Sized>(
        &mut self,
        predicate: &mut Option<TermId>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        if predicate.is_none() {
            *predicate = Some(
                self.intern_with_memory(TermLookup::Iri(purrdf_iri::vocab::rdf::REIFIES), memory)?,
            );
        }
        Ok(())
    }

    /// Transfer the original lexical allocations and reverse index unchanged.
    fn freeze_with_memory<S: Admission + ?Sized>(
        self,
        derivation_predicate: Option<String>,
        memory: &mut Memory<'_, S>,
    ) -> Result<super::frozen_terms::FrozenTerms, StorageError> {
        // Configuration does not intern a missing predicate at freeze.
        let derivation_predicate_iri = derivation_predicate
            .map(|text| -> Result<Box<str>, StorageError> {
                let exact = memory.string(&text)?;
                memory.release_string(text)?;
                Ok(exact.into_boxed_str())
            })
            .transpose()?;
        let derivation_predicate = derivation_predicate_iri
            .as_deref()
            .and_then(|iri| self.lookup_iri(iri));
        Ok(super::frozen_terms::FrozenTerms {
            arena: memory.boxed_slice(self.arena)?,
            terms: memory.boxed_slice(self.terms)?,
            term_index: self.index,
            content_ids: self.content_ids,
            content_scheme: self.content_scheme,
            derivation_predicate,
            derivation_predicate_iri,
        })
    }

    fn new() -> Self {
        Self {
            capacity: LocalCapacity::Native,
            arena: Vec::new(),
            terms: Vec::new(),
            index: HashTable::new(),
            content_scheme: None,
            content_ids: crate::FastMap::default(),
            content_ids_bytes: 0,
            relative_iri: None,
            invalid_literal: None,
        }
    }

    /// Append a string to the arena, returning its range.
    fn push_str_with_memory<S: Admission + ?Sized>(
        &mut self,
        s: &str,
        memory: &mut Memory<'_, S>,
    ) -> Result<StrRange, StorageError> {
        push_arena_str_with_memory(&mut self.arena, s, memory)
    }

    fn buffer_bytes(&self) -> Result<usize, StorageError> {
        let terms = std::alloc::Layout::array::<InternedTerm>(self.terms.capacity())
            .map_err(|_| StorageError::SizeOverflow)?
            .size();
        let index = crate::hash::hash_table_retained_bound(&self.index)
            .ok_or(StorageError::SizeOverflow)?;
        [
            self.arena.capacity(),
            terms,
            index,
            self.content_ids_bytes,
            self.relative_iri
                .as_ref()
                .map_or(0, |(iri, error)| iri.capacity() + error.owned_text_bytes()),
            self.content_scheme
                .as_ref()
                .map_or(0, ContentIdScheme::owned_text_bytes),
        ]
        .into_iter()
        .try_fold(0usize, |total, bytes| {
            total.checked_add(bytes).ok_or(StorageError::SizeOverflow)
        })
    }

    /// Intern a term BY VALUE: dedups against existing terms (resolving their ranges
    /// through the arena) and pushes the strings to the arena only on a MISS, so a
    /// duplicate value costs no arena bytes. Idempotent: equal values map to one id.
    ///
    /// # The absoluteness invariant is checked here, on the MISS path only
    ///
    /// The early return below is the HIT path: re-interning a string this table has
    /// already stored is a hash lookup and nothing else, so an IRI is validated for
    /// absoluteness exactly once — when it is first inserted — no matter how many
    /// times it is subsequently interned. See [`super::absolute`].
    fn intern(&mut self, lookup: TermLookup<'_>) -> Result<TermId, StorageError> {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.buffer_bytes()?);
        self.intern_with_memory(lookup, &mut memory)
    }
    fn intern_with_memory<S: Admission + ?Sized>(
        &mut self,
        lookup: TermLookup<'_>,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, StorageError> {
        let hash = hash_lookup_value(&lookup);
        if let Some(id) = self.lookup_hashed(hash, &lookup) {
            return Ok(id);
        }
        use purrdf_lex::allocation::CapacitySpace;
        let required_terms =
            self.capacity
                .required(CapacitySpace::LocalTerms, self.terms.len(), 1)?;
        let i = u32::try_from(self.terms.len()).map_err(|_| StorageError::SizeOverflow)?;
        let id = TermId::from_index(i)?;
        let text_bytes = match &lookup {
            TermLookup::Iri(text) | TermLookup::Blank { label: text, .. } => text.len(),
            TermLookup::Literal {
                lexical, language, ..
            } => lexical
                .len()
                .checked_add(language.map_or(0, str::len))
                .ok_or(StorageError::SizeOverflow)?,
            TermLookup::Triple { .. } => 0,
        };
        let required_bytes =
            self.capacity
                .required(CapacitySpace::LocalTermBytes, self.arena.len(), text_bytes)?;
        // Reserve every miss destination before recording even the first range.
        let capacity =
            purrdf_lex::allocation::growth_capacity::<u8>(self.arena.capacity(), required_bytes)?;
        memory.reserve(&mut self.arena, capacity)?;
        let capacity = purrdf_lex::allocation::growth_capacity::<InternedTerm>(
            self.terms.capacity(),
            required_terms,
        )?;
        memory.reserve(&mut self.terms, capacity)?;
        let (arena, terms) = (&self.arena, &self.terms);
        crate::hash::reserve_table_with_memory(
            &mut self.index,
            required_terms,
            |&i| hash_stored_value(arena, &terms[i as usize]),
            memory,
        )?;
        // Content-id recognition and its side table are part of the same miss.
        // Admit the side table before publishing either lexical bytes or a term;
        // a refused grant must not leave a term absent from the value index.
        let recognized: Option<Blake3ContentId> = match &lookup {
            TermLookup::Iri(iri) => self
                .content_scheme
                .as_ref()
                .and_then(|scheme| iri.strip_prefix(scheme.prefix()))
                .and_then(Blake3ContentId::from_hex),
            _ => None,
        };
        if recognized.is_some() {
            let required = self
                .content_ids
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            crate::hash::reserve_map_with_retained_memory(
                &mut self.content_ids,
                &mut self.content_ids_bytes,
                required,
                memory,
            )?;
        }
        // The IR-boundary absoluteness invariant (miss branch, IRI arm ONLY). A
        // violation cannot be reported from this signature, so it is recorded and
        // `super::validate::validate` turns it into the hard `Err` that
        // `RdfDatasetBuilder::freeze` returns — before any `RdfDataset` exists.
        // Skipped once a violation is already pending: the dataset can no longer be
        // frozen, so re-validating later IRIs could not change the outcome and would
        // only slow down the walk to the error.
        if let TermLookup::Iri(iri) = lookup
            && self.relative_iri.is_none()
            && let Err(err) = super::absolute::check_absolute(iri)
        {
            self.relative_iri = Some((memory.string(iri)?, err));
        }
        // Miss: now (and only now) push the strings to the arena and build the term.
        let term = match lookup {
            TermLookup::Iri(iri) => InternedTerm::Iri(self.push_str_with_memory(iri, memory)?),
            TermLookup::Blank { label, scope } => InternedTerm::Blank {
                label: self.push_str_with_memory(label, memory)?,
                scope,
            },
            TermLookup::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let lexical_form = self.push_str_with_memory(lexical, memory)?;
                let language = language
                    .map(|l| self.push_str_with_memory(l, memory))
                    .transpose()?;
                InternedTerm::Literal(InternedLiteral {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                })
            }
            TermLookup::Triple { s, p, o } => InternedTerm::Triple { s, p, o },
        };
        memory.push(&mut self.terms, term)?;
        if let Some(content_id) = recognized {
            self.content_ids.insert(id, content_id);
        }
        let (arena, terms) = (&self.arena, &self.terms);
        crate::hash::reserve_table_with_memory(
            &mut self.index,
            terms.len(),
            |&i| hash_stored_value(arena, &terms[i as usize]),
            memory,
        )?;
        self.index
            .insert_unique(hash, i, |&i| hash_stored_value(arena, &terms[i as usize]));
        Ok(id)
    }

    /// The decoded content id for a recognized term, if any (builder-level
    /// accessor for tests; the frozen-dataset public accessor is separate).
    #[cfg(test)]
    fn content_id(&self, id: TermId) -> Option<Blake3ContentId> {
        self.content_ids.get(&id).copied()
    }

    /// Look up the [`TermId`] of an already-interned IRI, WITHOUT interning a new
    /// term on a miss. Mirrors the hit path of [`Interner::intern`]'s IRI arm
    /// exactly (same hash, same `term_eq` comparison), so it agrees with `intern`
    /// on every IRI that has actually been interned.
    ///
    /// Used at [`RdfDatasetBuilder::materialize`] to resolve the configured
    /// derivation-predicate IRI to a frozen `TermId` without minting a new term
    /// after the arena/term table are otherwise frozen-bound: terms are frozen at
    /// that point, so a miss here must return `None`, never insert.
    fn lookup_iri(&self, iri: &str) -> Option<TermId> {
        let lookup = TermLookup::Iri(iri);
        self.lookup_hashed(hash_lookup_value(&lookup), &lookup)
    }

    fn lookup_hashed(&self, hash: u64, lookup: &TermLookup<'_>) -> Option<TermId> {
        let (arena, terms) = (&self.arena, &self.terms);
        self.index
            .find(hash, |&i| term_eq(arena, &terms[i as usize], lookup))
            .copied()
            .map(|index| TermId::from_index(index).expect("stored term index is representable"))
    }

    fn term(&self, id: TermId) -> &InternedTerm {
        &self.terms[id.index()]
    }

    fn lookup_owned_blank(&self, token: &str) -> Option<TermId> {
        let key = crate::blank_label::OwnedBlankKey::new(token);
        if let Some(label) = key.borrowed() {
            let lookup = TermLookup::Blank {
                label,
                scope: key.scope(),
            };
            return self.lookup_hashed(hash_lookup_value(&lookup), &lookup);
        }
        let hash =
            crate::hash::hash_blank_bytes_for_interner(key.len(), key.bytes(), key.scope().0);
        self.index
            .find(hash, |&index| {
                let InternedTerm::Blank { label, scope } = self.terms[index as usize] else {
                    return false;
                };
                scope == key.scope() && key.bytes().eq(arena_str(&self.arena, label).bytes())
            })
            .copied()
            .map(|index| TermId::from_index(index).expect("stored term index is representable"))
    }

    /// The byte arena backing this interner's term ranges (for the few build-time
    /// readers that resolve a term's string before freeze).
    fn arena(&self) -> &[u8] {
        &self.arena
    }

    fn term_count(&self) -> usize {
        self.terms.len()
    }
}

/// The fallible builder that interns terms, accumulates structure, and freezes
/// into an immutable `Arc<RdfDataset>`.
///
/// Pushed structure is accumulated in deterministic insertion order; quads and
/// annotations are deduplicated *during* push (the dataset is a set, C0.5), while
/// [`freeze`](RdfDatasetBuilder::freeze) re-sorts everything into a stable,
/// reproducible order.
#[derive(Debug)]
pub struct RdfDatasetBuilder {
    /// Owns terms + the value-intern index + the C0 identity policy.
    interner: Interner,
    /// Deduplicated quad rows in first-seen order; `g == None` is the default graph.
    /// The **sole** owner of each row; `quad_index` holds only indices (P3c).
    quads: Vec<QuadRow>,
    /// Store-once dedup index into `quads` (replaces the duplicate `HashSet<QuadRow>`).
    quad_index: HashTable<u32>,
    /// `(reifier, triple-term, graph)` bindings. Several reifiers MAY bind one triple
    /// term and the same binding MAY be pushed more than once; duplicates collapse
    /// (C0.4). The `graph` slot (`None` = default graph) records the named graph the
    /// reifier declaration was asserted in, so a reifier inside a TriG `GRAPH g { … }`
    /// block is matchable under `GRAPH ?g`.
    reifiers: Vec<(TermId, TermId, Option<TermId>)>,
    reifier_index: HashTable<u32>,
    /// `(reifier, predicate, object, graph)` annotations; duplicates collapse (C0.5).
    /// The `graph` slot mirrors [`Self::reifiers`].
    annotations: Vec<(TermId, TermId, TermId, Option<TermId>)>,
    annotation_index: HashTable<u32>,
    /// Sparse source locations keyed by the pushed-quad ordinal. Only quads with a
    /// recorded location appear here.
    locations: Vec<(QuadHandle, RdfLocation)>,
    /// Named graphs EXPLICITLY declared to exist even with zero quads (see
    /// [`declare_named_graph`](Self::declare_named_graph)). Deduplicated at freeze
    /// alongside every graph term that DOES own a quad.
    declared_graphs: Vec<TermId>,
    /// Counter for the next blank-node scope to use when merging a foreign dataset
    /// via [`push_dataset`](RdfDatasetBuilder::push_dataset). Each call to
    /// `push_dataset` claims one fresh scope on its first blank (starting at 1;
    /// 0 = DEFAULT) so that
    /// blank nodes from different source datasets can never collide even when their
    /// labels are identical (standardize-apart, C0.2).
    next_merge_scope: u64,
    /// Explicit and imported blank scopes reserved against independent merges.
    used_scopes: Vec<BlankScope>,
    /// Scope insertions made by an active compound operation, in publication order.
    scope_journal: Vec<BlankScope>,
    /// Nested compound operations share the same undo journal.
    transaction_depth: usize,
    /// The caller-supplied predicate IRI that marks a derivation edge between a
    /// content-addressed term and the term(s) it was derived from. `None` (the
    /// default) means no derivation predicate is configured — no fabricated
    /// default, per the no-vocabulary-minting policy. Fixed at construction time
    /// (see [`RdfDatasetBuilder::with_content_addressing`]) — there is no setter.
    derivation_predicate: Option<String>,
    /// The `rdf:reifies` predicate id, minted on the first reifier push and then
    /// answered from here: the interner never forgets a term, so the id stays
    /// valid for the builder's whole life, and every later push skips the
    /// 47-byte hash + probe + compare a by-string re-intern would pay.
    reifies_predicate: Option<TermId>,
}

/// A dataset builder whose structural validation has already passed.
///
/// This type-state splits validation from materialization for callers that need to
/// make that phase boundary explicit, while [`RdfDatasetBuilder::freeze`] keeps the
/// traditional one-shot validate-then-freeze API.
#[derive(Debug)]
pub struct ValidatedRdfDatasetBuilder {
    inner: RdfDatasetBuilder,
}

/// Scalar savepoint; historical terms, rows and scopes are never copied.
struct BuilderCheckpoint {
    terms: usize,
    arena: usize,
    quads: usize,
    reifiers: usize,
    annotations: usize,
    locations: usize,
    graphs: usize,
    journal: usize,
    depth: usize,
    next_merge_scope: u64,
    reifies_predicate: Option<TermId>,
    relative_iri: bool,
    invalid_literal: Option<(&'static str, &'static str)>,
}

impl ValidatedRdfDatasetBuilder {
    /// Materialize validated native storage while retaining the original Memory.
    ///
    /// # Errors
    /// Returns physical storage refusal before the corresponding allocation.
    pub fn freeze_with_memory<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<RdfDataset, StorageError> {
        self.inner.materialize_with_memory(memory)
    }

    /// Number of interned terms in this validated, unfrozen graph.
    #[must_use]
    pub fn term_count(&self) -> usize {
        self.inner.term_count()
    }

    /// Resolve one staged term without materializing a dataset.
    #[must_use]
    pub fn resolve(&self, id: TermId) -> TermRef<'_> {
        self.inner.resolve(id)
    }

    /// Number of distinct statements, including both RDF 1.2 overlay tables.
    #[must_use]
    pub fn statement_count(&self) -> usize {
        self.inner.statement_count()
    }

    /// Bytes occupied by the staged term payload, excluding indexes and rows.
    #[must_use]
    pub fn payload_bytes(&self) -> usize {
        self.inner.interner.arena.len()
    }

    /// Materialize the already-validated builder into an immutable dataset.
    ///
    /// # Errors
    /// Returns checked layout or physical buffer allocation refusal.
    pub fn freeze(self) -> Result<Arc<RdfDataset>, StorageError> {
        self.inner.materialize().map(Arc::new)
    }
}

purrdf_hash::default_from_new!(RdfDatasetBuilder);

/// A native graph build refuses physical storage separately from RDF structure.
#[derive(Debug)]
pub enum NativeBuildError {
    /// Checked layout, original admission or physical allocator refusal.
    Storage(StorageError),
    /// The original RDF diagnostic; its producer's Memory retains its buffers.
    Diagnostic(RdfDiagnostic),
}
impl purrdf_hash::error::StructuredError for NativeBuildError {
    purrdf_hash::error_visit! {
        Storage(cause) => "native-build-storage" { "cause" => Cause(cause) },
        Diagnostic(cause) => "native-build-diagnostic" { "cause" => Cause(cause) },
    }
}
purrdf_lex::variant_from!(NativeBuildError { Storage(StorageError) });
purrdf_lex::variant_from!(NativeBuildError { Diagnostic(RdfDiagnostic) });
impl core::fmt::Display for NativeBuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Storage(error) => core::fmt::Display::fmt(error, f),
            Self::Diagnostic(error) => core::fmt::Display::fmt(error, f),
        }
    }
}
impl std::error::Error for NativeBuildError {}

impl From<NativeBuildError> for RdfDiagnostic {
    fn from(error: NativeBuildError) -> Self {
        match error {
            NativeBuildError::Storage(error) => error.into(),
            NativeBuildError::Diagnostic(diagnostic) => diagnostic,
        }
    }
}

impl RdfDatasetBuilder {
    fn transaction_with_memory<T, S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
        body: impl FnOnce(&mut Self, &mut Memory<'_, S>) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        self.transaction_try_with_memory(memory, body, |_| Ok(0))
    }

    fn transaction_try_with_memory<T, E: From<StorageError>, S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
        body: impl FnOnce(&mut Self, &mut Memory<'_, S>) -> Result<T, E>,
        error_bytes: impl FnOnce(&E) -> Result<usize, StorageError>,
    ) -> Result<T, E> {
        let buffers = self.transaction_buffer_bytes()?;
        let admitted = memory.admitted_bytes();
        let checkpoint = BuilderCheckpoint {
            terms: self.interner.terms.len(),
            arena: self.interner.arena.len(),
            quads: self.quads.len(),
            reifiers: self.reifiers.len(),
            annotations: self.annotations.len(),
            locations: self.locations.len(),
            graphs: self.declared_graphs.len(),
            journal: self.scope_journal.len(),
            depth: self.transaction_depth,
            next_merge_scope: self.next_merge_scope,
            reifies_predicate: self.reifies_predicate,
            relative_iri: self.interner.relative_iri.is_some(),
            invalid_literal: self.interner.invalid_literal,
        };
        self.transaction_depth = self
            .transaction_depth
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        let result = body(self, memory);
        self.transaction_depth = checkpoint.depth;
        if let Err(error) = &result {
            for index in checkpoint.terms..self.interner.terms.len() {
                let hash = hash_stored_value(&self.interner.arena, &self.interner.terms[index]);
                if let Ok(entry) = self
                    .interner
                    .index
                    .find_entry(hash, |&stored| stored as usize == index)
                {
                    entry.remove();
                }
                let id =
                    TermId::from_index(u32::try_from(index).expect("stored term index fits u32"))
                        .expect("stored term index is representable");
                self.interner.content_ids.remove(&id);
            }
            self.interner.terms.truncate(checkpoint.terms);
            self.interner.arena.truncate(checkpoint.arena);
            remove_index_suffix(&self.quads, &mut self.quad_index, checkpoint.quads);
            remove_index_suffix(&self.reifiers, &mut self.reifier_index, checkpoint.reifiers);
            remove_index_suffix(
                &self.annotations,
                &mut self.annotation_index,
                checkpoint.annotations,
            );
            self.quads.truncate(checkpoint.quads);
            self.reifiers.truncate(checkpoint.reifiers);
            self.annotations.truncate(checkpoint.annotations);
            self.locations.truncate(checkpoint.locations);
            self.declared_graphs.truncate(checkpoint.graphs);
            for scope in self.scope_journal.drain(checkpoint.journal..) {
                let position = self
                    .used_scopes
                    .binary_search(&scope)
                    .expect("journaled scope is still present until rollback");
                self.used_scopes.remove(position);
            }
            self.next_merge_scope = checkpoint.next_merge_scope;
            self.reifies_predicate = checkpoint.reifies_predicate;
            self.interner.invalid_literal = checkpoint.invalid_literal;
            if !checkpoint.relative_iri {
                self.interner.relative_iri = None;
            }
            // Growth buffers still belong to this builder. Only dead scratch and
            // rolled-back owned payload are refunded; never refund retained capacity.
            // As in Memory::try_scope, cleanup failure cannot replace the first cause.
            let _ = (|| {
                let retained = self
                    .transaction_buffer_bytes()?
                    .checked_sub(buffers)
                    .ok_or(StorageError::SizeOverflow)?;
                let diagnostic = error_bytes(error)?;
                let live = admitted
                    .checked_add(retained)
                    .and_then(|live| live.checked_add(diagnostic))
                    .ok_or(StorageError::SizeOverflow)?;
                let released = memory
                    .admitted_bytes()
                    .checked_sub(live)
                    .ok_or(StorageError::SizeOverflow)?;
                memory.release_bytes(released)
            })();
        }
        if checkpoint.depth == 0 {
            self.scope_journal.clear();
        }
        result
    }

    /// Append already-interned local quads through the same checked store-once
    /// path as individual insertion. Foreign IDs must first be imported.
    ///
    /// # Errors
    /// Returns the original row capacity or physical storage refusal. Duplicate
    /// rows remain legal even at capacity because only actual misses mint IDs.
    pub fn extend_quads(
        &mut self,
        rows: impl IntoIterator<Item = QuadIds>,
    ) -> Result<(), StorageError> {
        for row in rows {
            self.push_quad(row.s, row.p, row.o, row.g)?;
        }
        Ok(())
    }

    /// Intern borrowed values while retaining their original allocation grant.
    ///
    /// # Errors
    /// Returns physical layout, admission or allocator refusal.
    pub fn intern_value_with_memory<S: Admission + ?Sized>(
        &mut self,
        value: &super::term::TermValue,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, StorageError> {
        crate::term_fixture::intern_value_with_memory(self, value, memory)
    }

    /// Native borrowed IRI ingress.
    ///
    /// # Errors
    /// Returns physical storage refusal before any unadmitted allocation.
    pub fn intern_iri_with_memory<S: Admission + ?Sized>(
        &mut self,
        iri: &str,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, StorageError> {
        self.interner
            .intern_with_memory(TermLookup::Iri(iri), memory)
    }

    /// Native already-bound blank ingress.
    ///
    /// # Errors
    /// Returns physical storage refusal before buffer growth.
    pub fn intern_blank_with_memory<S: Admission + ?Sized>(
        &mut self,
        label: &str,
        scope: BlankScope,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, StorageError> {
        let position = self.reserve_scope_with_memory(scope, memory)?;
        let id = self
            .interner
            .intern_with_memory(TermLookup::Blank { label, scope }, memory)?;
        if let Some(position) = position {
            self.publish_scope(position, scope);
        }
        Ok(id)
    }

    /// Native triple ingress, sharing the original interning body.
    ///
    /// # Errors
    /// Returns physical storage refusal before buffer growth.
    pub fn intern_triple_with_memory<S: Admission + ?Sized>(
        &mut self,
        s: TermId,
        p: TermId,
        o: TermId,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, StorageError> {
        self.interner
            .intern_with_memory(TermLookup::Triple { s, p, o }, memory)
    }

    /// Insert a native quad at the original store-once producer.
    ///
    /// # Errors
    /// Returns physical storage refusal before buffer growth.
    pub fn push_quad_with_memory<S: Admission + ?Sized>(
        &mut self,
        s: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        store_once_with_memory(
            &mut self.quads,
            &mut self.quad_index,
            QuadRow { s, p, o, g },
            self.interner.capacity,
            CapacitySpace::Quads,
            memory,
        )?;
        Ok(())
    }

    /// Insert a native RDF 1.2 reifier binding.
    ///
    /// # Errors
    /// Returns physical storage refusal before buffer growth.
    pub fn push_reifier_in_graph_with_memory<S: Admission + ?Sized>(
        &mut self,
        reifier: TermId,
        triple: TermId,
        graph: Option<TermId>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let interner = &mut self.interner;
        let predicate = &mut self.reifies_predicate;
        store_once_with_dependency(
            &mut self.reifiers,
            &mut self.reifier_index,
            (reifier, triple, graph),
            interner.capacity,
            CapacitySpace::Reifiers,
            memory,
            |memory| interner.ensure_reifies(predicate, memory),
        )?;
        Ok(())
    }

    /// Insert a native RDF 1.2 annotation.
    ///
    /// # Errors
    /// Returns physical storage refusal before buffer growth.
    pub fn push_annotation_in_graph_with_memory<S: Admission + ?Sized>(
        &mut self,
        reifier: TermId,
        predicate: TermId,
        object: TermId,
        graph: Option<TermId>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        store_once_with_memory(
            &mut self.annotations,
            &mut self.annotation_index,
            (reifier, predicate, object, graph),
            self.interner.capacity,
            CapacitySpace::Annotations,
            memory,
        )?;
        Ok(())
    }

    /// Native literal identity and original arena allocation. Borrowed lexical
    /// spelling remains verbatim; the shared language fold changes only its key.
    ///
    /// # Errors
    /// Returns physical storage refusal before buffer growth.
    pub fn intern_literal_parts_with_memory<S: Admission + ?Sized>(
        &mut self,
        lexical: &str,
        datatype: Option<&str>,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, StorageError> {
        self.transaction_with_memory(memory, |builder, memory| {
            builder.intern_literal_parts_admitted(lexical, datatype, language, direction, memory)
        })
    }

    fn intern_literal_parts_admitted<S: Admission + ?Sized>(
        &mut self,
        lexical: &str,
        datatype: Option<&str>,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, StorageError> {
        let datatype_iri = match (language, datatype) {
            (Some(_), _) => RdfLiteral::language_datatype_iri(direction),
            (None, Some(explicit)) => explicit,
            (None, None) => super::term::XSD_STRING,
        };
        let invalid_literal = self
            .interner
            .invalid_literal
            .or_else(|| RdfLiteral::diagnose_components(datatype_iri, language, direction).err());
        let datatype_id = self.intern_iri_with_memory(datatype_iri, memory)?;
        let language = language
            .map(|tag| super::term::interned_language_with_memory(tag, memory))
            .transpose()?;
        let lookup = TermLookup::Literal {
            lexical,
            datatype: datatype_id,
            language: language.as_deref(),
            direction,
        };
        if let Some(id) = self
            .interner
            .lookup_hashed(hash_lookup_value(&lookup), &lookup)
        {
            if let Some(std::borrow::Cow::Owned(language)) = language {
                memory.release_string(language)?;
            }
            return Ok(id);
        }
        let blanks =
            crate::cdt_blank::cdt_embedded_blanks_with_memory(lexical, datatype_iri, memory)?;
        for (label, scope) in &blanks {
            self.intern_blank_with_memory(label, *scope, memory)?;
        }
        let bytes = blanks.iter().try_fold(0usize, |bytes, (label, _)| {
            bytes
                .checked_add(label.capacity())
                .ok_or(StorageError::SizeOverflow)
        })?;
        memory.release_vec(blanks)?;
        memory.release_bytes(bytes)?;
        let result = self.interner.intern_with_memory(
            TermLookup::Literal {
                lexical,
                datatype: datatype_id,
                language: language.as_deref(),
                direction,
            },
            memory,
        )?;
        if let Some(std::borrow::Cow::Owned(language)) = language {
            memory.release_string(language)?;
        }
        // A refused literal is not part of the graph. Publish its validation
        // state only once its identity and dependencies have been admitted.
        self.interner.invalid_literal = invalid_literal;
        Ok(result)
    }

    // Resident wrappers resume only their own resident buffers. Operational
    // callers supply Memory from birth and never certify an already-built tree.
    pub(crate) fn interner_buffer_bytes(&self) -> Result<usize, StorageError> {
        self.interner
            .buffer_bytes()?
            .checked_add(self.used_scopes.capacity() * size_of::<BlankScope>())
            .and_then(|bytes| {
                bytes.checked_add(self.scope_journal.capacity() * size_of::<BlankScope>())
            })
            .ok_or(StorageError::SizeOverflow)
    }

    /// A fresh, empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            interner: Interner::new(),
            quads: Vec::new(),
            quad_index: HashTable::new(),
            reifiers: Vec::new(),
            reifier_index: HashTable::new(),
            annotations: Vec::new(),
            annotation_index: HashTable::new(),
            locations: Vec::new(),
            declared_graphs: Vec::new(),
            // Merge scopes start at 1; scope 0 is BlankScope::DEFAULT (local pushes).
            next_merge_scope: 1,
            used_scopes: Vec::new(),
            scope_journal: Vec::new(),
            transaction_depth: 0,
            derivation_predicate: None,
            reifies_predicate: None,
        }
    }

    /// A fresh builder configured for content addressing: `scheme` marks which
    /// IRIs are content-id term references and `derivation_predicate`, if given,
    /// is the predicate IRI used to record derivation edges.
    ///
    /// This is the **single** construction path for content addressing (§19
    /// one-path): the config is fixed here, before any term is interned, and
    /// deliberately has no setter — a builder's content-addressing config never
    /// changes mid-build.
    #[must_use]
    pub fn with_content_addressing(
        scheme: ContentIdScheme,
        derivation_predicate: Option<String>,
    ) -> Self {
        let mut builder = Self::new();
        builder.interner.content_scheme = Some(scheme);
        builder.derivation_predicate = derivation_predicate;
        builder
    }

    /// Intern an IRI term. Idempotent: the same IRI string yields the same id.
    pub fn intern_iri(&mut self, iri: &str) -> Result<TermId, StorageError> {
        self.interner.intern(TermLookup::Iri(iri))
    }

    /// Reserve room for a bulk replay of `terms` distinct terms whose strings occupy
    /// `term_bytes` bytes, plus `quads` base quads, so the string arena, the term
    /// table, the value index, the quad table and the quad index each grow ONCE
    /// instead of doubling their way through the replay.
    ///
    /// Every figure is a hint, never a contract: pushing more or fewer of anything is
    /// correct and costs only the ordinary growth. Deliberately not public — the
    /// caller that can state these numbers honestly is a whole-view import
    /// ([`super::import::DatasetImporter`]), which reads all three off the source
    /// view's own `term_count`/`term_bytes_hint`/`len_hint`, and a public `reserve`
    /// would invite callers to guess.
    ///
    /// `term_bytes` of `0` means "no figure available" and reserves no arena at all,
    /// rather than reserving a fabricated one: an over-reserved arena is retained for
    /// the frozen dataset's entire lifetime, so guessing here costs memory forever
    /// while guessing wrong on the tables costs one extra growth.
    pub(crate) fn reserve_for_replay(
        &mut self,
        terms: usize,
        term_bytes: usize,
        quads: usize,
    ) -> Result<(), StorageError> {
        let required_terms = self.interner.capacity.hint(
            CapacitySpace::LocalTerms,
            self.interner.terms.len(),
            terms,
        );
        let required_bytes = self.interner.capacity.hint(
            CapacitySpace::LocalTermBytes,
            self.interner.arena.len(),
            term_bytes,
        );
        let mut resident = Resident;
        let bytes = self.native_buffer_bytes()?;
        let mut memory = Memory::resume(&mut resident, bytes);
        let Interner {
            arena,
            terms: table,
            index,
            ..
        } = &mut self.interner;
        memory.reserve(arena, required_bytes)?;
        memory.reserve(table, required_terms)?;
        crate::hash::reserve_table_with_memory(
            index,
            required_terms,
            |&i| hash_stored_value(arena, &table[i as usize]),
            &mut memory,
        )?;
        reserve_store_once(
            &mut self.quads,
            &mut self.quad_index,
            quads,
            self.interner.capacity,
            CapacitySpace::Quads,
        )
    }

    /// Explicitly declare that a named graph exists, even if it turns out to own
    /// zero quads. The frozen dataset's `GRAPH ?g` enumeration
    /// ([`RdfDataset::named_graphs`](super::dataset::RdfDataset::named_graphs))
    /// is the union of this declaration list and every graph term that DOES own a
    /// quad — everything else in the engine (quad matching, `CREATE`/`CLEAR`/`DROP
    /// GRAPH`, capability flags) keeps the ordinary "a graph exists iff it holds a
    /// quad" doctrine untouched. Idempotent (deduplicated at freeze); `g` must be a
    /// [`TermId`] already interned in THIS builder (typically via
    /// [`intern_iri`](Self::intern_iri)).
    ///
    /// # Errors
    /// Refuses physical capacity before publishing a new declaration slot.
    pub fn declare_named_graph(&mut self, g: TermId) -> Result<(), StorageError> {
        let mut resident = Resident;
        let bytes = self.native_buffer_bytes()?;
        let mut memory = Memory::resume(&mut resident, bytes);
        self.declare_named_graph_with_memory(g, &mut memory)
    }

    /// Declare a graph under the original builder allocation authority.
    ///
    /// # Errors
    /// Returns the original layout, admission or allocator refusal before growth.
    pub fn declare_named_graph_with_memory<S: Admission + ?Sized>(
        &mut self,
        g: TermId,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        // Freeze already deduplicates declarations. At a growth boundary an
        // existing declaration needs no new slot, so duplicates remain legal
        // under a closed grant. Only growth boundaries scan: distinct bulk
        // insertion retains geometric growth and amortized linear work.
        if self.declared_graphs.len() == self.declared_graphs.capacity()
            && self.declared_graphs.contains(&g)
        {
            return Ok(());
        }
        memory.push(&mut self.declared_graphs, g)
    }

    /// Intern a blank node. Identity is `(label, scope)` (C0.2): same label + same
    /// scope → same id; same label + different scope → different id.
    pub fn intern_blank(&mut self, label: &str, scope: BlankScope) -> Result<TermId, StorageError> {
        let mut resident = Resident;
        let bytes = self.native_buffer_bytes()?;
        let mut memory = Memory::resume(&mut resident, bytes);
        self.intern_blank_with_memory(label, scope, &mut memory)
    }

    // Reserve only storage. Callers publish a missing scope after the operation
    // that needs it succeeds, so a refused blank term cannot consume a scope.
    fn reserve_scope_with_memory<S: Admission + ?Sized>(
        &mut self,
        scope: BlankScope,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<usize>, StorageError> {
        if scope == BlankScope::DEFAULT {
            return Ok(None);
        }
        if let Err(position) = self.used_scopes.binary_search(&scope) {
            let required = self
                .used_scopes
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            if required > self.used_scopes.capacity() {
                let capacity = required.max(
                    self.used_scopes
                        .capacity()
                        .checked_mul(2)
                        .ok_or(StorageError::SizeOverflow)?,
                );
                memory.reserve(&mut self.used_scopes, capacity)?;
            }
            if self.transaction_depth != 0 {
                let required = self
                    .scope_journal
                    .len()
                    .checked_add(1)
                    .ok_or(StorageError::SizeOverflow)?;
                let capacity = purrdf_lex::allocation::growth_capacity::<BlankScope>(
                    self.scope_journal.capacity(),
                    required,
                )?;
                memory.reserve(&mut self.scope_journal, capacity)?;
            }
            return Ok(Some(position));
        }
        Ok(None)
    }

    fn publish_scope(&mut self, position: usize, scope: BlankScope) {
        if self.transaction_depth != 0 {
            // reserve_scope_with_memory admitted this exact undo slot before the term.
            self.scope_journal.push(scope);
        }
        self.used_scopes.insert(position, scope);
    }

    /// Borrow every blank identity currently interned by this builder.
    /// Includes unused terms, so a publication boundary can choose a mint
    /// namespace disjoint from every identity the destination already owns.
    pub fn blank_identities(&self) -> impl Iterator<Item = (&str, BlankScope)> + '_ {
        self.interner.terms.iter().filter_map(|term| match term {
            InternedTerm::Blank { label, scope } => Some((self.interned_str(*label), *scope)),
            _ => None,
        })
    }

    /// Intern a blank node from a **text codec's** parsed `_:` token, decoding the
    /// egress encoding the serializers applied on the way out.
    ///
    /// This is the ingress choke point every native text codec (Turtle family,
    /// RDF/XML, TriX, HexTuples, JSON-LD / YAML-LD) routes its blank-node tokens
    /// through. The token is decoded by
    /// [`decode_blank_label`](crate::blank_label::decode_blank_label) — the exact
    /// inverse of [`encode_blank_label`](crate::blank_label::encode_blank_label)
    /// — so a document this workspace wrote re-parses to the very
    /// `(label, scope)` pair it was written from, and a token no serializer could
    /// have written (which is every token in a foreign document) is interned
    /// VERBATIM at [`BlankScope::DEFAULT`], byte for byte.
    ///
    /// `alphabet` is the label alphabet of the syntax the token was READ FROM —
    /// `BLANK_NODE_LABEL` for the Turtle family / HexTuples / JSON-LD, `NCName`
    /// for RDF/XML's `rdf:nodeID`, XML text for TriX's `<id>`. It is what the
    /// decode's image test re-encodes against, so ingress inverts exactly the
    /// egress that syntax performs and nothing else.
    ///
    /// Carrier formats (GTS, columnar, pack, OKF) deliberately do NOT use this:
    /// they store the IR's `(label, scope)` pair structurally, so their labels
    /// were never encoded and must stay opaque.
    pub fn intern_text_blank(
        &mut self,
        token: &str,
        alphabet: LabelAlphabet,
    ) -> Result<TermId, StorageError> {
        let (label, scope) = crate::blank_label::decode_blank_label(token, alphabet);
        self.intern_blank(&label, scope)
    }

    /// Intern a literal, applying the C0.1 identity policy:
    ///
    /// - A language tag → datatype `rdf:dirLangString` with a base direction,
    ///   otherwise `rdf:langString`; the language is lowercased for the key.
    /// - Otherwise an explicit datatype → that datatype.
    /// - Otherwise → `xsd:string`.
    ///
    /// The datatype is always stored as an interned IRI [`TermId`]. The lexical
    /// form is preserved byte-for-byte; base direction participates in identity.
    /// Invalid language/datatype/direction combinations are recorded and refused
    /// by [`validate`](Self::validate) or [`freeze`](Self::freeze), preserving this
    /// method's infallible builder interface without publishing invalid RDF.
    ///
    /// # A composite literal also interns the blank nodes it names
    ///
    /// A `cdt:List` / `cdt:Map` lexical form is not opaque text: its
    /// `BLANK_NODE_LABEL` tokens denote blank nodes of the enclosing document
    /// (see [`crate::cdt_blank`]). Every `(label, scope)` pair such a literal
    /// names is therefore interned here, exactly as though the label had been
    /// written as a bare term, so the pair always resolves through
    /// [`term_id_by_blank`](super::dataset::RdfDataset::term_id_by_blank) on the
    /// frozen dataset. That invariant is what lets the canonicalizer, the
    /// whole-dataset rewriters and the SPARQL evaluator's element accessors hand
    /// back a REAL blank node of the dataset rather than a dangling label.
    ///
    /// The lexical form is expected to be already BOUND — the labels spell their
    /// `(label, scope)` pairs. Ingress paths get that by routing through
    /// [`intern_literal_bound`](Self::intern_literal_bound); this entry point is
    /// the post-binding interner every other caller uses.
    ///
    /// The scan is guarded by the datatype IRI, so an ordinary literal pays two
    /// string comparisons and nothing more.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the by-value `RdfLiteral` is this method's PUBLISHED signature and every \
                  caller in and outside the workspace builds one inline, so narrowing it to a \
                  borrow is a breaking change, not a cleanup. The body stopped needing \
                  ownership when the work moved to `intern_literal_parts`, which borrows — the \
                  interner copies a literal's strings into its own arena either way, so \
                  consuming the literal here would free it a few lines earlier and nothing else"
    )]
    pub fn intern_literal(&mut self, lit: RdfLiteral) -> Result<TermId, StorageError> {
        self.intern_literal_parts(
            &lit.lexical_form,
            lit.datatype.as_deref(),
            lit.language.as_deref(),
            lit.direction,
        )
    }

    /// [`intern_literal`](Self::intern_literal) over BORROWED components — the same
    /// C0.1 identity policy, the same id, and no owned [`RdfLiteral`] in between.
    ///
    /// The interner's key ([`TermLookup`]) is borrowed and its arena is the only place
    /// a literal's strings are ever stored, so a caller that already holds `&str`s —
    /// a bulk import replaying another store's dictionary
    /// ([`super::import::DatasetImporter`]) — was allocating an owned lexical form, an
    /// owned datatype IRI and an owned language tag per literal purely to hand them to
    /// a function that borrows them straight back. This is that function, and
    /// `intern_literal` is now a thin owned wrapper over it, so the two cannot
    /// disagree about identity.
    ///
    /// `datatype` is the literal's EXPLICIT datatype (`None` meaning "unstated"),
    /// exactly as [`RdfLiteral::datatype`] carries it; the language-tag override and
    /// the `xsd:string` default are applied here, so both entry points expand the
    /// datatype the same way rather than each spelling the rule.
    ///
    /// The language tag is folded for the key ([`purrdf_iri::langtag::identity_fold`]) ONLY when
    /// it is not already its own fold. BCP 47 tags are case-insensitive and ingress
    /// normalizes them, so the common case is a tag that is already folded and a
    /// fold whose output is a copy of its input. The LOOKUP path
    /// (`RdfDataset::term_id_by_literal`) folds with the same function on purpose: a
    /// lookup that folded tags differently from the interner would report a term
    /// absent that this dataset holds.
    ///
    /// Crate-internal: the owned form is the published ingress, and a second public
    /// spelling of the same operation would be surface with no caller.
    pub(crate) fn intern_literal_parts(
        &mut self,
        lexical: &str,
        datatype: Option<&str>,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
    ) -> Result<TermId, StorageError> {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.interner_buffer_bytes()?);
        self.intern_literal_parts_with_memory(lexical, datatype, language, direction, &mut memory)
    }

    /// Intern a literal read from a document, binding the blank-node labels a
    /// composite (`cdt:List` / `cdt:Map`) lexical form embeds into that
    /// document's blank-node scope.
    ///
    /// This is the ingress choke point for literals, the twin of
    /// [`intern_text_blank`](Self::intern_text_blank) for bare blank terms.
    /// `binding` must be the SAME rule the path applies to its bare `_:` tokens
    /// — [`BlankBinding::Decoded`](crate::cdt_blank::BlankBinding::Decoded) with the syntax's alphabet for a text codec,
    /// [`BlankBinding::Ambient`](crate::cdt_blank::BlankBinding::Ambient) with the source's fixed scope for a carrier —
    /// because that agreement is precisely what makes `_:b` written as a subject
    /// and `_:b` written inside a composite literal the same node
    /// (`bnodes-turtle-05`), and what keeps the same label in two different
    /// documents two different nodes (`bnodes-turtle-15`).
    ///
    /// The binding is applied exactly ONCE, here, and never again: an
    /// already-bound literal re-interned through
    /// [`intern_literal`](Self::intern_literal) keeps its bytes.
    ///
    /// # Errors
    /// [`CdtBlankError`](crate::cdt_blank::CdtBlankError) when the literal claims a composite datatype but its
    /// lexical form is not a well-formed value of it, or breaks one of
    /// `purrdf-cdt`'s resource limits. The caller must refuse the whole document:
    /// an unparseable composite literal leaves the document's blank-node scope
    /// undefined, so there is no safe way to admit it. Never fails for any other
    /// datatype.
    pub fn intern_literal_bound(
        &mut self,
        lit: RdfLiteral,
        binding: crate::cdt_blank::BlankBinding,
    ) -> Result<TermId, crate::cdt_blank::CdtBlankError> {
        let mut resident = Resident;
        let bytes = self.native_buffer_bytes()?;
        let result = self.intern_literal_bound_with_memory(
            &lit,
            binding,
            &mut Memory::resume(&mut resident, bytes),
        );
        drop(lit);
        result
    }

    /// Validate and intern a document literal under its original physical owner.
    /// The returned diagnostic retains its own admitted payload until destroyed.
    /// # Errors
    /// Preserves malformed CDT and storage causes, with prior builder state intact.
    pub fn intern_literal_bound_with_memory<S: Admission + ?Sized>(
        &mut self,
        lit: &RdfLiteral,
        binding: crate::cdt_blank::BlankBinding,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, crate::cdt_blank::CdtBlankError> {
        self.transaction_try_with_memory(
            memory,
            |builder, memory| builder.intern_literal_bound_admitted(lit, binding, memory),
            crate::cdt_blank::CdtBlankError::owned_bytes,
        )
    }

    fn intern_literal_bound_admitted<S: Admission + ?Sized>(
        &mut self,
        lit: &RdfLiteral,
        binding: crate::cdt_blank::BlankBinding,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, crate::cdt_blank::CdtBlankError> {
        // The C0.1 datatype the literal will actually carry: a language tag wins
        // over any explicit datatype, and a language-tagged literal is never
        // composite.
        let datatype_iri = lit.datatype_iri();
        let bound = crate::cdt_blank::bind_cdt_blank_labels_with_memory(
            &lit.lexical_form,
            datatype_iri,
            binding,
            memory,
        )?;
        let id = self.intern_literal_parts_with_memory(
            &bound,
            lit.datatype.as_deref(),
            lit.language.as_deref(),
            lit.direction,
            memory,
        )?;
        if let std::borrow::Cow::Owned(bound) = bound {
            memory.release_string(bound)?;
        }
        Ok(id)
    }

    /// Intern a triple term (RDF 1.2 quoted triple). Identified structurally by the
    /// resolved `(s, p, o)` ids (C0.3); dedup is by that triple. Acyclicity is a
    /// freeze-time concern ([`super::validate`]), not enforced here.
    pub fn intern_triple(
        &mut self,
        s: TermId,
        p: TermId,
        o: TermId,
    ) -> Result<TermId, StorageError> {
        self.interner.intern(TermLookup::Triple { s, p, o })
    }

    /// Intern one owned model term into this builder, recursively for triple terms.
    ///
    /// This is the inverse of [`RdfDataset::to_owned_quad`](super::dataset::RdfDataset::to_owned_quad)
    /// at the owned boundary: tests and adapter edges that already hold
    /// [`RdfTerm`] values can freeze them into the concrete IR without detouring.
    ///
    /// # Blank labels are decoded, so the owned round trip is the identity
    ///
    /// The owned model has ONE string slot for a blank node, so
    /// [`to_owned_quad`](super::dataset::RdfDataset::to_owned_quad) renders the
    /// IR's `(label, scope)` pair into it through
    /// [`BlankScope::qualify_label`]. This function is that rendering's exact
    /// inverse: the label is decoded with
    /// [`BlankScope::unqualify_label`] and interned at the scope it names, so
    /// `dataset → owned → dataset` preserves blank-node IDENTITY rather than
    /// merely isomorphism. Re-materializing a dataset through the owned model —
    /// which the SHACL projection, the SHACL rules fixpoint, and the reasoning
    /// withholding passes all do, repeatedly — therefore leaves its labels
    /// untouched instead of encoding an already-encoded label once per pass.
    ///
    /// A caller minting a blank label of its own passes a label it has NOT
    /// qualified, and the decode leaves any such label alone unless it is spelled
    /// exactly as the encoder's output (see
    /// [`unqualify_label`](BlankScope::unqualify_label) for that grammar).
    ///
    /// For cross-dataset merges use [`push_dataset`](Self::push_dataset), which
    /// assigns a fresh scope per merged dataset (standardize-apart, C0.2).
    pub fn intern_owned_term(&mut self, term: &RdfTerm) -> Result<TermId, StorageError> {
        self.intern_owned_term_scoped(term, BlankScope::DEFAULT)
    }

    /// Like [`intern_owned_term`](Self::intern_owned_term) but overrides the scope
    /// used for every blank node (including blanks nested inside quoted triples).
    ///
    /// This is the scoped standardize-apart primitive (C0.2): callers merging
    /// several native sources into one builder assign each source a FRESH
    /// [`BlankScope`] so same-labelled blanks from different sources never collide
    /// — exactly the discipline [`push_dataset`](Self::push_dataset) applies
    /// internally. Direct `push_owned_*`/`intern_owned_term` pushes use
    /// [`BlankScope::DEFAULT`] (scope 0).
    ///
    /// # Why only the default scope decodes the label
    ///
    /// At [`BlankScope::DEFAULT`] this is the exact inverse of the owned
    /// rendering, so the label is decoded (see
    /// [`intern_owned_term`](Self::intern_owned_term)). An EXPLICIT non-default
    /// scope is a standardize-apart RELABELING, not an inverse: the merged
    /// source's own scopes are being collapsed into one fresh scope, and the
    /// qualified rendering is precisely what keeps two blanks the source held
    /// apart (`b` at its scope 0 and `b` at its scope 1) apart in the target
    /// scope. Decoding there would map both to the label `b` under one scope and
    /// silently conflate two distinct nodes, so the qualified spelling is kept
    /// verbatim as the merged node's raw label — injective, which is the property
    /// standardize-apart actually needs.
    /// # Composite literals bind here too
    ///
    /// A `cdt:List` / `cdt:Map` literal's embedded blank labels are bound by the
    /// SAME rule this function applies to a bare
    /// [`RdfTerm::BlankNode`], because a merge assigns
    /// each source a fresh scope and an embedded label left unbound would name a
    /// node the merged dataset does not have. Binding here is TOTAL — the owned
    /// model is an internal bridge with no document to refuse — so a document
    /// ingress that must reject an ill-formed composite literal uses
    /// [`intern_owned_term_bound`](Self::intern_owned_term_bound) instead.
    ///
    /// A triple term is interned over a work list rather than the call stack: its
    /// subject, then its predicate, then its object, each fully before the next, and
    /// the triple itself once all three have ids — so a term nested to any depth
    /// interns in the order a recursive descent would intern it, on constant stack.
    pub fn intern_owned_term_scoped(
        &mut self,
        term: &RdfTerm,
        scope: BlankScope,
    ) -> Result<TermId, StorageError> {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.interner_buffer_bytes()?);
        self.intern_owned_term_scoped_with_memory(term, scope, &mut memory)
    }

    /// Intern an owned term under the original admission, atomically with all children.
    ///
    /// # Errors
    /// Returns the original logical capacity or physical refusal. A failed walk
    /// preserves every prior identity and removes only this call's new terms.
    pub fn intern_owned_term_scoped_with_memory<S: Admission + ?Sized>(
        &mut self,
        term: &RdfTerm,
        scope: BlankScope,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, StorageError> {
        self.transaction_with_memory(memory, |builder, memory| {
            builder.intern_owned_term_admitted(term, scope, memory)
        })
    }

    fn intern_owned_term_admitted<S: Admission + ?Sized>(
        &mut self,
        term: &RdfTerm,
        scope: BlankScope,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, StorageError> {
        self.intern_owned_term_walk(term, scope, memory, |builder, literal, memory| {
            let lexical = crate::cdt_blank::bind_cdt_blank_labels_unchecked_with_memory(
                &literal.lexical_form,
                literal.datatype_iri(),
                owned_binding(scope),
                memory,
            )?;
            let id = builder.intern_literal_parts_with_memory(
                &lexical,
                literal.datatype.as_deref(),
                literal.language.as_deref(),
                literal.direction,
                memory,
            )?;
            if let std::borrow::Cow::Owned(lexical) = lexical {
                memory.release_string(lexical)?;
            }
            Ok(id)
        })
    }

    fn intern_owned_term_walk<E: From<StorageError>, S: Admission + ?Sized>(
        &mut self,
        term: &RdfTerm,
        scope: BlankScope,
        memory: &mut Memory<'_, S>,
        mut literal: impl FnMut(&mut Self, &RdfLiteral, &mut Memory<'_, S>) -> Result<TermId, E>,
    ) -> Result<TermId, E> {
        use purrdf_lex::walk::WorkList;
        let mut steps: WorkList<OwnedStep<'_>, 16> = WorkList::with(OwnedStep::Term(term));
        let mut ids: WorkList<TermId, 16> = WorkList::new();
        while let Some(step) = steps.pop() {
            let id = match step {
                OwnedStep::Term(RdfTerm::Iri(iri)) => self.intern_iri_with_memory(iri, memory)?,
                OwnedStep::Term(RdfTerm::BlankNode(label)) if scope == BlankScope::DEFAULT => {
                    if let Some(id) = self.interner.lookup_owned_blank(label) {
                        ids.try_push_admitted(id, memory)?;
                        continue;
                    }
                    let (label, scope) = crate::blank_label::decode_blank_label_with_memory(
                        label,
                        LabelAlphabet::Unconstrained,
                        memory,
                    )?;
                    let id = self.intern_blank_with_memory(&label, scope, memory)?;
                    if let std::borrow::Cow::Owned(label) = label {
                        memory.release_string(label)?;
                    }
                    id
                }
                OwnedStep::Term(RdfTerm::BlankNode(label)) => {
                    self.intern_blank_with_memory(label, scope, memory)?
                }
                OwnedStep::Term(RdfTerm::Literal(value)) => literal(self, value, memory)?,
                OwnedStep::Term(RdfTerm::Triple(triple)) => {
                    for child in [
                        OwnedStep::Assemble,
                        OwnedStep::Term(&triple.object),
                        OwnedStep::Predicate(&triple.predicate),
                        OwnedStep::Term(&triple.subject),
                    ] {
                        steps.try_push_admitted(child, memory)?;
                    }
                    continue;
                }
                OwnedStep::Predicate(predicate) => {
                    self.intern_iri_with_memory(predicate, memory)?
                }
                OwnedStep::Assemble => {
                    let o = ids.pop().expect("a triple term's object is interned");
                    let p = ids.pop().expect("a triple term's predicate is interned");
                    let s = ids.pop().expect("a triple term's subject is interned");
                    self.intern_triple_with_memory(s, p, o, memory)?
                }
            };
            ids.try_push_admitted(id, memory)?;
        }
        let result = ids
            .pop()
            .expect("the term's own id is the last one interned");
        steps.release_admitted(memory)?;
        ids.release_admitted(memory)?;
        Ok(result)
    }

    /// [`intern_owned_term_scoped`](Self::intern_owned_term_scoped) for a term
    /// read from a DOCUMENT: identical binding, but an ill-formed composite
    /// literal is refused rather than taken as given.
    ///
    /// The owned-model codecs (JSON-LD / YAML-LD) and the multi-source loader
    /// use this, so a document reaches the store through exactly the same
    /// validation every text codec applies.
    ///
    /// # Errors
    /// [`CdtBlankError`](crate::cdt_blank::CdtBlankError) when a composite
    /// literal anywhere in `term` — including inside a quoted triple — does not
    /// parse. The caller must refuse the whole document.
    ///
    /// A triple term is interned over a work list in the same order as
    /// [`intern_owned_term_scoped`](Self::intern_owned_term_scoped) — subject,
    /// predicate, object, then the triple — and the first refused composite literal
    /// restores the builder's prior logical state without publishing a prefix.
    pub fn intern_owned_term_bound(
        &mut self,
        term: &RdfTerm,
        scope: BlankScope,
    ) -> Result<TermId, crate::cdt_blank::CdtBlankError> {
        let mut resident = Resident;
        let bytes = self.native_buffer_bytes()?;
        self.intern_owned_term_bound_with_memory(
            term,
            scope,
            &mut Memory::resume(&mut resident, bytes),
        )
    }

    /// Validate an owned document term and every nested literal under one admission.
    /// Diagnostic payload remains owned by that account until the caller drops it.
    /// # Errors
    /// Returns the first malformed CDT or storage cause with prior IDs unchanged.
    pub fn intern_owned_term_bound_with_memory<S: Admission + ?Sized>(
        &mut self,
        term: &RdfTerm,
        scope: BlankScope,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, crate::cdt_blank::CdtBlankError> {
        self.transaction_try_with_memory(
            memory,
            |builder, memory| {
                builder.intern_owned_term_walk(term, scope, memory, |builder, literal, memory| {
                    builder.intern_literal_bound_admitted(literal, owned_binding(scope), memory)
                })
            },
            crate::cdt_blank::CdtBlankError::owned_bytes,
        )
    }

    /// Push one owned model quad into this builder.
    ///
    /// The predicate is interned as an IRI term, and any source location on the
    /// owned quad is preserved on the corresponding pushed quad handle. Structural
    /// validity remains the normal [`freeze`](Self::freeze) contract.
    /// # Errors
    /// Returns the original logical capacity or physical storage refusal.
    pub fn push_owned_quad(&mut self, quad: &RdfQuad) -> Result<(), StorageError> {
        self.push_owned_quad_scoped(quad, BlankScope::DEFAULT)?;
        Ok(())
    }

    /// Like [`push_owned_quad`](Self::push_owned_quad) but routes blank interning
    /// through `scope`.
    ///
    /// The scoped standardize-apart primitive (C0.2) for the quad path: a caller
    /// merging several native sources assigns each source a fresh [`BlankScope`] so
    /// same-labelled blanks from different sources never collide, the same
    /// discipline [`push_dataset`](Self::push_dataset) applies internally. Direct
    /// [`push_owned_quad`](Self::push_owned_quad) uses [`BlankScope::DEFAULT`].
    /// # Errors
    /// Returns the original logical capacity or physical storage refusal.
    pub fn push_owned_quad_scoped(
        &mut self,
        quad: &RdfQuad,
        scope: BlankScope,
    ) -> Result<(), StorageError> {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.native_buffer_bytes()?);
        self.push_owned_quad_scoped_with_memory(quad, scope, &mut memory)
    }

    /// Import a quad and its children as one originally admitted operation.
    ///
    /// # Errors
    /// Returns the first capacity or physical refusal without publishing a prefix.
    pub fn push_owned_quad_scoped_with_memory<S: Admission + ?Sized>(
        &mut self,
        quad: &RdfQuad,
        scope: BlankScope,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.transaction_with_memory(memory, |builder, memory| {
            builder.push_owned_quad_admitted(quad, scope, memory)
        })
    }

    fn push_owned_quad_admitted<S: Admission + ?Sized>(
        &mut self,
        quad: &RdfQuad,
        scope: BlankScope,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let s = self.intern_owned_term_admitted(&quad.subject, scope, memory)?;
        let p = self.intern_iri_with_memory(&quad.predicate, memory)?;
        let o = self.intern_owned_term_admitted(&quad.object, scope, memory)?;
        let g = quad
            .graph_name
            .as_ref()
            .map(|graph_name| self.intern_owned_term_admitted(graph_name, scope, memory))
            .transpose()?;
        let handle = store_once_with_memory(
            &mut self.quads,
            &mut self.quad_index,
            QuadRow { s, p, o, g },
            self.interner.capacity,
            CapacitySpace::Quads,
            memory,
        )
        .map(QuadHandle::from_index)?;
        if let Some(location) = quad
            .location
            .as_ref()
            .filter(|location| !location.is_empty())
        {
            let location = location.clone_with_memory(memory)?;
            memory.push(&mut self.locations, (handle, location))?;
        }
        Ok(())
    }

    /// Push one owned RDF 1.2 reifier binding into this builder, re-interning the
    /// reifier resource and the bound triple's `(s, p, o)` terms. The companion of
    /// [`push_owned_quad`](Self::push_owned_quad) for the reifier side-table.
    /// # Errors
    /// Returns the original logical capacity or physical storage refusal.
    pub fn push_owned_reifier(&mut self, reifier: &RdfReifier) -> Result<(), StorageError> {
        self.push_owned_reifier_scoped(reifier, BlankScope::DEFAULT)?;
        Ok(())
    }

    /// Like [`push_owned_reifier`](Self::push_owned_reifier) but routes blank
    /// interning through `scope`.
    ///
    /// The scoped standardize-apart primitive (C0.2) for the reifier side-table: a
    /// caller merging several native sources assigns each source a fresh
    /// [`BlankScope`] so same-labelled blanks from different sources never collide,
    /// as [`push_dataset`](Self::push_dataset) does internally. Direct
    /// [`push_owned_reifier`](Self::push_owned_reifier) uses [`BlankScope::DEFAULT`].
    /// # Errors
    /// Returns the original logical capacity or physical storage refusal.
    pub fn push_owned_reifier_scoped(
        &mut self,
        reifier: &RdfReifier,
        scope: BlankScope,
    ) -> Result<(), StorageError> {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.native_buffer_bytes()?);
        self.push_owned_reifier_scoped_with_memory(reifier, scope, &mut memory)
    }

    /// Import a reifier and its complete triple under the original admission.
    ///
    /// # Errors
    /// Returns the first storage refusal without retaining newly imported children.
    pub fn push_owned_reifier_scoped_with_memory<S: Admission + ?Sized>(
        &mut self,
        reifier: &RdfReifier,
        scope: BlankScope,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.transaction_with_memory(memory, |builder, memory| {
            let s =
                builder.intern_owned_term_admitted(&reifier.statement.subject, scope, memory)?;
            let p = builder.intern_iri_with_memory(&reifier.statement.predicate, memory)?;
            let o = builder.intern_owned_term_admitted(&reifier.statement.object, scope, memory)?;
            let triple = builder.intern_triple_with_memory(s, p, o, memory)?;
            let reifier_id = builder.intern_owned_term_admitted(&reifier.reifier, scope, memory)?;
            let g = reifier
                .graph
                .as_ref()
                .map(|graph| builder.intern_owned_term_admitted(graph, scope, memory))
                .transpose()?;
            builder.push_reifier_in_graph_with_memory(reifier_id, triple, g, memory)
        })
    }

    /// Push one owned RDF 1.2 statement annotation into this builder, re-interning
    /// the reifier resource and the `(predicate, object)` terms.
    /// # Errors
    /// Returns the original logical capacity or physical storage refusal.
    pub fn push_owned_annotation(
        &mut self,
        annotation: &RdfAnnotation,
    ) -> Result<(), StorageError> {
        self.push_owned_annotation_scoped(annotation, BlankScope::DEFAULT)?;
        Ok(())
    }

    /// Like [`push_owned_annotation`](Self::push_owned_annotation) but routes blank
    /// interning through `scope`.
    ///
    /// The scoped standardize-apart primitive (C0.2) for the annotation
    /// side-table: a caller merging several native sources assigns each source a
    /// fresh [`BlankScope`] so same-labelled blanks from different sources never
    /// collide, as [`push_dataset`](Self::push_dataset) does internally. Direct
    /// [`push_owned_annotation`](Self::push_owned_annotation) uses
    /// [`BlankScope::DEFAULT`].
    /// # Errors
    /// Returns the original logical capacity or physical storage refusal.
    pub fn push_owned_annotation_scoped(
        &mut self,
        annotation: &RdfAnnotation,
        scope: BlankScope,
    ) -> Result<(), StorageError> {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.native_buffer_bytes()?);
        self.push_owned_annotation_scoped_with_memory(annotation, scope, &mut memory)
    }

    /// Import an annotation and all its terms under the original admission.
    ///
    /// # Errors
    /// Returns the first storage refusal without retaining newly imported children.
    pub fn push_owned_annotation_scoped_with_memory<S: Admission + ?Sized>(
        &mut self,
        annotation: &RdfAnnotation,
        scope: BlankScope,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.transaction_with_memory(memory, |builder, memory| {
            let reifier_id =
                builder.intern_owned_term_admitted(&annotation.reifier, scope, memory)?;
            let p = builder.intern_iri_with_memory(&annotation.predicate, memory)?;
            let o = builder.intern_owned_term_admitted(&annotation.object, scope, memory)?;
            let g = annotation
                .graph
                .as_ref()
                .map(|graph| builder.intern_owned_term_admitted(graph, scope, memory))
                .transpose()?;
            builder.push_annotation_in_graph_with_memory(reifier_id, p, o, g, memory)
        })
    }

    /// Merge every quad, reifier, and annotation of `other` into this builder,
    /// re-interning each borrowed term into THIS builder's interner (`TermId`s are
    /// dataset-local, C0.8). Unlike [`extend_quads`](Self::extend_quads),
    /// it carries the FULL RDF 1.2 statement layer — reifier bindings and
    /// annotations, not just base quads — so merging a graph that carries `<<>>` /
    /// `rdf:reifies` does not silently drop its side-tables. Duplicates collapse on
    /// freeze per the normal C0.4/C0.5 contract.
    ///
    /// # Standardize-apart (blank-node safety)
    ///
    /// Each call importing a blank allocates a FRESH [`BlankScope`] for `other`
    /// (scopes 1, 2, 3, … on successive blank-bearing calls; scope 0 is [`BlankScope::DEFAULT`]
    /// reserved for direct `push_owned_*` pushes). Blank nodes from different source
    /// datasets can therefore never collide even when they share the same label
    /// (e.g. both datasets have `_:b0`). The qualified label rendered for legacy
    /// consumers is the scope envelope `"purrdfesc{n}_{body}"` per
    /// [`BlankScope::qualify_label`] (C0.2).
    /// # Errors
    /// Refuses exhausted merge scopes or resident tables with their exact cause.
    pub fn push_dataset(&mut self, other: &RdfDataset) -> Result<(), StorageError> {
        let mut resident = Resident;
        let bytes = self.native_buffer_bytes()?;
        self.push_dataset_with_memory(other, &mut Memory::resume(&mut resident, bytes))
    }

    /// Merge an immutable source under the destination's original admission.
    /// Existing destination buffers must already be charged to `memory`.
    /// Terms are imported by borrowed value, once per referenced source identity.
    ///
    /// # Errors
    /// Returns the original capacity or physical refusal. A failed merge restores
    /// prior identities, rows, locations and the next blank scope; retained grown
    /// capacity remains charged to its original owner.
    pub fn push_dataset_with_memory<S: Admission + ?Sized>(
        &mut self,
        other: &RdfDataset,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.transaction_with_memory(memory, |builder, memory| {
            builder.merge_dataset_with_memory(other, memory)
        })
    }

    fn import_binding_scope<S: Admission + ?Sized>(
        &mut self,
        binding: &mut ImportBinding,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<BlankScope>, StorageError> {
        let ImportBinding::Fresh(cached) = binding else {
            return Ok(None);
        };
        if let Some(scope) = cached {
            return Ok(Some(*scope));
        }
        use purrdf_lex::allocation::{CapacityError, CapacitySpace};
        let mut next = self.next_merge_scope;
        loop {
            CapacityError::check(
                CapacitySpace::BlankScopes,
                u128::from(next),
                u128::from(u32::MAX),
            )?;
            let scope = BlankScope(u32::try_from(next).map_err(|_| StorageError::SizeOverflow)?);
            if !self.used_scopes.contains(&scope) {
                break;
            }
            next += 1;
        }
        let scope = BlankScope(u32::try_from(next).map_err(|_| StorageError::SizeOverflow)?);
        if let Some(position) = self.reserve_scope_with_memory(scope, memory)? {
            self.publish_scope(position, scope);
        }
        self.next_merge_scope = next + 1;
        *cached = Some(scope);
        Ok(Some(scope))
    }

    fn merge_dataset_with_memory<S: Admission + ?Sized>(
        &mut self,
        other: &RdfDataset,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let remap = self.merge_dataset_mapping_with_memory(other, memory)?;
        memory.release_vec(remap)
    }

    /// Import a complete source graph and return its source-local term remap.
    /// The mapping shares the exact import's blank-scope translation, including
    /// quoted terms. Its capacity remains charged until the caller releases it.
    /// # Errors
    /// Preserves native storage refusal and rolls back logical publication.
    pub fn push_dataset_with_mapping_with_memory<S: Admission + ?Sized>(
        &mut self,
        other: &RdfDataset,
        memory: &mut Memory<'_, S>,
    ) -> Result<Vec<Option<TermId>>, StorageError> {
        self.transaction_with_memory(memory, |builder, memory| {
            builder.merge_dataset_mapping_with_memory(other, memory)
        })
    }

    /// Import a native term through the existing iterative term-transfer body.
    /// Source blank scopes are preserved; the term ID must be valid in source.
    /// This transfers a value, without merging documents or building owned terms.
    /// # Errors
    /// Returns original admission, layout or allocator refusal.
    pub fn import_native_term_with_memory<S: Admission + ?Sized>(
        &mut self,
        source: &RdfDataset,
        term: TermId,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, StorageError> {
        self.transaction_with_memory(memory, |builder, memory| {
            let mut remap = Vec::new();
            memory.reserve(&mut remap, source.term_count())?;
            remap.resize(source.term_count(), None);
            let result = builder.import_term_with_memory(
                &|id| source.resolve(id),
                term,
                &mut remap,
                &mut ImportBinding::Preserve,
                memory,
            );
            memory.release_vec_after(remap, result)
        })
    }

    fn merge_dataset_mapping_with_memory<S: Admission + ?Sized>(
        &mut self,
        other: &RdfDataset,
        memory: &mut Memory<'_, S>,
    ) -> Result<Vec<Option<TermId>>, StorageError> {
        let mut binding = ImportBinding::Fresh(None);
        let mut remap = Vec::new();
        for _ in 0..other.term_count() {
            memory.push(&mut remap, None)?;
        }
        let resolve = |id| other.resolve(id);
        for (ordinal, quad) in other.quads().enumerate() {
            let mut import =
                |id| self.import_term_with_memory(&resolve, id, &mut remap, &mut binding, memory);
            let s = import(quad.s)?;
            let p = import(quad.p)?;
            let o = import(quad.o)?;
            let g = quad.g.map(&mut import).transpose()?;
            let handle = QuadHandle::from_index(store_once_with_memory(
                &mut self.quads,
                &mut self.quad_index,
                QuadRow { s, p, o, g },
                self.interner.capacity,
                CapacitySpace::Quads,
                memory,
            )?);
            let old = QuadHandle::from_index(
                u32::try_from(ordinal).map_err(|_| StorageError::SizeOverflow)?,
            );
            if let Some(location) = other.location_of(old) {
                let location = location.clone_with_memory(memory)?;
                memory.push(&mut self.locations, (handle, location))?;
            }
        }
        for (r, t, g) in other.reifiers_with_graph() {
            let mut import =
                |id| self.import_term_with_memory(&resolve, id, &mut remap, &mut binding, memory);
            let r = import(r)?;
            let t = import(t)?;
            let g = g.map(&mut import).transpose()?;
            self.push_reifier_in_graph_with_memory(r, t, g, memory)?;
        }
        for (r, p, o, g) in other.annotations_with_graph() {
            let mut import =
                |id| self.import_term_with_memory(&resolve, id, &mut remap, &mut binding, memory);
            let r = import(r)?;
            let p = import(p)?;
            let o = import(o)?;
            let g = g.map(&mut import).transpose()?;
            self.push_annotation_in_graph_with_memory(r, p, o, g, memory)?;
        }
        for graph in other.named_graphs() {
            let graph =
                self.import_term_with_memory(&resolve, graph, &mut remap, &mut binding, memory)?;
            self.declare_named_graph_with_memory(graph, memory)?;
        }
        Ok(remap)
    }

    /// Append a validated graph directly, without an intermediate frozen dataset.
    ///
    /// Terms are imported once by borrowed value, with a compact ID remap. Blank
    /// scopes are preserved: this is composition within a shared identity space,
    /// rather than an independent-document merge. Quads, graph-qualified overlays,
    /// declaration-only graphs and source locations all retain their identity.
    /// Validation happens before this method receives its input, so no fallible
    /// operation can publish a prefix of the staged graph.
    ///
    /// The destination's content-address recognition and derivation configuration
    /// remains authoritative. RDF terms and derivation triples are preserved;
    /// source-side recognition caches are recomputed using the destination's
    /// configured scheme rather than silently changing its policy.
    ///
    /// Returns the number of newly copied term-payload bytes. This operational
    /// counter is independent of deterministic dataset content and ordering.
    /// # Errors
    /// Refuses the original destination capacity with its previous state intact.
    pub fn append_validated(
        &mut self,
        source: ValidatedRdfDatasetBuilder,
    ) -> Result<usize, StorageError> {
        let mut resident = Resident;
        let bytes = self.native_buffer_bytes()?;
        let result =
            self.append_validated_with_memory(&source, &mut Memory::resume(&mut resident, bytes));
        drop(source);
        result
    }

    /// Append validated statements transactionally under physical admission.
    /// Only new source terms and rows are staged; failures remove their suffix.
    /// Existing buffers, term IDs and configuration retain their original owners.
    /// Existing destination buffers must already be charged to `memory`. A
    /// caller adopting a resident builder first admits [`Self::native_buffer_bytes`]
    /// into its new account; an originally admitted builder keeps its old charge.
    ///
    /// # Errors
    /// Returns storage refusal with prior logical state and IDs unchanged. Grown
    /// capacity can remain allocated and admitted after a late refusal.
    pub fn append_validated_with_memory<S: Admission + ?Sized>(
        &mut self,
        source: &ValidatedRdfDatasetBuilder,
        memory: &mut Memory<'_, S>,
    ) -> Result<usize, StorageError> {
        self.transaction_with_memory(memory, |builder, memory| {
            let before = builder.interner.arena.len();
            builder.append_builder_with_memory(&source.inner, memory)?;
            builder
                .interner
                .arena
                .len()
                .checked_sub(before)
                .ok_or(StorageError::SizeOverflow)
        })
    }

    fn append_builder_with_memory<S: Admission + ?Sized>(
        &mut self,
        source: &Self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let mut remap = Vec::new();
        for _ in 0..source.term_count() {
            memory.push(&mut remap, None)?;
        }
        for index in 0..source.term_count() {
            let id =
                TermId::from_index(u32::try_from(index).map_err(|_| StorageError::SizeOverflow)?)
                    .expect("stored term index is representable");
            self.import_term_with_memory(
                &|id| source.resolve(id),
                id,
                &mut remap,
                &mut ImportBinding::Preserve,
                memory,
            )?;
        }
        let mapped = |id: TermId| remap[id.index()].expect("validated term import completed");
        let mut quad_map = Vec::new();
        for quad in &source.quads {
            let row = QuadRow {
                s: mapped(quad.s),
                p: mapped(quad.p),
                o: mapped(quad.o),
                g: quad.g.map(mapped),
            };
            let index = store_once_with_memory(
                &mut self.quads,
                &mut self.quad_index,
                row,
                self.interner.capacity,
                CapacitySpace::Quads,
                memory,
            )?;
            memory.push(&mut quad_map, QuadHandle::from_index(index))?;
        }
        for &(r, t, g) in &source.reifiers {
            self.push_reifier_in_graph_with_memory(mapped(r), mapped(t), g.map(mapped), memory)?;
        }
        for &(r, p, o, g) in &source.annotations {
            self.push_annotation_in_graph_with_memory(
                mapped(r),
                mapped(p),
                mapped(o),
                g.map(mapped),
                memory,
            )?;
        }
        for &graph in &source.declared_graphs {
            self.declare_named_graph_with_memory(mapped(graph), memory)?;
        }
        for (handle, location) in &source.locations {
            if let Some(&mapped) = quad_map.get(handle.index()) {
                let location = location.clone_with_memory(memory)?;
                memory.push(&mut self.locations, (mapped, location))?;
            }
        }
        memory.release_vec(quad_map)?;
        memory.release_vec(remap)
    }

    /// Import one resolved source term, memoised in `remap` by the source id.
    /// `binding` standardizes an independent document apart on its first blank;
    /// `Preserve` retains
    /// original blank identities for validated graph publication.
    ///
    /// The walk runs over a work list rather than the call stack. A term is entered
    /// by first consulting `remap`; a literal's datatype is imported before the
    /// literal itself, and a triple term's subject, predicate and object are each
    /// imported fully, in that order, before the triple. Every term is recorded in
    /// `remap` as soon as its own id exists, so a component shared by several terms
    /// is imported once and found in the memo on every later visit.
    fn import_term_with_memory<'s, S: Admission + ?Sized>(
        &mut self,
        resolve: &impl Fn(TermId) -> TermRef<'s>,
        id: TermId,
        remap: &mut [Option<TermId>],
        binding: &mut ImportBinding,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, StorageError> {
        enum Step<'s> {
            /// Import this source term, unless `remap` already holds it.
            Enter(TermId),
            /// Intern this source literal from its imported datatype.
            Literal {
                id: TermId,
                lexical: &'s str,
                language: Option<&'s str>,
                direction: Option<RdfTextDirection>,
            },
            /// Intern this source triple term from its three imported components.
            Triple(TermId),
        }
        let mut steps: Vec<Step<'_>> = Vec::new();
        memory.push(&mut steps, Step::Enter(id))?;
        let mut imported: Vec<TermId> = Vec::new();
        while let Some(step) = steps.pop() {
            let (id, mapped) = match step {
                Step::Enter(id) => {
                    if let Some(mapped) = remap[id.index()] {
                        memory.push(&mut imported, mapped)?;
                        continue;
                    }
                    match resolve(id) {
                        TermRef::Iri(iri) => (id, self.intern_iri_with_memory(iri, memory)?),
                        TermRef::Blank { label, scope } => {
                            let mapped = if let Some(destination) =
                                self.import_binding_scope(binding, memory)?
                            {
                                let label = crate::blank_label::encode_blank_label_with_memory(
                                    label,
                                    scope,
                                    LabelAlphabet::Unconstrained,
                                    memory,
                                )?;
                                let mapped =
                                    self.intern_blank_with_memory(&label, destination, memory)?;
                                if let std::borrow::Cow::Owned(label) = label {
                                    memory.release_string(label)?;
                                }
                                mapped
                            } else {
                                self.intern_blank_with_memory(label, scope, memory)?
                            };
                            (id, mapped)
                        }
                        TermRef::Literal {
                            lexical,
                            datatype,
                            language,
                            direction,
                        } => {
                            memory.extend(
                                &mut steps,
                                [
                                    Step::Literal {
                                        id,
                                        lexical,
                                        language,
                                        direction,
                                    },
                                    Step::Enter(datatype),
                                ],
                            )?;
                            continue;
                        }
                        TermRef::Triple { s, p, o } => {
                            memory.extend(
                                &mut steps,
                                [
                                    Step::Triple(id),
                                    Step::Enter(o),
                                    Step::Enter(p),
                                    Step::Enter(s),
                                ],
                            )?;
                            continue;
                        }
                    }
                }
                Step::Literal {
                    id,
                    lexical,
                    language,
                    direction,
                } => {
                    let datatype = imported.pop().expect("a literal's datatype is imported");
                    let mapped = if matches!(binding, ImportBinding::Fresh(_)) {
                        let TermRef::Literal {
                            datatype: source_datatype,
                            ..
                        } = resolve(id)
                        else {
                            unreachable!("literal import step retains its source literal")
                        };
                        let TermRef::Iri(datatype_iri) = resolve(source_datatype) else {
                            unreachable!("validated literal datatype is an IRI")
                        };
                        let lexical = crate::cdt_blank::bind_cdt_blank_labels_lazy_with_memory(
                            lexical,
                            datatype_iri,
                            &mut |memory| {
                                self.import_binding_scope(binding, memory).map(|scope| {
                                    owned_binding(
                                        scope.expect("independent source retains fresh binding"),
                                    )
                                })
                            },
                            memory,
                        )?;
                        let mapped = self.intern_literal_parts_with_memory(
                            &lexical,
                            Some(datatype_iri),
                            language,
                            direction,
                            memory,
                        )?;
                        if let std::borrow::Cow::Owned(lexical) = lexical {
                            memory.release_string(lexical)?;
                        }
                        mapped
                    } else {
                        self.interner.intern_with_memory(
                            TermLookup::Literal {
                                lexical,
                                datatype,
                                language,
                                direction,
                            },
                            memory,
                        )?
                    };
                    (id, mapped)
                }
                Step::Triple(id) => {
                    let o = imported.pop().expect("a triple term's object is imported");
                    let p = imported
                        .pop()
                        .expect("a triple term's predicate is imported");
                    let s = imported.pop().expect("a triple term's subject is imported");
                    (id, self.intern_triple_with_memory(s, p, o, memory)?)
                }
            };
            remap[id.index()] = Some(mapped);
            memory.push(&mut imported, mapped)?;
        }
        let result = imported
            .pop()
            .expect("the term's own id is the last one imported");
        memory.release_vec(imported)?;
        memory.release_vec(steps)?;
        Ok(result)
    }

    /// Crate-internal read access to an interned term. [`freeze`](Self::freeze) and
    /// [`super::validate`] consume this to materialize and check the dataset.
    pub(crate) fn term(&self, id: TermId) -> &InternedTerm {
        self.interner.term(id)
    }

    /// Resolve a term's [`StrRange`] to a `&str` borrowed from the builder's arena
    /// (for build-time readers that have an interned term's range).
    pub(crate) fn interned_str(&self, range: StrRange) -> &str {
        arena_str(self.interner.arena(), range)
    }

    /// Resolve a builder-local term id to a borrowed term view.
    ///
    /// This mirrors [`RdfDataset::resolve`] for adapters that need to validate a
    /// just-interned term before freezing the dataset.
    pub fn resolve(&self, id: TermId) -> TermRef<'_> {
        match self.term(id) {
            InternedTerm::Iri(iri) => TermRef::Iri(self.interned_str(*iri)),
            InternedTerm::Blank { label, scope } => TermRef::Blank {
                label: self.interned_str(*label),
                scope: *scope,
            },
            InternedTerm::Literal(literal) => TermRef::Literal {
                lexical: self.interned_str(literal.lexical_form),
                datatype: literal.datatype,
                language: literal.language.map(|lang| self.interned_str(lang)),
                direction: literal.direction,
            },
            InternedTerm::Triple { s, p, o } => TermRef::Triple {
                s: *s,
                p: *p,
                o: *o,
            },
        }
    }

    /// The number of distinct interned terms. Used by validation (the ID-reference
    /// bound) and as the frozen dataset's term count.
    pub(crate) fn term_count(&self) -> usize {
        self.interner.term_count()
    }

    /// Exact distinct rows retained by the three store-once statement tables.
    pub(crate) fn statement_count(&self) -> usize {
        super::statement_row_count!(self)
    }

    /// The terms, RDF rows and payload bytes freezing this builder now would
    /// retain, given the number of distinct named graphs it names: exactly what the
    /// frozen dataset's [`term_count`](RdfDataset::term_count),
    /// [`rdf_row_count`](RdfDataset::rdf_row_count) and
    /// [`rdf_payload_bytes`](RdfDataset::rdf_payload_bytes) will report. Every
    /// table already holds one entry per distinct term or row, and freezing only
    /// sorts and boxes them; the named-graph table is the one that freezing
    /// deduplicates, so its distinct count is the caller's. Each quantity only
    /// grows as interning proceeds.
    pub(crate) fn pending_extent(&self, named_graphs: usize) -> (usize, usize, usize) {
        let rows = self.statement_count();
        let payload = self.interner.arena.len()
            + size_of_val(self.interner.terms.as_slice())
            + size_of_val(self.quads.as_slice())
            + size_of_val(self.reifiers.as_slice())
            + size_of_val(self.annotations.as_slice())
            + named_graphs * size_of::<TermId>();
        (self.interner.term_count(), rows, payload)
    }

    /// The first IRI interned into this builder that violated the IR-boundary
    /// absoluteness invariant, with the typed reason.
    ///
    /// Read by [`super::validate`], which converts it into the hard `Err` that
    /// [`freeze`](Self::freeze) returns, so no `RdfDataset` can be materialized
    /// carrying a relative IRI term.
    pub(crate) fn relative_iri(&self) -> Option<(&str, &IriError)> {
        self.interner
            .relative_iri
            .as_ref()
            .map(|(iri, err)| (iri.as_str(), err))
    }

    /// Read the first refused literal recorded by the infallible interner, as
    /// `(diagnostic code, message)`.
    pub(crate) fn invalid_literal(&self) -> Option<(&'static str, &'static str)> {
        self.interner.invalid_literal
    }

    /// Push a quad. Duplicate quads collapse to a single row (C0.5); `g == None`
    /// names the default graph. Use [`push_quad_with_handle`](Self::push_quad_with_handle)
    /// when attaching a source location: it identifies the actual row even when
    /// the quad was already present.
    /// # Errors
    /// Returns the original logical capacity or physical storage refusal.
    pub fn push_quad(
        &mut self,
        s: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
    ) -> Result<(), StorageError> {
        let _ = self.push_quad_with_handle(s, p, o, g)?;
        Ok(())
    }

    /// Append an exact physical record after validating its complete RDF structure.
    ///
    /// # Errors
    /// Refuses malformed terms, positions or a noncanonical reifier record before
    /// adding any record to this builder.
    pub fn push_record(
        &mut self,
        record: &super::mutable::RecordValues,
    ) -> Result<(), NativeBuildError> {
        super::validate::validate_record(record)?;
        self.push_record_unchecked(record)?;
        Ok(())
    }

    pub(crate) fn push_record_unchecked(
        &mut self,
        record: &super::mutable::RecordValues,
    ) -> Result<(), StorageError> {
        use super::mutable::RecordKind;
        use crate::backend::TermFactory as _;
        let q = &record.quad;
        let s = self.intern_value(&q.s)?;
        let p = self.intern_value(&q.p)?;
        let o = self.intern_value(&q.o)?;
        let g = q.g.as_ref().map(|g| self.intern_value(g)).transpose()?;
        match record.kind {
            RecordKind::Ordinary => self.push_quad(s, p, o, g),
            RecordKind::Reifier => self.push_reifier_in_graph(s, o, g),
            RecordKind::Annotation => self.push_annotation_in_graph(s, p, o, g),
        }
    }

    /// Push a quad and return its actual deduplicated-row handle.
    ///
    /// A duplicate returns the existing row's handle; a new quad returns its
    /// newly assigned handle. The handle is immediately valid for
    /// [`attach_location`](Self::attach_location), and remains in builder push
    /// order until freezing remaps source locations to the frozen row order.
    /// `g == None` names the default graph, and graph identity participates in
    /// deduplication.
    /// # Errors
    /// Refuses a full resident row table before publishing the new row.
    pub fn push_quad_with_handle(
        &mut self,
        s: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
    ) -> Result<QuadHandle, StorageError> {
        let row = QuadRow { s, p, o, g };
        store_once(
            &mut self.quads,
            &mut self.quad_index,
            row,
            self.interner.capacity,
            CapacitySpace::Quads,
        )
        .map(QuadHandle::from_index)
    }

    /// Bind a reifier resource to a triple term (C0.4). Several reifiers MAY bind
    /// one triple term; an identical `(reifier, triple)` binding pushed twice
    /// collapses to one.
    ///
    /// Interns `rdf:reifies` into the term table so the side-table is always
    /// projectable as virtual `reifier rdf:reifies <<…>>` quads
    /// ([`RdfDataset::reifier_quads`]) — without it, a dataset rebuilt purely from
    /// `push_owned_reifier` (e.g. a named-graph projection) would carry reifiers the
    /// renderer / pattern-matcher cannot see, because the predicate id is absent.
    /// # Errors
    /// Returns the original logical capacity or physical storage refusal.
    pub fn push_reifier(&mut self, reifier: TermId, triple: TermId) -> Result<(), StorageError> {
        self.push_reifier_in_graph(reifier, triple, None)?;
        Ok(())
    }

    /// Like [`push_reifier`](Self::push_reifier) but records the named graph the
    /// reifier declaration was asserted in (`None` = default graph). A `(reifier,
    /// triple, graph)` triplet pushed twice collapses to one; the SAME `(reifier,
    /// triple)` in two distinct graphs is two bindings.
    /// # Errors
    /// Returns the original logical capacity or physical storage refusal.
    pub fn push_reifier_in_graph(
        &mut self,
        reifier: TermId,
        triple: TermId,
        g: Option<TermId>,
    ) -> Result<(), StorageError> {
        let mut resident = Resident;
        let bytes = self.native_buffer_bytes()?;
        self.push_reifier_in_graph_with_memory(
            reifier,
            triple,
            g,
            &mut Memory::resume(&mut resident, bytes),
        )
    }

    /// Reserve room for `rows` more reifier bindings, table and dedup index together.
    ///
    /// The RDF 1.2 twin of [`reserve_for_replay`](Self::reserve_for_replay), split out
    /// because a source states its overlay size separately from its base-quad count
    /// (a graph with no reifiers at all must reserve nothing for them). A hint, never
    /// a contract. See that method for why this is not public.
    pub(crate) fn reserve_reifiers(&mut self, rows: usize) -> Result<(), StorageError> {
        reserve_store_once(
            &mut self.reifiers,
            &mut self.reifier_index,
            rows,
            self.interner.capacity,
            CapacitySpace::Reifiers,
        )
    }

    /// Reserve room for `rows` more statement annotations, table and dedup index
    /// together. The annotation twin of
    /// [`reserve_reifiers`](Self::reserve_reifiers); the same hint discipline applies.
    pub(crate) fn reserve_annotations(&mut self, rows: usize) -> Result<(), StorageError> {
        reserve_store_once(
            &mut self.annotations,
            &mut self.annotation_index,
            rows,
            self.interner.capacity,
            CapacitySpace::Annotations,
        )
    }

    /// The predicate implicitly interned by reifier insertion, if any. Native
    /// rewrite observers use this actual ID without changing allocation order.
    pub(crate) const fn reifies_predicate(&self) -> Option<TermId> {
        self.reifies_predicate
    }

    /// Push a statement annotation `(reifier, predicate, object)`. Duplicate
    /// annotations collapse to one (C0.5).
    /// # Errors
    /// Returns the original logical capacity or physical storage refusal.
    pub fn push_annotation(
        &mut self,
        reifier: TermId,
        p: TermId,
        o: TermId,
    ) -> Result<(), StorageError> {
        self.push_annotation_in_graph(reifier, p, o, None)?;
        Ok(())
    }

    /// Like [`push_annotation`](Self::push_annotation) but records the named graph the
    /// annotation was asserted in (`None` = default graph); see
    /// [`push_reifier_in_graph`](Self::push_reifier_in_graph).
    /// # Errors
    /// Returns the original logical capacity or physical storage refusal.
    pub fn push_annotation_in_graph(
        &mut self,
        reifier: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
    ) -> Result<(), StorageError> {
        let annotation = (reifier, p, o, g);
        store_once(
            &mut self.annotations,
            &mut self.annotation_index,
            annotation,
            self.interner.capacity,
            CapacitySpace::Annotations,
        )?;
        Ok(())
    }

    /// Attach a source location to a previously pushed quad, identified by its
    /// [`QuadHandle`] (the dense ordinal of the deduplicated quad). Sparse: only
    /// quads with a recorded location are stored. An empty location is ignored.
    /// A handle that names no current row is also ignored; it cannot attach a
    /// location to a different row inserted later.
    pub fn attach_location(&mut self, handle: QuadHandle, loc: RdfLocation) {
        if handle.index() < self.quads.len() && !loc.is_empty() {
            self.locations.push((handle, loc));
        }
    }

    /// The [`QuadHandle`] that the next [`push_quad`](Self::push_quad) call will
    /// assign to a *newly seen* quad — i.e. the current deduplicated-quad count.
    /// A duplicate push does not realize this proposed handle. Use
    /// [`push_quad_with_handle`](Self::push_quad_with_handle) when the next quad
    /// might already be present and its source location needs attaching.
    /// # Errors
    /// Refuses the next handle when the local row count is already at capacity.
    pub fn next_quad_handle(&self) -> Result<QuadHandle, StorageError> {
        use purrdf_lex::allocation::{CapacityError, CapacitySpace};
        CapacityError::check(
            CapacitySpace::Quads,
            self.quads.len() as u128 + 1,
            u128::from(u32::MAX),
        )?;
        Ok(QuadHandle::from_index(
            u32::try_from(self.quads.len()).map_err(|_| StorageError::SizeOverflow)?,
        ))
    }

    /// Validate and freeze originally admitted native graph buffers by value.
    /// Shared publication happens later at the caller's fallible shared home.
    ///
    /// # Errors
    /// Returns physical refusal or the same RDF diagnostic as resident freeze.
    pub fn freeze_with_memory<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<RdfDataset, NativeBuildError> {
        let buffers = self.native_buffer_bytes()?;
        if let Err(error) = super::validate::validate_with_memory(&self, memory) {
            drop(self);
            memory.release_bytes(buffers)?;
            return Err(error);
        }
        self.materialize_with_memory(memory)
            .map_err(NativeBuildError::Storage)
    }

    /// Retained native allocation bound for explicit adoption into an admission.
    ///
    /// Callers transferring a resident builder into a fresh account admit this
    /// amount before mutation. Already admitted builders must not charge it twice.
    /// This includes index capacity and owned location/configuration payloads.
    ///
    /// # Errors
    /// Returns physical layout overflow rather than an underestimated charge.
    pub fn native_buffer_bytes(&self) -> Result<usize, StorageError> {
        let locations = self
            .locations
            .iter()
            .try_fold(0usize, |total, (_, location)| {
                [
                    location.path.as_ref(),
                    location.logical.as_ref(),
                    location.subject.as_ref(),
                ]
                .into_iter()
                .flatten()
                .try_fold(total, |total, text| {
                    total
                        .checked_add(text.capacity())
                        .ok_or(StorageError::SizeOverflow)
                })
            })?;
        self.transaction_buffer_bytes()?
            .checked_add(locations)
            .ok_or(StorageError::SizeOverflow)
    }

    fn transaction_buffer_bytes(&self) -> Result<usize, StorageError> {
        fn array<T>(values: &Vec<T>) -> Result<usize, StorageError> {
            std::alloc::Layout::array::<T>(values.capacity())
                .map(|layout| layout.size())
                .map_err(|_| StorageError::SizeOverflow)
        }
        let dropped_indexes = [
            &self.quad_index,
            &self.reifier_index,
            &self.annotation_index,
        ]
        .into_iter()
        .try_fold(0usize, |total, table| {
            total
                .checked_add(
                    crate::hash::hash_table_retained_bound(table)
                        .ok_or(StorageError::SizeOverflow)?,
                )
                .ok_or(StorageError::SizeOverflow)
        })?;
        [
            self.interner.buffer_bytes()?,
            array(&self.quads)?,
            array(&self.reifiers)?,
            array(&self.annotations)?,
            array(&self.locations)?,
            array(&self.declared_graphs)?,
            array(&self.used_scopes)?,
            array(&self.scope_journal)?,
            dropped_indexes,
            self.derivation_predicate
                .as_ref()
                .map_or(0, String::capacity),
        ]
        .into_iter()
        .try_fold(0usize, |total, bytes| {
            total.checked_add(bytes).ok_or(StorageError::SizeOverflow)
        })
    }

    /// Destroy originally admitted builder buffers before releasing their charge.
    ///
    /// # Errors
    /// Returns layout overflow or the original admission's release failure.
    pub fn discard_with_memory<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let bytes = self.native_buffer_bytes()?;
        drop(self);
        memory.release_bytes(bytes)
    }

    /// Validate structure (positional constraints, ID-reference validity,
    /// triple-term acyclicity) and FREEZE into an immutable, deterministically
    /// ordered, deduplicated `Arc<RdfDataset>`.
    ///
    /// Per the no-optionality doctrine this HARD-fails (`Err`) on malformed
    /// structure — there is no degraded fallback and no silent default.
    pub fn freeze(self) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        self.validate()?.freeze().map_err(RdfDiagnostic::from)
    }

    /// Validate structure and return the type-state that is allowed to materialize
    /// the frozen dataset.
    pub fn validate(self) -> Result<ValidatedRdfDatasetBuilder, RdfDiagnostic> {
        super::validate::validate(&self)?;
        Ok(ValidatedRdfDatasetBuilder { inner: self })
    }

    /// Validate native buffers under their original physical admission.
    ///
    /// # Errors
    /// Returns storage refusal or the original admitted RDF diagnostic.
    pub fn validate_with_memory<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<ValidatedRdfDatasetBuilder, NativeBuildError> {
        let buffers = self.native_buffer_bytes()?;
        if let Err(error) = super::validate::validate_with_memory(&self, memory) {
            drop(self);
            memory.release_bytes(buffers)?;
            return Err(error);
        }
        Ok(ValidatedRdfDatasetBuilder { inner: self })
    }

    /// Borrow the accumulated quad rows (validation reads these).
    pub(crate) fn quad_rows(&self) -> &[QuadRow] {
        &self.quads
    }

    /// Borrow the accumulated named graph declarations (validation reads these).
    pub(crate) fn declared_graph_rows(&self) -> &[TermId] {
        &self.declared_graphs
    }

    /// Borrow the accumulated reifier bindings (validation reads these).
    pub(crate) fn reifier_rows(&self) -> &[(TermId, TermId, Option<TermId>)] {
        &self.reifiers
    }

    /// Borrow the accumulated annotation rows (validation reads these).
    pub(crate) fn annotation_rows(&self) -> &[(TermId, TermId, TermId, Option<TermId>)] {
        &self.annotations
    }

    /// Consume the builder and materialize the frozen dataset. Called only by
    /// [`freeze`](Self::freeze) AFTER validation has passed.
    fn materialize(self) -> Result<RdfDataset, StorageError> {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.native_buffer_bytes()?);
        self.materialize_with_memory(&mut memory)
    }
    fn materialize_with_memory<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<RdfDataset, StorageError> {
        let discarded_indexes = [
            &self.quad_index,
            &self.reifier_index,
            &self.annotation_index,
        ]
        .into_iter()
        .try_fold(0usize, |total, table| {
            total
                .checked_add(
                    crate::hash::hash_table_retained_bound(table)
                        .ok_or(StorageError::SizeOverflow)?,
                )
                .ok_or(StorageError::SizeOverflow)
        })?;
        let scope_bytes = std::alloc::Layout::array::<BlankScope>(self.used_scopes.capacity())
            .map_err(|_| StorageError::SizeOverflow)?
            .size();
        let Self {
            interner,
            quads,
            quad_index,
            mut reifiers,
            reifier_index,
            mut annotations,
            annotation_index,
            locations,
            declared_graphs,
            used_scopes,
            scope_journal,
            derivation_predicate,
            ..
        } = self;
        drop(quad_index);
        drop(reifier_index);
        drop(annotation_index);
        drop(used_scopes);
        memory.release_vec(scope_journal)?;
        memory.release_bytes(discarded_indexes)?;
        memory.release_bytes(scope_bytes)?;

        // Deterministic, reproducible frozen order: sort by id tuples. Terms keep
        // their interning (allocation) order, which is itself deterministic for a
        // fixed push sequence.
        //
        // A `QuadHandle` addresses a quad by its FROZEN ordinal, but locations are
        // pushed keyed by the *push-order* ordinal. Sorting the quads moves each one
        // to a new position, so every location handle must be remapped to its quad's
        // post-sort index — otherwise `location_of` returns a *different* quad's
        // location, an LSP break for any consumer reading through the compat bridge.
        let (quads, mut locations) = if locations.is_empty() {
            let mut quads = quads;
            quads.sort_unstable();
            (quads, locations)
        } else {
            let old_quad_bytes = std::alloc::Layout::array::<QuadRow>(quads.capacity())
                .map_err(|_| StorageError::SizeOverflow)?
                .size();
            let mut indexed = memory.collect(
                quads
                    .into_iter()
                    .enumerate()
                    .map(|(push, row)| (row, push as u32)),
            )?;
            memory.release_bytes(old_quad_bytes)?;
            indexed.sort_unstable_by_key(|(row, _)| *row);

            // Push ordinals are dense, so a dense remap is both smaller and faster
            // than hashing each ordinal. This allocation exists only for datasets
            // that actually carry source locations.
            let mut push_to_frozen =
                memory.collect(core::iter::repeat_n(u32::MAX, indexed.len()))?;
            let old_indexed_bytes = std::alloc::Layout::array::<(QuadRow, u32)>(indexed.capacity())
                .map_err(|_| StorageError::SizeOverflow)?
                .size();
            let quads = memory.collect(indexed.into_iter().enumerate().map(
                |(frozen, (row, push))| {
                    push_to_frozen[push as usize] = frozen as u32;
                    row
                },
            ))?;
            memory.release_bytes(old_indexed_bytes)?;
            let old_location_bytes =
                std::alloc::Layout::array::<(QuadHandle, RdfLocation)>(locations.capacity())
                    .map_err(|_| StorageError::SizeOverflow)?
                    .size();
            let locations = memory.collect(locations.into_iter().filter_map(|(handle, loc)| {
                push_to_frozen
                    .get(handle.index())
                    .copied()
                    .filter(|&frozen| frozen != u32::MAX)
                    .map(|frozen| (QuadHandle::from_index(frozen), loc))
            }))?;
            memory.release_bytes(old_location_bytes)?;
            memory.release_vec(push_to_frozen)?;
            (quads, locations)
        };
        reifiers.sort_unstable();
        annotations.sort_unstable();
        locations.sort_unstable_by_key(|(handle, _)| *handle);

        // The frozen `GRAPH ?g` enumeration set: every graph term that owns a quad,
        // UNION every graph the caller explicitly declared (possibly empty).
        let mut named_graphs: Vec<TermId> = declared_graphs;
        // `filter_map` reports a zero lower bound, so the three `extend`s below
        // would otherwise grow by doubling; the exact upper bound is known here.
        //
        // Guarded by a scan for any graph slot at all, because the upper bound is the
        // ROW COUNT and the common dataset has no named graph whatsoever: reserving
        // unconditionally asks the allocator for one `TermId` per row of a
        // default-graph-only dataset and then frees every byte of it again at
        // `into_boxed_slice`. The scan allocates nothing and reads the same slices the
        // extends are about to.
        if quads.iter().any(|q| q.g.is_some())
            || reifiers.iter().any(|(_, _, g)| g.is_some())
            || annotations.iter().any(|(_, _, _, g)| g.is_some())
        {
            let additional = quads
                .len()
                .checked_add(reifiers.len())
                .and_then(|count| count.checked_add(annotations.len()))
                .ok_or(StorageError::SizeOverflow)?;
            let required = named_graphs
                .len()
                .checked_add(additional)
                .ok_or(StorageError::SizeOverflow)?;
            memory.reserve(&mut named_graphs, required)?;
        }
        memory.extend(&mut named_graphs, quads.iter().filter_map(|q| q.g))?;
        // A reifier / annotation declared inside a `GRAPH g { … }` block owns no base
        // quad in g (the `<< … >>` folds entirely into the side-tables), so g would be
        // invisible to `GRAPH ?g` enumeration unless its overlay rows are counted too.
        memory.extend(
            &mut named_graphs,
            reifiers.iter().filter_map(|(_, _, g)| *g),
        )?;
        memory.extend(
            &mut named_graphs,
            annotations.iter().filter_map(|(_, _, _, g)| *g),
        )?;
        named_graphs.sort_unstable();
        named_graphs.dedup();

        let caps = compute_capabilities(
            &interner.terms,
            !named_graphs.is_empty(),
            &reifiers,
            &annotations,
            !locations.is_empty(),
        );

        RdfDataset::from_parts(
            interner.freeze_with_memory(derivation_predicate, memory)?,
            memory.boxed_slice(quads)?,
            memory.boxed_slice(reifiers)?,
            memory.boxed_slice(annotations)?,
            memory.boxed_slice(locations)?,
            caps,
            memory.boxed_slice(named_graphs)?,
            memory,
        )
    }
}

use crate::{RdfDiagnostic, RdfStoreCapabilities};

/// The composite blank-label binding that matches how
/// [`RdfDatasetBuilder::intern_owned_term_scoped`] binds a bare
/// [`RdfTerm::BlankNode`](crate::RdfTerm::BlankNode) at `scope`.
///
/// The two MUST agree, so this states the projection once instead of restating
/// the rule: at [`BlankScope::DEFAULT`] the owned rendering is decoded (the
/// exact inverse of [`BlankScope::qualify_label`]); at an explicit scope the
/// label is a standardize-apart relabeling taken verbatim into that scope.
fn owned_binding(scope: BlankScope) -> crate::cdt_blank::BlankBinding {
    if scope == BlankScope::DEFAULT {
        crate::cdt_blank::BlankBinding::Decoded(LabelAlphabet::Unconstrained)
    } else {
        crate::cdt_blank::BlankBinding::Ambient(scope)
    }
}

/// Compute the dataset's capability flags ONCE at freeze, from the frozen tables.
fn compute_capabilities<R, A>(
    terms: &[InternedTerm],
    named_graphs: bool,
    reifiers: &[R],
    annotations: &[A],
    source_locations: bool,
) -> RdfStoreCapabilities {
    RdfStoreCapabilities {
        named_graphs,
        quoted_triples: terms
            .iter()
            .any(|t| matches!(t, InternedTerm::Triple { .. })),
        reifiers: !reifiers.is_empty(),
        annotations: !annotations.is_empty(),
        source_locations,
        // The frozen dataset is the hot graph only; envelope concerns (loss
        // records, lookaside) live elsewhere (C0.6).
        loss_records: false,
        lookaside: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RdfTextDirection;
    use crate::ir::term::{RDF_DIR_LANG_STRING, RDF_LANG_STRING, XSD_STRING};
    use purrdf_testkit::prop::prelude::*;

    /// The content-id recognition prefix these tests configure.
    const SCHEME: &str = "blake3:";

    fn lit_simple(s: &str) -> RdfLiteral {
        RdfLiteral::simple(s)
    }

    #[test]
    fn intern_iri_is_idempotent() {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/x").unwrap();
        let c = b.intern_iri("http://example.org/x").unwrap();
        let d = b.intern_iri("http://example.org/y").unwrap();
        assert_eq!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn iri_hash_matches_stored_form_across_lengths_and_table_growth() {
        let mut interner = Interner::new();
        let iris: Vec<String> = (2..=33)
            .map(|len| format!("a:{}", "x".repeat(len - 2)))
            .chain((0..=128).map(|len| format!("http://example.org/{}", "x".repeat(len))))
            .collect();
        let ids: Vec<TermId> = iris
            .iter()
            .map(|iri| interner.intern(TermLookup::Iri(iri)).unwrap())
            .collect();
        for (iri, id) in iris.iter().zip(ids) {
            assert_eq!(
                hash_lookup_value(&TermLookup::Iri(iri)),
                hash_stored_value(interner.arena(), interner.term(id)),
                "borrowed and stored IRI hashes differ at length {}",
                iri.len()
            );
            assert_eq!(interner.lookup_iri(iri), Some(id));
            assert_eq!(interner.intern(TermLookup::Iri(iri)).unwrap(), id);
        }
    }

    #[test]
    fn all_term_kinds_match_borrowed_and_stored_hashes() {
        let mut interner = Interner::new();
        let iri = TermLookup::Iri(RDF_DIR_LANG_STRING);
        let datatype = interner.intern(iri).unwrap();
        let blank = TermLookup::Blank {
            label: "b",
            scope: BlankScope(7),
        };
        let blank_id = interner.intern(blank).unwrap();
        let literal = TermLookup::Literal {
            lexical: "hello",
            datatype,
            language: Some("en"),
            direction: Some(RdfTextDirection::Ltr),
        };
        let literal_id = interner.intern(literal).unwrap();
        let triple = TermLookup::Triple {
            s: blank_id,
            p: datatype,
            o: literal_id,
        };
        let triple_id = interner.intern(triple).unwrap();
        // Rehash the table after every variant is present, then verify that
        // borrowed keys still find their original stored representatives.
        for n in 0..256 {
            interner
                .intern(TermLookup::Iri(&format!("http://example.org/fill/{n}")))
                .unwrap();
        }
        for (lookup, id) in [
            (iri, datatype),
            (blank, blank_id),
            (literal, literal_id),
            (triple, triple_id),
        ] {
            let hash = hash_lookup_value(&lookup);
            assert_eq!(hash, hash_stored_value(interner.arena(), interner.term(id)));
            assert_eq!(interner.intern(lookup).unwrap(), id);
        }
        assert_eq!(interner.term_count(), 260);
    }

    /// No fabricated default: a plain `new`/`default` builder has content-id
    /// recognition INACTIVE.
    #[test]
    fn new_builder_has_no_content_scheme() {
        let b = RdfDatasetBuilder::new();
        assert!(b.interner.content_scheme.is_none());
        assert!(b.derivation_predicate.is_none());
    }

    /// `with_content_addressing` is the single construction path: it fixes the
    /// scheme (and, optionally, the derivation predicate) before any intern.
    #[test]
    fn with_content_addressing_sets_config() {
        let scheme = ContentIdScheme::new(SCHEME).expect("valid scheme");
        let b = RdfDatasetBuilder::with_content_addressing(
            scheme.clone(),
            Some("http://example.org/derivedFrom".to_string()),
        );
        assert_eq!(b.interner.content_scheme, Some(scheme));
        assert_eq!(
            b.derivation_predicate.as_deref(),
            Some("http://example.org/derivedFrom")
        );
    }

    /// A recognized `blake3:<64hex>` IRI gets a side-table entry with the exact
    /// decoded bytes.
    #[test]
    fn intern_iri_recognizes_content_id() {
        let scheme = ContentIdScheme::new(SCHEME).expect("valid scheme");
        let mut b = RdfDatasetBuilder::with_content_addressing(scheme, None);
        let hex = "ab".repeat(32);
        let id = b.intern_iri(&format!("{SCHEME}{hex}")).unwrap();
        let expected = Blake3ContentId::from_hex(&hex).expect("valid hex");
        assert_eq!(b.interner.content_id(id), Some(expected));
    }

    /// Interning the same content-id IRI twice yields the same `TermId` and
    /// exactly one side-table entry (idempotent, no double-insert drift).
    #[test]
    fn intern_iri_content_id_is_idempotent() {
        let scheme = ContentIdScheme::new(SCHEME).expect("valid scheme");
        let mut b = RdfDatasetBuilder::with_content_addressing(scheme, None);
        let iri = format!("{SCHEME}{}", "cd".repeat(32));
        let a = b.intern_iri(&iri).unwrap();
        let c = b.intern_iri(&iri).unwrap();
        assert_eq!(a, c);
        assert_eq!(b.interner.content_ids.len(), 1);
    }

    /// A prefix hit with a malformed hex suffix is an ORDINARY IRI, not an
    /// error: no side-table entry, but interning still succeeds.
    #[test]
    fn intern_iri_rejects_malformed_suffix_as_ordinary_iri() {
        let scheme = ContentIdScheme::new(SCHEME).expect("valid scheme");
        let mut b = RdfDatasetBuilder::with_content_addressing(scheme, None);

        let too_short = format!("{SCHEME}{}", "a".repeat(63));
        let too_long = format!("{SCHEME}{}", "a".repeat(65));
        let non_hex = format!("{SCHEME}{}z", "a".repeat(63));
        let uppercase = format!("{SCHEME}{}", "AB".repeat(32));

        for iri in [&too_short, &too_long, &non_hex, &uppercase] {
            let id = b.intern_iri(iri).unwrap();
            assert_eq!(
                b.interner.content_id(id),
                None,
                "unexpected match for {iri}"
            );
        }
        assert_eq!(b.interner.content_ids.len(), 0);
    }

    /// An ordinary IRI with no content-id prefix at all gets no entry.
    #[test]
    fn intern_iri_ordinary_iri_has_no_content_id() {
        let scheme = ContentIdScheme::new(SCHEME).expect("valid scheme");
        let mut b = RdfDatasetBuilder::with_content_addressing(scheme, None);
        let id = b.intern_iri("http://example.org/x").unwrap();
        assert_eq!(b.interner.content_id(id), None);
    }

    /// Recognition is IRI-arm only: a blank node whose label happens to look
    /// like a content-id IRI is never recognized (rejected by construction,
    /// not by a runtime check).
    #[test]
    fn intern_blank_never_recognized_as_content_id() {
        let scheme = ContentIdScheme::new(SCHEME).expect("valid scheme");
        let mut b = RdfDatasetBuilder::with_content_addressing(scheme, None);
        let label = format!("{SCHEME}{}", "ef".repeat(32));
        let id = b.intern_blank(&label, BlankScope::DEFAULT).unwrap();
        assert_eq!(b.interner.content_id(id), None);
        assert_eq!(b.interner.content_ids.len(), 0);
    }

    /// With recognition inactive (plain `new()`), a `blake3:<64hex>` IRI is
    /// interned as an ordinary IRI: no side-table entry.
    #[test]
    fn intern_iri_no_recognition_when_scheme_inactive() {
        let mut b = RdfDatasetBuilder::new();
        let id = b
            .intern_iri(&format!("{SCHEME}{}", "12".repeat(32)))
            .unwrap();
        assert_eq!(b.interner.content_id(id), None);
        assert_eq!(b.interner.content_ids.len(), 0);
    }

    #[test]
    fn intern_blank_is_idempotent() {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_blank("b0", BlankScope::DEFAULT).unwrap();
        let c = b.intern_blank("b0", BlankScope::DEFAULT).unwrap();
        assert_eq!(a, c);
    }

    #[test]
    fn intern_literal_is_idempotent() {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_literal(lit_simple("x")).unwrap();
        let c = b.intern_literal(lit_simple("x")).unwrap();
        assert_eq!(a, c);
    }

    /// C0.1: a plain literal expands to `xsd:string`, so `"x"` and an explicit
    /// `"x"^^xsd:string` intern to the same id.
    #[test]
    fn datatype_expansion_equality() {
        let mut b = RdfDatasetBuilder::new();
        let plain = b.intern_literal(RdfLiteral::simple("x")).unwrap();
        let explicit = b
            .intern_literal(RdfLiteral::typed("x", XSD_STRING))
            .unwrap();
        assert_eq!(plain, explicit);
    }

    /// C0.1: base direction participates in identity — same lexical + language but
    /// different direction are distinct.
    #[test]
    fn directional_literal_distinctness() {
        let mut b = RdfDatasetBuilder::new();
        let base = RdfLiteral {
            lexical_form: "x".to_string(),
            datatype: None,
            language: Some("en".to_string()),
            direction: Some(RdfTextDirection::Ltr),
        };
        let mut other = base.clone();
        other.direction = Some(RdfTextDirection::Rtl);
        let none = RdfLiteral {
            direction: None,
            ..base.clone()
        };
        let ltr = b.intern_literal(base).unwrap();
        let rtl = b.intern_literal(other).unwrap();
        let no_dir = b.intern_literal(none).unwrap();
        assert_ne!(ltr, rtl);
        assert_ne!(ltr, no_dir);
        assert_ne!(rtl, no_dir);
        let dataset = b.freeze().unwrap();
        for (id, direction, datatype) in [
            (ltr, Some(RdfTextDirection::Ltr), RDF_DIR_LANG_STRING),
            (rtl, Some(RdfTextDirection::Rtl), RDF_DIR_LANG_STRING),
            (no_dir, None, RDF_LANG_STRING),
        ] {
            let expected = crate::TermValue::Literal {
                lexical_form: "x".into(),
                datatype: datatype.into(),
                language: Some("en".into()),
                direction,
            };
            assert_eq!(dataset.term_value(id), expected);
            assert_eq!(dataset.term_id_by_value(&expected), Some(id));
        }
    }

    /// C0.1: language tags are lowercased for the key, so `@EN` and `@en` are equal.
    #[test]
    fn language_lowercasing() {
        let mut b = RdfDatasetBuilder::new();
        let upper = b
            .intern_literal(RdfLiteral::language_tagged("x", "EN"))
            .unwrap();
        let lower = b
            .intern_literal(RdfLiteral::language_tagged("x", "en"))
            .unwrap();
        assert_eq!(upper, lower);
    }

    /// A language-tagged literal expands to `rdf:langString`, distinct from a plain
    /// `xsd:string` literal of the same lexical form.
    #[test]
    fn lang_tagged_distinct_from_plain() {
        let mut b = RdfDatasetBuilder::new();
        let plain = b.intern_literal(RdfLiteral::simple("x")).unwrap();
        let tagged = b
            .intern_literal(RdfLiteral::language_tagged("x", "en"))
            .unwrap();
        assert_ne!(plain, tagged);
    }

    /// C0.2: blank-node scope participates in the key.
    #[test]
    fn blank_scope_distinctness() {
        let mut b = RdfDatasetBuilder::new();
        let s1 = b.intern_blank("b", BlankScope(1)).unwrap();
        let s2 = b.intern_blank("b", BlankScope(2)).unwrap();
        let s1_again = b.intern_blank("b", BlankScope(1)).unwrap();
        assert_ne!(s1, s2);
        assert_eq!(s1, s1_again);
    }

    /// C0.3: triple terms are identified structurally by resolved `(s, p, o)`, and
    /// a triple term nests as the object of another triple and stays reusable.
    #[test]
    fn nested_triple_term_structural_identity() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s").unwrap();
        let p = b.intern_iri("http://example.org/p").unwrap();
        let o = b.intern_iri("http://example.org/o").unwrap();

        let t1 = b.intern_triple(s, p, o).unwrap();
        let t2 = b.intern_triple(s, p, o).unwrap();
        assert_eq!(t1, t2, "same (s,p,o) → same triple-term id");

        // Nest the triple term as the object of an outer triple, twice.
        let outer_p = b.intern_iri("http://example.org/asserts").unwrap();
        let outer1 = b.intern_triple(s, outer_p, t1).unwrap();
        let outer2 = b.intern_triple(s, outer_p, t2).unwrap();
        assert_eq!(outer1, outer2, "nested triple term is reusable by id");

        // The inner triple term remains a distinct, single interned term.
        let t3 = b.intern_triple(s, p, o).unwrap();
        assert_eq!(t1, t3);
    }

    /// A small helper interning a fresh IRI by suffix.
    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(&format!("http://example.org/{n}")).unwrap()
    }

    /// `dataset → owned → dataset` preserves blank-node `(label, scope)` IDENTITY:
    /// the owned rendering encodes, and re-interning at the default scope
    /// decodes, so a re-materialized dataset holds the SAME nodes rather than a
    /// doubly-encoded copy of them.
    #[test]
    fn the_owned_round_trip_preserves_blank_label_identity() {
        for &(label, scope) in &[
            ("b0", BlankScope::DEFAULT),
            ("a.b", BlankScope::DEFAULT),
            ("a.s1", BlankScope::DEFAULT),
            ("x", BlankScope(2)),
            ("a.b", BlankScope(3)),
        ] {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_blank(label, scope).unwrap();
            let (p, o) = (iri(&mut b, "p"), iri(&mut b, "o"));
            b.push_quad(s, p, o, None).unwrap();
            let dataset = b.freeze().expect("valid");

            let mut again = RdfDatasetBuilder::new();
            for quad in dataset.owned_quads() {
                again.push_owned_quad(&quad).unwrap();
            }
            let round_tripped = again.freeze().expect("valid");

            let blanks: Vec<(String, BlankScope)> = (0..round_tripped.term_count())
                .filter_map(|index| {
                    match round_tripped.resolve(
                        TermId::from_index(index as u32)
                            .expect("stored term index is representable"),
                    ) {
                        TermRef::Blank { label, scope } => Some((label.to_owned(), scope)),
                        _ => None,
                    }
                })
                .collect();
            assert_eq!(
                blanks,
                vec![(label.to_owned(), scope)],
                "owned round trip of {label:?} @ {scope:?}"
            );
        }
    }

    /// Standardize-apart stays INJECTIVE: merging a dataset that holds the same
    /// label at two different scopes keeps the two nodes apart in the merged
    /// scope, which is why an explicit non-default scope keeps the qualified
    /// spelling instead of decoding it.
    #[test]
    fn push_dataset_keeps_two_scopes_of_one_label_apart() {
        let source = {
            let mut b = RdfDatasetBuilder::new();
            let zero = b.intern_blank("b", BlankScope::DEFAULT).unwrap();
            let one = b.intern_blank("b", BlankScope(1)).unwrap();
            let (p, o) = (iri(&mut b, "p"), iri(&mut b, "o"));
            b.push_quad(zero, p, o, None).unwrap();
            b.push_quad(one, p, o, None).unwrap();
            b.freeze().expect("valid")
        };
        let mut merged = RdfDatasetBuilder::new();
        merged.push_dataset(&source).unwrap();
        let merged = merged.freeze().expect("valid");

        let blanks: std::collections::BTreeSet<(String, BlankScope)> = (0..merged.term_count())
            .filter_map(|index| {
                match merged.resolve(
                    TermId::from_index(index as u32).expect("stored term index is representable"),
                ) {
                    TermRef::Blank { label, scope } => Some((label.to_owned(), scope)),
                    _ => None,
                }
            })
            .collect();
        assert_eq!(
            blanks.len(),
            2,
            "the merge must not conflate two scopes of one label: {blanks:?}"
        );
    }

    #[test]
    fn freeze_dedupes_quads() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        b.push_quad(s, p, o, None).unwrap();
        b.push_quad(s, p, o, None).unwrap();
        let ds = b.freeze().expect("valid");
        assert_eq!(ds.quads().count(), 1, "duplicate quads collapse to one row");
    }

    #[test]
    fn validated_builder_type_state_freezes_after_validation() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        b.push_quad(s, p, o, None).unwrap();

        let validated = b.validate().expect("well-formed builder validates");
        let ds = validated.freeze().expect("validated buffers materialize");
        assert_eq!(ds.quads().count(), 1);
    }

    #[test]
    fn freeze_preserves_named_graphs() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let g = iri(&mut b, "g");
        b.push_quad(s, p, o, None).unwrap();
        b.push_quad(s, p, o, Some(g)).unwrap();
        let ds = b.freeze().expect("valid");
        assert_eq!(
            ds.quads().count(),
            2,
            "default and named graph are distinct"
        );
        assert!(ds.capabilities().named_graphs);
        let graphs: Vec<_> = ds.quads().map(|q| q.g).collect();
        assert!(graphs.contains(&None));
        assert!(graphs.contains(&Some(g)));
    }

    #[test]
    fn freeze_keeps_multiple_reifiers_for_one_triple() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let triple = b.intern_triple(s, p, o).unwrap();
        let r1 = iri(&mut b, "r1");
        let r2 = iri(&mut b, "r2");
        b.push_reifier(r1, triple).unwrap();
        b.push_reifier(r2, triple).unwrap();
        b.push_reifier(r1, triple).unwrap(); // duplicate binding collapses
        let ds = b.freeze().expect("valid");
        let reifiers: Vec<_> = ds.reifiers().collect();
        assert_eq!(
            reifiers.len(),
            2,
            "two distinct reifiers survive, dup collapses"
        );
        assert!(reifiers.contains(&(r1, triple)));
        assert!(reifiers.contains(&(r2, triple)));
    }

    #[test]
    fn freeze_dedupes_annotations() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let triple = b.intern_triple(s, p, o).unwrap();
        let r = iri(&mut b, "r");
        let ap = iri(&mut b, "ap");
        let ao = iri(&mut b, "ao");
        b.push_reifier(r, triple).unwrap();
        b.push_annotation(r, ap, ao).unwrap();
        b.push_annotation(r, ap, ao).unwrap();
        let ds = b.freeze().expect("valid");
        assert_eq!(
            ds.annotations().count(),
            1,
            "duplicate annotation collapses"
        );
    }

    /// `push_dataset` re-interns a foreign dataset's base quads into a fresh
    /// builder's interner; quads shared with an already-merged dataset collapse.
    #[test]
    fn push_dataset_merges_and_dedupes_quads() {
        let dataset_a = {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o, o2) = (
                iri(&mut b, "s"),
                iri(&mut b, "p"),
                iri(&mut b, "o"),
                iri(&mut b, "o2"),
            );
            b.push_quad(s, p, o, None).unwrap();
            b.push_quad(s, p, o2, None).unwrap();
            b.freeze().expect("valid A")
        };
        let dataset_b = {
            let mut b = RdfDatasetBuilder::new();
            let (s, s2, p, o, o2) = (
                iri(&mut b, "s"),
                iri(&mut b, "s2"),
                iri(&mut b, "p"),
                iri(&mut b, "o"),
                iri(&mut b, "o2"),
            );
            b.push_quad(s2, p, o, None).unwrap(); // B-only quad
            b.push_quad(s, p, o2, None).unwrap(); // shared with A → collapses on merge
            b.freeze().expect("valid B")
        };

        let mut merged = RdfDatasetBuilder::new();
        merged.push_dataset(&dataset_a).unwrap();
        merged.push_dataset(&dataset_b).unwrap();
        let merged = merged.freeze().expect("valid merged");

        assert_eq!(
            merged.quad_count(),
            3,
            "A(2) + B(2) with one shared quad → 3 distinct rows"
        );
        // The B-only quad resolves through the merged interner.
        let has_b_only = merged.owned_quads().any(|q| {
            q.subject == RdfTerm::Iri("http://example.org/s2".to_string())
                && q.predicate == "http://example.org/p"
                && q.object == RdfTerm::Iri("http://example.org/o".to_string())
        });
        assert!(has_b_only, "B-only quad survives the cross-dataset merge");
    }

    /// `push_dataset` carries the FULL RDF 1.2 statement layer: a merged dataset's
    /// reifier bindings AND annotations survive, not just its base quads. (A
    /// quad-only merge would silently drop these — the regression this guards.)
    #[test]
    fn push_dataset_carries_reifiers_and_annotations() {
        let dataset = {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let triple = b.intern_triple(s, p, o).unwrap();
            let r = iri(&mut b, "r");
            let ap = iri(&mut b, "ap");
            let ao = iri(&mut b, "ao");
            b.push_reifier(r, triple).unwrap();
            b.push_annotation(r, ap, ao).unwrap();
            b.freeze().expect("valid source")
        };

        let mut merged = RdfDatasetBuilder::new();
        merged.push_dataset(&dataset).unwrap();
        let merged = merged.freeze().expect("valid merged");

        assert_eq!(
            merged.reifiers().count(),
            1,
            "reifier binding survives merge"
        );
        assert_eq!(merged.annotations().count(), 1, "annotation survives merge");

        let reifier = merged.owned_reifiers().next().expect("one reifier");
        assert_eq!(
            reifier.reifier,
            RdfTerm::Iri("http://example.org/r".to_string())
        );
        assert_eq!(
            reifier.statement.subject,
            RdfTerm::Iri("http://example.org/s".to_string())
        );
        let annotation = merged.owned_annotations().next().expect("one annotation");
        assert_eq!(
            annotation.reifier,
            RdfTerm::Iri("http://example.org/r".to_string())
        );
        assert_eq!(annotation.predicate, "http://example.org/ap");
        assert_eq!(
            annotation.object,
            RdfTerm::Iri("http://example.org/ao".to_string())
        );
    }

    /// `push_dataset` must NOT collapse blank nodes that have the same label but
    /// originate from DIFFERENT source datasets (standardize-apart, C0.2).
    ///
    /// WITHOUT standardize-apart these would collapse to one node.
    #[test]
    fn push_dataset_standardizes_apart_blank_nodes() {
        // Dataset A: single quad  _:b0 <ex:p> <ex:o1>
        let dataset_a = {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_blank("b0", BlankScope::DEFAULT).unwrap();
            let p = b.intern_iri("http://example.org/p").unwrap();
            let o = b.intern_iri("http://example.org/o1").unwrap();
            b.push_quad(s, p, o, None).unwrap();
            b.freeze().expect("valid A")
        };

        // Dataset B: single quad  _:b0 <ex:p> <ex:o2>  (same blank label, different object)
        let dataset_b = {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_blank("b0", BlankScope::DEFAULT).unwrap();
            let p = b.intern_iri("http://example.org/p").unwrap();
            let o = b.intern_iri("http://example.org/o2").unwrap();
            b.push_quad(s, p, o, None).unwrap();
            b.freeze().expect("valid B")
        };

        let mut merged = RdfDatasetBuilder::new();
        merged.push_dataset(&dataset_a).unwrap();
        merged.push_dataset(&dataset_b).unwrap();
        let merged = merged.freeze().expect("valid merged");

        // Both quads must survive: without standardize-apart the two _:b0 subjects
        // would collapse into one node and one of the two objects would be lost.
        assert_eq!(merged.quad_count(), 2, "both quads survive the merge");

        // The two blank subject nodes must be DISTINCT (different TermIds / qualified
        // labels), because they came from different source datasets.
        let subjects: Vec<_> = merged.owned_quads().map(|q| q.subject).collect();
        assert_eq!(subjects.len(), 2);
        // Collect the unique subject labels; standardize-apart gives them distinct
        // qualified labels via BlankScope::qualify_label.
        let subject_labels: crate::FastSet<String> = subjects
            .iter()
            .filter_map(|t| {
                if let RdfTerm::BlankNode(label) = t {
                    Some(label.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            subject_labels.len(),
            2,
            "two distinct blank-node subjects after standardize-apart; got: {subject_labels:?}"
        );
    }

    /// A `Strategy` producing one of a small fixed pool of distinct intern calls,
    /// so we can count the distinct *values* requested and compare to `term_count`.
    #[derive(Clone, Debug)]
    enum Op {
        Iri(u8),
        Blank(u8, u32),
        Literal(u8),
    }

    fn op_strategy() -> impl Strategy<Value = Op> {
        prop_oneof![
            (0u8..4).prop_map(Op::Iri),
            (0u8..4, 0u32..3).prop_map(|(l, s)| Op::Blank(l, s)),
            (0u8..4).prop_map(Op::Literal),
        ]
    }

    prop_test! {
        /// Idempotence holds across arbitrary call sequences, and `term_count`
        /// never exceeds the number of distinct values interned.
        ///
        /// The distinct-value count is computed independently of the interner: a
        /// literal also interns its datatype IRI, so each literal value contributes
        /// itself plus its (shared) datatype term to the upper bound.
        #[test]
        fn property_idempotence_and_bounded_count(ops in prop::collection::vec(op_strategy(), 0..64)) {
            let mut b = RdfDatasetBuilder::new();
            // Map a value-key → the id it first produced, to assert idempotence.
            let mut seen: crate::FastMap<String, TermId> = crate::FastMap::default();
            // The set of distinct *terms* (value keys, incl. datatype IRIs) that
            // SHOULD exist after the run — the exact upper bound for term_count.
            let mut distinct_terms: crate::FastSet<String> = crate::FastSet::default();

            for op in ops {
                let (call_key, id) = match op {
                    Op::Iri(n) => {
                        let iri = format!("http://example.org/i{n}");
                        distinct_terms.insert(format!("iri:{iri}"));
                        (format!("iri:{iri}"), b.intern_iri(&iri).unwrap())
                    }
                    Op::Blank(l, s) => {
                        let key = format!("blank:{l}:{s}");
                        distinct_terms.insert(key.clone());
                        (key, b.intern_blank(&format!("b{l}"), BlankScope(s)).unwrap())
                    }
                    Op::Literal(n) => {
                        let lex = format!("v{n}");
                        // Plain literal → xsd:string; both the literal and its
                        // datatype IRI become distinct interned terms.
                        distinct_terms.insert(format!("lit:{lex}"));
                        distinct_terms.insert(format!("iri:{XSD_STRING}"));
                        (format!("lit:{lex}"), b.intern_literal(RdfLiteral::simple(lex)).unwrap())
                    }
                };

                match seen.get(&call_key) {
                    Some(&prev) => prop_assert_eq!(prev, id, "intern not idempotent"),
                    None => {
                        seen.insert(call_key, id);
                    }
                }
            }

            prop_assert_eq!(b.term_count(), distinct_terms.len());
        }
    }
}

#[cfg(test)]
mod native_graph_birth_tests {
    use super::*;

    #[test]
    fn local_capacity_checks_logical_sum_before_host_width_and_skips_only_hints() {
        use purrdf_lex::allocation::CapacityError;
        for space in [
            CapacitySpace::LocalTerms,
            CapacitySpace::LocalTermBytes,
            CapacitySpace::Quads,
            CapacitySpace::Reifiers,
            CapacitySpace::Annotations,
        ] {
            let last = usize::try_from(u32::MAX).unwrap();
            assert_eq!(
                LocalCapacity::Native.required(space, last, 1),
                Err(StorageError::Capacity(CapacityError::new(
                    space,
                    u128::from(u32::MAX) + 1,
                    u128::from(u32::MAX)
                )))
            );
            assert_eq!(LocalCapacity::Native.hint(space, last, 1), last);
            assert_eq!(LocalCapacity::Native.required(space, last - 1, 1), Ok(last));
        }
    }

    #[test]
    fn local_capacity_public_rows_and_duplicate_full_import_preserve_original_state() {
        use crate::ir::import::DatasetImporter;
        use purrdf_lex::allocation::CapacityError;
        let fixture = || {
            let mut builder = RdfDatasetBuilder::new();
            let s = builder.intern_iri("urn:s").unwrap();
            let p = builder.intern_iri("urn:p").unwrap();
            let o = builder.intern_iri("urn:o").unwrap();
            let t = builder.intern_triple(s, p, o).unwrap();
            builder.push_quad(s, p, o, None).unwrap();
            builder.push_reifier(s, t).unwrap();
            builder.push_annotation(s, p, o).unwrap();
            (builder, [s, p, o, t])
        };
        let (mut builder, [s, p, o, t]) = fixture();
        let source = fixture().0.freeze().unwrap();
        let terms = u32::try_from(builder.term_count()).unwrap();
        let text = u32::try_from(builder.interner.arena.len()).unwrap();
        builder.interner.capacity = LocalCapacity::Reduced {
            terms,
            text,
            rows: 1,
        };
        let bytes = builder.native_buffer_bytes().unwrap();
        let mut grant = Grant {
            live: bytes,
            limit: bytes,
            peak: bytes,
        };
        {
            let mut memory = Memory::resume(&mut grant, bytes);
            builder
                .push_quad_with_memory(s, p, o, None, &mut memory)
                .unwrap();
            builder
                .push_reifier_in_graph_with_memory(s, t, None, &mut memory)
                .unwrap();
            builder
                .push_annotation_in_graph_with_memory(s, p, o, None, &mut memory)
                .unwrap();
            assert_eq!(
                builder
                    .intern_iri_with_memory("urn:s", &mut memory)
                    .unwrap(),
                s
            );
            assert_eq!(memory.admitted_bytes(), bytes);
        }
        assert_eq!(grant.peak, bytes);
        // Each source count would overflow the full destination if treated as a
        // number of new entries. Replay must discover the existing identities.
        DatasetImporter::new(&mut builder, source.as_ref())
            .unwrap()
            .append()
            .unwrap();
        assert_eq!(builder.native_buffer_bytes().unwrap(), bytes);
        let mut new_source = RdfDatasetBuilder::new();
        let source_s = new_source.intern_iri("urn:s").unwrap();
        let source_p = new_source.intern_iri("urn:p").unwrap();
        let source_o = new_source.intern_iri("urn:new").unwrap();
        new_source
            .push_quad(source_s, source_p, source_o, None)
            .unwrap();
        let new_source = new_source.freeze().unwrap();
        for _ in 0..3 {
            assert_eq!(
                crate::term_fixture::intern_value(&mut builder, &crate::TermValue::iri("urn:s")),
                Ok(s)
            );
            assert_eq!(
                crate::term_fixture::intern_value(&mut builder, &crate::TermValue::iri("urn:new")),
                Err(StorageError::Capacity(CapacityError::new(
                    CapacitySpace::LocalTerms,
                    u128::from(terms) + 1,
                    u128::from(terms)
                )))
            );
            let result = DatasetImporter::new(&mut builder, new_source.as_ref())
                .unwrap()
                .append();
            let Err(crate::ir::import::DatasetImportError::Capacity(error)) = result else {
                panic!("public replay must preserve its actual term-capacity cause")
            };
            assert_eq!(
                error,
                CapacityError::new(
                    CapacitySpace::LocalTerms,
                    u128::from(terms) + 1,
                    u128::from(terms)
                )
            );
            assert_eq!(builder.term_count(), terms as usize);
            assert_eq!(builder.native_buffer_bytes().unwrap(), bytes);
        }
        for (space, result) in [
            (CapacitySpace::Quads, builder.push_quad(o, p, s, None)),
            (CapacitySpace::Reifiers, builder.push_reifier(o, t)),
            (CapacitySpace::Annotations, builder.push_annotation(o, p, s)),
        ] {
            assert_eq!(
                result,
                Err(StorageError::Capacity(CapacityError::new(space, 2, 1)))
            );
        }
        assert_eq!(
            builder.intern_iri("urn:new"),
            Err(StorageError::Capacity(CapacityError::new(
                CapacitySpace::LocalTerms,
                u128::from(terms) + 1,
                u128::from(terms)
            )))
        );
        assert_eq!(
            (
                builder.quads.len(),
                builder.reifiers.len(),
                builder.annotations.len()
            ),
            (1, 1, 1)
        );
        assert_eq!(builder.native_buffer_bytes().unwrap(), bytes);
        builder.interner.capacity = LocalCapacity::Reduced {
            terms: terms + 1,
            text: text + 7,
            rows: 2,
        };
        builder.push_quad(o, p, s, None).unwrap();
        builder.push_reifier(o, t).unwrap();
        builder.push_annotation(o, p, s).unwrap();
        let new_id =
            crate::term_fixture::intern_value(&mut builder, &crate::TermValue::iri("urn:new"))
                .unwrap();
        assert_eq!(builder.intern_iri("urn:new"), Ok(new_id));
        assert_eq!(
            (
                builder.quads.len(),
                builder.reifiers.len(),
                builder.annotations.len()
            ),
            (2, 2, 2)
        );
        builder.freeze().unwrap();
    }

    #[test]
    fn local_capacity_public_text_miss_is_atomic_and_duplicate_survives() {
        use purrdf_lex::allocation::CapacityError;
        let mut builder = RdfDatasetBuilder::new();
        builder.interner.capacity = LocalCapacity::Reduced {
            terms: 10,
            text: 5,
            rows: 1,
        };
        let first = builder.intern_iri("urn:a").unwrap();
        let bytes = builder.native_buffer_bytes().unwrap();
        assert_eq!(builder.intern_iri("urn:a").unwrap(), first);
        assert_eq!(
            builder.intern_iri("urn:b"),
            Err(StorageError::Capacity(CapacityError::new(
                CapacitySpace::LocalTermBytes,
                10,
                5
            )))
        );
        assert_eq!(builder.term_count(), 1);
        assert_eq!(builder.interner.arena, b"urn:a");
        assert_eq!(builder.native_buffer_bytes().unwrap(), bytes);
        builder.interner.capacity = LocalCapacity::Reduced {
            terms: 10,
            text: 10,
            rows: 1,
        };
        builder.intern_iri("urn:b").unwrap();
        assert_eq!(builder.term_count(), 2);
        builder.freeze().unwrap();
    }

    struct Grant {
        live: usize,
        limit: usize,
        peak: usize,
    }
    impl Admission for Grant {
        fn resize(&mut self, live: usize) -> Result<(), StorageError> {
            if live > self.limit {
                return Err(StorageError::AdmissionFailed);
            }
            self.live = live;
            self.peak = self.peak.max(live);
            Ok(())
        }
    }

    struct GrowthGrant {
        live: usize,
        remaining: usize,
    }

    impl Admission for GrowthGrant {
        fn resize(&mut self, live: usize) -> Result<(), StorageError> {
            if live > self.live {
                self.remaining = self
                    .remaining
                    .checked_sub(1)
                    .ok_or(StorageError::AdmissionFailed)?;
            }
            self.live = live;
            Ok(())
        }
    }

    #[test]
    fn compound_literal_refusal_rolls_back_children_and_sorted_scopes() {
        let lexical = format!(
            "[_:{}, _:{}]",
            BlankScope(1).qualify_label("new"),
            BlankScope(5).qualify_label("other")
        );
        let mut failures = 0;
        let mut completed = false;
        for allowed in 0..128 {
            let mut grant = GrowthGrant {
                live: 0,
                remaining: usize::MAX,
            };
            let mut memory = Memory::new(&mut grant);
            let mut builder = RdfDatasetBuilder::new();
            let first = builder
                .intern_blank_with_memory("prior", BlankScope(2), &mut memory)
                .unwrap();
            let second = builder
                .intern_blank_with_memory("later", BlankScope(7), &mut memory)
                .unwrap();
            let healthy = builder
                .intern_literal_parts_with_memory("healthy", None, None, None, &mut memory)
                .unwrap();
            let terms = builder.term_count();
            let arena = builder.interner.arena.clone();
            let scopes = builder.used_scopes.clone();
            let next_scope = builder.next_merge_scope;
            memory.admission_mut().remaining = allowed;
            let result = builder.intern_literal_parts_with_memory(
                &lexical,
                Some(purrdf_cdt::CDT_LIST),
                None,
                None,
                &mut memory,
            );
            if let Err(error) = result {
                assert_eq!(error, StorageError::AdmissionFailed);
                failures += 1;
                assert_eq!(builder.term_count(), terms, "growth boundary {allowed}");
                assert_eq!(builder.interner.arena, arena);
                assert_eq!(builder.interner.index.len(), terms);
                assert_eq!(builder.used_scopes, scopes);
                assert_eq!(builder.next_merge_scope, next_scope);
                assert_eq!(builder.reifies_predicate, None);
                assert_eq!(builder.interner.invalid_literal, None);
                assert_eq!(builder.scope_journal.len(), 0);
                assert_eq!(builder.transaction_depth, 0);
                assert_eq!(
                    memory.admitted_bytes(),
                    builder.native_buffer_bytes().unwrap()
                );
                memory.admission_mut().remaining = 0;
                assert_eq!(
                    builder
                        .intern_literal_parts_with_memory("healthy", None, None, None, &mut memory)
                        .unwrap(),
                    healthy
                );
                assert_eq!(
                    builder
                        .intern_blank_with_memory("prior", BlankScope(2), &mut memory)
                        .unwrap(),
                    first
                );
                assert_eq!(
                    builder
                        .intern_blank_with_memory("later", BlankScope(7), &mut memory)
                        .unwrap(),
                    second
                );
                memory.admission_mut().remaining = usize::MAX;
                builder
                    .intern_literal_parts_with_memory(
                        &lexical,
                        Some(purrdf_cdt::CDT_LIST),
                        None,
                        None,
                        &mut memory,
                    )
                    .unwrap();
            } else {
                completed = true;
            }
            assert_eq!(
                builder.used_scopes,
                [BlankScope(1), BlankScope(2), BlankScope(5), BlankScope(7)]
            );
            memory.admission_mut().remaining = 0;
            let existing = builder
                .intern_literal_parts_with_memory(
                    &lexical,
                    Some(purrdf_cdt::CDT_LIST),
                    None,
                    None,
                    &mut memory,
                )
                .expect("an admitted compound hit does not allocate parsing scratch");
            assert_eq!(existing.index(), builder.term_count() - 1);
            assert_eq!(
                memory.admitted_bytes(),
                builder.native_buffer_bytes().unwrap()
            );
            builder.discard_with_memory(&mut memory).unwrap();
            assert_eq!(memory.admitted_bytes(), 0);
            if completed {
                break;
            }
        }
        assert!(
            completed,
            "all growth boundaries must eventually be admitted"
        );
        assert!(
            failures > 3,
            "exercise child and final literal growth, not only initial admission"
        );
    }

    #[test]
    fn owned_quad_refusal_restores_terms_rows_scopes_and_location_payloads() {
        let mut object = RdfTerm::Literal(RdfLiteral::simple("leaf"));
        for _ in 0..20 {
            object = RdfTerm::Triple(Box::new(crate::RdfTriple::new(
                RdfTerm::BlankNode(BlankScope(5).qualify_label("child").into_owned()),
                "http://example.org/nested",
                object,
            )));
        }
        let quad = RdfQuad::new(
            RdfTerm::BlankNode(BlankScope(1).qualify_label("subject").into_owned()),
            "http://example.org/predicate",
            object,
        )
        .with_location(RdfLocation {
            path: Some("input.ttl".to_owned()),
            logical: Some("admitted owned ingress".to_owned()),
            subject: Some("subject".to_owned()),
            ..RdfLocation::default()
        });
        let mut failures = 0;
        for allowed in 0..256 {
            let mut grant = GrowthGrant {
                live: 0,
                remaining: usize::MAX,
            };
            let mut memory = Memory::new(&mut grant);
            let mut builder = RdfDatasetBuilder::new();
            let old = builder
                .intern_blank_with_memory("old", BlankScope(2), &mut memory)
                .unwrap();
            let arena = builder.interner.arena.clone();
            memory.admission_mut().remaining = allowed;
            let result =
                builder.push_owned_quad_scoped_with_memory(&quad, BlankScope::DEFAULT, &mut memory);
            if let Err(error) = result {
                assert_eq!(error, StorageError::AdmissionFailed);
                failures += 1;
                assert_eq!(builder.term_count(), 1, "growth boundary {allowed}");
                assert_eq!(builder.interner.arena, arena);
                assert_eq!(builder.interner.index.len(), 1);
                assert_eq!(builder.quads.len(), 0);
                assert_eq!(builder.quad_index.len(), 0);
                assert_eq!(builder.locations.len(), 0);
                assert_eq!(builder.used_scopes, [BlankScope(2)]);
                assert_eq!(builder.scope_journal.len(), 0);
                assert_eq!(builder.transaction_depth, 0);
                assert_eq!(
                    memory.admitted_bytes(),
                    builder.native_buffer_bytes().unwrap()
                );
                memory.admission_mut().remaining = 0;
                assert_eq!(
                    builder
                        .intern_blank_with_memory("old", BlankScope(2), &mut memory)
                        .unwrap(),
                    old
                );
                memory.admission_mut().remaining = usize::MAX;
                builder
                    .push_owned_quad_scoped_with_memory(&quad, BlankScope::DEFAULT, &mut memory)
                    .unwrap();
            }
            assert_eq!(builder.quads.len(), 1);
            assert_eq!(builder.locations[0].1, *quad.location.as_ref().unwrap());
            assert_eq!(
                builder.used_scopes,
                [BlankScope(1), BlankScope(2), BlankScope(5)]
            );
            assert_eq!(
                memory.admitted_bytes(),
                builder.native_buffer_bytes().unwrap()
            );
            builder.discard_with_memory(&mut memory).unwrap();
            assert_eq!(memory.admitted_bytes(), 0);
            if result.is_ok() {
                assert!(
                    failures > 10,
                    "include spilled walk and final location admission"
                );
                return;
            }
        }
        panic!("owned-quad growth sweep must reach complete admission");
    }

    #[test]
    fn checked_owned_document_refusal_rolls_back_prefix_and_retains_malformed_cause() {
        let mut object = RdfTerm::Literal(RdfLiteral::typed(
            format!("['[<relative>]'^^<{}>]", purrdf_cdt::CDT_LIST),
            purrdf_cdt::CDT_LIST,
        ));
        for _ in 0..20 {
            object = RdfTerm::Triple(Box::new(crate::RdfTriple::new(
                RdfTerm::BlankNode("prefix".to_owned()),
                "http://example.org/p",
                object,
            )));
        }
        let mut grant = GrowthGrant {
            live: 0,
            remaining: usize::MAX,
        };
        let mut memory = Memory::new(&mut grant);
        let mut builder = RdfDatasetBuilder::new();
        let prior = builder
            .intern_blank_with_memory("prior", BlankScope(2), &mut memory)
            .unwrap();
        for _ in 0..3 {
            let error = builder
                .intern_owned_term_bound_with_memory(&object, BlankScope(1), &mut memory)
                .unwrap_err();
            assert!(
                matches!(&error, crate::cdt_blank::CdtBlankError::Malformed {
                source: purrdf_cdt::CdtError::NotAbsoluteIri { iri, .. }, ..
            } if iri == "relative")
            );
            assert_eq!(builder.term_count(), 1);
            assert_eq!(builder.used_scopes, [BlankScope(2)]);
            assert_eq!(builder.scope_journal.len(), 0);
            let bytes = error.owned_bytes().unwrap();
            assert!(bytes > 0);
            assert_eq!(
                memory.admitted_bytes(),
                builder.native_buffer_bytes().unwrap() + bytes
            );
            drop(error);
            memory.release_bytes(bytes).unwrap();
        }
        memory.admission_mut().remaining = 0;
        assert_eq!(
            builder
                .intern_blank_with_memory("prior", BlankScope(2), &mut memory)
                .unwrap(),
            prior
        );
        memory.admission_mut().remaining = usize::MAX;
        builder
            .intern_owned_term_bound_with_memory(
                &RdfTerm::Literal(RdfLiteral::typed("[_:healthy]", purrdf_cdt::CDT_LIST)),
                BlankScope(1),
                &mut memory,
            )
            .unwrap();
        assert_eq!(
            memory.admitted_bytes(),
            builder.native_buffer_bytes().unwrap()
        );
        builder.discard_with_memory(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn checked_owned_document_growth_sweep_uses_original_admission() {
        let literal = RdfLiteral::typed(
            format!("[_:outer, '[_:inner]'^^<{}>]", purrdf_cdt::CDT_LIST),
            purrdf_cdt::CDT_LIST,
        );
        let term = RdfTerm::Triple(Box::new(crate::RdfTriple::new(
            RdfTerm::BlankNode("prefix".to_owned()),
            "http://example.org/p",
            RdfTerm::Literal(literal),
        )));
        let mut failures = 0;
        for allowed in 0..256 {
            let mut grant = GrowthGrant {
                live: 0,
                remaining: usize::MAX,
            };
            let mut memory = Memory::new(&mut grant);
            let mut builder = RdfDatasetBuilder::new();
            let prior = builder
                .intern_blank_with_memory("prior", BlankScope(2), &mut memory)
                .unwrap();
            memory.admission_mut().remaining = allowed;
            let result =
                builder.intern_owned_term_bound_with_memory(&term, BlankScope(1), &mut memory);
            if let Err(error) = &result {
                assert!(matches!(
                    error,
                    crate::cdt_blank::CdtBlankError::Storage(StorageError::AdmissionFailed)
                ));
                failures += 1;
                assert_eq!(builder.term_count(), 1, "growth boundary {allowed}");
                assert_eq!(builder.used_scopes, [BlankScope(2)]);
                assert_eq!(builder.scope_journal.len(), 0);
                assert_eq!(
                    memory.admitted_bytes(),
                    builder.native_buffer_bytes().unwrap()
                );
                memory.admission_mut().remaining = 0;
                assert_eq!(
                    builder
                        .intern_blank_with_memory("prior", BlankScope(2), &mut memory)
                        .unwrap(),
                    prior
                );
                memory.admission_mut().remaining = usize::MAX;
                builder
                    .intern_owned_term_bound_with_memory(&term, BlankScope(1), &mut memory)
                    .unwrap();
            }
            assert_eq!(
                memory.admitted_bytes(),
                builder.native_buffer_bytes().unwrap()
            );
            builder.discard_with_memory(&mut memory).unwrap();
            assert_eq!(memory.admitted_bytes(), 0);
            if result.is_ok() {
                assert!(
                    failures > 20,
                    "cover grammar, scanner, embedded parse and native publication"
                );
                return;
            }
        }
        panic!("checked document growth sweep must complete");
    }

    #[test]
    fn merge_scope_capacity_is_minted_only_for_actual_blank_ingress() {
        let empty = RdfDatasetBuilder::new().freeze().unwrap();
        let mut plain = RdfDatasetBuilder::new();
        let s = plain.intern_iri("urn:plain-s").unwrap();
        let p = plain.intern_iri("urn:p").unwrap();
        let o = plain
            .intern_literal(RdfLiteral::typed("[1]", purrdf_cdt::CDT_LIST))
            .unwrap();
        plain.push_quad(s, p, o, None).unwrap();
        let plain = plain.freeze().unwrap();
        for shape in 0..3 {
            let mut source = RdfDatasetBuilder::new();
            let s = source.intern_iri("urn:source-s").unwrap();
            let p = source.intern_iri("urn:p").unwrap();
            let object = match shape {
                0 => source.intern_blank("direct", BlankScope(7)).unwrap(),
                1 => {
                    let blank = source.intern_blank("nested", BlankScope(8)).unwrap();
                    let triple = source.intern_triple(blank, p, s).unwrap();
                    source.intern_triple(s, p, triple).unwrap()
                }
                _ => source
                    .intern_literal(RdfLiteral::typed(
                        format!("['[_:embedded]'^^<{}>]", purrdf_cdt::CDT_LIST),
                        purrdf_cdt::CDT_LIST,
                    ))
                    .unwrap(),
            };
            source.push_quad(s, p, object, None).unwrap();
            let source = source.freeze().unwrap();
            let mut destination = RdfDatasetBuilder::new();
            let exhausted = u64::from(u32::MAX) + 1;
            destination.next_merge_scope = exhausted;
            destination.push_dataset(&empty).unwrap();
            destination.push_dataset(&plain).unwrap();
            let terms = destination.term_count();
            destination.push_dataset(&plain).unwrap();
            assert_eq!(destination.term_count(), terms);
            assert_eq!(destination.quads.len(), 1);
            assert_eq!(destination.next_merge_scope, exhausted);
            assert_eq!(destination.used_scopes.len(), 0);
            let expected = StorageError::Capacity(purrdf_lex::allocation::CapacityError::new(
                CapacitySpace::BlankScopes,
                u128::from(exhausted),
                u128::from(u32::MAX),
            ));
            for _ in 0..3 {
                assert_eq!(destination.push_dataset(&source), Err(expected));
                assert_eq!(destination.term_count(), terms);
                assert_eq!(destination.quads.len(), 1);
                assert_eq!(destination.used_scopes.len(), 0);
                assert_eq!(destination.scope_journal.len(), 0);
                assert_eq!(destination.next_merge_scope, exhausted);
            }
            destination.next_merge_scope = u64::from(u32::MAX);
            destination.push_dataset(&source).unwrap();
            assert_eq!(destination.next_merge_scope, exhausted);
            assert_eq!(destination.used_scopes, [BlankScope(u32::MAX)]);
            assert_eq!(destination.quads.len(), 2);
            destination.freeze().unwrap();
        }
    }

    #[test]
    fn push_dataset_refusal_rolls_back_all_streams_and_merge_scope() {
        let mut source = RdfDatasetBuilder::new();
        let first = source.intern_blank("same", BlankScope(1)).unwrap();
        let second = source.intern_blank("same", BlankScope(5)).unwrap();
        let predicate = source.intern_iri("http://example.org/p").unwrap();
        let graph = source.intern_iri("http://example.org/graph").unwrap();
        let empty = source.intern_iri("http://example.org/empty").unwrap();
        let lexical = format!(
            "[_:{}, _:{}]",
            BlankScope(1).qualify_label("same"),
            BlankScope(5).qualify_label("same")
        );
        let literal = source
            .intern_literal(RdfLiteral::typed(lexical, purrdf_cdt::CDT_LIST))
            .unwrap();
        let mut triple = source.intern_triple(first, predicate, second).unwrap();
        // The immutable source retains all 16 nested triple terms.
        for _ in 0..15 {
            triple = source.intern_triple(first, predicate, triple).unwrap();
        }
        let handle = source
            .push_quad_with_handle(first, predicate, literal, Some(graph))
            .unwrap();
        let location = RdfLocation {
            path: Some("source.ttl".to_owned()),
            ..RdfLocation::default()
        };
        source.attach_location(handle, location.clone());
        source
            .push_reifier_in_graph(second, triple, Some(graph))
            .unwrap();
        source
            .push_annotation_in_graph(second, predicate, literal, Some(graph))
            .unwrap();
        source.declare_named_graph(empty).unwrap();
        let source = source.freeze().unwrap();
        let mut failures = 0;
        for allowed in 0..512 {
            let mut grant = GrowthGrant {
                live: 0,
                remaining: usize::MAX,
            };
            let mut memory = Memory::new(&mut grant);
            let mut destination = RdfDatasetBuilder::new();
            let prior = destination
                .intern_blank_with_memory("prior", BlankScope(2), &mut memory)
                .unwrap();
            let arena = destination.interner.arena.clone();
            memory.admission_mut().remaining = allowed;
            let result = destination.push_dataset_with_memory(&source, &mut memory);
            if let Err(error) = result {
                assert_eq!(error, StorageError::AdmissionFailed);
                failures += 1;
                assert_eq!(destination.term_count(), 1, "growth boundary {allowed}");
                assert_eq!(destination.interner.arena, arena);
                assert_eq!(destination.quads.len(), 0);
                assert_eq!(destination.reifiers.len(), 0);
                assert_eq!(destination.annotations.len(), 0);
                assert_eq!(destination.locations.len(), 0);
                assert_eq!(destination.declared_graphs.len(), 0);
                assert_eq!(destination.used_scopes, [BlankScope(2)]);
                assert_eq!(destination.next_merge_scope, 1);
                assert_eq!(destination.scope_journal.len(), 0);
                assert_eq!(
                    memory.admitted_bytes(),
                    destination.native_buffer_bytes().unwrap()
                );
                memory.admission_mut().remaining = 0;
                assert_eq!(
                    destination
                        .intern_blank_with_memory("prior", BlankScope(2), &mut memory)
                        .unwrap(),
                    prior
                );
                memory.admission_mut().remaining = usize::MAX;
                destination
                    .push_dataset_with_memory(&source, &mut memory)
                    .unwrap();
            }
            assert_eq!(destination.quads.len(), 1);
            assert_eq!(destination.reifiers.len(), 1);
            assert_eq!(destination.annotations.len(), 1);
            assert_eq!(destination.locations[0].1, location);
            assert_eq!(destination.next_merge_scope, 2);
            let one = destination
                .interner
                .lookup_owned_blank(
                    &BlankScope(1).qualify_label(&BlankScope(1).qualify_label("same")),
                )
                .unwrap();
            let five = destination
                .interner
                .lookup_owned_blank(
                    &BlankScope(1).qualify_label(&BlankScope(5).qualify_label("same")),
                )
                .unwrap();
            assert_ne!(
                one, five,
                "distinct original scopes remain distinct after merge"
            );
            assert_eq!(destination.quads[0].s, one);
            assert_eq!(destination.reifiers[0].0, five);
            let TermRef::Literal { lexical, .. } = destination.resolve(destination.quads[0].o)
            else {
                panic!("the imported object retains its composite literal")
            };
            assert_eq!(
                lexical,
                format!(
                    "[_:{}, _:{}]",
                    BlankScope(1).qualify_label(&BlankScope(1).qualify_label("same")),
                    BlankScope(1).qualify_label(&BlankScope(5).qualify_label("same")),
                ),
                "embedded labels use the same new identities as native blank terms",
            );
            assert!(destination.declared_graphs.iter().any(|&graph| {
                destination.resolve(graph) == TermRef::Iri("http://example.org/empty")
            }));
            assert_eq!(
                memory.admitted_bytes(),
                destination.native_buffer_bytes().unwrap()
            );
            destination.discard_with_memory(&mut memory).unwrap();
            assert_eq!(memory.admitted_bytes(), 0);
            if result.is_ok() {
                assert!(
                    failures > 20,
                    "exercise spilled walks, overlays and final graph declarations"
                );
                return;
            }
        }
        panic!("whole-dataset growth sweep must reach complete admission");
    }

    #[test]
    fn validated_append_keeps_original_account_exact_across_refusal_and_repetition() {
        let mut source = RdfDatasetBuilder::new();
        let quad = RdfQuad::new(
            RdfTerm::BlankNode(BlankScope(1).qualify_label("source").into_owned()),
            "http://example.org/new-property",
            RdfTerm::Literal(RdfLiteral::simple("source payload")),
        )
        .with_location(RdfLocation::logical("retained source position"));
        source.push_owned_quad(&quad).unwrap();
        let graph = source.intern_iri("http://example.org/empty").unwrap();
        source.declare_named_graph(graph).unwrap();
        let source = source.validate().unwrap();
        let mut failures = 0;
        for allowed in 0..128 {
            let mut grant = GrowthGrant {
                live: 0,
                remaining: usize::MAX,
            };
            let mut memory = Memory::new(&mut grant);
            let mut destination = RdfDatasetBuilder::new();
            let old = destination
                .intern_blank_with_memory("existing", BlankScope(2), &mut memory)
                .unwrap();
            let property = destination
                .intern_iri_with_memory("http://example.org/prior", &mut memory)
                .unwrap();
            destination
                .push_quad_with_memory(old, property, old, None, &mut memory)
                .unwrap();
            let arena = destination.interner.arena.clone();
            let terms = destination.term_count();
            memory.admission_mut().remaining = allowed;
            let result = destination.append_validated_with_memory(&source, &mut memory);
            if let Err(error) = result {
                assert_eq!(error, StorageError::AdmissionFailed);
                failures += 1;
                assert_eq!(destination.term_count(), terms);
                assert_eq!(destination.interner.arena, arena);
                assert_eq!(
                    destination.quads,
                    [QuadRow {
                        s: old,
                        p: property,
                        o: old,
                        g: None
                    }]
                );
                assert_eq!(destination.used_scopes, [BlankScope(2)]);
                assert_eq!(destination.locations.len(), 0);
                assert_eq!(destination.declared_graphs.len(), 0);
                assert_eq!(
                    memory.admitted_bytes(),
                    destination.native_buffer_bytes().unwrap()
                );
                memory.admission_mut().remaining = usize::MAX;
                destination
                    .append_validated_with_memory(&source, &mut memory)
                    .unwrap();
            }
            memory.admission_mut().remaining = usize::MAX;
            let terms = destination.term_count();
            for _ in 0..3 {
                assert_eq!(
                    destination
                        .append_validated_with_memory(&source, &mut memory)
                        .unwrap(),
                    0
                );
                assert_eq!(destination.term_count(), terms);
                assert_eq!(destination.quads.len(), 2);
                let graph = destination.declared_graphs[0];
                assert!(destination.declared_graphs.iter().all(|&id| id == graph));
                assert_eq!(
                    destination.resolve(graph),
                    TermRef::Iri("http://example.org/empty")
                );
                assert_eq!(
                    memory.admitted_bytes(),
                    destination.native_buffer_bytes().unwrap()
                );
            }
            destination.discard_with_memory(&mut memory).unwrap();
            assert_eq!(memory.admitted_bytes(), 0);
            if result.is_ok() {
                assert!(failures > 5);
                return;
            }
        }
        panic!("append growth sweep must complete");
    }

    #[test]
    fn escaped_owned_blank_hits_need_no_growth_and_match_materialized_hashes() {
        let long = "π_雪.x".repeat(257);
        let mut grant = GrowthGrant {
            live: 0,
            remaining: usize::MAX,
        };
        let mut memory = Memory::new(&mut grant);
        let mut builder = RdfDatasetBuilder::new();
        for label in [
            "",
            "a",
            "under_score",
            "purrdfesc1_raw",
            "π.雪",
            long.as_str(),
        ] {
            for scope in [BlankScope::DEFAULT, BlankScope(1), BlankScope(u32::MAX)] {
                memory.admission_mut().remaining = usize::MAX;
                let id = builder
                    .intern_blank_with_memory(label, scope, &mut memory)
                    .unwrap();
                let term = RdfTerm::BlankNode(scope.qualify_label(label).into_owned());
                let RdfTerm::BlankNode(token) = &term else {
                    unreachable!()
                };
                let key = crate::blank_label::OwnedBlankKey::new(token);
                assert_eq!(key.scope(), scope);
                assert_eq!(key.len(), label.len());
                assert!(key.bytes().eq(label.bytes()));
                assert_eq!(
                    crate::hash::hash_blank_bytes_for_interner(key.len(), key.bytes(), scope.0),
                    crate::hash::hash_blank_for_interner(label, scope.0),
                );
                memory.admission_mut().remaining = 0;
                for _ in 0..3 {
                    assert_eq!(
                        builder
                            .intern_owned_term_scoped_with_memory(
                                &term,
                                BlankScope::DEFAULT,
                                &mut memory
                            )
                            .unwrap(),
                        id
                    );
                }
                assert_eq!(
                    memory.admitted_bytes(),
                    builder.native_buffer_bytes().unwrap()
                );
            }
        }
        for token in [
            "purrdfesc_abc",
            "purrdfesc01_a",
            "purrdfesc1__000061",
            "purrdfesc1__00D800",
            "purrdfesc1__00002e",
        ] {
            memory.admission_mut().remaining = usize::MAX;
            let id = builder
                .intern_blank_with_memory(token, BlankScope::DEFAULT, &mut memory)
                .unwrap();
            let term = RdfTerm::BlankNode(token.to_owned());
            memory.admission_mut().remaining = 0;
            assert_eq!(
                builder
                    .intern_owned_term_scoped_with_memory(&term, BlankScope::DEFAULT, &mut memory)
                    .unwrap(),
                id
            );
        }
        builder.discard_with_memory(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn owned_overlay_refusal_restores_children_and_implicit_predicate() {
        let reifier = RdfReifier::new(
            RdfTerm::BlankNode(BlankScope(1).qualify_label("claim").into_owned()),
            crate::RdfTriple::new(
                RdfTerm::BlankNode(BlankScope(5).qualify_label("subject").into_owned()),
                "http://example.org/p",
                RdfTerm::Literal(RdfLiteral::simple("object")),
            ),
        )
        .in_graph(Some(RdfTerm::Iri("http://example.org/graph".to_owned())));
        let annotation = RdfAnnotation::new(
            reifier.reifier.clone(),
            "http://example.org/annotation",
            RdfTerm::Triple(Box::new(reifier.statement.clone())),
        )
        .in_graph(reifier.graph.clone());
        for annotation_only in [false, true] {
            let mut failures = 0;
            let mut completed = false;
            for allowed in 0..128 {
                let mut grant = GrowthGrant {
                    live: 0,
                    remaining: usize::MAX,
                };
                let mut memory = Memory::new(&mut grant);
                let mut builder = RdfDatasetBuilder::new();
                let old = builder
                    .intern_blank_with_memory("old", BlankScope(2), &mut memory)
                    .unwrap();
                let arena = builder.interner.arena.clone();
                let insert = |builder: &mut RdfDatasetBuilder,
                              memory: &mut Memory<'_, GrowthGrant>| {
                    if annotation_only {
                        builder.push_owned_annotation_scoped_with_memory(
                            &annotation,
                            BlankScope::DEFAULT,
                            memory,
                        )
                    } else {
                        builder.push_owned_reifier_scoped_with_memory(
                            &reifier,
                            BlankScope::DEFAULT,
                            memory,
                        )
                    }
                };
                memory.admission_mut().remaining = allowed;
                let result = insert(&mut builder, &mut memory);
                if let Err(error) = result {
                    assert_eq!(error, StorageError::AdmissionFailed);
                    failures += 1;
                    assert_eq!(builder.term_count(), 1, "growth boundary {allowed}");
                    assert_eq!(builder.interner.arena, arena);
                    assert_eq!(builder.interner.index.len(), 1);
                    assert_eq!(builder.reifiers.len(), 0);
                    assert_eq!(builder.reifier_index.len(), 0);
                    assert_eq!(builder.annotations.len(), 0);
                    assert_eq!(builder.annotation_index.len(), 0);
                    assert_eq!(builder.reifies_predicate, None);
                    assert_eq!(builder.used_scopes, [BlankScope(2)]);
                    assert_eq!(builder.scope_journal.len(), 0);
                    assert_eq!(builder.transaction_depth, 0);
                    assert_eq!(
                        memory.admitted_bytes(),
                        builder.native_buffer_bytes().unwrap()
                    );
                    memory.admission_mut().remaining = 0;
                    assert_eq!(
                        builder
                            .intern_blank_with_memory("old", BlankScope(2), &mut memory)
                            .unwrap(),
                        old
                    );
                    memory.admission_mut().remaining = usize::MAX;
                    insert(&mut builder, &mut memory).unwrap();
                }
                assert_eq!(builder.reifiers.len(), usize::from(!annotation_only));
                assert_eq!(builder.annotations.len(), usize::from(annotation_only));
                assert_eq!(builder.reifies_predicate.is_some(), !annotation_only);
                let count = builder.term_count();
                memory.admission_mut().remaining = 0;
                insert(&mut builder, &mut memory).unwrap();
                assert_eq!(builder.term_count(), count);
                assert_eq!(
                    memory.admitted_bytes(),
                    builder.native_buffer_bytes().unwrap()
                );
                builder.discard_with_memory(&mut memory).unwrap();
                assert_eq!(memory.admitted_bytes(), 0);
                if result.is_ok() {
                    completed = true;
                    break;
                }
            }
            assert!(
                completed && failures > 5,
                "exercise child and row growth for both overlays"
            );
        }
    }

    #[test]
    fn content_id_rollback_preserves_retained_charge_and_prior_lookup() {
        let iri = |n: u32| format!("blake3:{n:064x}");
        let mut object = RdfTerm::Iri(iri(100));
        for n in 101..133 {
            object = RdfTerm::Triple(Box::new(crate::RdfTriple::new(
                RdfTerm::Iri(iri(n)),
                "http://example.org/p",
                object,
            )));
        }
        let quad = RdfQuad::new(RdfTerm::Iri(iri(134)), "http://example.org/p", object)
            .with_location(RdfLocation::logical("last admitted payload"));
        for allowed in 0..256 {
            let mut builder = RdfDatasetBuilder::with_content_addressing(
                ContentIdScheme::new("blake3:").unwrap(),
                None,
            );
            let initial = builder.native_buffer_bytes().unwrap();
            let mut grant = GrowthGrant {
                live: initial,
                remaining: usize::MAX,
            };
            let mut memory = Memory::resume(&mut grant, initial);
            for n in 0..28 {
                builder
                    .intern_iri_with_memory(&iri(n), &mut memory)
                    .unwrap();
            }
            let previous = builder.interner.content_ids.clone();
            let terms = builder.term_count();
            memory.admission_mut().remaining = allowed;
            let result =
                builder.push_owned_quad_scoped_with_memory(&quad, BlankScope::DEFAULT, &mut memory);
            if result.is_err() {
                assert_eq!(result, Err(StorageError::AdmissionFailed));
                for _ in 0..3 {
                    assert_eq!(builder.term_count(), terms);
                    assert_eq!(builder.interner.content_ids, previous);
                    assert_eq!(
                        memory.admitted_bytes(),
                        builder.native_buffer_bytes().unwrap()
                    );
                    memory.admission_mut().remaining = 0;
                    assert_eq!(
                        builder.push_owned_quad_scoped_with_memory(
                            &quad,
                            BlankScope::DEFAULT,
                            &mut memory
                        ),
                        Err(StorageError::AdmissionFailed),
                    );
                }
                memory.admission_mut().remaining = usize::MAX;
                builder
                    .push_owned_quad_scoped_with_memory(&quad, BlankScope::DEFAULT, &mut memory)
                    .unwrap();
            }
            assert_eq!(builder.interner.content_ids.len(), previous.len() + 34);
            for (id, value) in previous {
                assert_eq!(builder.interner.content_id(id), Some(value));
            }
            assert_eq!(
                memory.admitted_bytes(),
                builder.native_buffer_bytes().unwrap()
            );
            builder.discard_with_memory(&mut memory).unwrap();
            assert_eq!(memory.admitted_bytes(), 0);
            if result.is_ok() {
                return;
            }
        }
        panic!("content-ID admission sweep must finish");
    }

    #[test]
    fn named_graph_growth_refusal_keeps_prior_declarations_and_duplicate_hits() {
        let mut builder = RdfDatasetBuilder::new();
        let graph = builder.intern_iri("http://example.org/graph").unwrap();
        let other = builder.intern_iri("http://example.org/other").unwrap();
        builder.declare_named_graph(graph).unwrap();
        while builder.declared_graphs.len() < builder.declared_graphs.capacity() {
            builder.declare_named_graph(graph).unwrap();
        }
        let original = builder.declared_graphs.clone();
        let terms = builder.term_count();
        let bytes = builder.native_buffer_bytes().unwrap();
        let mut grant = Grant {
            live: bytes,
            limit: bytes,
            peak: bytes,
        };
        let mut memory = Memory::resume(&mut grant, bytes);
        for _ in 0..2 {
            assert_eq!(
                builder.declare_named_graph_with_memory(other, &mut memory),
                Err(StorageError::AdmissionFailed)
            );
            assert_eq!(builder.declared_graphs, original);
            assert_eq!(builder.term_count(), terms);
            assert_eq!(memory.admitted_bytes(), bytes);
            builder
                .declare_named_graph_with_memory(graph, &mut memory)
                .unwrap();
            assert_eq!(builder.declared_graphs, original);
        }
        builder.declare_named_graph(other).unwrap();
        assert_eq!(builder.declared_graphs.last(), Some(&other));
    }

    #[test]
    fn native_graph_refuses_before_the_first_arena_allocation() {
        let mut grant = Grant {
            live: 0,
            limit: 0,
            peak: 0,
        };
        let mut memory = Memory::new(&mut grant);
        let mut builder = RdfDatasetBuilder::new();
        assert_eq!(
            builder.intern_iri_with_memory("http://example.org/subject", &mut memory),
            Err(StorageError::AdmissionFailed)
        );
        assert_eq!(builder.interner.arena.capacity(), 0);
        assert_eq!(builder.interner.terms.capacity(), 0);
        assert_eq!(builder.interner.index.capacity(), 0);
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn refused_literal_does_not_poison_later_healthy_freeze() {
        let mut builder = RdfDatasetBuilder::new();
        let mut grant = Grant {
            live: 0,
            limit: 0,
            peak: 0,
        };
        let mut memory = Memory::new(&mut grant);
        for _ in 0..2 {
            assert_eq!(
                builder.intern_literal_parts_with_memory(
                    "refused",
                    None,
                    None,
                    Some(RdfTextDirection::Ltr),
                    &mut memory,
                ),
                Err(StorageError::AdmissionFailed)
            );
            assert_eq!(builder.interner.invalid_literal, None);
            assert_eq!(builder.term_count(), 0);
            assert_eq!(memory.admitted_bytes(), 0);
        }
        builder
            .intern_literal(RdfLiteral::simple("healthy"))
            .unwrap();
        builder.freeze().unwrap();

        let mut invalid = RdfDatasetBuilder::new();
        invalid
            .intern_literal_parts("accepted", None, None, Some(RdfTextDirection::Ltr))
            .unwrap();
        assert!(
            invalid.freeze().is_err(),
            "an admitted invalid literal still refuses validation"
        );
    }

    #[test]
    fn blank_term_refusal_does_not_reserve_its_scope() {
        let mut builder = RdfDatasetBuilder::new();
        let prior = builder.intern_blank("prior", BlankScope(2)).unwrap();
        builder.used_scopes.reserve(4);
        let scopes = builder.used_scopes.clone();
        let text = builder.interner.arena.clone();
        let count = builder.term_count();
        let next_merge_scope = builder.next_merge_scope;
        let label = "b".repeat(builder.interner.arena.capacity() + 1);
        let bytes = builder.native_buffer_bytes().unwrap();
        let mut grant = Grant {
            live: bytes,
            limit: bytes,
            peak: bytes,
        };
        let mut memory = Memory::resume(&mut grant, bytes);
        for _ in 0..2 {
            assert_eq!(
                builder.intern_blank_with_memory(&label, BlankScope(1), &mut memory),
                Err(StorageError::AdmissionFailed)
            );
            assert_eq!(builder.used_scopes, scopes);
            assert_eq!(builder.next_merge_scope, next_merge_scope);
            assert_eq!(builder.term_count(), count);
            assert_eq!(builder.interner.arena, text);
            assert_eq!(memory.admitted_bytes(), bytes);
            assert_eq!(
                builder
                    .intern_blank_with_memory("prior", BlankScope(2), &mut memory)
                    .unwrap(),
                prior
            );
        }
        let id = builder.intern_blank(&label, BlankScope(1)).unwrap();
        assert_eq!(id.index(), count);
        assert_eq!(builder.used_scopes, [BlankScope(1), BlankScope(2)]);
        assert_eq!(builder.intern_blank(&label, BlankScope(1)).unwrap(), id);
    }

    #[test]
    fn content_id_side_table_refusal_does_not_publish_an_unindexed_term() {
        let scheme = ContentIdScheme::new("blake3:").unwrap();
        let mut builder = RdfDatasetBuilder::with_content_addressing(scheme, None);
        builder.reserve_for_replay(20, 1024, 0).unwrap();
        let prior = builder.intern_iri("http://example.org/prior").unwrap();
        let text = builder.interner.arena.clone();
        let count = builder.term_count();
        let iri = format!("blake3:{}", "ab".repeat(32));
        let bytes = builder.native_buffer_bytes().unwrap();
        let mut grant = Grant {
            live: bytes,
            limit: bytes,
            peak: bytes,
        };
        let mut memory = Memory::resume(&mut grant, bytes);
        for _ in 0..2 {
            assert_eq!(
                builder.intern_iri_with_memory(&iri, &mut memory),
                Err(StorageError::AdmissionFailed)
            );
            assert_eq!(builder.term_count(), count);
            assert_eq!(builder.interner.arena, text);
            assert_eq!(builder.interner.index.len(), count);
            assert_eq!(builder.interner.content_ids.len(), 0);
            assert_eq!(memory.admitted_bytes(), bytes);
            assert_eq!(
                builder
                    .intern_iri_with_memory("http://example.org/prior", &mut memory)
                    .unwrap(),
                prior
            );
        }
        let id = builder.intern_iri(&iri).unwrap();
        assert_eq!(id.index(), count);
        assert_eq!(
            builder.interner.content_id(id),
            Blake3ContentId::from_hex(&"ab".repeat(32))
        );
        assert_eq!(builder.intern_iri(&iri).unwrap(), id);
        assert_eq!(builder.term_count(), count + 1);
    }

    #[test]
    fn reifier_row_refusal_does_not_publish_its_implicit_predicate() {
        let mut builder = RdfDatasetBuilder::new();
        // The predicate fits the original interner without further allocation;
        // only the binding's first physical row requires a new grant.
        builder.reserve_for_replay(20, 1024, 0).unwrap();
        let s = builder.intern_iri("http://example.org/s").unwrap();
        let p = builder.intern_iri("http://example.org/p").unwrap();
        let r = builder.intern_iri("http://example.org/r").unwrap();
        let triple = builder.intern_triple(s, p, s).unwrap();
        let count = builder.term_count();
        let text = builder.interner.arena.clone();
        let bytes = builder.native_buffer_bytes().unwrap();
        let mut grant = Grant {
            live: bytes,
            limit: bytes,
            peak: bytes,
        };
        let mut memory = Memory::resume(&mut grant, bytes);
        for _ in 0..2 {
            assert_eq!(
                builder.push_reifier_in_graph_with_memory(r, triple, None, &mut memory),
                Err(StorageError::AdmissionFailed)
            );
            assert_eq!(builder.term_count(), count);
            assert_eq!(builder.interner.arena, text);
            assert!(builder.reifies_predicate.is_none());
            assert_eq!(builder.reifiers.len(), 0);
            assert_eq!(builder.reifier_index.len(), 0);
            assert_eq!(memory.admitted_bytes(), bytes);
        }
        // The same original owner remains usable after the refusal.
        builder.push_reifier_in_graph(r, triple, None).unwrap();
        assert_eq!(builder.term_count(), count + 1);
        assert_eq!(builder.reifiers, [(r, triple, None)]);
        let bytes = builder.native_buffer_bytes().unwrap();
        let mut grant = Grant {
            live: bytes,
            limit: bytes,
            peak: bytes,
        };
        let mut memory = Memory::resume(&mut grant, bytes);
        builder
            .push_reifier_in_graph_with_memory(r, triple, None, &mut memory)
            .unwrap();
        assert_eq!(builder.reifiers.len(), 1);
        assert_eq!(builder.term_count(), count + 1);
        assert_eq!(memory.admitted_bytes(), bytes);
    }

    #[test]
    fn native_statistics_refusal_refunds_partial_buffers_and_uses_original_grant() {
        let mut grant = GrowthGrant {
            live: 0,
            remaining: usize::MAX,
        };
        let mut memory = Memory::new(&mut grant);
        let mut builder = RdfDatasetBuilder::new();
        let id = builder
            .intern_iri_with_memory("urn:statistics:term", &mut memory)
            .unwrap();
        let dictionary = builder
            .interner
            .freeze_with_memory(None, &mut memory)
            .unwrap();
        let baseline = memory.admitted_bytes();
        let row = |_| QuadIds {
            s: id,
            p: id,
            o: id,
            g: None,
        };
        let mut completed = false;
        let mut failures = 0;
        for remaining in 0..64 {
            memory.admission_mut().remaining = remaining;
            let result = crate::ir::statistics::LocalStatistics::build(
                &dictionary,
                (1, row),
                &(0, |_| (id, id, None)),
                (0, row),
                &mut memory,
            );
            match result {
                Err(error) => {
                    assert_eq!(error, StorageError::AdmissionFailed);
                    failures += 1;
                }
                Ok(statistics) => {
                    let retained = memory.admitted_bytes() - baseline;
                    assert_eq!(retained, statistics.retained_bytes().unwrap());
                    drop(statistics);
                    memory.release_bytes(retained).unwrap();
                    completed = true;
                }
            }
            assert_eq!(memory.admitted_bytes(), baseline);
            if completed {
                break;
            }
        }
        assert!(completed && failures > 5);
        drop(dictionary);
        memory.release_bytes(baseline).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn native_freeze_preserves_rdf12_rows_and_original_grant_through_indexes() {
        let mut resident = RdfDatasetBuilder::new();
        let mut builder = RdfDatasetBuilder::new();
        let mut grant = Grant {
            live: 0,
            limit: usize::MAX,
            peak: 0,
        };
        let mut memory = Memory::new(&mut grant);
        let mut ids = [TermId::from_index(0).expect("stored term index is representable"); 5];
        let mut resident_ids = ids;
        for (index, iri) in [
            "http://example.org/s",
            "http://example.org/p",
            "http://example.org/o",
            "http://example.org/r",
            "http://example.org/g",
        ]
        .into_iter()
        .enumerate()
        {
            ids[index] = builder
                .intern_iri_with_memory(iri, &mut memory)
                .expect("native IRI");
            resident_ids[index] = resident.intern_iri(iri).unwrap();
        }
        let [s, p, o, r, g] = ids;
        let [rs, rp, ro, rr, rg] = resident_ids;
        let triple = builder
            .intern_triple_with_memory(s, p, o, &mut memory)
            .expect("native triple");
        let resident_triple = resident.intern_triple(rs, rp, ro).unwrap();
        builder
            .push_quad_with_memory(s, p, triple, Some(g), &mut memory)
            .expect("native quad");
        builder
            .push_reifier_in_graph_with_memory(r, triple, Some(g), &mut memory)
            .expect("native reifier");
        builder
            .push_annotation_in_graph_with_memory(r, p, o, Some(g), &mut memory)
            .expect("native annotation");
        resident
            .push_quad(rs, rp, resident_triple, Some(rg))
            .unwrap();
        resident
            .push_reifier_in_graph(rr, resident_triple, Some(rg))
            .unwrap();
        resident
            .push_annotation_in_graph(rr, rp, ro, Some(rg))
            .unwrap();
        let resident = resident.freeze().expect("resident RDF 1.2 graph");
        let mut graph = builder
            .freeze_with_memory(&mut memory)
            .expect("native RDF 1.2 graph");
        graph
            .warm_query_indexes_with_memory(&mut memory)
            .expect("originally admitted query indexes");
        assert_eq!(graph.rdf_row_count(), resident.rdf_row_count());
        assert_eq!(graph.term_count(), resident.term_count());
        assert_eq!(
            graph.named_graphs().collect::<Vec<_>>(),
            resident.named_graphs().collect::<Vec<_>>()
        );
        for index in 0..graph.term_count() {
            let id = TermId::from_index(index as u32).expect("stored term index is representable");
            assert_eq!(graph.term_value(id), resident.term_value(id));
        }
        let live = memory.admitted_bytes();
        assert!(live > 0);
        graph
            .warm_query_indexes_with_memory(&mut memory)
            .expect("warmed graph borrows the original indexes");
        assert_eq!(memory.admitted_bytes(), live);
        drop(graph);
        memory
            .release_bytes(live)
            .expect("graph dies before its original grant");
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn native_structural_failure_releases_builder_and_retains_only_the_diagnostic() {
        let mut grant = Grant {
            live: 0,
            limit: usize::MAX,
            peak: 0,
        };
        let mut memory = Memory::new(&mut grant);
        let mut builder = RdfDatasetBuilder::new();
        let s = builder
            .intern_literal_parts_with_memory("literal", None, None, None, &mut memory)
            .expect("native literal");
        let p = builder
            .intern_iri_with_memory("http://example.org/p", &mut memory)
            .expect("native predicate");
        builder
            .push_quad_with_memory(s, p, s, None, &mut memory)
            .expect("native invalid row");
        let NativeBuildError::Diagnostic(diagnostic) = builder
            .freeze_with_memory(&mut memory)
            .expect_err("RDF 1.2 rejects a literal subject")
        else {
            panic!("expected original structural diagnostic");
        };
        assert_eq!(diagnostic.code, "rdf-ir-literal-subject");
        assert_eq!(
            memory.admitted_bytes(),
            diagnostic.code.capacity() + diagnostic.message.capacity()
        );
        let bytes = memory.admitted_bytes();
        drop(diagnostic);
        memory
            .release_bytes(bytes)
            .expect("diagnostic dies before its grant");
        assert_eq!(memory.admitted_bytes(), 0);
    }
}
