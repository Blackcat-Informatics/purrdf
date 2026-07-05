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
//! This module keeps the shared nesting bound and literal-direction parser used by
//! the production importers. A test-only eager resolver remains here for regression
//! coverage over malformed folded graphs.
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

#[cfg(test)]
use purrdf_gts::model::{Graph, TermKind};

use crate::{RdfDiagnostic, RdfTextDirection};
#[cfg(test)]
use crate::{RdfLiteral, RdfLocation, RdfTerm, RdfTriple};

/// Depth bound for resolving nested quoted-triple terms. A cyclic or absurdly
/// nested triple term hard-fails rather than recursing without bound. Shared by the
/// eager resolver here and the move-based importer in [`super::import_graph`].
pub(crate) const MAX_GTS_TERM_NESTING_DEPTH: usize = 16;

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
            let Some(reifier_id) = term.reifier else {
                return Err(RdfDiagnostic::error(
                    "gts-unbound-triple-term",
                    "GTS triple term has no reifier binding",
                )
                .with_location(location.with_gts_term(term_id)));
            };
            let Some((s, p, o)) = graph.reifier(reifier_id) else {
                return Err(RdfDiagnostic::error(
                    "gts-missing-reifier-binding",
                    format!("GTS triple term references missing reifier {reifier_id}"),
                )
                .with_location(location.with_gts_term(term_id).with_gts_reifier(reifier_id)));
            };
            Ok(RdfTerm::triple(triple_from_ids_depth(
                graph,
                s,
                p,
                o,
                location.with_gts_reifier(reifier_id),
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
