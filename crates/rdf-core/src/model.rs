// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::RdfLocation;
use crate::ir::term::{RDF_DIR_LANG_STRING, RDF_LANG_STRING, XSD_STRING};

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
    /// The datatype IRI; `None` means the implied default (`rdf:dirLangString`
    /// for a directional language tag, `rdf:langString` for a language tag
    /// without direction, otherwise `xsd:string`), expanded at intern time.
    pub datatype: Option<String>,
    /// The language tag, for language-tagged strings.
    pub language: Option<String>,
    /// The RDF 1.2 base direction, for directional language-tagged strings.
    pub direction: Option<RdfTextDirection>,
}

/// The stable diagnostic code for a literal whose datatype, language and
/// direction do not agree. Language-tag *grammar* failures do not use it: they
/// carry `purrdf_iri::langtag`'s own `langtag-*` code, because that module owns
/// the grammar and its refusal reasons.
pub(crate) const LITERAL_SHAPE_CODE: &str = "rdf-ir-literal-shape";

impl RdfLiteral {
    /// Validate the relationship between an expanded datatype, language and
    /// direction, and the well-formedness of the language tag itself.
    ///
    /// Call after applying any ingress-specific implied datatype rules. A
    /// language tag is judged by the one owner of the grammar,
    /// [`purrdf_iri::langtag`], under
    /// [`Profile::ConcreteSyntaxLangtagBounded`](purrdf_iri::langtag::Profile::ConcreteSyntaxLangtagBounded)
    /// — the same acceptance language every codec reader names, so a tag this
    /// kernel admits is exactly a tag the concrete syntaxes can write and read
    /// back. This still does NOT check datatype lexical validity.
    ///
    /// The judgement is case-insensitive (the `LANGTAG` terminal is
    /// `[a-zA-Z]+('-'[a-zA-Z0-9]+)*`), so it gives the same answer either side of
    /// the intern-time lowercase fold: `en-US` and `en-us` are both accepted.
    ///
    /// # Errors
    /// Refuses missing language tags, datatype/direction mismatches, and any
    /// language tag the grammar does not accept — including the empty tag.
    pub fn validate_components(
        datatype: &str,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
    ) -> Result<(), &'static str> {
        Self::diagnose_components(datatype, language, direction).map_err(|(_code, message)| message)
    }

    /// [`validate_components`](Self::validate_components) with the refusal's
    /// stable diagnostic code attached.
    ///
    /// The single implementation; `validate_components` is the `&'static str`
    /// door onto it and keeps the published signature. Splitting the pair out
    /// here is what lets `crate::ir::validate` report a malformed language tag
    /// under the grammar's OWN code rather than collapsing every literal refusal
    /// into one.
    pub(crate) fn diagnose_components(
        datatype: &str,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
    ) -> Result<(), (&'static str, &'static str)> {
        if let Some(language) = language {
            // The grammar first: a datatype/direction mismatch on a tag that is
            // not a language tag at all would report the downstream symptom.
            if let Err(error) = purrdf_iri::langtag::parse_with(
                language,
                purrdf_iri::langtag::Profile::ConcreteSyntaxLangtagBounded,
            ) {
                return Err((error.diagnostic_code(), error.message()));
            }
            if datatype != Self::language_datatype_iri(direction) {
                return Err((
                    LITERAL_SHAPE_CODE,
                    "literal datatype does not match its language and base direction",
                ));
            }
        } else {
            if direction.is_some() {
                return Err((
                    LITERAL_SHAPE_CODE,
                    "a base direction requires a language tag",
                ));
            }
            if matches!(datatype, RDF_LANG_STRING | RDF_DIR_LANG_STRING) {
                return Err((
                    LITERAL_SHAPE_CODE,
                    "a language-string datatype requires a language tag",
                ));
            }
        }
        Ok(())
    }

    /// The RDF datatype implied by a language tag and its optional base direction.
    #[must_use]
    pub const fn language_datatype_iri(direction: Option<RdfTextDirection>) -> &'static str {
        match direction {
            Some(_) => RDF_DIR_LANG_STRING,
            None => RDF_LANG_STRING,
        }
    }

    /// The expanded datatype used when this owned literal is interned.
    ///
    /// A language tag determines the datatype, regardless of an explicit
    /// datatype field: `rdf:dirLangString` with a base direction, otherwise
    /// `rdf:langString`. Without a language tag, an explicit datatype is
    /// preserved and an absent datatype expands to `xsd:string`.
    /// This borrows the existing IRI or a constant and does not allocate.
    #[must_use]
    pub fn datatype_iri(&self) -> &str {
        match (&self.language, self.direction, &self.datatype) {
            (Some(_), direction, _) => Self::language_datatype_iri(direction),
            (None, _, Some(datatype)) => datatype,
            (None, _, None) => XSD_STRING,
        }
    }

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

    /// Fold this term bottom-up over [`try_fold_nested`](crate::try_fold_nested)'s work
    /// list: `leaf` answers for every term that is not a triple term, `predicate` for a
    /// triple term's predicate IRI, and `triple` combines a triple term's subject,
    /// predicate and object answers — each folded fully, in that order — into its own.
    /// The first error ends the fold and is returned.
    ///
    /// # Errors
    ///
    /// The first error `leaf`, `predicate` or `triple` returns.
    pub fn try_fold<T, E>(
        &self,
        mut leaf: impl FnMut(&Self) -> Result<T, E>,
        mut predicate: impl FnMut(&str) -> Result<T, E>,
        mut triple: impl FnMut(T, T, T) -> Result<T, E>,
    ) -> Result<T, E> {
        /// One node of a term's nesting: a term, or a triple term's predicate IRI.
        #[derive(Clone, Copy)]
        enum Node<'t> {
            Term(&'t RdfTerm),
            Predicate(&'t str),
        }
        crate::try_fold_nested(
            Node::Term(self),
            &mut (),
            |(), node| match node {
                Node::Term(Self::Triple(t)) => Ok(crate::Nested::Triple(
                    Node::Term(&t.subject),
                    Node::Predicate(&t.predicate),
                    Node::Term(&t.object),
                )),
                Node::Term(term) => leaf(term).map(crate::Nested::Leaf),
                Node::Predicate(iri) => predicate(iri).map(crate::Nested::Leaf),
            },
            |(), _, s, p, o| triple(s, p, o),
        )
    }

    /// This term as a dataset-independent [`TermValue`](crate::TermValue), bound
    /// under the kernel's own owned-model ingestion law (the one
    /// `RdfDatasetBuilder::intern_owned_term_scoped` interns by), so a value obtained
    /// here names exactly the term the builder would mint for `self`:
    ///
    /// * the literal datatype is expanded through [`RdfLiteral::datatype_iri`] — a
    ///   language tag implies `rdf:langString`, a tag with a direction
    ///   `rdf:dirLangString`, an absent datatype `xsd:string` — and the language tag
    ///   is lowercased, both per the C0.1 identity key;
    /// * at [`BlankScope::DEFAULT`](crate::BlankScope::DEFAULT) a blank label is
    ///   decoded through [`BlankScope::unqualify_label`](crate::BlankScope::unqualify_label),
    ///   the exact inverse of [`BlankLabelPolicy::Qualified`](crate::BlankLabelPolicy::Qualified),
    ///   so a qualified label round-trips to the `(label, scope)` pair it encodes and
    ///   an ordinary label passes through verbatim; at any other `scope` the label is
    ///   kept verbatim under that scope — the standardize-apart relabelling, which
    ///   must stay injective over the source's own scopes rather than decode them;
    /// * a triple term's predicate IRI becomes an IRI value.
    ///
    /// A triple term is assembled bottom-up over [`try_fold`](Self::try_fold)'s work
    /// list, so a term of any depth is converted without recursion.
    #[must_use]
    pub fn to_term_value(&self, scope: crate::BlankScope) -> crate::TermValue {
        use crate::{BlankScope, TermBox, TermValue};
        let lowered = self.try_fold::<_, core::convert::Infallible>(
            |leaf| {
                Ok(match leaf {
                    Self::Iri(iri) => TermValue::Iri(iri.clone()),
                    Self::BlankNode(label) if scope == BlankScope::DEFAULT => {
                        let (label, scope) = BlankScope::unqualify_label(label);
                        TermValue::Blank {
                            label: label.into_owned(),
                            scope,
                        }
                    }
                    Self::BlankNode(label) => TermValue::Blank {
                        label: label.clone(),
                        scope,
                    },
                    Self::Literal(literal) => TermValue::Literal {
                        lexical_form: literal.lexical_form.clone(),
                        datatype: literal.datatype_iri().to_owned(),
                        language: literal.language.as_deref().map(str::to_lowercase),
                        direction: literal.direction,
                    },
                    Self::Triple(_) => unreachable!("a triple term is folded from its parts"),
                })
            },
            |predicate| Ok(TermValue::Iri(predicate.to_owned())),
            |s, p, o| {
                Ok(TermValue::Triple {
                    s: TermBox::new(s),
                    p: TermBox::new(p),
                    o: TermBox::new(o),
                })
            },
        );
        match lowered {
            Ok(value) => value,
        }
    }
}

/// Renders the term in its canonical form (`<iri>`, `_:label`, a typed/lang literal,
/// or the RDF 1.2 triple-term shorthand `<< … >>`) — the single source of truth is
/// [`crate::turtle::display_term`] (the same rendering [`crate::turtle::emit_term`]
/// produces for an in-alphabet term), so `Display` and the serializer never
/// diverge on emittable terms. `Display` is a diagnostic surface, not document
/// egress: a blank-node label outside the Turtle alphabet still renders here
/// verbatim so a message can name the caller's own label, while `emit_term`
/// escapes it into the target alphabet.
impl core::fmt::Display for RdfTerm {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&crate::turtle::display_term(self))
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

    /// Every tag a concrete syntax could legally write must still be admitted —
    /// the refusal below is only worth having if its neighbours survive.
    #[test]
    fn well_formed_language_tags_are_still_admitted() {
        for tag in [
            "en",
            "en-US",
            "en-us",
            "zh-Hans-CN",
            "zh-hans-cn",
            "de-CH-x-phonebk",
            "i-enochian",
            "x-purrdf-afrikaans",
            "x-gmeow-english",
            "en-fr-jura",
            "fr-be-fbcl",
        ] {
            RdfLiteral::validate_components(RDF_LANG_STRING, Some(tag), None).unwrap_or_else(
                |reason| panic!("{tag:?} is well-formed but was refused: {reason}"),
            );
            // The intern-time lowercase fold must not change the answer: the
            // `LANGTAG` terminal is case-insensitive, so the check is the same
            // either side of it.
            let folded = tag.to_lowercase();
            RdfLiteral::validate_components(RDF_LANG_STRING, Some(folded.as_str()), None)
                .unwrap_or_else(|reason| panic!("{folded:?} refused after the fold: {reason}"));
        }
    }

    /// The hole this check closes: a tag no parser could read must never reach a
    /// serializer, and the refusal must name the grammar rule that refused it.
    #[test]
    fn malformed_language_tags_are_refused_with_the_grammars_own_code() {
        for tag in ["1", "9-9", "123-456", "en-", "-", "en us", "!!!", ""] {
            let (code, message) = RdfLiteral::diagnose_components(RDF_LANG_STRING, Some(tag), None)
                .expect_err("a malformed language tag must not construct");
            assert!(
                code.starts_with("langtag-"),
                "{tag:?} reported {code:?}, not a grammar code"
            );
            assert!(!message.is_empty(), "{tag:?} reported an empty reason");
            // The `&'static str` door onto the same judgement agrees.
            assert_eq!(
                RdfLiteral::validate_components(RDF_LANG_STRING, Some(tag), None),
                Err(message)
            );
        }
    }

    /// A datatype/language/direction disagreement is NOT a grammar failure and
    /// keeps its own code, so the two stay distinguishable downstream.
    #[test]
    fn shape_failures_keep_the_literal_shape_code() {
        for (datatype, language, direction) in [
            (RDF_LANG_STRING, Some("en"), Some(RdfTextDirection::Ltr)),
            (XSD_STRING, None, Some(RdfTextDirection::Rtl)),
            (RDF_LANG_STRING, None, None),
            (RDF_DIR_LANG_STRING, None, None),
        ] {
            let (code, _) = RdfLiteral::diagnose_components(datatype, language, direction)
                .expect_err("mismatched components must not construct");
            assert_eq!(code, LITERAL_SHAPE_CODE);
        }
    }

    /// The datatype expands exactly as the builder would expand it at intern time,
    /// and the language tag is lowercased to the identity key.
    #[test]
    fn to_term_value_expands_datatypes_and_lowercases_the_language() {
        use crate::{BlankScope, TermValue};
        let integer = "http://www.w3.org/2001/XMLSchema#integer";
        let cases = [
            (RdfLiteral::simple("a"), XSD_STRING, None, None),
            (
                RdfLiteral::language_tagged("a", "EN-us"),
                RDF_LANG_STRING,
                Some("en-us"),
                None,
            ),
            (
                RdfLiteral {
                    lexical_form: "a".to_owned(),
                    datatype: None,
                    language: Some("AR".to_owned()),
                    direction: Some(RdfTextDirection::Rtl),
                },
                RDF_DIR_LANG_STRING,
                Some("ar"),
                Some(RdfTextDirection::Rtl),
            ),
            (RdfLiteral::typed("1", integer), integer, None, None),
            // An explicit datatype alongside a language tag: the tag wins, as at
            // intern time.
            (
                RdfLiteral {
                    lexical_form: "a".to_owned(),
                    datatype: Some(integer.to_owned()),
                    language: Some("en".to_owned()),
                    direction: None,
                },
                RDF_LANG_STRING,
                Some("en"),
                None,
            ),
        ];
        for (literal, datatype, language, direction) in cases {
            let value = RdfTerm::literal(literal.clone()).to_term_value(BlankScope::DEFAULT);
            assert_eq!(
                value,
                TermValue::Literal {
                    lexical_form: literal.lexical_form.clone(),
                    datatype: datatype.to_owned(),
                    language: language.map(str::to_owned),
                    direction,
                },
                "{literal:?}"
            );
        }
    }

    /// At the default scope a qualified label is decoded — the exact inverse of the
    /// qualified bridge — and an ordinary label passes through; at any other scope
    /// the label is kept verbatim under that scope.
    #[test]
    fn to_term_value_decodes_qualified_labels_at_the_default_scope_only() {
        use crate::{BlankLabelPolicy, BlankScope, DatatypePolicy, TermBox, TermValue};
        assert_eq!(
            RdfTerm::blank_node("purrdfesc2_b").to_term_value(BlankScope::DEFAULT),
            TermValue::Blank {
                label: "b".to_owned(),
                scope: BlankScope(2),
            }
        );
        assert_eq!(
            RdfTerm::blank_node("b").to_term_value(BlankScope::DEFAULT),
            TermValue::blank("b")
        );
        assert_eq!(
            RdfTerm::blank_node("purrdfesc2_b").to_term_value(BlankScope(5)),
            TermValue::Blank {
                label: "purrdfesc2_b".to_owned(),
                scope: BlankScope(5),
            }
        );
        assert_eq!(
            RdfTerm::iri("http://example.org/i").to_term_value(BlankScope(5)),
            TermValue::iri("http://example.org/i")
        );

        // The round trip through the qualified bridge is the identity, nested triple
        // terms and scoped blanks included.
        let value = TermValue::Triple {
            s: TermBox::new(TermValue::Triple {
                s: TermBox::new(TermValue::Blank {
                    label: "a.b".to_owned(),
                    scope: BlankScope(7),
                }),
                p: TermBox::new(TermValue::iri("http://example.org/p")),
                o: TermBox::new(TermValue::lang_literal("o", "en")),
            }),
            p: TermBox::new(TermValue::iri("http://example.org/q")),
            o: TermBox::new(TermValue::simple_literal("x")),
        };
        for policy in [
            DatatypePolicy::Keep,
            DatatypePolicy::CollapseString,
            DatatypePolicy::CollapseStringAndLanguage,
        ] {
            let owned = value
                .to_rdf_term(policy, BlankLabelPolicy::Qualified)
                .expect("IRI predicates");
            assert_eq!(
                owned.to_term_value(BlankScope::DEFAULT),
                value,
                "{policy:?}"
            );
        }
    }

    /// A nested owned triple term lowers bottom-up, its predicate IRIs as IRI values.
    #[test]
    fn to_term_value_lowers_a_nested_triple_term() {
        use crate::{BlankScope, TermBox, TermValue};
        let owned = RdfTerm::triple(RdfTriple::new(
            RdfTerm::triple(RdfTriple::new(
                RdfTerm::iri("http://example.org/s"),
                "http://example.org/p",
                RdfTerm::literal(RdfLiteral::simple("o")),
            )),
            "http://example.org/q",
            RdfTerm::blank_node("b"),
        ));
        assert_eq!(
            owned.to_term_value(BlankScope::DEFAULT),
            TermValue::Triple {
                s: TermBox::new(TermValue::Triple {
                    s: TermBox::new(TermValue::iri("http://example.org/s")),
                    p: TermBox::new(TermValue::iri("http://example.org/p")),
                    o: TermBox::new(TermValue::simple_literal("o")),
                }),
                p: TermBox::new(TermValue::iri("http://example.org/q")),
                o: TermBox::new(TermValue::blank("b")),
            }
        );
    }

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
