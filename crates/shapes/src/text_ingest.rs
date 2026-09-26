// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native RDF-text ingestion for the SHACL engine.
//!
//! The SHACL engine ingests two kinds of RDF text: the shapes graph (Turtle) and
//! the data graph (N-Triples). Two things are needed beyond the dataset itself:
//!
//! - **The document prefix map** ([`parse_turtle_document`]). SHACL-SPARQL queries
//!   may use the shapes document's `@prefix` declarations as a fallback (see
//!   `crate::shapes::prefixes`), and the frozen IR does not retain them. They come
//!   from the Turtle codec's OWN record of the directives it parsed
//!   ([`::purrdf::ParseOutcome::document_prefixes`]) — one parser, no second reading
//!   of the text. A text scan cannot tell a directive from the same characters
//!   quoted inside a long string literal (a `PREFIX` line inside an `sh:select`
//!   query is exactly that), and every prefix such a scan invents is prepended to
//!   every other query in the document.
//! - **Multi-error reporting** ([`parse_turtle_to_dataset`] /
//!   [`parse_ntriples_to_dataset`]) parses the whole document natively on the happy
//!   path; when that fails, it re-parses the document one statement (Turtle) or
//!   one line (N-Triples) at a time so each independently-malformed statement
//!   yields its own error.

use std::sync::Arc;

use ::purrdf::RdfDataset;
use ::purrdf::{ParseOptions, parse_dataset, parse_dataset_with};
use purrdf_iri::terminals;

/// Strip the leading and trailing runs of Turtle `WS` from `text`.
///
/// `WS ::= #x20 | #x9 | #xD | #xA` (RDF 1.2 Turtle §6.5; SPARQL 1.2 §19.8 names
/// the same four code points), so this is deliberately NOT [`str::trim`], whose
/// class is the Unicode `White_Space` property: U+00A0 NO-BREAK SPACE, U+000C FORM
/// FEED, U+2001, U+2028 and U+3000 are `White_Space` and are not `WS`, and a chunk
/// made of one is content for the codec to judge, not layout to discard.
fn trim_ws(text: &str) -> &str {
    text.trim_matches(terminals::is_ws_char)
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

/// Everything one parse of a Turtle shapes document produced.
#[derive(Debug, Clone)]
pub struct TurtleDocument {
    /// The frozen dataset — identical to what [`parse_turtle_to_dataset`] returns.
    pub dataset: Arc<RdfDataset>,
    /// The base IRI the document ENDED under: the caller's base unless the document
    /// declares its own `@base`, in which case the base the directive established.
    ///
    /// It is the document's own statement of where it lives, which is what an
    /// `owl:imports` of the document's own IRI names — so the shapes lanes pass it to
    /// `purrdf_core::imports` as a loaded document IRI (see [`crate::imports`]).
    pub base: Option<String>,
    /// The document's prefix map, as the Turtle codec itself recorded it
    /// ([`::purrdf::ParseOutcome::document_prefixes`]): one `(label, namespace)` pair
    /// per label a `@prefix` / `PREFIX` directive declared, sorted by label, bound to
    /// the namespace of the label's LAST declaration.
    ///
    /// # Which map, when a prefix is redeclared
    ///
    /// Turtle lets a document redeclare a prefix mid-document; each prefixed name then
    /// means what the declaration in force at that point says. The dataset keeps no
    /// statement positions, so a consumer of the parsed graph cannot recover which
    /// declaration was in force where a given query literal sat — and the only consumer
    /// here, the SHACL-SPARQL document-prefix FALLBACK, is a PurRDF convenience with no
    /// spec-defined scope of its own. It therefore uses one map for the whole document:
    /// the final one, which is the document's last word on each label and what the
    /// previous line-scanning recovery produced (last declaration wins). A query that
    /// needs a different binding says so in its own prologue or through `sh:prefixes`,
    /// both of which outrank this map.
    pub prefixes: Vec<(String, String)>,
}

/// [`parse_turtle_to_dataset`], also returning the base the document ended under and
/// the document's prefix map — all three from ONE parse (see [`TurtleDocument`]).
///
/// # Errors
///
/// Exactly [`parse_turtle_to_dataset`]'s.
pub fn parse_turtle_document(ttl: &str, base: Option<&str>) -> Result<TurtleDocument, Vec<String>> {
    if ttl.is_empty() {
        return Ok(TurtleDocument {
            dataset: empty_dataset(),
            base: base.map(str::to_owned),
            prefixes: Vec::new(),
        });
    }
    match parse_dataset_with(
        ttl.as_bytes(),
        "text/turtle",
        base,
        &ParseOptions::default(),
    ) {
        Ok(outcome) => Ok(TurtleDocument {
            base: outcome.document_base_iri().map(str::to_owned),
            dataset: outcome.dataset,
            prefixes: outcome.document_prefixes,
        }),
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
/// statement (terminated by `.`) in document order, under the directives the
/// document had made BEFORE it, so each malformed statement surfaces independently.
///
/// # Where the directives come from
///
/// From the codec, never from the text. The recovery carries one directive state —
/// the prefix bindings and the base in force — from statement to statement: each
/// statement is parsed under the state the previous ones left, and the state it
/// leaves is read back from the parse ([`::purrdf::ParseOutcome::document_prefixes`]
/// and `document_base`), or, when that statement is itself malformed, from the
/// failure ([`::purrdf::ParseFailure`]), which reports the directives the grammar
/// read before the failing token. So a `PREFIX` line quoted inside a string literal
/// declares nothing here either, a directive applies only to the statements after
/// it (a later `@base` does not re-resolve an earlier statement), and a SPARQL-form
/// `PREFIX` that shares a chunk with a malformed statement still reaches the
/// statements that follow. Every chunk, a directive included, is offered to the
/// codec, so a malformed directive is reported rather than skipped.
///
/// The carried state is replayed as `@prefix` directives whose namespaces are the
/// already-resolved IRIs the codec reported, and the base goes in as the parse's own
/// base argument, so replaying resolves nothing a second time.
///
/// `base` is the SAME base the whole-document parse ran under. Threading it here is not
/// cosmetic: without it every statement holding a relative IRI would re-fail with
/// `iri-relative-no-base` during recovery, so a document whose only real defect was
/// elsewhere would report a list of fabricated base errors instead of the one true one.
///
/// This whole path runs only once the document has ALREADY failed to parse, so no
/// verdict here can turn a conforming document into a non-conforming one, and every
/// reported string is a diagnostic the codec itself produced for a chunk of the
/// document. Blank chunks — only `WS` (RDF 1.2 Turtle §6.5), never the Unicode
/// `White_Space` property — are skipped; any other chunk is the codec's to judge.
fn turtle_statement_errors(ttl: &str, base: Option<&str>) -> Vec<String> {
    use std::fmt::Write as _;

    let mut prefixes: Vec<(String, String)> = Vec::new();
    let mut state_base: Option<String> = base.map(str::to_owned);
    let mut errors: Vec<String> = Vec::new();
    for statement in split_turtle_statements(ttl) {
        let trimmed = trim_ws(&statement);
        if trimmed.is_empty() {
            continue;
        }
        let mut candidate = String::new();
        for (prefix, namespace) in &prefixes {
            let _ = writeln!(candidate, "@prefix {prefix}: <{namespace}> .");
        }
        candidate.push_str(trimmed);
        candidate.push('\n');
        match ::purrdf::parse_dataset_reporting_failure(
            candidate.as_bytes(),
            "text/turtle",
            state_base.as_deref(),
            &ParseOptions::default(),
        ) {
            Ok(outcome) => {
                state_base = outcome.document_base_iri().map(str::to_owned);
                prefixes = outcome.document_prefixes;
            }
            Err(failure) => {
                errors.push(format!("Turtle parse error: {}", failure.diagnostic));
                state_base = failure.document_base_iri().map(str::to_owned);
                prefixes = failure.document_prefixes;
            }
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
    let mut chars = ttl.chars().peekable();
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
                // A `.` followed by a name character is inside a token — the fraction
                // of a DECIMAL/DOUBLE (`1.5`), or an interior `.` of a `PN_LOCAL` /
                // `PN_PREFIX` (`ex:a.b`) — and ends no statement; splitting there would
                // hand the codec half a number and report an invented error.
                '.' if !in_iri
                    && !chars
                        .peek()
                        .is_some_and(|&next| terminals::is_pn_chars(next)) =>
                {
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

    fn prefixes_of(ttl: &str) -> Vec<(String, String)> {
        parse_turtle_document(ttl, None)
            .expect("the fixture parses")
            .prefixes
    }

    #[test]
    fn document_prefixes_turtle_and_sparql_forms() {
        let ttl = concat!(
            "@prefix ex: <http://example.org/ns#> .\n",
            "PREFIX meta: <https://example.org/meta/>\n",
            "@prefix : <http://example.org/default#> .\n",
        );
        assert_eq!(
            prefixes_of(ttl),
            vec![
                (String::new(), "http://example.org/default#".to_owned()),
                ("ex".to_owned(), "http://example.org/ns#".to_owned()),
                ("meta".to_owned(), "https://example.org/meta/".to_owned()),
            ]
        );
    }

    #[test]
    fn a_directive_quoted_in_a_literal_declares_nothing() {
        // The line-scanning recovery this replaced read every one of these as a
        // declaration; to the grammar each is the text of a string literal.
        let ttl = concat!(
            "@prefix ex: <http://example.org/ns#> .\n",
            "ex:Q ex:select \"\"\"\n",
            "PREFIX test: <urn:other:ns>\n",
            "@prefix ex: <urn:evil:> .\n",
            "SELECT * WHERE { ?s ?p ?o }\n",
            "\"\"\" .\n",
            "ex:R ex:comment '''\n",
            "prefix evil: <urn:ex:>\n",
            "@prefix\u{A0}nbsp: <urn:ex:> .\n",
            "''' .\n",
        );
        assert_eq!(
            prefixes_of(ttl),
            vec![("ex".to_owned(), "http://example.org/ns#".to_owned())]
        );
    }

    #[test]
    fn a_redeclared_prefix_reports_its_last_namespace() {
        // Turtle resolves each prefixed name against the declaration in force where it
        // sits (the dataset holds both IRIs); the reported map is the document's final
        // one.
        let ttl = concat!(
            "@prefix ex: <http://example.org/v1#> .\n",
            "ex:a ex:p ex:b .\n",
            "@prefix ex: <http://example.org/v2#> .\n",
            "ex:a ex:p ex:b .\n",
        );
        let document = parse_turtle_document(ttl, None).expect("parses");
        assert_eq!(document.dataset.quad_refs().count(), 2);
        assert_eq!(
            document.prefixes,
            vec![("ex".to_owned(), "http://example.org/v2#".to_owned())]
        );
    }

    #[test]
    fn a_relative_prefix_namespace_is_reported_resolved() {
        let document = parse_turtle_document(
            "@prefix rel: <vocab#> .\n@base <https://example.org/other/> .\n",
            Some("https://example.org/base/"),
        )
        .expect("parses");
        assert_eq!(
            document.prefixes,
            vec![(
                "rel".to_owned(),
                "https://example.org/base/vocab#".to_owned()
            )]
        );
        assert_eq!(document.base.as_deref(), Some("https://example.org/other/"));
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
        // LAW: test-only, self-built fixture Turtle (`ttl` above) — never
        // caller-supplied and never reachable through a binding, so the panicking
        // wrapper is sound here.
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

    /// A `PREFIX` line inside a string literal declares nothing on the recovery path
    /// either. The line scan this replaced put `PREFIX lit: <urn:lit:>` from the
    /// literal into the header of EVERY re-parsed statement, so the statement using
    /// the undeclared `lit:` parsed and only the missing object was reported. The
    /// codec's own state leaves `lit:` undeclared, so that statement is reported
    /// too; the control, which declares `lit:` properly, reports only the missing
    /// object.
    #[test]
    fn a_prefix_quoted_in_a_literal_declares_nothing_during_recovery() {
        let bad = concat!(
            "@prefix ex: <http://example.org/ns#> .\n",
            "ex:q ex:select \"\"\"\n",
            "PREFIX lit: <urn:lit:>\n",
            "\"\"\" .\n",
            "ex:a ex:p lit:b .\n", // `lit:` is declared nowhere but in the literal
            "ex:c ex:q .\n",       // missing object
        );
        let Err(errors) = parse_turtle_to_dataset(bad, None) else {
            panic!("malformed Turtle must error")
        };
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].contains("\"lit\""), "{errors:?}");

        // The control: `lit:` really declared — only the missing object is reported.
        let control = bad.replace("@prefix ex:", "@prefix lit: <urn:lit:> .\n@prefix ex:");
        let Err(errors) = parse_turtle_to_dataset(&control, None) else {
            panic!("malformed Turtle must error")
        };
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(!errors[0].contains("\"lit\""), "{errors:?}");
    }

    /// A directive applies to the statements after it, and a SPARQL-form `PREFIX`
    /// sharing a chunk with a malformed statement still reaches the statements that
    /// follow (the failure reports the directives read before it).
    #[test]
    fn recovery_carries_directives_forward_in_document_order() {
        let bad = concat!(
            "ex:early ex:p ex:o .\n", // `ex:` is not declared yet
            "PREFIX ex: <http://example.org/ns#>\n",
            "ex:a ex:p .\n", // missing object, in the same chunk as the PREFIX
            "ex:b ex:p ex:c .\n",
        );
        let Err(errors) = parse_turtle_to_dataset(bad, None) else {
            panic!("malformed Turtle must error")
        };
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].contains("\"ex\""), "{errors:?}");
        assert!(!errors[1].contains("\"ex\""), "{errors:?}");
    }

    /// A decimal's `.` ends no statement: the only defect reported is the real one.
    #[test]
    fn a_decimal_point_is_not_a_statement_terminator() {
        let bad = concat!(
            "@prefix ex: <http://example.org/ns#> .\n",
            "ex:a ex:p 1.5 .\n",
            "ex:a.b ex:p ex:c .\n",
            "ex:c ex:q .\n", // missing object
        );
        let Err(errors) = parse_turtle_to_dataset(bad, None) else {
            panic!("malformed Turtle must error")
        };
        assert_eq!(errors.len(), 1, "{errors:?}");
    }
}
