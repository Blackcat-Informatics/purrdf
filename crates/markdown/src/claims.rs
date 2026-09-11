// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The claim projection: a [`Document`] to one claim per node, each
//! claim the node's triples as sorted N-Triples lines, under the
//! profile that document carries.
//!
//! The profile is read off the document rather than passed in, which is
//! what makes this module total: every law it applies is a law
//! [`analyze`](crate::analyze) already answered for on these very bytes.
//!
//! Dialect-independent, and still a *projection*: the model is the
//! finding and this is one view of it. What the view no longer drops is
//! the pairing inside a concordance row — a row's lift onto a unit is
//! its own node, and its sources hang on that node rather than on the
//! unit — and a unit's scalar offsets and content digest, which are
//! emitted as data. What stays only in the model is the part that names
//! no node at all: the rows that lifted nothing, the rows too malformed
//! to read, and a unit's surrounding context.
//!
//! Every citation node states its class and the unit it is an edge of,
//! and states them *unconditionally*. A row that named source paths and
//! no anchors mints a node that reifies nothing, and without those two
//! lines nothing in the graph would point at it: its sources would be
//! emitted and unreachable, which is the silent drop in its quietest
//! spelling. Two lines per citation node on every document is the price,
//! and it is stated plainly here because it is paid on every document.
//!
//! Every term this module writes goes through `purrdf-core`'s canonical
//! writers ([`emit_term`]), so the crate's escaping is the kernel's
//! escaping by construction rather than by resemblance.

use std::collections::BTreeMap;

use purrdf_core::embedding::ChunkingContractId;
use purrdf_core::{ContentDigest, RdfLiteral, RdfTerm, RdfTriple, emit_term};

use crate::identity::{citation_iri, node_iri_of_digest};
use crate::model::{Document, Section, Unit};
use crate::profile::{Profile, Vocabulary};
use crate::{Claim, ClaimKind};

/// Projects an analyzed document into claims under the law it was
/// admitted under: [`Document::profile`], and no other.
///
/// Infallible, and the document's own shape is what makes it so. A
/// [`Document`] can only be obtained from [`analyze`](crate::analyze),
/// which has already answered for the profile's vocabulary, its
/// constants, the source id, the encoding, and every concordance anchor
/// the document names under that profile's canon base — and it *keeps*
/// that profile, so the law answered for and the law applied here are
/// the same value. There is nothing left to refuse and nothing left to
/// pass in.
///
/// That the projection takes no profile is the whole of the guarantee.
/// A second profile would be a second law — a different vocabulary, a
/// different bound, a different canon base — asked of a document that
/// was never checked against it, and an anchor admitted as a literal
/// because no base was declared would mint an unchecked IRI the moment
/// one was. The argument does not exist, so neither does that.
///
/// Claims come in document order: the document node first, then
/// sections and units interleaved as they occur.
#[must_use]
pub fn render(document: &Document<'_>) -> Vec<Claim> {
    let profile = document.profile();
    // The **chunking** id, which is the third field of every node
    // preimage: a node is addressed by the law that cut it, not by the
    // law that words it, so a new vocabulary or a newly declared canon
    // base describes the same nodes rather than minting new ones. The
    // wider law still reaches the graph — as the `sliceProfile` literal
    // of the document node, through [`Profile::label`].
    let contract = profile.chunking_id();
    let source = document.source();
    let section_iris: Vec<String> = document
        .sections()
        .iter()
        .map(|s| {
            let span = s.heading_span();
            node_iri_of_digest(
                &profile.vocabulary,
                "section",
                document.id(),
                &contract,
                span.start,
                span.end,
                &ContentDigest::of(&source.as_bytes()[span.start as usize..span.end as usize]),
            )
        })
        .collect();
    let unit_iris: Vec<String> = document
        .units()
        .iter()
        .map(|u| {
            node_iri_of_digest(
                &profile.vocabulary,
                "unit",
                document.id(),
                &contract,
                u.span().start,
                u.span().end,
                &u.digest(),
            )
        })
        .collect();
    let citations = citation_edges(document, profile, &contract, &unit_iris);

    let mut claims = Vec::with_capacity(1 + section_iris.len() + unit_iris.len());
    claims.push(document_claim(document));
    let mut sections = document.sections().iter().enumerate().peekable();
    let mut units = document.units().iter().enumerate().peekable();
    // Document order: whichever of the next section and the next unit
    // starts first.
    loop {
        let take_section = match (sections.peek(), units.peek()) {
            (None, None) => break,
            (Some((_, s)), Some((_, u))) => s.span().start <= u.span().start,
            (Some(_), None) => true,
            (None, Some(_)) => false,
        };
        if take_section {
            if let Some((i, section)) = sections.next() {
                claims.push(section_claim(
                    document.id(),
                    &profile.vocabulary,
                    section,
                    &section_iris,
                    i,
                ));
            }
        } else if let Some((i, unit)) = units.next() {
            let cites = citations.get(&i).map(Vec::as_slice).unwrap_or_default();
            claims.push(unit_claim(
                document.id(),
                profile,
                unit,
                &unit_iris,
                &section_iris,
                i,
                cites,
            ));
        }
    }
    claims
}

/// One concordance row's lift onto one unit: the node that keeps the
/// row's own anchors beside the row's own sources.
struct CitationEdge<'d> {
    /// The edge's content-addressed node IRI.
    iri: String,
    /// The row's canon source paths, in the order the row wrote them.
    sources: &'d [String],
    /// The row's anchors as the **objects the graph carries**, written
    /// in the order the row wrote them: the IRI `base ++ anchor` under a
    /// declared canon base, the anchor as a typed literal under none.
    /// These are the objects of the unit's asserted `cites` edges.
    anchors: Vec<String>,
    /// The triple terms this node reifies, one per anchor and in the
    /// same order: `<<( <unit> <cites> <anchor> )>>`, rendered.
    ///
    /// Rendered once, here, and then stated twice — as the object of
    /// this node's `rdf:reifies` line, and as a field of this node's own
    /// identity. Rendering it once is what keeps what the node *is* and
    /// what the node *says* from parting company.
    reified: Vec<String>,
}

/// One citation edge per (row, unit) the row lifted onto, indexed by
/// unit.
///
/// This is where the flat projection's loss is repaired. A row that
/// covers a verse another row also covers used to hand the unit a pile
/// of anchors and a pile of paths with nothing to say which came from
/// where; here each row's lift is its own node, so its paths annotate
/// its own anchors and a consumer reading the triples back can put the
/// row together again.
fn citation_edges<'d>(
    document: &'d Document<'_>,
    profile: &Profile,
    contract: &ChunkingContractId,
    unit_iris: &[String],
) -> BTreeMap<usize, Vec<CitationEdge<'d>>> {
    let source = document.source().as_bytes();
    let vocabulary = &profile.vocabulary;
    let mut out: BTreeMap<usize, Vec<CitationEdge<'d>>> = BTreeMap::new();
    for row in document.citations() {
        let span = row.span();
        let line = &source[span.start as usize..span.end as usize];
        // Minted before the node is addressed, because the node is
        // addressed *by* them: what a reifier reifies is part of what it
        // is, so one row read under two canon bases — or under two
        // spellings of the citing predicate — is two nodes.
        let objects: Vec<RdfTerm> = row
            .anchors()
            .iter()
            .map(|anchor| match &profile.canon_base {
                Some(base) => RdfTerm::Iri(format!("{base}{anchor}")),
                None => typed_term(anchor, &vocabulary.dt_anchor),
            })
            .collect();
        let anchors: Vec<String> = objects.iter().map(emit_term).collect();
        for &(_, unit) in row.lifted() {
            let me = &unit_iris[unit];
            let reified: Vec<String> = objects
                .iter()
                .map(|object| {
                    emit_term(&RdfTerm::triple(RdfTriple::new(
                        RdfTerm::Iri(me.clone()),
                        vocabulary.cites.as_str(),
                        object.clone(),
                    )))
                })
                .collect();
            out.entry(unit).or_default().push(CitationEdge {
                iri: citation_iri(
                    vocabulary,
                    document.id(),
                    contract,
                    span.start,
                    span.end,
                    line,
                    me,
                    &reified,
                ),
                sources: row.sources(),
                anchors: anchors.clone(),
                reified,
            });
        }
    }
    out
}

fn document_claim(document: &Document<'_>) -> Claim {
    let profile = document.profile();
    let v = &profile.vocabulary;
    let id = document.id();
    let mut lines = vec![
        triple(id, crate::RDF_TYPE, &iri(&v.document_class)),
        triple(
            id,
            &v.source_digest,
            &typed(
                &format!(
                    "sha256:{}",
                    ContentDigest::of(document.source().as_bytes()).to_hex()
                ),
                &v.dt_digest,
            ),
        ),
        triple(id, &v.media_type, &typed("text/markdown", &v.dt_media)),
        triple(id, &v.byte_length, &integer(document.byte_length())),
        triple(
            id,
            &v.slice_profile,
            &typed(&profile.label(), &v.dt_profile),
        ),
    ];
    if let Some(title) = document.title() {
        lines.push(triple(id, &v.title, &typed(title, &v.dt_heading)));
    }
    claim(lines, id.to_owned(), ClaimKind::Document, None)
}

fn section_claim(
    document_id: &str,
    v: &Vocabulary,
    s: &Section,
    iris: &[String],
    i: usize,
) -> Claim {
    let me = &iris[i];
    let class = if s.is_movement() {
        &v.movement_class
    } else {
        &v.section_class
    };
    let span = s.span();
    let mut lines = vec![
        triple(me, crate::RDF_TYPE, &iri(class)),
        triple(me, &v.in_document, &iri(document_id)),
        triple(me, &v.level, &integer(u64::from(s.level()))),
        triple(me, &v.ordinal, &integer(s.ordinal())),
        triple(me, &v.heading, &typed(s.heading(), &v.dt_heading)),
        triple(me, &v.byte_start, &integer(span.start)),
        triple(me, &v.byte_end, &integer(span.end)),
    ];
    if let Some(p) = s.parent() {
        lines.push(triple(me, &v.parent, &iri(&iris[p])));
    }
    claim(
        lines,
        me.clone(),
        ClaimKind::Section,
        Some((span.start, span.end)),
    )
}

fn unit_claim(
    document_id: &str,
    profile: &Profile,
    u: &Unit<'_>,
    unit_iris: &[String],
    section_iris: &[String],
    i: usize,
    cites: &[CitationEdge<'_>],
) -> Claim {
    let v = &profile.vocabulary;
    let me = &unit_iris[i];
    let span = u.span();
    let scalars = u.scalar_span();
    let mut lines = vec![
        triple(me, crate::RDF_TYPE, &iri(&v.unit_class)),
        triple(me, &v.text, &literal(u.quote())),
        triple(me, &v.in_document, &iri(document_id)),
        triple(me, &v.ordinal, &integer(u.ordinal())),
        triple(me, &v.byte_start, &integer(span.start)),
        triple(me, &v.byte_end, &integer(span.end)),
        triple(me, &v.scalar_start, &integer(scalars.start)),
        triple(me, &v.scalar_end, &integer(scalars.end)),
        triple(
            me,
            &v.content_digest,
            &typed(&u.digest().to_hex(), crate::XSD_HEX_BINARY),
        ),
    ];
    if let Some(s) = u.section() {
        lines.push(triple(me, &v.in_section, &iri(&section_iris[s])));
    }
    if let Some(number) = u.verse() {
        lines.push(triple(me, &v.verse, &integer(number)));
    }
    if !u.lineage().is_empty() {
        lines.push(triple(
            me,
            &v.lineage,
            &typed(&u.lineage().join(" > "), &v.dt_lineage),
        ));
    }
    if let Some(p) = u.continues() {
        lines.push(triple(me, &v.continues, &iri(&unit_iris[p])));
    }
    for edge in cites {
        // What the node is, and what it belongs to — stated for every
        // citation node, whatever its row lifted. A row that named
        // sources and no anchors reifies nothing, so these two lines are
        // the only thing between its node and an orphan: a consumer
        // walking out from the unit would otherwise never reach it, and
        // the row's sources would be in the graph and out of reach.
        lines.push(triple(&edge.iri, crate::RDF_TYPE, &iri(&v.citation_class)));
        lines.push(triple(&edge.iri, &v.in_unit, &iri(me)));
        // The asserted edge and the triple term the row's node reifies,
        // both read off values rendered once in `citation_edges` — and
        // the triple term is the very value the node's own identity was
        // taken over, so what it is and what it says cannot part
        // company however an anchor is spelled.
        for (object, reified) in edge.anchors.iter().zip(&edge.reified) {
            lines.push(triple(me, &v.cites, object));
            lines.push(triple(&edge.iri, crate::RDF_REIFIES, reified));
        }
        for path in edge.sources {
            lines.push(triple(&edge.iri, &v.canon_source, &typed(path, &v.dt_path)));
        }
    }
    claim(
        lines,
        me.clone(),
        ClaimKind::Unit,
        Some((span.start, span.end)),
    )
}

fn claim(
    mut lines: Vec<String>,
    subject: String,
    kind: ClaimKind,
    span: Option<(u64, u64)>,
) -> Claim {
    lines.sort_unstable();
    lines.dedup();
    let mut turtle = String::with_capacity(lines.iter().map(|l| l.len() + 1).sum());
    for line in lines {
        turtle.push_str(&line);
        turtle.push('\n');
    }
    Claim {
        turtle,
        subject,
        kind,
        span,
    }
}

/// One N-Triples line, its object already rendered.
///
/// The subject goes through the canonical writer; the predicate is
/// written bare between `<` and `>`, exactly as `purrdf-core`'s own quad
/// writer does, because a predicate is an IRI the profile already
/// answered for.
fn triple(subject: &str, predicate: &str, object: &str) -> String {
    format!("{} <{predicate}> {object} .", iri(subject))
}

fn iri(value: &str) -> String {
    emit_term(&RdfTerm::Iri(value.to_owned()))
}

/// A plain literal: an `xsd:string`, written with no datatype IRI.
fn literal(value: &str) -> String {
    emit_term(&RdfTerm::Literal(RdfLiteral {
        lexical_form: value.to_owned(),
        datatype: None,
        language: None,
        direction: None,
    }))
}

/// A datatyped literal **term**, for a position that needs the term
/// rather than its rendering — the object of a triple term.
fn typed_term(value: &str, datatype: &str) -> RdfTerm {
    RdfTerm::Literal(RdfLiteral {
        lexical_form: value.to_owned(),
        datatype: Some(datatype.to_owned()),
        language: None,
        direction: None,
    })
}

fn typed(value: &str, datatype: &str) -> String {
    emit_term(&typed_term(value, datatype))
}

fn integer(value: u64) -> String {
    typed(&value.to_string(), crate::XSD_INTEGER)
}

/// Characters that cannot appear inside an N-Triples IRI reference.
pub(crate) fn iri_forbids(c: char) -> bool {
    matches!(c, '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\') || (c as u32) <= 0x20
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_covers_the_named_escapes_the_controls_and_the_delete() {
        assert_eq!(
            literal("a\"b\\c\nd\re\tf\u{1}\u{7f}"),
            "\"a\\\"b\\\\c\\nd\\re\\tf\\u0001\\u007F\""
        );
        assert_eq!(
            literal("curly \u{201c}quotes\u{201d} \u{2013}"),
            "\"curly \u{201c}quotes\u{201d} \u{2013}\""
        );
        assert_eq!(
            typed("a\u{7f}b", "urn:test:dt"),
            "\"a\\u007Fb\"^^<urn:test:dt>"
        );
    }
}
