// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL test-format projection; graph isomorphism remains in the kernel.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::{
    BlankScope, DatasetView as _, RdfDataset, RdfDatasetBuilder, TermFactory, TermId, TermValue,
};
use purrdf_iri::vocab::{rdf, sh};
use purrdf_lex::walk::WorkList;

use super::{ReportContext, compare_contextual};
use crate::GradeError;

/// The selected test-format contract, separate from the validation language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// Every dedicated report statement is observed.
    Exact,
    /// W3C 2017 test format: prescribed fields, message objects and cloned paths.
    Recommendation2017,
    /// Dated SHACL1.2 expectations also observe declared details and annotations.
    Draft20260918,
}

/// Explicit report comparison rules. Required declared fields are always retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportPolicy {
    /// The report test format, never inferred from engine behavior.
    pub format: ReportFormat,
    /// Explicit compatibility rule for old frozen expected graphs omitting a
    /// source constraint. It never removes a constraint stated by an oracle.
    pub allow_unstated_source_constraint: bool,
}

/// Extract a dedicated report from an inline manifest root. Source observer
/// edges are retained; data/shapes triples are supplied separately as context.
///
/// # Errors
/// Refuses malformed property-path structures.
pub fn extract_report(dataset: &RdfDataset, root: TermId) -> Result<Arc<RdfDataset>, GradeError> {
    project(
        dataset,
        root,
        None,
        ReportPolicy {
            format: ReportFormat::Exact,
            allow_unstated_source_constraint: false,
        },
        None,
        false,
    )
}

/// Apply the selected test-format rules on both sides before global contextual
/// comparison. Raw artifacts remain with the caller; no engine output supplies
/// the expected graph or source correspondence.
///
/// # Errors
/// Returns typed malformed/mismatch/kernel refusals. Missing declared detail,
/// annotation, source constraint and message obligations remain mismatches.
pub fn compare_with_policy(
    expected: ReportContext<'_>,
    actual: ReportContext<'_>,
    policy: ReportPolicy,
) -> Result<(), GradeError> {
    if policy.format == ReportFormat::Exact {
        return compare_contextual(expected, actual);
    }
    let expected_root = root_of(expected)?;
    let actual_root = root_of(actual)?;
    super::admit_context(expected)?;
    super::admit_context(actual)?;
    let expected_projected = project(
        expected.report,
        expected_root,
        Some(expected.report),
        policy,
        None,
        false,
    )?;
    let patterns = optional_patterns(expected.report, expected_root)?;
    let actual_nodes = result_nodes(actual.report, actual_root)?;
    let mut choices: Vec<Vec<OptionalFields>> = actual_nodes
        .iter()
        .map(|&node| {
            let key = mandatory_key(actual.report, node);
            let choices: BTreeSet<_> = patterns
                .iter()
                .filter(|(expected_node, _)| mandatory_key(expected.report, **expected_node) == key)
                .map(|(_, fields)| fields.clone())
                .collect();
            if choices.is_empty() {
                vec![OptionalFields::default()]
            } else {
                choices.into_iter().collect()
            }
        })
        .collect();
    if policy.format != ReportFormat::Draft20260918 {
        for choice in &mut choices {
            choice.truncate(1);
        }
    }
    let mut digits = vec![0_usize; choices.len()];
    let mut attempts = 0_usize;
    loop {
        attempts += 1;
        if attempts > 100_000 {
            return Err(GradeError::Canonicalization(
                "optional report matching exceeds 100000 candidate projections".to_owned(),
            ));
        }
        let selected: BTreeMap<_, _> = actual_nodes
            .iter()
            .zip(choices.iter().zip(&digits))
            .map(|(&node, (choice, &digit))| (node, choice[digit].clone()))
            .collect();
        let actual_projected = project(
            actual.report,
            actual_root,
            Some(expected.report),
            policy,
            Some(&selected),
            true,
        )?;
        let expected_blanks = super::blank_identities(&expected_projected);
        let actual_blanks = super::blank_identities(&actual_projected);
        let expected_links: Vec<_> = expected
            .correspondence
            .iter()
            .filter(|link| expected_blanks.contains(&link.report))
            .cloned()
            .collect();
        let actual_links: Vec<_> = actual
            .correspondence
            .iter()
            .filter(|link| actual_blanks.contains(&link.report))
            .cloned()
            .collect();
        let comparison = compare_contextual(
            ReportContext {
                report: &expected_projected,
                root: None,
                correspondence: &expected_links,
                ..expected
            },
            ReportContext {
                report: &actual_projected,
                root: None,
                correspondence: &actual_links,
                ..actual
            },
        );
        match comparison {
            Ok(()) => return Ok(()),
            Err(GradeError::Mismatch(_)) => {}
            Err(error) => return Err(error),
        }
        let mut advanced = false;
        for position in (0..digits.len()).rev() {
            digits[position] += 1;
            if digits[position] < choices[position].len() {
                advanced = true;
                break;
            }
            digits[position] = 0;
        }
        if !advanced {
            return Err(GradeError::Mismatch("complete projected report graphs differ under every admitted optional-field correspondence".to_owned()));
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct OptionalFields {
    details: bool,
    messages: bool,
    annotations: BTreeSet<String>,
}

fn result_nodes(dataset: &RdfDataset, root: TermId) -> Result<Vec<TermId>, GradeError> {
    let reader = crate::graph::Reader {
        dataset,
        graph: purrdf_core::GraphMatch::Default,
    };
    let mut work: WorkList<TermId, 16> = WorkList::with(root);
    let mut seen = BTreeSet::new();
    while let Some(node) = work.pop() {
        if !seen.insert(node) {
            continue;
        }
        if !matches!(
            dataset.term_value(node),
            TermValue::Blank { .. } | TermValue::Iri(_)
        ) {
            return Err(GradeError::Malformed(
                "report/result node is not a resource".to_owned(),
            ));
        }
        for predicate in [sh::RESULT, sh::DETAIL] {
            for child in reader.objects(node, predicate) {
                work.push(child);
            }
        }
    }
    seen.remove(&root);
    Ok(seen.into_iter().collect())
}

fn optional_patterns(
    dataset: &RdfDataset,
    root: TermId,
) -> Result<BTreeMap<TermId, OptionalFields>, GradeError> {
    let reader = crate::graph::Reader {
        dataset,
        graph: purrdf_core::GraphMatch::Default,
    };
    Ok(result_nodes(dataset, root)?
        .into_iter()
        .map(|node| {
            let annotations = dataset
                .quads_for_pattern(Some(node), None, None, purrdf_core::GraphMatch::Default)
                .filter_map(|quad| match dataset.term_value(quad.p) {
                    TermValue::Iri(predicate)
                        if !predicate.starts_with(sh::NS)
                            && predicate != rdf::TYPE
                            && !predicate.starts_with(purrdf_iri::vocab::shnex::NS) =>
                    {
                        Some(predicate)
                    }
                    _ => None,
                })
                .collect();
            (
                node,
                OptionalFields {
                    details: !reader.objects(node, sh::DETAIL).is_empty(),
                    messages: !reader.objects(node, sh::RESULT_MESSAGE).is_empty(),
                    annotations,
                },
            )
        })
        .collect())
}

fn mandatory_key(dataset: &RdfDataset, node: TermId) -> Vec<(String, Option<TermValue>)> {
    let mut key: Vec<_> = dataset
        .quads_for_pattern(Some(node), None, None, purrdf_core::GraphMatch::Default)
        .filter_map(|quad| {
            let TermValue::Iri(predicate) = dataset.term_value(quad.p) else {
                return None;
            };
            if !core_predicate(&predicate)
                || matches!(predicate.as_str(), rdf::TYPE | sh::SOURCE_CONSTRAINT)
            {
                return None;
            }
            let value = dataset.term_value(quad.o);
            let mut blank = false;
            let _ = value.visit_blank_identities(|_, _| {
                blank = true;
                std::ops::ControlFlow::<()>::Continue(())
            });
            Some((predicate, (!blank).then_some(value)))
        })
        .collect();
    key.sort();
    key
}

fn root_of(context: ReportContext<'_>) -> Result<TermId, GradeError> {
    if let Some(root) = context.root {
        return Ok(root);
    }
    let reader = crate::graph::Reader {
        dataset: context.report,
        graph: purrdf_core::GraphMatch::Any,
    };
    let class = context
        .report
        .term_id_by_iri(sh::VALIDATION_REPORT)
        .ok_or_else(|| {
            GradeError::Malformed("report has no sh:ValidationReport class".to_owned())
        })?;
    let roots = reader.subjects(rdf::TYPE, class);
    match roots.as_slice() {
        [root] => Ok(*root),
        _ => Err(GradeError::Malformed(
            "report must have one distinguished root".to_owned(),
        )),
    }
}

fn path_predicate(predicate: &str) -> bool {
    matches!(
        predicate,
        rdf::FIRST
            | rdf::REST
            | sh::INVERSE_PATH
            | sh::ALTERNATIVE_PATH
            | sh::ZERO_OR_MORE_PATH
            | sh::ONE_OR_MORE_PATH
            | sh::ZERO_OR_ONE_PATH
    )
}

fn core_predicate(predicate: &str) -> bool {
    matches!(
        predicate,
        rdf::TYPE
            | sh::RESULT
            | sh::CONFORMS
            | sh::FOCUS_NODE
            | sh::RESULT_PATH
            | sh::RESULT_SEVERITY
            | sh::SOURCE_CONSTRAINT_COMPONENT
            | sh::SOURCE_SHAPE
            | sh::VALUE
    ) || predicate == sh::SOURCE_CONSTRAINT
}

fn project(
    dataset: &RdfDataset,
    root: TermId,
    expectation: Option<&RdfDataset>,
    policy: ReportPolicy,
    selected: Option<&BTreeMap<TermId, OptionalFields>>,
    include_orphans: bool,
) -> Result<Arc<RdfDataset>, GradeError> {
    if dataset.quads().any(|quad| quad.g.is_some()) {
        return Err(GradeError::Malformed(
            "SHACL test reports must be a dedicated default graph".to_owned(),
        ));
    }
    let mut builder = RdfDatasetBuilder::new();
    let mut nodes: WorkList<TermId, 16> = WorkList::with(root);
    if include_orphans {
        let reachable: BTreeSet<_> = result_nodes(dataset, root)?.into_iter().collect();
        for quad in dataset.quads() {
            if matches!(dataset.term_value(quad.p), TermValue::Iri(ref predicate) if predicate == rdf::TYPE)
                && matches!(dataset.term_value(quad.o), TermValue::Iri(ref class) if class == sh::VALIDATION_RESULT)
                && !reachable.contains(&quad.s)
            {
                nodes.push(quad.s);
            }
        }
    }
    let own_patterns = optional_patterns(dataset, root)?;
    let mut visited = BTreeSet::new();
    let mut emitted = BTreeMap::new();
    let mut fresh = 0_usize;
    let mut occupied = super::blank_identities(dataset);
    let expected_values = |predicate: &str| -> BTreeSet<TermValue> {
        expectation
            .into_iter()
            .flat_map(RdfDataset::quads)
            .filter(|quad| expected_predicate(expectation, quad.p, predicate))
            .map(|quad| {
                expectation
                    .expect("quad has expected dataset")
                    .term_value(quad.o)
            })
            .collect()
    };
    let messages = expected_values(sh::RESULT_MESSAGE);
    let has_expected = |predicate: &str| !expected_values(predicate).is_empty();

    while let Some(node) = nodes.pop() {
        if !visited.insert(node) {
            continue;
        }
        let fields = selected
            .and_then(|selected| selected.get(&node))
            .or_else(|| own_patterns.get(&node));
        let has_details = policy.format == ReportFormat::Exact
            || (policy.format == ReportFormat::Draft20260918
                && fields.is_some_and(|fields| fields.details));
        let value = dataset.term_value(node);
        let subject = if policy.format != ReportFormat::Exact && matches!(value, TermValue::Iri(_))
        {
            emitted
                .entry(node)
                .or_insert_with(|| fresh_blank(&mut occupied, &mut fresh))
                .clone()
        } else {
            value
        };
        let subject = builder.intern_value(&subject);
        for quad in
            dataset.quads_for_pattern(Some(node), None, None, purrdf_core::GraphMatch::Default)
        {
            let TermValue::Iri(predicate) = dataset.term_value(quad.p) else {
                return Err(GradeError::Malformed(
                    "report predicate is not an IRI".to_owned(),
                ));
            };
            let object = dataset.term_value(quad.o);
            let allowed = policy.format == ReportFormat::Exact
                || core_predicate(&predicate)
                || (predicate == sh::RESULT_MESSAGE
                    && (if policy.format == ReportFormat::Draft20260918 {
                        fields.is_some_and(|fields| fields.messages)
                    } else {
                        messages.contains(&object)
                    }))
                || (predicate == sh::DETAIL && has_details)
                || (policy.format == ReportFormat::Draft20260918
                    && (fields.is_some_and(|fields| fields.annotations.contains(&predicate))
                        || (matches!(
                            predicate.as_str(),
                            sh::CONFORMANCE_DISALLOWS | sh::SHAPES_GRAPH_WELL_FORMED
                        ) && has_expected(&predicate))));
            if (policy.format != ReportFormat::Exact
                && predicate == rdf::TYPE
                && !matches!(&object, TermValue::Iri(class) if matches!(class.as_str(), sh::VALIDATION_REPORT | sh::VALIDATION_RESULT)))
                || !allowed
                || (predicate == sh::SOURCE_CONSTRAINT
                    && policy.allow_unstated_source_constraint
                    && !has_expected(sh::SOURCE_CONSTRAINT))
            {
                continue;
            }
            let object = if predicate == sh::RESULT || predicate == sh::DETAIL {
                nodes.push(quad.o);
                if policy.format != ReportFormat::Exact && matches!(object, TermValue::Iri(_)) {
                    emitted
                        .entry(quad.o)
                        .or_insert_with(|| fresh_blank(&mut occupied, &mut fresh))
                        .clone()
                } else {
                    object
                }
            } else if predicate == sh::RESULT_PATH {
                if policy.format == ReportFormat::Exact {
                    copy_path(dataset, quad.o, &mut builder);
                    object
                } else {
                    clone_path(dataset, quad.o, &mut builder, &mut occupied, &mut fresh)?
                }
            } else {
                object
            };
            let predicate = builder.intern_iri(&predicate);
            let object = builder.intern_value(&object);
            builder.push_quad(subject, predicate, object, None);
        }
    }
    builder
        .freeze()
        .map_err(|error| GradeError::Malformed(error.to_string()))
}

fn expected_predicate(expectation: Option<&RdfDataset>, id: TermId, predicate: &str) -> bool {
    expectation.is_some_and(
        |expected| matches!(expected.term_value(id), TermValue::Iri(ref iri) if iri == predicate),
    )
}

fn fresh_blank(occupied: &mut BTreeSet<super::BlankIdentity>, ordinal: &mut usize) -> TermValue {
    loop {
        let blank = super::BlankIdentity {
            label: format!("projection{ordinal}"),
            scope: BlankScope::DEFAULT,
        };
        *ordinal += 1;
        if occupied.insert(blank.clone()) {
            return TermValue::Blank {
                label: blank.label,
                scope: blank.scope,
            };
        }
    }
}

fn clone_path(
    dataset: &RdfDataset,
    root: TermId,
    builder: &mut RdfDatasetBuilder,
    occupied: &mut BTreeSet<super::BlankIdentity>,
    fresh: &mut usize,
) -> Result<TermValue, GradeError> {
    if !matches!(dataset.term_value(root), TermValue::Blank { .. }) {
        return Ok(dataset.term_value(root));
    }
    enum Step {
        Enter(TermId, TermValue),
        Leave(TermId),
    }
    let root_value = fresh_blank(occupied, fresh);
    let mut work: WorkList<Step, 16> = WorkList::with(Step::Enter(root, root_value.clone()));
    let mut active = BTreeSet::new();
    while let Some(step) = work.pop() {
        match step {
            Step::Leave(id) => {
                active.remove(&id);
            }
            Step::Enter(id, value) => {
                if !active.insert(id) {
                    return Err(GradeError::Malformed("cyclic result path".to_owned()));
                }
                work.push(Step::Leave(id));
                let subject = builder.intern_value(&value);
                let edges: Vec<_> = dataset.quads_for_pattern(Some(id), None, None, purrdf_core::GraphMatch::Default)
                    .filter(|quad| matches!(dataset.term_value(quad.p), TermValue::Iri(ref predicate) if path_predicate(predicate))).collect();
                if edges.is_empty() {
                    return Err(GradeError::Malformed(
                        "blank result path has no structural edges".to_owned(),
                    ));
                }
                for quad in edges {
                    let object = dataset.term_value(quad.o);
                    let object = if matches!(object, TermValue::Blank { .. }) {
                        let copy = fresh_blank(occupied, fresh);
                        work.push(Step::Enter(quad.o, copy.clone()));
                        copy
                    } else {
                        object
                    };
                    let predicate = builder.intern_value(&dataset.term_value(quad.p));
                    let object = builder.intern_value(&object);
                    builder.push_quad(subject, predicate, object, None);
                }
            }
        }
    }
    Ok(root_value)
}

fn copy_path(dataset: &RdfDataset, root: TermId, builder: &mut RdfDatasetBuilder) {
    let mut work: WorkList<TermId, 16> = WorkList::with(root);
    let mut visited = BTreeSet::new();
    while let Some(node) = work.pop() {
        if !visited.insert(node) {
            continue;
        }
        for quad in dataset.quads_for_pattern(Some(node), None, None, purrdf_core::GraphMatch::Default)
            .filter(|quad| matches!(dataset.term_value(quad.p), TermValue::Iri(ref predicate) if path_predicate(predicate))) {
            let s = builder.intern_value(&dataset.term_value(quad.s));
            let p = builder.intern_value(&dataset.term_value(quad.p));
            let o = builder.intern_value(&dataset.term_value(quad.o));
            builder.push_quad(s, p, o, None);
            if matches!(dataset.term_value(quad.o), TermValue::Blank { .. }) { work.push(quad.o); }
        }
    }
}
