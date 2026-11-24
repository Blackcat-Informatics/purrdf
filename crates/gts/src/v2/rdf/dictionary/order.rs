// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::convert::Infallible;

use crate::FastMap;
use crate::v2::fields::copy_bytes;
use crate::v2::terms::TRIPLE_KIND;
use crate::v2::{
    DecodedTerms, FoldError, LiteralDirection, RdfFold, Term, TermDefinition, TermText, TypedValue,
    ValueReference,
};

use super::{LiteralFields, Run, SegmentDictionary};

// Variant order and field order are the keys of P3 §11.1.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Key<B> {
    Iri(B),
    Literal(B, (u8, B)),
    LiteralRef([u8; 32], u64, (u8, B)),
    Value(Value<B>),
    ValueRef([u8; 32], u64),
    Blank(B),
    Triple(u64, [u64; 3]),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Value<B> {
    Boolean(bool),
    Unsigned(u64),
    Integer(i128),
    Time(i64, u32),
    Octets(B),
}

impl<B> Key<B> {
    fn map<'a, T, E>(&'a self, map: impl Fn(&'a B) -> Result<T, E>) -> Result<Key<T>, E> {
        Ok(match self {
            Self::Iri(bytes) => Key::Iri(map(bytes)?),
            Self::Literal(bytes, (form, name)) => Key::Literal(map(bytes)?, (*form, map(name)?)),
            Self::LiteralRef(digest, len, (form, name)) => {
                Key::LiteralRef(*digest, *len, (*form, map(name)?))
            }
            Self::Value(value) => Key::Value(match value {
                Value::Boolean(value) => Value::Boolean(*value),
                Value::Unsigned(value) => Value::Unsigned(*value),
                Value::Integer(value) => Value::Integer(*value),
                Value::Time(seconds, nanos) => Value::Time(*seconds, *nanos),
                Value::Octets(bytes) => Value::Octets(map(bytes)?),
            }),
            Self::ValueRef(digest, len) => Key::ValueRef(*digest, *len),
            Self::Blank(bytes) => Key::Blank(map(bytes)?),
            Self::Triple(depth, ids) => Key::Triple(*depth, *ids),
        })
    }
}

type LastKey = Key<Vec<u8>>;

#[derive(Debug, Default)]
pub(super) struct History {
    pub(super) last: Option<LastKey>,
    depths: FastMap<u64, u64>,
    unreadable_iris: FastMap<u64, Vec<u8>>,
}

impl History {
    pub(super) fn rollback(&mut self, start: u128) {
        self.depths.retain(|id, _| u128::from(*id) < start);
        self.unreadable_iris.retain(|id, _| u128::from(*id) < start);
    }
}

impl SegmentDictionary {
    pub(super) fn check_order(
        &mut self,
        fold: &RdfFold,
        decoded: &DecodedTerms<'_>,
        canonical: bool,
    ) -> Result<(bool, Option<LastKey>), FoldError> {
        let base = self.next;
        let wire_id = |index: usize| {
            u64::try_from(base + index as u128)
                .expect("the checked term reservation fits the wire IDs")
        };
        let current_term = |id| {
            let index = u128::from(id).checked_sub(base)?;
            decoded.terms.get(usize::try_from(index).ok()?)
        };
        let iri_text = |id| {
            if u128::from(id) >= base {
                return match current_term(id) {
                    Some(TermDefinition::Iri(text)) => Some(text.bytes()),
                    _ => None,
                };
            }
            self.order
                .unreadable_iris
                .get(&id)
                .map(Vec::as_slice)
                .or_else(|| match fold.term(*self.readable.get(&id)?)? {
                    Term::Iri(text) => Some(text.as_bytes()),
                    _ => None,
                })
        };
        let is_atom = |id| {
            if u128::from(id) >= base {
                return current_term(id)
                    .is_some_and(|term| !matches!(term, TermDefinition::Triple(_)));
            }
            // A readable triple has a depth, because its components were
            // readable in ID order. A failed depth lookup therefore leaves
            // only atoms among the earlier readable IDs.
            self.readable.contains_key(&id)
                || self.order.unreadable_iris.contains_key(&id)
                || reserved_atom(&self.runs, id)
        };
        let previous = self.order.last.as_ref().map(|key| {
            key.map(|bytes| Ok::<_, Infallible>(bytes.as_slice()))
                .unwrap_or_else(|never| match never {})
        });
        let mut last = None;
        let mut violated = false;
        let depths = &mut self.order.depths;
        for (index, definition) in decoded.terms.iter().enumerate() {
            if !canonical && !matches!(definition, TermDefinition::Triple(_)) {
                continue;
            }
            let id = wire_id(index);
            let key = if let TermDefinition::Triple(ids) = definition {
                let Some(depth) = triple_depth(depths, id, *ids, is_atom) else {
                    continue;
                };
                depths.try_reserve(1)?;
                depths.insert(id, depth);
                if !canonical {
                    continue;
                }
                Key::Triple(depth, *ids)
            } else {
                let Some(key) = atom_key(id, definition, &decoded.languages, iri_text)? else {
                    continue;
                };
                key
            };
            violated = violated
                || last
                    .as_ref()
                    .or(previous.as_ref())
                    .is_some_and(|before| *before >= key);
            last = Some(key);
        }
        let last = last
            .map(|key| key.map(|bytes| copy_bytes(bytes)))
            .transpose()?;
        for (index, definition) in decoded.terms.iter().enumerate() {
            if let TermDefinition::Iri(text) = definition
                && (self.in_doubt || matches!(text, TermText::Invalid(_)))
            {
                self.order.unreadable_iris.try_reserve(1)?;
                self.order
                    .unreadable_iris
                    .insert(wire_id(index), copy_bytes(text.bytes())?);
            }
        }
        Ok((violated, last))
    }
}

fn reserved_atom(runs: &[Run], id: u64) -> bool {
    runs.get(runs.partition_point(|run| run.end <= u128::from(id)))
        .is_some_and(|run| run.kind != TRIPLE_KIND)
}

fn triple_depth(
    depths: &FastMap<u64, u64>,
    id: u64,
    ids: [u64; 3],
    is_atom: impl Fn(u64) -> bool,
) -> Option<u64> {
    let mut depth = 0;
    for member in ids {
        if member >= id {
            return None;
        }
        if let Some(&child) = depths.get(&member) {
            depth = depth.max(child);
        } else if !is_atom(member) {
            return None;
        }
    }
    Some(depth + 1)
}

fn atom_key<'a>(
    id: u64,
    definition: &'a TermDefinition<'_>,
    languages: &'a [&'a str],
    iri_text: impl Fn(u64) -> Option<&'a [u8]>,
) -> Result<Option<Key<&'a [u8]>>, FoldError> {
    let form_key = |form| {
        Ok::<_, FoldError>(match LiteralFields::read(form, languages)? {
            LiteralFields::Typed(datatype) => (datatype < id)
                .then(|| iri_text(datatype))
                .flatten()
                .map(|text| (1, text)),
            LiteralFields::Language(text, direction) => Some((
                match direction {
                    None => 2,
                    Some(LiteralDirection::LeftToRight) => 3,
                    Some(LiteralDirection::RightToLeft) => 4,
                },
                text.as_bytes(),
            )),
        })
    };
    Ok(match definition {
        TermDefinition::Iri(text) => Some(Key::Iri(text.bytes())),
        TermDefinition::Literal(text, form) => {
            form_key(*form)?.map(|form| Key::Literal(text.bytes(), form))
        }
        TermDefinition::LiteralRef(ValueReference { digest, length }, form) => {
            form_key(*form)?.map(|form| Key::LiteralRef(**digest, *length, form))
        }
        TermDefinition::Value(value) => Some(Key::Value(match value {
            TypedValue::Boolean(value) => Value::Boolean(*value),
            TypedValue::Unsigned(value) => Value::Unsigned(*value),
            TypedValue::Integer(value) => Value::Integer(*value),
            TypedValue::Time {
                seconds,
                nanoseconds,
            } => Value::Time(*seconds, *nanoseconds),
            TypedValue::Octets(bytes) => Value::Octets(bytes),
        })),
        TermDefinition::ValueRef(ValueReference { digest, length }) => {
            Some(Key::ValueRef(**digest, *length))
        }
        TermDefinition::Blank(label) => Some(Key::Blank(label)),
        TermDefinition::Triple(_) => unreachable!("triple depths are computed before ordering"),
    })
}
