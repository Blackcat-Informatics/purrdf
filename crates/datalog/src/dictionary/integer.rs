// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-owned integer identities with explicit program symbols and no text projection.

use super::{DictionaryError, DictionaryTerm, TermDictionary, TermIdentity, try_map_row};
use core::{cmp::Ordering, fmt};
use purrdf_core::{FastMap, FastSet};
use purrdf_lex::allocation::StorageError;

/// A refusal at an explicitly requested native dictionary boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegerDictionaryError {
    /// An authored symbol was not supplied by the caller.
    UnknownSymbol {
        /// The original symbol.
        symbol: String,
    },
    /// A symbol was declared more than once.
    DuplicateSymbol {
        /// The original symbol.
        symbol: String,
    },
    /// Integer identities have no fabricated lexical representation.
    LexicalUnavailable {
        /// The original identity, losslessly widened.
        id: u64,
    },
}

impl purrdf_hash::error::StructuredError for IntegerDictionaryError {
    purrdf_hash::error_visit! {
        UnknownSymbol { symbol } => "datalog-integer-unknown-symbol" { "symbol" => Text(symbol) },
        DuplicateSymbol { symbol } => "datalog-integer-duplicate-symbol" { "symbol" => Text(symbol) },
        LexicalUnavailable { id } => "datalog-integer-lexical-unavailable" { "id" => Unsigned(u128::from(*id)) },
    }
}

impl fmt::Display for IntegerDictionaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSymbol { symbol } => {
                write!(formatter, "undeclared native symbol: {symbol}")
            }
            Self::DuplicateSymbol { symbol } => {
                write!(formatter, "duplicate native symbol: {symbol}")
            }
            Self::LexicalUnavailable { id } => {
                write!(formatter, "native identity {id} has no lexical projection")
            }
        }
    }
}

impl std::error::Error for IntegerDictionaryError {}

/// A dictionary whose `u32` or `u64` identities are the caller's integers.
///
/// Rows, guards, comparisons and model-only fixed points carry these integers
/// unchanged. Program constants resolve through a caller-supplied symbol table;
/// cell values are never converted to strings or interned. Numeric ID order is
/// the canonical publication order. The dictionary tracks distinct admitted IDs
/// for the existing term-count governor, without owning any lexical bytes.
///
/// Lexical guards and recorded lexical derivations must use a dictionary that
/// supplies spellings instead: explicit projection here returns a typed error.
#[derive(Debug, Clone, Default)]
pub struct IntegerDictionary<'a, I: TermIdentity + Into<u64> = u32> {
    symbols: FastMap<&'a str, I>,
    identities: FastSet<I>,
}

impl<'a, I: TermIdentity + Into<u64>> IntegerDictionary<'a, I> {
    /// Bind authored program symbols to original caller identities.
    /// Different names may explicitly denote the same identity.
    ///
    /// # Errors
    /// Refuses duplicate names or a native allocation failure.
    pub fn with_symbols(
        symbols: &'a [(&'a str, I)],
    ) -> Result<Self, DictionaryError<IntegerDictionaryError>> {
        let mut dictionary = Self {
            symbols: FastMap::default(),
            identities: FastSet::default(),
        };
        dictionary
            .symbols
            .try_reserve(symbols.len())
            .map_err(|_| StorageError::AllocationFailed)?;
        dictionary
            .identities
            .try_reserve(symbols.len())
            .map_err(|_| StorageError::AllocationFailed)?;
        for &(symbol, id) in symbols {
            if dictionary.symbols.insert(symbol, id).is_some() {
                return Err(DictionaryError::Source(
                    IntegerDictionaryError::DuplicateSymbol {
                        symbol: symbol.to_owned(),
                    },
                ));
            }
            dictionary.identities.insert(id);
        }
        Ok(dictionary)
    }

    /// Resolve an explicit program symbol or retain an original cell identity.
    fn identity(&self, term: DictionaryTerm<'_, I>) -> Result<I, IntegerDictionaryError> {
        match term {
            DictionaryTerm::Id(id) => Ok(id),
            DictionaryTerm::Surface(symbol) => self.symbols.get(symbol).copied().ok_or_else(|| {
                IntegerDictionaryError::UnknownSymbol {
                    symbol: symbol.to_owned(),
                }
            }),
        }
    }
}

impl<I: TermIdentity + Into<u64>> TermDictionary for IntegerDictionary<'_, I> {
    type Id = I;
    type Error = IntegerDictionaryError;
    type Lexical<'a>
        = &'a str
    where
        Self: 'a;

    fn lookup_term(&self, surface: &str) -> Result<Option<I>, Self::Error> {
        Ok(self.symbols.get(surface).copied())
    }

    fn resolve_term(&self, id: I) -> Result<Self::Lexical<'_>, Self::Error> {
        Err(IntegerDictionaryError::LexicalUnavailable { id: id.into() })
    }

    fn compare_terms(
        &self,
        left: DictionaryTerm<'_, I>,
        right: DictionaryTerm<'_, I>,
    ) -> Result<Ordering, Self::Error> {
        Ok(self.identity(left)?.cmp(&self.identity(right)?))
    }

    fn with_interned<T, const N: usize>(
        &mut self,
        terms: [DictionaryTerm<'_, I>; N],
        validate: impl FnOnce(&Self, [I; N]) -> Result<T, DictionaryError<Self::Error>>,
    ) -> Result<T, DictionaryError<Self::Error>> {
        let ids =
            try_map_row(terms, |term| self.identity(term)).map_err(DictionaryError::Source)?;
        self.identities
            .try_reserve(N)
            .map_err(|_| StorageError::AllocationFailed)?;
        let result = validate(self, ids)?;
        self.identities.extend(ids);
        Ok(result)
    }

    fn check_read(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn term_count(&self) -> u64 {
        u64::try_from(self.identities.len()).expect("native address width fits u64")
    }

    fn resident_bytes(&self) -> usize {
        0
    }
}

impl<I: TermIdentity + Into<u64>> super::ReclaimDictionary for IntegerDictionary<'_, I> {
    fn owned_terms(&self) -> impl Iterator<Item = I> + '_ {
        self.identities.iter().copied()
    }

    fn owned_term_scan_work(&self) -> Result<usize, StorageError> {
        purrdf_core::hash::hash_table_allocation_bound::<I>(self.identities.capacity())
            .ok_or(StorageError::SizeOverflow)?
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)
    }

    fn release_term(&mut self, id: I) -> Result<(), DictionaryError<Self::Error>> {
        if !self.symbols.values().any(|&symbol| symbol == id) {
            self.identities.remove(&id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_symbols_alias_identities_and_refuse_ambiguous_names() {
        let symbols = [("first", 0_u32), ("alias", 0), ("last", u32::MAX)];
        let dictionary = IntegerDictionary::with_symbols(&symbols).unwrap();
        assert_eq!(dictionary.term_count(), 2);
        assert_eq!(dictionary.resident_bytes(), 0);
        assert_eq!(dictionary.lookup_term("alias"), Ok(Some(0)));
        assert_eq!(dictionary.lookup_term("missing"), Ok(None));
        assert_eq!(
            dictionary.compare_terms(
                DictionaryTerm::Surface("first"),
                DictionaryTerm::Id(u32::MAX)
            ),
            Ok(Ordering::Less)
        );
        assert!(
            matches!(dictionary.compare_terms(DictionaryTerm::Surface("missing"), DictionaryTerm::Id(0)), Err(IntegerDictionaryError::UnknownSymbol { symbol }) if symbol == "missing")
        );
        assert_eq!(
            dictionary.resolve_term(u32::MAX),
            Err(IntegerDictionaryError::LexicalUnavailable {
                id: u64::from(u32::MAX)
            })
        );
        assert!(
            matches!(IntegerDictionary::with_symbols(&[("same", 1_u32), ("same", 2)]), Err(DictionaryError::Source(IntegerDictionaryError::DuplicateSymbol { symbol })) if symbol == "same")
        );
    }

    #[test]
    fn validation_and_symbol_refusals_do_not_publish_identity_metadata() {
        let mut dictionary = IntegerDictionary::<u64>::default();
        let original = IntegerDictionaryError::LexicalUnavailable { id: u64::MAX };
        let refused: Result<(), _> = dictionary
            .with_interned([DictionaryTerm::Id(u64::MAX)], |_, _| {
                Err(DictionaryError::Source(original.clone()))
            });
        assert_eq!(refused, Err(DictionaryError::Source(original)));
        assert_eq!(dictionary.term_count(), 0);
        let refused: Result<(), _> = dictionary.with_interned(
            [DictionaryTerm::Id(0), DictionaryTerm::Surface("missing")],
            |_, _| panic!("an undeclared symbol cannot reach publication"),
        );
        assert!(
            matches!(refused, Err(DictionaryError::Source(IntegerDictionaryError::UnknownSymbol { symbol })) if symbol == "missing")
        );
        assert_eq!(dictionary.term_count(), 0);
        let admitted = dictionary
            .with_interned(
                [
                    DictionaryTerm::Id(0),
                    DictionaryTerm::Id(u64::MAX),
                    DictionaryTerm::Id(0),
                ],
                |_, ids| Ok(ids),
            )
            .unwrap();
        assert_eq!(admitted, [0, u64::MAX, 0]);
        assert_eq!(dictionary.term_count(), 2);
    }
}
