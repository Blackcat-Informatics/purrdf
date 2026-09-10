// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Vectors for the structural slicer: determinism, structure, the split
//! law, the concordance, identity, and the goldens a declared profile
//! reproduces byte for byte.

use std::collections::{BTreeMap, BTreeSet};

use pretty_assertions::assert_eq;
use purrdf_core::{CanonHash, try_canonicalize_with};
use purrdf_markdown::{
    Claim, ClaimKind, MIN_MAX_BYTES, MarkdownError, Profile, SourceDocument, Vocabulary,
    slice_markdown, unit_iri,
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

/// `(predicate, object)` pairs of one claim, objects as rendered. A
/// claim's lines are sorted bytewise, so repeated predicates come back
/// in object order.
fn pairs(claim: &Claim) -> Vec<(String, String)> {
    let prefix = format!("<{}> ", claim.subject);
    claim
        .turtle
        .lines()
        .map(|line| {
            let rest = line.strip_prefix(&prefix).expect("subject prefix");
            let rest = rest.strip_suffix(" .").expect("terminator");
            let (p, o) = rest.split_once(' ').expect("predicate then object");
            (
                p.trim_start_matches('<').trim_end_matches('>').to_owned(),
                o.to_owned(),
            )
        })
        .collect()
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
        6,
        "title, two movements, field notes, tide tables, concordance"
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
        Some(&*format!("\"{TITLE}\"^^<{}>", v().dt_heading))
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
        assert!(GUIDE[start as usize..].starts_with('\u{2042}'));
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
        Some(&*format!("\"Concordance\"^^<{}>", v().dt_heading))
    );
    assert_eq!(concordance.span.map(|s| s.1), Some(GUIDE.len() as u64));
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
                    &profile.contract_id(),
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
            typed_value(heading, &v().heading).expect("a heading"),
            integer(heading, &v().ordinal).expect("an ordinal"),
            integer(heading, &v().level).expect("a level"),
            chain.len(),
        ));
    }
    assert_eq!(
        before_a_heading,
        vec![
            ("Field notes".to_owned(), 3_u64, 2_u64, 2_usize),
            ("Tide tables".to_owned(), 4, 3, 3),
            ("Concordance".to_owned(), 5, 2, 5),
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
        Some(&*format!("\"T\u{ed}tulo\"^^<{}>", v().dt_heading))
    );
    assert_eq!(
        object(s[1], &v().heading).as_deref(),
        Some(&*format!("\"el camino\"^^<{}>", v().dt_heading))
    );
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
        assert_eq!(
            objects(verse(n), &v().canon_source),
            vec![path("atlas/outer-reefs.logic.ttl")]
        );
    }
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
        objects(verse(3), &v().canon_source),
        vec![
            path("atlas/inner-lagoon.logic.ttl"),
            path("atlas/outer-reefs.logic.ttl")
        ],
        "a source named by two rows is stated once"
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

#[test]
fn the_profile_id_states_itself_moves_with_every_constant_and_ignores_the_canon_base() {
    let id = |profile: &Profile| profile.contract_id().to_hex();
    let declared = id(&v1());
    assert_eq!(
        declared,
        id(&v1()),
        "the same constants state the same identity"
    );

    let mut renamed = v1();
    renamed.name = format!("{PROFILE_NAME}-other");
    let mut reversioned = v1();
    reversioned.version = 2;
    let mut revocabularied = v1();
    revocabularied.vocabulary.text = "https://example.org/other/body".to_owned();
    let mut wider = v1();
    wider.max_bytes = 4096;
    let mut looser = v1();
    looser.overlap = 64;
    let moved = [&renamed, &reversioned, &revocabularied, &wider, &looser];
    for profile in moved {
        assert_ne!(
            id(profile),
            declared,
            "a constant of the law is inside the identity: {}",
            String::from_utf8(profile.stage_bytes()).expect("utf8")
        );
    }
    let ids: BTreeSet<String> = moved.iter().map(|p| id(p)).collect();
    assert_eq!(ids.len(), moved.len(), "each change mints its own identity");

    let mut based = v1();
    based.canon_base = Some(CANON_BASE.to_owned());
    assert_eq!(
        id(&based),
        declared,
        "the canon base is the consumer's option, not a parameter of the law"
    );

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
    }
    assert_ne!(b[1].subject, a[1].subject);
    assert_eq!(b[1].span, a[1].span, "no boundary moved");
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
        8 + 4,
        "the headnote, two notes, and the concordance prose are paragraphs"
    );
    let count = |predicate: &str| -> usize {
        all.iter()
            .flat_map(|u| pairs(u))
            .filter(|(p, _)| p == predicate)
            .count()
    };
    assert_eq!(count(&v().cites), 16);
    assert_eq!(count(&v().canon_source), 10);
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
    assert_eq!(with_mark.len(), plain.len());
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
    let stage = String::from_utf8(explicit.stage_bytes()).expect("utf8");
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
fn an_empty_or_relative_canon_base_refuses_and_an_absolute_one_lifts() {
    for base in ["", "not an iri >", "canon#"] {
        assert_eq!(
            slice_of(&cited("a-one"), GUIDE_ID, &under_canon(base)),
            Err(MarkdownError::InvalidCanonBase {
                base: base.to_owned()
            }),
            "there is no IRI to mint under"
        );
    }
    for base in [CANON_BASE, CANON_PATH_BASE] {
        let claims = slice_of(&cited("a-one"), GUIDE_ID, &under_canon(base)).expect("slices");
        assert_eq!(verse_one_cites(&claims), vec![format!("<{base}a-one>")]);
    }
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
    assert_eq!(typed_value(s[0], &v().heading).as_deref(), Some("T"));
    assert_eq!(
        typed_value(s[1], &v().heading).as_deref(),
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
            .map(|x| typed_value(x, &v().heading))
            .collect::<Vec<_>>(),
        sections(&claims)
            .iter()
            .map(|x| typed_value(x, &v().heading))
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
    assert_eq!(typed_value(s[0], &v().heading).as_deref(), Some("T"));
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
        typed_value(s[0], &v().heading).as_deref(),
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
        typed_value(sections(&claims)[0], &v().heading).as_deref(),
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
