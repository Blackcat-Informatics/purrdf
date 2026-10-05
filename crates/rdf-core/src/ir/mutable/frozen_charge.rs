// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The terms and payload bytes a snapshot's frozen delta will retain, counted
//! before the freeze allocates it.
//!
//! [`super::MutableDataset::append_delta`] interns, into a fresh builder, every
//! term of every live added row and every declared graph name. The
//! builder dedups terms by value and stores a literal's datatype and each triple
//! component as terms of their own. This walk mirrors that interning without
//! materialising a value: each distinct term is keyed by its borrowed parts and
//! by the keys of the terms below it, so a term is hashed in time proportional
//! to its own parts, however deep its nesting.

use crate::RdfTextDirection;
use crate::hash::FastMap;
use crate::ir::{BlankScope, RdfDataset, TermId, TermRef, TermValue};

/// One distinct term of the frozen delta.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Key<'a> {
    /// A term the base already holds: the delta builder interns its value again.
    Base(TermId),
    Iri(&'a str),
    Blank(&'a str, BlankScope),
    Literal {
        lexical: &'a str,
        datatype: u32,
        language: Option<&'a str>,
        direction: Option<RdfTextDirection>,
    },
    Triple(u32, u32, u32),
}

/// Distinct terms keyed as the delta builder will intern them, with the arena
/// text each one carries.
pub(super) struct FrozenTerms<'a> {
    base: &'a RdfDataset,
    keys: FastMap<Key<'a>, u32>,
    text: usize,
}

impl<'a> FrozenTerms<'a> {
    pub(super) fn new(base: &'a RdfDataset) -> Self {
        Self {
            base,
            keys: FastMap::default(),
            text: 0,
        }
    }

    /// The number of distinct terms charged so far.
    pub(super) fn terms(&self) -> usize {
        self.keys.len()
    }

    /// The arena text bytes of the distinct terms charged so far.
    pub(super) const fn text_bytes(&self) -> usize {
        self.text
    }

    /// Charge `key` once, adding `text` the first time; its index either way.
    fn charge(&mut self, key: Key<'a>, text: usize) -> u32 {
        let next = u32::try_from(self.keys.len()).expect("frozen delta term count fits u32");
        *self.keys.entry(key).or_insert_with(|| {
            self.text += text;
            next
        })
    }

    /// Charge a base term and, below it, its datatype or triple components.
    pub(super) fn base_term(&mut self, id: TermId) -> u32 {
        let base = self.base;
        let mut pending = vec![id];
        while let Some(term) = pending.pop() {
            if self.keys.contains_key(&Key::Base(term)) {
                continue;
            }
            let text = match base.resolve(term) {
                TermRef::Iri(iri) => iri.len(),
                TermRef::Blank { label, .. } => label.len(),
                TermRef::Literal {
                    lexical,
                    datatype,
                    language,
                    ..
                } => {
                    pending.push(datatype);
                    lexical.len() + language.map_or(0, str::len)
                }
                TermRef::Triple { s, p, o } => {
                    pending.extend([s, p, o]);
                    0
                }
            };
            self.charge(Key::Base(term), text);
        }
        self.keys[&Key::Base(id)]
    }

    /// Charge an IRI the base may or may not hold.
    fn iri(&mut self, iri: &'a str) -> u32 {
        match self.base.term_id_by_iri(iri) {
            Some(id) => self.base_term(id),
            None => self.charge(Key::Iri(iri), iri.len()),
        }
    }

    /// Charge a term value and everything below it, bottom up over a work list.
    pub(super) fn value(&mut self, value: &'a TermValue) {
        // Each entry is a term and whether its components are already charged; a
        // charged term leaves its `(index, base id)` on `done`.
        let mut pending: Vec<(&'a TermValue, bool)> = vec![(value, false)];
        let mut done: Vec<(u32, Option<TermId>)> = Vec::new();
        while let Some((term, expanded)) = pending.pop() {
            let charged = match term {
                TermValue::Triple { s, p, o } if !expanded => {
                    pending.extend([(term, true), (&**o, false), (&**p, false), (&**s, false)]);
                    continue;
                }
                TermValue::Triple { .. } => {
                    let (o, o_id) = done.pop().expect("an object was charged");
                    let (p, p_id) = done.pop().expect("a predicate was charged");
                    let (s, s_id) = done.pop().expect("a subject was charged");
                    let base = match (s_id, p_id, o_id) {
                        (Some(s), Some(p), Some(o)) => self.base.term_id_by_triple(s, p, o),
                        _ => None,
                    };
                    match base {
                        Some(id) => (self.base_term(id), Some(id)),
                        None => (self.charge(Key::Triple(s, p, o), 0), None),
                    }
                }
                TermValue::Iri(iri) => {
                    let id = self.base.term_id_by_iri(iri);
                    (self.iri(iri), id)
                }
                TermValue::Blank { label, scope } => {
                    match self.base.term_id_by_blank(label, *scope) {
                        Some(id) => (self.base_term(id), Some(id)),
                        None => (self.charge(Key::Blank(label, *scope), label.len()), None),
                    }
                }
                TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                } => match self.base.term_id_by_literal(
                    lexical_form,
                    datatype,
                    language.as_deref(),
                    *direction,
                ) {
                    Some(id) => (self.base_term(id), Some(id)),
                    None => {
                        // A language tag names the datatype whatever the explicit one
                        // says, exactly as the builder interns it.
                        let datatype = if language.is_some() {
                            crate::RdfLiteral::language_datatype_iri(*direction)
                        } else {
                            datatype.as_str()
                        };
                        let datatype = self.iri(datatype);
                        let text = lexical_form.len() + language.as_ref().map_or(0, String::len);
                        let key = Key::Literal {
                            lexical: lexical_form,
                            datatype,
                            language: language.as_deref(),
                            direction: *direction,
                        };
                        (self.charge(key, text), None)
                    }
                },
            };
            done.push(charged);
        }
        debug_assert_eq!(done.len(), 1, "exactly the value is left charged");
    }
}
