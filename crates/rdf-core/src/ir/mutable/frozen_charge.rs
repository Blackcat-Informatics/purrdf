// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The terms and payload bytes a snapshot's frozen delta will retain, counted
//! before the freeze allocates it.
//!
//! [`super::MutableDataset::append_delta`] interns, into a fresh builder, every
//! term of every live added row and every declared graph name. This walk mirrors
//! that interning term for term, without materialising a value, and must agree
//! with [`RdfDatasetBuilder`](crate::ir::RdfDatasetBuilder) on every rule that
//! decides whether two values are one interned term and what it stores:
//!
//! - an IRI is stored as written, and a blank node as its `(label, scope)` pair;
//! - a literal's datatype is a term of its own, and a language tag names the
//!   datatype whatever the explicit one says
//!   ([`RdfLiteral::language_datatype_iri`](crate::RdfLiteral::language_datatype_iri));
//! - a language tag is keyed and stored folded
//!   ([`interned_language`](crate::ir::term::interned_language), the builder's own
//!   fold);
//! - a composite (`cdt:List` / `cdt:Map`) literal interns every blank node its
//!   lexical form embeds
//!   ([`cdt_embedded_blanks`](crate::cdt_blank::cdt_embedded_blanks), the
//!   builder's own extraction);
//! - a triple term is identified by its interned components, each a term of its
//!   own.
//!
//! Each distinct term is keyed by its borrowed parts and by the indices of the
//! terms below it, so a term is hashed in time proportional to its own parts,
//! however deep its nesting, and the walk keeps its own work list rather than
//! the call stack.

use std::borrow::Cow;

use crate::RdfTextDirection;
use crate::hash::FastMap;
use crate::ir::term::interned_language;
use crate::ir::{BlankScope, RdfDataset, TermId, TermRef, TermValue};

/// One distinct term of the frozen delta, keyed as the builder dedups it.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Key<'a> {
    /// A term the base already holds: the delta builder interns its value again.
    Base(TermId),
    Iri(&'a str),
    Blank(Cow<'a, str>, BlankScope),
    Literal {
        lexical: &'a str,
        datatype: u32,
        language: Option<Cow<'a, str>>,
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

    /// Charge the blank nodes a composite literal's lexical form embeds, exactly as
    /// the builder interns them alongside the literal.
    fn embedded_blanks(&mut self, lexical: &str, datatype: &str) {
        for (label, scope) in crate::cdt_blank::cdt_embedded_blanks(lexical, datatype) {
            self.blank(Cow::Owned(label), scope);
        }
    }

    /// Charge a blank node the base may or may not hold.
    fn blank(&mut self, label: Cow<'a, str>, scope: BlankScope) -> u32 {
        match self.base.term_id_by_blank(&label, scope) {
            Some(id) => self.base_term(id),
            None => {
                let text = label.len();
                self.charge(Key::Blank(label, scope), text)
            }
        }
    }

    /// Charge a base term and, below it, its datatype, embedded blanks or triple
    /// components.
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
                    if let TermRef::Iri(datatype) = base.resolve(datatype) {
                        for (label, scope) in
                            crate::cdt_blank::cdt_embedded_blanks(lexical, datatype)
                        {
                            match base.term_id_by_blank(&label, scope) {
                                Some(blank) => pending.push(blank),
                                None => {
                                    let text = label.len();
                                    self.charge(Key::Blank(Cow::Owned(label), scope), text);
                                }
                            }
                        }
                    }
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

    /// Charge a term value and everything below it, bottom up over a work list;
    /// the index of the value's own term.
    pub(super) fn value(&mut self, value: &'a TermValue) -> u32 {
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
                    let id = self.base.term_id_by_blank(label, *scope);
                    (self.blank(Cow::Borrowed(label), *scope), id)
                }
                TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                } => {
                    let language = language.as_deref().map(interned_language);
                    match self.base.term_id_by_literal(
                        lexical_form,
                        datatype,
                        language.as_deref(),
                        *direction,
                    ) {
                        Some(id) => (self.base_term(id), Some(id)),
                        None => {
                            let datatype = if language.is_some() {
                                crate::RdfLiteral::language_datatype_iri(*direction)
                            } else {
                                datatype.as_str()
                            };
                            let datatype_index = self.iri(datatype);
                            self.embedded_blanks(lexical_form, datatype);
                            let text =
                                lexical_form.len() + language.as_ref().map_or(0, |l| l.len());
                            let key = Key::Literal {
                                lexical: lexical_form,
                                datatype: datatype_index,
                                language,
                                direction: *direction,
                            };
                            (self.charge(key, text), None)
                        }
                    }
                }
            };
            done.push(charged);
        }
        debug_assert_eq!(done.len(), 1, "exactly the value is left charged");
        done.pop().expect("the value was charged").0
    }
}
