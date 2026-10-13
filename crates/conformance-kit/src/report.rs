// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Full report equality with independently specified source identity anchors.

use std::collections::BTreeSet;
use std::ops::ControlFlow;

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermFactory, TermId};

use crate::GradeError;
use crate::identity::Identities;

mod projection;
pub use projection::{ReportFormat, ReportPolicy, compare_with_policy, extract_report};

/// The semantic role of a source snapshot in a report comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceRole {
    /// Data graph identities observed by validation results.
    DataGraph,
    /// Shapes graph identities observed by validation results.
    ShapesGraph,
}

/// One scope-qualified blank identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlankIdentity {
    /// Raw kernel label, without an egress envelope or `_:` prefix.
    pub label: String,
    /// The scope participates in identity.
    pub scope: BlankScope,
}

/// A parsed source snapshot. A shared domain means deliberate shared identity;
/// independently acquired documents use distinct domains even when bytes agree.
#[derive(Debug, Clone, Copy)]
pub struct SourceContext<'a> {
    /// The exact source graph, or an independently selected anchoring subgraph.
    pub dataset: &'a RdfDataset,
    /// Nonempty caller-specified source identity domain.
    pub domain: &'a str,
    /// Data or shapes role in this comparison.
    pub role: SourceRole,
}

/// Correspondence from a produced/expected report blank to an actual source.
/// Expected links must be specified from the oracle input, never reconstructed
/// from the produced report to explain away a wrong focus or shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlankCorrespondence {
    /// The blank in the parsed report.
    pub report: BlankIdentity,
    /// Which source snapshot supplies its identity.
    pub source_role: SourceRole,
    /// The exact source blank.
    pub source: BlankIdentity,
}

/// A full RDF report and its independent source identity evidence.
#[derive(Debug, Clone, Copy)]
pub struct ReportContext<'a> {
    /// The complete dedicated RDF report graph, including result/path topology.
    pub report: &'a RdfDataset,
    /// Optional distinguished report root; its identity is then observed.
    pub root: Option<TermId>,
    /// Source snapshots anchor focus/value/sourceShape/sourceConstraint blanks.
    pub sources: &'a [SourceContext<'a>],
    /// Explicit source/report links supplied by the producer or expected oracle.
    pub correspondence: &'a [BlankCorrespondence],
}

/// Freeze the oracle's source correspondence from independently supplied inputs.
/// Only focus/value observe data identity and sourceShape/sourceConstraint observe
/// shapes identity. Minted report/result/path nodes are never linked merely
/// because an inline oracle also happens to occur in a source snapshot.
///
/// # Errors
/// Refuses missing source evidence or one report identity naming distinct origins.
pub fn expected_source_correspondence(
    report: &RdfDataset,
    sources: &[SourceContext<'_>],
) -> Result<Vec<BlankCorrespondence>, GradeError> {
    let mut links = std::collections::BTreeMap::new();
    let source_blanks = admit_sources(sources)?;
    for quad in report.quads() {
        let role = match report.term_value(quad.p) {
            purrdf_core::TermValue::Iri(predicate)
                if matches!(
                    predicate.as_str(),
                    purrdf_iri::vocab::sh::FOCUS_NODE | purrdf_iri::vocab::sh::VALUE
                ) =>
            {
                SourceRole::DataGraph
            }
            purrdf_core::TermValue::Iri(predicate)
                if matches!(
                    predicate.as_str(),
                    purrdf_iri::vocab::sh::SOURCE_SHAPE | purrdf_iri::vocab::sh::SOURCE_CONSTRAINT
                ) =>
            {
                SourceRole::ShapesGraph
            }
            _ => continue,
        };
        let observed = report
            .term_value(quad.o)
            .visit_blank_identities(|label, scope| {
                let identity = BlankIdentity {
                    label: label.to_owned(),
                    scope,
                };
                if !source_blanks
                    .get(&role)
                    .is_some_and(|blanks| blanks.contains(&identity))
                {
                    return ControlFlow::Break(GradeError::Malformed(
                        "oracle observer blank lacks independent source correspondence".to_owned(),
                    ));
                }
                let link = BlankCorrespondence {
                    report: identity.clone(),
                    source_role: role,
                    source: identity.clone(),
                };
                if let Some(previous) = links.get(&identity) {
                    let previous: &BlankCorrespondence = previous;
                    let old_domain = sources
                        .iter()
                        .find(|source| source.role == previous.source_role)
                        .map(|source| source.domain);
                    let domain = sources
                        .iter()
                        .find(|source| source.role == role)
                        .map(|source| source.domain);
                    if previous.source != identity || old_domain != domain {
                        return ControlFlow::Break(GradeError::Malformed(
                            "oracle report blank has contradictory source origins".to_owned(),
                        ));
                    }
                } else {
                    links.insert(identity, link);
                }
                ControlFlow::Continue(())
            });
        if let ControlFlow::Break(error) = observed {
            return Err(error);
        }
    }
    Ok(links.into_values().collect())
}

/// Exact RDF1.2 dataset equality under one global blank bijection.
///
/// # Errors
/// Returns a typed kernel refusal or full graph mismatch.
pub fn compare_graphs(expected: &RdfDataset, actual: &RdfDataset) -> Result<(), GradeError> {
    let encode = |dataset| {
        let mut builder = RdfDatasetBuilder::new();
        encode_dataset(
            &mut builder,
            &mut Identities::default(),
            "dataset",
            "dataset",
            dataset,
        );
        builder
            .freeze()
            .map_err(|error| GradeError::Malformed(error.to_string()))
    };
    let expected = encode(expected)?;
    let actual = encode(actual)?;
    canonical_equality(&expected, &actual)
}

fn canonical_equality(expected: &RdfDataset, actual: &RdfDataset) -> Result<(), GradeError> {
    let expected =
        purrdf_core::try_canonicalize_flat_view(expected, purrdf_core::CanonHash::Sha256)
            .map_err(|error| GradeError::Canonicalization(error.to_string()))?;
    let actual = purrdf_core::try_canonicalize_flat_view(actual, purrdf_core::CanonHash::Sha256)
        .map_err(|error| GradeError::Canonicalization(error.to_string()))?;
    if expected.nquads == actual.nquads {
        Ok(())
    } else {
        Err(GradeError::Mismatch(
            "complete RDF datasets differ".to_owned(),
        ))
    }
}

/// Compare full reports in their shared source context, retaining every result,
/// path/list edge and source scope. No SHACL field is removed by this entrypoint.
///
/// All source and report RDF statements, including graph-qualified reification
/// and annotations, enter one structural carrier. Physical statement tables are
/// flattened through the kernel's quad views. It canonicalizes the carrier once, so
/// a report-only relabelling cannot move a result onto a different source blank.
///
/// # Errors
/// Refuses contradictory or within-role non-injective correspondence, unequal
/// context or kernel limits. Deliberate same-domain data/shapes egress aliases
/// resolve to the same original source identity across the two roles.
pub fn compare_contextual(
    expected: ReportContext<'_>,
    actual: ReportContext<'_>,
) -> Result<(), GradeError> {
    let expected = encode_context(expected)?;
    let actual = encode_context(actual)?;
    canonical_equality(&expected, &actual)
}

fn blank_identities(dataset: &RdfDataset) -> BTreeSet<BlankIdentity> {
    let mut identities = BTreeSet::new();
    for id in dataset.named_graphs() {
        let _ = dataset
            .term_value(id)
            .visit_blank_identities(|label, scope| {
                identities.insert(BlankIdentity {
                    label: label.to_owned(),
                    scope,
                });
                ControlFlow::<()>::Continue(())
            });
    }
    for quad in dataset
        .quads()
        .chain(dataset.reifier_quads())
        .chain(dataset.annotation_quads())
    {
        for id in [Some(quad.s), Some(quad.p), Some(quad.o), quad.g]
            .into_iter()
            .flatten()
        {
            let _ = dataset
                .term_value(id)
                .visit_blank_identities(|label, scope| {
                    identities.insert(BlankIdentity {
                        label: label.to_owned(),
                        scope,
                    });
                    ControlFlow::<()>::Continue(())
                });
        }
    }
    identities
}

fn admit_sources(
    sources: &[SourceContext<'_>],
) -> Result<std::collections::BTreeMap<SourceRole, BTreeSet<BlankIdentity>>, GradeError> {
    let mut roles = std::collections::BTreeMap::new();
    for source in sources {
        if source.domain.is_empty()
            || roles
                .insert(source.role, blank_identities(source.dataset))
                .is_some()
        {
            return Err(GradeError::Malformed(
                "source domains must be nonempty and roles unique".to_owned(),
            ));
        }
    }
    Ok(roles)
}

fn admit_context(context: ReportContext<'_>) -> Result<Identities, GradeError> {
    let mut identities = Identities::default();

    let report_blanks = blank_identities(context.report);
    let source_roles = admit_sources(context.sources)?;
    let mut reports = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for link in context.correspondence {
        let source = context
            .sources
            .iter()
            .find(|source| source.role == link.source_role)
            .ok_or_else(|| {
                GradeError::Malformed("correspondence names absent source role".to_owned())
            })?;
        if !report_blanks.contains(&link.report)
            || !source_roles[&source.role].contains(&link.source)
            || !reports.insert(link.report.clone())
            || !targets.insert((source.role, source.domain, link.source.clone()))
        {
            return Err(GradeError::Malformed(
                "source correspondence is absent, contradictory or non-injective".to_owned(),
            ));
        }
        identities.link(
            "report",
            (&link.report.label, link.report.scope),
            &format!("source:{}", source.domain),
            (&link.source.label, link.source.scope),
        );
    }
    Ok(identities)
}

fn encode_context(context: ReportContext<'_>) -> Result<std::sync::Arc<RdfDataset>, GradeError> {
    let mut identities = admit_context(context)?;
    let mut builder = RdfDatasetBuilder::new();
    encode_dataset(
        &mut builder,
        &mut identities,
        "report",
        "report",
        context.report,
    );
    for source in context.sources {
        let declaration = builder.intern_iri("urn:purrdf:conformance:context:source");
        let role = builder.intern_iri(match source.role {
            SourceRole::DataGraph => "urn:purrdf:conformance:context:data",
            SourceRole::ShapesGraph => "urn:purrdf:conformance:context:shapes",
        });
        let present = builder.intern_iri("urn:purrdf:conformance:context:present");
        builder.push_quad(declaration, present, role, None);
        encode_dataset(
            &mut builder,
            &mut identities,
            &format!("source:{}", source.domain),
            match source.role {
                SourceRole::DataGraph => "data",
                SourceRole::ShapesGraph => "shapes",
            },
            source.dataset,
        );
    }
    if let Some(root) = context.root {
        let subject = builder.intern_iri("urn:purrdf:conformance:context:root");
        let predicate = builder.intern_iri("urn:purrdf:conformance:context:report");
        let mapped = identities.map("report", &context.report.term_value(root));
        let object = builder.intern_value(&mapped);
        builder.push_quad(subject, predicate, object, None);
    }
    builder
        .freeze()
        .map_err(|error| GradeError::Malformed(error.to_string()))
}

fn encode_dataset(
    builder: &mut RdfDatasetBuilder,
    identities: &mut Identities,
    domain: &str,
    role: &str,
    dataset: &RdfDataset,
) {
    let default_graph =
        builder.intern_iri(&format!("urn:purrdf:conformance:context:{role}:default"));
    let mut graph_names = std::collections::BTreeMap::new();
    for g in dataset.named_graphs() {
        let graph =
            builder.intern_blank(&format!("graph{role}{}", graph_names.len()), BlankScope(1));
        let role_node = builder.intern_iri(&format!("urn:purrdf:conformance:context:{role}"));
        let graph_relation = builder.intern_iri("urn:purrdf:conformance:context:graph");
        builder.push_quad(role_node, graph_relation, graph, None);
        let original_relation = builder.intern_iri("urn:purrdf:conformance:context:sourceGraph");
        let original = builder.intern_value(&identities.map(domain, &dataset.term_value(g)));
        builder.push_quad(graph, original_relation, original, None);
        graph_names.insert(g, graph);
    }
    let quads: BTreeSet<_> = dataset
        .quads()
        .chain(dataset.reifier_quads())
        .chain(dataset.annotation_quads())
        .map(|quad| (quad.s, quad.p, quad.o, quad.g))
        .collect();
    for (s, p, o, g) in quads {
        let graph = g.map_or(default_graph, |g| graph_names[&g]);
        let s = builder.intern_value(&identities.map(domain, &dataset.term_value(s)));
        let p = builder.intern_value(&identities.map(domain, &dataset.term_value(p)));
        let o = builder.intern_value(&identities.map(domain, &dataset.term_value(o)));
        builder.push_quad(s, p, o, Some(graph));
    }
}
