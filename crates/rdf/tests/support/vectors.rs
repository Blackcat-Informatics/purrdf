// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The frozen GTS corpus under `vectors/`, as the drift guards read it.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::path::PathBuf;

use purrdf_gts::wire::{iter_items, map_get};
use purrdf_lex::cbor::Value;

/// The workspace's frozen `vectors/` directory.
pub fn vectors_dir() -> PathBuf {
    purrdf_testkit::paths::workspace_root().join("vectors")
}

/// The bytes of the frozen vector `name`.
pub fn read_vector(name: &str) -> Vec<u8> {
    std::fs::read(vectors_dir().join(name)).unwrap_or_else(|err| panic!("read {name}: {err}"))
}

/// Whether the file's header item (the first CBOR item, §3.1) carries a
/// non-empty `"dct"` map (§5) — the functional signal that a pack dictionary
/// was actually pinned in-band, not merely that some codec ran.
pub fn header_carries_dct_entry(bytes: &[u8]) -> bool {
    let (items, _torn) = iter_items(bytes);
    let Some((_, first)) = items.first() else {
        return false;
    };
    let inner = match first {
        Value::Tag(_, inner) => inner.as_ref(),
        other => other,
    };
    let Value::Map(entries) = inner else {
        return false;
    };
    matches!(map_get(entries, "dct"), Some(Value::Map(dct)) if !dct.is_empty())
}
