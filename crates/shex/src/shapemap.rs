// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
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
//! Selectors see the **RDF 1.2 statement layer** in both directions, not only
//! the quad table: `{FOCUS <annotationPredicate> _}` selects the annotated
//! reifiers, `{_ <annotationPredicate> FOCUS}` selects the annotation objects,
//! and `rdf:reifies` selects reifiers / reified triple terms. (These are RDF
//! 1.2 *statement* annotations — unrelated to the ShEx *schema* annotations of
//! [`crate::ast::Annotation`].)
//!
//! Node and predicate IRIs are written `<iri>` (resolved against an optional
//! base); blank nodes `_:label`; literals `"lex"`, `"lex"^^<dt>`, `"lex"@tag`.
//! The shape label is `@START` or `@<label>`.
//!
//! # No prefixed names, because the grammar has none and the spec declines to
//! # say what one would mean
//!
//! `ex:S1` is rejected everywhere an IRI is expected, and the diagnostic says so
//! by name rather than as a bare syntax error. This is the grammar's own answer,
//! not a shortcut:
//!
//! * the ShapeMap grammar's `[136s] iri` production is **`IRIREF`** — the
//!   `| prefixedName` alternative is present in the specification's source but
//!   **commented out**, as are `shapeSpec`'s `ATPNAME_NS`/`ATPNAME_LN` arms. The
//!   `prefixedName`/`PNAME_LN`/`PNAME_NS`/`PN_PREFIX` productions survive as
//!   defined-but-unreachable, so the exclusion is deliberate rather than an
//!   oversight. ShapeMap does **not** inherit ShExC's
//!   `iri ::= IRIREF | PrefixedName`; it defines its own and drops the alternative;
//! * the one paragraph that mentions prefixes (§ "ShapeMap grammar") introduces a
//!   *resolution context* — "a base IRI and a map of prefix to namespace IRI" —
//!   and then says: "Though it is common practice to resolve shape references
//!   against a `resolution context` found in the schema and node references
//!   agianst \[sic] a `resolution context` found in the data (e.g. Turtle
//!   prefixes), **this specification does not specifiy \[sic] that behavior**."
//!
//! So the spec names the exact rule an implementer would reach for and explicitly
//! refuses to standardize it. Resolving `ex:` against the schema's prefixes would
//! be inventing a binding the specification withheld — the same fabricated default
//! this repository refuses for every other caller-supplied vocabulary — and the
//! spec's own sentence has the two halves of one association resolving against
//! *different* documents, which can bind one prefix to two namespaces. There is no
//! single rule to implement, so none is: a prefixed name is refused with the reason.
//!
//! The **base** is a different case and is threaded, which is why `<S1>` works: a
//! relative `IRIREF` is live in the grammar, and the base half of the resolution
//! context has a caller-supplied source (RFC-3986 §5.1.2). A prefixed name is a
//! disabled production whose environment the spec declined to define.
//!
//! # ShapeMap's terminals are ShapeMap's own, and it has a whitespace production
//!
//! This scanner is not the ShExC lexer and must not borrow its classes by
//! association. The ShapeMap grammar publishes its own terminals section, and it
//! answers both questions a scanner has to ask, for itself:
//!
//! * **Whitespace.** ShapeMap does define one. Its terminals section names the
//!   skippable text directly — *"The PASSED TOKENS below may appear between any
//!   terminals or literal strings which appear in the grammar above"* — and
//!   spells it
//!
//!   ```text
//!   PASSED TOKENS ::= [ \t\r\n]+ | "#" [^\r\n]*
//!   ```
//!
//!   The whitespace alternative enumerates exactly `#x20`, `#x9`, `#xD` and
//!   `#xA`. Those are the same four scalars as the `WS` terminal
//!   [`terminals::is_ws`] transcribes, so routing there is not an assumption
//!   that ShapeMap inherits Turtle's `WS` — it is two grammars independently
//!   naming one set. (PurRDF implements only the whitespace alternative; the
//!   `"#" [^\r\n]*` comment alternative is not accepted here, and never was.)
//!
//! * **Name characters.** The terminals are tagged with the grammar they come
//!   from — *"Production numbers followed by a letter correspond to productions
//!   in other grammars: t: RDF 1.1 Turtle, s: SPARQL 1.1, x: ShEx 2"* — and
//!   `[164s] PN_CHARS_BASE`, `[165s] PN_CHARS_U` and `[167s] PN_CHARS` carry the
//!   `s`, so they are SPARQL's productions, quoted verbatim into this grammar.
//!   That is the citation for reading them from [`terminals`], and it is a
//!   citation to ShapeMap's own document.
//!
//! A Unicode property is never the answer to either. `char::is_whitespace` is
//! twenty-six scalars where `PASSED TOKENS` names four, and `char::is_alphanumeric`
//! both admits scalars `PN_CHARS` excludes (U+00AA, U+00B2, U+2460) and excludes
//! scalars `PN_CHARS` admits (every combining mark in `[#x300-#x36F]` — which is
//! what an NFD-decomposed `é` is made of). Because a scanner's classes decide
//! where a token STOPS, either direction re-tokenizes silently rather than
//! erroring; see [`terminals`] for the worked counterexample.

use purrdf_core::{DatasetView, GraphMatch, RdfDataset, TermId, TermValue};
use purrdf_iri::{BaseIri, BaseOrigin, BaseScope, terminals};

use crate::ast::Schema;
use crate::error::{Result, ShexError};
use crate::lexer::{UcharDefect, decode_uchar};
use crate::statement;
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
/// when a relative IRI cannot be resolved — including when `base` is `None`, where
/// a relative reference is the shared `iri-relative-no-base` failure rather than a
/// selector that silently matches nothing.
pub fn parse_shape_map(input: &str, base: Option<&str>) -> Result<ShapeMap> {
    let scope = match base {
        Some(iri) => BaseScope::rooted(
            BaseIri::parse(iri).map_err(|e| ShexError::iri(iri, &e))?,
            BaseOrigin::Caller,
        ),
        None => BaseScope::empty(),
    };
    let mut parser = MapParser {
        chars: input.chars().collect(),
        pos: 0,
        base: scope,
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
/// # A shape the schema does not declare is refused, not answered
///
/// Every association's shape selector must name something the schema declares —
/// a label in its `shapes` map (after [`crate::resolve_imports`] has folded the
/// import closure in), or `START` when the schema has a `start`. One that does
/// not is [`ShexError::UnknownShape`] before any node is checked.
///
/// Neither specification defines this case. ShEx 2.1 §5.7's *Shape Expression
/// Reference Requirement* — "A shapeExprRef MUST appear in the schema's shapes
/// map (or an imported schema's map)" — binds a reference written INSIDE a
/// schema, and [`crate::check_structure`] is what enforces it; `satisfies` is
/// defined only where "se2 is the shape expression having se as id", so with no
/// such expression the relation is undefined rather than false. The ShapeMap
/// specification says a `shapeLabel` is "ShEx shapeExprLabel or the string START"
/// and is silent on labels the schema lacks, and its `status` vocabulary is
/// `conformant`/`nonconformant` with no third value meaning "not evaluated".
///
/// So answering `nonconformant` would spend the one word the format has for a
/// finding about the DATA on a mistake the data had no part in: it reads as "this
/// node fails that shape" when the truth is "there is no such shape". The
/// repository's no-optionality/hard-fail doctrine decides an undefined case, and
/// it decides this one as a caller error.
///
/// # Errors
///
/// Returns an error when `map_src` fails to parse (see [`parse_shape_map`]) or
/// names a shape the schema does not declare. A per-node validation failure is
/// reported as a
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
    refuse_undeclared_shapes(schema, &map)?;
    let resolved = resolve_shape_map(&map, data);
    Ok(validate_with(schema, data, &resolved, options))
}

/// Refuse a shape map naming a shape `schema` does not declare.
///
/// Checked BEFORE the node selectors are expanded, so the refusal does not
/// depend on whether the data happened to select any node: a map that selects
/// nothing and a map that selects a thousand nodes are the same mistake, and an
/// empty result shape map would hide it entirely.
///
/// The first offending association is named, in the ShapeMap grammar's own
/// spelling, rather than a count — the operator has to fix one label at a time.
fn refuse_undeclared_shapes(schema: &Schema, map: &ShapeMap) -> Result<()> {
    for assoc in &map.0 {
        let declared = match &assoc.shape {
            ShapeSelector::Start => schema.start.is_some(),
            ShapeSelector::Label(label) => schema.shapes.iter().any(|decl| &decl.id == label),
        };
        if !declared {
            return Err(ShexError::unknown_shape(
                crate::validate::shape_term_string(&assoc.shape),
            ));
        }
    }
    Ok(())
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
    // The RDF 1.2 statement layer lives in side-tables outside `quads`, so a
    // selector over `rdf:reifies` or an annotation predicate must consult it
    // too — in both directions.
    statement::visit_quads(data, s, Some(pid), o, |qs, _, qo| {
        ids.push(match direction {
            Direction::Subject => qs,
            Direction::Object => qo,
        });
    });
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

/// `[145s] LANGTAG ::= "@" ([a-zA-Z])+ ("-" ([a-zA-Z0-9])+)*`, on the body after
/// the `@`.
///
/// **This one is deliberately NOT a `PN_CHARS` run and not a Unicode property.**
/// `LANGTAG` is the one terminal in the ShapeMap grammar that enumerates plain
/// ASCII letters and digits, and it is position-dependent besides: the primary
/// subtag is letters only, every later subtag is letters or digits, and neither
/// may be empty. Answering it with `char::is_alphanumeric` was wrong twice over —
/// it admitted `"x"@日本語`, which the production does not name at all, and it
/// treated the structure as a flat character run, so `"x"@en-` and `"x"@1ab` were
/// accepted as language tags they are not.
///
/// Every real tag still passes, which is the point: `en`, `en-UK`, `zh-Hans`,
/// `de-CH-1901`, `x-private` and the grandfathered `i-klingon` all begin with an
/// alphabetic subtag and continue with alphanumeric ones.
fn is_langtag(tag: &str) -> bool {
    let mut subtags = tag.split('-');
    let primary = subtags.next().unwrap_or_default();
    if primary.is_empty() || !primary.bytes().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    subtags.all(|sub| !sub.is_empty() && sub.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// `[18t] IRIREF ::= "<" ([^#x00-#x20<>"{}|^`\] | UCHAR)* ">"` — the content class,
/// as the complement of [`terminals::is_iriref_forbidden`].
///
/// A content class is a terminal like any other, and `char::is_control` stood in for
/// this one while being wrong in **both** directions at once, which is why the error
/// was invisible from either side alone: it admitted `#x20` SPACE (so `<urn:ex:a b>`
/// named an IRI with a space in it), refused the lawful U+007F-U+009F, and said
/// nothing at all about `` ` ``, `|`, `^` and `\`, four of the nine delimiters the
/// production excludes by name. The enumerated replacement that followed was a THIRD
/// transcription of a production the workspace already owned, so the exclusion set
/// now comes from the shared module and this function is just the polarity flip the
/// scan loop reads more naturally in.
///
/// This answers the RAW half of the production only. `'\'` is excluded here because
/// it is excluded there — the production admits it solely as the lead of the `UCHAR`
/// alternative — so [`MapParser::parse_iri`] claims the backslash in its own arm,
/// before this class is ever consulted, and decodes the escape.
const fn is_iriref_content(c: char) -> bool {
    !terminals::is_iriref_forbidden(c)
}

/// `[14t] STRING_LITERAL2 ::= '"' ([^#x22#x5C#xA#xD] | ECHAR | UCHAR)* '"'` — the
/// content class of the quoted form this scanner accepts.
///
/// Exactly four scalars leave it: the closing quote and the escape lead (both handled
/// by their own arms before this one is reached) and the two line terminators. Every
/// other control is lawful *content* — a raw TAB inside a literal is a TAB — and
/// `char::is_control` refused all of them, so a shape map carrying a tabbed lexical
/// form was reported as an unterminated string.
const fn is_string_literal_content(c: char) -> bool {
    !matches!(c, '"' | '\\' | '\n' | '\r')
}

struct MapParser {
    chars: Vec<char>,
    pos: usize,
    base: BaseScope,
}

impl MapParser {
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
            // `_` first: `_:label` is a blank node, and `peek_prefixed_name` excludes it.
            Some('_') => self.parse_blank(),
            Some('"') => self.parse_literal(),
            _ => match self.peek_prefixed_name() {
                Some(name) => Err(self.prefixed_name_err(&name, "a node must be an IRI")),
                None => {
                    Err(self.err("expected a term (<iri>, _:blank, \"literal\" or <<triple>>)"))
                }
            },
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
        // BEFORE the `a` keyword: `a:b` is a prefixed name whose prefix happens to be
        // `a`, and taking the keyword first would read it as rdf:type plus junk.
        if let Some(name) = self.peek_prefixed_name() {
            return Err(self.prefixed_name_err(&name, "a predicate must be `a` or an IRI"));
        }
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
        // BEFORE the `START` keyword, for the reason `parse_predicate` checks before `a`:
        // `START:S` is a prefixed name, not the start-shape keyword followed by junk.
        if let Some(name) = self.peek_prefixed_name() {
            return Err(self.prefixed_name_err(&name, "a shape label must be `START` or an IRI"));
        }
        if self.take_keyword("START") {
            return Ok(ShapeSelector::Start);
        }
        if self.peek() == Some('<') {
            Ok(ShapeSelector::Label(self.parse_iri()?))
        } else {
            Err(self.err("expected a shape label (START or <iri>)"))
        }
    }

    /// The Turtle-family PREFIXED NAME at the cursor (`ex:S1`, `:S1`, `ex:`), if the
    /// input looks like one. Detection only — nothing is consumed.
    ///
    /// `_:label` is deliberately NOT one: that is `BLANK_NODE_LABEL`, which the grammar
    /// does admit wherever a term is allowed, so it must keep reaching [`Self::parse_blank`].
    ///
    /// # Which productions bound the two runs
    ///
    /// The name halves are scanned with the productions ShapeMap's terminals section
    /// keeps defined for them even though their only consumer is commented out —
    /// `[168s] PN_PREFIX ::= PN_CHARS_BASE ((PN_CHARS | ".")* PN_CHARS)?` before the
    /// colon and `[169s] PN_LOCAL ::= (PN_CHARS_U | ":" | [0-9] | PLX) ((PN_CHARS |
    /// "." | ":" | PLX)* (PN_CHARS | ":" | PLX))?` after it. Both bound a run of
    /// `[167s] PN_CHARS` plus `'.'`, and the local half additionally admits the `'%'`
    /// that opens `[171s] PERCENT ::= "%" HEX HEX` (via `[170s] PLX`). The trailing
    /// `'.'` is trimmed because neither production may end on one.
    ///
    /// `char::is_alphanumeric` stood here and was wrong on both edges: it admits
    /// U+00AA and U+2460, which `PN_CHARS` does not, and refuses every combining mark
    /// in `[#x300-#x36F]`, which `PN_CHARS` does — so an NFD-spelled `ex:café` was
    /// quoted back at the author as `ex:cafe`, naming a different IRI than the one
    /// being refused.
    ///
    /// This run decides what a REJECTION QUOTES, not what the parser accepts: nothing
    /// is consumed, and every position that calls it has already established that a
    /// conforming term cannot start here (a conforming one starts `<`, `_:` or `"`, or
    /// is the bare keyword the caller checks next). Getting the class right therefore
    /// cannot widen or narrow the accepted language — it decides only whether the
    /// diagnostic names the input the author actually wrote.
    fn peek_prefixed_name(&self) -> Option<String> {
        let mut at = self.pos;
        let mut name = String::new();
        while let Some(&c) = self.chars.get(at) {
            if terminals::is_pn_chars(c) || c == '.' {
                name.push(c);
                at += 1;
            } else {
                break;
            }
        }
        // `PNAME_NS ::= PN_PREFIX? ':'`, so an empty prefix still forms one.
        if self.chars.get(at) != Some(&':') || name == "_" {
            return None;
        }
        name.push(':');
        at += 1;
        while let Some(&c) = self.chars.get(at) {
            if terminals::is_pn_chars(c) || matches!(c, '.' | '%') {
                name.push(c);
                at += 1;
            } else {
                break;
            }
        }
        Some(name.trim_end_matches('.').to_owned())
    }

    /// Reject a prefixed name by NAMING the reason, rather than as a bare syntax error.
    ///
    /// The ShapeMap grammar's `iri` production is `IRIREF` alone; the specification's own
    /// source carries the `| prefixedName` alternative COMMENTED OUT (and the same for
    /// `shapeSpec`'s `ATPNAME_NS`/`ATPNAME_LN` arms), so this is a deliberate exclusion
    /// rather than an omission. See the module documentation for why PurRDF does not
    /// extend past it.
    fn prefixed_name_err(&self, name: &str, position: &str) -> ShexError {
        ShexError::syntax(
            format!(
                "{position} in angle brackets; `{name}` is a prefixed name, and the ShapeMap \
                 grammar admits none — its `iri` production is IRIREF only. The specification \
                 also declines to say where a prefix map would come from, recording that \
                 resolving shape references against the schema's prefixes and node references \
                 against the data's is \"common practice\" that \"this specification does not \
                 specify\"; two documents may bind one prefix differently, so PurRDF resolves \
                 none rather than silently choosing a side. Write the IRI in full \
                 (`<http://example.org/S1>`), or a relative `<S1>` against a base"
            ),
            self.pos,
        )
    }

    /// An `IRIREF`: `'<' … '>'`, resolved against the base in scope.
    ///
    /// ```text
    /// [18t] IRIREF ::= '<' ([^#x00-#x20<>"{}|^`\] | UCHAR)* '>'
    /// ```
    ///
    /// The opening `<` is verified HERE rather than trusted from the caller. It used not
    /// to be, and the one caller that did not pre-check — a literal's `^^` datatype — read
    /// the first character of `"7"^^xsd:integer`'s datatype as the opening bracket and ran
    /// to end-of-input, reporting `unterminated IRI` for a document whose real defect was a
    /// prefixed name.
    ///
    /// # The production has two alternatives, and this scan reads both
    ///
    /// `UCHAR` is the second one, and a scan that answers only the raw content class
    /// **over-refuses**: `'\'` is one of the nine delimiters, so the escape's own lead
    /// character falls out of the body and every escaped spelling — `<urn:ex:\U0001F600>`,
    /// or any `<…\uXXXX…>` at all — was reported as `unterminated IRI`. That spelling is
    /// not exotic: it is what this workspace's own IRI egress escape emits, since it
    /// carries `#x00-#x20`, the nine delimiters and the control blocks as `\uXXXX`, so
    /// the refusal broke read-back of PurRDF's own output. Refusing a document the
    /// grammar admits is the mirror of resolving the wrong node, not the safe side of it.
    ///
    /// The escape is decoded by [`decode_uchar`], the transcription the ShExC lexer
    /// scans `IRIREF` with, so the two front ends cannot drift on the three edges that
    /// decide identity: a bare `'\'` is a hard error rather than a literal backslash;
    /// `HEX ::= [0-9] | [A-F] | [a-f]`, so the upper- and lower-case spellings of an
    /// `é` name one IRI and not two; and a value above U+10FFFF or inside the
    /// U+D800-U+DFFF surrogate block is refused rather than replaced with U+FFFD.
    ///
    /// # A decoded scalar is a VALUE, not a re-scanned character
    ///
    /// The production's raw content class constrains the characters that stand in the
    /// *source*; `UCHAR` is the mechanism by which the others are written at all, so
    /// what it decodes to is not fed back through that class. This is forced, not
    /// chosen: [`purrdf_core`]'s IRI egress escape emits exactly the excluded scalars
    /// as `\uXXXX`, so re-refusing them on ingress would mean this parser could not
    /// read back what the workspace writes. A `UCHAR` denoting U+0020 therefore
    /// contributes a SPACE to the value, and whether *that* string is an IRI at all is
    /// the next layer's question — [`Self::resolve`] puts it to [`BaseScope`], which
    /// answers RFC 3987 rather than the Turtle-family terminal (and refuses the space,
    /// naming the IRI instead of blaming a runaway bracket scan).
    fn parse_iri(&mut self) -> Result<String> {
        if self.peek() != Some('<') {
            if let Some(name) = self.peek_prefixed_name() {
                return Err(self.prefixed_name_err(&name, "an IRI must be written"));
            }
            return Err(self.err("expected an IRI in angle brackets"));
        }
        self.pos += 1; // '<'
        let mut raw = String::new();
        loop {
            match self.peek() {
                Some('>') => {
                    self.pos += 1;
                    return self.resolve(&raw);
                }
                Some('\\') => {
                    let decoded = self.read_uchar()?;
                    raw.push(decoded);
                }
                Some(c) if is_iriref_content(c) => {
                    raw.push(c);
                    self.pos += 1;
                }
                _ => return Err(self.err("unterminated IRI")),
            }
        }
    }

    /// Decode the `UCHAR` whose backslash the cursor stands on, advancing past it.
    ///
    /// ```text
    /// UCHAR ::= '\u' HEX HEX HEX HEX | '\U' HEX HEX HEX HEX HEX HEX HEX HEX
    /// HEX   ::= [0-9] | [A-F] | [a-f]
    /// ```
    ///
    /// Every failure is a hard error rather than a fallback to the raw backslash,
    /// because no production reachable from here gives `'\'` any other reading.
    fn read_uchar(&mut self) -> Result<char> {
        let (decoded, consumed) =
            decode_uchar(|ahead| self.peek_at(ahead)).map_err(|defect| match defect {
                UcharDefect::NotAnEscape => {
                    self.err("a backslash in an IRI must open a \\u/\\U escape")
                }
                UcharDefect::BadHex => self.err("bad \\u/\\U escape in IRI (expected hex digits)"),
                UcharDefect::NotAScalar => {
                    self.err("\\u/\\U escape in IRI is not a Unicode scalar value")
                }
            })?;
        self.pos += consumed;
        Ok(decoded)
    }

    /// `[142s] BLANK_NODE_LABEL ::= "_:" (PN_CHARS_U | [0-9]) ((PN_CHARS | ".")* PN_CHARS)?`
    ///
    /// The label body is therefore a run of `[167s] PN_CHARS` plus `'.'`, which is why
    /// it is [`terminals::is_pn_chars`] and not a Unicode letter property. The two
    /// differ in both directions and each direction moves the boundary: U+0301
    /// COMBINING ACUTE is `PN_CHARS` and is not alphanumeric, so `_:café` spelled NFD
    /// used to end the label at `caf` and then fail on a `́` that had nowhere to go;
    /// U+2460 CIRCLED DIGIT ONE is alphanumeric and is not `PN_CHARS`, so it used to
    /// be absorbed into a label the grammar says stops before it.
    ///
    /// # The production is position-dependent, and so is the scan
    ///
    /// It names **three** classes, not one, and a run of `PN_CHARS` answers only the
    /// middle of them. Both edges decide identity, so getting either wrong mints a
    /// blank node the author did not write:
    ///
    /// * the FIRST scalar is `PN_CHARS_U | [0-9]` — narrower than `PN_CHARS`, which
    ///   also carries `'-'`, U+00B7 and the combining marks. A uniform scan read
    ///   `_:-z` and `_:` + U+0301 + `z` as labels, and neither is one;
    /// * the LAST may not be `'.'`. A dot is lawful *between* name characters, so the
    ///   scan over-consumes a trailing run and hands it back — the same pushback the
    ///   ShExC lexer and the query front end perform. A shape map has no production
    ///   that admits a bare `'.'` anywhere, so the handed-back dot then ends the
    ///   parse with the defect named rather than being absorbed into an identity.
    ///
    /// Neither edge is an over-refusal risk in the other direction: `_:a.b` keeps its
    /// internal dot, `_:0z` still begins with a digit, and `_:_z` still begins with
    /// the one `PN_CHARS_U` member that is not `PN_CHARS_BASE`.
    fn parse_blank(&mut self) -> Result<TermValue> {
        // '_' ':' NAME
        self.pos += 1;
        if self.peek() != Some(':') {
            return Err(self.err("expected ':' after '_' in blank node"));
        }
        self.pos += 1;
        let mut label = String::new();
        match self.peek() {
            Some(c) if terminals::is_pn_chars_u(c) || c.is_ascii_digit() => {
                label.push(c);
                self.pos += 1;
            }
            _ => {
                return Err(
                    self.err("a blank-node label must begin with PN_CHARS_U or a digit after `_:`")
                );
            }
        }
        let mut trailing_dots = 0usize;
        while let Some(c) = self.peek() {
            if c == '.' {
                label.push(c);
                trailing_dots += 1;
                self.pos += 1;
            } else if terminals::is_pn_chars(c) {
                label.push(c);
                trailing_dots = 0;
                self.pos += 1;
            } else {
                break;
            }
        }
        if trailing_dots > 0 {
            // `'.'` is one byte, so the trimmed byte length is the dot run.
            label.truncate(label.len() - trailing_dots);
            self.pos -= trailing_dots;
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
                Some(c) if is_string_literal_content(c) => {
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
                if c.is_ascii_alphanumeric() || c == '-' {
                    tag.push(c);
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if !is_langtag(&tag) {
                return Err(
                    self.err("expected a language tag: LANGTAG is [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*")
                );
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

    /// Resolve an `<iri>` against the base the map was parsed with.
    ///
    /// The compact shape-map syntax admits relative references, so this is
    /// [`BaseScope::resolve`] — the same entry point the ShExC parser uses, with the
    /// same refusal when no base is in scope. Keeping a relative reference verbatim
    /// here was worse than in a schema: an unresolvable node selector matches
    /// nothing in the data and reports a clean, empty result map.
    fn resolve(&self, reference: &str) -> Result<String> {
        self.base
            .resolve(reference)
            .map(|iri| iri.as_str().to_owned())
            .map_err(|e| ShexError::iri(reference, &e))
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

    /// Skip the whitespace alternative of ShapeMap's own
    /// `PASSED TOKENS ::= [ \t\r\n]+ | "#" [^\r\n]*`.
    ///
    /// Four scalars — `#x20`, `#x9`, `#xD`, `#xA` — which is the set
    /// [`terminals::is_ws`] transcribes; see the [module documentation](self) for
    /// why that is ShapeMap's own citation rather than a loan from ShExC.
    ///
    /// `char::is_whitespace` stood here, and the twenty-two extra scalars it skipped
    /// are not separators in this grammar: U+00A0 NO-BREAK SPACE and U+3000
    /// IDEOGRAPHIC SPACE between two terminals gave a document no reading at all, yet
    /// it parsed. `u8::is_ascii_whitespace` is not the correction either — it admits
    /// U+000C FORM FEED, which `PASSED TOKENS` does not name.
    ///
    /// This scanner holds decoded scalars, so it reaches for the scalar-shaped
    /// [`terminals::is_ws_char`] rather than narrowing to a byte here. The two are
    /// one table: every `WS` member is ASCII, so the widened search answers `false`
    /// for U+00A0 and everything above it, which is exactly what the narrowing used
    /// to prove locally.
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if terminals::is_ws_char(c) {
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

    /// Consume `kw` when it appears as a whole token, returning whether it matched.
    ///
    /// # What decides the follow boundary — and what does not
    ///
    /// `'a'`, `"FOCUS"` and `"START"` are literal strings inside productions
    /// `[4] subjectTerm`, `[6] triplePattern` and `[7] shapeLabel`, matched under the
    /// rule the terminals section states: *"Text is matched against the longest
    /// matching terminal."* So the question a keyword's right edge asks is not "is the
    /// next character a letter?" — a property of one scalar — but "could a LONGER
    /// terminal have started here?", which only the grammar can answer.
    ///
    /// The only terminal in this family built from a run of name characters is a
    /// Turtle-style name: `[168s] PN_PREFIX` and `[169s] PN_LOCAL`, which ShapeMap
    /// keeps defined (their consumer, `prefixedName`, is commented out of
    /// `[136s] iri` — see the [module documentation](self)). Both are runs of
    /// `[167s] PN_CHARS`, so `PN_CHARS` is exactly the class a keyword must not be
    /// carved out of the middle of, and [`terminals::is_pn_chars`] is what decides it.
    /// `START:S1` is one such name, and the callers check
    /// [`Self::peek_prefixed_name`] BEFORE reaching here so it is refused by name.
    ///
    /// **Everything else is a boundary, and refusing those would be the mirror bug.**
    /// The grammar's own follow sets are punctuation and end-of-input: `','` ends a
    /// `shapeAssociation` in `[1] shapeMap`, `'}'` closes `[6] triplePattern`, and
    /// `'<'` opens the `[18t] IRIREF` that `[136s] iri` is. None is `PN_CHARS`, so
    /// `@START,`, `{_ <p> FOCUS}`, `{FOCUS<p> _}` and `@START` at end-of-input all
    /// still take the keyword — a "keyword must be followed by whitespace" rule would
    /// reject every one of them.
    ///
    /// `char::is_alphanumeric() || '_'` stood here. It never rejected a conforming
    /// document, because no conforming follow character is alphanumeric — but it was
    /// the wrong question asked of the wrong set, and it disagreed with `PN_CHARS` in
    /// both directions (U+2460 is alphanumeric and not a name character; every
    /// combining mark in `[#x300-#x36F]` is a name character and not alphanumeric).
    fn take_keyword(&mut self, kw: &str) -> bool {
        let end = self.pos + kw.chars().count();
        if end > self.chars.len() {
            return false;
        }
        if !self.chars[self.pos..end].iter().copied().eq(kw.chars()) {
            return false;
        }
        if let Some(next) = self.chars.get(end)
            && terminals::is_pn_chars(*next)
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
