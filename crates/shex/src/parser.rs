// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The recursive-descent ShExC parser (ShEx 2.1 spec §6 grammar).
//!
//! Parses a full ShExC document into the ShExJ-aligned [`Schema`] AST:
//! `PREFIX`/`BASE`/`IMPORT` directives (interleavable with statements),
//! `start =`, labeled shape declarations, `EXTERNAL`, the
//! `OR`/`AND`/`NOT` shape-expression algebra, node constraints with the
//! full facet table, value sets with stems/ranges/exclusions, triple
//! expressions with `;`/`|` grouping, `$label`/`&label`
//! definitions/inclusions, `^` inverse, every cardinality form, `// pred
//! value` annotations and `%name{ … %}` semantic actions.
//!
//! Hard-fail discipline: every malformed document is a typed
//! [`ShexError`]; there is no lenient mode. The parser is fuzz-safe — no
//! panics on any input, and nesting depth is bounded so hostile input
//! cannot overflow the stack.
//!
//! Relative IRI references are resolved against the `BASE` in force (seeded
//! from the caller's base, delegated to `purrdf-iri`); with no base in force,
//! relative IRIs are preserved verbatim (matching the reference ShExJ
//! conversions in the conformance suite).

use std::collections::HashMap;

use crate::ast::{
    Annotation, NodeConstraint, NodeKind, NumericLiteral, ObjectLiteral, ObjectValue, Schema,
    SemAct, Shape, ShapeDecl, ShapeExpr, TripleConstraint, TripleExpr, TripleExprGroup,
    ValueSetValue,
};
use crate::ast::{IriExclusion, LanguageExclusion, LiteralExclusion, StemValue};
use crate::error::{Result, ShexError};
use crate::lexer::{CodeName, Spanned, Token, tokenize};

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// Deepest allowed expression nesting; beyond this the parser hard-fails
/// rather than risking stack exhaustion on hostile input. Each syntactic
/// nesting level costs several recursive-descent frames (and two depth
/// ticks), so this bound keeps worst-case stack use well under the default
/// 2 MiB test-thread stack even in unoptimized builds, while allowing far
/// deeper nesting than any real schema (the conformance corpus peaks at ~6).
const MAX_DEPTH: usize = 96;

/// Parse a ShExC document. `base` seeds relative-IRI resolution (a `BASE`
/// directive inside the document overrides it from that point on).
///
/// # Examples
///
/// ```
/// use purrdf_shex::parse_shexc;
///
/// let schema = parse_shexc(
///     "PREFIX ex: <http://example.org/>\n\
///      ex:UserShape { ex:name LITERAL }",
///     None,
/// )
/// .expect("a well-formed schema parses");
/// assert_eq!(schema.shapes.len(), 1);
/// assert_eq!(schema.shapes[0].id, "http://example.org/UserShape");
///
/// // Malformed input is a typed error, never a partial schema.
/// assert!(parse_shexc("ex:Broken {", None).is_err());
/// ```
pub fn parse_shexc(input: &str, base: Option<&str>) -> Result<Schema> {
    let tokens = tokenize(input)?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        prefixes: HashMap::new(),
        base: base.map(str::to_owned),
        depth: 0,
    };
    parser.parse_schema()
}

struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
    prefixes: HashMap<String, String>,
    base: Option<String>,
    depth: usize,
}

/// A parsed shape atom plus whether an enclosing `AND` chain may splice its
/// parts: the atom-level constraint+shape conjunction (`IRI @<S> AND …`)
/// flattens into the chain, while a parenthesized `ShapeAnd` stays nested —
/// matching the reference ShExJ conversion.
struct Conjunct {
    expr: ShapeExpr,
    splice: bool,
}

impl Conjunct {
    const fn single(expr: ShapeExpr) -> Self {
        Self {
            expr,
            splice: false,
        }
    }

    const fn spliceable(expr: ShapeExpr) -> Self {
        Self { expr, splice: true }
    }

    fn into_parts(self) -> Vec<ShapeExpr> {
        match self.expr {
            ShapeExpr::And(parts) if self.splice => parts,
            other => vec![other],
        }
    }
}

impl Parser {
    // ── token cursor ────────────────────────────────────────────────────────

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    fn peek2(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1).map(|s| &s.token)
    }

    fn span(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or_else(|| self.tokens.last().map_or(0, |s| s.end), |s| s.start)
    }

    fn eat(&mut self, t: &Token) -> bool {
        if self.peek() == Some(t) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: &Token, what: &str) -> Result<()> {
        if self.eat(t) {
            Ok(())
        } else {
            Err(self.err(format!("expected {what}, found {:?}", self.peek())))
        }
    }

    fn err(&self, reason: impl Into<String>) -> ShexError {
        ShexError::syntax(reason, self.span())
    }

    /// Is the current token the keyword `kw` (case-insensitive [`Token::Word`])?
    fn peek_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw))
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.peek_kw(kw) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn enter(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.err("expression nesting too deep"));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    // ── IRI plumbing ────────────────────────────────────────────────────────

    /// Resolve an IRIREF against the base in force; with no base, keep the
    /// reference verbatim (the conformance suite's convention).
    fn resolve(&self, reference: &str) -> Result<String> {
        let Some(base) = &self.base else {
            return Ok(reference.to_owned());
        };
        let base = purrdf_iri::parse(base).map_err(|e| ShexError::Iri {
            lexical: base.clone(),
            reason: e.to_string(),
        })?;
        let resolved = base.resolve(reference).map_err(|e| ShexError::Iri {
            lexical: reference.to_owned(),
            reason: e.to_string(),
        })?;
        Ok(resolved.as_str().to_owned())
    }

    fn expand(&self, prefix: &str, local: &str) -> Result<String> {
        self.prefixes.get(prefix).map_or_else(
            || Err(self.err(format!("undeclared prefix {prefix:?}"))),
            |ns| Ok(format!("{ns}{local}")),
        )
    }

    /// An `iri` production: IRIREF or prefixed name (never `a`).
    fn parse_iri(&mut self, what: &str) -> Result<String> {
        match self.peek().cloned() {
            Some(Token::Iri(iri)) => {
                self.pos += 1;
                self.resolve(&iri)
            }
            Some(Token::PName(p, l)) => {
                self.pos += 1;
                self.expand(&p, &l)
            }
            other => Err(self.err(format!("expected {what}, found {other:?}"))),
        }
    }

    /// A `predicate` production: `iri` or the `a` keyword (case-sensitive).
    fn parse_predicate(&mut self) -> Result<String> {
        if matches!(self.peek(), Some(Token::Word(w)) if w == "a") {
            self.pos += 1;
            return Ok(RDF_TYPE.to_owned());
        }
        self.parse_iri("predicate")
    }

    fn at_predicate(&self) -> bool {
        matches!(self.peek(), Some(Token::Iri(_) | Token::PName(..)))
            || matches!(self.peek(), Some(Token::Word(w)) if w == "a")
    }

    /// A `shapeExprLabel` / `tripleExprLabel`: `iri` or blank node.
    fn parse_label(&mut self) -> Result<String> {
        if let Some(Token::BNode(b)) = self.peek().cloned() {
            self.pos += 1;
            return Ok(format!("_:{b}"));
        }
        self.parse_iri("shape label")
    }

    // ── document structure ──────────────────────────────────────────────────

    fn parse_schema(&mut self) -> Result<Schema> {
        let mut schema = Schema::default();
        loop {
            match self.peek().cloned() {
                None => break,
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("prefix") => {
                    self.pos += 1;
                    let Some(Token::PName(p, l)) = self.peek().cloned() else {
                        return Err(self.err("expected prefix name after PREFIX"));
                    };
                    if !l.is_empty() {
                        return Err(self.err("PREFIX name must end with ':'"));
                    }
                    self.pos += 1;
                    let Some(Token::Iri(ns)) = self.peek().cloned() else {
                        return Err(self.err("expected IRI after PREFIX name"));
                    };
                    self.pos += 1;
                    let ns = self.resolve(&ns)?;
                    self.prefixes.insert(p, ns);
                }
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("base") => {
                    self.pos += 1;
                    let Some(Token::Iri(iri)) = self.peek().cloned() else {
                        return Err(self.err("expected IRI after BASE"));
                    };
                    self.pos += 1;
                    let resolved = self.resolve(&iri)?;
                    if purrdf_iri::parse(&resolved).is_err() {
                        return Err(ShexError::Iri {
                            lexical: resolved,
                            reason: "BASE must be a valid IRI".to_owned(),
                        });
                    }
                    self.base = Some(resolved);
                }
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("import") => {
                    self.pos += 1;
                    let Some(Token::Iri(iri)) = self.peek().cloned() else {
                        return Err(self.err("expected IRI after IMPORT"));
                    };
                    self.pos += 1;
                    let resolved = self.resolve(&iri)?;
                    schema.imports.push(resolved);
                }
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("start") => {
                    self.pos += 1;
                    self.expect(&Token::Eq, "'=' after start")?;
                    let expr = self.parse_shape_expression(true)?;
                    schema.start = Some(Box::new(expr));
                }
                Some(Token::Code { name, code }) => {
                    self.pos += 1;
                    let name = self.expand_code_name(&name)?;
                    schema.start_acts.push(SemAct { name, code });
                }
                Some(Token::Iri(_) | Token::PName(..) | Token::BNode(_)) => {
                    let decl = self.parse_shape_decl()?;
                    schema.shapes.push(decl);
                }
                Some(other) => {
                    return Err(self.err(format!(
                        "expected directive or shape declaration, found {other:?}"
                    )));
                }
            }
        }
        Ok(schema)
    }

    fn expand_code_name(&self, name: &CodeName) -> Result<String> {
        match name {
            CodeName::Iri(iri) => self.resolve(iri),
            CodeName::PName(p, l) => self.expand(p, l),
        }
    }

    fn parse_shape_decl(&mut self) -> Result<ShapeDecl> {
        let id = self.parse_label()?;
        let expr = if self.eat_kw("external") {
            ShapeExpr::External
        } else {
            self.parse_shape_expression(false)?
        };
        Ok(ShapeDecl { id, expr })
    }

    // ── shape expressions ───────────────────────────────────────────────────

    /// `shapeExpression` / `inlineShapeExpression` (`OR` level).
    fn parse_shape_expression(&mut self, inline: bool) -> Result<ShapeExpr> {
        self.enter()?;
        let result = self.parse_shape_or(inline);
        self.leave();
        result
    }

    fn parse_shape_or(&mut self, inline: bool) -> Result<ShapeExpr> {
        let first = self.parse_shape_and(inline)?;
        if !self.peek_kw("or") {
            return Ok(first);
        }
        let mut parts = vec![first];
        while self.eat_kw("or") {
            parts.push(self.parse_shape_and(inline)?);
        }
        Ok(ShapeExpr::Or(parts))
    }

    fn parse_shape_and(&mut self, inline: bool) -> Result<ShapeExpr> {
        let first = self.parse_shape_not(inline)?;
        if !self.peek_kw("and") {
            return Ok(first.expr);
        }
        // An `AND` chain is one flat `ShapeAnd`; an atom-level
        // constraint+shape conjunction (`IRI @<S>`) flattens INTO the chain,
        // while a parenthesized `ShapeAnd` stays nested — both matching the
        // reference ShExJ conversion.
        let mut parts = first.into_parts();
        while self.eat_kw("and") {
            parts.extend(self.parse_shape_not(inline)?.into_parts());
        }
        Ok(ShapeExpr::And(parts))
    }

    fn parse_shape_not(&mut self, inline: bool) -> Result<Conjunct> {
        if self.eat_kw("not") {
            let atom = self.parse_shape_atom(inline)?;
            Ok(Conjunct::single(ShapeExpr::Not(Box::new(atom.expr))))
        } else {
            self.parse_shape_atom(inline)
        }
    }

    /// `shapeAtom` / `inlineShapeAtom`.
    fn parse_shape_atom(&mut self, inline: bool) -> Result<Conjunct> {
        self.enter()?;
        let result = self.parse_shape_atom_inner(inline);
        self.leave();
        result
    }

    fn parse_shape_atom_inner(&mut self, inline: bool) -> Result<Conjunct> {
        match self.peek().cloned() {
            // nonLitNodeConstraint: node kind + string facets, then an
            // optional shape-or-ref conjunct.
            Some(Token::Word(w))
                if ["iri", "bnode", "nonliteral"]
                    .iter()
                    .any(|k| w.eq_ignore_ascii_case(k)) =>
            {
                self.pos += 1;
                let kind = match w.to_ascii_lowercase().as_str() {
                    "iri" => NodeKind::Iri,
                    "bnode" => NodeKind::BNode,
                    _ => NodeKind::NonLiteral,
                };
                let mut nc = NodeConstraint {
                    node_kind: Some(kind),
                    ..NodeConstraint::default()
                };
                self.parse_facets(&mut nc, false)?;
                let node = ShapeExpr::Node(nc);
                self.maybe_and_shape_or_ref(node, inline)
            }
            // stringFacet+ then an optional shape-or-ref conjunct.
            Some(Token::Word(w))
                if ["length", "minlength", "maxlength"]
                    .iter()
                    .any(|k| w.eq_ignore_ascii_case(k)) =>
            {
                let mut nc = NodeConstraint::default();
                self.parse_facets(&mut nc, false)?;
                let node = ShapeExpr::Node(nc);
                self.maybe_and_shape_or_ref(node, inline)
            }
            Some(Token::Regex { .. }) => {
                let mut nc = NodeConstraint::default();
                self.parse_facets(&mut nc, false)?;
                let node = ShapeExpr::Node(nc);
                self.maybe_and_shape_or_ref(node, inline)
            }
            // litNodeConstraint: LITERAL + any facets.
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("literal") => {
                self.pos += 1;
                let mut nc = NodeConstraint {
                    node_kind: Some(NodeKind::Literal),
                    ..NodeConstraint::default()
                };
                self.parse_facets(&mut nc, true)?;
                Ok(Conjunct::single(ShapeExpr::Node(nc)))
            }
            // litNodeConstraint: numericFacet+.
            Some(Token::Word(w)) if is_numeric_facet_kw(&w) => {
                let mut nc = NodeConstraint::default();
                self.parse_facets(&mut nc, true)?;
                if nc == NodeConstraint::default() {
                    return Err(self.err(format!("unexpected keyword {w}")));
                }
                Ok(Conjunct::single(ShapeExpr::Node(nc)))
            }
            // litNodeConstraint: datatype + facets.
            Some(Token::Iri(_) | Token::PName(..)) => {
                let datatype = self.parse_iri("datatype IRI")?;
                let mut nc = NodeConstraint {
                    datatype: Some(datatype),
                    ..NodeConstraint::default()
                };
                self.parse_facets(&mut nc, true)?;
                self.check_numeric_facet_datatype(&nc)?;
                Ok(Conjunct::single(ShapeExpr::Node(nc)))
            }
            // litNodeConstraint: value set + facets.
            Some(Token::LBracket) => {
                let values = self.parse_value_set()?;
                let mut nc = NodeConstraint {
                    values: Some(values),
                    ..NodeConstraint::default()
                };
                self.parse_facets(&mut nc, true)?;
                Ok(Conjunct::single(ShapeExpr::Node(nc)))
            }
            // shapeOrRef, then an optional nonLitNodeConstraint conjunct.
            Some(Token::At) => {
                self.pos += 1;
                let label = self.parse_label()?;
                self.maybe_and_non_lit(ShapeExpr::Ref(label), inline)
            }
            Some(Token::LBrace) => {
                let shape = self.parse_shape_definition(inline)?;
                self.maybe_and_non_lit(ShapeExpr::Shape(shape), inline)
            }
            Some(Token::Word(w))
                if w.eq_ignore_ascii_case("closed") || w.eq_ignore_ascii_case("extra") =>
            {
                let shape = self.parse_shape_definition(inline)?;
                self.maybe_and_non_lit(ShapeExpr::Shape(shape), inline)
            }
            Some(Token::LParen) => {
                self.pos += 1;
                let expr = self.parse_shape_expression(false)?;
                self.expect(&Token::RParen, "')'")?;
                Ok(Conjunct::single(expr))
            }
            // `.` — the "anything" atom: an empty Shape in ShExJ terms.
            Some(Token::Dot) => {
                self.pos += 1;
                Ok(Conjunct::single(ShapeExpr::Shape(Shape::default())))
            }
            other => Err(self.err(format!("expected shape expression, found {other:?}"))),
        }
    }

    /// After a nonLit node constraint: an optional `shapeOrRef` conjunct.
    fn maybe_and_shape_or_ref(&mut self, node: ShapeExpr, inline: bool) -> Result<Conjunct> {
        let extra = match self.peek() {
            Some(Token::At) => {
                self.pos += 1;
                let label = self.parse_label()?;
                Some(ShapeExpr::Ref(label))
            }
            Some(Token::LBrace) => Some(ShapeExpr::Shape(self.parse_shape_definition(inline)?)),
            Some(Token::Word(w))
                if w.eq_ignore_ascii_case("closed") || w.eq_ignore_ascii_case("extra") =>
            {
                Some(ShapeExpr::Shape(self.parse_shape_definition(inline)?))
            }
            _ => None,
        };
        Ok(match extra {
            Some(rhs) => Conjunct::spliceable(ShapeExpr::And(vec![node, rhs])),
            None => Conjunct::single(node),
        })
    }

    /// After a `shapeOrRef`: an optional nonLit node-constraint conjunct.
    fn maybe_and_non_lit(&mut self, lhs: ShapeExpr, _inline: bool) -> Result<Conjunct> {
        let nc = match self.peek().cloned() {
            Some(Token::Word(w))
                if ["iri", "bnode", "nonliteral"]
                    .iter()
                    .any(|k| w.eq_ignore_ascii_case(k)) =>
            {
                self.pos += 1;
                let kind = match w.to_ascii_lowercase().as_str() {
                    "iri" => NodeKind::Iri,
                    "bnode" => NodeKind::BNode,
                    _ => NodeKind::NonLiteral,
                };
                let mut nc = NodeConstraint {
                    node_kind: Some(kind),
                    ..NodeConstraint::default()
                };
                self.parse_facets(&mut nc, false)?;
                Some(nc)
            }
            Some(Token::Word(w))
                if ["length", "minlength", "maxlength"]
                    .iter()
                    .any(|k| w.eq_ignore_ascii_case(k)) =>
            {
                let mut nc = NodeConstraint::default();
                self.parse_facets(&mut nc, false)?;
                Some(nc)
            }
            Some(Token::Regex { .. }) => {
                let mut nc = NodeConstraint::default();
                self.parse_facets(&mut nc, false)?;
                Some(nc)
            }
            _ => None,
        };
        Ok(match nc {
            Some(nc) => Conjunct::spliceable(ShapeExpr::And(vec![lhs, ShapeExpr::Node(nc)])),
            None => Conjunct::single(lhs),
        })
    }

    // ── facets ──────────────────────────────────────────────────────────────

    /// `xsFacet*` — string facets always; numeric facets only when
    /// `allow_numeric`. Duplicate facets are a syntax error (spec §6).
    fn parse_facets(&mut self, nc: &mut NodeConstraint, allow_numeric: bool) -> Result<()> {
        loop {
            match self.peek().cloned() {
                Some(Token::Regex { pattern, flags }) => {
                    if nc.pattern.is_some() {
                        return Err(self.err("duplicate pattern facet"));
                    }
                    self.pos += 1;
                    nc.pattern = Some(pattern);
                    if !flags.is_empty() {
                        nc.flags = Some(flags);
                    }
                }
                Some(Token::Word(w)) if is_string_length_kw(&w) => {
                    self.pos += 1;
                    let n = self.parse_unsigned("string facet value")?;
                    let slot = match w.to_ascii_lowercase().as_str() {
                        "length" => &mut nc.length,
                        "minlength" => &mut nc.minlength,
                        _ => &mut nc.maxlength,
                    };
                    if slot.is_some() {
                        return Err(self.err(format!("duplicate {} facet", w.to_lowercase())));
                    }
                    *slot = Some(n);
                }
                Some(Token::Word(w)) if allow_numeric && is_numeric_range_kw(&w) => {
                    self.pos += 1;
                    let value = self.parse_numeric_literal()?;
                    let slot = match w.to_ascii_lowercase().as_str() {
                        "mininclusive" => &mut nc.mininclusive,
                        "minexclusive" => &mut nc.minexclusive,
                        "maxinclusive" => &mut nc.maxinclusive,
                        _ => &mut nc.maxexclusive,
                    };
                    if slot.is_some() {
                        return Err(self.err(format!("duplicate {} facet", w.to_lowercase())));
                    }
                    *slot = Some(value);
                }
                Some(Token::Word(w)) if allow_numeric && is_numeric_length_kw(&w) => {
                    self.pos += 1;
                    let n = self.parse_unsigned("digit-count facet value")?;
                    let slot = if w.eq_ignore_ascii_case("totaldigits") {
                        &mut nc.totaldigits
                    } else {
                        &mut nc.fractiondigits
                    };
                    if slot.is_some() {
                        return Err(self.err(format!("duplicate {} facet", w.to_lowercase())));
                    }
                    *slot = Some(n);
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn parse_unsigned(&mut self, what: &str) -> Result<u64> {
        let Some(Token::Integer(lexical)) = self.peek().cloned() else {
            return Err(self.err(format!("expected non-negative INTEGER as {what}")));
        };
        let n = lexical
            .parse::<u64>()
            .map_err(|_| self.err(format!("{what} must be a non-negative integer")))?;
        self.pos += 1;
        Ok(n)
    }

    fn parse_numeric_literal(&mut self) -> Result<NumericLiteral> {
        let lexical = match self.peek().cloned() {
            Some(Token::Integer(s) | Token::Decimal(s) | Token::Double(s)) => s,
            other => {
                return Err(self.err(format!(
                    "expected numeric literal as facet value, found {other:?}"
                )));
            }
        };
        let value = numeric_from_lexical(&lexical)
            .ok_or_else(|| self.err(format!("numeric facet value {lexical} out of range")))?;
        self.pos += 1;
        Ok(value)
    }

    /// Numeric facets demand a numeric datatype when one is given (the
    /// `1unknowndatatypeMaxInclusive` conformance rule).
    fn check_numeric_facet_datatype(&self, nc: &NodeConstraint) -> Result<()> {
        let has_numeric_facet = nc.mininclusive.is_some()
            || nc.minexclusive.is_some()
            || nc.maxinclusive.is_some()
            || nc.maxexclusive.is_some()
            || nc.totaldigits.is_some()
            || nc.fractiondigits.is_some();
        if !has_numeric_facet {
            return Ok(());
        }
        if let Some(dt) = &nc.datatype
            && !is_numeric_datatype(dt)
        {
            return Err(self.err(format!("numeric facet on non-numeric datatype <{dt}>")));
        }
        Ok(())
    }

    // ── value sets ──────────────────────────────────────────────────────────

    fn parse_value_set(&mut self) -> Result<Vec<ValueSetValue>> {
        self.expect(&Token::LBracket, "'['")?;
        let mut values = Vec::new();
        while !self.eat(&Token::RBracket) {
            values.push(self.parse_value_set_value()?);
        }
        Ok(values)
    }

    fn parse_value_set_value(&mut self) -> Result<ValueSetValue> {
        match self.peek().cloned() {
            Some(Token::Iri(_) | Token::PName(..)) => {
                let iri = self.parse_iri("value-set IRI")?;
                if self.eat(&Token::Tilde) {
                    let exclusions = self.parse_iri_exclusions()?;
                    if exclusions.is_empty() {
                        Ok(ValueSetValue::IriStem { stem: iri })
                    } else {
                        Ok(ValueSetValue::IriStemRange {
                            stem: StemValue::Str(iri),
                            exclusions,
                        })
                    }
                } else {
                    Ok(ValueSetValue::Iri(iri))
                }
            }
            Some(
                Token::StringLit(_) | Token::Integer(_) | Token::Decimal(_) | Token::Double(_),
            ) => {
                let literal = self.parse_literal()?;
                if self.eat(&Token::Tilde) {
                    let exclusions = self.parse_literal_exclusions()?;
                    if exclusions.is_empty() {
                        Ok(ValueSetValue::LiteralStem {
                            stem: literal.value,
                        })
                    } else {
                        Ok(ValueSetValue::LiteralStemRange {
                            stem: StemValue::Str(literal.value),
                            exclusions,
                        })
                    }
                } else {
                    Ok(ValueSetValue::Literal(literal))
                }
            }
            Some(Token::Word(w)) if w == "true" || w == "false" => {
                let literal = self.parse_literal()?;
                Ok(ValueSetValue::Literal(literal))
            }
            Some(Token::LangTag(tag)) => {
                self.pos += 1;
                if self.eat(&Token::Tilde) {
                    let exclusions = self.parse_language_exclusions()?;
                    if exclusions.is_empty() {
                        Ok(ValueSetValue::LanguageStem { stem: tag })
                    } else {
                        Ok(ValueSetValue::LanguageStemRange {
                            stem: StemValue::Str(tag),
                            exclusions,
                        })
                    }
                } else {
                    Ok(ValueSetValue::Language { language_tag: tag })
                }
            }
            // `@~` — the empty language stem.
            Some(Token::At) => {
                self.pos += 1;
                self.expect(&Token::Tilde, "'~' after '@' in a value set")?;
                let exclusions = self.parse_language_exclusions()?;
                if exclusions.is_empty() {
                    Ok(ValueSetValue::LanguageStem {
                        stem: String::new(),
                    })
                } else {
                    Ok(ValueSetValue::LanguageStemRange {
                        stem: StemValue::Str(String::new()),
                        exclusions,
                    })
                }
            }
            // `. - v …` — a wildcard stem with at least one exclusion; every
            // exclusion must be of the same kind.
            Some(Token::Dot) => {
                self.pos += 1;
                if self.peek() != Some(&Token::Minus) {
                    return Err(self.err("expected '-' exclusion after '.' in a value set"));
                }
                match self.peek2().cloned() {
                    Some(Token::Iri(_) | Token::PName(..)) => {
                        let exclusions = self.parse_iri_exclusions()?;
                        Ok(ValueSetValue::IriStemRange {
                            stem: StemValue::Wildcard,
                            exclusions,
                        })
                    }
                    Some(
                        Token::StringLit(_)
                        | Token::Integer(_)
                        | Token::Decimal(_)
                        | Token::Double(_),
                    ) => {
                        let exclusions = self.parse_literal_exclusions()?;
                        Ok(ValueSetValue::LiteralStemRange {
                            stem: StemValue::Wildcard,
                            exclusions,
                        })
                    }
                    Some(Token::LangTag(_)) => {
                        let exclusions = self.parse_language_exclusions()?;
                        Ok(ValueSetValue::LanguageStemRange {
                            stem: StemValue::Wildcard,
                            exclusions,
                        })
                    }
                    other => Err(self.err(format!(
                        "expected IRI, literal or language tag exclusion, found {other:?}"
                    ))),
                }
            }
            other => Err(self.err(format!("unexpected {other:?} in value set"))),
        }
    }

    fn parse_iri_exclusions(&mut self) -> Result<Vec<IriExclusion>> {
        let mut out = Vec::new();
        while self.eat(&Token::Minus) {
            let iri = self.parse_iri("IRI exclusion")?;
            if self.eat(&Token::Tilde) {
                out.push(IriExclusion::Stem(iri));
            } else {
                out.push(IriExclusion::Iri(iri));
            }
        }
        Ok(out)
    }

    fn parse_literal_exclusions(&mut self) -> Result<Vec<LiteralExclusion>> {
        let mut out = Vec::new();
        while self.eat(&Token::Minus) {
            let literal = self.parse_literal()?;
            if self.eat(&Token::Tilde) {
                out.push(LiteralExclusion::Stem(literal.value));
            } else {
                out.push(LiteralExclusion::Literal(literal.value));
            }
        }
        Ok(out)
    }

    fn parse_language_exclusions(&mut self) -> Result<Vec<LanguageExclusion>> {
        let mut out = Vec::new();
        while self.eat(&Token::Minus) {
            let Some(Token::LangTag(tag)) = self.peek().cloned() else {
                return Err(self.err("expected language tag exclusion"));
            };
            self.pos += 1;
            if self.eat(&Token::Tilde) {
                out.push(LanguageExclusion::Stem(tag));
            } else {
                out.push(LanguageExclusion::Language(tag));
            }
        }
        Ok(out)
    }

    /// An `rdfLiteral | numericLiteral | booleanLiteral`.
    fn parse_literal(&mut self) -> Result<ObjectLiteral> {
        match self.peek().cloned() {
            Some(Token::StringLit(value)) => {
                self.pos += 1;
                match self.peek().cloned() {
                    Some(Token::LangTag(tag)) => {
                        self.pos += 1;
                        if self.peek() == Some(&Token::HatHat) {
                            return Err(
                                self.err("literal cannot carry both language tag and datatype")
                            );
                        }
                        Ok(ObjectLiteral {
                            value,
                            // Language-tagged literals carry lowercase tags
                            // in the RDF data model (and the ShExJ ground
                            // truth).
                            language: Some(tag.to_ascii_lowercase()),
                            datatype: None,
                        })
                    }
                    Some(Token::HatHat) => {
                        self.pos += 1;
                        let dt = self.parse_iri("datatype IRI")?;
                        Ok(ObjectLiteral {
                            value,
                            language: None,
                            datatype: Some(dt),
                        })
                    }
                    _ => Ok(ObjectLiteral {
                        value,
                        language: None,
                        datatype: None,
                    }),
                }
            }
            Some(Token::Integer(s)) => {
                self.pos += 1;
                Ok(typed_literal(s, purrdf_xsd::datatype::XSD_INTEGER))
            }
            Some(Token::Decimal(s)) => {
                self.pos += 1;
                Ok(typed_literal(s, purrdf_xsd::datatype::XSD_DECIMAL))
            }
            Some(Token::Double(s)) => {
                self.pos += 1;
                Ok(typed_literal(s, purrdf_xsd::datatype::XSD_DOUBLE))
            }
            Some(Token::Word(w)) if w == "true" || w == "false" => {
                self.pos += 1;
                Ok(typed_literal(w, purrdf_xsd::datatype::XSD_BOOLEAN))
            }
            other => Err(self.err(format!("expected literal, found {other:?}"))),
        }
    }

    // ── shape definitions & triple expressions ──────────────────────────────

    /// `shapeDefinition` (non-inline gets trailing annotations + semActs).
    fn parse_shape_definition(&mut self, inline: bool) -> Result<Shape> {
        let mut shape = Shape::default();
        loop {
            if self.eat_kw("closed") {
                shape.closed = Some(true);
            } else if self.eat_kw("extra") {
                shape.extra.push(self.parse_predicate()?);
                while self.at_predicate() {
                    shape.extra.push(self.parse_predicate()?);
                }
            } else {
                break;
            }
        }
        self.expect(&Token::LBrace, "'{'")?;
        if !self.eat(&Token::RBrace) {
            let expr = self.parse_triple_expression()?;
            self.expect(&Token::RBrace, "'}'")?;
            shape.expression = Some(expr);
        }
        if !inline {
            shape.annotations = self.parse_annotations()?;
            shape.sem_acts = self.parse_sem_acts()?;
        }
        Ok(shape)
    }

    /// `tripleExpression ::= oneOfTripleExpr` (`|` level).
    fn parse_triple_expression(&mut self) -> Result<TripleExpr> {
        self.enter()?;
        let result = self.parse_one_of();
        self.leave();
        result
    }

    fn parse_one_of(&mut self) -> Result<TripleExpr> {
        let first = self.parse_each_of()?;
        if self.peek() != Some(&Token::Pipe) {
            return Ok(first);
        }
        let mut alternatives = vec![first];
        while self.eat(&Token::Pipe) {
            alternatives.push(self.parse_each_of()?);
        }
        Ok(TripleExpr::OneOf(TripleExprGroup {
            expressions: alternatives,
            ..TripleExprGroup::default()
        }))
    }

    fn parse_each_of(&mut self) -> Result<TripleExpr> {
        let first = self.parse_unary()?;
        if self.peek() != Some(&Token::Semi) {
            return Ok(first);
        }
        let mut members = vec![first];
        while self.eat(&Token::Semi) {
            if self.at_unary_start() {
                members.push(self.parse_unary()?);
            } else {
                break; // trailing `;`
            }
        }
        if members.len() == 1 {
            return Ok(members.remove(0));
        }
        Ok(TripleExpr::EachOf(TripleExprGroup {
            expressions: members,
            ..TripleExprGroup::default()
        }))
    }

    fn at_unary_start(&self) -> bool {
        matches!(
            self.peek(),
            Some(
                Token::Dollar
                    | Token::Amp
                    | Token::Caret
                    | Token::LParen
                    | Token::Iri(_)
                    | Token::PName(..)
            )
        ) || matches!(self.peek(), Some(Token::Word(w)) if w == "a")
    }

    /// `unaryTripleExpr ::= ('$' label)? (tripleConstraint | bracketedTripleExpr) | '&' label`
    fn parse_unary(&mut self) -> Result<TripleExpr> {
        self.enter()?;
        let result = self.parse_unary_inner();
        self.leave();
        result
    }

    fn parse_unary_inner(&mut self) -> Result<TripleExpr> {
        if self.eat(&Token::Amp) {
            let label = self.parse_label()?;
            return Ok(TripleExpr::Ref(label));
        }
        let id = if self.eat(&Token::Dollar) {
            Some(self.parse_label()?)
        } else {
            None
        };
        let mut expr = if self.peek() == Some(&Token::LParen) {
            self.parse_bracketed()?
        } else {
            self.parse_triple_constraint()?
        };
        if let Some(id) = id {
            match &mut expr {
                TripleExpr::EachOf(g) | TripleExpr::OneOf(g) => {
                    g.id = Some(id);
                }
                TripleExpr::TripleConstraint(tc) => {
                    tc.id = Some(id);
                }
                TripleExpr::Ref(_) => {
                    return Err(self.err("'$' label cannot name an inclusion"));
                }
            }
        }
        Ok(expr)
    }

    /// `'(' tripleExpression ')' cardinality? annotation* semanticActions`
    fn parse_bracketed(&mut self) -> Result<TripleExpr> {
        self.expect(&Token::LParen, "'('")?;
        let inner = self.parse_triple_expression()?;
        self.expect(&Token::RParen, "')'")?;
        let cardinality = self.parse_cardinality();
        let annotations = self.parse_annotations()?;
        let sem_acts = self.parse_sem_acts()?;
        if cardinality.is_none() && annotations.is_empty() && sem_acts.is_empty() {
            return Ok(inner);
        }
        Ok(apply_group_modifiers(
            inner,
            cardinality,
            annotations,
            sem_acts,
        ))
    }

    /// `tripleConstraint ::= '^'? predicate inlineShapeExpression cardinality? annotation* semanticActions`
    fn parse_triple_constraint(&mut self) -> Result<TripleExpr> {
        let inverse = self.eat(&Token::Caret);
        let predicate = self.parse_predicate()?;
        let value_expr = self.parse_inline_value_expr()?;
        let cardinality = self.parse_cardinality();
        let annotations = self.parse_annotations()?;
        let sem_acts = self.parse_sem_acts()?;
        let (min, max) = cardinality.map_or((None, None), |(a, b)| (Some(a), Some(b)));
        Ok(TripleExpr::TripleConstraint(TripleConstraint {
            id: None,
            inverse: inverse.then_some(true),
            predicate,
            value_expr: value_expr.map(Box::new),
            min,
            max,
            sem_acts,
            annotations,
        }))
    }

    /// The triple constraint's value expression; a bare `.` wildcard is the
    /// absent value expression in ShExJ terms.
    fn parse_inline_value_expr(&mut self) -> Result<Option<ShapeExpr>> {
        if self.peek() == Some(&Token::Dot) {
            let continues = matches!(
                self.peek2(),
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("and") || w.eq_ignore_ascii_case("or")
            );
            if !continues {
                self.pos += 1;
                return Ok(None);
            }
        }
        Ok(Some(self.parse_shape_expression(true)?))
    }

    fn parse_cardinality(&mut self) -> Option<(i64, i64)> {
        match self.peek().cloned() {
            Some(Token::Star) => {
                self.pos += 1;
                Some((0, -1))
            }
            Some(Token::Plus) => {
                self.pos += 1;
                Some((1, -1))
            }
            Some(Token::Question) => {
                self.pos += 1;
                Some((0, 1))
            }
            Some(Token::Repeat { min, max }) => {
                self.pos += 1;
                Some((min, max))
            }
            _ => None,
        }
    }

    fn parse_annotations(&mut self) -> Result<Vec<Annotation>> {
        let mut out = Vec::new();
        while self.eat(&Token::AnnotMarker) {
            let predicate = self.parse_predicate()?;
            let object = match self.peek() {
                Some(Token::Iri(_) | Token::PName(..)) => {
                    ObjectValue::Iri(self.parse_iri("annotation object")?)
                }
                _ => ObjectValue::Literal(self.parse_literal()?),
            };
            out.push(Annotation { predicate, object });
        }
        Ok(out)
    }

    fn parse_sem_acts(&mut self) -> Result<Vec<SemAct>> {
        let mut out = Vec::new();
        while let Some(Token::Code { name, code }) = self.peek().cloned() {
            self.pos += 1;
            let name = self.expand_code_name(&name)?;
            out.push(SemAct { name, code });
        }
        Ok(out)
    }
}

/// Attach a bracketed group's cardinality/annotations/semActs to `inner`,
/// wrapping in a singleton `EachOf` when `inner` already carries its own
/// modifiers (so `(x{2}){3}` keeps both cardinalities).
fn apply_group_modifiers(
    inner: TripleExpr,
    cardinality: Option<(i64, i64)>,
    annotations: Vec<Annotation>,
    sem_acts: Vec<SemAct>,
) -> TripleExpr {
    let (min, max) = cardinality.map_or((None, None), |(a, b)| (Some(a), Some(b)));
    let needs_wrap = match &inner {
        TripleExpr::EachOf(g) | TripleExpr::OneOf(g) => g.min.is_some() || g.max.is_some(),
        TripleExpr::TripleConstraint(tc) => tc.min.is_some() || tc.max.is_some(),
        TripleExpr::Ref(_) => true,
    };
    if needs_wrap {
        return TripleExpr::EachOf(TripleExprGroup {
            id: None,
            expressions: vec![inner],
            min,
            max,
            sem_acts,
            annotations,
        });
    }
    match inner {
        TripleExpr::EachOf(mut g) => {
            g.min = min;
            g.max = max;
            g.annotations.extend(annotations);
            g.sem_acts.extend(sem_acts);
            TripleExpr::EachOf(g)
        }
        TripleExpr::OneOf(mut g) => {
            g.min = min;
            g.max = max;
            g.annotations.extend(annotations);
            g.sem_acts.extend(sem_acts);
            TripleExpr::OneOf(g)
        }
        TripleExpr::TripleConstraint(mut tc) => {
            tc.min = min;
            tc.max = max;
            tc.annotations.extend(annotations);
            tc.sem_acts.extend(sem_acts);
            TripleExpr::TripleConstraint(tc)
        }
        TripleExpr::Ref(_) => unreachable!("Ref always wraps"),
    }
}

fn typed_literal(value: String, datatype: &str) -> ObjectLiteral {
    ObjectLiteral {
        value,
        language: None,
        datatype: Some(datatype.to_owned()),
    }
}

fn is_string_length_kw(w: &str) -> bool {
    ["length", "minlength", "maxlength"]
        .iter()
        .any(|k| w.eq_ignore_ascii_case(k))
}

fn is_numeric_range_kw(w: &str) -> bool {
    [
        "mininclusive",
        "minexclusive",
        "maxinclusive",
        "maxexclusive",
    ]
    .iter()
    .any(|k| w.eq_ignore_ascii_case(k))
}

fn is_numeric_length_kw(w: &str) -> bool {
    ["totaldigits", "fractiondigits"]
        .iter()
        .any(|k| w.eq_ignore_ascii_case(k))
}

fn is_numeric_facet_kw(w: &str) -> bool {
    is_numeric_range_kw(w) || is_numeric_length_kw(w)
}

/// Parse an INTEGER/DECIMAL/DOUBLE lexical form into the ShExJ numeric value
/// space: integral values normalize to `Integer` (matching the reference
/// implementation's JSON-number conversion), everything else is `Fractional`.
fn numeric_from_lexical(lexical: &str) -> Option<NumericLiteral> {
    if let Ok(i) = lexical.parse::<i64>() {
        return Some(NumericLiteral::Integer(i));
    }
    let f = lexical.parse::<f64>().ok()?;
    if !f.is_finite() {
        return None;
    }
    if f.fract() == 0.0 && (-9_007_199_254_740_992.0..=9_007_199_254_740_992.0).contains(&f) {
        return Some(NumericLiteral::Integer(f as i64));
    }
    Some(NumericLiteral::Fractional(f))
}

/// The XSD numeric datatypes (decimal/double/float and the integer-derived
/// family) admissible under a numeric facet.
fn is_numeric_datatype(dt: &str) -> bool {
    use purrdf_xsd::datatype as x;
    [
        x::XSD_INTEGER,
        x::XSD_DECIMAL,
        x::XSD_FLOAT,
        x::XSD_DOUBLE,
        x::XSD_LONG,
        x::XSD_INT,
        x::XSD_SHORT,
        x::XSD_BYTE,
        x::XSD_UNSIGNED_LONG,
        x::XSD_UNSIGNED_INT,
        x::XSD_UNSIGNED_SHORT,
        x::XSD_UNSIGNED_BYTE,
        x::XSD_NON_NEGATIVE_INTEGER,
        x::XSD_NON_POSITIVE_INTEGER,
        x::XSD_POSITIVE_INTEGER,
        x::XSD_NEGATIVE_INTEGER,
    ]
    .contains(&dt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn hostile_inputs_error_instead_of_panicking() {
        // Deep nesting is depth-bounded, not stack-fatal.
        let deep = format!("<http://x/S> {}.{}", "(".repeat(4096), ")".repeat(4096));
        assert!(parse_shexc(&deep, None).is_err());
        // Truncated documents at every prefix of a real schema.
        let doc =
            "PREFIX ex: <http://x/>\nex:S EXTRA ex:p { ^ex:p [ex:v~ - ex:w]{2,3} %ex:a{ c %} }";
        for end in 0..doc.len() {
            if doc.is_char_boundary(end) {
                let _ = parse_shexc(&doc[..end], None);
            }
        }
        // Assorted junk.
        for junk in ["\u{0}", "@", "%", "<", "((((", "[", "{", "start=", "&", "$"] {
            assert!(parse_shexc(junk, None).is_err(), "accepted junk {junk:?}");
        }
        // Empty input is the empty schema.
        assert_eq!(parse_shexc("", None), Ok(Schema::default()));
    }

    #[test]
    fn base_resolution_uses_purrdf_iri() {
        let schema =
            parse_shexc("<S1> { <p1> . }", Some("http://a.example/dir/doc")).expect("parse");
        assert_eq!(schema.shapes[0].id, "http://a.example/dir/S1");
        let err = parse_shexc("BASE <rel/only>\n<S1> { <p1> . }", None).unwrap_err();
        assert!(matches!(err, ShexError::Iri { .. }));
    }

    #[test]
    fn wildcard_value_expr_is_absent() {
        let schema = parse_shexc("<S> { <p> . }", None).expect("parse");
        let ShapeExpr::Shape(shape) = &schema.shapes[0].expr else {
            panic!("expected shape");
        };
        let Some(TripleExpr::TripleConstraint(tc)) = &shape.expression else {
            panic!("expected triple constraint");
        };
        assert_eq!(tc.value_expr, None);
        assert_eq!((tc.min, tc.max), (None, None));
    }
}
