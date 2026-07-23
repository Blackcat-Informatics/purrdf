// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Context-aware expansion from compact JSON-LD-star into the typed carrier.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::{Map, Value as JsonValue};

use super::carrier::{Document, Literal, NamedGraph, Node, Term, Triple, Value};
use super::{
    CompiledJsonLdContext, JsonLdContainer, JsonLdDirection, JsonLdNullable, JsonLdTermDefinition,
    JsonLdTypeMapping, RDF_FIRST, RDF_JSON, RDF_NIL, RDF_REIFIES, RDF_REST, RDF_TYPE, RdfDataset,
    RdfDiagnostic, RdfQuad, RdfTerm, XSD_STRING, decode, parse, validated_iri_term,
};
use crate::{RdfLiteral, RdfTextDirection, RdfTriple};

const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

pub(super) fn expand_document(
    document: &JsonValue,
    context: &CompiledJsonLdContext,
) -> Result<Document, RdfDiagnostic> {
    let mut builder = Builder::new(document);
    match document {
        JsonValue::Array(entries) => {
            for entry in entries {
                builder.expand_graph_entry(entry, None, context)?;
            }
        }
        JsonValue::Object(_) => builder.expand_graph_entry(document, None, context)?,
        _ => {
            return Err(decode(
                "JSON-LD document must be an object or array of objects",
            ));
        }
    }
    Ok(builder.finish())
}

pub(super) fn carrier_to_dataset(document: &Document) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let mut lowerer = Lowerer::new(document);
    for node in &document.default_nodes {
        lowerer.lower_node(node, None)?;
    }
    for graph in &document.named_graphs {
        let graph_name = id_term(&graph.id)?;
        for node in &graph.nodes {
            lowerer.lower_node(node, Some(&graph_name))?;
        }
    }
    crate::dataset_from_quads(&lowerer.quads)
        .map_err(|source| parse(format!("freeze JSON-LD-star quads: {source}")))
}

#[derive(Debug, Clone, Copy)]
enum NodeDisposition {
    Merge,
    Detached,
}

struct Builder {
    graphs: BTreeMap<Option<String>, BTreeMap<String, Node>>,
    reserved_blank_nodes: BTreeSet<String>,
    next_blank_node: u64,
}

impl Builder {
    fn new(document: &JsonValue) -> Self {
        let mut reserved_blank_nodes = BTreeSet::new();
        collect_blank_node_ids(document, &mut reserved_blank_nodes);
        Self {
            graphs: BTreeMap::new(),
            reserved_blank_nodes,
            next_blank_node: 0,
        }
    }

    fn finish(mut self) -> Document {
        for nodes in self.graphs.values_mut() {
            for node in nodes.values_mut() {
                node.sort_values();
            }
        }
        let default_nodes = self
            .graphs
            .remove(&None)
            .unwrap_or_default()
            .into_values()
            .collect();
        let named_graphs = self
            .graphs
            .into_iter()
            .filter_map(|(graph, nodes)| {
                graph.map(|id| NamedGraph {
                    id,
                    nodes: nodes.into_values().collect(),
                })
            })
            .collect();
        Document {
            default_nodes,
            named_graphs,
        }
    }

    fn fresh_blank_node(&mut self) -> String {
        loop {
            let id = format!("_:jsonld{}", self.next_blank_node);
            self.next_blank_node += 1;
            if self.reserved_blank_nodes.insert(id.clone()) {
                return id;
            }
        }
    }

    fn merge_node(&mut self, graph: Option<&str>, mut fragment: Node) {
        let nodes = self.graphs.entry(graph.map(str::to_owned)).or_default();
        let target = nodes
            .entry(fragment.id.clone())
            .or_insert_with(|| Node::new(fragment.id.clone()));
        target.types.append(&mut fragment.types);
        target.types.sort();
        target.types.dedup();
        for (property, mut values) in fragment.properties {
            target
                .properties
                .entry(property)
                .or_default()
                .append(&mut values);
        }
        target.sort_values();
    }

    fn add_property(
        &mut self,
        graph: Option<&str>,
        subject: String,
        property: String,
        value: Value,
    ) {
        let mut node = Node::new(subject);
        node.properties.entry(property).or_default().push(value);
        self.merge_node(graph, node);
    }

    fn expand_graph_entry(
        &mut self,
        entry: &JsonValue,
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
    ) -> Result<(), RdfDiagnostic> {
        let object = entry
            .as_object()
            .ok_or_else(|| decode("JSON-LD graph entry must be an object"))?;
        let active = object_context(context, object)?;
        let members = expanded_members(&active, object)?;
        if let Some(graph_value) = member(&members, "@graph") {
            let has_node_members = members.iter().any(|entry| {
                !matches!(
                    entry.expanded.as_str(),
                    "@context" | "@graph" | "@id" | "@included" | "@index"
                )
            });
            let graph_id = if let Some(id) = member(&members, "@id") {
                Some(expand_id_value(&active, id.value)?)
            } else if has_node_members {
                Some(self.fresh_blank_node())
            } else {
                graph.map(str::to_owned)
            };
            for node in as_values(graph_value.value) {
                self.expand_graph_entry(node, graph_id.as_deref(), &active.child_context())?;
            }
            if let Some(included) = member(&members, "@included") {
                for included in as_values(included.value) {
                    self.expand_graph_entry(included, graph, &active.child_context())?;
                }
            }

            // A graph object may also be a node object. Its @type, ordinary
            // properties, @reverse, and @nest members describe the graph name in the
            // containing graph and must not disappear merely because @graph is
            // present. A pure @id/@graph wrapper, however, contributes no node
            // statement of its own.
            if has_node_members {
                if member(&members, "@id").is_some() {
                    self.expand_node(entry, graph, &active, NodeDisposition::Merge)?;
                } else {
                    let mut node = object.clone();
                    insert_expanded_control(
                        &mut node,
                        &active,
                        "@id",
                        JsonValue::String(
                            graph_id
                                .clone()
                                .expect("mixed graph object minted a graph identifier"),
                        ),
                    )?;
                    self.expand_node(
                        &JsonValue::Object(node),
                        graph,
                        &active,
                        NodeDisposition::Merge,
                    )?;
                }
            }
            return Ok(());
        }
        self.expand_node(entry, graph, &active, NodeDisposition::Merge)
            .map(|_| ())
    }

    fn expand_node(
        &mut self,
        value: &JsonValue,
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
        disposition: NodeDisposition,
    ) -> Result<Node, RdfDiagnostic> {
        let object = value
            .as_object()
            .ok_or_else(|| decode("JSON-LD node must be an object"))?;
        let active = object_context(context, object)?;
        let members = expanded_members(&active, object)?;
        let id = member(&members, "@id")
            .map(|entry| expand_id_value(&active, entry.value))
            .transpose()?
            .unwrap_or_else(|| self.fresh_blank_node());
        let mut node = Node::new(id);

        if let Some(types) = member(&members, "@type") {
            for value in as_values(types.value) {
                let compact = value
                    .as_str()
                    .or_else(|| {
                        value
                            .as_object()
                            .and_then(|object| {
                                expanded_member(&active, object, "@id").ok().flatten()
                            })
                            .and_then(JsonValue::as_str)
                    })
                    .ok_or_else(|| decode("@type value must be an IRI string"))?;
                node.types
                    .push(expand_required(&active, compact, true, false)?);
            }
        }

        self.expand_node_members(&mut node, &members, graph, &active)?;
        node.sort_values();
        if matches!(disposition, NodeDisposition::Merge) {
            self.merge_node(graph, node.clone());
        }
        Ok(node)
    }

    fn expand_node_members(
        &mut self,
        node: &mut Node,
        members: &[ExpandedMember<'_>],
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
    ) -> Result<(), RdfDiagnostic> {
        for entry in members {
            match entry.expanded.as_str() {
                "@context" | "@id" | "@type" => {}
                "@included" => {
                    for included in as_values(entry.value) {
                        self.expand_node(
                            included,
                            graph,
                            &context.child_context(),
                            NodeDisposition::Merge,
                        )?;
                    }
                }
                "@nest" => {
                    for nested in as_values(entry.value) {
                        let object = nested
                            .as_object()
                            .ok_or_else(|| decode("@nest value must be an object"))?;
                        let nested_members = expanded_members(context, object)?;
                        self.expand_node_members(node, &nested_members, graph, context)?;
                    }
                }
                "@reverse" => {
                    let reverse = entry
                        .value
                        .as_object()
                        .ok_or_else(|| decode("@reverse value must be an object"))?;
                    let reverse_members = expanded_members(context, reverse)?;
                    for reverse_entry in reverse_members {
                        if reverse_entry.expanded.starts_with('@') {
                            return Err(decode(format!(
                                "unsupported keyword `{}` inside @reverse",
                                reverse_entry.expanded
                            )));
                        }
                        self.expand_property(node, graph, context, &reverse_entry, true)?;
                    }
                }
                // `expand_graph_entry` processes this member before asking the node
                // path to retain any co-resident node statements.
                "@graph" | "@index" => {}
                keyword if keyword.starts_with('@') => {
                    return Err(decode(format!(
                        "unsupported JSON-LD node keyword `{keyword}`"
                    )));
                }
                _ => self.expand_property(node, graph, context, entry, false)?,
            }
        }
        Ok(())
    }

    fn expand_property(
        &mut self,
        node: &mut Node,
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
        entry: &ExpandedMember<'_>,
        reverse_block: bool,
    ) -> Result<(), RdfDiagnostic> {
        let definition = context.term(entry.original);
        let scoped = context.scoped_context(entry.original)?;
        let value_context = scoped.as_ref().unwrap_or(context);
        let values = self.expand_property_values(entry.value, graph, value_context, definition)?;
        let reverse =
            reverse_block ^ definition.is_some_and(JsonLdTermDefinition::is_reverse_property);
        if reverse {
            for mut value in values {
                let Term::Id(target) = value.term else {
                    return Err(decode(format!(
                        "reverse property `{}` requires node-reference values",
                        entry.original
                    )));
                };
                value.term = Term::Id(node.id.clone());
                self.add_property(graph, target, entry.expanded.clone(), value);
            }
        } else {
            node.properties
                .entry(entry.expanded.clone())
                .or_default()
                .extend(values);
        }
        Ok(())
    }

    fn expand_property_values(
        &mut self,
        raw: &JsonValue,
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
        definition: Option<&JsonLdTermDefinition>,
    ) -> Result<Vec<Value>, RdfDiagnostic> {
        if definition.is_some_and(|definition| {
            definition.type_mapping() == Some(&JsonLdTypeMapping::Json)
                && !definition.containers().contains(&JsonLdContainer::Set)
        }) {
            return Ok(vec![expand_scalar(raw, context, definition)?]);
        }
        if definition
            .is_some_and(|definition| definition.containers().contains(&JsonLdContainer::Graph))
        {
            return self.expand_graph_container(raw, graph, context, definition);
        }
        if definition
            .is_some_and(|definition| definition.containers().contains(&JsonLdContainer::Language))
        {
            return expand_language_map(raw, context, definition);
        }
        if definition
            .is_some_and(|definition| definition.containers().contains(&JsonLdContainer::Id))
        {
            return self.expand_id_map(raw, graph, context, definition);
        }
        if definition
            .is_some_and(|definition| definition.containers().contains(&JsonLdContainer::Type))
        {
            return self.expand_type_map(raw, graph, context, definition);
        }
        if definition
            .is_some_and(|definition| definition.containers().contains(&JsonLdContainer::Index))
            && raw.is_object()
        {
            return self.expand_index_map(raw, graph, context, definition);
        }
        if definition
            .is_some_and(|definition| definition.containers().contains(&JsonLdContainer::List))
        {
            let mut items = Vec::new();
            for raw in as_values(raw) {
                if let Some(value) = self.expand_value(raw, graph, context, definition)? {
                    items.push(value);
                }
            }
            return Ok(vec![Value::plain(Term::List(items))]);
        }
        if let Some(object) = raw.as_object() {
            let active = object_context(context, object)?;
            let members = expanded_members(&active, object)?;
            if let Some(set) = member(&members, "@set") {
                reject_unexpected_members(&members, "@set object", &["@context", "@set"])?;
                return self.expand_property_values(set.value, graph, &active, definition);
            }
        }
        let mut values = Vec::new();
        for raw in as_values(raw) {
            if let Some(value) = self.expand_value(raw, graph, context, definition)? {
                values.push(value);
            }
        }
        Ok(values)
    }

    fn expand_value(
        &mut self,
        raw: &JsonValue,
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
        definition: Option<&JsonLdTermDefinition>,
    ) -> Result<Option<Value>, RdfDiagnostic> {
        if raw.is_null() {
            return Ok(None);
        }
        if !raw.is_object() {
            return expand_scalar(raw, context, definition).map(Some);
        }
        let object = raw
            .as_object()
            .expect("non-object values returned before this point");
        let active = object_context(context, object)?;
        let members = expanded_members(&active, object)?;
        if let Some(set) = member(&members, "@set") {
            reject_unexpected_members(&members, "@set object", &["@context", "@set"])?;
            let values = self.expand_property_values(set.value, graph, &active, definition)?;
            if let [only] = values.as_slice() {
                return Ok(Some(only.clone()));
            }
            return Err(decode("nested @set value must contain exactly one value"));
        }
        let mut value = if let Some(list) = member(&members, "@list") {
            reject_unexpected_members(
                &members,
                "@list object",
                &["@annotation", "@context", "@list"],
            )?;
            let mut items = Vec::new();
            for item in as_values(list.value) {
                if let Some(value) = self.expand_value(item, graph, &active, definition)? {
                    items.push(value);
                }
            }
            Value::plain(Term::List(items))
        } else if let Some(graph_value) = member(&members, "@graph") {
            reject_unexpected_members(
                &members,
                "@graph object",
                &["@annotation", "@context", "@graph", "@id", "@index"],
            )?;
            let graph_id = member(&members, "@id")
                .map(|entry| expand_id_value(&active, entry.value))
                .transpose()?
                .unwrap_or_else(|| self.fresh_blank_node());
            if member(&members, "@index").is_some() {
                return Err(decode(
                    "data-bearing @index metadata has no RDF dataset representation",
                ));
            }
            self.expand_graph_contents(graph_value.value, &graph_id, &active)?;
            Value::plain(Term::Id(graph_id))
        } else if let Some(triple) = member(&members, "@triple") {
            Value::plain(Term::Triple(Box::new(self.expand_triple(
                triple.value,
                graph,
                &active,
            )?)))
        } else if member(&members, "@value").is_some_and(|member| member.value.is_null()) {
            reject_unexpected_members(
                &members,
                "@value object",
                &[
                    "@annotation",
                    "@context",
                    "@direction",
                    "@language",
                    "@type",
                    "@value",
                ],
            )?;
            if member(&members, "@annotation").is_some() {
                return Err(decode("null @value object cannot carry @annotation"));
            }
            return Ok(None);
        } else if member(&members, "@value").is_some() {
            reject_unexpected_members(
                &members,
                "@value object",
                &[
                    "@annotation",
                    "@context",
                    "@direction",
                    "@language",
                    "@type",
                    "@value",
                ],
            )?;
            Value::plain(Term::Literal(expand_value_object(&members, &active)?))
        } else if let Some(id) = member(&members, "@id") {
            let id = expand_id_value(&active, id.value)?;
            let has_node_content = members.iter().any(|member| {
                !matches!(member.expanded.as_str(), "@context" | "@id" | "@annotation")
            });
            if has_node_content {
                let mut node_object = object.clone();
                node_object.remove("@annotation");
                self.expand_node(
                    &JsonValue::Object(node_object),
                    graph,
                    &active,
                    NodeDisposition::Merge,
                )?;
            }
            Value::plain(Term::Id(id))
        } else {
            let node = self.expand_node(raw, graph, &active, NodeDisposition::Merge)?;
            Value::plain(Term::Id(node.id))
        };

        if let Some(annotation) = member(&members, "@annotation") {
            for annotation in non_null_values(annotation.value) {
                value.annotations.push(self.expand_node(
                    annotation,
                    graph,
                    &active,
                    NodeDisposition::Detached,
                )?);
            }
        }
        Ok(Some(value))
    }

    fn expand_graph_container(
        &mut self,
        raw: &JsonValue,
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
        definition: Option<&JsonLdTermDefinition>,
    ) -> Result<Vec<Value>, RdfDiagnostic> {
        let definition = definition.expect("@graph container requires a term definition");
        let containers = definition.containers();
        if containers.contains(&JsonLdContainer::Id) || containers.contains(&JsonLdContainer::Index)
        {
            let map = raw
                .as_object()
                .ok_or_else(|| decode("@graph map container value must be an object"))?;
            let mut values = Vec::new();
            for (key, entries) in map {
                for entry in non_null_values(entries) {
                    let graph_id = if containers.contains(&JsonLdContainer::Id) && key != "@none" {
                        expand_required(context, key, false, true)?
                    } else {
                        self.fresh_blank_node()
                    };
                    self.expand_graph_contents(entry, &graph_id, context)?;
                    if containers.contains(&JsonLdContainer::Index) && key != "@none" {
                        let index_mapping = definition.index_mapping().ok_or_else(|| {
                            decode("data-bearing @index metadata has no RDF dataset representation")
                        })?;
                        self.add_property(
                            graph,
                            graph_id.clone(),
                            index_mapping.to_owned(),
                            Value::plain(Term::Literal(Literal {
                                lexical: key.clone(),
                                datatype: None,
                                language: None,
                                direction: None,
                            })),
                        );
                    }
                    values.push(Value::plain(Term::Id(graph_id)));
                }
            }
            return Ok(values);
        }

        let mut values = Vec::new();
        for entry in non_null_values(raw) {
            let graph_id = self.fresh_blank_node();
            self.expand_graph_contents(entry, &graph_id, context)?;
            values.push(Value::plain(Term::Id(graph_id)));
        }
        Ok(values)
    }

    fn expand_graph_contents(
        &mut self,
        raw: &JsonValue,
        graph_id: &str,
        context: &CompiledJsonLdContext,
    ) -> Result<(), RdfDiagnostic> {
        self.graphs.entry(Some(graph_id.to_owned())).or_default();
        for entry in non_null_values(raw) {
            self.expand_graph_entry(entry, Some(graph_id), &context.child_context())?;
        }
        Ok(())
    }

    fn expand_triple(
        &mut self,
        raw: &JsonValue,
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
    ) -> Result<Triple, RdfDiagnostic> {
        let object = raw
            .as_object()
            .ok_or_else(|| decode("@triple must be an object"))?;
        let members = expanded_members(context, object)?;
        reject_unexpected_members(
            &members,
            "@triple object",
            &[
                "@annotation",
                "@context",
                "@object",
                "@predicate",
                "@subject",
            ],
        )?;
        if member(&members, "@annotation").is_some() {
            return Err(decode(
                "@annotation is not permitted inside a @triple object in RDF 1.2",
            ));
        }
        let subject =
            member(&members, "@subject").ok_or_else(|| decode("@triple missing @subject"))?;
        let predicate =
            member(&members, "@predicate").ok_or_else(|| decode("@triple missing @predicate"))?;
        let object =
            member(&members, "@object").ok_or_else(|| decode("@triple missing @object"))?;
        let subject = self
            .expand_value(subject.value, graph, context, None)?
            .ok_or_else(|| decode("@triple @subject cannot be null"))?;
        let object = self
            .expand_value(object.value, graph, context, None)?
            .ok_or_else(|| decode("@triple @object cannot be null"))?;
        if !subject.annotations.is_empty() || !object.annotations.is_empty() {
            return Err(decode(
                "@annotation is not permitted inside a @triple component in RDF 1.2",
            ));
        }
        if matches!(subject.term, Term::Literal(_) | Term::List(_)) {
            return Err(decode("@triple @subject must be a node or triple term"));
        }
        let predicate = predicate
            .value
            .as_str()
            .ok_or_else(|| decode("@triple @predicate must be a string"))?;
        Ok(Triple {
            subject: Box::new(subject.term),
            predicate: expand_required(context, predicate, true, false)?,
            object: Box::new(object.term),
        })
    }

    fn expand_id_map(
        &mut self,
        raw: &JsonValue,
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
        definition: Option<&JsonLdTermDefinition>,
    ) -> Result<Vec<Value>, RdfDiagnostic> {
        let object = raw
            .as_object()
            .ok_or_else(|| decode("@id container value must be an object"))?;
        let mut result = Vec::new();
        for (key, entries) in object {
            for entry in non_null_values(entries) {
                let id = if key == "@none" {
                    None
                } else {
                    Some(expand_required(context, key, false, true)?)
                };
                result.push(self.expand_container_node(
                    entry,
                    graph,
                    context,
                    definition,
                    id.as_deref(),
                    None,
                    None,
                )?);
            }
        }
        Ok(result)
    }

    fn expand_type_map(
        &mut self,
        raw: &JsonValue,
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
        definition: Option<&JsonLdTermDefinition>,
    ) -> Result<Vec<Value>, RdfDiagnostic> {
        let object = raw
            .as_object()
            .ok_or_else(|| decode("@type container value must be an object"))?;
        let mut result = Vec::new();
        for (key, entries) in object {
            let rdf_type = (key != "@none")
                .then(|| expand_required(context, key, true, false))
                .transpose()?;
            for entry in non_null_values(entries) {
                result.push(self.expand_container_node(
                    entry,
                    graph,
                    context,
                    definition,
                    None,
                    rdf_type.as_deref(),
                    None,
                )?);
            }
        }
        Ok(result)
    }

    fn expand_index_map(
        &mut self,
        raw: &JsonValue,
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
        definition: Option<&JsonLdTermDefinition>,
    ) -> Result<Vec<Value>, RdfDiagnostic> {
        let object = raw
            .as_object()
            .ok_or_else(|| decode("@index container value must be an object"))?;
        let index_mapping = definition.and_then(JsonLdTermDefinition::index_mapping);
        let mut result = Vec::new();
        for (key, entries) in object {
            if key != "@none" && index_mapping.is_none() {
                return Err(decode(
                    "data-bearing @index metadata has no RDF dataset representation",
                ));
            }
            for entry in non_null_values(entries) {
                result.push(
                    self.expand_container_node(
                        entry,
                        graph,
                        context,
                        definition,
                        None,
                        None,
                        (key != "@none").then_some((
                            index_mapping.expect("checked mapped index"),
                            key.as_str(),
                        )),
                    )?,
                );
            }
        }
        Ok(result)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "container expansion keeps independent id, type, and index injections explicit"
    )]
    fn expand_container_node(
        &mut self,
        raw: &JsonValue,
        graph: Option<&str>,
        context: &CompiledJsonLdContext,
        definition: Option<&JsonLdTermDefinition>,
        id: Option<&str>,
        rdf_type: Option<&str>,
        index: Option<(&str, &str)>,
    ) -> Result<Value, RdfDiagnostic> {
        let mut object = match raw {
            JsonValue::Object(object) => object.clone(),
            _ => {
                return self
                    .expand_value(raw, graph, context, definition)?
                    .ok_or_else(|| decode("container value cannot be null"));
            }
        };
        if let Some(id) = id {
            insert_expanded_control(
                &mut object,
                context,
                "@id",
                JsonValue::String(id.to_owned()),
            )?;
        }
        if let Some(rdf_type) = rdf_type {
            insert_expanded_control(
                &mut object,
                context,
                "@type",
                JsonValue::String(rdf_type.to_owned()),
            )?;
        }
        let node = self.expand_node(
            &JsonValue::Object(object),
            graph,
            context,
            NodeDisposition::Merge,
        )?;
        if let Some((property, value)) = index {
            self.add_property(
                graph,
                node.id.clone(),
                property.to_owned(),
                Value::plain(Term::Literal(Literal {
                    lexical: value.to_owned(),
                    datatype: None,
                    language: None,
                    direction: None,
                })),
            );
        }
        Ok(Value::plain(Term::Id(node.id)))
    }
}

fn expand_language_map(
    raw: &JsonValue,
    context: &CompiledJsonLdContext,
    definition: Option<&JsonLdTermDefinition>,
) -> Result<Vec<Value>, RdfDiagnostic> {
    let object = raw
        .as_object()
        .ok_or_else(|| decode("@language container value must be an object"))?;
    let direction = effective_direction(context, definition);
    let mut values = Vec::new();
    for (language, entries) in object {
        for entry in non_null_values(entries) {
            let lexical = entry
                .as_str()
                .ok_or_else(|| decode("language-map values must be strings"))?;
            values.push(Value::plain(Term::Literal(Literal {
                lexical: lexical.to_owned(),
                datatype: None,
                language: (language != "@none").then(|| language.to_ascii_lowercase()),
                direction: direction.map(str::to_owned),
            })));
        }
    }
    Ok(values)
}

fn expand_scalar(
    raw: &JsonValue,
    context: &CompiledJsonLdContext,
    definition: Option<&JsonLdTermDefinition>,
) -> Result<Value, RdfDiagnostic> {
    if let Some(mapping) = definition.and_then(JsonLdTermDefinition::type_mapping) {
        match mapping {
            JsonLdTypeMapping::Id | JsonLdTypeMapping::Vocab => {
                let value = raw
                    .as_str()
                    .ok_or_else(|| decode("@id/@vocab coercion requires a string"))?;
                return Ok(Value::plain(Term::Id(expand_required(
                    context,
                    value,
                    matches!(mapping, JsonLdTypeMapping::Vocab),
                    matches!(mapping, JsonLdTypeMapping::Id),
                )?)));
            }
            JsonLdTypeMapping::Json => {
                return Ok(Value::plain(Term::Literal(Literal {
                    lexical: serde_json::to_string(raw)
                        .map_err(|source| decode(format!("encode rdf:JSON value: {source}")))?,
                    datatype: Some(RDF_JSON.to_owned()),
                    language: None,
                    direction: None,
                })));
            }
            JsonLdTypeMapping::Datatype(datatype) => {
                return Ok(Value::plain(Term::Literal(Literal {
                    lexical: scalar_lexical_for_datatype(raw, Some(datatype))?,
                    datatype: Some(datatype.clone()),
                    language: None,
                    direction: None,
                })));
            }
            JsonLdTypeMapping::None => {}
        }
    }
    let (lexical, datatype) = match raw {
        JsonValue::String(value) => (value.clone(), None),
        JsonValue::Bool(value) => (value.to_string(), Some(XSD_BOOLEAN.to_owned())),
        JsonValue::Number(value) if value.as_i64().is_some() || value.as_u64().is_some() => {
            (value.to_string(), Some(XSD_INTEGER.to_owned()))
        }
        JsonValue::Number(value) => (canonical_json_double(value)?, Some(XSD_DOUBLE.to_owned())),
        _ => {
            return Err(decode(
                "JSON-LD scalar must be a string, number, or boolean",
            ));
        }
    };
    Ok(Value::plain(Term::Literal(Literal {
        lexical,
        datatype,
        language: if raw.is_string() {
            effective_language(context, definition).map(str::to_owned)
        } else {
            None
        },
        direction: if raw.is_string() {
            effective_direction(context, definition).map(str::to_owned)
        } else {
            None
        },
    })))
}

fn expand_value_object(
    members: &[ExpandedMember<'_>],
    context: &CompiledJsonLdContext,
) -> Result<Literal, RdfDiagnostic> {
    let value = member(members, "@value")
        .expect("caller checked @value member")
        .value;
    let expanded_type = member(members, "@type")
        .map(|entry| {
            entry
                .value
                .as_str()
                .ok_or_else(|| decode("@type in a value object must be a string"))
                .and_then(|value| expand_required(context, value, true, false))
        })
        .transpose()?;
    let json_keyword = expanded_type.as_deref() == Some("@json");
    let datatype = expanded_type.map(|datatype| {
        if datatype == "@json" {
            RDF_JSON.to_owned()
        } else {
            datatype
        }
    });
    let language = member(members, "@language")
        .map(|entry| {
            entry
                .value
                .as_str()
                .map(str::to_ascii_lowercase)
                .ok_or_else(|| decode("@language must be a string"))
        })
        .transpose()?;
    let direction = member(members, "@direction")
        .map(|entry| {
            let direction = entry
                .value
                .as_str()
                .ok_or_else(|| decode("@direction must be a string"))?;
            if !matches!(direction, "ltr" | "rtl") {
                return Err(decode("@direction must be `ltr` or `rtl`"));
            }
            Ok(direction.to_owned())
        })
        .transpose()?;
    if datatype.is_some() && (language.is_some() || direction.is_some()) {
        return Err(decode(
            "JSON-LD value object cannot combine @type with @language/@direction",
        ));
    }
    if json_keyword {
        return Ok(Literal {
            lexical: serde_json::to_string(value)
                .map_err(|source| decode(format!("encode rdf:JSON value: {source}")))?,
            datatype,
            language: None,
            direction: None,
        });
    }
    let inferred_datatype = if datatype.is_none() {
        match value {
            JsonValue::Bool(_) => Some(XSD_BOOLEAN.to_owned()),
            JsonValue::Number(number) if number.as_i64().is_some() || number.as_u64().is_some() => {
                Some(XSD_INTEGER.to_owned())
            }
            JsonValue::Number(_) => Some(XSD_DOUBLE.to_owned()),
            _ => None,
        }
    } else {
        None
    };
    let datatype = datatype
        .filter(|datatype| datatype != XSD_STRING)
        .or(inferred_datatype);
    let lexical = scalar_lexical_for_datatype(value, datatype.as_deref())?;
    Ok(Literal {
        lexical,
        datatype,
        language,
        direction,
    })
}

fn scalar_lexical_for_datatype(
    value: &JsonValue,
    datatype: Option<&str>,
) -> Result<String, RdfDiagnostic> {
    match value {
        JsonValue::String(value) => Ok(value.clone()),
        JsonValue::Bool(value) => Ok(value.to_string()),
        JsonValue::Number(value) if datatype == Some(XSD_DOUBLE) => canonical_json_double(value),
        JsonValue::Number(value) if value.as_i64().is_some() || value.as_u64().is_some() => {
            Ok(value.to_string())
        }
        // JSON-LD's RDF conversion uses the canonical xsd:double lexical for
        // every non-integral JSON number even when a context coerces the final
        // datatype to another IRI.
        JsonValue::Number(value) => canonical_json_double(value),
        _ => Err(decode(
            "literal @value must be a scalar unless @type is @json",
        )),
    }
}

fn canonical_json_double(value: &serde_json::Number) -> Result<String, RdfDiagnostic> {
    value
        .as_f64()
        .map(purrdf_xsd::numeric::canonical_double)
        .ok_or_else(|| {
            decode(format!(
                "JSON number `{value}` is outside the xsd:double value space"
            ))
        })
}

fn effective_language<'a>(
    context: &'a CompiledJsonLdContext,
    definition: Option<&'a JsonLdTermDefinition>,
) -> Option<&'a str> {
    match definition.and_then(JsonLdTermDefinition::language_mapping) {
        Some(JsonLdNullable::Null) => None,
        Some(JsonLdNullable::Value(language)) => Some(language),
        None => context.default_language(),
    }
}

fn effective_direction(
    context: &CompiledJsonLdContext,
    definition: Option<&JsonLdTermDefinition>,
) -> Option<&'static str> {
    match definition.and_then(JsonLdTermDefinition::direction_mapping) {
        Some(JsonLdNullable::Null) => None,
        Some(JsonLdNullable::Value(JsonLdDirection::LeftToRight)) => Some("ltr"),
        Some(JsonLdNullable::Value(JsonLdDirection::RightToLeft)) => Some("rtl"),
        None => context.default_direction().map(JsonLdDirection::as_str),
    }
}

fn object_context(
    parent: &CompiledJsonLdContext,
    object: &Map<String, JsonValue>,
) -> Result<CompiledJsonLdContext, RdfDiagnostic> {
    object.get("@context").map_or_else(
        || Ok(parent.clone()),
        |local| parent.apply_local_context(local),
    )
}

struct ExpandedMember<'a> {
    original: &'a str,
    expanded: String,
    value: &'a JsonValue,
}

fn expanded_members<'a>(
    context: &CompiledJsonLdContext,
    object: &'a Map<String, JsonValue>,
) -> Result<Vec<ExpandedMember<'a>>, RdfDiagnostic> {
    let mut seen = BTreeSet::new();
    let mut members = Vec::with_capacity(object.len());
    for (key, value) in object {
        let Some(expanded) = context.expand_iri(key, true, false)? else {
            continue;
        };
        if expanded.starts_with('@') && !seen.insert(expanded.clone()) {
            return Err(decode(format!(
                "JSON-LD object has multiple members expanding to `{expanded}`"
            )));
        }
        members.push(ExpandedMember {
            original: key,
            expanded,
            value,
        });
    }
    Ok(members)
}

fn member<'a, 'b>(
    members: &'b [ExpandedMember<'a>],
    keyword: &str,
) -> Option<&'b ExpandedMember<'a>> {
    members.iter().find(|entry| entry.expanded == keyword)
}

fn expanded_member<'a>(
    context: &CompiledJsonLdContext,
    object: &'a Map<String, JsonValue>,
    keyword: &str,
) -> Result<Option<&'a JsonValue>, RdfDiagnostic> {
    let members = expanded_members(context, object)?;
    Ok(member(&members, keyword).map(|member| member.value))
}

fn reject_unexpected_members(
    members: &[ExpandedMember<'_>],
    shape: &str,
    allowed: &[&str],
) -> Result<(), RdfDiagnostic> {
    if let Some(member) = members
        .iter()
        .find(|member| !allowed.contains(&member.expanded.as_str()))
    {
        return Err(decode(format!(
            "{shape} contains unexpected member `{}`",
            member.original
        )));
    }
    Ok(())
}

fn expand_required(
    context: &CompiledJsonLdContext,
    value: &str,
    vocab: bool,
    document_relative: bool,
) -> Result<String, RdfDiagnostic> {
    context
        .expand_iri(value, vocab, document_relative)?
        .ok_or_else(|| decode(format!("`{value}` has a null JSON-LD IRI mapping")))
}

fn expand_id_value(
    context: &CompiledJsonLdContext,
    value: &JsonValue,
) -> Result<String, RdfDiagnostic> {
    let value = value
        .as_str()
        .ok_or_else(|| decode("@id must be a string"))?;
    if value.starts_with("_:") {
        Ok(value.to_owned())
    } else {
        expand_required(context, value, false, true)
    }
}

fn as_values(value: &JsonValue) -> &[JsonValue] {
    match value {
        JsonValue::Array(values) => values,
        value => std::slice::from_ref(value),
    }
}

fn non_null_values(value: &JsonValue) -> impl Iterator<Item = &JsonValue> {
    as_values(value).iter().filter(|value| !value.is_null())
}

fn insert_expanded_control(
    object: &mut Map<String, JsonValue>,
    context: &CompiledJsonLdContext,
    keyword: &str,
    value: JsonValue,
) -> Result<(), RdfDiagnostic> {
    if expanded_member(context, object, keyword)?.is_some() {
        return Err(decode(format!(
            "container map value already defines `{keyword}`"
        )));
    }
    object.insert(keyword.to_owned(), value);
    Ok(())
}

fn collect_blank_node_ids(value: &JsonValue, output: &mut BTreeSet<String>) {
    match value {
        JsonValue::String(value) if value.starts_with("_:") => {
            output.insert(value.clone());
        }
        JsonValue::Array(values) => {
            for value in values {
                collect_blank_node_ids(value, output);
            }
        }
        JsonValue::Object(object) => {
            for value in object.values() {
                collect_blank_node_ids(value, output);
            }
        }
        _ => {}
    }
}

struct Lowerer {
    quads: Vec<RdfQuad>,
    reserved_blank_nodes: BTreeSet<String>,
    next_list: u64,
}

impl Lowerer {
    fn new(document: &Document) -> Self {
        let mut reserved_blank_nodes = BTreeSet::new();
        for graph in &document.named_graphs {
            reserve_blank_id(&graph.id, &mut reserved_blank_nodes);
        }
        for node in document.default_nodes.iter().chain(
            document
                .named_graphs
                .iter()
                .flat_map(|graph| graph.nodes.iter()),
        ) {
            reserve_node_blank_ids(node, &mut reserved_blank_nodes);
        }
        Self {
            quads: Vec::new(),
            reserved_blank_nodes,
            next_list: 0,
        }
    }

    fn fresh_list_node(&mut self) -> RdfTerm {
        loop {
            let label = format!("jsonld_list_{}", self.next_list);
            self.next_list += 1;
            if self.reserved_blank_nodes.insert(label.clone()) {
                return RdfTerm::blank_node(label);
            }
        }
    }

    fn lower_node(&mut self, node: &Node, graph: Option<&RdfTerm>) -> Result<(), RdfDiagnostic> {
        let subject = id_term(&node.id)?;
        for rdf_type in &node.types {
            self.push(
                subject.clone(),
                RDF_TYPE,
                validated_iri_term(rdf_type)?,
                graph,
            );
        }
        for (predicate, values) in &node.properties {
            validated_iri_term(predicate)?;
            for value in values {
                let object = self.lower_term(&value.term, graph)?;
                self.push(subject.clone(), predicate, object.clone(), graph);
                self.lower_annotations(&subject, predicate, &object, &value.annotations, graph)?;
            }
        }
        Ok(())
    }

    fn lower_annotations(
        &mut self,
        subject: &RdfTerm,
        predicate: &str,
        object: &RdfTerm,
        annotations: &[Node],
        graph: Option<&RdfTerm>,
    ) -> Result<(), RdfDiagnostic> {
        for annotation in annotations {
            let reifier = id_term(&annotation.id)?;
            let triple =
                RdfTerm::triple(RdfTriple::new(subject.clone(), predicate, object.clone()));
            self.push(reifier.clone(), RDF_REIFIES, triple, graph);
            for rdf_type in &annotation.types {
                self.push(
                    reifier.clone(),
                    RDF_TYPE,
                    validated_iri_term(rdf_type)?,
                    graph,
                );
            }
            for (ann_predicate, values) in &annotation.properties {
                validated_iri_term(ann_predicate)?;
                for value in values {
                    let ann_object = self.lower_term(&value.term, graph)?;
                    self.push(reifier.clone(), ann_predicate, ann_object.clone(), graph);
                    self.lower_annotations(
                        &reifier,
                        ann_predicate,
                        &ann_object,
                        &value.annotations,
                        graph,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn lower_term(
        &mut self,
        term: &Term,
        graph: Option<&RdfTerm>,
    ) -> Result<RdfTerm, RdfDiagnostic> {
        match term {
            Term::Id(id) => id_term(id),
            Term::Literal(literal) => lower_literal(literal),
            Term::Triple(triple) => {
                let subject = self.lower_term(&triple.subject, graph)?;
                if matches!(subject, RdfTerm::Literal(_)) {
                    return Err(decode("RDF triple-term subject cannot be a literal"));
                }
                validated_iri_term(&triple.predicate)?;
                let object = self.lower_term(&triple.object, graph)?;
                Ok(RdfTerm::triple(RdfTriple::new(
                    subject,
                    &triple.predicate,
                    object,
                )))
            }
            Term::List(values) => self.lower_list(values, graph),
        }
    }

    fn lower_list(
        &mut self,
        values: &[Value],
        graph: Option<&RdfTerm>,
    ) -> Result<RdfTerm, RdfDiagnostic> {
        if values.is_empty() {
            return validated_iri_term(RDF_NIL);
        }
        let head = self.fresh_list_node();
        let mut current = head.clone();
        for (index, value) in values.iter().enumerate() {
            let item = self.lower_term(&value.term, graph)?;
            self.push(current.clone(), RDF_FIRST, item.clone(), graph);
            self.lower_annotations(&current, RDF_FIRST, &item, &value.annotations, graph)?;
            let rest = if index + 1 == values.len() {
                validated_iri_term(RDF_NIL)?
            } else {
                self.fresh_list_node()
            };
            self.push(current, RDF_REST, rest.clone(), graph);
            current = rest;
        }
        Ok(head)
    }

    fn push(
        &mut self,
        subject: RdfTerm,
        predicate: &str,
        object: RdfTerm,
        graph: Option<&RdfTerm>,
    ) {
        let mut quad = RdfQuad::new(subject, predicate, object);
        if let Some(graph) = graph {
            quad = quad.in_graph(graph.clone());
        }
        self.quads.push(quad);
    }
}

fn reserve_node_blank_ids(node: &Node, output: &mut BTreeSet<String>) {
    reserve_blank_id(&node.id, output);
    for values in node
        .properties
        .values()
        .chain(node.reverse_properties.values())
    {
        for value in values {
            reserve_value_blank_ids(value, output);
        }
    }
}

fn reserve_value_blank_ids(value: &Value, output: &mut BTreeSet<String>) {
    reserve_term_blank_ids(&value.term, output);
    for annotation in &value.annotations {
        reserve_node_blank_ids(annotation, output);
    }
}

fn reserve_term_blank_ids(term: &Term, output: &mut BTreeSet<String>) {
    match term {
        Term::Id(id) => reserve_blank_id(id, output),
        Term::Triple(triple) => {
            reserve_term_blank_ids(&triple.subject, output);
            reserve_term_blank_ids(&triple.object, output);
        }
        Term::List(values) => {
            for value in values {
                reserve_value_blank_ids(value, output);
            }
        }
        Term::Literal(_) => {}
    }
}

fn reserve_blank_id(id: &str, output: &mut BTreeSet<String>) {
    if let Some(label) = id.strip_prefix("_:") {
        output.insert(label.to_owned());
    }
}

fn id_term(id: &str) -> Result<RdfTerm, RdfDiagnostic> {
    if let Some(label) = id.strip_prefix("_:") {
        if label.is_empty() {
            return Err(decode("blank-node identifier cannot be empty"));
        }
        Ok(RdfTerm::blank_node(label.to_owned()))
    } else {
        validated_iri_term(id)
    }
}

fn lower_literal(literal: &Literal) -> Result<RdfTerm, RdfDiagnostic> {
    let value = match (
        literal.language.as_deref(),
        literal.direction.as_deref(),
        literal.datatype.as_deref(),
    ) {
        (Some(language), Some(direction), _) => {
            let direction = match direction {
                "ltr" => RdfTextDirection::Ltr,
                "rtl" => RdfTextDirection::Rtl,
                _ => return Err(decode(format!("invalid direction `{direction}`"))),
            };
            RdfLiteral {
                lexical_form: literal.lexical.clone(),
                datatype: None,
                language: Some(language.to_owned()),
                direction: Some(direction),
            }
        }
        (Some(language), None, _) => RdfLiteral::language_tagged(literal.lexical.clone(), language),
        (None, None, Some(datatype)) => {
            validated_iri_term(datatype)?;
            RdfLiteral::typed(literal.lexical.clone(), datatype)
        }
        (None, Some(_), _) => {
            return Err(decode("directional literal requires a language tag"));
        }
        (None, None, None) => RdfLiteral::simple(literal.lexical.clone()),
    };
    Ok(RdfTerm::literal(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_blank_node_sequence_is_not_pointer_width_limited() {
        let mut builder = Builder::new(&JsonValue::Null);
        builder.next_blank_node = u64::from(u32::MAX);

        assert_eq!(builder.fresh_blank_node(), "_:jsonld4294967295");
        assert_eq!(builder.next_blank_node, u64::from(u32::MAX) + 1);
    }
}
