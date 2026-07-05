// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! "Reject malformed, never panic" property gate (T7) for the purrdf
//! format frontends.
//!
//! Every parser given arbitrary input must return — `Ok` or `Err` — and NEVER
//! panic/abort. proptest runs each generated input through the parser; a panic is
//! caught and shrunk to the minimal failing input, which becomes a checked-in
//! regression. This is the always-on, portable realization of the contract (runs
//! in the existing `cargo nextest` lane); the `fuzz/` cargo-fuzz crate does the
//! deeper coverage-guided pass nightly.
//!
//! Inputs are BOUNDED (≤4 KiB, modest case count) so a superlinear parser cannot
//! turn a pathological input into a spurious timeout — a panic is a real find, a
//! timeout would be a false red.
//!
//! The parser under test is the native, oxigraph-free [`purrdf_rdf::parse_dataset`]
//! codec: it must return `Ok`/`Err` and NEVER panic on arbitrary input
//! across every text format it accepts (N-Quads / Turtle / TriG / N-Triples).

use proptest::prelude::*;
use purrdf_rdf::{NativeRdfFormat, parse_dataset};

/// Raw arbitrary bytes, bounded to keep parsing cheap.
fn arbitrary_bytes() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..4096)
}

/// Structure-aware text: a random interleaving of real RDF/Turtle fragments and
/// noise, so the generator reaches deep parser states (prefix tables, quoted
/// triples, blank-node scopes) instead of bouncing off the lexer immediately.
fn structured_turtle() -> impl Strategy<Value = String> {
    let fragments: Vec<&'static str> = vec![
        "@prefix ex: <https://example.org/> .\n",
        "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n",
        "ex:a ex:b ex:c .\n",
        "ex:a ex:b \"lit\"@en .\n",
        "ex:a ex:b \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
        "<<ex:a ex:b ex:c>> ex:d ex:e .\n",
        "ex:a ex:b [ ex:c ex:d ] .\n",
        "_:b0 ex:p _:b1 .\n",
        "ex:a ex:b ex:c, ex:d ;\n  ex:e ex:f .\n",
        "\u{0}\u{1}\u{7f}",
        "<not a valid iri> . . ;;",
        "@prefix",
        "\"unterminated",
    ];
    prop::collection::vec(prop::sample::select(fragments), 0..24).prop_map(|parts| parts.concat())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// The native `parse_dataset` codec (every text format) must never panic on
    /// arbitrary bytes — `Ok`/`Err` is fine, a panic is a failure.
    #[test]
    fn native_parse_never_panics(data in arbitrary_bytes()) {
        for format in [
            NativeRdfFormat::NQuads,
            NativeRdfFormat::Turtle,
            NativeRdfFormat::TriG,
            NativeRdfFormat::NTriples,
        ] {
            let _ = parse_dataset(&data, format.media_type(), None);
        }
    }

    /// Structured-Turtle variant: reach deep parser states without timing out.
    #[test]
    fn native_parse_never_panics_structured(text in structured_turtle()) {
        let _ = parse_dataset(text.as_bytes(), NativeRdfFormat::Turtle.media_type(), None);
    }

    /// The GTS container reader must never panic on arbitrary bytes, with or
    /// without multi-segment support.
    #[test]
    fn gts_read_graph_never_panics(data in arbitrary_bytes()) {
        let _ = purrdf_rdf::gts::read_graph(&data, false);
        let _ = purrdf_rdf::gts::read_graph(&data, true);
    }

    /// The SSSOM TSV parser must never panic on arbitrary text.
    #[test]
    fn sssom_parse_tsv_never_panics(data in arbitrary_bytes()) {
        if let Ok(text) = std::str::from_utf8(&data) {
            let _ = purrdf_rdf::sssom::parse_tsv(text);
        }
    }

    /// SSSOM with tab/newline structure (the format's delimiters), so the row
    /// splitter and header logic are exercised, not just the UTF-8 gate.
    #[test]
    fn sssom_parse_tsv_never_panics_structured(
        rows in prop::collection::vec(
            prop::collection::vec("[a-z:#/\\t]{0,12}", 0..8),
            0..32,
        )
    ) {
        let text = rows.into_iter().map(|r| r.join("\t")).collect::<Vec<_>>().join("\n");
        let _ = purrdf_rdf::sssom::parse_tsv(&text);
    }

    /// The RDF-1.2 ↔ OWL statement transforms parse untrusted Turtle; neither
    /// direction may panic.
    #[test]
    fn statements_transforms_never_panic(text in structured_turtle()) {
        let _ = purrdf_rdf::statements::project_owl_to_rdf12(&text);
        let _ = purrdf_rdf::statements::normalize_rdf12_to_owl(&text);
    }

    #[test]
    fn statements_transforms_never_panic_raw(data in arbitrary_bytes()) {
        if let Ok(text) = std::str::from_utf8(&data) {
            let _ = purrdf_rdf::statements::project_owl_to_rdf12(text);
            let _ = purrdf_rdf::statements::normalize_rdf12_to_owl(text);
        }
    }
}
