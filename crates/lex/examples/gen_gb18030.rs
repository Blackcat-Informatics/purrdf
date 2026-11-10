// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generate GB 18030 tables from licensed, pinned mapping DATA.
//! Run before wiring the generated module: cargo run -p purrdf-lex --example gen_gb18030.

use std::fmt::Write as _;

const BASE: &str = include_str!("../data/gb18030/gbkuni30.txt");
const REFERENCE: &str = include_str!("../data/gb18030/gb18030-2005.ucm");

// Annex D.1, reproduced in Unicode L2/22-274R Annex A. Each row swaps
// both directions; retaining the old four-byte target would create an alias.
const SWAPS: [(u32, u32, u32, u32); 18] = [
    (0xa6d9, 0x8431_8236, 0xe78d, 0xfe10),
    (0xa6da, 0x8431_8238, 0xe78e, 0xfe12),
    (0xa6db, 0x8431_8237, 0xe78f, 0xfe11),
    (0xa6dc, 0x8431_8239, 0xe790, 0xfe13),
    (0xa6dd, 0x8431_8330, 0xe791, 0xfe14),
    (0xa6de, 0x8431_8331, 0xe792, 0xfe15),
    (0xa6df, 0x8431_8332, 0xe793, 0xfe16),
    (0xa6ec, 0x8431_8333, 0xe794, 0xfe17),
    (0xa6ed, 0x8431_8334, 0xe795, 0xfe18),
    (0xa6f3, 0x8431_8335, 0xe796, 0xfe19),
    (0xfe59, 0x8235_9037, 0xe81e, 0x9fb4),
    (0xfe61, 0x8235_9038, 0xe826, 0x9fb5),
    (0xfe66, 0x8235_9039, 0xe82b, 0x9fb6),
    (0xfe67, 0x8235_9130, 0xe82c, 0x9fb7),
    (0xfe6d, 0x8235_9131, 0xe832, 0x9fb8),
    (0xfe7e, 0x8235_9132, 0xe843, 0x9fb9),
    (0xfe90, 0x8235_9133, 0xe854, 0x9fba),
    (0xfea0, 0x8235_9134, 0xe864, 0x9fbb),
];

fn pinned(text: &str, expected: &str) {
    let mut hash = purrdf_hash::sha1::Sha1::new();
    hash.update(format!("blob {}\0", text.len()).as_bytes());
    hash.update(text.as_bytes());
    assert_eq!(
        format!("{}", purrdf_hash::hex::Lower(&hash.finalize())),
        expected
    );
}

fn wire4(pointer: u32) -> u32 {
    let fourth = pointer % 10 + 0x30;
    let third = pointer / 10 % 126 + 0x81;
    let second = pointer / 1260 % 10 + 0x30;
    let first = pointer / 12600 + 0x81;
    first << 24 | second << 16 | third << 8 | fourth
}

fn overrides(current: bool) -> Vec<(u32, u32)> {
    let mut rows = vec![(0xa8bc, 0x1e3f), (0x8135_f437, 0xe7c7)];
    if current {
        for (two, four, old, new) in SWAPS {
            rows.extend([(two, new), (four, old)]);
        }
    }
    rows.sort_unstable();
    rows
}

fn main() {
    pinned(BASE, "ff332150043bcb0e4a9beae2b54153230189f22f");
    pinned(REFERENCE, "62b57a93fc24ccb400f75d6d7c02c5e2852382b5");
    let mut by_scalar = vec![None; 0x1_0000];
    let mut two = vec![None; 23940];
    for line in BASE.lines() {
        let (scalar, code) = line.split_once(':').expect("mapping separator");
        let scalar = u32::from_str_radix(scalar, 16).expect("Unicode scalar");
        let code = u32::from_str_radix(code, 16).expect("GB code");
        assert!(char::from_u32(scalar).is_some());
        assert!(by_scalar[scalar as usize].replace(code).is_none());
        if code < 0x80 {
            assert_eq!(scalar, code);
        } else {
            let [first, last] = u16::try_from(code).expect("two-byte code").to_be_bytes();
            assert!((0x81..=0xfe).contains(&first));
            assert!((0x40..=0xfe).contains(&last) && last != 0x7f);
            let index = usize::from(first - 0x81) * 190
                + usize::from(last - if last < 0x7f { 0x40 } else { 0x41 });
            assert!(two[index].replace(scalar as u16).is_none());
        }
    }
    assert_eq!(BASE.lines().count(), 24068);
    let two: Vec<_> = two
        .into_iter()
        .map(|v| v.expect("every pair mapped"))
        .collect();
    // Freeze the original complement BEFORE applying either edition's swaps.
    let missing: Vec<_> = (0u32..0x1_0000)
        .filter(|&scalar| char::from_u32(scalar).is_some() && by_scalar[scalar as usize].is_none())
        .collect();
    assert_eq!(missing.len(), 39420);
    let legacy = overrides(false);
    let current = overrides(true);
    let mut reference_map = std::collections::BTreeMap::new();
    for (scalar, code) in by_scalar.iter().enumerate() {
        if let Some(code) = code {
            reference_map.insert(*code, scalar as u32);
        }
    }
    for (pointer, &scalar) in missing.iter().enumerate() {
        reference_map.insert(wire4(pointer as u32), scalar);
    }
    reference_map.extend(legacy.iter().copied());
    let mut legacy_reference = 0;
    for line in REFERENCE.lines().filter(|line| line.starts_with("<U")) {
        let mut fields = line.split_whitespace();
        let scalar = fields.next().expect("scalar");
        let scalar = u32::from_str_radix(&scalar[2..scalar.len() - 1], 16).expect("scalar hex");
        let encoded = fields.next().expect("encoding");
        assert_eq!(
            fields.next(),
            Some("|0"),
            "only reversible reference mappings"
        );
        let code = encoded.split("\\x").skip(1).fold(0u32, |code, byte| {
            code << 8 | u32::from_str_radix(byte, 16).expect("byte hex")
        });
        let actual = reference_map.get(&code).copied();
        assert_eq!(
            actual,
            Some(scalar),
            "reference U+{scalar:04X} code {code:X}"
        );
        legacy_reference += 1;
    }
    assert!(legacy_reference > 30000);
    for (pair, four, old, new) in SWAPS {
        assert_eq!(scalar_for_pair(&two, pair), old);
        assert!(
            missing
                .iter()
                .enumerate()
                .any(|(p, &s)| s == new && wire4(p as u32) == four)
        );
    }
    for (code, scalar) in [
        (0x8139_a332, 0x2ffc),
        (0x8139_a333, 0x2ffd),
        (0x8139_a334, 0x2ffe),
        (0x8139_a335, 0x2fff),
        (0x8139_bc33, 0x31ef),
    ] {
        assert!(
            missing
                .iter()
                .enumerate()
                .any(|(p, &s)| s == scalar && wire4(p as u32) == code)
        );
    }
    let mut inverse: Vec<usize> = (0..two.len()).collect();
    inverse.sort_unstable_by_key(|&i| two[i]);
    assert!(inverse.windows(2).all(|pair| two[pair[0]] < two[pair[1]]));
    let mut out = String::from(
        "// SPDX-FileCopyrightText: 2000-2007 International Business Machines Corporation and others.\n// SPDX-FileCopyrightText: 2016-2024 Unicode, Inc.\n// SPDX-License-Identifier: Unicode-3.0\n\n// Generated by examples/gen_gb18030.rs; do not edit.\n// Original notices and source identities: data/gb18030/PROVENANCE.md.\n\n",
    );
    for (name, values) in [
        ("TWO", two.iter().map(|&v| u32::from(v)).collect::<Vec<_>>()),
        ("TWO_INVERSE", inverse.iter().map(|&v| v as u32).collect()),
    ] {
        writeln!(out, "pub(super) static {name}: [u16; {}] = [", values.len())
            .expect("string output");
        for row in values.chunks(16) {
            for value in row {
                write!(out, "0x{value:04x},").expect("string output");
            }
            out.push('\n');
        }
        out.push_str("];\n");
    }
    out.push_str("pub(super) const FOUR: &[(u32, u32)] = &[\n");
    for (pointer, &scalar) in missing.iter().enumerate() {
        if pointer == 0 || missing[pointer - 1] + 1 != scalar {
            writeln!(out, "({pointer},0x{scalar:x}),").expect("string output");
        }
    }
    out.push_str("];\n");
    for (name, rows) in [("LEGACY", legacy), ("CURRENT", current)] {
        for (suffix, mut values) in [
            ("DECODE", rows.clone()),
            ("ENCODE", rows.into_iter().map(|(a, b)| (b, a)).collect()),
        ] {
            values.sort_unstable();
            writeln!(out, "pub(super) const {name}_{suffix}: &[(u32,u32)] = &[")
                .expect("string output");
            for (a, b) in values {
                writeln!(out, "({},{}),", rust_hex(a), rust_hex(b)).expect("string output");
            }
            out.push_str("];\n");
        }
    }
    print!("{out}");
}

fn scalar_for_pair(two: &[u16], code: u32) -> u32 {
    if code < 0x80 {
        return code;
    }
    let [first, last] = (code as u16).to_be_bytes();
    let index =
        usize::from(first - 0x81) * 190 + usize::from(last - if last < 0x7f { 0x40 } else { 0x41 });
    u32::from(two[index])
}

fn rust_hex(value: u32) -> String {
    if value > 0xffff {
        format!("0x{:x}_{:04x}", value >> 16, value & 0xffff)
    } else {
        format!("0x{value:x}")
    }
}
