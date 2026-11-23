// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::borrow::Cow;

use purrdf_hash::{fixed::hash_one, hex::encode_upper_into};
use purrdf_xsd::datatype;

use super::FoldError;
use crate::v2::{TypedValue, fields::copy_bytes};
use crate::{FastMap, FastSet};

/// A handle to a term interned in one file fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TermId(usize);

impl TermId {
    /// The index in this fold's term arena.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Direction of a language literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LiteralDirection {
    /// Left to right.
    LeftToRight,
    /// Right to left.
    RightToLeft,
}

/// A resolved GTS v2 term.
#[derive(Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Term<'a> {
    /// IRI text.
    Iri(Cow<'a, str>),
    /// A blank node scoped by the segment's ordinal in the file.
    Blank {
        /// File-relative segment ordinal.
        segment: u64,
        /// Original opaque label.
        label: Cow<'a, [u8]>,
    },
    /// A typed literal.
    Typed {
        /// Literal lexical form.
        lexical: Cow<'a, str>,
        /// Handle to the datatype IRI.
        datatype: TermId,
    },
    /// A language literal.
    Language {
        /// Literal lexical form.
        lexical: Cow<'a, str>,
        /// Language text in the original spelling.
        language: Cow<'a, str>,
        /// Optional direction.
        direction: Option<LiteralDirection>,
    },
    /// Subject, predicate and object handles.
    Triple([TermId; 3]),
}

impl Term<'_> {
    fn into_owned(self) -> Result<Term<'static>, FoldError> {
        Ok(match self {
            Self::Iri(text) => Term::Iri(own_text(text)?),
            Self::Blank { segment, label } => Term::Blank {
                segment,
                label: match label {
                    Cow::Borrowed(label) => Cow::Owned(copy_bytes(label)?),
                    Cow::Owned(label) => Cow::Owned(label),
                },
            },
            Self::Typed { lexical, datatype } => Term::Typed {
                lexical: own_text(lexical)?,
                datatype,
            },
            Self::Language {
                lexical,
                language,
                direction,
            } => Term::Language {
                lexical: own_text(lexical)?,
                language: own_text(language)?,
                direction,
            },
            Self::Triple(terms) => Term::Triple(terms),
        })
    }
}

fn own_text(text: Cow<'_, str>) -> Result<Cow<'static, str>, FoldError> {
    Ok(Cow::Owned(match text {
        Cow::Owned(text) => text,
        Cow::Borrowed(text) => {
            let mut owned = String::new();
            owned.try_reserve_exact(text.len())?;
            owned.push_str(text);
            owned
        }
    }))
}

/// The original typed value paired with its generated literal.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LiteralValue {
    /// Boolean.
    Boolean(bool),
    /// Unsigned integer.
    Unsigned(u64),
    /// Signed CBOR integer.
    Integer(i128),
    /// UTC epoch seconds and fractional nanoseconds.
    Time(i64, u32),
    /// Original octets.
    Octets(Vec<u8>),
}

impl LiteralValue {
    fn copy(value: TypedValue<'_>) -> Result<Self, FoldError> {
        Ok(match value {
            TypedValue::Boolean(value) => Self::Boolean(value),
            TypedValue::Unsigned(value) => Self::Unsigned(value),
            TypedValue::Integer(value) => Self::Integer(value),
            TypedValue::Time {
                seconds,
                nanoseconds,
            } => Self::Time(seconds, nanoseconds),
            TypedValue::Octets(value) => Self::Octets(copy_bytes(value)?),
        })
    }
}

#[derive(Debug)]
struct Entry {
    term: Term<'static>,
    hash: u64,
    next: Option<TermId>,
}

#[derive(Debug, Default)]
pub(super) struct TermArena {
    entries: Vec<Entry>,
    buckets: FastMap<u64, TermId>,
    value_ids: FastSet<TermId>,
    values: Vec<(TermId, LiteralValue)>,
}

impl TermArena {
    pub(super) fn terms(
        &self,
    ) -> impl DoubleEndedIterator<Item = &Term<'static>> + ExactSizeIterator {
        self.entries.iter().map(|entry| &entry.term)
    }

    pub(super) fn get(&self, id: TermId) -> Option<&Term<'static>> {
        self.entries.get(id.index()).map(|entry| &entry.term)
    }

    pub(super) fn values(&self) -> &[(TermId, LiteralValue)] {
        &self.values
    }

    pub(super) fn intern(&mut self, term: Term<'_>) -> Result<TermId, FoldError> {
        let hash = hash_one(&term);
        let mut candidate = self.buckets.get(&hash).copied();
        let next = candidate;
        while let Some(id) = candidate {
            let entry = &self.entries[id.index()];
            if entry.term == term {
                return Ok(id);
            }
            candidate = entry.next;
        }
        self.entries.try_reserve(1)?;
        self.buckets.try_reserve(1)?;
        let term = term.into_owned()?;
        let id = TermId(self.entries.len());
        self.entries.push(Entry { term, hash, next });
        self.buckets.insert(hash, id);
        Ok(id)
    }

    pub(super) fn intern_value(&mut self, value: TypedValue<'_>) -> Result<TermId, FoldError> {
        let (datatype, lexical) = value_lexical(value)?;
        let datatype = self.intern(Term::Iri(Cow::Borrowed(datatype)))?;
        let id = self.intern(Term::Typed { lexical, datatype })?;
        if !self.value_ids.contains(&id) {
            self.value_ids.try_reserve(1)?;
            self.values.try_reserve(1)?;
            self.values.push((id, LiteralValue::copy(value)?));
            self.value_ids.insert(id);
        }
        Ok(id)
    }

    pub(super) const fn checkpoint(&self) -> (usize, usize) {
        (self.entries.len(), self.values.len())
    }

    pub(super) fn rollback(&mut self, (terms, values): (usize, usize)) {
        for (id, _) in self.values.drain(values..) {
            self.value_ids.remove(&id);
        }
        while self.entries.len() > terms {
            let entry = self.entries.pop().expect("the arena holds a new term");
            if let Some(previous) = entry.next {
                self.buckets.insert(entry.hash, previous);
            } else {
                self.buckets.remove(&entry.hash);
            }
        }
    }
}

fn value_lexical(value: TypedValue<'_>) -> Result<(&'static str, Cow<'static, str>), FoldError> {
    Ok(match value {
        TypedValue::Boolean(value) => (
            datatype::XSD_BOOLEAN,
            Cow::Borrowed(if value { "true" } else { "false" }),
        ),
        TypedValue::Unsigned(value) => (
            datatype::XSD_NON_NEGATIVE_INTEGER,
            Cow::Owned(value.to_string()),
        ),
        TypedValue::Integer(value) => (datatype::XSD_INTEGER, Cow::Owned(value.to_string())),
        TypedValue::Time {
            seconds,
            nanoseconds,
        } => {
            if seconds < -62_135_596_800 {
                return Err(FoldError::Definitions);
            }
            (
                datatype::XSD_DATE_TIME,
                Cow::Owned(
                    purrdf_xsd::rfc3339::format(seconds, nanoseconds).map_err(FoldError::Time)?,
                ),
            )
        }
        TypedValue::Octets(value) => {
            let mut lexical = String::new();
            lexical.try_reserve_exact(value.len().saturating_mul(2))?;
            encode_upper_into(value, &mut lexical);
            (datatype::XSD_HEX_BINARY, Cow::Owned(lexical))
        }
    })
}
