// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One consumer-config shape for every namespace-bound emitter.
//!
//! PurRDF parameterizes three otherwise-unrelated emitters on the consumer's
//! vocabulary, each with its own native config type reached through a different
//! sub-crate:
//!
//! * the slice catalog / analysis emitters take a [`purrdf_slice::SliceVocab`];
//! * the SHACL → JSON Schema emitter takes a
//!   [`purrdf_shapes::json_schema::Namespaces`];
//! * the JSON-LD-star statement-metadata downcast takes a
//!   [`purrdf_rdf::native_codecs::jsonld::StatementMetadataVocab`].
//!
//! A consumer with a single ontology namespace would otherwise construct "its
//! namespace" three different ways. [`OntologyProfile`] is the one shape it
//! builds once; the `slice_vocab` / `namespaces` / `statement_metadata_vocab`
//! accessors project it into whichever native config a given emitter accepts.
//!
//! Like the sub-crate config types it wraps, this mints no vocabulary of its
//! own and has NO `Default`: the *namespace* is always caller-supplied. The
//! `for_namespace` constructors only concatenate caller-supplied namespaces
//! with fixed local names — exactly the [`purrdf_slice::SliceVocab`] pattern —
//! so no fabricated namespace can leak into output.
//!
//! ```
//! use purrdf::OntologyProfile;
//!
//! let profile = OntologyProfile::for_namespace("https://example.org/vocab/");
//! assert_eq!(profile.prefix, "vocab");
//! // Project into each emitter's native config from the one profile:
//! assert_eq!(
//!     profile.slice_vocab().slice_class(),
//!     "https://example.org/vocab/Slice"
//! );
//! let ns = profile.namespaces().expect("primary prefix resolves");
//! let smv = profile.statement_metadata_vocab();
//! assert_eq!(smv.q_subject, "https://example.org/vocab/qSubject");
//! # let _ = ns;
//! ```

use purrdf_rdf::native_codecs::jsonld::StatementMetadataVocab;
use purrdf_shapes::json_schema::Namespaces;
use purrdf_slice::SliceVocab;

/// The caller's RDF-1.2 statement-metadata reification vocabulary, in OWNED
/// form (the borrowed [`StatementMetadataVocab`] the codec accepts keeps its
/// IRIs by reference, so a profile that outlives a single call needs owned
/// strings to lend from).
///
/// The five members mirror [`StatementMetadataVocab`] field-for-field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReifierVocab {
    /// The reifier's `rdf:type` (the statement-metadata class IRI).
    pub statement_metadata: String,
    /// The quoted-subject predicate IRI.
    pub q_subject: String,
    /// The quoted-predicate predicate IRI.
    pub q_predicate: String,
    /// The quoted-object predicate IRI (IRI / blank-node objects).
    pub q_object: String,
    /// The quoted-object predicate IRI for literal objects.
    pub q_object_literal: String,
}

impl ReifierVocab {
    /// Derive the five reifier IRIs from `ns` by concatenation with fixed local
    /// names (`StatementMetadata`, `qSubject`, `qPredicate`, `qObject`,
    /// `qObjectLiteral`) — the naming used in the [`StatementMetadataVocab`]
    /// documentation. `ns` is caller-supplied; nothing is fabricated.
    #[must_use]
    pub fn for_namespace(ns: &str) -> Self {
        Self {
            statement_metadata: format!("{ns}StatementMetadata"),
            q_subject: format!("{ns}qSubject"),
            q_predicate: format!("{ns}qPredicate"),
            q_object: format!("{ns}qObject"),
            q_object_literal: format!("{ns}qObjectLiteral"),
        }
    }

    /// Borrow this owned vocabulary as the [`StatementMetadataVocab`] the
    /// JSON-LD-star downcast accepts. The returned view borrows from `self`.
    #[must_use]
    pub fn as_statement_metadata_vocab(&self) -> StatementMetadataVocab<'_> {
        StatementMetadataVocab {
            statement_metadata: &self.statement_metadata,
            q_subject: &self.q_subject,
            q_predicate: &self.q_predicate,
            q_object: &self.q_object,
            q_object_literal: &self.q_object_literal,
        }
    }
}

/// The one consumer-config shape a downstream builds once and projects into
/// every namespace-bound emitter's native config.
///
/// See the [module docs][self] for the rationale and an example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyProfile {
    /// The primary vocabulary namespace (trailing `/`/`#` separator preserved).
    pub namespace: String,
    /// The CURIE prefix name for the primary namespace.
    pub prefix: String,
    /// Additional `(prefix, namespace)` declarations available for compaction
    /// (e.g. the shapes document's own `@prefix` lines). The W3C builtins
    /// (`xsd`/`rdf`/`rdfs`/`owl`/`sh`) are always available on top of these.
    pub prefixes: Vec<(String, String)>,
    /// The statement-metadata reification vocabulary for the JSON-LD-star
    /// downcast.
    pub reifier: ReifierVocab,
}

impl OntologyProfile {
    /// Build a profile rooted at `namespace`.
    ///
    /// The CURIE prefix is derived the same way [`SliceVocab::for_namespace`]
    /// derives its own (the last non-empty path/fragment segment), and the
    /// reifier IRIs are derived by [`ReifierVocab::for_namespace`]. Override
    /// either with the builder methods.
    #[must_use]
    pub fn for_namespace(namespace: &str) -> Self {
        // Reuse the slice crate's prefix-derivation so the two agree exactly.
        let prefix = SliceVocab::for_namespace(namespace)
            .prefix_name()
            .to_owned();
        Self {
            namespace: namespace.to_owned(),
            prefix,
            prefixes: Vec::new(),
            reifier: ReifierVocab::for_namespace(namespace),
        }
    }

    /// Override the primary CURIE prefix name.
    #[must_use]
    pub fn with_prefix(mut self, prefix: &str) -> Self {
        prefix.clone_into(&mut self.prefix);
        self
    }

    /// Set the additional `(prefix, namespace)` declarations.
    #[must_use]
    pub fn with_prefixes(mut self, prefixes: Vec<(String, String)>) -> Self {
        self.prefixes = prefixes;
        self
    }

    /// Override the statement-metadata reification vocabulary.
    #[must_use]
    pub fn with_reifier(mut self, reifier: ReifierVocab) -> Self {
        self.reifier = reifier;
        self
    }

    /// Project into the [`SliceVocab`] the slice catalog / analysis emitters
    /// accept.
    #[must_use]
    pub fn slice_vocab(&self) -> SliceVocab {
        SliceVocab::for_namespace(&self.namespace).with_prefix_name(&self.prefix)
    }

    /// Project into the [`Namespaces`] table the SHACL → JSON Schema emitter
    /// accepts. The primary `(prefix, namespace)` is prepended to
    /// [`self.prefixes`](Self::prefixes) so it is always declared.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the primary prefix resolves in neither the assembled
    /// declarations nor the W3C builtins (see [`Namespaces::new`]) — which
    /// cannot happen for a profile built through these constructors, but the
    /// fallibility is preserved from the underlying API.
    pub fn namespaces(&self) -> Result<Namespaces, String> {
        let mut declarations = Vec::with_capacity(self.prefixes.len() + 1);
        declarations.push((self.prefix.clone(), self.namespace.clone()));
        declarations.extend(self.prefixes.iter().cloned());
        Namespaces::new(&self.prefix, &declarations)
    }

    /// Project into the [`StatementMetadataVocab`] the JSON-LD-star downcast
    /// accepts. The returned view borrows from `self.reifier`.
    #[must_use]
    pub fn statement_metadata_vocab(&self) -> StatementMetadataVocab<'_> {
        self.reifier.as_statement_metadata_vocab()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_prefix_and_reifier_from_namespace() {
        let p = OntologyProfile::for_namespace("https://example.org/vocab/");
        assert_eq!(p.namespace, "https://example.org/vocab/");
        assert_eq!(p.prefix, "vocab");
        assert_eq!(
            p.reifier.statement_metadata,
            "https://example.org/vocab/StatementMetadata"
        );
        assert_eq!(
            p.reifier.q_object_literal,
            "https://example.org/vocab/qObjectLiteral"
        );
    }

    #[test]
    fn projects_into_every_native_config() {
        let p = OntologyProfile::for_namespace("https://example.org/vocab/").with_prefix("ex");

        // slice
        let sv = p.slice_vocab();
        assert_eq!(sv.slice_class(), "https://example.org/vocab/Slice");
        assert_eq!(sv.prefix_name(), "ex");

        // shapes json-schema
        let ns = p.namespaces().expect("primary prefix resolves");
        assert_eq!(ns.compact_iri("https://example.org/vocab/Cat"), "ex:Cat");

        // jsonld-star statement metadata (borrows from the profile)
        let smv = p.statement_metadata_vocab();
        assert_eq!(smv.q_predicate, "https://example.org/vocab/qPredicate");
    }

    #[test]
    fn extra_prefixes_are_carried_into_the_namespace_table() {
        let p = OntologyProfile::for_namespace("https://example.org/vocab/").with_prefixes(vec![(
            "friend".to_owned(),
            "https://example.org/friend/".to_owned(),
        )]);
        let ns = p.namespaces().expect("primary prefix resolves");
        assert_eq!(
            ns.compact_iri("https://example.org/friend/Dog"),
            "friend:Dog"
        );
    }
}
