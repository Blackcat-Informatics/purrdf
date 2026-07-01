// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Engine-side variable **pre-binding** (purrdf S6, EPIC #906 GAP-A).
//!
//! Bridges the engine's egress term model ([`TermValue`]) to the algebra's
//! [`Query::substitute_variable`] rewrite. Each `(name, value)` of a
//! [`SparqlRequest::substitutions`](purrdf_core::SparqlRequest) pre-binds the
//! query variable `name` to `value` before evaluation, exactly mirroring oxigraph's
//! `PreparedSparqlQuery::substitute_variable` (the SHACL `$this` focus-node path).
//!
//! The substitution is applied to a **clone** of the cached (un-substituted) parse,
//! so the plan cache is never poisoned by a focus-node-specific binding.

use purrdf_core::{RdfDiagnostic, RdfTextDirection, TermValue};
use purrdf_sparql_algebra::{
    BaseDirection, BlankNode, GroundTerm, GroundTriple, Literal, NamedNode, Query, Variable,
};

/// Apply every `(name, value)` substitution to `query` as a pre-binding rewrite,
/// returning the rewritten query. Each value is mapped to the algebra's
/// [`GroundTerm`] (blank-node focus nodes ride the injection-only
/// [`GroundTerm::BlankNode`]) and injected as a single-row `VALUES` join at the core
/// `WHERE` pattern, beneath the solution-modifier stack but visible to the projected
/// variable list.
///
/// # Errors
///
/// Returns a [`RdfDiagnostic`] if a literal substitution carries a datatype IRI that
/// is not a syntactically valid IRI (the only way a [`TermValue`] cannot become a
/// [`GroundTerm`]).
pub(crate) fn apply_substitutions(
    mut query: Query,
    substitutions: &[(String, TermValue)],
) -> Result<Query, RdfDiagnostic> {
    for (name, value) in substitutions {
        let var = Variable::new(name.clone());
        let ground = ground_term_from_value(value)?;
        query = query.substitute_variable(&var, ground);
    }
    Ok(query)
}

/// Convert a dataset-independent [`TermValue`] to the algebra's [`GroundTerm`].
fn ground_term_from_value(value: &TermValue) -> Result<GroundTerm, RdfDiagnostic> {
    match value {
        TermValue::Iri(iri) => Ok(GroundTerm::NamedNode(node(iri)?)),
        TermValue::Blank { label, .. } => Ok(GroundTerm::BlankNode(BlankNode::new(label.clone()))),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => Ok(GroundTerm::Literal(literal_from_value(
            lexical_form,
            datatype,
            language.as_deref(),
            *direction,
        )?)),
        TermValue::Triple { s, p, o } => {
            let subject = ground_term_from_value(s)?;
            let GroundTerm::NamedNode(predicate) = ground_term_from_value(p)? else {
                return Err(RdfDiagnostic::error(
                    "native-sparql-subst-triple-predicate",
                    "a quoted-triple predicate must be an IRI".to_owned(),
                ));
            };
            let object = ground_term_from_value(o)?;
            Ok(GroundTerm::Triple(Box::new(GroundTriple {
                subject,
                predicate,
                object,
            })))
        }
    }
}

/// Build an algebra [`Literal`] from a value's components, choosing the plain /
/// typed / lang / dir-lang constructor that matches its shape.
fn literal_from_value(
    lexical_form: &str,
    datatype: &str,
    language: Option<&str>,
    direction: Option<RdfTextDirection>,
) -> Result<Literal, RdfDiagnostic> {
    match (language, direction) {
        (Some(lang), dir) => Ok(Literal::new_lang(
            lexical_form,
            lang,
            dir.map(|d| match d {
                RdfTextDirection::Ltr => BaseDirection::Ltr,
                RdfTextDirection::Rtl => BaseDirection::Rtl,
            }),
        )),
        (None, _) => Ok(Literal::new_typed(lexical_form, node(datatype)?)),
    }
}

/// Validate-and-wrap an IRI, surfacing a malformed IRI as a diagnostic.
fn node(iri: &str) -> Result<NamedNode, RdfDiagnostic> {
    NamedNode::new(iri).map_err(|e| RdfDiagnostic::error("native-sparql-subst-iri", e.to_string()))
}
