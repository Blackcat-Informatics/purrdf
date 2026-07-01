// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RDF 1.2 term-level types for the SPARQL algebra.
//!
//! These mirror the *structure* of the W3C SPARQL term model (and the surface
//! the consumers of the prior oxigraph-family parser walk) but are purrdf-owned: a [`NamedNode`]
//! wraps a lexical IRI validated by [`purrdf_iri`], and a [`Literal`] carries a
//! lexical form + datatype (optionally validated by [`purrdf_xsd`]). They carry
//! **no variables** at the term level except through the `*Pattern` types, which
//! is exactly the split SPARQL's algebra needs (ground data vs. query patterns).
//!
//! RDF 1.2 quoted triple terms are first-class: [`TermPattern::Triple`] (in query
//! patterns) and [`GroundTerm::Triple`] (in ground data, e.g. `VALUES`).

use crate::error::{ParseError, Result};

/// A datatype IRI literal used for plain (non-typed) literals: `xsd:string`.
pub const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// The datatype IRI for language-tagged strings: `rdf:langString`.
pub const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
/// The datatype IRI for base-direction strings (RDF 1.2): `rdf:dirLangString`.
pub const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

/// An absolute IRI in term position (e.g. a predicate, a class, a datatype).
///
/// The lexical form is stored verbatim (Constitution C0.1: IRIs are
/// lexical-verbatim); [`NamedNode::new`] validates it against RFC-3987 via
/// `purrdf-iri`, while [`NamedNode::new_unchecked`] trusts the caller.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NamedNode {
    iri: String,
}

impl NamedNode {
    /// Validate and wrap an absolute IRI. Returns [`ParseError::Iri`] if the
    /// string is not a valid RFC-3987 IRI, or if it is a relative reference
    /// (term-position IRIs — predicates, datatypes, `GRAPH`/`SERVICE` names —
    /// must be absolute; `purrdf_iri::parse` itself admits relative references).
    pub fn new(iri: impl Into<String>) -> Result<Self> {
        let iri = iri.into();
        let parsed = purrdf_iri::parse(&iri).map_err(|e| ParseError::Iri {
            lexical: iri.clone(),
            reason: e.to_string(),
        })?;
        if !parsed.has_scheme() {
            return Err(ParseError::Iri {
                lexical: iri.clone(),
                reason: "relative IRI reference in term position (no scheme)".to_owned(),
            });
        }
        Ok(Self { iri })
    }

    /// Wrap an IRI without validation. Use only when the source is already known
    /// to be a valid IRI (e.g. round-tripping an already-parsed node).
    pub fn new_unchecked(iri: impl Into<String>) -> Self {
        Self { iri: iri.into() }
    }

    /// The IRI lexical form.
    pub fn as_str(&self) -> &str {
        &self.iri
    }
}

impl core::fmt::Debug for NamedNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<{}>", self.iri)
    }
}

/// A blank node, identified by its label (without the `_:` prefix).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BlankNode {
    id: String,
}

impl BlankNode {
    /// Wrap a blank-node label (the part after `_:`).
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// The blank-node label (without `_:`).
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

impl core::fmt::Debug for BlankNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "_:{}", self.id)
    }
}

/// A query variable (without the leading `?`/`$`).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Variable {
    name: String,
}

impl Variable {
    /// Wrap a variable name (the part after `?` or `$`).
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// The variable name (without the sigil).
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl core::fmt::Debug for Variable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "?{}", self.name)
    }
}

/// The base text direction of an RDF 1.2 directional language-tagged string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BaseDirection {
    /// Left-to-right (`--ltr`).
    Ltr,
    /// Right-to-left (`--rtl`).
    Rtl,
}

/// An RDF literal: a lexical form plus a datatype, and (for `rdf:langString`)
/// an optional language tag and RDF 1.2 base direction.
///
/// The datatype is **always** materialized: a plain string literal carries
/// `xsd:string`, a language-tagged one `rdf:langString`. This matches the
/// consumer contract `literal.datatype().as_str()` (the only thing the existing
/// IRI-extraction walker reads off a literal).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Literal {
    value: String,
    datatype: NamedNode,
    language: Option<String>,
    direction: Option<BaseDirection>,
}

impl Literal {
    /// A simple `xsd:string` literal.
    pub fn new_simple(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            datatype: NamedNode::new_unchecked(XSD_STRING),
            language: None,
            direction: None,
        }
    }

    /// A typed literal `"value"^^<datatype>`.
    pub fn new_typed(value: impl Into<String>, datatype: NamedNode) -> Self {
        Self {
            value: value.into(),
            datatype,
            language: None,
            direction: None,
        }
    }

    /// A language-tagged literal `"value"@lang`, optionally with an RDF 1.2 base
    /// direction (`"value"@lang--ltr`).
    pub fn new_lang(
        value: impl Into<String>,
        language: impl Into<String>,
        direction: Option<BaseDirection>,
    ) -> Self {
        let datatype = NamedNode::new_unchecked(if direction.is_some() {
            RDF_DIR_LANG_STRING
        } else {
            RDF_LANG_STRING
        });
        Self {
            value: value.into(),
            datatype,
            language: Some(language.into()),
            direction,
        }
    }

    /// The lexical form (never the value space — this is a syntactic AST).
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The datatype IRI. For language-tagged literals this is `rdf:langString`
    /// (or `rdf:dirLangString` when a base direction is present).
    pub fn datatype(&self) -> &NamedNode {
        &self.datatype
    }

    /// The language tag, if any.
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// The RDF 1.2 base direction, if any.
    pub fn direction(&self) -> Option<BaseDirection> {
        self.direction
    }
}

impl core::fmt::Debug for Literal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match (&self.language, self.direction) {
            (Some(lang), Some(dir)) => write!(f, "{:?}@{lang}--{dir:?}", self.value),
            (Some(lang), None) => write!(f, "{:?}@{lang}", self.value),
            (None, _) => write!(f, "{:?}^^{:?}", self.value, self.datatype),
        }
    }
}

/// An IRI or a variable in a position that admits only those two (a predicate,
/// or a `GRAPH`/`SERVICE` name).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NamedNodePattern {
    /// A concrete IRI.
    NamedNode(NamedNode),
    /// A variable standing in for the IRI.
    Variable(Variable),
}

/// A term in a query pattern: a concrete term, a variable, or — for RDF 1.2 — a
/// quoted triple term (`<<( s p o )>>`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TermPattern {
    /// An IRI.
    NamedNode(NamedNode),
    /// A blank node.
    BlankNode(BlankNode),
    /// A literal.
    Literal(Literal),
    /// A variable.
    Variable(Variable),
    /// An RDF 1.2 quoted triple term in term position.
    Triple(Box<TriplePattern>),
}

/// A triple pattern `s p o`, where the predicate admits only an IRI or variable
/// and the subject/object admit the full [`TermPattern`] surface (including
/// nested RDF 1.2 quoted triples).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TriplePattern {
    /// The subject term.
    pub subject: TermPattern,
    /// The predicate (IRI or variable).
    pub predicate: NamedNodePattern,
    /// The object term.
    pub object: TermPattern,
}

/// A ground term (no variables): the cell type of a `VALUES` block. RDF 1.2
/// ground quoted triples are admitted via [`GroundTerm::Triple`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GroundTerm {
    /// An IRI.
    NamedNode(NamedNode),
    /// A literal.
    Literal(Literal),
    /// A ground RDF 1.2 quoted triple term.
    Triple(Box<GroundTriple>),
    /// A blank node — **injection-only** (purrdf S5, EPIC #906 GAP-A). The SPARQL
    /// grammar forbids a blank node in a `VALUES`/`DataBlock` cell, so the
    /// [parser](crate::parser) NEVER produces this variant. It exists solely so
    /// [`Query::substitute_variable`](crate::Query::substitute_variable) can pre-bind
    /// a **blank-node** focus node (SHACL `$this` may be a blank) through the same
    /// single-row `VALUES`-join rewrite the IRI/literal/triple cases use, keeping the
    /// substitution path uniform across every term kind. The evaluator interns it as
    /// an ordinary blank (`purrdf-core` `TermValue::Blank`).
    BlankNode(BlankNode),
}

/// A ground triple term `s p o` (no variables), used inside [`GroundTerm`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GroundTriple {
    /// The subject (an IRI or a nested ground triple).
    pub subject: GroundTerm,
    /// The predicate (always an IRI).
    pub predicate: NamedNode,
    /// The object (an IRI, literal, or nested ground triple).
    pub object: GroundTerm,
}

/// A quad pattern: a [`TriplePattern`] optionally scoped to a named graph. The
/// `None` graph denotes the default graph (or, in a `GRAPH ?g { ... }` block, an
/// unscoped template position). Used by the UPDATE `DELETE`/`INSERT` templates.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QuadPattern {
    /// The triple pattern (subject/predicate/object).
    pub triple: TriplePattern,
    /// The graph name (IRI or variable), or `None` for the default graph.
    pub graph: Option<NamedNodePattern>,
}
