// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Query shape maps (the ShapeMap spec, <https://shex.io/shape-map/>).
//!
//! A shape map associates nodes with shapes. Besides a fixed
//! `(node, shape)` list ([`crate::validate::validate`]), the spec allows a
//! **query form** whose node selector is a triple pattern with a `FOCUS`
//! position and the other positions concrete or a `_` wildcard:
//!
//! * `{FOCUS <p> _}` — every subject of an arc with predicate `<p>`;
//! * `{FOCUS a <C>}` — every subject typed `<C>` (`a` is `rdf:type`);
//! * `{_ <p> FOCUS}` — every object of an arc with predicate `<p>`;
//! * `{<s> <p> FOCUS}` — the objects of `<s> <p> ?`.
//!
//! [`parse_shape_map`] parses the compact syntax into a [`ShapeMap`];
//! [`resolve_shape_map`] expands the query selectors against a frozen
//! [`RdfDataset`] into the concrete `(node, shape)` pairs that
//! [`crate::validate::validate`] consumes. Resolution is deterministic:
//! selected nodes are de-duplicated and sorted by their term string.
//!
//! Node and predicate IRIs are written `<iri>` (resolved against an optional
//! base); blank nodes `_:label`; literals `"lex"`, `"lex"^^<dt>`, `"lex"@tag`.
//! The shape label is `@START` or `@<label>`.

use purrdf_core::{DatasetView, GraphMatch, RdfDataset, TermId, TermValue};

use crate::ast::Schema;
use crate::error::{Result, ShexError};
use crate::validate::{ResultShapeMap, ShapeSelector, ValidationOptions, validate_with};

/// `rdf:type`, the expansion of the `a` predicate keyword.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// A shape-map node selector: a concrete node or a triple-pattern query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeSelector {
    /// A concrete node.
    Node(TermValue),
    /// `{FOCUS <p> obj?}` — subjects of arcs with this predicate (and object,
    /// when given).
    SubjectOf {
        /// The arc predicate IRI.
        predicate: String,
        /// The required object, or `None` for the `_` wildcard.
        object: Option<TermValue>,
    },
    /// `{subj? <p> FOCUS}` — objects of arcs with this predicate (and subject,
    /// when given).
    ObjectOf {
        /// The required subject, or `None` for the `_` wildcard.
        subject: Option<TermValue>,
        /// The arc predicate IRI.
        predicate: String,
    },
}

/// One `nodeSelector @ shape` association.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeAssociation {
    /// The node selector.
    pub node: NodeSelector,
    /// The associated shape.
    pub shape: ShapeSelector,
}

/// A parsed shape map: associations in document order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShapeMap(pub Vec<ShapeAssociation>);

/// Parse the compact shape-map syntax, resolving relative IRIs against `base`.
///
/// # Examples
///
/// ```
/// use purrdf_shex::{NodeSelector, parse_shape_map};
///
/// let map = parse_shape_map(
///     "<http://example.org/alice>@<http://example.org/UserShape>, \
///      {FOCUS <http://example.org/name> _}@START",
///     None,
/// )
/// .expect("a well-formed shape map parses");
/// assert_eq!(map.0.len(), 2);
/// assert!(matches!(map.0[0].node, NodeSelector::Node(_)));
/// assert!(matches!(map.0[1].node, NodeSelector::SubjectOf { .. }));
/// ```
///
/// # Errors
///
/// Returns [`ShexError::Syntax`] on a grammar violation and [`ShexError::Iri`]
/// when a relative IRI cannot be resolved against `base`.
pub fn parse_shape_map(input: &str, base: Option<&str>) -> Result<ShapeMap> {
    let mut parser = MapParser {
        chars: input.chars().collect(),
        pos: 0,
        base,
    };
    parser.parse_map()
}

/// Expand a shape map against `data` into concrete `(node, shape)` pairs.
///
/// Query selectors are resolved by triple-pattern lookup; concrete nodes pass
/// through unchanged. Selected nodes are de-duplicated and sorted by term
/// string for a deterministic, reproducible order.
#[must_use]
pub fn resolve_shape_map(map: &ShapeMap, data: &RdfDataset) -> Vec<(TermValue, ShapeSelector)> {
    let mut out = Vec::new();
    for assoc in &map.0 {
        match &assoc.node {
            NodeSelector::Node(value) => out.push((value.clone(), assoc.shape.clone())),
            NodeSelector::SubjectOf { predicate, object } => {
                let selected = select_terms(data, object.as_ref(), predicate, Direction::Subject);
                out.extend(selected.into_iter().map(|v| (v, assoc.shape.clone())));
            }
            NodeSelector::ObjectOf { subject, predicate } => {
                let selected = select_terms(data, subject.as_ref(), predicate, Direction::Object);
                out.extend(selected.into_iter().map(|v| (v, assoc.shape.clone())));
            }
        }
    }
    out
}

/// Parse, resolve, and validate a query shape map in one call: the
/// one-call form of [`parse_shape_map`] → [`resolve_shape_map`] →
/// [`crate::validate::validate_with`].
///
/// `map_src` is parsed against `base` and resolved against `data` exactly as
/// [`resolve_shape_map`] does (deterministic order: de-duplicated, sorted by
/// term string), then every resulting `(node, shape)` association is
/// validated with `options` — the same [`crate::validate::validate_with`]
/// call a single fixed shape map would use — and collected into a
/// [`ResultShapeMap`] in that order.
///
/// # Errors
///
/// Returns an error only when `map_src` fails to parse (see
/// [`parse_shape_map`]); a per-node validation failure is reported as a
/// [`ConformanceStatus::Nonconformant`](crate::validate::ConformanceStatus)
/// entry, not an `Err`.
pub fn validate_shape_map(
    schema: &Schema,
    data: &RdfDataset,
    map_src: &str,
    base: Option<&str>,
    options: &ValidationOptions<'_>,
) -> Result<ResultShapeMap> {
    let map = parse_shape_map(map_src, base)?;
    let resolved = resolve_shape_map(&map, data);
    Ok(validate_with(schema, data, &resolved, options))
}

/// Which triple position `FOCUS` occupies.
#[derive(Clone, Copy)]
enum Direction {
    /// `FOCUS` is the subject; `anchor` (if any) is the object.
    Subject,
    /// `FOCUS` is the object; `anchor` (if any) is the subject.
    Object,
}

/// The distinct focus terms of `(anchor? , predicate, ?)` (or `(?, predicate,
/// anchor?)`), sorted by term string.
fn select_terms(
    data: &RdfDataset,
    anchor: Option<&TermValue>,
    predicate: &str,
    direction: Direction,
) -> Vec<TermValue> {
    let Some(pid) = data.term_id_by_value(&TermValue::iri(predicate)) else {
        return Vec::new();
    };
    // A named anchor absent from the data can match nothing.
    let anchor_id = match anchor {
        Some(value) => match data.term_id_by_value(value) {
            Some(id) => Some(id),
            None => return Vec::new(),
        },
        None => None,
    };
    let (s, o): (Option<TermId>, Option<TermId>) = match direction {
        Direction::Subject => (None, anchor_id),
        Direction::Object => (anchor_id, None),
    };
    let mut ids: Vec<TermId> = data
        .quads_for_pattern(s, Some(pid), o, GraphMatch::Any)
        .map(|q| match direction {
            Direction::Subject => q.s,
            Direction::Object => q.o,
        })
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let mut values: Vec<TermValue> = ids.into_iter().map(|id| data.term_value(id)).collect();
    values.sort_by_cached_key(term_key);
    values
}

/// A stable, dataset-independent sort key for a term.
fn term_key(value: &TermValue) -> String {
    match value {
        TermValue::Iri(iri) => format!("0<{iri}>"),
        TermValue::Blank { label, .. } => format!("1_:{label}"),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        } => format!("2{lexical_form}\u{1}{datatype}\u{1}{language:?}"),
        TermValue::Triple { s, p, o } => {
            format!("3{}\u{1}{}\u{1}{}", term_key(s), term_key(p), term_key(o))
        }
    }
}

// ── the parser ────────────────────────────────────────────────────────────────

struct MapParser<'a> {
    chars: Vec<char>,
    pos: usize,
    base: Option<&'a str>,
}

impl MapParser<'_> {
    fn parse_map(&mut self) -> Result<ShapeMap> {
        let mut associations = Vec::new();
        self.skip_ws();
        if self.eof() {
            return Ok(ShapeMap(associations));
        }
        loop {
            associations.push(self.parse_association()?);
            self.skip_ws();
            if self.peek() == Some(',') {
                self.pos += 1;
                self.skip_ws();
                continue;
            }
            break;
        }
        self.skip_ws();
        if !self.eof() {
            return Err(self.err("trailing input after shape map"));
        }
        Ok(ShapeMap(associations))
    }

    fn parse_association(&mut self) -> Result<ShapeAssociation> {
        let node = self.parse_node_selector()?;
        self.skip_ws();
        if self.peek() != Some('@') {
            return Err(self.err("expected '@' between node selector and shape"));
        }
        self.pos += 1;
        self.skip_ws();
        let shape = self.parse_shape_label()?;
        Ok(ShapeAssociation { node, shape })
    }

    fn parse_node_selector(&mut self) -> Result<NodeSelector> {
        self.skip_ws();
        if self.peek() == Some('{') {
            self.parse_triple_pattern()
        } else {
            Ok(NodeSelector::Node(self.parse_term()?))
        }
    }

    fn parse_triple_pattern(&mut self) -> Result<NodeSelector> {
        self.pos += 1; // '{'
        self.skip_ws();
        if self.take_keyword("FOCUS") {
            self.skip_ws();
            let predicate = self.parse_predicate()?;
            self.skip_ws();
            let object = self.parse_term_or_wildcard()?;
            self.skip_ws();
            self.expect('}')?;
            Ok(NodeSelector::SubjectOf { predicate, object })
        } else {
            let subject = self.parse_term_or_wildcard()?;
            self.skip_ws();
            let predicate = self.parse_predicate()?;
            self.skip_ws();
            if !self.take_keyword("FOCUS") {
                return Err(self.err("expected FOCUS in triple pattern"));
            }
            self.skip_ws();
            self.expect('}')?;
            Ok(NodeSelector::ObjectOf { subject, predicate })
        }
    }

    /// A term or the `_` wildcard (`None`). `_:` introduces a blank node, not
    /// a wildcard.
    fn parse_term_or_wildcard(&mut self) -> Result<Option<TermValue>> {
        if self.peek() == Some('_') && self.peek_at(1) != Some(':') {
            self.pos += 1;
            Ok(None)
        } else {
            Ok(Some(self.parse_term()?))
        }
    }

    fn parse_term(&mut self) -> Result<TermValue> {
        match self.peek() {
            Some('<') if self.peek_at(1) == Some('<') => self.parse_triple_term(),
            Some('<') => Ok(TermValue::iri(self.parse_iri()?)),
            Some('_') => self.parse_blank(),
            Some('"') => self.parse_literal(),
            _ => Err(self.err("expected a term (<iri>, _:blank, \"literal\" or <<triple>>)")),
        }
    }

    /// An RDF-1.2 quoted-triple term `<< subject predicate object >>`,
    /// tolerating arbitrary whitespace between the tokens. Recurses so the
    /// three inner positions accept any node the emitter can produce,
    /// including nested `<< >>` terms.
    fn parse_triple_term(&mut self) -> Result<TermValue> {
        self.pos += 2; // '<<'
        self.skip_ws();
        let s = self.parse_term()?;
        self.skip_ws();
        let p = self.parse_term()?;
        self.skip_ws();
        let o = self.parse_term()?;
        self.skip_ws();
        if self.peek() != Some('>') || self.peek_at(1) != Some('>') {
            return Err(self.err("expected '>>' to close a quoted-triple term"));
        }
        self.pos += 2;
        Ok(TermValue::Triple {
            s: Box::new(s),
            p: Box::new(p),
            o: Box::new(o),
        })
    }

    fn parse_predicate(&mut self) -> Result<String> {
        if self.take_keyword("a") {
            return Ok(RDF_TYPE.to_owned());
        }
        if self.peek() == Some('<') {
            self.parse_iri()
        } else {
            Err(self.err("expected a predicate (<iri> or 'a')"))
        }
    }

    fn parse_shape_label(&mut self) -> Result<ShapeSelector> {
        if self.take_keyword("START") {
            return Ok(ShapeSelector::Start);
        }
        if self.peek() == Some('<') {
            Ok(ShapeSelector::Label(self.parse_iri()?))
        } else {
            Err(self.err("expected a shape label (START or <iri>)"))
        }
    }

    fn parse_iri(&mut self) -> Result<String> {
        self.pos += 1; // '<'
        let mut raw = String::new();
        loop {
            match self.peek() {
                Some('>') => {
                    self.pos += 1;
                    return self.resolve(&raw);
                }
                Some(c) if c != '<' && c != '"' && c != '{' && c != '}' && !c.is_control() => {
                    raw.push(c);
                    self.pos += 1;
                }
                _ => return Err(self.err("unterminated IRI")),
            }
        }
    }

    fn parse_blank(&mut self) -> Result<TermValue> {
        // '_' ':' NAME
        self.pos += 1;
        if self.peek() != Some(':') {
            return Err(self.err("expected ':' after '_' in blank node"));
        }
        self.pos += 1;
        let mut label = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
                label.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        if label.is_empty() {
            return Err(self.err("empty blank-node label"));
        }
        Ok(TermValue::blank(label))
    }

    fn parse_literal(&mut self) -> Result<TermValue> {
        self.pos += 1; // opening quote
        let mut lexical = String::new();
        loop {
            match self.peek() {
                Some('"') => {
                    self.pos += 1;
                    break;
                }
                Some('\\') => {
                    self.pos += 1;
                    lexical.push(self.parse_escape()?);
                }
                Some(c) if !c.is_control() => {
                    lexical.push(c);
                    self.pos += 1;
                }
                _ => return Err(self.err("unterminated string literal")),
            }
        }
        // Optional datatype or language tag.
        if self.peek() == Some('^') && self.peek_at(1) == Some('^') {
            self.pos += 2;
            let datatype = self.parse_iri()?;
            Ok(TermValue::typed_literal(lexical, datatype))
        } else if self.peek() == Some('@') {
            self.pos += 1;
            let mut tag = String::new();
            while let Some(c) = self.peek() {
                if c.is_alphanumeric() || c == '-' {
                    tag.push(c);
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if tag.is_empty() {
                return Err(self.err("empty language tag"));
            }
            Ok(TermValue::lang_literal(lexical, &tag))
        } else {
            Ok(TermValue::simple_literal(lexical))
        }
    }

    fn parse_escape(&mut self) -> Result<char> {
        let escaped = self.peek().ok_or_else(|| self.err("dangling escape"))?;
        self.pos += 1;
        match escaped {
            't' => Ok('\t'),
            'b' => Ok('\u{8}'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            'f' => Ok('\u{c}'),
            '"' => Ok('"'),
            '\'' => Ok('\''),
            '\\' => Ok('\\'),
            'u' => self.parse_hex(4),
            'U' => self.parse_hex(8),
            _ => Err(self.err("invalid string escape")),
        }
    }

    fn parse_hex(&mut self, digits: usize) -> Result<char> {
        let mut value: u32 = 0;
        for _ in 0..digits {
            let c = self
                .peek()
                .ok_or_else(|| self.err("short unicode escape"))?;
            let d = c.to_digit(16).ok_or_else(|| self.err("bad hex digit"))?;
            value = value * 16 + d;
            self.pos += 1;
        }
        char::from_u32(value).ok_or_else(|| self.err("escape is not a scalar value"))
    }

    fn resolve(&self, reference: &str) -> Result<String> {
        let Some(base) = self.base else {
            return Ok(reference.to_owned());
        };
        let base = purrdf_iri::parse(base).map_err(|e| ShexError::Iri {
            lexical: base.to_owned(),
            reason: e.to_string(),
        })?;
        let resolved = base.resolve(reference).map_err(|e| ShexError::Iri {
            lexical: reference.to_owned(),
            reason: e.to_string(),
        })?;
        Ok(resolved.as_str().to_owned())
    }

    // ── scanning primitives ──────────────────────────────────────────────────

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.pos + ahead).copied()
    }

    fn eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, ch: char) -> Result<()> {
        if self.peek() == Some(ch) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(&format!("expected '{ch}'")))
        }
    }

    /// Consume `kw` when it appears as a whole token (not followed by a
    /// name character), returning whether it matched.
    fn take_keyword(&mut self, kw: &str) -> bool {
        let end = self.pos + kw.chars().count();
        if end > self.chars.len() {
            return false;
        }
        if !self.chars[self.pos..end].iter().copied().eq(kw.chars()) {
            return false;
        }
        if let Some(next) = self.chars.get(end)
            && (next.is_alphanumeric() || *next == '_')
        {
            return false;
        }
        self.pos = end;
        true
    }

    fn err(&self, reason: &str) -> ShexError {
        ShexError::syntax(reason.to_owned(), self.pos)
    }
}
