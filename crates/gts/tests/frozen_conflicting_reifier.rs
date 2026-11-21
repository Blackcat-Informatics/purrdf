// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The frozen vector `vectors/12-conflicting-reifier` against this reader.
//!
//! `vectors/12-conflicting-reifier.gts` is a single generic-profile segment
//! holding five terms and no quads. Its `reifies` frame binds ONE reifier id,
//! `https://example.org/r1`, to TWO different triples:
//!
//! * `<https://example.org/Cat> rdfs:label "Cat"@en`
//! * `<https://example.org/Cat> rdfs:label "Chat"@fr`
//!
//! The file contains no quoted-triple term, so no `"tt"`-less `k:3` term has a
//! meaning that could depend on which binding wins. `rdf:reifies` is not a
//! functional property: a reader retains every row bound to a reifier id, and
//! `ConflictingReifier` is raised only for a `"tt"`-less `k:3` term whose reifier
//! binds several triples (covered by `tests/multi_binding_reifier.rs`). This
//! reader therefore folds the vector to two reifier rows and raises nothing, and
//! the expectation that ships beside the vector states the same.

use purrdf_gts::model::{Graph, TermKind};
use purrdf_gts::reader::read;

/// The frozen vector under test, without extension.
const VECTOR: &str = "12-conflicting-reifier";

/// Fold the frozen vector through the real reader, exactly as a consumer would.
fn fold_frozen_vector() -> Graph {
    let path = purrdf_testkit::paths::workspace_root!()
        .join("vectors")
        .join(format!("{VECTOR}.gts"));
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("read frozen vector {}: {error}", path.display()));
    read(&bytes, true, None)
}

/// Render one reifier row as `(reifier, subject, predicate, object, graph?)`
/// using each term's human-readable spelling, so an assertion failure names the
/// data rather than a term id.
fn describe_reifier_rows(graph: &Graph) -> Vec<(String, String, String, String, Option<String>)> {
    let spell = |id: usize| -> String {
        let term = &graph.terms[id];
        let value = term.value.clone().unwrap_or_default();
        match (term.kind, term.lang.as_deref()) {
            (TermKind::Literal, Some(lang)) => format!("\"{value}\"@{lang}"),
            (TermKind::Literal, None) => format!("\"{value}\""),
            (TermKind::Bnode, _) => format!("_:{value}"),
            _ => value,
        }
    };
    graph
        .reifiers
        .iter()
        .map(|(reifier, (subject, predicate, object), by)| {
            (
                spell(*reifier),
                spell(*subject),
                spell(*predicate),
                spell(*object),
                by.map(spell),
            )
        })
        .collect()
}

const CAT: &str = "https://example.org/Cat";
const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const R1: &str = "https://example.org/r1";

/// Both bindings survive, with their content and their (default) graph slot,
/// and nothing is reported.
#[test]
fn purrdf_folds_the_frozen_vector_to_two_bindings_and_no_diagnostic() {
    let graph = fold_frozen_vector();

    assert_eq!(
        describe_reifier_rows(&graph),
        vec![
            (
                R1.to_string(),
                CAT.to_string(),
                LABEL.to_string(),
                "\"Cat\"@en".to_string(),
                None,
            ),
            (
                R1.to_string(),
                CAT.to_string(),
                LABEL.to_string(),
                "\"Chat\"@fr".to_string(),
                None,
            ),
        ],
        "one reifier id bound to two distinct triples must keep BOTH rows",
    );

    assert!(
        graph.diagnostics.is_empty(),
        "a multi-valued statement layer is legitimate RDF 1.2, not a defect: {:?}",
        graph.diagnostics,
    );

    // The premise behind the surviving `ConflictingReifier` guard: the vector
    // has no quoted-triple term whose meaning the two bindings could split.
    assert!(
        graph.terms.iter().all(|term| term.kind != TermKind::Triple),
        "the vector encodes no quoted-triple term, so no term is made ambiguous",
    );

    // The rest of the fold, so a drift anywhere in this vector is caught here.
    assert_eq!(graph.terms.len(), 5, "term count");
    assert!(graph.quads.is_empty(), "the vector carries no base quads");
    assert!(
        graph.annotations.is_empty(),
        "the vector carries no annotations"
    );
    assert!(
        graph.suppressions.is_empty(),
        "the vector suppresses nothing"
    );
    assert_eq!(
        graph.segment_profiles,
        vec!["generic".to_string()],
        "profile"
    );
    assert_eq!(graph.segment_heads.len(), 1, "segment count");
}
