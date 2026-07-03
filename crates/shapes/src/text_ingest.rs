// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native RDF-text ingestion for the SHACL engine.
//!
//! The SHACL engine ingests two kinds of RDF text: the shapes graph (Turtle) and
//! the data graph (N-Triples). It used to drive the oxigraph `io` RDF parser
//! directly so it could (a) recover the document `@prefix` map — oxigraph stores
//! drop prefixes, but SHACL-AF `sh:select` queries reference prefixed names — and
//! (b) accumulate EVERY syntax error in one pass (item 4) instead of
//! short-circuiting on the first.
//!
//! This module reproduces both capabilities on top of the native purrdf
//! codecs ([`::purrdf::parse_dataset`]), which are lenient on the PurRDF
//! ontology's private-use `@x-purrdf-*` language tags (long BCP-47 subtags) and
//! carry no oxigraph `io` dependency:
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

use ::purrdf::parse_dataset;
use ::purrdf::RdfDataset;

/// Recover the document `(prefix, namespace)` map from Turtle/TriG source text.
///
/// Scans `@prefix pfx: <iri> .` and SPARQL-style `PREFIX pfx: <iri>` directives.
/// The native codec drops these once it folds to the IR, so this source scan is
/// the faithful replacement for the oxigraph `io` parser's `prefixes()`. Later
/// declarations of the same prefix win (last-writer), matching oxttl's final
/// prefix table.
pub fn extract_prefixes(text: &str) -> Vec<(String, String)> {
    let mut map: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for (prefix, namespace) in scan_prefixes(text) {
        map.insert(prefix, namespace);
    }
    map.into_iter().collect()
}

/// Iterate `(prefix, namespace)` directives in `text`, in source order.
fn scan_prefixes(text: &str) -> impl Iterator<Item = (String, String)> + '_ {
    text.lines().filter_map(|line| {
        let trimmed = line.trim_start();
        // `@prefix` (Turtle) or `PREFIX` (SPARQL, case-insensitive) directive head.
        let rest = trimmed
            .strip_prefix("@prefix")
            .or_else(|| trimmed.strip_prefix("PREFIX"))
            .or_else(|| trimmed.strip_prefix("prefix"))?;
        // The directive head must be followed by whitespace (so `@prefixfoo`
        // is not mistaken for a directive).
        if !rest.starts_with(|c: char| c.is_whitespace()) {
            return None;
        }
        let rest = rest.trim_start();
        // `pfx:` (possibly empty `:`) up to the `<`.
        let colon = rest.find(':')?;
        let angle = rest.find('<')?;
        if colon > angle {
            return None;
        }
        let prefix = rest[..colon].trim().to_owned();
        let after_angle = &rest[angle + 1..];
        let close = after_angle.find('>')?;
        let namespace = after_angle[..close].to_owned();
        Some((prefix, namespace))
    })
}

/// Parse a Turtle document into a frozen [`RdfDataset`] via the native codecs.
///
/// On a clean parse the dataset is returned. On a syntax error the document is
/// re-parsed one top-level statement at a time so EVERY independently-malformed
/// statement is reported (the multi-error contract, item 4); the returned
/// `Err` is the list of per-statement error strings.
pub fn parse_turtle_to_dataset(ttl: &str) -> Result<Arc<RdfDataset>, Vec<String>> {
    if ttl.is_empty() {
        return Ok(empty_dataset());
    }
    match parse_dataset(ttl.as_bytes(), "text/turtle", None) {
        Ok(dataset) => Ok(dataset),
        Err(_) => Err(turtle_statement_errors(ttl)),
    }
}

/// Parse an N-Triples document into a frozen [`RdfDataset`] via the native codecs,
/// accumulating every malformed line as its own error.
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
fn turtle_statement_errors(ttl: &str) -> Vec<String> {
    // The prefix/base directives every statement needs to resolve prefixed names.
    let header: String = ttl
        .lines()
        .filter(|line| {
            let t = line.trim_start();
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
        let trimmed = statement.trim();
        if trimmed.is_empty() || is_directive(trimmed) {
            continue;
        }
        let candidate = format!("{header}\n{trimmed}\n");
        if let Err(e) = parse_dataset(candidate.as_bytes(), "text/turtle", None) {
            errors.push(format!("Turtle parse error: {e}"));
        }
    }
    if errors.is_empty() {
        // The whole-document parse failed but no individual statement did (e.g. a
        // lexer-level break that consumes to EOF): re-surface the document error
        // as a single entry rather than swallowing it.
        if let Err(e) = parse_dataset(ttl.as_bytes(), "text/turtle", None) {
            errors.push(format!("Turtle parse error: {e}"));
        }
    }
    errors
}

/// Whether `statement` is a `@prefix`/`@base`/SPARQL `PREFIX`/`BASE` directive.
fn is_directive(statement: &str) -> bool {
    let t = statement.trim_start();
    t.starts_with("@prefix")
        || t.starts_with("@base")
        || t.starts_with("PREFIX")
        || t.starts_with("prefix")
        || t.starts_with("BASE")
        || t.starts_with("base")
}

/// Split Turtle source into top-level statements on the `.` terminator, ignoring
/// `.`s inside IRIs (`<...>`) and string literals (`"..."`, `'...'`).
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
                '.' if !in_iri => {
                    statements.push(std::mem::take(&mut current));
                }
                _ => {}
            },
        }
    }
    if !current.trim().is_empty() {
        statements.push(current);
    }
    statements
}

/// Enumerate per-line N-Triples parse errors by parsing each non-blank,
/// non-comment line independently.
fn ntriples_line_errors(data_nt: &str) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    for line in data_nt.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let candidate = format!("{trimmed}\n");
        if let Err(e) = parse_dataset(candidate.as_bytes(), "application/n-triples", None) {
            errors.push(format!("N-Triples parse error: {e}"));
        }
    }
    if errors.is_empty() {
        if let Err(e) = parse_dataset(data_nt.as_bytes(), "application/n-triples", None) {
            errors.push(format!("N-Triples parse error: {e}"));
        }
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
        let dataset = parse_turtle_to_dataset(ttl).expect("clean Turtle parses");
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
        let Err(errors) = parse_turtle_to_dataset(bad) else {
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
}
