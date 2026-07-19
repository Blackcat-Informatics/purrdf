// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! JSON-LD 1.1 active-context and inverse-context algorithms.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::{Map, Value};

use super::{
    ActiveContext, CompiledJsonLdContext, InverseContext, JsonLdContainer, JsonLdContextLimits,
    JsonLdContextRegistry, JsonLdDirection, JsonLdNullable, JsonLdTermDefinition,
    JsonLdTermSelection, JsonLdTermSelectionKind, JsonLdTypeMapping, canonicalize, context_error,
    context_limit, validate_absolute_iri,
};
use crate::RdfDiagnostic;

const KEYWORDS: &[&str] = &[
    "@base",
    "@container",
    "@context",
    "@direction",
    "@graph",
    "@id",
    "@import",
    "@included",
    "@index",
    "@json",
    "@language",
    "@list",
    "@nest",
    "@none",
    "@prefix",
    "@propagate",
    "@protected",
    "@reverse",
    "@set",
    "@type",
    "@value",
    "@version",
    "@vocab",
];

const CONTEXT_KEYWORDS: &[&str] = &[
    "@base",
    "@direction",
    "@import",
    "@language",
    "@propagate",
    "@protected",
    "@type",
    "@version",
    "@vocab",
];

const EXTENSION_CONTROLS: &[&str] = &[
    "@annotation",
    "@object",
    "@predicate",
    "@subject",
    "@triple",
];

const TERM_DEFINITION_KEYS: &[&str] = &[
    "@container",
    "@context",
    "@direction",
    "@id",
    "@index",
    "@language",
    "@nest",
    "@prefix",
    "@protected",
    "@reverse",
    "@type",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefinitionState {
    Defining,
    Defined,
}

#[derive(Debug, Default)]
struct Budget {
    terms: usize,
    work: usize,
    complexity: usize,
    loaded_bytes: usize,
}

struct Compiler<'a> {
    registry: &'a JsonLdContextRegistry,
    limits: JsonLdContextLimits,
    initial_base: Option<String>,
    budget: Budget,
    remote_stack: Vec<String>,
    remote_cache: BTreeMap<String, Arc<Value>>,
    override_protected: bool,
}

pub(super) fn compile(
    supplied: &Value,
    document_iri: Option<&str>,
    registry: &JsonLdContextRegistry,
    limits: JsonLdContextLimits,
) -> Result<CompiledJsonLdContext, RdfDiagnostic> {
    let document_iri = document_iri.map(str::to_owned);
    if let Some(iri) = &document_iri {
        validate_absolute_iri(iri, "JSON-LD document IRI")?;
    }
    let context = extract_context(supplied)?;
    let mut compiler = Compiler {
        registry,
        limits,
        initial_base: document_iri.clone(),
        budget: Budget::default(),
        remote_stack: Vec::new(),
        remote_cache: BTreeMap::new(),
        override_protected: false,
    };
    compiler.charge_value(context, 0)?;
    let canonical_context = canonicalize(context);
    let context_bytes = serde_json::to_vec(&canonical_context)
        .map_err(|source| context_error(format!("encode canonical JSON-LD context: {source}")))?;
    if context_bytes.len() > limits.max_context_bytes() {
        return Err(context_limit(format!(
            "canonical JSON-LD context is {} bytes; limit is {}",
            context_bytes.len(),
            limits.max_context_bytes()
        )));
    }
    let mut active = ActiveContext::new(document_iri.clone());
    compiler.process_context(&mut active, context, document_iri.as_deref(), false, 0)?;
    let inverse = build_inverse_context(&active);
    Ok(CompiledJsonLdContext {
        canonical_context,
        active,
        inverse,
        registry: registry.clone(),
    })
}

pub(super) fn compile_scoped(
    parent: &CompiledJsonLdContext,
    definition: &JsonLdTermDefinition,
) -> Result<Option<CompiledJsonLdContext>, RdfDiagnostic> {
    let Some(context) = definition.scoped_context.as_ref() else {
        return Ok(None);
    };
    apply_context(
        parent,
        context,
        definition.scoped_context_base.as_deref(),
        true,
    )
    .map(Some)
}

pub(super) fn apply_context(
    parent: &CompiledJsonLdContext,
    context: &Value,
    base_url: Option<&str>,
    override_protected: bool,
) -> Result<CompiledJsonLdContext, RdfDiagnostic> {
    let limits = JsonLdContextLimits::default();
    let mut compiler = Compiler {
        registry: &parent.registry,
        limits,
        initial_base: parent.active.original_base_iri.clone(),
        budget: Budget::default(),
        remote_stack: Vec::new(),
        remote_cache: BTreeMap::new(),
        override_protected,
    };
    compiler.charge_value(context, 0)?;
    let mut active = parent.active.clone();
    compiler.process_context(
        &mut active,
        context,
        base_url.or(parent.active.base_iri.as_deref()),
        false,
        0,
    )?;
    let inverse = build_inverse_context(&active);
    Ok(CompiledJsonLdContext {
        canonical_context: canonicalize(context),
        active,
        inverse,
        registry: parent.registry.clone(),
    })
}

pub(super) fn child_context(parent: &CompiledJsonLdContext) -> CompiledJsonLdContext {
    let active = if parent.active.propagate {
        parent.active.clone()
    } else {
        parent
            .active
            .previous_context
            .as_deref()
            .cloned()
            .unwrap_or_else(|| parent.active.clone())
    };
    CompiledJsonLdContext {
        canonical_context: parent.canonical_context.clone(),
        inverse: build_inverse_context(&active),
        active,
        registry: parent.registry.clone(),
    }
}

fn extract_context(value: &Value) -> Result<&Value, RdfDiagnostic> {
    match value {
        Value::Object(object) if object.contains_key("@context") => object
            .get("@context")
            .ok_or_else(|| context_error("JSON-LD context wrapper is missing @context")),
        _ => Ok(value),
    }
}

impl Compiler<'_> {
    fn charge_work(&mut self, description: &str) -> Result<(), RdfDiagnostic> {
        self.budget.work = self
            .budget
            .work
            .checked_add(1)
            .ok_or_else(|| context_limit("JSON-LD context work counter overflow"))?;
        if self.budget.work > self.limits.max_expansion_work() {
            return Err(context_limit(format!(
                "JSON-LD context expansion work exceeds {} operations while {description}",
                self.limits.max_expansion_work()
            )));
        }
        Ok(())
    }

    fn charge_complexity(&mut self, amount: usize, description: &str) -> Result<(), RdfDiagnostic> {
        self.budget.complexity = self
            .budget
            .complexity
            .checked_add(amount)
            .ok_or_else(|| context_limit("JSON-LD definition-complexity counter overflow"))?;
        if self.budget.complexity > self.limits.max_definition_complexity() {
            return Err(context_limit(format!(
                "JSON-LD definition complexity exceeds {} while {description}",
                self.limits.max_definition_complexity()
            )));
        }
        Ok(())
    }

    fn charge_value(&mut self, value: &Value, depth: usize) -> Result<(), RdfDiagnostic> {
        if depth > self.limits.max_nesting() {
            return Err(context_limit(format!(
                "JSON-LD context nesting exceeds depth {}",
                self.limits.max_nesting()
            )));
        }
        self.charge_work("walking the supplied context")?;
        match value {
            Value::String(text) => self.charge_complexity(text.len(), "reading a string"),
            Value::Array(values) => {
                self.charge_complexity(values.len(), "reading an array")?;
                for value in values {
                    self.charge_value(value, depth + 1)?;
                }
                Ok(())
            }
            Value::Object(values) => {
                self.charge_complexity(values.len(), "reading an object")?;
                for (key, value) in values {
                    self.charge_complexity(key.len(), "reading an object key")?;
                    self.charge_value(value, depth + 1)?;
                }
                Ok(())
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
        }
    }

    fn process_context(
        &mut self,
        active: &mut ActiveContext,
        local: &Value,
        base_url: Option<&str>,
        remote: bool,
        depth: usize,
    ) -> Result<(), RdfDiagnostic> {
        self.process_context_inner(active, local, base_url, remote, depth, true)
    }

    fn process_context_inner(
        &mut self,
        active: &mut ActiveContext,
        local: &Value,
        base_url: Option<&str>,
        remote: bool,
        depth: usize,
        select_propagation: bool,
    ) -> Result<(), RdfDiagnostic> {
        if depth > self.limits.max_nesting() {
            return Err(context_limit(format!(
                "JSON-LD context processing exceeds depth {}",
                self.limits.max_nesting()
            )));
        }
        self.charge_work("processing a local context")?;
        if select_propagation {
            let propagate = match local {
                Value::Object(object) => match object.get("@propagate") {
                    None => true,
                    Some(Value::Bool(value)) => *value,
                    Some(_) => {
                        return Err(context_error("JSON-LD @propagate must be boolean"));
                    }
                },
                _ => true,
            };
            if !propagate && active.previous_context.is_none() {
                active.previous_context = Some(Arc::new(active.clone()));
            }
            active.propagate = propagate;
        }
        match local {
            Value::Null => self.reset_context(active),
            Value::Array(entries) => {
                for entry in entries {
                    self.process_context_inner(active, entry, base_url, remote, depth + 1, false)?;
                }
                Ok(())
            }
            Value::String(reference) => {
                let iri = resolve_reference(reference, base_url, "context IRI")?;
                self.process_remote_context(active, &iri, depth + 1)
            }
            Value::Object(object) => {
                self.process_context_object(active, object, base_url, remote, depth + 1)
            }
            Value::Bool(_) | Value::Number(_) => Err(context_error(
                "JSON-LD local context must be null, an object, an array, or an IRI",
            )),
        }
    }

    fn reset_context(&self, active: &mut ActiveContext) -> Result<(), RdfDiagnostic> {
        if !self.override_protected
            && let Some((term, _)) = active
                .terms
                .iter()
                .find(|(_, definition)| definition.protected)
        {
            return Err(context_error(format!(
                "null context would erase protected term `{term}`"
            )));
        }
        let previous_context = (!active.propagate)
            .then(|| active.previous_context.clone())
            .flatten();
        let propagate = active.propagate;
        *active = ActiveContext::new(self.initial_base.clone());
        if !propagate {
            active.propagate = false;
            active.previous_context = previous_context;
        }
        Ok(())
    }

    fn process_remote_context(
        &mut self,
        active: &mut ActiveContext,
        iri: &str,
        depth: usize,
    ) -> Result<(), RdfDiagnostic> {
        if self.remote_stack.iter().any(|entry| entry == iri) {
            return Err(context_error(format!(
                "offline context cycle: {} -> {iri}",
                self.remote_stack.join(" -> ")
            )));
        }
        let document = self.load_registry_document(iri, depth, "context")?;
        let context = document
            .as_object()
            .and_then(|object| object.get("@context"))
            .ok_or_else(|| {
                context_error(format!(
                    "offline context document `{iri}` must contain @context"
                ))
            })?
            .clone();
        self.remote_stack.push(iri.to_owned());
        let result = self.process_context(active, &context, Some(iri), true, depth + 1);
        self.remote_stack.pop();
        result
    }

    fn process_context_object(
        &mut self,
        active: &mut ActiveContext,
        object: &Map<String, Value>,
        base_url: Option<&str>,
        remote: bool,
        depth: usize,
    ) -> Result<(), RdfDiagnostic> {
        let mut local = if let Some(import) = object.get("@import") {
            let reference = import
                .as_str()
                .ok_or_else(|| context_error("JSON-LD @import must be an IRI string"))?;
            let iri = resolve_reference(reference, base_url, "@import IRI")?;
            self.load_import_object(&iri, depth + 1)?
        } else {
            Map::new()
        };
        for (key, value) in object {
            if key != "@import" {
                local.insert(key.clone(), value.clone());
            }
        }
        self.validate_context_object_keywords(&local)?;
        if let Some(version) = local.get("@version") {
            let valid = version.as_f64().is_some_and(|number| number == 1.1);
            if !valid {
                return Err(context_error("JSON-LD @version must be the number 1.1"));
            }
        }
        self.process_context_settings(active, &local, remote)?;
        self.define_local_terms(active, &local, base_url, depth + 1)
    }

    fn load_import_object(
        &mut self,
        iri: &str,
        depth: usize,
    ) -> Result<Map<String, Value>, RdfDiagnostic> {
        if self.remote_stack.iter().any(|entry| entry == iri) {
            return Err(context_error(format!(
                "offline @import cycle: {} -> {iri}",
                self.remote_stack.join(" -> ")
            )));
        }
        let document = self.load_registry_document(iri, depth, "@import")?;
        let imported = document
            .as_object()
            .and_then(|wrapper| wrapper.get("@context"))
            .and_then(Value::as_object)
            .ok_or_else(|| {
                context_error(format!(
                    "offline @import document `{iri}` must contain an object @context"
                ))
            })?
            .clone();
        if imported.contains_key("@import") {
            return Err(context_error(format!(
                "imported context `{iri}` must not contain another @import"
            )));
        }
        Ok(imported)
    }

    fn load_registry_document(
        &mut self,
        iri: &str,
        depth: usize,
        purpose: &str,
    ) -> Result<Arc<Value>, RdfDiagnostic> {
        if let Some(document) = self.remote_cache.get(iri) {
            return Ok(Arc::clone(document));
        }
        let bytes = self.registry.get(iri).ok_or_else(|| {
            context_error(format!(
                "offline context registry has no document for `{iri}` ({purpose})"
            ))
        })?;
        self.budget.loaded_bytes = self
            .budget
            .loaded_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| context_limit("loaded context byte counter overflow"))?;
        if self.budget.loaded_bytes > self.limits.max_registry_bytes() {
            return Err(context_limit(format!(
                "loaded offline context bytes exceed {}",
                self.limits.max_registry_bytes()
            )));
        }
        let document = super::parse_strict_json(
            bytes,
            self.limits.strict_json(),
            &format!("offline {purpose} document `{iri}`"),
        )?;
        self.charge_value(&document, depth)?;
        let document = Arc::new(document);
        self.remote_cache
            .insert(iri.to_owned(), Arc::clone(&document));
        Ok(document)
    }

    fn validate_context_object_keywords(
        &self,
        object: &Map<String, Value>,
    ) -> Result<(), RdfDiagnostic> {
        for key in object.keys().filter(|key| key.starts_with('@')) {
            if is_extension_control(key) {
                return Err(context_error(format!(
                    "caller contexts cannot redefine PurRDF control `{key}`"
                )));
            }
        }
        Ok(())
    }

    fn process_context_settings(
        &self,
        active: &mut ActiveContext,
        object: &Map<String, Value>,
        remote: bool,
    ) -> Result<(), RdfDiagnostic> {
        if let Some(value) = object.get("@propagate") {
            value
                .as_bool()
                .ok_or_else(|| context_error("JSON-LD @propagate must be boolean"))?;
        }
        if !remote && let Some(value) = object.get("@base") {
            active.base_iri = match value {
                Value::Null => None,
                Value::String(reference) => Some(resolve_reference(
                    reference,
                    active.base_iri.as_deref(),
                    "@base",
                )?),
                _ => return Err(context_error("JSON-LD @base must be a string or null")),
            };
        }
        if let Some(value) = object.get("@vocab") {
            active.vocab_mapping = match value {
                Value::Null => None,
                Value::String(reference) => {
                    if is_keyword_form(reference) {
                        return Err(context_error(format!(
                            "JSON-LD @vocab cannot use reserved keyword-form value `{reference}`"
                        )));
                    }
                    let expanded = if is_blank_node_identifier(reference) {
                        reference.clone()
                    } else {
                        resolve_reference(reference, active.base_iri.as_deref(), "@vocab")?
                    };
                    validate_iri_or_blank_node(&expanded, "@vocab mapping")?;
                    Some(expanded)
                }
                _ => return Err(context_error("JSON-LD @vocab must be a string or null")),
            };
        }
        if let Some(value) = object.get("@language") {
            active.default_language = match value {
                Value::Null => None,
                Value::String(language) => Some(language.to_ascii_lowercase()),
                _ => {
                    return Err(context_error("JSON-LD @language must be a string or null"));
                }
            };
        }
        if let Some(value) = object.get("@direction") {
            active.default_direction = match value {
                Value::Null => None,
                Value::String(direction) => {
                    Some(JsonLdDirection::parse(direction).ok_or_else(|| {
                        context_error("JSON-LD @direction must be `ltr`, `rtl`, or null")
                    })?)
                }
                _ => {
                    return Err(context_error(
                        "JSON-LD @direction must be `ltr`, `rtl`, or null",
                    ));
                }
            };
        }
        Ok(())
    }

    fn define_local_terms(
        &mut self,
        active: &mut ActiveContext,
        object: &Map<String, Value>,
        base_url: Option<&str>,
        depth: usize,
    ) -> Result<(), RdfDiagnostic> {
        let default_protected = match object.get("@protected") {
            None => false,
            Some(Value::Bool(value)) => *value,
            Some(_) => return Err(context_error("JSON-LD @protected must be boolean")),
        };
        let terms: Vec<String> = object
            .keys()
            .filter(|key| key.as_str() == "@type" || !CONTEXT_KEYWORDS.contains(&key.as_str()))
            .cloned()
            .collect();
        let mut states = BTreeMap::new();
        for term in &terms {
            self.define_term(
                active,
                object,
                term,
                default_protected,
                base_url,
                &mut states,
                depth,
            )?;
        }

        // Validate each term-scoped context against the fully defined active context.
        // The canonical local value remains attached to the term and is reused by the
        // typed carrier; this validation deliberately does not materialize an expanded
        // JSON tree.
        let scoped: Vec<(String, Value)> = terms
            .iter()
            .filter_map(|term| {
                active
                    .terms
                    .get(term)
                    .and_then(|definition| definition.scoped_context.clone())
                    .map(|context| (term.clone(), context))
            })
            .collect();
        for (term, context) in scoped {
            let mut scoped_active = active.clone();
            // Avoid recursively validating the same scope merely because the parent
            // active context contains its definition. Nested scoped definitions that
            // occur textually inside `context` are still parsed and validated.
            if let Some(definition) = scoped_active.terms.get_mut(&term) {
                definition.scoped_context = None;
            }
            let previous_override = self.override_protected;
            self.override_protected = true;
            let result =
                self.process_context(&mut scoped_active, &context, base_url, false, depth + 1);
            self.override_protected = previous_override;
            result?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn define_term(
        &mut self,
        active: &mut ActiveContext,
        local: &Map<String, Value>,
        term: &str,
        default_protected: bool,
        base_url: Option<&str>,
        states: &mut BTreeMap<String, DefinitionState>,
        depth: usize,
    ) -> Result<(), RdfDiagnostic> {
        match states.get(term) {
            Some(DefinitionState::Defined) => return Ok(()),
            Some(DefinitionState::Defining) => {
                return Err(context_error(format!(
                    "cyclic JSON-LD term definition involving `{term}`"
                )));
            }
            None => {}
        }
        if is_keyword_form(term) && !is_keyword(term) {
            states.insert(term.to_owned(), DefinitionState::Defined);
            return Ok(());
        }
        if depth > self.limits.max_nesting() {
            return Err(context_limit(format!(
                "JSON-LD term-definition recursion exceeds depth {}",
                self.limits.max_nesting()
            )));
        }
        validate_term_name(term)?;
        self.charge_work(&format!("defining term `{term}`"))?;
        if self.budget.terms == self.limits.max_terms() {
            return Err(context_limit(format!(
                "JSON-LD context exceeds {} term definitions",
                self.limits.max_terms()
            )));
        }
        self.budget.terms += 1;
        states.insert(term.to_owned(), DefinitionState::Defining);

        let value = local
            .get(term)
            .ok_or_else(|| context_error(format!("missing local definition for `{term}`")))?
            .clone();
        let old = active.terms.remove(term);
        let compiled = self.compile_term_definition(
            active,
            local,
            term,
            value,
            default_protected,
            base_url,
            states,
            depth + 1,
        );
        let mut definition = match compiled {
            Ok(definition) => definition,
            Err(error) => {
                if let Some(old) = old {
                    active.terms.insert(term.to_owned(), old);
                }
                states.remove(term);
                return Err(error);
            }
        };

        if let Some(old) = old
            && old.protected
            && !self.override_protected
        {
            if !definitions_equal_ignoring_protected(&old, &definition) {
                active.terms.insert(term.to_owned(), old);
                states.remove(term);
                return Err(context_error(format!(
                    "protected JSON-LD term `{term}` cannot be redefined"
                )));
            }
            definition = old;
        }
        active.terms.insert(term.to_owned(), definition);
        states.insert(term.to_owned(), DefinitionState::Defined);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn compile_term_definition(
        &mut self,
        active: &mut ActiveContext,
        local: &Map<String, Value>,
        term: &str,
        value: Value,
        default_protected: bool,
        base_url: Option<&str>,
        states: &mut BTreeMap<String, DefinitionState>,
        depth: usize,
    ) -> Result<JsonLdTermDefinition, RdfDiagnostic> {
        let simple_term = value.is_string();
        let object: Map<String, Value> = match value {
            Value::Null => BTreeMap::from([("@id".to_owned(), Value::Null)])
                .into_iter()
                .collect(),
            Value::String(id) => BTreeMap::from([("@id".to_owned(), Value::String(id))])
                .into_iter()
                .collect(),
            Value::Object(object) => object,
            _ => {
                return Err(context_error(format!(
                    "term `{term}` definition must be null, a string, or an object"
                )));
            }
        };
        if term == "@type" {
            return compile_type_keyword_definition(&object, default_protected);
        }
        for key in object.keys() {
            if !TERM_DEFINITION_KEYS.contains(&key.as_str()) {
                return Err(context_error(format!(
                    "term `{term}` has unsupported definition member `{key}`"
                )));
            }
        }

        let protected = match object.get("@protected") {
            None => default_protected,
            Some(Value::Bool(value)) => *value,
            Some(_) => {
                return Err(context_error(format!(
                    "term `{term}` @protected must be boolean"
                )));
            }
        };

        let type_mapping = object
            .get("@type")
            .map(|value| {
                self.compile_type_mapping(
                    active,
                    local,
                    term,
                    value,
                    base_url,
                    default_protected,
                    states,
                    depth,
                )
            })
            .transpose()?;

        let reverse_property = object.contains_key("@reverse");
        if reverse_property {
            if object.contains_key("@id") || object.contains_key("@nest") {
                return Err(context_error(format!(
                    "reverse term `{term}` cannot define @id or @nest"
                )));
            }
            let reverse = object
                .get("@reverse")
                .expect("reverse member was checked as present");
            let reverse = reverse
                .as_str()
                .ok_or_else(|| context_error(format!("term `{term}` @reverse must be a string")))?;
            if is_keyword_form(reverse) && !is_keyword(reverse) {
                return Ok(empty_definition(protected));
            }
            let expanded = self.expand_local_iri(
                active,
                local,
                reverse,
                true,
                false,
                base_url,
                default_protected,
                states,
                depth,
            )?;
            validate_property_mapping(term, &expanded)?;
            let containers = object
                .get("@container")
                .map(|value| compile_reverse_containers(term, value))
                .transpose()?
                .unwrap_or_default();
            return Ok(JsonLdTermDefinition {
                iri_mapping: Some(expanded),
                reverse_property: true,
                prefix: false,
                protected,
                type_mapping,
                language_mapping: None,
                direction_mapping: None,
                containers,
                index_mapping: None,
                nest: None,
                scoped_context: None,
                scoped_context_base: None,
            });
        }

        let iri_mapping = if let Some(id) = object.get("@id") {
            match id {
                Value::Null => None,
                Value::String(id) if is_keyword_form(id) && !is_keyword(id) => None,
                Value::String(id) if id == term => Some(self.derive_term_mapping(
                    active,
                    local,
                    term,
                    base_url,
                    default_protected,
                    states,
                    depth,
                )?),
                Value::String(id) => {
                    let expanded = self.expand_local_iri(
                        active,
                        local,
                        id,
                        true,
                        false,
                        base_url,
                        default_protected,
                        states,
                        depth,
                    )?;
                    validate_term_mapping(term, &expanded)?;
                    if term_has_mapping_syntax(term) {
                        let term_mapping = self.derive_term_mapping(
                            active,
                            local,
                            term,
                            base_url,
                            default_protected,
                            states,
                            depth,
                        )?;
                        if term_mapping != expanded {
                            return Err(context_error(format!(
                                "term `{term}` @id expands to `{expanded}`, but the term itself expands to `{term_mapping}`"
                            )));
                        }
                    }
                    Some(expanded)
                }
                _ => {
                    return Err(context_error(format!(
                        "term `{term}` @id must be a string or null"
                    )));
                }
            }
        } else {
            Some(self.derive_term_mapping(
                active,
                local,
                term,
                base_url,
                default_protected,
                states,
                depth,
            )?)
        };

        let prefix = match object.get("@prefix") {
            None => simple_term && is_implicit_prefix(term, iri_mapping.as_deref()),
            Some(Value::Bool(value)) => *value,
            Some(_) => {
                return Err(context_error(format!(
                    "term `{term}` @prefix must be boolean"
                )));
            }
        };
        if object.contains_key("@prefix") && term.contains([':', '/']) {
            return Err(context_error(format!(
                "term `{term}` with @prefix must not contain `:` or `/`"
            )));
        }
        if prefix {
            let mapping = iri_mapping
                .as_deref()
                .ok_or_else(|| context_error(format!("prefix term `{term}` has a null mapping")))?;
            if is_keyword(mapping) || is_extension_control(mapping) {
                return Err(context_error(format!(
                    "prefix term `{term}` cannot map to `{mapping}`"
                )));
            }
            validate_iri_or_blank_node(mapping, &format!("prefix mapping for `{term}`"))?;
        }

        let containers = object
            .get("@container")
            .map(|value| compile_containers(term, value))
            .transpose()?
            .unwrap_or_default();
        validate_container_combination(term, &containers, false)?;
        let type_mapping = if containers.contains(&JsonLdContainer::Type) {
            match type_mapping {
                None => Some(JsonLdTypeMapping::Id),
                Some(JsonLdTypeMapping::Id | JsonLdTypeMapping::Vocab) => type_mapping,
                Some(_) => {
                    return Err(context_error(format!(
                        "term `{term}` with an @type container requires @type coercion @id or @vocab"
                    )));
                }
            }
        } else {
            type_mapping
        };

        let language_mapping = if type_mapping.is_none() {
            object
                .get("@language")
                .map(|value| compile_language_mapping(term, value))
                .transpose()?
        } else {
            None
        };
        let direction_mapping = if type_mapping.is_none() {
            object
                .get("@direction")
                .map(|value| compile_direction_mapping(term, value))
                .transpose()?
        } else {
            None
        };

        let index_mapping = object
            .get("@index")
            .map(|value| {
                if !containers.contains(&JsonLdContainer::Index) {
                    return Err(context_error(format!(
                        "term `{term}` @index requires an @index container"
                    )));
                }
                let index = value.as_str().ok_or_else(|| {
                    context_error(format!("term `{term}` @index must be a string"))
                })?;
                let expanded = self.expand_local_iri(
                    active,
                    local,
                    index,
                    true,
                    false,
                    base_url,
                    default_protected,
                    states,
                    depth,
                )?;
                validate_absolute_iri(&expanded, &format!("index mapping for term `{term}`"))?;
                Ok(expanded)
            })
            .transpose()?;

        let nest = object
            .get("@nest")
            .map(|value| {
                let nest = value.as_str().ok_or_else(|| {
                    context_error(format!("term `{term}` @nest must be a string"))
                })?;
                if is_keyword(nest) && nest != "@nest" {
                    return Err(context_error(format!(
                        "term `{term}` @nest cannot use keyword `{nest}`"
                    )));
                }
                Ok(nest.to_owned())
            })
            .transpose()?;

        let scoped_context = object.get("@context").map(canonicalize);
        let scoped_context_base = scoped_context
            .as_ref()
            .and_then(|_| base_url.map(str::to_owned));

        Ok(JsonLdTermDefinition {
            iri_mapping,
            reverse_property,
            prefix,
            protected,
            type_mapping,
            language_mapping,
            direction_mapping,
            containers,
            index_mapping,
            nest,
            scoped_context,
            scoped_context_base,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn derive_term_mapping(
        &mut self,
        active: &mut ActiveContext,
        local: &Map<String, Value>,
        term: &str,
        base_url: Option<&str>,
        default_protected: bool,
        states: &mut BTreeMap<String, DefinitionState>,
        depth: usize,
    ) -> Result<String, RdfDiagnostic> {
        if is_blank_node_identifier(term) {
            return Ok(term.to_owned());
        }
        if let Some((prefix, suffix)) = term.split_once(':')
            && !prefix.is_empty()
        {
            if local.contains_key(prefix) {
                self.define_term(
                    active,
                    local,
                    prefix,
                    default_protected,
                    base_url,
                    states,
                    depth + 1,
                )?;
            }
            if let Some(mapping) = active
                .terms
                .get(prefix)
                .and_then(|definition| definition.iri_mapping.as_deref())
            {
                let expanded = format!("{mapping}{suffix}");
                validate_iri_or_blank_node(&expanded, &format!("mapping for term `{term}`"))?;
                return Ok(expanded);
            }
            if let Ok(parsed) = purrdf_iri::parse(term)
                && parsed.has_scheme()
            {
                return Ok(term.to_owned());
            }
            return Err(context_error(format!(
                "term `{term}` uses an undefined compact-IRI prefix `{prefix}`"
            )));
        }
        if term.contains('/') {
            let expanded = resolve_reference(
                term,
                active.base_iri.as_deref(),
                &format!("mapping for term `{term}`"),
            )?;
            validate_absolute_iri(&expanded, &format!("mapping for term `{term}`"))?;
            return Ok(expanded);
        }
        if let Some(vocab) = &active.vocab_mapping {
            let expanded = format!("{vocab}{term}");
            validate_iri_or_blank_node(&expanded, &format!("mapping for term `{term}`"))?;
            return Ok(expanded);
        }
        Err(context_error(format!(
            "term `{term}` needs @id, a compact-IRI prefix, or an active @vocab"
        )))
    }

    #[allow(clippy::too_many_arguments)]
    fn expand_local_iri(
        &mut self,
        active: &mut ActiveContext,
        local: &Map<String, Value>,
        value: &str,
        vocab: bool,
        document_relative: bool,
        base_url: Option<&str>,
        default_protected: bool,
        states: &mut BTreeMap<String, DefinitionState>,
        depth: usize,
    ) -> Result<String, RdfDiagnostic> {
        self.charge_work(&format!("expanding `{value}`"))?;
        if is_keyword(value) {
            return Ok(value.to_owned());
        }
        if is_extension_control(value) {
            return Err(context_error(format!(
                "caller contexts cannot alias PurRDF control `{value}`"
            )));
        }
        if is_keyword_form(value) {
            return Err(context_error(format!(
                "reserved keyword-form value `{value}` does not expand to an IRI"
            )));
        }
        if vocab && local.contains_key(value) {
            self.define_term(
                active,
                local,
                value,
                default_protected,
                base_url,
                states,
                depth + 1,
            )?;
        }
        if vocab && let Some(definition) = active.terms.get(value) {
            return definition
                .iri_mapping
                .clone()
                .ok_or_else(|| context_error(format!("term `{value}` has a null IRI mapping")));
        }
        if let Some(definition) = active.terms.get(value)
            && definition.iri_mapping.as_deref().is_some_and(is_keyword)
        {
            return definition
                .iri_mapping
                .clone()
                .ok_or_else(|| context_error(format!("term `{value}` has a null IRI mapping")));
        }
        if let Some((prefix, suffix)) = value.split_once(':')
            && !prefix.is_empty()
        {
            if prefix == "_" || suffix.starts_with("//") {
                return Ok(value.to_owned());
            }
            if local.contains_key(prefix) {
                self.define_term(
                    active,
                    local,
                    prefix,
                    default_protected,
                    base_url,
                    states,
                    depth + 1,
                )?;
            }
            if let Some(mapping) = active
                .terms
                .get(prefix)
                .filter(|definition| definition.prefix)
                .and_then(|definition| definition.iri_mapping.as_deref())
            {
                let expanded = format!("{mapping}{suffix}");
                validate_iri_or_blank_node(&expanded, "expanded compact IRI")?;
                return Ok(expanded);
            }
            if let Ok(parsed) = purrdf_iri::parse(value)
                && parsed.has_scheme()
            {
                return Ok(value.to_owned());
            }
            return Err(context_error(format!(
                "compact IRI `{value}` uses undefined prefix `{prefix}`"
            )));
        }
        if vocab && let Some(vocab_mapping) = &active.vocab_mapping {
            return Ok(format!("{vocab_mapping}{value}"));
        }
        if document_relative {
            return resolve_reference(value, active.base_iri.as_deref(), "IRI");
        }
        Err(context_error(format!(
            "relative IRI `{value}` has no applicable @vocab or @base"
        )))
    }

    #[allow(clippy::too_many_arguments)]
    fn compile_type_mapping(
        &mut self,
        active: &mut ActiveContext,
        local: &Map<String, Value>,
        term: &str,
        value: &Value,
        base_url: Option<&str>,
        default_protected: bool,
        states: &mut BTreeMap<String, DefinitionState>,
        depth: usize,
    ) -> Result<JsonLdTypeMapping, RdfDiagnostic> {
        let value = value
            .as_str()
            .ok_or_else(|| context_error(format!("term `{term}` @type must be a string")))?;
        match value {
            "@id" => Ok(JsonLdTypeMapping::Id),
            "@vocab" => Ok(JsonLdTypeMapping::Vocab),
            "@json" => Ok(JsonLdTypeMapping::Json),
            "@none" => Ok(JsonLdTypeMapping::None),
            _ => {
                let expanded = self.expand_local_iri(
                    active,
                    local,
                    value,
                    true,
                    false,
                    base_url,
                    default_protected,
                    states,
                    depth,
                )?;
                if is_keyword(&expanded) || is_extension_control(&expanded) {
                    return Err(context_error(format!(
                        "term `{term}` has invalid @type mapping `{expanded}`"
                    )));
                }
                validate_absolute_iri(&expanded, &format!("@type mapping for `{term}`"))?;
                Ok(JsonLdTypeMapping::Datatype(expanded))
            }
        }
    }
}

fn empty_definition(protected: bool) -> JsonLdTermDefinition {
    JsonLdTermDefinition {
        iri_mapping: None,
        reverse_property: false,
        prefix: false,
        protected,
        type_mapping: None,
        language_mapping: None,
        direction_mapping: None,
        containers: BTreeSet::new(),
        index_mapping: None,
        nest: None,
        scoped_context: None,
        scoped_context_base: None,
    }
}

fn compile_type_keyword_definition(
    object: &Map<String, Value>,
    default_protected: bool,
) -> Result<JsonLdTermDefinition, RdfDiagnostic> {
    for key in object.keys() {
        if !matches!(key.as_str(), "@container" | "@protected") {
            return Err(context_error(format!(
                "JSON-LD @type definition cannot contain `{key}`"
            )));
        }
    }
    let protected = match object.get("@protected") {
        None => default_protected,
        Some(Value::Bool(value)) => *value,
        Some(_) => {
            return Err(context_error(
                "JSON-LD @type definition @protected must be boolean",
            ));
        }
    };
    let containers = match object.get("@container") {
        None => BTreeSet::new(),
        Some(Value::String(value)) if value == "@set" => BTreeSet::from([JsonLdContainer::Set]),
        Some(_) => {
            return Err(context_error(
                "JSON-LD @type definition @container must be @set",
            ));
        }
    };
    let mut definition = empty_definition(protected);
    definition.iri_mapping = Some("@type".to_owned());
    definition.containers = containers;
    Ok(definition)
}

fn compile_reverse_containers(
    term: &str,
    value: &Value,
) -> Result<BTreeSet<JsonLdContainer>, RdfDiagnostic> {
    match value {
        Value::Null => Ok(BTreeSet::new()),
        Value::String(value) if value == "@set" => Ok(BTreeSet::from([JsonLdContainer::Set])),
        Value::String(value) if value == "@index" => Ok(BTreeSet::from([JsonLdContainer::Index])),
        _ => Err(context_error(format!(
            "reverse term `{term}` @container must be @set, @index, or null"
        ))),
    }
}

fn definitions_equal_ignoring_protected(
    left: &JsonLdTermDefinition,
    right: &JsonLdTermDefinition,
) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.protected = false;
    right.protected = false;
    left == right
}

fn term_has_mapping_syntax(term: &str) -> bool {
    let colon = term
        .find(':')
        .is_some_and(|position| position > 0 && position + 1 < term.len());
    colon || term.contains('/')
}

fn is_keyword_form(value: &str) -> bool {
    value.strip_prefix('@').is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_alphabetic())
    })
}

fn is_blank_node_identifier(value: &str) -> bool {
    value
        .strip_prefix("_:")
        .is_some_and(|label| !label.is_empty() && !label.chars().any(char::is_whitespace))
}

fn validate_iri_or_blank_node(value: &str, description: &str) -> Result<(), RdfDiagnostic> {
    if is_blank_node_identifier(value) {
        Ok(())
    } else {
        validate_absolute_iri(value, description)
    }
}

fn validate_term_name(term: &str) -> Result<(), RdfDiagnostic> {
    if term.is_empty() {
        return Err(context_error("JSON-LD term must not be empty"));
    }
    if is_keyword(term) && term != "@type" {
        return Err(context_error(format!(
            "JSON-LD keyword `{term}` cannot be redefined"
        )));
    }
    if is_extension_control(term) {
        return Err(context_error(format!("invalid JSON-LD term `{term}`")));
    }
    Ok(())
}

fn validate_term_mapping(term: &str, mapping: &str) -> Result<(), RdfDiagnostic> {
    if mapping == "@context" || is_extension_control(mapping) {
        return Err(context_error(format!(
            "term `{term}` cannot alias reserved control `{mapping}`"
        )));
    }
    if is_keyword(mapping) {
        return Ok(());
    }
    validate_iri_or_blank_node(mapping, &format!("IRI mapping for term `{term}`"))
}

fn validate_property_mapping(term: &str, mapping: &str) -> Result<(), RdfDiagnostic> {
    if is_keyword(mapping) || is_extension_control(mapping) {
        return Err(context_error(format!(
            "property term `{term}` cannot map to keyword `{mapping}`"
        )));
    }
    validate_iri_or_blank_node(mapping, &format!("property mapping for term `{term}`"))
}

fn is_implicit_prefix(term: &str, mapping: Option<&str>) -> bool {
    if term.contains([':', '/']) {
        return false;
    }
    mapping.is_some_and(|iri| {
        is_blank_node_identifier(iri)
            || iri.ends_with('/')
            || iri.ends_with('#')
            || iri.ends_with(':')
            || iri.ends_with('?')
            || iri.ends_with('[')
            || iri.ends_with(']')
            || iri.ends_with('@')
    })
}

fn compile_language_mapping(
    term: &str,
    value: &Value,
) -> Result<JsonLdNullable<String>, RdfDiagnostic> {
    match value {
        Value::Null => Ok(JsonLdNullable::Null),
        Value::String(language) => Ok(JsonLdNullable::Value(language.to_ascii_lowercase())),
        _ => Err(context_error(format!(
            "term `{term}` @language must be a string or null"
        ))),
    }
}

fn compile_direction_mapping(
    term: &str,
    value: &Value,
) -> Result<JsonLdNullable<JsonLdDirection>, RdfDiagnostic> {
    match value {
        Value::Null => Ok(JsonLdNullable::Null),
        Value::String(direction) => JsonLdDirection::parse(direction)
            .map(JsonLdNullable::Value)
            .ok_or_else(|| {
                context_error(format!(
                    "term `{term}` @direction must be `ltr`, `rtl`, or null"
                ))
            }),
        _ => Err(context_error(format!(
            "term `{term}` @direction must be `ltr`, `rtl`, or null"
        ))),
    }
}

fn compile_containers(
    term: &str,
    value: &Value,
) -> Result<BTreeSet<JsonLdContainer>, RdfDiagnostic> {
    let entries: Vec<&str> = match value {
        Value::String(entry) => vec![entry],
        Value::Array(entries) if !entries.is_empty() => entries
            .iter()
            .map(|entry| {
                entry.as_str().ok_or_else(|| {
                    context_error(format!(
                        "term `{term}` @container array members must be strings"
                    ))
                })
            })
            .collect::<Result<_, _>>()?,
        Value::Array(_) => {
            return Err(context_error(format!(
                "term `{term}` @container array must not be empty"
            )));
        }
        _ => {
            return Err(context_error(format!(
                "term `{term}` @container must be a string or non-empty array"
            )));
        }
    };
    let mut containers = BTreeSet::new();
    for entry in entries {
        let container = JsonLdContainer::parse(entry).ok_or_else(|| {
            context_error(format!(
                "term `{term}` has unsupported @container value `{entry}`"
            ))
        })?;
        if !containers.insert(container) {
            return Err(context_error(format!(
                "term `{term}` repeats @container value `{entry}`"
            )));
        }
    }
    Ok(containers)
}

fn validate_container_combination(
    term: &str,
    containers: &BTreeSet<JsonLdContainer>,
    reverse: bool,
) -> Result<(), RdfDiagnostic> {
    use JsonLdContainer::{Graph, Id, Index, Language, List, Set, Type};
    if reverse && containers.iter().any(|entry| !matches!(entry, Set | Index)) {
        return Err(context_error(format!(
            "reverse term `{term}` may only use @set or @index containers"
        )));
    }
    let valid = match containers.len() {
        0 | 1 => true,
        2 => {
            containers == &BTreeSet::from([Set, Index])
                || containers == &BTreeSet::from([Set, Id])
                || containers == &BTreeSet::from([Set, Type])
                || containers == &BTreeSet::from([Set, Language])
                || containers == &BTreeSet::from([Set, Graph])
                || containers == &BTreeSet::from([Graph, Id])
                || containers == &BTreeSet::from([Graph, Index])
        }
        3 => {
            containers == &BTreeSet::from([Set, Graph, Id])
                || containers == &BTreeSet::from([Set, Graph, Index])
        }
        _ => false,
    };
    if !valid || (containers.contains(&List) && containers.len() != 1) {
        return Err(context_error(format!(
            "term `{term}` has an illegal @container combination"
        )));
    }
    Ok(())
}

fn is_keyword(value: &str) -> bool {
    KEYWORDS.contains(&value)
}

fn is_extension_control(value: &str) -> bool {
    EXTENSION_CONTROLS.contains(&value)
}

fn resolve_reference(
    reference: &str,
    base: Option<&str>,
    description: &str,
) -> Result<String, RdfDiagnostic> {
    if let Ok(parsed) = purrdf_iri::parse(reference)
        && parsed.has_scheme()
    {
        return Ok(reference.to_owned());
    }
    let base = base.ok_or_else(|| {
        context_error(format!(
            "relative {description} `{reference}` requires an absolute base IRI"
        ))
    })?;
    let base = purrdf_iri::parse(base)
        .map_err(|source| context_error(format!("invalid base IRI `{base}`: {source}")))?;
    base.resolve(reference)
        .map(|resolved| resolved.as_str().to_owned())
        .map_err(|source| {
            context_error(format!(
                "cannot resolve {description} `{reference}` against `{base}`: {source}"
            ))
        })
}

fn build_inverse_context(active: &ActiveContext) -> InverseContext {
    let mut inverse = InverseContext::new();
    let mut terms: Vec<(&String, &JsonLdTermDefinition)> = active.terms.iter().collect();
    terms.sort_by(|(left, _), (right, _)| {
        left.chars()
            .count()
            .cmp(&right.chars().count())
            .then_with(|| left.cmp(right))
    });
    for (term, definition) in terms {
        let Some(iri) = definition.iri_mapping.as_ref() else {
            continue;
        };
        let container = container_key(&definition.containers);
        let selection = inverse
            .entry(iri.clone())
            .or_default()
            .entry(container)
            .or_default();
        selection
            .fallback
            .entry("@none".to_owned())
            .or_insert_with(|| term.clone());

        if definition.reverse_property {
            selection
                .types
                .entry("@reverse".to_owned())
                .or_insert_with(|| term.clone());
            continue;
        }
        if definition.type_mapping == Some(JsonLdTypeMapping::None) {
            selection
                .languages
                .entry("@any".to_owned())
                .or_insert_with(|| term.clone());
            selection
                .types
                .entry("@any".to_owned())
                .or_insert_with(|| term.clone());
            continue;
        }
        if let Some(mapping) = &definition.type_mapping {
            selection
                .types
                .entry(mapping.inverse_key().to_owned())
                .or_insert_with(|| term.clone());
            continue;
        }

        match (
            definition.language_mapping.as_ref(),
            definition.direction_mapping.as_ref(),
        ) {
            (Some(language), Some(direction)) => {
                let key = explicit_language_direction_key(language, direction);
                selection
                    .languages
                    .entry(key)
                    .or_insert_with(|| term.clone());
            }
            (Some(language), None) => {
                let key = match language {
                    JsonLdNullable::Null => "@null".to_owned(),
                    JsonLdNullable::Value(language) => language.to_ascii_lowercase(),
                };
                selection
                    .languages
                    .entry(key)
                    .or_insert_with(|| term.clone());
            }
            (None, Some(direction)) => {
                let key = match direction {
                    JsonLdNullable::Null => "@none".to_owned(),
                    JsonLdNullable::Value(direction) => format!("_{}", direction.as_str()),
                };
                selection
                    .languages
                    .entry(key)
                    .or_insert_with(|| term.clone());
            }
            (None, None) => {
                let default_language = active.default_language.as_deref().unwrap_or("@none");
                if let Some(direction) = active.default_direction {
                    selection
                        .languages
                        .entry(format!("{default_language}_{}", direction.as_str()))
                        .or_insert_with(|| term.clone());
                } else {
                    selection
                        .languages
                        .entry(default_language.to_ascii_lowercase())
                        .or_insert_with(|| term.clone());
                }
                selection
                    .languages
                    .entry("@none".to_owned())
                    .or_insert_with(|| term.clone());
                selection
                    .types
                    .entry("@none".to_owned())
                    .or_insert_with(|| term.clone());
            }
        }
    }
    inverse
}

fn container_key(containers: &BTreeSet<JsonLdContainer>) -> String {
    if containers.is_empty() {
        return "@none".to_owned();
    }
    let mut values: Vec<&str> = containers.iter().map(|entry| entry.as_str()).collect();
    values.sort_unstable();
    values.concat()
}

fn explicit_language_direction_key(
    language: &JsonLdNullable<String>,
    direction: &JsonLdNullable<JsonLdDirection>,
) -> String {
    match (language, direction) {
        (JsonLdNullable::Value(language), JsonLdNullable::Value(direction)) => {
            format!("{}_{}", language.to_ascii_lowercase(), direction.as_str())
        }
        (JsonLdNullable::Value(language), JsonLdNullable::Null) => language.to_ascii_lowercase(),
        (JsonLdNullable::Null, JsonLdNullable::Value(direction)) => {
            format!("_{}", direction.as_str())
        }
        (JsonLdNullable::Null, JsonLdNullable::Null) => "@null".to_owned(),
    }
}

pub(super) fn expand_iri(
    active: &ActiveContext,
    value: &str,
    vocab: bool,
    document_relative: bool,
) -> Result<Option<String>, RdfDiagnostic> {
    if is_keyword(value) || is_extension_control(value) {
        return Ok(Some(value.to_owned()));
    }
    if is_keyword_form(value) {
        return Ok(None);
    }
    if let Some(definition) = active.terms.get(value)
        && definition.iri_mapping.as_deref().is_some_and(is_keyword)
    {
        return Ok(definition.iri_mapping.clone());
    }
    if vocab && let Some(definition) = active.terms.get(value) {
        return Ok(definition.iri_mapping.clone());
    }
    if let Some((prefix, suffix)) = value.split_once(':')
        && !prefix.is_empty()
    {
        if prefix == "_" || suffix.starts_with("//") {
            return Ok(Some(value.to_owned()));
        }
        if let Some(mapping) = active
            .terms
            .get(prefix)
            .filter(|definition| definition.prefix)
            .and_then(|definition| definition.iri_mapping.as_deref())
        {
            let expanded = format!("{mapping}{suffix}");
            validate_iri_or_blank_node(&expanded, "expanded compact IRI")?;
            return Ok(Some(expanded));
        }
        if let Ok(parsed) = purrdf_iri::parse(value)
            && parsed.has_scheme()
        {
            return Ok(Some(value.to_owned()));
        }
        return Err(context_error(format!(
            "compact IRI `{value}` uses an undefined prefix `{prefix}`"
        )));
    }
    if vocab && let Some(mapping) = &active.vocab_mapping {
        return Ok(Some(format!("{mapping}{value}")));
    }
    if document_relative {
        return resolve_reference(value, active.base_iri.as_deref(), "IRI").map(Some);
    }
    Err(context_error(format!(
        "relative IRI `{value}` has no applicable @vocab or @base"
    )))
}

pub(super) fn compact_iri(
    active: &ActiveContext,
    inverse: &InverseContext,
    iri: &str,
    vocab: bool,
    selection: Option<&JsonLdTermSelection>,
) -> Result<String, RdfDiagnostic> {
    if is_extension_control(iri) {
        return Ok(iri.to_owned());
    }
    if is_blank_node_identifier(iri) {
        return Ok(iri.to_owned());
    }
    if !is_keyword(iri) {
        validate_absolute_iri(iri, "IRI to compact")?;
    }
    if vocab {
        if let Some(term) = select_inverse_term(inverse, iri, selection) {
            return Ok(term.to_owned());
        }
        if let Some(vocab_mapping) = &active.vocab_mapping
            && let Some(suffix) = iri.strip_prefix(vocab_mapping)
            && !suffix.is_empty()
            && !suffix.contains(':')
            && !active.terms.contains_key(suffix)
        {
            return Ok(suffix.to_owned());
        }
    }
    if let Some(candidate) = compact_with_prefix(active, iri, selection.is_none()) {
        return Ok(candidate);
    }
    reject_iri_confused_with_prefix(active, iri)?;
    if vocab {
        return Ok(iri.to_owned());
    }
    Ok(active
        .base_iri
        .as_deref()
        .and_then(|base| remove_base(base, iri))
        .unwrap_or_else(|| iri.to_owned()))
}

fn select_inverse_term<'a>(
    inverse: &'a InverseContext,
    iri: &str,
    selection: Option<&JsonLdTermSelection>,
) -> Option<&'a str> {
    let containers = selection.map_or_else(
        || vec!["@none".to_owned()],
        |selection| selection.containers.clone(),
    );
    let kind = selection.map_or(JsonLdTermSelectionKind::Any, |selection| selection.kind);
    let preferred = selection.map_or_else(
        || vec!["@none".to_owned()],
        |selection| selection.preferred_values.clone(),
    );
    let by_container = inverse.get(iri)?;
    for container in containers {
        let Some(candidate) = by_container.get(&container) else {
            continue;
        };
        if kind == JsonLdTermSelectionKind::Any {
            if let Some(term) = candidate.fallback.get("@none") {
                return Some(term);
            }
            continue;
        }
        let table = match kind {
            JsonLdTermSelectionKind::Type => &candidate.types,
            JsonLdTermSelectionKind::Language => &candidate.languages,
            JsonLdTermSelectionKind::Any => unreachable!("handled above"),
        };
        for value in &preferred {
            if let Some(term) = table.get(value) {
                return Some(term);
            }
        }
    }
    None
}

fn compact_with_prefix(active: &ActiveContext, iri: &str, value_is_null: bool) -> Option<String> {
    let mut candidates = Vec::new();
    for (term, definition) in &active.terms {
        if !definition.prefix {
            continue;
        }
        let Some(mapping) = definition.iri_mapping.as_deref() else {
            continue;
        };
        let Some(suffix) = iri.strip_prefix(mapping) else {
            continue;
        };
        if suffix.is_empty() || suffix.starts_with("//") || suffix.chars().any(char::is_whitespace)
        {
            continue;
        }
        let candidate = format!("{term}:{suffix}");
        if let Some(definition) = active.terms.get(&candidate)
            && (!value_is_null || definition.iri_mapping.as_deref() != Some(iri))
        {
            continue;
        }
        candidates.push(candidate);
    }
    candidates.sort_by(|left, right| {
        left.chars()
            .count()
            .cmp(&right.chars().count())
            .then_with(|| left.cmp(right))
    });
    candidates.into_iter().next()
}

fn reject_iri_confused_with_prefix(active: &ActiveContext, iri: &str) -> Result<(), RdfDiagnostic> {
    let parsed = purrdf_iri::parse(iri).map_err(|source| {
        context_error(format!("parse IRI `{iri}` during compaction: {source}"))
    })?;
    let Some(scheme) = parsed.scheme() else {
        return Ok(());
    };
    if parsed.authority().is_none()
        && active
            .terms
            .get(scheme)
            .is_some_and(|definition| definition.prefix)
    {
        return Err(context_error(format!(
            "IRI `{iri}` is confused with active JSON-LD prefix `{scheme}`"
        )));
    }
    Ok(())
}

fn remove_base(base: &str, iri: &str) -> Option<String> {
    let base_iri = purrdf_iri::parse(base).ok()?;
    let target = purrdf_iri::parse(iri).ok()?;
    if base_iri.scheme() != target.scheme() || base_iri.authority() != target.authority() {
        return None;
    }

    let mut candidates = Vec::new();
    if base == iri {
        candidates.push(String::new());
    }
    if base_iri.path() == target.path()
        && base_iri.query() == target.query()
        && let Some(fragment) = target.fragment()
    {
        candidates.push(format!("#{fragment}"));
    }
    if base_iri.path() == target.path()
        && let Some(query) = target.query()
    {
        let mut candidate = format!("?{query}");
        if let Some(fragment) = target.fragment() {
            candidate.push('#');
            candidate.push_str(fragment);
        }
        candidates.push(candidate);
    }

    let mut absolute_path = target.path().to_owned();
    if let Some(query) = target.query() {
        absolute_path.push('?');
        absolute_path.push_str(query);
    }
    if let Some(fragment) = target.fragment() {
        absolute_path.push('#');
        absolute_path.push_str(fragment);
    }
    candidates.push(absolute_path);

    let base_directory = base_iri
        .path()
        .rsplit_once('/')
        .map_or("", |(directory, _)| directory);
    let base_segments: Vec<&str> = base_directory
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let target_segments: Vec<&str> = target
        .path()
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let shared = base_segments
        .iter()
        .zip(&target_segments)
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative = "../".repeat(base_segments.len().saturating_sub(shared));
    relative.push_str(&target_segments[shared..].join("/"));
    if relative.is_empty() {
        relative.push_str("./");
    }
    if let Some(query) = target.query() {
        relative.push('?');
        relative.push_str(query);
    }
    if let Some(fragment) = target.fragment() {
        relative.push('#');
        relative.push_str(fragment);
    }
    candidates.push(relative);

    candidates.retain(|candidate| {
        base_iri
            .resolve(candidate)
            .is_ok_and(|resolved| resolved.as_str() == iri)
    });
    candidates.sort_by(|left, right| {
        left.len()
            .cmp(&right.len())
            .then_with(|| lexical_relative_preference(left, right))
    });
    candidates.into_iter().next()
}

fn lexical_relative_preference(left: &str, right: &str) -> Ordering {
    left.cmp(right)
}
