// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::super::util::{iri_role_group, validate_distinct_roles};

use purrdf_lex::json::{Object, Value};

use super::super::{ProjectionError, ProjectionLimits, validate_absolute_iri};
use purrdf_lex::json::record::{DecodeError, FromJson, Record, ToJson};

iri_role_group! {
    /// Caller-owned RDF type and SKOS class roles.
    SkosClassRoles as "struct SkosClassRoles" in "SKOS" {
        /// RDF type predicate role.
        rdf_type,
        /// SKOS Concept class role.
        concept,
        /// SKOS ConceptScheme class role.
        concept_scheme,
    }
}

iri_role_group! {
    /// Caller-owned SKOS lexical-label and notation roles.
    SkosLabelRoles as "struct SkosLabelRoles" in "SKOS" {
        /// Preferred-label predicate role.
        pref_label,
        /// Alternate-label predicate role.
        alt_label,
        /// Hidden-label predicate role.
        hidden_label,
        /// Notation predicate role.
        notation,
    }
}

iri_role_group! {
    /// Caller-owned SKOS documentation-property roles.
    SkosDocumentationRoles as "struct SkosDocumentationRoles" in "SKOS" {
        /// Generic note predicate role.
        note,
        /// Change-note predicate role.
        change_note,
        /// Definition predicate role.
        definition,
        /// Editorial-note predicate role.
        editorial_note,
        /// Example predicate role.
        example,
        /// History-note predicate role.
        history_note,
        /// Scope-note predicate role.
        scope_note,
    }
}

iri_role_group! {
    /// Caller-owned SKOS hierarchy, mapping, membership, and top-concept roles.
    SkosRelationRoles as "struct SkosRelationRoles" in "SKOS" {
        /// Broader-concept predicate role.
        broader,
        /// Narrower-concept predicate role.
        narrower,
        /// Associative-related predicate role.
        related,
        /// Close-match predicate role.
        close_match,
        /// Exact-match predicate role.
        exact_match,
        /// Broad-match predicate role.
        broad_match,
        /// Narrow-match predicate role.
        narrow_match,
        /// Related-match predicate role.
        related_match,
        /// Concept-scheme membership predicate role.
        in_scheme,
        /// Scheme-to-top-concept predicate role.
        has_top_concept,
        /// Top-concept-to-scheme predicate role.
        top_concept_of,
    }
}

macro_rules! role_set {
    ($name:ident, $expecting:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Debug, Clone, PartialEq, Eq)]
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
                validate_distinct_roles(
                    stringify!($name),
                    self.classes
                        .named_iris()
                        .chain(self.labels.named_iris())
                        .chain(self.documentation.named_iris())
                        .chain(self.relations.named_iris()),
                )
            }
        }

        // The four role groups, read without cross-checking: [`SkosConfig`] does that.
        purrdf_lex::json_record!($name as $expecting {
            "classes" => classes: required,
            "labels" => labels: required,
            "documentation" => documentation: required,
            "relations" => relations: required,
        });
    };
}

role_set!(
    SkosSourceRoles,
    "struct SkosSourceRoles",
    "Complete caller-owned source interpretation for the RDF→SKOS projection."
);
role_set!(
    SkosTargetRoles,
    "struct SkosTargetRoles",
    "Complete caller-owned target vocabulary for the emitted SKOS view."
);

/// Source graph selection for one SKOS concept-scheme view.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkosConfig {
    source: SkosSourceRoles,
    target: SkosTargetRoles,
    scheme_iri: String,
    graph_selection: SkosGraphSelection,
    limits: ProjectionLimits,
    max_records: usize,
    document_base_iri: Option<String>,
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
            document_base_iri: None,
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

    /// Name the IRI the emitted SKOS Turtle document is published at.
    ///
    /// Turtle can express a base, so this projection declares it and relativizes against
    /// it. Distinct from [`scheme_iri`](Self::scheme_iri), which identifies the concept
    /// scheme the document is ABOUT rather than the document itself. Caller-owned with no
    /// fabricated default: unset, the document declares no base, exactly as before.
    ///
    /// # Errors
    ///
    /// Rejects a base that is not an absolute IRI.
    pub fn with_document_base_iri(
        mut self,
        document_base_iri: Option<String>,
    ) -> Result<Self, ProjectionError> {
        if let Some(base) = &document_base_iri {
            validate_absolute_iri(base, "SKOS document base IRI")?;
        }
        self.document_base_iri = document_base_iri;
        Ok(self)
    }

    /// The IRI the emitted document is published at, when the caller named one.
    pub fn document_base_iri(&self) -> Option<&str> {
        self.document_base_iri.as_deref()
    }
}

impl FromJson for SkosConfig {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "struct SkosConfig")?;
        let source = fields.required("source")?;
        let target = fields.required("target")?;
        let scheme_iri: String = fields.required("scheme_iri")?;
        let graph_selection = fields.required("graph_selection")?;
        let limits = fields.required("limits")?;
        let max_records = fields.required("max_records")?;
        let document_base_iri = fields.optional("document_base_iri")?;
        fields.deny_unknown()?;
        Ok(Self::new(
            source,
            target,
            scheme_iri,
            graph_selection,
            limits,
            max_records,
        )?
        .with_document_base_iri(document_base_iri)?)
    }
}

purrdf_lex::json_record!(impl ToJson for SkosConfig {
    "source" => source,
    "target" => target,
    "scheme_iri" => scheme_iri,
    "graph_selection" => graph_selection,
    "limits" => limits,
    "max_records" => max_records,
    "document_base_iri" => document_base_iri,
});

/// The `mode` tags of [`SkosGraphSelection`].
const GRAPH_SELECTION_MODES: &[&str] = &["default-graph", "named-graph", "union"];

impl FromJson for SkosGraphSelection {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "internally tagged enum SkosGraphSelection")?;
        let selection = match fields.tag("mode", GRAPH_SELECTION_MODES)? {
            "default-graph" => Self::DefaultGraph,
            "named-graph" => Self::NamedGraph {
                graph_iri: fields.required("graph_iri")?,
            },
            _ => Self::Union,
        };
        fields.deny_unknown()?;
        Ok(selection)
    }
}

impl ToJson for SkosGraphSelection {
    fn to_json(&self) -> Value {
        let object = match self {
            Self::DefaultGraph => Object::new().with("mode", "default-graph"),
            Self::NamedGraph { graph_iri } => Object::new()
                .with("mode", "named-graph")
                .with("graph_iri", graph_iri.as_str()),
            Self::Union => Object::new().with("mode", "union"),
        };
        Value::Object(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<SkosGraphSelection, DecodeError> {
        SkosGraphSelection::from_json(&purrdf_lex::json::read(text).expect("JSON"))
    }

    /// A field-less graph selection is exactly its tag: a member beside the tag
    /// is an unknown field, while the bare tag and the named-graph variant's own
    /// member are read.
    #[test]
    fn a_field_less_graph_selection_refuses_an_extra_member() {
        assert!(read(r#"{"mode":"union","graph_iri":"https://example.org/g"}"#).is_err());
        assert_eq!(read(r#"{"mode":"union"}"#), Ok(SkosGraphSelection::Union));
        assert_eq!(
            read(r#"{"mode":"named-graph","graph_iri":"https://example.org/g"}"#),
            Ok(SkosGraphSelection::NamedGraph {
                graph_iri: "https://example.org/g".to_owned()
            })
        );
        assert!(read(r#"{"mode":"named-graph","graph_iri":"x","extra":1}"#).is_err());
    }
}
