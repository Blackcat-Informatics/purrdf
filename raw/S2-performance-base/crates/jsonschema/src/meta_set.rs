// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A caller-supplied, shared set of meta-schema documents.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use purrdf_lex::json::Value;

use crate::compile;
use crate::dialect::Dialect;
use crate::error::SchemaError;
use crate::registry::Registry;
use crate::schema::Schema;

/// An immutable set of meta-schema documents, built once and shared.
///
/// This crate carries no meta-schema: the documents published for each
/// dialect (for 2020-12, `https://json-schema.org/draft/2020-12/schema` and
/// its vocabulary meta-schemas; for 2019-09 likewise; for draft-07 the one
/// `http://json-schema.org/draft-07/schema`) are the caller's to supply, as
/// is any custom meta-schema. [`Metaschemas::new`] registers them, checks
/// each against its own meta-schema, and compiles the meta-validators its
/// members use; [`crate::Registry::with_metaschemas`] and
/// [`crate::Schema::from_document`] then start from it without parsing or
/// compiling any of it again.
///
/// Cloning is an `Arc` clone. A validator for a member no member names as its
/// `$schema` is compiled on first use and cached here, for every registry
/// holding the set. Meta-validators read `format` as an annotation.
///
/// ```
/// use purrdf_jsonschema::{Metaschemas, Schema, SchemaError};
/// use purrdf_lex::json;
///
/// // A deliberately small custom dialect built on draft-07, which is
/// // self-describing and so needs nothing else.
/// let meta = json::read(
///     r#"{
///         "$schema": "http://json-schema.org/draft-07/schema#",
///         "$id": "https://example.org/meta",
///         "properties": {"type": {"enum": ["string", "number"]}}
///     }"#,
/// )
/// .expect("JSON");
/// // Without the draft-07 meta-schema itself the set is incomplete:
/// let missing = Metaschemas::new([("https://example.org/meta", meta)]);
/// assert!(matches!(
///     missing,
///     Err(SchemaError::MissingMetaschema { metaschema, .. })
///         if metaschema == "http://json-schema.org/draft-07/schema"
/// ));
/// # Ok::<(), SchemaError>(())
/// ```
#[derive(Clone)]
pub struct Metaschemas {
    inner: Arc<Inner>,
}

struct Inner {
    registry: Registry,
    validators: Mutex<BTreeMap<String, Arc<Schema>>>,
}

impl fmt::Debug for Metaschemas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Metaschemas")
            .field("registry", &self.inner.registry)
            .finish_non_exhaustive()
    }
}

impl Metaschemas {
    /// Register `documents` — `(retrieval URI, document)` pairs, in any
    /// order — as one meta-schema set.
    ///
    /// A document without `$schema` is read as draft 2020-12. Every member
    /// is checked against the meta-schema its `$schema` names, which must be
    /// a member too: [`SchemaError::MissingMetaschema`] names the first one
    /// that is not. A member invalid against its meta-schema is
    /// [`SchemaError::InvalidSchema`]; a reference out of the set is
    /// [`SchemaError::UnresolvedReference`].
    pub fn new<I, U>(documents: I) -> Result<Self, SchemaError>
    where
        I: IntoIterator<Item = (U, Value)>,
        U: AsRef<str>,
    {
        let mut pending: Vec<(String, Value)> = documents
            .into_iter()
            .map(|(uri, document)| (uri.as_ref().to_owned(), document))
            .collect();
        let mut registry = Registry::new();
        // A member whose `$schema` is another (custom) member is registered
        // after it, whatever order the caller listed them in.
        while !pending.is_empty() {
            let ready = pending.iter().position(|(_, document)| {
                document
                    .get("$schema")
                    .and_then(Value::as_str)
                    .is_none_or(|schema| {
                        let bare = schema.strip_suffix('#').unwrap_or(schema);
                        Dialect::from_uri(bare).is_some()
                            || registry.resource_by_uri(bare).is_some()
                            || !pending.iter().any(|(uri, _)| uri == bare)
                    })
            });
            // With no member ready, the first one reports its own error.
            let (uri, document) = pending.remove(ready.unwrap_or(0));
            registry.insert(&uri, document, true, Dialect::Draft2020_12)?;
        }
        let inner = Inner {
            registry,
            validators: Mutex::new(BTreeMap::new()),
        };
        let set = Self {
            inner: Arc::new(inner),
        };
        for doc in &set.inner.registry.docs {
            let root = &set.inner.registry.resources[doc.root_resource];
            if root.dialect.is_err() {
                return Err(SchemaError::UnsupportedDialect {
                    resource: doc.uri.clone(),
                    dialect: root.metaschema.clone(),
                });
            }
            let validator = set.validator(&root.metaschema, &doc.uri)?;
            compile::check(&validator, doc)?;
        }
        Ok(set)
    }

    /// Whether a member resource has the URI `uri` (without fragment).
    pub fn contains(&self, uri: &str) -> bool {
        self.inner.registry.resource_by_uri(uri).is_some()
    }

    pub(crate) fn registry(&self) -> &Registry {
        &self.inner.registry
    }

    /// The compiled meta-schema `uri`, a member of this set, compiled on
    /// first use; `resource` is who needs it, for the error.
    pub(crate) fn validator(&self, uri: &str, resource: &str) -> Result<Arc<Schema>, SchemaError> {
        let cached = self
            .inner
            .validators
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(uri)
            .cloned();
        if let Some(validator) = cached {
            return Ok(validator);
        }
        if self.inner.registry.resource_by_uri(uri).is_none() {
            return Err(SchemaError::MissingMetaschema {
                metaschema: uri.to_owned(),
                resource: resource.to_owned(),
            });
        }
        // Compiled outside the lock: every member is already checked or is
        // checked by `new`, so this compiles without meta-validation.
        let validator = Arc::new(compile::compile_unchecked(&self.inner.registry, uri, false)?.0);
        Ok(Arc::clone(
            self.inner
                .validators
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(uri.to_owned())
                .or_insert(validator),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft_2020_12() -> Metaschemas {
        Metaschemas::new(
            purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12
                .iter()
                .map(|&(uri, text)| (uri, purrdf_lex::json::read(text).expect("JSON"))),
        )
        .expect("the draft 2020-12 meta-schemas")
    }

    #[test]
    fn every_registry_started_from_a_set_shares_its_documents_and_validators() {
        let set = draft_2020_12();
        let uri = Dialect::Draft2020_12.uri();
        let first = Registry::with_metaschemas(&set);
        let second = Registry::with_metaschemas(&set);
        let clone = second.clone();
        for ((mine, theirs), cloned) in first.docs.iter().zip(&second.docs).zip(&clone.docs) {
            assert!(Arc::ptr_eq(mine, theirs), "{} was copied", mine.uri);
            assert!(Arc::ptr_eq(theirs, cloned), "{} was copied", mine.uri);
        }
        let from_first = first
            .metaschemas
            .as_ref()
            .expect("the set")
            .validator(uri, "first")
            .expect("validator");
        let from_second = second
            .metaschemas
            .as_ref()
            .expect("the set")
            .validator(uri, "second")
            .expect("validator");
        assert!(Arc::ptr_eq(&from_first, &from_second), "compiled twice");
        // A set built separately compiles its own.
        let other = draft_2020_12().validator(uri, "other").expect("validator");
        assert!(!Arc::ptr_eq(&from_first, &other));
    }

    #[test]
    fn a_validator_for_a_uri_outside_the_set_is_a_missing_metaschema() {
        let set = draft_2020_12();
        assert!(matches!(
            set.validator(Dialect::Draft07.uri(), "https://example.org/s.json"),
            Err(SchemaError::MissingMetaschema { metaschema, .. })
                if metaschema == Dialect::Draft07.uri()
        ));
        assert!(set.validator(Dialect::Draft2020_12.uri(), "x").is_ok());
    }
}
