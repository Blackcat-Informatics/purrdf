// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The caller-owned scopes a curated projection (CSVW terms, OKF terms) reads
//! its source through: which graphs, and which subjects.
//!
//! Both projections select rows by the same two declarations, so each is one
//! type here — one validation law and one JSON form — named by each
//! projection through a type alias.

use std::collections::BTreeSet;

use purrdf_lex::json::record::{DecodeError, FromJson, Record, ToJson};
use purrdf_lex::json::{Object, Value};

use super::{ProjectionError, validate_absolute_iri};

/// Explicit RDF graph scope a projection reads its source through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphSelection {
    /// Read the default graph and every declared named graph as one view.
    All,
    /// Read exactly the caller-selected default/named graph identities.
    Include {
        /// Whether default-graph statements are in scope.
        default_graph: bool,
        /// Exact named-graph IRIs in scope.
        named_graphs: BTreeSet<String>,
    },
}

impl GraphSelection {
    /// Construct and validate an exact graph selection.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an empty scope or a relative
    /// named-graph IRI.
    pub fn include(
        default_graph: bool,
        named_graphs: BTreeSet<String>,
    ) -> Result<Self, ProjectionError> {
        let selection = Self::Include {
            default_graph,
            named_graphs,
        };
        selection.validate()?;
        Ok(selection)
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        if let Self::Include {
            default_graph,
            named_graphs,
        } = self
        {
            if !default_graph && named_graphs.is_empty() {
                return Err(ProjectionError::configuration(
                    "graph selection must include at least one graph",
                ));
            }
            for graph in named_graphs {
                validate_absolute_iri(graph, "selected named graph")?;
            }
        }
        Ok(())
    }

    /// Whether the default graph is selected.
    pub const fn includes_default_graph(&self) -> bool {
        matches!(
            self,
            Self::All
                | Self::Include {
                    default_graph: true,
                    ..
                }
        )
    }

    /// Whether a resolved named-graph IRI is selected.
    pub fn includes_named_graph(&self, graph: &str) -> bool {
        match self {
            Self::All => true,
            Self::Include { named_graphs, .. } => named_graphs.contains(graph),
        }
    }
}

impl FromJson for GraphSelection {
    /// `{"kind": "all"}` or `{"kind": "include", "default_graph",
    /// "named_graphs"}`, validated as [`GraphSelection::include`] validates.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "internally tagged enum GraphSelection")?;
        let selection = match fields.tag("kind", &["all", "include"])? {
            "all" => Self::All,
            _ => Self::Include {
                default_graph: fields.required("default_graph")?,
                named_graphs: fields.required("named_graphs")?,
            },
        };
        fields.deny_unknown()?;
        selection.validate()?;
        Ok(selection)
    }
}

impl ToJson for GraphSelection {
    fn to_json(&self) -> Value {
        Value::Object(match self {
            Self::All => Object::new().with("kind", "all"),
            Self::Include {
                default_graph,
                named_graphs,
            } => Object::new()
                .with("kind", "include")
                .with("default_graph", *default_graph)
                .with("named_graphs", named_graphs.to_json()),
        })
    }
}

/// Caller-supplied RDF-type and subject-namespace membership test.
///
/// Empty type sets leave type unconstrained. Empty IRI prefixes admit every
/// subject; a non-empty prefix set admits only matching IRI subjects. A type
/// predicate is required exactly when a type constraint is present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectSelector {
    type_predicate: Option<String>,
    any_types: BTreeSet<String>,
    all_types: BTreeSet<String>,
    none_types: BTreeSet<String>,
    iri_prefixes: BTreeSet<String>,
}

impl SubjectSelector {
    /// Construct a validated subject selector.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for incomplete, contradictory, or relative
    /// IRI policy.
    pub fn new(
        type_predicate: Option<String>,
        any_types: BTreeSet<String>,
        all_types: BTreeSet<String>,
        none_types: BTreeSet<String>,
        iri_prefixes: BTreeSet<String>,
    ) -> Result<Self, ProjectionError> {
        let selector = Self {
            type_predicate,
            any_types,
            all_types,
            none_types,
            iri_prefixes,
        };
        selector.validate()?;
        Ok(selector)
    }

    /// Predicate whose IRI objects define type membership.
    pub fn type_predicate(&self) -> Option<&str> {
        self.type_predicate.as_deref()
    }

    /// Types of which at least one must be present, when non-empty.
    pub const fn any_types(&self) -> &BTreeSet<String> {
        &self.any_types
    }

    /// Types all of which must be present.
    pub const fn all_types(&self) -> &BTreeSet<String> {
        &self.all_types
    }

    /// Types none of which may be present.
    pub const fn none_types(&self) -> &BTreeSet<String> {
        &self.none_types
    }

    /// Allowed subject-IRI prefixes; empty admits every subject.
    pub const fn iri_prefixes(&self) -> &BTreeSet<String> {
        &self.iri_prefixes
    }

    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        let constrained =
            !(self.any_types.is_empty() && self.all_types.is_empty() && self.none_types.is_empty());
        if constrained != self.type_predicate.is_some() {
            return Err(ProjectionError::configuration(
                "subject selector requires type_predicate exactly when type constraints are present",
            ));
        }
        if let Some(predicate) = &self.type_predicate {
            validate_absolute_iri(predicate, "subject selector type predicate")?;
        }
        for (role, values) in [
            ("required-any type", &self.any_types),
            ("required-all type", &self.all_types),
            ("excluded type", &self.none_types),
        ] {
            for value in values {
                validate_absolute_iri(value, &format!("subject selector {role}"))?;
            }
        }
        if self
            .none_types
            .iter()
            .any(|value| self.any_types.contains(value) || self.all_types.contains(value))
        {
            return Err(ProjectionError::configuration(
                "subject selector cannot both require and exclude the same type",
            ));
        }
        for prefix in &self.iri_prefixes {
            validate_absolute_iri(prefix, "subject selector IRI prefix")?;
        }
        Ok(())
    }
}

purrdf_lex::json_record!(SubjectSelector as "struct SubjectSelector" {
    "type_predicate" => type_predicate: optional,
    "any_types" => any_types: required,
    "all_types" => all_types: required,
    "none_types" => none_types: required,
    "iri_prefixes" => iri_prefixes: required,
} => SubjectSelector::new);

#[cfg(test)]
mod tests {
    use super::*;

    const TYPE: &str = "https://example.org/type";
    const CLASS: &str = "https://example.org/Class";

    fn read<T: FromJson>(text: &str) -> Result<T, DecodeError> {
        T::from_json(&purrdf_lex::json::read(text).expect("JSON"))
    }

    #[test]
    fn a_graph_selection_reads_its_two_forms_and_refuses_an_empty_or_relative_scope() {
        assert_eq!(read(r#"{"kind":"all"}"#), Ok(GraphSelection::All));
        let named =
            r#"{"kind":"include","default_graph":false,"named_graphs":["https://example.org/g"]}"#;
        let selection: GraphSelection = read(named).expect("named graph");
        assert!(selection.includes_named_graph("https://example.org/g"));
        assert!(!selection.includes_default_graph());
        assert_eq!(purrdf_lex::json::write_compact(&selection.to_json()), named);
        assert!(
            read::<GraphSelection>(r#"{"kind":"include","default_graph":true,"named_graphs":[]}"#)
                .is_ok()
        );
        let empty =
            read::<GraphSelection>(r#"{"kind":"include","default_graph":false,"named_graphs":[]}"#)
                .expect_err("empty scope");
        assert!(empty.to_string().contains("at least one graph"), "{empty}");
        assert!(
            read::<GraphSelection>(
                r#"{"kind":"include","default_graph":false,"named_graphs":["g"]}"#
            )
            .is_err()
        );
        assert!(read::<GraphSelection>(r#"{"kind":"all","default_graph":true}"#).is_err());
    }

    #[test]
    fn a_subject_selector_reads_through_its_constructor() {
        let typed = format!(
            r#"{{"type_predicate":"{TYPE}","any_types":["{CLASS}"],"all_types":[],"none_types":[],"iri_prefixes":[]}}"#
        );
        let selector: SubjectSelector = read(&typed).expect("typed selector");
        assert_eq!(selector.type_predicate(), Some(TYPE));
        assert_eq!(purrdf_lex::json::write_compact(&selector.to_json()), typed);
        let untyped = r#"{"type_predicate":null,"any_types":[],"all_types":[],"none_types":[],"iri_prefixes":[]}"#;
        assert!(read::<SubjectSelector>(untyped).is_ok());
        let missing_predicate = format!(
            r#"{{"type_predicate":null,"any_types":["{CLASS}"],"all_types":[],"none_types":[],"iri_prefixes":[]}}"#
        );
        assert!(read::<SubjectSelector>(&missing_predicate).is_err());
        let contradictory = format!(
            r#"{{"type_predicate":"{TYPE}","any_types":["{CLASS}"],"all_types":[],"none_types":["{CLASS}"],"iri_prefixes":[]}}"#
        );
        assert!(read::<SubjectSelector>(&contradictory).is_err());
    }
}
