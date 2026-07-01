// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! First-party RDF text → in-memory [`SerGraph`] front-end for the line / Turtle
//! family (N-Triples, N-Quads, Turtle, TriG).
//!
//! This module REPLACES the `purrdf-gts` `from_ntriples` / `from_nquads` /
//! `from_turtle` / `from_trig` text codecs (which delegated all RDF text parsing
//! to the EXTERNAL crate, FORBIDDEN here) with an in-repo parser that lowers
//! directly to the first-party in-memory [`SerGraph`] the purrdf-gts roundtrip used to
//! produce — WITHOUT the text→GTS-bytes→reader indirection.
//!
//! ## Byte-identity discipline
//!
//! The downstream fold ([`super::parse::dataset_from_ser_graph`]) re-interns its
//! [`RdfDatasetBuilder`] from `graph.reifiers` THEN `graph.quads`, in order, so the
//! frozen IR's term table is the first-seen interning order over those rows. To stay
//! BYTE-IDENTICAL to the prior purrdf-gts path this parser reproduces, exactly, the
//! `from_nquads` `build_gts` structure: terms in first-seen order, quads in statement
//! order, reifiers in encounter order, the `rdf:reifies` statement-layer shorthand, and
//! the self-reifier sentinel for inline quoted-triple TERMS. The prior purrdf-gts
//! `Writer` / `read` roundtrip was append-order-preserving (it did NOT sort
//! terms/quads/reifiers), so the in-memory graph the reader produced was already exactly
//! this structure — only the serialize / deserialize hop, and the `\uXXXX` UCHAR-in-IRI
//! gap, are removed.
//!
//! ## The UCHAR fix (W3C `test060`)
//!
//! The purrdf-gts N-Quads/Turtle IRIREF readers took the raw bytes between `<` and
//! `>` and REJECTED a backslash as a forbidden IRI character, so `\uXXXX` UCHAR
//! escapes inside an IRIREF (`<urn:ex:s:000:s⁰1>`) failed to parse. This
//! front-end decodes `\u`/`\U` UCHAR escapes inside IRIREFs (via the proven
//! sparql-algebra lexer, which decodes them in `IRIREF` position), so `test060`
//! now parses.

use std::collections::HashMap;

use purrdf_sparql_algebra::lexer::{tokenize, tokenize_turtle, Spanned, Token};

use super::media_type::NativeRdfFormat;
use super::ser_model::{SerGraph, SerTerm, SerTermKind, SerTriple3};
use crate::RdfDiagnostic;

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

fn err(detail: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("native-codec-parse", detail.into())
}

/// A parsed RDF term node, mirroring the `from_nquads` `Node` so the
/// `build_gts` lowering is structurally identical.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Iri(String),
    Bnode(String),
    Literal {
        value: String,
        lang: Option<String>,
        direction: Option<String>,
        datatype: Option<String>,
    },
    Triple(Box<Node>, Box<Node>, Box<Node>),
}

/// Parse RDF text of one of the four line/Turtle-family `format`s into the first-party
/// in-memory [`SerGraph`] that the downstream statement-layer fold consumes. Mirrors the
/// `from_*` structure exactly (see the module note) so the resulting IR is byte-identical
/// to the prior purrdf-gts path, with the UCHAR-in-IRI gap fixed.
pub fn parse_to_gts_graph(
    format: NativeRdfFormat,
    text: &str,
    base_iri: Option<&str>,
) -> Result<SerGraph, RdfDiagnostic> {
    let statements = match format {
        NativeRdfFormat::NTriples => parse_lines(text, false)?,
        NativeRdfFormat::NQuads => parse_lines(text, true)?,
        NativeRdfFormat::Turtle => DocParser::new(text, base_iri, false).parse()?,
        NativeRdfFormat::TriG => DocParser::new(text, base_iri, true).parse()?,
        NativeRdfFormat::RdfXml => {
            return Err(err("RDF/XML is not a line/Turtle-family format"));
        }
    };
    build_gts_graph(&statements)
}

// ───────────────────────────────────────────────────────────────────────────────
// N-Triples / N-Quads (line-oriented; absolute IRIs only)
// ───────────────────────────────────────────────────────────────────────────────

/// One statement: subject, predicate, object, and (N-Quads) an optional graph name.
type Statement = Vec<Node>;

/// Parse N-Triples (`allow_graph == false`) / N-Quads (`allow_graph == true`).
///
/// Line-oriented like the purrdf-gts parser: blank lines and `#`-comment lines are
/// skipped, every other line is one statement of 3 (NT) or 3-or-4 (NQ) terms. The
/// `<<( s p o )>>` quoted-triple TERM is admitted in subject (NQ only) and object
/// position; IRIREFs are UCHAR-decoded (the test060 fix).
fn parse_lines(text: &str, allow_graph: bool) -> Result<Vec<Statement>, RdfDiagnostic> {
    let mut statements = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let tokens = tokenize(line).map_err(|e| err(format!("{e} in {line:?}")))?;
        let mut cursor = TokenCursor::new(&tokens, line);
        let mut nodes = Vec::new();
        while !cursor.at_statement_end() {
            nodes.push(cursor.term(allow_graph)?);
        }
        cursor.expect_dot()?;
        let valid_len = if allow_graph {
            nodes.len() == 3 || nodes.len() == 4
        } else {
            nodes.len() == 3
        };
        if !valid_len {
            return Err(err(format!(
                "expected {} terms, got {}: {line:?}",
                if allow_graph { "3 or 4" } else { "3" },
                nodes.len(),
            )));
        }
        validate_statement(&nodes, line, allow_graph)?;
        statements.push(nodes);
    }
    Ok(statements)
}

/// A cursor over one line's lexer tokens, parsing N-Triples/N-Quads terms.
struct TokenCursor<'a> {
    tokens: &'a [Spanned],
    pos: usize,
    line: &'a str,
}

impl<'a> TokenCursor<'a> {
    fn new(tokens: &'a [Spanned], line: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            line,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).map(|s| s.token.clone());
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// True at the statement terminator `.` or the end of the token stream.
    fn at_statement_end(&self) -> bool {
        matches!(self.peek(), None | Some(Token::Dot))
    }

    fn expect_dot(&mut self) -> Result<(), RdfDiagnostic> {
        match self.bump() {
            Some(Token::Dot) | None => Ok(()),
            other => Err(err(format!(
                "expected '.' terminator, found {other:?} in {:?}",
                self.line
            ))),
        }
    }

    /// Parse one term in N-Triples/N-Quads syntax. `allow_triple_subject` is unused
    /// here (the lexer admits `<<( … )>>` everywhere); positional validity is checked
    /// later by [`validate_statement`], exactly as the purrdf-gts parser does.
    fn term(&mut self, _allow_triple_subject: bool) -> Result<Node, RdfDiagnostic> {
        match self.peek() {
            Some(Token::TripleOpen) => self.quoted_triple(),
            Some(Token::Iri(_)) => {
                let Some(Token::Iri(value)) = self.bump() else {
                    unreachable!()
                };
                validate_iri(&value, self.line)?;
                Ok(Node::Iri(value))
            }
            Some(Token::BlankNodeLabel(_)) => {
                let Some(Token::BlankNodeLabel(label)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Bnode(label))
            }
            Some(Token::StringLit(_)) => self.literal(),
            other => Err(err(format!(
                "unexpected token {other:?} in {:?}",
                self.line
            ))),
        }
    }

    /// `<<( s p o )>>` quoted-triple term (the only triple form N-Triples/N-Quads
    /// admit). The purrdf-gts N-Quads parser requires the parenthesized form.
    fn quoted_triple(&mut self) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::TripleOpen)?;
        self.expect(&Token::LParen)?;
        let s = self.term(true)?;
        let p = self.term(true)?;
        let o = self.term(true)?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::TripleClose)?;
        Ok(Node::Triple(Box::new(s), Box::new(p), Box::new(o)))
    }

    /// A string literal with an optional `@lang[--dir]` tag or `^^<datatype>`.
    fn literal(&mut self) -> Result<Node, RdfDiagnostic> {
        let Some(Token::StringLit(value)) = self.bump() else {
            unreachable!()
        };
        let mut lang = None;
        let mut direction = None;
        let mut datatype = None;
        match self.peek() {
            Some(Token::LangTag(_)) => {
                let Some(Token::LangTag(raw)) = self.bump() else {
                    unreachable!()
                };
                let (base, dir) = split_lang_direction(&raw, self.line)?;
                validate_language_tag(&base, self.line)?;
                lang = Some(base);
                direction = dir;
            }
            Some(Token::HatHat) => {
                self.bump();
                let Some(Token::Iri(iri)) = self.bump() else {
                    return Err(err(format!("datatype must be an IRI in {:?}", self.line)));
                };
                validate_iri(&iri, self.line)?;
                if matches!(iri.as_str(), RDF_LANG_STRING | RDF_DIR_LANG_STRING) {
                    return Err(err(format!(
                        "literal cannot explicitly use the RDF language-string datatype in {:?}",
                        self.line
                    )));
                }
                datatype = Some(iri);
            }
            _ => {}
        }
        Ok(Node::Literal {
            value,
            lang,
            direction,
            datatype,
        })
    }

    fn expect(&mut self, token: &Token) -> Result<(), RdfDiagnostic> {
        if self.peek() == Some(token) {
            self.pos += 1;
            Ok(())
        } else {
            Err(err(format!(
                "expected {token:?}, found {:?} in {:?}",
                self.peek(),
                self.line
            )))
        }
    }
}

/// Split an N-Quads language tag into `(language, direction)`: `ar--rtl` →
/// `("ar", Some("rtl"))`; a plain `en` → `("en", None)`. A `--ltr`/`--rtl` suffix is
/// the RDF 1.2 base-direction marker; any other `--`-suffix is rejected, mirroring
/// purrdf-gts.
fn split_lang_direction(raw: &str, line: &str) -> Result<(String, Option<String>), RdfDiagnostic> {
    if let Some((base, dir)) = raw.rsplit_once("--") {
        if matches!(dir, "ltr" | "rtl") && !base.is_empty() {
            Ok((base.to_owned(), Some(dir.to_owned())))
        } else {
            Err(err(format!("invalid literal base direction in {line:?}")))
        }
    } else {
        Ok((raw.to_owned(), None))
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// Term validation (positional + IRI/lang shape), mirroring the prior purrdf-gts parser
// ───────────────────────────────────────────────────────────────────────────────

/// Whether `value` carries an absolute-IRI scheme (`scheme:`), matching the
/// purrdf-gts `has_iri_scheme`.
fn has_iri_scheme(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    for ch in chars {
        if ch == ':' {
            return true;
        }
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
            return false;
        }
    }
    false
}

/// Validate an absolute IRI's shape after UCHAR-decoding.
///
/// The N-Triples/N-Quads IRIREF grammar is `'<' ([^#x00-#x20<>"{}|^`\] | UCHAR)* '>'`:
/// a character is forbidden as a RAW byte but PERMITTED when introduced by a `UCHAR`
/// escape. The lexer ([`tokenize`]) already enforces the raw-byte restriction — its
/// IRIREF scan STOPS at a raw whitespace / `< " { } | ^ \`` — and decodes every
/// `\u`/`\U` escape, so by the time the value reaches here any otherwise-forbidden
/// character can ONLY have come from a (legal) UCHAR. So this checks ONLY the
/// absolute-IRI requirement (N-Triples/N-Quads admit no relative IRIs); rejecting the
/// decoded special characters would wrongly fail legal UCHAR IRIs such as
/// `<urn:ex: >` (W3C test060), whose canonical form keeps the decoded character.
fn validate_iri(value: &str, line: &str) -> Result<(), RdfDiagnostic> {
    if value.is_empty() || value.starts_with("//") || !has_iri_scheme(value) {
        return Err(err(format!("IRI must be absolute: {line:?}")));
    }
    Ok(())
}

/// Validate a BCP-47 language tag, including the long private-use subtag relaxation
/// (`x-purrdf-…`) purrdf-gts applies.
fn validate_language_tag(tag: &str, line: &str) -> Result<(), RdfDiagnostic> {
    let mut parts = tag.split('-');
    let Some(primary) = parts.next() else {
        return Err(err(format!("empty language tag in {line:?}")));
    };
    if primary.is_empty()
        || primary.len() > 8
        || !primary.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return Err(err(format!("invalid language tag {tag:?} in {line:?}")));
    }
    let mut private_use = primary.eq_ignore_ascii_case("x");
    for subtag in parts {
        let alnum = !subtag.is_empty() && subtag.bytes().all(|byte| byte.is_ascii_alphanumeric());
        let acceptable = if private_use {
            alnum
        } else {
            alnum && subtag.len() <= 8
        };
        if !acceptable {
            return Err(err(format!("invalid language tag {tag:?} in {line:?}")));
        }
        if subtag.eq_ignore_ascii_case("x") {
            private_use = true;
        }
    }
    Ok(())
}

fn node_is(node: &Node, kinds: &[fn(&Node) -> bool]) -> bool {
    kinds.iter().any(|p| p(node))
}

fn is_iri(node: &Node) -> bool {
    matches!(node, Node::Iri(_))
}
fn is_bnode(node: &Node) -> bool {
    matches!(node, Node::Bnode(_))
}
fn is_literal(node: &Node) -> bool {
    matches!(node, Node::Literal { .. })
}

fn validate_subject(
    node: &Node,
    line: &str,
    allow_triple_subject: bool,
) -> Result<(), RdfDiagnostic> {
    if node_is(node, &[is_iri, is_bnode]) {
        return Ok(());
    }
    if allow_triple_subject {
        if let Node::Triple(s, p, o) = node {
            return validate_triple(s, p, o, line, allow_triple_subject);
        }
    }
    Err(err(format!("invalid subject term: {line:?}")))
}

fn validate_predicate(node: &Node, line: &str) -> Result<(), RdfDiagnostic> {
    if is_iri(node) {
        Ok(())
    } else {
        Err(err(format!("predicate must be IRI: {line:?}")))
    }
}

fn validate_object(
    node: &Node,
    line: &str,
    allow_triple_subject: bool,
) -> Result<(), RdfDiagnostic> {
    if node_is(node, &[is_iri, is_bnode, is_literal]) {
        return Ok(());
    }
    if let Node::Triple(s, p, o) = node {
        return validate_triple(s, p, o, line, allow_triple_subject);
    }
    Err(err(format!("invalid object term: {line:?}")))
}

fn validate_triple(
    s: &Node,
    p: &Node,
    o: &Node,
    line: &str,
    allow_triple_subject: bool,
) -> Result<(), RdfDiagnostic> {
    validate_subject(s, line, allow_triple_subject)?;
    validate_predicate(p, line)?;
    validate_object(o, line, allow_triple_subject)
}

fn validate_statement(nodes: &[Node], line: &str, allow_graph: bool) -> Result<(), RdfDiagnostic> {
    validate_subject(&nodes[0], line, allow_graph)?;
    validate_predicate(&nodes[1], line)?;
    validate_object(&nodes[2], line, allow_graph)?;
    if let Some(graph_name) = nodes.get(3) {
        if !node_is(graph_name, &[is_iri, is_bnode]) {
            return Err(err(format!("invalid graph name term: {line:?}")));
        }
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────────
// Turtle / TriG (prefixes, base, collections, BNPL, quoted/reifying triples)
// ───────────────────────────────────────────────────────────────────────────────

/// A recursive-descent Turtle/TriG parser over the sparql-algebra token stream. It
/// emits the SAME flat statement list (subject/predicate/object[/graph] `Node`s) the
/// purrdf-gts Turtle/TriG parser produced before lowering through `from_nquads`'s
/// `build_gts`, so the resulting [`SerGraph`] is byte-identical.
struct DocParser<'a> {
    tokens: Vec<Spanned>,
    pos: usize,
    prefixes: HashMap<String, String>,
    base_iri: Option<String>,
    bnode_counter: usize,
    allow_named_graphs: bool,
    statements: Vec<Statement>,
    src: &'a str,
}

impl<'a> DocParser<'a> {
    fn new(text: &'a str, base_iri: Option<&str>, allow_named_graphs: bool) -> Self {
        let mut prefixes = HashMap::new();
        prefixes.insert("rdf".to_owned(), RDF_NS.to_owned());
        Self {
            tokens: Vec::new(),
            pos: 0,
            prefixes,
            base_iri: base_iri.map(str::to_owned),
            bnode_counter: 0,
            allow_named_graphs,
            statements: Vec::new(),
            src: text,
        }
    }

    fn parse(mut self) -> Result<Vec<Statement>, RdfDiagnostic> {
        // Turtle/TriG admit a bare `/` in a prefixed-name local part (e.g.
        // `purrdf:report/shacl/sarif`), matching oxigraph/purrdf-gts leniency.
        // Turtle has no `/` operator, so this is unambiguous in term position;
        // the SPARQL `tokenize` keeps `/` as the property-path operator.
        self.tokens = tokenize_turtle(self.src).map_err(|e| err(e.to_string()))?;
        while self.peek().is_some() {
            if self.try_directive()? {
                continue;
            }
            if self.eat_kw("GRAPH") {
                if !self.allow_named_graphs {
                    return Err(err("Turtle input cannot contain GRAPH blocks"));
                }
                let graph = self.term(None)?;
                self.expect(&Token::LBrace)?;
                self.graph_block(graph)?;
                continue;
            }
            let first = self.term(None)?;
            if self.eat(&Token::LBrace) {
                if !self.allow_named_graphs {
                    return Err(err("Turtle input cannot contain graph blocks"));
                }
                self.graph_block(first)?;
            } else {
                self.statement_after_subject(first, None)?;
            }
        }
        Ok(self.statements)
    }

    /// Consume a `@prefix`/`@base`/`@version` or `PREFIX`/`BASE`/`VERSION` directive
    /// when present. Returns whether one was consumed.
    fn try_directive(&mut self) -> Result<bool, RdfDiagnostic> {
        // `@prefix` / `@base` / `@version` lex as a `LangTag` (the `@` form).
        if let Some(Token::LangTag(tag)) = self.peek() {
            match tag.as_str() {
                "prefix" => {
                    self.pos += 1;
                    self.prefix_directive(true)?;
                    return Ok(true);
                }
                "base" => {
                    self.pos += 1;
                    self.base_directive(true)?;
                    return Ok(true);
                }
                "version" => {
                    self.pos += 1;
                    self.version_string()?;
                    self.expect(&Token::Dot)?;
                    return Ok(true);
                }
                _ => {}
            }
        }
        if self.eat_kw("PREFIX") {
            self.prefix_directive(false)?;
            return Ok(true);
        }
        if self.eat_kw("BASE") {
            self.base_directive(false)?;
            return Ok(true);
        }
        if self.eat_kw("VERSION") {
            self.version_string()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn prefix_directive(&mut self, require_dot: bool) -> Result<(), RdfDiagnostic> {
        let (prefix, _) = self.expect_prefix_ns()?;
        let iri = self.expect_iri_raw()?;
        self.prefixes.insert(prefix, iri);
        if require_dot {
            self.expect(&Token::Dot)?;
        } else {
            self.eat(&Token::Dot);
        }
        Ok(())
    }

    fn base_directive(&mut self, require_dot: bool) -> Result<(), RdfDiagnostic> {
        let iri = self.expect_iri_raw()?;
        if !has_iri_scheme(&iri) {
            return Err(err(format!("base IRI must be absolute: {iri:?}")));
        }
        self.base_iri = Some(iri);
        if require_dot {
            self.expect(&Token::Dot)?;
        } else {
            self.eat(&Token::Dot);
        }
        Ok(())
    }

    /// A `VERSION`/`@version` argument: a **single-line** string literal, recorded only
    /// to be accepted and skipped. RDF 1.2 forbids a triple-quoted (`'''`/`"""`) long
    /// string here, so the raw span is checked and a long form is rejected (the lexer
    /// collapses both quote styles into one `StringLit`, so the source span is the only
    /// place the distinction survives).
    fn version_string(&mut self) -> Result<(), RdfDiagnostic> {
        let span = self.tokens.get(self.pos).map(|s| (s.start, s.end));
        match self.bump() {
            Some(Token::StringLit(_)) => {
                if let Some((start, _)) = span {
                    let raw = &self.src[start..];
                    if raw.starts_with("\"\"\"") || raw.starts_with("'''") {
                        return Err(err(
                            "version directive needs a single-line string, found a triple-quoted string",
                        ));
                    }
                }
                Ok(())
            }
            other => Err(err(format!(
                "version directive needs a string, found {other:?}"
            ))),
        }
    }

    /// A bare `prefix:` namespace (PNAME_NS); the local part must be empty.
    fn expect_prefix_ns(&mut self) -> Result<(String, String), RdfDiagnostic> {
        match self.bump() {
            Some(Token::PrefixedName(p, l)) if l.is_empty() => Ok((p, l)),
            other => Err(err(format!("expected a prefix namespace, found {other:?}"))),
        }
    }

    /// An IRIREF, returned UNRESOLVED (for `@prefix`/`@base` targets). The lexer has
    /// already UCHAR-decoded it.
    fn expect_iri_raw(&mut self) -> Result<String, RdfDiagnostic> {
        match self.bump() {
            Some(Token::Iri(s)) => Ok(s),
            other => Err(err(format!("expected an IRIREF, found {other:?}"))),
        }
    }

    fn term(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        match self.peek() {
            Some(Token::TripleOpen) => {
                // Distinguish the value form `<<( s p o )>>` from the reifying form
                // `<< s p o [~r] >>` by the immediately-following `(`.
                if self.peek2() == Some(&Token::LParen) {
                    self.parenthesized_quoted_triple(graph)
                } else {
                    self.reifying_triple(graph)
                }
            }
            Some(Token::Iri(_)) => {
                let Some(Token::Iri(raw)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Iri(self.resolve_iri(&raw)))
            }
            Some(Token::PrefixedName(_, _)) => {
                let Some(Token::PrefixedName(prefix, local)) = self.bump() else {
                    unreachable!()
                };
                self.resolve_prefixed(&prefix, &local)
            }
            Some(Token::BlankNodeLabel(_)) => {
                let Some(Token::BlankNodeLabel(label)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Bnode(label))
            }
            Some(Token::Anon) => {
                self.pos += 1;
                Ok(self.next_bnode())
            }
            Some(Token::LBracket) => self.blank_node_property_list(graph),
            Some(Token::LParen) => self.collection(graph),
            Some(Token::StringLit(_)) => self.literal(),
            Some(Token::Integer(_)) | Some(Token::Decimal(_)) | Some(Token::Double(_)) => {
                self.numeric_literal("")
            }
            // A signed numeric literal `+N` / `-N`: the lexer emits the sign as a
            // separate `Plus`/`Minus` token, so consume it and fold it back into the
            // lexical form (kept verbatim, e.g. `-200.0`), matching purrdf-gts.
            Some(Token::Plus) | Some(Token::Minus)
                if matches!(
                    self.peek2(),
                    Some(Token::Integer(_) | Token::Decimal(_) | Token::Double(_))
                ) =>
            {
                let sign = if self.eat(&Token::Minus) {
                    "-"
                } else {
                    self.expect(&Token::Plus)?;
                    "+"
                };
                self.numeric_literal(sign)
            }
            Some(Token::Word(w)) if w == "true" || w == "false" => {
                let Some(Token::Word(value)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Literal {
                    value,
                    lang: None,
                    direction: None,
                    datatype: Some(XSD_BOOLEAN.to_owned()),
                })
            }
            other => Err(err(format!(
                "unexpected token {other:?} in Turtle/TriG term"
            ))),
        }
    }

    /// A subject/object inside a triple term. Non-empty `[ … ]` / `( … )` would emit
    /// extra triples that cannot live inside a triple term, so they are rejected
    /// (W3C-conformant); an empty `[]` / `()` is a plain term and is allowed.
    fn quoted_component(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        match self.peek() {
            Some(Token::LBracket) => Err(err(
                "blank-node property list is not allowed inside a quoted triple",
            )),
            Some(Token::LParen) => {
                if self.peek2() == Some(&Token::RParen) {
                    self.term(graph)
                } else {
                    Err(err("RDF collection is not allowed inside a quoted triple"))
                }
            }
            _ => self.term(graph),
        }
    }

    fn predicate(&mut self) -> Result<Node, RdfDiagnostic> {
        if matches!(self.peek(), Some(Token::Word(w)) if w == "a") {
            self.pos += 1;
            return Ok(Node::Iri(RDF_TYPE.to_owned()));
        }
        self.term(None)
    }

    fn parenthesized_quoted_triple(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::TripleOpen)?;
        self.expect(&Token::LParen)?;
        let s = self.quoted_component(graph)?;
        let p = self.predicate()?;
        let o = self.quoted_component(graph)?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::TripleClose)?;
        Ok(Node::Triple(Box::new(s), Box::new(p), Box::new(o)))
    }

    /// A triple TERM in `rdf:reifies` object position: `<<( s p o )>>` (canonical) or
    /// the legacy non-parenthesized `<< s p o >>` (purrdf pre-0.9.11 triple-term
    /// serialization). Always a [`Node::Triple`] — never a minted reifier — because the
    /// object of `rdf:reifies` denotes the reified triple itself.
    fn reifies_object_triple_term(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::TripleOpen)?;
        let parenthesized = self.eat(&Token::LParen);
        let s = self.quoted_component(graph)?;
        let p = self.predicate()?;
        let o = self.quoted_component(graph)?;
        if parenthesized {
            self.expect(&Token::RParen)?;
        }
        self.expect(&Token::TripleClose)?;
        Ok(Node::Triple(Box::new(s), Box::new(p), Box::new(o)))
    }

    /// RDF 1.2 reifying triple `<< s p o ~r? >>` in subject/object position: emits
    /// `r rdf:reifies <<( s p o )>>` and returns the reifier `r`. With an explicit
    /// `~ id`, `r` is that id; otherwise (`~` alone, or no reifier at all) a fresh
    /// blank node is minted. The inner triple is NOT independently asserted here — the
    /// reifiedTriple denotes its reifier, so only the `rdf:reifies` statement is emitted.
    fn reifying_triple(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::TripleOpen)?;
        let s = self.quoted_component(graph)?;
        let p = self.predicate()?;
        let o = self.quoted_component(graph)?;
        let reifier = if self.eat(&Token::Tilde) {
            if self.at_reifier_id() {
                self.term(graph)?
            } else {
                self.next_bnode()
            }
        } else {
            self.next_bnode()
        };
        self.expect(&Token::TripleClose)?;
        self.emit_reifies(&reifier, &s, &p, &o, graph);
        Ok(reifier)
    }

    fn blank_node_property_list(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        // `[]` lexes as a single `Anon`; `[ … ]` opens with `LBracket`.
        if self.eat(&Token::Anon) {
            return Ok(self.next_bnode());
        }
        self.expect(&Token::LBracket)?;
        let subject = self.next_bnode();
        if !self.eat(&Token::RBracket) {
            self.predicate_object_list(&subject, graph)?;
            self.expect(&Token::RBracket)?;
        }
        Ok(subject)
    }

    fn collection(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::LParen)?;
        let mut items = Vec::new();
        while !self.eat(&Token::RParen) {
            if self.peek().is_none() {
                return Err(err("unterminated RDF collection"));
            }
            items.push(self.term(graph)?);
        }
        if items.is_empty() {
            return Ok(Node::Iri(RDF_NIL.to_owned()));
        }
        let cells: Vec<Node> = (0..items.len()).map(|_| self.next_bnode()).collect();
        for (index, item) in items.into_iter().enumerate() {
            let current = cells[index].clone();
            let rest = if index + 1 == cells.len() {
                Node::Iri(RDF_NIL.to_owned())
            } else {
                cells[index + 1].clone()
            };
            self.emit(&current, &Node::Iri(RDF_FIRST.to_owned()), &item, graph);
            self.emit(&current, &Node::Iri(RDF_REST.to_owned()), &rest, graph);
        }
        Ok(cells.into_iter().next().expect("non-empty collection"))
    }

    fn literal(&mut self) -> Result<Node, RdfDiagnostic> {
        let Some(Token::StringLit(value)) = self.bump() else {
            unreachable!()
        };
        let mut lang = None;
        let mut direction = None;
        let mut datatype = None;
        match self.peek() {
            Some(Token::LangTag(_)) => {
                let Some(Token::LangTag(raw)) = self.bump() else {
                    unreachable!()
                };
                // purrdf-gts's Turtle parser keeps the raw `@lang` text (including any
                // `--dir`) on the literal `lang` field and lowers it to an N-Quads
                // `@lang` token, so the direction is re-parsed at the `from_nquads`
                // stage. To match that exactly, split here into lang + direction.
                let (base, dir) = split_lang_direction(&raw, "<turtle>")?;
                lang = Some(base);
                direction = dir;
            }
            Some(Token::HatHat) => {
                self.bump();
                datatype = Some(self.datatype_iri()?);
            }
            _ => {}
        }
        Ok(Node::Literal {
            value,
            lang,
            direction,
            datatype,
        })
    }

    fn datatype_iri(&mut self) -> Result<String, RdfDiagnostic> {
        match self.bump() {
            Some(Token::Iri(raw)) => Ok(self.resolve_iri(&raw)),
            Some(Token::PrefixedName(prefix, local)) => {
                match self.resolve_prefixed(&prefix, &local)? {
                    Node::Iri(iri) => Ok(iri),
                    _ => unreachable!("resolve_prefixed yields an IRI node"),
                }
            }
            other => Err(err(format!("expected a datatype IRI, found {other:?}"))),
        }
    }

    fn numeric_literal(&mut self, sign: &str) -> Result<Node, RdfDiagnostic> {
        match self.bump() {
            Some(Token::Integer(lexical)) => Ok(numeric(format!("{sign}{lexical}"), XSD_INTEGER)),
            Some(Token::Decimal(lexical)) => Ok(numeric(format!("{sign}{lexical}"), XSD_DECIMAL)),
            Some(Token::Double(lexical)) => Ok(numeric(format!("{sign}{lexical}"), XSD_DOUBLE)),
            other => Err(err(format!("expected a numeric literal, found {other:?}"))),
        }
    }

    fn graph_block(&mut self, graph: Node) -> Result<(), RdfDiagnostic> {
        if !matches!(graph, Node::Iri(_) | Node::Bnode(_)) {
            return Err(err("graph block name must be an IRI or blank node"));
        }
        while !self.eat(&Token::RBrace) {
            if self.peek().is_none() {
                return Err(err("unterminated graph block"));
            }
            let subject = self.term(Some(&graph))?;
            self.statement_after_subject_in_graph(subject, &graph)?;
        }
        Ok(())
    }

    fn statement_after_subject(
        &mut self,
        subject: Node,
        graph: Option<&Node>,
    ) -> Result<(), RdfDiagnostic> {
        // A self-asserting subject (reifying triple or blank-node property list) may
        // end immediately at `.`; a plain subject still needs a predicate-object list.
        if !self.at(&Token::Dot) {
            self.predicate_object_list(&subject, graph)?;
        }
        self.expect(&Token::Dot)
    }

    fn statement_after_subject_in_graph(
        &mut self,
        subject: Node,
        graph: &Node,
    ) -> Result<(), RdfDiagnostic> {
        if !(self.at(&Token::Dot) || self.at(&Token::RBrace)) {
            self.predicate_object_list(&subject, Some(graph))?;
        }
        // The trailing `.` is optional for the final statement before `}`.
        if self.eat(&Token::Dot) || self.at(&Token::RBrace) {
            Ok(())
        } else {
            Err(err("expected '.' to terminate statement in graph block"))
        }
    }

    fn predicate_object_list(
        &mut self,
        subject: &Node,
        graph: Option<&Node>,
    ) -> Result<(), RdfDiagnostic> {
        loop {
            let predicate = self.predicate()?;
            loop {
                // The object of `rdf:reifies` is a triple TERM. Parse `<<` here as a
                // triple term whether or not it carries parens, tolerating purrdf's
                // legacy non-parenthesized `<< s p o >>` triple-term serialization in
                // addition to the canonical `<<( s p o )>>` — in EVERY other position a
                // bare `<< … >>` keeps its W3C reifying-triple meaning (`reifying_triple`).
                let object = if matches!(&predicate, Node::Iri(p) if p == RDF_REIFIES)
                    && self.at(&Token::TripleOpen)
                {
                    self.reifies_object_triple_term(graph)?
                } else {
                    self.term(graph)?
                };
                self.emit(subject, &predicate, &object, graph);
                self.maybe_reify_and_annotate(subject, &predicate, &object, graph)?;
                if self.eat(&Token::Comma) {
                    continue;
                }
                break;
            }
            if self.eat(&Token::Semicolon) {
                // `Pipe` terminates a trailing `;` inside a `{| … |}` annotation block.
                if self.at(&Token::Dot)
                    || self.at(&Token::RBracket)
                    || self.at(&Token::RBrace)
                    || self.at(&Token::Pipe)
                {
                    break;
                }
                continue;
            }
            break;
        }
        Ok(())
    }

    /// The RDF 1.2 reifier (`~ id`) / annotation (`{| pol |}`) suffix on a just-emitted
    /// `s p o` triple, matching the W3C RDF 1.2 Turtle/TriG reification expansion:
    ///
    /// - `~ id?` mints (or names) a reifier `r` and emits `r rdf:reifies <<( s p o )>>`.
    /// - `{| pol |}` reuses the immediately-preceding `~`-reifier if one is pending,
    ///   else mints a fresh reifier (with its own `rdf:reifies` triple), then evaluates
    ///   `pol` with that reifier as subject.
    ///
    /// Multiple suffixes chain (`~r1 ~r2`, `{| a |} {| b |}`); each annotation block
    /// consumes at most the one pending reifier, so a second block mints fresh.
    /// `~` is `Token::Tilde`; `{|`/`|}` are the `LBrace Pipe` / `Pipe RBrace` pairs.
    fn maybe_reify_and_annotate(
        &mut self,
        s: &Node,
        p: &Node,
        o: &Node,
        graph: Option<&Node>,
    ) -> Result<(), RdfDiagnostic> {
        let mut pending: Option<Node> = None;
        loop {
            if self.eat(&Token::Tilde) {
                let reifier = if self.at_reifier_id() {
                    self.term(graph)?
                } else {
                    self.next_bnode()
                };
                self.emit_reifies(&reifier, s, p, o, graph);
                pending = Some(reifier);
            } else if self.at(&Token::LBrace) && self.peek2() == Some(&Token::Pipe) {
                self.bump(); // `{`
                self.bump(); // `|`
                let reifier = match pending.take() {
                    Some(reifier) => reifier,
                    None => {
                        let reifier = self.next_bnode();
                        self.emit_reifies(&reifier, s, p, o, graph);
                        reifier
                    }
                };
                self.predicate_object_list(&reifier, graph)?;
                self.expect(&Token::Pipe)?; // `|` of `|}`
                self.expect(&Token::RBrace)?; // `}` of `|}`
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Emit `reifier rdf:reifies <<( s p o )>>` (the triple term is self-reifying via
    /// [`Node::Triple`]), the canonical RDF 1.2 reification triple.
    fn emit_reifies(&mut self, reifier: &Node, s: &Node, p: &Node, o: &Node, graph: Option<&Node>) {
        let triple_term = Node::Triple(
            Box::new(s.clone()),
            Box::new(p.clone()),
            Box::new(o.clone()),
        );
        self.emit(
            reifier,
            &Node::Iri(RDF_REIFIES.to_owned()),
            &triple_term,
            graph,
        );
    }

    /// Whether the next token can begin a reifier identifier (`iri | BlankNode`).
    fn at_reifier_id(&self) -> bool {
        matches!(
            self.peek(),
            Some(
                Token::Iri(_) | Token::PrefixedName(_, _) | Token::BlankNodeLabel(_) | Token::Anon
            )
        )
    }

    fn emit(&mut self, subject: &Node, predicate: &Node, object: &Node, graph: Option<&Node>) {
        let mut nodes = vec![subject.clone(), predicate.clone(), object.clone()];
        if let Some(graph) = graph {
            nodes.push(graph.clone());
        }
        self.statements.push(nodes);
    }

    fn next_bnode(&mut self) -> Node {
        let id = self.bnode_counter;
        self.bnode_counter += 1;
        Node::Bnode(deterministic_label(id))
    }

    fn resolve_iri(&self, raw: &str) -> String {
        if has_iri_scheme(raw) {
            raw.to_owned()
        } else if let Some(base) = &self.base_iri {
            resolve_relative_iri(base, raw)
        } else {
            raw.to_owned()
        }
    }

    fn resolve_prefixed(&self, prefix: &str, local: &str) -> Result<Node, RdfDiagnostic> {
        match self.prefixes.get(prefix) {
            Some(base) => Ok(Node::Iri(format!("{base}{local}"))),
            None => Err(err(format!("unknown prefix {prefix:?}"))),
        }
    }

    // token cursor helpers

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    fn peek2(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1).map(|s| &s.token)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).map(|s| s.token.clone());
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn at(&self, token: &Token) -> bool {
        self.peek() == Some(token)
    }

    fn eat(&mut self, token: &Token) -> bool {
        if self.at(token) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        if matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &Token) -> Result<(), RdfDiagnostic> {
        if self.eat(token) {
            Ok(())
        } else {
            Err(err(format!("expected {token:?}, found {:?}", self.peek())))
        }
    }
}

fn numeric(lexical: String, datatype: &str) -> Node {
    Node::Literal {
        value: lexical,
        lang: None,
        direction: None,
        datatype: Some(datatype.to_owned()),
    }
}

/// A fresh blank-node label, delegating to the first-party
/// [`deterministic_blank_label`](super::ser_model::deterministic_blank_label): the
/// `gts_` prefix plus the Crockford Base32 ULID rendering of the zero-timestamp counter,
/// byte-identical to the prior purrdf-gts `deterministic_label("gts_", id)`.
fn deterministic_label(id: usize) -> String {
    super::ser_model::deterministic_blank_label(id)
}

// ───────────────────────────────────────────────────────────────────────────────
// Relative-IRI resolution (mirrors the prior from_trig `resolve_relative_iri`)
// ───────────────────────────────────────────────────────────────────────────────

fn remove_dot_segments(path: &str) -> String {
    let absolute = path.starts_with('/');
    let keep_trailing_slash = path.ends_with('/')
        || path.ends_with("/.")
        || path.ends_with("/..")
        || path == "."
        || path == "..";
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            segment => segments.push(segment),
        }
    }
    let mut normalized = String::new();
    if absolute {
        normalized.push('/');
    }
    normalized.push_str(&segments.join("/"));
    if keep_trailing_slash && !normalized.ends_with('/') {
        normalized.push('/');
    }
    if normalized.is_empty() && absolute {
        normalized.push('/');
    }
    normalized
}

fn split_raw_path_suffix(raw: &str) -> (&str, &str) {
    let split = raw.find(['?', '#']).unwrap_or(raw.len());
    (&raw[..split], &raw[split..])
}

fn split_base_for_path(base: &str) -> (String, &str) {
    let Some(scheme_end) = base.find(':') else {
        return (String::new(), base);
    };
    let scheme_prefix = &base[..=scheme_end];
    let rest = &base[scheme_end + 1..];
    if let Some(after_slashes) = rest.strip_prefix("//") {
        let authority_end = after_slashes.find('/').unwrap_or(after_slashes.len());
        let authority = &after_slashes[..authority_end];
        let path = &after_slashes[authority_end..];
        (format!("{scheme_prefix}//{authority}"), path)
    } else {
        (scheme_prefix.to_string(), rest)
    }
}

fn resolve_relative_iri(base: &str, raw: &str) -> String {
    if has_iri_scheme(raw) {
        return raw.to_string();
    }
    let base_without_fragment = base.split_once('#').map_or(base, |(before, _)| before);
    if raw.is_empty() {
        return base_without_fragment.to_string();
    }
    if raw.starts_with('#') {
        return format!("{base_without_fragment}{raw}");
    }
    let base_without_query = base_without_fragment
        .split_once('?')
        .map_or(base_without_fragment, |(before, _)| before);
    if raw.starts_with('?') {
        return format!("{base_without_query}{raw}");
    }
    if raw.starts_with("//") {
        if let Some(scheme_end) = base.find(':') {
            return format!("{}:{raw}", &base[..scheme_end]);
        }
        return raw.to_string();
    }
    let (prefix, base_path) = split_base_for_path(base_without_query);
    let (raw_path, suffix) = split_raw_path_suffix(raw);
    let merged_path = if raw_path.starts_with('/') {
        raw_path.to_string()
    } else {
        let base_dir = if base_path.is_empty() {
            "/"
        } else {
            base_path
                .rfind('/')
                .map(|index| &base_path[..=index])
                .unwrap_or("")
        };
        format!("{base_dir}{raw_path}")
    };
    format!("{prefix}{}{}", remove_dot_segments(&merged_path), suffix)
}

// ───────────────────────────────────────────────────────────────────────────────
// build_gts: lower the flat statement list to an in-memory SerGraph
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TermKey {
    Atom {
        kind: AtomKind,
        value: String,
        lang: Option<String>,
        direction: Option<String>,
        datatype: Option<String>,
    },
    Triple(usize, usize, usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AtomKind {
    Iri,
    Bnode,
    Literal,
}

/// The first-seen-order term interner, reproducing `from_nquads`'s `Interner`
/// so `dataset_from_ser_graph` re-interns its builder in the identical order.
struct Interner {
    ids: HashMap<TermKey, usize>,
    terms: Vec<SerTerm>,
}

impl Interner {
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            terms: Vec::new(),
        }
    }

    fn atom(&mut self, node: &Node) -> usize {
        let (kind, value, lang, direction, datatype) = match node {
            Node::Iri(value) => (AtomKind::Iri, value.clone(), None, None, None),
            Node::Bnode(value) => (AtomKind::Bnode, value.clone(), None, None, None),
            Node::Literal {
                value,
                lang,
                direction,
                datatype,
            } => (
                AtomKind::Literal,
                value.clone(),
                lang.clone(),
                direction.clone(),
                datatype.clone(),
            ),
            Node::Triple(..) => unreachable!("atom() is never called on a triple node"),
        };
        let key = TermKey::Atom {
            kind,
            value: value.clone(),
            lang: lang.clone(),
            direction: direction.clone(),
            datatype: datatype.clone(),
        };
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        // A literal's datatype IRI is interned as its own IRI term (first-seen), just
        // as purrdf-gts does, so the term table matches.
        let datatype_id = if kind == AtomKind::Literal {
            datatype
                .as_ref()
                .map(|iri| self.atom(&Node::Iri(iri.clone())))
        } else {
            None
        };
        let ser_kind = match kind {
            AtomKind::Iri => SerTermKind::Iri,
            AtomKind::Bnode => SerTermKind::Bnode,
            AtomKind::Literal => SerTermKind::Literal,
        };
        let id = self.terms.len();
        self.terms.push(SerTerm {
            kind: ser_kind,
            value: Some(value),
            datatype: datatype_id,
            lang,
            direction,
            reifier: None,
        });
        self.ids.insert(key, id);
        id
    }

    fn node(&mut self, node: &Node, reifiers: &mut Vec<(usize, SerTriple3)>) -> usize {
        match node {
            Node::Triple(s, p, o) => {
                let s = self.node(s, reifiers);
                let p = self.node(p, reifiers);
                let o = self.node(o, reifiers);
                let key = TermKey::Triple(s, p, o);
                if let Some(id) = self.ids.get(&key) {
                    return *id;
                }
                let id = self.terms.len();
                // A triple TERM is self-reifying: its reifier is its own id, matching
                // the purrdf-gts shape so `dataset_from_ser_graph` recognizes the
                // self-reifier sentinel (an inline quoted-triple object, NOT a statement
                // reifier).
                self.terms.push(SerTerm {
                    kind: SerTermKind::Triple,
                    value: None,
                    datatype: None,
                    lang: None,
                    direction: None,
                    reifier: Some(id),
                });
                self.ids.insert(key, id);
                reifiers.push((id, (s, p, o)));
                id
            }
            _ => self.atom(node),
        }
    }
}

/// Lower the flat statement list into the in-memory [`SerGraph`], reproducing
/// `from_nquads`'s `build_gts` (the `rdf:reifies` statement-layer shorthand,
/// first-seen interning, statement-order quads, encounter-order reifiers).
fn build_gts_graph(statements: &[Statement]) -> Result<SerGraph, RdfDiagnostic> {
    let mut interner = Interner::new();
    let mut reifiers: Vec<(usize, SerTriple3)> = Vec::new();
    let mut quads: Vec<(usize, usize, usize, Option<usize>)> = Vec::new();

    for nodes in statements {
        let s = &nodes[0];
        let p = &nodes[1];
        let o = &nodes[2];
        let gname = nodes.get(3);

        // `<subject> rdf:reifies <<( s p o )>> .` in the DEFAULT graph is the
        // statement-layer reifier shorthand: bind the reifier, do NOT emit a base quad.
        if let (
            Node::Iri(_) | Node::Bnode(_),
            Node::Iri(pred_iri),
            Node::Triple(ts, tp, to),
            None,
        ) = (s, p, o, gname)
        {
            if pred_iri == RDF_REIFIES {
                let rid = interner.atom(s);
                let ss = interner.node(ts, &mut reifiers);
                let pp = interner.node(tp, &mut reifiers);
                let oo = interner.node(to, &mut reifiers);
                set_reifier(&mut reifiers, rid, (ss, pp, oo))?;
                continue;
            }
        }

        let sid = interner.node(s, &mut reifiers);
        let pid = interner.node(p, &mut reifiers);
        let oid = interner.node(o, &mut reifiers);
        let gid = gname.map(|node| interner.node(node, &mut reifiers));
        quads.push((sid, pid, oid, gid));
    }

    Ok(SerGraph {
        terms: interner.terms,
        quads,
        // The reifier row carries an optional graph slot; this first-party text parser
        // binds reifiers only in the DEFAULT graph (the `rdf:reifies` shorthand is gated
        // on `None` graph above), so the slot is always `None`. Annotations are left in
        // `quads` here and reclassified by `fold_statement_layer`'s pass 2 (the
        // `annotations` table stays empty).
        reifiers: reifiers
            .into_iter()
            .map(|(rid, spo)| (rid, spo, None))
            .collect(),
        ..Default::default()
    })
}

/// Bind a reifier, hard-failing on a conflicting rebinding (CONSTITUTION P7: never
/// silently last-write-win), idempotent on an identical rebind. Mirrors
/// `from_nquads`'s `set_reifier`.
fn set_reifier(
    reifiers: &mut Vec<(usize, SerTriple3)>,
    rid: usize,
    spo: SerTriple3,
) -> Result<(), RdfDiagnostic> {
    if let Some((_, existing)) = reifiers.iter().find(|(r, _)| *r == rid) {
        if *existing != spo {
            return Err(err(format!(
                "conflicting rdf:reifies binding for reifier term {rid}"
            )));
        }
    } else {
        reifiers.push((rid, spo));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bare `/` in a prefixed-name local part (e.g. `purrdf:report/shacl/sarif`)
    /// must parse as ONE prefixed name and expand to the prefix namespace plus the
    /// slash-bearing local, matching oxigraph/purrdf-gts (strict Turtle would need
    /// `\/`, but the ontology + fixtures use the bare form).
    #[test]
    fn turtle_prefixed_name_allows_bare_slash_in_local() {
        let text = "@prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .\n\
                    purrdf:report/shacl/sarif purrdf:projection/okf purrdf:report/shacl/sarif .";
        let statements = DocParser::new(text, None, false).parse().expect("parses");
        assert_eq!(statements.len(), 1);
        let nodes = &statements[0];
        assert_eq!(
            nodes[0],
            Node::Iri("https://blackcatinformatics.ca/purrdf/report/shacl/sarif".to_owned())
        );
        assert_eq!(
            nodes[1],
            Node::Iri("https://blackcatinformatics.ca/purrdf/projection/okf".to_owned())
        );
        assert_eq!(
            nodes[2],
            Node::Iri("https://blackcatinformatics.ca/purrdf/report/shacl/sarif".to_owned())
        );
    }
}
