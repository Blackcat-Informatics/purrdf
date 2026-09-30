// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What the classic document codecs (RDF/XML, TriX, HexTuples) share beyond the
//! statement fold: diagnostics worded in the codec's own name, the checked
//! lookups a serializer makes into a [`SerGraph`], the XML 1.0 escaping of an
//! XML writer, the text of an XML element, and the classic
//! (triple-term-free) term those codecs accumulate before interning.
//!
//! Each codec names itself once, as a [`CodecName`] constant; everything here
//! is stated once and worded through that name, so the codecs cannot drift
//! apart in what they refuse or how they say it.

use std::fmt;
use std::sync::Arc;

use purrdf_core::blank_label::LabelAlphabet;
use purrdf_core::cdt_blank::BlankBinding;
use purrdf_core::sink::TextOut;
use purrdf_core::xml_escape::{self, Context};
use purrdf_iri::langtag::{self, LanguageTagError};
use purrdf_lex::xml::Node;

use super::parse::{FoldNode, FoldRow, RDF_REIFIES, fold_statement_layer};
use super::ser_model::{SerGraph, SerTerm};
use crate::{RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, TermId};

/// A native codec's display name (`"TriX"`), which its diagnostics lead with.
#[derive(Clone, Copy, Debug)]
pub(super) struct CodecName(pub(super) &'static str);

impl CodecName {
    /// A parse refusal (`native-codec-parse`), worded `"<codec>: <detail>"`.
    pub(super) fn parse_err(self, detail: impl fmt::Display) -> RdfDiagnostic {
        RdfDiagnostic::error("native-codec-parse", format!("{}: {detail}", self.0))
    }

    /// A serialize refusal (`native-codec-serialize`), worded
    /// `"<codec>: <detail>"`.
    pub(super) fn serialize_err(self, detail: impl fmt::Display) -> RdfDiagnostic {
        RdfDiagnostic::error("native-codec-serialize", format!("{}: {detail}", self.0))
    }

    /// The serialization term `tid` names.
    ///
    /// # Errors
    ///
    /// A serialize refusal for an id past the graph's terms.
    pub(super) fn term(self, graph: &SerGraph, tid: usize) -> Result<&SerTerm, RdfDiagnostic> {
        graph.terms.get(tid).ok_or_else(|| {
            self.serialize_err(format_args!(
                "term id {tid} is out of range for the serialization graph"
            ))
        })
    }

    /// A term's string value: its IRI, lexical form or blank label.
    ///
    /// # Errors
    ///
    /// A serialize refusal for a term without one.
    pub(super) fn value(self, term: &SerTerm) -> Result<&str, RdfDiagnostic> {
        term.value
            .as_deref()
            .ok_or_else(|| self.serialize_err("term is missing its value"))
    }

    /// Append `value` straight into `out` as lossless XML 1.0 character data
    /// or a double-quoted attribute value, per `context`.
    ///
    /// `push_into` rather than an allocating escape: an XML writer streams a
    /// whole document, and the allocating spelling would allocate once per
    /// term that needs a replacement.
    ///
    /// # Errors
    ///
    /// A serialize refusal for a character XML 1.0 cannot carry.
    pub(super) fn push_xml<W: TextOut + ?Sized>(
        self,
        value: &str,
        context: Context,
        out: &mut W,
    ) -> Result<(), RdfDiagnostic> {
        xml_escape::push_into(value, context, out).map_err(|error| self.serialize_err(error))
    }
}

/// Check `tag` against the concrete syntaxes' `LANGTAG` terminal under the
/// one bounded profile ([`langtag::Profile::ConcreteSyntaxLangtagBounded`])
/// every native codec reads a language tag with, so a tag one syntax admits
/// another cannot refuse; `message` words a refusal for the codec and the
/// tag's position, and is only called on one.
///
/// # Errors
///
/// A diagnostic carrying [`LanguageTagError::diagnostic_code`], so the user
/// learns which production refused.
pub(super) fn check_language_tag(
    tag: &str,
    message: impl FnOnce(&LanguageTagError) -> String,
) -> Result<(), RdfDiagnostic> {
    langtag::parse_with(tag, langtag::Profile::ConcreteSyntaxLangtagBounded)
        .map(drop)
        .map_err(|error| RdfDiagnostic::error(error.diagnostic_code(), message(&error)))
}

/// The concatenated direct text children of `element`: an XML literal's
/// lexical form, verbatim.
pub(super) fn element_text(element: Node<'_, '_>) -> String {
    element
        .children()
        .filter(Node::is_text)
        .filter_map(|node| node.text())
        .collect()
}

/// A term of a classic syntax (TriX, HexTuples), which carries no triple
/// terms, as its parser accumulates it before interning.
#[derive(Clone, Debug)]
pub(super) enum ClassicTerm {
    /// An IRI.
    Iri(String),
    /// A blank node, by its document label.
    Blank(String),
    /// A literal.
    Literal(RdfLiteral),
}

/// One classic statement: `(subject, predicate IRI, object, graph)`.
pub(super) type ClassicRow = (ClassicTerm, String, ClassicTerm, Option<ClassicTerm>);

impl ClassicTerm {
    /// Intern the term, decoding blank labels — a node's own and those a
    /// composite literal embeds, by the same rule so both spellings of one
    /// node agree — from `alphabet`, the alphabet the codec's serializer
    /// writes, so a document the codec wrote re-parses to the very
    /// `(label, scope)` pairs it was written from.
    ///
    /// # Errors
    ///
    /// A composite (`cdt:List` / `cdt:Map`) literal whose lexical form does
    /// not parse refuses the document; see [`purrdf_core::cdt_blank`].
    fn intern(
        &self,
        builder: &mut RdfDatasetBuilder,
        alphabet: LabelAlphabet,
    ) -> Result<TermId, RdfDiagnostic> {
        Ok(match self {
            Self::Iri(iri) => builder.intern_iri(iri),
            Self::Blank(label) => builder.intern_text_blank(label, alphabet),
            Self::Literal(literal) => {
                builder.intern_literal_bound(literal.clone(), BlankBinding::Decoded(alphabet))?
            }
        })
    }
}

/// Intern classic `rows` (blank labels under `alphabet`) and freeze them
/// through the shared statement-layer fold, which folds `rdf:reifies` rows
/// into reifiers exactly as every other parse path does.
///
/// # Errors
///
/// A term's interning refusal, or the fold's or the freeze's.
pub(super) fn freeze_classic_rows(
    rows: Vec<ClassicRow>,
    alphabet: LabelAlphabet,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let mut builder = RdfDatasetBuilder::new();
    let mut fold_rows: Vec<FoldRow> = Vec::with_capacity(rows.len());
    for (subject, predicate, object, graph) in rows {
        let subject = subject.intern(&mut builder, alphabet)?;
        let is_reifies = predicate == RDF_REIFIES;
        let predicate = builder.intern_iri(&predicate);
        let object = FoldNode::Term(object.intern(&mut builder, alphabet)?);
        let graph = graph
            .map(|graph| graph.intern(&mut builder, alphabet))
            .transpose()?;
        fold_rows.push(FoldRow {
            subject,
            is_reifies,
            predicate,
            object,
            graph,
        });
    }
    fold_statement_layer(&mut builder, fold_rows)?;
    builder.freeze()
}
