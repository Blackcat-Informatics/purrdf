// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-bound updates over retained lexical and auxiliary postings.

use std::collections::BTreeSet;

use super::document_key;
use super::{
    CorpusProfile, DatasetView, Document, Ordering, SourceRow, TermEntry, TextIndex, analyze_rows,
    build_occurrence_order, build_partitions, build_subject_order, check_document_count,
    collect_rows, digest_rows, posting_spans,
};
use crate::TextDeltaError;
use crate::error::term_ordinal;
use crate::surface::postings::{Joined, join_by};

impl<P: CorpusProfile> TextIndex<P> {
    /// Update this index using effective added and removed statements and the
    /// next immutable source. The inputs use the same physical carrier profile.
    /// Both asserted and annotation literal rows participate, including their
    /// multiplicity across layers. Nonliteral statements update source coverage.
    ///
    /// Reversing the supplied delta must recover this index's source identity.
    /// Only changed documents are analyzed. Retained token, gram, pronunciation
    /// and source-alignment buffers are reused, with canonical IDs rebased.
    /// Statistics and fingerprints agree with a fresh build under the same law.
    ///
    /// This consumes the index; clone it first to retain an older snapshot.
    /// Cloning copies its buffers. Source collection, canonical hashing and
    /// ordered table merges still traverse retained data; this is not a promise
    /// of constant or logarithmic total update work.
    ///
    /// # Errors
    /// Refuses inconsistent deltas before changing index tables. Preserves
    /// original source, analysis, capacity and ranking errors as causes.
    pub fn with_delta<D: DatasetView, A: DatasetView, R: DatasetView>(
        mut self,
        next: &D,
        added: &A,
        removed: &R,
    ) -> Result<Self, TextDeltaError> {
        let (rows, coverage) = collect_rows(next, &self.config)?;
        let (additions, _) = collect_rows(added, &self.config)?;
        let (removals, _) = collect_rows(removed, &self.config)?;
        validate_delta(&rows, &additions, &removals, self.source_fingerprint)?;
        let source_fingerprint = digest_rows(rows.iter());
        if source_fingerprint == self.source_fingerprint {
            self.coverage = coverage;
            return Ok(self);
        }
        let changed: BTreeSet<_> = additions
            .iter()
            .chain(&removals)
            .map(|row| (&row.graph, &row.subject, &row.language))
            .collect();
        let changed_rows: Vec<_> = rows
            .into_iter()
            .filter(|row| changed.contains(&(&row.graph, &row.subject, &row.language)))
            .collect();
        let (documents, dictionary) = analyze_rows(&changed_rows, &self.config)?;
        let partial = Self::assemble(
            self.config.clone(),
            documents,
            &dictionary,
            source_fingerprint,
            coverage.clone(),
            self.ranking.clone(),
        )?;
        self.merge_changed(partial, &changed)?;
        self.source_fingerprint = source_fingerprint;
        self.coverage = coverage;
        self.fingerprint = self.compute_fingerprint();
        if let Some(surface) = &mut self.surface {
            surface.bind_generation(self.fingerprint);
        }
        Ok(self)
    }

    fn merge_changed(
        &mut self,
        mut partial: Self,
        changed: &BTreeSet<DocumentIdentity<'_, P>>,
    ) -> Result<(), TextDeltaError> {
        let mut old_ids = vec![None; self.documents.len()];
        let mut new_ids = vec![None; partial.documents.len()];
        let retained = std::mem::take(&mut self.documents)
            .into_iter()
            .enumerate()
            .filter(|(_, document)| {
                !changed.contains(&(&document.graph, &document.subject, &document.language))
            });
        let added = std::mem::take(&mut partial.documents)
            .into_iter()
            .enumerate();
        for joined in join_by(retained, added, |a, b| compare_documents(&a.1, &b.1)) {
            check_document_count(self.documents.len() + 1)?;
            let id = self.documents.len() as u32;
            let document = match joined {
                Joined::Left((old, document)) => {
                    old_ids[old] = Some(id);
                    document
                }
                Joined::Right((new, document)) => {
                    new_ids[new] = Some(id);
                    document
                }
                Joined::Both(_, _) => {
                    unreachable!("changed documents were removed before insertion")
                }
            };
            self.documents.push(document);
        }
        let (partitions, partition_of) = build_partitions(self.documents.iter().map(|document| {
            (
                &document.graph,
                &document.subject,
                &document.language,
                document.length,
            )
        }))?;
        self.partitions = partitions;
        for document in &mut self.documents {
            document.partition = partition_of[&document_key(
                &document.graph,
                &document.subject,
                document.language.as_deref(),
            )];
        }
        let merged = merge_terms(
            std::mem::take(&mut self.terms),
            partial.terms,
            &old_ids,
            &new_ids,
            &self.documents,
        )?;
        self.terms = merged.entries;
        let mut streams = vec![Vec::new(); self.documents.len()];
        for (input, documents, terms) in [
            (
                std::mem::take(&mut self.token_streams),
                &old_ids,
                &merged.retained,
            ),
            (partial.token_streams, &new_ids, &merged.added),
        ] {
            for (mut stream, document) in input.into_iter().zip(documents) {
                if let Some(document) = *document {
                    for token in &mut stream {
                        token.term =
                            terms[token.term as usize].expect("retained token has a posting");
                    }
                    streams[document as usize] = stream;
                }
            }
        }
        self.token_streams = streams;
        self.subject_order = build_subject_order(&self.documents);
        self.occurrence_order = build_occurrence_order(&self.documents);
        let old_han = rebase_han(std::mem::take(&mut self.han_occurrences), &old_ids);
        let new_han = rebase_han(partial.han_occurrences, &new_ids);
        self.han_occurrences = join_by(old_han, new_han, |a, b| a.document.cmp(&b.document))
            .map(Joined::one)
            .collect();
        self.surface = match (self.surface.take(), partial.surface) {
            (Some(retained), Some(added)) => Some(
                retained
                    .rebase_documents(&old_ids)
                    .merge_disjoint(added.rebase_documents(&new_ids))?,
            ),
            (None, None) => None,
            _ => unreachable!("partial index uses the same surface configuration"),
        };
        self.rebuild_field_statistics()?;
        Ok(())
    }
}

type DocumentIdentity<'a, P> = (
    &'a <P as CorpusProfile>::Graph,
    &'a purrdf_core::TermValue,
    &'a Option<String>,
);

fn compare_documents<P: CorpusProfile>(a: &Document<P>, b: &Document<P>) -> Ordering {
    (&a.graph, &a.subject, &a.language).cmp(&(&b.graph, &b.subject, &b.language))
}

/// Reverse the multiset delta without retaining another copy of literal bytes.
fn validate_delta<P: CorpusProfile>(
    rows: &[SourceRow<P>],
    added: &[SourceRow<P>],
    removed: &[SourceRow<P>],
    expected: [u8; 32],
) -> Result<(), TextDeltaError> {
    let mut retained = Vec::with_capacity(rows.len());
    for row in join_by(rows.iter().enumerate(), added.iter().enumerate(), |a, b| {
        a.1.cmp(b.1)
    }) {
        match row {
            Joined::Left((_, row)) => retained.push(row),
            Joined::Both(_, _) => {}
            Joined::Right((ordinal, _)) => return Err(TextDeltaError::MissingAddition { ordinal }),
        }
    }
    let prior: Vec<_> = join_by(retained, removed, Ord::cmp)
        .flat_map(|row| match row {
            Joined::Left(row) | Joined::Right(row) => [Some(row), None],
            Joined::Both(left, right) => [Some(left), Some(right)],
        })
        .flatten()
        .collect();
    let actual = digest_rows(prior.into_iter());
    if expected != actual {
        return Err(TextDeltaError::SourceMismatch { expected, actual });
    }
    Ok(())
}

struct MergedTerms {
    entries: Vec<TermEntry>,
    retained: Vec<Option<u32>>,
    added: Vec<Option<u32>>,
}

fn merge_terms<P: CorpusProfile>(
    retained: Vec<TermEntry>,
    added: Vec<TermEntry>,
    old_ids: &[Option<u32>],
    new_ids: &[Option<u32>],
    documents: &[Document<P>],
) -> Result<MergedTerms, TextDeltaError> {
    let mut result = MergedTerms {
        retained: vec![None; retained.len()],
        added: vec![None; added.len()],
        entries: Vec::new(),
    };
    let left = rebase_terms(retained, old_ids);
    let right = rebase_terms(added, new_ids);
    for joined in join_by(left, right, |a, b| a.1.term.cmp(&b.1.term)) {
        let ordinal = term_ordinal(result.entries.len())?;
        let mut entry = match joined {
            Joined::Left((old, entry)) => {
                result.retained[old] = Some(ordinal);
                entry
            }
            Joined::Right((new, entry)) => {
                result.added[new] = Some(ordinal);
                entry
            }
            Joined::Both((old, mut left), (new, right)) => {
                result.retained[old] = Some(ordinal);
                result.added[new] = Some(ordinal);
                left.postings = join_by(left.postings, right.postings, |a, b| {
                    (documents[a.document as usize].partition, a.document)
                        .cmp(&(documents[b.document as usize].partition, b.document))
                })
                .map(Joined::one)
                .collect();
                left
            }
        };
        entry.spans = posting_spans(&entry.postings, documents);
        result.entries.push(entry);
    }
    Ok(result)
}

fn rebase_terms(
    terms: Vec<TermEntry>,
    documents: &[Option<u32>],
) -> impl Iterator<Item = (usize, TermEntry)> + '_ {
    terms.into_iter().enumerate().filter_map(|(id, mut entry)| {
        entry.postings.retain_mut(|posting| {
            if let Some(document) = documents[posting.document as usize] {
                posting.document = document;
                true
            } else {
                false
            }
        });
        (!entry.postings.is_empty()).then_some((id, entry))
    })
}

fn rebase_han(
    rows: Vec<crate::character::HanOccurrence>,
    documents: &[Option<u32>],
) -> impl Iterator<Item = crate::character::HanOccurrence> + '_ {
    rows.into_iter().filter_map(|mut row| {
        row.document = documents[row.document as usize]?;
        Some(row)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Analyzer, GraphFree, GraphSelector, TextIndexConfig};
    use purrdf_core::TermValue;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
    use std::sync::Arc;

    type Row = (
        &'static str,
        &'static str,
        Option<&'static str>,
        Option<&'static str>,
        bool,
    );

    fn dataset(rows: &[Row]) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let predicate = builder.intern_iri("urn:text").unwrap();
        for &(subject, text, language, graph, annotation) in rows {
            let subject = builder.intern_iri(subject).unwrap();
            let graph = graph.map(|graph| builder.intern_iri(graph).unwrap());
            let literal = language.map_or_else(
                || RdfLiteral::simple(text),
                |language| RdfLiteral::language_tagged(text, language),
            );
            let object = builder.intern_literal(literal).unwrap();
            if annotation {
                let quoted = builder.intern_triple(subject, predicate, object).unwrap();
                builder.push_reifier(subject, quoted).unwrap();
                builder.push_annotation(subject, predicate, object).unwrap();
            } else {
                builder
                    .push_quad(subject, predicate, object, graph)
                    .unwrap();
            }
        }
        builder.freeze().unwrap()
    }

    fn config() -> TextIndexConfig {
        TextIndexConfig::new(
            vec![TermValue::iri("urn:text")],
            GraphSelector::Any,
            Analyzer::empty_lexicon(),
        )
        .unwrap()
    }

    fn assert_fresh<P: CorpusProfile>(actual: &TextIndex<P>, fresh: &TextIndex<P>) {
        assert_eq!(actual.source_fingerprint, fresh.source_fingerprint);
        assert_eq!(actual.fingerprint, fresh.fingerprint);
        assert_eq!(actual.coverage, fresh.coverage);
        assert_eq!(actual.documents, fresh.documents);
        assert_eq!(actual.partitions, fresh.partitions);
        assert_eq!(actual.terms, fresh.terms);
        assert_eq!(actual.token_streams, fresh.token_streams);
        assert_eq!(actual.field_lengths, fresh.field_lengths);
        assert_eq!(actual.field_totals, fresh.field_totals);
        assert_eq!(actual.field_populations, fresh.field_populations);
        assert_eq!(actual.subject_order, fresh.subject_order);
        assert_eq!(actual.occurrence_order, fresh.occurrence_order);
        assert_eq!(
            format!("{:?}", actual.han_occurrences),
            format!("{:?}", fresh.han_occurrences)
        );
        for (partition, _) in &fresh.partitions {
            for needle in ["river", "bank", "schmidt", "東京", "missing"] {
                let terms = vec![needle.to_owned()];
                assert_eq!(
                    crate::rank_partition(actual, partition, &terms, None).unwrap(),
                    crate::rank_partition(fresh, partition, &terms, None).unwrap()
                );
            }
        }
        if let (Some(actual), Some(fresh)) = (&actual.surface, &fresh.surface) {
            for needle in ["r", "iv", "bank", "東京", "é", "c++"] {
                assert_eq!(
                    format!("{:?}", actual.substring_report(needle)),
                    format!("{:?}", fresh.substring_report(needle))
                );
            }
            assert_eq!(
                format!("{:?}", actual.phonetic("smith").unwrap()),
                format!("{:?}", fresh.phonetic("smith").unwrap())
            );
        }
    }

    #[test]
    fn text_delta_replaces_inserts_retracts_and_rebases_all_tables() {
        let before = [
            ("urn:b", "river bank", Some("en"), Some("urn:g"), false),
            ("urn:b", "c++ café", Some("en"), Some("urn:g"), false),
            ("urn:c", "東京", Some("ja"), None, false),
            ("urn:d", "schmidt bank", Some("en"), None, true),
            ("urn:e", "river", Some("fr"), None, false),
        ];
        let removed = [before[0], before[4]];
        let added = [
            ("urn:a", "alpha river", Some("de"), None, false),
            (
                "urn:b",
                "river valley bank",
                Some("en"),
                Some("urn:g"),
                false,
            ),
            ("urn:f", "---", None, None, false),
        ];
        let after: Vec<_> = before
            .iter()
            .copied()
            .filter(|row| !removed.contains(row))
            .chain(added)
            .collect();
        let old = dataset(&before);
        let next = dataset(&after);
        for config in [
            config(),
            config().for_han(),
            config().with_surface_index(false),
        ] {
            let index = TextIndex::from_dataset(&*old, &config).unwrap();
            let snapshot = index.clone();
            let retained = index
                .documents
                .iter()
                .position(|doc| doc.subject == TermValue::iri("urn:c"))
                .unwrap();
            let buffer = index.token_streams[retained].as_ptr();
            let calls = crate::analysis::PROJECTION_CALLS.get();
            let updated = index
                .with_delta(&*next, &*dataset(&added), &*dataset(&removed))
                .unwrap();
            assert_eq!(
                crate::analysis::PROJECTION_CALLS.get() - calls,
                4,
                "only the four literal rows in changed documents are analyzed"
            );
            let fresh = TextIndex::from_dataset(&*next, &config).unwrap();
            assert_fresh(&updated, &fresh);
            let retained = updated
                .documents
                .iter()
                .position(|doc| doc.subject == TermValue::iri("urn:c"))
                .unwrap();
            assert_eq!(updated.token_streams[retained].as_ptr(), buffer);
            assert_fresh(&snapshot, &TextIndex::from_dataset(&*old, &config).unwrap());
        }
    }

    #[test]
    fn text_delta_preserves_cross_layer_multiplicity_and_empty_transitions() {
        let row = ("urn:s", "river bank", None, None, false);
        let annotation = ("urn:s", "river bank", None, None, true);
        let empty = dataset(&[]);
        let one = dataset(&[row]);
        let two = dataset(&[row, annotation]);
        let mut index = TextIndex::from_dataset(&*empty, &config()).unwrap();
        for (next, added, removed) in [
            (&one, &one, &empty),
            (&two, &dataset(&[annotation]), &empty),
            (&one, &empty, &dataset(&[annotation])),
            (&empty, &empty, &one),
        ] {
            index = index.with_delta(&**next, &**added, &**removed).unwrap();
            assert_fresh(
                &index,
                &TextIndex::from_dataset(&**next, &config()).unwrap(),
            );
        }
        let generation = index.fingerprint;
        index = index.with_delta(&*empty, &*empty, &*empty).unwrap();
        assert_eq!(index.fingerprint, generation);
    }

    #[test]
    fn text_delta_rejects_missing_and_incomplete_changes_with_original_operands() {
        let old = dataset(&[("urn:s", "river", None, None, false)]);
        let next = dataset(&[("urn:s", "bank", None, None, false)]);
        let empty = dataset(&[]);
        let index = TextIndex::from_dataset(&*old, &config()).unwrap();
        assert!(matches!(
            index.clone().with_delta(&*next, &*old, &*empty),
            Err(TextDeltaError::MissingAddition { ordinal: 0 })
        ));
        let expected = index.source_fingerprint;
        let actual = TextIndex::from_dataset(&*next, &config())
            .unwrap()
            .source_fingerprint;
        assert_eq!(
            index.with_delta(&*next, &*empty, &*empty).unwrap_err(),
            TextDeltaError::SourceMismatch { expected, actual }
        );
    }

    #[test]
    fn text_delta_graph_free_carrier_matches_fresh() {
        use purrdf_core::ir::TripleDataset;
        let convert = |rows: &[Row]| {
            TripleDataset::try_from(Arc::try_unwrap(dataset(rows)).unwrap()).unwrap()
        };
        let row = ("urn:s", "river bank", Some("en"), None, false);
        let annotation = ("urn:r", "schmidt", Some("de"), None, true);
        let before = convert(&[row]);
        let after = convert(&[annotation]);
        let added = convert(&[annotation]);
        let removed = convert(&[row]);
        let config = TextIndexConfig::<GraphFree>::for_triples(
            vec![TermValue::iri("urn:text")],
            Analyzer::empty_lexicon(),
        )
        .unwrap();
        let index = TextIndex::from_dataset(&before.read().unwrap(), &config).unwrap();
        let actual = index
            .with_delta(
                &after.read().unwrap(),
                &added.read().unwrap(),
                &removed.read().unwrap(),
            )
            .unwrap();
        assert_fresh(
            &actual,
            &TextIndex::from_dataset(&after.read().unwrap(), &config).unwrap(),
        );
    }

    #[test]
    fn text_delta_chained_population_and_fielded_ranking_parity() {
        let rows = [
            ("urn:a", "river bank", Some("en"), Some("urn:a"), false),
            ("urn:b", "river river", Some("en"), Some("urn:a"), false),
            ("urn:a", "schmidt", Some("de"), Some("urn:b"), false),
            ("urn:c", "c++ café", None, None, true),
            ("urn:a", "東京", Some("ja"), None, false),
            ("urn:d", "---", None, None, false),
        ];
        let profile = crate::RankingProfile::new(
            vec![
                crate::RankingField::new("title", crate::Fixed::from_integer(2).unwrap(), crate::B)
                    .unwrap(),
            ],
            vec![(TermValue::iri("urn:text"), 0)],
            None,
        )
        .unwrap();
        for config in [
            config(),
            config().for_han(),
            config().with_surface_index(false),
        ] {
            let mut current = Vec::new();
            let mut index =
                TextIndex::from_dataset_with_ranking(&*dataset(&[]), &config, profile.clone())
                    .unwrap();
            for mask in (0..64).map(|n| n ^ (n >> 1)).chain([0]) {
                let next_rows: Vec<_> = rows
                    .iter()
                    .enumerate()
                    .filter(|(bit, _)| mask & (1 << bit) != 0)
                    .map(|(_, row)| *row)
                    .collect();
                let added: Vec<_> = next_rows
                    .iter()
                    .copied()
                    .filter(|row| !current.contains(row))
                    .collect();
                let removed: Vec<_> = current
                    .iter()
                    .copied()
                    .filter(|row| !next_rows.contains(row))
                    .collect();
                let next = dataset(&next_rows);
                index = index
                    .with_delta(&*next, &*dataset(&added), &*dataset(&removed))
                    .unwrap();
                let fresh =
                    TextIndex::from_dataset_with_ranking(&*next, &config, profile.clone()).unwrap();
                assert_fresh(&index, &fresh);
                current = next_rows;
            }
        }
    }
}
