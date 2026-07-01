// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Solution sequences: the column-oriented multiset of variable bindings.
//!
//! A [`SolutionSeq`] is a **bag** (multiset) of [`Solution`] rows over a shared,
//! ordered [`VarSchema`]. Duplicate rows are preserved until `DISTINCT`/`REDUCED`.
//! Each row is a dense `Vec<Option<SolutionTerm>>` indexed by column ordinal —
//! `None` means the variable is *in the schema's domain but unbound in this row*
//! (which `OPTIONAL`/`UNION` produce), distinct from "not a column at all".
//!
//! Column orientation (rather than per-row hash maps) is deliberate: multiset
//! semantics demand cheap duplicate-preserving rows, `DISTINCT` is a whole-row
//! tuple hash, and join keys are precomputed column ordinals rather than per-probe
//! variable-name lookups.

use std::rc::Rc;

use purrdf_sparql_algebra::Variable;

use crate::scratch::SolutionTerm;
use crate::DetHashMap;

/// The ordered, shared variable schema of a [`SolutionSeq`].
///
/// Maps each [`Variable`] to a stable column ordinal. Column order is significant:
/// it fixes `SELECT` result-column order and the deterministic left-then-right
/// ordering of join outputs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VarSchema {
    /// column ordinal → variable.
    cols: Vec<Variable>,
    /// variable → column ordinal.
    index: DetHashMap<Variable, usize>,
}

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
        if let Some(&i) = self.index.get(&var) {
            return i;
        }
        let i = self.cols.len();
        self.index.insert(var.clone(), i);
        self.cols.push(var);
        i
    }

    /// The column ordinal of `var`, if it is in the schema.
    #[inline]
    pub fn index_of(&self, var: &Variable) -> Option<usize> {
        self.index.get(var).copied()
    }

    /// Whether `var` is a column of this schema.
    #[inline]
    pub fn contains(&self, var: &Variable) -> bool {
        self.index.contains_key(var)
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
    pub fn union(&self, other: &VarSchema) -> VarSchema {
        let mut out = self.clone();
        for v in &other.cols {
            out.push(v.clone());
        }
        out
    }

    /// The shared columns of `self` and `other`, as `(self_ordinal, other_ordinal)`
    /// pairs in `self`'s column order. These are the join key columns and the
    /// columns a compatibility check compares.
    pub fn shared_columns(&self, other: &VarSchema) -> Vec<(usize, usize)> {
        self.cols
            .iter()
            .enumerate()
            .filter_map(|(i, v)| other.index_of(v).map(|j| (i, j)))
            .collect()
    }
}

/// One solution mapping: a dense row indexed by [`VarSchema`] column ordinal.
/// `None` = the variable is unbound in this row.
pub type Solution = Vec<Option<SolutionTerm>>;

/// A multiset (bag) of [`Solution`]s over a shared [`VarSchema`].
///
/// `rows.len()` is the solution cardinality; duplicate rows are preserved
/// (multiset semantics) until an explicit `DISTINCT`/`REDUCED`.
#[derive(Clone, Debug)]
pub struct SolutionSeq {
    /// The shared variable schema (so a row is just a `Vec<Option<SolutionTerm>>`).
    pub schema: Rc<VarSchema>,
    /// The solution rows (a bag — duplicates significant).
    pub rows: Vec<Solution>,
}

impl SolutionSeq {
    /// An empty sequence over `schema` (zero solutions).
    pub fn empty(schema: Rc<VarSchema>) -> Self {
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
            schema: Rc::new(VarSchema::new()),
            rows: vec![Vec::new()],
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
pub fn compatible(
    a: &[Option<SolutionTerm>],
    b: &[Option<SolutionTerm>],
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
        assert!(compatible(&[None], &[None], &shared));
    }

    #[test]
    fn unit_sequence_has_one_empty_solution() {
        let z = SolutionSeq::unit();
        assert_eq!(z.len(), 1);
        assert!(z.schema.is_empty());
        assert!(z.rows[0].is_empty());
    }
}
