// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ordered replacement of source-backed surface documents.

use super::postings::{FlatPostings, GramPosting, Joined, join_by};
use super::{Occurrence, Source, SurfaceIndex, SurfaceTerm};
use crate::TextDeltaError;
use crate::error::term_ordinal;

impl SurfaceIndex {
    /// Insert the supplied complete documents, replacing any existing documents
    /// with the same identifiers. Only the supplied index's sources need analysis.
    /// Retained source alignment, span shapes and pronunciation codes are moved
    /// into the result; existing gram and code postings are merged in order.
    ///
    /// # Errors
    /// Refuses a different analyzer or a merged term table past its ID space.
    pub fn with_documents(self, added: Self) -> Result<Self, TextDeltaError> {
        let expected = self.analyzer.fingerprint();
        let actual = added.analyzer.fingerprint();
        if expected != actual {
            return Err(TextDeltaError::AnalyzerMismatch { expected, actual });
        }
        if added.sources.is_empty() {
            return Ok(self);
        }
        let retained = self.without_documents(added.sources.iter().map(|source| source.document));
        let mut result = retained.merge_disjoint(added)?;
        result.generation = result.content_generation();
        Ok(result)
    }

    /// Inputs were admitted and rebased by the enclosing text-index update.
    pub(crate) fn merge_disjoint(self, added: Self) -> Result<Self, TextDeltaError> {
        let retained = self;
        let sources = merge_sources(retained.sources, added.sources)?;
        let words = merge_terms(
            retained
                .words
                .into_iter()
                .zip(retained.codes.into_iter().zip(retained.canonical)),
            added
                .words
                .into_iter()
                .zip(added.codes.into_iter().zip(added.canonical)),
            &sources.retained,
            &sources.added,
        )?;
        let spans = merge_terms(
            retained.spans.into_iter().zip(retained.shapes),
            added.spans.into_iter().zip(added.shapes),
            &sources.retained,
            &sources.added,
        )?;
        let remap = |table: FlatPostings<u64, GramPosting>, ordinals: &[u32]| {
            table.retain_remapped(|posting| {
                Some(GramPosting {
                    span: ordinals[posting.span as usize],
                    ..posting
                })
            })
        };
        let [left_one, left_two, left_three] = retained.grams;
        let [right_one, right_two, right_three] = added.grams;
        let grams = [
            remap(left_one, &spans.retained).merge(remap(right_one, &spans.added)),
            remap(left_two, &spans.retained).merge(remap(right_two, &spans.added)),
            remap(left_three, &spans.retained).merge(remap(right_three, &spans.added)),
        ];
        let phonetic_postings = retained
            .phonetic_postings
            .retain_remapped(|word| Some(words.retained[word as usize]))
            .merge(
                added
                    .phonetic_postings
                    .retain_remapped(|word| Some(words.added[word as usize])),
            );
        let (codes, canonical) = words.metadata.into_iter().unzip();
        let result = Self {
            analyzer: retained.analyzer,
            generation: [0; 32],
            sources: sources.rows,
            words: words.terms,
            spans: spans.terms,
            shapes: spans.metadata,
            grams,
            codes,
            canonical,
            phonetic_postings,
        };
        Ok(result)
    }
}

struct MergedSources {
    rows: Vec<Source>,
    retained: Vec<usize>,
    added: Vec<usize>,
}

fn merge_sources(
    retained: Vec<Source>,
    added: Vec<Source>,
) -> Result<MergedSources, TextDeltaError> {
    let mut retained_ids = vec![0; retained.len()];
    let mut added_ids = vec![0; added.len()];
    let mut sources = Vec::with_capacity(retained.len() + added.len());
    for row in join_by(
        retained.into_iter().enumerate(),
        added.into_iter().enumerate(),
        |a, b| {
            a.1.document
                .cmp(&b.1.document)
                .then_with(|| a.1.identity.cmp(&b.1.identity))
        },
    ) {
        let source = match row {
            Joined::Left((ordinal, source)) => {
                retained_ids[ordinal] = sources.len();
                source
            }
            Joined::Right((ordinal, source)) => {
                added_ids[ordinal] = sources.len();
                source
            }
            Joined::Both((_, source), _) => {
                return Err(TextDeltaError::SourceOverlap {
                    document: source.document,
                    identity: source.identity,
                });
            }
        };
        sources.push(source);
    }
    Ok(MergedSources {
        rows: sources,
        retained: retained_ids,
        added: added_ids,
    })
}

struct MergedTerms<M> {
    terms: Vec<SurfaceTerm>,
    metadata: Vec<M>,
    retained: Vec<u32>,
    added: Vec<u32>,
}

fn merge_terms<M>(
    retained: impl ExactSizeIterator<Item = (SurfaceTerm, M)>,
    added: impl ExactSizeIterator<Item = (SurfaceTerm, M)>,
    retained_sources: &[usize],
    added_sources: &[usize],
) -> Result<MergedTerms<M>, TextDeltaError> {
    let mut result = MergedTerms {
        retained: vec![0; retained.len()],
        added: vec![0; added.len()],
        terms: Vec::new(),
        metadata: Vec::new(),
    };
    for row in join_by(retained.enumerate(), added.enumerate(), |a, b| {
        a.1.0.text.cmp(&b.1.0.text)
    }) {
        let ordinal = term_ordinal(result.terms.len())?;
        let (term, metadata) = match row {
            Joined::Left((old, (mut term, metadata))) => {
                result.retained[old] = ordinal;
                remap_sources(&mut term, retained_sources);
                (term, metadata)
            }
            Joined::Right((new, (mut term, metadata))) => {
                result.added[new] = ordinal;
                remap_sources(&mut term, added_sources);
                (term, metadata)
            }
            Joined::Both((old, (mut left, metadata)), (new, (mut right, _))) => {
                result.retained[old] = ordinal;
                result.added[new] = ordinal;
                remap_sources(&mut left, retained_sources);
                remap_sources(&mut right, added_sources);
                left.documents = join_by(left.documents, right.documents, u32::cmp)
                    .map(Joined::one)
                    .collect();
                left.occurrences = join_by(
                    left.occurrences,
                    right.occurrences,
                    |a: &Occurrence, b: &Occurrence| {
                        (a.source, a.range.start, a.range.end).cmp(&(
                            b.source,
                            b.range.start,
                            b.range.end,
                        ))
                    },
                )
                .map(Joined::one)
                .collect();
                (left, metadata)
            }
        };
        result.terms.push(term);
        result.metadata.push(metadata);
    }
    Ok(result)
}

fn remap_sources(term: &mut SurfaceTerm, ordinals: &[usize]) {
    for occurrence in &mut term.occurrences {
        occurrence.source = ordinals[occurrence.source];
    }
}

#[cfg(test)]
mod tests {
    use super::{SurfaceIndex, merge_sources, term_ordinal};
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
        let capture = purrdf_testkit::error_capture::Capture::read(&refusal);
        assert_eq!(capture.code, "text-delta-term-capacity");
        assert!(
            matches!(capture.field("observed"), purrdf_hash::error::ErrorValue::Unsigned(value) if value == u128::from(u32::MAX) + 1)
        );
        assert_eq!(capture.child("cause").code, "integer-conversion-range");
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
        let duplicate = original.sources.clone();
        assert!(matches!(merge_sources(original.sources, duplicate),
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
            .position(|word| word.text == "smith")
            .unwrap();
        let allocation = base.canonical[word].as_ref().unwrap().as_ptr();
        let added = SurfaceIndex::from_texts(analyzer, [(1, "aardvark Smith")]).unwrap();
        let updated = base.with_documents(added).unwrap();
        let word = updated
            .words
            .iter()
            .position(|word| word.text == "smith")
            .unwrap();
        assert_eq!(
            updated.canonical[word].as_ref().unwrap().as_ptr(),
            allocation
        );
        let matches = updated.phonetic("Smith").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].term.documents(), [0, 1]);
    }
}
