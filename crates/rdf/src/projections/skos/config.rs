// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

use super::super::{ProjectionError, ProjectionLimits, validate_absolute_iri};

/// Caller-owned RDF type and SKOS class roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkosClassRoles {
    rdf_type: String,
    concept: String,
    concept_scheme: String,
}

impl SkosClassRoles {
    /// Construct the mandatory class-role group.
    pub fn new(
        rdf_type: impl Into<String>,
        concept: impl Into<String>,
        concept_scheme: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let roles = Self {
            rdf_type: rdf_type.into(),
            concept: concept.into(),
            concept_scheme: concept_scheme.into(),
        };
        validate_named_iris(roles.named_iris())?;
        Ok(roles)
    }

    /// RDF type predicate role.
    pub fn rdf_type(&self) -> &str {
        &self.rdf_type
    }

    /// SKOS Concept class role.
    pub fn concept(&self) -> &str {
        &self.concept
    }

    /// SKOS ConceptScheme class role.
    pub fn concept_scheme(&self) -> &str {
        &self.concept_scheme
    }

    fn named_iris(&self) -> [(&'static str, &str); 3] {
        [
            ("rdf_type", &self.rdf_type),
            ("concept", &self.concept),
            ("concept_scheme", &self.concept_scheme),
        ]
    }
}

/// Caller-owned SKOS lexical-label and notation roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkosLabelRoles {
    pref_label: String,
    alt_label: String,
    hidden_label: String,
    notation: String,
}

impl SkosLabelRoles {
    /// Construct the mandatory lexical-role group.
    pub fn new(
        pref_label: impl Into<String>,
        alt_label: impl Into<String>,
        hidden_label: impl Into<String>,
        notation: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let roles = Self {
            pref_label: pref_label.into(),
            alt_label: alt_label.into(),
            hidden_label: hidden_label.into(),
            notation: notation.into(),
        };
        validate_named_iris(roles.named_iris())?;
        Ok(roles)
    }

    /// Preferred-label predicate role.
    pub fn pref_label(&self) -> &str {
        &self.pref_label
    }

    /// Alternate-label predicate role.
    pub fn alt_label(&self) -> &str {
        &self.alt_label
    }

    /// Hidden-label predicate role.
    pub fn hidden_label(&self) -> &str {
        &self.hidden_label
    }

    /// Notation predicate role.
    pub fn notation(&self) -> &str {
        &self.notation
    }

    fn named_iris(&self) -> [(&'static str, &str); 4] {
        [
            ("pref_label", &self.pref_label),
            ("alt_label", &self.alt_label),
            ("hidden_label", &self.hidden_label),
            ("notation", &self.notation),
        ]
    }
}

/// Caller-owned SKOS documentation-property roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkosDocumentationRoles {
    note: String,
    change_note: String,
    definition: String,
    editorial_note: String,
    example: String,
    history_note: String,
    scope_note: String,
}

impl SkosDocumentationRoles {
    /// Construct the complete documentation-role group.
    #[allow(
        clippy::too_many_arguments,
        reason = "mandatory named roles forbid an incomplete or fabricated vocabulary"
    )]
    pub fn new(
        note: impl Into<String>,
        change_note: impl Into<String>,
        definition: impl Into<String>,
        editorial_note: impl Into<String>,
        example: impl Into<String>,
        history_note: impl Into<String>,
        scope_note: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let roles = Self {
            note: note.into(),
            change_note: change_note.into(),
            definition: definition.into(),
            editorial_note: editorial_note.into(),
            example: example.into(),
            history_note: history_note.into(),
            scope_note: scope_note.into(),
        };
        validate_named_iris(roles.named_iris())?;
        Ok(roles)
    }

    /// Generic note predicate role.
    pub fn note(&self) -> &str {
        &self.note
    }
    /// Change-note predicate role.
    pub fn change_note(&self) -> &str {
        &self.change_note
    }
    /// Definition predicate role.
    pub fn definition(&self) -> &str {
        &self.definition
    }
    /// Editorial-note predicate role.
    pub fn editorial_note(&self) -> &str {
        &self.editorial_note
    }
    /// Example predicate role.
    pub fn example(&self) -> &str {
        &self.example
    }
    /// History-note predicate role.
    pub fn history_note(&self) -> &str {
        &self.history_note
    }
    /// Scope-note predicate role.
    pub fn scope_note(&self) -> &str {
        &self.scope_note
    }

    fn named_iris(&self) -> [(&'static str, &str); 7] {
        [
            ("note", &self.note),
            ("change_note", &self.change_note),
            ("definition", &self.definition),
            ("editorial_note", &self.editorial_note),
            ("example", &self.example),
            ("history_note", &self.history_note),
            ("scope_note", &self.scope_note),
        ]
    }
}

/// Caller-owned SKOS hierarchy, mapping, membership, and top-concept roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkosRelationRoles {
    broader: String,
    narrower: String,
    related: String,
    close_match: String,
    exact_match: String,
    broad_match: String,
    narrow_match: String,
    related_match: String,
    in_scheme: String,
    has_top_concept: String,
    top_concept_of: String,
}

impl SkosRelationRoles {
    /// Construct the complete relation-role group.
    #[allow(
        clippy::too_many_arguments,
        reason = "mandatory named roles forbid an incomplete or fabricated vocabulary"
    )]
    pub fn new(
        broader: impl Into<String>,
        narrower: impl Into<String>,
        related: impl Into<String>,
        close_match: impl Into<String>,
        exact_match: impl Into<String>,
        broad_match: impl Into<String>,
        narrow_match: impl Into<String>,
        related_match: impl Into<String>,
        in_scheme: impl Into<String>,
        has_top_concept: impl Into<String>,
        top_concept_of: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let roles = Self {
            broader: broader.into(),
            narrower: narrower.into(),
            related: related.into(),
            close_match: close_match.into(),
            exact_match: exact_match.into(),
            broad_match: broad_match.into(),
            narrow_match: narrow_match.into(),
            related_match: related_match.into(),
            in_scheme: in_scheme.into(),
            has_top_concept: has_top_concept.into(),
            top_concept_of: top_concept_of.into(),
        };
        validate_named_iris(roles.named_iris())?;
        Ok(roles)
    }

    /// Broader-concept predicate role.
    pub fn broader(&self) -> &str {
        &self.broader
    }
    /// Narrower-concept predicate role.
    pub fn narrower(&self) -> &str {
        &self.narrower
    }
    /// Associative-related predicate role.
    pub fn related(&self) -> &str {
        &self.related
    }
    /// Close-match predicate role.
    pub fn close_match(&self) -> &str {
        &self.close_match
    }
    /// Exact-match predicate role.
    pub fn exact_match(&self) -> &str {
        &self.exact_match
    }
    /// Broad-match predicate role.
    pub fn broad_match(&self) -> &str {
        &self.broad_match
    }
    /// Narrow-match predicate role.
    pub fn narrow_match(&self) -> &str {
        &self.narrow_match
    }
    /// Related-match predicate role.
    pub fn related_match(&self) -> &str {
        &self.related_match
    }
    /// Concept-scheme membership predicate role.
    pub fn in_scheme(&self) -> &str {
        &self.in_scheme
    }
    /// Scheme-to-top-concept predicate role.
    pub fn has_top_concept(&self) -> &str {
        &self.has_top_concept
    }
    /// Top-concept-to-scheme predicate role.
    pub fn top_concept_of(&self) -> &str {
        &self.top_concept_of
    }

    fn named_iris(&self) -> [(&'static str, &str); 11] {
        [
            ("broader", &self.broader),
            ("narrower", &self.narrower),
            ("related", &self.related),
            ("close_match", &self.close_match),
            ("exact_match", &self.exact_match),
            ("broad_match", &self.broad_match),
            ("narrow_match", &self.narrow_match),
            ("related_match", &self.related_match),
            ("in_scheme", &self.in_scheme),
            ("has_top_concept", &self.has_top_concept),
            ("top_concept_of", &self.top_concept_of),
        ]
    }
}

macro_rules! role_set {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            classes: SkosClassRoles,
            labels: SkosLabelRoles,
            documentation: SkosDocumentationRoles,
            relations: SkosRelationRoles,
        }

        impl $name {
            /// Combine and cross-check all mandatory semantic-role groups.
            pub fn new(
                classes: SkosClassRoles,
                labels: SkosLabelRoles,
                documentation: SkosDocumentationRoles,
                relations: SkosRelationRoles,
            ) -> Result<Self, ProjectionError> {
                let roles = Self {
                    classes,
                    labels,
                    documentation,
                    relations,
                };
                roles.validate()?;
                Ok(roles)
            }

            /// RDF and SKOS class roles.
            pub const fn classes(&self) -> &SkosClassRoles {
                &self.classes
            }
            /// Lexical-label and notation roles.
            pub const fn labels(&self) -> &SkosLabelRoles {
                &self.labels
            }
            /// Documentation-property roles.
            pub const fn documentation(&self) -> &SkosDocumentationRoles {
                &self.documentation
            }
            /// Hierarchy, mapping, membership, and top-concept roles.
            pub const fn relations(&self) -> &SkosRelationRoles {
                &self.relations
            }

            fn validate(&self) -> Result<(), ProjectionError> {
                let mut seen = BTreeSet::new();
                for (role, iri) in self.named_iris() {
                    validate_absolute_iri(iri, concat!(stringify!($name), " role"))?;
                    if !seen.insert(iri) {
                        return Err(ProjectionError::configuration(format!(
                            "{} role `{role}` reuses `{iri}`; semantic roles must be distinct",
                            stringify!($name)
                        )));
                    }
                }
                Ok(())
            }

            fn named_iris(&self) -> Vec<(&'static str, &str)> {
                self.classes
                    .named_iris()
                    .into_iter()
                    .chain(self.labels.named_iris())
                    .chain(self.documentation.named_iris())
                    .chain(self.relations.named_iris())
                    .collect()
            }
        }
    };
}

role_set!(
    SkosSourceRoles,
    "Complete caller-owned source interpretation for the RDF→SKOS projection."
);
role_set!(
    SkosTargetRoles,
    "Complete caller-owned target vocabulary for the emitted SKOS view."
);

/// Source graph selection for one SKOS concept-scheme view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SkosGraphSelection {
    /// Read only default-graph statements.
    DefaultGraph,
    /// Read one caller-identified named graph and flatten its placement into the view.
    NamedGraph {
        /// Full IRI of the selected graph.
        graph_iri: String,
    },
    /// Read the union of default and all named graphs.
    Union,
}

impl SkosGraphSelection {
    fn validate(&self) -> Result<(), ProjectionError> {
        if let Self::NamedGraph { graph_iri } = self {
            validate_absolute_iri(graph_iri, "SKOS selected named graph")?;
        }
        Ok(())
    }
}

/// Mandatory identity, vocabulary, graph, and resource policy for RDF→SKOS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkosConfig {
    source: SkosSourceRoles,
    target: SkosTargetRoles,
    scheme_iri: String,
    graph_selection: SkosGraphSelection,
    limits: ProjectionLimits,
    max_records: usize,
}

impl SkosConfig {
    /// Construct a fully explicit SKOS projection policy.
    pub fn new(
        source: SkosSourceRoles,
        target: SkosTargetRoles,
        scheme_iri: impl Into<String>,
        graph_selection: SkosGraphSelection,
        limits: ProjectionLimits,
        max_records: usize,
    ) -> Result<Self, ProjectionError> {
        source.validate()?;
        target.validate()?;
        let scheme_iri = scheme_iri.into();
        validate_absolute_iri(&scheme_iri, "SKOS caller-owned concept-scheme IRI")?;
        graph_selection.validate()?;
        if max_records == 0 {
            return Err(ProjectionError::configuration(
                "SKOS max_records must be greater than zero",
            ));
        }
        if u32::try_from(max_records).is_err() {
            return Err(ProjectionError::configuration(
                "SKOS max_records exceeds the portable u32 record ceiling",
            ));
        }
        Ok(Self {
            source,
            target,
            scheme_iri,
            graph_selection,
            limits,
            max_records,
        })
    }

    /// Caller-owned source interpretation.
    pub const fn source(&self) -> &SkosSourceRoles {
        &self.source
    }
    /// Caller-owned target vocabulary.
    pub const fn target(&self) -> &SkosTargetRoles {
        &self.target
    }
    /// Caller-owned full IRI of the emitted concept scheme.
    pub fn scheme_iri(&self) -> &str {
        &self.scheme_iri
    }
    /// Source graph-selection policy.
    pub const fn graph_selection(&self) -> &SkosGraphSelection {
        &self.graph_selection
    }
    /// Shared projection byte and recursion bounds.
    pub const fn limits(&self) -> ProjectionLimits {
        self.limits
    }
    /// Maximum combined input and output record count.
    pub const fn max_records(&self) -> usize {
        self.max_records
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSkosConfig {
    source: SkosSourceRoles,
    target: SkosTargetRoles,
    scheme_iri: String,
    graph_selection: SkosGraphSelection,
    limits: ProjectionLimits,
    max_records: usize,
}

impl<'de> Deserialize<'de> for SkosConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawSkosConfig::deserialize(deserializer)?;
        Self::new(
            raw.source,
            raw.target,
            raw.scheme_iri,
            raw.graph_selection,
            raw.limits,
            raw.max_records,
        )
        .map_err(serde::de::Error::custom)
    }
}

fn validate_named_iris<const N: usize>(iris: [(&str, &str); N]) -> Result<(), ProjectionError> {
    for (name, iri) in iris {
        validate_absolute_iri(iri, &format!("SKOS vocabulary role `{name}`"))?;
    }
    Ok(())
}
