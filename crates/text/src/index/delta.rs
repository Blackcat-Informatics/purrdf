// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-bound updates over retained lexical and auxiliary postings.

use std::collections::BTreeSet;
use std::ops::ControlFlow;

use purrdf_core::source::{SourceFamily, SourceSide, SourceVersion, SourceVisit};
use purrdf_core::{GraphSelector, StatisticsStream};
use purrdf_hash::multiset::Multiset256;

use super::{
    CorpusProfile, DatasetView, Document, SourceCoverage, SourceDigest, SourceRow, TextIndex,
    analyze_rows, collect_rows, collect_subject_rows,
};
use super::{StoredToken, share_buffer};
use crate::surface::postings::{Joined, join_by};
use crate::{TextDeltaError, TextError};

impl<P: CorpusProfile> TextIndex<P> {
    /// Update this index using effective added and removed statements and the
    /// next immutable source. The inputs use the same physical carrier profile.
    /// Both asserted and annotation literal rows participate, including their
    /// multiplicity across layers. Nonliteral statements update source coverage.
    ///
    /// Reversing the supplied delta must recover this index's source identity.
    /// Only changed documents are analyzed. Retained token buffers are shared
    /// through a persistent positional table; private term addresses stay stable
    /// while public token ordinals follow the canonical dictionary order.
    /// Statistics and fingerprints agree with a fresh build under the same law.
    ///
    /// This consumes the index; clone it first to retain an older snapshot.
    /// Cloning shares document, lexical, partition, side-index, character and
    /// surface storage. Independent source observations and subject probes
    /// replace a complete next-source collection. Surface edits retain source
    /// evidence and update only affected term, membership and posting paths.
    ///
    /// # Errors
    /// Refuses inconsistent deltas before changing index tables. Preserves
    /// original source, analysis, capacity and ranking errors as causes.
    /// Refuses providers without independently maintained source evidence.
    pub fn with_delta<D: DatasetView, A: DatasetView, R: DatasetView>(
        mut self,
        next: &D,
        added: &A,
        removed: &R,
    ) -> Result<Self, TextDeltaError>
    where
        D::ReadError: purrdf_hash::error::StructuredError,
        A::ReadError: purrdf_hash::error::StructuredError,
        R::ReadError: purrdf_hash::error::StructuredError,
    {
        let (additions, added_coverage) = collect_rows(added, &self.config)?;
        let (removals, removed_coverage) = collect_rows(removed, &self.config)?;
        let (observed_source, observed_coverage, version) = next
            .checked_read(|next| self.observe_source(next))
            .map_err(TextError::source_read)??;
        let subjects = additions
            .iter()
            .chain(&removals)
            .map(|row| &row.subject)
            .collect();
        let rows = collect_subject_rows(next, &self.config, &subjects)?;
        let source = validate_delta(
            &rows,
            &additions,
            &removals,
            self.source_digest,
            observed_source,
        )?;
        let coverage = self.coverage.with_delta(
            &self.config,
            &added_coverage,
            &removed_coverage,
            &observed_coverage,
        )?;
        let source_fingerprint = source.fingerprint::<P>();
        if source_fingerprint == self.source_fingerprint {
            self.coverage = coverage;
            self.source_version = version;
            finish_read(next)?;
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
            source,
            coverage.clone(),
            self.ranking.clone(),
        )?;
        self.merge_changed(partial, &changed)?;
        self.source_fingerprint = source_fingerprint;
        self.source_digest = source;
        self.coverage = coverage;
        self.source_version = version;
        self.fingerprint = self.compute_fingerprint();
        if let Some(surface) = &mut self.surface {
            surface.bind_generation(self.fingerprint);
        }
        finish_read(next)?;
        Ok(self)
    }

    /// Obtain next-source evidence independently of the requested additions and
    /// removals. Neither branch enumerates the complete next source.
    fn observe_source<D: DatasetView>(&self, next: &D) -> Result<ObservedSource, TextDeltaError>
    where
        D::ReadError: purrdf_hash::error::StructuredError,
    {
        if P::GRAPH_FREE && next.supports_named_graphs() {
            return Err(TextError::config(
                "the graph-free text profile requires a graph-free source",
            )
            .into());
        }
        if let Some(previous) = &self.source_version {
            let mut digest = self.source_digest;
            let mut counts = self.coverage.represented.clone();
            let observed = next
                .visit_source_changes(previous, |side, family, row| {
                    if family != SourceFamily::Statements
                        || !selected_graph::<P>(&self.config, row.graph)
                    {
                        return ControlFlow::Continue(());
                    }
                    let Ok(ordinal) = self.config.predicates().binary_search(row.predicate) else {
                        return ControlFlow::Continue(());
                    };
                    let count = match side {
                        SourceSide::Previous => counts[ordinal].checked_sub(1),
                        SourceSide::Next => counts[ordinal].checked_add(1),
                    };
                    let Some(count) = count else {
                        return ControlFlow::Break(TextError::data(
                            "observed source coverage population is inconsistent",
                        ));
                    };
                    counts[ordinal] = count;
                    match digest.observe::<P>(side, row) {
                        Ok(()) => ControlFlow::Continue(()),
                        Err(error) => ControlFlow::Break(error),
                    }
                })
                .map_err(TextError::source_read)?;
            match observed {
                SourceVisit::Complete { version, .. } => {
                    let coverage = SourceCoverage::of_counts(
                        version.statement_count() != 0,
                        &self.config,
                        counts,
                    );
                    return Ok((digest, coverage, Some(version)));
                }
                SourceVisit::Stopped(error) => return Err(error.into()),
                SourceVisit::Unavailable => {}
            }
        }
        let count = next
            .source_statement_count()
            .map_err(TextError::source_read)?
            .ok_or(TextDeltaError::SourceObservationUnavailable)?;
        let mut digest = Multiset256::default();
        let mut counts = vec![0_u128; self.config.predicates().len()];
        if let Some(graph) =
            P::resolve(self.config.graph(), next).map_err(TextError::source_read)?
        {
            for (ordinal, predicate) in self.config.predicates().iter().enumerate() {
                let Some(predicate) = next
                    .term_id_by_value(predicate)
                    .map_err(TextError::source_read)?
                else {
                    continue;
                };
                for stream in [StatisticsStream::Asserted, StatisticsStream::Annotations] {
                    let summary = next
                        .source_summary(predicate, graph, stream)
                        .map_err(TextError::source_read)?
                        .ok_or(TextDeltaError::SourceObservationUnavailable)?;
                    digest
                        .combine(summary.literal_rows()[usize::from(P::GRAPH_FREE)])
                        .map_err(|error| TextError::data(error.to_string()))?;
                    counts[ordinal] = counts[ordinal]
                        .checked_add(summary.statement_count())
                        .ok_or_else(|| TextError::data("source coverage population overflow"))?;
                }
            }
        }
        Ok((
            SourceDigest::from_state(digest),
            SourceCoverage::of_counts(count != 0, &self.config, counts),
            None,
        ))
    }

    fn merge_changed(
        &mut self,
        partial: Self,
        changed: &BTreeSet<DocumentIdentity<'_, P>>,
    ) -> Result<(), TextDeltaError> {
        let mut removed = Vec::new();
        for key in changed {
            if let Ok(at) = self.documents.binary_search_by(|document| {
                (&document.graph, &document.subject, &document.language).cmp(key)
            }) {
                removed.push((
                    at,
                    self.documents[at].clone(),
                    self.token_streams[at].clone(),
                    self.field_lengths[at].clone(),
                ));
            }
        }
        removed.sort_by_key(|(at, _, _, _)| *at);
        for (at, document, _, fields) in removed.iter().rev() {
            self.adjust_partition(document, fields, false)?;
            self.edit_document_orders(document, false)?;
            self.documents = self.documents.remove(*at)?;
            self.token_streams = self.token_streams.remove(*at)?;
            self.field_lengths = self.field_lengths.remove(*at)?;
            if let Some(table) = &mut self.han_occurrences {
                *table = table.remove(*at)?;
            }
        }
        let mut affected: std::collections::BTreeMap<&str, Vec<&super::Shared<Document<P>>>> =
            std::collections::BTreeMap::new();
        for (_, document, tokens, _) in &removed {
            let words: BTreeSet<_> = tokens
                .as_slice()
                .iter()
                .map(|token| token.term.as_str())
                .collect();
            for word in words {
                affected.entry(word).or_default().push(document);
            }
        }
        for entry in partial.terms.iter() {
            affected.entry(entry.term.as_str()).or_default();
        }
        for (word, old_documents) in affected {
            let position = self
                .terms
                .binary_search_by(|entry| entry.term.as_str().cmp(word));
            let prior = position.ok().map(|at| self.terms[at].clone());
            let mut postings = prior
                .as_ref()
                .map_or_else(super::Table::default, |entry| entry.postings.clone());
            for document in old_documents {
                let at = postings
                    .binary_search_by(|posting| {
                        super::compare_posting_documents(&posting.document, document)
                    })
                    .expect("a removed token has its original posting");
                postings = postings.remove(at)?;
            }
            let added = partial.term_entry(word);
            if let Some(entry) = added {
                for posting in entry.postings.iter() {
                    let at = postings
                        .binary_search_by(|candidate| {
                            super::compare_posting_documents(&candidate.document, &posting.document)
                        })
                        .expect_err("changed documents were removed before insertion");
                    postings = postings.insert(at, posting.clone())?;
                }
            }
            if postings.is_empty() {
                if let Ok(at) = position {
                    self.terms = self.terms.remove(at)?;
                }
                continue;
            }
            let word = prior.as_ref().map_or_else(
                || {
                    added
                        .expect("a new dictionary entry has its analyzed term")
                        .term
                        .clone()
                },
                |entry| entry.term.clone(),
            );
            let entry = super::share_index(super::TermEntry {
                term: word,
                postings,
            })?;
            self.terms = match position {
                Ok(at) => self.terms.replace(at, entry)?,
                Err(at) => {
                    crate::error::term_ordinal(self.terms.len())?;
                    self.terms.insert(at, entry)?
                }
            };
        }
        for (ordinal, ((document, stream), fields)) in partial
            .documents
            .iter()
            .zip(partial.token_streams.iter())
            .zip(partial.field_lengths.iter())
            .enumerate()
        {
            let at = self
                .documents
                .binary_search_by(|candidate| super::compare_document_keys(candidate, document))
                .expect_err("changed documents were removed before insertion");
            super::check_document_count(
                self.documents
                    .len()
                    .checked_add(1)
                    .ok_or_else(crate::query_workspace::overflow)?,
            )?;
            self.documents = self.documents.insert(at, document.clone())?;
            self.edit_document_orders(document, true)?;
            self.field_lengths = self.field_lengths.insert(at, fields.clone())?;
            self.adjust_partition(document, fields, true)?;
            let tokens = stream
                .as_slice()
                .iter()
                .map(|token| StoredToken {
                    term: self
                        .term_entry(token.term.as_str())
                        .expect("a new token has its term")
                        .term
                        .clone(),
                    position: token.position,
                    predicate: token.predicate,
                    ordinal: token.ordinal,
                })
                .collect();
            self.token_streams = self.token_streams.insert(at, share_buffer(tokens)?)?;
            if let Some(table) = &mut self.han_occurrences {
                let buffer = &partial
                    .han_occurrences
                    .as_ref()
                    .expect("partial index uses the same character projection")[ordinal];
                *table = table.insert(at, buffer.clone())?;
            }
        }
        self.surface = match (self.surface.take(), partial.surface) {
            (Some(retained), Some(added)) => Some(retained.with_index_delta(
                added,
                removed.iter().map(|(_, document, _, _)| document.clone()),
                self.documents.clone(),
            )?),
            (None, None) => None,
            _ => unreachable!("partial index uses the same surface configuration"),
        };
        let partitions: BTreeSet<_> = removed
            .iter()
            .map(|(_, document, _, _)| (&document.graph, &document.language))
            .chain(
                partial
                    .documents
                    .iter()
                    .map(|document| (&document.graph, &document.language)),
            )
            .collect();
        for key in partitions {
            if let Ok(at) = self
                .partitions
                .binary_search_by(|row| (&row.key.graph, &row.key.language).cmp(&key))
            {
                self.prepared_corpus(&self.partitions[at].key)?;
            }
        }
        Ok(())
    }
}

type ObservedSource = (SourceDigest, SourceCoverage, Option<SourceVersion>);

fn finish_read<D: DatasetView>(source: &D) -> Result<(), TextError>
where
    D::ReadError: purrdf_hash::error::StructuredError,
{
    source.checked_read(|_| ()).map_err(TextError::source_read)
}

fn selected_graph<P: CorpusProfile>(
    config: &super::TextIndexConfig<P>,
    graph: Option<&purrdf_core::TermValue>,
) -> bool {
    match P::selector(config.graph()) {
        None | Some(GraphSelector::Any) => true,
        Some(GraphSelector::Default) => graph.is_none(),
        Some(GraphSelector::Named(name)) => graph == Some(name),
    }
}

impl SourceCoverage {
    /// Apply the supplied statement counts, including nonliteral changes. The
    /// observed counts belong to the independent next-source read, so an omitted
    /// coverage change cannot validate itself. Reuse the same independently
    /// observed nonempty bit, including the separate reifier layer.
    fn with_delta<P: CorpusProfile>(
        &self,
        config: &super::TextIndexConfig<P>,
        added: &Self,
        removed: &Self,
        observed: &Self,
    ) -> Result<Self, TextDeltaError> {
        let mut counts = Vec::with_capacity(config.predicates().len());
        for (ordinal, predicate) in config.predicates().iter().enumerate() {
            let before = self.represented[ordinal];
            let added = added.represented[ordinal];
            let removed = removed.represented[ordinal];
            let actual = observed.represented[ordinal];
            let Some(expected) = before
                .checked_add(added)
                .and_then(|n| n.checked_sub(removed))
                .filter(|&expected| expected == actual)
            else {
                return Err(TextDeltaError::CoverageMismatch {
                    predicate: Box::new(predicate.clone()),
                    before,
                    added,
                    removed,
                    observed: actual,
                });
            };
            counts.push(expected);
        }
        Ok(Self::of_counts(
            observed.dataset_holds_statements,
            config,
            counts,
        ))
    }
}

type DocumentIdentity<'a, P> = (
    &'a <P as CorpusProfile>::Graph,
    &'a purrdf_core::TermValue,
    &'a Option<String>,
);

/// Check added-row membership against the independently read next source, then
/// reverse its multiset summary without corpus-sized temporary reference tables.
fn validate_delta<P: CorpusProfile>(
    rows: &[SourceRow<P>],
    added: &[SourceRow<P>],
    removed: &[SourceRow<P>],
    mut expected: SourceDigest,
    mut prior: SourceDigest,
) -> Result<SourceDigest, TextDeltaError> {
    for row in join_by(rows.iter().enumerate(), added.iter().enumerate(), |a, b| {
        a.1.cmp(b.1)
    }) {
        match row {
            Joined::Left(_) | Joined::Both(_, _) => {}
            Joined::Right((ordinal, _)) => return Err(TextDeltaError::MissingAddition { ordinal }),
        }
    }
    for row in added {
        prior.remove(row);
    }
    for row in removed {
        prior.insert(row);
    }
    if expected != prior {
        return Err(TextDeltaError::SourceMismatch {
            expected: expected.fingerprint::<P>(),
            actual: prior.fingerprint::<P>(),
        });
    }
    for row in removed {
        expected.remove(row);
    }
    for row in added {
        expected.insert(row);
    }
    Ok(expected)
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
        assert_eq!(actual.source_digest, fresh.source_digest);
        assert_eq!(actual.source_fingerprint, fresh.source_fingerprint);
        assert_eq!(actual.fingerprint, fresh.fingerprint);
        assert_eq!(actual.coverage, fresh.coverage);
        assert_eq!(
            actual.documents().collect::<Vec<_>>(),
            fresh.documents().collect::<Vec<_>>()
        );
        assert_eq!(
            actual.partitions().collect::<Vec<_>>(),
            fresh.partitions().collect::<Vec<_>>()
        );
        assert_eq!(
            actual.terms().collect::<Vec<_>>(),
            fresh.terms().collect::<Vec<_>>()
        );
        for (a, b) in actual.terms.iter().zip(fresh.terms.iter()) {
            let facts = |posting: &super::super::Shared<super::super::Posting<P>>| {
                (
                    posting.document.subject.clone(),
                    posting.document.graph.clone(),
                    posting.document.language.clone(),
                    posting.positions.clone(),
                    posting.predicate_frequencies.clone(),
                )
            };
            assert_eq!(
                a.postings.iter().map(facts).collect::<Vec<_>>(),
                b.postings.iter().map(facts).collect::<Vec<_>>()
            );
        }
        for document in 0..actual.documents.len() as u32 {
            assert!(
                actual
                    .token_occurrences(document)
                    .unwrap()
                    .eq(fresh.token_occurrences(document).unwrap())
            );
        }
        for index in [actual, fresh] {
            for stream in index.token_streams.iter() {
                for token in stream.as_slice() {
                    let entry = index.term_entry(token.term.as_str()).unwrap();
                    assert!(core::ptr::eq(
                        &raw const *token.term,
                        &raw const *entry.term
                    ));
                }
            }
        }
        assert_eq!(
            actual
                .field_lengths
                .iter()
                .map(|v| v.as_slice())
                .collect::<Vec<_>>(),
            fresh
                .field_lengths
                .iter()
                .map(|v| v.as_slice())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            actual
                .subject_order
                .iter()
                .map(|d| &**d)
                .collect::<Vec<_>>(),
            fresh.subject_order.iter().map(|d| &**d).collect::<Vec<_>>()
        );
        assert_eq!(
            actual
                .occurrence_order
                .iter()
                .map(|d| &**d)
                .collect::<Vec<_>>(),
            fresh
                .occurrence_order
                .iter()
                .map(|d| &**d)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            actual.han_occurrences.is_some(),
            fresh.han_occurrences.is_some()
        );
        for document in 0..actual.document_count() as u32 {
            assert_eq!(
                actual.han_occurrences(document).collect::<Vec<_>>(),
                fresh.han_occurrences(document).collect::<Vec<_>>()
            );
        }
        for (partition, _) in fresh.partitions() {
            assert_eq!(
                actual.field_totals(partition),
                fresh.field_totals(partition)
            );
            assert_eq!(
                actual.field_populations(partition),
                fresh.field_populations(partition)
            );
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
    fn segmented_subject_collection_reads_the_changed_documents_only() {
        use purrdf_core::{FallibleDatasetView, ViewOperationStatus};
        let requests = |query: &purrdf_core::SegmentedStackQuery<'_>| match query.operation_status()
        {
            ViewOperationStatus::Ready { evidence } => evidence.request_count(),
            ViewOperationStatus::Failed { error, .. } => panic!("source read failed: {error}"),
        };
        let blocks = |query: &purrdf_core::SegmentedStackQuery<'_>| match query.operation_status() {
            ViewOperationStatus::Ready { evidence } => evidence.with_requests(|requests| {
                requests
                    .iter()
                    .filter(|request| request.bytes() == 512)
                    .map(purrdf_core::SegmentedRequest::position)
                    .collect::<BTreeSet<_>>()
                    .len()
            }),
            ViewOperationStatus::Failed { error, .. } => panic!("source read failed: {error}"),
        };
        let mut observations = Vec::new();
        for count in [128_u32, 2048] {
            let limits = purrdf_core::SegmentedBuildLimits::new(
                count * 8 + 32,
                count * 512,
                count * 3,
                512,
                8,
            )
            .unwrap();
            let mut builder = purrdf_core::SegmentedBuilder::new(limits).unwrap();
            let triple = TermValue::Triple {
                s: purrdf_core::TermBox::new(TermValue::iri("urn:base")),
                p: purrdf_core::TermBox::new(TermValue::iri("urn:relation")),
                o: purrdf_core::TermBox::new(TermValue::iri("urn:target")),
            };
            let mut quoted = Vec::new();
            builder
                .intern_batch(&[triple], |_, id| quoted.push(id))
                .unwrap();
            for ordinal in 0..count {
                let values = [
                    TermValue::iri(format!("urn:document:{ordinal:06}")),
                    TermValue::iri("urn:text"),
                    TermValue::simple_literal("river bank"),
                    TermValue::simple_literal("valley meadow"),
                ];
                let mut ids = Vec::new();
                builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
                builder
                    .push_quad(purrdf_core::QuadIds {
                        s: ids[0],
                        p: ids[1],
                        o: ids[2],
                        g: None,
                    })
                    .unwrap();
                builder.push_reifier(ids[0], quoted[0], None).unwrap();
                builder
                    .push_annotation(purrdf_core::QuadIds {
                        s: ids[0],
                        p: ids[1],
                        o: ids[3],
                        g: None,
                    })
                    .unwrap();
            }
            let image = builder.seal().unwrap();
            let layer = purrdf_core::SegmentedLayer::new(
                Arc::new(image.provider()),
                image.receipt().clone(),
                purrdf_core::SegmentedLayerMetadata::default(),
            )
            .unwrap();
            let read_limits =
                purrdf_core::SegmentedReadLimits::new(128_000_000, 8, 200_000, 128_000_000, 16);
            let mut stack = purrdf_core::SegmentedStack::default();
            stack.publish(vec![Arc::new(layer)], read_limits).unwrap();
            let snapshot = stack.snapshot().unwrap();
            let selected = TermValue::iri("urn:document:000001");
            let subjects = std::iter::once(&selected).collect();
            let query = snapshot.query(read_limits).unwrap();
            let before = requests(&query);
            let rows = collect_subject_rows(&query, &config(), &subjects).unwrap();
            let point_reads = requests(&query) - before;
            let point_blocks = blocks(&query);
            assert_eq!(rows.len(), 2);

            // Use a distinct operation with a cold cache for the independent
            // whole-source oracle. No warmed pages are credited to either path.
            // Profile 7's larger authenticated summary directory exceeds the
            // old full-scan request allowance at 2,048 documents. Admit that
            // reference scan without changing the subject lookup's limit.
            let full_query = snapshot
                .query(purrdf_core::SegmentedReadLimits::new(
                    128_000_000,
                    8,
                    400_000,
                    128_000_000,
                    16,
                ))
                .unwrap();
            let before = requests(&full_query);
            let (all_rows, _) = collect_rows(&full_query, &config()).unwrap();
            let full_reads = requests(&full_query) - before;
            let full_blocks = blocks(&full_query);
            assert_eq!(all_rows.len(), count as usize * 2);
            let expected: Vec<_> = all_rows
                .into_iter()
                .filter(|row| row.subject == selected)
                .collect();
            assert_eq!(rows, expected);
            observations.push((count, point_reads, full_reads, point_blocks, full_blocks));
        }
        eprintln!(
            "segmented subject collection (documents, point reads, full reads, point blocks, full blocks): {observations:?}"
        );
        // Each cache miss performs a complete Merkle proof; its request count
        // changes at proof-depth and tiny-cache thresholds. Keep those exact
        // reads visible while checking the distinct source blocks independently.
        assert!(observations[1].3 <= observations[0].3 * 3);
        assert!(observations[1].4 >= observations[0].4 * 4);
        assert!(observations[1].1 * 4 < observations[1].2);
    }

    #[test]
    fn text_delta_replaces_inserts_retracts_and_preserves_untouched_buffers() {
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
            let buffer = index.token_streams[retained].as_slice().as_ptr();
            assert_eq!(snapshot.token_streams[retained].as_slice().as_ptr(), buffer);
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
            assert_eq!(updated.token_streams[retained].as_slice().as_ptr(), buffer);
            assert_fresh(&snapshot, &TextIndex::from_dataset(&*old, &config).unwrap());
        }
    }

    #[test]
    fn index_delta_and_snapshot_clone_preserve_retained_entries_with_logarithmic_edit_work() {
        use super::super::token_streams::{HANDLE_CLONES, NODE_BUILDS};
        let mut measurements = Vec::new();
        let added_row = ("urn:000000", "aardvark bank", None, None, false);
        let build = |count, extra| {
            let mut builder = RdfDatasetBuilder::new();
            let predicate = builder.intern_iri("urn:text").unwrap();
            let object = builder
                .intern_literal(RdfLiteral::simple("bank river"))
                .unwrap();
            for at in 0..count {
                let subject = builder.intern_iri(&format!("urn:doc:{at:06}")).unwrap();
                builder.push_quad(subject, predicate, object, None).unwrap();
            }
            if extra {
                let subject = builder.intern_iri(added_row.0).unwrap();
                let object = builder
                    .intern_literal(RdfLiteral::simple(added_row.1))
                    .unwrap();
                builder.push_quad(subject, predicate, object, None).unwrap();
            }
            builder.freeze().unwrap()
        };
        for surface in [false, true] {
            for count in [128, 4096, 16_384] {
                let config = config().with_surface_index(surface);
                let old_source = build(count, false);
                let next_source = build(count, true);
                let added = dataset(&[added_row]);
                let empty = dataset(&[]);
                let index = TextIndex::from_dataset(&*old_source, &config).unwrap();
                let height = index.documents.height();
                NODE_BUILDS.set(0);
                HANDLE_CLONES.set(0);
                let old = index.clone();
                assert_eq!((NODE_BUILDS.get(), HANDLE_CLONES.get()), (0, 0));
                let updated = index.with_delta(&*next_source, &*added, &*empty).unwrap();
                let nodes = NODE_BUILDS.get();
                let copies = HANDLE_CLONES.get();
                assert!(
                    nodes <= 1024 * usize::from(surface) + 32 * (usize::from(height) + 1),
                    "whole-table rebuilding: {nodes}"
                );
                assert!(
                    copies <= 1024 * usize::from(surface) + 32 * 32 * (usize::from(height) + 1)
                );
                assert!(core::ptr::eq(
                    old.document(count - 1).unwrap(),
                    updated.document(count).unwrap()
                ));
                let old_bank = old.term_entry("bank").unwrap();
                let next_bank = updated.term_entry("bank").unwrap();
                assert!(core::ptr::eq(
                    &raw const *old_bank.postings[count as usize - 1],
                    &raw const *next_bank.postings[count as usize]
                ));
                assert!(core::ptr::eq(
                    old.term_entry("river").unwrap(),
                    updated.term_entry("river").unwrap()
                ));
                assert_fresh(
                    &updated,
                    &TextIndex::from_dataset(&*next_source, &config).unwrap(),
                );
                measurements.push((surface, count, height, nodes, copies));
            }
        }
        for group in measurements.as_chunks::<3>().0 {
            assert!(
                group[2].3 <= group[0].3 + 64 * usize::from(group[2].2 - group[0].2),
                "fixed edits must not rebuild a growing corpus"
            );
            assert!(
                group[2].4 <= group[0].4 + 64 * usize::from(group[2].2 - group[0].2),
                "fixed edits must not copy growing retained tables"
            );
        }
        eprintln!(
            "index delta (surface enabled, retained documents, height, new table nodes, copied leaf values): {measurements:?}"
        );
    }

    #[test]
    fn index_identity_work_ignores_corpus_size_but_binds_original_content() {
        let mut work = Vec::new();
        for count in [1, 512] {
            let mut builder = RdfDatasetBuilder::new();
            let predicate = builder.intern_iri("urn:text").unwrap();
            for ordinal in 0..count {
                let subject = builder
                    .intern_iri(&format!("urn:document:{ordinal}"))
                    .unwrap();
                let literal = builder
                    .intern_literal(RdfLiteral::simple(format!("river bank word{ordinal}")))
                    .unwrap();
                builder
                    .push_quad(subject, predicate, literal, None)
                    .unwrap();
            }
            let source = builder.freeze().unwrap();
            let index = TextIndex::from_dataset(&*source, &config()).unwrap();
            let before = super::super::DIGEST_BYTES.get();
            assert_eq!(index.compute_fingerprint(), index.fingerprint());
            work.push(super::super::DIGEST_BYTES.get() - before);
        }
        assert!(work[0] > 0);
        assert_eq!(
            work[0], work[1],
            "the identity encoder must not visit corpus tables"
        );

        let build = |text| {
            TextIndex::from_dataset(&*dataset(&[("urn:s", text, None, None, false)]), &config())
                .unwrap()
        };
        let first = build("river bank");
        let reordered = build("bank river");
        let original_spelling = build("RIVER bank");
        for other in [&reordered, &original_spelling] {
            assert_eq!(
                first.partitions().collect::<Vec<_>>(),
                other.partitions().collect::<Vec<_>>()
            );
            assert_eq!(
                first.documents().collect::<Vec<_>>(),
                other.documents().collect::<Vec<_>>()
            );
            assert_ne!(first.source_fingerprint(), other.source_fingerprint());
            assert_ne!(first.fingerprint(), other.fingerprint());
        }
        eprintln!("index identity bytes (1, 512 documents): {work:?}");
    }

    #[test]
    fn token_buffers_share_across_rank_changes_and_retired_words_release() {
        let retained = ("urn:z", "zebra", None, None, false);
        let mut changing = ("urn:a", "aardvark", None, None, false);
        let mut index =
            TextIndex::from_dataset(&*dataset(&[changing, retained]), &config()).unwrap();
        let buffer = index.token_streams[1].as_slice().as_ptr();
        for step in 0..128 {
            let retired = index.term_entry(changing.1).unwrap().term.clone();
            let old = index.clone();
            assert_eq!(old.token_streams[1].as_slice().as_ptr(), buffer);
            let removed = dataset(&[changing]);
            changing.1 = if step % 2 == 0 { "zzzz" } else { "aardvark" };
            let next = dataset(&[changing, retained]);
            index = index
                .with_delta(&*next, &*dataset(&[changing]), &*removed)
                .unwrap();
            let fresh = TextIndex::from_dataset(&*next, &config()).unwrap();
            assert_fresh(&index, &fresh);
            assert_eq!(index.token_streams[1].as_slice().as_ptr(), buffer);
            assert_eq!(index.term_count(), 2);
            let before = old.token_occurrences(1).unwrap().next().unwrap();
            let after = index.token_occurrences(1).unwrap().next().unwrap();
            assert_ne!(before.term, after.term, "public canonical rank changes");
            assert_eq!(old.occurrence_term(before.term), Some("zebra"));
            assert_eq!(index.occurrence_term(after.term), Some("zebra"));
            drop(old);
            assert_eq!(
                retired.strong_count(),
                1,
                "no retired vocabulary owner remains"
            );
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
        assert!(index.terms.is_empty());
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
