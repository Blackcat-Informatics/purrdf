// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A column-oriented provider with admitted, independently owned posting pages.

use purrdf_core::TermValue;
use purrdf_sparql_eval::{AdmittedVec, WorkspaceAllocation, WorkspaceCapability};
use purrdf_text::{
    Document, GraphLanguage, PartitionKey, PartitionStats, RankingProfile, TextError, TextIndex,
    TextIndexConfig, TextSource, TextSourceBounds, TokenOccurrence,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Debug)]
pub(crate) struct Counts {
    values: AdmittedVec<(u32, u64)>,
    _page: WorkspaceAllocation,
}
impl AsRef<[(u32, u64)]> for Counts {
    fn as_ref(&self) -> &[(u32, u64)] {
        &self.values
    }
}

#[derive(Debug, Default)]
pub(crate) struct Positions {
    posting: usize,
    position: usize,
}

#[derive(Debug)]
pub(crate) struct Columns {
    config: TextIndexConfig,
    ranking: RankingProfile,
    fingerprint: [u8; 32],
    source_fingerprint: [u8; 32],
    coverage: purrdf_text::SourceCoverage,
    partitions: Vec<(PartitionKey, PartitionStats, Vec<u128>, Vec<u64>)>,
    documents: BTreeMap<u32, Document>,
    subjects: BTreeMap<TermValue, Vec<u32>>,
    document_partitions: Vec<usize>,
    terms: Vec<String>,
    counts: BTreeMap<String, BTreeMap<u32, Vec<(u32, u64)>>>,
    positions: BTreeMap<(String, u32), Vec<u32>>,
    tokens: BTreeMap<u32, Vec<TokenOccurrence>>,
    occurrence_order: Vec<u32>,
    bounds: OnceLock<TextSourceBounds>,
    pub fail_reads: AtomicBool,
    pub page_bytes: AtomicU64,
    pub observed_workspace: Mutex<Option<WorkspaceCapability>>,
}

impl Columns {
    /// Import fixture facts once. No TextIndex is retained or consulted by reads.
    pub(crate) fn freeze(index: &TextIndex) -> Self {
        let workspace = WorkspaceCapability::resident();
        let mut columns = Self {
            config: index.config().clone(),
            source_fingerprint: index.source_fingerprint(),
            coverage: index.source_coverage().clone(),
            ranking: index.ranking_profile().clone(),
            fingerprint: purrdf_text::snapshot_fingerprint(
                index.config(),
                index.ranking_profile(),
                index.source_fingerprint(),
                purrdf_text::unicode_versions(),
            ),
            partitions: index
                .partitions()
                .map(|(key, stats)| {
                    (
                        key.clone(),
                        *stats,
                        index.field_totals(key).unwrap().to_vec(),
                        index.field_populations(key).unwrap().to_vec(),
                    )
                })
                .collect(),
            documents: index
                .documents()
                .enumerate()
                .map(|(id, row)| (u32::try_from(id).unwrap(), row.clone()))
                .collect(),
            subjects: BTreeMap::new(),
            document_partitions: Vec::new(),
            terms: index.terms().map(str::to_owned).collect(),
            counts: BTreeMap::new(),
            positions: BTreeMap::new(),
            tokens: BTreeMap::new(),
            occurrence_order: Vec::new(),
            bounds: OnceLock::new(),
            fail_reads: AtomicBool::new(false),
            page_bytes: AtomicU64::new(0),
            observed_workspace: Mutex::new(None),
        };
        for (&id, document) in &columns.documents {
            columns
                .subjects
                .entry(document.subject().clone())
                .or_default()
                .push(id);
            columns.document_partitions.push(
                TextSource::partition_ordinal_of_owned(index, id, &workspace)
                    .unwrap()
                    .unwrap(),
            );
            columns
                .tokens
                .insert(id, index.token_occurrences(id).unwrap().collect());
        }
        for (key, _) in index.partitions() {
            for term in index.terms() {
                for (document, positions) in index.postings(key, term) {
                    columns
                        .positions
                        .insert((term.to_owned(), document), positions.to_vec());
                    columns.counts.entry(term.to_owned()).or_default().insert(
                        document,
                        TextSource::posted_frequencies_owned(index, document, term, &workspace)
                            .unwrap()
                            .unwrap()
                            .to_vec(),
                    );
                }
            }
        }
        for ordinal in 0..columns.documents.len() {
            columns.occurrence_order.push(
                index
                    .occurrence_document_at(ordinal, &workspace)
                    .unwrap()
                    .unwrap(),
            );
        }
        columns
            .bounds
            .set(TextSourceBounds::measure(&columns, &workspace).unwrap())
            .unwrap();
        *columns.observed_workspace.lock().unwrap() = None;
        columns
    }

    fn read(&self, workspace: &WorkspaceCapability) -> Result<(), TextError> {
        if workspace.is_bounded() {
            *self.observed_workspace.lock().unwrap() = Some(workspace.clone());
        }
        if self.fail_reads.load(Ordering::Relaxed) {
            return Err(TextError::source_read_owned(
                purrdf_core::SegmentedError::Corrupt("column page checksum mismatch"),
                workspace,
            ));
        }
        Ok(())
    }

    fn page(
        &self,
        counts: &[(u32, u64)],
        workspace: &WorkspaceCapability,
    ) -> Result<Counts, TextError> {
        let reservation = workspace
            .charge(self.page_bytes.load(Ordering::Relaxed))
            .map_err(|error| TextError::workspace_failure(&error))?;
        let mut page = AdmittedVec::with_capacity(counts.len(), workspace)
            .map_err(|error| TextError::workspace_failure(&error))?;
        for &count in counts {
            page.push(count)
                .map_err(|error| TextError::workspace_failure(&error))?;
        }
        Ok(Counts {
            values: page,
            _page: reservation,
        })
    }

    fn partition(&self, key: &PartitionKey) -> Option<usize> {
        self.partitions
            .binary_search_by(|(candidate, ..)| candidate.cmp(key))
            .ok()
    }

    fn posting(&self, partition: usize, term: &str, ordinal: usize) -> Option<(u32, &[u32])> {
        self.positions
            .iter()
            .filter(|((word, document), _)| {
                word == term && self.document_partitions[*document as usize] == partition
            })
            .nth(ordinal)
            .map(|((_, document), positions)| (*document, positions.as_slice()))
    }
}

impl TextSource<GraphLanguage> for Columns {
    type Document<'a> = &'a Document;
    type Frequencies<'a> = Counts;
    type Term<'a> = &'a str;
    type Positions = Positions;

    fn config(&self) -> &TextIndexConfig {
        &self.config
    }
    fn ranking_profile(&self) -> &RankingProfile {
        &self.ranking
    }
    fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    fn source_fingerprint(&self) -> [u8; 32] {
        self.source_fingerprint
    }
    fn source_coverage(&self) -> &purrdf_text::SourceCoverage {
        &self.coverage
    }
    fn partitions(&self) -> impl Iterator<Item = (&PartitionKey, &PartitionStats)> {
        self.partitions.iter().map(|(key, stats, ..)| (key, stats))
    }
    fn partition_at(&self, ordinal: usize) -> Option<(&PartitionKey, &PartitionStats)> {
        self.partitions
            .get(ordinal)
            .map(|(key, stats, ..)| (key, stats))
    }
    fn local_partition_ordinal(&self, key: &PartitionKey) -> Option<usize> {
        self.partition(key)
    }
    fn partition_key_owned(
        &self,
        key: &PartitionKey,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<&PartitionKey>, TextError> {
        self.read(workspace)?;
        Ok(self
            .partition(key)
            .map(|ordinal| &self.partitions[ordinal].0))
    }
    fn field_totals(&self, key: &PartitionKey) -> Option<&[u128]> {
        self.partition(key)
            .map(|at| self.partitions[at].2.as_slice())
    }
    fn field_populations(&self, key: &PartitionKey) -> Option<&[u64]> {
        self.partition(key)
            .map(|at| self.partitions[at].3.as_slice())
    }
    fn document_count(&self) -> u64 {
        self.documents.len() as u64
    }
    fn term_count(&self) -> u64 {
        self.terms.len() as u64
    }
    fn relation_bounds(&self, _: &WorkspaceCapability) -> Result<TextSourceBounds, TextError> {
        Ok(*self.bounds.get().unwrap())
    }
    fn read_document(
        &self,
        document: u32,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<&Document>, TextError> {
        self.read(workspace)?;
        Ok(self.documents.get(&document))
    }
    fn documents_with_subject_owned(
        &self,
        subject: &TermValue,
        workspace: &WorkspaceCapability,
    ) -> Result<AdmittedVec<u32>, TextError> {
        self.read(workspace)?;
        let mut documents = AdmittedVec::new(workspace);
        if let Some(ids) = self.subjects.get(subject) {
            for &id in ids {
                documents
                    .push(id)
                    .map_err(|error| TextError::workspace_failure(&error))?;
            }
        }
        Ok(documents)
    }
    fn partition_ordinal_of_owned(
        &self,
        document: u32,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<usize>, TextError> {
        self.read(workspace)?;
        Ok(self.document_partitions.get(document as usize).copied())
    }
    fn posted_frequencies_owned(
        &self,
        document: u32,
        term: &str,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<Counts>, TextError> {
        self.read(workspace)?;
        self.counts
            .get(term)
            .and_then(|documents| documents.get(&document))
            .map(|counts| self.page(counts, workspace))
            .transpose()
    }
    fn document_frequency_owned(
        &self,
        key: &PartitionKey,
        term: &str,
        workspace: &WorkspaceCapability,
    ) -> Result<u64, TextError> {
        self.read(workspace)?;
        let Some(partition) = self.partition(key) else {
            return Ok(0);
        };
        Ok(self
            .positions
            .keys()
            .filter(|(word, document)| {
                word == term && self.document_partitions[*document as usize] == partition
            })
            .count() as u64)
    }
    fn field_posting_at(
        &self,
        key: &PartitionKey,
        term: &str,
        ordinal: usize,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<(u32, Counts)>, TextError> {
        self.read(workspace)?;
        let Some(partition) = self.partition(key) else {
            return Ok(None);
        };
        let Some((document, _)) = self.posting(partition, term, ordinal) else {
            return Ok(None);
        };
        Ok(self
            .posted_frequencies_owned(document, term, workspace)?
            .map(|counts| (document, counts)))
    }
    fn next_position(
        &self,
        partition: usize,
        term: &str,
        state: &mut Positions,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<(u32, u32)>, TextError> {
        self.read(workspace)?;
        while let Some((document, positions)) = self.posting(partition, term, state.posting) {
            if let Some(&position) = positions.get(state.position) {
                state.position += 1;
                return Ok(Some((document, position)));
            }
            state.posting += 1;
            state.position = 0;
        }
        Ok(None)
    }
    fn token_at(
        &self,
        document: u32,
        ordinal: usize,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<TokenOccurrence>, TextError> {
        self.read(workspace)?;
        Ok(self
            .tokens
            .get(&document)
            .and_then(|tokens| tokens.get(ordinal))
            .copied())
    }
    fn occurrence_document_at(
        &self,
        ordinal: usize,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<u32>, TextError> {
        self.read(workspace)?;
        Ok(self.occurrence_order.get(ordinal).copied())
    }
    fn read_term(
        &self,
        term: u32,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<&str>, TextError> {
        self.read(workspace)?;
        Ok(self.terms.get(term as usize).map(String::as_str))
    }
}
