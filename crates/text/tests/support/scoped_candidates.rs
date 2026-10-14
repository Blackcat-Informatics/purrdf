// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual native producer controls shared by the native and portable runner.

use std::sync::Arc;

use purrdf_core::evidence::{ReasonDescription, SearchCompleteness};
use purrdf_core::query_scope::ConstraintDecision;
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_hash::hex::Digest32;
use purrdf_sparql_eval::{
    CandidateConstraint, EvalError, NativeCandidateScope, PfArgs, PropertyFunction, QueryEvidence,
    WorkspaceCapability,
};
use purrdf_text::{Analyzer, GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};

#[derive(Debug)]
struct Scope {
    cause: TermValue,
    concern: TermValue,
    fails: bool,
}

fn scope(fails: bool) -> Scope {
    Scope {
        cause: TermValue::iri("https://example.org/missing-time"),
        concern: TermValue::Triple {
            s: purrdf_core::TermBox::new(TermValue::iri("https://example.org/request")),
            p: purrdf_core::TermBox::new(TermValue::iri("https://example.org/time")),
            o: purrdf_core::TermBox::new(TermValue::simple_literal("required")),
        },
        fails,
    }
}

impl CandidateConstraint for Scope {
    fn identity(&self) -> Digest32 {
        Digest32::new([1; 32])
    }

    fn decide(
        &self,
        candidate: &TermValue,
        evidence: &QueryEvidence,
        _: &WorkspaceCapability,
    ) -> Result<ConstraintDecision, EvalError> {
        if self.fails {
            return Err(EvalError::AllocationFailed {
                construct: "original scoped document read",
            });
        }
        if candidate == &TermValue::iri("https://example.org/d1") {
            return Ok(ConstraintDecision::Rejected);
        }
        if candidate == &TermValue::iri("https://example.org/d2")
            || candidate == &TermValue::iri("https://example.org/d4")
        {
            return evidence
                .declare_reason(ReasonDescription {
                    cause: &self.cause,
                    concern: &self.concern,
                    constraint: None,
                    vault: None,
                    watermark: None,
                })
                .map(ConstraintDecision::Undecided);
        }
        Ok(ConstraintDecision::Satisfied)
    }
}

fn relation() -> TextSearchRelation {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri("https://example.org/note").unwrap();
    for (name, text) in [
        ("d1", "quick quick quick brown"),
        ("d2", "quick quick brown fox"),
        ("d3", "quick brown fox jumps"),
        ("d4", "quick fox jumps high"),
        ("d5", "lazy dog sleeps late"),
        ("d6", "brown bear sleeps late"),
    ] {
        let subject = builder
            .intern_iri(&format!("https://example.org/{name}"))
            .unwrap();
        let value = builder.intern_literal(RdfLiteral::simple(text)).unwrap();
        builder.push_quad(subject, note, value, None).unwrap();
    }
    let dataset = builder.freeze().unwrap();
    let config = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/note")],
        GraphSelector::Any,
        Analyzer::empty_lexicon(),
    )
    .unwrap();
    TextSearchRelation::new(Arc::new(
        TextIndex::from_dataset(&dataset, &config).unwrap(),
    ))
}

/// Rejected and undecided leaders do not consume the established depth; the
/// original score and unknown declaration survive the native cursor's lifetime.
pub(crate) fn scoped_text_depth_preserves_original_unknown_rows_and_scores() {
    let relation = relation();
    let needle = TermValue::simple_literal("quick brown");
    let subject = [None];
    let object = [Some(&needle), None, None, None, None];
    let args = PfArgs::new(&subject, &object);
    let mut original = relation.open(&args, None).unwrap();
    let mut baseline = Vec::new();
    while let Some(row) = original.next().unwrap() {
        baseline.push(row);
    }
    assert_eq!(baseline.len(), 5);
    assert_eq!(baseline[0][0], TermValue::iri("https://example.org/d1"));

    let workspace = WorkspaceCapability::default();
    let constraint = scope(false);
    let scope = NativeCandidateScope::new(&[&constraint], &workspace).unwrap();
    let scoped = args.with_candidate_scope(&scope);
    let mut cursor = relation.open_admitted(&scoped, Some(1), workspace).unwrap();
    let mut rows = Vec::new();
    while let Some(row) = cursor.next_admitted().unwrap() {
        rows.push(row);
    }
    assert_eq!(rows.len(), 3);
    for (row, name, undecided) in rows
        .iter()
        .zip([("d2", true), ("d3", false), ("d4", true)])
        .map(|(row, (name, undecided))| (row, name, undecided))
    {
        let key = TermValue::iri(format!("https://example.org/{name}"));
        assert_eq!(row.cells()[0], key);
        assert_eq!(row.evidence().is_undecided(), undecided);
        let source = baseline.iter().find(|source| source[0] == key).unwrap();
        assert_eq!(row.cells()[2], source[2]);
        assert_eq!(row.cells()[5], source[5]);
        if undecided {
            assert_eq!(row.cells()[3], source[3]);
        } else {
            assert_eq!(
                row.cells()[3],
                TermValue::typed_literal("1", purrdf_core::datatype::XSD_INTEGER,)
            );
        }
    }
    scope
        .with_counts(|counts| {
            assert_eq!(counts[0].rejected, 1);
            assert_eq!(counts[0].undecided, 2);
        })
        .unwrap();
    drop(cursor);
    drop(scope);
    let unknown = rows.remove(0);
    let reason = unknown
        .evidence()
        .with_record(|record| {
            assert_eq!(record.completeness(), SearchCompleteness::Undetermined);
            assert_eq!(record.reasons().len(), 1);
            record.reasons()[0]
        })
        .unwrap();
    unknown
        .evidence()
        .with_reason(reason, |description| {
            assert_eq!(description.cause, &constraint.cause);
            assert_eq!(description.concern, &constraint.concern);
        })
        .unwrap();
    assert!(matches!(
        unknown.try_into_resident(),
        Err(EvalError::WorkspaceUnpriced(_))
    ));
}

/// Operational source failures retain their original hard error channel and
/// invalidate counts rather than publishing a partial scope receipt.
pub(crate) fn scoped_text_preserves_original_constraint_failure() {
    let relation = relation();
    let workspace = WorkspaceCapability::default();
    let constraint = scope(true);
    let scope = NativeCandidateScope::new(&[&constraint], &workspace).unwrap();
    let needle = TermValue::simple_literal("quick brown");
    let subject = [None];
    let object = [Some(&needle), None, None, None, None];
    let args = PfArgs::new(&subject, &object).with_candidate_scope(&scope);
    assert!(matches!(
        relation.open(&args, Some(1)),
        Err(EvalError::AllocationFailed {
            construct: "original scoped document read",
        })
    ));
    assert!(scope.with_counts(|_| ()).is_err());
}

/// Both real candidate-bound paths apply scope: observed ranks search the
/// partition, while unobserved ranks use the original membership lookup.
pub(crate) fn scoped_text_bound_candidates_retain_unknown_membership() {
    let relation = relation();
    let constraint = scope(false);
    let needle = TermValue::simple_literal("quick brown");
    let object = [Some(&needle), None, None, None, None];
    for unobserved in [false, true] {
        for (name, emits) in [("d1", false), ("d2", true), ("d3", false)] {
            let workspace = WorkspaceCapability::default();
            let scope = NativeCandidateScope::new(&[&constraint], &workspace).unwrap();
            let candidate = TermValue::iri(format!("https://example.org/{name}"));
            let subject = [Some(&candidate)];
            let unread = [false, false, false, unobserved, false, false];
            let args = PfArgs::new(&subject, &object)
                .with_unobserved(&unread)
                .with_candidate_scope(&scope);
            let before = relation.observations().rankings();
            let mut cursor = relation.open_admitted(&args, Some(0), workspace).unwrap();
            let row = cursor.next_admitted().unwrap();
            assert_eq!(row.is_some(), emits);
            if let Some(row) = row {
                assert_eq!(row.cells()[0], candidate);
                assert!(row.evidence().is_undecided());
            }
            assert!(cursor.next_admitted().unwrap().is_none());
            assert_eq!(
                relation.observations().rankings() - before,
                u64::from(!unobserved)
            );
        }
    }
}
