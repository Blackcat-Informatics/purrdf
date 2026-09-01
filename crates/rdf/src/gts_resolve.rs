// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The shared GTS term resolver (C2).
//!
//! GTS graph readers and the consuming [`super::import_graph`] importer both fold a
//! *folded* `purrdf_gts::model::Graph` into RDF terms, and both need the SAME
//! depth-bounded structural traversal: term-kind dispatch, the non-empty-IRI and
//! datatype-must-be-IRI checks, reifier lookup for quoted-triple terms, and the
//! cyclic-nesting depth guard.
//!
//! This module keeps the shared nesting bound, the shared **termination check**
//! ([`ensure_terms_terminate`]) every caller-built graph is admitted through, and the
//! literal-direction parser used by the production importers. A test-only eager
//! resolver remains here for regression coverage over malformed folded graphs.
//!
//! The consuming `import_graph` importer cannot reuse these directly: it consumes a
//! `Graph` *by value* and MOVES term strings into the interner, which is structurally
//! incompatible with borrowing the same `Graph` for a clone-based resolver. It
//! therefore mirrors this traversal in move form, sharing the
//! [`MAX_GTS_TERM_NESTING_DEPTH`] bound and the `gts-*` diagnostic codes so the two
//! cannot drift on structural contract.
//!
//! The diagnostic codes here are the historical `gts-*` codes (preserved verbatim
//! from the original `gts.rs` implementation), so error contracts are unchanged by
//! the extraction.

use purrdf_gts::model::{Graph, TermKind};

use crate::{RdfDiagnostic, RdfTextDirection};
#[cfg(test)]
use crate::{RdfLiteral, RdfLocation, RdfTerm, RdfTriple};

/// Depth bound for resolving nested quoted-triple terms. A cyclic or absurdly
/// nested triple term hard-fails rather than recursing without bound. Shared by the
/// eager resolver here and the move-based importer in [`super::import_graph`].
pub(crate) const MAX_GTS_TERM_NESTING_DEPTH: usize = 16;

/// The outgoing structural edges of one term: the ids it makes another walker
/// resolve. Three is the maximum (a quoted triple's `(s, p, o)`); a literal
/// contributes one (its datatype term) and an IRI or blank node none.
type TermEdges = ([usize; 3], usize);

/// Refuse a folded [`Graph`] whose term table lets a term reach ITSELF.
///
/// GTS-SPEC §7.3 makes termination normative for EVERY walk of a triple term's
/// resolved components — the segment union and every projection or re-authoring
/// pass alike. A term reaches another through exactly two edges, and both are
/// followed recursively by this crate's walkers:
///
/// * a quoted-triple term reaches its `(s, p, o)` through
///   [`Graph::triple_of`](purrdf_gts::model::Graph::triple_of), which prefers the
///   self-describing `tt` components and otherwise resolves the statement layer
///   through the reifier id the term names — or, when it names none, through its
///   OWN id (§7.1, a self-bound triple term may leave `rf` implicit);
/// * a literal reaches its datatype term.
///
/// A cycle over those edges makes every such walk non-terminating, and an unbounded
/// recursion in Rust overflows the stack, which **aborts the process** — it is not a
/// catchable panic, so no binding can contain it. The GTS reader already refuses a
/// `reifies` row that would close such a loop, which closes the wire route; a graph
/// assembled by a caller (`GtsFoldView::new`, the Python `from_parts`, any consumer
/// holding a `purrdf_gts::model::Graph`) carries no such guarantee, so it is checked
/// HERE, once, at every boundary where one enters this crate.
///
/// This is the fold-time refusal §7.3 permits, taken in preference to guarding each
/// walker: one O(terms + edges) pass paid once per graph, against a visited set or a
/// depth counter threaded through every render, every serialization and every fold.
/// The walk below is iterative — a recursive acyclicity check would abort on exactly
/// the input it exists to reject.
///
/// # Errors
///
/// Returns `gts-self-reaching-term` naming the first term that reaches itself. An
/// out-of-range component id is NOT this check's business and is skipped: it names no
/// term, so it closes no loop, and the resolvers report it under their own range codes
/// (`gts-term-out-of-range`, `native-codec-term-out-of-range`) when they reach it.
/// Reporting it here would put a range error under a misleading code.
pub(crate) fn ensure_terms_terminate(graph: &Graph) -> Result<(), RdfDiagnostic> {
    /// Never visited.
    const UNSEEN: u8 = 0;
    /// On the current DFS path — reaching it again is a cycle.
    const ON_PATH: u8 = 1;
    /// Fully explored and proven to terminate.
    const SETTLED: u8 = 2;

    let mut state = vec![UNSEEN; graph.terms.len()];
    // One frame per term on the current path: its id, its outgoing edges resolved
    // once, and how many of them have been followed.
    let mut path: Vec<(usize, TermEdges, usize)> = Vec::new();

    for root in 0..graph.terms.len() {
        if state[root] != UNSEEN {
            continue;
        }
        state[root] = ON_PATH;
        path.push((root, term_edges(graph, root), 0));
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
                    path.push((next, term_edges(graph, next), 0));
                }
            }
        }
    }
    Ok(())
}

/// The term ids `term_id` makes a walker resolve, in the order the walkers follow
/// them. This MUST mirror what those walkers actually read: an edge omitted here is
/// a loop the check admits and the first walker then dies on.
fn term_edges(graph: &Graph, term_id: usize) -> TermEdges {
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
        // `Graph::triple_of` is the ONE place a folded quoted triple's components are
        // resolved, so it is the one edge set that matters here.
        TermKind::Triple => match graph.triple_of(term_id) {
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
    let direction = match value {
        None => return Ok(None),
        Some("ltr") => RdfTextDirection::Ltr,
        Some("rtl") => RdfTextDirection::Rtl,
        Some(other) => {
            return Err(RdfDiagnostic::error(
                "gts-invalid-direction",
                format!("unrecognized GTS literal base direction {other:?}"),
            ));
        }
    };
    if language.is_none_or(str::is_empty) {
        return Err(RdfDiagnostic::error(
            "gts-direction-without-language",
            "an RDF 1.2 literal base direction requires a non-empty language tag",
        ));
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
        .with_location(location.with_gts_term(term_id))),
    }
}

#[cfg(test)]
fn term_from_id_depth(
    graph: &Graph,
    term_id: usize,
    location: RdfLocation,
    depth: usize,
) -> Result<RdfTerm, RdfDiagnostic> {
    if depth > MAX_GTS_TERM_NESTING_DEPTH {
        return Err(RdfDiagnostic::error(
            "gts-term-nesting-limit",
            "GTS term nesting depth limit exceeded",
        )
        .with_location(location.with_gts_term(term_id)));
    }
    let term = graph.terms.get(term_id).ok_or_else(|| {
        RdfDiagnostic::error(
            "gts-term-out-of-range",
            format!("GTS term id {term_id} is out of range"),
        )
        .with_location(location.clone().with_gts_term(term_id))
    })?;
    match term.kind {
        TermKind::Iri => {
            let Some(iri) = term.value.as_deref().filter(|value| !value.is_empty()) else {
                return Err(RdfDiagnostic::error(
                    "gts-iri-missing-value",
                    "GTS IRI term requires a non-empty value",
                )
                .with_location(location.with_gts_term(term_id)));
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
                            .with_location(location.with_gts_term(datatype_id)));
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
            // reifier id, which `Graph::triple_of` resolves as a fallback.
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
                let mut location = location.with_gts_term(term_id);
                if let Some(reifier_id) = term.reifier {
                    location = location.with_gts_reifier(reifier_id);
                }
                return Err(RdfDiagnostic::error(code, detail).with_location(location));
            };
            let location = match term.reifier {
                Some(reifier_id) => location.with_gts_reifier(reifier_id),
                None => location.with_gts_term(term_id),
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
