// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The `PREFIX` header prepended to every SHACL-SPARQL query of a shapes graph.
//!
//! # What the specification defines
//!
//! SHACL 1.2 SPARQL Extensions, "Prefix Declarations for SPARQL Queries":
//!
//! > A SHACL processor collects a set of prefix mappings as the union of all
//! > individual prefix mappings that are values of the SPARQL property path
//! > `sh:prefixes/(^owl:versionIRI?/owl:imports)*/sh:declare` of the SPARQL-based
//! > constraint or validator.
//!
//! > If such a collection of prefix declarations contains multiple different
//! > namespaces for the same value of `sh:prefix`, then the shapes graph is
//! > ill-formed. (Note that SHACL processors MAY ignore prefix declarations that are
//! > never reached).
//!
//! > If a SPARQL query has no value for `sh:prefixes` then the system will use those
//! > prefix declarations from the shapes graph that are values of `sh:declare` at a
//! > SHACL instance of `owl:Ontology`, `sh:DataGraph`, `sh:ShapesGraph`, or
//! > `sh:RulesGraph` (which is a subclass of `sh:ShapesGraph`).
//!
//! and of each declaration: "Prefix declarations have exactly one value for the
//! property `sh:prefix`. The values of `sh:prefix` are literals of datatype
//! `xsd:string`. Prefix declarations have exactly one value for the property
//! `sh:namespace`. The values of `sh:namespace` are literals of datatype
//! `xsd:anyURI` or `xsd:string`."
//!
//! # How this module applies it
//!
//! * **The spec collection.** When any owner of a query (the query node, and the
//!   shape or component that carries it) has `sh:prefixes`, the collection is the
//!   union, over every such owner, of the declarations the path above reaches. When
//!   none has, the collection is the IMPLICIT one: every `sh:declare` value of every
//!   SHACL instance (`rdf:type/rdfs:subClassOf*` in the shapes graph) of the four
//!   classes. `sh:RulesGraph` is listed as a class of its own, so its instances count
//!   whether or not the shapes graph states its subclass axiom.
//! * **Conflicts are refused**, in both collections: two different namespaces for one
//!   prefix is the spec's ill-formed shapes graph, and the load error names the
//!   prefix, both namespaces and both declarations. The same prefix bound to the same
//!   namespace twice is one mapping and loads. The check is scoped to REACHED
//!   collections, as the note permits: declarations no query of the shapes graph
//!   reaches are never read, so an unused conflicting declaration elsewhere in the
//!   graph (the W3C `prefixes-002` case has one) does not refuse the graph.
//! * **A reached declaration that is not one is refused**: a value of `sh:declare`
//!   or `sh:prefixes` that is a literal, or a declaration without exactly one
//!   `xsd:string` `sh:prefix` and exactly one `sh:namespace`. Skipping it would drop
//!   a mapping the author wrote and surface later as an unrelated "unparsable query".
//!   A `sh:namespace` spelled as an IRI rather than a literal is accepted: it names
//!   the same namespace without ambiguity, and refusing it would reject shapes graphs
//!   that load today with an unchanged meaning.
//! * **The document `@prefix` fallback** is a PurRDF convenience (the behaviour real
//!   shapes graphs written for pySHACL rely on), not a spec rule. It is the LOWEST
//!   precedence source: it only supplies a label the spec collection does not bind,
//!   and never replaces a binding the spec collection makes — so wherever the
//!   specification defines a mapping, the header carries exactly that mapping. It is
//!   the Turtle codec's own record of the document's directives (see
//!   `crate::text_ingest::TurtleDocument::prefixes` for the redefinition rule), never
//!   a text scan.
//! * **A query's own prologue** comes after the header, so a `PREFIX` the query
//!   itself declares rebinds the label for that query alone (SPARQL 1.2 §4.1.1), and
//!   never leaks into any other query.
//!
//! The implicit collection is the same for every query without `sh:prefixes`, so it
//! is collected — and its header rendered — once per parse, on first use.

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::fmt::Write as _;

use ::purrdf::RdfDataset;

use super::objects_of;
use crate::data::{GraphFilter, native_quads};
use crate::model::{rdf, sh, xsd};
use crate::term::{NamedNode, Term};

const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
const OWL_IMPORTS: &str = "http://www.w3.org/2002/07/owl#imports";
const OWL_VERSION_IRI: &str = "http://www.w3.org/2002/07/owl#versionIRI";
const SH_DATA_GRAPH: &str = "http://www.w3.org/ns/shacl#DataGraph";
const SH_SHAPES_GRAPH: &str = "http://www.w3.org/ns/shacl#ShapesGraph";
const SH_RULES_GRAPH: &str = "http://www.w3.org/ns/shacl#RulesGraph";
const XSD_ANY_URI: &str = "http://www.w3.org/2001/XMLSchema#anyURI";

/// The classes whose SHACL instances supply the implicit prefix declarations.
const IMPLICIT_HOLDER_CLASSES: [&str; 4] =
    [OWL_ONTOLOGY, SH_DATA_GRAPH, SH_SHAPES_GRAPH, SH_RULES_GRAPH];

/// One collection of prefix mappings: label → (namespace, the declaration node that
/// bound it). The declaration is kept for the conflict diagnostic.
type Collection = BTreeMap<String, (String, Term)>;

/// The prefix sources of one shapes-graph parse.
pub(crate) struct PrefixResolver {
    /// The shapes document's `@prefix` map (the fallback), exactly as the parse
    /// received it: its order is the parse's provenance
    /// ([`crate::provenance::ParseProvenance::doc_prefixes`]), and a label listed twice
    /// resolves to its last entry.
    document: Vec<(String, String)>,
    /// The rendered header for a query with no `sh:prefixes`: the implicit
    /// collection over the document fallback. Collected on first use.
    implicit_header: OnceCell<Result<String, String>>,
}

impl PrefixResolver {
    /// The resolver for a shapes graph whose document declared `document_prefixes`.
    pub(crate) fn new(document_prefixes: &[(String, String)]) -> Self {
        Self {
            document: document_prefixes.to_vec(),
            implicit_header: OnceCell::new(),
        }
    }

    /// The document prefix map this resolver falls back to, in the order it was
    /// received.
    pub(crate) fn document(&self) -> &[(String, String)] {
        &self.document
    }

    /// The `PREFIX` header for a query whose owners are `owners`: one
    /// `PREFIX p: <ns>` line per label, sorted by label. Empty when nothing binds a
    /// label.
    ///
    /// # Errors
    ///
    /// When the collection the query reaches is ill-formed: a conflicting binding,
    /// a value of `sh:prefixes` or `sh:declare` that cannot be a node, or a
    /// declaration without exactly one `sh:prefix` and one `sh:namespace`.
    pub(crate) fn header(&self, data: &RdfDataset, owners: &[&Term]) -> Result<String, String> {
        let mut roots: Vec<Term> = Vec::new();
        for owner in owners {
            roots.extend(objects_of(data, owner, sh::PREFIXES));
        }
        if roots.is_empty() {
            return self
                .implicit_header
                .get_or_init(|| {
                    implicit_collection(data).map(|collection| self.render(&collection))
                })
                .clone();
        }
        let origin = owners
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let mut collection = Collection::new();
        for root in roots {
            if !root.is_subject() {
                return Err(format!(
                    "the shapes graph is ill-formed: {origin} has the sh:prefixes value {root}, \
                     which is not an IRI or a blank node (SHACL 1.2 SPARQL Extensions: \"The \
                     values of sh:prefixes are either IRIs or blank nodes\")"
                ));
            }
            for holder in reached_holders(data, &root) {
                collect_declarations(data, &holder, &origin, &mut collection)?;
            }
        }
        Ok(self.render(&collection))
    }

    /// Render `collection` over the document fallback: a label the collection binds
    /// takes the collection's namespace; the fallback only fills labels it leaves
    /// unbound.
    fn render(&self, collection: &Collection) -> String {
        let mut merged: BTreeMap<&str, &str> = self
            .document
            .iter()
            .map(|(label, namespace)| (label.as_str(), namespace.as_str()))
            .collect();
        for (label, (namespace, _)) in collection {
            merged.insert(label.as_str(), namespace.as_str());
        }
        let mut header = String::new();
        for (label, namespace) in merged {
            let _ = writeln!(header, "PREFIX {label}: <{namespace}>");
        }
        header
    }
}

/// Every node the path `(^owl:versionIRI?/owl:imports)*` reaches from `root`,
/// `root` first, each once, in discovery order.
///
/// A step moves from a node `n` to every `owl:imports` value of `n` and of every
/// node whose `owl:versionIRI` is `n`: the version IRI a graph was imported by is
/// navigated back to the graph before its imports are followed. The node a step
/// passes through on the way (the unversioned graph IRI) is not itself reached —
/// only the targets of `owl:imports` are — exactly as the path says.
fn reached_holders(data: &RdfDataset, root: &Term) -> Vec<Term> {
    let mut reached: Vec<Term> = vec![root.clone()];
    let mut next = 0;
    while next < reached.len() {
        let node = reached[next].clone();
        next += 1;
        let mut heads = vec![node.clone()];
        heads.extend(subjects_of(data, OWL_VERSION_IRI, &node));
        for head in heads {
            for imported in objects_of(data, &head, OWL_IMPORTS) {
                if imported.is_subject() && !reached.contains(&imported) {
                    reached.push(imported);
                }
            }
        }
    }
    reached
}

/// The implicit collection: every `sh:declare` value of every SHACL instance of
/// [`IMPLICIT_HOLDER_CLASSES`], holders in canonical order so a conflict is reported
/// the same way on every run.
fn implicit_collection(data: &RdfDataset) -> Result<Collection, String> {
    let mut instances = super::parser::shacl_instance::ShaclInstances::new(data);
    let classes: Vec<_> = IMPLICIT_HOLDER_CLASSES
        .iter()
        .filter_map(|class| data.term_id_by_iri(class))
        .collect();
    let mut holders: Vec<Term> = Vec::new();
    if !classes.is_empty()
        && let Some(rdf_type) = data.term_id_by_iri(rdf::TYPE)
    {
        let mut seen = ::purrdf::IdSet::default();
        for quad in crate::data::quads_for_pattern_ids(
            data,
            None,
            Some(rdf_type),
            None,
            GraphFilter::AnyGraph,
        ) {
            if !seen.insert(quad.s) {
                continue;
            }
            if classes
                .iter()
                .any(|&class| instances.is_instance(quad.s, Some(class)))
            {
                holders.push(crate::term::term_id_to_native(data, quad.s));
            }
        }
    }
    crate::term::sort_terms_canonical(&mut holders);
    let origin = "the shapes graph's owl:Ontology / sh:DataGraph / sh:ShapesGraph / \
                  sh:RulesGraph declarations (used by every SPARQL query without \
                  sh:prefixes)";
    let mut collection = Collection::new();
    for holder in holders {
        collect_declarations(data, &holder, origin, &mut collection)?;
    }
    Ok(collection)
}

/// Add every `sh:declare` value of `holder` to `collection`, refusing a malformed
/// declaration and a conflicting binding.
fn collect_declarations(
    data: &RdfDataset,
    holder: &Term,
    origin: &str,
    collection: &mut Collection,
) -> Result<(), String> {
    let mut declarations = objects_of(data, holder, sh::DECLARE);
    crate::term::sort_terms_canonical(&mut declarations);
    for declaration in declarations {
        let (label, namespace) = declaration_mapping(data, holder, &declaration)?;
        match collection.get(&label) {
            Some((bound, _)) if *bound == namespace => {}
            Some((bound, by)) => {
                return Err(format!(
                    "the shapes graph is ill-formed: the prefix declarations reached from \
                     {origin} bind the prefix \"{label}\" to two different namespaces, \
                     <{bound}> (declared by {by}) and <{namespace}> (declared by \
                     {declaration}); SHACL 1.2 SPARQL Extensions: \"If such a collection of \
                     prefix declarations contains multiple different namespaces for the same \
                     value of sh:prefix, then the shapes graph is ill-formed\""
                ));
            }
            None => {
                collection.insert(label, (namespace, declaration));
            }
        }
    }
    Ok(())
}

/// The `(prefix, namespace)` mapping of one value of `sh:declare`.
fn declaration_mapping(
    data: &RdfDataset,
    holder: &Term,
    declaration: &Term,
) -> Result<(String, String), String> {
    let ill_formed = |detail: String| {
        format!(
            "the shapes graph is ill-formed: the sh:declare value {declaration} of {holder} \
             {detail} (SHACL 1.2 SPARQL Extensions, \"Prefix Declarations for SPARQL \
             Queries\")"
        )
    };
    if !declaration.is_subject() {
        return Err(ill_formed(
            "is not a prefix declaration: a prefix declaration is an IRI or a blank node"
                .to_owned(),
        ));
    }
    let label = match objects_of(data, declaration, sh::PREFIX).as_slice() {
        [Term::Literal(literal)] if literal.datatype_str() == xsd::STRING => {
            literal.value().to_owned()
        }
        [value] => {
            return Err(ill_formed(format!(
                "has the sh:prefix value {value}; the values of sh:prefix are literals of \
                 datatype xsd:string"
            )));
        }
        values => {
            return Err(ill_formed(format!(
                "has {} sh:prefix values; a prefix declaration has exactly one",
                values.len()
            )));
        }
    };
    let namespace = match objects_of(data, declaration, sh::NAMESPACE).as_slice() {
        [Term::Literal(literal)]
            if literal.datatype_str() == XSD_ANY_URI || literal.datatype_str() == xsd::STRING =>
        {
            literal.value().to_owned()
        }
        [Term::NamedNode(node)] => node.as_str().to_owned(),
        [value] => {
            return Err(ill_formed(format!(
                "has the sh:namespace value {value}; the values of sh:namespace are literals \
                 of datatype xsd:anyURI or xsd:string"
            )));
        }
        values => {
            return Err(ill_formed(format!(
                "has {} sh:namespace values; a prefix declaration has exactly one",
                values.len()
            )));
        }
    };
    Ok((label, namespace))
}

/// Every subject `s` of `(s, predicate, object)`.
fn subjects_of(data: &RdfDataset, predicate: &str, object: &Term) -> Vec<Term> {
    let predicate = Term::NamedNode(NamedNode::from(predicate));
    native_quads(
        data,
        None,
        Some(&predicate),
        Some(object),
        GraphFilter::AnyGraph,
    )
    .into_iter()
    .map(|(subject, _, _)| subject)
    .collect()
}
