// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! YAML-LD emits its keys in sorted order, straight from the carrier.
//!
//! YAML-LD used to reach its output by serializing the whole JSON-LD document to a
//! `String`, reparsing it into a `serde_json::Value`, and converting that. It therefore
//! held the `SerGraph`, the carrier, the JSON text and the value tree at once — strictly
//! more resident than the eager path it replaced, in the one format whose comment said
//! it could not be helped.
//!
//! The reason given was key order: `serde_json`'s map is a `BTreeMap` in this workspace
//! (no `preserve_order` feature), so reparsing SORTS, and the emitted order is a frozen
//! contract. The claim was that the round trip was what produced sorted output.
//!
//! It was not. The carrier already emits in sorted order — `@`-prefixed keys sort ahead
//! of IRI keys, and node properties come out of ordered maps — so the reparse sorted
//! something already sorted. Removing it was verified byte-for-byte against the old
//! path before it was removed, not assumed.
//!
//! This test is what keeps that true. There is no YAML-LD golden, so without it the
//! ordering contract rests on nothing a change would trip over.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, RdfTerm};
use purrdf_rdf::{NativeRdfFormat, serialize_dataset_to_format};

/// Predicates deliberately NOT in alphabetical order, so insertion order and sorted
/// order differ and a regression to insertion order is visible.
fn dataset() -> std::sync::Arc<purrdf_core::RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    for i in 0..8 {
        let s = b.intern_owned_term(&RdfTerm::iri(format!("https://example.org/s{i}")));
        for predicate in ["zeta", "alpha", "middle", "beta"] {
            let p = b.intern_iri(&format!("https://example.org/{predicate}"));
            let o = b.intern_literal(RdfLiteral::simple(format!("value {i} {predicate}")));
            b.push_quad(s, p, o, None);
        }
        let typed = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let class = b.intern_owned_term(&RdfTerm::iri("https://example.org/Class"));
        b.push_quad(s, typed, class, None);
    }
    b.freeze().expect("dataset freezes")
}

/// Within every YAML mapping, sibling keys appear in sorted order.
///
/// Siblings are keys at the same indentation with no shallower line between them, so
/// the comparison is between actual siblings rather than between every key in the
/// document. Deeper levels are discarded on a dedent, and a sequence item (`- `)
/// starts a fresh mapping whose first key sits on the dash line itself.
#[test]
fn yamlld_mapping_keys_are_sorted_within_each_block() {
    let yaml = serialize_dataset_to_format(&*dataset(), NativeRdfFormat::YamlLd, None)
        .expect("YAML-LD serializes");
    let text = String::from_utf8(yaml.bytes).expect("utf-8");

    let mut last: std::collections::BTreeMap<usize, String> = std::collections::BTreeMap::new();
    let mut checked = 0usize;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let dash_indent = line.len() - trimmed.len();
        // A sequence item starts a fresh mapping: every mapping open at or below this
        // level ends, and the item's first key sits two columns in, on the dash line.
        let (indent, body) = trimmed
            .strip_prefix("- ")
            .map_or((dash_indent, trimmed), |rest| {
                last.retain(|level, _| *level < dash_indent);
                (dash_indent + 2, rest)
            });
        // The key is everything before `": "`, or the whole line minus a trailing
        // colon. Splitting on a bare ':' would cut an IRI key at its scheme.
        let Some(key) = body
            .strip_suffix(':')
            .or_else(|| body.split_once(": ").map(|(key, _)| key))
        else {
            continue;
        };
        let key = key.trim().trim_matches('\'').to_owned();
        last.retain(|level, _| *level <= indent);
        if let Some(previous) = last.get(&indent) {
            assert!(
                previous.as_str() <= key.as_str(),
                "YAML-LD emitted `{previous}` before `{key}` among siblings: mapping \
                 keys must stay sorted, which is the contract the removed JSON reparse \
                 used to be credited with enforcing"
            );
            checked += 1;
        }
        last.insert(indent, key);
    }
    assert!(
        checked > 16,
        "only {checked} sibling-key comparisons were made; the fixture is not \
         exercising the ordering this test exists to pin"
    );
}

/// The fixture really does distinguish sorted from insertion order.
///
/// Without this, the test above could pass over a document whose keys happen to be
/// emitted in the order they were inserted — proving nothing. The predicates are
/// inserted zeta, alpha, middle, beta, so a document in insertion order would show
/// `zeta` before `alpha` and the assertion above would fire.
#[test]
fn the_fixture_would_expose_insertion_order() {
    let yaml = serialize_dataset_to_format(&*dataset(), NativeRdfFormat::YamlLd, None)
        .expect("YAML-LD serializes");
    let text = String::from_utf8(yaml.bytes).expect("utf-8");
    let alpha = text
        .find("alpha")
        .expect("the fixture emits its alpha predicate");
    let zeta = text
        .find("zeta")
        .expect("the fixture emits its zeta predicate");
    assert!(
        alpha < zeta,
        "the fixture's predicates were inserted zeta-first; if `zeta` still appears \
         first the document is in insertion order and the sortedness check above is \
         passing for the wrong reason"
    );
}
