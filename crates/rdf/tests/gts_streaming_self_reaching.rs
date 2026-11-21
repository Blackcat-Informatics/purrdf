// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The streaming importer decides which triple terms reach themselves exactly
//! as the materializing fold does (GTS-SPEC §7.3).
//!
//! `import_gts_events` resolves a segment one term at a time against bounded
//! state, and `dataset_from_gts_graph` folds the whole graph first. Both must
//! produce the same quads and the same `reifies` rows for every input, so each
//! case below is read both ways and compared. The distinguishing shapes are
//! the components that close through terms the search has already finished.

use purrdf_gts::model::Term;
use purrdf_gts::writer::Writer;
use purrdf_rdf::gts::dataset_from_gts_graph;
use purrdf_rdf::{RdfDataset, SerializeGraph, import_gts_events, serialize_dataset};

/// A `k:3` term with no `tt` that takes its components from the first row
/// bound to `reifier`. Naming its own id makes it self-bound.
fn indirect_triple(reifier: usize) -> Term {
    let mut term = Term::triple_term((0, 0, 0));
    term.triple = None;
    term.reifier = Some(reifier);
    term
}

/// The streaming importer scopes a blank label with an escape prefix and writes
/// `_` as `_00005F`; undo both so labels compare as the fold spells them.
fn unescape_blank_labels(text: &str) -> String {
    text.replace("purrdfesc1_", "").replace("_00005F", "_")
}

/// The sorted N-Quads and the sorted reifier rows of a dataset.
fn projection(dataset: &RdfDataset) -> (Vec<String>, Vec<String>) {
    let bytes = serialize_dataset(dataset, "application/n-quads", SerializeGraph::Dataset)
        .expect("the dataset serializes");
    let mut quads: Vec<String> = String::from_utf8(bytes)
        .expect("N-Quads are UTF-8")
        .lines()
        .map(str::to_owned)
        .collect();
    quads.sort();
    for line in &mut quads {
        *line = unescape_blank_labels(line);
    }
    let mut rows: Vec<String> = dataset
        .owned_reifiers()
        .map(|r| unescape_blank_labels(&format!("{:?} {:?} {:?}", r.reifier, r.statement, r.graph)))
        .collect();
    rows.sort();
    (quads, rows)
}

/// Write the case, import it both ways and return the common projection.
fn both_ways(
    terms: &[Term],
    quads: &[purrdf_gts::model::Quad],
    rows: &[purrdf_gts::model::ReifierRow],
) -> (Vec<String>, Vec<String>) {
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(terms);
    writer.add_quads(quads);
    writer.add_reifies(rows);
    let bytes = writer.into_bytes();

    let graph = purrdf_gts::reader::read(&bytes, false, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    let folded = projection(&dataset_from_gts_graph(&graph).expect("the fold admits the case"));
    let streamed = projection(
        &import_gts_events(&bytes)
            .expect("the streaming importer admits the case")
            .dataset,
    );
    assert_eq!(streamed, folded, "streaming and materializing folds agree");
    assert_eq!(streamed.1.len(), rows.len(), "every reifies row is kept");
    streamed
}

/// A strongly connected component of three triple terms, A -> C, C -> A,
/// A -> B and B -> C, in which the cycle through B closes through a term the
/// search has already visited. Every member degrades to its own blank node
/// whichever term the search starts from.
#[test]
fn every_member_of_a_component_degrades_whichever_term_the_search_starts_from() {
    for [a, b, c] in [[2, 3, 4], [4, 2, 3], [3, 4, 2]] {
        let mut terms = vec![
            Term::iri("https://example.org/s"),
            Term::iri("https://example.org/p"),
        ];
        // Each member is self-bound, so the row keyed by its own id binds it.
        terms.extend((2..5).map(indirect_triple));
        let rows = [
            (a, (c, 1, b), None),
            (b, (0, 1, c), None),
            (c, (a, 1, 0), None),
        ];
        let (quads, _) = both_ways(
            &terms,
            &[(0, 1, a, None), (0, 1, b, None), (0, 1, c, None)],
            &rows,
        );
        // Three asserted quads and the three reifies rows.
        assert_eq!(quads.len(), 6);
        for id in [a, b, c] {
            assert!(
                quads
                    .iter()
                    .any(|q| q.contains(&format!("_:unbound_triple_{id}"))),
                "term {id} degrades: {quads:?}"
            );
        }
    }
}

/// A long chain in which each term names the previous one is not a cycle: no
/// term degrades, and the nesting renders in full.
#[test]
fn a_long_chain_that_is_not_a_cycle_is_unchanged() {
    let mut terms = vec![
        Term::iri("https://example.org/s"),
        Term::iri("https://example.org/p"),
    ];
    let mut rows = Vec::new();
    let mut previous = 0;
    for link in 0..300 {
        let reifier = terms.len();
        terms.push(Term::iri(format!("https://example.org/r{link}")));
        terms.push(indirect_triple(reifier));
        rows.push((reifier, (0, 1, previous), None));
        previous = reifier + 1;
    }
    let (quads, _) = both_ways(&terms, &[(0, 1, previous, None)], &rows);
    assert!(
        quads.iter().all(|q| !q.contains("unbound_triple")),
        "{quads:?}"
    );
}

/// A reifier that reifies a triple which mentions a term indirectly bound to
/// that very reifier: the term degrades, the row stays, and the quad that uses
/// the term stays.
#[test]
fn a_row_naming_the_term_that_resolves_through_it_keeps_the_row() {
    let terms = [
        Term::iri("https://example.org/r1"),
        Term::iri("https://example.org/label"),
        Term::iri("https://example.org/Cat"),
        indirect_triple(0),
    ];
    let (quads, rows) = both_ways(&terms, &[(2, 1, 3, None)], &[(0, (3, 1, 2), None)]);
    assert!(
        quads.contains(
            &"<https://example.org/Cat> <https://example.org/label> _:unbound_triple_3 ."
                .to_owned()
        ),
        "{quads:?}"
    );
    assert_eq!(rows.len(), 1);
}

/// The neighbouring valid case: a triple term nested in another, bound by an
/// ordinary row, resolves in full.
#[test]
fn a_valid_nested_triple_term_resolves_in_full() {
    let terms = [
        Term::iri("https://example.org/a"),
        Term::iri("https://example.org/p"),
        Term::iri("https://example.org/b"),
        Term::triple_term((0, 1, 2)),
        Term::triple_term((0, 1, 3)),
    ];
    let (quads, rows) = both_ways(&terms, &[(0, 1, 4, None)], &[(0, (0, 1, 3), None)]);
    assert_eq!(rows.len(), 1);
    assert!(
        quads.iter().all(|q| !q.contains("unbound_triple")),
        "{quads:?}"
    );
}
