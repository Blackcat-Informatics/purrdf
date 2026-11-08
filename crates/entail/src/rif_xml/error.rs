// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original XML, IRI and structural causes of RIF document refusal.

use crate::rif::RifVariableError;
use purrdf_iri::{IriError, ScopedBase};
use purrdf_lex::xml::XmlError;

/// The IRI operation that refused an original RIF operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RifIriPhase {
    /// Admit the caller's initial absolute base.
    CallerBase,
    /// Apply an element's `xml:base` directive.
    ElementBase,
    /// Resolve IRI-valued content or an attribute.
    Reference,
}

impl purrdf_hash::error::StructuredError for RifIriPhase {
    purrdf_hash::error_visit! {
        CallerBase => "rif-iri-caller-base" {},
        ElementBase => "rif-iri-element-base" {},
        Reference => "rif-iri-reference" {},
    }
}

/// The required component absent from an authored RIF construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RifComponent {
    /// `Import` location.
    ImportLocation,
    /// `Forall` formula.
    ForallFormula,
    /// `Implies` body.
    ImpliesIf,
    /// `Implies` head.
    ImpliesThen,
    /// `Frame` subject.
    FrameObject,
    /// At least one `Frame` slot.
    FrameSlot,
    /// The first slot term.
    SlotPredicate,
    /// The second slot term.
    SlotValue,
    /// A constant's datatype attribute.
    ConstType,
}

impl purrdf_hash::error::StructuredError for RifComponent {
    purrdf_hash::error_visit! {
        ImportLocation => "rif-component-import-location" {},
        ForallFormula => "rif-component-forall-formula" {},
        ImpliesIf => "rif-component-implies-if" {},
        ImpliesThen => "rif-component-implies-then" {},
        FrameObject => "rif-component-frame-object" {},
        FrameSlot => "rif-component-frame-slot" {},
        SlotPredicate => "rif-component-slot-predicate" {},
        SlotValue => "rif-component-slot-value" {},
        ConstType => "rif-component-const-type" {},
    }
}

impl RifComponent {
    fn message(self) -> &'static str {
        match self {
            Self::ImportLocation => "Import without a <location>",
            Self::ForallFormula => "Forall without a <formula>",
            Self::ImpliesIf => "Implies without an <if>",
            Self::ImpliesThen => "Implies without a <then>",
            Self::FrameObject => "Frame without an <object>",
            Self::FrameSlot => "Frame without any <slot>",
            Self::SlotPredicate => "slot without a predicate",
            Self::SlotValue => "slot without a value",
            Self::ConstType => "Const without a type attribute",
        }
    }
}

/// A RIF document refusal before a rule set can be published.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RifXmlError {
    /// The native XML reader's complete original refusal and source coordinates.
    Xml {
        /// Original XML error, including its native byte offset and kind.
        cause: XmlError,
        /// One-based line in the original input.
        line: usize,
        /// One-based column in the original input.
        column: usize,
    },
    /// Original IRI refusal with the base and origin actually in force.
    Iri {
        /// The refusing operation.
        phase: RifIriPhase,
        /// The original reference or directive, before resolution.
        reference: String,
        /// The original current base and its origin, or explicit absence.
        base: Option<Box<ScopedBase>>,
        /// Original native IRI error.
        cause: IriError,
    },
    /// External DTD declarations remain unread and cannot silently be omitted.
    ExternalDeclarations,
    /// An element is not admitted as a child of this construct.
    UnexpectedChild {
        /// The native parent's local name.
        parent: &'static str,
        /// The original child's local name.
        element: String,
    },
    /// A sentence, conjunction or term has an unsupported element.
    UnexpectedNode {
        /// The native grammar position.
        position: &'static str,
        /// The original local name.
        element: String,
    },
    /// A construct has no required component.
    Missing(RifComponent),
    /// A single-element wrapper is empty.
    Empty {
        /// The grammar wrapper the reader was decoding.
        context: &'static str,
    },
    /// Enough children were observed to violate the wrapper's bound.
    TooManyChildren {
        /// The grammar wrapper or slot the reader was decoding.
        context: &'static str,
        /// Its admitted maximum.
        maximum: usize,
        /// The observed lower bound; the parser does not scan an unnecessary tail.
        observed_at_least: usize,
    },
    /// A required RIF element has another local name.
    ExpectedElement {
        /// The required local name.
        expected: &'static str,
        /// The original local name.
        actual: String,
    },
    /// An element is outside the required RIF namespace.
    Namespace {
        /// The original local name.
        element: String,
        /// Original namespace IRI, or explicit absence.
        actual: Option<String>,
    },
    /// A local constant occurs outside metadata, where this engine cannot interpret it.
    LocalConstant {
        /// Original resolved datatype IRI.
        datatype: String,
        /// Original decoded text value.
        value: String,
    },
    /// A constant names a datatype this engine cannot interpret.
    UnsupportedConstant {
        /// Original resolved datatype IRI.
        datatype: String,
        /// Original decoded text value.
        value: String,
    },
    /// A ground fact contains an original variable.
    Variable(RifVariableError),
}

impl purrdf_hash::error::StructuredError for RifXmlError {
    purrdf_hash::error_visit! {
        Xml { cause, line, column } => "rif-xml-reader" {
            "cause" => Cause(cause), "line" => Unsigned(*line as u128),
            "column" => Unsigned(*column as u128),
        },
        Iri { phase, reference, base, cause } => "rif-xml-iri" {
            "phase" => Record(phase), "reference" => Iri(reference),
            "base" => Value(base.as_deref().map_or(purrdf_hash::error::ErrorValue::Absent, |base| purrdf_hash::error::ErrorValue::Record(base))),
            "cause" => Cause(cause),
        },
        ExternalDeclarations => "rif-xml-external-declarations" {},
        UnexpectedChild { parent, element } => "rif-xml-unexpected-child" {
            "parent" => Text(parent), "element" => Text(element),
        },
        UnexpectedNode { position, element } => "rif-xml-unexpected-node" {
            "position" => Text(position), "element" => Text(element),
        },
        Missing(component) => "rif-xml-missing-component" { "component" => Record(component) },
        Empty { context } => "rif-xml-empty" { "context" => Text(context) },
        TooManyChildren { context, maximum, observed_at_least } => "rif-xml-too-many-children" {
            "context" => Text(context), "maximum" => Unsigned(*maximum as u128),
            "observed_at_least" => Unsigned(*observed_at_least as u128),
        },
        ExpectedElement { expected, actual } => "rif-xml-expected-element" {
            "expected" => Text(expected), "actual" => Text(actual),
        },
        Namespace { element, actual } => "rif-xml-namespace" {
            "element" => Text(element),
            "actual" => Value(actual.as_deref().map_or(purrdf_hash::error::ErrorValue::Absent, purrdf_hash::error::ErrorValue::Iri)),
        },
        LocalConstant { datatype, value } => "rif-xml-local-constant" {
            "datatype" => Iri(datatype), "value" => Text(value),
        },
        UnsupportedConstant { datatype, value } => "rif-xml-unsupported-constant" {
            "datatype" => Iri(datatype), "value" => Text(value),
        },
        Variable(cause) => "rif-xml-variable" { "cause" => Cause(cause) },
    }
}

purrdf_lex::variant_from!(RifXmlError { Variable(RifVariableError), Missing(RifComponent) });

impl core::fmt::Display for RifXmlError {
    fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Xml { cause, line, column } => write!(output, "{cause} (line {line}, column {column})"),
            Self::Iri { phase, reference, cause, .. } => match phase {
                RifIriPhase::CallerBase => write!(output, "invalid base IRI {reference:?}: {cause}"),
                RifIriPhase::ElementBase => write!(output, "invalid xml:base {reference:?}: {cause}"),
                RifIriPhase::Reference => write!(output, "{cause} [{}]", cause.diagnostic_code()),
            },
            Self::ExternalDeclarations => output.write_str("the document type declaration names an external DTD subset or references an external parameter entity, which is never fetched, so default attributes and entity declarations may be missing"),
            Self::UnexpectedChild { parent, element } => write!(output, "unexpected {parent} child <{element}>"),
            Self::UnexpectedNode { position, element } => write!(output, "unexpected {position} <{element}>"),
            Self::Missing(component) => output.write_str(component.message()),
            Self::Empty { context } => write!(output, "<{context}> is empty"),
            Self::TooManyChildren { context: "slot", .. } => output.write_str("slot with more than two children"),
            Self::TooManyChildren { context, .. } => write!(output, "<{context}> has more than one child element"),
            Self::ExpectedElement { expected, actual } => write!(output, "expected <{expected}>, found <{actual}>"),
            Self::Namespace { element, actual } => write!(output, "element <{element}> is not in the RIF namespace (found {actual:?})"),
            Self::LocalConstant { .. } => output.write_str("rif:local const outside <meta> is unsupported"),
            Self::UnsupportedConstant { datatype, .. } => write!(output, "unsupported Const type {datatype}"),
            Self::Variable(cause) => cause.fmt(output),
        }
    }
}

impl std::error::Error for RifXmlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Xml { cause, .. } => Some(cause),
            Self::Iri { cause, .. } => Some(cause),
            Self::Variable(cause) => Some(cause),
            _ => None,
        }
    }
}
