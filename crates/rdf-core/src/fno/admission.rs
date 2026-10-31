// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed admission followed by canonical ownership transfer.

use super::construction::{Context, PhysicalResult, Result as ConstructionResult, RetainedFields};
use super::{
    DCTERMS_IS_PART_OF, FNO, FNOM, FnFunction, FnImpl, FnMapping, FnOutput, FnParam,
    FnParamMapping, FnReturnMapping, FnoCatalogInput, OWL_ONTOLOGY, RDF_TYPE, RDFS_COMMENT,
    RDFS_LABEL, RDFS_SEE_ALSO, SKOS_DEFINITION,
};
use crate::TermValue;
use purrdf_lex::allocation::{Admission, OrderedTable};
use purrdf_lex::control::Progress;
type AdmitResult<T, E> = ConstructionResult<T, FnoAdmissionReason, E>;

/// Original catalog data and the native reason admission refused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnoAdmissionError {
    /// Original rejected records, strings, scopes and input ordering.
    pub input: FnoCatalogInput,
    /// Original semantic refusal and its location in the catalog.
    pub reason:
        super::FnoConstructionFailure<purrdf_lex::allocation::SharedOwned<FnoAdmissionReason>>,
}

/// Original reconstructed fields and the native semantic refusal of a reader.
/// The enclosing shared reader error retains one account for both inline values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnoRejectedCatalog {
    /// Exact original reconstructed records and their ordering.
    pub input: FnoCatalogInput,
    /// Original semantic reason and operands.
    pub reason: FnoAdmissionReason,
}
impl purrdf_hash::error::StructuredError for FnoRejectedCatalog {
    purrdf_hash::error_visit! {
        @record self {input,reason} => "fno-admission" {
            "input"=>Record(&self.input),"reason"=>Cause(&self.reason),
        }
    }
}
impl core::fmt::Display for FnoRejectedCatalog {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.reason.fmt(out)
    }
}
purrdf_lex::error_source!(FnoRejectedCatalog => {Self {reason,..}=>reason});

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
            "input" => Record(&self.input), "reason" => Cause(match &self.reason {
                super::FnoConstructionFailure::Semantic(reason) => &**reason as &dyn purrdf_hash::error::StructuredError,
                super::FnoConstructionFailure::Storage(cause) => cause,
                super::FnoConstructionFailure::Source(never)=>match *never {},
            }),
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
impl std::error::Error for FnoAdmissionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.reason {
            super::FnoConstructionFailure::Semantic(reason) => Some(&**reason),
            super::FnoConstructionFailure::Storage(cause) => Some(cause),
            super::FnoConstructionFailure::Source(never) => match *never {},
        }
    }
}

fn iri<S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    subject: &Key<'_>,
    predicate: &str,
    value: &str,
) -> AdmitResult<(), C::Error> {
    if let Err(cause) = context.absolute(value)? {
        let reason = FnoAdmissionReason::Iri {
            subject: subject.term(context)?,
            predicate: context.string(predicate)?,
            value: context.string(value)?,
            cause,
        };
        return context.refuse(reason);
    }
    Ok(())
}

fn text<S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    subject: &Key<'_>,
    predicate: &str,
    value: &TermValue,
) -> AdmitResult<(), C::Error> {
    if !matches!(value, TermValue::Literal { .. }) {
        let reason = FnoAdmissionReason::ExpectedLiteral {
            subject: subject.term(context)?,
            predicate: context.string(predicate)?,
            value: context.term(value)?,
        };
        return context.refuse(reason);
    }
    if let Err(cause) = context.literal(value)? {
        let reason = FnoAdmissionReason::Literal {
            subject: subject.term(context)?,
            predicate: context.string(predicate)?,
            cause,
        };
        return context.refuse(reason);
    }
    Ok(())
}

fn optional_text<S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    subject: &Key<'_>,
    predicate: &str,
    value: Option<&TermValue>,
) -> AdmitResult<(), C::Error> {
    if let Some(value) = value {
        text(context, subject, predicate, value)?;
    }
    Ok(())
}

fn same_text<S: Admission, C: Progress>(
    first: &TermValue,
    second: &TermValue,
    context: &mut Context<S, C>,
) -> AdmitResult<bool, C::Error> {
    context.check(1)?;
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
            if !context.compare(a, b)?.is_eq() {
                return Ok(false);
            }
            if !context.compare(ad, bd)?.is_eq() {
                return Ok(false);
            }
            context.check(1)?;
            if ax != bx {
                return Ok(false);
            }
            context.check(1)?;
            match (al, bl) {
                (None, None) => Ok(true),
                (None, Some(_)) | (Some(_), None) => Ok(false),
                (Some(a), Some(b)) => {
                    // Identity folding is ASCII case only. Compare bounded native
                    // chunks without allocating whole folded language strings.
                    context.check(1)?;
                    if a.len() != b.len() {
                        return Ok(false);
                    }
                    for (a, b) in a.as_bytes().chunks(256).zip(b.as_bytes().chunks(256)) {
                        context
                            .check(u64::try_from(a.len()).expect("bounded language comparison"))?;
                        if !a.eq_ignore_ascii_case(b) {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
            }
        }
        _ => Ok(context.compare(first, second)?.is_eq()),
    }
}

/// Borrowed native subjects remain tied to immutable original input while a
/// separate admitted staging copy is canonicalized. Both variants preserve the
/// native IRI-before-blank ordering and exact label/scope identity.
#[derive(Clone, Copy)]
pub(super) enum Key<'a> {
    Iri(&'a str),
    Blank(&'a super::FnBlank),
}
impl Key<'_> {
    fn term<S: Admission, C: Progress, N>(
        &self,
        context: &mut Context<S, C>,
    ) -> ConstructionResult<TermValue, N, C::Error> {
        match self {
            Self::Iri(value) => context.iri(value),
            Self::Blank(value) => Ok(TermValue::Blank {
                label: context.string(&value.label)?,
                scope: value.scope,
            }),
        }
    }
}
impl super::construction::CompareFields for Key<'_> {
    fn compare_fields<S: Admission, C: Progress>(
        &self,
        other: &Self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
        progress: &mut C,
    ) -> Result<
        core::cmp::Ordering,
        purrdf_lex::control::ControlledError<purrdf_lex::allocation::StorageError, C::Error>,
    > {
        progress
            .charge(1)
            .map_err(purrdf_lex::control::ControlledError::Control)?;
        match (self, other) {
            (Self::Iri(a), Self::Iri(b)) => a.compare_fields(b, memory, progress),
            (Self::Blank(a), Self::Blank(b)) => a.compare_fields(b, memory, progress),
            (Self::Iri(_), Self::Blank(_)) => Ok(core::cmp::Ordering::Less),
            (Self::Blank(_), Self::Iri(_)) => Ok(core::cmp::Ordering::Greater),
        }
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

type Nodes<'a> = OrderedTable<Key<'a>, Vec<Node<'a>>>;

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
    fn same<S: Admission, C: Progress>(
        self,
        other: Self,
        context: &mut Context<S, C>,
    ) -> AdmitResult<bool, C::Error> {
        match (self, other) {
            (Self::Text(a), Self::Text(b)) => same_text(a, b, context),
            (Self::Iri(a), Self::Iri(b)) | (Self::Plain(a), Self::Plain(b)) => {
                Ok(context.compare(a, b)?.is_eq())
            }
            (Self::Boolean(a), Self::Boolean(b)) => {
                context.check(1)?;
                Ok(a == b)
            }
            (Self::List(a), Self::List(b)) => {
                context.check(1)?;
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (a, b) in a.iter().zip(b) {
                    if !context.compare(a, b)?.is_eq() {
                        return Ok(false);
                    }
                }
                context.check(1)?;
                Ok(a.len() == b.len())
            }
            _ => {
                context.check(1)?;
                Ok(false)
            }
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

fn insert<'a, S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    nodes: &mut Nodes<'a>,
    subject: Key<'a>,
    node: Node<'a>,
) -> AdmitResult<(), C::Error> {
    let properties = node.properties();
    if let Some(previous) = context.lookup(nodes, &subject)? {
        for previous in previous {
            for (property, value) in previous.properties() {
                for (other_property, other_value) in &properties {
                    context.check(1)?;
                    if property == *other_property && !value.same(*other_value, context)? {
                        let subject = subject.term(context)?;
                        return context.refuse(FnoAdmissionReason::ConflictingNode { subject });
                    }
                }
            }
        }
    }
    let mut items = context
        .insert(nodes, subject, Vec::new())?
        .unwrap_or_default();
    context.push(&mut items, node)?;
    context.insert(nodes, subject, items)?;
    Ok(())
}

fn reference<S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    subject: &Key<'_>,
    predicate: &str,
    target: &str,
    valid: bool,
) -> AdmitResult<(), C::Error> {
    context.check(1)?;
    if !valid {
        let reason = FnoAdmissionReason::Reference {
            subject: subject.term(context)?,
            predicate: context.string(predicate)?,
            target: context.string(target)?,
        };
        return context.refuse(reason);
    }
    Ok(())
}

pub(super) fn validate<'a, S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    data: &'a FnoCatalogInput,
) -> AdmitResult<OrderedTable<Key<'a>, Shared>, C::Error> {
    let doc = Key::Iri(&data.document_iri);
    iri(context, &doc, RDF_TYPE, &data.document_iri)?;
    iri(context, &doc, DCTERMS_IS_PART_OF, &data.ontology_iri)?;
    text(context, &doc, RDFS_LABEL, &data.doc_label)?;
    text(context, &doc, RDFS_COMMENT, &data.banner)?;
    let mut nodes = OrderedTable::default();
    insert(context, &mut nodes, doc, Node::Document(data))?;
    for function in &data.functions {
        context.check(1)?;
        let subject = Key::Iri(&function.iri);
        iri(context, &subject, RDF_TYPE, &function.iri)?;
        text(context, &subject, RDFS_LABEL, &function.label)?;
        optional_text(
            context,
            &subject,
            SKOS_DEFINITION,
            function.description.as_ref(),
        )?;
        for kind in &function.kind_types {
            iri(context, &subject, RDF_TYPE, kind)?;
        }
        if let Some(related) = &function.see_also {
            iri(context, &subject, RDFS_SEE_ALSO, related)?;
        }
        insert(context, &mut nodes, subject, Node::Function(function))?;
        let output = &function.output;
        let subject = Key::Iri(&output.iri);
        iri(context, &subject, RDF_TYPE, &output.iri)?;
        iri(context, &subject, fno!("type"), &output.r#type)?;
        if let Some(predicate) = &output.predicate {
            iri(context, &subject, fno!("predicate"), predicate)?;
        }
        optional_text(context, &subject, RDFS_LABEL, output.label.as_ref())?;
        optional_text(
            context,
            &subject,
            SKOS_DEFINITION,
            output.description.as_ref(),
        )?;
        insert(context, &mut nodes, subject, Node::Output(output))?;
    }
    for param in &data.params {
        context.check(1)?;
        let subject = Key::Iri(&param.iri);
        iri(context, &subject, RDF_TYPE, &param.iri)?;
        iri(context, &subject, fno!("type"), &param.r#type)?;
        if let Some(predicate) = &param.predicate {
            iri(context, &subject, fno!("predicate"), predicate)?;
        }
        optional_text(context, &subject, RDFS_LABEL, param.label.as_ref())?;
        optional_text(
            context,
            &subject,
            SKOS_DEFINITION,
            param.description.as_ref(),
        )?;
        insert(context, &mut nodes, subject, Node::Param(param))?;
    }
    for implementation in &data.implementations {
        context.check(1)?;
        let subject = Key::Iri(&implementation.iri);
        iri(context, &subject, RDF_TYPE, &implementation.iri)?;
        iri(context, &subject, RDFS_SEE_ALSO, &implementation.see_also)?;
        insert(
            context,
            &mut nodes,
            subject,
            Node::Implementation(implementation),
        )?;
    }
    for mapping in &data.mappings {
        context.check(1)?;
        let subject = Key::Blank(&mapping.node);
        text(context, &subject, RDFS_LABEL, &mapping.label)?;
        insert(context, &mut nodes, subject, Node::Mapping(mapping))?;
        for item in &mapping.parameter_mappings {
            let subject = Key::Blank(&item.node);
            text(context, &subject, RDFS_LABEL, &item.label)?;
            insert(context, &mut nodes, subject, Node::ParamMapping(item))?;
        }
        for item in &mapping.return_mappings {
            let subject = Key::Blank(&item.node);
            text(context, &subject, RDFS_LABEL, &item.label)?;
            insert(context, &mut nodes, subject, Node::ReturnMapping(item))?;
        }
    }
    for function in &data.functions {
        let subject = Key::Iri(&function.iri);
        let roles = context
            .lookup(&nodes, &subject)?
            .expect("function was indexed");
        for kind in &function.kind_types {
            context.check(1)?;
            if kind.starts_with(FNO) || kind.starts_with(FNOM) {
                let mut found = false;
                for node in roles {
                    if context.compare(node.class(), kind.as_str())?.is_eq() {
                        found = true;
                        break;
                    }
                }
                if !found {
                    let subject = subject.term(context)?;
                    return context.refuse(FnoAdmissionReason::ConflictingNode { subject });
                }
            }
        }
        for target in &function.expects {
            let key = Key::Iri(target);
            let mut valid = false;
            if let Some(items) = context.lookup(&nodes, &key)? {
                for node in items {
                    context.check(1)?;
                    if matches!(node, Node::Param(_)) {
                        valid = true;
                        break;
                    }
                }
            }
            reference(context, &subject, fno!("expects"), target, valid)?;
        }
    }
    for mapping in &data.mappings {
        let subject = Key::Blank(&mapping.node);
        let key = Key::Iri(&mapping.function);
        let mut function = None;
        if let Some(items) = context.lookup(&nodes, &key)? {
            for node in items {
                context.check(1)?;
                if let Node::Function(value) = node {
                    function = Some(*value);
                    break;
                }
            }
        }
        reference(
            context,
            &subject,
            fno!("function"),
            &mapping.function,
            function.is_some(),
        )?;
        let key = Key::Iri(&mapping.implementation);
        let mut implementation = false;
        if let Some(items) = context.lookup(&nodes, &key)? {
            for node in items {
                context.check(1)?;
                if matches!(node, Node::Implementation(_)) {
                    implementation = true;
                    break;
                }
            }
        }
        reference(
            context,
            &subject,
            fno!("implementation"),
            &mapping.implementation,
            implementation,
        )?;
        let function = function.expect("the reference was checked");
        for item in &mapping.parameter_mappings {
            let mut valid = false;
            for target in &function.expects {
                if context.compare(target, &item.function_parameter)?.is_eq() {
                    valid = true;
                    break;
                }
            }
            let node = Key::Blank(&item.node);
            reference(
                context,
                &node,
                fnom!("functionParameter"),
                &item.function_parameter,
                valid,
            )?;
        }
        for item in &mapping.return_mappings {
            let valid = context
                .compare(&function.output.iri, &item.function_output)?
                .is_eq();
            let node = Key::Blank(&item.node);
            reference(
                context,
                &node,
                fnom!("functionOutput"),
                &item.function_output,
                valid,
            )?;
        }
    }
    let shared = shared_facts(context, &nodes)?;
    context.release(nodes)?;
    Ok(shared)
}

#[derive(Default)]
pub(super) struct Shared {
    label: Option<TermValue>,
    description: Option<TermValue>,
    predicate: Option<String>,
    see_also: Option<String>,
    kinds: OrderedTable<String, ()>,
    classes: OrderedTable<&'static str, ()>,
}

fn shared_facts<'a, S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    nodes: &Nodes<'a>,
) -> AdmitResult<OrderedTable<Key<'a>, Shared>, C::Error> {
    let mut shared = OrderedTable::default();
    for (subject, nodes) in nodes.ordered() {
        context.check(1)?;
        if nodes.len() <= 1 {
            continue;
        }
        let mut facts = Shared::default();
        for node in nodes {
            context.insert(&mut facts.classes, node.class(), ())?;
            if let Node::Function(function) = node {
                for kind in &function.kind_types {
                    let kind = context.string(kind)?;
                    context.insert(&mut facts.kinds, kind, ())?;
                }
            }
            for (property, value) in node.properties() {
                context.check(1)?;
                match (property, value) {
                    (Property::Label, Value::Text(value)) if facts.label.is_none() => {
                        facts.label = Some(context.term(value)?)
                    }
                    (Property::Description, Value::Text(value)) if facts.description.is_none() => {
                        facts.description = Some(context.term(value)?)
                    }
                    (Property::Predicate, Value::Iri(value)) if facts.predicate.is_none() => {
                        facts.predicate = Some(context.string(value)?)
                    }
                    (Property::SeeAlso, Value::Iri(value)) if facts.see_also.is_none() => {
                        facts.see_also = Some(context.string(value)?)
                    }
                    _ => {}
                }
            }
        }
        context.insert(&mut shared, *subject, facts)?;
    }
    Ok(shared)
}

fn canonical_text<S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    value: &mut TermValue,
) -> AdmitResult<(), C::Error> {
    if let TermValue::Literal {
        language: Some(language),
        ..
    } = value
    {
        context.check(u64::try_from(language.len()).map_err(|_| {
            purrdf_lex::control::ControlledError::Native(
                super::construction::FnoConstructionFailure::Storage(
                    purrdf_lex::allocation::StorageError::SizeOverflow,
                ),
            )
        })?)?;
        language.make_ascii_lowercase();
    }
    Ok(())
}

fn canonical_optional<S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    value: Option<&mut TermValue>,
) -> AdmitResult<(), C::Error> {
    if let Some(value) = value {
        canonical_text(context, value)?;
    }
    Ok(())
}

pub(super) fn canonicalize<S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    data: &mut FnoCatalogInput,
    shared: &OrderedTable<Key<'_>, Shared>,
) -> AdmitResult<(), C::Error> {
    canonical_text(context, &mut data.doc_label)?;
    canonical_text(context, &mut data.banner)?;
    for function in &mut data.functions {
        context.check(1)?;
        let key = Key::Iri(&function.iri);
        let facts = context.lookup(shared, &key)?;
        if let Some(facts) = facts {
            if function.description.is_none() {
                function.description = context.copy(&facts.description)?;
            }
            if function.see_also.is_none() {
                function.see_also = context.copy(&facts.see_also)?;
            }
            for (kind, ()) in facts.kinds.ordered() {
                let mut structural = false;
                for (class, ()) in facts.classes.ordered() {
                    if context.compare(kind.as_str(), *class)?.is_eq() {
                        structural = true;
                        break;
                    }
                }
                if !structural {
                    let kind = context.string(kind)?;
                    context.push(&mut function.kind_types, kind)?;
                }
            }
        }
        canonical_text(context, &mut function.label)?;
        canonical_optional(context, function.description.as_mut())?;
        let mut kept = Vec::new();
        let mut original_kinds = core::mem::take(&mut function.kind_types);
        for kind in original_kinds.drain(..) {
            context.check(1)?;
            let mut structural = kind.starts_with(FNO) || kind.starts_with(FNOM);
            if let Some(facts) = facts {
                for (class, ()) in facts.classes.ordered() {
                    if context.compare(kind.as_str(), *class)?.is_eq() {
                        structural = true;
                        break;
                    }
                }
            }
            if !structural {
                context.push(&mut kept, kind)?;
            } else {
                context.release(kind)?;
            }
        }
        context.release(original_kinds)?;
        function.kind_types = kept;
        context.sort(&mut function.kind_types)?;
        context.dedup(&mut function.kind_types)?;
        let key = Key::Iri(&function.output.iri);
        if let Some(facts) = context.lookup(shared, &key)? {
            if function.output.label.is_none() {
                function.output.label = context.copy(&facts.label)?;
            }
            if function.output.description.is_none() {
                function.output.description = context.copy(&facts.description)?;
            }
            if function.output.predicate.is_none() {
                function.output.predicate = context.copy(&facts.predicate)?;
            }
        }
        canonical_optional(context, function.output.label.as_mut())?;
        canonical_optional(context, function.output.description.as_mut())?;
    }
    for param in &mut data.params {
        let key = Key::Iri(&param.iri);
        if let Some(facts) = context.lookup(shared, &key)? {
            if param.label.is_none() {
                param.label = context.copy(&facts.label)?;
            }
            if param.description.is_none() {
                param.description = context.copy(&facts.description)?;
            }
            if param.predicate.is_none() {
                param.predicate = context.copy(&facts.predicate)?;
            }
        }
        canonical_optional(context, param.label.as_mut())?;
        canonical_optional(context, param.description.as_mut())?;
    }
    for mapping in &mut data.mappings {
        canonical_text(context, &mut mapping.label)?;
        for item in &mut mapping.parameter_mappings {
            canonical_text(context, &mut item.label)?;
        }
        for item in &mut mapping.return_mappings {
            canonical_text(context, &mut item.label)?;
        }
        context.sort(&mut mapping.parameter_mappings)?;
        context.dedup(&mut mapping.parameter_mappings)?;
        context.sort(&mut mapping.return_mappings)?;
        context.dedup(&mut mapping.return_mappings)?;
    }
    context.sort(&mut data.functions)?;
    context.dedup(&mut data.functions)?;
    context.sort(&mut data.params)?;
    context.dedup(&mut data.params)?;
    context.sort(&mut data.implementations)?;
    context.dedup(&mut data.implementations)?;
    context.sort(&mut data.mappings)?;
    // Equal native mapping subjects union their child sets in sorted encounter
    // order, exactly as the resident constructor did before publication.
    let mut mappings: Vec<FnMapping> = Vec::new();
    let mut original_mappings = core::mem::take(&mut data.mappings);
    for mut mapping in original_mappings.drain(..) {
        if let Some(previous) = mappings.last_mut() {
            if context.compare(&previous.node, &mapping.node)?.is_eq() {
                for item in mapping.parameter_mappings.drain(..) {
                    context.push(&mut previous.parameter_mappings, item)?;
                }
                for item in mapping.return_mappings.drain(..) {
                    context.push(&mut previous.return_mappings, item)?;
                }
                context.sort(&mut previous.parameter_mappings)?;
                context.dedup(&mut previous.parameter_mappings)?;
                context.sort(&mut previous.return_mappings)?;
                context.dedup(&mut previous.return_mappings)?;
                context.release(mapping)?;
                continue;
            }
        }
        context.push(&mut mappings, mapping)?;
    }
    context.release(original_mappings)?;
    data.mappings = mappings;
    Ok(())
}

pub(super) fn construct<S: Admission, C: Progress>(
    context: &mut Context<S, C>,
    original: &FnoCatalogInput,
) -> AdmitResult<FnoCatalogInput, C::Error> {
    let shared = validate(context, original)?;
    let mut staged = context.copy(original)?;
    canonicalize(context, &mut staged, &shared)?;
    context.release(shared)?;
    Ok(staged)
}

impl RetainedFields for Key<'_> {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        _: &mut purrdf_lex::allocation::Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        progress
            .charge(1)
            .map_err(purrdf_lex::control::ControlledError::Control)?;
        Ok(0)
    }
}
impl RetainedFields for Node<'_> {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        _: &mut purrdf_lex::allocation::Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        progress
            .charge(1)
            .map_err(purrdf_lex::control::ControlledError::Control)?;
        Ok(0)
    }
}
impl RetainedFields for Shared {
    fn retained_fields<S: Admission, C: Progress>(
        &self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
        progress: &mut C,
    ) -> PhysicalResult<usize, C::Error> {
        let mut bytes = 0usize;
        for amount in [
            self.label.retained_fields(memory, progress)?,
            self.description.retained_fields(memory, progress)?,
            self.predicate.retained_fields(memory, progress)?,
            self.see_also.retained_fields(memory, progress)?,
            self.kinds.retained_fields(memory, progress)?,
            self.classes.retained_fields(memory, progress)?,
        ] {
            bytes =
                bytes
                    .checked_add(amount)
                    .ok_or(purrdf_lex::control::ControlledError::Native(
                        purrdf_lex::allocation::StorageError::SizeOverflow,
                    ))?;
        }
        Ok(bytes)
    }
}
