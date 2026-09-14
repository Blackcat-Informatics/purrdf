// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native and wasm execution of the independent BM25F corpus.

use purrdf_text::{FieldInput, Fixed, PreparedCorpus, RankingField, RankingProfile};

fn pairs(text: &str) -> Vec<(u64, u64)> {
    text.split(',')
        .map(|pair| {
            let (left, right) = pair.split_once(':').expect("pair");
            (
                left.parse().expect("integer"),
                right.parse().expect("integer"),
            )
        })
        .collect()
}

/// Compare the complete nonempty reference corpus, one raw integer at a time.
pub(super) fn verify_reference_corpus() {
    let mut vectors = 0;
    for row in include_str!("../reference/bm25f.tsv")
        .lines()
        .filter(|row| !row.starts_with('#'))
    {
        let columns: Vec<&str> = row.split('\t').collect();
        assert_eq!(columns.len(), 7);
        let fields = pairs(columns[3])
            .into_iter()
            .enumerate()
            .map(|(at, (weight, b))| {
                RankingField::new(
                    format!("field-{at}"),
                    Fixed::from_raw(i128::from(weight)),
                    Fixed::from_raw(i128::from(b)),
                )
                .expect("field")
            })
            .collect();
        let profile = RankingProfile::new(fields, Vec::new(), Some(0)).expect("profile");
        let totals: Vec<u128> = columns[2]
            .split(',')
            .map(|value| value.parse().expect("total"))
            .collect();
        let corpus =
            PreparedCorpus::new(&profile, columns[1].parse().expect("population"), &totals)
                .expect("corpus");
        let frequencies: Vec<u64> = columns[4]
            .split(',')
            .map(|value| value.parse().expect("df"))
            .collect();
        let names: Vec<String> = (0..frequencies.len())
            .map(|ordinal| format!("term-{ordinal:04}"))
            .collect();
        let named: Vec<(&str, u64)> = names
            .iter()
            .zip(frequencies)
            .map(|(name, frequency)| (name.as_str(), frequency))
            .collect();
        let query = corpus.prepare_query(&named).expect("query");
        let inputs: Vec<Vec<FieldInput>> = columns[5]
            .split(';')
            .map(|term| {
                pairs(term)
                    .into_iter()
                    .map(|(term_frequency, length)| FieldInput {
                        term_frequency,
                        length,
                    })
                    .collect()
            })
            .collect();
        assert_eq!(
            query.score(&inputs).expect(columns[0]).into_raw(),
            columns[6].parse::<i128>().expect("raw score"),
            "{}",
            columns[0]
        );
        vectors += 1;
    }
    assert_eq!(
        vectors, 15,
        "the reference corpus must never pass vacuously"
    );
}
