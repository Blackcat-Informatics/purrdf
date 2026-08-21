// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Conversions from the lexical [`purrdf_sparql_algebra`] term types to the
//! dataset-independent [`TermValue`] lookup/build key.
//!
//! The algebra carries terms lexically (an IRI string, a literal's lexical form +
//! datatype IRI); the IR keys term identity on [`TermValue`]. These helpers bridge
//! the two and apply the one normalization the IR's C0.1 literal-identity contract
//! requires at the lookup boundary: a language tag is lowercased so a query literal
//! matches the dataset's interned (already-lowercased) form.

use purrdf_core::{RdfTextDirection, TermValue};
use purrdf_sparql_algebra::{
    BaseDirection, GroundTerm, GroundTriple, Literal, NamedNode, TermPattern, TriplePattern,
};

use crate::error::EvalError;

/// Map the algebra's RDF-1.2 base direction to the IR's.
#[inline]
pub(crate) fn map_direction(direction: BaseDirection) -> RdfTextDirection {
    match direction {
        BaseDirection::Ltr => RdfTextDirection::Ltr,
        BaseDirection::Rtl => RdfTextDirection::Rtl,
    }
}

/// An IRI term value.
#[inline]
pub(crate) fn named_node_to_value(node: &NamedNode) -> TermValue {
    TermValue::Iri(node.as_str().to_owned())
}

/// A literal term value, with the language tag lowercased to match the IR's C0.1
/// interned identity (so a query literal resolves to the dataset's stored form).
pub(crate) fn literal_to_value(lit: &Literal) -> TermValue {
    TermValue::Literal {
        lexical_form: lit.value().to_owned(),
        datatype: lit.datatype().as_str().to_owned(),
        language: lit.language().map(str::to_ascii_lowercase),
        direction: lit.direction().map(map_direction),
    }
}

/// Convert a **ground** quoted-triple pattern to a [`TermValue::Triple`].
///
/// `site` names the construct the pattern was found in (`"a BGP"`, `"a
/// property-path endpoint"`, …) so the [`EvalError::Unsupported`] a variable
/// component produces names where it actually was, rather than a fixed site
/// baked into this shared helper — see [`ground_term_pattern_to_value`]'s docs
/// for why one caller's wording must not leak into another's diagnostic.
///
/// Returns [`EvalError::Unsupported`] if any component is a variable: matching a
/// quoted triple term whose components *bind* variables (structural triple-term
/// matching) is out of the current S6 scope for every caller of this helper, not
/// only BGPs; only fully-ground quoted triples resolve to a single interned id.
pub(crate) fn ground_triple_pattern_to_value(
    pattern: &TriplePattern,
    site: &str,
) -> Result<TermValue, EvalError> {
    let s = ground_term_pattern_to_value(&pattern.subject, site)?;
    let p = match &pattern.predicate {
        purrdf_sparql_algebra::NamedNodePattern::NamedNode(n) => named_node_to_value(n),
        purrdf_sparql_algebra::NamedNodePattern::Variable(_) => {
            return Err(EvalError::unsupported_deferred(
                crate::error::UnsupportedKind::QuotedTripleTermVariable,
                format!("variable predicate inside a quoted triple term in {site}"),
            ));
        }
    };
    let o = ground_term_pattern_to_value(&pattern.object, site)?;
    Ok(TermValue::Triple {
        s: Box::new(s),
        p: Box::new(p),
        o: Box::new(o),
    })
}

/// Convert a **ground** term pattern (no variables) to a [`TermValue`].
///
/// `site` is threaded through to [`ground_triple_pattern_to_value`] and named in
/// the [`EvalError::Unsupported`] a variable component produces (see that
/// function's docs). This helper is shared across every caller that needs a
/// fully-ground term — a BGP triple position, a property-path endpoint, a
/// property-function argument — and each names its own site so the message a
/// caller sees always describes the construct it actually wrote, not whichever
/// caller happened to be first to need this conversion.
pub(crate) fn ground_term_pattern_to_value(
    pattern: &TermPattern,
    site: &str,
) -> Result<TermValue, EvalError> {
    match pattern {
        TermPattern::NamedNode(n) => Ok(named_node_to_value(n)),
        TermPattern::BlankNode(b) => Ok(TermValue::Blank {
            label: b.as_str().to_owned(),
            scope: purrdf_core::BlankScope::DEFAULT,
        }),
        TermPattern::Literal(l) => Ok(literal_to_value(l)),
        TermPattern::Triple(t) => ground_triple_pattern_to_value(t, site),
        TermPattern::Variable(_) => Err(EvalError::unsupported_deferred(
            crate::error::UnsupportedKind::QuotedTripleTermVariable,
            format!("variable inside a quoted triple term in {site}"),
        )),
    }
}

/// Convert a [`GroundTerm`] (a `VALUES` cell or quoted-triple component) to a
/// [`TermValue`]. Always succeeds — a `GroundTerm` carries no variables.
pub(crate) fn ground_term_to_value(term: &GroundTerm) -> TermValue {
    match term {
        GroundTerm::NamedNode(n) => named_node_to_value(n),
        GroundTerm::Literal(l) => literal_to_value(l),
        GroundTerm::Triple(t) => ground_triple_to_value(t),
        // Injection-only: a substituted blank-node focus node. The label carries
        // the scope-qualified rendering the injector wrote into the algebra's
        // single string slot, so decoding it is the exact inverse and restores
        // the `(label, scope)` pair `term_id_by_value` resolves against. A label
        // that was never qualified decodes to itself at the default scope.
        //
        // This is the documented contract on `GroundTerm::BlankNode` and
        // `Query::substitute_variable`: the string in this slot is read as a
        // scope-qualified spelling. A default-scope label is its own spelling
        // and passes through byte for byte (the literal label `a.s1` is spelled
        // `a.s1`); only a scoped pair has a distinct spelling — its envelope,
        // `("a", scope 1)` being `purrdfesc1_a`.
        GroundTerm::BlankNode(b) => {
            let (label, scope) = purrdf_core::BlankScope::unqualify_label(b.as_str());
            TermValue::Blank {
                label: label.into_owned(),
                scope,
            }
        }
    }
}

/// Convert a [`GroundTriple`] to a [`TermValue::Triple`].
pub(crate) fn ground_triple_to_value(triple: &GroundTriple) -> TermValue {
    TermValue::Triple {
        s: Box::new(ground_term_to_value(&triple.subject)),
        p: Box::new(named_node_to_value(&triple.predicate)),
        o: Box::new(ground_term_to_value(&triple.object)),
    }
}
