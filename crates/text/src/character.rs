// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent Han-character retrieval over the same source-aligned analysis.
//!
//! Keys contain one Han-script scalar or two adjacent Han-script scalars.
//! Attached combining marks and orthographic controls do not enter these keys;
//! evidence retains their enclosing grapheme highlight. Punctuation, whitespace,
//! another script, protected emoji and literal boundaries end adjacency. Every
//! stored run contributes all unigrams and adjacent bigrams. A one-character
//! query run reads unigrams; a longer run reads bigrams. This projection has its
//! own BM25 population, document lengths, term frequencies and generation.
//! Callers combine its ranked relation with lexical retrieval using the existing
//! retrieval composition API and explicit caller-selected weights.

use crate::query_workspace::{QueryString, admitted};
use purrdf_sparql_eval::{AdmittedVec, WorkspaceCapability};
use std::ops::Range;
use std::sync::Arc;

use purrdf_core::DatasetView;

use crate::analysis::{Analysis, Analyzer, Projection};
use crate::unicode::{self, SegmentationScript};
use crate::{RankingProfile, TextError, TextIndex, TextIndexConfig, TextSearchRelation};

/// Exact independent character projection and query-selection law.
pub const PROFILE_ID: &str = "unicode17-han-scalars-uni-bi-adjacent-egc-transparent/v2";

/// One matched character occurrence, bound to its source and index generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HanMatchEvidence {
    /// Document ordinal, resolved through [`HanCharacterIndex::index`].
    pub document: u32,
    /// Framed identity of the complete original RDF source row.
    pub literal: [u8; 32],
    /// Matched character key and its exact original contributors.
    pub projection: Projection,
    /// Analysis law shared by indexing and query analysis.
    pub analyzer: [u8; 32],
    /// Independent character index generation.
    pub generation: [u8; 32],
}

#[derive(Clone, Debug)]
pub(crate) struct HanOccurrence {
    pub document: u32,
    pub literal: [u8; 32],
    pub projection: Projection,
}

/// A distinct character corpus reusing the workspace's exact ranking engine.
#[derive(Clone, Debug)]
pub struct HanCharacterIndex {
    index: Arc<TextIndex>,
}

impl HanCharacterIndex {
    /// Build independent character statistics with the supplied immutable analyzer.
    ///
    /// # Errors
    /// Propagates source access, analysis and indexing bounds.
    pub fn from_dataset<D: DatasetView>(
        dataset: &D,
        config: &TextIndexConfig,
    ) -> Result<Self, TextError> {
        Self::from_dataset_with_ranking(dataset, config, RankingProfile::single_field())
    }

    /// Build with a caller-selected fielded ranking law.
    ///
    /// # Errors
    /// Propagates source access, analysis, field routing and indexing bounds.
    pub fn from_dataset_with_ranking<D: DatasetView>(
        dataset: &D,
        config: &TextIndexConfig,
        ranking: RankingProfile,
    ) -> Result<Self, TextError> {
        Ok(Self {
            index: Arc::new(TextIndex::from_dataset_with_ranking(
                dataset,
                &config.clone().for_han(),
                ranking,
            )?),
        })
    }

    /// Inspect this producer's independent statistics, postings and generation.
    #[must_use]
    pub fn index(&self) -> &TextIndex {
        &self.index
    }

    /// Expose the existing ranked producer seam. Registration IRIs, declarations
    /// and fusion parameters remain supplied by the caller.
    #[must_use]
    pub fn relation(&self) -> TextSearchRelation {
        TextSearchRelation::new(Arc::clone(&self.index))
    }

    /// Complete source evidence for one document's canonical character key.
    #[must_use]
    pub fn evidence(&self, document: u32, canonical: &str) -> Vec<HanMatchEvidence> {
        self.index
            .han_occurrences
            .iter()
            .filter(|occurrence| {
                occurrence.document == document && occurrence.projection.text == canonical
            })
            .map(|occurrence| HanMatchEvidence {
                document,
                literal: occurrence.literal,
                projection: occurrence.projection.clone(),
                analyzer: self.index.analyzer_fingerprint(),
                generation: self.index.fingerprint(),
            })
            .collect()
    }
}

struct Unit {
    scalar: char,
    range: Range<usize>,
    grapheme: Range<usize>,
}

/// Visit Han runs without joining across a different script or punctuation.
fn runs(text: &str, mut sink: impl FnMut(&[Unit])) {
    runs_owned(text, &WorkspaceCapability::default(), |run| {
        sink(run);
        Ok(())
    })
    .expect("resident Han run allocation");
}

fn runs_owned(
    text: &str,
    workspace: &WorkspaceCapability,
    mut sink: impl FnMut(&[Unit]) -> Result<(), TextError>,
) -> Result<(), TextError> {
    let mut run = AdmittedVec::new(workspace);
    for (start, grapheme) in unicode::grapheme_bounds(text) {
        if unicode::is_emoji_grapheme(grapheme) {
            sink(&run)?;
            run.clear();
            continue;
        }
        for (offset, scalar) in grapheme.char_indices() {
            if unicode::is_word_internal_control(scalar) || unicode::is_combining_mark(scalar) {
                continue;
            }
            if unicode::segmentation_script(scalar) == Some(SegmentationScript::Han) {
                admitted(run.push(Unit {
                    scalar,
                    range: start + offset..start + offset + scalar.len_utf8(),
                    grapheme: start..start + grapheme.len(),
                }))?;
            } else {
                sink(&run)?;
                run.clear();
            }
        }
    }
    sink(&run)
}

pub(crate) fn query_terms_owned(
    analyzer: &Analyzer,
    input: &str,
    workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<QueryString>, TextError> {
    let analysis = analyzer.analysis_form_owned(input, workspace)?;
    let mut out = AdmittedVec::new(workspace);
    runs_owned(&analysis, workspace, |run| {
        if run.len() == 1 {
            admitted(out.push(QueryString::chars([run[0].scalar].into_iter(), workspace)?))?;
        } else {
            for pair in run.windows(2) {
                admitted(out.push(QueryString::chars(
                    [pair[0].scalar, pair[1].scalar].into_iter(),
                    workspace,
                )?))?;
            }
        }
        Ok(())
    })?;
    Ok(out)
}

fn projection(analysis: &Analysis, units: &[Unit]) -> Projection {
    let first = &units[0];
    let last = &units[units.len() - 1];
    let mut sources = Vec::new();
    for unit in units {
        sources.extend(analysis.normalized.contributors(unit.range.clone()));
    }
    crate::analysis::merge_ranges(&mut sources);
    let enclosing = analysis
        .normalized
        .contributors(first.grapheme.start..last.grapheme.end);
    let highlight = enclosing.first().map_or(0, |range| range.start)
        ..enclosing.last().map_or(0, |range| range.end);
    Projection {
        text: units.iter().map(|unit| unit.scalar).collect(),
        range: first.range.start..last.range.end,
        sources,
        highlight,
        coarse: false,
    }
}

pub(crate) fn index_projections(analysis: &Analysis) -> Vec<Projection> {
    let mut out = Vec::new();
    runs(&analysis.normalized.text, |run| {
        for at in 0..run.len() {
            out.push(projection(analysis, &run[at..=at]));
            if at + 1 < run.len() {
                out.push(projection(analysis, &run[at..at + 2]));
            }
        }
    });
    out
}

pub(crate) fn query_terms(analyzer: &Analyzer, input: &str) -> Result<Vec<String>, TextError> {
    Ok(
        query_terms_owned(analyzer, input, &WorkspaceCapability::default())?
            .into_iter()
            .map(|term| term.as_str().to_owned())
            .collect(),
    )
}
