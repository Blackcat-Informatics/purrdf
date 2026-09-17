// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The typed refusal vocabulary of the prepared-product codec.
//!
//! [`ProductDimension`] is the closed set of reasons a candidate prepared shapes
//! product can fail admission, and [`ShapesProductError`] is one of those paired
//! with a prescriptive message. See the [module documentation](super) for why the
//! boundary refuses on a named dimension rather than returning one opaque error.

use std::fmt;

// ---------------------------------------------------------------------------
// ProductDimension
// ---------------------------------------------------------------------------

/// One way a candidate prepared shapes product can fail admission.
///
/// The set is deliberately closed and ordered from the outside of the container
/// inward: first the bytes are a product at all ([`Magic`] through [`Trailer`]),
/// then their contents are intact ([`SectionDigest`], [`ContainerDigest`]), then
/// the identity they were prepared against matches the identity supplied for
/// execution ([`DatasetIdentity`] through [`ClassCatalog`]), and only then the
/// residual structural refusals ([`UnsupportedCapability`] through
/// [`Malformed`]). A refusal reports the *first* dimension that fails, so the
/// message a caller sees always describes the outermost unmet precondition
/// rather than a downstream symptom of it.
///
/// Iterate [`ALL`] rather than hand-listing variants, so a new dimension reaches
/// every consumer.
///
/// [`Magic`]: Self::Magic
/// [`Trailer`]: Self::Trailer
/// [`SectionDigest`]: Self::SectionDigest
/// [`ContainerDigest`]: Self::ContainerDigest
/// [`DatasetIdentity`]: Self::DatasetIdentity
/// [`ClassCatalog`]: Self::ClassCatalog
/// [`UnsupportedCapability`]: Self::UnsupportedCapability
/// [`Malformed`]: Self::Malformed
/// [`ALL`]: Self::ALL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProductDimension {
    /// The leading bytes are not the prepared-product magic, so these bytes were
    /// never a prepared shapes product.
    Magic,
    /// The container format version is one this build does not decode. A product
    /// is an artifact of the PurRDF version that wrote it, not a stable
    /// interchange format.
    FormatVersion,
    /// The preparation stage identifier does not match the stage the loader is
    /// restoring into. A product prepared for one stage encodes decisions that
    /// are only valid for that stage.
    StageId,
    /// The preparation profile — the switchable behaviours compiled into the
    /// product — does not match the profile in force for execution.
    Profile,
    /// The byte sequence ends before a structure it declared is complete. A
    /// truncated product is distinguished from a [`Malformed`] one because the
    /// cause is nearly always an interrupted write, not corruption in place.
    ///
    /// [`Malformed`]: Self::Malformed
    Truncated,
    /// The trailing footer is absent or inconsistent with the sections that
    /// precede it, so the container's own directory cannot be trusted.
    Trailer,
    /// A section's recorded digest disagrees with the bytes actually stored for
    /// that section: the product is corrupt in place.
    SectionDigest,
    /// The whole-container digest disagrees with the container's contents. This
    /// is checked in addition to the per-section digests, because the container
    /// header itself is not covered by any section's digest.
    ContainerDigest,
    /// The data-graph identity the product was prepared against is not the
    /// identity supplied for execution.
    DatasetIdentity,
    /// The shapes-graph identity the product was prepared against is not the
    /// identity supplied for execution.
    ShapesGraph,
    /// The prefix map recorded at preparation is not the prefix map supplied for
    /// execution. Prefixes are caller-supplied configuration, so a difference
    /// changes which IRIs the product's compiled terms denote.
    Prefixes,
    /// The base IRI recorded at preparation is not the base IRI supplied for
    /// execution, so relative references resolve differently.
    Base,
    /// The caller-supplied vocabulary configuration recorded at preparation is
    /// not the vocabulary supplied for execution. PurRDF mints no vocabulary
    /// IRIs of its own, so this configuration is load-bearing and never has a
    /// fabricated default.
    Vocabulary,
    /// The SPARQL function registry recorded at preparation is not the registry
    /// supplied for execution, and the registry is what decides how a function
    /// IRI resolves.
    FunctionRegistry,
    /// The custom-aggregate registry recorded at preparation is not the registry
    /// supplied for execution, and the registry is what a custom aggregate IRI
    /// resolves against.
    AggregateRegistry,
    /// The property-function registry recorded at preparation is not the
    /// registry supplied for execution, and the registry is what decides which
    /// predicates are calls rather than ordinary triple patterns.
    PropertyFunctionRegistry,
    /// The class catalog — the asserted `rdfs:subClassOf` closure the product
    /// compiled its class-membership decisions against — does not match the one
    /// supplied for execution.
    ClassCatalog,
    /// The product declares a capability this build does not implement. The
    /// bytes are well formed; this build simply cannot honour what they ask for.
    UnsupportedCapability,
    /// A structure nests deeper than the decoder's fixed depth ceiling. The
    /// ceiling exists so that untrusted bytes cannot drive the decoder into
    /// unbounded recursion.
    DepthLimit,
    /// The bytes are structurally invalid in a way no other dimension names: a
    /// field is out of range, a length disagrees with its payload, or a
    /// reference points outside the container.
    Malformed,
}

impl ProductDimension {
    /// Every admission dimension, in declaration order — the order in which a
    /// decoder checks them. Iterate this rather than hand-listing variants, so a
    /// new dimension reaches every consumer.
    pub const ALL: [Self; 20] = [
        Self::Magic,
        Self::FormatVersion,
        Self::StageId,
        Self::Profile,
        Self::Truncated,
        Self::Trailer,
        Self::SectionDigest,
        Self::ContainerDigest,
        Self::DatasetIdentity,
        Self::ShapesGraph,
        Self::Prefixes,
        Self::Base,
        Self::Vocabulary,
        Self::FunctionRegistry,
        Self::AggregateRegistry,
        Self::PropertyFunctionRegistry,
        Self::ClassCatalog,
        Self::UnsupportedCapability,
        Self::DepthLimit,
        Self::Malformed,
    ];

    /// The number of admission dimensions.
    pub const COUNT: usize = Self::ALL.len();

    /// A stable kebab-case label for this dimension.
    ///
    /// These strings are a pinned contract: a frozen conformance corpus records
    /// them as refusal discriminants, so a consumer may match on them. Renaming
    /// one is a breaking change to that corpus, not a cosmetic edit. They are
    /// deliberately distinct from the prose a [`ShapesProductError`] renders.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Magic => "magic",
            Self::FormatVersion => "format-version",
            Self::StageId => "stage-id",
            Self::Profile => "profile",
            Self::Truncated => "truncated",
            Self::Trailer => "trailer",
            Self::SectionDigest => "section-digest",
            Self::ContainerDigest => "container-digest",
            Self::DatasetIdentity => "dataset-identity",
            Self::ShapesGraph => "shapes-graph",
            Self::Prefixes => "prefixes",
            Self::Base => "base",
            Self::Vocabulary => "vocabulary",
            Self::FunctionRegistry => "function-registry",
            Self::AggregateRegistry => "aggregate-registry",
            Self::PropertyFunctionRegistry => "property-function-registry",
            Self::ClassCatalog => "class-catalog",
            Self::UnsupportedCapability => "unsupported-capability",
            Self::DepthLimit => "depth-limit",
            Self::Malformed => "malformed",
        }
    }
}

impl fmt::Display for ProductDimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ---------------------------------------------------------------------------
// ShapesProductError
// ---------------------------------------------------------------------------

/// A refusal from the prepared-product admission boundary.
///
/// It carries the [`ProductDimension`] that failed and a message written in the
/// prescriptive register this repository uses for refusals: the message names
/// the action that resolves the refusal, not only the condition that caused it.
/// "this product was prepared against a different prefix map than the one
/// supplied for its execution; prepare it under the SAME prefixes the execution
/// uses" tells a caller what to do; "prefix mismatch" leaves them to guess
/// whether to rebuild the artifact or fix their configuration.
///
/// The dimension is what a caller branches on and the message is what a human
/// reads. Matching on the message text is not supported — that is what
/// [`ShapesProductError::dimension`] is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapesProductError {
    /// The admission dimension that refused.
    dimension: ProductDimension,
    /// The prescriptive explanation, naming the fix.
    message: String,
}

impl ShapesProductError {
    /// Refuse on `dimension`, with a message that names the fix.
    ///
    /// `message` is prose for a human: state what was expected, what was found,
    /// and the action that resolves it. Do not repeat the dimension label in it
    /// — [`fmt::Display`] already prefixes the label.
    #[must_use]
    pub fn new(dimension: ProductDimension, message: impl Into<String>) -> Self {
        Self {
            dimension,
            message: message.into(),
        }
    }

    /// The admission dimension that refused. This is the stable, matchable part
    /// of the refusal.
    #[must_use]
    pub const fn dimension(&self) -> ProductDimension {
        self.dimension
    }

    /// The prescriptive explanation, without the dimension label.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ShapesProductError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.dimension.label(), self.message)
    }
}

impl std::error::Error for ShapesProductError {}

#[cfg(test)]
mod tests {
    use super::{ProductDimension, ShapesProductError};

    #[test]
    fn dimension_labels_are_unique_and_kebab_case() {
        let mut labels: Vec<&'static str> =
            ProductDimension::ALL.iter().map(|d| d.label()).collect();
        assert_eq!(labels.len(), ProductDimension::COUNT);

        for label in &labels {
            assert!(!label.is_empty(), "empty label");
            assert!(
                label.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'),
                "label {label:?} is not lowercase ASCII plus hyphen",
            );
            assert!(
                !label.starts_with('-') && !label.ends_with('-'),
                "label {label:?} has a leading or trailing hyphen",
            );
            assert!(
                !label.contains("--"),
                "label {label:?} has an empty kebab segment",
            );
        }

        labels.sort_unstable();
        let duplicated = labels.len();
        labels.dedup();
        assert_eq!(
            labels.len(),
            duplicated,
            "labels are a pinned contract and must be distinct",
        );
    }

    #[test]
    fn dimension_all_covers_every_variant() {
        // This match is deliberately WILDCARD-FREE: adding a variant to
        // `ProductDimension` without adding it to `ALL` fails to compile here,
        // which is the point of the test.
        for dimension in ProductDimension::ALL {
            let declared_index: usize = match dimension {
                ProductDimension::Magic => 0,
                ProductDimension::FormatVersion => 1,
                ProductDimension::StageId => 2,
                ProductDimension::Profile => 3,
                ProductDimension::Truncated => 4,
                ProductDimension::Trailer => 5,
                ProductDimension::SectionDigest => 6,
                ProductDimension::ContainerDigest => 7,
                ProductDimension::DatasetIdentity => 8,
                ProductDimension::ShapesGraph => 9,
                ProductDimension::Prefixes => 10,
                ProductDimension::Base => 11,
                ProductDimension::Vocabulary => 12,
                ProductDimension::FunctionRegistry => 13,
                ProductDimension::AggregateRegistry => 14,
                ProductDimension::PropertyFunctionRegistry => 15,
                ProductDimension::ClassCatalog => 16,
                ProductDimension::UnsupportedCapability => 17,
                ProductDimension::DepthLimit => 18,
                ProductDimension::Malformed => 19,
            };
            assert_eq!(
                ProductDimension::ALL[declared_index],
                dimension,
                "{dimension} does not occupy its declared slot in ALL",
            );
        }

        assert_eq!(ProductDimension::ALL.len(), ProductDimension::COUNT);
    }

    #[test]
    fn error_display_names_the_dimension() {
        let error = ShapesProductError::new(
            ProductDimension::Prefixes,
            "this product was prepared against a different prefix map than the one supplied \
             for its execution; prepare it under the SAME prefixes the execution uses, because \
             the prefix map is what decides which IRI `ex:Shape` denotes in \
             `https://example.org/shapes`",
        );

        assert_eq!(error.dimension(), ProductDimension::Prefixes);

        let rendered = error.to_string();
        assert!(
            rendered.starts_with("prefixes: "),
            "display must lead with the dimension label, got {rendered:?}",
        );
        assert!(
            rendered.contains(error.message()),
            "display must carry the prescriptive message, got {rendered:?}",
        );
        assert!(
            rendered.contains("prepare it under the SAME prefixes"),
            "the message must name the fix, got {rendered:?}",
        );

        // A neighbouring dimension renders its own label, not this one's: the
        // discriminant a caller matches on is per-variant, not shared prose.
        let other = ShapesProductError::new(
            ProductDimension::Base,
            "this product was prepared against a different base IRI than the one supplied for \
             its execution; prepare it under the SAME base the execution uses, because the base \
             is what relative references resolve against",
        );
        assert_ne!(other.dimension(), error.dimension());
        assert!(other.to_string().starts_with("base: "));
    }
}
