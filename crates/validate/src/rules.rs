// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SARIF rule metadata for PurRDF rule ids.
//!
//! A SARIF `reportingDescriptor` (rule) carries the documentation a dashboard
//! shows next to a finding: a name, a short and full description, and a `helpUri`
//! to authoritative docs. This module derives that metadata from a rule id:
//!
//! * SHACL constraint-component IRIs → the local name, a description, and a
//!   `helpUri` deep-linking the W3C SHACL specification anchor for that
//!   component.
//! * PurRDF diagnostic codes (`native-codec-parse`, …) → a short description of
//!   the failure class.
//!
//! Unknown ids still get a valid descriptor (just the id), so the rule table is
//! always complete.

use crate::model::{Message, ReportingDescriptor};

/// The SHACL vocabulary namespace.
const SHACL_NS: &str = "http://www.w3.org/ns/shacl#";

/// The W3C SHACL Recommendation base URL (spec anchors hang off it).
const SHACL_SPEC: &str = "https://www.w3.org/TR/shacl/";

/// Build the SARIF rule descriptor for `rule_id`.
#[must_use]
pub fn descriptor_for(rule_id: &str) -> ReportingDescriptor {
    if let Some(local) = rule_id.strip_prefix(SHACL_NS) {
        return shacl_component_descriptor(rule_id, local);
    }
    if let Some(descriptor) = diagnostic_descriptor(rule_id) {
        return descriptor;
    }
    bare(rule_id)
}

/// A descriptor for a SHACL constraint component, deep-linking the spec anchor.
fn shacl_component_descriptor(rule_id: &str, local: &str) -> ReportingDescriptor {
    // The SHACL spec anchors a constraint component at `#<LocalName>`.
    let help_uri = format!("{SHACL_SPEC}#{local}");
    let short =
        curated_shacl_summary(local).map_or_else(|| format!("SHACL {local}."), ToOwned::to_owned);
    ReportingDescriptor {
        id: rule_id.to_owned(),
        name: Some(local.to_owned()),
        short_description: Some(Message::text(short)),
        full_description: Some(Message::text(format!(
            "A SHACL {local}: a data node failed this constraint. See the SHACL specification for the component's validation semantics."
        ))),
        help: Some(Message::text(format!(
            "Consult the W3C SHACL specification for {local} at {help_uri}."
        ))),
        help_uri: Some(help_uri),
        default_configuration: None,
    }
}

/// One-line summaries for the most common SHACL components, so a dashboard shows
/// intent without opening the spec. Unlisted components fall back to a generic
/// summary; the `helpUri` is always present.
fn curated_shacl_summary(local: &str) -> Option<&'static str> {
    let summary = match local {
        "DatatypeConstraintComponent" => "A value has the wrong datatype (sh:datatype).",
        "ClassConstraintComponent" => {
            "A value is not an instance of the required class (sh:class)."
        }
        "NodeKindConstraintComponent" => "A value has the wrong node kind (sh:nodeKind).",
        "MinCountConstraintComponent" => "Too few values for a property (sh:minCount).",
        "MaxCountConstraintComponent" => "Too many values for a property (sh:maxCount).",
        "MinInclusiveConstraintComponent" => {
            "A value is below the inclusive minimum (sh:minInclusive)."
        }
        "MaxInclusiveConstraintComponent" => {
            "A value is above the inclusive maximum (sh:maxInclusive)."
        }
        "MinExclusiveConstraintComponent" => {
            "A value is at or below the exclusive minimum (sh:minExclusive)."
        }
        "MaxExclusiveConstraintComponent" => {
            "A value is at or above the exclusive maximum (sh:maxExclusive)."
        }
        "MinLengthConstraintComponent" => {
            "A value is shorter than the minimum length (sh:minLength)."
        }
        "MaxLengthConstraintComponent" => {
            "A value is longer than the maximum length (sh:maxLength)."
        }
        "PatternConstraintComponent" => "A value does not match the required pattern (sh:pattern).",
        "InConstraintComponent" => "A value is not in the allowed set (sh:in).",
        "NodeConstraintComponent" => {
            "A value does not conform to the required node shape (sh:node)."
        }
        "PropertyConstraintComponent" => "A node violates a property shape (sh:property).",
        "HasValueConstraintComponent" => "A required value is missing (sh:hasValue).",
        "LanguageInConstraintComponent" => {
            "A literal has a disallowed language tag (sh:languageIn)."
        }
        "UniqueLangConstraintComponent" => "A language tag is used more than once (sh:uniqueLang).",
        "ClosedConstraintComponent" => "A node has properties outside a closed shape (sh:closed).",
        "SPARQLConstraintComponent" => "A SPARQL-based constraint was violated (sh:sparql).",
        _ => return None,
    };
    Some(summary)
}

/// A descriptor for a PurRDF diagnostic code, if recognised.
fn diagnostic_descriptor(code: &str) -> Option<ReportingDescriptor> {
    let short = match code {
        "native-codec-parse" => "The RDF text codec could not parse the input document.",
        "native-codec-utf8" => "The input bytes are not valid UTF-8.",
        "native-codec-panic" => "The native RDF codec panicked while parsing.",
        _ => return None,
    };
    Some(ReportingDescriptor {
        id: code.to_owned(),
        name: Some(code.to_owned()),
        short_description: Some(Message::text(short)),
        full_description: None,
        help: None,
        help_uri: None,
        default_configuration: None,
    })
}

/// A minimal-but-valid descriptor carrying only the id.
fn bare(rule_id: &str) -> ReportingDescriptor {
    ReportingDescriptor {
        id: rule_id.to_owned(),
        name: None,
        short_description: None,
        full_description: None,
        help: None,
        help_uri: None,
        default_configuration: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shacl_component_gets_spec_anchor() {
        let d = descriptor_for("http://www.w3.org/ns/shacl#DatatypeConstraintComponent");
        assert_eq!(d.name.as_deref(), Some("DatatypeConstraintComponent"));
        assert_eq!(
            d.help_uri.as_deref(),
            Some("https://www.w3.org/TR/shacl/#DatatypeConstraintComponent")
        );
        assert!(d.short_description.is_some());
    }

    #[test]
    fn unknown_shacl_component_still_gets_a_help_uri() {
        let d = descriptor_for("http://www.w3.org/ns/shacl#SomeFutureConstraintComponent");
        assert!(d.help_uri.is_some());
        assert_eq!(
            d.short_description.as_ref().map(|m| m.text.as_str()),
            Some("SHACL SomeFutureConstraintComponent.")
        );
    }

    #[test]
    fn diagnostic_code_gets_a_description() {
        let d = descriptor_for("native-codec-parse");
        assert!(d.short_description.is_some());
        assert!(d.help_uri.is_none());
    }

    #[test]
    fn unknown_id_is_bare_but_valid() {
        let d = descriptor_for("mystery");
        assert_eq!(d.id, "mystery");
        assert!(d.name.is_none());
    }
}
