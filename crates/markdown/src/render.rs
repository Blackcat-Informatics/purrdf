// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Rendering: the parsed structure to one claim per node, each claim
//! the node's triples as sorted N-Triples lines.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use purrdf_core::ContentDigest;
use purrdf_core::embedding::ChunkingContractId;

use crate::parse::{Section, Structure, Unit};
use crate::{Claim, ClaimKind, Profile, SourceDocument, Vocabulary, section_iri, unit_iri};

pub(crate) fn render(
    doc: &SourceDocument<'_>,
    text: &str,
    profile: &Profile,
    contract: &ChunkingContractId,
    structure: &Structure,
) -> Vec<Claim> {
    let section_iris: Vec<String> = structure
        .sections
        .iter()
        .map(|s| {
            section_iri(
                &profile.vocabulary,
                doc.id,
                contract,
                s.start as u64,
                s.line_end as u64,
                &text.as_bytes()[s.start..s.line_end],
            )
        })
        .collect();
    let unit_iris: Vec<String> = structure
        .units
        .iter()
        .map(|u| {
            unit_iri(
                &profile.vocabulary,
                doc.id,
                contract,
                u.start as u64,
                u.end as u64,
                &text.as_bytes()[u.start..u.end],
            )
        })
        .collect();
    let citations = citations_by_unit(structure);

    let mut claims = Vec::with_capacity(1 + section_iris.len() + unit_iris.len());
    claims.push(document_claim(doc, text, profile, structure));
    let mut sections = structure.sections.iter().enumerate().peekable();
    let mut units = structure.units.iter().enumerate().peekable();
    // Document order: whichever of the next section and the next unit
    // starts first.
    loop {
        let take_section = match (sections.peek(), units.peek()) {
            (None, None) => break,
            (Some((_, s)), Some((_, u))) => s.start <= u.start,
            (Some(_), None) => true,
            (None, Some(_)) => false,
        };
        if take_section {
            if let Some((i, section)) = sections.next() {
                claims.push(section_claim(
                    doc,
                    &profile.vocabulary,
                    section,
                    &section_iris,
                    i,
                ));
            }
        } else if let Some((i, unit)) = units.next() {
            let cites = citations.get(&i).map(Vec::as_slice).unwrap_or_default();
            claims.push(unit_claim(
                doc,
                text,
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

/// Citations attach to the first piece of the verse unit whose number
/// they name, within this document; a range names each verse in it.
fn citations_by_unit(structure: &Structure) -> BTreeMap<usize, Vec<(String, bool)>> {
    let mut first_piece: BTreeMap<u64, usize> = BTreeMap::new();
    for (i, u) in structure.units.iter().enumerate() {
        if let (Some(v), None) = (u.verse, u.continues) {
            first_piece.entry(v).or_insert(i);
        }
    }
    let mut out: BTreeMap<usize, Vec<(String, bool)>> = BTreeMap::new();
    for c in &structure.citations {
        for verse in c.first..=c.last {
            let Some(&unit) = first_piece.get(&verse) else {
                continue;
            };
            let entry = out.entry(unit).or_default();
            entry.extend(c.anchors.iter().map(|a| (a.clone(), true)));
            entry.extend(c.sources.iter().map(|s| (s.clone(), false)));
        }
    }
    out
}

fn document_claim(
    doc: &SourceDocument<'_>,
    text: &str,
    profile: &Profile,
    structure: &Structure,
) -> Claim {
    let v = &profile.vocabulary;
    let mut lines = vec![
        triple(doc.id, crate::RDF_TYPE, &iri(&v.document_class)),
        triple(
            doc.id,
            &v.source_digest,
            &typed(
                &format!("sha256:{}", ContentDigest::of(text.as_bytes()).to_hex()),
                &v.dt_digest,
            ),
        ),
        triple(doc.id, &v.media_type, &typed("text/markdown", &v.dt_media)),
        triple(doc.id, &v.byte_length, &integer(text.len() as u64)),
        triple(
            doc.id,
            &v.slice_profile,
            &typed(&profile.label(), &v.dt_profile),
        ),
    ];
    if let Some(title) = &structure.title {
        lines.push(triple(doc.id, &v.title, &typed(title, &v.dt_heading)));
    }
    claim(lines, doc.id.to_owned(), ClaimKind::Document, None)
}

fn section_claim(
    doc: &SourceDocument<'_>,
    v: &Vocabulary,
    s: &Section,
    iris: &[String],
    i: usize,
) -> Claim {
    let me = &iris[i];
    let class = if s.movement {
        &v.movement_class
    } else {
        &v.section_class
    };
    let mut lines = vec![
        triple(me, crate::RDF_TYPE, &iri(class)),
        triple(me, &v.in_document, &iri(doc.id)),
        triple(me, &v.level, &integer(u64::from(s.level))),
        triple(me, &v.ordinal, &integer(s.ordinal)),
        triple(me, &v.heading, &typed(&s.heading, &v.dt_heading)),
        triple(me, &v.byte_start, &integer(s.start as u64)),
        triple(me, &v.byte_end, &integer(s.end as u64)),
    ];
    if let Some(p) = s.parent {
        lines.push(triple(me, &v.parent, &iri(&iris[p])));
    }
    claim(
        lines,
        me.clone(),
        ClaimKind::Section,
        Some((s.start as u64, s.end as u64)),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "one node's inputs; a struct would only rename them"
)]
fn unit_claim(
    doc: &SourceDocument<'_>,
    text: &str,
    profile: &Profile,
    u: &Unit,
    unit_iris: &[String],
    section_iris: &[String],
    i: usize,
    cites: &[(String, bool)],
) -> Claim {
    let v = &profile.vocabulary;
    let me = &unit_iris[i];
    let mut lines = vec![
        triple(me, crate::RDF_TYPE, &iri(&v.unit_class)),
        triple(me, &v.text, &literal(&text[u.start..u.end])),
        triple(me, &v.in_document, &iri(doc.id)),
        triple(me, &v.ordinal, &integer(u.ordinal)),
        triple(me, &v.byte_start, &integer(u.start as u64)),
        triple(me, &v.byte_end, &integer(u.end as u64)),
    ];
    if let Some(s) = u.section {
        lines.push(triple(me, &v.in_section, &iri(&section_iris[s])));
    }
    if let Some(number) = u.verse {
        lines.push(triple(me, &v.verse, &integer(number)));
    }
    if !u.lineage.is_empty() {
        lines.push(triple(
            me,
            &v.lineage,
            &typed(&u.lineage.join(" > "), &v.dt_lineage),
        ));
    }
    if let Some(p) = u.continues {
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
        Some((u.start as u64, u.end as u64)),
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
