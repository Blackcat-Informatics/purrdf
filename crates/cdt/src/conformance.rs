// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Which lexical space a value's canonical form lies in: strict SEP-0009, or one of
//! PurRDF's two documented supersets.
//!
//! # Why the crate reports its own conformance position
//!
//! `purrdf-cdt` widens SEP-0009's `Element` production twice — RDF 1.2 triple terms
//! and directional language-tagged literals — and the crate docs argue at length why
//! refusing an RDF 1.2 term is not an admissible outcome for this toolkit. Both
//! widenings are real divergences from the spec, and **no upstream conformance vector
//! exercises either one**: SEP-0009's corpus contains no `<<(` and no `--ltr` /
//! `--rtl` anywhere, so running it can never tell anyone that PurRDF accepts more
//! than the spec does. An ungraded divergence is an invisible one.
//!
//! This module is the grading. [`lexical_space`] answers, for any value, whether its
//! canonical form is one SEP-0009 itself could have written; the crate's own
//! `tests/superset.rs` turns that into executable assertions, one per widened form.
//!
//! # This is not optionality
//!
//! Nothing here switches anything off. There is one scanner, one lexical space, and
//! one canonical form; every value PurRDF parses or mints is handled by the same code
//! whatever this reports. What the function adds is the crate's ability to *say* where
//! a given value stands relative to the spec — the difference between a library that
//! diverges and a library that diverges and knows it. A consumer that must publish
//! only strictly-conformant literals can ask before it writes; one that does not,
//! never calls it.
//!
//! # Iterative, like everything else here
//!
//! The walk carries an explicit heap worklist and never recurses, so it is safe on a
//! value of any admissible depth.

use alloc::vec::Vec;

use crate::datatype::RDF_DIR_LANG_STRING;
use crate::term::{CdtEntry, CdtKey, CdtLiteral, CdtTerm};
use crate::value::{CdtContents, CdtValue};

/// Where a lexical form stands relative to SEP-0009's own `Element` production.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LexicalSpace {
    /// Every part of the form is inside strict SEP-0009: the spec's grammar, as
    /// published, admits it, and any conformant implementation can read it.
    Sep0009,
    /// At least one part needs one of PurRDF's two documented supersets — an RDF 1.2
    /// triple term `<<( s p o )>>`, or a directional language-tagged literal
    /// `"lex"@lang--ltr` / `--rtl`. Under strict SEP-0009 that form is ill-typed.
    PurrdfSuperset,
}

impl LexicalSpace {
    /// `true` for [`LexicalSpace::PurrdfSuperset`].
    #[must_use]
    pub const fn is_extension(self) -> bool {
        matches!(self, Self::PurrdfSuperset)
    }
}

/// The lexical space a literal needs.
///
/// Only one literal form is outside SEP-0009: RDF 1.2's directional language-tagged
/// string, which the spec's `LANGTAG` terminal has no `--ltr` / `--rtl` suffix for.
/// Both the parsed direction and the `rdf:dirLangString` datatype are consulted, so a
/// hand-built literal that carries the datatype without the direction is classified
/// the same way as one that carries both.
#[must_use]
pub fn literal_lexical_space(literal: &CdtLiteral) -> LexicalSpace {
    if literal.direction.is_some() || literal.datatype == RDF_DIR_LANG_STRING {
        LexicalSpace::PurrdfSuperset
    } else {
        LexicalSpace::Sep0009
    }
}

/// The lexical space a map key needs. A key is always a leaf, so this never walks.
#[must_use]
pub fn key_lexical_space(key: &CdtKey) -> LexicalSpace {
    match key {
        CdtKey::Iri(_) => LexicalSpace::Sep0009,
        CdtKey::Literal(literal) => literal_lexical_space(literal),
    }
}

/// The lexical space an element needs, walking nested composites iteratively.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtLiteral, CdtTerm, LexicalSpace, TextDirection, term_lexical_space};
///
/// // A triple term is PurRDF's first superset.
/// let triple = CdtTerm::triple(
///     CdtTerm::Iri("http://example.org/s".into()),
///     CdtTerm::Iri("http://example.org/p".into()),
///     CdtTerm::Null,
/// )?;
/// assert_eq!(term_lexical_space(&triple), LexicalSpace::PurrdfSuperset);
///
/// // A plain language-tagged string is not.
/// let plain = CdtTerm::Literal(CdtLiteral::lang("hello", "en"));
/// assert_eq!(term_lexical_space(&plain), LexicalSpace::Sep0009);
///
/// // …but a directional one is the second superset.
/// let directional =
///     CdtTerm::Literal(CdtLiteral::dir_lang("hello", "en", TextDirection::Ltr));
/// assert_eq!(term_lexical_space(&directional), LexicalSpace::PurrdfSuperset);
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
#[must_use]
pub fn term_lexical_space(term: &CdtTerm) -> LexicalSpace {
    let mut work: Vec<&CdtTerm> = alloc::vec![term];
    let mut values: Vec<&CdtValue> = Vec::new();
    loop {
        while let Some(current) = work.pop() {
            match current {
                // Superset 1: production `[3]` / `[8]` gain no `TripleTerm`
                // alternative in the published grammar.
                CdtTerm::TripleTerm(_) => return LexicalSpace::PurrdfSuperset,
                CdtTerm::Literal(literal) => {
                    if literal_lexical_space(literal).is_extension() {
                        return LexicalSpace::PurrdfSuperset;
                    }
                }
                CdtTerm::Composite(inner) => values.push(inner.as_ref()),
                CdtTerm::Iri(_) | CdtTerm::Blank(_) | CdtTerm::Null => {}
            }
        }
        let Some(value) = values.pop() else {
            return LexicalSpace::Sep0009;
        };
        match value.contents() {
            CdtContents::List(items) => work.extend(items.iter()),
            CdtContents::Map(entries) => {
                for CdtEntry { key, value: item } in entries {
                    if key_lexical_space(key).is_extension() {
                        return LexicalSpace::PurrdfSuperset;
                    }
                    work.push(item);
                }
            }
        }
    }
}

/// The lexical space a whole composite value's canonical form lies in.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{LexicalSpace, lexical_space, parse_list};
///
/// assert_eq!(lexical_space(&parse_list("[1, 'a'@en, [true]]")?), LexicalSpace::Sep0009);
/// // One directional literal, however deeply nested, is enough.
/// assert_eq!(
///     lexical_space(&parse_list("[1, [ 'a'@en--rtl ]]")?),
///     LexicalSpace::PurrdfSuperset
/// );
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
#[must_use]
pub fn lexical_space(value: &CdtValue) -> LexicalSpace {
    match value.contents() {
        CdtContents::List(items) => {
            for item in items {
                if term_lexical_space(item).is_extension() {
                    return LexicalSpace::PurrdfSuperset;
                }
            }
        }
        CdtContents::Map(entries) => {
            for CdtEntry { key, value: item } in entries {
                if key_lexical_space(key).is_extension() || term_lexical_space(item).is_extension()
                {
                    return LexicalSpace::PurrdfSuperset;
                }
            }
        }
    }
    LexicalSpace::Sep0009
}

impl CdtValue {
    /// The lexical space this value's canonical form lies in — see [`lexical_space`].
    #[must_use]
    pub fn lexical_space(&self) -> LexicalSpace {
        lexical_space(self)
    }

    /// `true` when the canonical form of this value uses one of PurRDF's two
    /// documented supersets of SEP-0009 and would therefore be ill-typed under the
    /// published grammar.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_cdt::parse_list;
    ///
    /// assert!(!parse_list("[1, 'a'@en]")?.uses_extension());
    /// assert!(parse_list("[<<(<http://example.org/s> <http://example.org/p> 1)>>]")?.uses_extension());
    /// # Ok::<(), purrdf_cdt::CdtError>(())
    /// ```
    #[must_use]
    pub fn uses_extension(&self) -> bool {
        self.lexical_space().is_extension()
    }
}

impl CdtTerm {
    /// `true` when this element needs one of PurRDF's two supersets to be written at
    /// all — see [`term_lexical_space`].
    #[must_use]
    pub fn uses_extension(&self) -> bool {
        term_lexical_space(self).is_extension()
    }
}
