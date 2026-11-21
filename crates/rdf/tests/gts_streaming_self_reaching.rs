// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The streaming importer decides which triple terms state no triple exactly
//! as the materializing fold does (GTS-SPEC §7.3).
//!
//! `import_gts_events` resolves a segment one term at a time against bounded
//! state, and `dataset_from_gts_graph` folds the whole graph first. Both must
//! produce the same quads and the same `reifies` rows for every input, so each
//! case below is read both ways and compared. The distinguishing shapes are
//! the components that close through terms the search has already finished.

use purrdf_gts::model::Term;
use purrdf_gts::writer::Writer;
use purrdf_rdf::gts::{dataset_from_gts_graph, flattened_dataset_from_gts_graph};
use purrdf_rdf::gts_view::GtsFoldView;
use purrdf_rdf::{RdfDataset, SerializeGraph, import_gts_events, serialize_dataset};

/// A `k:3` term with no `tt` that takes its components from the first row
/// bound to `reifier`, an earlier term (GTS-SPEC §7.2).
fn indirect_triple(reifier: usize) -> Term {
    let mut term = Term::triple_term((0, 0, 0));
    term.triple = None;
    term.reifier = Some(reifier);
    term
}

/// The streaming importer scopes a document blank label with an escape prefix and
/// writes `_` as `_00005F`; undo both so document labels compare as the fold spells
/// them. A triple term that states no triple lives in the one reserved scope, keeps
/// that prefix, and names its segment: the segment `0` of a one-segment file is
/// dropped so the label reads as the fold's, which has no segments.
fn unescape_blank_labels(text: &str) -> String {
    text.replace("purrdfesc1_", "")
        .replace("_:purrdfesc_", "_:")
        .replace(
            "unbound_00005Ftriple_00005F0_00005F",
            "unbound_00005Ftriple_00005F",
        )
        .replace("_00005F", "_")
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
    for line in &mut quads {
        *line = unescape_blank_labels(line);
    }
    quads.sort();
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
    for [a, b, c] in [[5, 6, 7], [7, 5, 6], [6, 7, 5]] {
        let mut terms = vec![
            Term::iri("https://example.org/s"),
            Term::iri("https://example.org/p"),
        ];
        // Members 5, 6 and 7 read the IRI reifiers 2, 3 and 4, so the row keyed
        // by a member's reifier (its id minus 3) binds it.
        terms.extend((2..5).map(|r| Term::iri(format!("https://example.org/r{r}"))));
        terms.extend((2..5).map(indirect_triple));
        let rows = [
            (a - 3, (c, 1, b), None),
            (b - 3, (0, 1, c), None),
            (c - 3, (a, 1, 0), None),
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
                    .any(|q| q.contains(&format!("_:purrdfesc4294967295_unbound_triple_{id}"))),
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
            &"<https://example.org/Cat> <https://example.org/label> _:purrdfesc4294967295_unbound_triple_3 ."
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

/// A `k:3` term with neither `tt` nor `rf` states no triple (§7.3, third
/// resolution rule). A reader keeps it as a distinct term and a projection
/// renders it as a fresh blank node, so the fold view, both dataset bridges and
/// the streaming importer must all admit the file and project the same blank
/// node for it: the one id used in every position names one node, and a second
/// bare term names another. The neighbouring valid case, a term with `tt`, is
/// unchanged.
#[test]
fn a_bare_triple_term_is_one_fresh_blank_node_on_every_path() {
    let bare = || {
        let mut term = Term::triple_term((0, 0, 0));
        term.triple = None;
        term
    };
    let terms = [
        Term::iri("https://example.org/s"), // 0
        Term::iri("https://example.org/p"), // 1
        bare(),                             // 2
        bare(),                             // 3
        Term::triple_term((0, 1, 3)),       // 4: states a triple whose object is bare
    ];
    let quads = [
        (2, 1, 0, None),
        (0, 1, 2, None),
        (2, 1, 2, None),
        (0, 1, 3, None),
        (0, 1, 4, None),
    ];
    let (streamed, _) = both_ways(&terms, &quads, &[]);
    assert_eq!(
        streamed,
        [
            "<https://example.org/s> <https://example.org/p> <<( <https://example.org/s> <https://example.org/p> _:purrdfesc4294967295_unbound_triple_3 )>> .",
            "<https://example.org/s> <https://example.org/p> _:purrdfesc4294967295_unbound_triple_2 .",
            "<https://example.org/s> <https://example.org/p> _:purrdfesc4294967295_unbound_triple_3 .",
            "_:purrdfesc4294967295_unbound_triple_2 <https://example.org/p> <https://example.org/s> .",
            "_:purrdfesc4294967295_unbound_triple_2 <https://example.org/p> _:purrdfesc4294967295_unbound_triple_2 .",
        ]
    );

    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_quads(&quads);
    let graph = purrdf_gts::reader::read(&writer.into_bytes(), false, None);
    let view = GtsFoldView::new(graph).expect("the fold view admits the file");
    assert_eq!(
        view.nq_token(2),
        "_:purrdfesc4294967295_unbound_00005Ftriple_00005F2"
    );
    assert_eq!(
        view.nq_token(3),
        "_:purrdfesc4294967295_unbound_00005Ftriple_00005F3"
    );
    assert_eq!(
        view.nq_token(4),
        "<<( <https://example.org/s> <https://example.org/p> _:purrdfesc4294967295_unbound_00005Ftriple_00005F3 )>>"
    );

    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_quads(&quads);
    let graph = purrdf_gts::reader::read(&writer.into_bytes(), false, None);
    assert_eq!(
        projection(
            &flattened_dataset_from_gts_graph(&graph).expect("the flattened bridge admits it")
        ),
        (streamed, Vec::new()),
        "the flattened bridge projects what the other paths project"
    );
}

/// The N-Quads projection of every path that reads a folded graph or a file, each
/// asserted equal to the streaming importer's: the streaming importer, both dataset
/// bridges and the consuming importer. The fold view's tokens are checked by the
/// caller, which knows its term ids.
fn every_dataset_path(
    terms: &[Term],
    quads: &[purrdf_gts::model::Quad],
    rows: &[purrdf_gts::model::ReifierRow],
) -> Vec<String> {
    let (streamed, _) = both_ways(terms, quads, rows);
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(terms);
    writer.add_quads(quads);
    writer.add_reifies(rows);
    let graph = purrdf_gts::reader::read(&writer.into_bytes(), false, None);
    let flattened = projection(&flattened_dataset_from_gts_graph(&graph).expect("flattened"));
    assert_eq!(flattened.0, streamed, "the flattened bridge agrees");
    let consumed = projection(
        &purrdf_rdf::import_gts_graph(graph)
            .expect("consuming")
            .dataset,
    );
    assert_eq!(consumed.0, streamed, "the consuming importer agrees");
    streamed
}

/// Protects the identity of the projected node of a triple term that states no
/// triple (§7.3, "a fresh blank node"; §7.1, a label is local to its scope). A
/// document blank node labelled `unbound_triple_2` is a different node from the
/// bare term `2`: both quads survive, on every dataset path. The neighbouring case
/// with no such label, `a_bare_triple_term_is_one_fresh_blank_node_on_every_path`,
/// still agrees across paths.
#[test]
fn a_document_blank_node_labelled_like_an_unbound_triple_stays_a_distinct_node() {
    let mut bare = Term::triple_term((0, 0, 0));
    bare.triple = None;
    let terms = [
        Term::iri("https://example.org/s"), // 0
        Term::iri("https://example.org/p"), // 1
        bare,                               // 2
        Term::blank("unbound_triple_2"),    // 3: the label the old projection used
        // 4: a label that already spells the projected node's escaped token
        Term::blank("purrdfesc4294967295_unbound_00005Ftriple_00005F2"),
    ];
    let quads = [(2, 1, 0, None), (3, 1, 0, None), (4, 1, 0, None)];
    let streamed = every_dataset_path(&terms, &quads, &[]);
    assert_eq!(
        streamed.len(),
        3,
        "three subjects, three quads: {streamed:?}"
    );
    assert!(
        streamed.contains(
            &"_:purrdfesc4294967295_unbound_triple_2 <https://example.org/p> <https://example.org/s> ."
                .to_owned()
        ),
        "{streamed:?}"
    );
    assert!(
        streamed.contains(
            &"_:unbound_triple_2 <https://example.org/p> <https://example.org/s> .".to_owned()
        ),
        "{streamed:?}"
    );

    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_quads(&quads);
    let graph = purrdf_gts::reader::read(&writer.into_bytes(), false, None);
    let view = GtsFoldView::new(graph).expect("the fold view admits the file");
    assert_ne!(
        view.nq_token(2),
        view.nq_token(3),
        "the fold view's tokens differ"
    );
    assert_ne!(view.nq_token(2), view.nq_token(4));
    assert_ne!(view.nq_token(3), view.nq_token(4));
    assert_eq!(view.nq_token(3), "_:unbound_triple_2");
}

/// Protects §7.3 step 3 for a term whose `rf` names a reifier no `reifies` row
/// binds: it states no triple and projects as a fresh blank node on every path
/// instead of being refused, while a term whose `rf` has a row still resolves
/// through it.
#[test]
fn a_term_naming_a_reifier_with_no_row_is_a_blank_node_beside_one_with_a_row() {
    let terms = [
        Term::iri("https://example.org/s"), // 0
        Term::iri("https://example.org/p"), // 1
        Term::iri("https://example.org/r"), // 2: a reifier with no row
        indirect_triple(2),                 // 3: states no triple
        Term::iri("https://example.org/q"), // 4: a reifier with a row
        indirect_triple(4),                 // 5: resolves through the row
        Term::triple_term((0, 1, 0)),       // 6: a `tt` term, no row under its id
        indirect_triple(6),                 // 7: `rf` names the `tt` term: no row, no triple
    ];
    let quads = [(0, 1, 3, None), (0, 1, 5, None), (0, 1, 7, None)];
    let rows = [(4, (0, 1, 0), None)];
    let streamed = every_dataset_path(&terms, &quads, &rows);
    assert!(
        streamed.contains(
            &"<https://example.org/s> <https://example.org/p> _:purrdfesc4294967295_unbound_triple_3 ."
                .to_owned()
        ),
        "{streamed:?}"
    );
    assert!(
        streamed.contains(
            &"<https://example.org/s> <https://example.org/p> _:purrdfesc4294967295_unbound_triple_7 ."
                .to_owned()
        ),
        "an `rf` naming a `tt` term binds no triple: {streamed:?}"
    );
    assert!(
        streamed.contains(
            &"<https://example.org/s> <https://example.org/p> <<( <https://example.org/s> <https://example.org/p> <https://example.org/s> )>> ."
                .to_owned()
        ),
        "{streamed:?}"
    );

    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_quads(&quads);
    writer.add_reifies(&rows);
    let graph = purrdf_gts::reader::read(&writer.into_bytes(), false, None);
    let view = GtsFoldView::new(graph).expect("the fold view admits the file");
    assert_eq!(
        view.nq_token(3),
        "_:purrdfesc4294967295_unbound_00005Ftriple_00005F3"
    );
    assert_eq!(
        view.nq_token(5),
        "<<( <https://example.org/s> <https://example.org/p> <https://example.org/s> )>>"
    );
    assert_eq!(
        view.nq_token(7),
        "_:purrdfesc4294967295_unbound_00005Ftriple_00005F7"
    );
}
