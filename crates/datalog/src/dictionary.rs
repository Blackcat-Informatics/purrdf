// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Term identity and borrowed dictionary access at the original rule-store seam.

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
#[derive(Debug, Clone, Copy)]
pub enum DictionaryTerm<'a, I: TermIdentity> {
    /// Identity already owned by the supplied dictionary; never remapped.
    Id(I),
    /// Lexical term to look up or mint in that same dictionary.
    Surface(&'a str),
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
