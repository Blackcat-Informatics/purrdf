// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The consumer slice vocabulary.
//!
//! Every ontology term the slice framework reads or emits (`Slice`,
//! `sliceTier`, `sliceDependsOn`, the analysis-graph terms, the mapping-DSL
//! classes, …) belongs to the CALLING application's vocabulary — PurRDF mints
//! no such terms (its published carrier vocabulary in `vocab/purrdf.ttl` is
//! deliberately tiny and carries none of these). A [`SliceVocab`] is therefore
//! caller-constructed and threaded through every public entry point of the
//! catalog, ownership analyzer, dependency patcher, and emitters. There is NO
//! `Default` implementation: a fabricated namespace must never leak into
//! output, so callers state their namespace explicitly, e.g.
//!
//! ```
//! use purrdf_slice::SliceVocab;
//! let vocab = SliceVocab::for_namespace("https://example.org/vocab/");
//! assert_eq!(vocab.slice_class(), "https://example.org/vocab/Slice");
//! assert_eq!(vocab.prefix_name(), "vocab");
//! assert_eq!(vocab.ontology_iri(), "https://example.org/vocab");
//! ```
//!
//! ## Framework namespace vs. owned term namespaces
//!
//! Two distinct things are easy to conflate, and conflating them silently
//! disables ownership analysis for part of a corpus:
//!
//! * The **framework namespace** ([`SliceVocab::ns`]) mints the slice-framework
//!   terms themselves — `Slice`, `sliceTier`, `sliceDependsOn`, the
//!   analysis-graph terms. There is exactly one.
//! * The **owned term namespaces** ([`SliceVocab::term_namespaces`]) are the
//!   namespaces the caller's slices mint ONTOLOGY terms into — the IRIs that
//!   carry `rdfs:isDefinedBy` and that cross-slice references resolve against.
//!   A corpus may mint into several, and a slice may mint into a namespace no
//!   other slice uses.
//!
//! The framework namespace is always an owned term namespace, so a single-
//! namespace caller needs no extra configuration. A caller whose slices mint
//! elsewhere MUST declare those namespaces with
//! [`SliceVocab::with_term_namespaces`] — a term in an undeclared namespace is
//! invisible to the ownership analyzer, so every reference to it resolves to no
//! owner and contributes no dependency edge.
//!
//! ```
//! use purrdf_slice::SliceVocab;
//! let vocab = SliceVocab::for_namespace("https://example.org/vocab/")
//!     .with_term_namespaces(["https://example.org/math/"]);
//! assert!(vocab.owns_term("https://example.org/vocab/Thing"));
//! assert!(vocab.owns_term("https://example.org/math/Quantity"));
//! assert!(!vocab.owns_term("http://www.w3.org/2002/07/owl#Class"));
//! ```

use std::collections::BTreeSet;

/// The caller's slice-framework vocabulary: a namespace all framework term IRIs
/// are derived from by concatenation (`{ns}{localName}`), the set of namespaces
/// the caller's slices mint ontology terms into, plus the CURIE prefix name used
/// when emitting prefixed names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceVocab {
    ns: String,
    term_namespaces: BTreeSet<String>,
    prefix_name: String,
}

impl SliceVocab {
    /// Construct a vocabulary rooted at `ns` (every framework term is
    /// `{ns}{localName}`), owning terms in `ns` and no other namespace.
    ///
    /// The CURIE prefix name defaults to the last non-empty path segment of the
    /// namespace (e.g. `https://example.org/gm/` → `gm`); override it with
    /// [`SliceVocab::with_prefix_name`]. Declare additional term namespaces with
    /// [`SliceVocab::with_term_namespaces`].
    #[must_use]
    pub fn for_namespace(ns: &str) -> Self {
        let trimmed = ns.trim_end_matches(['/', '#']);
        let prefix_name = trimmed
            .rsplit(['/', '#'])
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("ns")
            .to_owned();
        Self {
            ns: ns.to_owned(),
            term_namespaces: BTreeSet::from([ns.to_owned()]),
            prefix_name,
        }
    }

    /// Declare additional namespaces the caller's slices mint ontology terms
    /// into. The framework namespace is always owned and never removed.
    ///
    /// Ownership is tested against the TERM IRI itself, so a slice minting
    /// exclusively into an undeclared namespace contributes no ownership data at
    /// all and every reference to its terms resolves to no owner.
    #[must_use]
    pub fn with_term_namespaces<I, S>(mut self, namespaces: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.term_namespaces
            .extend(namespaces.into_iter().map(|n| n.as_ref().to_owned()));
        self
    }

    /// Override the CURIE prefix name used for prefixed-name emission.
    #[must_use]
    pub fn with_prefix_name(mut self, prefix_name: &str) -> Self {
        prefix_name.clone_into(&mut self.prefix_name);
        self
    }

    /// The framework vocabulary namespace (as given, trailing separator
    /// preserved). This mints the slice-framework terms; it is NOT the test for
    /// whether an arbitrary term IRI is owned — see [`SliceVocab::owns_term`].
    #[must_use]
    pub fn ns(&self) -> &str {
        &self.ns
    }

    /// Every namespace the caller's slices mint ontology terms into, including
    /// the framework namespace. Sorted and deduplicated.
    #[must_use]
    pub fn term_namespaces(&self) -> &BTreeSet<String> {
        &self.term_namespaces
    }

    /// Whether `iri` lies in one of the owned term namespaces — the single test
    /// deciding whether a subject can carry ownership and whether a reference
    /// can resolve to an owning slice.
    #[must_use]
    pub fn owns_term(&self, iri: &str) -> bool {
        self.term_namespaces.iter().any(|ns| iri.starts_with(ns))
    }

    /// The CURIE prefix name for emitted prefixed names (`{prefix}:{local}`).
    #[must_use]
    pub fn prefix_name(&self) -> &str {
        &self.prefix_name
    }

    /// The ontology IRI: the namespace without its trailing `/`/`#` separator.
    #[must_use]
    pub fn ontology_iri(&self) -> &str {
        self.ns.trim_end_matches(['/', '#'])
    }

    /// A full term IRI: `{ns}{local}`.
    #[must_use]
    pub fn term(&self, local: &str) -> String {
        format!("{}{local}", self.ns)
    }

    // ── Catalog / manifest terms ─────────────────────────────────────────────

    /// The slice class (`{ns}Slice`): the manifest's `a <…>Slice` subject type.
    #[must_use]
    pub fn slice_class(&self) -> String {
        self.term("Slice")
    }

    /// `{ns}sliceTier`.
    #[must_use]
    pub fn slice_tier(&self) -> String {
        self.term("sliceTier")
    }

    /// `{ns}sliceConsumer`.
    #[must_use]
    pub fn slice_consumer(&self) -> String {
        self.term("sliceConsumer")
    }

    /// `{ns}sliceProfile`.
    #[must_use]
    pub fn slice_profile(&self) -> String {
        self.term("sliceProfile")
    }

    /// `{ns}sliceDependsOn`.
    #[must_use]
    pub fn slice_depends_on(&self) -> String {
        self.term("sliceDependsOn")
    }

    // ── Analysis-graph terms ─────────────────────────────────────────────────

    /// The named graph IRI for the computed slice-analysis output
    /// (`{ns}graph/slice-analysis`).
    #[must_use]
    pub fn analysis_graph_iri(&self) -> String {
        self.term("graph/slice-analysis")
    }

    /// `{ns}computedSliceDependency` — the computed-edge class.
    #[must_use]
    pub fn computed_slice_dependency(&self) -> String {
        self.term("computedSliceDependency")
    }

    /// `{ns}dependencyStatus` — edge status literal predicate.
    #[must_use]
    pub fn dependency_status(&self) -> String {
        self.term("dependencyStatus")
    }

    /// `{ns}dependencyEvidence` — edge evidence-summary predicate.
    #[must_use]
    pub fn dependency_evidence(&self) -> String {
        self.term("dependencyEvidence")
    }

    /// `{ns}computedProfileMembership` — profile membership assertion.
    #[must_use]
    pub fn computed_profile_membership(&self) -> String {
        self.term("computedProfileMembership")
    }

    /// `{ns}termCoverage` — per-slice owned-term count predicate.
    #[must_use]
    pub fn term_coverage(&self) -> String {
        self.term("termCoverage")
    }

    // ── Mapping-DSL classes ──────────────────────────────────────────────────

    /// `{ns}TermEquivalence`.
    #[must_use]
    pub fn term_equivalence(&self) -> String {
        self.term("TermEquivalence")
    }

    /// `{ns}ProjectionFunction`.
    #[must_use]
    pub fn projection_function(&self) -> String {
        self.term("ProjectionFunction")
    }

    /// `{ns}MappingSet`.
    #[must_use]
    pub fn mapping_set(&self) -> String {
        self.term("MappingSet")
    }

    /// `{ns}ProjectionMapping`.
    #[must_use]
    pub fn projection_mapping(&self) -> String {
        self.term("ProjectionMapping")
    }

    /// `{ns}sssomFile`.
    #[must_use]
    pub fn sssom_file(&self) -> String {
        self.term("sssomFile")
    }

    // ── Prefix-set projection ────────────────────────────────────────────────

    /// The importable named prefix set (`{ns}CorePrefixes`).
    #[must_use]
    pub fn core_prefixes_iri(&self) -> String {
        self.term("CorePrefixes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_terms_by_concatenation() {
        let v = SliceVocab::for_namespace("https://example.org/vocab/");
        assert_eq!(v.slice_class(), "https://example.org/vocab/Slice");
        assert_eq!(
            v.slice_depends_on(),
            "https://example.org/vocab/sliceDependsOn"
        );
        assert_eq!(
            v.analysis_graph_iri(),
            "https://example.org/vocab/graph/slice-analysis"
        );
        assert_eq!(v.ontology_iri(), "https://example.org/vocab");
        assert_eq!(v.prefix_name(), "vocab");
    }

    #[test]
    fn prefix_name_is_overridable() {
        let v = SliceVocab::for_namespace("https://example.org/vocab/").with_prefix_name("ex");
        assert_eq!(v.prefix_name(), "ex");
        assert_eq!(v.ns(), "https://example.org/vocab/");
    }

    #[test]
    fn framework_namespace_is_always_an_owned_term_namespace() {
        let v = SliceVocab::for_namespace("https://example.org/vocab/");
        assert_eq!(
            v.term_namespaces(),
            &BTreeSet::from(["https://example.org/vocab/".to_owned()])
        );
        assert!(v.owns_term("https://example.org/vocab/Thing"));
        assert!(!v.owns_term("https://example.org/math/Quantity"));
    }

    #[test]
    fn additional_term_namespaces_are_owned_and_never_displace_the_framework_ns() {
        let v = SliceVocab::for_namespace("https://example.org/vocab/")
            .with_term_namespaces(["https://example.org/math/", "https://example.org/lang/"]);
        assert!(v.owns_term("https://example.org/vocab/Slice"));
        assert!(v.owns_term("https://example.org/math/Quantity"));
        assert!(v.owns_term("https://example.org/lang/Rendering"));
        assert!(!v.owns_term("http://www.w3.org/2002/07/owl#Class"));
        // The framework namespace still mints the framework terms.
        assert_eq!(v.ns(), "https://example.org/vocab/");
        assert_eq!(v.slice_class(), "https://example.org/vocab/Slice");
        assert_eq!(v.term_namespaces().len(), 3);
    }

    #[test]
    fn hash_namespaces_work() {
        let v = SliceVocab::for_namespace("https://example.org/onto#");
        assert_eq!(v.term("Slice"), "https://example.org/onto#Slice");
        assert_eq!(v.ontology_iri(), "https://example.org/onto");
        assert_eq!(v.prefix_name(), "onto");
    }
}
