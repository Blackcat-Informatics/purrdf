// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Byte-exact deterministic-CBOR gate over the frozen cross-engine GTS corpus.

use std::path::PathBuf;

use purrdf_gts::wire::{canonical, iter_items};

#[test]
fn every_decodable_frozen_item_is_canonical_byte_exact() {
    let vectors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vectors");
    let mut paths: Vec<_> = std::fs::read_dir(&vectors)
        .expect("vectors directory")
        .map(|entry| entry.expect("vector entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "gts"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "frozen GTS corpus must not be empty");

    for path in paths {
        let bytes = std::fs::read(&path).expect("read frozen vector");
        let (items, torn) = iter_items(&bytes);
        for (index, (start, value)) in items.iter().enumerate() {
            let end = items
                .get(index + 1)
                .map_or_else(|| torn.unwrap_or(bytes.len()), |(offset, _)| *offset);
            assert_eq!(
                canonical(value),
                bytes[*start..end],
                "canonical bytes drifted for item {index} in {}",
                path.display()
            );
        }
    }
}
