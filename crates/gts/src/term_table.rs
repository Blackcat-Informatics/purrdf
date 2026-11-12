// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The same validation view over materialized and streaming term ownership.

use std::ops::Deref;
use std::sync::Arc;

use crate::dictionary::{DictionaryError, TermLease};
use crate::model::Term;
use crate::model::TermKind;
use crate::reader::term_decode::TermReferences;

/// The same admitted kind/reference law over owned terms or a source-bound
/// metadata projection. Navigation does not confer raw term ownership.
pub(crate) trait TermMetadata: Copy {
    fn len(self) -> usize;
    fn kind(self, id: usize) -> Result<TermKind, DictionaryError>;
    fn references(self, id: usize) -> Result<Option<TermReferences>, DictionaryError>;
    fn traversal_limit(self) -> Option<usize>;
}

pub(crate) trait StoredTerms {
    fn limits(&self) -> crate::dictionary::DictionaryLimits;
    fn len(&self) -> usize;
    fn get(&self, id: usize) -> Result<Option<TermLease>, DictionaryError>;
    fn append(&self, term: &Term) -> Result<TermLease, DictionaryError>;
    fn bind_first(&self, id: usize, triple: crate::model::Triple3) -> Result<(), DictionaryError>;
}

pub(crate) enum TermRef<'a> {
    Borrowed(&'a Term),
    Stored(TermLease),
}

impl Deref for TermRef<'_> {
    type Target = Term;

    fn deref(&self) -> &Term {
        match self {
            Self::Borrowed(term) => term,
            Self::Stored(term) => term,
        }
    }
}

/// Full terms remain available until segment validation finishes. Streaming
/// readers share their owners with the resolver instead of copying strings.
#[derive(Clone, Copy)]
pub(crate) enum TermsRef<'a> {
    Owned(&'a [Term]),
    Shared(&'a [Arc<Term>]),
    Stored(&'a dyn StoredTerms),
}
impl TermMetadata for TermsRef<'_> {
    fn len(self) -> usize {
        self.len()
    }
    fn kind(self, id: usize) -> Result<TermKind, DictionaryError> {
        self.required(id).map(|term| term.kind)
    }
    fn references(self, id: usize) -> Result<Option<TermReferences>, DictionaryError> {
        self.get(id)
            .map(|term| term.map(|term| TermReferences::from_term(&term)))
    }
    fn traversal_limit(self) -> Option<usize> {
        match self {
            Self::Stored(storage) => Some(storage.limits().metadata_entries),
            Self::Owned(_) | Self::Shared(_) => None,
        }
    }
}

impl<'a> TermsRef<'a> {
    pub(crate) fn from_sources(
        stored: Option<&'a dyn StoredTerms>,
        shared: Option<&'a [Arc<Term>]>,
        owned: &'a [Term],
    ) -> Self {
        if let Some(terms) = stored {
            Self::Stored(terms)
        } else if let Some(terms) = shared {
            Self::Shared(terms)
        } else {
            Self::Owned(owned)
        }
    }

    pub(crate) fn len(self) -> usize {
        match self {
            Self::Owned(terms) => terms.len(),
            Self::Shared(terms) => terms.len(),
            Self::Stored(terms) => terms.len(),
        }
    }

    pub(crate) fn get(self, id: usize) -> Result<Option<TermRef<'a>>, DictionaryError> {
        match self {
            Self::Owned(terms) => Ok(terms.get(id).map(TermRef::Borrowed)),
            Self::Shared(terms) => Ok(terms.get(id).map(|term| TermRef::Borrowed(term))),
            Self::Stored(terms) => terms.get(id).map(|term| term.map(TermRef::Stored)),
        }
    }

    pub(crate) fn required(self, id: usize) -> Result<TermRef<'a>, DictionaryError> {
        self.get(id)?.ok_or(DictionaryError::Corrupt(
            "a validated term identifier is absent from the dictionary",
        ))
    }
}
