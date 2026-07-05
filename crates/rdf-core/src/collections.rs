// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RDF Collection (`rdf:first`/`rdf:rest`/`rdf:nil`) and Container
//! (`rdf:Seq`/`rdf:Bag`/`rdf:Alt` with `rdf:_1`, `rdf:_2`, …) traversal over a
//! [`DatasetView`](crate::DatasetView).
//!
//! This module holds the shared standard-`rdf:` IRI const set, the membership
//! property parser, and the malformed-list error taxonomy. The traversal methods
//! themselves are **provided** methods on [`DatasetView`](crate::DatasetView)
//! (`rdf_list`, `rdf_container_members`, `members`) so every backend inherits one
//! id-native, graph-scoped, cycle-guarded, validating walker — no per-backend copy.

/// `rdf:first` — the head edge of a Collection cons cell.
pub(crate) const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
/// `rdf:rest` — the tail edge of a Collection cons cell.
pub(crate) const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
/// `rdf:nil` — the empty-list / list terminator resource.
pub(crate) const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
/// `rdf:type` — used to recognize a typed Container.
pub(crate) const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// `rdf:Seq` — an ordered Container class.
pub(crate) const RDF_SEQ: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Seq";
/// `rdf:Bag` — an unordered Container class.
pub(crate) const RDF_BAG: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Bag";
/// `rdf:Alt` — an alternatives Container class.
pub(crate) const RDF_ALT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Alt";

/// The `rdf:_<n>` container-membership property prefix (`rdf:_1`, `rdf:_2`, …).
const RDF_MEMBER_PREFIX: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#_";

/// Parse the numeric suffix of an `rdf:_<n>` container-membership property IRI.
///
/// Returns `Some(n)` iff `iri` is exactly `rdf:_<n>` with `<n>` a non-empty run of
/// ASCII digits parsing into a `u64` (the ordinal sort key). Any other IRI — the
/// bare `rdf:_`, a non-numeric or overflowing suffix, or an unrelated IRI — yields
/// `None`.
pub(crate) fn container_member_index(iri: &str) -> Option<u64> {
    let suffix = iri.strip_prefix(RDF_MEMBER_PREFIX)?;
    // `u64::parse` already rejects the empty string, a sign, and any non-digit
    // byte, so it is exactly the `rdf:_<n>` acceptance test.
    suffix.parse::<u64>().ok()
}

/// A malformed RDF Collection encountered while walking `rdf:first`/`rdf:rest`.
///
/// The list walker ([`DatasetView::rdf_list`](crate::DatasetView::rdf_list)) is
/// also a validator: cycles terminate gracefully, but a structurally broken cons
/// cell is a hard error carrying which invariant it violated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdfListError {
    /// A cons cell (a term on the `rdf:rest` chain) carries no `rdf:first` object.
    MissingFirst,
    /// A cons cell carries more than one `rdf:first` object (ambiguous head).
    MultipleFirst,
    /// An `rdf:rest` edge points at a term that is neither `rdf:nil` nor a cons
    /// cell (a term with no `rdf:first`/`rdf:rest`) — a dangling tail.
    DanglingRest,
}

impl std::fmt::Display for RdfListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::MissingFirst => "rdf:List cons cell missing an rdf:first object",
            Self::MultipleFirst => "rdf:List cons cell with multiple rdf:first objects",
            Self::DanglingRest => {
                "rdf:List rdf:rest points at a term that is neither rdf:nil nor a cons cell"
            }
        };
        f.write_str(msg)
    }
}

impl std::error::Error for RdfListError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{RdfDatasetBuilder, TermId};
    use crate::model::RdfLiteral;
    use crate::{BlankScope, DatasetView, GraphMatch};

    /// The RDF namespace (`rdf:`) prefix, for building fixture IRIs.
    const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(&format!("http://example.org/{n}"))
    }

    fn rdf(b: &mut RdfDatasetBuilder, local: &str) -> TermId {
        b.intern_iri(&format!("{RDF_NS}{local}"))
    }

    #[test]
    fn container_member_index_parses_suffix() {
        assert_eq!(container_member_index(RDF_FIRST), None);
        assert_eq!(container_member_index(&format!("{RDF_NS}_1")), Some(1));
        assert_eq!(container_member_index(&format!("{RDF_NS}_42")), Some(42));
        assert_eq!(container_member_index(&format!("{RDF_NS}_")), None);
        assert_eq!(container_member_index(&format!("{RDF_NS}_x")), None);
        assert_eq!(container_member_index("http://example.org/_1"), None);
    }

    #[test]
    fn rdf_list_error_display_and_error() {
        // Display renders a distinct message per variant and the type is an Error.
        let e: &dyn std::error::Error = &RdfListError::MissingFirst;
        assert!(e.to_string().contains("rdf:first"));
        assert_ne!(
            RdfListError::MissingFirst.to_string(),
            RdfListError::MultipleFirst.to_string()
        );
        assert!(RdfListError::DanglingRest.to_string().contains("rdf:rest"));
    }

    /// Build a proper Collection `( members… )` in `graph`, returning its head cell
    /// id. Each cons cell is a fresh blank node under a caller-unique `tag` prefix so
    /// separate lists never share a cell.
    fn build_list_tagged(
        b: &mut RdfDatasetBuilder,
        tag: &str,
        members: &[TermId],
        graph: Option<TermId>,
    ) -> TermId {
        let first = rdf(b, "first");
        let rest = rdf(b, "rest");
        let nil = rdf(b, "nil");
        // Fold from the tail so each cell's rest is already built.
        let mut tail = nil;
        for (i, &m) in members.iter().enumerate().rev() {
            let cell = b.intern_blank(&format!("{tag}_cell{i}"), BlankScope::DEFAULT);
            b.push_quad(cell, first, m, graph);
            b.push_quad(cell, rest, tail, graph);
            tail = cell;
        }
        tail
    }

    /// A single-list convenience wrapper over [`build_list_tagged`].
    fn build_list(b: &mut RdfDatasetBuilder, members: &[TermId], graph: Option<TermId>) -> TermId {
        build_list_tagged(b, "l", members, graph)
    }

    #[test]
    fn ordered_collection_in_order() {
        let mut b = RdfDatasetBuilder::new();
        let a = iri(&mut b, "a");
        let bb = iri(&mut b, "b");
        let c = iri(&mut b, "c");
        let head = build_list(&mut b, &[a, bb, c], None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            ds.rdf_list(head, GraphMatch::Any).expect("well-formed"),
            vec![a, bb, c]
        );
    }

    #[test]
    fn nested_and_blank_members_walk() {
        let mut b = RdfDatasetBuilder::new();
        let x = iri(&mut b, "x");
        let blank = b.intern_blank("member", BlankScope::DEFAULT);
        // Inner list ( x ), then outer ( _:member ( x ) ). Distinct tags keep the
        // two lists' cons cells disjoint.
        let inner = build_list_tagged(&mut b, "inner", &[x], None);
        let outer = build_list_tagged(&mut b, "outer", &[blank, inner], None);
        let ds = b.freeze().expect("freeze");
        let outer_members = ds.rdf_list(outer, GraphMatch::Any).expect("well-formed");
        assert_eq!(outer_members, vec![blank, inner]);
        // The nested list head itself walks to its own single member.
        assert_eq!(
            ds.rdf_list(inner, GraphMatch::Any).expect("well-formed"),
            vec![x]
        );
    }

    #[test]
    fn cyclic_rest_terminates_truncated() {
        let mut b = RdfDatasetBuilder::new();
        let first = rdf(&mut b, "first");
        let rest = rdf(&mut b, "rest");
        let a = iri(&mut b, "a");
        let bb = iri(&mut b, "b");
        let c0 = b.intern_blank("c0", BlankScope::DEFAULT);
        let c1 = b.intern_blank("c1", BlankScope::DEFAULT);
        // c0 -> a -> c1 -> b -> back to c0 (a rest cycle).
        b.push_quad(c0, first, a, None);
        b.push_quad(c0, rest, c1, None);
        b.push_quad(c1, first, bb, None);
        b.push_quad(c1, rest, c0, None);
        let ds = b.freeze().expect("freeze");
        // Terminates (no infinite loop), truncated at the revisited cell.
        assert_eq!(
            ds.rdf_list(c0, GraphMatch::Any).expect("cycle is graceful"),
            vec![a, bb]
        );
    }

    #[test]
    fn self_cycle_at_head_terminates() {
        let mut b = RdfDatasetBuilder::new();
        let first = rdf(&mut b, "first");
        let rest = rdf(&mut b, "rest");
        let a = iri(&mut b, "a");
        let head = b.intern_blank("head", BlankScope::DEFAULT);
        b.push_quad(head, first, a, None);
        b.push_quad(head, rest, head, None); // rest points at itself
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            ds.rdf_list(head, GraphMatch::Any).expect("graceful"),
            vec![a]
        );
    }

    #[test]
    fn malformed_missing_first_is_error() {
        let mut b = RdfDatasetBuilder::new();
        let first = rdf(&mut b, "first");
        let rest = rdf(&mut b, "rest");
        let nil = rdf(&mut b, "nil");
        let a = iri(&mut b, "a");
        // A well-formed cell (so the rdf:first vocabulary is present in the dataset)…
        let good = b.intern_blank("good", BlankScope::DEFAULT);
        b.push_quad(good, first, a, None);
        b.push_quad(good, rest, nil, None);
        // …and the cell under test: an rdf:rest but no rdf:first.
        let head = b.intern_blank("head", BlankScope::DEFAULT);
        b.push_quad(head, rest, nil, None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            ds.rdf_list(head, GraphMatch::Any),
            Err(RdfListError::MissingFirst)
        );
    }

    #[test]
    fn malformed_multiple_first_is_error() {
        let mut b = RdfDatasetBuilder::new();
        let first = rdf(&mut b, "first");
        let rest = rdf(&mut b, "rest");
        let nil = rdf(&mut b, "nil");
        let a = iri(&mut b, "a");
        let bb = iri(&mut b, "b");
        let head = b.intern_blank("head", BlankScope::DEFAULT);
        b.push_quad(head, first, a, None);
        b.push_quad(head, first, bb, None); // two rdf:first — ambiguous
        b.push_quad(head, rest, nil, None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            ds.rdf_list(head, GraphMatch::Any),
            Err(RdfListError::MultipleFirst)
        );
    }

    #[test]
    fn malformed_dangling_rest_is_error() {
        let mut b = RdfDatasetBuilder::new();
        let first = rdf(&mut b, "first");
        let rest = rdf(&mut b, "rest");
        let a = iri(&mut b, "a");
        let dangling = iri(&mut b, "dangling"); // a plain IRI, not nil, not a cell
        let head = b.intern_blank("head", BlankScope::DEFAULT);
        b.push_quad(head, first, a, None);
        b.push_quad(head, rest, dangling, None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            ds.rdf_list(head, GraphMatch::Any),
            Err(RdfListError::DanglingRest)
        );
    }

    #[test]
    fn terminator_taxonomy_empty() {
        let mut b = RdfDatasetBuilder::new();
        let nil = rdf(&mut b, "nil");
        let plain = iri(&mut b, "plain"); // an IRI with no list structure
        let lit = b.intern_literal(RdfLiteral::simple("hello"));
        // Give the dataset the list vocabulary so the walker's IRIs resolve.
        let first = rdf(&mut b, "first");
        let some_cell = b.intern_blank("c", BlankScope::DEFAULT);
        b.push_quad(some_cell, first, plain, None);
        let ds = b.freeze().expect("freeze");
        assert!(ds.rdf_list(nil, GraphMatch::Any).expect("nil").is_empty());
        assert!(ds
            .rdf_list(plain, GraphMatch::Any)
            .expect("plain")
            .is_empty());
        assert!(ds
            .rdf_list(lit, GraphMatch::Any)
            .expect("literal")
            .is_empty());
    }

    #[test]
    fn container_members_numeric_order_with_gap() {
        let mut b = RdfDatasetBuilder::new();
        let type_p = rdf(&mut b, "type");
        let seq = rdf(&mut b, "Seq");
        let m1 = rdf(&mut b, "_1");
        let m2 = rdf(&mut b, "_2");
        let m3 = rdf(&mut b, "_3");
        let x = iri(&mut b, "x");
        let y = iri(&mut b, "y");
        let z = iri(&mut b, "z");
        let bag = b.intern_blank("container", BlankScope::DEFAULT);
        b.push_quad(bag, type_p, seq, None);
        // Insert out of order and with a gap in the ordinals.
        b.push_quad(bag, m1, x, None);
        b.push_quad(bag, m3, z, None);
        b.push_quad(bag, m2, y, None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            ds.rdf_container_members(bag, GraphMatch::Any),
            vec![x, y, z]
        );
    }

    #[test]
    fn members_dispatch_collection_vs_container() {
        let mut b = RdfDatasetBuilder::new();
        let a = iri(&mut b, "a");
        let bb = iri(&mut b, "b");
        let list_head = build_list(&mut b, &[a, bb], None);

        let type_p = rdf(&mut b, "type");
        let bag_class = rdf(&mut b, "Bag");
        let m1 = rdf(&mut b, "_1");
        let x = iri(&mut b, "x");
        let container = b.intern_blank("container", BlankScope::DEFAULT);
        b.push_quad(container, type_p, bag_class, None);
        b.push_quad(container, m1, x, None);

        // A term that is neither a list nor a container.
        let plain = iri(&mut b, "plain");

        let ds = b.freeze().expect("freeze");
        assert_eq!(
            ds.members(list_head, GraphMatch::Any).expect("collection"),
            vec![a, bb]
        );
        assert_eq!(
            ds.members(container, GraphMatch::Any).expect("container"),
            vec![x]
        );
        assert!(ds
            .members(plain, GraphMatch::Any)
            .expect("neither")
            .is_empty());
    }

    #[test]
    fn graph_scoping_isolates_lists() {
        let mut b = RdfDatasetBuilder::new();
        let g = iri(&mut b, "g");
        let a = iri(&mut b, "a");
        // A list that lives ONLY in the named graph g.
        let head = build_list(&mut b, &[a], Some(g));
        let ds = b.freeze().expect("freeze");
        // Visible when scoped to g (or Any), invisible from the default graph.
        assert_eq!(
            ds.rdf_list(head, GraphMatch::Named(g)).expect("named"),
            vec![a]
        );
        assert_eq!(ds.rdf_list(head, GraphMatch::Any).expect("any"), vec![a]);
        assert!(ds
            .rdf_list(head, GraphMatch::Default)
            .expect("default empty")
            .is_empty());
    }
}
