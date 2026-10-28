// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one frozen lexical owner for native RDF carriers.

use super::dataset::TermRef;
use super::term::{BlankScope, InternedTerm, TermId, TermValue, arena_str};
use crate::content_id::{Blake3ContentId, ContentIdScheme};
use crate::hash::FastHasher;
use crate::{RdfLiteral, RdfTextDirection};
use hashbrown::HashTable;
use std::collections::HashMap;

/// Moved from the validated interner, never reconstructed from unchecked IDs.
/// No graph or statement table belongs to this owner.
#[derive(Debug, Clone)]
pub(crate) struct FrozenTerms {
    /// Owns every lexical byte once; term ranges borrow from this allocation.
    pub(super) arena: Box<[u8]>,
    /// Dense original local IDs; transferring this owner never remaps them.
    pub(super) terms: Box<[InternedTerm]>,
    /// The builder's store-once reverse index, transferred intact. Equality
    /// resolves through the same arena; no lazy whole-dictionary rebuild occurs.
    pub(super) term_index: HashTable<u32>,
    /// Derived recognition results, never a serialization input. Public egress
    /// sorts by local ID rather than exposing hash iteration order.
    pub(super) content_ids: HashMap<TermId, Blake3ContentId, FastHasher>,
    /// Original caller-selected recognition scheme; None means inactive.
    pub(super) content_scheme: Option<ContentIdScheme>,
    /// Lookup-only resolution of the configured predicate, if it was interned.
    pub(super) derivation_predicate: Option<TermId>,
    /// Retained even when absent from the dictionary so later rebuilding uses
    /// exactly the original caller configuration, never an invented default.
    pub(super) derivation_predicate_iri: Option<Box<str>>,
}

impl FrozenTerms {
    /// Stream the same value encoding as TermValue without reconstructing an
    /// owned term or copying lexical bytes. Batch callers reuse one admitted
    /// pending buffer; local IDs never enter the emitted content.
    pub(super) fn visit_canonical_bytes<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        id: TermId,
        pending: &mut Vec<TermId>,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
        mut output: impl FnMut(&[u8]),
    ) -> Result<(), purrdf_lex::allocation::StorageError> {
        use super::term::CanonicalNode;
        pending.clear();
        let mut current = Some(id);
        while let Some(id) = current {
            let node = match self.resolve(id) {
                TermRef::Iri(iri) => CanonicalNode::Iri(iri),
                TermRef::Blank { label, scope } => CanonicalNode::Blank { label, scope },
                TermRef::Literal {
                    lexical,
                    datatype,
                    language,
                    direction,
                } => {
                    let TermRef::Iri(datatype) = self.resolve(datatype) else {
                        unreachable!("validated literal datatype is an IRI")
                    };
                    CanonicalNode::Literal {
                        lexical,
                        datatype,
                        language,
                        direction,
                    }
                }
                TermRef::Triple { s, p, o } => {
                    for child in [o, p, s] {
                        memory.push(pending, child)?;
                    }
                    CanonicalNode::Triple
                }
            };
            node.visit_bytes(&mut output);
            current = pending.pop();
        }
        Ok(())
    }

    /// Bind every retained lexical owner, including unused blank names consulted
    /// by deterministic mint collision checks. Triple references name entries in
    /// this complete ordered dictionary; they never stand in for absent content.
    /// This linear representation seal avoids expanding nested triples once for
    /// every ancestor in the dictionary.
    pub(super) fn snapshot_fingerprint<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<[u8; 32], purrdf_lex::allocation::StorageError> {
        const DICTIONARY: purrdf_hash::Domain =
            purrdf_hash::Domain::new(b"purrdf-core/snapshot/dictionary/v1");
        let mut hash = purrdf_hash::blake3::RecordHasher::new();
        hash.update(DICTIONARY.as_bytes());
        hash.update(&(self.terms.len() as u64).to_le_bytes());
        memory.try_scope(|memory| {
            let mut pending = Vec::new();
            for (index, term) in self.terms.iter().enumerate() {
                if let TermRef::Triple { s, p, o } = self.resolve_interned(term) {
                    super::term::CanonicalNode::Triple.visit_bytes(&mut |bytes| {
                        hash.update(bytes);
                    });
                    for child in [s, p, o] {
                        hash.update(&(child.index() as u32).to_le_bytes());
                    }
                } else {
                    let id = TermId::from_index(
                        u32::try_from(index)
                            .map_err(|_| purrdf_lex::allocation::StorageError::SizeOverflow)?,
                    )?;
                    self.visit_canonical_bytes(id, &mut pending, memory, |bytes| {
                        hash.update(bytes);
                    })?;
                }
            }
            Ok::<_, purrdf_lex::allocation::StorageError>(*hash.finalize().as_bytes())
        })
    }

    /// Reifier targets have already passed the frozen triple-term validation.
    pub(super) fn triple_components(&self, target: TermId) -> [TermId; 3] {
        match self.terms[target.index()] {
            InternedTerm::Triple { s, p, o } => [s, p, o],
            _ => panic!("a validated reifier target must be a triple term"),
        }
    }

    pub(super) fn content_ids(&self) -> impl Iterator<Item = (TermId, Blake3ContentId)> + '_ {
        let mut pairs: Vec<(TermId, Blake3ContentId)> = self
            .content_ids
            .iter()
            .map(|(&id, &cid)| (id, cid))
            .collect();
        pairs.sort_unstable_by_key(|(id, _)| *id);
        pairs.into_iter()
    }

    fn iri_matches(&self, id: TermId, expected: &str) -> bool {
        matches!(&self.terms[id.index()], InternedTerm::Iri(iri) if arena_str(&self.arena, *iri) == expected)
    }

    fn find_term_hashed(
        &self,
        hash: u64,
        mut matches: impl FnMut(TermId) -> bool,
    ) -> Option<TermId> {
        self.term_index
            .find(hash, |&index| {
                matches(TermId::from_index(index).expect("stored term index is representable"))
            })
            .copied()
            .map(|index| TermId::from_index(index).expect("stored term index is representable"))
    }

    #[must_use]
    pub(super) fn term_id_by_iri(&self, iri: &str) -> Option<TermId> {
        self.find_term_hashed(crate::hash::hash_iri_for_interner(iri), |id| {
            self.iri_matches(id, iri)
        })
    }

    #[must_use]
    pub(super) fn term_id_by_blank(&self, label: &str, scope: BlankScope) -> Option<TermId> {
        self.find_term_hashed(crate::hash::hash_blank_for_interner(label, scope.0), |id| {
            matches!(
                &self.terms[id.index()],
                InternedTerm::Blank { label: stored, scope: stored_scope }
                    if arena_str(&self.arena, *stored) == label && *stored_scope == scope
            )
        })
    }

    #[must_use]
    pub(super) fn term_id_by_literal(
        &self,
        lexical_form: &str,
        datatype: &str,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
    ) -> Option<TermId> {
        // A language tag names the datatype whatever the explicit one says (C0.1),
        // identical to `RdfDatasetBuilder::intern_literal_parts`.
        let datatype = if language.is_some() {
            RdfLiteral::language_datatype_iri(direction)
        } else {
            datatype
        };
        let language = language.map(super::term::interned_language);
        let language = language.as_deref();

        let datatype_id = self.term_id_by_iri(datatype)?;
        let hash = crate::hash::hash_literal_for_interner(
            lexical_form,
            datatype_id.index() as u64,
            language,
            direction,
        );
        self.find_term_hashed(hash, |id| {
            let InternedTerm::Literal(lit) = &self.terms[id.index()] else {
                return false;
            };
            arena_str(&self.arena, lit.lexical_form) == lexical_form
                && lit.datatype == datatype_id
                && lit.language.map(|r| arena_str(&self.arena, r)) == language
                && lit.direction == direction
        })
    }

    #[must_use]
    pub(super) fn term_id_by_triple(&self, s: TermId, p: TermId, o: TermId) -> Option<TermId> {
        if s.index() >= self.terms.len()
            || p.index() >= self.terms.len()
            || o.index() >= self.terms.len()
        {
            return None;
        }
        let hash = crate::hash::hash_triple_for_interner(
            s.index() as u64,
            p.index() as u64,
            o.index() as u64,
        );
        self.find_term_hashed(hash, |id| {
            matches!(
                self.terms[id.index()],
                InternedTerm::Triple { s: stored_s, p: stored_p, o: stored_o }
                    if stored_s == s && stored_p == p && stored_o == o
            )
        })
    }

    #[must_use]
    pub(super) fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
        value
            .try_fold(
                |leaf| {
                    match leaf {
                        TermValue::Iri(iri) => self.term_id_by_iri(iri),
                        TermValue::Blank { label, scope } => self.term_id_by_blank(label, *scope),
                        TermValue::Literal {
                            lexical_form,
                            datatype,
                            language,
                            direction,
                        } => self.term_id_by_literal(
                            lexical_form,
                            datatype,
                            language.as_deref(),
                            *direction,
                        ),
                        TermValue::Triple { .. } => {
                            unreachable!("a triple term is assembled from its components")
                        }
                    }
                    .ok_or(())
                },
                |s, p, o| self.term_id_by_triple(s, p, o).ok_or(()),
            )
            .ok()
    }

    pub(super) fn resolve(&self, id: TermId) -> TermRef<'_> {
        self.resolve_interned(&self.terms[id.index()])
    }

    /// Resolve original lexical ranges without reconstructing a local ID.
    pub(super) fn resolve_interned(&self, term: &InternedTerm) -> TermRef<'_> {
        match term {
            InternedTerm::Iri(iri) => TermRef::Iri(arena_str(&self.arena, *iri)),
            InternedTerm::Blank { label, scope } => TermRef::Blank {
                label: arena_str(&self.arena, *label),
                scope: *scope,
            },
            InternedTerm::Literal(lit) => TermRef::Literal {
                lexical: arena_str(&self.arena, lit.lexical_form),
                datatype: lit.datatype,
                language: lit.language.map(|r| arena_str(&self.arena, r)),
                direction: lit.direction,
            },
            InternedTerm::Triple { s, p, o } => TermRef::Triple {
                s: *s,
                p: *p,
                o: *o,
            },
        }
    }
}
