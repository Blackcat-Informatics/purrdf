// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generates the compact HTML named-reference data table, to stdout.
//!
//! Run `cargo run -p purrdf-lex --example gen_html_entities --locked`, then
//! format the output with rustfmt. The input is independently pinned machine
//! data; see `data/html/PROVENANCE.md`. No upstream implementation is used.

use purrdf_lex::json::{self, Value};
use std::fmt::Write as _;

const DATA: &str = include_str!("../data/html/entities.json");
const NUMERIC: &str = include_str!("../data/html/numeric-recovery.json");

fn main() {
    let Value::Object(ref object) = json::read(DATA).expect("valid pinned JSON") else {
        panic!("named-reference data must be an object");
    };
    let mut rows = Vec::with_capacity(object.len());
    for (name, value) in object.iter() {
        let name = name.strip_prefix('&').expect("reference starts with &");
        let body = name.strip_suffix(';').unwrap_or(name);
        assert!(!body.is_empty() && body.bytes().all(|byte| byte.is_ascii_alphanumeric()));
        let Value::Object(value) = value else {
            panic!("reference value must be an object");
        };
        assert_eq!(value.len(), 2);
        let characters = value
            .get("characters")
            .and_then(Value::as_str)
            .expect("characters string");
        let Value::Array(points) = value.get("codepoints").expect("codepoints array") else {
            panic!("codepoints must be an array");
        };
        let chars: Vec<char> = characters.chars().collect();
        assert!((1..=2).contains(&chars.len()));
        assert_eq!(chars.len(), points.len());
        for (&character, point) in chars.iter().zip(points) {
            assert_eq!(point.as_u64(), Some(u64::from(character)));
            assert_ne!(character, '\0', "NUL is the absent second-scalar sentinel");
        }
        rows.push((name, chars[0], chars.get(1).copied().unwrap_or('\0')));
    }
    rows.sort_unstable_by_key(|row| row.0);
    assert!(rows.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert_eq!(rows.len(), 2_231);
    assert_eq!(rows.iter().filter(|row| !row.0.ends_with(';')).count(), 106);
    for &(name, first, second) in &rows {
        if !name.ends_with(';') {
            let terminated = format!("{name};");
            let index = rows
                .binary_search_by_key(&terminated.as_str(), |row| row.0)
                .expect("every legacy name has a terminated spelling");
            assert_eq!((rows[index].1, rows[index].2), (first, second));
        }
    }

    let mut names = String::new();
    let mut entries = String::new();
    for &(name, first, second) in &rows {
        let start = u16::try_from(names.len()).expect("name arena fits u16");
        let len = u8::try_from(name.len()).expect("name length fits u8");
        names.push_str(name);
        writeln!(
            entries,
            "Entry {{ start: {start}, len: {len}, first: {first:?}, second: {second:?} }},"
        )
        .expect("write to string");
    }
    assert!(u16::try_from(names.len()).is_ok());
    let Value::Array(ref numeric) = json::read(NUMERIC).expect("valid numeric recovery data")
    else {
        panic!("numeric recovery must be an array");
    };
    assert_eq!(numeric.len(), 32);
    let numeric: Vec<u32> = numeric
        .iter()
        .map(|value| {
            let point = u32::try_from(value.as_u64().expect("numeric code point")).expect("u32");
            assert!(char::from_u32(point).is_some());
            point
        })
        .collect();
    println!(
        "// SPDX-FileCopyrightText: WHATWG (Apple, Google, Mozilla, Microsoft)\n\
         // SPDX-License-Identifier: BSD-3-Clause\n\n\
         // Generated DATA by examples/gen_html_entities.rs. Do not edit.\n\
         // Exact input and notices: data/html/PROVENANCE.md.\n\n\
         use super::Entry;\n\n\
         pub(super) const DATA_BLAKE3: [u8; 32] = {:?};\n\
         pub(super) const NUMERIC_BLAKE3: [u8; 32] = {:?};\n\
         pub(super) const C1_REPLACEMENTS: [u32; 32] = {numeric:?};\n\
         pub(super) const MAX_NAME_LEN: usize = {};\n\
         pub(super) const NAMES: &[u8] = b{:?};\n\
         pub(super) const ENTRIES: &[Entry] = &[\n{entries}];",
        purrdf_hash::blake3::hash(DATA.as_bytes()).as_bytes(),
        purrdf_hash::blake3::hash(NUMERIC.as_bytes()).as_bytes(),
        rows.iter().map(|row| row.0.len()).max().expect("names"),
        names,
    );
}
