// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The content digest of a GTS file: the canonical labelling run over the terms
//! of its graph exactly as the file holds them.
//!
//! The GTS format compares a language text whole and bytewise, so `"x"@EN-us`
//! and `"x"@en-us` are two terms of one file. A dataset folds a language tag to
//! ASCII lower case in its term interner and holds one term. The two are two
//! identities:
//!
//! * the canonical RDF digest of a dataset (folded), and
//! * the content digest of a GTS file (as written),
//!
//! and they are equal exactly when every tag of the file is already lower case.
//!
//! [`GtsWrittenView`] is a [`DatasetView`] over a graph read from a file whose
//! language tags are those of the file. It lowers the graph through the one
//! statement-layer fold every GTS route into a dataset uses
//! (`dataset_from_ser_graph`), so quads, reifier rows, annotation rows and
//! triple terms mean what they mean on every other route. The fold would merge
//! two spellings of a tag, so each distinct written spelling that is not
//! already lower case enters the fold under a private-use surrogate tag of its
//! own; the view resolves the surrogate back to the written text. No surrogate
//! is ever visible through the view, and the labelling lowers the tag bytes as
//! written.

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::fmt;
use std::sync::Arc;

use purrdf_core::canonical_labelling::{Labelled, Limit, Scope, ViewError, label_view};
use purrdf_core::{
    DatasetView, GraphMatch, QuadIds, QuadProbePlan, RdfDataset, RdfDiagnostic,
    RdfStoreCapabilities, TermId, TermRef, TermValue,
};
use purrdf_gts::model::Graph;
use purrdf_hash::error::StructuredError;
use purrdf_iri::langtag::{identity_fold, is_identity_folded};

use purrdf_core::{LossEntry, RdfLocation};

use crate::gts::gts_to_ser;
use crate::native_codecs::parse::dataset_from_ser_graph;

/// Why the content digest of a GTS graph was not computed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GtsContentError {
    /// The graph cannot be lowered to a dataset: a dangling term, a term that
    /// reaches itself, an unbound triple term.
    Graph(RdfDiagnostic),
    /// The labelling refused the lines, or its limit tripped.
    Labelling(ViewError<Infallible>),
}

impl fmt::Display for GtsContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Graph(cause) => write!(f, "the GTS graph cannot be lowered to lines: {cause}"),
            Self::Labelling(cause) => {
                write!(f, "the GTS content digest was not computed: {cause}")
            }
        }
    }
}

impl std::error::Error for GtsContentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Graph(cause) => Some(cause),
            Self::Labelling(cause) => Some(cause),
        }
    }
}

impl StructuredError for GtsContentError {
    purrdf_hash::error_visit! {
        Graph(cause) => "gts-content-graph" { "cause" => Cause(cause) },
        Labelling(cause) => "gts-content-labelling" { "cause" => Cause(cause) },
    }
}

impl From<RdfDiagnostic> for GtsContentError {
    fn from(cause: RdfDiagnostic) -> Self {
        Self::Graph(cause)
    }
}

/// A [`DatasetView`] over the terms of a GTS graph as the file holds them.
///
/// Language tags are the written bytes: two spellings that differ only in case
/// are two terms. Built by [`GtsWrittenView::of`]; read-only.
#[derive(Debug)]
pub struct GtsWrittenView {
    /// The graph folded with each non-lower-case spelling replaced by its
    /// surrogate.
    inner: Arc<RdfDataset>,
    /// Surrogate tag to the written spelling.
    written: BTreeMap<String, String>,
    /// Written spelling to its surrogate.
    surrogate: BTreeMap<String, String>,
}

impl GtsWrittenView {
    /// The view of `graph`.
    ///
    /// # Errors
    /// [`RdfDiagnostic`] when the graph cannot be lowered: a dangling term
    /// reference, a term that reaches itself, an unbound triple term.
    pub fn of(graph: &Graph) -> Result<Self, RdfDiagnostic> {
        let mut ser = gts_to_ser(graph)?;
        // A surrogate must not be the folded form of any tag the file holds.
        let taken: BTreeSet<String> = ser
            .terms
            .iter()
            .filter_map(|term| term.lang.as_deref())
            .map(identity_fold)
            .collect();
        let mut written = BTreeMap::new();
        let mut surrogate: BTreeMap<String, String> = BTreeMap::new();
        let mut next = 0u64;
        for term in &mut ser.terms {
            let Some(tag) = term.lang.as_deref() else {
                continue;
            };
            if is_identity_folded(tag) {
                continue;
            }
            let replacement = match surrogate.get(tag) {
                Some(existing) => existing.clone(),
                None => {
                    let fresh = loop {
                        let candidate = format!("x-{next:08x}");
                        next += 1;
                        if !taken.contains(&candidate) {
                            break candidate;
                        }
                    };
                    written.insert(fresh.clone(), tag.to_owned());
                    surrogate.insert(tag.to_owned(), fresh.clone());
                    fresh
                }
            };
            term.lang = Some(replacement);
        }
        Ok(Self {
            inner: dataset_from_ser_graph(&ser)?,
            written,
            surrogate,
        })
    }

    /// The tag `view_tag` of the inner dataset as the file wrote it.
    fn written_tag<'a>(&'a self, tag: &'a str) -> &'a str {
        self.written.get(tag).map_or(tag, String::as_str)
    }
}

impl DatasetView for GtsWrittenView {
    type Id = TermId;
    type ReadError = Infallible;
    type TermGuard<'a> = TermRef<'a, TermId>;
    type ProbePlan = QuadProbePlan;

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::quads(self.inner.as_ref())
    }

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::reifier_quads(self.inner.as_ref())
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::annotation_quads(self.inner.as_ref())
    }

    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        DatasetView::named_graphs(self.inner.as_ref())
    }

    fn resolve(&self, id: TermId) -> Result<TermRef<'_, TermId>, Infallible> {
        Ok(match self.inner.as_ref().resolve(id) {
            TermRef::Literal {
                lexical,
                datatype,
                language: Some(tag),
                direction,
            } => TermRef::Literal {
                lexical,
                datatype,
                language: Some(self.written_tag(tag)),
                direction,
            },
            other => other,
        })
    }

    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, Infallible> {
        let TermValue::Literal {
            lexical_form,
            datatype,
            language: Some(tag),
            direction,
        } = value
        else {
            return Ok(self.inner.as_ref().term_id_by_value(value));
        };
        // A surrogate is private to the fold: it names no written term.
        if self.written.contains_key(tag) {
            return Ok(None);
        }
        let tag = self.surrogate.get(tag).unwrap_or(tag);
        Ok(self.inner.as_ref().term_id_by_value(&TermValue::Literal {
            lexical_form: lexical_form.clone(),
            datatype: datatype.clone(),
            language: Some(tag.clone()),
            direction: *direction,
        }))
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        self.inner.capabilities()
    }

    fn probe_plan(
        &self,
        s_bound: bool,
        p_bound: bool,
        o_bound: bool,
        g: GraphMatch,
    ) -> QuadProbePlan {
        DatasetView::probe_plan(self.inner.as_ref(), s_bound, p_bound, o_bound, g)
    }

    fn quads_for_pattern_with_plan(
        &self,
        plan: &QuadProbePlan,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::quads_for_pattern_with_plan(self.inner.as_ref(), plan, s, p, o, g)
    }

    fn term_count(&self) -> u64 {
        DatasetView::term_count(self.inner.as_ref())
    }
}

/// The labelling of `graph` over its terms as the file holds them: the
/// canonical line sequence, its digest and the structural record.
///
/// The lines are those of [`GtsWrittenView`] under the lowering rule of the
/// canonical labelling (every quad, reifier row, annotation row and declared
/// empty graph, no caller record).
///
/// # Errors
/// [`GtsContentError::Graph`] when the graph cannot be lowered;
/// [`GtsContentError::Labelling`] when the labelling refuses the lines.
pub fn gts_content_labelling(graph: &Graph) -> Result<Labelled, GtsContentError> {
    let view = GtsWrittenView::of(graph)?;
    label_view(&view, &[], Scope::Dataset, Limit::ENGINE, None)
        .map(|labelling| labelling.labelled().clone())
        .map_err(GtsContentError::Labelling)
}

/// The content digest of a GTS graph: the 32-byte digest of the canonical line
/// sequence of [`gts_content_labelling`], bound to the labelling identifier
/// `(nested-refinement-labelling, 1)`.
///
/// Differs from the canonical digest of the dataset the graph folds into
/// exactly when a language tag of the graph is not already lower case.
///
/// # Errors
/// As [`gts_content_labelling`].
pub fn gts_content_digest(graph: &Graph) -> Result<[u8; 32], GtsContentError> {
    gts_content_labelling(graph).map(|labelled| labelled.digest)
}

/// The loss entry for a language-tagged literal whose written tag is not
/// already lower case, or `None` when the dataset holds the term as written.
///
/// The note names the literal, the written tag and the tag the dataset holds,
/// so the term the fold merges with its case variant is identified.
pub(crate) fn language_fold_loss(
    lexical: &str,
    tag: &str,
    location: RdfLocation,
) -> Option<LossEntry> {
    if is_identity_folded(tag) {
        return None;
    }
    Some(LossEntry {
        code: "language-tag-case-folded".into(),
        from: "gts".into(),
        to: "rdf-1.2-dataset".into(),
        note: format!(
            "the literal {lexical:?} with written language tag {tag:?} is held as {:?}; \
             a term of this file that differs only in tag case is the same dataset term",
            identity_fold(tag)
        )
        .into(),
        location: Some(Box::new(location)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_gts::model::{Term, TermKind};

    fn graph_with(tags: &[&str]) -> Graph {
        let mut graph = Graph::default();
        graph.terms.push(Term::iri("http://example.org/s"));
        graph.terms.push(Term::iri("http://example.org/p"));
        for (i, tag) in tags.iter().enumerate() {
            graph.terms.push(Term::lang_literal("x", *tag));
            graph.quads.push((0, 1, 2 + i, None));
        }
        assert!(graph.terms.iter().all(|t| t.kind != TermKind::Triple));
        graph
    }

    /// The GTS format compares a language text bytewise: a file holding
    /// `"x"@EN-us` and `"x"@en-us` has two terms, so the labelling has two
    /// lines, and the digest differs from that of the folded dataset.
    #[test]
    fn case_variant_tags_are_two_lines_and_a_digest_apart_from_the_dataset() {
        let graph = graph_with(&["EN-us", "en-us"]);
        let labelled = gts_content_labelling(&graph).expect("labelled");
        assert_eq!(labelled.lines.len(), 2);
        let dataset = crate::gts::dataset_from_gts_graph(&graph).expect("dataset");
        let folded =
            label_view(dataset.as_ref(), &[], Scope::Dataset, Limit::ENGINE, None).expect("folded");
        assert_eq!(folded.labelled().lines.len(), 1);
        assert_ne!(folded.labelled().digest, labelled.digest);
        // The written bytes, not a surrogate, are in the lines.
        let joined: Vec<u8> = labelled
            .lines
            .iter()
            .flat_map(|l| l.iter().copied())
            .collect();
        assert!(joined.windows(5).any(|w| w == b"EN-us"));
        assert!(joined.windows(5).any(|w| w == b"en-us"));
    }

    /// The two identities are equal when every tag is already lower case.
    #[test]
    fn lower_case_tags_give_the_dataset_digest() {
        let graph = graph_with(&["en-us", "fr"]);
        let dataset = crate::gts::dataset_from_gts_graph(&graph).expect("dataset");
        let folded =
            label_view(dataset.as_ref(), &[], Scope::Dataset, Limit::ENGINE, None).expect("folded");
        assert_eq!(
            gts_content_digest(&graph).expect("digest"),
            folded.labelled().digest
        );
    }

    /// A surrogate never collides with a tag the file holds.
    #[test]
    fn a_written_tag_spelled_like_a_surrogate_stays_distinct() {
        let graph = graph_with(&["x-00000000", "X-00000000"]);
        let labelled = gts_content_labelling(&graph).expect("labelled");
        assert_eq!(labelled.lines.len(), 2);
    }

    /// A file with two case-variant terms is two lines through the raw view, and
    /// reading it into a dataset reports the written tag in the loss ledger
    /// instead of merging silently; an all-lower-case file reports nothing.
    #[test]
    fn reading_a_case_variant_file_into_a_dataset_reports_the_fold() {
        let bytes = |tags: &[&str]| {
            purrdf_gts::writer::Writer::deterministic(&graph_with(tags), "purrdf-test")
                .expect("writer")
                .to_bytes()
        };
        let variant = bytes(&["EN-us", "en-us"]);
        let graph = crate::gts::read_graph(&variant, true).expect("read");
        assert_eq!(
            gts_content_labelling(&graph).expect("labelled").lines.len(),
            2
        );

        let bundle = crate::import_gts_events(&variant).expect("import");
        let losses = bundle.envelope.losses.entries();
        assert_eq!(losses.len(), 1);
        assert_eq!(losses[0].code, "language-tag-case-folded");
        assert!(losses[0].note.contains("\"EN-us\"") && losses[0].note.contains("\"en-us\""));
        assert!(
            purrdf_core::check_ledger_sound(&bundle.envelope.losses, "gts", "rdf-1.2-dataset")
                .is_ok()
        );

        let lower = crate::import_gts_events(&bytes(&["en-us", "fr"])).expect("import");
        assert!(lower.envelope.losses.is_empty());
        let graph = crate::import_gts_graph(crate::gts::read_graph(&variant, true).expect("read"))
            .expect("graph import");
        assert_eq!(graph.envelope.losses.entries().len(), 1);
    }
}
