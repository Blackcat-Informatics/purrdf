// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-keyed persistent document replacement.

use super::{
    Member, Source, Spelling, StoredTerm, SurfaceDocument, SurfaceIndex, TermMetadata,
    compare_sources, distinct_codes, postings,
};
use crate::index::{Document, Table};
use crate::{CorpusProfile, TextDeltaError, TextError};
use purrdf_lex::allocation::shared::Shared;
use std::collections::BTreeSet;

impl<P: CorpusProfile> SurfaceIndex<P> {
    /// Replace complete supplied documents, analyzing only the added input.
    /// # Errors
    /// Refuses incompatible analyzer or document-coordinate owners, source
    /// collisions, allocation failure and index-size exhaustion.
    pub fn with_documents(mut self, added: Self) -> Result<Self, TextDeltaError> {
        self.check_analyzer(&added)?;
        if added.sources.is_empty() {
            return Ok(self);
        }
        if self.document_order.is_some() != added.document_order.is_some()
            || self.coordinate_generation != added.coordinate_generation
        {
            return Err(
                crate::configuration::ConfigurationRefusal::SurfaceCoordinates {
                    expected_owner: self.document_order.is_some(),
                    actual_owner: added.document_order.is_some(),
                    expected_generation: self.coordinate_generation,
                    actual_generation: added.coordinate_generation,
                }
                .into_error()
                .into(),
            );
        }
        let documents = added
            .sources
            .iter()
            .map(|source| source.document.clone())
            .collect();
        self.remove_document_keys(&documents)?;
        self = self.merge_disjoint(added)?;
        self.generation = self.content_generation();
        Ok(self)
    }

    /// Remove complete documents. Absent identifiers are no-ops.
    /// # Errors
    /// Refuses allocation failure or inconsistent retained source/index state.
    pub fn without_documents(
        mut self,
        documents: impl IntoIterator<Item = u32>,
    ) -> Result<Self, TextError> {
        let documents = documents
            .into_iter()
            .filter_map(|id| match &self.document_order {
                Some(order) => order
                    .get(id as usize)
                    .map(|document| SurfaceDocument::Indexed(document.clone())),
                None => Some(SurfaceDocument::Ordinal(id)),
            })
            .collect();
        if self.remove_document_keys(&documents)? {
            self.generation = self.content_generation();
        }
        Ok(self)
    }

    pub(crate) fn with_index_delta(
        mut self,
        added: Self,
        removed: impl IntoIterator<Item = Shared<Document<P>>>,
        order: Table<Shared<Document<P>>>,
    ) -> Result<Self, TextDeltaError> {
        self.check_analyzer(&added)?;
        if self.document_order.is_none() || added.document_order.is_none() {
            return Err(TextError::data(
                "indexed surface update requires original document-coordinate owners",
            )
            .into());
        }
        let removed = removed.into_iter().map(SurfaceDocument::Indexed).collect();
        self.remove_document_keys(&removed)?;
        self.document_order = Some(order);
        self.merge_disjoint(added)
    }

    fn check_analyzer(&self, added: &Self) -> Result<(), TextDeltaError> {
        let expected = self.analyzer.fingerprint();
        let actual = added.analyzer.fingerprint();
        if expected != actual {
            return Err(TextDeltaError::AnalyzerMismatch { expected, actual });
        }
        Ok(())
    }

    fn remove_document_keys(
        &mut self,
        documents: &BTreeSet<SurfaceDocument<P>>,
    ) -> Result<bool, TextError> {
        let mut changed = false;
        for document in documents {
            let start = self
                .sources
                .partition_point(|source| source.document < *document);
            let end = self
                .sources
                .partition_point(|source| source.document <= *document);
            let removed: Vec<_> = self.sources.slice(start, end).iter().cloned().collect();
            for source in removed {
                changed = true;
                for name in &source.words {
                    self.remove_source_term(name, &source, false)?;
                }
                for name in &source.spans {
                    self.remove_source_term(name, &source, true)?;
                }
                let at = self
                    .sources
                    .binary_search_by(|candidate| compare_sources(candidate, &source))
                    .expect("a removed surface source is still present");
                self.sources = self.sources.remove(at)?;
                self.source_state
                    .remove(source.digest)
                    .map_err(|error| TextError::data(error.to_string()))?;
            }
        }
        Ok(changed)
    }

    fn remove_source_term(
        &mut self,
        name: &Spelling,
        source: &Source<P>,
        span: bool,
    ) -> Result<(), TextError> {
        let table = if span { &self.spans } else { &self.words };
        let at = table
            .binary_search_by(|term| term.text.cmp(name))
            .map_err(|_| TextError::data("a retired surface source has no original term"))?;
        let prior = table[at].clone();
        let start = prior
            .occurrences
            .partition_point(|occurrence| compare_sources(occurrence.source(), source).is_lt());
        let end = prior
            .occurrences
            .partition_point(|occurrence| !compare_sources(occurrence.source(), source).is_gt());
        if start == end {
            return Err(TextError::data(
                "a surface source has no original term occurrences",
            ));
        }
        let mut occurrences = prior.occurrences.clone();
        for position in (start..end).rev() {
            occurrences = occurrences.remove(position)?;
        }
        let members = adjust_member(&prior.members, &source.document, end - start, false)?;
        let updated = if occurrences.is_empty() {
            self.edit_term_postings(&prior, false)?;
            None
        } else {
            Some(StoredTerm {
                text: prior.text.clone(),
                metadata: prior.metadata.clone(),
                occurrences,
                members,
            })
        };
        let table = if span {
            &mut self.spans
        } else {
            &mut self.words
        };
        *table = if let Some(term) = updated {
            table.replace(at, term)?
        } else {
            table.remove(at)?
        };
        Ok(())
    }

    pub(crate) fn merge_disjoint(mut self, added: Self) -> Result<Self, TextDeltaError> {
        let Self {
            sources,
            words,
            spans,
            source_state,
            ..
        } = added;
        for source in sources.iter() {
            let at = self
                .sources
                .binary_search_by(|candidate| compare_sources(candidate, source));
            match at {
                Ok(_) => {
                    return Err(TextDeltaError::SourceOverlap {
                        document: self.document_id(&source.document),
                        identity: source.identity,
                    });
                }
                Err(at) => self.sources = self.sources.insert(at, source.clone())?,
            }
        }
        self.source_state
            .combine(source_state)
            .map_err(|error| TextError::data(error.to_string()))?;
        for (span, table) in [(false, words), (true, spans)] {
            for term in table.iter() {
                self.insert_term(term, span)?;
            }
        }
        Ok(self)
    }

    fn insert_term(&mut self, term: &StoredTerm<P>, span: bool) -> Result<(), TextDeltaError> {
        let table = if span { &self.spans } else { &self.words };
        let position = table.binary_search_by(|prior| prior.text.cmp(&term.text));
        let updated = if let Ok(at) = position {
            let prior = &table[at];
            let mut occurrences = prior.occurrences.clone();
            for occurrence in term.occurrences.iter() {
                let at = occurrences
                    .binary_search_by(|candidate| candidate.cmp(occurrence))
                    .map_or_else(
                        |at| at,
                        |_| unreachable!("disjoint sources cannot have equal occurrences"),
                    );
                occurrences = occurrences.insert(at, occurrence.clone())?;
            }
            let mut members = prior.members.clone();
            for member in term.members.iter() {
                members = adjust_member(&members, &member.document, member.count, true)?;
            }
            StoredTerm {
                text: prior.text.clone(),
                metadata: prior.metadata.clone(),
                occurrences,
                members,
            }
        } else {
            crate::error::term_ordinal(table.len())?;
            self.edit_term_postings(term, true)?;
            term.clone()
        };
        let table = if span {
            &mut self.spans
        } else {
            &mut self.words
        };
        *table = match position {
            Ok(at) => table.replace(at, updated)?,
            Err(at) => table.insert(at, updated)?,
        };
        Ok(())
    }

    fn edit_term_postings(&mut self, term: &StoredTerm<P>, added: bool) -> Result<(), TextError> {
        match &*term.metadata {
            TermMetadata::Span(_) => {
                for (width, index) in self.grams.iter_mut().enumerate() {
                    for (key, posting) in postings::gram_rows(&term.text, width + 1) {
                        if added {
                            index.insert(key, posting)?;
                        } else {
                            index.remove(&key, &posting)?;
                        }
                    }
                }
            }
            TermMetadata::Word {
                codes: Some(codes), ..
            } => {
                for code in distinct_codes(codes) {
                    if added {
                        self.phonetic_postings
                            .insert(code.to_owned(), term.text.clone())?;
                    } else {
                        self.phonetic_postings
                            .remove(&code.to_owned(), &term.text)?;
                    }
                }
            }
            TermMetadata::Word { codes: None, .. } => {}
        }
        Ok(())
    }
}

fn adjust_member<P: CorpusProfile>(
    members: &Table<Member<P>>,
    document: &SurfaceDocument<P>,
    count: usize,
    added: bool,
) -> Result<Table<Member<P>>, TextError> {
    let position = members.binary_search_by(|member| member.document.cmp(document));
    let before = position.ok().map_or(0, |at| members[at].count);
    let after = if added {
        before.checked_add(count)
    } else {
        before.checked_sub(count)
    }
    .ok_or_else(|| TextError::data("surface document occurrence population is inconsistent"))?;
    if after == 0 {
        members.remove(
            position
                .map_err(|_| TextError::data("a removed surface document membership is absent"))?,
        )
    } else {
        let member = Member {
            document: document.clone(),
            count: after,
        };
        match position {
            Ok(at) => members.replace(at, member),
            Err(at) => members.insert(at, member),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SurfaceIndex;
    use crate::error::term_ordinal;
    use crate::{Analyzer, TextDeltaError};

    #[test]
    fn delta_capacity_refuses_before_allocating_an_unaddressable_table() {
        let limit = usize::try_from(u32::MAX).unwrap();
        assert_eq!(term_ordinal(limit - 1), Ok(u32::MAX - 1));
        let refusal = term_ordinal(limit).unwrap_err();
        assert!(
            matches!(refusal, TextDeltaError::TermCapacity { observed, limit, .. }
            if observed == u128::from(u32::MAX) + 1 && limit == u32::MAX)
        );
        assert!(
            std::error::Error::source(&refusal)
                .unwrap()
                .is::<std::num::TryFromIntError>()
        );
        assert!(matches!(
            term_ordinal(usize::MAX),
            Err(TextDeltaError::TermCapacity { .. })
        ));
    }

    #[test]
    fn conflicting_source_tables_retain_the_original_identity() {
        let original =
            SurfaceIndex::from_texts(Analyzer::empty_lexicon(), [(7, "source")]).unwrap();
        let identity = original.sources[0].identity;
        let duplicate = original.clone();
        assert!(matches!(original.merge_disjoint(duplicate),
            Err(TextDeltaError::SourceOverlap { document: 7, identity: actual }) if actual == identity));
    }

    #[test]
    fn document_merge_retains_shared_word_code_allocations() {
        let analyzer = Analyzer::empty_lexicon();
        let base =
            SurfaceIndex::from_texts(analyzer.clone(), [(0, "Smith"), (2, "Robert")]).unwrap();
        let word = base
            .words
            .iter()
            .position(|word| word.text.text() == "smith")
            .unwrap();
        let allocation = base.words[word].pronunciation().unwrap().1.as_ptr();
        let added = SurfaceIndex::from_texts(analyzer, [(1, "aardvark Smith")]).unwrap();
        let updated = base.with_documents(added).unwrap();
        let word = updated
            .words
            .iter()
            .position(|word| word.text.text() == "smith")
            .unwrap();
        assert_eq!(
            updated.words[word].pronunciation().unwrap().1.as_ptr(),
            allocation
        );
        let matches = updated.phonetic("Smith").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].term.documents().collect::<Vec<_>>(), [0, 1]);
    }
}
