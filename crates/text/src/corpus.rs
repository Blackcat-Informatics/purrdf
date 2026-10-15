// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Closed physical corpus shapes for the one native text index.

use crate::TextError;
use purrdf_core::{DatasetView, GraphMatch, GraphSelector, TermValue};
use std::hash::Hash;

mod sealed {
    pub trait Sealed {}
}

/// The existing graph/subject/language document and graph/language partition law.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphLanguage;

/// Explicit subject/language documents and language-only partitions. Its graph
/// coordinate and selector occupy zero bytes and contribute no digest field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphFree;

impl sealed::Sealed for GraphLanguage {}
impl sealed::Sealed for GraphFree {}

/// A closed compile-time corpus law. Both shapes use the original analyzer,
/// partition construction, scorer and retrieval machinery.
pub trait CorpusProfile:
    sealed::Sealed + Clone + Copy + std::fmt::Debug + Eq + Ord + Hash + Send + Sync + 'static
{
    /// Stable identity of the physical document and partition law.
    const ID: &'static str;
    /// Physically stored graph coordinate.
    type Graph: Clone + std::fmt::Debug + Eq + Ord + Hash + Send + Sync;
    /// Caller-selected graph scope, absent in the graph-free profile.
    type Selector: Clone + std::fmt::Debug + Eq + Send + Sync;
    /// Whether this profile omits graph coordinates from its identity preimage.
    const GRAPH_FREE: bool;
    /// Inspect an existing graph coordinate without allocating.
    fn graph(value: &Self::Graph) -> Option<&TermValue>;
    /// Preserve a graph coordinate, or refuse an impossible named graph.
    ///
    /// # Errors
    /// Refuses named data in the graph-free shape.
    fn retain_graph(value: Option<TermValue>) -> Result<Self::Graph, TextError>;
    /// Inspect the configured graph selector, absent for graph-free input.
    fn selector(value: &Self::Selector) -> Option<&GraphSelector>;
    /// Resolve source scope through the original typed read protocol.
    ///
    /// # Errors
    /// Returns the source's original support or read failure.
    fn resolve<D: DatasetView>(
        value: &Self::Selector,
        source: &D,
    ) -> Result<Option<GraphMatch<D::Id>>, D::ReadError>;
}

impl CorpusProfile for GraphLanguage {
    const ID: &'static str = crate::INDEX_CORPUS_PROFILE_ID;
    type Graph = Option<TermValue>;
    type Selector = GraphSelector;
    const GRAPH_FREE: bool = false;
    fn graph(value: &Self::Graph) -> Option<&TermValue> {
        value.as_ref()
    }
    fn retain_graph(value: Option<TermValue>) -> Result<Self::Graph, TextError> {
        Ok(value)
    }
    fn selector(value: &Self::Selector) -> Option<&GraphSelector> {
        Some(value)
    }
    fn resolve<D: DatasetView>(
        value: &Self::Selector,
        source: &D,
    ) -> Result<Option<GraphMatch<D::Id>>, D::ReadError> {
        value.resolve(source)
    }
}

impl CorpusProfile for GraphFree {
    const ID: &'static str = "purrdf-text-corpus-subject-language-v1";
    type Graph = ();
    type Selector = ();
    const GRAPH_FREE: bool = true;
    fn graph((): &Self::Graph) -> Option<&TermValue> {
        None
    }
    fn retain_graph(value: Option<TermValue>) -> Result<Self::Graph, TextError> {
        if value.is_some() {
            return Err(TextError::config(
                "named graph data cannot enter a graph-free text corpus",
            ));
        }
        Ok(())
    }
    fn selector((): &Self::Selector) -> Option<&GraphSelector> {
        None
    }
    fn resolve<D: DatasetView>(
        (): &Self::Selector,
        source: &D,
    ) -> Result<Option<GraphMatch<D::Id>>, D::ReadError> {
        source.checked_read(|_| Some(GraphMatch::Default))
    }
}
