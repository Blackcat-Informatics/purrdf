// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dataset-independent term interner shared by every entailment engine.
//!
//! Reasoning runs over dense `u32` ids rather than [`TermValue`]s so joins, label
//! sets, and adjacency indices stay cheap. Keys are [`TermValue`] — a value type
//! independent of any particular dataset's term table — which is what lets an
//! engine intern terms from the source dataset and re-materialize them into a fresh
//! builder soundly.

use std::hash::{Hash, Hasher};

use hashbrown::HashTable;

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermId, TermValue};

use crate::vocab::RDFS_RESOURCE;

/// Local `TermValue`→`u32` interner over dataset-independent terms.
#[derive(Default)]
pub(crate) struct Interner {
    index: HashTable<u32>,
    values: Vec<TermValue>,
}

fn hash_value(value: &TermValue) -> u64 {
    let mut hasher = ahash::AHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

impl Interner {
    /// Intern `v`, returning its stable dense id (assigned in first-seen order).
    pub(crate) fn intern(&mut self, v: TermValue) -> u32 {
        let hash = hash_value(&v);
        if let Some(&id) = self.index.find(hash, |&id| self.values[id as usize] == v) {
            return id;
        }
        let id = u32::try_from(self.values.len()).expect("term count fits u32");
        self.values.push(v);
        self.index
            .insert_unique(hash, id, |&id| hash_value(&self.values[id as usize]));
        id
    }

    /// Intern an IRI by string.
    pub(crate) fn intern_iri(&mut self, iri: &str) -> u32 {
        self.intern(TermValue::Iri(iri.to_owned()))
    }

    /// The `TermValue` behind an id.
    pub(crate) fn value(&self, id: u32) -> &TermValue {
        &self.values[id as usize]
    }

    /// The id already assigned to `iri`, if it has been interned (lookup only).
    #[cfg(test)]
    pub(crate) fn id_of_iri(&self, iri: &str) -> Option<u32> {
        let value = TermValue::Iri(iri.to_owned());
        let hash = hash_value(&value);
        self.index
            .find(hash, |&id| self.values[id as usize] == value)
            .copied()
    }

    /// Whether `id` may occupy a triple *subject* position (an IRI or blank node —
    /// never a literal or triple term reached by an inverse/range rule).
    pub(crate) fn is_subject(&self, id: u32) -> bool {
        matches!(
            self.values[id as usize],
            TermValue::Iri(_) | TermValue::Blank { .. }
        )
    }
}

/// Intern a [`TermValue`] into `b`, returning its dataset-local id.
///
/// A triple term reached in a subject/object slot (which the RDFS/OWL-RL rules
/// never derive) folds to `rdfs:Resource` rather than fabricating a term.
pub(crate) fn intern_into(b: &mut RdfDatasetBuilder, v: &TermValue) -> TermId {
    match v {
        TermValue::Iri(iri) => b.intern_iri(iri),
        TermValue::Blank { label, scope } => b.intern_blank(label, *scope),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        } => {
            let lit = if let Some(lang) = language {
                RdfLiteral::language_tagged(lexical_form, lang)
            } else {
                RdfLiteral::typed(lexical_form, datatype)
            };
            b.intern_literal(lit)
        }
        TermValue::Triple { .. } => b.intern_iri(RDFS_RESOURCE),
    }
}
