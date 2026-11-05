// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RDF term conversion and materialization after the shared expanded node map.

use super::carrier::{Document, Literal, NamedGraph, Node, Part, Term, Triple, Value};
use super::{
    RDF_FIRST, RDF_JSON, RDF_NIL, RDF_REIFIES, RDF_REST, RDF_TYPE, RdfDataset, RdfDiagnostic,
    RdfQuad, RdfTerm, XSD_STRING, decode, validated_iri_term as native_iri_term,
};
use crate::native_codecs::syntax::check_language_tag;
use crate::{RdfLiteral, RdfTextDirection, RdfTriple};
use purrdf_core::collections::{ListVocab, build_rdf_list};
use purrdf_lex::json::{self, Exponent, Number, Value as JsonValue};
use purrdf_xsd::datatype::{XSD_BOOLEAN, XSD_DOUBLE, XSD_INTEGER};
use std::collections::BTreeSet;
use std::sync::Arc;

pub(super) fn expanded_to_dataset(document: JsonValue) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    carrier_to_dataset(expanded_to_carrier(document)?)
}

pub(super) fn expanded_to_carrier(document: JsonValue) -> Result<Document, RdfDiagnostic> {
    let flattened = super::node_map::flatten(
        &document,
        super::JsonLdFlattenLimits::DEFAULT,
        super::node_map::Issuance::Rdf,
    )?;
    if flattened.semantic_index {
        return Err(decode(
            "data-bearing @index metadata has no RDF dataset representation",
        ));
    }
    drop(document);
    let mut document = Document {
        default_nodes: Vec::new(),
        named_graphs: Vec::new(),
    };
    for mut entry in into_values(flattened.value) {
        let object = entry
            .as_object_mut()
            .ok_or_else(|| decode("flattened graph entry must be an object"))?;
        if let Some(contents) = object.remove("@graph") {
            let id = object
                .get("@id")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| decode("flattened graph requires an identifier"))?
                .to_owned();
            let nodes = into_values(contents)
                .into_iter()
                .map(node)
                .collect::<Result<_, _>>()?;
            document.named_graphs.push(NamedGraph { id, nodes });
        }
        let node = node(entry)?;
        if !node.types.is_empty() || !node.properties.is_empty() {
            document.default_nodes.push(node);
        }
    }
    Ok(document)
}

fn carrier_to_dataset(mut document: Document) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let mut lowerer = Lowerer::new(&document);
    for node in core::mem::take(&mut document.default_nodes) {
        lowerer.lower_node(&node, None)?;
    }
    let mut declared_graphs = Vec::new();
    for mut graph in core::mem::take(&mut document.named_graphs) {
        let graph_name = id_term(&graph.id)?;
        for node in core::mem::take(&mut graph.nodes) {
            lowerer.lower_node(&node, Some(&graph_name))?;
        }
        declared_graphs.push(graph_name);
    }
    crate::native_quads::dataset_from_quads_declaring(&lowerer.quads, &declared_graphs)
}

fn node(mut raw: JsonValue) -> Result<Node, RdfDiagnostic> {
    let object = raw
        .as_object_mut()
        .ok_or_else(|| decode("flattened node must be an object"))?;
    let id = object
        .get("@id")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| decode("flattened node requires an identifier"))?
        .to_owned();
    let mut node = Node::new(id);
    for (key, raw) in std::mem::take(object).into_members() {
        match key.as_str() {
            "@id" => {}
            "@index" => {
                return Err(decode(
                    "data-bearing @index metadata has no RDF dataset representation",
                ));
            }
            "@type" => {
                for rdf_type in into_values(raw) {
                    node.types.push(
                        rdf_type
                            .as_str()
                            .ok_or_else(|| decode("expanded @type must contain IRIs"))?
                            .to_owned(),
                    );
                }
            }
            keyword if super::context::is_control(keyword) => {
                return Err(decode(format!(
                    "unsupported flattened node keyword `{keyword}`"
                )));
            }
            _ => {
                let values = into_values(raw)
                    .into_iter()
                    .map(value)
                    .collect::<Result<_, _>>()?;
                node.properties.insert(key, values);
            }
        }
    }
    node.sort_values();
    Ok(node)
}

fn value(mut raw: JsonValue) -> Result<Value, RdfDiagnostic> {
    let object = raw
        .as_object_mut()
        .ok_or_else(|| decode("flattened property value must be an object"))?;
    if object.contains_key("@index") {
        return Err(decode(
            "data-bearing @index metadata has no RDF dataset representation",
        ));
    }
    let annotations = object.remove("@annotation");
    let term = if let Some(list) = object.remove("@list") {
        Term::List(
            into_values(list)
                .into_iter()
                .map(value)
                .collect::<Result<_, _>>()?,
        )
    } else if object.contains_key("@value") {
        Term::Literal(literal(object)?)
    } else if let Some(mut triple) = object.remove("@triple") {
        let triple = triple
            .as_object_mut()
            .ok_or_else(|| decode("expanded @triple must be an object"))?;
        let subject = value(
            triple
                .remove("@subject")
                .ok_or_else(|| decode("triple missing subject"))?,
        )?;
        let object = value(
            triple
                .remove("@object")
                .ok_or_else(|| decode("triple missing object"))?,
        )?;
        if !subject.annotations.is_empty() || !object.annotations.is_empty() {
            return Err(decode("triple components cannot carry annotations"));
        }
        if matches!(subject.term, Term::Literal(_) | Term::List(_)) {
            return Err(decode(
                "RDF triple-term subject must be an identifier or triple",
            ));
        }
        let predicate = triple
            .get("@predicate")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| decode("expanded triple predicate must be an IRI"))?
            .to_owned();
        Term::Triple(Box::new(Triple {
            subject: Box::new(subject.into_term()),
            predicate,
            object: Box::new(object.into_term()),
        }))
    } else {
        Term::Id(
            object
                .get("@id")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| decode("flattened reference requires an identifier"))?
                .to_owned(),
        )
    };
    let mut value = Value::plain(term);
    if let Some(annotations) = annotations {
        for annotation in into_values(annotations) {
            value.annotations.push(node(annotation)?);
        }
    }
    Ok(value)
}

fn into_values(mut value: JsonValue) -> Vec<JsonValue> {
    match &mut value {
        JsonValue::Array(values) => std::mem::take(values),
        JsonValue::Null => Vec::new(),
        _ => vec![value],
    }
}
fn literal(object: &json::Object) -> Result<Literal, RdfDiagnostic> {
    // The sole caller selected this branch with contains_key on the same map.
    let value = object.get("@value").expect("expanded value object");
    let explicit = object.get("@type").and_then(JsonValue::as_str);
    let (lexical, inferred) = match (value, explicit) {
        (_, Some("@json")) => (rdf_json_lexical(value)?, None),
        (JsonValue::String(value), _) => (value.clone(), None),
        (JsonValue::Bool(value), _) => (value.to_string(), Some(XSD_BOOLEAN)),
        (JsonValue::Number(number), _) => {
            // API §8.2 steps 4–5 classify the VALUE, independent of its
            // spelling. Reuse the one exact decimal reading; integral values
            // below 10^21 never round through binary floating point.
            let decimal = number.decimal();
            let zeros = match decimal.exponent() {
                Exponent::Small(exponent) if *exponent >= 0 => {
                    usize::try_from(*exponent).ok().filter(|zeros| {
                        decimal
                            .coefficient()
                            .len()
                            .checked_add(*zeros)
                            .is_some_and(|digits| digits <= 21)
                    })
                }
                _ => None,
            };
            if let Some(zeros) = zeros.filter(|_| explicit != Some(XSD_DOUBLE)) {
                let mut lexical = String::new();
                if decimal.is_negative() {
                    lexical.push('-');
                }
                lexical.push_str(decimal.coefficient());
                // The branch established a nonnegative exponent and at most
                // 21 output digits, so this extends by at most 20 zeros.
                lexical.extend(std::iter::repeat_n('0', zeros));
                (lexical, Some(XSD_INTEGER))
            } else {
                (canonical_json_double(number)?, Some(XSD_DOUBLE))
            }
        }
        _ => {
            return Err(decode(
                "literal @value must be a scalar unless @type is @json",
            ));
        }
    };
    let datatype = match explicit {
        Some("@json") => Some(RDF_JSON.to_owned()),
        Some(datatype) if datatype != XSD_STRING => Some(datatype.to_owned()),
        Some(_) => None,
        None => inferred.map(str::to_owned),
    };
    Ok(Literal {
        lexical,
        datatype,
        language: object
            .get("@language")
            .and_then(JsonValue::as_str)
            .map(str::to_owned),
        direction: object
            .get("@direction")
            .and_then(JsonValue::as_str)
            .map(str::to_owned),
    })
}

/// The `rdf:JSON` lexical form of `value`: compact JSON, members ordered by name, every
/// number in its IEEE 754 double precision spelling.
fn rdf_json_lexical(value: &JsonValue) -> Result<String, RdfDiagnostic> {
    let mut value = crate::json_number::binary64(value)
        .map_err(|source| decode(format!("encode rdf:JSON value: {source}")))?;
    value.sort_keys();
    Ok(json::write_compact(&value))
}

fn canonical_json_double(value: &Number) -> Result<String, RdfDiagnostic> {
    Some(crate::json_number::read_json(|| value.as_f64()))
        .filter(|value| value.is_finite())
        .map(purrdf_xsd::numeric::canonical_double)
        .ok_or_else(|| {
            decode(format!(
                "JSON number `{value}` is outside the xsd:double value space"
            ))
        })
}

struct Lowerer {
    quads: Vec<RdfQuad>,
    reserved_blank_nodes: BTreeSet<String>,
    next_list: u64,
}

impl Lowerer {
    fn new(document: &Document) -> Self {
        let mut reserved_blank_nodes = BTreeSet::new();
        for graph in &document.named_graphs {
            reserve_blank_id(&graph.id, &mut reserved_blank_nodes);
        }
        for node in document.default_nodes.iter().chain(
            document
                .named_graphs
                .iter()
                .flat_map(|graph| graph.nodes.iter()),
        ) {
            reserve_node_blank_ids(node, &mut reserved_blank_nodes);
        }
        Self {
            quads: Vec::new(),
            reserved_blank_nodes,
            next_list: 0,
        }
    }

    fn fresh_list_node(&mut self) -> RdfTerm {
        loop {
            let label = format!("jsonld_list_{}", self.next_list);
            self.next_list += 1;
            if self.reserved_blank_nodes.insert(label.clone()) {
                return RdfTerm::blank_node(label);
            }
        }
    }

    fn lower_node(&mut self, node: &Node, graph: Option<&RdfTerm>) -> Result<(), RdfDiagnostic> {
        let subject = id_term(&node.id)?;
        for rdf_type in &node.types {
            self.push(
                subject.clone(),
                RDF_TYPE,
                validated_iri_term(rdf_type)?,
                graph,
            );
        }
        for (predicate, values) in &node.properties {
            validated_iri_term(predicate)?;
            for value in values {
                let object = self.lower_term(&value.term, graph)?;
                self.push(subject.clone(), predicate, object.clone(), graph);
                self.lower_annotations(&subject, predicate, &object, &value.annotations, graph)?;
            }
        }
        Ok(())
    }

    fn lower_annotations(
        &mut self,
        subject: &RdfTerm,
        predicate: &str,
        object: &RdfTerm,
        annotations: &[Node],
        graph: Option<&RdfTerm>,
    ) -> Result<(), RdfDiagnostic> {
        for annotation in annotations {
            let reifier = id_term(&annotation.id)?;
            let triple =
                RdfTerm::triple(RdfTriple::new(subject.clone(), predicate, object.clone()));
            self.push(reifier.clone(), RDF_REIFIES, triple, graph);
            for rdf_type in &annotation.types {
                self.push(
                    reifier.clone(),
                    RDF_TYPE,
                    validated_iri_term(rdf_type)?,
                    graph,
                );
            }
            for (ann_predicate, values) in &annotation.properties {
                validated_iri_term(ann_predicate)?;
                for value in values {
                    let ann_object = self.lower_term(&value.term, graph)?;
                    self.push(reifier.clone(), ann_predicate, ann_object.clone(), graph);
                    self.lower_annotations(
                        &reifier,
                        ann_predicate,
                        &ann_object,
                        &value.annotations,
                        graph,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn lower_term(
        &mut self,
        term: &Term,
        graph: Option<&RdfTerm>,
    ) -> Result<RdfTerm, RdfDiagnostic> {
        match term {
            Term::Id(id) => id_term(id),
            Term::Literal(literal) => lower_literal(literal),
            Term::Triple(triple) => {
                let subject = self.lower_term(&triple.subject, graph)?;
                if matches!(subject, RdfTerm::Literal(_)) {
                    return Err(decode("RDF triple-term subject cannot be a literal"));
                }
                validated_iri_term(&triple.predicate)?;
                let object = self.lower_term(&triple.object, graph)?;
                Ok(RdfTerm::triple(RdfTriple::new(
                    subject,
                    &triple.predicate,
                    object,
                )))
            }
            Term::List(values) => self.lower_list(values, graph),
        }
    }

    /// JSON-LD 1.1 Processing Algorithms and API §8.4 List Conversion: a fresh blank
    /// node for every entry of the list first, then each entry converted to its object,
    /// then the `rdf:first`/`rdf:rest` cells ([`build_rdf_list`]). An entry's
    /// annotations annotate its `rdf:first` statement.
    fn lower_list(
        &mut self,
        values: &[Value],
        graph: Option<&RdfTerm>,
    ) -> Result<RdfTerm, RdfDiagnostic> {
        let cells: Vec<RdfTerm> = values.iter().map(|_| self.fresh_list_node()).collect();
        let items = values
            .iter()
            .map(|value| self.lower_term(&value.term, graph))
            .collect::<Result<Vec<_>, _>>()?;
        let vocab = ListVocab {
            first: validated_iri_term(RDF_FIRST)?,
            rest: validated_iri_term(RDF_REST)?,
            nil: validated_iri_term(RDF_NIL)?,
        };
        let quads = &mut self.quads;
        let head = build_rdf_list(
            items.iter().cloned(),
            &vocab,
            |index| cells[index].clone(),
            |subject, predicate, object| {
                let RdfTerm::Iri(predicate) = predicate else {
                    unreachable!("the list vocabulary is rdf:first and rdf:rest, both IRIs")
                };
                quads.push(graph_quad(subject, &predicate, object, graph));
            },
        );
        for ((cell, item), value) in cells.iter().zip(&items).zip(values) {
            self.lower_annotations(cell, RDF_FIRST, item, &value.annotations, graph)?;
        }
        Ok(head)
    }

    fn push(
        &mut self,
        subject: RdfTerm,
        predicate: &str,
        object: RdfTerm,
        graph: Option<&RdfTerm>,
    ) {
        self.quads
            .push(graph_quad(subject, predicate, object, graph));
    }
}

/// The quad `(subject, predicate, object)`, in `graph` when one is given.
fn graph_quad(
    subject: RdfTerm,
    predicate: &str,
    object: RdfTerm,
    graph: Option<&RdfTerm>,
) -> RdfQuad {
    let quad = RdfQuad::new(subject, predicate, object);
    match graph {
        Some(graph) => quad.in_graph(graph.clone()),
        None => quad,
    }
}

fn reserve_node_blank_ids(node: &Node, output: &mut BTreeSet<String>) {
    reserve_blank_id(&node.id, output);
    for part in node.parts() {
        match part {
            Part::Term(Term::Id(id)) => reserve_blank_id(id.as_str(), output),
            Part::Annotation(annotation) => reserve_blank_id(&annotation.id, output),
            Part::Term(_) => {}
        }
    }
}

fn reserve_blank_id(id: &str, output: &mut BTreeSet<String>) {
    if let Some(label) = id.strip_prefix("_:") {
        output.insert(label.to_owned());
    }
}

/// Resolve a JSON-LD `@id` to its RDF term: a `_:`-prefixed identifier is a blank
/// node, anything else an absolute IRI.
///
/// # Blank-label text ingress
///
/// A blank identifier is a document token, so it carries the `(label, scope)`
/// encoding the serializer applied — under
/// [`LabelAlphabet::BlankNodeLabel`](purrdf_core::blank_label::LabelAlphabet::BlankNodeLabel),
/// the alphabet JSON-LD / YAML-LD type their `_:` names as. This lane lowers
/// through the OWNED model ([`RdfTerm`] has one string slot for a blank), which
/// is a DIFFERENT (unconstrained) surface, so the token is TRANSCODED here:
/// decoded against the document's alphabet, then re-encoded into the owned
/// spelling, which
/// [`RdfDatasetBuilder::intern_owned_term`](crate::RdfDatasetBuilder::intern_owned_term)
/// decodes when the owned quads are frozen — the only place a [`BlankScope`] can
/// be attached. Composed, the two steps are the exact inverse of egress, so a
/// JSON-LD / YAML-LD document this workspace wrote re-parses to the
/// `(label, scope)` pair it was written from; re-escaping the token instead
/// would envelope an envelope and lose that identity.
///
/// [`BlankScope`]: crate::BlankScope
fn validated_iri_term(iri: &str) -> Result<RdfTerm, RdfDiagnostic> {
    purrdf_iri::BaseScope::empty()
        .resolve(iri)
        .map_err(|source| {
            RdfDiagnostic::error(
                source.diagnostic_code(),
                format!("invalid RDF IRI: {source}"),
            )
        })?;
    native_iri_term(iri)
}

fn id_term(id: &str) -> Result<RdfTerm, RdfDiagnostic> {
    if let Some(token) = id.strip_prefix("_:") {
        if token.is_empty() {
            return Err(decode("blank-node identifier cannot be empty"));
        }
        let (label, scope) = purrdf_core::blank_label::decode_blank_label(
            token,
            purrdf_core::blank_label::LabelAlphabet::BlankNodeLabel,
        );
        Ok(RdfTerm::blank_node(
            scope.qualify_label(&label).into_owned(),
        ))
    } else {
        validated_iri_term(id)
    }
}

/// The JSON-LD / YAML-LD language-tag contract: the concrete syntaxes' `LANGTAG`
/// terminal under the RFC 5646 §2.1 eight-character subtag ceiling, decided by
/// [`purrdf_iri::langtag`].
///
/// There was NO contract here before. `@language` was read, case-folded, and
/// interned unexamined, so this reader admitted `1`, `9-9`, `123-456`, `en-`,
/// `-` and `!!!` — and, worst of all, `en us`, whose embedded space is not
/// expressible in `LANGTAG` at all. A document carrying it converted to N-Quads
/// with exit 0, an empty loss ledger, and a line reading `"hello"@en us .` that
/// no parser in this workspace (or any other) can read back. The writer was
/// faithful; the reader was the defect.
///
/// The profile is [`langtag::Profile::ConcreteSyntaxLangtagBounded`], the one
/// acceptance language every codec in this crate names — `text_parse`'s two
/// parsers, `rdfxml`, and the term projection in `projections::term` all name
/// the same one. It has to be shared, and for the same reason stated there: a
/// dataset read from JSON-LD is serialized to Turtle, N-Quads or RDF/XML
/// verbatim, so any tag this reader takes that those readers refuse is a file
/// this workspace writes and cannot read back.
///
/// # Why the check is here and not at the three `@language` entry points
///
/// A language tag reaches the carrier from three places — a value object's
/// `@language` member ([`literal`]), a `@container: "@language"`
/// map key (the structural language-map stage), and the `@context` default / term
/// `@language` mapping applied to a bare string (the structural value-expansion stage, via
/// the structural scalar stage). All three converge HERE: [`lower_literal`] is the only
/// function in this module that builds a language-tagged [`RdfLiteral`], and
/// [`Lowerer::lower_term`] is the only caller. Gating the funnel
/// is therefore complete by construction, where three entry-point gates would be
/// complete only as long as nobody adds a fourth.
///
/// The tag named in the failure is the case-folded one, because that is the tag
/// the carrier holds and the tag a serializer would have written; naming the
/// document's original casing would name a string that never reaches a file.
///
/// The failure reports the module's
/// [`langtag::LanguageTagError::diagnostic_code`], so the user learns which
/// production refused. JSON-LD decode diagnostics carry no line/column or JSON
/// pointer (the JSON value tree this walks has discarded both by the
/// time expansion runs), which this does not change.
/// # Why it is `pub(super)`, and what `what` is for
///
/// The funnel argument above covers every tag that becomes a LITERAL's tag. It
/// does not cover a tag that is only ever written back out: a `@context`'s
/// `@language` — default or per-term — is serialized into every compacted
/// document by [`super::carrier`], so an unjudged one makes this codec emit
/// `{"@context":{"@language":"en us"}}`, bytes whose own `@language` member no
/// reader in this workspace will take back the moment a bare string uses it.
/// [`super::context::compiler`] therefore calls this same function at the two
/// context entry points, and `what` names the position so the two callers'
/// messages differ while the grammar and the diagnostic code do not. There is
/// deliberately no second copy of the profile inside the codec.
fn validate_language_tag_at(tag: &str, what: &str) -> Result<(), RdfDiagnostic> {
    check_language_tag(tag, |error| {
        format!("JSON-LD: invalid {what} {tag:?}: {error}")
    })
}

/// [`validate_language_tag_at`] at a value's own `@language`.
fn validate_language_tag(tag: &str) -> Result<(), RdfDiagnostic> {
    validate_language_tag_at(tag, "language tag")
}

/// [`validate_language_tag_at`] for [`super::context::compiler`], whose two
/// `@language` entry points name their own positions.
pub(super) fn validate_context_language_tag(tag: &str, what: &str) -> Result<(), RdfDiagnostic> {
    validate_language_tag_at(tag, what)
}

fn lower_literal(literal: &Literal) -> Result<RdfTerm, RdfDiagnostic> {
    if let Some(language) = literal.language.as_deref() {
        validate_language_tag(language)?;
    }
    let value = match (
        literal.language.as_deref(),
        literal.direction.as_deref(),
        literal.datatype.as_deref(),
    ) {
        (Some(language), Some(direction), _) => {
            let direction = RdfTextDirection::from_str_token(direction)
                .ok_or_else(|| decode(format!("invalid direction `{direction}`")))?;
            RdfLiteral {
                lexical_form: literal.lexical.clone(),
                datatype: None,
                language: Some(language.to_owned()),
                direction: Some(direction),
            }
        }
        (Some(language), None, _) => RdfLiteral::language_tagged(literal.lexical.clone(), language),
        (None, None, Some(datatype)) => {
            validated_iri_term(datatype)?;
            RdfLiteral::typed(literal.lexical.clone(), datatype)
        }
        (None, Some(_), _) => {
            return Err(decode("directional literal requires a language tag"));
        }
        (None, None, None) => RdfLiteral::simple(literal.lexical.clone()),
    };
    Ok(RdfTerm::literal(value))
}
