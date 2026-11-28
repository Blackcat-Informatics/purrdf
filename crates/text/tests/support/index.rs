// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Index fixtures: a configuration, the default partition, and an index's
//! documents read back.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use purrdf_core::TermValue;
use purrdf_text::{GraphSelector, PartitionKey, TextIndex, TextIndexConfig};

/// A configuration over `predicates` covering every graph.
pub fn config(predicates: &[&str]) -> TextIndexConfig {
    TextIndexConfig::new(
        predicates.iter().map(|p| TermValue::iri(*p)).collect(),
        GraphSelector::Any,
        purrdf_text::Analyzer::empty_lexicon(),
    )
    .expect("the fixture configurations are well formed")
}

/// The subjects of every document of `index`, in id order.
pub fn subjects(index: &TextIndex) -> Vec<TermValue> {
    index
        .documents()
        .map(|document| document.subject().clone())
        .collect()
}

/// The default-graph, untagged partition — the one most fixtures live in.
pub fn plain() -> PartitionKey {
    PartitionKey::new(None, None)
}
