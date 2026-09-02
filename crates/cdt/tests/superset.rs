// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exactly which lexical forms are PurRDF supersets of SEP-0009, and which are not.
//!
//! # Why this file exists
//!
//! `purrdf-cdt` widens SEP-0009's `Element` production twice — RDF 1.2 triple terms
//! (`<<( s p o )>>`) and directional language-tagged literals (`"lex"@lang--ltr` /
//! `--rtl`). Under the published SEP-0009 grammar both forms are ill-typed, so both
//! are real divergences.
//!
//! SEP-0009's own conformance corpus cannot grade them: it contains no `<<(` and no
//! `--ltr` / `--rtl` anywhere, so a green corpus run says nothing at all about the
//! divergence. An ungraded divergence is an invisible one, and this file is the
//! grading. Every widened form gets two assertions — that it parses, renders and
//! round-trips byte-exactly, **and** that the crate classifies it as outside strict
//! SEP-0009 — and every neighbouring form that *is* strictly conformant gets the
//! matching negative assertion, so the boundary is pinned from both sides.
//!
//! Every fixture uses `example.org`.

use pretty_assertions::assert_eq;
use purrdf_cdt::{
    CdtLiteral, CdtTerm, CdtValue, LexicalSpace, TextDirection, key_lexical_space, lexical_space,
    literal_lexical_space, parse_list, parse_map, term_lexical_space,
};

/// Parse, render, re-parse: the form survives the canonical mapping byte-exactly, and
/// the value it denotes is unchanged.
fn assert_round_trips(lexical: &str) -> CdtValue {
    let first = parse_list(lexical).expect("the lexical form is well formed");
    let canonical = first.canonical_lexical();
    let second = parse_list(&canonical).expect("the canonical form re-parses");
    assert_eq!(
        second.canonical_lexical(),
        canonical,
        "the canonical form of {lexical} is not a fixpoint"
    );
    assert_eq!(second, first, "{lexical} does not survive the round trip");
    first
}

// ── The two forms that ARE PurRDF supersets ───────────────────────────────────

/// Superset 1, the whole production: `TripleTerm ::= '<<(' Element Element Element ')>>'`.
#[test]
fn superset_1_a_triple_term_parses_renders_and_round_trips() {
    let lexical = "[<<(<http://example.org/s> <http://example.org/p> 1)>>]";
    let value = assert_round_trips(lexical);
    assert_eq!(
        value.canonical_lexical(),
        "[<<(<http://example.org/s> <http://example.org/p> \
         \"1\"^^<http://www.w3.org/2001/XMLSchema#integer>)>>]"
    );
}

/// Superset 1 is reported as a superset, wherever it appears.
#[test]
fn superset_1_is_outside_strict_sep_0009() {
    let flat = parse_list("[<<(<http://example.org/s> <http://example.org/p> 1)>>]").unwrap();
    assert_eq!(lexical_space(&flat), LexicalSpace::PurrdfSuperset);
    assert!(flat.uses_extension());

    // Nested inside a list, inside a map value, and inside another triple term: the
    // classification is a property of the value, not of where the form sits.
    let nested = parse_list("[1, [ 2, <<(_:b <http://example.org/p> null)>> ]]").unwrap();
    assert!(nested.uses_extension());
    let in_a_map = parse_map("{\"k\": <<(_:b <http://example.org/p> null)>>}").unwrap();
    assert!(in_a_map.uses_extension());
    let in_a_triple = parse_list(
        "[<<(<http://example.org/s> <http://example.org/p> \
         <<(<http://example.org/s> <http://example.org/p> 1)>>)>>]",
    )
    .unwrap();
    assert!(in_a_triple.uses_extension());

    // …and the element-level answer agrees.
    let items = flat.into_list().expect("a cdt:List");
    assert_eq!(term_lexical_space(&items[0]), LexicalSpace::PurrdfSuperset);
    assert!(items[0].uses_extension());
}

/// Superset 2, both directions: `LANGTAG` gains RDF 1.2's `'--' ('ltr' | 'rtl')`.
#[test]
fn superset_2_directional_literals_parse_render_and_round_trip_in_both_directions() {
    let ltr = assert_round_trips("[\"hello\"@en--ltr]");
    assert_eq!(ltr.canonical_lexical(), "[\"hello\"@en--ltr]");

    let rtl = assert_round_trips("[\"\u{645}\u{631}\u{62d}\u{628}\u{627}\"@ar--rtl]");
    assert_eq!(
        rtl.canonical_lexical(),
        "[\"\u{645}\u{631}\u{62d}\u{628}\u{627}\"@ar--rtl]"
    );

    // The direction rides after a multi-subtag language tag too.
    let subtags = assert_round_trips("[\"x\"@en-GB--rtl]");
    assert_eq!(subtags.canonical_lexical(), "[\"x\"@en-GB--rtl]");
}

/// Superset 2 is reported as a superset, in either direction and in key position.
#[test]
fn superset_2_is_outside_strict_sep_0009() {
    for lexical in [
        "[\"hello\"@en--ltr]",
        "[\"hello\"@en--rtl]",
        "[\"x\"@en-GB--ltr]",
    ] {
        let value = parse_list(lexical).unwrap();
        assert_eq!(
            lexical_space(&value),
            LexicalSpace::PurrdfSuperset,
            "{lexical} should be reported as a PurRDF extension"
        );
    }

    // A directional literal as a MAP KEY is classified through the key path.
    let keyed = parse_map("{\"k\"@he--rtl: 1}").unwrap();
    assert!(keyed.uses_extension());
    let key = keyed.keys().next().expect("one key").clone();
    assert_eq!(key_lexical_space(&key), LexicalSpace::PurrdfSuperset);

    // The literal-level answer agrees, and a hand-built `rdf:dirLangString` that
    // carries the datatype without a parsed direction is classified the same way.
    let built = CdtLiteral::dir_lang("hello", "en", TextDirection::Rtl);
    assert_eq!(literal_lexical_space(&built), LexicalSpace::PurrdfSuperset);
    let datatype_only = CdtLiteral::typed("hello", purrdf_cdt::RDF_DIR_LANG_STRING);
    assert_eq!(
        literal_lexical_space(&datatype_only),
        LexicalSpace::PurrdfSuperset
    );
}

// ── Everything the published grammar does admit ───────────────────────────────

/// One assertion per SEP-0009 `Element` alternative, plus the map productions: each is
/// strictly conformant, so the crate must NOT report it as an extension.
///
/// This is the other half of the boundary. Without it, a classifier that answered
/// `PurrdfSuperset` for everything would pass the tests above.
#[test]
fn every_published_element_alternative_stays_inside_strict_sep_0009() {
    let strict = [
        // [1] / [4] — the empty forms.
        "[]",
        // [3] ListElement, one per alternative.
        "[<http://example.org/i>]",
        "[_:b0]",
        "[\"lit\"]",
        "[\"lit\"^^<http://example.org/t>]",
        "[1]",
        "[1.5]",
        "[1e0]",
        "[true]",
        "[false]",
        "[null]",
        "[[1]]",
        "[{\"k\": 1}]",
        // [128s] RDFLiteral with a plain LANGTAG — the base form the direction
        // suffix extends. This is the closest strictly-conformant neighbour of
        // superset 2, and it must land on the other side of the line.
        "[\"hello\"@en]",
        "[\"hello\"@en-GB]",
        // All four String forms.
        "['abc']",
        "[\"\"\"abc\"\"\"]",
        "['''abc''']",
        // Nested to depth, with everything above mixed together.
        "[1, 'a'@en, [true, {\"k\": [null, <http://example.org/i>]}], _:b]",
    ];
    for lexical in strict {
        let value = parse_list(lexical).expect("the lexical form is well formed");
        assert_eq!(
            lexical_space(&value),
            LexicalSpace::Sep0009,
            "{lexical} is strictly conformant and must not be reported as an extension"
        );
        assert!(!value.uses_extension());
    }

    // The map productions, from the map side.
    for lexical in [
        "{}",
        "{<http://example.org/k>: 1}",
        "{\"s\": 2}",
        "{3: 4}",
        "{true: 5}",
        "{\"l\"@en: 6}",
        "{\"k\": {\"j\": [1, null]}}",
    ] {
        let value = parse_map(lexical).expect("the lexical form is well formed");
        assert_eq!(
            lexical_space(&value),
            LexicalSpace::Sep0009,
            "{lexical} is strictly conformant and must not be reported as an extension"
        );
    }

    // Leaf elements, one by one.
    for term in [
        CdtTerm::Null,
        CdtTerm::Iri("http://example.org/i".into()),
        CdtTerm::Blank("b0".into()),
        CdtTerm::Literal(CdtLiteral::plain("x")),
        CdtTerm::Literal(CdtLiteral::lang("x", "en")),
    ] {
        assert_eq!(
            term_lexical_space(&term),
            LexicalSpace::Sep0009,
            "{term:?} is strictly conformant"
        );
        assert!(!term.uses_extension());
    }
    assert!(!CdtValue::empty_list().uses_extension());
    assert!(!CdtValue::empty_map().uses_extension());
}

/// A value that a *strict* SEP-0009 implementation could not have produced is exactly
/// the set of values that report [`LexicalSpace::PurrdfSuperset`] — and a value
/// PurRDF merely *carries* is written in SEP-0009's own space whenever it can be.
///
/// The second half is the conformance argument in the crate docs, run as code: minting
/// a list of ordinary terms never reaches for a widened form, so the extension is only
/// ever emitted for a term SEP-0009 cannot express at all.
#[test]
fn the_extension_is_only_emitted_for_terms_sep_0009_cannot_express() {
    let ordinary = CdtValue::list(vec![
        CdtTerm::Literal(CdtLiteral::lang("hello", "en")),
        CdtTerm::Iri("http://example.org/i".into()),
        CdtTerm::Blank("b".into()),
        CdtTerm::Null,
    ])
    .expect("within every bound");
    assert!(!ordinary.uses_extension());
    // Its canonical form contains neither widened spelling.
    let canonical = ordinary.canonical_lexical();
    assert!(!canonical.contains("<<("));
    assert!(!canonical.contains("--ltr"));
    assert!(!canonical.contains("--rtl"));

    // Add one term SEP-0009 has no production for, and exactly one widened spelling
    // appears.
    let widened = CdtValue::list(vec![
        CdtTerm::Literal(CdtLiteral::lang("hello", "en")),
        CdtTerm::triple(
            CdtTerm::Iri("http://example.org/s".into()),
            CdtTerm::Iri("http://example.org/p".into()),
            CdtTerm::Null,
        )
        .expect("within every bound"),
    ])
    .expect("within every bound");
    assert!(widened.uses_extension());
    assert!(widened.canonical_lexical().contains("<<("));
}

/// The two widened spellings are refused where SEP-0009 refuses them anyway, so the
/// supersets widen `Element` and nothing else.
#[test]
fn the_supersets_do_not_widen_anything_but_the_element_production() {
    // `[7] MapKey` admits neither a triple term nor a nested composite, and PurRDF
    // does not widen it: a triple term is still refused in key position.
    assert!(parse_map("{<<(<http://example.org/s> <http://example.org/p> 1)>>: 2}").is_err());
    // Only the two RDF 1.2 directions exist; `--up` is not a third one.
    assert!(parse_list("[\"x\"@en--up]").is_err());
    // A triple term takes exactly three components, as the production says.
    assert!(parse_list("[<<(<http://example.org/s> <http://example.org/p>)>>]").is_err());
    assert!(parse_list("[<<(1 2 3 4)>>]").is_err());
}
