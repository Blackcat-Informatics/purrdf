// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The whole-query proof that the scanner's name classes are the grammar's.
//!
//! A character class inside a scanner is a **boundary** test, not a membership
//! test: under maximal munch a wider class does not merely accept more, it moves
//! where one token stops and the next begins — in documents the narrow scanner
//! also accepts. So the failure never surfaces as a parse error. It surfaces as
//! a different, silently wrong, parse.
//!
//! The concrete one this file pins. `PN_CHARS_BASE` was approximated as "an
//! ASCII letter, `_`, or any scalar above `0x7F`", and `VARNAME` as "ASCII
//! alphanumeric, `_`, or any scalar above `0x7F`". U+00A0 NO-BREAK SPACE is
//! above `0x7F`, so the greedy name scan absorbed it — and the name scan runs
//! BEFORE the whitespace skip is ever consulted, so making the whitespace skip
//! exact could not reach the bug on its own. In
//!
//! ```text
//! SELECT ?s WHERE { ?s<NBSP><urn:ex:p> ?o . ?s <urn:ex:q> ?z }
//! ```
//!
//! the author named three variables. The liberal class produced FOUR — `s\u{a0}`,
//! `o`, `s`, `z` — because the NO-BREAK SPACE joined the first `?s` instead of
//! ending it. The two `?s` occurrences were then different variables, the join
//! between the two statement patterns became a cross product, and the query
//! returned a larger answer with exit status zero and no diagnostic.
//!
//! `crates/sparql-algebra/src/lexer.rs`'s own test module pins the token-level
//! boundaries. This file is the end-to-end claim: the query above is REFUSED by
//! the public parser, and the neighbouring query that differs only in using the
//! SPACE its author meant is still accepted and still names three variables.
//!
//! Every vector here is paired with a lawful neighbour, because tightening a
//! scanner class is exactly the change that over-refuses silently: `PN_CHARS`
//! names `#xB7`, `[#x300-#x36F]` and `[#x203F-#x2040]` beyond `PN_CHARS_U`, so a
//! class that carried only "base plus digits and hyphen" would refuse
//! `ex:col·lecció` and every NFD-decomposed name — the same word would parse
//! spelled in NFC and fail spelled in NFD.

use std::collections::BTreeSet;

use purrdf_iri::terminals;
use purrdf_sparql_algebra::{Query, SparqlParser, pattern_to_select_query};

/// The headline query, parameterised by what sits between `?s` and `<urn:ex:p>`.
fn headline(separator: &str) -> String {
    format!("SELECT ?s WHERE {{ ?s{separator}<urn:ex:p> ?o . ?s <urn:ex:q> ?z }}")
}

/// The distinct variable names a parsed `SELECT`'s `WHERE` algebra mentions,
/// read back off the crate's own serializer rather than off the input text.
///
/// Going through [`pattern_to_select_query`] is what makes the count a statement
/// about the ALGEBRA the parser built: two variables that differ by one
/// invisible scalar are two entries here, which is precisely the wrong answer
/// this file exists to refuse.
///
/// The name after each sigil is delimited with `VARNAME`'s own continue class
/// and not with an ad-hoc "alphanumeric or `_`" test, because the latter would
/// TRUNCATE exactly the names these vectors are about — `s\u{1680}` and
/// `cafe\u{301}` would both collapse onto a shorter name and the count would
/// silently come out right for the wrong reason.
fn variable_names(query: &Query) -> BTreeSet<String> {
    let Query::Select { pattern, .. } = query else {
        panic!("the vectors in this file are all SELECT queries");
    };
    let rendered = pattern_to_select_query(pattern);
    rendered
        .split('?')
        .skip(1) // text before the first sigil names no variable
        .map(|tail| {
            tail.chars()
                .take_while(|c| terminals::is_varname_continue(*c))
                .collect::<String>()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

/// The defect, end to end: a NO-BREAK SPACE in delimiter position is refused
/// rather than absorbed into the preceding variable's name.
///
/// The assertion is that the query does not PARSE — not that a particular error
/// arm fires. After tightening, a misplaced scalar surfaces wherever the token
/// split now falls, and pinning one offset or one message would pin an accident
/// of that split rather than the property.
#[test]
fn a_no_break_space_in_delimiter_position_is_refused() {
    let parser = SparqlParser::new();
    assert!(
        parser.parse_query(&headline("\u{a0}")).is_err(),
        "U+00A0 is neither `WS` nor a `VARNAME` character, so this is not SPARQL"
    );
}

/// The neighbour that must still work, and the measurement the defect got wrong:
/// with the SPACE its author meant, the query parses and names exactly THREE
/// variables — the two `?s` occurrences being the same variable is what makes
/// the two statement patterns a join rather than a cross product.
#[test]
fn the_lawful_neighbour_still_parses_and_still_names_three_variables() {
    let parser = SparqlParser::new();
    let query = parser
        .parse_query(&headline(" "))
        .expect("an ASCII SPACE is `WS` and separates the two terms");
    let names = variable_names(&query);
    assert_eq!(
        names,
        ["o", "s", "z"].iter().map(|s| (*s).to_owned()).collect(),
        "the author named three variables, so the algebra must hold three"
    );
}

/// Every `WS` member separates the two terms, and nothing else does. Both halves
/// run in one vector so the refusal is never asserted without its neighbour.
#[test]
fn exactly_the_four_ws_members_separate_the_two_terms() {
    let parser = SparqlParser::new();
    for separator in [" ", "\t", "\r", "\n", "\r\n", "  "] {
        assert!(
            parser.parse_query(&headline(separator)).is_ok(),
            "{separator:?} is `WS` and must still separate"
        );
    }
    for separator in ["\u{a0}", "\u{2000}", "\u{2028}", "\u{202f}", "\u{3000}"] {
        assert!(
            parser.parse_query(&headline(separator)).is_err(),
            "U+{:04X} is not `WS` and is not a name character",
            separator.chars().next().expect("non-empty") as u32
        );
    }
}

/// U+200B ZERO WIDTH SPACE is the member a `char::is_whitespace` audit never
/// looks at — it does not carry the Unicode `White_Space` property — and it sits
/// in the hole below `PN_CHARS_BASE`'s `[#x200C-#x200D]`, so it is not a name
/// character either. The liberal class absorbed it into names invisibly.
#[test]
fn a_zero_width_space_is_refused_in_delimiter_position() {
    let parser = SparqlParser::new();
    assert!(!'\u{200b}'.is_whitespace());
    assert!(parser.parse_query(&headline("\u{200b}")).is_err());
}

/// U+1680 OGHAM SPACE MARK is simultaneously Unicode `White_Space` and a member
/// of `PN_CHARS_BASE`'s `[#x37F-#x1FFF]`, so the grammar makes it a NAME
/// character: it does not separate the two terms, it JOINS the preceding
/// variable. The query below therefore names a variable `s\u{1680}` — and,
/// because that variable then appears only once, this parses exactly as any
/// other query with an unshared variable does.
///
/// Pinned positively and on purpose. It looks like the U+00A0 defect and is not
/// one; excluding the Unicode spaces from the name classes to "fix" it would
/// manufacture an over-refusal of a lawful SPARQL name.
#[test]
fn the_ogham_space_mark_is_a_name_character_not_a_separator() {
    let parser = SparqlParser::new();
    assert!('\u{1680}'.is_whitespace());
    let query = parser
        .parse_query(&headline("\u{1680}"))
        .expect("U+1680 is inside PN_CHARS_BASE, so it extends the variable name");
    assert_eq!(
        variable_names(&query),
        ["o", "s", "s\u{1680}", "z"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        "U+1680 joins the first `?s`, so this query really does name four"
    );
}

/// The over-refusal guard for `PN_CHARS`'s three non-ASCII additions and for
/// `PN_CHARS_BASE`'s reach beyond the Latin alphabet. Each of these is a lawful
/// prefixed name and must still parse.
#[test]
fn lawful_non_ascii_names_are_still_accepted() {
    let parser = SparqlParser::new();
    for local in [
        "col\u{b7}lecci\u{f3}", // MIDDLE DOT, and `ó` in NFC
        "cafe\u{301}",          // NFD: `e` + U+0301 COMBINING ACUTE ACCENT
        "caf\u{e9}",            // the same word in NFC
        "a\u{203f}b",           // UNDERTIE
        "a\u{2040}b",           // CHARACTER TIE
        "\u{4e2d}\u{6587}",     // CJK
        "\u{1f600}",            // astral plane, inside [#x10000-#xEFFFF]
        "a-b",                  // `PN_CHARS` keeps the hyphen a name character
    ] {
        let text = format!("PREFIX ex: <urn:ex:> SELECT ?s WHERE {{ ?s ex:{local} ?o }}");
        assert!(
            parser.parse_query(&text).is_ok(),
            "ex:{local} is a lawful prefixed name"
        );
    }
}

/// The over-refusal guard for `VARNAME`'s position-dependence: a combining mark,
/// a MIDDLE DOT and the two ties may CONTINUE a variable name even though they
/// may not begin one. Scanning the tail with a start-only class refuses all of
/// these and breaks nothing visibly until someone writes one.
#[test]
fn lawful_non_ascii_variable_names_are_still_accepted() {
    let parser = SparqlParser::new();
    for name in [
        "a\u{b7}",
        "a\u{300}",
        "a\u{203f}b",
        "a\u{2040}",
        "cafe\u{301}",
        "\u{4e2d}\u{6587}",
        "_1",
        "0",
    ] {
        let text = format!("SELECT ?{name} WHERE {{ ?{name} <urn:ex:p> ?o }}");
        assert!(parser.parse_query(&text).is_ok(), "?{name} is a lawful Var");
    }
}

/// `VARNAME` excludes `'-'` in both positions although `PN_CHARS` includes it,
/// so a variable's name scan must not reach for `is_pn_chars`: the hyphen after
/// a variable is still the subtraction operator.
#[test]
fn a_hyphen_after_a_variable_is_still_subtraction() {
    let parser = SparqlParser::new();
    for expression in ["?a - ?b", "?a-?b", "?x-1"] {
        let text = format!(
            "SELECT (({expression}) AS ?d) WHERE {{ ?a <urn:ex:p> ?b . ?x <urn:ex:q> ?y }}"
        );
        assert!(
            parser.parse_query(&text).is_ok(),
            "{expression} is a subtraction, not one variable"
        );
    }
}

/// The change that could silently reverse a fix made in the same file: U+00A0
/// and U+200B are LAWFUL raw inside an `IRIREF` (the production excludes only
/// `#x00-#x20` and nine delimiters), and they are RFC-3987 `ucschar`, so the
/// workspace's own writers emit them verbatim. Tightening the NAME classes must
/// leave the IRI body exactly where it was.
#[test]
fn non_ascii_whitespace_inside_an_iriref_still_parses() {
    let parser = SparqlParser::new();
    for c in ['\u{a0}', '\u{200b}', '\u{2000}', '\u{3000}'] {
        let text = format!("SELECT ?s WHERE {{ ?s <urn:ex:a{c}b> ?o }}");
        assert!(
            parser.parse_query(&text).is_ok(),
            "U+{:04X} is above #x20 and is not a reserved IRIREF delimiter",
            c as u32
        );
    }
}
