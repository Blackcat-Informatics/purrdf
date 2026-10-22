// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Term identity and borrowed dictionary access at the original rule-store seam.

mod integer;
pub use integer::{IntegerDictionary, IntegerDictionaryError};

use core::cmp::Ordering;
use core::fmt;
use core::hash::Hash;
use purrdf_lex::allocation::StorageError;

/// A term identity carried verbatim by every arrangement and join frame.
///
/// Equality is RDF identity in the supplied dictionary. `Ord` is only an internal
/// arrangement order; it does not attest lexical, numeric or temporal value order.
pub trait TermIdentity: Copy + Eq + Ord + Hash + fmt::Debug + Send + Sync + 'static {}

impl<T: Copy + Eq + Ord + Hash + fmt::Debug + Send + Sync + 'static> TermIdentity for T {}

/// One original identity or a genuinely fresh lexical term at row publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionaryTerm<'a, I: TermIdentity> {
    /// Identity already owned by the supplied dictionary; never remapped.
    Id(I),
    /// Lexical term to look up or mint in that same dictionary.
    Surface(&'a str),
}

impl<'a, I: TermIdentity> DictionaryTerm<'a, I> {
    /// Pin an original dictionary spelling at an explicit lexical boundary.
    ///
    /// # Errors
    ///
    /// Preserves the original dictionary read or resolution failure.
    pub fn lexical<'d, D: TermDictionary<Id = I>>(
        self,
        dictionary: &'d D,
    ) -> Result<LexicalTerm<'a, D::Lexical<'d>>, D::Error> {
        dictionary.check_read()?;
        let value = match self {
            Self::Id(id) => LexicalTerm::Pinned(dictionary.resolve_term(id)?),
            Self::Surface(surface) => LexicalTerm::Computed(surface),
        };
        dictionary.check_read()?;
        Ok(value)
    }

    /// Reuse an existing identity for a spelling, preserving the original borrow
    /// when the spelling is fresh. Original identities require no lexical lookup.
    ///
    /// # Errors
    ///
    /// Preserves the dictionary's original read or lookup failure.
    pub fn canonicalize<D: TermDictionary<Id = I>>(self, dictionary: &D) -> Result<Self, D::Error> {
        dictionary.check_read()?;
        let value = match self {
            Self::Id(id) => Self::Id(id),
            Self::Surface(surface) => match dictionary.lookup_term(surface)? {
                Some(id) => Self::Id(id),
                None => Self::Surface(surface),
            },
        };
        dictionary.check_read()?;
        Ok(value)
    }
}

/// An original caller identity or an owned lexical value produced by a guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictionaryValue<I: TermIdentity> {
    /// Identity already owned by the supplied dictionary; never remapped.
    Id(I),
    /// Lexical value to look up or mint in that same dictionary.
    Surface(String),
}

purrdf_lex::variant_from!(impl [I] DictionaryValue<I> where [I: TermIdentity] {
    Surface(String)
});
purrdf_lex::variant_from!(impl [I] DictionaryValue<I> where [I: TermIdentity] {
    Surface(&str) as ToString::to_string
});

impl<I: TermIdentity> DictionaryValue<I> {
    /// Borrow the original identity or owned spelling.
    pub fn as_term(&self) -> DictionaryTerm<'_, I> {
        match self {
            Self::Id(id) => DictionaryTerm::Id(*id),
            Self::Surface(surface) => DictionaryTerm::Surface(surface),
        }
    }
}

/// A pinned dictionary spelling or a borrowed, newly computed spelling.
/// The original dictionary pin stays alive while a lexical guard reads it.
#[derive(Debug)]
pub enum LexicalTerm<'a, L: AsRef<str>> {
    /// Original dictionary pin or borrow.
    Pinned(L),
    /// A computed spelling borrowed from the current binding frame.
    Computed(&'a str),
}

impl<L: AsRef<str>> AsRef<str> for LexicalTerm<'_, L> {
    fn as_ref(&self) -> &str {
        match self {
            Self::Pinned(pin) => pin.as_ref(),
            Self::Computed(surface) => surface,
        }
    }
}

/// Transform the fixed row in position order, stopping at its first refusal.
/// Completed owners are dropped on failure; the array introduces no allocation.
pub(crate) fn try_map_row<T, U, E, const N: usize>(
    row: [T; N],
    mut convert: impl FnMut(T) -> Result<U, E>,
) -> Result<[U; N], E> {
    let mut converted = core::array::from_fn(|_| None);
    for (slot, value) in converted.iter_mut().zip(row) {
        *slot = Some(convert(value)?);
    }
    Ok(converted.map(|value| value.expect("every successfully converted row position is present")))
}

/// Lexicographic comparison preserving the first original comparator refusal.
pub(crate) fn compare_sequences<T, E>(
    left: &[T],
    right: &[T],
    mut compare: impl FnMut(&T, &T) -> Result<Ordering, E>,
) -> Result<Ordering, E> {
    for (left, right) in left.iter().zip(right) {
        let order = compare(left, right)?;
        if !order.is_eq() {
            return Ok(order);
        }
    }
    Ok(left.len().cmp(&right.len()))
}

/// The RDF lexical order, retaining original pins throughout each comparison.
///
/// Dictionary implementations with RDF spellings can use this for
/// [`TermDictionary::compare_terms`]. Native integer dictionaries supply their
/// own comparison directly.
///
/// # Errors
/// Preserves the dictionary's original read or lexical-resolution refusal.
pub fn compare_lexical_terms<D: TermDictionary>(
    dictionary: &D,
    left: DictionaryTerm<'_, D::Id>,
    right: DictionaryTerm<'_, D::Id>,
) -> Result<Ordering, D::Error> {
    let left = left.lexical(dictionary)?;
    let right = right.lexical(dictionary)?;
    dictionary.check_read()?;
    Ok(left.as_ref().cmp(right.as_ref()))
}

/// Original storage refusal or original dictionary-source failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictionaryError<E> {
    /// Refusal before an actual native buffer or logical handle is minted.
    Storage(StorageError),
    /// The dictionary's original typed failure, retained without rendering it.
    Source(E),
}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError
    for DictionaryError<E>
{
    purrdf_hash::error_visit! {
        Storage(cause) => "datalog-dictionary-storage" { "cause" => Cause(cause) },
        Source(cause) => "datalog-dictionary-source" { "cause" => Cause(cause) },
    }
}

purrdf_lex::variant_from!(impl<E> DictionaryError<E> { Storage(StorageError) });

impl<E: fmt::Display> fmt::Display for DictionaryError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => error.fmt(formatter),
            Self::Source(error) => error.fmt(formatter),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for DictionaryError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::Source(error) => Some(error),
        }
    }
}

/// Explicit retirement authority for a dictionary whose unused terms can expire.
/// The model calls this only after releasing its last row, proof and result use.
/// Providers retain their own declared symbols and external authority separately.
pub trait ReclaimDictionary: TermDictionary {
    /// Original identities whose retained metadata this dictionary owns.
    /// Used once when the maintenance owner adopts the initial dictionary.
    fn owned_terms(&self) -> impl Iterator<Item = Self::Id> + '_;

    /// Release one original identity after its last semantic use.
    ///
    /// # Errors
    /// Preserves the original source or storage failure; no replacement ID is minted.
    fn release_term(&mut self, id: Self::Id) -> Result<(), DictionaryError<Self::Error>>;
}

/// The caller's identity space and dictionary, resident or provider-backed.
///
/// IDs enter the store unchanged. Constant lookup and lexical proof/publication
/// are dictionary boundaries; joins compare the original IDs and never translate
/// them. A lexical guard must pin the bytes it exposes until the guard dies.
/// Deletion or replacement is observed by `check_read`, including an empty run.
pub trait TermDictionary: Send + Sync {
    /// Native or host-supplied identity, including tagged and triple-term IDs.
    type Id: TermIdentity;
    /// Original source failure; callers must preserve it as a hard run failure.
    type Error: std::error::Error + Clone + Eq + Send + Sync + 'static;
    /// Original lexical owner or borrow, never a detached borrowed temporary.
    type Lexical<'a>: AsRef<str>
    where
        Self: 'a;

    /// Look up an authored constant without minting or remapping an ID.
    fn lookup_term(&self, surface: &str) -> Result<Option<Self::Id>, Self::Error>;
    /// Pin the canonical RDF surface at a semantic or publication boundary.
    fn resolve_term(&self, id: Self::Id) -> Result<Self::Lexical<'_>, Self::Error>;
    /// Compare terms in this dictionary's canonical publication order.
    ///
    /// This must be a total order consistent with term identity, stable for the
    /// lifetime of a read. RDF dictionaries compare canonical lexical spellings;
    /// native dictionaries may compare original integers without resolving text.
    /// Both operands may be fresh values produced by a guard. Unsupported values
    /// must return the original typed error, never a substitute order.
    fn compare_terms(
        &self,
        left: DictionaryTerm<'_, Self::Id>,
        right: DictionaryTerm<'_, Self::Id>,
    ) -> Result<Ordering, Self::Error>;
    /// Compare complete rows in position order through the same term contract.
    fn compare_rows(
        &self,
        left: &[DictionaryTerm<'_, Self::Id>],
        right: &[DictionaryTerm<'_, Self::Id>],
    ) -> Result<Ordering, Self::Error> {
        self.check_read()?;
        let order =
            compare_sequences(left, right, |left, right| self.compare_terms(*left, *right))?;
        self.check_read()?;
        Ok(order)
    }
    /// Admit the row terms in caller order, then validate their dependent publication.
    ///
    /// Original IDs are carried unchanged. On either interning or validation refusal,
    /// remove only identities minted by this operation and return the original error.
    /// The validator borrows the dictionary and cannot publish a partial store row.
    fn with_interned<T, const N: usize>(
        &mut self,
        terms: [DictionaryTerm<'_, Self::Id>; N],
        validate: impl FnOnce(&Self, [Self::Id; N]) -> Result<T, DictionaryError<Self::Error>>,
    ) -> Result<T, DictionaryError<Self::Error>>
    where
        Self: Sized;
    /// Validate the whole original read operation before returning an answer.
    fn check_read(&self) -> Result<(), Self::Error>;
    /// Distinct term count, independently of whether its dictionary is resident.
    fn term_count(&self) -> u64;
    /// Resident lexical bytes held by this dictionary, for the existing governor.
    fn resident_bytes(&self) -> usize;
}
