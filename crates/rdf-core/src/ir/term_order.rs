// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed canonical field order shared by owned terms and flat dictionaries.

use core::cmp::Ordering;
use core::convert::Infallible;

use super::term::canonical_kind_tag;
use crate::{BlankScope, RdfTextDirection, TermValue};

/// The canonical kind and scalar fields of one RDF term, without owned children.
///
/// This is the shallow part of [`TermValue`]'s order: IRI, literal, blank, then
/// triple. Two triple keys compare equal; the caller must then compare their
/// subject, predicate and object using its original term representation. A flat
/// dictionary can therefore share the native field order without expanding DAGs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShallowTermKey<'a> {
    /// The original IRI lexical value.
    Iri(&'a str),
    /// Literal fields in their canonical comparison precedence.
    Literal {
        /// The resolved datatype IRI value, rather than its local identifier.
        datatype: &'a str,
        /// The original identity-folded language tag, when present.
        language: Option<&'a str>,
        /// The original lexical form.
        lexical: &'a str,
        /// The original RDF base direction, when present.
        direction: Option<RdfTextDirection>,
    },
    /// Blank identity, compared by label and then scope.
    Blank {
        /// The original blank label.
        label: &'a str,
        /// Its original identity scope.
        scope: BlankScope,
    },
    /// A triple kind marker; components remain in the caller's graph.
    Triple {},
}

impl ShallowTermKey<'_> {
    const fn canonical_tag(self) -> u8 {
        canonical_kind_tag!(self)
    }

    pub(crate) fn try_cmp_with<E>(
        &self,
        other: &Self,
        mut text: impl FnMut(&str, &str) -> Result<Ordering, E>,
    ) -> Result<Ordering, E> {
        let kind = self.canonical_tag().cmp(&other.canonical_tag());
        if !kind.is_eq() {
            return Ok(kind);
        }
        match (*self, *other) {
            (Self::Iri(a), Self::Iri(b)) => text(a, b),
            (
                Self::Literal {
                    datatype: da,
                    language: ga,
                    lexical: la,
                    direction: dira,
                },
                Self::Literal {
                    datatype: db,
                    language: gb,
                    lexical: lb,
                    direction: dirb,
                },
            ) => {
                let order = text(da, db)?;
                if !order.is_eq() {
                    return Ok(order);
                }
                let order = match (ga, gb) {
                    (None, None) => Ordering::Equal,
                    (None, Some(_)) => Ordering::Less,
                    (Some(_), None) => Ordering::Greater,
                    (Some(a), Some(b)) => text(a, b)?,
                };
                if !order.is_eq() {
                    return Ok(order);
                }
                let order = text(la, lb)?;
                if !order.is_eq() {
                    return Ok(order);
                }
                Ok(dira.cmp(&dirb))
            }
            (
                Self::Blank {
                    label: la,
                    scope: sa,
                },
                Self::Blank {
                    label: lb,
                    scope: sb,
                },
            ) => {
                let order = text(la, lb)?;
                if !order.is_eq() {
                    return Ok(order);
                }
                Ok(sa.cmp(&sb))
            }
            _ => Ok(Ordering::Equal),
        }
    }
}

impl Ord for ShallowTermKey<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.try_cmp_with(other, |a, b| Ok::<_, Infallible>(a.cmp(b))) {
            Ok(order) => order,
            Err(never) => match never {},
        }
    }
}

impl PartialOrd for ShallowTermKey<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> From<&'a TermValue> for ShallowTermKey<'a> {
    fn from(value: &'a TermValue) -> Self {
        match value {
            TermValue::Iri(iri) => Self::Iri(iri),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => Self::Literal {
                datatype,
                language: language.as_deref(),
                lexical: lexical_form,
                direction: *direction,
            },
            TermValue::Blank { label, scope } => Self::Blank {
                label,
                scope: *scope,
            },
            TermValue::Triple { .. } => Self::Triple {},
        }
    }
}
