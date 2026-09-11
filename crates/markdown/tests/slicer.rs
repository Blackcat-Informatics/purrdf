// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Vectors for the structural slicer: determinism, structure, the split
//! law, the concordance, identity, and the goldens a declared profile
//! reproduces byte for byte.

use std::collections::{BTreeMap, BTreeSet};

use pretty_assertions::assert_eq;
use purrdf_core::embedding::{
    AppliedStage, ChunkingContractId, CorpusTarget, DocumentTarget, EmbeddingError, TargetId,
    derive_chunking_contract_id,
};
use purrdf_core::{BaseIri, CanonHash, ContentDigest, parse_iri, try_canonicalize_with};
use purrdf_markdown::{
    CONTEXT_BYTES, Claim, ClaimKind, DIGEST_ALGORITHM, Document, MIN_MAX_BYTES, MarkdownError,
    PURREMB_PARAMETER_ENCODING, Profile, RowDefect, STANDARD_NAMESPACE, SourceDocument, Span,
    SpanRelation, Unit, Vocabulary, analyze, render, section_iri, slice_markdown, span_relation,
    structure_iri, unit_iri, verify_unit,
};
use purrdf_rdf::parse_dataset;

/// A guide of numbered verses with movement markers, nested headings,
/// multi-byte scalars, oversize units, and a concordance table: the
/// authored shape the slicer was written against.
const GUIDE: &str = include_str!("fixtures/field-guide.md");
const GUIDE_ID: &str = "https://example.org/doc/field-guide";
/// The guide sliced under the declared profile below: the claims this
/// crate must reproduce byte for byte.
const GUIDE_GOLDEN: &str = include_str!("fixtures/field-guide.nt");
/// The same, with a canon base declared.
const GUIDE_GOLDEN_CANON: &str = include_str!("fixtures/field-guide.canon.nt");
const SLICE_BASE: &str = "https://example.org/slice/";
const PROFILE_NAME: &str = "example-slice-md-v1";
const CANON_BASE: &str = "https://example.org/canon#";
const TITLE: &str = "A Field Guide to the Marrow Archipelago";

fn v() -> Vocabulary {
    Vocabulary::under(SLICE_BASE).expect("a vocabulary")
}

/// The declared profile: the goldens are minted under it.
fn v1() -> Profile {
    Profile::new(PROFILE_NAME, 1, v())
}

/// The same law under a reduced bound, so the fixture's long units
/// split.
fn small() -> Profile {
    let mut profile = v1();
    profile.max_bytes = 200;
    profile.overlap = 40;
    profile
}

/// The triple terms a row's lift onto one unit reifies, written as the
/// graph writes them: `<<( <unit> <cites> <anchor> )>>`, the anchor
/// being the IRI `base ++ anchor` under a declared canon base and a
/// typed literal under none.
///
/// It is spelled out here rather than borrowed from the crate, because
/// it is the field a consumer re-deriving a citation node has to build
/// for itself out of the row, the unit and the profile it holds. The
/// guide's anchors need no escaping, so the plain format is the
/// canonical one.
fn reified_terms(row: &purrdf_markdown::Citation, profile: &Profile, unit: &str) -> Vec<String> {
    row.anchors()
        .iter()
        .map(|anchor| {
            let object = match &profile.canon_base {
                Some(base) => format!("<{base}{anchor}>"),
                None => format!("\"{anchor}\"^^<{}>", profile.vocabulary.dt_anchor),
            };
            format!("<<( <{unit}> <{}> {object} )>>", profile.vocabulary.cites)
        })
        .collect()
}

fn slice(text: &str, profile: &Profile) -> Vec<Claim> {
    slice_markdown(
        &SourceDocument {
            id: GUIDE_ID,
            bytes: text.as_bytes(),
        },
        profile,
    )
    .expect("slices")
}

/// Every claim of the guide under a profile, concatenated: the shape a
/// golden records.
fn rendered(profile: &Profile) -> String {
    slice(GUIDE, profile)
        .iter()
        .map(|c| c.turtle.as_str())
        .collect()
}

/// `(predicate, object)` pairs of the lines a claim states about **its
/// own** node, objects as rendered. A claim's lines are sorted bytewise,
/// so repeated predicates come back in object order.
///
/// A unit's claim also carries the triples of the citation nodes minted
/// for the concordance rows that lifted onto it; those are
/// [`citation_triples`], and
/// [`every_line_of_a_claim_is_its_own_node_or_a_citation_of_it`] is what
/// keeps this split from hiding a line neither reader looks at.
fn pairs(claim: &Claim) -> Vec<(String, String)> {
    let prefix = format!("<{}> ", claim.subject);
    claim
        .turtle
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .map(|rest| {
            let rest = rest.strip_suffix(" .").expect("terminator");
            let (p, o) = rest.split_once(' ').expect("predicate then object");
            (
                p.trim_start_matches('<').trim_end_matches('>').to_owned(),
                o.to_owned(),
            )
        })
        .collect()
}

/// `(subject, predicate, object)` of every line of a claim whose
/// subject is **not** the claim's own node: exactly the citation edges
/// of a unit.
fn citation_triples(claim: &Claim) -> Vec<(String, String, String)> {
    let prefix = format!("<{}> ", claim.subject);
    claim
        .turtle
        .lines()
        .filter(|line| !line.starts_with(&prefix))
        .map(|line| {
            let rest = line.strip_suffix(" .").expect("terminator");
            let (s, rest) = rest.split_once("> ").expect("a subject, then the rest");
            let (p, o) = rest.split_once(' ').expect("predicate then object");
            (
                s.trim_start_matches('<').to_owned(),
                p.trim_start_matches('<').trim_end_matches('>').to_owned(),
                o.to_owned(),
            )
        })
        .collect()
}

/// The objects a citation node states under a predicate, in the order
/// the claim's sorted lines give them.
fn citation_objects(claim: &Claim, citation: &str, predicate: &str) -> Vec<String> {
    citation_triples(claim)
        .into_iter()
        .filter(|(s, p, _)| s == citation && p == predicate)
        .map(|(_, _, o)| o)
        .collect()
}

/// The citation nodes a claim carries, in first-seen order, without
/// repeats.
fn citation_nodes(claim: &Claim) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (subject, _, _) in citation_triples(claim) {
        if !out.contains(&subject) {
            out.push(subject);
        }
    }
    out
}

fn object(claim: &Claim, predicate: &str) -> Option<String> {
    pairs(claim)
        .into_iter()
        .find(|(p, _)| p == predicate)
        .map(|(_, o)| o)
}

fn objects(claim: &Claim, predicate: &str) -> Vec<String> {
    pairs(claim)
        .into_iter()
        .filter(|(p, _)| p == predicate)
        .map(|(_, o)| o)
        .collect()
}

fn integer(claim: &Claim, predicate: &str) -> Option<u64> {
    object(claim, predicate).map(|o| {
        o.split_once("\"^^")
            .expect("typed")
            .0
            .trim_start_matches('"')
            .parse()
            .expect("integer")
    })
}

fn unescape(literal: &str) -> String {
    let inner = literal
        .strip_prefix('"')
        .and_then(|l| l.strip_suffix('"'))
        .expect("plain literal");
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                out.push(
                    char::from_u32(u32::from_str_radix(&hex, 16).expect("hex")).expect("char"),
                );
            }
            other => panic!("unexpected escape {other:?}"),
        }
    }
    out
}

fn units(claims: &[Claim]) -> Vec<&Claim> {
    claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Unit)
        .collect()
}

fn sections(claims: &[Claim]) -> Vec<&Claim> {
    claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Section)
        .collect()
}

/// A claim's byte span as offsets into the text it was sliced from.
fn span_of(claim: &Claim) -> (usize, usize) {
    let (start, end) = claim.span.expect("span");
    (start as usize, end as usize)
}

/// The lexical form of a typed literal object, without its datatype.
fn typed_value(claim: &Claim, predicate: &str) -> Option<String> {
    object(claim, predicate).map(|o| {
        o.split_once("\"^^")
            .expect("typed")
            .0
            .trim_start_matches('"')
            .to_owned()
    })
}

/// The lexical form of a **plain** literal object — a heading's words —
/// refusing a typed one: plainness is the assertion, not a formatting
/// detail, because plain is what a text index selects on.
fn plain_value(claim: &Claim, predicate: &str) -> Option<String> {
    object(claim, predicate).map(|o| {
        assert!(!o.contains("\"^^"), "a plain literal carries no datatype");
        o.trim_start_matches('"').trim_end_matches('"').to_owned()
    })
}

/// The continuation chains of a slice: the pieces of one unit in order,
/// a new chain opening at every piece that continues nothing. A unit
/// that never split is a chain of one.
fn chains(claims: &[Claim]) -> Vec<Vec<&Claim>> {
    let mut out: Vec<Vec<&Claim>> = Vec::new();
    for unit in units(claims) {
        if object(unit, &v().continues).is_some() {
            out.last_mut()
                .expect("a continuation follows the piece it continues")
                .push(unit);
        } else {
            out.push(vec![unit]);
        }
    }
    out
}

/// Whether a section claim is a movement marker rather than a heading.
fn is_movement(claim: &Claim) -> bool {
    object(claim, purrdf_markdown::RDF_TYPE).as_deref()
        == Some(&*format!("<{}>", v().movement_class))
}

/// Every split unit the very next claim of the document is an ATX
/// heading for, paired with that heading.
fn split_chains_before_a_heading(claims: &[Claim]) -> Vec<(Vec<&Claim>, &Claim)> {
    let mut out = Vec::new();
    let mut chain: Vec<&Claim> = Vec::new();
    for claim in claims {
        match claim.kind {
            ClaimKind::Unit => {
                if object(claim, &v().continues).is_none() {
                    chain.clear();
                }
                chain.push(claim);
            }
            ClaimKind::Section => {
                if chain.len() > 1 && !is_movement(claim) {
                    out.push((std::mem::take(&mut chain), claim));
                }
                chain.clear();
            }
            ClaimKind::Document => {}
            // Structure claims are the bytes *between* things — the cut
            // newline inside a chain, the blank run before the heading —
            // and neither open nor close a chain of pieces.
            ClaimKind::Structure => {}
        }
    }
    out
}

/// The last scalar boundary at or before an offset.
fn floor_boundary(text: &str, mut i: usize) -> usize {
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// The model of a document under a profile.
fn model<'a>(text: &'a str, profile: &Profile) -> Document<'a> {
    analyze(
        &SourceDocument {
            id: GUIDE_ID,
            bytes: text.as_bytes(),
        },
        profile,
    )
    .expect("analyzes")
}

#[test]
fn the_same_bytes_and_profile_slice_to_byte_identical_claims() {
    let a = slice(GUIDE, &v1());
    let b = slice(GUIDE, &v1());
    assert_eq!(a, b);
    assert_eq!(a[0].kind, ClaimKind::Document);
    assert_eq!(a[0].subject, GUIDE_ID);
}

#[test]
fn every_heading_and_movement_starts_a_section_with_its_level_ordinal_parent_and_span() {
    let claims = slice(GUIDE, &v1());
    let sections = sections(&claims);
    assert_eq!(
        sections.len(),
        8,
        "title, two movements, field notes, tide tables, concordance, and the concordance's two subsections"
    );
    let title = sections[0];
    assert_eq!(
        object(title, purrdf_markdown::RDF_TYPE).as_deref(),
        Some(&*format!("<{}>", v().section_class))
    );
    assert_eq!(integer(title, &v().level), Some(1));
    assert_eq!(integer(title, &v().ordinal), Some(0));
    assert_eq!(object(title, &v().parent), None);
    assert_eq!(title.span, Some((0, GUIDE.len() as u64)));
    assert_eq!(
        object(title, &v().heading).as_deref(),
        Some(&*format!("\"{TITLE}\"")),
        "the heading's words are the section's one plain literal"
    );
    // Two movement markers under one heading are siblings, not nested.
    for (i, movement) in sections[1..3].iter().enumerate() {
        assert_eq!(
            object(movement, purrdf_markdown::RDF_TYPE).as_deref(),
            Some(&*format!("<{}>", v().movement_class))
        );
        assert_eq!(integer(movement, &v().level), Some(2));
        assert_eq!(integer(movement, &v().ordinal), Some(i as u64 + 1));
        assert_eq!(
            object(movement, &v().parent).as_deref(),
            Some(&*format!("<{}>", title.subject))
        );
        let (start, end) = movement.span.expect("span");
        assert!(
            start == 0 || GUIDE.as_bytes()[start as usize - 1] == b'\n',
            "a section's span opens at its line's first byte, leading run and all"
        );
        assert!(
            GUIDE[start as usize..]
                .trim_start_matches(' ')
                .starts_with('\u{2042}'),
            "and the marker is read behind the leading run the law admits"
        );
        let next = sections[i + 2].span.expect("span").0;
        assert_eq!(end, next, "a movement ends where the next section begins");
    }
    let notes = sections[3];
    assert_eq!(
        object(notes, purrdf_markdown::RDF_TYPE).as_deref(),
        Some(&*format!("<{}>", v().section_class))
    );
    assert_eq!(integer(notes, &v().level), Some(2));
    assert_eq!(
        object(notes, &v().parent).as_deref(),
        Some(&*format!("<{}>", title.subject))
    );
    let tables = sections[4];
    assert_eq!(integer(tables, &v().level), Some(3));
    assert_eq!(integer(tables, &v().ordinal), Some(4));
    assert_eq!(
        object(tables, &v().parent).as_deref(),
        Some(&*format!("<{}>", notes.subject)),
        "a deeper heading nests under the one in force"
    );
    assert_eq!(
        notes.span.map(|s| s.1),
        sections[5].span.map(|s| s.0),
        "a heading ends where the next heading at its level begins"
    );
    let concordance = sections[5];
    assert_eq!(integer(concordance, &v().level), Some(2));
    assert_eq!(
        object(concordance, &v().heading).as_deref(),
        Some("\"Concordance\"")
    );
    assert_eq!(concordance.span.map(|s| s.1), Some(GUIDE.len() as u64));
    // The concordance's own subsections are deeper, so they nest under
    // the concordance rather than closing it — and the concordance's
    // span still holds every row of both.
    let beds = sections[6];
    let unnamed = sections[7];
    for subsection in [beds, unnamed] {
        assert_eq!(integer(subsection, &v().level), Some(3));
        assert_eq!(
            object(subsection, &v().parent).as_deref(),
            Some(&*format!("<{}>", concordance.subject))
        );
    }
    assert_eq!(
        beds.span.map(|s| s.1),
        unnamed.span.map(|s| s.0),
        "a subsection ends where its sibling at the same level begins"
    );
    assert_eq!(unnamed.span.map(|s| s.1), Some(GUIDE.len() as u64));
}

#[test]
fn every_numbered_verse_is_one_unit_carrying_its_number() {
    let claims = slice(GUIDE, &v1());
    let numbers: Vec<u64> = units(&claims)
        .iter()
        .filter_map(|u| integer(u, &v().verse))
        .collect();
    assert_eq!(numbers, (1..=8).collect::<Vec<_>>());
    let first = units(&claims)
        .into_iter()
        .find(|u| integer(u, &v().verse) == Some(1))
        .expect("verse 1");
    let text = unescape(&object(first, &v().text).expect("text"));
    assert!(text.starts_with("1. The outer reefs lie a half day north"));
    assert!(text.ends_with("the water draws off the shelf."));
}

#[test]
fn the_lineage_is_the_heading_stack_in_force_at_the_unit() {
    let claims = slice(GUIDE, &v1());
    let all = units(&claims);
    let headnote = all[0];
    assert_eq!(
        object(headnote, &v().lineage).as_deref(),
        Some(&*format!("\"{TITLE}\"^^<{}>", v().dt_lineage))
    );
    let verse_1 = all
        .iter()
        .find(|u| integer(u, &v().verse) == Some(1))
        .expect("verse 1");
    assert_eq!(
        object(verse_1, &v().lineage).as_deref(),
        Some(&*format!(
            "\"{TITLE} > the outer reefs\"^^<{}>",
            v().dt_lineage
        ))
    );
    assert_eq!(
        object(verse_1, &v().in_section).as_deref(),
        Some(&*format!("<{}>", sections(&claims)[1].subject))
    );
    let verse_6 = all
        .iter()
        .find(|u| integer(u, &v().verse) == Some(6))
        .expect("verse 6");
    assert_eq!(
        object(verse_6, &v().lineage).as_deref(),
        Some(&*format!(
            "\"{TITLE} > Field notes > Tide tables\"^^<{}>",
            v().dt_lineage
        ))
    );
    assert_eq!(
        object(verse_6, &v().in_section).as_deref(),
        Some(&*format!("<{}>", sections(&claims)[4].subject)),
        "the innermost heading in force, not its parent"
    );
}

#[test]
fn every_unit_literal_is_the_verbatim_span_and_the_only_plain_literal() {
    for profile in [&v1(), &small()] {
        let claims = slice(GUIDE, profile);
        for unit in units(&claims) {
            let (start, end) = unit.span.expect("span");
            let text = unescape(&object(unit, &v().text).expect("text"));
            assert_eq!(text, &GUIDE[start as usize..end as usize]);
            let plain = pairs(unit)
                .iter()
                .filter(|(_, o)| o.starts_with('"') && !o.contains("\"^^<"))
                .count();
            assert_eq!(plain, 1, "{}", unit.subject);
            assert_eq!(integer(unit, &v().byte_start), Some(start));
            assert_eq!(integer(unit, &v().byte_end), Some(end));
            assert_eq!(
                unit.subject,
                unit_iri(
                    &v(),
                    GUIDE_ID,
                    &profile.chunking_id(),
                    start,
                    end,
                    text.as_bytes()
                )
            );
        }
    }
}

#[test]
fn an_oversize_unit_splits_at_the_bound_on_a_boundary_and_continues_from_the_snapped_overlap() {
    let claims = slice(GUIDE, &small());
    let all = units(&claims);
    let by_subject: BTreeMap<&str, &Claim> = all.iter().map(|u| (u.subject.as_str(), *u)).collect();
    let mut continued = 0;
    let mut bound_inside_a_scalar = 0;
    for unit in &all {
        let (start, end) = unit.span.expect("span");
        let (start, end) = (start as usize, end as usize);
        assert!(GUIDE.is_char_boundary(start) && GUIDE.is_char_boundary(end));
        assert!(
            end - start <= small().max_bytes,
            "no piece is over the bound, line or no line: {}",
            unit.subject
        );
        let Some(prev) = object(unit, &v().continues) else {
            continue;
        };
        continued += 1;
        let prev = by_subject[prev.trim_start_matches('<').trim_end_matches('>')];
        let (p_start, p_end) = prev.span.expect("span");
        assert!(
            start > p_start as usize && start < p_end as usize,
            "overlaps the previous piece"
        );
        // The bound is a byte count, never a character: a four-byte
        // scalar sitting across it changes nothing about the cut.
        if !GUIDE.is_char_boundary(p_start as usize + small().max_bytes) {
            bound_inside_a_scalar += 1;
        }
        let candidate = p_end as usize - small().overlap;
        assert!(start <= candidate, "reaches back at least the overlap");
        assert_eq!(
            GUIDE.as_bytes()[start - 1],
            b'\n',
            "snapped to a line start"
        );
        assert!(
            !GUIDE.as_bytes()[start..candidate].contains(&b'\n'),
            "snapped to the nearest line start at or before the candidate"
        );
        assert_eq!(
            object(unit, &v().in_section),
            object(prev, &v().in_section),
            "never crosses a heading"
        );
        assert_eq!(object(unit, &v().verse), object(prev, &v().verse));
    }
    assert!(
        continued > 0,
        "the fixture has oversize units under the small bound"
    );
    assert!(
        bound_inside_a_scalar > 0,
        "the fixture puts a four-byte scalar across a cut bound"
    );

    // A unit that runs in pieces right up to a heading: each piece
    // carries the section and the verse of the piece before it, and the
    // heading opens the next section at the ordinal it would have held
    // had the unit never split.
    let mut before_a_heading = Vec::new();
    for (chain, heading) in split_chains_before_a_heading(&claims) {
        for piece in &chain {
            assert_eq!(
                object(piece, &v().in_section),
                object(chain[0], &v().in_section),
                "every piece sits in the section the unit opened in"
            );
            assert_eq!(object(piece, &v().verse), object(chain[0], &v().verse));
            assert_ne!(
                object(piece, &v().in_section).as_deref(),
                Some(&*format!("<{}>", heading.subject)),
                "no piece falls forward into the section the heading opens"
            );
        }
        before_a_heading.push((
            plain_value(heading, &v().heading).expect("a heading"),
            integer(heading, &v().ordinal).expect("an ordinal"),
            integer(heading, &v().level).expect("a level"),
            chain.len(),
        ));
    }
    assert_eq!(
        before_a_heading,
        vec![
            ("Field notes".to_owned(), 3_u64, 2_u64, 2_usize),
            // Four pieces, not three: the paragraph before this heading
            // carries the sheet's own four-space-indented margin line,
            // which is content rather than a heading and lengthens the
            // unit the split then cuts.
            ("Tide tables".to_owned(), 4, 3, 4),
            ("Concordance".to_owned(), 5, 2, 5),
            // The prose of the beds subsection runs just past the small
            // bound, so it is two pieces where the heading of the next
            // subsection closes it.
            ("Heads the chart leaves unnamed".to_owned(), 7, 3, 2),
        ],
        "a split leaves the sections of the document at the ordinals and levels they already held"
    );

    let whole = slice(GUIDE, &v1());
    assert!(
        units(&whole)
            .iter()
            .all(|u| object(u, &v().continues).is_none())
    );
}

#[test]
fn a_stretch_with_no_newline_cuts_at_a_scalar_boundary_and_never_inside_one() {
    let bytes = GUIDE.as_bytes();
    let mut cut_mid_line = 0;
    let mut pulled_back_off_a_four_byte_scalar = 0;
    for max_bytes in 48..=64 {
        let mut tight = v1();
        tight.max_bytes = max_bytes;
        tight.overlap = 0;
        let claims = slice(GUIDE, &tight);
        let all = units(&claims);
        let by_subject: BTreeMap<&str, &Claim> =
            all.iter().map(|u| (u.subject.as_str(), *u)).collect();
        for unit in &all {
            let (start, end) = unit.span.expect("span");
            let (start, end) = (start as usize, end as usize);
            assert!(GUIDE.is_char_boundary(start) && GUIDE.is_char_boundary(end));
            assert_eq!(
                unescape(&object(unit, &v().text).expect("text")),
                &GUIDE[start..end]
            );
            assert!(end - start <= max_bytes, "no piece is over the bound");
            let Some(prev) = object(unit, &v().continues) else {
                continue;
            };
            let prev = by_subject[prev.trim_start_matches('<').trim_end_matches('>')];
            let (p_start, p_end) = prev.span.expect("span");
            let (p_start, p_end) = (p_start as usize, p_end as usize);
            assert_eq!(
                start,
                if bytes[p_end] == b'\n' {
                    p_end + 1
                } else {
                    p_end
                },
                "with no overlap a piece resumes at the cut"
            );
            if bytes[start - 1] == b'\n' {
                continue;
            }
            cut_mid_line += 1;
            let bound = p_start + max_bytes;
            assert!(p_end <= bound, "the cut is at or before the bound");
            if !GUIDE.is_char_boundary(bound) {
                let scalar = GUIDE[p_end..].chars().next().expect("a scalar at the cut");
                assert!(
                    p_end + scalar.len_utf8() > bound,
                    "the cut is the boundary the bound fell inside"
                );
                if scalar.len_utf8() == 4 {
                    pulled_back_off_a_four_byte_scalar += 1;
                }
            }
        }
    }
    assert!(
        cut_mid_line > 0,
        "the fixture has lines longer than the tight bound"
    );
    assert!(
        pulled_back_off_a_four_byte_scalar > 0,
        "a four-byte scalar straddles a cut bound and the cut falls back off it"
    );
}

#[test]
fn multi_byte_text_keeps_its_boundaries_and_its_characters() {
    let text = "# T\u{ed}tulo\n\n\u{2042} *el camino*\n\n1. \u{201c}Quoted\u{201d} \u{2013} \u{4e2d} \u{1f41a} and \u{2042} inside.\n\n2. Tail\n";
    let claims = slice(text, &v1());
    let s = sections(&claims);
    assert_eq!(
        object(s[0], &v().heading).as_deref(),
        Some("\"T\u{ed}tulo\"")
    );
    assert_eq!(object(s[1], &v().heading).as_deref(), Some("\"el camino\""));
    let u = units(&claims);
    assert_eq!(
        unescape(&object(u[0], &v().text).expect("text")),
        "1. \u{201c}Quoted\u{201d} \u{2013} \u{4e2d} \u{1f41a} and \u{2042} inside."
    );
    let mut tiny = v1();
    tiny.max_bytes = 5;
    tiny.overlap = 0;
    for unit in units(&slice(text, &tiny)) {
        let (start, end) = unit.span.expect("span");
        assert!(text.is_char_boundary(start as usize) && text.is_char_boundary(end as usize));
        assert_eq!(
            unescape(&object(unit, &v().text).expect("text")),
            &text[start as usize..end as usize]
        );
    }
}

#[test]
fn a_concordance_table_lifts_into_citations_with_and_without_a_canon_base() {
    let claims = slice(GUIDE, &v1());
    let all = units(&claims);
    let verse = |n: u64| {
        *all.iter()
            .find(|u| integer(u, &v().verse) == Some(n))
            .expect("verse")
    };
    let anchor = |name: &str| format!("\"{name}\"^^<{}>", v().dt_anchor);
    let path = |name: &str| format!("\"{name}\"^^<{}>", v().dt_path);
    // Row `1–3` names two anchors and one source for each verse it covers.
    for n in [1, 2] {
        assert_eq!(
            objects(verse(n), &v().cites),
            vec![anchor("reef-shelf"), anchor("tide-line")]
        );
        assert!(
            object(verse(n), &v().canon_source).is_none(),
            "a source path annotates the row's citation node, never the unit"
        );
    }
    let one = citation_nodes(verse(1));
    assert_eq!(one.len(), 1, "one row lifted onto verse 1");
    assert_eq!(
        citation_objects(verse(1), &one[0], &v().canon_source),
        vec![path("atlas/outer-reefs.logic.ttl")]
    );
    // Verse 2 is covered by `1–3` and by the row that names a sheet and
    // no anchor: two rows, so two nodes, and only one of them reifies.
    let two = citation_nodes(verse(2));
    assert_eq!(two.len(), 2, "two rows lifted onto verse 2");
    let mut paths_of_two: Vec<String> = two
        .iter()
        .flat_map(|node| citation_objects(verse(2), node, &v().canon_source))
        .collect();
    paths_of_two.sort();
    assert_eq!(
        paths_of_two,
        vec![
            path("atlas/outer-reefs.logic.ttl"),
            path("atlas/unnamed-heads.logic.ttl")
        ],
        "the anchor-less row's source is in the graph beside the anchored row's"
    );
    // Verse 3 is covered by `1–3` and by `3–5`: it carries both rows.
    assert_eq!(
        objects(verse(3), &v().cites),
        vec![
            anchor("lagoon-floor"),
            anchor("reef-shelf"),
            anchor("salt-pan"),
            anchor("tide-line")
        ],
        "prose beside a backticked anchor lifts nothing"
    );
    assert_eq!(
        citation_nodes(verse(3)).len(),
        2,
        "two rows cover it, so it carries two citation nodes"
    );
    let mut paths_of_three: Vec<String> = citation_triples(verse(3))
        .iter()
        .filter(|(_, p, _)| *p == v().canon_source)
        .map(|(_, _, o)| o.clone())
        .collect();
    paths_of_three.sort();
    assert_eq!(
        paths_of_three,
        vec![
            path("atlas/inner-lagoon.logic.ttl"),
            path("atlas/outer-reefs.logic.ttl"),
            path("atlas/outer-reefs.logic.ttl")
        ],
        "each row keeps its own sources, so outer-reefs is stated once per row that named it"
    );
    // Row `7–9` runs past the last verse: the verses that exist lift, the
    // one that does not lifts nothing.
    for n in [7, 8] {
        assert_eq!(
            objects(verse(n), &v().cites),
            vec![anchor("neap-tide"), anchor("spring-tide")]
        );
    }
    assert!(
        all.iter().all(|u| integer(u, &v().verse) != Some(9)),
        "a row for an absent verse lifts nothing"
    );
    let prose = all.iter().find(|u| {
        unescape(&object(u, &v().text).expect("text"))
            == "Every anchor named below is a node of the atlas, not of this guide."
    });
    assert!(
        prose.is_some(),
        "the concordance prose is a paragraph unit; the table rows are not"
    );
    assert!(
        all.iter()
            .all(|u| !unescape(&object(u, &v().text).expect("text")).contains("| Verses |")),
        "no table row is a unit"
    );

    let mut based = v1();
    based.canon_base = Some(CANON_BASE.to_owned());
    let claims = slice(GUIDE, &based);
    let one = *units(&claims)
        .iter()
        .find(|u| integer(u, &v().verse) == Some(1))
        .expect("verse 1");
    assert_eq!(
        objects(one, &v().cites),
        vec![
            format!("<{CANON_BASE}reef-shelf>"),
            format!("<{CANON_BASE}tide-line>")
        ]
    );
}

/// The lexical form and the datatype IRI of a rendered typed literal.
fn typed_parts(term: &str) -> (String, String) {
    let (lexical, datatype) = term.split_once("\"^^<").expect("a typed literal");
    (
        unescape(&format!("{lexical}\"")),
        datatype.trim_end_matches('>').to_owned(),
    )
}

/// The `(subject, predicate, object)` a triple term states, read the
/// way an N-Triples consumer reads the line it sits in.
fn reified(term: &str) -> (String, String, String) {
    let inner = term
        .strip_prefix("<<( ")
        .and_then(|t| t.strip_suffix(" )>>"))
        .unwrap_or_else(|| panic!("an RDF 1.2 triple term: {term}"));
    let (subject, rest) = inner.split_once("> ").expect("a subject");
    let (predicate, object) = rest.split_once("> ").expect("a predicate");
    (
        subject.trim_start_matches('<').to_owned(),
        predicate.trim_start_matches('<').to_owned(),
        object.to_owned(),
    )
}

#[test]
fn every_line_of_a_claim_is_its_own_node_or_a_citation_of_it() {
    // A claim is read by two helpers — `pairs` for the node's own lines
    // and `citation_triples` for the rest — and this is what keeps a
    // line from falling between them and being asserted by neither.
    for profile in [&v1(), &small()] {
        for claim in slice(GUIDE, profile) {
            assert_eq!(
                pairs(&claim).len() + citation_triples(&claim).len(),
                claim.turtle.lines().count(),
                "{}",
                claim.subject
            );
            for (subject, _, _) in citation_triples(&claim) {
                assert_eq!(
                    claim.kind,
                    ClaimKind::Unit,
                    "only a unit carries a node that is not its own"
                );
                assert!(
                    subject.starts_with(&format!("{SLICE_BASE}citation:sha256:")),
                    "and that node is a citation of it: {subject}"
                );
            }
        }
    }
}

#[test]
fn a_reification_is_spelled_the_way_purrdf_core_spells_one() {
    // Not "resembles": the very line, from the kernel's own reifier
    // writer. It pins both halves of the spelling at once — the
    // `rdf:reifies` IRI, and the RDF 1.2 triple term `<<( s p o )>>`,
    // whose parentheses are what make it non-asserting.
    let claims = slice(GUIDE, &v1());
    let three = *units(&claims)
        .iter()
        .find(|u| integer(u, &v().verse) == Some(3))
        .expect("verse 3");
    let reifications: Vec<(String, String, String)> = citation_triples(three)
        .into_iter()
        .filter(|(_, p, _)| p == purrdf_markdown::RDF_REIFIES)
        .collect();
    assert_eq!(reifications.len(), 4, "two rows, two anchors each");
    for (citation, _, term) in reifications {
        let (_, _, anchor) = reified(&term);
        let (lexical, datatype) = typed_parts(&anchor);
        assert_eq!(datatype, v().dt_anchor);
        let expected = purrdf_core::emit_reifier(
            &purrdf_core::RdfReifier {
                reifier: purrdf_core::RdfTerm::Iri(citation),
                statement: purrdf_core::RdfTriple::new(
                    purrdf_core::RdfTerm::Iri(three.subject.clone()),
                    v().cites,
                    purrdf_core::RdfTerm::Literal(purrdf_core::RdfLiteral {
                        lexical_form: lexical,
                        datatype: Some(datatype),
                        language: None,
                        direction: None,
                    }),
                ),
                graph: None,
                location: None,
            },
            &[],
        );
        assert!(
            claims
                .iter()
                .any(|c| c.turtle.contains(expected.trim_end_matches('\n'))),
            "the kernel's own line: {expected}"
        );
        assert!(expected.contains(" <<( "), "the parenthesized triple term");
        assert!(expected.contains(" )>> ."), "and its close");
    }
}

#[test]
fn a_verse_two_rows_cover_keeps_each_rows_sources_paired_with_that_rows_anchors() {
    let claims = slice(GUIDE, &v1());
    let three = *units(&claims)
        .iter()
        .find(|u| integer(u, &v().verse) == Some(3))
        .expect("verse 3");

    // Exactly what an N-Triples consumer reconstructs: group the lines
    // by their citation node, read each reified triple term for the
    // anchor it states, and collect the paths that annotate the node.
    let mut rows: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    for (subject, predicate, object) in citation_triples(three) {
        let row = rows.entry(subject).or_default();
        if predicate == purrdf_markdown::RDF_REIFIES {
            let (unit, cites, anchor) = reified(&object);
            assert_eq!(
                unit, three.subject,
                "the reified subject is the unit itself"
            );
            assert_eq!(cites, v().cites);
            assert!(row.0.insert(anchor), "one reification per (row, anchor)");
        } else if predicate == purrdf_markdown::RDF_TYPE {
            assert_eq!(object, format!("<{}>", v().citation_class));
        } else if predicate == v().in_unit {
            assert_eq!(
                object,
                format!("<{}>", three.subject),
                "the node names the unit it is an edge of"
            );
        } else {
            assert_eq!(predicate, v().canon_source);
            assert!(row.1.insert(object), "one path per (row, source)");
        }
    }
    let anchor = |name: &str| format!("\"{name}\"^^<{}>", v().dt_anchor);
    let path = |name: &str| format!("\"{name}\"^^<{}>", v().dt_path);
    let reconstructed: BTreeSet<(Vec<String>, Vec<String>)> = rows
        .values()
        .map(|(anchors, paths)| {
            (
                anchors.iter().cloned().collect(),
                paths.iter().cloned().collect(),
            )
        })
        .collect();
    assert_eq!(
        reconstructed,
        BTreeSet::from([
            (
                vec![anchor("reef-shelf"), anchor("tide-line")],
                vec![path("atlas/outer-reefs.logic.ttl")],
            ),
            (
                vec![anchor("lagoon-floor"), anchor("salt-pan")],
                vec![
                    path("atlas/inner-lagoon.logic.ttl"),
                    path("atlas/outer-reefs.logic.ttl"),
                ],
            ),
        ]),
        "row 1\u{2013}3's anchors keep row 1\u{2013}3's source, and row 3\u{2013}5's keep both of its own"
    );

    // And this is what the flat form could not say: on the unit itself
    // the four anchors lie in one heap, with nothing to tell a reader
    // that `lagoon-floor` came in beside `atlas/inner-lagoon.logic.ttl`
    // and `reef-shelf` did not.
    assert_eq!(objects(three, &v().cites).len(), 4);
    assert!(object(three, &v().canon_source).is_none());
    parses_whole(&claims);
}

#[test]
fn a_citation_node_is_content_addressed_over_the_row_and_the_unit_it_lifted_onto() {
    let document = model(GUIDE, &v1());
    let claims = slice(GUIDE, &v1());
    let contract = v1().chunking_id();
    let line_of = |row: &purrdf_markdown::Citation| {
        let span = row.span();
        &GUIDE.as_bytes()[span.start as usize..span.end as usize]
    };
    let unit_claim = |verse: u64| {
        *units(&claims)
            .iter()
            .find(|u| integer(u, &v().verse) == Some(verse))
            .expect("the verse")
    };

    let mut seen = BTreeSet::new();
    for row in document.citations() {
        let span = row.span();
        for &(verse, unit) in row.lifted() {
            assert_eq!(document.unit(unit).expect("in range").verse(), Some(verse));
            let claim = unit_claim(verse);
            let minted = purrdf_markdown::citation_iri(
                &v(),
                GUIDE_ID,
                &contract,
                span.start,
                span.end,
                line_of(row),
                &claim.subject,
                &reified_terms(row, &v1(), &claim.subject),
            );
            assert!(
                citation_nodes(claim).contains(&minted),
                "a consumer holding the source re-derives the node: {minted}"
            );
            assert!(seen.insert(minted), "one node per (row, unit)");
        }
    }
    assert_eq!(
        seen.len(),
        12,
        "3 + 3 + 2 lifts, then 2 + 1 in the second table and 1 in the third"
    );

    // The pair is what is addressed, and both halves of it are load
    // bearing: one row over two verses is two nodes, and two rows over
    // one verse are two more.
    let (first, second) = (&document.citations()[0], &document.citations()[1]);
    let mint = |row: &purrdf_markdown::Citation, verse: u64| {
        let span = row.span();
        purrdf_markdown::citation_iri(
            &v(),
            GUIDE_ID,
            &contract,
            span.start,
            span.end,
            line_of(row),
            &unit_claim(verse).subject,
            &reified_terms(row, &v1(), &unit_claim(verse).subject),
        )
    };
    assert_ne!(mint(first, 1), mint(first, 2), "one row, two verses");
    assert_ne!(mint(first, 3), mint(second, 3), "two rows, one verse");
}

// --- a row that names sources and no anchors ------------------------------

/// The sheet the guide's anchor-less row names: an atlas file for the
/// heads the chart leaves unnamed, with nothing inside it yet to point
/// at.
const UNNAMED_SHEET: &str = "atlas/unnamed-heads.logic.ttl";

/// A whole small document whose only concordance row names a canon
/// source and no anchor at all. It is the shape whose citation node
/// reifies nothing, and therefore the shape that has nothing but §11.1's
/// class and back-edge holding it into the graph.
const ANCHORLESS: &str = "# T\n\n1. One.\n\n## Concordance\n\n\
                          | Verses | Canon source | Anchors |\n| --- | --- | --- |\n\
                          | 1 | `atlas/a.ttl` | |\n";

/// The `(predicate, object)` lines one citation node of a claim states,
/// in the claim's own bytewise order.
fn citation_lines(claim: &Claim, citation: &str) -> Vec<(String, String)> {
    citation_triples(claim)
        .into_iter()
        .filter(|(subject, _, _)| subject == citation)
        .map(|(_, predicate, object)| (predicate, object))
        .collect()
}

#[test]
fn a_row_with_sources_and_no_anchors_states_a_typed_citation_node_bound_to_its_unit() {
    // The model reports the row: its range, its source, and no anchors
    // at all.
    let document = model(GUIDE, &v1());
    let row = document
        .citations()
        .iter()
        .find(|row| row.sources() == [UNNAMED_SHEET])
        .expect("the guide names a sheet no anchor has been cut in yet");
    assert_eq!(row.verses(), (2, 2));
    assert!(row.anchors().is_empty(), "its anchor cell names nothing");
    assert_eq!(
        row.lifted().len(),
        1,
        "and it lifts onto verse 2 all the same"
    );
    assert!(!row.is_unmatched());

    // And so does the graph. The node the lift mints reifies nothing,
    // and it still states what it is and what it belongs to — which is
    // what keeps the sheet the row names reachable from the verse
    // instead of stranded beside it.
    let claims = slice(GUIDE, &v1());
    let two = *units(&claims)
        .iter()
        .find(|u| integer(u, &v().verse) == Some(2))
        .expect("verse 2");
    let span = row.span();
    let minted = purrdf_markdown::citation_iri(
        &v(),
        GUIDE_ID,
        &v1().chunking_id(),
        span.start,
        span.end,
        &GUIDE.as_bytes()[span.start as usize..span.end as usize],
        &two.subject,
        &reified_terms(row, &v1(), &two.subject),
    );
    let stated = citation_lines(two, &minted);
    assert_eq!(
        stated,
        vec![
            (
                purrdf_markdown::RDF_TYPE.to_owned(),
                format!("<{}>", v().citation_class)
            ),
            (
                v().canon_source,
                format!("\"{UNNAMED_SHEET}\"^^<{}>", v().dt_path)
            ),
            (v().in_unit, format!("<{}>", two.subject)),
        ],
        "the whole node: its class, its row's source, and the unit it is an edge of"
    );
    assert!(
        !stated
            .iter()
            .any(|(predicate, _)| predicate == purrdf_markdown::RDF_REIFIES),
        "there is no anchor to reify, which is the whole of the case"
    );
    // The row lifted no anchor, so the verse cites what the other row
    // covering it named, and nothing more.
    assert_eq!(
        objects(two, &v().cites),
        vec![
            format!("\"reef-shelf\"^^<{}>", v().dt_anchor),
            format!("\"tide-line\"^^<{}>", v().dt_anchor)
        ]
    );
    parses_whole(&claims);

    // The same shape alone in a document, where nothing else covers the
    // verse: three lines, and the node is still named by its unit.
    let claims = slice_of(ANCHORLESS, GUIDE_ID, &v1()).expect("slices");
    let one = units(&claims)[0];
    let nodes = citation_nodes(one);
    assert_eq!(nodes.len(), 1);
    assert_eq!(
        citation_lines(one, &nodes[0]),
        vec![
            (
                purrdf_markdown::RDF_TYPE.to_owned(),
                format!("<{}>", v().citation_class)
            ),
            (
                v().canon_source,
                format!("\"atlas/a.ttl\"^^<{}>", v().dt_path)
            ),
            (v().in_unit, format!("<{}>", one.subject)),
        ]
    );
    assert!(
        objects(one, &v().cites).is_empty(),
        "no anchor, so no asserted edge on the unit"
    );
    parses_whole(&claims);
}

#[test]
fn a_row_with_both_sources_and_anchors_lifts_exactly_what_it_always_lifted() {
    // The neighbouring case the law must keep admitting. A row that
    // names both states every line it always stated — the asserted edge
    // on the unit, one reification per anchor, one path per source —
    // with the class and the back-edge added beside them and nothing
    // taken away.
    let claims = slice(GUIDE, &v1());
    let one = *units(&claims)
        .iter()
        .find(|u| integer(u, &v().verse) == Some(1))
        .expect("verse 1");
    let nodes = citation_nodes(one);
    assert_eq!(
        nodes.len(),
        1,
        "row 1\u{2013}3 is the only row covering verse 1"
    );
    let reifies = |anchor: &str| {
        format!(
            "<<( <{}> <{}> \"{anchor}\"^^<{}> )>>",
            one.subject,
            v().cites,
            v().dt_anchor
        )
    };
    assert_eq!(
        citation_lines(one, &nodes[0]),
        vec![
            (
                purrdf_markdown::RDF_REIFIES.to_owned(),
                reifies("reef-shelf")
            ),
            (
                purrdf_markdown::RDF_REIFIES.to_owned(),
                reifies("tide-line")
            ),
            (
                purrdf_markdown::RDF_TYPE.to_owned(),
                format!("<{}>", v().citation_class)
            ),
            (
                v().canon_source,
                format!("\"atlas/outer-reefs.logic.ttl\"^^<{}>", v().dt_path)
            ),
            (v().in_unit, format!("<{}>", one.subject)),
        ]
    );
    assert_eq!(
        objects(one, &v().cites),
        vec![
            format!("\"reef-shelf\"^^<{}>", v().dt_anchor),
            format!("\"tide-line\"^^<{}>", v().dt_anchor)
        ],
        "the asserted edges are the row's anchors, unchanged"
    );
    parses_whole(&claims);
}

/// One document shape, one verse, one row, and only the anchor cell
/// differing: the three cardinalities §11.1 clause 2 has to answer for.
fn one_row_naming(anchors: &str) -> String {
    format!(
        "# T\n\n1. One.\n\n## Concordance\n\n\
         | Verses | Canon source | Anchors |\n| --- | --- | --- |\n\
         | 1 | `atlas/a.ttl` |{anchors}|\n"
    )
}

/// The **cardinality** law, executed. A citation node is a reifier of one
/// *lifting* and not of one triple, so a row lifting onto a unit mints
/// **exactly one** node whatever that lifting reified — two edges, one,
/// or none — and §5's preimage claim is what keeps liftings that reified
/// different sets apart.
///
/// This is the vector for a sentence the specification could otherwise
/// state two ways. Read as one node per reified triple, the first case
/// below would be two nodes and the last would be **none at all** — and
/// the last case is a row whose canon sources would then hang off no
/// node any unit reaches, which is exactly the orphan §11.1 closes by
/// name. The count and the reachability are one law, so they are asked
/// here together.
#[test]
fn a_rows_lifting_is_one_citation_node_carrying_every_edge_that_lifting_reified() {
    for (anchors, expected) in [
        (" `a-one`, `a-two` ", &["a-one", "a-two"][..]),
        (" `a-one` ", &["a-one"][..]),
        (" ", &[][..]),
    ] {
        let text = one_row_naming(anchors);
        let claims = slice_of(&text, GUIDE_ID, &v1()).expect("slices");
        let unit = units(&claims)[0];
        let nodes = citation_nodes(unit);
        assert_eq!(
            nodes.len(),
            1,
            "one row lifting onto one unit is one node, whatever it reified: {anchors:?}"
        );
        let lines = citation_lines(unit, &nodes[0]);
        let reified: Vec<String> = lines
            .iter()
            .filter(|(predicate, _)| predicate == purrdf_markdown::RDF_REIFIES)
            .map(|(_, object)| object.clone())
            .collect();
        let want: Vec<String> = expected
            .iter()
            .map(|anchor| {
                format!(
                    "<<( <{}> <{}> \"{anchor}\"^^<{}> )>>",
                    unit.subject,
                    v().cites,
                    v().dt_anchor
                )
            })
            .collect();
        assert_eq!(
            reified, want,
            "one `rdf:reifies` object per anchor, and all of them on the one node"
        );
        // Typed and back-edged unconditionally, the empty case included:
        // that is what makes a lifting of nothing a node and not an
        // orphaned source path.
        assert!(
            lines.contains(&(
                purrdf_markdown::RDF_TYPE.to_owned(),
                format!("<{}>", v().citation_class)
            )),
            "the class holds whatever the row lifted: {anchors:?}"
        );
        assert!(
            lines.contains(&(v().in_unit, format!("<{}>", unit.subject))),
            "and so does the back-edge to its unit: {anchors:?}"
        );
        parses_whole(&claims);
    }

    // And the reified **set** is inside the node's identity. The row's
    // bytes, its span and the unit are held fixed here, so the only
    // thing that can move the IRI is the list of terms: the whole list
    // mints the node the graph states, and that list one term short
    // mints one the graph never states. Were the terms outside the
    // preimage those two would carry the same IRI, and merging a store
    // whose node reified both with a store whose node reified one would
    // leave a single node asserting the union — an edge no document ever
    // stated of it.
    let text = one_row_naming(" `a-one`, `a-two` ");
    let claims = slice_of(&text, GUIDE_ID, &v1()).expect("slices");
    let unit = units(&claims)[0];
    let document = model(&text, &v1());
    let row = &document.citations()[0];
    let span = row.span();
    let terms = reified_terms(row, &v1(), &unit.subject);
    assert_eq!(terms.len(), 2, "the row named two anchors");
    let mint = |reified: &[String]| {
        purrdf_markdown::citation_iri(
            &v(),
            GUIDE_ID,
            &v1().chunking_id(),
            span.start,
            span.end,
            &text.as_bytes()[span.start as usize..span.end as usize],
            &unit.subject,
            reified,
        )
    };
    assert_eq!(
        citation_nodes(unit),
        vec![mint(&terms)],
        "the node the graph states is minted over the whole reified set"
    );
    for short in [&terms[..1], &terms[1..], &[][..]] {
        assert_ne!(
            mint(short),
            mint(&terms),
            "a lifting that reified a different set is a different node"
        );
        assert!(
            !citation_nodes(unit).contains(&mint(short)),
            "and it is a node this graph never states"
        );
    }
}

/// Every `(subject, predicate, object)` of every claim of a slice, read
/// off the emitted lines the way a consumer reads them.
fn emitted_triples(claims: &[Claim]) -> Vec<(String, String, String)> {
    claims
        .iter()
        .flat_map(|claim| claim.turtle.lines())
        .map(|line| {
            let rest = line.strip_suffix(" .").expect("terminator");
            let (subject, rest) = rest.split_once("> ").expect("a subject, then the rest");
            let (predicate, object) = rest.split_once(' ').expect("predicate then object");
            (
                subject.trim_start_matches('<').to_owned(),
                predicate
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_owned(),
                object.to_owned(),
            )
        })
        .collect()
}

/// Every node an object position **names**: a plain IRI object, and the
/// IRIs inside a triple term, which an N-Triples consumer reads as the
/// triple they are.
///
/// A literal's datatype IRI is not one. It types a value rather than
/// pointing at a node, and walking through it would wire every unit of
/// a document to every other through the datatypes they share — which
/// would make a reachability walk say yes to anything.
fn object_iris(object: &str) -> Vec<String> {
    if object.starts_with("<<( ") {
        let (subject, predicate, inner) = reified(object);
        let mut out = vec![subject, predicate];
        out.extend(object_iris(&inner));
        return out;
    }
    if object.starts_with('<') {
        return vec![
            object
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_owned(),
        ];
    }
    Vec::new()
}

/// How many steps each node of the emitted graph lies from the nearest
/// **unit** node, walking the triples as edges in either direction.
///
/// A breadth-first walk, not a count: it starts at the units, and a node
/// it never reaches is a node a consumer that starts where the law says
/// to start — a unit — cannot get to at all.
fn steps_from_a_unit(claims: &[Claim]) -> BTreeMap<String, usize> {
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (subject, _, object) in emitted_triples(claims) {
        for named in object_iris(&object) {
            edges
                .entry(subject.clone())
                .or_default()
                .insert(named.clone());
            edges.entry(named).or_default().insert(subject.clone());
        }
    }
    let mut depth: BTreeMap<String, usize> = units(claims)
        .iter()
        .map(|unit| (unit.subject.clone(), 0))
        .collect();
    let mut frontier: Vec<String> = depth.keys().cloned().collect();
    let mut step = 0;
    while !frontier.is_empty() {
        step += 1;
        let mut next = Vec::new();
        for node in frontier {
            for neighbour in edges.get(&node).into_iter().flatten() {
                if !depth.contains_key(neighbour) {
                    depth.insert(neighbour.clone(), step);
                    next.push(neighbour.clone());
                }
            }
        }
        frontier = next;
    }
    depth
}

/// Every subject of an emitted line that is no claim's own node: the
/// citation nodes, found without asking what they are called.
fn nodes_no_claim_is_about(claims: &[Claim]) -> BTreeSet<String> {
    let own: BTreeSet<&str> = claims.iter().map(|c| c.subject.as_str()).collect();
    emitted_triples(claims)
        .into_iter()
        .map(|(subject, _, _)| subject)
        .filter(|subject| !own.contains(subject.as_str()))
        .collect()
}

/// The property that makes the orphan shape impossible to write again:
/// **every** citation node of an emitted graph is reachable from a unit,
/// and reachable in one step, whatever its row lifted.
///
/// It is a walk of the emitted triples rather than a count of them
/// because a count is exactly what missed this: a row naming sources and
/// no anchors emitted its `canonSource` line, so every line was present
/// and every total was right, and nothing in the graph named the subject
/// that line hung on.
#[test]
fn every_citation_node_of_a_slice_is_reachable_from_a_unit_by_a_walk_of_the_graph() {
    for profile in [&v1(), &small(), &under_canon(CANON_BASE)] {
        let claims = slice(GUIDE, profile);
        let document = model(GUIDE, profile);
        let lifts: usize = document
            .citations()
            .iter()
            .map(|row| row.lifted().len())
            .sum();
        let depth = steps_from_a_unit(&claims);
        let citations = nodes_no_claim_is_about(&claims);
        assert_eq!(
            citations.len(),
            lifts,
            "one node per (row, unit) the model states a lift for"
        );
        for citation in &citations {
            assert_eq!(
                depth.get(citation).copied(),
                Some(1),
                "a unit names it directly: {citation}"
            );
        }
        // And nothing else of the graph is stranded either: every node
        // the emitted lines name is reached from some unit.
        for (subject, _, object) in emitted_triples(&claims) {
            for named in std::iter::once(subject).chain(object_iris(&object)) {
                assert!(
                    depth.contains_key(&named),
                    "the walk reaches every node the graph names: {named}"
                );
            }
        }
    }

    // The shape the law is written for, alone in a document: the row
    // that names a source and no anchor is reachable exactly as an
    // anchored row is.
    let claims = slice_of(ANCHORLESS, GUIDE_ID, &v1()).expect("slices");
    let depth = steps_from_a_unit(&claims);
    let citations = nodes_no_claim_is_about(&claims);
    assert_eq!(citations.len(), 1);
    for citation in &citations {
        assert_eq!(depth.get(citation).copied(), Some(1));
    }
}

#[test]
fn a_units_scalar_offsets_and_content_digest_are_stated_as_data() {
    let text = "# T\u{ed}tulo\n\n\u{2042} *el camino*\n\n1. \u{201c}Quoted\u{201d} \u{2013} \u{4e2d} \u{1f41a} and \u{2042} inside.\n\n2. Tail\n";
    let claims = slice(text, &v1());
    let document = model(text, &v1());
    for (unit, claim) in document.units().iter().zip(units(&claims)) {
        let (start, end) = span_of(claim);
        // Counted straight off the text, not read back off the model.
        assert_eq!(
            integer(claim, &v().scalar_start),
            Some(text[..start].chars().count() as u64)
        );
        assert_eq!(
            integer(claim, &v().scalar_end),
            Some(text[..end].chars().count() as u64)
        );
        assert_eq!(
            typed_value(claim, &v().content_digest),
            Some(unit.digest().to_hex()),
            "the digest as data is the digest the model carries"
        );
        assert_eq!(
            unit.digest(),
            ContentDigest::of(&text.as_bytes()[start..end])
        );
        // Both new facts are typed, so the unit's text stays the one
        // plain literal a text index selects.
        for predicate in [&v().scalar_start, &v().scalar_end] {
            assert!(
                object(claim, predicate)
                    .expect("an offset")
                    .ends_with(&format!("^^<{}>", purrdf_markdown::XSD_INTEGER))
            );
        }
        assert!(
            object(claim, &v().content_digest)
                .expect("a digest")
                .ends_with(&format!("^^<{}>", purrdf_markdown::XSD_HEX_BINARY))
        );
        assert_eq!(
            pairs(claim)
                .iter()
                .filter(|(_, o)| o.starts_with('"') && !o.contains("\"^^<"))
                .count(),
            1,
            "{}",
            claim.subject
        );
    }
    // The first unit, counted by hand: `# Título` is 8 scalars and 9
    // bytes, the blank line and the marker line another 17 scalars and
    // 19 bytes; the unit is 31 scalars and 44 bytes.
    let first = units(&claims)[0];
    assert_eq!(integer(first, &v().byte_start), Some(28));
    assert_eq!(integer(first, &v().byte_end), Some(72));
    assert_eq!(integer(first, &v().scalar_start), Some(25));
    assert_eq!(integer(first, &v().scalar_end), Some(56));
    // And the digest states the very hex the identity carries.
    let hex = typed_value(first, &v().content_digest).expect("a digest");
    assert_eq!(hex.len(), 64);
    assert_eq!(
        first.subject,
        unit_iri(
            &v(),
            GUIDE_ID,
            &v1().chunking_id(),
            28,
            72,
            &text.as_bytes()[28..72]
        )
    );
    assert_eq!(
        ContentDigest::from_hex(&hex),
        Some(ContentDigest::of(&text.as_bytes()[28..72]))
    );
}

/// Every C0 control except the newline — which would end the line and
/// with it the unit — plus the DEL, the two characters N-Triples names
/// an escape for, and a C1 character the literal grammar leaves raw.
fn control_corpus() -> String {
    let mut out = String::from("ctrl ");
    for c in 1..0x20_u32 {
        if c != u32::from(b'\n') {
            out.push(char::from_u32(c).expect("a control character"));
        }
    }
    out.push('\u{7f}');
    out.push_str(" \"quoted\" \\ backslash \u{80}\u{9f} end");
    out
}

#[test]
fn a_literal_is_escaped_by_purrdf_cores_canonical_writer_the_delete_included() {
    let corpus = control_corpus();
    assert!(corpus.contains('\u{7f}'), "the DEL is in the corpus");
    let claims = slice(&format!("# T\n\n{corpus}\n"), &v1());
    let written = object(units(&claims)[0], &v().text).expect("text");
    assert_eq!(
        written,
        purrdf_core::emit_term(&purrdf_core::RdfTerm::Literal(purrdf_core::RdfLiteral {
            lexical_form: corpus.clone(),
            datatype: None,
            language: None,
            direction: None,
        })),
        "the crate's literal is the kernel's literal, byte for byte"
    );
    assert!(
        written.contains("\\u007F"),
        "the DEL is escaped \u{2014} the divergence the crate's own escaper carried"
    );
    assert!(!written.contains('\u{7f}'), "and it is never written raw");
    assert!(
        written.contains('\u{80}'),
        "while the C1 block stays raw, which the literal grammar permits"
    );
    assert_eq!(
        unescape(&written),
        corpus,
        "and it reads back to the bytes it was written from"
    );
    parses_whole(&claims);
}

#[test]
fn two_identical_paragraphs_are_two_distinct_nodes() {
    let text = "# B\n\nSame words here.\n\nSame words here.\n";
    let claims = slice(text, &v1());
    let u = units(&claims);
    assert_eq!(u.len(), 2);
    assert_ne!(u[0].subject, u[1].subject);
    assert_eq!(object(u[0], &v().text), object(u[1], &v().text));
}

#[test]
fn every_claim_parses_as_turtle_and_canonicalizes_under_purrdf() {
    for profile in [&v1(), &small()] {
        for claim in slice(GUIDE, profile) {
            let dataset = parse_dataset(claim.turtle.as_bytes(), "text/turtle", None)
                .unwrap_or_else(|e| panic!("{}: {e:?}", claim.subject));
            let canonical = try_canonicalize_with(&dataset, CanonHash::Sha256)
                .unwrap_or_else(|e| panic!("{}: {e:?}", claim.subject));
            assert_eq!(
                canonical.nquads.lines().count(),
                claim.turtle.lines().count(),
                "every line is one distinct triple"
            );
        }
    }
}

/// The declared profile's two ids, pinned as the goldens pin the
/// claims. The contract id is the one the document node states as its
/// `sliceProfile`, so `field-guide.nt` carries it too; the chunking id
/// is the one inside every node IRI, and nothing in the graph spells it
/// out, which is why it is spelled out here.
///
/// A change to the law moves one or both, and editing these two lines
/// is the deliberate step that records which. They are the real values
/// a report about a law change should quote.
const DECLARED_CONTRACT_ID: &str =
    "d4053c9d35631e2da17f7219349cf653397a7d946195552868890392230b2a55";
const DECLARED_CHUNKING_ID: &str =
    "40c979545ccb6e9a3007d38c93adb72c9735f39080a519e6d399bd6e0ed1e872";

/// The same profile with the byte bound widened to 4096 and nothing
/// else touched: a split constant, so **both** ids move, and these are
/// the values they move to.
const WIDER_CONTRACT_ID: &str = "61436f7b876f9dc7927fa82d5b9cf26a7eeed047fe8e939bba57386376584e86";
const WIDER_CHUNKING_ID: &str = "2a832603cbd4458673dc826e8b9018d0b489530fa46b31bc95a243927af9bc49";

/// A second canon base, under another authority: the third profile of
/// the split's central vector.
const OTHER_CANON_BASE: &str = "https://other.example/atlas/";

/// The declared profile with its byte bound widened and nothing else
/// changed.
fn wider() -> Profile {
    let mut profile = v1();
    profile.max_bytes = 4096;
    profile
}

#[test]
fn the_profile_id_states_itself_and_moves_with_every_constant_the_canon_base_among_them() {
    let id = |profile: &Profile| profile.contract_id().to_hex();
    let declared = id(&v1());
    assert_eq!(
        declared,
        id(&v1()),
        "the same constants state the same identity"
    );
    // The before value, pinned: a law change that moved it silently
    // would pass every other vector in this file.
    assert_eq!(declared, DECLARED_CONTRACT_ID);
    assert_eq!(v1().chunking_id().to_hex(), DECLARED_CHUNKING_ID);

    let mut renamed = v1();
    renamed.name = format!("{PROFILE_NAME}-other");
    let mut reversioned = v1();
    reversioned.version = 2;
    let mut revocabularied = v1();
    revocabularied.vocabulary.text = "https://example.org/other/body".to_owned();
    let mut looser = v1();
    looser.overlap = 64;
    let based = under_canon(CANON_BASE);
    let otherwise_based = under_canon(OTHER_CANON_BASE);
    let widened = wider();
    let moved = [
        &renamed,
        &reversioned,
        &revocabularied,
        &widened,
        &looser,
        // The clause that used to be outside the id, and the one the
        // whole split was drawn for: a declared canon base flips every
        // citation object from a typed literal to a minted IRI, so a
        // profile that declares one emits different bytes and MUST NOT
        // answer to the same id.
        &based,
        &otherwise_based,
    ];
    for profile in moved {
        assert_ne!(
            id(profile),
            declared,
            "a clause of the law is inside the identity: {}",
            String::from_utf8(profile.emission_bytes()).expect("utf8")
        );
    }
    let ids: BTreeSet<String> = moved.iter().map(|p| id(p)).collect();
    assert_eq!(ids.len(), moved.len(), "each change mints its own identity");

    // And the after value of the one deliberate constant change, pinned
    // beside the before value.
    assert_eq!(id(&widened), WIDER_CONTRACT_ID);
    assert_eq!(widened.chunking_id().to_hex(), WIDER_CHUNKING_ID);
    assert_ne!(WIDER_CONTRACT_ID, DECLARED_CONTRACT_ID);
    assert_ne!(WIDER_CHUNKING_ID, DECLARED_CHUNKING_ID);

    let claims = slice(GUIDE, &v1());
    assert_eq!(
        object(&claims[0], &v().slice_profile).as_deref(),
        Some(&*format!(
            "\"{PROFILE_NAME}:{declared}\"^^<{}>",
            v().dt_profile
        )),
        "the document states the profile it was sliced under"
    );
}

#[test]
fn three_profiles_that_differ_only_in_their_canon_base_are_three_contracts_under_one_chunking_law()
{
    // The whole of the split, in one vector. These three profiles are
    // the same law word for word but for where a canon lives, and they
    // emit three different graphs — so their contract ids must be three
    // values, and their chunking ids one, because not a byte of any
    // unit moved.
    let three = [v1(), under_canon(CANON_BASE), under_canon(OTHER_CANON_BASE)];

    let contracts: BTreeSet<String> = three.iter().map(|p| p.contract_id().to_hex()).collect();
    assert_eq!(
        contracts.len(),
        3,
        "a graph that differs is a contract that differs"
    );
    let chunkings: BTreeSet<String> = three.iter().map(|p| p.chunking_id().to_hex()).collect();
    assert_eq!(
        chunkings.len(),
        1,
        "not one boundary moved, so a consumer's embeddings are not stale"
    );
    assert_eq!(
        chunkings.iter().next().map(String::as_str),
        Some(DECLARED_CHUNKING_ID)
    );

    // The three graphs really are three graphs, and they differ where
    // the canon base is read: the object of every citation.
    let graphs: Vec<String> = three.iter().map(rendered).collect();
    assert_ne!(graphs[0], graphs[1]);
    assert_ne!(graphs[0], graphs[2]);
    assert_ne!(graphs[1], graphs[2]);
    for (profile, graph) in three.iter().zip(&graphs) {
        let claims = slice(GUIDE, profile);
        let one = *units(&claims)
            .iter()
            .find(|u| integer(u, &v().verse) == Some(1))
            .expect("verse 1");
        let expected = match &profile.canon_base {
            Some(base) => vec![format!("<{base}reef-shelf>"), format!("<{base}tide-line>")],
            None => vec![
                format!("\"reef-shelf\"^^<{}>", v().dt_anchor),
                format!("\"tide-line\"^^<{}>", v().dt_anchor),
            ],
        };
        assert_eq!(objects(one, &v().cites), expected);
        assert!(graph.contains(&format!(
            "\"{PROFILE_NAME}:{}\"",
            profile.contract_id().to_hex()
        )));
    }

    // The units are the same units under all three, because a canon
    // base says more about a verse and never changes what the verse is.
    let subjects = |profile: &Profile| -> Vec<String> {
        slice(GUIDE, profile)
            .iter()
            .filter(|c| c.kind != ClaimKind::Document)
            .map(|c| c.subject.clone())
            .collect()
    };
    assert_eq!(subjects(&three[0]), subjects(&three[1]));
    assert_eq!(subjects(&three[0]), subjects(&three[2]));

    // And the citation nodes are not — precisely the ones that reify
    // something. What a reifier reifies is inside its identity, so a
    // node that states a triple term is re-minted by the base, and a
    // node that states none is not, because nothing about it changed.
    let reifying = |profile: &Profile| -> BTreeSet<String> {
        slice(GUIDE, profile)
            .iter()
            .flat_map(citation_triples)
            .filter(|(_, predicate, _)| predicate == purrdf_markdown::RDF_REIFIES)
            .map(|(subject, _, _)| subject)
            .collect()
    };
    let silent = |profile: &Profile| -> BTreeSet<String> {
        let reifies = reifying(profile);
        slice(GUIDE, profile)
            .iter()
            .flat_map(citation_triples)
            .map(|(subject, _, _)| subject)
            .filter(|subject| !reifies.contains(subject))
            .collect()
    };
    let (none, first, second) = (
        reifying(&three[0]),
        reifying(&three[1]),
        reifying(&three[2]),
    );
    assert_eq!(none.len(), 11, "every lift of the guide but the silent one");
    assert!(none.is_disjoint(&first));
    assert!(none.is_disjoint(&second));
    assert!(first.is_disjoint(&second));
    // The neighbouring case, which must NOT move: the guide's one row
    // that names a source and no anchor mints a node that reifies
    // nothing, and a canon base it never reads cannot re-mint it.
    let quiet = silent(&three[0]);
    assert_eq!(quiet.len(), 1, "the guide's anchor-less row");
    assert_eq!(quiet, silent(&three[1]));
    assert_eq!(quiet, silent(&three[2]));
}

#[test]
fn every_pair_of_profiles_that_agrees_on_the_contract_id_agrees_on_every_byte_it_emits() {
    // The MUST of the identity law, asked as a property over a spread
    // of profile shapes rather than of one pair: some of them the same
    // law reached by two routes, the rest differing in exactly one
    // clause apiece.
    let mut renamed = v1();
    renamed.name = format!("{PROFILE_NAME}-other");
    let mut reversioned = v1();
    reversioned.version = 2;
    let mut retermed = v1();
    retermed.vocabulary.text = "https://example.org/other/body".to_owned();
    let mut rebased = v1();
    rebased.vocabulary = Vocabulary::under("https://example.org/other/").expect("a vocabulary");
    let mut looser = v1();
    looser.overlap = 64;
    // The declared law, reached by declaring a canon base and taking it
    // back again: the same value, and the id has to say so.
    let mut restored = under_canon(CANON_BASE);
    restored.canon_base = None;
    // And the declared law written out a second time from its parts.
    let again = Profile::new(PROFILE_NAME, 1, v());

    let profiles = [
        v1(),
        small(),
        under_canon(CANON_BASE),
        under_canon(CANON_PATH_BASE),
        renamed,
        reversioned,
        retermed,
        rebased,
        wider(),
        looser,
        restored,
        again,
    ];
    let (mut agreed, mut differed) = (0usize, 0usize);
    for (i, left) in profiles.iter().enumerate() {
        for right in &profiles[i + 1..] {
            if left.contract_id() == right.contract_id() {
                assert_eq!(
                    rendered(left),
                    rendered(right),
                    "two runs that agree on the contract id agree on every byte they emit"
                );
                agreed += 1;
            } else if rendered(left) != rendered(right) {
                differed += 1;
            }
        }
    }
    // Neither half of the property is vacuous: three pairs really do
    // share an id, and the rest really do emit different graphs.
    assert_eq!(agreed, 3, "the declared law is in the spread three times");
    assert!(
        differed >= 40,
        "the spread emits a wide field of different graphs: {differed}"
    );
}

#[test]
fn a_split_constant_moves_both_ids_and_a_term_or_a_canon_base_moves_only_the_contract_id() {
    let declared = v1();
    // A split constant re-cuts the text, so it is inside both.
    for moved in [wider(), small()] {
        assert_ne!(moved.contract_id(), declared.contract_id());
        assert_ne!(
            moved.chunking_id(),
            declared.chunking_id(),
            "a bound that re-cuts a unit is a chunking law that changed"
        );
    }
    // A vocabulary IRI and a canon base move neither boundary, so they
    // are inside the contract id and outside the chunking id: a
    // consumer's embeddings survive both.
    let mut retermed = declared.clone();
    retermed.vocabulary.cites = "https://example.org/other/cites".to_owned();
    for standing in [retermed, under_canon(CANON_BASE)] {
        assert_ne!(
            standing.contract_id(),
            declared.contract_id(),
            "the graph changed, so the contract changed"
        );
        assert_eq!(
            standing.chunking_id(),
            declared.chunking_id(),
            "not one chunk boundary moved, so no embedding is stale"
        );
    }
    // The name and the version are declarations about the cut as much
    // as about the graph, so they are inside both.
    let mut renamed = declared.clone();
    renamed.name = format!("{PROFILE_NAME}-other");
    assert_ne!(renamed.chunking_id(), declared.chunking_id());
    assert_ne!(renamed.contract_id(), declared.contract_id());
}

#[test]
fn the_chunking_preimage_is_a_prefix_of_the_emission_one_and_both_state_the_canon_base_or_its_absence()
 {
    for profile in [v1(), under_canon(CANON_BASE), small()] {
        let chunking = profile.chunking_bytes();
        let emission = profile.emission_bytes();
        assert!(
            emission.starts_with(&chunking),
            "no clause that moves a boundary can be outside the wider id"
        );
        assert!(emission.len() > chunking.len());
        let chunking = String::from_utf8(chunking).expect("utf8");
        let emission = String::from_utf8(emission).expect("utf8");
        // What the narrow preimage holds, and what it deliberately does
        // not.
        assert!(chunking.starts_with(&format!("{}\nversion {}\n", profile.name, profile.version)));
        assert!(chunking.contains(&format!("max_bytes {}\n", profile.max_bytes)));
        assert!(chunking.contains(&format!("overlap {}\n", profile.overlap)));
        assert!(!chunking.contains("vocabulary "));
        assert!(!chunking.contains("canon base"));
        assert!(!chunking.contains("emit "));
        // The canon base is stated either way, so its absence is a fact
        // of the preimage and never an omission from it.
        let stated = match &profile.canon_base {
            Some(base) => format!("canon base {base}\n"),
            None => "canon base none\n".to_owned(),
        };
        assert!(emission.contains(&stated), "{emission}");
        assert!(emission.contains(&format!("vocabulary {SLICE_BASE}\n")));
    }
}

#[test]
fn two_liftings_of_one_row_that_reify_different_terms_are_two_citation_nodes() {
    // The row, the span and the unit held fixed, so the only thing that
    // can tell these apart is the triple term each node reifies. Before
    // that term entered the preimage every one of these was the same
    // IRI, and a consumer merging two stores held one reifier reifying
    // several mutually exclusive triples.
    let contract = v1().chunking_id();
    let row = b"| 1 | `atlas/x.logic.ttl` | `a-one` |";
    let unit = units(&slice(GUIDE, &v1()))[0].subject.clone();
    let mint = |reified: &[String]| {
        purrdf_markdown::citation_iri(&v(), GUIDE_ID, &contract, 40, 76, row, &unit, reified)
    };
    let term =
        |predicate: &str, object: String| vec![format!("<<( <{unit}> <{predicate}> {object} )>>")];
    let literal = |anchor: &str| format!("\"{anchor}\"^^<{}>", v().dt_anchor);
    let minted = [
        // Two anchors under one row.
        mint(&term(&v().cites, literal("a-one"))),
        mint(&term(&v().cites, literal("a-two"))),
        // One anchor under two canon bases.
        mint(&term(&v().cites, format!("<{CANON_BASE}a-one>"))),
        mint(&term(&v().cites, format!("<{OTHER_CANON_BASE}a-one>"))),
        // One anchor cited through two spellings of the predicate: the
        // reifier states the whole triple, so the whole triple is in it.
        mint(&term("https://example.org/other/cites", literal("a-one"))),
        // And a row that reified nothing at all.
        mint(&[]),
    ];
    let distinct: BTreeSet<&String> = minted.iter().collect();
    assert_eq!(
        distinct.len(),
        minted.len(),
        "a different term is a different reifier"
    );
    // And it is still a function of what it is handed.
    assert_eq!(mint(&term(&v().cites, literal("a-one"))), minted[0]);
}

const THREE: &str = "# B\n\n1. One.\n\n2. Two two.\n\n3. Three.\n";

#[test]
fn a_same_length_substitution_re_mints_only_the_unit_it_touches() {
    let before = slice(THREE, &v1());
    let after = slice(&THREE.replace("Two two.", "Too twoo"), &v1());
    let (b, a) = (units(&before), units(&after));
    assert_eq!(b.len(), 3);
    for i in [0, 2] {
        assert_eq!(b[i].subject, a[i].subject);
        assert_eq!(b[i].span, a[i].span);
        assert_eq!(pairs(b[i]), pairs(a[i]), "not one triple of it moved");
    }
    assert_ne!(b[1].subject, a[1].subject);
    assert_eq!(b[1].span, a[1].span, "no boundary moved");
    // The two new facts move exactly with the identity they belong to:
    // a substitution of equal length changes the bytes and nothing else,
    // so the digest re-mints and the offsets — byte and scalar alike —
    // stand still.
    assert_ne!(
        typed_value(b[1], &v().content_digest),
        typed_value(a[1], &v().content_digest),
        "the digest is a statement about the bytes"
    );
    for offsets in [
        &v().byte_start,
        &v().byte_end,
        &v().scalar_start,
        &v().scalar_end,
    ] {
        assert_eq!(integer(b[1], offsets), integer(a[1], offsets));
    }
    assert_eq!(
        integer(b[1], &v().scalar_end).expect("an offset")
            - integer(b[1], &v().scalar_start).expect("an offset"),
        "2. Two two.".chars().count() as u64
    );
}

#[test]
fn an_insertion_shifts_every_later_span_by_its_length_and_changes_no_content() {
    let inserted = "0. Zero.\n\n";
    let text = THREE.replacen("1. One.", &format!("{inserted}1. One."), 1);
    let before = slice(THREE, &v1());
    let after = slice(&text, &v1());
    let (b, a) = (units(&before), units(&after));
    assert_eq!(a.len(), b.len() + 1);
    for (old, new) in b.iter().zip(&a[1..]) {
        let (os, oe) = old.span.expect("span");
        let (ns, ne) = new.span.expect("span");
        assert_eq!(
            (ns, ne),
            (os + inserted.len() as u64, oe + inserted.len() as u64)
        );
        assert_eq!(object(old, &v().text), object(new, &v().text));
        assert_ne!(old.subject, new.subject, "the span is inside the identity");
        // The opposite signature, in the two new facts: the offsets all
        // move by the insertion's length — the text is ASCII, so its
        // scalars are its bytes — and the digest does not move at all.
        let shift = inserted.chars().count() as u64;
        assert_eq!(
            integer(new, &v().scalar_start),
            integer(old, &v().scalar_start).map(|s| s + shift)
        );
        assert_eq!(
            integer(new, &v().scalar_end),
            integer(old, &v().scalar_end).map(|s| s + shift)
        );
        assert_eq!(
            typed_value(old, &v().content_digest),
            typed_value(new, &v().content_digest),
            "the bytes are untouched, so the digest is"
        );
    }
}

#[test]
fn inside_an_oversize_unit_the_two_edits_have_opposite_signatures() {
    let line = "abcdefghij klmnopqrst\n";
    let body: String = (0..40).map(|_| line).collect();
    let text = format!("# B\n\n{body}\n");
    let mut profile = v1();
    profile.max_bytes = 100;
    profile.overlap = 20;
    let before = slice(&text, &profile);
    let pieces = units(&before);
    assert!(pieces.len() > 3);
    let (s2, _) = pieces[2].span.expect("span");
    let mut substituted = text.clone().into_bytes();
    substituted[s2 as usize..s2 as usize + 3].copy_from_slice(b"ABC");
    let after = slice(std::str::from_utf8(&substituted).expect("utf8"), &profile);
    let changed: Vec<usize> = units(&after)
        .iter()
        .enumerate()
        .filter(|(i, u)| u.subject != pieces[*i].subject)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(units(&after).len(), pieces.len());
    assert!(
        units(&after)
            .iter()
            .zip(&pieces)
            .all(|(a, b)| a.span == b.span),
        "no boundary moved"
    );
    assert!(
        changed.iter().all(|i| {
            let (s, e) = pieces[*i].span.expect("span");
            s <= s2 && s2 < e
        }),
        "only pieces covering the edit re-mint: {changed:?}"
    );

    let inserted = "Inserted paragraph.\n\n";
    let shifted = text.replacen(&body, &format!("{inserted}{body}"), 1);
    let after = slice(&shifted, &profile);
    let shifted_pieces = units(&after);
    assert_eq!(shifted_pieces.len(), pieces.len() + 1);
    for (old, new) in pieces.iter().zip(&shifted_pieces[1..]) {
        let (os, oe) = old.span.expect("span");
        let (ns, ne) = new.span.expect("span");
        assert_eq!(
            (ns, ne),
            (os + inserted.len() as u64, oe + inserted.len() as u64)
        );
        assert_eq!(object(old, &v().text), object(new, &v().text));
    }
}

#[test]
fn the_whole_guide_slices_to_its_recorded_counts() {
    let claims = slice(GUIDE, &v1());
    let all = units(&claims);
    let verses = all
        .iter()
        .filter(|u| integer(u, &v().verse).is_some())
        .count();
    assert_eq!(verses, 8);
    assert_eq!(
        all.len(),
        8 + 6,
        "the headnote, two notes, the concordance prose, and each subsection's prose are paragraphs"
    );
    let count = |predicate: &str| -> usize {
        all.iter()
            .flat_map(|u| pairs(u))
            .filter(|(p, _)| p == predicate)
            .count()
    };
    let on_citations = |predicate: &str| -> usize {
        all.iter()
            .flat_map(|u| citation_triples(u))
            .filter(|(_, p, _)| p == predicate)
            .count()
    };
    // Two anchors per row in the first table and one in each row of the
    // second's; every verse is lifted onto, verse 3 by two rows, verses
    // 4 and 5 by two, and verse 2 by two of which one names no anchor.
    assert_eq!(count(&v().cites), 19);
    assert_eq!(count(&v().scalar_start), 14);
    assert_eq!(count(&v().scalar_end), 14);
    assert_eq!(count(&v().content_digest), 14);
    assert_eq!(
        count(&v().canon_source),
        0,
        "no source path hangs on a unit any more"
    );
    // Twelve (row, unit) lifts: 3 + 3 + 2 from the first table, 2 + 1
    // from the second's, which the innermost reading lost whole, and 1
    // from the third's anchor-less row. Each states one reification per
    // anchor, one path per source of its own row, and — whatever it
    // lifted — its class and the unit it is an edge of.
    let rows: BTreeSet<String> = all.iter().flat_map(|u| citation_nodes(u)).collect();
    assert_eq!(rows.len(), 12);
    assert_eq!(on_citations(purrdf_markdown::RDF_REIFIES), 19);
    assert_eq!(
        on_citations(&v().canon_source),
        17,
        "3\u{d7}1 + 3\u{d7}2 + 2\u{d7}1 + 2\u{d7}2 + 1\u{d7}1 + 1\u{d7}1: the escaped pipe keeps both paths of the row that names two, and the anchor-less row keeps its one"
    );
    assert_eq!(
        on_citations(purrdf_markdown::RDF_TYPE),
        12,
        "one class per citation node, unconditionally"
    );
    assert_eq!(
        on_citations(&v().in_unit),
        12,
        "and one back-edge to the unit it is an edge of, likewise"
    );
    assert_eq!(
        integer(&claims[0], &v().byte_length),
        Some(GUIDE.len() as u64)
    );
    assert_eq!(
        object(&claims[0], &v().title).as_deref(),
        Some(&*format!("\"{TITLE}\"^^<{}>", v().dt_heading))
    );
}

#[test]
fn non_utf8_bytes_and_an_unwritable_source_id_refuse_typed() {
    let bad = slice_markdown(
        &SourceDocument {
            id: GUIDE_ID,
            bytes: &[0x23, 0x20, 0xFF],
        },
        &v1(),
    );
    assert!(matches!(
        bad,
        Err(MarkdownError::InvalidUtf8 { valid_up_to: 2 })
    ));
    let bad = slice_markdown(
        &SourceDocument {
            id: "https://example.org/with space",
            bytes: b"# x\n",
        },
        &v1(),
    );
    assert!(matches!(
        bad,
        Err(MarkdownError::InvalidSourceId { found: ' ' })
    ));
}

/// A profile's bound and overlap, with everything else declared.
fn bounded(max_bytes: usize, overlap: usize) -> Profile {
    let mut profile = v1();
    profile.max_bytes = max_bytes;
    profile.overlap = overlap;
    profile
}

fn slice_of(text: &str, id: &str, profile: &Profile) -> Result<Vec<Claim>, MarkdownError> {
    slice_markdown(
        &SourceDocument {
            id,
            bytes: text.as_bytes(),
        },
        profile,
    )
}

#[test]
fn a_bound_under_the_widest_scalar_refuses_and_a_bound_of_four_slices_a_four_byte_scalar() {
    let text = "# B\n\n\u{1f41a}\u{1f41a}\u{1f41a} tail\n";
    for max_bytes in [0, 3] {
        assert_eq!(
            slice_of(text, GUIDE_ID, &bounded(max_bytes, 0)),
            Err(MarkdownError::InvalidMaxBytes {
                max_bytes,
                least: 4
            })
        );
    }
    // The neighbouring bound is lawful, and it is exactly the width of
    // the scalar it must never cut.
    let claims = slice_of(text, GUIDE_ID, &bounded(4, 0)).expect("slices");
    let all = units(&claims);
    assert!(all.len() > 1, "the paragraph is over the bound and splits");
    assert_eq!(
        unescape(&object(all[0], &v().text).expect("text")),
        "\u{1f41a}",
        "a four-byte scalar fills the bound whole"
    );
    let mut rejoined = String::new();
    for unit in &all {
        let (start, end) = unit.span.expect("span");
        assert!(text.is_char_boundary(start as usize) && text.is_char_boundary(end as usize));
        assert!(end - start <= 4, "no piece is over the bound");
        rejoined.push_str(&unescape(&object(unit, &v().text).expect("text")));
    }
    assert_eq!(rejoined, "\u{1f41a}\u{1f41a}\u{1f41a} tail");
}

#[test]
fn an_overlap_at_the_bound_refuses_and_one_byte_under_it_slices() {
    let body: String = (0..10).map(|_| "abcdefghij\n").collect();
    let text = format!("# B\n\n{body}");
    assert_eq!(
        slice_of(&text, GUIDE_ID, &bounded(20, 20)),
        Err(MarkdownError::InvalidOverlap {
            overlap: 20,
            max_bytes: 20
        })
    );
    assert_eq!(
        slice_of(&text, GUIDE_ID, &bounded(20, 400)),
        Err(MarkdownError::InvalidOverlap {
            overlap: 400,
            max_bytes: 20
        })
    );
    // One byte under the bound is lawful, and the split still advances.
    let claims = slice_of(&text, GUIDE_ID, &bounded(20, 19)).expect("slices");
    let all = units(&claims);
    assert!(all.len() > 1, "the paragraph is over the bound and splits");
    let mut previous = 0;
    for unit in &all {
        let (start, end) = unit.span.expect("span");
        assert!(end - start <= 20, "no piece is over the bound");
        assert!(start >= previous, "every piece starts at or after the last");
        previous = start;
    }
    assert_eq!(
        all.last().and_then(|u| u.span).map(|s| s.1),
        Some(text.len() as u64 - 1),
        "the split reaches the end of the paragraph"
    );
}

#[test]
fn a_control_character_in_the_profile_name_refuses_and_a_worded_name_slices() {
    for (name, found) in [("a\nb", '\n'), ("a\u{7f}b", '\u{7f}'), ("a\tb", '\t')] {
        let mut profile = v1();
        profile.name = name.to_owned();
        assert_eq!(
            slice_of(THREE, GUIDE_ID, &profile),
            Err(MarkdownError::InvalidProfileName { found }),
            "the stage description states one fact per line"
        );
    }
    // A name of several words, spaces and all, is no threat to a line.
    let mut worded = v1();
    worded.name = "example slice md, first law".to_owned();
    let claims = slice_of(THREE, GUIDE_ID, &worded).expect("slices");
    assert_eq!(units(&claims).len(), 3);
    assert_eq!(
        object(&claims[0], &v().slice_profile).as_deref(),
        Some(&*format!(
            "\"{}:{}\"^^<{}>",
            worded.name,
            worded.contract_id().to_hex(),
            v().dt_profile
        ))
    );
}

#[test]
fn an_empty_source_id_refuses_and_an_absolute_one_slices() {
    assert_eq!(
        slice_of(THREE, "", &v1()),
        Err(MarkdownError::EmptySourceId),
        "<> is a relative reference, not a document"
    );
    let id = "https://example.org/doc/three";
    let claims = slice_of(THREE, id, &v1()).expect("slices");
    assert_eq!(claims[0].subject, id);
    assert_eq!(units(&claims).len(), 3);
}

/// A whole small document: a title, a headnote, a verse, and a
/// concordance row that cites it.
const MARKED: &str = "# T\n\nA headnote.\n\n1. One line.\n\n## Concordance\n\n| Verses | Canon source | Anchors |\n| --- | --- | --- |\n| 1 | `atlas/x.logic.ttl` | `a-one` |\n";

#[test]
fn a_leading_byte_order_mark_shifts_every_span_and_changes_no_structure() {
    let marked = format!("\u{feff}{MARKED}");
    let plain = slice(MARKED, &v1());
    let with_mark = slice(&marked, &v1());
    // The mark's own bytes are covered like every other byte — as one
    // structure claim the unmarked twin has no counterpart of, so the
    // graph still decodes back to the marked bytes exactly.
    assert_eq!(with_mark.len(), plain.len() + 1);
    let (mark_claims, with_mark): (Vec<Claim>, Vec<Claim>) = with_mark
        .into_iter()
        .partition(|c| c.kind == ClaimKind::Structure && c.span == Some((0, 3)));
    assert_eq!(mark_claims.len(), 1, "the mark is one structure node");
    assert_eq!(
        object(&mark_claims[0], &v().verbatim).as_deref(),
        Some(&*format!("\"\u{feff}\"^^<{}>", v().dt_verbatim)),
        "and that node carries the mark itself"
    );
    assert_eq!(
        object(&with_mark[0], &v().title),
        object(&plain[0], &v().title),
        "the first line is a heading, not a paragraph"
    );
    assert_eq!(sections(&plain).len(), 2);
    assert_eq!(sections(&with_mark).len(), 2);
    // Minted over a shifted span, so a node IRI is expected to move;
    // everything the document says about itself is expected not to.
    let minted = [
        v().parent,
        v().in_section,
        v().continues,
        v().byte_start,
        v().byte_end,
        v().scalar_start,
        v().scalar_end,
        v().verbatim_start,
        v().verbatim_end,
    ];
    for (a, b) in plain.iter().zip(&with_mark).skip(1) {
        assert_eq!(a.kind, b.kind);
        let (a_start, a_end) = a.span.expect("span");
        let (b_start, b_end) = b.span.expect("span");
        assert_eq!(
            (b_start, b_end),
            (a_start + 3, a_end + 3),
            "the mark falls before every span"
        );
        let (a_pairs, b_pairs) = (pairs(a), pairs(b));
        assert_eq!(a_pairs.len(), b_pairs.len());
        for ((a_p, a_o), (b_p, b_o)) in a_pairs.iter().zip(&b_pairs) {
            assert_eq!(a_p, b_p);
            if minted.contains(a_p) {
                continue;
            }
            assert_eq!(a_o, b_o, "{a_p}");
        }
        assert_eq!(
            unescape_if_plain(a),
            unescape_if_plain(b),
            "a unit's literal is the verbatim bytes of its span"
        );
        // A citation is minted over the row's own line span, which the
        // mark moved too, so the node re-mints while the row it states
        // does not change at all.
        let (a_rows, b_rows) = (citation_nodes(a), citation_nodes(b));
        assert_eq!(a_rows.len(), b_rows.len());
        for (a_row, b_row) in a_rows.iter().zip(&b_rows) {
            assert_ne!(a_row, b_row, "the row's span is inside the citation's id");
            assert_eq!(
                citation_objects(a, a_row, &v().canon_source),
                citation_objects(b, b_row, &v().canon_source)
            );
        }
    }
    let cited = units(&with_mark)
        .into_iter()
        .find(|u| integer(u, &v().verse) == Some(1))
        .expect("verse 1");
    assert_eq!(
        objects(cited, &v().cites),
        vec![format!("\"a-one\"^^<{}>", v().dt_anchor)],
        "the concordance still lifts"
    );
    assert_eq!(
        object(cited, &v().lineage).as_deref(),
        Some(&*format!("\"T\"^^<{}>", v().dt_lineage))
    );
    assert_eq!(
        integer(&with_mark[0], &v().byte_length),
        Some(marked.len() as u64),
        "the document still counts its own bytes, the mark among them"
    );
}

/// A unit's verbatim literal, or none for a node that carries no text.
fn unescape_if_plain(claim: &Claim) -> Option<String> {
    object(claim, &v().text).map(|o| unescape(&o))
}

#[test]
fn a_byte_order_mark_after_the_first_byte_is_ordinary_content() {
    let text = "# T\n\nA line with \u{feff} inside it.\n\n\u{feff}## Not a heading\n";
    let claims = slice(text, &v1());
    assert_eq!(
        sections(&claims).len(),
        1,
        "only the opening mark is read as encoding"
    );
    let literals: Vec<String> = units(&claims)
        .iter()
        .map(|u| unescape(&object(u, &v().text).expect("text")))
        .collect();
    assert_eq!(
        literals,
        vec![
            "A line with \u{feff} inside it.".to_owned(),
            "\u{feff}## Not a heading".to_owned()
        ],
        "a mark inside the text is content, and a mark before a heading leaves a paragraph"
    );
    for unit in units(&claims) {
        let (start, end) = unit.span.expect("span");
        assert_eq!(
            unescape(&object(unit, &v().text).expect("text")),
            &text[start as usize..end as usize]
        );
    }
}

// --- the leading indent ----------------------------------------------------

/// The deepest leading run a marker is still read behind.
const LEADING_RUN: &str = "   ";

/// [`MARKED`] with every non-blank line opened by [`LEADING_RUN`]: the
/// heading, the paragraph, the verse, the concordance heading, and every
/// row of its table, each behind the deepest run the law admits.
fn indented_twin() -> String {
    let mut out = String::new();
    for line in MARKED.split_inclusive('\n') {
        if !line.trim().is_empty() {
            out.push_str(LEADING_RUN);
        }
        out.push_str(line);
    }
    out
}

#[test]
fn each_marker_the_law_names_is_read_behind_a_leading_run_of_spaces() {
    // A heading behind one space, and the verse under it: one section
    // and one verse, not two paragraphs under no title at all.
    let claims = slice_of(" # Title\n\n1. one\n", GUIDE_ID, &v1()).expect("slices");
    assert_eq!(sections(&claims).len(), 1);
    assert_eq!(
        typed_value(&claims[0], &v().title).as_deref(),
        Some("Title")
    );
    let u = units(&claims);
    assert_eq!(u.len(), 1, "the verse is the document's only unit");
    assert_eq!(integer(u[0], &v().verse), Some(1));

    // A heading behind the deepest run the law admits, with no trailing
    // newline to close it.
    let claims = slice_of("   # Title", GUIDE_ID, &v1()).expect("slices");
    let s = sections(&claims);
    assert_eq!(s.len(), 1);
    assert_eq!(plain_value(s[0], &v().heading).as_deref(), Some("Title"));
    assert_eq!(
        s[0].span,
        Some((0, 10)),
        "the heading span opens at the run's first byte and runs to the last"
    );

    // A verse behind two spaces: the run is no part of its number.
    let claims = slice_of("  1. one", GUIDE_ID, &v1()).expect("slices");
    let u = units(&claims);
    assert_eq!(u.len(), 1);
    assert_eq!(integer(u[0], &v().verse), Some(1));
    assert_eq!(
        unescape(&object(u[0], &v().text).expect("text")),
        "  1. one",
        "and the run is inside the verbatim literal all the same"
    );

    // A movement marker behind one space: a section, not a paragraph.
    let claims = slice_of(" \u{2042} *m*", GUIDE_ID, &v1()).expect("slices");
    let s = sections(&claims);
    assert_eq!(s.len(), 1);
    assert!(is_movement(s[0]));
    assert_eq!(plain_value(s[0], &v().heading).as_deref(), Some("m"));
    assert!(
        units(&claims).is_empty(),
        "a movement marker line is a section, never a unit"
    );
}

#[test]
fn nought_to_three_leading_spaces_open_a_marker_and_four_leave_an_ordinary_unit() {
    for indent in 0..=3_usize {
        let run = " ".repeat(indent);
        let text = format!("{run}# T\n\n{run}\u{2042} *m*\n\n{run}1. one\n");
        let claims = slice_of(&text, GUIDE_ID, &v1()).expect("slices");
        let s = sections(&claims);
        assert_eq!(s.len(), 2, "{indent} spaces open a heading and a movement");
        assert_eq!(plain_value(s[0], &v().heading).as_deref(), Some("T"));
        assert!(is_movement(s[1]), "{indent} spaces");
        assert_eq!(plain_value(s[1], &v().heading).as_deref(), Some("m"));
        assert_eq!(
            integer(s[1], &v().level),
            Some(2),
            "a movement still sits one under the heading it is behind"
        );
        let u = units(&claims);
        assert_eq!(u.len(), 1, "{indent} spaces");
        assert_eq!(integer(u[0], &v().verse), Some(1), "{indent} spaces");
        assert_eq!(
            unescape(&object(u[0], &v().text).expect("text")),
            format!("{run}1. one"),
            "the run is in the literal, never in the number"
        );
    }

    // Four spaces is where a marker stops being one, and nothing is
    // refused: the three lines are ordinary content, they slice into
    // ordinary units, and they carry no verse and open no section.
    let deep = "    # T\n\n    \u{2042} *m*\n\n    1. one\n";
    let claims = slice_of(deep, GUIDE_ID, &v1()).expect("four spaces is content, never a refusal");
    assert!(
        sections(&claims).is_empty(),
        "four spaces open neither a heading nor a movement"
    );
    assert_eq!(
        object(&claims[0], &v().title),
        None,
        "a document with no heading claims no title"
    );
    let u = units(&claims);
    assert_eq!(u.len(), 3, "one paragraph per line");
    assert!(u.iter().all(|piece| object(piece, &v().verse).is_none()));
    assert_eq!(
        u.iter()
            .map(|piece| unescape(&object(piece, &v().text).expect("text")))
            .collect::<Vec<_>>(),
        vec![
            "    # T".to_owned(),
            "    \u{2042} *m*".to_owned(),
            "    1. one".to_owned()
        ],
        "each line is kept whole, its run among the bytes"
    );
}

#[test]
fn a_tab_in_the_leading_run_opens_no_marker_because_it_reaches_the_fourth_column() {
    // CommonMark expands a tab to the next four-column tab stop, so a
    // run of nought to three spaces followed by a tab reaches column
    // four exactly — at or past the bound above. The law states that
    // outcome directly, on the line's bytes, and it is total: a tab
    // anywhere in the leading run opens no marker.
    for run in ["\t", " \t", "  \t", "   \t"] {
        let text = format!("{run}# T\n\n{run}\u{2042} *m*\n\n{run}1. one\n");
        let claims =
            slice_of(&text, GUIDE_ID, &v1()).expect("a leading tab is content, never a refusal");
        assert!(sections(&claims).is_empty(), "{run:?}");
        let u = units(&claims);
        assert_eq!(u.len(), 3, "{run:?}");
        assert!(
            u.iter().all(|piece| object(piece, &v().verse).is_none()),
            "{run:?}"
        );
        assert_eq!(
            unescape(&object(u[0], &v().text).expect("text")),
            format!("{run}# T"),
            "and the tab stays in the verbatim literal"
        );
    }
    // A tab *after* the marker is in no leading run at all, and it is
    // still the separator a heading and a verse are read by (§2.1).
    let claims = slice_of("   #\tT\n\n   1.\tone\n", GUIDE_ID, &v1()).expect("slices");
    assert_eq!(
        plain_value(sections(&claims)[0], &v().heading).as_deref(),
        Some("T")
    );
    assert_eq!(integer(units(&claims)[0], &v().verse), Some(1));
}

#[test]
fn an_indented_document_states_its_twins_structure_over_spans_shifted_by_the_leading_run() {
    let indented = indented_twin();
    let plain = slice(MARKED, &v1());
    let with_run = slice(&indented, &v1());
    assert_eq!(with_run.len(), plain.len());
    assert_eq!(sections(&plain).len(), 2);
    assert_eq!(sections(&with_run).len(), 2);
    assert_eq!(
        object(&with_run[0], &v().title),
        object(&plain[0], &v().title),
        "the first line is a heading behind its run, not a paragraph"
    );

    // The run moves an offset by itself once for every indented line
    // that opens at or before it, so a plain line start and a plain
    // line end move by different multiples of it — and both exactly.
    let shifted = |offset: u64| -> u64 {
        let runs = MARKED[..offset as usize]
            .split_inclusive('\n')
            .filter(|line| !line.trim().is_empty())
            .count();
        offset + (runs * LEADING_RUN.len()) as u64
    };

    // Minted over a shifted span, so a node IRI is expected to move;
    // everything the document says about itself is expected not to.
    let minted = [
        v().parent,
        v().in_section,
        v().continues,
        v().byte_start,
        v().byte_end,
        v().scalar_start,
        v().scalar_end,
        v().verbatim_start,
        v().verbatim_end,
    ];
    for (a, b) in plain.iter().zip(&with_run).skip(1) {
        assert_eq!(a.kind, b.kind);
        let (a_start, a_end) = a.span.expect("span");
        let (b_start, b_end) = b.span.expect("span");
        assert_eq!(
            (b_start, b_end),
            (shifted(a_start), shifted(a_end)),
            "every span moved by the runs before it and by nothing else"
        );
        if b.kind != ClaimKind::Structure {
            assert!(
                indented[b_start as usize..].starts_with(LEADING_RUN),
                "a span opens at its line's first byte, the run among them"
            );
        }
        let (a_pairs, b_pairs) = (pairs(a), pairs(b));
        assert_eq!(a_pairs.len(), b_pairs.len());
        for ((a_p, a_o), (b_p, b_o)) in a_pairs.iter().zip(&b_pairs) {
            assert_eq!(a_p, b_p);
            if minted.contains(a_p) {
                continue;
            }
            if *a_p == v().content_digest {
                // The run is inside the span, so it is inside the very
                // bytes the digest states — which is the whole reason
                // the literal keeps it. This is where the run parts
                // company with the byte order mark, which falls before
                // every span and enters no digest at all.
                let hex = typed_parts(b_o).0;
                assert_eq!(
                    ContentDigest::from_hex(&hex),
                    Some(ContentDigest::of(
                        &indented.as_bytes()[b_start as usize..b_end as usize]
                    ))
                );
                assert_ne!(a_o, b_o, "a run inside the span is a run inside the digest");
                continue;
            }
            if *a_p == v().verbatim {
                // A section's verbatim line keeps the run its line was
                // written with, and is its twin's line once it comes
                // off; a structure run between the lines carries no
                // run at all and is its twin's byte for byte.
                let (a_lex, a_dt) = typed_parts(a_o);
                let (b_lex, b_dt) = typed_parts(b_o);
                assert_eq!(a_dt, b_dt);
                if a.kind == ClaimKind::Section {
                    assert_eq!(b_lex, format!("{LEADING_RUN}{a_lex}"));
                } else {
                    // A structure run keeps the run of every non-blank
                    // line it carries — the table rows among them —
                    // and is its twin's once each comes off.
                    let stripped: String = b_lex
                        .split_inclusive('\n')
                        .map(|line| line.strip_prefix(LEADING_RUN).unwrap_or(line))
                        .collect();
                    assert_eq!(stripped, a_lex);
                }
                continue;
            }
            if *a_p == v().text {
                // A unit's literal is the verbatim bytes of its span,
                // so it keeps every run its lines were written with —
                // and is the plain twin's literal once they come off.
                let literal = unescape(b_o);
                assert_eq!(literal, indented[b_start as usize..b_end as usize]);
                assert!(literal.starts_with(LEADING_RUN));
                assert_eq!(
                    literal
                        .lines()
                        .map(|l| l.strip_prefix(LEADING_RUN).expect("an indented line"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    unescape(a_o),
                );
                continue;
            }
            assert_eq!(a_o, b_o, "{a_p}");
        }
    }

    // And what the run never entered: the heading it sits under, the
    // verse number, and the lift of a concordance table written behind
    // a run of its own.
    let cited = units(&with_run)
        .into_iter()
        .find(|u| integer(u, &v().verse) == Some(1))
        .expect("verse 1");
    assert_eq!(
        objects(cited, &v().cites),
        vec![format!("\"a-one\"^^<{}>", v().dt_anchor)],
        "an indented concordance still lifts"
    );
    assert_eq!(
        object(cited, &v().lineage).as_deref(),
        Some(&*format!("\"T\"^^<{}>", v().dt_lineage)),
        "and the run is no part of the heading the verse is under"
    );
}

#[test]
fn a_heading_whose_title_is_empty_after_the_leading_run_states_that_empty_title() {
    let source = "   #   \n\nbody\n";
    let claims = slice_of(source, GUIDE_ID, &v1()).expect("slices");
    let s = sections(&claims);
    assert_eq!(s.len(), 1);
    assert_eq!(
        plain_value(s[0], &v().heading).as_deref(),
        Some(""),
        "the run is no part of the title, and nothing else is left of it"
    );
    assert_eq!(
        typed_value(&claims[0], &v().title).as_deref(),
        Some(""),
        "an empty title is a title, not the absence of one"
    );
    assert_eq!(integer(s[0], &v().level), Some(1));
    assert_eq!(
        s[0].span,
        Some((0, source.len() as u64)),
        "the section runs to the end of the source"
    );
    assert_eq!(
        model(source, &v1()).sections()[0].heading_span(),
        Span::new(0, 7),
        "and the span its identity is minted over carries the run and the \
         trailing spaces alike"
    );
    assert_eq!(
        object(units(&claims)[0], &v().lineage).as_deref(),
        Some(&*format!("\"\"^^<{}>", v().dt_lineage))
    );
    // One space further in and the same line is no heading at all — it
    // is content, and content is what it stays.
    let claims = slice_of("    #   \n\nbody\n", GUIDE_ID, &v1()).expect("slices");
    assert!(
        sections(&claims).is_empty(),
        "one space further in and the hashes open nothing"
    );
    assert_eq!(units(&claims).len(), 2);
    assert_eq!(
        unescape(&object(units(&claims)[0], &v().text).expect("text")),
        "    #   "
    );
}

#[test]
fn the_declared_profile_reproduces_its_goldens_byte_for_byte() {
    // The vocabulary, the name, and the constants are inside the identity;
    // the same profile reproduces every emitted claim, so nothing
    // downstream re-mints on a move of this crate.
    assert_eq!(rendered(&v1()), GUIDE_GOLDEN);
    let mut based = v1();
    based.canon_base = Some(CANON_BASE.to_owned());
    assert_eq!(rendered(&based), GUIDE_GOLDEN_CANON);
    assert_ne!(
        GUIDE_GOLDEN, GUIDE_GOLDEN_CANON,
        "a declared canon base moves every anchor into an IRI"
    );
}

/// Writes the two goldens from the law as it stands. Ignored, so the
/// suite only ever compares: run it explicitly
/// (`cargo test -p purrdf-markdown -- --ignored regenerate_goldens`)
/// after an intended change to the law, then read the diff.
#[test]
#[ignore = "writes the golden fixtures; run it explicitly after an intended change"]
fn regenerate_goldens() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    std::fs::write(dir.join("field-guide.nt"), rendered(&v1())).expect("writes the golden");
    let mut based = v1();
    based.canon_base = Some(CANON_BASE.to_owned());
    std::fs::write(dir.join("field-guide.canon.nt"), rendered(&based))
        .expect("writes the canon golden");
}

#[test]
fn a_vocabulary_that_is_not_under_one_base_is_its_own_profile() {
    let mut explicit = v1();
    explicit.vocabulary.text = "https://example.org/other/body".to_owned();
    assert_ne!(explicit.contract_id().to_hex(), v1().contract_id().to_hex());
    let stage = String::from_utf8(explicit.emission_bytes()).expect("utf8");
    assert!(stage.contains("vocabulary explicit\n"));
    assert!(stage.contains("  text https://example.org/other/body\n"));
    let claims = slice(GUIDE, &explicit);
    let unit = units(&claims)[0];
    assert!(object(unit, "https://example.org/other/body").is_some());
    assert!(object(unit, &v().text).is_none());
    let mut broken = v1();
    broken.vocabulary.cites = String::new();
    assert!(matches!(
        slice_markdown(
            &SourceDocument {
                id: GUIDE_ID,
                bytes: b"# x\n",
            },
            &broken
        ),
        Err(MarkdownError::InvalidVocabulary { field: "cites", .. })
    ));
}

/// The compact vocabulary line states the **term set** and not only the
/// base, because a base names a namespace and not a set of terms: the
/// local names are the crate's, and a producer deriving other ones under
/// the same base emits a different graph.
///
/// So the emission preimage writes them out, and the dual-role spelling
/// this crate used to derive — one IRI as a section's `heading`
/// predicate and as the datatype of that literal — is a different law
/// with a different contract id. It is the same *cut*, though: not one
/// unit boundary moved, so the chunking id stands still and every unit
/// keeps its identity, which is the whole point of the two ids.
#[test]
fn the_term_set_is_inside_the_contract_id_and_the_dual_role_spelling_is_another_law() {
    let stage = String::from_utf8(v1().emission_bytes()).expect("utf8");
    assert!(stage.contains(&format!("vocabulary {SLICE_BASE}\n")));
    let terms = stage
        .lines()
        .find(|line| line.starts_with("terms "))
        .expect("the law names the terms it derives");
    let named: BTreeSet<&str> = terms
        .strip_prefix("terms ")
        .expect("the keyword")
        .split(' ')
        .collect();
    for local in [
        "Document",
        "Unit",
        "Structure",
        "heading",
        "headingText",
        "lineage",
        "lineagePath",
        "canonSource",
        "path",
        "verbatim",
        "verbatimStart",
        "verbatimEnd",
        "verbatimText",
    ] {
        assert!(named.contains(local), "{local} is named in the law");
    }
    assert_eq!(named.len(), 40, "every derived term is named once");

    let mut dual = v1();
    dual.vocabulary.dt_heading.clone_from(&v().heading);
    dual.vocabulary.dt_lineage.clone_from(&v().lineage);
    assert_ne!(
        dual.contract_id().to_hex(),
        v1().contract_id().to_hex(),
        "a term that moved is a law that moved"
    );
    assert_eq!(
        dual.chunking_id().to_hex(),
        v1().chunking_id().to_hex(),
        "and a term moves no boundary"
    );
    let subjects = |profile: &Profile| -> Vec<String> {
        slice(GUIDE, profile)
            .iter()
            .map(|c| c.subject.clone())
            .collect()
    };
    assert_eq!(
        subjects(&dual),
        subjects(&v1()),
        "so every node of the document keeps its identity"
    );
}

/// A whole small document whose one concordance row names `anchor` for
/// verse 1: the smallest thing that exercises the anchor lift.
fn cited(anchor: &str) -> String {
    format!(
        "# T\n\nA headnote.\n\n1. One line.\n\n## Concordance\n\n\
         | Verses | Canon source | Anchors |\n| --- | --- | --- |\n\
         | 1 | `atlas/x.logic.ttl` | `{anchor}` |\n"
    )
}

/// The declared profile with a canon base, so anchors lift into IRIs.
fn under_canon(base: &str) -> Profile {
    let mut profile = v1();
    profile.canon_base = Some(base.to_owned());
    profile
}

/// The `cites` objects of verse 1 of a sliced small document.
fn verse_one_cites(claims: &[Claim]) -> Vec<String> {
    let one = *units(claims)
        .iter()
        .find(|u| integer(u, &v().verse) == Some(1))
        .expect("verse 1");
    objects(one, &v().cites)
}

/// Every claim of a slice parses as Turtle: the lift is not merely
/// accepted, it is readable.
fn parses_whole(claims: &[Claim]) {
    for claim in claims {
        parse_dataset(claim.turtle.as_bytes(), "text/turtle", None)
            .unwrap_or_else(|e| panic!("{}: {e:?}", claim.subject));
    }
}

#[test]
fn an_anchor_that_mints_no_iri_refuses_under_a_canon_base_and_names_the_row_that_wrote_it() {
    for anchor in ["two words", "a>b"] {
        assert_eq!(
            slice_of(&cited(anchor), GUIDE_ID, &under_canon(CANON_BASE)),
            Err(MarkdownError::InvalidAnchor {
                anchor: anchor.to_owned(),
                verses: (1, 1),
            }),
            "the concatenation is not an IRI, and the row is named so it can be found"
        );
    }
    // The neighbouring anchor is ordinary, and it lifts.
    let claims = slice_of(&cited("a-one"), GUIDE_ID, &under_canon(CANON_BASE)).expect("slices");
    assert_eq!(
        verse_one_cites(&claims),
        vec![format!("<{CANON_BASE}a-one>")]
    );
    parses_whole(&claims);
}

#[test]
fn the_same_refused_anchors_are_lawful_typed_literals_when_no_canon_base_is_declared() {
    // Nothing is minted, so nothing is refused: the refusal is scoped to
    // IRI minting and says nothing about an anchor's bytes.
    for anchor in ["two words", "a>b", "../x", ".elsewhere.example/x"] {
        let claims = slice_of(&cited(anchor), GUIDE_ID, &v1()).expect("slices");
        assert_eq!(
            verse_one_cites(&claims),
            vec![format!("\"{anchor}\"^^<{}>", v().dt_anchor)]
        );
        parses_whole(&claims);
    }
}

/// The law a projection applies is the law the document was admitted
/// under, and a document admitted with no canon base mints nothing —
/// however hostile the anchor, and whatever base some other profile
/// declares. Nothing is minted under the profile that admitted this
/// document, so nothing was refused of it, and nothing may appear as an
/// IRI now.
#[test]
fn a_document_admitted_with_no_canon_base_renders_its_anchors_as_literals_and_mints_no_iri() {
    let traversal = "../../etc/passwd";
    let text = cited(traversal);
    // The base-declaring profile refuses this document outright: the
    // anchor climbs out of the base it would be minted under, which is
    // the containment law of the anchor lift.
    assert_eq!(
        slice_of(&text, GUIDE_ID, &under_canon(CANON_PATH_BASE)),
        Err(MarkdownError::InvalidAnchor {
            anchor: traversal.to_owned(),
            verses: (1, 1),
        }),
        "a base is declared, so the anchor is walked, and this one is outside it"
    );

    // Admitted under the profile that mints nothing, the document
    // projects exactly the anchors it was admitted with.
    let document = model(&text, &v1());
    let claims = render(&document);
    assert_eq!(
        verse_one_cites(&claims),
        vec![format!("\"{traversal}\"^^<{}>", v().dt_anchor)],
        "no canon base, no minting: the anchor stays the literal it was admitted as"
    );
    for base in [CANON_BASE, CANON_PATH_BASE] {
        let minted = format!("{base}{traversal}");
        assert!(
            claims.iter().all(|claim| !claim.turtle.contains(&minted)),
            "an IRI no profile admitted this document under is written nowhere, \
             asserted or reified"
        );
    }
    assert_eq!(claims, slice(&text, &v1()), "one law, two doors");
    parses_whole(&claims);
}

/// `render` takes the document alone. Its signature carries no second
/// profile, so the law of a projection is the law the document carries
/// and there is no argument through which another could be handed in.
#[test]
fn render_takes_the_document_alone_and_applies_the_law_the_document_carries() {
    let text = cited("a-one");
    let plain = model(&text, &v1());
    let based = model(&text, &under_canon(CANON_BASE));
    assert_eq!(
        plain.profile(),
        &v1(),
        "the document keeps what admitted it"
    );
    assert_eq!(based.profile(), &under_canon(CANON_BASE));
    // One call shape, two documents, two laws: the projection follows
    // the document it is handed, and it is handed nothing else.
    assert_eq!(
        verse_one_cites(&render(&plain)),
        vec![format!("\"a-one\"^^<{}>", v().dt_anchor)]
    );
    assert_eq!(
        verse_one_cites(&render(&based)),
        vec![format!("<{CANON_BASE}a-one>")]
    );
}

/// The neighbouring case the law must keep admitting: a document
/// analyzed *with* a canon base lifts every lawful anchor into the IRI
/// it was checked as, and analyze-then-render is slice-whole.
#[test]
fn a_document_admitted_under_a_canon_base_lifts_its_lawful_anchors_identically() {
    for anchor in ["a-one", "\u{4e2d}\u{6587}"] {
        let text = cited(anchor);
        let claims = render(&model(&text, &under_canon(CANON_BASE)));
        assert_eq!(
            verse_one_cites(&claims),
            vec![format!("<{CANON_BASE}{anchor}>")]
        );
        assert_eq!(
            claims,
            slice_of(&text, GUIDE_ID, &under_canon(CANON_BASE)).expect("slices"),
            "one law, two doors"
        );
        parses_whole(&claims);
    }
}

#[test]
fn a_cjk_anchor_lifts_under_a_canon_base_because_the_law_is_iri_lawfulness_not_ascii() {
    // RFC-3987 `ucschar` is inside an IRI, so `中文` needs no escaping
    // and no permission: it mints, and it mints verbatim.
    let anchor = "\u{4e2d}\u{6587}";
    for base in [CANON_BASE, CANON_PATH_BASE] {
        let claims = slice_of(&cited(anchor), GUIDE_ID, &under_canon(base)).expect("slices");
        assert_eq!(
            verse_one_cites(&claims),
            vec![format!("<{base}{anchor}>")],
            "an IRI is not an ASCII URI"
        );
        parses_whole(&claims);
    }
}

/// A path-shaped canon base, beside the fragment-shaped one the goldens
/// are minted under: the two shapes containment has to answer for.
const CANON_PATH_BASE: &str = "https://example.org/canon/";

#[test]
fn an_anchor_that_climbs_out_of_a_path_canon_base_refuses_and_a_plain_one_lifts() {
    assert_eq!(
        slice_of(&cited("../x"), GUIDE_ID, &under_canon(CANON_PATH_BASE)),
        Err(MarkdownError::InvalidAnchor {
            anchor: "../x".to_owned(),
            verses: (1, 1),
        }),
        "the minted IRI resolves outside the canon the caller declared"
    );
    // The same traversal under a FRAGMENT base climbs out of nothing: it
    // is a fragment, and it stays under the base. Refusing it there
    // would be an over-refusal.
    let claims = slice_of(&cited("../x"), GUIDE_ID, &under_canon(CANON_BASE)).expect("slices");
    assert_eq!(
        verse_one_cites(&claims),
        vec![format!("<{CANON_BASE}../x>")]
    );
    parses_whole(&claims);
    // And a plain anchor under the path base lifts as it reads.
    let claims =
        slice_of(&cited("a-one"), GUIDE_ID, &under_canon(CANON_PATH_BASE)).expect("slices");
    assert_eq!(
        verse_one_cites(&claims),
        vec![format!("<{CANON_PATH_BASE}a-one>")]
    );
    parses_whole(&claims);
}

#[test]
fn an_anchor_that_extends_the_bases_host_into_another_authority_refuses() {
    // Every character here is lawful in an IRI; only containment sees it.
    let host_base = "https://example.org";
    assert_eq!(
        slice_of(
            &cited(".elsewhere.example/x"),
            GUIDE_ID,
            &under_canon(host_base)
        ),
        Err(MarkdownError::InvalidAnchor {
            anchor: ".elsewhere.example/x".to_owned(),
            verses: (1, 1),
        }),
        "the concatenation lands on example.org.elsewhere.example"
    );
    // Under the same base an anchor that stays a path lifts.
    let claims = slice_of(&cited("/x"), GUIDE_ID, &under_canon(host_base)).expect("slices");
    assert_eq!(verse_one_cites(&claims), vec![format!("<{host_base}/x>")]);
    parses_whole(&claims);
}

#[test]
fn a_relative_canon_base_refuses_as_relative_and_an_absolute_one_lifts() {
    for base in ["canon#", "canon/", "#anchors"] {
        assert_eq!(
            slice_of(&cited("a-one"), GUIDE_ID, &under_canon(base)),
            Err(MarkdownError::InvalidCanonBase {
                base: base.to_owned()
            }),
            "the base is an IRI reference and carries no scheme, so nothing minted under it could"
        );
    }
    for base in [CANON_BASE, CANON_PATH_BASE] {
        let claims = slice_of(&cited("a-one"), GUIDE_ID, &under_canon(base)).expect("slices");
        assert_eq!(verse_one_cites(&claims), vec![format!("<{base}a-one>")]);
    }
}

#[test]
fn a_canon_base_that_is_no_iri_at_all_refuses_with_the_laws_own_finding() {
    // The empty base is here rather than beside the relative ones: it
    // is not a scheme-less IRI reference, it is no IRI reference.
    for (base, code) in [("", "iri-empty"), ("not an iri >", "iri-disallowed-char")]
        .into_iter()
        .chain(MALFORMED_IRIS)
    {
        let refusal =
            slice_of(&cited("a-one"), GUIDE_ID, &under_canon(base)).expect_err("no IRI, no base");
        match &refusal {
            MarkdownError::MalformedCanonBase { base: named, cause } => {
                assert_eq!(named, base);
                assert_eq!(cause.diagnostic_code(), code, "each defect keeps its own");
                let rendered = refusal.to_string();
                assert!(rendered.starts_with(&format!("canon base {base:?}")));
                assert!(rendered.ends_with(&cause.to_string()));
            }
            other => panic!("{base:?} is no IRI at all, and the refusal must say so: {other:?}"),
        }
    }
    // A base one character away from the truncated percent-encoding —
    // the triplet completed — is lawful and lifts.
    let base = "https://example.org/canon%2F";
    let claims = slice_of(&cited("a-one"), GUIDE_ID, &under_canon(base)).expect("slices");
    assert_eq!(verse_one_cites(&claims), vec![format!("<{base}a-one>")]);
    parses_whole(&claims);
}

#[test]
fn a_relative_vocabulary_refuses_and_the_absolute_one_slices() {
    // `under("Document")` derives `DocumentDocument`, `DocumentcitesX`,
    // and a node base of `Document`: writable, and every one of them
    // relative, so the kernel would refuse the lot at intern time.
    assert!(matches!(
        Vocabulary::under("Document"),
        Err(MarkdownError::InvalidVocabulary { .. })
    ));
    let mut broken = v1();
    broken.vocabulary.cites = "cites".to_owned();
    assert_eq!(
        slice_of(THREE, GUIDE_ID, &broken),
        Err(MarkdownError::InvalidVocabulary {
            field: "cites",
            iri: "cites".to_owned(),
        })
    );
    // The declared vocabulary is absolute example.org, and it slices.
    Vocabulary::under(SLICE_BASE).expect("an absolute base derives a vocabulary");
    assert_eq!(
        units(&slice_of(THREE, GUIDE_ID, &v1()).expect("slices")).len(),
        3
    );
}

/// Strings that are no IRI reference at all, each with the diagnostic
/// code the workspace IRI law states about it.
///
/// Every one of them is writable between `<` and `>`, so it reaches the
/// law rather than being stopped by the character rule in front of it,
/// and every one of them names a different defect. That is what the
/// seams have to keep apart: the code is the law's own, and a seam that
/// answered "has no scheme" would be stating something false about four
/// of these five, each of which plainly carries one.
const MALFORMED_IRIS: [(&str, &str); 5] = [
    // A percent-encoding the end of the string cuts off.
    ("https://example.org/%", "iri-bad-percent-encoding"),
    // An IP-literal that never closes its bracket.
    ("http://[not-an-ipv6", "iri-bad-authority"),
    // `~` is no scheme character, so the scheme is read, and refused.
    ("ht~tp://example.org/", "iri-bad-scheme"),
    // A scheme may not begin with a digit, so the law reads no scheme
    // here at all and refuses the `:` where it stands — a relative
    // reference's first segment may not carry one.
    ("1http://example.org/", "iri-disallowed-char"),
    // U+007F, which the IRI grammar admits in no component.
    ("https://example.org/a\u{7f}b", "iri-disallowed-char"),
];

#[test]
fn a_malformed_source_id_refuses_as_malformed_and_never_as_relative() {
    let mut codes = BTreeSet::new();
    for (id, code) in MALFORMED_IRIS {
        let refusal = slice_of(THREE, id, &v1()).expect_err("no IRI, no document");
        match &refusal {
            MarkdownError::MalformedSourceId { id: named, cause } => {
                assert_eq!(named, id);
                assert_eq!(cause.diagnostic_code(), code);
                codes.insert(cause.diagnostic_code());
                // The message names the seam and carries the law's
                // finding whole, rather than wording it again.
                let rendered = refusal.to_string();
                assert!(rendered.starts_with(&format!("source id {id:?}")));
                assert!(rendered.ends_with(&cause.to_string()));
                assert!(
                    !rendered.contains("no scheme"),
                    "{rendered:?} states a defect the id does not have"
                );
            }
            other => panic!("{id:?} is no IRI at all, and the refusal must say so: {other:?}"),
        }
    }
    assert_eq!(
        codes.len(),
        4,
        "four defects, four findings, and not one of them a missing scheme"
    );
    // The neighbouring refusal, unmoved: an id the law reads whole and
    // finds scheme-less is relative, and says so.
    assert_eq!(
        slice_of(THREE, "not-absolute", &v1()),
        Err(MarkdownError::RelativeSourceId {
            id: "not-absolute".to_owned()
        })
    );
    // The valid neighbour: one character from the truncated one, and it
    // slices.
    let id = "https://example.org/doc";
    let claims = slice_of(THREE, id, &v1()).expect("slices");
    assert_eq!(claims[0].subject, id);
    assert_eq!(units(&claims).len(), 3);
}

#[test]
fn a_malformed_vocabulary_iri_refuses_as_malformed_and_names_the_field() {
    for (iri, code) in MALFORMED_IRIS {
        let mut broken = v1();
        broken.vocabulary.cites = iri.to_owned();
        let refusal = slice_of(THREE, GUIDE_ID, &broken).expect_err("no IRI, no term");
        match &refusal {
            MarkdownError::MalformedVocabulary {
                field,
                iri: named,
                cause,
            } => {
                assert_eq!(*field, "cites");
                assert_eq!(named, iri);
                assert_eq!(cause.diagnostic_code(), code);
                let rendered = refusal.to_string();
                assert!(rendered.starts_with("vocabulary field cites"));
                assert!(rendered.ends_with(&cause.to_string()));
            }
            other => panic!("{iri:?} states no term, and the refusal must say so: {other:?}"),
        }
        // The same law asked directly answers the same way.
        assert!(matches!(
            broken.vocabulary.validate(),
            Err(MarkdownError::MalformedVocabulary { field: "cites", .. })
        ));
    }
    // The neighbouring refusals are unmoved: an empty field and a
    // scheme-less one are still the relative refusal.
    for iri in ["", "cites"] {
        let mut relative = v1();
        relative.vocabulary.cites = iri.to_owned();
        assert_eq!(
            slice_of(THREE, GUIDE_ID, &relative),
            Err(MarkdownError::InvalidVocabulary {
                field: "cites",
                iri: iri.to_owned(),
            })
        );
    }
    // And the declared vocabulary still slices.
    assert_eq!(
        units(&slice_of(THREE, GUIDE_ID, &v1()).expect("slices")).len(),
        3
    );
}

#[test]
fn a_relative_source_id_refuses_and_an_absolute_one_slices() {
    for id in ["not-absolute", "doc/field-guide", "#fragment"] {
        assert_eq!(
            slice_of(THREE, id, &v1()),
            Err(MarkdownError::RelativeSourceId { id: id.to_owned() }),
            "a scheme-less id names a document only relative to whatever holds the claims"
        );
    }
    let claims = slice_of(THREE, GUIDE_ID, &v1()).expect("slices");
    assert_eq!(claims[0].subject, GUIDE_ID);
    assert_eq!(units(&claims).len(), 3);
}

/// Documents of two-, three-, and four-byte scalars, with lines both
/// under and far over a bound of four or five bytes: the shapes the
/// split law has to answer for at the narrowest bound a profile may
/// declare, where a single scalar can fill a whole piece.
const NARROW: [&str; 3] = [
    "# H\n\n\u{e9}\u{2013}\u{1f41a}\u{e9}\u{e9}\u{2013}\u{1f41a}abc\n\u{1f41a}\u{1f41a}\n\u{e9}\n",
    "# H\n\nab\ncd\nef\ngh\nij\n",
    "# H\n\n\u{e9}\u{e9}\n\u{2013}\u{2013}\n\u{1f41a}\u{1f41a}\nabcdefghij\n",
];

#[test]
fn at_the_narrowest_bounds_every_piece_stays_inside_the_bound_and_the_pieces_cover_their_unit() {
    // The sweep completing at all is half the claim: at these bounds a
    // cut past the bound would index into the middle of a scalar.
    let mut newline_cuts = 0;
    let mut scalar_cuts = 0;
    let mut bound_inside_a_scalar = 0;
    let mut one_scalar_filled_the_bound = 0;
    let mut reached_back = 0;
    let mut snapped_to_a_line_start = 0;
    let mut snapped_to_a_scalar_boundary = 0;
    let mut cut_newlines_stepped_over = 0;
    for text in NARROW {
        let bytes = text.as_bytes();
        // The same units under a bound nothing reaches: the units the
        // pieces have to add back up to.
        let whole: Vec<(usize, usize)> = units(&slice(text, &v1()))
            .iter()
            .map(|u| span_of(u))
            .collect();
        assert!(!whole.is_empty(), "the document has units to split");
        for max_bytes in [MIN_MAX_BYTES, MIN_MAX_BYTES + 1] {
            for overlap in 0..max_bytes {
                let claims = slice(text, &bounded(max_bytes, overlap));
                let chains = chains(&claims);
                assert_eq!(
                    chains.len(),
                    whole.len(),
                    "a split makes pieces of a unit, never another unit"
                );
                for (chain, &(unit_start, unit_end)) in chains.iter().zip(&whole) {
                    assert_eq!(
                        span_of(chain[0]).0,
                        unit_start,
                        "the first piece opens the unit"
                    );
                    assert_eq!(
                        span_of(chain[chain.len() - 1]).1,
                        unit_end,
                        "the last piece closes it"
                    );
                    let mut covered = unit_start;
                    let mut previous: Option<(usize, usize)> = None;
                    for (at, piece) in chain.iter().enumerate() {
                        let (start, end) = span_of(piece);
                        assert!(end > start, "a piece is never empty");
                        assert!(
                            text.is_char_boundary(start) && text.is_char_boundary(end),
                            "a piece never begins or ends inside a scalar"
                        );
                        assert!(
                            end - start <= max_bytes,
                            "no piece is over the bound, line or no line"
                        );
                        assert!(
                            start >= unit_start && end <= unit_end,
                            "a piece stays inside its unit"
                        );
                        assert_eq!(
                            unescape(&object(piece, &v().text).expect("text")),
                            &text[start..end]
                        );
                        if start > covered {
                            assert_eq!(
                                start,
                                covered + 1,
                                "a split steps over nothing but the newline it cut at"
                            );
                            assert_eq!(bytes[covered], b'\n');
                            cut_newlines_stepped_over += 1;
                        }
                        covered = covered.max(end);
                        if let Some((p_start, p_end)) = previous {
                            assert!(start > p_start, "every piece starts after the one before");
                            let resume = if bytes[p_end] == b'\n' {
                                p_end + 1
                            } else {
                                p_end
                            };
                            assert!(start <= resume, "a continuation never skips forward");
                            if start < resume {
                                reached_back += 1;
                                let candidate = p_end - overlap;
                                assert!(
                                    start <= candidate,
                                    "a continuation reaches back at least the overlap"
                                );
                                assert!(start > p_start, "and never past the piece it continues");
                                if bytes[p_start..candidate].contains(&b'\n') {
                                    snapped_to_a_line_start += 1;
                                    assert_eq!(bytes[start - 1], b'\n', "snapped to a line start");
                                    assert!(
                                        !bytes[start..candidate].contains(&b'\n'),
                                        "the nearest line start at or before the candidate"
                                    );
                                } else {
                                    snapped_to_a_scalar_boundary += 1;
                                    assert_eq!(
                                        start,
                                        floor_boundary(text, candidate),
                                        "with no line start to reach, the scalar boundary at or \
                                         before the candidate"
                                    );
                                }
                            }
                        }
                        previous = Some((start, end));
                        if at + 1 == chain.len() {
                            continue;
                        }
                        // Everything below is the law of a cut, so it is
                        // asked only of a piece that was cut.
                        let bound = start + max_bytes;
                        assert!(end <= bound, "a cut is never past the bound");
                        assert!(
                            bound < unit_end,
                            "a unit is only cut where it runs past the bound"
                        );
                        match bytes[start + 1..=bound].iter().rposition(|b| *b == b'\n') {
                            Some(i) => {
                                newline_cuts += 1;
                                assert_eq!(
                                    end,
                                    start + 1 + i,
                                    "the last newline at or before the bound is preferred"
                                );
                                assert_eq!(
                                    bytes[end], b'\n',
                                    "the cut lands on that newline and the piece stops short of it"
                                );
                            }
                            None => {
                                scalar_cuts += 1;
                                assert_eq!(
                                    end,
                                    floor_boundary(text, bound),
                                    "with no newline the cut is the last scalar boundary at or \
                                     before the bound"
                                );
                                if !text.is_char_boundary(bound) {
                                    bound_inside_a_scalar += 1;
                                }
                                let opening =
                                    text[start..].chars().next().expect("a scalar").len_utf8();
                                if end == start + opening {
                                    one_scalar_filled_the_bound += 1;
                                }
                            }
                        }
                    }
                    assert_eq!(covered, unit_end, "the pieces cover the whole unit");
                }
            }
        }
    }
    assert!(newline_cuts > 0, "a line ends inside the bound somewhere");
    assert!(
        scalar_cuts > 0,
        "and somewhere a stretch carries no newline"
    );
    assert!(
        bound_inside_a_scalar > 0,
        "a scalar straddles the bound and the cut falls back off it"
    );
    assert!(
        one_scalar_filled_the_bound > 0,
        "one scalar fills a whole piece, which is why the bound cannot go under four"
    );
    assert!(reached_back > 0, "an overlap is reachable somewhere");
    assert!(
        snapped_to_a_line_start > 0 && snapped_to_a_scalar_boundary > 0,
        "both snaps of a continuation are exercised"
    );
    assert!(
        cut_newlines_stepped_over > 0,
        "a cut at a newline is the one place a byte is not carried into a piece"
    );
}

/// A small document written with CRLF endings: a heading, a movement, a
/// verse, a rule, and a paragraph.
const CRLF: &str = "# T\r\n\r\n\u{2042} *m*\r\n\r\n1. One line.\r\n\r\n---\r\n\r\npara\r\n";

#[test]
fn a_crlf_document_states_the_structure_its_lf_twin_states_and_keeps_the_return_in_its_text() {
    let claims = slice(CRLF, &v1());
    let s = sections(&claims);
    assert_eq!(s.len(), 2, "the heading and the movement are both read");
    assert_eq!(plain_value(s[0], &v().heading).as_deref(), Some("T"));
    assert_eq!(
        plain_value(s[1], &v().heading).as_deref(),
        Some("m"),
        "a movement name is trimmed, and the return trims away with it"
    );
    assert!(is_movement(s[1]));
    assert_eq!(integer(s[1], &v().level), Some(2));
    let u = units(&claims);
    assert_eq!(
        integer(u[0], &v().verse),
        Some(1),
        "a verse number is read from the opening digits, whatever ends the line"
    );
    let literals: Vec<String> = u
        .iter()
        .map(|piece| unescape(&object(piece, &v().text).expect("text")))
        .collect();
    assert_eq!(
        literals,
        vec!["1. One line.\r".to_owned(), "para\r".to_owned()],
        "the rule closed the verse, and every literal keeps the return its span holds"
    );
    for unit in &u {
        let (start, end) = span_of(unit);
        assert_eq!(
            unescape(&object(unit, &v().text).expect("text")),
            &CRLF[start..end],
            "the literal is still the verbatim bytes of the span"
        );
    }
    // The same document with LF endings states the same structure, and
    // says it in the same order, over shorter spans.
    let lf = CRLF.replace("\r\n", "\n");
    let plain = slice(&lf, &v1());
    assert_eq!(plain.len(), claims.len());
    for (a, b) in plain.iter().zip(&claims) {
        assert_eq!(a.kind, b.kind);
    }
    assert_eq!(
        typed_value(&plain[0], &v().title),
        typed_value(&claims[0], &v().title)
    );
    assert_eq!(
        sections(&plain)
            .iter()
            .map(|x| plain_value(x, &v().heading))
            .collect::<Vec<_>>(),
        sections(&claims)
            .iter()
            .map(|x| plain_value(x, &v().heading))
            .collect::<Vec<_>>()
    );
}

#[test]
fn an_empty_document_states_one_claim_and_it_is_the_document_itself() {
    let claims = slice_of("", GUIDE_ID, &v1()).expect("slices");
    assert_eq!(claims.len(), 1, "there is no structure to state");
    assert_eq!(claims[0].kind, ClaimKind::Document);
    assert_eq!(claims[0].subject, GUIDE_ID);
    assert_eq!(claims[0].span, None);
    assert_eq!(integer(&claims[0], &v().byte_length), Some(0));
    assert_eq!(
        object(&claims[0], &v().title),
        None,
        "a document with no heading claims no title rather than an empty one"
    );
}

#[test]
fn a_heading_at_the_end_of_the_document_needs_no_trailing_newline_to_open_a_section() {
    let claims = slice_of("# T", GUIDE_ID, &v1()).expect("slices");
    let s = sections(&claims);
    assert_eq!(s.len(), 1);
    assert_eq!(plain_value(s[0], &v().heading).as_deref(), Some("T"));
    assert_eq!(s[0].span, Some((0, 3)), "the section runs to the last byte");
    assert_eq!(typed_value(&claims[0], &v().title).as_deref(), Some("T"));
    assert!(
        units(&claims).is_empty(),
        "a heading line is a section, never a unit"
    );
}

#[test]
fn a_heading_of_nothing_but_hashes_states_an_empty_heading_and_still_opens_its_section() {
    let claims = slice_of("# #\n\nbody\n", GUIDE_ID, &v1()).expect("slices");
    let s = sections(&claims);
    assert_eq!(s.len(), 1);
    assert_eq!(
        plain_value(s[0], &v().heading).as_deref(),
        Some(""),
        "the closing hashes are trimmed and nothing is left of the title"
    );
    assert_eq!(
        typed_value(&claims[0], &v().title).as_deref(),
        Some(""),
        "the document's title is that same empty text, not the absence of one"
    );
    assert_eq!(integer(s[0], &v().level), Some(1));
    let u = units(&claims);
    assert_eq!(u.len(), 1);
    assert_eq!(
        object(u[0], &v().lineage).as_deref(),
        Some(&*format!("\"\"^^<{}>", v().dt_lineage)),
        "an empty heading is a heading in force all the same"
    );
}

#[test]
fn a_tab_after_the_hashes_and_after_the_verse_dot_opens_a_heading_and_a_verse() {
    let claims = slice_of("#\tTitle\n\n1.\tText\n", GUIDE_ID, &v1()).expect("slices");
    assert_eq!(
        plain_value(sections(&claims)[0], &v().heading).as_deref(),
        Some("Title"),
        "the tab separates the hashes from the title and trims away with the rest"
    );
    let u = units(&claims);
    assert_eq!(integer(u[0], &v().verse), Some(1));
    assert_eq!(
        unescape(&object(u[0], &v().text).expect("text")),
        "1.\tText",
        "the tab stays in the verbatim literal"
    );
    // With nothing at all after the hashes or the dot there is no
    // separator, and neither is claimed: the space or tab is the mark.
    let claims = slice_of("#Title\n\n1.Text\n", GUIDE_ID, &v1()).expect("slices");
    assert!(
        sections(&claims).is_empty(),
        "hashes running straight into the text open no section"
    );
    assert_eq!(
        units(&claims)
            .iter()
            .map(|piece| unescape(&object(piece, &v().text).expect("text")))
            .collect::<Vec<_>>(),
        vec!["#Title".to_owned(), "1.Text".to_owned()],
        "both lines are ordinary paragraphs"
    );
    assert!(
        units(&claims)
            .iter()
            .all(|piece| object(piece, &v().verse).is_none())
    );
}

#[test]
fn a_verse_number_over_the_bound_is_no_verse_number_and_the_largest_one_that_fits_still_is() {
    let over = "# T\n\n99999999999999999999999. text\n";
    let claims = slice_of(over, GUIDE_ID, &v1()).expect("slices");
    let u = units(&claims);
    assert_eq!(u.len(), 1);
    assert_eq!(
        object(u[0], &v().verse),
        None,
        "a run of digits no u64 carries leaves an ordinary paragraph"
    );
    assert_eq!(
        unescape(&object(u[0], &v().text).expect("text")),
        "99999999999999999999999. text",
        "and the line is kept whole, digits and all"
    );
    // The neighbouring number is the largest a u64 states, and it is a
    // verse: the bound refuses what it cannot carry, nothing more.
    let at_the_bound = format!("# T\n\n{}. text\n", u64::MAX);
    let claims = slice_of(&at_the_bound, GUIDE_ID, &v1()).expect("slices");
    assert_eq!(integer(units(&claims)[0], &v().verse), Some(u64::MAX));
}

// --- the typed stand-off model -------------------------------------------

#[test]
fn analyzing_then_rendering_is_slicing_and_the_model_counts_what_the_claims_state() {
    for profile in [&v1(), &small()] {
        let document = model(GUIDE, profile);
        let claims = slice(GUIDE, profile);
        assert_eq!(render(&document), claims, "one law, two doors");
        assert_eq!(document.id(), GUIDE_ID);
        assert_eq!(document.source(), GUIDE);
        assert_eq!(document.byte_length(), GUIDE.len() as u64);
        assert_eq!(document.title(), Some(TITLE));
        assert_eq!(document.sections().len(), sections(&claims).len());
        assert_eq!(document.units().len(), units(&claims).len());
        // Every claim's span is its node's span in the model, in order.
        let section_spans: Vec<Option<(u64, u64)>> = document
            .sections()
            .iter()
            .map(|s| Some((s.span().start, s.span().end)))
            .collect();
        assert_eq!(
            sections(&claims).iter().map(|c| c.span).collect::<Vec<_>>(),
            section_spans
        );
        let unit_spans: Vec<Option<(u64, u64)>> = document
            .units()
            .iter()
            .map(|u| Some((u.span().start, u.span().end)))
            .collect();
        assert_eq!(
            units(&claims).iter().map(|c| c.span).collect::<Vec<_>>(),
            unit_spans
        );
        // The digest the model carries is the one the identity is minted
        // from, so a consumer re-derives an IRI without re-hashing.
        for (unit, claim) in document.units().iter().zip(units(&claims)) {
            assert_eq!(unit.digest(), ContentDigest::of(unit.quote().as_bytes()));
            assert_eq!(
                claim.subject,
                unit_iri(
                    &v(),
                    GUIDE_ID,
                    &profile.chunking_id(),
                    unit.span().start,
                    unit.span().end,
                    unit.quote().as_bytes()
                )
            );
        }
    }
}

#[test]
fn the_lattice_answers_which_unit_holds_a_byte_which_units_a_range_touches_and_what_sits_under_a_section()
 {
    let document = model(GUIDE, &v1());
    let verse = |n: u64| {
        document
            .units()
            .iter()
            .find(|u| u.verse() == Some(n))
            .expect("a verse")
    };

    // A byte of a unit is that unit's; a byte of a heading line, or of
    // the newline that ends a unit, is no unit's.
    let one = verse(1);
    assert_eq!(document.unit_at(one.span().start), Some(one));
    assert_eq!(document.unit_at(one.span().end - 1), Some(one));
    assert_eq!(document.unit_at(one.span().end), None, "the line's newline");
    assert_eq!(document.unit_at(0), None, "the title's heading line");
    assert_eq!(document.unit_at(document.byte_length()), None);
    let concordance = &document.sections()[5];
    assert_eq!(
        document.unit_at(concordance.heading_span().start),
        None,
        "a heading is a section, never a unit"
    );

    // Intersection, not containment, and an empty range touches nothing.
    let two = verse(2);
    assert_eq!(
        document
            .covering(Span::new(one.span().start, two.span().end))
            .map(Unit::verse)
            .collect::<Vec<_>>(),
        vec![Some(1), Some(2)]
    );
    assert_eq!(
        document
            .covering(Span::new(one.span().start + 1, one.span().start + 2))
            .count(),
        1,
        "a range inside one unit touches that one"
    );
    assert_eq!(document.covering(Span::new(4, 4)).count(), 0);
    assert_eq!(
        document.covering(document.span()).count(),
        document.units().len()
    );

    // Containment, so a nested subsection's units are under its
    // ancestors too — which is exactly where it differs from the
    // innermost section a unit records.
    let title = &document.sections()[0];
    assert_eq!(title.span(), document.span());
    assert_eq!(
        document.units_under(title).count(),
        document.units().len(),
        "the title holds the whole document"
    );
    let notes = &document.sections()[3];
    assert_eq!(notes.heading(), "Field notes");
    assert_eq!(
        document.units_under(notes).count(),
        5,
        "its own paragraph, the Tide tables paragraph, and verses 6, 7 and 8"
    );
    assert_eq!(
        document
            .units()
            .iter()
            .filter(|u| u.section() == Some(3))
            .count(),
        1,
        "membership is the innermost section, and it is not containment"
    );
    let tables = &document.sections()[4];
    assert_eq!(document.units_under(tables).count(), 4);
    assert_eq!(
        document.units_under(&document.sections()[5]).count(),
        3,
        "the concordance prose and each subsection's prose; no table row is a unit"
    );
    for subsection in 6..=7 {
        assert_eq!(
            document
                .units_under(&document.sections()[subsection])
                .count(),
            1,
            "the subsection's own prose"
        );
    }

    // The section tree, read off the levels.
    assert_eq!(title.children(), &[1, 2, 3, 5]);
    assert_eq!(notes.children(), &[4]);
    assert_eq!(tables.children(), [0_usize; 0]);
    assert_eq!(
        document.sections()[5].children(),
        &[6, 7],
        "the concordance holds both of its subsections"
    );
    assert_eq!(
        document
            .child_sections(title)
            .map(|s| s.heading().to_owned())
            .collect::<Vec<_>>(),
        vec![
            "the outer reefs".to_owned(),
            "the inner lagoon".to_owned(),
            "Field notes".to_owned(),
            "Concordance".to_owned()
        ]
    );
    for section in document.sections() {
        assert!(
            title.span().contains_span(section.span()),
            "every section is under the document's own span"
        );
        if let Some(parent) = section.parent() {
            assert!(
                document.sections()[parent]
                    .span()
                    .contains_span(section.span()),
                "a child is contained in its parent"
            );
        }
    }
}

#[test]
fn where_a_split_overlaps_two_pieces_cover_a_byte_and_the_earlier_one_answers_for_it() {
    let document = model(GUIDE, &small());
    let (index, continuation) = document
        .units()
        .iter()
        .enumerate()
        .find(|(_, u)| u.continues().is_some())
        .expect("the fixture splits under the small bound");
    let previous = document
        .unit(continuation.continues().expect("a previous piece"))
        .expect("in range");
    let byte = continuation.span().start;
    assert!(previous.span().contains(byte), "the overlap reaches back");
    assert_eq!(
        document
            .covering(Span::new(byte, byte + 1))
            .map(Unit::ordinal)
            .collect::<Vec<_>>(),
        vec![previous.ordinal(), index as u64],
        "both pieces cover it"
    );
    assert_eq!(
        document.unit_at(byte),
        Some(previous),
        "and the earlier piece is the one unit_at answers with"
    );
    assert_eq!(
        previous.span().relation(continuation.span()),
        SpanRelation::Overlaps,
        "pieces of one unit overlap; they do not nest"
    );
}

#[test]
fn a_units_scalar_span_counts_scalars_where_its_byte_span_counts_bytes() {
    let text = "# T\u{ed}tulo\n\n\u{2042} *el camino*\n\n1. \u{201c}Quoted\u{201d} \u{2013} \u{4e2d} \u{1f41a} and \u{2042} inside.\n\n2. Tail\n";
    let document = model(text, &v1());
    let first = &document.units()[0];
    // Counted by hand: `# Título` is 8 scalars and 9 bytes, the blank
    // line and the marker line another 17 scalars and 19 bytes.
    assert_eq!(first.span(), Span::new(28, 72));
    assert_eq!(first.scalar_span(), Span::new(25, 56));
    assert_eq!(first.quote().chars().count(), 31);
    assert_eq!(first.quote().len(), 44);
    for unit in document.units() {
        let (start, end) = (unit.span().start as usize, unit.span().end as usize);
        assert_eq!(
            unit.scalar_span(),
            Span::new(
                text[..start].chars().count() as u64,
                text[..end].chars().count() as u64
            )
        );
        assert_eq!(
            unit.scalar_span().len(),
            unit.quote().chars().count() as u64,
            "the span's own length is its scalar count"
        );
        assert!(unit.scalar_span().len() <= unit.span().len());
    }
    // A leading byte order mark is one scalar of the document, counted,
    // exactly as it is three bytes of it, counted.
    let marked = format!("\u{feff}{text}");
    let with_mark = model(&marked, &v1());
    for (plain, marked) in document.units().iter().zip(with_mark.units()) {
        assert_eq!(marked.span().start, plain.span().start + 3);
        assert_eq!(marked.scalar_span().start, plain.scalar_span().start + 1);
    }
}

/// Ten four-byte scalars, a short unit, then ten more: the shape that
/// puts a scalar boundary across the context bound on both sides.
fn shells() -> String {
    let shells = "\u{1f41a}".repeat(10);
    format!("{shells}\n\nmiddle unit.\n\n{shells}\n")
}

#[test]
fn a_content_anchor_quotes_a_unit_exactly_and_snaps_its_context_to_scalars_and_to_the_documents_edges()
 {
    let text = shells();
    let document = model(&text, &v1());
    let units = document.units();
    assert_eq!(units.len(), 3);

    let middle = &units[1];
    assert_eq!(middle.quote(), "middle unit.");
    let anchor = middle.anchor();
    assert_eq!(anchor.exact(), middle.quote());
    assert_eq!(anchor.prefix(), middle.prefix());
    assert_eq!(anchor.suffix(), middle.suffix());
    // 32 bytes back from the unit's start lands inside a four-byte
    // scalar, so the prefix snaps forward and comes back short.
    assert_eq!(
        middle.prefix(),
        format!("{}\n\n", "\u{1f41a}".repeat(7)),
        "the context begins at a scalar boundary, never inside one"
    );
    assert_eq!(middle.prefix().len(), 30);
    assert_eq!(
        middle.suffix(),
        format!("\n\n{}", "\u{1f41a}".repeat(7)),
        "and ends at one"
    );
    assert_eq!(middle.suffix().len(), 30);

    // The document's own edges bound the context before the byte count
    // does: nothing precedes the first unit, and one newline follows the
    // last.
    assert_eq!(units[0].prefix(), "");
    assert_eq!(units[0].anchor().prefix(), "");
    assert_eq!(units[2].suffix(), "\n");
    // And an anchor is deterministic, bounded, and always text.
    for unit in units {
        assert!(unit.prefix().len() <= CONTEXT_BYTES);
        assert!(unit.suffix().len() <= CONTEXT_BYTES);
        let start = unit.span().start as usize;
        let end = unit.span().end as usize;
        assert_eq!(&text[start - unit.prefix().len()..start], unit.prefix());
        assert_eq!(&text[end..end + unit.suffix().len()], unit.suffix());
        assert_eq!(
            unit.anchor(),
            model(&text, &v1()).units()[unit.ordinal() as usize].anchor()
        );
    }
    // Two identical paragraphs share their quote and differ in context:
    // that is what the context is for.
    let twins = "# B\n\nSame words here.\n\nSame words here.\n";
    let twinned = model(twins, &v1());
    let (a, b) = (&twinned.units()[0], &twinned.units()[1]);
    assert_eq!(a.quote(), b.quote());
    assert_ne!(a.anchor(), b.anchor());
    assert_eq!(a.prefix(), "# B\n\n");
    assert_eq!(b.prefix(), "# B\n\nSame words here.\n\n");
}

// --- the concordance, and what it did not lift ----------------------------

#[test]
fn a_row_states_its_own_anchors_beside_its_own_sources_which_the_flat_projection_cannot() {
    let document = model(GUIDE, &v1());
    let rows = document.citations();
    assert_eq!(
        rows.len(),
        6,
        "three data rows, the second table's two, and the anchor-less row of the third; no table's frame is one"
    );
    assert_eq!(rows[0].verses(), (1, 3));
    assert_eq!(rows[0].anchors(), ["reef-shelf", "tide-line"]);
    assert_eq!(rows[0].sources(), ["atlas/outer-reefs.logic.ttl"]);
    assert_eq!(
        rows[1].anchors(),
        ["lagoon-floor", "salt-pan"],
        "prose beside a backticked anchor lifts nothing"
    );
    assert_eq!(
        rows[1].sources(),
        [
            "atlas/inner-lagoon.logic.ttl",
            "atlas/outer-reefs.logic.ttl"
        ],
        "and the row keeps both of its own sources, unmerged"
    );
    // A row names the units it lifted onto, and they are first pieces.
    for row in rows {
        for &(verse, unit) in row.lifted() {
            let unit = document.unit(unit).expect("in range");
            assert_eq!(unit.verse(), Some(verse));
            assert_eq!(unit.continues(), None, "a row lifts onto a first piece");
        }
        assert!(
            document.source()[row.span().start as usize..row.span().end as usize].contains('|')
        );
    }
}

#[test]
fn a_row_whose_verses_are_not_in_this_document_lifts_nothing_here_and_is_not_an_error() {
    // The guide's last row covers 7–9 and the guide stops at 8: the
    // verses that exist lift, the one that does not is reported.
    let document = model(GUIDE, &v1());
    let last = &document.citations()[2];
    assert_eq!(last.verses(), (7, 9));
    assert_eq!(
        last.lifted().iter().map(|&(v, _)| v).collect::<Vec<_>>(),
        vec![7, 8]
    );
    assert_eq!(last.unmatched(), [(9, 9)], "one verse is one run of one");
    assert!(!last.is_unmatched(), "it lifted two of its three verses");
    assert_eq!(
        document.unmatched_citations().count(),
        0,
        "no row of the guide lifted nothing at all"
    );
    assert!(
        document.malformed_rows().is_empty(),
        "a table's header and its |---|---|---| are frame, not defects"
    );

    // A whole row for a canon this document does not carry: still no
    // refusal, still reported, and the neighbouring row still lifts.
    let wider = "# T\n\n1. One.\n\n## Concordance\n\n| Verses | Canon source | Anchors |\n| --- | --- | --- |\n| 1 | `atlas/a.ttl` | `a-one` |\n| 40\u{2013}42 | `atlas/b.ttl` | `b-two` |\n";
    let document = model(wider, &v1());
    assert_eq!(document.citations().len(), 2);
    assert_eq!(document.malformed_rows().len(), 0);
    let unmatched: Vec<(u64, u64)> = document
        .unmatched_citations()
        .map(purrdf_markdown::Citation::verses)
        .collect();
    assert_eq!(unmatched, vec![(40, 42)]);
    assert_eq!(
        document.citations()[1].unmatched(),
        [(40, 42)],
        "a row that lifted nothing states its whole range as one run"
    );
    assert_eq!(
        document.citations()[0].lifted(),
        [(1, 0)],
        "verse 1 is the document's first unit"
    );
    let claims = slice_of(wider, GUIDE_ID, &v1()).expect("slices");
    assert_eq!(
        verse_one_cites(&claims),
        vec![format!("\"a-one\"^^<{}>", v().dt_anchor)],
        "the row that matches lifts exactly as it would alone"
    );
    parses_whole(&claims);
}

/// A row's verse range is two `u64`s the **document** wrote, and a
/// document is not trusted: `1–18446744073709551615` is a row the
/// dialect reads without complaint, and it is not malformed — there is
/// nothing wrong with it to report. So it has to be *answered*, in time
/// proportional to what this document carries rather than to what the
/// row claims. This test returning at all is that proof: a slicer that
/// walked the range verse by verse would never reach its first
/// assertion, and one that listed the misses verse by verse would ask
/// for more memory than exists.
///
/// The neighbouring valid case rides in the same table — an ordinary
/// small range over the same verses — and lifts exactly what it lifts
/// alone. What is bounded here is the walk, not the answer.
#[test]
fn a_row_naming_the_whole_u64_range_answers_in_the_documents_own_terms() {
    let text = "# T\n\n1. One.\n\n2. Two.\n\n4. Four.\n\n## Concordance\n\n\
                | Verses | Canon source | Anchors |\n| --- | --- | --- |\n\
                | 1\u{2013}18446744073709551615 | `atlas/a.ttl` | `a-all` |\n\
                | 2\u{2013}4 | `atlas/b.ttl` | `b-some` |\n";
    let document = model(text, &v1());
    assert!(
        document.malformed_rows().is_empty(),
        "the row reads as a range, and enormous is not malformed"
    );
    assert_eq!(document.citations().len(), 2);

    let whole = &document.citations()[0];
    assert_eq!(
        whole.verses(),
        (1, u64::MAX),
        "the row's own claim, carried out unchanged"
    );
    assert_eq!(
        whole.lifted(),
        [(1, 0), (2, 1), (4, 2)],
        "every verse this document carries inside the range, and nothing else"
    );
    assert_eq!(
        whole.unmatched(),
        [(3, 3), (5, u64::MAX)],
        "the gap it named and the tail past the last verse, as two runs"
    );
    assert!(!whole.is_unmatched(), "it lifted three verses");
    assert_eq!(
        document.unmatched_citations().count(),
        0,
        "neither row lifted nothing at all"
    );

    let small = &document.citations()[1];
    assert_eq!(
        small.lifted(),
        [(2, 1), (4, 2)],
        "the small range lifts exactly what it would lift alone"
    );
    assert_eq!(small.unmatched(), [(3, 3)]);

    // Verse 2 is named by both rows, so the projection still keeps each
    // row's anchor beside that row's own sources.
    let claims = slice_of(text, GUIDE_ID, &v1()).expect("slices");
    let two = *units(&claims)
        .iter()
        .find(|u| integer(u, &v().verse) == Some(2))
        .expect("verse 2");
    assert_eq!(
        objects(two, &v().cites),
        vec![
            format!("\"a-all\"^^<{}>", v().dt_anchor),
            format!("\"b-some\"^^<{}>", v().dt_anchor),
        ]
    );
    parses_whole(&claims);
}

#[test]
fn a_row_too_malformed_to_read_is_reported_as_data_and_the_rows_beside_it_still_lift() {
    let text = "# T\n\n1. One.\n\n2. Two.\n\n## Concordance\n\n\
                | Verses | Canon source | Anchors |\n| --- | --- | --- |\n\
                | 1 | `atlas/a.ttl` | `a-one` |\n\
                | oops | `atlas/b.ttl` | `b-two` |\n\
                | 2 |\n\
                | 2 | `atlas/c.ttl` | `c-two` |\n";
    let document = model(text, &v1());
    let malformed = document.malformed_rows();
    assert_eq!(malformed.len(), 2, "the header and the delimiter are frame");
    assert_eq!(malformed[0].defect(), RowDefect::UnreadableVerseRange);
    assert_eq!(malformed[0].line(), "| oops | `atlas/b.ttl` | `b-two` |");
    assert_eq!(malformed[1].defect(), RowDefect::TooFewCells { found: 1 });
    assert_eq!(malformed[1].line(), "| 2 |");
    for row in malformed {
        assert_eq!(
            row.line(),
            &text[row.span().start as usize..row.span().end as usize]
        );
    }
    // The rows that read still read, and nothing about the emitted
    // claims changed: a malformed row is reported, never refused.
    assert_eq!(document.citations().len(), 2);
    assert_eq!(document.citations()[0].verses(), (1, 1));
    assert_eq!(document.citations()[1].verses(), (2, 2));
    let claims = slice_of(text, GUIDE_ID, &v1()).expect("slices");
    assert_eq!(
        verse_one_cites(&claims),
        vec![format!("\"a-one\"^^<{}>", v().dt_anchor)]
    );
    let two = *units(&claims)
        .iter()
        .find(|u| integer(u, &v().verse) == Some(2))
        .expect("verse 2");
    assert_eq!(
        objects(two, &v().cites),
        vec![format!("\"c-two\"^^<{}>", v().dt_anchor)],
        "the readable row for verse 2 lifts, the malformed ones lift nothing"
    );
    parses_whole(&claims);
}

/// The rows both halves of the containment vector read, written once so
/// that the flat document and the subsectioned one differ in nothing but
/// the heading between the prose and the table.
const NESTED_ROWS: &str = "| Verses | Canon source | Anchors |\n|---|---|---|\n\
                           | 1\u{2013}3 | `atlas/a.ttl` | `alpha` |\n\
                           | 2 | `atlas/b.ttl` | `beta` |\n";

/// One row's claim and its lift, with no node identity in it: the verse
/// range, the sources, the anchors, and the verses it lifted onto.
type LiftedRow = ((u64, u64), Vec<String>, Vec<String>, Vec<u64>);

/// What every row of a document claims and what it lifted: the part of a
/// concordance two differently-spelled documents must agree on.
fn lifted_rows(document: &Document<'_>) -> Vec<LiftedRow> {
    document
        .citations()
        .iter()
        .map(|row| {
            (
                row.verses(),
                row.sources().to_vec(),
                row.anchors().to_vec(),
                row.lifted().iter().map(|&(verse, _)| verse).collect(),
            )
        })
        .collect()
}

/// §4: *inside* the concordance is §1.1's containment, not the innermost
/// section. A concordance organised into subsections — one per volume,
/// one per surveyor — must lift exactly what the same rows lift written
/// flat, because the rows are inside the concordance's span either way.
///
/// The failure this closes is silent, which is why the assertion is
/// equality with the flat twin rather than a count: under the innermost
/// reading every row of the subsectioned document is read as nothing, so
/// it is neither a citation nor a malformed row, and the model states
/// `citations=0 malformed=0 unmatched=0` — a document that looks in every
/// report exactly like one whose concordance was empty.
#[test]
fn a_concordance_with_a_subsection_lifts_exactly_what_the_flat_form_lifts() {
    let verses = "# T\n\n1. One.\n\n2. Two.\n\n3. Three.\n\n## Concordance\n\n";
    let flat = format!("{verses}{NESTED_ROWS}");
    let subsectioned = format!("{verses}### Book one\n\n{NESTED_ROWS}");
    let flat = model(&flat, &v1());
    let subsectioned = model(&subsectioned, &v1());

    assert_eq!(lifted_rows(&subsectioned), lifted_rows(&flat));
    assert_eq!(
        lifted_rows(&flat).len(),
        2,
        "and it is not that both lifted nothing"
    );
    assert_eq!(
        subsectioned.citations()[0].lifted(),
        [(1, 0), (2, 1), (3, 2)],
        "onto the very units the flat document lifts onto"
    );
    assert!(
        subsectioned.malformed_rows().is_empty(),
        "and no row of it was read as a defect either"
    );
    assert_eq!(subsectioned.unmatched_citations().count(), 0);
}

/// The same law at a depth the innermost reading cannot reach even by
/// accident, and with the subsection's own table carrying its own frame.
#[test]
fn a_row_two_levels_under_the_concordance_heading_still_lifts() {
    let text = "# T\n\n1. One.\n\n## Concordance\n\n### Book one\n\n\u{2042} *the first hand*\n\n\
                | Verses | Canon source | Anchors |\n|---|---|---|\n\
                | 1 | `atlas/a.ttl` | `alpha` |\n";
    let document = model(text, &v1());
    assert_eq!(document.citations().len(), 1);
    assert_eq!(document.citations()[0].lifted(), [(1, 0)]);
    assert_eq!(document.citations()[0].anchors(), ["alpha"]);
    assert!(
        document.malformed_rows().is_empty(),
        "the subsection's table carries its own header and delimiter, and both are frame"
    );
    let claims = slice_of(text, GUIDE_ID, &v1()).expect("slices");
    assert_eq!(
        verse_one_cites(&claims),
        vec![format!("\"alpha\"^^<{}>", v().dt_anchor)]
    );
    parses_whole(&claims);
}

/// The neighbouring valid case, and the half of §4 the containment
/// reading must not trade away: a table no concordance section **holds**
/// lifts nothing at all. Containment is what admits a row, so a table
/// before the concordance, a table in the section whose heading closed
/// it, and a table under a subsection of *that* section are all
/// structure — and none of them is a defect either.
#[test]
fn a_table_no_concordance_section_holds_lifts_nothing_and_is_no_defect() {
    let table = |path: &str, anchor: &str| {
        format!(
            "| Verses | Canon source | Anchors |\n|---|---|---|\n| 1 | `{path}` | `{anchor}` |\n\n"
        )
    };
    let text = format!(
        "# T\n\n1. One.\n\n## Tide tables\n\n{}## Concordance\n\n{}## Afterword\n\n{}### Later notes\n\n{}",
        table("atlas/before.ttl", "not-a-citation"),
        table("atlas/a.ttl", "alpha"),
        table("atlas/after.ttl", "nor-this-one"),
        table("atlas/deeper.ttl", "nor-this-one-either"),
    );
    let document = model(&text, &v1());
    assert_eq!(
        document.citations().len(),
        1,
        "only the row a concordance section holds"
    );
    assert_eq!(document.citations()[0].anchors(), ["alpha"]);
    assert!(
        document.malformed_rows().is_empty(),
        "a table elsewhere is structure, and structure is no defect"
    );
    let claims = slice_of(&text, GUIDE_ID, &v1()).expect("slices");
    assert_eq!(
        verse_one_cites(&claims),
        vec![format!("\"alpha\"^^<{}>", v().dt_anchor)],
        "the verse cites what the concordance said and nothing the other tables said"
    );
    parses_whole(&claims);
}

/// §4's cell-escape law: `\|` is a pipe in a cell and never a delimiter,
/// so the cell's value carries the `|` and the row's other cells are
/// read at the columns their author wrote them in.
///
/// Split on every `|` regardless, this row still states three cells, so
/// it still reads as a citation and still lifts onto verse 1 — carrying
/// the wreck of two cells cut in the wrong places, reported by nothing.
/// That is why the assertions are on the values and not on the count.
#[test]
fn an_escaped_pipe_is_a_cell_value_and_never_a_delimiter() {
    let text = "# T\n\n1. One.\n\n2. Two.\n\n## Concordance\n\n\
                | Verses | Canon source | Anchors |\n|---|---|---|\n\
                | 1 | `atlas/a\\|b.ttl` | `alpha` |\n\
                | 2 | `atlas/c.ttl` \\| `atlas/d.ttl` | `beta` (west \\| east) |\n";
    let document = model(text, &v1());
    assert!(
        document.malformed_rows().is_empty(),
        "both rows read as citations, cut at the columns their author wrote"
    );
    let rows = document.citations();
    assert_eq!(rows.len(), 2);

    assert_eq!(rows[0].verses(), (1, 1));
    assert_eq!(
        rows[0].sources(),
        ["atlas/a|b.ttl"],
        "the cell's value carries the literal pipe the author escaped"
    );
    assert_eq!(rows[0].anchors(), ["alpha"], "and the anchor is intact");

    assert_eq!(rows[1].verses(), (2, 2));
    assert_eq!(
        rows[1].sources(),
        ["atlas/c.ttl", "atlas/d.ttl"],
        "a pipe dividing two paths inside one cell keeps both of them in that cell"
    );
    assert_eq!(
        rows[1].anchors(),
        ["beta"],
        "prose carrying a pipe beside a backticked anchor lifts nothing, as prose"
    );

    // And the path travels into the graph with its pipe, on the citation
    // node of the row that wrote it.
    let claims = slice_of(text, GUIDE_ID, &v1()).expect("slices");
    let one = *units(&claims)
        .iter()
        .find(|u| integer(u, &v().verse) == Some(1))
        .expect("verse 1");
    let node = citation_nodes(one);
    assert_eq!(
        citation_objects(one, &node[0], &v().canon_source),
        vec![format!("\"atlas/a|b.ttl\"^^<{}>", v().dt_path)]
    );
    parses_whole(&claims);
}

/// The rest of the escape law, stated by the edges it has to answer:
/// `\\` is a literal backslash and the `|` after it still delimits; a `\`
/// before anything else is content and keeps its backslash; and a `\` at
/// the end of a row's line escapes nothing, so it becomes content of a
/// trailing cell rather than swallowing the row's closing delimiter.
#[test]
fn the_escape_law_answers_an_escaped_backslash_and_a_trailing_one() {
    let text = "# T\n\n1. One.\n\n2. Two.\n\n3. Three.\n\n## Concordance\n\n\
                | Verses | Canon source | Anchors |\n|---|---|---|\n\
                | 1 | `atlas\\\\a.ttl` | `alpha` |\n\
                | 2 | `atlas/b\\nc.ttl` | `beta` |\n\
                | 3 | `atlas/d.ttl` | `gamma` |\\\n";
    let document = model(text, &v1());
    assert!(
        document.malformed_rows().is_empty(),
        "every one of the three reads as a citation"
    );
    let rows = document.citations();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows[0].sources(),
        ["atlas\\a.ttl"],
        "`\\\\` is one literal backslash, and the pipe after it still delimited"
    );
    assert_eq!(rows[0].anchors(), ["alpha"]);
    assert_eq!(
        rows[1].sources(),
        ["atlas/b\\nc.ttl"],
        "a backslash before anything else is content, backslash and all"
    );
    assert_eq!(rows[2].verses(), (3, 3));
    assert_eq!(
        rows[2].sources(),
        ["atlas/d.ttl"],
        "a trailing backslash escapes nothing and takes no cell with it"
    );
    assert_eq!(rows[2].anchors(), ["gamma"]);
    assert_eq!(
        rows[2].lifted(),
        [(3, 2)],
        "so the row still lifts onto the verse it names"
    );
    parses_whole(&slice_of(text, GUIDE_ID, &v1()).expect("slices"));
}

// --- the interval relations ----------------------------------------------

#[test]
fn the_thirteen_allen_relations_name_every_way_two_spans_can_lie() {
    let a = Span::new(4, 8);
    let table = [
        (Span::new(10, 12), SpanRelation::Before),
        (Span::new(8, 12), SpanRelation::Meets),
        (Span::new(6, 12), SpanRelation::Overlaps),
        (Span::new(4, 12), SpanRelation::Starts),
        (Span::new(2, 12), SpanRelation::During),
        (Span::new(2, 8), SpanRelation::Finishes),
        (Span::new(4, 8), SpanRelation::Equals),
        (Span::new(6, 8), SpanRelation::FinishedBy),
        (Span::new(5, 7), SpanRelation::Contains),
        (Span::new(4, 6), SpanRelation::StartedBy),
        (Span::new(2, 6), SpanRelation::OverlappedBy),
        (Span::new(2, 4), SpanRelation::MetBy),
        (Span::new(0, 2), SpanRelation::After),
    ];
    let mut seen = BTreeSet::new();
    for (b, expected) in table {
        assert_eq!(span_relation(a, b), expected, "{a:?} against {b:?}");
        assert_eq!(a.relation(b), expected);
        assert_eq!(b.relation(a), expected.inverse(), "and back again");
        assert_eq!(
            expected.is_containment(),
            a.contains_span(b),
            "containment is the four relations that hold b inside a"
        );
        assert_eq!(
            a.intersects(b),
            !matches!(
                expected,
                SpanRelation::Before
                    | SpanRelation::Meets
                    | SpanRelation::MetBy
                    | SpanRelation::After
            ),
            "touching is not overlapping"
        );
        seen.insert(format!("{expected:?}"));
    }
    assert_eq!(seen.len(), 13, "every relation is exercised");
    // The degenerate span is a caller's own, and it is answered as a
    // point rather than refused.
    assert!(Span::new(3, 3).is_empty());
    assert_eq!(Span::new(3, 3).len(), 0);
    assert_eq!(
        span_relation(Span::new(3, 3), Span::new(3, 8)),
        SpanRelation::Meets
    );
    assert!(!Span::new(3, 3).intersects(Span::new(0, 8)));
}

// --- the designated namespace --------------------------------------------

#[test]
fn the_standard_vocabulary_is_the_designated_namespace_term_for_term_and_slices_a_document() {
    let standard = Vocabulary::standard().expect("the designated namespace derives one");
    standard.validate().expect("and it validates");
    assert_eq!(STANDARD_NAMESPACE, "https://w3id.org/purrdf/markdown#");
    assert_eq!(
        standard,
        Vocabulary::under(STANDARD_NAMESPACE).expect("a vocabulary"),
        "standard() is under(the designated namespace), field for field"
    );
    // Spot-checked term by term, so a renamed local name is caught here
    // and not only by the equality above.
    assert_eq!(standard.node_base, STANDARD_NAMESPACE);
    assert_eq!(
        standard.document_class,
        format!("{STANDARD_NAMESPACE}Document")
    );
    assert_eq!(standard.unit_class, format!("{STANDARD_NAMESPACE}Unit"));
    assert_eq!(standard.text, format!("{STANDARD_NAMESPACE}text"));
    assert_eq!(standard.cites, format!("{STANDARD_NAMESPACE}cites"));
    assert_eq!(
        standard.dt_lineage,
        format!("{STANDARD_NAMESPACE}lineagePath")
    );
    assert_eq!(
        standard.scalar_start,
        format!("{STANDARD_NAMESPACE}scalarStart")
    );
    assert_eq!(
        standard.scalar_end,
        format!("{STANDARD_NAMESPACE}scalarEnd")
    );
    assert_eq!(
        standard.content_digest,
        format!("{STANDARD_NAMESPACE}contentDigest")
    );
    // The predicate and the datatype of the same literal are two terms,
    // because the namespace is meant to be described: `heading` carries
    // a section's heading text and `headingText` types it.
    assert_eq!(standard.heading, format!("{STANDARD_NAMESPACE}heading"));
    assert_eq!(
        standard.dt_heading,
        format!("{STANDARD_NAMESPACE}headingText")
    );
    assert_ne!(standard.heading, standard.dt_heading);
    assert_eq!(standard.lineage, format!("{STANDARD_NAMESPACE}lineage"));
    assert_ne!(standard.lineage, standard.dt_lineage);
    // A document sliced under it parses as Turtle, whole.
    let profile = Profile::new("designated-md-v1", 1, standard);
    let claims = slice_of(GUIDE, GUIDE_ID, &profile).expect("slices");
    parses_whole(&claims);
    assert!(
        claims
            .iter()
            .all(|c| c.turtle.contains(STANDARD_NAMESPACE) || c.kind == ClaimKind::Document)
    );
    assert!(
        units(&claims)[0]
            .subject
            .starts_with(&format!("{STANDARD_NAMESPACE}unit:sha256:"))
    );
    // And it is its own profile: a vocabulary is inside the identity.
    assert_ne!(profile.contract_id().to_hex(), v1().contract_id().to_hex());
    assert!(
        String::from_utf8(profile.emission_bytes())
            .expect("utf8")
            .contains(&format!("vocabulary {STANDARD_NAMESPACE}\n"))
    );
}

/// The IRIs a run of claims writes, gathered by the **position** they
/// are written in: `class` for the object of an `rdf:type` line,
/// `predicate` for the predicate of a line — inside a reified triple
/// term as well as outside one — and `datatype` for the datatype IRI of
/// a typed literal.
///
/// It reads the rendered lines rather than the vocabulary, because the
/// question is about the bytes a consumer receives. Every line is one
/// N-Triples triple written by the crate's own writer, so the subject
/// and the predicate are the first two space-separated tokens and an
/// IRI never carries a space.
fn iris_by_role(claims: &[Claim]) -> BTreeMap<&'static str, BTreeSet<String>> {
    let bare = |token: &str| {
        token
            .trim_start_matches('<')
            .trim_end_matches('>')
            .to_owned()
    };
    let mut written: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    for line in claims.iter().flat_map(|c| c.turtle.lines()) {
        let tokens: Vec<&str> = line.split(' ').collect();
        let predicate = bare(tokens[1]);
        if predicate == purrdf_markdown::RDF_TYPE {
            written.entry("class").or_default().insert(bare(tokens[2]));
        }
        written.entry("predicate").or_default().insert(predicate);
        if tokens[2] == "<<(" {
            // The predicate of the triple term a citation node reifies.
            written
                .entry("predicate")
                .or_default()
                .insert(bare(tokens[4]));
        }
        let mut rest = line;
        while let Some((_, tail)) = rest.split_once("^^<") {
            let (datatype, tail) = tail.split_once('>').expect("a datatype IRI closes");
            written
                .entry("datatype")
                .or_default()
                .insert(datatype.to_owned());
            rest = tail;
        }
    }
    written
}

/// No IRI the slicer writes is written in two positions: the classes,
/// the predicates and the datatypes of an emitted graph are three
/// disjoint sets of IRIs.
///
/// This is the property that lets the designated namespace be
/// *described*: no OWL or SHACL document can call one IRI an
/// `owl:DatatypeProperty` and an `rdfs:Datatype` at once, so a namespace
/// offered for interchange must never ask it to. It is asked of the
/// emitted bytes and over whatever roles the scan finds, rather than of
/// the two terms that once doubled up, so a term added later that reuses
/// a name fails here.
#[test]
fn no_iri_the_slicer_writes_is_both_a_predicate_and_a_datatype() {
    assert!(
        !GUIDE.contains("^^<"),
        "the fixture writes no `^^<` of its own, so every one in the output is a datatype"
    );
    let designated = Profile::new(
        "designated-md-v1",
        1,
        Vocabulary::standard().expect("the designated namespace derives one"),
    );
    let mut designated_canon = designated.clone();
    designated_canon.canon_base = Some(CANON_BASE.to_owned());
    for profile in [
        &designated,
        &designated_canon,
        &v1(),
        &under_canon(CANON_BASE),
        &small(),
    ] {
        let written = iris_by_role(&slice(GUIDE, profile));
        assert_eq!(
            written.keys().copied().collect::<Vec<_>>(),
            ["class", "datatype", "predicate"],
            "the guide exercises all three positions"
        );
        for (left_role, left) in &written {
            for (right_role, right) in &written {
                if left_role >= right_role {
                    continue;
                }
                let shared: Vec<&String> = left.intersection(right).collect();
                assert!(
                    shared.is_empty(),
                    "{left_role} and {right_role} share {shared:?} under {}",
                    profile.name
                );
            }
        }
        // The two that used to double up, named so the vector says what
        // it is guarding: the predicate and the datatype of the same
        // literal are two IRIs.
        let v = &profile.vocabulary;
        assert!(written["predicate"].contains(&v.heading));
        assert!(written["datatype"].contains(&v.dt_heading));
        assert!(written["predicate"].contains(&v.lineage));
        assert!(written["datatype"].contains(&v.dt_lineage));
    }
    // The neighbouring case, so the property above is not vacuous: a
    // caller is still sovereign over its own terms and may point a
    // datatype field at a predicate's IRI — this crate derives no such
    // vocabulary, and the scan sees it the moment one is emitted.
    let mut dual = v1();
    dual.vocabulary.dt_heading.clone_from(&v().heading);
    dual.vocabulary.dt_lineage.clone_from(&v().lineage);
    let written = iris_by_role(&slice(GUIDE, &dual));
    assert_eq!(
        written["predicate"]
            .intersection(&written["datatype"])
            .cloned()
            .collect::<Vec<String>>(),
        vec![v().heading, v().lineage],
        "the two roles a derived vocabulary no longer puts on one IRI"
    );
}

/// The heading and lineage literals of the designated namespace carry
/// their own datatype IRIs — `headingText` and `lineagePath`, neither of
/// which is a predicate — and the document they sit in still parses and
/// canonicalizes whole.
#[test]
fn the_designated_heading_and_lineage_literals_carry_their_own_datatypes() {
    let standard = Vocabulary::standard().expect("the designated namespace derives one");
    let profile = Profile::new("designated-md-v1", 1, standard.clone());
    let claims = slice(GUIDE, &profile);
    let datatype_of =
        |claim: &Claim, predicate: &str| object(claim, predicate).map(|term| typed_parts(&term).1);
    assert_eq!(
        datatype_of(&claims[0], &standard.title).as_deref(),
        Some(&*format!("{STANDARD_NAMESPACE}headingText")),
        "the document's title is a heading text"
    );
    let section = sections(&claims)[0];
    assert_eq!(
        object(section, &standard.heading).as_deref(),
        Some(&*format!("\"{TITLE}\"")),
        "a heading's words are the section's one plain literal"
    );
    assert_eq!(
        object(section, &standard.verbatim).map(|term| typed_parts(&term).1),
        Some(format!("{STANDARD_NAMESPACE}verbatimText")),
        "and its line's bytes stay typed beside them"
    );
    let lineages: Vec<(String, String)> = units(&claims)
        .iter()
        .filter_map(|u| object(u, &standard.lineage))
        .map(|term| typed_parts(&term))
        .collect();
    assert!(!lineages.is_empty(), "the guide's units sit under headings");
    for (lexical, datatype) in &lineages {
        assert_eq!(datatype, &format!("{STANDARD_NAMESPACE}lineagePath"));
        assert_ne!(lexical, "", "and a stated lineage is never the empty one");
    }
    assert!(
        lineages.iter().any(|(lexical, _)| lexical.contains(" > ")),
        "and a lineage is a path through the heading stack"
    );
    // Neither datatype is a predicate of the namespace.
    for datatype in [&standard.dt_heading, &standard.dt_lineage] {
        assert!(
            !claims
                .iter()
                .flat_map(|c| c.turtle.lines())
                .any(|line| line.split(' ').nth(1) == Some(&*format!("<{datatype}>"))),
            "{datatype} is written in no predicate position"
        );
    }
    // And the graph is still a graph.
    parses_whole(&claims);
    for claim in &claims {
        let dataset = parse_dataset(claim.turtle.as_bytes(), "text/turtle", None)
            .unwrap_or_else(|e| panic!("{}: {e:?}", claim.subject));
        let canonical = try_canonicalize_with(&dataset, CanonHash::Sha256)
            .unwrap_or_else(|e| panic!("{}: {e:?}", claim.subject));
        assert_eq!(
            canonical.nquads.lines().count(),
            claim.turtle.lines().count()
        );
    }
}

// --- the PURREMB bridge ----------------------------------------------------

/// The document target and chunking id a `.purremb` producer would mint
/// for the guide: the two ids every chunk of it is addressed under.
///
/// The chunking id is derived the way a family derives it — over the
/// canonical bytes of the profile's chunking stage — and never from the
/// profile's law id, which is the confusion the bridge exists to
/// prevent.
fn purremb_ids(profile: &Profile) -> (TargetId, ChunkingContractId) {
    let corpus = CorpusTarget {
        manifest_digest: ContentDigest::of(GUIDE.as_bytes()),
        manifest_media_type: "application/example".to_owned(),
        logical_id_digest: ContentDigest::of(GUIDE_ID.as_bytes()),
    }
    .into_target(false)
    .expect("a corpus target");
    let document = DocumentTarget::from_content(
        corpus.id,
        ContentDigest::of(GUIDE_ID.as_bytes()),
        "text/markdown;charset=utf-8",
        GUIDE.as_bytes(),
    )
    .expect("a document subject")
    .into_target(false)
    .expect("a document target");
    let stage = profile
        .purremb_chunking_stage()
        .canonical_bytes()
        .expect("the stage encodes");
    (document.id, derive_chunking_contract_id(&stage))
}

#[test]
fn the_profiles_three_identities_are_stable_and_no_two_of_them_are_ever_each_other() {
    let profile = v1();
    let stage = profile.purremb_chunking_stage();
    assert_eq!(
        stage,
        profile.purremb_chunking_stage(),
        "the stage is a pure function of the profile"
    );
    let AppliedStage::Applied(implementation) = &stage else {
        panic!("this crate always chunks, so the stage is always applied");
    };
    assert_eq!(implementation.identifier, STANDARD_NAMESPACE);
    assert_eq!(
        implementation.parameter_encoding,
        PURREMB_PARAMETER_ENCODING
    );
    // A chunking stage carries the chunking law, and only it: that is
    // what makes the label honest rather than merely present.
    assert_eq!(
        implementation.parameters,
        profile.chunking_bytes(),
        "the chunking law is the stage's parameters"
    );
    assert_eq!(
        implementation.digest,
        ContentDigest::of(&profile.chunking_bytes())
    );

    // The id a family derives from that stage is stable across calls.
    let id = |profile: &Profile| {
        derive_chunking_contract_id(
            &profile
                .purremb_chunking_stage()
                .canonical_bytes()
                .expect("the stage encodes"),
        )
    };
    assert_eq!(id(&profile), id(&profile));

    // Three ids, and no two of them are one. The family's is TLV-framed
    // where this crate's two are line-oriented, and this crate's two are
    // taken over preimages of different lengths.
    let three: BTreeSet<String> = [
        id(&profile).to_hex(),
        profile.chunking_id().to_hex(),
        profile.contract_id().to_hex(),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        three.len(),
        3,
        "a stage id, a chunking id and a law id are three identities, never fewer"
    );

    // A changed bound is a changed cut, so it moves all three.
    assert_ne!(id(&profile), id(&small()));
    assert_ne!(profile.chunking_id(), small().chunking_id());
    assert_ne!(profile.contract_id(), small().contract_id());

    // A changed term is not a changed cut, so it moves the law id and
    // leaves a consumer's chunk addressing exactly where it was.
    let mut retermed = profile.clone();
    retermed.vocabulary.cites = "https://example.org/other/cites".to_owned();
    assert_eq!(
        id(&retermed),
        id(&profile),
        "a renamed predicate costs no consumer a re-embedding"
    );
    assert_eq!(retermed.chunking_id(), profile.chunking_id());
    assert_ne!(retermed.contract_id(), profile.contract_id());
}

#[test]
fn a_units_chunk_target_verifies_against_the_documents_own_bytes() {
    let profile = v1();
    let (document_id, chunking_id) = purremb_ids(&profile);
    let document = model(GUIDE, &profile);
    let mut multi_byte = 0;
    for unit in document.units() {
        let target = unit.text_chunk_target(document_id, chunking_id);
        // Every field is the model's, handed over unchanged.
        assert_eq!(target.byte_start, unit.span().start);
        assert_eq!(target.byte_end, unit.span().end);
        assert_eq!(target.scalar_start, unit.scalar_span().start);
        assert_eq!(target.scalar_end, unit.scalar_span().end);
        assert_eq!(target.content_digest, unit.digest());
        // And the kernel's own law accepts it against the source.
        target
            .verify_document(GUIDE.as_bytes())
            .expect("the kernel's verification law accepts the unit");
        // The target is addressable: it mints a canonical chunk subject.
        target.into_target(true).expect("a chunk target mints");
        if unit.quote().chars().count() < unit.quote().len() {
            multi_byte += 1;
        }
    }
    assert!(
        multi_byte > 0,
        "the fixture carries units of multi-byte scalars, and they verified too"
    );
}

#[test]
fn verify_unit_accepts_the_bytes_a_unit_was_minted_over() {
    let profile = v1();
    let (document_id, chunking_id) = purremb_ids(&profile);
    let document = model(GUIDE, &profile);
    for unit in document.units() {
        verify_unit(GUIDE.as_bytes(), unit, document_id, chunking_id)
            .expect("the unit answers for the bytes it was minted over");
    }
    // The ids travel through and cannot decide the answer: another pair
    // accepts exactly the same bytes.
    let other = (
        TargetId::from_raw([9; 32]),
        ChunkingContractId::from_raw([9; 32]),
    );
    for unit in document.units() {
        verify_unit(GUIDE.as_bytes(), unit, other.0, other.1)
            .expect("the ids are carried, not consulted");
    }
}

#[test]
fn verify_unit_refuses_a_tampered_byte_and_the_kernel_refuses_it_too() {
    let profile = v1();
    let (document_id, chunking_id) = purremb_ids(&profile);
    let document = model(GUIDE, &profile);
    let unit = document
        .units()
        .iter()
        .find(|u| u.quote().chars().count() < u.quote().len())
        .expect("a unit of multi-byte scalars");

    // One ASCII letter inside the unit, swapped for another: the same
    // length, the same boundaries, different bytes.
    let (start, end) = (unit.span().start as usize, unit.span().end as usize);
    let at = start
        + GUIDE.as_bytes()[start..end]
            .iter()
            .position(u8::is_ascii_alphabetic)
            .expect("an ASCII letter inside the unit");
    let mut tampered = GUIDE.as_bytes().to_vec();
    tampered[at] = if tampered[at] == b'x' { b'y' } else { b'x' };
    assert_ne!(tampered, GUIDE.as_bytes(), "a byte moved");

    let refusal = verify_unit(&tampered, unit, document_id, chunking_id)
        .expect_err("the digest no longer answers for these bytes");
    assert_eq!(
        refusal,
        MarkdownError::TamperedUnit {
            span: (unit.span().start, unit.span().end),
            cause: EmbeddingError::ContentMismatch("chunk coordinates or digest"),
        }
    );
    assert!(refusal.to_string().contains("does not answer"));

    // The kernel refuses the same bytes for the same reason: one law.
    let kernel = unit
        .text_chunk_target(document_id, chunking_id)
        .verify_document(&tampered)
        .expect_err("the kernel's own law refuses it");
    assert_eq!(
        MarkdownError::TamperedUnit {
            span: (unit.span().start, unit.span().end),
            cause: kernel,
        },
        refusal
    );

    // A refusal is a claim: the untouched bytes still verify.
    verify_unit(GUIDE.as_bytes(), unit, document_id, chunking_id)
        .expect("the neighbouring valid case is still valid");
}

#[test]
fn verify_unit_refuses_a_span_that_is_not_inside_the_bytes() {
    let profile = v1();
    let (document_id, chunking_id) = purremb_ids(&profile);
    let document = model(GUIDE, &profile);
    let unit = document.units().last().expect("a unit");
    let (start, end) = (unit.span().start as usize, unit.span().end as usize);

    // Truncated to the unit's own start, the span runs off the end.
    let refusal = verify_unit(&GUIDE.as_bytes()[..start], unit, document_id, chunking_id)
        .expect_err("the span is not inside these bytes");
    let MarkdownError::TamperedUnit { span, cause } = &refusal else {
        panic!("an out-of-bounds span is a tampered unit: {refusal:?}");
    };
    assert_eq!(*span, (unit.span().start, unit.span().end));
    assert!(
        matches!(cause, EmbeddingError::InvalidSpan { .. }),
        "the kernel names the span: {cause:?}"
    );

    // Truncated to the unit's own end it is inside them, and verifies:
    // the bound is exactly where it is claimed to be.
    verify_unit(&GUIDE.as_bytes()[..end], unit, document_id, chunking_id)
        .expect("a document that ends where the unit ends still carries it");
}

#[test]
fn scalar_and_byte_offsets_convert_into_each_other_across_the_whole_guide() {
    let document = model(GUIDE, &v1());
    let scalars = GUIDE.chars().count() as u64;

    // Every unit's stored scalar span is what the conversion answers.
    for unit in document.units() {
        assert_eq!(
            document.byte_to_scalar(unit.span().start),
            Some(unit.scalar_span().start)
        );
        assert_eq!(
            document.byte_to_scalar(unit.span().end),
            Some(unit.scalar_span().end)
        );
        assert_eq!(
            document.scalar_to_byte(unit.scalar_span().start),
            Some(unit.span().start)
        );
        assert_eq!(
            document.scalar_to_byte(unit.scalar_span().end),
            Some(unit.span().end)
        );
    }

    // Both ends of the document, and one past each.
    assert_eq!(document.byte_to_scalar(0), Some(0));
    assert_eq!(document.scalar_to_byte(0), Some(0));
    assert_eq!(
        document.byte_to_scalar(document.byte_length()),
        Some(scalars),
        "the document's end is a boundary and is its scalar count"
    );
    assert_eq!(
        document.scalar_to_byte(scalars),
        Some(document.byte_length())
    );
    assert_eq!(document.byte_to_scalar(document.byte_length() + 1), None);
    assert_eq!(document.scalar_to_byte(scalars + 1), None);

    // Inside a multi-byte scalar there is no position to answer with.
    let (at, wide) = GUIDE
        .char_indices()
        .find(|(_, c)| c.len_utf8() > 1)
        .expect("the fixture carries a multi-byte scalar");
    assert!(document.byte_to_scalar(at as u64).is_some(), "its start is");
    for inside in 1..wide.len_utf8() as u64 {
        assert_eq!(
            document.byte_to_scalar(at as u64 + inside),
            None,
            "byte {inside} of a {}-byte scalar names no position",
            wide.len_utf8()
        );
    }
}

// --- §2: the dialect grammar, executed rather than quoted -----------------

/// Every section a text states: its level, its heading text, and whether
/// a movement marker opened it.
fn section_shape(text: &str) -> Vec<(u32, String, bool)> {
    model(text, &v1())
        .sections()
        .iter()
        .map(|s| (s.level(), s.heading().to_owned(), s.is_movement()))
        .collect()
}

/// Every unit a text states, as its verse number and its verbatim quote.
fn unit_shape(text: &str) -> Vec<(Option<u64>, String)> {
    model(text, &v1())
        .units()
        .iter()
        .map(|u| (u.verse(), u.quote().to_owned()))
        .collect()
}

/// The verbatim quote of every unit a text states.
fn unit_quotes(text: &str) -> Vec<String> {
    unit_shape(text).into_iter().map(|(_, q)| q).collect()
}

/// §2 of the specification, asked of the slicer instead of read: every
/// recognition rule the dialect grammar states, executed against real
/// slicing, each beside the **neighbouring** line it must not read.
///
/// A false grammar clause is the costliest kind of false clause, because
/// a wrong reading here still produces output: a heading read as prose
/// is a paragraph, a verse read as prose is a paragraph, and nothing
/// refuses, nothing is empty, and no consumer learns. So each rule is
/// asked in both directions — the line the clause admits, and the
/// nearest line it does not.
#[test]
fn the_dialect_grammar_of_section_two_recognizes_exactly_what_it_states() {
    // §2.1 — one to six hashes and then a space open a heading whose
    // level is the number of hashes, and the heading line is the content
    // of no unit.
    for hashes in 1..=6_usize {
        let text = format!("{} T\n", "#".repeat(hashes));
        assert_eq!(
            section_shape(&text),
            vec![(hashes as u32, "T".to_owned(), false)],
            "{hashes} hashes open a heading at level {hashes}"
        );
        assert!(unit_quotes(&text).is_empty(), "a heading is no unit");
    }
    // Seven or more open none at all, and the line is ordinary prose.
    for hashes in 7..=9_usize {
        let text = format!("{} T\n", "#".repeat(hashes));
        assert!(section_shape(&text).is_empty(), "{hashes} hashes");
        assert_eq!(
            unit_quotes(&text),
            vec![format!("{} T", "#".repeat(hashes))],
            "and the line falls into a paragraph, marker and all"
        );
    }
    // Hashes running straight into text open none either: the space (or
    // the tab) is the mark.
    assert!(
        section_shape("#T\n").is_empty(),
        "hashes running into text open no heading"
    );
    assert_eq!(unit_quotes("#T\n"), vec!["#T".to_owned()]);
    // The title is the rest of the marker text, trimmed, with trailing
    // hashes removed and trimmed again — and an empty title is a title.
    assert_eq!(
        section_shape("##   Marrow   ##  \n"),
        vec![(2, "Marrow".to_owned(), false)]
    );
    assert_eq!(section_shape("## \n"), vec![(2, String::new(), false)]);

    // §2 — the terminating newline is part of no line, so a section's
    // heading span is its opening line alone, which is the span §5 mints
    // its identity over.
    let text = "## Marrow\n\n1. One.\n";
    let document = model(text, &v1());
    assert_eq!(document.sections()[0].heading_span(), Span::new(0, 9));
    assert_eq!(&text[0..9], "## Marrow");

    // §2.1 — a movement marker is U+2042 and then a space; its name is
    // the rest, trimmed, with one surrounding `*` or `_` pair removed;
    // and it sits one level under the nearest *heading*, so consecutive
    // movements are siblings rather than a descending chain.
    let movements = "# H\n\n\u{2042} *a*\n\n\u{2042} _b_\n";
    assert_eq!(
        section_shape(movements),
        vec![
            (1, "H".to_owned(), false),
            (2, "a".to_owned(), true),
            (2, "b".to_owned(), true),
        ]
    );
    let walked = model(movements, &v1());
    assert_eq!(walked.sections()[1].parent(), Some(0));
    assert_eq!(
        walked.sections()[2].parent(),
        Some(0),
        "a movement is no parent of the movement after it"
    );
    // Without the space it opens nothing, and the marker stays in the
    // text where the author wrote it.
    assert!(
        section_shape("\u{2042}*a*\n").is_empty(),
        "the space after the marker is the mark"
    );
    assert_eq!(unit_quotes("\u{2042}*a*\n"), vec!["\u{2042}*a*".to_owned()]);

    // §2.2 — digits, a `.` and a space open a verse; the same digits
    // without the space open none, and the line is prose.
    assert_eq!(
        unit_shape("12. Hear this.\n"),
        vec![(Some(12), "12. Hear this.".to_owned())]
    );
    assert_eq!(
        unit_shape("12.Hear this.\n"),
        vec![(None, "12.Hear this.".to_owned())]
    );

    // §2.2 — a blank line, a horizontal rule and a table row each close
    // the open unit and are part of no unit.
    for separator in ["", "---", "***", "-----", "  ---  ", "| a |"] {
        assert_eq!(
            unit_quotes(&format!("one\n{separator}\ntwo\n")),
            vec!["one".to_owned(), "two".to_owned()],
            "{separator:?} closes the unit and joins none"
        );
    }
    // And the neighbours none of the three rules name run straight on: a
    // rule is three or more characters, all `-` or all `*`.
    for content in ["--", "**", "-*-", "- - -", "----x"] {
        assert_eq!(
            unit_quotes(&format!("one\n{content}\ntwo\n")),
            vec![format!("one\n{content}\ntwo")],
            "{content:?} is ordinary content"
        );
    }

    // §2.1 — the document's title is the first heading that is not a
    // movement, so a movement before it is no title.
    assert_eq!(
        model("\u{2042} *m*\n\n# H\n\n## Deeper\n", &v1()).title(),
        Some("H")
    );
}

// --- §4.3: the anchor lift, executed --------------------------------------

/// A document whose only concordance row names a verse this document
/// does **not** carry: the row §4.2 admits, and whose anchors §4.3 walks
/// all the same.
fn cited_elsewhere(anchor: &str) -> String {
    format!(
        "# T\n\n1. One line.\n\n## Concordance\n\n\
         | Verses | Canon source | Anchors |\n| --- | --- | --- |\n\
         | 9 | `atlas/x.logic.ttl` | `{anchor}` |\n"
    )
}

/// §4.3 executed: the lift is **concatenation** and never RFC-3986
/// resolution, its containment test is relativization, and it is asked
/// of every anchor of the table rather than of the anchors that happen
/// to lift here.
///
/// The first of those is a clause a reading of the code cannot settle,
/// because under most bases the two answers coincide. Under the
/// fragment base the goldens are minted under they do not: resolution
/// throws the fragment away and lands somewhere else entirely, so the
/// two answers are written out side by side here and the graph is held
/// to the one the law names.
#[test]
fn the_anchor_lift_concatenates_never_resolves_and_walks_every_row_of_the_table() {
    // Concatenation, stated as the IRI the graph carries.
    let claims = slice_of(&cited("tide-line"), GUIDE_ID, &under_canon(CANON_BASE)).expect("slices");
    let minted = format!("{CANON_BASE}tide-line");
    assert_eq!(verse_one_cites(&claims), vec![format!("<{minted}>")]);
    assert_eq!(minted, "https://example.org/canon#tide-line");

    // Resolution is the other answer, and it is not this one: RFC-3986
    // dissolves the base's `#` and the anchor lands outside the canon
    // altogether.
    let base = BaseIri::parse(CANON_BASE).expect("an absolute base");
    let resolved = base.resolve("tide-line").expect("it resolves");
    assert_eq!(resolved.as_str(), "https://example.org/tide-line");
    assert_ne!(resolved.as_str(), minted, "concatenation, never resolution");

    // And the containment test is relativization, which is the only rule
    // that answers for a fragment base at all: the minted IRI has a
    // relative spelling against it, and that is what admits it.
    let parsed = parse_iri(&minted).expect("an IRI");
    assert_eq!(base.relativize(&parsed), Some("#tide-line".to_owned()));

    // Every anchor of the table is walked, and not only those whose
    // verses this document carries: verse 9 is nowhere in this document,
    // and its row's anchor is refused where it is written.
    for anchor in ["two words", "../x"] {
        assert_eq!(
            slice_of(
                &cited_elsewhere(anchor),
                GUIDE_ID,
                &under_canon(CANON_PATH_BASE)
            ),
            Err(MarkdownError::InvalidAnchor {
                anchor: anchor.to_owned(),
                verses: (9, 9),
            }),
            "a row that lifts nothing here still names the anchor it wrote"
        );
    }
    // The neighbouring row — the same verse this document does not
    // carry, with a lawful anchor — is no refusal at all. It lifts
    // nothing, it is reported as data, and nothing cites it.
    let lawful = cited_elsewhere("far-shore");
    let document = model(&lawful, &under_canon(CANON_PATH_BASE));
    let unmatched: Vec<(u64, u64)> = document
        .unmatched_citations()
        .map(purrdf_markdown::Citation::verses)
        .collect();
    assert_eq!(unmatched, vec![(9, 9)], "reported, never refused");
    let claims = render(&document);
    assert!(
        claims
            .iter()
            .all(|claim| !claim.turtle.contains(&v().cites)),
        "a row that lifted nothing states no edge"
    );
    parses_whole(&claims);
}

// --- §5: the preimage, rebuilt from the specification's own words ---------

/// A length-prefixed preimage exactly as §5 states one: each field
/// preceded by its length as eight bytes little-endian, in the order the
/// section lists them.
///
/// It is spelled out here rather than reached for in the crate. A second
/// implementation has only these words, and an identity the crate agrees
/// with *itself* about is no evidence that the words say what the crate
/// does.
fn spec_preimage(fields: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for field in fields {
        out.extend_from_slice(&(field.len() as u64).to_le_bytes());
        out.extend_from_slice(field);
    }
    out
}

/// §5's node IRI over a content digest already taken:
/// `<node base><kind>:<alg>:<hex>`, the hex being the digest of the
/// seven-field preimage — the kind, the source id, the profile's
/// **chunking** id, the span's two endpoints as eight bytes
/// little-endian each, the digest algorithm tag, and the content digest.
fn spec_node_iri(
    kind: &str,
    contract: &ChunkingContractId,
    span: Span,
    content: &ContentDigest,
) -> String {
    let preimage = spec_preimage(&[
        kind.as_bytes(),
        GUIDE_ID.as_bytes(),
        contract.as_bytes(),
        &span.start.to_le_bytes(),
        &span.end.to_le_bytes(),
        DIGEST_ALGORITHM.as_bytes(),
        content.as_bytes(),
    ]);
    format!(
        "{SLICE_BASE}{kind}:{DIGEST_ALGORITHM}:{}",
        ContentDigest::of(&preimage).to_hex()
    )
}

/// §5 executed: the preimage the specification writes out, rebuilt field
/// by field from its words, mints the very IRIs the slicer emits for a
/// real document — every section, every unit, and a citation.
///
/// This is the vector a second implementation is really held to. The
/// crate's own [`unit_iri`] and `citation_iri` are pinned against the
/// projection elsewhere; both of those would still agree if the *fields*
/// were in another order, or a length prefix were dropped, or the wrong
/// profile id were in position three. Nothing but a preimage built from
/// the specification can tell.
#[test]
fn the_preimage_section_five_states_mints_the_very_ids_the_slicer_emits() {
    let document = model(GUIDE, &v1());
    let claims = slice(GUIDE, &v1());
    let contract = v1().chunking_id();
    let bytes = |span: Span| &GUIDE.as_bytes()[span.start as usize..span.end as usize];

    // A section: the kind `section`, over its *heading span* and that
    // span's bytes.
    let emitted = sections(&claims);
    assert_eq!(emitted.len(), document.sections().len());
    assert!(emitted.len() > 4, "the guide has sections to speak of");
    for (claim, section) in emitted.iter().zip(document.sections()) {
        let span = section.heading_span();
        assert_eq!(
            claim.subject,
            spec_node_iri("section", &contract, span, &ContentDigest::of(bytes(span)))
        );
    }

    // A unit: the kind `unit`, over its own span and its own bytes — and
    // the digest the model carries is the digest of those bytes.
    let emitted = units(&claims);
    assert_eq!(emitted.len(), document.units().len());
    assert!(emitted.len() > 4, "and units to speak of");
    for (claim, unit) in emitted.iter().zip(document.units()) {
        assert_eq!(unit.digest(), ContentDigest::of(bytes(unit.span())));
        assert_eq!(
            claim.subject,
            spec_node_iri("unit", &contract, unit.span(), &unit.digest())
        );
    }

    // A citation: the kind `citation`, over the row's own line span,
    // with the content digest taken over the row's line, the unit's IRI,
    // and the triple terms the node reifies, in the row's own order.
    let row = &document.citations()[0];
    let (verse, unit) = row.lifted()[0];
    let claim = emitted[unit];
    assert_eq!(document.units()[unit].verse(), Some(verse));
    let terms = reified_terms(row, &v1(), &claim.subject);
    let mut fields: Vec<&[u8]> = vec![bytes(row.span()), claim.subject.as_bytes()];
    fields.extend(terms.iter().map(String::as_bytes));
    let content = ContentDigest::of(&spec_preimage(&fields));
    assert!(
        citation_nodes(claim).contains(&spec_node_iri("citation", &contract, row.span(), &content)),
        "the row's node is the one the specification's formula mints"
    );

    // Field 3 is the chunking id, and the specification forbids the
    // contract id there by name. Nothing else would do: the same formula
    // under the wider id mints an IRI this document states nowhere.
    let wider = v1().contract_id();
    assert_ne!(wider.to_hex(), contract.to_hex());
    let first = &document.units()[0];
    let under_wider = spec_node_iri("unit", &wider, first.span(), &first.digest());
    assert!(
        claims.iter().all(|c| !c.turtle.contains(&under_wider)),
        "a node addressed by the emission id is a node this graph does not have"
    );
}

/// §5's length prefix, executed: *no two field sequences share a
/// preimage*, which is a claim about the sequences whose bytes
/// concatenate alike — the only ones that could have collided.
#[test]
fn the_length_prefix_keeps_two_field_sequences_that_concatenate_alike_apart() {
    let contract = v1().chunking_id();
    let mint = |line: &[u8], unit: &str, terms: &[&str]| {
        let terms: Vec<String> = terms.iter().map(|t| (*t).to_owned()).collect();
        purrdf_markdown::citation_iri(&v(), GUIDE_ID, &contract, 0, 4, line, unit, &terms)
    };
    // Unframed, `ab` ++ `c` and `a` ++ `bc` are one string.
    assert_eq!(
        [b"ab".as_slice(), b"c"].concat(),
        [b"a".as_slice(), b"bc"].concat()
    );
    assert_ne!(mint(b"ab", "c", &[]), mint(b"a", "bc", &[]));
    // So are one reified term of two names and two of one.
    assert_ne!(mint(b"row", "u", &["ab"]), mint(b"row", "u", &["a", "b"]));
    // And the formula is still a function of what it is handed.
    assert_eq!(mint(b"ab", "c", &[]), mint(b"ab", "c", &[]));
}

/// The second path to a section's identity, held to the first.
///
/// [`section_iri`] is public surface a consumer mints with, and the
/// projection mints its own section IRIs from a digest it has already
/// taken. They are one formula in the source; nothing but this says they
/// are one answer over a real document, where the *fields* — which span,
/// which bytes — are what the projection chooses and the consumer has to
/// guess.
///
/// The unit half of this equivalence is pinned in `identity`'s own unit
/// tests, and the preimage behind both is pinned from the specification
/// in [`the_preimage_section_five_states_mints_the_very_ids_the_slicer_emits`].
#[test]
fn the_section_iri_a_consumer_mints_is_the_section_iri_the_projection_emits() {
    let document = model(GUIDE, &v1());
    let claims = slice(GUIDE, &v1());
    let contract = v1().chunking_id();
    let emitted = sections(&claims);
    assert_eq!(emitted.len(), document.sections().len());
    assert!(!emitted.is_empty(), "the guide states sections");
    for (claim, section) in emitted.iter().zip(document.sections()) {
        let span = section.heading_span();
        assert_eq!(
            claim.subject,
            section_iri(
                &v(),
                GUIDE_ID,
                &contract,
                span.start,
                span.end,
                &GUIDE.as_bytes()[span.start as usize..span.end as usize]
            ),
            "one formula, one answer"
        );
    }
    // The span is the heading line's and not the section's, which is the
    // part a consumer holding a model has to get right: minting over the
    // whole section names a node this graph does not have.
    let whole = document.sections()[0].span();
    let over_whole = section_iri(
        &v(),
        GUIDE_ID,
        &contract,
        whole.start,
        whole.end,
        &GUIDE.as_bytes()[whole.start as usize..whole.end as usize],
    );
    assert_ne!(emitted[0].subject, over_whole);
    assert!(claims.iter().all(|c| !c.turtle.contains(&over_whole)));
}

/// The same equivalence for the fourth kind: [`structure_iri`] is the
/// public formula a consumer mints with, and since this change the
/// projection mints its own structure IRIs through the very same call
/// — but one call site is one spelling, and only this vector says the
/// fields it fills (which span, which bytes) are the ones a consumer
/// holding a model would choose.
#[test]
fn the_structure_iri_a_consumer_mints_is_the_structure_iri_the_projection_emits() {
    let document = model(GUIDE, &v1());
    let claims = slice(GUIDE, &v1());
    let contract = v1().chunking_id();
    let emitted: Vec<&Claim> = claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Structure)
        .collect();
    assert_eq!(emitted.len(), document.structures().len());
    assert!(
        !emitted.is_empty(),
        "the guide has structure between its units"
    );
    for (claim, span) in emitted.iter().zip(document.structures()) {
        assert_eq!(
            claim.subject,
            structure_iri(
                &v(),
                GUIDE_ID,
                &contract,
                span.start,
                span.end,
                &GUIDE.as_bytes()[span.start as usize..span.end as usize]
            ),
            "one formula, one answer"
        );
    }
    // And the kind is a field of the preimage: the same span of the
    // same bytes under the unit kind is a node this graph never mints.
    let first = document.structures()[0];
    let as_unit = unit_iri(
        &v(),
        GUIDE_ID,
        &contract,
        first.start,
        first.end,
        &GUIDE.as_bytes()[first.start as usize..first.end as usize],
    );
    assert_ne!(emitted[0].subject, as_unit);
    assert!(claims.iter().all(|c| !c.turtle.contains(&as_unit)));
}

// --- the heading stack, stated twice and held to one answer ---------------

/// A document built from a sequence of levels: each entry opens a
/// section — a heading at that level, or a movement marker where the
/// level is `0` — and is followed by one paragraph, so every section has
/// a unit under it to report the stack the reader held there.
///
/// Every heading and every movement name is distinct, so a lineage names
/// its sections unambiguously.
fn levelled(levels: &[u32]) -> String {
    levels
        .iter()
        .enumerate()
        .map(|(i, &level)| {
            let opener = if level == 0 {
                format!("\u{2042} *m{i}*")
            } else {
                format!("{} h{i}", "#".repeat(level as usize))
            };
            format!("{opener}\n\nbody of {i}.\n\n")
        })
        .collect::<Vec<String>>()
        .concat()
}

/// The reader's stack and the law's closure, asked the same question
/// about one document and required to answer the same way.
///
/// The reader answers *what was in force at this unit* — the innermost
/// section and the lineage — off the stack it pops as it walks the
/// lines. The law answers *whose child is this section, and where does
/// it end* off a stack of its own, built after the reading is over. Here
/// the two are put side by side: the lineage the reader handed the unit
/// must be the parent chain the law built, heading for heading, and the
/// spans the law closed must contain that unit exactly where the chain
/// says they do and nowhere else.
fn the_reader_and_the_law_agree_on(levels: &[u32]) {
    let text = levelled(levels);
    let document = model(&text, &v1());
    assert_eq!(document.sections().len(), levels.len(), "{levels:?}");
    assert_eq!(document.units().len(), levels.len(), "{levels:?}");
    for unit in document.units() {
        // The reader's answer.
        let innermost = unit.section().expect("a unit under a section");
        let opened_before = document
            .sections()
            .iter()
            .rposition(|s| s.span().start < unit.span().start)
            .expect("a section opened before it");
        assert_eq!(
            innermost, opened_before,
            "{levels:?}: the section in force is the last one opened"
        );
        // The law's answer: the parent chain of that section, outermost
        // first.
        let mut chain: Vec<String> = Vec::new();
        let mut at = Some(innermost);
        while let Some(index) = at {
            let section = &document.sections()[index];
            chain.push(section.heading().to_owned());
            at = section.parent();
        }
        chain.reverse();
        assert_eq!(unit.lineage(), chain.as_slice(), "{levels:?}");
        // And the closure the law states holds exactly the chain: every
        // section of it contains the unit, and no other section does.
        for section in document.sections() {
            assert_eq!(
                section.span().contains_span(unit.span()),
                chain.iter().any(|h| h.as_str() == section.heading()),
                "{levels:?}: {:?} against the unit at {:?}",
                section.heading(),
                unit.span()
            );
        }
    }
}

/// The heading-stack law is stated in two places — the dialect reader
/// pops it to answer what is in force at a line, and the model pops it
/// to close every section's span and name its parent — because the two
/// are asked different questions at different times and neither can hand
/// the other its answer. This is what keeps them from drifting apart:
/// the same documents, both answers, and an equality.
///
/// Exhaustive over every sequence of one to three sections at any level
/// (six heading levels and the movement marker, so 7 + 49 + 343
/// documents), then over deterministic eight-section sequences — the
/// depths a short exhaustion cannot reach, generated by a fixed
/// recurrence so the suite reads no clock and no entropy and slices the
/// same documents on every run and every target.
#[test]
fn the_reader_and_the_law_agree_on_the_heading_stack_over_generated_level_sequences() {
    for a in 0..=6 {
        the_reader_and_the_law_agree_on(&[a]);
        for b in 0..=6 {
            the_reader_and_the_law_agree_on(&[a, b]);
            for c in 0..=6 {
                the_reader_and_the_law_agree_on(&[a, b, c]);
            }
        }
    }
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..64 {
        let mut levels: Vec<u32> = Vec::with_capacity(8);
        for _ in 0..8 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            levels.push(u32::try_from((state >> 33) % 7).expect("a level under seven"));
        }
        the_reader_and_the_law_agree_on(&levels);
    }
}

// --- the two forbidden sets, and the boundary that closes the gap ---------

/// The C1 control block, which the crate's own character rule does not
/// name and the canonical writer would escape: the gap between the two
/// forbidden sets, closed by the IRI law before a byte of output exists.
const C1_CONTROLS: [char; 3] = ['\u{80}', '\u{85}', '\u{9f}'];

/// U+00A0, the first code point of RFC-3987 `ucschar`: one past the end
/// of C1, and the neighbour that proves the boundary above is the IRI
/// grammar's and not a blacklist's.
const FIRST_UCSCHAR: char = '\u{a0}';

/// The two "forbidden" sets in this crate are not the same set, and the
/// difference is closed by a boundary neither of them states.
///
/// The crate's own character rule refuses the reserved delimiters, the
/// space and the C0 controls. `purrdf-core`'s canonical writer escapes
/// every control — C0, the DEL, **and** the C1 block (U+0080–U+009F),
/// which the character rule never mentions. Nothing would stop a C1
/// character reaching the writer except that `ucschar` opens at U+00A0,
/// so the IRI law refuses C1 at every seam that takes a caller's IRI.
/// That is a fact about a third law, and this is what states it: the C1
/// refusals are **malformed** — the IRI law's finding, carried — and
/// never the crate's own character refusal, and U+00A0 one code point
/// later is admitted everywhere.
#[test]
fn a_c1_control_in_a_caller_iri_is_refused_by_the_iri_law_and_the_first_ucschar_still_mints() {
    for c1 in C1_CONTROLS {
        // The source id.
        let id = format!("https://example.org/doc{c1}");
        match slice_of(THREE, &id, &v1()).expect_err("no IRI, no document") {
            MarkdownError::MalformedSourceId { id: named, cause } => {
                assert_eq!(named, id);
                assert_eq!(cause.diagnostic_code(), "iri-disallowed-char");
            }
            other => panic!("{c1:?} is no IRI character, and the law must say so: {other:?}"),
        }
        // A vocabulary field.
        let mut broken = v1();
        broken.vocabulary.cites = format!("urn:test:cites{c1}");
        match slice_of(THREE, GUIDE_ID, &broken).expect_err("no IRI, no term") {
            MarkdownError::MalformedVocabulary { field, cause, .. } => {
                assert_eq!(field, "cites");
                assert_eq!(cause.diagnostic_code(), "iri-disallowed-char");
            }
            other => panic!("{c1:?} states no term: {other:?}"),
        }
        // A declared canon base.
        let base = format!("https://example.org/canon{c1}#");
        match slice_of(&cited("a-one"), GUIDE_ID, &under_canon(&base)).expect_err("no IRI, no base")
        {
            MarkdownError::MalformedCanonBase { base: named, cause } => {
                assert_eq!(named, base);
                assert_eq!(cause.diagnostic_code(), "iri-disallowed-char");
            }
            other => panic!("{c1:?} is no base to mint under: {other:?}"),
        }
        // And an anchor, which mints its IRI by concatenation and is
        // walked against the same law. The character sits *inside* the
        // anchor: a cell's value is trimmed (§4), and two of these three
        // are Unicode whitespace, so at an edge they would be gone
        // before the lift ever saw them.
        let anchor = format!("a{c1}b");
        assert_eq!(
            slice_of(&cited(&anchor), GUIDE_ID, &under_canon(CANON_BASE)),
            Err(MarkdownError::InvalidAnchor {
                anchor,
                verses: (1, 1),
            })
        );
    }

    // The neighbour, one code point later, at every one of those seams:
    // U+00A0 is `ucschar`, so it is an IRI character, and it mints.
    let id = format!("https://example.org/doc{FIRST_UCSCHAR}");
    let claims = slice_of(THREE, &id, &v1()).expect("the first ucschar is an IRI character");
    assert_eq!(claims[0].subject, id);
    let mut widened = v1();
    widened.vocabulary.cites = format!("urn:test:cites{FIRST_UCSCHAR}");
    assert_eq!(widened.vocabulary.validate(), Ok(()));
    let base = format!("https://example.org/canon{FIRST_UCSCHAR}#");
    let anchor = format!("a{FIRST_UCSCHAR}one");
    let claims = slice_of(&cited(&anchor), GUIDE_ID, &under_canon(&base)).expect("slices");
    assert_eq!(verse_one_cites(&claims), vec![format!("<{base}{anchor}>")]);

    // And what a `ucschar` anchor mints is readable Turtle. This leg
    // carries U+00A1 rather than U+00A0, because it asks a different
    // question of a different law: the lexer this suite re-reads claims
    // with ends an IRI reference at Unicode whitespace, and U+00A0 is
    // Unicode whitespace. What the IRI law admits into a graph and what
    // a lexer reads back out of one are two questions, and only the
    // first is this crate's.
    let anchor = "a\u{a1}one";
    let claims = slice_of(&cited(anchor), GUIDE_ID, &under_canon(CANON_BASE)).expect("slices");
    assert_eq!(
        verse_one_cites(&claims),
        vec![format!("<{CANON_BASE}{anchor}>")]
    );
    parses_whole(&claims);

    // The eager rule really is the narrower of the two: it names none of
    // the characters above, which is why the IRI law is the one that
    // answers for them.
    for c in C1_CONTROLS.into_iter().chain([FIRST_UCSCHAR, '\u{7f}']) {
        let id = format!("https://example.org/doc{c}");
        assert!(
            !matches!(
                slice_of(THREE, &id, &v1()),
                Err(MarkdownError::InvalidSourceId { .. })
            ),
            "{c:?} is refused by the IRI law or not at all"
        );
    }
}

// --- the specification ----------------------------------------------------

/// The specification the conformance clause names, read from the crate.
const SPEC: &str = include_str!("../SPEC.md");

/// An **anti-rot pin**, and deliberately nothing more: every clause of
/// the specification this suite executes elsewhere is still *present* in
/// the specification, spelled the way the vectors were written against.
///
/// It catches a clause deleted, reworded past recognition, or quietly
/// weakened in an edit, and that is worth having — a law nobody can read
/// is not a law. What it cannot catch is the opposite failure, and no
/// reader should mistake it for conformance: **a specification can state
/// a clause the code does not implement, and every one of these
/// assertions will still pass.** Twice in this crate's history it did —
/// a clause about trimmed text, and a clause putting the canon base
/// outside a contract id — and this vector was green throughout both.
///
/// The clause-to-behaviour vectors are the ones that answer for the law
/// being *true*: §2 in
/// [`the_dialect_grammar_of_section_two_recognizes_exactly_what_it_states`],
/// §4.3 in
/// [`the_anchor_lift_concatenates_never_resolves_and_walks_every_row_of_the_table`],
/// and §5 in
/// [`the_preimage_section_five_states_mints_the_very_ids_the_slicer_emits`]
/// and its neighbours. Adding a clause here is cheap and proves nothing;
/// adding one there is the work.
#[test]
fn the_specification_still_states_every_clause_this_suite_pins_and_carries_no_process() {
    assert!(SPEC.starts_with("<!--"), "a license header opens it");
    for clause in [
        "Version 2.0.0-draft",
        "2026-09-10",
        STANDARD_NAMESPACE,
        "crates/markdown/tests/slicer.rs",
        // The split law, with the nuances the vectors pin.
        "at most `max_bytes` bytes",
        "the cut lands on that newline",
        "no piece carries it",
        "snapped backward to a line start",
        "never inside a scalar",
        "never across a heading",
        // The dialect and the concordance.
        "U+2042",
        // The leading-indent bound, both halves of it, and the tab
        // clause that keeps it total.
        "A line's **leading run** is its run of U+0020 SPACE characters",
        "leading run is **at most three** spaces",
        "A leading run of **four or more** spaces opens no marker.",
        "MUST NOT treat such a\nline as one",
        "A **tab** (U+0009) in the leading run opens no marker either",
        "MUST NOT expand a tab to do it",
        "MUST NOT trim them from any of the three",
        "`## Concordance`",
        "lifts nothing here, and is not an error",
        // The two containment rules of the concordance, each of which
        // loses a row in silence when it is read the other way.
        "**Inside is containment (§1.1), not innermost.**",
        "at **any** depth",
        "### 4.1 The cell-escape law",
        "is a literal `|`",
        "is a literal `\\`",
        // The emission law, with the shapes the vectors pin.
        "<citation> rdf:reifies <<( <unit> <cites> <anchor> )>>",
        "the RDF 1.2 **triple term**",
        "only in **object** position",
        "MUST NOT emit `canonSource` on a unit",
        // One writer, and the predicate position inside it.
        "**The predicate position is an IRI position.**",
        "MUST NOT keep a second rendering",
        // The anchor lift, which is concatenation and never resolution.
        "**pure concatenation**",
        // The law that keeps a citation node attached to its unit, and
        // the row shape that has nothing else holding it there.
        "**A citation node is never an orphan.**",
        "MUST NOT make either one conditional",
        "**Zero is one of those numbers.**",
        "is reachable from the unit it is an edge of, in one step",
        "the DEL (U+007F)",
        "`scalarStart`",
        "`contentDigest`",
        "`xsd:hexBinary`",
        "`rdf:reifies`",
        // Identity: the two preimages, and which id addresses a node.
        "A **citation's IRI**",
        "**The chunking preimage** states only what decides where a unit starts",
        "**The emission preimage** is the chunking preimage **verbatim, as a",
        "two runs that agree on the id MUST agree on",
        // The canon base, inside the contract id and stated either way.
        "`canon base none`",
        "MUST NOT be left implied by a missing line",
        // The term set: inside the id beside the base, and every term of
        // it in one role.
        "the vocabulary, written as one base **and the local names\nderived under it**",
        "**No IRI of this term set is written in two roles.**",
        "MUST use these names and MUST NOT\ncollapse two of them into one",
        "`headingText`",
        "`lineagePath`",
        "both an `owl:DatatypeProperty` and an `rdfs:Datatype`",
        // Which id is field 3, and what a citation's identity carries.
        "Field 3 is the chunking id and MUST NOT be the contract id.",
        "triple terms the node reifies",
        // The cardinality: one node per lifting, and the reified set —
        // not one node per reified triple. Both halves are pinned,
        // because a sentence stating only the first is the one that
        // read two ways.
        "A citation node is a **reifier of one lifting**",
        "**every** cited edge that lifting produced — none,\none, or many",
        "tell apart two liftings that reify\n**different sets of terms**",
        "asserts edges no single document ever stated of it",
        "is still exactly one node, with exactly one\nidentity",
        "a row naming five anchors on\none verse is **one** citation carrying five reified terms",
        "One node per (row, unit) — **exactly one**,",
        "a row of two anchors gives **one** node carrying **two**",
        "MUST NOT split it into\n   two nodes, one per reified triple",
        // The three identities, and the law that verifies a chunk.
        "### 5.1 The three identities, and the verification law",
        "NOT write the contract id or the chunking id where a chunking-stage id is",
        "carrying **the\nchunking preimage** as its parameters",
        "The **verification law** for such a chunk is",
        // Ordering, provenance, conformance, determinism.
        "rdf:Seq",
        // Provenance guidance: the recommendation is only usable if it
        // names the term, and only honest if it says the namespace does
        // not resolve yet. A bare ontology name pins nothing.
        "`https://blackcatinformatics.ca/logic/Plan`",
        "<https://blackcatinformatics.ca/gmeow/slices/logic>",
        "`logic:planSuccessMode`",
        "the slice it is defined in is a provable layer",
        "PROV-O is not a rival here.",
        "gives up no PROV interoperability",
        "not yet publicly dereferenceable",
    ] {
        assert!(SPEC.contains(clause), "the specification states {clause:?}");
    }
    // Process lives where process lives, and that is not in the repository.
    for token in ["PR #", "issue #", "Issue #", "pull request"] {
        assert!(!SPEC.contains(token), "no process reference: {token:?}");
    }
}

// --- the README ------------------------------------------------------------

/// The crate's front page, read from the crate: its emission examples
/// are output of the law, and this is what keeps them so.
const README: &str = include_str!("../README.md");

/// The Markdown the README slices in its example.
const README_BOOK: &str = "# The Book\n\n\u{2042} *the crossing*\n\n1. The first verse crosses the socket whole.\n\n\
     2. Two sovereign stars share one trajectory.\n";

/// The concordance the README appends to it.
const README_CONCORDANCE: &str = "\n## Concordance\n\n| Verses | Canon source | Anchors |\n\
                                  |---|---|---|\n| 2 | `atlas/crossing.logic.ttl` | \
                                  `the-crossing` |\n";

/// Every run of exactly 64 lowercase hex digits shortened to its first
/// eight and an ellipsis: the abbreviation the README announces.
fn shorten_digests(text: &str) -> String {
    let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
    let bytes: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let run = bytes[i..].iter().take_while(|c| hex(**c)).count();
        // A 64-digit run is a digest; a shorter or longer one is text.
        if run == 64 && !bytes[..i].last().is_some_and(|c| hex(*c)) {
            out.extend(&bytes[i..i + 8]);
            out.push('\u{2026}');
        } else {
            out.extend(&bytes[i..i + run.max(1)]);
        }
        i += run.max(1);
    }
    out
}

/// The `n`th fenced block of a language in the README, without its
/// fences.
fn readme_block(language: &str, n: usize) -> String {
    README
        .split(&format!("```{language}\n"))
        .skip(1)
        .map(|rest| {
            rest.split_once("```")
                .expect("a closing fence")
                .0
                .to_owned()
        })
        .nth(n)
        .expect("the block")
}

#[test]
fn the_readmes_emission_examples_are_the_claims_the_law_emits() {
    let profile = Profile::new(
        "example-v1",
        1,
        Vocabulary::under("urn:example:doc:").expect("a vocabulary"),
    );
    let verse_two = |text: &str| {
        let claims = slice_of(text, "urn:example:book", &profile).expect("slices");
        let two = *units(&claims)
            .iter()
            .find(|u| integer(u, &profile.vocabulary.verse) == Some(2))
            .expect("verse 2");
        (shorten_digests(&two.turtle), two.subject.clone())
    };

    // The README's own Markdown, sliced, is the README's own claim.
    assert_eq!(
        readme_block("markdown", 0).trim_end(),
        README_BOOK.trim_end()
    );
    let (plain, plain_subject) = verse_two(README_BOOK);
    assert_eq!(plain, readme_block("text", 0));

    // And with the concordance appended: the same node, five lines more.
    assert_eq!(
        readme_block("markdown", 1).trim(),
        README_CONCORDANCE.trim()
    );
    let with_rows = format!("{README_BOOK}{README_CONCORDANCE}");
    let (cited, cited_subject) = verse_two(&with_rows);
    assert_eq!(
        cited_subject, plain_subject,
        "the verse's bytes and span did not move, so its IRI did not either"
    );
    assert_eq!(cited.lines().count(), plain.lines().count() + 5);
    let added: Vec<&str> = cited
        .lines()
        .filter(|line| !plain.lines().any(|kept| kept == *line))
        .collect();
    assert_eq!(added, readme_block("text", 1).lines().collect::<Vec<_>>());
    assert!(
        added.iter().any(|line| line.contains("<<( ")),
        "one of the three is the reified edge"
    );
    parses_whole(&slice_of(&with_rows, "urn:example:book", &profile).expect("slices"));
}
