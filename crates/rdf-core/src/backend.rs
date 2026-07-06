// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Narrow purrdf backend traits (P2d).
//!
//! These are the dependency-inversion seams that remain after `DatasetView`
//! the oxigraph crate ring-fence, and `DatasetMut`: term
//! interning, parser ingress, SPARQL execution, and serializer egress. They live in
//! `purrdf-core` so consumers can depend on the contract without depending on
//! oxigraph. Concrete oxigraph adapters live in the sibling `purrdf` crate.

use std::io::Write;
use std::sync::Arc;

use purrdf_events::RdfEventSink;

use crate::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, TermId, TermValue,
};

/// Term interning seam: dataset-independent values enter a concrete term table.
///
/// `TermId` remains local to the implementer's dataset/builder (C0.8). Callers
/// that cross dataset boundaries must carry [`TermValue`]s and re-intern them
/// here, rather than persisting ids.
pub trait TermFactory {
    /// Intern a complete dataset-independent term value.
    fn intern_value(&mut self, value: &TermValue) -> TermId;

    /// Intern an IRI term.
    fn intern_iri_value(&mut self, iri: &str) -> TermId;

    /// Intern a blank node identified by `(label, scope)`.
    fn intern_blank_value(&mut self, label: &str, scope: BlankScope) -> TermId;

    /// Intern a literal term.
    fn intern_literal_value(&mut self, literal: RdfLiteral) -> TermId;

    /// Intern an RDF 1.2 quoted-triple term from already-interned component ids.
    fn intern_triple_value(&mut self, s: TermId, p: TermId, o: TermId) -> TermId;
}

impl TermFactory for RdfDatasetBuilder {
    fn intern_value(&mut self, value: &TermValue) -> TermId {
        match value {
            TermValue::Iri(iri) => self.intern_iri_value(iri),
            TermValue::Blank { label, scope } => self.intern_blank_value(label, *scope),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => self.intern_literal_value(RdfLiteral {
                lexical_form: lexical_form.clone(),
                datatype: Some(datatype.clone()),
                language: language.clone(),
                direction: *direction,
            }),
            TermValue::Triple { s, p, o } => {
                let s = self.intern_value(s);
                let p = self.intern_value(p);
                let o = self.intern_value(o);
                self.intern_triple_value(s, p, o)
            }
        }
    }

    fn intern_iri_value(&mut self, iri: &str) -> TermId {
        self.intern_iri(iri)
    }

    fn intern_blank_value(&mut self, label: &str, scope: BlankScope) -> TermId {
        self.intern_blank(label, scope)
    }

    fn intern_literal_value(&mut self, literal: RdfLiteral) -> TermId {
        self.intern_literal(literal)
    }

    fn intern_triple_value(&mut self, s: TermId, p: TermId, o: TermId) -> TermId {
        self.intern_triple(s, p, o)
    }
}

/// RDF parser request. Formats are named by media type or local format id at the
/// contract boundary so the core trait does not leak an oxigraph enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RdfParseRequest<'a> {
    /// The raw RDF bytes to parse.
    pub bytes: &'a [u8],
    /// The format's media type (or local format id).
    pub media_type: &'a str,
    /// The base IRI for resolving relative IRIs, when any.
    pub base_iri: Option<&'a str>,
    /// A display name for the source (e.g. a file path) used in diagnostics.
    pub source_name: Option<&'a str>,
}

/// Parser ingress seam: drive RDF bytes into any event sink.
pub trait RdfParserBackend {
    /// Parses the request's bytes and delivers the resulting events into
    /// `sink`.
    ///
    /// # Errors
    ///
    /// Returns the first fatal parse diagnostic; events delivered before the
    /// failure may already have reached the sink.
    fn parse_into<S: RdfEventSink + ?Sized>(
        &self,
        request: RdfParseRequest<'_>,
        sink: &mut S,
    ) -> Result<(), RdfDiagnostic>;
}

/// SPARQL operation request.
///
/// `substitutions` carries variable **pre-bindings** (purrdf S5,  GAP-A):
/// each `(name, value)` pre-binds the query variable `name` to `value` before
/// evaluation, as if the `WHERE` had been joined with a single-row
/// `VALUES { ?name value }`. This is the native replacement for oxigraph's
/// `PreparedSparqlQuery::substitute_variable`, used by SHACL-AF to inject the focus
/// node as `$this`. A slice (rather than a `Vec`) keeps the request `Copy` and
/// borrow-only; the empty slice (`&[]`) is "no pre-binding". `value` is a
/// [`TermValue`], so a blank-node focus node is representable (unlike a `VALUES`
/// cell), and the substitution propagates into `OPTIONAL`/`MINUS`/`EXISTS`/
/// sub-queries while keeping the variable projectable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparqlRequest<'a> {
    /// The SPARQL query or update text.
    pub query: &'a str,
    /// The base IRI for resolving relative IRIs in the query, when any.
    pub base_iri: Option<&'a str>,
    /// Variable pre-bindings (`(name, value)` pairs); `&[]` means none.
    pub substitutions: &'a [(String, TermValue)],
}

/// Materialized SPARQL result model independent of any concrete query engine.
#[derive(Debug, Clone)]
pub enum SparqlResult {
    /// A SELECT solution sequence.
    Solutions {
        /// The projected variable names, in projection order.
        variables: Vec<String>,
        /// One row per solution; each cell is the binding for the variable at
        /// the same position in `variables` (`None` = unbound).
        rows: Vec<Vec<Option<TermValue>>>,
        /// Auxiliary quads invented during evaluation by value-constructing builtins
        /// (`purrdf:listSlice`/`purrdf:listConcat` mint fresh `rdf:List` cells). Empty for
        /// an ordinary SELECT. Carried so an in-process consumer can dereference a list
        /// head returned in a solution cell; W3C tabular result formats ignore it.
        aux: Arc<RdfDataset>,
    },
    /// A CONSTRUCT/DESCRIBE result, materialized as a frozen dataset.
    Graph(Arc<RdfDataset>),
    /// An ASK result.
    Boolean(bool),
}

/// SPARQL query/update seam. The dataset type is associated so an oxigraph-backed
/// engine can operate on its store while a future native engine can operate on the
/// IR/native query store.
pub trait SparqlEngine {
    /// The dataset representation this engine evaluates against.
    type Dataset;

    /// Evaluates a SPARQL query against the dataset.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the query fails to parse or evaluate.
    fn query(
        &self,
        dataset: &Self::Dataset,
        request: SparqlRequest<'_>,
    ) -> Result<SparqlResult, RdfDiagnostic>;

    /// Applies a SPARQL update to the dataset in place.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the update fails to parse or apply.
    fn update(
        &self,
        dataset: &mut Self::Dataset,
        request: SparqlRequest<'_>,
    ) -> Result<(), RdfDiagnostic>;
}

/// Which graph(s) a serializer should emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializeGraph<'a> {
    /// The whole dataset (default graph plus every named graph).
    Dataset,
    /// The default graph only.
    DefaultGraph,
    /// One named graph, by its graph-name term value.
    Named(&'a TermValue),
}

/// RDF serializer request. Formats are media types/local ids for the same reason
/// as [`RdfParseRequest`]: the core trait must not expose an oxigraph enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RdfSerializeRequest<'a> {
    /// The output format's media type (or local format id).
    pub media_type: &'a str,
    /// Which graph(s) to emit.
    pub graph: SerializeGraph<'a>,
    /// The base IRI the serializer may abbreviate against, when any.
    pub base_iri: Option<&'a str>,
}

/// Serializer egress seam over the frozen IR.
pub trait RdfSerializer {
    /// Serializes the requested graph(s) of a frozen dataset to `output`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the format is unsupported, the dataset cannot
    /// be represented in it, or writing to `output` fails.
    fn serialize<W: Write>(
        &self,
        dataset: &RdfDataset,
        request: RdfSerializeRequest<'_>,
        output: W,
    ) -> Result<(), RdfDiagnostic>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RdfTextDirection;

    fn iri(value: &str) -> TermValue {
        TermValue::Iri(value.to_owned())
    }

    #[test]
    fn term_factory_interns_all_value_variants() {
        let s = iri("https://example.org/s");
        let p = iri("https://example.org/p");
        let o = iri("https://example.org/o");
        let blank = TermValue::Blank {
            label: "b0".to_owned(),
            scope: BlankScope(7),
        };
        let typed = TermValue::Literal {
            lexical_form: "1".to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
            language: None,
            direction: None,
        };
        let directional = TermValue::Literal {
            lexical_form: "مرحبا".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        };
        let triple = TermValue::Triple {
            s: Box::new(s.clone()),
            p: Box::new(p.clone()),
            o: Box::new(o),
        };

        let mut builder = RdfDatasetBuilder::new();
        let s_id = builder.intern_value(&s);
        let p_id = builder.intern_value(&p);
        let triple_id = builder.intern_value(&triple);
        let blank_id = builder.intern_value(&blank);
        let typed_id = builder.intern_value(&typed);
        let directional_id = builder.intern_value(&directional);
        builder.push_quad(s_id, p_id, triple_id, Some(blank_id));
        builder.push_quad(s_id, p_id, typed_id, None);
        builder.push_quad(s_id, p_id, directional_id, None);

        let dataset = builder.freeze().expect("term factory output is valid");
        assert_eq!(dataset.term_id_by_value(&s), Some(s_id));
        assert_eq!(dataset.term_id_by_value(&p), Some(p_id));
        assert_eq!(dataset.term_id_by_value(&triple), Some(triple_id));
        assert_eq!(dataset.term_id_by_value(&blank), Some(blank_id));
        assert_eq!(dataset.term_id_by_value(&typed), Some(typed_id));
        assert_eq!(dataset.term_id_by_value(&directional), Some(directional_id));
    }
}
