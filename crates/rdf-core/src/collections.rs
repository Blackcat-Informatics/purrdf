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

/// `rdf:Alt` — an alternatives Container class.
pub(crate) use purrdf_iri::vocab::rdf::ALT as RDF_ALT;
/// `rdf:Bag` — an unordered Container class.
pub(crate) use purrdf_iri::vocab::rdf::BAG as RDF_BAG;
/// `rdf:first` — the head edge of a Collection cons cell.
pub(crate) use purrdf_iri::vocab::rdf::FIRST as RDF_FIRST;
/// `rdf:nil` — the empty-list / list terminator resource.
pub(crate) use purrdf_iri::vocab::rdf::NIL as RDF_NIL;
/// `rdf:rest` — the tail edge of a Collection cons cell.
pub(crate) use purrdf_iri::vocab::rdf::REST as RDF_REST;
/// `rdf:Seq` — an ordered Container class.
pub(crate) use purrdf_iri::vocab::rdf::SEQ as RDF_SEQ;
/// `rdf:type` — used to recognize a typed Container.
pub(crate) use purrdf_iri::vocab::rdf::TYPE as RDF_TYPE;

/// The `rdf:_<n>` container-membership property prefix (`rdf:_1`, `rdf:_2`, …).
use purrdf_iri::vocab::rdf::MEMBER_PREFIX as RDF_MEMBER_PREFIX;

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
/// also a validator: cycles terminate gracefully, but a structurally broken cons
/// cell is a hard error carrying which invariant it violated.
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
    /// A cons cell carries more than one `rdf:rest` object (ambiguous tail).
    MultipleRest,
}

impl std::fmt::Display for RdfListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::MissingFirst => "rdf:List cons cell missing an rdf:first object",
            Self::MultipleFirst => "rdf:List cons cell with multiple rdf:first objects",
            Self::DanglingRest => {
                "rdf:List rdf:rest points at a term that is neither rdf:nil nor a cons cell"
            }
            Self::MultipleRest => "rdf:List cons cell with multiple rdf:rest objects",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for RdfListError {}

/// Which invariant of an RDF Collection the strict walker
/// ([`DatasetView::rdf_list_strict`](crate::DatasetView::rdf_list_strict))
/// found broken, at [`ListError::node`].
///
/// RDF 1.2 Semantics §D.3 and SHACL 1.2 Core §1.4 define a well-formed list:
/// every cell has exactly one `rdf:first` and exactly one `rdf:rest`, the
/// `rdf:rest` chain reaches `rdf:nil` without revisiting a cell, and `rdf:nil`
/// itself has neither edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ListErrorKind {
    /// The `rdf:rest` chain returns to `node`, a cell already walked.
    Cycle,
    /// `node` is on the chain (the head, or an `rdf:rest` object that is not
    /// `rdf:nil`) and has no `rdf:first`.
    MissingFirst,
    /// `node` has more than one distinct `rdf:first` object.
    MultipleFirst,
    /// `node` has an `rdf:first` and no `rdf:rest`: the chain never ends.
    MissingRest,
    /// `node` has more than one distinct `rdf:rest` object.
    MultipleRest,
    /// `node` is `rdf:nil` and carries an `rdf:first` or `rdf:rest`.
    NonEmptyNil,
}

impl ListErrorKind {
    const fn describe(self) -> &'static str {
        match self {
            Self::Cycle => "its rdf:rest chain returns to a cell already walked",
            Self::MissingFirst => "a cell on its rdf:rest chain has no rdf:first",
            Self::MultipleFirst => "a cell has more than one rdf:first",
            Self::MissingRest => "a cell has no rdf:rest",
            Self::MultipleRest => "a cell has more than one rdf:rest",
            Self::NonEmptyNil => "rdf:nil carries an rdf:first or rdf:rest",
        }
    }
}

impl std::fmt::Display for ListErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.describe())
    }
}

/// A malformed RDF Collection: what was broken, where, and every member read
/// before the walk stopped.
///
/// `members` holds, in list order, the `rdf:first` object of every cell that
/// had exactly one. For [`ListErrorKind::MissingRest`] and
/// [`ListErrorKind::MultipleRest`] that includes `node`'s own member; for
/// [`ListErrorKind::Cycle`] it is each distinct cell's member once, the list a
/// reader that stops at the revisited cell would see. A caller whose
/// specification recovers from a variant (truncating at a cycle, say) reads
/// the recovered list from here instead of walking again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListError<Id = crate::ir::TermId> {
    /// What was broken.
    pub kind: ListErrorKind,
    /// The members read before the walk stopped (see the type docs).
    pub members: Vec<Id>,
    /// The cell (or `rdf:nil`) where the walk stopped.
    pub node: Id,
}

impl<Id: std::fmt::Debug> std::fmt::Display for ListError<Id> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "malformed rdf:List: {} (at {:?}, after {} member(s))",
            self.kind.describe(),
            self.node,
            self.members.len()
        )
    }
}

impl<Id: std::fmt::Debug> std::error::Error for ListError<Id> {}

/// How many distinct objects one `(cell, predicate, ?)` edge has — the one
/// question the strict walker asks of an edge source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoleObject<Id> {
    /// No object.
    None,
    /// Exactly one distinct object.
    One(Id),
    /// Two or more distinct objects.
    Many,
}

impl<Id: Copy + Eq> SoleObject<Id> {
    /// Fold `objects` (repeats allowed: a statement asserted in several graphs
    /// is one object) into how many distinct ones there are, stopping at the
    /// second distinct one.
    pub fn of(objects: impl IntoIterator<Item = Id>) -> Self {
        let mut found = Self::None;
        for object in objects {
            found = found.and(object);
            if found == Self::Many {
                break;
            }
        }
        found
    }

    /// The count after one more object: a repeat of the one already seen
    /// changes nothing. For a source that reports objects through a callback
    /// rather than an iterator.
    #[must_use]
    pub fn and(self, object: Id) -> Self {
        match self {
            Self::None => Self::One(object),
            Self::One(existing) if existing != object => Self::Many,
            same => same,
        }
    }
}

/// The members of the RDF Collection headed by `head`, in list order, over any
/// edge source — the strict walker behind
/// [`DatasetView::rdf_list_strict`](crate::DatasetView::rdf_list_strict).
///
/// `first(cell)` and `rest(cell)` answer how many distinct `rdf:first` and
/// `rdf:rest` objects `cell` has, and `nil` is `rdf:nil`'s identifier (`None`
/// when the source holds no such term, so no chain can end). A source that is
/// not a single [`GraphMatch`](crate::GraphMatch) — a merge of graphs, or a
/// buffer of statements minted during a query — asks the same question here, so
/// every reader of a collection shares one definition of a well-formed list
/// (RDF 1.2 Semantics §D.3, SHACL 1.2 Core §1.4): exactly one `rdf:first` and
/// one `rdf:rest` per cell, a chain reaching `rdf:nil` without revisiting a
/// cell, and `rdf:nil` carrying neither edge.
///
/// The cycle check allocates nothing: Brent's algorithm keeps one saved cell
/// and compares each step against it, so a cycle is found within twice the
/// walked length and a well-formed list costs no lookup beyond its own edges.
///
/// # Errors
///
/// [`ListError`] when the collection is malformed; see [`ListErrorKind`].
pub fn walk_rdf_list<Id: Copy + Eq>(
    head: Id,
    nil: Option<Id>,
    mut first: impl FnMut(Id) -> SoleObject<Id>,
    mut rest: impl FnMut(Id) -> SoleObject<Id>,
) -> Result<Vec<Id>, ListError<Id>> {
    let mut members = Vec::new();
    let fail = |kind, members, node| {
        Err(ListError {
            kind,
            members,
            node,
        })
    };
    // Brent: `saved` is compared against every cell; it jumps forward to the
    // current cell whenever `steps` reaches `power`, which doubles.
    let (mut saved, mut power, mut steps) = (head, 1_usize, 0_usize);
    let mut cell = head;
    loop {
        if Some(cell) == nil {
            if first(cell) != SoleObject::None || rest(cell) != SoleObject::None {
                return fail(ListErrorKind::NonEmptyNil, members, cell);
            }
            return Ok(members);
        }
        match first(cell) {
            SoleObject::None => return fail(ListErrorKind::MissingFirst, members, cell),
            SoleObject::Many => return fail(ListErrorKind::MultipleFirst, members, cell),
            SoleObject::One(member) => members.push(member),
        }
        let next = match rest(cell) {
            SoleObject::None => return fail(ListErrorKind::MissingRest, members, cell),
            SoleObject::Many => return fail(ListErrorKind::MultipleRest, members, cell),
            SoleObject::One(next) => next,
        };
        steps += 1;
        if next == saved {
            // `steps` is now the cycle's length: find where it starts (two
            // cursors `steps` apart meet at the first repeated cell) and keep
            // each distinct cell's member once.
            let (prefix, entry) = cycle_start(head, steps, &mut rest);
            members.truncate(prefix + steps);
            return fail(ListErrorKind::Cycle, members, entry);
        }
        if steps == power {
            saved = next;
            power *= 2;
            steps = 0;
        }
        cell = next;
    }
}

/// Where a cycle of length `length` on the `rdf:rest` chain from `head`
/// begins: the number of cells before it, and its first cell. Two cursors
/// `length` cells apart meet exactly there. Every cell they visit was already
/// validated by the walk that found the cycle, so each has one `rdf:rest`.
fn cycle_start<Id: Copy + Eq>(
    head: Id,
    length: usize,
    rest: &mut impl FnMut(Id) -> SoleObject<Id>,
) -> (usize, Id) {
    let mut step = |cell: Id| match rest(cell) {
        SoleObject::One(next) => next,
        SoleObject::None | SoleObject::Many => {
            unreachable!("the walk validated every cell on the cycle")
        }
    };
    let mut ahead = head;
    for _ in 0..length {
        ahead = step(ahead);
    }
    let (mut behind, mut prefix) = (head, 0);
    while behind != ahead {
        behind = step(behind);
        ahead = step(ahead);
        prefix += 1;
    }
    (prefix, behind)
}

/// The three terms an RDF Collection is written with, in a caller's term
/// type: `rdf:first`, `rdf:rest` and `rdf:nil`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListVocab<T> {
    /// `rdf:first`.
    pub first: T,
    /// `rdf:rest`.
    pub rest: T,
    /// `rdf:nil`.
    pub nil: T,
}

impl ListVocab<crate::ir::TermValue> {
    /// The three terms as owned [`TermValue`](crate::ir::TermValue) IRIs.
    #[must_use]
    pub fn term_values() -> Self {
        use crate::ir::TermValue;
        Self {
            first: TermValue::iri(RDF_FIRST),
            rest: TermValue::iri(RDF_REST),
            nil: TermValue::iri(RDF_NIL),
        }
    }
}

impl ListVocab<crate::model::RdfTerm> {
    /// The three terms as owned-model [`RdfTerm`](crate::model::RdfTerm) IRIs.
    #[must_use]
    pub fn rdf_terms() -> Self {
        use crate::model::RdfTerm;
        Self {
            first: RdfTerm::iri(RDF_FIRST),
            rest: RdfTerm::iri(RDF_REST),
            nil: RdfTerm::iri(RDF_NIL),
        }
    }
}

/// Write `members` as an RDF Collection and return its head: `rdf:nil` for
/// no members, else the first cell.
///
/// `cell(i)` mints the `i`-th cell (a blank node or an IRI, in the caller's
/// scheme), called once per member in order; `emit(subject, predicate,
/// object)` receives, per cell, its `rdf:first` statement and then its
/// `rdf:rest` statement, whose object is the next cell or `rdf:nil`. The
/// caller links the head from wherever the list hangs, and adds any further
/// statements about the cells (an `rdf:type rdf:List`, say) in `emit`.
///
/// Generic over the term type, so one construction serves a builder keyed by
/// ids, an owned-term quad list and a CONSTRUCT buffer alike.
///
/// ```
/// use purrdf_core::collections::{ListVocab, build_rdf_list};
///
/// let vocab = ListVocab { first: "first", rest: "rest", nil: "nil" };
/// let mut triples = Vec::new();
/// let cells = ["c0", "c1"];
/// let head = build_rdf_list(["a", "b"], &vocab, |i| cells[i], |s, p, o| triples.push((s, p, o)));
/// assert_eq!(head, "c0");
/// assert_eq!(
///     triples,
///     [("c0", "first", "a"), ("c0", "rest", "c1"), ("c1", "first", "b"), ("c1", "rest", "nil")]
/// );
/// let empty = build_rdf_list(Vec::<&str>::new(), &vocab, |i| cells[i], |_, _, _| unreachable!());
/// assert_eq!(empty, "nil");
/// ```
pub fn build_rdf_list<T, I>(
    members: I,
    vocab: &ListVocab<T>,
    mut cell: impl FnMut(usize) -> T,
    mut emit: impl FnMut(T, T, T),
) -> T
where
    T: Clone,
    I: IntoIterator<Item = T>,
{
    let mut members = members.into_iter().peekable();
    if members.peek().is_none() {
        return vocab.nil.clone();
    }
    let head = cell(0);
    let mut current = head.clone();
    let mut index = 0;
    while let Some(member) = members.next() {
        emit(current.clone(), vocab.first.clone(), member);
        let next = if members.peek().is_some() {
            index += 1;
            cell(index)
        } else {
            vocab.nil.clone()
        };
        emit(current, vocab.rest.clone(), next.clone());
        current = next;
    }
    head
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{RdfDatasetBuilder, TermId};
    use crate::model::RdfLiteral;
    use crate::{BlankScope, DatasetView, GraphMatch};

    /// The RDF namespace (`rdf:`) prefix, for building fixture IRIs.
    use purrdf_iri::vocab::rdf::NS as RDF_NS;

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

    fn strict(ds: &crate::ir::RdfDataset, head: TermId) -> Result<Vec<TermId>, ListError> {
        ds.rdf_list_strict(head, GraphMatch::Any)
    }

    #[test]
    fn two_rdf_rest_edges_are_refused() {
        let mut b = RdfDatasetBuilder::new();
        let (first, rest, nil) = (
            rdf(&mut b, "first"),
            rdf(&mut b, "rest"),
            rdf(&mut b, "nil"),
        );
        let a = iri(&mut b, "a");
        let head = b.intern_blank("head", BlankScope::DEFAULT);
        let other = b.intern_blank("other", BlankScope::DEFAULT);
        b.push_quad(head, first, a, None);
        b.push_quad(head, rest, nil, None);
        b.push_quad(head, rest, other, None);
        b.push_quad(other, first, a, None);
        b.push_quad(other, rest, nil, None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            strict(&ds, head),
            Err(ListError {
                kind: ListErrorKind::MultipleRest,
                members: vec![a],
                node: head,
            })
        );
        // The lenient reading refuses it too rather than picking one tail.
        assert_eq!(
            ds.rdf_list(head, GraphMatch::Any),
            Err(RdfListError::MultipleRest)
        );
    }

    #[test]
    fn a_well_formed_three_element_list_walks() {
        let mut b = RdfDatasetBuilder::new();
        let (a, bb, c) = (iri(&mut b, "a"), iri(&mut b, "b"), iri(&mut b, "c"));
        let head = build_list(&mut b, &[a, bb, c], None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(strict(&ds, head), Ok(vec![a, bb, c]));
    }

    #[test]
    fn an_rdf_nil_head_is_the_empty_list() {
        let mut b = RdfDatasetBuilder::new();
        let nil = rdf(&mut b, "nil");
        let ds = b.freeze().expect("freeze");
        assert_eq!(strict(&ds, nil), Ok(Vec::new()));
    }

    #[test]
    fn cells_typed_rdf_list_and_annotated_still_walk() {
        let mut b = RdfDatasetBuilder::new();
        let (a, bb) = (iri(&mut b, "a"), iri(&mut b, "b"));
        let head = build_list(&mut b, &[a, bb], None);
        let (first, ty, list) = (
            rdf(&mut b, "first"),
            rdf(&mut b, "type"),
            rdf(&mut b, "List"),
        );
        let note = iri(&mut b, "note");
        let reifier = iri(&mut b, "reifier");
        let text = b.intern_literal(RdfLiteral::simple("the first member"));
        b.push_quad(head, ty, list, None);
        b.push_quad(head, note, text, None);
        let statement = b.intern_triple(head, first, a);
        b.push_reifier(reifier, statement);
        b.push_annotation(reifier, note, text);
        let ds = b.freeze().expect("freeze");
        assert_eq!(strict(&ds, head), Ok(vec![a, bb]));
    }

    #[test]
    fn iri_named_cells_walk() {
        let mut b = RdfDatasetBuilder::new();
        let (first, rest, nil) = (
            rdf(&mut b, "first"),
            rdf(&mut b, "rest"),
            rdf(&mut b, "nil"),
        );
        let (a, bb) = (iri(&mut b, "a"), iri(&mut b, "b"));
        let (c0, c1) = (iri(&mut b, "cell0"), iri(&mut b, "cell1"));
        b.push_quad(c0, first, a, None);
        b.push_quad(c0, rest, c1, None);
        b.push_quad(c1, first, bb, None);
        b.push_quad(c1, rest, nil, None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(strict(&ds, c0), Ok(vec![a, bb]));
    }

    #[test]
    fn literal_and_triple_term_members_walk() {
        let mut b = RdfDatasetBuilder::new();
        let literal = b.intern_literal(RdfLiteral::simple("x"));
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let triple = b.intern_triple(s, p, o);
        let head = build_list(&mut b, &[literal, triple], None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(strict(&ds, head), Ok(vec![literal, triple]));
    }

    #[test]
    fn a_statement_in_several_graphs_is_one_edge() {
        let mut b = RdfDatasetBuilder::new();
        let (first, rest, nil) = (
            rdf(&mut b, "first"),
            rdf(&mut b, "rest"),
            rdf(&mut b, "nil"),
        );
        let a = iri(&mut b, "a");
        let (g1, g2) = (iri(&mut b, "g1"), iri(&mut b, "g2"));
        let head = b.intern_blank("head", BlankScope::DEFAULT);
        for g in [g1, g2] {
            b.push_quad(head, first, a, Some(g));
            b.push_quad(head, rest, nil, Some(g));
        }
        let ds = b.freeze().expect("freeze");
        assert_eq!(strict(&ds, head), Ok(vec![a]));
        assert_eq!(ds.objects(head, first, GraphMatch::Any), vec![a]);
        assert_eq!(ds.objects(head, first, GraphMatch::Named(g1)), vec![a]);
        assert_eq!(ds.objects(head, first, GraphMatch::Default), Vec::new());
    }

    #[test]
    fn a_cycle_is_refused_with_each_cell_read_once() {
        let mut b = RdfDatasetBuilder::new();
        let (first, rest) = (rdf(&mut b, "first"), rdf(&mut b, "rest"));
        let members: Vec<TermId> = (0..7).map(|i| iri(&mut b, &format!("m{i}"))).collect();
        let cells: Vec<TermId> = (0..7)
            .map(|i| b.intern_blank(&format!("c{i}"), BlankScope::DEFAULT))
            .collect();
        // c0 → … → c6 → c2: a prefix of two cells and a cycle of five.
        for i in 0..7 {
            b.push_quad(cells[i], first, members[i], None);
            let next = if i == 6 { cells[2] } else { cells[i + 1] };
            b.push_quad(cells[i], rest, next, None);
        }
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            strict(&ds, cells[0]),
            Err(ListError {
                kind: ListErrorKind::Cycle,
                members,
                node: cells[2],
            })
        );
        // Every length of prefix and cycle is found with the right entry.
        for prefix in 0..5_usize {
            for cycle in 1..9_usize {
                let mut b = RdfDatasetBuilder::new();
                let (first, rest) = (rdf(&mut b, "first"), rdf(&mut b, "rest"));
                let total = prefix + cycle;
                let cells: Vec<TermId> = (0..total)
                    .map(|i| b.intern_blank(&format!("c{i}"), BlankScope::DEFAULT))
                    .collect();
                let m = iri(&mut b, "m");
                for i in 0..total {
                    b.push_quad(cells[i], first, m, None);
                    let next = if i + 1 == total {
                        cells[prefix]
                    } else {
                        cells[i + 1]
                    };
                    b.push_quad(cells[i], rest, next, None);
                }
                let ds = b.freeze().expect("freeze");
                let error = strict(&ds, cells[0]).expect_err("a cycle");
                assert_eq!(error.kind, ListErrorKind::Cycle);
                assert_eq!(error.node, cells[prefix], "{prefix} {cycle}");
                assert_eq!(error.members.len(), total, "{prefix} {cycle}");
            }
        }
    }

    #[test]
    fn missing_edges_are_refused_and_the_error_names_the_cell() {
        let mut b = RdfDatasetBuilder::new();
        let (first, rest) = (rdf(&mut b, "first"), rdf(&mut b, "rest"));
        let a = iri(&mut b, "a");
        let no_rest = b.intern_blank("no_rest", BlankScope::DEFAULT);
        b.push_quad(no_rest, first, a, None);
        let plain = iri(&mut b, "plain");
        let to_plain = b.intern_blank("to_plain", BlankScope::DEFAULT);
        b.push_quad(to_plain, first, a, None);
        b.push_quad(to_plain, rest, plain, None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            strict(&ds, no_rest),
            Err(ListError {
                kind: ListErrorKind::MissingRest,
                members: vec![a],
                node: no_rest,
            })
        );
        assert_eq!(
            strict(&ds, to_plain),
            Err(ListError {
                kind: ListErrorKind::MissingFirst,
                members: vec![a],
                node: plain,
            })
        );
        // The lenient reading keeps its recoveries.
        assert_eq!(ds.rdf_list(no_rest, GraphMatch::Any), Ok(vec![a]));
        assert_eq!(
            ds.rdf_list(to_plain, GraphMatch::Any),
            Err(RdfListError::DanglingRest)
        );
        assert_eq!(
            strict(&ds, plain).map_err(|e| e.kind),
            Err(ListErrorKind::MissingFirst)
        );
        assert_eq!(ds.rdf_list(plain, GraphMatch::Any), Ok(Vec::new()));
        let _ = rest;
    }

    #[test]
    fn an_rdf_nil_with_edges_is_refused_and_a_bare_one_is_read() {
        let mut b = RdfDatasetBuilder::new();
        let (first, nil) = (rdf(&mut b, "first"), rdf(&mut b, "nil"));
        let a = iri(&mut b, "a");
        let head = build_list(&mut b, &[a], None);
        b.push_quad(nil, first, a, None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(
            strict(&ds, head),
            Err(ListError {
                kind: ListErrorKind::NonEmptyNil,
                members: vec![a],
                node: nil,
            })
        );
        let mut b = RdfDatasetBuilder::new();
        let a = iri(&mut b, "a");
        let head = build_list(&mut b, &[a], None);
        let ds = b.freeze().expect("freeze");
        assert_eq!(strict(&ds, head), Ok(vec![a]));
    }

    #[test]
    fn a_built_list_walks_back_to_its_members() {
        let mut b = RdfDatasetBuilder::new();
        let vocab = ListVocab {
            first: rdf(&mut b, "first"),
            rest: rdf(&mut b, "rest"),
            nil: rdf(&mut b, "nil"),
        };
        let members: Vec<TermId> = (0..4).map(|i| iri(&mut b, &format!("m{i}"))).collect();
        let cells: Vec<TermId> = (0..4)
            .map(|i| b.intern_blank(&format!("cell{i}"), BlankScope::DEFAULT))
            .collect();
        let mut quads = Vec::new();
        let head = build_rdf_list(
            members.iter().copied(),
            &vocab,
            |i| cells[i],
            |s, p, o| {
                quads.push((s, p, o));
            },
        );
        assert_eq!(head, cells[0]);
        assert_eq!(quads.len(), 8);
        for (s, p, o) in quads {
            b.push_quad(s, p, o, None);
        }
        let empty = build_rdf_list(
            Vec::new(),
            &vocab,
            |_| unreachable!(),
            |_, _, _| unreachable!(),
        );
        assert_eq!(empty, vocab.nil);
        let ds = b.freeze().expect("freeze");
        assert_eq!(strict(&ds, head), Ok(members));
        assert_eq!(strict(&ds, empty), Ok(Vec::new()));
    }

    #[test]
    fn a_list_of_owned_terms_is_built_with_the_rdf_vocabulary() {
        use crate::ir::TermValue;
        let vocab = ListVocab::term_values();
        let mut triples = Vec::new();
        let head = build_rdf_list(
            [TermValue::simple_literal("x")],
            &vocab,
            |i| TermValue::Blank {
                label: format!("c{i}"),
                scope: BlankScope::DEFAULT,
            },
            |s, p, o| triples.push((s, p, o)),
        );
        assert_eq!(
            triples,
            [
                (
                    head.clone(),
                    TermValue::iri(RDF_FIRST),
                    TermValue::simple_literal("x")
                ),
                (head, TermValue::iri(RDF_REST), TermValue::iri(RDF_NIL)),
            ]
        );
    }

    /// The walker over a plain statement slice: the edge source a merged graph
    /// scope or a per-query buffer supplies.
    fn walk_slice(edges: &[(u8, u8, u8)], head: u8) -> Result<Vec<u8>, ListError<u8>> {
        const FIRST: u8 = 100;
        const REST: u8 = 101;
        const NIL: u8 = 0;
        let objects = |cell: u8, predicate: u8| {
            SoleObject::of(
                edges
                    .iter()
                    .filter(|(s, p, _)| *s == cell && *p == predicate)
                    .map(|&(_, _, o)| o),
            )
        };
        walk_rdf_list(
            head,
            Some(NIL),
            |cell| objects(cell, FIRST),
            |cell| objects(cell, REST),
        )
    }

    #[test]
    fn the_walker_reads_any_edge_source_and_counts_a_repeated_edge_once() {
        let list = [
            (1, 100, 7),
            (1, 101, 2),
            (2, 100, 8),
            (2, 101, 0),
            (1, 100, 7),
        ];
        assert_eq!(walk_slice(&list, 1), Ok(vec![7, 8]));
        assert_eq!(walk_slice(&list, 0), Ok(vec![]));

        let two_rests = [(1, 100, 7), (1, 101, 0), (1, 101, 2)];
        let error = walk_slice(&two_rests, 1).expect_err("two rdf:rest objects");
        assert_eq!(error.kind, ListErrorKind::MultipleRest);
        assert_eq!((error.node, error.members), (1, vec![7]));

        let cycle = [(1, 100, 7), (1, 101, 2), (2, 100, 8), (2, 101, 1)];
        let error = walk_slice(&cycle, 1).expect_err("a cycle");
        assert_eq!(error.kind, ListErrorKind::Cycle);
        assert_eq!(error.members, vec![7, 8]);

        let busy_nil = [(1, 100, 7), (1, 101, 0), (0, 100, 9)];
        let error = walk_slice(&busy_nil, 1).expect_err("rdf:nil with an edge");
        assert_eq!(error.kind, ListErrorKind::NonEmptyNil);
    }

    #[test]
    fn sole_object_counts_distinct_objects_up_to_two() {
        assert_eq!(SoleObject::<u8>::of([]), SoleObject::None);
        assert_eq!(SoleObject::of([3, 3, 3]), SoleObject::One(3));
        assert_eq!(SoleObject::of([3, 4, 3]), SoleObject::Many);
        assert_eq!(SoleObject::None.and(1).and(1), SoleObject::One(1));
        assert_eq!(SoleObject::One(1).and(2).and(1), SoleObject::Many);
    }
}
