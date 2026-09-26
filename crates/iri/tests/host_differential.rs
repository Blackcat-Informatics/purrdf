// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Replays `host_differential_vectors.txt` against [`purrdf_iri::host`].
//!
//! `purrdf-jsonschema` once carried its own RFC 2673 dotted-quad and RFC 4291
//! IPv6 text-form parsers for its `ipv4` and `ipv6` formats, a second address
//! parser beside the one this crate validates IP literals with. Before they
//! were deleted in favour of [`is_ipv4_address`] and [`is_ipv6_address`],
//! their verdicts over 20,000 seeded address-shaped strings were frozen here
//! (answers only, never code; see `PROVENANCE.md`). Every record must agree:
//! a disagreement is a defect in `purrdf_iri::host`, resolved against the RFC
//! text, and never a reason to edit a vector.

use purrdf_iri::host::{Mode, is_ipv4_address, is_ipv6_address, is_reg_name};
use purrdf_testkit::vectors::{VectorFile, decode_str};

const VECTORS: &str = include_str!("host_differential_vectors.txt");

const fn verdict(accepted: bool) -> &'static str {
    if accepted { "accept" } else { "reject" }
}

#[test]
fn every_frozen_address_verdict_is_reproduced() {
    let vectors = VectorFile::parse(VECTORS).expect("the vector file is intact");
    assert_eq!(vectors.records().len(), 20_000);
    let replayed = vectors
        .replay(1, |fields| {
            let input = decode_str(fields[0]).expect("a text field");
            vec![
                verdict(is_ipv4_address(&input)).to_owned(),
                verdict(is_ipv6_address(&input)).to_owned(),
            ]
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, 20_000);
}

#[test]
fn the_table_exercises_both_verdicts_of_both_productions() {
    let vectors = VectorFile::parse(VECTORS).expect("the vector file is intact");
    let mut seen = [[0_usize; 2]; 2];
    for record in vectors.records() {
        for (production, field) in record.fields[1..].iter().enumerate() {
            seen[production][usize::from(*field == "accept")] += 1;
        }
    }
    for (production, name) in ["ipv4", "ipv6"].into_iter().enumerate() {
        assert!(
            seen[production].iter().all(|&count| count >= 1_000),
            "{name} verdicts are one-sided: {:?}",
            seen[production]
        );
    }
}

#[test]
fn an_address_the_table_accepts_is_a_host_the_parser_accepts() {
    let vectors = VectorFile::parse(VECTORS).expect("the vector file is intact");
    for record in vectors.records() {
        let input = decode_str(record.fields[0]).expect("a text field");
        if record.fields[1] == "accept" {
            assert!(is_reg_name(&input, Mode::Uri), "{input:?} is a reg-name");
            let uri = format!("http://{input}/");
            assert!(purrdf_iri::parse_uri(&uri).is_ok(), "{uri:?} parses");
        }
        if record.fields[2] == "accept" {
            let uri = format!("http://[{input}]/");
            assert!(purrdf_iri::parse_uri(&uri).is_ok(), "{uri:?} parses");
        } else if !input.is_empty() && !input.starts_with(['v', 'V']) {
            let uri = format!("http://[{input}]/");
            assert!(
                purrdf_iri::parse_uri(&uri).is_err(),
                "{uri:?} is refused as an IP-literal"
            );
        }
    }
}
