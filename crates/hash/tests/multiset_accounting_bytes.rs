// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The byte contract of the additive accounting check of [`Multiset256`]:
//! BLAKE3 of the framed domain, the 16-byte little-endian population, then the
//! 32-byte little-endian sum. Accounting consumers compare these bytes, so the
//! values below are frozen.

use purrdf_hash::Domain;
use purrdf_hash::hex::encode;
use purrdf_hash::multiset::Multiset256;

const ROWS: Domain = Domain::new(b"purrdf-hash/test/rows");
const OTHER: Domain = Domain::new(b"purrdf-hash/test/other");

fn member(seed: u8) -> [u8; 32] {
    let mut bytes = [0_u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = seed.wrapping_mul(31).wrapping_add(i as u8);
    }
    bytes
}

fn set(members: &[u8]) -> Multiset256 {
    let mut value = Multiset256::default();
    for &seed in members {
        value.insert(member(seed)).unwrap();
    }
    value
}

#[test]
fn the_accounting_check_bytes_are_frozen() {
    let cases: [(&str, Multiset256, Domain, &str); 5] = [
        (
            "empty",
            set(&[]),
            ROWS,
            "1a9343ced7155159c356a6ce1fa3bee2ab6418ae515da2f894df28a447da8083",
        ),
        (
            "one member",
            set(&[1]),
            ROWS,
            "8f310916adf9ec397eb04c3e0d496203097df4dc4872c1e7c39c96f08df260ba",
        ),
        (
            "repeated member",
            set(&[1, 1, 1]),
            ROWS,
            "9c71e4c0da4f943e596a0a1459aa6fb2a0a66e1594714e436d8c55071bb215e5",
        ),
        (
            "two members",
            set(&[1, 2]),
            ROWS,
            "2582113491bd5831fc9cd770b8273cb0bd68f79421e812e4cbb1b5a04a192ca0",
        ),
        (
            "second domain",
            set(&[1, 2]),
            OTHER,
            "30548d9cc79c7187ad9797c6f4f96e239018e11d632fe81418fa0e98682b72ae",
        ),
    ];
    for (name, value, domain, expected) in cases {
        assert_eq!(
            encode(&value.accounting_check(domain)),
            expected,
            "accounting bytes of {name}"
        );
    }
}
