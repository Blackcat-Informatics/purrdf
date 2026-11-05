// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Expanded-value preferences for the existing inverse-context selector (§6.2).

use super::{
    CompiledJsonLdContext, JsonLdContainer as C, JsonLdTermSelection, JsonLdTermSelectionKind as K,
    JsonLdTypeMapping, RdfDiagnostic,
};
use purrdf_lex::json::Value;

pub(super) fn compact(
    context: &CompiledJsonLdContext,
    iri: &str,
    value: &Value,
    reverse: bool,
    json_eligible: bool,
    used_lists: &[String],
) -> Result<String, RdfDiagnostic> {
    let preferences = selection(context, value, reverse, json_eligible)?;
    let state = Candidate {
        context,
        iri,
        value,
        reverse,
        json_eligible,
        used_lists,
    };
    let accept = |term: &str| state.compatible(term);
    let term = context.compact_property_accepting(iri, &preferences, accept)?;
    if accept(&term) {
        return Ok(term);
    }
    // A full-IRI fallback can itself have a definition. Reuse the same inverse
    // index to find a safe unused List key, then an ordinary explicit spelling.
    if value.get("@list").is_some() {
        let lists = JsonLdTermSelection::new([vec![C::List]], K::Any, ["@none"]);
        let term = context.compact_property_accepting(iri, &lists, accept)?;
        if accept(&term) {
            return Ok(term);
        }
    }
    let ordinary = JsonLdTermSelection::new([vec![C::Set], Vec::new()], K::Any, ["@none"]);
    let term = context.compact_property_accepting(iri, &ordinary, accept)?;
    if accept(&term) {
        return Ok(term);
    }
    Err(super::context_error(format!(
        "output context has no lossless property spelling for `{iri}`; supply an unused compatible term or prefix"
    )))
}

struct Candidate<'a> {
    context: &'a CompiledJsonLdContext,
    iri: &'a str,
    value: &'a Value,
    reverse: bool,
    json_eligible: bool,
    used_lists: &'a [String],
}
impl Candidate<'_> {
    fn compatible(&self, term: &str) -> bool {
        let Some(definition) = self.context.term(term) else {
            return true;
        };
        if definition.iri_mapping() != Some(self.iri)
            || definition.is_reverse_property() && !self.reverse
        {
            return false;
        }
        if definition.type_mapping() == Some(&JsonLdTypeMapping::Json) {
            return self.json_eligible && !definition.containers().contains(&C::Set);
        }
        let has = |container| definition.containers().contains(&container);
        let annotated = self.value.get("@annotation").is_some();
        let list = self.value.get("@list").is_some();
        let graph = self.value.get("@graph").is_some();
        if has(C::List)
            && (!list
                || self
                    .used_lists
                    .binary_search_by(|used| used.as_str().cmp(term))
                    .is_ok())
        {
            return false;
        }
        if has(C::Graph) {
            if !graph || annotated && !has(C::Id) && !has(C::Index) {
                return false;
            }
            if !has(C::Id)
                && !has(C::Index)
                && self
                    .value
                    .as_object()
                    .is_some_and(|object| object.keys().any(|key| key != "@graph"))
            {
                return false;
            }
        }
        true
    }
}

/// Ordinary map syntax is optional for raw arrays in the shared expansion home.
/// Preserve explicit objects when a map key or its implied metadata is unsafe.
pub(super) fn explicit_values(
    context: &CompiledJsonLdContext,
    term: &str,
    value: &Value,
) -> Result<bool, RdfDiagnostic> {
    let Some(definition) = context.term(term) else {
        return Ok(false);
    };
    let has = |container| definition.containers().contains(&container);
    if has(C::Graph) || has(C::List) || definition.type_mapping() == Some(&JsonLdTypeMapping::Json)
    {
        return Ok(false);
    }
    if ![C::Language, C::Index, C::Id, C::Type].into_iter().any(has) {
        return Ok(false);
    }
    if value.get("@annotation").is_some() {
        return Ok(true);
    }
    let literal = value.get("@value").is_some();
    let non_node = literal
        || value.get("@list").is_some()
        || value.get("@graph").is_some()
        || value.get("@triple").is_some();
    if (has(C::Id) || has(C::Type)) && non_node {
        return Ok(true);
    }
    if has(C::Index) {
        if value.get("@list").is_some()
            || value.get("@graph").is_some()
            || definition.index_mapping().is_some() && literal
        {
            return Ok(true);
        }
        if keep_index(context, term, value)? {
            return Ok(true);
        }
    }
    if has(C::Language) {
        let direction = match definition.direction_mapping() {
            Some(super::JsonLdNullable::Null) => None,
            Some(super::JsonLdNullable::Value(direction)) => Some(direction.as_str()),
            None => context
                .default_direction()
                .map(super::JsonLdDirection::as_str),
        };
        if !literal
            || value.get("@type").is_some()
            || value.get("@index").is_some()
            || value.get("@direction").and_then(Value::as_str) != direction
        {
            return Ok(true);
        }
        if let Some(key) = value.get("@language").and_then(Value::as_str)
            && context.expand_iri(key, true, false)?.as_deref() == Some("@none")
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// A sentinel-looking semantic index must stay in its explicit value object.
pub(super) fn keep_index(
    context: &CompiledJsonLdContext,
    term: &str,
    value: &Value,
) -> Result<bool, RdfDiagnostic> {
    let Some(definition) = context.term(term) else {
        return Ok(false);
    };
    if !definition.containers().contains(&C::Index) || definition.index_mapping().is_some() {
        return Ok(false);
    }
    let Some(key) = value.get("@index").and_then(Value::as_str) else {
        return Ok(false);
    };
    Ok(context.expand_iri(key, true, false)?.as_deref() == Some("@none"))
}

fn selection(
    context: &CompiledJsonLdContext,
    value: &Value,
    reverse: bool,
    json_eligible: bool,
) -> Result<JsonLdTermSelection, RdfDiagnostic> {
    let mut containers = Vec::new();
    let indexed = value.get("@index").is_some();
    let graph = value.get("@graph").is_some();
    if indexed && !graph {
        containers.extend([vec![C::Index], vec![C::Index, C::Set]]);
    }
    let (mut kind, mut preferred) = if reverse {
        containers.push(vec![C::Set]);
        (K::Type, vec!["@reverse".to_owned()])
    } else if let Some(list) = value.get("@list").and_then(Value::as_array) {
        if !indexed {
            containers.push(vec![C::List]);
        }
        list_preferences(context, list)
    } else if graph {
        if indexed {
            containers.extend([vec![C::Graph, C::Index], vec![C::Graph, C::Index, C::Set]]);
        }
        if value.get("@id").is_some() {
            containers.extend([vec![C::Graph, C::Id], vec![C::Graph, C::Id, C::Set]]);
        }
        containers.extend([vec![C::Graph], vec![C::Graph, C::Set], vec![C::Set]]);
        if !indexed {
            containers.extend([vec![C::Graph, C::Index], vec![C::Graph, C::Index, C::Set]]);
        }
        if value.get("@id").is_none() {
            containers.extend([vec![C::Graph, C::Id], vec![C::Graph, C::Id, C::Set]]);
        }
        containers.extend([vec![C::Index], vec![C::Index, C::Set]]);
        (K::Type, vec!["@id".to_owned()])
    } else if value.get("@value").is_some() {
        if !indexed && (value.get("@language").is_some() || value.get("@direction").is_some()) {
            containers.extend([vec![C::Language], vec![C::Language, C::Set]]);
        }
        preference(context, value)
    } else {
        containers.extend([
            vec![C::Id],
            vec![C::Id, C::Set],
            vec![C::Type],
            vec![C::Type, C::Set],
        ]);
        preference(context, value)
    };
    containers.extend([vec![C::Set], Vec::new()]);
    if !indexed {
        containers.extend([vec![C::Index], vec![C::Index, C::Set]]);
    }
    if value.get("@value").is_some()
        && value.get("@language").is_none()
        && value.get("@direction").is_none()
        && value.get("@type").is_none()
    {
        containers.extend([vec![C::Language], vec![C::Language, C::Set]]);
    }
    if value.get("@annotation").is_some() {
        containers.retain(|container| container.is_empty() || container == &[C::Set]);
        preferred = vec!["@none".to_owned()];
    }
    if !json_eligible {
        preferred.retain(|value| value != "@json");
    }
    if preferred.is_empty() {
        preferred.push("@none".to_owned());
    }
    if preferred.first().is_some_and(|value| value == "@id") {
        let id = value.get("@id").and_then(Value::as_str);
        let vocab_first = if let Some(id) = id {
            let term = context.compact_iri(id, true)?;
            context
                .term(&term)
                .is_some_and(|definition| definition.iri_mapping() == Some(id))
        } else {
            false
        };
        preferred = if vocab_first {
            vec!["@vocab".to_owned(), "@id".to_owned()]
        } else {
            vec!["@id".to_owned(), "@vocab".to_owned()]
        };
    }
    preferred.push("@none".to_owned());
    if value
        .get("@list")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        kind = K::Any;
    }
    preferred.push("@any".to_owned());
    // REC §6.2: a direction-only term remains eligible for any language.
    if let Some(suffix) = preferred
        .iter()
        .find_map(|value| value.find('_').map(|at| value[at..].to_owned()))
    {
        preferred.push(suffix);
    }
    Ok(JsonLdTermSelection::new(containers, kind, preferred))
}

fn default_language(context: &CompiledJsonLdContext) -> String {
    match (context.default_language(), context.default_direction()) {
        (language, Some(direction)) => {
            format!("{}_{}", language.unwrap_or_default(), direction.as_str())
        }
        (Some(language), None) => language.to_owned(),
        (None, None) => "@none".to_owned(),
    }
}
fn preference(context: &CompiledJsonLdContext, value: &Value) -> (K, Vec<String>) {
    if value.get("@value").is_none() {
        return (K::Type, vec!["@id".to_owned()]);
    }
    if let Some(datatype) = value.get("@type").and_then(Value::as_str) {
        return (K::Type, vec![datatype.to_owned()]);
    }
    let language = value.get("@language").and_then(Value::as_str);
    let direction = value.get("@direction").and_then(Value::as_str);
    let mut preferred = match (language, direction) {
        (language, Some(direction)) => vec![format!(
            "{}_{}",
            purrdf_iri::langtag::identity_fold(language.unwrap_or_default()),
            direction
        )],
        (Some(language), None) => vec![purrdf_iri::langtag::identity_fold(language)],
        (None, None) => vec!["@null".to_owned()],
    };
    if language.is_none() && direction.is_none() {
        preferred.push(default_language(context));
    }
    (K::Language, preferred)
}

fn list_preferences(context: &CompiledJsonLdContext, list: &[Value]) -> (K, Vec<String>) {
    if list.is_empty() {
        return (K::Language, vec![default_language(context)]);
    }
    let mut common_language: Option<String> = None;
    let mut common_type: Option<&str> = None;
    for item in list {
        let is_value = item.get("@value").is_some();
        let (language, datatype) = if !is_value {
            ("@none".to_owned(), "@id")
        } else if let Some(direction) = item.get("@direction").and_then(Value::as_str) {
            (
                format!(
                    "{}_{}",
                    purrdf_iri::langtag::identity_fold(
                        item.get("@language")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                    ),
                    direction
                ),
                "@none",
            )
        } else if let Some(language) = item.get("@language").and_then(Value::as_str) {
            (purrdf_iri::langtag::identity_fold(language), "@none")
        } else if let Some(datatype) = item.get("@type").and_then(Value::as_str) {
            ("@none".to_owned(), datatype)
        } else {
            ("@null".to_owned(), "@none")
        };
        match &common_language {
            None => common_language = Some(language),
            Some(common) if common != &language && is_value => {
                common_language = Some("@none".to_owned());
            }
            _ => {}
        }
        match common_type {
            None => common_type = Some(datatype),
            Some(common) if common != datatype => common_type = Some("@none"),
            _ => {}
        }
    }
    if let Some(datatype) = common_type.filter(|datatype| *datatype != "@none") {
        (K::Type, vec![datatype.to_owned()])
    } else {
        (
            K::Language,
            vec![common_language.unwrap_or_else(|| "@none".to_owned())],
        )
    }
}
