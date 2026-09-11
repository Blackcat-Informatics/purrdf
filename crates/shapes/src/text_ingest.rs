// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native RDF-text ingestion for the SHACL engine.
//!
//! The SHACL engine ingests two kinds of RDF text: the shapes graph (Turtle) and
//! the data graph (N-Triples). It used to drive the oxigraph `io` RDF parser
//! directly so it could (a) recover the document `@prefix` map — oxigraph stores
//! drop prefixes, but SHACL-AF `sh:select` queries reference prefixed names — and
//! (b) accumulate every independently recoverable syntax error in one pass instead of
//! short-circuiting on the first.
//!
//! This module reproduces both capabilities on top of the native purrdf
//! codecs ([`::purrdf::parse_dataset`]), which are lenient on private-use
//! language tags with long BCP-47 subtags and carry no oxigraph `io` dependency:
//!
//! - **Prefix recovery** ([`extract_prefixes`]) re-derives the `(prefix,
//!   namespace)` pairs by scanning the source text's `@prefix` / SPARQL `PREFIX`
//!   directives. The native codec discards prefixes once it folds to the IR, so
//!   the source scan is the faithful replacement for `RdfParser::prefixes()`.
//! - **Multi-error reporting** ([`parse_turtle_to_dataset`] /
//!   [`parse_ntriples_to_dataset`]) parses the whole document natively on the happy
//!   path; when that fails, it re-parses the document one statement (Turtle) or
//!   one line (N-Triples) at a time so each independently-malformed statement
//!   yields its own error, matching the oxttl error-recovery behavior the engine
//!   relied on.

use std::sync::Arc;

use ::purrdf::RdfDataset;
use ::purrdf::parse_dataset;
use purrdf_iri::terminals;

/// Strip the leading run of Turtle `WS` from `text`.
///
/// `WS ::= #x20 | #x9 | #xD | #xA` (RDF 1.2 Turtle §6.5; SPARQL 1.2 §19.8 names
/// the same four code points), so this is deliberately NOT [`str::trim_start`],
/// whose class is the Unicode `White_Space` property. Every scan in this module
/// uses the run it removes to decide where a directive begins, and U+00A0
/// NO-BREAK SPACE, U+000C FORM FEED, U+2001, U+2028 and U+3000 are `White_Space`
/// and are not `WS` — treating one as layout moves the boundary.
fn trim_ws_start(text: &str) -> &str {
    text.trim_start_matches(terminals::is_ws_char)
}

/// Strip the leading and trailing runs of Turtle `WS` from `text`.
///
/// The two-sided form of [`trim_ws_start`], over the same four-member `WS`
/// table, and deliberately NOT [`str::trim`].
fn trim_ws(text: &str) -> &str {
    text.trim_matches(terminals::is_ws_char)
}

/// Whether `label` is the `PN_PREFIX` of a `PNAME_NS`.
///
/// ```text
/// PNAME_NS  ::= PN_PREFIX? ':'
/// PN_PREFIX ::= PN_CHARS_BASE ((PN_CHARS | '.')* PN_CHARS)?
/// ```
///
/// RDF 1.2 Turtle §6.5 (SPARQL 1.2 §19.8 spells both identically). The empty
/// label is admitted because `PN_PREFIX` is OPTIONAL in `PNAME_NS` — `@prefix :
/// <…>` declares the default prefix.
///
/// This runs on what the line scan believed was a label, and it is the guard
/// that keeps a scan from minting a prefix the grammar cannot name. Without it,
/// tightening the surrounding whitespace class would trade one defect for its
/// mirror image: a junk directive quoted inside a long string literal used to be
/// laundered into the plausible prefix `evil` by a Unicode-property `trim`, and
/// once the trim is `WS`-exact the junk survives INTO the generated `PREFIX`
/// header, where it is a lex error that breaks every SHACL-AF query in the
/// document — including the ones that were always correct. A label that is not a
/// `PN_PREFIX` is not a prefix declaration, so the line is simply not a
/// directive.
///
/// No real declaration can be lost to this. `extract_prefixes` is only ever
/// called on source the codec has ALREADY parsed, and a document carrying a
/// `PNAME_NS` this refuses would not have parsed.
fn is_pn_prefix(label: &str) -> bool {
    let mut characters = label.chars();
    let Some(first) = characters.next() else {
        return true; // `PN_PREFIX?` — the default prefix declares an empty label.
    };
    if !terminals::is_pn_chars_base(first) {
        return false;
    }
    // `(PN_CHARS | '.')* PN_CHARS`: a `'.'` may appear between name characters
    // and may not end the label, so track whether the last scalar was `PN_CHARS`.
    let mut ends_at_pn_chars = true;
    for character in characters {
        if terminals::is_pn_chars(character) {
            ends_at_pn_chars = true;
        } else if character == '.' {
            ends_at_pn_chars = false;
        } else {
            return false;
        }
    }
    ends_at_pn_chars
}

/// Recover the document `(prefix, namespace)` map from Turtle/TriG source text.
///
/// Scans `@prefix pfx: <iri> .` and SPARQL-style `PREFIX pfx: <iri>` directives.
/// The native codec drops these once it folds to the IR, so this source scan is
/// the faithful replacement for the oxigraph `io` parser's `prefixes()`. Later
/// declarations of the same prefix win (last-writer), matching oxttl's final
/// prefix table.
///
/// # This is a scan, not a parse — and narrowing its class does not change that
///
/// The map this returns becomes the `PREFIX` header prepended to every SHACL-AF
/// `sh:sparql` / `sh:select` query (the `build_prefix_header` composition in
/// `crate::shapes`), so a prefix invented here is a prefix the validator's own
/// queries can resolve — and one it cannot resolve is a hard error. The scan
/// walks raw source lines and therefore remains an APPROXIMATION of the Turtle
/// grammar: it cannot tell a directive from one quoted inside a long string
/// literal, so
///
/// ```text
/// ex:Shape rdfs:comment """
/// @prefix evil: <urn:ex:> .
/// """ .
/// ```
///
/// still yields an `evil` binding that the grammar does not declare. Spelling
/// the character classes exactly closes the case where the two readers
/// DISAGREE — where a scalar the grammar does not call whitespace let the scan
/// find a directive text the parser never sees as one — and it does not make
/// this function a parser. A caller that needs the document's real prefix map
/// must get it from the codec, not from here.
pub fn extract_prefixes(text: &str) -> Vec<(String, String)> {
    let mut map: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for (prefix, namespace) in scan_prefixes(text) {
        map.insert(prefix, namespace);
    }
    map.into_iter().collect()
}

/// Iterate `(prefix, namespace)` directives in `text`, in source order.
///
/// Every boundary below is decided by Turtle's own whitespace terminal —
/// `WS ::= #x20 | #x9 | #xD | #xA` (RDF 1.2 Turtle §6.5) — through
/// [`trim_ws_start`] / [`trim_ws`] / [`terminals::is_ws_char`], never by the
/// Unicode `White_Space` property. The difference is a verdict, not a nicety:
/// with the property, `@prefix<U+00A0>evil: <urn:ex:>` quoted inside a long
/// string literal was read as a directive, the phantom `evil` binding was
/// prepended to every SHACL-AF query, and a shapes graph whose query names an
/// undeclared prefix reported `conforms false` with a Violation instead of
/// failing to parse. The `PNAME_NS` label is likewise trimmed with `WS` only, so
/// `@prefix evil<U+00A0>: <…>` can no longer be laundered into the label `evil`.
fn scan_prefixes(text: &str) -> impl Iterator<Item = (String, String)> + '_ {
    text.lines().filter_map(|line| {
        let trimmed = trim_ws_start(line);
        // `@prefix` (Turtle) or `PREFIX` (SPARQL, case-insensitive) directive head.
        let rest = trimmed
            .strip_prefix("@prefix")
            .or_else(|| trimmed.strip_prefix("PREFIX"))
            .or_else(|| trimmed.strip_prefix("prefix"))?;
        // The directive head must be followed by `WS` (so `@prefixfoo` is not
        // mistaken for a directive) — the grammar separates `@prefix` from
        // `PNAME_NS` with `WS`, and nothing else separates them.
        if !rest.starts_with(terminals::is_ws_char) {
            return None;
        }
        let rest = trim_ws_start(rest);
        // `pfx:` (possibly empty `:`) up to the `<`.
        let colon = rest.find(':')?;
        let angle = rest.find('<')?;
        if colon > angle {
            return None;
        }
        let prefix = trim_ws(&rest[..colon]);
        if !is_pn_prefix(prefix) {
            return None;
        }
        let prefix = prefix.to_owned();
        let after_angle = &rest[angle + 1..];
        let close = after_angle.find('>')?;
        let namespace = after_angle[..close].to_owned();
        Some((prefix, namespace))
    })
}

/// Parse a Turtle document into a frozen [`RdfDataset`] via the native codecs,
/// resolving relative IRI references against `base`.
///
/// `base` is the document's base IRI (RFC-3986 §5.1.2, the caller-supplied base) and is
/// threaded verbatim into [`parse_dataset`] — the SAME `Option<&str>` parameter every
/// other PurRDF ingress takes, so a Turtle shapes graph resolves exactly like the same
/// bytes read through any other seam. `None` means *no base is in scope*: an in-document
/// `@base` can still establish one, and failing that a relative reference is a hard
/// `iri-relative-no-base` rather than a silently-interned non-IRI term.
///
/// On a clean parse the dataset is returned. On a syntax error the document is
/// re-parsed one top-level statement at a time so EVERY independently-malformed
/// statement is reported; the returned `Err` is the list of per-statement error
/// strings.
pub fn parse_turtle_to_dataset(
    ttl: &str,
    base: Option<&str>,
) -> Result<Arc<RdfDataset>, Vec<String>> {
    if ttl.is_empty() {
        return Ok(empty_dataset());
    }
    match parse_dataset(ttl.as_bytes(), "text/turtle", base) {
        Ok(dataset) => Ok(dataset),
        Err(_) => Err(turtle_statement_errors(ttl, base)),
    }
}

/// Parse an N-Triples document into a frozen [`RdfDataset`] via the native codecs,
/// accumulating every malformed line as its own error.
///
/// N-Triples admits **no** relative IRI reference by grammar (RDF 1.2 N-Triples §2.3),
/// so this ingress takes no base: there is nothing a base could resolve, and accepting
/// one would imply it might rescue a document the grammar rejects. A relative reference
/// here is `iri-not-absolute-by-grammar`, which no base can fix.
pub fn parse_ntriples_to_dataset(data_nt: &str) -> Result<Arc<RdfDataset>, Vec<String>> {
    if data_nt.is_empty() {
        return Ok(empty_dataset());
    }
    match parse_dataset(data_nt.as_bytes(), "application/n-triples", None) {
        Ok(dataset) => Ok(dataset),
        Err(_) => Err(ntriples_line_errors(data_nt)),
    }
}

/// A frozen empty dataset (the `parse_dataset` of an empty document).
fn empty_dataset() -> Arc<RdfDataset> {
    ::purrdf::RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty dataset freezes")
}

/// Enumerate per-statement Turtle parse errors by re-parsing each top-level
/// statement (terminated by `.`) with the document's prefix directives prepended,
/// so each malformed statement surfaces independently.
///
/// `base` is the SAME base the whole-document parse ran under. Threading it here is not
/// cosmetic: without it every statement holding a relative IRI would re-fail with
/// `iri-relative-no-base` during recovery, so a document whose only real defect was
/// elsewhere would report a list of fabricated base errors instead of the one true one.
///
/// # Why the `WS` class matters on a path that only runs after a failure
///
/// This whole path runs only once the document has ALREADY failed to parse, so no
/// verdict here can turn a conforming document into a non-conforming one. What the
/// class decides is which chunks are offered back to the codec. Trimming and
/// directive-matching with `WS ::= #x20 | #x9 | #xD | #xA` (RDF 1.2 Turtle §6.5)
/// rather than the Unicode `White_Space` property means a chunk whose only content
/// is a U+00A0 is no longer silently treated as blank: it is handed to the codec,
/// which is the one thing entitled to say whether it is an error. No error can be
/// fabricated by narrowing, because every reported string is a diagnostic
/// `parse_dataset` itself produced for that exact text.
fn turtle_statement_errors(ttl: &str, base: Option<&str>) -> Vec<String> {
    // The prefix/base directives every statement needs to resolve prefixed names.
    let header: String = ttl
        .lines()
        .filter(|line| {
            let t = trim_ws_start(line);
            t.starts_with("@prefix")
                || t.starts_with("@base")
                || t.starts_with("PREFIX")
                || t.starts_with("prefix")
                || t.starts_with("BASE")
                || t.starts_with("base")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut errors: Vec<String> = Vec::new();
    for statement in split_turtle_statements(ttl) {
        let trimmed = trim_ws(&statement);
        if trimmed.is_empty() || is_directive(trimmed) {
            continue;
        }
        let candidate = format!("{header}\n{trimmed}\n");
        if let Err(e) = parse_dataset(candidate.as_bytes(), "text/turtle", base) {
            errors.push(format!("Turtle parse error: {e}"));
        }
    }
    if errors.is_empty() {
        // The whole-document parse failed but no individual statement did (e.g. a
        // lexer-level break that consumes to EOF): re-surface the document error
        // as a single entry rather than swallowing it.
        if let Err(e) = parse_dataset(ttl.as_bytes(), "text/turtle", base) {
            errors.push(format!("Turtle parse error: {e}"));
        }
    }
    errors
}

/// Whether `statement` is a `@prefix`/`@base`/SPARQL `PREFIX`/`BASE` directive.
///
/// The leading run this skips is Turtle's `WS` (§6.5), not the Unicode
/// `White_Space` property: indentation a Turtle document may lawfully carry is
/// exactly those four code points, and anything else before a `@` is content the
/// codec must be allowed to reject rather than layout this scan may discard.
fn is_directive(statement: &str) -> bool {
    let t = trim_ws_start(statement);
    t.starts_with("@prefix")
        || t.starts_with("@base")
        || t.starts_with("PREFIX")
        || t.starts_with("prefix")
        || t.starts_with("BASE")
        || t.starts_with("base")
}

/// Split Turtle source into top-level statements on the `.` terminator, ignoring
/// `.`s inside IRIs (`<...>`), string literals (`"..."`, `'...'`) and `#` comments.
///
/// The comment arm matters as much as the other two. A `#` comment runs to end of line
/// (Turtle §6.2) and may contain anything, including a prose `.` — and this splitter
/// feeds the RECOVERY path, whose whole job is to name the document's real defects. A
/// splitter that terminated a statement on a comment's full stop would hand the parser
/// half a sentence of English and report an invented syntax error, burying the actual
/// one. `#` is only a comment outside an IRI and outside a string, since it is also the
/// fragment delimiter of every `<http://…#…>` IRI in a SHACL document.
fn split_turtle_statements(ttl: &str) -> Vec<String> {
    let mut statements: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = ttl.chars();
    let mut in_iri = false;
    let mut string_delim: Option<char> = None;
    while let Some(c) = chars.next() {
        current.push(c);
        match string_delim {
            Some(delim) => {
                if c == '\\' {
                    // Escaped char inside a string: consume the next char verbatim.
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                } else if c == delim {
                    string_delim = None;
                }
            }
            None => match c {
                '<' if !in_iri => in_iri = true,
                '>' if in_iri => in_iri = false,
                '"' | '\'' if !in_iri => string_delim = Some(c),
                // A comment: copy it through verbatim to end of line so the statement
                // keeps its original text, but let none of its bytes be interpreted.
                '#' if !in_iri => {
                    for next in chars.by_ref() {
                        current.push(next);
                        if next == '\n' {
                            break;
                        }
                    }
                }
                '.' if !in_iri => {
                    statements.push(std::mem::take(&mut current));
                }
                _ => {}
            },
        }
    }
    // `WS`-exact (§6.5): a trailing chunk made only of U+00A0 is not blank Turtle,
    // so it is kept as a statement and offered to the codec rather than discarded
    // here as if it were layout.
    if !trim_ws(&current).is_empty() {
        statements.push(current);
    }
    statements
}

/// Enumerate per-line N-Triples parse errors by parsing each non-blank,
/// non-comment line independently.
///
/// The per-line trim is N-Triples' own whitespace class — RDF 1.2 N-Triples
/// admits exactly the four code points `WS ::= #x20 | #x9 | #xD | #xA` between
/// terminals, the same set Turtle §6.5 enumerates — and not the Unicode
/// `White_Space` property, so a line carrying a
/// U+00A0 is handed to the codec instead of being trimmed into a well-formed
/// shape or silently skipped as blank. As in the Turtle recovery, this path runs
/// only after the document has already failed and every message is one
/// `parse_dataset` produced for that exact line, so narrowing can surface a real
/// defect but cannot invent one.
fn ntriples_line_errors(data_nt: &str) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    for line in data_nt.lines() {
        let trimmed = trim_ws(line);
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let candidate = format!("{trimmed}\n");
        if let Err(e) = parse_dataset(candidate.as_bytes(), "application/n-triples", None) {
            errors.push(format!("N-Triples parse error: {e}"));
        }
    }
    if errors.is_empty()
        && let Err(e) = parse_dataset(data_nt.as_bytes(), "application/n-triples", None)
    {
        errors.push(format!("N-Triples parse error: {e}"));
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_prefixes_turtle_and_sparql_forms() {
        let ttl = concat!(
            "@prefix ex: <http://example.org/ns#> .\n",
            "PREFIX meta: <https://example.org/meta/>\n",
            "@prefix : <http://example.org/default#> .\n",
        );
        let prefixes = extract_prefixes(ttl);
        assert!(prefixes.contains(&("ex".to_owned(), "http://example.org/ns#".to_owned())));
        assert!(prefixes.contains(&("meta".to_owned(), "https://example.org/meta/".to_owned())));
        assert!(prefixes.contains(&(String::new(), "http://example.org/default#".to_owned())));
    }

    #[test]
    fn a_non_ws_separator_does_not_open_a_directive() {
        // The verdict flip: with the Unicode `White_Space` property, a U+00A0
        // between `@prefix` and the label made this a directive, and the phantom
        // `evil` binding was prepended to every SHACL-AF query — so a shapes graph
        // whose query named an undeclared prefix reported `conforms false` instead
        // of failing to parse. `WS` names four code points and U+00A0 is not one.
        for separator in ['\u{A0}', '\u{2028}', '\u{3000}', '\u{0C}', '\u{2001}'] {
            let ttl = format!("@prefix{separator}evil: <urn:ex:> .\n");
            assert_eq!(
                extract_prefixes(&ttl),
                Vec::new(),
                "{separator:?} is not WS and may not separate `@prefix` from its label"
            );
        }
    }

    #[test]
    fn a_non_ws_indent_does_not_open_a_directive() {
        // The same class, at the other end of the line: a line "indented" with a
        // U+00A0 is not an indented directive, it is malformed Turtle.
        let ttl = "\u{A0}@prefix evil: <urn:ex:> .\n";
        assert_eq!(extract_prefixes(ttl), Vec::new());
    }

    #[test]
    fn a_non_ws_scalar_is_not_trimmed_out_of_the_pname_ns_label() {
        // `rest[..colon].trim()` used to launder the U+00A0 out of the label and
        // mint the prefix `evil`. The label is a `PN_PREFIX`, and a `WS`-exact trim
        // cannot turn `evil<U+00A0>` into `evil` — nor may the junk label reach the
        // generated header, where it would be a lex error that breaks every query
        // in the document.
        let ttl = "@prefix evil\u{A0}: <urn:ex:> .\n";
        assert_eq!(extract_prefixes(ttl), Vec::new());
    }

    #[test]
    fn only_a_pn_prefix_may_be_scanned_out_as_a_prefix() {
        // `PN_PREFIX ::= PN_CHARS_BASE ((PN_CHARS | '.')* PN_CHARS)?`. Each refusal
        // is a head, tail or interior the production does not name.
        for label in ["evil\u{A0}", "-bad", ".bad", "0bad", "_bad", "bad."] {
            let ttl = format!("@prefix {label}: <urn:ex:> .\n");
            assert_eq!(
                extract_prefixes(&ttl),
                Vec::new(),
                "`{label}` is not a PN_PREFIX"
            );
        }
        // The executed valid neighbours, including the empty default prefix and the
        // interior `'.'` and `'-'` the production DOES admit, and a non-ASCII
        // `PN_CHARS_BASE` head — so this is the grammar and not an ASCII-only rule.
        for label in ["ex", "", "a.b", "ex-1", "x9", "\u{E9}t", "\u{4E2D}"] {
            let ttl = format!("@prefix {label}: <urn:ex:> .\n");
            assert_eq!(
                extract_prefixes(&ttl),
                vec![(label.to_owned(), "urn:ex:".to_owned())],
                "`{label}` is a PN_PREFIX and must still be found"
            );
        }
    }

    #[test]
    fn every_ws_separated_and_indented_directive_is_still_found() {
        // The executed valid neighbours of the three refusals above: each of the
        // four `WS` code points still separates and still indents a directive, in
        // both the Turtle and the SPARQL spelling, so this is exactness and not a
        // narrowed scanner that stopped finding prefixes.
        let ttl = concat!(
            "@prefix ex: <http://example.org/ns#> .\n",
            "  PREFIX two: <http://example.org/two#>\n",
            "@prefix\ttab: <http://example.org/tab#> .\n",
            "\t@prefix ind: <http://example.org/ind#> .\n",
            "@prefix crlf: <http://example.org/crlf#> .\r\n",
        );
        let prefixes = extract_prefixes(ttl);
        for (prefix, namespace) in [
            ("ex", "http://example.org/ns#"),
            ("two", "http://example.org/two#"),
            ("tab", "http://example.org/tab#"),
            ("ind", "http://example.org/ind#"),
            ("crlf", "http://example.org/crlf#"),
        ] {
            assert!(
                prefixes.contains(&(prefix.to_owned(), namespace.to_owned())),
                "`{prefix}` must still be found: {prefixes:?}"
            );
        }
    }

    #[test]
    fn extract_prefixes_last_declaration_wins() {
        let ttl = concat!(
            "@prefix ex: <http://example.org/v1#> .\n",
            "@prefix ex: <http://example.org/v2#> .\n",
        );
        let prefixes = extract_prefixes(ttl);
        let ex: Vec<_> = prefixes.iter().filter(|(p, _)| p == "ex").collect();
        assert_eq!(ex.len(), 1);
        assert_eq!(ex[0].1, "http://example.org/v2#");
    }

    #[test]
    fn parse_turtle_clean_input_succeeds() {
        let ttl = "@prefix ex: <http://example.org/ns#> .\nex:a ex:p ex:b .\n";
        let dataset = parse_turtle_to_dataset(ttl, None).expect("clean Turtle parses");
        assert_eq!(dataset.quad_refs().count(), 1);
    }

    #[test]
    fn parse_turtle_reports_multiple_statement_errors() {
        let bad = concat!(
            "@prefix ex: <http://example.org/ns#> .\n",
            "ex:a ex:p .\n",                // missing object → error
            "ex:b ex:q ex:c .\n",           // valid
            "ex:d ex:r ex:s ex:t ex:u .\n", // too many terms → error
        );
        let Err(errors) = parse_turtle_to_dataset(bad, None) else {
            panic!("malformed Turtle must error")
        };
        assert!(
            errors.len() >= 2,
            "expected >=2 statement errors, got {}: {errors:?}",
            errors.len()
        );
    }

    #[test]
    fn parse_ntriples_reports_multiple_line_errors() {
        let bad = concat!(
            "this is not a triple\n",
            "<http://example.org/s> <http://example.org/p> .\n",
            "neither is this\n",
        );
        let Err(errors) = parse_ntriples_to_dataset(bad) else {
            panic!("malformed N-Triples must error")
        };
        assert!(
            errors.len() >= 2,
            "expected >=2 line errors, got {}: {errors:?}",
            errors.len()
        );
    }

    #[test]
    fn parse_ntriples_clean_input_succeeds() {
        let nt = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
        let dataset = parse_ntriples_to_dataset(nt).expect("clean N-Triples parses");
        assert_eq!(dataset.quad_refs().count(), 1);
    }

    #[test]
    fn split_turtle_ignores_dots_in_iris_and_strings() {
        let ttl = "ex:a ex:p \"a. b. c\" . ex:d ex:e <http://x.y/z> .";
        let statements = split_turtle_statements(ttl);
        let non_empty = statements.iter().filter(|s| !s.trim().is_empty()).count();
        assert_eq!(non_empty, 2, "two statements: {statements:?}");
    }

    #[test]
    fn split_turtle_ignores_dots_inside_comments() {
        // A `#` comment runs to end of line and may contain prose with a full stop.
        // Terminating a statement there would hand the parser half an English sentence
        // and report an invented syntax error over the document's real one.
        let ttl = concat!(
            "# The subject below is relative. It resolves against the base.\n",
            "ex:a ex:p ex:b .\n",
        );
        let statements = split_turtle_statements(ttl);
        let non_empty = statements.iter().filter(|s| !s.trim().is_empty()).count();
        assert_eq!(
            non_empty, 1,
            "the comment is not a statement boundary: {statements:?}"
        );
    }

    #[test]
    fn split_turtle_does_not_treat_an_iri_fragment_as_a_comment() {
        // `#` is also the fragment delimiter of every `<http://…#…>` IRI a SHACL
        // document uses, so it only starts a comment OUTSIDE an IRI.
        let ttl = "<http://example.org/ns#a> <http://example.org/ns#p> \"v\" .";
        let statements = split_turtle_statements(ttl);
        let non_empty = statements.iter().filter(|s| !s.trim().is_empty()).count();
        assert_eq!(non_empty, 1, "one statement: {statements:?}");
    }

    #[test]
    fn a_comment_containing_a_full_stop_does_not_fabricate_an_error() {
        // End-to-end over the recovery path: the document's ONLY defect is the missing
        // object, and that is the only error reported. Before the splitter understood
        // comments, the prose full stop produced extra bogus entries alongside it.
        let bad = concat!(
            "@prefix ex: <http://example.org/ns#> .\n",
            "# This graph is deliberately broken. See the next line.\n",
            "ex:a ex:p .\n",
        );
        let Err(errors) = parse_turtle_to_dataset(bad, None) else {
            panic!("malformed Turtle must error")
        };
        assert_eq!(
            errors.len(),
            1,
            "exactly one real defect must be reported: {errors:?}"
        );
    }

    #[test]
    fn the_recovery_path_still_indents_and_skips_ws_layout() {
        // The executed valid neighbour for the recovery path's own `WS` narrowing:
        // directives indented with spaces and tabs still reach the per-statement
        // header (so no prefixed name goes undeclared during recovery), blank and
        // `WS`-only statements are still skipped, and the ONE real defect is still
        // the only thing reported.
        let bad = concat!(
            "  @prefix ex: <http://example.org/ns#> .\n",
            "\t@base <http://example.org/> .\n",
            "\n",
            "   \t \n",
            "ex:a ex:p ex:b .\n",
            "ex:c ex:q .\n", // the only real defect: missing object
        );
        let Err(errors) = parse_turtle_to_dataset(bad, None) else {
            panic!("the malformed statement must error")
        };
        assert_eq!(
            errors.len(),
            1,
            "only the genuinely malformed statement is reported: {errors:?}"
        );
    }

    #[test]
    fn a_ws_only_ntriples_line_is_still_skipped() {
        // The mirror neighbour on the N-Triples path: lines made of the four `WS`
        // code points are still blank, so they add no error, while the one bad line
        // does.
        let bad = concat!(
            "   \t \n",
            "\n",
            "  # a comment\n",
            "<http://example.org/s> <http://example.org/p> .\n",
        );
        let Err(errors) = parse_ntriples_to_dataset(bad) else {
            panic!("the malformed line must error")
        };
        assert_eq!(errors.len(), 1, "only the bad line is reported: {errors:?}");
    }

    #[test]
    fn parse_turtle_resolves_relative_iris_against_the_base() {
        let ttl = "<rel> <http://example.org/p> <http://example.org/o> .\n";
        let dataset = parse_turtle_to_dataset(ttl, Some("http://example.org/dir/doc.ttl"))
            .expect("a based document parses");
        let nt = ::purrdf::canonical_flat_nquads(dataset.as_ref()).expect("serialize");
        assert!(
            nt.contains("<http://example.org/dir/rel>"),
            "the relative subject must resolve against the base: {nt}"
        );
    }

    #[test]
    fn parse_turtle_without_a_base_refuses_a_relative_iri() {
        let ttl = "<rel> <http://example.org/p> <http://example.org/o> .\n";
        let Err(errors) = parse_turtle_to_dataset(ttl, None) else {
            panic!("a relative IRI with no base must be refused")
        };
        assert!(
            errors.iter().any(|e| e.contains("iri-relative-no-base")),
            "the refusal must name the actionable condition: {errors:?}"
        );
    }

    #[test]
    fn the_recovery_path_does_not_report_fabricated_base_errors() {
        // A document with BOTH a relative IRI (legitimate under the base) and one real
        // syntax error. Recovery re-parses statement by statement; without the base
        // threaded through, every relative-IRI statement would re-fail with
        // `iri-relative-no-base` and bury the one defect the author must fix.
        let bad = concat!(
            "@prefix ex: <http://example.org/ns#> .\n",
            "<rel-a> ex:p ex:b .\n",
            "<rel-b> ex:q .\n", // the only real error: missing object
            "<rel-c> ex:r ex:d .\n",
        );
        let Err(errors) = parse_turtle_to_dataset(bad, Some("http://example.org/doc")) else {
            panic!("the malformed statement must error")
        };
        assert!(
            !errors.iter().any(|e| e.contains("iri-relative-no-base")),
            "no base error may be fabricated during recovery: {errors:?}"
        );
        assert_eq!(
            errors.len(),
            1,
            "only the genuinely malformed statement is reported: {errors:?}"
        );
    }
}
