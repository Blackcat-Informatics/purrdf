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

/// The caller's slice-framework vocabulary: a namespace all term IRIs are
/// derived from by concatenation (`{ns}{localName}`), plus the CURIE prefix
/// name used when emitting prefixed names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceVocab {
    ns: String,
    prefix_name: String,
}

impl SliceVocab {
    /// Construct a vocabulary rooted at `ns` (every term is `{ns}{localName}`).
    ///
    /// The CURIE prefix name defaults to the last non-empty path segment of the
    /// namespace (e.g. `https://example.org/gm/` → `gm`); override it with
    /// [`SliceVocab::with_prefix_name`].
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
            prefix_name,
        }
    }

    /// Override the CURIE prefix name used for prefixed-name emission.
    #[must_use]
    pub fn with_prefix_name(mut self, prefix_name: &str) -> Self {
        prefix_name.clone_into(&mut self.prefix_name);
        self
    }

    /// The vocabulary namespace (as given, trailing separator preserved).
    #[must_use]
    pub fn ns(&self) -> &str {
        &self.ns
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
    fn hash_namespaces_work() {
        let v = SliceVocab::for_namespace("https://example.org/onto#");
        assert_eq!(v.term("Slice"), "https://example.org/onto#Slice");
        assert_eq!(v.ontology_iri(), "https://example.org/onto");
        assert_eq!(v.prefix_name(), "onto");
    }
}
