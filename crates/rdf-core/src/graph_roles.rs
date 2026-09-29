// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE graph-role classifier: which nodes of a document declare a graph — an OWL 2
//! ontology header, a SHACL shapes graph, a SHACL data graph — and so which of them the
//! workspace's import rule ([`crate::imports`]) and SHACL-SPARQL's implicit prefixes read.
//!
//! # Why one classifier
//!
//! Two rules read "the nodes that name a graph": the `owl:imports` rule (which triples are
//! imports) and SHACL-SPARQL's implicit prefix collection (which `sh:declare` values a query
//! with no `sh:prefixes` sees). Each once kept its own list of classes. Two lists drift: one
//! engine followed an import on a `sh:ShapesGraph` node that another treated as data, and the
//! prefix collection read declarations off nodes the import rule could not see. So the roles
//! are classified here, once, and both rules SELECT from this answer.
//!
//! # The roles, and the specification text for each
//!
//! * [`GraphRoles::ONTOLOGY_HEADER`] — a SHACL instance of `owl:Ontology`, IRI or blank node.
//!   *OWL 2 Mapping to RDF Graphs* §3.1.2, Table 4, reads a document's ontology header off
//!   `*:x rdf:type owl:Ontology` (and `_:x rdf:type owl:Ontology` for an anonymous one).
//! * [`GraphRoles::SHAPES_GRAPH`] — a SHACL instance of `sh:ShapesGraph`. SHACL 1.2 Core §6.1:
//!   "The sh:ShapesGraph class MAY be used as an rdf:type of the IRI of a graph that typically
//!   acts in the role of a shapes graph." `sh:RulesGraph` counts in its own right: SHACL 1.2
//!   SPARQL Extensions names it "sh:RulesGraph (which is a subclass of sh:ShapesGraph)", so a
//!   document that types a node `sh:RulesGraph` without also stating that axiom is still
//!   declaring a shapes graph.
//! * [`GraphRoles::DATA_GRAPH`] — a SHACL instance of `sh:DataGraph`. SHACL 1.2 Core §6.2:
//!   "The sh:DataGraph class MAY be used as an rdf:type of the IRI of a graph that is
//!   typically used as a data graph."
//!
//! "SHACL instance" is SHACL 1.2 Core's: "The SHACL types of an RDF term in an RDF graph is
//! the set of its values for rdf:type in the graph as well as the SHACL superclasses of these
//! values in the graph" — `rdf:type/rdfs:subClassOf*` over the document itself, so a user
//! class declared `rdfs:subClassOf sh:ShapesGraph` declares shapes graphs too. OWL 2's Table 4
//! matches `rdf:type owl:Ontology` literally; reading it through `rdfs:subClassOf*` as well is
//! a superset of that pattern, never a subset — OWL 2 is the floor here, not the limit.
//!
//! The roles are not exclusive. SHACL 1.2 Core §6.1: "The graph type classes (such as
//! sh:DataGraph and sh:ShapesGraph) represent roles that are not mutually exclusive; a single
//! graph MAY be typed with more than one of these classes." A node carries every role it has.
//!
//! # What each rule selects
//!
//! * **Imports** ([`GraphRoles::is_import_anchor`]): an ontology header or a shapes graph.
//!   A node whose only role is [`GraphRoles::DATA_GRAPH`] is not: SHACL 1.2 Core §6.2,
//!   "owl:imports in the data graph is not enacted, in order to avoid uncontrolled increase of
//!   validation work", and its next note asks that "when using statements of the form X a
//!   sh:DataGraph ., X a owl:Ontology . should also be included" — the `owl:Ontology` type,
//!   not the data-graph one, is what makes such an `owl:imports` a directive.
//! * **Implicit prefixes** ([`GraphRoles::declares_implicit_prefixes`]): every role. SHACL 1.2
//!   SPARQL Extensions: "If a SPARQL query has no value for sh:prefixes then the system will
//!   use those prefix declarations from the shapes graph that are values of sh:declare at a
//!   SHACL instance of owl:Ontology, sh:DataGraph, sh:ShapesGraph, or sh:RulesGraph (which is
//!   a subclass of sh:ShapesGraph)."
//!
//! # The kernel names these W3C terms
//!
//! PurRDF mints no vocabulary. These are not minted: they are the W3C's own classes, named
//! by the specifications the kernel's import rule implements, and the rule cannot be stated
//! without them. The kernel once limited itself to OWL's vocabulary; that limit is retired
//! (`docs/SUPERSEDED-CLAIMS.md`).
//!
//! # Cost
//!
//! [`GraphRoleIndex::classify`] looks up `rdf:type` and the four classes first and returns
//! the empty index when the document interns `rdf:type` and none of them. Otherwise it walks
//! `rdfs:subClassOf` BACKWARDS from each class (so a document without subclass axioms pays
//! one lookup per class), then reads the `rdf:type` quads whose object is one of the classes
//! found — never a scan of the whole document. Callers classify only when the rule they
//! apply can fire at all: the import rule only when the document interns `owl:imports`, the
//! prefix rule only when it interns `sh:declare`.

use crate::dataset_view::{DatasetView, GraphMatch};
use crate::ir::TermValue;

/// `owl:Ontology`.
pub use purrdf_iri::vocab::owl::ONTOLOGY as OWL_ONTOLOGY;
/// `rdf:type`.
pub use purrdf_iri::vocab::rdf::TYPE as RDF_TYPE;
/// `rdfs:subClassOf`.
pub use purrdf_iri::vocab::rdfs::SUB_CLASS_OF as RDFS_SUB_CLASS_OF;
/// `sh:DataGraph`.
pub use purrdf_iri::vocab::sh::DATA_GRAPH as SH_DATA_GRAPH;
/// `sh:RulesGraph`, a subclass of `sh:ShapesGraph`.
pub use purrdf_iri::vocab::sh::RULES_GRAPH as SH_RULES_GRAPH;
/// `sh:ShapesGraph`.
pub use purrdf_iri::vocab::sh::SHAPES_GRAPH as SH_SHAPES_GRAPH;

/// The set of graph roles one node declares. See the [module documentation](self).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GraphRoles(u8);

impl GraphRoles {
    /// No role.
    pub const NONE: Self = Self(0);
    /// A SHACL instance of `owl:Ontology`: an OWL 2 ontology header.
    pub const ONTOLOGY_HEADER: Self = Self(1);
    /// A SHACL instance of `sh:ShapesGraph` (`sh:RulesGraph` and user subclasses included).
    pub const SHAPES_GRAPH: Self = Self(1 << 1);
    /// A SHACL instance of `sh:DataGraph`.
    pub const DATA_GRAPH: Self = Self(1 << 2);

    /// Every role in `self` or `other`.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether `self` holds every role in `other`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether `self` holds no role.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether an `owl:imports` on a node with these roles is an import: the node is an
    /// ontology header or a shapes graph. A data-graph role alone is not enough (SHACL 1.2
    /// Core §6.2, "owl:imports in the data graph is not enacted").
    #[must_use]
    pub const fn is_import_anchor(self) -> bool {
        self.0 & (Self::ONTOLOGY_HEADER.0 | Self::SHAPES_GRAPH.0) != 0
    }

    /// Whether the node's `sh:declare` values are implicit prefix declarations: it has any
    /// role at all (SHACL 1.2 SPARQL Extensions, "Prefix Declarations for SPARQL Queries").
    #[must_use]
    pub const fn declares_implicit_prefixes(self) -> bool {
        !self.is_empty()
    }

    /// The roles' names, in the order `ontology-header`, `shapes-graph`, `data-graph`.
    #[must_use]
    pub fn labels(self) -> Vec<&'static str> {
        [
            (Self::ONTOLOGY_HEADER, "ontology-header"),
            (Self::SHAPES_GRAPH, "shapes-graph"),
            (Self::DATA_GRAPH, "data-graph"),
        ]
        .into_iter()
        .filter(|(role, _)| self.contains(*role))
        .map(|(_, label)| label)
        .collect()
    }
}

/// Every node of one document that declares a graph role, with its roles, sorted by id.
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_core::graph_roles::{GraphRoleIndex, GraphRoles};
///
/// let mut b = RdfDatasetBuilder::new();
/// let rdf_type = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// let sub_class_of = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let shapes_graph = b.intern_iri("http://www.w3.org/ns/shacl#ShapesGraph");
/// let data_graph = b.intern_iri("http://www.w3.org/ns/shacl#DataGraph");
/// let module = b.intern_iri("http://example.org/Module");
/// let a = b.intern_iri("http://example.org/a");
/// let d = b.intern_iri("http://example.org/d");
/// b.push_quad(module, sub_class_of, shapes_graph, None);
/// b.push_quad(a, rdf_type, module, None);
/// b.push_quad(d, rdf_type, data_graph, None);
/// let graph = b.freeze().expect("freeze");
///
/// let roles = GraphRoleIndex::classify(graph.as_ref());
/// // A user subclass of sh:ShapesGraph declares a shapes graph: an import anchor.
/// assert!(roles.roles(a).is_import_anchor());
/// // A data graph declares implicit prefixes but anchors no import.
/// assert_eq!(roles.roles(d), GraphRoles::DATA_GRAPH);
/// assert!(!roles.roles(d).is_import_anchor());
/// assert!(roles.roles(d).declares_implicit_prefixes());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRoleIndex<I> {
    /// `(node, roles)`, sorted by node, each node once, every role non-empty.
    nodes: Vec<(I, GraphRoles)>,
}

impl<I: Copy + Ord> Default for GraphRoleIndex<I> {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

impl<I: Copy + Ord> GraphRoleIndex<I> {
    /// Classify every node of `graph` (all of its graphs, default and named).
    pub fn classify<D: DatasetView<Id = I>>(graph: &D) -> Self {
        let term = |iri: &str| graph.term_id_by_value(&TermValue::iri(iri));
        let Some(rdf_type) = term(RDF_TYPE) else {
            return Self::default();
        };
        let roots: [(GraphRoles, &[&str]); 3] = [
            (GraphRoles::ONTOLOGY_HEADER, &[OWL_ONTOLOGY]),
            (GraphRoles::SHAPES_GRAPH, &[SH_SHAPES_GRAPH, SH_RULES_GRAPH]),
            (GraphRoles::DATA_GRAPH, &[SH_DATA_GRAPH]),
        ];
        let sub_class_of = term(RDFS_SUB_CLASS_OF);
        let mut nodes: Vec<(I, GraphRoles)> = Vec::new();
        for (role, classes) in roots {
            let mut reached: Vec<I> = classes.iter().filter_map(|iri| term(iri)).collect();
            if reached.is_empty() {
                continue;
            }
            // Every class that reaches one of `classes` through `rdfs:subClassOf*`: a
            // backwards walk, each class once.
            if let Some(sub_class_of) = sub_class_of {
                let mut next = 0;
                while next < reached.len() {
                    let class = reached[next];
                    next += 1;
                    for quad in graph.quads_for_pattern(
                        None,
                        Some(sub_class_of),
                        Some(class),
                        GraphMatch::Any,
                    ) {
                        if !reached.contains(&quad.s) {
                            reached.push(quad.s);
                        }
                    }
                }
            }
            for class in reached {
                nodes.extend(
                    graph
                        .quads_for_pattern(None, Some(rdf_type), Some(class), GraphMatch::Any)
                        .map(|quad| (quad.s, role)),
                );
            }
        }
        nodes.sort_unstable();
        let mut merged: Vec<(I, GraphRoles)> = Vec::with_capacity(nodes.len());
        for (node, role) in nodes {
            match merged.last_mut() {
                Some((last, roles)) if *last == node => *roles = roles.union(role),
                _ => merged.push((node, role)),
            }
        }
        Self { nodes: merged }
    }

    /// The roles `node` declares; [`GraphRoles::NONE`] for a node that declares none.
    #[must_use]
    pub fn roles(&self, node: I) -> GraphRoles {
        self.nodes
            .binary_search_by(|(id, _)| id.cmp(&node))
            .map_or(GraphRoles::NONE, |at| self.nodes[at].1)
    }

    /// Every node with a role, and its roles, in id order.
    pub fn iter(&self) -> impl Iterator<Item = (I, GraphRoles)> + '_ {
        self.nodes.iter().copied()
    }

    /// Every node an `owl:imports` is an import on ([`GraphRoles::is_import_anchor`]), in id
    /// order.
    pub fn import_anchors(&self) -> impl Iterator<Item = I> + '_ {
        self.iter()
            .filter(|(_, roles)| roles.is_import_anchor())
            .map(|(node, _)| node)
    }

    /// Every node whose `sh:declare` values are implicit prefix declarations
    /// ([`GraphRoles::declares_implicit_prefixes`]), in id order.
    pub fn implicit_prefix_holders(&self) -> impl Iterator<Item = I> + '_ {
        self.iter()
            .filter(|(_, roles)| roles.declares_implicit_prefixes())
            .map(|(node, _)| node)
    }

    /// Whether no node declares a role.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// How many nodes declare a role.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        GraphRoleIndex, GraphRoles, OWL_ONTOLOGY, RDF_TYPE, RDFS_SUB_CLASS_OF, SH_DATA_GRAPH,
        SH_RULES_GRAPH, SH_SHAPES_GRAPH,
    };
    use crate::ir::{BlankScope, RdfDataset, RdfDatasetBuilder, TermId};

    /// A dataset of IRI triples; a subject starting `_:` is a blank node.
    fn triples(rows: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in rows {
            let s = match s.strip_prefix("_:") {
                Some(label) => b.intern_blank(label, BlankScope::DEFAULT),
                None => b.intern_iri(s),
            };
            let p = b.intern_iri(p);
            let o = b.intern_iri(o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    fn id(graph: &RdfDataset, iri: &str) -> TermId {
        graph.term_id_by_iri(iri).expect("interned")
    }

    /// Each class gives its role, and the control row — a node typed some other class —
    /// gives none, so an answer that ignored the type would be observed.
    #[test]
    fn each_graph_class_gives_its_role_and_another_class_gives_none() {
        let graph = triples(&[
            ("http://example.org/o", RDF_TYPE, OWL_ONTOLOGY),
            ("http://example.org/s", RDF_TYPE, SH_SHAPES_GRAPH),
            ("http://example.org/r", RDF_TYPE, SH_RULES_GRAPH),
            ("http://example.org/d", RDF_TYPE, SH_DATA_GRAPH),
            ("http://example.org/t", RDF_TYPE, "http://example.org/Thing"),
            ("_:h", RDF_TYPE, OWL_ONTOLOGY),
        ]);
        let roles = GraphRoleIndex::classify(graph.as_ref());
        let of = |iri: &str| roles.roles(id(&graph, iri));
        assert_eq!(of("http://example.org/o"), GraphRoles::ONTOLOGY_HEADER);
        assert_eq!(of("http://example.org/s"), GraphRoles::SHAPES_GRAPH);
        assert_eq!(
            of("http://example.org/r"),
            GraphRoles::SHAPES_GRAPH,
            "sh:RulesGraph is a shapes graph with no subclass axiom in the document"
        );
        assert_eq!(of("http://example.org/d"), GraphRoles::DATA_GRAPH);
        assert_eq!(of("http://example.org/t"), GraphRoles::NONE);
        assert_eq!(roles.len(), 5, "the blank-node header is classified too");
        assert_eq!(roles.import_anchors().count(), 4);
        assert_eq!(roles.implicit_prefix_holders().count(), 5);
    }

    /// `rdfs:subClassOf*` is followed through two steps; the neighbour class that is a
    /// subclass of nothing gives no role.
    #[test]
    fn a_transitive_user_subclass_gives_the_role_and_an_unrelated_class_does_not() {
        let graph = triples(&[
            (
                "http://example.org/Module",
                RDFS_SUB_CLASS_OF,
                SH_SHAPES_GRAPH,
            ),
            (
                "http://example.org/SubModule",
                RDFS_SUB_CLASS_OF,
                "http://example.org/Module",
            ),
            (
                "http://example.org/m",
                RDF_TYPE,
                "http://example.org/SubModule",
            ),
            ("http://example.org/n", RDF_TYPE, "http://example.org/Other"),
        ]);
        let roles = GraphRoleIndex::classify(graph.as_ref());
        assert_eq!(
            roles.roles(id(&graph, "http://example.org/m")),
            GraphRoles::SHAPES_GRAPH
        );
        assert_eq!(
            roles.roles(id(&graph, "http://example.org/n")),
            GraphRoles::NONE
        );
    }

    /// Roles are not exclusive: a node typed two classes carries both, and a data graph
    /// that is also an ontology header is an import anchor while a data graph alone is not.
    #[test]
    fn roles_accumulate_and_only_the_data_graph_role_anchors_nothing() {
        let graph = triples(&[
            ("http://example.org/both", RDF_TYPE, SH_DATA_GRAPH),
            ("http://example.org/both", RDF_TYPE, OWL_ONTOLOGY),
            ("http://example.org/data", RDF_TYPE, SH_DATA_GRAPH),
        ]);
        let roles = GraphRoleIndex::classify(graph.as_ref());
        let both = roles.roles(id(&graph, "http://example.org/both"));
        assert!(both.contains(GraphRoles::DATA_GRAPH));
        assert!(both.contains(GraphRoles::ONTOLOGY_HEADER));
        assert!(both.is_import_anchor());
        assert_eq!(both.labels(), ["ontology-header", "data-graph"]);
        let data = roles.roles(id(&graph, "http://example.org/data"));
        assert!(!data.is_import_anchor());
        assert!(data.declares_implicit_prefixes());
    }

    /// A document with no `rdf:type` — or none of the graph classes — classifies nothing.
    #[test]
    fn a_document_without_graph_types_classifies_nothing() {
        let graph = triples(&[("http://example.org/a", "http://example.org/p", OWL_ONTOLOGY)]);
        assert!(GraphRoleIndex::classify(graph.as_ref()).is_empty());
        let typed = triples(&[("http://example.org/a", RDF_TYPE, "http://example.org/C")]);
        assert!(GraphRoleIndex::classify(typed.as_ref()).is_empty());
    }
}
