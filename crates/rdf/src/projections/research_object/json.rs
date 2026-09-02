// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::borrow::Cow;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::loss::{LossEntry, check_ledger_sound};
use purrdf_core::{LossLedger, RdfDataset, RdfLocation};
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};

use crate::native_codecs::jsonld::{
    CompiledJsonLdContext, parse_jsonld, serialize_dataset_to_jsonld,
};

use super::super::{ProjectionError, ProjectionLimits, ProjectionPackage, validate_absolute_iri};
use super::{ResearchObjectConfig, ResearchObjectModel};

/// Caller-owned, locally interpreted JSON-LD context.
///
/// `value` is carried byte-semantically into emitted documents. `definitions`
/// is the complete offline expansion table used by profile adapters; PurRDF
/// never dereferences a context IRI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OfflineJsonLdContext {
    value: Value,
    definitions: BTreeMap<String, String>,
    #[serde(skip)]
    compiled: Arc<CompiledJsonLdContext>,
}

impl OfflineJsonLdContext {
    /// Validate one emitted context value and its local term definitions.
    ///
    /// # Errors
    ///
    /// Rejects an unusable context shape, an import directive, an empty or
    /// keyword-like compact term, a non-absolute expansion, or ambiguous IRI
    /// aliases.
    pub fn new(
        value: Value,
        definitions: BTreeMap<String, String>,
    ) -> Result<Self, ProjectionError> {
        validate_context_value(&value)?;
        if definitions.is_empty() {
            return Err(ProjectionError::configuration(
                "offline JSON-LD context requires local term definitions",
            ));
        }
        let mut expanded = BTreeSet::new();
        for (term, iri) in &definitions {
            if term.is_empty()
                || term.starts_with('@')
                || term.chars().any(char::is_whitespace)
                || term.contains(['{', '}', '[', ']', '"'])
            {
                return Err(ProjectionError::configuration(format!(
                    "invalid offline JSON-LD compact term `{term}`"
                )));
            }
            validate_absolute_iri(iri, &format!("offline JSON-LD term `{term}`"))?;
            if !expanded.insert(iri.as_str()) {
                return Err(ProjectionError::configuration(format!(
                    "offline JSON-LD context maps more than one compact term to `{iri}`"
                )));
            }
        }
        let compiled_value = Value::Object(
            definitions
                .iter()
                // Profile lookup tables historically permit colon-bearing role keys
                // whose expansion is not CURIE concatenation. Those remain explicit
                // profile rules; plain JSON-LD terms use the shared context engine.
                .filter(|(term, _)| !term.contains(':'))
                .map(|(term, iri)| (term.clone(), Value::String(iri.clone())))
                .collect(),
        );
        let compiled = CompiledJsonLdContext::compile(&compiled_value, None).map_err(|error| {
            ProjectionError::configuration(format!(
                "compile offline JSON-LD term definitions: {error}"
            ))
        })?;
        Ok(Self {
            value,
            definitions,
            compiled: Arc::new(compiled),
        })
    }

    /// Exact JSON value emitted as `@context`.
    pub const fn value(&self) -> &Value {
        &self.value
    }

    /// Complete, deterministic compact-term expansion table.
    pub const fn definitions(&self) -> &BTreeMap<String, String> {
        &self.definitions
    }

    /// Reusable compiled active context backing this compatibility adapter.
    pub fn compiled_context(&self) -> &CompiledJsonLdContext {
        &self.compiled
    }

    /// Resolve a configured compact term to its absolute vocabulary IRI.
    pub fn expand(&self, term: &str) -> Option<&str> {
        let expanded = self.definitions.get(term).map(String::as_str);
        if !term.contains(':') {
            debug_assert_eq!(
                self.compiled
                    .expand_iri(term, true, false)
                    .ok()
                    .flatten()
                    .as_deref(),
                expanded
            );
        }
        expanded
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOfflineJsonLdContext {
    value: Value,
    definitions: BTreeMap<String, String>,
}

impl<'de> Deserialize<'de> for OfflineJsonLdContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawOfflineJsonLdContext::deserialize(deserializer)?;
        Self::new(raw.value, raw.definitions).map_err(serde::de::Error::custom)
    }
}

fn validate_context_value(value: &Value) -> Result<(), ProjectionError> {
    match value {
        Value::String(iri) => validate_absolute_iri(iri, "JSON-LD context identity"),
        Value::Object(values) => {
            if values.contains_key("@import") {
                return Err(ProjectionError::configuration(
                    "offline JSON-LD contexts cannot use @import",
                ));
            }
            if values.is_empty() {
                return Err(ProjectionError::configuration(
                    "JSON-LD context object cannot be empty",
                ));
            }
            Ok(())
        }
        Value::Array(values) if !values.is_empty() => {
            for value in values {
                validate_context_value(value)?;
            }
            Ok(())
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => {
            Err(ProjectionError::configuration(
                "JSON-LD context must be an IRI, a non-empty object, or a non-empty array",
            ))
        }
    }
}

/// Native-profile projection result before USTAR encoding.
#[derive(Debug, Clone)]
pub struct ResearchObjectPackageProjection {
    /// Canonical profile artifact package.
    pub package: ProjectionPackage,
    /// Normalized semantic pivot that was encoded.
    pub model: ResearchObjectModel,
    /// Located RDF-to-profile losses.
    pub loss_ledger: LossLedger,
}

/// Native-profile reader result after caller-vocabulary RDF lift.
#[derive(Debug, Clone)]
pub struct ResearchObjectReadOutcome {
    /// Lifted and JSON-LD-normalized RDF 1.2 dataset.
    pub dataset: Arc<RdfDataset>,
    /// Normalized semantic pivot interpreted from the native document.
    pub model: ResearchObjectModel,
    /// Located profile-to-RDF losses.
    pub loss_ledger: LossLedger,
}

pub(super) fn canonical_json(
    value: &Value,
    limits: ProjectionLimits,
    description: &str,
) -> Result<Vec<u8>, ProjectionError> {
    let mut bytes = super::super::util::canonical_json_bounded(value, limits, description)?;
    if bytes.len() == limits.max_artifact_bytes() {
        return Err(ProjectionError::limit(format!(
            "{description} plus its canonical newline exceeds the {}-byte artifact limit",
            limits.max_artifact_bytes()
        )));
    }
    bytes.push(b'\n');
    Ok(bytes)
}

pub(super) fn parse_strict_json(
    bytes: &[u8],
    config: &ResearchObjectConfig,
    description: &str,
    path: &str,
) -> Result<Value, ProjectionError> {
    if bytes.len() > config.limits().max_artifact_bytes() {
        return Err(ProjectionError::limit(format!(
            "{description} exceeds the per-artifact byte limit"
        ))
        .at_path(path));
    }
    let remaining = Cell::new(config.policy().max_records());
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let seed = StrictJsonSeed {
        remaining: &remaining,
        depth: 0,
        max_depth: config.policy().max_json_depth(),
    };
    let value = seed.deserialize(&mut deserializer).map_err(|error| {
        ProjectionError::syntax(format!("parse {description}: {error}")).at_path(path)
    })?;
    deserializer.end().map_err(|error| {
        ProjectionError::syntax(format!("parse {description}: {error}")).at_path(path)
    })?;
    Ok(value)
}

#[derive(Clone, Copy)]
struct StrictJsonSeed<'a> {
    remaining: &'a Cell<usize>,
    depth: usize,
    max_depth: usize,
}

impl<'de> DeserializeSeed<'de> for StrictJsonSeed<'_> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        if self.depth > self.max_depth {
            return Err(D::Error::custom(format!(
                "JSON nesting exceeds depth limit {}",
                self.max_depth
            )));
        }
        let remaining = self.remaining.get();
        if remaining == 0 {
            return Err(D::Error::custom(
                "JSON value count exceeds configured limit",
            ));
        }
        self.remaining.set(remaining - 1);
        deserializer.deserialize_any(StrictJsonVisitor(self))
    }
}

struct StrictJsonVisitor<'a>(StrictJsonSeed<'a>);

impl<'de> Visitor<'de> for StrictJsonVisitor<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a duplicate-free JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.0.deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let child = StrictJsonSeed {
            depth: self.0.depth + 1,
            ..self.0
        };
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(1_024));
        while let Some(value) = sequence.next_element_seed(child)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let child = StrictJsonSeed {
            depth: self.0.depth + 1,
            ..self.0
        };
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(A::Error::custom(format!(
                    "duplicate JSON object member `{key}`"
                )));
            }
            values.insert(key, map.next_value_seed(child)?);
        }
        Ok(Value::Object(values))
    }
}

pub(super) fn require_artifact<'a>(
    package: &'a ProjectionPackage,
    path: &str,
    config: &ResearchObjectConfig,
) -> Result<&'a [u8], ProjectionError> {
    validate_package_bounds(package, config.limits())?;
    if package.len() != 1 {
        return Err(ProjectionError::package(format!(
            "research-object package must contain exactly `{path}`"
        )));
    }
    package
        .get(path)
        .ok_or_else(|| ProjectionError::package("required artifact is missing").at_path(path))
}

fn validate_package_bounds(
    package: &ProjectionPackage,
    limits: ProjectionLimits,
) -> Result<(), ProjectionError> {
    if package.len() > limits.max_artifacts()
        || package.total_bytes() > limits.max_total_bytes()
        || package.archive_bytes() > limits.max_archive_bytes()
    {
        return Err(ProjectionError::limit(
            "research-object package exceeds configured limits",
        ));
    }
    for (path, bytes) in package.artifacts() {
        if bytes.len() > limits.max_artifact_bytes() {
            return Err(ProjectionError::limit(format!(
                "artifact is {} bytes; reader limit is {}",
                bytes.len(),
                limits.max_artifact_bytes()
            ))
            .at_path(path));
        }
    }
    Ok(())
}

pub(super) fn record_loss(
    ledger: &mut LossLedger,
    contract: &LossLedger,
    code: &'static str,
    path: &str,
    subject: &str,
) {
    let template = contract
        .entries()
        .iter()
        .find(|entry| entry.code == code)
        .expect("native research-object loss must exist in closed contract");
    ledger.record(LossEntry {
        code: Cow::Borrowed(code),
        from: template.from.clone(),
        to: template.to.clone(),
        note: template.note.clone(),
        location: Some(Box::new(
            RdfLocation::file(path).with_subject(subject.to_owned()),
        )),
    });
}

pub(super) fn ensure_sound(
    ledger: &LossLedger,
    from: &str,
    to: &str,
) -> Result<(), ProjectionError> {
    check_ledger_sound(ledger, from, to).map_err(ProjectionError::integrity)
}

/// Re-parse a lifted dataset's own JSON-LD serialization, so every profile adapter hands
/// back a dataset in the one normalized shape.
///
/// # This is NOT the caller-facing ingress, and no base belongs here
///
/// RO-Crate metadata canonically spells `@id` relatively (`"./"`,
/// `"ro-crate-metadata.json"`, `"data/train.csv"`), so it is reasonable to expect the
/// RO-Crate lane's JSON-LD reader to need a document base. It does — but that reader is
/// not this function, and this function is not on the read path from caller bytes.
///
/// The caller-facing ingress is [`read_ro_crate`](super::ro_crate::read_ro_crate) (and its
/// siblings `read_croissant` / `read_datacite` / `read_dcat` / `read_frictionless`), which
/// takes the package bytes through [`parse_strict_json`] — plain JSON, not JSON-LD — into
/// each profile's own `decode_document`. A relative `@id` is resolved there, by
/// [`ResearchObjectIdentity::resolve_relative`](super::ResearchObjectIdentity::resolve_relative),
/// against the **caller-owned** `entity_base_iri` the configuration carries. That base is
/// configuration with no fabricated default: a caller who supplies none gets no projection
/// at all. `parse_jsonld` is never reached from caller bytes in this lane.
///
/// What reaches THIS function is JSON-LD that `serialize_dataset_to_jsonld` produced from
/// a frozen dataset one line above, and that serializer emits every IRI absolute — which
/// `serializer_emits_no_relative_id_for_a_base_to_resolve` asserts rather than assumes. So
/// `None` is not a discarded base: there is no relative reference for one to resolve, and
/// inventing one here would make a round trip depend on a value the round trip never saw.
pub(super) fn normalize_lifted_jsonld(
    dataset: &RdfDataset,
) -> Result<Arc<RdfDataset>, ProjectionError> {
    let json = serialize_dataset_to_jsonld(dataset).map_err(|error| {
        ProjectionError::integrity(format!("normalize lifted research-object JSON-LD: {error}"))
    })?;
    parse_jsonld(json.as_bytes(), None).map_err(|error| {
        ProjectionError::integrity(format!("reparse lifted research-object JSON-LD: {error}"))
    })
}

pub(super) fn json_pointer(parent: &str, member: &str) -> String {
    let escaped = escape_json_pointer_member(member);
    if parent.is_empty() {
        format!("/{escaped}")
    } else {
        format!("{parent}/{escaped}")
    }
}

fn escape_json_pointer_member(member: &str) -> Cow<'_, str> {
    let escape_count = member
        .bytes()
        .filter(|byte| matches!(byte, b'~' | b'/'))
        .count();
    if escape_count == 0 {
        return Cow::Borrowed(member);
    }

    let mut escaped = String::with_capacity(member.len() + escape_count);
    for character in member.chars() {
        match character {
            '~' => escaped.push_str("~0"),
            '/' => escaped.push_str("~1"),
            _ => escaped.push(character),
        }
    }
    Cow::Owned(escaped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_json_rejects_duplicate_members_depth_and_value_exhaustion() {
        let parse = |bytes: &[u8], values: usize, depth: usize| {
            let remaining = Cell::new(values);
            let mut deserializer = serde_json::Deserializer::from_slice(bytes);
            StrictJsonSeed {
                remaining: &remaining,
                depth: 0,
                max_depth: depth,
            }
            .deserialize(&mut deserializer)
        };

        assert!(parse(br#"{"a":1,"a":2}"#, 10, 4).is_err());
        assert!(parse(br"[[[0]]]", 10, 1).is_err());
        assert!(parse(br"[1,2,3]", 3, 4).is_err());
        assert_eq!(
            parse(br#"{"a":[1,true]}"#, 10, 4).expect("value")["a"][0],
            1
        );
    }

    #[test]
    fn offline_context_is_complete_absolute_and_import_free() {
        let valid = OfflineJsonLdContext::new(
            Value::String("https://example.org/context.jsonld".to_owned()),
            BTreeMap::from([("name".to_owned(), "https://example.org/name".to_owned())]),
        )
        .expect("context");
        assert_eq!(valid.expand("name"), Some("https://example.org/name"));
        assert_eq!(
            valid
                .compiled_context()
                .expand_iri("name", true, false)
                .expect("expand configured term")
                .as_deref(),
            Some("https://example.org/name")
        );

        assert!(
            OfflineJsonLdContext::new(
                serde_json::json!({"@import": "https://example.org/base"}),
                valid.definitions,
            )
            .is_err()
        );
    }

    /// The precondition [`normalize_lifted_jsonld`]'s `None` base rests on, asserted
    /// instead of assumed: the serializer it re-parses emits no relative reference, so
    /// there is nothing for a base to resolve on this internal round trip.
    #[test]
    fn serializer_emits_no_relative_id_for_a_base_to_resolve() {
        let dataset = lifted_fixture();
        let json = serialize_dataset_to_jsonld(&dataset)
            .expect("the lifted dataset serializes to JSON-LD");
        let value: Value = serde_json::from_str(&json).expect("serializer emits JSON");

        let mut ids = Vec::new();
        collect_ids(&value, &mut ids);
        assert!(!ids.is_empty(), "the fixture must produce node identifiers");
        for id in ids {
            assert!(
                purrdf_iri::parse(&id).is_ok_and(|iri| iri.has_scheme()),
                "serialize_dataset_to_jsonld must emit only absolute IRIs, found {id:?}"
            );
        }
    }

    /// And the round trip is faithful: re-parsing under no base returns the same quads,
    /// so the `None` is not quietly dropping anything either.
    #[test]
    fn the_internal_round_trip_preserves_every_quad() {
        let dataset = lifted_fixture();
        let normalized = normalize_lifted_jsonld(&dataset).expect("the round trip succeeds");
        assert_eq!(normalized.quad_count(), dataset.quad_count());
        let before: BTreeSet<String> = dataset.owned_quads().map(|q| format!("{q:?}")).collect();
        let after: BTreeSet<String> = normalized.owned_quads().map(|q| format!("{q:?}")).collect();
        assert_eq!(before, after);
    }

    /// A dataset shaped like a lifted research object: an absolute entity IRI carrying a
    /// typed edge to another entity and a literal.
    fn lifted_fixture() -> Arc<RdfDataset> {
        use purrdf_core::TermFactory as _;

        let mut builder = purrdf_core::RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/crate/");
        let predicate = builder.intern_iri("https://example.org/vocab/hasPart");
        let object = builder.intern_iri("https://example.org/crate/data/train.csv");
        builder.push_quad(subject, predicate, object, None);
        let label = builder.intern_iri("https://example.org/vocab/name");
        let literal = builder.intern_value(&purrdf_core::TermValue::simple_literal("train"));
        builder.push_quad(object, label, literal, None);
        builder.freeze().expect("fixture freezes")
    }

    /// Every `@id` string anywhere in a JSON-LD document, in document order.
    fn collect_ids(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(members) => {
                for (key, member) in members {
                    if key == "@id"
                        && let Value::String(id) = member
                    {
                        out.push(id.clone());
                    }
                    collect_ids(member, out);
                }
            }
            Value::Array(values) => {
                for member in values {
                    collect_ids(member, out);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn json_pointer_borrows_clean_members_and_escapes_rfc6901_tokens() {
        assert!(matches!(
            escape_json_pointer_member("plain"),
            Cow::Borrowed("plain")
        ));
        assert_eq!(escape_json_pointer_member("a~/b"), "a~0~1b");
        assert_eq!(json_pointer("", "plain"), "/plain");
        assert_eq!(json_pointer("/items/0", "a~/b"), "/items/0/a~0~1b");
    }
}
