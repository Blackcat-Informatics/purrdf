// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What the JSON-LD research-object profiles (Croissant, DCAT, RO-Crate)
//! share: the node and value-object shapes they write, the member and value
//! readers they read with, and the relative data-path law.
//!
//! The three profiles differ in their vocabularies, their graph shapes and how
//! they resolve references; the JSON-LD value grammar underneath — a node
//! reference is `{"@id": …}`, a literal is a value object carrying `@value`
//! and at most one of `@language` or `@type`, plus `@direction` (JSON-LD 1.1
//! §4.2.4, §4.2.4.2) — is one grammar, written and read here once.

use std::collections::BTreeMap;
use std::fmt;

use purrdf_core::LossLedger;
use purrdf_core::loss::{
    LOSS_RESEARCH_ORDER_DROPPED, LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
    LOSS_RESEARCH_UNSUPPORTED_VALUE_DROPPED,
};
use purrdf_lex::json::record::{Owned, into_owned};
use purrdf_lex::json::{Object, Value};

use super::super::{ProjectionDirection, ProjectionError, validate_absolute_iri};
use super::json::{OfflineJsonLdContext, json_pointer, record_loss};
use super::{ResearchObjectConfig, ResearchObjectRoles, ResearchRole, ResearchText, ResearchValue};

/// A JSON-LD profile's compact-term vocabulary, as its configuration sees it.
///
/// Sealed: the profiles are a closed family, each vocabulary validated by its
/// own constructor.
pub trait JsonLdProfileVocabulary: sealed::Sealed {
    /// The profile's closed role set.
    type Role: fmt::Debug;

    /// The profile's name in a refusal (`"DCAT"`).
    const PROFILE: &'static str;

    /// The compact term bound to each role.
    fn terms(&self) -> &BTreeMap<Self::Role, String>;
}

pub(super) mod sealed {
    /// The closed set of [`super::JsonLdProfileVocabulary`] implementors.
    pub trait Sealed {}
}

/// The mandatory configuration of a JSON-LD research-object profile whose
/// documents are exactly its caller-owned context, vocabulary and profile
/// identity over the shared research-object configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonLdProfileConfig<V> {
    pub(super) common: ResearchObjectConfig,
    pub(super) context: OfflineJsonLdContext,
    pub(super) vocabulary: V,
    pub(super) profile_iri: String,
}

impl<V: JsonLdProfileVocabulary> JsonLdProfileConfig<V> {
    /// Construct and cross-validate a profile configuration.
    ///
    /// # Errors
    ///
    /// Rejects a non-absolute profile identity, or a compact term without a
    /// caller-owned offline expansion. No profile term is supplied by PurRDF.
    pub fn new(
        common: ResearchObjectConfig,
        context: OfflineJsonLdContext,
        vocabulary: V,
        profile_iri: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let profile_iri = profile_iri.into();
        validate_absolute_iri(&profile_iri, &format!("{} profile identity", V::PROFILE))?;
        for (role, term) in vocabulary.terms() {
            if context.expand(term).is_none() {
                return Err(ProjectionError::configuration(format!(
                    "{} term `{term}` for role `{role:?}` has no offline expansion",
                    V::PROFILE
                )));
            }
        }
        Ok(Self {
            common,
            context,
            vocabulary,
            profile_iri,
        })
    }
}

impl<V> JsonLdProfileConfig<V> {
    /// Shared RDF vocabulary, identity, and bounds.
    pub const fn common(&self) -> &ResearchObjectConfig {
        &self.common
    }

    /// Exact emitted context and offline expansion table.
    pub const fn context(&self) -> &OfflineJsonLdContext {
        &self.context
    }

    /// Caller-owned compact terms.
    pub const fn vocabulary(&self) -> &V {
        &self.vocabulary
    }

    /// Absolute profile identity the documents declare conformance to.
    pub fn profile_iri(&self) -> &str {
        &self.profile_iri
    }
}

/// `{"@id": id, "@type": class}`, the head of a typed node object.
pub(super) fn typed_object(id: &str, class: &str) -> Object {
    Object::from_iter([
        ("@id".to_owned(), Value::String(id.to_owned())),
        ("@type".to_owned(), Value::String(class.to_owned())),
    ])
}

/// `{keyword: value}`: a node reference (`@id`) or a plain value object
/// (`@value`).
pub(super) fn keyword_object(keyword: &str, value: &str) -> Value {
    Value::Object(Object::from_iter([(
        keyword.to_owned(),
        Value::String(value.to_owned()),
    )]))
}

/// The node reference `{"@id": id}`.
pub(super) fn id_object(id: &str) -> Value {
    keyword_object("@id", id)
}

/// Write `values` under `term`, leaving the member out when there are none.
pub(super) fn insert_values(object: &mut Object, term: &str, values: Vec<Value>) {
    if !values.is_empty() {
        object.insert(term.to_owned(), Value::Array(values));
    }
}

/// The JSON Pointer of item `index` of member `term` of the object at
/// `parent`.
pub(super) fn item_pointer(parent: &str, term: &str, index: usize) -> String {
    format!("{}/{index}", json_pointer(parent, term))
}

/// A text as a value object: `@value`, then `@language` or else `@type`, then
/// `@direction` when present.
pub(super) fn text_object(value: &ResearchText) -> Value {
    let mut object = Object::from_iter([("@value".to_owned(), Value::String(value.value.clone()))]);
    if let Some(language) = &value.language {
        object.insert("@language".to_owned(), Value::String(language.clone()));
    } else {
        object.insert("@type".to_owned(), Value::String(value.datatype.clone()));
    }
    if let Some(direction) = value.direction {
        object.insert(
            "@direction".to_owned(),
            Value::String(
                purrdf_core::RdfTextDirection::from(direction)
                    .as_str()
                    .to_owned(),
            ),
        );
    }
    Value::Object(object)
}

/// A text in compact form: a plain `xsd:string` (no language, no direction)
/// as a bare JSON string, anything else as its [`text_object`].
pub(super) fn compact_text(value: &ResearchText, roles: &ResearchObjectRoles) -> Value {
    if value.datatype == roles.iri(ResearchRole::XsdString)
        && value.language.is_none()
        && value.direction.is_none()
    {
        return Value::String(value.value.clone());
    }
    text_object(value)
}

/// Each text in compact form.
pub(super) fn compact_texts(values: &[ResearchText], roles: &ResearchObjectRoles) -> Vec<Value> {
    values
        .iter()
        .map(|value| compact_text(value, roles))
        .collect()
}

/// Refuse a data path that is not a plain relative path inside the package:
/// empty, absolute, backslashed, carrying a query or fragment, or holding an
/// empty, `.` or `..` segment. `what` names the path in the refusal.
///
/// # Errors
///
/// Returns an integrity error naming `what` and `path`.
pub(super) fn validate_data_path(path: &str, what: &str) -> Result<(), ProjectionError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(['?', '#'])
        || path
            .split('/')
            .any(|segment| matches!(segment, "" | "." | ".."))
    {
        return Err(ProjectionError::integrity(format!(
            "unsafe {what} `{path}`"
        )));
    }
    Ok(())
}

/// A JSON-LD research-object profile reader.
///
/// A profile supplies its vocabulary lookup, its ledgers and how it reads a
/// node reference; the member readers, the literal grammar and the loss
/// bookkeeping are provided here once for every profile.
pub(super) trait ProfileReader {
    /// The profile's closed role vocabulary.
    type Role: Copy;

    /// The artifact every loss this reader records is located in.
    const ARTIFACT: &'static str;

    /// Whether a bare JSON string is a plain `xsd:string` literal in this
    /// profile's value grammar. DCAT writes every literal as a value object
    /// and reads only that form; Croissant and RO-Crate write a plain string
    /// bare.
    const BARE_STRING_TEXT: bool;

    /// The compact term the profile's vocabulary binds to `role`.
    fn term(&self, role: Self::Role) -> String;

    /// The research-object RDF roles.
    fn roles(&self) -> &ResearchObjectRoles;

    /// The runtime ledger and the closed contract it records against.
    fn ledgers(&mut self) -> (&mut LossLedger, &LossLedger);

    /// A node reference (an object holding `@id`) read as an IRI value.
    ///
    /// # Errors
    ///
    /// Returns the profile's refusal of the reference.
    fn parse_reference_value(
        &mut self,
        value: &Value,
        pointer: &str,
    ) -> Result<Option<ResearchValue>, ProjectionError>;

    /// Record the contract loss `code` at `pointer`.
    fn loss(&mut self, code: &'static str, pointer: &str) {
        let (ledger, contract) = self.ledgers();
        record_loss(ledger, contract, code, Self::ARTIFACT, pointer);
    }

    /// Record the value at `pointer` as unsupported and dropped.
    fn unsupported(&mut self, pointer: &str) {
        self.loss(LOSS_RESEARCH_UNSUPPORTED_VALUE_DROPPED, pointer);
    }

    /// Record every member left in `object` as unknown and dropped.
    fn record_unknowns(&mut self, object: &Object, parent: &str) {
        for member in object.keys() {
            self.loss(
                LOSS_RESEARCH_UNKNOWN_MEMBER_DROPPED,
                &json_pointer(parent, member),
            );
        }
    }

    /// Take the member bound to `role` out of `object` as its items: an array
    /// is its items (their order is dropped, and recorded as lost when there
    /// is more than one), anything else is one item, and absence is none.
    fn take_items(&mut self, object: &mut Object, role: Self::Role, parent: &str) -> Vec<Value> {
        let term = self.term(role);
        let Some(value) = object.remove(&term) else {
            return Vec::new();
        };
        match into_owned(value) {
            Owned::Array(values) => {
                if values.len() > 1 {
                    self.loss(LOSS_RESEARCH_ORDER_DROPPED, &json_pointer(parent, &term));
                }
                values
            }
            other => vec![Value::from(other)],
        }
    }

    /// Take the member bound to `role` and read each item with `parse`, at
    /// its own pointer, keeping the items `parse` admits.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of `parse`.
    fn take_parsed<T>(
        &mut self,
        object: &mut Object,
        role: Self::Role,
        parent: &str,
        mut parse: impl FnMut(&mut Self, Value, &str) -> Result<Option<T>, ProjectionError>,
    ) -> Result<Vec<T>, ProjectionError>
    where
        Self: Sized,
    {
        let term = self.term(role);
        let items = self.take_items(object, role, parent);
        let mut parsed = Vec::with_capacity(items.len());
        for (index, value) in items.into_iter().enumerate() {
            if let Some(item) = parse(self, value, &item_pointer(parent, &term, index))? {
                parsed.push(item);
            }
        }
        Ok(parsed)
    }

    /// The texts of the member bound to `role`.
    ///
    /// # Errors
    ///
    /// Returns a refusal of an invalid literal.
    fn take_texts(
        &mut self,
        object: &mut Object,
        role: Self::Role,
        parent: &str,
    ) -> Result<Vec<ResearchText>, ProjectionError>
    where
        Self: Sized,
    {
        self.take_parsed(object, role, parent, Self::parse_text)
    }

    /// The values (node references or texts) of the member bound to `role`.
    ///
    /// # Errors
    ///
    /// Returns a refusal of an invalid reference or literal.
    fn take_values(
        &mut self,
        object: &mut Object,
        role: Self::Role,
        parent: &str,
    ) -> Result<Vec<ResearchValue>, ProjectionError>
    where
        Self: Sized,
    {
        self.take_parsed(object, role, parent, Self::parse_value)
    }

    /// One value: a node reference when it is an object holding `@id`, a
    /// text otherwise.
    ///
    /// # Errors
    ///
    /// Returns a refusal of an invalid reference or literal.
    fn parse_value(
        &mut self,
        value: Value,
        pointer: &str,
    ) -> Result<Option<ResearchValue>, ProjectionError> {
        if value
            .as_object()
            .is_some_and(|object| object.contains_key("@id"))
        {
            return self.parse_reference_value(&value, pointer);
        }
        self.parse_text(value, pointer)
            .map(|value| value.map(ResearchValue::Text))
    }

    /// One literal: a value object, or (where the profile admits it) a bare
    /// string. An unreadable literal is recorded as unsupported and dropped.
    ///
    /// # Errors
    ///
    /// Returns [`ResearchText::new`]'s refusal of the literal read.
    fn parse_text(
        &mut self,
        value: Value,
        pointer: &str,
    ) -> Result<Option<ResearchText>, ProjectionError> {
        let mut object = match into_owned(value) {
            Owned::String(value) if Self::BARE_STRING_TEXT => {
                let xsd_string = self.roles().iri(ResearchRole::XsdString).to_owned();
                return ResearchText::plain(value, xsd_string).map(Some);
            }
            Owned::Object(object) => object,
            _ => {
                self.unsupported(pointer);
                return Ok(None);
            }
        };
        let Some(Owned::String(value)) = object.remove("@value").map(into_owned) else {
            self.unsupported(pointer);
            return Ok(None);
        };
        let language = match object.remove("@language").map(into_owned) {
            Some(Owned::String(language)) => Some(language),
            Some(_) => {
                self.unsupported(&json_pointer(pointer, "@language"));
                return Ok(None);
            }
            None => None,
        };
        let direction = match object.remove("@direction") {
            None => None,
            Some(value) => match value
                .as_str()
                .and_then(purrdf_core::RdfTextDirection::from_str_token)
            {
                Some(direction) => Some(ProjectionDirection::from(direction)),
                None => {
                    self.unsupported(&json_pointer(pointer, "@direction"));
                    return Ok(None);
                }
            },
        };
        let explicit_datatype = match object.remove("@type").map(into_owned) {
            Some(Owned::String(datatype)) => Some(datatype),
            Some(_) => {
                self.unsupported(&json_pointer(pointer, "@type"));
                return Ok(None);
            }
            None => None,
        };
        self.record_unknowns(&object, pointer);
        let datatype = explicit_datatype.unwrap_or_else(|| {
            let role = if direction.is_some() {
                ResearchRole::RdfDirLangString
            } else if language.is_some() {
                ResearchRole::RdfLangString
            } else {
                ResearchRole::XsdString
            };
            self.roles().iri(role).to_owned()
        });
        ResearchText::new(value, datatype, language, direction).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_data_path_is_a_plain_relative_path_inside_the_package() {
        for path in ["data.csv", "data/a.csv", "a.b/c-d_e.csv"] {
            assert!(validate_data_path(path, "data path").is_ok(), "{path}");
        }
        for path in [
            "",
            "/data.csv",
            "data\\a.csv",
            "a.csv?x",
            "a.csv#x",
            "a//b",
            "./a",
            "a/../b",
            "a/",
        ] {
            let error = validate_data_path(path, "data path").expect_err(path);
            assert!(error.to_string().contains("unsafe data path"), "{error}");
        }
    }

    #[test]
    fn node_and_value_objects_write_their_one_member() {
        assert_eq!(
            purrdf_lex::json::write_compact(&id_object("https://example.org/a")),
            r#"{"@id":"https://example.org/a"}"#
        );
        assert_eq!(
            purrdf_lex::json::write_compact(&keyword_object("@value", "x")),
            r#"{"@value":"x"}"#
        );
        let mut object = typed_object("https://example.org/a", "Dataset");
        insert_values(&mut object, "name", Vec::new());
        assert_eq!(object.len(), 2);
        insert_values(&mut object, "name", vec![Value::from("n")]);
        assert_eq!(object.len(), 3);
        assert_eq!(item_pointer("/a", "b/c", 2), "/a/b~1c/2");
    }
}
