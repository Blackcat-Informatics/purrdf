// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The one named-graph refusal vocabulary every host shares.
//!
//! Turtle, N-Triples and RDF/XML have no named-graph construct, and the single-graph
//! serializers DROP every graph-scoped row rather than folding it into the default
//! graph (see [`loss`](crate::loss)'s `named-graph-dropped` contract note). Whenever a
//! host is handed a graph-carrying result and a single-graph target it must say so
//! instead of emitting a well-formed document that silently omits what was asked for.
//!
//! The CLI, the Python extension and the wasm binding all raise that refusal, each in
//! its own error type and each naming its own format spelling. Everything else about
//! the message — which graphs, in what order, how many are listed, and the sentence
//! itself — lives here exactly once, so three hosts cannot drift into three different
//! accounts of the same behaviour.

use std::collections::BTreeSet;

use crate::dataset_view::DatasetView;
use crate::ir::TermRef;

/// How many graph names a refusal spells out individually before it summarises the
/// rest as a count.
///
/// A `CONSTRUCT` template can name a graph per statement — and with a graph VARIABLE
/// one template writes as many graphs as the `WHERE` has distinct bindings — so the
/// name list is unbounded in principle. Eight is enough to identify the mistake in
/// every hand-written query and short enough that the message stays a message; the
/// tail is reported as "and N more" rather than truncated silently, so the count is
/// always exact even when the list is not complete.
pub const NAMED_GRAPH_SAMPLE_LIMIT: usize = 8;

/// Every distinct non-default graph name `view` carries, rendered in N-Triples term
/// syntax and sorted lexicographically.
///
/// Sorted through a [`BTreeSet`], not merely deduplicated: a refusal message must be
/// byte-identical across runs, and both the dataset's quad order and any hash-map
/// iteration would make it a function of insertion order. It reads the graph slot of
/// the base quads AND of the RDF-1.2 statement-layer rows, because a reifier or
/// annotation can be scoped to a graph whose base quads the template never wrote —
/// a graph the single-graph flattening would drop just as silently.
///
/// An empty result means the view is default-graph-only, which is the signal a host
/// uses to let the serialization proceed untouched.
pub fn distinct_graph_names<D: DatasetView>(view: &D) -> Vec<String> {
    let slots = view
        .quads()
        .map(|q| q.g)
        .chain(view.reifier_quads().map(|q| q.g))
        .chain(view.annotation_quads().map(|q| q.g));
    let names: BTreeSet<String> = slots
        .flatten()
        .map(|id| render_graph_name(view, id))
        .collect();
    names.into_iter().collect()
}

/// Render one graph-name term for a diagnostic, in N-Triples term syntax.
///
/// A CONSTRUCT template's graph slot only ever resolves to an IRI (a graph variable
/// bound to anything else skips the statement, per SPARQL §16.2), and the RDF 1.2
/// abstract syntax admits only an IRI or a blank node in the graph position — but the
/// match is total over [`TermRef`] rather than partial, because a diagnostic that
/// panics on a term it did not expect is worse than one that names it.
fn render_graph_name<D: DatasetView>(view: &D, id: D::Id) -> String {
    match view.resolve(id) {
        TermRef::Iri(iri) => format!("<{iri}>"),
        TermRef::Blank { label, .. } => format!("_:{label}"),
        TermRef::Literal { lexical, .. } => format!("\"{lexical}\""),
        TermRef::Triple { .. } => "<<( … )>>".to_owned(),
    }
}

/// The refusal sentence a host raises when a graph-carrying result meets a
/// single-graph RDF syntax: which graphs, which format, and what to use instead.
///
/// `names` is a [`distinct_graph_names`] list (or the host's equivalent over its own
/// quad model) and MUST be non-empty — a caller with no named graph has nothing to
/// refuse and must not build this message. `target` is the format spelled the way the
/// caller spelled it (`turtle`, `RdfFormat.TURTLE`, …) and `remedy` is the closing
/// imperative naming that host's quad-capable alternatives, because "re-run with
/// `--results-format`" and "re-serialize with an `RdfFormat` member" are the same
/// instruction in two different vocabularies.
///
/// # Why every host REFUSES rather than serializing what fits
///
/// The graph name is in the QUERY the caller wrote, one token at a time
/// (`CONSTRUCT { GRAPH ex:out { … } }`), so it is the single most explicit thing in the
/// request; and a mixed template makes refusal the only honest answer, because
/// emitting the default-graph half would report a partial answer as a complete one,
/// which is worse than emitting nothing. So ANY non-default graph refuses.
#[must_use]
pub fn named_graph_refusal(names: &[String], target: &str, remedy: &str) -> String {
    let count = names.len();
    debug_assert!(
        count > 0,
        "a refusal needs at least one named graph to name"
    );
    let listed = if count > NAMED_GRAPH_SAMPLE_LIMIT {
        format!(
            "{}, and {} more",
            names[..NAMED_GRAPH_SAMPLE_LIMIT].join(", "),
            count - NAMED_GRAPH_SAMPLE_LIMIT
        )
    } else {
        names.join(", ")
    };
    let (graphs, them) = if count == 1 {
        ("named graph", "it")
    } else {
        ("named graphs", "them")
    };
    format!(
        "a CONSTRUCT/DESCRIBE result carrying {count} {graphs} ({listed}) cannot be \
         serialized to the single-graph RDF syntax `{target}`: {target} has no named-graph \
         construct, so every statement in {them} would be DROPPED (not folded into the \
         default graph) and the output would silently omit what the query asked for. \
         {remedy}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(count: usize) -> Vec<String> {
        (0..count)
            .map(|i| format!("<https://example.org/g{i}>"))
            .collect()
    }

    #[test]
    fn one_graph_reads_singular() {
        let message = named_graph_refusal(&names(1), "turtle", "Use nquads");
        assert!(message.contains("carrying 1 named graph (<https://example.org/g0>)"));
        assert!(message.contains("every statement in it would be DROPPED"));
        assert!(message.ends_with("Use nquads"));
    }

    #[test]
    fn several_graphs_read_plural_and_list_in_order() {
        let message = named_graph_refusal(&names(3), "ntriples", "Use trig");
        assert!(message.contains(
            "carrying 3 named graphs (<https://example.org/g0>, <https://example.org/g1>, \
             <https://example.org/g2>)"
        ));
        assert!(message.contains("every statement in them would be DROPPED"));
    }

    #[test]
    fn the_tail_beyond_the_sample_limit_is_counted_not_truncated() {
        let message =
            named_graph_refusal(&names(NAMED_GRAPH_SAMPLE_LIMIT + 2), "turtle", "Use trig");
        // The count is exact even though the list is not complete.
        assert!(message.contains(&format!(
            "carrying {} named graphs",
            NAMED_GRAPH_SAMPLE_LIMIT + 2
        )));
        assert!(message.contains("and 2 more"));
        assert!(!message.contains("<https://example.org/g8>"));
    }
}
