// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
/// Returns `Some(n)` iff `iri` is exactly `rdf:_<n>` with `<n>` a decimal for a
/// **positive** integer with **no leading zeros** (the W3C container-membership
/// contract): `rdf:_1`, `rdf:_12`, … parse; `rdf:_0`, `rdf:_01`, the bare
/// `rdf:_`, a non-numeric or overflowing suffix, or an unrelated IRI yield `None`.
pub(crate) fn container_member_index(iri: &str) -> Option<u64> {
    let suffix = iri.strip_prefix(RDF_MEMBER_PREFIX)?;
    // A leading zero (`_0`, `_01`, …) is not a well-formed membership ordinal, and
    // the empty suffix (`rdf:_`) does not start with '0' but fails to parse below.
    if suffix.starts_with('0') {
        return None;
    }
    let n = suffix.parse::<u64>().ok()?;
    (n >= 1).then_some(n)
}

/// A malformed RDF Collection encountered while walking `rdf:first`/`rdf:rest`.
///
/// The list walker ([`DatasetView::rdf_list`](crate::DatasetView::rdf_list)) is
/// also a validator: under its default policy cycles terminate gracefully, but a
/// structurally broken cons cell is a hard error carrying which invariant it
/// violated. Which faults are errors is the caller's [`RdfListPolicy`]; the
/// variants here name every fault a policy can turn into an error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RdfListError {
    /// A cons cell (a term on the `rdf:rest` chain) carries no `rdf:first` object.
    MissingFirst,
    /// A cons cell carries more than one `rdf:first` object (ambiguous head).
    MultipleFirst,
    /// An `rdf:rest` edge points at a term that is neither `rdf:nil` nor a cons
    /// cell (a term with no `rdf:first`/`rdf:rest`) — a dangling tail.
    DanglingRest,
    /// A cons cell carries no `rdf:rest` edge — a truncated tail (an error only
    /// under a policy whose [`missing_rest`](RdfListPolicy::missing_rest) says so).
    MissingRest,
    /// A cons cell carries more than one `rdf:rest` edge (ambiguous tail).
    MultipleRest,
    /// The `rdf:rest` chain revisits a cell (an error only under a policy whose
    /// [`cycle`](RdfListPolicy::cycle) says so).
    Cycle,
}

impl std::fmt::Display for RdfListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::MissingFirst => "rdf:List cons cell missing an rdf:first object",
            Self::MultipleFirst => "rdf:List cons cell with multiple rdf:first objects",
            Self::DanglingRest => {
                "rdf:List rdf:rest points at a term that is neither rdf:nil nor a cons cell"
            }
            Self::MissingRest => "rdf:List cons cell missing an rdf:rest edge",
            Self::MultipleRest => "rdf:List cons cell with multiple rdf:rest edges",
            Self::Cycle => "rdf:List rdf:rest chain revisits a cons cell",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for RdfListError {}

/// What the list walker does when it meets one structural fault.
///
/// Not every variant applies to every fault: a cycle has nothing to take "first",
/// and a cell with two `rdf:first` edges has nothing to skip. Each
/// [`RdfListPolicy`] field documents the variants it admits; a variant a field does
/// not admit is treated as [`Error`](Self::Error) for that fault — the strictest
/// outcome, never a silent one — and [`RdfListPolicy::is_well_formed`] says so up
/// front.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListFault {
    /// The walk stops and returns the matching [`RdfListError`].
    Error,
    /// The walk ends at the fault and returns what it has: every member read before
    /// the faulty cell. When the fault is in the cell's `rdf:rest` (missing or
    /// multiple), the cell's own unambiguous `rdf:first` member is kept, since the
    /// fault lies past it; when it is in the cell's `rdf:first` (missing or
    /// multiple) or the cell is a revisit, nothing of the cell is kept.
    StopBefore,
    /// Of several edges, the first one the view yields is taken. For the `multiple_*`
    /// faults only.
    FirstWins,
    /// Of several edges, the last one the view yields is taken. For the `multiple_*`
    /// faults only. This is the tail the walker silently followed for a multiple
    /// `rdf:rest` before the policy existed.
    LastWins,
    /// The cell contributes no member and the walk follows its `rdf:rest`. For
    /// `missing_first` only.
    Skip,
}

/// What the list walker does at each structural fault of an `rdf:first`/`rdf:rest`
/// chain — the argument of [`DatasetView::rdf_list_with`](crate::DatasetView::rdf_list_with).
///
/// Faults are judged per cell in `rdf:first` then `rdf:rest` order, so a cell that is
/// faulty in both reports (or stops at) its `rdf:first` fault. A head that is
/// `rdf:nil` or is not a list at all (carries neither edge) yields an empty list
/// under every policy, and an `rdf:rest` pointing at a term that is neither
/// `rdf:nil` nor a cons cell is [`RdfListError::DanglingRest`] under every policy:
/// that is not a fault a walker can read past, since there is no cell to continue
/// from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RdfListPolicy {
    /// The `rdf:rest` chain revisits a cell. Admits [`ListFault::Error`] and
    /// [`ListFault::StopBefore`].
    pub cycle: ListFault,
    /// A cons cell has no `rdf:first`. Admits [`ListFault::Error`],
    /// [`ListFault::StopBefore`] and [`ListFault::Skip`].
    pub missing_first: ListFault,
    /// A cons cell has more than one `rdf:first`. Admits [`ListFault::Error`],
    /// [`ListFault::StopBefore`], [`ListFault::FirstWins`] and [`ListFault::LastWins`].
    pub multiple_first: ListFault,
    /// A cons cell has no `rdf:rest`. Admits [`ListFault::Error`] and
    /// [`ListFault::StopBefore`].
    pub missing_rest: ListFault,
    /// A cons cell has more than one `rdf:rest`. Admits [`ListFault::Error`],
    /// [`ListFault::StopBefore`], [`ListFault::FirstWins`] and [`ListFault::LastWins`].
    pub multiple_rest: ListFault,
}

impl RdfListPolicy {
    /// Every fault is an error: the walker is a validator and nothing malformed is
    /// read past.
    pub const STRICT: Self = Self {
        cycle: ListFault::Error,
        missing_first: ListFault::Error,
        multiple_first: ListFault::Error,
        missing_rest: ListFault::Error,
        multiple_rest: ListFault::Error,
    };

    /// What [`DatasetView::rdf_list`](crate::DatasetView::rdf_list) walks by: a
    /// cycle and a missing `rdf:rest` end the list gracefully (the reference GTS
    /// walker's behaviour), a missing or multiple `rdf:first` is an error, and a
    /// multiple `rdf:rest` is an error too — before the policy existed the walker
    /// silently followed the last such edge the view yielded, which is
    /// [`ListFault::LastWins`] for a caller that wants it back.
    pub const CORE_DEFAULT: Self = Self {
        cycle: ListFault::StopBefore,
        missing_first: ListFault::Error,
        multiple_first: ListFault::Error,
        missing_rest: ListFault::StopBefore,
        multiple_rest: ListFault::Error,
    };

    /// Whether every field holds a variant it admits (see the field docs). A policy
    /// that does not is still walked — each inadmissible variant acts as
    /// [`ListFault::Error`] — but it is a programming error, and the walker asserts
    /// this in debug builds.
    #[must_use]
    pub const fn is_well_formed(&self) -> bool {
        matches!(self.cycle, ListFault::Error | ListFault::StopBefore)
            && matches!(
                self.missing_first,
                ListFault::Error | ListFault::StopBefore | ListFault::Skip
            )
            && matches!(
                self.multiple_first,
                ListFault::Error
                    | ListFault::StopBefore
                    | ListFault::FirstWins
                    | ListFault::LastWins
            )
            && matches!(self.missing_rest, ListFault::Error | ListFault::StopBefore)
            && matches!(
                self.multiple_rest,
                ListFault::Error
                    | ListFault::StopBefore
                    | ListFault::FirstWins
                    | ListFault::LastWins
            )
    }
}

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
        // W3C: the ordinal is a positive integer with no leading zeros.
        assert_eq!(container_member_index(&format!("{RDF_NS}_12")), Some(12));
        assert_eq!(container_member_index(&format!("{RDF_NS}_0")), None);
        assert_eq!(container_member_index(&format!("{RDF_NS}_01")), None);
        assert_eq!(container_member_index(&format!("{RDF_NS}_007")), None);
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
        assert_eq!(
            ds.rdf_list(nil, GraphMatch::Any).expect("nil"),
            [] as [_; 0]
        );
        assert_eq!(
            ds.rdf_list(plain, GraphMatch::Any).expect("plain"),
            [] as [_; 0]
        );
        assert_eq!(
            ds.rdf_list(lit, GraphMatch::Any).expect("literal"),
            [] as [_; 0]
        );
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
        assert_eq!(
            ds.members(plain, GraphMatch::Any).expect("neither"),
            [] as [_; 0]
        );
    }

    /// The member a cons cell holds, for reading a walk's answer back as members
    /// when the choice between two tails is the thing under test.
    fn member_of(ds: &crate::RdfDataset, cell: TermId) -> TermId {
        let first = ds
            .term_id_by_value(&crate::TermValue::iri(RDF_FIRST))
            .expect("rdf:first is interned");
        ds.first_object(cell, first, GraphMatch::Any)
            .expect("a cons cell has a member")
    }

    /// A cell with two `rdf:rest` edges: an error under `rdf_list`, `CORE_DEFAULT`
    /// and `STRICT`; the first edge the view yields under `FirstWins`, the last under
    /// `LastWins` (two DIFFERENT tails, so an honoured choice is distinguishable
    /// from a dropped one); and the list ends after the cell's own member under
    /// `StopBefore`.
    #[test]
    fn multiple_rest_is_judged_by_the_policy() {
        let mut b = RdfDatasetBuilder::new();
        let first = rdf(&mut b, "first");
        let rest = rdf(&mut b, "rest");
        let nil = rdf(&mut b, "nil");
        let a = iri(&mut b, "a");
        let bb = iri(&mut b, "b");
        let c = iri(&mut b, "c");
        let head = b.intern_blank("head", BlankScope::DEFAULT);
        let tail_b = b.intern_blank("tail_b", BlankScope::DEFAULT);
        let tail_c = b.intern_blank("tail_c", BlankScope::DEFAULT);
        b.push_quad(head, first, a, None);
        b.push_quad(head, rest, tail_b, None);
        b.push_quad(head, rest, tail_c, None);
        b.push_quad(tail_b, first, bb, None);
        b.push_quad(tail_b, rest, nil, None);
        b.push_quad(tail_c, first, c, None);
        b.push_quad(tail_c, rest, nil, None);
        let ds = b.freeze().expect("freeze");

        assert_eq!(
            ds.rdf_list(head, GraphMatch::Any),
            Err(RdfListError::MultipleRest)
        );
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, RdfListPolicy::CORE_DEFAULT),
            Err(RdfListError::MultipleRest)
        );
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, RdfListPolicy::STRICT),
            Err(RdfListError::MultipleRest)
        );

        // "First" and "last" are by the view's own yield order, which is what the
        // policy promises; the two tails differ, so the two answers must too.
        let tails: Vec<TermId> = ds
            .quads_for_pattern(Some(head), Some(rest), None, GraphMatch::Any)
            .map(|q| q.o)
            .collect();
        assert_eq!(tails.len(), 2);
        let first_wins = RdfListPolicy {
            multiple_rest: ListFault::FirstWins,
            ..RdfListPolicy::CORE_DEFAULT
        };
        let last_wins = RdfListPolicy {
            multiple_rest: ListFault::LastWins,
            ..RdfListPolicy::CORE_DEFAULT
        };
        let stop = RdfListPolicy {
            multiple_rest: ListFault::StopBefore,
            ..RdfListPolicy::CORE_DEFAULT
        };
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, first_wins),
            Ok(vec![a, member_of(&ds, tails[0])])
        );
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, last_wins),
            Ok(vec![a, member_of(&ds, tails[1])])
        );
        assert_ne!(
            ds.rdf_list_with(head, GraphMatch::Any, first_wins),
            ds.rdf_list_with(head, GraphMatch::Any, last_wins),
            "the two choices name different tails"
        );
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, stop),
            Ok(vec![a]),
            "the cell's own member is kept; the ambiguous tail is not followed"
        );
    }

    /// A cell with two `rdf:first` edges: `FirstWins` and `LastWins` take different
    /// members, `StopBefore` ends the list before the ambiguous cell.
    #[test]
    fn multiple_first_is_judged_by_the_policy() {
        let mut b = RdfDatasetBuilder::new();
        let first = rdf(&mut b, "first");
        let rest = rdf(&mut b, "rest");
        let nil = rdf(&mut b, "nil");
        let a = iri(&mut b, "a");
        let bb = iri(&mut b, "b");
        let z = iri(&mut b, "z");
        let head = b.intern_blank("head", BlankScope::DEFAULT);
        let tail = b.intern_blank("tail", BlankScope::DEFAULT);
        b.push_quad(head, first, z, None);
        b.push_quad(head, rest, tail, None);
        b.push_quad(tail, first, a, None);
        b.push_quad(tail, first, bb, None); // two rdf:first — ambiguous
        b.push_quad(tail, rest, nil, None);
        let ds = b.freeze().expect("freeze");
        let members: Vec<TermId> = ds
            .quads_for_pattern(Some(tail), Some(first), None, GraphMatch::Any)
            .map(|q| q.o)
            .collect();
        assert_eq!(members.len(), 2);
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, RdfListPolicy::STRICT),
            Err(RdfListError::MultipleFirst)
        );
        let first_wins = RdfListPolicy {
            multiple_first: ListFault::FirstWins,
            ..RdfListPolicy::CORE_DEFAULT
        };
        let last_wins = RdfListPolicy {
            multiple_first: ListFault::LastWins,
            ..RdfListPolicy::CORE_DEFAULT
        };
        let stop = RdfListPolicy {
            multiple_first: ListFault::StopBefore,
            ..RdfListPolicy::CORE_DEFAULT
        };
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, first_wins),
            Ok(vec![z, members[0]])
        );
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, last_wins),
            Ok(vec![z, members[1]])
        );
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, stop),
            Ok(vec![z]),
            "the ambiguous cell contributes nothing"
        );
    }

    /// A cell with no `rdf:first`: an error under `STRICT` (and `CORE_DEFAULT`), a
    /// cell that contributes nothing under `Skip` — the tail after it is still
    /// walked, which is what tells a skip from a stop — and the end of the list under
    /// `StopBefore`.
    #[test]
    fn missing_first_is_an_error_under_strict_and_skipped_under_skip() {
        let mut b = RdfDatasetBuilder::new();
        let first = rdf(&mut b, "first");
        let rest = rdf(&mut b, "rest");
        let nil = rdf(&mut b, "nil");
        let a = iri(&mut b, "a");
        let bb = iri(&mut b, "b");
        let head = b.intern_blank("head", BlankScope::DEFAULT);
        let gap = b.intern_blank("gap", BlankScope::DEFAULT);
        let tail = b.intern_blank("tail", BlankScope::DEFAULT);
        b.push_quad(head, first, a, None);
        b.push_quad(head, rest, gap, None);
        b.push_quad(gap, rest, tail, None); // no rdf:first
        b.push_quad(tail, first, bb, None);
        b.push_quad(tail, rest, nil, None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, RdfListPolicy::STRICT),
            Err(RdfListError::MissingFirst)
        );
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, RdfListPolicy::CORE_DEFAULT),
            Err(RdfListError::MissingFirst)
        );
        let skip = RdfListPolicy {
            missing_first: ListFault::Skip,
            ..RdfListPolicy::CORE_DEFAULT
        };
        let stop = RdfListPolicy {
            missing_first: ListFault::StopBefore,
            ..RdfListPolicy::CORE_DEFAULT
        };
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, skip),
            Ok(vec![a, bb])
        );
        assert_eq!(ds.rdf_list_with(head, GraphMatch::Any, stop), Ok(vec![a]));
    }

    /// A cycle: truncated at the revisited cell under `StopBefore` (the default),
    /// refused under `Error` (`STRICT`).
    #[test]
    fn a_cycle_stops_under_stop_before_and_errs_under_error() {
        let mut b = RdfDatasetBuilder::new();
        let first = rdf(&mut b, "first");
        let rest = rdf(&mut b, "rest");
        let a = iri(&mut b, "a");
        let bb = iri(&mut b, "b");
        let c0 = b.intern_blank("c0", BlankScope::DEFAULT);
        let c1 = b.intern_blank("c1", BlankScope::DEFAULT);
        b.push_quad(c0, first, a, None);
        b.push_quad(c0, rest, c1, None);
        b.push_quad(c1, first, bb, None);
        b.push_quad(c1, rest, c0, None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            ds.rdf_list_with(c0, GraphMatch::Any, RdfListPolicy::CORE_DEFAULT),
            Ok(vec![a, bb])
        );
        assert_eq!(
            ds.rdf_list_with(c0, GraphMatch::Any, RdfListPolicy::STRICT),
            Err(RdfListError::Cycle)
        );
        let error_only_on_cycle = RdfListPolicy {
            cycle: ListFault::Error,
            ..RdfListPolicy::CORE_DEFAULT
        };
        assert_eq!(
            ds.rdf_list_with(c0, GraphMatch::Any, error_only_on_cycle),
            Err(RdfListError::Cycle)
        );
    }

    /// A cons cell with no `rdf:rest`: the list ends after its member by default,
    /// and `STRICT` refuses it. The dataset also holds a proper list so the
    /// `rdf:rest` vocabulary is present and the fault is the cell's, not the view's.
    #[test]
    fn missing_rest_ends_the_list_by_default_and_errs_under_strict() {
        let mut b = RdfDatasetBuilder::new();
        let first = rdf(&mut b, "first");
        let a = iri(&mut b, "a");
        let bb = iri(&mut b, "b");
        let proper = build_list(&mut b, &[bb], None);
        let head = b.intern_blank("head", BlankScope::DEFAULT);
        b.push_quad(head, first, a, None); // no rdf:rest
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, RdfListPolicy::CORE_DEFAULT),
            Ok(vec![a])
        );
        assert_eq!(
            ds.rdf_list_with(head, GraphMatch::Any, RdfListPolicy::STRICT),
            Err(RdfListError::MissingRest)
        );
        // The valid neighbour under the strict policy: the proper list is unaffected.
        assert_eq!(
            ds.rdf_list_with(proper, GraphMatch::Any, RdfListPolicy::STRICT),
            Ok(vec![bb])
        );
    }

    /// The valid neighbour of every refusal above: a well-formed three-item list
    /// answers the same under every preset and under the most lenient policy.
    #[test]
    fn a_well_formed_list_answers_the_same_under_every_policy() {
        let mut b = RdfDatasetBuilder::new();
        let a = iri(&mut b, "a");
        let bb = iri(&mut b, "b");
        let c = iri(&mut b, "c");
        let head = build_list(&mut b, &[a, bb, c], None);
        let ds = b.freeze().expect("freeze");
        let lenient = RdfListPolicy {
            cycle: ListFault::StopBefore,
            missing_first: ListFault::Skip,
            multiple_first: ListFault::LastWins,
            missing_rest: ListFault::StopBefore,
            multiple_rest: ListFault::FirstWins,
        };
        for policy in [RdfListPolicy::STRICT, RdfListPolicy::CORE_DEFAULT, lenient] {
            assert_eq!(
                ds.rdf_list_with(head, GraphMatch::Any, policy),
                Ok(vec![a, bb, c]),
                "{policy:?}"
            );
        }
        assert_eq!(ds.rdf_list(head, GraphMatch::Any), Ok(vec![a, bb, c]));
    }

    /// The presets admit only the variants their fields document; a misplaced
    /// variant is reported rather than silently reinterpreted.
    #[test]
    fn presets_are_well_formed_and_a_misplaced_variant_is_not() {
        assert!(RdfListPolicy::STRICT.is_well_formed());
        assert!(RdfListPolicy::CORE_DEFAULT.is_well_formed());
        for bad in [
            RdfListPolicy {
                cycle: ListFault::FirstWins,
                ..RdfListPolicy::STRICT
            },
            RdfListPolicy {
                missing_first: ListFault::LastWins,
                ..RdfListPolicy::STRICT
            },
            RdfListPolicy {
                multiple_first: ListFault::Skip,
                ..RdfListPolicy::STRICT
            },
            RdfListPolicy {
                missing_rest: ListFault::Skip,
                ..RdfListPolicy::STRICT
            },
            RdfListPolicy {
                multiple_rest: ListFault::Skip,
                ..RdfListPolicy::STRICT
            },
        ] {
            assert!(!bad.is_well_formed(), "{bad:?}");
        }
        for variant in [
            RdfListError::MissingRest,
            RdfListError::MultipleRest,
            RdfListError::Cycle,
        ] {
            assert!(variant.to_string().contains("rdf:"), "{variant:?}");
        }
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
        assert_eq!(
            ds.rdf_list(head, GraphMatch::Default)
                .expect("default empty"),
            [] as [_; 0]
        );
    }
}
