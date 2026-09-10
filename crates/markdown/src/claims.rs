// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The claim projection: a [`Document`] to one claim per node, each
//! claim the node's triples as sorted N-Triples lines.
//!
//! Dialect-independent, and — deliberately — *lossy*. The model is the
//! finding; this is one view of it, flattened for a triple store. What
//! the flattening drops (which anchors a row named beside which
//! sources, which rows lifted nothing, a unit's scalar span, its
//! context) stays in the model for a consumer that wants it.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use purrdf_core::ContentDigest;

use crate::identity::node_iri_of_digest;
use crate::model::{Document, Section, Unit};
use crate::profile::{Profile, Vocabulary};
use crate::{Claim, ClaimKind};

/// Projects an analyzed document into claims under a profile.
///
/// Infallible, and it is the seam that makes it so: a [`Document`] can
/// only be obtained from [`analyze`](crate::analyze), which has already
/// answered for the profile's vocabulary, its constants, the source id,
/// the encoding, and every concordance anchor the document names under
/// the profile's canon base. There is nothing left here to refuse.
///
/// Render under the profile the document was analyzed with. A different
/// profile is a different law — a different vocabulary, a different
/// bound, a different canon base — and it was never asked of this
/// document.
///
/// Claims come in document order: the document node first, then
/// sections and units interleaved as they occur.
#[must_use]
pub fn render(document: &Document<'_>, profile: &Profile) -> Vec<Claim> {
    let contract = profile.contract_id();
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
    let citations = citations_by_unit(document);

    let mut claims = Vec::with_capacity(1 + section_iris.len() + unit_iris.len());
    claims.push(document_claim(document, profile));
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

/// The citations of each unit, flattened: a row states one `cites` per
/// anchor and one `canonSource` per path on every unit it lifted onto,
/// and the pairing between the two is what the flattening loses.
fn citations_by_unit(document: &Document<'_>) -> BTreeMap<usize, Vec<(String, bool)>> {
    let mut out: BTreeMap<usize, Vec<(String, bool)>> = BTreeMap::new();
    for citation in document.citations() {
        for &(_, unit) in citation.lifted() {
            let entry = out.entry(unit).or_default();
            entry.extend(citation.anchors().iter().map(|a| (a.clone(), true)));
            entry.extend(citation.sources().iter().map(|s| (s.clone(), false)));
        }
    }
    out
}

fn document_claim(document: &Document<'_>, profile: &Profile) -> Claim {
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
    cites: &[(String, bool)],
) -> Claim {
    let v = &profile.vocabulary;
    let me = &unit_iris[i];
    let span = u.span();
    let mut lines = vec![
        triple(me, crate::RDF_TYPE, &iri(&v.unit_class)),
        triple(me, &v.text, &literal(u.quote())),
        triple(me, &v.in_document, &iri(document_id)),
        triple(me, &v.ordinal, &integer(u.ordinal())),
        triple(me, &v.byte_start, &integer(span.start)),
        triple(me, &v.byte_end, &integer(span.end)),
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
    for (name, is_anchor) in cites {
        if *is_anchor {
            let object = match &profile.canon_base {
                Some(base) => iri(&format!("{base}{name}")),
                None => typed(name, &v.dt_anchor),
            };
            lines.push(triple(me, &v.cites, &object));
        } else {
            lines.push(triple(me, &v.canon_source, &typed(name, &v.dt_path)));
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

fn triple(subject: &str, predicate: &str, object: &str) -> String {
    format!("<{subject}> <{predicate}> {object} .")
}

fn iri(value: &str) -> String {
    format!("<{value}>")
}

fn literal(value: &str) -> String {
    format!("\"{}\"", escape(value))
}

fn typed(value: &str, datatype: &str) -> String {
    format!("\"{}\"^^<{datatype}>", escape(value))
}

fn integer(value: u64) -> String {
    format!("\"{value}\"^^<{}>", crate::XSD_INTEGER)
}

/// N-Triples string escaping: the five named escapes, and `\uXXXX` for
/// any other control character.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

/// Characters that cannot appear inside an N-Triples IRI reference.
pub(crate) fn iri_forbids(c: char) -> bool {
    matches!(c, '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\') || (c as u32) <= 0x20
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_covers_the_named_escapes_and_control_characters() {
        assert_eq!(
            escape("a\"b\\c\nd\re\tf\u{1}"),
            "a\\\"b\\\\c\\nd\\re\\tf\\u0001"
        );
        assert_eq!(
            escape("curly \u{201c}quotes\u{201d} \u{2013}"),
            "curly \u{201c}quotes\u{201d} \u{2013}"
        );
    }
}
