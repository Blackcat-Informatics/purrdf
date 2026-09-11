// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The codec's round trip, byte for byte: Markdown to RDF to Markdown.
//!
//! The decode leg is the shipped one. Every claim's Turtle is parsed
//! back through `purrdf-rdf` — the same parser any consumer holds —
//! and handed to [`decode_document`], the crate's own graph half of
//! the decode law, so what these vectors prove is the codec's whole
//! promise as a consumer actually reaches it: hold the triples, call
//! the law, get the document, and the digest proves it.
//!
//! The span half's refusal vectors live beside the law itself, in the
//! kernel's own suite; here are the graph half's — and the invariants
//! that are about the *emission*: structure runs maximal and covering,
//! plain literals exactly the content, split chains present in the
//! graphs that claim to exercise them.

use pretty_assertions::assert_eq;
use purrdf_core::ir::RdfDataset;
use purrdf_markdown::{
    Claim, ClaimKind, DecodeError, Profile, SourceDocument, Vocabulary, decode_document,
    slice_markdown,
};
use purrdf_rdf::parse_dataset;

const DOC_ID: &str = "https://example.org/doc/roundtrip";
const SLICE_BASE: &str = "https://example.org/slice/";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

fn v() -> Vocabulary {
    Vocabulary::under(SLICE_BASE).expect("a vocabulary")
}

fn profile() -> Profile {
    Profile::new("roundtrip-md-v1", 1, v())
}

/// The declared law under a reduced bound, so long units split and the
/// split's declared overlap enters the graph.
fn tight(max_bytes: usize, overlap: usize) -> Profile {
    let mut p = profile();
    p.max_bytes = max_bytes;
    p.overlap = overlap;
    p
}

fn slice(text: &str, profile: &Profile) -> Vec<Claim> {
    slice_markdown(
        &SourceDocument {
            id: DOC_ID,
            bytes: text.as_bytes(),
        },
        profile,
    )
    .expect("slices")
}

/// The claims as one Turtle document, parsed back through the
/// workspace parser: the graph a consumer holds.
fn graph_of(claims: &[Claim]) -> (String, std::sync::Arc<RdfDataset>) {
    let turtle: String = claims.iter().map(|c| c.turtle.as_str()).collect();
    let dataset = parse_dataset(turtle.as_bytes(), "text/turtle", None).expect("the graph parses");
    (turtle, dataset)
}

/// Markdown to RDF to Markdown, through the parser and the shipped
/// decode law, byte for byte.
fn assert_roundtrips(text: &str, profile: &Profile) {
    let claims = slice(text, profile);
    let (first, dataset) = graph_of(&claims);
    let rebuilt =
        decode_document(&dataset, &v(), DOC_ID).expect("the graph decodes back to a document");
    assert_eq!(rebuilt, text, "and the document is the very bytes sliced");
    // The encode leg is deterministic in the bytes and the profile:
    // the same input renders the same graph, byte for byte.
    let again: String = slice(text, profile)
        .iter()
        .map(|c| c.turtle.as_str())
        .collect();
    assert_eq!(first, again);
}

/// The authored fixture, and the same fixture under bounds that split
/// its long units so the declared overlap enters the graph.
const GUIDE: &str = include_str!("fixtures/field-guide.md");

#[test]
fn the_field_guide_roundtrips_under_the_declared_law_and_under_splitting_bounds() {
    assert_roundtrips(GUIDE, &profile());
    for (max_bytes, overlap) in [(200, 40), (200, 0), (64, 63), (5, 2)] {
        let bounded = tight(max_bytes, overlap);
        // The bound is small enough that the guide's long units MUST
        // split, and the graph must say so: a split law that regressed
        // to never splitting would otherwise round-trip green here
        // while the chains it claims to exercise never existed.
        let (turtle, _) = graph_of(&slice(GUIDE, &bounded));
        assert!(
            turtle.contains(&format!("<{}>", v().continues)),
            "a {max_bytes}-byte bound splits the guide's units"
        );
        assert_roundtrips(GUIDE, &bounded);
    }
    let mut based = profile();
    based.canon_base = Some("https://example.org/canon#".to_owned());
    assert_roundtrips(GUIDE, &based);
}

/// The shared-cut geometry, in a real graph rather than a hand-built
/// span array: two pieces of one chain ending at one cut, the very
/// case where the walk's frontier and the declared piece part company.
#[test]
fn a_real_split_chain_with_a_shared_cut_declares_its_overlaps_and_roundtrips() {
    let text = "# H\n\nab\ncd\nef\ngh\nij\n";
    let bounded = tight(4, 1);
    let claims = slice(text, &bounded);
    let unit_spans: Vec<(u64, u64)> = claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Unit)
        .map(|c| c.span.expect("a unit span"))
        .collect();
    assert!(
        unit_spans
            .iter()
            .any(|a| unit_spans.iter().any(|b| a != b && a.1 == b.1)),
        "two pieces of the chain end at one cut: {unit_spans:?}"
    );
    let (turtle, dataset) = graph_of(&claims);
    assert!(
        turtle.contains(&format!("<{}>", v().continues)),
        "and the chain declares its pieces"
    );
    assert_eq!(decode_document(&dataset, &v(), DOC_ID).as_deref(), Ok(text));
}

#[test]
fn every_shape_of_document_roundtrips_byte_for_byte() {
    let fixtures: &[&str] = &[
        // Nothing at all, and nothing but structure.
        "",
        "\n",
        "\n\n\n",
        "---\n",
        // A bare paragraph, with and without its trailing newline.
        "one paragraph\n",
        "no trailing newline",
        "# T\n\n1. ends without a newline",
        // Two byte-identical paragraphs: two units, two spans, one
        // text — the case that catches any decoder keyed on content
        // rather than position.
        "same\n\nsame\n",
        // CRLF endings: the `\r` is in every span and every literal.
        "# T\r\n\r\n1. one\r\n\r\n2. two\r\n",
        // A leading byte order mark: encoding at byte zero, structure
        // afterwards, every byte still covered.
        "\u{feff}# T\u{ed}tulo\n\n1. \u{4e2d} \u{1f41a}\n",
        // Markers behind leading runs, and a run too deep to be one.
        "   # T\n\n  \u{2042} *m*\n\n 1. one\n\n    # content, not a heading\n",
        // A tab in the run: content, never a marker.
        "\t# content\n\nbody\n",
        // Horizontal rules and a table no concordance holds: structure
        // that lifts nothing, carried all the same.
        "# T\n\n---\n\n| a | b |\n| 1 | 2 |\n\n***\n\ntail\n",
        // A concordance that lifts, wide verses, an escaped pipe.
        "# T\n\n1. One.\n\n## Concordance\n\n| Verses | Canon source | Anchors |\n\
         | --- | --- | --- |\n| 1 | `atlas/x.ttl` | `a-one` |\n\
         | 2-9 | `atlas/y\\|z.ttl` | `a-two` |\n",
        // Movements as siblings, empty headings, seven hashes as prose.
        "## \u{201c}Deep\u{201d}\n\n\u{2042} *first*\n\n\u{2042} *second*\n\n####\n\n####### prose\n",
    ];
    for text in fixtures {
        assert_roundtrips(text, &profile());
        assert_roundtrips(text, &tight(6, 3));
    }
}

/// The graph half's refusals, each beside the neighbouring graph that
/// still decodes: a decode anchored to a document node that is absent
/// or unreadable, and a text-bearing node shorn of its span.
#[test]
fn a_graph_that_cannot_anchor_a_decode_names_what_it_lacks() {
    let claims = slice("# T\n\nbody\n", &profile());
    let (turtle, dataset) = graph_of(&claims);
    // The wrong document id anchors nothing: the facts are absent.
    assert_eq!(
        decode_document(&dataset, &v(), "https://example.org/doc/other"),
        Err(DecodeError::MissingDocumentFact {
            predicate: "byteLength"
        })
    );
    // A cover whose text-bearing node lost its offsets is named by
    // what it left out: drop the byteStart line of the one unit.
    let amputated: String = turtle
        .lines()
        .filter(|line| !(line.contains(&format!("<{}>", v().byte_start)) && line.contains("unit:")))
        .map(|line| format!("{line}\n"))
        .collect();
    let dataset = parse_dataset(amputated.as_bytes(), "text/turtle", None).expect("still a graph");
    assert!(matches!(
        decode_document(&dataset, &v(), DOC_ID),
        Err(DecodeError::IncompleteSpan {
            missing: "byteStart",
            ..
        })
    ));
    // The neighbour: the whole graph still decodes.
    let (_, whole) = graph_of(&claims);
    assert_eq!(
        decode_document(&whole, &v(), DOC_ID).as_deref(),
        Ok("# T\n\nbody\n")
    );
}

/// "Maximal run" in its checkable form, held over the graph: no two
/// structure runs touch — a maximal run absorbed its neighbour — and
/// none is empty, the last included.
#[test]
fn no_two_structure_nodes_are_adjacent_and_none_is_empty() {
    for p in [profile(), tight(6, 3)] {
        let claims = slice(GUIDE, &p);
        let mut runs: Vec<(u64, u64)> = claims
            .iter()
            .filter(|c| c.kind == ClaimKind::Structure)
            .map(|c| c.span.expect("a structure span"))
            .collect();
        assert!(
            !runs.is_empty(),
            "the guide has structure between its units"
        );
        runs.sort_unstable();
        // Every run, the last included: `windows(2)` alone would leave
        // the final run unasserted and a one-run document unasserted
        // entirely.
        for &(start, end) in &runs {
            assert!(start < end, "a structure span covers bytes");
        }
        for pair in runs.windows(2) {
            assert!(
                pair[0].1 < pair[1].0,
                "two structure runs never touch: a maximal run absorbed its neighbour"
            );
        }
    }
}

/// Plain means content, typed means bytes — held over every literal of
/// the graph: a unit's text and a section's heading words are the only
/// plain literals a claim carries, so an index that selects plain
/// strings sees content and not a byte of structure.
#[test]
fn the_only_plain_literals_are_unit_texts_and_heading_words() {
    use purrdf_core::ir::TermRef;
    let claims = slice(GUIDE, &tight(200, 40));
    let voc = v();
    let (_, dataset) = graph_of(&claims);
    let mut plain = 0usize;
    for quad in dataset.iter() {
        let TermRef::Iri(predicate) = quad.p else {
            continue;
        };
        if let TermRef::Literal {
            datatype,
            language: None,
            ..
        } = quad.o
            && matches!(dataset.resolve(datatype), TermRef::Iri(XSD_STRING))
        {
            plain += 1;
            assert!(
                predicate == voc.text || predicate == voc.heading,
                "a plain literal is a unit's text or a heading's words, never {predicate}"
            );
        }
    }
    assert!(plain > 0, "the guide has content to index");
}
