// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A harness relation that declares a **restricted** access pattern.
//!
//! [`MemoryRelation`] declares the all-free mode,
//! which subsumes every access pattern of its arity — so a suite built only out of
//! memory relations can never reach the parts of the seam that exist *because* a
//! relation is rarely computable in every direction: mode restriction, feasibility
//! ordering, and the subsumption rule that lets a narrow declaration still serve a
//! wider call.
//!
//! [`BoundSubjectLookup`] is the smallest honest relation that does. It is a lookup:
//! it can answer "what is `?x`'s value" and nothing else, so it declares exactly one
//! mode, `bf`, and the consequences fall out of the engine rather than out of this
//! file:
//!
//! * An `ff` invocation is **infeasible** — no declared mode subsumes it — so a group
//!   whose written order leaves the subject free is either reordered into a feasible
//!   one by the prepare-time feasibility pass or refused with a diagnostic naming the
//!   mode it could not reach.
//! * A `bb` invocation **is** feasible, by subsumption: `bf` covers it. This relation
//!   deliberately ignores the bound object-side value and emits every row for the
//!   bound subject, which is the generate-then-filter licence the trait grants — so a
//!   `bb` call that must answer "no" answers it through the ENGINE's equality filter
//!   on bound positions, not through anything this relation did.
//!
//! Its rows come from the same fixture graph the memory tables do, through the same
//! [`MemoryRelation::from_graph`] reader, so the tuple data has one home.

use std::sync::Arc;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{DatasetView, GraphMatch, TermValue};
use purrdf_sparql_eval::{
    EvalError, MemoryRelation, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, Volatility,
};

/// A two-column relation that can be computed **only** with its subject side bound.
///
/// Deterministic: the rows are frozen at construction and emitted in table order,
/// filtered by the bound subject and never reordered, so a query over it has one
/// answer in one order — the same contract
/// [`MemoryRelation`] meets, under a narrower
/// declaration.
#[derive(Debug, Clone)]
pub struct BoundSubjectLookup {
    /// The table, in emission order: one row per `[subject, object]` pair.
    rows: Arc<Vec<PfRow>>,
    /// The single declared mode (`bf`), materialized so [`PropertyFunction::modes`]
    /// can hand out a slice.
    modes: [BindingPattern; 1],
    /// The largest number of rows any one subject has — an EXACT upper bound on what
    /// one invocation can emit, measured from the table rather than guessed, because
    /// a declared bound that under-states reality turns an admission decision into a
    /// wrong one.
    rows_per_subject: u64,
}

impl BoundSubjectLookup {
    /// Read the lookup's rows out of `dataset`: the `rdf:List` of two-element
    /// `rdf:List`s whose head is `head`, exactly as
    /// [`MemoryRelation::from_graph`] defines it.
    ///
    /// # Errors
    ///
    /// Whatever [`MemoryRelation::from_graph`] raises for a torn list, an absent
    /// head, or a row that is not two values wide.
    pub fn from_graph<D: DatasetView>(
        dataset: &D,
        head: &TermValue,
        graph: GraphMatch<D::Id>,
    ) -> Result<Self, EvalError> {
        let table = MemoryRelation::from_graph(dataset, head, graph, 1, 1)?;
        Ok(Self::new(table.rows().to_vec()))
    }

    /// A lookup over `rows`, each `[subject, object]`.
    #[must_use]
    pub fn new(rows: Vec<PfRow>) -> Self {
        let rows_per_subject = rows
            .iter()
            .map(|row| rows.iter().filter(|other| other[0] == row[0]).count() as u64)
            .max()
            .unwrap_or(0);
        Self {
            rows: Arc::new(rows),
            modes: [BindingPattern::from_code("bf")],
            rows_per_subject,
        }
    }
}

impl PropertyFunction for BoundSubjectLookup {
    fn volatility(&self) -> Volatility {
        // A frozen table scanned in order: the same invocation returns the same rows
        // in the same order for the lifetime of a query, so it may run across
        // fork-join workers.
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        self.rows_per_subject
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        // The declared mode is `bf`, so a free subject is an invocation no declared
        // mode subsumes: refusing it is the hard-fail doctrine (an empty cursor would
        // be indistinguishable from an honest empty answer). The feasibility pass
        // never lets one through; this is the relation holding its own line.
        let Some(subject) = args.get(0) else {
            return Err(EvalError::function(
                "the bound-subject lookup declares only `bf`; it cannot enumerate its \
                 subject column",
            ));
        };
        // The object-side argument is deliberately NOT read: a `bb` call is served by
        // emitting the subject's rows and letting the engine's equality filter on
        // bound positions cut the ones that disagree.
        //
        // The row ceiling is likewise not read. It is a licence to stop early, not an
        // obligation — and a relation that emits candidates it knows the engine may
        // cut must not spend the licence on them, so declining it outright is the
        // honest reading of the ceiling contract here.
        Ok(Box::new(BoundSubjectCursor {
            rows: Arc::clone(&self.rows),
            subject: subject.clone(),
            next_index: 0,
        }))
    }
}

/// The cursor [`BoundSubjectLookup::open`] returns: a linear scan emitting the rows
/// whose subject equals the bound one, in table order.
#[derive(Debug)]
struct BoundSubjectCursor {
    rows: Arc<Vec<PfRow>>,
    subject: TermValue,
    next_index: usize,
}

impl PfCursor for BoundSubjectCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        while let Some(row) = self.rows.get(self.next_index) {
            self.next_index += 1;
            if row[0] == self.subject {
                return Ok(Some(row.clone()));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iri(local: &str) -> TermValue {
        TermValue::iri(format!("http://example.org/{local}"))
    }

    fn lookup() -> BoundSubjectLookup {
        BoundSubjectLookup::new(vec![
            vec![iri("a"), iri("1")],
            vec![iri("b"), iri("2")],
            vec![iri("a"), iri("3")],
        ])
    }

    #[test]
    fn declares_only_the_bound_subject_mode() {
        let relation = lookup();
        assert!(relation.admits(BindingPattern::from_code("bf")));
        assert!(
            relation.admits(BindingPattern::from_code("bb")),
            "`bf` subsumes `bb`: generate-then-filter"
        );
        assert!(!relation.admits(BindingPattern::from_code("ff")));
        assert!(!relation.admits(BindingPattern::from_code("fb")));
    }

    #[test]
    fn the_declared_row_bound_is_the_largest_subject_group() {
        assert_eq!(
            lookup().rows_per_invocation(BindingPattern::from_code("bf")),
            2,
            "`a` has two rows and `b` one"
        );
    }

    #[test]
    fn a_bound_call_emits_the_subject_rows_in_table_order() {
        let relation = lookup();
        let subject = iri("a");
        let bound = [Some(&subject)];
        let free = [None];
        let args = PfArgs::new(&bound, &free);
        let mut cursor = relation.open(&args, None).expect("open");
        let mut rows = Vec::new();
        while let Some(row) = cursor.next().expect("no error") {
            rows.push(row);
        }
        assert_eq!(
            rows,
            vec![vec![iri("a"), iri("1")], vec![iri("a"), iri("3")]]
        );
    }

    #[test]
    fn a_bound_object_is_left_to_the_engine_filter() {
        // A `bb` invocation whose object disagrees still receives the subject's rows:
        // this relation never filters the object position, so a suite case that
        // expects the disagreeing row to vanish is measuring the ENGINE.
        let relation = lookup();
        let subject = iri("b");
        let object = iri("999");
        let bound = [Some(&subject)];
        let also_bound = [Some(&object)];
        let args = PfArgs::new(&bound, &also_bound);
        let mut cursor = relation.open(&args, None).expect("open");
        assert_eq!(
            cursor.next().expect("no error"),
            Some(vec![iri("b"), iri("2")]),
            "the relation emits a candidate the engine must cut"
        );
    }

    #[test]
    fn a_free_subject_is_refused_rather_than_answered_empty() {
        let relation = lookup();
        let free = [None];
        let also_free = [None];
        let args = PfArgs::new(&free, &also_free);
        let error = relation
            .open(&args, None)
            .err()
            .expect("an `ff` invocation is not serveable");
        assert!(error.to_string().contains("declares only `bf`"), "{error}");
    }
}
