// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parsing for SHACL-AF `sh:SPARQLTargetType` declarations.

use std::collections::BTreeMap;

use ::purrdf::FastSet;

use purrdf_sparql_algebra::{Query, SparqlParser};

use crate::model::{rdf, sh};
use crate::term::{NamedNode, Term};

use crate::shapes::{Parser, SparqlTargetType, TargetTypeParam};

impl Parser<'_> {
    /// Parse every `sh:SPARQLTargetType` declaration in the shapes graph into a
    /// map keyed by target-type IRI string.
    ///
    /// Each declaration must have one or more `sh:parameter`s naming a predicate
    /// (via `sh:path`), a `sh:select` SELECT query, and may carry a `sh:prefixes`
    /// declaration. Malformed declarations are hard failures.
    pub(crate) fn parse_sparql_target_types(
        &self,
    ) -> Result<BTreeMap<String, SparqlTargetType>, String> {
        let mut ids: Vec<Term> = self
            .quads_with(None, Some(rdf::TYPE), Some(sh::SPARQL_TARGET_TYPE))
            .into_iter()
            .map(|(subject, _, _)| subject)
            .collect();
        crate::term::sort_terms_canonical(&mut ids);
        ids.dedup();

        let mut registry: BTreeMap<String, SparqlTargetType> = BTreeMap::new();
        for id in ids {
            let iri = match &id {
                Term::NamedNode(n) => n.as_str().to_owned(),
                other => {
                    return Err(format!("sh:SPARQLTargetType must be an IRI, got {other}"));
                }
            };
            let target_type = self.parse_one_sparql_target_type(&id, &iri)?;
            registry.insert(iri, target_type);
        }
        Ok(registry)
    }

    /// Parse a single `sh:SPARQLTargetType` declaration node.
    fn parse_one_sparql_target_type(
        &self,
        id: &Term,
        iri: &str,
    ) -> Result<SparqlTargetType, String> {
        // Parameters, ordered by (sh:order, predicate IRI).
        let mut raw: Vec<(f64, NamedNode, String)> = Vec::new();
        for p_node in self.objects_of(id, sh::PARAMETER_PROPERTY) {
            let predicate = self
                .first_object_of(&p_node, sh::PATH)
                .or_else(|| self.first_object_of(&p_node, sh::PREDICATE))
                .and_then(|t| match t {
                    Term::NamedNode(n) => Some(n),
                    _ => None,
                })
                .ok_or_else(|| {
                    format!(
                        "sh:SPARQLTargetType <{iri}> has a sh:parameter without an IRI sh:path/sh:predicate"
                    )
                })?;
            let var = crate::shapes::local_name(predicate.as_str()).to_owned();
            if var.is_empty() {
                return Err(format!(
                    "sh:SPARQLTargetType <{iri}> has a sh:parameter whose predicate <{}> has an empty local name",
                    predicate.as_str()
                ));
            }
            const RESERVED_VARS: [&str; 6] = [
                "this",
                "path",
                "PATH",
                "value",
                "shapesGraph",
                "currentShape",
            ];
            if RESERVED_VARS.contains(&var.as_str()) {
                return Err(format!(
                    "sh:SPARQLTargetType <{iri}> parameter variable ?{var} is a SHACL/SHACL-AF reserved name"
                ));
            }
            let order = match self.first_object_of(&p_node, sh::ORDER) {
                None => f64::INFINITY,
                Some(Term::Literal(lit)) => lit.value().parse::<f64>().map_err(|_| {
                    format!(
                        "sh:SPARQLTargetType <{iri}> parameter ?{var} has a non-numeric sh:order '{}'",
                        lit.value()
                    )
                })?,
                Some(other) => {
                    return Err(format!(
                        "sh:SPARQLTargetType <{iri}> parameter ?{var} has a non-literal sh:order {other}"
                    ));
                }
            };
            raw.push((order, predicate, var));
        }
        raw.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.as_str().cmp(b.1.as_str()))
        });

        // Reject colliding derived variable names.
        let mut seen: FastSet<&str> = FastSet::default();
        for (_, _, var) in &raw {
            if !seen.insert(var.as_str()) {
                return Err(format!(
                    "sh:SPARQLTargetType <{iri}> has two parameters whose variable name ?{var} collides"
                ));
            }
        }

        let params: Vec<TargetTypeParam> = raw
            .into_iter()
            .map(|(_, predicate, var)| TargetTypeParam { predicate, var })
            .collect();

        // sh:select is required and must be a SELECT query.
        let raw_select = self
            .first_object_of(id, sh::SELECT)
            .and_then(|t| match t {
                Term::Literal(lit) => Some(lit.value().to_owned()),
                _ => None,
            })
            .ok_or_else(|| {
                format!("sh:SPARQLTargetType <{iri}> is missing a sh:select string literal")
            })?;
        let select = format!("{}{raw_select}", self.prefix_header(&[id]));
        match SparqlParser::new().parse_query(&select) {
            Ok(Query::Select { .. }) => {}
            Ok(_) => {
                return Err(format!(
                    "sh:SPARQLTargetType <{iri}> must be a SELECT query (ASK/CONSTRUCT/DESCRIBE are not valid)"
                ));
            }
            Err(e) => {
                return Err(format!(
                    "sh:SPARQLTargetType <{iri}> has an unparsable sh:select query: {e}"
                ));
            }
        }

        Ok(SparqlTargetType {
            id: id.clone(),
            params,
            select: raw_select,
        })
    }
}
