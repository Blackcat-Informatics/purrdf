// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! "SHACL instance" over the SHAPES graph, shared by every reader that asks it.
//!
//! SHACL 1.2 Core, "SHACL Type": "The SHACL types of an RDF term in an RDF graph
//! is the set of its values for rdf:type in the graph as well as the SHACL
//! superclasses of these values in the graph." And "SHACL Class Instance": "A node
//! n in an RDF graph G is a SHACL instance of a SHACL class C in G if one of the
//! SHACL types of n in G is C."
//!
//! Three readers of the shapes graph ask it, and they must agree:
//!
//! * shape discovery — "s is a SHACL instance of sh:NodeShape or
//!   sh:PropertyShape" is the first clause of SHACL 1.2 Core's definition of a
//!   shape;
//! * implicit class targets — "If s is a SHACL instance of sh:NodeShape or
//!   sh:PropertyShape in a shapes graph SG and s is also a SHACL instance of
//!   rdfs:Class in SG then the set of SHACL instances of s in a data graph DG is a
//!   target from DG for s in SG";
//! * `sh:closed sh:ByTypes`, whose `collectProperties` moves past a node that is a
//!   SHACL instance of `rdfs:Class` or of `sh:NodeShape`.
//!
//! # `sh:ShapeClass`
//!
//! SHACL 1.2 Core states the subclass axioms of `sh:ShapeClass` normatively, in
//! the text rather than only in the vocabulary file: "The class sh:ShapeClass is
//! an rdfs:subClassOf of both sh:NodeShape and rdfs:Class." A shapes graph need not
//! merge the vocabulary to say `ex:Person a sh:ShapeClass` — the section's own
//! example does exactly that — so those two `rdfs:subClassOf` triples are honoured
//! here whether or not the shapes graph asserts them: a type that reaches
//! `sh:ShapeClass` reaches `sh:NodeShape` and `rdfs:Class` too.

use ::purrdf::{FastMap, IdSet, RdfDataset, TermId};

use crate::data::{GraphFilter, quads_for_pattern_ids};
use crate::model::{rdf, rdfs, sh};

/// The memoized "SHACL instance" relation of one shapes graph.
pub(crate) struct ShaclInstances<'s> {
    data: &'s RdfDataset,
    rdf_type: Option<TermId>,
    sub_class_of: Option<TermId>,
    shape_class: Option<TermId>,
    node_shape: Option<TermId>,
    property_shape: Option<TermId>,
    rdfs_class: Option<TermId>,
    /// Whether a TYPE reaches a class through `rdfs:subClassOf*`, by
    /// `(type, class)`.
    reaches: FastMap<(TermId, TermId), bool>,
}

impl<'s> ShaclInstances<'s> {
    /// The relation over `data`, the shapes graph.
    pub(crate) fn new(data: &'s RdfDataset) -> Self {
        Self {
            data,
            rdf_type: data.term_id_by_iri(rdf::TYPE),
            sub_class_of: data.term_id_by_iri(rdfs::SUB_CLASS_OF),
            shape_class: data.term_id_by_iri(sh::SHAPE_CLASS),
            node_shape: data.term_id_by_iri(sh::NODE_SHAPE),
            property_shape: data.term_id_by_iri(sh::PROPERTY_SHAPE),
            rdfs_class: data.term_id_by_iri(rdfs::CLASS),
            reaches: FastMap::default(),
        }
    }

    /// Whether `node` is a SHACL instance of `class` in the shapes graph, by the
    /// graph's own triples alone; `false` when the shapes graph does not intern
    /// `class`.
    pub(crate) fn is_instance(&mut self, node: TermId, class: Option<TermId>) -> bool {
        let (Some(rdf_type), Some(class)) = (self.rdf_type, class) else {
            return false;
        };
        let data = self.data;
        quads_for_pattern_ids(
            data,
            Some(node),
            Some(rdf_type),
            None,
            GraphFilter::AnyGraph,
        )
        .any(|quad| self.reaches(quad.o, class))
    }

    /// Whether `node` is a SHACL instance of `sh:NodeShape` — directly, through a
    /// SHACL subclass, or through `sh:ShapeClass`.
    pub(crate) fn is_node_shape(&mut self, node: TermId) -> bool {
        self.is_instance(node, self.node_shape) || self.is_shape_class(node)
    }

    /// Whether `node` is a SHACL instance of `sh:PropertyShape`.
    pub(crate) fn is_property_shape(&mut self, node: TermId) -> bool {
        self.is_instance(node, self.property_shape)
    }

    /// Whether `node` is a SHACL instance of `rdfs:Class` — directly, through a
    /// SHACL subclass, or through `sh:ShapeClass`.
    pub(crate) fn is_class(&mut self, node: TermId) -> bool {
        self.is_instance(node, self.rdfs_class) || self.is_shape_class(node)
    }

    /// Whether `node` is a SHACL instance of `sh:ShapeClass`.
    pub(crate) fn is_shape_class(&mut self, node: TermId) -> bool {
        self.is_instance(node, self.shape_class)
    }

    /// Whether `node` carries an IMPLICIT CLASS TARGET: it is a SHACL instance of
    /// `sh:NodeShape` or `sh:PropertyShape` and also of `rdfs:Class` — which a
    /// SHACL instance of `sh:ShapeClass` always is.
    pub(crate) fn has_implicit_class_target(&mut self, node: TermId) -> bool {
        (self.is_node_shape(node) || self.is_property_shape(node)) && self.is_class(node)
    }

    /// Whether `ty` is `class` or a SHACL subclass of it.
    fn reaches(&mut self, ty: TermId, class: TermId) -> bool {
        if let Some(&known) = self.reaches.get(&(ty, class)) {
            return known;
        }
        let mut seen = IdSet::default();
        let mut pending = vec![ty];
        let mut found = false;
        while let Some(current) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }
            if current == class {
                found = true;
                break;
            }
            if let Some(sub_class_of) = self.sub_class_of {
                pending.extend(
                    quads_for_pattern_ids(
                        self.data,
                        Some(current),
                        Some(sub_class_of),
                        None,
                        GraphFilter::AnyGraph,
                    )
                    .map(|quad| quad.o),
                );
            }
        }
        self.reaches.insert((ty, class), found);
        found
    }
}
