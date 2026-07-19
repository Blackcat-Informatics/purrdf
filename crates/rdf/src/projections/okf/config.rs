// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

use super::super::{ProjectionError, ProjectionLimits, validate_absolute_iri};
use crate::native_codecs::okf::{MAX_OKF_BUNDLE_BYTES, MAX_OKF_DOCUMENT_BYTES, MAX_OKF_DOCUMENTS};

/// Stable unified-projection profile name for caller-curated OKF bundles.
pub const OKF_TERMS_PROFILE: &str = "okf-terms";

const STANDARD_KEYS: [&str; 6] = [
    "type",
    "title",
    "description",
    "resource",
    "tags",
    "timestamp",
];

/// Explicit RDF graph scope used for concept discovery and mapped values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub enum OkfGraphSelection {
    /// Read the default graph and every declared named graph as one semantic view.
    All,
    /// Read exactly the caller-selected default/named graph identities.
    Include {
        /// Whether default-graph statements are in scope.
        default_graph: bool,
        /// Exact named-graph IRIs in scope.
        named_graphs: BTreeSet<String>,
    },
}

impl OkfGraphSelection {
    /// Construct and validate an exact graph selection.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an empty scope or relative named-graph IRI.
    pub fn include(
        default_graph: bool,
        named_graphs: BTreeSet<String>,
    ) -> Result<Self, ProjectionError> {
        let selection = Self::Include {
            default_graph,
            named_graphs,
        };
        selection.validate()?;
        Ok(selection)
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        if let Self::Include {
            default_graph,
            named_graphs,
        } = self
        {
            if !default_graph && named_graphs.is_empty() {
                return Err(ProjectionError::configuration(
                    "OKF graph selection must include at least one graph",
                ));
            }
            for graph in named_graphs {
                validate_absolute_iri(graph, "OKF selected named graph")?;
            }
        }
        Ok(())
    }

    /// Whether the default graph is selected.
    pub fn includes_default_graph(&self) -> bool {
        matches!(
            self,
            Self::All
                | Self::Include {
                    default_graph: true,
                    ..
                }
        )
    }

    /// Whether a resolved named-graph IRI is selected.
    pub fn includes_named_graph(&self, graph: &str) -> bool {
        match self {
            Self::All => true,
            Self::Include { named_graphs, .. } => named_graphs.contains(graph),
        }
    }
}

/// Caller-supplied type-set and IRI-prefix classifier for one OKF category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OkfConceptSelector {
    type_predicate: Option<String>,
    any_types: BTreeSet<String>,
    all_types: BTreeSet<String>,
    none_types: BTreeSet<String>,
    iri_prefixes: BTreeSet<String>,
}

impl OkfConceptSelector {
    /// Construct a validated declarative concept classifier.
    ///
    /// Empty type sets leave type unconstrained. Empty IRI prefixes accept every
    /// RDF subject kind; a non-empty prefix set accepts only matching IRI subjects.
    /// A type predicate is required exactly when a type constraint is present.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for incomplete, contradictory, or relative
    /// IRI policy.
    pub fn new(
        type_predicate: Option<String>,
        any_types: BTreeSet<String>,
        all_types: BTreeSet<String>,
        none_types: BTreeSet<String>,
        iri_prefixes: BTreeSet<String>,
    ) -> Result<Self, ProjectionError> {
        let selector = Self {
            type_predicate,
            any_types,
            all_types,
            none_types,
            iri_prefixes,
        };
        selector.validate()?;
        Ok(selector)
    }

    /// Predicate whose IRI objects define classifier type membership.
    pub fn type_predicate(&self) -> Option<&str> {
        self.type_predicate.as_deref()
    }

    /// Types of which at least one must be present, when non-empty.
    pub const fn any_types(&self) -> &BTreeSet<String> {
        &self.any_types
    }

    /// Types all of which must be present.
    pub const fn all_types(&self) -> &BTreeSet<String> {
        &self.all_types
    }

    /// Types none of which may be present.
    pub const fn none_types(&self) -> &BTreeSet<String> {
        &self.none_types
    }

    /// Allowed subject-IRI prefixes; empty accepts every RDF subject kind.
    pub const fn iri_prefixes(&self) -> &BTreeSet<String> {
        &self.iri_prefixes
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        let constrained =
            !(self.any_types.is_empty() && self.all_types.is_empty() && self.none_types.is_empty());
        if constrained != self.type_predicate.is_some() {
            return Err(ProjectionError::configuration(
                "OKF concept selector requires type_predicate exactly when type constraints are present",
            ));
        }
        if let Some(predicate) = &self.type_predicate {
            validate_absolute_iri(predicate, "OKF concept type predicate")?;
        }
        for (role, values) in [
            ("required-any type", &self.any_types),
            ("required-all type", &self.all_types),
            ("excluded type", &self.none_types),
        ] {
            for value in values {
                validate_absolute_iri(value, &format!("OKF selector {role}"))?;
            }
        }
        if self
            .none_types
            .iter()
            .any(|value| self.any_types.contains(value) || self.all_types.contains(value))
        {
            return Err(ProjectionError::configuration(
                "OKF concept selector cannot both require and exclude the same type",
            ));
        }
        for prefix in &self.iri_prefixes {
            validate_absolute_iri(prefix, "OKF concept subject IRI prefix")?;
        }
        Ok(())
    }
}

/// Caller-authored category metadata and classifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OkfCategory {
    directory: String,
    document_type: String,
    index_heading: String,
    index_description: String,
    selector: OkfConceptSelector,
}

impl OkfCategory {
    /// Construct one validated OKF category.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an unsafe directory, empty type/heading,
    /// multiline index metadata, or invalid classifier.
    pub fn new(
        directory: impl Into<String>,
        document_type: impl Into<String>,
        index_heading: impl Into<String>,
        index_description: impl Into<String>,
        selector: OkfConceptSelector,
    ) -> Result<Self, ProjectionError> {
        let category = Self {
            directory: directory.into(),
            document_type: document_type.into(),
            index_heading: index_heading.into(),
            index_description: index_description.into(),
            selector,
        };
        category.validate()?;
        Ok(category)
    }

    /// Safe bundle directory for this category.
    pub fn directory(&self) -> &str {
        &self.directory
    }

    /// Required OKF `type` value for category members.
    pub fn document_type(&self) -> &str {
        &self.document_type
    }

    /// Heading of the category's reserved `index.md`.
    pub fn index_heading(&self) -> &str {
        &self.index_heading
    }

    /// Short category description used by the root index.
    pub fn index_description(&self) -> &str {
        &self.index_description
    }

    /// Declarative resource classifier.
    pub const fn selector(&self) -> &OkfConceptSelector {
        &self.selector
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        validate_path_component(&self.directory, "OKF category directory")?;
        validate_nonempty_line(&self.document_type, "OKF category document type")?;
        validate_nonempty_line(&self.index_heading, "OKF category index heading")?;
        validate_single_line(&self.index_description, "OKF category index description")?;
        self.selector.validate()
    }
}

/// Deterministic bundle path identity strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub enum OkfPathStrategy {
    /// Use the fragment or final path segment of an IRI subject as a strict stem.
    SubjectLocalName,
    /// Read exactly one mapped value and require it to be a strict safe stem.
    Predicate {
        /// Predicate whose value supplies the filename stem.
        predicate: String,
        /// Total term-to-text rendering applied before path validation.
        rendering: OkfTermRendering,
    },
    /// Use the full SHA-256 digest of the canonical RDF term identity.
    StableHash {
        /// Safe ASCII stem prefix prepended to the digest.
        prefix: String,
    },
}

impl OkfPathStrategy {
    /// Construct a strict mapped path strategy.
    ///
    /// # Errors
    ///
    /// Returns a configuration error unless the predicate is an absolute IRI.
    pub fn predicate(
        predicate: impl Into<String>,
        rendering: OkfTermRendering,
    ) -> Result<Self, ProjectionError> {
        let strategy = Self::Predicate {
            predicate: predicate.into(),
            rendering,
        };
        strategy.validate()?;
        Ok(strategy)
    }

    /// Construct a canonical-hash path strategy.
    ///
    /// # Errors
    ///
    /// Returns a configuration error unless `prefix` is a safe path stem.
    pub fn stable_hash(prefix: impl Into<String>) -> Result<Self, ProjectionError> {
        let strategy = Self::StableHash {
            prefix: prefix.into(),
        };
        strategy.validate()?;
        Ok(strategy)
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        match self {
            Self::SubjectLocalName => Ok(()),
            Self::Predicate { predicate, .. } => {
                validate_absolute_iri(predicate, "OKF path predicate")
            }
            Self::StableHash { prefix } => validate_path_stem(prefix, "OKF hash path prefix"),
        }
    }

    /// Mapped path predicate, when this strategy uses one.
    pub fn predicate_iri(&self) -> Option<&str> {
        match self {
            Self::Predicate { predicate, .. } => Some(predicate),
            Self::SubjectLocalName | Self::StableHash { .. } => None,
        }
    }
}

/// Total textual rendering for arbitrary RDF 1.2 term values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OkfTermRendering {
    /// Human-oriented lexical text: full IRI, blank label, literal lexical form,
    /// or recursively rendered quoted-triple syntax.
    Lexical,
    /// Lossless identity text retaining term kind and every literal/triple facet.
    Canonical,
}

/// Typed scalar policy for one mapped RDF object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub enum OkfValueMode {
    /// Always emit a YAML string using a total RDF-term renderer.
    Text {
        /// Textual identity policy.
        rendering: OkfTermRendering,
    },
    /// Require an IRI object and emit its absolute value as a YAML string.
    Iri,
    /// Require an XSD boolean literal and emit a YAML boolean.
    Boolean,
    /// Require an XSD integer-family literal and emit its canonical numeric lexical form.
    Integer,
    /// Require an XSD decimal literal and emit its canonical non-exponent lexical form.
    Decimal,
    /// Require an XSD dateTime literal and emit its canonical lexical form as a string.
    DateTime,
}

/// Output cardinality and missing-value policy for one mapped field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OkfCardinality {
    /// Omit the field when absent and reject more than one distinct value.
    ZeroOrOne,
    /// Require exactly one distinct value.
    One,
    /// Emit every distinct value as a deterministically sorted YAML sequence.
    Many,
}

/// Predicate set, cardinality, and value policy for one output field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OkfFieldMapping {
    predicates: BTreeSet<String>,
    cardinality: OkfCardinality,
    value_mode: OkfValueMode,
}

impl OkfFieldMapping {
    /// Construct a validated field mapping.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an empty predicate set or relative IRI.
    pub fn new(
        predicates: BTreeSet<String>,
        cardinality: OkfCardinality,
        value_mode: OkfValueMode,
    ) -> Result<Self, ProjectionError> {
        let mapping = Self {
            predicates,
            cardinality,
            value_mode,
        };
        mapping.validate()?;
        Ok(mapping)
    }

    /// Source predicate IRIs in lexical order.
    pub const fn predicates(&self) -> &BTreeSet<String> {
        &self.predicates
    }

    /// Output cardinality.
    pub const fn cardinality(&self) -> OkfCardinality {
        self.cardinality
    }

    /// Typed scalar policy.
    pub const fn value_mode(&self) -> OkfValueMode {
        self.value_mode
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        if self.predicates.is_empty() {
            return Err(ProjectionError::configuration(
                "OKF field mapping requires at least one predicate",
            ));
        }
        for predicate in &self.predicates {
            validate_absolute_iri(predicate, "OKF field predicate")?;
        }
        Ok(())
    }

    fn validate_scalar(&self, field: &str) -> Result<(), ProjectionError> {
        if self.cardinality == OkfCardinality::Many {
            return Err(ProjectionError::configuration(format!(
                "OKF standard `{field}` mapping must be scalar"
            )));
        }
        Ok(())
    }

    fn validate_mode(
        &self,
        field: &str,
        expected: fn(OkfValueMode) -> bool,
    ) -> Result<(), ProjectionError> {
        if !expected(self.value_mode) {
            return Err(ProjectionError::configuration(format!(
                "OKF standard `{field}` mapping has an incompatible value mode"
            )));
        }
        Ok(())
    }
}

/// Caller-owned policy for the standard `resource` frontmatter field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub enum OkfResourceMapping {
    /// Omit `resource` for every concept.
    Omit,
    /// Emit an IRI concept subject as its own resource and reject non-IRI subjects.
    Subject,
    /// Map exactly zero/one or exactly one predicate object as an IRI resource.
    Predicate {
        /// Scalar IRI field mapping.
        mapping: OkfFieldMapping,
    },
}

impl OkfResourceMapping {
    /// Construct a validated predicate-backed resource mapping.
    ///
    /// # Errors
    ///
    /// Returns a configuration error unless the mapping is scalar and IRI-valued.
    pub fn predicate(mapping: OkfFieldMapping) -> Result<Self, ProjectionError> {
        let resource = Self::Predicate { mapping };
        resource.validate()?;
        Ok(resource)
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        match self {
            Self::Omit | Self::Subject => Ok(()),
            Self::Predicate { mapping } => {
                mapping.validate()?;
                mapping.validate_scalar("resource")?;
                mapping.validate_mode("resource", |mode| mode == OkfValueMode::Iri)
            }
        }
    }
}

/// Complete mapping for standard and producer-defined OKF frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OkfFrontmatterMappings {
    title: Option<OkfFieldMapping>,
    description: Option<OkfFieldMapping>,
    resource: OkfResourceMapping,
    tags: Option<OkfFieldMapping>,
    timestamp: Option<OkfFieldMapping>,
    extensions: BTreeMap<String, OkfFieldMapping>,
}

impl OkfFrontmatterMappings {
    /// Construct and validate every frontmatter role.
    ///
    /// `type` is supplied by category classification. Other standard fields may be
    /// intentionally absent, as allowed by OKF v0.1. Extension keys are emitted
    /// after standard keys in lexical order.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for a standard-key extension collision,
    /// unsafe key, invalid nested mapping, incompatible value mode, or incompatible
    /// cardinality.
    pub fn new(
        title: Option<OkfFieldMapping>,
        description: Option<OkfFieldMapping>,
        resource: OkfResourceMapping,
        tags: Option<OkfFieldMapping>,
        timestamp: Option<OkfFieldMapping>,
        extensions: BTreeMap<String, OkfFieldMapping>,
    ) -> Result<Self, ProjectionError> {
        let mappings = Self {
            title,
            description,
            resource,
            tags,
            timestamp,
            extensions,
        };
        mappings.validate()?;
        Ok(mappings)
    }

    /// Optional title mapping.
    pub const fn title(&self) -> Option<&OkfFieldMapping> {
        self.title.as_ref()
    }

    /// Optional one-line description mapping.
    pub const fn description(&self) -> Option<&OkfFieldMapping> {
        self.description.as_ref()
    }

    /// Resource-field policy.
    pub const fn resource(&self) -> &OkfResourceMapping {
        &self.resource
    }

    /// Optional tags mapping.
    pub const fn tags(&self) -> Option<&OkfFieldMapping> {
        self.tags.as_ref()
    }

    /// Optional timestamp mapping.
    pub const fn timestamp(&self) -> Option<&OkfFieldMapping> {
        self.timestamp.as_ref()
    }

    /// Producer-defined extension fields in lexical key order.
    pub const fn extensions(&self) -> &BTreeMap<String, OkfFieldMapping> {
        &self.extensions
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        for (name, mapping) in [
            ("title", self.title.as_ref()),
            ("description", self.description.as_ref()),
        ] {
            if let Some(mapping) = mapping {
                mapping.validate()?;
                mapping.validate_scalar(name)?;
                mapping.validate_mode(name, |mode| matches!(mode, OkfValueMode::Text { .. }))?;
            }
        }
        self.resource.validate()?;
        if let Some(tags) = &self.tags {
            tags.validate()?;
            if tags.cardinality != OkfCardinality::Many {
                return Err(ProjectionError::configuration(
                    "OKF standard `tags` mapping must use many cardinality",
                ));
            }
            tags.validate_mode("tags", |mode| matches!(mode, OkfValueMode::Text { .. }))?;
        }
        if let Some(timestamp) = &self.timestamp {
            timestamp.validate()?;
            timestamp.validate_scalar("timestamp")?;
            timestamp.validate_mode("timestamp", |mode| mode == OkfValueMode::DateTime)?;
        }
        for (key, mapping) in &self.extensions {
            validate_frontmatter_key(key)?;
            if STANDARD_KEYS.contains(&key.as_str()) {
                return Err(ProjectionError::configuration(format!(
                    "OKF extension key `{key}` collides with a standard frontmatter key"
                )));
            }
            mapping.validate()?;
        }
        Ok(())
    }
}

/// How mapped body values are represented before Markdown layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub enum OkfBodyValueMode {
    /// Render every RDF term to escaped Markdown text.
    Text {
        /// Total term-to-text policy.
        rendering: OkfTermRendering,
    },
    /// Require string-like literals and preserve their lexical forms as authored Markdown.
    MarkdownLiteral,
}

/// Structural layout for values in a body or link section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OkfBodyStyle {
    /// One value per paragraph.
    Paragraphs,
    /// One value per `- ` list item.
    Bullets,
}

/// Caller-authored Markdown body section backed by one or more predicates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OkfBodySection {
    heading: Option<String>,
    predicates: BTreeSet<String>,
    style: OkfBodyStyle,
    value_mode: OkfBodyValueMode,
}

impl OkfBodySection {
    /// Construct a validated body section.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an empty predicate set, relative predicate,
    /// or empty/multiline heading.
    pub fn new(
        heading: Option<String>,
        predicates: BTreeSet<String>,
        style: OkfBodyStyle,
        value_mode: OkfBodyValueMode,
    ) -> Result<Self, ProjectionError> {
        let section = Self {
            heading,
            predicates,
            style,
            value_mode,
        };
        section.validate()?;
        Ok(section)
    }

    /// Optional level-two Markdown heading.
    pub fn heading(&self) -> Option<&str> {
        self.heading.as_deref()
    }

    /// Source predicates in lexical order.
    pub const fn predicates(&self) -> &BTreeSet<String> {
        &self.predicates
    }

    /// Body layout.
    pub const fn style(&self) -> OkfBodyStyle {
        self.style
    }

    /// Value representation policy.
    pub const fn value_mode(&self) -> OkfBodyValueMode {
        self.value_mode
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        validate_predicates(&self.predicates, "OKF body")?;
        if let Some(heading) = &self.heading {
            validate_nonempty_line(heading, "OKF body heading")?;
        }
        Ok(())
    }
}

/// Which link targets a section renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OkfLinkTargetMode {
    /// Render only targets that are selected concept documents.
    InternalOnly,
    /// Also render absolute IRI targets that are not documents in this bundle.
    IncludeExternalIris,
}

/// Link-destination policy for selected concept documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OkfLinkPathStyle {
    /// Standard relative paths from the source document.
    Relative,
    /// Leading-slash bundle-relative paths recommended by OKF v0.1.
    BundleAbsolute,
}

/// Structural layout for one set of Markdown links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OkfLinkStyle {
    /// One link per bullet.
    Bullets,
    /// One link per paragraph.
    Paragraphs,
}

/// Caller-authored Markdown link section backed by RDF predicates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OkfLinkSection {
    heading: Option<String>,
    predicates: BTreeSet<String>,
    relation_label: Option<String>,
    style: OkfLinkStyle,
    path_style: OkfLinkPathStyle,
    targets: OkfLinkTargetMode,
}

impl OkfLinkSection {
    /// Construct a validated link section.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an empty predicate set, relative predicate,
    /// or empty/multiline heading/relation label.
    #[allow(
        clippy::too_many_arguments,
        reason = "each mandatory link-rendering axis is independent caller policy"
    )]
    pub fn new(
        heading: Option<String>,
        predicates: BTreeSet<String>,
        relation_label: Option<String>,
        style: OkfLinkStyle,
        path_style: OkfLinkPathStyle,
        targets: OkfLinkTargetMode,
    ) -> Result<Self, ProjectionError> {
        let section = Self {
            heading,
            predicates,
            relation_label,
            style,
            path_style,
            targets,
        };
        section.validate()?;
        Ok(section)
    }

    /// Optional level-two Markdown heading.
    pub fn heading(&self) -> Option<&str> {
        self.heading.as_deref()
    }

    /// Source predicates in lexical order.
    pub const fn predicates(&self) -> &BTreeSet<String> {
        &self.predicates
    }

    /// Optional prose prefix before each link.
    pub fn relation_label(&self) -> Option<&str> {
        self.relation_label.as_deref()
    }

    /// Link layout.
    pub const fn style(&self) -> OkfLinkStyle {
        self.style
    }

    /// Internal destination policy.
    pub const fn path_style(&self) -> OkfLinkPathStyle {
        self.path_style
    }

    /// Target inclusion policy.
    pub const fn targets(&self) -> OkfLinkTargetMode {
        self.targets
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        validate_predicates(&self.predicates, "OKF link")?;
        if let Some(heading) = &self.heading {
            validate_nonempty_line(heading, "OKF link heading")?;
        }
        if let Some(label) = &self.relation_label {
            validate_nonempty_line(label, "OKF link relation label")?;
        }
        Ok(())
    }
}

/// Caller-authored root-index and in-band projection-fidelity prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OkfIndexConfig {
    root_heading: String,
    categories_heading: String,
    fidelity_heading: String,
    loss_declaration: String,
}

impl OkfIndexConfig {
    /// Construct validated index prose.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an empty/multiline heading or empty loss
    /// declaration.
    pub fn new(
        root_heading: impl Into<String>,
        categories_heading: impl Into<String>,
        fidelity_heading: impl Into<String>,
        loss_declaration: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let index = Self {
            root_heading: root_heading.into(),
            categories_heading: categories_heading.into(),
            fidelity_heading: fidelity_heading.into(),
            loss_declaration: loss_declaration.into(),
        };
        index.validate()?;
        Ok(index)
    }

    /// Root `index.md` level-one heading.
    pub fn root_heading(&self) -> &str {
        &self.root_heading
    }

    /// Root category-list level-two heading.
    pub fn categories_heading(&self) -> &str {
        &self.categories_heading
    }

    /// Root fidelity-section level-two heading.
    pub fn fidelity_heading(&self) -> &str {
        &self.fidelity_heading
    }

    /// Exact caller-authored in-band loss declaration.
    pub fn loss_declaration(&self) -> &str {
        &self.loss_declaration
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        validate_nonempty_line(&self.root_heading, "OKF root index heading")?;
        validate_nonempty_line(&self.categories_heading, "OKF root categories heading")?;
        validate_nonempty_line(&self.fidelity_heading, "OKF fidelity heading")?;
        if self.loss_declaration.trim().is_empty() {
            return Err(ProjectionError::configuration(
                "OKF loss declaration must not be empty",
            ));
        }
        if self.loss_declaration.contains('\0') {
            return Err(ProjectionError::configuration(
                "OKF loss declaration must not contain NUL",
            ));
        }
        Ok(())
    }
}

/// Complete mandatory configuration for the write-only `okf-terms` projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OkfGenerationConfig {
    graph_selection: OkfGraphSelection,
    categories: BTreeMap<String, OkfCategory>,
    path_strategy: OkfPathStrategy,
    frontmatter: OkfFrontmatterMappings,
    body_sections: Vec<OkfBodySection>,
    link_sections: Vec<OkfLinkSection>,
    index: OkfIndexConfig,
    limits: ProjectionLimits,
    max_records: usize,
    max_concepts: usize,
    max_values_per_field: usize,
}

impl OkfGenerationConfig {
    /// Construct a complete, validated OKF generation profile.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for missing/duplicate/unsafe categories,
    /// invalid nested mappings, contradictory or non-portable bounds, or an artifact
    /// budget that cannot contain the declared maximum concept/category indexes.
    #[allow(
        clippy::too_many_arguments,
        reason = "the constructor requires every independent semantic and resource policy"
    )]
    pub fn new(
        graph_selection: OkfGraphSelection,
        categories: BTreeMap<String, OkfCategory>,
        path_strategy: OkfPathStrategy,
        frontmatter: OkfFrontmatterMappings,
        body_sections: Vec<OkfBodySection>,
        link_sections: Vec<OkfLinkSection>,
        index: OkfIndexConfig,
        limits: ProjectionLimits,
        max_records: usize,
        max_concepts: usize,
        max_values_per_field: usize,
    ) -> Result<Self, ProjectionError> {
        let config = Self {
            graph_selection,
            categories,
            path_strategy,
            frontmatter,
            body_sections,
            link_sections,
            index,
            limits,
            max_records,
            max_concepts,
            max_values_per_field,
        };
        config.validate()?;
        Ok(config)
    }

    /// Explicit RDF graph scope.
    pub const fn graph_selection(&self) -> &OkfGraphSelection {
        &self.graph_selection
    }

    /// Category classifiers in deterministic category-key order.
    pub const fn categories(&self) -> &BTreeMap<String, OkfCategory> {
        &self.categories
    }

    /// Stable concept-path strategy.
    pub const fn path_strategy(&self) -> &OkfPathStrategy {
        &self.path_strategy
    }

    /// Standard and producer-defined frontmatter mappings.
    pub const fn frontmatter(&self) -> &OkfFrontmatterMappings {
        &self.frontmatter
    }

    /// Body sections in caller-declared semantic order.
    pub fn body_sections(&self) -> &[OkfBodySection] {
        &self.body_sections
    }

    /// Link sections in caller-declared semantic order.
    pub fn link_sections(&self) -> &[OkfLinkSection] {
        &self.link_sections
    }

    /// Root/category index prose policy.
    pub const fn index(&self) -> &OkfIndexConfig {
        &self.index
    }

    /// Shared package and recursive-term limits.
    pub const fn limits(&self) -> ProjectionLimits {
        self.limits
    }

    /// Maximum source named-graph/quad/reifier/annotation records.
    pub const fn max_records(&self) -> usize {
        self.max_records
    }

    /// Maximum selected concept documents.
    pub const fn max_concepts(&self) -> usize {
        self.max_concepts
    }

    /// Maximum distinct values accepted by one mapped field/section on one concept.
    pub const fn max_values_per_field(&self) -> usize {
        self.max_values_per_field
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        self.graph_selection.validate()?;
        if self.categories.is_empty() {
            return Err(ProjectionError::configuration(
                "OKF generation requires at least one category",
            ));
        }
        let mut directories = BTreeSet::new();
        for (key, category) in &self.categories {
            validate_frontmatter_key(key)?;
            category.validate()?;
            if !directories.insert(category.directory.clone()) {
                return Err(ProjectionError::configuration(format!(
                    "duplicate OKF category directory `{}`",
                    category.directory
                )));
            }
        }
        self.path_strategy.validate()?;
        self.frontmatter.validate()?;
        for section in &self.body_sections {
            section.validate()?;
        }
        for section in &self.link_sections {
            section.validate()?;
        }
        self.index.validate()?;

        for (name, value) in [
            ("max_records", self.max_records),
            ("max_concepts", self.max_concepts),
            ("max_values_per_field", self.max_values_per_field),
        ] {
            if value == 0 {
                return Err(ProjectionError::configuration(format!(
                    "OKF {name} must be greater than zero"
                )));
            }
            if u32::try_from(value).is_err() {
                return Err(ProjectionError::configuration(format!(
                    "OKF {name} exceeds the portable u32 ceiling"
                )));
            }
        }
        let maximum_documents = self
            .max_concepts
            .checked_add(self.categories.len())
            .and_then(|count| count.checked_add(1))
            .ok_or_else(|| ProjectionError::configuration("OKF artifact count overflow"))?;
        if maximum_documents > self.limits.max_artifacts() {
            return Err(ProjectionError::configuration(format!(
                "OKF package max_artifacts {} cannot contain max_concepts {} plus {} category indexes and the root index",
                self.limits.max_artifacts(),
                self.max_concepts,
                self.categories.len()
            )));
        }
        if maximum_documents > MAX_OKF_DOCUMENTS {
            return Err(ProjectionError::configuration(format!(
                "OKF declared maximum document count {maximum_documents} exceeds the codec ceiling {MAX_OKF_DOCUMENTS}"
            )));
        }
        if self.limits.max_artifact_bytes() > MAX_OKF_DOCUMENT_BYTES {
            return Err(ProjectionError::configuration(format!(
                "OKF max_artifact_bytes {} exceeds the codec document ceiling {MAX_OKF_DOCUMENT_BYTES}",
                self.limits.max_artifact_bytes()
            )));
        }
        if self.limits.max_total_bytes() > MAX_OKF_BUNDLE_BYTES {
            return Err(ProjectionError::configuration(format!(
                "OKF max_total_bytes {} exceeds the codec bundle ceiling {MAX_OKF_BUNDLE_BYTES}",
                self.limits.max_total_bytes()
            )));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOkfGenerationConfig {
    graph_selection: OkfGraphSelection,
    categories: BTreeMap<String, OkfCategory>,
    path_strategy: OkfPathStrategy,
    frontmatter: OkfFrontmatterMappings,
    body_sections: Vec<OkfBodySection>,
    link_sections: Vec<OkfLinkSection>,
    index: OkfIndexConfig,
    limits: ProjectionLimits,
    max_records: usize,
    max_concepts: usize,
    max_values_per_field: usize,
}

impl<'de> Deserialize<'de> for OkfGenerationConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawOkfGenerationConfig::deserialize(deserializer)?;
        Self::new(
            raw.graph_selection,
            raw.categories,
            raw.path_strategy,
            raw.frontmatter,
            raw.body_sections,
            raw.link_sections,
            raw.index,
            raw.limits,
            raw.max_records,
            raw.max_concepts,
            raw.max_values_per_field,
        )
        .map_err(serde::de::Error::custom)
    }
}

pub(crate) fn validate_frontmatter_key(key: &str) -> Result<(), ProjectionError> {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err(ProjectionError::configuration(
            "OKF frontmatter keys must not be empty",
        ));
    };
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
    {
        return Err(ProjectionError::configuration(format!(
            "unsafe OKF frontmatter key `{key}`; expected an ASCII identifier"
        )));
    }
    Ok(())
}

pub(crate) fn validate_path_stem(value: &str, role: &str) -> Result<(), ProjectionError> {
    if value.eq_ignore_ascii_case("index") || value.eq_ignore_ascii_case("log") {
        return Err(ProjectionError::configuration(format!(
            "{role} `{value}` collides with an OKF reserved filename"
        )));
    }
    validate_path_component(value, role)
}

fn validate_path_component(value: &str, role: &str) -> Result<(), ProjectionError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.starts_with('.')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(ProjectionError::configuration(format!(
            "{role} `{value}` must be a non-hidden ASCII path component containing only letters, digits, `.`, `_`, or `-`"
        )));
    }
    Ok(())
}

fn validate_predicates(predicates: &BTreeSet<String>, role: &str) -> Result<(), ProjectionError> {
    if predicates.is_empty() {
        return Err(ProjectionError::configuration(format!(
            "{role} section requires at least one predicate"
        )));
    }
    for predicate in predicates {
        validate_absolute_iri(predicate, &format!("{role} predicate"))?;
    }
    Ok(())
}

fn validate_nonempty_line(value: &str, role: &str) -> Result<(), ProjectionError> {
    if value.trim().is_empty() {
        return Err(ProjectionError::configuration(format!(
            "{role} must not be empty"
        )));
    }
    validate_single_line(value, role)
}

fn validate_single_line(value: &str, role: &str) -> Result<(), ProjectionError> {
    if value.contains(['\n', '\r', '\0']) {
        return Err(ProjectionError::configuration(format!(
            "{role} must be a single NUL-free line"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
    const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";

    fn text_mapping(cardinality: OkfCardinality) -> OkfFieldMapping {
        OkfFieldMapping::new(
            BTreeSet::from([LABEL.to_owned()]),
            cardinality,
            OkfValueMode::Text {
                rendering: OkfTermRendering::Lexical,
            },
        )
        .expect("mapping")
    }

    fn config() -> OkfGenerationConfig {
        let selector = OkfConceptSelector::new(
            Some(RDF_TYPE.to_owned()),
            BTreeSet::from([OWL_CLASS.to_owned()]),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::from(["https://example.org/".to_owned()]),
        )
        .expect("selector");
        let category =
            OkfCategory::new("classes", "Class", "Classes", "Ontology classes.", selector)
                .expect("category");
        let frontmatter = OkfFrontmatterMappings::new(
            Some(text_mapping(OkfCardinality::ZeroOrOne)),
            None,
            OkfResourceMapping::Subject,
            Some(text_mapping(OkfCardinality::Many)),
            None,
            BTreeMap::from([("label_copy".to_owned(), text_mapping(OkfCardinality::Many))]),
        )
        .expect("frontmatter");
        let limits =
            ProjectionLimits::new(16, 1_000_000, 4_000_000, 5_000_000, 16).expect("limits");
        OkfGenerationConfig::new(
            OkfGraphSelection::Include {
                default_graph: true,
                named_graphs: BTreeSet::new(),
            },
            BTreeMap::from([("class".to_owned(), category)]),
            OkfPathStrategy::SubjectLocalName,
            frontmatter,
            vec![
                OkfBodySection::new(
                    None,
                    BTreeSet::from([LABEL.to_owned()]),
                    OkfBodyStyle::Paragraphs,
                    OkfBodyValueMode::Text {
                        rendering: OkfTermRendering::Lexical,
                    },
                )
                .expect("body"),
            ],
            Vec::new(),
            OkfIndexConfig::new(
                "Example ontology",
                "Categories",
                "Projection fidelity",
                "Only caller-mapped terms are represented.",
            )
            .expect("index"),
            limits,
            1_000,
            10,
            100,
        )
        .expect("config")
    }

    #[test]
    fn complete_configuration_round_trips_and_revalidates_json() {
        let config = config();
        let bytes = serde_json::to_vec(&config).expect("serialize");
        assert_eq!(
            serde_json::from_slice::<OkfGenerationConfig>(&bytes).expect("deserialize"),
            config
        );

        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
        value["categories"]["class"]["directory"] = serde_json::json!("../escape");
        let invalid = serde_json::to_vec(&value).expect("invalid JSON bytes");
        assert!(serde_json::from_slice::<OkfGenerationConfig>(&invalid).is_err());
    }

    #[test]
    fn configuration_rejects_ambiguity_and_hidden_optionality() {
        assert!(
            OkfConceptSelector::new(
                None,
                BTreeSet::from([OWL_CLASS.to_owned()]),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
            )
            .is_err()
        );
        assert!(OkfPathStrategy::stable_hash("index").is_err());
        assert!(
            OkfFieldMapping::new(BTreeSet::new(), OkfCardinality::One, OkfValueMode::Iri,).is_err()
        );
        assert!(
            OkfFrontmatterMappings::new(
                Some(text_mapping(OkfCardinality::Many)),
                None,
                OkfResourceMapping::Omit,
                None,
                None,
                BTreeMap::new(),
            )
            .is_err()
        );
        assert!(
            OkfFrontmatterMappings::new(
                None,
                None,
                OkfResourceMapping::Omit,
                None,
                None,
                BTreeMap::from([("type".to_owned(), text_mapping(OkfCardinality::Many))]),
            )
            .is_err()
        );
    }
}
