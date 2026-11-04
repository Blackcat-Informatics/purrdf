// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable caller-owned text snapshots used by the native relations.

use crate::query_workspace::{QueryString, admitted, data_error};
use crate::ranking::{FieldInputs, PreparedCorpus};
use crate::{
    CorpusProfile, Document, PartitionKey, PartitionStats, RankingProfile, SourceCoverage,
    TextError, TextIndex, TextIndexConfig, TokenOccurrence,
};
use purrdf_core::TermValue;
use purrdf_sparql_eval::{AdmittedVec, WorkspaceCapability};
use std::ops::Deref;

/// A pinned source snapshot. Document identifiers and partition ordinals use the
/// native canonical order, independently of the provider's physical addresses.
///
/// Configuration and partition descriptors are immutable source-owned metadata.
/// Storage reads and advances use the supplied query capability. A provider must
/// admit decoded pages and retained guards before allocating them, and preserve
/// their admission until the last reader releases them. A yielded guard remains
/// valid when another cursor advances. Missing keys return `None`; failed reads
/// return the original failure through [`TextError::source_read_owned`].
///
/// Cursor states own their storage and never borrow the source. This lets a
/// relation cursor own both an `Arc` snapshot and its cursor state safely.
pub trait TextSource<P: CorpusProfile>: std::fmt::Debug + Send + Sync + 'static {
    /// A document record, optionally retaining an admitted decoded page.
    type Document<'a>: Deref<Target = Document<P>>
    where
        Self: 'a;
    /// One posting's predicate frequencies, in selected-predicate order.
    type Frequencies<'a>: AsRef<[(u32, u64)]>
    where
        Self: 'a;
    /// Original token spelling, optionally retaining an admitted decoded page.
    type Term<'a>: AsRef<str>
    where
        Self: 'a;
    /// Owned positional walk state; no borrow of the snapshot is retained.
    type Positions: Default + std::fmt::Debug + Send + 'static;

    /// The immutable analysis and selected-predicate configuration.
    fn config(&self) -> &TextIndexConfig<P>;
    /// The immutable native ranking law.
    fn ranking_profile(&self) -> &RankingProfile;
    /// Native content identity of the complete snapshot.
    fn fingerprint(&self) -> [u8; 32];
    /// Exact selected literal-row multiset identity.
    fn source_fingerprint(&self) -> [u8; 32];
    /// Coverage of the configured predicates in this same immutable snapshot.
    fn source_coverage(&self) -> &SourceCoverage;
    /// Canonically ordered partition descriptors, retained by the snapshot.
    fn partitions(&self) -> impl Iterator<Item = (&PartitionKey<P>, &PartitionStats)>;
    /// Direct metadata lookup by canonical partition ordinal.
    fn partition_at(&self, ordinal: usize) -> Option<(&PartitionKey<P>, &PartitionStats)>;
    /// Indexed lookup of an immutable partition key; no corpus metadata scan.
    fn local_partition_ordinal(&self, key: &PartitionKey<P>) -> Option<usize>;
    /// Resolve a caller key through the query's admitted term comparator.
    fn partition_key_owned(
        &self,
        key: &PartitionKey<P>,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<&PartitionKey<P>>, TextError>;
    /// Exact field token totals for a partition.
    fn field_totals(&self, partition: &PartitionKey<P>) -> Option<&[u128]>;
    /// Exact field document populations for a partition.
    fn field_populations(&self, partition: &PartitionKey<P>) -> Option<&[u64]>;
    /// Number of canonical dictionary terms.
    fn term_count(&self) -> u64;
    /// Prepare native relation bounds once, or return the snapshot's cached native measurement.
    fn relation_bounds(
        &self,
        workspace: &WorkspaceCapability,
    ) -> Result<crate::TextSourceBounds, TextError>
    where
        Self: Sized,
    {
        crate::TextSourceBounds::measure(self, workspace)
    }
    /// Number of documents in canonical order.
    fn document_count(&self) -> u64;
    /// Read one canonical document.
    fn read_document(
        &self,
        document: u32,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<Self::Document<'_>>, TextError>;
    /// Ascending canonical document IDs with this subject, using its side index.
    fn documents_with_subject_owned(
        &self,
        subject: &TermValue,
        workspace: &WorkspaceCapability,
    ) -> Result<AdmittedVec<u32>, TextError>;
    /// Resolve a document to a canonical partition ordinal.
    fn partition_ordinal_of_owned(
        &self,
        document: u32,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<usize>, TextError>;
    /// Point membership and retained predicate counts; no posting-list scan.
    fn posted_frequencies_owned(
        &self,
        document: u32,
        term: &str,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<Self::Frequencies<'_>>, TextError>;
    /// Exact document frequency, without reading the posting list.
    fn document_frequency_owned(
        &self,
        partition: &PartitionKey<P>,
        term: &str,
        workspace: &WorkspaceCapability,
    ) -> Result<u64, TextError>;
    /// One ordered posting and its independently retained counts.
    fn field_posting_at(
        &self,
        partition: &PartitionKey<P>,
        term: &str,
        ordinal: usize,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<(u32, Self::Frequencies<'_>)>, TextError>;
    /// Stream ordered postings. Providers may override the ordinal adapter to
    /// retain a seek cursor; yielded guards remain independent of its advance.
    fn field_postings_owned<'a>(
        &'a self,
        key: &'a PartitionKey<P>,
        term: &'a str,
        workspace: &'a WorkspaceCapability,
    ) -> impl Iterator<Item = Result<(u32, Self::Frequencies<'a>), TextError>> + 'a {
        let mut ordinal = 0;
        let mut done = false;
        std::iter::from_fn(move || {
            if done {
                return None;
            }
            match self.field_posting_at(key, term, ordinal, workspace) {
                Ok(Some(row)) => {
                    ordinal += 1;
                    Some(Ok(row))
                }
                Ok(None) => {
                    done = true;
                    None
                }
                Err(error) => {
                    done = true;
                    Some(Err(error))
                }
            }
        })
    }
    /// Advance the positional stream in `(document, position)` order.
    fn next_position(
        &self,
        partition: usize,
        term: &str,
        state: &mut Self::Positions,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<(u32, u32)>, TextError>;
    /// Original token at an ordinal within a document's token stream.
    fn token_at(
        &self,
        document: u32,
        ordinal: usize,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<TokenOccurrence>, TextError>;
    /// Document in `(subject, language, graph)` order for dictionary matching.
    fn occurrence_document_at(
        &self,
        ordinal: usize,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<u32>, TextError>;
    /// Resolve the canonical term dictionary ordinal.
    fn read_term(
        &self,
        term: u32,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<Self::Term<'_>>, TextError>;
}

/// Native derived operations. Providers implement facts, never scoring or analysis.
pub(crate) trait SourceExt<P: CorpusProfile>: TextSource<P> {
    fn required_document(
        &self,
        document: u32,
        workspace: &WorkspaceCapability,
    ) -> Result<Self::Document<'_>, TextError> {
        self.read_document(document, workspace)?
            .ok_or_else(|| data_error("source reference names an absent document", Some(workspace)))
    }
    fn required_term(
        &self,
        term: u32,
        workspace: &WorkspaceCapability,
    ) -> Result<Self::Term<'_>, TextError> {
        self.read_term(term, workspace)?.ok_or_else(|| {
            data_error(
                "source occurrence names an absent dictionary term",
                Some(workspace),
            )
        })
    }
    fn analyzer_fingerprint(&self) -> [u8; 32] {
        self.config().analyzer().fingerprint()
    }
    fn query_terms_owned(
        &self,
        input: &str,
        workspace: &WorkspaceCapability,
    ) -> Result<AdmittedVec<QueryString>, TextError> {
        self.config().query_terms_owned(input, workspace)
    }
    fn partition_stats(&self, key: &PartitionKey<P>) -> Option<&PartitionStats> {
        self.local_partition_ordinal(key)
            .and_then(|ordinal| self.partition_at(ordinal))
            .map(|(_, stats)| stats)
    }
    fn partition_key_of_owned(
        &self,
        document: u32,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<&PartitionKey<P>>, TextError> {
        let Some(ordinal) = self.partition_ordinal_of_owned(document, workspace)? else {
            return Ok(None);
        };
        self.partition_at(ordinal)
            .map(|(key, _)| Some(key))
            .ok_or_else(|| data_error("document names an absent partition", Some(workspace)))
    }
    fn prepared_corpus_owned(
        &self,
        key: &PartitionKey<P>,
        workspace: &WorkspaceCapability,
    ) -> Result<PreparedCorpus<'_>, TextError> {
        let key = self
            .partition_key_owned(key, workspace)?
            .ok_or_else(|| data_error("ranking partition is absent", Some(workspace)))?;
        let stats = self.partition_stats(key).ok_or_else(|| {
            data_error("ranking partition statistics are absent", Some(workspace))
        })?;
        let totals = self
            .field_totals(key)
            .ok_or_else(|| data_error("ranking field totals are absent", Some(workspace)))?;
        let populations = self
            .field_populations(key)
            .ok_or_else(|| data_error("ranking field populations are absent", Some(workspace)))?;
        PreparedCorpus::from_index_owned(
            self.ranking_profile(),
            stats.document_count(),
            totals,
            populations,
            workspace,
        )
    }
    fn field_inputs_from_counts_owned(
        &self,
        document: u32,
        frequencies: &[(u32, u64)],
        workspace: &WorkspaceCapability,
    ) -> Result<FieldInputs, TextError> {
        let document_row = self
            .read_document(document, workspace)?
            .ok_or_else(|| data_error("field input names an absent document", Some(workspace)))?;
        let ranking = self.ranking_profile();
        let mut fields = FieldInputs::new(ranking.fields().len(), workspace)?;
        for (facts, frequency) in [
            (document_row.predicate_lengths(), false),
            (frequencies, true),
        ] {
            for &(predicate, count) in facts {
                let predicate_ordinal = predicate;
                let predicate_term = self
                    .config()
                    .predicates()
                    .get(predicate as usize)
                    .ok_or_else(|| {
                        data_error("posting names an absent predicate", Some(workspace))
                    })?;
                let field = ranking.field_for(predicate_term)?;
                let value = if frequency {
                    &mut fields[field].term_frequency
                } else {
                    &mut fields[field].length
                };
                *value = value.checked_add(count).ok_or_else(|| {
                    let cause = if frequency {
                        crate::arithmetic::ArithmeticRefusal::FieldFrequency {
                            document,
                            predicate: predicate_ordinal,
                            current: *value,
                            added: count,
                        }
                    } else {
                        crate::arithmetic::ArithmeticRefusal::FieldLength {
                            field,
                            current: *value,
                            added: count,
                        }
                    };
                    crate::query_workspace::arithmetic_error(
                        cause,
                        workspace.is_bounded().then_some(workspace),
                    )
                })?;
            }
        }
        Ok(fields)
    }
    fn term_frequency_in_predicate_owned(
        &self,
        document: u32,
        term: &str,
        predicate: &TermValue,
        workspace: &WorkspaceCapability,
    ) -> Result<u64, TextError> {
        let ordinal = self
            .config()
            .predicates()
            .binary_search(predicate)
            .map_err(|_| {
                crate::configuration::ConfigurationRefusal::ScopePredicate {
                    predicate: predicate.clone(),
                }
                .into_error()
            })? as u32;
        Ok(self
            .posted_frequencies_owned(document, term, workspace)?
            .map_or(0, |counts| {
                crate::index::predicate_frequency(counts.as_ref(), ordinal)
            }))
    }
}
impl<P: CorpusProfile, S: TextSource<P>> SourceExt<P> for S {}

/// The positional alias bound's single saturating recurrence.
pub(crate) fn accumulate_phrase(bound: &mut u64, suffix: &mut u64, count: u64, contiguous: bool) {
    if !contiguous {
        *suffix = 1;
    }
    *suffix = suffix.saturating_mul(count);
    *bound = bound.saturating_add(*suffix);
}

pub(crate) fn phrase_bound<P: CorpusProfile, S: TextSource<P>>(
    source: &S,
    workspace: &WorkspaceCapability,
) -> Result<u64, TextError> {
    fn finish(positions: &mut AdmittedVec<u32>) -> u64 {
        positions.as_mut_slice().sort_unstable();
        let mut at = positions.len();
        let mut next = None;
        let mut suffix = 1_u64;
        let mut bound = 0_u64;
        while at != 0 {
            let position = positions[at - 1];
            let end = at;
            while at != 0 && positions[at - 1] == position {
                at -= 1;
            }
            accumulate_phrase(
                &mut bound,
                &mut suffix,
                (end - at) as u64,
                position.checked_add(1) == next,
            );
            next = Some(position);
        }
        positions.truncate(0);
        bound
    }
    let mut bound = 0_u64;
    let mut positions = AdmittedVec::new(workspace);
    let mut previous: Option<S::Document<'_>> = None;
    for ordinal in 0..source.document_count() {
        let ordinal = usize::try_from(ordinal).map_err(|_| crate::query_workspace::overflow())?;
        let document = source
            .occurrence_document_at(ordinal, workspace)?
            .ok_or_else(|| {
                data_error(
                    "source occurrence order contains a missing document",
                    Some(workspace),
                )
            })?;
        let row = source.read_document(document, workspace)?.ok_or_else(|| {
            data_error(
                "source occurrence order names an absent document",
                Some(workspace),
            )
        })?;
        if let Some(ref prior) = previous
            && (prior.language() != row.language()
                || !admitted(workspace.terms_equal(prior.subject(), row.subject()))?)
        {
            bound = bound.saturating_add(finish(&mut positions));
        }
        let mut token = 0;
        while let Some(occurrence) = source.token_at(document, token, workspace)? {
            admitted(positions.push(occurrence.position))?;
            token += 1;
        }
        previous = Some(row);
    }
    Ok(bound.saturating_add(finish(&mut positions)))
}

/// The resident index's owned positional cursor state.
#[derive(Default, Debug)]
pub struct IndexPositions {
    posting: usize,
    position: usize,
}

impl<P: CorpusProfile> TextSource<P> for TextIndex<P> {
    type Document<'a> = &'a Document<P>;
    type Frequencies<'a> = &'a [(u32, u64)];
    type Term<'a> = &'a str;
    type Positions = IndexPositions;

    fn config(&self) -> &TextIndexConfig<P> {
        self.config()
    }
    fn ranking_profile(&self) -> &RankingProfile {
        self.ranking_profile()
    }
    fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint()
    }
    fn source_fingerprint(&self) -> [u8; 32] {
        self.source_fingerprint()
    }
    fn source_coverage(&self) -> &SourceCoverage {
        self.source_coverage()
    }
    fn partitions(&self) -> impl Iterator<Item = (&PartitionKey<P>, &PartitionStats)> {
        self.partitions()
    }
    fn partition_at(&self, ordinal: usize) -> Option<(&PartitionKey<P>, &PartitionStats)> {
        self.partition_at(ordinal)
    }
    fn local_partition_ordinal(&self, key: &PartitionKey<P>) -> Option<usize> {
        self.partition_index(key).map(|ordinal| ordinal as usize)
    }
    fn partition_key_owned(
        &self,
        key: &PartitionKey<P>,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<&PartitionKey<P>>, TextError> {
        admitted(self.partition_key_owned(key, workspace))
    }
    fn field_totals(&self, key: &PartitionKey<P>) -> Option<&[u128]> {
        self.field_totals(key)
    }
    fn field_populations(&self, key: &PartitionKey<P>) -> Option<&[u64]> {
        self.field_populations(key)
    }
    fn document_count(&self) -> u64 {
        self.document_count()
    }
    fn term_count(&self) -> u64 {
        self.term_count()
    }
    fn read_document(
        &self,
        document: u32,
        _: &WorkspaceCapability,
    ) -> Result<Option<&Document<P>>, TextError> {
        Ok(self.document(document))
    }
    fn documents_with_subject_owned(
        &self,
        subject: &TermValue,
        workspace: &WorkspaceCapability,
    ) -> Result<AdmittedVec<u32>, TextError> {
        admitted(self.documents_with_subject_owned(subject, workspace))
    }
    fn partition_ordinal_of_owned(
        &self,
        document: u32,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<usize>, TextError> {
        admitted(self.partition_ordinal_of_owned(document, workspace))
    }
    fn posted_frequencies_owned(
        &self,
        document: u32,
        term: &str,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<&[(u32, u64)]>, TextError> {
        admitted(self.posted_frequencies_owned(document, term, workspace))
    }
    fn document_frequency_owned(
        &self,
        key: &PartitionKey<P>,
        term: &str,
        workspace: &WorkspaceCapability,
    ) -> Result<u64, TextError> {
        admitted(self.document_frequency_owned(key, term, workspace))
    }
    fn field_posting_at(
        &self,
        key: &PartitionKey<P>,
        term: &str,
        ordinal: usize,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<(u32, &[(u32, u64)])>, TextError> {
        admitted(self.field_posting_at_owned(key, term, ordinal, workspace))
    }
    fn field_postings_owned<'a>(
        &'a self,
        key: &'a PartitionKey<P>,
        term: &'a str,
        workspace: &'a WorkspaceCapability,
    ) -> impl Iterator<Item = Result<(u32, Self::Frequencies<'a>), TextError>> + 'a {
        let mut opened = Some(self.field_postings_owned(key, term, workspace));
        let mut run = None;
        std::iter::from_fn(move || {
            if let Some(opened) = opened.take() {
                match opened {
                    Ok(iterator) => run = Some(iterator),
                    Err(error) => return Some(Err(error)),
                }
            }
            run.as_mut().and_then(Iterator::next)
        })
    }
    fn next_position(
        &self,
        partition: usize,
        term: &str,
        state: &mut IndexPositions,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<(u32, u32)>, TextError> {
        loop {
            let Some((document, positions)) =
                admitted(self.posting_at_owned(partition, term, state.posting, workspace))?
            else {
                return Ok(None);
            };
            if let Some(&position) = positions.get(state.position) {
                state.position += 1;
                return Ok(Some((document, position)));
            }
            state.posting += 1;
            state.position = 0;
        }
    }
    fn token_at(
        &self,
        document: u32,
        ordinal: usize,
        _: &WorkspaceCapability,
    ) -> Result<Option<TokenOccurrence>, TextError> {
        Ok(self.token_occurrence_at(document, ordinal))
    }
    fn occurrence_document_at(
        &self,
        ordinal: usize,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<u32>, TextError> {
        admitted(self.occurrence_order().get_owned(ordinal, workspace))
    }
    fn read_term(&self, term: u32, _: &WorkspaceCapability) -> Result<Option<&str>, TextError> {
        Ok(self.occurrence_term(term))
    }
}
