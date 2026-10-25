// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed admission followed by canonical ownership transfer.

use super::{
    DCTERMS_IS_PART_OF, FNO, FNOM, FnFunction, FnImpl, FnMapping, FnOutput, FnParam,
    FnParamMapping, FnReturnMapping, FnoCatalogInput, OWL_ONTOLOGY, RDF_TYPE, RDFS_COMMENT,
    RDFS_LABEL, RDFS_SEE_ALSO, SKOS_DEFINITION,
};
use crate::TermValue;
use std::collections::{BTreeMap, BTreeSet};

/// Original catalog data and the native reason admission refused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnoAdmissionError {
    /// Original rejected records, strings, scopes and input ordering.
    pub input: Box<FnoCatalogInput>,
    /// Original semantic refusal and its location in the catalog.
    pub reason: Box<FnoAdmissionReason>,
}

/// A catalog cannot be represented reversibly by the FnO profile.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FnoAdmissionReason {
    /// A catalog IRI violates the kernel's absolute-IRI boundary.
    Iri {
        /// Original subject.
        subject: TermValue,
        /// Predicate whose value was refused.
        predicate: String,
        /// Original rejected spelling.
        value: String,
        /// Original native IRI refusal.
        cause: purrdf_iri::IriError,
    },
    /// A text field contains a nonliteral term.
    ExpectedLiteral {
        /// Original subject.
        subject: TermValue,
        /// Predicate whose value was refused.
        predicate: String,
        /// Original rejected native value.
        value: TermValue,
    },
    /// A literal's facets violate the shared RDF boundary.
    Literal {
        /// Original subject.
        subject: TermValue,
        /// Predicate whose value was refused.
        predicate: String,
        /// Original native RDF diagnostic.
        cause: crate::RdfDiagnostic,
    },
    /// One native subject has incompatible descriptions or structural roles.
    ConflictingNode {
        /// Original conflicting subject.
        subject: TermValue,
    },
    /// A reference does not name the required catalog member.
    Reference {
        /// Original referencing subject.
        subject: TermValue,
        /// Original reference predicate.
        predicate: String,
        /// Original unresolved or wrongly associated target.
        target: String,
    },
}

impl purrdf_hash::error::StructuredError for FnoAdmissionReason {
    purrdf_hash::error_visit! {
        Iri { subject, predicate, value, cause } => "fno-iri" {
            "subject" => Term(subject), "predicate" => Iri(predicate),
            "value" => Text(value), "cause" => Cause(cause),
        },
        ExpectedLiteral { subject, predicate, value } => "fno-expected-literal" {
            "subject" => Term(subject), "predicate" => Iri(predicate), "value" => Term(value),
        },
        Literal { subject, predicate, cause } => "fno-literal" {
            "subject" => Term(subject), "predicate" => Iri(predicate), "cause" => Cause(cause),
        },
        ConflictingNode { subject } => "fno-conflicting-node" { "subject" => Term(subject) },
        Reference { subject, predicate, target } => "fno-reference" {
            "subject" => Term(subject), "predicate" => Iri(predicate), "target" => Iri(target),
        },
    }
}

impl purrdf_hash::error::StructuredError for FnoAdmissionError {
    purrdf_hash::error_visit! {
        @record self { input, reason } => "fno-admission" {
            "input" => Record(&*self.input), "reason" => Cause(&*self.reason),
        }
    }
}

impl std::fmt::Display for FnoAdmissionReason {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Iri { value, cause, .. } => write!(out, "invalid FnO IRI {value:?}: {cause}"),
            Self::ExpectedLiteral { predicate, .. } => {
                write!(out, "FnO {predicate} requires a literal")
            }
            Self::Literal {
                predicate, cause, ..
            } => write!(out, "invalid FnO {predicate} literal: {cause}"),
            Self::ConflictingNode { subject } => {
                write!(out, "conflicting FnO node descriptions for {subject:?}")
            }
            Self::Reference {
                predicate, target, ..
            } => write!(
                out,
                "FnO {predicate} references an unavailable member {target:?}"
            ),
        }
    }
}

impl std::fmt::Display for FnoAdmissionError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.reason, out)
    }
}

purrdf_lex::error_source!(FnoAdmissionReason => {
    Self::Iri { cause, .. } => cause,
    Self::Literal { cause, .. } => cause,
});
purrdf_lex::error_source!(FnoAdmissionError => { Self { reason, .. } => reason.as_ref() });

fn iri(subject: &TermValue, predicate: &str, value: &str) -> Result<(), Box<FnoAdmissionReason>> {
    crate::ir::absolute::check_absolute(value).map_err(|cause| {
        Box::new(FnoAdmissionReason::Iri {
            subject: subject.clone(),
            predicate: predicate.to_owned(),
            value: value.to_owned(),
            cause,
        })
    })
}

fn text(
    subject: &TermValue,
    predicate: &str,
    value: &TermValue,
) -> Result<(), Box<FnoAdmissionReason>> {
    if !matches!(value, TermValue::Literal { .. }) {
        return Err(Box::new(FnoAdmissionReason::ExpectedLiteral {
            subject: subject.clone(),
            predicate: predicate.to_owned(),
            value: value.clone(),
        }));
    }
    let record = crate::ir::mutable::RecordValues {
        kind: crate::ir::mutable::RecordKind::Ordinary,
        quad: crate::ir::mutable::QuadValues {
            s: subject.clone(),
            p: TermValue::iri(predicate),
            o: value.clone(),
            g: None,
        },
    };
    crate::ir::validate::validate_record(&record).map_err(|cause| {
        Box::new(FnoAdmissionReason::Literal {
            subject: subject.clone(),
            predicate: predicate.to_owned(),
            cause,
        })
    })
}

fn optional_text(
    subject: &TermValue,
    predicate: &str,
    value: Option<&TermValue>,
) -> Result<(), Box<FnoAdmissionReason>> {
    value.map_or(Ok(()), |value| text(subject, predicate, value))
}

fn same_text(first: &TermValue, second: &TermValue) -> bool {
    match (first, second) {
        (
            TermValue::Literal {
                lexical_form: a,
                datatype: ad,
                language: al,
                direction: ax,
            },
            TermValue::Literal {
                lexical_form: b,
                datatype: bd,
                language: bl,
                direction: bx,
            },
        ) => {
            a == b
                && ad == bd
                && ax == bx
                && al.as_deref().map(purrdf_iri::langtag::identity_fold)
                    == bl.as_deref().map(purrdf_iri::langtag::identity_fold)
        }
        _ => first == second,
    }
}

#[derive(Clone, Copy)]
enum Node<'a> {
    Document(&'a FnoCatalogInput),
    Function(&'a FnFunction),
    Output(&'a FnOutput),
    Param(&'a FnParam),
    Implementation(&'a FnImpl),
    Mapping(&'a FnMapping),
    ParamMapping(&'a FnParamMapping),
    ReturnMapping(&'a FnReturnMapping),
}

type Nodes<'a> = BTreeMap<TermValue, crate::SmallVec<[Node<'a>; 1]>>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Property {
    Label,
    Description,
    Comment,
    PartOf,
    Predicate,
    Type,
    Required,
    SeeAlso,
    Format,
    Expects,
    Returns,
    Function,
    Implementation,
    FunctionParameter,
    FunctionOutput,
    Binding,
}

#[derive(Clone, Copy)]
enum Value<'a> {
    Text(&'a TermValue),
    Iri(&'a str),
    Plain(&'a str),
    Boolean(bool),
    List(&'a [String]),
}

impl Value<'_> {
    fn same(self, other: Self) -> bool {
        match (self, other) {
            (Self::Text(a), Self::Text(b)) => same_text(a, b),
            (Self::Iri(a), Self::Iri(b)) | (Self::Plain(a), Self::Plain(b)) => a == b,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            _ => false,
        }
    }
}

impl<'a> Node<'a> {
    /// Scalar agreement is shared across structural roles; type/link sets union.
    fn properties(self) -> crate::SmallVec<[(Property, Value<'a>); 5]> {
        use Property as P;
        use Value as V;
        let mut values = crate::SmallVec::new();
        match self {
            Self::Document(data) => values.extend([
                (P::Label, V::Text(&data.doc_label)),
                (P::Comment, V::Text(&data.banner)),
                (P::PartOf, V::Iri(&data.ontology_iri)),
            ]),
            Self::Function(item) => {
                values.extend([
                    (P::Label, V::Text(&item.label)),
                    (P::Expects, V::List(&item.expects)),
                    (P::Returns, V::Iri(&item.output.iri)),
                ]);
                if let Some(value) = &item.description {
                    values.push((P::Description, V::Text(value)));
                }
                if let Some(value) = &item.see_also {
                    values.push((P::SeeAlso, V::Iri(value)));
                }
            }
            Self::Output(item) => {
                values.push((P::Type, V::Iri(&item.r#type)));
                if let Some(value) = &item.predicate {
                    values.push((P::Predicate, V::Iri(value)));
                }
                if let Some(value) = &item.label {
                    values.push((P::Label, V::Text(value)));
                }
                if let Some(value) = &item.description {
                    values.push((P::Description, V::Text(value)));
                }
            }
            Self::Param(item) => {
                values.extend([
                    (P::Type, V::Iri(&item.r#type)),
                    (P::Required, V::Boolean(item.required)),
                ]);
                if let Some(value) = &item.predicate {
                    values.push((P::Predicate, V::Iri(value)));
                }
                if let Some(value) = &item.label {
                    values.push((P::Label, V::Text(value)));
                }
                if let Some(value) = &item.description {
                    values.push((P::Description, V::Text(value)));
                }
            }
            Self::Implementation(item) => values.extend([
                (P::Format, V::Plain(&item.format)),
                (P::SeeAlso, V::Iri(&item.see_also)),
            ]),
            Self::Mapping(item) => values.extend([
                (P::Label, V::Text(&item.label)),
                (P::Function, V::Iri(&item.function)),
                (P::Implementation, V::Iri(&item.implementation)),
            ]),
            Self::ParamMapping(item) => values.extend([
                (P::Label, V::Text(&item.label)),
                (P::FunctionParameter, V::Iri(&item.function_parameter)),
                (P::Binding, V::Plain(&item.implementation_property)),
            ]),
            Self::ReturnMapping(item) => values.extend([
                (P::Label, V::Text(&item.label)),
                (P::FunctionOutput, V::Iri(&item.function_output)),
                (P::Binding, V::Plain(&item.implementation_property)),
            ]),
        }
        values
    }

    fn class(self) -> &'static str {
        match self {
            Self::Document(_) => OWL_ONTOLOGY,
            Self::Function(_) => "https://w3id.org/function/ontology#Function",
            Self::Output(_) => "https://w3id.org/function/ontology#Output",
            Self::Param(_) => "https://w3id.org/function/ontology#Parameter",
            Self::Implementation(_) => "https://w3id.org/function/ontology#Implementation",
            Self::Mapping(_) => "https://w3id.org/function/ontology#Mapping",
            Self::ParamMapping(_) => {
                "https://w3id.org/function/vocabulary/mapping#PropertyParameterMapping"
            }
            Self::ReturnMapping(_) => {
                "https://w3id.org/function/vocabulary/mapping#DefaultReturnMapping"
            }
        }
    }
}

fn insert<'a>(
    nodes: &mut Nodes<'a>,
    subject: TermValue,
    node: Node<'a>,
) -> Result<(), Box<FnoAdmissionReason>> {
    let properties = node.properties();
    if let Some(previous) = nodes.get(&subject) {
        for previous in previous {
            if previous.properties().iter().any(|(property, value)| {
                properties.iter().any(|(other_property, other_value)| {
                    property == other_property && !value.same(*other_value)
                })
            }) {
                return Err(Box::new(FnoAdmissionReason::ConflictingNode { subject }));
            }
        }
    }
    nodes.entry(subject).or_default().push(node);
    Ok(())
}

fn reference(
    subject: &TermValue,
    predicate: &str,
    target: &str,
    valid: bool,
) -> Result<(), Box<FnoAdmissionReason>> {
    if valid {
        Ok(())
    } else {
        Err(Box::new(FnoAdmissionReason::Reference {
            subject: subject.clone(),
            predicate: predicate.to_owned(),
            target: target.to_owned(),
        }))
    }
}

pub(super) fn validate(
    data: &FnoCatalogInput,
) -> Result<BTreeMap<TermValue, Shared>, Box<FnoAdmissionReason>> {
    let doc = TermValue::iri(&data.document_iri);
    iri(&doc, RDF_TYPE, &data.document_iri)?;
    iri(&doc, DCTERMS_IS_PART_OF, &data.ontology_iri)?;
    text(&doc, RDFS_LABEL, &data.doc_label)?;
    text(&doc, RDFS_COMMENT, &data.banner)?;
    let mut nodes = BTreeMap::new();
    insert(&mut nodes, doc, Node::Document(data))?;
    for function in &data.functions {
        let subject = TermValue::iri(&function.iri);
        iri(&subject, RDF_TYPE, &function.iri)?;
        text(&subject, RDFS_LABEL, &function.label)?;
        optional_text(&subject, SKOS_DEFINITION, function.description.as_ref())?;
        for kind in &function.kind_types {
            iri(&subject, RDF_TYPE, kind)?;
        }
        if let Some(related) = &function.see_also {
            iri(&subject, RDFS_SEE_ALSO, related)?;
        }
        insert(&mut nodes, subject, Node::Function(function))?;
        let output = &function.output;
        let subject = TermValue::iri(&output.iri);
        iri(&subject, RDF_TYPE, &output.iri)?;
        iri(&subject, &format!("{FNO}type"), &output.r#type)?;
        if let Some(predicate) = &output.predicate {
            iri(&subject, &format!("{FNO}predicate"), predicate)?;
        }
        optional_text(&subject, RDFS_LABEL, output.label.as_ref())?;
        optional_text(&subject, SKOS_DEFINITION, output.description.as_ref())?;
        insert(&mut nodes, subject, Node::Output(output))?;
    }
    for param in &data.params {
        let subject = TermValue::iri(&param.iri);
        iri(&subject, RDF_TYPE, &param.iri)?;
        iri(&subject, &format!("{FNO}type"), &param.r#type)?;
        if let Some(predicate) = &param.predicate {
            iri(&subject, &format!("{FNO}predicate"), predicate)?;
        }
        optional_text(&subject, RDFS_LABEL, param.label.as_ref())?;
        optional_text(&subject, SKOS_DEFINITION, param.description.as_ref())?;
        insert(&mut nodes, subject, Node::Param(param))?;
    }
    for implementation in &data.implementations {
        let subject = TermValue::iri(&implementation.iri);
        iri(&subject, RDF_TYPE, &implementation.iri)?;
        iri(&subject, RDFS_SEE_ALSO, &implementation.see_also)?;
        insert(&mut nodes, subject, Node::Implementation(implementation))?;
    }
    for mapping in &data.mappings {
        let subject = mapping.node.term_value();
        text(&subject, RDFS_LABEL, &mapping.label)?;
        insert(&mut nodes, subject, Node::Mapping(mapping))?;
        for item in &mapping.parameter_mappings {
            let subject = item.node.term_value();
            text(&subject, RDFS_LABEL, &item.label)?;
            insert(&mut nodes, subject, Node::ParamMapping(item))?;
        }
        for item in &mapping.return_mappings {
            let subject = item.node.term_value();
            text(&subject, RDFS_LABEL, &item.label)?;
            insert(&mut nodes, subject, Node::ReturnMapping(item))?;
        }
    }
    for function in &data.functions {
        let subject = TermValue::iri(&function.iri);
        for kind in &function.kind_types {
            if (kind.starts_with(FNO) || kind.starts_with(FNOM))
                && !nodes[&subject].iter().any(|node| node.class() == kind)
            {
                return Err(Box::new(FnoAdmissionReason::ConflictingNode { subject }));
            }
        }
        for target in &function.expects {
            reference(
                &TermValue::iri(&function.iri),
                &format!("{FNO}expects"),
                target,
                nodes
                    .get(&TermValue::iri(target))
                    .is_some_and(|items| items.iter().any(|node| matches!(node, Node::Param(_)))),
            )?;
        }
    }
    for mapping in &data.mappings {
        let subject = mapping.node.term_value();
        let function = nodes
            .get(&TermValue::iri(&mapping.function))
            .and_then(|items| {
                items.iter().find_map(|node| match node {
                    Node::Function(function) => Some(*function),
                    _ => None,
                })
            });
        reference(
            &subject,
            &format!("{FNO}function"),
            &mapping.function,
            function.is_some(),
        )?;
        reference(
            &subject,
            &format!("{FNO}implementation"),
            &mapping.implementation,
            nodes
                .get(&TermValue::iri(&mapping.implementation))
                .is_some_and(|items| {
                    items
                        .iter()
                        .any(|node| matches!(node, Node::Implementation(_)))
                }),
        )?;
        let Some(function) = function else {
            unreachable!("reference was checked")
        };
        for item in &mapping.parameter_mappings {
            reference(
                &item.node.term_value(),
                &format!("{FNOM}functionParameter"),
                &item.function_parameter,
                function.expects.contains(&item.function_parameter),
            )?;
        }
        for item in &mapping.return_mappings {
            reference(
                &item.node.term_value(),
                &format!("{FNOM}functionOutput"),
                &item.function_output,
                function.output.iri == item.function_output,
            )?;
        }
    }
    Ok(shared_facts(&nodes))
}

/// Only genuinely shared subjects need copied optional facts at canonicalization.
#[derive(Default)]
pub(super) struct Shared {
    label: Option<TermValue>,
    description: Option<TermValue>,
    predicate: Option<String>,
    see_also: Option<String>,
    kinds: BTreeSet<String>,
    classes: BTreeSet<&'static str>,
}

fn shared_facts(nodes: &Nodes<'_>) -> BTreeMap<TermValue, Shared> {
    let mut shared = BTreeMap::new();
    for (subject, nodes) in nodes.iter().filter(|(_, nodes)| nodes.len() > 1) {
        let mut facts = Shared::default();
        for node in nodes {
            facts.classes.insert(node.class());
            if let Node::Function(function) = node {
                facts.kinds.extend(function.kind_types.iter().cloned());
            }
            for (property, value) in node.properties() {
                match (property, value) {
                    (Property::Label, Value::Text(value)) => {
                        facts.label.get_or_insert_with(|| value.clone());
                    }
                    (Property::Description, Value::Text(value)) => {
                        facts.description.get_or_insert_with(|| value.clone());
                    }
                    (Property::Predicate, Value::Iri(value)) => {
                        facts.predicate.get_or_insert_with(|| value.to_owned());
                    }
                    (Property::SeeAlso, Value::Iri(value)) => {
                        facts.see_also.get_or_insert_with(|| value.to_owned());
                    }
                    _ => {}
                }
            }
        }
        shared.insert(subject.clone(), facts);
    }
    shared
}

fn canonical_text(value: &mut TermValue) {
    if let TermValue::Literal {
        language: Some(language),
        ..
    } = value
    {
        *language = purrdf_iri::langtag::identity_fold(language);
    }
}

fn canonical_optional(value: Option<&mut TermValue>) {
    if let Some(value) = value {
        canonical_text(value);
    }
}

pub(super) fn canonicalize(data: &mut FnoCatalogInput, shared: &BTreeMap<TermValue, Shared>) {
    canonical_text(&mut data.doc_label);
    canonical_text(&mut data.banner);
    for function in &mut data.functions {
        if let Some(facts) = shared.get(&TermValue::iri(&function.iri)) {
            if function.description.is_none() {
                function.description.clone_from(&facts.description);
            }
            if function.see_also.is_none() {
                function.see_also.clone_from(&facts.see_also);
            }
            function.kind_types.extend(
                facts
                    .kinds
                    .iter()
                    .filter(|kind| !facts.classes.contains(kind.as_str()))
                    .cloned(),
            );
        }
        canonical_text(&mut function.label);
        canonical_optional(function.description.as_mut());
        function.kind_types.retain(|kind| {
            !(kind.starts_with(FNO) || kind.starts_with(FNOM))
                && shared
                    .get(&TermValue::iri(&function.iri))
                    .is_none_or(|facts| !facts.classes.contains(kind.as_str()))
        });
        function.kind_types.sort();
        function.kind_types.dedup();
        if let Some(facts) = shared.get(&TermValue::iri(&function.output.iri)) {
            if function.output.label.is_none() {
                function.output.label.clone_from(&facts.label);
            }
            if function.output.description.is_none() {
                function.output.description.clone_from(&facts.description);
            }
            if function.output.predicate.is_none() {
                function.output.predicate.clone_from(&facts.predicate);
            }
        }
        canonical_optional(function.output.label.as_mut());
        canonical_optional(function.output.description.as_mut());
    }
    for param in &mut data.params {
        if let Some(facts) = shared.get(&TermValue::iri(&param.iri)) {
            if param.label.is_none() {
                param.label.clone_from(&facts.label);
            }
            if param.description.is_none() {
                param.description.clone_from(&facts.description);
            }
            if param.predicate.is_none() {
                param.predicate.clone_from(&facts.predicate);
            }
        }
        canonical_optional(param.label.as_mut());
        canonical_optional(param.description.as_mut());
    }
    for mapping in &mut data.mappings {
        canonical_text(&mut mapping.label);
        for item in &mut mapping.parameter_mappings {
            canonical_text(&mut item.label);
        }
        for item in &mut mapping.return_mappings {
            canonical_text(&mut item.label);
        }
        mapping.parameter_mappings.sort();
        mapping.parameter_mappings.dedup();
        mapping.return_mappings.sort();
        mapping.return_mappings.dedup();
    }
    data.functions.sort();
    data.functions.dedup();
    data.params.sort();
    data.params.dedup();
    data.implementations.sort();
    data.implementations.dedup();
    data.mappings.sort();
    data.mappings.dedup_by(|later, earlier| {
        if later.node != earlier.node {
            return false;
        }
        earlier
            .parameter_mappings
            .append(&mut later.parameter_mappings);
        earlier.return_mappings.append(&mut later.return_mappings);
        earlier.parameter_mappings.sort();
        earlier.parameter_mappings.dedup();
        earlier.return_mappings.sort();
        earlier.return_mappings.dedup();
        true
    });
}
