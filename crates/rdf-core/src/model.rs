// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::RdfLocation;

/// RDF term category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RdfTermKind {
    /// An IRI.
    Iri,
    /// A blank node.
    BlankNode,
    /// A literal.
    Literal,
    /// An RDF 1.2 triple term (quoted triple).
    Triple,
}

/// RDF 1.2 base direction for directional language-tagged literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RdfTextDirection {
    /// Left-to-right base direction (`ltr`).
    Ltr,
    /// Right-to-left base direction (`rtl`).
    Rtl,
}

impl RdfTextDirection {
    /// The lowercase direction token (`"ltr"` or `"rtl"`) as it appears in
    /// concrete syntaxes.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}

/// An RDF literal, including RDF 1.2 language direction when available.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RdfLiteral {
    /// The lexical form, byte-for-byte as authored.
    pub lexical_form: String,
    /// The datatype IRI; `None` means the implied default (`rdf:langString`
    /// when a language tag is present, otherwise `xsd:string`), expanded at
    /// intern time.
    pub datatype: Option<String>,
    /// The language tag, for language-tagged strings.
    pub language: Option<String>,
    /// The RDF 1.2 base direction, for directional language-tagged strings.
    pub direction: Option<RdfTextDirection>,
}

impl RdfLiteral {
    /// A simple literal: bare lexical form with no datatype, language, or
    /// direction.
    pub fn simple(lexical_form: impl Into<String>) -> Self {
        Self {
            lexical_form: lexical_form.into(),
            datatype: None,
            language: None,
            direction: None,
        }
    }

    /// A datatyped literal from its lexical form and datatype IRI.
    pub fn typed(lexical_form: impl Into<String>, datatype: impl Into<String>) -> Self {
        Self {
            lexical_form: lexical_form.into(),
            datatype: Some(datatype.into()),
            language: None,
            direction: None,
        }
    }

    /// A language-tagged string from its lexical form and language tag, with
    /// no base direction.
    pub fn language_tagged(lexical_form: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            lexical_form: lexical_form.into(),
            datatype: None,
            language: Some(language.into()),
            direction: None,
        }
    }
}

/// Owned RDF 1.2 term.
///
/// Deliberately exhaustive (NOT `#[non_exhaustive]`): the RDF data model fixes the
/// set of term kinds (IRI, blank node, literal, triple term), so consumers SHOULD
/// match all four — there is no future variant to guard against.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RdfTerm {
    /// An IRI, by its full string.
    Iri(String),
    /// A blank node, by its label (without the `_:` prefix).
    BlankNode(String),
    /// A literal.
    Literal(RdfLiteral),
    /// An RDF 1.2 triple term (quoted triple).
    Triple(Box<RdfTriple>),
}

impl RdfTerm {
    /// An IRI term from its full string.
    #[must_use]
    pub fn iri(value: impl Into<String>) -> Self {
        Self::Iri(value.into())
    }

    /// A blank-node term from its label (without the `_:` prefix).
    #[must_use]
    pub fn blank_node(value: impl Into<String>) -> Self {
        Self::BlankNode(value.into())
    }

    /// A literal term.
    #[must_use]
    pub fn literal(literal: RdfLiteral) -> Self {
        Self::Literal(literal)
    }

    /// An RDF 1.2 triple term (quoted triple) from an owned triple.
    #[must_use]
    pub fn triple(triple: RdfTriple) -> Self {
        Self::Triple(Box::new(triple))
    }

    /// This term's category.
    #[must_use]
    pub fn kind(&self) -> RdfTermKind {
        match self {
            Self::Iri(_) => RdfTermKind::Iri,
            Self::BlankNode(_) => RdfTermKind::BlankNode,
            Self::Literal(_) => RdfTermKind::Literal,
            Self::Triple(_) => RdfTermKind::Triple,
        }
    }
}

/// Renders the term in its canonical form (`<iri>`, `_:label`, a typed/lang literal,
/// or the RDF 1.2 triple-term shorthand `<< … >>`) — the single source of truth is
/// [`crate::turtle::emit_term`], so `Display` and the serializer never diverge.
impl core::fmt::Display for RdfTerm {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&crate::turtle::emit_term(self))
    }
}

/// Owned RDF 1.2 triple. The model keeps triple-term subjects representable;
/// downstream adapters decide whether a target store can encode them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RdfTriple {
    /// The subject term (may itself be a triple term).
    pub subject: RdfTerm,
    /// The predicate IRI.
    pub predicate: String,
    /// The object term.
    pub object: RdfTerm,
    /// The source location the triple was parsed from, when known.
    pub location: Option<RdfLocation>,
}

impl RdfTriple {
    /// A triple from its subject, predicate IRI, and object, with no location.
    pub fn new(subject: RdfTerm, predicate: impl Into<String>, object: RdfTerm) -> Self {
        Self {
            subject,
            predicate: predicate.into(),
            object,
            location: None,
        }
    }

    /// Attaches a source location; an empty location is dropped rather than
    /// stored.
    #[must_use]
    pub fn with_location(mut self, location: RdfLocation) -> Self {
        if !location.is_empty() {
            self.location = Some(location);
        }
        self
    }
}

/// Owned RDF 1.2 quad with optional adapter/source context.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RdfQuad {
    /// The subject term (may itself be a triple term).
    pub subject: RdfTerm,
    /// The predicate IRI.
    pub predicate: String,
    /// The object term.
    pub object: RdfTerm,
    /// The named graph the quad belongs to (`None` = default graph).
    pub graph_name: Option<RdfTerm>,
    /// The source location the quad was parsed from, when known.
    pub location: Option<RdfLocation>,
}

impl RdfQuad {
    /// A default-graph quad from its subject, predicate IRI, and object, with
    /// no location.
    pub fn new(subject: RdfTerm, predicate: impl Into<String>, object: RdfTerm) -> Self {
        Self {
            subject,
            predicate: predicate.into(),
            object,
            graph_name: None,
            location: None,
        }
    }

    /// Places the quad in a named graph.
    #[must_use]
    pub fn in_graph(mut self, graph_name: RdfTerm) -> Self {
        self.graph_name = Some(graph_name);
        self
    }

    /// Attaches a source location; an empty location is dropped rather than
    /// stored.
    #[must_use]
    pub fn with_location(mut self, location: RdfLocation) -> Self {
        if !location.is_empty() {
            self.location = Some(location);
        }
        self
    }
}

/// RDF 1.2 reifier binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RdfReifier {
    /// The reifier term (the RDF 1.2 `~` handle naming a triple occurrence).
    pub reifier: RdfTerm,
    /// The reified statement (the triple the reifier binds).
    pub statement: RdfTriple,
    /// The named graph the reifier declaration was asserted in (`None` = default
    /// graph). A reifier declared inside a TriG/N-Quads `GRAPH g { … }` block carries
    /// that graph so `GRAPH ?g { << … >> … }` binds `?g` to it.
    pub graph: Option<RdfTerm>,
    /// The source location the reifier binding was parsed from, when known.
    pub location: Option<RdfLocation>,
}

impl RdfReifier {
    /// A reifier binding in the default graph, with no location.
    pub fn new(reifier: RdfTerm, statement: RdfTriple) -> Self {
        Self {
            reifier,
            statement,
            graph: None,
            location: None,
        }
    }

    /// A reifier binding asserted in a specific named graph (`None` = default graph).
    #[must_use]
    pub fn in_graph(mut self, graph: Option<RdfTerm>) -> Self {
        self.graph = graph;
        self
    }

    /// Attaches a source location; an empty location is dropped rather than
    /// stored.
    #[must_use]
    pub fn with_location(mut self, location: RdfLocation) -> Self {
        if !location.is_empty() {
            self.location = Some(location);
        }
        self
    }
}

/// RDF 1.2 statement annotation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RdfAnnotation {
    /// The reifier term the annotation is asserted about.
    pub reifier: RdfTerm,
    /// The annotation's predicate IRI.
    pub predicate: String,
    /// The annotation's object term.
    pub object: RdfTerm,
    /// The named graph the annotation was asserted in (`None` = default graph); see
    /// [`RdfReifier::graph`].
    pub graph: Option<RdfTerm>,
    /// The source location the annotation was parsed from, when known.
    pub location: Option<RdfLocation>,
}

impl RdfAnnotation {
    /// An annotation on a reifier in the default graph, with no location.
    pub fn new(reifier: RdfTerm, predicate: impl Into<String>, object: RdfTerm) -> Self {
        Self {
            reifier,
            predicate: predicate.into(),
            object,
            graph: None,
            location: None,
        }
    }

    /// An annotation asserted in a specific named graph (`None` = default graph).
    #[must_use]
    pub fn in_graph(mut self, graph: Option<RdfTerm>) -> Self {
        self.graph = graph;
        self
    }

    /// Attaches a source location; an empty location is dropped rather than
    /// stored.
    #[must_use]
    pub fn with_location(mut self, location: RdfLocation) -> Self {
        if !location.is_empty() {
            self.location = Some(location);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_for_rdfterm_matches_canonical_emit() {
        assert_eq!(
            RdfTerm::iri("https://example.org/s").to_string(),
            "<https://example.org/s>"
        );
        assert_eq!(RdfTerm::blank_node("b0").to_string(), "_:b0");
        // `Display` MUST delegate to the single-source-of-truth serializer for ALL
        // four RDF term kinds (IRI, blank node, literal, triple term).
        for t in [
            RdfTerm::iri("https://example.org/x"),
            RdfTerm::blank_node("b1"),
            RdfTerm::literal(RdfLiteral::simple("hello")),
            RdfTerm::triple(RdfTriple::new(
                RdfTerm::iri("https://example.org/s"),
                "https://example.org/p",
                RdfTerm::iri("https://example.org/o"),
            )),
        ] {
            assert_eq!(t.to_string(), crate::turtle::emit_term(&t));
        }
    }
}
