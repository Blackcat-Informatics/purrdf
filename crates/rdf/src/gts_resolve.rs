// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared GTS graph termination and literal-direction checks.
//!
//! [`ensure_terms_terminate`] checks caller-built folded graphs iteratively.
//! Production importers also resolve identifiers iteratively, preserving their
//! non-empty-IRI, datatype-position and reifier-binding contracts while moving
//! leaf strings into native storage.
//!
//! A test-only eager resolver remains here as a bounded malformed-graph
//! reference. Its legacy [`purrdf_events::MAX_TERM_NESTING_DEPTH`] guard is not a
//! production storage or import limit. The literal-direction parser below is
//! shared by the production consuming and streaming importers.

use purrdf_gts::model::{Graph, TermKind};
pub(crate) use purrdf_gts::model::{anonymous_blank_label, unbound_triple_label};
use purrdf_gts::resolution::SelfReaching;

use crate::{BlankScope, RdfDiagnostic, RdfTextDirection};
#[cfg(test)]
use crate::{RdfLiteral, RdfLocation, RdfTerm, RdfTriple};

/// The blank-node scope of synthetic v1 projections: fresh anonymous blank nodes
/// (§7.1) and triple terms that state no triple (§7.3), with disjoint prefixes.
///
/// A document blank node lands in [`BlankScope::DEFAULT`] (a folded graph) or in the
/// scope of its segment (`segment index + 1`, which the streaming importer keeps
/// at most `u32::MAX - 2`), so no document label ever names a node of this scope: the
/// synthetic node is fresh, as the specification requires, whatever labels the
/// document uses. A label is only ever read together with its scope, so the same
/// label spelled by the document and by the projection is two nodes.
pub(crate) const SYNTHETIC_BLANK_SCOPE: BlankScope =
    purrdf_core::blank_label::UNBOUND_TRIPLE_BLANK_SCOPE;

/// Render authored and synthetic blank labels in the same injective
/// `(label, scope)` alphabet for fold view tokens and streaming text.
///
/// Proof. `encode_blank_label` is injective over `(label, scope)` for a fixed
/// alphabet. A document blank node is encoded at a scope other than
/// [`SYNTHETIC_BLANK_SCOPE`] (the default scope, or a segment scope at most
/// `u32::MAX - 2`), and an anonymous blank or unbound triple term is encoded at
/// [`SYNTHETIC_BLANK_SCOPE`]. Two nodes with distinct scopes are distinct pairs, so
/// their tokens differ, whatever label the document spells.
pub(crate) fn blank_token(label: &str, scope: BlankScope) -> std::borrow::Cow<'_, str> {
    purrdf_core::blank_label::encode_blank_label(
        label,
        scope,
        purrdf_core::blank_label::LabelAlphabet::BlankNodeLabel,
    )
}

/// The outgoing structural edges of one term: the ids it makes another walker
/// resolve. Three is the maximum (a quoted triple's `(s, p, o)`); a literal
/// contributes one (its datatype term) and an IRI or blank node none.
type TermEdges = ([usize; 3], usize);

/// The triple terms of `graph` that reach themselves, after proving that no other
/// chain of the term table does.
///
/// GTS-SPEC §7.3 makes termination normative for EVERY walk of a triple term's
/// resolved components. A triple term that is reachable from its own components
/// states no triple: it keeps an identity of its own and projects as the fresh blank
/// node an unbound triple term projects as, while every `reifies` row stays recorded.
/// [`SelfReaching::of`] is the one definition of that set, and the walkers of this
/// crate read components through
/// [`Graph::resolved_triple_of`](purrdf_gts::model::Graph::resolved_triple_of) so
/// each observes the same answer. An unbounded recursion in Rust overflows the stack,
/// which **aborts the process**, so the set is found with an iterative search.
///
/// What remains after those terms stop contributing components is a literal whose
/// datatype chain leads back to itself. A literal is not a triple term, so §7.3 does
/// not degrade it, and every walker that resolves a datatype as a term of its own
/// would never finish; that shape is refused.
///
/// # Errors
///
/// Returns `gts-self-reaching-term` naming the first term of such a chain. An
/// out-of-range component id is NOT this check's business and is skipped: it names no
/// term, so it closes no loop, and the resolvers report it under their own range codes
/// (`gts-term-out-of-range`, `native-codec-term-out-of-range`) when they reach it.
pub(crate) fn ensure_terms_terminate(graph: &Graph) -> Result<SelfReaching, RdfDiagnostic> {
    /// Never visited.
    const UNSEEN: u8 = 0;
    /// On the current DFS path — reaching it again is a cycle.
    const ON_PATH: u8 = 1;
    /// Fully explored and proven to terminate.
    const SETTLED: u8 = 2;

    let reaching = SelfReaching::of(graph);
    let mut state = vec![UNSEEN; graph.terms.len()];
    // One frame per term on the current path: its id, its outgoing edges resolved
    // once, and how many of them have been followed.
    let mut path: Vec<(usize, TermEdges, usize)> = Vec::new();

    for root in 0..graph.terms.len() {
        if state[root] != UNSEEN {
            continue;
        }
        state[root] = ON_PATH;
        path.push((root, term_edges(graph, &reaching, root), 0));
        while let Some(frame) = path.last_mut() {
            let (tid, (edges, edge_count), cursor) = *frame;
            if cursor == edge_count {
                state[tid] = SETTLED;
                path.pop();
                continue;
            }
            frame.2 = cursor + 1;
            let next = edges[cursor];
            match state.get(next) {
                // A dangling component id resolves to no term, so it closes no loop.
                None | Some(&SETTLED) => {}
                Some(&ON_PATH) => {
                    return Err(RdfDiagnostic::error(
                        "gts-self-reaching-term",
                        format!(
                            "GTS term {next} resolves through itself, so no walk of its \
                             components can terminate"
                        ),
                    ));
                }
                Some(_) => {
                    state[next] = ON_PATH;
                    path.push((next, term_edges(graph, &reaching, next), 0));
                }
            }
        }
    }
    Ok(reaching)
}

/// The term ids `term_id` makes a walker resolve, in the order the walkers follow
/// them. This MUST mirror what those walkers actually read: an edge omitted here is
/// a loop the check admits and the first walker then dies on.
fn term_edges(graph: &Graph, reaching: &SelfReaching, term_id: usize) -> TermEdges {
    let Some(term) = graph.terms.get(term_id) else {
        return ([0; 3], 0);
    };
    match term.kind {
        // `render_literal`, the N-Triples/TriG/RDF-XML writers and the IR fold all
        // resolve a literal's datatype as a term of its own.
        TermKind::Literal => match term.datatype {
            Some(datatype) => ([datatype, 0, 0], 1),
            None => ([0; 3], 0),
        },
        // `Graph::resolved_triple_of` is the ONE place a folded quoted triple's
        // components are resolved, so it is the one edge set that matters here.
        TermKind::Triple => match graph.resolved_triple_of(term_id, reaching) {
            Some(triple) => (<[usize; 3]>::from(triple), 3),
            None => ([0; 3], 0),
        },
        TermKind::Iri | TermKind::Bnode => ([0; 3], 0),
    }
}

/// Parse a GTS literal base-direction string (`"ltr"`/`"rtl"`)
/// into the IR's [`RdfTextDirection`]. `None` is legitimate absence; an
/// unrecognized non-empty value is a hard error rather than a silent drop —
/// the GTS round-trip is ours, so a malformed direction is corrupt input, not
/// an intentional loss. Shared by all three decode paths (eager resolver,
/// consuming `import_graph`, streaming `import_sink`).
///
/// RDF 1.2 admits a base direction ONLY on a language-tagged string, so `language`
/// MUST be present (non-empty) whenever a direction is given; a direction without a
/// language tag hard-fails (`gts-direction-without-language`) rather than silently
/// producing an ill-formed literal.
pub(crate) fn parse_gts_direction(
    value: Option<&str>,
    language: Option<&str>,
) -> Result<Option<RdfTextDirection>, RdfDiagnostic> {
    parse_gts_direction_with_memory(
        value,
        language,
        &mut purrdf_lex::allocation::Memory::new(&mut purrdf_lex::allocation::Resident),
    )
    .map_err(RdfDiagnostic::from)
}

/// Inspect the same native direction through the source's original account.
pub(crate) fn parse_gts_direction_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
    value: Option<&str>,
    language: Option<&str>,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<Option<RdfTextDirection>, purrdf_core::ir::NativeBuildError> {
    memory.admission_mut().checkpoint(1)?;
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(direction) = RdfTextDirection::from_str_token(value) else {
        return Err(RdfDiagnostic::try_error_with_memory(
            "gts-invalid-direction",
            &format_args!("unrecognized GTS literal base direction {value:?}"),
            memory,
        )?
        .into());
    };
    if language.is_none_or(str::is_empty) {
        return Err(RdfDiagnostic::try_error_with_memory(
            "gts-direction-without-language",
            &"an RDF 1.2 literal base direction requires a non-empty language tag",
            memory,
        )?
        .into());
    }
    Ok(Some(direction))
}

/// Resolve a graph term id into an [`RdfTerm`], cloning the borrowed strings.
#[cfg(test)]
pub(crate) fn term_from_id(
    graph: &Graph,
    term_id: usize,
    location: RdfLocation,
) -> Result<RdfTerm, RdfDiagnostic> {
    term_from_id_depth(graph, term_id, location, 0)
}

#[cfg(test)]
fn triple_from_ids_depth(
    graph: &Graph,
    s: usize,
    p: usize,
    o: usize,
    location: RdfLocation,
    depth: usize,
) -> Result<RdfTriple, RdfDiagnostic> {
    let subject = term_from_id_depth(graph, s, location.clone(), depth)?;
    let predicate = predicate_from_id_depth(graph, p, location.clone(), depth)?;
    let object = term_from_id_depth(graph, o, location.clone(), depth)?;
    Ok(RdfTriple::new(subject, predicate, object).with_location(location))
}

#[cfg(test)]
fn predicate_from_id_depth(
    graph: &Graph,
    term_id: usize,
    location: RdfLocation,
    depth: usize,
) -> Result<String, RdfDiagnostic> {
    match term_from_id_depth(graph, term_id, location.clone(), depth)? {
        RdfTerm::Iri(iri) => Ok(iri),
        other => Err(RdfDiagnostic::error(
            "gts-predicate-not-iri",
            format!("GTS predicate term must be an IRI, got {:?}", other.kind()),
        )
        .with_location(location.with_gts_term(term_id as u64))),
    }
}

#[cfg(test)]
fn term_from_id_depth(
    graph: &Graph,
    term_id: usize,
    location: RdfLocation,
    depth: usize,
) -> Result<RdfTerm, RdfDiagnostic> {
    if depth > purrdf_events::MAX_TERM_NESTING_DEPTH {
        return Err(RdfDiagnostic::error(
            "gts-term-nesting-limit",
            "GTS term nesting depth limit exceeded",
        )
        .with_location(location.with_gts_term(term_id as u64)));
    }
    let term = graph.terms.get(term_id).ok_or_else(|| {
        RdfDiagnostic::error(
            "gts-term-out-of-range",
            format!("GTS term id {term_id} is out of range"),
        )
        .with_location(location.clone().with_gts_term(term_id as u64))
    })?;
    match term.kind {
        TermKind::Iri => {
            let Some(iri) = term.value.as_deref().filter(|value| !value.is_empty()) else {
                return Err(RdfDiagnostic::error(
                    "gts-iri-missing-value",
                    "GTS IRI term requires a non-empty value",
                )
                .with_location(location.with_gts_term(term_id as u64)));
            };
            Ok(RdfTerm::iri(iri))
        }
        TermKind::Bnode => Ok(RdfTerm::blank_node(
            term.value
                .clone()
                .unwrap_or_else(|| format!("gts_bnode_{term_id}")),
        )),
        TermKind::Literal => {
            let datatype = match term.datatype {
                Some(datatype_id) => {
                    match term_from_id_depth(graph, datatype_id, location.clone(), depth + 1)? {
                        RdfTerm::Iri(iri) => Some(iri),
                        other => {
                            return Err(RdfDiagnostic::error(
                                "gts-literal-datatype-not-iri",
                                format!(
                                    "GTS literal datatype must resolve to an IRI, got {:?}",
                                    other.kind()
                                ),
                            )
                            .with_location(location.with_gts_term(datatype_id as u64)));
                        }
                    }
                }
                None => None,
            };
            Ok(RdfTerm::literal(RdfLiteral {
                lexical_form: term.value.clone().unwrap_or_default(),
                datatype,
                language: term.lang.clone(),
                direction: parse_gts_direction(term.direction.as_deref(), term.lang.as_deref())?,
            }))
        }
        TermKind::Triple => {
            // A self-describing triple term (wire `"tt"`) states its own
            // components; only the older indirect spelling routes through a
            // reifier id, which `Graph::triple_of` resolves as a fallback. A
            // term with neither states no triple (§7.3) and is a blank node.
            if term.is_bare_triple() {
                return Ok(RdfTerm::blank_node(format!("unbound_triple_{term_id}")));
            }
            let Some((s, p, o)) = graph.triple_of(term_id) else {
                let (code, detail) = match term.reifier {
                    Some(reifier_id) => (
                        "gts-missing-reifier-binding",
                        format!("GTS triple term references missing reifier {reifier_id}"),
                    ),
                    None => (
                        "gts-unbound-triple-term",
                        "GTS triple term names neither its own components nor a reifier"
                            .to_string(),
                    ),
                };
                let mut location = location.with_gts_term(term_id as u64);
                if let Some(reifier_id) = term.reifier {
                    location = location.with_gts_reifier(reifier_id as u64);
                }
                return Err(RdfDiagnostic::error(code, detail).with_location(location));
            };
            let location = match term.reifier {
                Some(reifier_id) => location.with_gts_reifier(reifier_id as u64),
                None => location.with_gts_term(term_id as u64),
            };
            Ok(RdfTerm::triple(triple_from_ids_depth(
                graph,
                s,
                p,
                o,
                location,
                depth + 1,
            )?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_without_language_is_rejected() {
        // RDF 1.2 admits a base direction only on a language-tagged string.
        let err = parse_gts_direction(Some("ltr"), None)
            .expect_err("direction without a language tag must hard-fail");
        assert_eq!(err.code, "gts-direction-without-language");
        let err = parse_gts_direction(Some("rtl"), Some(""))
            .expect_err("direction with an empty language tag must hard-fail");
        assert_eq!(err.code, "gts-direction-without-language");
    }

    #[test]
    fn direction_with_language_round_trips() {
        assert_eq!(
            parse_gts_direction(Some("ltr"), Some("en")).unwrap(),
            Some(RdfTextDirection::Ltr)
        );
        assert_eq!(
            parse_gts_direction(Some("rtl"), Some("ar")).unwrap(),
            Some(RdfTextDirection::Rtl)
        );
        assert_eq!(parse_gts_direction(None, Some("en")).unwrap(), None);
        assert_eq!(parse_gts_direction(None, None).unwrap(), None);
    }

    #[test]
    fn unrecognized_direction_is_rejected() {
        let err = parse_gts_direction(Some("sideways"), Some("en"))
            .expect_err("unknown direction must hard-fail");
        assert_eq!(err.code, "gts-invalid-direction");
    }
}
