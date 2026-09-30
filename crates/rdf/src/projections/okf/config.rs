// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::super::selection::{GraphSelection, SubjectSelector};
use crate::projections::util::validated;
use std::collections::{BTreeMap, BTreeSet};

use purrdf_lex::json::{Object, Value};

use super::super::{ProjectionError, ProjectionLimits, validate_absolute_iri};
use crate::native_codecs::okf::{MAX_OKF_BUNDLE_BYTES, MAX_OKF_DOCUMENT_BYTES, MAX_OKF_DOCUMENTS};
use purrdf_lex::json::record::{DecodeError, FromJson, Record, ToJson};
use purrdf_lex::json_string_enum;

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
pub type OkfGraphSelection = GraphSelection;

/// Caller-supplied type-set and IRI-prefix classifier for one OKF category.
pub type OkfConceptSelector = SubjectSelector;

/// Caller-authored category metadata and classifier.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkfTermRendering {
    /// Human-oriented lexical text: full IRI, blank label, literal lexical form,
    /// or recursively rendered quoted-triple syntax.
    Lexical,
    /// Lossless identity text retaining term kind and every literal/triple facet.
    Canonical,
}

/// Typed scalar policy for one mapped RDF object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkfCardinality {
    /// Omit the field when absent and reject more than one distinct value.
    ZeroOrOne,
    /// Require exactly one distinct value.
    One,
    /// Emit every distinct value as a deterministically sorted YAML sequence.
    Many,
}

/// Predicate set, cardinality, and value policy for one output field.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
        validated(
            Self {
                title,
                description,
                resource,
                tags,
                timestamp,
                extensions,
            },
            Self::validate,
        )
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkfBodyStyle {
    /// One value per paragraph.
    Paragraphs,
    /// One value per `- ` list item.
    Bullets,
}

/// Caller-authored Markdown body section backed by one or more predicates.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        validated(
            Self {
                heading,
                predicates,
                style,
                value_mode,
            },
            Self::validate,
        )
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkfLinkTargetMode {
    /// Render only targets that are selected concept documents.
    InternalOnly,
    /// Also render absolute IRI targets that are not documents in this bundle.
    IncludeExternalIris,
}

/// Link-destination policy for selected concept documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkfLinkPathStyle {
    /// Standard relative paths from the source document.
    Relative,
    /// Leading-slash bundle-relative paths recommended by OKF v0.1.
    BundleAbsolute,
}

/// Structural layout for one set of Markdown links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkfLinkStyle {
    /// One link per bullet.
    Bullets,
    /// One link per paragraph.
    Paragraphs,
}

/// Caller-authored Markdown link section backed by RDF predicates.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        validated(
            Self {
                heading,
                predicates,
                relation_label,
                style,
                path_style,
                targets,
            },
            Self::validate,
        )
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

json_string_enum!(OkfTermRendering {
    Lexical => "lexical",
    Canonical => "canonical",
});

json_string_enum!(OkfCardinality {
    ZeroOrOne => "zero-or-one",
    One => "one",
    Many => "many",
});

json_string_enum!(OkfBodyStyle {
    Paragraphs => "paragraphs",
    Bullets => "bullets",
});

json_string_enum!(OkfLinkTargetMode {
    InternalOnly => "internal-only",
    IncludeExternalIris => "include-external-iris",
});

json_string_enum!(OkfLinkPathStyle {
    Relative => "relative",
    BundleAbsolute => "bundle-absolute",
});

json_string_enum!(OkfLinkStyle {
    Bullets => "bullets",
    Paragraphs => "paragraphs",
});

/// `{"kind": …}` plus the variant's members, and no other member.
fn tagged(kind: &str) -> Object {
    Object::new().with("kind", kind)
}

purrdf_lex::json_record!(OkfCategory as "struct OkfCategory" {
    "directory" => directory: required,
    "document_type" => document_type: required,
    "index_heading" => index_heading: required,
    "index_description" => index_description: required,
    "selector" => selector: required,
});

impl FromJson for OkfPathStrategy {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "internally tagged enum OkfPathStrategy")?;
        let strategy =
            match fields.tag("kind", &["subject-local-name", "predicate", "stable-hash"])? {
                "subject-local-name" => Self::SubjectLocalName,
                "predicate" => Self::Predicate {
                    predicate: fields.required("predicate")?,
                    rendering: fields.required("rendering")?,
                },
                _ => Self::StableHash {
                    prefix: fields.required("prefix")?,
                },
            };
        fields.deny_unknown()?;
        Ok(strategy)
    }
}

impl ToJson for OkfPathStrategy {
    fn to_json(&self) -> Value {
        Value::Object(match self {
            Self::SubjectLocalName => tagged("subject-local-name"),
            Self::Predicate {
                predicate,
                rendering,
            } => tagged("predicate")
                .with("predicate", predicate.as_str())
                .with("rendering", rendering.to_json()),
            Self::StableHash { prefix } => tagged("stable-hash").with("prefix", prefix.as_str()),
        })
    }
}

impl FromJson for OkfValueMode {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "internally tagged enum OkfValueMode")?;
        let mode = match fields.tag(
            "kind",
            &["text", "iri", "boolean", "integer", "decimal", "date-time"],
        )? {
            "text" => Self::Text {
                rendering: fields.required("rendering")?,
            },
            "iri" => Self::Iri,
            "boolean" => Self::Boolean,
            "integer" => Self::Integer,
            "decimal" => Self::Decimal,
            _ => Self::DateTime,
        };
        fields.deny_unknown()?;
        Ok(mode)
    }
}

impl ToJson for OkfValueMode {
    fn to_json(&self) -> Value {
        Value::Object(match self {
            Self::Text { rendering } => tagged("text").with("rendering", rendering.to_json()),
            Self::Iri => tagged("iri"),
            Self::Boolean => tagged("boolean"),
            Self::Integer => tagged("integer"),
            Self::Decimal => tagged("decimal"),
            Self::DateTime => tagged("date-time"),
        })
    }
}

purrdf_lex::json_record!(OkfFieldMapping as "struct OkfFieldMapping" {
    "predicates" => predicates: required,
    "cardinality" => cardinality: required,
    "value_mode" => value_mode: required,
});

impl FromJson for OkfResourceMapping {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "internally tagged enum OkfResourceMapping")?;
        let resource = match fields.tag("kind", &["omit", "subject", "predicate"])? {
            "omit" => Self::Omit,
            "subject" => Self::Subject,
            _ => Self::Predicate {
                mapping: fields.required("mapping")?,
            },
        };
        fields.deny_unknown()?;
        Ok(resource)
    }
}

impl ToJson for OkfResourceMapping {
    fn to_json(&self) -> Value {
        Value::Object(match self {
            Self::Omit => tagged("omit"),
            Self::Subject => tagged("subject"),
            Self::Predicate { mapping } => tagged("predicate").with("mapping", mapping.to_json()),
        })
    }
}

purrdf_lex::json_record!(OkfFrontmatterMappings as "struct OkfFrontmatterMappings" {
    "title" => title: optional,
    "description" => description: optional,
    "resource" => resource: required,
    "tags" => tags: optional,
    "timestamp" => timestamp: optional,
    "extensions" => extensions: required,
});

impl FromJson for OkfBodyValueMode {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "internally tagged enum OkfBodyValueMode")?;
        let mode = match fields.tag("kind", &["text", "markdown-literal"])? {
            "text" => Self::Text {
                rendering: fields.required("rendering")?,
            },
            _ => Self::MarkdownLiteral,
        };
        fields.deny_unknown()?;
        Ok(mode)
    }
}

impl ToJson for OkfBodyValueMode {
    fn to_json(&self) -> Value {
        Value::Object(match self {
            Self::Text { rendering } => tagged("text").with("rendering", rendering.to_json()),
            Self::MarkdownLiteral => tagged("markdown-literal"),
        })
    }
}

purrdf_lex::json_record!(OkfBodySection as "struct OkfBodySection" {
    "heading" => heading: optional,
    "predicates" => predicates: required,
    "style" => style: required,
    "value_mode" => value_mode: required,
});

purrdf_lex::json_record!(OkfLinkSection as "struct OkfLinkSection" {
    "heading" => heading: optional,
    "predicates" => predicates: required,
    "relation_label" => relation_label: optional,
    "style" => style: required,
    "path_style" => path_style: required,
    "targets" => targets: required,
});

purrdf_lex::json_record!(OkfIndexConfig as "struct OkfIndexConfig" {
    "root_heading" => root_heading: required,
    "categories_heading" => categories_heading: required,
    "fidelity_heading" => fidelity_heading: required,
    "loss_declaration" => loss_declaration: required,
});

impl FromJson for OkfGenerationConfig {
    /// Every member, then [`OkfGenerationConfig::new`]'s whole-profile checks.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "struct OkfGenerationConfig")?;
        let graph_selection = fields.required("graph_selection")?;
        let categories = fields.required("categories")?;
        let path_strategy = fields.required("path_strategy")?;
        let frontmatter = fields.required("frontmatter")?;
        let body_sections = fields.required("body_sections")?;
        let link_sections = fields.required("link_sections")?;
        let index = fields.required("index")?;
        let limits = fields.required("limits")?;
        let max_records = fields.required("max_records")?;
        let max_concepts = fields.required("max_concepts")?;
        let max_values_per_field = fields.required("max_values_per_field")?;
        fields.deny_unknown()?;
        Ok(Self::new(
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
        )?)
    }
}

purrdf_lex::json_record!(impl ToJson for OkfGenerationConfig {
    "graph_selection" => graph_selection,
    "categories" => categories,
    "path_strategy" => path_strategy,
    "frontmatter" => frontmatter,
    "body_sections" => body_sections,
    "link_sections" => link_sections,
    "index" => index,
    "limits" => limits,
    "max_records" => max_records,
    "max_concepts" => max_concepts,
    "max_values_per_field" => max_values_per_field,
});

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
    use purrdf_lex::json::record::{from_slice, to_vec};

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
        let bytes = to_vec(&config);
        assert_eq!(
            from_slice::<OkfGenerationConfig>(&bytes).expect("deserialize"),
            config
        );

        let mut value = config.to_json();
        value["categories"]["class"]["directory"] = Value::from("../escape");
        assert!(OkfGenerationConfig::from_json(&value).is_err());
    }

    /// A field-less tagged variant is exactly its tag: a member beside the tag is
    /// an unknown field, while the bare tag beside it is read.
    #[test]
    fn a_unit_tagged_variant_refuses_an_extra_member() {
        let read = |text: &str| {
            OkfResourceMapping::from_json(&purrdf_lex::json::read(text).expect("JSON"))
        };
        assert_eq!(
            read(r#"{"kind":"subject","mapping":null}"#)
                .expect_err("extra member")
                .to_string(),
            "unknown field `mapping`, expected `kind`"
        );
        assert_eq!(
            read(r#"{"kind":"subject"}"#),
            Ok(OkfResourceMapping::Subject)
        );
        assert!(
            OkfGraphSelection::from_json(
                &purrdf_lex::json::read(r#"{"kind":"all","default_graph":true}"#).expect("JSON")
            )
            .is_err()
        );
        assert_eq!(
            OkfGraphSelection::from_json(
                &purrdf_lex::json::read(r#"{"kind":"all"}"#).expect("JSON")
            ),
            Ok(OkfGraphSelection::All)
        );
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
