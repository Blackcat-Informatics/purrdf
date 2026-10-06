// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! Annotated forms satisfy the official normalization conformance columns.
use purrdf_lex::unicode::{TaggedScalar, compose_tagged, decompose_tagged};
use std::iter::Extend;
fn sequence(hex: &str) -> String {
    hex.split_whitespace()
        .map(|hex| char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap())
        .collect()
}
fn normalize<const COMPAT: bool>(text: &str, compose: bool) -> String {
    let mut scalars = text
        .chars()
        .enumerate()
        .map(|(at, value)| TaggedScalar {
            value,
            metadata: vec![at],
        })
        .collect();
    let mut scratch = Vec::new();
    decompose_tagged::<COMPAT, _>(&mut scalars, &mut scratch);
    if compose {
        compose_tagged(&mut scalars, &mut scratch, |left, mut right| {
            left.append(&mut right);
            left.sort_unstable();
            left.dedup();
        });
    }
    scalars.iter().map(|scalar| scalar.value).collect()
}
#[test]
fn official_all_columns() {
    let data = include_str!("../../iri/unicode/17.0.0/NormalizationTest.txt");
    let mut count = 0;
    for line in data.lines() {
        let row = line.split('#').next().unwrap().trim();
        if row.is_empty() || row.starts_with('@') {
            continue;
        }
        let columns = row.split(';').take(5).map(sequence).collect::<Vec<_>>();
        for input in &columns[..3] {
            assert_eq!(normalize::<false>(input, false), columns[2]);
            assert_eq!(normalize::<false>(input, true), columns[1]);
        }
        for input in &columns[3..] {
            assert_eq!(normalize::<false>(input, false), columns[4]);
            assert_eq!(normalize::<false>(input, true), columns[3]);
        }
        for input in &columns {
            assert_eq!(normalize::<true>(input, false), columns[4]);
            assert_eq!(normalize::<true>(input, true), columns[3]);
        }
        count += 1;
    }
    assert_eq!(count, 20034);
}
#[test]
fn reorder_and_composition_keep_actual_contributors() {
    let mut scalars = "A\u{315}\u{300}"
        .chars()
        .enumerate()
        .map(|(at, value)| TaggedScalar {
            value,
            metadata: vec![at],
        })
        .collect();
    let mut scratch = Vec::new();
    decompose_tagged::<false, _>(&mut scalars, &mut scratch);
    assert_eq!(
        scalars
            .iter()
            .map(|scalar| scalar.metadata[0])
            .collect::<Vec<_>>(),
        [0, 2, 1]
    );
    compose_tagged(&mut scalars, &mut scratch, Extend::extend);
    assert_eq!(scalars[0].value, 'À');
    assert_eq!(scalars[0].metadata, [0, 2]);
    assert_eq!(scalars[1].metadata, [1]);
}
