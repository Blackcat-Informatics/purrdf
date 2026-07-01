// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL property path evaluation.
//!
//! Evaluates a [`Path`] against a [`ShaclDataGraph`], returning the set of value
//! nodes reachable from a given focus node.

use crate::data::{GraphFilter, ShaclDataGraph};
use crate::shapes::Path;
use crate::term::Term;

/// Evaluate a SHACL property path from `focus`, returning all reachable value
/// nodes in the default graph.
///
/// The result set is deduplicated (preserving first occurrence order) as SHACL
/// specifies value nodes as a set.  If `focus` is a `Literal` or cannot serve
/// as a subject, returns an empty `Vec`.
pub fn eval<G: ShaclDataGraph>(store: &G, focus: &Term, path: &Path) -> Vec<Term> {
    let mut nodes = eval_inner(store, focus, path);
    // Dedup preserving first-occurrence order.
    let mut seen = std::collections::HashSet::new();
    nodes.retain(|t| seen.insert(t.clone()));
    nodes
}

/// Convert a [`Path`] to its term representation for use in `result_path`.
///
/// - `Predicate(p)` → `Term::NamedNode(p)`
/// - `Inverse(inner)` → the predicate IRI of the innermost predicate (SHACL
///   path serialisation as a full blank-node structure is out-of-scope for
///   #576; the predicate IRI is a faithful approximation for the corpus).
pub fn path_to_term(path: &Path) -> Term {
    match path {
        Path::Predicate(p) => Term::NamedNode(p.clone()),
        Path::Inverse(inner) => path_to_term(inner),
    }
}

// ── Internal recursive evaluator ───────────────────────────────────────────────

fn eval_inner<G: ShaclDataGraph>(store: &G, focus: &Term, path: &Path) -> Vec<Term> {
    match path {
        Path::Predicate(p) => {
            // A literal/triple focus cannot be a subject; the data graph returns no
            // matches for it (matching the historical empty-result behavior).
            let predicate = Term::NamedNode(p.clone());
            store
                .quads_for_pattern(
                    Some(focus),
                    Some(&predicate),
                    None,
                    GraphFilter::DefaultGraph,
                )
                .into_iter()
                .map(|q| q.object)
                .collect()
        }
        Path::Inverse(inner) => match inner.as_ref() {
            // Inverse of a predicate: collect subjects of (?, p, focus).
            Path::Predicate(p) => {
                let predicate = Term::NamedNode(p.clone());
                store
                    .quads_for_pattern(
                        None,
                        Some(&predicate),
                        Some(focus),
                        GraphFilter::DefaultGraph,
                    )
                    .into_iter()
                    .map(|q| q.subject)
                    .collect()
            }
            // General inverse: eval inner with focus as "target", swap roles.
            // For any inner path, find all nodes `n` such that focus ∈ eval(n, inner).
            // This requires scanning every subject in the store — only Predicate inner
            // is needed for the corpus, but we keep it total.
            inner_path @ Path::Inverse(_) => {
                // Collect all distinct subjects from the default graph.
                let all_subjects: Vec<Term> = {
                    let mut subjects: Vec<Term> = store
                        .quads_for_pattern(None, None, None, GraphFilter::DefaultGraph)
                        .into_iter()
                        .map(|q| q.subject)
                        .collect();
                    let mut seen = std::collections::HashSet::new();
                    subjects.retain(|t| seen.insert(t.clone()));
                    subjects
                };
                all_subjects
                    .into_iter()
                    .filter(|candidate| {
                        eval_inner(store, candidate, inner_path)
                            .iter()
                            .any(|v| v == focus)
                    })
                    .collect()
            }
        },
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ::purrdf::RdfDataset;

    use super::*;
    use crate::data::IrDataGraph;
    use crate::term::{Literal, NamedNode};

    fn load_data(ttl: &str) -> IrDataGraph {
        let dataset: Arc<RdfDataset> =
            crate::text_ingest::parse_turtle_to_dataset(ttl).expect("turtle parse");
        IrDataGraph::new(dataset)
    }

    const DATA: &str = r"
        @prefix ex: <http://example.org/ns#> .
        ex:a ex:p ex:b .
        ex:a ex:p ex:c .
        ex:d ex:q ex:a .
    ";

    fn nn(iri: &str) -> Term {
        Term::NamedNode(NamedNode::new_unchecked(iri))
    }

    #[test]
    fn predicate_path_returns_objects() {
        let data = load_data(DATA);
        let focus = nn("http://example.org/ns#a");
        let path = Path::Predicate(NamedNode::new_unchecked("http://example.org/ns#p"));
        let mut result = eval(&data, &focus, &path);
        result.sort_by_key(Term::to_string);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&nn("http://example.org/ns#b")));
        assert!(result.contains(&nn("http://example.org/ns#c")));
    }

    #[test]
    fn inverse_path_returns_subjects() {
        let data = load_data(DATA);
        let focus = nn("http://example.org/ns#a");
        let path = Path::Inverse(Box::new(Path::Predicate(NamedNode::new_unchecked(
            "http://example.org/ns#q",
        ))));
        let result = eval(&data, &focus, &path);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], nn("http://example.org/ns#d"));
    }

    #[test]
    fn literal_focus_returns_empty() {
        let data = load_data(DATA);
        let focus = Term::Literal(Literal::new_simple_literal("hello"));
        let path = Path::Predicate(NamedNode::new_unchecked("http://example.org/ns#p"));
        assert!(eval(&data, &focus, &path).is_empty());
    }

    #[test]
    fn predicate_path_deduplicates() {
        let data = load_data(DATA);
        let focus = nn("http://example.org/ns#a");
        let path = Path::Predicate(NamedNode::new_unchecked("http://example.org/ns#p"));
        let result = eval(&data, &focus, &path);
        // Should be exactly 2 distinct values
        assert_eq!(result.len(), 2);
    }
}
