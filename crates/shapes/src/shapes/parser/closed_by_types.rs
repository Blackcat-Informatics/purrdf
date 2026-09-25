// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The `sh:closed sh:ByTypes` index (SHACL 1.2 Core §7.9.1).
//!
//! "If $closed is sh:ByTypes, then P is the set of IRI properties that can be
//! reached from the value node via the following algorithm, plus rdf:type" — the
//! `collectProperties` algorithm quoted on [`ClosedTypeIndex`]. Every step of it
//! except the first (reading the value node's `rdf:type` values in the DATA graph)
//! reads the SHAPES graph, so `collectProperties(T)` is computed here once per
//! node `T` it can be non-empty for, and validation looks the value node's types
//! up in the result.
//!
//! `collectProperties(T)` adds something only through `sh:property/sh:path` on a
//! node it visits, and it moves past `T` only when `T` is a SHACL instance of
//! `rdfs:Class` or of `sh:NodeShape`. So a node that is none of those three —
//! no `sh:property`, not a class, not a node shape — collects nothing, and the
//! index is keyed by exactly the nodes that are at least one of them.

use std::collections::BTreeSet;
use std::sync::Arc;

use ::purrdf::{FastMap, IdSet, TermId};

use crate::data::{GraphFilter, quads_for_pattern_ids};
use crate::model::{rdf, rdfs, sh};
use crate::shapes::{ClosedTypeIndex, Parser};
use crate::term::{NamedNode, Term, term_id_to_native};

/// The shapes-graph identities `collectProperties` reads, resolved once. `None`
/// is a term the shapes graph does not intern, which therefore matches no triple.
struct Vocabulary {
    rdf_type: Option<TermId>,
    sub_class_of: Option<TermId>,
    rdfs_class: Option<TermId>,
    node_shape: Option<TermId>,
    property: Option<TermId>,
    path: Option<TermId>,
    target_class: Option<TermId>,
    node: Option<TermId>,
}

impl Parser<'_> {
    /// The shapes graph's `sh:closed sh:ByTypes` index, built on first use and
    /// shared by every constraint that asks for it afterwards.
    ///
    /// # Errors
    ///
    /// Propagates [`ClosedTypeIndex::from_entries`]' refusal, which a build keyed
    /// by a set cannot trigger.
    pub(crate) fn closed_type_index(&mut self) -> Result<Arc<ClosedTypeIndex>, String> {
        if let Some(index) = &self.closed_type_index {
            return Ok(Arc::clone(index));
        }
        let index = Arc::new(self.build_closed_type_index()?);
        self.closed_type_index = Some(Arc::clone(&index));
        Ok(index)
    }

    fn build_closed_type_index(&self) -> Result<ClosedTypeIndex, String> {
        let data = self.data;
        let vocabulary = Vocabulary {
            rdf_type: data.term_id_by_iri(rdf::TYPE),
            sub_class_of: data.term_id_by_iri(rdfs::SUB_CLASS_OF),
            rdfs_class: data.term_id_by_iri(rdfs::CLASS),
            node_shape: data.term_id_by_iri(sh::NODE_SHAPE),
            property: data.term_id_by_iri(sh::PROPERTY),
            path: data.term_id_by_iri(sh::PATH),
            target_class: data.term_id_by_iri(sh::TARGET_CLASS),
            node: data.term_id_by_iri(sh::NODE),
        };
        let mut instances = ShaclInstances::default();

        // The nodes `collectProperties` can collect anything from: the subjects of
        // `sh:property`, and the SHACL instances of `rdfs:Class` and of
        // `sh:NodeShape` (each typed by some `rdf:type` value).
        let mut keys: BTreeSet<TermId> = BTreeSet::new();
        keys.extend(self.subjects_of(vocabulary.property));
        if let Some(rdf_type) = vocabulary.rdf_type {
            for quad in
                quads_for_pattern_ids(data, None, Some(rdf_type), None, GraphFilter::AnyGraph)
            {
                if !keys.contains(&quad.s)
                    && (instances.is_instance(self, &vocabulary, quad.s, vocabulary.rdfs_class)
                        || instances.is_instance(self, &vocabulary, quad.s, vocabulary.node_shape))
                {
                    keys.insert(quad.s);
                }
            }
        }

        let mut entries: Vec<(Term, Vec<NamedNode>)> = Vec::with_capacity(keys.len());
        let mut visited = IdSet::default();
        let mut pending: Vec<TermId> = Vec::new();
        for key in keys {
            let mut properties: Vec<NamedNode> = Vec::new();
            visited.clear();
            pending.clear();
            pending.push(key);
            // "implementations need to avoid infinite loops in the algorithm above
            // by preventing it from visiting the same S twice" — `visited` is that
            // guard, and the order nodes are visited in cannot change the union.
            while let Some(node) = pending.pop() {
                if !visited.insert(node) {
                    continue;
                }
                // "add all IRI properties that can be reached from S via the SPARQL
                // path sh:property/sh:path" — a non-IRI path is not an IRI property.
                for shape in self.objects_of_id(node, vocabulary.property) {
                    for path in self.objects_of_id(shape, vocabulary.path) {
                        match term_id_to_native(data, path) {
                            Term::NamedNode(property) => properties.push(property),
                            Term::BlankNode(_) | Term::Literal(_) | Term::Triple(_) => {}
                        }
                    }
                }
                if instances.is_instance(self, &vocabulary, node, vocabulary.rdfs_class) {
                    pending.extend(self.objects_of_id(node, vocabulary.sub_class_of));
                    if let Some(target_class) = vocabulary.target_class {
                        pending.extend(
                            quads_for_pattern_ids(
                                data,
                                None,
                                Some(target_class),
                                Some(node),
                                GraphFilter::AnyGraph,
                            )
                            .map(|quad| quad.s),
                        );
                    }
                }
                if instances.is_instance(self, &vocabulary, node, vocabulary.node_shape) {
                    pending.extend(self.objects_of_id(node, vocabulary.node));
                }
            }
            entries.push((term_id_to_native(data, key), properties));
        }
        ClosedTypeIndex::from_entries(entries)
    }

    /// The objects of `(subject, predicate, ?)` in the shapes graph; nothing when
    /// the shapes graph does not intern `predicate`.
    fn objects_of_id(
        &self,
        subject: TermId,
        predicate: Option<TermId>,
    ) -> impl Iterator<Item = TermId> + '_ {
        predicate.into_iter().flat_map(move |predicate| {
            quads_for_pattern_ids(
                self.data,
                Some(subject),
                Some(predicate),
                None,
                GraphFilter::AnyGraph,
            )
            .map(|quad| quad.o)
        })
    }

    /// The subjects of `(?, predicate, ?)` in the shapes graph.
    fn subjects_of(&self, predicate: Option<TermId>) -> impl Iterator<Item = TermId> + '_ {
        predicate.into_iter().flat_map(move |predicate| {
            quads_for_pattern_ids(
                self.data,
                None,
                Some(predicate),
                None,
                GraphFilter::AnyGraph,
            )
            .map(|quad| quad.s)
        })
    }
}

/// SHACL 1.2 Core, "SHACL Instance": "A node n in an RDF graph G is a SHACL instance of a
/// SHACL class C in G if one of the SHACL types of n in G is C", where the SHACL
/// types are the node's `rdf:type` values and their SHACL superclasses
/// (`rdfs:subClassOf+`). Memoized per `(type, class)`, because every node the
/// index visits asks it of the same few classes.
#[derive(Default)]
struct ShaclInstances {
    /// Whether a TYPE reaches a class through `rdfs:subClassOf*`, by
    /// `(type, class)`.
    reaches: FastMap<(TermId, TermId), bool>,
}

impl ShaclInstances {
    /// Whether `node` is a SHACL instance of `class` in the shapes graph; `false`
    /// when the shapes graph does not intern `class`.
    fn is_instance(
        &mut self,
        parser: &Parser<'_>,
        vocabulary: &Vocabulary,
        node: TermId,
        class: Option<TermId>,
    ) -> bool {
        let Some(class) = class else {
            return false;
        };
        parser
            .objects_of_id(node, vocabulary.rdf_type)
            .any(|ty| self.reaches(parser, vocabulary, ty, class))
    }

    /// Whether `ty` is `class` or a SHACL subclass of it.
    fn reaches(
        &mut self,
        parser: &Parser<'_>,
        vocabulary: &Vocabulary,
        ty: TermId,
        class: TermId,
    ) -> bool {
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
            pending.extend(parser.objects_of_id(current, vocabulary.sub_class_of));
        }
        self.reaches.insert((ty, class), found);
        found
    }
}
