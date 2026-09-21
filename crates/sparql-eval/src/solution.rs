// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Solution sequences: the column-oriented multiset of variable bindings.
//!
//! A [`SolutionSeq`] is a **bag** (multiset) of [`Solution`] rows over a shared,
//! ordered [`VarSchema`]. Duplicate rows are preserved until `DISTINCT`/`REDUCED`.
//! Each row is a dense, inline-capacity-4 `SmallVec<[Option<SolutionTerm>; 4]>`
//! indexed by column ordinal — see [`Solution`] for why it is not a `Vec`, which
//! matters to anyone reasoning about where a row lives. `None` means the variable
//! is *in the schema's domain but unbound in this row*
//! (which `OPTIONAL`/`UNION` produce), distinct from "not a column at all".
//!
//! Column orientation (rather than per-row hash maps) is deliberate: multiset
//! semantics demand cheap duplicate-preserving rows, `DISTINCT` is a whole-row
//! tuple hash, and join keys are precomputed column ordinals rather than per-probe
//! variable-name lookups.

use std::sync::Arc;

use purrdf_core::{TermId, ViewTermId};
use purrdf_sparql_algebra::Variable;

use crate::DetHashMap;
use crate::scratch::SolutionTerm;

/// The ordered, shared variable schema of a [`SolutionSeq`].
///
/// Maps each [`Variable`] to a stable column ordinal. Column order is significant:
/// it fixes `SELECT` result-column order and the deterministic left-then-right
/// ordering of join outputs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VarSchema {
    /// column ordinal → variable.
    cols: Vec<Variable>,
    /// variable → column ordinal, built only once [`INDEXED_ABOVE`] is exceeded.
    ///
    /// Empty means "not built": below the threshold the ordinal is found by scanning
    /// [`Self::cols`], and a `DetHashMap` that has never had an insert has never
    /// allocated a table.
    index: DetHashMap<Variable, usize>,
}

/// Above this many columns a schema builds a hash index; at or below it, lookups
/// scan the column vector.
///
/// The same argument `crate::substitute`'s `ExprSubs` makes, one layer down and on a
/// hotter path: hashing a string to find one of three entries is slower than
/// comparing three short strings, and it charges a heap allocation for the table.
/// Every `Project`, `Values` and `Bgp` node builds a schema on every execution, so
/// on the SHACL change path that table was allocated several times per focus node
/// for a handful of columns. `Variable` is `Arc<str>`-backed and every column of a
/// given query comes from one parse, so the scan's comparisons are usually pointer
/// equality.
///
/// The threshold is not a tolerance. Above it the index is built and is
/// authoritative, so a wide schema keeps its O(1) lookup; this only decides which
/// representation answers, never what the answer is. Nothing reads `index_of` inside
/// a per-ROW loop — every caller resolves a column once per node — so the scan is
/// not on a quadratic path.
const INDEXED_ABOVE: usize = 8;

impl VarSchema {
    /// An empty schema (zero columns) — the schema of the identity table `Z`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a schema from an ordered iterator of variables, keeping first
    /// occurrence and dropping later duplicates (so the column order is the
    /// variables' first-seen order).
    pub fn from_vars(vars: impl IntoIterator<Item = Variable>) -> Self {
        let mut schema = Self::new();
        for v in vars {
            schema.push(v);
        }
        schema
    }

    /// Append a variable as a new column if absent; return its column ordinal.
    pub fn push(&mut self, var: Variable) -> usize {
        if let Some(i) = self.index_of(&var) {
            return i;
        }
        let ordinal = self.cols.len();
        self.cols.push(var);
        if self.cols.len() > INDEXED_ABOVE {
            if self.index.is_empty() {
                // Crossing the threshold: index everything accumulated so far, this
                // column included, so the map is authoritative from here on.
                self.index.reserve(self.cols.len());
                for (i, col) in self.cols.iter().enumerate() {
                    self.index.insert(col.clone(), i);
                }
            } else {
                self.index.insert(self.cols[ordinal].clone(), ordinal);
            }
        }
        ordinal
    }

    /// The column ordinal of `var`, if it is in the schema.
    #[inline]
    pub fn index_of(&self, var: &Variable) -> Option<usize> {
        if self.index.is_empty() {
            self.cols.iter().position(|col| col == var)
        } else {
            self.index.get(var).copied()
        }
    }

    /// Whether `var` is a column of this schema.
    #[inline]
    pub fn contains(&self, var: &Variable) -> bool {
        self.index_of(var).is_some()
    }

    /// The columns in order.
    #[inline]
    pub fn vars(&self) -> &[Variable] {
        &self.cols
    }

    /// The number of columns.
    #[inline]
    pub fn len(&self) -> usize {
        self.cols.len()
    }

    /// Whether the schema has no columns.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cols.is_empty()
    }

    /// The **ordered union** of two schemas: `self`'s columns first (in order),
    /// then `other`'s columns not already present (in order). This is the result
    /// schema of a binary algebra operator and matches SPARQL's deterministic
    /// variable ordering (left operand's variables lead).
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let mut out = self.clone();
        for v in &other.cols {
            out.push(v.clone());
        }
        out
    }

    /// The shared columns of `self` and `other`, as `(self_ordinal, other_ordinal)`
    /// pairs in `self`'s column order. These are the join key columns and the
    /// columns a compatibility check compares.
    pub fn shared_columns(&self, other: &Self) -> Vec<(usize, usize)> {
        self.cols
            .iter()
            .enumerate()
            .filter_map(|(i, v)| other.index_of(v).map(|j| (i, j)))
            .collect()
    }
}

/// One solution mapping: a dense row indexed by [`VarSchema`] column ordinal.
/// `None` = the variable is unbound in this row.
///
/// A small-vector with inline capacity 4: most queries bind ≤4-8 variables, so
/// the common row lives inline with no heap allocation and spills to the heap
/// only for wider schemas. Derefs to `&[Option<SolutionTerm>]`, so indexing,
/// iteration, slicing, and `&[Option<SolutionTerm>]` parameters are unchanged.
pub type Solution<I = TermId> = smallvec::SmallVec<[Option<SolutionTerm<I>>; 4]>;

/// A multiset (bag) of [`Solution`]s over a shared [`VarSchema`].
///
/// `rows.len()` is the solution cardinality; duplicate rows are preserved
/// (multiset semantics) until an explicit `DISTINCT`/`REDUCED`.
#[derive(Clone, Debug)]
pub struct SolutionSeq<I: ViewTermId = TermId> {
    /// The shared variable schema — the reason a row carries only its cells and no
    /// per-row map of variable names (see [`Solution`] for the row's own storage).
    pub schema: Arc<VarSchema>,
    /// The solution rows (a bag — duplicates significant).
    pub rows: Vec<Solution<I>>,
}

impl VarSchema {
    /// The shared empty schema.
    ///
    /// The empty schema is a constant, and it is reached on every execution: an
    /// empty BGP is the identity table `Z`, and the unit sequence that represents it
    /// seeds every group pattern that starts from nothing. Minting it charged one
    /// heap allocation per call — on the SHACL change path, several per focus node —
    /// for a value that is the same every time.
    ///
    /// Shared by `Arc` rather than cloned, which is sound because nothing anywhere in
    /// the crate reaches a `VarSchema` through `Arc::make_mut` or `Arc::get_mut`: a
    /// schema is built, wrapped, and thereafter only read. This is the same
    /// process-wide-constant device `crate::bgp` uses for the `rdf:reifies` lookup
    /// value and for the single-pattern join order.
    pub fn empty_shared() -> Arc<Self> {
        static EMPTY: std::sync::OnceLock<Arc<VarSchema>> = std::sync::OnceLock::new();
        Arc::clone(EMPTY.get_or_init(|| Arc::new(Self::new())))
    }
}

/// How many distinct column layouts one worker keeps interned.
///
/// A layout is a plan constant — the projected variable list of a `SELECT`, the
/// variables of a `VALUES` block — so a workload's whole vocabulary of layouts is
/// small and bounded by the queries it runs. Reaching the cap clears rather than
/// evicts, because the table memoizes a pure function of the variable list and
/// losing it costs one reconstruction rather than a wrong answer.
const INTERNED_SCHEMA_CAP: usize = 1_024;

/// [`INTERNED_SCHEMAS`]' table: each entry is the column list a layout was built
/// from, beside the layout itself.
type InternedLayouts = hashbrown::HashTable<(Box<[Variable]>, Arc<VarSchema>)>;

thread_local! {
    /// Column layouts, interned per worker and keyed by their CONTENT.
    ///
    /// A `VarSchema` is a pure function of its variable list, and that list is a
    /// plan constant — but the node it belongs to is a fresh heap temporary on every
    /// execution, so a memo keyed by node address would be reading an address that a
    /// later allocation can reuse. Keying by content sidesteps that entirely: the
    /// question "what is the layout for these columns" has the same answer forever,
    /// whoever asks and from whichever node.
    ///
    /// A `hashbrown::HashTable` rather than a `HashMap` so the probe can hash the
    /// caller's BORROWED slice and compare against the stored layout's columns —
    /// owning a key to look one up would be the allocation this exists to remove.
    static INTERNED_SCHEMAS: std::cell::RefCell<InternedLayouts> =
        const { std::cell::RefCell::new(hashbrown::HashTable::new()) };
}

/// The hash of a column layout, as [`INTERNED_SCHEMAS`] keys it.
fn layout_hash(vars: &[Variable]) -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut hasher = crate::DetHasher::default().build_hasher();
    hasher.write_usize(vars.len());
    for var in vars {
        hasher.write(var.as_str().as_bytes());
        hasher.write_u8(0xff);
    }
    hasher.finish()
}

impl VarSchema {
    /// The shared [`VarSchema`] for the column layout `vars`, interned per worker.
    ///
    /// Building one costs a `Vec`, an `Arc`, and — above the index threshold — a hash
    /// table, at every `Project` and `VALUES` node, on every execution. The layout is
    /// a plan constant, so on a path that runs one query per focus node that is the
    /// same three allocations repeated per focus node for an identical answer.
    ///
    /// The layout is stored beside the column list it was built from, and the probe
    /// compares against THAT rather than against the layout's own columns.
    /// [`Self::from_vars`] drops later duplicates, so a request for `?s ?s` yields a
    /// one-column layout — which does not equal the two-column list it was asked
    /// for. Comparing against the stored request keeps such a lookup a hit instead
    /// of a permanent miss that re-inserts on every call. `SELECT ?s ?s` is legal
    /// and the crate has a test for it, so this is a real case and not a defensive
    /// one.
    pub fn interned(vars: &[Variable]) -> Arc<Self> {
        let hash = layout_hash(vars);
        INTERNED_SCHEMAS.with(|table| {
            let mut table = table.borrow_mut();
            if let Some((_, schema)) = table.find(hash, |(key, _)| &**key == vars) {
                return Arc::clone(schema);
            }
            if table.len() >= INTERNED_SCHEMA_CAP {
                table.clear();
            }
            let schema = Arc::new(Self::from_vars(vars.iter().cloned()));
            table.insert_unique(hash, (Box::from(vars), Arc::clone(&schema)), |(key, _)| {
                layout_hash(key)
            });
            schema
        })
    }
}

impl<I: ViewTermId> SolutionSeq<I> {
    /// An empty sequence over `schema` (zero solutions).
    pub fn empty(schema: Arc<VarSchema>) -> Self {
        Self {
            schema,
            rows: Vec::new(),
        }
    }

    /// The **unit** sequence: one solution that binds nothing (the algebra identity
    /// table `Z`, i.e. the result of the empty BGP). Joining with `Z` is the
    /// identity, so this is the correct seed for an empty group pattern.
    pub fn unit() -> Self {
        Self {
            schema: VarSchema::empty_shared(),
            rows: vec![Solution::new()],
        }
    }

    /// The number of solutions (multiset cardinality).
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the sequence has no solutions.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Whether two solutions are **compatible** over their shared columns: every
/// variable bound in *both* must be bound to the same [`SolutionTerm`]. A variable
/// unbound (`None`) in either row is compatible with anything.
///
/// `shared` is the precomputed `(a_ordinal, b_ordinal)` column pairing (see
/// [`VarSchema::shared_columns`]). This is the predicate underlying `Join`,
/// `LeftJoin`, and `Minus`.
#[must_use]
pub fn compatible<I: ViewTermId>(
    a: &[Option<SolutionTerm<I>>],
    b: &[Option<SolutionTerm<I>>],
    shared: &[(usize, usize)],
) -> bool {
    shared.iter().all(|&(ia, ib)| match (a[ia], b[ib]) {
        (Some(x), Some(y)) => x == y,
        // `None` (unbound) is compatible with anything.
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use purrdf_core::TermId;

    fn var(name: &str) -> Variable {
        Variable::new(name)
    }

    fn term(i: u32) -> Option<SolutionTerm> {
        Some(SolutionTerm::Existing(TermId::from_index(i)))
    }

    /// **The two lookup representations answer identically, on both sides of the
    /// threshold and across the crossing.**
    ///
    /// [`INDEXED_ABOVE`] chooses between scanning the column vector and consulting a
    /// hash index. That choice is invisible only if both answer the same for every
    /// column and every non-column, so this drives a schema one variable at a time
    /// from empty to well past the threshold and checks every ordinal after each
    /// push — including the ordinals assigned BEFORE the index existed, which the
    /// crossing has to carry over rather than renumber.
    #[test]
    fn schema_lookup_agrees_on_both_sides_of_the_index_threshold() {
        let names: Vec<String> = (0..INDEXED_ABOVE * 3).map(|i| format!("v{i}")).collect();
        let mut schema = VarSchema::new();
        for (expected_ordinal, name) in names.iter().enumerate() {
            assert_eq!(schema.push(var(name)), expected_ordinal);
            assert!(
                schema.len() <= INDEXED_ABOVE || !schema.index.is_empty(),
                "a schema wider than the threshold must have built its index"
            );
            assert!(
                schema.len() > INDEXED_ABOVE || schema.index.is_empty(),
                "a schema at or below the threshold must not have allocated an index"
            );
            // Every column placed so far, including those numbered before the
            // crossing, still resolves to the ordinal it was given.
            for (ordinal, seen) in names[..=expected_ordinal].iter().enumerate() {
                assert_eq!(
                    schema.index_of(&var(seen)),
                    Some(ordinal),
                    "column {seen} moved at width {}",
                    schema.len()
                );
                assert!(schema.contains(&var(seen)));
            }
            assert_eq!(schema.index_of(&var("absent")), None);
            assert!(!schema.contains(&var("absent")));
        }
    }

    /// A repeated variable is still deduplicated once the index is authoritative.
    #[test]
    fn schema_dedups_above_the_index_threshold() {
        let wide: Vec<Variable> = (0..INDEXED_ABOVE * 2)
            .map(|i| var(&format!("v{i}")))
            .collect();
        let mut schema = VarSchema::from_vars(wide.clone());
        let width = schema.len();
        assert!(
            width > INDEXED_ABOVE,
            "the fixture must cross the threshold"
        );
        for (ordinal, v) in wide.iter().enumerate() {
            assert_eq!(
                schema.push(v.clone()),
                ordinal,
                "re-push must not add a column"
            );
        }
        assert_eq!(schema.len(), width);
    }

    #[test]
    fn schema_dedups_preserving_first_seen_order() {
        let s = VarSchema::from_vars([var("a"), var("b"), var("a"), var("c")]);
        assert_eq!(s.len(), 3);
        assert_eq!(s.vars(), &[var("a"), var("b"), var("c")]);
        assert_eq!(s.index_of(&var("b")), Some(1));
        assert_eq!(s.index_of(&var("z")), None);
    }

    #[test]
    fn schema_union_is_ordered_left_then_new_right() {
        let left = VarSchema::from_vars([var("a"), var("b")]);
        let right = VarSchema::from_vars([var("b"), var("c"), var("d")]);
        let u = left.union(&right);
        // left's columns first, then right's not-already-present, in order.
        assert_eq!(u.vars(), &[var("a"), var("b"), var("c"), var("d")]);
    }

    #[test]
    fn shared_columns_pairs_by_ordinal() {
        let left = VarSchema::from_vars([var("a"), var("b"), var("c")]);
        let right = VarSchema::from_vars([var("c"), var("a")]);
        // shared in LEFT order: a(0)~right1, c(2)~right0.
        assert_eq!(left.shared_columns(&right), vec![(0, 1), (2, 0)]);
    }

    #[test]
    fn compatible_requires_equality_on_shared_bound_columns() {
        let shared = [(0usize, 0usize)];
        assert!(compatible(&[term(1)], &[term(1)], &shared));
        assert!(!compatible(&[term(1)], &[term(2)], &shared));
        // None is compatible with anything.
        assert!(compatible(&[None], &[term(2)], &shared));
        assert!(compatible(&[term(1)], &[None], &shared));
        assert!(compatible::<TermId>(&[None], &[None], &shared));
    }

    #[test]
    fn unit_sequence_has_one_empty_solution() {
        let z = SolutionSeq::<TermId>::unit();
        assert_eq!(z.len(), 1);
        assert!(z.schema.is_empty());
        assert!(z.rows[0].is_empty());
    }
}
