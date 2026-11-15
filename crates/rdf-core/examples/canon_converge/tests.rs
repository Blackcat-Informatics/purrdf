// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Behaviour checks of the revision 3.1 labelling.

use crate::r31::{Cert, Input, Labelled, Options, literal, optimised};
use crate::{checks, families, fold_text};

fn both(text: &str) -> [Labelled; 2] {
    let input = Input::read(text).expect("the example reads");
    [
        literal::label_input(&input, Options::DEFAULT).expect("literal labels"),
        optimised::label_input(&input, Options::DEFAULT).expect("optimised labels"),
    ]
}

fn entries(cert: &Cert) -> Vec<(u64, String)> {
    cert.e
        .iter()
        .map(|e| (e.m, String::from_utf8(e.text.to_vec()).expect("UTF-8")))
        .collect()
}

fn position_of(input: &str, done: &Labelled, label: &str) -> u64 {
    let input = Input::read(input).expect("reads");
    let i = input
        .labels
        .iter()
        .position(|l| l == label)
        .expect("a label");
    done.positions[i]
}

fn rows(doc: &str) -> Vec<&str> {
    doc.lines().collect()
}

/// The four hand-worked examples of the text reproduce byte for byte in both
/// modes: entries, path positions, identifiers and spelled lines.
#[test]
fn worked_examples_reproduce_in_both_modes() {
    let path = "_:a <p> _:b .\n_:b <p> _:c .\n_:c <p> _:d .\n";
    for done in both(path) {
        assert_eq!(
            entries(&done.tops[0]),
            [
                (1, "_:0 <p> _:1 .".to_owned()),
                (2, "_:1 <p> _:2 .".to_owned()),
                (3, "_:2 <p> _:3 .".to_owned())
            ]
        );
        for (label, q) in [("a", 0), ("b", 1), ("c", 2), ("d", 3)] {
            assert_eq!(position_of(path, &done, label), q);
        }
    }
    let cycles = "_:a <p> _:b .\n_:b <p> _:c .\n_:c <p> _:a .\n_:x <p> _:y .\n_:y <p> _:z .\n_:z <p> _:x .\n";
    for done in both(cycles) {
        for top in &done.tops {
            assert_eq!(
                entries(top),
                [
                    (1, "_:1 <p> _:0 .".to_owned()),
                    (2, "_:0 <p> _:2 .".to_owned()),
                    (2, "_:2 <p> _:1 .".to_owned())
                ]
            );
        }
        assert_eq!(
            rows(&done.document),
            [
                "_:c14n0 <p> _:c14n2 .",
                "_:c14n1 <p> _:c14n0 .",
                "_:c14n2 <p> _:c14n1 .",
                "_:c14n3 <p> _:c14n5 .",
                "_:c14n4 <p> _:c14n3 .",
                "_:c14n5 <p> _:c14n4 ."
            ]
        );
    }
    let hub =
        "_:h <p> _:l1 .\n_:h <p> _:l2 .\n_:h <p> _:l3 .\n_:l1 <q> <<( _:l2 <r> _:l1 )>> _:h .\n";
    for done in both(hub) {
        assert_eq!(
            entries(&done.tops[0]),
            [
                (1, "_:0 <p> _:1 .".to_owned()),
                (2, "_:0 <p> _:2 .".to_owned()),
                (3, "_:0 <p> _:3 .".to_owned()),
                (3, "_:2 <q> <<( _:3 <r> _:2 )>> _:0 .".to_owned())
            ]
        );
        for (label, q) in [("h", 0), ("l3", 1), ("l1", 2), ("l2", 3)] {
            assert_eq!(position_of(hub, &done, label), q);
        }
    }
    let k22 = "_:a1 <p> _:b1 .\n_:a1 <p> _:b2 .\n_:a2 <p> _:b1 .\n_:a2 <p> _:b2 .\n";
    for done in both(k22) {
        let top = &done.tops[0];
        assert!(top.e.is_empty());
        assert_eq!(top.c.len(), 2);
        for nested in &top.c {
            assert_eq!(
                entries(nested),
                [
                    (2, "_:0 <p> _:2 .".to_owned()),
                    (2, "_:1 <p> _:2 .".to_owned())
                ]
            );
            assert!(nested.c.is_empty());
        }
        assert_eq!(
            rows(&done.document),
            [
                "_:c14n0 <p> _:c14n2 .",
                "_:c14n0 <p> _:c14n3 .",
                "_:c14n1 <p> _:c14n2 .",
                "_:c14n1 <p> _:c14n3 ."
            ]
        );
    }
}

/// The work units of the four examples under the literal mode equal the
/// text's totals (its comparison count follows the method the literal mode
/// uses: children against the best so far in cell order, an insertion sort of
/// components).
#[test]
fn worked_examples_cost_the_texts_units() {
    let cases = [
        ("_:a <p> _:b .\n_:b <p> _:c .\n_:c <p> _:d .\n", 51),
        (
            "_:a <p> _:b .\n_:b <p> _:c .\n_:c <p> _:a .\n_:x <p> _:y .\n_:y <p> _:z .\n_:z <p> _:x .\n",
            267,
        ),
        (
            "_:h <p> _:l1 .\n_:h <p> _:l2 .\n_:h <p> _:l3 .\n_:l1 <q> <<( _:l2 <r> _:l1 )>> _:h .\n",
            79,
        ),
        (
            "_:a1 <p> _:b1 .\n_:a1 <p> _:b2 .\n_:a2 <p> _:b1 .\n_:a2 <p> _:b2 .\n",
            130,
        ),
    ];
    for (text, units) in cases {
        let input = Input::read(text).expect("reads");
        let done = literal::label_input(&input, Options::DEFAULT).expect("labels");
        assert_eq!(done.stats.units(), units, "{text}{:?}", done.stats);
    }
}

/// Optimised equals literal, and the document is invariant under renaming and
/// shuffling, on small instances of each family and on seeded random inputs
/// with triple terms, repeated nodes and blank graph names.
#[test]
fn optimised_equals_literal_and_is_invariant() {
    let folds = [
        families::hubs(3, 2, 2, true),
        families::hubs(3, 2, 2, false),
        families::tree(2, 3),
        families::path(6),
        families::list(5, false),
        families::bipartite(3, true),
        families::bipartite(3, false),
        families::cube(3),
        families::rook(3),
        families::triangles(4),
        families::paley(13),
        families::cfi(4, 0, true),
    ];
    let texts = folds
        .iter()
        .map(fold_text)
        .chain((0..60).map(|seed| checks::random_input(seed, 7, 9)));
    for text in texts {
        let input = Input::read(&text).expect("reads");
        let (report, _) = checks::invariance("case", &input, u64::MAX);
        assert!(report.contains("invariant=yes"), "{report}");
        assert!(report.contains("literal-equal"), "{report}\n{text}");
    }
}

/// A run over its limit refuses and labels nothing; under it, it labels.
#[test]
fn a_work_limit_refuses_instead_of_labelling() {
    let input = Input::read(&fold_text(&families::bipartite(4, true))).expect("reads");
    for limit in [10, u64::MAX] {
        let o = Options {
            limit,
            ..Options::DEFAULT
        };
        assert_eq!(literal::label_input(&input, o).is_ok(), limit > 10);
        assert_eq!(optimised::label_input(&input, o).is_ok(), limit > 10);
    }
}
