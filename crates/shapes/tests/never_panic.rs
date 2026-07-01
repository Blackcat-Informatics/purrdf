// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! "Reject malformed, never panic" property gate (T7, #788) for the purrdf-shapes
//! shapes frontend.
//!
//! `engine::parse_shapes` parses untrusted SHACL Turtle; given arbitrary input it
//! must return `Ok`/`Err`, never panic. Inputs are bounded so a superlinear
//! parse cannot become a spurious timeout. See `crates/rdf/tests/never_panic.rs`
//! for the contract rationale.

use proptest::prelude::*;
use purrdf_shapes::engine::parse_shapes;

fn arbitrary_bytes() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..4096)
}

/// Structure-aware SHACL Turtle: real `sh:` shape fragments interleaved with
/// noise, to reach the shape-graph interpreter, not just the Turtle lexer.
fn structured_shapes() -> impl Strategy<Value = String> {
    let fragments: Vec<&'static str> = vec![
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
        "@prefix ex: <https://example.org/> .\n",
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
        "ex:S a sh:NodeShape ; sh:targetClass ex:C .\n",
        "ex:S sh:property [ sh:path ex:p ; sh:minCount 1 ] .\n",
        "ex:S sh:property [ sh:path ex:p ; sh:datatype xsd:string ] .\n",
        "ex:S sh:property [ sh:path ex:p ; sh:pattern \"^a+$\" ] .\n",
        "ex:S sh:property [ sh:path ex:p ; sh:minCount \"notanint\" ] .\n",
        "ex:S sh:node ex:S .\n",
        "ex:S sh:property [ sh:path [ sh:inversePath ex:p ] ] .\n",
        "\u{0}\u{1}",
        "ex:S a sh:NodeShape ; sh:property",
        "@prefix sh:",
    ];
    prop::collection::vec(prop::sample::select(fragments), 0..24).prop_map(|parts| parts.concat())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    #[test]
    fn parse_shapes_never_panics_raw(data in arbitrary_bytes()) {
        if let Ok(text) = std::str::from_utf8(&data) {
            let _ = parse_shapes(text);
        }
    }

    #[test]
    fn parse_shapes_never_panics_structured(text in structured_shapes()) {
        let _ = parse_shapes(&text);
    }
}
