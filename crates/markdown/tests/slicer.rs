// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Vectors for the structural slicer: determinism, structure, the split
//! law, the concordance, identity, and the goldens a declared profile
//! reproduces byte for byte.

use std::collections::{BTreeMap, BTreeSet};

use pretty_assertions::assert_eq;
use purrdf_core::{CanonHash, try_canonicalize_with};
use purrdf_markdown::{
    Claim, ClaimKind, MarkdownError, Profile, SourceDocument, Vocabulary, slice_markdown, unit_iri,
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
        let single_line = !GUIDE[start..end].contains('\n');
        assert!(
            end - start <= small().max_bytes || single_line,
            "{}",
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
