// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Template instantiation shared by `CONSTRUCT` and SPARQL `UPDATE`.
//!
//! Both the `CONSTRUCT` output template ([`construct`](crate::construct)) and the
//! `DELETE`/`INSERT` quad templates of an UPDATE operation ([`update`](crate::update))
//! turn a triple/quad *pattern* into concrete [`TermValue`]s, once per solution row,
//! under the same three SPARQL §16.2 rules:
//!
//! 1. A template position holding an **unbound variable** makes the whole quad be
//!    skipped (`None`).
//! 2. A template **blank node is minted fresh per solution row** — the same label
//!    co-refers within one row (the `blanks` map), distinct across rows (the
//!    cross-row monotonic [`EvalCtx::bnode_counter`]).
//! 3. **Positional validity** (an asserted subject that is not an IRI or a blank
//!    node, a non-IRI predicate, or an object triple term whose own components break
//!    the RDF 1.2 term model) is decided by the *caller* after instantiation, because
//!    the two callers intern into different sinks (a builder vs. a
//!    [`MutableDataset`](purrdf_core::MutableDataset)).
//!
//! These helpers stop at the dataset-independent [`TermValue`]: a bound variable is
//! resolved via `ctx.scratch.value_of(ctx.dataset, term)`, so the value is valid
//! across a snapshot→mutable boundary (the UPDATE round-trip).

use purrdf_core::{BlankScope, DatasetView, TermValue};
use purrdf_sparql_algebra::{NamedNodePattern, TermPattern};

use crate::DetHashMap;
use crate::convert::{literal_to_value, named_node_to_value};
use crate::eval::EvalCtx;
use crate::solution::{Solution, VarSchema};

/// SPARQL §16.2 positional validity: an instantiated triple is **ill-formed** — and
/// the caller SKIPS it rather than erroring — when it is not a legal RDF 1.2
/// statement. Shared by `CONSTRUCT`, the UPDATE `DELETE`/`INSERT` templates, and the
/// variable-free `DATA` path so the rule lives in exactly one place.
///
/// The rule is the RDF 1.2 term model, position by position, and it is deliberately
/// the EXACT complement of what the IR's pre-freeze validator accepts — anything this
/// predicate lets through must be internable, or a skippable instantiation would
/// instead hard-fail the whole query at `freeze` time:
///
/// * **subject** — an ASSERTED subject is an IRI or a blank node. A literal is
///   illegal, and so is a triple term: a quoted triple is a *value*, and an asserted
///   statement cannot be made about one without a reifier standing in for it. Both
///   are reachable purely from data — `CONSTRUCT { ?o ?p ?s }` over RDF 1.2 annotated
///   input binds `?o` to a triple term exactly as easily as to a literal.
/// * **predicate** — an IRI, and nothing else (a literal, a blank node and a triple
///   term are all illegal).
/// * **object** — every term kind is legal, but when the object *is* a triple term
///   its own components carry the (weaker) triple-component rules, recursively; see
///   [`triple_term_well_formed`].
///
/// A graph term is NOT decided here: the two callers reach it by different routes
/// (a plan-time slot versus a `WITH`-defaulted pattern) and each rejects a non-IRI
/// graph name at its own site.
pub(crate) fn positionally_ill_formed(
    subject: &TermValue,
    predicate: &TermValue,
    object: &TermValue,
) -> bool {
    !matches!(subject, TermValue::Iri(_) | TermValue::Blank { .. })
        || !matches!(predicate, TermValue::Iri(_))
        || !triple_term_well_formed(object)
}

/// Whether `term` is legal WHERE IT STANDS as a triple-term component (an object
/// position, or a position nested inside a quoted triple).
///
/// A non-triple term is always legal there — an IRI, a blank node and a literal are
/// all admissible objects. A triple term is legal when its own subject is not a
/// literal and its predicate is an IRI, recursively through both its subject and its
/// object. The subject rule is WEAKER than the asserted-subject rule on purpose: a
/// quoted triple may be nested inside another quoted triple, it just may not be
/// *asserted about*.
fn triple_term_well_formed(term: &TermValue) -> bool {
    let TermValue::Triple { s, p, o } = term else {
        return true;
    };
    !matches!(**s, TermValue::Literal { .. })
        && matches!(**p, TermValue::Iri(_))
        && triple_term_well_formed(s)
        && triple_term_well_formed(o)
}

/// Instantiate a **variable-free** template term (the `INSERT DATA` / `DELETE DATA`
/// path). DATA is variable-free by a hard parser invariant, so no solution/dataset is
/// consulted — a `Variable` here is a malformed-input guard that skips the quad
/// (`None`). Blank labels mint fresh from `counter`, co-referring within the shared
/// `blanks` scope (one DATA block), exactly like the solution-driven path.
pub(crate) fn instantiate_ground_term(
    term: &TermPattern,
    blanks: &mut DetHashMap<String, String>,
    counter: &mut u64,
) -> Option<TermValue> {
    match term {
        TermPattern::NamedNode(n) => Some(named_node_to_value(n)),
        TermPattern::Literal(l) => Some(literal_to_value(l)),
        // The DATA path mints unprefixed: it is variable-free ingestion with a
        // request-local counter, never a per-focus SHACL evaluation.
        TermPattern::BlankNode(b) => Some(mint_blank(b.as_str(), blanks, counter, None)),
        TermPattern::Triple(t) => {
            let s = instantiate_ground_term(&t.subject, blanks, counter)?;
            let p = match &t.predicate {
                NamedNodePattern::NamedNode(n) => named_node_to_value(n),
                NamedNodePattern::Variable(_) => return None,
            };
            let o = instantiate_ground_term(&t.object, blanks, counter)?;
            Some(TermValue::Triple {
                s: Box::new(s),
                p: Box::new(p),
                o: Box::new(o),
            })
        }
        TermPattern::Variable(_) => None,
    }
}

/// Instantiate a subject/object template term. `None` = an unbound variable.
pub(crate) fn instantiate_term<D: DatasetView + Sync>(
    term: &TermPattern,
    row: &Solution<D::Id>,
    schema: &VarSchema,
    blanks: &mut DetHashMap<String, String>,
    ctx: &mut EvalCtx<'_, D>,
) -> Option<TermValue> {
    match term {
        TermPattern::NamedNode(n) => Some(named_node_to_value(n)),
        TermPattern::Literal(l) => Some(literal_to_value(l)),
        TermPattern::Variable(v) => {
            let term = schema.index_of(v).and_then(|c| row[c])?;
            Some(ctx.scratch.value_of(ctx.dataset, term))
        }
        TermPattern::BlankNode(b) => Some(fresh_blank(b.as_str(), blanks, ctx)),
        TermPattern::Triple(t) => {
            // RDF 1.2 quoted-triple term in the template: instantiate recursively.
            let s = instantiate_term(&t.subject, row, schema, blanks, ctx)?;
            let p = instantiate_predicate(&t.predicate, row, schema, ctx)?;
            let o = instantiate_term(&t.object, row, schema, blanks, ctx)?;
            Some(TermValue::Triple {
                s: Box::new(s),
                p: Box::new(p),
                o: Box::new(o),
            })
        }
    }
}

/// Instantiate a predicate template position. `None` = an unbound variable.
pub(crate) fn instantiate_predicate<D: DatasetView + Sync>(
    predicate: &NamedNodePattern,
    row: &Solution<D::Id>,
    schema: &VarSchema,
    ctx: &EvalCtx<'_, D>,
) -> Option<TermValue> {
    match predicate {
        NamedNodePattern::NamedNode(n) => Some(named_node_to_value(n)),
        NamedNodePattern::Variable(v) => {
            let term = schema.index_of(v).and_then(|c| row[c])?;
            Some(ctx.scratch.value_of(ctx.dataset, term))
        }
    }
}

/// The fresh blank value for a template label within the current solution row: the
/// first occurrence mints a globally-unique label from the **cross-row** monotonic
/// `bnode_counter`, later occurrences in the same row reuse it (the `blanks` map
/// resets per row, so the counter — not the map — is what makes two rows' blanks
/// distinct). Minted labels carry the context's deterministic
/// [`EvalCtx::bnode_mint_prefix`], when one is set.
pub(crate) fn fresh_blank<D: DatasetView + Sync>(
    template_label: &str,
    blanks: &mut DetHashMap<String, String>,
    ctx: &mut EvalCtx<'_, D>,
) -> TermValue {
    mint_blank(
        template_label,
        blanks,
        &mut ctx.bnode_counter,
        ctx.bnode_mint_prefix.as_deref(),
    )
}

/// The blank-minting core (independent of [`EvalCtx`]): first occurrence of
/// `template_label` mints a unique label from the monotonic `counter`, later
/// occurrences in the same `blanks` scope reuse it. Used by [`fresh_blank`]
/// (threading `ctx.bnode_counter` and the context's mint prefix) and the
/// variable-free DATA path (a local counter, no prefix).
///
/// With `prefix: None` the minted label is exactly `c{n}` — byte-identical to
/// every pre-prefix caller; with `Some(prefix)` it is `{prefix}c{n}`.
pub(crate) fn mint_blank(
    template_label: &str,
    blanks: &mut DetHashMap<String, String>,
    counter: &mut u64,
    prefix: Option<&str>,
) -> TermValue {
    if let Some(existing) = blanks.get(template_label) {
        return TermValue::Blank {
            label: existing.clone(),
            scope: BlankScope::DEFAULT,
        };
    }
    *counter += 1;
    let fresh = crate::eval::minted_label(prefix, "c", *counter);
    blanks.insert(template_label.to_owned(), fresh.clone());
    TermValue::Blank {
        label: fresh,
        scope: BlankScope::DEFAULT,
    }
}
