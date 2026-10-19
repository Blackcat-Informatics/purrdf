// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Results-document shape and token refusals, independent of presentation.

use crate::{Error, SparqlResultsFormat};
use purrdf_core::TermValue;
use purrdf_lex::allocation::{Admission, Memory};

/// A structural requirement imposed by the results-document grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadShape {
    /// The JSON document root must be an object.
    TopObject,
    /// The SELECT reader cannot decode an ASK document.
    Select,
    /// A required head object is absent or malformed.
    Head,
    /// A required results object or element is absent.
    Results,
    /// A required bindings array is absent or malformed.
    Bindings,
    /// A required Boolean field or element is absent.
    Boolean,
    /// A provenance solution must be an object.
    ProvenanceSolution,
    /// A provenance source must be a string.
    ProvenanceSource,
    /// A bindings-array entry must be an object.
    BindingEntry,
    /// A projected variable must be a string.
    Variable,
    /// A triple term has no subject.
    Subject,
    /// A triple term has no predicate.
    Predicate,
    /// A triple term has no object.
    Object,
    /// A binding requires a string type member.
    BindingType,
    /// A triple binding requires an object value.
    TripleValue,
    /// A binding must be an object.
    BindingObject,
    /// A binding requires a string value member.
    BindingValue,
    /// The reader requires an array.
    Array,
    /// An XML binding requires a name attribute.
    BindingName,
    /// The XML document requires a sparql root element.
    SparqlRoot,
    /// An XML triple lacks its component wrapper.
    Component {
        /// Required component role.
        role: &'static str,
    },
    /// A component wrapper contains no term.
    ComponentTerm {
        /// Original component role.
        role: &'static str,
    },
}

impl purrdf_hash::error::StructuredError for ReadShape {
    purrdf_hash::error_visit! {
        TopObject => "results-read-top-object" {},
        Select => "results-read-select" {},
        Head => "results-read-head" {},
        Results => "results-read-results" {},
        Bindings => "results-read-bindings" {},
        Boolean => "results-read-boolean" {},
        ProvenanceSolution => "results-read-provenance-solution" {},
        ProvenanceSource => "results-read-provenance-source" {},
        BindingEntry => "results-read-binding-entry" {},
        Variable => "results-read-variable" {},
        Subject => "results-read-subject" {},
        Predicate => "results-read-predicate" {},
        Object => "results-read-object" {},
        BindingType => "results-read-binding-type" {},
        TripleValue => "results-read-triple-value" {},
        BindingObject => "results-read-binding-object" {},
        BindingValue => "results-read-binding-value" {},
        Array => "results-read-array" {},
        BindingName => "results-read-binding-name" {},
        SparqlRoot => "results-read-sparql-root" {},
        Component { role } => "results-read-component" { "role" => Code(role) },
        ComponentTerm { role } => "results-read-component-term" { "role" => Code(role) },
    }
}

/// The native token category that rejected an original spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadToken {
    /// A literal's base direction.
    Direction,
    /// A JSON binding type.
    BindingType,
    /// An XML Boolean value.
    Boolean,
    /// An XML term element name.
    Element,
    /// An XML binding names no projected variable.
    Variable,
}

impl purrdf_hash::error::StructuredError for ReadToken {
    purrdf_hash::error_visit! {
        Direction => "results-token-direction" {},
        BindingType => "results-token-binding-type" {},
        Boolean => "results-token-boolean" {},
        Element => "results-token-element" {},
        Variable => "results-token-variable" {},
    }
}

/// Original semantic operands of a results-document refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadReason {
    /// A required structural shape is absent.
    Shape(ReadShape),
    /// A token has no interpretation in its original grammar position.
    Token {
        /// The position that refused it.
        kind: ReadToken,
        /// Original spelling, not a formatted reason.
        token: String,
    },
    /// A triple predicate is not an IRI.
    Predicate(TermValue),
}

impl purrdf_hash::error::StructuredError for ReadReason {
    purrdf_hash::error_visit! {
        Shape(shape) => "results-read-shape" { "shape" => Record(shape) },
        Token { kind, token } => "results-read-token" { "kind" => Record(kind), "token" => Text(token) },
        Predicate(term) => "results-read-non-iri-predicate" { "term" => Term(term) },
    }
}

/// A native refusal attributed to the actual input format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadRefusal {
    /// Original input format.
    pub format: SparqlResultsFormat,
    /// Original structural or lexical reason.
    pub reason: ReadReason,
}

impl ReadRefusal {
    /// A structural refusal needs no heap payload or presentation allocation.
    #[must_use]
    pub const fn shape(format: SparqlResultsFormat, shape: ReadShape) -> Error {
        Error::Format(Self {
            format,
            reason: ReadReason::Shape(shape),
        })
    }

    /// Retain an original token under its caller's existing memory admission.
    /// Refusal to store it preserves the physical storage failure.
    pub(crate) fn token<S: Admission + ?Sized>(
        format: SparqlResultsFormat,
        kind: ReadToken,
        token: &str,
        memory: &mut Memory<'_, S>,
    ) -> Error {
        match memory.string(token) {
            Ok(token) => Error::Format(Self {
                format,
                reason: ReadReason::Token { kind, token },
            }),
            Err(error) => Error::Storage(error),
        }
    }
}

impl purrdf_hash::error::StructuredError for ReadRefusal {
    purrdf_hash::error_visit! {
        @record self { format, reason } => "results-read-refusal" {
            "format" => Record(&self.format), "reason" => Cause(&self.reason)
        }
    }
}

impl core::fmt::Display for ReadRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let xml = self.format == SparqlResultsFormat::Xml;
        f.write_str(match self.format {
            SparqlResultsFormat::Xml => "SPARQL-XML: ",
            SparqlResultsFormat::Json => "SPARQL-JSON: ",
            SparqlResultsFormat::Csv => "SPARQL-CSV: ",
            SparqlResultsFormat::Tsv => "SPARQL-TSV: ",
        })?;
        match &self.reason {
            ReadReason::Predicate(_) => f.write_str("triple-term predicate is not an IRI"),
            ReadReason::Token { kind, token } => match kind {
                ReadToken::Direction => write!(f, "unknown base direction `{token}`"),
                ReadToken::BindingType => write!(f, "unknown binding type `{token}`"),
                ReadToken::Boolean => write!(f, "invalid <boolean> value `{token}`"),
                ReadToken::Element => write!(f, "unexpected term element <{token}>"),
                ReadToken::Variable => f.write_str("<binding> names an undeclared variable"),
            },
            ReadReason::Shape(shape) => match shape {
                ReadShape::TopObject => f.write_str("top level is not an object"),
                ReadShape::Select => {
                    f.write_str("expected SELECT results, got an ASK (boolean) document")
                }
                ReadShape::Head => f.write_str("missing `head` object"),
                ReadShape::Results => f.write_str(if xml {
                    "missing <results>"
                } else {
                    "missing `results` object"
                }),
                ReadShape::Bindings => f.write_str("missing `results.bindings` array"),
                ReadShape::Boolean => f.write_str(if xml {
                    "missing <boolean>"
                } else {
                    "missing boolean `boolean` field"
                }),
                ReadShape::ProvenanceSolution => {
                    f.write_str("provenance solution is not an object")
                }
                ReadShape::ProvenanceSource => f.write_str("provenance source is not a string"),
                ReadShape::BindingEntry => f.write_str("`results.bindings` entry is not an object"),
                ReadShape::Variable => f.write_str("`head.vars` entry is not a string"),
                ReadShape::Subject => f.write_str("triple has no subject"),
                ReadShape::Predicate => f.write_str("triple has no predicate"),
                ReadShape::Object => f.write_str("triple has no object"),
                ReadShape::BindingType => f.write_str("binding has no string `type`"),
                ReadShape::TripleValue => f.write_str("triple binding has no object `value`"),
                ReadShape::BindingObject => f.write_str("binding is not an object"),
                ReadShape::BindingValue => f.write_str("binding has no string `value`"),
                ReadShape::Array => f.write_str("expected array"),
                ReadShape::BindingName => f.write_str("<binding> without name"),
                ReadShape::SparqlRoot => f.write_str("root element is not <sparql>"),
                ReadShape::Component { role } => write!(f, "<triple> missing <{role}>"),
                ReadShape::ComponentTerm { role } => write!(f, "<{role}> has no term"),
            },
        }
    }
}

impl std::error::Error for ReadRefusal {}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;

    #[test]
    fn readers_retain_original_tokens_after_input_destruction() {
        for (input, format, boolean, kind, token) in [
            (
                r#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":{"type":"unrecognized-original","value":"x"}}]}}"#,
                SparqlResultsFormat::Json,
                false,
                "results-token-binding-type",
                "unrecognized-original",
            ),
            (
                "<sparql><boolean>not-a-boolean</boolean></sparql>",
                SparqlResultsFormat::Xml,
                true,
                "results-token-boolean",
                "not-a-boolean",
            ),
            (
                r#"<sparql><head><variable name="x"/></head><results><result><binding name="absent-original"><uri>http://example.org/x</uri></binding></result></results></sparql>"#,
                SparqlResultsFormat::Xml,
                false,
                "results-token-variable",
                "absent-original",
            ),
            (
                r#"<sparql><head><variable name="x"/></head><results><result><binding name="x"><original-element/></binding></result></results></sparql>"#,
                SparqlResultsFormat::Xml,
                false,
                "results-token-element",
                "original-element",
            ),
        ] {
            let input = input.as_bytes().to_vec();
            let error = match (format, boolean) {
                (SparqlResultsFormat::Json, false) => crate::from_json(&input).unwrap_err(),
                (SparqlResultsFormat::Xml, false) => crate::from_xml(&input).unwrap_err(),
                (SparqlResultsFormat::Xml, true) => crate::from_xml_boolean(&input).unwrap_err(),
                _ => unreachable!("the cases contain only actual reader entry points"),
            };
            drop(input);
            let capture = Capture::read(&error);
            assert_eq!(capture.code, "sparql-results-format");
            let refusal = capture.child("cause");
            let reason = refusal.child("reason");
            assert_eq!(reason.code, "results-read-token");
            assert_eq!(reason.child("kind").code, kind);
            assert!(matches!(reason.field("token"), ErrorValue::Text(value) if value == token));
        }
    }
}
