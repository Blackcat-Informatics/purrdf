// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrow semantic graph data without materializing terms or warming indexes.
//! ID fields are the original dataset-local ordinals, resolved by `terms`.

use super::super::frozen_terms::FrozenTerms;
use super::{QuadIds, RdfDataset, TermRef};
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError, visit_list};

impl<'error> TermRef<'error> {
    pub(super) fn visit_error_record(
        self,
        name: Option<&'static str>,
        visitor: &mut dyn ErrorVisitor<'error>,
    ) {
        match self {
            Self::Iri(iri) => {
                visitor.begin_record(name, "rdf-interned-iri");
                visitor.value(Some("iri"), ErrorValue::Iri(iri));
            }
            Self::Blank { label, scope } => {
                visitor.begin_record(name, "rdf-interned-blank");
                visitor.value(Some("label"), ErrorValue::Text(label));
                visitor.value(
                    Some("scope"),
                    ErrorValue::Unsigned(u128::from(scope.ordinal())),
                );
            }
            Self::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                visitor.begin_record(name, "rdf-interned-literal");
                visitor.value(Some("lexical_form"), ErrorValue::Text(lexical));
                visitor.value(
                    Some("datatype_id"),
                    ErrorValue::Unsigned(datatype.index() as u128),
                );
                visitor.value(
                    Some("language"),
                    language.map_or(ErrorValue::Absent, ErrorValue::Text),
                );
                visitor.value(
                    Some("direction"),
                    direction.map_or(ErrorValue::Absent, |value| ErrorValue::Code(value.as_str())),
                );
            }
            Self::Triple { s, p, o } => {
                visitor.begin_record(name, "rdf-interned-triple");
                visitor.value(Some("subject_id"), ErrorValue::Unsigned(s.index() as u128));
                visitor.value(
                    Some("predicate_id"),
                    ErrorValue::Unsigned(p.index() as u128),
                );
                visitor.value(Some("object_id"), ErrorValue::Unsigned(o.index() as u128));
            }
        }
        visitor.end_record();
    }
}

fn visit_quad(visitor: &mut dyn ErrorVisitor<'_>, quad: QuadIds) {
    let QuadIds { s, p, o, g } = quad;
    visitor.begin_record(None, "rdf-quad");
    visitor.value(Some("subject_id"), ErrorValue::Unsigned(s.index() as u128));
    visitor.value(
        Some("predicate_id"),
        ErrorValue::Unsigned(p.index() as u128),
    );
    visitor.value(Some("object_id"), ErrorValue::Unsigned(o.index() as u128));
    visitor.value(
        Some("graph_id"),
        g.map_or(ErrorValue::Absent, |id| {
            ErrorValue::Unsigned(id.index() as u128)
        }),
    );
    visitor.end_record();
}

impl StructuredError for FrozenTerms {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        let Self {
            arena: _,
            terms,
            term_index: _,
            content_ids: _,
            content_scheme,
            derivation_predicate,
            derivation_predicate_iri,
        } = self;
        visitor.variant("rdf-dictionary");
        visitor.begin_list(Some("terms"), terms.len());
        for (index, term) in terms.iter().enumerate() {
            visitor.begin_record(None, "rdf-dictionary-entry");
            visitor.value(Some("id"), ErrorValue::Unsigned(index as u128));
            self.resolve_interned(term)
                .visit_error_record(Some("term"), visitor);
            visitor.end_record();
        }
        visitor.end_list();
        visitor.value(
            Some("content_id_scheme"),
            content_scheme
                .as_ref()
                .map_or(ErrorValue::Absent, |scheme| ErrorValue::Record(scheme)),
        );
        visitor.value(
            Some("derivation_predicate_id"),
            derivation_predicate.map_or(ErrorValue::Absent, |id| {
                ErrorValue::Unsigned(id.index() as u128)
            }),
        );
        visitor.value(
            Some("derivation_predicate_iri"),
            derivation_predicate_iri
                .as_deref()
                .map_or(ErrorValue::Absent, ErrorValue::Iri),
        );
    }
}

impl StructuredError for RdfDataset {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        // Derived indexes, statistics and recognition caches are not graph data.
        // Explicitly classify every field so future semantic additions are reviewed.
        let Self {
            dictionary,
            statistics: _,
            snapshot_identity: _,
            quads,
            reifiers,
            reifier_targets: _,
            reifier_components: _,
            annotations,
            annotation_orders: _,
            locations,
            caps,
            indexes: _,
            named_graphs,
            predecessor_index: _,
            statement_graph_rows: _,
        } = self;
        visitor.variant("rdf-dataset");
        visitor.value(Some("dictionary"), ErrorValue::Record(dictionary));
        visitor.begin_list(Some("quads"), quads.len());
        for row in quads {
            visit_quad(
                visitor,
                QuadIds {
                    s: row.s,
                    p: row.p,
                    o: row.o,
                    g: row.g,
                },
            );
        }
        visitor.end_list();
        visitor.begin_list(Some("reifiers"), reifiers.len());
        for &(reifier, triple, graph) in reifiers {
            visitor.begin_record(None, "rdf-reifier");
            visitor.value(
                Some("reifier_id"),
                ErrorValue::Unsigned(reifier.index() as u128),
            );
            visitor.value(
                Some("triple_id"),
                ErrorValue::Unsigned(triple.index() as u128),
            );
            visitor.value(
                Some("graph_id"),
                graph.map_or(ErrorValue::Absent, |id| {
                    ErrorValue::Unsigned(id.index() as u128)
                }),
            );
            visitor.end_record();
        }
        visitor.end_list();
        visitor.begin_list(Some("annotations"), annotations.len());
        for &(s, p, o, g) in annotations {
            visit_quad(visitor, QuadIds { s, p, o, g });
        }
        visitor.end_list();
        visit_list(
            visitor,
            Some("named_graphs"),
            named_graphs
                .iter()
                .map(|id| ErrorValue::Unsigned(id.index() as u128)),
        );
        visitor.begin_list(Some("locations"), locations.len());
        for (handle, location) in locations {
            visitor.begin_record(None, "rdf-location-entry");
            visitor.value(
                Some("quad_ordinal"),
                ErrorValue::Unsigned(handle.index() as u128),
            );
            visitor.value(Some("location"), ErrorValue::Record(location));
            visitor.end_record();
        }
        visitor.end_list();
        visitor.value(Some("capabilities"), ErrorValue::Record(caps));
    }
}
